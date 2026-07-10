//! Tranche 8 negative probes — the `dc.b`/`dc.w`/`dc.l` proc-body statement
//! (H8, the rings-port demanded feature) fails LOUD on every mis-use, against
//! a positive control that pins the emitted bytes (no false-comfort: each
//! doctored run pairs with a resolving one through the same plumbing).
//!
//! (a) POSITIVE CONTROL — `dc.b` string + ints, `dc.w`, `dc.l` emit the exact
//!     expected bytes (strings raw-ASCII per D2.16, scalars big-endian in a
//!     68k section, no implicit terminator, no alignment padding).
//! (b) `dc` with NO size suffix is `[dc.missing-size]`.
//! (c) An out-of-range element (`dc.b 256`) is `[dc.range]` — loud, never a
//!     silent truncation.
//! (d) A string element in `dc.w` is `[dc.string-width]` (a string is a run
//!     of bytes; it has no word reading).
//! (e) A non-comptime element (a register) is `[dc.comptime-only]` — the
//!     link-resolved-cell extension is recorded (ledger), not silently
//!     half-built.
//! (f) A comptime fn named `dc` can never shadow the statement (tenet 3 —
//!     `dc` is mnemonic-position reserved, same footing as jbra/jbsr): the
//!     statement still lowers as data.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;

/// Lower `src` (one self-contained module) to (diags, linked-section bytes).
/// The probe map places the module's `probe` section at 0x1000.
fn lower_probe(src: &str) -> (Vec<sigil_span::Diagnostic>, Option<Vec<u8>>) {
    let (file, pdiags) = parse_str(src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "probe parse errors (probes doctor SEMANTICS, not syntax): {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    if ldiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return (ldiags, None);
    }
    let map = "fill = 0x00\n[[region]]\nname = \"probe\"\nlma_base = 0x1000\nsize = 0x100\nkind = \"rom\"\n";
    let mapv = sigil_link::load_map(map).expect("probe map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &mapv);
    assert!(pdiags.iter().all(|d| d.level != sigil_span::Level::Error), "{pdiags:?}");
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("probe resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("probe link failed: {d:?}"));
    let bytes = linked.section("probe").map(|s| s.bytes.clone());
    (ldiags, bytes)
}

/// (a) The positive control: every width emits its exact bytes.
#[test]
fn dc_emits_exact_bytes() {
    let src = "module probe.dc_ok in probe\n\
               pub proc P () {\n\
               \tdc.b \"Hi:\", $E0, 1, -1\n\
               \tdc.w $1234, -2\n\
               \tdc.l $DEADBEEF\n\
               \trts\n\
               }\n";
    let (diags, bytes) = lower_probe(src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the control must lower clean: {diags:?}"
    );
    let bytes = bytes.expect("control must link");
    let expected: &[u8] = &[
        b'H', b'i', b':', 0xE0, 0x01, 0xFF, // dc.b — raw ASCII + ints, no terminator
        0x12, 0x34, 0xFF, 0xFE, // dc.w — big-endian in a 68k section
        0xDE, 0xAD, 0xBE, 0xEF, // dc.l
        0x4E, 0x75, // rts
    ];
    assert_eq!(&bytes[..expected.len()], expected, "dc bytes must be exact");
}

/// (b) `dc` with no width is loud.
#[test]
fn dc_without_size_is_loud() {
    let src = "module probe.dc_nosize in probe\n\
               pub proc P () {\n\
               \tdc 1, 2\n\
               \trts\n\
               }\n";
    let (diags, _) = lower_probe(src);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("[dc.missing-size]")),
        "expected [dc.missing-size], got: {diags:?}"
    );
}

/// (c) An element past the width's window is loud, never truncated.
#[test]
fn dc_out_of_range_is_loud() {
    let src = "module probe.dc_range in probe\n\
               pub proc P () {\n\
               \tdc.b 256\n\
               \trts\n\
               }\n";
    let (diags, _) = lower_probe(src);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("[dc.range]")),
        "expected [dc.range], got: {diags:?}"
    );
}

/// (d) A string in `dc.w` is loud.
#[test]
fn dc_word_string_is_loud() {
    let src = "module probe.dc_strw in probe\n\
               pub proc P () {\n\
               \tdc.w \"no\"\n\
               \trts\n\
               }\n";
    let (diags, _) = lower_probe(src);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("[dc.string-width]")),
        "expected [dc.string-width], got: {diags:?}"
    );
}

/// (e) A register element is loud — registers are OPERAND-level, not
///     expression values, so `d0` in an element (expression) position is the
///     evaluator's `unknown name` (never silent bytes). Pinned so a future
///     register-values change can't silently give `dc.w d0` a meaning.
#[test]
fn dc_register_element_is_loud() {
    let src = "module probe.dc_reg in probe\n\
               pub proc P () {\n\
               \tdc.w d0\n\
               \trts\n\
               }\n";
    let (diags, _) = lower_probe(src);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("unknown name")),
        "expected the evaluator's unknown-name error, got: {diags:?}"
    );
}

/// (e2) A comptime value of the WRONG KIND (a `Code` fragment) is
///     `[dc.comptime-only]`, naming the got-kind and steering to typed data
///     items; link-resolved dc cells are a recorded extension.
#[test]
fn dc_code_element_is_loud() {
    let src = "module probe.dc_code in probe\n\
               comptime fn frag() -> Code { return asm { nop } }\n\
               pub proc P () {\n\
               \tdc.w frag()\n\
               \trts\n\
               }\n";
    let (diags, _) = lower_probe(src);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("[dc.comptime-only]")),
        "expected [dc.comptime-only], got: {diags:?}"
    );
}

/// (f) `dc` is mnemonic-position RESERVED (tenet 3): a comptime fn named `dc`
///     never shadows the statement — the statement still lowers as data.
#[test]
fn dc_cannot_be_shadowed_by_comptime_fn() {
    let src = "module probe.dc_shadow in probe\n\
               comptime fn dc() -> int { return 0 }\n\
               pub proc P () {\n\
               \tdc.b 7\n\
               \trts\n\
               }\n";
    let (diags, bytes) = lower_probe(src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the reserved mnemonic must win cleanly: {diags:?}"
    );
    let bytes = bytes.expect("must link");
    assert_eq!(&bytes[..3], &[0x07, 0x4E, 0x75], "dc.b 7 then rts — the statement lowered as data");
}
