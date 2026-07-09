//! Regression gate for the moduled-compression heap overflow (quality
//! review of Plan-7 #10 T2a, CRITICAL finding).
//!
//! ## The bug (RED evidence)
//!
//! The original shim's `RunCompress` had no `out_capacity` parameter and no
//! bounds check — safety rested entirely on the Rust-side worst-case bound,
//! which for moduled compression was the flat single-stream bound
//! (`len + len/8 + 64`) plus a flat `+16`. But
//! `ModuledCompressionWrapper` emits `k` independent streams, each with its
//! own ~5-byte worst-case terminator overhead plus up to 15 bytes of 0x10
//! alignment padding (Kosinski) — overhead that scales with module count
//! while the `+16` slack did not. For the input below (32768 incompressible
//! bytes, k = 8 modules of 0x1000), the bound allowed 36944 bytes but the
//! true output is ~36994 bytes: a ~50-byte heap buffer overrun.
//!
//! Recorded RED run (pre-fix, the `moduled_incompressible_k8_round_trips`
//! repro below, run against the unfixed shim on 2026-07-09):
//! ```text
//! double free or corruption (out)
//! error: test failed, ... (signal: 6, SIGABRT: process abort signal)
//! ```
//!
//! ## The fix (both layers)
//!
//! 1. The shim compresses into an `std::ostringstream` (upstream's own
//!    primary output path — the clownlzss CLI compresses straight into
//!    `std::ofstream`) and only copies into the caller's buffer after an
//!    explicit capacity check, so the shim can no longer write past the
//!    buffer *regardless* of what capacity Rust passes. On a too-small
//!    buffer it returns `-1` with the exact required size, and the Rust
//!    wrapper retries once with an exactly-sized buffer.
//! 2. The Rust-side moduled bound now scales with module count:
//!    `len + len/8 + num_modules*16 + 64` (`num_modules = ceil(len /
//!    module_size)`). Together with the new moduled-header representability
//!    check (`num_modules <= 16`), this bound provably covers the true
//!    worst case (~20 bytes/module of terminator+padding overhead:
//!    `16k + 64 >= 20k - 13` for all `k <= 19`), so the retry path is a
//!    defensive fallback, not the normal path.

use sigil_clownlzss_sys::Error;

/// Prefix of the de Bruijn sequence B(256, 2) (FKM construction, 65536
/// bytes total): every 2-byte pair appears at most once anywhere in the
/// sequence, so no LZSS match of length >= 2 exists — a *guaranteed*
/// all-literals worst case, not merely a statistical one. (A first draft
/// used an xorshift byte stream, which Kosinski still shaved ~0.3% off —
/// 85 bytes short of triggering the overflow. De Bruijn is exact.)
fn incompressible(len: usize) -> Vec<u8> {
    assert!(len <= 0x10000, "B(256,2) is only 65536 bytes long");
    fn db(t: usize, p: usize, a: &mut [usize; 3], seq: &mut Vec<u8>) {
        const K: usize = 256;
        const N: usize = 2;
        if t > N {
            if N.is_multiple_of(p) {
                for &v in &a[1..=p] {
                    seq.push(v as u8);
                }
            }
        } else {
            a[t] = a[t - p];
            db(t + 1, p, a, seq);
            for j in (a[t - p] + 1)..K {
                a[t] = j;
                db(t + 1, t, a, seq);
            }
        }
    }
    let mut seq = Vec::with_capacity(0x10000);
    db(1, 1, &mut [0usize; 3], &mut seq);
    seq.truncate(len);
    seq
}

/// THE reviewer repro: incompressible input, k = 8 modules of 0x1000.
/// Pre-fix this aborted the process with heap corruption (see module doc);
/// post-fix it must round-trip cleanly.
#[test]
fn moduled_incompressible_k8_round_trips() {
    let plain = incompressible(0x8000); // 32768 bytes = 8 modules of 0x1000
    let compressed = sigil_clownlzss_sys::compress_kosinski_moduled(&plain, 0x1000).unwrap();
    let decompressed = sigil_clownlzss_sys::decompress_kosinski_moduled(&compressed).unwrap();
    assert_eq!(decompressed, plain);
}

