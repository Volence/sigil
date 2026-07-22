# Phase 2 (pass-3) design gate — §C trigger checks + batching plan

**Author:** Opus (self-overseen Phase-2 lead). **Status:** **design gate PASSED — overseer-
confirmed 2026-07-21** (all three escalations ruled; parcel order A→B→C→8b→D→E approved, 8b
lands before Tier-B). **Date:** 2026-07-21.

**State entering:** Phase 1 COMPLETE — the D1b `[call.input-undefined]` ERROR gate is live on
the verified-out fixpoint (sigil master `871ec7d` / roadmap rider `94d65da` / aeon `ae1de4d`
UNCHANGED). Canonical ROMs plain `3aa43cb6`/420749 · debug `ce0e83a6`/428768. **The net is
live before the cut.** Pass-3 has cut ZERO code — everything merged since pass-2 (G4, G4.5
#1–#4, S2-D6 #3) was byte-neutral analysis-only, so **the current ROM's hot-path profile IS
the post-pass-2 profile** the §C triggers ask about (no confound).

---

## 1 · §C trigger checks (the design gate's gating question) — MEASURED

Method: canonical `s4.bin` (`3aa43cb6`/420749, verified hash + reload) in oracle, symbols from
the co-built `s4.lst` (identical mtime). `reset → start 240f` into the OJZScroll scene, then a
per-regime held-input drive under the CPU profiler (near-zero overhead), accumulator reset
between regimes. Frame budget **128000 cy**; NTSC VBlank window **~18,560 cy** (~14.5% of frame).
Three regimes; `VInt_Level` = the true per-frame VBlank CPU cost (dispatch + `Process_DMA_Critical`
+ `VInt_DrawLevel` drain + parallax VSCROLL).

| Regime (drive) | main-loop idle (`VSync_Wait`) | VBlank (`VInt_Level`) | % of VBlank window | `FindStagedBlock` |
|---|---|---|---|---|
| diagonal ↓→ (120f) | 7.9% *(+ crossing spikes)* | ~6.0k cy (VBlank_Handler) | ~33% | 2215 cy / 1.7% / 6 calls |
| max-H → (80f) | **48.3%** | 10256 cy | **~55%** | 3656 cy / 2.9% / 11 calls |
| max-V ↓ (60f, heaviest) | 34.9% | 9027 cy | ~49% | **6641 cy / 5.2% / 19 calls** |

*(max-V idle 34.9% here vs pass-2's ~53% is expected, not a regression: this is the post-
Deep-Forest-BG-merge ROM (parallax + BG cost the OJZ scene now carries) measured on a raw
held-input down-drive that includes cold-crossing frames, whereas pass-2's 53% was a warm
steady-state sample on the pre-art ROM. Different ROM + different measure.)*

### Trigger 1 — DMA-drain reopen: **DOES NOT FIRE.**
Reopen only if the worst-case VBlank audit *still binds*. It does not. Worst VBlank CPU usage is
**max-H at ~55% of the window** (~8.3k cy idle VBlank headroom); the plane-buffer drain itself
(`VInt_DrawLevel`) is **1375 cy** (1.1%). The main loop carries ≥35% idle in every steady regime.
VBlank is not the bottleneck in any regime, so there is no pressure to relocate producer work
into VBlank DMA — the (e) DMA-drain lever was always a *VBlank-bound* remedy, and that condition
is absent. **Verdict: skip; do not commission the measure-first sub-note.** (The lag that *does*
occur is the documented cold-crossing **producer** spike — `DecompressBlock` hits 16.3% in max-V
— which is a main-loop decompress-amortization question, NOT a VBlank/DMA-drain one.)

### Trigger 2 — FindStagedBlock direct-mapped: **DOES NOT FIRE — but the premise is stale, and a real candidate hides behind it.**
Two findings:

1. **The "pass-2 memoize" the trigger names was never built.** I grepped all of aeon history
   (`git log --all`): there is no memoize / scan-generation commit. `TileCache_FindStagedBlock`
   is at its **full pre-memoize cost** — the per-frame prefetch scans (`.pfx_scan` / `.cs_scan`)
   still re-probe every block col/row of the target line every frame (exactly review finding #3),
   which is why max-V shows **19 calls / 6641 cy (5.2%)**. Pass-2 shipped the finding-#1 copy
   restructures (1.1a/1.1b/1.3) + t16 unified-prefetch, and *deferred* finding #3; the roadmap §C
   text assumed it shipped.
2. **Direct-mapped is still the wrong call, independent of the premise.** It carries
   conflict-eviction thrash risk and a mandatory diagonal A/B, to shave ~2–6k cy when the main
   loop already idles 35–48%. Low value, real risk. **Verdict on direct-mapped: skip.**

   **BUT** FindStagedBlock at 5.2% max-V is hot enough to matter, and the *behavior-preserving*
   memoize (finding #3 option 1 — skip re-probing a fully-staged line, ~30 cy check, invalidate on
   a generation bump) reclaims most of that 6641 cy with **no thrash risk and no A/B burden**. It
   is the correct, safe Phase-2 candidate that the direct-mapped framing skipped over.
   → **Surfaced to the overseer as item 8b below** (net-new, not on the roadmap's item list).

**Both §C triggers resolve to "do not reopen the flagged high-risk option."** No STOP-fork. One
net-new safe candidate (the memoize) surfaced.

---

## 2 · Phase-2 batching plan (roadmap items 5–9)

Guiding shape: **byte-changing register surgery under the live ERROR net.** Every parcel =
checkpoint docs (census/design/packet/snapshot) + bisectable commits + full firing snapshot
across ALL surfaces (closure / §6 / D1b / D1c / out-verify / dead-saves / dropped) + the identity
bar + the **full 5-site repin ripple + PROVENANCE re-baseline** (per the golden/PROVENANCE.md
2026-07-21 Deep-Forest-BG entry: pins.rs, engine.inc org table, mixed_dac_rom, repin_pins, +
aeon main.asm sound-gate orgs; DEBUG=1 build → cp → plain, hash FINAL bytes only, both shapes).

Proposed parcel order (each its own merge; smallest-blast-radius first):

- **Parcel A — dead-save deletions (item 5).** The **15-row** worklist
  (`2026-07-17-dead-save-worklist-reissued…tsv` rows 1–15; **row 16 WarmupBelowRow d6 is the
  proven-FALSE dead-save item #3 removed — it stays**). Mechanical `movem`-save removals; each
  deletion its own bisectable commit + identity bar (a deleted save that was live corrupts
  silently — the net catches undeclared *inputs*, not a wrongly-deleted *preserve*, so the byte
  gate + per-row reasoning is the guard here). **Per-row guard (overseer-ruled): before each
  deletion, re-run the S2-D6 checked-clobbers analysis at CURRENT line numbers** (the row-16
  WarmupBelowRow lesson generalizes — a save can bracket a clobbering call, and byte-neutral
  merges have drifted lines) **+ identity bar per bisectable commit.** Pre-merge attack-the-diff
  granted. This is the cleanest first parcel.
- **Parcel B — D1c-tagged caller-side hoist fuel (item 6).** The 12 D1c clears from G4 Stage A
  gave callers VERIFIED register freedom across those calls; hoist loads out of loops / across
  the now-trusted calls. Pairs naturally with the 2 documented D1c FP sites (FillRow@FindStagedBlock,
  Load_Object@AllocDynamic — observe-only, do NOT "fix" by weakening; leave as-is).
- **Parcel C — step-5 pass-3-adjacent riders (item 7).** ledger-1092 `move.l` pairing · W022 /
  W025. Small, mechanical.
- **Parcel D — object/render Tier-B (item 8).** EV-ranked from the review §"Consolidated"
  (sprites H1/H2/H3, rings R2/R3, entity_window #1/#3/#4, core #1/#2, animate A2/A3, section
  H1/H3). **Each verified at its own mini design gate against the review doc before commit** —
  this is the highest-variance bucket; do not batch blind.
- **Parcel 8b (NET-NEW) — FindStagedBlock scan memoize** *(lands after B, BEFORE Tier-B D —
  mechanical vs D's variance).* The trigger-2 surfacing; behavior-preserving. Design obligations
  (overseer-ruled):
  - **(a) KEYED, not boolean.** Key = `(axis, target row/col value)`. A bare "row is warm" flag
    survives a scroll-direction flip or a target-row change within the quiet frames; keying on
    the target value makes those **self-invalidating** and shrinks the explicit-invalidation
    surface to staging-state changes only. The generation-bump-on-EVERY-claim
    (compressed/raw/empty) + `InvalidateStaging` deaths (`:447`/`:1729`) then carry the rest.
    Belt-and-braces boundary-move deaths (the write sites `416–445`/`795–936`/`1701–1727`) may
    stay if cheap, but **the key is the load-bearing guard.**
  - **(b) PER-AXIS memos.** `.pfx_scan` and `.cs_scan` each get their own memo; never shared
    state across axes.
  - Diagonal A/B retained (to confirm zero behavior change — there is no thrash). **Pre-merge
    attack-the-diff granted** (touches staging-state invalidation).
- **Parcel E — micro-batch riders (item 9).** aabb 3-piece split · vdp_init M1 · dplc D3 · the
  2nd typed-VDP-consumer shared module (ledger 1073). Trailing cleanup batch.

**Explicitly NOT in Phase 2:** D7 dead-code deletions + Tier-C movem deletions = Phase **2.5**
(their own byte-changing batch). G5 typed slots + prelude domain types = Phase **3**,
overseer-owned — I will not start them.

---

## 3 · Escalations — ALL RULED (overseer, 2026-07-21)

1. **§C outcomes** — both triggers skip **CONFIRMED**; **8b memoize SANCTIONED** as a net-new
   Phase-2 item (overseer independently verified the stale premise: no memoize commit in aeon
   history, and `e96448d` (t16 Wave 2 (i)) is the commit that created the `.pfx_scan` re-probe).
2. **Cold-crossing decompress spike** — **PARK RATIFIED.** It's the gap-ledger 1057/1065
   amortization thread, an engine-arch design question; nothing gates at ≥35% idle. Stays
   t18-adjacent; NOT pulled into Phase 2.
3. **Attack-the-diff** — **GRANTED** on Parcel A and 8b, and made a **standing Phase-2 rule: any
   parcel whose diff touches a save/preserve gets one pre-merge.**

## 4 · Current gate snapshot (baseline for every Phase-2 firing diff)
closure 0 / §6 0 / **D1b 0 (ERROR)** / D1c 2 (documented FPs) / dead-saves **15** / out-verify 16
/ dropped 0. strict **2454/0/1**. Canonical plain `3aa43cb6`/420749 · debug `ce0e83a6`/428768.
