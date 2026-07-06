# Handoff — Sigil Spec 2 Plan 5 (capability sandbox: embed/import/zx0 + `as.*` bit-compat float)

**Purpose:** orient a fresh session (or Volence) to start Plan 5 cleanly, the way the Plan 2→3→4 handoffs
did. Written 2026-07-05 right after Plan 4 (IrStreamer & lowering) merged to master (`b166362`,
`--no-ff`). This is the *orientation* doc; write a detailed T-task plan doc (`superpowers:writing-plans`)
after the ⚠️ prerequisites below are assessed and the early decisions are settled, mirroring
`empyrean/docs/plans/2026-07-05-sigil-spec2-p4-lowering.md`.

## Where the branch stands (Plans 1-4 all MERGED to master)

`sigil-frontend-emp` now goes **all the way from `.emp` source to Core IR bytes+fixups**:
- **Plan 1:** lexer/parser/AST. **Plan 2:** comptime value evaluator (pure exprs, `const`, `comptime fn`
  recursion-bounded, control flow, §6.8 builtins, lambdas/`|>`, guards, step budget). **Plan 3:** types &
  layout (struct/bitfield/enum layout, `newtype`/`fixed`/`where`, comptime sum types + `match`, checked
  `Value::Data`). **Plan 4:** lowering — `Value::Data`→bytes+fixups, `asm{}`→`Value::Code`→IR, proc
  lowering, label hygiene, `patch`/`bind` primitive, sections + `vma:`/`here()` + cross-CPU fixups.
- Crate shape: the pure evaluator (`src/value.rs`, `src/eval/`, `src/layout.rs`) is **Core-free**; only
  `src/lower/` imports `sigil-ir`/backends (D-P4.1). The comptime evaluator + `src/eval/builtins.rs`
  (the §6.8 builtin dispatch) is where Plan 5's new builtins land.
- **881 workspace tests, clippy `-D warnings` clean, s4.bin byte-exact harness (`m1d_rom`) intact.**
- Full Plan-4 completion log + the gap list is in the `spec2-progress` memory note.

## Scope (Plan 5) — §6.6 + §6.7 of `SIGIL_SPEC2_LANGUAGE.md`

**IN:**
- **§6.7 The capability sandbox** — comptime is hermetic (no writes/clock/env/FFI/nondeterminism);
  external input ONLY via declared, **content-hashed** builtins, each an incremental-build edge:
  - **`embed(path, skip: N, len: N)`** — `BINCLUDE` parity, slicing included → a `Value::Data` (bytes).
  - **`import(path)`** — structured data (JSON/TOML) parsed into comptime values (arrays/structs/ints/strings).
  - **`zx0(data)`** — ZX0 compression via the **vendored salvador**, emitting the EXACT
    `[u16 BE size][u8 flags=0][u8 ver=2]` wrapper the `build.sh` loop hand-emits today.
  - The step budget already exists (Plan 2); a non-terminating chain names the innermost call (done).
- **§6.6 Numeric honesty — `as.*` vs `math.*`** — two float namespaces:
  - `math.sin/cos/...` = IEEE, for new code.
  - `as.sin`, `as.int`, ... = **bit-compatible with `asl 1.42 Bld 212`'s numeric routines** (the Core §7.1
    obligation — reuse Core's implementation + its four golden 256-byte vectors). Ported tables use `as.*`
    and **diff clean** against the reference ROM; the namespaces make the compat surface greppable and
    eventually deletable.

**OUT (later):** `@as_compat` reproduction + mixed `.asm`+`.emp` build + per-file port diff — **Plan 6**.
The Plan-4 gaps below are a SEPARATE follow-up pool (sequence them with Volence; some may slot before
Plan 5).

## ⚠️ PREREQUISITE 1 — Core readiness for `zx0` (salvador) and `as.*` float

