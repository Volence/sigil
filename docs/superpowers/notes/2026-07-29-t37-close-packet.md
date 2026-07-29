# 2026-07-29 — t37 close packet (the sound_sfx port — rung-3b, the Out-heavy mirror)

Status: **CLOSE PACKET — gate (c) OPEN (overseer countersigned checkpoints a + b).** The `.emp` port
of the Phase-5a SFX engine (steal + per-frame interpreter), the FOURTH resident Z80 CODE port and
the last interpreter before the driver top (rung 4). Byte movement ZERO; the extern trust surface is
the windowed-oracle boundary (dies at the seam sub-tranche). Overseer own-runs from the tips: plain
**4b66cace/421041** EXACT · debug **1c256b3b/429102** EXACT · strict **2832/0 (1 ignored)** ·
the iy lockstep fix + the RHS-only guard comment verified in the log · kill row 83 verified. The
rebase onto the post-hoist-ambient masters, the merge, provenance, roadmap, and sweep are the
overseer's.

Tips: aeon `port-tranche37` @ `7c9bbd5` · sigil `port-tranche37` @ `b0c135b`.

## 0. Bars (overseer-countersigned at checkpoints a + b)

- Canonical held EXACT throughout: plain 4b66cace/421041 · debug 1c256b3b/429102 (dual rebuild after
  every `.asm`-touching edit — the iy header lockstep fix).
- Strict paired **2832/0 (1 ignored)** = the 2827 baseline + 5 new sfx-oracle tests, zero
  regressions, held across every loop step.
- **Byte movement ZERO** — the `.asm` twin's EMITTED bytes are untouched (Z80-blob-precedes-engine);
  the one `.asm` edit is comment-only (the iy header, byte-neutral, gate-proven). STOP-not-absorb held.
- The three other resident Z80 `.emp` (sequencer/psg/fm) + all 68k files stayed READ-ONLY.

## 1. What landed

- `engine/sound/sound_sfx.emp` (aeon) — the faithful `.emp` port of `sound_sfx.asm` (1627 L, 23
  top-level labels = 17 code procs + 3 data tables + 3 `_End` markers), proven by the SCALE-1
  dual-shape windowed oracle. The PSG t32 case (shape-variant base, shape-invariant layout).
- `crates/sigil-cli/tests/sound_sfx_port.rs` (sigil) — the windowed byte gate BOTH shapes + the t24
  positive control (moved SfxBlobWinTab) + doctored-both-equal (ModUpdate) (5 tests).
- Kill row 83 (the sound_sfx.asm canonical twin) + three drift/diagnostic ledger rows + the dry-panel
  records + the type-layer/construct candidate rows.

## 2. Byte-delta table — ZERO

| Region | plain | debug | Δ |
|---|---|---|---|
| whole ROM | 4b66cace / 421041 | 1c256b3b / 429102 | 0 (canonical, `.asm` emitted bytes untouched) |
| sfx window | $0CD7–$12C3 = 1516 B | $0D55–$1341 = 1516 B | +$7E BASE only (shape-invariant LENGTH) |

sfx is the PSG t32 CASE — shape-VARIANT BASE (+$7E from the SEQUENCER's 16 internal `if DEBUG==1`
blocks that PRECEDE this window), shape-INVARIANT LAYOUT (1516 B both shapes; sfx has NO own
`__DEBUG__` blocks — its only conditional is the game-config `if SFXID_REV_LOOP>=0`, always-on,
SFXID_SPINDASH=$AB). The two byte images differ only in the base embedded in internal jp/call targets
+ the +$7E-shifted fm/psg external targets; the two sequencer procs (ModUpdate/Sequencer_Channel) +
the two driver procs (SndDrv_SetBank/Snd_RouteClassFlags) PRECEDE the growth → shape-invariant. The
link seam was MEASURED from both listings, not assumed (the split is exact).

## 3. THE HEADLINE — firing arc 3 → 0 self-corrected + the FIRST clobbers UNDER-claim

The Out-heavy interpreter ported with **the full contract set firing 3, driven to 0 in-tranche, NO
finisher** — like fm's graduation, but with 3 self-corrections (the checker is complete since t33).
The 3 (all MY initial declaration issues, not checker gaps): (1) `Sfx_QueueEntryPtr` callee-preserves
cascade — declared the `bc,ix,iy` it genuinely holds (body touches only a,h,l,d,e) so
`Sfx_QueueEnqueue`'s `preserves(ix,iy)` verify + its append-path reliance on `bc` surviving the call
holds; (2,3) `Sfx_SelectVoice` `out(a,d,...)` overlapped `clobbers(...a...d...)` — a register is
output XOR scratch, split to `clobbers(f, bc, e, hl, ix, iy)`. Non-vacuity proven t33-style: an
injected false `preserves(hl)` on Sfx_Frame fires `[proc.preserves-unverifiable]` for h+l (reverted).

