# Plan 7 #9b — script/yield MVP: implementation notes

Branch plan7-item9 (worktree .worktrees/plan7-item9). Plan:
docs/superpowers/plans/2026-07-08-spec2-plan7-item9b-script-yield.md.
RED evidence recorded per task, per the 2026-07-08 here-fix precedent.

## T1 — script parses (decl, `loop`, `yield`, `shows`)

RED evidence:
- `cargo test -p sigil-frontend-emp --test script` failed to COMPILE (the AST
  types the tests reference do not exist yet):
  ```
  error[E0433]: cannot find `ScriptStmt` in `ast`
  error[E0599]: no variant, associated function, or constant named `Script`
    found for enum `Item` in the current scope
   --> crates/sigil-frontend-emp/tests/script.rs:64:45
  ```

GREEN:
- New tests `script_decl_parses_with_loop_yield_and_shows` and
  `script_requires_encoding_attr` pass:
  `cargo test -p sigil-frontend-emp --test script` → 2 passed; 0 failed.
- Whole crate: `cargo test -p sigil-frontend-emp` → 46 suite-result lines,
  all ok, 0 failures. `cargo clippy -p sigil-frontend-emp --all-targets --
  -D warnings` clean.

Implementation:
- ast.rs: `ScriptDecl` / `ScriptStmt` (Asm/Loop/Yield) / `ScriptLabel` added
  after `ProcDecl`; `Item::Script(ScriptDecl)` variant after `Item::Proc`.
- parser.rs: `item()` dispatches `script` after `proc`; `"script"` added to
  the `OPENERS` recovery const (16→17; unconditional opener like `proc`, no
  lookahead special-case — verified the `ensure` guard branch is unaffected).
  New `script_decl` / `script_label` / `script_body`. `asm_body`'s loop
  interior factored into `asm_stmt(splices_allowed) -> Option<AsmStmt>`,
  called by BOTH `asm_body` and `script_body` (behavior-identical for procs;
  whole-crate suite green confirms).
- lower/mod.rs: `// #9b Task 2` breadcrumbs on the wildcard arms of the
  top-level loop and `lower_section_items` (where the desugar will hook).
  `Item::Script` is inert everywhere else (falls into existing wildcards /
  `if let` guards in resolve/eval).

### T1 review fold-in (fix-first: 1 Important + 3 minors)

1. **Loop-nesting depth guard (Important).** `script_body`'s `loop` arm
   recursed unguarded. RED evidence, empirical (pre-guard code):
   - New test `deep_loop_nesting_is_an_error_not_an_abort` (600 nested
     `loop {`) failed `assertion failed: !diags.is_empty()` — 600-deep
     recursion parsed silently, no diagnostic.
   - At reviewer scale (temporary 50k-nesting probe, deleted after use):
     ```
     thread 'probe_50k_nested_loops' has overflowed its stack
     fatal runtime error: stack overflow, aborting
     (signal: 6, SIGABRT: process abort signal)
     ```
   Fix: the `loop` arm is now gated on the SAME `block_depth` counter /
   `MAX_EXPR_DEPTH` ceiling as `stmt_block` (increment on entry, ceiling →
   "block nesting too deep (max 128)" + consume-to-balanced-`}`, decrement
   on exit). `stmt_block`'s recovery scan was lifted verbatim into a shared
   `skip_unparsed_block` helper used by both (pure move; the parser_bodies
   deep-nesting tests guard the stmt_block side). GREEN: the 600-nesting
   test passes (one depth diagnostic, no flood, following `const GOOD`
   still parses); the re-run 50k probe printed `diags: 1 — first:
   Some("block nesting too deep (max 128)")` and exited cleanly.
2. **`yield` line-end parity (minor).** `expect_line_end()` →
   `expect_line_end_or_rbrace()` (the instruction-line rule), so
   `{ yield }` parses like `{ nop }`. RED: `yield_tolerates_same_line_close`
   failed with `expected end of line` plus a cascading
   "expected `}`, found Eof". GREEN: passes.
