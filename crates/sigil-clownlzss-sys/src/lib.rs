//! Safe wrappers around the vendored `clownlzss` compressor/decompressor
//! library (see `vendor/VENDOR.md` for provenance).
//!
//! ## Output buffer sizing
//!
//! Every compress function asks the vendored `clownlzss_max_compressed_size`
//! helper for a conservative upper bound on the compressed size, allocates
//! that much, compresses into it, then truncates to the real length the
//! shim reports. The bound is `input_size + input_size/8 + 64`.
//!
//! Why this is safe: every format here is an LZSS variant built on
//! `ClownLZSS_FindOptimalMatches`, whose optimal parser never chooses a
//! match whose cost exceeds treating the same bytes as literals (see the
//! `GetMatchCost` callbacks in each vendored `compressors/*.h` — they
//! return the encoded cost in bits, and the graph search picks the cheapest
//! path, literal-run included). So the true worst case is bounded by "every
//! input byte/word becomes a literal", which costs (per format):
//!   - Kosinski/Kosinski+: 1 descriptor bit + 8 data bits per literal byte
//!     (2-bit descriptor field, 1/8 growth) plus a handful of terminator
//!     bytes.
//!   - Saxman/Comper/Rocket: 1 descriptor bit per literal byte/word plus
//!     the byte/word itself, plus a 2-4 byte header.
//!   - Enigma: a 6-bit header per value in the worst case (raw copy
//!     encoding) vs. 16 bits of source data — always shrinks per-value,
//!     plus an 6-byte header/table overhead.
//!
//! In every case the descriptor-bit overhead rounds to at most ~12.5% growth
//! (1 bit per 8-bit byte), which `input_size/8` covers, and the small fixed
//! header/table/terminator overhead is covered by the `+64` constant. Every
//! real test input in `tests/byte_exact.rs` compresses to well under this
//! bound (checked explicitly in `compressed_size_stays_within_bound`).
//!
//! ## Decompression bounds
//!
//! The vendored decompressor templates are unchecked: they trust the
//! caller's output buffer is large enough (see `vendor/decompressors/*.h`
//! — `DecompressorOutput::Copy`/`Write` have no bounds check). This wrapper
//! guards against overflow by giving the shim a caller-supplied capacity;
//! if the shim's write would exceed it, the shim returns failure rather
//! than writing out of bounds (see `src/shim.cpp`). The Rust wrapper starts
//! with a generous capacity and grows it on failure, matching the "unknown
//! output size" nature of a compressed stream.

use std::os::raw::c_int;

// ---------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------

/// Errors from the clownlzss safe wrapper. Mirrors the constraint table
/// from Plan-7 #10's CR5 ruling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `kosinski_m`'s `module_size` exceeds `$1000` — the 12-bit moduled
    /// header field cannot represent a larger module size.
    ModuleSizeTooLarge { requested: usize, max: usize },
    /// Saxman-with-header's compressed size (the 2-byte LE header field)
    /// overflowed `u16`, checked AFTER compression per CR5.
    CompressedSizeExceedsU16 { actual: usize },
    /// Enigma/Comper require word-even (multiple-of-2) input.
    NotWordEven { len: usize },
    /// The vendored compressor/decompressor reported failure (e.g. buffer
    /// capacity exceeded during decompression, or an internal allocation
    /// failure in the C core).
    Overflow,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ModuleSizeTooLarge { requested, max } => write!(
                f,
                "module_size {requested:#x} exceeds the maximum {max:#x} representable by the moduled header"
            ),
            Error::CompressedSizeExceedsU16 { actual } => write!(
                f,
                "compressed size {actual} exceeds u16 (65535), cannot fit the Saxman header field"
            ),
            Error::NotWordEven { len } => write!(f, "input length {len} must be word-even (a multiple of 2)"),
            Error::Overflow => write!(f, "clownlzss reported a compression/decompression failure"),
        }
    }
}

