# migmask A/B — the specification (porter-written, overseer-executed)

`EntityWindow_MigrateMasks` strided `Entity_Scan_State` by 22 where the struct is
26. This parcel makes the stride derive from the struct
(`mul_const.w d0, #sizeof(EntityScanState), d1`). **The parcel is deliberately
behaviour-CHANGING**, so the bar is inverted from the recent optimization
parcels: a probe that reads the same on both ROMs has proven nothing. Every
probe below is designed to DIFFER, and each says both what "no difference" means
and what it does NOT mean.

The porter did not run the emulator. Every address, CRC and offset below was
re-read this session out of the built listings and a hexdump of the two golden
sets; nothing is carried from an earlier note.

## The two carts

| | OLD (chain 50) | NEW (chain 51) |
|---|---|---|
| debug | `crates/sigil-harness/golden/s4.debug.bin` at master `f4d87aae` | branch `migmask` |
| plain | `crates/sigil-harness/golden/s4.bin` at master `f4d87aae` | branch `migmask` |

CRC32 / size, both re-derived this session:

| shape | OLD | NEW |
|---|---|---|
| `s4.debug` | `159b152f` / 423571 | `7e273b14` / 423571 |
| `s4` | `4f43c8d9` / 411167 | `84c33dfc` / 411167 |

Identify the loaded cart by **ROM hash**, never by the reload diagnostic.

**Run every probe on the DEBUG shape.** Probe 3 needs it (`replay.emp` compares
checkpoint hashes only under `DEBUG == 1`), and running probes 1 and 2 there too
makes probe 3 the same run continued rather than a third setup.

## Hygiene: this parcel changes cycle counts

The replaced chain is 7 instructions / 44 cycles; the new lowering is 6
instructions / 38 cycles, per iteration, ×4 iterations per slide. A
`pause`-anchored A/B is therefore **invalid** — after the first slide the two
runs are in different phase, so any "same frame count" comparison is measuring
the phase difference, not the behaviour. Anchor every probe at a **PC
breakpoint**, and prefer anchors whose address is the same in both ROMs.

## Addresses — re-read from the listings and the two golden sets this session

ROM (68k address = file offset):

| symbol | plain OLD | plain NEW | debug OLD | debug NEW |
|---|---|---|---|---|
| `EntityWindow_Scan` (entry) | `$003A34` | `$003A34` | `$0046A8` | `$0046A8` |
| `EntityWindow_MigrateMasks` (entry) | `$003E1A` | `$003E1A` | `$004B1C` | `$004B1C` |
| the `move.b (a0,d0.w),d2` id read | `$003E30` | `$003E2E` | `$004B32` | `$004B30` |
| `EntityWindow_Slide` (entry) | `$003E70` | `$003E6E` | `$004B72` | `$004B70` |
| `GameState_OJZScroll_Init` | `$05C230` | `$05C230` | `$05E00E` | `$05E00E` |
| `Replay_OJZ_Fixture` | `$05C778` | `$05C778` | `$05E568` | `$05E568` |

`EntityWindow_MigrateMasks`, `EntityWindow_Scan`, `GameState_OJZScroll_Init` and
`Replay_OJZ_Fixture` are at the **same address in both carts** — that is why they
are the anchors. `EntityWindow_Slide` moved by −2 and is not a valid shared
anchor. The fixture **did not move**: the 340-byte blob at `$5E568` in the NEW
debug ROM is found at exactly `$5E568` in the OLD debug ROM, and at `$5C778` in
both plain ROMs. Do not re-derive it per cart.

RAM is untouched by the parcel; every cell below is at the same address in OLD
and NEW, and the plain and debug columns genuinely differ — read the right one.

