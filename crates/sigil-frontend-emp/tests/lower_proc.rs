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
fn movep_load_undeclared_warns() {
    // S2-D6 U1: `movep.w 4(a1), d0` (LOAD) writes d0 — the ISA model now
    // classifies movep as a write-form (the old string list missed it). Under
    // `clobbers(a1)`, d0 is undeclared → `[proc.clobber-undeclared]` naming d0.
    let src = "module m\nproc p() clobbers(a1) {\n    movep.w 4(a1), d0\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let w = diags
        .iter()
        .find(|d| d.message.contains("[proc.clobber-undeclared]"))
        .expect("expected a clobber-undeclared diagnostic for the movep load");
    assert_eq!(w.level, Level::Warning);
    assert!(w.message.contains("d0"), "names the movep load destination: {}", w.message);
}

#[test]
fn movep_store_does_not_warn() {
    // Companion: `movep.l d1, 4(a1)` (STORE) writes MEMORY, not a register — the
    // last-operand-register test correctly finds nothing (d1 is the source). This
    // is the ONLY movep direction in the aeon corpus, which is why the U1 census
    // delta is empty. Under `clobbers()` (touches nothing), no register fires.
    let src = "module m\nproc p() clobbers() {\n    movep.l d1, 4(a1)\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "a movep STORE writes memory, not a register: {diags:?}"
    );
}

#[test]
fn addx_undeclared_warns() {
    // S2-D6 U1: `addx.l d1, d0` writes d0 (Dy,Dx form). Under `clobbers(d1)`, d0
    // is undeclared → `[proc.clobber-undeclared]` naming d0. addx was in the old
    // string list too; this pins the ISA model preserves that classification.
    let src = "module m\nproc p() clobbers(d1) {\n    addx.l d1, d0\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let w = diags
        .iter()
        .find(|d| d.message.contains("[proc.clobber-undeclared]"))
        .expect("expected a clobber-undeclared diagnostic for the addx write");
    assert_eq!(w.level, Level::Warning);
    assert!(w.message.contains("d0"), "names the addx destination: {}", w.message);
}

#[test]
fn dbcc_counter_undeclared_warns() {
    // S2-D6 effect (3): `dbf d7, .loop` DECREMENTS d7 (its first operand). Under
    // `clobbers(d0)`, d7 is undeclared → `[proc.clobber-undeclared]` naming d7.
    // Mutation trap: dropping the dbcc arm from `instr_written_regs` makes this
    // go green (no d7 write detected) — proving the arm load-bearing.
    let src = "module m\nproc p() clobbers(d0) {\n    moveq #3, d0\n.loop:\n    nop\n    dbf d0, .loop\n    dbf d7, .loop\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let w = diags
        .iter()
        .find(|d| d.message.contains("[proc.clobber-undeclared]"))
        .expect("expected a clobber-undeclared diagnostic for the dbf counter");
    assert_eq!(w.level, Level::Warning);
    assert!(w.message.contains("d7"), "names the dbf counter register: {}", w.message);
}

#[test]
fn dbcc_counter_declared_does_not_warn() {
    // Companion: a `dbf d0` whose counter d0 IS declared clobbers → silent (the
    // real-corpus shape — every counter is declared or moveq-initialized).
    let src = "module m\nproc p() clobbers(d0) {\n    moveq #3, d0\n.loop:\n    nop\n    dbf d0, .loop\n    rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "a declared dbf counter must not trip the clobber lint: {diags:?}"
    );
}

#[test]
fn nonstack_movem_load_reglist_undeclared_warns() {
    // S2-D6 effect (4): a NON-stack `movem.l (a0)+, d0-d1` LOADS d0/d1 with fresh
    // values (a real clobber — the tile_cache DecompressBlock burst shape). Under
    // `clobbers(a0)`, d0/d1 are undeclared → clobber-undeclared naming both.
    let src = "module m\nproc p() clobbers(a0) {\n    movem.l (a0)+, d0-d1\n    rts\n}\n";
    let (_module, diags) = lower(src);
    let named: Vec<&str> = diags
        .iter()
        .filter(|d| d.message.contains("[proc.clobber-undeclared]"))
        .map(|d| d.message.as_str())
        .collect();
    assert!(named.iter().any(|m| m.contains("`d0`")), "movem-load target d0 must warn: {diags:?}");
    assert!(named.iter().any(|m| m.contains("`d1`")), "movem-load target d1 must warn: {diags:?}");
}

#[test]
fn stack_movem_restore_over_save_does_not_warn_the_rider2_trap() {
    // RIDER 2 (the (sp)+ exemption trap): a defensive over-save that pushes d0-d7
    // but only modifies d0-d3 declares `clobbers(d0-d3)`. The `movem.l (sp)+, d0-d7`
    // RESTORE is preserve-discipline — d4-d7 are saved+restored, NOT clobbered — so
    // it must NOT fire on d4-d7. An implementation that counts stack restores as
    // writes (drops the `(sp)+` exemption in effect (4)) breaks this test.
    let src = "module m\nproc p() clobbers(d0-d3) {\n\
               \x20   movem.l d0-d7, -(sp)\n\
               \x20   moveq #0, d0\n\
               \x20   moveq #0, d1\n\
               \x20   moveq #0, d2\n\
               \x20   moveq #0, d3\n\
               \x20   movem.l (sp)+, d0-d7\n\
               \x20   rts\n}\n";
    let (_module, diags) = lower(src);
    let named: Vec<&str> = diags
        .iter()
        .filter(|d| d.message.contains("[proc.clobber-undeclared]"))
        .map(|d| d.message.as_str())
        .collect();
    for r in ["d4", "d5", "d6", "d7"] {
        assert!(
            !named.iter().any(|m| m.contains(&format!("`{r}`"))),
            "a (sp)+ movem restore must NOT clobber-fire {r} (preserve-discipline): {diags:?}"
        );
    }
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

#[test]
fn verified_preserves_is_not_an_undeclared_clobber() {
    // S2-D6 FP-kill (commit B): a proc that WRITES a0 (`lea`) but SAVE/RESTORES it
    // by individual push/pop declares `preserves(a0)`; §5 verifies it, so a0 is a
    // preserved write, NOT an undeclared clobber. Before B the local lint fired a0
    // here even though the register is honestly `preserves`-declared (the
    // AllocDynamic/Collected_*/TrySpawn* shape — 25 corpus FPs). Now silent.
    let src = "module m\nproc p() clobbers() preserves(a0) {\n\
               \x20   move.l a0, -(sp)\n\
               \x20   lea Foo, a0\n\
               \x20   movea.l (sp)+, a0\n\
               \x20   rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "a §5-verified preserved register must not fire clobber-undeclared: {diags:?}"
    );
}

#[test]
fn unverifiable_preserves_still_fires_clobber_undeclared_the_rider1_trap() {
    // RIDER 1 (the verified-only trap): a DECLARED but UNVERIFIABLE `preserves(a0)`
    // (push, corrupt sp, never a clean restore of a0's entry value) must STILL fire
    // `[proc.clobber-undeclared]` on a0 — subtracting a merely-DECLARED preserves
    // would let the lint inherit the exact dishonesty pressure it exists to kill.
    // (`verified_preserves_regs` returns ∅ on the §5 error, so a0 is not allowed.)
    let src = "module m\nproc p() clobbers() preserves(a0) {\n\
               \x20   move.l a0, -(sp)\n\
               \x20   lea Foo, a0\n\
               \x20   adda.w #4, sp\n\
               \x20   movea.l (sp)+, a0\n\
               \x20   rts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        diags
            .iter()
            .any(|d| d.message.contains("[proc.clobber-undeclared]") && d.message.contains("a0")),
        "an unverifiable preserves must NOT subtract → a0 still fires: {diags:?}"
    );
}

