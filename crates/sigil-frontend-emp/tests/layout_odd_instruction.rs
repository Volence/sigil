//! `[layout.odd-instruction]` (A10 W11): an instruction at an odd address inside a
//! 68000 code stream is refused. Before A10 `dc.b 1` followed by `nop` built with
//! no diagnostic, and the `nop` sat at an odd address: a guaranteed address-error
//! crash. Every 68000 instruction is even-sized at every relaxation, so the check
//! is static: the parity of the inline `dc` data before the instruction.
//!
//! Limits (Review Focus 1 and 3): the Z80 has no alignment rule; an even run, two
//! odd runs summing to even, and odd data after the last instruction are legal.
//! `@as_compat` does not exempt it.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level};

const ID: &str = "[layout.odd-instruction]";

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

#[test]
fn an_instruction_after_an_odd_data_run_is_refused() {
    let src = "module m\nproc p () clobbers() {\n    dc.b 1\n    nop\n    rts\n}\n";
    let d = lower(src);
    let f: Vec<_> = d.iter().filter(|d| d.message.contains(ID)).collect();
    assert_eq!(f.len(), 1, "one firing per body: {d:?}");
    assert_eq!(f[0].level, Level::Error);
    assert!(f[0].message.contains("1 byte(s) of inline `dc` data"), "{}", f[0].message);
    assert_eq!(
        f[0].primary.start as usize,
        src.find("nop").unwrap(),
        "anchored at the first odd instruction"
    );
}

#[test]
fn as_compat_does_not_exempt_it() {
    let d = lower("module m\n@as_compat\nproc p () {\n    dc.b 1\n    nop\n    rts\n}\n");
    assert_eq!(d.iter().filter(|d| d.message.contains(ID)).count(), 1, "{d:?}");
}

/// Review Focus 3: even runs, odd runs that sum to even across a label, and odd data
/// after the last instruction are all legal.
#[test]
fn even_and_trailing_data_stay_legal() {
    for src in [
        "module m\nproc p () clobbers() {\n    dc.b 1, 2\n    nop\n    rts\n}\n",
        "module m\nproc p () clobbers() {\n    dc.b 1\n.x:\n    dc.b 2\n    nop\n    rts\n}\n",
        "module m\nproc p () clobbers() {\n    rts\n    dc.b \"abc\"\n}\n",
        "module m\nproc p () clobbers() {\n    dc.w 1\n    dc.l 2\n    nop\n    rts\n}\n",
    ] {
        let d = lower(src);
        assert!(!d.iter().any(|d| d.message.contains(ID)), "{src}\n{d:?}");
    }
}

/// Review Focus 1: the Z80 has no alignment rule.
#[test]
fn a_z80_body_is_untouched() {
    let d = lower_z80("module m\nproc p () clobbers(a) {\n    dc.b 1\n    nop\n    ld a, 5\n    ret\n}\n");
    assert!(!d.iter().any(|d| d.message.contains(ID)), "{d:?}");
}
