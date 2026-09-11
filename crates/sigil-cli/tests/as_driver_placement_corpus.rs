//! Sonic 1, whole: given the instruction s1disasm's `build.lua` gives `p2bin`,
//! the AS route builds the ROM asl + p2bin build, byte for byte, with nothing
//! excluded.
//!
//! The reference is not stored. It is built here, from s1disasm at the revision
//! below, by the asl and p2bin that revision ships, each checked against its md5
//! before it runs, so the comparison is with what the two binaries write today.
//! Its CRC32 and size are pinned as well (measured 2026-09-11, and equal to the
//! md5 `09dadb5071eb35050067a32462e39c5f` the phased-overlap note recorded), so
//! a change in the instrument reddens as one, not as a sigil defect.
//!
//! The corpus comes from the suite root (the directory holding `aeon/` and
//! `empyrean/` beside this checkout, or `EMPYREAN_SUITE_ROOT`). Only its
//! `s1disasm` checkout is read, through `git archive` of the pinned revision,
//! into a directory under this crate's target tmp dir; the checkout itself is
//! never built in or written, and its working-tree edits never reach the build.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sigil_harness::test_support::{suite_root_absent, unnamed_default_tree};

const WHAT: &str = "Sonic 1 built whole by the AS route with its build script's p2bin instruction";
const REV: &str = "f6ece657c1cf253404312137dfcb8ec15fa42318";
const ASL_MD5: &str = "61e672562465725a8c102288a7da9098";
const P2BIN_MD5: &str = "4f2fff99c3347bafb93b12d5be1db754";
/// `build.lua` line 28, with its `compression` setting's default.
const INSTRUCTION: [&str; 2] = ["-p=FF", "-z=0,kosinski,Size_of_DAC_driver_guess,after"];
const REF_LEN: usize = 524_288;
const REF_CRC: u32 = 0xafe0_5eee;
/// Where the stored driver goes: the end of the record before the Z80 records
/// in asl's object file, up to the record after them.
const DRIVER_WINDOW: (u32, u32) = (0x72E7C, 0x745DC);

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

