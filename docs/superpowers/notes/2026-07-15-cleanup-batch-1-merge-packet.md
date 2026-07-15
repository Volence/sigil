# cleanup-batch-1 — merge packet (Fable's gate)

Branch `cleanup-batch-1` (both repos). The post-t15 dedicated cleanup wave
(Volence-directed): the retro macro-interface retrofits + the enumerated
at-next-touch backlog. Everything except the one probed jsr-site is byte-neutral
and gate-proven.

**Canonical gate-off ROMs UNCHANGED** — rebuilt both shapes in the branch
worktree: plain `s4.bin` 452500 / crc32 **5a47851a**, debug `s4.debug.bin`
460521 / **b0ceca0b** (both MATCH post-t15 canonical). Only `.emp` files changed
in aeon (23 files, zero `.asm`), so the gate-off asl build is definitionally
untouched — proven, not assumed.

## Contents

| Item | What | Ledger row closed |
|---|---|---|
| 1 | `vram_art` typed retrofit (objdef.emp) + the evaluator feature it demanded | 1044 (retro macro-interface sweep) |
| 2 | rings.emp RING_ART_ATTR → `vram_art(...)`; shared-home relocation deferred | 1044 |
| 3 | RomPtr +1 demand data point (ojz_sec `dict`) — jot only | new jot row (under 1044) |
| 4 | aabb.emp:62 `bpl.s` → bare; :70 kept + commented | 1045 (step-2 retro conformance) |
| 5 | game_loop.emp:28 `jsr Debug_MusicToggle` — kept, real reason named | 1045 |
| 6 | paren-width sweep (core operands + 43 comment vestiges) | 1045 |
| 7 | codename comment scrub (59 sites / 19 files) | 1046 (codename-narration) |
| 8 | contract-reglist range conversion (12 sites) | 1057 (enumerated reglists) |

## Gate artifacts (paired-state discipline)

- **Full workspace strict suite**: `AEON_DIR=<worktree> SIGIL_STRICT_GATE=1
  cargo test --workspace` → **193 ok suites, 0 failed** (AEON_DIR pointed at
  `aeon/.worktrees/cleanup-batch-1`, per the paired-state gate).
- **clippy**: `cargo clippy --workspace --all-targets` → clean (0 warnings).
- **repin --check**: `pins.rs unchanged` (item 5 kept `jsr`, so NO re-pin — the
  branch changes no pinned bytes).
- **Per-file byte gates** (both shapes where applicable): objdef_port (8, incl.
  the new negative test), rings_port (5), aabb via rings/collision/animate,
  game_loop_port (3), core_port (4), sprites_port (4), controllers_port (2),
  collision_lookup_port (2), vdp_init_port (2), hblank_port (2), section_port
  (2), entity_window_port (4), sound_api_port (2), dplc_port (3), mixed_dac_rom
  (24) — all green.
- **Gate-off neutrality**: rebuilt plain + debug, CRCs unchanged (above).

## Item detail + the ⚠ GATE FLAGS (decisions that go beyond a mechanical fix)

### Item 1 — vram_art typed + a DEMANDED evaluator feature ⚠

`vram_art` now takes `tile: int where 0..$1FFF, pal: int where 0..3 = 0,
pri: int where 0..1 = 0` and returns `VramArtTile`.

**The refinement was not enforceable — I had to build the feature.** The step-0
premise (ledger row 1044) was that this is "comptime/erasing → byte-neutral,
cheap by byte gate." True for the bytes, but the *totality* check the retrofit
is FOR did not exist: comptime-fn param `where` bounds were never enforced. Proof
it was a real gap, not a mis-config — the sibling `objdef priority: u8 where
0..7` "compile error" test passed only because `8 << 5` overflows the u8
`render_flags` field, NOT because of the refinement; `vram_art(pal=4)` produces
`$8000|tile` (fits u16), so nothing caught it. TDD confirmed red first.

Built it minimally (`sigil-frontend-emp`):
- `call.rs::call_fn_with_values` — for each param whose `ast::Type` is
  `Refined(_, lo, hi)`, range-checks the bound int via the existing
  `check_in_range` (same mechanism as newtype-construction bounds). Bare
  `int`/`u8`/Reg/Label params are untouched (loosely typed as before) — the
  check only fires on refined params.
- `layout.rs::eval_const_index` promoted to `pub(crate)` so the caller can eval
  the bound exprs. (Note: bare `int` is not a resolvable layout `Ty`, so I did
  NOT route through `resolve_type` — matching `ast::Type::Refined` directly
  avoids a spurious "unknown type: int" on the `int` inner.)