/// Same shape across the other moduled formats (all shared the flat-bound
/// bug through the same helper): each must round-trip incompressible
/// multi-module input without corruption.
#[test]
fn moduled_incompressible_other_formats_round_trip() {
    let plain = incompressible(0x8000);

    let kp = sigil_clownlzss_sys::compress_kosplus_moduled(&plain, 0x1000).unwrap();
    assert_eq!(sigil_clownlzss_sys::decompress_kosplus_moduled(&kp).unwrap(), plain);

    // Comper requires word-even input; 0x8000 is. (Comper/Rocket/Saxman
    // moduled have no decompress-moduled counterparts in the vendored set
    // worth gating here; compressing without heap corruption and staying
    // within the documented bound is the property under test.)
    let bound = |len: usize, module_size: usize| len + len / 8 + len.div_ceil(module_size) * 16 + 64;
    let cp = sigil_clownlzss_sys::compress_comper_moduled(&plain, 0x1000).unwrap();
    assert!(cp.len() <= bound(plain.len(), 0x1000), "comper moduled {} exceeds bound", cp.len());
    let rk = sigil_clownlzss_sys::compress_rocket_moduled(&plain, 0x1000).unwrap();
    assert!(rk.len() <= bound(plain.len(), 0x1000), "rocket moduled {} exceeds bound", rk.len());
}

/// The moduled bound formula must cover the true output for the worst-case
/// repro input (this is the arithmetic the CRITICAL fix rests on).
#[test]
fn moduled_bound_covers_incompressible_output() {
    let plain = incompressible(0x8000);
    let num_modules = plain.len().div_ceil(0x1000);
    let bound = plain.len() + plain.len() / 8 + num_modules * 16 + 64;
    let compressed = sigil_clownlzss_sys::compress_kosinski_moduled(&plain, 0x1000).unwrap();
    assert!(
        compressed.len() <= bound,
        "kosinski moduled output {} exceeds the documented bound {bound}",
        compressed.len()
    );
    // And the flat bound genuinely does NOT cover it — i.e. this test would
    // be vacuous if the old bound had been sufficient all along.
    let old_flat_bound = plain.len() + plain.len() / 8 + 64 + 16;
    assert!(
        compressed.len() > old_flat_bound,
        "repro no longer exercises the old overflow: output {} fits the old bound {old_flat_bound}",
        compressed.len()
    );
}

// ---------------------------------------------------------------------
// New CR5-style constraint diagnostics introduced alongside the fix
// ---------------------------------------------------------------------

/// `module_size == 0` would be `data_size % 0` (UB) inside the vendored
/// `ModuledCompressionWrapper` — must be a typed error, never reach C.
#[test]
fn moduled_rejects_module_size_zero() {
    let err = sigil_clownlzss_sys::compress_kosinski_moduled(&[0u8; 8], 0).unwrap_err();
    assert_eq!(err, Error::ModuleSizeZero);
}

/// The 16-bit moduled header stores `len % module_size` in its low 12 bits
/// and `len / module_size` in its high 4 bits: a quotient above 15 silently
/// truncates upstream, emitting a corrupt stream. Must be a typed error.
#[test]
fn moduled_rejects_data_too_large_for_header() {
    // Same boundary logic at every module_size; 0x100 keeps the accepted
    // boundary case small (0xFFF bytes) — the optimal parser is very slow
    // on multi-tens-of-KB runs in debug builds (the 0x1000 version of this
    // test took ~100s), and the length check itself is pure arithmetic.
    let max = 16 * 0x100 - 1; // largest representable len for module_size 0x100
    let err = sigil_clownlzss_sys::compress_kosinski_moduled(&vec![0u8; max + 1], 0x100).unwrap_err();
    assert_eq!(err, Error::DataTooLargeForModuled { len: max + 1, max });

    // Boundary: exactly the maximum representable length is accepted and
    // round-trips.
    let plain = vec![0u8; max];
    let compressed = sigil_clownlzss_sys::compress_kosinski_moduled(&plain, 0x100).unwrap();
    assert_eq!(sigil_clownlzss_sys::decompress_kosinski_moduled(&compressed).unwrap(), plain);

    // The rejection path at the default module_size 0x1000 (never reaches
    // the compressor, so the large buffer costs nothing).
    let err = sigil_clownlzss_sys::compress_kosinski_moduled(&vec![0u8; 16 * 0x1000], 0x1000).unwrap_err();
    assert_eq!(err, Error::DataTooLargeForModuled { len: 16 * 0x1000, max: 16 * 0x1000 - 1 });
}

/// Saxman-Moduled gains the same u16-fit post-check as its non-moduled
/// sibling (quality-review minor (a)): 0xF000 bytes of incompressible input
/// is representable by the moduled header (15 full modules) but compresses
/// to ~0x10E00 bytes of total payload, exceeding the u16 ceiling.
#[test]
fn saxman_moduled_rejects_compressed_size_over_u16() {
    let plain = incompressible(0xF000);
    let err = sigil_clownlzss_sys::compress_saxman_moduled(&plain, 0x1000).unwrap_err();
    match err {
        Error::CompressedSizeExceedsU16 { actual } => {
            assert!(actual > u16::MAX as usize, "expected actual > u16::MAX, got {actual}");
        }
        other => panic!("expected CompressedSizeExceedsU16, got {other:?}"),
    }
}
