//! Task 4 (#7-pre): the link-time placement pass + placement⇄relaxation joint
//! fixpoint (rulings R7p.2–R7p.4). A `Chained` section's final base derives from
//! its predecessors' FINAL (post-relaxation) extents; `Pinned` sections don't
//! move; colliding pins are a loud link error. These tests construct `Section`s
//! by hand (like `two_section_ab.rs`) and drive `resolve_layout` directly: the
//! placement pass rewrites each section's `lma`, so we assert the RESOLVED lmas
//! (and the linked image) rather than what the section carried on input.

use sigil_ir::{
    Cpu, DataFragment, Expr, Fragment, Label, Section, SectionPlacement, SymbolTable, SymbolValue,
};
use sigil_span::{SourceId, Span};

fn sp() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

/// A `Chained` data section of `bytes.len()` bytes at `lma`, defining `label` at
/// offset 0 so predecessors' jmps can target it.
fn data_section(name: &str, lma: u32, label: &str, bytes: Vec<u8>) -> Section {
    Section {
        name: name.into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma,
        labels: vec![Label { name: label.into(), offset: 0 }],
        fragments: vec![Fragment::Data(DataFragment { bytes, fixups: vec![], span: sp() })],
        placement: SectionPlacement::Chained,
        reserved_span: 0,
        group: None,
        bank: None,
    }
}

/// A single-fragment data section with an explicit `bank`, `placement`, and
/// `reserved_span` — the T2 bank-placement builder. `reserved_span` defaults to
/// the byte length so the group cursor advances by the data extent.
fn bank_section(
    name: &str,
    lma: u32,
    bytes: Vec<u8>,
    bank: u32,
    placement: SectionPlacement,
) -> Section {
    let span = bytes.len() as u32;
    Section {
        name: name.into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma,
        labels: vec![],
        fragments: vec![Fragment::Data(DataFragment { bytes, fixups: vec![], span: sp() })],
        placement,
        reserved_span: span,
        group: None,
        bank: Some(bank),
    }
}

/// (a) A `Chained` section AFTER a `JmpJsrSym` section whose rung GROWS 4→6 →
/// the successor's placed base is `pred_base + 6`, NOT the baked `+ 4`. This is
/// the L-H.1 fix at the linker level: the chain cursor advances by FINAL size.
#[test]
fn chained_successor_follows_grown_predecessor_final_size() {
    // `code` is Pinned at 0 (group anchor); its lone `jmp Tail` targets a $8000
    // VMA symbol → abs.l (6 bytes). `data` is Chained; baked at lma 4 (baseline)
    // but reserved_span = 4 (the baked baseline extent). After placement it must
    // land at 6, and its Tail label at VMA $8000-ish... no: Tail's VMA follows
    // its placed lma. We instead target a HIGH stub so the jmp grows to abs.l.
    let mut stubs = SymbolTable::new();
    stubs.define("Hi", SymbolValue::Int(0x12_3456));
    let code = Section {
        name: "code".into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma: 0,
        labels: vec![],
        fragments: vec![Fragment::JmpJsrSym {
            is_jsr: false,
            target: Expr::Sym("Hi".into()),
            span: sp(),
        }],
        placement: SectionPlacement::Pinned,
        reserved_span: 4, // baked baseline (abs.w) extent
        group: None,
        bank: None,
    };
    let data = data_section("data", 4, "Tail", vec![0xDE, 0xAD, 0xBE, 0xEF]);
    let out = sigil_link::resolve_layout(&[code, data], &stubs, true).unwrap();
    // code stays pinned at 0; data's placed lma = 0 + max(reserved 4, final 6) = 6.
    assert_eq!(out[0].lma, 0, "pinned code anchor");
    assert_eq!(out[1].lma, 6, "chained data must follow the FINAL 6-byte code, not baked 4");
    // The whole image links: code = jmp abs.l (6 bytes), then data at lma 6.
    let linked = sigil_link::link(&out, &stubs).unwrap();
    let image = sigil_link::flatten(&linked, 0x00);
    assert_eq!(image, vec![0x4E, 0xF9, 0x00, 0x12, 0x34, 0x56, 0xDE, 0xAD, 0xBE, 0xEF]);
}

