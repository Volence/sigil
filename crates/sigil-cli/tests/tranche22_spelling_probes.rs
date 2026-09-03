//! Tranche 22 — step-0 spelling probes for the compression-cluster port,
//! pinned as permanent tests against the independent AS front-end.
//!
//! Each probe replicates the REAL site's binding class (2026-07-15 probe rule);
//! see `docs/superpowers/notes/2026-07-24-t22-step0-design.md` for the design
//! decisions each outcome feeds.
//!
//! - P1 `falls_into` between two PUB procs with a NON-empty first body
//!   (the S4LZ_DecompressDict -> S4LZ_Decompress shared-body class): byte
//!   adjacency (no pad), byte parity vs the AS twin, and BOTH top-level names
//!   link-resolving from a second .emp module's `jbsr`s.
//! - P2 a pub-proc param typed `*DictBase` where `DictBase` is declared
//!   nowhere (the opaque named-pointee class the extern decl already uses —
//!   this proves the DEFINING-proc context).
//! - P3 `assert.w d4, hs, d1` — REGISTER compare-dest (cmp form) inside
//!   `if DEBUG == 1 {}` (the dict-range assert class).
//! - P4 link-time immediate EXPRESSIONS over a cross-seam value symbol
//!   (the `#CSELF_PAYLOAD_SIZE/2-1` class): bare whole-symbol vs bare-name
//!   arithmetic vs the `#extern("...")` arithmetic form.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_span::Level;

/// Assemble a `.asm` source through the AS front-end (68k) into raw sections.
fn as_sections(asm: &str) -> Vec<Section> {
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
    let module = assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble failed: {d:?}"));
    module.sections
}

