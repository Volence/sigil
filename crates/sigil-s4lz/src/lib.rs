//! Pure-Rust byte-exact port of `aeon/tools/s4lz.py`'s S4LZ v3 encoder.
//!
//! S4LZ is a word-aligned LZ compressor designed for Sega Genesis (68000)
//! decompression. This crate ports ONLY the encoder (`compress`) — v1/legacy
//! decode is explicitly out of scope, and decode in general is not needed by
//! the `.emp` comptime `s4lz()` builtin this crate backs.
//!
//! Stream format v3 (version byte = 1), mirrored exactly from `s4lz.py`'s
//! module docstring:
//! ```text
//! Header (4 bytes): [u16 BE uncompressed_size][u8 flags: bit0=tile_delta][u8 version=1]
//! Per sequence, token word = [token.b][offmark.b]:
//!     token.b hi nibble = literal word count (15 = u16 BE extension word)
//!     token.b lo nibble = match word count   (15 = u16 BE extension word)
//!     token.b $00 = end of stream (offmark.b is $00; full word consumed)
//!     offmark.b = match_offset/2 for offsets 2..510 (short form, no offset
//!                 word); $00 = long form (u16 BE offset word after literals)
//!                 or no match at all (match nibble 0)
//! Stream order: token word, [lit ext word], literals,
//!               [offset word — long form only], [match ext word]
//! ```
//!
//! **BYTE-IDENTITY CONTRACT**: this port must reproduce `s4lz.py`'s exact
//! output for every input, including its heuristics — the hash-chain
//! insertion order, the >=32-word early-exit in the match finder, and the
//! forward-DP tie-breaking. It is not enough to emit a VALID S4LZ stream;
//! the bytes must match the Python encoder's bytes exactly. See
//! `crates/sigil-frontend-emp/tests/vectors/s4lz/README.md` for the vector
//! provenance backing this claim, and `sigil-s4lz/tests/byte_exact.rs` for
//! the gate itself.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Constants (s4lz.py:39-48)
// ---------------------------------------------------------------------------

/// Genesis tile: 8x8 pixels, 4bpp = 32 bytes (s4lz.py:39, `TILE_SIZE`).
pub const TILE_SIZE: usize = 32;
/// Minimum match length in words (4 bytes) (s4lz.py:40, `MIN_MATCH_WORDS`).
const MIN_MATCH_WORDS: usize = 2;
/// Maximum backwards offset in bytes. The 68000 decoder uses `suba.w`
/// (sign-extends); offsets must stay < $8000 (s4lz.py:41-43, `MAX_WINDOW`).
const MAX_WINDOW: usize = 32766;
/// Maximum byte offset encodable in offmark (255 * 2) (s4lz.py:44,
/// `MAX_SHORT_OFFSET`).
const MAX_SHORT_OFFSET: usize = 510;
/// End-of-stream marker (s4lz.py:45, `TOKEN_END`).
const TOKEN_END: u8 = 0x00;
/// Nibble value that triggers an extension word (s4lz.py:46,
/// `EXTENDED_THRESHOLD`).
const EXTENDED_THRESHOLD: usize = 15;
/// v3: token word with offmark short-offset slot (s4lz.py:48, `VERSION_V3`).
const VERSION_V3: u8 = 1;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Compression options mirroring `compress(data, tile_delta=False,
/// dictionary=b'')`'s keyword arguments (s4lz.py:174-175).
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub tile_delta: bool,
    pub dictionary: Vec<u8>,
}

impl Options {
    pub fn with_dictionary(dictionary: Vec<u8>) -> Self {
        Options { tile_delta: false, dictionary }
    }

    pub fn with_tile_delta() -> Self {
        Options { tile_delta: true, dictionary: Vec::new() }
    }
}

/// Errors mirroring the `ValueError`s `compress()` raises (s4lz.py:186-198).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompressError {
    /// `dictionary is not supported with tile_delta` (s4lz.py:188).
    DictTileDeltaExclusive,
    /// `dictionary length {n} must be word-even` (s4lz.py:192-193).
    DictLengthOdd(usize),
    /// `dict+data {n} exceeds window {MAX_WINDOW}` (s4lz.py:196-198).
    WindowExceeded { dict_len: usize, data_len: usize },
}

// ---------------------------------------------------------------------------
// Tile-delta XOR preprocessing (s4lz.py:54-78)
// ---------------------------------------------------------------------------

