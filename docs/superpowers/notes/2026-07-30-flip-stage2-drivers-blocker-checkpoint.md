# 2026-07-30 — FLIP STAGE 2 · the off-canonical DRIVERS — infrastructure + THREE PRE-FLIP BLOCKERS

Status: **the GameProfile refactor + the frozen-table chainer are BUILT and PROVEN
(canonical sonic4 both shapes stay byte-green through them — the regression harness),
but building the three off-canonical drivers surfaced THREE genuine pre-flip identity
blockers that reshape the six-proof preamble. Two of the four off-canonical targets
(Config-B, demo) CANNOT be byte-identical to their goldens PRE-FLIP; they reference
sonic4-shape constants that only "resolve natively" at the flip commit (the brief's own
rows 52/90/6/58). This is a STOP per the valve (identity surprise + design fork) — the
flip sequence needs the overseer's ruling before the six-proof matrix can close.**

Sigil `flip-stage2`; aeon read-only `bcb8f64` (ZERO aeon commits). Strict **2919/0
(1 ignored)** from BOTH `AEON_DIR=<worktree>` AND `AEON_DIR=<main-checkout>`. asl default
byte-identical (s4 eff2396f · s4.debug 1e9097bc · demo 18c64002).

## What LANDED (byte-neutral for sonic4; the refactor the brief asked for)

`crates/sigil-harness/src/native.rs` — the sonic4-hardcoded driver is now a
`GameProfile`-parameterized engine:
- **`GameProfile` + `SizeSource`** (`PinnedBaked` = canonical sonic4, baked lmas
  asl-correct; `Frozen(table)` = demo/Config, sizes from the committed listing tables).
  `sonic4_profile` / `demo_profile` / `config_a_profile` / `config_b_profile`.
- **`assemble_as_side` / `build_emp` / `build_rom_chained`** take `&GameProfile`; the old
  `(aeon, debug)` entry points are thin sonic4 wrappers, so `native_rom` /
  `native_declared_chain` / `native_full_rom` / `native_chained_resume` /
  `mixed_offcanonical_rom` are UNCHANGED and green.
- **The frozen-table chainer** (`true_bases_by_index` / `declared_spans_by_index` /
  `apply_declared_chain` / `phase_region_mask` / `image_lens_pinned`): each ROM section's
  TRUE base is `frozen[L] − offset[L]` for a contained frozen label; label-less DATA
  blobs derive by contiguity (the inline Z80 idle) or keep their absolute hard-org
  (sound banks). The chainer, sort, and layout diagnostic share ONE placement routine.
- **Two real chainer soundness fixes proven along the way** (both regression-covered by
  sonic4 staying green): (a) a PHASE-BANK anchor must be detected against the section's
  OWN baked lma, not the re-based frozen `tb` — else any object-bank address ≥ 0x8000 is
  misread as a phase and its stale sonic4 VMA leaks into every reference to its labels
  (the +0x32 whole-object-bank drift); (b) a re-based section's VMA must track its new
  LMA (stale baked VMAs otherwise resolve every cross-reference to the sonic4 address).
- **`capture_offcanon`** grew the `HeightMaps` anchor (a real listing label) so the
  config tables can ORDER that label-less data blob after the emp anim regions; the
  config_a/config_b tables re-captured (golden CRCs preserved: config_a b4a6756d,
  config_b 92776720 — verified, asl-live). demo tables unchanged (no HeightMaps there).

Empirically, the frozen chainer places EVERY frozen-labeled section byte-correctly:
`config_b` `dump_frozen_layout` = **0 frozen-label mismatches** (TestPlayer, OJZ, the
error-handler tail, EndOfRom — all at their config_b addresses, not sonic4's). The
mechanism WORKS; the residual diffs are NOT placement — they are baked constants.

## THE THREE BLOCKERS (why Config-B and demo can't prove pre-flip)

### Blocker 1 — assembly-time-FOLDED sonic4 constants (rows 52 + 90) — Config-B AND demo

`config_b` builds cleanly (no drift, no overlap) but lands **111 header-neutral diffs**,
EVERY one a reference to a sonic4-PLAIN address of a symbol that is a numeric constant
FOLDED AT ASSEMBLY TIME, not a section label the chainer can re-place:
- **`Game_Entry = $5C7EC`** (`games/sonic4/config/game.asm`) — the boot state-handoff
  pointer. The file's own comment: *"Game_Entry FOLDED at assembly time (a
  link-time-equ-off-external-base is not available — the error_handler ErrorHandler-equ
  precedent)."* Config-B's real entry is `GameState_OJZScroll_Init = 0x4215C`; the AS
  side bakes `$5C7EC`. (First diff, `0x38F`.)
- **`ErrorHandler: equ $5CC0A`** (`engine/engine.inc` error_handler gate else-arm) —
  sonic4-plain. Config-B's is `0x4257A`. Every `jsr ErrorHandler` bakes `$5CC0A`
  (the `0x42000` cluster, 72 diffs).
- **`EndOfRom` via `org $5DB60`** and the object-bank data-tail references likewise fold
  to the sonic4-plain tail (the `0x43000` cluster).

