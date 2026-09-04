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

/// The single-operand shift's operand row.
///
/// A DATA REGISTER is legal here and asl accepts it: `asl d0` assembles to
/// `E3 40`, the count-1 REGISTER form — identical bytes to `asl #1,d0`. (This
/// test previously asserted the opposite, "a data register is not it"; that was
/// sigil's own limitation written down as an ISA rule, and the assembler
/// refutes it. `s1disasm/build_tools/Linux-x86_64/asl`, md5
/// `61e672562465725a8c102288a7da9098`, `cpu 68000 -U`.)
///
/// What the row genuinely excludes is everything that is neither a data
/// register nor a MEMORY ALTERABLE address: `An`, `#imm`, and the two
/// PC-relative modes.
#[test]
fn single_operand_shift_row() {
    reject("lsr.w a0",   ins(Mnemonic::Lsr, Size::W, vec![Operand::An(0)]));
    reject("ror.w #4",   ins(Mnemonic::Ror, Size::W, vec![Operand::Imm(4)]));
    reject("roxl.w (d16,pc)", ins(Mnemonic::Roxl, Size::W, vec![Operand::Pcd16(8)]));
    accept("asr.w (a0)", ins(Mnemonic::Asr, Size::W, vec![Operand::Ind(0)]));
    accept("asl.w d0",   ins(Mnemonic::Asl, Size::W, vec![Operand::Dn(0)]));
    accept("roxl.w d0",  ins(Mnemonic::Roxl, Size::W, vec![Operand::Dn(0)]));
    // The register form's bytes are the count-1 two-operand form's, exactly.
    assert_eq!(
        encode(&ins(Mnemonic::Asl, Size::W, vec![Operand::Dn(0)])).unwrap(),
        encode(&ins(Mnemonic::Asl, Size::W, vec![Operand::Imm(1), Operand::Dn(0)])).unwrap(),
        "`asl.w d0` and `asl.w #1,d0` are one encoding (asl: both E3 40)"
    );
    // ...and the memory form has no count field, so only `#1` reaches it.
    accept("roxl.w #1,(a0)", ins(Mnemonic::Roxl, Size::W, vec![Operand::Imm(1), Operand::Ind(0)]));
    reject("roxl.w #2,(a0)", ins(Mnemonic::Roxl, Size::W, vec![Operand::Imm(2), Operand::Ind(0)]));
}

/// The `bchg` destination row is `bset`'s (DATA ALTERABLE) in both forms — no
/// `An`, no `#imm`, no PC-relative — and `move <ea>,ccr`'s source row is DATA,
/// which DOES admit both PC-relative modes and an immediate.
#[test]
fn bchg_and_move_to_ccr_rows() {
    reject("bchg #3,a0", ins(Mnemonic::Bchg, Size::B, vec![Operand::Imm(3), Operand::An(0)]));
    reject("bchg d2,a0", ins(Mnemonic::Bchg, Size::B, vec![Operand::Dn(2), Operand::An(0)]));
    reject("bchg #3,(d16,pc)", ins(Mnemonic::Bchg, Size::B, vec![Operand::Imm(3), Operand::Pcd16(8)]));
    reject("bchg #3,#$12", ins(Mnemonic::Bchg, Size::B, vec![Operand::Imm(3), Operand::Imm(0x12)]));
    accept("bchg #3,(a0)", ins(Mnemonic::Bchg, Size::B, vec![Operand::Imm(3), Operand::Ind(0)]));
    accept("bchg d2,-(a0)", ins(Mnemonic::Bchg, Size::B, vec![Operand::Dn(2), Operand::PreDec(0)]));
    reject("move.w a0,ccr", ins(Mnemonic::MoveToCcr, Size::W, vec![Operand::An(0), Operand::Ccr]));
    accept("move.w #$12,ccr", ins(Mnemonic::MoveToCcr, Size::W, vec![Operand::Imm(0x12), Operand::Ccr]));
    accept("move.w (d16,pc),ccr", ins(Mnemonic::MoveToCcr, Size::W, vec![Operand::Pcd16(8), Operand::Ccr]));
}