/// XOR each 32-byte tile against the previous tile. First tile unchanged.
/// Mirrors `tile_delta_encode` (s4lz.py:54-64) exactly, including its
/// last-partial-tile handling (`chunk_end = min(offset + TILE_SIZE, len)`).
fn tile_delta_encode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut result = data[..TILE_SIZE.min(data.len())].to_vec();
    let mut offset = TILE_SIZE;
    while offset < data.len() {
        let chunk_end = (offset + TILE_SIZE).min(data.len());
        let prev_start = offset - TILE_SIZE;
        for i in offset..chunk_end {
            result.push(data[i] ^ data[prev_start + (i - offset)]);
        }
        offset += TILE_SIZE;
    }
    result
}

// ---------------------------------------------------------------------------
// Encoding helpers (s4lz.py:84-90)
// ---------------------------------------------------------------------------

/// Build a token byte from literal and match word counts. Each nibble
/// encodes 0-14 directly; 15 means "read next word for count"
/// (s4lz.py:84-89, `_build_token`).
fn build_token(lit_count: usize, match_count: usize) -> u8 {
    let lit_nibble = lit_count.min(15) as u8;
    let match_nibble = match_count.min(15) as u8;
    (lit_nibble << 4) | match_nibble
}

// ---------------------------------------------------------------------------
// Match finder (hash-chain, two candidates per position) (s4lz.py:95-156)
// ---------------------------------------------------------------------------

/// One match candidate: `(byte_offset, word_length)`. `(0, 0)` means no
/// match found — mirrors the Python's tuple-of-tuples return convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct Candidate {
    offset: usize,
    length: usize,
}

/// Hash-chain match finder returning two candidates: (best-overall,
/// best-short-offset). Either is `(0, 0)` when no match of
/// `MIN_MATCH_WORDS` exists. Ports `_find_best_matches` (s4lz.py:95-156)
/// function-for-function, preserving the REVERSED chain-scan order and the
/// `best_length >= 32` early exit exactly (these are load-bearing for byte
/// identity, not just "a valid match").
fn find_best_matches(
    data: &[u8],
    pos: usize,
    data_len: usize,
    hash_table: &HashMap<[u8; 4], Vec<usize>>,
    dict_len: usize,
) -> (Candidate, Candidate) {
    let mut best = Candidate::default();
    let mut best_short = Candidate::default();
    let min_bytes = MIN_MATCH_WORDS * 2;

    if pos + min_bytes > data_len {
        return (best, best_short);
    }

    let key = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
    let candidates = match hash_table.get(&key) {
        Some(v) => v,
        None => return (best, best_short),
    };

    // Check candidates in reverse order (most recent = smallest offset
    // first). Because offsets only grow as we iterate, every short-offset
    // candidate is examined before the early-exit below can trigger
    // (s4lz.py:121-123).
    for &scan_pos in candidates.iter().rev() {
        // dist = pos - scan_pos; Python relies on dist <= 0 being skipped,
        // which also catches scan_pos > pos (impossible here since the hash
        // table is built incrementally, but mirror the check exactly).
        if scan_pos >= pos {
            continue;
        }
        let dist = pos - scan_pos;
        if dist > MAX_WINDOW {
            continue;
        }

        // Count matching words (capped at the dict boundary for dict
        // sources) (s4lz.py:131-142).
        let mut max_words = (data_len - pos) / 2;
        if scan_pos < dict_len {
            max_words = max_words.min((dict_len - scan_pos) / 2);
        }
        let mut match_words = 0usize;
        while match_words < max_words {
            let src_off = scan_pos + match_words * 2;
            let dst_off = pos + match_words * 2;
            if data[src_off] == data[dst_off] && data[src_off + 1] == data[dst_off + 1] {
                match_words += 1;
            } else {
                break;
            }
        }

        if match_words >= MIN_MATCH_WORDS {
            if match_words > best.length {
                best.length = match_words;
                best.offset = dist;
            }
            if dist <= MAX_SHORT_OFFSET && match_words > best_short.length {
                best_short.length = match_words;
                best_short.offset = dist;
            }
            // Good enough heuristic: stop if we found a long match. All
            // nearer (short-offset) candidates were already seen
            // (s4lz.py:151-154). DO NOT remove, reorder, or "tune" this
            // early-exit: it changes WHICH candidates the DP sees, silently
            // breaking byte-exactness vs s4lz.py even though the output
            // stays a valid stream.
            if best.length >= 32 {
                break;
            }
        }
    }

    (best, best_short)
}

