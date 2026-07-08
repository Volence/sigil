//! Capability-sandbox hermeticity + the content-hash capture ledger (Spec 2,
//! Plan 5 — Task 5): closes three findings deferred from the Task-1 review.
//!
//! - **A**: `LowerOptions::include_root` actually reaches `embed`/`import`
//!   through the PRODUCTION lowering path (`lower_module`), not just the
//!   test-only `eval_data_with_root` seam Tasks 1-3 exercised directly.
//! - **B**: symlink containment — a unit-test concern, covered in
//!   `src/eval/sandbox.rs`'s `#[cfg(test)] mod tests` (it needs the
//!   `pub(crate)` `resolve_sandbox_path` seam directly, which an integration
//!   test cannot reach).
//! - **C**: the capture ledger is exposed publicly (`layout::Capture` +
//!   `layout::eval_data_captures`), is deterministic across repeated runs, and
//!   no OTHER module in the crate secretly opens a nondeterministic/
//!   external-world edge outside the one declared in `eval/sandbox.rs`.
use sha2::{Digest, Sha256};
use sigil_frontend_emp::layout::eval_data_captures;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use std::path::{Path, PathBuf};

/// The fixture directory `embed`/`import` resolve paths against: `tests/vectors/`,
/// containing the deterministic `embed_fixture.bin` (12 bytes, `0x00..=0x0B`).
fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("vectors")
}

const FIXTURE_BYTES: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

// ---- A: include_root through production lowering ----------------------

#[test]
fn lower_module_with_include_root_resolves_embed() {
    let src = "module m\ndata X = embed(\"embed_fixture.bin\")\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: Some(vectors_dir()) },
    );
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");

    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let bytes = sigil_link::flatten(&linked, 0x00);
    assert_eq!(bytes, FIXTURE_BYTES.to_vec());
}

#[test]
fn lower_module_without_include_root_reports_no_root() {
    // The documented production default (until a CLI wires a real root in):
    // `embed`/`import` inside a lowered `data` item hits `[sandbox.no-root]`.
    let src = "module m\ndata X = embed(\"embed_fixture.bin\")\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (_module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        diags.iter().any(|d| d.message.contains("[sandbox.no-root]")),
        "expected [sandbox.no-root], got {diags:?}"
    );
}

// ---- C: public capture ledger + determinism + no-other-edges ----------

