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

**Decision (Spike 0 resolved this — bit-match confirmed, source-cure NOT needed):**
1. Extract the 4 golden 256-byte tables from the reference ROM **unconditionally** (Spike
   0, done — committed at `crates/sigil-frontend-as/tests/vectors/sine_goldens/`).
2. Implement the real fold in Rust `f64` with `int()` = **floor (round toward −∞)** and
   libm `sin`. **Spike 0 finding (verified 2026-07-03):** `floor` reproduces all 4 golden
   tables byte-for-byte; `trunc`/`round_half`/`round_even` each fail (trunc diverges on
   ~123/256 indices in the largest-amplitude table). AS `int()` is therefore floor here,
   **not** truncate-toward-zero as originally assumed.
3. libm `sin` + `floor` **bit-matches** the reference ROM with no observed FP/libm
   discrepancy, so the D2 source-cure fallback (§12 R7 pre-bake to `BINCLUDE` + re-baseline)
   is **not indicated**. It remains the documented escape hatch only if a future
   re-baseline surfaces a mismatch.

Note: `cos()` has **zero real occurrences** in the source (Spike 0: all 3 raw hits are
comment prose). `sin` and `int` are used at exactly **one** site — the `deform_table_sine`
macro (`parallax_macros.inc:223`) — so the float-builtin work folds into **T8**; there is
no separate mainline builtin task.

### D3 — AS-faithful operator + builtin layer is a foundation task, lands first

The current Pratt parser (`expr.rs`) supports only `* / + - << >> & | = != < > <= >=` and
has **zero builtins**. **Spike 0 (verified 2026-07-03) collapsed this scope** by separating
raw grep hits from genuine operator uses (stripping comments and message strings):

- **Mainline operators actually used:** `<>` (×70, not-equal), `||` (×5, logical-or),
  `&&` (×4, logical-and) — plus comparisons that must **yield `0` / `-1` masks** (§7.1).
- **Zero real occurrences** (all comment prose or literal text inside `error`/`fatal`
  message strings): `mod` (63 raw), `!=` (33 raw), `~` (76 raw), `~~` (0), `cos` (3 raw),
  `even` (62 raw). These are **dropped from scope** — reproducing them would be phantom work.
- **Debug-only operator:** `!`=bitwise-or (×3, all in `engine/debug/debugger.asm`) →
  handled in **T9**, not the mainline T2.
- **Builtins:** the entire string-builtin surface (`substr` ×32, `strlen` ×18, `strstr`
  ×11, `lowstring` ×5, `val` ×5, `switch` ×5) is **confined to `debugger.asm`** → **T9**
  (debug-only). The mainline needs only `sin`/`int` at the single sine site → **T8**.

**Decision:** T2 lands the mainline operator set (`<>`, `||`, `&&`, comparison masks) +
generalized `\{}` interpolation + string comparison **before** 68k instruction lowering
(macros like `deform_table_sine` need `<>`/string-compare/`\{}` to expand). The former "T3
builtin table" task is **dissolved**: mainline float builtins move to T8, all string
builtins move to T9.

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

### D7 — 68k line classification uses AS's column rule (found during T1)

`exec_one`'s bare-label-without-colon detection historically keyed on the Z80-only
`is_mnemonic` table. Under `cpu 68000` there is no mnemonic table until T4, so a leading
identifier cannot be classified as label-vs-instruction that way. AS's actual rule is
**column-based**: a bare label (no colon) sits at **column 0**; an instruction is
**indented**. T1 implemented this for the M68000 path (`indented = head.span.start >
line.base`; a stripped colon label forces instruction classification). **T4 note:** when the
68k mnemonic table lands, classification may consult it *in addition to* the column rule,
but the column rule remains the AS-faithful ground truth and must not regress (the T1 tests
`m68k_operandless_instruction_reaches_stub_not_swallowed_as_label` and
`m68k_colon_label_then_instruction_both_handled` guard it).

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
| Expr operators (mainline) | subset | `<>`/`\|\|`/`&&` + comparison masks (Spike 0: `mod`/`!=`/`~~`/`~` unused) | low |
| Builtins (mainline) | none | `sin`/`int` only, at the single sine site (→ T8) | low |
| Builtins (debug) | none | strstr(bug)/substr/strlen/lowstring/val/switch, `!`=or (→ T9) | med |
| `\{expr}` interp | error/fatal only | generalize to all string contexts | low |
| CPU→backend dispatch | Z80 hardcoded | split `lower_z80`/`lower_m68k` | high |
| Mnemonic table | 26 Z80 | 68k table + `.b/.w/.l/.s` suffix parse | med |
| Register words | Z80 only | d0–d7/a0–a7/pc/sp/ccr/sr | low |
| Operand convert | Z80 | `convert_atoms_m68k` + EA modes | med |
| Instr lowering | lower/rel/abs16 | branch/jmp-jsr-sym/pcrel-ea + qualify | high |
| Data directives | `db`/`dc.b`/`dw` | `dc.w`/`dc.l`/`ds.*`/`align` (arbitrary, incl `$8000`)/`org`+padding (Spike 0: `even` unused) | low |
| struct/`_len` | capture only | `_len` symbol + `if _len<>N/error` | low |
| Macro args | positional | keyword args + ALLARGS/.ATTRIBUTE/MOMCPUNAME | med |
| rept/irp/irpc/while | rept only | irp/irpc/while + sine fold | med |
| Debugger surface | none | switch/lowstring + `!name` escape | med |

