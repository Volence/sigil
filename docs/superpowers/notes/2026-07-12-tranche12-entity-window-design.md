# Tranche 12 — entity_window.asm port (step-0 design)

Date: 2026-07-12. Target: `aeon/engine/objects/entity_window.asm` (1532 lines,
30 procs) → `engine/objects/entity_window.emp`. The **ratifying demand** for the
diagnostics construct (assert/`if DEBUG`): 11 assert sites already live in the
`.asm` twin. Largest port target yet — a **multi-session** tranche.

Baseline at start: both repos on `master`, clean; 26 port tests green;
diagnostics + object-pool-occupancy merged both sides. This file is the
POST-occupancy shape (DespawnObjects walks the dynamic live list, `2bb1e92`).

## Shape inventory (5 families)

1. **Rolling collected/killed bitmask** — `Collected_{Init,FindSlot,ClaimSlot,
   ParkSlot,UnparkSlot,UpdateCenter}`, `Collected_{Check,Mark}Ring`,
   `Killed_{Check,Mark}Object`. Slot/park linear scans, respawn-park copy loops,
   3×3 evict with divide-by-repeated-subtraction.
2. **Loaded bitmasks** — `EntityLoaded_{Test,Set,Clear}` (32-byte slots, `lsl #5`),
   `EntityWindow_EntryForSection`. Two compile-time `if/error` static asserts guard
   the slot geometry (lines 421-426).
3. **Window core** — `EntityWindow_{InitSection,DeriveWindow,BuildEntries,Init}`.
4. **Scan/spawn** — `EntityWindow_{Scan,TrySpawnRing,ScanRingsRight,
   PopulateSectionRings,TrySpawnObject,ScanObjectsRight}`, `RescanY`,
   `Rescan{Rings,Objects}`.
5. **Despawn/slide** — `EntityWindow_{DespawnRings,DespawnObjects,MigrateMasks,Slide}`.

## Extern surface (all cross-seam — mirror + drift-lock per rings.emp precedent)

- **Constants** (~28): `COLLECTED_*`, `KILLED_BITMASK_OFFSET`, `MAX_LIST_ENTRIES`
  (=128, `constants.asm:463`), `MAX_TRACKED_SECTIONS`, `ENTITY_LOADED_SLOT_SIZE`,
  `ENTITY_LOADED_OBJ_OFFSET`, `SEC_VOID`, `SECTION_SIZE(_SHIFT)`, `SCREEN_{WIDTH,
  HEIGHT}`, `ENTITY_{LOAD,DESPAWN}_BUFFER(_Y)`, `ENTITY_RESCAN_COARSE_MASK`,
  `OEF_TYPE_{SHIFT,MASK}`, `OBJ_ENTRY_SIZE`, `RING_BUFFER_ENTRY_SIZE`,
  `SLOT_TAG_UNTAGGED`. All plain `=`/`equ` → **co-spellable** (see assert note).
- **RAM**: `Ring_Collected_{Window,Park}`, `Collected_Park_Next`,
  `Entity_{Loaded_Masks,Scan_State,Mask_Scratch}`, `Entity_Window_{Center_ID,
  Anchor,Active,OriginX,OriginY}`, `Camera_{X,Y,Y_Coarse_Prev}`, `Ring_{Count,
  Buffer}`, `Current_Act_Ptr`, `Dynamic_Live(_Count)`.
- **Structs**: `EntityScanState_*` (11 fields + `_len`), `Sec_sec_*`, `SST_*`,
  `Act_grid_w`.
- **External procs** (jbsr/jbra): `RingBuffer_{Clear,Add,Remove}`,
  `Section_{GetSecPtrXY,FlatIDXY}`, `Load_Object`, `DeleteObject`.
- **Macros** (defined `engine/macros.asm:315,331`): `clearLoadedRing bufReg,idxReg`
  and `clearLoadedObj sstReg` — each an ~8-instr block with an internal proc-local
  label (`.clr_ring_skip`/`.clr_obj_skip`). See §Construct pass.

## The 11 assert sites — operand co-spellability (LOW risk)

| line | assert | comparand |
|---|---|---|
| 74,98,119,147,437,452,467 | `assert.w dN, lo, #MAX_LIST_ENTRIES` | symbol |
| 517 | `assert.l a0, ne, #0` | imm 0 |
| 282 | `assert.b d1, eq, #1` | imm 1 |
| 883 | `assert.w d5, ne, d4` | register |
| 1501 | `assert.w d1, eq` | (no comparand) |

