//! R-T0.4: AS `db`/`dw` defer an UNRESOLVED expression (bare symbol OR compound)
//! to the linker as a general link-expr VALUE fixup — `Value8` for `db`,
//! `Value16Le` for `dw` — carrying the full parsed+qualified expr tree. This is
//! the consumption half of the .emp→.asm seam: aeon's `dac_sample_tab.asm`
//! (Z80) reads `SND_*` constants via `db BANK / dw PTR / dw LEN`, and those
//! constants move to `.emp` as link-folded equ symbols.
//!
//! Crucially `dw` no longer routes an unresolved symbol through `BankPtr16Le`,
//! an ADDRESS fixup kind (its 68k `BankPtr16Be` counterpart masks the windowed
//! low-16, and its apply arm truncates any out-of-range fold to `value as u16`
//! silently). `Value16Le` is the VALUE kind: it writes the folded value verbatim
//! after an unsigned-window range check, erroring LOUDLY on overflow. Window
//! masking, when wanted, belongs in SOURCE (aeon's `sfx_winptr()` macro writes
//! `(v & $7FFF) | $8000` explicitly, and that tree folds through `Value16Le`).
//!
//! NOTE: the current linker `BankPtr16Le` apply arm does NOT itself mask — it
//! writes `value as u16` verbatim LE — so for an IN-RANGE `dw SND_KICK_LEN`
//! ($057E) the OLD and NEW paths emit the SAME bytes `7E 05`. The recorded RED
//! for `dw` is therefore about the fixup KIND (address vs value) and the loud
//! range check, not a byte flip on in-range values; the byte flip the ruling
//! guards against is an OUT-OF-RANGE fold (silently truncated by BankPtr16Le,
//! loudly rejected by Value16Le). `db` had NO deferral path at all — it errored
//! `unresolved symbol ... in operand`; that is its RED.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::{
    Cpu, DataFragment, EquSym, Expr, FixupKind, Fragment, Label, Section, SectionPlacement,
    SymbolTable,
};
use sigil_span::{SourceId, Span};

fn sp() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

/// A section that DEFINES `equ`-style link symbols (no image bytes of its own
/// beyond a label anchor). Mirrors the relax.rs `equ_section` idiom.
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

/// A 68k section PINNED at `lma` carrying `label` at offset 0, used to give a
/// cross-seam label a concrete VMA the AS Z80 table then references.
fn label_section(name: &str, lma: u32, label: &str) -> Section {
    Section {
        name: name.into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma,
        labels: vec![Label { name: label.into(), offset: 0 }],
        fragments: vec![Fragment::Data(DataFragment {
            bytes: vec![0x00, 0x00],
            fixups: vec![],
            span: sp(),
        })],
        placement: SectionPlacement::Pinned,
        reserved_span: 0,
        group: None,
        bank: None,
        equ_syms: Vec::new(),
    }
}

fn equ(name: &str, expr: Expr) -> EquSym {
    EquSym { name: name.into(), expr, span: sp() }
}

// ---------------------------------------------------------------------------
// db — Value8
// ---------------------------------------------------------------------------

#[test]
fn db_unresolved_symbol_defers_as_value8() {
    // `db SND_KICK_BANK`, symbol undefined at assembly → one placeholder byte
    // $00 + a Value8 fixup targeting Sym("SND_KICK_BANK"). Then link against a
    // section defining SND_KICK_BANK = $B → final byte $0B.
    let src = "        cpu z80\n        phase 0\n        db SND_KICK_BANK\n";
    let module = assemble(src, &Options::default()).expect("db of undefined symbol defers, no error");

    // Emit shape: exactly one $00 byte + a Value8 fixup at offset 0 with the
    // bare symbol as target.
    let sec = &module.sections[0];
    let frag = sec
        .fragments
        .iter()
        .find_map(|f| match f {
            Fragment::Data(d) if !d.fixups.is_empty() => Some(d),
            _ => None,
        })
        .expect("a data fragment with a fixup");
    assert_eq!(frag.bytes, vec![0x00], "one placeholder byte");
    assert_eq!(frag.fixups.len(), 1);
    assert_eq!(frag.fixups[0].kind, FixupKind::Value8);
    assert_eq!(frag.fixups[0].offset, 0);
    assert_eq!(frag.fixups[0].target, Expr::Sym("SND_KICK_BANK".into()));

    // Link against a defining section: SND_KICK_BANK = $B → byte $0B.
    let defs = equ_defining_section(0x1000, vec![equ("SND_KICK_BANK", Expr::Int(0xB))]);
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let tbl = linked.section(&sec.name).expect("Z80 table section");
    assert_eq!(tbl.bytes, vec![0x0B], "SND_KICK_BANK=$B folded verbatim");
}

