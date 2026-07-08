//! Lowering the checked, CPU-neutral `.emp` evaluator output into the Core IR
//! (Spec 2, Plan 4). This is the ONLY module in the crate that may import
//! `sigil_ir` / the backend crates (design decision D-P4.1): the pure evaluator
//! (`value`, `eval`, `layout`) stays Core-free so it can be tested in isolation.
//!
//! T0 proves the seam with the thinnest working path: `data` items only, one
//! section, defer-to-link placement (D-P4.2) — every pointer field becomes a
//! symbolic fixup the linker resolves. T2 grows the real per-CPU byte-order /
//! fixup-kind serializer, which lives in [`data`]. Instruction lowering is T3+.

mod code;
mod data;
pub(crate) mod hygiene;
pub mod patch;
mod proc;

pub use code::lower_code_buf;
pub(crate) use code::is_recognized_mnemonic;

use crate::ast;
use crate::eval::eval_proc_body;
use crate::layout::{
    eval_attr_int, eval_data_with_root, eval_dispatch_with_root, eval_offsets_with_root,
    validate_overlay, HerePos,
};
use sigil_ir::backend::{Cpu, IrStreamer};
use sigil_ir::{IrBuilder, Module};
use sigil_span::{Diagnostic, Level, Span};
use std::path::{Path, PathBuf};

/// Options controlling how a `.emp` module lowers to Core IR.
pub struct LowerOptions {
    /// The CPU the initial (top-level, no `section {}`) section is encoded for.
    pub initial_cpu: Cpu,
    /// The capability-sandbox root (Spec 2, Plan 5 — Task 5) `embed`/`import`
    /// paths resolve against while lowering `data` items. `None` (the default
    /// until a CLI wires the source file's directory in) means a comptime
    /// `embed`/`import` inside any lowered `data` item reports
    /// `[sandbox.no-root]`, exactly as it did before this option existed —
    /// every pre-existing `LowerOptions { initial_cpu, .. }` construction is
    /// therefore unaffected by adding this field.
    pub include_root: Option<PathBuf>,
}

/// The CPU, physical origin (`here()` base), and sandbox root a `data` item
/// lowers against — bundled into one struct so `lower_data_item`/
/// `lower_section_items` stay under clippy's argument-count lint (mirroring
/// how [`proc::Siblings`] already bundles a proc's fallthrough-adjacency
/// context). Just a borrow of what the caller already has; no owned state.
struct Placement<'a> {
    cpu: Cpu,
    origin: u32,
    include_root: Option<&'a Path>,
}

