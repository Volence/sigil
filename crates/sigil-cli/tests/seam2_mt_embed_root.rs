//! seam-2 lowers `mt_bank.emp` whether its `embed` paths are written relative to the
//! module's own directory (`games/sonic4/data/sound`) or relative to the aeon root, the
//! way the ROM build resolves every module's paths. Where a path could mean two files,
//! or the module mixes the two spellings, seam-2 refuses rather than choosing.
//!
//! THE INPUTS ARE DERIVED, NOT TYPED. Each test copies the reference tree's `engine/`
//! and `games/sonic4/` into a scratch directory under `CARGO_TARGET_TMPDIR` and rewrites
//! that copy's `mt_bank.emp` from its own `embed("...")` literals, so the tests follow
//! aeon's module as it changes. A reference module with no `embed` literal fails the
//! derivation by name instead of passing on an input it did not build.
//!
//! THE EXPECTATIONS ARE DERIVED TOO. A root-relative rewrite names the same files, so it
//! must emit the unmodified tree's bank byte for byte, in both shapes. The refusals are
//! asserted by their diagnostic codes, and each one's input is checked to be the case it
//! claims (the planted file differs, or is the same file) before the verdict is read.
//!
//! Reference tree: `AEON_DIR`. Absent, each test skips green outside strict mode and
//! hard-fails naming the path under `SIGIL_STRICT_GATE=1`.

use sigil_harness::seam2::{emit_mt_bank, sound_layout};
use sigil_harness::test_support::reference_tree;
use std::path::{Path, PathBuf};

const SOUND_REL: &str = "games/sonic4/data/sound";
const MT_BANK_REL: &str = "games/sonic4/data/sound/mt_bank.emp";

fn reference() -> Option<PathBuf> {
    reference_tree(&[MT_BANK_REL, "games/sonic4/map.toml", "engine/sound/z80_sound_driver.emp"])
}

/// Copy the directory tree at `from` to `to`, real files and directories only.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        let ty = entry.file_type().unwrap();
        if ty.is_dir() {
            copy_tree(&entry.path(), &dest);
        } else if ty.is_file() {
            std::fs::copy(entry.path(), &dest).unwrap();
        } else {
            panic!("{} is neither a file nor a directory, the copy would not be faithful", entry.path().display());
        }
    }
}

/// A scratch copy of the reference tree's `engine/` and `games/sonic4/`, everything the
/// seam-2 emitters read.
fn scratch_copy(real: &Path) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch dir on disk");
    let root = tmp.path().to_path_buf();
    copy_tree(&real.join("engine"), &root.join("engine"));
    copy_tree(&real.join("games/sonic4"), &root.join("games/sonic4"));
    (tmp, root)
}

/// Every `embed("<path>")` literal in `src`, in source order.
fn embed_literals(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("embed(\"") {
        let after = &rest[i + "embed(\"".len()..];
        let end = after.find('"').expect("an embed literal closes its quote");
        out.push(after[..end].to_string());
        rest = &after[end..];
    }
    assert!(
        !out.is_empty(),
        "the reference {MT_BANK_REL} has no embed(\"...\") literal, so these tests cannot derive \
         their inputs from it"
    );
    out
}

/// The copy's `mt_bank.emp` with the first `count` embed literals rewritten from
/// module-relative to aeon-root-relative. Each rewritten path must name a file under
/// the copy's root and none under the sound directory, or the input is not the one the
/// test claims.
fn rewrite_to_root_relative(root: &Path, count: usize) -> Vec<String> {
    let path = root.join(MT_BANK_REL);
    let mut src = std::fs::read_to_string(&path).unwrap();
    let lits = embed_literals(&src);
    assert!(count <= lits.len());
    let mut rewritten = Vec::new();
    for lit in lits.iter().take(count) {
        let from = format!("embed(\"{lit}\")");
        let to = format!("embed(\"{SOUND_REL}/{lit}\")");
        assert!(src.contains(&from), "the literal {from} must be in the module");
        src = src.replacen(&from, &to, 1);
        let rel = format!("{SOUND_REL}/{lit}");
        assert!(root.join(&rel).is_file(), "{rel} must name a file under the aeon root");
        assert!(!root.join(SOUND_REL).join(&rel).exists(), "{rel} must not also name one under the sound dir");
        rewritten.push(rel);
    }
    std::fs::write(&path, src).unwrap();
    rewritten
}

