// Sigil-original C++ shim over clownlzss's C++20 template headers.
//
// clownlzss's optimal-parse core (vendor/compressors/clownlzss.c) is C89,
// but every per-format compressor/decompressor is a template header with no
// C linkage entry point. This file instantiates each template against a
// raw `unsigned char*` (a random-access iterator) and exposes one
// `extern "C"` function per format, so `src/lib.rs`'s safe wrapper can call
// into it without depending on the C++ ABI beyond `extern "C"`.
//
// Output contract (mirrors sigil-salvador-sys): the caller allocates a
// buffer sized by the matching `*_max_size` helper below, passes it in, and
// gets the actual written length back via an out-parameter. This is NOT a
// convenience choice: Saxman's and Rocket's compressors seek backward to
// patch a header field (compressed size) after the fact
// (`Saxman::CompressWithHeader`, `Rocket::Compress` in the vendored
// headers), which requires a random-access *output* iterator — a
// `back_inserter` cannot support `Seek`/`Tell`. Using a raw pointer for
// every format (not just the two that need it) keeps one uniform contract.
//
// Decompression bounds-checking: the vendored decompressor templates write
// through the output iterator with no bounds check of their own (see
// `ClownLZSS::DecompressorOutput::Copy`/`Write` in
// `vendor/decompressors/common.h` — they trust the caller's buffer is large
// enough). This shim guards decompression by giving each decompress
// function a caller-supplied maximum output size; if decompression writes
// past a small sentinel margin we cannot fully trust the templates to stop
// exactly at the boundary (they don't take one), so instead this shim
// requires the caller to size its buffer generously (see the *_max_size
// helpers, which are compression-side sizing; decompression callers in the
// Rust wrapper use a generous multiplicative growth strategy and re-try on
// a `written == capacity` heuristic — documented in `src/lib.rs`).

#include <cstddef>
#include <cstring>

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
	unsigned char* cursor = out;
	if (!ClownLZSS::KosinskiCompress(data, data_size, cursor))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_kosinski_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::ModuledKosinskiCompress(data, data_size, cursor, module_size))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_kosinski_decompress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size; // The Kosinski bitstream is self-terminating (see decompressors/kosinski.h).
	unsigned char* out_start = out;
	ClownLZSS::DecompressorInput<const unsigned char*> input_wrapped(data);
	ClownLZSS::Internal::Kosinski::DecompressorOutput<unsigned char*> output_wrapped(out_start);
	ClownLZSS::Internal::Kosinski::Decompress(input_wrapped, output_wrapped);
	const std::size_t written = static_cast<std::size_t>(output_wrapped.Tell() - out_start);
	if (written > out_capacity)
		return 0;
	*out_len = written;
	return 1;
}

int clownlzss_kosinski_decompress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	unsigned char* out_start = out;
	ClownLZSS::DecompressorInput<const unsigned char*> input_wrapped(data);
	ClownLZSS::Internal::Kosinski::DecompressorOutput<unsigned char*> output_wrapped(out_start);
	ClownLZSS::Internal::ModuledDecompressionWrapper<2, ClownLZSS::Internal::Endian::Big>(input_wrapped, output_wrapped, ClownLZSS::Internal::Kosinski::Decompress, 0x10);
	const std::size_t written = static_cast<std::size_t>(output_wrapped.Tell() - out_start);
	if (written > out_capacity)
		return 0;
	*out_len = written;
	return 1;
}

// ---------------------------------------------------------------------
// Kosinski+
// ---------------------------------------------------------------------

int clownlzss_kosinskiplus_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::KosinskiPlusCompress(data, data_size, cursor))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_kosinskiplus_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::ModuledKosinskiPlusCompress(data, data_size, cursor, module_size))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_kosinskiplus_decompress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	unsigned char* out_start = out;
	ClownLZSS::DecompressorInput<const unsigned char*> input_wrapped(data);
	ClownLZSS::Internal::KosinskiPlus::DecompressorOutput<unsigned char*> output_wrapped(out_start);
	ClownLZSS::Internal::KosinskiPlus::Decompress(input_wrapped, output_wrapped);
	const std::size_t written = static_cast<std::size_t>(output_wrapped.Tell() - out_start);
	if (written > out_capacity)
		return 0;
	*out_len = written;
	return 1;
}

int clownlzss_kosinskiplus_decompress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	unsigned char* out_start = out;
	ClownLZSS::DecompressorInput<const unsigned char*> input_wrapped(data);
	ClownLZSS::Internal::KosinskiPlus::DecompressorOutput<unsigned char*> output_wrapped(out_start);
	ClownLZSS::Internal::ModuledDecompressionWrapper<2, ClownLZSS::Internal::Endian::Big>(input_wrapped, output_wrapped, ClownLZSS::Internal::KosinskiPlus::Decompress, 1);
	const std::size_t written = static_cast<std::size_t>(output_wrapped.Tell() - out_start);
	if (written > out_capacity)
		return 0;
	*out_len = written;
	return 1;
}

