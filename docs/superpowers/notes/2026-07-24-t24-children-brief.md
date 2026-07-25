# TRANCHE 24 BRIEF — children.asm conversion (LEAN tranche)

**Dispatch: overseer-cut 2026-07-24 (Volence present; t23 merged, pushed and
swept; merge queue EMPTY). Single-lane.** Sixth tranche under the LEAN
amendment.

**Target ruled: `engine/objects/children.asm` → `children.emp`.** It is the
LAST unported island inside the object chain (`entity_window` → **children**
→ `load_object` are otherwise contiguous .emp regions), the last non-debug
engine 68k file, and a pure plain-shape file (no `ifdef`, no `__DEBUG__`, no
`assert` — shape-INVARIANT length). The debug trio (`debugger.asm` 806 L +
`error_handler.asm` 258 L + `sound_debug.asm` 98 L) is deliberately NOT in
this tranche: those three are debug-shape-only, share the assert/macro-tower
entanglement (kill row 16), and want their own step-0 design note — they are
the t25 cluster that closes the engine 68k backlog.

**Canonical sources (read before cutting code):**
`docs/superpowers/notes/campaign-port-loop.md` (re-read at EVERY step
boundary — the LEAN amendment and the dry-panel rule both live there), the
t23 close packet (freshest template), campaign gap-ledger + twin-scaffolding
kill-list for the step-0 sweep.

## Scope

1. **`engine/objects/children.asm`** (483 L, 7 procs: `PopulateSpawnedPieceCount`,
   `CreateChild_Normal`, `CreateChild_Complex`, `CreateChild_FlipAware`,
   `CreateChild_Linked`, `DeleteChildren`, `CreateEffect_Normal`,
   `CreateEffect_Simple` — 8 labels, 7 entry points plus the shared helper)
   → `children.emp`. ONE file; no seam split (no data tail — the descriptor
   tables live GAME-side in `games/sonic4/objects/test_parent.asm` and stay
   there).

Full loop `0 → 1 → 2 → (3 → 4 → 5)* → 6 → merge`; one dry-panel; step 6
once; one close packet.

## THE DEAD-ENTRY-POINT RULING (overseer-cut; do NOT re-litigate, do NOT cut)

Caller census run this session (whole tree, `.asm` + `.emp`, worktrees
excluded):

| proc | live call sites |
|---|---|
| `PopulateSpawnedPieceCount` | test_churn.asm:45, test_stress_emitter.asm:29 (+4 internal `bsr`) |
| `CreateChild_Normal` | test_parent.asm:138 |
| `DeleteChildren` | test_parent.asm:165 |
| `CreateEffect_Normal` | test_emitter.asm:41, test_stress_emitter.asm:44 |
| **`CreateChild_Complex`** | **NONE** |
| **`CreateChild_FlipAware`** | **NONE** |
| **`CreateChild_Linked`** | **NONE** |
| **`CreateEffect_Simple`** | **NONE** |

Four of eight entry points have zero call sites. Per step-4 verb (d), this
is **deliberate/feature dead code — an engine API awaiting its consumer, not
orphaned by our work** (the `AnimateSprite_PerFrame` precedent): **PORT ALL
EIGHT FAITHFULLY, cut nothing.** The census is a FLAG TO VOLENCE, carried in
the close packet as a named decision row (keep-as-API vs delete-and-slide);
the overseer surfaces it at the merge gate. A silent cut is a gate failure;
so is walking past the census without recording it.

## Mechanics (standing bars — t24 values)

- Branch `port-tranche24` both repos, worktrees `.worktrees/port-tranche24`;
  **seed the aeon worktree's `games/sonic4/data/editor/` by rsync from main
  and verify canonical CRCs before any code** (the padded-ROM trap).
- **Canonical: plain `01832b1a`/421157 · debug `154076f8`/429232**
  (`EndOfRom` plain `0x5DB60` / debug `0x5F65A`; masters aeon `1470af2` /
  sigil `9f0abfd`). Strict baseline **2588/0** paired, `AEON_DIR` pointed at
  the BRANCH tree (never master — the paired-state gate). One shape per
  build invocation (`./build.sh` = plain only, `DEBUG=1` = debug only).
- cwd resets every Bash call — `cd` explicitly; explicit-path commits only
  (never `git add -u`); never push; failures-first test output; oracle MCP is
  overseer-only — bring NAMED probe lists for anything needing live
  measurement.

## Region + pin machinery (overseer-derived — verify against your listing)

