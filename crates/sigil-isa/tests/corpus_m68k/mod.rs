//! The single canonical Sigil 68000 asl-oracle corpus — one `(snippet, Instruction)`
//! pair per MOVE EA-matrix form. Shared by the golden generator
//! (`src/bin/gen_m68k_vectors.rs`, via `#[path]`) and the encoder tests
//! (`tests/encode_m68k.rs`), which pull it in with `mod corpus_m68k;`. Each snippet
//! string is EXACTLY what `asl` is fed, so it matches the committed golden file's
//! keys character-for-character.
//!
//! Cargo does not compile a `tests/<name>/mod.rs` subdirectory module as its own
//! integration-test binary, so it is safe to share.
#![allow(dead_code)]

use sigil_isa::m68k::{Cond, Instruction, Mnemonic, Operand, Size, Xn};

fn mov(size: Size, src: Operand, dst: Operand) -> Instruction {
    Instruction { mnemonic: Mnemonic::Move, size, ops: vec![src, dst] }
}

/// `bchg <src>,<dst>`. The size slot is not encoded (the bit ops derive it from
/// the destination) but is spelled out per row so the corpus records the
/// operation width asl reports for that destination.
fn bch(src: Operand, dst: Operand, size: Size) -> Instruction {
    Instruction { mnemonic: Mnemonic::Bchg, size, ops: vec![src, dst] }
}

fn sh(mnemonic: Mnemonic, size: Size, ops: Vec<Operand>) -> Instruction {
    Instruction { mnemonic, size, ops }
}

/// `exg.l Rx,Ry` — long by construction.
fn ex(a: Operand, b: Operand) -> Instruction {
    Instruction { mnemonic: Mnemonic::Exg, size: Size::L, ops: vec![a, b] }
}

/// `move.w <ea>,ccr` — word by construction; there is no move-FROM-ccr on the 68000.
fn mcc(src: Operand) -> Instruction {
    Instruction { mnemonic: Mnemonic::MoveToCcr, size: Size::W, ops: vec![src, Operand::Ccr] }
}

use Operand::*;
use Size::{B, L, W};

