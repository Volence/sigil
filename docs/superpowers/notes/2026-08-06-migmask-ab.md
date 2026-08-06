# migmask A/B — the specification (porter-written, overseer-executed)

`EntityWindow_MigrateMasks` strided `Entity_Scan_State` by 22 where the struct is
26. This parcel makes the stride derive from the struct
(`mul_const.w d0, #sizeof(EntityScanState), d1`). **The parcel is deliberately
behaviour-CHANGING**, so the bar is inverted from the recent optimization
parcels: a probe that reads the same on both ROMs has proven nothing. Every
probe below is designed to DIFFER, and each says what "no difference" would mean.

The porter did not run the emulator. Everything here is derived statically from
the two ROM images and their listings, and every address was read out of a
hexdump this session.

## The two carts

| | OLD (chain 50) | NEW (chain 51) |
|---|---|---|
| plain | `crates/sigil-harness/golden/s4.bin` at master `f4d87aae` | the branch build |
| debug | `crates/sigil-harness/golden/s4.debug.bin` at master `f4d87aae` | the branch build |

Identify the loaded cart by **ROM hash**, never by the reload diagnostic. CRC32 +
size for both chains are in the packet.

## Hygiene: this parcel changes cycle counts

The replaced chain is 7 instructions / 44 cycles; the new lowering is 6
instructions / 38 cycles, per iteration, ×4 iterations per slide. A
`pause`-anchored A/B is therefore **invalid** (the two runs are in different
phase after the first slide). Anchor every probe at a **PC breakpoint**, and
prefer anchors whose address is the same in both ROMs.

`EntityWindow_MigrateMasks` is such an anchor: its entry address is **unchanged**
by this parcel (the two bytes at the entry are byte-identical across the diff).
`EntityWindow_Slide` is **not** — it moved by 2 — so do not anchor on it.

| symbol | plain OLD | plain NEW | debug OLD | debug NEW |
|---|---|---|---|---|
| `EntityWindow_MigrateMasks` (entry) | `$003E1A` | `$003E1A` | `$004B1C` | `$004B1C` |
| the `move.b (a0,d0.w),d2` id read | `$003E30` | `$003E2E` | `$004B32` | `$004B30` |
| `EntityWindow_Slide` (entry) | `$003E70` | `$003E6E` | `$004B72` | `$004B70` |

RAM is untouched by the parcel; the cells below are the same in OLD and NEW.

| cell | plain | debug | bytes |
|---|---|---|---|
| `Entity_Scan_State` | `$FFFFAC2E` | `$FFFFAC52` | 104 (4 × 26) |
| `Entity_Window_Active` | `$FFFFAC9A` | `$FFFFACBE` | 1 |
| `Entity_Window_Anchor` | `$FFFFAC9C` | `$FFFFACC0` | 2 |
| `Entity_Loaded_Masks` | `$FFFFACA2` | `$FFFFACC6` | 128 (4 × 32) |
| `Entity_Mask_Scratch` | `$FFFFAD22` | `$FFFFAD46` | 132 (4 ids + 128 mask) |
| `Camera_X` / `Camera_Y` | `$FFFFA152` / `$FFFFA156` | same | 2 each |
| `Debug_Scene_Freeze` | — | `$FFFF8A22` | 1 |
| `Input_Source` / `Replay_Ptr` / `Replay_Done` | `$FFFF803A` / `$FFFF8040` / `$FFFF803C` | same | 1 / 4 / 1 |

The ROM boots straight into the level: `Game_Entry = GameState_OJZScroll_Init`
(`games/sonic4/config/game.emp:24`). Scrolling far enough on either axis crosses
a `SECTION_SIZE` = `$800` px boundary and calls `EntityWindow_Slide`, which calls
`EntityWindow_MigrateMasks` once per slide.

## Probe 1 — the mechanism. Guaranteed to differ; run it first

**Anchor:** breakpoint at the id-read instruction (plain `$003E30` OLD /
`$003E2E` NEW; debug `$004B32` / `$004B30`). It is hit exactly 4 times per
slide, once per window entry, before any `SEC_VOID` skip — so all four values
are always observed.

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

**If this does not differ, stop**: either the wrong cart is loaded or the
breakpoint is on the wrong instruction. Nothing downstream is interpretable.

## Probe 2 — the consequence, controlled. The parcel's primary evidence

**Anchor:** breakpoint at `EntityWindow_MigrateMasks` entry (`$003E1A` plain /
`$004B1C` debug — the SAME address in both ROMs, which is why this anchor was
chosen).

**Step 1, the control (non-vacuity).** At the anchor, on both carts, dump:

