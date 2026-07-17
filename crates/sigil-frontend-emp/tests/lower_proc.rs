//! T4 (Plan 4) — `proc` lowering. A `proc` lowers to a label named after the
//! proc plus its body, run through the SAME `eval_asm` → `lower_code_buf` path
//! `asm { }` uses (no instruction lowering is re-implemented). This exercises
//! the byte-exact body emission, the label placement, and the three §5.1
//! proc-contract diagnostics: declared-fallthrough adjacency
//! (`[proc.fallthrough-separated]`), undeclared fallthrough
//! (`[proc.undeclared-fallthrough]`), and the clobbers lint
//! (`[proc.clobber-undeclared]`).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

/// Parse + lower `src` to a `Module` for the 68k, asserting the source parsed
/// cleanly. Returns the module and the lowering diagnostics.
fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] })
}

/// Link a lowered `Module` to a flat image (mirrors T0/T2/T3 link helpers).
fn flatten(module: &Module) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// True if any diagnostic message contains `tag` (the bracketed lint code).
fn has_tag(diags: &[Diagnostic], tag: &str) -> bool {
    diags.iter().any(|d| d.message.contains(tag))
}

#[test]
fn proc_emits_label_and_body() {
    // `proc foo() { moveq #0, d0  rts }` → label `foo` at offset 0 plus the exact
    // encoded bytes: moveq #0,d0 = 70 00 (golden), rts = 4E 75 (golden).
    let (module, diags) = lower("module m\nproc foo() {\n    moveq #0, d0\n    rts\n}\n");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    let section = module.sections.first().expect("one section");
    let foo = section.labels.iter().find(|l| l.name == "foo").expect("`foo` label");
    assert_eq!(foo.offset, 0, "proc label sits at the start of its body");

    assert_eq!(flatten(&module), vec![0x70, 0x00, 0x4E, 0x75]);
}

#[test]
fn falls_into_adjacent_ok() {
    // `proc a falls_into b` immediately followed by `proc b` — physically
    // adjacent, so NO `[proc.fallthrough-separated]` (and no undeclared-fallthrough
    // warning for `a`, since it declares the fall).
    let src = "module m\n\
               proc a() falls_into b {\n    moveq #0, d0\n}\n\
               proc b() {\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.fallthrough-separated]"),
        "adjacent falls_into must not be flagged: {diags:?}"
    );
    // Declaring `falls_into` also suppresses the undeclared-fallthrough warning
    // for `a`, even though its body ends without a terminator.
    assert!(
        !has_tag(&diags, "[proc.undeclared-fallthrough]"),
        "a declared fall must suppress the undeclared-fallthrough warning: {diags:?}"
    );
}

#[test]
fn falls_into_separated_errors() {
    // `proc a falls_into b` with another proc between `a` and `b` — the fall
    // cannot happen, so `[proc.fallthrough-separated]` (an error) naming both.
    let src = "module m\n\
               proc a() falls_into b {\n    moveq #0, d0\n}\n\
               proc middle() {\n    rts\n}\n\
               proc b() {\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let sep = diags
        .iter()
        .find(|d| d.message.contains("[proc.fallthrough-separated]"))
        .expect("expected a fallthrough-separated diagnostic");
    assert_eq!(sep.level, Level::Error);
    assert!(sep.message.contains('a') && sep.message.contains('b'), "names both procs");
}

#[test]
fn undeclared_fallthrough_warns() {
    // A proc whose body ends WITHOUT a terminator and does not declare
    // `falls_into` → `[proc.undeclared-fallthrough]` warning.
    let (_module, diags) = lower("module m\nproc p() {\n    moveq #0, d0\n}\n");
    let w = diags
        .iter()
        .find(|d| d.message.contains("[proc.undeclared-fallthrough]"))
        .expect("expected an undeclared-fallthrough diagnostic");
    assert_eq!(w.level, Level::Warning);
}

#[test]
fn as_compat_silences_undeclared_fallthrough() {
    // Spec 2 · Plan 6 (D-P6.3): a module-level `@as_compat` marks a faithful port
    // and silences the modernization / faithful-port lints. The SAME proc that
    // warns above (undeclared fallthrough) emits NO such warning under
    // `@as_compat`.
    let (_module, diags) =
        lower("module m\n@as_compat\nproc p() {\n    moveq #0, d0\n}\n");
    assert!(
        !has_tag(&diags, "[proc.undeclared-fallthrough]"),
        "@as_compat must silence the undeclared-fallthrough lint: {diags:?}"
    );
}

#[test]
fn as_compat_silences_clobber_undeclared() {
    // Companion: `@as_compat` also silences the heuristic clobber lint. The same
    // `move.l d2, d3` under `clobbers(d0, d1)` that warns above stays quiet here.
    let src = "module m\n@as_compat\nproc p() clobbers(d0, d1) {\n    move.l d2, d3\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "@as_compat must silence the clobber-undeclared lint: {diags:?}"
    );
}