/// Lower a `.emp` module into Core IR, returning the finished [`Module`] plus any
/// diagnostics (from evaluation or lowering).
///
/// Top-level `data`/`proc` items lower into the default `text` section (VMA==LMA).
/// A `section name (cpu:, vma:) { .. }` (§7.1) opens a placed section: its bytes
/// land at the next physical LMA (a continuous counter across sections in
/// declaration order — emp's own placement policy, map-file regions being
/// S2-D3-deferred), while its labels/PC compute at the explicit `vma:` base. A
/// `cpu: z80` section lowers its code as Z80 and serializes its data
/// little-endian; the CPU flows through to the streamer and `lower_code_buf`.
///
/// NOTE: `"text"` is NOT a unique section handle. Interleaving top-level items
/// with `section {}` blocks can emit several distinct `Section`s all named
/// `"text"` (one per run of top-level items). Placement is by `lma`, never by
/// name — do not rely on name uniqueness (`LinkedImage::section("text")` returns
/// the FIRST match). This is fine because those sections carry disjoint labels
/// and non-overlapping LMAs; it is only a naming concern.
pub fn lower_module(file: &ast::File, opts: &LowerOptions) -> (Module, Vec<Diagnostic>) {
    let mut builder = IrBuilder::new();
    let mut diags = Vec::new();

    // Whole-file comptime validation that must fire exactly ONCE per compile
    // (not per per-item evaluator): duplicate `offsets` members. The evaluator's
    // `index_items` populates the offsets map on EVERY `with_file` construction
    // (once per data item / proc), so reporting there would duplicate the
    // diagnostic; this driver runs once.
    validate_offsets(&file.items, &mut diags);
    validate_dispatch(&file.items, &mut diags);

    // Spec 2 · Plan 6 (D-P6.3): a module-level `@as_compat` attribute marks this
    // file as a faithful port of AS-assembled source, opting it into the
    // byte-diff contract and silencing the modernization / faithful-port lints
    // (the `[proc.*]` heuristic WARNINGs — never the hard errors). On a data-only
    // module its observable byte effect is nil (proven byte-neutral by the Plan 6
    // harness); its load-bearing width/lint pinning rides instruction ports (the
    // attribute is read here so the mechanism is wired now, §3.2). Read straight
    // from the source (`file.attrs`) rather than a caller option: the file itself
    // declares its port status.
    let as_compat = file.attrs.iter().any(|a| a.name == "as_compat");

    // Continuous physical LMA counter across sections in declaration order
    // (mirrors the AS front-end's `phys_base`). INVARIANT: `next_lma` is the
    // physical start of the currently-open section. `builder.current_offset()`
    // (0 when nothing is open) is that section's length, so `next_lma +=
    // current_offset()` right before every `switch_section_lma` folds the
    // just-closed section's bytes and leaves `next_lma` at the new section's
    // start. The default `text` section is opened LAZILY (only when a top-level
    // item needs it), so a module that is all `section {}`s — or two adjacent
    // sections — materializes no empty `text` sections.
    let mut next_lma: u32 = 0;
    let mut default_open = false;

    // The name of the default (top-level items) section. `module x.y in obj_bank`
    // (§7) places this module's top-level code in the named section `obj_bank`;
    // absent the `in` clause it is the literal default `text`. VMA/LMA behavior is
    // unchanged — only the section NAME differs, so a later region-placement pass
    // (keyed by section name) can route the module's bytes to its map region.
    let default_name = file.module.in_section.as_deref().unwrap_or("text");

    // Diagnostics produced by the always-on `Item::Vars` overlay-validation pass
    // (Plan 7 #6). Overlay decl checks fire in EVERY evaluator that forces the
    // overlay's layout, and each per-item evaluator is fresh (own memo) — so an
    // erroring overlay that is also referenced via `sizeof`/`offsetof` in a data
    // item would report twice (once per pass). The struct exemplar reports once
    // only because lowering has NO always-on struct pass: its single forcing
    // evaluator is the referencing item's. To match that once-per-compile
    // behavior, `dedup_overlay_pass_diags` (end of this fn) drops later EXACT
    // copies (level+span+message) of the diagnostics collected here — exactness
    // means a genuinely distinct diagnostic can never be suppressed.
    let mut overlay_pass_diags: Vec<Diagnostic> = Vec::new();

    // The instantiation counter for `asm { }` / `proc` label hygiene (D-P4.6),
    // threaded across EVERY proc in the module (top-level AND section-nested) so
    // `k` stays globally monotonic. Lowering builds a fresh evaluator per proc,
    // which would otherwise restart `k` at 0 each proc and mint colliding
    // `$asm0…` symbols for `asm {}` bodies generated in different procs. Each
    // proc seeds from this value and hands back the advanced one.
    let mut asm_counter: u32 = 0;

    // Monotonic counter for item-guard anonymous `here()` ANCHOR labels (D-H.8),
    // threaded across every guard in the module (top-level AND section-nested) so
    // each provisional item guard that uses `here()` mints a program-unique name
    // `__here$<module>$<n>`. `$` is unlexable by both the emp and AS frontends, so
    // an anchor can never collide with a user symbol; module-qualification +
    // counter keeps it unique across the whole multi-module program (`link()` has
    // whole-program duplicate-label detection).
    let module_id = file.module.path.segments.join(".");
    let mut here_anchor_counter: u32 = 0;

    for (index, item) in file.items.iter().enumerate() {
        match item {
            ast::Item::Data(decl) => {
                ensure_default(&mut builder, &mut next_lma, &mut default_open, opts.initial_cpu, default_name);
                // Default section: vma_origin == lma == `next_lma` (VMA base
                // `None`, so origin == lma == its physical start).
                lower_data_item(
                    file,
                    decl,
                    &Placement {
                        cpu: opts.initial_cpu,
                        origin: next_lma,
                        include_root: opts.include_root.as_deref(),
                    },
                    &mut builder,
                    &mut diags,
                );
            }
            ast::Item::Proc(decl) => {
                ensure_default(&mut builder, &mut next_lma, &mut default_open, opts.initial_cpu, default_name);
                proc::lower_proc(
                    file,
                    decl,
                    proc::Siblings { index, items: &file.items },
                    proc::ProcCtx { cpu: opts.initial_cpu, as_compat },
                    &mut builder,
                    &mut diags,
                    &mut asm_counter,
                );
            }
            ast::Item::Offsets(decl) => {
                ensure_default(&mut builder, &mut next_lma, &mut default_open, opts.initial_cpu, default_name);
                lower_offsets_item(
                    file,
                    decl,
                    &Placement {
                        cpu: opts.initial_cpu,
                        origin: next_lma,
                        include_root: opts.include_root.as_deref(),
                    },
                    &mut builder,
                    &mut diags,
                );
            }
            ast::Item::Dispatch(decl) => {
                ensure_default(&mut builder, &mut next_lma, &mut default_open, opts.initial_cpu, default_name);
                lower_dispatch_item(
                    file,
                    decl,
                    &Placement {
                        cpu: opts.initial_cpu,
                        origin: next_lma,
                        include_root: opts.include_root.as_deref(),
                    },
                    as_compat,
                    &mut builder,
                    &mut diags,
                    &mut asm_counter,
                );
            }
            ast::Item::Ensure(decl) => {
                ensure_default(&mut builder, &mut next_lma, &mut default_open, opts.initial_cpu, default_name);
                // Default section: VMA == LMA == `next_lma`, so the current
                // position VMA is `next_lma + current_offset()` — the same
                // `here_base` a data item at this position would see.
                let cont = lower_item_guard(
                    file,
                    decl,
                    next_lma,
                    &module_id,
                    &mut here_anchor_counter,
                    opts.include_root.as_deref(),
                    &mut builder,
                    &mut diags,
                );
                if !cont {
                    break; // ensure_fatal: stop the module's remaining items (D5.3).
                }
            }
            ast::Item::Section(sec) => {
                // Close whatever is open (default text or a prior adjacent
                // section), folding its length into the counter.
                next_lma += builder.current_offset();
                default_open = false;
                let (cpu, vma) = section_attrs(file, sec, &mut diags);
                builder.switch_section_lma(&sec.name, cpu, Some(vma), next_lma);
                let cont = lower_section_items(
                    file,
                    sec,
                    &Placement { cpu, origin: vma, include_root: opts.include_root.as_deref() },
                    as_compat,
                    &module_id,
                    &mut here_anchor_counter,
                    &mut builder,
                    &mut diags,
                    &mut asm_counter,
                    &mut overlay_pass_diags,
                );
                // Leave the named section open; the next item (or `finish`)
                // folds its length.
                if !cont {
                    break; // a fatal guard inside the section stops the module (D5.3).
                }
            }
            ast::Item::Vars(decl) => {
                // Overlay form (`vars Name: window { .. }`): force its layout so
                // the always-on declaration checks (window/capacity/shadow) fire
                // (D6.A2); it emits ZERO bytes. Region form (`name: None`) is
                // inert by design (Plan 7 #6 OUT-list).
                if let Some(name) = &decl.name {
                    let mut d = validate_overlay(file, name, decl.span);
                    overlay_pass_diags.extend(d.iter().cloned());
                    diags.append(&mut d);
                }
            }
            _ => {}
        }
    }

    let (module, mut build_diags) = builder.finish();
    diags.append(&mut build_diags);
    dedup_overlay_pass_diags(&mut diags, &overlay_pass_diags);
    (module, diags)
}

