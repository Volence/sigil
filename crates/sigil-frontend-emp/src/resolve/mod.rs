//! Cross-module resolution driver (Spec 2 §3): gather modules, resolve
//! `use`/prelude names, place items, and produce one linkable Vec<Section>.
pub mod imports;
pub mod manifest;
pub mod rename;

use crate::ast;
use crate::lower::{lower_module, LowerOptions};
use imports::{ExportIndex, ResolveEnv};
use manifest::{Manifest, ParsedModule};
use sigil_ir::map::MemoryMap;
use sigil_ir::Section;
use sigil_span::{Diagnostic, Level, Span};
use std::collections::{HashMap, HashSet, VecDeque};
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
        // `pub vars` OVERLAY form (`vars Name: window { .. }`, D6.A8): overlays
        // are ordinary module items shared by `use`, so a consumer that imports
        // the overlay (and its base struct) gets qualified/bare field access. The
        // overlay emits ZERO bytes when lowered — only always-on decl checks fire
        // — so it is safe to inject like a struct. The REGION form (`name: None`)
        // is not a comptime item and is never injected.
        ast::Item::Vars(d) if d.public && d.name.is_some() => d.name.as_deref(),
        _ => None,
    }
}

/// Name of a `pub data` item whose type annotation is a single bare `Named`
/// type (a struct / newtype / refined name) — the only shape a TYPE-ONLY stub
/// can be injected for (D-PP.5). Returns `None` for a non-public data item, one
/// with no type annotation, or one whose type is an array / pointer / tuple
/// (those are not struct-typed field-access receivers). Whether the named type
/// is REALLY a struct is decided later, in the consumer's evaluator (a bad name
/// errors loudly there via `layout_of_struct`) — the resolver only filters on
/// the annotation SHAPE, which needs no type index here.
fn pub_struct_data_name(item: &ast::Item) -> Option<&str> {
    match item {
        ast::Item::Data(d) if d.public => match &d.ty {
            Some(ast::Type::Named(p)) if p.segments.len() == 1 => Some(&d.name),
            _ => None,
        },
        _ => None,
    }
}

/// Collect the pub comptime-only items directly in `items` AND one level inside
/// any `section {}` body (sections do not nest further — Task 1 rejects that at
/// parse time — so a single level of recursion is exhaustive), matching `pred`.
/// Mirrors `imports::collect_exported`/`collect_defined`'s recursion shape.
///
/// `def_file` is the DEFINING module's file — the namespace an injected overlay's
/// window must resolve against (Plan 7 #8). Each collected `pub vars` overlay clone
/// has its window resolved here and STAMPED (`resolved_window`), so the consumer
/// binds it at the definition site verbatim rather than re-scanning its own structs.
fn collect_pub_comptime(
    def_file: &ast::File,
    items: &[ast::Item],
    pred: &impl Fn(&str) -> bool,
    out: &mut Vec<ast::Item>,
) {
    for item in items {
        // A `pub data` item of struct type (D-PP.5): inject a TYPE-ONLY clone so
        // the consumer's evaluator learns its struct type for `Item.field`
        // field-address operands, WITHOUT re-emitting its bytes. This is the
        // data-item analogue of the comptime-item injection below — a data item
        // emits, so it cannot ride the `pub_comptime_name` path; the `type_only`
        // flag strips its bytes at lowering while keeping its name+type visible.
        if let Some(name) = pub_struct_data_name(item) {
            if pred(name) {
                if let ast::Item::Data(d) = item {
                    let mut stub = d.clone();
                    stub.type_only = true;
                    // The stub carries only its name + `ty`; blank the initializer
                    // to a Unit so a stray eval can never read the (absent) value.
                    stub.value = ast::Expr::TupleLit { elems: vec![], span: d.span };
                    stub.max_size = None;
                    out.push(ast::Item::Data(stub));
                }
            }
        }
        if pub_comptime_name(item).is_some_and(pred) {
            let mut cloned = item.clone();
            // TODO(perf): this stamp re-resolves the overlay's window PER CONSUMER
            // module — each call spins a fresh eval-stack thread and re-indexes the
            // whole defining file, so M imported overlays across N consumers cost
            // M×N resolutions. Intended fix: resolve each defining module's
            // overlays ONCE (a per-module cache keyed by overlay name, built in
            // `build_program`) and reuse across consumers (deferred — imported
            // overlay counts are small today, like the own-items clone note in
            // `build_program`).
            stamp_overlay_window(def_file, &mut cloned);
            out.push(cloned);
        }
        if let ast::Item::Section(sec) = item {
            collect_pub_comptime(def_file, &sec.items, pred, out);
        }
    }
}

