//! Negative oracle for the 68000 effective-address classes.
//!
//! WHY THIS FILE IS HAND-WRITTEN. The golden corpus in `tests/corpus_m68k/` is
//! minted by feeding snippets to a real `asl`, so **an illegal form can never
//! appear in it** — `asl` refuses to assemble one. That makes the corpus a
//! positive-only oracle, and it is why `encode_ea` accepted address-register mode
//! everywhere for as long as it did: nothing in the suite could express the
//! question. Every assertion below therefore has to be written by hand.
//!
//! THE CLASS (sigil lens sweep 2026-08-13, seat CGa / finding S1). An earlier fix
//! patched `add/sub/and/or dN,aM` — one site of a class with five members. The
//! other four were still live, silent, and reachable from ordinary `.emp` source.
//! Each has an opcode neighbour reachable by exactly the mode that was wrongly
//! accepted, so the assembler did not emit an illegal instruction that traps — it
//! emitted a DIFFERENT, VALID instruction. Decoded with Capstone 5.0.7 in
//! `CS_MODE_M68K_000`:
//!
//! | source        | was emitted | the 68000 actually executes | consequence |
//! |---------------|-------------|-----------------------------|-------------|
//! | `bset d0,a1`  | `01C9`      | `movep.l d0,$8(a1)`         | memory WRITE; 2 bytes emitted vs 4 consumed, so it eats the next opcode word and desyncs the instruction stream |
//! | `sne a3`      | `56CB`      | `dbne d3,…`                 | backward branch to garbage |
//! | `eor.w d0,a1` | `B149`      | `cmpm.w (a1)+,(a0)+`        | two address registers advance, two stray reads |
//! | `pea d0`      | `4840`      | `swap d0`                   | nothing pushed; stack depth wrong for the rest of the routine |
use sigil_isa::m68k::*;

fn ins(mnemonic: Mnemonic, size: Size, ops: Vec<Operand>) -> Instruction {
    Instruction { mnemonic, size, ops }
}

#[track_caller]
fn reject(what: &str, i: Instruction) {
    match encode(&i) {
        Err(_) => {}
        Ok(bytes) => panic!("{what} must not encode, but produced {bytes:02X?}"),
    }
}

#[track_caller]
fn accept(what: &str, i: Instruction) -> Vec<u8> {
    let bytes =
        encode(&i).unwrap_or_else(|e| panic!("{what} is legal and must still encode, got {e:?}"));
    // Every accepted form must also survive the decode round trip — the
    // categorical form of the alias defense this file probes by hand.
    if let Err(msg) = sigil_isa::m68k_decode::roundtrip_check(&i, &bytes) {
        panic!("{what}: {msg}");
    }
    bytes
}

/// The four exact-alias classes. These are the dangerous ones: silently valid,
/// silently WRONG. Named individually so a failure says which one came back.
#[test]
fn alias_classes_are_rejected() {
    reject("bset d0,a1 (aliases movep.l)", ins(Mnemonic::Bset, Size::L, vec![Operand::Dn(0), Operand::An(1)]));
    reject("bclr d0,a1 (same family)",     ins(Mnemonic::Bclr, Size::L, vec![Operand::Dn(0), Operand::An(1)]));
    reject("btst d0,a1 (same family)",     ins(Mnemonic::Btst, Size::L, vec![Operand::Dn(0), Operand::An(1)]));
    reject("sne a3 (aliases dbne)",        ins(Mnemonic::Scc(Cond::Ne), Size::B, vec![Operand::An(3)]));
    reject("eor.w d0,a1 (aliases cmpm.w)", ins(Mnemonic::Eor, Size::W, vec![Operand::Dn(0), Operand::An(1)]));
    reject("eor.l d0,a1 (aliases cmpm.l)", ins(Mnemonic::Eor, Size::L, vec![Operand::Dn(0), Operand::An(1)]));
    reject("pea d0 (aliases swap)",        ins(Mnemonic::Pea, Size::L, vec![Operand::Dn(0)]));
    reject("pea a0 (no An form)",          ins(Mnemonic::Pea, Size::L, vec![Operand::An(0)]));
}

