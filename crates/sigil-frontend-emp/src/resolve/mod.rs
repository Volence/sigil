//! Cross-module resolution driver (Spec 2 §3): gather modules, resolve
//! `use`/prelude names, place items, and produce one linkable Vec<Section>.
pub mod imports;
pub mod manifest;
pub mod rename;

use crate::ast;
use crate::lower::{lower_module, LowerOptions};
use imports::{ExportIndex, ResolveEnv};
use manifest::Manifest;
use sigil_ir::Section;
use sigil_span::{Diagnostic, Level};
use std::collections::{HashSet, VecDeque};
use std::path::Path;

/// Compile the whole reachable module program rooted at `entry_id` into one flat
/// list of linkable [`Section`]s. BFS over `use` edges (plus the optional prelude
/// id) discovers the reachable modules; each is resolved (short names → canonical
/// symbols), lowered, checked for unresolved references, renamed to canonical
/// names, and its sections concatenated. Cross-module LABEL references become
/// fixups that the flat-symbol-table linker resolves after concatenation.
///
/// Returns the concatenated sections plus every diagnostic collected. A
/// `Level::Error` diagnostic means the caller must not link.
pub fn build_program(
    manifest: &Manifest,
    entry_id: &str,
    prelude_id: Option<&str>,
    opts: &LowerOptions,
) -> (Vec<Section>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let mut sections = Vec::new();

    // 1. Reachability BFS over `use` edges from the entry (and the prelude seed).
    let reachable = reachable_modules(manifest, entry_id, prelude_id, &mut diags);

    // 2. Export index over ALL modules in the manifest — not just the reachable
    //    set. `suggest_use` must be able to point at an exporting module the entry
    //    hasn't imported yet (that's the whole "add `use …`" fix-it), which is
    //    impossible if the un-imported module is absent from the index. The
    //    in-scope rename map is still driven by reachability + explicit `use`, so
    //    a wider index never resolves a name that isn't actually imported.
    let all_pairs: Vec<(&str, &ast::File)> = manifest
        .modules
        .iter()
        .map(|pm| (pm.id.as_str(), &pm.file))
        .collect();
    let index = ExportIndex::build(&all_pairs);

    // 3. Resolve the prelude tuple once (module id + parsed file).
    let prelude = prelude_id.and_then(|pid| {
        manifest
            .by_id
            .get(pid)
            .map(|&i| (manifest.modules[i].id.as_str(), &manifest.modules[i].file))
    });

    // 4. Per-module: resolve names, lower, report unresolved, rename, concat.
    for &i in &reachable {
        let pm = &manifest.modules[i];
        let (env, ediags) = ResolveEnv::build(&pm.id, &pm.file, &index, prelude);
        diags.extend(ediags);

        let (mut module, ldiags) = lower_module(&pm.file, opts);
        diags.extend(ldiags);

        report_unresolved(pm, &module, &env, &mut diags);

        rename::rename_module(&mut module, env.rename_map());
        sections.extend(module.sections);
    }

    (sections, diags)
}

/// BFS from `entry_id` (and, if `Some`, the `prelude_id` seed) over `use` edges.
/// A `use a.b.c` edge targets the module id `a.b.c`. Unknown ids get an error
/// diagnostic and are skipped. Returns reachable module indices in discovery
/// order (deduped).
fn reachable_modules(
    manifest: &Manifest,
    entry_id: &str,
    prelude_id: Option<&str>,
    diags: &mut Vec<Diagnostic>,
) -> Vec<usize> {
    let mut order = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    queue.push_back(entry_id.to_string());
    if let Some(pid) = prelude_id {
        queue.push_back(pid.to_string());
    }

    while let Some(id) = queue.pop_front() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let idx = match manifest.by_id.get(&id) {
            Some(&idx) => idx,
            None => {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!("no module `{id}` found under the scan root"),
                    primary: sigil_span::Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
                });
                continue;
            }
        };
        order.push(idx);
        for item in &manifest.modules[idx].file.items {
            if let ast::Item::Use(u) = item {
                let target = u.base.segments.join(".");
                if !seen.contains(&target) {
                    queue.push_back(target);
                }
            }
        }
    }
    order
}

/// For every fixup target symbol in `module`, emit an error diagnostic if it is
/// neither a proc-local hygiene symbol (starts with `$`) nor resolvable via the
/// env's rename map. When exactly one other module exports the name, the message
/// carries the "add `use …`" fix-it; otherwise it's a generic unknown-symbol
/// error. Repeated names are deduped so one missing name yields one error.
fn report_unresolved(
    pm: &manifest::ParsedModule,
    module: &sigil_ir::Module,
    env: &ResolveEnv,
    diags: &mut Vec<Diagnostic>,
) {
    let mut seen: HashSet<String> = HashSet::new();
    for sec in &module.sections {
        for frag in &sec.fragments {
            let mut targets = Vec::new();
            rename::collect_target_syms(frag, &mut targets);
            for s in targets {
                if s.starts_with('$') {
                    continue; // proc-local hygiene symbol — resolved intra-module.
                }
                if env.rename_map().contains_key(&s) {
                    continue; // resolvable to a canonical symbol.
                }
                if !seen.insert(s.clone()) {
                    continue; // already reported this name.
                }
                let message = match env.suggest_use(&s) {
                    Some(fixit) => format!("unresolved name `{s}` — {fixit}"),
                    None => format!("unknown symbol `{s}`"),
                };
                diags.push(Diagnostic {
                    level: Level::Error,
                    message,
                    primary: pm.file.module.span,
                });
            }
        }
    }
}

/// Find the module id whose source path matches `entry_path` (canonicalized), for
/// CLI entry resolution. Falls back to a raw path compare if canonicalization
/// fails on either side.
pub fn entry_id_for_path(manifest: &Manifest, entry_path: &Path) -> Option<String> {
    let want = std::fs::canonicalize(entry_path).ok();
    for pm in &manifest.modules {
        let have = std::fs::canonicalize(&pm.path).ok();
        let matches = match (&want, &have) {
            (Some(a), Some(b)) => a == b,
            _ => pm.path == entry_path,
        };
        if matches {
            return Some(pm.id.clone());
        }
    }
    None
}
