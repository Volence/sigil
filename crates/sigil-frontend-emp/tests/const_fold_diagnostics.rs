//! The definition-site `pub const` fold probe must distinguish an evaluation that
//! FAILED from one that simply produced no literal.
//!
//! `collect_pub_comptime` folds each imported `pub const` against its DEFINING
//! file, so the injected clone carries a self-contained value instead of an
//! expression that would re-resolve in the consumer's scope. When that probe
//! cannot produce a literal the clone keeps its original expression — which is
//! correct for a name the probe's narrower scope simply cannot see, and silently
//! WRONG for a const whose own definition is broken: the consumer recomputes the
//! expression in its namespace, where a same-named sibling can give a different
//! answer with no diagnostic anywhere.
//!
//! The gates below pin both halves: a fold that failed on a real fault surfaces
//! its diagnostic, and a fold that merely found no literal stays silent.

use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{build_program, manifest::Manifest};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

/// A build whose `embed_base` is a SUBDIRECTORY of `include_root`, the aeon
/// convention (a module's `embed("blob.bin")` names a path relative to its own
/// asset directory, not to the scan root). The definition-site probe evaluates
/// with `include_root` alone, so this is the skew under which an `embed` that the
/// real lowering reads fine cannot be resolved by the probe.
struct Build {
    dir: tempfile::TempDir,
    sections: Vec<Section>,
    diags: Vec<Diagnostic>,
}

fn build_with_asset_base(files: &[(&str, &str)], assets: &[(&str, &[u8])], entry: &str) -> Build {
    let dir = tempfile::tempdir().unwrap();
    for (rel, content) in files {
        write(dir.path(), rel, content);
    }
    std::fs::create_dir_all(dir.path().join("assets")).unwrap();
    for (rel, bytes) in assets {
        std::fs::write(dir.path().join("assets").join(rel), bytes).unwrap();
    }
    let (manifest, mdiags) = Manifest::scan(dir.path());
    assert!(
        mdiags.iter().all(|d| d.level != Level::Error),
        "manifest errors: {:?}",
        mdiags.iter().filter(|d| d.level == Level::Error).collect::<Vec<_>>()
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.path().to_path_buf()),
        embed_base: Some(dir.path().join("assets")),
        defines: vec![],
    };
    let (sections, _asserts, diags) = build_program(&manifest, entry, None, &opts);
    Build { dir, sections, diags }
}

