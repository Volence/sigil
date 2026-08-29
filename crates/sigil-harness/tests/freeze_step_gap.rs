//! THE THREE JOINTS BETWEEN THE FREEZE'S STEPS, MEASURED UNDER A KILL.
//!
//! `golden/atomic_freeze.sh` closed the half-written blob: a capture killed part-way can
//! no longer leave a truncated golden, and `tests/golden_freeze_atomicity.rs` gates that.
//! This file is about what sits BESIDE it — the three gaps between `--freeze`'s four
//! steps (capture → sizes → pins → ledger), which the staged capture does not touch.
//!
//! THE WRECKAGE IS DIFFERENT AT EACH JOINT, so there is a gate per joint rather than one
//! over the set: a kill after the capture leaves fresh blobs beside a stale size table, a
//! stale `pins.rs` and a stale ledger; after the sizes, only the pins and the ledger are
//! stale; after the pins, everything but the ledger is current. A gate over one of those
//! establishes nothing about the other two.
//!
//! THE CONTROL IS THE POINT. [`Mode::Unjournaled`] reproduces the sequence as it ran
//! before this file's mechanism existed, and the gates assert the hazard IS observed
//! through it — the mixed fresh/stale artifact set, and, decisively, that NOTHING on disk
//! records that a run was interrupted. Without the control the journaled assertions would
//! be claims that have never been shown capable of failing.
//!
//! WHAT IS ASSERTED OF THE JOURNAL, and it is narrower than "atomic":
//!
//!   1. The mixed artifact set is UNCHANGED — the journal does not prevent it, and a gate
//!      claiming otherwise would be asserting something false.
//!   2. The extent of the interrupted run is recorded, and the record AGREES WITH THE
//!      FILESYSTEM: what the journal calls fresh is exactly what holds new bytes.
//!   3. The report names every fresh path, every stale path, the interrupted command
//!      verbatim, and the exact `git checkout` that returns the tree to committed state.
//!   4. An unreadable journal is COULD NOT MEASURE, never "the run completed".
//!   5. A completed run leaves NO journal, so nothing here can fire on a correct freeze.
//!
//! EXPECTATIONS COME FROM THE DISK, NOT FROM THE STEP TABLE. `fresh()`/`stale()` are
//! checked against which artifacts actually hold new bytes after the kill, so a step table
//! that mis-attributes an artifact fails these gates instead of defining them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sigil_harness::freeze_journal::{self, STEPS};

/// ONE TEST PER JOINT, not one over the set. The three gaps leave different wreckage — a
/// kill after the capture strands three artifact classes, a kill after the pins strands
/// one — so a single looping gate would stop at the first failure and report nothing
/// about the other two joints, which is the shape of a gate that covers less than its
/// name claims.
macro_rules! per_joint {
    ($body:ident: $one:ident, $two:ident, $three:ident) => {
        #[test]
        fn $one() {
            $body(1);
        }
        #[test]
        fn $two() {
            $body(2);
        }
        #[test]
        fn $three() {
            $body(3);
        }
    };
}

/// Which sequence a fixture runs. `Unjournaled` is the step sequence as it stood before
/// the journal — run the steps, record nothing.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Unjournaled,
    Journaled,
}

/// A scratch harness root carrying one artifact per step, seeded with the previous
/// freeze's output.
struct Tree {
    root: PathBuf,
    _dir: tempfile::TempDir,
}

fn old_bytes(rel: &str) -> String {
    format!("OLD {rel}\n")
}

fn new_bytes(rel: &str) -> String {
    format!("NEW {rel}\n")
}

impl Tree {
    fn seeded() -> Tree {
        let dir = tempfile::tempdir().expect("scratch dir");
        let root = dir.path().to_path_buf();
        // The two markers that make this a harness root, so the fixture is the shape the
        // real tools resolve rather than a bare directory.
        write(&root.join("repin.toml"), "# scratch\n");
        for spec in STEPS.iter() {
            for rel in spec.produces {
                write(&root.join(rel), &old_bytes(rel));
            }
        }
        Tree { root, _dir: dir }
    }

