//! Spec-review gap (sound-migration T3): pin `Assembler::partial_fold`'s
//! env-equ BAKING on ALL THREE deferral arms (dw → Value16Le, db → Value8,
//! imm32 → Value32Be).
//!
//! `partial_fold` (eval.rs) is the "bake what you can" step on a deferred
//! fixup's target: every env-resolvable subterm (an AS `=`/`equ` symbol the
//! linker's section-label table cannot see) is collapsed to `Expr::Int` HERE,
//! leaving ONLY the genuinely-external leaf as `Expr::Sym` for the linker to
//! resolve. The production driver of this is `sfx_winptr(addr) = (addr &
//! SFX_WIN_MASK) | SFX_WIN_BASE`, where the mask/base are env `=` equs.
//!
//! `phase_dw_winptr_defer.rs` proves the dw arm's SHAPE — but all its target-
//! tree assertions use LITERAL masks (`$7FFF`/`$8000` spelled inline), so they
//! would STILL PASS if `partial_fold` were a no-op (the literal is already an
//! `Expr::Int` in the parsed tree). The tests here close that gap: the masks
//! are AS-env `=` symbols, so a deferred target where the mask stayed `Sym`
//! (an un-baked no-op fold) is DISTINGUISHABLE from one where it was baked to
//! `Int`. These are the non-tautological falsification of `partial_fold`: with
//! the fold neutered they FAIL (the equ leaks as `Sym`); with it live they
//! pass (the equ is baked, only the true cross-seam leaf survives).

use sigil_frontend_as::{assemble, Options};
use sigil_ir::{
    expr::BinOp, Cpu, DataFragment, EquSym, Expr, FixupKind, Fragment, Label, Section,
    SectionPlacement, SymbolTable,
};
use sigil_span::{SourceId, Span};

fn sp() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

/// The first fixup-bearing Data fragment across a module's sections (NOT
/// hardcoded to `sections[0]`: Task B1 (seam re-eval) made AS-side `equ`/`=`
/// export a link-level `EquSym`, which — like `define_label` already did —
/// opens a carrier section on demand. A source with an equate BEFORE its
/// first `phase`/`org`/instruction (as every test below has: `WIN_MASK =`/
/// `MASK7 =`/`OFFS =` precede `phase`) now opens an empty leading `sec0`
/// equ-only section ahead of the real one, so the real fragment can land in
/// `sections[1]`. Empty sections carry no fragments and are dropped before
/// link/emission (see eval.rs's `dedup_section_names` doc) — searching ALL
/// sections is the correct, convention-matching fix, not a workaround.
fn first_fixup_frag(module: &sigil_ir::Module) -> &DataFragment {
    module
        .sections
        .iter()
        .flat_map(|s| s.fragments.iter())
        .find_map(|f| match f {
            Fragment::Data(d) if !d.fixups.is_empty() => Some(d),
            _ => None,
        })
        .expect("a data fragment with a fixup")
}

/// The linked section that actually carries emitted bytes — i.e. skip a
/// leading empty equ-only auto-section (see `first_fixup_frag`'s doc). NOT a
/// name lookup (`LinkedImage::section` returns the FIRST name match, and an
/// equ-only `sec0` and a same-LMA real `sec0` — both auto-named from LMA —
/// can share the bare name post-dedup, since `dedup_section_names` only
/// disambiguates among NON-EMPTY sections; the empty one keeps the bare name
/// too and sorts first). Find by content instead.
fn real_section_bytes(linked: &sigil_link::LinkedImage) -> &[u8] {
    linked
        .sections
        .iter()
        .find(|s| !s.bytes.is_empty())
        .map(|s| s.bytes.as_slice())
        .expect("a linked section with emitted bytes")
}

/// A 68k section PINNED at `lma` carrying `label` at offset 0 — gives a
/// cross-seam label a concrete VMA. (Mirrors `phase_dw_winptr_defer.rs`.)
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