- Tests: `objdef_port::vram_art_pal_over_3_is_a_compile_error` (the TDD driver),
  plus `eval_fns::param_refinement_{in_range,out_of_range,default_in_range}`
  pinning the feature at the evaluator layer.

Full frontend-emp + workspace strict suites stayed green with the new check in
the hot path of every comptime-fn call — no existing call regressed.

### Item 2 — rings adopts vram_art; DECISION: defer the shared home ⚠

rings.emp `RING_ART_ATTR = vram_art(VRAM_RING_PLACEHOLDER, 1, 1)`, importing
`vram_art` cross-module (the established `aabb_axis_test` precedent). The AS twin
keeps its inline hand-pack (its isolated gate sees no macros.asm; lockstep is
byte-level) — byte-identical, rings_port green.

**Decision (row 1044 asked me to name it): shared-home relocation DEFERRED to
the VRAM-layout port**, not relocated piecemeal now. Rationale: types.emp already
declares the VRAM-layout port as the home of the whole art/VRAM family (VramTile,
vram_bytes ×32); relocating a lone 3-line fn now would prejudge a module boundary
(engine.vdp? engine.art?) that port will settle, and it matches row 1051's
"consolidate at the porting wave" pattern for the VDP macros. objdef.emp's
"relocate at the second consumer" comment was amended to say this.

Test-harness note: rings' cross-module `vram_art` import needed objdef added to
the rings ambient in BOTH `rings_port.rs` and `mixed_dac_rom.rs` (the latter
caught by the full strict suite — tranche8/9 mixed ROMs lower rings.emp; a new
`objdef_ambient_items` helper mirrors `aabb_ambient_items`).

### Item 4 — aabb: one bare, one pinned-with-comment ⚠

- `:62 bpl.s .aov` → **bare** `bpl .aov`. `.aov` is a LOCAL label 1 instruction
  ahead → unconditionally relaxes to `.s`, byte-identical. The corpus uses bare
  Bcc inside `asm{}` templates *everywhere*, including splice-hole targets
  (`collision.emp:41 beq {skip}`); animate:54's pin cites "twin has explicit
  width", which the 2026-07-11 port-loop clarification rules is NEVER a valid
  exception.
- `:70 bhs.s {mlab}` — the ledger flagged only :62, but :70 was ALSO uncommented
  explicit-width. It is a GENUINE structural pin: `{mlab}` is a caller-supplied
  splice hole whose reach the template can't guarantee, so the `.s` locks the
  macro's near-target contract (byte-locked to aabb.inc's `bhs.s mlab`; a far
  target must fail loud, not silently widen). KEPT + given its exception comment
  (closes the checklist-item-2 gap the ledger's "ONLY uncommented" wording
  missed).

### Item 5 — game_loop jsr KEPT, real reason named (the only byte-probe) ⚠

Probed empirically (temporary jbsr swap, reverted): `jbsr Debug_MusicToggle`
emits `bsr.w` (`61 00 1f f6`) ≠ the AS twin's `jsr` abs.w (`4e b8 30 00`) —
byte-CHANGING. Also: the site is *elided in every shipped build*
(SOUND_DEBUG_HOTKEYS is off in both pins), exercised only by a synthetic
near-placement matrix test, so there is no shipped-ROM truth for it.

**Decision: keep `jsr`, don't convert.** The underspecified "placement-free"
comment is replaced by the real structural reason: this line byte-mirrors
game.asm's `gameDebugTick` macro body (a kill-list binding — "this file must
follow that macro body") AND `Debug_MusicToggle` is a GAME-side symbol
(games/sonic4/debug/), so an engine→game **cross-seam** call uses absolute `jsr`
to stay placement-independent — `jbsr`→`bsr.w` would PC-relative-couple the
engine to a specific game's debug-section placement, wrong after the engine/game
split. No re-pin, no re-baseline (repin --check confirms pins unchanged).

### Item 6 — the 56 "sites" were mostly a grep over-count ⚠

Classified as the ledger asked. Only `core.emp` had REAL `(Sym).w` OPERANDS
(13): 9 → bare-symbol width-rule spelling (`Dynamic_Free_SP`/`Effect_Free_SP`,
abs.w unchanged, core_port green). **4 KEPT `(Sym).w`** — a `#extern(...)`
link-imm SOURCE combined with a bare (relaxable) symbolic DEST hits the imm-link
lowering gap (`[lower.imm-link] a link-time immediate combined with another
symbolic operand is not yet supported`), an already-ledgered limitation; each
carries that exception comment.

