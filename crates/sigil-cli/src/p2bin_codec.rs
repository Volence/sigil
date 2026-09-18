//! The compressors `-z` placement uses on the AS route: p2bin's own compressors
//! to store a blob, and sigil's own decompressors to read the stored stream back
//! for the round-trip check. The linker takes them as a
//! [`sigil_link::BlobCodec`] because it depends on nothing but the IR.
//!
//! p2bin offers each of Kosinski and Saxman twice: an "authentic" compressor,
//! which is Sega's own greedy one, and an "optimised" one, which is clownlzss's
//! optimal parser. A build script picks between them (Sonic 1's
//! `improved_dac_driver_compression`, Sonic 2's
//! `improved_sound_driver_compression`), so sigil has to write both. The two
//! halves of a pair share a container: an optimised stream is read back by the
//! same decompressor as its authentic sibling.
//!
//! Which Rust function is which p2bin format was MEASURED, not assumed: each of
//! the fourteen generated blobs in
//! `docs/superpowers/notes/2026-09-18-p2bin-optimised-compressors/` was handed to
//! the p2bin binary (md5 `4f2fff99c3347bafb93b12d5be1db754`) under all five
//! compressed formats and the stored stream compared byte for byte with what
//! these calls produce; the pins live in `sigil-clownlzss-sys`'s
//! `p2bin_optimised_vectors.rs` and `accurate_vectors.rs`.

use sigil_link::{BlobCodec, BlobFormat};

/// The AS route's codec.
pub(crate) struct Codec;

/// The blob is at most `sigil_link::MAX_BLOB` bytes, refused before it reaches a
/// compressor, and every compressor below allocates its output through the
/// shim's capacity protocol, which retries once at the exact size the shim
/// reports. What is left is an allocation failure inside the C core, which is
/// not a program sigil can go on linking.
fn expect_compressed(format: BlobFormat, r: Result<Vec<u8>, sigil_clownlzss_sys::Error>) -> Vec<u8> {
    match r {
        Ok(out) => out,
        Err(e) => panic!("the {} compressor failed on a blob of at most {:#X} bytes: {e}", format.name(), sigil_link::MAX_BLOB),
    }
}

impl BlobCodec for Codec {
    fn compress(&self, format: BlobFormat, data: &[u8]) -> Vec<u8> {
        match format {
            BlobFormat::Uncompressed => data.to_vec(),
            BlobFormat::Kosinski => sigil_clownlzss_sys::accurate::compress_kosinski_authentic(data),
            BlobFormat::KosinskiOptimised => expect_compressed(format, sigil_clownlzss_sys::compress_kosinski(data)),
            BlobFormat::Saxman => sigil_clownlzss_sys::accurate::compress_saxman_authentic(data),
            BlobFormat::SaxmanBugged => sigil_clownlzss_sys::accurate::compress_saxman_bugged(data),
            // Header-less: p2bin's `-z` writes the stored size into the header
            // file it is given, not into the stream.
            BlobFormat::SaxmanOptimised => expect_compressed(format, sigil_clownlzss_sys::compress_saxman(data, false)),
        }
    }

    /// Saxman is read as Sonic 2's decompressor reads it: a match whose source
    /// starts before the output begins is zeros throughout. For `saxman-bugged`
    /// the stored length counts the junk byte, which is never used.
    fn decompress(&self, format: BlobFormat, stored: &[u8]) -> Result<Vec<u8>, String> {
        match format {
            BlobFormat::Uncompressed => Ok(stored.to_vec()),
            BlobFormat::Kosinski | BlobFormat::KosinskiOptimised => {
                sigil_clownlzss_sys::decompress_kosinski(stored).map_err(|e| e.to_string())
            }
            BlobFormat::Saxman | BlobFormat::SaxmanOptimised => {
                sigil_clownlzss_sys::decompress_saxman_no_header(stored, stored.len()).map_err(|e| e.to_string())
            }
            BlobFormat::SaxmanBugged => {
                let used = stored.len().saturating_sub(1);
                sigil_clownlzss_sys::decompress_saxman_no_header(stored, used).map_err(|e| e.to_string())
            }
        }
    }
}
