# slide-fixture A/B — recording, and the proof it fails on the broken build

The parcel adds a SECOND recorded input fixture,
`games/sonic4/data/replays/ojz_slide_fixture.bin`, and embeds it beside the
standing one. It is ROM-size-changing (+0xF0 assembled), so it re-freezes the goldens,
and this note is its evidence.

## Why it exists — the hole, measured not assumed

The migmask A/B established, on BOTH carts, that the shipped
`ojz_fixture.bin` (2059 ticks, 33 checkpoints) **never leaves section 0**: the
entity-window anchor stays `(0,0)`, `EntityWindow_MigrateMasks` never executes,
and `Entity_Loaded_Masks` hashes `0x25913C7E` identically on a cart with the bug
and a cart without it.

The mulw-parallax A/B then measured a second face of the same hole: a breakpoint
on `Section_GetSecPtrXY`'s stride was armed for the whole 2059-tick run on both
carts and **never fired**.

So the corpus's only recorded behavioural fixture had ZERO coverage of
`EntityWindow_Slide`, `EntityWindow_MigrateMasks`, `PopulateSectionRings` and
`EntityWindow_InitSection`'s compare-clear. That is a large part of why a live
mis-index survived nine days and two byte-changing parcels.

## The recording

Cart: the DEBUG build carrying the first draft of this fixture, `memory_hash`
`0x17756075` / 423767 (verified, never the reload diagnostic). Recording on a
cart that already embeds a fixture is safe because `Replay_Hash` is address-free
by construction — that is the property the hash map exists for.

Drive: the standard anchored recipe with `Input_Source` = `INPUT_RECORD` (2)
poked at `GameState_OJZScroll_Init` (`$05E00E`) — the same anchor playback uses,
per aeon `b014865`'s procedure note ("never rewind `Replay_Record_Idx`
mid-session — ring index N must mean 'Nth tick after the init anchor' on both
record and playback"). `Replay_Record_Idx` was never touched.

Input schedule, driven with deterministic `press(buttons, frames)` calls:

| ticks | held | effect |
|---|---|---|
| 0-270 | RIGHT (`$08`) | run right past section 1's rings (world x `$900`-`$9C0`), collecting them; `Camera_X` reaches `$1150`, anchor `(0,0)` → **`(1,0)` — RIGHTWARD** |
| 271-670 | LEFT (`$04`) | run back; `Camera_X` returns to `$0000`, anchor `(1,0)` → **`(0,0)` — LEFTWARD** |
| 671-750 | none (`$00`) | settle on the ground |
| 751-752 | B (`$10`) | a NO-OP toggle attempt (see below) |
| 753-1052 | DOWN (`$02`) | crouch — still not flying |
| 1053-1054 | B (`$10`) | **enters debug free flight** |
| 1055-1214 | DOWN (`$02`) | fly down at 16 px/frame; anchor `(0,0)` → **`(0,1)` — DOWNWARD** |
| 1215-1394 | UP (`$01`) | fly back up; anchor `(0,1)` → **`(0,0)` — UPWARD** |
| 1395-1396 | B (`$10`) | leaves free flight |
| 1397-1495 | none (`$00`) | settle |

**All four directions, and the anchor was read after every step** — `(0,0)` →
`(1,0)` → `(0,0)` → `(0,1)` → `(0,0)`.

**The doubled B tap is deliberate and is recorded as-is.** The first tap after a
run does not toggle; the second does. Whatever the cause, playback is
deterministic by construction, so a no-op tap in the stream replays as a no-op
tap. It is left in rather than trimmed, because trimming it would mean shipping a
stream that was never actually executed.

A word read at `Ring_Count` returned `$0E15` after the first run — that is
`Ring_Count = $0E` and its neighbour `Ring_HighWater = $15`, since `Ring_Count`
is a `u8`. Note `Ring_Count` counts entries in the spawn `Ring_Buffer`, not rings
collected; the collected counter is `Ring_Counter`. What actually establishes the
content precondition is not this read but probe 2's outcome: the ×22 cart's
duplicate tripwire fires naming section `$01` list index `$00`, which is only
reachable if section 1's rings were spawned and marked loaded.

