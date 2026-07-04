# Sigil M1.C — AS 68k Front-End Fidelity — Design

**Status:** approved (brainstorm 2026-07-03)
**Milestone:** M1.C (follows M1.B linker, merged `943aa2c`)
**Crate:** `sigil-frontend-as` (the quarantined AS oracle; SIGIL_CORE_SPEC §7)
**Reference toolchain:** `asl 1.42 Beta Build 212` → `p2bin` → `convsym` (no-op) → `fixheader`
**Reference source pin:** aeon `9bacc93` (`s4.bin` 458666 B under current config; checksum `$18E`)

---

## 1. Goal & Scope

Grow `sigil-frontend-as` from Z80-only (M0 Plan 4) to parsing and lowering **real Aeon
68000 source** through the existing `IrStreamer` boundary (SIGIL_CORE_SPEC §4.9) into the
M1.A 68000 encoder (`sigil-backend-m68k`) and the M1.B linker's width-fixpoint / fixup
machinery (`sigil-link`).

Everything in scope is drawn from **SIGIL_CORE_SPEC §7** (the quarantined AS front-end).
No new AS-specific IR nodes may be introduced (§7.4 contamination safeguard): every AS
quirk is *lowered away* inside this crate into ordinary IR. The crate keeps its one-way
dependency (`sigil-frontend-as → sigil-ir`, plus now `sigil-backend-m68k` for lowering,
matching the existing `sigil-backend-z80` dependency); nothing else may depend on it.

### 1.1 What is already CPU-agnostic (no M1.C work)

Verified by architecture survey (2026-07-03):

- **Multi-pass eval loop** (`eval.rs:30–73`): bounded (`PASS_CAP = 8`), monotonic,
  seed-forward symbol resolution, non-convergence diagnostic. Backend-independent.
- **State stack** (`state.rs`): `cpu`, `vma_base`, `padding` (68k default ON, Aeon sets
  OFF globally), `supmode`, save/restore stack. 68k defaults already present.
- **Lexer** (`lexer.rs:12`): already `cpu`-parameterized; `$hex` handled for 68k.
- **Operand-atom layer** (`operands.rs`): `OperandAtom` is structural / CPU-agnostic.
- **IrStreamer boundary** (`sigil-ir/src/backend.rs:65`): `switch_section`, `emit_data`,
  `emit_fill`, `reserve`, `define_label`, `diag`. Unchanged.

### 1.2 Exit criterion

The front-end assembles **every 68k construct present in Aeon source**, verified
**byte-exact at file/section granularity via asl-diff** — i.e. Core's emitted bytes for a
given source unit match `asl`'s for that unit. Full-image `sha256(sigil_s4.bin) ==
sha256(ref_s4.bin)` for `__DEBUG__` on/off, plus deletion of the ~42-symbol stub table, is
**M1.D** (A+C resolve each other there — the stub table exists only because the front-end
could not yet define those symbols).

---

## 2. Key Design Decisions

### D1 — Backend multiplexing: split, do not unify

The Z80 and 68000 backends have deliberately different inherent method signatures
(`Z80Backend::lower_rel/lower_abs16` vs `M68kBackend::lower_branch/lower_jmp_jsr_sym/
lower_pcrel_ea`). Forcing them behind one `lower()` trait would leak a false abstraction.
The **real** shared abstraction is already `IrStreamer` (emit_data/fill/reserve/
define_label), which both paths funnel into.

**Decision:** dispatch at `lower_instruction` (`eval.rs:893`) on `state.cpu`:
- refactor the existing Z80 logic out into `lower_z80(...)` (pure move; tests stay green),
- add a parallel `lower_m68k(...)`.

Both backend structs are stateless fields on `Asm` (`z80: Z80Backend`, `m68k:
M68kBackend`). Each CPU path is independently readable and testable. The `Lowered` enum
(`eval.rs:137`) gains 68k forms (`Branch { size, target }`, `JmpJsrSym { is_jsr, target }`,
`PcRelEa { inst, pcd16_offset, target }`) or the m68k path bypasses it with its own local
routing — implementer's choice at plan time, whichever keeps each path clearest.

### D2 — `deform_table_sine`: bit-match gated on goldens, source-cure as recorded fallback

`deform_table_sine` (`engine/parallax_macros.inc:211`) emits
`rept 256 / dc.b int(AMPLITUDE * sin(6.283185307179586 * i / PERIOD))`, instantiated 4×
(ojz_windy A=96/P=64, rocking A=20/P=64, haze A=16/P=64, shimmer A=8/P=32). These bytes are
on the byte-exact ROM path; AS's float `sin()` + `int()`-truncation are in fidelity scope
(SIGIL_CORE_SPEC §7.1).

**Decision:**
1. Extract the 4 golden 256-byte tables from the reference ROM **unconditionally** (Spike
   0) — committed golden vectors, same pattern as the Z80 encoder vectors. The harness
   needs them regardless of which strategy wins.
2. Implement the real fold in Rust `f64` with `int()` = **truncate-toward-zero** (confirm
   vs asl in Spike 0), and asl-faithful `sin`/`cos`. Gate on the goldens.
3. asl 1.42's `sin`/`cos` operate on C `double`; `f64` is *likely* bit-identical, and these
   amplitudes rarely land on a `.5` truncation boundary. If it resists after **one bounded
   task** (T8), fall back to §12 R7 **source-level cure**: pre-bake the 4 tables to a
   `BINCLUDE` in Aeon, rebuild the reference with `asl`, re-baseline A1 (new length/checksum
   landmarks recorded in SIGIL_CORE_SPEC header). A deliberate, recorded decision — never
   silent drift.

Note: `cos()` is used 3× elsewhere in 68k source (not in this macro) — the float-builtin
work (T3) must cover `sin`, `cos`, and `int` together.

### D3 — AS-faithful operator + builtin layer is a foundation task, lands first

The current Pratt parser (`expr.rs`) supports only `* / + - << >> & | = != < > <= >=` and
has **zero builtins**. Aeon 68k source additionally uses (verified counts):
`mod` (×63), `<>` (×59), `!=` (×33), `||` (×20), `&&` (×5), plus `#`=modulo, `!`=bitwise-or,
`~~`=boolean-not, and comparisons that must **yield `0` / `-1` masks** (§7.1). Builtins used:
`substr` (×32), `strlen` (×18), `sin` (×13), `strstr` (×11), `lowstring` (×5), `cos` (×3),
`int` (×1).

