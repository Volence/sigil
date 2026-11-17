//! A6 fix round 1 (`EMP-USP-READ-AS-SYMBOL`): `usp` is a register word.
//!
//! `.emp` had no `usp` operand, so a bare `usp` in operand position resolved
//! as a SYMBOL. Beside an item named `usp` that is a silent wrong instruction:
//! `move.l a2, usp` assembled `21CA 0000` (`move.l a2, ($0000).w`) and
//! `move.l usp, a3` `2678 0000`, where asl assembles `4E62` / `4E6B`. Now an
//! item named `usp` is refused where it is declared, and `usp` in operand
//! position never resolves to a symbol (refused until `EMP-USP-CCR-SURFACE`
//! exposes the form).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{SymbolTable, SymbolValue};
use sigil_span::Level;

const DECL: &str = "[name.reserved-register] `usp` is the 68000 user stack pointer, a register word, \
                    and cannot name an item";
const OPERAND: &str = "[operand.usp-unsupported] `usp` is the 68000 user stack pointer, never a symbol; \
                       `.emp` has no `move` to or from `usp` yet";

/// Lower and link `src` with `env` as the link environment: the bytes, or
/// every ERROR message (with the source offset of its primary span).
fn emp(src: &str, env: &SymbolTable) -> Result<Vec<u8>, Vec<(String, usize)>> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    let errs: Vec<(String, usize)> = diags
        .iter()
        .filter(|d| d.level == Level::Error)
        .map(|d| (d.message.clone(), d.primary.start as usize))
        .collect();
    if !errs.is_empty() {
        return Err(errs);
    }
    let resolved = sigil_link::resolve_layout(&module.sections, env, true)
        .map_err(|e| vec![(format!("resolve_layout: {e:?}"), 0)])?;
    let linked = sigil_link::link(&resolved, env).map_err(|e| vec![(format!("link: {e:?}"), 0)])?;
    Ok(linked.sections.iter().flat_map(|s| s.bytes.clone()).collect())
}

fn refused(src: &str, env: &SymbolTable) -> Vec<(String, usize)> {
    match emp(src, env) {
        Ok(b) => panic!("assembled to {b:02X?}; expected a refusal:\n{src}"),
        Err(e) => e,
    }
}

fn has(errs: &[(String, usize)], prefix: &str) -> bool {
    errs.iter().any(|(m, _)| m.starts_with(prefix))
}

/// The reviewer's reproduction: a `proc usp` beside `move.l a2, usp`. It
/// assembled `4E75 21CA 0000 4E75`.
#[test]
fn a_proc_named_usp_no_longer_turns_move_to_usp_into_an_absolute_move() {
    let src = "module m\nproc usp() {\n    rts\n}\nproc f() {\n    move.l a2, usp\n    rts\n}\n";
    let e = refused(src, &SymbolTable::new());
    assert!(has(&e, DECL), "{e:?}");
    assert!(has(&e, OPERAND), "{e:?}");
    // Located: the declaration error points at the `proc usp` item.
    let at = e.iter().find(|(m, _)| m.starts_with(DECL)).unwrap().1;
    assert!(src[at..].starts_with("proc usp"), "declaration error at {at}: {:?}", &src[at..]);
}

#[test]
fn move_from_usp_is_refused_beside_a_data_item_named_usp() {
    let src = "module m\ndata usp: u16 = 0\nproc f() {\n    move.l usp, a3\n    rts\n}\n";
    let e = refused(src, &SymbolTable::new());
    assert!(has(&e, DECL), "{e:?}");
    assert!(has(&e, OPERAND), "{e:?}");
}

/// A `usp` the module cannot see being declared (another unit's symbol, in the
/// link environment) never reaches the operand either.
#[test]
fn an_outside_symbol_named_usp_never_resolves_as_the_operand() {
    let mut env = SymbolTable::new();
    env.define("usp", SymbolValue::Int(0x1234));
    for inst in ["move.l a2, usp", "move.l usp, a3", "move.l usp, d0", "lea usp, a0"] {
        let src = format!("module m\nproc f() {{\n    {inst}\n    rts\n}}\n");
        let e = refused(&src, &env);
        assert!(has(&e, OPERAND), "`{inst}`: {e:?}");
    }
}

#[test]
fn a_vars_field_const_and_param_named_usp_are_refused() {
    for item in [
        "region r @ $FFFF8000 .. $FFFF8100\nvars r {\n    usp: u16,\n}",
        "const usp = 4",
        "equ usp = 4",
        "comptime fn f(usp: int) -> int {\n    usp\n}",
    ] {
        let e = refused(&format!("module m\n{item}\n"), &SymbolTable::new());
        assert!(has(&e, DECL), "`{item}`: {e:?}");
    }
}

/// Byte-neutral elsewhere: names that merely contain `usp` are ordinary.
#[test]
fn names_containing_usp_are_ordinary() {
    let src = "module m\nproc usp_save() {\n    rts\n}\nproc f() {\n    jsr usp_save\n    move.l a2, a3\n    rts\n}\n";
    assert!(emp(src, &SymbolTable::new()).is_ok());
}