/// Drop later EXACT copies (same level, span, AND message — [`Diagnostic`]'s
/// full `Eq`) of the diagnostics the always-on overlay-validation pass produced,
/// keeping the FIRST occurrence wherever it appeared (a referencing data item
/// may lower before OR after the `vars` decl). Diagnostics not in the pass set
/// are untouched, so pre-existing duplication behavior (e.g. two data items each
/// forcing an odd-field struct warning, no overlay involved) is unchanged, and a
/// DISTINCT diagnostic — differing in span, message, or level — can never be
/// suppressed. See the `overlay_pass_diags` comment in [`lower_module`] for the
/// struct-vs-overlay root cause.
fn dedup_overlay_pass_diags(diags: &mut Vec<Diagnostic>, pass: &[Diagnostic]) {
    if pass.is_empty() {
        return;
    }
    // O(n·m) scans: both lists are per-module diagnostic sets — tiny.
    let mut kept: Vec<Diagnostic> = Vec::new();
    diags.retain(|d| {
        if pass.contains(d) {
            if kept.contains(d) {
                return false;
            }
            kept.push(d.clone());
        }
        true
    });
}

/// Ensure the default (top-level items) section — named `name`, which is the
/// module's `in <section>` target or the literal `text` — is the currently-open
/// one before lowering a top-level item. If a named `section {}` block (or
/// nothing) is open, fold its length into `next_lma` and open the default at that
/// physical offset (VMA==LMA). A no-op when the default is already open.
fn ensure_default(
    builder: &mut IrBuilder,
    next_lma: &mut u32,
    default_open: &mut bool,
    cpu: Cpu,
    name: &str,
) {
    if !*default_open {
        *next_lma += builder.current_offset();
        builder.switch_section_lma(name, cpu, None, *next_lma);
        *default_open = true;
    }
}

