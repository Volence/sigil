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

use sigil_ir::{Module, SymbolTable};

/// Link `m` (resolve_layout then link) and return one section's final bytes.
fn linked_bytes(m: &Module, section: &str) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(section).map(|s| s.bytes.clone()).unwrap_or_default()
}

fn lower_ok(src: &str) -> Module {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    let errs: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errs.is_empty(), "lower errors: {errs:?}");
    m
}

/// D-H.3: a PLAIN provisional `here()` emitted into a u32 field becomes a
/// SymRef to the item's own label, so the emitted address equals the item's
/// FINAL (post-relaxation) VMA — NOT the stale baseline. A `jbra` that grows
/// from bra.s (2B) to a wider form shifts the data item, and the emitted word
/// must track that shift.
///
/// RED on master: master folds here() to the BASELINE VMA (jbra counted at 2
/// bytes) and emits that stale constant, which diverges from the item's final
/// address once the jbra grows.
#[test]
fn provisional_here_emits_item_final_vma_as_symref() {
    // `jbra Far` with Far past +127 bytes forces jbra to grow beyond bra.s. The
    // data item H sits right after the jbra; its here() must equal H's own VMA.
    // A 200-byte pad proc between H and Far pushes Far out of bra.s reach.
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 proc p () {\n\
                   jbra Far\n\
                 }\n\
                 data H: u32 = here()\n\
                 data Pad = bytes(for i in 0..200 { 0 })\n\
                 proc Far () {\n\
                   rts\n\
                 }\n\
               }\n";
    let m = lower_ok(src);
    let bytes = linked_bytes(&m, "s");
    // jbra grows to 4 bytes (bra.w) reaching Far. So H is at $8000 + 4 = $8004.
    // The first 4 bytes are the jbra; the next 4 are H = its own VMA $00008004.
    assert_eq!(&bytes[4..8], &[0x00, 0x00, 0x80, 0x04], "H must emit its own final VMA; got {:02X?}", &bytes[4..8]);
}

/// D-H.3: a provisional here() into a 1-byte field is an error (no 8-bit
/// absolute fixup kind).
#[test]
fn provisional_here_into_u8_field_refuses() {
    let src = "module m\n\
               proc p () {\n\
                 jbra Far\n\
               }\n\
               data Bad: u8 = here()\n\
               proc Far () {\n\
                 rts\n\
               }\n";
    let ds = msgs(src);
    assert!(
        ds.iter().any(|m| m.contains("[here.provisional]") && m.contains("1-byte")),
        "expected a 1-byte-field refusal, got: {ds:?}"
    );
}

/// D-H.3: an ARITHMETICALLY-combined provisional here() emitted into a cell is
/// [here.provisional] (the general link-expr data cell is deferred, L-H.2).
#[test]
fn provisional_here_arithmetic_then_emit_refuses() {
    let src = "module m\n\
               proc p () {\n\
                 jbra Far\n\
               }\n\
               data Bad: u32 = here() + 4\n\
               proc Far () {\n\
                 rts\n\
               }\n";
    let ds = msgs(src);
    assert!(
        ds.iter().any(|m| m.contains("[here.provisional]")),
        "expected [here.provisional] for arithmetic-then-emit, got: {ds:?}"
    );
}
