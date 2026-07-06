//! Integration tests for the `embed(path, skip, len)` comptime builtin (Spec
//! 2, Plan 5 — Task 1): reads a file at comptime, within the capability
//! sandbox rooted at a fixed `include_root`, and yields its bytes (or a slice
//! of them) as a `Value::Data` — `BINCLUDE` parity with slicing. Also
//! exercises the shared sandbox path-resolution guard (`[sandbox.path-escape]`)
//! and the `embed`-specific diagnostics (`[embed.read]`, `[embed.range]`).
use sigil_frontend_emp::layout::eval_data_with_root;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{Cell, DataBuf};
use sigil_span::Diagnostic;
use std::path::{Path, PathBuf};

/// The fixture directory `embed` resolves paths against for every test here:
/// `tests/vectors/`, containing the deterministic `embed_fixture.bin` (the
/// bytes `0x00..=0x0B`, 12 bytes).
fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("vectors")
}

/// Parse `src` (asserting a clean parse) and lower the data item named `name`,
/// resolving any `embed(...)` sandbox path against [`vectors_dir`].
fn data(src: &str, name: &str) -> (Option<DataBuf>, Vec<Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_data_with_root(&file, name, None, Some(&vectors_dir()))
}

const FIXTURE_BYTES: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

#[test]
fn embed_full_file() {
    let src = "module m\ndata X = embed(\"embed_fixture.bin\")\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 12);
    assert_eq!(buf.cells, vec![Cell::Bytes(FIXTURE_BYTES.to_vec())]);
}

#[test]
fn embed_with_skip_and_len() {
    let src = "module m\ndata X = embed(\"embed_fixture.bin\", skip: 2, len: 4)\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 4);
    assert_eq!(buf.cells, vec![Cell::Bytes(vec![2, 3, 4, 5])]);
}

#[test]
fn embed_path_escape_rejected() {
    let src = "module m\ndata X = embed(\"../secret.bin\")\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[sandbox.path-escape]")),
        "expected a [sandbox.path-escape] diagnostic, got {diags:?}"
    );
    // Poisoned: no bytes escape the sandbox.
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn embed_missing_file() {
    let src = "module m\ndata X = embed(\"does_not_exist.bin\")\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[embed.read]")),
        "expected an [embed.read] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn embed_range_out_of_bounds() {
    let src = "module m\ndata X = embed(\"embed_fixture.bin\", skip: 100, len: 100)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[embed.range]")),
        "expected an [embed.range] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}
