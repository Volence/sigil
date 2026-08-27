//! The golden provenance chain (`golden/provenance.toml`) — the §17 optimization
//! arc's machine-checkable re-freeze discipline.
//!
//! Post-flip the six golden blobs are sigil's OWN frozen output, so a byte-CHANGING
//! optimization proves correctness on the oracle (the emulator A/B) and then re-freezes
//! the goldens. This module models the append-only chain that records every such
//! re-freeze, and enforces the two invariants:
//!
//!   1. **TIP-MATCH** — each committed golden's recomputed `full_crc`/`full_size` and
//!      header-neutral `anchor_crc` equals the chain tip.
//!   2. **ANCHOR-MOVE-NEEDS-A/B** — a target whose `anchor_crc` differs from the prior
//!      entry forces the newer entry to carry a non-empty `ab` A/B-evidence ref. An
//!      anchor that moved without evidence is a HARD failure.
//!   3. **AEON-REV-WELL-FORMED** — an entry carrying an `aeon_rev` at all carries a full
//!      40-character SHA, wherever it sits in the chain.
//!   4. **AEON-REV-MONOTONIC** — once any entry names the aeon revision its bytes were
//!      built from, no later entry may omit it. The boundary is DERIVED from the chain,
//!      never pinned to a number: see [`check`] for the merge race a constant creates.
//!   5. **STRICT-ATTEST-WELL-FORMED** — an entry carrying a [`StrictRun`] carries a
//!      coherent one: real revisions, a NONZERO strict-body count, and golden CRCs that
//!      match its own targets. An entry marked [`Superseded`] names the entry that
//!      actually follows it and carries a RED run.
//!   6. **STRICT-ATTEST-MONOTONIC** — once any entry records a strict run, every entry
//!      the chain has been BUILT ON must carry a passing one or be superseded. The TIP
//!      is exempt: its strict run necessarily happens after the freeze that created it.
//!      Also derived, never pinned — see [`append_gate`].
//!
//! [`check`] validates a loaded chain against the committed blobs (the gate, and the
//! `--check` half of the `refreeze` bin). [`recompute_targets`] rebuilds the per-target
//! CRC set from the blobs (the `--freeze` half). Both reuse [`crate::native::crc32`] and
//! [`crate::native::assembled_anchor_crc`] so the CRC math never diverges from the
//! full-file / anchor gates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The parsed `provenance.toml` — an append-only list of re-freeze entries, oldest
/// (`root-flip-freeze`) first, tip last.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chain {
    pub entry: Vec<Entry>,
}

/// One re-freeze event: a named parcel, its A/B evidence ref, and the per-target CRC
/// set the goldens hold AFTER it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Entry {
    pub name: String,
    /// A/B evidence reference. The root carries the `asl-witness` sentinel; every later
    /// entry that MOVES an anchor must carry a real ref (a design-note / A/B-log path).
    pub ab: String,
    /// The aeon revision the frozen bytes were built from: a full 40-character SHA.
    ///
    /// `None` means the KEY IS ABSENT — the entry was appended by a `refreeze` that
    /// predates the field. `Some` means the key is present and, since `render_entry`
    /// always writes it and `--freeze` refuses without a vetted SHA, should always be a
    /// valid one; a `Some` that is not is a hand edit and fails [`check`] wherever it
    /// sits. That distinction is the whole reason this is an `Option` rather than a
    /// `String` defaulting to empty: "nobody had written it yet" and "somebody blanked
    /// it" are different facts and only one of them is legitimate.
    ///
    /// Full, never abbreviated — a short SHA is a coordinate that grows ambiguous as
    /// history grows, and this field's whole job is to be unambiguous years later.
    #[serde(default)]
    pub aeon_rev: Option<String>,
    #[serde(default)]
    pub note: String,
    /// The witness that sigil's STRICT full suite was RUN on the tree carrying this
    /// entry — written by `refreeze --attest` from a run it performed itself, never by
    /// hand. `None` means the key is absent: either the entry predates the field, or it
    /// is the tip and its strict run has not happened yet. See [`StrictRun`].
    #[serde(default)]
    pub strict: Option<StrictRun>,
    /// The THIRD state. An entry whose strict run came back RED can never be attested
    /// honestly, yet the fix for what turned it red usually MOVES BYTES — so the chain
    /// must be able to continue past it without either forging a pass or hand-editing
    /// the file. This records that the entry was abandoned and NAMES THE ENTRY THAT
    /// REPLACED IT. See [`Superseded`] for why reaching this state costs a real freeze
    /// and a real red run.
    #[serde(default)]
    pub superseded: Option<Superseded>,
    /// target-key -> the golden's frozen CRC set. Keys are stable
    /// (`s4`/`s4_debug`/`demo`/`demo_debug`/`config_a`/`config_b`).
    pub targets: BTreeMap<String, Target>,
}

/// One golden's two CRC layers plus the anchor window.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Target {
    /// The committed blob filename in `golden/`.
    pub golden: String,
    /// Full-file CRC-32 (hex, no `0x`) — the regression pin (drifts on appendix change).
    pub full_crc: String,
    pub full_size: usize,
    /// Header-neutral assembled-anchor CRC-32 (hex) — the PRIMARY, drift-stable bar.
    pub anchor_crc: String,
    /// EndOfRom — the anchor window `[0, anchor_end)`.
    pub anchor_end: usize,
}

/// A run of sigil's STRICT full suite, recorded against the entry whose goldens it
/// tested. Written ONLY by `refreeze --attest`, from a suite the tool ran itself.
///
/// # Why this exists
///
/// A refreeze could land here with the strict suite never having run. Chains 169 and
/// 170 both did: the landing rule named the full suite but spelled it as a bare
/// `cargo test --release --workspace --no-fail-fast` with no `SIGIL_STRICT_GATE=1`, so
/// every `strict_gate()`-guarded body early-returned and the run was green and inert.
/// A stale `SFX_BODY_LEN` rode two chains before chain 171 finally ran strict. Fixed
/// PROSE cannot close that: nobody audits a command line for a missing environment
/// variable. This record is the enforcement.
///
/// # What makes it honest
///
/// A self-emitted attestation is self-certifying by construction, so its whole value is
/// in carrying things a run that did not happen could not produce. Every field is here
/// because it answers "could this value be right if the run never happened?" with NO:
///
///   * [`Self::strict_bodies`] — THE load-bearing field. Structurally zero without the
///     flag, and no aggregate pass count can substitute, because a NON-strict suite is
///     also fully green. This is the single number that separates "ran strict" from
///     "ran", which is exactly what chains 169/170 could not be asked.
///   * [`Self::sigil_rev`] / [`Self::aeon_rev`] — tree identity, so an attestation
///     cannot silently travel to a different pair of trees.
///   * [`Self::goldens`] — the per-target `crc/size` set recomputed FROM THE BLOBS at
///     run time, never copied out of the entry. An attestation that outlives its
///     artifacts stops matching them, and [`check`] fails it when it does.
///
/// The first two stop an attestation being REUSED; the third stops it going stale; the
/// first stops it being VACUOUS.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct StrictRun {
    /// [`OUTCOME_PASSED`] or [`OUTCOME_FAILED`]. A red run is recorded rather than
    /// discarded: it is the only thing that unlocks [`Superseded`], so the tool that
    /// observed the red is the one that authorises the chain to move past it.
    pub outcome: String,
    /// The sigil revision the suite RAN ON — HEAD of a clean sigil checkout. Not "the
    /// commit that introduced this entry": when a sigil-side fix is needed before the
    /// suite can run at all, the honest subject is the tree that was tested.
    ///
    /// Cleanliness is required at attest time for a second reason: the suite contains
    /// `version_reports_the_head_of_the_tree_it_was_built_from`, which compares the
    /// binary's baked-in revision against HEAD *at assertion time*. Attesting from a
    /// committed, unmoving HEAD is what keeps this field and that test agreeing.
    pub sigil_rev: String,
    /// HEAD of the `AEON_DIR` the suite ran against — a full 40-char SHA. When the
    /// entry carries an [`Entry::aeon_rev`], [`check`] requires these to be EQUAL: the
    /// only aeon tree whose ROMs match these goldens is the one they were frozen from.
    pub aeon_rev: String,
    /// Distinct strict-gated decision points that consulted `SIGIL_STRICT_GATE` and
    /// found it SET, counted by `sigil_harness::test_support::strict_gate`'s witness.
    /// Zero is not a small number here — it is the signature of a suite that ran
    /// without the flag, and [`check`] rejects it.
    pub strict_bodies: usize,
    /// Test binaries that reported a `test result:` line. Zero means the run could not
    /// be measured, which is never green.
    pub suites: usize,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    /// `skip:` lines seen in the run. A `skip:` under strict is a gate that reported
    /// green while measuring nothing, so this number is what the green was worth.
    /// RECORDED, not refused — see `refreeze`'s `--attest` for why this lane's
    /// zero-skip bar is reported here rather than enforced here.
    pub skips: usize,
    /// When the run finished (RFC-3339-ish, local). Forensic only: unlike every other
    /// field it cannot be cross-checked, and it is kept solely to correlate the record
    /// with a suite log.
    pub ran_at: String,
    /// On a red run, the failing test names — the aggregate counts alone would leave
    /// the reason for a [`Superseded`] unnamed.
    #[serde(default)]
    pub failing: Vec<String>,
    /// Tests the operator required to appear in this run (`--expect-test`), verified
    /// present before the record was written. This is the "the landed code's own test
    /// name appears in its own green log" discipline, moved out of a human's grep.
    #[serde(default)]
    pub expected_tests: Vec<String>,
    /// target-key -> `"<full_crc>/<full_size>"`, recomputed from the committed blobs at
    /// run time. CRC32+size is this campaign's identity standard; never SHA1.
    pub goldens: BTreeMap<String, String>,
}

