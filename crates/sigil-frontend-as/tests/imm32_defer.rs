//! R3 (sound-migration T2, Task 3): AS `movea.l #expr,aN` / `move.l #expr,dN`
//! defer an UNRESOLVED 32-bit immediate operand (bare symbol OR compound) to
//! the linker as a general link-expr VALUE fixup (`Value32Be`) instead of the
//! `unresolved symbol ... in operand` hard error — the same treatment T0 gave
//! `db`/`dw` (see `db_dw_defer.rs`). This is the consumption half of the
//! MT-bank cross-seam: `sound_api.asm` reads `SongTable`/`SongPatchTable`
//! (moving to `.emp`) via `movea.l #SongTable, a0`.
//!
//! Deferral is scoped to LONG immediates only: `moveq` (imm8, packed in the
//! opcode itself, no extension word), 16-bit immediates, and branch targets
//! are untouched and still fail loudly on a genuinely unresolved symbol.
//!
//! Kind choice: `dc.l`'s OWN unresolved arm (`directive_dc_l`) defers ONLY a
//! bare `Expr::Sym` via the ADDRESS kind `Abs32Be` and hard-errors any
//! compound ("unresolved long expression") — by design (R-T0.4's asymmetry
//! note: `dc.l`/`dc.w` were deliberately NOT migrated to the general `Value*`
//! deferral). Since R3 explicitly wants compounds to defer too (mirroring
//! `db`/`dw`'s "ANY unresolved expression" rule), the correct reused kind is
//! the general `Value32Be` (added in a8b0f63 alongside `Value8`/`Value16*`,
//! and already production-exercised by the .emp frontend for 68k 4-byte
//! value cells — `sigil-frontend-emp/src/lower/data.rs`'s `value_fixup_kind`)
//! — NOT `Abs32Be`. `Value32Be` writes the folded value verbatim big-endian
//! after an unsigned-window range check (`0 <= v < 2^32`), exactly the
//! `db`/`dw` Value-kind semantics at width 4.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::{
    Cpu, DataFragment, EquSym, Expr, FixupKind, Fragment, Section, SectionPlacement, SymbolTable,
};
use sigil_span::{SourceId, Span};

fn sp() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

/// A section that DEFINES `equ`-style link symbols (no image bytes of its own
/// beyond a label anchor). Mirrors `db_dw_defer.rs`'s `equ_defining_section`.
fn equ_defining_section(lma: u32, equ_syms: Vec<EquSym>) -> Section {
    Section {
        name: "defs".into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma,
        labels: vec![],
        fragments: vec![],
        placement: SectionPlacement::Pinned,
        reserved_span: 0,
        group: None,
        bank: None,
        equ_syms,
    }
}

fn equ(name: &str, expr: Expr) -> EquSym {
    EquSym { name: name.into(), expr, span: sp() }
}

/// Find the first fixup-bearing Data fragment in a module's first section.
fn first_fixup_frag(module: &sigil_ir::Module) -> &DataFragment {
    module.sections[0]
        .fragments
        .iter()
        .find_map(|f| match f {
            Fragment::Data(d) if !d.fixups.is_empty() => Some(d),
            _ => None,
        })
        .expect("a data fragment with a fixup")
}

// ---------------------------------------------------------------------------
// movea.l #Sym, aN — defers
// ---------------------------------------------------------------------------

#[test]
fn movea_l_unresolved_symbol_defers_as_value32be() {
    // `movea.l #ExternalSym, a0` — ExternalSym undefined in-unit. Expect: the
    // module assembles (no hard error), the instruction fragment carries the
    // opcode word (`movea.l <ea>,a0`, ea=abs mode `111,100` = immediate) then
    // a 4-byte hole with a Value32Be fixup targeting Sym("ExternalSym").
    let src = "        cpu 68000\n        phase 0\n        movea.l #ExternalSym, a0\n";
    let module = assemble(src, &Options::default())
        .expect("movea.l of undefined symbol defers, no error");

    let frag = first_fixup_frag(&module);
    // movea.l #imm,a0 opcode: size_bits(.l=10)<<12 | an(0)<<9 | 0b001<<6 | src_mode(111)<<3 | src_reg(100)
    // = 0010 000 001 111 100 = 0x207C
    assert_eq!(
        frag.bytes,
        vec![0x20, 0x7C, 0x00, 0x00, 0x00, 0x00],
        "opcode word (2 bytes) + 4-byte hole for the deferred immediate"
    );
    assert_eq!(frag.fixups.len(), 1);
    assert_eq!(frag.fixups[0].kind, FixupKind::Value32Be);
    assert_eq!(frag.fixups[0].offset, 2, "hole starts right after the 2-byte opcode");
    assert_eq!(frag.fixups[0].target, Expr::Sym("ExternalSym".into()));

    // Link against a defining section: ExternalSym = $63AE0 → the hole becomes
    // that address big-endian.
    let defs = equ_defining_section(0x1000, vec![equ("ExternalSym", Expr::Int(0x0006_3AE0))]);
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let sec = linked.section(&module.sections[0].name).expect("section");
    assert_eq!(
        sec.bytes,
        vec![0x20, 0x7C, 0x00, 0x06, 0x3A, 0xE0],
        "ExternalSym=$63AE0 folded verbatim big-endian"
    );
}