Only ONE symbol comparand: `#MAX_LIST_ENTRIES`, a bare `= 128` constant that
resolves identically in both dialects → **co-spellable**, so this file does NOT
hit the operand-spelling divergence that bit core.emp's occupancy asserts
(`#extern("Dynamic_Slots")` vs `#Dynamic_Slots`, the 24-byte message split). The
mirrored `MAX_LIST_ENTRIES` const in the .emp spells `#MAX_LIST_ENTRIES` exactly
as AS. Verify at build: `assert.w d1, eq` (no comparand) must lower to the same
`assert.w d1, eq` bytes AS emits — confirm the construct supports the 2-arg form.

## DEBUG conditionals → `if DEBUG == 1 { ... }` (core.emp / rings.emp precedent)

- `ifdebug X` (one-liner) → `if DEBUG == 1 { X }`.
- `ifdef __DEBUG__ … endif` (multi-line: lines 268-283, 857-886, 1487-1503) →
  `if DEBUG == 1 { … }`. Same target; the `.emp` DEBUG symbol gates both.
  Precedent: `core.emp:260,426`, `rings.emp:78`.
- The two compile-time static asserts (`if ENTITY_LOADED_SLOT_SIZE <> 32 / error`,
  lines 421-426) → **`ensure(...)`** (rings.emp:31-34 precedent):
  `ensure(ENTITY_LOADED_SLOT_SIZE == 32, "…")` and
  `ensure(ENTITY_LOADED_OBJ_OFFSET*8 == MAX_LIST_ENTRIES, "…")`.

## DEBUG-conditional branch widths (6 sites) — step-1 lock, step-2 decision

Lines 144, 197, 824, 827, 837, 846 are explicit `.w` branches commented
"DEBUG assert expansions exceed short range". The width is **conditional on the
DEBUG flag**: with asserts present (DEBUG shape) the target is far → needs `.w`;
in the plain shape the assert is gone → `.s` may reach.

**Step-1 (transcription gate)**: keep all 6 **explicit `.w`**, matching the AS
twin's current `.w`-in-both-shapes byte-exactly. Correct and required for step 1.

**This is NOT a width exception and NOT a Spec-5 deferral** (amended per Volence,
2026-07-12; my earlier "twin can't match" framing was wrong). The emp frontend
already relaxes an UNSIZED conditional branch `.s`/`.w` **per shape** via Core's
relaxation ladder (`lower/code.rs:15`), and the AS twin CAN match per-shape by
wrapping each site in the same DEBUG conditional —
`if DEBUG==1 / beq.w / else / beq.s / endif` — or a suffixless auto-sized branch
(house-style call: the engine currently has ZERO suffixless branches).

Per the ratified port-loop clarification, **explicit twin width is never a width
exception**. So the step-2 default here is the **lockstep shrink + re-pin**:
convert the 6 sites to the per-shape form on BOTH the .emp and the AS twin, let
the unsized build width-select, re-pin. Skipping the shrink is a **logged step-5
decision for Volence's gate**, not a silent default and not deferred to Spec 5.

**Do NOT commit to a byte count.** The magnitude is whatever the unsized build
measures — several sites may still need `.w` in the plain shape (line 197's
`bsr.w Collected_UnparkSlot` spans the entire `Collected_ParkSlot` body and very
likely does NOT reach `.s` even with asserts gone). Measure, don't predict.

## Construct pass (step 4) candidates — inventory only, decide in-port

- **`clearLoadedRing`/`clearLoadedObj`** → comptime-fn helpers returning `Code`
  with a self-contained local label (`.clr_ring_skip`). The asm-splice /
  skeleton-owns-labels resolution (t11) makes this expressible now. Likely built
  at step 1 (the file DEMANDS the two factored blocks; AS already factored them).
- **`clear_longs` adopt ×3** — the 8× `clr.l {COLLECTED,KILLED}_BITMASK_OFFSET+N`
  block appears verbatim in `Collected_Init`, `ClaimSlot.claim`,
  `UpdateCenter.evict`. `clear_longs` already shipped — adopt.
- **section-match unroll ×2** — the 4× `cmp.b Entity_Scan_State+len*N+ess_id / beq`
  chain in `DespawnRings` and `DespawnObjects` → a `section_match_any(target)`
  comptime-fn.
