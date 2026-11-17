//! A6 fix round 2 (re-review N1): a register word never becomes a symbol
//! through a comptime splice.
//!
//! Fix round 1 refused a bare `usp` operand, but a `Label` argument or a
//! string spliced into a template still reached `classify_operand_splice`,
//! which mapped it straight to a symbol. With an outside `usp = $1234` in the
//! link environment, both of these assembled `move.l a2, usp` as `21CA 1234`
//! (`move.l a2, ($1234).w`) where asl assembles `4E62`:
//!
//! - `comptime fn put(p: Label) -> Code { return asm { move.l a2, {p} } }` then
//!   `put usp`;
//! - `const N = "usp"` then `{N}` in a template.
//!
//! `sr` and `ccr` took the same path. Now a `Label` argument naming a register
//! word is refused at the call, and a spliced label or string naming one is
//! refused at the splice: `usp` with its unspliced `[operand.usp-unsupported]`,
//! the others with `[asm.splice-register-word]`. Refusal only.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{SymbolTable, SymbolValue};
use sigil_span::Level;

const USP: &str = "[operand.usp-unsupported] `usp` is the 68000 user stack pointer, never a symbol";
const SPLICE: &str = "[asm.splice-register-word] `";
const LABEL_ARG: &str = "is a register word, never a label";

/// The words the re-review named. Each is a register wherever it is written
/// bare, so none may reach an operand as a symbol.
const WORDS: [&str; 3] = ["usp", "sr", "ccr"];

/// Every word as an outside symbol at `$1234`: the environment in which the
/// defect assembled instead of failing to link.
fn env() -> SymbolTable {
    let mut env = SymbolTable::new();
    for w in WORDS.iter().chain(["d0", "hl"].iter()) {
        env.define(w, SymbolValue::Int(0x1234));
    }
    env
}

/// Lower and link: the bytes, or every ERROR (message, primary-span offset).
fn emp(src: &str) -> Result<Vec<u8>, Vec<(String, usize)>> {
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
    let env = env();
    let resolved = sigil_link::resolve_layout(&module.sections, &env, true)
        .map_err(|e| vec![(format!("resolve_layout: {e:?}"), 0)])?;
    let linked = sigil_link::link(&resolved, &env).map_err(|e| vec![(format!("link: {e:?}"), 0)])?;
    Ok(linked.sections.iter().flat_map(|s| s.bytes.clone()).collect())
}

/// The refusal at the first error whose message contains `needle`, checked to
/// sit where `at` begins in `src`.
fn refused_at(src: &str, needle: &str, at: &str) {
    let errs = match emp(src) {
        Ok(b) => panic!("assembled to {b:02X?}; expected a refusal:\n{src}"),
        Err(e) => e,
    };
    let Some((_, off)) = errs.iter().find(|(m, _)| m.contains(needle)) else {
        panic!("no error containing {needle:?}: {errs:?}\n{src}");
    };
    assert!(src[*off..].starts_with(at), "error at {off} reads {:?}, expected {at:?}\n{src}", &src[*off..]);
}

fn label_template(word: &str) -> String {
    format!(
        "module m\ncomptime fn put(p: Label) -> Code {{\n    return asm {{ move.l a2, {{p}} }}\n}}\n\
         proc f() {{\n    put {word}\n    rts\n}}\n"
    )
}

/// The reviewer's first reproduction, for each word: `put usp` assembled
/// `21CA 1234`. Refused at the argument, like `put d0` always was.
#[test]
fn a_label_argument_naming_a_register_word_is_refused_at_the_call() {
    for w in WORDS {
        let src = label_template(w);
        refused_at(&src, LABEL_ARG, w);
    }
}

/// The reviewer's second reproduction, for each word: a `const` string spliced
/// into the operand. Refused at the splice.
#[test]
fn a_string_const_naming_a_register_word_is_refused_at_the_splice() {
    for w in WORDS {
        let src = format!(
            "module m\nconst N = \"{w}\"\ncomptime fn put() -> Code {{\n    return asm {{ move.l a2, {{N}} }}\n}}\n\
             proc f() {{\n    put()\n    rts\n}}\n"
        );
        let needle = if w == "usp" { USP.to_string() } else { format!("{SPLICE}{w}`") };
        refused_at(&src, &needle, "N}");
    }
}

/// The same through a `string` parameter and a literal splice.
#[test]
fn a_string_parameter_or_literal_naming_a_register_word_is_refused() {
    for w in WORDS.iter().chain(["d0", "sp"].iter()) {
        let needle = if *w == "usp" { USP.to_string() } else { format!("{SPLICE}{w}`") };
        let param = format!(
            "module m\ncomptime fn put(t: string) -> Code {{\n    return asm {{ move.l a2, {{t}} }}\n}}\n\
             proc f() {{\n    put(\"{w}\")\n    rts\n}}\n"
        );
        refused_at(&param, &needle, "t}");
        let literal = format!(
            "module m\ncomptime fn put() -> Code {{\n    return asm {{ move.l a2, {{\"{w}\"}} }}\n}}\n\
             proc f() {{\n    put()\n    rts\n}}\n"
        );
        refused_at(&literal, &needle, &format!("\"{w}\"}}"));
    }
}

/// A Z80 section reads its own register words: a spliced `"hl"` is the pair,
/// never a symbol named `hl`. A 68k word there is an ordinary name.
#[test]
fn a_z80_splice_refuses_z80_register_words_only() {
    let z = |s: &str| {
        format!(
            "module m\nsection z (cpu: z80, vma: $0000) {{\n    comptime fn put() -> Code {{\n        \
             return asm {{ ld hl, {{\"{s}\"}} }}\n    }}\n    proc f() {{\n        put()\n        ret\n    }}\n}}\n"
        )
    };
    refused_at(&z("hl"), &format!("{SPLICE}hl`"), "\"hl\"}");
    assert!(emp(&z("d0")).is_ok(), "{:?}", emp(&z("d0")));
}

/// Byte-neutral for every ordinary name: the splices still link as symbols.
#[test]
fn ordinary_labels_and_strings_still_splice_as_symbols() {
    let src = "module m\nproc usp_save() {\n    rts\n}\ncomptime fn put(p: Label) -> Code {\n    return asm { jsr {p} }\n}\n\
               comptime fn str(t: string) -> Code {\n    return asm { jsr {t} }\n}\n\
               proc f() {\n    put usp_save\n    str(\"usp_save\")\n    rts\n}\n";
    let b = emp(src).expect("ordinary splices assemble");
    // rts; jsr usp_save (abs.w 0) twice; rts.
    assert_eq!(b, [0x4E, 0x75, 0x4E, 0xB8, 0x00, 0x00, 0x4E, 0xB8, 0x00, 0x00, 0x4E, 0x75]);
}
