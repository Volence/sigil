//! The contract-closure baseline diff, one copy, run by both the build-integrated
//! closure gate and the CI gates.
//!
//! **Where the rows live.** In the descriptions: the `baseline.toml` files the engine and
//! game descriptions name, unioned per shape by `sigil_build::baseline::closure_baseline`
//! (G0b). The constant tables that used to sit here, and the profile reading that picked
//! a shape's family rows out of them (`is_demo_family`, `for_profile`), were deleted with
//! the profile-driven driver (G0b Task 11). This module keeps the half that is not a
//! fact about any game: the [`ShapeBaseline`] the gate diffs against, the multiset diff,
//! and the one adjudication wording.
//!
//! **What a baseline means.** These are not lists of bugs. They are the corpus's
//! standing residue at freeze time, split by the ruling into two buckets that the
//! ledger tracks separately: a genuinely-loose contract (burn it down) and a
//! verifier-model gap (it stays pinned until the model grows — forcing it to zero
//! by editing engine code to please a checker is the barred move).
//!
//! **The ratchet, stated precisely.** Both directions are errors. A firing NOT in
//! the baseline is a NEW violation and fails. A baseline row that STOPS firing is
//! a stale pin and also fails — because the analysis narrowing is the destructive
//! direction: the same closure feeds `find_dead_saves`, so a silently dropped row
//! can mean a load-bearing save is now reported dead. Diffs are MULTISETS, not
//! sets, so a gained or lost duplicate at a second call site is visible.

use std::collections::BTreeMap;

/// A baseline comparison result. Empty `added` and `removed` is the pass.
#[derive(Debug, Default)]
pub struct BaselineDiff {
    /// Firings present in the corpus and absent from (or over-represented
    /// against) the baseline — NEW violations.
    pub added: Vec<String>,
    /// Baseline rows that no longer fire — a STALE pin, and possibly an analysis
    /// that has silently narrowed.
    pub removed: Vec<String>,
}

impl BaselineDiff {
    /// True when the corpus matches the frozen baseline exactly.
    pub fn is_clean(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

/// Multiset diff of `got` against `want`. Counting — not set membership — is what
/// makes a gained or lost DUPLICATE visible: the same triple can fire at two call
/// sites in one proc, and a set test would let that through both lists.
fn diff_multiset(got: Vec<String>, want: Vec<String>) -> BaselineDiff {
    let tally = |rows: &[String]| {
        let mut m: BTreeMap<String, usize> = BTreeMap::new();
        for r in rows {
            *m.entry(r.clone()).or_default() += 1;
        }
        m
    };
    let (g, w) = (tally(&got), tally(&want));
    // Report the COUNTS, not the bare key. A row that goes 2 -> 1 is a real
    // change and a bare key renders it as if the row vanished entirely — the
    // reader greps, finds the call still there, and concludes the gate is broken.
    let render = |k: &String, got_n: usize, want_n: usize| format!("{k} (got {got_n}, want {want_n})");
    BaselineDiff {
        added: g
            .iter()
            .filter(|(k, n)| w.get(*k).copied().unwrap_or(0) < **n)
            .map(|(k, n)| render(k, *n, w.get(k).copied().unwrap_or(0)))
            .collect(),
        removed: w
            .iter()
            .filter(|(k, n)| g.get(*k).copied().unwrap_or(0) < **n)
            .map(|(k, n)| render(k, g.get(k).copied().unwrap_or(0), *n))
            .collect(),
    }
}

/// The frozen rows ONE shape's closure gate diffs against: the `baseline.toml` rows its
/// engine and game descriptions name, unioned (sigil-cli `project_subject`, via
/// `sigil_build::baseline::closure_baseline`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShapeBaseline {
    /// `[proc.out-unverified]` for Z80 procs: `(proc, unit)`.
    pub z80_out_unverified: Vec<(String, String)>,
    /// `[proc.out-unverified]` for 68000 procs: `(proc, register)`.
    pub out_unverified: Vec<(String, String)>,
    /// `[proc.inout-unverified]`: `(proc, register)`.
    pub inout_unverified: Vec<(String, String)>,
    /// `[call.live-clobbered]` for this shape, its family rows already added:
    /// `(caller, callee, register)`, a multiset.
    pub d1c: Vec<(String, String, String)>,
    /// Where an adjudicated row is written, named by [`adjudication_message_in`]: the
    /// description files the rows came from.
    pub home: String,
}

impl ShapeBaseline {
    /// Diff `[proc.out-unverified]` (68000) firings.
    pub fn diff_out_unverified(&self, got: &[(String, String)]) -> BaselineDiff {
        diff_pairs(got, &self.out_unverified, "out")
    }

    /// Diff `[proc.out-unverified]` (Z80) firings: their OWN rows, never folded into
    /// the 68000 multiset, so a Z80 row and a 68k row cannot stand in for each other.
    pub fn diff_z80_out_unverified(&self, got: &[(String, String)]) -> BaselineDiff {
        diff_pairs(got, &self.z80_out_unverified, "out")
    }

    /// Diff `[proc.inout-unverified]` firings.
    pub fn diff_inout_unverified(&self, got: &[(String, String)]) -> BaselineDiff {
        diff_pairs(got, &self.inout_unverified, "inout")
    }

    /// Diff `[call.live-clobbered]` (D1c) firings.
    pub fn diff_d1c(&self, got: &[(String, String, String)]) -> BaselineDiff {
        let key = |(p, c, r): &(String, String, String)| format!("{p} @ {c} :: {r}");
        diff_multiset(got.iter().map(key).collect(), self.d1c.iter().map(key).collect())
    }
}

fn diff_pairs(got: &[(String, String)], want: &[(String, String)], facet: &str) -> BaselineDiff {
    let key = |(p, r): &(String, String)| format!("{p} :: {facet}({r})");
    diff_multiset(got.iter().map(key).collect(), want.iter().map(key).collect())
}

/// The adjudication instruction every gate prints on a mismatch, naming `home` as where
/// the baseline is updated (the descriptions' `baseline.toml` files). One wording, so
/// the build gate and the CI gate cannot drift into telling different stories.
pub fn adjudication_message_in(family: &str, d: &BaselineDiff, home: &str) -> String {
    format!(
        "{family} moved against the frozen baseline.\n  \
         NEW firings (a real violation, or an analysis that just got sharper): {:?}\n  \
         GONE firings (a STALE pin, or the analysis NARROWED, which is the \
         destructive direction: the same closure feeds find_dead_saves, so a dropped \
         row can mean a load-bearing save is now reported dead): {:?}\n  \
         If the change is intended, adjudicate each row and update the baseline in \
         {home} in the SAME commit.",
        d.added, d.removed
    )
}
