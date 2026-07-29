# 2026-07-29 — t37 checkpoint (a): sound_sfx step-1 COMPLETE (the Out-heavy mirror)

Status: **CHECKPOINT (a) — STOPPED for overseer countersign.** Step-1 (faithful transcription +
dual-shape windowed oracle + full contract set + t24 controls) is done; byte movement ZERO,
STOP-not-absorb held. Tips: aeon `port-tranche37` @ `18c3c7f` · sigil `port-tranche37` @ `b5c2790`
(+ step-0 `7bbc23f` / this note). Masters: sigil `103a4a7` / aeon `1cbd4fd`.

## Evidence block

### Windowed byte gates — BOTH shapes GREEN (the psg t32 case)
- `sound_sfx_matches_as_twin_plain` — 1516 B ($5EC), window **$0CD7..$12C3**, DEBUG=0. GREEN.
- `sound_sfx_matches_as_twin_debug` — 1516 B ($5EC), window **$0D55..$1341**, DEBUG=1. GREEN.
- `plain_and_debug_shapes_differ` — GREEN (base + fm/psg targets differ +$7E).
- `emp_diverges_from_doctored_twin` (t24 positive control, moved SfxBlobWinTab) — GREEN.
- `doctored_both_sides_stay_equal` (both ModUpdate=$1234) — GREEN.
- **5/5.** Shape-variant BASE (+$7E), shape-INVARIANT LAYOUT (identical 1516 B both shapes; sfx has
  no own `__DEBUG__` blocks — the +$7E is the sequencer's growth preceding this window).

### Whole-ROM CRCs — byte movement ZERO
- plain **4b66cace / 421041** · debug **1c256b3b / 429102** — canonical, UNCHANGED (the `.emp` is not
  wired into build.sh; the `.asm` twin is untouched — `git status` shows only `sound_sfx.emp` added).

### Own strict counts
- `SIGIL_STRICT_GATE=1 AEON_DIR=<t37-aeon-worktree> cargo test --workspace --release`
  → **2832 passed / 0 failed / 1 ignored** = the 2827 baseline + 5 new sfx oracle tests, zero
  regressions. Baseline re-confirmed 2827/0 before any edit.

### Contract firing arc: 3 → 0 (self-corrected in-tranche, NO finisher)
The checker is complete (rung-2 graduated at t33); my initial contract declarations had 3 issues,
all self-corrected during step 1:
- (1) `Sfx_QueueEnqueue` `preserves(iy)` was unverifiable until `Sfx_QueueEntryPtr` (its callee)
  declared the preserves it genuinely holds — `preserves(bc, ix, iy)` (body touches only a,h,l,d,e).
  The callee-preserves-declaration discipline: a caller's `preserves(X)` needs every callee on the
  path to DECLARE `preserves(X)`. Also fixes `Sfx_QueueEnqueue`'s reliance on `bc` surviving the
  append-path `call Sfx_QueueEntryPtr` (it holds c=id/b=prio across it).
- (2,3) `Sfx_SelectVoice` `out(a, d, carry: dropped)` overlapped `clobbers(...a...d...)` — a register
  is either output or scratch, not both. Fixed to `clobbers(f, bc, e, hl, ix, iy)` (a+d are the outs;
  f/e carry the af/de scratch halves).

### Contract-gate NON-VACUITY proven (mirrors t33)
Injected a false `preserves(hl)` on `Sfx_Frame` (which clobbers hl) → the checker fires
`[proc.preserves-unverifiable] 'h'/'l' written and not restored` (both halves). Reverted → green.

### The extern-decl-vs-def BY-HAND verification set (C3 — no machine cross-check exists)
13 externs verified conservative-subset against the co-resident defs (full table in the step-0 note
§3): 11 co-resident (ModUpdate/Sequencer_Channel ex sound_sequencer.emp; Fm_PatchLoad/NoteOff/
SetVolume/PatchPtr/NoteOnFreqExact ex sound_fm.emp; Psg_NoteOff/SetVolume/NoteOn/Noise ex
sound_psg.emp) + 2 driver DIE-AT-PORT boundary externs (SndDrv_SetBank `preserves(bc,de,ix)`;
Snd_RouteClassFlags `preserves(bc,de,hl,ix)`). Notable: **Sequencer_Channel is declared WITHOUT
`preserves(ix)`** — matching its co-resident def's honest omission (the t36 checker-limitation);
`Psg_SetVolume` declared `preserves(de,hl,ix)` (matches sequencer.emp's tighter decl). These 11
co-resident decls are new instances of the t36-ledgered extern-decl-vs-def silent-drift hazard.

### The invariant(ix) adjudication — the t36 MIRROR (confirmed by ledger row 1777)
NO module `invariant: preserves(ix)`. `Sfx_Frame` is the SFX-side channel walker
(`ld ix,SND_SFX_CHANNELS` … `add ix,de`) that reads `(ix+sc_flags)` across `call Sequencer_Channel`
— relying on ix survival that is TRUE but checker-invisible across the shared interpreter's `ex(sp),hl`
computed dispatch (ledger row 1777 names sound_sfx.asm:294 as the 2nd relied-upon site). Per-proc
`preserves(ix)` declared on the verifiable leaves (Steal/MusicKeyOffKeepKeyed/Restore push/pop-
bracketed; MusicChanPtr/DeepestDuck/MinActiveKind iy-walkers; RouteKind/QueueEntryPtr/QueueEnqueue).
The dispatch/allocation/walker procs (Frame/DrainQueue/DuckRamp/SfxDispatch/BeginSound/SelectVoice/
StopAll/SlotPtr-out) clobber or produce ix and correctly OMIT preserves(ix). Adjudicated, not forced.

### Header-accuracy vs the 36/6 scoreboard
- **The FIRST under-claim in the sound corpus: `Sfx_Frame` `.asm` header omits `iy`** — but its
  callees Sfx_DrainQueue/Sfx_DuckRamp/Sfx_Restore destroy iy (their iy-walkers), so omitting it would
  falsely tell callers iy is preserved. The `.emp` declares the honest superset `clobbers(...,iy)`.
  PROVEN checker-invisible: removing iy from Sfx_Frame's clobbers does NOT fire (the checker enforces
  clobbers-completeness LOCALLY, not transitively across calls — the discipline, not the checker,
  demands the honest superset). Safe: the sole caller Sequencer_Frame also clobbers iy.
- Minor safe over-claim carried faithfully: `Sfx_QueueEnqueue` `.asm` "Clobbers bc" (only c is
  written; b is a read-only input) — kept as declared (over-claim-safe direction; not tightened).

### out(carry:) consumer coverage (by-hand; all INTERNAL — unlike t36's cross-file)
- `Sfx_MusicChanPtr out(carry: found)` → consumed by `jr c` (Sfx_Steal) + `jp c` (Sfx_Restore) — 2.
- `Sfx_SelectVoice out(carry: dropped)` → consumed by `jp c, .chan_drop` (Sfx_BeginSound) — 1.
- All lower clean. The machine `[call.flag-result-unused]` cross-proc credit runs via the CORPUS
  contract pass (not the windowed oracle's `lower_module`, per t36 §3.2) — a drop-probe non-vacuity
  run is available for the gate if wanted (mirrors t36).

### Census — the t32 §5.1 sound_sfx row
23 top-level labels = **17 code procs + 3 data tables (SfxEligTable/SfxRouteSlot/SfxSlotRoute, each
`pub proc () clobbers() { dc.b }` + an RHS-only `ensure` length guard) + 3 `_End` markers** (folded
into the tables — no separate emission). Out-heavy CONFIRMED: 7 register/flag-out procs + 2
carry-arbitration procs (heavier than psg/fm/sequencer).

## Where I stopped
Checkpoint (a): step-1 complete + committed (aeon `18c3c7f` sound_sfx.emp; sigil `b5c2790`
sound_sfx_port.rs; step-0 `7bbc23f`). NOT started: step 2 (modernize), the loop (3→4→5), the
dry-panel, step 6, merge. Awaiting overseer countersign. No STOP condition surfaced; every brief
prediction held (psg-class base, no own trampoline, Out-heavy, invariant(ix) walker mirror).
