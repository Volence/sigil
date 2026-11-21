//! A11: `.emp` can write every Z80 instruction the shared encoder can encode, and
//! every 16-bit field of one can be a link-time symbol.
//!
//! DEMAND is brute force over `sigil_isa::z80::encode`: every `Mnemonic` (the
//! `mnemonics!` match has no wildcard arm, so the list is locked to the enum) x
//! every 0-, 1- and 2-operand shape of [`pool`]. Distinct BYTES are distinct
//! instructions. SUPPLY: the same instruction spelled for `.emp` must assemble
//! to exactly those bytes. REFUSED is a diagnostic or any other bytes.
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_isa::z80::{encode, Cond, IndexReg, Instruction, Mnemonic, Operand, Reg16, Reg8};
use sigil_span::Level;
use std::collections::BTreeMap;

macro_rules! mnemonics {
    ($($v:ident => $s:expr),* $(,)?) => {
        #[allow(dead_code)]
        fn spelling(m: Mnemonic) -> &'static str { match m { $(Mnemonic::$v => $s),* } }
        fn all_mnemonics() -> Vec<(Mnemonic, &'static str)> { vec![$((Mnemonic::$v, $s)),*] }
    };
}
mnemonics! {
    Nop => "nop", Ld => "ld", Add => "add", Adc => "adc", Sub => "sub",
    Sbc => "sbc", And => "and", Or => "or", Xor => "xor", Cp => "cp",
    Inc => "inc", Dec => "dec", Push => "push", Pop => "pop", Ex => "ex",
    Exx => "exx", Ret => "ret", Jr => "jr", Jp => "jp", Call => "call",
    Djnz => "djnz", Rrca => "rrca", Scf => "scf", Ei => "ei", Di => "di",
    Bit => "bit", Res => "res", Set => "set", Srl => "srl", Rr => "rr",
    Sla => "sla", Rlc => "rlc", Rrc => "rrc", Rl => "rl", Sra => "sra",
    Neg => "neg", Im => "im",
    Ldi => "ldi", Ldir => "ldir", Ldd => "ldd", Lddr => "lddr",
    Cpi => "cpi", Cpd => "cpd", Cpir => "cpir", Cpdr => "cpdr",
    Ini => "ini", Ind => "ind", Inir => "inir", Indr => "indr",
    Outi => "outi", Outd => "outd", Otir => "otir", Otdr => "otdr",
    In => "in", Out => "out", Reti => "reti", Retn => "retn",
    Rrd => "rrd", Rld => "rld",
    LdIA => "ld", LdRA => "ld",
    Rlca => "rlca", Rla => "rla", Rra => "rra", Daa => "daa",
    Cpl => "cpl", Ccf => "ccf", Halt => "halt", Rst => "rst",
}

fn pool() -> Vec<Operand> {
    use Operand::*;
    let mut v = vec![];
    for r in [Reg8::A, Reg8::B, Reg8::C, Reg8::D, Reg8::E, Reg8::H, Reg8::L] { v.push(Reg(r)); }
    for p in [Reg16::Bc, Reg16::De, Reg16::Hl, Reg16::Sp, Reg16::Af, Reg16::Ix, Reg16::Iy] { v.push(Pair(p)); }
    v.extend([IndHl, IndBc, IndDe, Indexed { reg: IndexReg::Ix, disp: 5 }, Indexed { reg: IndexReg::Iy, disp: -1 }]);
    v.extend([Imm8(0x12), Imm8(0), Imm8(1), Imm8(2), Imm8(0x18), Imm16(0x1234), Mem(0x4000), Mem(0x00FE)]);
    for c in [Cond::Nz, Cond::Z, Cond::Nc, Cond::C, Cond::Po, Cond::Pe, Cond::P, Cond::M] { v.push(Cc(c)); }
    v.extend([Bit(3), Rel(-3), AfShadow, RegI, RegR, IndC]);
    v
}

fn op_text(m: Mnemonic, o: &Operand, sym: Option<&str>) -> String {
    match *o {
        Operand::Reg(r) => format!("{r:?}").to_lowercase(),
        Operand::Pair(Reg16::Sp) if m == Mnemonic::Ex => "(sp)".into(),
        Operand::Pair(p) => format!("{p:?}").to_lowercase(),
        Operand::IndHl => "(hl)".into(),
        Operand::IndBc => "(bc)".into(),
        Operand::IndDe => "(de)".into(),
        Operand::Indexed { reg, disp } => format!("({}{:+})", format!("{reg:?}").to_lowercase(), disp),
        Operand::Imm8(n) => format!("{n}"),
        Operand::Imm16(n) => sym.map(|s| s.to_string()).unwrap_or(format!("{n}")),
        Operand::Mem(a) => sym.map(|s| format!("({s})")).unwrap_or(format!("({a})")),
        Operand::Cc(c) => format!("{c:?}").to_lowercase(),
        Operand::Bit(b) => format!("{b}"),
        Operand::Rel(_) => ".tgt".into(),
        Operand::AfShadow => "af'".into(),
        Operand::RegI => "i".into(),
        Operand::RegR => "r".into(),
        Operand::IndC => "(c)".into(),
    }
}

fn assemble(src: &str, env: &SymbolTable) -> Result<Vec<u8>, Vec<String>> {
    let (file, perrs) = parse_str(src);
    let perrs: Vec<String> = perrs.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
    if !perrs.is_empty() { return Err(perrs); }
    let (m, diags) = lower_module(&file, &LowerOptions { roots: None, initial_cpu: Cpu::Z80, include_root: None, embed_base: None, defines: vec![] });
    let errs: Vec<String> = diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
    if !errs.is_empty() { return Err(errs); }
    let resolved = sigil_link::resolve_layout(&m.sections, env, true).map_err(|e| vec![format!("resolve_layout: {e:?}")])?;
    let linked = sigil_link::link(&resolved, env).map_err(|e| vec![format!("link: {e:?}")])?;
    sigil_link::flatten(&linked, 0x00).map_err(|e| vec![format!("flatten: {e}")])
}

fn frame(line: &str) -> String {
    format!("module m (cpu: z80)\nproc f() {{\n.tgt:\n    nop\n    {line}\n    ret\n}}\n")
}

fn line(m: Mnemonic, s: &str, ops: &[Operand], sym: Option<&str>) -> String {
    let o = ops.iter().map(|o| op_text(m, o, sym)).collect::<Vec<_>>().join(", ");
    format!("{s} {o}")
}

/// Assemble `l` in the frame and require exactly `nop`, `bytes`, `ret`.
fn emp_writes(l: &str, bytes: &[u8], env: &SymbolTable) -> Result<(), String> {
    let want: Vec<u8> = [&[0x00u8][..], bytes, &[0xC9][..]].concat();
    match assemble(&frame(l), env) {
        Ok(b) if b == want => Ok(()),
        Ok(b) => Err(format!("WRONG BYTES {b:02X?} want {want:02X?}")),
        Err(e) => Err(e.join(" | ")),
    }
}

type Demand = BTreeMap<Vec<u8>, (Mnemonic, &'static str, Vec<Operand>)>;

fn demand() -> (Demand, Vec<String>) {
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
    let mut demand: Demand = BTreeMap::new();
    let mut reached: Vec<Mnemonic> = Vec::new();
    for (m, s) in all_mnemonics() {
        for ops in &shapes {
            if let Ok(b) = encode(&Instruction { mnemonic: m, ops: ops.clone() }) {
                reached.push(m);
                demand.entry(b).or_insert((m, s, ops.clone()));
            }
        }
    }
    let unreached = all_mnemonics().into_iter().filter(|(m, _)| !reached.contains(m)).map(|(m, _)| format!("{m:?}")).collect();
    (demand, unreached)
}

/// The floor under the demand, measured at sigil `55c6ec32`.
const MIN_DISTINCT: usize = 538;

/// 16-bit-field forms that still refuse a link-time symbol, as
/// `("<mnemonic> <mem|imm16>", count)`. A row whose count is no longer true fails.
const SYM_REFUSED: &[(&str, usize)] = &[
    // `in a, (n)` / `out (n), a`: the port is ONE byte, so there is no 16-bit
    // field to carry a symbol. Booked: EMP-Z80-SYMBOLIC-BYTE-OPERAND.
    ("in mem", 1),
    ("out mem", 1),
];

#[test]
fn the_walk_demands_every_z80_mnemonic() {
    let (demand, unreached) = demand();
    // `ld i, a` / `ld r, a` encode through `Mnemonic::Ld` (`ED 47`, `ED 4F`, both
    // in the demand); the two dedicated variants are arms `encode` never accepts.
    // Booked: ISA-Z80-DEAD-MNEMONICS. Any other name here is a hole in the pool.
    assert_eq!(unreached, ["LdIA", "LdRA"], "Z80 mnemonics with no encodable form in the pool");
    assert!(demand.len() >= MIN_DISTINCT, "{} distinct encodings, floor {MIN_DISTINCT}", demand.len());
}

#[test]
fn emp_assembles_every_z80_instruction_the_shared_encoder_accepts() {
    let (demand, _) = demand();
    let empty = SymbolTable::new();
    let refused: Vec<String> = demand
        .iter()
        .filter_map(|(bytes, (m, s, ops))| {
            let l = line(*m, s, ops, None);
            emp_writes(&l, bytes, &empty).err().map(|e| format!("`{l}`: {e}"))
        })
        .collect();
    assert!(refused.is_empty(), "{} Z80 instruction(s) `.emp` refuses:\n{refused:#?}", refused.len());
}

#[test]
fn every_16_bit_z80_field_takes_a_link_time_symbol() {
    let (demand, _) = demand();
    let mut tried = 0usize;
    let mut refused: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (bytes, (m, s, ops)) in &demand {
        let field = ops.iter().find_map(|o| match o {
            Operand::Mem(a) => Some(("mem", *a)),
            Operand::Imm16(n) => Some(("imm16", *n)),
            _ => None,
        });
        let Some((kind, v)) = field else { continue };
        tried += 1;
        let mut env = SymbolTable::new();
        env.define("ZSym", sigil_ir::SymbolValue::Int(i64::from(v)));
        let l = line(*m, s, ops, Some("ZSym"));
        if let Err(e) = emp_writes(&l, bytes, &env) {
            refused.entry(format!("{s} {kind}")).or_default().push(format!("`{l}`: {e}"));
        }
    }
    assert!(tried >= 54, "only {tried} forms carry a 16-bit field, floor 54");
    let got: BTreeMap<&str, usize> = refused.iter().map(|(k, rows)| (k.as_str(), rows.len())).collect();
    let want: BTreeMap<&str, usize> = SYM_REFUSED.iter().copied().collect();
    assert_eq!(got, want, "symbol refusals differ from SYM_REFUSED:\n{refused:#?}");
}

#[test]
fn the_walk_can_see_a_refusal() {
    let empty = SymbolTable::new();
    assert_eq!(emp_writes("nop", &[0x00], &empty), Ok(()));
    let wrong = emp_writes("nop", &[0x76], &empty).unwrap_err();
    assert!(wrong.starts_with("WRONG BYTES"), "{wrong}");
    let unknown = emp_writes("frobnicate a", &[0x00], &empty).unwrap_err();
    assert!(unknown.contains("frobnicate"), "{unknown}");
    // An undefined link-time symbol is a refusal, not a zero.
    assert!(emp_writes("ld hl, ZSym", &[0x21, 0x00, 0x00], &empty).is_err());
}
