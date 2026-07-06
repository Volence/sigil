# Handoff — Sigil Spec 2 Plan 6 (`@as_compat` + mixed build + per-file byte-diff) — DONE, awaiting checkpoint

**State:** Plans 1–5 merged; **Plan 6 IMPLEMENTED on branch `spec2-p6-as-compat`, NOT merged** (milestone
rule — needs a Volence checkpoint, same as Plans 2–5). Written 2026-07-06 overnight. Worktree:
`sigil/.worktrees/spec2-p6-as-compat`, 4 commits off master `11a6bc9`. Design:
`docs/superpowers/specs/2026-07-06-sigil-spec2-p6-as-compat-mixed-build-design.md`. Plan:
`empyrean/docs/plans/2026-07-06-sigil-spec2-p6-as-compat-mixed-build.md` (decisions D-P6.1..D-P6.6).
Supersedes the pre-Plan-6 orientation note `2026-07-06-spec2-plan6-handoff.md`.

## What landed (T1–T4, all green-gated per commit)
- **T1** `37a53ad` — per-file byte-diff harness (`crates/sigil-cli/tests/ports.rs`): `as_reference()`
  independently AS-assembles the `.asm`; `emp_candidate()` runs the emp pipeline; `assert_byte_identical`
  reports the first-diff offset. Vendored `song_drumtest.asm` + `sfx_33.asm` under `tests/vectors/ports/`
  (hermetic — no reach into `aeon/`).
- **T2** `8697e43` — the port `song_drumtest.emp` (`data … : [u8; 82] = [ … ]`), **byte-identical** to the
  AS-assembled original. Target = `song_drumtest.asm` (82 bytes, pure `dc.b`, even → `align 2` no-op).
- **T3** `306d27f` — `@as_compat` wired: `lower_module` reads `file.attrs`; a `proc::ProcCtx{cpu,as_compat}`
  silences the two heuristic modernization WARNINGs (`[proc.undeclared-fallthrough]`,
  `[proc.clobber-undeclared]`) and **never** the hard `[proc.fallthrough-separated]` ERROR. Proven
  byte-neutral on the data path.
- **T4** `0cf4f4d` — mixed-build link seam: emp defines `Song_DrumTest` @ VMA $8000, synthetic AS
  `dc.l Song_DrumTest` consumer resolves across the seam to `00 00 80 00`; a cross-front-end name
  collision is a hard link `Error`.

## Verification (done by the orchestrator, not just the impl agent)
- Green gate re-run independently: **947 passed, 0 failed**; `clippy --workspace --all-targets -D warnings`
  clean; s4.bin harness (`m1d_rom`/`m1d_debug_rom`) intact.
- Adversarial check: corrupting one byte in the `.emp` changes the emitted output (`08…`), so the diff is
  genuine — the emp front-end emits what its source declares, not an AS echo.
- **Two-stage review** (spec-compliance + code-quality via `superpowers:code-reviewer`): **CLEAN — no
  Critical or Important findings.** The Plans-2/3/4 CRITICAL class has no analogue here.

## Known gaps / low-severity notes to weigh at checkpoint (none block merge)
1. **`@as_compat` silences only the proc-lints, not `[layout.odd-field]` or others.** Under-silencing (never
   hides an error, never changes bytes) and unexercised by a data-only target. When struct/instruction ports
   land, decide whether `@as_compat` should blanket all faithful-port lints; add the threading then.
2. **Design prose references `[branch.suboptimal-size]`, which does not exist** — the codebase has
   `[branch.missing-size]` (an ERROR, correctly NOT silenced under `@as_compat`, which pins widths). Prose
   to reconcile when Plan 7 freezes the spec.
3. **The byte-proof is intentionally thin** (pure-byte mechanism proof, D-P6.1). The *representative* data
   files (particle_anims/test_mappings/sonic_anims) need the **offset-table** (`dc.w Target-Base`) that
   `.emp` cannot express yet — the #1 Plan 7 item (see below).
4. ROM-slice cross-check (D-P6.4 "belt-and-suspenders") dropped for hermeticity — reasonable; design phrased
   it conditional.

## The Plan 6 payoff insight → Plan 7
Plan 6 proves the *mechanism*. The real finding: **`.emp` cannot express a symbol difference in data
(`Cell::SymRef` is absolute-only), so the offset-table pattern blocks 3 of 4 real data files** — and that
pattern is also the #1 idiom by frequency (14k S3K / 4.6k S2). The Plan 7 language-completion **research is
DONE** (`docs/superpowers/specs/2026-07-06-sigil-spec2-p7-language-completion-research.md`, on master): top
buys are the bidirectional offset-table (table + id-enum), a typed state machine, a user-definable
byte-command DSL, and a bank/window placement type (the hardest sound-migration stumble).

## Checkpoint ask
Merge `spec2-p6-as-compat` to master (`--no-ff`, mirroring Plans 2–5)? Then Plan 7 = language finalization
(ratify the research → close the deferred ledger → freeze the spec → implement the finalized features),
starting with the offset-table.