/// (b) MAX-SPAN provenance (the multi-module gap degeneracy, R7p.6): a section
/// whose `reserved_span` (6, the max/abs.l width the placer reserved) EXCEEDS its
/// final size (4, abs.w chosen) → the cursor advances by `max(6, 4) = 6`, so the
/// successor stays at `+6`. This is exactly the 2-byte gap `module_resolution.rs`
/// pins: over-reserved spacing must be preserved, never compacted.
#[test]
fn max_span_reservation_holds_gap_when_final_is_smaller() {
    // `code`'s jmp targets a LOW stub → abs.w (final 4), but reserved_span = 6
    // (the placer reserved the abs.l max). Successor must sit at +6, not +4.
    let mut stubs = SymbolTable::new();
    stubs.define("Lo", SymbolValue::Int(0x1000));
    let code = Section {
        name: "code".into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma: 0,
        labels: vec![],
        fragments: vec![Fragment::JmpJsrSym {
            is_jsr: false,
            target: Expr::Sym("Lo".into()),
            span: sp(),
        }],
        placement: SectionPlacement::Pinned,
        reserved_span: 6, // max-span (abs.l) reservation, à la placement_span()
        group: None,
        bank: None,
    };
    let data = data_section("data", 6, "Tail", vec![0xDE, 0xAD, 0xBE, 0xEF]);
    let out = sigil_link::resolve_layout(&[code, data], &stubs, true).unwrap();
    // jmp Lo is abs.w (final 4), but reserved 6 holds the gap → data at 6.
    assert_eq!(out[0].lma, 0);
    assert_eq!(out[1].lma, 6, "max-span reservation must hold the gap (degeneracy)");
    let linked = sigil_link::link(&out, &stubs).unwrap();
    let image = sigil_link::flatten(&linked, 0x00);
    // jmp abs.w (4 bytes) + a 2-byte gap (0x00 fill) + data at 6.
    assert_eq!(
        image,
        vec![0x4E, 0xF8, 0x10, 0x00, 0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF]
    );
}

/// (c) Two `Pinned` sections whose `[lma, lma+final)` ranges COLLIDE → a loud
/// link `Err` naming BOTH sections (R7p.4). Chained sections can't overlap by
/// construction; this catches mis-assigned pins.
#[test]
fn colliding_pins_are_a_loud_link_error() {
    let a = Section {
        name: "alpha".into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma: 0x100,
        labels: vec![],
        fragments: vec![Fragment::Data(DataFragment {
            bytes: vec![0x11, 0x22, 0x33, 0x44],
            fixups: vec![],
            span: sp(),
        })],
        placement: SectionPlacement::Pinned,
        reserved_span: 4,
        group: None,
        bank: None,
    };
    // `beta` pinned at 0x102 → [0x102, 0x106) intersects alpha's [0x100, 0x104).
    let b = Section {
        name: "beta".into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma: 0x102,
        labels: vec![],
        fragments: vec![Fragment::Data(DataFragment {
            bytes: vec![0x55, 0x66, 0x77, 0x88],
            fixups: vec![],
            span: sp(),
        })],
        placement: SectionPlacement::Pinned,
        reserved_span: 4,
        group: None,
        bank: None,
    };
    let err = sigil_link::resolve_layout(&[a, b], &SymbolTable::new(), true).unwrap_err();
    assert!(
        err.iter().any(|d| d.message.contains("alpha") && d.message.contains("beta")),
        "overlap error must name BOTH sections, got: {err:?}"
    );
}

