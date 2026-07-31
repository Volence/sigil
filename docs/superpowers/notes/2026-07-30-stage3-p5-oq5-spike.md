# 2026-07-30 — STAGE 3 · THE OQ-5 SPIKE (the `.emp`→residual-AS export mechanism) — **STOP**

Status: **SPIKE RESULT — a designed STOP at Checkpoint 1.** The spike proves the
export path the P5 ownership flips need against REAL residual DATA readers on the
live six-target build. The finding: **the mechanism the flip needs does NOT exist
today** — link-deferral (the proven reverse-seam) serves only link-position
readers, and the residual AS has load-bearing COMPTIME readers of the very
constants the flip moves. Closing the P5 rows byte-neutrally requires a NEW
`.emp`→AS comptime-value bridge plus a design ruling. No aeon file was changed; no
pin moved. Per the brief's OQ-F and the "STOP on missing capability / design fork"
valve, execution halts here for the overseer's ruling before P5.

Masters: aeon `c8f2948` / sigil `6aea17b`. Branch pair `stage3-p5`. All six goldens
+ anchors verified UNMOVED at spike open (the scratch six-target proof, below).

---

## §0 — BASELINE VERIFIED (the byte-neutral bar this spike must not move)

Scratch `sigil build --native` of all six targets (never `build.sh`), CRC'd
full-file + header-neutral anchor, all match the frozen goldens exactly:

```
s4         full 7f071417/412306  anchor e5765873      demo       full 705a5871/90436  anchor cfda98d3
s4.debug   full 0b8efc7a/422147  anchor dab4f06c      demo.debug full 37ded207/92935 anchor 20c5571d
config_a   full 1b4c49d2/422483  anchor 3d9bac53      config_b   full bfe2509e/303660 anchor fd3f7f8e
```

(Worktree note: `engine/debug/generated/` is gitignored — copied from the peer
checkout to build the debug/config shapes, the seed-worktree pattern; the sound
`generated/` set is minted by `ensure_generated`.)

---

## §1 — THE QUESTION (restated exactly)

P5 flips `engine/constants.asm` (rows 1/2/12/14/17/19/20) and `engine/structs.asm`
(rows 7/8/11/15/25) from DEFINITION to consumer: `constants.emp`/`structs.emp` +
the sst/act/EntityScanState overlays become the SOLE definitions; residual AS takes
the values via "the proven mechanism." OQ-5/OQ-F requires proving that mechanism
against residual DATA readers (config, generated tree, parallax, ram.asm,
macros.asm, keystone headers), not just the now-deleted code twins.

`constants.emp` owns **115** constants (the drift-mirror set). `constants.asm`
carries **243** total (`=` equates); the flip would remove the 115 from AS-side
authorship, leaving **128** AS-retained.

---

## §2 — THE TWO SEAM DIRECTIONS, AND WHICH ONE EXISTS

**Direction A — AS defines, `.emp` reads (the FORWARD seam):** works today. This
is how the drift guards run: `constants.emp` carries `const X = v` + `ensure(
extern("X") == X)`, reading the AS-side equate as a link symbol.

**Direction B — `.emp` defines, AS reads (the REVERSE seam = the flip):** partially
exists.

- **Link-position readers — PROVEN.** `reverse_seam_ordinals.rs`: an `.emp`
  `equ NAME = <val>` lowers to a link-level `EquSym`, and AS reads it in the
  deferral shapes (`move.l #NAME` imm32, `dc.b/dc.w NAME`) through the joint link.
  The AS frontend's `partial_fold`/`fixup_target` machinery (`eval.rs:3832+`)
  defers an AS-undefined symbol to the linker, which resolves it from the `.emp`
  section-label / equ table. **This half of the mechanism is real.**

- **Comptime-position readers — MISSING, and load-bearing.** A value used in a
  `ds.b`/`ds.w` COUNT, an `if`, a `rept` count, or a derived AS `=` must be known
  at AS ASSEMBLE time — before layout — so the linker CANNOT serve it (it sets
  section length, which shifts everything after). The AS frontend receives
  comptime values ONLY through its `defines: Vec<(String,i64)>` input; there is NO
  wired `.emp`→AS comptime-value path. An `.emp` `const` moreover lowers to ZERO
  link symbols (only `equ` is link-visible), and link ≠ comptime regardless.