#[test]
fn preserves_restore_into_different_register_still_fires() {
    // Mutation-resistant companion: a proc that saves a0 but "restores" into a1
    // (a1 gets a FRESH value, a0 is never restored) — `preserves(a0)` is
    // unverifiable (a0 not round-tripped), so a0 STILL fires, and the freshly
    // written a1 fires too. A balance-only heuristic (push count == pop count)
    // would wrongly clear a0; §5's entry-value tracking does not.
    let src = "module m\nproc p() clobbers() preserves(a0) {\n\
               \x20   move.l a0, -(sp)\n\
               \x20   lea Foo, a0\n\
               \x20   movea.l (sp)+, a1\n\
               \x20   rts\n}\n";
    let (_module, diags) = lower(src);
    let named: Vec<&str> = diags
        .iter()
        .filter(|d| d.message.contains("[proc.clobber-undeclared]"))
        .map(|d| d.message.as_str())
        .collect();
    assert!(named.iter().any(|m| m.contains("`a0`")), "a0 (not round-tripped) must fire: {diags:?}");
    assert!(named.iter().any(|m| m.contains("`a1`")), "a1 (fresh restore target) must fire: {diags:?}");
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
fn preserves_superset_save_verifies_declared_subset() {
    // §5 upgrade over the D2.32 intersects-must-equal rule: a movem that saves a
    // SUPERSET of the declared set still preserves the declared subset (each
    // declared register round-trips). This is the Collected_CheckRing shape
    // (`movem.l d0-d1` saves both, but only `d1` is declared preserved). No error.
    let src = "module m\n\
               proc h() preserves(d0-d1/a0) {\n\
               \x20   movem.l d0-d2/a0, -(sp)\n\
               \x20   movem.l (sp)+, d0-d2/a0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| d.message.contains("[proc.preserves")),
        "a superset save still verifies the declared subset: {diags:?}"
    );
}

#[test]
fn preserves_asymmetric_restore_of_clobbered_reg_errors() {
    // a0 is saved, CLOBBERED, then the restore pops into a1 (a typo/bug) — a0 is
    // never round-tripped, so its declared preserves is unverifiable.
    let src = "module m\n\
               proc h() clobbers(a1) preserves(a0) {\n\
               \x20   move.l  a0, -(sp)\n\
               \x20   lea     X, a0\n\
               \x20   movea.l (sp)+, a1\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a clobbered reg restored into the WRONG register is unverifiable: {diags:?}"
    );
}

#[test]
fn preserves_untouched_regs_are_vacuously_preserved() {
    // §5's "or never writes it" clause: a proc that never touches the declared
    // registers preserves them trivially — no save/restore needed, no error.
    // (The D2.32 slice wrongly demanded a movem pair here.)
    let src = "module m\n\
               proc h() preserves(d0-d1/a0) {\n\
               \x20   nop\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| d.message.contains("[proc.preserves")),
        "untouched declared registers are vacuously preserved: {diags:?}"
    );
}