/// [`StrictRun::outcome`] for a run that came back green.
pub const OUTCOME_PASSED: &str = "passed";
/// [`StrictRun::outcome`] for a run that came back red. The only thing that unlocks
/// [`Superseded`].
pub const OUTCOME_FAILED: &str = "failed";

/// The record that an entry was ABANDONED rather than attested, naming the entry that
/// replaced it.
///
/// # Why a third state, and why it is expensive to reach
///
/// Two states — attested or not — deadlock the chain on a failed freeze: entry N lands,
/// its strict run is genuinely RED, and the fix moves bytes. Entry N is now permanently
/// unattestable, so under a two-state rule the only exits are hand-editing the file or
/// writing a false attestation: the exact two acts this gate exists to prevent. A
/// ratchet whose failure mode is "the honest operator must forge a field" teaches the
/// wrong reflex.
///
/// So this state exists — but it must not become the cheap way out, or it retires the
/// gate. TWO independent costs guard it, and neither can be paid by typing:
///
///   1. **A successor must be NAMED**, and [`check`] requires the name to be the very
///      next entry in the chain. So abandonment is only reachable by actually
///      performing the next freeze — never as a way to skip attestation on a freeze you
///      intend to keep.
///   2. **The abandoned entry must already carry a RED [`StrictRun`]**. Without this,
///      serial supersession evades the gate completely: freeze, supersede, freeze,
///      supersede, and the strict suite never runs again. With it, every abandonment
///      costs a real strict run that genuinely came back red, which is the run the
///      whole mechanism is trying to force.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Superseded {
    /// The name of the entry that replaced this one. [`check`] requires it to equal the
    /// NEXT entry's name — derived from the chain, so it cannot name a fiction.
    pub by: String,
    /// Why the entry was abandoned, in one line.
    pub reason: String,
}

/// The `ab` sentinel plus the two `outcome` words are the only magic strings here.
impl StrictRun {
    /// Every way this record is malformed, judged against the entry it sits on. Empty =
    /// well-formed. `entry_targets`/`entry_aeon_rev` come from the OWNING entry, which
    /// is what makes the cross-checks derived rather than copied.
    pub fn defects(
        &self,
        entry_targets: &BTreeMap<String, Target>,
        entry_aeon_rev: Option<&str>,
    ) -> Vec<String> {
        let mut d = Vec::new();
        if self.outcome != OUTCOME_PASSED && self.outcome != OUTCOME_FAILED {
            d.push(format!(
                "outcome = \"{}\" is neither \"{OUTCOME_PASSED}\" nor \"{OUTCOME_FAILED}\"",
                self.outcome
            ));
        }
        if !is_full_sha(&self.sigil_rev) {
            d.push(format!("sigil_rev = \"{}\" is not a full 40-char SHA", self.sigil_rev));
        }
        if !is_full_sha(&self.aeon_rev) {
            d.push(format!("aeon_rev = \"{}\" is not a full 40-char SHA", self.aeon_rev));
        }
        // THE VACUITY CHECK. A suite run without SIGIL_STRICT_GATE cannot reach a single
        // strict-gated body, so it cannot produce a nonzero count here however green it
        // is. Zero therefore means "this attestation is about a run that proved nothing
        // the strict gate exists to prove" — never "a small run".
        if self.strict_bodies == 0 {
            d.push(
                "strict_bodies = 0 — no strict-gated body executed, which is the signature \
                 of a suite run WITHOUT SIGIL_STRICT_GATE=1; an attestation of such a run \
                 is vacuous"
                    .to_string(),
            );
        }
        // Loud on unmeasurable: a run nobody could count is not a run that passed.
        if self.suites == 0 {
            d.push("suites = 0 — no test binary reported a result line".to_string());
        }
        if self.passed == 0 {
            d.push("passed = 0 — no test executed".to_string());
        }
        match self.outcome.as_str() {
            OUTCOME_PASSED => {
                if self.failed != 0 {
                    d.push(format!("outcome = \"passed\" but failed = {}", self.failed));
                }
                if !self.failing.is_empty() {
                    d.push(format!(
                        "outcome = \"passed\" but {} failing test(s) are named",
                        self.failing.len()
                    ));
                }
            }
            // Guard rather than a nested `if`: the `_` arm below is a no-op, so a
            // non-matching guard falls through to exactly the same nothing.
            OUTCOME_FAILED if self.failed == 0 => {
                d.push(
                    "outcome = \"failed\" but failed = 0 — a red run must say what was red"
                        .to_string(),
                );
            }
            _ => {}
        }
        // ANTI-TRAVEL. These CRCs were read from the blobs at run time; the entry's were
        // read from the blobs at freeze time. They agree exactly when the attestation is
        // about THIS entry's artifacts.
        for (key, t) in entry_targets {
            let want = format!("{}/{}", t.full_crc, t.full_size);
            match self.goldens.get(key) {
                None => d.push(format!("goldens has no `{key}`, but the entry freezes one")),
                Some(got) if *got != want => d.push(format!(
                    "goldens.{key} = \"{got}\" but the entry freezes \"{want}\" — this \
                     attestation is about different bytes"
                )),
                Some(_) => {}
            }
        }
        for key in self.goldens.keys() {
            if !entry_targets.contains_key(key) {
                d.push(format!("goldens names `{key}`, which the entry does not freeze"));
            }
        }
        // PAIRING. The only aeon tree whose ROMs match these goldens is the one they were
        // frozen from, so a suite run against any other tree is not a test of this entry.
        if let Some(want) = entry_aeon_rev.filter(|r| is_full_sha(r)) {
            if self.aeon_rev != want {
                d.push(format!(
                    "aeon_rev = \"{}\" but the entry was frozen from \"{want}\" — the suite \
                     ran against a different aeon tree than these goldens came from",
                    self.aeon_rev
                ));
            }
        }
        d
    }

    /// Well-formed AND green — the state that satisfies the append gate.
    pub fn is_pass(&self, targets: &BTreeMap<String, Target>, aeon_rev: Option<&str>) -> bool {
        self.outcome == OUTCOME_PASSED && self.defects(targets, aeon_rev).is_empty()
    }

    /// Well-formed AND red — the state that unlocks [`Superseded`].
    pub fn is_red(&self, targets: &BTreeMap<String, Target>, aeon_rev: Option<&str>) -> bool {
        self.outcome == OUTCOME_FAILED && self.defects(targets, aeon_rev).is_empty()
    }
}

impl Entry {
    /// This entry's strict record, if it carries a well-formed one of EITHER outcome.
    /// A malformed record is not a record: it must raise its own well-formedness error
    /// and nothing else, so one fault produces one error.
    fn sound_strict(&self) -> Option<&StrictRun> {
        self.strict
            .as_ref()
            .filter(|s| s.defects(&self.targets, self.aeon_rev.as_deref()).is_empty())
    }

    /// The entry has a green strict run recorded against its own goldens.
    pub fn is_attested(&self) -> bool {
        self.sound_strict().is_some_and(|s| s.outcome == OUTCOME_PASSED)
    }

    /// The entry has a red strict run recorded against its own goldens — the
    /// precondition for abandoning it.
    pub fn is_red(&self) -> bool {
        self.sound_strict().is_some_and(|s| s.outcome == OUTCOME_FAILED)
    }
}

/// What appending a new entry to this chain is allowed to do — the freeze-time half of
/// the strict-attestation rule, as a pure function so it can be tested without a build,
/// an aeon tree or a subprocess.
///
/// The boundary is DERIVED from the chain and never pinned to an entry number. That is a
/// correctness property, not a tidiness one: the aeon lane refreezes by running this
/// crate's `refreeze` out of sigil master, so a byte-moving refreeze landing before this
/// field ships appends a legitimate record-less entry that a pinned rule would then turn
/// red on somebody else's correct work. Derived, such an entry is simply still pre-field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppendGate {
    /// No entry carries a strict record yet, so the rule is not in force. Self-disarming:
    /// it stops being reachable the moment any entry records a run. Reported as
    /// `ratchet:`, never `skip:` — this lane's strict bar requires zero `skip:` lines and
    /// this is not a missing reference.
    Ratchet(String),
    /// The tip carries a green strict run. Append freely.
    Allowed,
    /// The tip's strict run came back RED. Appending is allowed, but only as an explicit
    /// abandonment that names this new entry as the successor.
    NeedsSupersede(String),
    /// The tip carries no strict run at all and the rule is in force.
    Refused(String),
}

