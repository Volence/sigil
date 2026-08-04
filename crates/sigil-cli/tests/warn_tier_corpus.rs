//! THE WARN TIER over the real aeon corpus.
//!
//! `sigil build` prints a one-line tally of every warn-tier lint that fires. This
//! gate is the machine that watches the same thing when nobody is reading the build
//! log: the SET of firing lint ids per shape must equal [`WARN_ID_BASELINE`], so a
//! newly-merged lint cannot fire across the whole corpus and leave no trace.
//!
//! WHY IDS AND NOT COUNTS. The house baseline shape (`out_verify_corpus`'s
//! `D1C_BASELINE`) pins identified rows, and the natural analogue here would be a
//! per-`(shape, id)` count. It is deliberately NOT pinned, for two reasons:
//!   - The counts are SHAPE-DEPENDENT and move on ordinary engine work — most
//!     `[proc.sr-undeclared]` firings are the `assert` desugar's own
//!     `move.w sr,-(sp)` / `move.w (sp)+,sr` pair, so adding one debug assert to one
//!     proc moves the number. A baseline that churns on unrelated work gets
//!     rubber-stamped, and a rubber-stamped gate asserts nothing.
//!   - The id SET is shape-INVARIANT and moves only when a lint starts or stops
//!     firing on the corpus — which is exactly the event that must never pass
//!     unnoticed. It also has teeth in the retirement direction: clearing a class
//!     fails this gate until the win is recorded here.
//!
//! The limitation is honest and stated: this gate does NOT catch growth WITHIN an
//! already-firing class. That is the build's tally line's job.
//!
//! Reference tree: defaults to the sibling aeon checkout (override with `AEON_DIR`);
//! under `SIGIL_STRICT_GATE` a missing tree HARD-FAILS, the house pattern.

use sigil_harness::native;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The warn-tier lint classes the aeon corpus fires. Named once so retiring a
/// class — or admitting a new one — is the ONE-LINE diff it should be, which is the
/// most conspicuous form this baseline's escape hatch can take.
///
/// Every id here is a KNOWN-OPEN class, triaged in
/// `docs/superpowers/notes/2026-08-04-warning-tier.md` — which also records which
/// firings are true positives and which are lint gaps. Listing an id says "known",
/// not "fine".
const CORPUS_LINTS: &[&str] = &[
    "module.path-mismatch",
    "proc.clobber-undeclared",
    "proc.out-unwritten",
    "proc.sr-undeclared",
    "proc.undeclared-fallthrough",
];

/// The firing set per build shape. Every SHIPPED shape has its own row, so a lint
/// that fires in one shape only cannot hide behind a shape nobody watches. The rows
/// share [`CORPUS_LINTS`] while their sets agree; a shape that diverges spells its
/// own set out, and that divergence is then visible in the diff.
const WARN_ID_BASELINE: &[(&str, &[&str])] = &[
    ("sonic4 plain", CORPUS_LINTS),
    ("sonic4 debug", CORPUS_LINTS),
    ("demo plain", CORPUS_LINTS),
    ("demo debug", CORPUS_LINTS),
    ("config_a", CORPUS_LINTS),
    ("config_b", CORPUS_LINTS),
    ("lean", CORPUS_LINTS),
];

/// The reference tree, or `None` when it is absent and strict mode is off.
fn aeon_dir() -> Option<PathBuf> {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return None;
    }
    Some(aeon)
}

