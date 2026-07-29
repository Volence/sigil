# 2026-07-29 — t37 checkpoint (b) packet (sound_sfx — the Out-heavy mirror, DRY by panel)

Status: **CHECKPOINT (b) — STOPPED for overseer gate (c).** Step-1 countersigned at (a); this
packet closes the 2→(3→4→5)→panel loop. Byte movement ZERO, STOP-not-absorb held. Gate (c), the
rebase, and the merge are the overseer's.

Tips (continuation): aeon `port-tranche37` @ `7c9bbd5` · sigil `port-tranche37` @ `<this commit>`. Masters
still sigil `103a4a7` / aeon `1cbd4fd` (the overseer executes the rebase onto the post-hoist-ambient
masters at the gate — NOT rebased here per standing practice).

## 0. Bars (own-verified)
- Canonical held EXACT: plain **4b66cace/421041** · debug **1c256b3b/429102** (whole-ROM dual rebuild
  after the lockstep `.asm` comment edit — byte movement ZERO).
- Strict paired **2832/0 (1 ignored)** = 2827 baseline + 5 sfx oracle tests. The `.asm`/`.emp`
  comment lockstep + the doc/ledger commits are all byte-neutral (sfx oracle re-ran 5/5).
- The four other resident Z80 `.emp` (sequencer/psg/fm) + all 68k files stayed READ-ONLY.

## 1. Continuation commits (post checkpoint-a)
1. sigil `05c02db` — **kill row 83** (sound_sfx twin; same-commit precedent, designed at step-0,
   written FIRST per the overseer directive) + **the transitive-clobbers-completeness ledger row**
   (the demanded diagnostic; §3 below).
2. aeon `a7d0b77` — the **Sfx_Frame iy header lockstep fix** (comment-only, byte-neutral; both twins
   now name iy + the reason; whole-ROM canonical unchanged, sfx gates 5/5).
3. sigil `810421d` — the **step-2 type-layer** + **step-3/4 construct** ledger rows.

## 2. Step 2 — modernize (Z80 flavor; the file CONFORMED from step-1 transcription)
The transcription already followed the sequencer.emp house format, so step 2 is a verification pass
(zero code change) + the type-layer ledger. The checklist (items 1-11):
- (1) Branch conversions — N/A (Z80: jr/jp/call/djnz are native, no bra/bsr/width-select).
- (2) Structural width-pins — N/A (Z80 has no width relaxation). The `(ix+(field+k))` forced spelling
  (4 sites: sx_patch_base+1 ×2, sc_stream_ptr+1, sc_base_freq+1) carries the FILE-HEADER note naming
  the deferred parser-fix ledger row (the sequencer.emp precedent — file-header, not per-site).
- (3) Bare-symbol spellings — conformant (bare imm symbols; hex `NNh`→`$NN`; no leftover h-suffix).
- (4) Brace-indent — conformant (section/proc/`if SFXID_REV_LOOP` bodies indent one level; labels
  keep the shallower offset).
- (5) Idiom list — Z80 subset: contract clause-order **out→clobbers→preserves, comma-joined** verified
  on all 20 procs; pure-data tables `pub proc () clobbers() { dc.b }` + RHS-only `ensure` length guard
  (the fm RegDeltaGroupBase precedent); the `_End`-label + `if len<>COUNT` asserts folded into `ensure`.
  (No 68k idioms apply: Sst/overlay/bare-abs-EA/dc.l/raise_exception/VDP-fns all N/A.)
- (6) Type-layer — LEDGERED not-adopted (§ below): 5 domain candidates (SfxId/Route/Kind/Slot/Priority)
  are compute-heavy (table-index arithmetic / cp chains) + the Z80 newtypes don't exist (T1 item-13).
