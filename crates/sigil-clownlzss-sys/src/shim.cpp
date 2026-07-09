// Sigil-original C++ shim over clownlzss's C++20 template headers.
//
// clownlzss's optimal-parse core (vendor/compressors/clownlzss.c) is C89,
// but every per-format compressor/decompressor is a template header with no
// C linkage entry point. This file drives the upstream top-level
// convenience wrappers (`ClownLZSS::KosinskiCompress`, ...) with an
// `std::ostringstream` sink — the same instantiation the upstream CLI uses
// with `std::ofstream` — and exposes one `extern "C"` function per format,
// so `src/lib.rs`'s safe wrapper can call into it without depending on the
// C++ ABI beyond `extern "C"`.
//
// Historical caution (recon-verified, kept so nobody reintroduces it): the
// top-level convenience wrappers take their iterator argument by forwarding
// reference and construct their OWN `CompressorOutput`/`DecompressorOutput`
// wrapper around a *copy* of it internally. Passing a raw pointer lvalue
// (e.g. `unsigned char* cursor = out; KosinskiCompress(data, size,
// cursor);`) advances only the wrapper's internal copy — `cursor - out`
// afterwards silently reports 0 bytes written every time, even though
// compression succeeded. Stream sinks (`std::ostream&`) do not have this
// problem: the wrapper holds a reference to the stream, not a copy of a
// cursor — and this shim now uses stream sinks exclusively, for both
// compression and decompression.
//
// Compression output contract: the caller allocates a buffer (sized by the
// Rust wrapper's worst-case bound), passes it in with its capacity, and
// gets the actual written length back via an out-parameter. Internally,
// however, compression does NOT write into the caller's buffer directly:
// it compresses into an `std::ostringstream` (exactly like the decompress
// path below) and only copies into the caller's buffer after an explicit
// capacity check. Two reasons:
//
//   1. SAFETY (the load-bearing one): the vendored compressor templates are
//      unchecked — with a raw-pointer output they write however many bytes
//      the stream needs, trusting the caller's buffer. A first version of
//      this shim did exactly that, and the Rust-side moduled bound was ~50
//      bytes short for worst-case (incompressible) multi-module input:
//      a live heap overflow (reviewer-reproduced; see
//      `tests/moduled_capacity.rs` for the RED evidence). With the
//      ostringstream intermediary the shim CANNOT write past the caller's
//      buffer regardless of what capacity Rust passes — a too-small buffer
//      is reported (return -1, required size in *out_len), never overrun.
//   2. It still satisfies the formats' random-access needs: Saxman's and
//      Rocket's compressors seek backward to patch a header field after
//      the fact (`Saxman::CompressWithHeader`, `Rocket::Compress`), which
//      a `back_inserter` cannot support — but the vendored ostream-hosted
//      `CompressorOutput` specialization (`vendor/common.h`,
//      `OutputCommon<std::ostream&>`) implements `Seek`/`Tell`/`Distance`
//      via `seekp`/`tellp`. This is upstream's own primary output path:
//      the clownlzss CLI (`main.cpp` upstream) compresses every format,
//      moduled included, straight into an `std::ofstream`.
//
// Return-code protocol shared by every compress function below:
//    1  success; *out_len = bytes written into `out`.
//    0  the vendored compressor itself reported failure.
//   -1  the compressed stream does not fit in `out_capacity`; *out_len is
//       set to the required size so the caller can retry with an
//       exactly-sized buffer. Nothing is written into `out`.
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

// Shared helper for compression: run a callable that writes into an
// std::ostringstream, then copy its bytes into the caller's C buffer,
// bounds-checked against out_capacity. The intermediary stream is what
// makes the shim incapable of overrunning the caller's buffer no matter
// what capacity the caller passes (see the "Compression output contract"
// note above, including the return-code protocol: 1 ok / 0 compressor
// failure / -1 capacity exceeded with the required size in *out_len).
template <typename F>
int RunCompress(unsigned char* out, std::size_t out_capacity, std::size_t* out_len, F&& compress_fn)
{
	std::ostringstream oss(std::ios::binary | std::ios::out);
	if (!compress_fn(oss))
		return 0;
	const std::string result = oss.str();
	if (result.size() > out_capacity)
	{
		*out_len = result.size();
		return -1;
	}
	std::memcpy(out, result.data(), result.size());
	*out_len = result.size();
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

int clownlzss_kosinski_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::KosinskiCompress(data, data_size, oss);
	});
}

int clownlzss_kosinski_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::ModuledKosinskiCompress(data, data_size, oss, module_size);
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

int clownlzss_kosinskiplus_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::KosinskiPlusCompress(data, data_size, oss);
	});
}

int clownlzss_kosinskiplus_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::ModuledKosinskiPlusCompress(data, data_size, oss, module_size);
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

int clownlzss_saxman_compress_with_header(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::SaxmanCompressWithHeader(data, data_size, oss);
	});
}

int clownlzss_saxman_compress_without_header(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::SaxmanCompressWithoutHeader(data, data_size, oss);
	});
}

int clownlzss_saxman_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::ModuledSaxmanCompress(data, data_size, oss, module_size);
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

// Note: upstream's top-level EnigmaCompress itself appends the odd-length
// pad byte (word-align the stream) — the padding an earlier version of this
// shim replicated by hand when it called Internal::Enigma::Compress
// directly. Byte-identical output either way (golden-gated).
int clownlzss_enigma_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::EnigmaCompress(data, data_size, oss);
	});
}

int clownlzss_enigma_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::ModuledEnigmaCompress(data, data_size, oss, module_size);
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

int clownlzss_comper_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::ComperCompress(data, data_size, oss);
	});
}

int clownlzss_comper_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::ModuledComperCompress(data, data_size, oss, module_size);
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

int clownlzss_rocket_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::RocketCompress(data, data_size, oss);
	});
}

int clownlzss_rocket_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len, std::size_t module_size)
{
	return RunCompress(out, out_capacity, out_len, [&](std::ostream& oss) {
		return ClownLZSS::ModuledRocketCompress(data, data_size, oss, module_size);
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