/// One shape's label and the warn tier its lowering produced.
type ShapeWarnings = (&'static str, Vec<native::BuildWarning>);

/// `(label, warnings)` for each of the SEVEN shipped shapes, lowered ONCE per test
/// binary — the gates below share one measurement rather than each paying for its
/// own lowering.
///
/// All seven are watched, not a canonical subset: the off-canonical shapes differ by
/// comptime defines that DO gate lint-bearing code (`lean` drops
/// `ErrorHandlerBlob`'s `[proc.undeclared-fallthrough]` and gains `ReleaseFault`'s
/// `[proc.sr-undeclared]`), so a shape-only lint is a real escape route.
fn corpus_warnings() -> Option<&'static [ShapeWarnings]> {
    static CACHE: OnceLock<Option<Vec<ShapeWarnings>>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let aeon = aeon_dir()?;
            // The `.emp` corpus embeds the seam-emitted sound blobs, so they must
            // exist before any shape lowers.
            native::ensure_generated(&aeon);
            Some(
                [
                    ("sonic4 plain", native::sonic4_profile(false)),
                    ("sonic4 debug", native::sonic4_profile(true)),
                    ("demo plain", native::demo_profile(false)),
                    ("demo debug", native::demo_profile(true)),
                    ("config_a", native::config_a_profile()),
                    ("config_b", native::config_b_profile()),
                    ("lean", native::lean_profile()),
                ]
                .into_iter()
                .map(|(label, profile)| {
                    let built = native::build_emp(&aeon, &profile)
                        .unwrap_or_else(|e| panic!("build_emp({label}): {e}"));
                    (label, built.warnings)
                })
                .collect(),
            )
        })
        .as_deref()
}

/// The firing id set per shape must equal [`WARN_ID_BASELINE`] exactly.
#[test]
fn warn_tier_lint_ids_match_the_frozen_baseline() {
    let Some(shapes) = corpus_warnings() else { return };

    for (label, warnings) in shapes {
        let got: BTreeSet<&str> = warnings.iter().map(|w| w.id.as_str()).collect();
        let want: BTreeSet<&str> = WARN_ID_BASELINE
            .iter()
            .find(|(l, _)| l == label)
            .unwrap_or_else(|| panic!("no baseline row for shape `{label}`"))
            .1
            .iter()
            .copied()
            .collect();

        let appeared: Vec<_> = got.difference(&want).collect();
        let vanished: Vec<_> = want.difference(&got).collect();
        assert!(
            appeared.is_empty() && vanished.is_empty(),
            "the warn-tier lint id set moved for `{label}`.\n  \
             NEWLY FIRING (a lint fires on the corpus and nobody decided that): {appeared:?}\n  \
             NO LONGER FIRING (a class was retired, or a lint lost its teeth): {vanished:?}\n  \
             Adjudicate each id and update WARN_ID_BASELINE in the same commit."
        );
    }
}

/// Every warn-tier diagnostic the corpus fires carries a `[area.name]` id.
///
/// NOT VACUOUS: the id is what makes the tally line legible and what
/// [`warn_tier_lint_ids_match_the_frozen_baseline`] keys on. A lint that ships
/// without one tallies into a single meaningless `unclassified` bucket and is
/// invisible to that gate, so it has to fail here instead.
#[test]
fn every_corpus_warning_carries_a_lint_id() {
    let Some(shapes) = corpus_warnings() else { return };

    for (label, warnings) in shapes {
        assert!(!warnings.is_empty(), "`{label}` reported no warnings — measure before trusting");
        let bare: Vec<&str> =
            warnings.iter().filter(|w| w.id.is_empty()).map(|w| w.message.as_str()).collect();
        assert!(
            bare.is_empty(),
            "`{label}`: {} warn-tier diagnostic(s) carry no `[area.name]` id, so they tally as \
             `unclassified` and the baseline gate cannot key on them: {bare:#?}",
            bare.len()
        );
    }
}

