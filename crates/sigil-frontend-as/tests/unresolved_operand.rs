//! A genuinely-undefined symbol used as an instruction operand must be a LOUD
//! error, not a silent `0x00` fold. The multi-pass loop still tolerates forward
//! references (unresolved on early passes, resolved by convergence) — only a
//! symbol still unresolved on the CONVERGED pass is reported.

use sigil_frontend_as::{assemble, Options};

#[test]
fn unresolved_operand_symbol_errors_after_convergence() {
    // `ld a, Undefined` where Undefined is never defined anywhere. Previously the
    // front end silently folded it to 0x00 (a latent miscompile the byte-diff had
    // to catch); now the converged pass reports it as unresolved.
    let src = "        cpu z80\n        phase 0\n        ld a, Undefined\n";
    let err = assemble(src, &Options::default()).expect_err("undefined operand must error");
    assert!(
        err.iter().any(|d| {
            let m = d.message.to_lowercase();
            d.message.contains("Undefined") && m.contains("unresolved")
        }),
        "expected an unresolved-symbol error naming `Undefined`, got: {err:?}"
    );
}

#[test]
fn forward_referenced_equate_operand_resolves_to_the_right_byte() {
    // 8-bit immediate referencing an equate defined LATER in the source: it is
    // unresolved (placeholder) on pass 0 and resolved by convergence — it must
    // NOT trip the unresolved-operand error, and must emit the real byte.
    let src = "        cpu z80\n        phase 0\n        ld a, Later\nLater = 5\n";
    let module = assemble(src, &Options::default()).expect("forward equate must resolve");
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    // ld a,5 = 3E 05
    assert_eq!(sigil_link::flatten(&linked, 0x00), vec![0x3E, 0x05]);
}

#[test]
fn resolved_operand_is_unaffected() {
    // A defined equate used as an operand assembles cleanly (sanity: the change
    // does not spuriously flag resolved symbols).
    let src = "        cpu z80\n        phase 0\nK = 7\n        ld a, K\n";
    assert!(assemble(src, &Options::default()).is_ok());
}