    /// Run the first `stop_after` steps and then stop dead, as a kill would.
    fn interrupted(mode: Mode, stop_after: usize) -> Tree {
        let t = Tree::seeded();
        let mut journal = match mode {
            Mode::Journaled => Some(
                freeze_journal::open(&t.root, "0".repeat(40).as_str(), &t.command())
                    .expect("open the journal"),
            ),
            Mode::Unjournaled => None,
        };
        for spec in STEPS.iter().take(stop_after) {
            for rel in spec.produces {
                write(&t.root.join(rel), &new_bytes(rel));
            }
            if let Some(j) = journal.as_mut() {
                j.record(spec.key).expect("record a completed step");
            }
        }
        // The kill: the process ends here. A journal that was opened is NOT closed, which
        // is exactly what a dropped `Journal` leaves on disk.
        drop(journal);
        t
    }

    fn command(&self) -> String {
        format!("SIGIL_HARNESS_ROOT={} refreeze --freeze a-parcel --ab ab/2026", self.root.display())
    }

    /// Every artifact that currently holds this run's bytes.
    fn holding_new_bytes(&self) -> BTreeSet<&'static str> {
        self.partition_by(|rel, body| body == new_bytes(rel))
    }

    /// Every artifact still holding the PREVIOUS freeze's bytes.
    fn holding_old_bytes(&self) -> BTreeSet<&'static str> {
        self.partition_by(|rel, body| body == old_bytes(rel))
    }

    fn partition_by(
        &self,
        want: impl Fn(&str, &str) -> bool,
    ) -> BTreeSet<&'static str> {
        STEPS
            .iter()
            .flat_map(|s| s.produces.iter().copied())
            .filter(|rel| {
                let body = std::fs::read_to_string(self.root.join(rel)).unwrap_or_default();
                want(rel, &body)
            })
            .collect()
    }

    /// Every file under the root, relative — used to ask whether ANYTHING records the
    /// interruption.
    fn files(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        walk(&self.root, &self.root, &mut out);
        out
    }
}

fn walk(base: &Path, dir: &Path, out: &mut BTreeSet<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(base, &p, out);
        } else {
            out.insert(p.strip_prefix(base).unwrap().display().to_string());
        }
    }
}

fn write(path: &Path, body: &str) {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).expect("mkdir");
    }
    std::fs::write(path, body).expect("write");
}

/// The artifacts a completed run's step table attributes to the first `n` steps — used
/// only to name a gate's subject in its message, never as the expectation.
fn produced_by_first(n: usize) -> BTreeSet<&'static str> {
    STEPS.iter().take(n).flat_map(|s| s.produces.iter().copied()).collect()
}

// ── THE CONTROL: what the gap leaves when nothing records it ────────────────

// RED-FIRST, one gate per joint. The sequence without a journal leaves a mixed artifact
// set AND leaves no trace that it was interrupted: the tree after a kill is the tree
// after a completed partial run, and there is nothing to read that says otherwise.
per_joint!(unjournaled_joint:
    the_unjournaled_capture_to_sizes_joint_says_nothing_about_itself,
    the_unjournaled_sizes_to_pins_joint_says_nothing_about_itself,
    the_unjournaled_pins_to_ledger_joint_says_nothing_about_itself
);
fn unjournaled_joint(stop_after: usize) {
    let seeded_files = Tree::seeded().files();
    {
        let t = Tree::interrupted(Mode::Unjournaled, stop_after);

        // The wreckage: fresh artifacts beside stale ones, all of them well-formed.
        let fresh = t.holding_new_bytes();
        let stale = t.holding_old_bytes();
        assert_eq!(
            fresh,
            produced_by_first(stop_after),
            "gap {stop_after}: the completed steps' artifacts must hold the new bytes"
        );
        assert!(
            !fresh.is_empty() && !stale.is_empty(),
            "gap {stop_after}: a joint is only a joint if BOTH halves are non-empty; \
             fresh={fresh:?} stale={stale:?}"
        );
        assert_eq!(
            fresh.len() + stale.len(),
            STEPS.iter().map(|s| s.produces.len()).sum::<usize>(),
            "gap {stop_after}: every artifact must be one or the other — a third state \
             would mean the fixture, not the mechanism, produced the mixture"
        );

        // THE FINDING. Not one byte anywhere says a run was interrupted: the file set is
        // the seeded one, so a reader has nothing to consult but the artifacts, and every
        // artifact parses.
        assert_eq!(
            t.files(),
            seeded_files,
            "gap {stop_after}: the killed run left no residue at all, which is what makes \
             the state silent"
        );
        assert!(
            freeze_journal::read(&t.root).is_none(),
            "gap {stop_after}: nothing to read means nothing can be asked"
        );
    }
}

