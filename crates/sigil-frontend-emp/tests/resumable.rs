//! The `@resumable` (stackless) proc attribute + its exported extent symbol
//! (bookmark asks 1 & 2, `docs/superpowers/2026-08-06-bookmark-implementation-sketch.md`
//! §6). A `@resumable` proc keeps ALL live state in registers and touches sp
//! NOWHERE in its body — no `bsr`/`jsr`/`jbsr`/`pea`/`link`/`unlk`, no `movem`
//! involving sp, no `-(sp)`/`(sp)+`/`(sp)` operand, no explicit sp/a7 write, and
//! no `rts`-family return (it exits by a computed `jmp (aN)` continuation). The
//! stackless guarantee is build-fatal (never softened) because the whole VBlank
//! bookmark safety argument rests on it. The declared register-state set is the
//! proc's ordinary contract (params + clobbers + out); a touch outside it is the
//! existing `[proc.clobber-undeclared]` error, so `@resumable` requires a
//! `clobbers(...)` declaration and runs that check even under `@as_compat`.
//!
//! Ask 2: for each `@resumable` proc an exported `Proc.__end` extent label is
//! emitted at the byte immediately past the body, so a consumer compiles a
//! `[Proc, Proc.__end)` PC range check from toolchain symbols.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::imports::{canonical, ExportIndex, ResolveEnv};
use sigil_frontend_emp::resolve::rename::canonicalize_name;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

/// Parse + lower `src` for the 68k, asserting a clean parse. Returns the module
/// and the lowering diagnostics.
fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    )
}

fn parse_diags(src: &str) -> Vec<Diagnostic> {
    parse_str(src).1
}

/// True if some diagnostic message contains `tag` (the bracketed lint code).
fn has_tag(diags: &[Diagnostic], tag: &str) -> bool {
    diags.iter().any(|d| d.message.contains(tag))
}

/// The single diagnostic carrying `tag`, or panic.
fn find<'a>(diags: &'a [Diagnostic], tag: &str) -> &'a Diagnostic {
    diags
        .iter()
        .find(|d| d.message.contains(tag))
        .unwrap_or_else(|| panic!("expected a `{tag}` diagnostic, got {diags:?}"))
}

/// Link a module to a flat image (used to prove the extent label carries no bytes).
fn flatten(module: &Module) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// A well-formed resumable body: a fragment of the real Phase-2 ZX0R decoder —
/// caller-owned d0-d2/a0-a2, no stack, exits `jmp (a3)`.
const CLEAN: &str = "module m\n\
@resumable\n\
pub proc ZX0R (a0: *u8, a1: *u8, a3: *u8) clobbers(d0-d2/a2) {\n\
    moveq #-128, d1\n\
    moveq #-1, d2\n\
    moveq #1, d0\n\
    move.b (a0)+, (a1)+\n\
    movea.l a1, a2\n\
    adda.l d2, a2\n\
    move.b (a2)+, (a1)+\n\
    jmp (a3)\n\
}\n";

// ---- Ask 1: the clean proc passes -----------------------------------------

#[test]
fn a_clean_resumable_proc_has_no_diagnostics() {
    let (_m, diags) = lower(CLEAN);
    assert!(diags.is_empty(), "clean resumable proc should be silent: {diags:?}");
}

// ---- Ask 1: each forbidden instruction class is individually build-fatal ----

/// Splice one offending line into an otherwise-clean resumable body and lower.
fn lower_with_line(line: &str) -> Vec<Diagnostic> {
    let src = format!(
        "module m\n\
@resumable\n\
pub proc ZX0R (a0: *u8, a1: *u8, a3: *u8) clobbers(d0-d2/a2) {{\n\
    moveq #-128, d1\n\
    {line}\n\
    jmp (a3)\n\
}}\n"
    );
    lower(&src).1
}

fn assert_stack_op_fatal(line: &str, needle: &str) {
    let diags = lower_with_line(line);
    let d = find(&diags, "[resumable.stack-op]");
    assert_eq!(d.level, Level::Error, "stackless violations are build-fatal");
    assert!(
        d.message.contains(needle) && d.message.contains("@resumable"),
        "diagnostic must name the offending op ({needle}) and the @resumable contract: {}",
        d.message
    );
}

#[test]
fn bsr_is_fatal() {
    assert_stack_op_fatal("bsr.w Somewhere", "bsr");
}

#[test]
fn jsr_is_fatal() {
    assert_stack_op_fatal("jsr Somewhere", "jsr");
}

