# Pre-t18 Roadmap

**Milestone target:** t18 = the **parallax port** (next lag lever, ~18-22%/frame; parallax is still unported `.asm`).

> **✅ t18 MERGED 2026-07-23** — parallax.emp fresh port (11 procs, full 0→5 loop) + HBlank RAM-jmp trampoline (row 1088) + Hscroll_Dirty PAD + demanded `[lower.abs-sym-operand]` feature. Masters **aeon `6261c29` / sigil `8ab53f8`** (`port-tranche18` --no-ff). **NEW CANONICAL plain `00f609a5`/421089 · debug `80d14183`/429134** (byte-changing re-baseline). Full paired strict on merged masters **2488/0**. Oracle trampoline live-verify 5/5; H2 flat-fill unroll −1924 cyc/f live A/B. **DRY-PANEL DEBUT** (campaign first): caught B6 (gate-blind CC-clobber → post-merge parcel), refuted 1 false positive, 3 folds, ledger rows. Close packet `2026-07-23-t18-close-packet.md`. **Post-merge queue:** item-13 impl brief (7b9afa6) + the boundary-crossing transition parcel (window-slide close + B6/B3/B1/B2, B2-design-first).

> **✅ TRANSITION PARCEL MERGED 2026-07-24 (FIRST in the queue)** — five section-boundary parallax transition fixes: **B6** (promote-frame CC-clobber rebuild-skip, length-neutral reorder), **B2** (active-config mode contract — single `Parallax_Active_Config` accessor, routed HScroll DMA-length + Vscroll), **B3** (frames-remaining `divs.w` ramp, exact convergence, no pop), **B1** (re-cross cancel branch). All latent (no shipped transition fires; configs share mode) — mechanism-hardening, each proven with crossing-drive rig A/B. **Demanded + built `divs.w`/`divu.w` in the sigil ISA** (own commit before consumer). **Window-slide rider CLOSED** (real slide observed, single-axis assert held; per-section VALUE audit ledgered deferred). Kill-list **row 35 live** (OJZ harness mode-register force-write; kill=B2-(ii) ships). Masters **aeon `aa354fa` / sigil `4028bc8`** (`parallax-transition-parcel` --no-ff). **NEW CANONICAL plain `0bfa5b79`/421161 · debug `9d962703`/429204** (byte-changing; `EndOfRom` stable). Full paired strict on merged masters **2490/0**. Close packet `2026-07-23-transition-parcel-close-packet.md`. **Queue now:** item-13 (in flight, re-verifies its byte-neutral bar on this moved canonical).

