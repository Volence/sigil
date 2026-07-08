# Handoff — Sigil Spec 2 · Plan 7 backlog #6: state machine + SST overlay

Orientation for the next Fable agent (reviewer/spec-writer/decision-maker; a lesser model
implements). Written 2026-07-07 (Fable), immediately after merging #5.

## Where things stand (Plan 7 so far)
- **Backlog #1–#5 ALL MERGED to master + pushed** (#5 = `d1e4288`, 2026-07-07): lexical gaps,
  symbolic operands, `offsets`, module resolution (`--root`/`--prelude`/`--map`), and item-position
  `ensure`/`ensure_fatal` + `data (max_size:)`. `examples/guards.emp` is #5's worked exhibit.
- **#5's merge also carried two #4 audit fixes**: the resolver now recurses into `section {}` items
  (`faf5191` — this also un-deferred §4.7 cross-module offsets targets, byte-checked) and
  module-qualifies dotted exported labels via `rename::canonicalize_name` (`c5228e5`).
- **Spec is current through D2.20** in `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` — **UNCOMMITTED**
  working tree per Volence's docs cadence (empyrean is ~38 commits ahead of origin). Read §4.6
  (vars/overlays), §5 (procs/labels), D2.15 (offsets is DATA-only; dispatch is a separate
  encoding-agnostic construct), S2-D10 ledger row, before designing #6.
- **Process discipline that paid off this cycle:** post-merge adversarial audits of #3/#4 found the
  two resolver defects above (both loud errors, zero tests covered them). Recommend a light
  adversarial audit of the #5 merge (`ba3fb98..d1e4288`) before building on it — cheap insurance,
  proven ROI. Dispatch it to opus; you judge the findings.

## ⚠️ Green-gate baseline correction (bites every branch until fixed)
`cargo test --workspace` FAIL-FASTS across test binaries. The real baseline is **4 red tests**, all
in `sigil-harness`, all one upstream root cause: the sibling **aeon** repo's sound driver was
refactored 2026-07-07 and the AS front-end can't evaluate a `strlen()` in the new code
(`full_build_reproduces_sound_driver_regions`, `vector_table_matches_reference_rom_first_256_bytes`,
`full_debug_rom_matches_assembled_reference`, `full_rom_matches_assembled_reference`). Verified
identical on clean master. Until the aeon-side situation is resolved (Volence's call: fix the AS
front-end's `strlen`, or re-vendor the harness reference), every green gate must be
`cargo test --workspace --no-fail-fast` + this exact 4-test allowlist + "zero NEW failures".

## What #6 IS (research T1-b + T2-c, revised by Part-II R1/R2)
Two constructs that together are "the object-code migration pair":
1. **SST scratch overlay (T2-c)** — a typed union view over the shared SST scratch bytes:
   `overlay PitcherPlantV over Sst.scratch { charge: u16, timer: u8 }` → field access
   **as displacement**: `timer(a0)` lowers to `$3C(a0)`-style bytes, byte-identical to the
   `objoff_XX` idiom it replaces (26,697 `field(aN)` accesses in S3K Levels/ alone). This is the
   **pitcher_plant blocker** (`timer(a0)` = "unknown name" today) and the smaller/safer half —
   consider landing it FIRST as its own commit-series.
2. **Typed state machine (T1-b)** — an object's state field as an enum whose variants bind to
   code, compiler-emitted dispatch, exhaustiveness guaranteed (a missing entry is a compile error,
   not a jump-to-garbage). **MUST be encoding-agnostic (R1)**: Sonic self-relative word offsets,
   Vectorman absolute 32-bit pointers, Ristar pre-shifted indices — enable all three, impose none;
   admit states as first-class function values, not only enum ordinals. **Scope seam to decide
   (R2):** the full scripted-coroutine/`yield` construct is backlog #9 — #6 should ship the typed
   dispatch container and leave the coroutine surface a designed-for extension, not build it.

## Verified code facts (grepped/probed 2026-07-07 — re-verify at T0, code is authoritative)
- **SST overlay is UNBUILT**: `vars X: sst_custom {}` parses/builds, but `timer(a0)` in a proc body
  = "unknown name" (probed in the item-4 gap analysis,
  `notes/2026-07-07-item4-pitcher-plant-gap-analysis.md` — read it; it lists every pitcher_plant
  gap with evidence). `vars` blocks + the map-file region concept exist (§4.6); what's missing is
  the overlay TYPE + field-access-as-displacement in operand lowering.
- **Operand lowering seam**: symbolic absolute operands ride `Fragment::RelaxAbsSym` (item #2);
  displacement operands `d(An)` exist in the ISA but `.emp`'s `CodeOperand` model is deliberately
  narrow — extending it is where `timer(a0)` lands. Study `lower/` + how `asm{}` splices resolve
  operand classes (§6.2).
- **`offsets` + `Name.count` + ordinals (D2.15) are shipped and byte-exact** — the Sonic-encoding
  state machine can lower ONTO them; don't rebuild emission.
- **Item-position `ensure` (D2.20) is shipped** — state-machine invariants can lean on it.
- **The `routine(a0)` byte + `add.w`-into-jump-table idiom** is the port target for the Sonic
  encoding: 2,917 reads / 3,120 writes in S3K. SCE uses `move.l #.label, code_addr(a0)`
  continuations instead — the two encodings the construct must unify.
- **pitcher_plant ALSO still needs** (NOT #6 scope — don't creep): `jbra`/`jbsr` (#8), statement-
  position comptime helpers (`spawn`/`anim`/`routine` call grammar), `code: init` proc-name-as-
  pointer. #6's acceptance should be `timer(a0)`-class access byte-checked + a dispatch-table
  exhibit, not "pitcher_plant compiles".

## Process (NON-NEGOTIABLE — unchanged, plus the baseline note above)
- Isolated worktree `sigil/.worktrees/<branch>`; Fable designs/decides, lesser model implements
  (opus for gnarly lowering work; sonnet fine for mechanical parser/test tasks — pick per task);
  TDD; commit-per-task; two-stage reviews on load-bearing tasks; whole-branch adversarial review
  that byte-diffs against AS wherever a byte argument exists.
- Green gate per commit: `--no-fail-fast` + the 4-test allowlist (above) + clippy `-D warnings`.
- **Milestone: do NOT merge to master without a Volence checkpoint.**
- Volence defers technical calls; checkpoint at milestone boundaries.

## After #6 — remaining backlog (context, not now)
#7 bank/window placement (gates sound; `no_straddle`) → #8 jbra/jbsr + relaxation (pitcher_plant)
→ #9 byte-command DSL / scripted coroutine (`yield`; #6's dispatch container should anticipate it)
→ #10 compression builtins. Small viable increments any time: offsets inline-target members
(§4.7, outside-reader demand), qualified `mod.name` reference resolution (#4 limitation), stray-`;`
fix-it, `[operand.const-as-address]` lint (natural fit DURING #6's operand-lowering work).

## References
- Research: `specs/2026-07-06-sigil-spec2-p7-language-completion-research.md` (T1-b, T2-c, R1, R2).
- #5 completion: `notes/2026-07-07-spec2-plan7-item5-complete-handoff.md`; design
  `specs/2026-07-07-spec2-plan7-item5-ensure-capacity-design.md` (D5.5 parks `fits_within` for #6).
- pitcher_plant gaps: `notes/2026-07-07-item4-pitcher-plant-gap-analysis.md`.
- Memory: [[spec2-progress]], [[emp-sonic-newtype-candidates]], [[emp-language-design-principles]],
  [[fable-role-reviewer-spec-writer]], [[user-defers-sigil-technical-calls]].