| cell | plain | debug | bytes |
|---|---|---|---|
| `Entity_Scan_State` | `$FFFFAC2E` | `$FFFFAC52` | 104 (4 × 26) |
| `Entity_Window_Active` | `$FFFFAC9A` | `$FFFFACBE` | 1 |
| `Entity_Window_Anchor` | `$FFFFAC9C` | `$FFFFACC0` | 2 (`sec_x0`, `sec_y0`) |
| `Entity_Loaded_Masks` | `$FFFFACA2` | `$FFFFACC6` | 128 (4 × 32) |
| `Entity_Mask_Scratch` | `$FFFFAD22` | `$FFFFAD46` | 132 (4 ids + 4 × 32 mask) |
| `Camera_X` / `Camera_Y` | `$FFFFA12E` / `$FFFFA132` | `$FFFFA152` / `$FFFFA156` | 2 each |
| `Logic_Tick` | `$FFFF8004` | `$FFFF8004` | 4 |
| `Input_Source` | `$FFFF803A` | `$FFFF803A` | 1 |
| `Replay_Exit_Request` | `$FFFF803B` | `$FFFF803B` | 1 |
| `Replay_Done` | `$FFFF803C` | `$FFFF803C` | 1 |
| `Replay_Hold` | `$FFFF803D` | `$FFFF803D` | 1 |
| `Replay_Prev` | `$FFFF803E` | `$FFFF803E` | 1 |
| `Replay_Ptr` | `$FFFF8040` | `$FFFF8040` | 4 |
| `Replay_Check_Idx` | — | `$FFFFB406` | 2 (DEBUG only) |

`SEC_VOID` is `$FF` (read out of the `cmpi.b #$FF,d2` at the guard).
`offsetof(EntityScanState, ess_section_id)` is `$12`; `sizeof(EntityScanState)`
is `$1A` = 26.

## THE DRIVE — replay playback, anchored at the init breakpoint

**Do not use `Debug_Scene_Freeze`.** The freeze REMOVES THE PROC UNDER TEST:
`games/sonic4/test/ojz_scroll_test.emp:256-263` gates `jbsr EntityWindow_Scan`
behind `tst.b Debug_Scene_Freeze / bne .skip_entity_scan`, and
`EntityWindow_Scan` is the only caller of `EntityWindow_Slide`
(`entity_window.emp:897`), which is the only caller of `EntityWindow_MigrateMasks`
(`:1725`). A freeze-driven A/B would report "no difference" while measuring
nothing at all.

The drive is the standing input-replay net. The recipe, from
`aeon docs/superpowers/notes/2026-08-05-crash-report-ab.md` and
`2026-08-05-objtest-gate-ab.md`:

> persistent bp at `GameState_OJZScroll_Init` **before** `reload_rom`, poke
> `Input_Source` = 1 and `Replay_Ptr` = `Replay_OJZ_Fixture + REPLAY_HEADER_LEN
> (20)`, clear bps, resume.

**The anchor is not optional.** aeon `b014865`'s commit message states why,
verbatim:

> RECORDING PROCEDURE NOTE: never rewind Replay_Record_Idx mid-session — ring
> index N must mean 'Nth tick after the init anchor' on both record and
> playback, or checkpoint 0's hash is captured at a different game state than
> playback reaches

Poke at any other moment and the stream is phase-shifted against the checkpoint
hashes; the run then desyncs on both carts for a reason that has nothing to do
with this parcel.

Concretely, on each cart:

1. Persistent breakpoint at `GameState_OJZScroll_Init` (`$05E00E` debug).
2. `reload_rom`; verify the ROM by hash against the table above.
3. Run to the breakpoint.
4. Poke `Input_Source` = `$01` (`INPUT_PLAYBACK`), `Replay_Ptr` =
   `$05E568 + 20` = `$05E57C`.
5. Confirm the companion cells are at their init values before resuming —
   `Replay_Hold` = 0, `Replay_Prev` = 0, `Replay_Done` = 0,
   `Replay_Exit_Request` = 0, and (DEBUG) `Replay_Check_Idx` = 0. A non-zero
   `Replay_Prev` fabricates a press edge on the first replayed tick; a non-zero
   `Replay_Hold` eats the first stream byte.
6. Clear breakpoints, install the probe's breakpoint, resume.

Both carts get the identical sequence, and every comparison below is at the
**same breakpoint-hit ordinal**, counted from the resume in step 6.

## The consequence model — three directions, not two

`EntityWindow_BuildEntries` (`entity_window.emp:722-795`) assigns entries
ABSOLUTELY from the anchor it just stored: entry *k* is section
`(sec_x0 + (k & 1), sec_y0 + (k >> 1))` (`:742-749`), and `MigrateMasks` runs
only from `Slide`, which `Scan` calls only when that anchor MOVED (`:891-897`).
A DEBUG assert in `Slide` (`:1710-1721`) pins that at most one anchor byte
changes per slide.