Unlike Plan 4 (whose Core seam was ready), Plan 5's two hardest pieces depend on Core/vendored assets that
**are not yet in the sigil tree** as of the merge. **First action for the Plan-5 agent: assess these.**
- **salvador / ZX0.** No `salvador`/`zx0` crate or code exists in `sigil/crates/` today. The C source +
  a built binary live in **`aeon/tools/salvador/`** (`src/salvador.c`, `src/libsalvador.h`, `salvador`
  binary) and the decompressor is `aeon/engine/compression/zx0_decompress.asm`. Decide: vendor the C via a
  `-sys` crate + `cc`/`bindgen`, port the compressor to Rust, or shell out to a hashed binary. The spec
  says "vendored salvador" and requires the exact `[u16 BE size][u8 flags=0][u8 ver=2]` wrapper — confirm
  against the current `aeon/build.sh` ZX0 loop for byte-exact parity, and against `zx0_decompress.asm`'s
  expectations. This is the biggest unknown; settle the vendoring approach WITH Volence.
- **`as.*` asl-compat float.** The Core spec §7.1 names an obligation: an `asl 1.42`-bit-compatible float/
  trig implementation + **four golden 256-byte vectors**. **Confirm whether Core actually implemented it**
  (search `sigil-isa`/`sigil-ir`/`sigil-harness` for the float routines + the golden vectors — the earlier
  Plan-4 readiness pattern of "spec says reserved but code never built it" bit us once with
  `ProvFrame::Comptime`, so verify in CODE, not just the spec). If Core has it, Plan 5 reuses it; if not,
  the golden-vector-backed asl float is itself a Core deliverable to schedule first.

## ⚠️ PREREQUISITE 2 — content-hashing / incremental-build edge model

§6.7's builtins are each "a declared, content-hashed builtin, an incremental-build edge." The evaluator is
currently a batch tree-walker (salsa runtime deferred through M1, per the M0 design). Decide how a hash of
the embedded/imported file (and the salvador output) is captured — for Plan 5 a **content hash recorded
per builtin call** (so re-runs are deterministic and a future incremental engine has the edge) is enough;
you do NOT need to build the salsa runtime. Keep it minimal + deterministic (comptime hermeticity, §6.7).

## Design decisions to make early (load-bearing)

1. **salvador vendoring** (prereq 1) — `-sys` crate vs Rust port vs hashed-binary shell-out. Byte-exact
   wrapper is the acceptance gate. Settle with Volence.
2. **`import` value mapping** — how JSON/TOML maps to the comptime `Value` model (objects→structs or a
   generic map? arrays→`Value::Array`; numbers→`Int`/`Float`; the type a `data X = import(...)` expects).
   The Plan-3 `Value` model (`value.rs`) is the target; a schema/typed-import may be needed for struct
   layout — decide the typing story.
