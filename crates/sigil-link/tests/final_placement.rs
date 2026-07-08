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