#[test]
fn preserves_clobber_without_restore_errors() {
    // A declared register clobbered on the return path with no restore is a false
    // contract → [proc.preserves-unverifiable].
    let src = "module m\n\
               proc h() preserves(d0-d1/a0) {\n\
               \x20   lea     Somewhere, a0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags.iter().find(|d| d.message.contains("[proc.preserves-unverifiable]"));
    let hit =
        hit.unwrap_or_else(|| panic!("expected [proc.preserves-unverifiable], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
}

#[test]
fn preserves_pop_only_underflows() {
    // A restore with no matching save pops past the tracked stack (the caller's
    // frame / return address) — the model is inconsistent → unverifiable.
    let src = "module m\n\
               proc h() preserves(d0) {\n\
               \x20   movem.l (sp)+, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a pop-only body underflows and is unverifiable: {diags:?}"
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
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
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
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "the wrong-list early-exit pop underflows and must be caught: {diags:?}"
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

/// `preserves(ccr)` steers to the one blessed spelling of the CCR half
/// (`sr.ccr` — the partition has no synonyms); `sr` inside a reglist RANGE
/// stays invalid.
#[test]
fn preserves_ccr_and_sr_range_are_rejected() {
    let src = "module m\nproc f() preserves(ccr) {\n\trts\n}\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-invalid]")
            && diags.iter().any(|d| d.message.contains("sr.ccr")),
        "ccr must steer to the `sr.ccr` spelling: {diags:?}"
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

// ---- `with <ctx> { }` — whose SR write is it? ------------------------------

/// The engine's real interrupt-mask bracket, inline (a context's acquire and
/// release evaluate in the CONSUMER's scope, so a test spells them out rather
/// than importing them).
const INTS_OFF_CTX: &str = "context ints_off {\n\
     \tacquire = asm { move.w sr, -(sp)\n\
     \t                move.w #$2700, sr }\n\
     \trelease = asm { move.w (sp)+, sr }\n\
     }\n";

/// How many `[proc.sr-undeclared]` firings did this lowering produce?
fn sr_firings(diags: &[Diagnostic]) -> usize {
    diags.iter().filter(|d| d.message.contains("[proc.sr-undeclared]")).count()
}

/// A `with ints_off { }` bracket splices SR traffic into its consumer, and that
/// traffic is the CONTEXT's — the bracket is the declaration, and the consuming
/// proc has no honest contract clause that could name a write it does not make.
///
/// NOT VACUOUS: the spliced acquire masks SR and the spliced release restores it,
/// so two write-form SR-destination instructions reach the lint. Without the
/// exemption each is charged to the consumer, at a source line in the context's
/// module rather than in the consumer's.
#[test]
fn bracketed_sr_traffic_is_the_contexts_declaration_not_the_consumers() {
    let src = format!(
        "module m\n{INTS_OFF_CTX}\
         proc f() clobbers(d0) {{\n\
         \twith ints_off {{\n\
         \t\tmoveq #0, d0\n\
         \t}}\n\
         \trts\n\
         }}\n"
    );
    let (_module, diags) = lower(&src);
    assert_eq!(sr_firings(&diags), 0, "the context's SR traffic must not be charged: {diags:?}");
    assert!(diags.iter().all(|d| d.level != Level::Error), "the bracket must lower clean: {diags:?}");
}

/// The exemption is the ACQUIRE and the RELEASE, never the body between them: a
/// hand-written SR write inside the bracket is the consumer's own code, and
/// defeating the mask the bracket just installed is exactly what the lint is for.
///
/// The sharpest form of the negative probe — one proc, three SR-writing
/// instructions, and exactly ONE firing, so a blanket region exemption fails here
/// while the acquire/release halves stay silent.
#[test]
fn a_hand_written_sr_write_inside_the_bracketed_body_still_fires() {
    let src = format!(
        "module m\n{INTS_OFF_CTX}\
         proc f() clobbers(d0) {{\n\
         \twith ints_off {{\n\
         \t\tmove.w #$2000, sr\n\
         \t\tmoveq #0, d0\n\
         \t}}\n\
         \trts\n\
         }}\n"
    );
    let (_module, diags) = lower(&src);
    assert_eq!(sr_firings(&diags), 1, "the body's own SR write must still be charged: {diags:?}");
}

/// A context that masks and never restores — the round-trip proof's
/// counterexample.
const MASK_ONLY_CTX: &str = "context mask_only {\n\
     \tacquire = asm { move.w #$2700, sr }\n\
     \trelease = asm { nop }\n\
     }\n";

/// The exemption never waives the round-trip obligation — it REDIRECTS it to
/// the context definition: a context that masks and never restores leaves every
/// consumer's SR genuinely changed, and the firing lands at the context's own
/// declaration (naming the context), not at the consumer.
///
/// NOT VACUOUS: the only difference from
/// [`bracketed_sr_traffic_is_the_contexts_declaration_not_the_consumers`] is the
/// missing `move.w (sp)+, sr` in the release.
#[test]
fn a_context_that_never_restores_sr_fires_at_its_own_definition() {
    let src = format!(
        "module m\n{MASK_ONLY_CTX}\
         proc f() clobbers(d0) {{\n\
         \twith mask_only {{\n\
         \t\tmoveq #0, d0\n\
         \t}}\n\
         \trts\n\
         }}\n"
    );
    let (_module, diags) = lower(&src);
    assert_eq!(sr_firings(&diags), 1, "an unrestored mask must fire: {diags:?}");
    assert!(
        diags.iter().any(|d| d.message.contains("context `mask_only`")),
        "the firing names the CONTEXT (the obligation lives at its definition): {diags:?}"
    );
}

/// A release whose LAST SR write is not the restore does not round-trip either —
/// the trailing-write limb of the balance rule, reached from the release half.
/// One firing at the definition, not one per spliced instruction: the context
/// is the party charged, and it has exactly one declaration to fix.
#[test]
fn a_release_that_re_masks_after_restoring_fires_at_the_definition() {
    let src = "module m\n\
               context restore_then_mask {\n\
               \tacquire = asm { move.w sr, -(sp)\n\
               \t                move.w #$2700, sr }\n\
               \trelease = asm { move.w (sp)+, sr\n\
               \t                move.w #$2000, sr }\n\
               }\n\
               proc f() clobbers(d0) {\n\
               \twith restore_then_mask {\n\
               \t\tmoveq #0, d0\n\
               \t}\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert_eq!(sr_firings(&diags), 1, "a release that re-masks leaves SR changed: {diags:?}");
    assert!(
        diags.iter().any(|d| d.message.contains("context `restore_then_mask`")),
        "the firing names the context: {diags:?}"
    );
}

/// The round trip is decided PER CONTEXT, each on its own acquire/release.
///
/// NOT VACUOUS, and nothing else covers it: every other gate here builds a proc
/// with exactly ONE bracket, so a mutant that folds the proofs into a single
/// "do they all round-trip" bool passes all of them. Here two contexts disagree,
/// and only the sound one is silent — `ints_off`'s mask and restore stay exempt
/// while `mask_only` fires at its definition. The CHECK runs at every bracket
/// (each site's spliced stream earns its own exemption — see
/// [`a_context_whose_halves_diverge_per_site_is_checked_at_every_site`]); the
/// REPORT is deduped, so a second `mask_only` bracket adds no second firing
/// (one declaration, one report).
#[test]
fn each_context_is_judged_once_on_its_own_round_trip() {
    let src = format!(
        "module m\n{INTS_OFF_CTX}{MASK_ONLY_CTX}\
         proc f() clobbers(d0) {{\n\
         \twith ints_off {{\n\
         \t\tmoveq #0, d0\n\
         \t}}\n\
         \twith mask_only {{\n\
         \t\tmoveq #1, d0\n\
         \t}}\n\
         \twith mask_only {{\n\
         \t\tmoveq #2, d0\n\
         \t}}\n\
         \trts\n\
         }}\n"
    );
    let (_module, diags) = lower(&src);
    assert_eq!(
        sr_firings(&diags),
        1,
        "only the context that fails the round trip fires, once: {diags:?}"
    );
}

/// The round-trip check runs on EVERY bracket's actually-spliced stream, not
/// once per context name — the Lens C counterexample made a regression pin.
///
/// A context's halves evaluate per site in the consumer's env, so a comptime
/// fn's param can gate them: `gate(1)` splices a round-tripping stream,
/// `gate(0)` splices a mask with no save and no restore — same context name,
/// same proc, one evaluator. A checked-once-per-name scheme proves site 1 and
/// then EXEMPTS site 2 unproven (`Context`-authored, zero firings) — the
/// authored-but-unchecked path §2 forbids. The check-every-site rule fires at
/// the definition regardless of call order; the report is deduped to one.
#[test]
fn a_context_whose_halves_diverge_per_site_is_checked_at_every_site() {
    let ctx_and_fn = "context masked {\n\
         \tacquire = asm {\n\
         \t\tif n == 1 {\n\
         \t\t\tmove.w sr, -(sp)\n\
         \t\t}\n\
         \t\tmove.w #$2700, sr\n\
         \t}\n\
         \trelease = asm {\n\
         \t\tif n == 1 {\n\
         \t\t\tmove.w (sp)+, sr\n\
         \t\t} else {\n\
         \t\t\tnop\n\
         \t\t}\n\
         \t}\n\
         }\n\
         comptime fn gate(n: int) -> Code {\n\
         \treturn asm {\n\
         \t\twith masked {\n\
         \t\t\tnop\n\
         \t\t}\n\
         \t}\n\
         }\n";
    // The attack order: the round-tripping evaluation first. Site 2's stream
    // must still be checked and fire (once, at the definition).
    let src = format!(
        "module m\n{ctx_and_fn}\
         proc f() clobbers() {{\n\
         \tgate(1)\n\
         \tgate(0)\n\
         \trts\n\
         }}\n"
    );
    let (_module, diags) = lower(&src);
    assert_eq!(
        sr_firings(&diags),
        1,
        "the non-round-tripping site must be checked despite an earlier clean one: {diags:?}"
    );
    assert!(
        diags.iter().any(|d| d.message.contains("context `masked`")),
        "the firing lands at the context definition: {diags:?}"
    );
}

/// A spliced TEMPLATE's SR write is `Splice`-authored, which is NOT exempt:
/// the lint charges the consumer exactly as if the line were written inline.
/// The `Splice` author is carried for the future `-> Code` fn-contract check
/// (ledgered); until that exists, the consumer's contract is the only honest
/// address — an exemption here would be an obligation landing nowhere.
#[test]
fn a_spliced_templates_sr_write_is_charged_to_the_consumer() {
    let src = "module m\n\
               comptime fn mask_ints() -> Code {\n\
               \treturn asm {\n\
               \t\tmove.w #$2700, sr\n\
               \t}\n\
               }\n\
               proc f() clobbers() {\n\
               \tmask_ints()\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert_eq!(
        sr_firings(&diags),
        1,
        "a template's undeclared SR write stays the consumer's to declare: {diags:?}"
    );
}

/// NESTED brackets — the corpus's real shape at every `ints_off` site, which wraps
/// a `z80_stopped` region. The outer region's acquire and release ranges must not
/// swallow the inner region's body, and the inner region (which touches no SR)
/// must not shadow the outer one's exemption.
///
/// NOT VACUOUS: a hand-written SR write in the INNER body is the consumer's own
/// code at a doubly-bracketed position, and it is still charged — so the ranges
/// are exact rather than merely generous.
#[test]
fn nested_brackets_keep_their_own_acquire_and_release_ranges() {
    let bus = "context z80_stopped {\n\
               \tacquire = asm { move.w #$0100, Z80_BUS_REQUEST }\n\
               \trelease = asm { move.w #$0000, Z80_BUS_REQUEST }\n\
               }\n";
    let nested = |body: &str| {
        format!(
            "module m\n{INTS_OFF_CTX}{bus}\
             proc f() clobbers(d0) {{\n\
             \twith ints_off {{\n\
             \t\twith z80_stopped {{\n\
             \t\t\t{body}\n\
             \t\t}}\n\
             \t}}\n\
             \trts\n\
             }}\n\
             data Z80_BUS_REQUEST: [u8;2] = [$00, $00]\n"
        )
    };

    let (_module, diags) = lower(&nested("moveq #0, d0"));
    assert_eq!(sr_firings(&diags), 0, "the outer bracket's SR traffic is its own: {diags:?}");

    let (_module, diags) = lower(&nested("move.w #$2000, sr"));
    assert_eq!(sr_firings(&diags), 1, "the inner body is still the consumer's: {diags:?}");
}

/// The declared SR clauses keep their meaning alongside a bracket — the exemption
/// adds a fourth way for an SR write to be declared, it does not replace them.
/// Both clauses a bracketing proc can carry are exercised: `preserves(sr)`, which
/// the spliced save/restore pair must also SATISFY (`[proc.preserves-sr-unbalanced]`
/// stays silent), and `clobbers(sr)`, which is a bare silencer.
#[test]
fn a_bracket_does_not_disturb_the_declared_sr_clauses() {
    let bracketing = |contract: &str| {
        format!(
            "module m\n{INTS_OFF_CTX}\
             proc f() {contract} {{\n\
             \twith ints_off {{\n\
             \t\tmoveq #0, d0\n\
             \t}}\n\
             \trts\n\
             }}\n"
        )
    };

    let (_module, diags) = lower(&bracketing("clobbers(d0) preserves(sr)"));
    assert_eq!(sr_firings(&diags), 0, "a declared preserves(sr) stays silent: {diags:?}");
    assert!(
        !has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "the spliced pair satisfies the balance check: {diags:?}"
    );

    let (_module, diags) = lower(&bracketing("clobbers(d0/sr)"));
    assert_eq!(sr_firings(&diags), 0, "a declared clobbers(sr) stays silent: {diags:?}");
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "clobbers(sr) must not collide with the bracket: {diags:?}"
    );
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
fn cond_out_may_overlap_clobbers() {
    // A register can be a RESULT on one condition edge and destroyed scratch on
    // every other — not the result-or-scratch contradiction
    // `[proc.out-clobbers-overlap]` names, so the honest declaration compiles
    // clean. The check ranges over the whole out set (the parser lands a
    // conditional result there too) and subtracts the conditionally-guarded
    // registers.
    let src = "module m\n\
               proc f() clobbers(d0/a1) out(a1 if eq) {\n\
               \x20   movea.w #0, a1\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| d.message.contains("[proc.out-clobbers-overlap]")),
        "a CONDITIONAL out may overlap clobbers (result on the cc edge, scratch \
         otherwise); got: {diags:?}"
    );
}

#[test]
fn uncond_out_still_may_not_overlap_clobbers() {
    // An UNCONDITIONAL out that is also declared clobbered is the
    // result-or-scratch contradiction and must still error.
    let src = "module m\n\
               proc f() clobbers(d0/a1) out(a1) {\n\
               \x20   movea.w #0, a1\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.out-clobbers-overlap]"))
        .unwrap_or_else(|| panic!("expected [proc.out-clobbers-overlap], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
    assert!(hit.message.contains("a1"), "must name the overlapping register: {}", hit.message);
}

#[test]
fn cond_out_exemption_is_canonical_not_textual() {
    // `sp` and `a7` name the same register. The exemption is expanded through the
    // same register file as the sets it is tested against, so the spelling the
    // author used cannot decide whether the overlap fires. (A textual guard set
    // would hold "sp", the out set "a7", and this would error.) The incidental
    // out-unwritten warning is not this test's subject.
    let src = "module m\n\
               proc f() clobbers(a7) out(sp if eq) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| d.message.contains("[proc.out-clobbers-overlap]")),
        "the cc-guard exemption must survive an sp/a7 spelling difference; got: {diags:?}"
    );
}

