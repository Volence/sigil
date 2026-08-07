# mulw-parallax A/B — the specification

Two commits, one parcel, because they interact:

* sigil `mul_lower`: a single doubling in a WORD chain is `add.w d,d` (4 cycles)
  instead of `lsl.w #1,d` (8). Same two bytes, cycles-only.
* aeon `engine/level/parallax.emp`: Step 4a's shadow-view entry stride and copy
  derive from `sizeof(band_entry)` instead of restating 10 twice.

They are one parcel because the parallax chain is itself one of the four
lowerings the sigil commit reaches: landing them apart would move the same
instruction's bytes twice, and would make the aeon commit alone read as a
2-cycle regression rather than the 2-cycle win it is.

## THE BAR IS INVERTED THE OTHER WAY FROM `migmask` — read this first

`migmask` fixed a LIVE mis-index, so its bar was "a probe that reads the same on
both ROMs has proven nothing". **This parcel is the opposite case and needs the
opposite bar.** `band_entry` is 10 bytes today and the hand-rolled `x10` chain
computed `x10` correctly; every one of the four sigil deltas re-encodes an
identical doubling. **The parcel is behaviour-NEUTRAL by construction, and any
observable difference REFUTES it.**

So the inversion applies to a different axis:

* **What MUST DIFFER:** the *instruction encodings* at all five changed
  addresses. That is what identifies the two carts and proves the change is live
  at EVERY site rather than at the one site the author happened to look at.
* **What MUST MATCH:** observable state.
* **What must be PROVEN, not assumed:** that the changed code EXECUTED. A
  matching probe over code that never ran is the failure mode this class of
  parcel is uniquely exposed to — an all-green A/B that measured nothing looks
  exactly like a correct one. Every site carries its own hit proof.

The one deliberate register difference is named up front so it is not read as a
defect: at the parallax site `d5` ends the chain holding the band index on NEW
and twice the band index on OLD. `d5` is written exactly once in the whole of
`Parallax_Step4_Fill` (`parallax.emp:606`, re-counted this session) and is
declared in the proc's `clobbers(d0-d7/a0-a6)`, so no consumer may read it.

## Hygiene: this parcel changes cycle counts

Every delta is cheaper, so the two carts drift in phase from the first changed
site onward. A `pause`-anchored comparison is therefore **invalid**. Every probe
below anchors on a PC breakpoint, and every anchor address is the SAME on both
carts (nothing was re-placed — see the sizes below).

**Trap, standing rule for this instrument:** resuming while the PC sits ON a
breakpoint re-triggers it without executing. Read `Logic_Tick` (`$FFFF8004`) at
two consecutive anchors and prove it ADVANCED before reading anything as
evidence. Use `step 1` then `resume`.

## The two carts

Run every probe on the DEBUG shape (probe 4 needs it; the rest ride along).

| | OLD (chain 51) | NEW (this branch) |
|---|---|---|
| debug | `crates/sigil-harness/golden/s4.debug.bin` | aeon `.worktrees/lane-a/s4.debug.bin` |
| CRC32 / size | `7e273b14` / 423571 | `da2e3646` / 423571 |

Identify the loaded cart by `memory_hash` over the full 423571 bytes, **never by
the reload diagnostic** — `first16` is byte-identical for the two.

## The complete byte delta — five sites, every shape, nothing else

Re-derived this session by differing the built ROMs against the chain-51 golden
blobs. All seven targets carry **exactly the same seven runs**: the header
checksum, four 2-byte sites, and the parallax site — which splits into TWO runs
(5 bytes then 1) because `D645` collides in the middle of the replaced chain. **Every ROM size and every golden
`anchor_end` is unmoved**, so nothing is re-placed and every symbol address in
this note is the same on both carts.

DEBUG-shape addresses (68k address = file offset):

