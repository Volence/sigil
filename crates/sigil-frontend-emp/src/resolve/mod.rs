//! Cross-module resolution driver (Spec 2 §3): gather modules, resolve
//! `use`/prelude names, place items, and produce one linkable Vec<Section>.
pub mod contract;
pub mod imports;
pub mod manifest;
pub mod rename;

use crate::ast;
use crate::lower::{lower_module_with_region_ends_and_contracts, LowerOptions};
use imports::{ExportIndex, ResolveEnv};
use manifest::{Manifest, ParsedModule};
use sigil_ir::map::MemoryMap;
use sigil_ir::{Section, SectionPlacement};
use sigil_span::{Diagnostic, Level, Span};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

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
        // A `pub context` (§3.1) is comptime-only — it emits nothing itself, and
        // a `with` bracket's bytes come from evaluating its acquire/release AT
        // THE USE SITE. Injecting the DECL is what makes an imported context
        // resolvable there. Consequence, and the rule a cross-module context must
        // obey: those exprs evaluate in the CONSUMER's scope, so they may name
        // only link-resolved symbols and names the consumer itself has — which is
        // why the engine's contexts spell their brackets inline rather than
        // calling a module-private template.
        ast::Item::Context(d) if d.public => Some(&d.name),
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

/// Per-build memo of "every `pub const` this defining module folds to".
///
/// `collect_pub_comptime` folds a defining module's `pub const`s at the DEFINITION
/// site once per CONSUMING module, so an M-const module imported by N modules paid
/// M×N evaluator constructions — each one a fresh 64 MiB-stack thread that
/// re-indexed the whole defining file to resolve a single integer. Folding the
/// module's consts once and reading the map collapses that to one construction per
/// defining module.
///
/// DELIBERATELY NOT a global or thread-local. This cache is created inside a single
/// `build_program_with` (or report) call and dropped with it, keyed by module id,
/// which is unique within one build. A cache that outlived a build and was keyed by
/// a name that repeats across builds would not fail loudly — it would produce a
/// WRONG ROM THAT PASSES, which is the one failure mode the golden/pin apparatus
/// cannot catch (the same argument that ruled out cross-invocation incremental
/// caching; lens sweep findings S15/S22).
#[derive(Default)]
struct ConstFoldCache {
    by_const: HashMap<(String, String), Option<i64>>,
    /// Error-level diagnostics from probes that failed on a fault in the const's
    /// own definition ([`ConstFold::Failed`]), accumulated once per `(module,
    /// const)` because the memo asks each question once. The driver holding this
    /// cache drains them into its own diagnostic list — see [`take_faults`].
    ///
    /// A sink rather than a return value: the fold runs deep inside the ambient
    /// walk, whose result type is a list of items, and every driver already has a
    /// single point where it owns the build's `Vec<Diagnostic>`.
    ///
    /// [`take_faults`]: ConstFoldCache::take_faults
    faults: Vec<Diagnostic>,
}

impl ConstFoldCache {
    /// This const's folded literal, computing it on first ask.
    ///
    /// The computation is [`fold_const_literal`] verbatim, so the answer for a
    /// given `(module, const)` is exactly what the uncached path produced — the
    /// saving is entirely in not asking the same question again.
    ///
    /// A [`ConstFold::Failed`] probe's diagnostics land in the cache's fault sink
    /// on the ask that computed them, so a fault is reported ONCE however many
    /// consumers import the const.
    ///
    /// Folding a module's consts EAGERLY as a batch would be faster still (one
    /// evaluator per module rather than one per const), but it is NOT equivalent,
    /// and the corpus says so: a batch also folds consts no consumer imports, and
    /// `raster_dsl.emp`'s `pub comptime fn fire` calls the module-private
    /// `op_cram_words`, so eager folding fails where the lazy path never asked and
    /// leaves 73 `unknown function` errors in its wake. Memoization is the part
    /// that is provably behaviour-preserving, so it is the part taken here.
    fn folded(
        &mut self,
        def_id: &str,
        def_file: &ast::File,
        name: &str,
        defines: &[(String, i128)],
        include_root: Option<&Path>,
    ) -> Option<i64> {
        let key = (def_id.to_string(), name.to_string());
        if let Some(&hit) = self.by_const.get(&key) {
            return hit;
        }
        let folded = match fold_const_literal(def_file, name, defines, include_root) {
            ConstFold::Literal(n) => Some(n),
            ConstFold::NotLiteral => None,
            ConstFold::Failed(faults) => {
                self.faults.extend(faults);
                None
            }
        };
        self.by_const.insert(key, folded);
        folded
    }

    /// Take the accumulated definition-site fold faults, leaving the sink empty.
    fn take_faults(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.faults)
    }
}

