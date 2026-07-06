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

use crate::ast;
use crate::layout::{eval_attr_int, eval_data_with_root, eval_offsets_with_root};
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

    // The instantiation counter for `asm { }` / `proc` label hygiene (D-P4.6),
    // threaded across EVERY proc in the module (top-level AND section-nested) so
    // `k` stays globally monotonic. Lowering builds a fresh evaluator per proc,
    // which would otherwise restart `k` at 0 each proc and mint colliding
    // `$asm0…` symbols for `asm {}` bodies generated in different procs. Each
    // proc seeds from this value and hands back the advanced one.
    let mut asm_counter: u32 = 0;

    for (index, item) in file.items.iter().enumerate() {
        match item {
            ast::Item::Data(decl) => {
                ensure_default(&mut builder, &mut next_lma, &mut default_open, opts.initial_cpu);
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
                ensure_default(&mut builder, &mut next_lma, &mut default_open, opts.initial_cpu);
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
                ensure_default(&mut builder, &mut next_lma, &mut default_open, opts.initial_cpu);
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
            ast::Item::Section(sec) => {
                // Close whatever is open (default text or a prior adjacent
                // section), folding its length into the counter.
                next_lma += builder.current_offset();
                default_open = false;
                let (cpu, vma) = section_attrs(file, sec, &mut diags);
                builder.switch_section_lma(&sec.name, cpu, Some(vma), next_lma);
                lower_section_items(
                    file,
                    sec,
                    &Placement { cpu, origin: vma, include_root: opts.include_root.as_deref() },
                    as_compat,
                    &mut builder,
                    &mut diags,
                    &mut asm_counter,
                );
                // Leave the named section open; the next item (or `finish`)
                // folds its length.
            }
            _ => {}
        }
    }

    let (module, mut build_diags) = builder.finish();
    diags.append(&mut build_diags);
    (module, diags)
}

/// Ensure the default `text` section is the currently-open one before lowering a
/// top-level item. If a named section (or nothing) is open, fold its length into
/// `next_lma` and open `text` at that physical offset (VMA==LMA). A no-op when
/// the default is already open.
fn ensure_default(
    builder: &mut IrBuilder,
    next_lma: &mut u32,
    default_open: &mut bool,
    cpu: Cpu,
) {
    if !*default_open {
        *next_lma += builder.current_offset();
        builder.switch_section_lma("text", cpu, None, *next_lma);
        *default_open = true;
    }
}

/// Lower the items nested inside a `section {}` block into the already-open
/// section. `placement.cpu` is the section's CPU (drives byte order + code
/// lowering); `placement.origin` is its VMA base, used to compute `here()` for
/// each data item; `placement.include_root` is the sandbox root threaded to
/// every data item's `embed`/`import`.
fn lower_section_items(
    file: &ast::File,
    sec: &ast::SectionDecl,
    placement: &Placement,
    as_compat: bool,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
    asm_counter: &mut u32,
) {
    for (index, item) in sec.items.iter().enumerate() {
        match item {
            ast::Item::Data(decl) => {
                lower_data_item(file, decl, placement, builder, diags);
            }
            ast::Item::Offsets(decl) => {
                lower_offsets_item(file, decl, placement, builder, diags);
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
            _ => {}
        }
    }
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
    let here_base = placement.origin + builder.current_offset();
    let (buf, mut ds) =
        eval_data_with_root(file, &decl.name, Some(here_base), placement.include_root);
    diags.append(&mut ds);
    let Some(buf) = buf else { return };

    let (bytes, fixups, mut stream_diags) = data::stream_data(&buf, placement.cpu, decl.span);
    diags.append(&mut stream_diags);
    builder.define_label(&decl.name);
    builder.emit_data(&bytes, fixups, decl.span);
}

/// Lower one `offsets` block (Spec 2, Plan 7 backlog #3 — Task 6, the FORWARD
/// direction): evaluate its members to a [`Cell::RelOffset`](crate::value::Cell)
/// per entry, serialize them (each a `dc.w target - base` `RelWord16Be` word),
/// define the table's base label at its first byte, then emit the bytes +
/// fixups. Unlike [`lower_data_item`] there is no `here_base`: a `RelOffset`
/// resolves against the SYMBOLIC base label `decl.name`, folded at link time —
/// not against a physical `here()` position.
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
