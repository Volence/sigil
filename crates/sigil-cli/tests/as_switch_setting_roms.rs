//! The corpora at settings they are not shipped at: flip one assembly option the
//! source itself offers and the AS route must still write the ROM the stock
//! toolchain writes, or refuse out loud.
//!
//! This is the gate for the two faults the 2026-09-16 AS-corpus census found in
//! its Q5, and it exists because NO test at the shipped settings can see either
//! of them. At the shipped settings Sonic 1's and Sonic 2's hardcoded header
//! checksums already equal the sums of the images they sit in, and each
//! corpus's `Size_of_..._guess` already equals the size its driver stores at, so
//! both faults are invisible and both corpora compare byte-identical anyway.
//! `as_driver_placement_corpus` and `as_sonic2_whole_rom` are those two
//! shipped-settings gates and they stayed green through the whole silent period.
//!
//! Every edit here is anchored to a LINE NUMBER and asserts the text it is
//! replacing. The census's first switch-flip script anchored on text, matched a
//! mention inside a warning string, never applied its edit, and then reported
//! zero bytes different: an unapplied mutation and a real pass are the same
//! artifact unless something separates them. The assertion is that something. It
//! also means a corpus that renames or moves the option reddens HERE, rather than
//! quietly testing the shipped settings twice.
//!
//! The corpora come from the suite root (the directory holding `aeon/` and
//! `empyrean/` beside this checkout, or `EMPYREAN_SUITE_ROOT`). Only the
//! `s1disasm` and `s2disasm` checkouts are read, through `git archive` of the
//! pinned revision, into a directory under this crate's target tmp dir; neither
//! checkout is built in or written.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sigil_harness::test_support::{suite_root_absent, unnamed_default_tree};

const WHAT: &str = "the corpora built by the AS route at flipped assembly-option settings";

const S1_REV: &str = "f6ece657c1cf253404312137dfcb8ec15fa42318";
const S1_ASL_MD5: &str = "61e672562465725a8c102288a7da9098";
const S1_INSTRUCTION: [&str; 2] = ["-p=FF", "-z=0,kosinski,Size_of_DAC_driver_guess,after"];

const S2_REV: &str = "e45ebf332f39987424ca3102e50c717628f71269";
/// s2disasm's own asl, not the pinned reference one: the pinned build cuts macro
/// arguments at 255 characters and cannot assemble Sonic 2 at all. Same pin, and
/// same reason, as `as_sonic2_whole_rom`.
const S2_ASL_MD5: &str = "0dee1f98e6480a4783d27ffd8b90896f";
const S2_INSTRUCTION: [&str; 2] = ["-p=0", "-z=0,saxman-bugged,Size_of_Snd_driver_guess,after"];

const P2BIN_MD5: &str = "4f2fff99c3347bafb93b12d5be1db754";

fn md5_of(path: &Path) -> String {
    let out = Command::new("md5sum").arg(path).output().expect("spawn md5sum");
    assert!(out.status.success(), "md5sum {}", path.display());
    String::from_utf8_lossy(&out.stdout).split_whitespace().next().unwrap_or("").to_string()
}

fn suite_root() -> Option<PathBuf> {
    match unnamed_default_tree() {
        Ok(tree) => Some(tree.path.parent().expect("a tree under the suite root has a parent").to_path_buf()),
        Err(why) => {
            suite_root_absent(WHAT, &why);
            None
        }
    }
}

/// A private extraction of `corpus` at `rev`, or `None` when the suite root or
/// that checkout is not here (the skip every corpus row in this crate takes).
fn extract(corpus: &str, rev: &str, prefix: &str) -> Option<(tempfile::TempDir, PathBuf)> {
    let root = suite_root()?;
    let checkout = root.join(corpus);
    if !checkout.join(".git").exists() {
        suite_root_absent(WHAT, &format!("the suite root {} holds no {corpus} checkout", root.display()));
        return None;
    }
    let work = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .expect("a directory under the target tmp dir");
    let archive = Command::new("git")
        .arg("-C")
        .arg(&checkout)
        .args(["archive", "--format=tar", rev])
        .output()
        .expect("spawn git");
    assert!(
        archive.status.success(),
        "{} has no revision {rev}, which is the revision this row measures: {}",
        checkout.display(),
        String::from_utf8_lossy(&archive.stderr)
    );
    let mut tar =
        Command::new("tar").arg("-x").arg("-C").arg(work.path()).stdin(Stdio::piped()).spawn().expect("spawn tar");
    tar.stdin.take().expect("tar stdin").write_all(&archive.stdout).expect("feed tar");
    assert!(tar.wait().expect("tar").success(), "tar could not unpack the archive");
    let path = work.path().to_path_buf();
    Some((work, path))
}