impl std::error::Error for Error {}

/// The maximum `module_size` for `kosinski_m`/`kosplus_m`/etc: the 12-bit
/// moduled-header field (`data_size % module_size`) cannot represent a
/// module_size greater than this without losing information, per CR3/CR5.
pub const MAX_MODULE_SIZE: usize = 0x1000;

// ---------------------------------------------------------------------
// FFI declarations (see src/shim.cpp for definitions)
// ---------------------------------------------------------------------

extern "C" {
    fn clownlzss_max_compressed_size(input_size: usize) -> usize;

    fn clownlzss_kosinski_compress(data: *const u8, data_size: usize, out: *mut u8, out_len: *mut usize) -> c_int;
    fn clownlzss_kosinski_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_len: *mut usize,
        module_size: usize,
    ) -> c_int;
    fn clownlzss_kosinski_decompress(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;
    fn clownlzss_kosinski_decompress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;

    fn clownlzss_kosinskiplus_compress(data: *const u8, data_size: usize, out: *mut u8, out_len: *mut usize) -> c_int;
    fn clownlzss_kosinskiplus_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_len: *mut usize,
        module_size: usize,
    ) -> c_int;
    fn clownlzss_kosinskiplus_decompress(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;
    fn clownlzss_kosinskiplus_decompress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;

    fn clownlzss_saxman_compress_with_header(data: *const u8, data_size: usize, out: *mut u8, out_len: *mut usize) -> c_int;
    fn clownlzss_saxman_compress_without_header(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_len: *mut usize,
    ) -> c_int;
    fn clownlzss_saxman_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_len: *mut usize,
        module_size: usize,
    ) -> c_int;
    fn clownlzss_saxman_decompress_no_header(
        data: *const u8,
        compressed_length: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;
    fn clownlzss_saxman_decompress_with_header(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;

    fn clownlzss_enigma_compress(data: *const u8, data_size: usize, out: *mut u8, out_len: *mut usize) -> c_int;
    fn clownlzss_enigma_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_len: *mut usize,
        module_size: usize,
    ) -> c_int;
    fn clownlzss_enigma_decompress(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;

    fn clownlzss_comper_compress(data: *const u8, data_size: usize, out: *mut u8, out_len: *mut usize) -> c_int;
    fn clownlzss_comper_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_len: *mut usize,
        module_size: usize,
    ) -> c_int;
    fn clownlzss_comper_decompress(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;

    fn clownlzss_rocket_compress(data: *const u8, data_size: usize, out: *mut u8, out_len: *mut usize) -> c_int;
    fn clownlzss_rocket_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_len: *mut usize,
        module_size: usize,
    ) -> c_int;
    fn clownlzss_rocket_decompress(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;
}

// ---------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------

fn max_size(input_len: usize) -> usize {
    unsafe { clownlzss_max_compressed_size(input_len) }
}

/// Runs a compress shim function with a freshly-allocated, conservatively
/// sized output buffer, truncating to the real reported length on success.
unsafe fn run_compress(
    data: &[u8],
    f: unsafe extern "C" fn(*const u8, usize, *mut u8, *mut usize) -> c_int,
) -> Result<Vec<u8>, Error> {
    let cap = max_size(data.len());
    let mut out = vec![0u8; cap];
    let mut out_len: usize = 0;
    let ok = f(data.as_ptr(), data.len(), out.as_mut_ptr(), &mut out_len);
    if ok == 0 {
        return Err(Error::Overflow);
    }
    out.truncate(out_len);
    Ok(out)
}

/// Runs a decompress shim function, growing the output buffer if the
/// initial capacity guess was too small.
unsafe fn run_decompress(
    data: &[u8],
    initial_cap: usize,
    f: unsafe extern "C" fn(*const u8, usize, *mut u8, usize, *mut usize) -> c_int,
) -> Result<Vec<u8>, Error> {
    let mut cap = initial_cap.max(64);
    loop {
        let mut out = vec![0u8; cap];
        let mut out_len: usize = 0;
        let ok = f(data.as_ptr(), data.len(), out.as_mut_ptr(), cap, &mut out_len);
        if ok != 0 {
            out.truncate(out_len);
            return Ok(out);
        }
        if cap >= (1 << 28) {
            return Err(Error::Overflow);
        }
        cap *= 4;
    }
}

// ---------------------------------------------------------------------
// Kosinski
// ---------------------------------------------------------------------

/// Compress `data` with plain Kosinski. Emits the raw Kosinski stream (no
/// outer framing).
pub fn compress_kosinski(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_compress(data, clownlzss_kosinski_compress) }
}

