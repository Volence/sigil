//! Sonic 2, whole: given the instruction s2disasm's `build.lua` gives `p2bin`,
//! the AS route builds the ROM `build.lua` builds, byte for byte, with nothing
//! excluded.
//!
//! The reference is not stored. It is built here by running `build.lua` itself,
//! unmodified, in an extraction of s2disasm at the revision below, with the
//! asl, p2bin and saxman that revision ships, each checked against its md5
//! before it runs. That asl is s2disasm's own build (md5 `0dee1f98...`), not
//! the pinned reference asl: the pinned build cuts macro arguments at 255
//! characters and cannot assemble Sonic 2 at all (274 undefined `JmpTo` labels,
//! the residual census). The s2 build's known defect is a varying value for an
//! operand it declines, so its ROM is pinned by CRC32 and size (measured
//! 2026-09-11, the census's md5 `9feeb724052c39982d432a7851c98d3e`), and a
//! change in the instrument reddens as one, not as a sigil defect.
//!
//! `build.lua` does more than assemble. Before it does, it compresses the songs
//! (assembling each with that asl, then `saxman -a`) and converts the WAV
//! samples into gitignored `generated/` includes; sigil assembles the tree those
//! steps leave, which is why the reference is built first, in the same
//! directory. After `p2bin` it patches the driver's compressed size into the
//! `move.w` at `movewZ80CompSize+2` and fixes the header; at this revision both
//! change zero bytes (measured: the p2bin image and the final ROM share one
//! md5), so sigil's image is compared with the final ROM.
//!
//! The corpus comes from the suite root (the directory holding `aeon/` and
//! `empyrean/` beside this checkout, or `EMPYREAN_SUITE_ROOT`). Only its
//! `s2disasm` checkout is read, through `git archive` of the pinned revision,
//! into a directory under this crate's target tmp dir; the checkout itself is
//! never built in or written.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sigil_harness::test_support::{suite_root_absent, unnamed_default_tree};

const WHAT: &str = "Sonic 2 built whole by the AS route with its build script's p2bin instruction";
const REV: &str = "e45ebf332f39987424ca3102e50c717628f71269";
const ASL_MD5: &str = "0dee1f98e6480a4783d27ffd8b90896f";
const P2BIN_MD5: &str = "4f2fff99c3347bafb93b12d5be1db754";
const SAXMAN_MD5: &str = "704e5c8c361b9959f45bac3e803f0033";
/// `build.lua` line 186, with `improved_sound_driver_compression = false`.
const INSTRUCTION: [&str; 2] = ["-p=0", "-z=0,saxman-bugged,Size_of_Snd_driver_guess,after"];
const REF_LEN: usize = 1_048_576;
const REF_CRC: u32 = 0x7b90_5383;

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn md5_of(path: &Path) -> String {
    let out = Command::new("md5sum").arg(path).output().expect("spawn md5sum");
    assert!(out.status.success(), "md5sum {}", path.display());
    String::from_utf8_lossy(&out.stdout).split_whitespace().next().unwrap_or("").to_string()
}

/// The suite root by the harness's own precedence (its variable, then the
/// derivation from this checkout), read through the harness and never here: the
/// tree a run resolves to when nobody names one sits directly under the root.
fn suite_root() -> Option<PathBuf> {
    match unnamed_default_tree() {
        Ok(tree) => Some(tree.path.parent().expect("a tree under the suite root has a parent").to_path_buf()),
        Err(why) => {
            suite_root_absent(WHAT, &why);
            None
        }
    }
}

#[test]
fn sonic_2_builds_to_the_reference_rom_byte_for_byte() {
    let Some(root) = suite_root() else { return };
    let s2 = root.join("s2disasm");
    if !s2.join(".git").exists() {
        suite_root_absent(WHAT, &format!("the suite root {} holds no s2disasm checkout", root.display()));
        return;
    }

    let work = tempfile::Builder::new()
        .prefix("s2-whole-rom-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .expect("a directory under the target tmp dir");
    let tree = work.path();
    let archive = Command::new("git")
        .arg("-C")
        .arg(&s2)
        .args(["archive", "--format=tar", REV])
        .output()
        .expect("spawn git");
    assert!(
        archive.status.success(),
        "{} has no revision {REV}, which is the revision this row measures: {}",
        s2.display(),
        String::from_utf8_lossy(&archive.stderr)
    );
    let mut tar = Command::new("tar").arg("-x").arg("-C").arg(tree).stdin(Stdio::piped()).spawn().expect("spawn tar");
    tar.stdin.take().expect("tar stdin").write_all(&archive.stdout).expect("feed tar");
    assert!(tar.wait().expect("tar").success(), "tar could not unpack the archive");

    let tools = tree.join("build_tools/Linux-x86_64");
    assert_eq!(md5_of(&tools.join("asl")), ASL_MD5, "the revision's asl is not the build this row measured with");
    assert_eq!(md5_of(&tools.join("p2bin")), P2BIN_MD5, "the revision's p2bin is not the pinned build");
    assert_eq!(md5_of(&tools.join("saxman")), SAXMAN_MD5, "the revision's saxman is not the pinned build");

    // The reference, and the generated inputs sigil needs: build.lua, as shipped.
    let lua = Command::new("lua")
        .current_dir(tree)
        .arg("build.lua")
        .output()
        .expect("spawn lua, which s2disasm's build.lua needs");
    assert!(
        lua.status.success(),
        "build.lua failed:\n{}\n{}",
        String::from_utf8_lossy(&lua.stdout),
        String::from_utf8_lossy(&lua.stderr)
    );
    let reference = std::fs::read(tree.join("s2built.bin")).expect("build.lua's ROM");
    assert_eq!(
        (reference.len(), crc32(&reference)),
        (REF_LEN, REF_CRC),
        "the reference toolchain's ROM is not the one measured on 2026-09-11"
    );

    let built = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .current_dir(tree)
        .args(["s2.asm", "-o", "sigil.bin"])
        .args(INSTRUCTION)
        .output()
        .expect("spawn sigil");
    let stderr = String::from_utf8_lossy(&built.stderr);
    assert!(built.status.success(), "sigil failed:\n{stderr}");
    assert!(
        !stderr.lines().any(|l| l.contains(": error:")),
        "sigil succeeded while printing an error row:\n{stderr}"
    );
    let image = std::fs::read(tree.join("sigil.bin")).expect("sigil's ROM");
    if image != reference {
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for x in 0..image.len().min(reference.len()) {
            if image[x] != reference[x] {
                match runs.last_mut() {
                    Some(r) if r.1 == x => r.1 = x + 1,
                    _ => runs.push((x, x + 1)),
                }
            }
        }
        let shown: Vec<String> = runs.iter().take(20).map(|(a, b)| format!("[{a:#08X}, {b:#08X})")).collect();
        panic!(
            "sigil's ROM is {} bytes, the reference {}; {} bytes differ in {} runs, the first: {}",
            image.len(),
            reference.len(),
            runs.iter().map(|(a, b)| b - a).sum::<usize>(),
            runs.len(),
            shown.join(" ")
        );
    }
}