/// The non-alias illegal forms (finding S2). These decode as illegal/undefined
/// rather than as a different instruction, so they trap at runtime instead of
/// corrupting silently — still `Ok` from an encoder that has no encoding for them.
#[test]
fn address_register_is_not_a_data_operand() {
    reject("tst.w a0",      ins(Mnemonic::Tst,  Size::W, vec![Operand::An(0)]));
    reject("tst.l a0",      ins(Mnemonic::Tst,  Size::L, vec![Operand::An(0)]));
    reject("clr.w a0",      ins(Mnemonic::Clr,  Size::W, vec![Operand::An(0)]));
    reject("neg.w a0",      ins(Mnemonic::Neg,  Size::W, vec![Operand::An(0)]));
    reject("not.w a0",      ins(Mnemonic::Not,  Size::W, vec![Operand::An(0)]));
    reject("addi.w #1,a0",  ins(Mnemonic::Addi, Size::W, vec![Operand::Imm(1), Operand::An(0)]));
    reject("subi.w #1,a0",  ins(Mnemonic::Subi, Size::W, vec![Operand::Imm(1), Operand::An(0)]));
    reject("cmpi.w #1,a0",  ins(Mnemonic::Cmpi, Size::W, vec![Operand::Imm(1), Operand::An(0)]));
    reject("and.w a0,d0",   ins(Mnemonic::And,  Size::W, vec![Operand::An(0), Operand::Dn(0)]));
    reject("or.w a0,d0",    ins(Mnemonic::Or,   Size::W, vec![Operand::An(0), Operand::Dn(0)]));
    reject("muls.w a0,d0",  ins(Mnemonic::Muls, Size::W, vec![Operand::An(0), Operand::Dn(0)]));
    reject("divu.w a0,d0",  ins(Mnemonic::Divu, Size::W, vec![Operand::An(0), Operand::Dn(0)]));
}

/// `TST`'s operand is DATA ALTERABLE on the MC68000. PC-relative and immediate
/// operands became legal only on the 68020, so all nine widened forms (three EA
/// classes x .b/.w/.l) are illegal-instruction traps on a Genesis. Widening the
/// row back to plain DATA is invisible to the opcode sweep — the sweep's oracle
/// is `encode()`, which reads the same `EaSet` constant the decoder reads — so
/// this is the only place the tightening is pinned.
#[test]
fn tst_destination_is_data_alterable() {
    for size in [Size::B, Size::W, Size::L] {
        reject("tst (d16,pc)", ins(Mnemonic::Tst, size, vec![Operand::Pcd16(8)]));
        reject(
            "tst (d8,pc,xn)",
            ins(Mnemonic::Tst, size, vec![Operand::Pcd8Xn { d: 8, xn: Xn::D(0), long: false }]),
        );
        reject("tst #imm", ins(Mnemonic::Tst, size, vec![Operand::Imm(1)]));
    }
    // The alterable data modes stay legal at every size.
    accept("tst.b (a0)", ins(Mnemonic::Tst, Size::B, vec![Operand::Ind(0)]));
    accept("tst.w (d16,a0)", ins(Mnemonic::Tst, Size::W, vec![Operand::Disp16An(8, 0)]));
    accept("tst.l (xxx).w", ins(Mnemonic::Tst, Size::L, vec![Operand::AbsW(0x1000)]));
}

/// An is a word/long operand only — there is no byte access to an address
/// register anywhere in the 68000, including where An is otherwise legal.
#[test]
fn address_register_has_no_byte_form() {
    reject("add.b a0,d0",   ins(Mnemonic::Add,  Size::B, vec![Operand::An(0), Operand::Dn(0)]));
    reject("cmp.b a0,d0",   ins(Mnemonic::Cmp,  Size::B, vec![Operand::An(0), Operand::Dn(0)]));
    reject("addq.b #1,a0",  ins(Mnemonic::Addq, Size::B, vec![Operand::Imm(1), Operand::An(0)]));
    reject("subq.b #1,a0",  ins(Mnemonic::Subq, Size::B, vec![Operand::Imm(1), Operand::An(0)]));
    reject("move.b d0,a0",  ins(Mnemonic::Move, Size::B, vec![Operand::Dn(0), Operand::An(0)]));
    // ...but the word/long forms of the same instructions ARE legal.
    accept("addq.w #1,a0",  ins(Mnemonic::Addq, Size::W, vec![Operand::Imm(1), Operand::An(0)]));
    accept("add.w a0,d0",   ins(Mnemonic::Add,  Size::W, vec![Operand::An(0), Operand::Dn(0)]));
    accept("cmp.w a0,d0",   ins(Mnemonic::Cmp,  Size::W, vec![Operand::An(0), Operand::Dn(0)]));
}