/// (d) FIXPOINT interaction: growth caused BY re-placement. A jmp whose target
/// address only crosses the abs.w→abs.l boundary AFTER its section is moved by an
/// earlier chained growth. Convergence must land BOTH effects: the earlier jmp
/// grows, that pushes the later section past $7FFF, and the later jmp then grows
/// too — all in one joint fixpoint.
#[test]
fn placement_growth_feeds_relaxation_growth_to_a_joint_fixpoint() {
    // Group layout (single anonymous group, program order):
    //   s0 (Pinned @ 0):  jmp Hi  — Hi is high → grows to abs.l (4→6), pushing
    //                      everything after it up by 2.
    //   s1 (Chained):     Fill(0x7FFC bytes) — padding so s2's label `T` sits
    //                      right at the $8000 boundary.
    //   s2 (Chained):     defines T at its base; a `jmp T` in s3 targets it.
    //   s3 (Chained):     jmp T — T's VMA depends on s2's placed base. Tuned so
    //                      that with s0 at baseline (4) T is at $7FFE (abs.w), but
    //                      after s0 grows +2 T lands at $8000 (abs.l) → s3 grows.
    //
    // Baseline (s0 = 4): s1 base 4, s1 spans 0x7FFC → s2 base 0x8000... that is
    // already ≥ $8000. Simpler and exact: size the fill so that at baseline T is
    // BELOW $8000 and after the +2 shift T is AT $8000.
    //   Want baseline T (s2 base) = 0x7FFE, post-shift = 0x8000.
    //   s2 base = s0_final + s1_fill. Baseline s0_final = 4 → s1_fill = 0x7FFA.
    //   Post-shift s0_final = 6 → s2 base = 6 + 0x7FFA = 0x8000. ✓
    let s0 = Section {
        name: "s0".into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma: 0,
        labels: vec![],
        fragments: vec![Fragment::JmpJsrSym {
            is_jsr: false,
            target: Expr::Sym("Hi".into()),
            span: sp(),
        }],
        placement: SectionPlacement::Pinned,
        reserved_span: 4,
        group: None,
        bank: None,
    };
    let s1 = Section {
        name: "s1".into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma: 4,
        labels: vec![],
        fragments: vec![Fragment::Fill { value: 0, count: 0x7FFA, span: sp() }],
        placement: SectionPlacement::Chained,
        reserved_span: 0x7FFA,
        group: None,
        bank: None,
    };
    let s2 = Section {
        name: "s2".into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma: 0x7FFE,
        labels: vec![Label { name: "T".into(), offset: 0 }],
        fragments: vec![Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() })],
        placement: SectionPlacement::Chained,
        reserved_span: 2,
        group: None,
        bank: None,
    };
    let s3 = Section {
        name: "s3".into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma: 0x8000,
        labels: vec![],
        fragments: vec![Fragment::JmpJsrSym {
            is_jsr: false,
            target: Expr::Sym("T".into()),
            span: sp(),
        }],
        placement: SectionPlacement::Chained,
        reserved_span: 4,
        group: None,
        bank: None,
    };
    let mut stubs = SymbolTable::new();
    stubs.define("Hi", SymbolValue::Int(0x12_3456));
    let out = sigil_link::resolve_layout(&[s0, s1, s2, s3], &stubs, true).unwrap();
    // s0 grew to abs.l (6) → s1 at 6, s2 (`T`) at 6 + 0x7FFA = 0x8000, s3 at 0x8002.
    assert_eq!(out[0].lma, 0, "s0 pinned");
    assert_eq!(out[1].lma, 6, "s1 follows grown s0");
    assert_eq!(out[2].lma, 0x8000, "s2 (T) pushed to the $8000 boundary");
    assert_eq!(out[3].lma, 0x8002, "s3 follows s2");
    // T's VMA is now $8000, so s3's `jmp T` must have grown to abs.l (6 bytes) too.
    match &out[3].fragments[0] {
        Fragment::Data(d) => {
            assert_eq!(d.bytes.len(), 6, "jmp T must be abs.l once T crossed $8000");
            assert_eq!(&d.bytes[..2], &[0x4E, 0xF9]);
        }
        other => panic!("expected lowered abs.l jmp Data, got {other:?}"),
    }
    let linked = sigil_link::link(&out, &stubs).unwrap();
    // s3's jmp T resolves to T's VMA = $8000.
    assert_eq!(&linked.section("s3").unwrap().bytes, &[0x4E, 0xF9, 0x00, 0x00, 0x80, 0x00]);
}

// ---- #7-main: no-straddle bank placement (R7m.2) ---------------------------

/// A `Pinned` data section of `size` bytes at `lma`, the group anchor that
/// pushes a following chained section's cursor to `lma + size`.
fn pin_filler(name: &str, lma: u32, size: usize) -> Section {
    Section {
        name: name.into(),
        cpu: Cpu::M68000,
        vma_base: None,
        lma,
        labels: vec![],
        fragments: vec![Fragment::Data(DataFragment {
            bytes: vec![0xAA; size],
            fixups: vec![],
            span: sp(),
        })],
        placement: SectionPlacement::Pinned,
        reserved_span: size as u32,
        group: None,
        bank: None,
    }
}

/// (a) A `Chained` bank-$100 section whose cursor sits at $F8 with $10 bytes of
/// data → `[$F8, $108)` straddles the $100 boundary ($F8 in bank 0, $107 in bank
/// 1) → the base is BUMPED to the next multiple of $100 = $100.
#[test]
fn chained_bank_section_bumps_when_it_would_straddle() {
    // Pin a $F8-byte filler at 0 → the chained bank section's cursor is $F8.
    let filler = pin_filler("filler", 0, 0xF8);
    let banked =
        bank_section("dac_bank", 0xF8, vec![0xDE; 0x10], 0x100, SectionPlacement::Chained);
    let out = sigil_link::resolve_layout(&[filler, banked], &SymbolTable::new(), true).unwrap();
    assert_eq!(out[0].lma, 0, "filler pinned");
    assert_eq!(out[1].lma, 0x100, "chained bank section bumped to the $100 boundary");
}

