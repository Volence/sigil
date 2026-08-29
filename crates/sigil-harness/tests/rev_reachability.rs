//! The ledger's recorded revisions must still EXIST in the histories they name.
//!
//! `provenance` checks that `aeon_rev` / `strict.sigil_rev` / `strict.aeon_rev` are 40
//! hex characters. A well-formed orphan passes that forever, which is the hole these
//! gates close.
//!
//! # What each half proves
//!
//! The FAKE-oracle half stages the classifier's branches directly. It is where the three
//! states the design turns on are separated, because a constructed history is the only
//! way to hold one variable at a time.
//!
//! The REAL-GIT half runs [`GitRevOracle`] against genuine repositories built in the
//! test's own temp directory, with the remote a local path so nothing touches a network.
//! It exists because a fake oracle proves the classifier and proves nothing about the
//! subprocess wiring underneath it — `cat-file` exit codes, `merge-base --is-ancestor`'s
//! three-valued exit, and `ls-remote`'s output shape are exactly where an oracle silently
//! answers the wrong question.
//!
//! No assertion here names a revision from the live ledger. A test that asserted today's
//! orphan is present would go poison-green the day it stopped being present.

use sigil_harness::provenance;
use sigil_harness::rev_reachability::{
    audit, classify, recorded_revisions, Field, GitRevOracle, RemoteTip, Repo, RevOracle, RevState,
    UnavailableRepo,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// A constructed history.
// ---------------------------------------------------------------------------

/// An oracle over a history written out by hand: which commits the clone holds, and the
/// transitive ancestor set of each. Enough to stage every state exactly once.
struct FakeRepo {
    tip: Result<RemoteTip, String>,
    present: Vec<String>,
    ancestors: BTreeMap<String, Vec<String>>,
}

impl FakeRepo {
    fn with_tip(tip_sha: &str) -> Self {
        Self {
            tip: Ok(RemoteTip { label: "origin/master".into(), sha: tip_sha.to_string() }),
            present: vec![tip_sha.to_string()],
            ancestors: BTreeMap::new(),
        }
    }
    fn holding(mut self, rev: &str) -> Self {
        self.present.push(rev.to_string());
        self
    }
    fn ancestry(mut self, descendant: &str, ancestors: &[&str]) -> Self {
        self.ancestors
            .insert(descendant.to_string(), ancestors.iter().map(|s| s.to_string()).collect());
        self
    }
}

impl RevOracle for FakeRepo {
    fn remote_tip(&self) -> Result<RemoteTip, String> {
        self.tip.clone()
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

/// A distinct, well-formed 40-char lowercase-hex SHA per letter. Constructed here rather
/// than copied from anywhere, so no assertion below is pinned to a revision that exists.
fn sha(tag: char) -> String {
    std::iter::repeat(tag).take(40).collect()
}

// ---------------------------------------------------------------------------
// THE THREE STATES, held apart.
// ---------------------------------------------------------------------------

#[test]
fn a_revision_reachable_from_the_remote_branch_does_not_fire() {
    let (tip, old) = (sha('a'), sha('b'));
    let repo = FakeRepo::with_tip(&tip).holding(&old).ancestry(&tip, &[&old]);
    let (state, against) = classify(&repo, &old);
    assert_eq!(state, RevState::Reachable, "an ancestor of the branch tip must be REACHABLE");
    assert_eq!(against.map(|t| t.sha), Some(tip), "the state must name what it was judged against");
    assert!(state.is_reachable());
}

#[test]
fn an_object_this_clone_does_not_hold_is_absent_not_orphaned() {
    let (tip, missing) = (sha('a'), sha('c'));
    // Deliberately NOT in `present`, and with no ancestry recorded either way.
    let repo = FakeRepo::with_tip(&tip);
    let (state, against) = classify(&repo, &missing);
    assert_eq!(
        state,
        RevState::ObjectAbsent,
        "a revision this clone has never fetched must be OBJECT ABSENT, never DIVERGENT"
    );
    let why = state.explain(against.as_ref());
    assert!(
        why.contains("fetch"),
        "state (2) must name its remedy; the reader is owed `git fetch`, got: {why}"
    );
    assert!(
        why.contains("TRANSIENT"),
        "state (2) must say it is transient, or it reads as the permanent defect: {why}"
    );
    assert!(!state.is_reachable(), "an absent object is not reachable");
}

#[test]
fn a_revision_a_rebase_left_behind_is_divergent_and_says_it_is_permanent() {
    // base <- tip (the branch), and base <- orphan (the abandoned line). Neither of tip
    // and orphan reaches the other: exactly what a rebase leaves behind.
    let (base, tip, orphan) = (sha('a'), sha('b'), sha('c'));
    let repo = FakeRepo::with_tip(&tip)
        .holding(&base)
        .holding(&orphan)
        .ancestry(&tip, &[&base])
        .ancestry(&orphan, &[&base]);
    let (state, against) = classify(&repo, &orphan);
    assert_eq!(
        state,
        RevState::Divergent,
        "present, unreachable, and not a descendant of the tip is the ORPHAN state"
    );
    let why = state.explain(against.as_ref());
    assert!(
        why.contains("PERMANENT") && why.contains("fetching cannot fix"),
        "state (3) must say fetching cannot fix it, or it reads like state (2): {why}"
    );
    assert!(!state.is_reachable());
}

#[test]
fn a_committed_but_unpushed_revision_is_ahead_not_orphaned() {
    // THE SPLIT THAT MAKES THE REPORT ACTIONABLE. A freeze commit between `--freeze` and
    // `git push` is "present but not reachable from the remote branch" — structurally the
    // same sentence as the orphan, and a completely different situation.
    let (tip, local) = (sha('a'), sha('b'));
    let repo = FakeRepo::with_tip(&tip).holding(&local).ancestry(&local, &[&tip]);
    let (state, against) = classify(&repo, &local);
    assert_eq!(
        state,
        RevState::AheadOfRemote,
        "the remote tip being an ANCESTOR of the revision means it is merely unpushed"
    );
    assert_ne!(state, RevState::Divergent);
    let why = state.explain(against.as_ref());
    assert!(why.contains("PUSH IT"), "the unpushed state must name its remedy: {why}");
}

// ---------------------------------------------------------------------------
// LOUD ON UNMEASURABLE.
// ---------------------------------------------------------------------------

#[test]
fn a_repository_that_cannot_be_asked_reads_as_could_not_measure() {
    let repo = UnavailableRepo::new("COULD NOT MEASURE: AEON_DIR is not set");
    let (state, against) = classify(&repo, &sha('a'));
    assert!(matches!(state, RevState::CouldNotMeasure(_)), "{state:?}");
    assert!(!state.is_reachable(), "an unmeasurable revision must never read as reachable");
    assert!(!state.is_measured());
    assert!(
        state.explain(against.as_ref()).contains("COULD NOT MEASURE"),
        "the house idiom must survive into the rendered line"
    );
}

#[test]
fn a_remote_that_cannot_be_reached_is_not_a_pass() {
    let mut repo = FakeRepo::with_tip(&sha('a'));
    repo.tip = Err("COULD NOT MEASURE: `git ls-remote origin` exited 128: no such host".into());
    let (state, _) = classify(&repo, &sha('b'));
    assert!(matches!(state, RevState::CouldNotMeasure(_)), "{state:?}");
    assert!(!state.is_reachable());
}

#[test]
fn a_malformed_revision_is_unmeasurable_rather_than_reachable() {
    let repo = FakeRepo::with_tip(&sha('a'));
    for bad in ["", "deadbeef", "NOTAHEXSTRINGNOTAHEXSTRINGNOTAHEXSTR1234"] {
        let (state, _) = classify(&repo, bad);
        assert!(
            matches!(state, RevState::CouldNotMeasure(_)),
            "`{bad}` has no history to search, so it cannot be judged; got {state:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// THE REPORT NAMES THE ENTRY AND THE REVISION.
// ---------------------------------------------------------------------------

/// A two-entry ledger: a root, and an entry carrying both an `aeon_rev` and a strict
/// record. Parsed through the real parser so the fixture cannot drift from the schema.
fn fixture_chain(aeon_rev: &str, sigil_rev: &str, strict_aeon_rev: &str) -> provenance::Chain {
    let src = format!(
        r#"
[[entry]]
name = "root-flip-freeze"
ab = "asl-witness"

[entry.targets.s4]
golden = "s4.bin"
full_crc = "00000001"
full_size = 16
anchor_crc = "00000002"
anchor_end = 0x10

[[entry]]
name = "ball-seating"
ab = "a/b log"
aeon_rev = "{aeon_rev}"

[entry.targets.s4]
golden = "s4.bin"
full_crc = "00000003"
full_size = 16
anchor_crc = "00000004"
anchor_end = 0x10

[entry.strict]
outcome = "passed"
sigil_rev = "{sigil_rev}"
aeon_rev = "{strict_aeon_rev}"
strict_bodies = 29
suites = 352
passed = 4081
failed = 0
ignored = 4
skips = 0
ran_at = "unix:1"

[entry.strict.goldens]
s4 = "00000003/16"
"#
    );
    provenance::parse(&src).expect("the fixture ledger parses")
}

#[test]
fn every_recorded_revision_is_walked_with_its_field_and_chain_position() {
    let (a, s) = (sha('a'), sha('b'));
    let chain = fixture_chain(&a, &s, &a);
    let revs = recorded_revisions(&chain);
    // DERIVED from the fixture, not transcribed: one `aeon_rev` per entry that carries
    // one, plus two revisions per strict record.
    let want: usize = chain
        .entry
        .iter()
        .map(|e| usize::from(e.aeon_rev.is_some()) + if e.strict.is_some() { 2 } else { 0 })
        .sum();
    assert_eq!(revs.len(), want, "the walk must see every recorded revision");
    assert!(
        revs.iter().any(|(p, n, r, f, v)| *p == 2
            && n == "ball-seating"
            && *r == Repo::Sigil
            && *f == Field::StrictSigilRev
            && *v == s),
        "the sigil revision must be attributed to sigil and to its own entry: {revs:?}"
    );
    assert!(
        revs.iter().any(|(_, _, r, f, _)| *r == Repo::Aeon && *f == Field::EntryAeonRev),
        "the entry's own aeon_rev must be walked too"
    );
}

#[test]
fn a_failure_names_the_parcel_the_chain_position_the_revision_and_the_state() {
    let (base, tip, orphan, aeon) = (sha('a'), sha('b'), sha('c'), sha('d'));
    let chain = fixture_chain(&aeon, &orphan, &aeon);
    let sigil_repo = FakeRepo::with_tip(&tip)
        .holding(&base)
        .holding(&orphan)
        .ancestry(&tip, &[&base])
        .ancestry(&orphan, &[&base]);
    let aeon_repo = FakeRepo::with_tip(&aeon);
    let result = audit(&chain, &sigil_repo, &aeon_repo);

    let orphans = result.divergent();
    assert_eq!(orphans.len(), 1, "exactly the one staged orphan: {:?}", result.notable());
    let line = orphans[0].line();
    for owed in ["entry #2", "ball-seating", "strict.sigil_rev", orphan.as_str(), "DIVERGENT"] {
        assert!(line.contains(owed), "the finding must name `{owed}`; got: {line}");
    }

    // The reachable aeon revision must NOT be reported. A check that flags everything is
    // a check nobody reads.
    let counts = result.counts_for(Repo::Aeon);
    assert_eq!(counts.divergent, 0, "the aeon revisions here are reachable: {:?}", result.notable());
    assert_eq!(counts.reachable, counts.total);

    let report = result.report();
    assert!(report.contains("DIVERGENT"), "{report}");
    assert!(report.contains(&orphan), "the report must carry the offending revision: {report}");
}

#[test]
fn an_all_reachable_ledger_reports_nothing_notable() {
    let (base, tip, aeon) = (sha('a'), sha('b'), sha('d'));
    let chain = fixture_chain(&aeon, &base, &aeon);
    let sigil_repo = FakeRepo::with_tip(&tip).holding(&base).ancestry(&tip, &[&base]);
    let aeon_repo = FakeRepo::with_tip(&aeon);
    let result = audit(&chain, &sigil_repo, &aeon_repo);
    assert!(result.notable().is_empty(), "{:?}", result.notable());
    assert!(result.report().contains("every recorded revision is reachable"), "{}", result.report());
}

#[test]
fn an_unavailable_repository_is_summarised_as_unmeasured_not_as_clean() {
    let (aeon, sigil_rev, tip) = (sha('d'), sha('b'), sha('b'));
    let chain = fixture_chain(&aeon, &sigil_rev, &aeon);
    let sigil_repo = FakeRepo::with_tip(&tip);
    let unavailable = UnavailableRepo::new("COULD NOT MEASURE: AEON_DIR is not set");
    let result = audit(&chain, &sigil_repo, &unavailable);
    let c = result.counts_for(Repo::Aeon);
    assert!(c.total > 0, "the fixture records aeon revisions");
    assert_eq!(c.unmeasured, c.total, "no aeon revision could be judged, so none may read as clean");
    assert_eq!(c.reachable, 0);
    let summary = result.summary_lines().join("\n");
    assert!(summary.contains("COULD NOT MEASURE"), "{summary}");
}

#[test]
fn one_unanswerable_repository_is_one_finding_and_still_names_every_entry() {
    let (aeon, sigil_rev) = (sha('d'), sha('b'));
    let chain = fixture_chain(&aeon, &sigil_rev, &aeon);
    let sigil_repo = FakeRepo::with_tip(&sigil_rev);
    let unavailable = UnavailableRepo::new("COULD NOT MEASURE: AEON_DIR is not set");
    let result = audit(&chain, &sigil_repo, &unavailable);

    let groups = result.groups();
    assert_eq!(groups.len(), 1, "one unavailable repository is ONE fact: {groups:?}");
    let (text, findings) = &groups[0];
    assert_eq!(findings.len(), result.counts_for(Repo::Aeon).total, "the group covers them all");
    let report = result.report();
    assert_eq!(
        report.matches(text.as_str()).count(),
        1,
        "the explanation is printed once, not once per revision:\n{report}"
    );
    for f in findings {
        assert!(report.contains(&f.site()), "every entry is still named:\n{report}");
    }
    assert!(
        !report.contains("COULD NOT MEASURE: COULD NOT MEASURE"),
        "the house idiom must not be stamped twice onto one reason:\n{report}"
    );
}

#[test]
fn the_orphan_is_reported_before_the_merely_unmeasured() {
    // Severity order, so the finding that cannot be fixed is not buried under the
    // findings that a fetch or a push would clear.
    let (base, tip, orphan, aeon) = (sha('a'), sha('b'), sha('c'), sha('d'));
    let chain = fixture_chain(&aeon, &orphan, &aeon);
    let sigil_repo = FakeRepo::with_tip(&tip)
        .holding(&base)
        .holding(&orphan)
        .ancestry(&tip, &[&base])
        .ancestry(&orphan, &[&base]);
    let result = audit(&chain, &sigil_repo, &UnavailableRepo::new("COULD NOT MEASURE: no aeon"));
    let groups = result.groups();
    assert_eq!(groups.len(), 2, "{groups:?}");
    assert!(groups[0].0.starts_with("DIVERGENT"), "the orphan leads: {}", groups[0].0);
}

// ---------------------------------------------------------------------------
// THE REAL ORACLE, over real repositories, with no network.
// ---------------------------------------------------------------------------

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=sigil-test",
            "-c",
            "user.email=sigil@test.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap_or_else(|e| {
            panic!("COULD NOT MEASURE: `git {}` in {} did not run ({e})", args.join(" "), dir.display())
        });
    assert!(
        out.status.success(),
        "COULD NOT MEASURE: `git {}` in {} exited {}: {}",
        args.join(" "),
        dir.display(),
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// origin: base -> tip on `master`. clone: fetched from origin, plus a commit built on
/// `tip` that was never pushed, plus a commit built on `base` that the branch has left
/// behind. Returns `(clone dir, base, tip, ahead, orphan)`.
fn staged_repositories(case: &str) -> (PathBuf, String, String, String, String) {
    let base_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rev_reachability").join(case);
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).expect("create the scratch tree");
    let origin = base_dir.join("origin");
    let clone = base_dir.join("clone");
    std::fs::create_dir_all(&origin).expect("create the origin tree");

    git(&origin, &["init", "--quiet"]);
    // Named explicitly rather than relying on git's default branch, which varies.
    git(&origin, &["symbolic-ref", "HEAD", "refs/heads/master"]);
    git(&origin, &["commit", "--allow-empty", "--quiet", "-m", "base"]);
    let base = git(&origin, &["rev-parse", "HEAD"]);
    git(&origin, &["commit", "--allow-empty", "--quiet", "-m", "tip"]);
    let tip = git(&origin, &["rev-parse", "HEAD"]);

    git(
        &base_dir,
        &["clone", "--quiet", origin.to_str().expect("utf-8 path"), clone.to_str().expect("utf-8 path")],
    );
    git(&clone, &["checkout", "--quiet", "-b", "unpushed", &tip]);
    git(&clone, &["commit", "--allow-empty", "--quiet", "-m", "ahead"]);
    let ahead = git(&clone, &["rev-parse", "HEAD"]);
    git(&clone, &["checkout", "--quiet", "-b", "abandoned", &base]);
    git(&clone, &["commit", "--allow-empty", "--quiet", "-m", "orphan"]);
    let orphan = git(&clone, &["rev-parse", "HEAD"]);

    (clone, base, tip, ahead, orphan)
}

#[test]
fn the_git_oracle_separates_the_states_over_real_repositories() {
    let (clone, base, tip, ahead, orphan) = staged_repositories("states");
    let oracle = GitRevOracle::at(&clone);

    let measured = oracle.remote_tip().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(measured.sha, tip, "the tip must come off the remote, not off a local branch");

    // The clone's own `master` is at `tip`; the oracle must not be reading HEAD.
    assert_eq!(classify(&oracle, &base).0, RevState::Reachable);
    assert_eq!(classify(&oracle, &tip).0, RevState::Reachable);
    assert_eq!(
        classify(&oracle, &ahead).0,
        RevState::AheadOfRemote,
        "a local commit on top of the remote tip is unpushed, not orphaned"
    );
    assert_eq!(
        classify(&oracle, &orphan).0,
        RevState::Divergent,
        "a commit on an abandoned line is the orphan state"
    );
    // Well-formed and nowhere: the fetch case.
    assert_eq!(classify(&oracle, &sha('b')).0, RevState::ObjectAbsent);
}

#[test]
fn the_git_oracle_reads_the_remote_rather_than_the_local_tracking_ref() {
    // A STALE TRACKING REF IS THE FAILURE MODE THIS GUARDS. `origin/master` in the clone
    // is a cached answer; the branch moves in the origin without it, and a check that
    // trusted the cache would call the new tip absent and the old tip current.
    let (clone, _base, tip, _ahead, _orphan) = staged_repositories("stale-tracking");
    let origin = clone.parent().expect("scratch tree").join("origin");
    git(&origin, &["commit", "--allow-empty", "--quiet", "-m", "moved"]);
    let moved = git(&origin, &["rev-parse", "HEAD"]);
    assert_ne!(moved, tip);

    let cached = git(&clone, &["rev-parse", "refs/remotes/origin/master"]);
    assert_eq!(cached, tip, "the tracking ref is still the old answer — that is the point");

    // The remote has moved and this clone has not fetched it, so the tip is not here to
    // compute ancestry against. That is unmeasurable, and it must SAY so.
    let oracle = GitRevOracle::at(&clone);
    let err = oracle.remote_tip().expect_err("the moved tip is not in this clone");
    assert!(err.contains("COULD NOT MEASURE"), "{err}");
    assert!(err.contains(&moved), "the reason must name the tip it could not resolve: {err}");
    assert!(err.contains("git fetch"), "the reason must name the remedy: {err}");

    let (state, _) = classify(&oracle, &tip);
    assert!(
        matches!(state, RevState::CouldNotMeasure(_)),
        "an unfetched branch tip makes every judgement unmeasurable, not green: {state:?}"
    );

    // After a fetch the same question is answerable, which is what makes the refusal
    // above a MEASUREMENT problem rather than a verdict.
    git(&clone, &["fetch", "--quiet", "origin"]);
    let after = GitRevOracle::at(&clone);
    assert_eq!(after.remote_tip().unwrap_or_else(|e| panic!("{e}")).sha, moved);
    assert_eq!(classify(&after, &tip).0, RevState::Reachable);
}

#[test]
fn a_repository_with_no_such_remote_is_unmeasurable_and_names_why() {
    let (clone, _base, tip, _ahead, _orphan) = staged_repositories("no-remote");
    let oracle = GitRevOracle::new(&clone, "nowhere", "master");
    let (state, _) = classify(&oracle, &tip);
    match state {
        RevState::CouldNotMeasure(why) => {
            assert!(why.contains("COULD NOT MEASURE"), "{why}");
            assert!(why.contains("ls-remote"), "the reason must name what failed: {why}");
        }
        other => panic!("a missing remote must not resolve to {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// THE LIVE LEDGER — enumeration only. No network, no pinned revision.
// ---------------------------------------------------------------------------

#[test]
fn the_walk_covers_the_committed_ledger_and_finds_only_well_formed_revisions() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("golden");
    let chain = provenance::load(&golden)
        .unwrap_or_else(|e| panic!("COULD NOT MEASURE: the committed ledger did not load: {e}"));
    let revs = recorded_revisions(&chain);
    let want: usize = chain
        .entry
        .iter()
        .map(|e| usize::from(e.aeon_rev.is_some()) + if e.strict.is_some() { 2 } else { 0 })
        .sum();
    assert_eq!(revs.len(), want, "the walk must see every revision the ledger records");
    assert!(want > 0, "COULD NOT MEASURE: the committed ledger records no revision at all");
    for (position, name, repo, field, rev) in &revs {
        assert!(
            provenance::is_full_sha(rev),
            "entry #{position} `{name}` · {} · {} `{rev}` is not a full 40-char SHA, so its \
             reachability can never be judged",
            field.as_str(),
            repo.as_str()
        );
    }
    // Both repositories are represented, or the walk is only half a walk.
    assert!(revs.iter().any(|(_, _, r, _, _)| *r == Repo::Sigil));
    assert!(revs.iter().any(|(_, _, r, _, _)| *r == Repo::Aeon));
}
