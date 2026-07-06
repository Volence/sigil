# Handoff — Sigil Spec 2, post-Plan-5 (next: Plan 6 `@as_compat` + mixed build, OR a follow-up)

**Purpose:** orient a fresh session (or Volence) on what to do after Plan 5 merged. Written 2026-07-06
right after Plan 5 (capability sandbox + `as.*` float) merged to master (`bfffd0a`, `--no-ff`). Mirrors
the Plan-2→3→4→5 handoff style. Assess the ⚠️ prerequisites, settle early decisions, write a T-task
plan doc (`superpowers:writing-plans`) mirroring
`empyrean/docs/plans/2026-07-05-sigil-spec2-p5-sandbox-float.md`, checkpoint with Volence, THEN build.

## Where the branch stands (Plans 1–5 all MERGED to master)

`sigil-frontend-emp` takes `.emp` source **all the way to Core IR bytes+fixups**, and comptime now has
its hermetic capability sandbox + the asl-compat float surface:
- **Plans 1–4:** lexer/parser/AST → comptime evaluator → types & layout → lowering (Data→bytes+fixups,
  `asm{}`→Code→IR, procs, hygiene, sections, cross-CPU fixups). See the `spec2-progress` memory note.
- **Plan 5 (just merged):** the §6.7 capability sandbox — `embed(path, skip, len)`, `import(path)`
  (JSON/TOML), `zx0(data)` (byte-exact vs `aeon/build.sh`'s wrapper via the vendored
  `sigil-salvador-sys` crate) — plus §6.6 `as.*`/`math.*` float namespaces (std f64; `as.int`=floor;
  gated on the four sine goldens). Sandbox is hermetic: include-root path resolution, symlink
  containment, a content-hash capture ledger, and a source-scan "no other external edges" tripwire.
  Also landed the T0′ **`dbcc`/`dbra` lowering** follow-up (backend `lower_dbcc` + `code.rs` route).
- **Test state:** workspace green (79 result-OK suites, 0 failures), clippy `-D warnings` clean, s4.bin
  regression harness (`m1d_rom`/`full_debug_rom_matches_assembled_reference`) intact. The Plan-5 whole-
  branch adversarial review (two prongs, ~39 cross-feature programs) was **CLEAN — zero findings**.

## What's next — pick WITH Volence (Plan 6, or promote a follow-up first)

### Option 1 — Plan 6: `@as_compat` reproduction + mixed `.asm`+`.emp` build + per-file port diff
This was always the milestone after Plan 5 (see the deferred ledger in `SIGIL_SPEC2_LANGUAGE.md` and the
Plan-5 handoff's "OUT (later)"). The goal: take a REAL Aeon `.asm` file, port it to `.emp`, and prove
the ported output is byte-identical to the AS-assembled original — the payoff the whole `as.*`/sandbox
surface was built for. Likely shape: an `@as_compat` attribute/mode that pins the emp output to AS
conventions, a build path that assembles a mix of `.asm` (via `sigil-frontend-as`) and `.emp` (via
`sigil-frontend-emp`) into ONE image, and a per-file diff harness. ⚠️ Prereq to assess in CODE (not the
spec): how `sigil-frontend-as` and `sigil-frontend-emp` modules combine at the link layer today, and
whether there is a shared section/symbol namespace to merge them.

### Option 2 — promote a follow-up BEFORE Plan 6 (some are near-prerequisites)
Sequence these with Volence; the first is arguably a real prerequisite for any live porting:
1. **emp-compile CLI command (likely a prerequisite).** There is NO `sigil build --emp <file>` path
   yet — `embed`/`import` only work through `lower_module` with a test-set `include_root`; through the
   production CLI they'd hit `[sandbox.no-root]`. A CLI entry that parses a `.emp` file, sets
   `include_root` from the source file's parent (canonicalized/absolutized — see the Plan-5 T5 review
   note about relative-root cwd-dependence), lowers, and emits a `.bin` makes the sandbox usable for
   real work. Small-to-moderate; `LowerOptions.include_root` already exists.