/// Decide what an append to `chain` may do. See [`AppendGate`].
pub fn append_gate(chain: &Chain) -> AppendGate {
    let Ok(tip) = chain.tip() else {
        return AppendGate::Refused("provenance.toml has no entries".into());
    };
    // ARMED once ANY entry records a run of either outcome: that is the proof the
    // enforcing tool is in use on this chain, and it is read out of the chain rather
    // than out of a constant.
    let armer = chain.entry.iter().position(|e| e.sound_strict().is_some());
    let Some(armer) = armer else {
        return AppendGate::Ratchet(format!(
            "ratchet: no entry in this chain records a strict run yet (tip `{}`, entry #{}), \
             so the strict-attestation rule is not yet in force. It arms permanently at the \
             first `refreeze --attest`, and from then on a refreeze cannot be built on top \
             of an entry whose strict suite never ran.",
            tip.name,
            chain.entry.len()
        ));
    };
    let armer_name = &chain.entry[armer].name;
    if tip.is_attested() {
        return AppendGate::Allowed;
    }
    if tip.is_red() {
        return AppendGate::NeedsSupersede(format!(
            "the tip `{}` (entry #{}) carries a RED strict run. Appending past it is an \
             abandonment: re-run with `--supersede-tip \"<why>\"` and this freeze will \
             record itself as the entry that replaces it.",
            tip.name,
            chain.entry.len()
        ));
    }
    AppendGate::Refused(format!(
        "the tip `{}` (entry #{}) carries no strict run, and entry #{} `{}` already \
         records one, so the strict-attestation rule is in force. A refreeze must not be \
         built on top of goldens whose strict suite never ran — that is exactly how chains \
         169 and 170 landed a stale constant. Run `refreeze --attest` first. If that run \
         comes back RED, it is recorded as such and `--supersede-tip` becomes available.",
        tip.name,
        chain.entry.len(),
        armer + 1,
        armer_name
    ))
}

/// The `asl-witness` sentinel: the root's `ab`. Any OTHER entry carrying it (or an empty
/// `ab`) while moving an anchor is a discipline violation.
pub const ASL_WITNESS: &str = "asl-witness";

/// Whether a string is a full 40-character lowercase-hex git SHA — the only shape
/// [`Entry::aeon_rev`] may carry. Abbreviations are rejected rather than normalized:
/// the emitter always has the full SHA, so a short one means something hand-edited it.
pub fn is_full_sha(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Chain {
    pub fn tip(&self) -> Result<&Entry, String> {
        self.entry.last().ok_or_else(|| "provenance.toml has no entries".to_string())
    }
}

/// Load + parse `golden/provenance.toml` from a golden directory. The one-liner the
/// gates use to source their expected CRC/size/anchor_end from the chain tip.
pub fn load(golden_dir: &Path) -> Result<Chain, String> {
    let path = golden_dir.join("provenance.toml");
    let src = std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    parse(&src)
}

/// The tip's target for a key, or an error naming the miss. `full_crc`/`anchor_crc` are
/// hex strings; use [`hex_u32`] to compare against a computed CRC.
pub fn tip_target(golden_dir: &Path, key: &str) -> Result<Target, String> {
    let chain = load(golden_dir)?;
    let tip = chain.tip()?;
    tip.targets
        .get(key)
        .cloned()
        .ok_or_else(|| format!("provenance tip `{}` has no target `{key}`", tip.name))
}

/// Parse a hex CRC string (no `0x`) to u32.
pub fn hex_u32(s: &str) -> Result<u32, String> {
    u32::from_str_radix(s, 16).map_err(|e| format!("bad hex crc `{s}`: {e}"))
}

/// Parse a `provenance.toml` source string.
pub fn parse(src: &str) -> Result<Chain, String> {
    let chain: Chain = toml::from_str(src).map_err(|e| format!("provenance.toml parse: {e}"))?;
    if chain.entry.is_empty() {
        return Err("provenance.toml has no [[entry]] blocks".into());
    }
    if chain.entry[0].ab != ASL_WITNESS {
        return Err(format!(
            "root entry `{}` must carry ab = \"{ASL_WITNESS}\" (got \"{}\")",
            chain.entry[0].name, chain.entry[0].ab
        ));
    }
    Ok(chain)
}

/// Recompute a target's frozen CRC set from its committed golden blob at a KNOWN
/// `anchor_end`. Used both to validate the tip (anchor_end read from the chain) and to
/// freeze a new tip (anchor_end read from the authoritative pins / size tables).
pub fn recompute_target(golden_dir: &Path, golden: &str, anchor_end: usize) -> Result<(String, usize, String), String> {
    let path = golden_dir.join(golden);
    let bytes = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if bytes.len() < anchor_end {
        return Err(format!(
            "{golden}: blob len {} shorter than anchor_end {anchor_end:#x}",
            bytes.len()
        ));
    }
    let full_crc = format!("{:08x}", crate::native::crc32(&bytes));
    let anchor_crc = format!("{:08x}", crate::native::assembled_anchor_crc(&bytes, anchor_end));
    Ok((full_crc, bytes.len(), anchor_crc))
}

/// Rebuild the per-target set for a fresh freeze from the committed blobs. `golden_map`
/// gives target-key -> golden filename; `anchor_ends` gives target-key -> EndOfRom from
/// the AUTHORITATIVE sources (pins.rs for canonical, `offcanonical_sizes/*.txt` headers
/// for the off-canonical shapes) so a size-changing optimization re-anchors correctly.
pub fn recompute_targets(
    golden_dir: &Path,
    golden_map: &BTreeMap<String, String>,
    anchor_ends: &BTreeMap<String, usize>,
) -> Result<BTreeMap<String, Target>, String> {
    let mut out = BTreeMap::new();
    for (key, golden) in golden_map {
        let end = *anchor_ends
            .get(key)
            .ok_or_else(|| format!("no authoritative anchor_end for target `{key}`"))?;
        let (full_crc, full_size, anchor_crc) = recompute_target(golden_dir, golden, end)?;
        out.insert(
            key.clone(),
            Target { golden: golden.clone(), full_crc, full_size, anchor_crc, anchor_end: end },
        );
    }
    Ok(out)
}

/// Validate a loaded chain against the committed golden blobs. Returns every violation
/// (empty = green). Two classes:
///   * TIP-MATCH — recomputed blob CRCs vs the tip entry.
///   * ANCHOR-MOVE-NEEDS-A/B — swept across EVERY consecutive pair (the whole chain stays
///     disciplined, not just the tip).
pub fn check(golden_dir: &Path, chain: &Chain) -> Vec<String> {
    let mut errs = Vec::new();

    // (3) AEON-REV-WELL-FORMED — POSITION-INDEPENDENT. An entry that carries the key at
    // all must carry a full 40-char SHA. `render_entry` always writes the key and
    // `--freeze` refuses without a vetted SHA, so a present-but-malformed value can only
    // be a hand edit, and that is equally wrong at entry #3 or #300.
    for (i, e) in chain.entry.iter().enumerate() {
        if let Some(rev) = &e.aeon_rev {
            if !is_full_sha(rev) {
                errs.push(format!(
                    "entry #{} `{}`: aeon_rev = \"{rev}\" is present but is not a full \
                     40-char lowercase-hex SHA (aeon-rev-well-formed)",
                    i + 1,
                    e.name
                ));
            }
        }
    }

    // (4) AEON-REV-MONOTONIC — DERIVED, not pinned. Once any entry names its aeon
    // revision, every later entry must too.
    //
    // The boundary is read out of the chain rather than hardcoded, and that is a
    // correctness property, not a tidiness one. A pinned "entries after #166" constant
    // has a MERGE RACE: the aeon lane refreezes by running this crate's `refreeze` out
    // of sigil master, so a byte-moving refreeze landing before the field does appends a
    // field-less entry #167 that was entirely legitimate when written — and the pinned
    // rule would then turn master red on somebody else's correct work, which is exactly
    // the failure this whole module exists to prevent, aimed at ourselves. Derived, that
    // entry is simply still pre-field and passes, while an entry appended after the
    // field is armed still cannot drop it.
    if let Some(first) = chain.entry.iter().position(|e| e.aeon_rev.as_deref().is_some_and(is_full_sha))
    {
        let armed = &chain.entry[first];
        for (i, e) in chain.entry.iter().enumerate().skip(first + 1) {
            if e.aeon_rev.is_none() {
                errs.push(format!(
                    "entry #{} `{}`: no aeon_rev, but entry #{} `{}` already names one — \
                     once the chain records the aeon revision it cannot stop \
                     (aeon-rev-monotonic)",
                    i + 1,
                    e.name,
                    first + 1,
                    armed.name
                ));
            }
        }
    }

    // (5) STRICT-ATTEST-WELL-FORMED — POSITION-INDEPENDENT, exactly as (3) is. A record
    // is written only by `refreeze --attest` from a run it performed itself, so a
    // malformed one can only be a hand edit, and that is equally wrong anywhere in the
    // chain. The cross-checks are against the OWNING entry's own targets and aeon_rev,
    // so they are derived rather than copied.
    for (i, e) in chain.entry.iter().enumerate() {
        if let Some(st) = &e.strict {
            for d in st.defects(&e.targets, e.aeon_rev.as_deref()) {
                errs.push(format!(
                    "entry #{} `{}`: strict {d} (strict-attest-well-formed)",
                    i + 1,
                    e.name
                ));
            }
        }
        if let Some(sup) = &e.superseded {
            let n = i + 1;
            if sup.by.trim().is_empty() {
                errs.push(format!(
                    "entry #{n} `{}`: superseded.by is empty — abandoning an entry must NAME \
                     the entry that replaced it (superseded-well-formed)",
                    e.name
                ));
            }
            match chain.entry.get(i + 1) {
                None => errs.push(format!(
                    "entry #{n} `{}`: is the TIP but claims to be superseded by `{}` — \
                     nothing follows it (superseded-well-formed)",
                    e.name, sup.by
                )),
                Some(next) if next.name != sup.by => errs.push(format!(
                    "entry #{n} `{}`: superseded.by = \"{}\" but the entry that actually \
                     follows it is `{}` (superseded-well-formed)",
                    e.name, sup.by, next.name
                )),
                Some(_) => {}
            }
            // THE ANTI-EVASION GUARD. Without it, abandonment is the cheap way out and
            // the whole ratchet dissolves: freeze, supersede, freeze, supersede, and the
            // strict suite never runs again. A red run is what makes it expensive.
            if !e.is_red() {
                errs.push(format!(
                    "entry #{n} `{}`: is marked superseded but carries no RED strict run. \
                     An entry may only be abandoned when its strict suite actually came \
                     back red and `refreeze --attest` recorded that (superseded-well-formed)",
                    e.name
                ));
            }
        }
    }

    // (6) STRICT-ATTEST-MONOTONIC — DERIVED, not pinned, and with ONE exemption: the
    // TIP. The strict run necessarily happens AFTER a freeze (the freeze is what moves
    // the goldens the suite then checks), so a tip awaiting its run is the normal state
    // and gating it would deadlock. Every entry the chain has been BUILT ON must have
    // been proven, though — that is the rule chains 169 and 170 broke.
    //
    // On the merge race a constant would create here, and on why the boundary is read
    // out of the chain instead, see [`append_gate`].
    if let Some(armer) = chain.entry.iter().position(|e| e.sound_strict().is_some()) {
        let last = chain.entry.len().saturating_sub(1);
        for (i, e) in chain.entry.iter().enumerate().take(last).skip(armer) {
            if e.is_attested() || e.superseded.is_some() {
                continue;
            }
            errs.push(format!(
                "entry #{} `{}`: was built on by entry #{} but records no passing strict \
                 run and is not superseded — entry #{} `{}` already records a run, so the \
                 rule is in force (strict-attest-monotonic)",
                i + 1,
                e.name,
                i + 2,
                armer + 1,
                chain.entry[armer].name
            ));
        }
    }

    // (2) chain discipline: an anchor that differs from the prior entry's same target
    // forces the newer entry to carry a real A/B ref.
    for pair in chain.entry.windows(2) {
        let (prev, cur) = (&pair[0], &pair[1]);
        let cur_needs_ab = cur.ab.trim().is_empty() || cur.ab == ASL_WITNESS;
        for (key, t) in &cur.targets {
            if let Some(pt) = prev.targets.get(key) {
                if pt.anchor_crc != t.anchor_crc && cur_needs_ab {
                    errs.push(format!(
                        "entry `{}`: target `{key}` anchor moved {} -> {} but ab=\"{}\" carries no A/B evidence (anchor-move-needs-A/B)",
                        cur.name, pt.anchor_crc, t.anchor_crc, cur.ab
                    ));
                }
            }
        }
    }

    // (1) tip-match: the committed blobs ARE the tip.
    let tip = match chain.tip() {
        Ok(t) => t,
        Err(e) => {
            errs.push(e);
            return errs;
        }
    };
    for (key, t) in &tip.targets {
        match recompute_target(golden_dir, &t.golden, t.anchor_end) {
            Ok((full_crc, full_size, anchor_crc)) => {
                if full_crc != t.full_crc {
                    errs.push(format!(
                        "tip `{}`: target `{key}` ({}) full_crc {} != recomputed {full_crc} (re-freeze?)",
                        tip.name, t.golden, t.full_crc
                    ));
                }
                if full_size != t.full_size {
                    errs.push(format!(
                        "tip `{}`: target `{key}` ({}) full_size {} != actual {full_size}",
                        tip.name, t.golden, t.full_size
                    ));
                }
                if anchor_crc != t.anchor_crc {
                    errs.push(format!(
                        "tip `{}`: target `{key}` ({}) anchor_crc {} != recomputed {anchor_crc} (re-freeze?)",
                        tip.name, t.golden, t.anchor_crc
                    ));
                }
            }
            Err(e) => errs.push(e),
        }
    }
    errs
}

/// Render one `[[entry]]` block (append-only text) in the file's house style. Kept as
/// text rendering (not `toml::to_string` of the whole chain) so appending preserves the
/// header comments and every existing entry verbatim.
pub fn render_entry(
    name: &str,
    ab: &str,
    aeon_rev: &str,
    note: &str,
    targets: &BTreeMap<String, Target>,
) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "\n[[entry]]");
    let _ = writeln!(s, "name = {}", toml_str(name));
    let _ = writeln!(s, "ab = {}", toml_str(ab));
    // UNCONDITIONAL, unlike `note`. An entry whose aeon_rev is absent is exactly the
    // ambiguity this field exists to close, so an empty one is written out visibly and
    // fails `check`'s aeon-rev-present rule rather than vanishing from the file.
    let _ = writeln!(s, "aeon_rev = {}", toml_str(aeon_rev));
    if !note.is_empty() {
        let _ = writeln!(s, "note = {}", toml_str(note));
    }
    for (key, t) in targets {
        // The one interpolation that is NOT a string value: a TOML table header, whose
        // key must be bare. [`fault_in_key`] is what keeps it one, and [`entry_faults`]
        // runs it before any caller reaches this renderer.
        let _ = writeln!(s, "\n[entry.targets.{key}]");
        let _ = writeln!(s, "golden = {}", toml_str(&t.golden));
        let _ = writeln!(s, "full_crc = {}", toml_str(&t.full_crc));
        let _ = writeln!(s, "full_size = {}", t.full_size);
        let _ = writeln!(s, "anchor_crc = {}", toml_str(&t.anchor_crc));
        let _ = writeln!(s, "anchor_end = {:#x}", t.anchor_end);
    }
    s
}

