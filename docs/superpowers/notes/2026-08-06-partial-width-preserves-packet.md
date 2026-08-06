# Partial-width preserves — `preserves(dN.w)` (lane pw) packet

Spec: `docs/superpowers/specs/2026-08-06-partial-width-preserves-design.md` (+ §6
amendment). Closes ledger row 2144, resolves row 2138's vocabulary note.

## What shipped

A data-register FACET claim `preserves(dN.w)`: on return dN's low 16 bits equal
their entry value; the upper word is unspecified (implicitly clobberable, licensed
by the facet). The exact analog of `sr.mask`/`sr.ccr` — dotted, through the
existing reg-list grammar, validated semantically.

- **Model** (`preserves.rs`): the single-bool `entry` bit became `entry` +
  `entry_word`, each stack `Slot` carrying its `save_width`. A `.w` save/restore
  round-trips the WORD (crediting `entry_word`, not `entry`); a `.b` round-trips
  NEITHER (the width the equal stack `bytes` — 2 for both `.w` and `.b` on a7 —
  cannot tell apart, now carried by `save_width`). `Facet::{Full,Word}` selects
  which exit map the verdict reads; one dataflow computes both. Full `.l` proves
  the word (the monotone `entry ⟹ entry_word` invariant).
- **Conservative v1**: a `.w`-preserving callee is a FULL clobber to every
  consumer. `dN` joins `declared_clobbers` / the local allowed set (closure
  verdicts IDENTICAL to `clobbers(dN)`), and is NOT credited to
  `verified_preserves`, so a caller through a `.w`-preserving callee still refuses.
  The low-word round-trip is a SEPARATE obligation: the per-file byte gate DEFERS a
  call-blocked claim (silent), and the corpus `word_preserve_firings` oracle gate
  is its final authority (the analog of `[proc.clobber-undeclared]` catching a
  full-preserve deferral, which the word facet silences by licensing dN's clobber).
- **Surface**: `.b` / `aN.w` / `.l` / unknown-facet each refuse with their own
  `[proc.preserves-invalid]` arm; the clobbers-overlap diagnostic extends to the
  facet; a register in both full-preserve and word-preserve is redundant (subsumed).
- **Adoption** (byte-neutral): the four genuine witnesses — `Player_Main` d7,
  `TestPlayer_Main` d7 (`preserves(a0, d7.w)` post-rebase), `Collected_ParkSlot`
  d2, `EntityWindow_TrySpawnRing` d5.

## Catches (foregrounded)

1. **Census was wrong: 4 genuine witnesses, not 7 (the §4 STOP).** The spec's
   "five DEBUG arms" listed all four `EntityWindow_*` procs as d5 witnesses, but
   only `TrySpawnRing` round-trips d5 in its OWN body. `ScanRingsRight` /
   `PopulateSectionRings` / `RescanRings` carry d5 TRANSITIVELY through
   `jbsr TrySpawnRing` — under conservative v1 a `.w`-preserving callee is a full
   clobber to them, so flipping them would `[proc.preserves-unverifiable]` and break
   the build. This is exactly §3's conservative-refusal pin: they stay
   `clobbers(d0-d5)` as its real-corpus witnesses. Stopped pre-build; overseer
   re-adjudicated (§6 amendment).
2. **`TestPlayer` → `TestPlayer_Main` witness-identity correction.** The spec's
   "TestPlayer d7 (244/246)" cited lines that live inside `TestPlayer_Main`.
   `TestPlayer` (the init proc) never touches d7 — it falls into `TestPlayer_Main`,
   which is a full d7 clobberer to it, so `TestPlayer` would refuse. Flipped the
   proc that actually round-trips (`TestPlayer_Main`); `TestPlayer` keeps
   `clobbers(d0-d7)` as a second transitive/fall-into negative witness.
3. **Verdict-invariance of the transitive callers, verified.** After
   `TrySpawnRing` flips, the census closure firings stay 0 in both plain and debug
   profiles — the three callers' `clobbers(d5)` remains honest (d5 clobbered via
   the call), no verdict changed.

## Per-pass findings

**Step-3 (retrospect / language asks):**
- The `.w` facet is the minimal vocabulary with witnesses; `.b`/`.uw` deliberately
  absent (demand-gated, per §2). A future `.uw` clobber spelling has no independent
  claimant on a data register and stays unbuilt.
- Deferred-verification note: a DEBUG-gated word facet verifies via "never written"
  in the plain shape and via the round-trip in the debug shape — the corpus gate
  runs per-shape so both are proven. Stated in the corpus gate's doc comment.

**Step-5 (perf / hazard):**
- Zero machine-code change: the parcel is the row-2144 principle in action (the
  MODEL gained the width; the program was left alone). Byte bar ×7 identical.
- The conservative-v1 propagation is the sound, monotone reading — building a
  width-aware CONSUMER (a caller that credits a `.w`-preserving callee's low word)
  is explicitly NOT this parcel and would be the next arc if a consumer appears.

**Neither-bucket headline:**
- The word facet reuses the ENTIRE existing preserve machinery (the shared
  `run_stack_dataflow`, the DEFER discipline, the oracle round) — the only new
  surface is one bit per register (`entry_word`), one field per slot (`save_width`),
  a `Facet` selector, and a dedicated corpus firing vector. No new dataflow.

## Bars (all own-run, post-rebase onto the current masters)

- Byte bar: SEVEN targets byte-identical vs chain-48 goldens (s4 `b5ffb094`,
  s4.debug `57fd08f9`, demo `cbddc142`, demo.debug `b61f462d`, config_a `61e4e78e`,
  config_b `07e3f465`, lean `b92cb485`).
- Full strict: `cargo test --workspace --release` EXIT 0. Closing arithmetic:
  passed 3463 + ignored 4 = 3467 == branch `#[test]` total 3467, failed 0.
- `refreeze --check`: OK (tip `enum-dispatch`, chain len 48).
- Warn tiers: firing lint-id SET identical ×7 (plain 9/1/3/6, debug 9/1/3/5 across
  {module.path-mismatch, proc.clobber-undeclared, proc.out-unwritten,
  proc.undeclared-fallthrough}) — the flips add NO warning (the check_clobbers
  allowed-set fix silences clobber-undeclared on the licensed word registers).
- Census: closure firings 0 and word-facet firings 0 in both plain and
  debug profiles — the four witnesses verify under the oracle.
- Probes: model (both `.w`/full polarities, non-vacuity, `.b`-refuses-word,
  write-after-restore-refutes, conservative-v1 caller-through-`.w`-callee) +
  surface (every refusal arm both polarities, positive `.w`, overlap, redundant
  full+word) + a real-corpus per-shape word gate.
- Clippy `--all-targets` clean.

Rebased onto sigil `686e6f62` (hyg) / aeon `76013f2` (hyg); `TestPlayer_Main`
resolved to `preserves(a0, d7.w)` (hyg's closure-proved a0 + this parcel's d7.w),
`Player_Main`/entity_window auto-merged clean. Re-verify master positions at merge
(more may land).