/// The sharpest form of the control, and the one the CRC gates cannot reach: when the
/// parcel moves no bytes, an unjournaled kill at the LAST joint leaves a tree
/// byte-identical to one where the freeze never ran. `refreeze --check` compares the
/// ledger against the blobs, and neither moved — so the entry a `--supersede-tip` freeze
/// was going to append is simply absent, with nothing anywhere to say so.
#[test]
fn a_byte_neutral_kill_at_the_last_joint_is_invisible_without_a_journal() {
    let untouched = Tree::seeded();
    let t = Tree::seeded();
    // A byte-neutral freeze: every step re-derives what was already there.
    for spec in STEPS.iter().take(3) {
        for rel in spec.produces {
            write(&t.root.join(rel), &old_bytes(rel));
        }
    }
    assert_eq!(t.files(), untouched.files(), "no file appeared");
    for rel in STEPS.iter().flat_map(|s| s.produces.iter()) {
        assert_eq!(
            std::fs::read_to_string(t.root.join(rel)).unwrap(),
            std::fs::read_to_string(untouched.root.join(rel)).unwrap(),
            "{rel} differs — then this is not the byte-neutral case"
        );
    }
    assert!(
        freeze_journal::read(&t.root).is_none(),
        "and nothing records the run that did not finish"
    );

    // Journaled, the same interruption is nameable.
    let j = Tree::interrupted(Mode::Journaled, 3);
    let l = freeze_journal::read(&j.root).expect("a journal survives the kill");
    assert!(!l.completed(), "three of four steps is not a completed run");
    assert!(
        l.report().contains("DID NOT COMPLETE"),
        "the report must say so in the first line: {}",
        l.report()
    );
}

// ── THE JOURNAL: the same wreckage, named ───────────────────────────────────

// One gate per joint. The journal's account of what is fresh and what is stale is
// checked against the FILESYSTEM, not against the step table it is derived from.
per_joint!(joint_is_recorded:
    the_capture_to_sizes_joint_is_recorded_and_agrees_with_the_disk,
    the_sizes_to_pins_joint_is_recorded_and_agrees_with_the_disk,
    the_pins_to_ledger_joint_is_recorded_and_agrees_with_the_disk
);
fn joint_is_recorded(stop_after: usize) {
    {
        let t = Tree::interrupted(Mode::Journaled, stop_after);
        let l = freeze_journal::read(&t.root).expect("a journal survives a kill");

        assert!(l.measurable(), "gap {stop_after}: the journal this run wrote must parse");
        assert!(
            !l.completed(),
            "gap {stop_after}: {stop_after} of {} steps is not a completed run",
            STEPS.len()
        );
        assert_eq!(
            l.recorded(),
            &STEPS.iter().take(stop_after).map(|s| s.key.to_string()).collect::<Vec<_>>()[..],
            "gap {stop_after}: the recorded steps are the ones that ran, in order"
        );

        // The cross-check that makes this more than a restatement.
        let fresh: BTreeSet<&str> = l.fresh().into_iter().collect();
        let stale: BTreeSet<&str> = l.stale().into_iter().collect();
        assert_eq!(
            fresh,
            t.holding_new_bytes(),
            "gap {stop_after}: the journal calls fresh exactly what holds new bytes"
        );
        assert_eq!(
            stale,
            t.holding_old_bytes(),
            "gap {stop_after}: the journal calls stale exactly what holds the old bytes"
        );
        assert!(
            fresh.is_disjoint(&stale),
            "gap {stop_after}: an artifact cannot be both"
        );
    }
}

