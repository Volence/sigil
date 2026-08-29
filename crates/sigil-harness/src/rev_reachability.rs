//! Whether the revisions `golden/provenance.toml` records still EXIST in the histories
//! they name.
//!
//! [`crate::provenance`] validates that `aeon_rev` / `strict.sigil_rev` /
//! `strict.aeon_rev` are well-FORMED — 40 lowercase hex characters. Shape is not
//! existence: a well-formed SHA that no branch reaches is a coordinate into a history
//! that has moved on, and it passes a shape check forever. The fields exist to give an
//! attestation TREE IDENTITY, so an attestation cannot silently travel to a different
//! pair of trees; a revision nothing reaches answers that question with a dangling
//! pointer.
//!
//! # The states, and why four rather than two
//!
//! A binary reachable/unreachable collapses causes whose REMEDIES are different, and the
//! remedy is the only thing a reader of the report needs:
//!
//!   * [`RevState::Reachable`] — an ancestor of the remote branch tip. Settled: nothing
//!     short of a force-push can take it away.
//!   * [`RevState::ObjectAbsent`] — the clone has never heard of the object. Almost
//!     always a missing `git fetch`. TRANSIENT and operator-fixable, and it says nothing
//!     about whether the revision is reachable in the remote's history — this clone
//!     simply cannot see far enough to answer.
//!   * [`RevState::AheadOfRemote`] — present, NOT an ancestor of the remote tip, but the
//!     remote tip IS an ancestor of it. That is a local commit sitting on top of the
//!     remote branch, waiting to be pushed. It is the NORMAL state of a freeze commit
//!     between `--freeze` and `git push`, so it is not a defect — but it is not durable
//!     either: a rebase before the push moves it and the recorded coordinate is orphaned
//!     with nothing to notice.
//!   * [`RevState::Divergent`] — present, and neither commit reaches the other. The
//!     revision was orphaned by a rebase or a force-push. PERMANENT: fetching cannot fix
//!     it, because the history that carried it is no longer the history the branch has.
//!     This is the real defect.
//!
//! Splitting `AheadOfRemote` out of `Divergent` is what makes the report actionable.
//! Both are "present but not reachable from the remote branch"; one is a routine
//! mid-ritual state and the other is an unrepairable record, and a check that prints the
//! same sentence for both trains its reader to ignore the sentence.
//!
//! [`RevState::CouldNotMeasure`] is the fifth state and is never green. No remote, no
//! network, no `AEON_DIR`, a malformed SHA — each renders as an explicit
//! `COULD NOT MEASURE` carrying its own reason, never as `Reachable` and never as a
//! silent omission.
//!
//! # What is judged against what
//!
//! `sigil_rev` is a coordinate in THIS repository; `aeon_rev` is a coordinate in the aeon
//! repository. Each is judged against its own remote branch, read with `git ls-remote`
//! at measurement time — not against a local tracking ref, which is a cached answer that
//! goes stale silently, and not against some sibling working directory's `HEAD`, which
//! is one checkout's opinion rather than the branch. The tip SHA `ls-remote` returns must
//! also be an object this clone HOLDS, or ancestry cannot be computed against it; when it
//! is not, that is [`RevState::CouldNotMeasure`] naming `git fetch` as the remedy, not a
//! pass.
//!
//! # What this module does NOT do
//!
//! It does not decide policy. [`audit`] returns findings; whether an orphan is a warning,
//! a report or a hard failure is the caller's ruling, and `refreeze` makes it. The
//! separation matters because the ledger is APPEND-ONLY: its historical entries are facts
//! that no gate can require anything of, while the revision a tool is ABOUT TO WRITE is
//! still the operator's to correct.

use crate::provenance::{is_full_sha, Chain};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The remote whose branch decides reachability. One remote, named rather than
/// discovered: a repository with several remotes has several answers, and the durable
/// one is the publication remote.
pub const REMOTE: &str = "origin";

/// The branch whose history a recorded revision must live in. Both repositories publish
/// on `master`; a revision reachable only from a parcel branch is not durable, because a
/// parcel branch is deleted after it merges.
pub const BRANCH: &str = "master";

