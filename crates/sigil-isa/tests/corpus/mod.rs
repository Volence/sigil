//! The single canonical Sigil Z80 asl-oracle corpus — one `(snippet, Instruction)`
//! pair per catalog §2 form. This is the ONE list shared by the golden-vector
//! generator (`src/bin/gen_z80_vectors.rs`, via `#[path]`), the golden CI test
//! (`tests/z80_golden.rs`), and Task 9's completeness gate (`tests/completeness.rs`),
//! all of which pull it in with `mod corpus;`. Each snippet string is EXACTLY what
//! `asl` is fed, so it matches the committed golden file's keys character-for-character.
//!
//! Cargo does not compile a `tests/<name>/mod.rs` subdirectory module as its own
//! integration-test binary, so it is safe to share. `#![allow(dead_code)]` silences
//! per-binary "unused" warnings when a consumer only reads the snippet strings.
//!
//! Representation choices baked in (must match the encoder's operand model):
//! - `ex (sp),hl` -> `Ex, [Pair(Sp), Pair(Hl)]` (the model has no `(sp)` indirect;
//!   the `Ex` encoder special-cases `[Pair(Sp), Pair(Hl)]` -> `E3`).
//! - `im 1` -> `Im, [Imm8(1)]` (interrupt mode carried as `Imm8`).
//! - `ld i,a` / `ld r,a` -> `Ld, [RegI, Reg(A)]` / `Ld, [RegR, Reg(A)]` (via the
//!   `RegI`/`RegR` operands, not the optional `LdIA`/`LdRA` sugar mnemonics).
#![allow(dead_code)]

use sigil_isa::z80::{Cond, IndexReg, Instruction, Mnemonic, Operand, Reg16, Reg8};

use Cond::{Nz, Z};
use IndexReg::{Ix as IxR, Iy as IyR};
use Mnemonic::*;
use Operand::{
    AfShadow, Cc, Imm16, Imm8, IndBc, IndDe, IndHl, Indexed, Mem, Pair, Reg, RegI, RegR, Rel,
};
use Reg16::{Af, Bc, De, Hl, Ix, Iy, Sp};
use Reg8::{A, B, C, D, H, L}; // no E: the canonical corpus uses `rrc a`, not `rrc e`

fn inst(m: Mnemonic, ops: Vec<Operand>) -> Instruction {
    Instruction { mnemonic: m, ops }
}

