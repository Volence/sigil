# t13 load_object — step-5 second-look follow-up: burst-copy de-pessimization

**Branch** `load-object-step5-burst-copy` (aeon + sigil), NOT merged — Volence gate
(sprites-step-5-followup precedent). Post-merge follow-up on the already-landed t13
(aeon `0bb1ca4` / sigil `3963617`).

## What changed & why

Fable's step-5 second look (post-merge, hot-path rule applied late) overturned the
t13 packet's "no opt taken on the burst copy" line. `Load_Object`'s 24-byte template
copy used three `movem.l (a2)+,d3-d4` / `movem.l d3-d4,N(a3)` PAIRS. A 2-register
`movem` costs ~28 cyc/long; `move.l (a2)+,(a3)+` costs 20 cyc/long — `movem` only
breaks even at ≥6 registers. This was a **pessimization**, not a clean idiom.

Replaced `lea Sst.x_vel(a1),a3` + 3×(movem-in/movem-out) with `lea` + six
`move.l (a2)+,(a3)+` (comment groups `$0A-$11` / `$12-$19` / `$1A-$21`). Consequences:

- **−0x10 bytes, ~−68 cyc/spawn** on the burst copy.
- **d4 eliminated entirely** — the movem pairs were the only d4 scratch, so the
  `move.l d4,-(sp)` / `move.l (sp)+,d4` cross-branch save/restore is DELETED. The
  caller (`EntityWindow_TrySpawnObject`) reads d4 after return; that reliance is now
  satisfied by **non-use** (d4 is never touched). `clobbers(d0-d3,a1-a3)` unchanged
  and still accurate. Another **−0x4**.
- **`bsr.w Load_Object` → `bsr.s`** in `Load_ObjectList` — NEWLY ENABLED by the
  −0x14 shrink: the backward displacement to `Load_Object` moved from -146 (needs
  .w) to -126 (fits .s). The `.emp`'s `jbsr` auto-relaxes; the `.asm` twin is
  hand-set, so it was lockstepped to `.s` per the ratified rule (twin explicit width
  is never a width exception — shrink it + re-pin). Another **−0x2**.

**Total −0x16 (−22 bytes).** Region `load_object` **$98 → $82** both shapes (the
task modeled −0x14 → $84; the bsr relaxation is the extra −0x2, a logged step-5
improvement beyond the stated expectation).

Also added a module-level drift tripwire `ensure(RF_XFLIP == 1 && RF_YFLIP == 2, …)`
— the `rol.w #4` flip fold silently assumes OEF bits 13/14 land on RF bits 1/2;
this catches the realistic sigil-side RF drift with no constants-twin ripple.

## Byte gate (emp == asm, both shapes)

`SIGIL_STRICT_GATE=1` `load_object_port`: **both shapes byte-identical** at the new
pins (plain `s4.bin[0x3FDC..0x405E]`, debug `s4.debug.bin[0x4BA6..0x4C28]`, len $82).

## Re-pin wave

Region shrank −0x16; everything downstream slides −0x16 (nothing upstream moves;
data past org $10000 is org-fixed and unmoved).

- `pins.rs` regenerated (`repin` tool): LOAD_OBJECT len $98→$82; COLLISION_LOOKUP,
  SOUND_API, + their derived symbols (TILE_CACHE_GET_COLLISION, SOUND_DRAIN_SFX_RING,
  SOUND_PLAY_RING/SFX, SECTION_GET_SEC_PTR_XY, SECTION_FLAT_IDXY) each −0x16.
  `repin --check` clean.
- `engine.inc` gate resume orgs (−0x16 each, both shapes): load_object $4074/$4C3E →
  $405E/$4C28; collision_lookup $4C58/$58DA → $4C42/$58C4; sound_api $5F8E/$79A0 →
  $5F78/$798A.
- `repin_pins.rs` hand-typed baseline: SOUND_API bases $5DAA/$76C6 → $5D94/$76B0
  (documenting comment added).

## Provenance (byte-CHANGING — this is a code optimization, not byte-neutral)

New master ROM markers (supersede t13's plain ab855868 / debug 410cf2c5):
- plain `s4.bin` 452500 / crc32 **11382fa7** / sha256 `c05ce9da5971bc06…`
- debug `s4.debug.bin` 460521 / crc32 **36bf0f17** / sha256 `34ff40ab742ffeab…`

Lengths unchanged (EndOfRom org-$10000-shielded); engine-block content shifted.

## Gate results

- `load_object_port` both shapes byte-identical (strict).
- Full workspace strict vs the branch aeon tree: **2213 pass / 0 fail** (paired-state gate).
- `cargo clippy --workspace --all-targets`: clean.
- `repin --check`: pins.rs unchanged (idempotent).
- Both aeon shapes build clean (exit 0, abort-on-AS-error rider silent).

## Ledger

- **Row 2 (proc.clobber-undeclared FP on individual-push-across-branch, d4) —
  CLOSED by construction.** The push-across-branch pattern is gone with d4; zero
  residual warnings on the port. The underlying S2-D6 heuristic gap stays open (no
  live instance here).
- **Row 4 (movem block-copy idiom) — AMENDED: reclassified idiom → PESSIMIZATION.**
  New t14/t15 trigger: grep `children.asm` (CreateEffect template setup) + objdef
  emitters for the `movem.l …,d3-d4` small-block anti-pattern and apply the same
  `move.l (a2)+,(a3)+` rewrite where the block is <6 longs. The "revisit for a
  construct at a 2nd site" note is superseded — the right move is the peephole, not
  a comptime-fn around a pessimized instruction.

## Per-pass step-3 vs step-5

- **Step 5 (engine optimize):** the whole finding — movem-pair → move.l de-pessimization,
  d4 elimination, bsr relaxation. Headline: a "clean idiom" the t13 packet ledgered
  for a future construct was actually a movem-for-small-block misuse.
- **Step 3 (retrospect):** none new — the interrogation was Fable's second look; this
  packet is the build-out.
- **Neither bucket:** the `bsr.w→.s` relaxation was NOT in the task's −0x14/$84 model;
  taking it (ratified "BETTER not SAME") is the one deviation from the stated expectation,
  flagged here for the gate.
