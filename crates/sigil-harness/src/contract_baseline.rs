//! The FROZEN contract-closure baselines — one copy, read by both the CI gates
//! and the build-integrated closure gate.
//!
//! **Why this module exists at all.** A frozen baseline that is written down
//! twice forks: one copy is updated when a firing is adjudicated, the other is
//! not, and from then on the two halves disagree about what the corpus is
//! allowed to do. The build gate and the corpus tests therefore read the SAME
//! constants from here — neither owns a copy.
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

/// `[proc.out-unverified]` for `(cpu: z80)` procs (§G4.5, the Z80 unit-domain
/// twin): a Z80 proc declares `out(rN)` but the production dataflow cannot prove
/// `rN` (a register UNIT) is written on every required return path.
///
/// EVERY ROW IS ONE CLASS — a CARRY-CONDITIONAL RESULT. Each firing names a
/// register that is a valid result ONLY on the callee's carry-CLEAR (success)
/// edge: produced there (`… / or a / ret`), and left unproduced on the carry-SET
/// (failure) edge (`scf / ret`), which the caller does not read. The declaration
/// pairs the register out with a `out(carry: NAME)` flag result that IS the
/// register's validity signal — and the caller-side `[call.flag-result-unused]`
/// gate proves every caller consumes that carry before touching the register.
///
/// The register is declared UNCONDITIONALLY (`out(iy)`, `out(de)`, …) rather than
/// as the honest conditional form `out(rN if nc)`, because the Z80 conditional
/// REGISTER out is represented-not-wired (unlike the 68k `out(rN if cc)`, which is
/// modeled). So the production check — reading the declaration as written — cannot
/// see the conditional edge and correctly reports the register unproduced on the
/// failure path. These are verifier-model / contract-precision gaps, NOT dishonest
/// declarations: none is a register never written on ANY path (each IS produced on
/// its success edge), so there is no byte-neutral aeon fix that is not a narrowing
/// of a true contract. KILL CONDITION: wiring the Z80 `out(rN if cc)` form (so the
/// declaration can name the carry edge) closes every row here at once.
///
/// A SEPARATE array from [`OUT_UNVERIFIED_BASELINE`], never merged into it: the
/// 68k residue is keyed by `dN`/`aN` and carries a width facet; the Z80 residue is
/// keyed by the 8-bit/index UNITS (`h`/`l`/`a`/`ix`/…) an `out(hl)` expands to and
/// has no width. One flat array, because a Z80 out is a DECLARATION (not
/// comptime-gated), so its residue is the SAME set on every shipped shape — the
/// closure gate walks all seven and the diff must hold for each. The non-vacuity
/// census (`z80_out_claims`) proves the gate ranged over the real out declarations
/// rather than passing because the walk saw nothing.
pub const Z80_OUT_UNVERIFIED_BASELINE: &[(&str, &str)] = &[
    ("Sfx_MusicChanPtr", "iy"),
    ("Sfx_ResolveBlob", "d"),
    ("Sfx_ResolveBlob", "e"),
    ("Sfx_SelectVoice", "d"),
    ("Snd_DacLookup", "h"),
    ("Snd_DacLookup", "l"),
];