/// Lower the items nested inside a `section {}` block into the already-open
/// section. `placement.cpu` is the section's CPU (drives byte order + code
/// lowering); `placement.origin` is its VMA base, used to compute `here()` for
/// each data item; `placement.include_root` is the sandbox root threaded to
/// every data item's `embed`/`import`.
///
/// Returns `false` when a failing `ensure_fatal` in this section aborted
/// evaluation, so the caller stops lowering the module's remaining items (D5.3).
#[allow(clippy::too_many_arguments)] // internal driver; mirrors lower_module's state set
fn lower_section_items(
    file: &ast::File,
    sec: &ast::SectionDecl,
    placement: &Placement,
    as_compat: bool,
    module_id: &str,
    here_anchor_counter: &mut u32,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
    asm_counter: &mut u32,
    overlay_pass_diags: &mut Vec<Diagnostic>,
) -> bool {
    for (index, item) in sec.items.iter().enumerate() {
        match item {
            ast::Item::Data(decl) => {
                lower_data_item(file, decl, placement, builder, diags);
            }
            ast::Item::Offsets(decl) => {
                lower_offsets_item(file, decl, placement, builder, diags);
            }
            ast::Item::Dispatch(decl) => {
                lower_dispatch_item(file, decl, placement, as_compat, builder, diags, asm_counter);
            }
            // Fallthrough adjacency is checked within THIS section's item list.
            ast::Item::Proc(decl) => {
                proc::lower_proc(
                    file,
                    decl,
                    proc::Siblings { index, items: &sec.items },
                    proc::ProcCtx { cpu: placement.cpu, as_compat },
                    builder,
                    diags,
                    asm_counter,
                );
            }
            ast::Item::Ensure(decl) => {
                let cont = lower_item_guard(
                    file,
                    decl,
                    placement.origin,
                    module_id,
                    here_anchor_counter,
                    placement.include_root,
                    builder,
                    diags,
                );
                if !cont {
                    return false; // ensure_fatal in-section: stop the whole module.
                }
            }
            ast::Item::Vars(decl) => {
                // Same as the top-level arm: overlay form → force layout so its
                // always-on checks fire, zero bytes; region form → inert.
                if let Some(name) = &decl.name {
                    let mut d = validate_overlay(file, name, decl.span);
                    overlay_pass_diags.extend(d.iter().cloned());
                    diags.append(&mut d);
                }
            }
            _ => {}
        }
    }
    true
}

/// Classify the `here()` position for an item whose provisional anchor (when the
/// open section already holds a size-relaxable fragment) is `anchor_name` — for a
/// data item its own label, which `lower_data_item` defines at exactly this byte
/// (D-H.3). At an EXACT position (no relaxable yet) the anchor is `None` and
/// `here()` returns the byte-identical `Value::Int(base)`; at a PROVISIONAL one it
/// is `Some(anchor_name)` and `here()` returns a link-time value (D-H.1).
fn here_pos(builder: &IrBuilder, origin: u32, anchor_name: &str) -> HerePos {
    let base = origin + builder.current_offset();
    let anchor = builder.section_has_relaxable().then(|| anchor_name.to_string());
    HerePos { base, anchor }
}

