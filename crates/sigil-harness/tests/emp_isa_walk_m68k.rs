//! A11: `.emp` can write every 68000 instruction the shared encoder can encode.
//!
//! DEMAND is brute force over the shared encoder, not a hand list: every
//! `sigil_isa::m68k::Mnemonic` (a `match` with no wildcard arm locks the list to
//! the enum) x four sizes x every 0-, 1- and 2-operand shape of [`pool`] goes
//! through `m68k::encode`; whatever it accepts is an instruction. Two accepted
//! forms are the same instruction when `m68k_decode::decode_exact` reads their
//! bytes back to the same thing, modulo a link-resolved displacement.
//!
//! SUPPLY is the `.emp` front end: each demanded instruction is spelled from its
//! DECODED form (sized, then with no suffix) and assembled. REFUSED means neither
//! spelling produces bytes that decode to the same instruction: a diagnostic, or
//! different bytes, both count.
use sigil_isa::m68k::{self, encode, family_name, Cond, Instruction, Mnemonic, Operand, Size, Xn, ALL_FAMILY_NAMES};
use sigil_isa::m68k_decode::{canonicalize, decode_exact};
use std::collections::{BTreeMap, BTreeSet};

#[path = "../../sigil-cli/tests/support/frontend_diff.rs"]
mod frontend_diff;

const CONDS: [Cond; 16] = [
    Cond::T, Cond::F, Cond::Hi, Cond::Ls, Cond::Cc, Cond::Cs, Cond::Ne, Cond::Eq, Cond::Vc,
    Cond::Vs, Cond::Pl, Cond::Mi, Cond::Ge, Cond::Lt, Cond::Gt, Cond::Le,
];
fn cond_name(c: Cond) -> &'static str {
    match c {
        Cond::T => "t", Cond::F => "f", Cond::Hi => "hi", Cond::Ls => "ls", Cond::Cc => "cc",
        Cond::Cs => "cs", Cond::Ne => "ne", Cond::Eq => "eq", Cond::Vc => "vc", Cond::Vs => "vs",
        Cond::Pl => "pl", Cond::Mi => "mi", Cond::Ge => "ge", Cond::Lt => "lt", Cond::Gt => "gt",
        Cond::Le => "le",
    }
}

macro_rules! unit_mnemonics {
    ($($v:ident),* $(,)?) => {
        #[allow(dead_code)]
        fn locked(m: Mnemonic) {
            match m { $(Mnemonic::$v => {}),* Mnemonic::Scc(_) | Mnemonic::Bcc(_) | Mnemonic::Dbcc(_) => {} }
        }
        fn all_mnemonics() -> Vec<Mnemonic> {
            let mut v = vec![$(Mnemonic::$v),*];
            for c in CONDS { v.push(Mnemonic::Scc(c)); v.push(Mnemonic::Bcc(c)); v.push(Mnemonic::Dbcc(c)); }
            v
        }
    };
}
unit_mnemonics! {
    Move, Movea, Add, Adda, Sub, Suba, And, Or, Eor, Cmp, Cmpa, Muls, Mulu, Divs, Divu,
    Addi, Subi, Andi, Ori, Eori, Cmpi, Moveq, Addq, Subq, Asl, Asr, Lsl, Lsr, Rol, Ror, Roxl, Roxr,
    Btst, Bset, Bclr, Bchg, Clr, Neg, Not, Tst, Tas, Jmp, Jsr, Lea, Pea, Nop, Rts, Rte, Trap, Swap,
    Ext, Illegal, Bra, Bsr, Movem, Movep, Addx, Cmpm, Subx, Abcd, Sbcd, Negx, Nbcd, MoveToSr,
    MoveFromSr, MoveToCcr, AndiCcr, OriCcr, EoriCcr, AndiSr, OriSr, EoriSr, MoveToUsp, MoveFromUsp,
    Exg, Chk, Link, Unlk, Reset, Rtr, Trapv, Stop,
}

fn pool() -> Vec<Operand> {
    use Operand::*;
    vec![
        Dn(1), An(2), Ind(3), PostInc(4), PreDec(5), Disp16An(8, 1),
        Disp8AnXn { d: 4, an: 2, xn: Xn::D(3), long: false },
        Pcd8Xn { d: -4, xn: Xn::D(0), long: false },
        AbsW(0x1234), AbsL(0x00FF_8000), Pcd16(-4), Imm(1), RegList(0x0301), Disp(-4), Ccr, Sr, Usp,
    ]
}

