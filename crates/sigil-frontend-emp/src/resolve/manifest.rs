//! Module manifest (Spec 2 §3.1): scan a root directory for `.emp` files,
//! parse each, and index them by their `module <dotted.path>` header.
//!
//! Later resolution stages (`use` imports, item placement, linking) read from
//! the [`Manifest`] produced here. This stage does not resolve imports.

use crate::ast;
use sigil_span::{Diagnostic, Level, SourceId, Span};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One parsed `.emp` module: its dotted id (from the header), the parsed AST,
/// and the on-disk path it was read from.
pub struct ParsedModule {
    /// Dotted module id, e.g. `badniks.pitcher_plant` (from the `module` header).
    pub id: String,
    /// The parsed source file.
    pub file: ast::File,
    /// Path the module was read from.
    pub path: PathBuf,
}

/// An index of every `.emp` module found under a root directory, keyed by
/// dotted id. The `by_id` map's values are indices into `modules`.
pub struct Manifest {
    /// Every parsed module, in deterministic (sorted-path) order.
    pub modules: Vec<ParsedModule>,
    /// Map from dotted module id to its index in `modules`.
    pub by_id: HashMap<String, usize>,
    /// Registry mapping each file's [`SourceId`] back to its on-disk path, so a
    /// diagnostic renderer can turn a module's span into `path:line:col`.
    pub sources: HashMap<SourceId, PathBuf>,
}

/// A zero-width span with no meaningful source, for diagnostics that are not
/// attributable to a location within a source file (e.g. a directory read error).
fn no_span() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

impl Manifest {
    /// Recursively scan `root` for `*.emp` files, parse each, and index them by
    /// their `module` header. Returns the manifest and all collected diagnostics
    /// (parse diagnostics, the §3.1 path/dir lint as warnings, duplicate-id and
    /// read errors).
    ///
    /// Each file is parsed under a distinct [`SourceId`] (allocated sequentially
    /// over the sorted file list) so downstream diagnostics keyed to a module's
    /// header span resolve to the right file via [`Manifest::sources`].
    pub fn scan(root: &Path) -> (Manifest, Vec<Diagnostic>) {
        let mut modules = Vec::new();
        let mut by_id = HashMap::new();
        let mut sources = HashMap::new();
        let mut diags = Vec::new();
        let mut files = Vec::new();
        collect_emp(root, root, &mut files, &mut diags);
        files.sort();
        for (i, path) in files.iter().enumerate() {
            // Allocate the SourceId and register its path BEFORE the fallible read,
            // so EVERY allocated id has a `sources` entry. Otherwise a file that
            // fails to read (TOCTOU: removed/chmod'd between the dir-walk and the
            // read) would be skipped in `sources` while the id counter advances,
            // leaving `sources` keys non-dense and breaking the id↔sorted-index
            // density invariant a positional diagnostic renderer relies on.
            let source = SourceId(i as u32);
            sources.insert(source, path.clone());
            let src = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    diags.push(Diagnostic {
                        level: Level::Error,
                        message: format!("cannot read `{}`: {e}", path.display()),
                        primary: no_span(),
                    });
                    continue;
                }
            };
            let (file, mut pdiags) = crate::parse_file(&src, source);
            diags.append(&mut pdiags);
            let id = file.module.path.segments.join(".");
            if let Some(expected) = expected_id_from_path(root, path) {
                if expected != id {
                    diags.push(Diagnostic {
                        level: Level::Warning,
                        message: format!(
                            "module `{id}` is at `{}`, which suggests id `{expected}` (rename the file/dir or the header to agree)",
                            path.strip_prefix(root).unwrap_or(path).display()
                        ),
                        primary: file.module.span,
                    });
                }
            }
            if let Some(prev) = by_id.insert(id.clone(), modules.len()) {
                // Deliberate last-wins policy: after emitting the dup error we
                // leave `by_id` pointing at the LAST occurrence of the id.
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!("module `{id}` declared twice (also at module #{prev})"),
                    primary: file.module.span,
                });
            }
            modules.push(ParsedModule { id, file, path: path.clone() });
        }
        (Manifest { modules, by_id, sources }, diags)
    }
}

/// Recursively collect every `*.emp` file under `dir` into `out`. `root` is the
/// original scan root: a failed `read_dir` on `root` itself is reported as an
/// error (a mistyped/nonexistent root must not silently look like an empty
/// tree); a failed `read_dir` on a subdirectory is swallowed.
fn collect_emp(dir: &Path, root: &Path, out: &mut Vec<PathBuf>, diags: &mut Vec<Diagnostic>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            if dir == root {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!("cannot read module root `{}`: {e}", dir.display()),
                    primary: no_span(),
                });
            }
            return;
        }
    };
    for e in entries.flatten() {
        let p = e.path();
        // Use the DirEntry's own file type, which does NOT follow symlinks, so a
        // directory symlink pointing at an ancestor cannot cause infinite
        // recursion (such symlinked dirs are simply skipped).
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            collect_emp(&p, root, out, diags);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

/// Compute the module id implied by a file's location relative to `root`:
/// the directory segments below `root` plus the file stem, joined with `.`.
fn expected_id_from_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let stem = rel.file_stem()?.to_str()?;
    let mut segs: Vec<String> = rel
        .parent()
        .into_iter()
        .flat_map(|p| p.components())
        .filter_map(|c| c.as_os_str().to_str().map(String::from))
        .collect();
    segs.push(stem.to_string());
    Some(segs.join("."))
}