#[test]
fn as_compat_does_not_silence_hard_fallthrough_error() {
    // `@as_compat` silences WARNING-level modernization lints, never a hard error.
    // A broken `falls_into` (target not the immediately-following proc) is a
    // correctness ERROR (`[proc.fallthrough-separated]`) and must still fire.
    let src = "module m\n@as_compat\n\
               proc a() falls_into c {\n    moveq #0, d0\n}\n\
               proc b() {\n    rts\n}\n\
               proc c() {\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let sep = diags
        .iter()
        .find(|d| d.message.contains("[proc.fallthrough-separated]"))
        .expect("a hard fallthrough-separated error must survive @as_compat");
    assert_eq!(sep.level, Level::Error);
}

#[test]
fn empty_proc_body_warns_fallthrough() {
    // An empty body has no terminating instruction, so it falls through → the
    // undeclared-fallthrough warning fires (pins the documented behavior).
    let (_module, diags) = lower("module m\nproc p() {\n}\n");
    assert!(
        has_tag(&diags, "[proc.undeclared-fallthrough]"),
        "an empty proc body must warn about fallthrough: {diags:?}"
    );
}

#[test]
fn terminated_proc_does_not_warn_fallthrough() {
    // Companion: a proc ending in `rts` terminates straight-line flow → NO
    // undeclared-fallthrough warning.
    let (_module, diags) = lower("module m\nproc p() {\n    moveq #0, d0\n    rts\n}\n");
    assert!(
        !has_tag(&diags, "[proc.undeclared-fallthrough]"),
        "a proc ending in rts must not warn: {diags:?}"
    );
}

#[test]
fn clobber_undeclared_warns() {
    // `move.l d2, d3` writes d3 (the destination) under `clobbers(d0, d1)` — d3 is
    // neither declared nor a param → `[proc.clobber-undeclared]` naming it.
    let src = "module m\nproc p() clobbers(d0, d1) {\n    move.l d2, d3\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let w = diags
        .iter()
        .find(|d| d.message.contains("[proc.clobber-undeclared]"))
        .expect("expected a clobber-undeclared diagnostic");
    assert_eq!(w.level, Level::Warning);
    assert!(w.message.contains("d3"), "names the undeclared destination register: {}", w.message);
}

#[test]
fn clobbers_reglist_range_expands_for_the_lint() {
    // C1 item 2: `clobbers(d0-d3/a1)` is the movem-reglist grammar. A write to
    // a register INSIDE the range (d2) is allowed (no undeclared warning); a
    // write OUTSIDE it (d4) is still `[proc.clobber-undeclared]`.
    let src = "module m\nproc p() clobbers(d0-d3/a1) {\n    move.l d5, d2\n    move.l d5, d4\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let undeclared: Vec<&str> = diags
        .iter()
        .filter(|d| d.message.contains("[proc.clobber-undeclared]"))
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        undeclared.iter().any(|m| m.contains("d4")),
        "d4 (outside the range) must warn: {diags:?}"
    );
    assert!(
        !undeclared.iter().any(|m| m.contains("`d2`")),
        "d2 (inside the d0-d3 range) must NOT warn: {diags:?}"
    );
}

#[test]
fn clobbers_invalid_register_errors() {
    // C1 item 6: `clobbers(d9)` is not a register — a loud `[proc.clobber-invalid]`.
    let src = "module m\nproc p() clobbers(d9) {\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.clobber-invalid]"))
        .unwrap_or_else(|| panic!("expected [proc.clobber-invalid], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
}

#[test]
fn out_reglist_range_all_written_is_clean() {
    // C1 item 2: `out(d0-d1)` expands; both written → no out-unwritten warning.
    let src = "module m\nproc p() out(d0-d1) {\n    moveq #0, d0\n    moveq #0, d1\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| d.message.contains("[proc.out-")),
        "a fully-written out range must be clean: {diags:?}"
    );
}

#[test]
fn scc_write_undeclared_warns() {
    // `seq d0` (Scc) sets a byte in its sole operand — a real register write.
    // Under `clobbers(d1)`, d0 is undeclared → `[proc.clobber-undeclared]` naming d0.
    let src = "module m\nproc p() clobbers(d1) {\n    seq d0\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let w = diags
        .iter()
        .find(|d| d.message.contains("[proc.clobber-undeclared]"))
        .expect("expected a clobber-undeclared diagnostic for the Scc write");
    assert_eq!(w.level, Level::Warning);
    assert!(w.message.contains("d0"), "names the Scc destination register: {}", w.message);
}

#[test]
fn read_only_op_does_not_warn() {
    // A read-only mnemonic (`cmp`) with a register in last-operand position must
    // NOT warn — this guards the write-form allowlist from a careless future edit
    // that adds a read-only mnemonic to it.
    let src = "module m\nproc p() clobbers(d0) {\n    cmp.l d2, d3\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "a read-only op must not trip the clobber lint: {diags:?}"
    );
}

#[test]
fn memory_destination_does_not_warn() {
    // A memory-destination write (`move.l d0, (a1)`) has no register destination —
    // guards the `ops.last()` == Reg filter (d0 here is the source, not written).
    let src = "module m\nproc p() clobbers(d0) {\n    move.l d0, (a1)\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "a memory-destination write must not trip the clobber lint: {diags:?}"
    );
}