fn mnem_text(m: Mnemonic) -> String {
    match m {
        Mnemonic::Scc(c) => format!("s{}", cond_name(c)),
        Mnemonic::Bcc(c) => format!("b{}", cond_name(c)),
        Mnemonic::Dbcc(c) => format!("db{}", cond_name(c)),
        other => frontend_diff::mnemonic(family_name(other)).to_string(),
    }
}

fn op_text(o: &Operand) -> String {
    match *o {
        Operand::Dn(n) => format!("d{n}"),
        Operand::An(n) => format!("a{n}"),
        Operand::Ind(n) => format!("(a{n})"),
        Operand::PostInc(n) => format!("(a{n})+"),
        Operand::PreDec(n) => format!("-(a{n})"),
        Operand::Disp16An(d, n) => format!("{d}(a{n})"),
        Operand::Disp8AnXn { d, an, xn, long } => format!("{d}(a{an},{}.{})", xn_text(xn), if long { 'l' } else { 'w' }),
        Operand::Pcd8Xn { xn, long, .. } => format!(".tgt(pc,{}.{})", xn_text(xn), if long { 'l' } else { 'w' }),
        Operand::AbsW(w) if w < 0 => format!("(${:08X}).w", w as i32 as u32),
        Operand::AbsW(w) => format!("(${w:04X}).w"),
        Operand::AbsL(a) => format!("(${:08X}).l", a as u32),
        Operand::Pcd16(_) => ".tgt(pc)".into(),
        Operand::Imm(v) => format!("#{v}"),
        Operand::RegList(m) => reglist(m),
        Operand::Disp(_) => ".tgt".into(),
        Operand::Ccr => "ccr".into(),
        Operand::Sr => "sr".into(),
        Operand::Usp => "usp".into(),
    }
}
fn xn_text(x: Xn) -> String {
    match x { Xn::D(n) => format!("d{n}"), Xn::A(n) => format!("a{n}") }
}
fn reglist(mask: u16) -> String {
    let mut p = Vec::new();
    for i in 0..8 { if mask & (1 << i) != 0 { p.push(format!("d{i}")); } }
    for i in 0..8 { if mask & (1 << (8 + i)) != 0 { p.push(format!("a{i}")); } }
    p.join("/")
}

fn size_char(s: Size) -> char {
    match s { Size::B => 'b', Size::W => 'w', Size::L => 'l', Size::S => 's' }
}

/// The instruction with every link-resolved displacement zeroed.
fn modulo_disp(i: &Instruction) -> Instruction {
    let mut c = canonicalize(i);
    for o in &mut c.ops {
        match o {
            Operand::Pcd16(d) => *d = 0,
            Operand::Pcd8Xn { d, .. } => *d = 0,
            Operand::Disp(d) => *d = 0,
            _ => {}
        }
    }
    c
}

fn emp_src(line: &str) -> String {
    format!("module m\n@allow(\"stack.unbalanced\")\nproc f() {{\n.tgt:\n    nop\n    {line}\n    rts\n}}\n")
}

/// One demanded instruction: the form first given to the encoder, and what its bytes decode to.
struct Demand {
    by_decode: BTreeMap<String, Instruction>,
    families: BTreeSet<&'static str>,
    accepted: usize,
    undecodable: Vec<String>,
}

fn demand() -> Demand {
    let pool = pool();
    let mut shapes: Vec<Vec<Operand>> = vec![vec![]];
    for a in &pool {
        shapes.push(vec![*a]);
    }
    for a in &pool {
        for b in &pool {
            shapes.push(vec![*a, *b]);
        }
    }
    let mut out = Demand { by_decode: BTreeMap::new(), families: BTreeSet::new(), accepted: 0, undecodable: Vec::new() };
    for m in all_mnemonics() {
        for size in [Size::B, Size::W, Size::L, Size::S] {
            for ops in &shapes {
                let inst = Instruction { mnemonic: m, size, ops: ops.clone() };
                let Ok(bytes) = encode(&inst) else { continue };
                out.accepted += 1;
                out.families.insert(family_name(m));
                match decode_exact(&bytes) {
                    Ok(d) => {
                        out.by_decode.entry(format!("{:?}", modulo_disp(&d))).or_insert(d);
                    }
                    Err(e) => out.undecodable.push(format!("{inst:?}: {e:?}")),
                }
            }
        }
    }
    out
}

