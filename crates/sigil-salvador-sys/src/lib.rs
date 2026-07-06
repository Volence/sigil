//! Safe wrapper around the vendored `salvador` ZX0 compressor library.
//!
//! This crate compiles the unmodified C sources vendored in `vendor/` (see
//! `vendor/README.md` for provenance) and exposes a single safe entry point,
//! [`compress`], that is byte-identical to running the `salvador` CLI with no
//! flags (modern V2 format, forward direction, default window size).

use std::os::raw::{c_longlong, c_uint, c_void};

extern "C" {
    fn salvador_compress(
        p_input_data: *const u8,
        p_out_buffer: *mut u8,
        n_input_size: usize,
        n_max_out_buffer_size: usize,
        n_flags: c_uint,
        n_max_offset: usize,
        n_dictionary_size: usize,
        progress: Option<extern "C" fn(c_longlong, c_longlong)>,
        p_stats: *mut c_void,
    ) -> usize;

    fn salvador_get_max_compressed_size(n_input_size: usize) -> usize;
}

/// Matches `FLG_IS_INVERTED` in `vendor/shrink.c` — the modern (V2) format,
/// as used by the `salvador` CLI when `-classic` is not passed.
const FLG_IS_INVERTED: c_uint = 1;

/// `0` tells the library to use its own default maximum offset
/// (`MAX_OFFSET` = 0x7f80, see `shrink.c`), exactly like the CLI's default
/// `-w` (unset -> `nMaxWindowSize = 0`).
const MAX_OFFSET_DEFAULT: usize = 0;

/// Compress `input` with ZX0, byte-identical to the `salvador` CLI invoked
/// with no flags (`salvador <in> <out>`): modern (V2/inverted) format,
/// forward direction, default (maximum) window, no dictionary.
///
/// Returns the raw salvador stream. Any outer framing (e.g. the
/// `[u16 BE size][0x00][0x02]` wrapper produced by Aeon's build tooling) is
/// the caller's responsibility, not this crate's.
pub fn compress(input: &[u8]) -> Vec<u8> {
    let cap = unsafe { salvador_get_max_compressed_size(input.len()) };
    let mut out = vec![0u8; cap];
    let n = unsafe {
        salvador_compress(
            input.as_ptr(),
            out.as_mut_ptr(),
            input.len(),
            cap,
            FLG_IS_INVERTED,
            MAX_OFFSET_DEFAULT,
            0,
            None,
            std::ptr::null_mut(),
        )
    };
    assert_ne!(n, usize::MAX, "salvador_compress reported an error");
    out.truncate(n);
    out
}