#[test]
fn declared_clobber_does_not_warn() {
    // Companion: writing only a declared clobber (`d0`) → no clobber diagnostic.
    let src = "module m\nproc p() clobbers(d0, d1) {\n    moveq #0, d0\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "writing a declared clobber must not warn: {diags:?}"
    );
}

#[test]
fn param_register_write_is_not_an_undeclared_clobber() {
    // A write to a PARAM register is part of the proc's contract, not an
    // undeclared clobber: `move.l d0, d2` with `d2` a param and `d0` clobbered.
    let src = "module m\n\
               proc p(d2: u8) clobbers(d0) {\n    move.l d0, d2\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "writing a param register must not warn: {diags:?}"
    );
}

// ---- Plan 7 #8: jbra/jbsr fallthrough-terminator recognition (D2.18) --------

#[test]
fn jbra_terminates_proc_no_fallthrough_warning() {
    // `jbra <label>` is an UNCONDITIONAL control transfer, so a proc ending in it
    // terminates straight-line flow — no `[proc.undeclared-fallthrough]` warning
    // (the pitcher_plant `jbra Draw_Sprite` tail case). Also proves `jbra` is a
    // recognized proc-body mnemonic (the b1 gap: it used to error "not a
    // recognized 68000 mnemonic").
    let (_module, diags) =
        lower("module m\nproc p() {\n    moveq #0, d0\n    jbra Draw_Sprite\n}\ndata Draw_Sprite: [u8;2] = [$00, $00]\n");
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "jbra must lower without error: {diags:?}"
    );
    assert!(
        !has_tag(&diags, "[proc.undeclared-fallthrough]"),
        "a proc ending in jbra must not warn about fallthrough: {diags:?}"
    );
}

#[test]
fn jbsr_does_not_terminate_proc() {
    // `jbsr <label>` is a CALL — control returns, so a proc whose last instruction
    // is `jbsr` still falls through → the undeclared-fallthrough warning fires
    // (jbsr is deliberately NOT a terminator, mirroring bsr/jsr).
    let (_module, diags) =
        lower("module m\nproc p() {\n    moveq #0, d0\n    jbsr ObjectMove\n}\ndata ObjectMove: [u8;2] = [$00, $00]\n");
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "jbsr must lower without error: {diags:?}"
    );
    assert!(
        has_tag(&diags, "[proc.undeclared-fallthrough]"),
        "a proc ending in jbsr (a call) must still warn about fallthrough: {diags:?}"
    );
}

#[test]
fn jbra_with_size_suffix_is_jbra_sized_error() {
    // `jbra` sizes itself — a `.s`/`.w` suffix is a contradiction, not a pin.
    for src in [
        "module m\nproc p() {\n    jbra.s Target\n}\ndata Target: [u8;2] = [$00,$00]\n",
        "module m\nproc p() {\n    jbra.w Target\n}\ndata Target: [u8;2] = [$00,$00]\n",
    ] {
        let (_module, diags) = lower(src);
        assert!(
            has_tag(&diags, "[jbra.sized]"),
            "a sized jbra must be [jbra.sized]: {diags:?}"
        );
    }
}

#[test]
fn jbsr_with_size_suffix_is_jbra_sized_error() {
    // Same self-sizing contract for the call form.
    let (_module, diags) =
        lower("module m\nproc p() {\n    jbsr.w Target\n}\ndata Target: [u8;2] = [$00,$00]\n");
    assert!(has_tag(&diags, "[jbra.sized]"), "a sized jbsr must be [jbra.sized]: {diags:?}");
}

#[test]
fn jbra_non_label_operand_is_label_only_error() {
    // A register-indirect target is a COMPUTED transfer (jmp's job), not jbra's.
    let (_module, diags) = lower("module m\nproc p(a0: *u8) {\n    jbra (a0)\n}\n");
    assert!(
        has_tag(&diags, "[jbra.label-only]"),
        "a register-indirect jbra target must be [jbra.label-only]: {diags:?}"
    );
}

#[test]
fn jbra_immediate_operand_is_label_only_error() {
    // An immediate is not a label either.
    let (_module, diags) = lower("module m\nproc p() {\n    jbra #5\n}\n");
    assert!(
        has_tag(&diags, "[jbra.label-only]"),
        "an immediate jbra target must be [jbra.label-only]: {diags:?}"
    );
}

#[test]
fn jbra_in_z80_section_is_branch_non_68k() {
    // `jbra`/`jbsr` are 68k auto-reaching branches; in a `cpu: z80` section they
    // are `[branch.non-68k]` (the Z80 `jr`→`jp` ladder is deferred), mirroring
    // `[dispatch.non-68k]`'s guard shape.
    let src = "module m\nsection s (cpu: z80, vma: $8000) {\n\
               proc p() {\n    jbra Target\n}\n\
               data Target: [u8;2] = [$00,$00]\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[branch.non-68k]"),
        "jbra in a z80 section must be [branch.non-68k]: {diags:?}"
    );
}

