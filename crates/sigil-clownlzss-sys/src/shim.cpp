// Sigil-original C++ shim over clownlzss's C++20 template headers.
//
// clownlzss's optimal-parse core (vendor/compressors/clownlzss.c) is C89,
// but every per-format compressor/decompressor is a template header with no
// C linkage entry point. This file instantiates each template directly
// (via the `Internal::<Format>::Compress`/`Decompress` functions, not the
// top-level convenience wrappers — see the "Tell(), not the caller's
// pointer" note below for why) and exposes one `extern "C"` function per
// format, so `src/lib.rs`'s safe wrapper can call into it without depending
// on the C++ ABI beyond `extern "C"`.
//
// ---------------------------------------------------------------------
// "Tell(), not the caller's pointer": why every function below constructs
// its own CompressorOutput/DecompressorOutput and reads back .Tell()
// ---------------------------------------------------------------------
//
// The top-level convenience wrappers (`ClownLZSS::KosinskiCompress(data,
// size, output)`, `ClownLZSS::KosinskiDecompress(input, output)`, etc.) take
// their iterator argument by forwarding reference and construct their OWN
// `CompressorOutput`/`DecompressorOutput` wrapper around a *copy* of it
// internally. If you pass a raw pointer lvalue (e.g. `unsigned char* cursor
// = out; KosinskiCompress(data, size, cursor);`), the wrapper's internal
// copy of `cursor` advances as bytes are written, but YOUR `cursor`
// variable is never touched — reading `cursor - out` afterwards silently
// reports 0 bytes written every time, even though compression succeeded
// (recon-verified: this is exactly what a naive first draft of this shim
// did, and every compress function returned `out_len = 0` for real input).
// The fix used throughout this file: construct the `CompressorOutput`/
// `DecompressorOutput` wrapper explicitly, pass it BY REFERENCE to the
// `Internal::<Format>::Compress`/`Decompress` function, and read the true
// written length back via the wrapper's own `.Tell() - start` after the
// call returns — the wrapper object itself (not the raw pointer) is what
// actually advances.
//
// Compression output contract (mirrors sigil-salvador-sys): the caller
// allocates a buffer sized by the matching `*_max_size` helper below,
// passes it in, and gets the actual written length back via an
// out-parameter. This is NOT a convenience choice: Saxman's and Rocket's
// compressors seek backward to patch a header field (compressed size)
// after the fact (`Saxman::CompressWithHeader`, `Rocket::Compress` in the
// vendored headers), which requires a random-access *output* iterator — a
// `back_inserter` cannot support `Seek`/`Tell`. Using a raw pointer for
// every format (not just the two that need it) keeps one uniform contract.
//
// ---------------------------------------------------------------------
// DECOMPRESSION: why this shim uses std::ostringstream, not a raw pointer
// ---------------------------------------------------------------------
//
// `ClownLZSS::DecompressorOutput<T, ...>::Copy(distance, count)` implements
// LZSS's classic *self-overlapping* backward copy (used for RLE-style runs
// where distance < count, i.e. a byte written earlier in the SAME Copy call
// becomes a valid source for a later byte in that same call). The
// random-access-iterator specialization (`vendor/decompressors/common.h`,
// the `Internal::random_access_input_output_iterator` branch) implements
// this via:
//     std::copy(iterator - distance, iterator - distance + count, iterator);
// This is BROKEN for overlapping ranges under a raw-pointer iterator on
// modern standard libraries: `std::copy` over trivially-copyable types is
// permitted (and, empirically, both libstdc++ and libc++ at the versions
// used to build this crate DO) to lower to `memmove`, which does NOT give
// the "each newly-written byte becomes visible as a source for later bytes
// in the same call" semantics LZSS relies on for its self-referential runs.
// Recon-verified: compressing `tests/vectors/level_select_2p.raw` (which
// contains a repeated-word run) with the vendored Kosinski compressor and
// decompressing the result with the raw-pointer `DecompressorOutput`
// silently produced WRONG output (a truncated run, zero-filled) — while an
// independent, unrelated Kosinski decompressor
// (`programs/accurate-kosinski` elsewhere in this workspace, also by
// Clownacy but a from-scratch implementation) and this crate's OWN
// compressed bytes agreed the correct output was the full repeated run.
// This confirms the bug is in the vendored template's `Copy()`, not in our
// usage or our compressed bytes.
//
// The SAME vendored header (`vendor/decompressors/common.h`) also defines
// an `std::ostream`-hosted specialization of `DecompressorOutput` whose
// `Copy()` goes through a byte-by-byte ring-buffer loop
// (`WriteToBuffer`/`buffer[...]`) instead of `std::copy` — this IS
// overlap-correct, because each byte is written into the ring buffer (and
// thus becomes readable) before the loop advances to read it again.
// Decompression has no need for random-access *output* in the first place
// (no seek-back-and-patch step, unlike compression) — so this shim simply
// gives every decompress function a `std::ostringstream` as its output
// sink instead of a raw pointer, which selects the correct, overlap-safe
// code path. This is a shim-level (Sigil-original) design decision, NOT a
// vendored-file modification — the vendored headers are untouched; we are
// simply choosing a different (already-provided, upstream-authored)
// template instantiation of `DecompressorOutput` than a naive raw-pointer
// port would reach for.

