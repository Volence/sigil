# Lane D packet — the slide fixture

Branches: sigil `lane-d` (1 commit), aeon `lane-d` (1 commit), both from master.
**Not merged, not pushed.**

A/B: `docs/superpowers/notes/2026-08-07-slide-fixture-ab.md`.

## MERGE-ORDER FLAG — read before queueing

**Lanes A and D both append a chain-52 entry.** They are independent
byte-changing parcels frozen from the same chain-51 base, so whichever merges
second must re-capture and re-freeze on top of the first; its `ab` ref carries
over unchanged. This is a normal consequence of two parallel byte-changing lanes,
not a defect in either, but it is the one thing the merge cannot do implicitly.

Lane D's aeon change also touches no file lane A touches, so the only conflict is
the golden blobs and `provenance.toml`.

## The acceptance bar, and it is met

> "the new fixture must be proven to FAIL on a cart built with the OLD (x22)
> stride. A regression test that passes on the broken build tests nothing."

**Fixed cart** (`memory_hash` `0x8C420EFD`): `Replay_Done` = `$FF`, `Replay_Ptr`
at the end-of-stream record, **zero desyncs**, clean log.

**Broken cart** (`0x0EB095EB`, `#sizeof(EntityScanState)` reverted to `#22`,
same 423831 bytes so every address matches): the run **raises**.

```
Assertion failed:
> assert.w d5,ne,d4
Got: 0100
Offset: 0047BA  engine.objects.entity_window.raise
```

`$47BA` is the DEBUG no-dup scan inside `EntityWindow_TrySpawnRing`; `Got: 0100`
is section `$01`, list index `$00` — the section whose rings the stream collected
and whose mask the ×22 stride loses. `Logic_Tick` 391, mid-leftward-run.
Screenshot in the aeon commit.

The DRAFT of this stream was validated with the bytes **injected into RAM**
(`Replay_Record_Buf`, unused during playback) so it could be tested before being
embedded; the FINAL validation above reads the fixture out of each cart's own
ROM, which is the real shipping path.

## The hole this closes, restated as measurement

Two independent measurements, both from this week's A/Bs, not from reading code:

* migmask probe 3: the shipped 2059-tick fixture never leaves section 0 — anchor
  stays `(0,0)`, `MigrateMasks` never executes, `Entity_Loaded_Masks` hashes
  `0x25913C7E` identically on the buggy and fixed carts.
* mulw-parallax probe 4: `Section_GetSecPtrXY`'s stride, armed for the whole run
  on both carts, **never fires**.

So `EntityWindow_Slide`, `MigrateMasks`, `PopulateSectionRings` and
`InitSection`'s compare-clear had no automated behavioural coverage at all.

## The recording

Standard anchored recipe, `Input_Source` = `INPUT_RECORD` poked at
`GameState_OJZScroll_Init`, `Replay_Record_Idx` never rewound (aeon `b014865`).
Input driven with deterministic `press(buttons, frames)`:

| ticks | held | effect |
|---|---|---|
| 0-270 | RIGHT | collect section 1's rings (`Ring_Count` `$0E15`); anchor `(0,0)` → `(1,0)` — **RIGHTWARD** |
| 271-670 | LEFT | anchor `(1,0)` → `(0,0)` — **LEFTWARD**, the exhibit direction |
| 671-1052 | idle, B, DOWN | settle; the first B tap is a no-op, recorded as-is |
| 1053-1054 | B | enters debug free flight |
| 1055-1214 | DOWN | anchor `(0,0)` → `(0,1)` — **DOWNWARD** |
| 1215-1394 | UP | anchor `(0,1)` → `(0,0)` — **UPWARD** |
| 1395-1495 | B, idle | leaves free flight, settles |

The anchor was read after every step. Read out: 1496 ticks, 24 checkpoints, first
at `Logic_Tick` 2. Packed by `tools/replay_pack.py` → 240 bytes; the packer's own
SOCD/escape validation accepted it, and the decoded run lengths match the drive
exactly.

## Gates — all own-run