/// Stamp an injected `pub vars` overlay's window binding, resolved against its
/// DEFINING file (Plan 7 #8). No-op for any non-overlay item (or a region-form
/// `vars`, or an overlay whose window fails to resolve — a poisoned overlay stays
/// silent in the consumer as before). This is what makes a bare-window overlay
/// bind where it was defined instead of re-resolving in the consumer's namespace.
fn stamp_overlay_window(def_file: &ast::File, item: &mut ast::Item) {
    if let ast::Item::Vars(v) = item {
        if let Some(name) = v.name.clone() {
            v.resolved_window = crate::layout::resolve_overlay_window(def_file, &name);
        }
    }
}

/// Collect the pub comptime-only items (const/struct/enum/bitfield/newtype/comptime fn)
/// that `module` should see from the prelude and from the modules it `use`s. These are
/// PREPENDED to the module's items so the evaluator resolves cross-module types/consts,
/// without emitting any bytes (lower_module skips these item kinds). Recurses one level
/// into `section {}` bodies (see `collect_pub_comptime`) so a section-nested `pub const`/
/// `pub struct`/etc. is injected too, not just exported.
fn ambient_items(
    module: &ParsedModule,
    prelude: Option<&ParsedModule>,
    manifest: &Manifest,
) -> Vec<ast::Item> {
    let mut out = Vec::new();

    // Prelude first (own items, added in Part B, shadow these via last-wins).
    if let Some(p) = prelude {
        if p.id != module.id {
            collect_pub_comptime(&p.file, &p.file.items, &|_| true, &mut out);
        }
    }

    // Then `use`-imported pub comptime-only items (these shadow prelude, matching
    // the prelude<use precedence; own items shadow both via Part B ordering).
    // Recurses one level into `section {}` bodies so a section-nested `use` is
    // honored too, not just top-level ones.
    ambient_from_uses(&module.file.items, module, manifest, &mut out);

    out
}

fn ambient_from_uses(
    items: &[ast::Item],
    module: &ParsedModule,
    manifest: &Manifest,
    out: &mut Vec<ast::Item>,
) {
    for item in items {
        match item {
            ast::Item::Use(u) => {
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
                    ast::UseNames::Glob => {
                        collect_pub_comptime(&base_mod.file, &base_mod.file.items, &|_| true, out)
                    }
                    ast::UseNames::List(names) => collect_pub_comptime(
                        &base_mod.file,
                        &base_mod.file.items,
                        &|n| names.iter().any(|w| w == n),
                        out,
                    ),
                }
            }
            ast::Item::Section(sec) => ambient_from_uses(&sec.items, module, manifest, out),
            _ => {}
        }
    }
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

    // Struct-declaration diagnostics (size/@offset mismatch, odd-field warning —
    // whatever `layout_of_struct`'s always-on checks produce) already dedup
    // WITHIN one `lower_module` call (`dedup_overlay_pass_diags`, keyed off the
    // module's own overlay-forced pass), but that memo is per-call: a `pub vars`
    // overlay forces its base struct's layout in the DEFINING module, and a
    // separate CONSUMER module forces the SAME struct's layout again (field
    // access / sizeof) via its own, independent `lower_module` call — a second
    // `Evaluator` the defining module's dedup never sees. Both copies carry the
    // struct's home-file span (declaration checks always anchor there, never at
    // the forcing site), so they are EXACT duplicates once concatenated here.
    // `seen_across_modules` tracks every `(level, message, primary span)` triple
    // already contributed by an EARLIER module in this loop; a later module's
    // `ldiags` that repeats one is dropped before it ever reaches `diags`. This
    // must not touch duplicates that arise WITHIN a single module (two procs in
    // the same file each forcing the same unrelated-to-any-overlay struct) —
    // that pre-existing intra-module duplication is pinned by overlay.rs tests
    // and stays exactly as-is, because `ldiags` is filtered only against PRIOR
    // modules' contributions, never against itself. A `Vec` + linear `contains`
    // (not a `HashSet`) — `Diagnostic` derives `Eq` but not `Hash`, and these
    // lists are per-compile diagnostic counts (tiny); mirrors
    // `dedup_overlay_pass_diags`'s own O(n·m) shape in `lower/mod.rs`.
    let mut seen_across_modules: Vec<Diagnostic> = Vec::new();

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
        // Drop only what an EARLIER module already contributed (`seen_across_modules`
        // is empty on this module's first appearance in the loop, so a module's
        // OWN first-time diagnostics — including intra-module duplicates among
        // themselves — always survive this filter untouched); then record this
        // module's (post-filter) diagnostics so a LATER module's repeat of them
        // collapses too. Keeps the first occurrence's position (this module's, or
        // whichever earlier module first produced it) per the diagnostics-order
        // contract.
        let ldiags: Vec<Diagnostic> =
            ldiags.into_iter().filter(|d| !seen_across_modules.contains(d)).collect();
        seen_across_modules.extend(ldiags.iter().cloned());
        diags.extend(ldiags);

        report_unresolved(pm, &module, &env, &mut diags);

        rename::rename_module(&mut module, env.rename_map());
        sections.extend(module.sections);
    }

    (sections, diags)
}

