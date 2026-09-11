//! The two authentic compressors `p2bin` uses for its `kosinski` and `saxman`
//! formats, in pure Rust.
//!
//! These are not clownlzss. clownlzss parses optimally and writes smaller
//! streams; these reproduce the greedy output of the original Sega tools byte
//! for byte, mistakes included, because a disassembly's reference ROM holds
//! exactly those bytes. `p2bin` names the pair "Kosinski (authentic)" and
//! "Saxman (authentic)" in its own help text, and this module uses its words.
//!
//! Ported from:
//!
//! * Kosinski: Clownacy's accurate-kosinski, `lib/kosinski-compress.c`
//!   (revision `45abe26a`, the checkout at `programs/accurate-kosinski` in this
//!   workspace). Copyright (c) 2018-2023 Clownacy, distributed under a
//!   permission notice that allows use, copying, modification and distribution
//!   for any purpose.
//! * Saxman: Haruhiko Okumura's 1989 `LZSS.C` ("Use, distribute, and modify
//!   this program freely"), as modified by Clownacy for the Saxman format and
//!   shipped in `s2disasm/build_tools/source_code/lz_comp2/`.
//!
//! The sources are where the algorithm came from, not the evidence that it is
//! right. Every expected stream in `tests/accurate_vectors.rs` was produced by
//! the `p2bin` binary itself (md5 `4f2fff99c3347bafb93b12d5be1db754`).

/// Kosinski's sliding window.
const KOS_WINDOW: usize = 0x2000;
/// The longest match Sega's compressor looks for. The format allows 0x100.
const KOS_MAX_MATCH: usize = 0xFD;
/// The farthest match Sega's compressor looks for. The format allows the whole
/// window.
const KOS_MAX_DISTANCE: usize = KOS_WINDOW - KOS_MAX_MATCH;

/// Kosinski's output: 16-bit descriptor fields, each written little-endian ahead
/// of the match bytes it describes.
struct KosWriter {
    out: Vec<u8>,
    pending: Vec<u8>,
    descriptor: u32,
    bits_left: u32,
}

impl KosWriter {
    fn flush(&mut self) {
        self.descriptor >>= self.bits_left;
        self.out.push(self.descriptor as u8);
        self.out.push((self.descriptor >> 8) as u8);
        self.out.extend_from_slice(&self.pending);
    }

    /// One descriptor bit. The sixteenth completes the field, which is written
    /// with the match bytes queued so far, before any byte queued after this bit.
    fn bit(&mut self, bit: bool) {
        self.descriptor >>= 1;
        self.descriptor |= u32::from(bit) << 15;
        self.bits_left -= 1;
        if self.bits_left == 0 {
            self.flush();
            self.bits_left = 16;
            self.pending.clear();
        }
    }

    fn byte(&mut self, byte: u8) {
        self.pending.push(byte);
    }
}

