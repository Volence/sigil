# Tranche 12 packet — entity_window.asm → .emp (steps 0–5, at the merge gate)

Date: 2026-07-12. The largest port yet (1532 lines, 30 procs) and the ratifying
demand for the diagnostics construct (11 `assert` sites). Design note:
`2026-07-12-tranche12-entity-window-design.md`.

## Status: steps 0–5 done, loop dry, AT THE MERGE GATE

- **Byte gate GREEN both shapes** — `entity_window_port.rs`: plain
  `s4.bin[$3388..$3C5A]`, debug `s4.debug.bin[$3838..$4570]`.
- Full workspace strict **0-fail**, clippy clean.
- Region: plain **$8D2** / debug **$D38** (post step-2 shrink −0x1C / −0xC).
- NOT yet gated into the aeon build (`SIGIL_EMP_ENTITY_WINDOW` in repin.toml but
  not engine.inc) — wire at integration. Provenance hashes update at merge
  (master ROMs are now modernized).

## Step 1 — transcribe (byte-exact both shapes)

Faithful 1-1 port: 11 asserts via the merged construct (bare `assert`
self-gates; `ifdef __DEBUG__` blocks → `if DEBUG == 1 {}`); two compile-time
`if/error` static asserts → `ensure()`; ~28 constants + 17 `EntityScanState`/
`Sec`/`Act` field offsets mirrored locally + `ensure(extern())` drift-locked;
`Sst` via the existing sst.emp twin; the two `clearLoaded*` macros inline-expanded.
**Bug the gate caught (drift-lock structurally can't):** `COLLECTED_WINDOW_SLOTS`
mirrored from the DEMO config (=4); real sonic4 =9 — both `.emp` mirror and
test-equ agreed (=4) so the guard passed; only the ROM comparison exposed it.
Lesson: mirror game-owned consts from the game being built.

## Step 2 — modernize (byte-changing; hand-set twin lockstep)

`.emp` fully modern (jbsr/jbra/bare-Bcc/brace-indent). `EntityWindow_Init`'s
redundant tail branch-to-next-proc **deleted via `falls_into EntityWindow_Scan`**
(−4 both shapes). **THE HEADLINE FINDING:** ASL's branch relaxation ≠ sigil's —
the "bare the twin, let ASL auto-relax" shortcut is DEAD (ASL nop'd a bra-to-next
and kept `bsr.w −32` where sigil picks `.s`). The twin carries **hand-set explicit
widths** (why rings.asm always did). Method: diff sigil's fixed `.emp` bytes vs
the fresh master region (from the `.lst`, not the stale pin) with a shift-aware
matcher → 4 UNIFORM `bsr.w`→`bsr.s` + 8 PER-SHAPE `ifdef __DEBUG__ .w / else .s`
(3 FindSlot calls the debug asserts widen + the 5 DEBUG-width Bcc); `197`'s
`bsr Collected_UnparkSlot` stayed `.w` (spans ParkSlot — as predicted).
**Deltas: plain −0x1C, debug −0xC.** Downstream fallout, all in one re-pin wave:
collision_lookup/sound_api slid; **engine.inc resume orgs** updated; mixed_dac_rom
game_loop `bsr.w Sound_DrainSfxRing` disp **pin-spliced**; repin_pins baseline
updated. Process note: `./build.sh` = plain only; debug ROM needs the manual
`DEBUG=1 … cp … ./build.sh` dance.

## Step 4 — construct pass

- **BUILT: `clear_slot_bitmasks()`** comptime-fn (the `clr.b 1(a0)` pad + 8 mask
  `clr.l`, 3 sites; byte-neutral, AS twin inline in lockstep like `clear_longs`).
- **DEFERRED (reasoned, ledgered):** EntityScanState struct-twin (needs
  `offsetof()` — absent — for the 7 absolute-address EAs, plus a home: no
  file-local struct precedent, single consumer); `clearLoaded*` helper (1 site
  each, two divergent shapes → zero dedup); section-match unroll (needs a
  proc-local label as a param → the cross-fragment-label gap).

## Per-pass findings (step-3 retro vs step-5 engine vs neither)

### Step-3(a) — language / format asks
- **`sym_off_operand` generalization** (ledgered) — `compound + const-name`
  (`Base + len*N + field`) misfires; parenthesizing the offset is the byte-neutral
  workaround; the ask is peel-the-single-link-symbol-leaf.
- **ASL-vs-sigil relaxation** (ledgered, RESOLVED as method) — hand-set twin
  widths are THE lockstep; closes the auto-relax shortcut permanently.
- Ceremony: the ~28 const mirrors + 40 drift-locks are the imm-link-gap tax
  (unavoidable today; the struct-twin would absorb the 17 field-offset ones once
  `offsetof` lands).

### Step-3(b) — reads-wrong / audits
- **Comment-claim audit FIXED**: the 6 byte-lock comments still said ".w —
  exceeds short range" after the branches went bare/`falls_into`; rewritten to
  describe the per-shape relaxation (byte-neutral).
- **Contract audit (clobbers()-is-exhaustive-license ruling) — CLEAN, both
  directions, all 30 procs.** Direction 1 (body ⊆ license): every proc writes
  only registers in its `clobbers()`/`out()`; the `(aN)+` advances (ParkSlot/
  UnparkSlot copy loops, InitSection mask clear) are on declared-clobbered or
  save/restore-preserved registers — no under-declaration (contracts transcribed
  from the AS twin's tested `Clobbers:` headers; lower-clean, no
  `[proc.clobber-undeclared]`). Direction 2 (no disclaiming prose): no "not a
  guarantee / do not rely / incidental" language; the one "d4/a0 are clobberable"
  note (TrySpawnRing:983) is an INTERNAL register-availability comment (they sit
  on the movem frame + RingBuffer_Add clobbers them), not a preserved-register
  disclaimer — conformant.
- Name / magic-number audits: clean (descriptive labels; literals named or
  site-commented).

### Step-0 hazard sweep (retroactive, per the 2026-07-12 rule)
Gap-ledger grep for `entity_window` surfaced the **A2 walk-live rider** (ledger
line 999): `EntityWindow_DespawnObjects` is the 4th dynamic-live-list walker; the
A2 mid-walk-compact rail (retro-fix batch item 1) needs its DEBUG set/clear flag
hook, but that's fenced OUT of t12 and lands as a **post-merge batch rider** (the
batch fence opens for exactly this hook, using the batch's flag symbol).
Delete-only today ⇒ A1-safe ⇒ invariant COMPLETENESS, not a live hole. Carried to
the handoff.

### Step-5 — engine interrogation (per hot proc; verdict per line)

`EntityWindow_Scan` / `RescanY` / `DespawnRings` / `DespawnObjects` /
`TrySpawnRing` / `TrySpawnObject`. Conclusion (review-verified): bounded
per-frame cost, no fixed hotspot, **no optimization taken** — each cell says why.

| proc | invariant ladder | counter/cache | guard-coverage | hardware x-check | silent-tradeoffs |
|---|---|---|---|---|---|
| Scan | not-taken: frame edges (right-load d7, coarse-row d0) already hoisted once/frame; loop body is per-entry | n/a: `Camera_Y_Coarse_Prev` write-on-crossing / read-per-frame is symmetric, no budget | clean: `Entity_Window_Active` gates the section loop, reloaded after DeriveWindow/Slide (site-commented) | n/a: no VDP/DMA — pure RAM/ROM bookkeeping | clean: d5-reload-after-DeriveWindow documented |
| RescanY | not-taken: active mask loaded once, 4-entry loop; nothing below scope | n/a: no counter | clean: `btst d6,d5` gates each entry, sole path | n/a | clean: none present |
| DespawnRings | not-taken: X despawn edges (d6/d7) hoisted before the backward loop; per-entry ×6 math is per-entry | n/a: `Ring_Count` loop-bound + swap-with-last removal is the documented safe pattern, symmetric | clean: `.remove` reached from X-straggler-in-dead-section OR Y-far, both gated; clearLoadedRing on that path | n/a | clean: "far below → fall into .remove" site-commented |
| DespawnObjects | **not-taken (logged)**: `d2 = camY − DESPAWN_Y` recomputed per-object on the `.check_y` path (not every object — ANY_Y skips); hoisting adds a frame-const compute for all-ANY_Y frames + burns a reg across the DeleteObject clobber → cold trade | n/a: `Dynamic_Live_Count` loop-bound; spawn-order live-list walk (empty slots cost 0 — the occupancy win); symmetric | clean: null-guard + `code_addr` truth-guard + UNTAGGED skip + section-lifetime + Y-band all gate `.despawn`; clearLoadedObj on that path. **A2 walk-live flag hook OWED (post-merge batch rider)** | n/a | clean: ANY_Y-exempt-from-Y-band-only + fall-into-.despawn site-commented |
| TrySpawnRing | **not-taken (logged)**: Y load-band (camY±LOAD_Y) recomputed per-ring; invariant across a walker's ring loop, but this is the SHARED gate for 3 walkers — hoisting threads the band + a reg through all 3 sites for ~4 cyc/ring on X-ratcheted (small) counts → micro/cold | clean: loaded bit read(Test)-before-add / set-after; buffer-full → carry, no bit, retry — documented | clean: Y-band → collected → loaded (cheapest-first, commented); DEBUG no-dup assert guards every spawn path | n/a | clean: buffer-full-leaves-bit-clear-retry documented |
| TrySpawnObject | **not-taken (logged)**: same shared-gate Y-band recompute as TrySpawnRing; type-table lookup is per-object (necessary) | clean: loaded bit symmetric; Load_Object alloc-fail → no bit, retry — documented | clean: ANY_Y/Y-band → killed → loaded; alloc-fail guard; `slot_tag = entry|ANY_Y<<7` never collides with UNTAGGED $FF | n/a | clean: alloc-fail-retry + slot_tag ANY_Y-bit encoding site-commented |

**Owed oracle probe** (the hardware-x-check "can't verify statically" escape is
empty here — no VDP — so the only owed measurement is cost): a profile of
`EntityWindow_Scan` under a high-entity-churn scene to confirm the two logged
per-object/per-ring recomputes stay cold (the object-test scene fights static
setup per the oracle-harness ledger row).

### Neither-bucket headlines
- `falls_into` cleanly modeled the Init→Scan fall-through (collision.emp
  precedent) — the RIGHT answer over matching ASL's nop or sigil's bra.w.
- The shift-aware `.emp`-vs-fresh-`.lst`-region diff is the reusable technique
  for any future hand-set-twin lockstep (no disassembler needed).

## Merge addendum (2026-07-12)

**Attribution note (no history rewrite, per the review ruling):** the tranche-12
harness re-pins (pins.rs entity_window −0x1C/−0xC + downstream, mixed_dac_rom
game_loop pin-splice, repin_pins SOUND_API baseline, gap-ledger t12 rows) rode
the audit docs commits on sigil master (`93220c0`/`5c65641`/`bec172d`) — the
review seat's `git add -u` in the shared checkout; root cause owned, a hygiene
rule recorded. The hunks stay where they landed; the clean tranche commit
`869c6bf` carries the byte gate + kill-rows + design/packet. `f640473` (branch,
docs-only) rode the merge harmlessly.

**Merge commits:** aeon `2751a27` (merge) + `281198f` (gate-wiring integration);
sigil `e2ad6d7` (merge). `SIGIL_EMP_ENTITY_WINDOW` gate live (resume orgs plain
`$3C5A` / debug `$4570`).

**Final master ROM hashes** (rebuilt, gate-off = normal build unchanged):
- `s4.bin` 451861 B — sha256 `e55a010ce6470f3f4caca8c51cad9b795888fb9f39b32417ea87c79dff760046`
- `s4.debug.bin` 459735 B — sha256 `6c21b56cb2f68390c94fbcc548faf307e23f32981af4c292fe0e233f375708e5`

**Handoff — retro-fix batch:** gap-ledger **row 999** (A2 walk-live rider) is the
batch's entry point — the entity_window fence opens for exactly one hook: the
DEBUG set/clear walk-live flag on `EntityWindow_DespawnObjects` (4th live-list
walker), using the batch's flag symbol, asserted clear at `CompactDynamicLive`
entry. Delete-only today ⇒ A1-safe ⇒ invariant completeness, not a live hole.