/// The character class a one-line ledger field cannot show its reader verbatim: any
/// control character, the raw newline included.
///
/// This is the REFUSAL half of the two-layer contract; [`toml_str`] is the other. A
/// quote or a backslash is escaped, round-trips exactly, and reads correctly in the
/// file, so it is accepted. A newline does not: TOML would carry it as the two
/// characters `\n`, and the sentence the ledger shows is then not the sentence its
/// author wrote. Refusing names the character and its position so the author can fix
/// their own prose, which is what an escape would silently deny them.
pub fn fault_in_prose(field: &str, v: &str) -> Option<String> {
    for (i, c) in v.char_indices() {
        if c.is_control() {
            return Some(format!(
                "{field}: {} at byte {i} cannot appear in a one-line ledger field \
                 (in `{}`)",
                name_of_control(c),
                v.escape_debug()
            ));
        }
    }
    None
}

/// Human name for a control character, so a refusal reads as prose rather than as a
/// codepoint the author has to look up.
fn name_of_control(c: char) -> String {
    match c {
        '\n' => "a newline (U+000A)".to_string(),
        '\r' => "a carriage return (U+000D)".to_string(),
        '\t' => "a tab (U+0009)".to_string(),
        '\0' => "a NUL (U+0000)".to_string(),
        c => format!("a control character (U+{:04X})", c as u32),
    }
}

/// Whether a target key is a TOML BARE key, which is what `[entry.targets.<key>]`
/// requires. A key outside `A-Za-z0-9_-` would need quoting in the header, and an empty
/// one has no header at all.
pub fn fault_in_key(key: &str) -> Option<String> {
    if key.is_empty() {
        return Some("target key is empty; `[entry.targets.]` is not a table header".to_string());
    }
    for (i, c) in key.char_indices() {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Some(format!(
                "target key `{key}`: `{c}` at byte {i} is not allowed in a TOML bare key \
                 (A-Za-z0-9_-)"
            ));
        }
    }
    None
}

/// Every reason an `[[entry]]` built from these values could not be written FAITHFULLY,
/// gathered before anything touches the file.
///
/// Callers refuse on a non-empty result. The point is the ORDER: a caller that checks
/// here has not yet rendered, not yet appended and not yet written, so the ledger on
/// disk is untouched and the author still has their sentence to correct.
pub fn entry_faults(
    name: &str,
    ab: &str,
    aeon_rev: &str,
    note: &str,
    targets: &BTreeMap<String, Target>,
) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(fault_in_prose("name", name));
    out.extend(fault_in_prose("ab", ab));
    out.extend(fault_in_prose("aeon_rev", aeon_rev));
    out.extend(fault_in_prose("note", note));
    for (key, t) in targets {
        out.extend(fault_in_key(key));
        out.extend(fault_in_prose(&format!("targets.{key}.golden"), &t.golden));
        out.extend(fault_in_prose(&format!("targets.{key}.full_crc"), &t.full_crc));
        out.extend(fault_in_prose(&format!("targets.{key}.anchor_crc"), &t.anchor_crc));
    }
    out
}

