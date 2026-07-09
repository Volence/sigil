//! Safe wrappers around the vendored `clownlzss` compressor/decompressor
//! library (see `vendor/VENDOR.md` for provenance).
//!
//! ## Output buffer sizing and the shim capacity protocol
//!
//! Every compress function allocates a conservatively-sized output buffer,
//! passes it to the shim **with its capacity**, and truncates to the real
//! length the shim reports. The shim (`src/shim.cpp`) compresses into an
//! internal `std::ostringstream` and only copies into the caller's buffer
//! after an explicit capacity check — it *cannot* overrun the buffer no
//! matter what capacity is passed. If the buffer is too small the shim
//! returns `-1` with the exact required size, and the wrapper retries once
//! with an exactly-sized buffer. (An earlier version trusted the Rust-side
//! bound alone with an unchecked raw-pointer sink; worst-case moduled input
//! overran it by ~50 bytes — a live heap overflow. See
//! `tests/moduled_capacity.rs` for the recorded RED evidence.)
//!
//! ### The non-moduled bound: `input + input/8 + 64`
//!
//! Every format here is an LZSS variant built on
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
//!     plus a 6-byte header/table overhead.
//!
//! The descriptor-bit overhead rounds to at most ~12.5% growth (1 bit per
//! 8-bit byte), which `input/8` covers, and the small fixed
//! header/table/terminator overhead is covered by the `+64` constant.
//!
//! ### The moduled bound: `input + input/8 + num_modules*16 + 64`
//!
//! `ModuledCompressionWrapper` emits `num_modules = ceil(input /
//! module_size)` *independent* streams, each with its own terminator
//! (~5 bytes worst case for Kosinski) plus up to 15 bytes of alignment
//! padding between modules (Kosinski aligns modules to 0x10; the other
//! formats to 1 or 2) — per-module overhead the flat `+64` cannot cover.
//! `num_modules*16` covers it: worst per-module overhead is ~20 bytes
//! (5 terminator + 15 padding), and since the moduled header caps
//! `num_modules` at 16 (see [`Error::DataTooLargeForModuled`]),
//! `16k + 64 >= 20k - 13` holds for every legal `k` (it holds for all
//! `k <= 19`). The empirical worst case is exercised in
//! `tests/moduled_capacity.rs` with a de Bruijn (zero-match) input.
//!
//! Even if either bound were ever short, the shim's capacity check turns
//! that into an exact-size retry, never an overrun.
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
    /// `module_size` is zero — the vendored `ModuledCompressionWrapper`
    /// would compute `data_size % 0` (UB), so this never reaches C.
    ModuleSizeZero,
    /// The input is too long for the 16-bit moduled header: it stores
    /// `len % module_size` in its low 12 bits and `len / module_size` in
    /// its high 4 bits, so a quotient above 15 silently truncates upstream
    /// (a corrupt stream). `max = 16 * module_size - 1`.
    DataTooLargeForModuled { len: usize, max: usize },
    /// Saxman-with-header's compressed size (the 2-byte LE header field)
    /// overflowed `u16`, checked AFTER compression per CR5. Also raised by
    /// the moduled Saxman wrapper for its total compressed payload (see
    /// [`compress_saxman_moduled`]).
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
            Error::ModuleSizeZero => write!(f, "module_size 0 is not a valid module size"),
            Error::DataTooLargeForModuled { len, max } => write!(
                f,
                "input length {len} exceeds the maximum {max} representable by the 16-bit moduled header at this module_size"
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

/// Shim compress return protocol (see `src/shim.cpp`):
/// `1` = success, `0` = compressor failure, `-1` = `out_capacity` too
/// small, required size stored in `out_len`.
type CompressFn = unsafe extern "C" fn(*const u8, usize, *mut u8, usize, *mut usize) -> c_int;
/// Moduled variant of [`CompressFn`] with a trailing `module_size`.
type ModuledCompressFn = unsafe extern "C" fn(*const u8, usize, *mut u8, usize, *mut usize, usize) -> c_int;
/// Shim decompress protocol: `1` = success, `0` = capacity too small.
type DecompressFn = unsafe extern "C" fn(*const u8, usize, *mut u8, usize, *mut usize) -> c_int;

extern "C" {
    fn clownlzss_max_compressed_size(input_size: usize) -> usize;

    fn clownlzss_kosinski_compress(data: *const u8, data_size: usize, out: *mut u8, out_capacity: usize, out_len: *mut usize) -> c_int;
    fn clownlzss_kosinski_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
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

    fn clownlzss_kosinskiplus_compress(data: *const u8, data_size: usize, out: *mut u8, out_capacity: usize, out_len: *mut usize) -> c_int;
    fn clownlzss_kosinskiplus_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
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

    fn clownlzss_saxman_compress_with_header(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;
    fn clownlzss_saxman_compress_without_header(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> c_int;
    fn clownlzss_saxman_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
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

    fn clownlzss_enigma_compress(data: *const u8, data_size: usize, out: *mut u8, out_capacity: usize, out_len: *mut usize) -> c_int;
    fn clownlzss_enigma_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
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

    fn clownlzss_comper_compress(data: *const u8, data_size: usize, out: *mut u8, out_capacity: usize, out_len: *mut usize) -> c_int;
    fn clownlzss_comper_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
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

    fn clownlzss_rocket_compress(data: *const u8, data_size: usize, out: *mut u8, out_capacity: usize, out_len: *mut usize) -> c_int;
    fn clownlzss_rocket_compress_moduled(
        data: *const u8,
        data_size: usize,
        out: *mut u8,
        out_capacity: usize,
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
    // SAFETY: `clownlzss_max_compressed_size` is a pure arithmetic helper
    // in the shim (no pointers, no state).
    unsafe { clownlzss_max_compressed_size(input_len) }
}

/// Conservative bound for moduled compression; see the module doc
/// ("The moduled bound") for the derivation. `module_size` must be > 0
/// (guaranteed by [`check_module_constraints`]).
fn moduled_max_size(input_len: usize, module_size: usize) -> usize {
    let num_modules = input_len.div_ceil(module_size);
    input_len + input_len / 8 + num_modules * 16 + 64
}

/// CR5 constraint checks shared by every moduled compress wrapper. Runs
/// BEFORE any C call; see the [`Error`] variants for the individual
/// rationales (`% 0` UB, 12-bit module_size field, 4-bit module-count
/// field).
fn check_module_constraints(data: &[u8], module_size: usize) -> Result<(), Error> {
    if module_size == 0 {
        return Err(Error::ModuleSizeZero);
    }
    if module_size > MAX_MODULE_SIZE {
        return Err(Error::ModuleSizeTooLarge { requested: module_size, max: MAX_MODULE_SIZE });
    }
    let max_len = 16 * module_size - 1;
    if data.len() > max_len {
        return Err(Error::DataTooLargeForModuled { len: data.len(), max: max_len });
    }
    Ok(())
}

/// Drives a shim compress function through the capacity protocol: allocate
/// `initial_cap`, and if the shim reports `-1` ("does not fit; required
/// size in out_len"), retry once with an exactly-sized buffer. The second
/// attempt cannot report `-1` again (the shim's required size is exact);
/// if it somehow does, fail loudly rather than loop.
fn run_compress_with_cap(
    data: &[u8],
    initial_cap: usize,
    f: impl Fn(*const u8, usize, *mut u8, usize, *mut usize) -> c_int,
) -> Result<Vec<u8>, Error> {
    let mut cap = initial_cap.max(64);
    for _ in 0..2 {
        let mut out = vec![0u8; cap];
        let mut out_len: usize = 0;
        let ret = f(data.as_ptr(), data.len(), out.as_mut_ptr(), cap, &mut out_len);
        match ret {
            1 => {
                out.truncate(out_len);
                return Ok(out);
            }
            -1 => cap = out_len, // exact required size, reported by the shim
            _ => return Err(Error::Overflow),
        }
    }
    Err(Error::Overflow)
}

/// Non-moduled compress: worst-case bound + capacity-protocol retry.
fn run_compress(data: &[u8], f: CompressFn) -> Result<Vec<u8>, Error> {
    run_compress_with_cap(data, max_size(data.len()), |d, n, o, c, l| {
        // SAFETY: `d`/`n` describe a live borrowed slice; `o`/`c` describe
        // a live, exactly-`c`-byte Vec allocation; `l` points at a live
        // usize. The shim never writes more than `c` bytes into `o`
        // (ostringstream intermediary + explicit capacity check) and does
        // not retain any pointer past the call.
        unsafe { f(d, n, o, c, l) }
    })
}

/// Moduled compress: CR5 constraint checks, then the module-count-scaled
/// bound + capacity-protocol retry.
fn run_compress_moduled(data: &[u8], module_size: u16, f: ModuledCompressFn) -> Result<Vec<u8>, Error> {
    let module_size = module_size as usize;
    check_module_constraints(data, module_size)?;
    run_compress_with_cap(data, moduled_max_size(data.len(), module_size), |d, n, o, c, l| {
        // SAFETY: as in `run_compress`; `module_size` is a plain integer,
        // validated non-zero and <= MAX_MODULE_SIZE above.
        unsafe { f(d, n, o, c, l, module_size) }
    })
}

/// Runs a decompress shim function, growing the output buffer if the
/// initial capacity guess was too small.
fn run_decompress(data: &[u8], initial_cap: usize, f: DecompressFn) -> Result<Vec<u8>, Error> {
    let mut cap = initial_cap.max(64);
    loop {
        let mut out = vec![0u8; cap];
        let mut out_len: usize = 0;
        // SAFETY: `data` is a live borrowed slice; `out` is a live,
        // exactly-`cap`-byte Vec allocation; `out_len` points at a live
        // usize. The shim decompresses into an internal ostringstream and
        // only memcpy's into `out` after checking the result fits `cap`,
        // so it cannot write out of bounds; no pointer is retained.
        let ok = unsafe { f(data.as_ptr(), data.len(), out.as_mut_ptr(), cap, &mut out_len) };
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
    run_compress(data, clownlzss_kosinski_compress)
}

/// Compress `data` with Kosinski-Moduled, splitting into `module_size`-byte
/// modules (default `$1000` upstream). `module_size` must be in
/// `1..=`[`MAX_MODULE_SIZE`], and `data` must fit the 16-bit moduled header
/// (`len < 16 * module_size`).
pub fn compress_kosinski_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    run_compress_moduled(data, module_size, clownlzss_kosinski_compress_moduled)
}

/// Decompress a plain Kosinski stream.
pub fn decompress_kosinski(data: &[u8]) -> Result<Vec<u8>, Error> {
    run_decompress(data, data.len() * 8 + 256, clownlzss_kosinski_decompress)
}

/// Decompress a Kosinski-Moduled stream.
pub fn decompress_kosinski_moduled(data: &[u8]) -> Result<Vec<u8>, Error> {
    run_decompress(data, data.len() * 8 + 256, clownlzss_kosinski_decompress_moduled)
}

// ---------------------------------------------------------------------
// Kosinski+
// ---------------------------------------------------------------------

/// Compress `data` with Kosinski+.
pub fn compress_kosplus(data: &[u8]) -> Result<Vec<u8>, Error> {
    run_compress(data, clownlzss_kosinskiplus_compress)
}

/// Compress `data` with Kosinski+-Moduled. Same constraints as
/// [`compress_kosinski_moduled`].
pub fn compress_kosplus_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    run_compress_moduled(data, module_size, clownlzss_kosinskiplus_compress_moduled)
}

/// Decompress a plain Kosinski+ stream.
pub fn decompress_kosplus(data: &[u8]) -> Result<Vec<u8>, Error> {
    run_decompress(data, data.len() * 8 + 256, clownlzss_kosinskiplus_decompress)
}

/// Decompress a Kosinski+-Moduled stream.
pub fn decompress_kosplus_moduled(data: &[u8]) -> Result<Vec<u8>, Error> {
    run_decompress(data, data.len() * 8 + 256, clownlzss_kosinskiplus_decompress_moduled)
}

// ---------------------------------------------------------------------
// Saxman
// ---------------------------------------------------------------------

/// Compress `data` with Saxman. If `header` is true, prefixes the stream
/// with a 2-byte little-endian compressed-size field (checked to fit
/// `u16` AFTER compression, per CR5); if false, emits the raw stream.
pub fn compress_saxman(data: &[u8], header: bool) -> Result<Vec<u8>, Error> {
    let out = if header {
        run_compress(data, clownlzss_saxman_compress_with_header)?
    } else {
        run_compress(data, clownlzss_saxman_compress_without_header)?
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
/// `CompressWithHeader`). Same module constraints as
/// [`compress_kosinski_moduled`].
///
/// Mirrors [`compress_saxman`]'s u16-fit post-check on the total compressed
/// payload (everything after the 2-byte moduled header). Note this is a
/// conservative, whole-stream reading of the sibling's per-stream check:
/// each embedded module's own u16 header cannot overflow (a module is at
/// most `MAX_MODULE_SIZE` bytes, whose worst-case compressed size is far
/// below u16), but Saxman consumers track stream sizes in u16 (SMPS), so a
/// total payload beyond u16 is rejected loudly rather than emitted.
pub fn compress_saxman_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    let out = run_compress_moduled(data, module_size, clownlzss_saxman_compress_moduled)?;
    let payload_len = out.len().saturating_sub(2);
    if payload_len > u16::MAX as usize {
        return Err(Error::CompressedSizeExceedsU16 { actual: payload_len });
    }
    Ok(out)
}

/// Decompress a header-less Saxman stream of exactly `compressed_length`
/// bytes (the caller must know the length out of band).
pub fn decompress_saxman_no_header(data: &[u8], compressed_length: usize) -> Result<Vec<u8>, Error> {
    let mut cap = compressed_length.max(64) * 8 + 256;
    loop {
        let mut out = vec![0u8; cap];
        let mut out_len: usize = 0;
        // SAFETY: as in `run_decompress` — the only difference is that the
        // second argument is the caller-declared compressed length rather
        // than `data.len()`; the shim reads at most `compressed_length`
        // bytes from `data`, which the caller contract requires to be
        // within the slice.
        let ok = unsafe {
            clownlzss_saxman_decompress_no_header(data.as_ptr(), compressed_length, out.as_mut_ptr(), cap, &mut out_len)
        };
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

/// Decompress a Saxman stream that begins with its own 2-byte
/// little-endian compressed-size header.
pub fn decompress_saxman_with_header(data: &[u8]) -> Result<Vec<u8>, Error> {
    run_decompress(data, data.len() * 8 + 256, clownlzss_saxman_decompress_with_header)
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
    run_compress(data, clownlzss_enigma_compress)
}

/// Compress `data` with Enigma-Moduled. `data` must be word-even; same
/// module constraints as [`compress_kosinski_moduled`].
pub fn compress_enigma_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    check_word_even(data)?;
    run_compress_moduled(data, module_size, clownlzss_enigma_compress_moduled)
}

/// Decompress an Enigma stream.
pub fn decompress_enigma(data: &[u8]) -> Result<Vec<u8>, Error> {
    run_decompress(data, data.len() * 8 + 256, clownlzss_enigma_decompress)
}

// ---------------------------------------------------------------------
// Comper
// ---------------------------------------------------------------------

/// Compress `data` with Comper. `data` must be word-even.
pub fn compress_comper(data: &[u8]) -> Result<Vec<u8>, Error> {
    check_word_even(data)?;
    run_compress(data, clownlzss_comper_compress)
}

/// Compress `data` with Comper-Moduled. `data` must be word-even; same
/// module constraints as [`compress_kosinski_moduled`].
pub fn compress_comper_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    check_word_even(data)?;
    run_compress_moduled(data, module_size, clownlzss_comper_compress_moduled)
}

/// Decompress a Comper stream.
pub fn decompress_comper(data: &[u8]) -> Result<Vec<u8>, Error> {
    run_decompress(data, data.len() * 8 + 256, clownlzss_comper_decompress)
}

// ---------------------------------------------------------------------
// Rocket
// ---------------------------------------------------------------------

/// Compress `data` with Rocket (always header-framed — upstream has no
/// header-less Rocket compressor).
pub fn compress_rocket(data: &[u8]) -> Result<Vec<u8>, Error> {
    run_compress(data, clownlzss_rocket_compress)
}

/// Compress `data` with Rocket-Moduled. Same module constraints as
/// [`compress_kosinski_moduled`].
pub fn compress_rocket_moduled(data: &[u8], module_size: u16) -> Result<Vec<u8>, Error> {
    run_compress_moduled(data, module_size, clownlzss_rocket_compress_moduled)
}

/// Decompress a Rocket stream.
pub fn decompress_rocket(data: &[u8]) -> Result<Vec<u8>, Error> {
    run_decompress(data, data.len() * 8 + 256, clownlzss_rocket_decompress)
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
    fn kosinski_module_size_zero_is_rejected() {
        let err = compress_kosinski_moduled(&[0u8; 8], 0).unwrap_err();
        assert_eq!(err, Error::ModuleSizeZero);
    }

    #[test]
    fn moduled_data_length_ceiling_is_enforced() {
        // module_size 0x80: max representable = 16 * 0x80 - 1 = 0x7FF.
        // (Small module_size keeps the accepted boundary case fast — the
        // optimal parser is slow on large zero runs in debug builds; the
        // check itself is the same arithmetic at every module_size.)
        let err = compress_kosinski_moduled(&vec![0u8; 0x800], 0x80).unwrap_err();
        assert_eq!(err, Error::DataTooLargeForModuled { len: 0x800, max: 0x7FF });
        assert!(compress_kosinski_moduled(&vec![0u8; 0x7FF], 0x80).is_ok());
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