/// What the definition-site fold probe learned about one `pub const`.
///
/// The three outcomes are DELIBERATELY distinct. A const that simply is not a
/// foldable literal is an ordinary, expected result and must stay silent; a const
/// whose evaluation FAILED is a fault the author has to hear about. Collapsing both
/// into one "no value" answer lets a `const` whose `embed(...)` cannot be read reach
/// a green build with its error discarded.
enum ConstFold {
    /// Clean resolution to a value that fits the [`ast::Expr::Int`] payload.
    Literal(i64),
    /// Clean evaluation, but not an `i64` literal — a non-int value, an
    /// out-of-range magnitude, or a name the probe's narrower scope cannot see.
    /// Ordinary and silent: the caller keeps the const's original expression and
    /// the consumer resolves it exactly as it always did.
    NotLiteral,
    /// Evaluation raised Error-level diagnostics that do NOT come from the probe's
    /// narrower scope — a fault in the const's own definition, true for every
    /// consumer. Carries those diagnostics so the caller can report them.
    Failed(Vec<Diagnostic>),
}

/// Whether `d` is the fold probe's OWN scope shortfall rather than a fault in the
/// const being folded.
///
/// The probe evaluates a const against its DEFINING file alone, so a name that
/// file cannot see — a `use`d sibling const, a comptime fn another module owns, an
/// interface member (the probe seeds an empty [`crate::contract::InterfaceEnv`]) —
/// misses. That miss is expected and carries no information: the consumer resolves
/// the same expression in its own, wider scope. It is also the ONLY class of Error
/// the shipped corpus's fold probes raise, across every shipped shape — the whole
/// population is `unknown name` / `unknown function`, which is what makes treating
/// every OTHER Error as a real fault safe rather than noisy.
///
/// That "only class" holds because the probe runs with the D-PP.3 label fallback
/// OFF ([`crate::eval::eval_const_in_partial_scope`]). With the fallback ON, a
/// miss inside a call argument becomes a `Value::Label` instead of a diagnostic,
/// and whatever downstream check then refuses that label reports a message with
/// no missing symbol in it — a shortfall this predicate cannot recognise, in an
/// open-ended set of wordings that grows with every new check on a value. The
/// fallback is off precisely so this predicate keeps facing one small, closed
/// population.
///
/// Conservative in the loud direction: if either message is ever reworded this
/// predicate stops matching and those diagnostics start surfacing, which is a
/// visible failure, not a silent one.
fn is_probe_scope_shortfall(d: &Diagnostic) -> bool {
    d.message.starts_with("unknown name `") || d.message.starts_with("unknown function `")
}

