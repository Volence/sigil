//! `CFG-BT-BF-EDGES`: `.emp` refuses `bt` and `bf` on the 68000.
//!
//! Motorola's 68000 `Bcc` excludes condition codes `0000` (`T`, which is `bra`)
//! and `0001` (`F`, which is `bsr`, a subroutine call pushing a return address).
//! Before this refusal, `m68k_mnemonic` matched `"bt"`/`"bf"` accidentally
//! through `strip_prefix('b')` + `m68k_cond`, emitting `bra`/`bsr` opcodes while
//! every CFG/call/stack/contract/cycle analysis treated both as fall-through
//! instructions (so W2 leaks via `bt .leak` and unmodelled calls via `bf .h`
//! built clean).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Level;

#[derive(Debug, Clone, PartialEq, Eq)]
struct D {
    level: Level,
    message: String,
    line: usize,
}

fn line_of(src: &str, offset: u32) -> usize {
    src.as_bytes()[..(offset as usize).min(src.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

fn lower(src: &str) -> Vec<D> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let opts = LowerOptions {
        roots: None,
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (_m, diags) = lower_module(&file, &opts);
    diags
        .iter()
        .map(|d| D { level: d.level, message: d.message.clone(), line: line_of(src, d.primary.start) })
        .collect()
}

fn errors(src: &str) -> Vec<D> {
    lower(src).into_iter().filter(|d| d.level == Level::Error).collect()
}

#[test]
fn w2_leak_via_bt_is_refused_for_all_sizes() {
    for form in ["bt .leak", "bt.s .leak", "bt.w .leak", "bt.b .leak", "bt.l .leak"] {
        let src = format!(
            "module m\n\
             proc P () clobbers(d0, d1, a0) preserves(sr.mask) {{\n\
                 move.w sr, -(sp)\n\
                 move.w #$2700, sr\n\
                 {form}\n\
                 move.w (sp)+, sr\n\
                 rts\n\
             .leak:\n\
                 addq.l #2, sp\n\
                 rts\n\
             }}\n"
        );
        let errs = errors(&src);
        assert!(
            errs.iter().any(|d| d.line == 5
                && d.message.starts_with("[mnemonic.bt-bf] `bt`")
                && d.message.contains("use `bra`")),
            "expected `[mnemonic.bt-bf]` error guiding to `bra` on line 5 for `{form}`, got: {errs:#?}"
        );
    }
}

#[test]
fn hidden_call_via_bf_is_refused_for_all_sizes() {
    for form in ["bf .h", "bf.s .h", "bf.w .h", "bf.b .h", "bf.l .h"] {
        let src = format!(
            "module m\n\
             proc P () clobbers(d0) {{\n\
                 {form}\n\
                 rts\n\
             .h:\n\
                 addq.l #6, sp\n\
                 rts\n\
             }}\n"
        );
        let errs = errors(&src);
        assert!(
            errs.iter().any(|d| d.line == 3
                && d.message.starts_with("[mnemonic.bt-bf] `bf`")
                && d.message.contains("use `bsr`")),
            "expected `[mnemonic.bt-bf]` error guiding to `bsr` on line 3 for `{form}`, got: {errs:#?}"
        );
    }
}

#[test]
fn bt_and_bf_are_still_refused_under_as_compat() {
    for (form, mnem, replacement) in [("bt .done", "bt", "bra"), ("bf .done", "bf", "bsr")] {
        let src = format!(
            "module m\n\
             @as_compat\n\
             proc P () {{\n\
                 {form}\n\
             .done:\n\
                 rts\n\
             }}\n"
        );
        let errs = errors(&src);
        assert!(
            errs.iter().any(|d| d.line == 4
                && d.message.starts_with(&format!("[mnemonic.bt-bf] `{mnem}`"))
                && d.message.contains(&format!("use `{replacement}`"))),
            "expected `[mnemonic.bt-bf]` error under @as_compat for `{form}`, got: {errs:#?}"
        );
    }
}

#[test]
fn uppercase_mixed_case_and_j_prefixed_bt_bf_are_refused() {
    for form in ["BT .done", "BF .done", "Bt .done", "jbt .done", "jbf .done"] {
        let src = format!(
            "module m\n\
             proc P () {{\n\
                 {form}\n\
             .done:\n\
                 rts\n\
             }}\n"
        );
        let errs = errors(&src);
        assert!(
            errs.iter().any(|d| d.line == 3 && d.message.contains("not a recognized 68000 mnemonic")),
            "expected unrecognized-mnemonic error on line 3 for `{form}`, got: {errs:#?}"
        );
    }
}

#[test]
fn bt_and_bf_inside_comptime_asm_template_are_refused() {
    for (line, expected, replacement) in [("bt .done", "bt", "bra"), ("bf .done", "bf", "bsr")] {
        let src = format!(
            "module m\n\
             comptime fn emit_bad() -> Code {{\n\
                 return asm {{\n\
                     {line}\n\
                 }}\n\
             }}\n\
             proc P () {{\n\
                 emit_bad()\n\
             .done:\n\
                 rts\n\
             }}\n"
        );
        let errs = errors(&src);
        assert!(
            errs.iter().any(|d| d.message.starts_with(&format!("[mnemonic.bt-bf] `{expected}`"))
                && d.message.contains(&format!("use `{replacement}`"))),
            "expected `[mnemonic.bt-bf]` refusal for `{line}` in comptime asm template, got: {errs:#?}"
        );
    }
}

#[test]
fn valid_t_f_instructions_and_real_branches_stay_accepted() {
    let src = "\
module m
proc Helper () {
    rts
}
proc P () clobbers(d0) {
    st d0
    sf d0
.loop:
    dbt d0, .loop
    dbf d0, .loop
    dbra d0, .loop
    bsr.s Helper
    beq.s .done
    bhs.s .done
    blo.s .done
    bra.s .done
.done:
    rts
}
";
    let errs = errors(src);
    assert!(errs.is_empty(), "valid 68000 forms must stay accepted, got: {errs:#?}");
}

#[test]
fn comptime_fn_named_bt_or_bf_does_not_warn_mnemonic_shadow() {
    let src = "\
module m
comptime fn bt() -> Code {
    return asm { nop }
}
comptime fn bf() -> Code {
    return asm { nop }
}
proc P () {
    bt
    bf
    rts
}
";
    let diags = lower(src);
    assert!(diags.is_empty(), "expected no diagnostics for comptime fns named bt/bf, got: {diags:#?}");
}
