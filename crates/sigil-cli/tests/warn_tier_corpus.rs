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
//!   - The counts are SHAPE-DEPENDENT and move on ordinary engine work — every
//!     `[proc.sr-undeclared]` firing is the `assert` desugar's own
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
    "proc.undeclared-fallthrough",
];

/// What a `DEBUG == 1` shape fires ON TOP of [`CORPUS_LINTS`].
///
/// `proc.sr-undeclared` is a shape-only class because `assert` is: the desugar
/// emits its own `move.w sr,-(sp)` … `move.w (sp)+,sr` pair and the lint charges
/// the compiler's code to the asserting proc, so the class appears exactly where
/// asserts compile. Every hand-written SR write in the corpus is declared, so with
/// the asserts elided it does not fire at all.
const DEBUG_ONLY_LINTS: &[&str] = &["proc.sr-undeclared"];

/// The firing set per build shape, spelled as [`CORPUS_LINTS`] plus whatever that
/// shape adds. Every SHIPPED shape has its own row, so a lint that fires in one
/// shape only cannot hide behind a shape nobody watches — and because the shared
/// ids live in ONE list, retiring a class corpus-wide stays the one-line diff it
/// should be while a shape-only class is visible as its own entry.
const WARN_ID_BASELINE: &[(&str, &[&str])] = &[
    ("sonic4 plain", &[]),
    ("sonic4 debug", DEBUG_ONLY_LINTS),
    ("demo plain", &[]),
    ("demo debug", DEBUG_ONLY_LINTS),
    ("config_a", DEBUG_ONLY_LINTS),
    ("config_b", &[]),
    ("lean", &[]),
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
/// All seven are watched, not a canonical subset: the shapes differ by comptime
/// defines that DO gate lint-bearing code, and the divergence reaches the ID SET
/// this gate pins — the three `DEBUG == 1` shapes are the only ones where
/// `[proc.sr-undeclared]` fires at all, so a shape-only lint is a real escape
/// route. (Shapes also diverge below the id set, which this gate deliberately does
/// not watch: `lean` fires one fewer `[proc.undeclared-fallthrough]` than the
/// canonical shapes because it drops `ErrorHandlerBlob`, and the class stays in
/// its row.)
fn corpus_warnings() -> Option<&'static [ShapeWarnings]> {
    static CACHE: OnceLock<Option<Vec<ShapeWarnings>>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let aeon = aeon_dir()?;
            // The `.emp` corpus embeds the seam-emitted sound blobs, so they must
            // exist before any shape lowers.
            native::ensure_generated(&aeon);
            Some(
                native::shipped_shapes()
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
        let extra = WARN_ID_BASELINE
            .iter()
            .find(|(l, _)| l == label)
            .unwrap_or_else(|| panic!("no baseline row for shape `{label}`"))
            .1;
        let want: BTreeSet<&str> = CORPUS_LINTS.iter().chain(extra).copied().collect();

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

/// EVERY surviving `[proc.sr-undeclared]` firing is the `assert` desugar's own
/// save/restore pair — the property [`DEBUG_ONLY_LINTS`] asserts in prose, made
/// executable.
///
/// This is the gate that keeps the id set honest for a class that is now 100%
/// compiler-generated. Listing `proc.sr-undeclared` in the `DEBUG == 1` rows
/// admits the class, and an id SET cannot tell 43 firings from 44 — so without
/// this, a NEW hand-written undeclared SR write in any debug-gated proc would
/// join the crowd and never be seen again.
///
/// A COUNT would also catch it, and was rejected for this id on measured grounds
/// (see the header): adding one debug assert to one proc moves the number, so the
/// baseline would churn on work that has nothing to do with it and get
/// rubber-stamped. This asserts the property instead, which is what the project
/// actually controls — a new assert is still an assert and changes nothing here,
/// while the first hand-written SR write to go undeclared fails immediately with
/// the site named. Its kill condition is the desugar's own: when sigil stops
/// linting its own emitted code this whole class goes to zero and the gate below
/// starts asserting emptiness on its own.
#[test]
fn every_surviving_sr_firing_is_the_assert_desugar() {
    let Some(shapes) = corpus_warnings() else { return };
    let Some(aeon) = aeon_dir() else { return };

    let mut seen = 0usize;
    for (label, warnings) in shapes {
        for w in warnings.iter().filter(|w| w.id == "proc.sr-undeclared") {
            let loc = w.location.as_deref().unwrap_or_else(|| {
                panic!("`{label}`: an sr-undeclared firing with no source location: {}", w.message)
            });
            // `path:line:col`, the path relative to the scanned aeon root.
            let (path, rest) = loc.rsplit_once(':').and_then(|(p, _col)| p.rsplit_once(':')).unwrap_or_else(
                || panic!("`{label}`: unparseable location `{loc}`"),
            );
            let line: usize = rest.parse().unwrap_or_else(|_| panic!("`{label}`: `{loc}`"));
            let text = std::fs::read_to_string(aeon.join(path))
                .unwrap_or_else(|e| panic!("`{label}`: cannot read `{path}`: {e}"));
            let src = text.lines().nth(line - 1).unwrap_or_else(|| panic!("`{label}`: `{loc}`"));
            assert!(
                src.contains("assert."),
                "`{label}`: a HAND-WRITTEN SR write is undeclared at {loc}:\n  {}\n\
                 Declare `clobbers(sr)`, or `preserves(sr)` if the body save/restores it — \
                 every other firing in this class is the `assert` desugar's own pair.",
                src.trim()
            );
            seen += 1;
        }
    }
    // Cannot pass by measuring nothing: the DEBUG shapes fire this class today.
    assert!(seen > 0, "no sr-undeclared firing was examined — measure before trusting");
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
/// One `--report ram` invocation per view rather than a full ROM build: it runs the
/// same `report_warnings` under the same `WarningView`, and costs seconds instead of
/// minutes. Both report kinds share one prologue, so this covers `contracts` too.
#[test]
fn the_build_binary_prints_the_tally_and_off_silences_it() {
    let Some(aeon) = aeon_dir() else { return };

    let run = |view: Option<&str>| -> String {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"));
        cmd.args(["build", "--aeon", aeon.to_str().unwrap(), "--native", "--report", "ram"]);
        match view {
            Some(v) => cmd.env("SIGIL_WARNINGS", v),
            None => cmd.env_remove("SIGIL_WARNINGS"),
        };
        let out = cmd.output().expect("run sigil build");
        assert!(out.status.success(), "sigil build --report ram failed: {out:?}");
        String::from_utf8_lossy(&out.stderr).into_owned()
    };

    // A report shows the SAME warn tier the build shows over the same tree — the
    // manifest scan's `[module.path-mismatch]` family included, which no later stage
    // re-reports. A report that renders a cleaner tree than the build is a report
    // nobody can trust, so the DEFAULT view carries the tally here too.
    let default = run(None);
    assert!(
        default.contains("warning: ") && default.contains("module.path-mismatch"),
        "a report must show the manifest scan's own warn tier, got: {default:?}"
    );
    assert!(
        !run(Some("off")).contains("warning: "),
        "`SIGIL_WARNINGS=off` must silence the tier on the real corpus too"
    );

    // And the printer IS reachable from this path: a `--report ram` over a tree with
    // no RAM-region module warns, and the tally names the class.
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("engine")).unwrap();
    std::fs::write(tmp.path().join("engine/ram.emp"), "module engine.ram\n").unwrap();
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--aeon", tmp.path().to_str().unwrap(), "--native", "--report", "ram"]);
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
    off.args(["build", "--aeon", tmp.path().to_str().unwrap(), "--native", "--report", "ram"])
        .env("SIGIL_WARNINGS", "off");
    let silenced = String::from_utf8_lossy(&off.output().expect("run").stderr).into_owned();
    assert!(
        !silenced.contains("warning: "),
        "`SIGIL_WARNINGS=off` must silence the tier, got: {silenced:?}"
    );
}