/// The definitive M0 Z80 corpus: every catalog `(mnemonic, operand-form)`, paired with
/// the `Instruction` value its asl snippet parses to. Snippet strings are verbatim asl
/// input and match the committed golden file's keys exactly.
pub fn corpus() -> Vec<(&'static str, Instruction)> {
    vec![
        // --- base (unprefixed) ---
        ("nop", inst(Nop, vec![])),
        ("ld b,a", inst(Ld, vec![Reg(B), Reg(A)])),
        ("ld d,0", inst(Ld, vec![Reg(D), Imm8(0)])),
        ("ld a,(hl)", inst(Ld, vec![Reg(A), IndHl])),
        ("ld (hl),c", inst(Ld, vec![IndHl, Reg(C)])),
        ("ld (hl),0", inst(Ld, vec![IndHl, Imm8(0)])),
        ("ld (de),a", inst(Ld, vec![IndDe, Reg(A)])),
        ("ld a,(de)", inst(Ld, vec![Reg(A), IndDe])),
        ("ld (bc),a", inst(Ld, vec![IndBc, Reg(A)])),
        ("ld a,(bc)", inst(Ld, vec![Reg(A), IndBc])),
        ("ld a,(1234h)", inst(Ld, vec![Reg(A), Mem(0x1234)])),
        ("ld (1234h),a", inst(Ld, vec![Mem(0x1234), Reg(A)])),
        ("ld bc,1234h", inst(Ld, vec![Pair(Bc), Imm16(0x1234)])),
        ("ld de,8DFCh", inst(Ld, vec![Pair(De), Imm16(0x8DFC)])),
        ("ld hl,1234h", inst(Ld, vec![Pair(Hl), Imm16(0x1234)])),
        ("ld sp,1234h", inst(Ld, vec![Pair(Sp), Imm16(0x1234)])),
        ("ld hl,(1234h)", inst(Ld, vec![Pair(Hl), Mem(0x1234)])),
        ("ld (1234h),hl", inst(Ld, vec![Mem(0x1234), Pair(Hl)])),
        ("ld sp,hl", inst(Ld, vec![Pair(Sp), Pair(Hl)])),
        ("inc a", inst(Inc, vec![Reg(A)])),
        ("inc hl", inst(Inc, vec![Pair(Hl)])),
        ("inc (hl)", inst(Inc, vec![IndHl])),
        ("dec a", inst(Dec, vec![Reg(A)])),
        ("dec bc", inst(Dec, vec![Pair(Bc)])),
        ("dec (hl)", inst(Dec, vec![IndHl])),
        ("add a,a", inst(Add, vec![Reg(A), Reg(A)])),
        ("add a,5", inst(Add, vec![Reg(A), Imm8(5)])),
        ("add a,(hl)", inst(Add, vec![Reg(A), IndHl])),
        ("add hl,de", inst(Add, vec![Pair(Hl), Pair(De)])),
        ("adc a,0", inst(Adc, vec![Reg(A), Imm8(0)])),
        ("adc a,h", inst(Adc, vec![Reg(A), Reg(H)])),
        ("adc a,(hl)", inst(Adc, vec![Reg(A), IndHl])),
        ("sub c", inst(Sub, vec![Reg(C)])),
        ("sub 5", inst(Sub, vec![Imm8(5)])),
        ("sbc a,a", inst(Sbc, vec![Reg(A), Reg(A)])),
        ("and 007h", inst(And, vec![Imm8(7)])),
        ("and c", inst(And, vec![Reg(C)])),
        ("or a", inst(Or, vec![Reg(A)])),
        ("or 5", inst(Or, vec![Imm8(5)])),
        ("xor a", inst(Xor, vec![Reg(A)])),
        ("cp b", inst(Cp, vec![Reg(B)])),
        ("cp 2", inst(Cp, vec![Imm8(2)])),
        ("cp (hl)", inst(Cp, vec![IndHl])),
        ("push bc", inst(Push, vec![Pair(Bc)])),
        ("push de", inst(Push, vec![Pair(De)])),
        ("push hl", inst(Push, vec![Pair(Hl)])),
        ("push af", inst(Push, vec![Pair(Af)])),
        ("pop bc", inst(Pop, vec![Pair(Bc)])),
        ("pop de", inst(Pop, vec![Pair(De)])),
        ("pop hl", inst(Pop, vec![Pair(Hl)])),
        ("pop af", inst(Pop, vec![Pair(Af)])),
        ("ex (sp),hl", inst(Ex, vec![Pair(Sp), Pair(Hl)])),
        ("ex de,hl", inst(Ex, vec![Pair(De), Pair(Hl)])),
        ("ex af,af'", inst(Ex, vec![Pair(Af), AfShadow])),
        ("exx", inst(Exx, vec![])),
        ("ret", inst(Ret, vec![])),
        ("ret z", inst(Ret, vec![Cc(Z)])),
        ("jr $", inst(Jr, vec![Rel(-2)])),
        ("jr z,$", inst(Jr, vec![Cc(Z), Rel(-2)])),
        ("jp 1234h", inst(Jp, vec![Imm16(0x1234)])),
        ("jp z,1234h", inst(Jp, vec![Cc(Z), Imm16(0x1234)])),
        ("jp (hl)", inst(Jp, vec![IndHl])),
        ("call 1234h", inst(Call, vec![Imm16(0x1234)])),
        ("call nz,1234h", inst(Call, vec![Cc(Nz), Imm16(0x1234)])),
        ("djnz $", inst(Djnz, vec![Rel(-2)])),
        ("rrca", inst(Rrca, vec![])),
        ("scf", inst(Scf, vec![])),
        ("ei", inst(Ei, vec![])),
        ("di", inst(Di, vec![])),
        // --- CB (rotate/shift/bit on r) ---
        ("rlc a", inst(Rlc, vec![Reg(A)])),
        ("rrc a", inst(Rrc, vec![Reg(A)])),
        ("rl b", inst(Rl, vec![Reg(B)])),
        ("rr l", inst(Rr, vec![Reg(L)])),
        ("sla c", inst(Sla, vec![Reg(C)])),
        ("sra a", inst(Sra, vec![Reg(A)])),
        ("srl a", inst(Srl, vec![Reg(A)])),
        ("bit 7,d", inst(Bit, vec![Operand::Bit(7), Reg(D)])),
        ("res 0,a", inst(Res, vec![Operand::Bit(0), Reg(A)])),
        ("set 5,b", inst(Set, vec![Operand::Bit(5), Reg(B)])),
        // --- ED (block ops / 16-bit mem loads / neg / im / ld i,a / ld r,a) ---
        ("ld (1234h),de", inst(Ld, vec![Mem(0x1234), Pair(De)])),
        ("ld (1234h),bc", inst(Ld, vec![Mem(0x1234), Pair(Bc)])),
        ("ld (1234h),sp", inst(Ld, vec![Mem(0x1234), Pair(Sp)])),
        ("ld de,(1234h)", inst(Ld, vec![Pair(De), Mem(0x1234)])),
        ("ld bc,(1234h)", inst(Ld, vec![Pair(Bc), Mem(0x1234)])),
        ("ld sp,(1234h)", inst(Ld, vec![Pair(Sp), Mem(0x1234)])),
        ("neg", inst(Neg, vec![])),
        ("im 1", inst(Im, vec![Imm8(1)])),
        ("ldir", inst(Ldir, vec![])),
        ("ld i,a", inst(Ld, vec![RegI, Reg(A)])),
        ("ld r,a", inst(Ld, vec![RegR, Reg(A)])),
        // --- DD (ix) ---
        ("ld ix,1234h", inst(Ld, vec![Pair(Ix), Imm16(0x1234)])),
        ("ld l,(ix+3)", inst(Ld, vec![Reg(L), Indexed { reg: IxR, disp: 3 }])),
        ("ld (ix+3),l", inst(Ld, vec![Indexed { reg: IxR, disp: 3 }, Reg(L)])),
        ("ld (ix+3),0", inst(Ld, vec![Indexed { reg: IxR, disp: 3 }, Imm8(0)])),
        ("inc (ix+3)", inst(Inc, vec![Indexed { reg: IxR, disp: 3 }])),
        ("dec (ix+3)", inst(Dec, vec![Indexed { reg: IxR, disp: 3 }])),
        ("add a,(ix+3)", inst(Add, vec![Reg(A), Indexed { reg: IxR, disp: 3 }])),
        ("add ix,de", inst(Add, vec![Pair(Ix), Pair(De)])),
        ("or (ix+3)", inst(Or, vec![Indexed { reg: IxR, disp: 3 }])),
        ("cp (ix+3)", inst(Cp, vec![Indexed { reg: IxR, disp: 3 }])),
        ("push ix", inst(Push, vec![Pair(Ix)])),
        ("pop ix", inst(Pop, vec![Pair(Ix)])),
        // --- FD (iy) ---
        ("ld iy,1234h", inst(Ld, vec![Pair(Iy), Imm16(0x1234)])),
        ("ld iy,(1234h)", inst(Ld, vec![Pair(Iy), Mem(0x1234)])),
        ("ld a,(iy+3)", inst(Ld, vec![Reg(A), Indexed { reg: IyR, disp: 3 }])),
        ("add iy,de", inst(Add, vec![Pair(Iy), Pair(De)])),
        ("push iy", inst(Push, vec![Pair(Iy)])),
        ("pop iy", inst(Pop, vec![Pair(Iy)])),
        // --- DDCB (bit/res/set on (ix+d)) ---
        ("bit 1,(ix+10)", inst(Bit, vec![Operand::Bit(1), Indexed { reg: IxR, disp: 10 }])),
        ("set 1,(ix+10)", inst(Set, vec![Operand::Bit(1), Indexed { reg: IxR, disp: 10 }])),
        ("res 1,(ix+10)", inst(Res, vec![Operand::Bit(1), Indexed { reg: IxR, disp: 10 }])),
        // --- FDCB (bit/res/set on (iy+d)) ---
        ("bit 1,(iy+10)", inst(Bit, vec![Operand::Bit(1), Indexed { reg: IyR, disp: 10 }])),
        ("set 1,(iy+10)", inst(Set, vec![Operand::Bit(1), Indexed { reg: IyR, disp: 10 }])),
        ("res 1,(iy+10)", inst(Res, vec![Operand::Bit(1), Indexed { reg: IyR, disp: 10 }])),
    ]
}