/// `exg` has no EA field at all: three register-pair forms and nothing else,
/// and USP exchanges only with an address register.
#[test]
fn exg_and_usp_take_registers_only() {
    reject("exg.l d0,(a0)", ins(Mnemonic::Exg, Size::L, vec![Operand::Dn(0), Operand::Ind(0)]));
    reject("exg.l #1,d0",   ins(Mnemonic::Exg, Size::L, vec![Operand::Imm(1), Operand::Dn(0)]));
    accept("exg.l d0,d1",   ins(Mnemonic::Exg, Size::L, vec![Operand::Dn(0), Operand::Dn(1)]));
    accept("exg.l a0,a1",   ins(Mnemonic::Exg, Size::L, vec![Operand::An(0), Operand::An(1)]));
    accept("exg.l d0,a1",   ins(Mnemonic::Exg, Size::L, vec![Operand::Dn(0), Operand::An(1)]));
    // Written the other way round, asl emits the SAME word; so does this.
    assert_eq!(
        encode(&ins(Mnemonic::Exg, Size::L, vec![Operand::An(1), Operand::Dn(0)])).unwrap(),
        encode(&ins(Mnemonic::Exg, Size::L, vec![Operand::Dn(0), Operand::An(1)])).unwrap(),
        "`exg a1,d0` and `exg d0,a1` are one encoding (asl: both C1 89)"
    );
    reject("move.l d0,usp", ins(Mnemonic::MoveToUsp, Size::L, vec![Operand::Dn(0), Operand::Usp]));
    reject("move.l usp,d0", ins(Mnemonic::MoveFromUsp, Size::L, vec![Operand::Usp, Operand::Dn(0)]));
    accept("move.l a6,usp", ins(Mnemonic::MoveToUsp, Size::L, vec![Operand::An(6), Operand::Usp]));
    accept("move.l usp,a6", ins(Mnemonic::MoveFromUsp, Size::L, vec![Operand::Usp, Operand::An(6)]));
}

/// Every one of these forms has exactly ONE legal size in the ISA, and asl
/// rejects the others outright. A wrong size here is not cosmetic: the old
/// `move ... ,sr` defect (an `.l` immediate read as `sr := 0`) is the shape.
#[test]
fn size_locked_forms_reject_every_other_size() {
    // `move <ea>,ccr` is the one form here with TWO accepted suffixes: asl takes
    // `.b` and `.w` and emits identical bytes for both, and rejects `.l`. The
    // equality is the point — the operand width is a word whatever was written,
    // so accepting `.b` cannot shorten the immediate.
    reject("move to ccr at .l",
           ins(Mnemonic::MoveToCcr, Size::L, vec![Operand::Dn(6), Operand::Ccr]));
    assert_eq!(
        encode(&ins(Mnemonic::MoveToCcr, Size::B, vec![Operand::Imm(0x12), Operand::Ccr])).unwrap(),
        encode(&ins(Mnemonic::MoveToCcr, Size::W, vec![Operand::Imm(0x12), Operand::Ccr])).unwrap(),
        "`move.b #$12,ccr` and `move.w #$12,ccr` are one encoding (asl: both 44 FC 00 12)"
    );
    accept("move.b d6,ccr", ins(Mnemonic::MoveToCcr, Size::B, vec![Operand::Dn(6), Operand::Ccr]));
    for sz in [Size::B, Size::W] {
        reject("exg at a non-long size", ins(Mnemonic::Exg, sz, vec![Operand::Dn(0), Operand::Dn(1)]));
        reject("move to usp at a non-long size",
               ins(Mnemonic::MoveToUsp, sz, vec![Operand::An(0), Operand::Usp]));
        reject("move from usp at a non-long size",
               ins(Mnemonic::MoveFromUsp, sz, vec![Operand::Usp, Operand::An(0)]));
    }
    for sz in [Size::B, Size::L] {
        reject("roxl memory form at a non-word size",
               ins(Mnemonic::Roxl, sz, vec![Operand::Ind(0)]));
    }
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
