//! Tranche 24 — step-0 spelling probes for the children port, pinned as
//! permanent tests against the independent AS front-end.
//!
//! Each probe replicates the REAL site's binding class (2026-07-15 probe rule);
//! see `docs/superpowers/notes/2026-07-24-t24-step0-design.md` for the design
//! decisions each outcome feeds. children.asm carries NO link-time immediates
//! and NO comptime gating, so every probe here is the LOCAL INSTRUCTION-
//! ENCODING class (comptime-const operands where the real site reads the
//! constants twin) — the one link-time slot in the file (`jsr AllocDynamic`)
//! is the already-proven load_object class and needs no synthetic probe.
//!
//! - P1 `4(sp)` / `(sp)` stack-resident running position + `addq.w #8, sp`
//!   (CreateChild_Linked's hand-computed stack window, under a
//!   `movem.l d1-d5/a0` push).
//! - P2 `btst`/`bset #RF_XFLIP, render_flags(aN)` — a COMPTIME const in the
//!   bit-number slot of a memory byte operand (CreateChild_FlipAware).
//! - P3 memory-to-memory `move.l`/`move.w` with a displacement EA on BOTH
//!   sides (the parent→child mappings/art_tile inheritance, 8 sites).
//! - P4 indexed EAs: zero-displacement `(a0,d0.w)` and comptime-symbolic
//!   `FRAME_PIECE_COUNT(a0,d0.w)` (PopulateSpawnedPieceCount's frame walk).
//! - P6 (DEMANDED FEATURE, shipped in the loop's step-4 construct pass) — a
//!   struct-field displacement over a SPLICED base register inside a
//!   comptime-fn template (`Sst.field({sst})`). The sibling of the spliced
//!   INDEX-register gap: `peek_inner_reg` resolved the base register only from
//!   a LITERAL path, so both field-displacement forms (bare `f(aN)` and
//!   qualified `S.f(aN)`) fell through to comptime eval and died as "unknown
//!   name `Struct.field`" — which blocked every multi-instruction template that
//!   touches a typed record.
//! - P5 address-register word traffic: `move.w a0, parent_ptr(a2)`,
//!   `move.w a2, d3`, `movea.w d1, a1` (the sibling-chain link pointers,
//!   which live in the sign-extending $FFFFxxxx RAM window).

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_span::Level;

