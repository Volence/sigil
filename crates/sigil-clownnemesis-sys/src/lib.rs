//! Safe wrappers around the vendored `clownnemesis` compressor/decompressor
//! library (see `vendor/VENDOR.md` for provenance, including the
//! load-bearing "input callback must rewind on EOF" contract).
//!
//! Stub: extern "C" declarations only, wired up in a follow-up commit.

#![allow(dead_code)]

use std::os::raw::c_int;

extern "C" {
    fn ClownNemesis_Compress(
        accurate: c_int,
        read_byte: extern "C" fn(*mut std::ffi::c_void) -> c_int,
        read_byte_user_data: *mut std::ffi::c_void,
        write_byte: extern "C" fn(*mut std::ffi::c_void, u8) -> c_int,
        write_byte_user_data: *mut std::ffi::c_void,
    ) -> c_int;
}