**Therefore no valid entry can keep its section across a `MigrateMasks` call.**
Every entry's section coordinate shifts with the anchor, and distinct grid cells
have distinct flat ids. The only entry that "keeps its section" is
`SEC_VOID` → `SEC_VOID`, where nothing is carried and nothing is lost.

The three reachable directions, all of which the executor must be able to place:

* **(a) Failed migration → duplicate spawns.** Entry *k*'s (correct) section id
  appears in the snapshot at some *j*. On NEW, slot *k* receives snapshot block
  *j*. On OLD the id read at `22k + $12` is a ROM-pointer byte, the identity
  match fails, and slot *k* keeps the zeroes that
  `EntityWindow_InitSection`'s compare-clear (`:637-647`, which runs from
  `BuildEntries` BEFORE `MigrateMasks`) wrote. Every already-collected entity in
  that surviving section is re-treated as unloaded and spawns a second time.

* **(b) Chance match → foreign mask, and the entities NEVER SPAWN.** On OLD the
  garbage byte at `22k + $12` happens to equal snapshot id *j*, so snapshot
  block *j* is copied into slot *k* — an entry whose section is genuinely NEW
  (its slot was just zeroed by the compare-clear, correctly). Those bits mark
  entities of a *different* section as already loaded, so **rings and objects
  that should appear do not appear at all**. This is the worse of the two and it
  is a MISSING-content symptom, not a duplicate one. It is probabilistic — a
  1-in-256-ish byte collision per (entry, snapshot slot) pair — so **its absence
  refutes nothing**. Do not require it.

* **(c) A void entry is not skipped.** The `cmpi.b #SEC_VOID, d2` guard
  (`:1627`) tests the byte the stride just fetched. On OLD that byte is garbage
  for entries 1-3, so an entry whose real id IS `SEC_VOID` can fall through the
  guard into the match loop and receive a foreign mask. Harmless in itself (a
  void entry spawns nothing), but it produces a slot delta that fits neither (a)
  nor (b), and the executor must be able to say "that one is (c)" rather than
  treat it as an anomaly. The mirror case — a valid entry whose garbage byte
  happens to be `$FF` and is skipped — collapses into (a).

## Probe 1 — the mechanism. Guaranteed to differ; run it first

**Anchor:** breakpoint at the id-read instruction — debug OLD `$004B32`, debug
NEW `$004B30`. It is hit exactly 4 times per slide, once per window entry,
before any `SEC_VOID` skip, so all four values are always observed.

**Read:** `d0.w` at each of the four hits.

| hit | OLD (×22) | NEW (×26) |
|---|---|---|
| 0 | `$0000` | `$0000` |
| 1 | `$0016` | `$001A` |
| 2 | `$002C` | `$0034` |
| 3 | `$0042` | `$004E` |

The NEW column is `entry × sizeof(EntityScanState)`. The OLD column lands
entries 1/2/3 inside `ess_rom_type_tbl_ptr` / `ess_rom_obj_ptr` /
`ess_rom_ring_ptr` — ROM-pointer bytes read as section ids.

**If this does not differ, stop.** Either the wrong cart is loaded or the
breakpoint is on the wrong instruction. Nothing downstream is interpretable.
Probe 1 is a pure register read and cannot be defeated by the content of the
level, so it is also the drive's smoke test: reaching it at all proves the
replay drive got the window to slide.

## Probe 2 — the consequence, controlled. The parcel's primary evidence

**Anchor:** breakpoint at `EntityWindow_MigrateMasks` entry, `$004B1C` on BOTH
carts.

### Step 0 — the CONTENT precondition (without it the probe can pass while measuring nothing)

At the anchor, read (stride 26, on either cart — the RAM is identical here):

* the four live ids `E[k] = byte at Entity_Scan_State + 26k + $12`, k = 0..3;
* the four snapshot ids `S[j] = byte at Entity_Mask_Scratch + j`, j = 0..3;
* the four snapshot mask blocks `M[j] = 32 bytes at Entity_Mask_Scratch + 4 + 32j`.

**Require: some k in 1..3 such that `E[k] != $FF`, `E[k] == S[j]` for some j, and
`M[j]` is not all zero.**

Entry 0 is `0 × stride` and is computed correctly at ×22 AND at ×26, so a slide
whose only mask-carrying survivor lands at entry 0 produces byte-identical output
on both carts. If no `MigrateMasks` hit in the run satisfies the precondition,
the finding is **"no qualifying slide found in this stream"** — it is NOT "no
difference", and it does NOT refute the parcel. Advance to the next hit and
repeat; if the stream ends with none, say so and see "If the fixture never
qualifies" below.

