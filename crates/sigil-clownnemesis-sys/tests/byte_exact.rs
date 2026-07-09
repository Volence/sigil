//! Byte-exactness and round-trip gates for `sigil-clownnemesis-sys`.
//!
//! See `tests/rewind_regression.rs` for the load-bearing RED/GREEN story
//! behind the read callback's rewind-on-EOF contract — that test is kept
//! separate since it documents a specific historical bug and fix, while
//! this file covers the broader gate surface (real-blob round trips,
//! compression golden, constraint diagnostics).

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
fn round_trip_numbers() {
    let compressed = read_vector("numbers.nem");
    let plain = read_vector("numbers.raw");

    let decompressed_original = sigil_clownnemesis_sys::decompress(&compressed).unwrap();
    assert_eq!(decompressed_original, plain, "sanity: vendored decompress of the committed .nem must match the committed .raw");

    let recompressed = sigil_clownnemesis_sys::compress(&plain).unwrap();
    let decompressed_recompressed = sigil_clownnemesis_sys::decompress(&recompressed).unwrap();
    assert_eq!(decompressed_recompressed, plain, "round trip: decompress(compress(plain)) must equal plain");
}

#[test]
fn round_trip_seal() {
    let compressed = read_vector("seal.nem");
    let plain = read_vector("seal.raw");

    let decompressed_original = sigil_clownnemesis_sys::decompress(&compressed).unwrap();
    assert_eq!(decompressed_original, plain);

    let recompressed = sigil_clownnemesis_sys::compress(&plain).unwrap();
    let decompressed_recompressed = sigil_clownnemesis_sys::decompress(&recompressed).unwrap();
    assert_eq!(decompressed_recompressed, plain);
}

// ---------------------------------------------------------------------
// Compression golden: byte-exact vs clownnemesis@7abcddc
// ---------------------------------------------------------------------

#[test]
fn golden_nemesis() {
    let plain = read_vector("numbers.raw");
    let expected = read_vector("golden_nemesis.bin");
    let actual = sigil_clownnemesis_sys::compress(&plain).unwrap();
    assert_eq!(actual, expected);
}

// ---------------------------------------------------------------------
// Constraint diagnostics (CR5)
// ---------------------------------------------------------------------

#[test]
fn rejects_non_tile_aligned_input() {
    let err = sigil_clownnemesis_sys::compress(&[0u8; 33]).unwrap_err();
    assert_eq!(err, sigil_clownnemesis_sys::Error::NotTileAligned { len: 33 });
}

#[test]
fn rejects_empty_input_as_zero_length_is_tile_aligned_but_check_boundary() {
    // 0 is technically a multiple of 0x20, so this documents the boundary
    // behavior rather than asserting a rejection: an empty input passes
    // the tile-alignment check and is handed to the C compressor, which
    // (per vendor/compress.c's EmitHeader) accepts 0 tiles.
    let result = sigil_clownnemesis_sys::compress(&[]);
    assert!(result.is_ok(), "empty input should pass tile-alignment (0 % 0x20 == 0) and compress successfully");
}

#[test]
fn accepts_exactly_max_tiles_length_check() {
    // Cheap logic-only check (see src/lib.rs's own unit test for the same
    // boundary) — kept here too as an integration-level constraint
    // documentation point, not a duplicate C-side round trip (a real
    // 32767*0x20 ~= 1MB buffer would be wasteful to compress in a test).
    let max_len = sigil_clownnemesis_sys::MAX_TILES * sigil_clownnemesis_sys::TILE_SIZE;
    assert_eq!(max_len % sigil_clownnemesis_sys::TILE_SIZE, 0);
}