The other **43 were comment-only** — already-bare operands whose trailing
`// (Sym).w — ram.asm; abs.w picked by the width rule` is a pre/mid-ratification
annotation vestige (entity_window, the mature precedent named in the task,
carries NONE). Scrubbed to entity_window's style, preserving semantic tails
(e.g. sprites' `d0 = screen-relative X origin`, vdp_init's I/O-clr-hazard note).
All 7 files' operand-spelling count → 0 (core's 4 are commented exceptions).

### Item 7 — codename scrub, durable anchors kept

62 replacements / 19 files (58 first pass + 1 audit-ref + 3 Fable-gate riders:
`core.emp` "A1 double-dispatch check", `entity_window.emp` "A1-safe today",
`animate.emp` "audit §clobbers-semantics", and a dangling-colon scrub artifact
in `test_solid.emp`). Ephemeral session codenames → the adjacent
behavioral reason. The two recurring *names* got consistent rewordings: `A1` →
the delete-zeroes-the-live-list-entry fix; `A2` → "overflow latch (spec §9)" /
"walk-flag assert rail". KEPT (per the classification rule): `spec §`, `kill-list
row`, `C1 item` (grammar spec), `construct-walk #3` / `R7` (named
design-doc/milestone refs — notes-resolvable), `Volence-ratified/-approved`
provenance. Comment-only.

### Item 8 — reglist ranges

All 12 → the suggested movem-range `/` spellings (the dominant corpus separator).
Byte-neutral. NOTICING-CLAUSE observation (out of this row's scope, logged in the
ledger + here): entity_window's ~30 already-range contracts join with a COMMA
(`clobbers(d0-d7, a0-a3)`) rather than `/` — a separator-consistency candidate
for a future entity_window touch, NOT converted here.

## What each pass added

- **Step-1/demanded-feature bucket**: the comptime-fn param-refinement enforcement
  (item 1) — a genuine language feature the retrofit demanded, TDD-driven, with
  its evaluator + two test-layer artifacts.
- **Step-2 (modernize) findings**: aabb bare-vs-pinned split (item 4); the
  game_loop cross-seam/macro-mirror reason (item 5); core operand bare
  conversion + the imm-link-gap structural exceptions (item 6); reglist ranges
  (item 8). All byte-neutral except the item-5 PROBE (reverted; final state
  byte-neutral).
- **Step-3(b) (reads-wrong) findings**: 43 paren-width comment vestiges + 59
  codename references scrubbed to the mature house style (items 6/7).
- **Neither bucket**: the item-2 shared-home DEFERRAL decision (a design call,
  named not silently defaulted); the RomPtr +1 demand jot (item 3, parked).

## Ledger rows closed (same-wave, per close-out discipline)

1044 (retro macro-interface sweep — items 1/2 + item-3 jot as a new row), 1045
(step-2 retro conformance — items 4/5/6), 1046 (codename-narration — item 7),
1057 (enumerated reglists — item 8). All CLOSED with their outcomes in
`campaign-gap-ledger.md`.

## Fable gate result

**PASS-WITH-NITS** (2026-07-15). Fable re-ran every falsifiable claim against the
repos rather than trusting prose: reverted the item-1 `call.rs` hunk and
confirmed the two refinement tests go RED (red-first is real); re-ran the strict
suite (193 ok / 0 failed), clippy (clean), repin (`pins.rs unchanged`); rebuilt
both gate-off ROMs to the canonical CRCs; empirically re-confirmed the item-6
imm-link block by baring core.emp:63 and observing the exact `[lower.imm-link]`
error; and validated the item-4/5/2/6 judgment calls against the real files. No
misrepresentations found.

Nits (all comment-only) were folded in the same wave as a **rider commit**:
3 A1/audit-ref codename residuals the first pass missed (`core.emp:371`,
`entity_window.emp:1474`, `animate.emp:221`) + a dangling-colon scrub artifact
(`test_solid.emp:11`). Post-rider, the codename count → 0 claim holds (corpus
grep for `\bA[12]\b`/`item N`/`audit §` empty modulo durable anchors). Touched
files re-gated green.

**Merge-ready.** Volence's call on the merge (`--no-ff` both sides + push together
per the paired-state gate).
