# Port #1 (hblank.asm → hblank.emp) — COMPLETE, awaiting Volence checkpoint

The 68k engine-port campaign's opening act. Both branches UNMERGED.

- **sigil `port-hblank`** (worktree `.worktrees/port-hblank`, off master `aafdc95`, 8 commits):
  `d1106cd` sp alias + movem reglists → `ced3cce` byte gates (+ the `move.l #imm,(abs)` deferral
  fix) → docs/review-minor commits → `6a5d047`+`6bc0069` the equ-relaxation fix (T5 — pulled
  into this port at Volence's ruling, see the retrospect).
- **aeon `sigil-emp-hblank`** (checked out in the MAIN aeon tree, off `a103e46`, 1 commit
  `43e252d`): `engine/system/hblank.emp` + the `SIGIL_EMP_HBLANK` gate in `engine/engine.inc`
  + `CODING_CONVENTIONS.md` §10 (the `.emp` formatting convention, Volence-driven).

**Final validation:** workspace **1804/0** tests, strict gates 175/0 zero skips, clippy
`--workspace --all-targets -D warnings` clean, strict gates
(`SIGIL_STRICT_GATE=1 AEON_DIR=…`) zero skips — port gates 2/2, negative probes 3/3,
mixed-ROM 8/8 (the FULL mixed ROM with all four gates — DAC+MT+SFX+HBLANK — byte-identical to
both reference ROMs). Gate-off asl byte-neutrality proven by sha256 before/after on all three
builds (plain, DEBUG=1, demo). Reviews: T1 two-stage (spec compliant / quality approved,
adversarial cross-front-end reglist verification), T2 spec-reviewed (orchestrator), T3+whole-
branch two-prong review (COMPLIANT / APPROVED, no Critical/Important; the two Minor doc nits
fixed on-branch).

## What the port shipped beyond the file

1. **`sp` register spelling** — general `a7` alias at `Reg::from_name` (every EA form +
   reglists), byte-identical by construction.
2. **`movem` register lists** — mnemonic-directed reinterpretation of the parsed expr tree
   (`d0-d1/a0`, ranges + `/` unions, canonical mask; AS-parity refusals: descending ranges,
   `movem.b`). Cross-front-end byte agreement verified adversarially on precedence corners.
3. **`move.l #<unresolved>, (abs).w/.l` cross-seam deferral** — `try_defer_long_imm` extended
   beyond register destinations (the REAL `boot.asm:185` shape; destination resolves eagerly,
   only the source imm32 defers; verified against the reference encoding `21FC 0000228E 8022`).
   Deliberate narrowness preserved and probe-proven: imm8/imm16, other mnemonics, other
   destination modes all still hard-error.
4. **Equ-visible width relaxation + symbol-naming diagnostic** (T5 — Volence's mid-session
   ruling: fix it in-port so later files are written the correct way the first time, no
   revisit pass). `equ` values (`.emp` or exported AS-side) now resolve as `RelaxAbsSym` and
   `jmp`/`jsr` targets inside the placement⇄relaxation fixpoint (bounded partial fold per
   pass; same grow-only gate labels use); the unresolved-operand diagnostic names the missing
   symbol with the Item-C cross-seam-standalone framing (the hblank standalone probe now pins
   the improved wording). **Review ruling folded in:** `jbra`/`jbsr` (the pc-relative ladder)
   deliberately stays labels-only — a near-integer equ as a branch target silently mis-encoded
   as pc-relative (reviewer-probed `60 1E`), so the ladder refuses equs with a steering
   diagnostic ("branch targets must be labels; use jmp/jsr for an absolute target"), pinned by
   RED-first regression tests.
5. **The `.emp` formatting convention** (Volence's catch — "start off strong"): code files use
   the braceless `module X in <section>` form; braced sections indent 4; instruction lines
   keep the .asm column style. aeon `CODING_CONVENTIONS.md` §10; restyle proven byte-neutral.
6. **The campaign gap ledger + per-conversion retrospect cadence** (Volence, this session):
   `docs/superpowers/notes/campaign-gap-ledger.md` — every port sweeps observations in; the
   checkpoint retrospect rules on them while fresh.

## THE GATE-PATTERN WRITEUP (the kickoff deliverable — the port loop for campaign files)

Per-file recipe, proven by four ports (dac, mt, sfx, hblank — first CODE file):

1. **Survey (orchestrator, before any code):** pin the file's per-shape addresses from
   `s4.lst`/`s4.debug.lst` (label addresses + region end = the org resume value); list
   cross-seam symbols BOTH ways (what the file reads = must be link-visible; who reads the
   file's labels = the consumers your synthetic test mirrors); probe-compile the instruction
   surface through `sigil emp` — anything that doesn't lower is a T1 language task, not a
   port-time surprise.
2. **Language gaps first (sigil branch):** TDD the missing spellings with AS-parity byte pins
   before the port test exists.
3. **The aeon edit (2 files):** `<name>.emp` next to the `.asm` (braceless `module … in
   <section>` for code files; `@as_compat`; comments describe function) + the gate at the
   include site: `ifndef SIGIL_EMP_<NAME> / include … / else / [ifdef __DEBUG__] org
   <debug-end> [else] org <plain-end> [endif] / endif`. Prove gate-off byte-neutrality by
   sha256 before/after on ALL reference builds (plain, DEBUG=1, demo). **Build only in the
   main tree's environment — see the reproducibility watch item.**
4. **The byte gates (sigil branch):** a `<name>_port.rs` (sfx_port/hblank_port template):
   per-shape inline map region (base = the pinned addresses), compile the REAL aeon file,
   synthetic sections for cross-seam inputs (labels the file reads) AND a synthetic AS-side
   consumer that mirrors the real consumers' instruction shapes (this is what surfaced the
   imm32-deferral gap — a port whose harness never builds a REAL AS-shaped consumer can ship
   silently incomplete); byte-diff both shapes against the reference ROM slices. Extend the
   cumulative mixed-ROM gate (`mixed_dac_rom.rs` + `assemble_mixed_*_as_side`) with the new
   define — full-ROM identity is the strongest gate and the only one that exercises the org
   resume. Negative probes: doctored source → bytes differ; standalone compile → loud
   missing-symbol diagnostic; wrong-base map → placement moves.
5. **Reviews + retrospect:** two-stage per substantial task; whole-branch two-prong at the
   end; the code-sense pass ("is this the code we'd WANT?" — [[port-code-sense-review]]);
   gap-ledger sweep + the retrospect section below; checkpoint packet; UNMERGED for Volence.

Code-file specifics vs the data ports: the section is shape-invariant with a shape-dependent
BASE (map key), so the module needs no `-D` defines; `pub proc` names export as bare link
symbols (proven structurally + end-to-end — same sink as data items); `@as_compat` today pins
branch sizes only (hblank has none; the abs-width rule is link-time and AS-parity by
construction).

## Code-sense verdict (the [[port-code-sense-review]] pass)

`hblank.emp` is the code we'd want, not just the code that matches: braceless form reads like
the asm it replaces; the one non-obvious spelling (bare symbolic operand for `(…).w`) carries
its explanation inline. "Reads wrong" list: EMPTY. The construct that would make the file
*nicer* — a typed interrupt-handler slot — is already ledgered (S2-D9, gated on an
HBlank-effect consumer); no action.

## RETROSPECT #1 (per-conversion cadence — rulings requested at this checkpoint)

New ledger entries this port, each with a recommendation (full context in
`campaign-gap-ledger.md`):

| Entry | Recommendation |
|---|---|
| `sp` alias; movem reglists; `move.l #imm,(abs)` deferral | SHIPPED in-port (blocked the port) — ratify with merge |
| Formatting convention (braceless code files / indent-4 sections) | SHIPPED as convention (CODING_CONVENTIONS §10) — ratify; `sigil fmt` stays S2-D11(c) |
| Symbolic absolute operand targeting an **equ** fails; and the `RelaxAbsSym` diagnostic names only the section, not the symbol | **SHIPPED in-port (Volence's ruling, 2026-07-09: fix before the files that need it, not after)** — one fix, T5 (`6a5d047`+`6bc0069`), two-stage reviewed; ratify with merge. Review-ruled scope: equs resolve for abs operands + `jmp`/`jsr`; `jbra` ladder stays labels-only with a steering refusal |
| `initial_cpu` is caller convention, not module fact (4 hardcoded call sites) | Jot for now; revisit at the first Z80-adjacent port (module-declared CPU or default-and-warn) |
| movem `(0,An)` collapse not ported | Leave jotted (believed unreachable from `.emp`'s resolved model; breadcrumbed in code) |
| **Aeon clean-build reproducibility** (fresh worktree build ≠ pinned ROMs, ~131KB generator drift; port branches must run IN the main tree; never rebuild there without re-pinning) | **Own session** — tooling, pre-existing, but it makes the pinned baseline unreproducible from a clean clone. Decide: track/pin generator outputs vs deterministic generators + re-baseline |

## Checkpoint asks

1. Merge sigil `port-hblank` → master (`--no-ff`), delete worktree/branch.
2. Merge aeon `sigil-emp-hblank` → master (`--no-ff`), delete branch.
3. Rule on the retrospect table above (esp. the equ-operand fix next tranche + scheduling the
   reproducibility session).
4. Next per the kickoff ranking: **port #2 `controllers.asm`** (62 ln, straight-line I/O,
   single caller) — can batch with `math.asm` (27 ln, GetSineCosine + `embed()` sine table)
   as one tranche per the suggested cadence.