| # | site | addr | OLD | NEW |
|---|---|---|---|---|
| 1 | `EntityWindow_MigrateMasks.new_loop`+4 | `$004B22` | `E348` `lsl.w #1,d0` | `D040` `add.w d0,d0` |
| 2 | `EntityWindow_MigrateMasks.new_loop`+$C | `$004B2A` | `E348` | `D040` |
| 3 | `TileCache_DecompressBlock.rr_ok`+$4A | `$0054C4` | `E34B` `lsl.w #1,d3` | `D643` `add.w d3,d3` |
| 4 | `Section_GetSecPtrXY`+$24 | `$0066F8` | `E348` | `D040` |
| 5 | `Parallax_Step4_Fill.copy_band`+2 | `$006F24`..`$006F2B` | `E74B 3A02 DA45 D645` | `3A03 E54B D645 D643` |

Site 5 decodes as OLD `lsl.w #3,d3 / move.w d2,d5 / add.w d5,d5 / add.w d5,d3`
(= `d3 = 8·d2 + 2·d2`) against NEW
`move.w d3,d5 / lsl.w #2,d3 / add.w d5,d3 / add.w d3,d3`
(= `d3 = ((4·d2)+d2)·2`). Both are `d2 × 10 mod 2^16`; OLD costs 24 cycles, NEW
22 (site 5 is where the sigil commit's `add.w d3,d3` tail lands).

Anchors, all identical on both carts: `EntityWindow_Scan` `$0046A8`,
`EntityWindow_MigrateMasks` `$004B1C`, `Parallax_Step4_Fill` `$006EE2`,
`Section_GetSecPtrXY` `$0066D4`, `TileCache_DecompressBlock` `$005460`,
`GameState_OJZScroll_Init` `$05E00E`, `Replay_OJZ_Fixture` `$05E568`.

RAM (debug shape), re-resolved from `s4.debug.lst` this session and unchanged
from the migmask table: `Logic_Tick` `$FFFF8004`, `Input_Source` `$FFFF803A`,
`Replay_Exit_Request` `$FFFF803B`, `Replay_Done` `$FFFF803C`, `Replay_Hold`
`$FFFF803D`, `Replay_Prev` `$FFFF803E`, `Replay_Ptr` `$FFFF8040`,
`Camera_X` `$FFFFA152`, `Camera_Y` `$FFFFA156`, `Entity_Scan_State` `$FFFFAC52`,
`Entity_Window_Active` `$FFFFACBE`, `Entity_Window_Anchor` `$FFFFACC0`,
`Entity_Loaded_Masks` `$FFFFACC6`, `Entity_Mask_Scratch` `$FFFFAD46`,
`Replay_Check_Idx` `$FFFFB406`, `Parallax_Shadow_Bands` `$FFFF8926`,
`Parallax_Shadow_Scroll_A` `$FFFF8976`, `Parallax_Shadow_Scroll_B` `$FFFF8986`.

## The drive

The standing anchored input-replay net, exactly as `2026-08-06-migmask-ab.md`
records it:

1. Persistent breakpoint at `GameState_OJZScroll_Init` (`$05E00E`) **before**
   `reload_rom`.
2. `reload_rom`; verify the cart by `memory_hash`.
3. Run to the breakpoint.
4. Poke `Input_Source` = `$01`, `Replay_Ptr` = `$05E568 + 20` = `$05E57C`.
5. Confirm `Replay_Hold` / `Replay_Prev` / `Replay_Done` / `Replay_Exit_Request`
   / `Replay_Check_Idx` are all 0 before resuming.
6. Clear breakpoints, install the probe's breakpoint, `step 1`, resume.

Probe 3 needs a second drive (the camera walk) because probe 4 established, and
this parcel does not change, that the shipped fixture never slides the window.

## Probe 1 — cart identity and the five deltas. Must DIFFER. Run first

Read the five addresses out of ROM on each cart and match the table above.

**If any of the five reads the same on both carts, stop.** Either the wrong cart
is loaded or the parcel did not reach that site, and every downstream "no
difference" would be measuring an unchanged instruction. This is the probe that
makes site coverage a measurement instead of a claim.

Non-vacuity: the reads are of five *distinct* addresses with three distinct
old/new byte pairs; a stuck reader returning one cart's bytes for both would
fail all five, not pass them.

## Probe 2 — the parallax arithmetic. `d3` must MATCH, `d5` must DIFFER

**Anchor:** `Parallax_Step4_Fill.copy_band` `$006F22`, hit every frame.

At the hit, `step` to the `lea` that follows the chain — **5 instructions on
both carts**: the `move.w d2,d3` preload the anchor sits on, plus the
4-instruction chain. (Stepping only 4 stops one instruction short, where `d3`
holds 5·d2 on NEW against 8·d2 on OLD; a reader who did that would see `d3`
differ and wrongly conclude the arithmetic claim is refuted.) Read `d2`, `d3`,
`d5`.

* `d2` (the band index, the chain's input) — MUST match. If it differs the two
  runs are not at the same game state and nothing below is interpretable.
* `d3` (the product) — MUST match. This is the parcel's arithmetic claim.
* `d5` — EXPECTED to differ: OLD `2 × d2`, NEW `d2`. Dead scratch, declared
  clobbered. Its difference is a *positive* control that the replaced chain is
  the one running, and its being the ONLY differing register is the claim.

Non-vacuity: the chain must have been entered with `d2 != 0`, or the whole chain
computes 0 on both carts and the equality is vacuous. `d2` starts at the band
index `k` and the loop wraps, so require at least one observed hit with
`d2 != 0` before reporting a match. **If every hit in the run has `d2 == 0` the
finding is "the config has one band and the stride is never exercised" — not
"no difference".**

## Probe 3 — `MigrateMasks` at ×26 across a real slide. Must MATCH

The fixture never slides the window, so sites 1-2 are unreachable from probe 4
and need their own drive: the scripted camera walk from
`2026-08-06-migmask-ab.md`'s second sitting, which is the drive that made OLD
and NEW differ for `migmask`. **Here it must make them AGREE.**

Poke `Camera_X` **at the `EntityWindow_Scan` breakpoint** (`$0046A8`, the same
address on both carts and downstream of `Camera_Update`, which is what stops the
poke being pulled back), on a `Logic_Tick`-valued anchor so both carts are at the
same game state:

| at | poke `Camera_X` | effect |
|---|---|---|
| `Scan` break, `Logic_Tick == 1` | `$0900` | park by section 1's rings |
| next `Scan` break | `$0A00` | anchor `(0,0)` → `(1,0)`, rightward |
| next `Scan` break | `$0200` | anchor `(1,0)` → `(0,0)`, **leftward** |

**Content precondition** (without it the probe can pass while measuring
nothing): at the leftward slide's `MigrateMasks` entry (`$004B1C`), some entry
`k` in 1..3 must have `E[k] = byte at Entity_Scan_State + 26k + $12` non-`$FF`,
present in the snapshot ids at `Entity_Mask_Scratch`, with a NON-ZERO 32-byte
block. `migmask`'s executed run reached exactly this state with `k = 1`; if this
run does not, the finding is "no qualifying slide", not "no difference".

**Measurement:** at `$004B1C` dump the proc's whole input
(`Entity_Scan_State` 104 B, `Entity_Mask_Scratch` 132 B, `Entity_Loaded_Masks`
128 B, `Entity_Window_Active`, `Entity_Window_Anchor`, `a4`); `step_out`; dump
`Entity_Loaded_Masks` again. **Inputs AND outputs must be byte-identical on both
carts.**

Hit proof for sites 1-2: the `$004B22` breakpoint must fire before the
`step_out`. Without that this probe proves the two carts agree about code
neither of them ran.

**A difference here refutes the parcel** — the ×26 chain's arithmetic is
supposed to be untouched.

## Probe 4 — the shipped fixture. Must MATCH, and must be shown to reach sites 3-5

Same anchored drive, DEBUG cart, **no pokes**, run to `Replay_Done = $FF`.

`ojz_fixture.bin` is 2059 ticks with 33 curated checkpoints; a mismatch raises
`REPLAY DESYNC` under DEBUG. **Both carts must complete with zero desyncs, the
same end `Replay_Ptr`, and the same `Entity_Loaded_Masks` hash.** The chain-51
value recorded by the migmask run is `0x25913C7E`; it must not move.

**A desync on NEW refutes the parcel.** For `migmask` a desync would only have
meant "the fixture recorded the bug"; here the parcel claims to change no
behaviour at all, so a hashed-state divergence is a defect, full stop.

Non-vacuity, and this is the probe's load-bearing half: breakpoints at
`Parallax_Step4_Fill.copy_band` (site 5), `Section_GetSecPtrXY` (site 4) and
`TileCache_DecompressBlock` (site 3) must each be observed to FIRE during the
run. Sites 1-2 will NOT fire — the fixture never slides — which is why probe 3
exists and why "the fixture is clean" is not on its own a proof of anything.

## What would refute the parcel

* Probe 1: any of the five sites reading the same bytes on both carts.
* Probe 2: `d3` differing at a hit where `d2` matched.
* Probe 3: any input or output byte differing, at a hit that met the content
  precondition and after `$004B22` was proven to fire.
* Probe 4: a `REPLAY DESYNC` on NEW, a different end `Replay_Ptr`, or a moved
  `Entity_Loaded_Masks` hash.

## What would NOT refute the parcel

* `d5` differing at the parallax site. Declared-clobbered dead scratch; the
  difference is predicted and is a positive control.
* Overflow (`V`) differing after any of the five doublings. `add` and `lsl` set
  `V` differently for the same result; `mul_const`'s CC contract is
  clobbered-undefined, and every consumer at all four `mul_const` sites was
  re-read this session — each next flag-reader is fed by a later instruction
  (`move.l a2,d3` / `tst.l (a0)` / the copy's own moves).
* Sites 1-2 never firing during probe 4. The fixture does not slide; that is a
  measured fact from `2026-08-06-migmask-ab.md` probe 3, and it is the reason
  probe 3 here exists. It is also the coverage hole lane D closes.
* Frame or cycle counts differing between the carts at any un-anchored moment.
  Every delta is cheaper by construction; phase drift is the parcel working.

---

# EXECUTION — foreground run, 2026-08-07

Carts identified by `memory_hash` over the full 423571 bytes: OLD `0x7E273B14`,
NEW `0xDA2E3646`. The reload diagnostic printed the SAME `first16`
(`FFFFFF00000002000005E6A80005E6C0`) for both, confirming again that it cannot
distinguish them. Symbols from `s4.debug.lst`.

## PROBE 1 — PASS. All five sites differ, 10 of 10 reads exact

| site | addr | OLD read | NEW read |
|---|---|---|---|
| 1 `MigrateMasks.new_loop`+4 | `$004B22` | **`E348`** | **`D040`** |
| 2 `MigrateMasks.new_loop`+$C | `$004B2A` | **`E348`** | **`D040`** |
| 3 `TileCache_DecompressBlock.rr_ok`+$4A | `$0054C4` | **`E34B`** | **`D643`** |
| 4 `Section_GetSecPtrXY`+$24 | `$0066F8` | **`E348`** | **`D040`** |
| 5 `Parallax_Step4_Fill.copy_band` | `$006F22` | **`3602 E74B 3A02 DA45 D645 43E8`** | **`3602 3A03 E54B D645 D643 43E8`** |

Every read matched the predicted table. The change is live at every site, not
only at the one the author inspected.

## PROBE 2 — PASS. `d3` identical, `d5` differs exactly as predicted

Anchored drive to `GameState_OJZScroll_Init`, `Input_Source` = 1,
`Replay_Ptr` = `$0005E57C`, companions all zero on both carts; breakpoint at
`copy_band` `$006F22`; three consecutive hits (the same `Parallax_Step4_Fill`
call — `Logic_Tick` = 1, `d6` counting 3 → 2 → 1), stepped 5 instructions to the
`lea` and read.

| hit | `d2` | `d3` OLD | `d3` NEW | `d5` OLD | `d5` NEW |
|---|---|---|---|---|---|
| 1 | `$3` | **`$1E`** | **`$1E`** | `$6` | `$3` |
| 2 | `$0` | `$00` | `$00` | `$0` | `$0` |
| 3 | `$1` | **`$0A`** | **`$0A`** | `$2` | `$1` |

`$1E` = 30 = 3 × 10 and `$0A` = 10 = 1 × 10, so hits 1 and 3 are **non-vacuous**
(hit 2, at `d2 = 0`, is the vacuous case and is reported as such). At every hit
**a0-a7 and d0,d1,d2,d3,d4,d6,d7 were byte-identical between the carts**; `d5`
was the ONLY differing register, at exactly the predicted values (OLD `2·d2`,
NEW `d2`), and it is written once in the whole proc and declared clobbered.

## PROBE 3 — PASS. `MigrateMasks` at ×26 across a real leftward slide

Camera walk with the poke applied AT the `EntityWindow_Scan` breakpoint
(`$0046A8`), `Logic_Tick == 1` at the first poke; `Camera_X` high word
`$0900` → `$0A00` → `$0200`. Both carts tracked identically through the
intermediate checkpoints: anchor `(0,0)` slot 0 `$7F` after the park, anchor
`(1,0)` slot 0 `$3F` after the rightward slide.

Content precondition SATISFIED and identical on both: new anchor `(0,0)`,
`E[1] = $01`, snapshot ids `$01 $02 $04 $05`, block `j=0` = `$3F` rings + `$01`
obj (non-zero) — the same qualifying state `migmask` reached.

At `MigrateMasks` entry (`$004B1C`) the proc's ENTIRE input was byte-identical:

| input | OLD | NEW |
|---|---|---|
| `Entity_Scan_State` (104 B) | `0x2E441043` | `0x2E441043` |
| `Entity_Mask_Scratch` (132 B) | `0x238702C1` | `0x238702C1` |
| `Entity_Loaded_Masks` (128 B) | `0xC2A8FA9D` | `0xC2A8FA9D` |
| `Entity_Window_Active` | `$0F` | `$0F` |
| `Entity_Window_Anchor` | `(0,0)` | `(0,0)` |
| `a4` | `$FFFFAD46` | `$FFFFAD46` |
| whole register file | identical | identical |

**Hit proof for sites 1-2:** stepped through `$004B22` and `$004B2A` on BOTH
carts (PC observed at each). This probe is not comparing agreement about code
neither cart ran.

**Output at `MigrateMasks`' RETURN** (`step_out`, landing at
`EntityWindow_Slide+$BA` = `$004C2A` on both): `Entity_Loaded_Masks` =
**`0x9CFA99EF` on both carts**, slot 1 = `$3F` rings + `$01` obj — migrated.

That output is only reachable if the ×26 stride produced 26 for entry 1, so the
substitution is exercised **non-vacuously**: a broken doubling would have left
slot 1 zero, which is precisely the failure `migmask` exhibited.

## PROBE 4 — PASS. The shipped fixture is unmoved

Anchored drive, no pokes, run to completion on both carts.

| | OLD | NEW |
|---|---|---|
| `Replay_Done` | `$FF` | `$FF` |
| `Replay_Ptr` (end) | `$0005E6A0` | `$0005E6A0` |
| `Entity_Loaded_Masks` (128 B) | `0x25913C7E` | `0x25913C7E` |
| `REPLAY DESYNC` | none | none |
| system log | clean | clean |

`0x25913C7E` and `$0005E6A0` are the values `2026-08-06-migmask-ab.md` recorded
for chain 51; they are reproduced here and unmoved by this parcel. Zero desyncs
means all 33 curated checkpoint hashes agree.

**Non-vacuity:** site 3 (`TileCache_DecompressBlock`) fired during the run on
both carts, at the **same `Logic_Tick` = `$54E`**. Site 5 is NOT observed here —
it is covered by probe 2's own drive — and site 4 never fires at all (below).
The spec listed all three under this probe's non-vacuity condition; only site 3
discharges it here, and the other two are discharged or reported elsewhere rather
than left implied. `Logic_Tick` equality is a DETERMINISM check, not a timing
one: an all-cheaper parcel moves cycle phase, not logic frames — itself an equality
measurement, since a timing-sensitive divergence would move the tick at which
the first block decompresses.

`Logic_Tick` was read at the init anchor (1) and at the site-3 hit (`$54E`) on
both carts: the tick-advance control holds, and no reading below rests on a run
that did not run.

**`Logic_Tick` at the END is NOT a comparand** and is not reported as one: both
carts were paused at an arbitrary wall-clock moment after `Replay_Done`, so the
end tick reflects how long the emulator free-ran, not the parcel.

## Coverage residue — honest, and it is lane D's case

**Site 4 (`Section_GetSecPtrXY`+$24) is never exercised behaviourally by any
drive available here.** Measured, not assumed:

* the 2059-tick shipped fixture: a breakpoint at `$0066F8` was armed for the
  whole remainder of the run on both carts and **never fired**;
* the camera walk: it fired **exactly once**, with `d0 = 0` — flat section index
  0, so `0 × 66 = 0` under both encodings, i.e. vacuous — and did not fire again
  in a further 60 s of free run.

So the corpus's only automated behavioural net does not reach this proc at all,
and the one drive that does reaches it only at the one input where the change
cannot show. That is a coverage finding about the fixture, not evidence about
the parcel, and it is reported as such.

What DOES cover site 4's claim is the in-tree executed oracle:
`word_lowering_matches_low_word_and_leaves_upper_free` runs every chosen `.w`
lowering over 9 sampled/boundary `x` values × 3 upper-word garbage seeds × with-
and without-scratch, and asserts the low word equals `x · n mod 2^16`. `n = 66`
is in that domain, so the substituted chain is machine-checked exhaustively over
it. **This parcel adds `n = 10` to the same domain**, so the new parallax stride
is covered there too.

## Verdict

**All four probes pass.** The five encoding changes are proven live on the
shipping code path; the arithmetic is proven identical at the parallax site with
non-zero operands, and at the ×26 site through an outcome that could not occur
if the doubling were wrong; the shipped fixture's 33 checkpoints, its end
pointer and the entity mask buffer are all unmoved; and the one register that
differs is the declared-clobbered scratch this note predicted before the run.

Nothing about the change is unmeasured except site 4's behaviour, which is named
above as a fixture-coverage hole rather than glossed.

## Instrument finding — the halt can land BEFORE the breakpoint

New this session, and it is a fresh member of the same family as the
resume-on-a-breakpoint trap: **`wait_for_break` can return with the PC several
instructions BEFORE the breakpoint address**, while the log records
"Breakpoint triggered at <addr>" (twice — the tell of a rollback-and-replay).
Observed at `$006F22` (halt reported at `$006F16`, three instructions early,
verified real by stepping: `$6F16` → `$6F1A`) and at `$0054C4` (halt at
`$0054BC`) and `$004B1C` (halt at `$0046A8`-relative sites). It is
intermittent — the same breakpoint halted exactly on address on later hits.

A reader who trusts the returned PC and reads registers there gets a register
file from BEFORE the code under test, which on an identity A/B looks like a
clean pass.

**RULE, alongside the tick-advance rule: after every `wait_for_break`, verify
the PC equals the anchor and single-step forward until it does, before reading
anything as evidence.** Every register read in this A/B was taken after such a
verification.
