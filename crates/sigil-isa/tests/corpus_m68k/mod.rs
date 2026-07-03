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

use sigil_isa::m68k::{Instruction, Mnemonic, Operand, Size, Xn};

fn mov(size: Size, src: Operand, dst: Operand) -> Instruction {
    Instruction { mnemonic: Mnemonic::Move, size, ops: vec![src, dst] }
}

use Operand::*;
use Size::{L, W};

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
        ("move.w (8,pc),d0", mov(W, Pcd16(8), Dn(0))),
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
    ]
}