/// How long a network probe of the remote may take before it is reported as unmeasured.
/// Bounded rather than unlimited because this runs inside a gate: an answer that never
/// arrives is worse than `COULD NOT MEASURE`, which at least says so.
pub const REMOTE_TIMEOUT: Duration = Duration::from_secs(20);

/// How long a purely local git query may take. Generous — it exists so a wedged
/// subprocess surfaces as a measurement failure rather than as a hung gate.
pub const LOCAL_TIMEOUT: Duration = Duration::from_secs(30);

/// Which repository a recorded revision is a coordinate in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Repo {
    Sigil,
    Aeon,
}

impl Repo {
    pub fn as_str(self) -> &'static str {
        match self {
            Repo::Sigil => "sigil",
            Repo::Aeon => "aeon",
        }
    }
}

/// Which ledger field carried the revision. Named exactly as it is spelled in the file,
/// so a reader can find it without a search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Field {
    EntryAeonRev,
    StrictSigilRev,
    StrictAeonRev,
}

impl Field {
    pub fn as_str(self) -> &'static str {
        match self {
            Field::EntryAeonRev => "aeon_rev",
            Field::StrictSigilRev => "strict.sigil_rev",
            Field::StrictAeonRev => "strict.aeon_rev",
        }
    }
}

/// A remote branch tip: what a revision's reachability is judged against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTip {
    /// Human label, e.g. `origin/master`.
    pub label: String,
    /// The tip commit as the remote reports it.
    pub sha: String,
}

/// One recorded revision's standing in the history it names. See the module doc for why
/// there are four measured states rather than two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevState {
    /// An ancestor of the remote branch tip.
    Reachable,
    /// Not an object in this clone at all.
    ObjectAbsent,
    /// Present; the remote tip is an ancestor of it. A local commit awaiting a push.
    AheadOfRemote,
    /// Present; neither reaches the other. Orphaned by a rebase or a force-push.
    Divergent,
    /// The question could not be asked. Carries the reason.
    CouldNotMeasure(String),
}

impl RevState {
    /// The short, all-caps word the report leads a line with.
    pub fn headline(&self) -> &'static str {
        match self {
            RevState::Reachable => "REACHABLE",
            RevState::ObjectAbsent => "OBJECT ABSENT",
            RevState::AheadOfRemote => "AHEAD OF REMOTE",
            RevState::Divergent => "DIVERGENT",
            RevState::CouldNotMeasure(_) => "COULD NOT MEASURE",
        }
    }

    /// The state's own sentence: what it means and what, if anything, fixes it. This is
    /// the whole point of distinguishing the states, so it lives with the state rather
    /// than at the call site where a caller could forget one.
    pub fn explain(&self, tip: Option<&RemoteTip>) -> String {
        let against =
            tip.map(|t| format!("{} ({})", t.label, t.sha)).unwrap_or_else(|| "the remote branch".to_string());
        match self {
            RevState::Reachable => format!("an ancestor of {against}"),
            RevState::ObjectAbsent => {
                "this clone holds no such object, so its standing in the branch's history \
                 could not be judged from here. TRANSIENT and operator-fixable: run `git \
                 fetch` and re-run. It is NOT evidence that the revision is orphaned."
                    .to_string()
            }
            RevState::AheadOfRemote => format!(
                "present, and {against} is an ancestor of it — a local commit that has not \
                 been pushed. Not yet a defect, and not yet durable: a rebase before the \
                 push orphans it and nothing else would notice. PUSH IT."
            ),
            RevState::Divergent => format!(
                "present in this clone, but NOT reachable from {against}, and {against} does \
                 not reach it either — the two histories have diverged. Orphaned by a rebase \
                 or a force-push. PERMANENT: fetching cannot fix this, because the history \
                 that carried this commit is not the history the branch has."
            ),
            RevState::CouldNotMeasure(why) => why.clone(),
        }
    }

    /// Headline and explanation as one sentence — the form a single finding is announced
    /// in. [`RevState::CouldNotMeasure`] carries the house idiom inside its own reason, so
    /// it is not prefixed a second time: a doubled `COULD NOT MEASURE: COULD NOT MEASURE:`
    /// reads as a formatting fault and costs the phrase the weight it is there for.
    pub fn describe(&self, tip: Option<&RemoteTip>) -> String {
        match self {
            RevState::CouldNotMeasure(_) => self.explain(tip),
            _ => format!("{}: {}", self.headline(), self.explain(tip)),
        }
    }

    pub fn is_reachable(&self) -> bool {
        matches!(self, RevState::Reachable)
    }

    pub fn is_measured(&self) -> bool {
        !matches!(self, RevState::CouldNotMeasure(_))
    }
}

