//! Ladder item 8 (§5) — the latent Z80 `jr → jp` relaxation ladder. A symbolic
//! unconditional `jr` lowers to a two-rung `RelaxLadder` (`jr e` 2 B →
//! `jp nn` 3 B); the linker selects the smallest reaching rung exactly like the
//! 68k `bra.s → bra.w` ladder. Byte-neutrality is the point: an in-reach target
//! stays 2 bytes (asl's choice); a genuinely out-of-reach target relaxes to
//! `jp` (rather than the old hard error) — but the current `.emp` corpus has no
//! symbolic `jr`, so the ladder is latent capacity, proven here synthetically.
//! `djnz` has no long form and never ladders — an out-of-range `djnz` stays a
//! hard error, matching asl.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::Level;

fn lower(src: &str) -> Module {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(diags.iter().all(|d| d.level != Level::Error), "lower: {diags:?}");
    module
}

/// The linked bytes of a named section, or the link/layout error string.
fn section_bytes(module: &Module, name: &str) -> Result<Vec<u8>, String> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .map_err(|ds| format!("{ds:?}"))?;
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).map_err(|ds| format!("{ds:?}"))?;
    Ok(linked.section(name).expect("linked section").bytes.clone())
}

/// An in-reach symbolic `jr` selects the 2-byte `jr e` rung — byte-identical to
/// the old hard `Z80JrRel8` lowering. `P` is `jr Q` (2 B) + `ret` (1 B); `Q` at
/// offset 3 sits +1 from the post-instruction PC, so `jr +1` = `18 01`.
#[test]
fn in_reach_jr_stays_two_bytes() {
    let src = "module m\n\
               section s (cpu: z80, vma: $0) {\n\
                 proc P() { jr Q\n ret }\n\
                 proc Q() { ret }\n\
               }\n";
    let module = lower(src);
    assert_eq!(section_bytes(&module, "s").unwrap(), vec![0x18, 0x01, 0xC9, 0xC9]);
}

/// A genuinely out-of-reach symbolic `jr` relaxes to the 3-byte `jp nn` rung
/// (little-endian absolute) — proving the ladder wires. `Far` sits at VMA $0200,
/// far past the ±128 `jr` window, so `P` becomes `jp $0200` = `C3 00 02` + `ret`.
#[test]
fn out_of_reach_jr_relaxes_to_jp() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() { jr Far\n ret }\n\
               }\n\
               section tgt (cpu: z80, vma: $0200) {\n\
                 proc Far() { ret }\n\
               }\n";
    let module = lower(src);
    assert_eq!(section_bytes(&module, "code").unwrap(), vec![0xC3, 0x00, 0x02, 0xC9]);
}

/// `djnz` is short-only — never a ladder. An out-of-range `djnz` stays a hard
/// error (a real code-structure problem, matching asl), NOT a silent relax to a
/// non-existent long form.
#[test]
fn out_of_range_djnz_is_a_hard_error() {
    let src = "module m\n\
               section code (cpu: z80, vma: $0) {\n\
                 proc P() { djnz Far\n ret }\n\
               }\n\
               section tgt (cpu: z80, vma: $0200) {\n\
                 proc Far() { ret }\n\
               }\n";
    let module = lower(src);
    let err = section_bytes(&module, "code").expect_err("djnz out of range must error");
    assert!(
        err.to_lowercase().contains("range") || err.contains("Z80JrRel8") || err.contains("out-of-reach"),
        "expected a djnz out-of-range error, got: {err}"
    );
}
