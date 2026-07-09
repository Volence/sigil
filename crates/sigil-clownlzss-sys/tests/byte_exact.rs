//! Byte-exactness and round-trip gates for `sigil-clownlzss-sys`.
//!
//! Two kinds of gates:
//!   1. **Round-trip vs real blobs**: vendored-decompress(original) ->
//!      our-compress -> vendored-decompress(recompressed) -> equal the
//!      plain bytes from the first decompress. See `tests/vectors/PROVENANCE.md`
//!      for where each blob came from.
//!   2. **Compression goldens**: compress a fixed small input and compare
//!      against a committed reference byte string, captured directly from
//!      the vendored `clownlzss` C++ templates by a standalone driver during
//!      test-vector preparation (see `tests/vectors/PROVENANCE.md`). These
//!      are "byte-exact vs clownlzss@8055bd2", NOT vs Sega's own encoders.
//!
//! IMPORTANT CAVEAT discovered during recon (see `src/shim.cpp`'s big
//! comment for the full story): the vendored decompressor's overlapping-copy
//! path (`DecompressorOutput::Copy` for a raw-pointer/random-access output
//! iterator) is broken against `std::copy`-as-`memmove` semantics on modern
//! standard libraries. The shim works around this by using
//! `std::ostringstream` for all decompression (a byte-by-byte, overlap-safe
//! code path also provided by the vendored headers) — so
//! `sigil_clownlzss_sys::decompress_*` is correct. All `.raw` reference
//! payloads in `tests/vectors/` were regenerated with the corrected
//! ostream-based decoder and cross-checked against an independent,
//! unrelated Kosinski implementation (`programs/accurate-kosinski`
//! elsewhere in this workspace) before being committed.

use std::path::{Path, PathBuf};

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors")
}

fn read_vector(name: &str) -> Vec<u8> {
    std::fs::read(vectors_dir().join(name)).unwrap_or_else(|e| panic!("read {name}: {e}"))
}

// ---------------------------------------------------------------------
// Round-trip vs real blobs
// ---------------------------------------------------------------------

#[test]
fn kosinski_round_trip_oozprimary() {
    let compressed = read_vector("oozprimary.kos");
    let plain = read_vector("oozprimary.raw");

    let decompressed_original = sigil_clownlzss_sys::decompress_kosinski(&compressed).unwrap();
    assert_eq!(decompressed_original, plain, "sanity: vendored decompress of the committed .kos must match the committed .raw");

    let recompressed = sigil_clownlzss_sys::compress_kosinski(&plain).unwrap();
    let decompressed_recompressed = sigil_clownlzss_sys::decompress_kosinski(&recompressed).unwrap();
    assert_eq!(decompressed_recompressed, plain, "round trip: decompress(compress(plain)) must equal plain");
}

#[test]
fn kosinski_round_trip_specstag() {
    let compressed = read_vector("specstag.kos");
    let plain = read_vector("specstag.raw");

    let decompressed_original = sigil_clownlzss_sys::decompress_kosinski(&compressed).unwrap();
    assert_eq!(decompressed_original, plain);

    let recompressed = sigil_clownlzss_sys::compress_kosinski(&plain).unwrap();
    let decompressed_recompressed = sigil_clownlzss_sys::decompress_kosinski(&recompressed).unwrap();
    assert_eq!(decompressed_recompressed, plain);
}

#[test]
fn kosinski_moduled_round_trip_sand_particles() {
    let compressed = read_vector("sand_particles.kosm");
    let plain = read_vector("sand_particles.raw");

    let decompressed_original = sigil_clownlzss_sys::decompress_kosinski_moduled(&compressed).unwrap();
    assert_eq!(decompressed_original, plain);

    let recompressed = sigil_clownlzss_sys::compress_kosinski_moduled(&plain, 0x1000).unwrap();
    let decompressed_recompressed = sigil_clownlzss_sys::decompress_kosinski_moduled(&recompressed).unwrap();
    assert_eq!(decompressed_recompressed, plain);
}

#[test]
fn kosinski_moduled_round_trip_ship_propeller() {
    let compressed = read_vector("ship_propeller.kosm");
    let plain = read_vector("ship_propeller.raw");

    let decompressed_original = sigil_clownlzss_sys::decompress_kosinski_moduled(&compressed).unwrap();
    assert_eq!(decompressed_original, plain);

    let recompressed = sigil_clownlzss_sys::compress_kosinski_moduled(&plain, 0x1000).unwrap();
    let decompressed_recompressed = sigil_clownlzss_sys::decompress_kosinski_moduled(&recompressed).unwrap();
    assert_eq!(decompressed_recompressed, plain);
}

