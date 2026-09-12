//! The import rule for the harness paths that lower `.emp` files one at a time with
//! no resolve pass: seam 1's five resident sound modules and seam 2's banked tables.
//!
//! The map build applies the rule in its resolve pass (`ResolveEnv::build`). These
//! paths never run that pass, so before this module a `use` naming a module or an
//! item that does not exist lowered clean on them whenever nothing read the name.
//! [`standalone_import_errors`] applies the same rule, with the same wording, so a
//! `use` is refused at its own span on these paths too.

use sigil_frontend_emp::ast;
use sigil_frontend_emp::resolve::imports::{
    unknown_module_error, use_decl_errors, use_decls, ExportIndex,
};
use sigil_frontend_emp::resolve::manifest::{emp_files, Manifest};
use sigil_span::Diagnostic;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// A tree's `.emp` file set with a CRC32 of each file's contents (`None` for one that
/// cannot be read). Equal fingerprints mean no `.emp` file was added, removed or
/// rewritten, a same-length rewrite included.
type Fingerprint = Vec<(PathBuf, Option<u32>)>;

thread_local! {
    /// The last scan of each root this thread made, with the fingerprint it was taken at.
    static SCANS: RefCell<Vec<(PathBuf, Fingerprint, Rc<Manifest>)>> = const { RefCell::new(Vec::new()) };
}

fn fingerprint(root: &Path) -> Fingerprint {
    emp_files(root)
        .into_iter()
        .map(|p| {
            let crc = sigil_span::read_set::read(&p).ok().map(|bytes| sigil_span::read_set::crc32(&bytes));
            (p, crc)
        })
        .collect()
}

/// [`Manifest::scan`] of `root`, parsed once per tree state per thread. One build lowers
/// the seam-2 files that import outside themselves several times (six scans per
/// `sigil build`, about 45 ms each on aeon, measured), and every scan parses the whole
/// tree. The kept scan is reused only while the tree's fingerprint is unchanged, so an
/// edit, an added file or a removed one is always seen. The fingerprint reads each file
/// through the read ledger, so a reuse still records the bytes it compared, and a file
/// that changed between two checks is kept there as a conflict rather than lost.
fn scan_cached(root: &Path) -> Rc<Manifest> {
    let now = fingerprint(root);
    SCANS.with(|scans| {
        let mut scans = scans.borrow_mut();
        if let Some((_, seen, manifest)) = scans.iter().find(|(r, _, _)| r == root) {
            if *seen == now {
                return Rc::clone(manifest);
            }
        }
        let manifest = Rc::new(Manifest::scan(root).0);
        scans.retain(|(r, _, _)| r != root);
        scans.push((root.to_path_buf(), now, Rc::clone(&manifest)));
        manifest
    })
}

