//! Safe wrapper around the vendored `clownnemesis` compressor/decompressor
//! library (see `vendor/VENDOR.md` for provenance, including the
//! load-bearing "input callback must rewind on EOF" contract).
//!
//! `accurate` (clownnemesis's optional Shannon-Fano mode, matching Sega's
//! own compressor bit-for-bit on some inputs) is hardcoded to `0`/false
//! always — it is NOT exposed as a public option in this task (CR1).

use std::ffi::c_void;
use std::os::raw::c_int;

// ---------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------

/// Errors from the clownnemesis safe wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Input length is not a multiple of `0x20` (tile granularity).
    NotTileAligned { len: usize },
    /// Input exceeds `32767 * 0x20` bytes (32767 tiles, the 15-bit header
    /// field's ceiling).
    TooManyTiles { tiles: usize, max_tiles: usize },
    /// The vendored compressor/decompressor reported failure.
    Overflow,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotTileAligned { len } => {
                write!(f, "input length {len} must be a multiple of 0x20 (tile granularity)")
            }
            Error::TooManyTiles { tiles, max_tiles } => {
                write!(f, "input has {tiles} tiles, exceeding the maximum {max_tiles} representable by the 15-bit header field")
            }
            Error::Overflow => write!(f, "clownnemesis reported a compression/decompression failure"),
        }
    }
}

impl std::error::Error for Error {}

/// Tile size in bytes: Genesis 8x8 4bpp tile.
pub const TILE_SIZE: usize = 0x20;
/// Maximum tile count representable by the 15-bit header field.
pub const MAX_TILES: usize = 32767;

// ---------------------------------------------------------------------
// FFI declarations
// ---------------------------------------------------------------------

type ReadCallback = extern "C" fn(*mut c_void) -> c_int;
type WriteCallback = extern "C" fn(*mut c_void, u8) -> c_int;

const CLOWNNEMESIS_EOF: c_int = -2;

extern "C" {
    fn ClownNemesis_Compress(
        accurate: c_int,
        read_byte: ReadCallback,
        read_byte_user_data: *mut c_void,
        write_byte: WriteCallback,
        write_byte_user_data: *mut c_void,
    ) -> c_int;

    fn ClownNemesis_Decompress(
        read_byte: ReadCallback,
        read_byte_user_data: *mut c_void,
        write_byte: WriteCallback,
        write_byte_user_data: *mut c_void,
    ) -> c_int;
}

// ---------------------------------------------------------------------
// Read/write callback contexts
// ---------------------------------------------------------------------

/// Read-callback context for compression.
///
/// **Load-bearing**: `ClownNemesis_Compress` is a multi-pass encoder (see
/// `vendor/VENDOR.md` and `tests/rewind_regression.rs` for the full story
/// — including RED evidence from a naive, non-rewinding first draft of
/// this callback, which caused compression to silently succeed with a
/// truncated 3-byte output instead of the correct ~218 bytes). Every time
/// this callback is asked to read past the end of `data`, it MUST rewind
/// back to position 0 before returning `CLOWNNEMESIS_EOF`, because the C
/// encoder does not longjmp on EOF during compression
/// (`throw_on_eof = cc_false`, `vendor/compress.c`) — it simply keeps
/// calling this callback across further passes, expecting to see the same
/// data again from the start.
struct ReadContext<'a> {
    data: &'a [u8],
    pos: usize,
}

// ---------------------------------------------------------------------
// Callback infallibility contract (LOAD-BEARING — read before editing)
// ---------------------------------------------------------------------
// These two callbacks are called from C frames. Unwinding a Rust panic
// across an `extern "C"` boundary is immediate UB-adjacent territory (the
// callback ABI aborts the process on unwind since Rust 1.81, which is
// "safe" but still torpedoes the assembler mid-build with no diagnostic).
// They are therefore written to be infallible BY CONSTRUCTION, and must
// stay that way:
//   - `read_cb` performs no allocation and no arithmetic that can
//     overflow (`pos + 1` is bounded by `data.len() <= isize::MAX`); the
//     slice index is guarded by the explicit `pos >= len` check above it.
//   - `write_cb`'s only fallible operation is `Vec::push`: allocation
//     *failure* calls `handle_alloc_error` (abort, not unwind), and the
//     capacity-overflow panic requires a Vec beyond `isize::MAX` bytes —
//     unreachable for any compressed stream of a `<= 32767`-tile input.
// If you add ANY other operation that can panic (indexing, arithmetic,
// I/O, ...), wrap the callback body in `std::panic::catch_unwind` and
// translate the panic into a failure return code instead.