These are precisely the brief's **rows 52 (ErrorHandler) / 90 (Game_Entry) / 6+58
(resume orgs)** — the numeric equs the flip commit says *"resolve natively."* Config-B's
layout SHIFTS off sonic4 (sound-off → EndOfRom `0x434D0` ≠ `0x5DB60`), so every one of
these folded constants is wrong for it. **There is no pre-flip fix without editing aeon
(nativizing the folds) — which is the flip commit itself.**

Why sonic4 and Config-A are immune: sonic4's folds ARE sonic4's own addresses.
**Config-A's tail is byte-identical to sonic4-DEBUG** (`config_a.txt` BusError `0x5E5AA`,
EndOfRom `0x5F65A` == the debug golden), so `Game_Entry = $5E2DA` and
`ErrorHandler = $5E704` (the debug else-arm) are CORRECT for Config-A. Config-A is the
only off-canonical target whose folded constants land right.

### Blocker 2 — engine `.emp` hardcodes GAME-config constants — demo only

demo's build FAILS the drift guards (correctly): three engine `.emp` modules embed
sonic4 game-config constants that demo's config sets differently AND that lower into
emitted immediates, so the demo native bytes cannot equal the demo golden (built from
the `.asm` twins reading demo's config):
- `engine/objects/rings.emp`: `const MAX_RING_BUFFER = 128` (demo config = 16;
  `cmpi.b #MAX_RING_BUFFER`), `const VRAM_RING_PLACEHOLDER = $3E8` (demo `$3E4`).
- `engine/objects/entity_window.emp`: `const COLLECTED_WINDOW_SLOTS = 9` (demo = 4;
  `moveq #COLLECTED_WINDOW_SLOTS-1`).

A comptime `-D` cannot override a `const` (that is a `[defines.collision]` error), so the
demo lockstep flip needs these promoted from hardcoded `const`s to game-config comptime
inputs — an **aeon `.emp` change**, outside the zero-commit span. (These are the only
guards that fire; there may be further UN-guarded game-config bakes the guards don't
catch — the guard net is partial.)

### Blocker 3 — sound-bank `align $8000` recomputation — Config-A only

Config-A gets furthest (folds correct, drift clean) but hits ONE layout snag: the
pre-sound-bank data blob `HeightMaps` (asl `0x257C6`) carries trailing `align $8000`
padding baked for its AS-residual position (`0x257BA`), so its image is `0xC` too long
for its true position and OVERSHOOTS the `MovingTrucks_Bank_Start` hard org at `0x58000`
(image `[0x257C6, 0x5800C)` vs bank `0x58000`). The declared SIZE clamps correctly to the
gap, but the ACTUAL image bytes still carry the stale padding — the linker does not
shrink a baked `align` when a section relocates. This is a real sigil-linker capability
gap (align-recompute-on-relocation), NOT a fold/config issue; Config-A would otherwise
prove (its constants are all correct).

## THE 6×2 MATRIX (pre-flip, honest)

| target | assembled anchor | status |
|---|---|---|
| sonic4 plain  | ✅ e5765873 | GREEN (`native_rom` + `native_declared_chain` + `native_full_rom`) |
| sonic4 debug  | ✅ dab4f06c | GREEN |
| config_a      | ⛔ 3d9bac53 | folds CORRECT; blocked by the `align $8000` linker recompute (blocker 3) |
| config_b      | ⛔ fd3f7f8e | builds clean, 0 placement mismatches; **111 diffs = folded sonic4 constants** (blocker 1) |
| demo plain    | ⛔ cfda98d3 | **drift guards fire** — engine `.emp` bakes sonic4 game consts (blocker 2) + blocker 1 |
| demo debug    | ⛔ 20c5571d | same as demo plain |

## THE RESHAPED-SEQUENCE QUESTION (for the overseer)

The brief's six-proof preamble assumes all four off-canonical targets are provable
`native == golden` BEFORE the flip. Blocker 1 shows that is **false for any config whose
layout shifts off its sonic4 twin** (Config-B, demo): they reference `Game_Entry` /
`ErrorHandler` folded to the sonic4 shape, and those folds only become per-config-correct
when the flip nativizes them (rows 52/90). So one of:

1. **Move Config-B + demo proofs POST-flip** (after rows 52/90/6/58 nativize the folds);
   keep sonic4 (2) + Config-A as the PRE-flip gate (Config-A needs blocker 3 first).
2. **Land the row-52/90 nativization as a PRE-flip precursor** (an aeon change making
   `Game_Entry`/`ErrorHandler` resolve per-config natively), then re-run the preamble.
3. **Demo lockstep additionally needs blocker 2** (engine `.emp` game-config
   parameterization) regardless — a Fable/aeon design call the memory already flagged
   ("Fable calls" on demo).

Blocker 3 (align recompute) is an independent sigil-linker task that unblocks Config-A
alone; it does not touch blockers 1/2.

## The valve

Additive/refactor only — sonic4's five native gates + the whole strict suite stay green
from BOTH environments (2919/0/1); asl default byte-identical; zero aeon commits. The
three blockers are surfaced with byte-level evidence rather than worked around. The
off-canonical DRIVERS are BUILT (the profiles + the frozen chainer are the head start);
the six-proof matrix WAITS on the overseer's sequence ruling. The flip commit remains
the overseer's.