### Step 1 — the control (non-vacuity), and it binds ONE slide only

At the anchor, on both carts, dump the proc's entire input:

* `Entity_Scan_State` (104 B), `Entity_Mask_Scratch` (132 B),
  `Entity_Loaded_Masks` (128 B), `Entity_Window_Active` (1 B),
  `Entity_Window_Anchor` (2 B), and the register `a4`.

`a4` is the snapshot base and is an argument (`Slide` sets `lea
Entity_Mask_Scratch, a4` at `:1724`); if it differs between carts every dump
address below is meaningless, so check it explicitly.

**These must be byte-identical between OLD and NEW at the FIRST hit that
satisfies step 0.** They will be, because everything upstream is byte-identical
until this proc first behaves differently.

**After that hit, input divergence is EXPECTED and is the parcel working.** Once
slide *N* writes different `Entity_Loaded_Masks`, slide *N+1*'s inputs differ by
construction. Do not read that as a broken control and do not read it as a
refutation. If per-slide isolation is wanted for a later slide, that slide must
be reached independently from a common state (a fresh anchored run stopped at
that hit ordinal), never by continuing one run.

### Step 2 — the output

`step_out`, then dump `Entity_Loaded_Masks` (128 B) on both carts. **Expect a
difference.** Identical inputs and different outputs at the same anchor isolate
the difference to this proc — that is the whole design of the probe.

### Step 3 — read the exhibit against the slide DIRECTION

Because entries are absolutely anchored, WHICH destination slots can lose their
mask is deterministic by slide direction, not "dependent on the ids in play".
Compute the direction from `Entity_Window_Anchor` (the NEW anchor, live at the
entry) against the old window the snapshot describes, or simply from which
entries' ids appear in the snapshot:

| slide | survivors land at entries | destinations reading a WRONG id under OLD | expected differing slots |
|---|---|---|---|
| right (`sec_x0`+1) | 0, 2 | 2 | **exactly one** |
| down (`sec_y0`+1) | 0, 1 | 1 | **exactly one** |
| left (`sec_x0`−1) | 1, 3 | 1 and 3 | **two** |
| up (`sec_y0`−1) | 2, 3 | 2 and 3 | **two** |

(Derivation: with entries 0..3 = (x0,y0), (x0+1,y0), (x0,y0+1), (x0+1,y0+1), a
rightward slide makes new entry 0 = old entry 1 and new entry 2 = old entry 3,
and so on. Destination 0 is the only one the bug computes correctly.)

The "expected differing slots" column counts direction (a) only — the
destinations that SHOULD receive a mask and whose id the bug misreads. A
direction-(b) chance match can add a differing slot at any other entry, so the
count is a floor, not a cap. Note also that the step-0 precondition and this
table agree by construction: `E[k]` can appear in the snapshot only at a
survivor destination, so on a right/down slide the qualifying `k` is forced to be
2 or 1 respectively.

**The ROM boots scrolling right, so the default run shows the WEAKER
one-slot exhibit.** A left or up slide is worth having deliberately. Two ways,
in order:

* **Take it from the stream if it is there.** Record the direction at every
  qualifying hit; if any is left or up, that is the slide to write up.
* **If the stream only ever slides right/down** and the one-slot exhibit is
  judged too thin, force one: break at `EntityWindow_Scan` (`$0046A8`, the same
  address on both carts, and upstream of everything this parcel touched), poke
  `Camera_X` (or `Camera_Y`) so the derived anchor moves by exactly one section
  in the chosen direction, then resume. Two constraints: the poke must move
  exactly one axis by exactly one section or `Slide`'s DEBUG single-axis assert
  (`:1710-1721`) fires; and it must be the same poke at the same breakpoint
  ordinal on both carts. **A poked run cannot also serve probe 3** — `Camera_X`
  is inside `Replay_Hash`, so the poke desyncs the fixture by construction. Run
  probe 3 on a clean, un-poked run.

**Corroboration for (a)**, optional: a few frames after a slide that exhibited
(a), census the object slots on both carts (`emulator_object_list`). OLD should
carry duplicates of at least one entity from the surviving section that NEW does
not. This is a symptom observation, not the proof — step 2 is the proof.

## Probe 3 — the shipped replay fixture, and the parcel's chief follow-up