/// Assemble a `.asm` source through the AS front-end (68k), link, flatten.
fn as_reference(asm: &str) -> Vec<u8> {
    let sections = as_sections(asm);
    let linked = sigil_link::link(&sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Lower an `.emp` source into raw sections (with optional comptime defines).
/// Panics on lower ERRORS; warnings pass (falls_into is the declared spelling
/// exactly so `[proc.undeclared-fallthrough]` must NOT be needed to silence).
fn emp_sections(emp: &str, defines: &[(&str, i128)]) -> Vec<Section> {
    let (file, pdiags) = parse_str(emp);
    assert!(
        !pdiags.iter().any(|d| d.level == Level::Error),
        "emp parse errors: {:?}",
        pdiags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        !ldiags.iter().any(|d| d.level == Level::Error),
        "emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
    module.sections
}

fn assert_byte_identical(reference: &[u8], candidate: &[u8], what: &str) {
    if reference == candidate {
        return;
    }
    let n = reference.len().min(candidate.len());
    if let Some(i) = (0..n).find(|&i| reference[i] != candidate[i]) {
        panic!(
            "{what}: first byte diff at {i:#x}: ref {:#04x} != cand {:#04x}\n ref = {:02X?}\n cand = {:02X?}",
            reference[i],
            candidate[i],
            &reference[i..(i + 8).min(reference.len())],
            &candidate[i..(i + 8).min(candidate.len())],
        );
    }
    panic!("{what}: length differ — ref {} vs cand {}", reference.len(), candidate.len());
}

// ---------------------------------------------------------------------------
// P1 — falls_into between two PUB procs, NON-empty first body, modelled on the
// S4LZ dict preamble falling into the shared decompressor body. The contracts
// below are a fixture for that SHAPE, not a copy of those procs' declarations.
// Byte parity + adjacency + both names visible to a second module's jbsr.
// ---------------------------------------------------------------------------

const EMP_FALLS_INTO_OWNER: &str = "\
module m in owner
pub proc Dict (a0: *u8, a1: *u8, a4: *u8, d4: u16) clobbers(d0-d3/a0/a2-a4) out(a1) falls_into Plain {
        adda.w  d4, a4
        suba.l  a1, a4
}
pub proc Plain (a0: *u8, a1: *u8) clobbers(d0-d3/a2-a3) out(a0, a1) {
        movea.l a1, a3
        move.w  (a0)+, d3
        move.w  (a0)+, (a1)+
        rts
}
";

const ASM_FALLS_INTO_TWIN: &str = "\
cpu 68000
Dict:
        adda.w  d4, a4
        suba.l  a1, a4
Plain:
        movea.l a1, a3
        move.w  (a0)+, d3
        move.w  (a0)+, (a1)+
        rts
";

const EMP_FALLS_INTO_CALLER: &str = "\
module c in caller
pub proc CallBoth () clobbers(d0-d4/a0-a4) {
        jbsr    Dict
        jbsr    Plain
        rts
}
";

fn pin_at(mut secs: Vec<Section>, lma: u32) -> Vec<Section> {
    for sec in secs.iter_mut() {
        sec.lma = lma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    secs
}

#[test]
fn falls_into_pub_pair_nonempty_body_matches_as() {
    let reference = as_reference(ASM_FALLS_INTO_TWIN);
    let sections = pin_at(emp_sections(EMP_FALLS_INTO_OWNER, &[]), 0);
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("owner resolve failed: {d:?}"));
    let linked =
        sigil_link::link(&resolved, &empty).unwrap_or_else(|d| panic!("owner link failed: {d:?}"));
    let rom = sigil_link::flatten(&linked, 0x00);
    // Byte parity implies adjacency: the Dict preamble is 4 bytes and Plain's
    // body must start at offset 4 with NO pad between the two proc bodies.
    assert_byte_identical(&reference, &rom, "falls_into pub pair (owner bytes)");
}

#[test]
fn falls_into_pub_pair_names_resolve_cross_module() {
    // Owner pinned at $2000, caller at $0 — both top-level names must resolve
    // at link from the second module, Dict at $2000 and Plain at $2004.
    let mut sections = pin_at(emp_sections(EMP_FALLS_INTO_CALLER, &[]), 0);
    sections.extend(pin_at(emp_sections(EMP_FALLS_INTO_OWNER, &[]), 0x2000));
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("cross-module resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &empty)
        .unwrap_or_else(|d| panic!("cross-module link failed: {d:?}"));
    let rom = sigil_link::flatten(&linked, 0x00);
    // jbsr lowers to bsr.w here: 6100 <disp>. Caller at 0: disp to $2000 from
    // pc=2 is $1FFE; disp to $2004 from pc=6 is $1FFE + 4 - 4 = $1FFE... spell
    // both out: bsr.w Dict at 0 -> disp = $2000 - 2 = $1FFE; bsr.w Plain at 4
    // -> disp = $2004 - 6 = $1FFE.
    assert_eq!(&rom[0..4], &[0x61, 0x00, 0x1F, 0xFE], "jbsr Dict displacement");
    assert_eq!(&rom[4..8], &[0x61, 0x00, 0x1F, 0xFE], "jbsr Plain displacement");
}

// ---------------------------------------------------------------------------
// P2 — opaque named pointee on a DEFINING proc's param (`a4: *DictBase` where
// DictBase is declared nowhere) — the extern-decl context accepts this today
// (tile_cache.emp:18); this proves the pub-proc context.
// ---------------------------------------------------------------------------

const EMP_OPAQUE_PTR: &str = "\
module m in owner
pub proc P (a0: *u8, a1: *u8, a4: *DictBase, d4: u16) clobbers(d0-d3/a0/a2-a4) out(a1) {
        adda.w  d4, a4
        move.w  (a0)+, (a1)+
        rts
}
";

#[test]
fn opaque_pointer_param_on_pub_proc_lowers() {
    let rom = link_pinned(emp_sections(EMP_OPAQUE_PTR, &[]));
    // adda.w d4,a4 = D8C4; move.w (a0)+,(a1)+ = 32D8; rts = 4E75.
    assert_eq!(rom, vec![0xD8, 0xC4, 0x32, 0xD8, 0x4E, 0x75], "proc body with *DictBase param");
}

/// Pin at 0, resolve, link, flatten (module-local probes).
fn link_pinned(sections: Vec<Section>) -> Vec<u8> {
    let sections = pin_at(sections, 0);
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("probe resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &empty)
        .unwrap_or_else(|d| panic!("probe link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

// ---------------------------------------------------------------------------
// P3 — assert with a REGISTER compare-dest: `assert.w d4, hs, d1` (the
// dict-range assert). DEBUG=1 must expand to the CCR-safe cmp form —
// move.w sr,-(sp) / cmp.w d1,d4 / bhs.w .skip / ... — and DEBUG=0 to zero
// bytes. (Full byte parity vs the AS macro tower is the s4lz byte gate's
// business; the probe proves the construct accepts the operand class and
// emits the correct compare head.)
// ---------------------------------------------------------------------------

const EMP_ASSERT_REG_DEST: &str = "\
module m in owner
pub proc P () clobbers(d0-d1) {
        if DEBUG == 1 {
            move.l  a3, d1
            sub.l   a2, d1
            assert.w d4, hs, d1
        }
        rts
}
";

#[test]
fn assert_register_dest_expands_cmp_form() {
    // The RaiseError blob targets the MDDBG handlers — feed them as value
    // carriers (any address; the probe checks the compare head, not the tail).
    let mut sections = pin_at(emp_sections(EMP_ASSERT_REG_DEST, &[("DEBUG", 1)]), 0x400);
    sections.extend(sigil_harness::test_support::assemble_equ_pairs(&[
        ("MDDBG__ErrorHandler", "$BE00"),
        ("MDDBG__ErrorHandler_PagesController", "$BE40"),
    ]));
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("assert probe resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &empty)
        .unwrap_or_else(|d| panic!("assert probe link failed: {d:?}"));
    let rom = sigil_link::flatten(&linked, 0x00);
    let bytes = &rom[0x400..];
    // move.l a3,d1 = 220B; sub.l a2,d1 = 928A; then the assert head:
    // move.w sr,-(sp) = 40E7; cmp.w d1,d4 = B841; bhs.w = 6400.
    assert_eq!(&bytes[0..4], &[0x22, 0x0B, 0x92, 0x8A], "gated loads");
    assert_eq!(&bytes[4..6], &[0x40, 0xE7], "assert CCR save");
    assert_eq!(&bytes[6..8], &[0xB8, 0x41], "cmp.w d1, d4 (register dest)");
    assert_eq!(&bytes[8..10], &[0x64, 0x00], "bhs.w skip");
}

#[test]
fn assert_register_dest_gates_to_zero_in_plain() {
    let bytes = link_pinned(emp_sections(EMP_ASSERT_REG_DEST, &[("DEBUG", 0)]));
    // Only the rts survives.
    assert_eq!(bytes, vec![0x4E, 0x75], "plain shape: rts only");
}

// ---------------------------------------------------------------------------
// P4 — link-time immediate expressions over a cross-seam VALUE symbol (the
// CSELF_PAYLOAD_SIZE class). The symbol is fed ONLY at link (an AS-side equ
// section, the assemble_equ_pairs seam the port tests use).
// ---------------------------------------------------------------------------

/// The value seam: CS = 744 (this build's CSELF_PAYLOAD_SIZE), fed AS-side.
fn cs_equ_sections() -> Vec<Section> {
    sigil_harness::test_support::assemble_equ_pairs(&[("CS", "744")])
}

const EMP_LINKIMM_WHOLE: &str = "\
module m in owner
pub proc P () clobbers(d0-d1) {
        move.w  #CS, d1
        rts
}
";

const EMP_LINKIMM_BARE_ARITH: &str = "\
module m in owner
pub proc P () clobbers(d0-d1) {
        move.w  #CS/2-1, d1
        rts
}
";

const EMP_LINKIMM_EXTERN_ARITH: &str = "\
module m in owner
pub proc P () clobbers(d0-d1) {
        move.w  #extern(\"CS\")/2-1, d1
        rts
}
";

fn link_with_cs(emp: &str) -> Vec<u8> {
    let mut sections = pin_at(emp_sections(emp, &[]), 0x400);
    sections.extend(cs_equ_sections());
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("linkimm resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &empty)
        .unwrap_or_else(|d| panic!("linkimm link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

#[test]
fn linkimm_bare_whole_symbol_defers_to_link() {
    let rom = link_with_cs(EMP_LINKIMM_WHOLE);
    // move.w #744, d1 = 323C 02E8.
    assert_eq!(&rom[0x400..0x404], &[0x32, 0x3C, 0x02, 0xE8], "move.w #CS (whole symbol)");
}

/// BOTH arithmetic forms defer to link and fold the same value — the BARE
/// spelling (`#CS/2-1`, twin-identical) is the shipped one; the extern() form
/// (core.emp:53's class) is pinned here as the proven equivalent.
#[test]
fn linkimm_bare_name_arith_vs_extern_form() {
    // The bare form — the shipped spelling (matches the AS twin verbatim).
    let rom = link_with_cs(EMP_LINKIMM_BARE_ARITH);
    // move.w #(744/2-1) = #371 = 323C 0173.
    assert_eq!(&rom[0x400..0x404], &[0x32, 0x3C, 0x01, 0x73], "move.w #CS/2-1 (bare)");

    // The extern() arithmetic form (the proven core.emp:53 class) folds the
    // same value.
    let rom = link_with_cs(EMP_LINKIMM_EXTERN_ARITH);
    assert_eq!(&rom[0x400..0x404], &[0x32, 0x3C, 0x01, 0x73], "move.w #extern(CS)/2-1");
}

// ---------------------------------------------------------------------------
// P6 (found at the t22 mixed arm) — assert diag-label hygiene ACROSS modules:
// two separately-lowered modules each minting low-numbered `$diagN$` labels
// must link together without symbol redefinition (the mint carries the
// module id, like every other hidden-local symbol).
// ---------------------------------------------------------------------------

const EMP_ASSERT_MOD_A: &str = "\
module a in amod
pub proc PA () clobbers(d0) {
        if DEBUG == 1 {
            moveq   #0, d0
            assert.w d0, eq
        }
        rts
}
";

const EMP_ASSERT_MOD_B: &str = "\
module b in bmod
pub proc PB () clobbers(d0) {
        if DEBUG == 1 {
            moveq   #1, d0
            assert.w d0, ne
        }
        rts
}
";

#[test]
fn assert_diag_labels_unique_across_modules() {
    let mut sections = pin_at(emp_sections(EMP_ASSERT_MOD_A, &[("DEBUG", 1)]), 0x400);
    sections.extend(pin_at(emp_sections(EMP_ASSERT_MOD_B, &[("DEBUG", 1)]), 0x800));
    sections.extend(sigil_harness::test_support::assemble_equ_pairs(&[
        ("MDDBG__ErrorHandler", "$BE00"),
        ("MDDBG__ErrorHandler_PagesController", "$BE40"),
    ]));
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("two-module assert resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &empty)
        .unwrap_or_else(|d| panic!("two-module assert link failed (diag-label collision?): {d:?}"));
    let rom = sigil_link::flatten(&linked, 0x00);
    assert!(rom.len() > 0x800, "both modules must place");
}