* `Entity_Scan_State` (104 B), `Entity_Mask_Scratch` (132 B),
  `Entity_Loaded_Masks` (128 B), `Entity_Window_Active` (1 B),
  `Entity_Window_Anchor` (2 B).

These are the proc's entire input. **They must be byte-identical between OLD and
NEW.** If they are not, the two runs diverged before the proc and the output
comparison is void — back up to an earlier slide, or use `Debug_Scene_Freeze`
plus a camera poke to drive a scripted slide from a common state.

**Step 2, the output.** `step_out`, then dump `Entity_Loaded_Masks` (128 B) on
both carts. **Expect a difference.** Identical inputs and different outputs at
the same anchor isolate the difference to this proc — that is the whole design
of the probe.

**Step 3, name which direction fired.** Compare each 32-byte slot `k`
(`Entity_Loaded_Masks + 32k`) against the snapshot block
(`Entity_Mask_Scratch + 4 + 32j`) and against the ids
(`Entity_Mask_Scratch + j`, `Entity_Scan_State + 26k + $12`):

* **(a) failed migration → duplicate spawns.** Entry `k`'s section id appears in
  the snapshot at some `j`, and NEW slot `k` equals snapshot block `j` while OLD
  slot `k` is all zeroes. The zeroes are `EntityWindow_InitSection`'s
  compare-clear (`engine/objects/entity_window.emp:637-647`), which runs from
  `BuildEntries` BEFORE `MigrateMasks`; the OLD identity match then failed on a
  garbage id, so every already-collected entity in that surviving section is
  re-treated as unloaded and spawns a second time.
* **(b) false match → foreign mask.** Entry `k` kept its section (its id is
  unchanged from the previous slide) and OLD slot `k` equals snapshot block `j`
  for some `j != k`, while NEW leaves it correct. A garbage byte chance-matched
  an old snapshot id and a foreign 32-byte mask was copied over a correct one.

Which direction fires depends on the ids in play, so **run several consecutive
slides** and record which direction each one exhibits. Both directions are
claimed by the fix; the A/B should show at least one instance of each.

**Corroboration for (a):** a few frames after a slide that exhibited (a),
census the object slots on both carts (`emulator_object_list`). OLD should carry
duplicates of at least one entity from the surviving section that NEW does not.
This is a symptom observation, not the proof — probe 2 step 2 is the proof.

## Probe 3 — the shipped replay fixture, and the parcel's chief follow-up

`games/sonic4/data/replays/ojz_fixture.bin` is a 2059-tick recorded input stream
with 33 curated state checkpoints; in DEBUG a checkpoint mismatch raises
`REPLAY DESYNC` (`engine/system/replay.emp`). The curated hash covers SST
(object) state.

**The fixture was last re-recorded at aeon `806a0de` (2026-08-05), which is AFTER
`4ad9f9b` (2026-07-31) introduced the stride bug.** Its checkpoint hashes
therefore encode the buggy object state. If the stream crosses a section boundary
with entities collected in a surviving section, the fixed ROM must desync.

**Run:** DEBUG cart, `Input_Source = 1` (`INPUT_PLAYBACK`),
`Replay_Ptr = Replay_OJZ_Fixture + 20` (`REPLAY_HEADER_LEN`; `Replay_OJZ_Fixture`
is `$05E568` in the NEW debug shape — re-read it from the OLD listing for the OLD
cart, the fixture sits after the code that moved).

* **OLD must first reach `Replay_Done = $FF` with zero desyncs.** This is the
  probe's non-vacuity control: it is what aeon `b014865` recorded. If OLD also
  desyncs, the fixture is stale for an unrelated reason and probe 3 proves
  nothing — report that and drop the probe.
* **NEW desyncing is the exhibit**: the shipped, recorded behaviour of the game
  changed, at a named tick, in the object state the hash covers.
* **NEW completing clean is also a real result** — it bounds the fix's blast
  radius: this stream never slides with a survivor carrying collected entities.

**Either way this is a follow-up, not a regression to fix in this parcel.** A
desync means the fixture is a recording of the bug and must be re-recorded on the
fixed ROM before it can serve as a regression net again. Re-recording needs the
emulator and is the overseer's call; the porter has ledgered it.

## What would refute the parcel

* Probe 2 step 1 shows differing inputs at the shared anchor on every slide
  tried — the probe cannot isolate the proc; redesign before claiming anything.
* Probe 2 step 2 shows identical outputs across many slides — then the corpus
  never reaches a state where the stride matters, and the "live bug" framing is
  overstated even though the read addresses (probe 1) are still wrong.
* Any probe run without first confirming, by ROM hash, which cart is loaded.