// ---------------------------------------------------------------------------
// Compressor (s4lz.py:162-340)
// ---------------------------------------------------------------------------

/// Extra bytes a match adds beyond the words it replaces: long offsets need
/// a u16 offset word, counts >= 15 need an extension word, and any match
/// ends the sequence (next token word, unless at end). Ports `_match_cost`
/// (s4lz.py:162-171).
fn match_cost(offset: usize, m_len: usize, at_end: bool) -> usize {
    let mut cost = if offset <= MAX_SHORT_OFFSET { 0 } else { 2 };
    if m_len >= EXTENDED_THRESHOLD {
        cost += 2;
    }
    if !at_end {
        cost += 2;
    }
    cost
}

/// DP arrival cost sentinel mirroring Python's `float('inf')` — using
/// `usize::MAX` since all real costs are small non-negative byte counts and
/// this crate never needs float semantics here.
const INF: usize = usize::MAX;

/// A DP predecessor entry, mirroring the Python `prev[i]` tuples
/// (`('lit', i)` / `('match', i, offset, len)`) (s4lz.py:253,263,276).
#[derive(Clone, Copy, Debug)]
enum Prev {
    Lit(usize),
    Match(usize, usize, usize), // (source word index, offset, length)
}

/// Compress `data` into an S4LZ v3 stream. Returns the compressed bytes, or
/// a [`CompressError`] mirroring the `ValueError`s `s4lz.py`'s `compress()`
/// raises (s4lz.py:174-340).
///
/// `opts.dictionary` pre-seeds the LZ window: matches may reach back into
/// it, with offsets measured as distances in the dict+data concatenation.
/// The emitted stream encodes ONLY `data` (header size = `data.len()`); a
/// decoder must be given the same dictionary. Matches never straddle the
/// dict/data boundary (see [`find_best_matches`]).
pub fn compress(data: &[u8], opts: &Options) -> Vec<u8> {
    match try_compress(data, opts) {
        Ok(out) => out,
        Err(e) => panic!("sigil_s4lz::compress: {e:?}"),
    }
}