- **Y load/despawn band check** — `subi BUFFER_Y / cmp / blt / addi / cmp / bgt`
  recurs (TrySpawnRing, TrySpawnObject, DespawnRings, DespawnObjects) → band-check
  helper (parameterize LOAD vs DESPAWN buffer). Weigh against the byte-lock
  branches inside it.
- **divide-by-subtraction ×2** (`.div_center`/`.div_slot`) and **index×6 scaling**
  (`add d0,d0/add d4,d0/add d0,d0`, ×3 sites) — small; taste-gated.

## Step-1 sourcing decisions (surfaced during setup, 2026-07-12; my calls)

- **Constant sources**: `use engine.system.constants.{SLOT_TAG_UNTAGGED,
  SCREEN_WIDTH, SCREEN_HEIGHT}` (already exported there). Everything else is
  **mirrored locally** in entity_window.emp as `const` + `ensure(extern()==)`
  drift-lock — the game-owned collected constants (`COLLECTED_WINDOW_SLOTS,
  COLLECTED_SLOT_SIZE, COLLECTED_PARK_SLOTS, COLLECTED_PARK_ENTRY_SIZE`; truth
  `games/sonic4/config/constants.asm`, rings.emp precedent) AND the engine
  constants not yet in constants.emp (`COLLECTED_*_OFFSET`, `COLLECTED_EMPTY_TAG`,
  `COLLECTED_MASK_BYTES`, `MAX_LIST_ENTRIES`, `MAX_TRACKED_SECTIONS`, `ENTITY_*`,
  `SECTION_SIZE(_SHIFT)`, `SEC_VOID`, `OEF_*`, `OBJ_ENTRY_SIZE`,
  `RING_BUFFER_ENTRY_SIZE`). Local keeps step-1 byte-isolated. Step-4 candidate:
  hoist the genuinely engine-wide ones to constants.emp.
- **Struct field access**: `Sst` uses the existing `sst.emp` twin
  (`Sst.field(a0)`). `EntityScanState`/`Sec`/`Act` have NO twins yet →
  **mirror their field offsets as local `const` + drift-lock** and use the
  faithful `EntityScanState_ess_section_id(a1)` displacement spelling (an
  `extern()` can't be a displacement — must be comptime). **Defer the
  `EntityScanState` struct-twin to step 4** (construct-pass "adopt", byte-neutral
  typed access — the sst.emp architecture). Not scaffolding-to-undo: the offset
  consts are the faithful step-1 form; step 4 replaces them with the typed twin.
- **The two `clearLoaded{Ring,Obj}` macros**: inline-expand at their single call
  sites (faithful AS macro-expansion). Step-4 candidate: comptime-fn helper
  (asm-splice/self-contained-label makes it expressible).

## Port test + sequencing

- New `crates/sigil-cli/tests/entity_window_port.rs`; register in `ports.rs`.
  Byte gate BOTH shapes (plain + DEBUG), region pin, mixed-build acceptance,
  gate-off neutrality. The DEBUG shape is load-bearing here (11 asserts) — the
  debug byte gate is the primary guard for the `.w` byte-locks and the assert
  message bytes.
- Sequence: **step 1 transcribe** (mirror consts + drift-lock, faithful 1-1,
  explicit `.w` byte-locks, asserts via construct, macros → helpers) → prove both
  shapes byte-identical → **step 2 modernize** (bare Bcc; the 6 DEBUG-width sites
  → per-shape unsized/conditional form on BOTH sides, measure the unsized build,
  lockstep shrink + re-pin OR log a not-taken decision for Volence's gate;
  jbra/jbsr, brace-indent) → **step 3 retrospect**
  → **step 4 construct pass** (clear_longs adopt, section-match, band-check) →
  **step 5 engine** → merge gate (Volence).

## Open questions to settle before/at step 1

1. Does the diagnostics construct support the **2-arg `assert.w d1, eq`** (no
   comparand) form line 1501 needs? If not → step-1 language ask.
2. `if DEBUG == 1 {}` around a **single mid-proc statement** (the `ifdebug`
   one-liners at 74/98/… sit between live instructions) — confirm it lowers with
   zero plain-shape bytes AND doesn't perturb the surrounding branch offsets in
   the plain shape (it must, for the byte-locks to be the ONLY width divergence).
