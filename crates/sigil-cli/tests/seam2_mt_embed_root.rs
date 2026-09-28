//! seam-2 lowers `mt_bank.emp` whether its `embed` paths are written relative to the
//! module's own directory (`games/sonic4/data/sound`) or relative to the aeon root, the
//! way the ROM build resolves every module's paths. Where a path could mean two files,
//! or the module mixes the two spellings, seam-2 refuses rather than choosing.
//!
//! THE INPUTS ARE DERIVED, NOT TYPED. Each test copies the reference tree's `engine/`
//! and `games/sonic4/` into a scratch directory under `CARGO_TARGET_TMPDIR` and respells
//! that copy's `mt_bank.emp` from its own `embed("...")` literals. The reference module
//! may use either spelling: each literal is read as whichever one it is and rewritten to
//! the form a test needs, so the tests hold on both sides of aeon's respelling. A
//! reference module with no `embed` literal fails the derivation by name instead of
//! passing on an input it did not build.
//!
//! THE EXPECTATIONS ARE DERIVED TOO. Both spellings name the same files, so each must
//! emit the unmodified tree's bank byte for byte, in both shapes. The refusals are
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

/// A literal's module-relative spelling (the path under the sound dir), whichever
/// spelling it is written in.
fn module_relative(lit: &str) -> String {
    lit.strip_prefix(&format!("{SOUND_REL}/")).unwrap_or(lit).to_string()
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Spelling {
    Module,
    Root,
}

/// Respell the copy's `mt_bank.emp`: the first `count` embed literals take spelling
/// `first`, the rest take `rest`. Every literal must then name its file under exactly the
/// directory its spelling says and not under the other, or the input is not the one the
/// test claims. Returns the module-relative names in source order.
fn respell(root: &Path, count: usize, first: Spelling, rest: Spelling) -> Vec<String> {
    let path = root.join(MT_BANK_REL);
    let src = std::fs::read_to_string(&path).unwrap();
    let lits = embed_literals(&src);
    assert!(count <= lits.len());
    let mut out = String::with_capacity(src.len());
    let mut tail = src.as_str();
    let mut names = Vec::new();
    for (i, lit) in lits.iter().enumerate() {
        let from = format!("embed(\"{lit}\")");
        let at = tail.find(&from).expect("the literal is in the module, in order");
        let name = module_relative(lit);
        let spelling = if i < count { first } else { rest };
        let written = match spelling {
            Spelling::Module => name.clone(),
            Spelling::Root => format!("{SOUND_REL}/{name}"),
        };
        let (here, other) = match spelling {
            Spelling::Module => (root.join(SOUND_REL).join(&written), root.join(&written)),
            Spelling::Root => (root.join(&written), root.join(SOUND_REL).join(&written)),
        };
        assert!(here.is_file(), "{written} ({spelling:?}) must name a file at {}", here.display());
        assert!(!other.exists(), "{written} ({spelling:?}) must not also name {}", other.display());
        out.push_str(&tail[..at]);
        out.push_str(&format!("embed(\"{written}\")"));
        tail = &tail[at + from.len()..];
        names.push(name);
    }
    out.push_str(tail);
    std::fs::write(&path, out).unwrap();
    let now = embed_literals(&std::fs::read_to_string(&path).unwrap());
    assert_eq!(now.len(), lits.len(), "the respelling must keep every literal");
    names
}

/// The module-relative and the root-relative spelling of one module emit the same
/// bank as the unmodified reference, in both shapes, and derive the same sound layout.
#[test]
fn both_spellings_emit_the_reference_bank() {
    let Some(real) = reference() else { return };
    let (_tmp_m, module_copy) = scratch_copy(&real);
    let (_tmp_r, root_copy) = scratch_copy(&real);
    let n = respell(&module_copy, 0, Spelling::Module, Spelling::Module).len();
    assert_eq!(respell(&root_copy, n, Spelling::Root, Spelling::Root).len(), n);

    for debug in [false, true] {
        let want = emit_mt_bank(&real, debug).unwrap_or_else(|e| panic!("reference emit_mt_bank({debug}): {e}"));
        assert!(!want.bytes.is_empty(), "the reference bank emitted no bytes, the comparison would prove nothing");
        for (what, copy) in [("module-relative", &module_copy), ("root-relative", &root_copy)] {
            let got = emit_mt_bank(copy, debug).unwrap_or_else(|e| panic!("{what} emit_mt_bank({debug}): {e}"));
            assert_eq!(got.bytes, want.bytes, "{what} embeds changed the DEBUG={debug} bank bytes");
            assert_eq!(got.tables, want.tables, "{what} embeds changed the DEBUG={debug} table offsets");
        }
    }
    let want = sound_layout(&real).expect("reference layout");
    assert_eq!(sound_layout(&module_copy).expect("module-relative layout"), want);
    assert_eq!(
        sound_layout(&root_copy).expect("root-relative layout"),
        want,
        "the sound layout measures the MT bank's length, so it must not move either"
    );
}

/// One `embed` path that names a file under BOTH the aeon root and the sound dir is
/// refused, whether the second file differs from the first or only holds equal bytes.
/// The same file reached through a symlink is not ambiguous, and emits the real bank.
#[test]
fn a_path_naming_two_files_is_refused() {
    let Some(real) = reference() else { return };
    let want = emit_mt_bank(&real, false).expect("reference emit_mt_bank").bytes;

    // Each case starts from a module-relative copy and plants a file at the root
    // spelling of the first literal's path.
    let module_copy = || {
        let (tmp, copy) = scratch_copy(&real);
        let first = respell(&copy, 0, Spelling::Module, Spelling::Module).remove(0);
        (tmp, copy, first)
    };

    // A different file of the same length.
    let (_tmp, copy, first) = module_copy();
    let bytes = std::fs::read(copy.join(SOUND_REL).join(&first)).unwrap();
    assert!(!bytes.is_empty(), "{first} must hold bytes to flip");
    let mut flipped = bytes.clone();
    flipped[0] ^= 0xFF;
    std::fs::write(copy.join(&first), &flipped).unwrap();
    assert_ne!(std::fs::read(copy.join(&first)).unwrap(), bytes, "the planted file must differ");
    let err = emit_mt_bank(&copy, false).map(|_| ()).expect_err("two different files must be refused");
    assert!(err.contains("[sound.mt-embed-ambiguous]"), "got: {err}");
    assert!(err.contains(&first), "the refusal must name the path: {err}");

    // Equal bytes in a second file: still two files, still refused.
    let (_tmp2, copy2, first) = module_copy();
    std::fs::copy(copy2.join(SOUND_REL).join(&first), copy2.join(&first)).unwrap();
    let err = emit_mt_bank(&copy2, false).map(|_| ()).expect_err("an equal copy is still a second file");
    assert!(err.contains("[sound.mt-embed-ambiguous]"), "got: {err}");

    // The same file through a symlink: one file, accepted, the real bank.
    let (_tmp3, copy3, first) = module_copy();
    std::os::unix::fs::symlink(copy3.join(SOUND_REL).join(&first), copy3.join(&first)).unwrap();
    assert_eq!(
        std::fs::canonicalize(copy3.join(&first)).unwrap(),
        std::fs::canonicalize(copy3.join(SOUND_REL).join(&first)).unwrap(),
        "the symlink must reach the module's own file"
    );
    let got = emit_mt_bank(&copy3, false).unwrap_or_else(|e| panic!("one file through two spellings: {e}"));
    assert_eq!(got.bytes, want);
}

/// A module that spells some paths root-relative and others module-relative is refused,
/// in either direction.
#[test]
fn mixed_spellings_are_refused() {
    let Some(real) = reference() else { return };
    for (first, rest) in [(Spelling::Root, Spelling::Module), (Spelling::Module, Spelling::Root)] {
        let (_tmp, copy) = scratch_copy(&real);
        let names = respell(&copy, 1, first, rest);
        assert!(names.len() >= 2, "a mixed module needs two embeds, the reference has {}", names.len());
        let err = emit_mt_bank(&copy, false).map(|_| ()).expect_err("mixed spellings must be refused");
        assert!(err.contains("[sound.mt-embed-mixed]"), "{first:?} then {rest:?}: {err}");
    }
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