/// The exemption is keyed on (register, EXCLUSIVELY conditional). A register
/// mentioned unconditionally AS WELL keeps the unconditional reading — `out(a1,
/// a1 if eq) clobbers(a1)` states outright that a1 is a result and that it is
/// scratch, so it must still error. Non-vacuous against
/// `cond_out_may_overlap_clobbers` above, which is the same declaration minus the
/// unconditional mention and must stay clean.
#[test]
fn an_unconditional_mention_defeats_the_cond_out_exemption() {
    let src = "module m\n\
               proc f() clobbers(d0/a1) out(a1, a1 if eq) {\n\
               \x20   movea.w #0, a1\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.out-clobbers-overlap]"))
        .unwrap_or_else(|| panic!("expected [proc.out-clobbers-overlap], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
    assert!(hit.message.contains("a1"), "must name the overlapping register: {}", hit.message);
}

/// A RANGE covering the register is an unconditional mention too — `out(a0-a2,
/// a1 if eq)` says a1 is produced on every edge, so `clobbers(a1)` still
/// contradicts it. A key built by subtracting the guarded set would remove a1
/// wholesale; the counting key does not.
#[test]
fn a_range_mention_defeats_the_cond_out_exemption() {
    let src = "module m\n\
               proc f() clobbers(a1) out(a0-a2, a1 if eq) {\n\
               \x20   lea Slot, a0\n\
               \x20   lea Slot, a1\n\
               \x20   lea Slot, a2\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.out-clobbers-overlap]"))
        .unwrap_or_else(|| panic!("expected [proc.out-clobbers-overlap], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
    assert!(hit.message.contains("a1"), "must name the overlapping register: {}", hit.message);
}

// === the SURVIVES half of a conditional out (delta spec §7.1) ==============

/// The `AllocEffect` shape — the spec's required PASSING witness. The pool test
/// runs BEFORE the pop, so a1 is untouched on the `!eq` (failure) return and
/// `clobbers(d0)` honestly omits it.
#[test]
fn cond_out_survives_claim_proves_when_the_test_precedes_the_write() {
    let src = "module m\n\
               proc f() clobbers(d0) out(a1 if eq) {\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   lea Slot, a1\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               .full:\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "a1 is never written on the !eq path — the survives claim holds: {diags:?}"
    );
}

/// Hoist the pop above the test and a1 becomes trash on the failure edge while
/// the contract still omits it from `clobbers`. Same terminals as the passing
/// witness above; only the write moves, so the firing is about the write's
/// position and not the shape.
#[test]
fn cond_out_survives_claim_fires_when_the_write_precedes_the_test() {
    let src = "module m\n\
               proc f() clobbers(d0) out(a1 if eq) {\n\
               \x20   lea Slot, a1\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               .full:\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.out-cond-survives-unverifiable]"))
        .unwrap_or_else(|| panic!("expected the survives firing, got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
    assert!(hit.message.contains("a1"), "must name the register: {}", hit.message);
    assert!(hit.message.contains("clobbers"), "must state the remedy: {}", hit.message);
}

/// The `AllocDynamic` shape: the same hoisted-pop body makes NO claim once a1 is
/// declared clobbered, so nothing fires. This is the honest downgrade the
/// error tier depends on being free.
#[test]
fn a_clobbered_cond_out_makes_no_survives_claim() {
    let src = "module m\n\
               proc f() clobbers(d0/a1) out(a1 if eq) {\n\
               \x20   lea Slot, a1\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               .full:\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "a1 in clobbers means no survives claim to break: {diags:?}"
    );
}

/// ⊤ DOES NOT OBLIGATE. The success path here sets its Z result with `move.w
/// #0, d0` instead of `moveq` and then stores through the pointer — flags the cc
/// lattice reads as ⊤ — but a1 is still untouched on the `!eq` path, so the
/// contract is TRUE and must compile clean. Charging ⊤ would reject it at error
/// tier and the only escape (`clobbers(a1)`) would be a false declaration.
///
/// Non-vacuous: the `.full` exit IS classified (`moveq #1, d0`), so the checker
/// is live on this body — `cond_out_survives_claim_still_fires_past_an_unclassifiable_exit`
/// below fires on the same shape once a1 is genuinely destroyed there.
#[test]
fn an_unclassifiable_exit_is_not_charged_the_survives_claim() {
    let src = "module m\n\
               proc f() clobbers(d0) out(a1 if eq) {\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   lea Slot, a1\n\
               \x20   move.w #0, d0\n\
               \x20   move.l d0, (a1)\n\
               \x20   rts\n\
               .full:\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "an unclassifiable cc-success exit must not be charged the ¬cc obligation: {diags:?}"
    );
}

/// The control for the test above: the same ⊤-on-the-success-path body, with the
/// pop hoisted so a1 IS destroyed on the classified `!eq` exit. Skipping ⊤ costs
/// nothing here — the obligation lands on the exit that can be judged.
#[test]
fn cond_out_survives_claim_still_fires_past_an_unclassifiable_exit() {
    let src = "module m\n\
               proc f() clobbers(d0) out(a1 if eq) {\n\
               \x20   lea Slot, a1\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   move.w #0, d0\n\
               \x20   move.l d0, (a1)\n\
               \x20   rts\n\
               .full:\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "a classified !eq exit still carries the obligation: {diags:?}"
    );
}

/// THE COST OF THE ⊤ RULING, pinned so it stays visible. When EVERY exit is
/// unclassifiable the claim goes unchecked — a false negative, deliberately
/// chosen over rejecting honest code. a1 is destroyed on the only exit and
/// nothing fires.
///
/// This test is expected to FLIP the day [`Flags::after`] learns `clr` /
/// `move #imm` (which `branch_const::const_flag_writer` already folds). That is
/// the intended direction: widening the lattice can only add checking here.
#[test]
fn an_all_unclassifiable_body_leaves_the_survives_claim_unchecked() {
    let src = "module m\n\
               proc f() clobbers(d0) out(a1 if eq) {\n\
               \x20   lea Slot, a1\n\
               \x20   tst.w Flag\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "the documented incompleteness: no provably-!eq exit means no obligation: {diags:?}"
    );
}

/// The proof is `preserves`', not merely "never written": a save/restore
/// round-trip across the ¬cc path carries the claim even though a1 IS written
/// there. Non-vacuous against the hoisted-pop firing above — same write, plus a
/// restore.
#[test]
fn a_save_restore_round_trip_carries_the_survives_claim() {
    let src = "module m\n\
               proc f() clobbers(d0) out(a1 if eq) {\n\
               \x20   move.l a1, -(sp)\n\
               \x20   lea Slot, a1\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   addq.l #4, sp\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               .full:\n\
               \x20   movea.l (sp)+, a1\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "a1 is restored on the !eq path — the round-trip proof carries: {diags:?}"
    );
}

/// A proc with NO `clobbers(...)` clause declares no clobber contract at all, so
/// §7.1's membership rule has no input and there is no survives claim to check.
/// Same hoisted-pop body as the firing case; only the clause is gone.
#[test]
fn no_clobber_contract_means_no_survives_claim() {
    // Non-vacuity: this is byte-for-byte the FIRING body above with its
    // `clobbers` clause removed and nothing else weakened. `.full`'s `moveq #1`
    // leaves Z clear, so that `rts` is a PROVABLY ¬cc exit (not ⊤ — the skip-⊤
    // rule is not what is being exercised here), and `lea Slot, a1` writes a1
    // before the branch, so a1 is demonstrably trash when control reaches it.
    // A register is therefore destroyed on a classified ¬cc return and the gate
    // is still silent, which is attributable to the absent clause alone.
    let src = "module m\n\
               proc f() out(a1 if eq) {\n\
               \x20   lea Slot, a1\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               .full:\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "absent a clobbers clause there is no membership to read: {diags:?}"
    );
}

/// Like every declared contract, the survives claim is NOT silenced by
/// `@as_compat` (only the heuristic modernization lints are).
#[test]
fn as_compat_does_not_silence_the_survives_claim() {
    // The undeclared `d5` write is the CONTROL: `[proc.clobber-undeclared]` is a
    // modernization lint, so its absence proves `@as_compat` is genuinely in
    // force and the survives firing below is not passing by default.
    let src = "module m\n\
               @as_compat\n\
               proc f() clobbers(d0) out(a1 if eq) {\n\
               \x20   lea Slot, a1\n\
               \x20   moveq #0, d5\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               .full:\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "control: @as_compat must be in force for this test to mean anything: {diags:?}"
    );
    assert!(
        has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "@as_compat silences modernization lints, not declared contracts: {diags:?}"
    );
}