---

## §3 — THE RESIDUAL AS *DOES* READ THESE CONSTANTS AT COMPTIME (the census)

The 11 `.asm` files referencing any of the 115 emp-owned constants; the load-bearing
comptime readers, verbatim:

**`engine/ram.asm` (residual — the RAM layout, `ds` reserves that fix RAM
addresses; ~20 comptime uses):**
```
ds.b  TILE_CACHE_NT_SIZE                 ds.b  VDP_Shadow_len
if    ART_STAGING_BUFFER_SIZE > TILE_CACHE_NT_SIZE
ds.b  DMA_CRITICAL_SLOTS*DMAEntry_len    ds.b  SST_len * NUM_DYNAMIC/NUM_SYSTEM/NUM_EFFECTS
ds.w  NUM_DYNAMIC / NUM_EFFECTS / NUM_DYNAMIC_PENDING
ds.w  SPRITES_PER_BAND * PRIORITY_BANDS  ds.b  PRIORITY_BANDS / SCANLINE_BANDS
```
**`engine/macros.asm` (residual — comptime):**
```
if (px) > SECTION_SIZE-1
dc.b ODZ_RFVAL|(ODZ_PB<<RF_PRIORITY_SHIFT)
```
**`engine/constants.asm` itself — 128 AS-retained equates DERIVE from emp-owned
ones at comptime:**
```
ART_STAGING_BUFFER_SIZE = ART_POOL_PAGE_TILES*32
SECTION_TILE_WIDTH/HEIGHT = SECTION_SIZE/8
TILE_CACHE_COLL_ROWS = TILE_CACHE_ROWS/2   TILE_CACHE_COLL_SIZE = TILE_CACHE_COLS*TILE_CACHE_COLL_ROWS
```
**`engine/structs.asm`** reads `SECTION_SIZE_SHIFT`/`EDGE_CLAMP` in field math
(it is itself a P5 flip target, but is the AS-side struct-equ producer today).

### The empirical proof (ironclad)

Commenting out ONE `ds`-consumed emp-owned constant
(`TILE_CACHE_NT_SIZE = TILE_CACHE_COLS*TILE_CACHE_ROWS*2`) from `constants.asm` and
rebuilding s4:

```
error: assemble (native AS side, sonic4): "unresolved ds count"
```

The linker cannot defer a `ds` count. Link-deferral is structurally insufficient
for the residual readers the flip must satisfy. **Removing the 115 from AS-side
authorship WITHOUT a comptime bridge breaks the build.**

---

## §4 — WHY THIS IS A STOP (not a continue)

