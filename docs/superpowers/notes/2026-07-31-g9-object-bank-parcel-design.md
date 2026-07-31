# G9 (player_ground d7 high-word clear) — deferred to its own object-bank parcel

**Status:** DEFERRED with a design step first (overseer ruling, 2026-07-31). G9 was the
Wave-A pathfinder candidate; it was re-scoped OUT when its true cost surfaced. animate A3 took
the pathfinder seat instead.

## The census correction (G9 was undercounted)

The §17 census and the opt-sweep design note rated G9 effort **S** ("one `moveq #0,d7`, 4 cyc").
That undercounted it: `player_ground` is the **FIRST native section in the object code bank**
(`org $10000`), so a +2 growth at the bank head does NOT stay local — it collides `player_air`'s
pinned resume org and cascades through the whole downstream object-bank org chain. Building the
fix fails at `resolve_layout`:

> sections `player_ground` [0x10448, 0x10898) and `player_air` [0x10896, 0x10B58) overlap
> (colliding pins)

The cascade is ~15 fixed resume orgs in `games/sonic4/main.asm` (`$10896`, `$10B58`, `$10BF4`,
`$10C34`, `$10C38`, `$10C92`, `$10FAA`, `$10FFE`, `$11128`, `$11182`, `$111FA`, the per-shape
`ifdef __DEBUG__` pair `$112F4`/`$1128C`, the later `$11DE6`/`$11D7E` pair) PLUS `test_player.asm`
(`$10F02`) and `test_enemy.asm` (`$10F4A`), PLUS possibly `act_descriptor.asm` (`$14E06`/`$14D9E`)
if the shift reaches that far — the absorption boundary is undetermined without trial builds.
These resume orgs are exactly the **rows-6/58 pin-tax scaffolding** the campaign has twice
declined to hand-maintain. Paying a 15-org hand-bump for a 4-cycle fix is that tax.

**Correction filed:** an item's effort must account for its POSITION in a fixed-org chain, not
just its own byte delta. A byte-changer at a bank/region HEAD inherits the whole downstream
cascade. (Contrast animate A3, mid-engine-block + size-neutral: 2-byte diff, zero shift.)

## The design step (do this BEFORE executing G9)

Evaluate two ways to pay the cascade:

1. **Hand-bump** — bump the ~15 per-shape resume orgs +2, determine the absorption boundary by
   build, run the 5-site re-pin. Mechanical but error-prone (a wrong-but-building layout silently
   poisons the re-frozen goldens; post-flip there is no independent asl oracle for object-bank
   bytes — only the emulator A/B catches a mis-layout).

2. **Computed placement (LEAN THIS WAY)** — flip canonical sonic4's object-bank placement from
   PinnedBaked to the chainer's computed mode. The `SizeSource::Frozen` machinery
   (`native.rs::build_rom_chained`) ALREADY computes downstream bases from section geometry for
   the four off-canonical targets; extending it to canonical sonic4 makes a +2 at the bank head
   shift downstream sections AUTOMATICALLY, and the re-freeze captures the new layout with no
   hand-bump. This is the **partial realization of the capstone-ledgered pins→map flip** that
   rows 6/58 always wanted, and G9 becomes its natural first exercise.

If the design step finds computed-for-canonical is genuinely **capstone-sized** (too large to
land as G9's prerequisite), G9 PARKS with an annotated ledger row rather than paying the
hand-bump — the fix waits for the capstone flip.

## A/B reachability prep (real prep, not a blocker)

G9 lives in `Ground_Move_Cap`'s wall-probe, which runs only with a **grounded, moving** sonic4
player (`gsp != 0`, gated at `.wall_probe`). Prep notes for that parcel's A/B:
- Needs a real sonic4 gameplay scene (grounded Sonic running) — the wall-probe runs EVERY
  grounded-moving frame, so any running-Sonic scene reaches `Ground_Move_Cap` (`0x10724`, both
  shapes); no wall contact required to reach the `move.b`/`.off_*` decode.
- **No register-injection on the oracle** (ledger row 21: no set-PC/write-register), so the
  latent bug can't be forced by writing a dirty `d7` — the A/B must reach the guard naturally and
  show `d7`'s high word clean at the `move.b` (benign-under-current-dispatch, made concrete),
  plus OLD/NEW state-identity over a grounded-movement scene (the fix is inert today).
- Breakpoint by address (`0x10724` + the `.dir_fwd` offset) works without loading symbols.

## The fix (preserved verbatim)

```
.dir_fwd:
        lea     .dir_table(pc), a1
        moveq   #0, d7                  // clear the high word: the probe code is a
                                        // BYTE, but d7 is read as a word below
                                        // (.off_*/.cancel_* move.w/tst.w)
        move.b  (a1,d2.w), d7           // probe-core direction code
```

One instruction (`moveq #0, d7`, +2 bytes) before `player_ground.emp`'s `move.b (a1,d2.w), d7`
(~:670). Correctness-hardening: the byte-loaded probe code is consumed as a word, benign today
only because `Player_Main` enters with `d7`=slot-counter=0.
