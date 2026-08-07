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

---

# EXECUTION — overseer's foreground run (2026-08-06)

Carts: OLD = `s4.debug.bin` golden at master `f4d87aae` (`159b152f`/423571);
NEW = the same golden on this branch (`7e273b14`/423571). **Both verified by
`memory_hash`, never by the reload diagnostic — whose `first16` is byte-identical
for the two carts and therefore cannot distinguish them.** Symbols loaded from
`s4.debug.lst`, which independently confirmed the RAM table (`Camera_X` =
`$00FFA152`, `Entity_Window_Anchor` = `$00FFACC0`).

## PROBE 1 — PASS. The mechanism differs, 8 of 8 predictions exact.

Breakpoint at the id-read (`$004B32` OLD / `$004B30` NEW), `d0.w` at each of the
four per-entry hits. Read with step-then-resume, because resuming *at* a
breakpoint re-triggers it without executing (an identical register file on two
consecutive breaks is the tell).

| entry | OLD predicted | OLD measured | NEW predicted | NEW measured |
|---|---|---|---|---|
| 0 | `$0000` | **`$0000`** | `$0000` | **`$0000`** |
| 1 | `$0016` | **`$0016`** | `$001A` | **`$001A`** |
| 2 | `$002C` | **`$002C`** | `$0034` | **`$0034`** |
| 3 | `$0042` | **`$0042`** | `$004E` | **`$004E`** |

**Corroboration nobody specified.** The breakpoint sits *on* the `move.b`, so
`d2` still holds the PREVIOUS entry's fetched id. Reading it across the hits
gives the ids actually fetched:

* OLD: `$01`, `$1F`, `$1F` — a repeated value that is a ROM-pointer byte.
* NEW: `$01`, `$02`, `$04` — a coherent set of section ids for a 2×2 window.

That is the defect exhibited directly on the shipping code path: the old stride
reads pointer bytes and calls them section ids.

## PROBE 3 — ANSWERED, and it CORRECTS the packet's headline.

Both carts ran the fixture from the anchored init breakpoint to completion.
`Replay_Ptr` ended at **`$0005E6A0` on both**; `Replay_Done` = `$FF` on both;
and `Entity_Loaded_Masks` after the full 2059-tick run hashed **`0x25913C7E` on
both carts, identically**.

**The breakpoint at the id-read was never hit during the entire replay.** The
window never slides: the anchor stayed `(0,0)` and the camera never left section
0. So `MigrateMasks` never executes during the fixture.

**Therefore the bug was PRESENT in the recording build but never FIRED.** The 33
checkpoint hashes do not encode the mis-indexing, the fixture does **not** need
re-recording on account of this parcel, and it will not desync against the fix.
The dating argument established presence; only this run establishes non-firing.

## PROBE 2 — NOT ESTABLISHED. No difference observed, and the run is the reason.

Forced a slide on both carts by poking `Camera_X` to `$0C000000`, then compared
`Entity_Loaded_Masks` at the next `EntityWindow_Scan`. Both carts: **`0x90A0AADC`,
anchor `(1,1)`** — identical.

Per this document's own rule that is "no qualifying slide found", NOT a
refutation. Two concrete reasons, both found by running it:

1. **The capture point is wrong, and it is my error, not the spec's.** The next
   `Scan` is DOWNSTREAM of the post-slide entity re-load, which repopulates the
   masks and can absorb the very difference being measured. Pre-slide slot 0 was
   `$7F` and slot 1 `$01`; post-slide slot 1 is `$03` on both carts — a value
   that is neither input, i.e. rebuilt rather than migrated. **The correct capture
   is at `MigrateMasks`' RETURN (`step_out` from the id-read breakpoint), before
   anything else touches the masks.**
2. **The camera-poke drive is not reproducible run-to-run.** The same poke value
   slid the window on some runs and not others: `Camera_Update` pulls the poked
   value back (observed `$0C000000` → `$0BF00000`) between an arbitrary pause and
   the scan. **The poke must be applied AT the `EntityWindow_Scan` breakpoint**
   (`$0046A8`, same address on both carts, downstream of `Camera_Update`) — done
   that way it fired reliably on both carts.

