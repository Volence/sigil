//! T8b flagship gate: `deform_table_sine` (Aeon's `sin`/`int`-driven deform
//! table generator, `engine/parallax_macros.inc:211`) must assemble
//! byte-for-byte identical to the 256-byte tables shipped as goldens
//! (`tests/vectors/sine_goldens/*.bin`), captured from real `asl`/Aeon builds.
//!
//! The macro body is inlined here rather than `include`d (the harness has no
//! filesystem include-root wired up for this crate's tests) — this is
//! byte-for-byte the same macro text as `parallax_macros.inc`, just without
//! the surrounding file. See `full_macro_matches_shimmer_via_invocation` for
//! the end-to-end proof (this crate's front end alone) and
//! `crates/sigil-frontend-as/tests/asl_snippets.rs`'s `deform_table_sine_*`
//! blocks (added by this same change) for the real-`asl` cross-check.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::SymbolTable;

const MACRO_SRC: &str = r#"
deform_table_sine macro AMPLITUDE,PERIOD
    if "AMPLITUDE" = ""
        fatal "deform_table_sine: AMPLITUDE required"
    endif
    if "PERIOD" = ""
        fatal "deform_table_sine: PERIOD required"
    endif
    if (256 # PERIOD) <> 0
        fatal "deform_table_sine: PERIOD=\{PERIOD} must divide 256"
    endif
deform_sine_i set 0
    rept 256
        dc.b int(AMPLITUDE * sin(6.283185307179586 * deform_sine_i / PERIOD))
deform_sine_i set deform_sine_i + 1
    endr
    endm
"#;

fn assemble_bytes(asm: &str) -> Vec<u8> {
    let module = assemble(asm, &Options::default()).expect("assemble");
    let resolved =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

fn table_for(amplitude: i64, period: i64) -> Vec<u8> {
    let src = format!(
        "{MACRO_SRC}\n        cpu 68000\n        padding off\n        phase 0\n        deform_table_sine AMPLITUDE={amplitude}, PERIOD={period}\n"
    );
    let bytes = assemble_bytes(&src);
    assert_eq!(bytes.len(), 256, "expected exactly 256 emitted bytes");
    bytes
}

fn golden(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/vectors/sine_goldens/{name}.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

#[test]
fn ojz_calm_a96_p64_matches_golden() {
    assert_eq!(table_for(96, 64), golden("ojz_calm_a96_p64"));
}

#[test]
fn rocking_a20_p64_matches_golden() {
    assert_eq!(table_for(20, 64), golden("rocking_a20_p64"));
}

#[test]
fn haze_a16_p64_matches_golden() {
    assert_eq!(table_for(16, 64), golden("haze_a16_p64"));
}

#[test]
fn shimmer_a8_p32_matches_golden() {
    assert_eq!(table_for(8, 32), golden("shimmer_a8_p32"));
}
