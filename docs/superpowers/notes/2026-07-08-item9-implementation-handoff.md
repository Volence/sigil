# Handoff — Plan 7 #9 (scripted states / coroutines): 9a + 9b implementation

Written 2026-07-08 (Fable, day checkpoint) for the next implementation session. Repo
/home/volence/sonic_hacks/sigil. Read the workspace CLAUDE.md conventions and memory
[[spec2-progress]] first.

## Where master stands (fd8b59e → 17a2a7e, pushed)

- Plan 7 #1–#6, #8, the pitcher-plant tranche, AND the here()-vs-relaxation fix are ALL
  MERGED. The standing acceptance exhibit compiles end-to-end:
  `cargo run -p sigil-cli -- emp examples/game/badniks/pitcher_plant.emp --root
  examples/game --prelude prelude` → exit 0, zero diagnostics, 340 bytes (byte-pinned in
  crates/sigil-cli/tests/pitcher_plant_acceptance.rs).
- Green gate: `cargo test --workspace --no-fail-fast` → exactly 4 allowlisted
  sigil-harness reds (aeon sound-driver strlen drift: full_build_reproduces_sound_driver_regions,
  vector_table_matches_reference_rom_first_256_bytes, full_debug_rom_matches_assembled_reference,
  full_rom_matches_assembled_reference); `cargo clippy --workspace --all-targets -- -D warnings`
  clean. Re-verify at T0; zero NEW failures ever. No cargo fmt sweeps (never repo-clean).
- Empyrean spec is current through D2.23 + S2-D13 in the WORKING TREE (uncommitted —
  Volence's docs cadence; do not commit empyrean).

## The task: #9, RATIFIED design

Design doc (status RATIFIED, rulings inline):
`docs/superpowers/specs/2026-07-08-spec2-plan7-item9-scripted-states-design-draft.md`
— D9.1–D9.6 locked. Summary of the rulings:

- `script` is the surface (contextual opener per §10 policy).
- 9b MVP: BARE `yield` (no value protocol; that is 9c). `wait_frames` stays a comptime
  helper / sugar.
- The resume slot is a TYPED Sst FIELD (`resume: ScriptPc`, a construct-defined newtype)
  — it IS the engine's existing next-frame routine pointer, engine-owned offset; only
  real resume points / proc entries are writable (totality). See D9.5: `routine` helper
  and yield write the same storage; both coexist.
- Hidden resume table encodings: word_offsets + long_ptrs ONLY (both shipped by #6's
  dispatch). Pre-shifted ×4 deferred to demonstrated need.
- **D9.6 (per-frame epilogue, Volence's own addition — do not deviate):** `yield` lowers
  to "store resume point, then `jbra <epilogue>`", NEVER bare rts. Epilogue declared once
  per script (`shows <label>`-shaped surface) with per-site override (`yield <label>`);
  bare yield with no declared epilogue = compile error.

## Staging (D9.4)

- **9a (small, do first, own commit(s)):** `dispatch Name (encoding: …) { Member: { … } }`
  inline bodies = sugar for an anonymous per-member proc (hygienic label, same encoding
  row as a named target). The seam already exists and errors specifically (reserved since
  #6) — find it via the `[dispatch...]` diagnostics in frontend-emp lower/. NO
  state/yield semantics in 9a.
- **9b (the MVP):** `script` construct — `loop`/straight-line bodies, comptime helpers
  legal inside, `yield` per D9.2+D9.6, lowering onto a hidden dispatch-encoded resume
  table + the typed resume slot. Exhibit: pitcher_plant's brain REWRITTEN as a script
  alongside the proc version (both compile; argue equivalence in the doc/tests).

## Machinery you build on (all shipped)

- `dispatch` lowering (frontend-emp lower/, the offsets/dispatch pair — mirror-comment
  guarded; see S2-D12f about their shared shape).
- `Fragment::RelaxLadder` + jbra (sigil-link/src/relax.rs) — yield's `jbra <epilogue>` is
  an ordinary jbra; undeclared-fallthrough already treats jbra as a terminator.
- Hygienic labels per instantiation (asm{} machinery) — resume labels ride this.
- Newtypes + typed Sst fields (game prelude at examples/game/prelude.emp; Sst is the
  teaching layout — the `routine` helper there is the manual spelling per D9.5).
- The here() fix's anchor precedent (`__here$<module>$<n>`) if you need synthetic labels:
  `$`-names are unlexable by both frontends and program-unique by module+counter.

## Process (NON-NEGOTIABLE, the standing loop)

Isolated worktree under sigil/.worktrees/ off master; strict TDD with RECORDED RED
evidence (notes file per the 2026-07-08 here-fix precedent); commit-per-task; two-stage
reviews (spec + code-quality) on load-bearing tasks; whole-branch adversarial review with
byte-diff probes vs master across examples/ + examples/game before calling it
checkpoint-ready; controller independently verifies implementer claims; NO merge to
master without a Volence checkpoint. Fable designs/audits; opus implements gnarly
lowering, sonnet mechanical/authoring.

## Watch-outs

- --root examples is unusable (four pre-existing `module m` collisions); the game corpus
  root is examples/game/.
- Byte-exactness bar: programs not using script/inline-bodies must be byte-identical to
  master (probe it).
- The epilogue label in D9.6 is a LABEL operand (jbra target) — reuse jbra's
  `[jbra.label-only]`-class diagnostics for a non-label epilogue.
- After 9a+9b: 9c (value yields, for-loops, script-calls-script) needs no new
  ratification but should get its own short design note; 9d (byte-command DSL) stays
  gated per D9.3. #7 banks is NEXT after #9 and needs Volence engaged (bring ledger
  L-H.1 cross-section origin staleness to that conversation).