/// The clobbers-membership skip reads the EXPANDED reglist: `clobbers(d0-d1)`
/// contains d1, so `out(d1 if eq)` makes no claim. Non-vacuous against the
/// control below, which is the same body with d1 outside the declared range.
#[test]
fn the_survives_skip_expands_the_clobbers_reglist() {
    let src = "module m\n\
               proc f() clobbers(d0-d1) out(d1 if eq) {\n\
               \x20   moveq #7, d1\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               .full:\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "a range mention puts d1 in clobbers — no claim to check: {diags:?}"
    );
}

/// The control for the range skip: the same body with d1 OUTSIDE `clobbers`
/// makes the claim, and d1 is destroyed before the test, so it fires. Without
/// this pair the test above would pass with the check deleted.
#[test]
fn a_cond_out_outside_the_clobbers_range_still_claims() {
    let src = "module m\n\
               proc f() clobbers(d0) out(d1 if eq) {\n\
               \x20   moveq #7, d1\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               .full:\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "d1 outside clobbers claims survival and does not survive: {diags:?}"
    );
}

/// A register named BOTH unconditionally and conditionally makes no survives
/// claim. It cannot: `[proc.out-clobbers-overlap]` (re-keyed above) rejects
/// `clobbers(a1)` for it, so charging the claim would leave a contract with no
/// legal spelling — the remedy the diagnostic names would itself be an error.
/// Non-vacuous against `cond_out_survives_claim_fires_when_the_write_precedes_the_test`,
/// which is this body with the plain `a1` mention removed and DOES fire.
#[test]
fn a_register_named_unconditionally_too_makes_no_survives_claim() {
    let src = "module m\n\
               proc f() clobbers(d0) out(a1, a1 if eq) {\n\
               \x20   lea Slot, a1\n\
               \x20   cmpi.w #0, Flag\n\
               \x20   beq .full\n\
               \x20   moveq #0, d0\n\
               \x20   rts\n\
               .full:\n\
               \x20   moveq #1, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.out-cond-survives-unverifiable]"),
        "an unconditional mention makes this an unconditional result, not a claim: {diags:?}"
    );
}

#[test]
fn cond_out_may_not_overlap_preserves() {
    // The preserves half is NOT relaxed for a conditional result: a register
    // written on ANY path contradicts one left untouched on ALL paths, so the
    // cc guard buys nothing here.
    let src = "module m\n\
               proc f() preserves(a1) out(a1 if eq) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.out-preserves-overlap]"))
        .unwrap_or_else(|| panic!("expected [proc.out-preserves-overlap], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
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

#[test]
fn preserves_trailing_preserving_call_defers_not_errors() {
    // §5 callee-preserves oracle (t30): a0 is saved, restored, then a TRAILING
    // call runs before `rts` (the `TestChurnObj_Main` shape). The byte gate has no
    // cross-file contract knowledge, so it cannot prove a0 round-trips — but the
    // failure is blocked SOLELY by the call, so it must DEFER to the corpus closure
    // (which credits the callee's verified `preserves`), NOT emit a per-file error.
    let src = "module m\n\
               proc h() clobbers(d0) preserves(a0) {\n\
               \x20   move.l  a0, -(sp)\n\
               \x20   lea     Foo, a0\n\
               \x20   movea.l (sp)+, a0\n\
               \x20   jsr     DeleteObject\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a call-only preserves failure DEFERS to the closure — no per-file error: {diags:?}"
    );
}

#[test]
fn preserves_post_call_clobber_still_errors() {
    // The DEFER must NOT swallow a genuine local failure: a clobber AFTER the
    // trailing call with no restore fails even the optimistic probe, so the byte
    // gate still errors (defer is call-only, not a blanket amnesty).
    let src = "module m\n\
               proc h() clobbers(d0) preserves(a0) {\n\
               \x20   move.l  a0, -(sp)\n\
               \x20   movea.l (sp)+, a0\n\
               \x20   jsr     DeleteObject\n\
               \x20   lea     Foo, a0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a post-call clobber with no restore is a genuine, non-deferrable error: {diags:?}"
    );
}

// --- The SR split: `sr.mask` / `sr.ccr` half tokens ------------------------
//
// SR partitions into the system byte (`sr.mask`) and the condition codes
// (`sr.ccr`); bare `sr` means both halves. The split exists so the
// out/preserves partition check can SEE a flag result against a preserved
// CCR — an overlap invisible to the register walk (`preserves_reg_bit`
// answers `None` for `sr`, and a flag result never joins the out reglist).

/// THE PINNED PARTITION FACT, half 1: a flag result lives in CCR, so
/// `out(carry: …)` against `preserves(sr.ccr)` is the returned-and-untouched
/// contradiction `[proc.out-preserves-overlap]` exists to catch.
#[test]
fn out_carry_overlapping_preserves_sr_ccr_errors() {
    let src = "module m\n\
               proc f() out(carry: found) preserves(sr.ccr) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.out-preserves-overlap]"))
        .unwrap_or_else(|| panic!("expected [proc.out-preserves-overlap], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
    assert!(
        hit.message.contains("carry"),
        "must name the flag result: {}",
        hit.message
    );
}

/// THE PINNED PARTITION FACT, half 2: the mask half is disjoint from every
/// flag, so `out(carry: …)` beside `preserves(sr.mask)` is the honest
/// partition (the QueueDMA_Deferrable signature) and must NOT overlap. The
/// body is the real shape: mask round-trip, carry pinned AFTER the restore.
#[test]
fn out_carry_does_not_overlap_preserves_sr_mask() {
    let src = "module m\n\
               proc f() clobbers(d0) out(carry: dropped) preserves(sr.mask) {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   move.w  #$2700, sr\n\
               \x20   moveq   #0, d0\n\
               \x20   move.w  (sp)+, sr\n\
               \x20   ori.b   #1, ccr\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "the honest mask/carry partition must be clean: {diags:?}"
    );
}

/// Bare `sr` covers the CCR half, so `out(carry: …) preserves(sr)` is the
/// same returned-and-untouched contradiction as the explicit half — a flag
/// result partitions cleanly only against `sr.mask`.
#[test]
fn out_carry_overlapping_bare_preserves_sr_errors() {
    let src = "module m\n\
               proc f() out(carry: dropped) preserves(sr) {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   move.w  (sp)+, sr\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.out-preserves-overlap]"),
        "bare sr covers the ccr half, so the flag result must overlap: {diags:?}"
    );
}

/// A conditional result's `if cc` guard is read from CCR, so it contradicts a
/// preserved CCR exactly as a flag result does.
#[test]
fn a_cc_guarded_result_contradicts_preserved_flags() {
    let src = "module m\n\
               proc f() clobbers(d0) out(d0 if eq) preserves(sr.ccr) {\n\
               \x20   moveq   #0, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.out-preserves-overlap]"),
        "a cc guard demands CCR carry exit state — preserved flags contradict it: {diags:?}"
    );
}

/// A flag result against a CLOBBERED CCR is the returned-and-scratch
/// contradiction, mirroring the register rule.
#[test]
fn out_carry_overlapping_clobbers_sr_ccr_errors() {
    let src = "module m\n\
               proc f() clobbers(sr.ccr) out(carry: found) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.out-clobbers-overlap]"),
        "carry lives in the clobbered ccr half: {diags:?}"
    );
}

/// The honest split across clauses — mask preserved, flags scratch (the
/// Sound_DrainSfxRing signature) — partitions cleanly: no overlap, no
/// undeclared-SR warning.
#[test]
fn preserves_sr_mask_clobbers_sr_ccr_partitions_cleanly() {
    let src = "module m\n\
               proc f() clobbers(d0/sr.ccr) preserves(sr.mask) {\n\
               \x20   moveq   #0, d0\n\
               \x20   beq     .done\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   move.w  #$2700, sr\n\
               \x20   move.w  (sp)+, sr\n\
               .done:\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().all(|d| d.level != Level::Error)
            && !has_tag(&diags, "[proc.sr-undeclared]"),
        "the disjoint-halves partition must be clean: {diags:?}"
    );
}

/// Bare `sr` against a half token across clauses is the contradiction the
/// half-aware overlap must still catch — `preserves(sr)` covers the ccr half
/// that `clobbers(sr.ccr)` destroys.
#[test]
fn preserves_sr_overlapping_clobbers_sr_ccr_errors() {
    let src = "module m\n\
               proc f() clobbers(sr.ccr) preserves(sr) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-clobbers-overlap]"),
        "bare sr covers the clobbered ccr half: {diags:?}"
    );
}