/// Escape a value for a TOML basic string. EVERY string this module writes goes through
/// here, with no exception for fields that "only ever hold" a SHA or a filename: the
/// ledger's prose fields exist to carry human sentences, a sentence may contain a quote,
/// and a basic string ends at the first unescaped one.
fn toml_str(v: &str) -> String {
    let mut out = String::with_capacity(v.len() + 2);
    out.push('"');
    for c in v.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn toml_array(v: &[String]) -> String {
    let items: Vec<String> = v.iter().map(|s| toml_str(s)).collect();
    format!("[{}]", items.join(", "))
}

/// Render the `[entry.strict]` block for the chain's LAST entry.
///
/// APPEND-ONLY, like everything else in this file, and that is not a stylistic choice.
/// In TOML a `[entry.strict]` table written at the end of the file attaches to the last
/// `[[entry]]` already there — so recording a strict run needs no surgery into the
/// middle of the file, rewrites no existing entry, and leaves all ~172 predecessors
/// byte-identical. The same property lets `--freeze --supersede-tip` write
/// [`render_superseded`] and then a whole new `[[entry]]`, in that order, as one append.
pub fn render_strict(s: &StrictRun) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "\n[entry.strict]");
    let _ = writeln!(out, "outcome = {}", toml_str(&s.outcome));
    let _ = writeln!(out, "sigil_rev = {}", toml_str(&s.sigil_rev));
    let _ = writeln!(out, "aeon_rev = {}", toml_str(&s.aeon_rev));
    let _ = writeln!(out, "strict_bodies = {}", s.strict_bodies);
    let _ = writeln!(out, "suites = {}", s.suites);
    let _ = writeln!(out, "passed = {}", s.passed);
    let _ = writeln!(out, "failed = {}", s.failed);
    let _ = writeln!(out, "ignored = {}", s.ignored);
    let _ = writeln!(out, "skips = {}", s.skips);
    let _ = writeln!(out, "ran_at = {}", toml_str(&s.ran_at));
    if !s.failing.is_empty() {
        let _ = writeln!(out, "failing = {}", toml_array(&s.failing));
    }
    if !s.expected_tests.is_empty() {
        let _ = writeln!(out, "expected_tests = {}", toml_array(&s.expected_tests));
    }
    // A sub-table, so it must follow every scalar key of `[entry.strict]`.
    let _ = writeln!(out, "\n[entry.strict.goldens]");
    for (k, v) in &s.goldens {
        let _ = writeln!(out, "{k} = {}", toml_str(v));
    }
    out
}

/// Render the `[entry.superseded]` block for the chain's LAST entry — written by
/// `--freeze --supersede-tip` immediately before the successor's own `[[entry]]` block,
/// which is what makes the successor's name true by construction.
///
/// A TABLE rather than a bare key, deliberately: a bare `superseded_by = …` appended at
/// end-of-file would attach to whatever table happens to be last, which after
/// [`render_strict`] is `[entry.strict.goldens]` — silently recording the abandonment on
/// the wrong table.
pub fn render_superseded(s: &Superseded) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "\n[entry.superseded]");
    let _ = writeln!(out, "by = {}", toml_str(&s.by));
    let _ = writeln!(out, "reason = {}", toml_str(&s.reason));
    out
}

/// Append `block` to the provenance text `existing` and install the result at `path`.
///
/// The ledger is only ever replaced by text that PARSES. Validation runs against the
/// in-memory result first, and a parse error returns `Err` with the file on disk
/// untouched — so a run that reports an error always leaves a `provenance.toml` that
/// still parses. That ordering is the whole contract: an authoritative-looking ledger
/// that no longer parses fails every later `--check`, `--attest` and byte gate with a
/// line number for a cause, and the file is the only copy.
///
/// The install itself is a write to a sibling temporary followed by a rename, which is
/// atomic within the directory: an interrupted write cannot leave the ledger truncated.
/// That is a SEPARATE and smaller guarantee than the one above — a rename installs
/// whatever it is handed, valid or not, so it protects against interruption and against
/// nothing else.
pub fn append_block(path: &Path, existing: &str, block: &str) -> Result<(String, Chain), String> {
    let mut new_src = existing.to_string();
    if !new_src.ends_with('\n') {
        new_src.push('\n');
    }
    new_src.push_str(block);
    let chain = parse(&new_src)?;
    write_atomic(path, &new_src)?;
    Ok((new_src, chain))
}

/// Replace `path`'s contents with `contents` by rename, never by truncation.
///
/// The temporary is a SIBLING of the target. A rename is only atomic within one
/// filesystem, and the one directory guaranteed to share the target's filesystem is its
/// own. The data is flushed before the rename so the installed name never resolves to
/// content the kernel has not yet written.
pub fn write_atomic(path: &Path, contents: &str) -> Result<(), String> {
    use std::io::Write;
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    let stem = path
        .file_name()
        .ok_or_else(|| format!("{} names no file", path.display()))?
        .to_string_lossy()
        .into_owned();
    let tmp = dir.join(format!(".{stem}.{}.tmp", std::process::id()));

    let install = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    };
    match install() {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(format!("write {}: {e}", path.display()))
        }
    }
}

