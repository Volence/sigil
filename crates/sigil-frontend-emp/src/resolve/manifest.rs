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
}

/// A zero-width span with no meaningful source, for diagnostics that are not
/// attributable to a location within a source file (e.g. a file read error).
fn no_span() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

impl Manifest {
    /// Recursively scan `root` for `*.emp` files, parse each, and index them by
    /// their `module` header. Returns the manifest and all collected diagnostics
    /// (parse diagnostics, the §3.1 path/dir lint as warnings, duplicate-id and
    /// read errors).
    pub fn scan(root: &Path) -> (Manifest, Vec<Diagnostic>) {
        let mut modules = Vec::new();
        let mut by_id = HashMap::new();
        let mut diags = Vec::new();
        let mut files = Vec::new();
        collect_emp(root, &mut files);
        files.sort();
        for path in files {
            let src = match std::fs::read_to_string(&path) {
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
            let (file, mut pdiags) = crate::parse_str(&src);
            diags.append(&mut pdiags);
            let id = file.module.path.segments.join(".");
            if let Some(expected) = expected_id_from_path(root, &path) {
                if expected != id {
                    diags.push(Diagnostic {
                        level: Level::Warning,
                        message: format!(
                            "module `{id}` is at `{}`, which suggests id `{expected}` (rename the file/dir or the header to agree)",
                            path.strip_prefix(root).unwrap_or(&path).display()
                        ),
                        primary: file.module.span,
                    });
                }
            }
            if let Some(prev) = by_id.insert(id.clone(), modules.len()) {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!("module `{id}` declared twice (also at module #{prev})"),
                    primary: file.module.span,
                });
            }
            modules.push(ParsedModule { id, file, path });
        }
        (Manifest { modules, by_id }, diags)
    }
}

/// Recursively collect every `*.emp` file under `dir` into `out`.
fn collect_emp(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_emp(&p, out);
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