/// Compress `data` with Kosinski-Moduled, splitting into `module_size`-byte
/// modules (default `$1000` upstream). `module_size` must not exceed
/// [`MAX_MODULE_SIZE`] — the 12-bit moduled header cannot represent more.
pub fn compress_kosinski_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    let module_size = module_size as usize;
    if module_size > MAX_MODULE_SIZE {
        return Err(Error::ModuleSizeTooLarge { requested: module_size, max: MAX_MODULE_SIZE });
    }
    unsafe {
        let cap = max_size(data.len()) + 16; // + moduled header/padding slack
        let mut out = vec![0u8; cap];
        let mut out_len: usize = 0;
        let ok = clownlzss_kosinski_compress_moduled(data.as_ptr(), data.len(), out.as_mut_ptr(), &mut out_len, module_size);
        if ok == 0 {
            return Err(Error::Overflow);
        }
        out.truncate(out_len);
        Ok(out)
    }
}

/// Decompress a plain Kosinski stream.
pub fn decompress_kosinski(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_decompress(data, data.len() * 8 + 256, clownlzss_kosinski_decompress) }
}

/// Decompress a Kosinski-Moduled stream.
pub fn decompress_kosinski_moduled(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_decompress(data, data.len() * 8 + 256, clownlzss_kosinski_decompress_moduled) }
}

// ---------------------------------------------------------------------
// Kosinski+
// ---------------------------------------------------------------------

/// Compress `data` with Kosinski+.
pub fn compress_kosplus(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_compress(data, clownlzss_kosinskiplus_compress) }
}

/// Compress `data` with Kosinski+-Moduled. `module_size` must not exceed
/// [`MAX_MODULE_SIZE`].
pub fn compress_kosplus_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    let module_size = module_size as usize;
    if module_size > MAX_MODULE_SIZE {
        return Err(Error::ModuleSizeTooLarge { requested: module_size, max: MAX_MODULE_SIZE });
    }
    unsafe {
        let cap = max_size(data.len()) + 16;
        let mut out = vec![0u8; cap];
        let mut out_len: usize = 0;
        let ok = clownlzss_kosinskiplus_compress_moduled(data.as_ptr(), data.len(), out.as_mut_ptr(), &mut out_len, module_size);
        if ok == 0 {
            return Err(Error::Overflow);
        }
        out.truncate(out_len);
        Ok(out)
    }
}

/// Decompress a plain Kosinski+ stream.
pub fn decompress_kosplus(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_decompress(data, data.len() * 8 + 256, clownlzss_kosinskiplus_decompress) }
}

/// Decompress a Kosinski+-Moduled stream.
pub fn decompress_kosplus_moduled(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_decompress(data, data.len() * 8 + 256, clownlzss_kosinskiplus_decompress_moduled) }
}

// ---------------------------------------------------------------------
// Saxman
// ---------------------------------------------------------------------