/// `lea`/`jmp`/`jsr`/`pea` take a CONTROL address: no register direct, no
/// autoincrement/decrement (the side effect is meaningless for an address), no
/// immediate.
#[test]
fn control_addressing_rejects_non_addresses() {
    reject("lea (a0)+,a1",  ins(Mnemonic::Lea, Size::L, vec![Operand::PostInc(0), Operand::An(1)]));
    reject("lea -(a0),a1",  ins(Mnemonic::Lea, Size::L, vec![Operand::PreDec(0), Operand::An(1)]));
    reject("lea #$1000,a0", ins(Mnemonic::Lea, Size::L, vec![Operand::Imm(0x1000), Operand::An(0)]));
    reject("lea d0,a0",     ins(Mnemonic::Lea, Size::L, vec![Operand::Dn(0), Operand::An(0)]));
    reject("jmp (a0)+",     ins(Mnemonic::Jmp, Size::L, vec![Operand::PostInc(0)]));
    reject("jmp d0",        ins(Mnemonic::Jmp, Size::L, vec![Operand::Dn(0)]));
    reject("jsr -(a0)",     ins(Mnemonic::Jsr, Size::L, vec![Operand::PreDec(0)]));
    reject("pea -(a0)",     ins(Mnemonic::Pea, Size::L, vec![Operand::PreDec(0)]));
    // The legal control modes still encode, PC-relative included.
    accept("lea (a0),a1",       ins(Mnemonic::Lea, Size::L, vec![Operand::Ind(0), Operand::An(1)]));
    accept("lea (d16,a0),a1",   ins(Mnemonic::Lea, Size::L, vec![Operand::Disp16An(8, 0), Operand::An(1)]));
    accept("lea (d16,pc),a1",   ins(Mnemonic::Lea, Size::L, vec![Operand::Pcd16(8), Operand::An(1)]));
    accept("jmp (xxx).l",       ins(Mnemonic::Jmp, Size::L, vec![Operand::AbsL(0x1000)]));
    accept("pea (xxx).l",       ins(Mnemonic::Pea, Size::L, vec![Operand::AbsL(0x1000)]));
}

/// The single-operand shift is the MEMORY form; a data register is not it.
#[test]
fn memory_shift_is_memory_only() {
    reject("asl.w d0",  ins(Mnemonic::Asl, Size::W, vec![Operand::Dn(0)]));
    reject("lsr.w a0",  ins(Mnemonic::Lsr, Size::W, vec![Operand::An(0)]));
    reject("ror.w #4",  ins(Mnemonic::Ror, Size::W, vec![Operand::Imm(4)]));
    accept("asr.w (a0)", ins(Mnemonic::Asr, Size::W, vec![Operand::Ind(0)]));
}