/// The definitive M0.5 MOVE EA-matrix corpus. Snippet strings are verbatim asl input.
pub fn corpus_m68k() -> Vec<(&'static str, Instruction)> {
    vec![
        // reg <-> reg baseline
        ("move.w d1,d0", mov(W, Dn(1), Dn(0))),
        ("move.l a1,d0", mov(L, An(1), Dn(0))),
        // source-mode sweep into d0
        ("move.w (a1),d0", mov(W, Ind(1), Dn(0))),
        ("move.w (a1)+,d0", mov(W, PostInc(1), Dn(0))),
        ("move.w -(a1),d0", mov(W, PreDec(1), Dn(0))),
        ("move.w (4,a1),d0", mov(W, Disp16An(4, 1), Dn(0))),
        ("move.w (6,a1,d2.w),d0", mov(W, Disp8AnXn { d: 6, an: 1, xn: Xn::D(2), long: false }, Dn(0))),
        ("move.w ($1234).w,d0", mov(W, AbsW(0x1234), Dn(0))),
        ("move.w ($12345678).l,d0", mov(W, AbsL(0x12345678), Dn(0))),
        // `Pcd16` holds the RESOLVED displacement emitted into the extension word
        // (like the Z80 `Rel` operand). asl reads `(8,pc)` as an absolute target at
        // `org 0` and resolves the stored disp to `target - ext_word_addr = 8 - 2 = 6`;
        // the spike encodes that resolved disp — target→disp resolution is an M1 fixup.
        ("move.w (8,pc),d0", mov(W, Pcd16(6), Dn(0))),
        ("move.w #$1234,d0", mov(W, Imm(0x1234), Dn(0))),
        // dest-mode sweep from d1 (proves the dest-EA mode/register swap)
        ("move.w d1,(a0)", mov(W, Dn(1), Ind(0))),
        ("move.w d1,(a0)+", mov(W, Dn(1), PostInc(0))),
        ("move.w d1,-(a0)", mov(W, Dn(1), PreDec(0))),
        ("move.w d1,(4,a0)", mov(W, Dn(1), Disp16An(4, 0))),
        ("move.w d1,($1234).w", mov(W, Dn(1), AbsW(0x1234))),
        ("move.w d1,($12345678).l", mov(W, Dn(1), AbsL(0x12345678))),
        // size + extension-word long flag
        ("move.l (2,a3,a4.l),d0", mov(L, Disp8AnXn { d: 2, an: 3, xn: Xn::A(4), long: true }, Dn(0))),
        // review hardening: pin source-before-dest ext-word ordering, sign, and B size
        ("move.w ($1234).w,($5678).w", mov(W, AbsW(0x1234), AbsW(0x5678))),
        ("move.b #$12,d0", mov(B, Imm(0x12), Dn(0))),
        ("move.w (-4,a1),d0", mov(W, Disp16An(-4, 1), Dn(0))),
        ("move.w (-2,a2,d3.w),d0", mov(W, Disp8AnXn { d: -2, an: 2, xn: Xn::D(3), long: false }, Dn(0))),
        // --- ALU-EA family ---
        ("add.w d1,d0", Instruction { mnemonic: Mnemonic::Add, size: W, ops: vec![Dn(1), Dn(0)] }),
        ("add.w (a1),d0", Instruction { mnemonic: Mnemonic::Add, size: W, ops: vec![Ind(1), Dn(0)] }),
        ("add.l d0,(a1)", Instruction { mnemonic: Mnemonic::Add, size: L, ops: vec![Dn(0), Ind(1)] }),
        ("sub.w d1,d0", Instruction { mnemonic: Mnemonic::Sub, size: W, ops: vec![Dn(1), Dn(0)] }),
        ("and.w d1,d0", Instruction { mnemonic: Mnemonic::And, size: W, ops: vec![Dn(1), Dn(0)] }),
        ("or.b d1,d0", Instruction { mnemonic: Mnemonic::Or, size: B, ops: vec![Dn(1), Dn(0)] }),
        ("eor.w d0,d1", Instruction { mnemonic: Mnemonic::Eor, size: W, ops: vec![Dn(0), Dn(1)] }),
        ("cmp.w (a1),d0", Instruction { mnemonic: Mnemonic::Cmp, size: W, ops: vec![Ind(1), Dn(0)] }),
        ("cmpa.l a1,a0", Instruction { mnemonic: Mnemonic::Cmpa, size: L, ops: vec![An(1), An(0)] }),
        ("adda.w d0,a1", Instruction { mnemonic: Mnemonic::Adda, size: W, ops: vec![Dn(0), An(1)] }),
        ("suba.l a2,a3", Instruction { mnemonic: Mnemonic::Suba, size: L, ops: vec![An(2), An(3)] }),
        ("muls.w d1,d0", Instruction { mnemonic: Mnemonic::Muls, size: W, ops: vec![Dn(1), Dn(0)] }),
        ("mulu.w d1,d0", Instruction { mnemonic: Mnemonic::Mulu, size: W, ops: vec![Dn(1), Dn(0)] }),
        ("mulu.w (a1),d0", Instruction { mnemonic: Mnemonic::Mulu, size: W, ops: vec![Ind(1), Dn(0)] }),
        ("mulu.w #36,d0", Instruction { mnemonic: Mnemonic::Mulu, size: W, ops: vec![Imm(36), Dn(0)] }),
        ("mulu.w #40,d0", Instruction { mnemonic: Mnemonic::Mulu, size: W, ops: vec![Imm(40), Dn(0)] }),
        ("mulu.w ($1234).w,d0", Instruction { mnemonic: Mnemonic::Mulu, size: W, ops: vec![AbsW(0x1234), Dn(0)] }),
        // --- ALU-immediate family ---
        ("addi.w #$10,d0", Instruction { mnemonic: Mnemonic::Addi, size: W, ops: vec![Imm(0x10), Dn(0)] }),
        ("subi.l #$1000,d1", Instruction { mnemonic: Mnemonic::Subi, size: L, ops: vec![Imm(0x1000), Dn(1)] }),
        ("andi.w #$00FF,d0", Instruction { mnemonic: Mnemonic::Andi, size: W, ops: vec![Imm(0x00FF), Dn(0)] }),
        ("ori.b #$01,d0", Instruction { mnemonic: Mnemonic::Ori, size: B, ops: vec![Imm(0x01), Dn(0)] }),
        ("eori.w #$FFFF,d0", Instruction { mnemonic: Mnemonic::Eori, size: W, ops: vec![Imm(0xFFFF), Dn(0)] }),
        ("cmpi.w #$0010,(a1)", Instruction { mnemonic: Mnemonic::Cmpi, size: W, ops: vec![Imm(0x10), Ind(1)] }),
        ("andi.b #$FE,ccr", Instruction { mnemonic: Mnemonic::AndiCcr, size: B, ops: vec![Imm(0xFE), Ccr] }),
        ("ori.b #$01,ccr", Instruction { mnemonic: Mnemonic::OriCcr, size: B, ops: vec![Imm(0x01), Ccr] }),
        ("move.w #$2700,sr", Instruction { mnemonic: Mnemonic::MoveToSr, size: W, ops: vec![Imm(0x2700), Sr] }),
        ("move.w sr,-(sp)", Instruction { mnemonic: Mnemonic::MoveFromSr, size: W, ops: vec![Sr, PreDec(7)] }),
        // --- quick family ---
        ("moveq #1,d0", Instruction { mnemonic: Mnemonic::Moveq, size: L, ops: vec![Imm(1), Dn(0)] }),
        ("moveq #-1,d3", Instruction { mnemonic: Mnemonic::Moveq, size: L, ops: vec![Imm(-1), Dn(3)] }),
        ("addq.w #1,d0", Instruction { mnemonic: Mnemonic::Addq, size: W, ops: vec![Imm(1), Dn(0)] }),
        ("addq.l #8,a1", Instruction { mnemonic: Mnemonic::Addq, size: L, ops: vec![Imm(8), An(1)] }),
        ("subq.w #2,d1", Instruction { mnemonic: Mnemonic::Subq, size: W, ops: vec![Imm(2), Dn(1)] }),
        // --- shift/rotate family ---
        ("asl.w #1,d0", Instruction { mnemonic: Mnemonic::Asl, size: W, ops: vec![Imm(1), Dn(0)] }),
        ("asr.l #3,d1", Instruction { mnemonic: Mnemonic::Asr, size: L, ops: vec![Imm(3), Dn(1)] }),
        ("lsl.w d2,d0", Instruction { mnemonic: Mnemonic::Lsl, size: W, ops: vec![Dn(2), Dn(0)] }),
        ("lsr.b #1,d0", Instruction { mnemonic: Mnemonic::Lsr, size: B, ops: vec![Imm(1), Dn(0)] }),
        ("rol.w #2,d0", Instruction { mnemonic: Mnemonic::Rol, size: W, ops: vec![Imm(2), Dn(0)] }),
        ("ror.w d1,d0", Instruction { mnemonic: Mnemonic::Ror, size: W, ops: vec![Dn(1), Dn(0)] }),
        // --- bit ops ---
        // `bchg` is the fourth `tt` row; its destination matrix is `bset`'s
        // (DATA ALTERABLE), so the sweep below is the full row: every legal
        // destination mode, in both the static `#n` and dynamic `Dn` forms.

        ("btst #7,d0", Instruction { mnemonic: Mnemonic::Btst, size: L, ops: vec![Imm(7), Dn(0)] }),
        ("bset #0,(a0)", Instruction { mnemonic: Mnemonic::Bset, size: B, ops: vec![Imm(0), Ind(0)] }),
        ("bclr #5,d1", Instruction { mnemonic: Mnemonic::Bclr, size: L, ops: vec![Imm(5), Dn(1)] }),
        ("btst d2,d0", Instruction { mnemonic: Mnemonic::Btst, size: L, ops: vec![Dn(2), Dn(0)] }),
        ("bset d1,(a0)", Instruction { mnemonic: Mnemonic::Bset, size: B, ops: vec![Dn(1), Ind(0)] }),
        // --- single-EA family ---
        ("clr.w d0", Instruction { mnemonic: Mnemonic::Clr, size: W, ops: vec![Dn(0)] }),
        ("clr.l (a1)", Instruction { mnemonic: Mnemonic::Clr, size: L, ops: vec![Ind(1)] }),
        ("neg.w d0", Instruction { mnemonic: Mnemonic::Neg, size: W, ops: vec![Dn(0)] }),
        ("not.b d0", Instruction { mnemonic: Mnemonic::Not, size: B, ops: vec![Dn(0)] }),
        ("tst.w d0", Instruction { mnemonic: Mnemonic::Tst, size: W, ops: vec![Dn(0)] }),
        ("tst.l (a1)", Instruction { mnemonic: Mnemonic::Tst, size: L, ops: vec![Ind(1)] }),
        ("tas.b d0", Instruction { mnemonic: Mnemonic::Tas, size: B, ops: vec![Dn(0)] }),
        ("st d0", Instruction { mnemonic: Mnemonic::Scc(Cond::T), size: B, ops: vec![Dn(0)] }),
        ("sf d0", Instruction { mnemonic: Mnemonic::Scc(Cond::F), size: B, ops: vec![Dn(0)] }),
        ("sgt d0", Instruction { mnemonic: Mnemonic::Scc(Cond::Gt), size: B, ops: vec![Dn(0)] }),
        // --- control / misc ---
        ("jmp ($1234).w", Instruction { mnemonic: Mnemonic::Jmp, size: W, ops: vec![AbsW(0x1234)] }),
        ("jmp ($12345678).l", Instruction { mnemonic: Mnemonic::Jmp, size: L, ops: vec![AbsL(0x12345678)] }),
        ("jsr ($1234).w", Instruction { mnemonic: Mnemonic::Jsr, size: W, ops: vec![AbsW(0x1234)] }),
        ("jmp (a0)", Instruction { mnemonic: Mnemonic::Jmp, size: W, ops: vec![Ind(0)] }),
        // Like `Pcd16`, the stored `d` is the RESOLVED displacement asl emits: it reads
        // `(4,pc,...)` as an absolute target at `org 0` and resolves the brief-ext disp to
        // `target - ext_word_addr = 4 - 2 = 2`; target→disp resolution is an M1 fixup.
        ("jmp (4,pc,d0.w)", Instruction { mnemonic: Mnemonic::Jmp, size: W, ops: vec![Pcd8Xn { d: 2, xn: Xn::D(0), long: false }] }),
        ("lea (4,a0),a1", Instruction { mnemonic: Mnemonic::Lea, size: L, ops: vec![Disp16An(4, 0), An(1)] }),
        ("pea (a0)", Instruction { mnemonic: Mnemonic::Pea, size: L, ops: vec![Ind(0)] }),
        ("nop", Instruction { mnemonic: Mnemonic::Nop, size: W, ops: vec![] }),
        ("rts", Instruction { mnemonic: Mnemonic::Rts, size: W, ops: vec![] }),
        ("rte", Instruction { mnemonic: Mnemonic::Rte, size: W, ops: vec![] }),
        ("trap #0", Instruction { mnemonic: Mnemonic::Trap, size: W, ops: vec![Imm(0)] }),
        ("swap d0", Instruction { mnemonic: Mnemonic::Swap, size: W, ops: vec![Dn(0)] }),
        ("ext.w d0", Instruction { mnemonic: Mnemonic::Ext, size: W, ops: vec![Dn(0)] }),
        ("ext.l d1", Instruction { mnemonic: Mnemonic::Ext, size: L, ops: vec![Dn(1)] }),
        // --- branches (2-wide only) + DBcc (non-relaxable) ---
        ("bra.s *", Instruction { mnemonic: Mnemonic::Bra, size: Size::S, ops: vec![Disp(-2)] }),
        ("bra.w *", Instruction { mnemonic: Mnemonic::Bra, size: W, ops: vec![Disp(-2)] }),
        ("bsr.s *", Instruction { mnemonic: Mnemonic::Bsr, size: Size::S, ops: vec![Disp(-2)] }),
        ("bsr.w *", Instruction { mnemonic: Mnemonic::Bsr, size: W, ops: vec![Disp(-2)] }),
        ("beq.s *", Instruction { mnemonic: Mnemonic::Bcc(Cond::Eq), size: Size::S, ops: vec![Disp(-2)] }),
        ("bne.w *", Instruction { mnemonic: Mnemonic::Bcc(Cond::Ne), size: W, ops: vec![Disp(-2)] }),
        ("dbf d0,*", Instruction { mnemonic: Mnemonic::Dbcc(Cond::F), size: W, ops: vec![Dn(0), Disp(-2)] }),
        ("dbeq d1,*", Instruction { mnemonic: Mnemonic::Dbcc(Cond::Eq), size: W, ops: vec![Dn(1), Disp(-2)] }),
        // --- MOVEM: register-store (to -(An)) and register-load (from (An)+/others) ---
        // masks: d0-d7 = 0x00FF; a0-a6 = 0x7F00; d0-a6 (all-but-a7) = 0x7FFF; single a2 = 0x0400; d3/d4 = 0x0018
        ("movem.l d0-d7/a0-a6,-(sp)", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![RegList(0x7FFF), PreDec(7)] }),
        ("movem.l (sp)+,d0-d7/a0-a6", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![PostInc(7), RegList(0x7FFF)] }),
        ("movem.l a2,-(sp)", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![RegList(0x0400), PreDec(7)] }),
        ("movem.l d3-d4,(a3)", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![RegList(0x0018), Ind(3)] }),
        ("movem.l d3-d4,(8,a3)", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![RegList(0x0018), Disp16An(8, 3)] }),
        ("movem.w d0-d6/a2,(a1)", Instruction { mnemonic: Mnemonic::Movem, size: W, ops: vec![RegList(0x047F), Ind(1)] }),
        ("movem.l (a0)+,d0-a4", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![PostInc(0), RegList(0x1FFF)] }),
        // --- specials ---
        ("movep.w (4,a1),d0", Instruction { mnemonic: Mnemonic::Movep, size: W, ops: vec![Disp16An(4, 1), Dn(0)] }),
        ("movep.l d0,(8,a1)", Instruction { mnemonic: Mnemonic::Movep, size: L, ops: vec![Dn(0), Disp16An(8, 1)] }),
        ("addx.b d1,d0", Instruction { mnemonic: Mnemonic::Addx, size: B, ops: vec![Dn(1), Dn(0)] }),
        ("addx.l d3,d2", Instruction { mnemonic: Mnemonic::Addx, size: L, ops: vec![Dn(3), Dn(2)] }),
        ("cmpm.w (a0)+,(a1)+", Instruction { mnemonic: Mnemonic::Cmpm, size: W, ops: vec![PostInc(0), PostInc(1)] }),
        // --- MOVEA ---
        ("movea.w d0,a1", Instruction { mnemonic: Mnemonic::Movea, size: W, ops: vec![Dn(0), An(1)] }),
        ("movea.l a0,a1", Instruction { mnemonic: Mnemonic::Movea, size: L, ops: vec![An(0), An(1)] }),
        ("movea.w (a2),a3", Instruction { mnemonic: Mnemonic::Movea, size: W, ops: vec![Ind(2), An(3)] }),
        ("movea.l #$1000,a0", Instruction { mnemonic: Mnemonic::Movea, size: L, ops: vec![Imm(0x1000), An(0)] }),
        ("movea.w (4,a1),a2", Instruction { mnemonic: Mnemonic::Movea, size: W, ops: vec![Disp16An(4, 1), An(2)] }),
        // --- BCHG: the full DATA-ALTERABLE destination row, both forms ---
        // Cells the two Sonic corpora exercise are marked; the rest are here
        // because the corpus not reaching a cell is not evidence the cell works.
        ("bchg #3,d0", bch(Imm(3), Dn(0), L)),                                 // S1 (2 sites)
        ("bchg #3,(a0)", bch(Imm(3), Ind(0), B)),
        ("bchg #3,(a0)+", bch(Imm(3), PostInc(0), B)),
        ("bchg #3,-(a0)", bch(Imm(3), PreDec(0), B)),
        ("bchg #3,(4,a0)", bch(Imm(3), Disp16An(4, 0), B)),                    // S1/S2: the `#n,off(aN)` bulk
        ("bchg #3,(6,a0,d1.w)", bch(Imm(3), Disp8AnXn { d: 6, an: 0, xn: Xn::D(1), long: false }, B)),
        ("bchg #3,($1234).w", bch(Imm(3), AbsW(0x1234), B)),
        ("bchg #3,($12345678).l", bch(Imm(3), AbsL(0x12345678), B)),
        ("bchg #0,d0", bch(Imm(0), Dn(0), L)),
        ("bchg #31,d0", bch(Imm(31), Dn(0), L)),
        // asl range-checks nothing here — the bit number is a full word and the
        // hardware masks it (mod 32 for `Dn`, mod 8 for memory).
        ("bchg #255,(a0)", bch(Imm(255), Ind(0), B)),
        ("bchg d2,d0", bch(Dn(2), Dn(0), L)),
        ("bchg d2,(a0)", bch(Dn(2), Ind(0), B)),
        ("bchg d2,(a0)+", bch(Dn(2), PostInc(0), B)),
        ("bchg d2,-(a0)", bch(Dn(2), PreDec(0), B)),
        ("bchg d2,(4,a0)", bch(Dn(2), Disp16An(4, 0), B)),
        ("bchg d2,(6,a0,d1.w)", bch(Dn(2), Disp8AnXn { d: 6, an: 0, xn: Xn::D(1), long: false }, B)),
        ("bchg d2,($1234).w", bch(Dn(2), AbsW(0x1234), B)),
        ("bchg d2,($12345678).l", bch(Dn(2), AbsL(0x12345678), B)),
        // --- ROXL/ROXR: register (immediate + Dn count) and memory forms ---
        ("roxl.w #1,d3", sh(Mnemonic::Roxl, W, vec![Imm(1), Dn(3)])),          // S1/S2 (2 sites each)
        ("roxl.b #8,d0", sh(Mnemonic::Roxl, B, vec![Imm(8), Dn(0)])),
        ("roxl.l #4,d1", sh(Mnemonic::Roxl, L, vec![Imm(4), Dn(1)])),
        ("roxr.w #1,d0", sh(Mnemonic::Roxr, W, vec![Imm(1), Dn(0)])),
        ("roxr.l #8,d7", sh(Mnemonic::Roxr, L, vec![Imm(8), Dn(7)])),
        ("roxl.w d2,d0", sh(Mnemonic::Roxl, W, vec![Dn(2), Dn(0)])),
        ("roxr.b d1,d5", sh(Mnemonic::Roxr, B, vec![Dn(1), Dn(5)])),
        ("roxl.w (a0)", sh(Mnemonic::Roxl, W, vec![Ind(0)])),
        ("roxl.w (a0)+", sh(Mnemonic::Roxl, W, vec![PostInc(0)])),
        ("roxl.w -(a0)", sh(Mnemonic::Roxl, W, vec![PreDec(0)])),
        ("roxl.w (4,a0)", sh(Mnemonic::Roxl, W, vec![Disp16An(4, 0)])),
        ("roxl.w (6,a0,d1.w)", sh(Mnemonic::Roxl, W, vec![Disp8AnXn { d: 6, an: 0, xn: Xn::D(1), long: false }])),
        ("roxl.w ($1234).w", sh(Mnemonic::Roxl, W, vec![AbsW(0x1234)])),
        ("roxr.w ($12345678).l", sh(Mnemonic::Roxr, W, vec![AbsL(0x12345678)])),
        // The two alias spellings asl accepts, pinned as BYTES so the aliasing
        // is proven against asl and not merely asserted: `<shift> Dn` is the
        // count-1 register form and `<shift> #1,<mem>` is the memory form.
        ("roxl.w d0", sh(Mnemonic::Roxl, W, vec![Dn(0)])),
        ("asl.l d0", sh(Mnemonic::Asl, L, vec![Dn(0)])),
        ("roxl.w #1,(a0)", sh(Mnemonic::Roxl, W, vec![Imm(1), Ind(0)])),
        ("asr.w #1,($1234).w", sh(Mnemonic::Asr, W, vec![Imm(1), AbsW(0x1234)])),
        // --- EXG: all three register-pair forms, both written orders ---
        ("exg.l d0,d1", ex(Dn(0), Dn(1))),                                     // S1/S2 (both spellings)
        ("exg.l d7,d0", ex(Dn(7), Dn(0))),
        ("exg.l a0,a1", ex(An(0), An(1))),
        ("exg.l a7,a2", ex(An(7), An(2))),
        ("exg.l d0,a0", ex(Dn(0), An(0))),
        ("exg.l d3,a7", ex(Dn(3), An(7))),
        // asl NORMALISES this order to `Dx,Ay`; the golden bytes prove it.
        ("exg.l a1,d4", ex(An(1), Dn(4))),
        // --- MOVE to CCR: the full DATA source row ---
        ("move.w d6,ccr", mcc(Dn(6))),                                         // S1 (2 sites), S2 (2, bare)
        ("move.w #0,ccr", mcc(Imm(0))),                                        // S2 (3 sites, bare)
        ("move.w (a0),ccr", mcc(Ind(0))),
        ("move.w (a0)+,ccr", mcc(PostInc(0))),
        ("move.w -(a0),ccr", mcc(PreDec(0))),
        ("move.w (4,a0),ccr", mcc(Disp16An(4, 0))),
        ("move.w (6,a0,d1.w),ccr", mcc(Disp8AnXn { d: 6, an: 0, xn: Xn::D(1), long: false })),
        ("move.w ($1234).w,ccr", mcc(AbsW(0x1234))),
        ("move.w ($12345678).l,ccr", mcc(AbsL(0x12345678))),
        // As with `move.w (8,pc),d0` above, the stored disp is the RESOLVED one
        // asl emits: target 8 minus the extension word's own address 2.
        ("move.w (8,pc),ccr", mcc(Pcd16(6))),
        ("move.w (4,pc,d0.w),ccr", mcc(Pcd8Xn { d: 2, xn: Xn::D(0), long: false })),
        // asl takes `.b` here too and emits the same bytes as `.w`, immediate
        // included — the golden row is what proves the two spellings are one.
        ("move.b d6,ccr", Instruction { mnemonic: Mnemonic::MoveToCcr, size: B, ops: vec![Dn(6), Operand::Ccr] }),
        ("move.b #$12,ccr", Instruction { mnemonic: Mnemonic::MoveToCcr, size: B, ops: vec![Imm(0x12), Operand::Ccr] }),
        // --- MOVE to/from USP: the whole matrix is the eight address registers ---
        ("move.l a6,usp", Instruction { mnemonic: Mnemonic::MoveToUsp, size: L, ops: vec![An(6), Usp] }), // S1/S2
        ("move.l a0,usp", Instruction { mnemonic: Mnemonic::MoveToUsp, size: L, ops: vec![An(0), Usp] }),
        ("move.l a7,usp", Instruction { mnemonic: Mnemonic::MoveToUsp, size: L, ops: vec![An(7), Usp] }),
        ("move.l usp,a0", Instruction { mnemonic: Mnemonic::MoveFromUsp, size: L, ops: vec![Usp, An(0)] }),
        ("move.l usp,a7", Instruction { mnemonic: Mnemonic::MoveFromUsp, size: L, ops: vec![Usp, An(7)] }),
    ]
}