/// A section that DEFINES `equ`-style link symbols (no image bytes beyond a
/// label anchor). Mirrors `imm32_defer.rs`'s `equ_defining_section`.
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

/// The masked window tree `(Sym & mask) | base` — the BAKED shape a deferred
/// `sfx_winptr(Sym)` target must have after `partial_fold`: mask/base are
/// `Expr::Int` (env equs collapsed), the leaf still `Expr::Sym`. (Mirrors
/// `phase_dw_winptr_defer.rs`'s `winptr_tree`.)
fn winptr_tree(sym: &str, mask: i64, base: i64) -> Expr {
    Expr::Binary {
        op: BinOp::Or,
        lhs: Box::new(Expr::Binary {
            op: BinOp::And,
            lhs: Box::new(Expr::Sym(sym.into())),
            rhs: Box::new(Expr::Int(mask)),
        }),
        rhs: Box::new(Expr::Int(base)),
    }
}

// ---------------------------------------------------------------------------
// dw arm (Value16Le) — `dw (ExtSym & WIN_MASK) | WIN_BASE` with WIN_MASK/BASE
// as AS-env `=` equs. The production `sfx_winptr()` shape, but with the mask/
// base spelled as SYMBOLS (not literals) so a no-op fold is observable.
// ---------------------------------------------------------------------------

#[test]
fn dw_env_equ_mask_and_base_are_baked_to_int() {
    // WIN_MASK/WIN_BASE are AS `=` env symbols; ExtSym is the cross-seam leaf.
    // After `partial_fold` the deferred target must be `(Sym(ExtSym) & Int(32767))
    // | Int(32768)` — the equs BAKED, only ExtSym still `Sym`.
    let src = "        cpu z80\n\
               WIN_MASK = 7FFFh\n\
               WIN_BASE = 8000h\n\
               \x20       phase 08000h\n\
               \x20       dw (ExtSym & WIN_MASK) | WIN_BASE\n";
    let module = assemble(src, &Options::default())
        .expect("dw with env-equ masks defers, no error");

    let frag = first_fixup_frag(&module);
    assert_eq!(frag.bytes, vec![0x00, 0x00], "two placeholder bytes for the deferred dw");
    assert_eq!(frag.fixups.len(), 1);
    assert_eq!(frag.fixups[0].kind, FixupKind::Value16Le);
    assert_eq!(frag.fixups[0].offset, 0);
    assert_eq!(
        frag.fixups[0].target,
        winptr_tree("ExtSym", 0x7FFF, 0x8000),
        "the env-equ mask ($7FFF) and base ($8000) must be BAKED to Int, only ExtSym stays Sym"
    );
}

#[test]
fn dw_env_equ_masked_target_links_to_windowed_le_bytes() {
    // Link with ExtSym supplied: sfx_winptr($63AE8) = ($63AE8 & $7FFF) | $8000
    // = $3AE8 | $8000 = $BAE8 → LE bytes `E8 BA`.
    let src = "        cpu z80\n\
               WIN_MASK = 7FFFh\n\
               WIN_BASE = 8000h\n\
               \x20       phase 08000h\n\
               \x20       dw (ExtSym & WIN_MASK) | WIN_BASE\n";
    let module = assemble(src, &Options::default()).expect("defers");

    let defs = label_section("labeldefs", 0x0006_3AE8, "ExtSym");
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    assert_eq!(
        real_section_bytes(&linked),
        vec![0xE8, 0xBA],
        "($63AE8 & $7FFF) | $8000 = $BAE8 → LE E8 BA",
    );
}

// ---------------------------------------------------------------------------
// db arm (Value8) — same bake-the-env-equ property at width 1.
// ---------------------------------------------------------------------------