/// Lower one item-position guard (D5.2 / D-H.4). At an EXACT position it evaluates
/// eagerly (byte-identical to before — a passing guard is silent, a failing
/// `ensure_fatal` stops the module's remaining items via the `false` return). At
/// a PROVISIONAL position it hands the guard an anonymous `here()` ANCHOR
/// (`__here$<module>$<n>`, D-H.8); if the guard actually used `here()`, the anchor
/// label is defined at the current cursor and any deferred `LinkAssert`s are
/// drained onto the builder (the linker decides them post-relaxation). A deferred
/// guard NEVER stops lowering (D-H.7: lowering already finished) — only a
/// comptime-exact fatal guard does. Returns `false` only for that comptime abort.
#[allow(clippy::too_many_arguments)] // internal driver; mirrors lower_module's state set
fn lower_item_guard(
    file: &ast::File,
    decl: &ast::EnsureDecl,
    origin: u32,
    module_id: &str,
    here_anchor_counter: &mut u32,
    include_root: Option<&Path>,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) -> bool {
    // Provisional position → mint a candidate anonymous anchor for the guard's
    // `here()` (D-H.8); the label is only DEFINED below if the guard used it.
    let provisional = builder.section_has_relaxable();
    let anchor_name = format!("__here${module_id}${}", *here_anchor_counter);
    let base = origin + builder.current_offset();
    let here = HerePos {
        base,
        anchor: provisional.then(|| anchor_name.clone()),
    };
    let mut outcome = crate::eval::guards::eval_item_guard(file, decl, here, include_root);
    diags.append(&mut outcome.diags);
    if provisional && outcome.anchor_used {
        // Define the anchor at the guard's cursor (its `here()` VMA), advance the
        // counter so the next provisional guard mints a distinct name, and drain
        // the deferred assertions.
        builder.define_label(&anchor_name);
        *here_anchor_counter += 1;
    }
    for a in outcome.link_asserts {
        builder.push_link_assert(a);
    }
    outcome.cont
}

/// Lower one `data` item: evaluate its checked buffer (with `here()` resolving to
/// the item's start VMA = `origin + current_offset`, and `embed`/`import` paths
/// resolving against `placement.include_root`), serialize it in `placement.cpu`'s
/// byte order, then define its label and emit the bytes.
fn lower_data_item(
    file: &ast::File,
    decl: &ast::DataDecl,
    placement: &Placement,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    // A cross-module TYPE-ONLY injection (D-PP.5) carries no bytes — it exists
    // only so the consumer's evaluator learns the item's struct type for
    // `Item.field` field-address operands. Emit nothing (no label, no data); the
    // real item's bytes are emitted once, by its defining module.
    if decl.type_only {
        return;
    }
    let here = here_pos(builder, placement.origin, &decl.name);
    let (buf, asserts, mut ds) =
        eval_data_with_root(file, &decl.name, Some(here), placement.include_root);
    diags.append(&mut ds);
    let Some(buf) = buf else { return };

    let (bytes, fixups, mut stream_diags) = data::stream_data(&buf, placement.cpu, decl.span);
    diags.append(&mut stream_diags);
    builder.define_label(&decl.name);
    builder.emit_data(&bytes, fixups, decl.span);
    // Drain any deferred guards from inside the item's initializer (D-H.4): their
    // anchor is the item's own label, defined just above.
    for a in asserts {
        builder.push_link_assert(a);
    }
}

/// Lower one `offsets` block (Spec 2, Plan 7 backlog #3 — Task 6, the FORWARD
/// direction): evaluate its members to a [`Cell::RelOffset`](crate::value::Cell)
/// per entry, serialize them (each a `dc.w target - base` `RelWord16Be` word),
/// define the table's base label at its first byte, then emit the bytes +
/// fixups. Unlike [`lower_data_item`] there is no `here_base`: a `RelOffset`
/// resolves against the SYMBOLIC base label `decl.name`, folded at link time —
/// not against a physical `here()` position.
///
/// NOTE: [`lower_dispatch_item`] mirrors this function's shape (eval → stream →
/// define base label → emit) — consider both when editing the lowering flow.
fn lower_offsets_item(
    file: &ast::File,
    decl: &ast::OffsetsDecl,
    placement: &Placement,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let (buf, mut ds) = eval_offsets_with_root(file, decl, placement.include_root);
    diags.append(&mut ds);
    let Some(buf) = buf else { return };

    let (bytes, fixups, mut stream_diags) = data::stream_data(&buf, placement.cpu, decl.span);
    diags.append(&mut stream_diags);
    builder.define_label(&decl.name);
    builder.emit_data(&bytes, fixups, decl.span);
}

