//! Spec 2 · Plan 6 — per-file byte-diff harness + `.asm`→`.emp` port proof.
//!
//! The capstone proof: take a REAL Aeon `.asm` data file, assemble it in
//! isolation through the AS front-end (the reference bytes), compile its `.emp`
//! port through the modern front-end (the candidate bytes), and assert the two
//! are **byte-identical**. Plus the mixed-build link seam (T4) — an emp section
//! and an AS section composing into one linked image through the shared symbol
//! table.
//!
//! Target: `song_drumtest.asm` (82 bytes, pure `dc.b`, even length so `align 2`
//! is a no-op). Verified to assemble standalone via `sigil-frontend-as` under
//! `Cpu::M68000` (the `$xx` hex form requires 68k mode; under Z80 `$` is the
//! location counter). `sfx_33.asm` (58 bytes) is the documented fallback; both
//! are vendored under `tests/vectors/ports/` verbatim so this harness is
//! hermetic (it does not reach into the sibling `aeon/` tree).

use sigil_frontend_as::{assemble, Options};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::Level;

/// Assemble a single `.asm` source string through the AS front-end in isolation
/// (68k mode — the ports are 68k `dc.b` data), link with an empty external
/// table, and flatten to the reference bytes. Panics with the AS diagnostics on
/// failure (the ports are self-contained: no external symbols).
fn as_reference(asm: &str) -> Vec<u8> {
    let opts = Options { initial_cpu: Cpu::M68000, ..Options::default() };
    let module = assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble failed: {d:?}"));
    let linked = sigil_link::link(&module.sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Compile a `.emp` source string through the modern front-end to its flat
/// linked image — the same pipeline the `sigil emp` CLI runs (parse →
/// `lower_module` → `resolve_layout` → `link` → `flatten`), with no sandbox root
/// (these ports use no `embed`/`import`). Panics on any `Error`-level
/// diagnostic.
fn emp_candidate(emp: &str) -> Vec<u8> {
    let (file, pdiags) = parse_str(emp);
    assert!(
        pdiags.iter().all(|d| d.level != Level::Error),
        "emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != Level::Error),
        "emp lower errors: {ldiags:?}"
    );
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("emp resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("emp link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Assert two byte streams are identical, reporting the first differing offset
/// (and a short context window) on failure — the per-file byte-diff contract.
fn assert_byte_identical(reference: &[u8], candidate: &[u8], what: &str) {
    if reference == candidate {
        return;
    }
    let n = reference.len().min(candidate.len());
    if let Some(i) = (0..n).find(|&i| reference[i] != candidate[i]) {
        panic!(
            "{what}: first byte diff at offset {i:#x}: reference {:#04x} != candidate {:#04x}\n\
             reference[{i:#x}..] = {:02X?}\n candidate[{i:#x}..] = {:02X?}",
            reference[i],
            candidate[i],
            &reference[i..(i + 8).min(reference.len())],
            &candidate[i..(i + 8).min(candidate.len())],
        );
    }
    panic!(
        "{what}: lengths differ — reference {} bytes, candidate {} bytes (common prefix matches)",
        reference.len(),
        candidate.len()
    );
}

const DRUMTEST_ASM: &str = include_str!("vectors/ports/song_drumtest.asm");
const DRUMTEST_EMP: &str = include_str!("vectors/ports/song_drumtest.emp");

/// T1 — the AS reference side assembles standalone. Records the target choice:
/// `song_drumtest.asm` assembles in isolation to exactly its 82 source bytes
/// (the emitted image is the literal `dc.b` stream; `align 2` on an even length
/// is a no-op). This is the reference the emp port must reproduce byte-for-byte.
#[test]
fn as_reference_assembles_drumtest_standalone() {
    let bytes = as_reference(DRUMTEST_ASM);
    assert_eq!(bytes.len(), 82, "song_drumtest assembles to 82 bytes");
    // Spot-check the header + tail so a silent AS regression can't pass this.
    assert_eq!(&bytes[..4], &[0x07, 0x80, 0x00, 0x05]);
    assert_eq!(&bytes[80..], &[0x80, 0xEF]);
}

/// T1 — the harness pipeline is wired end-to-end: `emp_candidate` compiles a
/// trivial inline `[u8; N]` module to exactly its literal bytes, and
/// `assert_byte_identical` accepts an exact match. Proves both harness halves
/// before the real port lands (T2), so a T2 diff failure is unambiguously the
/// port, never the harness.
#[test]
fn harness_pipeline_roundtrips_inline_bytes() {
    let bytes = emp_candidate("module t\ndata X: [u8; 3] = [$AA, $BB, $CC]\n");
    assert_byte_identical(&[0xAA, 0xBB, 0xCC], &bytes, "harness self-test");
}

/// T2 — THE CAPSTONE. The `.emp` port of `song_drumtest.asm` compiles through
/// the modern front-end to bytes **byte-identical** to the AS-assembled
/// original. This is Plan 6's core acceptance criterion: a real Aeon data file,
/// ported and byte-exact.
#[test]
fn emp_port_matches_as_reference() {
    let reference = as_reference(DRUMTEST_ASM);
    let candidate = emp_candidate(DRUMTEST_EMP);
    assert_byte_identical(&reference, &candidate, "song_drumtest port");
}

/// T3 — `@as_compat` is proven **byte-neutral on the data path** (D-P6.3). The
/// port ships with `@as_compat`; stripping that one attribute line must not
/// change a single emitted byte (data emission is already AS-faithful — the
/// attribute's only effect is silencing modernization lints, of which a
/// data-only module has none). Both variants must also equal the AS reference.
#[test]
fn as_compat_is_byte_neutral_on_data() {
    assert!(
        DRUMTEST_EMP.contains("@as_compat"),
        "precondition: the port declares @as_compat"
    );
    // Strip exactly the `@as_compat` attribute line to build the no-compat twin
    // (prose comments mention the word, so filter the standalone attribute line,
    // not every occurrence of the substring).
    let without: String = DRUMTEST_EMP
        .lines()
        .filter(|l| l.trim() != "@as_compat")
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !without.lines().any(|l| l.trim() == "@as_compat"),
        "the twin has no @as_compat attribute line"
    );

    let with_compat = emp_candidate(DRUMTEST_EMP);
    let no_compat = emp_candidate(&without);
    assert_byte_identical(&with_compat, &no_compat, "@as_compat byte-neutrality");

    // And both still match the AS reference (byte-neutral means byte-exact).
    let reference = as_reference(DRUMTEST_ASM);
    assert_byte_identical(&reference, &with_compat, "with @as_compat vs AS");
    assert_byte_identical(&reference, &no_compat, "without @as_compat vs AS");
}

// ---------------------------------------------------------------------------
// T4 — mixed-build link seam (D-P6.2): an emp section and an AS section compose
// into ONE linked image through the shared flat symbol table. No new link
// infra — concat the two `Vec<Section>` and `resolve_layout` + `link` once.
// ---------------------------------------------------------------------------

/// The `Vec<Section>` an emp module lowers to (no sandbox root, 68k initial).
fn emp_sections(emp: &str) -> Vec<Section> {
    let (file, pdiags) = parse_str(emp);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "emp parse: {pdiags:?}");
    let (module, ldiags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "emp lower: {ldiags:?}");
    module.sections
}

/// The `Vec<Section>` an AS source assembles to (68k — pointer tables are 68k).
fn as_sections(asm: &str) -> Vec<Section> {
    let opts = Options { initial_cpu: Cpu::M68000, ..Options::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble: {d:?}")).sections
}

/// T4 — cross-seam symbol resolution. An emp section defines the ported symbol
/// `Song_DrumTest` at VMA $8000; a synthetic AS consumer (`dc.l Song_DrumTest`,
/// a pointer-table entry — the real consumer shape) references it. Concatenated
/// and linked ONCE, the AS fixup resolves through the shared table to the emp
/// symbol's VMA: $00008000, big-endian.
#[test]
fn mixed_build_cross_seam_symbol_resolves() {
    // emp defines the symbol at an explicit, distinctive VMA.
    let emp = "module seam.payload\n\
               section payload (cpu: m68000, vma: $8000) {\n\
                 data Song_DrumTest: [u8; 4] = [$07, $80, $00, $05]\n\
               }\n";
    // AS consumer references it (unresolved in-file → a link-time fixup).
    let asm = "Consumer:\n\tdc.l Song_DrumTest\n";

    let mut sections = emp_sections(emp);
    sections.extend(as_sections(asm));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link across the seam failed: {d:?}"));

    // The AS consumer lands in the auto-named `sec0` section; its 4 bytes are the
    // resolved pointer to the emp-defined `Song_DrumTest` ($8000), big-endian.
    let consumer = linked.section("sec0").expect("AS consumer section `sec0`");
    assert_eq!(consumer.bytes, vec![0x00, 0x00, 0x80, 0x00], "cross-seam pointer resolves to $8000");
}

/// T4 negative — a cross-section name collision between an emp-defined and an
/// AS-defined symbol of the SAME name is a hard link `Error`. The shared symbol
/// table admits exactly one definer per name regardless of producing front-end.
#[test]
fn mixed_build_cross_seam_name_collision_errors() {
    let emp = "module seam.payload\n\
               section payload (cpu: m68000, vma: $8000) {\n\
                 data Song_DrumTest: [u8; 2] = [$07, $80]\n\
               }\n";
    // The AS side ALSO defines `Song_DrumTest` — a genuine collision.
    let asm = "Song_DrumTest:\n\tdc.b $00\n";

    let mut sections = emp_sections(emp);
    sections.extend(as_sections(asm));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let err = sigil_link::link(&resolved, &SymbolTable::new())
        .expect_err("a cross-seam name collision must be a hard link error");
    assert!(
        err.iter().any(|d| d.level == Level::Error
            && d.message.contains("Song_DrumTest")
            && d.message.contains("redefined")),
        "expected a `Song_DrumTest redefined` error, got: {err:?}"
    );
}
