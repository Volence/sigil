//! Integration tests for the classic-format `.emp` comptime compression
//! builtins (Plan-7 #10, T2b): `kosinski`/`kosinski_m`/`kosplus`/
//! `kosplus_m`/`saxman`/`enigma`/`nemesis`/`comper`/`rocket`. Mirrors
//! `tests/sandbox_zx0.rs` and `tests/s4lz_vectors.rs`'s e2e (parse -> eval
//! -> lower) pattern.
//!
//! CR4: every builtin here emits the RAW format stream — no aeon 4-byte
//! wrapper, no headers beyond what the format itself defines.
//!
//! Fixture provenance: `tests/vectors/classic/PROVENANCE.md`.
use sigil_frontend_emp::layout::eval_data_with_root;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{Cell, DataBuf};
use sigil_span::Diagnostic;
use std::path::{Path, PathBuf};

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("vectors").join("classic")
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
            other => panic!("unexpected non-Bytes cell: {other:?}"),
        }
    }
    out
}

fn read_vec(name: &str) -> Vec<u8> {
    std::fs::read(vectors_dir().join(name)).unwrap_or_else(|e| panic!("read vector {name}: {e}"))
}

// ---------------------------------------------------------------------------
// kosinski / kosinski_m — byte-exact gate (T2a golden reuse)
// ---------------------------------------------------------------------------

#[test]
fn kosinski_matches_t2a_golden() {
    let expected = read_vec("golden_kosinski.bin");
    let src = "module m\ndata X = kosinski(embed(\"level_select_2p.raw\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn kosinski_m_default_module_size_matches_sys_wrapper() {
    let plain = read_vec("sand_particles.raw");
    let expected = sigil_clownlzss_sys::compress_kosinski_moduled(&plain, 0x1000).unwrap();
    let src = "module m\ndata X = kosinski_m(embed(\"sand_particles.raw\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn kosinski_m_explicit_module_size_matches_sys_wrapper() {
    let plain = read_vec("sand_particles.raw");
    let expected = sigil_clownlzss_sys::compress_kosinski_moduled(&plain, 0x100).unwrap();
    let src = "module m\ndata X = kosinski_m(embed(\"sand_particles.raw\"), module_size: $100)\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

// ---------------------------------------------------------------------------
// kosinski / kosinski_m — diagnostics
// ---------------------------------------------------------------------------

#[test]
fn kosinski_non_data_arg_errors() {
    let src = "module m\ndata X = kosinski(42)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[kosinski.arg]")),
        "expected a [kosinski.arg] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn kosinski_no_args_errors() {
    // `kosinski` follows `zx0`'s sole-positional-arg pattern (exact arity
    // 1), not `s4lz`'s "requires a data argument" phrasing (s4lz has
    // optional named args, so it collects args rather than checking arity
    // directly) — see `eval_sole_data_arg` in `classic_compress.rs`.
    let src = "module m\ndata X = kosinski()\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("expects exactly 1 argument")),
        "expected an arity diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn kosinski_link_expr_input_errors() {
    let src = "module m\ndata X = kosinski(winptr(\"Foo\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[kosinski.arg]") && d.message.contains("link-expr")),
        "expected a [kosinski.arg] diagnostic naming link-expr, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn kosinski_m_module_size_over_0x1000_errors() {
    let src = "module m\ndata X = kosinski_m(embed(\"sand_particles.raw\"), module_size: $1001)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[kosinski_m.module-size]")),
        "expected a [kosinski_m.module-size] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn kosinski_m_module_size_zero_errors() {
    let src = "module m\ndata X = kosinski_m(embed(\"sand_particles.raw\"), module_size: 0)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[kosinski_m.module-size]")),
        "expected a [kosinski_m.module-size] diagnostic for module_size 0, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn kosinski_m_module_size_at_max_is_accepted() {
    let plain = read_vec("sand_particles.raw");
    let expected = sigil_clownlzss_sys::compress_kosinski_moduled(&plain, 0x1000).unwrap();
    let src = "module m\ndata X = kosinski_m(embed(\"sand_particles.raw\"), module_size: $1000)\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(flatten(&buf.expect("data buf")), expected);
}

#[test]
fn kosinski_m_module_size_wrong_type_errors() {
    let src = "module m\ndata X = kosinski_m(embed(\"sand_particles.raw\"), module_size: \"x\")\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("module_size")),
        "expected a module_size type diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn kosinski_m_unknown_named_arg_errors() {
    let src = "module m\ndata X = kosinski_m(embed(\"sand_particles.raw\"), bogus: 1)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("unknown named argument `bogus`")),
        "expected an unknown-named-argument diagnostic, got {diags:?}"
    );
    assert!(buf.expect("data buf").size > 0, "expected the call to still produce a real result");
}

#[test]
fn kosinski_m_module_size_given_twice_errors() {
    let src = "module m\ndata X = kosinski_m(embed(\"sand_particles.raw\"), module_size: $100, module_size: $200)\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("`module_size` given more than once")),
        "expected a module_size-given-twice diagnostic, got {diags:?}"
    );
    assert!(buf.expect("data buf").size > 0, "expected the call to still produce a real result");
}

#[test]
fn kosinski_m_two_positional_args_errors() {
    let src = "module m\ndata X = kosinski_m(embed(\"sand_particles.raw\"), embed(\"level_select_2p.raw\"))\n";
    let (buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("exactly one positional data argument")),
        "expected a too-many-positional-args diagnostic, got {diags:?}"
    );
    assert!(buf.expect("data buf").size > 0, "expected the first positional arg's result to still be used");
}
