//! The `@continuation` proc form (bookmark ask 4,
//! `docs/superpowers/2026-08-06-bookmark-implementation-sketch.md` §6): the
//! MANUFACTURED-FRAME license for the supervisor bookmark's re-entry primitives.
//! A `@continuation` proc is entered by a manufactured or hardware control
//! transfer (`rte`/`jmp`), not a call; its register-state set is DECLARED
//! (trusted), not proven. It is the trust root that licenses the two code shapes
//! that fit no ordinary proc form:
//!
//!   (a) an `rte`-entered register-banking stub that exits by a bare `rts` into
//!       its grand-caller's frame (PageIn_BankRegs);
//!   (b) a `jmp`-entered resume that manufactures a frame and `rte`s to it —
//!       `move.w <sr>,-(sp)` / `move.l <pc>,-(sp)` / `rte` (the shape stack-balance
//!       rejects as `[stack.unbalanced]` in an ordinary proc).
//!
//! The license is the stack-balance exemption; its two guards are
//! `[continuation.contract-required]` (a declared `clobbers(...)`) and
//! `[continuation.z80-unsupported]` (68k only). Opt-in and syntactically explicit,
//! and check-neutral over every proc that does not carry it.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level};

fn diags68(src: &str) -> Vec<Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    )
    .1
}

fn parse_diags(src: &str) -> Vec<Diagnostic> {
    parse_str(src).1
}

fn has_tag(diags: &[Diagnostic], tag: &str) -> bool {
    diags.iter().any(|d| d.message.contains(tag))
}

fn find<'a>(diags: &'a [Diagnostic], tag: &str) -> &'a Diagnostic {
    diags.iter().find(|d| d.message.contains(tag)).unwrap_or_else(|| panic!("expected `{tag}`, got {diags:?}"))
}

// ---- shape (a): the rte-entered banking stub, bare rts --------------------

#[test]
fn a_banking_stub_continuation_is_clean() {
    // PageIn_BankRegs: entered by the hijacked handler `rte`, banks the live
    // decoder registers to RAM, `rts` into VSync_Wait's frame.
    let src = "module m\n\
@continuation\n\
pub proc PageIn_BankRegs (a5: *u8) clobbers() {\n\
    move.w sr, (a5)\n\
    movem.l d0-d2/a0-a3, (a5)\n\
    rts\n\
}\n";
    let ds = diags68(src);
    assert!(ds.is_empty(), "a continuation banking stub should be silent: {ds:?}");
}

// ---- shape (b): the jmp-entered manufactured resume ------------------------

#[test]
fn a_manufactured_resume_continuation_is_clean() {
    // The resume: manufacture the suspended decoder's frame and rte to it. In an
    // ORDINARY proc this is `[stack.unbalanced]` (see the negative test below);
    // `@continuation` licenses it.
    let src = "module m\n\
@continuation\n\
pub proc PageIn_Resume () clobbers(d0-d2/a0-a3) {\n\
    move.w Saved_Sr, -(sp)\n\
    move.l Saved_Pc, -(sp)\n\
    rte\n\
}\n";
    let ds = diags68(src);
    assert!(!has_tag(&ds, "[stack."), "the manufactured rte must be licensed: {ds:?}");
    assert!(ds.iter().all(|d| d.level != Level::Error), "the resume continuation must lower clean: {ds:?}");
}

/// The negative control: the SAME manufactured-resume body WITHOUT `@continuation`
/// still fires `[stack.unbalanced]`. The license is opt-in; it does not weaken the
/// guarantee for ordinary procs.
#[test]
fn the_same_body_without_the_attribute_still_fires() {
    let src = "module m\n\
pub proc PageIn_Resume () clobbers(d0-d2/a0-a3) {\n\
    move.w Saved_Sr, -(sp)\n\
    move.l Saved_Pc, -(sp)\n\
    rte\n\
}\n";
    assert!(
        has_tag(&diags68(src), "[stack.unbalanced]"),
        "an ORDINARY proc's manufactured rte must still be flagged"
    );
}

// ---- guard: a declared clobbers set is required ----------------------------

#[test]
fn continuation_requires_a_declared_clobbers_set() {
    let src = "module m\n\
@continuation\n\
pub proc C () {\n\
    rts\n\
}\n";
    let ds = diags68(src);
    let d = find(&ds, "[continuation.contract-required]");
    assert_eq!(d.level, Level::Error);
}

// ---- guard: 68k only -------------------------------------------------------

#[test]
fn continuation_on_z80_is_rejected() {
    let src = "module m (cpu: z80)\n\
@continuation\n\
pub proc C () clobbers() {\n\
    ret\n\
}\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let ds = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::Z80, include_root: None, embed_base: None, defines: vec![] },
    )
    .1;
    let d = find(&ds, "[continuation.z80-unsupported]");
    assert_eq!(d.level, Level::Error);
}

// ---- parse-form guard ------------------------------------------------------

#[test]
fn continuation_takes_no_arguments() {
    let ds = parse_diags("module m\n@continuation(d0)\npub proc C () clobbers() { rts }\n");
    assert!(has_tag(&ds, "[attr.form]"), "@continuation takes no args: {ds:?}");
}

#[test]
fn continuation_is_a_known_attribute() {
    let ds = parse_diags("module m\n@continuation\npub proc C () clobbers() { rts }\n");
    assert!(!has_tag(&ds, "[attr.unknown]"), "@continuation must be known: {ds:?}");
}

// ---- the license is exemption-scoped, not a blanket silence ----------------

/// A `@continuation` proc still runs every OTHER check: an undeclared register
/// write is still `[proc.clobber-undeclared]` (the exemption is stack-balance
/// only, not a free pass).
#[test]
fn continuation_still_runs_the_clobber_lint() {
    let src = "module m\n\
@continuation\n\
pub proc C () clobbers(d0) {\n\
    moveq #0, d0\n\
    moveq #0, d4\n\
    rts\n\
}\n";
    assert!(
        has_tag(&diags68(src), "[proc.clobber-undeclared]"),
        "the clobber lint still applies inside a continuation"
    );
}