/// One recorded revision, where it sits in the ledger, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevFinding {
    /// 1-based chain position — the number the ledger's readers call an entry by.
    pub position: usize,
    /// The entry's `name`.
    pub entry: String,
    pub repo: Repo,
    pub field: Field,
    pub rev: String,
    pub state: RevState,
    /// What the state was judged against, when a tip could be learned at all.
    pub tip: Option<RemoteTip>,
}

impl RevFinding {
    /// The one line a report prints for this finding. Names the entry BY PARCEL NAME AND
    /// CHAIN POSITION, names the field, names the revision, and says which state it is
    /// in — everything the reader would otherwise have to re-derive.
    pub fn line(&self) -> String {
        format!("{} — {}", self.site(), self.state.describe(self.tip.as_ref()))
    }

    /// Just the coordinate: entry, chain position, field, repository, revision. What a
    /// grouped report prints under a heading that already carries the explanation.
    pub fn site(&self) -> String {
        format!(
            "entry #{} `{}` · {} · {} {}",
            self.position,
            self.entry,
            self.field.as_str(),
            self.repo.as_str(),
            self.rev
        )
    }
}

/// The questions [`audit`] asks about a revision. A trait so the classification is
/// testable over constructed histories: the three states this exists to distinguish are
/// exactly the states that are awkward to stage in a real repository, and a check whose
/// interesting branches are only ever exercised against live git is a check whose
/// interesting branches are untested.
pub trait RevOracle {
    /// The branch tip revisions are judged against, or why it could not be learned.
    fn remote_tip(&self) -> Result<RemoteTip, String>;
    /// Whether this clone holds `rev` as a commit.
    fn has_commit(&self, rev: &str) -> Result<bool, String>;
    /// Whether `ancestor` is an ancestor of `descendant`. Both are known-present commits.
    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, String>;
}

/// A repository that is not available to be asked — no `AEON_DIR`, no checkout, no
/// remote. Every question it is asked answers `COULD NOT MEASURE` with the SAME reason,
/// so an unavailable repository reads as one stated fact repeated rather than as a
/// scattering of different failures.
pub struct UnavailableRepo {
    pub reason: String,
}

impl UnavailableRepo {
    pub fn new(reason: impl Into<String>) -> Self {
        Self { reason: reason.into() }
    }
}

impl RevOracle for UnavailableRepo {
    fn remote_tip(&self) -> Result<RemoteTip, String> {
        Err(self.reason.clone())
    }
    fn has_commit(&self, _rev: &str) -> Result<bool, String> {
        Err(self.reason.clone())
    }
    fn is_ancestor(&self, _a: &str, _d: &str) -> Result<bool, String> {
        Err(self.reason.clone())
    }
}

/// Run a git subprocess under a wall-clock bound. Returns `(exit code, stdout, stderr)`.
///
/// The bound is the reason this is not `Command::output()`: `ls-remote` talks to a
/// network, and a gate that hangs is a gate that gets removed. `GIT_TERMINAL_PROMPT=0`
/// keeps an auth prompt from turning the hang into an interactive one.
fn run_git(dir: &Path, args: &[&str], budget: Duration) -> Result<(i32, String, String), String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("COULD NOT MEASURE: spawn `git {}` in {}: {e}", args.join(" "), dir.display()))?;
    let deadline = Instant::now() + budget;
    loop {
        match child.try_wait() {
            Err(e) => {
                let _ = child.kill();
                return Err(format!("COULD NOT MEASURE: waiting on `git {}`: {e}", args.join(" ")));
            }
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "COULD NOT MEASURE: `git {}` in {} did not finish within {}s and was \
                         killed. An unanswered question is not a pass.",
                        args.join(" "),
                        dir.display(),
                        budget.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("COULD NOT MEASURE: reading `git {}`: {e}", args.join(" ")))?;
    Ok((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
    ))
}