/// Assign each section a physical LMA from the memory map, keyed by SECTION NAME
/// → REGION NAME (§7). The concatenated sections handed back by [`build_program`]
/// carry module-local LMAs (each module's own physical counter starts near 0), so
/// they must be re-based into their declared map region before link/emit. For
/// each section, find the region whose `name` matches the section's, then set
/// `section.lma = region.lma_base + <bytes already placed in that region>`,
/// packing multiple same-named sections sequentially within the region (in the
/// order they appear). `vma_base` is preserved untouched — placement only moves
/// bytes physically, never their VMA/PC. A section whose name matches NO region is
/// a hard [`Level::Error`]; region-budget overflow is caught later by
/// `emit_rom`/`validate_section` (§7.3).
pub fn place_sections(sections: &mut [Section], map: &MemoryMap) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    // Cumulative bytes placed so far in each region, by region name.
    let mut used: HashMap<&str, u32> = HashMap::new();
    for sec in sections.iter_mut() {
        let Some(region) = map.regions.iter().find(|r| r.name == sec.name) else {
            diags.push(Diagnostic {
                level: Level::Error,
                message: format!("section `{}` has no region in the map", sec.name),
                // No span: the offending name comes from the module's `in <section>`
                // header, but the section itself carries none here. Best available.
                // TODO: thread the module-header span (like report_unresolved uses
                // pm.file.module.span) so this renders at the `in <name>` clause
                // instead of a misleading <first-file>:1:1.
                primary: Span {
                    source: sigil_span::SourceId(0),
                    start: 0,
                    end: 0,
                },
            });
            continue;
        };
        let cursor = used.entry(region.name.as_str()).or_insert(0);
        sec.lma = region.lma_base + *cursor;
        // Advance by the MAX address-span length (`placement_span`), not
        // `image_len` and not `vma_len`. `placement_span` (a) counts trailing
        // `ds`/`Reserve` (VMA/LMA space that emits no image bytes) so a sibling
        // never lands inside the reserved span — a silent overlap `flatten_checked`
        // never catches — AND (b) is panic-safe on the width-variable `jmp`/`jsr`
        // (`JmpJsrSym`) / deferred-operand (`RelaxAbsSym`) fragments, which
        // placement sees BEFORE `resolve_layout` lowers them (so `vma_len`'s
        // `unreachable!` would crash any code module). For data-only sections
        // `placement_span == vma_len == image_len`, so no behavior change there.
        *cursor += sec.placement_span();
    }
    diags
}

/// Pack every section CONTIGUOUSLY from `base`, in order, assigning each an LMA:
/// `sections[i].lma = base + Σ placement_span(sections[..i])`. This is the
/// no-`--map` default: without a region map nothing would place, so every module's
/// section would keep `lma == 0` and silently OVERLAP at the image origin (BUG I3).
/// Sequential packing makes a multi-module no-map build correct-by-default —
/// distinct, non-overlapping LMAs, so cross-module branches resolve to the right
/// addresses. `vma_base` is preserved untouched (placement moves bytes physically,
/// never their VMA/PC). `placement_span` is the MAX span (long width for
/// relaxables), so a later short-relax leaves a small gap but never an overlap.
pub fn place_sequential(sections: &mut [Section], base: u32) {
    let mut cursor = base;
    for sec in sections.iter_mut() {
        sec.lma = cursor;
        cursor += sec.placement_span();
    }
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
        enqueue_uses(&manifest.modules[idx].file.items, &mut queue, &mut seen, &enqueue);
    }
    order
}

/// Enqueue the BFS target of every `Item::Use` in `items`, recursing one level
/// into `section {}` bodies (sections do not nest further — Task 1 rejects that
/// at parse time) so a section-nested `use` is discovered too, not just
/// top-level ones.
fn enqueue_uses(
    items: &[ast::Item],
    queue: &mut VecDeque<(String, Span)>,
    seen: &mut HashSet<String>,
    enqueue: &impl Fn(&mut VecDeque<(String, Span)>, &mut HashSet<String>, String, Span),
) {
    for item in items {
        match item {
            ast::Item::Use(u) => {
                let target = u.base.segments.join(".");
                enqueue(queue, seen, target, u.span);
            }
            ast::Item::Section(sec) => enqueue_uses(&sec.items, queue, seen, enqueue),
            _ => {}
        }
    }
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
                // Resolvable to a canonical symbol — directly, or (for a dotted
                // exported label `Owner.local`) via its OWNER segment. The same
                // dotted-owner rule the rename pass uses (`canonicalize_name`), so
                // an accepted reference is exactly one the rename pass rewrites.
                // NOTE: acceptance guarantees REWRITABILITY, not existence — a
                // dotted name with a known owner but a typo'd local (`foo.typo`)
                // passes here and surfaces at link time as an undefined symbol.
                if rename::canonicalize_name(&s, env.rename_map()).is_some() {
                    continue;
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
