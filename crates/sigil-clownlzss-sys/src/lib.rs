//! Safe wrappers around the vendored `clownlzss` compressor/decompressor
//! library (see `vendor/VENDOR.md` for provenance).
//!
//! Stub: extern "C" declarations only, wired up in a follow-up commit.

#![allow(dead_code)]

use std::os::raw::c_int;

extern "C" {
    fn clownlzss_max_compressed_size(input_size: usize) -> usize;

    fn clownlzss_kosinski_compress(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_len: *mut usize,
    ) -> c_int;
}