/// A [`RevOracle`] backed by a real clone: the remote tip comes off the wire, presence
/// and ancestry come out of the local object database.
pub struct GitRevOracle {
    dir: PathBuf,
    remote: String,
    branch: String,
    /// The tip is asked for ONCE. A per-revision `ls-remote` would be hundreds of network
    /// round trips and, worse, hundreds of chances to judge two revisions against two
    /// different tips within one report.
    tip: RefCell<Option<Result<RemoteTip, String>>>,
}

impl GitRevOracle {
    pub fn new(dir: impl Into<PathBuf>, remote: impl Into<String>, branch: impl Into<String>) -> Self {
        Self {
            dir: dir.into(),
            remote: remote.into(),
            branch: branch.into(),
            tip: RefCell::new(None),
        }
    }

    /// The conventional oracle for a repository: `origin/master`.
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self::new(dir, REMOTE, BRANCH)
    }

    fn resolve_tip(&self) -> Result<RemoteTip, String> {
        let label = format!("{}/{}", self.remote, self.branch);
        let refspec = format!("refs/heads/{}", self.branch);
        let (code, stdout, stderr) = run_git(
            &self.dir,
            &["ls-remote", "--exit-code", &self.remote, &refspec],
            REMOTE_TIMEOUT,
        )?;
        if code != 0 {
            return Err(format!(
                "COULD NOT MEASURE: `git ls-remote {} {refspec}` in {} exited {code}: {}. \
                 Without the branch tip there is nothing to judge a revision against.",
                self.remote,
                self.dir.display(),
                if stderr.is_empty() { "(no stderr)" } else { &stderr }
            ));
        }
        let sha = stdout
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string();
        if !is_full_sha(&sha) {
            return Err(format!(
                "COULD NOT MEASURE: `git ls-remote {} {refspec}` returned `{stdout}`, whose \
                 first field is not a full 40-char SHA.",
                self.remote
            ));
        }
        // THE TIP MUST BE AN OBJECT WE HOLD. `ls-remote` reports what the remote has; it
        // does not deliver it. Ancestry against a commit this clone does not carry cannot
        // be computed, and answering anyway would be answering a different question.
        let (t_code, t_out, _) = run_git(&self.dir, &["cat-file", "-t", &sha], LOCAL_TIMEOUT)?;
        if t_code != 0 || t_out != "commit" {
            return Err(format!(
                "COULD NOT MEASURE: {label} is at {sha}, which this clone at {} does not hold \
                 as a commit. Ancestry cannot be computed against a tip that is not here — \
                 run `git fetch {}` and re-run.",
                self.dir.display(),
                self.remote
            ));
        }
        Ok(RemoteTip { label, sha })
    }
}

impl RevOracle for GitRevOracle {
    fn remote_tip(&self) -> Result<RemoteTip, String> {
        let mut slot = self.tip.borrow_mut();
        if slot.is_none() {
            *slot = Some(self.resolve_tip());
        }
        slot.clone().expect("just populated")
    }

    fn has_commit(&self, rev: &str) -> Result<bool, String> {
        let (code, out, _) = run_git(&self.dir, &["cat-file", "-t", rev], LOCAL_TIMEOUT)?;
        if code != 0 {
            return Ok(false);
        }
        if out != "commit" {
            return Err(format!(
                "COULD NOT MEASURE: {rev} names a {out} in {}, not a commit, so it has no \
                 place in a branch's history to look for.",
                self.dir.display()
            ));
        }
        Ok(true)
    }

    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, String> {
        let (code, _, stderr) = run_git(
            &self.dir,
            &["merge-base", "--is-ancestor", ancestor, descendant],
            LOCAL_TIMEOUT,
        )?;
        match code {
            0 => Ok(true),
            1 => Ok(false),
            other => Err(format!(
                "COULD NOT MEASURE: `git merge-base --is-ancestor {ancestor} {descendant}` in \
                 {} exited {other}: {}",
                self.dir.display(),
                if stderr.is_empty() { "(no stderr)" } else { &stderr }
            )),
        }
    }
}