/// Compress `data` with Saxman. If `header` is true, prefixes the stream
/// with a 2-byte little-endian compressed-size field (checked to fit
/// `u16` AFTER compression, per CR5); if false, emits the raw stream.
pub fn compress_saxman(data: &[u8], header: bool) -> Result<Vec<u8>, Error> {
    let out = if header {
        unsafe { run_compress(data, clownlzss_saxman_compress_with_header)? }
    } else {
        unsafe { run_compress(data, clownlzss_saxman_compress_without_header)? }
    };
    if header {
        // The header encodes (compressed size - 2) as its own u16 field, so
        // the *payload* (out.len() - 2) is what must fit.
        let payload_len = out.len() - 2;
        if payload_len > u16::MAX as usize {
            return Err(Error::CompressedSizeExceedsU16 { actual: payload_len });
        }
    }
    Ok(out)
}

/// Compress `data` with Saxman-Moduled (always header-framed per module,
/// matching upstream's `ModuledSaxmanCompress`, which is built on
/// `CompressWithHeader`). `module_size` must not exceed [`MAX_MODULE_SIZE`].
pub fn compress_saxman_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    let module_size = module_size as usize;
    if module_size > MAX_MODULE_SIZE {
        return Err(Error::ModuleSizeTooLarge { requested: module_size, max: MAX_MODULE_SIZE });
    }
    unsafe {
        let cap = max_size(data.len()) + 16;
        let mut out = vec![0u8; cap];
        let mut out_len: usize = 0;
        let ok = clownlzss_saxman_compress_moduled(data.as_ptr(), data.len(), out.as_mut_ptr(), &mut out_len, module_size);
        if ok == 0 {
            return Err(Error::Overflow);
        }
        out.truncate(out_len);
        Ok(out)
    }
}

/// Decompress a header-less Saxman stream of exactly `compressed_length`
/// bytes (the caller must know the length out of band).
pub fn decompress_saxman_no_header(data: &[u8], compressed_length: usize) -> Result<Vec<u8>, Error> {
    unsafe {
        let mut cap = compressed_length.max(64) * 8 + 256;
        loop {
            let mut out = vec![0u8; cap];
            let mut out_len: usize = 0;
            let ok = clownlzss_saxman_decompress_no_header(data.as_ptr(), compressed_length, out.as_mut_ptr(), cap, &mut out_len);
            if ok != 0 {
                out.truncate(out_len);
                return Ok(out);
            }
            if cap >= (1 << 28) {
                return Err(Error::Overflow);
            }
            cap *= 4;
        }
    }
}

/// Decompress a Saxman stream that begins with its own 2-byte
/// little-endian compressed-size header.
pub fn decompress_saxman_with_header(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_decompress(data, data.len() * 8 + 256, clownlzss_saxman_decompress_with_header) }
}

// ---------------------------------------------------------------------
// Enigma
// ---------------------------------------------------------------------

fn check_word_even(data: &[u8]) -> Result<(), Error> {
    if !data.len().is_multiple_of(2) {
        return Err(Error::NotWordEven { len: data.len() });
    }
    Ok(())
}

/// Compress `data` with Enigma. `data` must be word-even (a multiple of 2
/// bytes), checked BEFORE calling the C compressor per CR5.
pub fn compress_enigma(data: &[u8]) -> Result<Vec<u8>, Error> {
    check_word_even(data)?;
    unsafe { run_compress(data, clownlzss_enigma_compress) }
}

/// Compress `data` with Enigma-Moduled. `module_size` must not exceed
/// [`MAX_MODULE_SIZE`]; `data` must be word-even.
pub fn compress_enigma_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    check_word_even(data)?;
    let module_size = module_size as usize;
    if module_size > MAX_MODULE_SIZE {
        return Err(Error::ModuleSizeTooLarge { requested: module_size, max: MAX_MODULE_SIZE });
    }
    unsafe {
        let cap = max_size(data.len()) + 16;
        let mut out = vec![0u8; cap];
        let mut out_len: usize = 0;
        let ok = clownlzss_enigma_compress_moduled(data.as_ptr(), data.len(), out.as_mut_ptr(), &mut out_len, module_size);
        if ok == 0 {
            return Err(Error::Overflow);
        }
        out.truncate(out_len);
        Ok(out)
    }
}

