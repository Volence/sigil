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

## THE PANEL ROUND — two unsound credits and a hollow pin, caught before merge

The parcel was gate-green (byte ×7, strict, refreeze, warn tiers) and still
carried two credits that were **wrong about the machine**. Green gates did not
catch them because a byte-neutral contract parcel moves no bytes and fires no
lint — only a reader following the model could. Lenses B and C found the first
hole INDEPENDENTLY, from opposite ends, which is the strongest signal in the
round.

**P1 — the `(sp)` PEEK credited a word facet from a mismatched save width
(lenses B + C).** `credit_restore` required both widths to be *at least* a word,
not to MATCH. The POP arm is accidentally protected by its byte-balance check
(`got != want` bails); the PEEK arm has no balance check at all — it reads
`stack.last()` and credits directly. So `move.l d5,-(sp)` … `move.w (sp),d5`
verified `preserves(d5.w)` while on a big-endian 68000 that `.w` read loads the
saved HIGH word into d5's low word: the entry low word is destroyed and the
verifier said Verified. Seven live corpus peek sites, all currently over `.w`
pushes, so no shipped verdict was wrong — one edit away from live.
FIX: `restore_w == save_w` for both facets, in the function both arms share.

**P2 — the same fix closed a WEAKENING I introduced (lens C).** Master tagged
every push from `entry`; my `tag` tagged `.w`/`.b` pushes from the strictly
weaker `entry_word`, and `credit_restore` set `full` from the popped slot without
consulting that slot's own width (byte balance compares only TOTALS). Witness: a
`.w` round-trip of d5, then `move.w d0,-(sp)` / `move.w d5,-(sp)` /
`move.l (sp)+,d5` — want 4, got 2+2, matched, is_long → Verified, when the
machine leaves `d5 = (entry_d5.w << 16) | d0.w`. **Master correctly refused
this**, and spec §3 forbids any consumer weakening in terms. Requiring a
`.l`-SAVED slot for the full credit refuses it again. My original probe pinned
only the single-slot case, which is not where the regression was.

**P3 — my conservative-v1 pin proved nothing (lens C).** It hand-built an oracle
whose effective set already contained d5 and asserted a property that held
identically on master; the implementing line could be deleted with the test still
green. Replaced with pins that drive the real path, and **revert-probed**:
deleting `declared_clobbers`' word extension now fails a named test.
*Correction to the ruling:* deleting that line is not caught by "nothing" — the
real-corpus `corpus_closure_residue_is_empty_the_error_gate` does fail (measured,
both directions). But it is caught only incidentally, and only while the corpus
happens to contain a `.w` witness in a debug shape, which is exactly why the
explicit pin was still owed.

**P4 — the corpus gate had never been observed to fire (lens B).** A bare
assert-empty over a vector nothing made non-empty. Rebuilt on the sibling
`preserves_corpus.rs` shape: an exact claim census (four witnesses × every
shape) plus a count assertion beside the error gate, and a synthetic probe that
exercises the firing path.

**P5 — two comment-truth defects in aeon (lens A).** The module's authoritative
d5 note still said the register crosses callees that *declare* `clobbers(d5)`
(false once TrySpawnRing declared the facet, and it is the note the `ensure`
hangs off); `TestPlayer`'s header called `TestPlayer_Main`'s set "its whole set"
when it is now a strict superset whose extra d7 IS the conservative-v1 fact.

Also fixed: one fold for the facet token (`WordFacet::fold_token` — "accepted"
and "obligated" could previously disagree silently); the word obligation folded
into `preserve_oracle_inputs` and made to FAIL CLOSED, now pinned by a probe that
watches the credit collapse while the obligation survives; `out(dN)
preserves(dN.w)` got its overlap arm; the full-preserve refusal steers to
`preserves(dN.w)` instead of claiming a `.w` restore "sign-extends and preserves
nothing" (false, and doubly so for a data register); `ObjRoutine` narrowed to
`preserves(a0, d7.w)` — it had promised callers a full d7 no implementation
restores (verdict-neutral: the contract report is byte-identical before and
after, measured).