// ---- `preserves(...)` — the S2-D6(b) SYNTACTIC slice (tranche 3) ----------
//
// A proc may declare `preserves(d0-d1/a0)`: the registers it saves and
// restores around its body. The syntactic slice verifies the DECLARED set
// against the literal `movem <list>, -(sp)` / `movem (sp)+, <list>` pair
// (first save, last restore) — no dataflow; the full register-contract batch
// stays gated on S2-D6. HBlank_Dispatch is the poster child. This is an
// opt-in declared CONTRACT (like `falls_into`, unlike the clobber lint), so
// violations are error-tier and `@as_compat` does not silence them.

#[test]
fn preserves_matching_movem_pair_ok() {
    // The HBlank_Dispatch shape: save d0-d1/a0, work, restore, rte.
    let src = "module m\n\
               proc h() preserves(d0-d1/a0) {\n\
               \x20   movem.l d0-d1/a0, -(sp)\n\
               \x20   nop\n\
               \x20   movem.l (sp)+, d0-d1/a0\n\
               \x20   rte\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| d.message.contains("[proc.preserves")),
        "a matching movem pair must satisfy the declared preserves set: {diags:?}"
    );
}

#[test]
fn preserves_mask_mismatch_errors() {
    // Declares d0-d1/a0 but the pair saves/restores d0-d2/a0 — the attribute
    // is stale (or the movem wrong); name both sides.
    let src = "module m\n\
               proc h() preserves(d0-d1/a0) {\n\
               \x20   movem.l d0-d2/a0, -(sp)\n\
               \x20   movem.l (sp)+, d0-d2/a0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags.iter().find(|d| d.message.contains("[proc.preserves-mismatch]"));
    let hit = hit.unwrap_or_else(|| panic!("expected [proc.preserves-mismatch], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
    assert!(
        hit.message.contains("d0-d1/a0") && hit.message.contains("d0-d2/a0"),
        "the mismatch must name both the declared and the actual set: {}",
        hit.message
    );
}

#[test]
fn preserves_pop_mask_must_match_too() {
    // Push matches the declaration but the restore differs — an asymmetric
    // save/restore corrupts registers; still a mismatch error.
    let src = "module m\n\
               proc h() preserves(d0-d1/a0) {\n\
               \x20   movem.l d0-d1/a0, -(sp)\n\
               \x20   movem.l (sp)+, d0-d1/a1\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-mismatch]"),
        "a restore differing from the declared set must be a mismatch: {diags:?}"
    );
}

#[test]
fn preserves_missing_pair_errors() {
    // Declares preserves but the body never saves to the stack.
    let src = "module m\n\
               proc h() preserves(d0-d1/a0) {\n\
               \x20   nop\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags.iter().find(|d| d.message.contains("[proc.preserves-missing-pair]"));
    let hit =
        hit.unwrap_or_else(|| panic!("expected [proc.preserves-missing-pair], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
}

#[test]
fn preserves_pop_only_is_missing_pair() {
    // A restore with no save is not a pair (order matters: save, then restore).
    let src = "module m\n\
               proc h() preserves(d0) {\n\
               \x20   movem.l (sp)+, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-missing-pair]"),
        "a pop-only body must be a missing pair: {diags:?}"
    );
}

#[test]
fn preserves_clobbers_overlap_errors() {
    // A register cannot be both preserved and clobbered.
    let src = "module m\n\
               proc h() clobbers(d0) preserves(d0-d1) {\n\
               \x20   movem.l d0-d1, -(sp)\n\
               \x20   movem.l (sp)+, d0-d1\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit =
        diags.iter().find(|d| d.message.contains("[proc.preserves-clobbers-overlap]"));
    let hit = hit
        .unwrap_or_else(|| panic!("expected [proc.preserves-clobbers-overlap], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
    assert!(hit.message.contains("d0"), "must name the overlapping register: {}", hit.message);
}

#[test]
fn preserves_invalid_register_errors() {
    // `d9` is not a register; a declared contract over nonsense is an error.
    let src = "module m\n\
               proc h() preserves(d9) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(has_tag(&diags, "[proc.preserves-invalid]"), "expected invalid-register: {diags:?}");
}

#[test]
fn preserves_reversed_range_errors() {
    let src = "module m\n\
               proc h() preserves(d1-d0) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(has_tag(&diags, "[proc.preserves-invalid]"), "expected reversed-range: {diags:?}");
}

#[test]
fn as_compat_does_not_silence_preserves() {
    // `@as_compat` silences the heuristic modernization lints, NOT declared
    // contracts (same rule as the falls_into adjacency error).
    let src = "module m\n@as_compat\n\
               proc h() preserves(d0-d1/a0) {\n\
               \x20   nop\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-missing-pair]"),
        "@as_compat must not silence a declared preserves contract: {diags:?}"
    );
}

#[test]
fn preserves_composes_with_clobbers_and_falls_into() {
    // Attribute order is free; disjoint clobbers+preserves+falls_into all on
    // one proc must parse and check cleanly.
    let src = "module m\n\
               proc a() clobbers(d2) preserves(d0/a0) falls_into b {\n\
               \x20   movem.l d0/a0, -(sp)\n\
               \x20   moveq #1, d2\n\
               \x20   movem.l (sp)+, d0/a0\n\
               }\n\
               proc b() {\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| d.level == Level::Error),
        "composed attributes must lower cleanly: {diags:?}"
    );
}

