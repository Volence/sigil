//! RED/GREEN regression test for the "input callback must rewind on EOF"
//! contract (see `vendor/VENDOR.md`).
//!
//! `ClownNemesis_Compress` is a multi-pass encoder: `ComputeCodes`
//! (`vendor/compress.c`) runs `FindRuns` (which drives the read callback
//! through a full pass over the input) in regular mode, then XOR mode, and
//! then — if regular mode won — a third time in regular mode again;
//! `EmitCodes` then runs `FindRuns` a FOURTH time to actually emit the
//! bitstream. `ReadByte` (`vendor/common-internal.c`) does NOT longjmp on
//! `CLOWNNEMESIS_EOF` during compression (`throw_on_eof = cc_false`) — it
//! just returns the sentinel value the read callback gave it. This means:
//! after the read callback has returned EOF once, `ClownNemesis_Compress`
//! WILL call it again, expecting a fresh pass from position 0.
//!
//! RED (recon-verified BEFORE this test existed, and reproduced here): a
//! naive read callback that exhausts its input once and then always
//! returns EOF causes `ClownNemesis_Compress` to report SUCCESS
//! (`ok=1`/`Ok(...)`) while silently emitting a severely truncated stream
//! — 3 bytes instead of the correct ~218 bytes for a 576-byte real tile
//! buffer (`tests/vectors/numbers.raw`). This is the dangerous case: no
//! error, just silently wrong output.
//!
//! This test proves the fix: `sigil_clownnemesis_sys::compress`'s read
//! callback (`src/lib.rs::read_cb`) rewinds `ReadContext::pos` to 0 every
//! time it is asked to read past the end, so every pass sees the same
//! data. The gate: compress a real (non-trivial, larger than the 3-byte
//! truncation artifact) input, decompress the result with the vendored
//! decompressor, and assert the round trip reproduces the original input
//! exactly — a truncated compress would either fail the vendored
//! decompressor outright or reproduce something much shorter than the
//! original, so this assertion cannot pass by accident.

use std::path::{Path, PathBuf};

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors")
}

fn read_vector(name: &str) -> Vec<u8> {
    std::fs::read(vectors_dir().join(name)).unwrap_or_else(|e| panic!("read {name}: {e}"))
}

#[test]
fn compress_round_trip_is_not_truncated() {
    // numbers.raw is 576 bytes (18 tiles) — comfortably bigger than the
    // 3-byte truncation artifact the naive non-rewinding callback produces,
    // so this test cannot pass by coincidence.
    let plain = read_vector("numbers.raw");
    assert_eq!(plain.len(), 576, "sanity: test fixture size assumption");

    let compressed = sigil_clownnemesis_sys::compress(&plain).expect("compress must succeed");

    // The truncation bug reports success with ~3 bytes of output; a
    // correct compression of 576 bytes of real (non-degenerate) tile data
    // is far larger than that. This assertion is the direct RED/GREEN
    // signal: it FAILS under the naive non-rewinding callback (3 bytes)
    // and PASSES under the rewinding one (recon measured 218 bytes).
    assert!(
        compressed.len() > 50,
        "compressed output ({} bytes) looks truncated — the read callback likely did not rewind on EOF",
        compressed.len()
    );

    let decompressed = sigil_clownnemesis_sys::decompress(&compressed).expect("decompress must succeed");
    assert_eq!(decompressed, plain, "round trip: decompress(compress(plain)) must equal plain exactly");
}