/// Assemble `line` inside the frame and require the bytes between the frame's
/// `nop` and `rts` to decode to `want` (modulo displacement).
fn emp_writes(line: &str, want: &Instruction) -> Result<(), String> {
    let bytes = frontend_diff::emp_assemble(&emp_src(line)).map_err(|e| e.join(" | "))?;
    if bytes.len() < 6 || bytes[..2] != [0x4E, 0x71] || bytes[bytes.len() - 2..] != [0x4E, 0x75] {
        return Err(format!("frame lost: {bytes:02X?}"));
    }
    let mid = &bytes[2..bytes.len() - 2];
    let got = decode_exact(mid).map_err(|e| format!("emp bytes {mid:02X?} do not decode: {e:?}"))?;
    if modulo_disp(&got) == modulo_disp(want) {
        Ok(())
    } else {
        Err(format!("DIFFERENT INSTRUCTION {mid:02X?} = {got:?}"))
    }
}

/// Families `.emp` still refuses, with the number of refused instructions in
/// each. A11 empties this table one task at a time; a row whose count is no
/// longer true fails the walk, so a closed gap cannot keep its row.
const KNOWN_REFUSED: &[(&str, usize)] = &[
    ("move-to-usp", 1), ("move-from-usp", 1), ("move-to-ccr", 11), ("eori-ccr", 1), ("movem", 4),
    ("link", 1), ("unlk", 1),
    ("stop", 1), ("andi-sr", 1), ("ori-sr", 1), ("eori-sr", 1),
    ("reset", 1),
    ("chk", 11), ("trapv", 1),
];

/// The floor under the demand, measured at sigil `55c6ec32`. A pool or mnemonic
/// list that shrinks fails here instead of passing with less work done.
const MIN_ACCEPTED: usize = 2439;
const MIN_DISTINCT: usize = 1527;

#[test]
fn the_walk_demands_every_family_of_the_shared_table() {
    let d = demand();
    assert!(d.undecodable.is_empty(), "encoder output the decoder cannot read: {:#?}", d.undecodable);
    let missing: Vec<&str> = ALL_FAMILY_NAMES.iter().copied().filter(|f| !d.families.contains(f)).collect();
    assert!(missing.is_empty(), "families of ALL_FAMILY_NAMES with no encodable form in the pool: {missing:?}");
    assert!(d.accepted >= MIN_ACCEPTED, "encoder accepted {} forms, floor {MIN_ACCEPTED}", d.accepted);
    assert!(d.by_decode.len() >= MIN_DISTINCT, "{} distinct instructions, floor {MIN_DISTINCT}", d.by_decode.len());
}

#[test]
fn emp_assembles_every_68000_instruction_the_shared_encoder_accepts() {
    let d = demand();
    let mut refused: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for inst in d.by_decode.values() {
        let mn = mnem_text(inst.mnemonic);
        let ops = inst.ops.iter().map(op_text).collect::<Vec<_>>().join(", ");
        let sized = format!("{mn}.{} {ops}", size_char(inst.size));
        let bare = format!("{mn} {ops}");
        if let (Err(a), Err(b)) = (emp_writes(&sized, inst), emp_writes(&bare, inst)) {
            refused.entry(family_name(inst.mnemonic)).or_default().push(format!("`{sized}`: {a}  ||  `{bare}`: {b}"));
        }
    }
    let got: BTreeMap<&str, usize> = refused.iter().map(|(f, rows)| (*f, rows.len())).collect();
    let want: BTreeMap<&str, usize> = KNOWN_REFUSED.iter().copied().collect();
    assert_eq!(got, want, "refused instructions per family differ from KNOWN_REFUSED:\n{refused:#?}");
}

#[test]
fn the_walk_can_see_a_refusal() {
    let nop = Instruction { mnemonic: Mnemonic::Nop, size: Size::W, ops: vec![] };
    let rts = Instruction { mnemonic: Mnemonic::Rts, size: Size::W, ops: vec![] };
    assert_eq!(emp_writes("nop", &nop), Ok(()));
    let unknown = emp_writes("frobnicate d0", &nop).unwrap_err();
    assert!(unknown.contains("not a recognized 68000 mnemonic"), "{unknown}");
    let wrong = emp_writes("nop", &rts).unwrap_err();
    assert!(wrong.starts_with("DIFFERENT INSTRUCTION"), "{wrong}");
}