/// Resolve the `pub const` named `name` to an `i64` literal in its DEFINING file's
/// scope (siblings + `defines` visible), for the injected-clone value fold in
/// [`collect_pub_comptime`].
///
/// [`ConstFold::Literal`] ONLY on a clean resolution: no Error-level diagnostic, an
/// integer value, and a magnitude that fits `i64`. A value that resolves cleanly to
/// something else, and a miss the probe's narrower scope explains
/// ([`is_probe_scope_shortfall`]), are [`ConstFold::NotLiteral`] — silent, caller
/// keeps the original expression. Any other Error is [`ConstFold::Failed`], which
/// the caller reports: the const's definition is broken for every consumer, and
/// nothing downstream is guaranteed to raise it again (a const nobody demands is
/// never evaluated a second time).
///
/// Reached only through [`ConstFoldCache::folded`] — every caller wants the memo.
fn fold_const_literal(
    def_file: &ast::File,
    name: &str,
    defines: &[(String, i128)],
    include_root: Option<&Path>,
) -> ConstFold {
    // PARTIAL SCOPE: `def_file` alone, without the defining module's own `use`
    // imports, so the D-PP.3 label fallback would be inferring a link symbol from
    // a name that is only missing because of how this probe is scoped. Off here —
    // see `eval_const_in_partial_scope`.
    let (value, diags) = crate::eval::eval_const_in_partial_scope(
        def_file,
        name,
        include_root,
        defines,
        &crate::contract::InterfaceEnv::empty(),
    );
    let errors: Vec<Diagnostic> =
        diags.into_iter().filter(|d| d.level == Level::Error).collect();
    if !errors.is_empty() {
        // An errored evaluation NEVER folds, whatever it left in `value` — a
        // poisoned partial result is not a literal. Which kind of error it was
        // decides only whether the author hears about it.
        let faults: Vec<Diagnostic> =
            errors.into_iter().filter(|d| !is_probe_scope_shortfall(d)).collect();
        return if faults.is_empty() { ConstFold::NotLiteral } else { ConstFold::Failed(faults) };
    }
    match value.and_then(|v| v.as_stored_int()).and_then(|n| i64::try_from(n).ok()) {
        Some(n) => ConstFold::Literal(n),
        None => ConstFold::NotLiteral,
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
// 8 args: `def_id` identifies the defining module for the fold memo, which is
// keyed by (module, const) — see `ConstFoldCache`.
#[allow(clippy::too_many_arguments)]
fn collect_pub_comptime(
    def_id: &str,
    def_file: &ast::File,
    items: &[ast::Item],
    pred: &impl Fn(&str) -> bool,
    defines: &[(String, i128)],
    include_root: Option<&Path>,
    folds: &RefCell<ConstFoldCache>,
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
            // A `const NAME: T = EXPR`'s initializer resolves in its DEFINING
            // module's scope, where its sibling consts are visible. A `use`d
            // consumer sees only the named import, not those siblings, so a
            // derived initializer (`= OTHER_CONST + n`) would fail `unknown name`
            // when the injected clone's expression re-evaluates in the consumer.
            // Fold the value HERE, at the definition site (the const-value analogue
            // of `stamp_overlay_window`): resolve it against `def_file` and replace
            // the injected clone's expression with the resolved literal, so the
            // consumer reads a self-contained value and no sibling name leaks into
            // its scope. Best-effort — a value that does not resolve cleanly to an
            // `i64` (a cross-module reference `def_file` alone cannot see, a
            // non-int, an out-of-range magnitude) keeps its original expression, so
            // behavior is unchanged for every const the consumer already resolved.
            if let ast::Item::Const(c) = &mut cloned {
                let folded =
                    folds.borrow_mut().folded(def_id, def_file, &c.name, defines, include_root);
                if let Some(lit) = folded {
                    c.value = ast::Expr::Int(lit, c.span);
                }
            }
            out.push(cloned);
        }
        if let ast::Item::Section(sec) = item {
            collect_pub_comptime(def_id, def_file, &sec.items, pred, defines, include_root, folds, out);
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
/// Whether a file declares an `implement` block (top-level or one section deep,
/// the bind pass's own recursion depth). These are the only bind modules that
/// need ambient-prepend: their binding VALUES / comptime-`if` conditions may read
/// `use`d game consts, which the plain file's scope lacks.
/// How many item-position `ensure`/`ensure_fatal` guards this file declares —
/// top level plus one `section` deep, the same depth the lowering walks.
///
/// An item-position `ensure` is evaluated IFF its module is lowered, and a module
/// is lowered iff it is in the `use`-reachability closure of the profile's entry.
/// A module outside that closure therefore ships guards that never run, and
/// nothing said so — no warning, no report mode; the only way to learn the rule
/// was to read `native.rs` (lens sweep, seat COMPTIME, finding S21).
fn ensure_count(file: &ast::File) -> usize {
    fn walk(items: &[ast::Item], depth: u32) -> usize {
        items
            .iter()
            .map(|it| match it {
                ast::Item::Ensure(_) => 1,
                ast::Item::Section(s) if depth == 0 => walk(&s.items, 1),
                _ => 0,
            })
            .sum()
    }
    walk(&file.items, 0)
}

fn file_declares_implement(file: &ast::File) -> bool {
    file.items.iter().any(|it| match it {
        ast::Item::Implement(_) => true,
        ast::Item::Section(s) => s.items.iter().any(|i| matches!(i, ast::Item::Implement(_))),
        _ => false,
    })
}

fn ambient_items(
    module: &ParsedModule,
    prelude: Option<&ParsedModule>,
    manifest: &Manifest,
    defines: &[(String, i128)],
    include_root: Option<&Path>,
    folds: &RefCell<ConstFoldCache>,
) -> Vec<ast::Item> {
    let mut out = Vec::new();

    // Prelude first (own items, added in Part B, shadow these via last-wins).
    if let Some(p) = prelude {
        if p.id != module.id {
            collect_pub_comptime(&p.id, &p.file, &p.file.items, &|_| true, defines, include_root, folds, &mut out);
        }
    }

    // Then `use`-imported pub comptime-only items (these shadow prelude, matching
    // the prelude<use precedence; own items shadow both via Part B ordering).
    // Recurses one level into `section {}` bodies so a section-nested `use` is
    // honored too, not just top-level ones.
    ambient_from_uses(&module.file.items, module, manifest, defines, include_root, folds, &mut out);

    out
}

fn ambient_from_uses(
    items: &[ast::Item],
    module: &ParsedModule,
    manifest: &Manifest,
    defines: &[(String, i128)],
    include_root: Option<&Path>,
    folds: &RefCell<ConstFoldCache>,
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
                    // Whole-path label import — handled by rename/link. The blank
                    // import binds nothing at all, and both leave the closure edge
                    // (`enqueue_uses`) to do the work, so neither injects here.
                    ast::UseNames::Whole | ast::UseNames::Blank => {}
                    ast::UseNames::Glob => collect_pub_comptime(
                        &base_mod.id,
                        &base_mod.file,
                        &base_mod.file.items,
                        &|_| true,
                        defines,
                        include_root,
                        folds,
                        out,
                    ),
                    ast::UseNames::List(names) => collect_pub_comptime(
                        &base_mod.id,
                        &base_mod.file,
                        &base_mod.file.items,
                        &|n| names.iter().any(|w| w == n),
                        defines,
                        include_root,
                        folds,
                        out,
                    ),
                }
            }
            ast::Item::Section(sec) => {
                ambient_from_uses(&sec.items, module, manifest, defines, include_root, folds, out)
            }
            _ => {}
        }
    }
}

