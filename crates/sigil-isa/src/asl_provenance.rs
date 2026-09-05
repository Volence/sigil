//! Identify the asl build that minted a golden, at the point of use.
//!
//! The three vector generators (`gen-m68k-vectors`, `gen-z80-vectors`,
//! `gen_snippet_vectors`) take their assembler from `ASL_BIN` with no default.
//! That hook is deliberate: `asl` was deleted from the tree at the P4d flip, so
//! extending a corpus means pointing the generator at an out-of-repo install.
//! The defect it left behind is that the goldens those runs write — the frozen
//! "an independent implementation said so" witness that CI compares against
//! forever — recorded nothing about which implementation said so.
//!
//! This module STAMPS, it does not CONSTRAIN. It records the identity of
//! whatever binary the generator was handed and makes writing a golden
//! conditional on being able to record it. Which build is the caller's choice;
//! that the choice is written down is not.
//!
//! **Identity is the digest, never the banner.** Four `asl` binaries in this
//! workspace print `Macro Assembler 1.42 Beta [Bld 212]` verbatim and are not
//! the same program (`docs/superpowers/notes/asl-reference/README.md`). The
//! banner is recorded beside the digest as human context; it discriminates
//! nothing on its own.
//!
//! **The install path is deliberately not recorded.** A path is not a property
//! of the binary — it is where one copy happens to sit — and writing it in
//! would make the header churn between two machines holding the byte-identical
//! build, training readers to ignore exactly the diff that matters. The
//! generators print the path to stderr instead, where it is context for the
//! person running the mint rather than committed content.
//!
//! **Every value here is read from the binary's bytes at run time.** A digest
//! written as a literal in this file would move with whatever it was copied
//! from and could never disagree with the tool actually run; the test
//! `no_digest_literal_in_this_source` holds that line.

use std::path::Path;
use std::process::Command;

/// What one tool binary is, as read from the binary itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolIdentity {
    /// MD5 of the file's bytes, computed here, never quoted from anywhere.
    pub md5: String,
    /// The version lines the tool prints, or empty when it prints none.
    pub banner: Vec<String>,
}

/// The instrument behind one minting run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// The generator binary's own name, so a header names the tool that wrote it.
    pub minted_by: String,
    pub asl: ToolIdentity,
    pub p2bin: ToolIdentity,
}

impl Provenance {
    /// Read both binaries and describe them.
    ///
    /// Returns `Err` rather than a partial stamp: a generator must refuse to
    /// write a golden it cannot identify, and a header with a hole in it is
    /// worse than a loud failure because it still looks like provenance.
    pub fn capture(minted_by: &str, asl: &Path, p2bin: &Path) -> Result<Self, String> {
        Ok(Provenance {
            minted_by: minted_by.to_string(),
            asl: identify(asl, BannerSource::FirstTwoStdoutLines)?,
            p2bin: identify(p2bin, BannerSource::None)?,
        })
    }

    /// The header for a `<snippet> => <hex>` golden, whose readers already skip
    /// `#` lines (`tests/golden_common`, `tests/m68k_common`, `completeness.rs`,
    /// `sigil-frontend-as/tests/isa_golden.rs`).
    pub fn header(&self) -> String {
        let mut s = String::new();
        for line in PREAMBLE {
            // No trailing space on a blank comment line — it is invisible churn
            // that survives every future diff of this header.
            if line.is_empty() {
                s.push_str("#\n");
            } else {
                s.push_str(&format!("# {line}\n"));
            }
        }
        push_field(&mut s, "minted-by", &self.minted_by);
        push_field(&mut s, "asl-md5", &self.asl.md5);
        push_banner(&mut s, "asl", &self.asl.banner);
        push_field(&mut s, "p2bin-md5", &self.p2bin.md5);
        push_banner(&mut s, "p2bin", &self.p2bin.banner);
        s.push('\n');
        s
    }
}

/// Field-label column width, sized to the longest label (`p2bin-banner`).
const LABEL_W: usize = 12;

fn push_field(s: &mut String, label: &str, value: &str) {
    s.push_str(&format!("# {label:<LABEL_W$}  {value}\n"));
}

fn push_banner(s: &mut String, tool: &str, banner: &[String]) {
    let label = format!("{tool}-banner");
    if banner.is_empty() {
        push_field(
            s,
            &label,
            "(none, this tool prints no version string, so its digest is its",
        );
        s.push_str(&format!(
            "# {:<LABEL_W$}  only identity, which is the general case, stated plainly)\n",
            ""
        ));
    } else {
        for line in banner {
            push_field(s, &label, line);
        }
    }
}