Vertical movement uses **debug free flight**, which needs no level-data change
and no poke: `games/sonic4/test/ojz_scroll_test.emp:77` arms
`CHEAT_DEBUG_FLY` in `Cheat_Flags` under `if DEBUG == 1`, so on the DEBUG cart
the B toggle in `Player_Main` is live and is **pure input** — armed identically
on record and playback. Verified at runtime: `Cheat_Flags` read `$01`, and flight
moves 16 px/frame against a stationary 0 px/frame on the ground.

Read out at the end: `Replay_Record_Idx` = `$05D8` = **1496 ticks**,
`Replay_Check_Idx` = `$18` = **24 checkpoints**, first checkpoint at
`Logic_Tick` 2. Packed with `tools/replay_pack.py` → **240 bytes**, which the
packer's own SOCD/escape validation accepted (no `$FF`, no U+D, no L+R; the idle
stretches record as `$00`). The decoded run lengths match the drive
exactly: `$08`×271, `$04`×400, `$00`×80, `$10`×2, `$02`×300, `$10`×2, `$02`×160,
`$01`×180, `$10`×2, `$00`×99.

## Probe 1 — the fixed cart must complete CLEAN

Validated twice, on two different streams — the draft by RAM injection, this one
from ROM.

**Draft validation, stream injected into RAM** — written to
`Replay_Record_Buf` (`$FFB408`, 8 KB of DEBUG RAM playback never touches),
`Replay_Ptr` = `$00FFB41C` (= buffer + `REPLAY_HEADER_LEN`). This is how a
candidate stream is tested BEFORE it is embedded: no rebuild, no golden churn, no
chain entry, and only the stream that survives gets committed.

**Final validation, played FROM ROM — the real shipping path.** Cart
`0x8C420EFD` / 423831, `Replay_Ptr` = `Replay_OJZ_Slide_Fixture` (`$0005E6A2`) +
`REPLAY_HEADER_LEN` = `$0005E6B6`, `Input_Source` = `INPUT_PLAYBACK`.

**Result: `Replay_Done` = `$FF`, `Input_Source` reverted to 0, `Replay_Ptr`
ended at `$0005E788` (= fixture + `$E6`, the end-of-stream record), zero
desyncs, clean system log.** All 24 checkpoint hashes matched.

This is the non-vacuity control: a fixture that cannot complete on the build that
recorded it proves nothing about any other build.

## Probe 2 — the OLD (×22) cart must FAIL. THE ACCEPTANCE BAR

Broken cart built deliberately from the SAME tree as the fixed one, differing in
exactly one instruction: `EntityWindow_MigrateMasks`' stride reverted from
`#sizeof(EntityScanState)` to `#22`. DEBUG shape, `memory_hash` `0x0EB095EB` /
**423831 — byte-for-byte the same size as the fixed cart**, so the embedded
fixture sits at the same address in both and every address in this note holds for
each.

Identical drive, and both carts read the fixture out of their OWN ROM.

**Result: the run raises.**

```
Assertion failed:
> assert.w d5,ne,d4
Got: 0100
Offset: 0047BA  engine.objects.entity_window.raise
```

`$47BA` is inside `EntityWindow_TrySpawnRing`; the assertion is the DEBUG no-dup
scan whose source comment reads *"always fails: duplicate (sec,idx)"*, and
`Got: 0100` is the entry key word — **section `$01`, list index `$00`**, i.e.
exactly the section whose rings were collected and whose mask the ×22 stride
loses. It is the same signature the migmask A/B forced with camera pokes, reached
here by the recorded stream alone.

`Logic_Tick` at the raise = `$187` = **391** — mid-LEFT-run, a few ticks after
the leftward slide, and identical to the tick the draft fixture raised at.
Screenshot: `assets/2026-08-07-slide-fixture-broken22-fails.png`.

**Bar met: the fixture fails on a cart built with the old stride, and completes
clean on the fixed one.**

## What carried the failure — stated precisely, because it is not what one assumes

