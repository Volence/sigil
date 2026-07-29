//! t27 phase-1 (a) — Z80 EXPRESSION symbolic imm16 defer.
//!
//! T1 wired only a BARE `ld hl, Label` (the symbolic imm16 path matched a
//! single `CodeOperand::Sym`). The rung-1 file `z80_init.asm` demands the
//! COMPOUND forms two of its three imm16 lines use:
//!   ld de, Z80_IdleProgram_CodeEnd+1                      (Label + const)
//!   ld bc, (Z80_RAM_END-Z80_RAM)-Z80_IdleProgram_CodeEnd-1 (const − Label − 1)
//! A symbol-bearing operand EXPRESSION that cannot fold at eval now defers to a
//! link-time `Value16Le` fixup carrying the residual Expr — the Z80 analog of
//! the working 68k `move.l #(Label+1), d0` deferral. Pure-comptime expressions
//! keep folding exactly as before (no behavior change), and forms outside the
//! wired set (a symbol in imm8 position) still error loudly (bounded scope).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::expr::BinOp;
use sigil_ir::{Expr, Fixup, FixupKind, Fragment, Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    )
}

fn fixups_of(module: &Module, name: &str) -> Vec<Fixup> {
    module
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section `{name}`"))
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(d.fixups.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn section_bytes(module: &Module, name: &str) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

fn z80_body_diags(body: &str) -> Vec<Diagnostic> {
    let src = format!(
        "module m\nsection s (cpu: z80, vma: $0) {{\n  proc P() {{\n    {body}\n    ret\n  }}\n}}\n"
    );
    lower(&src).1
}

/// FRAGMENT level: `ld de, Target+1` defers to ONE `Value16Le` fixup whose
/// target is the residual COMPOUND expression `Target + 1` (not a bare Sym).
#[test]
fn expr_ld_de_defers_value16le_with_addend() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() {\n\
                   ld de, Target+1\n\
                   ret\n\
                 }\n\
               }\n\
               section tgt (cpu: z80, vma: $1234) {\n\
                 data Target: u8 = $00\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    let fixups = fixups_of(&module, "code");
    assert_eq!(fixups.len(), 1, "one symbolic fixup, got: {fixups:?}");
    assert_eq!(fixups[0].kind, FixupKind::Value16Le);
    assert_eq!(fixups[0].offset, 1, "imm hole is the last 2 of `11 nn nn`");
    assert_eq!(
        fixups[0].target,
        Expr::Binary {
            op: BinOp::Add,
            lhs: Box::new(Expr::Sym("Target".into())),
            rhs: Box::new(Expr::Int(1)),
        },
        "the fixup target is the residual Label+1 expr"
    );
}

/// LINKED: `ld de, Target+1` with `Target` at $1234 → imm $1235, little-endian
/// `35 12`; the whole section is `11 35 12 C9`.
#[test]
fn expr_ld_de_links_little_endian() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() {\n\
                   ld de, Target+1\n\
                   ret\n\
                 }\n\
               }\n\
               section tgt (cpu: z80, vma: $1234) {\n\
                 data Target: u8 = $00\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    assert_eq!(section_bytes(&module, "code"), vec![0x11, 0x35, 0x12, 0xC9]);
}

/// LINKED: `ld bc, $2000 - Target - 1` (the z80_init clear-count shape) with
/// `Target` at $1234 → $2000-$1234-1 = $0DCB, little-endian `CB 0D`; section
/// `01 CB 0D C9`.
#[test]
fn expr_ld_bc_const_minus_label_links() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() {\n\
                   ld bc, $2000 - Target - 1\n\
                   ret\n\
                 }\n\
               }\n\
               section tgt (cpu: z80, vma: $1234) {\n\
                 data Target: u8 = $00\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    assert_eq!(section_bytes(&module, "code"), vec![0x01, 0xCB, 0x0D, 0xC9]);
}

/// `jp Target+1` also defers through the same path (the `jp`/`call` imm16 arm).
#[test]
fn expr_jp_label_plus_1_links() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() {\n\
                   jp Target+1\n\
                 }\n\
               }\n\
               section tgt (cpu: z80, vma: $ABCD) {\n\
                 data Target: u8 = $00\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    // $ABCD+1 = $ABCE → C3 CE AB.
    assert_eq!(section_bytes(&module, "code"), vec![0xC3, 0xCE, 0xAB]);
}

/// NO-BEHAVIOR-CHANGE control: a PURE-comptime arithmetic immediate still FOLDS
/// at eval to a plain `Imm16` with NO fixup — the defer path never fires for a
/// foldable expression.
#[test]
fn comptime_arith_imm16_still_folds_no_fixup() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() {\n\
                   ld bc, $1200 + $34\n\
                   ret\n\
                 }\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    assert!(fixups_of(&module, "code").is_empty(), "a foldable expr must NOT defer");
    assert_eq!(section_bytes(&module, "code"), vec![0x01, 0x34, 0x12, 0xC9]);
}

/// BOUNDED-SCOPE negative control (t24 rule): a symbol-bearing expression in
/// IMM8 position (`ld a, Target+1`) is NOT wired — the model must still error
/// loudly, never silently accept un-oracled bytes.
#[test]
fn symbol_expr_in_imm8_position_still_unsupported() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() {\n\
                   ld a, Target+1\n\
                   ret\n\
                 }\n\
               }\n\
               section tgt (cpu: z80, vma: $1234) {\n\
                 data Target: u8 = $00\n\
               }\n";
    let diags = {
        let (_m, d) = lower(src);
        d
    };
    assert!(
        diags.iter().any(|d| d.level == Level::Error && d.message.contains("[lower.z80-unsupported]")),
        "a symbol in imm8 position must stay [lower.z80-unsupported], got: {diags:?}"
    );
}

/// The bare form T1 already wired stays IDENTICAL (regression guard): the
/// residual is a bare `Sym`, not a Binary.
#[test]
fn bare_label_unchanged_regression() {
    let _ = z80_body_diags("ld hl, X"); // smoke: parses/lowers under the new path
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() {\n\
                   ld hl, Target\n\
                   ret\n\
                 }\n\
               }\n\
               section tgt (cpu: z80, vma: $1234) {\n\
                 data Target: u8 = $00\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    let fixups = fixups_of(&module, "code");
    assert_eq!(fixups[0].target, Expr::Sym("Target".into()), "bare label stays a bare Sym");
    assert_eq!(section_bytes(&module, "code"), vec![0x21, 0x34, 0x12, 0xC9]);
}