#[test]
fn stack_pointer_writes_are_not_clobbers() {
    // Tranche 3 (motivated by collision_lookup's original `addq.l #2, sp`
    // discard path, since optimized away in step 5): direct
    // stack-pointer arithmetic is stack DISCIPLINE, not a register clobber —
    // every proc that pushes/pops adjusts sp, and balanced-stack verification
    // is S2-D7(b)'s dataflow job, not the clobber heuristic's. A declared
    // clobber set must not force `sp` (or warn on it).
    let src = "module m\n\
               proc h() clobbers(d0) {\n\
               \x20   move.w  d0, -(sp)\n\
               \x20   addq.l  #2, sp\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "sp adjustment must not be flagged as an undeclared clobber: {diags:?}"
    );
}

#[test]
fn preserves_movem_w_pair_is_not_verification() {
    // Review finding (tranche 3, Important): `movem.w (sp)+, <list>`
    // SIGN-EXTENDS each word into the full 32-bit register — a `.w` pair
    // does NOT preserve registers, so it must not verify the contract.
    let src = "module m\n\
               proc h() preserves(d0-d1) {\n\
               \x20   movem.w d0-d1, -(sp)\n\
               \x20   movem.w (sp)+, d0-d1\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().any(|d| {
            d.level == Level::Error
                && d.message.contains("[proc.preserves")
                && d.message.contains("movem.l")
        }),
        "a movem.w pair must not verify preserves (sign-extension corrupts \
         upper halves) and the error must steer to movem.l: {diags:?}"
    );
}

#[test]
fn preserves_early_exit_wrong_list_pop_is_caught() {
    // Review finding (tranche 3): an early-exit restore with the WRONG list
    // must not slip past a first-push/last-pop-only comparison. Rule: every
    // stack movem whose list INTERSECTS the declared set must EQUAL it.
    let src = "module m\n\
               proc h() preserves(d0-d1) {\n\
               \x20   movem.l d0-d1, -(sp)\n\
               \x20   tst.w   d0\n\
               \x20   beq.s   .out\n\
               \x20   movem.l (sp)+, d0-d2\n\
               \x20   rts\n\
               .out:\n\
               \x20   movem.l (sp)+, d0-d1\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-mismatch]"),
        "the wrong-list early-exit pop must be caught: {diags:?}"
    );
}

#[test]
fn preserves_disjoint_nested_save_is_allowed() {
    // The complement of the intersects-must-equal rule: a nested movem pair
    // saving DISJOINT registers (e.g. around an inner call) is not part of
    // the declared contract and must not false-positive.
    let src = "module m\n\
               proc h() preserves(d0-d1) {\n\
               \x20   movem.l d0-d1, -(sp)\n\
               \x20   movem.l d3-d4, -(sp)\n\
               \x20   nop\n\
               \x20   movem.l (sp)+, d3-d4\n\
               \x20   movem.l (sp)+, d0-d1\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| d.message.contains("[proc.preserves")),
        "a disjoint nested save must not trip the contract: {diags:?}"
    );
}

#[test]
fn stack_pointer_replacement_is_still_a_clobber() {
    // Review finding (tranche 3): the sp exemption must cover stack
    // ARITHMETIC (addq/adda/lea-over-sp cleanup), not stack REPLACEMENT —
    // `movea.l d0, sp` is a genuine, dangerous a7 clobber.
    let src = "module m\n\
               proc h() clobbers(d0) {\n\
               \x20   movea.l d0, sp\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.clobber-undeclared]"),
        "replacing sp must still be flagged as an undeclared clobber: {diags:?}"
    );
}

#[test]
fn lea_stack_cleanup_over_sp_is_not_a_clobber() {
    // The classic `lea N(sp), sp` frame-cleanup idiom is stack arithmetic
    // (the same class as addq #N, sp) — exempt.
    let src = "module m\n\
               proc h() clobbers(d0) {\n\
               \x20   move.w  d0, -(sp)\n\
               \x20   lea.l   2(sp), sp\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "lea N(sp), sp cleanup must not be flagged: {diags:?}"
    );
}

#[test]
fn empty_clobbers_means_touches_nothing_and_flags_any_write() {
    // Volence ruling (tranche-3 packet review): explicit `clobbers()` is the
    // strongest contract — "verified: touches nothing" — so ANY register
    // write inside is an undeclared clobber.
    let src = "module m\n\
               proc h() clobbers() {\n\
               \x20   moveq   #0, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.clobber-undeclared]"),
        "a write inside clobbers() must be flagged: {diags:?}"
    );
}

#[test]
fn empty_clobbers_on_a_no_effect_proc_is_clean() {
    // The HBlank_Null shape: bare rts, contract declared and verified.
    let src = "module m\n\
               proc h() clobbers() {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| d.level == Level::Error || d.message.contains("[proc.clobber")),
        "a no-effect proc with clobbers() must lower clean: {diags:?}"
    );
}

