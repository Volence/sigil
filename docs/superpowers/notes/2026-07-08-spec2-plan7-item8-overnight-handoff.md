# Handoff — Plan 7 overnight session: #8 jbra/jbsr + branch relaxation, then the pitcher_plant completion tranche

Orientation for the next Fable agent (reviewer/spec-writer/decision-maker; lesser models
implement — opus for gnarly lowering/relaxation work, sonnet for mechanical parser/authoring
tasks, your pick per task). Written 2026-07-08 (Fable), immediately after merging #6.
**Volence is asleep: this is an EXTENDED session — he pre-authorized more than one backlog
item. Scope decision (Fable's, locked): do the plan below; do NOT merge anything to master —
stack checkpoint-ready branches for him to review in the morning.**

## Where things stand
- **Plan 7 backlog #1–#6 ALL MERGED** (#6 = `90a21b6`, 2026-07-08, pushed). #6 shipped SST
  overlay field access (`timer(a0)` → `$2E(a0)`, bare + qualified + cross-module `pub vars`)
  and `dispatch (encoding: word_offsets|long_ptrs)` — read
  `notes/2026-07-08-spec2-plan7-item6-complete-handoff.md` for its full ledger, including the
  three sanctioned notes (D6.A3 const-fallback meaning change; cross-module reverse ordinals
  resolve for neither offsets nor dispatch; offsets/dispatch same-name link collision).
- **Spec is current through D2.21** in `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` (working tree,
  UNCOMMITTED per Volence's cadence): §4.6 pinned overlay semantics, NEW §5.5 `dispatch`,
  §3.3 no-nested-sections, S2-D3 updated (resolution shipped; prelude CONTENTS remain),
  NEW S2-D12 (table/overlay increments incl. cross-module ordinals).
- **Green-gate baseline UNCHANGED:** `cargo test --workspace --no-fail-fast` + exactly 4
  allowlisted sigil-harness reds (upstream aeon sound-driver strlen refactor:
  `full_build_reproduces_sound_driver_regions`, `vector_table_matches_reference_rom_first_256_bytes`,
  `full_debug_rom_matches_assembled_reference`, `full_rom_matches_assembled_reference`) +
  `cargo clippy --workspace --all-targets -- -D warnings`. Zero NEW failures. Re-verify the
  4-red set at T0 in case aeon moved again.

## Overnight plan (in order; stop cleanly wherever tokens/valor run out)

**Step 0 — light post-merge audit of the #6 merge (`d1e4288..90a21b6`).** The proven pattern:
1–2 opus adversarial auditors (the branch had heavy review already, so LIGHT — focus on
anything the whole-branch review's probes didn't commit as tests, and on the resolver-fix
composition). You judge findings; real defects become pre-task fixes on the #8 branch.

**Step 1 — backlog #8: `jbra`/`jbsr` + unsized conditional-branch relaxation.** BOTH halves
are ratified spec (§5.4 + D2.18) — this is implementation, not design:
- `jbra label`/`jbsr label` → auto-select `bra.s`/`bra.w`/`jmp (abs)` resp. `bsr.s`/`bsr.w`/`jsr`
  by resolved reach. The deferred-relaxation machinery EXISTS: `Fragment::JmpJsrSym` (jmp/jsr
  width by `asl_width_rule`) and the two-candidate `Fragment::RelaxAbsSym` (#2's seam) are the
  exemplars; jbra likely needs a THREE-candidate fragment (s/w/jmp) — check whether Core's
  grow-only monotonic relaxation (`sigil-link::resolve_layout`) generalizes; if a Core edit is
  needed, keep it additive like `BankPtr16Be` was (D-P4.7 precedent).
- Unsized `bne .draw`-class conditionals relax `.s`↔`.w` (§5.4 "unpinned branches are sized by
  Core's relaxation"; today they hard-error `[branch.missing-size]`). NO far form — out of
  ±32KB reach is a compile error naming the distance (`jbcc` trampolines stay deferred, D2.18).
- `@as_compat` files: unchanged behavior (AS ports keep explicit sizes; `[branch.suboptimal-size]`
  informational lint is spec'd prose — implement only if trivial).
- Byte-exactness: every width choice byte-diffed vs hand-derived encodings; the jbra→jmp fallback
  must match the shipped jmp bytes. `examples/pitcher_plant.emp` uses `jbra .draw` (short reach),
  `jbra Draw_Sprite`/`jbsr ObjectMove` (cross-module) and unsized `bne`/`bhi` — its errors are
  your RED corpus (gap analysis b1 + b2 in `notes/2026-07-07-item4-pitcher-plant-gap-analysis.md`).
- Worktree branch off master: `sigil/.worktrees/plan7-item8-jbra-relaxation`.

**Step 2 — the pitcher_plant completion tranche (branch STACKED on #8's branch, separately
checkpointable: `plan7-pitcher-plant-tranche`).** Goal: **`examples/pitcher_plant.emp` compiles
end-to-end, byte-argued** (Appendix D of the spec argues its layout). Remaining gaps (gap
analysis b3/b4/b7 + category a):
- **b3 grammar:** statement-position comptime-helper calls in proc bodies (`anim Ani.Shoot`,
  `routine shoot`, `facing_abs d0`, `despawn_below_level`) — today the instruction parser
  rejects each bareword as a bad mnemonic. Design intent (§3.4/§6): a call to an in-scope
  `comptime fn` returning `Code`, instantiated at statement position. Decide the exact grammar
  rule yourself (contextual: bareword resolving to a comptime fn; keep tenet 3 — instruction
  lines stay assembly, so the rule must never shadow a real mnemonic — mnemonics win).
- **b4 grammar + authoring:** `spawn(SeedDef, offset: Vec{ x: -16, y: -4 }, flip: inherit)` —
  named-arg call syntax, `inherit` as a prelude value (NOT a keyword if avoidable), `Vec{}`
  struct-literal args.
- **b7:** bareword proc-name-as-pointer in data (`code: init` — string form `code: "init"`
  works today; make the bareword resolve to the same SymRef when the name is a known proc).
  This ALSO closes the SCE-continuation half of R1 (first-class code values in data).
- **a1–a4 authoring (sonnet):** the game prelude + sibling art/engine modules under
  `examples/` (full `ObjDef`, `ArtTile`/`Collision`/`Size`/`Vel`/`Vec`, `Sst` with windows,
  `Map_PitcherPlant`, `VRAM_PITCHER_PLANT`, `Player_1` (a `vars`-region or label), `Draw_Sprite`,
  `ObjectMove`, and the `spawn`/`anim`/`routine`/`facing_abs`/`despawn_below_level` comptime
  fns). §3.4 lists the intended prelude contents; [[emp-sonic-newtype-candidates]] memory has
  the type designs. NOTE b6 (`move.w Player_1.x_pos, d0` — straight-line symbolic operand with
  a field displacement) is ALSO needed by the exhibit: `Player_1` + `.x_pos` = abs-sym +
  comptime field offset → extend the #2 `RelaxAbsSym` path to carry `Sym + const` (the IR
  `Fixup.target: Expr` already holds arbitrary exprs — check `Expr::Add` folding in the linker).
  If that turns out heavy, descope it LAST (rewrite the exhibit line as two instructions only
  with a recorded decision — prefer building it; it is the last operand-class gap).
- Acceptance: `cargo run -p sigil-cli -- emp examples/badniks/pitcher_plant.emp --root examples
  --prelude <id>`-shaped build → zero errors, byte image asserted in ports.rs with hand-derived
  bytes; a first-diff against Appendix D's byte argument where it makes one.

**Step 3 — only if everything above is landed + reviewed:** Fable writes the **#9 design doc
ONLY** (scripted-state/coroutine surface on dispatch's reserved `Member: { … }` seam, research
R2 + T1-c; byte-command DSL scope decision). NO implementation — #9 is the largest feature and
Volence should see the design first. Park it as a specs/ draft for the morning checkpoint.

**Explicitly NOT tonight:** #7 bank/window placement (it gates the SOUND migration and
interacts with aeon-side map decisions better made with Volence awake); merging ANYTHING.

## Process (NON-NEGOTIABLE, unchanged)
Isolated worktrees under `sigil/.worktrees/`; Fable audits/designs/decides, lesser models
implement; strict TDD, commit-per-task; two-stage reviews (spec + code-quality) on load-bearing
tasks, single-pass on mechanical ones; whole-branch adversarial review per branch, byte-diffing
wherever a byte argument exists; green gate per commit (baseline above); independent Fable
verification of implementer claims before writing any checkpoint; morning deliverable = ONE
consolidated checkpoint message covering all branches (each separately mergeable), plus updated
memory. Do NOT push unmerged branches unless asked.

## References
- #6 completion: `notes/2026-07-08-spec2-plan7-item6-complete-handoff.md`; design
  `specs/2026-07-07-spec2-plan7-item6-overlay-dispatch-design.md` (the D6 decisions, deferral
  lists); plan `plans/2026-07-07-spec2-plan7-item6-overlay-dispatch.md` (house plan style).
- Gap analysis: `notes/2026-07-07-item4-pitcher-plant-gap-analysis.md` (b1–b7 with evidence).
- Research: `specs/2026-07-06-sigil-spec2-p7-language-completion-research.md` (Part V solutions,
  R1/R2, T3-b tail-call notes adjacent to #8).
- Spec: `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` §5.4/§5.5/§3.4/D2.18/D2.21 (uncommitted WIP).
- Memory: [[spec2-progress]], [[jbra-jbsr-auto-reaching-branches]],
  [[emp-sonic-newtype-candidates]], [[fable-role-reviewer-spec-writer]],
  [[user-defers-sigil-technical-calls]].
