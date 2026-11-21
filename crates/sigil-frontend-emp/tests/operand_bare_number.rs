//! `[operand.bare-number]` (A10 W1): a bare number written in a 68000 operand
//! position is refused. Before A10 `move.w $1234, d0` assembled as the immediate
//! `#$1234` with no diagnostic: the author meant either the value (`#`) or the
//! memory at that address (`(addr).w` / `(addr).l`), and the assembler guessed.
//!
//! Limits (Review Focus 1 and 2): a Z80 section is untouched (a bare number IS
//! the Z80 immediate), and a spliced integer in a template (`move.w {n}, d0`)
//! stays an immediate (booked as `EMP-SPLICED-INT-OPERAND`).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level};

const ID: &str = "[operand.bare-number]";

fn lower_on(src: &str, cpu: Cpu) -> Vec<Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(
        &file,
        &LowerOptions {
            roots: None,
            initial_cpu: cpu,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    diags
}

fn lower(src: &str) -> Vec<Diagnostic> {
    lower_on(src, Cpu::M68000)
}

fn lower_z80(src: &str) -> Vec<Diagnostic> {
    lower_on(src, Cpu::Z80)
}

fn firings(d: &[Diagnostic]) -> Vec<&Diagnostic> {
    d.iter().filter(|d| d.message.contains(ID)).collect()
}

fn errors(d: &[Diagnostic]) -> Vec<&Diagnostic> {
    d.iter().filter(|d| d.level == Level::Error).collect()
}

#[test]
fn a_bare_number_operand_is_refused() {
    let d = lower("module m\nproc p () clobbers(d0) {\n    move.w $1234, d0\n    rts\n}\n");
    let f = firings(&d);
    assert_eq!(f.len(), 1, "{d:?}");
    assert_eq!(f[0].level, Level::Error);
    assert!(f[0].message.contains("a bare number (4660)"), "{}", f[0].message);
}

#[test]
fn a_number_valued_expression_is_refused_too() {
    let d = lower("module m\nproc p () clobbers(d0) {\n    move.w sizeof(u16), d0\n    rts\n}\n");
    assert_eq!(firings(&d).len(), 1, "{d:?}");
}

/// The two spellings the message offers both assemble.
#[test]
fn the_immediate_and_the_sized_absolute_stay_legal() {
    let d = lower(
        "module m\nproc p () clobbers(d0) {\n    move.w #$1234, d0\n    move.w ($1234).w, d0\n    rts\n}\n",
    );
    assert!(errors(&d).is_empty(), "{d:?}");
}

/// Review Focus 2: a spliced integer in a template is the macro idiom and stays an immediate.
#[test]
fn a_spliced_integer_stays_an_immediate() {
    let d = lower(
        "module m\ncomptime fn put(n: int) -> Code {\n    return asm {\n        move.w  {n}, d0\n    }\n}\nproc p () clobbers(d0) {\n    put(5)\n    rts\n}\n",
    );
    assert!(errors(&d).is_empty(), "{d:?}");
}

/// Review Focus 1: on the Z80 a bare number IS the immediate.
#[test]
fn a_z80_bare_number_is_untouched() {
    let d = lower_z80("module m\nproc p () clobbers(a) {\n    ld a, 5\n    ret\n}\n");
    assert!(firings(&d).is_empty() && errors(&d).is_empty(), "{d:?}");
}