/// The RAM map report (T1): resolve the named RAM `region` modules into per-region
/// geometry rows (name, base/end, size, padding, budget headroom). Pure analysis — no
/// lowering, no link, no ROM. It reuses the SAME region resolver the byte-emitting build
/// runs ([`crate::lower::collect_region_report`], over
/// [`crate::lower::resolve_program_region_ends`] for cross-module `after(..)` chains),
/// so the numbers ARE the shipping layout.
///
/// `region_module_ids` is EXPLICIT (e.g. `["engine.ram", "games.sonic4.ram"]`) because
/// the RAM modules are not `use`-reachable — their `pub vars` are cross-seam link labels
/// no module imports, so a `use`-graph BFS never finds them. Passing the set explicitly
/// also scopes the report to ONE game (sonic4's `game_ram` vs demo's, both declared
/// `game_ram` in sibling modules). Order is preserved; a `game_ram @ after(upper_ram)`
/// resolves regardless of listing order (the whole-program end pass is a fixpoint).
///
/// Each module is ambient-prepended (its `use`d comptime sizes — `SYSTEM_STACK`, the
/// `DEBUG` flag, `sizeof(VdpShadow)`, the game `-D` sizing consts — resolve), exactly as
/// `build_program`'s region pass does. An unknown id yields a diagnostic and is skipped.
/// Rows come back in `region_module_ids` order; diagnostics are the resolver's own
/// (overflow, odd-field, unknown parent, chain cycle).
pub fn build_ram_report(
    manifest: &Manifest,
    region_module_ids: &[&str],
    opts: &LowerOptions,
) -> (Vec<crate::lower::RamRegionRow>, Vec<Diagnostic>) {
    let seed_span = Span { source: sigil_span::SourceId(0), start: 0, end: 0 };
    let mut diags: Vec<Diagnostic> = Vec::new();
    // Scoped to THIS report and dropped with it — see `ConstFoldCache`.
    let folds = RefCell::new(ConstFoldCache::default());

    // Gather each named module, ambient-prepended so its `use`d comptime sizes resolve.
    let mut region_modules: Vec<(&str, ast::File)> = Vec::new();
    for id in region_module_ids {
        let Some(&i) = manifest.by_id.get(*id) else {
            diags.push(Diagnostic {
                level: Level::Error,
                message: format!("--report ram: no module `{id}` under the scan root"),
                primary: seed_span,
            });
            continue;
        };
        let pm = &manifest.modules[i];
        if !crate::lower::file_declares_region(&pm.file) {
            diags.push(Diagnostic {
                level: Level::Warning,
                message: format!(
                    "[ram.no-region] --report ram: module `{id}` declares no `region` — skipped"
                ),
                primary: seed_span,
            });
            continue;
        }
        let ambient =
            ambient_items(pm, None, manifest, &opts.defines, opts.include_root.as_deref(), &folds);
        let file = if ambient.is_empty() {
            pm.file.clone()
        } else {
            ast::File {
                module: pm.file.module.clone(),
                attrs: pm.file.attrs.clone(),
                items: ambient.into_iter().chain(pm.file.items.iter().cloned()).collect(),
                docs: pm.file.docs.clone(),
            }
        };
        region_modules.push((pm.id.as_str(), file));
    }

    // Cross-module `after(..)` ends first (game_ram chains onto the engine's upper_ram),
    // then each region module's rows against those ends.
    let (region_ends, mut end_diags) =
        crate::lower::resolve_program_region_ends(&region_modules, &opts.defines);
    diags.append(&mut end_diags);

    let mut rows = Vec::new();
    for (_id, file) in &region_modules {
        let (mut r, mut d) = crate::lower::collect_region_report(file, &opts.defines, &region_ends);
        rows.append(&mut r);
        diags.append(&mut d);
    }

    // Definition-site fold faults from the ambient prepends above. This path never
    // lowers the defining modules, so nothing else here would ever raise them.
    diags.append(&mut folds.borrow_mut().take_faults());
    (rows, diags)
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
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>, Vec<Diagnostic>) {
    build_program_with(manifest, entry_id, prelude_id, opts, true, true, &|_| opts.embed_base.clone())
}

/// [`build_program`] for an OPEN program — one whose `.emp` modules reference
/// symbols supplied by a SEPARATE unit linked alongside them (the flip Stage-1
/// mixed native build: the `.emp` engine references AS-residual RAM labels / proc
/// seams resolved only in the joint symbol table). Identical to `build_program`
/// except it does NOT report unresolved references as errors: `lower_module`
/// already emits those as link-time fixups, and whether they resolve is decided by
/// the joint `link` over the union, not here. Names that stay undefined surface as
/// the linker's own "undefined symbol" errors, so nothing is silently dropped.
pub fn build_program_open(
    manifest: &Manifest,
    entry_id: &str,
    prelude_id: Option<&str>,
    opts: &LowerOptions,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>, Vec<Diagnostic>) {
    build_program_with(manifest, entry_id, prelude_id, opts, false, true, &|_| opts.embed_base.clone())
}