/// Lower one `dispatch` block's FORWARD emission (Spec 2, Plan 7 backlog #6,
/// Part B — D6.B2). The sibling of [`lower_offsets_item`]: it evaluates the
/// members to a [`DataBuf`] via [`eval_dispatch_with_root`] (RelOffset cells
/// for `word_offsets`, `SymRef` `dc.l`/Abs32 cells for `long_ptrs`), serializes
/// them in `placement.cpu`'s byte order, defines the table's base label
/// (`decl.name`) at its first byte, then emits the bytes + fixups. Dispatch is
/// 68k-only in v1 for BOTH encodings: a `cpu: z80` section is rejected by the
/// `[dispatch.non-68k]` guard below (at the dispatch's own span) before eval.
///
/// 9a: after the table, each `Member: { … }` inline body lowers as an anonymous
/// proc at `__dispatch$<module>$<table>$<member>`, in member order (R9a.1-R9a.4).
fn lower_dispatch_item(
    file: &ast::File,
    decl: &ast::DispatchDecl,
    placement: &Placement,
    as_compat: bool,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
    asm_counter: &mut u32,
) {
    // D6.B1: 68k sections only in v1, mirroring `[offsets.non-68k]`. Guard HERE
    // (at the dispatch's own span, with a dispatch-specific code) rather than
    // rely on the shared `RelOffset` streamer arm, which would report the
    // `offsets`-flavored `[offsets.non-68k]` message.
    if placement.cpu != Cpu::M68000 {
        err(
            diags,
            decl.span,
            "[dispatch.non-68k] a dispatch table is a 68k idiom; \
             Z80 dispatch tables are not supported"
                .to_string(),
        );
        return;
    }

    let (buf, mut ds) = eval_dispatch_with_root(file, decl, placement.include_root);
    diags.append(&mut ds);
    let Some(buf) = buf else { return };

    let (bytes, fixups, mut stream_diags) = data::stream_data(&buf, placement.cpu, decl.span);
    diags.append(&mut stream_diags);
    builder.define_label(&decl.name);
    builder.emit_data(&bytes, fixups, decl.span);

    // 9a (D9.1, R9a.1): inline bodies lower immediately after the table, in
    // member order, as anonymous procs — hygienic label, then the SAME
    // eval_proc_body + lower_code_buf path a named proc takes (D-P4.1). No
    // params / clobbers / falls_into surface (R9a.3): a member needing a proc
    // contract binds a named proc instead.
    for member in &decl.members {
        let ast::DispatchTarget::Body(body) = &member.target else { continue };
        let label = crate::layout::dispatch_body_label(&file.module.path, &decl.name, &member.name);
        builder.define_label(&label);
        let (buf, mut ds, next_counter) =
            eval_proc_body(file, &label, &[], body, member.span, *asm_counter, placement.cpu);
        *asm_counter = next_counter;
        diags.append(&mut ds);
        // `None` = the body failed to EVALUATE (already diagnosed) — skip it.
        // An EMPTY body is `Some(empty buf)` and still reaches the lint below.
        let Some(buf) = buf else { continue };
        lower_code_buf(&buf, placement.cpu, as_compat, builder, diags);
        if !as_compat {
            proc::check_member_body_fallthrough(&decl.name, member, &buf, placement.cpu, diags);
        }
    }
}

/// Read a section's `cpu:`/`vma:` attributes (§7.1). `cpu:` defaults to
/// `M68000` (`z80` selects [`Cpu::Z80`]); `vma:` is evaluated to a comptime
/// integer (defaulting to 0, with a diagnostic if it is not an integer).
/// Unknown attribute names are diagnosed but otherwise ignored.
fn section_attrs(
    file: &ast::File,
    sec: &ast::SectionDecl,
    diags: &mut Vec<Diagnostic>,
) -> (Cpu, u32) {
    let mut cpu = Cpu::M68000;
    let mut vma: u32 = 0;
    for (name, expr) in &sec.attrs {
        match name.as_str() {
            "cpu" => cpu = attr_cpu(expr),
            "vma" => {
                let (n, mut ds) = eval_attr_int(file, expr);
                diags.append(&mut ds);
                match n {
                    Some(v) => vma = v as u32,
                    // Point at the value expression itself (it carries its own
                    // span), not the whole section, for precision.
                    None => err(
                        diags,
                        crate::parser::expr_span(expr),
                        format!("section `{}` `vma:` is not a comptime integer", sec.name),
                    ),
                }
            }
            other => err(
                diags,
                sec.span,
                format!("section `{}` has unknown attribute `{other}`", sec.name),
            ),
        }
    }
    (cpu, vma)
}