### 3.1 The FIRST clobbers UNDER-claim in the sound corpus (Sfx_Frame iy) + the checker-invisibility proof

`Sfx_Frame`'s `.asm` header declared `clobbers(af,bc,de,hl,ix)` — OMITTING `iy`, which its callees
Sfx_DrainQueue / Sfx_DuckRamp / Sfx_Restore DESTROY (their iy-walkers own it). Under exhaustive-license
semantics, omitting iy FALSELY tells callers iy is preserved. The `.emp` declares the honest superset
`clobbers(...,iy)`; the `.asm` header is fixed in lockstep (comment-only, byte-neutral, gate-proven).

**Proven checker-invisible:** removing iy from the `.emp` Sfx_Frame's clobbers does NOT fire the
checker — `[proc.clobbers-undeclared]` enforces clobbers-completeness LOCALLY (own-body register
writes) but NOT transitively across `call`s. So a clobbers UNDER-claim across calls silently drifts.

**The two halves of the contract-drift problem (connected explicitly):** this is the SYMMETRIC
partner of the t36 extern-decl-vs-def row. t36 showed a `preserves` claim on an EXTERN can silently
over-state (a stale decl claims a register survives that the def clobbers). t37 shows a `clobbers`
claim on a PROC can silently under-state (the declared clobbers misses a register a callee destroys).
Both are contract-drift the checker's current LOCAL analysis cannot catch; both are closed by the
same fixpoint the honest-contract rule already specifies in PROSE ("clobbers = callee-union ∪
locally-written; over-claim safe; never an unverifiable preserves") but does not yet MACHINE-verify.
The demanded diagnostic: `[call.clobbers-incomplete]` = declared clobbers ⊇ reachable-callee-union ∪
local writes (the mirror of the demanded extern-decl-vs-def consistency check). LEDGERED; KILL = the
diagnostic landing OR the seam sub-tranche (same kill as the extern-decl-vs-def row).

### 3.2 The extern-decl-vs-def BY-HAND verification set → 13 (11 co-resident subsets + 2 driver die-at-port)

No machine cross-check exists (the t36 hazard), so each of sfx's 13 `extern proc` decls was verified
BY HAND against the co-resident/driver def, kept exact-or-conservative-subset:
- **11 co-resident:** ModUpdate `preserves(ix)` (EXACT vs sequencer.emp) + Sequencer_Channel with NO
  `preserves(ix)` (MATCHING the def's honest omission — the t36 checker-limitation) + Fm_PatchLoad/
  NoteOff/SetVolume/PatchPtr/NoteOnFreqExact `preserves(ix)` (subsets vs fm.emp, ix via the module
  invariant) + Psg_NoteOff/NoteOn/Noise `preserves(ix)` + Psg_SetVolume `preserves(de,hl,ix)` (matches
  sequencer.emp's tighter decl). New instances of the t36 extern-decl-vs-def drift hazard class.
- **2 driver DIE-AT-PORT:** SndDrv_SetBank `preserves(bc,de,ix)` + Snd_RouteClassFlags
  `preserves(bc,de,hl,ix)` — boundary decls to the not-yet-ported rung-4 driver (the resolution-
  ladder's die-at-port exception). C2 independently re-verified one pair; C3 verified the ix-preserve
  claims against the resident tree.

### 3.3 The invariant(ix) adjudication — the t36 MIRROR (confirmed by ledger row 1777)

NO module `invariant: preserves(ix)`. `Sfx_Frame` is the SFX-side channel walker
(`ld ix,SND_SFX_CHANNELS` … `add ix,de`) that reads `(ix+sc_flags)` across `call Sequencer_Channel`
— relying on ix survival that is TRUE but checker-invisible through the shared interpreter's `ex(sp),hl`
computed dispatch (ledger row 1777 names sound_sfx.asm:294 as the 2nd relied-upon site). Per-proc
`preserves(ix)` declared on the verifiable leaves (Steal/MusicKeyOffKeepKeyed/Restore push/pop-
bracketed; MusicChanPtr/DeepestDuck/MinActiveKind iy-walkers; RouteKind/QueueEntryPtr/QueueEnqueue);
OMITTED on the walker/dispatch/allocation procs that clobber or produce ix. C2's re-derivation
independently confirmed the omission is correct. Adjudicated, not forced (exactly the brief predicted).

## 4. What each pass added (step-3 vs step-5, per the standing packet format)

**Step-1 (demanded/neither-bucket):** the psg-class window derivation MEASURED both shapes (§2); the
firing arc 3→0 + non-vacuity (§3); the 13-extern by-hand set (§3.2); the first clobbers under-claim +
its diagnostic (§3.1); the invariant(ix) t36-mirror (§3.3). No demanded LANGUAGE feature — every
feature sfx needs shipped already (bare-symbol-imm8 t32, `(ix+(field+k))` t32, comptime `if X>=0`).

**Loop pass 1 — step-3 findings:**
- 3(a) asks (all LEDGERED, byte-frozen): the base_plus_idx / ix_field_ptr FAMILY (8 sfx label-base
  sites → the t36 row-1781 cross-file build case); the type-layer 5 domain candidates; the 13-extern
  oracle boundary + the 4 `(ix+(field+k))` parser workarounds (escape-hatch census).
- 3(b) precision — **TAKEN (byte-neutral, lockstep):** the Sfx_Frame iy header under-claim (§3.1).
  Comment claims deferred to C3 (all verified clean). The `Task N`/`5a-5b` phase refs carried from the
  `.asm` are borderline codename-narration → at-next-touch ledger note (lockstep-only, not churned).

**Loop pass 1 — step-5 findings:**
- **NO byte-takeable — byte-frozen (STOP-not-absorb held).** Any algorithmic/cycle win MOVES BYTES =
  a STOP, deferred to the post-conversion optimization sweep. Interrogation logged per hot proc
  (Sfx_Frame walker / DrainQueue / DuckRamp / the SfxDispatch mailbox path): the per-iteration
  `ld de, SfxChannel_len` stride reloads are NECESSARY (de clobbered by the intervening calls);
  counter/cache (QUEUE_CNT + DUCK_TARGET/LEVEL) symmetric; the range-check + instance-cap + priority
  gates are the load-bearing guards (all commented); the non-latching-priority + dormant
  continuous-extend seam + cap-1-degenerates are commented as chosen.
- **C1 (cycle) INACTIVE — NAMED BASIS** (flagged, gate-reviewable): no in-source T-state/cycle
  annotations (grep clean; the only `T3` is the PSG const SND_PSG_SILENCE_T3); rung 4 owns Z80
  T-states; the file is byte-frozen so no cycle change is takeable.

**Loop pass 2:** the FULL 3→4→5 circuit came up empty (step 3 nothing new; step 4 built nothing — the
base_plus_idx + clone constructs stay ledgered/cross-file per the byte-freeze + t36 precedent; step 5
no byte-takeable). DRY → panel dispatched.

**Dry-panel (A1·B1·C2·C3, weighted to step-5; C1 inactive named-basis) — neither-bucket + takeable:**
- **C2 (correctness, highest weight) — CLEAN, CONFIRMS the porter.** All 3 mandated re-derivations
  confirm (Sfx_MusicChanPtr + Sfx_SelectVoice contracts EXACT; 11 externs sound conservative subsets;
  the iy-superset + invariant(ix) omission correct) + full sweep clean (CC-clobber, loop termination,
  Sfx_Restore push/pop balanced on EVERY exit, strides, priority-compare directions). Two harmless
  imprecisions noted-not-fixed (Fm_PatchPtr extern omits out(hl) = conservative, matches sequencer.emp;
  QueueEntryPtr/QueueEnqueue `clobbers(af)` = safe over-claim, a/b read-only, faithful to the `.asm`).
- **C3 (hardware prose) — CLEAN, ZERO t33-class rot.** 6 claim categories ALL VERIFIED against the
  resident tree (SFX_BLOB_BANK == engine-table bank + build fatals; SetBank $6000-latch/DMA-safe/
  mailbox-restore; de=$4001 + $2A re-park + both Sequencer_Frame re-park sites; channel-steal
  SCF_KEYED snapshot logic; Fm_*/Psg_* ix-preservation; the cached-no-op); 1 unverifiable-statically
  (the transcoder-drops-periodic-noise premise, carried verbatim from the twin, not port-introduced).
- **A1 (ceremony) — 1 real finding TAKEN byte-neutral + ledger asks.** TAKEN: the 3 table length
  guards `ensure(N==COUNT)` are RHS-only (weaker than the twin's `(End-Base)` emitted-length check);
  my comment overstated the mirror → corrected byte-neutral to state the limitation + ledger the
  data-proc `sizeof`/emitted-length ask (the fidelity gap is corpus-wide — fm/dac share it). LEDGERED:
  the Z80 named-locals/caller-save ask (the file's largest ceremony source); extended domain-types
  (Route↔Slot the strongest pair; DuckDepth; Maybe<Slot>/<Priority> sentinels). A1 spot-checked 5
  contracts → all accurate.
- **B1 (corpus-pattern) — ledger-only, demand data TRIPLED.** base_plus_idx 4→12 cross-corpus sites
  (independently confirmed the sfx 8; unified ix-base + label-base as ONE family — the sequencer
  hand-rolls both spellings). 3 more clone candidates: Sfx_ResolveBlob (2 sites), Sfx_RecordPtr (2),
  the active-channel-walk iterator (9× — the highest-frequency shape). Build-vs-ledger = overseer/
  step-6 call; nothing takeable in this byte-frozen tranche.

**DRY confirmed:** the panel round surfaced confirmations + ledger-deferred candidates + one
byte-neutral comment fix — nothing re-opening a byte-moving 3→4→5 pass.

## 5. Census — the t32 §5.1 sound_sfx row → DONE (the Z80 code front is DRIVER-ONLY after t37)

| Metric | t32 §5.1 (indicative) | Measured | Note |
|---|---|---|---|
| Top-level labels | 23 | **23** | 17 code procs + 3 data tables + 3 `_End` (folded into `ensure`) |
| Clobbers headers | ~28 | 28 | in band |
| Preserves headers | ~11 | 10 | in band |
| Out headers | ~11 | **7 reg/flag-out procs + 2 carry-arbitration** | Out-heavy CONFIRMED (the heaviest file yet) |

**Header-accuracy scoreboard — a NEW CLASS.** The running board was 36 procs / 6 OVER-claims (psg 3 +
fm 3, all safe-direction). t37 adds sfx's ~19 procs AND surfaces the **FIRST UNDER-claim** (Sfx_Frame
iy) — a DISTINCT class from the over-claims: an over-claim tells callers "worse than reality" (safe);
an under-claim tells callers "better than reality" (UNSAFE — a false-preserve). Board now:
**over-claims 6** (unchanged; sfx's other ~19 contracts verified accurate, the firing arc closed at 0),
**under-claims 1** (sfx Sfx_Frame iy — corrected in both twins, checker-invisible). The under-claim is
counted separately because it is the unsafe direction and drives the new `[call.clobbers-incomplete]`
diagnostic (§3.1).

## 6. Corrections list (the packet owns its errors)

- **Kill row 83 was DESIGNED at step 0 but not WRITTEN until the continuation** — a same-commit-rule
  miss (a designed kill row should land with its port commit, the t35 precedent). Written FIRST in the
  continuation per the overseer directive; logged here.
- **My step-1 honest-clobbers reasoning briefly overreached before the probe** — I initially assumed
  the checker would DEMAND iy on Sfx_Frame (transitive clobbers-completeness). The probe proved it does
  NOT (local-only) — which STRENGTHENED the finding into the checker-invisibility proof + the demanded
  diagnostic. The corrected understanding is §3.1.
- **No other loop-surfaced errors** — C2/C3 confirmed the porter on every re-derivation; A1's single
  finding (the guard-comment overstatement) is corrected.

## 7. Kill-list + ledger state

- Kill row **83** — `sound_sfx.asm` canonical twin; kill = the seam sub-tranche (also closes rows
  70/71/78 psg/fm/sequencer twins + the 2 driver die-at-port externs when rung 4 ports + the
  extern-decl-vs-def AND transitive-clobbers-completeness ledger rows).
- Ledger rows added: the transitive-clobbers-completeness diagnostic (§3.1); the type-layer 5 domain
  candidates; the base_plus_idx/ix_field_ptr 8-site addition (→ row 1781, count 4→12); the 3 B1 clone
  candidates; the A1 data-proc-`sizeof` ask + named-locals ask + extended domain-types; the panel
  C2/C3-clean record.

## 8. Overseer rulings applied (recorded)

- Kill row 83 written FIRST in the continuation (same-commit precedent); the miss logged as a
  corrections row.
- The Sfx_Frame iy under-claim: fixed in lockstep (byte-neutral), the checker-invisibility PROVEN
  (probe reverted), the demanded diagnostic LEDGERED as the extern-decl-vs-def partner.
- C1 INACTIVE named-basis ruling on the record; byte movement ZERO / STOP-not-absorb held; the panel
  takeable is byte-neutral only.

## 9. Residue for rung 4 (the driver top) + the seam sub-tranche

- **rung 4 (z80_sound_driver):** the LAST Z80 code front — after it the Z80 CODE conversion is
  COMPLETE. T-state capability FIRST (owns Z80 cycle costs; the C1 lens activates there). DEFINES the
  2 sfx die-at-port externs (SndDrv_SetBank / Snd_RouteClassFlags).
- **the seam sub-tranche — its INPUT SET IS NOW COMPLETE:** retires the 4 sound `.asm` twins (rows
  70/71/78/83), links the sound files as one module (extern → import, closing the extern-decl-vs-def
  drift structurally), closes the 2 driver die-at-port externs (when rung 4 ports), and is the kill
  vehicle for the base_plus_idx construct + the Z80 domain-newtype family + the transitive-clobbers
  and extern-decl-vs-def diagnostics.
