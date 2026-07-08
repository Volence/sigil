//! Cross-module resolution driver (Spec 2 §3): gather modules, resolve
//! `use`/prelude names, place items, and produce one linkable Vec<Section>.
pub mod imports;
pub mod manifest;
pub mod rename;

use crate::ast;
use crate::lower::{lower_module, LowerOptions};
use imports::{ExportIndex, ResolveEnv};
use manifest::{Manifest, ParsedModule};
use sigil_ir::Section;
use sigil_span::{Diagnostic, Level, Span};
use std::collections::{HashSet, VecDeque};
use std::path::Path;

/// Name of a `pub`, comptime-only item — the only kind we inject. Such an item
/// never emits bytes when lowered (`lower_module`'s item loop skips these kinds),
/// so it is safe to PREPEND to another module's item list to make its name
/// visible to the evaluator without changing output. Returns `Some(name)` for a
/// pub `const`/`struct`/`enum`/`bitfield`/`newtype`/`comptime fn`; else `None`.
fn pub_comptime_name(item: &ast::Item) -> Option<&str> {
    match item {
        ast::Item::Const(d) if d.public => Some(&d.name),
        ast::Item::Struct(d) if d.public => Some(&d.name),
        ast::Item::Enum(d) if d.public => Some(&d.name),
        ast::Item::Bitfield(d) if d.public => Some(&d.name),
        ast::Item::Newtype(d) if d.public => Some(&d.name),
        ast::Item::ComptimeFn(d) if d.public => Some(&d.name),
        _ => None,
    }
}

/// Collect the pub comptime-only items (const/struct/enum/bitfield/newtype/comptime fn)
/// that `module` should see from the prelude and from the modules it `use`s. These are
/// PREPENDED to the module's items so the evaluator resolves cross-module types/consts,
/// without emitting any bytes (lower_module skips these item kinds).
fn ambient_items(
    module: &ParsedModule,
    prelude: Option<&ParsedModule>,
    manifest: &Manifest,
) -> Vec<ast::Item> {
    let mut out = Vec::new();

    // Prelude first (own items, added in Part B, shadow these via last-wins).
    if let Some(p) = prelude {
        if p.id != module.id {
            out.extend(
                p.file
                    .items
                    .iter()
                    .filter(|it| pub_comptime_name(it).is_some())
                    .cloned(),
            );
        }
    }

    // Then `use`-imported pub comptime-only items (these shadow prelude, matching
    // the prelude<use precedence; own items shadow both via Part B ordering).
    for item in &module.file.items {
        let ast::Item::Use(u) = item else { continue };
        let base = u.base.segments.join(".");
        let Some(&bi) = manifest.by_id.get(&base) else {
            continue;
        };
        let base_mod = &manifest.modules[bi];
        if base_mod.id == module.id {
            continue; // never inject a module's own items.
        }
        match &u.names {
            ast::UseNames::Whole => {} // whole-path label import — handled by rename/link.
            ast::UseNames::Glob => out.extend(
                base_mod
                    .file
                    .items
                    .iter()
                    .filter(|it| pub_comptime_name(it).is_some())
                    .cloned(),
            ),
            ast::UseNames::List(names) => out.extend(
                base_mod
                    .file
                    .items
                    .iter()
                    .filter(|it| {
                        pub_comptime_name(it).is_some_and(|n| names.iter().any(|w| w == n))
                    })
                    .cloned(),
            ),
        }
    }

    out
}

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

    // Prelude as a ParsedModule (for ambient comptime-def gathering).
    let prelude_pm = prelude_id
        .and_then(|pid| manifest.by_id.get(pid))
        .map(|&i| &manifest.modules[i]);

    // 4. Per-module: resolve names, lower, report unresolved, rename, concat.
    for &i in &reachable {
        let pm = &manifest.modules[i];
        // ResolveEnv/report_unresolved/rename all operate on the ORIGINAL file &
        // env — the rename map is this module's own defs + its label imports. The
        // prepended comptime items belong to OTHER modules and must not be renamed.
        let (env, ediags) = ResolveEnv::build(&pm.id, &pm.file, &index, prelude);
        diags.extend(ediags);

        // Prepend imported pub comptime-only defs (prelude + `use`d) so the
        // evaluator resolves cross-module types/consts. These emit no bytes and
        // no labels (lower_module skips these kinds), so output is byte-identical
        // to lowering `pm.file` directly. The common no-prelude/no-comptime-use
        // path has an empty ambient list and lowers BY REFERENCE (zero clones);
        // only the injected path builds a synthetic file.
        let ambient = ambient_items(pm, prelude_pm, manifest);
        let (mut module, ldiags) = if ambient.is_empty() {
            lower_module(&pm.file, opts) // zero-clone common path.
        } else {
            // The own-items clone here could later be avoided by having the
            // evaluator index a separate ambient slice (deferred — preludes are
            // small).
            let synthetic = ast::File {
                module: pm.file.module.clone(),
                attrs: pm.file.attrs.clone(),
                items: ambient
                    .into_iter()
                    .chain(pm.file.items.iter().cloned())
                    .collect(),
            };
            lower_module(&synthetic, opts)
        };
        diags.extend(ldiags);

        report_unresolved(pm, &module, &env, &mut diags);

        rename::rename_module(&mut module, env.rename_map());
        sections.extend(module.sections);
    }

    (sections, diags)
}

/// BFS from `entry_id` (and, if `Some`, the `prelude_id` seed) over `use` edges.
/// A `use a.b.c` edge targets the module id `a.b.c`. Unknown ids get an error
/// diagnostic (anchored at the `use` decl that named them) and are skipped.
/// Returns reachable module indices in discovery order.
///
/// Each queue entry carries the [`Span`] to blame if the id turns out unknown:
/// a `use` decl's own span for edges, and a zero span for the entry/prelude
/// seeds (which come from the CLI, not from source).
fn reachable_modules(
    manifest: &Manifest,
    entry_id: &str,
    prelude_id: Option<&str>,
    diags: &mut Vec<Diagnostic>,
) -> Vec<usize> {
    let seed_span = Span {
        source: sigil_span::SourceId(0),
        start: 0,
        end: 0,
    };
    let mut order = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, Span)> = VecDeque::new();

    // Insert into `seen` at ENQUEUE time so an id is never queued twice.
    let enqueue = |queue: &mut VecDeque<(String, Span)>,
                   seen: &mut HashSet<String>,
                   id: String,
                   span: Span| {
        if seen.insert(id.clone()) {
            queue.push_back((id, span));
        }
    };

    enqueue(&mut queue, &mut seen, entry_id.to_string(), seed_span);
    if let Some(pid) = prelude_id {
        enqueue(&mut queue, &mut seen, pid.to_string(), seed_span);
    }

    while let Some((id, blame)) = queue.pop_front() {
        let idx = match manifest.by_id.get(&id) {
            Some(&idx) => idx,
            None => {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!("no module `{id}` found under the scan root"),
                    primary: blame,
                });
                continue;
            }
        };
        order.push(idx);
        for item in &manifest.modules[idx].file.items {
            if let ast::Item::Use(u) = item {
                let target = u.base.segments.join(".");
                enqueue(&mut queue, &mut seen, target, u.span);
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
                    // TODO: thread fixup spans so this anchors at the use-site
                    // rather than the module header (best available today).
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