// ---------------------------------------------------------------------------
// move.l #Sym, dN — defers
// ---------------------------------------------------------------------------

#[test]
fn move_l_unresolved_symbol_defers_as_value32be() {
    let src = "        cpu 68000\n        phase 0\n        move.l #ExternalSym, d0\n";
    let module = assemble(src, &Options::default())
        .expect("move.l of undefined symbol defers, no error");

    let frag = first_fixup_frag(&module);
    // move.l #imm,d0: size_bits(.l=10)<<12 | dst_reg(0)<<9 | dst_mode(000)<<6 | src_mode(111)<<3 | src_reg(100)
    // = 0010 000 000 111 100 = 0x203C
    assert_eq!(
        frag.bytes,
        vec![0x20, 0x3C, 0x00, 0x00, 0x00, 0x00],
        "opcode word (2 bytes) + 4-byte hole for the deferred immediate"
    );
    assert_eq!(frag.fixups.len(), 1);
    assert_eq!(frag.fixups[0].kind, FixupKind::Value32Be);
    assert_eq!(frag.fixups[0].offset, 2);
    assert_eq!(frag.fixups[0].target, Expr::Sym("ExternalSym".into()));

    let defs = equ_defining_section(0x1000, vec![equ("ExternalSym", Expr::Int(0x0006_5522))]);
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let sec = linked.section(&module.sections[0].name).expect("section");
    assert_eq!(sec.bytes, vec![0x20, 0x3C, 0x00, 0x06, 0x55, 0x22]);
}

// ---------------------------------------------------------------------------
// move.l #Sym+N, dN — a COMPOUND unresolved long immediate also defers
// ---------------------------------------------------------------------------