/// Replace 1-based line `line` of `file`, asserting it reads `was` first.
///
/// The assertion is the whole point: without it an edit that lands nowhere and a
/// build that genuinely agrees are the same artifact. The replaced text is
/// returned so a failure message can quote what was actually on disk.
fn flip(tree: &Path, file: &str, line: usize, was: &str, now: &str) {
    let path = tree.join(file);
    let text = std::fs::read(&path).expect("the corpus file");
    // The corpora are Latin-1: a byte is a character, and no line is re-encoded.
    let mut lines: Vec<Vec<u8>> = text.split(|&b| b == b'\n').map(<[u8]>::to_vec).collect();
    let found = String::from_utf8_lossy(&lines[line - 1]).to_string();
    assert_eq!(
        found, was,
        "{file}:{line} does not hold the option this row flips; the corpus moved it, and flipping the wrong line would test the shipped settings twice"
    );
    lines[line - 1] = now.as_bytes().to_vec();
    std::fs::write(&path, lines.join(&b'\n')).expect("write the flipped corpus file");
    // Read it back off disk rather than trusting the write: `now` may itself be
    // several lines, so the check is that exactly those lines are there.
    let reread = std::fs::read_to_string(&path).expect("re-read");
    let on_disk: Vec<&str> =
        reread.split('\n').skip(line - 1).take(now.split('\n').count()).collect();
    assert_eq!(on_disk.join("\n"), now, "the edit did not reach the disk at {file}:{line}");
}

/// `lua build.lua` in `tree`, which both produces the reference ROM and leaves
/// the generated inputs (compressed songs, converted samples) sigil reads.
fn build_lua(tree: &Path, script: &str) {
    let lua = Command::new("lua")
        .current_dir(tree)
        .arg(script)
        .output()
        .expect("spawn lua, which the disassemblies' build scripts need");
    assert!(
        lua.status.success(),
        "{script} failed:\n{}\n{}",
        String::from_utf8_lossy(&lua.stdout),
        String::from_utf8_lossy(&lua.stderr)
    );
}

fn run_sigil(tree: &Path, root_asm: &str, instruction: [&str; 2]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sigil"))
        .current_dir(tree)
        .args([root_asm, "-o", "sigil.bin"])
        .args(instruction)
        .output()
        .expect("spawn sigil")
}

/// Whole-image compare with the runs named, so a failure says where.
fn assert_same(image: &[u8], reference: &[u8], what: &str) {
    if image == reference {
        return;
    }
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for x in 0..image.len().min(reference.len()) {
        if image[x] != reference[x] {
            match runs.last_mut() {
                Some(r) if r.1 == x => r.1 = x + 1,
                _ => runs.push((x, x + 1)),
            }
        }
    }
    let shown: Vec<String> = runs
        .iter()
        .take(20)
        .map(|(a, b)| format!("[{a:#08X}, {b:#08X}) ref {:02X?} sigil {:02X?}", &reference[*a..*b], &image[*a..*b]))
        .collect();
    panic!(
        "{what}: sigil's ROM is {} bytes, the reference {}; {} bytes differ in {} runs: {}",
        image.len(),
        reference.len(),
        runs.iter().map(|(a, b)| b - a).sum::<usize>(),
        runs.len(),
        shown.join(" ")
    );
}

/// FAULT 1. `CheatsEnabled = 1` moves bytes after 0x200, so the sum the header
/// must carry moves with them, and `sonic.asm`'s hardcoded `dc.w $AFC7` becomes
/// a lie. Before this row's fix sigil wrote `AFC7` against the reference's
/// `BF37`, at exit 0, with nothing on stderr.
#[test]
fn sonic_1_with_cheats_enabled_still_builds_the_reference_rom() {
    let Some((_work, tree)) = extract("s1disasm", S1_REV, "s1-cheats-") else { return };
    let tools = tree.join("build_tools/Linux-x86_64");
    assert_eq!(md5_of(&tools.join("asl")), S1_ASL_MD5, "the revision's asl is not the build this row measured with");
    assert_eq!(md5_of(&tools.join("p2bin")), P2BIN_MD5, "the revision's p2bin is not the pinned build");

    flip(&tree, "sonic.asm", 24, "CheatsEnabled = 0", "CheatsEnabled = 1");
    build_lua(&tree, "build.lua");
    let reference = std::fs::read(tree.join("s1built.bin")).expect("build.lua's ROM");
    // Derived, not pinned: this is the arithmetic `common.lua`'s `fix_header`
    // runs, re-derived here so the row states WHAT must be true of the reference
    // rather than copying a number measured once.
    assert_eq!(
        u16::from_be_bytes([reference[0x18E], reference[0x18F]]),
        sigil_link::header_checksum(&reference),
        "the reference ROM's own header checksum is not the sum over [0x200, EOF)"
    );

    let built = run_sigil(&tree, "sonic.asm", S1_INSTRUCTION);
    let stderr = String::from_utf8_lossy(&built.stderr);
    assert!(built.status.success(), "sigil failed:\n{stderr}");
    let image = std::fs::read(tree.join("sigil.bin")).expect("sigil's ROM");
    assert_same(&image, &reference, "Sonic 1 with CheatsEnabled = 1");
}