**The DEBUG duplicate assert caught it, NOT a checkpoint hash mismatch.** The
measurement: the assert fired at `Logic_Tick` 391. Checkpoints sit at ring
indices 0, 64, … i.e. `Logic_Tick` 2, 66, …, 386, 450. The checkpoint at 386
**matched** (no desync fired), and the run never reached 450. So this stream's
hash net is not proven to catch the defect on its own; the engine's own tripwire
is what fires, 59 ticks before the next comparison.

Three consequences, none of them glossed:

* The net is **DEBUG-only**, exactly like the existing fixture: a release build
  carries neither the assert nor the checkpoint compare (`DEBUG == 0` steps over
  the payload without comparing).
* The failure it produces is BETTER than a desync would be — it names the defect
  and the offending (section, index) pair, instead of reporting that two numbers
  differ.
* Whether the checkpoint hashes would independently diverge is **untested**. It
  is likely — the migmask A/B established the mask is lost, so the rings respawn,
  and `Dynamic_Live_Count` / `Dynamic_Free_SP` are inside `Replay_Hash` — but
  likely is not measured. Establishing it needs a third cart with the duplicate
  assert removed, or a stream whose next checkpoint lands between the slide and
  the re-scan. Ledgered.

## Coverage this stream adds — and a CORRECTION to two claims in this note's first draft

**Adds all four directions:** rightward, **leftward**, downward and upward
crossings, plus `PopulateSectionRings` and `InitSection`'s compare-clear along
the way. The anchor was read after every step and moved
`(0,0)` → `(1,0)` → `(0,0)` → `(0,1)` → `(0,0)`.

The first draft of this note claimed vertical coverage was out of reach. **Both
halves of that claim were wrong, and the errors are worth naming because each was
an unverified inference stated as a measurement.**

1. *"the OJZ act's grid is 3 sections wide, so up/down needs 2048 px of vertical
   geometry the test scene's spawn does not obviously provide."* The grid is
   **3 × 3** — `GRID_W = 3` **and `GRID_H = 3`**, pinned by
   `act_descriptor.emp`'s own `ensure(GRID_W * GRID_H == 9)` over a `[Sec; 9]`
   table. Only `grid_w` was checked; `grid_h` was assumed from it. And the scene
   arms `CHEAT_DEBUG_FLY` itself (`ojz_scroll_test.emp:77`, `if DEBUG == 1`), so
   free flight is available from **input alone** — no level-data change, no poke,
   nothing temporary.

2. *"the two delivered are the two that matter — a rightward or downward slide
   moves its only survivor onto entry 0 and cannot exhibit anything."* That is the
   migmask A/B's **boot-state** argument (from the boot anchor only section
   `(0,0)` is populated) over-generalised into a claim about DIRECTIONS. The
   direction model in that same note says otherwise: a DOWNWARD slide's survivors
   land at entries **0 and 1**, and entry 1 IS a destination the ×22 stride
   misreads. So downward can exhibit, given a populated section that survives onto
   it — which makes vertical coverage more valuable than the draft claimed, not
   less.

**Still not covered:** a vertical crossing with a POPULATED section surviving onto
entry 1 — this stream's vertical legs run down an empty column, so they exercise
the slide path without being able to exhibit the defect. That is a smaller and
sharper gap than "no vertical coverage", and it is what remains ledgered.

## The byte cost

`+0xF0` = **240 assembled bytes in both shapes — the fixture exactly, with no
alignment padding** (the base `0x5C8B2` is even and the length 240 is even). The
golden BLOBS grow by more: `s4`/`config_b` +262, `s4.debug`/`config_a` +260,
`lean` +240. The extra bytes are the compressed `deb2` symbol appendix gaining
the new label — measured directly, `s4.bin`'s appendix goes 27831 → 27853 — not
alignment, and `lean` shows +240 exactly because it carries no appendix. `replay_fixture.emp`'s own
header states the consequence and it holds here: the fixture is placed after all
gameplay content and before the fault-handler island, so **zero gameplay
addresses shift** — only the island and `EndOfRom` move, and `repin` re-pins
them. The recording cart and the fixture cart therefore agree on every address
the curated hash covers, which is why a stream recorded on the chain-51 cart
plays back on the chain-52 cart at all.
