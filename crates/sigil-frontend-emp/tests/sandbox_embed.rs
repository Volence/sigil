//! Integration tests for the `embed(path, skip, len)` comptime builtin (Spec
//! 2, Plan 5 — Task 1): reads a file at comptime, within the capability
//! sandbox rooted at a fixed `include_root`, and yields its bytes (or a slice
//! of them) as a `Value::Data` — `BINCLUDE` parity with slicing. Also
//! exercises the shared sandbox path-resolution guard (`[sandbox.path-escape]`)
//! and the `embed`-specific diagnostics (`[embed.not-found]`, `[embed.range]`),
//! plus the embed-spec §1 data-item forms: length inference (no annotation) and
//! the explicit-length assertion (`[u8; N]` vs the file's actual byte count).
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
    let (buf, _asserts, ds) = eval_data_with_root(&file, name, None, Some(&vectors_dir()), &[]);
    (buf, ds)
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
        diags.iter().any(|d| d.message.contains("[embed.not-found]")),
        "expected an [embed.not-found] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

/// The `[embed.not-found]` message names the RESOLVED ABSOLUTE path (spec §2),
/// not the relative spelling — so an author sees exactly where the sandbox
/// looked. The fixture dir is the resolution root here.
#[test]
fn embed_missing_file_names_resolved_path() {
    let src = "module m\ndata X = embed(\"does_not_exist.bin\")\n";
    let (_buf, diags) = data(src, "X");
    let msg = diags
        .iter()
        .find(|d| d.message.contains("[embed.not-found]"))
        .map(|d| d.message.clone())
        .expect("an [embed.not-found] diagnostic");
    let resolved = vectors_dir().join("does_not_exist.bin");
    assert!(
        msg.contains(&resolved.display().to_string()),
        "message should name the resolved absolute path {resolved:?}, got {msg:?}"
    );
}

/// Spec §1 data-item form: omitting the annotation infers the length from the
/// file (the `[u8; _]` intent). A trailing explicit annotation is not required.
#[test]
fn embed_infers_length_without_annotation() {
    let src = "module m\npub data BootBlob = embed(\"embed_fixture.bin\")\n";
    let (buf, diags) = data(src, "BootBlob");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(buf.expect("data buf").size, 12);
}

/// Spec §1 data-item form: an explicit `[u8; N]` that MATCHES the file's byte
/// count lowers clean (the assertion holds).
#[test]
fn embed_explicit_length_exact_ok() {
    let src = "module m\npub data BootBlob: [u8; 12] = embed(\"embed_fixture.bin\")\n";
    let (buf, diags) = data(src, "BootBlob");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(buf.expect("data buf").size, 12);
}

/// Spec §1 data-item form: an explicit `[u8; N]` that does NOT match the file's
/// byte count is a compile error naming actual vs declared. The shipped surface
/// routes this through the general data-item size check (`[emit.size-mismatch]`)
/// rather than a bespoke `[embed.length-mismatch]` — see the parcel note's
/// deviation ledger. Either way the assertion fires with both counts.
#[test]
fn embed_explicit_length_mismatch_rejected() {
    let src = "module m\npub data BootBlob: [u8; 8] = embed(\"embed_fixture.bin\")\n";
    let (_buf, diags) = data(src, "BootBlob");
    let msg = diags
        .iter()
        .find(|d| d.message.contains("size-mismatch"))
        .map(|d| d.message.clone())
        .expect("a size-mismatch diagnostic");
    // Actual (12) and declared (8) both surface.
    assert!(msg.contains("12"), "message should name the actual 12 bytes, got {msg:?}");
    assert!(msg.contains('8'), "message should name the declared 8 bytes, got {msg:?}");
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
