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

/// DELIBERATE PIN UPDATE (Plan 7 #7-main Task 3, D7.3/R7m.4 — S2-D13f
/// un-deferred): an ARITHMETICALLY-combined provisional here() emitted into a
/// data cell now EMITS as a general link-expr VALUE cell (`Cell::Expr` →
/// `ValueN` fixup), REPLACING the here-fix design's case-5 `[here.provisional]`
/// arithmetic-then-emit refusal. The here-fix design (D-H.3) deferred the
/// general link-expr data cell to L-H.2 and refused this path; R7m.4 lifts that
/// deferral verbatim, so the SAME source now compiles and folds at link.
///
/// Every OTHER provisional refusal is UNCHANGED (asserted elsewhere in this
/// file): array length, if-condition, max_size, byte/bytes elements, vma: — all
/// still `[here.provisional]`; and a PLAIN width-1 `here()` SymRef stays an
/// error (`provisional_here_into_u8_field_refuses`) since a bare-symbol address
/// cell has no 8-bit kind. Only Cell::Expr (a residual arithmetic tree) carries
/// width 1.
#[test]
fn provisional_here_arithmetic_then_emit_now_emits_value_cell() {
    // `here() + 4` into a u32 field: no longer a refusal — it emits and folds.
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 proc p () {\n\
                   jbra Far\n\
                 }\n\
                 data Ok: u32 = here() + 4\n\
                 data Pad = bytes(for i in 0..200 { 0 })\n\
                 proc Far () {\n\
                   rts\n\
                 }\n\
               }\n";
    let ds = msgs(src);
    assert!(
        !ds.iter().any(|m| m.contains("[here.provisional]")),
        "arithmetic-then-emit must NO LONGER refuse (D7.3/R7m.4), got: {ds:?}"
    );
    // And it folds to the FINAL value: Ok sits at $8004 (jbra grows to bra.w),
    // here()+4 = $8008, big-endian u32.
    let m = lower_ok(src);
    let bytes = linked_bytes(&m, "s");
    assert_eq!(&bytes[4..8], &[0x00, 0x00, 0x80, 0x08], "here()+4 must fold to $8008; got {:02X?}", &bytes[4..8]);
}

// ---- T4: deferred guards (D-H.4/D-H.5/D-H.8) --------------------------------

/// A provisional `ensure_fatal(here() <= N, ...)` DEFERS: the module carries a
/// LinkAssert (not a comptime pass/fail), and — because here() was used — an
/// anonymous anchor label `__here$m$0` is defined at the guard's cursor (D-H.8).
///
/// RED on master: master folds here() to a baseline int and the guard passes
/// silently at comptime (no LinkAssert, no anchor).
#[test]
fn provisional_item_guard_defers_and_mints_anchor() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 proc p () {\n\
                   jbra Far\n\
                 }\n\
                 ensure_fatal(here() <= $9000, \"overran at {here()}\")\n\
                 proc Far () {\n\
                   rts\n\
                 }\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    let errs: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errs.is_empty(), "provisional guard must defer (no comptime error): {errs:?}");
    assert_eq!(m.link_asserts.len(), 1, "expected exactly one deferred LinkAssert");
    let a = &m.link_asserts[0];
    assert!(a.fatal, "ensure_fatal should carry fatal=true");
    // The message keeps its comptime text and a lazy Expr part for `{here()}`.
    assert!(
        a.message.iter().any(|p| matches!(p, sigil_ir::MsgPart::Expr(_))),
        "the {{here()}} placeholder must be a lazy Expr part: {:?}",
        a.message
    );
    // An anonymous anchor label was minted (D-H.8).
    let has_anchor = m
        .sections
        .iter()
        .flat_map(|s| &s.labels)
        .any(|l| l.name.starts_with("__here$"));
    assert!(has_anchor, "expected a minted __here$ anchor label");
}

/// A provisional guard that does NOT use here() (e.g. `here` never appears —
/// but the section is provisional) still mints no anchor when here() is unused.
/// Here we prove the negative: an EXACT-position guard using here() defers
/// nothing and mints no anchor (byte-identical to before).
#[test]
fn exact_item_guard_does_not_defer() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 data D: u16 = $0000\n\
                 ensure_fatal(here() <= $9000, \"ok\")\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(diags.is_empty(), "exact guard should pass silently: {diags:?}");
    assert!(m.link_asserts.is_empty(), "exact guard must not defer");
    assert!(
        !m.sections.iter().flat_map(|s| &s.labels).any(|l| l.name.starts_with("__here$")),
        "no anchor for an exact-position guard"
    );
}

// ---- Review fold-in (NOTE-1): the D-H.2-named int-consumer sites must refuse a
// provisional here() with the SPECIFIC [here.provisional] steering message, not
// the generic "expected an integer, got link-expr". ----------------------------

/// Shared program shape: a jbra makes every later position provisional; `item`
/// is the site under test, spliced after the relaxable proc.
fn provisional_msgs(item: &str) -> Vec<String> {
    let src = format!(
        "module m\n\
         proc p () {{\n\
           jbra Far\n\
         }}\n\
         {item}\n\
         proc Far () {{\n\
           rts\n\
         }}\n"
    );
    msgs(&src)
}

fn assert_provisional(item: &str) {
    let ds = provisional_msgs(item);
    assert!(
        ds.iter().any(|m| m.contains("[here.provisional]")),
        "expected the specific [here.provisional] message for `{item}`, got: {ds:?}"
    );
}

/// `(max_size: here())` — a capacity bound cannot be a link-time value (D-H.2
/// names max_size explicitly).
#[test]
fn provisional_here_as_max_size_refuses_specifically() {
    assert_provisional("data D (max_size: here()): u8 = 0");
}

/// `byte(here())` / `bytes([here()])` — the Data constructors (D-H.2-named
/// builtins) take concrete comptime integers.
#[test]
fn provisional_here_in_byte_refuses_specifically() {
    assert_provisional("data D = byte(here())");
}

#[test]
fn provisional_here_in_bytes_element_refuses_specifically() {
    assert_provisional("data D = bytes([here()])");
}

/// Representative for the remaining int-consumer sites: a section `vma:`
/// attribute (eval_attr_int) steering placement from a link-time value.
#[test]
fn provisional_here_as_section_vma_refuses_specifically() {
    let src = "module m\n\
               proc p () {\n\
                 jbra Far\n\
               }\n\
               proc Far () {\n\
                 rts\n\
               }\n\
               section s (cpu: m68000, vma: here()) {\n\
                 data D: u8 = 0\n\
               }\n";
    let ds = msgs(src);
    // here() in an attribute expression carries no position today (attr eval has
    // no here_base) — accept the specific provisional message or the no-position
    // error; what must NOT happen is a silent fold or the generic "expected an
    // integer".
    assert!(
        ds.iter().any(|m| m.contains("[here.provisional]") || m.contains("no current position")),
        "vma: here() must refuse loudly, got: {ds:?}"
    );
}
