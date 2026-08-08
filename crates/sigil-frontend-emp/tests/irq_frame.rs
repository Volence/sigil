//! The `irq_frame.pc` intrinsic (bookmark ask 3,
//! `docs/superpowers/2026-08-06-bookmark-implementation-sketch.md` §6): the
//! sanctioned stacked-interrupted-PC accessor an IRQ handler uses to read or
//! rewrite the return PC (the supervisor-bookmark redirect). It lowers to a
//! `(disp, sp)` memory operand whose displacement is DERIVED by the toolchain from
//! the handler's full-save `movem` (`save_bytes + 2`, the +2 being the exception
//! frame's SR word) — no hand-maintained `62(sp)` magic. Two validity rules:
//! `[irqframe.not-handler]` (must be a `grants(...)` proc) and `[irqframe.no-save]`
//! (a `movem …,-(sp)` must precede it); `[irqframe.unknown-field]` for any field
//! but `.pc`. Nuance (a): a handler that REWRITES the PC still satisfies
//! `preserves(d0-a6)` — the write hits the stacked frame (memory), never a
//! register.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    )
}

fn diags(src: &str) -> Vec<Diagnostic> {
    lower(src).1
}

fn has_tag(diags: &[Diagnostic], tag: &str) -> bool {
    diags.iter().any(|d| d.message.contains(tag))
}

fn find<'a>(diags: &'a [Diagnostic], tag: &str) -> &'a Diagnostic {
    diags.iter().find(|d| d.message.contains(tag)).unwrap_or_else(|| panic!("expected `{tag}`, got {diags:?}"))
}

fn flatten(module: &Module) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// A well-formed VBlank handler that reads the stacked PC: full save, read
/// `irq_frame.pc`, restore, `rte`.
const HANDLER: &str = "module m\n\
context vblank { granted }\n\
pub proc VBlank_Handler () clobbers(d0-d7/a0-a6) grants(vblank) {\n\
    movem.l d0-a6, -(sp)\n\
    move.l irq_frame.pc, d0\n\
    movem.l (sp)+, d0-a6\n\
    rte\n\
}\n";

// ---- the clean handler resolves and is silent ------------------------------

#[test]
fn a_clean_handler_read_is_silent() {
    let (_m, ds) = lower(HANDLER);
    assert!(ds.is_empty(), "clean handler read should be silent: {ds:?}");
}

/// The intrinsic lowers to EXACTLY `62(sp)` for a `movem.l d0-a6` (15 regs) save:
/// its emitted bytes equal the hand-written `62(sp)` form's.
#[test]
fn read_lowers_to_the_derived_displacement() {
    let (intrinsic, _) = lower(HANDLER);
    let (manual, _) = lower(&HANDLER.replace("irq_frame.pc", "62(sp)"));
    assert_eq!(
        flatten(&intrinsic),
        flatten(&manual),
        "irq_frame.pc must lower to 62(sp) for a movem.l d0-a6 save"
    );
}

/// The displacement is DERIVED from the actual save set: a smaller save
/// (`movem.l d0-d3,-(sp)` = 4 longs = 16 bytes) yields `18(sp)`, not a fixed 62.
#[test]
fn displacement_tracks_the_save_set() {
    let src = "module m\n\
context vblank { granted }\n\
pub proc H () clobbers(d0-d3) grants(vblank) {\n\
    movem.l d0-d3, -(sp)\n\
    move.l irq_frame.pc, d0\n\
    movem.l (sp)+, d0-d3\n\
    rte\n\
}\n";
    let (intrinsic, ds) = lower(src);
    assert!(ds.is_empty(), "clean: {ds:?}");
    let (manual, _) = lower(&src.replace("irq_frame.pc", "18(sp)"));
    assert_eq!(flatten(&intrinsic), flatten(&manual), "4-long save ⇒ 16+2 = 18(sp)");
    // And it is NOT the d0-a6 value (guards against a hard-coded 62).
    let (fixed62, _) = lower(&src.replace("irq_frame.pc", "62(sp)"));
    assert_ne!(flatten(&intrinsic), flatten(&fixed62), "must derive, not hard-code 62");
}