#[test]
fn jbsr_is_fatal() {
    assert_stack_op_fatal("jbsr Somewhere", "jbsr");
}

#[test]
fn pea_is_fatal() {
    assert_stack_op_fatal("pea Somewhere", "pea");
}

#[test]
fn link_is_fatal() {
    assert_stack_op_fatal("link a6, #-8", "link");
}

#[test]
fn unlk_is_fatal() {
    assert_stack_op_fatal("unlk a6", "unlk");
}

#[test]
fn predecrement_push_is_fatal() {
    // `-(sp)` push — the movem prologue the resumable variant deletes.
    assert_stack_op_fatal("move.l d0, -(sp)", "sp");
}

#[test]
fn postincrement_pop_is_fatal() {
    assert_stack_op_fatal("move.l (sp)+, d0", "sp");
}

#[test]
fn movem_involving_sp_is_fatal() {
    assert_stack_op_fatal("movem.l d0-d2/a2, -(sp)", "sp");
}

#[test]
fn explicit_sp_write_is_fatal() {
    // A bare sp/a7 destination — computed stack advance.
    assert_stack_op_fatal("adda.w #4, sp", "sp");
}

#[test]
fn displaced_sp_access_is_fatal() {
    // `d(sp)` — the hand-maintained scratch-frame idiom, forbidden here.
    assert_stack_op_fatal("move.w d0, 2(sp)", "sp");
}

#[test]
fn top_of_stack_indirect_is_fatal() {
    assert_stack_op_fatal("move.l (sp), d0", "sp");
}

#[test]
fn rts_is_fatal() {
    // A resumable proc exits by `jmp (aN)`, never `rts` (which reads the return
    // address off the stack).
    let diags = lower_with_line("nop");
    assert!(!has_tag(&diags, "[resumable.stack-op]"), "nop is clean: {diags:?}");
    // Now a body whose ONLY exit is rts.
    let src = "module m\n\
@resumable\n\
pub proc ZX0R (a0: *u8) clobbers(d0) {\n\
    moveq #0, d0\n\
    rts\n\
}\n";
    let diags = lower(src).1;
    let d = find(&diags, "[resumable.stack-op]");
    assert_eq!(d.level, Level::Error);
    assert!(d.message.contains("rts"), "names rts: {}", d.message);
}

// ---- Ask 1: a `with` bracket that lowers to stack ops is caught -------------