#[test]
fn eval_data_captures_records_exactly_one_edge_with_pinned_hash() {
    let src = "module m\ndata X = embed(\"embed_fixture.bin\")\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (buf, captures, diags) =
        eval_data_captures(&file, "X", None, Some(&vectors_dir()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(buf.expect("data buf").size, 12);

    assert_eq!(captures.len(), 1, "expected exactly one capture edge, got {captures:?}");
    let cap = &captures[0];

    let expected_path = std::fs::canonicalize(vectors_dir())
        .expect("canonicalize vectors dir")
        .join("embed_fixture.bin");
    assert_eq!(cap.path, expected_path);
    assert_eq!(cap.len, 12);

    let mut hasher = Sha256::new();
    hasher.update(FIXTURE_BYTES);
    let expected_hash: [u8; 32] = hasher.finalize().into();
    assert_eq!(cap.hash, expected_hash);
}

#[test]
fn eval_data_captures_is_deterministic_across_runs() {
    let src = "module m\ndata X = embed(\"embed_fixture.bin\")\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (buf1, captures1, diags1) = eval_data_captures(&file, "X", None, Some(&vectors_dir()));
    let (buf2, captures2, diags2) = eval_data_captures(&file, "X", None, Some(&vectors_dir()));

    assert!(diags1.is_empty() && diags2.is_empty());
    assert_eq!(buf1.expect("buf1").cells, buf2.expect("buf2").cells);
    assert_eq!(captures1, captures2);
}

/// Hermeticity, D-P5.6: the ONLY module in this crate that may open an edge to
/// the external, nondeterministic world (an env var read, a filesystem read, a
/// wall-clock/monotonic-clock read, or a subprocess spawn) is `eval/sandbox.rs`
/// — the ONE declared capture edge `embed`/`import` route through. Every other
/// evaluator/lowering module must be a pure function of its `.emp` source (plus
/// whatever `include_root`-sandboxed bytes flow through `sandbox.rs` itself).
///
/// Design choice: rather than a broad "no OTHER module touches `std::fs`" scan
/// (which risks false positives the moment some unrelated module legitimately
/// needs, say, a `Path` helper that happens to share a substring), this scans
/// for a NARROW, high-signal set of APIs that have NO legitimate use ANYWHERE
/// in this crate except the sandboxed file read itself:
///
///   - `std::env::` (reads process environment — nondeterministic across runs)
///   - `std::fs::` (any filesystem access at all)
///   - `SystemTime`/`Instant::now` (wall-clock/monotonic reads)
///   - `std::process::` (subprocess spawn / exit)
///
/// Each facility is matched in BOTH spellings — the fully-qualified call style
/// (`std::fs::read`) AND the imported style (`use std::fs; fs::read`, including
/// `use std::fs as f` and `use std::fs::read`). Since any use of one of these
/// facilities requires either a fully-qualified path or a `use` of its module,
/// the two pattern sets together close the `use`-aliasing bypass (an earlier
/// version keyed only on `std::fs::` and would have missed `use std::fs;`).
///
/// As of this test, EVERY hit across the entire `src/` tree is confined to
/// `eval/sandbox.rs`. A future PR that adds a genuinely new legitimate use
/// elsewhere must extend an explicit allowlist here (documenting why), rather
/// than silently widening the hermeticity boundary — so this test is a
/// tripwire, not a rubber stamp. (It is a source-substring guard, not a
/// type-level proof: a determined author could still reach the outside world
/// via a transitive dependency or `unsafe`/FFI that names none of these — the
/// guard raises the bar against ACCIDENTAL edges, it does not sandbox a hostile
/// contributor.)
#[test]
fn no_hidden_external_world_edges_outside_sandbox_rs() {
    let forbidden = [
        // Fully-qualified call style (`std::fs::read(...)`).
        "std::env::",
        "std::fs::",
        "SystemTime",
        "Instant::now",
        "std::process::",
        // Imported style (`use std::fs; fs::read(...)`, incl. `as` aliases and
        // `use std::fs::read`). Any use of these facilities needs EITHER a
        // fully-qualified path (caught above) OR a `use` of the module (caught
        // here), so the two sets together close the aliasing bypass. `use
        // std::fmt`/`std::cmp`/etc. do not contain these substrings.
        "use std::env",
        "use std::fs",
        "use std::process",
        "use std::time",
    ];
    // The files allowed to reference any of the above.
    //   - `eval/sandbox.rs`: the declared capability-sandbox edge itself, the
    //     one route `embed`/`import` byte reads take through the evaluator.
    //   - `resolve/manifest.rs`: the module-manifest scanner (Spec 2 §3.1). It
    //     is a build-driver concern OUTSIDE the pure evaluator: scanning a root
    //     directory for `.emp` files and reading each is inherently filesystem
    //     work (`read_dir` + `read_to_string`), and cannot be expressed as a
    //     pure function of a single `.emp` source. It never participates in
    //     `data` evaluation, so it does not widen the evaluator's hermeticity
    //     boundary.
    //   - `resolve/mod.rs`: the cross-module build driver (Spec 2 §3). Its
    //     `entry_id_for_path` matches the CLI's entry path against the manifest's
    //     module paths, which requires `std::fs::canonicalize` to compare through
    //     symlinks/relative segments — a build-driver path-resolution concern,
    //     not evaluation. Like `manifest.rs` it never participates in `data`
    //     evaluation, so it does not widen the evaluator's hermeticity boundary.
    let allowlisted_paths = ["eval/sandbox.rs", "resolve/manifest.rs", "resolve/mod.rs"];

    let src_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders: Vec<String> = Vec::new();
    visit_rs_files(&src_root, &mut |path| {
        let rel = path
            .strip_prefix(&src_root)
            .expect("path under src_root")
            .to_string_lossy()
            .replace('\\', "/");
        if allowlisted_paths.contains(&rel.as_str()) {
            return;
        }
        let text = std::fs::read_to_string(path).expect("read source file");
        for pat in forbidden {
            if text.contains(pat) {
                offenders.push(format!("{rel}: contains {pat:?}"));
            }
        }
    });

    assert!(
        offenders.is_empty(),
        "found external-world/nondeterministic API use outside eval/sandbox.rs:\n{}",
        offenders.join("\n")
    );
}

/// Recursively visit every `.rs` file under `dir`, calling `f` on each path.
fn visit_rs_files(dir: &Path, f: &mut impl FnMut(&Path)) {
    let entries = std::fs::read_dir(dir).expect("read_dir src");
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, f);
        } else if path.extension().is_some_and(|e| e == "rs") {
            f(&path);
        }
    }
}