/// Classify ONE revision. The order of the questions is the design: shape before
/// existence (a malformed SHA has no history to search), existence before ancestry (an
/// absent object cannot be judged and must not read as orphaned), and the reverse
/// ancestry test last, because it is the only thing that separates a commit awaiting a
/// push from one a rebase left behind.
pub fn classify(oracle: &dyn RevOracle, rev: &str) -> (RevState, Option<RemoteTip>) {
    if !is_full_sha(rev) {
        return (
            RevState::CouldNotMeasure(format!(
                "COULD NOT MEASURE: `{rev}` is not a full 40-char lowercase-hex SHA, so no \
                 history can be searched for it. The malformation itself is reported by the \
                 ledger's own well-formedness rules."
            )),
            None,
        );
    }
    let tip = match oracle.remote_tip() {
        Ok(t) => t,
        Err(why) => return (RevState::CouldNotMeasure(why), None),
    };
    match oracle.has_commit(rev) {
        Err(why) => return (RevState::CouldNotMeasure(why), Some(tip)),
        Ok(false) => return (RevState::ObjectAbsent, Some(tip)),
        Ok(true) => {}
    }
    match oracle.is_ancestor(rev, &tip.sha) {
        Err(why) => (RevState::CouldNotMeasure(why), Some(tip)),
        Ok(true) => (RevState::Reachable, Some(tip)),
        Ok(false) => match oracle.is_ancestor(&tip.sha, rev) {
            Err(why) => (RevState::CouldNotMeasure(why), Some(tip)),
            Ok(true) => (RevState::AheadOfRemote, Some(tip)),
            Ok(false) => (RevState::Divergent, Some(tip)),
        },
    }
}

/// Every revision the ledger records, in chain order, with the field it came from. Pure:
/// no git, no network. Separated from [`audit`] so "did the walk see everything?" is
/// answerable without a repository.
pub fn recorded_revisions(chain: &Chain) -> Vec<(usize, String, Repo, Field, String)> {
    let mut out = Vec::new();
    for (i, e) in chain.entry.iter().enumerate() {
        let pos = i + 1;
        if let Some(rev) = &e.aeon_rev {
            out.push((pos, e.name.clone(), Repo::Aeon, Field::EntryAeonRev, rev.clone()));
        }
        if let Some(st) = &e.strict {
            out.push((pos, e.name.clone(), Repo::Sigil, Field::StrictSigilRev, st.sigil_rev.clone()));
            out.push((pos, e.name.clone(), Repo::Aeon, Field::StrictAeonRev, st.aeon_rev.clone()));
        }
    }
    out
}

/// The result of walking the whole ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Audit {
    pub findings: Vec<RevFinding>,
}

/// Per-repository tallies — what a one-line summary is built from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub total: usize,
    pub reachable: usize,
    pub absent: usize,
    pub ahead: usize,
    pub divergent: usize,
    pub unmeasured: usize,
}

impl Audit {
    pub fn counts_for(&self, repo: Repo) -> Counts {
        let mut c = Counts::default();
        for f in self.findings.iter().filter(|f| f.repo == repo) {
            c.total += 1;
            match f.state {
                RevState::Reachable => c.reachable += 1,
                RevState::ObjectAbsent => c.absent += 1,
                RevState::AheadOfRemote => c.ahead += 1,
                RevState::Divergent => c.divergent += 1,
                RevState::CouldNotMeasure(_) => c.unmeasured += 1,
            }
        }
        c
    }

    /// Findings that are not plainly reachable, in chain order — everything a reader has
    /// to act on or account for.
    pub fn notable(&self) -> Vec<&RevFinding> {
        self.findings.iter().filter(|f| !f.state.is_reachable()).collect()
    }

    /// Orphans: the permanent, unrepairable class.
    pub fn divergent(&self) -> Vec<&RevFinding> {
        self.findings.iter().filter(|f| f.state == RevState::Divergent).collect()
    }