/// Fallible form of [`compress`] mirroring `s4lz.py`'s `ValueError` paths
/// exactly, for callers (the `.emp` builtin) that need to turn these into
/// loud diagnostics rather than a panic.
pub fn try_compress(data: &[u8], opts: &Options) -> Result<Vec<u8>, CompressError> {
    // Apply tile-delta preprocessing if requested (s4lz.py:186-189).
    let (data_owned, tile_delta);
    if opts.tile_delta {
        if !opts.dictionary.is_empty() {
            return Err(CompressError::DictTileDeltaExclusive);
        }
        data_owned = tile_delta_encode(data);
        tile_delta = true;
    } else {
        data_owned = data.to_vec();
        tile_delta = false;
    }
    let data = data_owned.as_slice();

    let dict_len = opts.dictionary.len();
    if !dict_len.is_multiple_of(2) {
        return Err(CompressError::DictLengthOdd(dict_len));
    }

    let data_len_orig = data.len();
    if dict_len + data_len_orig > MAX_WINDOW {
        return Err(CompressError::WindowExceeded { dict_len, data_len: data_len_orig });
    }

    // Build header (s4lz.py:201-202).
    let flags: u8 = if tile_delta { 1 } else { 0 };
    let header = vec![(data_len_orig >> 8) as u8, (data_len_orig & 0xFF) as u8, flags, VERSION_V3];

    if data_len_orig == 0 {
        let mut out = header;
        out.push(TOKEN_END);
        out.push(0);
        return Ok(out);
    }

    // Ensure data is word-aligned for processing (s4lz.py:207-211).
    let mut work_data_only = data.to_vec();
    if data_len_orig % 2 != 0 {
        work_data_only.push(0x00);
    }
    let data_len = work_data_only.len();

    // The match window is the dictionary prepended to the data; offsets are
    // distances within this concatenation (s4lz.py:213-216).
    let mut work_data = Vec::with_capacity(dict_len + data_len);
    work_data.extend_from_slice(&opts.dictionary);
    work_data.extend_from_slice(&work_data_only);
    let concat_len = dict_len + data_len;

    // Build hash table for fast matching (dict + data positions)
    // (s4lz.py:218-224). Iterate ascending so each key's Vec is built in
    // the SAME insertion order as the Python `dict`-of-`list` (append
    // order == position order); the match finder's `.rev()` then walks it
    // nearest-first, exactly like Python's `reversed(candidates)`.
    let mut hash_table: HashMap<[u8; 4], Vec<usize>> = HashMap::new();
    if concat_len >= 4 {
        let mut i = 0usize;
        while i <= concat_len - 4 {
            let key = [work_data[i], work_data[i + 1], work_data[i + 2], work_data[i + 3]];
            hash_table.entry(key).or_default().push(i);
            i += 2;
        }
    }

    // Optimal parser (forward DP with sequence-aware cost model)
    // (s4lz.py:226-285).
    let num_words = data_len / 2;

    // Phase 1: best matches at each DATA word position (concat position
    // dict_len + i*2) — (any, short) candidates (s4lz.py:236-248).
    let mut match_at: Vec<Option<Vec<Candidate>>> = Vec::with_capacity(num_words);
    for i in 0..num_words {
        let (best_any, best_short) =
            find_best_matches(&work_data, dict_len + i * 2, concat_len, &hash_table, dict_len);
        let mut cands = Vec::new();
        if best_any.length >= MIN_MATCH_WORDS {
            cands.push(best_any);
        }
        if best_short.length >= MIN_MATCH_WORDS && best_short != best_any {
            cands.push(best_short);
        }
        match_at.push(if cands.is_empty() { None } else { Some(cands) });
    }

    // Phase 2: forward DP — arrival[i] = min compressed bytes for
    // words[0..i) (s4lz.py:251-276). DO NOT reorder the option enumeration
    // (literal first, then matches; ascending sublengths) or relax the
    // strict `<` tie-break: equal-cost parses are broken by FIRST-FOUND, so
    // enumeration order IS the tie-break — any change silently diverges
    // from s4lz.py while still emitting valid streams.
    let mut arrival = vec![INF; num_words + 1];
    arrival[0] = 2; // first sequence always costs a token word
    let mut prev: Vec<Option<Prev>> = vec![None; num_words + 1];

    for i in 0..num_words {
        if arrival[i] == INF {
            continue;
        }

        // Option 1: literal word (2 bytes, stays in current sequence).
        let new_cost = arrival[i] + 2;
        if new_cost < arrival[i + 1] {
            arrival[i + 1] = new_cost;
            prev[i + 1] = Some(Prev::Lit(i));
        }

        // Option 2: match — try all sublengths of each candidate.
        if let Some(cands) = &match_at[i] {
            for cand in cands {
                let (m_offset, max_len) = (cand.offset, cand.length);
                for m_len in MIN_MATCH_WORDS..=max_len {
                    let dest = i + m_len;
                    if dest > num_words {
                        break;
                    }
                    let m_cost = match_cost(m_offset, m_len, dest >= num_words);
                    let new_cost = arrival[i] + m_cost;
                    if new_cost < arrival[dest] {
                        arrival[dest] = new_cost;
                        prev[dest] = Some(Prev::Match(i, m_offset, m_len));
                    }
                }
            }
        }
    }

    // Phase 3: trace back to recover optimal parse (s4lz.py:278-285).
    let mut path: Vec<Prev> = Vec::new();
    let mut i = num_words;
    while i > 0 {
        let p = prev[i].expect("DP arrival reached with no predecessor");
        let src = match p {
            Prev::Lit(s) => s,
            Prev::Match(s, _, _) => s,
        };
        path.push(p);
        i = src;
    }
    path.reverse();

    // Phase 4: build sequences from path (s4lz.py:287-300).
    struct Seq {
        lits: Vec<(u8, u8)>,
        match_offset: usize,
        match_words: usize,
    }
    let mut sequences: Vec<Seq> = Vec::new();
    let mut literal_words: Vec<(u8, u8)> = Vec::new();
    for entry in &path {
        match *entry {
            Prev::Lit(word_idx) => {
                let pos = dict_len + word_idx * 2; // data word -> concat byte pos
                literal_words.push((work_data[pos], work_data[pos + 1]));
            }
            Prev::Match(_, m_offset, m_length) => {
                sequences.push(Seq {
                    lits: std::mem::take(&mut literal_words),
                    match_offset: m_offset,
                    match_words: m_length,
                });
            }
        }
    }
    if !literal_words.is_empty() {
        sequences.push(Seq { lits: literal_words, match_offset: 0, match_words: 0 });
    }

    // Encode sequences into the compressed stream (s4lz.py:302-338).
    let mut out = header;

    for seq in &sequences {
        let lit_count = seq.lits.len();
        let match_count = seq.match_words;

        let token = build_token(lit_count, match_count);
        let lit_nibble = (token >> 4) & 0x0F;
        let match_nibble = token & 0x0F;

        // Token word: token byte + offmark byte.
        let short_form = match_count > 0 && (2..=MAX_SHORT_OFFSET).contains(&seq.match_offset);
        let offmark: u8 = if short_form { (seq.match_offset >> 1) as u8 } else { 0 };
        out.push(token);
        out.push(offmark);

        // Literal count extension (word, when nibble == 15).
        if lit_nibble == 15 {
            out.push((lit_count >> 8) as u8);
            out.push((lit_count & 0xFF) as u8);
        }

        // Literal data words.
        for &(hi, lo) in &seq.lits {
            out.push(hi);
            out.push(lo);
        }

        // Long-form match offset (after literals), then count extension.
        if match_count > 0 {
            if !short_form {
                out.push((seq.match_offset >> 8) as u8);
                out.push((seq.match_offset & 0xFF) as u8);
            }
            if match_nibble == 15 {
                out.push((match_count >> 8) as u8);
                out.push((match_count & 0xFF) as u8);
            }
        }
    }

    // End-of-stream word.
    out.push(TOKEN_END);
    out.push(0);

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_delta_encode_empty_is_empty() {
        assert_eq!(tile_delta_encode(&[]), Vec::<u8>::new());
    }

    #[test]
    fn tile_delta_encode_first_tile_unchanged() {
        let tile: Vec<u8> = (0..32u8).collect();
        assert_eq!(tile_delta_encode(&tile), tile);
    }

    #[test]
    fn tile_delta_encode_second_tile_xored_against_first() {
        let mut data = vec![0xFFu8; 32];
        data.extend(vec![0x0Fu8; 32]);
        let out = tile_delta_encode(&data);
        assert_eq!(&out[0..32], &vec![0xFFu8; 32][..]);
        assert_eq!(&out[32..64], &vec![0xF0u8; 32][..]); // 0x0F ^ 0xFF
    }

    #[test]
    fn tile_delta_encode_handles_partial_final_tile() {
        // 40 bytes: one full tile (32B) + 8B partial second tile.
        let data: Vec<u8> = (0..40u8).collect();
        let out = tile_delta_encode(&data);
        assert_eq!(out.len(), 40);
        // Partial tile bytes XOR against the corresponding prefix of tile 1.
        for i in 32..40 {
            assert_eq!(out[i], data[i] ^ data[i - 32]);
        }
    }

    #[test]
    fn build_token_packs_nibbles() {
        assert_eq!(build_token(3, 5), 0x35);
        assert_eq!(build_token(0, 0), 0x00);
    }

    #[test]
    fn build_token_caps_at_15() {
        assert_eq!(build_token(20, 40), 0xFF);
    }

    #[test]
    fn try_compress_rejects_dict_and_tile_delta_together() {
        let opts = Options { tile_delta: true, dictionary: vec![1, 2] };
        let err = try_compress(&[1, 2, 3, 4], &opts).unwrap_err();
        assert_eq!(err, CompressError::DictTileDeltaExclusive);
    }

    #[test]
    fn try_compress_rejects_odd_dictionary_length() {
        let opts = Options::with_dictionary(vec![1, 2, 3]);
        let err = try_compress(&[1, 2, 3, 4], &opts).unwrap_err();
        assert_eq!(err, CompressError::DictLengthOdd(3));
    }

    #[test]
    fn try_compress_rejects_window_overflow() {
        let data = vec![0u8; MAX_WINDOW + 2];
        let opts = Options::default();
        let err = try_compress(&data, &opts).unwrap_err();
        assert_eq!(
            err,
            CompressError::WindowExceeded { dict_len: 0, data_len: MAX_WINDOW + 2 }
        );
    }

    #[test]
    fn try_compress_empty_input_is_header_plus_eos() {
        let out = try_compress(&[], &Options::default()).unwrap();
        assert_eq!(out, vec![0x00, 0x00, 0x00, VERSION_V3, 0x00, 0x00]);
    }

    #[test]
    #[should_panic]
    fn compress_panics_on_error() {
        let opts = Options { tile_delta: true, dictionary: vec![1, 2] };
        let _ = compress(&[1, 2, 3, 4], &opts);
    }
}
