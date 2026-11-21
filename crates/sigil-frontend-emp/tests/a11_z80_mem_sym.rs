//! A11: a Z80 `(nn)` operand takes a link-time symbol in every `ld` form that
//! has a 16-bit address field, and nowhere else.
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

fn assemble(src: &str, env: &SymbolTable) -> Result<Vec<u8>, Vec<String>> {
    let (file, perrs) = parse_str(src);
    let perrs: Vec<String> = perrs.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
    if !perrs.is_empty() {
        return Err(perrs);
    }
    let opts = LowerOptions { roots: None, initial_cpu: Cpu::Z80, include_root: None, embed_base: None, defines: vec![] };
    let (m, diags) = lower_module(&file, &opts);
    let errs: Vec<String> = diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
    if !errs.is_empty() {
        return Err(errs);
    }
    let resolved = sigil_link::resolve_layout(&m.sections, env, true).map_err(|e| vec![format!("resolve_layout: {e:?}")])?;
    let linked = sigil_link::link(&resolved, env).map_err(|e| vec![format!("link: {e:?}")])?;
    sigil_link::flatten(&linked, 0x00).map_err(|e| vec![format!("flatten: {e}")])
}

/// `line` then `ret` in `f`, followed by a one-byte proc `q`.
fn with_q(line: &str) -> Result<Vec<u8>, Vec<String>> {
    let src = format!("module m (cpu: z80)\nproc f() {{\n    {line}\n    ret\n}}\nproc q() {{\n    ret\n}}\n");
    assemble(&src, &SymbolTable::new())
}