#include <cstddef>
#include <cstring>
#include <sstream>
#include <string>
#include <utility>

#include "../vendor/compressors/clownlzss.h"
#include "../vendor/compressors/kosinski.h"
#include "../vendor/compressors/kosinskiplus.h"
#include "../vendor/compressors/saxman.h"
#include "../vendor/compressors/enigma.h"
#include "../vendor/compressors/comper.h"
#include "../vendor/compressors/rocket.h"

#include "../vendor/decompressors/common.h"
#include "../vendor/decompressors/kosinski.h"
#include "../vendor/decompressors/kosinskiplus.h"
#include "../vendor/decompressors/saxman.h"
#include "../vendor/decompressors/enigma.h"
#include "../vendor/decompressors/comper.h"
#include "../vendor/decompressors/rocket.h"

namespace {

// Shared helper for compression: run a callable that writes into a
// ClownLZSS::CompressorOutput<unsigned char*> wrapping `out`, then read the
// true written length back via the wrapper's own .Tell() (see the
// "Tell(), not the caller's pointer" note above for why this indirection
// is required). Returns false (without touching *out_len) if the vendored
// compressor itself reports failure.
template <typename F>
int RunCompress(const unsigned char* out_buf_start, std::size_t* out_len, F&& compress_fn)
{
	unsigned char* out_start = const_cast<unsigned char*>(out_buf_start);
	ClownLZSS::CompressorOutput<unsigned char*> output(out_start);
	if (!compress_fn(output))
		return 0;
	*out_len = static_cast<std::size_t>(output.Tell() - out_start);
	return 1;
}

// Shared helper for decompression: run a callable that writes into an
// std::ostringstream, then copy its bytes into the caller's C buffer
// (bounds-checked against out_capacity). Returns false (without touching
// *out_len) if the decompressed size exceeds out_capacity.
template <typename F>
int RunDecompressToOstream(unsigned char* out, std::size_t out_capacity, std::size_t* out_len, F&& decompress_fn)
{
	std::ostringstream oss(std::ios::binary | std::ios::out);
	decompress_fn(oss);
	const std::string result = oss.str();
	if (result.size() > out_capacity)
		return 0;
	std::memcpy(out, result.data(), result.size());
	*out_len = result.size();
	return 1;
}

} // namespace

extern "C" {

// ---------------------------------------------------------------------
// Max-compressed-size helpers.
//
// See `src/lib.rs` module doc for the derivation. Summary: every one of
// these formats is an LZSS variant whose worst case (fully incompressible
// input) is "everything becomes a 1-bit-flagged literal", i.e. very close
// to 1 descriptor bit per literal byte/word plus that byte/word itself,
// plus a small fixed header/footer. We use `size*9/8 + 64` uniformly (12.5%
// literal-descriptor overhead rounded generously + 64-byte constant for
// headers/terminators/code tables) which is safely above every format's
// true worst case; see the Rust doc comment for the per-format reasoning
// and the empirical sanity check in tests/byte_exact.rs.
// ---------------------------------------------------------------------

std::size_t clownlzss_max_compressed_size(std::size_t input_size)
{
	return input_size + input_size / 8 + 64;
}

// ---------------------------------------------------------------------
// Kosinski
// ---------------------------------------------------------------------

int clownlzss_kosinski_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::Kosinski::Compress(data, data_size, output);
	});
}

int clownlzss_kosinski_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::ModuledCompressionWrapper<2, ClownLZSS::Internal::Endian::Big>(data, data_size, output, ClownLZSS::Internal::Kosinski::Compress, module_size, 0x10);
	});
}

int clownlzss_kosinski_decompress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size; // The Kosinski bitstream is self-terminating (see decompressors/kosinski.h).
	return RunDecompressToOstream(out, out_capacity, out_len, [&](std::ostream& oss) {
		ClownLZSS::KosinskiDecompress(data, oss);
	});
}

