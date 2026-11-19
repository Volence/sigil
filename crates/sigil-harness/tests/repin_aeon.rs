//! repin_aeon — `refreeze --freeze NAME --ab REF --repin-aeon`, the narrow mode that moves
//! the chain's aeon pin to a LATER revision whose changes moved no ROM bytes.
//!
//! The strict pairing gate (`provenance_chain::aeon_dir_matches_the_provenance_tip`) is red
//! whenever `AEON_DIR`'s HEAD is not the tip's `aeon_rev`. A byte-neutral aeon change moves
//! that HEAD, but a plain re-freeze lands on the fixpoint and appends nothing, and
//! `--supersede-tip` only abandons a RED tip. `--repin-aeon` appends an ordinary entry whose
//! goldens equal its predecessor's and whose `aeon_rev` is the new revision — and only when
//! every precondition holds. This file gates each precondition by its own refusal constant,
//! the happy path, and that the supersede path is unchanged:
//!
//!   1. the rebuilt goldens hit the fixpoint            -> `REPIN_MOVED_BYTES`
//!   2. the new revision differs from the tip's and     -> `REPIN_SAME_AEON_REV`,
//!      descends from it                                   `REPIN_NOT_A_DESCENDANT`
//!   3. it is reachable from aeon `origin/master`       -> `REPIN_NOT_ON_AEON_MASTER`
//!   4. the tip is green and attested (`append_gate`)   -> `REPIN_OF_AN_UNATTESTED_TIP`
//!   5. not together with `--supersede-tip`             -> `REPIN_WITH_SUPERSEDE`
//!
//! These drive [`provenance::freeze_into_with_repin`] — the real ledger half the binary
//! calls — against a scratch `golden/` with throwaway blobs, and answer every git question
//! from a constructed history (a [`RevOracle`] written out by hand). No real aeon, no real
//! chain, no rebuild.
//!
//! Runner: `cargo test -p sigil-harness --test repin_aeon` (and the plain
//! `cargo test -p sigil-harness` set — no aeon tree, no sigil build, no strict gate).

use sigil_harness::provenance::{
    self, Applied, Chain, FreezeAction, StrictRun, Target, ASL_WITNESS, OUTCOME_FAILED,
    OUTCOME_PASSED, REPIN_MOVED_BYTES, REPIN_NOT_A_DESCENDANT, REPIN_NOT_ON_AEON_MASTER,
    REPIN_OF_AN_UNATTESTED_TIP, REPIN_SAME_AEON_REV, REPIN_WITHOUT_EVIDENCE, REPIN_WITH_SUPERSEDE,
};
use sigil_harness::rev_reachability::{
    audit, Field, RemoteTip, Repo, RevOracle, RevState, UnavailableRepo,
};
use std::collections::BTreeMap;
use std::path::Path;

const SIGIL_REV: &str = "e671a5b6383f82a20683ca838035e655b2c803c9";

/// A distinct, well-formed 40-char SHA per letter, constructed rather than copied, so no
/// assertion is pinned to a revision that exists anywhere.
fn sha(tag: char) -> String {
    std::iter::repeat_n(tag, 40).collect()
}

/// The tip's revision, the re-pin target that descends from it, the remote branch tip
/// above both, a commit on a side branch, and one older than the tip.
fn old() -> String {
    sha('a')
}
fn new() -> String {
    sha('b')
}
fn master() -> String {
    sha('c')
}
fn side() -> String {
    sha('d')
}
fn older() -> String {
    sha('e')
}

/// A history written out by hand: the remote tip, the commits this clone holds, and the
/// transitive ancestor set of each. Same shape as `tests/rev_reachability.rs`'s.
struct FakeRepo {
    tip: RemoteTip,
    present: Vec<String>,
    ancestors: BTreeMap<String, Vec<String>>,
}

impl RevOracle for FakeRepo {
    fn remote_tip(&self) -> Result<RemoteTip, String> {
        Ok(self.tip.clone())
    }
    fn has_commit(&self, rev: &str) -> Result<bool, String> {
        Ok(self.present.iter().any(|p| p == rev))
    }
    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, String> {
        if ancestor == descendant {
            return Ok(true);
        }
        Ok(self.ancestors.get(descendant).is_some_and(|a| a.iter().any(|x| x == ancestor)))
    }
}

