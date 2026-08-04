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
            // The id's LAST segment must name the file; the segments above it are
            // a free semantic namespace (see `file_stem_of`).
            if let Some(stem) = file_stem_of(root, path) {
                let last = id.rsplit('.').next().unwrap_or(&id);
                if last != stem {
                    diags.push(Diagnostic {
                        level: Level::Warning,
                        message: format!(
                            "[module.path-mismatch] module `{id}` ends in `{last}` but its file is `{}` — the last id segment and the file stem must agree (rename the file or the header)",
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

/// The source texts behind a [`Manifest`], indexed so a [`Span`] resolves to
/// `path:line:col`.
///
/// [`Manifest::scan`] parses each file under a sequential [`SourceId`], so a
/// [`SourceMap`](sigil_span::SourceMap) rebuilt in that same order has
/// `index == SourceId` and [`SourceMap::location`](sigil_span::SourceMap::location)
/// is correct. Any gap in the id space, or a file that no longer reads, indexes as
/// empty text with no path — [`locate`](SourceIndex::locate) answers `None` for it
/// rather than over-indexing.
///
/// This is the single location authority for every diagnostic tier: errors and
/// warnings both render through it, so the two look like one system.
pub struct SourceIndex {
    map: sigil_span::SourceMap,
    paths: Vec<Option<PathBuf>>,
}

impl SourceIndex {
    /// Read every source file `manifest` recorded and index it by its [`SourceId`].
    pub fn new(manifest: &Manifest) -> SourceIndex {
        let max_id = manifest.sources.keys().map(|id| id.0).max().unwrap_or(0);
        let mut map = sigil_span::SourceMap::new();
        let mut paths = Vec::new();
        for k in 0..=max_id {
            // A source whose text does not read has no usable line/col, so it keeps
            // no path either — `locate` degrades to `None` instead of reporting a
            // position computed against empty text.
            let entry = manifest
                .sources
                .get(&SourceId(k))
                .and_then(|p| std::fs::read_to_string(p).ok().map(|t| (p.clone(), t)));
            match entry {
                Some((path, text)) => {
                    map.add(text);
                    paths.push(Some(path));
                }
                None => {
                    map.add(String::new());
                    paths.push(None);
                }
            }
        }
        SourceIndex { map, paths }
    }

    /// `path:line:col` of `span`'s start, or `None` when `span`'s source is not a
    /// file this index knows (a synthetic module, or an unattributed diagnostic).
    pub fn locate(&self, span: Span) -> Option<String> {
        let path = self.paths.get(span.source.0 as usize)?.as_ref()?;
        let (line, col) = self.map.location(span);
        Some(format!("{}:{line}:{col}", path.display()))
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
            // Do NOT descend into a NESTED CHECKOUT. A `git worktree` (or a nested
            // clone) living under the scan root — e.g. `<root>/.worktrees/<branch>`
            // — is a byte-identical COPY of the same `.emp` tree, so walking into it
            // reports every module `declared twice`. The scan root itself is legal to
            // be a worktree (its `.git` FILE is never tested — only children are), so
            // this excludes ONLY nested checkouts, never the root. Two signatures:
            //   (1) a `.worktrees/` container (the canonical worktree home), and
            //   (2) any subdirectory carrying its own `.git` (file OR dir) — the
            //       general nested-repo/worktree root signature.
            if is_nested_checkout_dir(&p) {
                continue;
            }
            collect_emp(&p, root, out, diags);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

/// True if `dir` is a nested checkout the module scan must not descend into: the
/// `.worktrees/` container, or any directory that carries its own `.git` entry
/// (file or dir) — the general nested-clone / `git worktree` root signature. Only
/// ever tested on CHILD directories, so the scan root (which may itself be a
/// worktree) is never excluded.
fn is_nested_checkout_dir(dir: &Path) -> bool {
    if dir.file_name().is_some_and(|n| n == ".worktrees") {
        return true;
    }
    dir.join(".git").exists()
}

/// The file stem of `path`, which is the only part of a module's location the
/// id is required to agree with.
///
/// A module id is a SEMANTIC name, not a path transcription: an id is free to be
/// flatter than the tree it lives in (`engine.level.parallax_dsl` under
/// `engine/level/`, but equally `games.sonic4.constants` under
/// `games/sonic4/config/`). Requiring the whole dotted id to reproduce the
/// directory chain would make the layout and the namespace the same decision,
/// which this codebase deliberately separates — 93 of 122 modules disagree with
/// their directory chain on purpose, and pinning that would report the
/// convention itself as an error.
///
/// What DOES have to hold is the last segment: the file a reader opens must be
/// named after the module they looked up. That is the half a stale rename
/// actually breaks, and the half that is cheap to keep true.
fn file_stem_of(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    Some(rel.file_stem()?.to_str()?.to_string())
}
