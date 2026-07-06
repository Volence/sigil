//! Integration tests for the `zx0(data)` comptime builtin (Spec 2, Plan 5 —
//! Task 3): ZX0-compresses a `Value::Data` at comptime and wraps it in the
//! exact 4-byte header `aeon/build.sh` hand-emits (`[u16 BE uncompressed-size]
//! [0x00][0x02]` ++ the raw salvador stream), producing bytes byte-identical
//! to the ROM's compressed art blobs.
use sigil_frontend_emp::layout::eval_data_with_root;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{Cell, DataBuf};
use sigil_span::Diagnostic;
use std::path::{Path, PathBuf};

/// The fixture directory `embed`/`zx0` resolve paths against for every test
/// here: `tests/vectors/`.
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

/// Flatten a `DataBuf`'s cells to raw bytes for comparison — every cell here
/// is expected to be a single `Cell::Bytes` (that's what `zx0` produces), but
/// walk generically anyway so a shape mismatch fails loudly instead of
/// panicking on an index.
fn flatten(buf: &DataBuf) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.size);
    for cell in &buf.cells {
        match cell {
            Cell::Bytes(b) => out.extend_from_slice(b),
            other => panic!("unexpected non-Bytes cell in zx0 output: {other:?}"),
        }
    }
    out
}

#[test]
fn zx0_wraps_and_compresses() {
    let fixture = std::fs::read(vectors_dir().join("embed_fixture.bin")).expect("read fixture");
    assert_eq!(fixture.len(), 12);
    let src = "module m\ndata C = zx0(embed(\"embed_fixture.bin\"))\n";
    let (buf, diags) = data(src, "C");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    let mut expected = vec![0x00, 0x0C, 0x00, 0x02];
    expected.extend_from_slice(&sigil_salvador_sys::compress(&fixture));
    assert_eq!(flatten(&buf), expected);
    assert_eq!(buf.size, expected.len());
}

#[test]
fn zx0_matches_build_sh_reference() {
    // The TRUE end-to-end gate, independent of our own `compress()`: the
    // reference `.zx0` was produced by running the REAL `salvador` CLI
    // exactly as `aeon/build.sh` does (see that file's ZX0-wrapping loop),
    // then hand-prepending the 4-byte header:
    //
    //   aeon/tools/salvador/salvador tests/vectors/zx0_pipeline_input.bin /tmp/x.tmp
    //   size=$(stat -c%s tests/vectors/zx0_pipeline_input.bin)   # 584 = 0x0248
    //   printf '%b' "$(printf '\\x%02x\\x%02x\\x00\\x02' $((size>>8)) $((size&255)))" \
    //       > tests/vectors/zx0_pipeline_input.zx0
    //   cat /tmp/x.tmp >> tests/vectors/zx0_pipeline_input.zx0
    //
    // This reference is committed as `tests/vectors/zx0_pipeline_input.zx0`
    // and never regenerated from our own crate — it's the ground truth.
    let expected =
        std::fs::read(vectors_dir().join("zx0_pipeline_input.zx0")).expect("read reference blob");
    let src = "module m\ndata C = zx0(embed(\"zx0_pipeline_input.bin\"))\n";
    let (buf, diags) = data(src, "C");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(flatten(&buf), expected, "emp zx0() output must match build.sh's reference byte-for-byte");
}

#[test]
fn zx0_symbolic_input_errors() {
    // `winptr(sym)` yields a `Data` holding a single `Cell::SymRef` — an
    // unresolved symbol reference with no concrete bytes to compress.
    let src = "module m\ndata C = zx0(winptr(\"Foo\"))\n";
    let (buf, diags) = data(src, "C");
    assert!(
        diags.iter().any(|d| d.message.contains("[zx0.symbolic]")),
        "expected a [zx0.symbolic] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn zx0_non_data_arg_errors() {
    let src = "module m\ndata C = zx0(42)\n";
    let (buf, diags) = data(src, "C");
    assert!(
        diags.iter().any(|d| d.message.contains("[zx0.arg]")),
        "expected a [zx0.arg] diagnostic, got {diags:?}"
    );
    assert_eq!(buf.expect("data buf").size, 0);
}