#[test]
fn db_env_equ_mask_is_baked_to_int() {
    // `db ExtSym & MASK7` with MASK7 an env `=` equ. After partial_fold the
    // target is `Sym(ExtSym) & Int(127)` — the equ baked, the leaf still Sym.
    let src = "        cpu z80\n\
               MASK7 = 7Fh\n\
               \x20       phase 0\n\
               \x20       db ExtSym & MASK7\n";
    let module = assemble(src, &Options::default())
        .expect("db with env-equ mask defers, no error");

    let frag = first_fixup_frag(&module);
    assert_eq!(frag.bytes, vec![0x00], "one placeholder byte for the deferred db");
    assert_eq!(frag.fixups.len(), 1);
    assert_eq!(frag.fixups[0].kind, FixupKind::Value8);
    assert_eq!(frag.fixups[0].offset, 0);
    assert_eq!(
        frag.fixups[0].target,
        Expr::Binary {
            op: BinOp::And,
            lhs: Box::new(Expr::Sym("ExtSym".into())),
            rhs: Box::new(Expr::Int(0x7F)),
        },
        "the env-equ mask ($7F) must be BAKED to Int, only ExtSym stays Sym",
    );
}

#[test]
fn db_env_equ_masked_target_links_to_byte() {
    // Link: ExtSym = $63AE8 → ($63AE8 & $7F) = $68 → single byte $68.
    let src = "        cpu z80\n\
               MASK7 = 7Fh\n\
               \x20       phase 0\n\
               \x20       db ExtSym & MASK7\n";
    let module = assemble(src, &Options::default()).expect("defers");

    let defs = equ_defining_section(0x1000, vec![equ("ExtSym", Expr::Int(0x0006_3AE8))]);
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    assert_eq!(
        real_section_bytes(&linked),
        vec![0x68],
        "($63AE8 & $7F) = $68",
    );
}

// ---------------------------------------------------------------------------
// imm32 arm (Value32Be) — `movea.l #ExtSym+OFFS, a0` with OFFS an env `=` equ.
// ---------------------------------------------------------------------------

#[test]
fn imm32_env_equ_offset_is_baked_to_int() {
    // `movea.l #ExtSym+OFFS, a0` with OFFS an env `=` equ ($10). After
    // partial_fold the deferred target is `Sym(ExtSym) + Int(16)` — OFFS baked.
    let src = "        cpu 68000\n\
               OFFS = 10h\n\
               \x20       phase 0\n\
               \x20       movea.l #ExtSym+OFFS, a0\n";
    let module = assemble(src, &Options::default())
        .expect("movea.l with env-equ offset defers, no error");

    let frag = first_fixup_frag(&module);
    // movea.l #imm,a0 opcode 0x207C, then a 4-byte hole.
    assert_eq!(frag.bytes, vec![0x20, 0x7C, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(frag.fixups.len(), 1);
    assert_eq!(frag.fixups[0].kind, FixupKind::Value32Be);
    assert_eq!(frag.fixups[0].offset, 2, "hole starts right after the 2-byte opcode");
    assert_eq!(
        frag.fixups[0].target,
        Expr::Binary {
            op: BinOp::Add,
            lhs: Box::new(Expr::Sym("ExtSym".into())),
            rhs: Box::new(Expr::Int(0x10)),
        },
        "the env-equ offset ($10) must be BAKED to Int, only ExtSym stays Sym",
    );
}

#[test]
fn imm32_env_equ_offset_target_links_big_endian() {
    // Link: ExtSym = $63AE0 → $63AE0 + $10 = $63AF0 → big-endian in the hole.
    let src = "        cpu 68000\n\
               OFFS = 10h\n\
               \x20       phase 0\n\
               \x20       movea.l #ExtSym+OFFS, a0\n";
    let module = assemble(src, &Options::default()).expect("defers");

    let defs = equ_defining_section(0x1000, vec![equ("ExtSym", Expr::Int(0x0006_3AE0))]);
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    assert_eq!(
        real_section_bytes(&linked),
        vec![0x20, 0x7C, 0x00, 0x06, 0x3A, 0xF0],
        "ExtSym=$63AE0 + $10 = $63AF0 folded verbatim big-endian",
    );
}