/// Compress `data` the way Sega's Kosinski compressor did: a greedy longest-match
/// search, a dummy match before the first match that starts past each 0xA000
/// bytes, a terminator, and zero padding to a multiple of 16 bytes.
pub fn compress_kosinski_authentic(data: &[u8]) -> Vec<u8> {
    let mut input = data.iter().copied();
    // Zero-filled, which is the state the original leaves the unread part of its
    // ring buffer in, and which the end-of-input match search reads.
    let mut ring = vec![0u8; KOS_WINDOW + KOS_MAX_MATCH - 1];
    let mut w = KosWriter { out: Vec::new(), pending: Vec::new(), descriptor: 0, bits_left: 16 };
    let mut write_index = 0usize;
    let mut read_index = 0usize;
    let mut since_boundary = 0usize;

    while write_index < KOS_MAX_MATCH {
        match input.next() {
            Some(b) => {
                ring[write_index] = b;
                write_index += 1;
            }
            None => break,
        }
    }

    while read_index != write_index {
        let max_distance = read_index.min(KOS_MAX_DISTANCE);
        let cur = read_index % KOS_WINDOW;
        let mut best_back = 0usize;
        let mut best_len = 0usize;
        for back in 1..=max_distance {
            let prev = (read_index - back) % KOS_WINDOW;
            // Counts up to the maximum even past the end of the input, reading
            // whatever the ring buffer still holds there.
            let mut len = 0usize;
            while len < KOS_MAX_MATCH && ring[cur + len] == ring[prev + len] {
                len += 1;
            }
            if len > best_len {
                best_back = back;
                best_len = len;
            }
        }
        best_len = best_len.min(write_index - read_index);

        if since_boundary >= 0xA000 {
            since_boundary %= 0xA000;
            w.bit(false);
            w.bit(true);
            w.byte(0x00);
            w.byte(0xF0);
            w.byte(0x01);
        }

        let distance = (best_back as u64).wrapping_neg();
        if (2..=5).contains(&best_len) && best_back < 0x100 {
            let length = best_len - 2;
            w.bit(false);
            w.bit(false);
            w.bit((length & 2) != 0);
            w.bit((length & 1) != 0);
            w.byte(distance as u8);
        } else if (3..=9).contains(&best_len) {
            w.bit(false);
            w.bit(true);
            w.byte(distance as u8);
            w.byte((((distance >> 5) & 0xF8) as u8) | (((best_len - 2) & 7) as u8));
        } else if best_len >= 3 {
            w.bit(false);
            w.bit(true);
            w.byte(distance as u8);
            w.byte(((distance >> 5) & 0xF8) as u8);
            w.byte((best_len - 1) as u8);
        } else {
            best_len = 1;
            w.bit(true);
            w.byte(ring[cur]);
        }

        for _ in 0..best_len {
            let Some(b) = input.next() else { break };
            let ri = write_index % KOS_WINDOW;
            write_index += 1;
            ring[ri] = b;
            // The spill area past the window mirrors its start, so a comparison
            // never wraps.
            if ri < KOS_MAX_MATCH - 1 {
                ring[KOS_WINDOW + ri] = b;
            }
        }
        read_index += best_len;
        since_boundary += best_len;
    }

    w.bit(false);
    w.bit(true);
    w.byte(0x00);
    w.byte(0xF0);
    w.byte(0x00);
    w.flush();
    while !w.out.len().is_multiple_of(0x10) {
        w.out.push(0);
    }
    w.out
}

/// Saxman's ring buffer.
const LZ_N: usize = 4096;
/// The longest match.
const LZ_F: usize = 18;
/// A match must be longer than this to be encoded as one.
const LZ_THRESHOLD: usize = 2;
/// The "no node" index of the binary search trees.
const LZ_NIL: usize = LZ_N;

/// Okumura's binary-search-tree match finder over the ring buffer.
struct Lzss {
    text: Vec<u8>,
    lson: Vec<usize>,
    rson: Vec<usize>,
    dad: Vec<usize>,
    match_position: usize,
    match_length: usize,
}

impl Lzss {
    fn new() -> Lzss {
        let mut t = Lzss {
            text: vec![0; LZ_N + LZ_F - 1],
            lson: vec![0; LZ_N + 257],
            rson: vec![0; LZ_N + 257],
            dad: vec![0; LZ_N + 257],
            match_position: 0,
            match_length: 0,
        };
        for i in LZ_N + 1..=LZ_N + 256 {
            t.rson[i] = LZ_NIL;
        }
        for i in 0..LZ_N {
            t.dad[i] = LZ_NIL;
        }
        t
    }

    /// Insert the string at `r` into its tree, leaving the longest match found
    /// on the way in `match_position` / `match_length`. A full-length match
    /// replaces the older node.
    fn insert_node(&mut self, r: usize) {
        let mut cmp: i32 = 1;
        let mut p = LZ_N + 1 + self.text[r] as usize;
        self.rson[r] = LZ_NIL;
        self.lson[r] = LZ_NIL;
        self.match_length = 0;
        loop {
            if cmp >= 0 {
                if self.rson[p] != LZ_NIL {
                    p = self.rson[p];
                } else {
                    self.rson[p] = r;
                    self.dad[r] = p;
                    return;
                }
            } else if self.lson[p] != LZ_NIL {
                p = self.lson[p];
            } else {
                self.lson[p] = r;
                self.dad[r] = p;
                return;
            }
            let mut i = 1;
            while i < LZ_F {
                cmp = i32::from(self.text[r + i]) - i32::from(self.text[p + i]);
                if cmp != 0 {
                    break;
                }
                i += 1;
            }
            if i > self.match_length {
                self.match_position = p;
                self.match_length = i;
                if i >= LZ_F {
                    break;
                }
            }
        }
        self.dad[r] = self.dad[p];
        self.lson[r] = self.lson[p];
        self.rson[r] = self.rson[p];
        let (lp, rp) = (self.lson[p], self.rson[p]);
        self.dad[lp] = r;
        self.dad[rp] = r;
        let dp = self.dad[p];
        if self.rson[dp] == p {
            self.rson[dp] = r;
        } else {
            self.lson[dp] = r;
        }
        self.dad[p] = LZ_NIL;
    }

