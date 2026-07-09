//! P1 (sound-migration T3, Task 1): the win-tab `dw sfx_winptr(Sfx_NN)` deferral,
//! IN A Z80 `phase` REGION (vma≠lma), pinned in exactly the production context.
//!
//! aeon's `sfx_blob_win_tab.asm` STAYS driver-side this arc (DSM.7/R3): it sits
//! inside main.asm's `cpu z80 / phase 08000h` block and reads the SFX blob
//! labels (which T3 moves to the `.emp` side) via
//! `dw sfx_winptr(Sfx_NN)` — where `sfx_winptr(addr)` is the AS function
//! `(((addr) & SFX_WIN_MASK) | SFX_WIN_BASE)` with `SFX_WIN_MASK = 32767`
//! ($7FFF) and `SFX_WIN_BASE = 32768` ($8000) (engine/sound/sound_sfx.asm:56-58).
//!
//! So each live entry is a COMPOUND `(Sfx_NN & $7FFF) | $8000` expr in a Z80
//! phase `dw`, with `Sfx_NN` an UNRESOLVED cross-seam 68k label. T0's dw
//! deferral (`db_dw_defer.rs`) proved compound `dw` exprs defer as `Value16Le`
//! in a bare `phase 0` Z80 context; this pins the SAME behavior in the exact
//! phase shape the win-tab uses (vma≠lma, non-zero phase base) with both a
//! synthetic mask and the LITERAL production mask/base, and proves the link-time
//! resolution folds to the windowed LE bytes.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::{
    Cpu, DataFragment, Expr, FixupKind, Fragment, Label, Section, SectionPlacement, SymbolTable,
};
use sigil_span::{Level, SourceId, Span};

fn sp() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

/// A 68k section PINNED at `lma` carrying `label` at offset 0 — gives a
/// cross-seam label a concrete VMA the AS Z80 phase table then references.
/// (Copied verbatim from `db_dw_defer.rs`'s `label_section`.)
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

/// The first fixup-bearing Data fragment of a module's first section.
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