/// The data records of an AS object file: (cpu, load address, bytes), in order.
fn records(p: &[u8]) -> Vec<(u8, u32, Vec<u8>)> {
    assert_eq!(&p[..2], &[0x89, 0x14], "not an AS object file");
    let mut i = 2;
    let mut out = Vec::new();
    while i < p.len() {
        let header = p[i];
        i += 1;
        match header {
            0x00 => break,
            0x80 => {
                i += 4;
                continue;
            }
            _ => {}
        }
        let cpu = if header == 0x81 {
            let c = p[i];
            i += 3;
            c
        } else {
            header
        };
        let start = u32::from_le_bytes([p[i], p[i + 1], p[i + 2], p[i + 3]]);
        let len = usize::from(u16::from_le_bytes([p[i + 4], p[i + 5]]));
        i += 6;
        out.push((cpu, start, p[i..i + len].to_vec()));
        i += len;
    }
    out
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
fn sonic_1_builds_to_the_reference_rom_byte_for_byte() {
    let Some(root) = suite_root() else { return };
    let s1 = root.join("s1disasm");
    if !s1.join(".git").exists() {
        suite_root_absent(WHAT, &format!("the suite root {} holds no s1disasm checkout", root.display()));
        return;
    }

    let work = tempfile::Builder::new()
        .prefix("s1-driver-placement-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .expect("a directory under the target tmp dir");
    let tree = work.path();
    let archive = Command::new("git")
        .arg("-C")
        .arg(&s1)
        .args(["archive", "--format=tar", REV])
        .output()
        .expect("spawn git");
    assert!(
        archive.status.success(),
        "{} has no revision {REV}, which is the revision this row measures: {}",
        s1.display(),
        String::from_utf8_lossy(&archive.stderr)
    );
    let mut tar = Command::new("tar").arg("-x").arg("-C").arg(tree).stdin(Stdio::piped()).spawn().expect("spawn tar");
    tar.stdin.take().expect("tar stdin").write_all(&archive.stdout).expect("feed tar");
    assert!(tar.wait().expect("tar").success(), "tar could not unpack the archive");

    // build.lua's pre-step: the DAC samples are converted from the tracked WAV
    // files into gitignored `generated/` includes before anything assembles. A
    // checkout that has been built before already carries them, which is how a
    // tree "restored" with `git clean -fd` still builds; an archive does not.
    let pre = Command::new("lua")
        .current_dir(tree)
        .args([
            "-e",
            "local common = require \"build_tools.lua.common\"; \
             common.convert_pcm_files_in_directory(\"sound/dac/pcm\"); \
             common.convert_dpcm_files_in_directory(\"sound/dac/dpcm\")",
        ])
        .output()
        .expect("spawn lua, which s1disasm's build.lua needs for its sample conversion");
    assert!(pre.status.success(), "build.lua's sample conversion failed: {}", String::from_utf8_lossy(&pre.stderr));

    let tools = tree.join("build_tools/Linux-x86_64");
    assert_eq!(md5_of(&tools.join("asl")), ASL_MD5, "the revision's asl is not the reference build");
    assert_eq!(md5_of(&tools.join("p2bin")), P2BIN_MD5, "the revision's p2bin is not the pinned build");

    // The reference: build.lua's asl line, then its p2bin line.
    let asl = Command::new(tools.join("asl"))
        .current_dir(tree)
        .env("AS_MSGPATH", &tools)
        .args(["-xx", "-n", "-q", "-A", "-L", "-U", "-E", "-i", ".", "sonic.asm"])
        .output()
        .expect("spawn asl");
    assert!(asl.status.success(), "asl failed, so its object file is not a source of values");
    let listing = std::fs::read_to_string(tree.join("sonic.lst")).expect("asl wrote its listing");
    assert!(
        listing.lines().any(|l| l.trim() == "2 passes" || l.trim() == "1 pass")
            && !listing.lines().any(|l| l.trim_start().starts_with("Additional necessary passes")),
        "asl's listing does not end on a complete pass loop"
    );
    let p2bin = Command::new(tools.join("p2bin"))
        .current_dir(tree)
        .args(INSTRUCTION)
        .args(["sonic.p", "reference.bin"])
        .output()
        .expect("spawn p2bin");
    assert!(p2bin.status.success(), "p2bin failed: {}", String::from_utf8_lossy(&p2bin.stdout));
    let reference = std::fs::read(tree.join("reference.bin")).expect("the reference ROM");
    assert_eq!(
        (reference.len(), crc32(&reference)),
        (REF_LEN, REF_CRC),
        "the reference toolchain's ROM is not the one measured on 2026-09-11"
    );

    // Without the instruction the driver has no place, and the refusal is at its org.
    let plain = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .current_dir(tree)
        .args(["sonic.asm", "-o", "plain.bin"])
        .output()
        .expect("spawn sigil");
    let stderr = String::from_utf8_lossy(&plain.stderr);
    assert!(!plain.status.success(), "without -z the driver must be refused:\n{stderr}");
    assert!(
        stderr.contains("sound/z80.asm(9):") && stderr.contains("no -z instruction places it"),
        "the refusal is at the driver's `!org 0`:\n{stderr}"
    );

    // With it, the whole ROM.
    let built = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .current_dir(tree)
        .args(["sonic.asm", "-o", "sigil.bin"])
        .args(INSTRUCTION)
        .output()
        .expect("spawn sigil");
    assert!(built.status.success(), "sigil failed:\n{}", String::from_utf8_lossy(&built.stderr));
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

    // The stored driver, read back with sigil's own decompressor, is the driver
    // asl assembled: its Z80 records, taken from asl's object file, as is the window.
    let recs = records(&std::fs::read(tree.join("sonic.p")).expect("asl's object file"));
    let k = recs.iter().position(|(cpu, start, _)| *cpu != 1 && *start == 0).expect("a Z80 record at 0");
    let mut blob = Vec::new();
    let mut j = k;
    while j < recs.len() && recs[j].0 == recs[k].0 && recs[j].1 as usize == blob.len() {
        blob.extend_from_slice(&recs[j].2);
        j += 1;
    }
    let window = (recs[k - 1].1 + recs[k - 1].2.len() as u32, recs[j].1);
    assert_eq!(window, DRIVER_WINDOW, "asl's records put the driver's gap somewhere else");
    let stored = &image[window.0 as usize..window.1 as usize];
    let back = sigil_clownlzss_sys::decompress_kosinski(stored).expect("the stored driver decompresses");
    assert_eq!(blob.len(), 0x1BC6, "the driver asl assembled");
    assert!(back == blob, "the stored driver does not decompress to the driver asl assembled");
}