#[test]
fn absent_clobbers_still_means_no_contract() {
    // Absence stays legal (half-ported files): no declaration, no lint.
    let src = "module m\n\
               proc h() {\n\
               \x20   moveq   #0, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "no declared contract must mean no clobber lint: {diags:?}"
    );
}

// ---- preserves(sr) — S2-D7's first syntactic slice (tranche 5) ------------

/// The Sound_PostByte shape: save → mask → restore, declared `preserves(sr)`
/// — clean (the balance heuristic passes), and no `[proc.sr-undeclared]`.
#[test]
fn preserves_sr_balanced_idiom_is_clean() {
    let src = "module m\n\
               proc f() clobbers() preserves(sr) {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \tnop\n\
               \tmove.w (sp)+, sr\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().all(|d| d.level != Level::Error)
            && !diags.iter().any(|d| d.message.contains("[proc.sr-undeclared]")),
        "the balanced idiom must be clean: {diags:?}"
    );
}

/// Missing restore (or a trailing non-restore SR write) under `preserves(sr)`
/// is the `[proc.preserves-sr-unbalanced]` error.
#[test]
fn preserves_sr_missing_restore_errors() {
    let src = "module m\n\
               proc f() preserves(sr) {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "a missing restore must fail the balance check: {diags:?}"
    );
}

/// An SR write BEFORE the save is unbalanced too (the save must bracket).
#[test]
fn preserves_sr_write_before_save_errors() {
    let src = "module m\n\
               proc f() preserves(sr) {\n\
               \tmove.w #$2700, sr\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w (sp)+, sr\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "a mask before the save must fail the balance check: {diags:?}"
    );
}

/// No SR writes at all — `preserves(sr)` holds vacuously.
#[test]
fn preserves_sr_vacuous_is_clean() {
    let src = "module m\nproc f() preserves(sr) {\n\tnop\n\trts\n}\n";
    let (_module, diags) = lower(src);
    assert!(diags.iter().all(|d| d.level != Level::Error), "vacuous must be clean: {diags:?}");
}

/// An SR write in a proc whose contract names neither `clobbers(sr)` nor
/// `preserves(sr)` warns `[proc.sr-undeclared]`; `clobbers(sr)` silences it.
#[test]
fn sr_write_without_declaration_warns() {
    let src = "module m\n\
               proc f() clobbers() {\n\
               \tmove.w #$2700, sr\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(has_tag(&diags, "[proc.sr-undeclared]"), "expected the warning: {diags:?}");

    let src = "module m\n\
               proc f() clobbers(sr) {\n\
               \tmove.w #$2700, sr\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.sr-undeclared]"),
        "clobbers(sr) must silence the warning: {diags:?}"
    );
}

/// `preserves(sr)` + `clobbers(sr)` is the contradiction error.
#[test]
fn preserves_sr_clobbers_sr_overlap_errors() {
    let src = "module m\nproc f() clobbers(sr) preserves(sr) {\n\trts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-clobbers-overlap]"),
        "sr in both sets must be diagnosed: {diags:?}"
    );
}

/// `preserves(ccr)` steers to S2-D7 (flag liveness needs dataflow, not the
/// syntactic slice); `sr` inside a reglist RANGE stays invalid.
#[test]
fn preserves_ccr_and_sr_range_are_rejected() {
    let src = "module m\nproc f() preserves(ccr) {\n\trts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().any(|d| d.message.contains("S2-D7")),
        "ccr must steer to S2-D7: {diags:?}"
    );

    let src = "module m\nproc f() preserves(sr-d0) {\n\trts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-invalid]"),
        "sr in a range must stay invalid: {diags:?}"
    );
}

/// `preserves(sr)` composes with a reg-list `preserves` — the movem pair is
/// still demanded for the REGISTER set, the balance check for sr.
#[test]
fn preserves_sr_composes_with_reglist() {
    let src = "module m\n\
               proc f() preserves(d1/sr) {\n\
               \tmovem.l d1, -(sp)\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \tmove.w (sp)+, sr\n\
               \tmovem.l (sp)+, d1\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(diags.iter().all(|d| d.level != Level::Error), "composed contract: {diags:?}");
}

// ---- `out(...)` — the S2-D6(e) register-output partition member ------------
//
// A proc does one of three things to each register: preserves it (untouched),
// clobbers it (destroyed scratch), or RETURNS it (a result the caller reads).
// `out(...)` spells the third. Output registers join `check_clobbers`' allowed
// set (a result write is not `[proc.clobber-undeclared]` — the immediate win),
// and a declared-but-unwritten output / an out-clobbers|preserves overlap /
// an invalid spelling are diagnosed. Like `preserves`, a DECLARED contract —
// NOT silenced by `@as_compat`. Byte-neutral: `out` is pure metadata.