**Decision:** land the operator set (T2) and builtin table (T3) **before** 68k instruction
lowering, because load-bearing macros depend on them (e.g. `deform_table_sine` needs
`#`/`<>`/`sin`/string-compare/`\{}`-interpolation just to expand). Both asl-diff-gated on
expression snippets.

### D4 — `__DEBUG__` debugger surface stays in M1.C, lands last

`switch`/`lowstring`/`strstr`/`%<…>` produce real ROM bytes under `__DEBUG__`, and A2
requires proving both `__DEBUG__` on/off. So they are **in M1.C scope** (T9) — but land
**after** the non-debug path is byte-exact, so they never block the mainline. (User
confirmed this boundary in brainstorm.)

### D5 — Bug-for-bug `strstr`

AS `strstr` **fails to check the last character of the haystack** (`debugger.asm:664`
comment). Two distinct in-source compensations depend on it (lines 664, 726). Core's
`strstr` builtin returns `-1` whenever a match exists **only at the final character
position** (correct 0-based index otherwise), matching `asl 1.42 Bld 212`. This quirk lives
**only** in the front-end builtin table, never in IR (§7.4).

### D6 — Dotted-local qualification for jmp/jsr

The M1.B linker folds `JmpJsrSym` targets in **global scope** (`sigil-link/src/lib.rs:132`:
"global scope at link time"). Therefore the front-end must **qualify dotted-local targets**
(`.loop` → `EnclosingGlobal.loop`) before emitting the `JmpJsrSym` fragment / branch fixup.
This is a front-end responsibility, not a linker one.

---

## 3. Gap Analysis by Subsystem

| Subsystem | Current (Z80) | M1.C graft | Effort |
|---|---|---|---|
| Multi-pass loop | generic | none | — |
| State stack | 68k-ready | none | — |
| Lexer | cpu-aware | none | — |
| Operand-atom parse | structural | add `-(An)`,`(An)+`,PC-rel,abs.w/.l atoms | med |
| Expr operators | subset | `mod`/`#`/`<>`/`!=`/`\|\|`/`&&`/`~~`, masks | med |
| Builtins | none | strstr(bug)/substr/strlen/sin/cos/int/lowstring | med |
| `\{expr}` interp | error/fatal only | generalize to all string contexts | low |
| CPU→backend dispatch | Z80 hardcoded | split `lower_z80`/`lower_m68k` | high |
| Mnemonic table | 26 Z80 | 68k table + `.b/.w/.l/.s` suffix parse | med |
| Register words | Z80 only | d0–d7/a0–a7/pc/sp/ccr/sr | low |
| Operand convert | Z80 | `convert_atoms_m68k` + EA modes | med |
| Instr lowering | lower/rel/abs16 | branch/jmp-jsr-sym/pcrel-ea + qualify | high |
| Data directives | `db`/`dc.b`/`dw` | `dc.w`/`dc.l`/`ds.*`/`align`/`even`+padding | low |
| struct/`_len` | capture only | `_len` symbol + `if _len<>N/error` | low |
| Macro args | positional | keyword args + ALLARGS/.ATTRIBUTE/MOMCPUNAME | med |
| rept/irp/irpc/while | rept only | irp/irpc/while + sine fold | med |
| Debugger surface | none | switch/lowstring + `!name` escape | med |