## Catches from the build round (pre-panel)

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

- The POP/PEEK asymmetry is the structural root of P1 and will bite the next
  facet: one arm gets its width discipline from byte balance, the other from
  nothing. Ledgered with the ask (one `restore_slot` primitive owning drain-or-peek
  + width agreement + facet credit).
- A contract TYPE's `preserves` clause is never validated — `contract_type_bound`
  reads it through `expand_reglist_regs`, which silently drops what it does not
  recognise. `ObjRoutine`'s `d7.w` is therefore correct-by-luck in the safe
  direction (a dropped token makes the bound WIDER); `d7.q` would drop just as
  silently. Ledgered.

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
- **The panel earned its keep on a gate-green parcel.** Every bar passed both
  before and after the fixups; the two unsound credits were invisible to all of
  them because a byte-neutral contract change moves nothing a gate measures. The
  catches came from reading the model against the machine (big-endian `.w` reads,
  slot widths vs byte totals) and from asking what a pin would still prove if the
  code under it were deleted.

## Bars (all own-run, post-rebase onto the current masters)

- Byte bar: SEVEN targets byte-identical vs chain-48 goldens (s4 `b5ffb094`,
  s4.debug `57fd08f9`, demo `cbddc142`, demo.debug `b61f462d`, config_a `61e4e78e`,
  config_b `07e3f465`, lean `b92cb485`).
- Full strict: `cargo test --workspace --release` EXIT 0. Closing arithmetic:
  passed 3500 + ignored 4 = 3504 == branch `#[test]` total 3504, failed 0.
- `refreeze --check`: OK (tip `enum-dispatch`, chain len 48).
- Warn tiers: firing lint-id SET identical ×7 (plain 9/1/3/6, debug 9/1/3/5 across
  {module.path-mismatch, proc.clobber-undeclared, proc.out-unwritten,
  proc.undeclared-fallthrough}) — the flips add NO warning (the check_clobbers
  allowed-set fix silences clobber-undeclared on the licensed word registers).
- Census: closure firings 0 and word-facet firings 0 in both plain and
  debug profiles — the four witnesses verify under the oracle.
- Probes: model (both `.w`/full polarities, non-vacuity, `.b`-refuses-word,
  write-after-restore-refutes, plus the four panel regressions: `.w`-peek-of-`.l`,
  `.w`-pop-of-`.l`, `.l`-peek-of-`.w`, and the `.l`-pop-over-two-`.w`-slots
  non-weakening pin) + surface (every refusal arm both polarities, positive `.w`,
  overlap incl. the new `out`∩`.w` arm, redundant full+word) + conservative-v1
  through the real path (permission, no-credit, surface-record, firing-path) with a
  revert probe + the fail-closed polarity probe + the real-corpus per-shape word
  gate with its claim census.
- Clippy `--workspace --all-targets` clean.

Rebased across three master moves — hyg, then srmask, then Track C. Final bases:
sigil `4a5841f7` / aeon `ad4c6ef`. Resolutions: `TestPlayer_Main` takes hyg's
closure-proved a0 + this parcel's d7.w (`preserves(a0, d7.w)`); the word set folded
into srmask's threaded `preserve_oracle_inputs` (real `@noreturn` +
mask-preservers, no empty stand-ins re-introduced) with its three destructures in
`preserve_oracle_threading.rs` updated; Track C's `entity_window.emp` changes
(`EntryForSection` → `out(d0: EntryRef)`, the `assume_some!` sites) and this lane's
d2/d5 flips both survive in that file, and the three transitive callers still
resolve exactly as before (verified: closure firings 0, word firings 0, warn-tier
sets unchanged, byte ×7 identical). Re-verify master positions at merge — the ltr
lane's chain-49 refreeze is still in flight, so this branch's byte bar is stated
against chain 48.