/// Redundant same-clause co-declaration FOLDS — a reglist denotes a set union
/// everywhere in the grammar (`d0, d0-d3` folds), and the SR family is no
/// different: `preserves(sr/sr.mask)` covers what bare `sr` covers, with no
/// extra diagnostic.
#[test]
fn sr_half_tokens_fold_with_bare_sr() {
    let src = "module m\n\
               proc f() preserves(sr/sr.mask) {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   move.w  #$2700, sr\n\
               \x20   move.w  (sp)+, sr\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(diags.is_empty(), "the fold is a set union, not a diagnostic: {diags:?}");
}

/// `preserves(sr.ccr)` verifies in the one shape the slice can see: every
/// flag-affecting instruction bracketed by the SR save/restore pair (the
/// restore puts back the entry CCR the save captured).
#[test]
fn preserves_sr_ccr_accepts_a_fully_bracketed_body() {
    let src = "module m\n\
               proc f() clobbers(d0) preserves(sr.ccr) {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   moveq   #0, d0\n\
               \x20   add.w   d0, d0\n\
               \x20   move.w  (sp)+, sr\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "a fully bracketed body proves the ccr claim: {diags:?}"
    );
}

/// A flag effect OUTSIDE the bracket refuses — the entry CCR the restore put
/// back is overwritten before the caller sees it. An unverifiable claim is an
/// error, never trusted (mirroring `[proc.out-cond-survives-unverifiable]`).
#[test]
fn preserves_sr_ccr_refuses_a_flag_effect_outside_the_bracket() {
    let src = "module m\n\
               proc f() clobbers(d0) preserves(sr.ccr) {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   move.w  (sp)+, sr\n\
               \x20   moveq   #0, d0\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a post-restore flag write must refuse the ccr claim: {diags:?}"
    );
}

/// A call outside the bracket refuses — the callee's flags are unknown.
#[test]
fn preserves_sr_ccr_refuses_a_call_outside_the_bracket() {
    let src = "module m\n\
               proc f() preserves(sr.ccr) {\n\
               \x20   jsr     Helper\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a call returns with the callee's flags — the claim must refuse: {diags:?}"
    );
}

/// A return BETWEEN save and restore refuses — that path skips the restore.
/// (Sharper than the bare-`sr` slice, which cannot see a mid-bracket return;
/// the ccr slice can, for free, because it walks every item.)
#[test]
fn preserves_sr_ccr_refuses_a_return_inside_the_bracket() {
    let src = "module m\n\
               proc f() preserves(sr.ccr) {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   rts\n\
               \x20   move.w  (sp)+, sr\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a return inside the bracket skips the restore: {diags:?}"
    );
}

/// A half token names the proc's SR traffic for the `[proc.sr-undeclared]`
/// heuristic — declaring `preserves(sr.mask)` addresses the whole-SR writes
/// exactly as bare `sr` did (what each half's claim MEANS is the contract
/// checks' job, not the warn tier's).
#[test]
fn a_half_token_declares_the_procs_sr_traffic() {
    let src = "module m\n\
               proc f() clobbers() preserves(sr.mask) {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   move.w  #$2700, sr\n\
               \x20   move.w  (sp)+, sr\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.sr-undeclared]"),
        "a half token addresses the SR traffic: {diags:?}"
    );
}

/// The dotted spelling is grammar, not magic: a dotted non-SR name is
/// diagnosed at lowering like every other invalid endpoint.
#[test]
fn a_dotted_non_sr_register_is_invalid() {
    let src = "module m\n\
               proc f() clobbers(d0.mask) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.clobber-invalid]"),
        "d0.mask is not a register or SR token: {diags:?}"
    );
}

/// A declared `falls_into` is a tail transfer with no mnemonic: the successor's
/// flag traffic is invisible to the walk, so the claim refuses.
#[test]
fn preserves_sr_ccr_refuses_a_declared_fallthrough() {
    let src = "module m\n\
               proc a() preserves(sr.ccr) falls_into b {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   move.w  (sp)+, sr\n\
               }\n\
               proc b() {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a fallthrough hands the caller the successor's flags: {diags:?}"
    );
}

/// A body that can run off its end is the same mnemonic-less tail, undeclared.
#[test]
fn preserves_sr_ccr_refuses_a_body_that_can_run_off_the_end() {
    let src = "module m\n\
               proc f() preserves(sr.ccr) {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   move.w  (sp)+, sr\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "control running off the end escapes the walk: {diags:?}"
    );
}

/// A nested save refuses — the slice pairs one bracket at a time.
#[test]
fn preserves_sr_ccr_refuses_a_nested_save() {
    let src = "module m\n\
               proc f() preserves(sr.ccr) {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   move.w  (sp)+, sr\n\
               \x20   move.w  (sp)+, sr\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "nested saves cannot be paired statically: {diags:?}"
    );
}

/// An unmatched save refuses — nothing restores the captured CCR.
#[test]
fn preserves_sr_ccr_refuses_an_unmatched_save() {
    let src = "module m\n\
               proc f() preserves(sr.ccr) {\n\
               \x20   move.w  sr, -(sp)\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a save with no restore proves nothing: {diags:?}"
    );
}

/// An unconditional tail transfer refuses — the flags the caller finally sees
/// are the target's.
#[test]
fn preserves_sr_ccr_refuses_a_tail_transfer() {
    let src = "module m\n\
               proc f() preserves(sr.ccr) {\n\
               \x20   jbra    Helper\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a tail transfer's flags are the target's: {diags:?}"
    );
}

/// `rte` loads SR wholesale from the stack with no `sr` operand — the one SR
/// write the round-trip slice's operand shapes cannot see, so a mask claim
/// over an `rte`-bearing body fails the balance check.
#[test]
fn an_rte_fails_a_mask_claim() {
    let src = "module m\n\
               proc f() preserves(sr.mask) {\n\
               \x20   rte\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "rte rewrites the mask from the stack: {diags:?}"
    );
}

/// A whole-SR destination always perturbs the interrupt mask, so a contract
/// covering only the CCR half leaves that write undeclared — the warn tier's
/// bar is MASK coverage, not any-token.
#[test]
fn clobbers_sr_ccr_alone_does_not_silence_a_whole_sr_write() {
    let src = "module m\n\
               proc f() clobbers(sr.ccr) {\n\
               \x20   move.w  #$2700, sr\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.sr-undeclared]"),
        "a ccr-only token does not address a mask write: {diags:?}"
    );
}

/// The mask arms of the out partition: an `out` SR token's mask half against a
/// preserved mask is output-and-untouched, per half.
#[test]
fn out_sr_token_overlaps_a_preserved_mask() {
    let src = "module m\n\
               proc f() out(sr.mask) preserves(sr.mask) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.out-preserves-overlap]"),
        "the mask half partitions like any state: {diags:?}"
    );
}

/// A dotted token inside a range is invalid at lowering, like every other
/// non-register endpoint.
#[test]
fn a_dotted_token_in_a_range_is_invalid() {
    let src = "module m\n\
               proc f() preserves(sr.mask-d0) {\n\
               \x20   rts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-invalid]"),
        "an SR token cannot bound a range: {diags:?}"
    );
}

// ---- @noreturn (noreturn-tail model) --------------------------------------

/// Parse + lower with `DEBUG=1` (the assert/raise rails expand) for the CCR
/// advisory tests; the base `lower` helper leaves DEBUG undefined.
fn lower_debug(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![("DEBUG".into(), 1)],
        },
    )
}

/// A `@noreturn` proc ending in a terminal transfer is clean — no path returns.
#[test]
fn noreturn_terminal_transfer_is_clean() {
    let src = "module m\n@noreturn\nproc loop() clobbers(d0-d7/a0-a6) {\n\tnop\n\tjbra loop\n}\n";
    let (_module, diags) = lower(src);
    assert!(!has_tag(&diags, "[noreturn.returns]"), "a looping tail never returns: {diags:?}");
}

/// A `@noreturn` proc with an `rts` fires `[noreturn.returns]` at error tier.
#[test]
fn noreturn_with_rts_errors() {
    let src = "module m\n@noreturn\nproc f() clobbers() {\n\trts\n}\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[noreturn.returns]"))
        .unwrap_or_else(|| panic!("expected [noreturn.returns], got: {diags:?}"));
    assert_eq!(hit.level, Level::Error);
}

/// A CONDITIONAL return is caught too — the `rts` carries a return edge whatever
/// branch reaches it.
#[test]
fn noreturn_with_conditional_rts_errors() {
    let src = "module m\n@noreturn\nproc f() clobbers() {\n\
               \ttst.b d0\n\
               \tbeq .skip\n\
               \trts\n\
               .skip:\n\
               \tjmp Somewhere\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(has_tag(&diags, "[noreturn.returns]"), "a conditional rts still returns: {diags:?}");
}