extern "C" fn read_cb(user_data: *mut c_void) -> c_int {
    // SAFETY: `user_data` is always the `&mut ReadContext` passed by
    // `compress`/`decompress` below, which outlives the whole
    // `ClownNemesis_*` call; the C library is single-threaded and never
    // calls the callbacks re-entrantly, so this is the only live reference.
    let ctx = unsafe { &mut *(user_data as *mut ReadContext) };
    if ctx.pos >= ctx.data.len() {
        // Rewind for the next pass (see the ReadContext doc comment and
        // tests/rewind_regression.rs — this line is the actual fix).
        ctx.pos = 0;
        return CLOWNNEMESIS_EOF;
    }
    let byte = ctx.data[ctx.pos];
    ctx.pos += 1;
    byte as c_int
}

/// Write-callback context: accumulates into a growable `Vec<u8>`.
struct WriteContext {
    out: Vec<u8>,
}

extern "C" fn write_cb(user_data: *mut c_void, byte: u8) -> c_int {
    // SAFETY: `user_data` is always the `&mut WriteContext` passed by
    // `compress`/`decompress` below; same liveness/aliasing argument as
    // `read_cb`.
    let ctx = unsafe { &mut *(user_data as *mut WriteContext) };
    ctx.out.push(byte);
    0
}

// ---------------------------------------------------------------------
// Safe API
// ---------------------------------------------------------------------

fn check_tile_constraints(data: &[u8]) -> Result<(), Error> {
    if !data.len().is_multiple_of(TILE_SIZE) {
        return Err(Error::NotTileAligned { len: data.len() });
    }
    let tiles = data.len() / TILE_SIZE;
    if tiles > MAX_TILES {
        return Err(Error::TooManyTiles { tiles, max_tiles: MAX_TILES });
    }
    Ok(())
}

/// Compress `data` (raw Genesis tile data) with Nemesis. `accurate` mode
/// (Sega-bit-exact Shannon-Fano) is always off (CR1). `data` must be a
/// multiple of `0x20` bytes (tile granularity) and at most `32767 * 0x20`
/// bytes (32767 tiles, the 15-bit header field's ceiling) — both checked
/// BEFORE calling the C compressor, per CR5.
pub fn compress(data: &[u8]) -> Result<Vec<u8>, Error> {
    check_tile_constraints(data)?;

    let mut read_ctx = ReadContext { data, pos: 0 };
    let mut write_ctx = WriteContext { out: Vec::new() };

    // SAFETY: the two context pointers stay live and exclusively owned for
    // the duration of this call (they are locals borrowed mutably right
    // here, and the C library neither retains them past the call nor calls
    // the callbacks from another thread); the callbacks cast them back to
    // exactly the types passed in and are infallible by construction (see
    // the contract note above `read_cb`).
    let ok = unsafe {
        ClownNemesis_Compress(
            0,
            read_cb,
            &mut read_ctx as *mut _ as *mut c_void,
            write_cb,
            &mut write_ctx as *mut _ as *mut c_void,
        )
    };

    if ok == 0 {
        return Err(Error::Overflow);
    }
    Ok(write_ctx.out)
}

/// Decompress a Nemesis stream.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut read_ctx = ReadContext { data, pos: 0 };
    let mut write_ctx = WriteContext { out: Vec::new() };

    // SAFETY: same argument as in `compress` — context pointers are live,
    // exclusively-owned locals for the whole call; callbacks are
    // infallible by construction.
    let ok = unsafe {
        ClownNemesis_Decompress(
            read_cb,
            &mut read_ctx as *mut _ as *mut c_void,
            write_cb,
            &mut write_ctx as *mut _ as *mut c_void,
        )
    };

    if ok == 0 {
        return Err(Error::Overflow);
    }
    Ok(write_ctx.out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_tile_aligned_length() {
        let err = compress(&[0u8; 33]).unwrap_err();
        assert_eq!(err, Error::NotTileAligned { len: 33 });
    }

    #[test]
    fn accepts_exact_tile_length() {
        let err = check_tile_constraints(&[0u8; 0x20]);
        assert!(err.is_ok());
    }

    #[test]
    fn tile_count_boundary_check_logic() {
        // Cheap unit test on the length-check logic alone, per the plan's
        // allowance (constructing a real 32767*0x20+ buffer would be
        // ~1MB — wasteful for a unit test, and TooManyTiles is pure
        // arithmetic with no C-side interaction).
        let exactly_max = vec![0u8; MAX_TILES * TILE_SIZE];
        assert!(check_tile_constraints(&exactly_max).is_ok());

        let one_too_many = vec![0u8; (MAX_TILES + 1) * TILE_SIZE];
        let err = check_tile_constraints(&one_too_many).unwrap_err();
        assert_eq!(err, Error::TooManyTiles { tiles: MAX_TILES + 1, max_tiles: MAX_TILES });
    }
}