> **✅ item-13 WAVE-1 MERGED 2026-07-24 (SECOND in the queue) — BYTE-NEUTRAL confirm variant.** The three frozen domain-type families (construct-walk #4 `7b9afa6`): **SongId/SfxId** (sound-API value slots + `SONG_*`/`SFXID_*` const defs + as-blessed construction sites), **AnimId/AnimFrame/MappingFrame** (`FrameId` renamed→`MappingFrame`; `anim_frame` u8→`AnimFrame`), **VramTile/VramAddr** (`vram_art` takes `VramTile`, $1FFF bound preserved; tile-index consts born `VramTile`). Masters **aeon `9fb6fcb` / sigil `4f4f0e2`** (`item13-wave1` --no-ff). **Canonical plain `0bfa5b79`/421161 · debug `9d962703`/429204 UNCHANGED** (confirm variant, NO re-baseline — dual rebuild from merged master exact). Full paired strict on merged masters **2499/0**; zero new clippy. **ENFORCEMENT-TIER FINDING (packet-worthy):** the G5 slice bites only at register call-slots, so only **F1 has live enforcement** (Sound_PlayRing→Sound_PlaySFX corpus swap pin fires); **F2** (SST-memory-resident) and **F3** (comptime-fn loose params) ship DOCUMENTARY + synthetic swap-pins, with the reopen markers (F2 field-store domain-check · F3 comptime-arg newtype-check) making the types retroactively enforceable. `vram_bytes` ledgered-not-created (stage-0: absent in aeon); Option-A optional-param LOG-AND-SPLIT (size-cap). Close packet `2026-07-24-item13-wave1-close-packet.md`. **Queue now EMPTY** — campaign returns to single-lane lean conversion tranches; sprites-hardening stays PARKED (post-conversion).

> **✅ TRANCHE 19 MERGED 2026-07-24 — camera/bg/bg_anim conversion (first LEAN tranche).** `engine/level/` trio → `.emp` (camera 2 procs · bg 1 · bg_anim 2), full 0→6 loop. **Demanded/derived:** the comptime-select game-contract mirror idiom (`-D GAME_CAMERA_JUMP_LOCK`; =0 arm short-circuits before `extern()` — lock-less games link with NO game symbols, proven by the `jump_lock_off_compiles_without_game_symbols` probe); **NEW `engine/z80_bus.emp`** (2nd-consumer lift, sound_api retrofitted); **VDP_DATA/VDP_CTRL hoisted into `engine.vdp`** (3rd consumer; t18 B1 debt executed — parallax/plane_buffer/section retrofitted); `bganim_band` struct (all walk magics derived, sizeof==44 ensure, emitter LOCKSTEP kill row); `clamp_camera_axis` template; 2 DEBUG asserts (band count + piece-1 length — plain zero bytes; the twin carries the campaign's first `ifdef __DEBUG__` shape-dependent branch widths). **Step-5 measured, no cut** (Camera_Update 670/716 cyc/f · BgAnim_Update 124 exit-only; live P3 bonus: both chases parked at the EXACT documented deadzone boundaries). Dry-panel round: DRY STOOD (marquee: partial-retry comment overpromise → truthful comments + ledgered fix sketch; piece-1 assert; GRID_*>=1 ensure). Masters **aeon `3938250` / sigil `f2c4361`** (`port-tranche19` --no-ff). **NEW CANONICAL plain `eab19b3f`/421159 · debug `f1c1aa12`/429204** (`EndOfRom` stable both shapes). Full paired strict on merged masters **2509/0**. Close packet `2026-07-24-t19-close-packet.md`. Remaining engine 68k conversion backlog: ~11 files (bg cluster now DONE; next big rocks: vblank/dma_queue/load_art/boot/buffers).

**Current state (2026-07-21):** **PHASE 1 COMPLETE.** All four Phase-1 items merged —
contract-grammar v2 arc is G1+G2+G3+substrate+G4+G4.5(#1–#4). Masters: sigil `871ec7d` /
aeon `ae1de4d` (aeon untouched since #2; #3 and #4 were both pure-sigil, byte-neutral).
Item **#4 (the D1b WARN→ERROR flip) MERGED `871ec7d`** — the verified-out fixpoint is live
and the net is an ERROR gate before pass-3 opens. **Phase 2 (pass-3) is now OPEN.**
Canonical ROMs **RE-BASELINED 2026-07-21** (Deep-Forest-BG art parcel, byte-CHANGING):
plain **`3aa43cb6`/`420749`**, debug **`ce0e83a6`/`428768`** (supersede `8984e510`/`453533` ·
`c80465dc`/`461554`; see `golden/PROVENANCE.md`).
Strict mode **ON for D1b** — `[call.input-undefined]` is now a live ERROR gate under
`SIGIL_STRICT_GATE`, resting on the VERIFIED-out fixpoint (an out() is credited as a
definition only once proven honest). D1c stays observe-only; the out-verify residue (16)
stays WARN.

**Guiding principle:** *Contracts go on stable code.* Finish **and verify** the safety net → optimize under it → delete dead code → type the settled register layout → then port parallax.

> **Ordering note that supersedes the original v2 design doc.** The doc says "pass-3 unblocks after G1+G2." That predates the G4 session. G4 made `out()` labels **load-bearing but unverified**, and pass-3's "trust the contract" hoists now depend on those labels being honest (FindStagedBlock was a real mislabel — proof the trust is currently misplaced). So **Phase 1 (verify the contract system) must land before Phase 2 (pass-3).**

This roadmap covers the three output buckets of the 4-wave optimization review — **optimization**, **bugs**, and **language failings** — each is tagged `[opt]` / `[bug]` / `[lang]` below.

---

## Phase 1 — Complete & verify the register-contract system  *(before any optimization)* — ✅ **COMPLETE (all 4 items merged; D1b is a live ERROR gate)**

1. ~~**out()-verification arc** ("G4.5") `[lang]`~~ — **DONE, merged 2026-07-19** (sigil
   `cd10321`). Caught 2 real allocator mislabels. Residue: Buckets 2/3 (→G5), mutual-callee-out
   fixpoint, conditional-external-tail (grep-proven-absent) — all on the #4 blocker list.

2. ~~**Edge-sensitive conditional-out crediting** `[lang]`~~ — **DONE, merged 2026-07-21**
   (sigil `0d4b529` / aeon `3eba8fb`). Bucket 1 CLEARED (AllocDynamic/AllocEffect honest
   `out(a1 if eq)` + Load_Object cascade), FillColumn D1b FP CLEARED. Includes the Finding-7
   label-join bail (review-proven `valid_edge` hole, fixed pre-merge). D1c FPs now 2,
   documented, deliberately not edge-coupled.

3. **S2-D6 checked-clobbers / preserves lint** `[lang]` ✅ **DONE — merged sigil `3f333d2`
   (2026-07-21, byte-neutral; aeon `ae1de4d` untouched, pure-sigil).** Stage-0 re-census
   confirmed the brief's hypothesis: the transitive `[proc.clobber-undeclared]` residue was
   already 0 (error-gated since G3), the individual-push FP trio retired. Re-scoped to the two
   real defects Stage 0 surfaced — **A** write-detector completeness (dbcc counter + non-stack
   movem-LOAD reglist; the `(sp)+` restore exempt as preserve-discipline) and **B** the local
   `check_clobbers` FP-kill (subtract §5-verified preserves; cleared 25 firings on honestly-
   `preserves()`-declared registers). **Headline catch: A removed a FALSE dead-save
   (`WarmupBelowRow` d6 bracketing `DecompressBlock`) — the dead-save worklist is now 15 rows,
   not 16; d6 is clobbered ONLY by the `movem.l (a0)+, d0-d6/a2` burst the detector was missing,
   so pass-3 would have wrongly deleted a necessary save (see packet
   `2026-07-21-s2d6-packet.md`).** Two attack-diff findings fixed (written_names conditional-dbcc
   over-credit; tripwire gate-compliance). Gaps adjudicated: dbcc → closed (A); movem-pair →
   already closed (substrate §5); s4lint W021 → moot (not a live tool); per-callee union (d) →
   deferred to its Phase-2.5 Tier-C consumer (gap-ledger). D1a transitivity stayed out of scope.
   Snapshot: closure 0 / D1b 0 / D1c 2 / dead-saves 15 / out-verify 15; strict 2445/0/1.

4. **WARN→ERROR flip** (D1b strict) `[lang]` ✅ **DONE — merged sigil `871ec7d` (2026-07-21,
   byte-neutral; aeon `ae1de4d` untouched, pure-sigil).** `[call.input-undefined]` is now a
   HARD ERROR gate, and the credits it rests on are VERIFIED, not declared. Built from the
   overseer brief. **Stage-0a settled it:** predicted new-D1b-firing set = EMPTY at the
   aggressive 17-out bound (every unverified out is in-out — defined upstream, must-def never
   kills — or a pure-out read-consumed / `.asm`-only), so the flip needed **NO G5 pull-forward**;
   Buckets 2/3 stay WARN residue. **Mechanism:** verified-out fixpoint (`compute_verified_outs`
   — extern outs seed verified as §3 axioms; monotone; round-cap panic) grounds the credit; the
   DEFINITION surfaces (D1b must-def + out-verify residue) switch to verified credit, the
   REDEFINE-EXCUSE surfaces (§6 taint-kill, D1c held-value) keep declared — the **define-vs-
   redefine dividing line**, empirically proven (switching D1c would add 11 narrow-width FPs).
   Also awakened four AEON_DIR-gated corpus tests (incl. three shipping ERROR gates) that
   silently skipped under the standard strict invocation. Residue moved 15→16 (fixpoint chain-
   grounding surfaces `Collision_GetType::out(d0)`). Mutual/circular callee-out CLOSED (fixpoint,
   no corpus instance); conditional-external-tail re-confirmed grep-absent, guard stands. §6
   existence-lie exposure deferred to the per-lie-class-credit gap-ledger row, tripwire-guarded.
   Snapshot: closure 0 / D1b 0 (**ERROR**) / D1c 2 / dead-saves 15 / out-verify 16; strict
   2454/0/1. Packet + dividing-line table: `2026-07-19-out-verification-residue.md` (2026-07-21
   addendum).
   *Why before pass-3:* a WARN net doesn't *catch* bad hoists during surgery — an ERROR net does. Net live before you cut. ✅ **The net is live before the cut.**

---

## Phase 2 — Pass-3: register / object-render optimization  *(under the live net)* — 🟢 **OPEN (the net is live; Phase 1 complete)**

*Design-gate first: run the §C trigger checks, then batch the work.*

5. **Dead-save worklist** `[opt]` ✅ **DONE — Parcel A, merged aeon `39faa02` / sigil `015f76e` (2026-07-22, BYTE-CHANGING).** All 15 rows removed (9 length-preserving movem narrows + 2 full movem-pair removals, −16 bytes); attack-the-diff PASSED. NEW canonical plain `748ca5ba`/420749 · debug `d5d8e163`/428768 (EndOfRom unchanged). Merge-phase catch: a stale `s4.debug.lst` mis-repinned debug → shipped a `repin` listing-freshness hard-check as a hardening rider. (was 16: item #3's detector fix exposed the `WarmupBelowRow` d6 row as a FALSE dead-save — deleting it would have corrupted d6; see the s2d6 packet.)
6. **D1c contract-precision (byte-neutral)** `[opt]` ✅ **= Parcel B (branch `pass3-parcelB-hoists`).** Reframed from "caller-side-hoist fuel": the D1c-clear fuel contained **no code hoists** — the sites are TIGHT BY CONSTRUCTION (D1c fires only when a caller reads a reg after a call *without* reloading, so there is nothing to remove), and the freed registers are not hoistable loop-invariants. So B is **4 byte-neutral clobber tightenings** (`Load_Object` d2 · `RescanObjects`/`ScanObjectsRight` d5 · `Scan` a5), found via a `declared ∖ effective` sweep (banked as the `[proc.clobber-unexercised]` lint's regression seed). *(Second time the net converted anticipated byte-surgery into contract precision — cf. S2-D6 #3's stage-0.)* LICM hoist-hunt rejected (no frame-lag pressure; Parcel D carries the EV-ranked review candidates).
7. **Step-5 pass-3-adjacent riders** `[opt]` ⊘ **DISSOLVED (Parcel C stage-0, 2026-07-22).**
   **W022/W025 = 0 firings** (whole `.asm` corpus is s4lint-clean — pass-2 restructures + the port
   campaign dissolved their sites; census cross-ref confirms W022 was reclassified a future perf-lint).
   The **`move.l`-pairing** rider survives (tile_cache FillRow/collision + plane_buffer drain still
   `move.w`/`move.b`) but is non-lag-critical (≥35% idle); **folded into the 8b parcel** as its own
   bisectable commit(s) sharing 8b's ripple + PROVENANCE (logged slot, not dropped, per no-silent-skips).
   *Third net-conversion of anticipated byte-surgery into already-done/evaporated (after S2-D6 #3 stage-0
   + Parcel B hoists).*
   *(Note: "D7 deletions" is NOT here — it is its own byte-changing batch, Phase 2.5.)*
8. **Object/render candidates** `[opt]` — Tier-B — ⊘ **CLOSED-EARLY (2026-07-22): ZERO CUTS AFTER 8b.**
   sprites H1/H2/H3 · rings R2/R3 · entity_window #1/#3/#4 · core #1/#2 · animate A2/A3 · section H1/H3.
   > **HEADLINE:** the entire Tier-B census hotness was an **inclusive-artifact, queue-wide** — every
   > "hot" number (RunObjects 34.8%, section 6.3%, ew 2.9%, …) was inclusive of dispatched/fill/walker
   > work the proposed transforms don't touch; measured **plain-shape addressable SELF-time is sub-2%
   > for all 13 items**. And there is **no lag to relieve**: the pass-2 / unified-prefetch / free-lunch
   > arcs already ate it (VSync idle **48.8% max-H, 54.3% churn; Lag=0 all regimes**). Closed via 3 mini
   > design gates (core #1 #4, sprites #5) + one Bar-A census sweep (the remaining 10 rows); all surfaces
   > banked to the gap-ledger with reopen conditions. Notes: `2026-07-22-{core1-runobjects,sprites-h1h2h3,
   > barA-census-sweep}-design.md`.
   - **8b DONE** (FindStagedBlock memoize + 2 move.l riders + F2 census; aeon `5c975af`/sigil `4993b0b`).
   - **core #1 (RunObjects) ⊘ DISSOLVED-STAGE-0 (2026-07-22, dissolution #4 — hotness-measurement
     artifact).** Census "34.8%" was DEBUG *inclusive*; addressable machinery self ≈**5.75%** in a
     **54%-idle** plain frame. No parcel cut (byte-changing AND byte-neutral both declined). Two surfaces
     banked to the gap-ledger (`.culled_loop` declared∖effective sweep · branchless-abs cull math w/ `$8000`
     pin). **Reopen = a real-scene lag report** (54%-idle is churn-stressor-relative). Note:
     `2026-07-22-core1-runobjects-design.md`.
   - **sprites H1/H2/H3 ⊘ DISSOLVED-STAGE-0 (2026-07-22, dissolution #5 — value-vs-ceremony).** Ranking
     CONFIRMED (Render_Sprites self **8.9%** > RunObjects 5.75%) but no cut: scene-bias is **per-path** —
     churn over-counts H1's per-object lever (40/40 ≥ gameplay), so H1's **~0.5-1% is a CEILING**; no lag
     (54.3% idle), no paid ripple to ride → core-#1 branchless-abs shape. H2 evaporated (already
     comptime-unrolled); H3 non-binding VBlank DMA (PB1 satisfied). Three surfaces banked to gap-ledger.
     Reopen = real-scene lag OR elected headroom pass; H1's reopen first-gate = corpus-wide
     `mapping_frame`-drift writer sweep. Note: `2026-07-22-sprites-h1h2h3-design.md`.
   - **BAR-A CENSUS SWEEP — DONE, RULED-ACCEPTED (2026-07-22).** All 10 remaining rows measured
     plain-shape self-time and DISSOLVED (section H1 0.85% · ew #1 0.66% · core #2 1.9% in its own
     delete-storm vehicle · animate A2/A3 ~60c/~24c structural · rings R2/R3 0.7%/0.8% @ 13 rings). Rings
     harness-vs-close ruled **CLOSE both** (no realistic ring-saturation scene). Reopen conditions banked
     per row (core #2 = >4 deletes/f; rings = ring-heavy content + X=0-mask rider; rest = real-scene lag /
     elected headroom). Table: `2026-07-22-barA-census-sweep.md`.
   - **MILESTONE CHECKPOINT → Volence** (next-arc options): **t18 parallax port (overseer-recommended
     next)** · **VDP shared-module row 1073** (optional warm-up) · **H-streaming successor charter (rows
     1066/1074) — PARKED, and REQUIRES ITS OWN BAR-A STAGE-0 before any charter** (census numbers are
     inadmissible there too — same inclusive-artifact risk). The next arc opens on Volence's pick.

   > **STANDING BAR A (ratified at the core #1 gate, 2026-07-22).** Every remaining D-parcel stage-0 MUST
   > present **fresh PLAIN-SHAPE addressable SELF-time** (inclusive − children, decomposition shown).
   > **Census inclusive/DEBUG numbers are INADMISSIBLE as hotness evidence** — the whole D census is
   > suspect the way core #1 was. Applies to survivors AND parks.
   >
   > **STANDING BAR B (ratified 2026-07-22).** When the idle-margin frame-anchored A/B method is used, the
   > run MUST **record the lag-frame counter on both sides** — "no lag frame" is measured evidence, not an
   > assumption.
9. **Micro-batch riders** `[opt]` — aabb 3-piece split · vdp_init M1 · dplc D3 · VDP shared-module (row 1073, 2nd typed-VDP consumer).

---

## Phase 2.5 — D7 dead-code deletion batch  *(its own gated, byte-CHANGING batch)*

> **✅ CLOSED-MERGED 2026-07-22** — combined item-9 + Phase-2.5 arc (c1–c6 + MigrateMasks fix)
> merged: **aeon master `033865f` / sigil master `9b89d67`** (`pass3-phase25-item9` --no-ff).
> NEW CANONICAL: plain **`406c773b`**/421122 · debug **`5752c2e3`**/429107 (ASSEMBLED_LEN unchanged).
> Full paired strict on merged masters **2457/0/1**; overseer attack-the-diff PASS + re-attack PASS.
> **Cut:** Spawn_Count · CROSS_RESET (whole dead mechanism) · ess_*_left_idx (mid-struct) + the
> byte-neutral D7 const/flag purge. **PARKED (gate rulings, gap-ledger reopen conditions):**
> Hscroll_Dirty_* → t18 (parallax retires it free) · Tier-C movem (item 11, children.asm port
> adjudicates) · Tile_Cache_GetTile (next tile_cache byte-changing edit) · aabb#1 · dplc D3.
> **Caught:** a silent, byte-gate+boot-invisible MigrateMasks hand-stride bug (ledger blind-spot row).

10. **D7 dead-code / dead-symbol deletions** `[opt]`  *(ledger row 1091)*
    Deletes live RAM symbols / dead writers: `Spawn_Count`, `CROSS_RESET_MAGIC`, dead `ess_*` indices, `Hscroll_Dirty_*`, `Tile_Cache_GetTile` (zero callers), the wave-4 dead-constants list, dead `DEBUG_*` flags.
    **Byte-changing** (RAM layout shifts / removed stores) → moves the byte gates → needs both-shape lockstep + re-pin + provenance. Cannot ride a byte-neutral pass. Ratified scaffolding (e.g. `Plane_Buffer_Reset`'s reset hook) is **annotated, not deleted**.
11. **Tier-C movem deletions** `[opt]` — dplc / load_object / AllocDynamic; unblocked once the Phase-1 S2-D6 lint exists (per-caller union proven mechanically).

---

## Phase 3 — G5: type the now-stable register layout

12. **G5 — typed register slots (§7)** `[lang]`  *(ledger rows 1054, 1069)* — ✅ **DONE 2026-07-23** (merged; byte-neutral, canonical 406c773b/5752c2e3 UNCHANGED = the Phase-3 landing proof). Axis-split `GridX`/`GridY` + `SectionId` at the FlatIDXY/GetSecPtrXY seam; `out(dN: Type)` grammar + `as`-bless + the `[call.slot-type-mismatch]` strict-degrade reaching-def slice; swap class closed at every seam site. `FlatIDXY.d2` banked u8 (preserves-verifier limit — new ledger row). Spec+addendum: `2026-07-23-g5-typed-register-slots-spec.md`.
    *Why last:* G5 types the exact register slots pass-3 reshapes — type the **final** layout, not a moving target. **Byte-neutral**, so it lands on pass-3's settled canonical and proves it changed nothing. Demand step: run the `// In:`/`// Out:` proc-header census (row 1069) once signatures are stable.
13. **Prelude domain-type pass** `[lang]`  *(sibling / follow-on; "construct walk #3/#4")* — **WAVE 1 ✅ DONE, merged 2026-07-24** (aeon `9fb6fcb` / sigil `4f4f0e2`; byte-neutral, canonical `0bfa5b79`/`9d962703` UNCHANGED). Shipped `SongId`/`SfxId` · `AnimId`/`AnimFrame`/`MappingFrame` (`FrameId`→`MappingFrame`) · `VramTile`/`VramAddr` (comptime-first). Enforcement-tier finding: F1 live (register call-slot), F2/F3 documentary (SST-memory / comptime-fn substrate) with reopen markers ledgered. **Wave 2 A4-i-GATED** (Tile/Block/Chunk, Coord/Velocity — shift/add chains wait for arithmetic-preservation). Ratification `7b9afa6`; close packet `2026-07-24-item13-wave1-close-packet.md`.
    Populates G5's typed-register mechanism with the wider newtype family: `Angle` (partly shipped for GetSineCosine), `SoundId`/`SongId`/`SfxId`, `VramTile`, `AnimId`/`FrameId`, `Tile`/`Block`/`Chunk`. Volence-driven design.

---

## C · Trigger-keyed reopens  *(run these checks at the Phase-2 design gate; skip if the trigger doesn't bind)*

> **CHECKED 2026-07-21, overseer-CONFIRMED 2026-07-21 (oracle profiler, 3 regimes; outcomes + method in `2026-07-21-phase2-design-gate.md`).** Both triggers resolve to **skip**; the finding-#3 memoize is sanctioned as net-new item **8b** (keyed + per-axis; before Tier-B).

- **DMA-drain reopen** `[opt]` — only if the post-pass-2 worst-case VBlank audit still binds.
  → **DOES NOT FIRE.** Worst VBlank CPU = ~55% of the ~18.5k window (max-H); drain itself 1.1%;
  main loop ≥35% idle every regime. VBlank never binds. (The lag that occurs is the cold-crossing
  *producer* decompress spike, not VBlank.)
- **FindStagedBlock direct-mapped** `[opt]` — only if it's still hot after the pass-2 memoize.
  → **DOES NOT FIRE (direct-mapped).** Stale premise: the memoize **was never built** (no commit
  in aeon history). FindStagedBlock is at full pre-memoize cost (5.2% / 19 calls, max-V). Direct-
  mapped stays skipped (thrash risk + A/B, low value vs idle). **Net-new candidate surfaced: the
  behavior-preserving finding-#3 memoize** (safe, no thrash) → design-gate note item 8b.

---

## D · Backlog  *(any time; not gating t18)* — ✅ **ARC MERGED 2026-07-23 (attack-the-diff PASS; masters aeon `c39f308` / sigil `0c27746`; NEW CANONICAL plain `ab787bd1`/421122 · debug `6a19669f`/429165; paired strict 2484/0/1; close packet `2026-07-23-sectionD-backlog-arc-close-packet.md`)**

- ✅ **`Sound_PlayMusic.await_slot` DEBUG watchdog** `[bug]`  *(ledger row 1090)* — **DONE + reframed.** Stage-0 surfaced the real bug behind it: the H-1 repost gate never iterated (`startZ80`'s `move #$0000` clobbered the `tst` flag before the `bne` — the constant-flag-clobber class), and `[branch.condition-constant]` (item-4 rider) found a SECOND identical bug in `Sound_Init.wait_alive`. Fixed as capture-then-test (aeon **c1** `c0db661`, both twins) + the DEBUG bounded-spin watchdog on both spins (aeon **c2** `4b5a2c0`, `SPIN_WATCHDOG_LIMIT=$8000`). The now-working spin was live-single-step-proven (the `d4` watchdog counted `$8000→$7C64` before the slot cleared — the loop iterates). Plain + debug re-baselined (sigil `32bc836`).
- ✅ **Optional-param design** `[lang]` — **DESIGN-ONLY, ruled.** `AnimateSprite d3`: Option A (`?`-optional marker) ships *when animate is next touched*; Option D (type the animation-script duration byte — the enforcing successor) gated behind the item-13 domain-type family. No implementation this arc. (Stage-0 gate item 5.)
- ✅ **@scaffolding attribute** `[lang]` — **EVAPORATED** (shipped in G1 + already applied to the one keep, `Plane_Buffer_Reset`).
- ✅ **D11 name-linkage** `[lang]` — **RETIRED.** Its guard (`ensure(extern("X")==X)`) shipped + is a mandatory port-loop rule; its auto-detector's target population (seam mirrors) is a draining kill-list artifact. Thin intra-`.emp` residual logged as a low-priority link-warning idea, not a diagnostics-tier item.
- ✅ **s4lint absorption #1 — Z80-bus machine-state contract** `[lang]` — **BUILT (item-4 core, sigil `93a309f`).** `[bus.double-stop]`/`[bus.start-without-stop]`/`[bus.stopped-at-return]`/`[bus.vdp-write-unstopped]` (E011/E008/E007/E006) over the shared `flag_check::Cfg`; 3-point MUST lattice, zero-FP polarity. First-corpus-run 0/0 both shapes (the fixed corpus is clean; teeth sentinel-proven). E008 included per the gate; E006 largely inert (indirect `(a4)` VDP writes punted). **Remaining absorb candidates** (`2026-07-22-s4lint-absorption-census.md`): #2 W026 width-discipline dataflow (pairs with G5 width typing), #3 E010 SR coverage via §5, #4 debug-seam checks, then perf/peephole WARN tiers. End-state unchanged: the last `.asm` port retires s4lint — one tool, one truth.

---

## Bucket coverage check (the review's three outputs)

| Bucket | Status |
|---|---|
| **Optimization** `[opt]` | Tier-A done (pass-1/2); Tier-B = Phase 2; Tier-C = Phase 2.5 (gated on S2-D6); D7 batch = Phase 2.5. ✅ |
| **Bugs** `[bug]` | All confirmed bugs (sprites PB1/PB2 + wave-2 B1/H-1/C1/D1/E1) fixed+merged; one open rider = await_slot watchdog (§D). ✅ |
| **Language failings** `[lang]` | out()-verification + edge-sensitive + S2-D6 lint (Phase 1); G5 + domain-type pass (Phase 3); diagnostics remainder (§D). ✅ |

---

## Not a hard dependency

Parallax (t18) is **unported**, so it depends on *none* of Phase 2/2.5/3. If beating frame-lag is the priority, t18 can jump ahead of pass-3 — a deliberate call, not a forced sequence.

## Two spots to confirm before committing  *(summarized from dense notes)*

- **Item 8** — the Tier-B pass-3 candidate list: verify against the review doc.
- **§D diagnostics remainder** — verify exact scope against the v2 design doc.

## References

- Contract-grammar v2 design doc: `sigil/docs/superpowers/specs/2026-07-17-contract-grammar-v2-design.md` (the G1–G5 table is §"tiers"; G5 = §7).
- 4-wave optimization review: `aeon/docs/reviews/2026-07-16-emp-port-optimization-review.md`.
- Gap-ledger (row detail): `sigil/docs/superpowers/notes/campaign-gap-ledger.md` — rows 1023/1030/1062 (S2-D6), 1054/1069 (G5), 1090 (watchdog), 1091 (D7), 1092 (move.l pairing).
- Campaign log: `spec2-progress.md` (memory; newest at file tail).