/// The masked window tree `(Sym & mask) | base` — the shape `sfx_winptr(Sym)`
/// expands to. Mirrors the emp side's `winptr_target` in `lower_data.rs`.
fn winptr_tree(sym: &str, mask: i64, base: i64) -> Expr {
    use sigil_ir::expr::BinOp;
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
// The fixup SHAPE in a phase region — a synthetic mask ($7F00) to prove the
// full tree is carried (distinct low bits from the production $7FFF).
// ---------------------------------------------------------------------------

#[test]
fn phase_dw_compound_winptr_defers_as_value16le_with_full_tree() {
    // A Z80 `phase 08000h` region (vma≠lma, the win-tab's exact context) with
    // `dw (ExtSym & $7F00) | $8000` where ExtSym is undefined in-unit. Expect:
    // the module assembles, a 2-byte hole + a Value16Le fixup carrying the full
    // compound tree `(ExtSym & $7F00) | $8000`.
    let src = "        cpu z80\n        phase 08000h\n        dw (ExtSym & 7F00h) | 8000h\n";
    let module =
        assemble(src, &Options::default()).expect("phase-region compound dw defers, no error");

    let frag = first_fixup_frag(&module);
    assert_eq!(frag.bytes, vec![0x00, 0x00], "two placeholder bytes for the deferred dw");
    assert_eq!(frag.fixups.len(), 1);
    assert_eq!(
        frag.fixups[0].kind,
        FixupKind::Value16Le,
        "phase dw defers to the VALUE kind (verbatim LE), NOT an address kind"
    );
    assert_eq!(frag.fixups[0].offset, 0);
    assert_eq!(
        frag.fixups[0].target,
        winptr_tree("ExtSym", 0x7F00, 0x8000),
        "the fixup carries the FULL masked+or'd compound tree"
    );
}

#[test]
fn phase_dw_compound_winptr_links_to_windowed_le_bytes() {
    // LINK the phase-region dw against ExtSym at a known 68k address and assert
    // the resolved LE bytes equal `((addr & $7F00) | $8000)` little-endian.
    // ExtSym = $63AE8 (Sfx_33's plain-shape reference address): the fold is
    // ($63AE8 & $7F00) | $8000 = $3A00 | $8000 = $BA00 → LE bytes `00 BA`.
    let src = "        cpu z80\n        phase 08000h\n        dw (ExtSym & 7F00h) | 8000h\n";
    let module = assemble(src, &Options::default()).expect("defers");

    let defs = label_section("labeldefs", 0x0006_3AE8, "ExtSym");
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    assert_eq!(
        linked.section(&module.sections[0].name).unwrap().bytes,
        vec![0x00, 0xBA],
        "($63AE8 & $7F00) | $8000 = $BA00 → LE 00 BA"
    );
}

// ---------------------------------------------------------------------------
// The EXACT production shape — the literal `sfx_winptr` mask/base values
// (SFX_WIN_MASK=32767=$7FFF, SFX_WIN_BASE=32768=$8000), written DECIMAL exactly
// as the aeon source does (a `$`-hex literal inside an AS `function` body under
// `cpu z80` trips the expr parser — see sound_sfx.asm:52-56 — so the equates
// are decimal, and `dw sfx_winptr(Sfx_NN)` expands to `(Sfx_NN & 32767)|32768`).
// ---------------------------------------------------------------------------

#[test]
fn phase_dw_production_winptr_shape_defers_and_links() {
    // The production expression, spelled with the real decimal mask/base.
    let src =
        "        cpu z80\n        phase 08000h\n        dw (Sfx_33 & 32767) | 32768\n";
    let module = assemble(src, &Options::default()).expect("production winptr dw defers");

    // Shape: Value16Le carrying `(Sfx_33 & 32767) | 32768` (= $7FFF/$8000).
    let frag = first_fixup_frag(&module);
    assert_eq!(frag.bytes, vec![0x00, 0x00]);
    assert_eq!(frag.fixups.len(), 1);
    assert_eq!(frag.fixups[0].kind, FixupKind::Value16Le);
    assert_eq!(frag.fixups[0].offset, 0);
    assert_eq!(
        frag.fixups[0].target,
        winptr_tree("Sfx_33", 32767, 32768),
        "the full production window tree `(Sfx_33 & 32767) | 32768`"
    );

    // Link: Sfx_33 = $63AE8 → sfx_winptr($63AE8) = ($63AE8 & $7FFF) | $8000
    // = $3AE8 | $8000 = $BAE8 → LE bytes `E8 BA` (the value the win-tab's first
    // entry holds in the reference ROM — main.asm:147 phase block).
    let defs = label_section("labeldefs", 0x0006_3AE8, "Sfx_33");
    let mut all = module.sections.clone();
    all.push(defs);
    let resolved =
        sigil_link::resolve_layout(&all, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    assert_eq!(
        linked.section(&module.sections[0].name).unwrap().bytes,
        vec![0xE8, 0xBA],
        "sfx_winptr($63AE8) = $BAE8 → LE E8 BA (SfxBlobWinTab[0])"
    );
}

// ---------------------------------------------------------------------------
// NEGATIVE control — a genuinely-never-resolved phase-region `dw` symbol still
// fails LOUD at link (the deferral must not swallow an unresolvable reference).
// T0 left the unresolved-at-link path loud (see `db_dw_defer.rs`'s
// range-check test / `imm32_defer.rs`'s `branch_to_undefined_label_still_errors`);
// this pins that a win-tab entry whose blob label is never supplied ANYWHERE is
// a build failure, not a silent zero.
// ---------------------------------------------------------------------------

#[test]
fn phase_dw_never_defined_symbol_fails_loud_at_link() {
    let src =
        "        cpu z80\n        phase 08000h\n        dw (NeverDefined & 32767) | 32768\n";
    let module = assemble(src, &Options::default()).expect("assembles — defers to link");
    // No defining section: NeverDefined is unresolvable anywhere.
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout (no width ambiguity for a fixed-width dw)");
    let err = sigil_link::link(&resolved, &SymbolTable::new())
        .expect_err("link must fail: NeverDefined is never defined anywhere");
    // The compound fixup can't fold its target, so the link reports an
    // unresolved-target error for the deferred fixup (naming the section+offset
    // of the hole). It is LOUD — the deferral does NOT emit a silent zero. (A
    // BARE-symbol dw would name the symbol; a compound tree names the fixup
    // site instead — either way it is a build failure, which is the property
    // this negative control pins.)
    assert!(
        err.iter().any(|d| d.level == Level::Error
            && d.message.contains("unresolved")
            && d.message.contains("fixup")),
        "expected a loud unresolved-fixup error, got: {err:?}"
    );
}