/// FAULT 1, the second field. `fix_header` writes the end-of-ROM address at
/// 0x1A4 as well, and `sonic.asm` hardcodes that too, as `dc.l EndOfRom-1`. The
/// census names only 0x18E; this is the control that showed the other field can
/// go stale on its own, by emitting bytes AFTER the label the field is computed
/// from. Before this row's fix the two ROMs differed in six bytes, two fields.
#[test]
fn sonic_1_with_bytes_past_the_end_label_still_builds_the_reference_rom() {
    let Some((_work, tree)) = extract("s1disasm", S1_REV, "s1-romend-") else { return };
    let tools = tree.join("build_tools/Linux-x86_64");
    assert_eq!(md5_of(&tools.join("asl")), S1_ASL_MD5, "the revision's asl is not the build this row measured with");
    assert_eq!(md5_of(&tools.join("p2bin")), P2BIN_MD5, "the revision's p2bin is not the pinned build");

    // Line 5236 is the blank line under `EndOfRom:`; the label itself is 5235.
    flip(&tree, "sonic.asm", 5235, "EndOfRom:", "EndOfRom:\n\t\tdcb.b\t$100,$FF");
    build_lua(&tree, "build.lua");
    let reference = std::fs::read(tree.join("s1built.bin")).expect("build.lua's ROM");
    assert_eq!(
        u32::from_be_bytes(reference[0x1A4..0x1A8].try_into().unwrap()),
        (reference.len() - 1) as u32,
        "the reference ROM's end-of-ROM field is not the offset of its last byte"
    );
    // The control on the control: the field must actually have MOVED, or this
    // row is comparing two ROMs that never disagreed and proves nothing.
    assert_ne!(
        u32::from_be_bytes(reference[0x1A4..0x1A8].try_into().unwrap()),
        0x0007_FFFF,
        "the padding did not extend the ROM, so the end-of-ROM field never went stale and this row is vacuous"
    );

    let built = run_sigil(&tree, "sonic.asm", S1_INSTRUCTION);
    let stderr = String::from_utf8_lossy(&built.stderr);
    assert!(built.status.success(), "sigil failed:\n{stderr}");
    let image = std::fs::read(tree.join("sigil.bin")).expect("sigil's ROM");
    assert_same(&image, &reference, "Sonic 1 with bytes emitted after EndOfRom");
}

/// FAULT 2. `fixBugs = 1` grows the Z80 driver, which compresses to `0xF88`
/// against the `$F64` the source declares and bakes into the `move.w` its own
/// decompressor reads. `build.lua` patches that immediate from asl's share file;
/// sigil writes no share file, so nothing patches it. Before this row's fix sigil
/// wrote that ROM at exit 0 and the game would have decompressed `0x24` bytes too
/// few.
///
/// The gate is the refusal, not a ROM: sigil cannot know WHICH immediate encodes
/// the declared size, so the honest answer is to decline and name both numbers.
#[test]
fn sonic_2_with_fixbugs_is_refused_rather_than_written_with_a_stale_driver_size() {
    let Some((_work, tree)) = extract("s2disasm", S2_REV, "s2-fixbugs-") else { return };
    let tools = tree.join("build_tools/Linux-x86_64");
    assert_eq!(md5_of(&tools.join("asl")), S2_ASL_MD5, "the revision's asl is not the build this row measured with");
    assert_eq!(md5_of(&tools.join("p2bin")), P2BIN_MD5, "the revision's p2bin is not the pinned build");

    flip(&tree, "s2.asm", 27, "fixBugs = 0", "fixBugs = 1");
    // The reference is built for its generated inputs and for the control below:
    // the stock toolchain does NOT refuse this tree, so the refusal being tested
    // is about a ROM that really is wrong rather than one that cannot be built.
    build_lua(&tree, "build.lua");
    let reference = std::fs::read(tree.join("s2built.bin")).expect("build.lua's ROM");

    let built = run_sigil(&tree, "s2.asm", S2_INSTRUCTION);
    let stderr = String::from_utf8_lossy(&built.stderr);
    assert!(
        !built.status.success(),
        "sigil built a ROM whose driver-size immediate the build script patches and sigil cannot:\n{stderr}"
    );
    assert!(
        !tree.join("sigil.bin").exists(),
        "sigil refused and left an image behind; a refused build must write nothing"
    );
    let row = stderr
        .lines()
        .find(|l| l.contains(": error:"))
        .unwrap_or_else(|| panic!("sigil failed with no error row:\n{stderr}"));
    for needle in ["Size_of_Snd_driver_guess", "0xF88", "$F64"] {
        assert!(row.contains(needle), "the refusal does not name {needle}:\n{row}");
    }

    // The control on the numbers: they are read off the reference ROM rather
    // than copied from the message, so a message that agrees with itself and
    // with nothing else cannot pass. `build.lua` writes the true compressed size
    // into the immediate at `movewZ80CompSize+2`, and that is what sigil's
    // refusal says the source's `$F64` should have been.
    let patched = reference
        .windows(4)
        .position(|w| w == [0x3E, 0x3C, 0x0F, 0x88])
        .expect("the reference ROM's patched `move.w #$F88,d7`");
    assert!(patched > 0x200, "the patched immediate is inside the header, which cannot be right");
}