/// `[proc.out-unverified]` (§G4.5): a proc declares `out(rN)` but the closure
/// cannot prove `rN` is produced on every required return path.
///
/// SHAPE-INVARIANT, unlike [`D1C_BASELINE`] — measured on every one of the seven
/// shipped shapes. If that ever stops holding, the D1c pair above is the shape to
/// copy; do not flatten a real per-family difference into one array.
///
/// EVERY row here carries the reason "not produced on a required return path": the
/// register has no write at all on some path, and no declaration can change that.
/// The `DrawRings` / `InsertSpriteMasks` / `Emit_ObjectPieces` cursors thread a
/// sprite counter (`d5`) and a SAT pointer (`a4`) that advance only when a sprite
/// is emitted; the empty-buffer, cap-already-reached-at-entry, and
/// every-candidate-culled paths write neither, and the no-param-seed model
/// (`out_verify` Finding 2) does not credit a threaded entry value as production.
/// These are production-completeness rows, not width rows.
///
/// The width facet is real and exercised by `out_verify`'s unit tests, but the
/// corpus currently exhibits no sub-width production row: the three sprite-counter
/// procs increment `d5` with `addq.w` and declare `out(d5: u16)`, so on the paths
/// that DO produce `d5` the width matches every caller's `.w` read (and the lone
/// `.b` cap test a `u16` also covers). `out(d5: u8)` is the barred adoption — it
/// would publish a contract narrower than the callers consume, sound as a claim and
/// useless as a promise. (The array is the one authority for counts; a number
/// written down twice is the fork this module's preamble prevents.)
///
/// Which rows are sub-width production and which are genuinely unproduced is
/// measured by turning the width rule off and reading the result as a SET DIFF,
/// never by classifying rows by eye. Reproduce it that way rather than trusting any
/// prose here; the standing write-up is the item-1 fixpoint census, a lane document
/// that may not sit beside this file.
///
/// ROWS ARE NOT INDEPENDENT SITES, in general. Credit flows along calls, tails and
/// `falls_into` successors, so one unproven production can hold several rows open.
/// A burn-down that counts rows then over-counts the work and can "fix" a
/// dependant by papering over its root — compute the census over the fixpoint, and
/// read every measurement as a SET DIFF rather than a count.
///
/// Every row presently here is nonetheless self-contained: each proc writes its
/// own register in its own body and sources none of it from a callee, tail or
/// successor. That is a fact about today's corpus, not a property of the analysis,
/// and it is why no row here can serve as the cross-proc credit witness — see
/// `contract_closure_corpus.rs::corpus_out_residue_is_the_verified_complement`,
/// whose witness moved to a synthetic for exactly this reason.
pub const OUT_UNVERIFIED_BASELINE: &[(&str, &str)] = &[
    ("Collision_ProbeDown", "d1"),
    ("Collision_ProbeDown", "d2"),
    ("Collision_ProbeLeft", "d1"),
    ("Collision_ProbeLeft", "d2"),
    ("Collision_ProbeRight", "d1"),
    ("Collision_ProbeRight", "d2"),
    ("Collision_ProbeUp", "d1"),
    ("Collision_ProbeUp", "d2"),
    ("DrawRings", "a4"),
    ("DrawRings", "d5"),
    ("InsertSpriteMasks", "a4"),
    ("InsertSpriteMasks", "d5"),
];

/// `[call.live-clobbered]` (D1c) — the SHAPE-INVARIANT rows: a caller holds a
/// value in a register across a call whose callee destroys it.
///
/// D1c is shape-DEPENDENT and this constant is only half the pin. The debug
/// family assembles debug-only code that the plain family never sees, so it
/// carries [`D1C_DEBUG_EXTRA`] on top. This is the `warn_tier_corpus` shape
/// exactly — a shape-invariant set plus a per-family addition — and it exists
/// because one flat baseline is WRONG here, not merely coarse: measured, the
/// plain family fires 20 and the debug family 25.
///
/// Two rows are DOCUMENTED false positives of one class — D1c's close is
/// edge-blind, so a register read that is really a conditional callee's PRODUCED
/// value on the success edge looks like a destroyed held value. They are
/// `TileCache_FillRow @ TileCache_FindStagedBlock :: a1` and `Load_Object @
/// AllocDynamic :: a1`; `calls.rs`'s `destroys_value` header carries the per-site
/// reasoning. An edge-precise D1c would dissolve the class.
pub const D1C_BASELINE: &[(&str, &str, &str)] = &[
    ("CreateChild_Complex", "AllocDynamic", "a1"),
    ("CreateChild_FlipAware", "AllocDynamic", "a1"),
    ("CreateChild_Linked", "AllocDynamic", "a1"),
    ("CreateChild_Normal", "AllocDynamic", "a1"),
    ("CreateEffect_Normal", "AllocEffect", "a1"),
    ("CreateEffect_Simple", "AllocEffect", "a1"),
    ("GameState_ObjectTestChurn_Init", "AllocDynamic", "a1"),
    ("GameState_ObjectTest_Init", "AllocDynamic", "a1"),
    ("Ground_Move_Cap", "Player_SensorWallDir", "d0"),
    // DOCUMENTED FP (edge-blind close) — see calls.rs::destroys_value.
    ("Load_Object", "AllocDynamic", "a1"),
    ("PState_AirShared", "Air_WallProbeLeft", "d4"),
    ("PState_AirShared", "Air_WallProbeRight", "d1"),
    ("PState_AirShared", "Air_WallProbeRight", "d4"),
    ("PState_Spindash", "Player_SensorFloor", "d0"),
    ("PState_Spindash", "Player_SensorFloor", "d1"),
    ("Parallax_Update", "Decode_Factor_A", "d2"),
    ("Parallax_Update", "Decode_Factor_B", "d2"),
    ("TestPlayer_Main", "Player_SensorFloor", "d0"),
    ("TestPlayer_Main", "Player_SensorFloor", "d2"),
    // DOCUMENTED FP (edge-blind close) — see calls.rs::destroys_value.
    ("TileCache_FillRow", "TileCache_FindStagedBlock", "a1"),
];