/// `older -> old -> new -> master` on origin/master, and `side` branching off `older`
/// and merged into master — so `side` is REACHABLE but does not descend from `old`.
fn history(remote_tip: &str) -> FakeRepo {
    let mut ancestors = BTreeMap::new();
    ancestors.insert(old(), vec![older()]);
    ancestors.insert(new(), vec![old(), older()]);
    ancestors.insert(side(), vec![older()]);
    ancestors.insert(master(), vec![new(), old(), older(), side()]);
    FakeRepo {
        tip: RemoteTip { label: "origin/master".into(), sha: remote_tip.to_string() },
        present: vec![older(), old(), new(), master(), side()],
        ancestors,
    }
}

fn golden_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("alpha".to_string(), "alpha.bin".to_string()),
        ("beta".to_string(), "beta.bin".to_string()),
    ])
}

/// Throwaway blobs, with the target set DERIVED by the same `recompute_targets` a real
/// freeze uses. `fill` is what makes one generation differ from the next.
fn write_blobs(golden: &Path, fill: u8) -> BTreeMap<String, Target> {
    let mut ends = BTreeMap::new();
    for (i, (key, file)) in golden_map().iter().enumerate() {
        let len = 512 + i * 64;
        let bytes: Vec<u8> = (0..len).map(|n| fill.wrapping_add(n as u8)).collect();
        std::fs::write(golden.join(file), &bytes).expect("write scratch blob");
        ends.insert(key.clone(), len - 16);
    }
    provenance::recompute_targets(golden, &golden_map(), &ends).expect("recompute scratch targets")
}