---

## 4. Decomposition (ordered subagent tasks)

Each task: spec-slice → TDD plan → fresh implementer → review. asl-diff / golden-vector
gates throughout (the M1.B pattern). Lighter controller-verify on transcription tasks
(T2/T3/T6); full two-stage review on high-latitude tasks (T1/T5/T8/T9); whole-branch review
before `--no-ff` merge to master.

- **Spike 0** (pre-dispatch de-risk, no production code):
  extract 4 sine goldens from ref ROM; confirm `int()` truncation direction + asl float
  probe (`sin`/`cos` bit behavior); enumerate the **exact** operator + builtin + directive
  set across *all* 68k source so T2–T9 scopes are pinned, not guessed.
- **T1** Backend multiplexing refactor — extract `lower_z80`, add `m68k` field, dispatch on
  `state.cpu`. Pure refactor; existing Z80 tests stay green. (high-latitude)
- **T2** AS-faithful operators + generalized `\{expr}` interpolation + string comparison,
  comparisons→`0`/`-1` masks. asl-diff-gated on expression snippets. (transcription)
- **T3** Builtin table — `strstr` bug-for-bug (D5) + `substr`/`strlen`/`sin`/`cos`/`int`/
  `lowstring`. asl-diff + strstr golden cases. (transcription)
- **T4** 68k mnemonics + `.b/.w/.l/.s` size-suffix parse + operand-atom extensions
  (`-(An)`, `(An)+`, `(d16,PC)`, `abs.w`/`abs.l`).
- **T5** `convert_atoms_m68k` + lower routing (branch / jmp-jsr-sym / pcrel-ea) +
  **dotted-local qualification** (D6). (high-latitude)
- **T6** `dc.w`/`dc.l`/`ds.b`/`ds.w`/`ds.l`/`align`/`even` + padding-state interaction
  (Aeon `padding off` global → odd `dc.b` runs unpadded). (transcription)
- **T7** struct/`_len` + `if _len<>N/error` assertion; keyword macro args; ALLARGS /
  `.ATTRIBUTE` / `MOMCPUNAME` `pbyte` dispatch (`db` vs `dc.b`).
- **T8** rept/irp/irpc/while completeness + `deform_table_sine` fold — resolves D2
  (bit-match vs source-cure). (high-latitude)
- **T9** switch/lowstring + `!name` escape (`!error` ×4, `!align 2`) + `__DEBUG__`
  debugger `%<…>` bytes. (high-latitude)
- **T10** progressive real-source integration — assemble growing 68k subsets, byte-exact
  per section vs asl. **Exit gate for M1.C.**

---

## 5. Out of scope (deferred)

- **Full-image `sha256` ROM match + stub-table deletion** → M1.D (A+C interlock).
- **`MOMPASS`/`MOMLINE` gating, `charset`, `!org`/`!dc.b` zero-offset defeat** — zero
  occurrences in Aeon source (§7.3), phantom scope.
- **Human-readable `.lst` fidelity** beyond the symbol/address columns already handled by
  M1.B `emit_listing` (§7.3).
- **Modern comptime rewrite** of `deform_table_sine` (Spec 2).
- **Any new debugger capability** — reproduce today's bytes only (§7.3).

---

## 6. Contamination Safeguard (standing)

CI dependency-graph assertion (§7.4, §9.1): `sigil-frontend-as` must not appear in the
dependency graph of `sigil-ir`, `sigil-backend-*`, or `sigil-link`. Deleting the crate for
Spec 5 must remain a no-op for everything else. No IR node / backend / linker construct may
encode an AS-specific concept (operator quirks, the `strstr` bug, the `!` escape,
size-suffix text substitution). All lowered away inside this crate.