**Probe 2 is OWED.** Re-run with the poke at `$0046A8` and the capture at
`MigrateMasks`' return.

## Verdict

The mechanism is **proven live and exactly as predicted**, including the old code
reading ROM-pointer bytes as section ids. The fixture question is **settled**: the
bug never fires there, so no re-recording is owed. The downstream mask-outcome
comparison is **not established** and is not claimed.

---

# PROBE 2 — RE-RUN AND ESTABLISHED (2026-08-06, second sitting)

Re-run per this note's own prescription after the first attempt's two executor
deviations. **Result: identical inputs, different outputs, at the same anchor —
and the OLD cart trips the engine's own duplicate tripwire while the NEW cart
runs clean.**

Carts by `memory_hash` over the full 423571 bytes, never the reload diagnostic
(whose `first16` is byte-identical for the two): OLD `0x159B152F`, NEW
`0x7E273B14`.

## The drive that works, and why the fixture cannot be it

Probe 3 established that the shipped stream never slides, so the fixture cannot
drive probe 2 at all. The drive is a **scripted camera walk with the poke applied
AT the `EntityWindow_Scan` breakpoint** (`$0046A8` — same address on both carts,
and downstream of `Camera_Update`, which is what stops the poke being pulled
back). Input stays LIVE with no buttons pressed, so the run is deterministic.

**A slide alone is not enough — the exhibit needs a populated section that
SURVIVES onto a MISREAD entry.** Entry 0 is `0 × stride` and is correct under
both strides, so:

* from the boot anchor `(0,0)` only section `(0,0)` is populated, and it is the
  one section that no rightward or downward slide keeps — **no slide from boot
  can exhibit anything**;
* a rightward slide moves the surviving `(1,0)` onto entry **0** — correct even
  under the bug;
* only a **leftward** slide moves `(1,0)` onto entry **1**, a misread
  destination.

So the schedule walks right to populate section 1, slides right to seat it on
entry 0, then slides left to move it to entry 1.

Schedule, anchored on `Logic_Tick` (value-anchored, not step-counted, so both
carts are at the same game state):

| at | poke `Camera_X` | effect |
|---|---|---|
| `Scan` break, `Logic_Tick == 1` | `$0900` | camera parks by section 1's rings (world x `$900`-`$9C0`, read out of the ROM ring list at `$00011F62`) |
| next `Scan` break | `$0A00` | anchor `(0,0)` → `(1,0)` — the rightward slide |
| next `Scan` break | `$0200` (far) or `$09FF` (near) | anchor `(1,0)` → `(0,0)` — **the leftward slide** |

`EntityWindow_Slide` is called at `Scan:897`, **before** the per-section
scan/despawn loop, so the masks are intact when `MigrateMasks` runs even when the
poke moves the camera far away.

## The control — and the rightward slide proves it holds

Both carts were byte-identical at every checkpoint up to the leftward slide:

| checkpoint | OLD | NEW |
|---|---|---|
| `Logic_Tick == 1`, masks | all zero | all zero |
| after the `$0900` frame | slot 0 `$7F`+obj`$01`, slot 1 `$3F`+obj`$01` | identical |
| after the rightward slide | anchor `(1,0)`, slot 0 `$3F`+obj`$01`, slot 1 `$F00F` | identical |

**The rightward slide migrating identically on both carts is itself a measured
confirmation of the direction model:** its only survivor lands on entry 0, which
the ×22 stride computes correctly, so no difference is possible there — and none
occurred. The control is therefore intact right up to the first slide that can
differ.

At the leftward slide's `MigrateMasks` entry (`$004B1C`, the same address on both
carts) the proc's entire input was byte-identical, `a4` included:

* `Entity_Scan_State` (104 B) — identical
* `Entity_Mask_Scratch` (132 B) — identical
* `Entity_Window_Active` = `$0F` — identical
* `a4` = `$FFFFAD46` (= `Entity_Mask_Scratch`) — identical
* in fact the **entire register file** matched, d0-d7 and a0-a7

## Step 0 — the content precondition, SATISFIED

New anchor `(0,0)`; grid_w = 3. Live entry ids and the snapshot:

| new entry k | id `E[k]` | ×22 read lands on | snapshot | block non-zero? |
|---|---|---|---|---|
| 0 | `$00` | `$00` (correct — offset 0) | not present | — |
| 1 | `$01` | **`$1F`** (offset 40, not 44) | j=0 | **YES** — `$3F` rings + `$01` obj |
| 2 | `$03` | `$1F` (offset 62, not 70) | not present | — |
| 3 | `$04` | `$1F` (offset 84, not 96) | j=2 | no (all zero) |

Snapshot ids `$01 $02 $04 $05`. **Qualifying entry: k = 1** — non-void, present in
the snapshot, and its block is non-zero. `$1F` is a `ess_rom_*_ptr` byte and is
absent from the snapshot id set, so the OLD match must fail. (This is the same
`$1F` probe 1 saw in `d2`.)

## Steps 1-2 — the output at `MigrateMasks`' RETURN

Captured by `step_out` from inside the proc, before anything else touches the
masks:

| slot | OLD (×22) | NEW (×26) |
|---|---|---|
| 0 | all zero | all zero |
| **1** | **all zero — the mask is LOST** | **`$3F` rings + `$01` obj — MIGRATED** |
| 2 | all zero | all zero |
| 3 | all zero | all zero |

**Exactly one slot differs, and it is slot 1** — the predicted count for a
leftward slide in which one survivor carries a non-zero mask. (Entry 3's survivor
qualifies structurally but its snapshot block is empty, so it cannot show.)

This is **direction (a)**: the compare-clear zeroed entry 1 because its section
changed, the identity match then failed on the garbage `$1F`, and six already-
spawned rings plus one object are now marked unloaded.

## Corroboration — the OLD cart trips the engine's OWN duplicate tripwire

Re-run with the leftward poke at `$09FF` instead of `$0200`, so the rings stay
inside the load band and are re-scanned on the following frame.

**OLD cart: `ErrorHandlerBlob`.**

```
Assertion failed:
> assert.w d5,ne,d4
Got: 0100
Offset: 0047BA  engine.objects.entity_window.raise
```

`$47BA` is inside `EntityWindow_TrySpawnRing` (`$4728`-`$4822`); the assertion is
the DEBUG no-dup scan whose source comment reads *"always fails: duplicate
(sec,idx)"*. `Got: 0100` is the entry key word — **section `$01`, list index
`$00`** — i.e. exactly the section whose mask was lost, spawning a ring it
already had in the buffer.

**NEW cart, identical schedule: no assertion, gameplay continues normally.**

* `assets/2026-08-06-migmask-probe2-old-dup-assert.png`
* `assets/2026-08-06-migmask-probe2-new-clean.png`

So the duplicate-spawn consequence is not merely inferred from the mask bytes —
the engine's own tripwire fires on the buggy cart and stays silent on the fixed
one.

## Verdict

**Probe 2 is ESTABLISHED, in both the mask outcome and the downstream symptom,
with the control measured rather than assumed.** Together with probe 1 (mechanism
live, 8/8 exact) and probe 3 (the shipped fixture never reaches the code), the
A/B is complete. Nothing about the change is now unmeasured.

## Methodology trap found in this sitting — worth more than the probe

**Resuming while the PC sits ON a breakpoint address re-triggers that breakpoint
without executing anything.** The first hour of this sitting drove a camera walk
that never happened: `Logic_Tick` stayed at **1** across a dozen resume/wait
cycles while every poke landed in RAM and was silently discarded, and every read
returned a plausible, unchanging state. It looks exactly like "the game is
running and nothing is happening", which is indistinguishable from a real
negative result — and a negative result is precisely what an inverted-bar A/B
must never accept uncritically.

The tell is a **frame or tick counter that does not advance**; the fix is
`step 1` then `resume`. It also fires at `EntityPoint` right after `reload_rom`.
The overseer hit the same trap in the first sitting and recorded it for probe 1
("an identical register file on two consecutive breaks is the tell"); it cost a
second sitting because it was recorded as a probe-1 detail rather than as a
standing rule for this instrument.

**RULE: every emulator A/B reads a tick/frame counter at two consecutive
anchors and proves it advanced, before reading anything else as evidence.**