// ---------------------------------------------------------------------
// Saxman
// ---------------------------------------------------------------------

int clownlzss_saxman_compress_with_header(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::SaxmanCompressWithHeader(data, data_size, cursor))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_saxman_compress_without_header(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::SaxmanCompressWithoutHeader(data, data_size, cursor))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_saxman_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::ModuledSaxmanCompress(data, data_size, cursor, module_size))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

// Saxman decompress with an explicit compressed length (no 2-byte header in the stream).
int clownlzss_saxman_decompress_no_header(const unsigned char* data, std::size_t compressed_length, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	unsigned char* out_start = out;
	ClownLZSS::DecompressorInput<const unsigned char*> input_wrapped(data);
	ClownLZSS::Internal::Saxman::DecompressorOutput<unsigned char*> output_wrapped(out_start);
	ClownLZSS::Internal::Saxman::Decompress(input_wrapped, output_wrapped, static_cast<unsigned int>(compressed_length));
	const std::size_t written = static_cast<std::size_t>(output_wrapped.Tell() - out_start);
	if (written > out_capacity)
		return 0;
	*out_len = written;
	return 1;
}

// Saxman decompress reading the 2-byte little-endian compressed-length header from the stream itself.
int clownlzss_saxman_decompress_with_header(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	unsigned char* out_start = out;
	ClownLZSS::DecompressorInput<const unsigned char*> input_wrapped(data);
	ClownLZSS::Internal::Saxman::DecompressorOutput<unsigned char*> output_wrapped(out_start);
	ClownLZSS::Internal::Saxman::Decompress(input_wrapped, output_wrapped);
	const std::size_t written = static_cast<std::size_t>(output_wrapped.Tell() - out_start);
	if (written > out_capacity)
		return 0;
	*out_len = written;
	return 1;
}

// ---------------------------------------------------------------------
// Enigma
// ---------------------------------------------------------------------

int clownlzss_enigma_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::EnigmaCompress(data, data_size, cursor))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_enigma_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::ModuledEnigmaCompress(data, data_size, cursor, module_size))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_enigma_decompress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	unsigned char* out_start = out;
	ClownLZSS::DecompressorInput<const unsigned char*> input_wrapped(data);
	ClownLZSS::Internal::Enigma::DecompressorOutput<unsigned char*> output_wrapped(out_start);
	ClownLZSS::Internal::Enigma::Decompress(input_wrapped, output_wrapped);
	const std::size_t written = static_cast<std::size_t>(output_wrapped.Tell() - out_start);
	if (written > out_capacity)
		return 0;
	*out_len = written;
	return 1;
}

// ---------------------------------------------------------------------
// Comper
// ---------------------------------------------------------------------

int clownlzss_comper_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::ComperCompress(data, data_size, cursor))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_comper_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::ModuledComperCompress(data, data_size, cursor, module_size))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_comper_decompress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	unsigned char* out_start = out;
	ClownLZSS::DecompressorInput<const unsigned char*> input_wrapped(data);
	ClownLZSS::Internal::Comper::DecompressorOutput<unsigned char*> output_wrapped(out_start);
	ClownLZSS::Internal::Comper::Decompress(input_wrapped, output_wrapped);
	const std::size_t written = static_cast<std::size_t>(output_wrapped.Tell() - out_start);
	if (written > out_capacity)
		return 0;
	*out_len = written;
	return 1;
}

// ---------------------------------------------------------------------
// Rocket
// ---------------------------------------------------------------------

int clownlzss_rocket_compress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::RocketCompress(data, data_size, cursor))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_rocket_compress_moduled(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t* out_len, std::size_t module_size)
{
	unsigned char* cursor = out;
	if (!ClownLZSS::ModuledRocketCompress(data, data_size, cursor, module_size))
		return 0;
	*out_len = static_cast<std::size_t>(cursor - out);
	return 1;
}

int clownlzss_rocket_decompress(const unsigned char* data, std::size_t data_size, unsigned char* out, std::size_t out_capacity, std::size_t* out_len)
{
	(void)data_size;
	unsigned char* out_start = out;
	ClownLZSS::DecompressorInput<const unsigned char*> input_wrapped(data);
	ClownLZSS::Internal::Rocket::DecompressorOutput<unsigned char*> output_wrapped(out_start);
	ClownLZSS::Internal::Rocket::Decompress(input_wrapped, output_wrapped);
	const std::size_t written = static_cast<std::size_t>(output_wrapped.Tell() - out_start);
	if (written > out_capacity)
		return 0;
	*out_len = written;
	return 1;
}

} // extern "C"