#[test]
fn enigma_round_trip_level_select_2p() {
    let compressed = read_vector("level_select_2p.eni");
    let plain = read_vector("level_select_2p.raw");

    let decompressed_original = sigil_clownlzss_sys::decompress_enigma(&compressed).unwrap();
    assert_eq!(decompressed_original, plain);

    let recompressed = sigil_clownlzss_sys::compress_enigma(&plain).unwrap();
    let decompressed_recompressed = sigil_clownlzss_sys::decompress_enigma(&recompressed).unwrap();
    assert_eq!(decompressed_recompressed, plain);
}

/// Saxman: no committed compressed Saxman blob was found anywhere in this
/// workspace (s2disasm's SMPS sound driver/songs are Saxman-compressed only
/// as a *build-time* step via an external `saxman` CLI tool invoked from
/// `s2disasm/build.lua` — the compressed bytes are a transient build
/// artifact, never committed; searched `s2disasm/sound/**`,
/// `skdisasm/Sound/**`, and `sonic_hack/sound/**` for any `.bin` that could
/// plausibly be a compiled Saxman stream and found only DAC/PCM samples and
/// raw uncompressed Z80 driver dumps). Per the plan's own allowance, this
/// falls back to a synthetic round-trip: compress a small hand-built buffer
/// derived from a real decompressed asset, decompress it back, compare.
#[test]
fn saxman_synthetic_round_trip() {
    let plain = read_vector("level_select_2p.raw");

    let with_header = sigil_clownlzss_sys::compress_saxman(&plain, true).unwrap();
    let decompressed = sigil_clownlzss_sys::decompress_saxman_with_header(&with_header).unwrap();
    assert_eq!(decompressed, plain);

    let without_header = sigil_clownlzss_sys::compress_saxman(&plain, false).unwrap();
    let decompressed2 = sigil_clownlzss_sys::decompress_saxman_no_header(&without_header, without_header.len()).unwrap();
    assert_eq!(decompressed2, plain);
}

/// Comper and Rocket: no real committed blobs identified for these formats
/// (they are far less common than Kosinski/Enigma/Saxman in the disasm
/// projects available); synthetic round-trip only, per the plan's explicit
/// allowance for these two formats.
#[test]
fn comper_synthetic_round_trip() {
    let plain = read_vector("level_select_2p.raw"); // already word-even (408 bytes)
    let compressed = sigil_clownlzss_sys::compress_comper(&plain).unwrap();
    let decompressed = sigil_clownlzss_sys::decompress_comper(&compressed).unwrap();
    assert_eq!(decompressed, plain);
}

#[test]
fn rocket_synthetic_round_trip() {
    let plain = read_vector("level_select_2p.raw");
    let compressed = sigil_clownlzss_sys::compress_rocket(&plain).unwrap();
    let decompressed = sigil_clownlzss_sys::decompress_rocket(&compressed).unwrap();
    assert_eq!(decompressed, plain);
}

#[test]
fn kosplus_synthetic_round_trip() {
    // Kosinski+ has no dedicated real-asset gate in this crate (kosinski
    // plain/moduled and enigma cover the "real blob" requirement); this is
    // an additional round-trip sanity check using a real decompressed asset
    // as input, going a bit beyond the plan's bar.
    let plain = read_vector("oozprimary.raw");
    let compressed = sigil_clownlzss_sys::compress_kosplus(&plain).unwrap();
    let decompressed = sigil_clownlzss_sys::decompress_kosplus(&compressed).unwrap();
    assert_eq!(decompressed, plain);
}

// ---------------------------------------------------------------------
// Compression goldens: byte-exact vs clownlzss@8055bd2
// ---------------------------------------------------------------------
//
// Each golden was captured by a standalone driver linked directly against
// the vendored templates (see tests/vectors/PROVENANCE.md), compressing
// `level_select_2p.raw` (408 bytes, word-even, derived from a real Enigma
// asset). These are regression vectors: "byte-exact vs clownlzss@8055bd2",
// NOT byte-exact vs Sega's own compressors (per CR3 — these are OPTIMAL
// compressors, not Sega-accurate).

