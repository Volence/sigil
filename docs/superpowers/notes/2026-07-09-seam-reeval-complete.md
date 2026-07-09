# Seam re-evaluation session — COMPLETE, awaiting Volence checkpoint (overnight, handoff 1 of 2)

Written 2026-07-09 (overnight Fable session, per
`notes/2026-07-09-overnight-handoff-1-seam-reeval.md`). **Branch `seam-reeval` is UNMERGED —
no merge without Volence's morning checkpoint.** All rulings + rationale:
`specs/2026-07-09-seam-reeval-decisions.md` (committed alongside this note).

## What shipped (sigil branch `seam-reeval`, worktree `.worktrees/seam-reeval`, off master `5a24cee`, 6 commits `fa890f8..ffab698`)

- **Item B — RULING: option (ii), `extern("NAME")`** (decision record has the full
  fact-check + rationale). Implementation:
  - `fa890f8` — AS front-end exports int `equ`/`=` equates as link-level `EquSym`s
    (attach-to-open-section, else a `pending_equ_syms` queue drained on next section open;
    pass-end carrier flush for equates-only sources; fresh-builder-per-pass makes export
    structurally once-only). String equs and `set`/`:=` never export.
  - `20c0124` — `extern(name)` builtin: raw `Value::LinkExpr(Expr::Sym)` passthrough (no
    mask/shift), `bankid`-taxonomy diagnostics, standard LinkExpr comptime-position refusal.
  - `27f762f` — pre-existing gap exposed and fixed: `check_link_asserts`'s own
    `build_symbol_table` seeded labels but never `equ_syms` (link()'s Pass 1b did both).
  - `a52eb61` — e2e cross-seam probes: AS equ read via `ensure(extern(...))` (holds + fires
    variants), typo probe, data-cell emission probe.
  - `9cf82b5` — quality-review fix round (debug_assert on the unfolded-equ invariant;
    comment-accuracy fixes; `eval_symbol_ref_arg` helper extraction — rule of three).
- **Item C — the "internal: … anchor label" diagnostic** (`ffab698`): `Fold::Poison` on a
  link-assert condition now walks the unresolved `Sym` leaves — ordinary names get
  ``link assertion condition references symbol(s) `X`, `Y` not defined in this link —
  expected when compiling a cross-seam module standalone; supply the map/harness
  composition that defines them``; the compiler-bug wording survives ONLY for a `__here$…`
  anchor leaf (structurally unreachable = genuinely internal); Poison-without-missing-
  symbols gets a generic loud error. New standalone-compile test at the mt/sfx shape; the
  extern typo probe now pins the new wording; all resolvable-but-false negative probes
  (mt ×4, sfx ×5, probe_b, cond-zero) untouched and green.
- **Item A — ledger re-evaluation + the accumulated empyrean spec-integration pass** (Fable
  personally): dispositions in the decision record (S2-D14(a) gate re-affirmed; (d) held
  through partial_fold; (e) re-affirmed; 9d re-gate re-affirmed at arc end — 34 embeds,
  zero hand-authoring demand; imm32 narrow scope re-affirmed as design; ensure-spelling gap
  CLOSED by B). **empyrean spec edits are in its WORKING TREE, UNCOMMITTED** (house cadence,
  stacked on the pending #7/D2.25 + D2.26 passes): new changelog row **D2.27**, §4.5
  ptr-array int elements, §7.5 bidirectional-seam + `extern` paragraph, §8.1
  defines/imm32/partial_fold/standalone-diagnostic contract text, §10 inventory + ledger
  row annotations.

## Verification state

- Full workspace on branch HEAD: **1510 tests / 0 failed** (was 1507 at T3; +4 new, −1
  rewritten). `SIGIL_STRICT_GATE=1` harness gates: 15/0 — all ROM byte-identity gates
  UNCHANGED (no allowlist or expected-byte edits anywhere on the branch). Clippy
  `-D warnings` clean.
- **F1 flake: zero occurrences tonight** (multiple full-workspace runs across the session).
- Process: sonnet implementers, TDD with recorded RED throughout; Task B got the full
  two-stage review (sonnet spec reviewer — compliant; sonnet quality reviewer —
  ready-with-fixes, all four findings fixed in `9cf82b5` and re-verified). **Deviation:**
  Item C's review was done by Fable personally (diff-level, wording + message-test focus —
  the handoff's named risk surface) instead of a second sonnet round; C is a single-purpose
  diagnostic change and the personal review was the stronger check.

## Notable engineering notes

- The naive B1 approach (force-open a section at an equate) broke the real corpus
  (`m0_regions`): a leading `DEBUGGER__EXTENSIONS__ENABLE: equ 1` flipped `engine.inc`'s
  `org 0` onto its validated path. The pending-queue design exists BECAUSE of that trace —
  see `fa890f8`'s commit body before "simplifying" it away.
- AS equs arrive in the linker already folded to `Expr::Int` (the AS env folds them); the
  linker-side machinery (Pass 1b, fold-vs-labels) was already in place from T0/R-T0.3 —
  the AS frontend just never fed it. No linker placement changes were needed.
- No same-name-equ collisions surfaced anywhere in the real corpus (the contingency
  ruling — identical-value dup tolerance — was NOT needed).

## Checkpoint asks for Volence (morning)

1. Merge `seam-reeval` → sigil master (`--no-ff`, house pattern). Six commits, all green.
2. Ratify the **Item B ruling** (extern + AS equ export as THE cross-seam spelling) and the
   **Item A dispositions** — then the empyrean working-tree spec pass (now spanning
   #7/D2.25 + D2.26 + D2.27) is ready to commit whenever you do your spec-commit sweep.
3. Follow-up (recorded, not tonight): migrate mt_bank.emp/sfx_bank.emp co-residency ensures
   from the bankid-label-proxy idiom to `extern("SND_ENGINE_TABLE_BANK")` — ride the next
   tranche that touches those files; needs its own byte-gate run.

## Next

Handoff 2 (Plan-7 #10 compression builtins) — same overnight session, branch
`compression-builtins`. Then the 68k engine-port campaign kickoff note.
