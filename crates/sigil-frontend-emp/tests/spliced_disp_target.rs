//! Final review I2: a spliced `Label` in a pc-relative or sized-absolute
//! operand is the label's VALUE, never the parameter's name.
//!
//! `{x}` splices the value of `x` everywhere else (`jsr {x}`, `#{x}`). In five
//! forms the mapper read the spliced path as a symbol NAME instead, so with a
//! `t: Label = Target` parameter and a proc that happens to be named `t`, each
//! form silently aimed at `t`:
//!
//! - `({t}).w` and `({t}).l` (the sized-absolute mapper's bare-path shortcut);
//! - `{t}(pc)`, `{t}(pc,Xn)` and `jmp {t}(pc)` (the pc-relative mapper).
//!
//! The reviewer measured `lea {t}(pc),a0` = `41FA 0008` (at `t`) and
//! `move.l ({t}).l,d0` = `2039 0000 0008`. Each form must now assemble exactly
//! as its UNSPLICED spelling with `Target` written in place of `{t}`; that
//! module is the oracle the expected bytes are derived from. A spliced name
//! that is a register word is still refused (A6 fix round 2), and a value with
//! no symbol meaning is refused loudly.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

/// Lower and link: the bytes, or every ERROR message.
fn emp(src: &str) -> Result<Vec<u8>, Vec<String>> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { roots: None, initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    let errs: Vec<String> =
        diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
    if !errs.is_empty() {
        return Err(errs);
    }
    let env = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&module.sections, &env, true)
        .map_err(|e| vec![format!("resolve_layout: {e:?}")])?;
    let linked = sigil_link::link(&resolved, &env)
        .map_err(|e| e.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    Ok(linked.sections.iter().flat_map(|s| s.bytes.clone()).collect())
}

/// The five forms, each with `{t}` where the operand names its target.
/// The last three are the WRAPPED displacement spellings (final re-review N1):
/// `({t})(pc)` reached the pc-relative mapper with `disp_spliced: false`.
const FORMS: [&str; 8] = [
    "move.l  ({t}).w, d0",
    "move.l  ({t}).l, d0",
    "lea     {t}(pc), a0",
    "move.w  {t}(pc,d0.w), d1",
    "jmp     {t}(pc)",
    "lea     ({t})(pc), a0",
    "move.w  ({t})(pc,d0.w), d1",
    "jmp     ({t})(pc)",
];

/// A module whose `main` runs `line` once through a comptime template. The
/// proc named `t` (the decoy the old mapping linked) sits first, `Target`
/// second, so the two never share an address. `spliced` passes `Target` to a
/// `t: Label` parameter and splices `{t}`; otherwise the template spells
/// `Target` itself (the unspliced oracle).
fn module(line: &str, spliced: bool) -> String {
    let (params, arg, body) = if spliced {
        ("t: Label", " Target", line.to_string())
    } else {
        ("", "()", line.replace("{t}", "Target"))
    };
    format!(
        "module m\n\
         proc t() {{\n    rts\n}}\n\
         proc Target() {{\n    rts\n}}\n\
         comptime fn form({params}) -> Code {{\n    return asm {{ {body} }}\n}}\n\
         proc main() {{\n    form{arg}\n    rts\n}}\n"
    )
}

#[test]
fn each_spliced_form_assembles_as_its_unspliced_spelling() {
    for line in FORMS {
        let want = emp(&module(line, false))
            .unwrap_or_else(|e| panic!("unspliced `{line}` refused: {e:?}"));
        let got = emp(&module(line, true))
            .unwrap_or_else(|e| panic!("spliced `{line}` refused: {e:?}"));
        assert_eq!(got, want, "`{line}`: spliced bytes differ from the unspliced spelling");
    }
}

/// The oracle itself, pinned for two forms so a change that moved BOTH
/// spellings together cannot pass the comparison above unnoticed. `t` at 0,
/// `Target` at 2, `main` at 4.
#[test]
fn the_unspliced_oracle_targets_target() {
    // move.l ($0002).l, d0; rts
    let b = emp(&module(FORMS[1], false)).unwrap();
    assert_eq!(&b[4..], [0x20, 0x39, 0x00, 0x00, 0x00, 0x02, 0x4E, 0x75]);
    // lea (Target,pc), a0: ext word at 6, disp = 2 - 6 = -4.
    let b = emp(&module(FORMS[2], false)).unwrap();
    assert_eq!(&b[4..], [0x41, 0xFA, 0xFF, 0xFC, 0x4E, 0x75]);
    // The wrapped spelling `(Target)(pc)` is the same instruction.
    let b = emp(&module(FORMS[5], false)).unwrap();
    assert_eq!(&b[4..], [0x41, 0xFA, 0xFF, 0xFC, 0x4E, 0x75]);
}

/// A string naming a register word is refused in every form (all eight), as `jsr {p}`
/// refuses it (A6 fix round 2): never a symbol named `sr`.
#[test]
fn a_register_word_is_still_refused_in_every_form() {
    for line in FORMS {
        let src = format!(
            "module m\nconst N = \"sr\"\n\
             comptime fn form() -> Code {{\n    return asm {{ {} }}\n}}\n\
             proc main() {{\n    form()\n    rts\n}}\n",
            line.replace("{t}", "{N}")
        );
        let errs = emp(&src).expect_err(line);
        assert!(
            errs.iter().any(|m| m.contains("[asm.splice-register-word] `sr`")),
            "`{line}`: {errs:?}"
        );
    }
}

/// An integer has no pc-relative spelling (`4(pc)` is refused unspliced too),
/// so a spliced one is refused loudly; in a sized absolute it is the address,
/// as `($1234).w` is.
#[test]
fn a_spliced_integer_is_refused_pc_relative_and_an_address_absolute() {
    let pc = "module m\ncomptime fn form(n: u16) -> Code {\n    return asm { lea {n}(pc), a0 }\n}\n\
              proc main() {\n    form(4)\n    rts\n}\n";
    let errs = emp(pc).expect_err("a spliced integer pc-relative target");
    assert!(errs.iter().any(|m| m.contains("[asm.splice-kind]")), "{errs:?}");
    let abs = |body: &str| {
        format!(
            "module m\ncomptime fn form(n: u16) -> Code {{\n    return asm {{ {body} }}\n}}\n\
             proc main() {{\n    form($1234)\n    rts\n}}\n"
        )
    };
    let spliced = emp(&abs("move.l ({n}).w, d0")).unwrap();
    let plain = emp(&abs("move.l ($1234).w, d0")).unwrap();
    assert_eq!(spliced, plain);
    assert_eq!(spliced, [0x20, 0x38, 0x12, 0x34, 0x4E, 0x75]);
}