/// [`build_program_open`] with a PER-MODULE `embed_base` override. The aeon `.emp`
/// tree mixes two `embed(...)` path conventions — module-relative
/// (`math.emp: "../data/sine.bin"`) and repo-root-relative
/// (`object_test_state.emp: "games/sonic4/test/ring_art.bin"`) — that no single
/// `embed_base` satisfies. `embed_base_for(module_id)` picks the base per module;
/// `None` falls back to `opts.embed_base`. The isolated port oracles pick the base
/// per module already; this restores that freedom in the whole-program build.
///
/// It also SKIPS the canonical-rename pass, so exported labels keep their PLAIN
/// names (`AnimateSprite`, not `engine.objects.animate.AnimateSprite`). The mixed
/// build links `.emp` sections against an AS residual that references those procs
/// by their bare cross-seam names (the `pub proc` link contract the port oracles
/// prove); module-qualifying them would leave every AS→`.emp` call unresolved.
/// Private labels already carry module-unique `$`-hygiene from lowering, so bare
/// exports collide only if two modules export the same name — which the aeon
/// cross-seam contract forbids, exactly as the flat link table already assumes.
pub fn build_program_open_embed(
    manifest: &Manifest,
    entry_id: &str,
    prelude_id: Option<&str>,
    opts: &LowerOptions,
    embed_base_for: &dyn Fn(&str) -> Option<PathBuf>,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>, Vec<Diagnostic>) {
    build_program_with(manifest, entry_id, prelude_id, opts, false, false, embed_base_for)
}