`children.asm` currently assembles into the hole between two pinned regions.
From `crates/sigil-harness/src/pins.rs` and `engine/engine.inc`:

- `ENTITY_WINDOW`: plain_base `0x347E` len `0x8BA` → ends **`0x3D38`**;
  debug_base `0x3BDE` len `0xD28` → ends **`0x4906`**. Those two values are
  exactly the `SIGIL_EMP_ENTITY_WINDOW` else-arm resume orgs
  (engine.inc:383-387).
- `LOAD_OBJECT`: plain_base **`0x4046`**, debug_base **`0x4C14`**.
- ⇒ **NEW region `children`: start `PopulateSpawnedPieceCount`, end
  `Load_Object`; plain `0x3D38`, debug `0x4906`, len `0x30E` BOTH shapes**
  (shape-INVARIANT — no asserts, no `__DEBUG__`; confirm from the real
  listing, do not trust this arithmetic alone). Gate `SIGIL_EMP_CHILDREN`,
  test `children_port` (`crates/sigil-cli/tests/children_port.rs`,
  `load_object_port.rs` is the closest template — same shape-invariant
  single-file class).
- engine.inc gets the standard gate around line 389 (`ifndef
  SIGIL_EMP_CHILDREN … else org <children end> endif`), with the else-arm
  org = `LOAD_OBJECT.plain_base`/`debug_base`. The house comment block on
  the neighbouring gates is the template, including the "these org values
  are sonic4-shape addresses — the gate define must never be set for other
  games" note.
