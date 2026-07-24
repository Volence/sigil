//! Tranche 20 — step-0 spelling probes for the dma_queue/load_art port,
//! pinned as permanent byte-parity tests against the independent AS front-end.
//!
//! Each probe replicates the REAL site's binding class (2026-07-15 probe rule):
//! proc-body instructions through the production parse → lower → resolve → link
//! pipeline, compared byte-for-byte with the hand-spelled `.asm` equivalent
//! through `sigil-frontend-as` (which assembles the canonical ROM's twin, so
//! parity with it IS parity with asl).
//!
//! - P1 `movep` both directions, literal / const-name / spliced / struct-sugar
//!   displacements (dma_queue's slot-write + size-read forms).
//! - P2 `trap #0` (the jump-table filler slots).
//! - P3 CCR surgery under a restored SR (`move.w (sp)+, sr` THEN
//!   `andi.b #$FE, ccr` / `ori.b #1, ccr` — the carry-contract order).
//! - P4 `jmp .jump_table(a1)` — a local label riding the d16(An) field
//!   (legal because the table sits in abs.w-reachable ROM).
//! - P5 the D3 named-drain-label jump-table shape vs the AS twin's
//!   `bra.w .drain_end-.c*8` arithmetic `rept` — the shape-equivalence proof
//!   in miniature (3 slots).
//! - P6 the D2 fold-generated slot pre-fill vs the AS `rept`/`set` twin
//!   (miniature, 4 slots) — including the slot-0 zero-disp `(a0)` collapse on
//!   `move.b` and the movep non-collapse.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