/// A `@noreturn` proc that runs off the end of the body fires — falling into
/// whatever follows is not leaving by a transfer or a loop.
#[test]
fn noreturn_fall_off_end_errors() {
    let src = "module m\n@noreturn\nproc f() clobbers() {\n\tnop\n}\n";
    let (_module, diags) = lower(src);
    assert!(has_tag(&diags, "[noreturn.returns]"), "a fall-off end returns to a successor: {diags:?}");
}

/// `@noreturn` takes no arguments — the form is a parse-time steering error.
#[test]
fn noreturn_takes_no_arguments() {
    let (_file, perrs) = parse_str("module m\n@noreturn(\"why\")\nproc f() clobbers() {\n\tjmp X\n}\n");
    assert!(
        perrs.iter().any(|d| d.message.contains("[attr.form]") && d.message.contains("noreturn")),
        "expected the no-args form error: {perrs:?}"
    );
}

/// `@noreturn` is accepted on an `extern proc` sig (the attrs channel).
#[test]
fn noreturn_on_extern_proc_parses() {
    let (_file, perrs) = parse_str("module m\n@noreturn\nextern proc Diverge () clobbers()\n");
    assert!(perrs.is_empty(), "an extern proc carries @noreturn: {perrs:?}");
}

// ---- bare-`sr` CCR advisory (noreturn-tail model) -------------------------

/// A bare-`preserves(sr)` proc whose mask round-trips but which writes flags
/// AFTER the restore is named by the warn-tier `[proc.ccr-advisory]` — the CCR
/// half the mask proof does not reach.
#[test]
fn ccr_advisory_names_post_restore_flag_traffic() {
    let src = "module m\n\
               proc f() clobbers(d0) preserves(sr) {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \tmove.w (sp)+, sr\n\
               \ttst.b d0\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    let hit = diags
        .iter()
        .find(|d| d.message.contains("[proc.ccr-advisory]"))
        .unwrap_or_else(|| panic!("expected [proc.ccr-advisory], got: {diags:?}"));
    assert_eq!(hit.level, Level::Warning);
}

/// A bare-`sr` proc whose every CCR effect sits inside the bracket draws NO
/// advisory (the honest bare adopter — Sound_PostByte's shape).
#[test]
fn ccr_advisory_silent_when_whole_body_bracketed() {
    let src = "module m\n\
               proc f() clobbers() preserves(sr) {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \tnop\n\
               \tmove.w (sp)+, sr\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(!has_tag(&diags, "[proc.ccr-advisory]"), "a fully-bracketed body is clean: {diags:?}");
}

/// A `jbra` to a LOCAL label is intra-proc flow, not a caller-visible leave —
/// the advisory does not false-positive on it (local-label awareness).
#[test]
fn ccr_advisory_silent_on_local_jump() {
    let src = "module m\n\
               proc f() clobbers() preserves(sr) {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \tjbra .done\n\
               .done:\n\
               \tmove.w (sp)+, sr\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(!has_tag(&diags, "[proc.ccr-advisory]"), "a local jump is not a leave: {diags:?}");
}

/// A DEBUG-shape `raise_error` rail inside the bracket does NOT false-positive
/// the advisory — the rail is `AssertDesugar`-authored and diverges, so its
/// internal `move.w sr, -(sp)` and `jmp (pages)` are skipped (Sound_Init shape).
#[test]
fn ccr_advisory_silent_on_debug_rail_inside_bracket() {
    let src = "module m\n\
               proc f() clobbers(d0) preserves(sr) {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \ttst.b d0\n\
               \tbne .ok\n\
               \traise_error \"bad\"\n\
               .ok:\n\
               \tmove.w (sp)+, sr\n\
               \trts\n\
               }\n";
    let (_module, diags) = lower_debug(src);
    assert!(
        !has_tag(&diags, "[proc.ccr-advisory]"),
        "an authored raise rail is not a caller-visible CCR effect: {diags:?}"
    );
}

// ---- @noreturn refinements (amended spec ad670db4): trailing-local + falls_into

/// M1 counterexample (a): a `@noreturn` proc with an unconditional transfer to a
/// TRAILING local label (`.out:` closes the body) — `Cfg::edges` hands it back as
/// `Edge::Defer` on 68k, but control runs off the end and returns. Refused.
#[test]
fn noreturn_trailing_local_transfer_is_a_fall_off() {
    let src = "module m\n@noreturn\nproc P () clobbers() {\n\
               \tmoveq #0, d0\n\
               \tbra .out\n\
               .spin:\n\
               \tjbra .spin\n\
               .out:\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        has_tag(&diags, "[noreturn.returns]"),
        "a transfer to a body-closing local label falls off: {diags:?}"
    );
}

/// M1 counterexample (b): the CCR walk must NOT read a `jbra .end` whose `.end:`
/// closes the body as intra-proc flow — control falls off, so the flags the
/// caller sees are the successor's. Was ACCEPTED when `is_local_label` cleared
/// trailing labels; `label_index` (None for a trailing label) refuses it.
#[test]
fn ccr_trailing_local_transfer_is_a_leave() {
    let src = "module m\nproc f() clobbers(sr.mask) preserves(sr.ccr) {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \tmove.w (sp)+, sr\n\
               \tjbra .end\n\
               .end:\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a jbra to a body-closing label is a leave, not intra-proc flow: {diags:?}"
    );
}

/// S2 (falls_into composition), honest polarity: a `@noreturn` proc that falls
/// into a successor which is ITSELF `@noreturn` is accepted.
#[test]
fn noreturn_falls_into_noreturn_successor_composes() {
    let src = "module m\n\
               @noreturn\nproc P () clobbers() falls_into Q {\n\tnop\n}\n\
               @noreturn\nproc Q () clobbers() {\n\tjbra Q\n}\n";
    let (_m, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[noreturn.returns]"),
        "a fall into a @noreturn successor composes: {diags:?}"
    );
}

/// S2, refused polarity: a `@noreturn` proc falling into a RETURNING successor
/// returns into it — refused.
#[test]
fn noreturn_falls_into_returning_successor_is_refused() {
    let src = "module m\n\
               @noreturn\nproc P () clobbers() falls_into Q {\n\tnop\n}\n\
               proc Q () clobbers() {\n\trts\n}\n";
    let (_m, diags) = lower(src);
    assert!(
        has_tag(&diags, "[noreturn.returns]"),
        "a fall into a returning successor returns: {diags:?}"
    );
}

/// S1: the bare-`sr` advisory reuses the ERROR check's tail refusal — a bare-`sr`
/// proc that falls into its successor is NAMED (its CCR is the successor's), not
/// silently green.
#[test]
fn ccr_advisory_fires_on_a_falls_into_bare_sr_proc() {
    let src = "module m\n\
               proc f() clobbers() preserves(sr) falls_into g {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \tmove.w (sp)+, sr\n\
               }\n\
               proc g() clobbers() { rts }\n";
    let (_m, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.ccr-advisory]"),
        "a bare-sr proc that falls into its successor is not silently green: {diags:?}"
    );
}

// ===========================================================================
// 68k preserves-through-tail credit — the sr.mask half (§2.4). An unconditional
// external tail to a callee that preserves the interrupt mask is CREDITED for a
// `preserves(sr.mask)` claim; anything else is refused, closing the vacuity a
// mask-claiming tail-only body would pass through. Explicit `sr.mask` also
// refuses the mnemonic-less tails (`falls_into` / run-off-end).
// ===========================================================================

/// CREDIT (holds): a mask-claiming proc whose body has no SR write of its own but
/// tails unconditionally into a sibling that preserves the mask is CREDITED — no
/// diagnostic. (The QueueDMA_Critical → QueueDMA_Deferrable shape, synthetic.)
#[test]
fn sr_mask_tail_into_preserving_sibling_is_credited() {
    let src = "module m\n\
               proc Sib() clobbers() preserves(sr.mask) {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \tmove.w (sp)+, sr\n\
               \trts\n\
               }\n\
               proc P() clobbers() preserves(sr.mask) {\n\
               \tjbra Sib\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "a tail into a mask-preserving sibling is credited: {diags:?}"
    );
}

/// REFUSAL (fires): the SAME shape, but the tail target does NOT preserve the
/// mask → the mask claim is refused (the vacuity hole a tail-only body would
/// otherwise pass through).
#[test]
fn sr_mask_tail_into_non_preserving_target_fires() {
    let src = "module m\n\
               proc Other() clobbers() {\n\
               \trts\n\
               }\n\
               proc P() clobbers() preserves(sr.mask) {\n\
               \tjbra Other\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "a tail into a non-mask-preserving target is refused: {diags:?}"
    );
}