int clownlzss_kosinski_decompress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	return RunDecompressToOstream(out, out_capacity, out_len, [&](std::ostream& oss) {
		ClownLZSS::ModuledKosinskiDecompress(data, oss);
	});
}

// ---------------------------------------------------------------------
// Kosinski+
// ---------------------------------------------------------------------

int clownlzss_kosinskiplus_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::KosinskiPlus::Compress(data, data_size, output);
	});
}

int clownlzss_kosinskiplus_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::ModuledCompressionWrapper<2, ClownLZSS::Internal::Endian::Big>(data, data_size, output, ClownLZSS::Internal::KosinskiPlus::Compress, module_size, 1);
	});
}

int clownlzss_kosinskiplus_decompress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	return RunDecompressToOstream(out, out_capacity, out_len, [&](std::ostream& oss) {
		ClownLZSS::KosinskiPlusDecompress(data, oss);
	});
}

int clownlzss_kosinskiplus_decompress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	return RunDecompressToOstream(out, out_capacity, out_len, [&](std::ostream& oss) {
		ClownLZSS::ModuledKosinskiPlusDecompress(data, oss);
	});
}

// ---------------------------------------------------------------------
// Saxman
// ---------------------------------------------------------------------

int clownlzss_saxman_compress_with_header(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::Saxman::CompressWithHeader(data, data_size, output);
	});
}

int clownlzss_saxman_compress_without_header(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::Saxman::CompressWithoutHeader(data, data_size, output);
	});
}

int clownlzss_saxman_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::ModuledCompressionWrapper<2, ClownLZSS::Internal::Endian::Big>(data, data_size, output, ClownLZSS::Internal::Saxman::CompressWithHeader, module_size, 2);
	});
}

// Saxman decompress with an explicit compressed length (no 2-byte header in the stream).
int clownlzss_saxman_decompress_no_header(const unsigned char* data, std::size_t compressed_length, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	return RunDecompressToOstream(out, out_capacity, out_len, [&](std::ostream& oss) {
		ClownLZSS::SaxmanDecompress(data, oss, static_cast<unsigned int>(compressed_length));
	});
}

// Saxman decompress reading the 2-byte little-endian compressed-length header from the stream itself.
int clownlzss_saxman_decompress_with_header(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	return RunDecompressToOstream(out, out_capacity, out_len, [&](std::ostream& oss) {
		ClownLZSS::SaxmanDecompress(data, oss);
	});
}

// ---------------------------------------------------------------------
// Enigma
// ---------------------------------------------------------------------

int clownlzss_enigma_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		const auto start = output.Tell();
		const bool success = ClownLZSS::Internal::Enigma::Compress(data, data_size, output);
		if (output.Distance(start) % 2 != 0)
			output.Write(0);
		return success;
	});
}

int clownlzss_enigma_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::ModuledCompressionWrapper<2, ClownLZSS::Internal::Endian::Big>(data, data_size, output, ClownLZSS::Internal::Enigma::Compress, module_size, 2);
	});
}

int clownlzss_enigma_decompress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	return RunDecompressToOstream(out, out_capacity, out_len, [&](std::ostream& oss) {
		ClownLZSS::EnigmaDecompress(data, oss);
	});
}

// ---------------------------------------------------------------------
// Comper
// ---------------------------------------------------------------------

int clownlzss_comper_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::Comper::Compress(data, data_size, output);
	});
}

int clownlzss_comper_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::ModuledCompressionWrapper<2, ClownLZSS::Internal::Endian::Big>(data, data_size, output, ClownLZSS::Internal::Comper::Compress, module_size, 2);
	});
}

int clownlzss_comper_decompress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	return RunDecompressToOstream(out, out_capacity, out_len, [&](std::ostream& oss) {
		ClownLZSS::ComperDecompress(data, oss);
	});
}

// ---------------------------------------------------------------------
// Rocket
// ---------------------------------------------------------------------

int clownlzss_rocket_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::Rocket::Compress(data, data_size, output);
	});
}

int clownlzss_rocket_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_len, [&](ClownLZSS::CompressorOutput<unsigned char*>& output) {
		return ClownLZSS::Internal::ModuledCompressionWrapper<2, ClownLZSS::Internal::Endian::Big>(data, data_size, output, ClownLZSS::Internal::Rocket::Compress, module_size, 2);
	});
}

int clownlzss_rocket_decompress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	return RunDecompressToOstream(out, out_capacity, out_len, [&](std::ostream& oss) {
		ClownLZSS::RocketDecompress(data, oss);
	});
}

} // extern "C"
