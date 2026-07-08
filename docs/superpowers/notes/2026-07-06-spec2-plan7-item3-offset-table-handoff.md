# Handoff — Sigil Spec 2 · Plan 7 · next up: backlog #3 (bidirectional offset-table)

Orientation for a fresh agent. Written 2026-07-06, after Plan 7 Step 1 + backlog #2 merged.

## Where things stand
- **Plans 1–6 MERGED.** Plan 7 = language finalization (research done → ratify → freeze → implement).
- **Plan 7 Step 0 (ratify) DONE:** the one open decision — text encoding — is settled: char/string literals
  default **raw ASCII**; a **charmap** is a separate opt-in ASCII→tile layer. See the `emp-text-encoding-decision`
  memory. (The engine-side runtime `asciiToCharmap`/`DrawText` mechanism lives in aeon, built when the text
  subsystem is; Sigil's charmap is the compile-time half — a later Tier-3 feature, not blocking.)
- **Plan 7 Step 1 (lexical gaps) MERGED+pushed** (master `8dc13e7`): binary `%1010` literals, `-1` signed-sentinel
  convention (zero new code), char `'A'` literals (raw ASCII + escapes `\n \t \\ \' \0`), string→bytes in data
  (`bytes("..")`, `[u8;N]="..."`, author-controlled termination; a string in a POINTER field stays a symbol ref).
- **Plan 7 backlog #2 (symbolic operands in straight-line 68k instrs) MERGED+pushed** (master `e5e01a7`):
  `move.w Foo,d0` / `lea Foo,a0` / `move.l Foo,d0` / `tst.w Foo` / `clr.w Foo`. See "What #2 built" below — it's a
  reusable template.

## Process (NON-NEGOTIABLE — it keeps finding real bugs)
- Isolated git worktree `sigil/.worktrees/<branch>`; subagent-driven; **TDD per task; commit-per-task.**
- **Two-stage reviews** (spec-compliance THEN code-quality via `superpowers:code-reviewer`) on load-bearing tasks;
  a **whole-branch adversarial review** at the end that CONSTRUCTS + RUNS cross-feature programs and **byte-diffs
  against the AS reference** (`sigil-frontend-as`, an INDEPENDENT front-end — a real cross-check) wherever a byte
  argument exists.
- **Green gate before EVERY commit:** `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings`.
- Ground semantics in the spec, BUT where spec and code disagree, **the CODE is authoritative** — verify by grep.
- **Milestone: do NOT merge to master without a Volence checkpoint.** Volence has chosen `--no-ff` merge + push
  for the last several milestones.

## NEXT: backlog #3 — bidirectional offset-table (`dc.w Target-Base`)
The #1 data idiom by volume (~14k uses S3K / 4.6k S2) and the **Plan-6 blocker**: `.emp` cannot express a
self-relative word offset today. Concretely, mappings/DPLC/art pointer tables are `dc.w Target-Base` (a 16-bit
offset from a base label to each target), and:
- **The blocker (verified):** `Cell::SymRef` is **absolute-only** — it yields an absolute `Abs16Be`/`Abs32Be`
  fixup, never a symbol *difference*. There is no way to emit `Target - Base` as a relocation. This needs a new
  fixup kind (a symbol-difference / PC-or-base-relative 16-bit offset) resolved in `sigil-link`, plus the `.emp`
  surface + data-lowering to produce it (and an `id-enum` companion — the reverse index).
- **CRITICAL scoping note from the research (R1):** keep the **DATA offset-table** (`dc.w Target-Base`) SEPARATE
  from **state dispatch**. State dispatch must be **encoding-agnostic** (Sonic uses self-relative word offsets,
  Vectorman raw absolute 32-bit pointers, Ristar pre-shifted ×4 indices, Treasure word-index tables) — do NOT bake
  Sonic's offset form into dispatch. #3 is the DATA table only.
- Start by investigating (Explore agent) exactly how `sigil-link` resolves `Fixup`s and how the AS front-end
  (`sigil-frontend-as`) already emits `dc.w Target-Base` (it does — it's the byte-diff reference), then mirror that
  as a new fixup kind + `.emp` surface. Ground the width/overflow behavior against AS byte-for-byte.

## What #2 built (a reusable template for deferred-width / relocation work)
- **`Fragment::RelaxAbsSym { short: RelaxCandidate, long: RelaxCandidate, target, span }`** in `sigil-ir`
  (`crates/sigil-ir/src/lib.rs`), where `RelaxCandidate { bytes, fixup }`. `short` = abs.w encoding (`Abs16Be`
  fixup), `long` = abs.l (`Abs32Be`). The FRONT-END encodes BOTH candidates; `resolve_layout` (`sigil-link/
  src/relax.rs`) only **selects** via `asl_width_rule` on the resolved target address, inside the existing
  grow-only relaxation fixed-point (monotonic W→L, pass-cap terminates, downstream labels shift). **No m68k
  encoding lives in the linker** — keep it that way for any future relaxable fragment.
- Lowering (`crates/sigil-frontend-emp/src/lower/code.rs`): `lower_m68k_abs_sym` + `operand_has_ext_words`. Fixup
  offset = `short.len()-2`, exact because the emp `CodeOperand` model has NO absolute/indexed/PC-rel EA except
  `Sym` ⇒ the symbolic operand is the ONLY extension-word producer.

## Verified facts a fresh agent MUST know
- **`asl_width_rule`** (`sigil-ir/src/width.rs`): abs.w for addresses in `[0x0000,0x7FFF] ∪ [0xFF_8000,0xFF_FFFF]`,
  else abs.l. It is the single source of truth for 68k absolute width; reuse it, never hand-roll.
- **`sigil-frontend-as` is the byte-diff reference AND it is an INDEPENDENT implementation** — its width selection
  is an eager multi-pass fold (`abs_ea_from_expr`, emits finished `Fragment::Data`), sharing only `asl_width_rule`.
  So emp-vs-AS byte-diffs genuinely cross-check two implementations. Use it (or `crates/sigil-cli/tests/ports.rs`'s
  `as_reference()`/`emp_candidate()` harness) for #3's byte-diffs.
- **`Cell::SymRef` is absolute-only** — the #3 blocker (no symbol-difference relocation exists yet).
- **`.emp` surface gotchas** (learned in #2/#3-adjacent work): symbols that must relocate at link time have to be
  **labels** (a `vma:` section or a forward label), NOT `const` (comptime-only, folds early). `.emp` has no
  `byte`/`bytes` *keyword* for arrays — ordinary data is `data X: [u8;N] = [...]`; `byte(..)`/`bytes(..)`/`++` are
  the Data-monoid builtins. Out-of-range emit ERRORS (totality), never truncates — don't "fix" it.
- **Symbolic-operand scope is a FIRST CUT.** Deferred (each its own future task, all currently DIAGNOSED not
  mis-encoded): explicit `.w`/`.l` operand-width syntax, `#Sym` immediate, `Sym(pc)` PC-relative, indexed modes,
  and abs-operand-combined-with-another-ext-word-operand (`#imm`/`d(An)`).
- **CLI:** `sigil emp <file.emp> [-o out.bin] [--hex]` compiles end-to-end; `sigil parse <file.emp>` parses only;
  `sigil build/diff --aeon <dir>` for the full ROM.

## The remaining Plan-7 backlog (ordered; #2 done, #3 next)
3. **Bidirectional offset-table** (`dc.w Target-Base` + id-enum) — NEXT. Data table only; keep separate from dispatch.
4. **Scan/manifest module resolution + map-file placement + game prelude (S2-D3)** — unblocks code ports at all.
5. **`assert!` / capacity-refined regions** (~195 aeon hand-guards; cheap, high coverage).
6. **State machine + SST scratch overlay** — object-code migration pair. Make dispatch ENCODING-AGNOSTIC (R1);
   research R2 suggests MERGING this with #9 into one scripted-coroutine construct with `yield` as a primitive.
7. **Bank/window placement** (gates the sound subsystem migration).
8. **`jbra`/`jbsr` + conditional-branch relaxation** (see the `jbra-jbsr-auto-reaching-branches` memory).
9. **Byte-command DSL / scripted-coroutine w/ `yield`** (largest; may stage; may merge with #6 per R2).
10. **Compression builtins** — `s4lz` + the classic-Sonic family (`nemesis`/`kosinski`/`kosinski_m`/`enigma`/`saxman`).

Plan 7 ends when the spec is frozen + finalized features implemented → then the migration campaign (68k source
first, cycle-exact Z80 DAC driver last) → Spec 5 deletes the AS front-end (gated on "every load-bearing AS feature
has a byte-exact Spec-2 equivalent").

## References
- Research (the exact solutions + Parts I–V): `sigil/docs/superpowers/specs/2026-07-06-sigil-spec2-p7-language-completion-research.md`
- Prior handoff (Step 1 → this work): `sigil/docs/superpowers/notes/2026-07-06-spec2-plan7-implementation-handoff.md`
- Examples (real `.emp`): `sigil/examples/pitcher_plant.emp`, `sigil/examples/main.emp`.
- Memory notes: `spec2-progress`, `emp-text-encoding-decision`, `emp-data-table-dsl-candidates`,
  `jbra-jbsr-auto-reaching-branches`, `emp-language-design-principles`, `fable-role-reviewer-spec-writer`,
  `user-defers-sigil-technical-calls`.
