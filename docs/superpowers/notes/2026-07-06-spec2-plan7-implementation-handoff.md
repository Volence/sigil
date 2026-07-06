# Handoff — Sigil Spec 2 · Plan 7 (language finalization → implementation)

Orientation for a fresh agent starting Plan 7 implementation. Written 2026-07-06.

## Where things stand
- **Plans 1–6 MERGED to master + pushed.** Plan 6 = `@as_compat` + mixed `.asm`/`.emp` build +
  per-file byte-diff (branch merged at `eb5a802`).
- **Plan 7 RESEARCH is complete** (the big deliverable): the candidate feature set, mined from **all**
  local disassemblies (s2disasm, skdisasm, S.C.E., sonic_hack, aeon + the 6 non-Sonic Genesis
  disassemblies), plus online modern-assembler / demoscene / NESHLA research, plus a live-probed
  **gaps & landmines** pass with proposed solutions. Read it first:
  `sigil/docs/superpowers/specs/2026-07-06-sigil-spec2-p7-language-completion-research.md` (Parts I–V).
- Visual catalog (shipped vs proposed vs gaps): the artifact linked in the session (regenerate via
  `examples/`-style before/after if needed).

## What Plan 7 IS
Language **finalization**: research (done) → **ratify + freeze the spec** → **implement** the finalized
features. Then the migration campaign (68k source first, cycle-exact Z80 DAC driver last), then Spec 5
deletes the AS front-end (gated on "every load-bearing AS feature has a byte-exact Spec-2 equivalent").

## Step 0 — ratify before building the big stuff (checkpoint: Volence + Fable)
- Decide in/out on the candidate set (research doc Parts I–IV).
- **Settle the ONE genuinely open design decision: TEXT ENCODING** — char/string literals default to
  **raw ASCII/bytes**, with a **charmap** as a separate opt-in layer for Sonic tile-text. (Recommended;
  Part V.) Everything else is decided or mechanical.
- Fable freezes `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` (spec-writer role — see
  `fable-role-reviewer-spec-writer` memory).

## Step 1 — FIRST implementation tranche (safe to start NOW): the lexical gaps
Grounded, low-risk, and they gate the first real **code** port. Exact solutions in Part V:
- **Binary `%1010` literals** — `%`-immediately-followed-by-binary-digits = literal; spaced `%` stays
  modulo (`%` IS `.emp`'s modulo — verified `7 % 3` → `01`).
- **Char `'A'` literals** — raw byte (ASCII) + escapes.
- **String `"text"` in data** — emit ASCII bytes, author-controlled termination (no implicit `0`).
- **`dc.w -1` sentinels** — ALREADY WORKS via signed types (`[i16;1]=[-1]` → `FF FF`, verified). Just
  document the convention (signed types for signed values; collection constructs own terminators).
TDD, one construct per commit.

## Step 2+ — the ordered backlog (from the research)
2. **Symbolic operands in straight-line instructions** (`move.w Foo, d0`, `lea Table, a0`) — extend the
   existing `jmp`/`jsr` fixup path to all operands (`Abs16Be`/`Abs32Be` + the shared `asl_width_rule`).
   The biggest structural unblocker: separates "ports data files" from "ports code files."
3. **Bidirectional offset-table** (table + id-enum) — #1 data idiom (14k S3K / 4.6k S2) and the Plan-6
   blocker (`.emp` can't express `dc.w Target-Base` today). NOTE from the non-Sonic pass: keep the data
   offset-table SEPARATE from *state dispatch*, which must be encoding-agnostic (Sonic word-offset vs
   Vectorman absolute pointer vs Ristar pre-shifted index).
4. **Scan-and-index manifest + map-file placement + game prelude (S2-D3)** — unblocks code ports at all.
5. **`assert!` / capacity-refined regions** (~195 hand guards in aeon; cheap, high coverage).
6. **State machine + SST scratch overlay** (the object-code migration pair).
7. **Bank/window placement** (gates the *sound* subsystem migration).
8. **`jbra`/`jbsr` + conditional-branch relaxation** (see `jbra-jbsr-auto-reaching-branches` memory).
9. **Byte-command DSL / scripted-coroutine w/ `yield`** (largest; may stage).
10. **Compression builtins** — `s4lz` + the classic-Sonic family (`nemesis`/`kosinski`/`kosinski_m`/
    `enigma`/`saxman`) — enables porting S1/S2/S3K, finishes absorbing the `build.sh` generators.

## Process (NON-NEGOTIABLE — caught a CRITICAL in Plans 2/3/4)
- Isolated git worktree `sigil/.worktrees/<branch>`; subagent-driven; TDD per task; commit-per-task.
- **Two-stage reviews** (spec-compliance THEN code-quality via `superpowers:code-reviewer`) on
  load-bearing tasks; a **whole-branch adversarial review** at the end that CONSTRUCTS + RUNS
  cross-feature programs and **byte-diffs against the AS reference** wherever a byte argument exists.
- **Green gate before EVERY commit:** `cargo test --workspace` + `cargo clippy --workspace
  --all-targets -- -D warnings`. Keep the s4.bin harness (`m1d_rom`/`m1d_debug_rom`) green.
- Ground semantics in the spec, BUT where spec and code disagree, **the CODE is authoritative** —
  verify by grep, not by trusting a "reserved" doc line (the `ProvFrame::Comptime` lesson).
- **Milestone: do NOT merge to master without a Volence checkpoint.**

## Verified facts a fresh agent MUST know (don't rediscover the hard way)
- **`jbra`/`jbsr` is NOT built** (it's a candidate). Branches take an explicit `.s`/`.w` today; only
  `jmp`/`jsr` width is relaxed by `resolve_layout`.
- **Offset-table (`dc.w Target-Base`) is NOT expressible** — `Cell::SymRef` is absolute-only. #1 blocker.
- **Out-of-range emit already ERRORS** (`$100` into `u8` → error), never truncates — totality holds,
  don't "fix" it.
- **Label model is GOOD, keep it**: `.name` locals are proc-scoped; non-export locals owner-mangle to
  `$proc$name` (cross-proc collisions impossible by construction — was the Plan-4 CRITICAL). No
  anonymous labels (decided against). Debug un-mangling (`foo.loop`) is a Spec-4 concern.
- **`@as_compat` currently silences only the proc-lints** (not `[layout.odd-field]`/branch) — revisit
  when struct/instruction ports land.
- **CLI:** `sigil emp <file.emp> [-o out.bin] [--hex]` compiles end-to-end; `sigil parse <file.emp>`
  parses only; `sigil build/diff --aeon <dir>` for the full ROM.

## References
- Research: `sigil/docs/superpowers/specs/2026-07-06-sigil-spec2-p7-language-completion-research.md`
- Plan 6: design `…/specs/2026-07-06-sigil-spec2-p6-as-compat-mixed-build-design.md`, plan
  `empyrean/docs/plans/2026-07-06-sigil-spec2-p6-as-compat-mixed-build.md`, completion handoff
  `…/notes/2026-07-06-spec2-plan6-complete-handoff.md`.
- Examples (real `.emp`): `examples/pitcher_plant.emp`, `examples/main.emp`.
- Memory notes: `spec2-progress`, `emp-data-table-dsl-candidates`, `jbra-jbsr-auto-reaching-branches`,
  `emp-language-design-principles`, `fable-role-reviewer-spec-writer`, `user-defers-sigil-technical-calls`.
