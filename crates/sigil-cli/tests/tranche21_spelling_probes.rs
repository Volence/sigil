//! Tranche 21 — step-0 spelling probes for the buffers/vblank port,
//! pinned as permanent byte-parity tests against the independent AS front-end.
//!
//! Each probe replicates the REAL site's binding class (2026-07-15 probe rule);
//! see `docs/superpowers/notes/2026-07-24-t21-step0-design.md` for the design
//! decisions each outcome feeds.
//!
//! - P1 `rte` as a proc terminator (VBlank_Handler's class: full movem
//!   round-trip proving `clobbers()`, a computed `jsr (a0) as T` dispatch in
//!   between, rte as the only exit).
//! - P2 `dc.l Sym` from the AS front-end where Sym is .emp-exported (the
//!   vectors.asm IRQ6 vector class) — link-time cross-frontend data directive.
//! - P3 `move.l #Sym, (abs).w` from the AS front-end where Sym is
//!   .emp-exported (the boot.asm `#VInt_Level` class) — link-time
//!   cross-frontend immediate operand.
//! - P4 a comptime fn taking a GLOBAL link-resolved symbol argument spliced
//!   into an EA operand, its CCR result consumed by a caller-side `bcs`
//!   (the queue_static_dma class).
//! - P5 `extern proc` decl under a module-level comptime `if` (the gated
//!   Sound_DebugMirror decl class).
//! - P6 internal `bsr.w .local_tail` where the tail is ALSO reached by
//!   fallthrough and its `rts` is the shared return (the .build_entry class).
//! - P8 an `equ` whose link-time expression uses `>>` and `&` over
//!   `extern()`, feeding a `move.l #imm` cell (the dmaSource-over-RAM class).

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

/// Full emp pipeline (parse -> lower -> resolve_layout -> link -> flatten).
fn emp_candidate(emp: &str) -> Vec<u8> {
    let sections = emp_sections(emp, &[]);
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
    panic!("{what}: length differ — ref {} vs cand {}", reference.len(), candidate.len());
}

// ---------------------------------------------------------------------------
// P1 — rte as a proc terminator, at VBlank_Handler's full class: movem
// round-trip proving clobbers(), computed dispatch blessed to an honest-⊤
// handler type, conditional lag path, rte the only exit.
// ---------------------------------------------------------------------------

const EMP_RTE: &str = "\
module m
equ READY = $FFFF8000
equ PTR   = $FFFF8004
type H = proc () clobbers(d0-d7/a0-a6)
pub proc Probe () clobbers() {
        movem.l d0-a6, -(sp)
        tst.b   READY
        beq     .lag
        movea.l PTR, a0
        jsr     (a0) as H
        bra.s   .done
    .lag:
        moveq   #1, d0
    .done:
        moveq   #0, d0
        move.b  d0, READY
        movem.l (sp)+, d0-a6
        rte
}
";

const ASM_RTE: &str = "\
cpu 68000
READY = $FFFF8000
PTR   = $FFFF8004
Probe:
        movem.l d0-a6, -(sp)
        tst.b   (READY).w
        beq.s   .lag
        movea.l (PTR).w, a0
        jsr     (a0)
        bra.s   .done
.lag:
        moveq   #1, d0
.done:
        moveq   #0, d0
        move.b  d0, (READY).w
        movem.l (sp)+, d0-a6
        rte
";

#[test]
fn rte_terminated_handler_matches_as() {
    assert_byte_identical(&as_reference(ASM_RTE), &emp_candidate(EMP_RTE), "rte handler");
}

// ---------------------------------------------------------------------------
// P2/P3 — AS-side references to an .emp-exported symbol: dc.l (data
// directive) and move.l #imm (immediate operand), resolved cross-frontend at
// link. The .emp proc is pinned at $2000; the AS section at $0.
// ---------------------------------------------------------------------------

const EMP_OWNER: &str = "\
module m in probe
pub proc Probe () clobbers() {
        rts
}
";

const ASM_CONSUMER: &str = "\
cpu 68000
Vec:
        dc.l    Probe
        move.l  #Probe, ($1234).w
";