#[test]
fn db_unresolved_compound_defers() {
    // `db BANKSYM+1` → Value8 with the full compound tree (was a hard error).
    let src = "        cpu z80\n        phase 0\n        db BANKSYM+1\n";
    let module = assemble(src, &Options::default()).expect("db of compound expr defers");

    let sec = &module.sections[0];
    let frag = sec
        .fragments
        .iter()
        .find_map(|f| match f {
            Fragment::Data(d) if !d.fixups.is_empty() => Some(d),
            _ => None,
        })
        .expect("a data fragment with a fixup");
    assert_eq!(frag.bytes, vec![0x00]);
    assert_eq!(frag.fixups[0].kind, FixupKind::Value8);
    // The tree is `BANKSYM + 1`.
    assert_eq!(
        frag.fixups[0].target,
        Expr::Binary {
            op: sigil_ir::expr::BinOp::Add,
            lhs: Box::new(Expr::Sym("BANKSYM".into())),
            rhs: Box::new(Expr::Int(1)),
        }
    );

    // Link: BANKSYM = $0A → $0A + 1 = $0B.
    let defs = equ_defining_section(0x1000, vec![equ("BANKSYM", Expr::Int(0x0A))]);
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    assert_eq!(linked.section(&sec.name).unwrap().bytes, vec![0x0B]);
}

#[test]
fn db_value8_range_check_is_loud_over_ff() {
    // A folded db value > $FF trips the Value8 unsigned-window range check at
    // link (loud error, not a silent truncation). Pins the Plan-7-#7 apply arm.
    let src = "        cpu z80\n        phase 0\n        db BIG\n";
    let module = assemble(src, &Options::default()).expect("defers");
    let defs = equ_defining_section(0x1000, vec![equ("BIG", Expr::Int(0x1_23))]);
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let err = sigil_link::link(&resolved, &SymbolTable::new())
        .expect_err("value 0x123 does not fit an 8-bit cell");
    assert!(
        err.iter().any(|d| d.message.contains("value") && d.message.contains("8-bit")),
        "expected a loud value-out-of-range error, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// dw — Value16Le (verbatim, NO window mask)
// ---------------------------------------------------------------------------

#[test]
fn dw_unresolved_symbol_defers_verbatim_le() {
    // `dw SND_KICK_LEN` with SND_KICK_LEN=$057E at link → bytes `7E 05` (verbatim
    // little-endian). Under the OLD `BankPtr16Le` rule this emitted `7E 85` (the
    // `(v & $7FFF) | $8000` mask forced bit 15). That silent mask is the bug this
    // ruling removes.
    let src = "        cpu z80\n        phase 0\n        dw SND_KICK_LEN\n";
    let module = assemble(src, &Options::default()).expect("dw of undefined symbol defers");

    let sec = &module.sections[0];
    let frag = sec
        .fragments
        .iter()
        .find_map(|f| match f {
            Fragment::Data(d) if !d.fixups.is_empty() => Some(d),
            _ => None,
        })
        .expect("a data fragment with a fixup");
    assert_eq!(frag.bytes, vec![0x00, 0x00], "two placeholder bytes");
    assert_eq!(frag.fixups[0].kind, FixupKind::Value16Le, "verbatim LE, NOT BankPtr16Le");
    assert_eq!(frag.fixups[0].target, Expr::Sym("SND_KICK_LEN".into()));

    let defs = equ_defining_section(0x1000, vec![equ("SND_KICK_LEN", Expr::Int(0x057E))]);
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    assert_eq!(
        linked.section(&sec.name).unwrap().bytes,
        vec![0x7E, 0x05],
        "verbatim LE — NOT 7E 85 (the old BankPtr16Le mask)"
    );
}

#[test]
fn dw_unresolved_compound_defers_with_tree() {
    // `dw (SomeLabel & $7FFF) | $8000` with SomeLabel a cross-seam 68k label at
    // $58000 → bytes `00 80`: the mask is written in SOURCE and folded at link.
    // ($58000 & $7FFF) | $8000 = 0 | $8000 = $8000 → LE `00 80`. Under the old
    // rule a compound `dw` expr was a hard error.
    let src =
        "        cpu z80\n        phase 0\n        dw (SomeLabel & 7FFFh) | 8000h\n";
    let module = assemble(src, &Options::default()).expect("dw of compound expr defers");

    let sec = &module.sections[0];
    let frag = sec
        .fragments
        .iter()
        .find_map(|f| match f {
            Fragment::Data(d) if !d.fixups.is_empty() => Some(d),
            _ => None,
        })
        .expect("a data fragment with a fixup");
    assert_eq!(frag.bytes, vec![0x00, 0x00]);
    assert_eq!(frag.fixups[0].kind, FixupKind::Value16Le);
    // The target is the full masked tree `(SomeLabel & $7FFF) | $8000`.
    assert_eq!(
        frag.fixups[0].target,
        Expr::Binary {
            op: sigil_ir::expr::BinOp::Or,
            lhs: Box::new(Expr::Binary {
                op: sigil_ir::expr::BinOp::And,
                lhs: Box::new(Expr::Sym("SomeLabel".into())),
                rhs: Box::new(Expr::Int(0x7FFF)),
            }),
            rhs: Box::new(Expr::Int(0x8000)),
        }
    );

    let defs = label_section("labeldefs", 0x5_8000, "SomeLabel");
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    assert_eq!(
        linked.section(&sec.name).unwrap().bytes,
        vec![0x00, 0x80],
        "($58000 & $7FFF) | $8000 = $8000 → LE 00 80"
    );
}