/// A plain build: no comptime file reads, so no roots to skew.
fn build(files: &[(&str, &str)], entry: &str) -> Build {
    let dir = tempfile::tempdir().unwrap();
    for (rel, content) in files {
        write(dir.path(), rel, content);
    }
    let (manifest, mdiags) = Manifest::scan(dir.path());
    assert!(
        mdiags.iter().all(|d| d.level != Level::Error),
        "manifest errors: {:?}",
        mdiags.iter().filter(|d| d.level == Level::Error).collect::<Vec<_>>()
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (sections, _asserts, diags) = build_program(&manifest, entry, None, &opts);
    Build { dir, sections, diags }
}

fn errors(b: &Build) -> Vec<String> {
    b.diags
        .iter()
        .filter(|d| d.level == Level::Error)
        .map(|d| d.message.clone())
        .collect()
}

/// Flatten the whole program to one byte image — used to show WHAT a silent
/// fold failure emitted, not merely that it was silent.
fn flatten(b: &Build) -> Vec<u8> {
    let _keep = &b.dir;
    let resolved = sigil_link::resolve_layout(&b.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

// ---- headline: a fold that failed on a real fault is not silent -------------

/// The defect. `a.MARKER` is `SCALE * embed("blob.bin").len`; `SCALE` is `a`'s own
/// private const, and `blob.bin` is 4 bytes, so `MARKER` is 4 and the consumer's
/// `data D: u16` must be `$0004`.
///
/// The definition-site probe evaluates with `include_root` only, so its
/// `embed("blob.bin")` looks under the scan root rather than under `assets/` and
/// raises `[embed.not-found]`. Discard that Error and the fold yields no literal,
/// the clone keeps `SCALE * embed("blob.bin").len`, and the consumer recomputes it
/// against ITS `SCALE` of 100 — emitting `$0190` (400) from a build with zero
/// diagnostics.
///
/// The gate is on the diagnostic, not the byte: the fault must reach the user.
#[test]
fn failed_fold_surfaces_its_diagnostic() {
    let a = "\
module a
const SCALE: u16 = 1
pub const MARKER: u16 = SCALE * embed(\"blob.bin\").len
";
    let b = "\
module b
use a.{MARKER}
const SCALE: u16 = 100
data D: u16 = MARKER
";
    let built = build_with_asset_base(
        &[("a.emp", a), ("b.emp", b)],
        &[("blob.bin", &[1u8, 2, 3, 4])],
        "b",
    );
    let errs = errors(&built);
    assert!(
        errs.iter().any(|m| m.contains("[embed.not-found]")),
        "the failed fold of `a.MARKER` reported nothing; errors={errs:?}, image={:?}",
        if errs.is_empty() { flatten(&built) } else { Vec::new() },
    );
}

/// A fault the consumer DOES re-raise is reported once, not twice. Here the
/// consumer demands the const and its own evaluation hits the same unreadable
/// `embed`, so the definition-site report must collapse into that one.
#[test]
fn failed_fold_does_not_double_report() {
    let a = "\
module a
pub const BLOB_LEN: u16 = embed(\"missing.bin\").len
";
    let b = "\
module b
use a.{BLOB_LEN}
data D: u16 = BLOB_LEN
";
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.emp", a);
    write(dir.path(), "b.emp", b);
    let (manifest, _m) = Manifest::scan(dir.path());
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.path().to_path_buf()),
        embed_base: None,
        defines: vec![],
    };
    let (_s, _a, diags) = build_program(&manifest, "b", None, &opts);
    let hits: Vec<&Diagnostic> =
        diags.iter().filter(|d| d.message.contains("[embed.not-found]")).collect();
    assert_eq!(hits.len(), 1, "expected exactly one report, got {hits:?}");
}

// ---- control: an ordinary non-foldable const stays quiet --------------------

/// A `pub const` derived from a name the DEFINING file cannot see is the probe's
/// own scope shortfall, not a fault. It is also the entire error population the
/// shipped corpus's fold probes raise, so a fix that reported every failed probe
/// would bury the build in false alarms.
///
/// `a.DERIVED` reads `TILE_SIZE`, which `a` imports from `c`; the probe evaluates
/// `a` alone and misses it. The consumer resolves it and the build must be clean.
#[test]
fn scope_shortfall_stays_quiet() {
    let c = "\
module c
pub const TILE_SIZE: u16 = 32
";
    let a = "\
module a
use c.{TILE_SIZE}
pub const DERIVED: u16 = TILE_SIZE * 2
";
    let b = "\
module b
use a.{DERIVED}
use c.{TILE_SIZE}
data D: u16 = DERIVED
";
    let built = build(&[("a.emp", a), ("b.emp", b), ("c.emp", c)], "b");
    assert!(errors(&built).is_empty(), "scope shortfall reported: {:?}", errors(&built));
    assert_eq!(flatten(&built), vec![0x00, 0x40], "DERIVED must still be 64");
}

/// The other half of the corpus population: a call to a comptime fn another
/// module owns (`unknown function`). Same rule, same silence.
#[test]
fn unknown_function_shortfall_stays_quiet() {
    let c = "\
module c
pub comptime fn double(n: u16) -> u16 { return n * 2 }
";
    let a = "\
module a
use c.{double}
pub const DERIVED: u16 = double(21)
";
    let b = "\
module b
use a.{DERIVED}
use c.{double}
data D: u16 = DERIVED
";
    let built = build(&[("a.emp", a), ("b.emp", b), ("c.emp", c)], "b");
    assert!(errors(&built).is_empty(), "shortfall reported: {:?}", errors(&built));
    assert_eq!(flatten(&built), vec![0x00, 0x2A], "DERIVED must still be 42");
}

/// A const that evaluates cleanly to something that is not an `i64` literal is
/// not a fault either — it never raised a diagnostic to begin with, and the
/// consumer reads its expression exactly as before.
#[test]
fn non_integer_const_stays_quiet() {
    let a = "\
module a
pub const NAME = \"hello\"
pub const OK: u16 = 7
";
    let b = "\
module b
use a.{NAME, OK}
data D: u16 = OK
";
    let built = build(&[("a.emp", a), ("b.emp", b)], "b");
    assert!(errors(&built).is_empty(), "non-int const reported: {:?}", errors(&built));
    assert_eq!(flatten(&built), vec![0x00, 0x07]);
}

/// A build with no imported consts at all must gain nothing — the drain is a
/// no-op where there was never a probe.
#[test]
fn plain_build_gains_no_diagnostics() {
    let b = "\
module b
data D: u16 = $1234
";
    let built = build(&[("b.emp", b)], "b");
    assert!(built.diags.is_empty(), "plain build gained diagnostics: {:?}", built.diags);
    assert_eq!(flatten(&built), vec![0x12, 0x34]);
}