- **Anchor-sharing check (named because it is the one novel bit of pin
  machinery here):** `PopulateSpawnedPieceCount` is simultaneously
  `ENTITY_WINDOW`'s END anchor and `CHILDREN`'s START anchor, and
  `Load_Object` is simultaneously `CHILDREN`'s end and `LOAD_OBJECT`'s
  start. Prove the repin pipeline still resolves both after the new gate
  exists (it derives anchors from the gate-off AS listing, where the twin
  still defines every symbol — but prove it, don't assume it), and that
  `ENTITY_WINDOW`'s resume org and `CHILDREN.plain_base` remain equal.
- **Expected byte movement (overseer pre-computed from the live listing —
  verify, don't trust):** the step-2 `bsr.w PopulateSpawnedPieceCount` →
  `jbsr` conversion relaxes ONLY where the 8-bit displacement reaches. From
  the canonical plain listing (`PopulateSpawnedPieceCount` = `0x3D38`), the
  `CreateChild_Normal` site at `0x3DB2` is −124 and **relaxes (−2)**; the
  sites at `0x3E3A` (−260) and `0x3EE8` (−434) and everything further down
  the file do NOT. Same story for the in-proc `bra.w`/`beq.w` sites once
  they go bare. So expect a SMALL negative delta, all of it absorbed by
  re-pinning `load_object` and downstream — one wave, one re-pin.
- **5-site ripple** on any byte-changing wave: `repin` does ONLY `pins.rs`
  (and prints the engine.inc orgs); `engine.inc` orgs / `mixed_dac_rom.rs` /
  `repin_pins.rs` are HAND-edited; `repin.toml` gets the new `[[region]]`
  block THIS tranche (region added). children is mid-chain, so a byte change
  here slides every region from `load_object` downstream — batch step-2/loop
  byte changes into ONE wave with ONE re-pin, and run the row-1257 sweep
  (every hardcoded engine-address fixture in `crates/` at or past
  `0x3D38`/`0x4906`) on that wave.

## Ownership flips

**NONE.** Recon-verified: no `.emp` file calls any children symbol (the only
`.emp` hits are comments in `test_particle.emp` and `core.emp`). All live
callers are game-side `.asm` (`test_parent`, `test_emitter`,
`test_stress_emitter`, `test_churn`) calling INTO the new `.emp` owner —
the well-proven `.asm → .emp` jsr class (`AnimateSprite`, `Perform_DPLC`,
`AllocDynamic`, `Load_ObjectList` all already crossed this way from
`.asm` callers). children's own callees are already `.emp`-owned
(`AllocDynamic` core.emp:105, `AllocEffect` core.emp:164, `DeleteObject`
core.emp:196) — a pure-callee port, same posture as boot. If the step-0
sweep finds an extern the overseer missed, it flips per standing law and
owes its two-module link test.

## Step-0 hazard pre-sweep (overseer findings — verify and complete)

**Ledger/kill-list sweep (done; complete it, don't repeat it blind):**
file-name and proc-symbol greps over the gap-ledger and kill-list come back
essentially clean. The TRIP-CHECKS that DO bite:

- **`FRAME_PIECE_COUNT` (kill row 2)** — children is a new `.emp` consumer.
  Take it from the constants twin (`use engine.constants`); a file-local
  mirror is EXTINCT per the shared-module batch. Same for `RF_XFLIP`
  (`engine/system/constants.emp:33`, load_object.emp:21 is the import
  precedent).
- **Ledger F2 row (AnimId/MappingFrame documentary, ~line 1360-1375)** —
  its REOPEN condition is "the field-store domain-check is designed, or an
  anim/frame value is next passed in a register param". children **stores**
  `SST_anim` from a raw descriptor byte and `#$FF` into `SST_prev_anim`:
  this tranche is a DEMAND INCREMENT on the field-store class. Adjudicate on
  the record (build vs increment-the-row); do not walk past it silently.
  Expected outcome is ledger, not build — but say so with the reasoning.

**Type layer — this is the type-rich part of the tranche.** `Sst` is fully
typed (`engine/objects/sst.emp`): `code_addr: ObjRoutine`, `x_pos/y_pos:
Coord`, `x_vel/y_vel: Velocity`, `anim: AnimId`, `prev_anim: AnimId`,
`art_tile: VramArtTile`, `mappings: u32`, `parent_ptr/sibling_ptr: u16`.
children is almost entirely *raw ROM bytes → typed SST fields*, so step-2
item 6 has real work: `d2` carries an `ObjRoutine` through every variant,
`d0` carries a pixel offset that becomes a `Coord`, `d1` in `Linked` carries
a slot pointer. Adopt existing newtypes at signatures (`(a0: *Sst)` at
minimum, matching core.emp's `DeleteObject`), bless the true construction
sites, and LEDGER (don't force) anything the strict-degrade lattice can't
carry.

**Named idiom census (step-3(a)/step-4 material — overseer pre-count, the
porter re-counts):**

- **The pixel → 16.16 `Coord` idiom** (`ext.w / swap / clr.w / add.l
  parent-coord / move.l`) runs **8 times inside children alone**, and the
  `swap / clr.w / move.l → Coord` tail is CORPUS-WIDE: `camera.emp:111,132,
  143`, `load_object.asm:44-49`, `ojz_scroll_test.asm:61-68`. That is well
  past the consolidation bar — a comptime-fn / construct candidate whose
  interface is TYPED (`Coord` in, per the step-4 typed-signature rule), with
  a ready-made step-6 corpus sweep attached. Run the macro-port rule on the
  interface if you build it. Adopt / build / ask — named outcome either way.
- **Structural clones**: `CreateChild_Normal` vs `_Complex` vs `_FlipAware`
  are the same loop with additive segments; `CreateEffect_Normal` is
  `_Normal` minus the sibling chain; the alloc-fail *skip-the-rest* walker
  appears in FIVE variants over two record strides (4-byte and 14-byte).
  This is the `emit_piece_loop` template class — the varying terms name the
  parameters. Adjudicate on the record (a template that makes the five
  bodies one skeleton is the strongest step-4 candidate in this file).
- **Descriptor record format** (4-byte and 14-byte, `dc.w objroutine(...)` +
  signed offset bytes, `dc.w 0` sentinel — see test_parent.asm:143-150) is a
  data-table DSL candidate, but the tables are GAME-side `.asm` and out of
  scope: LEDGER as demand data (it joins the offset-table/`table` family),
  do not port game data this tranche.

**Correctness/contract hazards to carry into step 3(b) and the panel:**

- **`clobbers()` is an EXHAUSTIVE LICENSE.** `AllocDynamic` is
  `clobbers(d0) out(a1 if eq) preserves(a0)`; `AllocEffect` is
  `clobbers(d0) out(a1 if eq)`. children pushes `d3/a0-a1` (and `d1-d5/a0`
  in `Linked`) around every alloc call — under the ratified convention only
  `a1` is actually at risk. **Overseer-named step-5 candidate: the redundant
  movem save/restore around 6 alloc call sites.** It is byte-changing and
  spawn-path (not per-frame-hot), so the LEAN bar applies — measure it in
  the emitter/churn regime, then take it or log it with numbers. Do NOT take
  it on reasoning alone, and do NOT skip the interrogation because it looks
  small.
- **`AllocDynamic`'s CALLER INVARIANT** (core.emp:96-103): the caller must
  write `code_addr` before the NEXT alloc, or a compaction pass drops the
  slot. Every children variant currently satisfies it (the `move.w d2,
  SST_code_addr(a2)` immediately after the alloc). Any step-4/step-5
  reshuffle that moves that store is a correctness regression — name the
  invariant in the ported file's comments so the next reader can't break it.
- **The stack-resident running position in `CreateChild_Linked`**
  (`4(sp)`/`(sp)` under a `movem.l d1-d5/a0` push/pop, cleaned with
  `addq.w #8, sp` on BOTH exits incl. `.link_fail`) — hand-computed stack
  displacements are exactly the gate-blind class; C2's named input.
- **`movea.w` pointer reconstitution** (`movea.w d1, a1`, `movea.w d0, a1`)
  relies on RAM living in the sign-extending `$FFFFxxxx` window; SST links
  are stored as `u16`. Present-tense contract comment if it isn't already
  said; C2 input.
- **Chain-shape divergence**: `_Normal`/`_Complex`/`_FlipAware` PREPEND
  (new child points at the old head) while `_Linked` APPENDS (previous child
  points at the new one), and `DeleteChildren` walks whatever it finds.
  Both terminate only because a freshly allocated slot's `sibling_ptr` is
  zero. Verify that against the allocator and make the reliance explicit —
  an undocumented reliance on allocator-zeroed fields is a finding.
- **`DeleteChildren` header claims `Clobbers: d0-d1, a1-a2`** but `a2` is
  never touched (and `d1` comes only from `DeleteObject`'s license) —
  contract audit item; over-claimed licenses get tightened on touch.
  `DeleteChildren` also does not clear the deleted children's `parent_ptr`
  and does not recurse into grandchildren: decide whether each is contract
  or omission, and say which in a comment.
- **Comment-claim audit**: `; piece count is at FRAME_PIECE_COUNT (+4),
  after 4 bbox bytes` (line 21) is a layout claim — verify it against the
  real mapping-frame format, and prefer the named const over the prose.

**Step-5 regime (this file HAS a live regime — use it):** children runs at
spawn/despawn events, and the object-test scenes drive it hard —
`test_stress_emitter` and `test_churn` call `CreateEffect_Normal` /
`PopulateSpawnedPieceCount` under sustained churn. **Bar A applies: fresh
plain-shape addressable SELF-time, census/inclusive/debug numbers
INADMISSIBLE; Bar B applies to any A/B (frame-anchored, lag-frame counter
recorded both sides).** Any live measurement comes to the overseer as a
NAMED probe list — oracle is overseer-only, and never driven from a
background subagent.

## Panel composition (dry-panel rule)

**A1 + B1 + B2 + C1 + C2 — five lenses; C3 NOT active** (children touches no
VDP, DMA, interrupt, or bus state — say so in the packet rather than running
an empty lens). B2 is active BECAUSE of the `Coord` idiom census: the
question "does the corpus re-hand-roll this file's new shape" has a concrete
target this time. C2 is the weighted lens (stack displacements, alloc
contracts, chain termination, `movea.w` pointer width). **Lens subagents run
SYNCHRONOUSLY — the running parent awaits them; no fire-and-stop.**

## Acceptance

Per-file step-1 gate list with named artifacts (byte gate BOTH shapes; NEW
`children` region pin per-shape; `mixed_tranche24` acceptance arm including
the shared-anchor proof and the `.asm → .emp` caller resolution from at
least one game-side test object; negative probes; gate-off CRC neutrality);
full paired strict green from the BRANCH tree at every byte-changing commit;
any byte-changing wave batched into ONE re-pin carrying the row-1257 sweep +
the 5-site ripple; dry = a full 3→4→5 circuit empty at all three steps, THEN
a clean panel round; step-6 enumeration (the `Coord` construct's corpus
sweep is the expected headline); close packet in house format (scoreboard,
filled step-2 checklist, filled 3(a)/3(b)/5 interrogations, byte-delta
table, per-pass step-3-vs-step-5 breakdown, neither-bucket headlines, the
dead-entry-point census row); ledger/kill rows same-commit.

**STOP at the merge gate** — the overseer countersigns (own dual rebuild,
own strict run, second look) and runs the merge ceremony + PROVENANCE
re-baseline. **Checkpoint discipline (a)/(b)/(c): STOP after steps 0-2 with
the raw-data report before entering the loop.**