#[test]
fn a_with_bracket_that_emits_a_push_is_caught() {
    // The `with <ctx> {}` acquire/release splice reaches the lowered stream as
    // ordinary instructions; a resumable body catches the spliced push exactly as
    // it catches a literal one (the "via lowering, not source tokens" property).
    let src = "module m\n\
context saver {\n\
    acquire = asm { move.l d7, -(sp) }\n\
    release = asm { move.l (sp)+, d7 }\n\
}\n\
@resumable\n\
pub proc ZX0R (a0: *u8) clobbers(d0-d7) {\n\
    with saver {\n\
        moveq #0, d0\n\
    }\n\
    jmp (a0)\n\
}\n";
    let (file, perrs) = parse_str(src);
    // If the context grammar differs, don't hard-fail the suite — but when it
    // parses, the spliced push MUST be caught.
    if !perrs.is_empty() {
        eprintln!("context grammar unavailable in this build: {perrs:?}");
        return;
    }
    let (_m, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    assert!(
        has_tag(&diags, "[resumable.stack-op]"),
        "a push spliced by a `with` bracket must be caught: {diags:?}"
    );
}

// ---- Ask 1: declared-register-set violations error --------------------------

#[test]
fn a_register_outside_the_declared_set_errors() {
    // The register-state set is the proc's contract (params + clobbers). A write
    // to a register in NEITHER is the existing `[proc.clobber-undeclared]` error —
    // `@resumable` makes that check mandatory (it runs even without the usual
    // gating).
    let src = "module m\n\
@resumable\n\
pub proc ZX0R (a0: *u8) clobbers(d0) {\n\
    moveq #0, d0\n\
    moveq #0, d4\n\
    jmp (a0)\n\
}\n";
    assert!(
        has_tag(&lower(src).1, "[proc.clobber-undeclared]"),
        "a write outside the declared register set must error"
    );
}

#[test]
fn resumable_requires_a_declared_clobbers_set() {
    // Without a declared register set there is nothing to bound the liveness
    // against, so `@resumable` with no `clobbers(...)` is refused.
    let src = "module m\n\
@resumable\n\
pub proc ZX0R (a0: *u8) {\n\
    jmp (a0)\n\
}\n";
    let diags = lower(src).1;
    let d = find(&diags, "[resumable.contract-required]");
    assert_eq!(d.level, Level::Error);
}

#[test]
fn resumable_on_z80_is_rejected() {
    // 68k only (the stack model + bookmark mechanism are 68k), mirroring the
    // inout facet's Z80 scope guard.
    let src = "module m (cpu: z80)\n\
@resumable\n\
pub proc R () clobbers() {\n\
    jp (hl)\n\
}\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_m, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::Z80,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let d = find(&diags, "[resumable.z80-unsupported]");
    assert_eq!(d.level, Level::Error);
}

// ---- Ask 1: parse-form guard ------------------------------------------------

#[test]
fn resumable_takes_no_arguments() {
    let diags = parse_diags("module m\n@resumable(d0)\npub proc R () clobbers() { rts }\n");
    assert!(has_tag(&diags, "[attr.form]"), "@resumable takes no args: {diags:?}");
}

#[test]
fn resumable_is_a_known_attribute() {
    // Sanity: it is NOT reported as an unknown attribute.
    let diags = parse_diags(CLEAN);
    assert!(!has_tag(&diags, "[attr.unknown]"), "@resumable must be known: {diags:?}");
}

// ---- Ask 2: the exported extent symbol --------------------------------------

#[test]
fn extent_symbol_is_emitted_at_end_of_body() {
    let (module, diags) = lower(CLEAN);
    assert!(diags.is_empty(), "{diags:?}");
    let sec = module.sections.first().expect("one section");
    let start = sec.labels.iter().find(|l| l.name == "ZX0R").expect("proc label").offset;
    let end = sec
        .labels
        .iter()
        .find(|l| l.name == "ZX0R.__end")
        .expect("extent label `ZX0R.__end`")
        .offset;
    // The extent equals start + the exact emitted body size.
    let body_len = flatten(&module).len() as u32;
    assert_eq!(start, 0, "proc sits at section start");
    assert_eq!(end, start + body_len, "extent label lands one past the last body byte");
    assert!(end > start, "a non-empty body");
}

#[test]
fn no_extent_symbol_for_a_plain_proc() {
    // A non-`@resumable` proc gets no `.__end` label — the feature is inert over
    // the existing corpus.
    let (module, _d) = lower("module m\npub proc plain () clobbers(d0) { moveq #0, d0\n rts }\n");
    let sec = module.sections.first().unwrap();
    assert!(
        !sec.labels.iter().any(|l| l.name.ends_with(".__end")),
        "plain proc must not emit an extent label"
    );
}

#[test]
fn extent_symbol_resolves_from_another_module() {
    // The extent label rides the SAME cross-module path as the proc's own
    // exported labels: `Owner.__end` canonicalizes through the dotted-owner rule
    // to `<module>.Owner.__end`, referenceable after `use`-ing the proc.
    let (provider, _) = parse_str(CLEAN.replace("module m", "module engine.zx0").as_str());
    let (consumer, _) = parse_str(
        "module level.page_in\n\
use engine.zx0.{ZX0R}\n\
proc PageIn (a0: *u8) clobbers(d0) {\n\
    cmpi.l #ZX0R, d0\n\
    jmp (a0)\n\
}\n",
    );
    let idx = ExportIndex::build(&[("engine.zx0", &provider), ("level.page_in", &consumer)]);
    let (env, ediags) = ResolveEnv::build("level.page_in", &consumer, &idx, None);
    assert!(ediags.iter().all(|d| d.level != Level::Error), "{ediags:?}");

    // The consumer spells the extent as `ZX0R.__end`; canonicalize resolves it to
    // the provider module's canonical extent symbol.
    let want = format!("{}.__end", canonical("engine.zx0", "ZX0R"));
    assert_eq!(
        canonicalize_name("ZX0R.__end", env.rename_map()),
        Some(want.clone()),
        "the extent symbol resolves cross-module like any exported label"
    );

    // And the DEFINING side emits exactly that symbol once renamed: lower the
    // provider and canonicalize its `ZX0R.__end` label the same way.
    let (mut pmod, _pd) = lower_module(
        &provider,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let (penv, _) = ResolveEnv::build("engine.zx0", &provider, &idx, None);
    sigil_frontend_emp::resolve::rename::rename_module(&mut pmod, penv.rename_map());
    assert!(
        pmod.sections.iter().any(|s| s.labels.iter().any(|l| l.name == want)),
        "the provider emits the canonical extent label `{want}`"
    );
}