/// (b) The SAME arrangement but only $8 bytes → `[$F8, $100)` fits entirely
/// before the boundary ($FF is the last byte, still bank 0) → NO bump. It stays
/// at $F8 (D7.2: bump ONLY when straddling; not aeon's always-align).
#[test]
fn chained_bank_section_stays_when_it_fits_before_boundary() {
    let filler = pin_filler("filler", 0, 0xF8);
    let banked =
        bank_section("dac_bank", 0xF8, vec![0xDE; 0x8], 0x100, SectionPlacement::Chained);
    let out = sigil_link::resolve_layout(&[filler, banked], &SymbolTable::new(), true).unwrap();
    assert_eq!(out[0].lma, 0, "filler pinned");
    assert_eq!(out[1].lma, 0xF8, "bank section that fits before the boundary stays put");
}

/// (c) A bank-$100 section holding $110 bytes → content larger than the bank →
/// unsatisfiable, "over by" Err naming the section.
#[test]
fn bank_section_over_bank_size_is_a_loud_error() {
    let banked =
        bank_section("dac_bank", 0, vec![0xDE; 0x110], 0x100, SectionPlacement::Chained);
    let err = sigil_link::resolve_layout(&[banked], &SymbolTable::new(), true).unwrap_err();
    assert!(
        err.iter().any(|d| d.message.contains("dac_bank") && d.message.contains("over by")),
        "over-bank error must name the section and say 'over by', got: {err:?}"
    );
}

/// (d) A `Pinned` bank-$100 section pinned ASTRIDE a boundary (lma $F8, $10 bytes
/// → `[$F8, $108)` crosses $100). Pins are NEVER moved (R7m.2) → the always-on
/// post-check Err names the section, its extent, and the boundary — not silently
/// bumped.
#[test]
fn pinned_bank_section_straddling_is_a_loud_error_not_moved() {
    let banked =
        bank_section("dac_bank", 0xF8, vec![0xDE; 0x10], 0x100, SectionPlacement::Pinned);
    let err = sigil_link::resolve_layout(&[banked], &SymbolTable::new(), true).unwrap_err();
    assert!(
        err.iter().any(|d| {
            d.message.contains("dac_bank")
                && d.message.contains("0xF8")
                && d.message.contains("0x100")
        }),
        "straddling-pin error must name the section, its extent, and the $100 boundary, got: {err:?}"
    );
}

/// (e) FIXPOINT interaction — what this test pins: a bank bump propagates
/// through the placement cursor WITHIN the fixpoint (the section after a bumped
/// section lands past the bump). Layout (one anonymous group):
///   s0 (Pinned @ 0):   $F8 filler bytes → cursor at $F8.
///   s1 (Chained, bank $100): $10 bytes defining `T` at its base → BUMPS to
///                      $100, so T's VMA is $100 (below $8000 → still abs.w).
///   s2 (Chained) follows s1 → its base must be $100 + $10 = $110, proving the
///                      bumped base (not the pre-bump $F8) drove the cursor.
/// The other half of the feedback loop — a placement move growing a rung, which
/// re-runs placement — is pinned by
/// `placement_growth_feeds_relaxation_growth_to_a_joint_fixpoint` above; a bump
/// enters that loop through the same `moved` flag, so the two tests compose to
/// cover bump→relaxation feedback.
#[test]
fn bank_bump_feeds_the_placement_fixpoint() {
    let s0 = pin_filler("s0", 0, 0xF8);
    let s1 =
        bank_section("s1_bank", 0xF8, vec![0xDE; 0x10], 0x100, SectionPlacement::Chained);
    let s2 = data_section("s2", 0x108, "T2", vec![0x11, 0x22, 0x33, 0x44]);
    let out = sigil_link::resolve_layout(&[s0, s1, s2], &SymbolTable::new(), true).unwrap();
    assert_eq!(out[0].lma, 0, "s0 pinned");
    assert_eq!(out[1].lma, 0x100, "s1 bumped to the $100 boundary");
    assert_eq!(out[2].lma, 0x110, "s2 follows the BUMPED s1 base ($100 + $10), not the pre-bump $F8");
}