* **Byte bar, seven targets: CHANGED, and every delta named.** +198 bytes in both
  sonic4 shapes; `demo` and `demo.debug` **unchanged**, which is an independent
  confirmation the delta is where it is claimed. The load-bearing proof: outside
  the vector table and header, the **lowest differing byte in `s4.bin` is
  `0x5C8B2`, which is `Replay_OJZ_Slide_Fixture`'s own address** — so every
  gameplay byte before it is identical, and `Replay_OJZ_Fixture` itself is still
  at `0x5C778`. Above it: the fixture, then `ErrorHandlerBlob` shifted `+0xB0`,
  then `EndOfRom` at `0x5DA18`. The 62 sub-`0x200` runs are the fault vectors
  that point into the moved island plus the checksum and ROM-end pointer.
* **Chain 52 frozen** with the A/B ref; `refreeze --check: OK (tip
  slide-fixture, chain len 52)`.
* **Ripple, all sites checked:** `repin` rewrote `pins.rs` (every fault-vector
  pin `+0xF0`); the hand-typed `ASSEMBLED_LEN` / `DEBUG_ASSEMBLED_LEN` baseline
  in `tests/repin_pins.rs` was updated and annotated to this parcel — it FAILED
  first and was caught by the strict run, not by inspection. `engine.inc` and
  `mixed_dac_rom.rs` are deleted from both repos (empty sites); `repin.toml`
  untouched (no region added).
* **Full strict**, foreground, streams separated: **3511 passed / 0 failed /
  4 ignored = 3515**, and the branch's `#[test]` total counted this session is
  **3515**. Closes exactly; equals master (no new Rust test).
* **Warn tiers**: unchanged; gate green.

## Per-pass findings

**Step 3 (retrospect)** — the fixture format has a `core_hash` header field
documented as "build identity — a stale replay fails loudly", and **the engine
never reads it**: playback starts at `Replay_Ptr = fixture + REPLAY_HEADER_LEN`,
so the whole header is skipped at runtime. Only `replay_pack.py`'s decoder looks
at it. The new fixture carries the same value as the shipped one because nothing
distinguishes them. That is a promise in a doc comment with no mechanism behind
it — the same class as Lane B's lying `ensure` comment, in a different file.

**Step 5 (engine optimize)** — none.

**Neither bucket — the headline** — **injecting the stream into RAM is what made
this lane tractable, and it should be the standard technique.** The fixture is
reached through `Replay_Ptr`, an ordinary long, and `Replay_Record_Buf` is 8 KB of
DEBUG RAM that playback does not touch. So a candidate stream can be validated
against *any* cart — including a deliberately-broken one — with no rebuild, no
golden churn and no chain entry, and only the stream that survives that gets
embedded. Every previous fixture change had to pay a ROM rebuild to be testable at
all, which makes "record it, embed it, then find out" the path of least
resistance; this inverts that.

## Honest residue

* The failure on the broken cart is carried by the **DEBUG duplicate assert**,
  not by a checkpoint hash mismatch — measured: the checkpoint at tick 386
  matched and the run never reached the one at 450. Whether the hashes would
  diverge independently is untested. Ledgered with its kill condition.
* **All four directions are covered, and the draft of this packet was wrong to
  say otherwise — twice.** (a) The act grid is **3 × 3**, not 3 × 1: only
  `grid_w` had been checked and `grid_h` was inferred from it. (b) The scene arms
  `CHEAT_DEBUG_FLY` itself under `if DEBUG == 1`, so vertical movement is
  reachable from **input alone** — no temporary sections, no poke — and replays
  deterministically. A third error rode along: "right/down slides cannot exhibit
  anything" was the migmask A/B's boot-state argument over-generalised into a
  direction claim; that note's own table puts a downward slide's survivors at
  entries 0 **and 1**, and entry 1 is misread. All three were unverified
  inferences stated as measurements.
* The residual gap is narrower and sharper: the vertical legs run down an
  **empty** column, so they exercise the slide path without carrying a mask that
  could be lost. Ledgered with that kill condition.
* Nothing yet RUNS either fixture automatically — both are played by the standard
  oracle recipe by hand. Making the slide fixture a CI-driven gate is a separate
  piece of work and is not claimed here.