/// The build reports on code its reader can EDIT.
///
/// `build_emp` writes a synthetic entry module to drive the reachability BFS, whose
/// every line is a bare `use <module>` — the exact shape `[import.no-names]` warns
/// about. Reporting those would put unactionable rows against a file that does not
/// exist on disk.
///
/// NOT VACUOUS in either direction: the corpus hand-writes no bare `use` statement,
/// so an `[import.no-names]` firing can only come from the generated module; and the
/// lint's teeth against real code are proven in `module_resolution.rs`, so a green
/// here cannot mean the lint is dead.
#[test]
fn the_generated_entry_module_is_not_reported() {
    let Some(shapes) = corpus_warnings() else { return };

    for (label, warnings) in shapes {
        let generated: Vec<&str> = warnings
            .iter()
            .filter(|w| w.id == "import.no-names")
            .map(|w| w.message.as_str())
            .collect();
        assert!(
            generated.is_empty(),
            "`{label}`: the synthetic entry's own diagnostics reached the build report: \
             {generated:#?}"
        );
        // Every reported warning names a file the reader can open.
        let unlocated: Vec<&str> =
            warnings.iter().filter(|w| w.location.is_none()).map(|w| w.message.as_str()).collect();
        assert!(
            unlocated.is_empty(),
            "`{label}`: {} reported warning(s) carry no source location: {unlocated:#?}",
            unlocated.len()
        );
    }
}

/// THE SURFACE IS WIRED: `sigil build` prints the tally on stderr, and
/// `SIGIL_WARNINGS=off` silences it.
///
/// NOT VACUOUS, and nothing else covers this: every other gate here calls
/// `build_emp` and reads `EmpProgram::warnings` directly, so all of them stay green
/// if the `report_warnings` call in `run_build_native` is deleted. This drives the
/// real binary and reads its real stderr, which is the only thing that can tell the
/// difference between a collected warn tier and a REPORTED one.
///
/// One `--ram-report` invocation per view rather than a full ROM build: it runs the
/// same `report_warnings` under the same `WarningView`, and costs seconds instead of
/// minutes.
#[test]
fn the_build_binary_prints_the_tally_and_off_silences_it() {
    let Some(aeon) = aeon_dir() else { return };

    let run = |view: Option<&str>| -> String {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"));
        cmd.args(["build", "--aeon", aeon.to_str().unwrap(), "--native", "--ram-report"]);
        match view {
            Some(v) => cmd.env("SIGIL_WARNINGS", v),
            None => cmd.env_remove("SIGIL_WARNINGS"),
        };
        let out = cmd.output().expect("run sigil build");
        assert!(out.status.success(), "sigil build --ram-report failed: {out:?}");
        String::from_utf8_lossy(&out.stderr).into_owned()
    };

    // `--ram-report` fires no warn-tier diagnostic today, so the DEFAULT view must
    // print nothing at all — the "silence means zero, and only zero" contract, on
    // the real binary.
    let default = run(None);
    assert!(
        !default.contains("warning:"),
        "a warning-free report must print no warn-tier line, got: {default:?}"
    );
    assert_eq!(run(Some("off")), default, "`off` and a clean tier agree");

    // And the printer IS reachable from this path: a `--ram-report` over a tree with
    // no RAM-region module warns, and the tally names the class.
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("engine")).unwrap();
    std::fs::write(tmp.path().join("engine/ram.emp"), "module engine.ram\n").unwrap();
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--aeon", tmp.path().to_str().unwrap(), "--native", "--ram-report"]);
    let warned = String::from_utf8_lossy(&cmd.output().expect("run").stderr).into_owned();
    assert!(
        warned.contains("warning: ") && warned.contains("ram.no-region"),
        "the tally must reach stderr and name the firing class, got: {warned:?}"
    );
    assert!(
        warned.contains("SIGIL_WARNINGS=full to list"),
        "the summary view must point at the full view, got: {warned:?}"
    );

    // `off` on the SAME tree prints no warn-tier line — proof the view is consulted
    // rather than the tier being empty.
    let mut off = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"));
    off.args(["build", "--aeon", tmp.path().to_str().unwrap(), "--native", "--ram-report"])
        .env("SIGIL_WARNINGS", "off");
    let silenced = String::from_utf8_lossy(&off.output().expect("run").stderr).into_owned();
    assert!(
        !silenced.contains("warning: "),
        "`SIGIL_WARNINGS=off` must silence the tier, got: {silenced:?}"
    );
}