3. **Construct-neutral encoding-attr wording (minor).**
   `dispatch_encoding_attr` is shared with `script`, so its two messages no
   longer say "dispatch": "this declaration requires an `(encoding:
   word_offsets | long_ptrs)` attribute" / "expected `encoding:` in the
   attribute list". No test pinned the old wording (verified:
   `dispatch_requires_encoding` and `script_requires_encoding_attr` only
   check for "encoding" / the encoding names).
4. **`param_list` extraction (optional — done).** The proc/script param
   loops were byte-identical including the surrounding paren expects — a
   clean lift into `fn param_list(&mut self) -> Vec<(String, Type, Span)>`
   used by both `proc_decl` and `script_decl`.

Post-fold-in: `cargo test -p sigil-frontend-emp --test script` → 4 passed;
whole crate → 46 suite-result lines, all ok, 0 failures; clippy
`-D warnings` clean.

## T2 — `lower/script.rs`: desugar + hidden resume table + body lowering

### Probe (mandated design evidence — throwaway, deleted before commit)

Parsed `proc p (a0: *S) { jbra done\n jbra .top\n move.w #2, $20(a0) }` and
Debug-printed the InstrLines. The synthesis mirrors these EXACT shapes:

- `jbra done` (global ident): `Instr(InstrLine { mnemonic: [Text("jbra")],
  size: None, operands: [Plain { expr: Path(Path { segments: ["done"] }),
  size: None, span }] })`.
- `jbra .top` (dot-local): identical, but the segment string KEEPS the leading
  dot — `Path { segments: [".top"] }` (NOT `"top"`). So the loop-back / local
  epilogue jbra is built as `Path([".__loop$0"])` / `Path([".<name>"])`.
- `move.w #2, $20(a0)`: `Instr(InstrLine { mnemonic: [Text("move")],
  size: Some(Text("w")), operands: [ Imm(Int(2, span)),
  DispInd { disp: Int(32, span), inner: Ind { parts: [(Path(["a0"]), None)],
  size: None, span }, span } ] })`. The displacement is a NUMERIC `Int` (the
  field offset), so the yield store is independent of bare-field-access rules.

`Reg` names print `a0`..`a7`/`d0`..`d7` (inverse of `from_name`); an address
register is detected by the `'a'` first byte of that spelling.

### Owner / module-string (R9b.11 — one source of truth)

`eval_asm_owned` (eval/asm.rs:56–59) builds `Owner::Proc { module:
self.module_id, name }` where `module_id = file.module.path.segments.join(".")`
(eval/mod.rs:453). `lower_script_item` computes the SAME `Owner::Proc { module,
name: script_name }` and derives each resume row's final name via
`Owner::local_symbol("__resume$k")` → `$<module>$<script>$__resume$<k>`. The
table's `DispatchTarget::Label(Expr::Str(final_name))` passes through
`eval_dispatch_with_root`'s Str arm verbatim, so the row targets exactly the
symbol the body's `__resume$k` label definition renames to. `local_symbol` was
exposed `pub(super)`; `proc::ends_in_terminator` likewise (was private).

### RED (before implementation)

`cargo test -p sigil-frontend-emp --test script` — the 4 T1 parse tests passed;
the 6 T2 tests all failed (Item::Script fell into the lowering wildcard → only
the trailing `proc done` emitted, e.g. byte-exact tests got `[0x4E,0x75]` vs the
full 18-byte image; the diagnostic tests got `msgs: []`).

### Byte-vector verification

Probes A/B/C reproduced EXACTLY as hand-derived in the plan — no expectation was
touched:
- A (word_offsets, one yield): `00 04 00 0E 4E 71 31 7C 00 02 00 20 60 02 4E 75 4E 75`.
- B (long_ptrs, ×4 ordinal WORD): `00 00 00 08 00 00 00 12 4E 71 31 7C 00 04 00 20 60 02 4E 75 4E 75`.
- C (loop → `__loop$0` + `jbra` back = `60 F4`): `00 04 00 0E 4E 71 31 7C 00 02 00 20 60 02 60 F4 4E 75`.

### GREEN

`cargo test -p sigil-frontend-emp --test script` → 10 passed (4 T1 + 6 T2), 0
failed. Whole crate: 52-suite run, all ok, 0 failures. `cargo clippy -p
sigil-frontend-emp --all-targets -- -D warnings` clean. Workspace: the only red
is the pre-existing `full_build_reproduces_sound_driver_regions` (`strlen()`
builtin in the sound-driver corpus) — confirmed failing on a clean stash of this
work, i.e. an allowlisted red, not introduced by T2.

### T2 fold-in (spec review): `Item::Script` resolver arms

The review found the ONE contract violation in T2: `Item::Script` was absent
from `resolve/imports.rs`, which only the PROGRAM path (`build_program`, i.e.
CLI `--root`) exercises — the unit-test `lower_module` harness bypasses the
resolve pass entirely, which is why T2's byte tests could not catch it.

1. **`collect_defined` (defined-names map):** a script's hidden table SELF-
   references its base label (`dc.w resume_k - name` rows), so without a
   Script arm ANY script — even unreferenced — failed `report_unresolved`.
2. **`item_pub_name` (pub exports):** `pub script` exported nothing,
   contradicting R9b.8 ("pub script exports it like pub dispatch").

Fix: `ast::Item::Script` arms mirroring the adjacent `Dispatch` arms in both
functions. (Parity check: `resolve/mod.rs`'s injectable-item list is the
comptime TYPE-injection channel — Proc/Dispatch/Offsets are absent there too,
so no third arm is needed.)

Regression tests (the established item-4 program-path pattern:
`crates/sigil-cli/tests/module_resolution.rs`, CLI + tempdir + `--root`,
byte-pinned like the cross-module dispatch tests):
- `script_compiles_unreferenced_under_program_path` — solo module, script
  unreferenced; out.bin = the 18-byte Probe A image. RED (pre-fix):
  `m.emp:1:1: unknown symbol `brain``.
- `cross_module_pub_script_resolves_via_use` — `pub script brain` in
  `engine`, entry `obj` does `use engine.{brain}` + `jmp brain`; 24 bytes =
  jmp abs.w `4E F8 00 06` + 2-gap + the Probe-A image verbatim at LMA 6 (the
  script image is position-relative: RelOffset rows + short jbra). RED
  (pre-fix): `module `engine` has no `pub` name `brain`` + two cascading
  unknown-symbol errors.

GREEN: both new tests pass byte-exact; `cargo test -p sigil-frontend-emp`
(46 ok suites) and `-p sigil-cli` fully green; clippy `-D warnings` clean on
both crates.

## T3 — coverage: overrides, hygiene-across-yields, guards, edges

Ten tests added (Task 3's numbered list), same `SCRIPT_TYPES` const + `lower`/
`msgs`/`linked_bytes` helpers reused from T2. First-run result: **9 of 10
green immediately**, exactly as the plan predicted (the spec review had
already adversarially probed most of this ground); one needed a byte
derivation correction (arithmetic bug in the plan writer's own hand-derivation,
not a code bug).

First-run green (9): `yield_per_site_epilogue_overrides_shows`,
`nested_loops_get_distinct_labels`, `user_label_crosses_yield_boundary`,
`zero_yield_script_emits_entry_only_table`, `script_in_z80_section_errors`,
`script_under_as_compat_silences_fallthrough`, `ambiguous_resume_slot_errors`,
`resume_width_errors`, `comptime_call_inside_script_expands`.

- **1 (overrides byte-exact):** confirmed the reviewer's quoted reference
  point verbatim — `yield other` with `done`/`other` procs after the script
  produces jbra `60 04` (target `other` at +18, PC+2=14, disp=4). Table stays
  `00 04 00 0E` (identical layout to Probe A up to the resume label — only the
  jbra's target differs).
- **3 (nested loops byte-exact):** confirmed the reviewer's quoted shape —
  inner back-edge `60 F4` (disp −12, target the coincident loop-top offset 4),
  outer back-edge `60 F2` (disp −14, same target, one instruction further downstream).
  Full 20-byte image hand-derived and pinned (stronger than the plan's
  "byte length or full bytes" minimum bar).
- **5 (zero-yield byte-exact):** reproduced the plan's quoted reference exactly,
  `00 02 4E 75` — a 1-row table (entry only) + `rts`.
- **10 (comptime-in-script):** zero yields, so it collapses to the same
  `00 02 4E 75` shape as #5, sourced from the comptime fn's `asm { rts }`
  expansion instead of a literal `rts` — pins that `epi()` on its own
  statement line inside a script body goes through the same asm-instantiation
  path dispatch's inline bodies use (asm_counter threading unaffected by the
  script desugar prepending the entry label).

**Needed a derivation fix (1): `yield_local_epilogue_resolves` (#2).**
First-run assertion `linked_bytes(&module).len() == 16` FAILED: actual was 18.
Root cause (not a script-lowering bug): `.fin` is defined immediately after
`yield .fin`'s synthesized `jbra`, at the SAME offset as the `__resume$1`
label (both zero-width, coincident). At the jbra's rung-0 (`bra.s`, 2-byte)
trial encoding, the displacement bottoms out to exactly 0 — the reserved
0x00-disp-byte escape that signals "read the following word instead," which
is UNENCODABLE as a short branch. `sigil-link`'s own relax ladder already
pins this exact rule (`relax.rs::ladder_skips_bra_s_on_disp_zero`), so
relaxation correctly promotes to rung 1 (`bra.w`, 4 bytes), adding the 2
bytes the naive hand-count missed. Fixed the test to assert `len() == 18`
with the full derivation (including the disp-0 escape citation) shown in the
test's doc comment — no code was touched; this is a real, previously-tested
linker behavior surfacing through a new caller, not a script-specific defect.

**Verification:** `cargo test -p sigil-frontend-emp --test script` → 20
passed, 0 failed (10 from T1/T2 + 10 new). Whole crate: all suites green,
`cargo clippy -p sigil-frontend-emp --all-targets -- -D warnings` clean.

No DONE_WITH_CONCERNS items — the one failure diagnosed to a correct,
independently-pinned linker behavior, not a script lowering defect.

## T2/T3 fold-in: yield-site spans on synthesized instructions

Quality review finding (2026-07-08, post-T2): `yield_store` and `jbra`
(script.rs's synthesized-instruction helpers) stamped every `InstrLine` /
sub-expression with `sp()` — a zero span (`SourceId(0)`, `0..0`) — meaning a
link-time `[branch.out-of-reach]` on an epilogue `jbra` or a loop back-edge
would render at byte 0 of source file 0 instead of pointing at the `yield` or
`loop { }` that produced it. The `sp()` docblock claimed "no user-visible
diagnostic path," which the review showed was wrong (the out-of-reach case is
real for a script with enough body between a yield and a far-away epilogue,
or a huge loop body).

Fix: `desugar_yield` and `desugar_loop` already receive the statement's real
`Span` (the yield's span for resume-label placement; loop now also threads
its own span via `ScriptStmt::Loop { span, .. }`, previously discarded with
`..`). Both are now passed all the way through:
- `yield_store(ordinal, offset, reg, span)` — the `move.w` `InstrLine` and
  every sub-expression/operand (`Imm`, `DispInd`, the inner `Ind`, `path_expr`
  for the register) carry the yield's span.
- `jbra(target, span)` — the `InstrLine`, its `Operand::Plain`, and its
  `path_expr` carry the caller's span (yield's span for the epilogue exit and
  the no-epilogue-error resume label path; the loop STATEMENT's span for the
  back-edge and the hidden loop label).
- `path_expr(seg, span)` picked up a `span` parameter (both call sites had a
  real span available once the above threaded through), which let `sp()` be
  deleted outright — nothing else in the file used it.

No byte or behavior change (spans do not affect emitted bytes): confirmed via
the full T1–T3 script suite (20/20 green, byte-exact tests unchanged) and the
whole-crate suite (all green), plus `cargo clippy -p sigil-frontend-emp
--all-targets -- -D warnings` clean.

## Deferred (rule-of-three)

The table-emit shape — evaluate a synthesized/parsed declaration → 
`stream_data` → `builder.define_label(name)` → `builder.emit_data(bytes,
fixups, span)` — now has **three** verbatim (or near-verbatim) instances:
`lower_offsets_item`, `lower_dispatch_item` (the table half), and
`lower_script_item` (the hidden resume table half, step 5). Per the house
rule-of-three, this is now warranted for extraction into a shared helper
(something like `fn emit_streamed_table(bytes_result, placement, name,
builder, diags)`). Deferred to the checkpoint conversation rather than done
opportunistically here: `lower_script.rs` also has its own local variations
(the table's `DispatchDecl` is itself synthesized, not parsed, and the
Task-2 T2-fold-in already touched `lower/mod.rs` resolver arms once this
session) — a clean extraction deserves its own reviewed diff rather than
riding along inside a coverage-tests-plus-span-fix commit. Flagged here per
the quality review (2026-07-08) for the controller to schedule.

## T4 — game prelude: ScriptPc + the resume slot (controller-run)

- `pub newtype ScriptPc = u16` added above Sst; `routine: u16 @ $20` → `resume: ScriptPc @ $20`
  (same offset, same width); Player_1 literal field renamed; `routine` HELPER keeps its name
  (D9.5 — the manual spelling) and now stores to `Sst.resume(a0)`, doc updated.
- Byte-neutrality verified: pitcher_plant_acceptance passes UNCHANGED (2 tests, 340-byte pin
  intact) and the CLI build reports 340 bytes. The proc exhibit's `routine shoot`/`routine wait`
  call sites are untouched.

## T5 — the exhibit + equivalence

New sibling module `examples/game/badniks/pitcher_plant_script.emp` (module
`badniks.pitcher_plant_script in obj_bank`): the SAME badnik as
`pitcher_plant.emp`, brain rewritten as ONE `script brain (a0: *Sst)
(encoding: word_offsets) shows Draw_Sprite`. Own `vars PitcherPlantV`/consts/
`offsets Ani`/ani data/`pub data Def`/`SeedDef`; `seed` stays a `proc`
verbatim (a seed is a separate object, not a plant state). The proc version is
UNTOUCHED (its 340-byte acceptance pin remains the R9b.7 regression guard).

Build (verified, clean): `cargo run -p sigil-cli -- emp
examples/game/badniks/pitcher_plant_script.emp --root examples/game --prelude
prelude` → exit 0, ZERO diagnostics, **358 bytes**. Pinned by
`crates/sigil-cli/tests/pitcher_plant_script_acceptance.rs`
(`script_exhibit_builds_clean_and_pins_hidden_table`).

### State mapping (proc → script)

| proc version | script version |
|---|---|
| `init` (`move.b #WAIT_TIME, timer(a0)` then `falls_into wait`) | the ENTRY segment (before `loop`); `falls_into` is gone — the entry segment simply flows into the loop |
| `wait` (per-frame countdown, `routine wait` re-store) | the `.wait_tick` resume loop: `subq.b #1, timer(a0)` / `beq .check` / `yield` / `jbra .wait_tick` |
| `shoot` (per-frame windup countdown + FIRE_FRAME spawn, `routine shoot`) | the `.windup_tick` resume loop: `subq` / `cmpi.b #FIRE_FRAME` / `spawn(...)` / `tst.b` / `beq .rearmed` / `yield` / `jbra .windup_tick` |
| `routine wait` / `routine shoot` (manual `pea`/`move.w (a7)+, Sst.resume(a0)`) | `yield`'s synthesized `move.w #<ordinal×2>, resume(a0)` — the SAME `resume @ $20` slot, D9.5 made literal |

The proximity check (`Player_1.x_pos` / `sub` / `facing_abs` / `cmp
#ATTACK_RANGE` / `bhi .rearm`) and the `.rearm`/`.rearmed` re-arm (anim Idle,
reset timer) fold inline into the one loop body. `.wait_tick`/`.check`/
`.windup_tick`/`.no_fire`/`.rearmed`/`.rearm` are ordinary proc-local labels
that resolve across the yield boundaries for free (single flattened body,
R9b.11).

### The hidden table (verified)

`brain` labels the table at obj_bank offset 0x40 (= `Def.code`'s fixup value).
3 bare `yield`s → 4 rows, word_offsets → 8 bytes:

| row | bytes | offset from base | resume point |
|---|---|---|---|
| 0 (entry) | `00 08` | +8 | `move.b #WAIT_TIME` (entry segment, = table width `2*(1+3)`) |
| 1 | `00 1E` | +30 | `.wait_tick`'s `jbra .wait_tick` back-edge (bra.s `60 EE`) |
| 2 | `00 6C` | +108 | `.windup_tick`'s `jbra .windup_tick` back-edge (bra.s `60 D2`) |
| 3 | `00 82` | +130 | the `.rearm` tail yield's loop-top back-edge (bra.s `60 8A`) |

Each yield's resume point is the `jbra` immediately AFTER it — so on the next
frame the engine indexes the table with the stored ordinal, lands on the
back-edge, and re-enters the countdown/loop-top. Rows are strictly increasing
(a straight-line body invariant the acceptance test checks).

### What differs from the proc version (equivalence caveats)

- **Epilogue:** the proc version has THREE per-proc `jbra Draw_Sprite` tails
  (one each in `wait`/`shoot`/`seed`). The script funnels EVERY frame exit
  through the ONE declared `shows Draw_Sprite` epilogue — `yield` synthesizes
  the `jbra <epilogue>` per site (bare `yield` uses `shows`). `seed`, still a
  proc, keeps its own explicit `jbra Draw_Sprite`.
- **State ids:** the proc version stores a proc ADDRESS (`pea wait`/`pea
  shoot`, low word) into `resume`. The script stores a hidden table ORDINAL
  (`#2`/`#4`/`#6` = index×2) — the engine dispatches through the table rather
  than jumping to a raw address. Same slot, typed (`ScriptPc`), abstracted.
- **Byte size:** 358 (script) vs 340 (proc). The script pays 8 bytes for the
  hidden table plus the per-frame `move.w #ordinal, resume(a0)` stores; the
  proc pays `routine` helper `pea`+`move.w` stores at fewer sites. Not
  byte-identical (nor expected to be — R9b.12 pins the proc version, argues
  the script version by equivalence).

### Authoring friction

NONE. The exhibit compiled clean on the FIRST attempt (exit 0, zero
diagnostics, 358 bytes) directly from the plan's Task-5 sketch — the only
adjustments from the sketch were cosmetic (comment voice, label ordering) and
substituting `beq` for the sketch's flow where the proc version used `bne`-to-
tail (the script's yield-per-frame structure inverts the sense: `beq .check`
"elapsed → act" vs the proc's `bne .draw` "not elapsed → just draw"). No
compiler defect surfaced; every construct the plan promised (statement-position
`anim`/`spawn`/`facing_abs` each on their own line, `yield` bare + labels
across yields, `loop`) worked end-to-end exactly as `tests/script.rs`
predicted.

## T7 — gate + byte-diff probes vs master (controller-run)

- `cargo test --workspace --no-fail-fast`: exactly the 4 allowlisted sigil-harness reds
  (aeon sound-driver strlen drift), ZERO new failures. `cargo clippy --workspace
  --all-targets -- -D warnings`: clean.
- Byte-diff probes vs master (359d9cd): pitcher_plant.emp (--root examples/game --prelude
  prelude, 340B) BYTE-IDENTICAL; standalone corpus dispatch/guards/offset_table/
  sst_overlay/reach_branches ALL BYTE-IDENTICAL. The script exhibit (358B) exists only
  on-branch; its image head was structurally hand-verified by the controller (table rows
  0008/001E/006C/0082 at the script base; WAIT_TIME entry store; 317C 0002 0020 yield
  ordinal store into resume @ $20; 6000 00AA epilogue jbra; 60 EE loop back-edge).
- Spec §5.6 + D2.24 + §10 inventory drafted in the empyrean WORKING TREE (uncommitted,
  Volence's cadence).

9b complete on branch plan7-item9. Commits: T1 c9e7a0b (+c867e94 fold-in: loop-depth
guard [SIGABRT→diagnostic], yield line-end parity, neutral encoding wording, shared
param_list), T2 c6ba7af (+53b8595 fold-in: Item::Script resolver arms — the --root gap
the adversarial spec review caught), T3 66c48a8 (10 pins + yield-site spans), T4 3d52eee
(prelude ScriptPc/resume), T5 f99da8e (exhibit + pin), docs 25c7a21/21d652f. Reviews:
T1 spec ✅ + quality fix-first→folded; T2 spec ❌→fixed→program-path regression tests +
quality ✅ approve (2 minors: spans folded into T3; rule-of-three table-emit extraction
DEFERRED to checkpoint). NEXT: whole-branch adversarial review (9a+9b) → Volence checkpoint.
