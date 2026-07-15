# Spliced index register in asm-template EAs + frame_piece_count (first consumer)

**Branch** `spliced-index-register` (sigil language feature + aeon consumer), NOT
merged — Volence gate. Small TDD'd language task (NOT a port); closes gap-ledger
row 1031 (t13 second-retrospect).

## The gap

An `asm{}`-template indexed EA `d(An,Xn)` accepted a spliced/evaluated BASE
register (via `ind_single_reg`) but rejected a spliced INDEX register — so a
comptime-fn helper emitting `move.w DISP({base},{off}), dst` with `off: Reg`
failed `indexed addressing needs a valid index register`. This blocked the
`frame_piece_count` shared helper (load_object + animate spell the identical
`move.w FRAME_PIECE_COUNT(base,off.w),dest` with a duplicated "+4 bbox" comment).

The ratified diagnosis proposed two sub-fixes: **A1** (parser accepts a size
suffix after a spliced index) + **A2** (eval resolves a spliced index Reg).

## What was actually needed (a refinement of the diagnosis)

**A2 was the real fix.** `map_an_indexed` (eval/asm.rs) now mirrors
`ind_single_reg`: a literal register spelling (`d3`) resolves without eval;
anything else (`{off}` lowers to `Path([off])`, a const, an expr) evaluates and
must yield `Value::Reg`, else the clean "needs a valid index register" error.
All 3 core tests fail without it and pass with it (verified by stashing the change).

**A1 was SHAPE-SPECIFIC — not the blanket "`{off}.w` doesn't parse" the row
claimed.** With a NUMERIC displacement (`4({base},{off}.w)`) the size already
parsed (paren_operand → trailing_size). The `"expected \`)\`, found Dot"` only
fires with a SYMBOLIC disp (`FRAME_PIECE_COUNT(...)`), which parses as an
`Expr::Call` whose args go through `arg()→expr()`: a literal `d3.w` is pre-folded
into `Path["d3","w"]` by `path()` (paths eat their own dots), but a splice
`{off}` returns bare `Path["off"]` and `postfix_expr` deliberately breaks on the
size letter (leaving it for the size-suffix rule) — so the `.w` dangles and the
call-arg loop rejects it. FIX: `fold_spliced_index_size` (parser.rs) re-folds an
adjacency-guarded `.b`/`.w`/`.l` after a spliced Path onto its last segment — the
SAME `Path[...,"w"]` shape `split_size_suffix` already decodes — scoped to
`splice_ctx`. The numeric-disp `paren_operand` path is untouched.

The exact consumer shape (`FRAME_PIECE_COUNT({base},{off}.w)`) failed to parse
before A1 (verified RED: "expected `)`, found Dot") and emits the correct bytes
after A1+A2.

## Tests (TDD, `crates/sigil-frontend-emp/tests/indexed_splice.rs`, 5)

1. `spliced_index_register_resolves_via_eval` — `4({base},{off})` no size → `30 33 30 04` (A2 RED→GREEN).
2. `word_size_suffix_after_spliced_index_parses` — `{off}.w` (numeric disp) → word index.
3. `long_size_suffix_after_spliced_index_selects_long_index` — `{off}.l` → `30 33 38 04` (bit 11 set).
4. `non_register_index_errors_cleanly` — a const int in the index slot → clean error, no panic.
5. `symbolic_disp_with_spliced_base_and_index` — the exact consumer shape `FRAME_PIECE_COUNT({base},{off}.w)` → `36 33 30 04` (A1 RED→GREEN).

## First consumer (byte-neutral)

New `engine/objects/frames.emp` — `pub comptime fn frame_piece_count(base, off,
dest) -> Code` (aabb.emp pattern: 0 procs, zero bytes anywhere). Adopted:
- `load_object.emp:80` and `animate.emp:276` replace their inline
  `move.w FRAME_PIECE_COUNT(base,off.w),dest` + the duplicated "+4 bbox" comment
  with a bare `frame_piece_count(...)` call.
- `FRAME_PIECE_COUNT` dropped from both files' `use engine.constants.{...}` (now
  imported by frames.emp — single source of truth for the +4 offset).
- The AS twins (load_object.asm, animate.asm) STAY inline — the byte reference,
  aabb.inc-style twin scaffolding. **Reference ROMs UNCHANGED** (no .asm touched).

`children.asm:22` also has an indexed `FRAME_PIECE_COUNT(a0,d0.w)` — a future
t14/t15 consumer once children ports (confirms the helper generalizes).

## Gates

- `indexed_splice` 5/5; each new production change RED-verified first.
- `load_object_port` / `animate_port` / `mixed_dac_rom` (tranche9) byte gates GREEN
  both shapes — byte-identical to the unchanged reference ROMs (adoption is
  byte-neutral).
- Full workspace strict vs the branch aeon tree: **2218 / 0** (2213 + 5 new).
- clippy clean; `repin --check` clean (no pins moved).

## Harness note

`mixed_dac_rom`'s animate arm + `load_object_port`/`animate_port` gained the
`frames.emp` ambient prepend (new `frames_ambient_items`, after
`constants_ambient_items` so `FRAME_PIECE_COUNT` resolves). No pin/provenance
change — the feature and its adoption are byte-neutral.
