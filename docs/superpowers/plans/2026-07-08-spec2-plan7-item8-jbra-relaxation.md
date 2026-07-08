# Plan — Spec 2 · Plan 7 backlog #8: `jbra`/`jbsr` + unsized-conditional relaxation

Design: `../specs/2026-07-08-spec2-plan7-item8-jbra-relaxation-design.md` (all decisions live
there; this file is the execution order). Branch `plan7-item8-jbra-relaxation`
(worktree `.worktrees/plan7-item8-jbra-relaxation`), off master `6775a81`. Strict TDD,
commit-per-task, green gate per commit (workspace tests → only the 4 allowlisted
sigil-harness reds; clippy -D warnings). Two-stage review (spec, then code-quality) on
T1/T2/T3; single-pass on T0b/T4. Whole-branch adversarial review at the end.

- **T0a (opus) — audit fix: definition-site binding for bare-window overlays** (design
  Part 0a). RED: cross-module rebind repro (wrong $8 vs $2), spurious-ambiguity repro,
  plus a control (same-module bare window still works; dotted window unchanged).
  Files: frontend-emp resolve/eval overlay path. Commit `fix(emp): bare overlay windows
  bind at definition site — cross-module consumers can no longer rebind or break them`.
- **T0b (sonnet, timeboxed) — audit fix: once-per-compile dedup of struct-declaration
  diagnostics** (Part 0b). RED: cross-module double-report repro. If invasive: drop,
  record in the checkpoint note. Commit `fix(emp): struct declaration diagnostics report
  once per compile`.
- **T1 (opus) — Core `Fragment::RelaxLadder` + rung fixpoint** (Part A, incl. the PcRel8
  disp-0 apply guard). Files: sigil-ir (variant + doc), sigil-link relax.rs (+ lib.rs arms).
  Pure Core TDD with hand-built fragments, relax.rs test style. Commit `feat(link): generic
  grow-only relaxation ladder (RelaxLadder) — reach derived from candidate fixup kinds`.
- **T2 (opus) — `jbra`/`jbsr` frontend** (Part B). Files: frontend-emp lower/code.rs,
  backend-m68k candidate builders, proc.rs terminator recognition. Commit `feat(emp):
  jbra/jbsr auto-reaching branches (D2.18)`.
- **T3 (opus) — unsized branch relaxation** (Part C). Files: lower/code.rs +
  as_compat plumb. Commit `feat(emp): unsized branches relax .s/.w via Core (§5.4);
  @as_compat keeps the pin requirement`.
- **T4 (sonnet) — exhibit + byte-exact ports test**: `examples/reach_branches.emp` +
  ports.rs image assert (controller independently re-derives the bytes); scratch-probe
  confirmation that pitcher_plant's b1/b2 error classes are gone. Commit `test(emp):
  reach_branches exhibit — every ladder rung exercised, byte-exact`.
- **T5 (opus) — whole-branch adversarial review** with byte-diff probes vs master for
  non-participating code (sized branches, jmp/jsr, offsets/dispatch corpus).