#[test]
fn move_l_unresolved_compound_defers_with_tree() {
    // `move.l #ExternalSym+4, d0` — mirrors db/dw's "ANY unresolved expression
    // (bare symbol OR compound) defers" rule (R-T0.4), applied to imm32.
    let src = "        cpu 68000\n        phase 0\n        move.l #ExternalSym+4, d0\n";
    let module = assemble(src, &Options::default())
        .expect("move.l of compound expr defers, no error");

    let frag = first_fixup_frag(&module);
    assert_eq!(frag.bytes, vec![0x20, 0x3C, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(frag.fixups.len(), 1);
    assert_eq!(frag.fixups[0].kind, FixupKind::Value32Be);
    assert_eq!(frag.fixups[0].offset, 2);
    assert_eq!(
        frag.fixups[0].target,
        Expr::Binary {
            op: sigil_ir::expr::BinOp::Add,
            lhs: Box::new(Expr::Sym("ExternalSym".into())),
            rhs: Box::new(Expr::Int(4)),
        }
    );

    let defs = equ_defining_section(0x1000, vec![equ("ExternalSym", Expr::Int(0x0006_3AE0))]);
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let sec = linked.section(&module.sections[0].name).expect("section");
    // $63AE0 + 4 = $63AE4
    assert_eq!(sec.bytes, vec![0x20, 0x3C, 0x00, 0x06, 0x3A, 0xE4]);
}

// ---------------------------------------------------------------------------
// NEGATIVE controls — imm8 (moveq)/imm16 (move.w)/branch targets still error
// ---------------------------------------------------------------------------

#[test]
fn moveq_unresolved_symbol_still_errors() {
    // `moveq` packs its imm8 into the opcode itself (no extension word) — an
    // entirely different encoding path from the long-immediate EA class R3
    // scopes to. An unresolved symbol here must still hard-error.
    let src = "        cpu 68000\n        phase 0\n        moveq #ExternalSym, d0\n";
    let err = assemble(src, &Options::default()).expect_err("moveq of undefined symbol errors");
    assert!(
        err.iter().any(|d| d.message.contains("unresolved symbol") && d.message.contains("ExternalSym")),
        "expected an unresolved-symbol error, got: {err:?}"
    );
}

#[test]
fn move_w_unresolved_symbol_still_errors() {
    // 16-bit immediate — R3 scopes deferral to LONG immediates only.
    let src = "        cpu 68000\n        phase 0\n        move.w #ExternalSym, d0\n";
    let err = assemble(src, &Options::default()).expect_err("move.w of undefined symbol errors");
    assert!(
        err.iter().any(|d| d.message.contains("unresolved symbol") && d.message.contains("ExternalSym")),
        "expected an unresolved-symbol error, got: {err:?}"
    );
}

#[test]
fn add_l_unresolved_symbol_still_errors() {
    // A NON-move mnemonic with a long immediate — R3 scopes deferral to the
    // `movea.l`/`move.l` class only; `add.l #Sym, d0` stays loud.
    let src = "        cpu 68000\n        phase 0\n        add.l #ExternalSym, d0\n";
    let err = assemble(src, &Options::default()).expect_err("add.l of undefined symbol errors");
    assert!(
        err.iter().any(|d| d.message.contains("unresolved symbol") && d.message.contains("ExternalSym")),
        "expected an unresolved-symbol error, got: {err:?}"
    );
}

#[test]
fn move_l_to_memory_dest_unresolved_symbol_still_errors() {
    // A MEMORY destination (`(a0)`) — outside the bare-`dN`/`aN` destination
    // shapes the deferral is scoped to; still loud.
    let src = "        cpu 68000\n        phase 0\n        move.l #ExternalSym, (a0)\n";
    let err = assemble(src, &Options::default())
        .expect_err("move.l to memory dest of undefined symbol errors");
    assert!(
        err.iter().any(|d| d.message.contains("unresolved symbol") && d.message.contains("ExternalSym")),
        "expected an unresolved-symbol error, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// RESOLVED long immediates are unaffected — sanity net for R3's "resolved
// operands keep the existing eager path" guarantee (the harness reference
// gates are the authoritative byte-neutrality proof; this pins the shape).
// ---------------------------------------------------------------------------

#[test]
fn movea_l_resolved_immediate_is_unaffected() {
    let src = "        cpu 68000\n        phase 0\n        movea.l #$63AE0, a0\n";
    let module = assemble(src, &Options::default()).expect("resolved movea.l assembles");
    let sec = &module.sections[0];
    let frag = sec
        .fragments
        .iter()
        .find_map(|f| match f {
            Fragment::Data(d) => Some(d),
            _ => None,
        })
        .expect("a data fragment");
    assert!(frag.fixups.is_empty(), "resolved immediate must not carry a deferred fixup");
    assert_eq!(frag.bytes, vec![0x20, 0x7C, 0x00, 0x06, 0x3A, 0xE0]);
}

#[test]
fn move_l_resolved_immediate_is_unaffected() {
    let src = "        cpu 68000\n        phase 0\n        move.l #$63AE0, d0\n";
    let module = assemble(src, &Options::default()).expect("resolved move.l assembles");
    let sec = &module.sections[0];
    let frag = sec
        .fragments
        .iter()
        .find_map(|f| match f {
            Fragment::Data(d) => Some(d),
            _ => None,
        })
        .expect("a data fragment");
    assert!(frag.fixups.is_empty(), "resolved immediate must not carry a deferred fixup");
    assert_eq!(frag.bytes, vec![0x20, 0x3C, 0x00, 0x06, 0x3A, 0xE0]);
}

#[test]
fn movea_l_in_unit_forward_reference_resolves_without_deferral() {
    // A forward-referenced label DEFINED LATER IN THE SAME UNIT is unresolved
    // on early multi-pass iterations (like any other forward ref) but resolves
    // by convergence — it must take the EAGER path on the converged pass (not
    // linger as a deferred fixup), since it never needed the linker's
    // cross-seam resolution at all. Both the deferred-shape (early passes)
    // and eager-shape (converged pass) fragments are 6 bytes, so the
    // multi-pass cursor/length math stays stable regardless of which arm
    // fires on any given pass.
    let src = "        cpu 68000\n        phase 0\n        movea.l #Later, a0\nLater:\n        dc.w 0\n";
    let module = assemble(src, &Options::default()).expect("in-unit forward ref resolves");
    let sec = &module.sections[0];
    let frag = sec
        .fragments
        .iter()
        .find_map(|f| match f {
            Fragment::Data(d) if d.bytes.len() == 6 => Some(d),
            _ => None,
        })
        .expect("the movea.l fragment");
    assert!(
        frag.fixups.is_empty(),
        "an in-unit-resolvable forward ref must NOT still carry a deferred fixup on the converged pass"
    );
    // Later is right after the 6-byte movea.l instruction, at phase-relative $6.
    assert_eq!(frag.bytes, vec![0x20, 0x7C, 0x00, 0x00, 0x00, 0x06]);
}

#[test]
fn branch_to_undefined_label_still_errors() {
    // A branch target is a wholly separate mechanism (a symbolic PcRel fixup
    // resolved at LINK time, not the front-end's poison/operand promotion), so
    // this must fail during resolve_layout/link rather than assemble — but it
    // must still fail loud, not silently emit a bogus displacement.
    let src = "        cpu 68000\n        phase 0\n        bra.w UndefinedLabel\n";
    let module = assemble(src, &Options::default()).expect("assembles — branch defers to link");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout (no branch-width ambiguity for bra.w)");
    let err = sigil_link::link(&resolved, &SymbolTable::new())
        .expect_err("link must fail: UndefinedLabel is never defined anywhere");
    assert!(
        err.iter().any(|d| d.message.contains("UndefinedLabel")),
        "expected an unresolved-symbol error naming UndefinedLabel, got: {err:?}"
    );
}
