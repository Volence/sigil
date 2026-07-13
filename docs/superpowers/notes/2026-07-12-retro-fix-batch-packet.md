# Retro-fix batch `retro-fix-audit-1` — implementation packet

**Date:** 2026-07-12. **Branch:** `retro-fix-audit-1` off master, both repos
(NOT merged — Volence's gate). Source of truth: the 12-item brief in
`notes/2026-07-12-steps2-5-retro-audit.md` (§Retro-fix batch brief) + its
per-file findings.

**Final state:** full workspace strict suite **2210/0**, clippy clean, both
aeon shapes build. `repin` idempotent (`pins.rs unchanged` on a fresh run).
Plain `s4.bin` md5 `dbc0126d…`, debug `s4.debug.bin` md5 `7fbb9e7e…`.

**Commits (per-item-group, lockstep):**

| aeon | sigil | items |
|---|---|---|
| `ff646c8` | `d647a97` | 1 + 12 (A2 walk-live rail) |
| `a2b7efd` | `1fd98a7` | 2–9 (asserts / ensures / contracts) |
| `ff797bd` | `2d6f95f` | 5 / 10 / 11 (byte-changing) |
| `64ef75f` | `5520c38` | **item 6 REMOVED** (soak finding) |

**One deviation from the literal brief** — commit granularity: the brief's
"green after every item" (hard rule) and "defer all pins to one re-pin wave"
conflict (the port tests won't compile without the regenerated `pins.rs`). I
kept every commit green (pins.rs + baseline regenerated per byte-changing
commit) and ran `repin` as the authoritative idempotent wave at the end. The
pins diff is still reviewable as one coherent movement (below).

---

## Per-item outcomes

### Item 1 + 12 — A2 walk-live rail — **DONE**
*(audit §collision+aabb HEADLINE; §RULINGS 1; §entity_window finding 1)*

A DEBUG-only RAM flag `Dynamic_Live_Walking` (reuses the release pad slot after
`Dynamic_Live_Dirty` → **Engine_RAM_End shape-invariant**, zero downstream RAM
movement, plain ROM byte-identical). Set/cleared (`st`/`sf`) around all FOUR
dynamic live-list walks — `.run_culled` + `RunObjects_Frozen`'s `.frozen_dyn`
(core), `TouchResponse`'s `.dyn` segment (collision), `EntityWindow_Despawn­Objects`
(entity_window, item-12 rider) — and asserted CLEAR (`assert.b d0, eq, #0`) at
`CompactDynamicLive` entry. Every st/sf/assert sits inside `if DEBUG == 1`, so
release is byte-neutral (debug grows +0x6C core / +0x8 collision / +0x8
entity_window). entity_window fence honored: only the DespawnObjects hook
touched. **A2 soak report below.**

### Item 2 — DeleteObject asserts — **DONE** *(audit §core findings 3 + 4)*
`if DEBUG == 1` entry rails: `a0` within `Object_RAM..Object_RAM_End`
(out-of-range → wild-memory `.clear_slot`) + `code_addr != 0` (double-delete →
free-stack corruption). Register-comparand form (memory operand loaded to d0
first, per RingBuffer_Add). Byte-neutral in release.

### Item 3 — core doc/comment/import — **DONE** *(audit §core findings 2 + 5 + 6)*
AllocDynamic header gains the set-code_addr-immediately caller invariant;
unused `NUM_TOTAL_SLOTS` import removed; CompactDynamicLive's false "before any
dispatch this frame" comment corrected to name the A2 rail. All byte-neutral.

### Item 4 — animate script rails — **DONE** *(audit §animate findings 1–4)*
`.evt_set_field`: `assert.w d0, lo, #SST_len` + `assert.w d0, ne,
#SST_mapping_frame` (bare cross-seam symbols — `#Object_RAM` precedent — so the
auto-message spelling matches the twin). `.cc_back`: `assert.b d0, ne, #0`
(AF_BACK N≠0 hang). `.cc_change` + `.cc_end`: site comments for the
AF_CHANGE-to-self freeze and the frameless-script hang (no cheap register
comparand; the typed-script DSL is the full cure — byte-command-DSL ledger row).
Debug shape grows; plain byte-neutral.

### Item 5 — animate drop both Sound_PlaySFX saves — **DONE (byte-changing −8 B)**
*(audit §animate finding 5; §clobbers ruling)*
Under the exhaustive-license ruling `Sound_PlaySFX` clobbers only d0, so a1/d1
are contractually preserved and the `movem.l a1/d1` save/restore pair was dead.
Dropped. a1 is reused in `.after_event`; d1 reloaded there.

### Item 6 — dplc single-entry assert — **REMOVED (soak finding)**
*(audit §dplc finding 2 — the audit conditioned it "if the invariant is REAL
corpus-wide")*
Shipped in the items-2–9 commit, then **the A2 oracle soak HALTED on it**:
TestPlayer renders through `DPLC_Sonic`, whose frames legitimately carry up to
6 DPLC entries (verified a `0x0006` entry-count word in the frame data; call
stack `MDDBG__ErrorHandler ← Perform_DPLC`). perform_dplc's own entry loop is
built for N entries, so the "exactly 1 entry" note is false and the assert
fired on valid data. **Removed both procs' assert**, corrected the header
(multi-entry supported; a single-DMA guarantee, if wanted for shipping art,
belongs in the build tool). dplc lost its only DEBUG divergence → debug shape
== plain again.

### Item 7 — aabb ensure + edge comment — **DONE** *(audit §aabb finding 3 + 4)*
`ensure(stmp != cdim)` + `ensure(stmp != delt)` inside the `aabb_axis_test`
comptime-fn body (Reg-equality is comptime-decidable — `lead_move` precedent;
confirmed `ensure` works in fn-body position). One-line comment on the
delta == −32768 `neg.w` edge. The `tranche7_negative_probes` probe (f) flipped
from "assembles clean" to "compile error naming the constraint" — **resolves
the ledger's tranche-7 distinct-regs row.** (aabb has no `.asm` twin for the
ensure — the `.inc` macro can't express it; the ensure is an `.emp`-only
improvement.)

### Item 8 — vdp_init ensure + hblank comment — **DONE** *(audit §old-loop findings)*
`ensure(VDP_Shadow_len <= 32)` (item-position; btst mod-32 dirty indexing).
hblank handler-contract comment (handlers may clobber only d0-d1/a0). Both
byte-neutral.

### Item 9 — sound_api rewrite + core RAM-adjacency ensure — **DONE**
*(audit §clobbers ruling; §core finding 5)*
sound_api's "incidental — do not rely" paragraph rewritten per the
exhaustive-license ruling (a1/d2-d7/SR are contractually preserved). Core gains
`ensure(extern("Effect_Slots") == extern("System_Slots") + sizeof(Sst)*NUM_SYSTEM)`
— **extern-in-ensure over RAM labels WORKS** (captured + passing link assert; no
ram.asm fallback needed — the ledger's "link-time ensure" ask is answered
positively). Locks RunObjects_Frozen's System+Effect contiguous-sweep
assumption.

### Item 10 — rings DrawRings camera-bias fold — **DONE (byte-changing)**
*(audit §rings finding 1; §byte-shackle sweep)*
Pre-bias the cached camera regs (`d6 = Camera_X − (VDP_SPRITE_X_OFFSET −
RING_WIDTH/2)`), so `sub.w d6,d2` yields the SAT coord directly, dropping the
per-ring subi/addi pairs (−3 ops/ring in the hot loop). The cull `addi`
immediate compensates to reproduce the **EXACT pre-fold `d0`** (`cmpi`
unchanged → wraparound-safe — the naive "add BIAS to the cmpi" is NOT
wraparound-equivalent; documented in the source). SAT bytes identical.
**DEVIATION from the audit's −16 B estimate: net −6 B.** The cull `addi` cannot
be *dropped*, only *compensated* (same instruction, changed immediate) — the
audit's −16 assumed it vanished. Oracle-verified: rings render identically (see
soak note; the fold produces byte-identical SAT X/Y).

### Item 11 — dplc enqueue-order fix — **DONE (byte-changing)** *(audit §dplc finding 3)*
Design choice: **give QueueDMATransfer the carry-return** (the alternative in
the brief). The proc header ALREADY documented "carry set = queue was full" —
the impl never honored it (it restored the caller's entry SR on both paths, so
the carry was garbage and every carry-checking caller — notably `bg_anim`'s
`bcs .queue_full` retry — was **silently dead**). Now `.full` sets carry, the
success paths clear it (`ori.b`/`andi.b #…, ccr` after the SR restore).
`perform_dplc` commits `prev_frame` only AFTER a successful enqueue (`bcs .done`
skips the commit + re-reads mapping_frame to commit post-loop), so a full queue
no longer leaves the object believing stale art loaded. **bg_anim.asm source
untouched** — its existing `bcs` now works as its author intended (behavior
becomes correct, which the brief's "keep unchanged" is read as "don't modify
its source"). Known remaining edge documented in the QueueDMATransfer header: a
128KB-split-with-one-slot still returns carry-clear (atomic rollback is the
art-streaming plan's Vectorman work; vanishingly rare for dplc — a small source
can still straddle a 128KB boundary — not impossible).

dma_queue's +12 B (3 CCR ops) shifts the whole engine block +0xC.

---

## A2 soak report (items 1+12 acceptance — feeds Volence's A2 design ruling)

**Method:** debug ROM `s4.debug.bin` in oracle, symbols from `s4.debug.lst`,
press-only frame driving (no bare `resume`, no `step_out`/`press` interleave).
ObjectTest scene entered by writing `GameState_ObjectTest_Init` to `Game_State`
at runtime. Breakpoints on `MDDBG__ErrorHandler` (catches ANY assert) and
`CompactDynamicLive`.

**Results (~1300 frames in ObjectTest):**

1. **NO assert fires** — `MDDBG__ErrorHandler` 0 hits across the whole soak
   (after item 6 was removed; see below). The A2 assert never fires.
2. **`CompactDynamicLive` is not invoked naturally.** The ObjectTest dynamic
   pool saturates to 40/40 (NUM_DYNAMIC) at init and stays **static** — the
   scene's per-frame churn is in the EFFECT pool (`AllocEffect`, which never
   touches the dynamic live list). With no dynamic deletions, `Dynamic_Live_Dirty`
   never sets (no frame-end compaction) and the free stack stays empty (allocs
   fail the free-stack check before reaching the compact-on-full guard). So the
   A2 mid-walk-compact hazard's **trigger is dormant** in this scene.
3. **Positive control** — inducing a frame-end compaction (write
   `Dynamic_Live_Dirty = 1`) DID call `CompactDynamicLive`; at its entry
   `Dynamic_Live_Walking` read **0x00**, and single-stepping confirmed the
   assert **falls through cleanly** (PC advanced into the compaction logic, NOT
   to the error handler). The rail's frame-end path and the no-false-positive
   behavior are validated.

**The soak's headline catch: item 6.** On the FIRST soak run the game halted at
`MDDBG__ErrorHandler ← Perform_DPLC` — the item-6 single-entry assert firing on
`DPLC_Sonic`'s legitimate multi-entry frames. This is exactly the value of a
soak: it caught an over-strict assert the gate could not (the gate uses
contiguous test art or none). Item 6 removed; re-soak clean.

**Recommendation for the A2 ruling:** the hazard did NOT manifest — but the
ObjectTest scene does not exercise the trigger (`CompactDynamicLive` under a
live dynamic walk), so this is "not reached", not "proven safe". The rail is
cheap, gate-proven installed, and its clean path is validated. Keep the rail;
the design fix (alloc-fail / latch / hole-fill, occupancy amendment A2) remains
gated on the rail ever firing in a scene with genuine dynamic-pool churn
(deletes + mid-dispatch respawns). A churn-first ObjectTest variant would be
the definitive stress.

**Churn profile (conditional add-on): LEFT OPEN.** The oracle profiler's
stale-after-ROM-reload bug is unfixed on oracle main (the flush fix is on
`linux-port`, not merged), and the soak reloaded the ROM — so profiler frame
shares would be stale. Per the brief's rule (c), the soak answered fire/no-fire
only; the entity-window churn-profile ledger row stays OPEN.

---

## Re-pin diff (master → branch)

Plain shape moved only from the byte-changing items (5/10/11); everything else
is DEBUG-shape-only (self-gating asserts) or comment/ensure.

New symbol: `DYNAMIC_LIVE_WALKING: u32 = 0xFFFFB065` (debug_only).

Region base/len movements (plain → new / debug → new):

```
HBLANK       plain 227A→2286  debug 2308→2314   (dma_queue item-11 +0xC)
CONTROLLERS  plain 228C→2298  debug 231A→2326
GAME_LOOP    plain 22FE→230A  debug 238C→2398
MATH         plain 2464→2470  debug 25F6→2602
DPLC         plain 26FC→2708  debug 288E→289A   len 98→A4  (item-11 restructure)
CORE         plain 2794→27AC  debug 2926→293E   dlen 546→6C8 (items 1/2 asserts, debug-only)
ANIMATE      plain 2E38→2E50  debug 328C→3426   plen 192→18A (item 5 −8)  dlen 192→2A8
RINGS        plain 31CA→31DA  debug 361E→38D6   plen 1BE→1B8 (item 10 −6)  dlen 21A→214
SOUND_API    plain 5D3C→5D46  debug 73A4→765E
DELETE_OBJECT      284E→2866        29E0→29F8
CC_DELETE_OFF  usize 104 → ShapeOffset {104, 15E}   (item 4, now shape-dep)
REFRESH_OFF    usize 174 → ShapeOffset {16C, 28A}   (item 5 shrink + item 4)
RINGCOL_OFF    ShapeOffset {11C,178} → {116,172}    (item 10)
```

engine.inc: all 14 `SIGIL_EMP_*` gate resume orgs re-derived (both shapes across
the byte-changing wave; the debug-only asserts re-pinned the debug orgs).
mixed_dac_rom: the early sound-tranche maps hardcode engine-block addresses
(pre-pin era) — hblank/controllers/game_loop/math bases + the game_loop window
slice/disp anchor bumped +0xC (object-bank `$10000+` regions unchanged, absorbed
by `org $10000`).

---

## Ledger rows closed / updated (see the companion edits to campaign-gap-ledger.md)

- **`retro-audit A2 rider` (OPEN → landed)** — the entity_window DespawnObjects
  walk-live hook shipped (item 12); the Compact assert is now TOTAL over all
  four walkers.
- **dplc §HAZ prev_frame-before-enqueue (audit dplc finding 3)** — fixed
  (item 11); QueueDMATransfer's documented carry contract now honored;
  bg_anim's dead retry revived.
- **tranche-7 `distinct(a,b)` / aabb non-alias (ledger row ~724)** — RESOLVED
  by the item-7 comptime `ensure` (Reg-equality); probe (f) flipped.
- **diag-retrofit follow-on sweep (ledger row ~988): DeleteObject range +
  animate underflow asserts** — landed (items 2, 4).
- **entity-window churn-profile debt** — stays OPEN (profiler fix unavailable;
  soak did fire/no-fire only).
- **NEW ledger note: item-6 single-entry invariant is FALSE** — DPLC_Sonic
  carries multi-entry frames; a single-DMA guarantee is a build-tool check, not
  a runtime assert. (perform_dplc's entry loop is load-bearing, not dead
  generality — reverses the audit's dplc-finding-2 `[OPT]` speculation.)

---

## What each pass added

**Step-3 (contract/claim/comment) findings applied:** the exhaustive-license
rewrite (sound_api, item 9), the AllocDynamic caller-invariant + CompactDynamicLive
comment correction (item 3), the hblank handler contract (item 8), the dplc
header correction (item 6 removal), the DrawRings wraparound-safety note (item 10).

**Step-5 (optimize / invariant) findings applied:** the DrawRings camera-bias
fold (item 10, hot loop), the animate dead-save drop (item 5), the dplc
enqueue-order correctness fix (item 11).

**Neither-bucket (live findings / verification):** the A2 rail (a hazard rail,
not an opt), the DEBUG asserts (items 2/4), the comptime ensures (items 7/8/9),
and — the batch's headline — the **oracle soak disproving item 6**, the one
finding that only live verification could produce.

---

## PRECEDENT — the assert → disproof → fix-the-claim pattern (Volence, 2026-07-12)

Item 6 is the model outcome for a shipped assert that a soak contradicts.
When a DEBUG assert derived from a documented CLAIM ("exactly 1 DPLC entry")
FIRES on legitimate live data, the resolution is NOT to silence the assert or
special-case the data — it is:

1. **Trust the soak over the claim.** The assert firing on real data
   (`DPLC_Sonic`, 6-entry frames) is EVIDENCE the invariant is false, not a
   nuisance. The gate could not catch this (test art or none); only live
   verification against real content could.
2. **Remove the assert AND correct the source claim that spawned it.** The
   `.emp`/`.asm` header's "each frame has exactly 1 DPLC entry" comment was
   itself wrong — it was corrected to state the truth (N entries; the loop is
   load-bearing), so the next reader is not re-misled into re-adding the assert.
3. **Record the reversal at the finding's origin.** The audit's dplc-finding-2
   `[OPT]` ("if the invariant is real, the entry loop is dead generality") is
   explicitly reversed in the ledger — the loop is NOT dead.

A DEBUG assert is a HYPOTHESIS about an invariant; a soak is the experiment.
When the experiment refutes the hypothesis, the deliverable is the corrected
claim, with the disproof recorded — never a green assert kept green by narrowing
what it runs on.