fn strict_run(outcome: &str, aeon_rev: &str, targets: &BTreeMap<String, Target>) -> StrictRun {
    let failed = usize::from(outcome == OUTCOME_FAILED);
    StrictRun {
        outcome: outcome.to_string(),
        sigil_rev: SIGIL_REV.to_string(),
        aeon_rev: aeon_rev.to_string(),
        strict_bodies: 29,
        suites: 349,
        passed: 4037,
        failed,
        ignored: 4,
        skips: 0,
        ran_at: "unix:1787834118".to_string(),
        failing: if failed == 1 { vec!["a_test_that_was_red".to_string()] } else { Vec::new() },
        expected_tests: Vec::new(),
        goldens: targets
            .iter()
            .map(|(k, t)| (k.clone(), format!("{}/{}", t.full_crc, t.full_size)))
            .collect(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tip {
    Green,
    Red,
    UnattestedArmed,
    UnattestedRatchet,
}

/// A scratch `golden/` whose blobs and chain agree; both entries name `old()`.
fn fixture(tip: Tip) -> (tempfile::TempDir, BTreeMap<String, Target>) {
    let dir = tempfile::tempdir().expect("scratch golden dir");
    let golden = dir.path();
    let targets = write_blobs(golden, 0x10);
    let rev = old();
    let mut src = provenance::render_entry("root", ASL_WITNESS, &rev, "the scratch root", &targets);
    if tip == Tip::UnattestedArmed {
        src.push_str(&provenance::render_strict(&strict_run(OUTCOME_PASSED, &rev, &targets)));
    }
    src.push_str(&provenance::render_entry("tip", "ab/scratch", &rev, "the scratch tip", &targets));
    match tip {
        Tip::Green => src.push_str(&provenance::render_strict(&strict_run(OUTCOME_PASSED, &rev, &targets))),
        Tip::Red => src.push_str(&provenance::render_strict(&strict_run(OUTCOME_FAILED, &rev, &targets))),
        Tip::UnattestedArmed | Tip::UnattestedRatchet => {}
    }
    std::fs::write(golden.join("provenance.toml"), &src).expect("write scratch chain");

    // POSITIVE CONTROL: the fixture holds the state the test names, measured by the
    // module's own predicates.
    let chain = load(golden);
    assert!(
        provenance::check(golden, &chain).is_empty(),
        "fixture chain must be clean: {:?}",
        provenance::check(golden, &chain)
    );
    let t = chain.tip().expect("fixture tip");
    match tip {
        Tip::Green => assert!(t.is_attested(), "fixture asked for a GREEN tip"),
        Tip::Red => assert!(t.is_red(), "fixture asked for a RED tip"),
        _ => assert!(!t.is_attested() && !t.is_red(), "fixture asked for an unattested tip"),
    }
    (dir, targets)
}

fn load(golden: &Path) -> Chain {
    provenance::parse(&std::fs::read_to_string(golden.join("provenance.toml")).expect("read"))
        .expect("parse scratch chain")
}

fn bytes(golden: &Path) -> Vec<u8> {
    std::fs::read(golden.join("provenance.toml")).expect("read scratch chain bytes")
}

/// Run a re-pin to `to` against `golden` and expect a refusal that leaves the file alone.
fn refused(golden: &Path, fresh: &BTreeMap<String, Target>, to: &str, oracle: &dyn RevOracle, supersede: Option<&str>) -> String {
    let before = bytes(golden);
    let err = provenance::freeze_into_with_repin(
        golden, "the-repin", "ab/scratch repin", to, "", fresh, supersede, Some(oracle),
    )
    .expect_err("this re-pin must be refused");
    assert_eq!(before, bytes(golden), "a refusal must not touch the ledger");
    err
}

fn assert_refused_by(err: &str, constant: &str, which: &str) {
    assert!(err.starts_with(constant), "{which}: expected the refusal to lead with its constant\n  want: {constant}\n  got:  {err}");
}

// ---------------------------------------------------------------------------
// THE PRECONDITIONS, each refused by name.
// ---------------------------------------------------------------------------

/// (1) A byte move with `--repin-aeon` is an ordinary freeze, not a re-pin.
#[test]
fn p1_a_repin_whose_goldens_moved_is_refused() {
    let (dir, _) = fixture(Tip::Green);
    let golden = dir.path();
    let moved = write_blobs(golden, 0x77);
    let err = refused(golden, &moved, &new(), &history(&master()), None);
    assert_refused_by(&err, REPIN_MOVED_BYTES, "precondition 1");
}

/// (2a) The new revision IS the tip's revision: nothing to re-pin.
#[test]
fn p2_a_repin_to_the_tips_own_revision_is_refused() {
    let (dir, fresh) = fixture(Tip::Green);
    let err = refused(dir.path(), &fresh, &old(), &history(&master()), None);
    assert_refused_by(&err, REPIN_SAME_AEON_REV, "precondition 2 (same)");
}

/// (2b) Not a descendant: a side-branch commit that IS on origin/master, and an older
/// one. Both are reachable, so only the descent rule can be what refuses them.
#[test]
fn p2_a_repin_to_a_revision_that_does_not_descend_from_the_tip_is_refused() {
    for to in [side(), older()] {
        let (dir, fresh) = fixture(Tip::Green);
        let err = refused(dir.path(), &fresh, &to, &history(&master()), None);
        assert_refused_by(&err, REPIN_NOT_A_DESCENDANT, "precondition 2 (descent)");
    }
}

/// (3) A descendant that origin/master does not reach yet (AHEAD OF REMOTE: committed,
/// not merged/pushed) is refused, and so is one this clone does not hold.
#[test]
fn p3_a_repin_to_a_revision_not_reachable_from_origin_master_is_refused() {
    // origin/master still at `old`: `new` descends from the tip but is not published.
    let (dir, fresh) = fixture(Tip::Green);
    let err = refused(dir.path(), &fresh, &new(), &history(&old()), None);
    assert_refused_by(&err, REPIN_NOT_ON_AEON_MASTER, "precondition 3 (ahead of remote)");
    assert!(err.contains("AHEAD OF REMOTE"), "the state must be named: {err}");

    // A descendant the remote tip does not reach, whose object the clone lacks after the
    // descent answer (descent can be known from a different oracle state; reachability not).
    let mut h = history(&master());
    let ghost = sha('f');
    h.ancestors.insert(ghost.clone(), vec![old(), older()]);
    let (dir, fresh) = fixture(Tip::Green);
    let err = refused(dir.path(), &fresh, &ghost, &h, None);
    assert_refused_by(&err, REPIN_NOT_ON_AEON_MASTER, "precondition 3 (object absent)");
}

/// (3, unmeasurable) An oracle that cannot answer never reads as a pass, and it refuses as
/// precondition 3 (reachability) carrying its COULD NOT MEASURE reason, not as a descent
/// failure it was never measured for.
#[test]
fn p3_an_unmeasurable_history_refuses_and_never_passes() {
    let (dir, fresh) = fixture(Tip::Green);
    let none = UnavailableRepo::new("COULD NOT MEASURE: AEON_DIR is not set");
    let err = refused(dir.path(), &fresh, &new(), &none, None);
    assert_refused_by(&err, REPIN_NOT_ON_AEON_MASTER, "precondition 3 (unmeasurable)");
    assert!(err.contains("COULD NOT MEASURE"), "the unmeasured reason must be carried: {err}");
}

/// `--ab` on a re-pin must be evidence: empty, the root sentinel, a chain entry's name and
/// a bare token are each refused by `REPIN_WITHOUT_EVIDENCE`; evidence-shaped refs pass
/// (every other test here uses one).
#[test]
fn a_repin_whose_ab_is_not_evidence_is_refused() {
    for ab in ["", "   ", "asl-witness", "tip", "master"] {
        let (dir, fresh) = fixture(Tip::Green);
        let golden = dir.path();
        let before = bytes(golden);
        let oracle = history(&master());
        let err = provenance::freeze_into_with_repin(
            golden, "the-repin", ab, &new(), "", &fresh, None, Some(&oracle),
        )
        .expect_err("a re-pin without evidence must be refused");
        assert_refused_by(&err, REPIN_WITHOUT_EVIDENCE, &format!("--ab {ab:?}"));
        assert_eq!(before, bytes(golden), "a refusal must not touch the ledger");
    }
}

/// (4) The tip must be green and attested: a red tip, an unattested tip on an armed
/// chain, and a chain where the rule never armed all refuse.
#[test]
fn p4_a_repin_past_a_tip_that_is_not_green_and_attested_is_refused() {
    for state in [Tip::Red, Tip::UnattestedArmed, Tip::UnattestedRatchet] {
        let (dir, fresh) = fixture(state);
        let err = refused(dir.path(), &fresh, &new(), &history(&master()), None);
        assert_refused_by(&err, REPIN_OF_AN_UNATTESTED_TIP, &format!("precondition 4 ({state:?})"));
    }
}

/// (5) Together with `--supersede-tip` — on a green tip and on a red one.
#[test]
fn p5_repin_with_supersede_tip_is_refused() {
    for state in [Tip::Green, Tip::Red] {
        let (dir, fresh) = fixture(state);
        let err = refused(dir.path(), &fresh, &new(), &history(&master()), Some("why"));
        assert_eq!(err, REPIN_WITH_SUPERSEDE, "precondition 5 ({state:?})");
    }
}

// ---------------------------------------------------------------------------
// THE HAPPY PATH.
// ---------------------------------------------------------------------------

#[test]
fn a_byte_neutral_repin_to_a_published_descendant_appends_an_entry_that_equals_its_predecessor() {
    let (dir, fresh) = fixture(Tip::Green);
    let golden = dir.path();
    let oracle = history(&master());
    let applied = provenance::freeze_into_with_repin(
        golden,
        "the-repin",
        "scripts/step0_shapes_cmp.sh 7/7 IDENTICAL",
        &new(),
        "operator note",
        &fresh,
        None,
        Some(&oracle),
    )
    .expect("a byte-neutral re-pin to a published descendant past a green tip must append");

    match applied {
        Applied::Appended { abandoned, byte_neutral, repinned, chain_len, notices } => {
            assert_eq!(abandoned, None, "a re-pin abandons nothing");
            assert!(byte_neutral, "the goldens did not move");
            assert_eq!(repinned, Some(old()), "the re-pin must name the revision it moved from");
            assert_eq!(chain_len, 3);
            assert!(notices.is_empty(), "no ratchet on an armed chain");
        }
        other => panic!("expected an append, got {other:?}"),
    }

    let chain = load(golden);
    let (pred, succ) = (&chain.entry[1], &chain.entry[2]);
    assert_eq!(succ.aeon_rev.as_deref(), Some(new().as_str()), "the new entry names the new revision");
    assert_eq!(succ.name, "the-repin");
    assert_eq!(succ.ab, "scripts/step0_shapes_cmp.sh 7/7 IDENTICAL");
    assert!(
        succ.note.contains(&old()) && succ.note.contains(&new()) && succ.note.contains("byte-identical")
            && succ.note.ends_with("operator note"),
        "the note must say what the repetition is: {:?}",
        succ.note
    );
    // EQUAL EXCEPT aeon_rev / name / ab / note: blank exactly those and compare whole.
    let mut a = pred.clone();
    let mut b = succ.clone();
    for e in [&mut a, &mut b] {
        e.aeon_rev = None;
        e.name.clear();
        e.ab.clear();
        e.note.clear();
    }
    a.strict = None; // the predecessor's attestation is about IT; the new tip awaits its own
    assert_eq!(a, b, "the re-pin entry must equal its predecessor except aeon_rev/name/ab/note");
    assert_eq!(succ.targets, fresh, "and its targets are what the blobs on disk hold");
    assert!(succ.strict.is_none() && succ.superseded.is_none() && pred.superseded.is_none());

    assert!(
        provenance::check(golden, &chain).is_empty(),
        "the chain must still pass provenance::check: {:?}",
        provenance::check(golden, &chain)
    );
    // And the EXISTING reachability rule judges the new coordinate reachable.
    let sigil = UnavailableRepo::new("COULD NOT MEASURE: sigil not asked here");
    let report = audit(&chain, &sigil, &oracle);
    let f = report
        .findings
        .iter()
        .find(|f| f.position == 3 && f.repo == Repo::Aeon && f.field == Field::EntryAeonRev)
        .expect("the audit must see the new entry's aeon_rev");
    assert_eq!(f.state, RevState::Reachable, "the re-pinned revision must be REACHABLE");
}

// ---------------------------------------------------------------------------
// --supersede-tip, and the plain fixpoint, are UNCHANGED.
// ---------------------------------------------------------------------------

/// Without `--repin-aeon` the decision is exactly what `plan_freeze` always decided, over
/// every tip state, both goldens outcomes and both supersede options.
#[test]
fn without_repin_the_plan_is_exactly_plan_freeze() {
    for state in [Tip::Green, Tip::Red, Tip::UnattestedArmed, Tip::UnattestedRatchet] {
        let (dir, same) = fixture(state);
        let chain = load(dir.path());
        let moved = write_blobs(dir.path(), 0x77);
        for fresh in [&same, &moved] {
            for sup in [None, Some("why"), Some("  ")] {
                let a = provenance::plan_freeze(&chain, fresh, "n", sup);
                let b = provenance::plan_freeze_with_repin(&chain, fresh, "n", sup, None);
                assert_eq!(a, b, "{state:?} sup={sup:?}: the no-repin path must not change");
                if let FreezeAction::Append(p) = &a {
                    assert_eq!(p.repin_from, None, "an ordinary/supersede append is never a re-pin");
                }
            }
        }
    }
}

/// The byte-neutral supersede of a red tip, through the repin-aware entry point with no
/// re-pin, writes the same file and reports the same thing as `freeze_into` — and the
/// plain byte-neutral freeze on a green tip is still the fixpoint.
#[test]
fn supersede_tip_behaviour_is_unchanged() {
    let (d1, fresh) = fixture(Tip::Red);
    let (d2, _) = fixture(Tip::Red);
    let why = Some("the fix moved no bytes");
    let a = provenance::freeze_into(d1.path(), "succ", "ab/scratch", &old(), "", &fresh, why).expect("supersede appends");
    let b = provenance::freeze_into_with_repin(d2.path(), "succ", "ab/scratch", &old(), "", &fresh, why, None)
        .expect("supersede appends");
    assert_eq!(a, b, "same outcome through either entry point");
    assert_eq!(bytes(d1.path()), bytes(d2.path()), "same ledger bytes through either entry point");
    match a {
        Applied::Appended { abandoned, repinned, .. } => {
            assert_eq!(abandoned.as_deref(), Some("tip"));
            assert_eq!(repinned, None);
        }
        other => panic!("expected an append, got {other:?}"),
    }

    let (d3, fresh) = fixture(Tip::Green);
    let before = bytes(d3.path());
    let r = provenance::freeze_into_with_repin(d3.path(), "succ", "ab/scratch", &new(), "", &fresh, None, None)
        .expect("a no-op re-freeze succeeds");
    assert_eq!(r, Applied::Fixpoint { tip: "tip".into() }, "no flag, no bytes moved: still the fixpoint");
    assert_eq!(before, bytes(d3.path()));
}