#[test]
fn golden_kosinski() {
    let plain = read_vector("level_select_2p.raw");
    let expected = read_vector("golden_kosinski.bin");
    let actual = sigil_clownlzss_sys::compress_kosinski(&plain).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn golden_kosplus() {
    let plain = read_vector("level_select_2p.raw");
    let expected = read_vector("golden_kosplus.bin");
    let actual = sigil_clownlzss_sys::compress_kosplus(&plain).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn golden_saxman_with_header() {
    let plain = read_vector("level_select_2p.raw");
    let expected = read_vector("golden_saxman_header.bin");
    let actual = sigil_clownlzss_sys::compress_saxman(&plain, true).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn golden_saxman_without_header() {
    let plain = read_vector("level_select_2p.raw");
    let expected = read_vector("golden_saxman_noheader.bin");
    let actual = sigil_clownlzss_sys::compress_saxman(&plain, false).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn golden_enigma() {
    let plain = read_vector("level_select_2p.raw");
    let expected = read_vector("golden_enigma.bin");
    let actual = sigil_clownlzss_sys::compress_enigma(&plain).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn golden_comper() {
    let plain = read_vector("level_select_2p.raw");
    let expected = read_vector("golden_comper.bin");
    let actual = sigil_clownlzss_sys::compress_comper(&plain).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn golden_rocket() {
    let plain = read_vector("level_select_2p.raw");
    let expected = read_vector("golden_rocket.bin");
    let actual = sigil_clownlzss_sys::compress_rocket(&plain).unwrap();
    assert_eq!(actual, expected);
}

// ---------------------------------------------------------------------
// Max-compressed-size bound sanity check
// ---------------------------------------------------------------------

#[test]
fn compressed_size_stays_within_bound() {
    // Every real test input's compressed size must stay comfortably under
    // the non-moduled max_compressed_size bound (input + input/8 + 64),
    // which sizes every non-moduled compress_* function's initial buffer.
    // (Moduled compression uses a module-count-scaled bound, and both are
    // backed by the shim's capacity check + exact-size retry — see
    // tests/moduled_capacity.rs and the src/lib.rs module doc.)
    for name in ["oozprimary.raw", "specstag.raw", "sand_particles.raw", "ship_propeller.raw", "level_select_2p.raw"] {
        let plain = read_vector(name);
        let bound = plain.len() + plain.len() / 8 + 64;
        let kos = sigil_clownlzss_sys::compress_kosinski(&plain).unwrap();
        assert!(kos.len() <= bound, "{name}: kosinski {} exceeds bound {bound}", kos.len());
    }
}

// ---------------------------------------------------------------------
// Constraint diagnostics (CR5)
// ---------------------------------------------------------------------

#[test]
fn kosinski_moduled_rejects_module_size_over_0x1000() {
    let plain = read_vector("level_select_2p.raw");
    let err = sigil_clownlzss_sys::compress_kosinski_moduled(&plain, 0x1001).unwrap_err();
    assert_eq!(err, sigil_clownlzss_sys::Error::ModuleSizeTooLarge { requested: 0x1001, max: sigil_clownlzss_sys::MAX_MODULE_SIZE });
}

#[test]
fn enigma_rejects_non_word_even_input() {
    let err = sigil_clownlzss_sys::compress_enigma(&[1, 2, 3]).unwrap_err();
    assert_eq!(err, sigil_clownlzss_sys::Error::NotWordEven { len: 3 });
}

#[test]
fn comper_rejects_non_word_even_input() {
    let err = sigil_clownlzss_sys::compress_comper(&[1, 2, 3]).unwrap_err();
    assert_eq!(err, sigil_clownlzss_sys::Error::NotWordEven { len: 3 });
}

/// Saxman-with-header's compressed size must fit in the 2-byte LE header
/// field (u16), checked AFTER compression per CR5. Constructing a real
/// input that compresses to over 65535 bytes would need a multi-hundred-KB
/// buffer; instead this exercises the check directly against a large,
/// highly-incompressible (pseudo-random, so Saxman can't find matches)
/// input sized so its *compressed* form (not just its raw form) crosses
/// the u16 boundary.
#[test]
fn saxman_with_header_rejects_compressed_size_over_u16() {
    // Saxman's worst case is close to 1:1 (a literal costs 9 bits for 8 bits
    // of data - about 12.5% growth), so an input a bit over 65535 bytes of
    // incompressible data comfortably pushes the compressed size over u16
    // once the header's own 2 bytes are added in.
    let mut plain = Vec::with_capacity(70_000);
    let mut state: u32 = 0x2463_5910;
    for _ in 0..70_000 {
        // Small xorshift PRNG: incompressible-enough that Saxman (window
        // 0x1000, min match 3) finds few or no profitable matches.
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        plain.push((state & 0xFF) as u8);
    }
    let err = sigil_clownlzss_sys::compress_saxman(&plain, true).unwrap_err();
    match err {
        sigil_clownlzss_sys::Error::CompressedSizeExceedsU16 { actual } => {
            assert!(actual > u16::MAX as usize, "expected actual > u16::MAX, got {actual}");
        }
        other => panic!("expected CompressedSizeExceedsU16, got {other:?}"),
    }
}