/// Whether a freshly recomputed target set is IDENTICAL to the current tip — the
/// fixpoint predicate: a re-freeze that produces the tip again is a no-op (append
/// nothing).
pub fn equals_tip(chain: &Chain, fresh: &BTreeMap<String, Target>) -> bool {
    match chain.tip() {
        Ok(tip) => &tip.targets == fresh,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(golden: &str, full: &str, size: usize, anchor: &str, end: usize) -> Target {
        Target {
            golden: golden.into(),
            full_crc: full.into(),
            full_size: size,
            anchor_crc: anchor.into(),
            anchor_end: end,
        }
    }

    #[test]
    fn root_ab_sentinel_required() {
        let src = "[[entry]]\nname=\"root\"\nab=\"nope\"\n[entry.targets.s4]\ngolden=\"s4.bin\"\nfull_crc=\"0\"\nfull_size=1\nanchor_crc=\"0\"\nanchor_end=1\n";
        assert!(parse(src).is_err(), "non-sentinel root ab must be rejected");
    }

    #[test]
    fn anchor_move_without_ab_is_flagged() {
        let mut root_t = BTreeMap::new();
        root_t.insert("s4".to_string(), t("s4.bin", "aaaa", 10, "1111", 8));
        let mut moved_t = BTreeMap::new();
        // anchor changed 1111 -> 2222, but ab is the sentinel -> violation.
        moved_t.insert("s4".to_string(), t("s4.bin", "bbbb", 10, "2222", 8));
        let chain = Chain {
            entry: vec![
                Entry { name: "root".into(), ab: ASL_WITNESS.into(), aeon_rev: None, note: String::new(), strict: None, superseded: None, targets: root_t },
                Entry { name: "bad".into(), ab: ASL_WITNESS.into(), aeon_rev: None, note: String::new(), strict: None, superseded: None, targets: moved_t },
            ],
        };
        // discipline sweep alone (no blobs) should flag the moved anchor.
        let errs = check(Path::new("/nonexistent"), &chain);
        assert!(
            errs.iter().any(|e| e.contains("anchor-move-needs-A/B")),
            "expected anchor-move-needs-A/B violation, got: {errs:?}"
        );
    }

    #[test]
    fn anchor_move_with_ab_ok_full_crc_free() {
        let mut root_t = BTreeMap::new();
        root_t.insert("s4".to_string(), t("s4.bin", "aaaa", 10, "1111", 8));
        let mut moved_t = BTreeMap::new();
        moved_t.insert("s4".to_string(), t("s4.bin", "bbbb", 10, "2222", 8));
        let chain = Chain {
            entry: vec![
                Entry { name: "root".into(), ab: ASL_WITNESS.into(), aeon_rev: None, note: String::new(), strict: None, superseded: None, targets: root_t },
                Entry { name: "g9".into(), ab: "notes/g9-ab.md".into(), aeon_rev: None, note: String::new(), strict: None, superseded: None, targets: moved_t },
            ],
        };
        let errs = check(Path::new("/nonexistent"), &chain);
        // no discipline violation (full_crc moved freely; anchor move carries ab).
        assert!(
            !errs.iter().any(|e| e.contains("anchor-move-needs-A/B")),
            "anchor move with a real ab must be allowed: {errs:?}"
        );
    }

    #[test]
    fn render_roundtrips() {
        let mut targets = BTreeMap::new();
        targets.insert("s4".to_string(), t("s4.bin", "7f071417", 412306, "e5765873", 0x5db60));
        let block = render_entry("p", "ref", SHA_A, "n", &targets);
        // raw deserialize (bypasses parse()'s root-sentinel guard — this is a lone entry).
        let chain: Chain = toml::from_str(&block).unwrap();
        assert_eq!(chain.entry[0].targets["s4"].anchor_end, 0x5db60);
        assert_eq!(chain.entry[0].ab, "ref");
        assert_eq!(chain.entry[0].aeon_rev.as_deref(), Some(SHA_A));
        assert_eq!(chain.entry[0].targets["s4"].full_crc, "7f071417");
    }

    // ── aeon_rev ────────────────────────────────────────────────────────────────

    const SHA_A: &str = "55ea25576c7e523a8982d1a4175e1effaccb3570";
    const SHA_B: &str = "0123456789abcdef0123456789abcdef01234567";

    /// Build a chain from a list of per-entry `aeon_rev` values. `None` models an entry
    /// whose KEY IS ABSENT (appended by a refreeze predating the field); `Some` models
    /// one that carries it. Entry #1 is the root. All targets are identical, so the
    /// anchor-move rule stays quiet and only the aeon_rev rules can fire.
    fn chain_with(revs: &[Option<&str>]) -> Chain {
        let mut targets = BTreeMap::new();
        targets.insert("s4".to_string(), t("s4.bin", "aaaa", 10, "1111", 8));
        let entry = revs
            .iter()
            .enumerate()
            .map(|(i, r)| Entry {
                name: format!("e{}", i + 1),
                ab: if i == 0 { ASL_WITNESS.into() } else { "ref".into() },
                aeon_rev: r.map(|s| s.to_string()),
                note: String::new(),
                strict: None,
                superseded: None,
                targets: targets.clone(),
            })
            .collect();
        Chain { entry }
    }

    fn aeon_rev_errs(chain: &Chain) -> Vec<String> {
        check(Path::new("/nonexistent"), chain)
            .into_iter()
            .filter(|e| e.contains("aeon-rev-"))
            .collect()
    }

    #[test]
    fn absent_aeon_rev_key_parses_as_none() {
        // The 166 committed entries carry no `aeon_rev` key at all. serde(default) is
        // what keeps them parsing; without it every one of them is a parse error. The
        // value must be None (key absent), NOT Some("") — the two mean different things.
        let src = "[[entry]]\nname=\"root\"\nab=\"asl-witness\"\n[entry.targets.s4]\ngolden=\"s4.bin\"\nfull_crc=\"0\"\nfull_size=1\nanchor_crc=\"0\"\nanchor_end=1\n";
        let chain = parse(src).expect("a keyless historical entry must still parse");
        assert_eq!(chain.entry[0].aeon_rev, None, "absent key must be None, not Some(\"\")");
    }

    #[test]
    fn a_chain_with_no_aeon_rev_anywhere_is_clean() {
        // Today's file: 166 entries, none carrying the field. No backfill, no violation.
        let chain = chain_with(&[None, None, None]);
        assert!(aeon_rev_errs(&chain).is_empty(), "pre-field entries must not be flagged");
    }

    /// THE MERGE-RACE CASE. DO NOT "TIDY" THIS AWAY.
    ///
    /// The aeon lane refreezes by running this crate's `refreeze` out of sigil master. A
    /// byte-moving refreeze that lands BEFORE the aeon_rev field does appends an entry
    /// with no such key — entirely legitimate when written. A rule pinned to "entries
    /// after #166" would turn master red on that entry the moment this branch merged.
    /// The derived rule must let it through, because nothing before it names a revision
    /// either.
    #[test]
    fn a_pre_field_entry_appended_after_the_tip_is_not_a_violation() {
        let chain = chain_with(&[None, None, None, None]);
        assert!(
            aeon_rev_errs(&chain).is_empty(),
            "a pre-field entry appended by an older refreeze must pass — this is the \
             merge race the derived boundary exists to survive"
        );
    }

    #[test]
    fn the_first_entry_to_name_a_revision_needs_nothing_before_it() {
        // Arming the ratchet is legal at any point: everything before is pre-field.
        let chain = chain_with(&[None, None, Some(SHA_A)]);
        assert!(aeon_rev_errs(&chain).is_empty(), "arming the chain must be allowed");
    }

    #[test]
    fn an_entry_after_the_first_armed_one_may_not_drop_the_field() {
        let chain = chain_with(&[None, Some(SHA_A), None]);
        let errs = aeon_rev_errs(&chain);
        assert_eq!(errs.len(), 1, "the dropped field must be flagged: {errs:?}");
        assert!(errs[0].contains("aeon-rev-monotonic"), "wrong rule fired: {}", errs[0]);
        assert!(errs[0].contains("entry #3"), "must name the offender: {}", errs[0]);
        assert!(errs[0].contains("entry #2"), "must name the armer: {}", errs[0]);
    }

    #[test]
    fn every_entry_after_the_armed_one_is_checked_not_just_the_tip() {
        let chain = chain_with(&[Some(SHA_A), None, None]);
        assert_eq!(
            aeon_rev_errs(&chain).len(),
            2,
            "the sweep covers every later entry, not only the last"
        );
    }

    #[test]
    fn a_chain_that_keeps_naming_revisions_is_clean() {
        let chain = chain_with(&[None, Some(SHA_A), Some(SHA_B), Some(SHA_A)]);
        assert!(aeon_rev_errs(&chain).is_empty(), "a fully armed chain must be clean");
    }

    /// Position-INDEPENDENT: a present-but-malformed value is a hand edit and is wrong
    /// anywhere, including before anything else has armed the chain. The pinned-constant
    /// form could not say this — it only looked past #166.
    #[test]
    fn a_present_but_malformed_aeon_rev_is_flagged_anywhere() {
        for bad in ["", &SHA_A[..8], &SHA_A.to_uppercase(), &"g".repeat(40)] {
            let chain = chain_with(&[Some(bad), None]);
            let errs = aeon_rev_errs(&chain);
            assert!(
                errs.iter().any(|e| e.contains("aeon-rev-well-formed")),
                "aeon_rev = {bad:?} must be rejected wherever it sits, got: {errs:?}"
            );
        }
    }

    #[test]
    fn a_malformed_value_does_not_arm_the_monotonic_rule() {
        // Some("") is malformed, so it is not a revision; the entries after it are still
        // pre-field and must not be dragged into a monotonic violation on top of the
        // well-formedness one. One fault, one error.
        let chain = chain_with(&[Some(""), None, None]);
        let errs = aeon_rev_errs(&chain);
        assert_eq!(errs.len(), 1, "one fault must produce one error, got: {errs:?}");
        assert!(errs[0].contains("aeon-rev-well-formed"), "{}", errs[0]);
    }

    // ── strict attestation ──────────────────────────────────────────────────────

    /// A well-formed green record for a chain built by [`chain_with`] — whose single
    /// target is always `t("s4.bin", "aaaa", 10, "1111", 8)`, so the golden identity is
    /// DERIVED from that fixture rather than copied from a neighbouring literal.
    fn run(outcome: &str, aeon: &str) -> StrictRun {
        let target = t("s4.bin", "aaaa", 10, "1111", 8);
        let mut goldens = BTreeMap::new();
        goldens.insert("s4".to_string(), format!("{}/{}", target.full_crc, target.full_size));
        StrictRun {
            outcome: outcome.to_string(),
            sigil_rev: SHA_B.to_string(),
            aeon_rev: aeon.to_string(),
            strict_bodies: 137,
            suites: 41,
            passed: 3881,
            failed: if outcome == OUTCOME_FAILED { 2 } else { 0 },
            ignored: 4,
            skips: 0,
            ran_at: "unix:0".into(),
            failing: if outcome == OUTCOME_FAILED {
                vec!["a::b".into(), "c::d".into()]
            } else {
                vec![]
            },
            expected_tests: vec![],
            goldens,
        }
    }

    fn green() -> StrictRun {
        run(OUTCOME_PASSED, SHA_A)
    }
    fn red() -> StrictRun {
        run(OUTCOME_FAILED, SHA_A)
    }

    /// A chain whose entries all name `SHA_A` as their aeon_rev, with per-entry strict
    /// records supplied by the caller.
    fn attested_chain(records: &[Option<StrictRun>]) -> Chain {
        let mut chain = chain_with(&vec![Some(SHA_A); records.len()]);
        for (e, r) in chain.entry.iter_mut().zip(records) {
            e.strict = r.clone();
        }
        chain
    }

    fn strict_errs(chain: &Chain) -> Vec<String> {
        check(Path::new("/nonexistent"), chain)
            .into_iter()
            .filter(|e| e.contains("strict-attest-") || e.contains("superseded-well-formed"))
            .collect()
    }

    #[test]
    fn absent_strict_key_parses_as_none() {
        // The ~172 committed entries carry neither key. serde(default) on BOTH is what
        // keeps them parsing; without it every one of them is a parse error.
        let src = "[[entry]]\nname=\"root\"\nab=\"asl-witness\"\n[entry.targets.s4]\ngolden=\"s4.bin\"\nfull_crc=\"0\"\nfull_size=1\nanchor_crc=\"0\"\nanchor_end=1\n";
        let chain = parse(src).expect("a historical entry must still parse");
        assert_eq!(chain.entry[0].strict, None, "absent strict key must be None");
        assert_eq!(chain.entry[0].superseded, None, "absent superseded key must be None");
    }

    #[test]
    fn a_chain_with_no_strict_record_anywhere_is_clean() {
        // Today's file: no entry carries a record. No backfill, no violation.
        let chain = attested_chain(&[None, None, None]);
        assert!(strict_errs(&chain).is_empty(), "pre-field entries must not be flagged");
    }

    /// THE MERGE-RACE CASE for this field, and it is LIVE: the aeon lane refreezes by
    /// running this crate's `refreeze` out of sigil master, and lands paired freezes on
    /// the same nights this branch is in flight. A refreeze from a revision predating
    /// this field appends a record-less entry that was entirely legitimate when written.
    /// Because the tip is exempt and the boundary is derived, it passes.
    #[test]
    fn a_pre_field_entry_appended_after_an_attested_tip_is_not_a_violation() {
        let chain = attested_chain(&[None, Some(green()), None]);
        assert!(
            strict_errs(&chain).is_empty(),
            "an entry appended by an older refreeze on top of an attested tip must pass — \
             it is the tip, and its strict run has not happened yet"
        );
    }

    #[test]
    fn the_tip_is_exempt_from_the_monotonic_rule() {
        // The strict run necessarily happens AFTER the freeze that created the tip, so a
        // tip awaiting its run is the NORMAL state. Gating it would deadlock the chain.
        let chain = attested_chain(&[Some(green()), Some(green()), None]);
        assert!(strict_errs(&chain).is_empty(), "the tip may always be awaiting its run");
    }

    #[test]
    fn an_entry_that_was_built_on_without_a_strict_run_is_flagged() {
        // THE INCIDENT SHAPE: entry #2 was refrozen on top of and never ran strict.
        let chain = attested_chain(&[Some(green()), None, None]);
        let errs = strict_errs(&chain);
        assert_eq!(errs.len(), 1, "exactly the built-on entry must be flagged: {errs:?}");
        assert!(errs[0].contains("strict-attest-monotonic"), "wrong rule: {}", errs[0]);
        assert!(errs[0].contains("entry #2"), "must name the offender: {}", errs[0]);
    }

    #[test]
    fn every_built_on_entry_after_the_armer_is_checked_not_just_the_last() {
        let chain = attested_chain(&[Some(green()), None, None, None]);
        assert_eq!(strict_errs(&chain).len(), 2, "entries #2 and #3 are both built on");
    }

    // ── what makes a record honest ──────────────────────────────────────────────

    /// THE VACUITY CHECK, and the single reason the witness mechanism exists.
    ///
    /// A suite run WITHOUT `SIGIL_STRICT_GATE=1` early-returns every `strict_gate()`
    /// body and is nevertheless fully green — identical pass counts, identical exit
    /// code. `strict_bodies` is the only quantity that can tell the two apart, because
    /// the only call that writes a witness line is one that already saw the flag set.
    /// Zero here is the signature of the run that let chains 169 and 170 through.
    #[test]
    fn a_record_of_a_run_that_reached_no_strict_body_is_rejected() {
        let mut r = green();
        r.strict_bodies = 0;
        let chain = attested_chain(&[Some(r)]);
        let errs = strict_errs(&chain);
        assert!(
            errs.iter().any(|e| e.contains("strict_bodies = 0")),
            "a run with no strict body is vacuous and must be rejected: {errs:?}"
        );
    }

    #[test]
    fn a_vacuous_record_does_not_satisfy_the_append_gate() {
        // One fault, one consequence: the malformed record raises its own error AND
        // leaves the tip unattested, rather than silently counting as a pass.
        let mut r = green();
        r.strict_bodies = 0;
        let chain = attested_chain(&[Some(r)]);
        assert!(!chain.entry[0].is_attested(), "a vacuous record is not an attestation");
    }

    /// ANTI-TRAVEL. The record's CRCs are read from the BLOBS at run time; the entry's
    /// were read from the blobs at freeze time. They agree exactly when the attestation
    /// is about this entry's artifacts, so a record copied onto another entry fails.
    #[test]
    fn a_record_whose_goldens_are_not_this_entrys_is_rejected() {
        let mut r = green();
        r.goldens.insert("s4".into(), "deadbeef/10".into());
        let chain = attested_chain(&[Some(r)]);
        let errs = strict_errs(&chain);
        assert!(
            errs.iter().any(|e| e.contains("different bytes")),
            "an attestation about other bytes must be rejected: {errs:?}"
        );
    }

    #[test]
    fn a_record_missing_a_target_the_entry_freezes_is_rejected() {
        let mut r = green();
        r.goldens.clear();
        let chain = attested_chain(&[Some(r)]);
        assert!(
            strict_errs(&chain).iter().any(|e| e.contains("goldens has no `s4`")),
            "a record that does not cover every frozen target must be rejected"
        );
    }

    #[test]
    fn a_record_naming_a_target_the_entry_does_not_freeze_is_rejected() {
        let mut r = green();
        r.goldens.insert("lean".into(), "aaaa/10".into());
        let chain = attested_chain(&[Some(r)]);
        assert!(
            strict_errs(&chain).iter().any(|e| e.contains("does not freeze")),
            "a record covering a target the entry does not name must be rejected"
        );
    }

    /// PAIRING. The only aeon tree whose ROMs match these goldens is the one they were
    /// frozen from, so a suite run against another revision is not a test of this entry.
    #[test]
    fn a_record_of_a_run_against_a_different_aeon_tree_is_rejected() {
        let chain = attested_chain(&[Some(run(OUTCOME_PASSED, SHA_B))]);
        let errs = strict_errs(&chain);
        assert!(
            errs.iter().any(|e| e.contains("different aeon tree")),
            "a run against the wrong aeon tree must be rejected: {errs:?}"
        );
    }

    #[test]
    fn an_unmeasurable_run_is_never_green() {
        for (label, mut r) in [
            ("suites", green()),
            ("passed", green()),
        ] {
            match label {
                "suites" => r.suites = 0,
                _ => r.passed = 0,
            }
            let chain = attested_chain(&[Some(r)]);
            let errs = strict_errs(&chain);
            assert!(
                errs.iter().any(|e| e.contains(&format!("{label} = 0"))),
                "{label} = 0 means the run could not be measured, which is not a pass: {errs:?}"
            );
        }
    }

    #[test]
    fn a_green_outcome_that_names_failures_is_incoherent() {
        let mut r = green();
        r.failed = 3;
        let chain = attested_chain(&[Some(r)]);
        assert!(strict_errs(&chain).iter().any(|e| e.contains("but failed = 3")));
    }

    #[test]
    fn a_red_outcome_with_nothing_red_is_incoherent() {
        let mut r = red();
        r.failed = 0;
        r.failing.clear();
        let chain = attested_chain(&[Some(r)]);
        assert!(strict_errs(&chain).iter().any(|e| e.contains("a red run must say what was red")));
    }

    #[test]
    fn a_record_with_a_bad_revision_is_rejected_wherever_it_sits() {
        for bad in ["", &SHA_A[..8], &SHA_A.to_uppercase(), &"g".repeat(40)] {
            let mut r = green();
            r.sigil_rev = bad.to_string();
            let chain = attested_chain(&[Some(r), None]);
            assert!(
                strict_errs(&chain).iter().any(|e| e.contains("sigil_rev")),
                "sigil_rev = {bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn a_malformed_record_does_not_arm_the_monotonic_rule() {
        // One fault, one error: the entries after a malformed record are still pre-field
        // and must not be dragged into a monotonic violation on top of the vacuity one.
        let mut r = green();
        r.strict_bodies = 0;
        let chain = attested_chain(&[Some(r), None, None]);
        let errs = strict_errs(&chain);
        assert_eq!(errs.len(), 1, "one fault must produce one error, got: {errs:?}");
        assert!(errs[0].contains("strict_bodies = 0"), "{}", errs[0]);
    }

    // ── the third state ─────────────────────────────────────────────────────────

    fn sup(by: &str) -> Superseded {
        Superseded { by: by.into(), reason: "the run was red".into() }
    }

    #[test]
    fn a_superseded_entry_satisfies_the_monotonic_rule() {
        // The deadlock case: entry #1's run was genuinely RED and the fix moved bytes.
        // Entry #2 replaces it. Nothing here may be red, or the honest operator's only
        // exits are a hand edit and a forged pass.
        let mut chain = attested_chain(&[Some(red()), Some(green()), None]);
        chain.entry[0].superseded = Some(sup(&chain.entry[1].name.clone()));
        assert!(
            strict_errs(&chain).is_empty(),
            "an abandoned entry must not deadlock the chain: {:?}",
            strict_errs(&chain)
        );
    }

    /// THE ANTI-EVASION GUARD. Without it, abandonment is the cheap way out and the
    /// ratchet dissolves: freeze, supersede, freeze, supersede, and the strict suite
    /// never runs again. Requiring a RED run means every abandonment costs the very run
    /// the mechanism is trying to force.
    #[test]
    fn an_entry_with_no_red_run_cannot_be_abandoned() {
        let mut chain = attested_chain(&[None, Some(green()), None]);
        chain.entry[0].superseded = Some(sup(&chain.entry[1].name.clone()));
        let errs = strict_errs(&chain);
        assert!(
            errs.iter().any(|e| e.contains("carries no RED strict run")),
            "abandoning without a red run must be refused: {errs:?}"
        );
    }

    #[test]
    fn a_green_entry_cannot_be_abandoned() {
        let mut chain = attested_chain(&[Some(green()), Some(green()), None]);
        chain.entry[0].superseded = Some(sup(&chain.entry[1].name.clone()));
        assert!(strict_errs(&chain).iter().any(|e| e.contains("carries no RED strict run")));
    }

    /// The successor's name is DERIVED from the chain, not taken on trust — so it cannot
    /// name a fiction, and abandonment is only reachable by actually performing the next
    /// freeze.
    #[test]
    fn superseded_must_name_the_entry_that_actually_follows() {
        let mut chain = attested_chain(&[Some(red()), Some(green()), None]);
        chain.entry[0].superseded = Some(sup("some-other-parcel"));
        let errs = strict_errs(&chain);
        assert!(
            errs.iter().any(|e| e.contains("the entry that actually follows it is `e2`")),
            "a fictional successor must be rejected: {errs:?}"
        );
    }

    #[test]
    fn the_tip_cannot_be_superseded() {
        let mut chain = attested_chain(&[Some(green()), Some(red())]);
        chain.entry[1].superseded = Some(sup("nothing"));
        assert!(strict_errs(&chain).iter().any(|e| e.contains("nothing follows it")));
    }

    #[test]
    fn an_empty_successor_name_is_rejected() {
        let mut chain = attested_chain(&[Some(red()), Some(green()), None]);
        chain.entry[0].superseded = Some(sup(""));
        assert!(strict_errs(&chain).iter().any(|e| e.contains("must NAME the entry")));
    }

    // ── the append gate ─────────────────────────────────────────────────────────

    #[test]
    fn the_append_gate_is_a_ratchet_while_no_entry_records_a_run() {
        let chain = attested_chain(&[None, None, None]);
        let AppendGate::Ratchet(m) = append_gate(&chain) else {
            panic!("an unarmed chain must ratchet, got {:?}", append_gate(&chain));
        };
        // `ratchet:`, NEVER `skip:` — this lane's strict bar requires zero `skip:` lines
        // and an unarmed rule is not a missing reference.
        assert!(m.starts_with("ratchet:"), "must be reported as a ratchet: {m}");
        assert!(!m.contains("skip:"), "must never use the skip sentinel: {m}");
    }

    #[test]
    fn the_append_gate_allows_an_attested_tip() {
        let chain = attested_chain(&[None, Some(green())]);
        assert_eq!(append_gate(&chain), AppendGate::Allowed);
    }

    /// THE ENFORCEMENT. Chains 169 and 170 are exactly this shape: a refreeze appended
    /// on top of an entry whose strict suite never ran.
    #[test]
    fn the_append_gate_refuses_to_build_on_an_entry_whose_suite_never_ran() {
        let chain = attested_chain(&[Some(green()), None]);
        let AppendGate::Refused(m) = append_gate(&chain) else {
            panic!("an armed rule must refuse an unattested tip, got {:?}", append_gate(&chain));
        };
        assert!(m.contains("carries no strict run"), "{m}");
        assert!(m.contains("--attest"), "the refusal must name the way out: {m}");
    }

    #[test]
    fn the_append_gate_demands_an_explicit_abandonment_when_the_tip_is_red() {
        let chain = attested_chain(&[Some(green()), Some(red())]);
        let AppendGate::NeedsSupersede(m) = append_gate(&chain) else {
            panic!("a red tip must demand a supersede, got {:?}", append_gate(&chain));
        };
        assert!(m.contains("--supersede-tip"), "{m}");
    }

    /// A red record ARMS the rule too. If only a green one did, a chain that had only
    /// ever recorded red runs would leave the gate unarmed — a hole exactly where the
    /// discipline is under most pressure.
    #[test]
    fn a_red_record_arms_the_rule() {
        let chain = attested_chain(&[Some(red()), None]);
        assert!(
            matches!(append_gate(&chain), AppendGate::Refused(_)),
            "a chain that has recorded any run is armed, got {:?}",
            append_gate(&chain)
        );
    }

    // ── rendering: the append-only property ─────────────────────────────────────

    /// THE STRUCTURAL CLAIM this whole design rests on. In TOML a `[entry.strict]` table
    /// written at the END of the file attaches to the LAST `[[entry]]` already there. So
    /// recording a run needs no surgery into the middle of the file, rewrites no
    /// existing entry, and leaves every predecessor byte-identical — the same discipline
    /// the chain already has for appends.
    #[test]
    fn an_appended_strict_table_attaches_to_the_last_entry_and_leaves_the_rest_untouched() {
        let mut targets = BTreeMap::new();
        targets.insert("s4".to_string(), t("s4.bin", "aaaa", 10, "1111", 8));
        let mut src = String::new();
        src.push_str(&render_entry("root", ASL_WITNESS, SHA_A, "", &targets));
        src.push_str(&render_entry("second", "ref", SHA_A, "", &targets));
        let before = parse(&src).expect("two-entry chain parses");

        let appended = format!("{src}{}", render_strict(&green()));
        let after = parse(&appended).expect("the appended table must still parse");

        assert_eq!(after.entry.len(), 2, "appending must not create an entry");
        assert_eq!(after.entry[0], before.entry[0], "entry #1 must be untouched");
        assert_eq!(
            after.entry[1].strict.as_ref().map(|s| s.strict_bodies),
            Some(137),
            "the table must attach to the LAST entry"
        );
        assert!(appended.starts_with(&src), "the write must be a pure append");
    }

    /// And a `[[entry]]` appended afterwards still starts a new entry, so `--attest` and
    /// a later `--freeze` compose.
    #[test]
    fn a_freeze_can_still_append_after_a_strict_table() {
        let mut targets = BTreeMap::new();
        targets.insert("s4".to_string(), t("s4.bin", "aaaa", 10, "1111", 8));
        let mut src = render_entry("root", ASL_WITNESS, SHA_A, "", &targets);
        src.push_str(&render_strict(&green()));
        src.push_str(&render_entry("next", "ref", SHA_A, "", &targets));
        let chain = parse(&src).unwrap();
        assert_eq!(chain.entry.len(), 2);
        assert!(chain.entry[0].strict.is_some(), "the record stayed on entry #1");
        assert!(chain.entry[1].strict.is_none(), "and did not leak onto entry #2");
    }

    /// `[entry.superseded]` must be written while the OLD tip is still last, and it must
    /// be a TABLE: a bare `superseded_by = …` appended at end-of-file would attach to
    /// whatever table happens to be last, which after `render_strict` is
    /// `[entry.strict.goldens]` — silently recording the abandonment on the wrong table.
    #[test]
    fn a_supersede_then_freeze_append_lands_on_the_right_entries() {
        let mut targets = BTreeMap::new();
        targets.insert("s4".to_string(), t("s4.bin", "aaaa", 10, "1111", 8));
        let mut src = render_entry("root", ASL_WITNESS, SHA_A, "", &targets);
        src.push_str(&render_strict(&red()));
        src.push_str(&render_superseded(&sup("successor")));
        src.push_str(&render_entry("successor", "ref", SHA_A, "", &targets));
        let chain = parse(&src).unwrap();
        assert_eq!(chain.entry.len(), 2);
        assert_eq!(chain.entry[0].superseded.as_ref().map(|s| s.by.as_str()), Some("successor"));
        assert_eq!(chain.entry[1].superseded, None, "the abandonment stayed on the old tip");
        assert_eq!(chain.entry[1].name, "successor");
    }

    #[test]
    fn render_strict_roundtrips_every_field() {
        let mut r = red();
        r.expected_tests = vec!["mod::my_new_gate".into()];
        r.skips = 27;
        let mut targets = BTreeMap::new();
        targets.insert("s4".to_string(), t("s4.bin", "aaaa", 10, "1111", 8));
        // A bare `[entry.strict]` with no `[[entry]]` before it makes `entry` a TABLE
        // rather than an array and fails to parse outright — the append can only ever
        // land on a real entry, never float free.
        assert!(
            toml::from_str::<Chain>(&render_strict(&r)).is_err(),
            "a strict table with no entry before it must not parse"
        );
        let src = format!("{}{}", render_entry("e", "ref", SHA_A, "", &targets), render_strict(&r));
        let chain: Chain = toml::from_str(&src).unwrap();
        assert_eq!(chain.entry[0].strict.as_ref(), Some(&r), "every field must round-trip");
    }

    /// The reason comes off a command line and a failing test name is whatever libtest
    /// printed, so both need escaping. `render_entry`'s own fields never have.
    #[test]
    fn rendered_values_escape_quotes_and_backslashes() {
        let s = Superseded {
            by: "next".into(),
            reason: r#"red: "SFX_BODY_LEN" != 0x8DA, see C:\logs"#.into(),
        };
        let block = render_superseded(&s);
        let chain: Chain = toml::from_str(&format!(
            "[[entry]]\nname=\"x\"\nab=\"asl-witness\"\n[entry.targets.s4]\ngolden=\"s4.bin\"\nfull_crc=\"0\"\nfull_size=1\nanchor_crc=\"0\"\nanchor_end=1\n{block}"
        ))
        .expect("an unescaped quote would make this unparseable");
        assert_eq!(chain.entry[0].superseded.as_ref(), Some(&s));
    }

    #[test]
    fn is_full_sha_rejects_near_misses() {
        assert!(is_full_sha(SHA_A));
        assert!(!is_full_sha(""), "empty");
        assert!(!is_full_sha(&SHA_A[..39]), "39 chars");
        assert!(!is_full_sha(&format!("{SHA_A}0")), "41 chars");
        assert!(!is_full_sha(&SHA_A.to_uppercase()), "uppercase hex");
        assert!(!is_full_sha(&"g".repeat(40)), "non-hex");
    }
}