- (7) Module-header — present-tense hardware/bank claims present + checkable (C3's audit surface).
- (8) Symbol-resolution ladder — the 13 `extern proc` are the WINDOWED-ORACLE boundary (the
  sequencer.emp precedent: co-resident defs exist but the oracle compiles sfx in isolation) → they
  DIE at the seam sub-tranche (kill row 83); the 2 driver externs are die-at-port (rung 4). Not a
  ladder violation — the resident-blob-oracle idiom.
- (9) Honest-contract derivation — the firing arc 3→0 + the iy superset (§3).
- (10) `as`-bless — N/A (no newtypes adopted).
- (11) Noticing — no new house-format item (the transitive-clobbers diagnostic is a checker ask, not
  a format item; the Z80 clause-order is already codified A1).

## 3. THE HEADLINE — the Sfx_Frame iy UNDER-claim (first in the sound corpus) + its diagnostic
`Sfx_Frame`'s `.asm` header declared `clobbers(af,bc,de,hl,ix)` — OMITTING `iy`, which its callees
Sfx_DrainQueue / Sfx_DuckRamp / Sfx_Restore destroy (their iy-walkers own it). Omitting it FALSELY
reads as iy-preserved (exhaustive-license semantics). The `.emp` declares the honest superset
`clobbers(...,iy)`; the `.asm` header is fixed in lockstep (byte-neutral, gate-proven).
**PROVEN checker-invisible:** removing iy from the `.emp` Sfx_Frame's clobbers does NOT fire — the
`[proc.clobbers-undeclared]` check enforces clobbers-completeness LOCALLY (own body writes), NOT
transitively across `call`s. So a clobbers under-claim silently drifts, exactly as a stale `extern
proc` preserves does (t36). **Ledgered as a demanded diagnostic** (`[call.clobbers-incomplete]` =
declared clobbers ⊇ reachable-callee-union ∪ local; the fixpoint the honest-contract rule already
specifies in prose); kill = the diagnostic landing OR the seam sub-tranche.

## 4. Contract firing arc: 3 → 0 (self-corrected, no finisher) + non-vacuity
The checker is complete (graduated t33). 3 initial declaration issues, all self-corrected in step 1:
QueueEntryPtr callee-preserves cascade (declared the bc/ix/iy it genuinely holds so QueueEnqueue's
preserves verify); Sfx_SelectVoice out/clobbers overlap on a+d (×2). Non-vacuity: an injected false
`preserves(hl)` on Sfx_Frame fires `[proc.preserves-unverifiable]` for h+l (reverted). The
extern-decl-vs-def by-hand set (13) is in the step-1 note §Evidence; all conservative-subset.

## 5. What each pass added (step-3 vs step-5)

**Loop pass 1 — step-3 findings:**
- 3(a) asks (all LEDGERED, byte-frozen): the `base_plus_idx`/`ix_field_ptr` FAMILY — 8 sfx-side
  label-base sites in Sfx_SelectVoice (the `ld hl,TABLE; add a,l; ld l,a; ld a,0; adc a,h; ld h,a`
  7-line idiom) → adds to the t36 row-1781 cross-file build case (3 seq + 1 fm + 8 sfx = 12 sites).
  The type-layer 5 domain candidates (compute-heavy, newtype-absent). The 13-extern oracle boundary
  + the 4 `(ix+(field+k))` parser workarounds (escape-hatch census).
- 3(b) precision: the Sfx_Frame iy header under-claim (§3, TAKEN byte-neutral in lockstep). Comment
  claims deferred to the C3 panel (hardware/bank) + the codename-narration note (the `Task N`/`5a-5b`
  phase refs carried from the `.asm` — borderline; ledger-or-lockstep-rewrite is an at-next-touch
  call, flagged for the panel/gate).

**Loop pass 1 — step-5 findings (byte-FROZEN — STOP-not-absorb held):**
- **NO byte-takeable.** The file is faithful port of tuned S3K-derived SFX-engine code; any
  algorithmic/cycle win MOVES BYTES = STOP, deferred to the post-conversion optimization sweep.
- Interrogation logged per hot proc (Sfx_Frame walker / Sfx_DrainQueue / Sfx_DuckRamp / the
  SfxDispatch mailbox path): Invariant-ladder — the per-iteration `ld de, SfxChannel_len` stride
  reloads are NECESSARY (de is clobbered by the intervening ModUpdate/Sequencer_Channel/writer calls),
  NOT hoistable. Counter/cache — SND_SFX_QUEUE_CNT + SND_SFX_DUCK_TARGET/LEVEL writers/readers are
  symmetric. Guard-coverage — the range-check appears in BOTH SfxDispatch + Sfx_BeginSound (commented
  belt-and-suspenders); the instance-cap + priority gates are the load-bearing guards. Silent-tradeoff
  — the non-latching-priority (bit7), the dormant continuous-extend seam, cap-1-degenerates are all
  commented as chosen. Debug-growth $8000 bar — N/A (sfx has no debug blocks; `.emp` not wired).
- **C1 (cycle) INACTIVE — NAMED BASIS** (flagged, gate-reviewable): NO in-source T-state/cycle
  annotations in sound_sfx.asm (grep clean; the only `T3` is the PSG register const SND_PSG_SILENCE_T3);
  rung 4 owns Z80 T-states; the file is byte-frozen so no cycle change is takeable.

**Loop pass 2:** the FULL 3→4→5 circuit came up empty (step 3 nothing new, step 4 built nothing —
the base_plus_idx construct stays ledgered/cross-file per the byte-freeze + t36 precedent; step 5 no
byte-takeable). DRY → panel dispatched (A1·B1·C2·C3, weighted to step-5; C1 inactive named-basis).

## 6. Dry-panel (A1+B1+C2+C3, weighted to step-5; C1 inactive named-basis) — DRY
One round, 4 fresh read-only analysts. **No finding re-opens a byte-moving cycle → DRY.** All
takeables byte-neutral (comment) or ledger-deferred (byte-frozen). Adjudication:

- **C2 (correctness, highest weight) — CLEAN, CONFIRMS the porter.** All 3 mandated re-derivations
  confirm: (1) Sfx_MusicChanPtr `out(iy,carry:found) clobbers(af,bc,de) preserves(hl,ix)` EXACT +
  Sfx_SelectVoice `out(a,d,carry:dropped) clobbers(f,bc,e,hl,ix,iy)` EXACT (the self-corrected one);
  (2) all 11 co-resident externs sound conservative subsets under exhaustive-license; (3) Sfx_Frame
  iy genuinely destroyed by its callees (superset mandatory) + invariant(ix) omission correct (walker
  reloads ix, the t36 §3.4 reliance). Full sweep clean: no CC-clobber between test/branch, every
  loop terminates, Sfx_Restore push/pop balanced on EVERY exit (.no_music/.fm/.psg/noise all reach
  the single `pop ix`), strides correct (SeqChannel_len vs SfxChannel_len vs SFXHC_LEN, no cross-up),
  priority-compare directions all match comments. Two harmless imprecisions noted-not-fixed
  (Fm_PatchPtr extern omits out(hl) — conservative, matches sequencer.emp's decl; QueueEntryPtr/
  QueueEnqueue `clobbers(af)` — safe over-claim, a/b read-only, faithful to the `.asm`).
- **C3 (hardware prose) — CLEAN, ZERO t33-class rot.** 6 claim categories ALL VERIFIED against the
  resident tree: SFX_BLOB_BANK == engine-table bank (main.asm:314 + the 425/432 build fatals);
  SetBank $6000-latch/DMA-safe/leaves-set + mailbox-wrapper restore; de=$4001 + $2A re-park + both
  Sequencer_Frame call sites re-park (SndDrv_TimerATick/IdleTick); channel-steal SCF_KEYED-snapshot
  logic (sequencer .note/.rest set/clear BEFORE the override gate); Fm_*/Psg_* ix-preservation
  (grep: zero ix-writes); the cached-no-op. 1 unverifiable-statically (the transcoder-drops-periodic-
  noise premise — carried verbatim from the twin, NOT port-introduced).
- **A1 (ceremony/language) — 1 real finding TAKEN byte-neutral + ledger asks.** TAKEN: the 3 table
  length guards `ensure(N == COUNT)` are RHS-only (weaker than the twin's `(End-Base)` emitted-length
  check); my comment overstated the mirror → corrected byte-neutral to state the RHS-only limitation
  honestly + ledger the data-proc `sizeof`/emitted-length ask (aeon `7c9bbd5`; sfx gates 5/5).
  LEDGERED: the base_plus_idx ceremony (8 sites); the Z80 named-locals/caller-save ask (the file's
  largest ceremony source — register-narration + push/pop + RAM-spill + the duplicated iy-recompute);
  extended domain-types (Route↔Slot the strongest pair; DuckDepth; Maybe<Slot>/<Priority> sentinels).
  A1 spot-checked 5 proc contracts → all ACCURATE (confirms porter). 2 low-pri inherited-`.asm`
  loose comments (`.emp:209/668` "add a,a") → ledgered at-next-touch (lockstep-only, marginal).
- **B1 (corpus-pattern) — ledger-only (byte-frozen), demand data TRIPLED.** base_plus_idx: 4→12
  cross-corpus sites (independently confirmed the sfx 8; unified ix-base + label-base as one family —
  the sequencer hand-rolls both spellings). 3 more clone candidates: Sfx_ResolveBlob (2 sites),
  Sfx_RecordPtr (2), the active-channel-walk iterator (9× — highest-frequency shape). All build-vs-
  ledger = overseer/step-6 call; nothing takeable in this byte-frozen tranche (the t36 build-nothing
  precedent).
- **C1 INACTIVE — NAMED BASIS** (flagged): no in-source T-state/cycle annotations (grep clean; the
  only `T3` is the PSG const SND_PSG_SILENCE_T3); rung 4 owns Z80 T-states; byte-frozen.

**DRY confirmed:** the panel round surfaced confirmations + ledger-deferred candidates + one
byte-neutral comment fix — nothing re-opening a byte-moving 3→4→5 pass.

## 7. Census — the t32 §5.1 sound_sfx row → DONE
| Metric | t32 §5.1 (indicative) | Measured | Note |
|---|---|---|---|
| Top-level labels | 23 | **23** | 17 code procs + 3 data tables + 3 `_End` (folded into `ensure`) |
| Clobbers headers | ~28 | 28 | in band |
| Preserves headers | ~11 | 10 | in band |
| Out headers | ~11 | 7 reg/flag-out procs + 2 carry-arbitration | Out-heavy CONFIRMED (heaviest yet) |

**Header-accuracy scoreboard:** the FIRST clobbers UNDER-claim in the sound corpus (Sfx_Frame iy) —
distinct from the 6 psg/fm OVER-claims (which were all safe-direction). Corrected in the `.emp` + the
`.asm` lockstep. sfx's other ~19 procs: contracts accurate as declared (the firing arc closed at 0).

## 8. Corrections list (the packet owns its errors)
- **Kill row 83 was DESIGNED at step 0 but not WRITTEN until the continuation** — a same-commit-rule
  miss (a designed kill row should land with its port commit). Written FIRST in the continuation per
  the overseer directive; logged here per the t35 corrections precedent.

## 9. Kill-list + ledger state
- Kill row **83** — sound_sfx.asm canonical twin; kill = the seam sub-tranche (also closes the 2
  driver die-at-port externs + the extern-decl-vs-def & transitive-clobbers ledger rows).
- Ledger rows added: transitive-clobbers-completeness diagnostic; the type-layer 5 domain candidates;
  the base_plus_idx/ix_field_ptr 8-site construct addition (→ row 1781).

## 10. Residue for rung 4 (the driver top) + the seam sub-tranche
- **rung 4 (z80_sound_driver):** the LAST Z80 code front; T-state capability FIRST (owns Z80 cycle
  costs — the C1 lens activates there); DEFINES the 2 sfx die-at-port externs (SndDrv_SetBank /
  Snd_RouteClassFlags). After it the Z80 code front is COMPLETE.
- **the seam sub-tranche:** retires the 4 sound `.asm` twins (rows 70/71/78/83), links the sound files
  as one module (extern → import), closes the extern-decl-vs-def + transitive-clobbers ledger rows,
  and is the build vehicle for the base_plus_idx construct + the Z80 domain-newtype family.