fn mixed_link(asm: &str, emp: &str) -> Vec<u8> {
    let mut sections = Vec::new();
    let mut a = as_sections(asm);
    for sec in a.iter_mut() {
        sec.lma = 0;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.append(&mut a);
    let mut e = emp_sections(emp, &[]);
    for sec in e.iter_mut() {
        sec.lma = 0x2000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.append(&mut e);
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("mixed resolve failed: {d:?}"));
    let linked =
        sigil_link::link(&resolved, &empty).unwrap_or_else(|d| panic!("mixed link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

#[test]
fn as_data_directive_resolves_emp_proc() {
    let rom = mixed_link(ASM_CONSUMER, EMP_OWNER);
    // dc.l Probe at offset 0 must carry the pinned .emp proc address.
    assert_eq!(&rom[0..4], &[0x00, 0x00, 0x20, 0x00], "dc.l <emp-proc> cell");
}

#[test]
fn as_immediate_operand_resolves_emp_proc() {
    let rom = mixed_link(ASM_CONSUMER, EMP_OWNER);
    // move.l #Probe, ($1234).w — 21FC <imm.l> <abs.w>
    assert_eq!(
        &rom[4..12],
        &[0x21, 0xFC, 0x00, 0x00, 0x20, 0x00, 0x12, 0x34],
        "move.l #<emp-sym> immediate"
    );
}

// ---------------------------------------------------------------------------
// P4 — queue_static_dma class: comptime fn taking a global link-resolved
// symbol argument, spliced into a `lea` EA; the spliced CCR surgery result is
// consumed by a caller-side bcs.
// ---------------------------------------------------------------------------

const QSD_CARRIER: &str = "\
cpu 68000
        phase $FFFF9000
QSLOT:  ds.w 1
        ds.b 14
QEND:   ds.b 0
ENT:    ds.b 14
DIRTY:  ds.b 1
        dephase
";

const EMP_QSD: &str = "\
module m
comptime fn queue_static_dma(entry: Label) -> Code {
    return asm {
        movea.w QSLOT, a1
        cmpa.w  #QEND, a1
        beq     .drop
        lea     {entry}, a2
        move.l  (a2)+, (a1)+
        move.l  (a2)+, (a1)+
        move.l  (a2)+, (a1)+
        move.w  (a2)+, (a1)+
        move.w  a1, QSLOT
        andi.b  #$FE, ccr
        bra     .done
    .drop:
        ori.b   #1, ccr
    .done:
    }
}
pub proc Probe () clobbers(d0/a1-a2) {
        queue_static_dma(ENT)
        bcs     .skip
        bclr    #0, DIRTY
    .skip:
        rts
}
";

// The AS reference is ONE module (the real build: main.asm includes ram.asm
// and the engine code in a single assemble), so the symbols are in-module;
// only the .emp candidate crosses the link seam.
const ASM_QSD: &str = "\
cpu 68000
Probe:
        movea.w (QSLOT).w, a1
        cmpa.w  #QEND, a1
        beq.s   .drop
        lea     (ENT).w, a2
        move.l  (a2)+, (a1)+
        move.l  (a2)+, (a1)+
        move.l  (a2)+, (a1)+
        move.w  (a2)+, (a1)+
        move.w  a1, (QSLOT).w
        andi.b  #$FE, ccr
        bra.s   .done
.drop:
        ori.b   #1, ccr
.done:
        bcs.s   .skip
        bclr    #0, (DIRTY).w
.skip:
        rts
        phase $FFFF9000
QSLOT:  ds.w 1
        ds.b 14
QEND:   ds.b 0
ENT:    ds.b 14
DIRTY:  ds.b 1
        dephase
";

/// Link a code source (AS text or lowered emp sections) against the QSD RAM
/// carrier (the real ram.asm binding class: phase labels, cross-module),
/// returning the code section's flattened bytes.
fn qsd_link(mut code_sections: Vec<Section>) -> Vec<u8> {
    let mut sections = Vec::new();
    let mut carrier = as_sections(QSD_CARRIER);
    for sec in carrier.iter_mut() {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    for sec in code_sections.iter_mut() {
        sec.lma = 0;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.append(&mut code_sections);
    sections.append(&mut carrier);
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("qsd resolve failed: {d:?}"));
    let linked =
        sigil_link::link(&resolved, &empty).unwrap_or_else(|d| panic!("qsd link failed: {d:?}"));
    let img = sigil_link::flatten(&linked, 0x00);
    // The code sits at 0; the carrier is far above — trim to the code bytes.
    let code_len = linked
        .sections
        .iter()
        .filter(|s| s.lma < 0x0100_0000)
        .map(|s| s.lma as usize + s.bytes.len())
        .max()
        .unwrap_or(0);
    img[..code_len].to_vec()
}

#[test]
fn global_symbol_arg_splice_with_ccr_consumer_matches_as() {
    let candidate = qsd_link(emp_sections(EMP_QSD, &[]));
    let reference = as_reference(ASM_QSD);
    assert_byte_identical(&reference[..candidate.len()], &candidate, "queue_static_dma");
}

// ---------------------------------------------------------------------------
// P5 — the gated Sound_DebugMirror class. A module-level comptime `if` around
// the extern decl is NOT in the grammar ("expected a declaration, found
// Ident(if)" — probed 2026-07-24, banked as demand data). The shipped
// spelling: the decl stays UNGATED (an extern decl emits nothing and an
// unreferenced one never reaches the link), the CALL is gated by a
// statement-level comptime if (the game_loop Debug_MusicToggle precedent).
// ON shape: the call resolves against a pinned label. OFF shape: the call
// vanishes (bare rts body) and the unreferenced decl must not produce an
// unresolved-symbol link error.
// ---------------------------------------------------------------------------

const EMP_GATED_DECL: &str = "\
module m
extern proc SndMirror () clobbers(d0-d1/a0-a1)
pub proc Probe () clobbers(d0-d1/a0-a1) {
        if MIRROR == 1 {
            jbsr    SndMirror
        }
        rts
}
";

#[test]
fn gated_extern_decl_compiles_both_shapes() {
    // OFF shape: no decl, no call — must lower clean with no unresolved ref.
    let off = emp_sections(EMP_GATED_DECL, &[("MIRROR", 0)]);
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&off, &empty, true)
        .unwrap_or_else(|d| panic!("gated-decl OFF resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &empty)
        .unwrap_or_else(|d| panic!("gated-decl OFF link failed: {d:?}"));
    let bytes = sigil_link::flatten(&linked, 0x00);
    assert_eq!(bytes, vec![0x4E, 0x75], "OFF shape must be a bare rts");

    // ON shape: decl present, call resolves against a pinned AS-side label.
    let mut sections = as_sections("cpu 68000\nSndMirror:\n        rts\n");
    for sec in sections.iter_mut() {
        sec.lma = 0x100;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    let mut on = emp_sections(EMP_GATED_DECL, &[("MIRROR", 1)]);
    for sec in on.iter_mut() {
        sec.lma = 0;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.append(&mut on);
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("gated-decl ON resolve failed: {d:?}"));
    sigil_link::link(&resolved, &empty)
        .unwrap_or_else(|d| panic!("gated-decl ON link failed: {d:?}"));
}

// ---------------------------------------------------------------------------
// P6 — the .build_entry class: internal bsr.w to a local tail label that is
// ALSO entered by fallthrough; the tail's rts is the shared return.
// ---------------------------------------------------------------------------

const EMP_TAIL: &str = "\
module m
pub proc Probe () clobbers(d0/a0) {
        bsr.w   .tail
        bsr.w   .tail
        moveq   #1, d0
    .tail:
        move.b  d0, (a0)
        rts
}
";

const ASM_TAIL: &str = "\
cpu 68000
Probe:
        bsr.w   .tail
        bsr.w   .tail
        moveq   #1, d0
.tail:
        move.b  d0, (a0)
        rts
";

#[test]
fn internal_bsr_tail_with_fallthrough_matches_as() {
    assert_byte_identical(&as_reference(ASM_TAIL), &emp_candidate(EMP_TAIL), ".build_entry tail");
}

// ---------------------------------------------------------------------------
// P8 — dmaSource over a link-time RAM address: an equ whose expression
// shifts and masks an extern() value, feeding a move.l #imm cell.
// ---------------------------------------------------------------------------

const EMP_LINKEXPR: &str = "\
module m
equ SRC = (extern(\"PalBuf\") >> 1) & $7FFFFF
pub proc Probe () clobbers(d1) {
        move.l  #SRC, d1
        rts
}
";

const ASM_LINKEXPR: &str = "\
cpu 68000
PalBuf = $FFFF8206
Probe:
        move.l  #(PalBuf>>1)&$7FFFFF, d1
        rts
";

#[test]
fn linkexpr_shift_mask_over_extern_matches_as() {
    // Reference: AS computes the fold locally (drop its equ-only prelude by
    // linking the whole source in one module).
    let reference = as_reference(ASM_LINKEXPR);

    // Candidate: the .emp module resolves PalBuf through the link seam from
    // an AS-side equ carrier (the real ram.asm binding class).
    let mut sections = as_sections("cpu 68000\nPalBuf = $FFFF8206\n");
    for sec in sections.iter_mut() {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    let mut e = emp_sections(EMP_LINKEXPR, &[]);
    for sec in e.iter_mut() {
        sec.lma = 0;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.append(&mut e);
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("linkexpr resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &empty)
        .unwrap_or_else(|d| panic!("linkexpr link failed: {d:?}"));
    let bytes = sigil_link::flatten(&linked, 0x00);
    assert_byte_identical(&reference, &bytes[..reference.len()], "dmaSource link expr");
}

// ---------------------------------------------------------------------------
// P9 (loop pass 1, step 4) — sr_masked: a comptime fn taking a Code ARGUMENT
// (the skeleton-with-holes class inverted: the hole is the parameter). The
// bracket construct is only buildable if a `Code`-typed fn param evaluates
// and concatenates; byte parity vs the hand-spelled AS bracket.
// ---------------------------------------------------------------------------

const EMP_SRMASK: &str = "\
module m
equ FLAG  = $FFFF8000
equ READY = $FFFF8001
comptime fn sr_masked(code: Code) -> Code {
    return asm {
        move.w  sr, -(sp)
        move.w  #$2700, sr
    } ++ code ++ asm {
        move.w  (sp)+, sr
    }
}
pub proc Probe () clobbers(d0) preserves(sr) {
        moveq   #0, d0
        sr_masked(asm {
            move.b  d0, FLAG
            move.b  #1, READY
        })
        rts
}
";

const ASM_SRMASK: &str = "\
cpu 68000
FLAG  = $FFFF8000
READY = $FFFF8001
Probe:
        moveq   #0, d0
        move.w  sr, -(sp)
        move.w  #$2700, sr
        move.b  d0, (FLAG).w
        move.b  #1, (READY).w
        move.w  (sp)+, sr
        rts
";

#[test]
fn sr_masked_code_argument_matches_as() {
    assert_byte_identical(&as_reference(ASM_SRMASK), &emp_candidate(EMP_SRMASK), "sr_masked");
}