/// Decompress an Enigma stream.
pub fn decompress_enigma(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_decompress(data, data.len() * 8 + 256, clownlzss_enigma_decompress) }
}

// ---------------------------------------------------------------------
// Comper
// ---------------------------------------------------------------------

/// Compress `data` with Comper. `data` must be word-even.
pub fn compress_comper(data: &[u8]) -> Result<Vec<u8>, Error> {
    check_word_even(data)?;
    unsafe { run_compress(data, clownlzss_comper_compress) }
}

/// Compress `data` with Comper-Moduled. `module_size` must not exceed
/// [`MAX_MODULE_SIZE`]; `data` must be word-even.
pub fn compress_comper_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    check_word_even(data)?;
    let module_size = module_size as usize;
    if module_size > MAX_MODULE_SIZE {
        return Err(Error::ModuleSizeTooLarge { requested: module_size, max: MAX_MODULE_SIZE });
    }
    unsafe {
        let cap = max_size(data.len()) + 16;
        let mut out = vec![0u8; cap];
        let mut out_len: usize = 0;
        let ok = clownlzss_comper_compress_moduled(data.as_ptr(), data.len(), out.as_mut_ptr(), &mut out_len, module_size);
        if ok == 0 {
            return Err(Error::Overflow);
        }
        out.truncate(out_len);
        Ok(out)
    }
}

/// Decompress a Comper stream.
pub fn decompress_comper(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_decompress(data, data.len() * 8 + 256, clownlzss_comper_decompress) }
}

// ---------------------------------------------------------------------
// Rocket
// ---------------------------------------------------------------------

/// Compress `data` with Rocket (always header-framed — upstream has no
/// header-less Rocket compressor).
pub fn compress_rocket(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_compress(data, clownlzss_rocket_compress) }
}

/// Compress `data` with Rocket-Moduled. `module_size` must not exceed
/// [`MAX_MODULE_SIZE`].
pub fn compress_rocket_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    let module_size = module_size as usize;
    if module_size > MAX_MODULE_SIZE {
        return Err(Error::ModuleSizeTooLarge { requested: module_size, max: MAX_MODULE_SIZE });
    }
    unsafe {
        let cap = max_size(data.len()) + 16;
        let mut out = vec![0u8; cap];
        let mut out_len: usize = 0;
        let ok = clownlzss_rocket_compress_moduled(data.as_ptr(), data.len(), out.as_mut_ptr(), &mut out_len, module_size);
        if ok == 0 {
            return Err(Error::Overflow);
        }
        out.truncate(out_len);
        Ok(out)
    }
}

/// Decompress a Rocket stream.
pub fn decompress_rocket(data: &[u8]) -> Result<Vec<u8>, Error> {
    unsafe { run_decompress(data, data.len() * 8 + 256, clownlzss_rocket_decompress) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kosinski_module_size_too_large_is_rejected() {
        let err = compress_kosinski_moduled(&[0u8; 8], 0x1001).unwrap_err();
        assert_eq!(err, Error::ModuleSizeTooLarge { requested: 0x1001, max: MAX_MODULE_SIZE });
    }

    #[test]
    fn kosinski_module_size_at_max_is_accepted() {
        let out = compress_kosinski_moduled(&[0u8; 8], 0x1000);
        assert!(out.is_ok());
    }

    #[test]
    fn enigma_rejects_odd_length_input() {
        let err = compress_enigma(&[1, 2, 3]).unwrap_err();
        assert_eq!(err, Error::NotWordEven { len: 3 });
    }

    #[test]
    fn comper_rejects_odd_length_input() {
        let err = compress_comper(&[1, 2, 3]).unwrap_err();
        assert_eq!(err, Error::NotWordEven { len: 3 });
    }
}