/// The `Owner.label` exported-label tail (`jbra Owner.transfer`) credits when the
/// label is a SAVE-FIRST-BRACKET entry — the label precedes the owner's
/// `move.w sr, -(sp)` save, so a tail-entrant executes the save and the restore
/// pops what it pushed. This is the real QueueDMA layout (`.transfer` at the top
/// of the core, before the save).
#[test]
fn sr_mask_tail_into_owner_export_label_is_credited() {
    let src = "module m\n\
               proc Sib() clobbers() preserves(sr.mask) {\n\
               \tmove.w #$2700, d0\n\
               \texport .transfer:\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \tmove.w (sp)+, sr\n\
               \trts\n\
               }\n\
               proc P() clobbers() preserves(sr.mask) {\n\
               \tjbra Sib.transfer\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "a save-first-bracket entry label credits via the owner's mask contract: {diags:?}"
    );
}

/// The UNSOUND `Owner.label` layout the position check must catch: the label sits
/// AFTER the owner's `move.w sr, -(sp)` save, so a tail-entrant SKIPS the save and
/// the owner's `move.w (sp)+, sr` restore pops a word it never pushed (the `rts`
/// then returns to garbage). The credit REFUSES — the SR round-trip is a bracket
/// property that does not survive entry-point restriction.
#[test]
fn sr_mask_tail_into_owner_label_after_save_fires() {
    let src = "module m\n\
               proc Sib() clobbers() preserves(sr.mask) {\n\
               \tmove.w sr, -(sp)\n\
               \texport .transfer:\n\
               \tmove.w #$2700, sr\n\
               \tmove.w (sp)+, sr\n\
               \trts\n\
               }\n\
               proc P() clobbers() preserves(sr.mask) {\n\
               \tjbra Sib.transfer\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "a label past the owner's save is not a safe entry — the mask claim is refused: {diags:?}"
    );
}

/// EXPLICIT `sr.mask` + `falls_into` is refused — the mask is not provably the
/// caller's past a fall-through this slice cannot see (the mnemonic-less tail
/// refusal, now covering the explicit mask claim nothing else guarded).
#[test]
fn sr_mask_falls_into_is_refused() {
    let src = "module m\n\
               proc P() clobbers() preserves(sr.mask) falls_into Q {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w (sp)+, sr\n\
               }\n\
               proc Q() clobbers() { rts }\n";
    let (_m, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "an explicit sr.mask that falls into its successor is refused: {diags:?}"
    );
}

/// CONTROL: a mask-claiming proc that RETURNS (no tail) with a genuine SR
/// round-trip still passes — the tail machinery does not disturb the base slice.
#[test]
fn sr_mask_round_trip_returning_body_still_holds() {
    let src = "module m\n\
               proc P() clobbers() preserves(sr.mask) {\n\
               \tmove.w sr, -(sp)\n\
               \tmove.w #$2700, sr\n\
               \tmove.w (sp)+, sr\n\
               \trts\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "a returning round-trip body is unaffected by the tail credit: {diags:?}"
    );
}

/// A mask-claiming proc whose terminating tail DIVERGES into a `@noreturn`
/// handler is NOT refused — the diverging exit never returns, so it carries no
/// mask obligation (the same `@noreturn` composition the register credit uses;
/// the assert/raise rail case is covered by `diag_assert_vector`).
#[test]
fn sr_mask_noreturn_tail_is_not_refused() {
    let src = "module m\n\
               @noreturn\nextern proc Handler () clobbers()\n\
               proc P() clobbers() preserves(sr.mask) {\n\
               \tjbra Handler\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.preserves-sr-unbalanced]"),
        "a @noreturn tail carries no mask obligation: {diags:?}"
    );
}

// ===========================================================================
// §6 partial-width — `preserves(dN.w)` surface (facet spelling + obligation).
// Each refusal arm gets both polarities; the accepted `.w` facet is proven clean.
// ===========================================================================

/// POSITIVE polarity: a well-formed `preserves(d5.w)` whose body round-trips the
/// low word compiles with NO diagnostic — neither `-invalid` nor `-unverifiable`.
#[test]
fn word_facet_valid_roundtrip_is_clean() {
    let src = "module m\n\
               proc p() clobbers(d0) preserves(d5.w) {\n\
               \x20   move.w d5, -(sp)\n\
               \x20   moveq #0, d5\n\
               \x20   move.w (sp)+, d5\n\
               \x20   rts\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.preserves-invalid]")
            && !has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a valid word facet with a round-trip is clean: {diags:?}"
    );
}

/// NEGATIVE (non-vacuity): a `preserves(d5.w)` whose body does NOT round-trip the
/// low word is `[proc.preserves-unverifiable]`.
#[test]
fn word_facet_no_roundtrip_is_unverifiable() {
    let src = "module m\n\
               proc p() clobbers(d0) preserves(d5.w) {\n\
               \x20   moveq #0, d5\n\
               \x20   rts\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        has_tag(&diags, "[proc.preserves-unverifiable]"),
        "a word facet with no round-trip must refuse: {diags:?}"
    );
}

/// `.b` facet — REFUSED (demand-gated to `.w`, no byte witness).
#[test]
fn word_facet_b_spelling_refused() {
    let (_m, diags) = lower("module m\nproc p() clobbers(d0) preserves(d5.b) {\n    rts\n}\n");
    assert!(has_tag(&diags, "[proc.preserves-invalid]"), "`.b` facet refused: {diags:?}");
    assert!(
        diags.iter().any(|d| d.message.contains(".b") && d.message.contains("demand-gated")),
        "the `.b` arm names its own reason: {diags:?}"
    );
}

/// `aN.w` — REFUSED (address-register word writes sign-extend the full register).
#[test]
fn word_facet_address_register_refused() {
    let (_m, diags) = lower("module m\nproc p() clobbers(d0) preserves(a3.w) {\n    rts\n}\n");
    assert!(has_tag(&diags, "[proc.preserves-invalid]"), "`aN.w` refused: {diags:?}");
    assert!(
        diags.iter().any(|d| d.message.contains("sign-extend")),
        "the address arm names sign-extension: {diags:?}"
    );
}

/// `.l` — REFUSED (bare `dN` IS the full claim; one spelling per meaning).
#[test]
fn word_facet_l_spelling_refused() {
    let (_m, diags) = lower("module m\nproc p() clobbers(d0) preserves(d5.l) {\n    rts\n}\n");
    assert!(has_tag(&diags, "[proc.preserves-invalid]"), "`.l` facet refused: {diags:?}");
    assert!(
        diags.iter().any(|d| d.message.contains("full-width claim")),
        "the `.l` arm steers to the bare spelling: {diags:?}"
    );
}

/// An unknown facet (`.q`) — REFUSED (the only partial-width facet is `.w`).
#[test]
fn word_facet_unknown_spelling_refused() {
    let (_m, diags) = lower("module m\nproc p() clobbers(d0) preserves(d5.q) {\n    rts\n}\n");
    assert!(has_tag(&diags, "[proc.preserves-invalid]"), "`.q` facet refused: {diags:?}");
}

/// A word facet on a NON-register (`foo.w`) — REFUSED as an invalid register.
#[test]
fn word_facet_non_register_refused() {
    let (_m, diags) = lower("module m\nproc p() clobbers(d0) preserves(foo.w) {\n    rts\n}\n");
    assert!(has_tag(&diags, "[proc.preserves-invalid]"), "`foo.w` refused: {diags:?}");
}

/// `preserves(d5.w) clobbers(d5)` — the facet is a preserve; declaring the same
/// register clobbered is the overlap contradiction.
#[test]
fn word_facet_clobbers_overlap_refused() {
    let (_m, diags) = lower(
        "module m\nproc p() clobbers(d0, d5) preserves(d5.w) {\n\
         \x20   move.w d5, -(sp)\n    move.w (sp)+, d5\n    rts\n}\n",
    );
    assert!(
        has_tag(&diags, "[proc.preserves-clobbers-overlap]"),
        "word-preserved AND clobbered is a contradiction: {diags:?}"
    );
}

/// A register full-preserved AND word-preserved is redundant, not an error —
/// the full proof subsumes the word (the `& !declared` fold).
#[test]
fn word_facet_full_plus_word_is_not_an_error() {
    let src = "module m\n\
               proc p() clobbers(d0) preserves(d5, d5.w) {\n\
               \x20   move.l d5, -(sp)\n\
               \x20   moveq #0, d5\n\
               \x20   move.l (sp)+, d5\n\
               \x20   rts\n\
               }\n";
    let (_m, diags) = lower(src);
    assert!(
        !has_tag(&diags, "[proc.preserves-invalid]")
            && !has_tag(&diags, "[proc.preserves-clobbers-overlap]")
            && !has_tag(&diags, "[proc.preserves-unverifiable]"),
        "full+word on the same register is redundant, not an error: {diags:?}"
    );
}
