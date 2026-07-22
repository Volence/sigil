# Pre-t18 Roadmap

**Milestone target:** t18 = the **parallax port** (next lag lever, ~18-22%/frame; parallax is still unported `.asm`).

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
7. **Step-5 pass-3-adjacent riders** `[opt]` — ledger-1092 `move.l` pairing · W022/W025.
   *(Note: "D7 deletions" is NOT here — it is its own byte-changing batch, Phase 2.5.)*
8. **Object/render candidates** `[opt]` — Tier-B; **verify each at the pass design gate before committing.**
   sprites H1/H2/H3 · rings R2/R3 · entity_window #1/#3/#4 · core #1/#2 · animate A2/A3 · section H1/H3.
   *(Confirm against the review doc — see References.)*
9. **Micro-batch riders** `[opt]` — aabb 3-piece split · vdp_init M1 · dplc D3 · VDP shared-module (row 1073, 2nd typed-VDP consumer).

---

## Phase 2.5 — D7 dead-code deletion batch  *(its own gated, byte-CHANGING batch)*

10. **D7 dead-code / dead-symbol deletions** `[opt]`  *(ledger row 1091)*
    Deletes live RAM symbols / dead writers: `Spawn_Count`, `CROSS_RESET_MAGIC`, dead `ess_*` indices, `Hscroll_Dirty_*`, `Tile_Cache_GetTile` (zero callers), the wave-4 dead-constants list, dead `DEBUG_*` flags.
    **Byte-changing** (RAM layout shifts / removed stores) → moves the byte gates → needs both-shape lockstep + re-pin + provenance. Cannot ride a byte-neutral pass. Ratified scaffolding (e.g. `Plane_Buffer_Reset`'s reset hook) is **annotated, not deleted**.
11. **Tier-C movem deletions** `[opt]` — dplc / load_object / AllocDynamic; unblocked once the Phase-1 S2-D6 lint exists (per-caller union proven mechanically).

---

## Phase 3 — G5: type the now-stable register layout

12. **G5 — typed register slots (§7)** `[lang]`  *(ledger rows 1054, 1069)*
    `GridCoord`/`SectionId` at the FlatIDXY seam: `proc FlatIDXY(d2: GridCoord, d3: GridCoord) out(d0: SectionId)`, checks at the ~4 cross-`jbsr` sites; closes row 1054.
    *Why last:* G5 types the exact register slots pass-3 reshapes — type the **final** layout, not a moving target. **Byte-neutral**, so it lands on pass-3's settled canonical and proves it changed nothing. Demand step: run the `// In:`/`// Out:` proc-header census (row 1069) once signatures are stable.
13. **Prelude domain-type pass** `[lang]`  *(sibling / follow-on; "construct walk #3")*
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

## D · Backlog  *(any time; not gating t18)*

- **`Sound_PlayMusic.await_slot` DEBUG watchdog** `[bug]`  *(ledger row 1090)* — the H-1 residual: the spin is correct but *unbounded*; a wedged Z80 hangs the 68k silently in plain. Add a DEBUG-only spin counter + `raise_error` on overrun (self-gates to zero bytes in plain). Do-when-you-next-touch-sound_api.
- **Optional-param design** `[lang]` — the AnimateSprite `d3` case (conditional/data-dependent input; no cc to hang it on). 1 site, no rush.
- **Diagnostics-tier remainder** `[lang]` — items beyond G1–G4 (D7 `@scaffolding` annotation, D11 name-linkage, any D5/D6/D8–D10 not folded into G1–G5). Confirm exact scope against the v2 design doc. **Now includes the s4lint absorption list** (`2026-07-22-s4lint-absorption-census.md`): Tier-B ranked absorb candidates — #1 Z80-bus machine-state contract (E006/E007/E011, a crash class, builds on the shared Cfg), #2 W026 width-discipline dataflow (pairs with G5 width typing), #3 E010 SR coverage via §5, #4 debug-seam checks, then perf/peephole WARN tiers. End-state: the last .asm port retires s4lint entirely — one tool, one truth.

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