/// Assemble a `.asm` source through the AS front-end (68k), link, flatten.
fn as_reference(asm: &str) -> Vec<u8> {
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
    let sections: Vec<Section> =
        assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble failed: {d:?}")).sections;
    let linked = sigil_link::link(&sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Lower an `.emp` source, link, flatten. Panics on lower ERRORS.
fn emp_image(emp: &str) -> Vec<u8> {
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
        defines: Vec::new(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        !ldiags.iter().any(|d| d.level == Level::Error),
        "emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
    // Bare Bcc lowers to a RelaxLadder — resolve_layout must run before link.
    let mut sections = module.sections;
    for sec in sections.iter_mut() {
        sec.lma = 0;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("emp resolve failed: {d:?}"));
    let linked =
        sigil_link::link(&resolved, &empty).unwrap_or_else(|d| panic!("emp link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
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
    panic!("{what}: length differ, ref {} vs cand {}", reference.len(), candidate.len());
}

// ---------------------------------------------------------------------------
// P1 — the stack-resident running position (CreateChild_Linked): two longs
// pushed, read back through `4(sp)`/`(sp)` UNDER a `movem.l d1-d5/a0` push,
// advanced in place with `add.l dN, 4(sp)`, and released with `addq.w #8, sp`.
// ---------------------------------------------------------------------------

const EMP_STACK_WINDOW: &str = "\
module m in p1
pub proc P (a0: *u8, a2: *u8) clobbers(d0-d5, a0-a2) {
        move.l  $02(a0), -(sp)
        move.l  $06(a0), -(sp)
    .spawn_loop:
        movem.l d1-d5/a0, -(sp)
        movem.l (sp)+, d1-d5/a0
        move.l  4(sp), $02(a2)
        move.l  (sp), $06(a2)
        add.l   d0, 4(sp)
        add.l   d0, (sp)
        dbf     d5, .spawn_loop
        addq.w  #8, sp
        rts
}
";

const ASM_STACK_WINDOW: &str = "\
cpu 68000
P:
        move.l  $02(a0), -(sp)
        move.l  $06(a0), -(sp)
.spawn_loop:
        movem.l d1-d5/a0, -(sp)
        movem.l (sp)+, d1-d5/a0
        move.l  4(sp), $02(a2)
        move.l  (sp), $06(a2)
        add.l   d0, 4(sp)
        add.l   d0, (sp)
        dbf     d5, .spawn_loop
        addq.w  #8, sp
        rts
";

#[test]
fn stack_resident_running_position_matches_as() {
    assert_byte_identical(
        &as_reference(ASM_STACK_WINDOW),
        &emp_image(EMP_STACK_WINDOW),
        "P1 stack-window EAs (4(sp)/(sp)/addq sp)",
    );
}

// ---------------------------------------------------------------------------
// P2 — bit ops on a memory BYTE with a COMPTIME const bit number (the real
// site reads RF_XFLIP from the engine.constants twin, not a link extern).
// ---------------------------------------------------------------------------

const EMP_BIT_OPS: &str = "\
module m in p2
const RF_XFLIP = 1
pub proc P (a0: *u8, a2: *u8) clobbers(d4) {
        moveq   #0, d4
        btst    #RF_XFLIP, $0E(a0)
        beq     .no_flip
        moveq   #1, d4
    .no_flip:
        tst.w   d4
        beq     .rf_no_flip
        bset    #RF_XFLIP, $0E(a2)
    .rf_no_flip:
        rts
}
";

const ASM_BIT_OPS: &str = "\
cpu 68000
RF_XFLIP        = 1
P:
        moveq   #0, d4
        btst    #RF_XFLIP, $0E(a0)
        beq.s   .no_flip
        moveq   #1, d4
.no_flip:
        tst.w   d4
        beq.s   .rf_no_flip
        bset    #RF_XFLIP, $0E(a2)
.rf_no_flip:
        rts
";

#[test]
fn btst_bset_comptime_bit_on_memory_byte_matches_as() {
    assert_byte_identical(
        &as_reference(ASM_BIT_OPS),
        &emp_image(EMP_BIT_OPS),
        "P2 btst/bset comptime bit number on d16(An)",
    );
}

// ---------------------------------------------------------------------------
// P3 — memory-to-memory move with a displacement EA on BOTH sides (the
// parent→child mappings/art_tile inheritance; 8 sites in children.asm).
// ---------------------------------------------------------------------------

const EMP_MEM_TO_MEM: &str = "\
module m in p3
pub proc P (a0: *u8, a2: *u8) clobbers() {
        move.l  $10(a0), $10(a2)
        move.w  $14(a0), $14(a2)
        move.l  $02(a0), $02(a2)
        rts
}
";

const ASM_MEM_TO_MEM: &str = "\
cpu 68000
P:
        move.l  $10(a0), $10(a2)
        move.w  $14(a0), $14(a2)
        move.l  $02(a0), $02(a2)
        rts
";

#[test]
fn displacement_memory_to_memory_moves_match_as() {
    assert_byte_identical(
        &as_reference(ASM_MEM_TO_MEM),
        &emp_image(EMP_MEM_TO_MEM),
        "P3 d16(An) → d16(An) moves",
    );
}

// ---------------------------------------------------------------------------
// P4 — indexed EAs in PopulateSpawnedPieceCount's frame walk: the zero-
// displacement `(a0,d0.w)` table read and the comptime-symbolic
// `FRAME_PIECE_COUNT(a0,d0.w)` header read.
// ---------------------------------------------------------------------------

const EMP_INDEXED: &str = "\
module m in p4
const FRAME_PIECE_COUNT = 4
pub proc P (a0: *u8) clobbers(d0) {
        move.w  (a0,d0.w), d0
        move.w  FRAME_PIECE_COUNT(a0,d0.w), d0
        rts
}
";

const ASM_INDEXED: &str = "\
cpu 68000
FRAME_PIECE_COUNT = 4
P:
        move.w  (a0,d0.w), d0
        move.w  FRAME_PIECE_COUNT(a0,d0.w), d0
        rts
";

#[test]
fn indexed_frame_reads_match_as() {
    assert_byte_identical(
        &as_reference(ASM_INDEXED),
        &emp_image(EMP_INDEXED),
        "P4 (An,Dn.w) with zero and comptime-symbolic displacement",
    );
}

// ---------------------------------------------------------------------------
// P5 — address-register word traffic for the sibling chain: an An SOURCE
// stored to a memory word, An → Dn, and `movea.w Dn, An` reconstituting a
// pointer out of the sign-extending $FFFFxxxx RAM window.
// ---------------------------------------------------------------------------

const EMP_AREG_WORD: &str = "\
module m in p5
pub proc P (a0: *u8, a2: *u8) clobbers(d1, d3, a1) {
        move.w  a0, $26(a2)
        move.w  d3, $28(a2)
        move.w  a2, d3
        move.w  a2, $28(a0)
        movea.w d1, a1
        move.w  a2, $28(a1)
        move.w  #0, $28(a0)
        rts
}
";

const ASM_AREG_WORD: &str = "\
cpu 68000
P:
        move.w  a0, $26(a2)
        move.w  d3, $28(a2)
        move.w  a2, d3
        move.w  a2, $28(a0)
        movea.w d1, a1
        move.w  a2, $28(a1)
        move.w  #0, $28(a0)
        rts
";

#[test]
fn address_register_word_links_match_as() {
    assert_byte_identical(
        &as_reference(ASM_AREG_WORD),
        &emp_image(EMP_AREG_WORD),
        "P5 An-source word stores + movea.w Dn,An",
    );
}

// ---------------------------------------------------------------------------
// P6 — DEMANDED FEATURE: struct-field displacement over a SPLICED base
// register inside a comptime-fn template. Byte-identical to the same
// instructions written with a literal register (the twin spelling).
// ---------------------------------------------------------------------------

const EMP_FIELD_OVER_SPLICED_BASE: &str = "\
module m in p6
pub struct Rec (size: 8) {
    lo: u16,
    pad: u16 @ $02,
    hi: u32 @ $04,
}
pub comptime fn touch(rec: Reg, scratch: Reg) -> Code {
    return asm {
        move.w  Rec.lo({rec}), {scratch}
        move.l  Rec.hi({rec}), {scratch}
        move.w  {scratch}, Rec.lo({rec})
    }
}
pub proc P (a2: *u8) clobbers(d0) {
        touch(a2, d0)
        rts
}
";

const ASM_FIELD_OVER_SPLICED_BASE: &str = "\
cpu 68000
P:
        move.w  $00(a2), d0
        move.l  $04(a2), d0
        move.w  d0, $00(a2)
        rts
";

#[test]
fn struct_field_displacement_over_spliced_base_matches_as() {
    assert_byte_identical(
        &as_reference(ASM_FIELD_OVER_SPLICED_BASE),
        &emp_image(EMP_FIELD_OVER_SPLICED_BASE),
        "P6 Struct.field({splice}) inside a comptime-fn template",
    );
}

// ---------------------------------------------------------------------------
// P7 — engine/coords.emp is a TYPE/TEMPLATE-ONLY module: it must emit ZERO
// bytes anywhere (no section, no placement), the sst.emp / frames.emp /
// aabb.emp precedent. A new module that emits even one byte needs the full
// region + gate + pin treatment, so this is checked mechanically rather than
// argued from the module header.
// ---------------------------------------------------------------------------

#[test]
fn coords_module_emits_zero_bytes() {
    let aeon = sigil_harness::test_support::aeon_dir();
    let path = aeon.join("engine/coords.emp");
    let Ok(src) = std::fs::read_to_string(&path) else {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but {} missing", path.display());
        }
        eprintln!("skip: {} not found (set AEON_DIR)", path.display());
        return;
    };
    let (file, pdiags) = parse_str(&src);
    assert!(
        !pdiags.iter().any(|d| d.level == Level::Error),
        "coords.emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: Vec::new(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        !ldiags.iter().any(|d| d.level == Level::Error),
        "coords.emp lower errors: {ldiags:?}"
    );
    let bytes: usize = module.sections.iter().map(|s| s.fragments.len()).sum();
    assert_eq!(
        module.sections.len(),
        0,
        "engine.coords must open NO section (it emitted {} section(s), {bytes} fragment(s)), \
         a module that emits bytes needs a region, a gate and a pin",
        module.sections.len()
    );
}