#[test]
fn a_forward_label_is_a_z80_memory_operand() {
    // 3-byte forms: `q` is at 4. Prefixed forms: `q` is at 5.
    assert_eq!(with_q("ld a, (q)"), Ok(vec![0x3A, 0x04, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld (q), a"), Ok(vec![0x32, 0x04, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld hl, (q)"), Ok(vec![0x2A, 0x04, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld (q), hl"), Ok(vec![0x22, 0x04, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld (q), bc"), Ok(vec![0xED, 0x43, 0x05, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld sp, (q)"), Ok(vec![0xED, 0x7B, 0x05, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld ix, (q)"), Ok(vec![0xDD, 0x2A, 0x05, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld (q), iy"), Ok(vec![0xFD, 0x22, 0x05, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld (q+1), a"), Ok(vec![0x32, 0x05, 0x00, 0xC9, 0xC9]));
}

#[test]
fn a_literal_address_is_unchanged() {
    assert_eq!(with_q("ld a, ($1234)"), Ok(vec![0x3A, 0x34, 0x12, 0xC9, 0xC9]));
}

#[test]
fn a_symbolic_port_is_refused_not_truncated() {
    for line in ["in a, (q)", "out (q), a"] {
        let e = with_q(line).unwrap_err();
        assert!(e[0].starts_with("[lower.z80-unsupported] "), "`{line}`: {e:?}");
    }
}

#[test]
fn an_undefined_symbol_is_a_link_error_not_a_zero() {
    let src = "module m (cpu: z80)\nproc f() {\n    ld a, (extern(\"Foo\"))\n    ret\n}\n";
    let e = assemble(src, &SymbolTable::new()).unwrap_err();
    assert!(e[0].contains("Foo"), "{e:?}");
    let mut env = SymbolTable::new();
    env.define("Foo", sigil_ir::SymbolValue::Int(0x4000));
    assert_eq!(assemble(src, &env), Ok(vec![0x3A, 0x00, 0x40, 0xC9]));
}

// ---- added beyond the brief's file (review lessons of this parcel) ----------

use sigil_backend_z80::z80::{encode, Instruction, Mnemonic, Operand, Reg16, Reg8};

/// The link environment of the three-sided vectors: `ZSym` is `$1F34`, two
/// DISTINCT non-zero bytes, so a high/low swap (`1F 34`) or a dropped byte reads
/// as a different address, never as the right one.
fn zsym_env(v: i64) -> SymbolTable {
    let mut env = SymbolTable::new();
    env.define("ZSym", sigil_ir::SymbolValue::Int(v));
    env
}

/// `line` then `ret`, in `f`. `ZSym` is not defined in the file: under the
/// operand's link context the bareword is a link-time label resolved from `env`.
fn emp_with(line: &str, env: &SymbolTable) -> Result<Vec<u8>, Vec<String>> {
    assemble(&format!("module m (cpu: z80)\nproc f() {{\n    {line}\n    ret\n}}\n"), env)
}

/// The AS route (`sigil-frontend-as`, which copies asl) on the same line. The AS
/// front end refuses a symbol it cannot resolve in the file (`unresolved symbol
/// \`ZSym\` in operand`), so `ZSym` is an `equ` there, of the environment's
/// value, written in decimal (`$` is the program counter in AS source).
fn as_with(line: &str, env: &SymbolTable) -> Result<Vec<u8>, Vec<String>> {
    let v = env.resolve("ZSym", None).expect("ZSym is an integer in the environment");
    let src = format!("ZSym    equ {v}\n        cpu z80\n        phase 0\n        {line}\n        ret\n");
    let module = sigil_frontend_as::assemble(&src, &sigil_frontend_as::Options::default())
        .map_err(|ds| ds.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    let linked = sigil_link::link(&module.sections, env).map_err(|e| vec![format!("link: {e:?}")])?;
    sigil_link::flatten(&linked, 0x00).map_err(|e| vec![format!("flatten: {e}")])
}

/// Three sides: the hand-derived literal `hex`, the shared encoder on `inst`, and
/// both front ends on `line` (each followed by `ret`, `C9`).
#[track_caller]
fn golden(line: &str, inst: Instruction, hex: &[u8]) {
    let env = zsym_env(0x1F34);
    assert_eq!(encode(&inst).as_deref(), Ok(hex), "shared encoder, `{line}`");
    let framed = [hex, &[0xC9][..]].concat();
    assert_eq!(emp_with(line, &env), Ok(framed.clone()), "`.emp`, `{line}`");
    assert_eq!(as_with(line, &env), Ok(framed), "AS route, `{line}`");
}

fn i(mnemonic: Mnemonic, ops: &[Operand]) -> Instruction {
    Instruction { mnemonic, ops: ops.to_vec() }
}

/// Derivation (Zilog UM0080, "8-Bit Load Group" / "16-Bit Load Group"): the
/// address follows the opcode LOW byte first, so `$1F34` is `34 1F`.
/// `ld a,(nn)` 3A; `ld (nn),a` 32; `ld hl,(nn)` 2A; `ld (nn),hl` 22;
/// `ld dd,(nn)` ED 01dd1011 and `ld (nn),dd` ED 01dd0011 with dd = bc 00, de 01,
/// hl 10, sp 11 (so de-read 5B, sp-write 73); `ld ix/iy,(nn)` DD/FD 2A and
/// `ld (nn),ix/iy` DD/FD 22.
#[test]
fn three_sides_agree_on_a_link_time_address_with_two_distinct_bytes() {
    use Operand::*;
    let m = Mem(0x1F34);
    golden("ld a, (ZSym)", i(Mnemonic::Ld, &[Reg(Reg8::A), m]), &[0x3A, 0x34, 0x1F]);
    golden("ld (ZSym), a", i(Mnemonic::Ld, &[m, Reg(Reg8::A)]), &[0x32, 0x34, 0x1F]);
    golden("ld hl, (ZSym)", i(Mnemonic::Ld, &[Pair(Reg16::Hl), m]), &[0x2A, 0x34, 0x1F]);
    golden("ld (ZSym), hl", i(Mnemonic::Ld, &[m, Pair(Reg16::Hl)]), &[0x22, 0x34, 0x1F]);
    golden("ld de, (ZSym)", i(Mnemonic::Ld, &[Pair(Reg16::De), m]), &[0xED, 0x5B, 0x34, 0x1F]);
    golden("ld (ZSym), sp", i(Mnemonic::Ld, &[m, Pair(Reg16::Sp)]), &[0xED, 0x73, 0x34, 0x1F]);
    golden("ld iy, (ZSym)", i(Mnemonic::Ld, &[Pair(Reg16::Iy), m]), &[0xFD, 0x2A, 0x34, 0x1F]);
    golden("ld (ZSym), ix", i(Mnemonic::Ld, &[m, Pair(Reg16::Ix)]), &[0xDD, 0x22, 0x34, 0x1F]);
    // The displacement rides the link-time expression: `$1F34 + 1` is `$1F35`.
    golden("ld a, (ZSym+1)", i(Mnemonic::Ld, &[Reg(Reg8::A), Mem(0x1F35)]), &[0x3A, 0x35, 0x1F]);
}

#[test]
fn a_label_in_another_section_is_a_memory_operand() {
    let src = "module m\nsection z (cpu: z80, vma: $0000) {\n    proc f() {\n        ld a, (D)\n        ld (D), hl\n        ret\n    }\n}\nsection w (cpu: z80, vma: $1F34) {\n    data D: [u8; 2] = [1, 2]\n}\n";
    let (file, _) = parse_str(src);
    let opts = LowerOptions { roots: None, initial_cpu: Cpu::Z80, include_root: None, embed_base: None, defines: vec![] };
    let (m, diags) = lower_module(&file, &opts);
    let errs: Vec<_> = diags.iter().filter(|d| d.level == Level::Error).collect();
    assert!(errs.is_empty(), "{errs:?}");
    let env = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&m.sections, &env, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &env).expect("link");
    assert_eq!(linked.section("z").expect("section z").bytes, vec![0x3A, 0x34, 0x1F, 0x22, 0x34, 0x1F, 0xC9]);
}

/// The fixup is `Value16Le`: the linker refuses an address outside `0..=$FFFF`
/// instead of writing its low 16 bits. `$FFFF` is the last address that fits.
#[test]
fn an_address_outside_16_bits_is_a_link_error_not_a_truncation() {
    assert_eq!(emp_with("ld a, (ZSym)", &zsym_env(0xFFFF)), Ok(vec![0x3A, 0xFF, 0xFF, 0xC9]));
    for (line, v) in [("ld a, (ZSym)", 0x1_0000), ("ld (ZSym), hl", 0x1_1F34), ("ld a, (ZSym-1)", 0)] {
        let e = emp_with(line, &zsym_env(v)).unwrap_err();
        assert!(e[0].starts_with("link: "), "`{line}` with ZSym = {v:#x}: {e:?}");
    }
}

/// Every neighbour of the new path that is not an `ld` with a 16-bit address
/// field stays a NAMED refusal: never a panic, never bytes.
#[test]
fn the_neighbouring_spellings_stay_named_refusals() {
    for line in ["jp (q)", "ld b, (q)", "ld (q), 5", "ld (q), (q)", "ld i, (q)", "ld a, (bankid(q))"] {
        let e = with_q(line).unwrap_err();
        assert!(e[0].starts_with("[lower.z80-unsupported] "), "`{line}`: {e:?}");
    }
    // The 8-bit port: refused by the symbolic-operand gate, which names the forms
    // it supports.
    for line in ["in a, (q)", "out (q), a"] {
        let e = with_q(line).unwrap_err();
        assert!(e[0].contains("a symbolic operand is only supported as"), "`{line}`: {e:?}");
        assert!(e[0].contains("`ld r, (Label)` / `ld (Label), r`"), "`{line}`: {e:?}");
    }
}

/// A comptime constant still takes the comptime `(nn)` path.
#[test]
fn a_comptime_constant_address_is_unchanged() {
    let src = "module m (cpu: z80)\nconst C = $1234\nproc f() {\n    ld a, (C)\n    ld (C), hl\n    ret\n}\n";
    assert_eq!(assemble(src, &SymbolTable::new()), Ok(vec![0x3A, 0x34, 0x12, 0x22, 0x34, 0x12, 0xC9]));
}

/// A register name in parentheses is never read as a link-time label, even when
/// a proc of that name exists (`proc b` beside `ld a, (b)` would otherwise
/// assemble `3A 04 00`, a read of the proc's address).
#[test]
fn a_register_name_is_not_a_link_time_address() {
    for reg in ["a", "b", "c", "e", "h", "l", "af", "i", "r"] {
        let src = format!("module m (cpu: z80)\nproc f() {{\n    ld a, ({reg})\n    ret\n}}\nproc {reg}() {{\n    ret\n}}\n");
        let e = assemble(&src, &SymbolTable::new()).unwrap_err();
        let want = format!("[lower.z80-unsupported] `({reg})` is not a Z80 operand form: `{reg}` is a register, not a memory address");
        assert!(e.contains(&want), "`ld a, ({reg})` beside `proc {reg}`: {e:?}");
    }
    // Without a proc of that name it is the same named refusal, not a link error.
    let e = with_q("ld a, (c)").unwrap_err();
    assert!(e[0].starts_with("[lower.z80-unsupported] `(c)`"), "{e:?}");
}