/// The sound-dir-relative and the root-relative spelling of one module emit the same
/// bank, in both shapes, and derive the same sound layout.
#[test]
fn root_relative_embeds_emit_the_same_bank_as_module_relative() {
    let Some(real) = reference() else { return };
    let (_tmp, copy) = scratch_copy(&real);
    let n = embed_literals(&std::fs::read_to_string(copy.join(MT_BANK_REL)).unwrap()).len();
    let rewritten = rewrite_to_root_relative(&copy, n);
    assert_eq!(rewritten.len(), n);
    let left = embed_literals(&std::fs::read_to_string(copy.join(MT_BANK_REL)).unwrap());
    assert!(
        left.iter().all(|l| l.starts_with(SOUND_REL)),
        "every embed must now be root-relative: {left:?}"
    );

    for debug in [false, true] {
        let want = emit_mt_bank(&real, debug).unwrap_or_else(|e| panic!("reference emit_mt_bank({debug}): {e}"));
        let got = emit_mt_bank(&copy, debug).unwrap_or_else(|e| panic!("root-relative emit_mt_bank({debug}): {e}"));
        assert!(!want.bytes.is_empty(), "the reference bank emitted no bytes, the comparison would prove nothing");
        assert_eq!(got.bytes, want.bytes, "root-relative embeds changed the DEBUG={debug} bank bytes");
        assert_eq!(got.tables, want.tables, "root-relative embeds changed the DEBUG={debug} table offsets");
    }
    assert_eq!(
        sound_layout(&copy).expect("root-relative layout"),
        sound_layout(&real).expect("reference layout"),
        "the sound layout measures the MT bank's length, so it must not move either"
    );
}

/// One `embed` path that names a file under BOTH the aeon root and the sound dir is
/// refused, whether the second file differs from the first or only holds equal bytes.
/// The same file reached through a symlink is not ambiguous, and emits the real bank.
#[test]
fn a_path_naming_two_files_is_refused() {
    let Some(real) = reference() else { return };
    let src = std::fs::read_to_string(real.join(MT_BANK_REL)).unwrap();
    let first = embed_literals(&src).remove(0);
    let module_file = real.join(SOUND_REL).join(&first);
    let bytes = std::fs::read(&module_file).unwrap();
    assert!(!bytes.is_empty(), "{} must hold bytes to flip", module_file.display());

    // A different file of the same length at the root spelling of the same path.
    let (_tmp, copy) = scratch_copy(&real);
    let mut flipped = bytes.clone();
    flipped[0] ^= 0xFF;
    std::fs::write(copy.join(&first), &flipped).unwrap();
    assert_ne!(std::fs::read(copy.join(&first)).unwrap(), bytes, "the planted file must differ");
    let err = emit_mt_bank(&copy, false).map(|_| ()).expect_err("two different files must be refused");
    assert!(err.contains("[sound.mt-embed-ambiguous]"), "got: {err}");
    assert!(err.contains(&first), "the refusal must name the path: {err}");

    // Equal bytes in a second file: still two files, still refused.
    let (_tmp2, copy2) = scratch_copy(&real);
    std::fs::write(copy2.join(&first), &bytes).unwrap();
    let err = emit_mt_bank(&copy2, false).map(|_| ()).expect_err("an equal copy is still a second file");
    assert!(err.contains("[sound.mt-embed-ambiguous]"), "got: {err}");

    // The same file through a symlink: one file, accepted, the real bank.
    let (_tmp3, copy3) = scratch_copy(&real);
    std::os::unix::fs::symlink(copy3.join(SOUND_REL).join(&first), copy3.join(&first)).unwrap();
    assert_eq!(
        std::fs::canonicalize(copy3.join(&first)).unwrap(),
        std::fs::canonicalize(copy3.join(SOUND_REL).join(&first)).unwrap(),
        "the symlink must reach the module's own file"
    );
    let got = emit_mt_bank(&copy3, false).unwrap_or_else(|e| panic!("one file through two spellings: {e}"));
    assert_eq!(got.bytes, emit_mt_bank(&real, false).unwrap().bytes);
}

/// A module that spells some paths root-relative and others module-relative is refused.
#[test]
fn mixed_spellings_are_refused() {
    let Some(real) = reference() else { return };
    let n = embed_literals(&std::fs::read_to_string(real.join(MT_BANK_REL)).unwrap()).len();
    assert!(n >= 2, "a mixed module needs two embeds, the reference has {n}");
    let (_tmp, copy) = scratch_copy(&real);
    rewrite_to_root_relative(&copy, 1);
    let err = emit_mt_bank(&copy, false).map(|_| ()).expect_err("mixed spellings must be refused");
    assert!(err.contains("[sound.mt-embed-mixed]"), "got: {err}");
}

/// An `embed` whose path is not a string literal is refused before lowering: where it
/// points cannot be checked.
#[test]
fn a_computed_embed_path_is_refused() {
    let Some(real) = reference() else { return };
    let (_tmp, copy) = scratch_copy(&real);
    let path = copy.join(MT_BANK_REL);
    let src = std::fs::read_to_string(&path).unwrap();
    let first = embed_literals(&src).remove(0);
    let from = format!("embed(\"{first}\")");
    let mut out = String::new();
    let mut inserted = false;
    for line in src.lines() {
        if !inserted && line.contains(&from) {
            out.push_str(&format!("const _MT_EMBED_PATH = \"{first}\"\n"));
            out.push_str(&line.replacen(&from, "embed(_MT_EMBED_PATH)", 1));
            inserted = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    assert!(inserted, "the first embed literal must be rewritten");
    std::fs::write(&path, out).unwrap();
    let err = emit_mt_bank(&copy, false).map(|_| ()).expect_err("a computed path must be refused");
    assert!(err.contains("[sound.mt-embed-computed]"), "got: {err}");
}