#[test]
fn out_register_write_is_not_an_undeclared_clobber() {
    // THE win: `movea.w (X).w, a1` writes a1, which is neither a clobber nor a
    // param — but `out(a1)` declares it a returned result, so no
    // clobber-undeclared for a1.
    let src = "module m\n\
               proc f() clobbers(d0) out(a1) {\n\
               \x20   movea.w (0).w, a1\n\
               \x20   moveq   #0, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "an out-declared register write must not be a clobber: {diags:?}"
    );
    assert!(
        !has_tag(&diags, "[proc.out-unwritten]"),
        "a1 IS written, so no out-unwritten: {diags:?}"
    );
    assert!(diags.iter().all(|d| d.level != Level::Error), "clean contract: {diags:?}");
}

#[test]
fn out_unwritten_warns() {
    // `out(a1)` but a1 is never written on any path — a false output claim.
    let src = "module m\n\
               proc f() out(a1) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.out-unwritten]"))
        .expect("expected an out-unwritten diagnostic");
    assert_eq!(hit.level, Level::Warning);
    assert!(hit.message.contains("a1"), "must name the unwritten output: {}", hit.message);
}

#[test]
fn out_clobbers_overlap_errors() {
    // A register cannot be both a returned result and destroyed scratch.
    let src = "module m\n\
               proc f() clobbers(d0) out(d0) {\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.out-clobbers-overlap]"))
        .unwrap_or_else(|| panic!("expected [proc.out-clobbers-overlap], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
    assert!(hit.message.contains("d0"), "must name the overlapping register: {}", hit.message);
}

#[test]
fn out_preserves_overlap_errors() {
    // A register cannot be both a returned result and left untouched.
    let src = "module m\n\
               proc f() preserves(a0) out(a0) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.out-preserves-overlap]"))
        .unwrap_or_else(|| panic!("expected [proc.out-preserves-overlap], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
    assert!(hit.message.contains("a0"), "must name the overlapping register: {}", hit.message);
}

#[test]
fn out_preserves_overlap_within_a_range_errors() {
    // The overlap check must expand a preserves RANGE — `out(a1)` overlaps
    // `preserves(a0-a2)`.
    let src = "module m\n\
               proc f() preserves(a0-a2) out(a1) {\n\
               \x20   movem.l a0-a2, -(sp)\n\
               \x20   movem.l (sp)+, a0-a2\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.out-preserves-overlap]"),
        "an out register inside a preserves range must overlap: {diags:?}"
    );
}

#[test]
fn out_invalid_register_errors() {
    // `zz` is not a register; a declared output over nonsense is an error.
    let src = "module m\n\
               proc f() out(zz) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.out-invalid]"))
        .unwrap_or_else(|| panic!("expected [proc.out-invalid], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
}

#[test]
fn as_compat_does_not_silence_out_contract() {
    // `out` is a declared contract, not a heuristic modernization lint — so
    // `@as_compat` must NOT silence its checks (mirrors preserves).
    let src = "module m\n@as_compat\n\
               proc f() out(a1) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.out-unwritten]"),
        "@as_compat must not silence a declared out contract: {diags:?}"
    );
}

#[test]
fn out_composes_with_clobbers_and_preserves() {
    // Clause order is free; disjoint clobbers + preserves + out all on one
    // proc must parse and check cleanly.
    let src = "module m\n\
               proc f() clobbers(d0) preserves(a2) out(a1) {\n\
               \x20   movem.l a2, -(sp)\n\
               \x20   movea.w (0).w, a1\n\
               \x20   moveq   #0, d0\n\
               \x20   movem.l (sp)+, a2\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "composed contract must lower cleanly: {diags:?}"
    );
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]") && !has_tag(&diags, "[proc.out-unwritten]"),
        "no spurious clobber/out warnings: {diags:?}"
    );
}

#[test]
fn out_is_byte_neutral() {
    // `out(...)` is metadata — it changes NO codegen. A proc with vs without
    // `out(a1)` must emit IDENTICAL bytes.
    let with = "module m\n\
                proc f() out(a1) {\n\
                \x20   movea.w (0).w, a1\n\
                \x20   rts\n\
                }\n";
    let without = "module m\n\
                   proc f() {\n\
                   \x20   movea.w (0).w, a1\n\
                   \x20   rts\n\
                   }\n";
    let (m_with, _) = lower(with);
    let (m_without, _) = lower(without);
    assert_eq!(
        flatten(&m_with),
        flatten(&m_without),
        "out is metadata — the emitted bytes must be identical"
    );
}

// ---------------------------------------------------------------------------
// Auto-inc / -dec write detection ([out-clause, 2026-07-11] gap-ledger row).
// `(An)+` and `-(An)` MODIFY `An` regardless of operand position or mnemonic;
// the write set must count them so a scratch pointer scribbled via `(a4)+`
// warns, and a genuine in-out pointer output can be declared `out(a4)`.
// ---------------------------------------------------------------------------

#[test]
fn postinc_dest_clobber_undeclared_warns() {
    // `move.w d0, (a4)+` ADVANCES a4 (post-increment destination). Under
    // `clobbers(d0)`, a4 is undeclared → `[proc.clobber-undeclared]` naming a4.
    let src = "module m\nproc p() clobbers(d0) {\n    move.w d0, (a4)+\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let w = diags
        .iter()
        .find(|d| d.message.contains("[proc.clobber-undeclared]"))
        .expect("post-increment of a4 is a write of a4");
    assert_eq!(w.level, Level::Warning);
    assert!(w.message.contains("a4"), "names the advanced pointer register: {}", w.message);
}