/// The fixed prose above the derived values. It is worded as a PROHIBITION on
/// the misreading, not a hedge about it: a reader who takes a provenance line
/// for a correctness claim has made the exact error this header exists to
/// prevent, so the header says so rather than merely qualifying itself.
const PREAMBLE: &[&str] = &[
    "PROVENANCE, generated. This records WHICH BUILD ANSWERED.",
    "",
    "Do NOT read it as a claim that the answer is correct, and do not cite it as one.",
    "asl substitutes a value for an operand it declines to evaluate rather than",
    "refusing to emit, and on a given build that substitute is stable, it echoes the",
    "last value that build computed, so it re-mints identically and reads like a",
    "measurement. Re-derivability is evidence about the INSTRUMENT and never about",
    "the vectors. The question a digest cannot answer is whether asl answered at all;",
    "for that see docs/superpowers/notes/2026-09-05-asl-gen-vector-provenance-probes/.",
    "",
    "Identity is the digest. Four asl builds in this workspace print the same version",
    "banner and are different programs, so the banner below discriminates nothing; it",
    "is human context. The install path is deliberately absent, it is not a property",
    "of the binary and would churn this header between machines holding the identical",
    "build. Regenerate with the generator named below; a diff here means the",
    "instrument changed.",
];

enum BannerSource {
    /// Run the tool with no arguments and keep the first two stdout lines.
    FirstTwoStdoutLines,
    /// The tool prints no version string.
    None,
}

fn identify(path: &Path, banner: BannerSource) -> Result<ToolIdentity, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("cannot read {} to identify it: {e}", path.display()))?;
    if bytes.is_empty() {
        return Err(format!("{} is empty, refusing to stamp it", path.display()));
    }
    let md5 = md5_hex(&bytes);
    let banner = match banner {
        BannerSource::None => Vec::new(),
        BannerSource::FirstTwoStdoutLines => {
            // asl with no arguments prints its banner on stdout and exits 1, so
            // the exit status is not a failure signal here and is not checked.
            let out = Command::new(path)
                .env("USEANSI", "n")
                .output()
                .map_err(|e| format!("cannot run {} to read its banner: {e}", path.display()))?;
            let text = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<String> =
                text.lines().take(2).map(|l| l.trim_end().to_string()).collect();
            if lines.is_empty() || lines[0].is_empty() {
                return Err(format!(
                    "{} printed no banner on stdout, refusing to stamp a tool it \
                     cannot describe",
                    path.display()
                ));
            }
            lines
        }
    };
    Ok(ToolIdentity { md5, banner })
}

// ── MD5 ──────────────────────────────────────────────────────────────────────
// RFC 1321. Implemented here because the generators' crates carry no third-party
// dependencies and shelling out to `md5sum` would make the stamp depend on a
// tool that may be absent — a stamp that can silently not happen is the defect
// this module exists to close.

