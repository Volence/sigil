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
use sigil_ir::SymbolTable;
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
