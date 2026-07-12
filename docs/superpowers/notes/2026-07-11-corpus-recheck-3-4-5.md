# Corpus recheck — steps 3(a)/3(b)/4/5 over the full ported corpus (Fable)

2026-07-11 · Companion to the hot-path retro review (which covered
core/collision+aabb/rings/animate line-by-line; sprites has its own
addendum). This pass: the remaining 15 files line-audited (dplc,
sound_api, game_loop, hblank, controllers, math, vdp_init, types, sst,
collision_lookup, constants, test_solid, test_particle) plus a
corpus-wide mechanical 3(a) census. Scope note: the four data files
(mt_bank, dac_samples, act_descriptor, the two anims) were shape-audited
by the table survey + constructor spot-checks, not re-line-audited (data
+ ensure blocks, low risk); rings 1-70 (RingBuffer_Add) rides its
already-triple-reviewed file.

## The 3(a) census (escape hatches BY SHAPE, corpus-wide)

| Shape | Count | Reading |
|---|---|---|
| `const`+`ensure(extern(...))` mirror pairs | **~130** (constants 49, sst 30, anims 12, dac 10, mt 9, act 9, sound_api 7, rings 4, sfx 2, collision 1) | THE dominant detour, and GROWING (+15 in t11 alone) |
| `equ` link-folded sums (`extern()+extern()`) | ~15 (sound_api) | fine — values deliberately AS-owned, nothing to drift |
| Transliteration blocks | 2 files (rings, core) | already dying (diagnostics construct) |
| call-expr displacement escape | 1 (`interact_off()`) | documented boundary, no trend |

**ASK #1 (flagship, C1-class): a `mirror` declaration.**
`pub mirror NAME = expr` ≡ `pub const NAME = expr` + the
`ensure(extern("NAME") == NAME, "...")` drift guard, auto-generated.
Kills ~130 duplicated lines, and — the real win — makes the guard
STRUCTURAL: today nothing stops a mirrored const being added WITHOUT its
ensure (the pairing is convention; the counts don't even match 1:1 in
constants.emp/sst.emp — each off-by-one needs a look). Scaffolding-era
feature that dies at Spec 5 with the twins, but the demand grows with
every port between now and then. Recommend: mini-spec + build in the
current construct wave. (Also subsumes sst.emp's double-lock rows.)

## Findings in the newly-audited files

**F1 — `Radius` documentation CONTRADICTION (types.emp:39-41 vs
sst.emp:39-40) — fix now, decide the name.** types.emp says Radius is "a
hitbox HALF-extent"; sst.emp says width_pixels is "collision width
(FULL, not half)"; the AABB math (verified in the retro review: template
tests `2|d| < wa+wb`) proves FULL is the truth. A reader trusting the
type doc halves twice. Minimum fix: correct types.emp's doc. Better:
the name `Radius` itself asserts half-extent — rename the newtype
(`BoxSize`/`Extent`?) or genuinely move the engine to half-extents.
Volence's naming call; doc fix is not optional.

**F2 — Sound_PlaySFX header vs declaration (sound_api.emp:209-211).**
Header claims "Preserves a0/a1/d1-d7/SR"; the enforced decl is
`preserves(d1/a0)` (the rest is incidental untouched-ness). Align the
header to name what's CONTRACT vs what's incidental — callers should
only lean on the enforced set. One-line fix.

**F3 — 4(b) name-the-idiom candidates (do in the construct wave):**
- Z80 window/bank derivation (`addr → bank>>15, (addr&$7FFF)|$8000`) —
  spelled twice in Sound_PlayMusic, again in the DAC class; a
  `z80_window(...)`/`z80_bank(...)` Code-template pair names the
  addressing scheme.
- controllers.emp P1/P2 blocks (6 lines ×2, differ only in port + RAM
  triple) — marginal; take only if the helper reads better at the call
  site (the new 4(b) gate), else record as looked-at.

**Clean bills (line-audited, no findings):** dplc (the perform_dplc
dedup holds up), game_loop, hblank, math (typed Angle param + the
pre-made out-typing ledger note), vdp_init, collision_lookup (register
convention documented), test_solid, test_particle, and sound_api's ring
buffer (the index-bug lesson documented at BOTH applicable sites with
the found-live date, data-before-pointer commit ordering, single
release point — cite alongside RingCollision as the house exemplar).

## Outcome routing
- ASK #1 (`mirror`) → gap-ledger row + mini-spec (Fable) + build in the
  current wave.
- F1/F2 doc fixes + F3 helpers → the follow-up wave agent (byte-neutral
  except F3-if-taken which is byte-neutral too — comptime templates).
- The interrogations' retro run is now COMPLETE across the corpus: every
  ported file has had a 3/4/5-grade look under the new checklists.
