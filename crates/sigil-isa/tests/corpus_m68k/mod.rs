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
    ]
}