#[test]
fn postinc_source_clobber_undeclared_warns() {
    // `move.w (a4)+, d0` advances a4 even though a4 is the SOURCE operand.
    // d0 is declared; a4 is not → warns naming a4 (not d0).
    let src = "module m\nproc p() clobbers(d0) {\n    move.w (a4)+, d0\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let undeclared: Vec<&str> = diags
        .iter()
        .filter(|d| d.message.contains("[proc.clobber-undeclared]"))
        .map(|d| d.message.as_str())
        .collect();
    assert!(undeclared.iter().any(|m| m.contains("`a4`")), "source-position a4 postinc must warn: {diags:?}");
    assert!(!undeclared.iter().any(|m| m.contains("`d0`")), "declared d0 must not warn: {diags:?}");
}

#[test]
fn predec_clobber_undeclared_warns() {
    // `move.w d0, -(a3)` pre-decrements a3. Under `clobbers(d0)`, a3 warns.
    let src = "module m\nproc p() clobbers(d0) {\n    move.w d0, -(a3)\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let w = diags
        .iter()
        .find(|d| d.message.contains("[proc.clobber-undeclared]"))
        .expect("pre-decrement of a3 is a write of a3");
    assert!(w.message.contains("a3"), "names the pre-decremented register: {}", w.message);
}

#[test]
fn autoinc_on_read_only_mnemonic_warns() {
    // `tst.w (a2)+` advances a2 even though `tst` is read-only (not a write-form
    // mnemonic) — the auto-inc effect is on the addressing mode, not the opcode.
    let src = "module m\nproc p() clobbers() {\n    tst.w (a2)+\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let w = diags
        .iter()
        .find(|d| d.message.contains("[proc.clobber-undeclared]"))
        .expect("post-increment on a read-only op still writes a2");
    assert!(w.message.contains("a2"), "names a2: {}", w.message);
}

#[test]
fn declared_autoinc_pointer_is_silent() {
    // Positive control: declaring `clobbers(d0, a4)` silences the a4 post-increment.
    let src = "module m\nproc p() clobbers(d0, a4) {\n    move.w d0, (a4)+\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "a declared auto-inc pointer must not warn: {diags:?}"
    );
}

#[test]
fn out_pointer_advanced_via_postinc_is_written() {
    // The DrawRings case: an in-out pointer output written ONLY via `(a4)+` is a
    // genuine write of a4, so `out(a4)` must NOT trip `[proc.out-unwritten]`.
    let src = "module m\nproc p() out(a4) {\n    move.w d0, (a4)+\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.out-unwritten]"),
        "an out pointer advanced via postinc is written: {diags:?}"
    );
}

#[test]
fn stack_replacement_pop_into_sp_is_a_clobber() {
    // `movea.l (sp)+, sp` pops the top of stack INTO sp — stack REPLACEMENT
    // (loading a new stack pointer), which per the tranche-3 scoping is a
    // genuine a7 clobber, NOT stack discipline. Under `clobbers(d0)`, a7 is
    // undeclared → it must warn (the `(sp)+` push/pop exemption must not swallow
    // a bare-a7 destination write in the same instruction).
    let src = "module m\nproc p() clobbers(d0) {\n    movea.l (sp)+, sp\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let a7_warn = diags.iter().any(|d| {
        d.message.contains("[proc.clobber-undeclared]") && d.message.contains("a7")
    });
    assert!(a7_warn, "stack replacement `movea.l (sp)+, sp` must warn on a7: {diags:?}");
}

#[test]
fn pop_into_dreg_keeps_a7_exempt() {
    // `movea.l (sp)+, d0` pops into d0 — the `(sp)+` advances a7 (stack
    // discipline), and a7 is NOT the destination, so a7 stays exempt.
    let src = "module m\nproc p() clobbers(d0) {\n    movea.l (sp)+, d0\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let a7_warn = diags.iter().any(|d| {
        d.message.contains("[proc.clobber-undeclared]") && d.message.contains("a7")
    });
    assert!(!a7_warn, "a pop into a data register keeps a7 exempt (stack discipline): {diags:?}");
}

#[test]
fn stack_push_pop_is_not_a_clobber() {
    // `-(sp)` / `(sp)+` push/pop advance a7 but are stack DISCIPLINE, not a
    // register clobber — the auto-inc detection must stay exempt for a7 (else
    // every push/pop-balancing proc newly false-positives). `move.l d0, -(sp)`
    // and `movea.l (sp)+, d0` under `clobbers(d0)` → no a7 warning.
    let src = "module m\nproc p() clobbers(d0) {\n    move.l d0, -(sp)\n    movea.l (sp)+, d0\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let a7_warn = diags.iter().any(|d| {
        d.message.contains("[proc.clobber-undeclared]") && d.message.contains("a7")
    });
    assert!(!a7_warn, "stack push/pop must not trip the clobber lint on a7: {diags:?}");
}