Closing rows 1/2/7/8/11/12/14/15/17/19/20/25 requires removing the 115 constants'
AS-side authorship (that IS the ownership flip — "delete its mirror consts AND its
drift guards together"). But:

1. Almost every drift-block has a COMPTIME residual reader (ram.asm hits rows 1
   `VDP_Shadow_len`/`VDP_SPRITE_*_OFFSET`, 19 `NUM_*`, 20 sprite geometry; macros.asm
   hits 12/20 `RF_*`). A piecemeal "flip only the link-position constants" cannot
   close a row whose block contains a comptime-read member.
2. The AS-retained 128 DERIVE from the emp-owned 115 at comptime, so the emp values
   must be present AS-side at assemble start.
3. No `.emp`→AS comptime-value bridge exists. Building one is a real (if modest)
   mechanism, and choosing its shape is a design fork.

Per the brief: *"If the mechanism needs new capability work, STOP and present the
design."* It does. Halting.

---

## §5 — THE DESIGN TO RULE ON (the recommendation)

The bridge must make the 115 emp-owned VALUES available AS-side at COMPTIME. Two
shapes; both need the same core new piece — **harvest the resolved `.emp` const
values into a `name → i64` map** (feasible: express/export them as `.emp` `equ`
so they land in `Module.equ_syms` (`sigil-ir/src/lib.rs:52/215`), or add a
comptime-const harvest to the lowering; the values are pure comptime scalars).

- **Option A (RECOMMENDED) — inject the harvested map as AS `-D` defines.**
  `assemble_as_side` already takes `defines: Vec<(String,i64)>` (and per-profile
  `extra_as_defines`). Pre-lower the `.emp` const-equs, harvest the 115, PREPEND
  them to the AS defines so `constants.asm`/`ram.asm`/`macros.asm` see them at
  assemble start. `constants.asm` loses the 115 `=` lines (keeps the 128 derived
  AS-retained, which now fold off the injected defines). No new file, no include
  change. Byte-neutral by construction (identical values). **The one verification
  the design owes:** that an AS `-D NAME=v` folds byte-identically to an in-file
  `NAME = v` in every position the 115 appear (immediate, `ds` count, `if`, shift,
  derived `=`) — a small proof harness, not a language change.

- **Option B — generate an AS equ include from the `.emp`** (the `mt_syms.asm`
  precedent): emit `engine/constants.gen.asm` = `X = v ×115`, include it where
  `constants.asm` is included. Same harvest; adds a generated file + a
  build-ordering dependency + an include edit. More surface than A for no gain
  (the values are the same); A is the cleaner reverse-seam.

- **Option C — NOT a flip.** Keep `constants.asm` hand-authored, only drop the
  `.emp` drift guards. Rejected: this keeps dual authorship, does not close the
  rows honestly (the mirror still exists, just unguarded — strictly worse).

**Recommended ruling:** authorize **Option A**. Its new mechanism is a harness
build-flow addition (harvest emp const-equs → inject as AS defines) + a
byte-identity proof of `-D` vs in-file `=` folding. It is byte-neutral, closes the
constant rows (1/2/12/14/17/19/20), and generalizes to `structs.asm`'s struct-equ
producers (rows 7/8/11/15/25 — the struct offsets are the same comptime-value
class, harvested from the `.emp` struct `sizeof`/`offsetof`) and to P6's config
constants.

### Open sub-decisions folded into the ruling

- **The 115/128 split policy.** Does the flip move ALL of `constants.asm` to `.emp`
  eventually, or permanently keep the 128 AS-retained (many are residual-only:
  `ST_YFLIP`, `OEF_*`, `SECTION_TILE_*`, `DMAEntry_len`)? Recommend: flip only the
  115 drift-mirror set now (that closes the rows); the 128 stay AS-authored and
  fold off the injected defines. A later parcel can migrate the residual-only
  remainder if desired.
- **Drift-guard transformation.** Post-flip the `ensure(extern("X")==X)` guards in
  `constants.emp` have nothing to check (the `.emp` is the sole author). They
  DELETE with the flip (row kill = "delete mirror consts AND drift guards
  together"). The byte gates vs the six goldens become the guard.
- **`STAGE1_INAPPLICABLE_GUARDS` retirement (row 93 data-half).** The 4 Poison
  guards are the `bg.asm`/`camera.asm` extern class (`VRAM_PLANE_B_BYTES`,
  `CAM_SCREEN_HALF_*`). They resolve once those bg/camera constants become
  exported link symbols — i.e. the SAME reverse-seam. If Option A also exports the
  bg/camera constants (or they join the injected-defines set), the allowlist +
  `enforce_inapplicable_allowlist` machinery retire with its t24 cleanup. To
  confirm at execution: those two constants are NOT in the 115 constants.emp set —
  they live in `bg.emp`/`camera.emp`. Their export is a parallel application of the
  same mechanism, ruled together.

### P6 note (independently assessable)

P6's untyped game constants (`SONG_*`/`SFXID_*` numeric ids, `VRAM_TEST_*`,
buttons) fold mostly into IMMEDIATES (rows 54/62/65: art_tile/vram_bytes/`move #id`)
— link-position, which the PROVEN half already serves. P6 MAY be feasible on the
existing mechanism alone IF a census shows no comptime residual reader of those
ids. But P6 also touches `config/*.asm`, which needs the same census; and the
game-constants `.emp` module needs a home the gate-off AS build can't see today.
Recommend ruling P6 together with the P5 mechanism (Option A serves both), rather
than splitting the bridge decision.

---

## §6 — WHAT DID NOT MOVE

No aeon edit survived the spike (the one probe edit was reverted; baseline
re-proven). No pin, no golden, no anchor moved. Strict not re-run (read-only
spike; no code change to gate). The row-91 witness untouched.

**Awaiting the overseer's mechanism ruling before P5.**