/// MOVEM's legal modes differ BY DIRECTION, and the encoder used to pass a
/// destination field for both — which admitted an illegal store AND rejected a
/// legal load. Both directions are asserted so neither can regress alone.
#[test]
fn movem_direction_governs_its_modes() {
    let regs = Operand::RegList(0x00FF);
    // STORE (regs -> memory): control alterable + `-(An)`. `(An)+` is NOT a store mode.
    reject("movem.l d0-d7,(a0)+",
           ins(Mnemonic::Movem, Size::L, vec![regs, Operand::PostInc(0)]));
    reject("movem.l d0-d7,(d16,pc)",
           ins(Mnemonic::Movem, Size::L, vec![regs, Operand::Pcd16(8)]));
    accept("movem.l d0-d7,-(a7)",
           ins(Mnemonic::Movem, Size::L, vec![regs, Operand::PreDec(7)]));
    accept("movem.l d0-d7,(a0)",
           ins(Mnemonic::Movem, Size::L, vec![regs, Operand::Ind(0)]));

    // LOAD (memory -> regs): control + `(An)+`. `-(An)` is NOT a load mode, and
    // PC-relative IS legal here — MOVEM load explicitly permits it. That form used
    // to be rejected outright.
    reject("movem.l -(a0),d0-d7",
           ins(Mnemonic::Movem, Size::L, vec![Operand::PreDec(0), regs]));
    accept("movem.l (a7)+,d0-d7",
           ins(Mnemonic::Movem, Size::L, vec![Operand::PostInc(7), regs]));
    accept("movem.l (d16,pc),d0-d7",
           ins(Mnemonic::Movem, Size::L, vec![Operand::Pcd16(8), regs]));
}

/// Guard against the fix being too STRICT. An An destination on `move` is not a
/// grudging exception: MOVEA *is* MOVE with destination mode 001, one opcode
/// layout, so these must keep encoding — and to the same bytes `movea` gives.
#[test]
fn address_register_stays_legal_where_the_isa_allows_it() {
    let via_move  = accept("move.l d0,a0",  ins(Mnemonic::Move,  Size::L, vec![Operand::Dn(0), Operand::An(0)]));
    let via_movea = accept("movea.l d0,a0", ins(Mnemonic::Movea, Size::L, vec![Operand::Dn(0), Operand::An(0)]));
    assert_eq!(via_move, via_movea, "an An-destination MOVE must emit the MOVEA bytes");

    // The address-arithmetic family takes ANY source mode, An included.
    accept("adda.w a0,a1", ins(Mnemonic::Adda, Size::W, vec![Operand::An(0), Operand::An(1)]));
    accept("suba.l a0,a1", ins(Mnemonic::Suba, Size::L, vec![Operand::An(0), Operand::An(1)]));
    accept("cmpa.w a0,a1", ins(Mnemonic::Cmpa, Size::W, vec![Operand::An(0), Operand::An(1)]));
    // An source into a data register is legal for ADD/SUB/CMP (not AND/OR).
    accept("sub.w a0,d0",  ins(Mnemonic::Sub,  Size::W, vec![Operand::An(0), Operand::Dn(0)]));
    // ...and the plain data forms are of course untouched.
    accept("tst.w d0",     ins(Mnemonic::Tst,  Size::W, vec![Operand::Dn(0)]));
    accept("clr.w (a0)",   ins(Mnemonic::Clr,  Size::W, vec![Operand::Ind(0)]));
    accept("eor.w d0,d1",  ins(Mnemonic::Eor,  Size::W, vec![Operand::Dn(0), Operand::Dn(1)]));
    accept("bset d0,(a1)", ins(Mnemonic::Bset, Size::B, vec![Operand::Dn(0), Operand::Ind(1)]));
}

/// A write position must still refuse the three non-alterable modes.
#[test]
fn non_alterable_modes_cannot_be_written() {
    reject("move.w d0,#imm",     ins(Mnemonic::Move, Size::W, vec![Operand::Dn(0), Operand::Imm(1)]));
    reject("move.w d0,(d16,pc)", ins(Mnemonic::Move, Size::W, vec![Operand::Dn(0), Operand::Pcd16(8)]));
    reject("clr.w (d16,pc)",     ins(Mnemonic::Clr,  Size::W, vec![Operand::Pcd16(8)]));
    // `encode_alu_ea` used to validate its DESTINATION as if it were a source,
    // which let both of these through.
    reject("add.w d0,#5",        ins(Mnemonic::Add,  Size::W, vec![Operand::Dn(0), Operand::Imm(5)]));
    reject("add.w d0,(d16,pc)",  ins(Mnemonic::Add,  Size::W, vec![Operand::Dn(0), Operand::Pcd16(8)]));
}