/// Resolve a `cpu:` attribute expression to a [`Cpu`]: `z80` (case-insensitive)
/// selects [`Cpu::Z80`]; anything else defaults to [`Cpu::M68000`].
fn attr_cpu(expr: &ast::Expr) -> Cpu {
    if let ast::Expr::Path(p) = expr {
        if p.segments.last().is_some_and(|s| s.eq_ignore_ascii_case("z80")) {
            return Cpu::Z80;
        }
    }
    Cpu::M68000
}

/// Push an error diagnostic at `span`.
fn err(diags: &mut Vec<Diagnostic>, span: Span, message: String) {
    diags.push(Diagnostic { level: Level::Error, message, primary: span });
}

/// Once-per-compile validation of `offsets` blocks. Two hard errors: (1) a
/// duplicate member name makes the reverse-direction ordinal ambiguous
/// (`Name.Variant` resolution silently picks the first match); (2) a member
/// named `count` collides with the reserved `Name.count` pseudo-member (the
/// entry count), which `eval_path` resolves before members, so it would be
/// silently unreachable. Both violate the totality tenet (no silent wrong
/// answers). Reported HERE rather than in the evaluator's `index_items` because
/// that runs per per-item evaluator (once per data item / proc) and would emit
/// the diagnostic N times. Recurses into `section {}` blocks so a
/// section-nested `offsets` is checked exactly like a top-level one (mirroring
/// `index_items`' flat namespace).
///
/// NOTE: [`validate_dispatch`] mirrors this function's shape (reserved-`count`
/// + duplicate-member checks, section recursion) — consider both when editing.
fn validate_offsets(items: &[ast::Item], diags: &mut Vec<Diagnostic>) {
    for item in items {
        match item {
            ast::Item::Offsets(decl) => {
                let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
                for m in &decl.members {
                    // `count` is a reserved pseudo-member: `Name.count` names the
                    // table's entry count in `eval_path`. Reject a real member
                    // named `count` rather than let it be silently unreachable
                    // (the totality tenet — no silent wrong answers).
                    if m.name == "count" {
                        err(
                            diags,
                            m.span,
                            "offset entry `count` is reserved (it names the table's entry count)"
                                .to_string(),
                        );
                    }
                    if !seen.insert(m.name.as_str()) {
                        err(diags, m.span, format!("duplicate offset entry `{}`", m.name));
                    }
                }
            }
            ast::Item::Section(sec) => validate_offsets(&sec.items, diags),
            _ => {}
        }
    }
}

/// Once-per-compile validation of `dispatch` blocks (Spec 2, Plan 7 #6 — D6.B3),
/// mirroring [`validate_offsets`] exactly: (1) a member named `count` collides
/// with the reserved `Name.count` pseudo-member (the member count), which
/// `eval_path` resolves before members, so it would be silently unreachable;
/// (2) a duplicate member name makes the reverse-direction ordinal ambiguous.
/// Both violate the totality tenet (no silent wrong answers). Reported HERE
/// (once per compile) rather than in `index_items` (which re-runs per per-item
/// evaluator). Recurses into `section {}` blocks so a section-nested `dispatch`
/// is checked like a top-level one.
fn validate_dispatch(items: &[ast::Item], diags: &mut Vec<Diagnostic>) {
    for item in items {
        match item {
            ast::Item::Dispatch(decl) => {
                let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
                for m in &decl.members {
                    if m.name == "count" {
                        err(
                            diags,
                            m.span,
                            "dispatch member `count` is reserved (it names the table's member count)"
                                .to_string(),
                        );
                    }
                    if !seen.insert(m.name.as_str()) {
                        err(diags, m.span, format!("duplicate dispatch member `{}`", m.name));
                    }
                }
            }
            ast::Item::Section(sec) => validate_dispatch(&sec.items, diags),
            _ => {}
        }
    }
}