    fn delete_node(&mut self, p: usize) {
        if self.dad[p] == LZ_NIL {
            return;
        }
        let q = if self.rson[p] == LZ_NIL {
            self.lson[p]
        } else if self.lson[p] == LZ_NIL {
            self.rson[p]
        } else {
            let mut q = self.lson[p];
            if self.rson[q] != LZ_NIL {
                loop {
                    q = self.rson[q];
                    if self.rson[q] == LZ_NIL {
                        break;
                    }
                }
                let (dq, lq) = (self.dad[q], self.lson[q]);
                self.rson[dq] = lq;
                self.dad[lq] = dq;
                let lp = self.lson[p];
                self.lson[q] = lp;
                self.dad[lp] = q;
            }
            let rp = self.rson[p];
            self.rson[q] = rp;
            self.dad[rp] = q;
            q
        };
        self.dad[q] = self.dad[p];
        let dp = self.dad[p];
        if self.rson[dp] == p {
            self.rson[dp] = q;
        } else {
            self.lson[dp] = q;
        }
        self.dad[p] = LZ_NIL;
    }
}

/// Compress `data` the way Sega's Saxman compressor did (Okumura's 1989 LZSS
/// with Saxman's field layout), with no size header.
pub fn compress_saxman_authentic(data: &[u8]) -> Vec<u8> {
    let mut input = data.iter().copied();
    let mut t = Lzss::new();
    let mut out = Vec::new();
    // code[0] holds eight flags, 1 for a literal; the rest the units they flag.
    let mut code = [0u8; 17];
    let mut code_len = 1usize;
    let mut mask: u8 = 1;
    let mut s = 0usize;
    let mut r = LZ_N - LZ_F;

    let mut len = 0usize;
    while len < LZ_F {
        let Some(c) = input.next() else { break };
        t.text[r + len] = c;
        len += 1;
    }
    if len == 0 {
        return out;
    }
    for i in 1..=LZ_F {
        t.insert_node(r - i);
    }
    t.insert_node(r);
    loop {
        if t.match_length > len {
            t.match_length = len;
        }
        if t.match_length <= LZ_THRESHOLD {
            t.match_length = 1;
            code[0] |= mask;
            code[code_len] = t.text[r];
            code_len += 1;
        } else {
            code[code_len] = t.match_position as u8;
            code[code_len + 1] =
                (((t.match_position >> 4) & 0xF0) | (t.match_length - (LZ_THRESHOLD + 1))) as u8;
            code_len += 2;
        }
        mask = mask.wrapping_shl(1);
        if mask == 0 {
            out.extend_from_slice(&code[..code_len]);
            code[0] = 0;
            code_len = 1;
            mask = 1;
        }
        let last = t.match_length;
        let mut i = 0;
        while i < last {
            let Some(c) = input.next() else { break };
            t.delete_node(s);
            t.text[s] = c;
            if s < LZ_F - 1 {
                t.text[s + LZ_N] = c;
            }
            s = (s + 1) & (LZ_N - 1);
            r = (r + 1) & (LZ_N - 1);
            t.insert_node(r);
            i += 1;
        }
        while i < last {
            i += 1;
            t.delete_node(s);
            s = (s + 1) & (LZ_N - 1);
            r = (r + 1) & (LZ_N - 1);
            len -= 1;
            if len != 0 {
                t.insert_node(r);
            }
        }
        if len == 0 {
            break;
        }
    }
    if code_len > 1 {
        out.extend_from_slice(&code[..code_len]);
    }
    out
}

/// [`compress_saxman_authentic`] followed by the one byte `p2bin`'s
/// `saxman-bugged` format appends: `0x4E` after a stream of odd length, `0x00`
/// after an even one.
pub fn compress_saxman_bugged(data: &[u8]) -> Vec<u8> {
    let mut out = compress_saxman_authentic(data);
    let junk = if out.len() % 2 == 1 { 0x4E } else { 0x00 };
    out.push(junk);
    out
}