/// Every import-rule Error in `files`, each paired with the index of the file it is
/// in: a `use` whose module does not exist, a listed name that is not a `pub` item
/// of its module, and a glob whose module exports nothing.
///
/// A `use` resolves first among `files` themselves, each under its declared module
/// id and exactly as the caller parsed it (so an in-memory source override is what
/// gets checked), and otherwise among the modules scanned from `root`. The tree is
/// scanned only when some `use` names a module outside `files`, and parsed at most
/// once per tree state ([`scan_cached`]).
pub fn standalone_import_errors(root: &Path, files: &[&ast::File]) -> Vec<(usize, Diagnostic)> {
    let local: Vec<(String, &ast::File)> =
        files.iter().map(|f| (f.module.path.segments.join("."), *f)).collect();
    let is_local = |id: &str| local.iter().any(|(l, _)| l == id);
    let needs_scan = files
        .iter()
        .any(|f| use_decls(&f.items).iter().any(|u| !is_local(&u.base.segments.join("."))));
    let scanned = needs_scan.then(|| scan_cached(root));

    let mut pairs: Vec<(&str, &ast::File)> = local.iter().map(|(id, f)| (id.as_str(), *f)).collect();
    if let Some(m) = &scanned {
        pairs.extend(
            m.modules.iter().filter(|pm| !is_local(&pm.id)).map(|pm| (pm.id.as_str(), &pm.file)),
        );
    }
    let index = ExportIndex::build(&pairs);

    let mut out = Vec::new();
    for (i, f) in files.iter().enumerate() {
        for u in use_decls(&f.items) {
            let base = u.base.segments.join(".");
            if index.has_module(&base) {
                out.extend(use_decl_errors(u, &index).into_iter().map(|d| (i, d)));
            } else {
                out.push((i, unknown_module_error(&base, u.span)));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::standalone_import_errors;
    use sigil_frontend_emp::parse_str;

    /// A scratch tree holding `pkg.b` and `pkg.c` on disk, each with one `pub` const.
    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("pkg")).unwrap();
        std::fs::write(dir.path().join("pkg/b.emp"), "module pkg.b\npub const X = 1\n").unwrap();
        std::fs::write(dir.path().join("pkg/c.emp"), "module pkg.c\npub const Y = 2\n").unwrap();
        dir
    }

    fn messages(root: &std::path::Path, srcs: &[&str]) -> Vec<(usize, String)> {
        let files: Vec<_> = srcs.iter().map(|s| parse_str(s).0).collect();
        let refs: Vec<_> = files.iter().collect();
        standalone_import_errors(root, &refs).into_iter().map(|(i, d)| (i, d.message)).collect()
    }

    /// A `use` of a module outside the given files resolves in the scanned tree:
    /// a real `pub` name passes, a missing one is refused, a missing module is
    /// refused with the reachability walk's wording.
    #[test]
    fn a_use_outside_the_files_is_checked_against_the_tree() {
        let dir = tree();
        assert!(messages(dir.path(), &["module pkg.a\nuse pkg.b.{X}\n"]).is_empty());
        assert_eq!(
            messages(dir.path(), &["module pkg.a\nuse pkg.b.{NOPE}\n"]),
            vec![(0, "module `pkg.b` has no `pub` name `NOPE`".to_string())]
        );
        assert_eq!(
            messages(dir.path(), &["module pkg.a\nuse pkg.zz._\n"]),
            vec![(0, "no module `pkg.zz` found under the scan root".to_string())]
        );
    }

    /// A given file outranks the tree's copy of the same module, so an in-memory
    /// override is what gets checked: here the override drops `X`, and the import
    /// of `X` from the second file is refused although the disk copy has it. The
    /// second file also imports `pkg.c`, which only the tree has, so the tree IS
    /// scanned and its copy of `pkg.b` is in reach: without that import nothing is
    /// scanned and the precedence this test names is never exercised.
    #[test]
    fn a_given_file_outranks_the_trees_copy_of_its_module() {
        let dir = tree();
        let got = messages(
            dir.path(),
            &["module pkg.b\nconst X = 1\n", "module pkg.a\nuse pkg.c.{Y}\nuse pkg.b.{X}\n"],
        );
        assert_eq!(got, vec![(1, "module `pkg.b` has no `pub` name `X`".to_string())]);
    }

    /// The kept scan is not reused once the tree moves. The rewrite keeps the file's
    /// LENGTH (`X` becomes `Z`) and lands within milliseconds, inside one coarse mtime
    /// tick, so only a content fingerprint can see it: the import accepted before the
    /// edit is refused after it, in the same thread.
    #[test]
    fn a_tree_rewritten_in_place_is_scanned_again() {
        let dir = tree();
        let src = ["module pkg.a\nuse pkg.b.{X}\n"];
        assert!(messages(dir.path(), &src).is_empty(), "the unedited tree exports `X`");
        let b = dir.path().join("pkg/b.emp");
        let before = std::fs::metadata(&b).unwrap().len();
        std::fs::write(&b, "module pkg.b\npub const Z = 1\n").unwrap();
        assert_eq!(std::fs::metadata(&b).unwrap().len(), before, "the rewrite must keep the length");
        assert_eq!(
            messages(dir.path(), &src),
            vec![(0, "module `pkg.b` has no `pub` name `X`".to_string())]
        );
    }
}
