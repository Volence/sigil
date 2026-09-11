//! The compressors `-z` placement uses on the AS route: p2bin's authentic
//! Kosinski and Saxman to store a blob, and sigil's own decompressors to read the
//! stored stream back for the round-trip check. The linker takes them as a
//! [`sigil_link::BlobCodec`] because it depends on nothing but the IR.

use sigil_link::{BlobCodec, BlobFormat};

/// The AS route's codec.
pub(crate) struct Codec;

impl BlobCodec for Codec {
    fn compress(&self, format: BlobFormat, data: &[u8]) -> Vec<u8> {
        match format {
            BlobFormat::Uncompressed => data.to_vec(),
            BlobFormat::Kosinski => sigil_clownlzss_sys::accurate::compress_kosinski_authentic(data),
            BlobFormat::SaxmanBugged => sigil_clownlzss_sys::accurate::compress_saxman_bugged(data),
        }
    }

    /// Saxman is read as Sonic 2's decompressor reads it: the stored length
    /// counts the junk byte, which is never used, and a match whose source starts
    /// before the output begins is zeros throughout.
    fn decompress(&self, format: BlobFormat, stored: &[u8]) -> Result<Vec<u8>, String> {
        match format {
            BlobFormat::Uncompressed => Ok(stored.to_vec()),
            BlobFormat::Kosinski => sigil_clownlzss_sys::decompress_kosinski(stored).map_err(|e| e.to_string()),
            BlobFormat::SaxmanBugged => {
                let used = stored.len().saturating_sub(1);
                sigil_clownlzss_sys::decompress_saxman_no_header(stored, used).map_err(|e| e.to_string())
            }
        }
    }
}