---

## 4. Decomposition (ordered subagent tasks)

Each task: spec-slice → TDD plan → fresh implementer → review. asl-diff / golden-vector
gates throughout (the M1.B pattern). Lighter controller-verify on transcription tasks
(T2/T6); full two-stage review on high-latitude tasks (T1/T5/T8/T9); whole-branch review
before `--no-ff` merge to master.

> **Re-scoped by Spike 0 (2026-07-03, DONE at `6a3bfc3`):** `int()` = floor (bit-matches
> via libm `sin`); mainline operators reduce to `<>`/`||`/`&&`; the former T3 builtin task
> is dissolved (mainline float builtins → T8, all string builtins → T9); T6 drops `even`,
> adds `org` + arbitrary-boundary `align`.

- **Spike 0** ✅ DONE (`6a3bfc3`) — 4 sine goldens extracted; `int()`=floor confirmed
  (libm `sin`+`floor` bit-matches); full operator/builtin/directive surface enumerated.
  Findings: `docs/superpowers/notes/2026-07-03-m1c-spike0-findings.md`.
- **T1** Backend multiplexing refactor — extract `lower_z80`, add `m68k` field, dispatch on
  `state.cpu`. Pure refactor; existing Z80 tests stay green. (high-latitude)
- **T2** Mainline operators (`<>`, `||`, `&&`, comparison→`0`/`-1` masks) + generalized
  `\{expr}` interpolation + `"a"="b"` string comparison. asl-diff-gated on expression
  snippets. (transcription)
- ~~**T3**~~ **Dissolved by Spike 0** — mainline float builtins (`sin`/`int`) fold into T8;
  all string builtins (`strstr`/`substr`/`strlen`/`lowstring`/`val`/`switch`) are
  debug-only → T9.
- **T4** 68k mnemonics + `.b/.w/.l/.s` size-suffix parse + operand-atom extensions
  (`-(An)`, `(An)+`, `(d16,PC)`, `abs.w`/`abs.l`).
- **T5** `convert_atoms_m68k` + lower routing (branch / jmp-jsr-sym / pcrel-ea) +
  **dotted-local qualification** (D6). (high-latitude)
- **T6** `dc.w`/`dc.l`/`ds.b`/`ds.w`/`ds.l`/`align` (arbitrary boundary, incl `align $8000`)
  /`org` (4 sites) + padding-state interaction (Aeon `padding off` global → odd `dc.b` runs
  unpadded). Spike 0: `even` has 0 real uses, out of scope. (transcription)
- **T7** struct/`_len` + `if _len<>N/error` assertion; keyword macro args; ALLARGS /
  `MOMCPUNAME` `pbyte` dispatch (`db` vs `dc.b`) — the non-debug dual-CPU idiom in the 8
  `*_patches.asm` sound files. (`.ATTRIBUTE` is debug-only → T9.)
- **T8** rept/irp/irpc/while completeness + `deform_table_sine` fold **incl. the `sin`/`int`
  builtins** (libm `sin` + `floor`, gated on the 4 Spike-0 goldens). (high-latitude)
- **T9** (debug-only, `__DEBUG__`) string builtins `strstr` (bug-for-bug D5)/`substr`/
  `strlen`/`lowstring`/`val` + `switch`/`lowstring` `%<…>` machinery + `!`=bitwise-or
  operator + `.ATTRIBUTE` + `!name` escape (`!error` ×4, `!align` ×10). (high-latitude)
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