2. **Reserve `math`/`as` as declaration names** (parser follow-up). Today a user `enum math`/`let as`
   parses, and a 2-segment CALL like `math.Red(x)` is hijacked to the float namespace →
   `[float-ns.unknown]` (an error, never silently wrong — see `call.rs`'s documented limitation).
   Clean fix: reject `math`/`as` as declaration names with a diagnostic at the decl.
3. **Pure-Rust salvador port (delete the C).** D-P5.1 vendored salvador as a `-sys` crate for
   byte-exactness by construction; it sits behind a swappable Rust `zx0()` seam
   (`sigil_salvador_sys::compress`). A faithful pure-Rust port (shrink.c + matchfinder.c +
   libdivsufsort, ~4k LOC, gated on the committed `.zx0`/`sample.zx0raw` vectors) removes the build-time
   C dependency. Volence's stated preference is Rust; this is the clean future swap, not milestone-
   blocking.
4. **Carried Plan-4 pool** (still open): `ProvFrame::Comptime` structured frame (Core provenance model,
   for full §9); `patch`/`bind` section-emission surface (no emit-forward-bind-later statement position
   yet); `CodeItem::Inline` (Data spliced into a code stream, §6.2, unreachable). See the `spec2-progress`
   memory Plan-4 entry.
5. **Minor / latent:** module-level capture-ledger aggregation (today per-data-item — each item builds a
   fresh `Evaluator`); `libm`-crate float determinism (std f64 today, cross-platform bit-exactness not
   guaranteed by IEEE for transcendentals — gated on goldens so a mismatch would be caught);
   no-other-edges tripwire is `#[cfg(test)]`-brittle if another `src` module ever grows a test touching
   `std::fs`; `fixed<I,F>` non-{1,2,4} widths; `data X = <const>` inference; proc-name-as-pointer-value;
   SST overlay + field-access-as-displacement; prelude / cross-module `use` (S2-D3, still deferred).

## Process to keep (it worked in Plans 2–5 — caught a CRITICAL in each of 2/3/4; Plan 5 was clean)
- Isolated git worktree (`sigil/.worktrees/<branch>`), subagent-driven, TDD per task, commit-per-task.
- **Two-stage reviews** (spec compliance THEN code-quality via `superpowers:code-reviewer`) on
  load-bearing tasks; a **whole-branch adversarial review** at the end that CONSTRUCTS and RUNS
  cross-feature programs (not just reads the diff) and **byte-diffs against the AS/`aeon/build.sh`
  reference wherever a byte argument exists**.
- Green gate before EVERY commit: `cargo test --workspace` + `cargo clippy --workspace --all-targets --
  -D warnings`. Keep the s4.bin harness green (no ROM regression).
- Ground semantics in the spec (`empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` + `SIGIL_CORE_SPEC.md`), but
  where spec and code disagree, **the code is authoritative** (verify by grep, not by trusting a
  "reserved" doc line — the `ProvFrame::Comptime` lesson).
- **Milestone boundary:** do NOT merge to master without a Volence checkpoint (same as Plans 2–5).

## Reference
- Plan-5 plan doc `empyrean/docs/plans/2026-07-05-sigil-spec2-p5-sandbox-float.md` (decisions
  D-P5.1..D-P5.8 + the salvador/`as.*` readiness assessment). NOTE: this doc + the Plan-4 doc are
  currently UNTRACKED in the empyrean repo (Volence's docs cadence) — commit them there when convenient.
- `spec2-progress` memory note (full Plan-1..5 log + gap lists).
- Plan-5 code: `sigil-frontend-emp/src/eval/{sandbox.rs, float_ns.rs}`, `src/lower/mod.rs`
  (`include_root`/`Placement`), `crates/sigil-salvador-sys/` (vendored ZX0). Fixtures/goldens under
  `sigil-frontend-emp/tests/vectors/`. Reference: `aeon/build.sh:106-121`, `aeon/tools/salvador/`.
</content>