Same anchored drive, DEBUG cart, **no pokes**, run to `Replay_Done = $FF` or to
`REPLAY DESYNC`.

`games/sonic4/data/replays/ojz_fixture.bin` is a 2059-tick recorded input stream
with 33 curated state checkpoints; in DEBUG a checkpoint mismatch raises
`REPLAY DESYNC` (`engine/system/replay.emp`), with `d0` = actual, `d2` =
expected, `d1` = `Logic_Tick` in the register dump.

**What `Replay_Hash` actually covers** (read out of `replay.emp:267-334` this
session) — this bounds how loud a desync can be:

* `Logic_Tick`;
* `Player_1`'s **address-free** SST spans only: `$02` motion, `$14` display,
  `$1E` status/anim, `$2A` entity, `$30` `sst_custom`, plus four word folds
  (`render_flags`, `anim`, the `sst_custom` tail, `interact`);
* `Camera_X` + `Camera_Y`;
* the section streaming cells `Section_Top_Row_Written`,
  `Section_Right_Col_Written`, `Section_Fwd_Neighbor_Data`,
  `Section_Bwd_Neighbor_Data`, and `Section_Plane_Dirty`;
* three object-system counters: `Dynamic_Live_Count`, `Dynamic_Free_SP`,
  `Effect_Free_SP`.

**It does NOT cover the SSTs of spawned entities.** A duplicate spawn or a
never-spawned ring reaches the hash only through `Dynamic_Live_Count` /
`Dynamic_Free_SP` / `Effect_Free_SP` (and, once the player touches the changed
world, through `Player_1`). So a desync is a real signal, but a *quiet*
behavioural difference — one that changes which entities exist without moving a
counter at a checkpoint tick — can hide from it.

**Dating (git-verified):** the fixture's last content-changing commit is aeon
`806a0de` (2026-08-05), which is AFTER `4ad9f9b` (2026-07-31) introduced the
stride bug. That proves the bug was **PRESENT in the build that recorded the
fixture**. It does **not** prove the bug ever **FIRED** during those 2059 ticks —
whether it did is exactly what this probe measures.

* **OLD must first reach `Replay_Done = $FF` with zero desyncs.** This is the
  probe's non-vacuity control; it is what aeon `b014865` recorded. If OLD also
  desyncs, the fixture is stale for an unrelated reason and probe 3 proves
  nothing — report that and drop the probe.
* **NEW desyncing is the exhibit**: the shipped, recorded behaviour of the game
  changed, at a named tick, inside the state the hash covers.
* **NEW completing clean is also a real result**, but a bounded one: it says the
  stream never slides in a way that moves a hashed cell at a checkpoint tick. It
  does not say the fix changed nothing — probe 2 is what says that.

**Either way this is a follow-up, not a regression to fix in this parcel.** A
desync means the fixture is a recording of the bug and must be re-recorded on the
fixed ROM before it can serve as a regression net again. Re-recording needs the
emulator and is the overseer's call; the porter has ledgered it.

## If the fixture never qualifies

If no `MigrateMasks` hit in the whole stream satisfies probe 2's step-0 content
precondition, the honest report is **"the shipped stream never slides with a
mask-carrying survivor outside entry 0"** — which is itself a finding (it bounds
the live blast radius, and it explains a clean probe 3). The exhibit then comes
from probe 2 run over a forced slide (step 3's second bullet), or from recording
a new stream that traverses a section boundary leftward with rings collected —
which is work the overseer needs anyway if probe 3 desyncs.

## What would refute the parcel

* Probe 1 shows the same four `d0.w` values on both carts. Then the loaded carts
  are not the two carts named above.
* Probe 2 step 2 shows identical outputs at a hit that DID satisfy step 0's
  content precondition, with byte-identical inputs. That would mean the stride
  does not reach the outcome the fix claims, and the "live bug" framing is wrong.

## What would NOT refute the parcel (each of these was a defect in the first draft of this note)

* Inputs differing at the second and later slides. That is the fix propagating,
  by construction.
* Direction (b) never appearing. It is a chance byte collision.
* "No difference" from a `Debug_Scene_Freeze`-driven run. That run does not
  execute the proc.
* A clean probe 3 on NEW. It bounds the blast radius; it does not deny the
  mechanism probe 1 measured directly.
* Only one slot differing. That is the correct, predicted result for a rightward
  or downward slide.
