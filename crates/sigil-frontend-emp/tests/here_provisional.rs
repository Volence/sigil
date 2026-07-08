//! The `here()`-vs-relaxation fix (NOTE-1): a PROVISIONAL `here()` — one after a
//! size-relaxable instruction (`jbra`/unsized branch/bare `jmp`/`jsr`) in the
//! open section — becomes a link-time value. It may be emitted (D-H.3) or guarded
//! (D-H.4) but MUST NOT silently size or steer comptime evaluation (D-H.2): every
//! such use is the loud `[here.provisional]` error.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;

fn msgs(src: &str) -> Vec<String> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    diags.into_iter().map(|d| d.message).collect()
}

/// A `jbra` to a far target makes every position after it provisional. A `here()`
/// used as a comptime array length there cannot be folded — it must refuse with
/// `[here.provisional]` (RED on master: master folds `here()` to a stale baseline
/// int and the array sizes silently against the wrong number).
#[test]
fn provisional_here_as_array_length_refuses() {
    // `jbra Far` before the data item; `Far` sits far enough that the jbra grows
    // past bra.s — either way, the jbra is a relaxable fragment in the section, so
    // the position is provisional. `[u8; here()]` needs a concrete comptime length.
    let src = "module m\n\
               proc p () {\n\
                 jbra Far\n\
               }\n\
               data Bad: [u8; here()] = []\n\
               proc Far () {\n\
                 rts\n\
               }\n";
    let ds = msgs(src);
    assert!(
        ds.iter().any(|m| m.contains("[here.provisional]")),
        "expected [here.provisional] for a provisional here() array length, got: {ds:?}"
    );
}

/// A `here()` steering a comptime `if` after a relaxable is also provisional.
#[test]
fn provisional_here_in_if_condition_refuses() {
    let src = "module m\n\
               proc p () {\n\
                 jbra Far\n\
               }\n\
               data Bad: u8 = if here() > 0 { 1 } else { 0 }\n\
               proc Far () {\n\
                 rts\n\
               }\n";
    let ds = msgs(src);
    assert!(
        ds.iter().any(|m| m.contains("[here.provisional]")),
        "expected [here.provisional] for a provisional here() if-condition, got: {ds:?}"
    );
}

/// EXACT positions are untouched: a `here()` with no relaxable before it in the
/// section is still a plain comptime int and sizes/steers as before (byte-exact
/// path). This is the byte-identical guarantee — no [here.provisional] here.
#[test]
fn exact_here_still_folds_and_steers() {
    // No relaxable before the data item → exact position → here() is Value::Int(0)
    // (default section, vma==lma==0), so the `if` folds normally.
    let src = "module m\n\
               data Ok: u8 = if here() == 0 { 7 } else { 9 }\n";
    let ds = msgs(src);
    assert!(ds.is_empty(), "exact here() must not refuse, got: {ds:?}");
}