/// The D1c rows that only the DEBUG family produces — debug-gated code the plain
/// shapes do not assemble. `Input_Tick @ Replay_Hash :: d0` appears TWICE on
/// purpose: `Replay_Hash` is called at two sites in `Input_Tick` (the checkpoint
/// compare and the record path), and the multiset diff is what keeps the second
/// one visible.
pub const D1C_DEBUG_EXTRA: &[(&str, &str, &str)] = &[
    ("Collected_UpdateCenter", "Collected_ParkSlot", "d2"),
    ("EntityWindow_RescanY", "EntityWindow_RescanRings", "d5"),
    ("EntityWindow_Scan", "EntityWindow_ScanRingsRight", "d5"),
    ("Input_Tick", "Replay_Hash", "d0"),
    ("Input_Tick", "Replay_Hash", "d0"),
];

/// The D1c baseline for a shape: invariant rows, plus the debug-family addition.
///
/// `[proc.out-unverified]` needs no such split — measured shape by shape, it
/// fires the SAME set on every one of the seven targets, so
/// [`OUT_UNVERIFIED_BASELINE`] is flat. The count is not restated here: the array
/// is the one authority, and a number written down twice is the fork this
/// module's preamble exists to prevent.
pub fn d1c_baseline(debug_family: bool) -> Vec<(&'static str, &'static str, &'static str)> {
    let mut rows: Vec<_> = D1C_BASELINE.to_vec();
    if debug_family {
        rows.extend_from_slice(D1C_DEBUG_EXTRA);
    }
    // Sorted, so the combined baseline stays comparable to the analysis's own
    // sort order — appending the debug rows after the invariant ones would
    // otherwise foreclose an order pin for the debug family.
    rows.sort_unstable();
    rows
}

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

/// Diff the corpus's `[proc.out-unverified]` firings against the frozen baseline.
pub fn diff_out_unverified(got: &[(String, String)]) -> BaselineDiff {
    diff_multiset(
        got.iter().map(|(p, r)| format!("{p} :: out({r})")).collect(),
        OUT_UNVERIFIED_BASELINE
            .iter()
            .map(|(p, r)| format!("{p} :: out({r})"))
            .collect(),
    )
}

/// Diff the corpus's Z80 `[proc.out-unverified]` firings against the frozen Z80
/// baseline. Its OWN diff fn against its OWN array — never folded into
/// [`diff_out_unverified`], so a Z80 row and a 68k row can never masquerade for
/// each other in a multiset that spans both key spaces.
pub fn diff_z80_out_unverified(got: &[(String, String)]) -> BaselineDiff {
    diff_multiset(
        got.iter().map(|(p, u)| format!("{p} :: out({u})")).collect(),
        Z80_OUT_UNVERIFIED_BASELINE
            .iter()
            .map(|(p, u)| format!("{p} :: out({u})"))
            .collect(),
    )
}

/// Diff the corpus's `[call.live-clobbered]` firings against the frozen baseline
/// for that shape's family.
pub fn diff_d1c(got: &[(String, String, String)], debug_family: bool) -> BaselineDiff {
    diff_multiset(
        got.iter().map(|(p, c, r)| format!("{p} @ {c} :: {r}")).collect(),
        d1c_baseline(debug_family)
            .iter()
            .map(|(p, c, r)| format!("{p} @ {c} :: {r}"))
            .collect(),
    )
}

/// The adjudication instruction every gate prints on a mismatch. One wording, so
/// the build gate and the CI gate cannot drift into telling different stories.
pub fn adjudication_message(family: &str, d: &BaselineDiff) -> String {
    format!(
        "{family} moved against the frozen baseline.\n  \
         NEW firings (a real violation, or an analysis that just got sharper): {:?}\n  \
         GONE firings (a STALE pin — or the analysis NARROWED, which is the \
         destructive direction: the same closure feeds find_dead_saves, so a dropped \
         row can mean a load-bearing save is now reported dead): {:?}\n  \
         If the change is intended, adjudicate each row and update the baseline in \
         crates/sigil-harness/src/contract_baseline.rs in the SAME commit.",
        d.added, d.removed
    )
}