3. **`embed`/`import`/`zx0` return types** — `embed`/`zx0` → `Value::Data`; `import` → structured values.
   All must flow into `data` items (they already lower via Plan 4's `Value::Data` streamer — reuse it).
4. **`as.*` reuse vs reimplement** — if Core has the asl float + golden vectors, wire `as.sin`/`as.int`/…
   to it; else scope the asl-compat float as its own sub-milestone (with the four golden vectors as the
   diff gate). `math.*` is ordinary IEEE (Rust `f64`).
5. **Capability sandbox enforcement** — how "no FFI/clock/env" is guaranteed: the ONLY external edges are
   these three builtins; everything else stays pure. Confirm no accidental non-determinism leaks in.

## Suggested task shape (turn into a real plan doc; TDD, commit-per-task, mirroring Plans 2-4)

- **T0** — prereq spike: stand up the salvador vendoring (smallest slice: compress one buffer, emit the
  wrapper, byte-diff vs `aeon/build.sh`'s output) + confirm the `as.*` float / golden-vector Core state.
- **T1** — `embed(path, skip, len)` → `Value::Data` (hashed edge), diffed vs `BINCLUDE`. *(reviewed)*
- **T2** — `import(path)` (JSON then TOML) → comptime values; the value-mapping decision. *(reviewed)*
- **T3** — `zx0(data)` → the wrapped compressed `Value::Data` (byte-exact wrapper). *(load-bearing)*
- **T4** — `as.*` namespace wired to the asl-compat float (or the float sub-milestone) + the four golden
  256-byte vectors as the diff gate; `math.*` IEEE.
- **T5** — corpus: a real ported table via `as.*` diffs clean; an `embed`+`zx0` pipeline reproduces a
  `build.sh` compressed blob byte-for-byte + whole-branch review.

## Plan-4 follow-up pool (sequence with Volence — some may slot BEFORE Plan 5)

From the Plan-4 whole-branch review + corpus (all recorded in `spec2-progress` memory):
1. **`dbcc`/`dbra` with a label target does not lower** (clean diagnostic today, not a crash). `dbra` is
   ubiquitous in real Sonic procs → **top follow-up**. Needs a backend `lower_dbcc` (the ISA has
   `encode_dbcc`; the m68k backend has `lower_branch` for Bra/Bsr/Bcc but no public `lower_dbcc`; the AS
   front-end's `lower_m68k_dbcc` is behind the quarantine). Small-to-moderate: add the backend method +
   route `Dbcc` in `src/lower/code.rs` (currently line ~85 only routes Bra/Bsr/Bcc).
2. **`ProvFrame::Comptime` structured frame** — the Core spec §4.1 calls it "reserved" but it was NEVER
   built in Core code (no `sigil-ir/src/prov.rs`; `sigil-span`'s `Diagnostic` has no provenance/notes
   field). Plan 4 ships a functional `[prov.comptime]` **Note** substitute. Full §9 compliance needs a
   Core provenance data-model change (add the type + a field on `DataFragment`/`Diagnostic` + linker
   rendering). This is a Core-track task.
3. **`patch`/`bind` surface** — the mechanism (`src/lower/patch.rs`) is tested standalone, but `.emp` has
   no section-emission-statement position to run comptime `patch`/`bind` into a section stream, so
   `[patch.unbound]` can't fire from a real program. Needs a surface-design decision (where do
   emit-forward-bind-later statements live?).
4. **`CodeItem::Inline`** (Data spliced into a code stream, §6.2) is currently unreachable (a `Value::Data`
   in an `asm{}` statement-call position errors instead of inlining).
5. Minor: `fixed<I,F>` can emit non-{1,2,4}-width scalars vs the `Cell::Scalar` doc (harmless, byte-correct);
   `data X = <const>` type inference only fires for struct-literal initializers; proc-name-as-pointer-value
   (`code: init` needs `code: "init"` — only a `FnRef`/string is a valid pointer target); SST overlay +
   `timer(a0)` field-access-as-displacement (§4.6) does not lower; prelude names / cross-module `use`
   (S2-D3, still deferred — the prelude is data, finalized at first real port).

## Process to keep (it worked in Plans 2-4 — caught a CRITICAL in each)

- Subagent-driven with **two-stage reviews** (spec compliance THEN code-quality via
  `superpowers:code-reviewer`) on load-bearing tasks; TDD per task; commit after each; green gate
  (`cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings`) before every commit.
- **Add a whole-branch review at the end** — in Plans 2, 3 & 4 it caught the CRITICAL cross-feature bug the
  isolated reviews missed (Plan 4: a cross-proc non-export label collision from a per-proc-reset hygiene
  counter). Run it as ≥1 adversarial reviewer that CONSTRUCTS and RUNS cross-feature `.emp` programs, not
  just reads the diff. **Byte-diff against `aeon/build.sh`'s output wherever a byte argument exists** (§8.3)
  — this is Plan 5's whole point for `embed`/`zx0`/`as.*`.
- Ground semantics in the **spec** (`empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` §6.6/§6.7 + Core §7.1) and, for
  salvador/ZX0 and `as.*` float, byte-diff against the AS/`build.sh` reference. Record every design call in
  the plan doc (`D-P5.x` numbering).
- **Milestone boundary:** Plan 5 is a milestone — do NOT merge to master without a Volence checkpoint,
  same as Plans 2, 3 & 4.

## Reference
- Plan-4 plan doc `empyrean/docs/plans/2026-07-05-sigil-spec2-p4-lowering.md` (`## Core-readiness
  assessment` for the exact IR seam API + the `Value::Data`/`Value::Code` shapes Plan 5 reuses).
- `spec2-progress` memory note (`## How to apply` + the Plan-4 entry) for the merged state + full gap list.
- Deferred ledger (Spec 2's own) at the bottom of `SIGIL_SPEC2_LANGUAGE.md` for S2-D3/D6/D7/D8.
