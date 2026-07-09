//! Byte-exact gate + diagnostic + e2e pipeline tests for the `s4lz(...)`
//! comptime builtin (Plan-7 #10, Tier 1, CR-S4LZ).
//!
//! The byte-exact vectors (`tests/vectors/s4lz/*.bin` / `*.s4lz`) were
//! generated from the REAL `aeon/tools/s4lz.py` encoder — see that
//! directory's `README.md` for exact provenance (s4lz.py path, aeon git rev,
//! regeneration script). Every pair is asserted equal to
//! `s4lz(embed(...))`'s output through the FULL `.emp` pipeline (parse ->
//! eval -> lower), not just the Rust core directly — the core itself has its
//! own separate gate in `sigil-s4lz/tests/byte_exact.rs`.
use sigil_frontend_emp::layout::eval_data_with_root;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{Cell, DataBuf};
use sigil_span::Diagnostic;
use std::path::{Path, PathBuf};

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("vectors").join("s4lz")
}

fn data(src: &str, name: &str) -> (Option<DataBuf>, Vec<Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (buf, _asserts, ds) = eval_data_with_root(&file, name, None, Some(&vectors_dir()), &[]);
    (buf, ds)
}

fn flatten(buf: &DataBuf) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.size);
    for cell in &buf.cells {
        match cell {
            Cell::Bytes(b) => out.extend_from_slice(b),
            other => panic!("unexpected non-Bytes cell in s4lz output: {other:?}"),
        }
    }
    out
}

fn read_vec(name: &str) -> Vec<u8> {
    std::fs::read(vectors_dir().join(name)).unwrap_or_else(|e| panic!("read vector {name}: {e}"))
}

// ---------------------------------------------------------------------------
// Byte-exact gate: s4lz(embed(...)) through the full `.emp` pipeline
// ---------------------------------------------------------------------------

#[test]
fn s4lz_payload_744_plain_matches_python() {
    let expected = read_vec("payload_744_plain.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"payload_744.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_payload_744_dict_matches_python() {
    let expected = read_vec("payload_744_dict.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"payload_744.bin\"), dict: embed(\"payload_744_dict.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_shield_block_768_plain_matches_python() {
    let expected = read_vec("shield_block_768_plain.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"shield_block_768.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_shield_block_768_dict768_matches_python() {
    let expected = read_vec("shield_block_768_dict768.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"shield_block_768.bin\"), dict: embed(\"shield_dict_768.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_shield_block_768_dict1536_matches_python() {
    let expected = read_vec("shield_block_768_dict1536.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"shield_block_768.bin\"), dict: embed(\"shield_dict_1536.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_shield_block_768_dict2304_matches_python() {
    let expected = read_vec("shield_block_768_dict2304.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"shield_block_768.bin\"), dict: embed(\"shield_dict_2304.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_edge_empty_matches_python() {
    let expected = read_vec("edge_empty.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"edge_empty.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_edge_odd1_matches_python() {
    let expected = read_vec("edge_odd1.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"edge_odd1.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_edge_boundary_offset_510_matches_python() {
    let expected = read_vec("edge_boundary_offset_510.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"edge_boundary_offset_510.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_edge_boundary_offset_512_matches_python() {
    let expected = read_vec("edge_boundary_offset_512.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"edge_boundary_offset_512.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_edge_both_extended_matches_python() {
    let expected = read_vec("edge_both_extended.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"edge_both_extended.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn s4lz_tile_delta_5tiles_matches_python() {
    let expected = read_vec("tile_delta_5tiles.s4lz");
    let src = "module m\ndata X = s4lz(embed(\"tile_delta_5tiles.bin\"), tile_delta: true)\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[test]
fn s4lz_no_args_errors() {
    let src = "module m\ndata X = s4lz()\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("requires a data argument")),
        "expected a missing-data-argument diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn s4lz_two_positional_args_errors() {
    // Mirrors `embed`'s pattern (sandbox.rs `eval_embed`): a repeated
    // positional argument is a non-fatal diagnostic — the FIRST positional
    // is still used as `data`, so the call still produces a real (non-empty)
    // result alongside the loud diagnostic.
    let src = "module m\ndata X = s4lz(embed(\"edge_empty.bin\"), embed(\"edge_odd1.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("exactly one positional data argument")),
        "expected a too-many-positional-args diagnostic, got {diags:?}"
    );
    assert!(buf.expect("data buf").size > 0, "expected the first positional arg's result to still be used");
}

#[test]
fn s4lz_unknown_named_arg_errors() {
    // Same non-fatal-diagnostic pattern as embed's unknown-named-argument
    // case: the diagnostic fires but the otherwise-valid call still compresses.
    let src = "module m\ndata X = s4lz(embed(\"edge_empty.bin\"), bogus: 1)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("unknown named argument `bogus`")),
        "expected an unknown-named-argument diagnostic, got {diags:?}"
    );
    assert!(buf.expect("data buf").size > 0, "expected the call to still produce a real result");
}

#[test]
fn s4lz_non_data_arg_errors() {
    let src = "module m\ndata X = s4lz(42)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[s4lz.arg]")),
        "expected a [s4lz.arg] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn s4lz_dict_and_tile_delta_together_errors() {
    let src = "module m\ndata X = s4lz(embed(\"payload_744.bin\"), dict: embed(\"payload_744_dict.bin\"), tile_delta: true)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[s4lz.dict-tile-delta-exclusive]")),
        "expected a [s4lz.dict-tile-delta-exclusive] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn s4lz_odd_dict_errors() {
    // 3-byte dict — deliberately word-odd.
    let src = "module m\ndata X = s4lz(embed(\"payload_744.bin\"), dict: bytes([1, 2, 3]))\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[s4lz.dict-odd]")),
        "expected a [s4lz.dict-odd] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn s4lz_tile_delta_wrong_type_errors() {
    let src = "module m\ndata X = s4lz(embed(\"payload_744.bin\"), tile_delta: 1)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("`tile_delta` must be a bool")),
        "expected a tile_delta-type diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn s4lz_dict_wrong_type_errors() {
    let src = "module m\ndata X = s4lz(embed(\"payload_744.bin\"), dict: 42)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[s4lz.dict-arg]")),
        "expected a [s4lz.dict-arg] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn s4lz_dict_given_twice_errors() {
    // Same non-fatal-diagnostic pattern: `dict` given twice still evaluates
    // (using the LAST-seen `dict:` argument, matching how `dict_arg` is
    // overwritten in `eval_s4lz`'s arg-collection loop) and compresses.
    let src = "module m\ndata X = s4lz(embed(\"payload_744.bin\"), dict: embed(\"payload_744_dict.bin\"), dict: embed(\"payload_744_dict.bin\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("`dict` given more than once")),
        "expected a dict-given-twice diagnostic, got {diags:?}"
    );
    assert!(buf.expect("data buf").size > 0, "expected the call to still produce a real result");
}
