//! T6 — the Plan-5 cross-feature corpus: `embed`/`import`/`zx0`/`as.*` exercised
//! TOGETHER through the REAL production lowering path (`lower_module`), byte-diffed
//! against the same committed references the individual Plan-5 task tests use
//! (`tests/sandbox_embed.rs`, `tests/sandbox_import.rs`, `tests/sandbox_zx0.rs`,
//! `tests/float_ns.rs`). Those tests prove each feature in isolation via the T4
//! `eval_data`/`eval_data_with_root` seam; this file proves they coexist and stay
//! byte-exact when routed through the T5 `lower_module` + `sigil_link` production
//! path (`include_root` wiring, per-CPU serialization, `resolve_layout`/`link`/
//! `flatten`) — the same harness as `tests/lower_data.rs` and `tests/lower_sections.rs`.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use std::path::{Path, PathBuf};

/// The fixture directory `embed`/`import` resolve paths against: `tests/vectors/`
/// (same as every other Plan-5 test file).
fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("vectors")
}

/// Lower `src` via the REAL production path (`lower_module` with `include_root`
/// set to [`vectors_dir`]), then link + flatten to a flat byte image — mirrors
/// `tests/lower_data.rs`'s harness exactly.
fn lower_and_flatten(src: &str) -> (Vec<u8>, Vec<sigil_span::Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: Some(vectors_dir()) },
    );
    if diags.is_empty() {
        let resolved =
            sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
        (sigil_link::flatten(&linked, 0x00), diags)
    } else {
        (Vec::new(), diags)
    }
}

#[test]
fn deform_table_through_production_lowering() {
    // Proves `as.*` works through the REAL lowering path (not just the T4
    // `eval_data` seam `tests/float_ns.rs` exercises): the same deform-table
    // recipe, lowered + linked + flattened, must byte-diff against the golden.
    let src = "module m\n\
               data Deform = bytes(for i in 0..256 { as.int(20 * as.sin(6.283185307179586 * i / 64)) })\n";
    let (bytes, diags) = lower_and_flatten(src);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    let golden = std::fs::read(vectors_dir().join("sine_goldens").join("rocking_a20_p64.bin"))
        .expect("read golden");
    assert_eq!(bytes, golden);
}

#[test]
fn zx0_embed_pipeline_through_production_lowering() {
    // THE HEADLINE end-to-end: `embed` -> `zx0` -> the 4-byte wrapper,
    // reproducing a `build.sh`-format compressed blob byte-for-byte through
    // production lowering (`include_root` wiring from T5, real `resolve_layout`/
    // `link`/`flatten`).
    let src = "module m\ndata Packed = zx0(embed(\"zx0_pipeline_input.bin\"))\n";
    let (bytes, diags) = lower_and_flatten(src);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    let expected = std::fs::read(vectors_dir().join("zx0_pipeline_input.zx0")).expect("read reference blob");
    assert_eq!(bytes, expected, "emp zx0(embed(...)) must match the build.sh reference byte-for-byte");
}

#[test]
fn typed_import_through_production_lowering() {
    let src = "module m\nstruct Point { x: u16, y: u16 }\ndata P: Point = import(\"point.json\")\n";
    let (bytes, diags) = lower_and_flatten(src);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(bytes, vec![0x00, 0x0A, 0x00, 0x14]);
}

#[test]
fn all_features_coexist_in_one_module() {
    // ONE module containing the deform table AND `zx0(embed(...))` AND the typed
    // import, as three top-level `data` items in declaration order. `lower_module`
    // opens a single default `text` section for consecutive top-level items
    // (`ensure_default` in `src/lower/mod.rs`) and concatenates their bytes in
    // declaration order — so the flattened image should be exactly
    // `rocking_golden ++ zx0_reference ++ [0,10,0,20]`.
    let src = "module m\n\
               data Deform = bytes(for i in 0..256 { as.int(20 * as.sin(6.283185307179586 * i / 64)) })\n\
               data Packed = zx0(embed(\"zx0_pipeline_input.bin\"))\n\
               struct Point { x: u16, y: u16 }\n\
               data P: Point = import(\"point.json\")\n";
    let (bytes, diags) = lower_and_flatten(src);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    let rocking = std::fs::read(vectors_dir().join("sine_goldens").join("rocking_a20_p64.bin"))
        .expect("read golden");
    let zx0_ref = std::fs::read(vectors_dir().join("zx0_pipeline_input.zx0")).expect("read reference blob");

    let mut expected = Vec::new();
    expected.extend_from_slice(&rocking);
    expected.extend_from_slice(&zx0_ref);
    expected.extend_from_slice(&[0x00, 0x0A, 0x00, 0x14]);

    assert_eq!(bytes.len(), rocking.len() + zx0_ref.len() + 4);
    assert_eq!(bytes, expected, "cross-feature module must lower/link/flatten in declaration order, byte-exact");
}

#[test]
fn sandbox_and_features_in_a_placed_section() {
    // A feature (`zx0(embed(...))`) inside a placed `section {}` (§7.1), proving
    // the sandbox root and the placement machinery compose: lowers cleanly, and
    // the section's linked bytes are the expected wrapped blob.
    let src = "module m\n\
               section pack (cpu: m68000, vma: $8000) {\n\
                 data D = zx0(embed(\"embed_fixture.bin\"))\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: Some(vectors_dir()) },
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    let resolved =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let section_bytes = linked.section("pack").expect("linked section").bytes.clone();

    let fixture = std::fs::read(vectors_dir().join("embed_fixture.bin")).expect("read fixture");
    assert_eq!(fixture.len(), 12);
    let mut expected = vec![0x00, 0x0C, 0x00, 0x02];
    expected.extend_from_slice(&sigil_salvador_sys::compress(&fixture));
    assert_eq!(section_bytes, expected);
}