/// Assemble a `.asm` source through the AS front-end (68k), link, flatten.
fn as_reference(asm: &str) -> Vec<u8> {
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let module = assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble failed: {d:?}"));
    let linked = sigil_link::link(&module.sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Full emp pipeline (parse -> lower -> resolve_layout -> link -> flatten).
fn emp_candidate(emp: &str) -> Vec<u8> {
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
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        !ldiags.iter().any(|d| d.level == Level::Error),
        "emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&module.sections, &empty, true)
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
// P1 — movep, all displacement spellings the port uses.
// ---------------------------------------------------------------------------

const EMP_MOVEP: &str = "\
module m
struct Ent {
    Reg94: u8,
    SizeH: u8,
    Reg93: u8,
    SizeL: u8,
}
const SIZE_L = offsetof(Ent, SizeL)
comptime fn slot_write(off: int) -> Code {
    return asm {
        movep.l d1, {off}(a1)
    }
}
pub proc Probe () {
        movep.l d1, 3(a1)
        movep.w d3, 1(a1)
        movep.w 1(a0), d1
        movep.l 2(a0), d5
        movep.l d1, SIZE_L(a1)
        movep.w Ent.SizeH(a0), d1
        slot_write(17)
        rts
}
";

const ASM_MOVEP: &str = "\
cpu 68000
Probe:
        movep.l d1, 3(a1)
        movep.w d3, 1(a1)
        movep.w 1(a0), d1
        movep.l 2(a0), d5
        movep.l d1, 3(a1)
        movep.w 1(a0), d1
        movep.l d1, 17(a1)
        rts
";

#[test]
fn movep_both_directions_disp_forms_match_as() {
    assert_byte_identical(
        &as_reference(ASM_MOVEP),
        &emp_candidate(EMP_MOVEP),
        "movep displacement forms",
    );
}

// ---------------------------------------------------------------------------
// P2 — trap #0 filler.
// ---------------------------------------------------------------------------

const EMP_TRAP: &str = "\
module m
comptime fn filler(n: int) -> Code {
    return 0..n |> fold(asm {}, |acc, _i| acc ++ asm { trap #0 })
}
pub proc Probe () {
        trap    #0
        filler(5)
        rts
}
";

const ASM_TRAP: &str = "\
cpu 68000
Probe:
        trap    #0
    rept 5
        trap    #0
    endr
        rts
";

#[test]
fn trap_fillers_match_as() {
    assert_byte_identical(&as_reference(ASM_TRAP), &emp_candidate(EMP_TRAP), "trap #0 filler");
}

// ---------------------------------------------------------------------------
// P3 — CCR surgery under a restored SR (the QueueDMATransfer carry contract:
// restore the caller's SR first, THEN pin the carry — order is load-bearing).
// ---------------------------------------------------------------------------

const EMP_CCR: &str = "\
module m
pub proc Probe () {
        move.w  sr, -(sp)
        move.w  #$2700, sr
        move.w  (sp)+, sr
        andi.b  #$FE, ccr
        move.w  (sp)+, sr
        ori.b   #1, ccr
        rts
}
";

const ASM_CCR: &str = "\
cpu 68000
Probe:
        move.w  sr, -(sp)
        move.w  #$2700, sr
        move.w  (sp)+, sr
        andi.b  #$FE, ccr
        move.w  (sp)+, sr
        ori.b   #1, ccr
        rts
";

#[test]
fn ccr_surgery_after_sr_restore_matches_as() {
    assert_byte_identical(&as_reference(ASM_CCR), &emp_candidate(EMP_CCR), "ccr surgery");
}

// ---------------------------------------------------------------------------
// P4 — jmp .jump_table(a1): the local label rides the d16 field over a1.
// ---------------------------------------------------------------------------

const EMP_JMP: &str = "\
module m
pub proc Probe () {
        jmp     .jump_table(a1)
    .jump_table:
        nop
        rts
}
";

const ASM_JMP: &str = "\
cpu 68000
Probe:
        jmp     .jump_table(a1)
.jump_table:
        nop
        rts
";

#[test]
fn jmp_local_label_disp_over_an_matches_as() {
    assert_byte_identical(&as_reference(ASM_JMP), &emp_candidate(EMP_JMP), "jmp label(a1)");
}

// ---------------------------------------------------------------------------
// P5 — the D3 named-drain-label jump-table shape (miniature, 3 slots) vs the
// AS twin's arithmetic `bra.w .drain_end-.c*8` rept. Byte-identity here is
// the proof that naming the drain entry points changes NOTHING in the bytes.
// ---------------------------------------------------------------------------

const EMP_JT: &str = "\
module m
equ VDP_CTRL = $C00004
equ QBASE    = $FFFF8000
equ SLOTVAR  = $FFFF9000
comptime fn jt_slot(drain: Label) -> Code {
    return asm {
        lea     VDP_CTRL, a5
        lea     QBASE, a1
        bra.w   {drain}
    }
}
comptime fn jt_filler(n: int) -> Code {
    return 0..n |> fold(asm {}, |acc, _i| acc ++ asm { trap #0 })
}
comptime fn send_entry() -> Code {
    return asm {
        move.l  (a1)+, (a5)
        move.l  (a1)+, (a5)
        move.l  (a1)+, (a5)
        move.w  (a1)+, (a5)
    }
}
pub proc Probe () {
        jmp     .jump_table(a1)
    .jump_table:
        bra.w   .done
        jt_filler(5)
        jt_slot(.drain_1)
        jt_slot(.drain_2)
        lea     VDP_CTRL, a5
        lea     QBASE, a1
    .drain_3:
        send_entry()
    .drain_2:
        send_entry()
    .drain_1:
        send_entry()
        move.w  #QBASE, SLOTVAR
    .done:
        rts
}
";

const ASM_JT: &str = "\
cpu 68000
Probe:
        jmp     .jump_table(a1)
.jump_table:
        bra.w   .done
    rept 5
        trap    #0
    endr
    set .c, 1
    rept 3
        lea     ($C00004).l, a5
        lea     ($FFFF8000).w, a1
    if .c <> 3
        bra.w   .drain_end-.c*8
    endif
    set .c, .c+1
    endr
    rept 3
        move.l  (a1)+, (a5)
        move.l  (a1)+, (a5)
        move.l  (a1)+, (a5)
        move.w  (a1)+, (a5)
    endr
.drain_end:
        move.w  #$8000, ($FFFF9000).w
.done:
        rts
";

#[test]
fn named_drain_label_jump_table_matches_as_arith_rept() {
    assert_byte_identical(
        &as_reference(ASM_JT),
        &emp_candidate(EMP_JT),
        "named-drain-label jump table",
    );
}

// ---------------------------------------------------------------------------
// P6 — the D2 fold-generated slot pre-fill (miniature, 4 slots): the slot-0
// `move.b` displacement is 0 and must collapse to `(a0)` exactly like asl;
// the movep displacement stays load-bearing at every slot.
// ---------------------------------------------------------------------------

const EMP_FILL: &str = "\
module m
struct Ent {
    Reg94: u8,
    SizeH: u8,
    Reg93: u8,
    SizeL: u8,
    Reg97: u8,
    SrcH:  u8,
    Reg96: u8,
    SrcM:  u8,
    Reg95: u8,
    SrcL:  u8,
    Command: u32,
}
comptime fn fill_slot_markers(n: int) -> Code {
    return 0..n |> fold(asm {}, |acc, c| acc ++ asm {
        move.b  d0, {c * sizeof(Ent) + offsetof(Ent, Reg94)}(a0)
        movep.l d1, {c * sizeof(Ent) + offsetof(Ent, Reg93)}(a0)
    })
}
pub proc Probe () {
        fill_slot_markers(4)
        rts
}
";

const ASM_FILL: &str = "\
cpu 68000
Probe:
    set .c, 0
    rept 4
        move.b  d0, .c+0(a0)
        movep.l d1, .c+2(a0)
    set .c, .c+14
    endr
        rts
";

#[test]
fn fold_prefill_matches_as_rept_set() {
    assert_byte_identical(&as_reference(ASM_FILL), &emp_candidate(EMP_FILL), "slot pre-fill fold");
}