// ---- nuance (a): a PC rewrite still satisfies preserves(d0-a6) --------------

#[test]
fn a_pc_rewrite_preserves_the_registers() {
    // The handler REWRITES the stacked PC (the bookmark redirect) and declares it
    // preserves every register. The write hits the stacked frame (memory), never a
    // register, so the movem round-trip proof holds and stack-balance sees no delta.
    let src = "module m\n\
context vblank { granted }\n\
pub proc VBlank_Handler () clobbers() preserves(d0-d7/a0-a6) grants(vblank) {\n\
    movem.l d0-a6, -(sp)\n\
    move.l #$00FF0000, irq_frame.pc\n\
    movem.l (sp)+, d0-a6\n\
    rte\n\
}\n";
    let ds = diags(src);
    assert!(!has_tag(&ds, "[preserves"), "a PC rewrite must not break preserves: {ds:?}");
    assert!(!has_tag(&ds, "[stack."), "a PC rewrite is not a stack delta: {ds:?}");
    assert!(!has_tag(&ds, "[proc.clobber"), "a memory write is not a register clobber: {ds:?}");
    assert!(ds.iter().all(|d| d.level != Level::Error), "the redirect handler must lower clean: {ds:?}");
}

// ---- validity rule: handler context ----------------------------------------

#[test]
fn not_a_handler_is_fatal() {
    // Same body, but the proc has no `grants(...)` — an ordinary proc.
    let src = "module m\n\
pub proc NotAHandler () clobbers(d0-d7/a0-a6) {\n\
    movem.l d0-a6, -(sp)\n\
    move.l irq_frame.pc, d0\n\
    movem.l (sp)+, d0-a6\n\
    rts\n\
}\n";
    let ds = diags(src);
    let d = find(&ds, "[irqframe.not-handler]");
    assert_eq!(d.level, Level::Error);
}

// ---- validity rule: a prior full-save movem --------------------------------

#[test]
fn no_prior_save_is_fatal() {
    let src = "module m\n\
context vblank { granted }\n\
pub proc H () clobbers(d0) grants(vblank) {\n\
    move.l irq_frame.pc, d0\n\
    rte\n\
}\n";
    let ds = diags(src);
    let d = find(&ds, "[irqframe.no-save]");
    assert_eq!(d.level, Level::Error);
}

/// A save-then-RESTORE clears the anchor: an accessor AFTER the restore has no
/// live frame and is `[irqframe.no-save]` (the exception frame's registers are
/// already popped, so `disp(sp)` no longer addresses the PC).
#[test]
fn an_accessor_after_the_restore_is_fatal() {
    let src = "module m\n\
context vblank { granted }\n\
pub proc H () clobbers(d0) grants(vblank) {\n\
    movem.l d0-a6, -(sp)\n\
    movem.l (sp)+, d0-a6\n\
    move.l irq_frame.pc, d0\n\
    rte\n\
}\n";
    assert!(has_tag(&diags(src), "[irqframe.no-save]"), "restore clears the frame anchor");
}

// ---- validity rule: only `.pc` ---------------------------------------------

#[test]
fn an_unknown_field_is_fatal() {
    let src = "module m\n\
context vblank { granted }\n\
pub proc H () clobbers(d0) grants(vblank) {\n\
    movem.l d0-a6, -(sp)\n\
    move.l irq_frame.sr, d0\n\
    movem.l (sp)+, d0-a6\n\
    rte\n\
}\n";
    let ds = diags(src);
    let d = find(&ds, "[irqframe.unknown-field]");
    assert_eq!(d.level, Level::Error);
}

// ---- inertness: `irq_frame` is the ONLY reserved prefix --------------------

#[test]
fn an_ordinary_two_segment_symbol_is_untouched() {
    // `Owner.label` cross-body references (the existing two-segment path) still
    // resolve — only `irq_frame.*` is intercepted. `Other.label` here is a plain
    // link symbol, no `[irqframe.*]` anywhere.
    let src = "module m\n\
pub proc P () clobbers(d0) {\n\
    lea Other.label, a0\n\
    rts\n\
}\n";
    assert!(!has_tag(&diags(src), "[irqframe."), "only irq_frame.* is reserved");
}