    /// One line per repository. Never omitted and never silently green: a repository
    /// nothing could be measured in says so in its own line.
    pub fn summary_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for repo in [Repo::Sigil, Repo::Aeon] {
            let c = self.counts_for(repo);
            if c.total == 0 {
                out.push(format!("{}: no revision recorded in the ledger", repo.as_str()));
                continue;
            }
            let against = self
                .findings
                .iter()
                .find(|f| f.repo == repo && f.tip.is_some())
                .and_then(|f| f.tip.as_ref())
                .map(|t| format!("{} {}", t.label, t.sha))
                .unwrap_or_else(|| "no remote tip".to_string());
            out.push(format!(
                "{}: {} revision(s) vs {against} — {} reachable, {} OBJECT ABSENT, {} AHEAD OF \
                 REMOTE, {} DIVERGENT, {} COULD NOT MEASURE",
                repo.as_str(),
                c.total,
                c.reachable,
                c.absent,
                c.ahead,
                c.divergent,
                c.unmeasured
            ));
        }
        out
    }

    /// The findings that are not plainly reachable, GROUPED by the sentence that
    /// describes them, worst first.
    ///
    /// Grouping is not cosmetic. One unavailable repository produces one fact — its
    /// history could not be searched — and printing that fact once per recorded revision
    /// buries the single real finding under two dozen copies of a sentence that is the
    /// same every time. Each group still names EVERY entry it covers, by parcel name,
    /// chain position, field and revision: the repetition that is dropped is the
    /// explanation, never the coordinates.
    pub fn groups(&self) -> Vec<(String, Vec<&RevFinding>)> {
        // Worst first: an orphan is unrepairable, an absent object is a fetch, an
        // unpushed commit is a push, and an unmeasured revision is a question nobody
        // answered. Order within a class stays chain order.
        let rank = |s: &RevState| match s {
            RevState::Divergent => 0,
            RevState::ObjectAbsent => 1,
            RevState::AheadOfRemote => 2,
            RevState::CouldNotMeasure(_) => 3,
            RevState::Reachable => 4,
        };
        let mut order: Vec<String> = Vec::new();
        let mut by_text: BTreeMap<String, (usize, Vec<&RevFinding>)> = BTreeMap::new();
        for f in self.notable() {
            let text = f.state.describe(f.tip.as_ref());
            let slot = by_text.entry(text.clone()).or_insert_with(|| {
                order.push(text.clone());
                (rank(&f.state), Vec::new())
            });
            slot.1.push(f);
        }
        order.sort_by_key(|t| (by_text[t].0, t.clone()));
        order
            .into_iter()
            .map(|t| {
                let group = by_text.remove(&t).expect("every ordered text has a group");
                (t, group.1)
            })
            .collect()
    }

    /// The full report: the per-repository summary, then every not-plainly-reachable
    /// revision under the sentence that explains it.
    pub fn report(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        for l in self.summary_lines() {
            let _ = writeln!(s, "  {l}");
        }
        let groups = self.groups();
        if groups.is_empty() {
            let _ = writeln!(s, "  every recorded revision is reachable from its remote branch.");
            return s;
        }
        for (text, findings) in groups {
            let _ = writeln!(s, "  {} revision(s) — {text}", findings.len());
            for f in findings {
                let _ = writeln!(s, "    {}", f.site());
            }
        }
        s
    }
}

/// Walk the whole ledger and classify every recorded revision.
///
/// Each distinct `(repo, revision)` is asked about ONCE — the same aeon SHA appears in
/// both an entry's `aeon_rev` and its `strict.aeon_rev`, and asking twice would be
/// hundreds of redundant subprocesses and a chance for one report to contain two answers
/// to one question.
pub fn audit(chain: &Chain, sigil: &dyn RevOracle, aeon: &dyn RevOracle) -> Audit {
    let mut cache: BTreeMap<(Repo, String), (RevState, Option<RemoteTip>)> = BTreeMap::new();
    let mut findings = Vec::new();
    for (position, entry, repo, field, rev) in recorded_revisions(chain) {
        let key = (repo, rev.clone());
        let answer = match cache.get(&key) {
            Some(a) => a.clone(),
            None => {
                let oracle: &dyn RevOracle = match repo {
                    Repo::Sigil => sigil,
                    Repo::Aeon => aeon,
                };
                let a = classify(oracle, &rev);
                cache.insert(key, a.clone());
                a
            }
        };
        findings.push(RevFinding {
            position,
            entry,
            repo,
            field,
            rev,
            state: answer.0,
            tip: answer.1,
        });
    }
    Audit { findings }
}