/// MD5 of `data`, lowercase hex.
pub fn md5_hex(data: &[u8]) -> String {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    // K[i] = floor(2^32 * abs(sin(i + 1))), the RFC's table.
    let mut k = [0u32; 64];
    for (i, slot) in k.iter_mut().enumerate() {
        *slot = ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32;
    }

    let mut msg = data.to_vec();
    let bitlen = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_le_bytes());

    let (mut a0, mut b0, mut c0, mut d0) =
        (0x6745_2301u32, 0xefcd_ab89u32, 0x98ba_dcfeu32, 0x1032_5476u32);

    // `as_chunks::<64>().0` — the padding above guarantees a whole number of
    // 64-byte blocks, so the remainder is empty by construction.
    for chunk in msg.as_chunks::<64>().0 {
        let mut m = [0u32; 16];
        for (i, slot) in m.iter_mut().enumerate() {
            *slot = u32::from_le_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let tmp = d;
            d = c;
            c = b;
            let sum = a
                .wrapping_add(f)
                .wrapping_add(k[i])
                .wrapping_add(m[g]);
            b = b.wrapping_add(sum.rotate_left(S[i]));
            a = tmp;
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut out = String::with_capacity(32);
    for word in [a0, b0, c0, d0] {
        for byte in word.to_le_bytes() {
            out.push_str(&format!("{byte:02x}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The expectations are RFC 1321's own published test suite — an independent
    /// authority, not a value this implementation produced and then froze.
    #[test]
    fn md5_matches_rfc1321_test_suite() {
        let cases: &[(&str, &str)] = &[
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("a", "0cc175b9c0f1b6a831c399e269772661"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
            ("abcdefghijklmnopqrstuvwxyz", "c3fcd3d76192e4007dfb496cca67e13b"),
            (
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
                "d174ab98d277d9f5a5611c2c9f419d9f",
            ),
            (
                "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "57edf4a22be3c955ac49da2e2107b67a",
            ),
        ];
        let mut bad = Vec::new();
        for (input, want) in cases {
            let got = md5_hex(input.as_bytes());
            if got != *want {
                bad.push(format!("md5({input:?}) = {got}, want {want}"));
            }
        }
        assert!(bad.is_empty(), "{} RFC 1321 vectors failed:\n{}", bad.len(), bad.join("\n"));
    }

    /// A digest that came from a literal would be the same whatever it was
    /// handed. This asserts the stamp TRACKS ITS INPUT: two files differing in
    /// one byte must stamp differently, and each must equal the value RFC 1321
    /// fixes for those bytes.
    #[test]
    fn stamp_tracks_the_bytes_it_is_given() {
        let dir = std::env::temp_dir().join("sigil_asl_provenance_derive_test");
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        let one = dir.join("abc");
        let two = dir.join("abcd");
        std::fs::write(&one, b"abc").expect("write");
        std::fs::write(&two, b"abcd").expect("write");

        let a = identify(&one, BannerSource::None).expect("identify abc");
        let b = identify(&two, BannerSource::None).expect("identify abcd");

        assert_eq!(a.md5, "900150983cd24fb0d6963f7d28e17f72", "md5 of the file's bytes");
        assert_eq!(b.md5, "e2fc714c4727ee9395f324cd2e7f331f", "md5 of the file's bytes");
        assert_ne!(a.md5, b.md5, "a stamp that did not move with its input is a constant");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A tool that cannot be read must not be stamped as if it had been.
    #[test]
    fn refuses_a_binary_it_cannot_identify() {
        let missing = std::env::temp_dir().join("sigil_asl_provenance_no_such_binary");
        let _ = std::fs::remove_file(&missing);
        let err = identify(&missing, BannerSource::None).expect_err("must refuse");
        assert!(err.contains("cannot read"), "refusal must say why: {err}");

        let dir = std::env::temp_dir().join("sigil_asl_provenance_empty_test");
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        let empty = dir.join("empty");
        std::fs::write(&empty, b"").expect("write");
        let err = identify(&empty, BannerSource::None).expect_err("must refuse an empty file");
        assert!(err.contains("is empty"), "refusal must say why: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THE LINE THIS MODULE MUST HOLD. A provenance value derived from a
    /// constant in the source moves with whatever it was copied from and can
    /// never disagree with the tool actually run — an expectation derived from
    /// its own subject. Nothing that could be a digest may appear as a literal
    /// here.
    ///
    /// This is the inverse of the guard in `docs/superpowers/notes/asl-reference/`,
    /// where the wanted digest IS written out on purpose. That is a GATE and its
    /// expectation must not move; this is a STAMP and its value must.
    #[test]
    fn no_digest_literal_in_this_source() {
        let src = include_str!("asl_provenance.rs");
        let mut found = Vec::new();
        for (n, line) in src.lines().enumerate() {
            // The test's own RFC vectors are the deliberate exception: they are
            // digests OF KNOWN STRINGS, checked against a published authority,
            // and no header value is ever taken from them.
            if line.contains("d41d8cd98f00b204") || n > rfc_block_start(src) {
                continue;
            }
            for word in line.split(|c: char| !c.is_ascii_alphanumeric()) {
                if word.len() == 32 && word.chars().all(|c| c.is_ascii_hexdigit()) {
                    found.push(format!("line {}: {word}", n + 1));
                }
            }
        }
        assert!(
            found.is_empty(),
            "{} hex-digest literal(s) outside the RFC test block, a stamp must be \
             derived from the binary, never quoted:\n{}",
            found.len(),
            found.join("\n")
        );
    }

    /// Line index at which the RFC 1321 expectation table begins; everything
    /// after it is test data checked against a published authority.
    fn rfc_block_start(src: &str) -> usize {
        src.lines()
            .position(|l| l.contains("fn md5_matches_rfc1321_test_suite"))
            .expect("the RFC test must exist for this exemption to mean anything")
    }
}