// The recovery must be executable, not descriptive. Every fresh path, every stale path,
// the interrupted command verbatim, and a `git checkout` naming exactly the paths that
// moved — nothing wider, so the instruction cannot revert work it did not cause.
per_joint!(report_is_exact:
    the_capture_to_sizes_report_names_the_paths_and_the_way_back,
    the_sizes_to_pins_report_names_the_paths_and_the_way_back,
    the_pins_to_ledger_report_names_the_paths_and_the_way_back
);
fn report_is_exact(stop_after: usize) {
    {
        let t = Tree::interrupted(Mode::Journaled, stop_after);
        let l = freeze_journal::read(&t.root).expect("journal");
        let r = l.report();

        for rel in t.holding_new_bytes() {
            let abs = t.root.join(rel).display().to_string();
            assert!(r.contains(&abs), "gap {stop_after}: the fresh path {abs} is not in:\n{r}");
        }
        for rel in t.holding_old_bytes() {
            let abs = t.root.join(rel).display().to_string();
            assert!(r.contains(&abs), "gap {stop_after}: the stale path {abs} is not in:\n{r}");
        }
        assert!(
            r.contains(&t.command()),
            "gap {stop_after}: the interrupted command must appear verbatim, not paraphrased:\n{r}"
        );
        assert!(r.contains("FRESH") && r.contains("STALE"), "both halves are labelled:\n{r}");

        // The discard path names the checkout scope and the journal, and nothing beyond
        // the artifacts that actually moved.
        let checkout = format!("git -C {} checkout --", t.root.display());
        assert!(r.contains(&checkout), "gap {stop_after}: no exact checkout command:\n{r}");
        let line = r
            .lines()
            .find(|l| l.contains(&checkout))
            .expect("the checkout line was just asserted present");
        for rel in t.holding_old_bytes() {
            assert!(
                !line.contains(rel),
                "gap {stop_after}: the checkout must not revert {rel}, which never moved: {line}"
            );
        }
        assert!(
            r.contains(&format!("rm {}", freeze_journal::path(&t.root).display())),
            "gap {stop_after}: the journal's own removal must be spelled out:\n{r}"
        );
    }
}

/// A step that FAILS leaves the same state as a kill, and must be reported the same way —
/// the tool is still alive to say it, so the report goes out while someone is watching.
#[test]
fn an_ordinary_step_failure_leaves_the_journal_standing() {
    let t = Tree::seeded();
    let mut j =
        freeze_journal::open(&t.root, &"0".repeat(40), &t.command()).expect("open");
    for rel in STEPS[0].produces {
        write(&t.root.join(rel), &new_bytes(rel));
    }
    j.record(STEPS[0].key).expect("record");
    let report = j.state_report();
    // The step-2 script exits nonzero; the run returns an error and the journal is dropped
    // without closing.
    drop(j);
    assert!(report.contains("DID NOT COMPLETE"), "{report}");
    assert!(
        freeze_journal::read(&t.root).is_some(),
        "a failed run's journal must survive for the next reader, exactly as a killed one does"
    );
}

// ── The two states that must NOT read as a fault ────────────────────────────

/// A COMPLETED run leaves nothing. This is the gate that keeps the mechanism from being a
/// new way to refuse a correct freeze: with every step recorded, the journal is removed
/// and the tree is indistinguishable from one that never ran a freeze.
#[test]
fn a_completed_run_removes_its_journal_and_the_tree_is_clean() {
    let before = Tree::seeded().files();
    let t = Tree::seeded();
    let mut j = freeze_journal::open(&t.root, &"0".repeat(40), &t.command()).expect("open");
    for spec in STEPS.iter() {
        for rel in spec.produces {
            write(&t.root.join(rel), &new_bytes(rel));
        }
        j.record(spec.key).expect("record");
    }
    j.close().expect("a run that recorded every step must close");
    assert!(freeze_journal::read(&t.root).is_none(), "no journal after a completed run");
    assert_eq!(t.files(), before, "and no residue of any kind");
}

/// A journal recording every step but not removed — a kill in the instant between the
/// last record and the removal. Nothing is inconsistent, so it is a note, not a fault.
#[test]
fn an_unremoved_journal_from_a_finished_run_is_a_note_and_not_a_fault() {
    let t = Tree::seeded();
    let mut j = freeze_journal::open(&t.root, &"0".repeat(40), &t.command()).expect("open");
    for spec in STEPS.iter() {
        j.record(spec.key).expect("record");
    }
    std::mem::forget(j);
    let l = freeze_journal::read(&t.root).expect("journal");
    assert!(l.completed(), "every step recorded is a finished run");
    assert!(l.stale().is_empty(), "nothing is stale after a finished run");
    assert!(
        l.completed_note().contains("current"),
        "the note must say the artifacts agree: {}",
        l.completed_note()
    );
}

// ── Loud on unmeasurable ────────────────────────────────────────────────────