/// Shared body of [`build_program`] / [`build_program_open`]. `closed` gates the
/// unresolved-reference check: a closed pure-`.emp` program must define every
/// name it references (CLI `sigil emp`); an open one defers them to the joint link.
/// `rename` gates the canonical-rename pass: a pure-`.emp` program module-qualifies
/// its cross-module symbols, while the mixed AS+`.emp` build keeps them bare.
/// `embed_base_for` overrides `opts.embed_base` per module.
fn build_program_with(
    manifest: &Manifest,
    entry_id: &str,
    prelude_id: Option<&str>,
    opts: &LowerOptions,
    closed: bool,
    rename: bool,
    embed_base_for: &dyn Fn(&str) -> Option<PathBuf>,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let mut sections = Vec::new();
    // Scoped to THIS build and dropped with it — see `ConstFoldCache`.
    let folds = RefCell::new(ConstFoldCache::default());
    // Deferred link-time assertions (D-H.4) from every reachable module,
    // concatenated (post-rename) — the whole-program list the linker decides.
    let mut link_asserts: Vec<sigil_ir::LinkAssert> = Vec::new();

    // 1. Reachability BFS over `use` edges from the entry (and the prelude seed).
    let reachable = reachable_modules(manifest, entry_id, prelude_id, &mut diags);

    // [module.unreachable] — a module OUTSIDE the closure is never lowered, so the
    // item-position `ensure` guards it ships never evaluate. That is a silent shape
    // rule: a guard can be written, reviewed, and merged while being incapable of
    // firing, and no diagnostic distinguishes it from one that holds. Report the
    // ones that carry guards, so an unevaluated guard is a visible fact about the
    // profile rather than something you learn by reading the driver.
    //
    // Warning, not error: not being reachable is legitimate (a module can belong to
    // another profile, or be lowered through a different seam — the sound modules
    // reach lowering via seam-1/seam-2, not this closure). What is NOT legitimate
    // is not knowing.
    {
        let in_closure: HashSet<usize> = reachable.iter().copied().collect();
        for (i, pm) in manifest.modules.iter().enumerate() {
            if in_closure.contains(&i) {
                continue;
            }
            let n = ensure_count(&pm.file);
            if n == 0 {
                continue;
            }
            diags.push(Diagnostic {
                level: Level::Warning,
                message: format!(
                    "[module.unreachable] module `{}` is outside this profile's `use` closure, \
                     so its {n} `ensure` guard(s) are never evaluated for this target — they \
                     cannot fail here, whatever they assert. (A module lowered through another \
                     seam evaluates them THERE: the Z80 sound modules reach lowering via \
                     seam-1/seam-2, not this closure. Check which applies before treating it as \
                     a defect — and do not `use` a module merely to silence this.)",
                    pm.id
                ),
                primary: pm.file.module.span,
            });
        }
    }

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

    // C1 item 4: a `pub equ` keeps its plain link name, so two modules declaring
    // the same one genuinely collide in the flat link table. Detect it HERE,
    // where module identity is known, so the diagnostic can NAME both modules
    // (the linker's dup-symbol check sees only section names). `pub equ` name →
    // the first module id that declared it.
    let mut pub_equ_owner: HashMap<String, String> = HashMap::new();

    // Item #7c: whole-program region-END resolution for cross-module `after(..)`
    // chains. A region-form `vars` block may chain `@ after(<region>)` onto a
    // region declared in another module (the game's `game_ram @ after(upper_ram)`
    // onto the engine's `upper_ram`). Resolve every region's running end ONCE,
    // here, so each module's per-file region lowering (below) can look up a
    // cross-module parent's end. Each region module is passed as its
    // ambient-prepended synthetic file so a region's `use`d comptime sizes resolve
    // — identical to what the per-module loop lowers. A no-op (empty map, zero
    // passes) for a program with no region modules, so a region-free build is
    // untouched. `[region.chain-cycle]` for a cross-module `after(..)` cycle.
    let region_modules: Vec<(&str, ast::File)> = reachable
        .iter()
        .filter_map(|&i| {
            let pm = &manifest.modules[i];
            if !crate::lower::file_declares_region(&pm.file) {
                return None;
            }
            let ambient =
                ambient_items(pm, prelude_pm, manifest, &opts.defines, opts.include_root.as_deref(), &folds);
            let file = if ambient.is_empty() {
                pm.file.clone()
            } else {
                ast::File {
                    module: pm.file.module.clone(),
                    attrs: pm.file.attrs.clone(),
                    items: ambient.into_iter().chain(pm.file.items.iter().cloned()).collect(),
                    docs: pm.file.docs.clone(),
                }
            };
            Some((pm.id.as_str(), file))
        })
        .collect();
    let (region_ends, mut region_diags) =
        crate::lower::resolve_program_region_ends(&region_modules, &opts.defines);
    diags.append(&mut region_diags);

    // L1 (P2): the game-contract bind pass over the reachable module set. Collect
    // every `interface`/`implement` and resolve each interface against its one
    // `implement`, producing the [`InterfaceEnv`](crate::contract::InterfaceEnv)
    // the per-module lowering threads so `Game.MEMBER` folds and `invoke
    // Game.hook` lowers (a bound `jsr`, or nothing when `= empty`). A build with
    // NO contract declarations yields the empty env — no diagnostics, byte-
    // identical to the whole pre-L1 corpus. An `implement` module is ambient-
    // prepended (its binding VALUES may read `use`d game consts, e.g.
    // `ENTRY_ID = GS_OJZ_SCROLL_TEST`); every other reachable module contributes
    // its plain items — the proc / `extern proc` contracts the hook-signature
    // check reads, and the `type = proc` contract types — at no ambient cost.
    let contract_ambient: Vec<(usize, ast::File)> = reachable
        .iter()
        .filter_map(|&i| {
            let pm = &manifest.modules[i];
            if !file_declares_implement(&pm.file) {
                return None;
            }
            let ambient = ambient_items(
                pm,
                prelude_pm,
                manifest,
                &opts.defines,
                opts.include_root.as_deref(),
                &folds,
            );
            if ambient.is_empty() {
                return None;
            }
            let file = ast::File {
                module: pm.file.module.clone(),
                attrs: pm.file.attrs.clone(),
                items: ambient.into_iter().chain(pm.file.items.iter().cloned()).collect(),
                docs: pm.file.docs.clone(),
            };
            Some((i, file))
        })
        .collect();
    let contract_ambient_by_idx: HashMap<usize, &ast::File> =
        contract_ambient.iter().map(|(i, f)| (*i, f)).collect();
    let contract_mods: Vec<contract::ContractModule> = reachable
        .iter()
        .map(|&i| {
            let pm = &manifest.modules[i];
            let file = contract_ambient_by_idx.get(&i).copied().unwrap_or(&pm.file);
            contract::ContractModule { id: pm.id.as_str(), file }
        })
        .collect();
    let (iface_env, mut contract_diags) = contract::bind(&contract_mods, &opts.defines);
    diags.append(&mut contract_diags);

    // 4. Per-module: resolve names, lower, report unresolved, rename, concat.
    for &i in &reachable {
        let pm = &manifest.modules[i];
        for equ_name in imports::pub_equ_names(&pm.file) {
            if let Some(prev) = pub_equ_owner.get(&equ_name) {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!(
                        "[equ.collision] `pub equ {equ_name}` is declared by both module `{prev}` \
                         and module `{}` — a `pub equ` is a plain cross-seam link symbol, so its \
                         name must be unique across the program; rename one, or make one non-`pub` \
                         (a private equ is module-scoped)",
                        pm.id
                    ),
                    primary: pm.file.module.span,
                });
            } else {
                pub_equ_owner.insert(equ_name, pm.id.clone());
            }
        }
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
        // Per-module `embed_base` (the aeon tree mixes module-relative and
        // repo-root-relative `embed(...)` conventions). Clone opts only when the
        // override differs from the ambient one, keeping the common path cheap.
        let module_embed_base = embed_base_for(&pm.id);
        let per_module_opts;
        let opts: &LowerOptions = if module_embed_base == opts.embed_base {
            opts
        } else {
            per_module_opts = LowerOptions { embed_base: module_embed_base, ..opts.clone() };
            &per_module_opts
        };

        let ambient =
            ambient_items(pm, prelude_pm, manifest, &opts.defines, opts.include_root.as_deref(), &folds);
        let (mut module, ldiags) = if ambient.is_empty() {
            // zero-clone common path; `region_ends` is empty for region-free builds.
            lower_module_with_region_ends_and_contracts(&pm.file, opts, &region_ends, &iface_env)
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
                // Docs are keyed by item span and cloned items keep their
                // spans, so the module's own entries stay valid (ambient
                // prelude docs live in the prelude's own File; no consumer
                // reads docs during lowering yet — S2-D11(d) is parse-and-
                // attach only).
                docs: pm.file.docs.clone(),
            };
            lower_module_with_region_ends_and_contracts(&synthetic, opts, &region_ends, &iface_env)
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

        if closed {
            report_unresolved(pm, &module, &env, &mut diags);
        }

        if rename {
            rename::rename_module(&mut module, env.rename_map());
        }
        sections.extend(module.sections);
        link_asserts.extend(module.link_asserts);
    }

    // Item #7a §2.3: a region's `vars` blocks must all live in one owner module
    // (`[region.multiple-owners]`). A whole-program check over the reachable set
    // — per-module lowering cannot see it. A no-op (no diagnostics) for a program
    // with no region-form `vars` blocks.
    let reachable_pairs: Vec<(&str, &ast::File)> = reachable
        .iter()
        .map(|&i| (manifest.modules[i].id.as_str(), &manifest.modules[i].file))
        .collect();
    diags.extend(crate::lower::check_single_owner(&reachable_pairs));

    // Definition-site fold faults (`ConstFold::Failed`): a `pub const` whose own
    // evaluation raised an Error that the probe's narrower scope does not explain.
    // The caller kept the const's original expression, so the value is still
    // whatever the consumer's scope computes — the fault has to be said out loud
    // or it is not said at all: a const no consumer DEMANDS is never evaluated
    // again, and one that is demanded may resolve to a different, wrong value
    // there rather than to an error.
    //
    // Filtered through `seen_across_modules` for the demanded case: when a
    // consumer's lowering already reported the identical diagnostic (same level,
    // message and span — a cloned item keeps its home span), this adds nothing.
    // So the change is strictly additive, and only where the build was silent.
    let fold_faults: Vec<Diagnostic> = folds
        .borrow_mut()
        .take_faults()
        .into_iter()
        .filter(|d| !seen_across_modules.contains(d))
        .collect();
    diags.extend(fold_faults);

    (sections, link_asserts, diags)
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
        // Item #7b: a RAM section (the reserve-only Core section a region-form
        // `vars` block lowers to) is VMA-placed at its region base already and
        // contributes ZERO image bytes — it never occupies a ROM map region.
        // Skip it: matching its name (`upper_ram`/`lower_ram`) against the ROM
        // regions would spuriously fire the no-region error. The RAM/ROM split
        // is `vma_origin >= $F00000` (the `is_rom_section` threshold): RAM lives
        // at `$FFFF0000`/`$FFFF8000`+, far above any ROM VMA. Its `lma` is left
        // as the builder stamped it (irrelevant — no bytes flatten from it).
        if sec.vma_origin() >= 0x00F0_0000 {
            continue;
        }
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
        let region_name = region.name.as_str();
        // §7p: this placer, not the builder's baked chain, is the placement
        // authority on the `--map` path — overwrite provenance on every section
        // it places rather than trusting whatever `IrBuilder` stamped. The FIRST
        // section landed in a given region is `Pinned` at that region's
        // `lma_base`; every subsequent section sharing the region is `Chained`
        // (its base derives from the prior one at link time). `group` records
        // the region name so the later placement pass can tell sections apart
        // by destination.
        let first_in_region = !used.contains_key(region_name);
        let cursor = used.entry(region_name).or_insert(0);
        sec.lma = region.lma_base + *cursor;
        sec.placement =
            if first_in_region { SectionPlacement::Pinned } else { SectionPlacement::Chained };
        sec.group = Some(region.name.clone());
        // Advance by the MAX address-span length (`placement_span`), not
        // `image_len` and not `vma_len`. `placement_span` (a) counts trailing
        // `ds`/`Reserve` (VMA/LMA space that emits no image bytes) so a sibling
        // never lands inside the reserved span — a silent overlap `flatten_checked`
        // never catches — AND (b) is panic-safe on the width-variable `jmp`/`jsr`
        // (`JmpJsrSym`) / deferred-operand (`RelaxAbsSym`) fragments, which
        // placement sees BEFORE `resolve_layout` lowers them (so `vma_len`'s
        // `unreachable!` would crash any code module). For data-only sections
        // `placement_span == vma_len == image_len`, so no behavior change there.
        // Also RECORDED as `reserved_span` — the placement-provenance field a
        // later link-time pass (T4) will read instead of re-deriving it.
        let span = sec.placement_span();
        *cursor += span;
        sec.reserved_span = span;
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
    for (i, sec) in sections.iter_mut().enumerate() {
        sec.lma = cursor;
        // §7p: this placer is the placement authority on the no-`--map` path —
        // overwrite provenance rather than trusting the builder's baked chain.
        // The first section overall is `Pinned` at `base`; every subsequent one
        // is `Chained` (packed after its predecessor at link time). There is no
        // region here, so `group` stays the anonymous group (`None`).
        sec.placement = if i == 0 { SectionPlacement::Pinned } else { SectionPlacement::Chained };
        sec.group = None;
        let span = sec.placement_span();
        cursor += span;
        sec.reserved_span = span;
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
                if s.contains('$') {
                    // A compiler-minted symbol — `$` is unlexable in BOTH
                    // frontends, so ANY `$`-bearing name is internal and
                    // resolved intra-module by construction: proc-local
                    // hygiene (`$m$…`), dispatch/offsets inline-body labels
                    // (`__dispatch$…` / `__offsets$…`), here()/align anchors.
                    // (Previously only a LEADING `$` was accepted, which
                    // wrongly rejected the mid-name-`$` hidden-label family
                    // under the program path — tranche-0 acceptance catch.)
                    continue;
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

#[cfg(test)]
mod placement_provenance_tests {
    use super::*;
    use sigil_ir::map::{Region, RegionKind};
    use sigil_ir::{Cpu, DataFragment, Fragment, SectionPlacement};
    use sigil_span::SourceId;

    fn span() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    /// A bare, pre-placement section carrying `len` bytes of `Data` and stale
    /// (builder-baked) provenance — `Chained`/`reserved_span: 0`/`group: None`
    /// regardless of `len`, so a passing test proves the placer OVERWROTE these
    /// fields rather than merely observing the builder's own defaults.
    fn stub_section(name: &str, len: u32) -> Section {
        Section {
            name: name.to_string(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0u8; len as usize],
                fixups: vec![],
                span: span(),
            })],
            placement: SectionPlacement::Chained,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    #[test]
    fn place_sequential_marks_first_pinned_rest_chained_with_max_span() {
        let mut sections = vec![stub_section("a", 4), stub_section("b", 3), stub_section("c", 5)];
        place_sequential(&mut sections, 0x1000);

        assert_eq!(sections[0].placement, SectionPlacement::Pinned);
        assert_eq!(sections[0].lma, 0x1000);
        assert_eq!(sections[0].reserved_span, sections[0].placement_span());
        assert_eq!(sections[0].reserved_span, 4);
        assert_eq!(sections[0].group, None);

        assert_eq!(sections[1].placement, SectionPlacement::Chained);
        assert_eq!(sections[1].reserved_span, sections[1].placement_span());
        assert_eq!(sections[1].reserved_span, 3);
        assert_eq!(sections[1].group, None);

        assert_eq!(sections[2].placement, SectionPlacement::Chained);
        assert_eq!(sections[2].reserved_span, sections[2].placement_span());
        assert_eq!(sections[2].reserved_span, 5);
        assert_eq!(sections[2].group, None);
    }

    #[test]
    fn place_sections_stamps_group_and_first_per_region_pinned() {
        let map = MemoryMap::new(
            vec![
                Region {
                    name: "regionA".to_string(),
                    lma_base: 0x2000,
                    size: 0x100,
                    kind: RegionKind::Rom,
                    vma_base: None,
                },
                Region {
                    name: "regionB".to_string(),
                    lma_base: 0x5000,
                    size: 0x100,
                    kind: RegionKind::Rom,
                    vma_base: None,
                },
            ],
            0xFF,
        );
        // Two sections placed into regionA (first-then-second), one into regionB.
        let mut sections =
            vec![stub_section("regionA", 4), stub_section("regionB", 6), stub_section("regionA", 2)];
        let diags = place_sections(&mut sections, &map);
        assert!(diags.is_empty());

        // First section landed in regionA: Pinned at the region's lma_base.
        assert_eq!(sections[0].placement, SectionPlacement::Pinned);
        assert_eq!(sections[0].lma, 0x2000);
        assert_eq!(sections[0].group, Some("regionA".to_string()));
        assert_eq!(sections[0].reserved_span, sections[0].placement_span());
        assert_eq!(sections[0].reserved_span, 4);

        // First section landed in regionB: also Pinned (first-per-region, not
        // first-overall) at ITS region's lma_base.
        assert_eq!(sections[1].placement, SectionPlacement::Pinned);
        assert_eq!(sections[1].lma, 0x5000);
        assert_eq!(sections[1].group, Some("regionB".to_string()));
        assert_eq!(sections[1].reserved_span, sections[1].placement_span());
        assert_eq!(sections[1].reserved_span, 6);

        // Second section into regionA: Chained (not the first in its region).
        assert_eq!(sections[2].placement, SectionPlacement::Chained);
        assert_eq!(sections[2].group, Some("regionA".to_string()));
        assert_eq!(sections[2].reserved_span, sections[2].placement_span());
        assert_eq!(sections[2].reserved_span, 2);
    }
}
