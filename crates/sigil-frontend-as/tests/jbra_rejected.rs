//! `jbra`/`jbsr` are emp-ONLY mnemonic-position words (D2.18): they must NEVER
//! enter sigil-isa's shared mnemonic table, or the AS front-end (which faithful
//! ports drive) would start accepting them. This pins that the AS path keeps
//! rejecting them as unrecognized 68000 mnemonics — the invariant the emp-side
//! recognition (in `lower_code_buf`, before the isa lookup) depends on.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::backend::Cpu;

fn m68k_opts() -> Options {
    Options { initial_cpu: Cpu::M68000, ..Options::default() }
}

#[test]
fn as_frontend_rejects_jbra() {
    // A faithful AS port must not silently accept the emp-only `jbra`.
    let err = assemble("\tjbra Target\nTarget:\n\trts\n", &m68k_opts())
        .expect_err("AS must reject jbra");
    assert!(
        err.iter().any(|d| d.message.contains("jbra")
            && d.message.contains("not a recognized 68000 mnemonic")),
        "expected an unrecognized-mnemonic error naming jbra, got: {err:?}"
    );
}

#[test]
fn as_frontend_rejects_jbsr() {
    let err = assemble("\tjbsr Target\nTarget:\n\trts\n", &m68k_opts())
        .expect_err("AS must reject jbsr");
    assert!(
        err.iter().any(|d| d.message.contains("jbsr")
            && d.message.contains("not a recognized 68000 mnemonic")),
        "expected an unrecognized-mnemonic error naming jbsr, got: {err:?}"
    );
}