/// A journal that cannot be understood must not read as "the run completed". It reports
/// COULD NOT MEASURE, and every artifact goes into the stale column, because none of them
/// can be shown current.
#[test]
fn an_unreadable_journal_could_not_measure_and_is_never_a_pass() {
    let every: BTreeSet<&str> =
        STEPS.iter().flat_map(|s| s.produces.iter().copied()).collect();
    for (what, body) in [
        ("an unknown format version", "version 99\ndone capture\ndone sizes\ndone pins\ndone ledger\n"),
        ("no version line at all", "done capture\ndone sizes\ndone pins\ndone ledger\n"),
        ("nothing but noise", "\u{0}\u{0}garbage\n"),
    ] {
        let t = Tree::seeded();
        write(&freeze_journal::path(&t.root), body);
        let l = freeze_journal::read(&t.root).expect("the file is there");
        assert!(!l.measurable(), "{what}: must not claim to have measured anything");
        assert!(
            !l.completed(),
            "{what}: four `done` lines under an unreadable header must NOT read as completed"
        );
        assert!(l.fresh().is_empty(), "{what}: nothing may be claimed fresh");
        assert_eq!(
            l.stale().into_iter().collect::<BTreeSet<_>>(),
            every,
            "{what}: every artifact is possibly stale — the conservative direction"
        );
        let r = l.report();
        assert!(r.contains("COULD NOT MEASURE"), "{what}: the house idiom is missing:\n{r}");
    }
}

/// A journal that cannot be CREATED refuses the freeze. Running unjournaled would spend
/// the whole capture producing precisely the state that cannot then be named.
#[test]
fn a_journal_that_cannot_be_created_refuses_rather_than_running_blind() {
    let t = Tree::seeded();
    // A directory where the journal file goes: the write fails, nothing else does.
    std::fs::create_dir_all(freeze_journal::path(&t.root)).expect("occupy the path");
    let e = match freeze_journal::open(&t.root, &"0".repeat(40), &t.command()) {
        Err(e) => e,
        Ok(_) => panic!("an unwritable journal must refuse"),
    };
    assert!(e.contains("COULD NOT MEASURE"), "{e}");
    assert!(e.contains("unjournaled"), "the refusal must say what it is refusing to do: {e}");
}

// A journal cannot be closed on a partial run. Closing is the ONE statement that a freeze
// completed, so the only thing that may make it is a run that recorded every step.
per_joint!(cannot_close_partial:
    a_run_stopped_at_the_capture_to_sizes_joint_cannot_close_its_journal,
    a_run_stopped_at_the_sizes_to_pins_joint_cannot_close_its_journal,
    a_run_stopped_at_the_pins_to_ledger_joint_cannot_close_its_journal
);
fn cannot_close_partial(stop_after: usize) {
    {
        let t = Tree::seeded();
        let mut j = freeze_journal::open(&t.root, &"0".repeat(40), &t.command()).expect("open");
        for spec in STEPS.iter().take(stop_after) {
            j.record(spec.key).expect("record");
        }
        let e = j.close().expect_err("a partial run must not be closable");
        for spec in STEPS.iter().skip(stop_after) {
            assert!(e.contains(spec.key), "gap {stop_after}: the refusal must name `{}`: {e}", spec.key);
        }
        assert!(
            freeze_journal::read(&t.root).is_some(),
            "gap {stop_after}: and the refusal must leave the journal where it is"
        );
    }
}

// ── The real tree ───────────────────────────────────────────────────────────

/// THIS checkout carries no interrupted freeze. The gate that reaches the unattended
/// overnight lane: a `--freeze` killed between steps leaves a journal, the suite goes red
/// here, and the interrupted state cannot be committed green. It can only fire after a
/// real interruption — the journal is git-ignored, so a fresh clone never has one.
#[test]
fn no_interrupted_freeze_is_outstanding_in_this_tree() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(l) = freeze_journal::read(&root) {
        if l.completed() {
            eprintln!("{}", l.completed_note());
        } else {
            panic!("{}", l.report());
        }
    }
}

/// The journal must be ignored by git, or a killed freeze leaves an untracked file in a
/// shared checkout for a later `git add` to sweep into history — the same hazard the
/// capture's staging area is ignored for.
#[test]
fn the_journal_is_git_ignored() {
    let harness = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = harness.parent().and_then(|p| p.parent()).expect("the repository toplevel");
    let ignore = std::fs::read_to_string(repo.join(".gitignore")).expect("read .gitignore");
    let rel = format!(
        "crates/sigil-harness/{}",
        freeze_journal::JOURNAL_SUBPATH
    );
    assert!(
        ignore.contains(&rel),
        ".gitignore must name {rel}; it currently says:\n{ignore}"
    );
}
