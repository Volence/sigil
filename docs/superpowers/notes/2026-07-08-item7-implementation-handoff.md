# Handoff — Plan 7 #7 (banks/window placement + L-H.1 fix): implementation

Written 2026-07-08 (Fable, post-#9 checkpoint session) for the next implementation session.
Repo /home/volence/sonic_hacks/sigil. Read the workspace CLAUDE.md conventions and memory
[[spec2-progress]] first.

## Where master stands (e53041c, pushed)

- Plan 7 #1–#6, #8, pitcher-plant tranche, here()-fix, AND **#9 (9a+9b)** are ALL MERGED.
  Two standing acceptance exhibits compile end-to-end (both pinned in crates/sigil-cli/tests/):
  `pitcher_plant.emp` (proc version, 340B) and `pitcher_plant_script.emp` (script version,
  358B) — `--root examples/game --prelude prelude`.
- Green gate: `cargo test --workspace --no-fail-fast` → exactly 4 allowlisted sigil-harness
  reds (aeon sound-driver strlen drift: full_build_reproduces_sound_driver_regions,
  vector_table_matches_reference_rom_first_256_bytes, full_debug_rom_matches_assembled_reference,
  full_rom_matches_assembled_reference); clippy --workspace -D warnings clean. Re-verify at
  T0; zero NEW failures ever. No cargo fmt sweeps.
- Empyrean spec current through **D2.24 + §5.6** in the WORKING TREE (uncommitted — Volence's
  docs cadence; do not commit empyrean).

## The task: #7, APPROVED design

Design doc (status APPROVED, decisions D7.1–D7.7 + ledger L7.1–L7.4):
`docs/superpowers/specs/2026-07-08-spec2-plan7-item7-banks-design.md` — READ IT FIRST.
One-line summary: `section name (bank: $8000)` = no-straddle-a-boundary section property
(bump-only-when-straddling placement, always-on generated link assertion); `bankid(Label)`
builtin = link-time value `(Sym & $7F8000) >> 15` on the D2.23 LinkExpr machinery,
un-deferring S2-D13(f) general link-expr DATA CELLS (its first real customer); `winptr`
untouched; ALL of it founded on the **L-H.1 fix** — section bases derive from FINAL
(post-relaxation) sizes, placement⇄relaxation joint fixpoint, pins stay pins, surviving
overlap = loud link error.

## Suggested staging (planner's call — mirror 9a/9b's separable halves)

- **7-pre (the L-H.1 fix):** own tasks, own byte-diff probes. This is the gnarly half —
  sigil-link resolve_layout/place_sections + the baked next_lma chain (lower/mod.rs). The
  s4.bin harness + full examples corpus byte-diffs are the net; any byte divergence must be
  an itemized, argued correction of a previously-silent overlap (here-fix precedent), never
  an unexplained drift.
- **7-main:** `bank:` attr (parse at section_attrs, lower/mod.rs:548-ish; thread to link),
  the generated no-straddle LinkAssert, `bankid()` builtin + LinkExpr data cells, the
  dac_samples exhibit (+ negative straddle probe), docs.

## Machinery you build on (all shipped)

- D2.23 here()-fix: `Value::LinkExpr` + operator lifting (eval/expr.rs), `LinkAssert`
  (Module IR, checked in sigil-link post-resolve), lazily-folded message parts,
  `[here.provisional]` refusals at comptime-required sites. Design doc:
  `specs/2026-07-08-spec2-plan7-here-relaxation-fix-design.md` (D-H.1–D-H.9).
- `winptr` builtin (Plan 4) — windowed SymRef cell; the per-idiom-fixup pattern D7.3
  explicitly REJECTS for bankid (use the general LinkExpr cell instead).
- `--map` region placement + budgets (#4), auto-sequential placement (the #4 review's I3
  fix), `Fragment::RelaxLadder` grow-only fixpoint (#8) — the termination argument D7.4's
  joint fixpoint borrows.
- aeon reference idioms: `games/sonic4/data/sound/dac_samples.asm` (the exhibit's source
  shape), `song_table.asm`, `main.asm:231-241` (the co-location fatals).

## Process (NON-NEGOTIABLE, the standing loop)

Isolated worktree under sigil/.worktrees/ off master; strict TDD with RECORDED RED evidence
(notes file per the item-9 precedent: docs/superpowers/notes/2026-07-08-item9{a,b}-implementation-notes.md
show the format); plan doc(s) with frozen micro-rulings (R7.x) via superpowers:writing-plans;
subagent-driven (opus = gnarly linker/lowering work — the L-H.1 half is ALL opus; sonnet =
mechanical/tests/authoring); commit-per-task; two-stage reviews (spec + code-quality) on
load-bearing tasks; whole-branch adversarial review with byte-diff probes vs master across
examples/ + examples/game AND the s4.bin harness before calling it checkpoint-ready;
controller independently verifies implementer claims; NO merge to master without a Volence
checkpoint. Volence has ratified scope/exhibit/direction — technical calls within D7.1–D7.7
are the implementing session's to make and RECORD.

## Watch-outs

- **The unit-test blind spot that bit #9 twice:** the tests/*.rs `lower()` helper bypasses
  `build_program`/`report_unresolved` (the --root program path) AND resolve_layout runs with
  an empty SymbolTable. For #7 — whose whole point is placement — per-task tests MUST
  include program-path (--map/--root, CLI-invoked) cases, not just single-module lowers.
- --root examples is unusable (four pre-existing `module m` collisions); the game corpus
  root is examples/game/.
- The 4 allowlisted harness reds are UPSTREAM (aeon strlen drift) — do not "fix" them, do
  not let them mask new reds (match on the exact 4 test names).
- L-H.1 touches placement that the AS front-end (s4.bin) shares — byte-identity there is
  the hard bar; the joint fixpoint must degenerate to today's behavior when nothing grows.
- Rule-of-three from #9 stands: extract the shared table-emit helper IF this item touches
  that seam; otherwise leave it.
- After #7: the sound migration is the next big arc (it re-evaluates ledger L7.1 packing
  linker + 9d byte-command DSL); 9c (value yields etc.) needs only a short design note when
  taken up. Spec integration for #7 (a §7.x/D2.25 pass in empyrean's working tree,
  UNCOMMITTED) belongs to Fable at the post-merge checkpoint — flag it, don't do it.
