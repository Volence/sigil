---
kind: audit-stage
system: emp-language
stage: "0 Inventory"
agent: Opus
status: done
description: "190 hand files / 86k lines, 60% comments; contracts on 596 of 605 procs; 18 shipped constructs unused, 24 shipped constructs absent from every spec; game logic is 14% of code. Tests 2,782 passed / 2 env-refused; one disclosed write into sigil/target."
---
# EMP language: inventory

> [!note] Result (mechanical, no judgment)
> - **Trees measured:** aeon `936439fa` (2026-09-28), sigil `7630ba0d` (2026-09-28). Both have untracked files only, no tracked changes.
> - **Corpus:** 204 `.emp` files. 190 are hand-written (86,080 lines). 14 are under `generated/` (1,029 lines) and counted separately. 6 `.asm` files remain: two `game_root.asm`, the vendored debugger, and three editor exports. All counts MEASURED by the script in the appendix.
> - **Comments outweigh code:** 48,841 comment-only lines against 33,178 code lines, a **60% comment share** (MEASURED). There are 1,033 comment lines carrying a date (MEASURED), 367 AS operand echoes `// (Name).w` (MEASURED, 129 of them in `parallax.emp`) and 198 comment references to `.asm` files (MEASURED).
> - **Feature use is concentrated.** The register contracts are used nearly everywhere: `clobbers` on 596 of 605 procs, `preserves` on 176, `out` on 75 (MEASURED). `ensure` appears 1,486 times, and 352 of those pin a name to a literal (MEASURED). **18 shipped constructs have zero aeon uses** (§1e), among them `dispatch`, `script`, `bitfield`, runtime `enum`, `here()`, the compression builtins, `comptime test`, `todo!`, `@cycles_exact`, `@as_compat` and the game prelude. `table` has 1 use, `@budget` 1, `fixed<>` 4.
> - **Built beyond the frozen spec.** 24 shipped constructs are absent from every `SIGIL_*.md` file and are specified only in sigil design notes: `requires`/`grants`, `interface`/`implement`/`invoke`/`hook`, `type`, `inout`, five attributes, `assert`/`raise_*`, `mul_const`/`mul_bounded`, `cycles()`/`pad_to_cycles()` and conditional/flag `out`. Specified but not built: `?` on `Result` (D2.12), `jbcc`, charmap, `patch` as an item surface, dimensional types, SMC slots, VBlank-queue types, and the LSP (Spec 3).
> - **Code by kind** (code lines, rule in §5): cycle-critical procs 14.0%, tight loops elsewhere 1.9%, engine-systems procs 24.1%, engine object-system procs 6.9%, **game and object logic procs 14.1%**, comptime fn bodies 13.6%, declarations and layout 14.2%, data and tables 2.5%, item-position guards 5.7%, imports 2.6%. 3,758 of the 4,654 cycle-critical lines are the Z80 sound driver.
> - **Frontend tests** (release, Ryzen 7 9800X3D, 16 threads, 47.1 s wall clock including the build): 138 binaries, **2,782 passed, 2 failed, 0 ignored**. Both failures are the reference-tree refusal (no `AEON_DIR` named). Re-run with `AEON_DIR` set: that binary passed 12/12. MEASURED.
> - **Process breach, disclosed:** my first test launch put `CARGO_TARGET_DIR` on the wrong command, so cargo wrote about 185 build-artifact files into `sigil/target/release/` (`deps/`, `.fingerprint/`, `build/`) before I stopped it. `target/release/sigil` and `emit_sound_blob` are untouched (mtime 2026-11-13). Nothing was deleted, to avoid a second write. Details in §7.

## 0. Provenance and commands

| What | Command (run from `/home/volence/sonic_hacks`) |
|---|---|
| Revisions | `git -C sigil log -1 --format='%h %cd'` → `7630ba0d`; `git -C aeon log -1` → `936439fa`; `git status --short` (untracked only) |
| Corpus size | `find aeon/engine aeon/games -name '*.emp' \| wc -l` → 204; `-path '*generated*'` → 14; `xargs cat \| wc -l` → 86,080 hand / 1,029 generated |
| All per-file, idiom and feature counts | `python3 -I inventory.py /home/volence/sonic_hacks/aeon OUT` (script in the appendix; outputs `perfile.tsv`, `features.tsv`, `summary.json`, `hits/*.txt` under the scratchpad `emp_audit/out/`) |
| Proc-header contract coverage | `python3 -I procs.py /home/volence/sonic_hacks/aeon` (appendix) |
| Parser keyword sites | `grep -n -E '"[a-z_]+"' crates/sigil-frontend-emp/src/parser.rs`, `grep -n 'fn ' parser.rs` |
| Spec coverage of a construct | `grep -c -F '<kw>' empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`, `grep -rlF '<kw>' sigil/docs/superpowers/specs` |
| Editor grammar diff | python over `sigil/editors/vscode/syntaxes/emp.tmLanguage.json` (§6) |
| Tests | `CARGO_TARGET_DIR=<scratch>/audit-builds/emp-inventory cargo test --release -p sigil-frontend-emp --offline --no-fail-fast`, then the `cfg_blind_spots` target again with `AEON_DIR=/home/volence/sonic_hacks/aeon` |

Line counting: a line is **blank**, **comment-only** (nothing outside `//`, `///` or `/* */`), or **code**. Strings are stripped before any syntax regex runs, so text inside `ensure` messages never counts as a feature use.

## 1. Feature inventory

Status classes: **SHIPPED**: parsed, lowered or checked, with tests in `crates/sigil-frontend-emp/tests/`. **PARTIAL**: part of the specified behaviour is built. **SPEC-NOT-BUILT**: specified (or ratified and deferred) with no implementation. **ABSENT**: neither specified nor built, or decided against. "Uses" counts lines in the 190 hand-written files (with files in brackets), measured by the `features.tsv` regex for that row. Spec2 = `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`; `specs/` = `sigil/docs/superpowers/specs/`. `parser.rs` lines are the parse site in `crates/sigil-frontend-emp/src/`.

### 1a. Items and declarations

| Feature | Status | Spec | Frontend | aeon uses |
|---|---|---|---|---|
| `module` / `use` / `pub` | SHIPPED | Spec2 §3.1–3.2, §3.5 | `parser.rs:486`, `:780`, `:545`; `resolve/imports.rs` | 190 / 552 (135) / 2,178 (135) |
| Game prelude (zero-import files) | SHIPPED in sigil (`--prelude`, `sigil-cli/src/main.rs:1602`) | Spec2 §3.4, D2.22f | `--prelude` flag | **Not used:** `aeon/build.sh` contains no `prelude`; 552 `use` lines |
| `section` blocks (`cpu:`, `vma:`, `bank:`) | SHIPPED | Spec2 §7.1, §7.4, D2.25 | `parser.rs:3906` | 21 (19); `cpu: z80` 22; `vma:` 7; `bank:` 5 |
| `access: byte` | SHIPPED | D2.31, §7.1 | section attrs | **0** |
| `const` | SHIPPED | Spec2 §4 | `parser.rs:973` | 2,084 (133) |
| `equ` link equates | SHIPPED | Spec2 §7.5, D2.26 | `parser.rs:988` | 197 (21) |
| `newtype` | SHIPPED | D2.9 | `parser.rs:1045` | 20 (1: `engine/system/types.emp`) |
| `fixed<I,F>` scaled type | SHIPPED | D2.10 | `parser.rs:931` | 4 (2) |
| `T where LO..HI` refinement | SHIPPED | D2.11 | `parser.rs:886` | 16 (6) |
| `struct` (+ defaults, `(size: N)`) | SHIPPED | Spec2 §4.3, D2.31 | `parser.rs:1221`; `layout.rs` | 64 (20); `(size:)` 13 (7) |
| struct `pad` / `pad_to` | SHIPPED | §4.3.1, D2.38 | `parser.rs:1207` | 32 (4) |
| struct field `(align: N)` | SHIPPED | D2.37 | `parser.rs:1144` | 15 (7) |
| `bitfield` | SHIPPED | Spec2 §4.4 | `parser.rs:1063` | **0** |
| `enum` (runtime) | SHIPPED | Spec2 §4.4 | `parser.rs:1005` | **0** |
| `comptime enum` (sum types) + `match` | SHIPPED | D2.14 | `parser.rs:620`, `:4563`; `eval/pattern.rs` | 13 (3) + 65 (4) |
| `vars` (+ SST overlays) | SHIPPED | Spec2 §4.6, D2.21 | `parser.rs:1580` | 14 (12) |
| `region` RAM allocation | SHIPPED | Spec2 §4.6; `specs/2026-08-01-item7-ram-regions-design.md` | `parser.rs:1786`; `lower/regions.rs` | 8 (5) |
| `data` (+ typed views, `max_size:`) | SHIPPED | Spec2 §4.5, D2.20, D2.33 | `parser.rs:1828` | 223 (45); `max_size:` **0** |
| `align N` item | SHIPPED | Spec2 §4.8, D2.29 | `parser.rs:603` | 11 (9) |
| `table` (counted/sentinel/sparse) | SHIPPED | Spec2 §4.9, D2.36 | `parser.rs:1322` | **1** (`sfx_bank.emp:107`) |
| `offsets` | SHIPPED | Spec2 §4.7, D2.15 | `parser.rs:1279` | 17 (11) |
| `dispatch` | SHIPPED | Spec2 §5.5, D2.21 | `parser.rs:1499` | **0** |
| `script` (`yield`/`loop`/`wait_frames`/`shows`) | SHIPPED | Spec2 §5.6, D2.24, D2.30 | `parser.rs:2550-2660`; `lower/script.rs` | **0** |
| `proc` | SHIPPED | Spec2 §5.1 | `parser.rs:2064`; `lower/proc.rs` | 607 lines / 605 headers (88 files) |
| `extern proc` / `extern const` | SHIPPED | none in `SIGIL_*.md` | `parser.rs:2335`, `:2353` | 0 / 2 (1) |
| `type Name = proc (...) contract` | SHIPPED | none in `SIGIL_*.md`; `specs/2026-07-17-contract-grammar-v2-design.md` | `parser.rs:2368` | 10 (9) |
| `interface` / `hook` / `implement` / `invoke` | SHIPPED (one implementation per interface, bound at link) | none in `SIGIL_*.md`; `specs/2026-08-02-l1-game-contract-design.md` | `parser.rs:2395`, `:2420`, `:2467`, `:2827`; `resolve/contract.rs` | 1 / 12 / 2 / 8 |
| `context` + `with` brackets (params, named slots) | SHIPPED | Spec2 §3.6 (d-33) | `parser.rs:2182`, `:2790`, `:3160`; `context.rs` | 4 (2) contexts; 31 (10) `with` |
| Generics (type parameters on fns or types) | ABSENT | none (comptime values are dynamically typed; `fixed<>`, `rescale<>` are the only parameterized forms) | `comptime_fn_decl` `parser.rs:3687` takes no type params | n/a |
| Macros (AS-style text macros) | ABSENT by design | D2.6: one mechanism, comptime fn + `asm{}` | n/a | n/a |
| Traits (polymorphic dispatch over many impls) | ABSENT | `interface` is a one-binding link contract, not a trait | `resolve/contract.rs:1` | n/a |

### 1b. Proc contracts and attributes

| Feature | Status | Spec | Frontend | aeon uses |
|---|---|---|---|---|
| `clobbers(...)`, checked transitively | SHIPPED (S2-D6a, beyond the spec text's "optional lint") | Spec2 §5.1; contract-grammar v2 §1 | `parser.rs:2085`; `closure.rs:514`; `calls.rs:474` | 611 (85); **596 of 605** proc headers |
| `preserves(...)`, verified by symbolic stack | SHIPPED (beyond the D2.32 syntactic slice) | D2.32; contract-grammar v2 §5 | `parser.rs:2117`; `preserves.rs` | 183 (45); 176 headers |
| `out(...)` (+ `rN: T` types) | SHIPPED | D2.35; `specs/2026-08-07-out-type-width-design.md` | `parser.rs:1990`, `:2091`; `out_verify.rs:256` | 76 (27); 75 headers |
| `out(rN if cc)` conditional / `out(carry: name)` | SHIPPED (Spec2 says deferred to S2-D7) | contract-grammar v2 §6 | `parser.rs:2015`, `:2039`; `out_verify.rs:1445`; `flag_check.rs:1243` | 3 (2) / 18 (9) |
| `inout(...)` | SHIPPED | none in `SIGIL_*.md` | `parser.rs:2103`; `out_verify.rs:1268` | 6 (4) |
| `requires(...)` / `grants(...)` | SHIPPED | `specs/2026-08-03-contract-unification-spec.md` | `parser.rs:2081-2084`, `:2139`; `context.rs:327` | 10 (7) / 1 (1) |
| `falls_into` | SHIPPED | Spec2 §5.1 | `parser.rs:2122` | 42 (21) |
| Stack-delta verification (S2-D7b) | SHIPPED | Spec2 S2-D7(b) (ledger says deferred) | `preserves.rs:1399` | implicit, every proc |
| CCR flag liveness (S2-D7a) | PARTIAL: caller-side flag-result must-use and the result-invalid path only | S2-D7(a); contract-grammar v2 §6 | `flag_check.rs:1243-1626` | implicit |
| `@budget(cycles: N)` / `@cycles_exact` | SHIPPED | S2-D7(c) (ledger says deferred) | `parser.rs:301-350`; `cycle_budget.rs:430`; `m68k_cycles.rs`, `z80_cycles.rs` | **1** (`dma_queue.emp:358`) / **0** |
| `@noreturn` | SHIPPED | `specs/2026-08-05-noreturn-tail-design.md` | `parser.rs:358` | 21 (6) |
| `@resumable` / `@continuation` | SHIPPED | no design doc under `specs/`; `resumable.rs:1` header | `parser.rs:369`, `:381`; `resumable.rs` | 1 / 2 |
| `@scaffolding` | SHIPPED | contract-grammar v2 | `parser.rs:388` | 5 (2) |
| `@shape_divergent` | SHIPPED | item7 ram-regions design | `parser.rs:1745` | 11 (2) |
| `@as_compat` | SHIPPED | Spec2 §8.2 | `parser.rs:439` | **0** |
| `@allow` / `@align` | SHIPPED | Spec2 §10 | `parser.rs:395`, `:1645` | 1 / 1 |
| `export` labels | SHIPPED | Spec2 §5.2 | `parser.rs:2707` | 8 (4) |
| Z80 bus-state contract | SHIPPED | §D backlog | `z80_bus.rs:214` | implicit |
| Domain-newtype slot check at call sites | SHIPPED | G5 §7 tier 5 | `type_slice.rs:274` | implicit |

### 1c. Comptime

| Feature | Status | Spec | Frontend | aeon uses |
|---|---|---|---|---|
| `comptime fn` | SHIPPED | Spec2 §6.1, D2.6 | `parser.rs:3687`; `eval/call.rs` | 384 (45) + 13 generated |
| `asm {}` templates + `{}` splices | SHIPPED | Spec2 §6.2 | `parser.rs:4551`; `eval/asm.rs` | 103 (27) |
| `comptime var` / `for` / `block` / `if` / `while` | SHIPPED | Spec2 §6.3 | `parser.rs:3804-3823` | 215 (14) / 21 (9) / 0 / 0 / 0 (plain `if` inside fns is counted with the 583 indented `if` lines) |
| `let` | SHIPPED | Spec2 §6 | `parser.rs:2801`, `:3785` | 214 (15) |
| `\|>` pipe / lambdas | SHIPPED | D2.12 | `parser.rs:4010`, `:4339` | 7 (4) / 15 (7) |
| `?` on `Result` | **SPEC-NOT-BUILT** | D2.12 | `Tok::Question` is used only as the enum sentinel (`parser.rs:1056`); no postfix `?` | n/a |
| `++` on `Data`/`Code` | SHIPPED | Spec2 §10 | `parser.rs:4100` | 54 (13) |
| `ensure` / `ensure_fatal` (item + statement position) | SHIPPED | Spec2 §6.5, D2.20, D2.23 | `parser.rs:636`; `eval/guards.rs` | 1,486 (133); 1,185 at item position |
| `cycles(.a, .b)` / `pad_to_cycles()` | SHIPPED | `cycles(` is mentioned only in the ledger (`SIGIL_SPEC2_LANGUAGE.md:1294`) | `eval/call.rs:87`; `eval/asm.rs:308` | 28 (3) / 2 (1) |
| `sizeof` / `offsetof` / `span` | SHIPPED | Spec2 §6.8 | `parser.rs:4623`, `:4633`; `eval/builtins.rs` | 178 (26) / 368 (26) / 11 (2) |
| `here()` (link-time when provisional) | SHIPPED | D2.23 | builtin | **0** |
| `embed` / `import` | SHIPPED | Spec2 §6.7 | `eval/sandbox.rs` | 107 (24) + 20 generated |
| Compression builtins (`zx0`, `s4lz`, 9 classic formats) | SHIPPED | D2.28 | `eval/s4lz.rs`, `eval/classic_compress.rs` | **0** (aeon compresses outside the language) |
| `winptr` / `bankid` | SHIPPED | Spec2 §7.4 | builtins | 39 (3) |
| `extern("sym")` | SHIPPED | D2.27 | `eval/call.rs` | 372 (47) + 62 generated |
| `as.*` / `math.*` floats | SHIPPED | Spec2 §6.6 | `eval/float_ns.rs` | 2 (1) |
| `rescale<I,F>` | SHIPPED | D2.10 | `parser.rs:4646` | **0** |
| `comptime test` | SHIPPED | S2-D11a, D2.31 | `parser.rs:3651` | **0** (`ensure` on fn calls stands in: 95 `f(...) == literal` guards, §4) |
| `todo!` / `unreachable!` | SHIPPED | S2-D11e | `parser.rs:2728` | **0** |
| `///` doc comments | SHIPPED | D2.19, S2-D11d | `parser.rs:179` | **0** lines start with `///` |
| `#Tbl[i]` comptime index immediate | SHIPPED | D2.33 | `parser.rs:3596` | **0** |
| `patch` / `bind` | PARTIAL: comptime-statement surface plus `lower/patch.rs`; §6.4 marked DEMOTED; no item surface | Spec2 §6.4, S2-D17 | `parser.rs:3843`, `:3852` | 0 |
| Charmap text layer | SPEC-NOT-BUILT | D2.16, S2-D16(a) | none (`eval/expr.rs:1126` refuses it) | n/a |
| Dimensional types | SPEC-NOT-BUILT (deferred) | S2-D8 | none | n/a |

### 1d. Instruction lines

| Feature | Status | Spec | Frontend | aeon uses |
|---|---|---|---|---|
| Classic 68k/Z80 instruction lines | SHIPPED | tenet 3 | `parser.rs:3231` | 33,178 code lines in total |
| Unsized branches (relaxed by reach) | SHIPPED | D2.18, Spec2 §5.4 | `lower/` relax ladder | 1,751 unsized `bcc`/`bra`/`bsr` lines vs 46 sized (sized hits listed in §4) |
| `jbra` / `jbsr` | SHIPPED | D2.18 | mnemonic position | 404 (59) / 575 (57); raw `jsr`/`jmp` 20 (11) |
| `jbcc` conditional trampolines | SPEC-NOT-BUILT (deferred) | S2-D13(a) | none (`grep jbcc src` empty) | n/a |
| `mul_const` / `mul_bounded`: the **compiler picks the instruction sequence** | SHIPPED | none in `SIGIL_*.md`; `specs/2026-08-03-mul-lowering-design.md` | `mul_lower.rs:109`, `:124` | 32 (9); raw `mulu`/`divu` 10 (4) |
| `assert.<w>` / `raise_error` / `raise_exception` | SHIPPED | `specs/2026-07-11-emp-diagnostics-construct-design.md` | `parser.rs:2887`, `:2971`; `eval/diag.rs` | 66 (21) / 35 (9) / 14 (2) |
| Comptime `if` gates in instruction bodies | SHIPPED | Spec2 §6.2 | `parser.rs:3105` | included in the 583 indented `if` lines |
| Structured control flow (`if`/`while` lowering to generated branches) | ABSENT, decided against | S2-D15 | none | n/a |
| Register allocation / expression compilation / coroutine lowering (`.emp2` tier) | ABSENT (ideas only) | `SIGIL_IDEAS_FUNCTIONAL_TIER.md` ("not a plan") | none | n/a |
| Hot-swappable IRQ handlers + typed SMC slots | SPEC-NOT-BUILT (deferred) | S2-D9 | none | n/a |
| Phase split + bounded VBlank queue type; multi-kind collections | SPEC-NOT-BUILT (deferred) | S2-D10 | none (`table` covers flat ROM collections only) | n/a |
| Terseness sugar | SPEC-NOT-BUILT (post-release) | S2-D4 | none | n/a |
| Formal EBNF | ABSENT | S2-D1 ("remains with the implementation") | none | n/a |
| LSP, inlay hints, diagnostic rendering | SPEC-NOT-BUILT | Spec 3 (Spec2 §1) | no LSP crate under `sigil/crates/`; CLI `--report ram\|contracts\|indirect-cost` (`main.rs:2645`) | n/a |

### 1e. Status tally

| Class | Count (rows above) |
|---|---|
| SHIPPED, used by aeon | every other SHIPPED row |
| SHIPPED, **zero aeon uses** | 18: the game prelude, `access: byte`, `bitfield`, runtime `enum`, `data (max_size:)`, `dispatch`, `script`, `extern proc`, `@cycles_exact`, `@as_compat`, `comptime test`, `todo!`/`unreachable!`, `///`, `here()`, compression builtins, `rescale`, `#Tbl[i]`, `comptime if/while/block` |
| PARTIAL | 2: `patch`/`bind`, CCR flag liveness |
| SPEC-NOT-BUILT | 8: `?` on `Result`, charmap, dimensional types, `jbcc`, IRQ/SMC slots (S2-D9), engine-architecture constructs (S2-D10), terseness sugar (S2-D4), LSP (Spec 3) |
| ABSENT | 6: generics, text macros, traits, structured control flow, the `.emp2` compiler tier, formal EBNF |
| Shipped but missing from every `SIGIL_*.md` | 24: `requires`, `grants`, `interface`, `implement`, `invoke`, `hook`, `type`, `inout`, `extern proc`, `extern const`, conditional `out`, flag `out`, `@noreturn`, `@resumable`, `@continuation`, `@scaffolding`, `@shape_divergent`, `assert.<w>`, `raise_error`, `raise_exception`, `mul_const`, `mul_bounded`, `cycles()`, `pad_to_cycles()` (`@budget`/`@cycles_exact` appear only in the ledger as deferred) |

## 2. Doc-claim list (for section 7)

| # | Claim | Source | Mechanical check |
|---|---|---|---|
| C1 | "Status: FROZEN for v1, 2026-07-09 … §10's concept inventory is CLOSED" | Spec2 header | D2.36–D2.38 were added after the freeze (Spec2 `:188-190`), and 24 shipped constructs (§1e) are not in §10 or anywhere in Spec2 |
| C2 | "The entire concept inventory fits on one page (§10)" | tenet 1 | §10 lists 9 attributes; the parser's closed attribute set is 8 `@` names plus 7 clause words (`requires`, `grants`, `clobbers`, `out`, `inout`, `preserves`, `falls_into`), and 6 of the `@` names are not in §10 (`parser.rs:307`) |
| C3 | "The instructions don't change … cycle-for-cycle. Only the shell modernizes" | tenet 3 | Exceptions that exist: `mul_const`/`mul_bounded` choose the instruction sequence (32 uses); reach-relaxed branches choose widths (1,751 + 979 sites); `asm{}` templates synthesize instruction lines (103 templates) |
| C4 | "A new object file starts at zero imports (game prelude)" | tenet 2, §3.4 | aeon does not use a prelude (no `prelude` in `aeon/build.sh`); 552 `use` lines in 135 files |
| C5 | "Explicit-by-default … invariants as checked asserts" | tenet 5 | 1,486 `ensure`; 66 `assert.<w>` |
| C6 | "Totality: illegal states don't compile" (exhaustive `match`, refinements, stack delta) | tenet 7 | `match` 65 uses in 4 files; refinements 16; stack delta checked (`preserves.rs:1399`) |
| C7 | `?` early return on `Result` is part of the comptime glue | D2.12 | not parsed (§1c) |
| C8 | Cycle budgets, stack delta and CCR liveness are deferred (S2-D7) | Spec2 ledger | stack delta and budgets are shipped; CCR is partial (§1b). The ledger is stale |
| C9 | Conditional `out(a1) if z` "rides S2-D7 (deferred)" | D2.35 | shipped (`parser.rs:2015`), 3 uses |
| C10 | No structured control-flow sugar in v1 | S2-D15 | none in the parser; holds |
| C11 | "`patch` … ZERO consumers, no item surface" | S2-D17 | holds (0 uses); a comptime-statement parse exists (`parser.rs:3843`) |
| C12 | "Transfers are auto-sized: explicit `.s`/`.w` only in `@as_compat` ports and patched fields" | `aeon/CLAUDE.md`, CODING_CONVENTIONS §1.4 | 46 sized branches with 0 `@as_compat` files: `game_debug.emp` 16, `collision.emp` 13, `animate.emp` 10, `raster.emp` 4, `dma_queue.emp` 2, `aabb.emp` 1 (templates). Whether each is a patched field is a reviewer question |
| C13 | "`struct (size: N)` for ALL data structures, no manual const/equ chains" | `aeon/CLAUDE.md` | 64 structs, 13 with `(size:)`; 2,084 `const` lines (not classified further) |
| C14 | "No `mulu`/`divu` casually … four-point argument written at the instruction" | `aeon/CLAUDE.md`, CONVENTIONS §2.1 | 10 raw `mul`/`div` lines in 4 files (`hits/feat_mulu_muls_divu_divs.txt`); argument presence not checked |
| C15 | "Default: no comments … explain WHY" | CODING_CONVENTIONS §5.4 | 60% comment share; 1,033 dated lines; 367 AS echoes |
| C16 | "never compute at runtime what sigil can compute at build time" | `aeon/CLAUDE.md`, CONVENTIONS §7.1 | 13 static-table loop candidates; 32 literal-plus-derivation-comment lines (§4) |
| C17 | The VS Code grammar is "hand-maintained against the parser keyword list" | `sigil/editors/vscode/README.md` | 38 parser words are missing from the grammar (§6) |
| C18 | `interface` binds against "its one" implementation | `resolve/contract.rs:1` | 1 interface (`Game`), 2 implementations (one per game) |
| C19 | EMP_PITFALLS: 14 measured traps (nested `if` yields unit; imports do not travel; guards in unreachable modules are dead; register tokens in comptime; `extern()` poisons comptime; `assert.<w>` traps; `adda`/`suba`; per-import struct re-elaboration; `Game.*` missing in layout contexts; always-red equality guard; signature lengths; unchecked `Struct.field(aN)`) | `aeon/docs/EMP_PITFALLS.md` §1–14, `sigil/docs/EMP_PITFALLS_EQUALITY.md` | listed for reviewers; §11 is marked "mechanism refuted"; §12/§13 are mirrored in the sigil hand-off |

## 3. Proc contract coverage (`procs.py`, comments stripped, header up to `{`)

| Metric | Count (MEASURED) |
|---|---|
| Proc headers | 605 |
| with `clobbers` | 596 |
| with `preserves` | 176 |
| with `out` | 75 |
| with `inout` | 6 |
| with typed register params (`aN: *T`) | 248 |
| with `requires` / `grants` | 10 / 1 |
| with `falls_into` | 42 |
| with no register contract at all | 7 |

## 4. Idiom and port-residue counts (template §7)

Totals for the 190 hand-written files, with `generated/` separate. Every number is MEASURED by `inventory.py`, and the `hits/` file named holds the `file:line` of every hit.

| Check | Rule (as scripted) | Hand | Generated | Hit list |
|---|---|---|---|---|
| Comment share | comment-only ÷ (comment-only + code) lines | **60%** (48,841 / 82,019) | 55% (523 / 952) | `perfile.tsv` |
| Trailing comments | code lines that also carry a comment | 8,056 of 33,178 | 40 | `perfile.tsv` |
| Dated history | comment containing `20YY-MM-DD` | **1,033** | 14 | `hits/dated.txt` |
| History words (broad) | comment matching formerly / previously / no longer / retired / SINCE / WAS … | 1,770 | 6 | (count only) |
| AS operand echo | comment containing `(Name).w` or `(Name).l` | **367** | 0 | `hits/as_echo.txt` |
| Widths-pinned notes | `widths pinned` / `width-PINNED` in a comment | 20 | 0 | `hits/widths_pinned.txt` |
| References to `.asm` files | `*.asm` in a comment | 198 | 0 | `hits/asm_ref.txt` |
| Raw `n(aN)` offsets | numeric displacement on a0–a6 (symbol±n excluded) | 50 | 0 | `hits/numoff_any.txt` |
| …on a register the proc types as `*Struct` | same, where the header binds `aN: *T` | **0** | 0 | `hits/numoff_typed.txt` (empty) |
| Plane/stride literals | `#64/128/256/512/1K/2K/4K/8K` on add/sub/lea/cmp/move/and/mul (not `.b`) | 14 | 0 | `hits/stride.txt` |
| Stride shifts | `lsl/asl/lsr/asr #6` or `#7` | 11 | 0 | `hits/stride_shift.txt` |
| Literal with derivation in a comment | `#literal` with arithmetic in the trailing comment | 32 | 0 | `hits/literal_deriv.txt` |
| `ensure` total | every `ensure(`/`ensure_fatal(` outside comments | 1,486 | 15 | `hits/feat_ensure_ensure_fatal.txt` |
| …pin: NAME == literal | LHS a bare name / sizeof / offsetof / span / extern pair, RHS a literal | **352** | 1 | `hits/ensure_pin_name_eq_literal.txt` |
| …pin: expression == literal | other LHS expression == literal | 102 | 1 | `hits/ensure_pin_expr_eq_literal.txt` |
| …self-test: `f(...) == literal` | LHS a comptime fn call | 95 | 0 | `hits/ensure_selftest_call_eq_literal.txt` |
| …equality between derived values | `==` with no literal side | 506 | 1 | `hits/ensure_eq_derived.txt` |
| …bound (`<`, `<=`, `>`, `>=`) | | 372 | 10 | `hits/ensure_bound.txt` |
| …`cycles()` | | 22 | 0 | `hits/ensure_cycles.txt` |
| …other | | 37 | 2 | `hits/ensure_other.txt` |
| Run-time loops over static tables | proc that `lea`s a `data`/`table`/`offsets` item into aN, contains `dbf`/`dbra`, and reads `(aN)+` | 13 candidates | 0 | `hits/static_table_loop.txt` |
| Sized branches | `b<cc>.s/.w` in proc, fn or template bodies | 46 | 0 | `hits/branch_sized.txt` |

The six-way `ensure` split sums to 1,501, which is the 1,486 hand-written plus the 15 generated guards. The four `==` rows ("pin: name", "pin: expression", "self-test" and "derived") together hold 1,071 hand-written guards, about 72% of the total. The class rules are regex-based; reviewers decide which pins are derivable.

**Where the residue is** (top files, MEASURED):

| Check | Top files |
|---|---|
| AS echo (367) | `parallax.emp` 129, `ojz_scroll_test.emp` 74, `bg.emp` 41, `dma_queue.emp` 22, `camera.emp` 15, `bg_anim.emp` 14, `object_test_state.emp` 13, `section.emp` 12, `buffers.emp` 9 |
| Dated comments (1,033) | `parallax.emp` 113, `ojz_effects.emp` 65, `scene_dsl.emp` 59, `ram.emp` 55, `ojz_scroll_test.emp` 47 |
| Widths-pinned (20) | `vblank.emp` 142, 304–322 (9); `section.emp` 899, 903, 904, 1273; `tile_cache.emp` 1699, 1742, 1919; `page_cache.emp` 1519, 1539; `dma_queue.emp` 59, 407 |
| Raw `n(aN)` (50) | `raster.emp` 15, `entity_window.emp` 14 (`COLLECTED_BITMASK_OFFSET+4(a0)` style excluded; plain `1(a0)` counted), `parallax.emp` 4, others ≤3 |
| Stride shifts (11) | `bg.emp` 985/987/991, `plane_buffer.emp` 198/385/663/669, `section.emp` 431, `sprites.emp` 292, `sound_api.emp` 93, `player_spindash.emp` 167. Example: `bg.emp:985` `lsl.l #7, d1 // m * PLANE_H_CELLS*2` (the constant exists, and the derivation sits in the comment) |
| Ensure name==literal pins (352) | `scene_equiv_proof.emp` 99, `scene_registry.emp` 32, `scene_dsl.emp` 31, `sound_constants.emp` 21, `raster_dsl.emp` 19, `games/sonic4/config/constants.emp` 14, `games/demo/config/constants.emp` 10 |
| Static-table loops (13) | `compression_selftest.emp:107`, `:184`; `preset.emp:459`; `raster.emp:2304`; `bg_anim.emp:270`; `tile_cache.emp:274`; `demo_state.emp:48`; `ojz_effects.emp:2882`, `:2883`; `object_test_state.emp:99`, `:106`, `:244`; `ojz_scroll_test.emp:858` (most are init-time copies, not sums; 2 have accumulating adds) |

Not counted, because no mechanical rule was found: **hand-kept protocols** (flags or orderings whose rules live only in comments). Reviewers own that row.

## 5. Code by kind

**Rule.** Each code line (non-blank, non-comment) is classed by the top-level item that encloses it. Brackets `([{` are tracked with strings and comments stripped, and `section {}` bodies count as top level.

- **Proc lines** are split by proc:
  - **cycle-critical**: the file path matches `vblank|hblank|irq|raster|dma_queue|z80_sound_driver|sound/sound_(fm|psg|sequencer|sfx|dac)`, or the proc name matches `VBlank|HBlank|HInt|VInt|Raster_|Irq`, or the proc carries `@budget`/`@cycles_exact`;
  - **tight loop**: the lines from a label to a `db<cc>` that branches back to it within ≤8 code lines, in procs that are not already cycle-critical;
  - **game and object logic**: any other proc under `games/`;
  - **engine object system**: any other proc under `engine/objects/`;
  - **engine systems**: everything else.
- **Non-proc lines** are classed by item kind:
  - `comptime fn`/`test` → comptime;
  - `data`/`table`/`offsets` → data;
  - `const`/`equ`/`struct`/`bitfield`/`enum`/`vars`/`region`/`newtype` → declarations and layout;
  - item-position `ensure` → guards;
  - `use`/`module` → imports;
  - `context`/`interface`/`implement`/`type` → contract declarations.

| Kind | Hand lines | Share | Generated |
|---|---|---|---|
| Proc: cycle-critical | 4,654 | 14.0% | 0 |
| Proc: tight loops (outside cycle-critical files) | 614 | 1.9% | 0 |
| Proc: engine systems | 8,008 | 24.1% | 0 |
| Proc: engine object system | 2,305 | 6.9% | 0 |
| Proc: game and object logic (`games/`) | 4,684 | 14.1% | 0 |
| Comptime fn bodies (incl. `asm{}` templates) | 4,499 | 13.6% | 64 |
| Declarations and layout | 4,710 | 14.2% | 226 |
| Data and tables | 814 | 2.5% | 97 |
| Item-position guards | 1,879 | 5.7% | 12 |
| Imports | 857 | 2.6% | 29 |
| Contract declarations | 68 | 0.2% | 0 |
| Other top level | 86 | 0.3% | 1 |
| **Total** | **33,178** | 100% | 429 |

Proc counts by class: cycle-critical 173, engine systems 188, engine object system 82, game 161 (604 parsed procs). Cycle-critical files: `raster.emp`, `plane_buffer.emp` (VInt-named procs), `sound_fm`/`psg`/`sequencer`/`sfx.emp`, `z80_sound_driver.emp`, `dma_queue.emp`, `hblank.emp`, `vblank.emp`.

| Directory | Proc lines by class (MEASURED) |
|---|---|
| `engine/sound` | cycle-critical 3,758 (Z80), engine 478 |
| `engine/system` | cycle-critical 440, engine 768, tight 55 |
| `engine/effects` | cycle-critical 404, engine 451, tight 39 |
| `engine/level` | engine 5,755, tight 277, cycle-critical 52 |
| `engine/objects` | object system 2,305, tight 164 |
| `engine/compression`, `engine/debug` | engine 556, tight 53 |
| `games/sonic4/player` | game 2,650, tight 3 |
| `games/sonic4/test` (incl. the shipped tick `ojz_scroll_test.emp`) | game 1,152, tight 15 |
| `games/sonic4/objects` | game 692 |
| `games/sonic4/data`, `debug`; `games/demo` | game 190, tight 8 |

> [!note] Classification limits
> Per-frame streaming code in `engine/level` (tile and page caches, section streaming) is classed as engine systems, not cycle-critical: the path rule does not name it. The game-logic bucket includes the game-state test harness (`games/sonic4/test`, 1,152 lines). Without it, Sonic 4 player plus object logic is 3,342 proc lines, **10.1%** of code.

## 6. Editor support

- `sigil/editors/vscode`: a TextMate grammar only, version 0.1.0, unpublished. It has no LSP, hover, go-to-definition or diagnostics. No LSP crate exists under `sigil/crates/`.
- Grammar vs parser (MEASURED). These parser words are absent from `emp.tmLanguage.json`: `requires grants preserves inout context with interface implement invoke hook table region assert raise_error raise_exception extern noreturn resumable continuation budget cycles_exact scaffolding shape_divergent cycles pad_to_cycles mul_const mul_bounded granted released_by_rte acquire release embed import winptr bankid span pad pad_to` (38).

## 7. Test run and the process breach

| Run | Result |
|---|---|
| `cargo test --release -p sigil-frontend-emp --offline --no-fail-fast`, target `<scratch>/audit-builds/emp-inventory` | 138 test binaries: **2,782 passed, 2 failed, 0 ignored, 0 filtered**. Wall clock 47.1 s including a cold release build (user 306.8 s), AMD Ryzen 7 9800X3D, 16 threads. MEASURED |
| The 2 failures | `cfg_blind_spots`: `corpus_computed_dispatch_census_is_six_sites_five_procs`, `no_out_declaring_proc_carries_a_targets_dispatch`. Both panic at `sigil-harness/src/test_support.rs:1463`: "NO REFERENCE TREE IS NAMED". This is an environment refusal, not a code failure |
| `cfg_blind_spots` re-run with `AEON_DIR=/home/volence/sonic_hacks/aeon` | **12 passed, 0 failed**. MEASURED |
| Not checked | Whether other corpus-dependent tests passed in the first run by skipping their reference rows (DID NOT CHECK) |

> [!finding] Breach of the read-only rule (mine)
> My first launch was `CARGO_TARGET_DIR=<scratch> nproc && cargo test ...`. The variable bound only to `nproc`, so cargo used `sigil/target/`. I stopped it during compilation. **About 185 files under `sigil/target/release/` were created or updated:**
> - `deps/`: 65, new test binaries and dev-dependency rlibs;
> - `.fingerprint/`: about 100 under `sigil-frontend-emp-*`, `tempfile`, `rustix`, `libc`, `getrandom` and others;
> - `build/`: a handful.
>
> `target/release/sigil` and `target/release/emit_sound_blob`, which aeon links against, are unchanged (mtime 2026-11-13 14:35). No source, docs or git state was touched. I deleted nothing, to avoid a second write into the repo. `cargo clean -p sigil-frontend-emp --release` or leaving it are both safe; that is the controller's call. The repeat run used the scratch target dir.

## 8. Per-file table (190 hand-written files)

> [!note]- Per-file counts (click to expand). Columns as in §4
> | File | Lines | Code | Comment share | Dated | AS echo | Width-pin notes | `.asm` refs | Raw n(aN) | Stride lit/shift | Lit+derivation comment | Ensure pins (name==lit) | Static-table loops | Sized branches |
> |---|---|---|---|---|---|---|---|---|---|---|---|---|---|
> | `engine/level/parallax.emp` | 5032 | 1841 | 62% | 113 | 129 | 0 | 5 | 4 | 2/0 | 0 | 9 | 0 | 0 |
> | `games/sonic4/test/ojz_scroll_test.emp` | 4570 | 1343 | 69% | 47 | 74 | 0 | 0 | 3 | 0/0 | 4 | 1 | 1 | 0 |
> | `engine/effects/raster_dsl.emp` | 3861 | 1274 | 66% | 41 | 0 | 0 | 0 | 0 | 0/0 | 0 | 19 | 0 | 0 |
> | `engine/level/scene_dsl.emp` | 3664 | 1205 | 66% | 59 | 0 | 0 | 1 | 0 | 0/0 | 0 | 31 | 0 | 0 |
> | `games/sonic4/data/effects/ojz_effects.emp` | 3072 | 593 | 80% | 65 | 2 | 0 | 4 | 0 | 0/0 | 0 | 6 | 2 | 0 |
> | `games/sonic4/player/player_common.emp` | 2625 | 1032 | 59% | 4 | 5 | 0 | 21 | 0 | 2/0 | 2 | 5 | 0 | 0 |
> | `engine/level/tile_cache.emp` | 2466 | 1473 | 37% | 12 | 0 | 3 | 0 | 0 | 0/0 | 4 | 1 | 1 | 0 |
> | `engine/effects/raster.emp` | 2426 | 589 | 75% | 39 | 1 | 0 | 5 | 15 | 0/0 | 1 | 1 | 1 | 4 |
> | `engine/level/page_cache.emp` | 2285 | 1209 | 45% | 35 | 0 | 2 | 0 | 3 | 0/0 | 0 | 3 | 0 | 0 |
> | `engine/sound/sound_sequencer.emp` | 2184 | 1278 | 39% | 8 | 0 | 0 | 0 | 0 | 0/0 | 0 | 2 | 0 | 0 |
> | `engine/ram.emp` | 2004 | 510 | 73% | 55 | 0 | 0 | 2 | 0 | 0/0 | 0 | 3 | 0 | 0 |
> | `engine/sound/sound_sfx.emp` | 1915 | 955 | 48% | 7 | 0 | 0 | 1 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> | `engine/objects/entity_window.emp` | 1867 | 1032 | 40% | 11 | 0 | 0 | 0 | 14 | 0/0 | 2 | 2 | 0 | 0 |
> | `engine/sound/z80_sound_driver.emp` | 1827 | 840 | 52% | 13 | 0 | 0 | 0 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> | `engine/level/section.emp` | 1533 | 767 | 47% | 21 | 12 | 4 | 0 | 0 | 2/1 | 0 | 1 | 0 | 0 |
> | `engine/system/constants.emp` | 1369 | 427 | 67% | 37 | 0 | 0 | 3 | 0 | 0/0 | 0 | 2 | 0 | 0 |
> | `engine/sound/sound_fm.emp` | 1275 | 579 | 53% | 9 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/levels/ojz/act1/act_descriptor.emp` | 1196 | 384 | 67% | 42 | 0 | 0 | 0 | 0 | 0/0 | 0 | 8 | 0 | 0 |
> | `games/sonic4/player/player_ground.emp` | 1185 | 583 | 49% | 12 | 0 | 0 | 4 | 0 | 2/0 | 2 | 7 | 0 | 0 |
> | `engine/sound/sound_constants.emp` | 1118 | 674 | 34% | 12 | 0 | 0 | 0 | 0 | 0/0 | 0 | 21 | 0 | 0 |
> | `games/sonic4/objects/test_solid.emp` | 1117 | 332 | 68% | 10 | 0 | 0 | 26 | 0 | 0/0 | 0 | 3 | 0 | 0 |
> | `engine/level/bg.emp` | 1113 | 326 | 70% | 33 | 41 | 0 | 5 | 0 | 0/3 | 0 | 1 | 0 | 0 |
> | `games/sonic4/data/effects/ojz_scenes.emp` | 1061 | 275 | 74% | 23 | 0 | 0 | 2 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/objects/sprites.emp` | 945 | 479 | 45% | 4 | 0 | 0 | 0 | 2 | 0/1 | 1 | 0 | 0 | 0 |
> | `engine/effects/palette.emp` | 919 | 487 | 45% | 10 | 0 | 0 | 0 | 2 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/level/plane_buffer.emp` | 849 | 418 | 48% | 4 | 0 | 0 | 3 | 0 | 1/4 | 1 | 1 | 0 | 0 |
> | `games/sonic4/data/effects/scene_registry.emp` | 840 | 244 | 70% | 18 | 0 | 0 | 0 | 0 | 0/0 | 0 | 32 | 0 | 0 |
> | `engine/objects/core.emp` | 803 | 376 | 51% | 4 | 0 | 0 | 2 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/level/page_in.emp` | 798 | 359 | 53% | 3 | 0 | 0 | 0 | 0 | 0/0 | 0 | 2 | 0 | 0 |
> | `games/sonic4/player/player_air.emp` | 788 | 360 | 52% | 1 | 2 | 0 | 4 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/scene_equiv_proof.emp` | 770 | 409 | 44% | 8 | 0 | 0 | 0 | 0 | 0/0 | 0 | 99 | 0 | 0 |
> | `games/sonic4/player/player_glide.emp` | 754 | 308 | 57% | 5 | 2 | 0 | 11 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/sound/sound_psg.emp` | 728 | 267 | 62% | 5 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/objects/children.emp` | 710 | 298 | 53% | 4 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/sound/sound_api.emp` | 679 | 284 | 56% | 13 | 4 | 0 | 1 | 0 | 0/1 | 0 | 0 | 0 | 0 |
> | `games/sonic4/player/player_sensors.emp` | 659 | 365 | 42% | 3 | 0 | 0 | 10 | 0 | 0/0 | 1 | 0 | 0 | 0 |
> | `engine/system/dma_queue.emp` | 657 | 328 | 47% | 3 | 22 | 2 | 3 | 0 | 0/0 | 1 | 2 | 0 | 2 |
> | `games/sonic4/player/player_instashield.emp` | 647 | 161 | 74% | 6 | 2 | 0 | 6 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/structs.emp` | 642 | 155 | 75% | 23 | 0 | 0 | 0 | 0 | 0/0 | 0 | 3 | 0 | 0 |
> | `games/sonic4/player/player_climb.emp` | 634 | 301 | 50% | 1 | 1 | 0 | 6 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/objects/collision.emp` | 617 | 247 | 56% | 2 | 0 | 0 | 4 | 0 | 0/0 | 0 | 0 | 0 | 13 |
> | `engine/system/vblank.emp` | 553 | 169 | 68% | 11 | 4 | 9 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/level/bg_anim.emp` | 540 | 171 | 67% | 6 | 14 | 0 | 1 | 0 | 0/0 | 1 | 2 | 1 | 0 |
> | `games/sonic4/config/constants.emp` | 535 | 173 | 66% | 6 | 0 | 0 | 6 | 0 | 0/0 | 0 | 14 | 0 | 0 |
> | `engine/level/parallax_dsl.emp` | 526 | 161 | 68% | 6 | 0 | 0 | 2 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> | `engine/system/buffers.emp` | 510 | 217 | 56% | 9 | 9 | 0 | 2 | 3 | 0/0 | 1 | 2 | 0 | 0 |
> | `games/sonic4/config/ram.emp` | 489 | 69 | 85% | 3 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/effects/preset.emp` | 468 | 117 | 74% | 8 | 0 | 0 | 0 | 0 | 0/0 | 0 | 2 | 1 | 0 |
> | `games/sonic4/player/player_fly.emp` | 443 | 136 | 68% | 1 | 1 | 0 | 12 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/level/camera.emp` | 442 | 199 | 52% | 0 | 15 | 0 | 0 | 0 | 0/0 | 3 | 0 | 0 | 0 |
> | `engine/objects/dplc.emp` | 418 | 108 | 73% | 2 | 0 | 0 | 0 | 0 | 0/0 | 2 | 1 | 0 | 0 |
> | `games/sonic4/test/object_test_state.emp` | 405 | 220 | 39% | 2 | 13 | 0 | 2 | 0 | 0/0 | 0 | 0 | 3 | 0 |
> | `engine/objects/rings.emp` | 403 | 195 | 46% | 1 | 0 | 0 | 0 | 0 | 0/0 | 1 | 0 | 0 | 0 |
> | `games/sonic4/objects/tails_appendage.emp` | 396 | 107 | 71% | 2 | 0 | 0 | 3 | 0 | 0/0 | 1 | 0 | 0 | 0 |
> | `engine/system/boot.emp` | 387 | 132 | 62% | 6 | 0 | 0 | 0 | 2 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/debug/compression_selftest.emp` | 385 | 179 | 50% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 2 | 0 |
> | `engine/system/replay.emp` | 378 | 204 | 44% | 5 | 0 | 0 | 0 | 0 | 0/0 | 0 | 6 | 0 | 0 |
> | `engine/compression/s4lz.emp` | 369 | 167 | 52% | 4 | 0 | 0 | 0 | 0 | 0/0 | 1 | 0 | 0 | 0 |
> | `engine/objects/animate.emp` | 346 | 165 | 48% | 0 | 0 | 0 | 0 | 2 | 0/0 | 0 | 0 | 0 | 10 |
> | `games/sonic4/objects/test_player.emp` | 346 | 228 | 28% | 0 | 1 | 0 | 1 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/debug/error_handler.emp` | 340 | 225 | 31% | 2 | 1 | 0 | 3 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/animations/tails_anims.emp` | 283 | 118 | 54% | 0 | 0 | 0 | 4 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/characters/knuckles_data.emp` | 271 | 61 | 76% | 8 | 0 | 0 | 0 | 0 | 0/0 | 0 | 2 | 0 | 0 |
> | `games/sonic4/objects/dust_spindash.emp` | 269 | 104 | 58% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/vdp.emp` | 258 | 113 | 53% | 0 | 0 | 0 | 1 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> | `engine/level/load_art.emp` | 240 | 78 | 65% | 3 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/system/boot_data.emp` | 237 | 79 | 64% | 5 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_vbounds_range.emp` | 227 | 135 | 36% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 5 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_lcm_factor0.emp` | 225 | 126 | 42% | 3 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/objects/ring_sparkle.emp` | 223 | 67 | 67% | 3 | 0 | 0 | 4 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> | `games/sonic4/data/sound/dac_samples.emp` | 221 | 59 | 71% | 6 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/sound/mt_bank.emp` | 217 | 36 | 82% | 2 | 0 | 0 | 9 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/objects/test_parent.emp` | 214 | 108 | 43% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/compression/zx0_resume.emp` | 200 | 81 | 58% | 1 | 1 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/collision/collision_data.emp` | 200 | 37 | 80% | 8 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/sound/sound_tables_z80.emp` | 197 | 140 | 27% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 4 | 0 | 0 |
> | `games/sonic4/player/player_spindash.emp` | 195 | 93 | 50% | 0 | 0 | 0 | 0 | 0 | 0/1 | 2 | 0 | 0 | 0 |
> | `engine/sound/dac_sample_tab.emp` | 194 | 88 | 53% | 3 | 0 | 0 | 2 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/debug/game_debug.emp` | 193 | 87 | 53% | 0 | 1 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 16 |
> | `games/sonic4/data/animations/knuckles_anims.emp` | 191 | 68 | 61% | 0 | 0 | 0 | 1 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/objects/sst.emp` | 190 | 84 | 53% | 3 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/z80_bus.emp` | 185 | 11 | 94% | 8 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/config/game.emp` | 183 | 17 | 90% | 9 | 0 | 0 | 0 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> | `engine/system/math.emp` | 180 | 60 | 65% | 3 | 0 | 0 | 1 | 0 | 5/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/sound/sfx/sfx_bank.emp` | 177 | 42 | 76% | 5 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/system/game_contract.emp` | 174 | 13 | 92% | 1 | 1 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/characters/tails_data.emp` | 174 | 51 | 68% | 3 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/system/controllers.emp` | 173 | 96 | 43% | 2 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_budget_axis1.emp` | 167 | 58 | 64% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 2 | 0 | 0 |
> | `engine/debug/sound_debug.emp` | 165 | 50 | 68% | 4 | 5 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/player/characters.emp` | 161 | 43 | 72% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/config/sound_ids.emp` | 160 | 45 | 70% | 2 | 0 | 0 | 1 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/system/vectors.emp` | 153 | 73 | 52% | 2 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/system/z80_init.emp` | 142 | 34 | 75% | 5 | 0 | 0 | 1 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_vfactor_range.emp` | 141 | 63 | 52% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 4 | 0 | 0 |
> | `games/sonic4/player/knuckles.emp` | 139 | 40 | 70% | 0 | 0 | 0 | 3 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_bob_range.emp` | 132 | 43 | 65% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 4 | 0 | 0 |
> | `engine/effects/palette_dsl.emp` | 131 | 62 | 49% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 5 | 0 | 0 |
> | `engine/system/types.emp` | 131 | 21 | 82% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_layer_drift.emp` | 130 | 54 | 57% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 2 | 0 | 0 |
> | `games/sonic4/data/sound/soundbankhead.emp` | 126 | 31 | 74% | 5 | 0 | 0 | 0 | 0 | 0/0 | 0 | 3 | 0 | 0 |
> | `engine/objects/load_object.emp` | 123 | 60 | 47% | 0 | 0 | 0 | 0 | 0 | 0/0 | 1 | 1 | 0 | 0 |
> | `engine/system/vdp_init.emp` | 121 | 34 | 70% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/objects/mapping_dsl.emp` | 120 | 60 | 46% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/animations/sonic_anims.emp` | 117 | 61 | 45% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/fixtures/sigil_objroutine_probe.emp` | 117 | 10 | 91% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/player/tails.emp` | 116 | 35 | 68% | 0 | 0 | 0 | 1 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/sound/sfx_bank_blob.emp` | 114 | 41 | 61% | 2 | 0 | 0 | 1 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_own_caps.emp` | 112 | 45 | 58% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 2 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_curve_deform.emp` | 108 | 60 | 42% | 2 | 0 | 0 | 0 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_own_flat.emp` | 106 | 40 | 60% | 2 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_capacity.emp` | 105 | 25 | 75% | 2 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_mask.emp` | 104 | 42 | 57% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 2 | 0 | 0 |
> | `games/sonic4/objects/dust_puff.emp` | 100 | 28 | 70% | 1 | 0 | 0 | 1 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/player/sonic.emp` | 100 | 35 | 63% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_own_only_table.emp` | 99 | 38 | 60% | 2 | 0 | 0 | 0 | 0 | 0/0 | 0 | 2 | 0 | 0 |
> | `games/sonic4/objects/test_churn.emp` | 97 | 40 | 55% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/irq.emp` | 96 | 17 | 82% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/levels/ojz/act1/act_assets.emp` | 95 | 22 | 75% | 2 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_lcm_undeclared.emp` | 95 | 36 | 60% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_tail_if_unit_fold.emp` | 94 | 14 | 85% | 3 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_actspan.emp` | 93 | 36 | 59% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 2 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_own_placement.emp` | 93 | 38 | 56% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_scene_proof.emp` | 90 | 25 | 71% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> | `games/sonic4/data/dust_data.emp` | 88 | 32 | 60% | 2 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/demo/config/constants.emp` | 86 | 24 | 70% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 10 | 0 | 0 |
> | `engine/level/collision_lookup.emp` | 85 | 44 | 44% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/objects/objdef.emp` | 85 | 48 | 41% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/sound/seq_opcode_tab.emp` | 85 | 37 | 55% | 0 | 0 | 0 | 2 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/sound/sfx_blob_win_tab.emp` | 83 | 39 | 51% | 0 | 0 | 0 | 1 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/system/hblank.emp` | 82 | 22 | 71% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/system/release_fault.emp` | 79 | 16 | 79% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/demo/data/demo_data.emp` | 79 | 34 | 51% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/demo/demo_state.emp` | 73 | 34 | 46% | 0 | 4 | 0 | 0 | 0 | 0/0 | 0 | 0 | 1 | 0 |
> | `engine/objects/frames.emp` | 72 | 24 | 65% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_four_bands.emp` | 72 | 9 | 87% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/demo/config/game.emp` | 68 | 11 | 82% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/objects/aabb.emp` | 67 | 22 | 65% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 1 |
> | `games/sonic4/data/sound/dac_banks.emp` | 66 | 7 | 89% | 2 | 0 | 0 | 1 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/objects/test_enemy.emp` | 65 | 32 | 42% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/system/game_loop.emp` | 64 | 22 | 63% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/sound/song_bank2.emp` | 64 | 21 | 64% | 2 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_landing_late.emp` | 63 | 7 | 89% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/objects/test_emitter.emp` | 59 | 28 | 45% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/objects/test_stress_emitter.emp` | 58 | 28 | 46% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/sound/movingtrucks_pitchtable.emp` | 57 | 27 | 51% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/objects/test_particle.emp` | 57 | 26 | 47% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_dplc_tile_start.emp` | 57 | 9 | 83% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/objects/test_animated.emp` | 56 | 30 | 39% | 0 | 0 | 0 | 1 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_band_nested.emp` | 54 | 7 | 87% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_deep_four_words.emp` | 53 | 6 | 88% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_extern_equate.emp` | 52 | 3 | 94% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_extern_span.emp` | 52 | 3 | 94% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/config/header.emp` | 50 | 36 | 20% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_patchable_overlap.emp` | 49 | 6 | 87% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/coords.emp` | 48 | 14 | 70% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/demo/config/header.emp` | 46 | 36 | 12% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/objects/test_helpers.emp` | 42 | 25 | 36% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_cram_four_words.emp` | 40 | 6 | 84% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_instashield_frames.emp` | 40 | 7 | 81% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_landing_early.emp` | 40 | 6 | 84% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/replay_fixture.emp` | 40 | 3 | 92% | 2 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_patchable_partner.emp` | 39 | 5 | 86% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_band_buried_tint.emp` | 38 | 7 | 81% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_band_orphan_restore.emp` | 38 | 6 | 83% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_setreg_on_restore.emp` | 38 | 7 | 81% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_ship_plus_restore.emp` | 38 | 8 | 78% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_band_span_mismatch.emp` | 37 | 8 | 76% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_band_base_above.emp` | 36 | 7 | 79% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_direct_8a.emp` | 36 | 6 | 82% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/system/epilogue.emp` | 35 | 6 | 79% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_band_h1_region.emp` | 35 | 4 | 88% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_band_h2_sh.emp` | 35 | 4 | 88% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_band_no_owner.emp` | 35 | 7 | 79% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_extern_addr.emp` | 35 | 2 | 94% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_sec_bg_layout.emp` | 33 | 4 | 87% | 1 | 0 | 0 | 0 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> | `games/sonic4/test/poison/poison_vdp_base_residue.emp` | 33 | 4 | 87% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_patchable_band_fire.emp` | 32 | 5 | 83% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_ring_sparkle_frames.emp` | 31 | 7 | 75% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/objdefs/test_objects.emp` | 29 | 11 | 52% | 1 | 0 | 0 | 2 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_regset_8f.emp` | 27 | 6 | 76% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/animations/dust_anims.emp` | 24 | 10 | 52% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_direct_8f.emp` | 24 | 6 | 73% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/mappings/test_mappings.emp` | 22 | 7 | 65% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `engine/objects/objcodebase.emp` | 20 | 6 | 67% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/test/poison/poison_sentinel.emp` | 20 | 2 | 89% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/data/animations/particle_anims.emp` | 19 | 6 | 65% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/demo/objects/demo_box.emp` | 17 | 5 | 67% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/demo/config/ram.emp` | 16 | 6 | 50% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | `games/sonic4/objects/test_static.emp` | 15 | 5 | 62% | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 | 0 | 0 | 0 |
> | **TOTAL hand-written (190 files)** | 86080 | 33178 | 60% | 1033 | 367 | 20 | 198 | 50 | 14/11 | 32 | 352 | 13 | 46 |
> | **TOTAL generated/ (14 files)** | 1029 | 429 | 55% | 14 | 0 | 0 | 0 | 0 | 0/0 | 0 | 1 | 0 | 0 |
> 
> | Directory | Files | Lines | Code | Comment share | Dated | AS echo |
> |---|---|---|---|---|---|---|
> | `engine/compression` | 2 | 569 | 248 | 54% | 5 | 1 |
> | `engine/coords.emp` | 1 | 48 | 14 | 70% | 0 | 0 |
> | `engine/debug` | 3 | 890 | 454 | 46% | 7 | 6 |
> | `engine/effects` | 5 | 7805 | 2529 | 67% | 98 | 1 |
> | `engine/irq.emp` | 1 | 96 | 17 | 82% | 0 | 0 |
> | `engine/level` | 13 | 19573 | 8251 | 56% | 295 | 211 |
> | `engine/objects` | 15 | 6786 | 3204 | 49% | 32 | 0 |
> | `engine/ram.emp` | 1 | 2004 | 510 | 73% | 55 | 0 |
> | `engine/sound` | 10 | 10202 | 5142 | 47% | 70 | 4 |
> | `engine/structs.emp` | 1 | 642 | 155 | 75% | 23 | 0 |
> | `engine/system` | 18 | 5425 | 1953 | 62% | 91 | 36 |
> | `engine/vdp.emp` | 1 | 258 | 113 | 53% | 0 | 0 |
> | `engine/z80_bus.emp` | 1 | 185 | 11 | 94% | 8 | 0 |
> | `games/demo` | 1 | 73 | 34 | 46% | 0 | 4 |
> | `games/demo/config` | 4 | 216 | 77 | 61% | 1 | 0 |
> | `games/demo/data` | 1 | 79 | 34 | 51% | 0 | 0 |
> | `games/demo/objects` | 1 | 17 | 5 | 67% | 0 | 0 |
> | `games/sonic4/config` | 5 | 1417 | 340 | 75% | 20 | 0 |
> | `games/sonic4/data` | 25 | 8807 | 2283 | 73% | 196 | 2 |
> | `games/sonic4/debug` | 1 | 193 | 87 | 53% | 0 | 1 |
> | `games/sonic4/objects` | 15 | 3114 | 1188 | 59% | 17 | 1 |
> | `games/sonic4/player` | 13 | 8446 | 3492 | 57% | 33 | 13 |
> | `games/sonic4/test` | 52 | 9235 | 3037 | 65% | 82 | 87 |

## Appendix: scripts

> [!note]- `inventory.py` (the counting script, exactly as run)
> ```python
> #!/usr/bin/env python3
> """Mechanical inventory of the aeon .emp corpus (EMP language audit, stage 0).
> 
> Usage: python3 -I inventory.py /home/volence/sonic_hacks/aeon OUTDIR
> Writes OUTDIR/{perfile.tsv,features.tsv,hits/*.txt,summary.json}.
> Read-only over the aeon tree.
> """
> import json, os, re, sys
> from collections import defaultdict, Counter
> 
> ROOT, OUT = sys.argv[1], sys.argv[2]
> os.makedirs(os.path.join(OUT, 'hits'), exist_ok=True)
> 
> files = []
> for top in ('engine', 'games'):
>     for dp, dn, fn in os.walk(os.path.join(ROOT, top)):
>         for f in fn:
>             if f.endswith('.emp'):
>                 files.append(os.path.relpath(os.path.join(dp, f), ROOT))
> files.sort()
> 
> def split_line(line, st):
>     """Return (code, comment, st). st = dict(in_block, in_str). Strings -> "" in code."""
>     code, com = [], []
>     i, n = 0, len(line)
>     while i < n:
>         c = line[i]
>         if st['in_block']:
>             j = line.find('*/', i)
>             if j < 0:
>                 com.append(line[i:]); i = n
>             else:
>                 com.append(line[i:j]); i = j + 2; st['in_block'] = False
>             continue
>         if st['in_str']:
>             if c == '\\':
>                 i += 2; continue
>             if c == '"':
>                 st['in_str'] = False; code.append('"')
>             i += 1; continue
>         if line.startswith('//', i):
>             com.append(line[i:]); break
>         if line.startswith('/*', i):
>             st['in_block'] = True; i += 2; continue
>         if c == '"':
>             st['in_str'] = True; code.append('"'); i += 1; continue
>         if c == "'" and re.match(r"'(\\.|[^'\\])'", line[i:]):
>             m = re.match(r"'(\\.|[^'\\])'", line[i:]); code.append("'c'"); i += len(m.group(0)); continue
>         code.append(c); i += 1
>     return ''.join(code), ''.join(com), st
> 
> ITEM_RE = re.compile(r'^\s*(?:pub\s+)?(?:extern\s+)?(proc|script|dispatch|comptime\s+fn|comptime\s+test|comptime\s+enum|data|table|offsets|const|equ|struct|bitfield|enum|vars|region|newtype|context|interface|implement|type|use|module|section|align|ensure_fatal|ensure)\b')
> DATE_RE = re.compile(r'\b20\d\d-[01]\d-[0-3]\d\b')
> HIST_RE = re.compile(r'\b(formerly|previously|used to|no longer|was (?:removed|renamed|changed|deleted|retired)|retired|since 20\d\d|as of 20\d\d|(?:SINCE|WAS|FORMERLY|RETIRED)\b)', re.I)
> ASECHO_RE = re.compile(r'\(\s*[A-Za-z_][\w.]*\s*\)\.[wl]\b')
> PIN_RE = re.compile(r'widths?[ -]pinned|pinned widths?|width[- ]PINNED|widths? PINNED', re.I)
> ASM_REF_RE = re.compile(r'\b[\w/.-]+\.asm\b')
> NUMOFF_RE = re.compile(r'(?<![\w.)*+])(-?(?:\$[0-9A-Fa-f]+|\d+))\((a[0-6])\)')
> INSTR_RE = re.compile(r'^\s*(?:[.\w]+:\s*)?([a-z][a-z0-9]*)(?:\.([bwls]))?\b')
> BRANCH_MN = re.compile(r'^(b(?:ra|sr|hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le|hs|lo))$')
> PARAM_RE = re.compile(r'\b(a[0-6])\s*:\s*\*\s*([A-Z]\w*)')
> STRIDE_VALUES = {64, 128, 256, 512, 1024, 2048, 4096, 8192}
> CYC_PATH = re.compile(r'(vblank|hblank|irq|raster|dma_queue|z80_sound_driver|/sound/sound_(fm|psg|sequencer|sfx|dac))', re.I)
> CYC_NAME = re.compile(r'(VBlank|HBlank|HInt|VInt|Raster_|_IRQ|Irq)', re.I)
> 
> def num(s):
>     s = s.lstrip('-')
>     return int(s[1:], 16) if s.startswith('$') else int(s)
> 
> FEATURES = [
>     # (key, regex over code text (strings stripped), where) where: any/item0
>     ('module decl', r'^\s*module\b'),
>     ('use', r'^\s*(?:pub\s+)?use\b'),
>     ('pub (item visibility)', r'^\s*pub\s+'),
>     ('section block', r'^\s*section\s+\w+'),
>     ('section cpu: z80', r'\bcpu:\s*z80\b'),
>     ('section vma:', r'\bvma:\s*'),
>     ('section bank:', r'\bbank:\s*'),
>     ('access: byte', r'\baccess:\s*byte\b'),
>     ('const', r'^\s*(?:pub\s+)?const\b'),
>     ('equ (link equate)', r'^\s*(?:pub\s+)?equ\b'),
>     ('newtype', r'^\s*(?:pub\s+)?newtype\b'),
>     ('fixed<I,F>', r'\bfixed\s*<'),
>     ('where LO..HI refinement', r'\bwhere\s+[-\w$]+\s*\.\.'),
>     ('struct', r'^\s*(?:pub\s+)?struct\b'),
>     ('struct (size: N)', r'^\s*(?:pub\s+)?struct\s+\w+\s*\(\s*size:'),
>     ('struct pad/pad_to', r'^\s*pad(?:_to)?\b'),
>     ('struct field (align: N)', r'\(\s*align:\s*'),
>     ('bitfield', r'^\s*(?:pub\s+)?bitfield\b'),
>     ('enum', r'^\s*(?:pub\s+)?enum\b'),
>     ('comptime enum', r'\bcomptime\s+enum\b'),
>     ('match', r'\bmatch\b'),
>     ('vars', r'^\s*(?:pub\s+)?vars\b'),
>     ('region', r'^\s*(?:pub\s+)?region\b'),
>     ('data', r'^\s*(?:pub\s+)?data\b'),
>     ('data (max_size:)', r'\bmax_size:'),
>     ('align N (item)', r'^\s*align\s+[^=\s]'),
>     ('table', r'^\s*(?:pub\s+)?table\b'),
>     ('offsets', r'^\s*(?:pub\s+)?offsets\b'),
>     ('dispatch', r'^\s*(?:pub\s+)?dispatch\b'),
>     ('script', r'^\s*(?:pub\s+)?script\b'),
>     ('script yield', r'^\s*yield\b'),
>     ('script loop', r'^\s*loop\b'),
>     ('script wait_frames', r'^\s*wait_frames\b'),
>     ('proc', r'^\s*(?:pub\s+)?proc\b'),
>     ('extern proc', r'^\s*(?:pub\s+)?extern\s+proc\b'),
>     ('extern const/item', r'^\s*(?:pub\s+)?extern\s+(?!proc)\w'),
>     ('type (proc contract type)', r'^\s*(?:pub\s+)?type\s+\w+\s*='),
>     ('interface', r'^\s*(?:pub\s+)?interface\s+\w'),
>     ('interface hook member', r'^\s*hook\b'),
>     ('implement', r'^\s*(?:pub\s+)?implement\s+\w'),
>     ('invoke', r'^\s*invoke\s+\w'),
>     ('context decl', r'^\s*(?:pub\s+)?context\s+\w'),
>     ('with block', r'^\s*with\s+\w'),
>     ('requires(...)', r'\brequires\s*\('),
>     ('grants(...)', r'\bgrants\s*\('),
>     ('clobbers(...)', r'\bclobbers\s*\('),
>     ('preserves(...)', r'\bpreserves\s*\('),
>     ('out(...)', r'(?<![\w.])out\s*\('),
>     ('out(... if cc) conditional', r'(?<![\w.])out\s*\([^)]*\bif\s+\w+'),
>     ('out(carry: ...) flag result', r'(?<![\w.])out\s*\([^)]*\bcarry:'),
>     ('inout(...)', r'\binout\s*\('),
>     ('falls_into', r'\bfalls_into\b'),
>     ('@as_compat', r'@as_compat\b'),
>     ('@allow', r'@allow\b'),
>     ('@align', r'@align\b'),
>     ('@budget(cycles:)', r'@budget\b'),
>     ('@cycles_exact', r'@cycles_exact\b'),
>     ('@noreturn', r'@noreturn\b'),
>     ('@resumable', r'@resumable\b'),
>     ('@continuation', r'@continuation\b'),
>     ('@scaffolding', r'@scaffolding\b'),
>     ('@shape_divergent', r'@shape_divergent\b'),
>     ('export label', r'\bexport\s*\.'),
>     ('comptime fn', r'\bcomptime\s+fn\b'),
>     ('comptime test', r'\bcomptime\s+test\b'),
>     ('comptime block', r'\bcomptime\s+block\b'),
>     ('comptime var', r'\bcomptime\s+var\b'),
>     ('comptime if', r'\bcomptime\s+if\b'),
>     ('comptime for', r'\bcomptime\s+for\b'),
>     ('comptime while', r'\bcomptime\s+while\b'),
>     ('asm {} template', r'\basm\s*\{'),
>     ('|> pipe', r'\|>'),
>     ('lambda |x|', r'(?<![|\w])\|\s*\w+(?:\s*,\s*\w+)*\s*\|(?!\|)'),
>     ('++ concat', r'\+\+'),
>     ('let (comptime/asm)', r'^\s*let\b'),
>     ('ensure / ensure_fatal', r'\bensure(?:_fatal)?\s*\('),
>     ('cycles(.a,.b)', r'\bcycles\s*\('),
>     ('pad_to_cycles()', r'\bpad_to_cycles\s*\('),
>     ('assert.<w>', r'^\s*assert\b'),
>     ('raise_error', r'\braise_error\b'),
>     ('raise_exception', r'\braise_exception\b'),
>     ('todo!/unreachable!', r'\b(?:todo|unreachable)!'),
>     ('sizeof()', r'\bsizeof\s*\('),
>     ('offsetof()', r'\boffsetof\s*\('),
>     ('here()', r'\bhere\s*\(\s*\)'),
>     ('span()', r'\bspan\s*\('),
>     ('embed()/import()', r'\b(?:embed|import)\s*\('),
>     ('compression builtin', r'\b(?:zx0|s4lz|kosinski|kosinski_m|kosplus|kosplus_m|saxman|enigma|nemesis|comper|rocket)\s*\('),
>     ('winptr()/bankid()', r'\b(?:winptr|bankid)\s*\('),
>     ('extern("sym")', r'\bextern\s*\(\s*"'),
>     ('as.* / math.* float', r'\b(?:as|math)\.[a-z]\w*\s*\('),
>     ('rescale<>', r'\brescale\s*<'),
>     ('mul_const/mul_bounded', r'\bmul_(?:const|bounded)\b'),
>     ('jbra', r'^\s*(?:[.\w]+:\s*)?jbra\b'),
>     ('jbsr', r'^\s*(?:[.\w]+:\s*)?jbsr\b'),
>     ('jsr/jmp (raw)', r'^\s*(?:[.\w]+:\s*)?j(?:sr|mp)\b'),
>     ('dbf/dbra/dbcc', r'^\s*(?:[.\w]+:\s*)?db(?:f|ra|t|ne|eq|cc|cs|pl|mi|hi|ls|ge|lt|gt|le)\b'),
>     ('mulu/muls/divu/divs', r'^\s*(?:[.\w]+:\s*)?(?:mul|div)[us]\b'),
>     ('movem', r'^\s*(?:[.\w]+:\s*)?movem\b'),
>     ('#Tbl[i] index immediate', r'#\s*\w+\[[^\]]+\]'),
> ]
> FEAT_RX = [(k, re.compile(r)) for k, r in FEATURES]
> 
> perfile = {}
> feat_file = defaultdict(Counter)    # feature -> file -> count
> feat_hits = defaultdict(list)
> hits = defaultdict(list)            # check -> ["file:line: text"]
> kind_lines = defaultdict(Counter)   # file -> class -> lines
> proc_records = []
> 
> for f in files:
>     gen = '/generated/' in f
>     with open(os.path.join(ROOT, f), encoding='utf-8', errors='replace') as fh:
>         lines = fh.read().split('\n')
>     if lines and lines[-1] == '':
>         lines.pop()
>     st = {'in_block': False, 'in_str': False}
>     stats = Counter()
>     depth = 0
>     section_bases = []          # depths at which a section block body is the item level
>     item_kind = None; item_open_depth = None; item_name = None
>     item_start = None; item_attrs = ''
>     proc_regs = {}              # aN -> Type for the current proc
>     proc_lines = []
>     pending_attrs = ''
>     for ln, raw in enumerate(lines, 1):
>         code, com, st = split_line(raw, st)
>         cs = code.strip()
>         stats['lines'] += 1
>         blank = raw.strip() == ''
>         if blank:
>             stats['blank'] += 1
>         elif cs == '' or cs == '"' :
>             stats['comment_only'] += 1
>             if raw.lstrip().startswith('///'):
>                 stats['doc_comment'] += 1
>         else:
>             stats['code'] += 1
>             if com.strip():
>                 stats['code_with_trailing_comment'] += 1
>         # comment-derived checks
>         if com:
>             if DATE_RE.search(com):
>                 stats['dated_comment_lines'] += 1
>                 hits['dated'].append(f'{f}:{ln}')
>             if HIST_RE.search(com):
>                 stats['history_word_lines'] += 1
>             if ASECHO_RE.search(com):
>                 stats['as_echo'] += 1
>                 hits['as_echo'].append(f'{f}:{ln}: {raw.strip()[:140]}')
>             if PIN_RE.search(com):
>                 stats['widths_pinned_note'] += 1
>                 hits['widths_pinned'].append(f'{f}:{ln}: {raw.strip()[:140]}')
>             if ASM_REF_RE.search(com):
>                 stats['asm_file_ref'] += 1
>                 hits['asm_ref'].append(f'{f}:{ln}: {ASM_REF_RE.search(com).group(0)}')
>         # item tracking (top level or section-body level)
>         base = section_bases[-1] if section_bases else 0
>         if item_kind is None and depth == base and cs:
>             if cs.startswith('@'):
>                 pending_attrs += ' ' + cs
>             m = ITEM_RE.match(code)
>             if m:
>                 k = re.sub(r'\s+', ' ', m.group(1))
>                 if k == 'section' and '{' in code:
>                     section_bases.append(depth + sum(code.count(c) for c in '([{') - sum(code.count(c) for c in ')]}'))
>                 else:
>                     item_kind = k; item_open_depth = depth; item_start = ln
>                     item_attrs = pending_attrs + ' ' + code
>                     pending_attrs = ''
>                     nm = re.search(r'\b(?:proc|script|dispatch|fn|data|table|offsets|struct|vars|region)\s+(\w+)', code)
>                     item_name = nm.group(1) if nm else ''
>                     proc_regs = dict(PARAM_RE.findall(code)) if k == 'proc' else {}
>                     proc_lines = []
>                     # item-position ensure
>                     if k in ('ensure', 'ensure_fatal'):
>                         stats['ensure_item_position'] += 1
>         depth_before = depth
>         depth += sum(code.count(c) for c in '([{') - sum(code.count(c) for c in ')]}')
>         if section_bases and depth < section_bases[-1]:
>             section_bases.pop()
>         # classify the line
>         cls = 'other'
>         if item_kind is not None:
>             cls = item_kind
>             if item_kind == 'proc':
>                 proc_lines.append((ln, code))
>                 # typed-param params can span lines (header)
>                 if ln - item_start < 4:
>                     proc_regs.update(dict(PARAM_RE.findall(code)))
>         if not blank and cs and cs != '"':
>             kind_lines[f][cls] += 1
>         # feature counts
>         if cs:
>             for k, rx in FEAT_RX:
>                 c = len(rx.findall(code))
>                 if c:
>                     feat_file[k][f] += c
>                     if len(feat_hits[k]) < 4000:
>                         feat_hits[k].append(f'{f}:{ln}')
>         # instruction-level checks (inside proc bodies, or asm templates)
>         if item_kind in ('proc', 'comptime fn', 'script', 'dispatch', 'context', 'implement') and cs:
>             mi = INSTR_RE.match(code)
>             mn = mi.group(1) if mi else ''
>             sz = mi.group(2) if mi else None
>             if mn and BRANCH_MN.match(mn):
>                 key = 'branch_sized' if sz in ('s', 'w', 'b', 'l') else 'branch_unsized'
>                 stats[key] += 1
>                 if sz:
>                     hits['branch_sized'].append(f'{f}:{ln}: {cs[:100]}')
>             for m in NUMOFF_RE.finditer(code):
>                 reg = m.group(2)
>                 stats['numoff_any'] += 1
>                 hits['numoff_any'].append(f'{f}:{ln}: [{reg}{": *"+proc_regs[reg] if reg in proc_regs else ""}] {cs[:110]}')
>                 if item_kind == 'proc' and reg in proc_regs:
>                     stats['numoff_typed_reg'] += 1
>                     hits['numoff_typed'].append(f'{f}:{ln}: [{reg}: *{proc_regs[reg]}] {cs[:110]}')
>             # plane/stride literals
>             for m in re.finditer(r'#\s*(\$[0-9A-Fa-f]+|\d+)\b', code):
>                 try:
>                     v = num(m.group(1))
>                 except ValueError:
>                     continue
>                 if v in STRIDE_VALUES and sz != 'b' and mn in ('add', 'adda', 'addi', 'sub', 'suba', 'subi', 'lea', 'cmp', 'cmpi', 'cmpa', 'move', 'movea', 'and', 'andi', 'mulu', 'muls'):
>                     stats['stride_literal'] += 1
>                     hits['stride'].append(f'{f}:{ln}: {cs[:110]}')
>             if mn in ('lsl', 'asl', 'lsr', 'asr') and re.search(r'#\s*(6|7)\b', code):
>                 stats['stride_shift'] += 1
>                 hits['stride_shift'].append(f'{f}:{ln}: {cs[:110]}')
>             # literals whose derivation sits in a comment
>             if re.search(r'#\s*-?(\$[0-9A-Fa-f]+|\d+)\b', code) and com and re.search(r'\d+\s*[*×/]\s*\d+|\d+\s*x\s*\d+|<<|>>|\d+\s*[+-]\s*\d+\s*=', com):
>                 stats['literal_with_derivation_comment'] += 1
>                 hits['literal_deriv'].append(f'{f}:{ln}: {raw.strip()[:140]}')
>         # close item
>         if item_kind is not None and depth <= item_open_depth and not (depth_before == depth and '{' in code and code.rstrip().endswith('{')):
>             if item_kind == 'proc':
>                 proc_records.append((f, item_name, item_start, ln, item_attrs, proc_lines))
>             item_kind = None; item_open_depth = None
>     stats['generated'] = gen
>     perfile[f] = stats
> 
> # ---- ensure classification (multi-line aware: re-scan whole text) ----
> ens = Counter(); ens_hits = defaultdict(list)
> for f in files:
>     txt = open(os.path.join(ROOT, f), encoding='utf-8', errors='replace').read()
>     # strip // comments but keep strings for paren matching simplicity
>     for m in re.finditer(r'\bensure(?:_fatal)?\s*\(', txt):
>         ls = txt.rfind('\n', 0, m.start()) + 1
>         if '//' in txt[ls:m.start()] or txt[ls:m.start()].count('"') % 2:
>             continue
>         i = m.end(); d = 1; instr = False
>         while i < len(txt) and d:
>             c = txt[i]
>             if instr:
>                 if c == '\\': i += 1
>                 elif c == '"': instr = False
>             elif c == '"': instr = True
>             elif c == '(': d += 1
>             elif c == ')': d -= 1
>             i += 1
>         body = txt[m.end():i-1]
>         cond = re.split(r',\s*"', body, maxsplit=1)[0]
>         cond = re.sub(r'//[^\n]*', '', cond)
>         ln = txt.count('\n', 0, m.start()) + 1
>         one = ' '.join(cond.split())[:150]
>         if re.search(r'\bcycles\s*\(', cond):
>             c = 'cycles'
>         elif re.search(r'(==)\s*-?(\$[0-9A-Fa-f]+|%[01]+|\d+)\s*$', cond.strip()) or re.search(r'^\s*-?(\$[0-9A-Fa-f]+|\d+)\s*==', cond):
>             lhs = cond.strip().split('==')[0].strip()
>             if re.fullmatch(r'\(?\s*(?:[A-Za-z_][\w.]*|(?:sizeof|offsetof|span)\s*\([^()]*\)|extern\s*\(\s*"[^"]*"\s*\)(?:\s*-\s*extern\s*\(\s*"[^"]*"\s*\))?)\s*\)?', lhs):
>                 c = 'pin_name_eq_literal'
>             elif re.match(r'^[a-z_]\w*\s*\(', lhs) and not re.match(r'^(sizeof|offsetof|extern|span)\b', lhs):
>                 c = 'selftest_call_eq_literal'
>             else:
>                 c = 'pin_expr_eq_literal'
>         elif '==' in cond:
>             c = 'eq_derived'
>         elif re.search(r'[<>]=?', cond):
>             c = 'bound'
>         else:
>             c = 'other'
>         ens[c] += 1
>         ens_hits[c].append(f'{f}:{ln}: {one}')
>         perfile[f]['ensure_' + c] += 1
> 
> # ---- static-table loops: proc with dbf + lea <data/table item> + (aN)+ read ----
> static_items = set()
> for f in files:
>     for line in open(os.path.join(ROOT, f), encoding='utf-8', errors='replace'):
>         m = re.match(r'^\s*(?:pub\s+)?(?:data|table|offsets)\s+(\w+)', line)
>         if m:
>             static_items.add(m.group(1))
> loop_hits = []
> for (f, name, a, b, attrs, pl) in proc_records:
>     body = [c for _, c in pl]
>     leas = {}
>     for ln, c in pl:
>         m = re.search(r'\blea\s+(?:\()?(\w+)(?:\)(?:\.[wl])?)?(?:\(pc\))?\s*,\s*(a[0-6])', c)
>         if m and m.group(1) in static_items:
>             leas[m.group(2)] = (m.group(1), ln)
>     if not leas:
>         continue
>     has_dbf = any(re.match(r'^\s*(?:[.\w]+:\s*)?db(?:f|ra)\b', c) for c in body)
>     if not has_dbf:
>         continue
>     for reg, (item, lln) in leas.items():
>         reads = [ln for ln, c in pl if re.search(r'\(' + reg + r'\)\+', c)]
>         acc = [ln for ln, c in pl if re.search(r'\badd[aqi]?(?:\.[bwl])?\s+\(' + reg + r'\)\+', c)]
>         if reads:
>             loop_hits.append(f'{f}:{lln}: {name}: lea {item},{reg}; dbf loop; {len(reads)} ({reg})+ reads; accumulate-adds={len(acc)}')
> hits['static_table_loop'] = loop_hits
> for h in loop_hits:
>     perfile[h.split(':')[0]]['static_table_loop'] += 1
> 
> # ---- code classification (cycle-critical / engine / object / game / comptime / data) ----
> def proc_class(f, name, attrs, pl):
>     if CYC_PATH.search(f) or CYC_NAME.search(name or '') or re.search(r'@(budget|cycles_exact)', attrs):
>         return 'cycle_critical'
>     # tight inner loop: a dbf whose target label is <= 8 code lines above
>     labels = {}
>     for idx, (ln, c) in enumerate(pl):
>         m = re.match(r'^\s*(\.?\w+):', c)
>         if m: labels[m.group(1)] = idx
>         m2 = re.match(r'^\s*db(?:f|ra|\w\w)(?:\.w)?\s+d\d\s*,\s*(\.?\w+)', c)
>         if m2 and m2.group(1) in labels and 0 < idx - labels[m2.group(1)] <= 8:
>             pass
>     if f.startswith('games/'):
>         return 'game_logic'
>     if f.startswith('engine/objects/'):
>         return 'engine_objects'
>     return 'engine_other'
> 
> class_lines = Counter(); class_lines_gen = Counter()
> proc_class_count = Counter()
> proc_lines_by_line = defaultdict(dict)
> for (f, name, a, b, attrs, pl) in proc_records:
>     pc = proc_class(f, name, attrs, pl)
>     proc_class_count[pc] += 1
>     tight = set()
>     labels = {}
>     code_idx = [(ln, c) for ln, c in pl if c.strip() and c.strip() != '"']
>     for idx, (ln, c) in enumerate(code_idx):
>         m = re.match(r'^\s*(\.?\w+):', c)
>         if m: labels[m.group(1)] = idx
>         m2 = re.match(r'^\s*db(?:f|ra|\w\w)(?:\.w)?\s+d\d\s*,\s*(\.?\w+)', c)
>         if m2 and m2.group(1) in labels and 0 < idx - labels[m2.group(1)] <= 8:
>             for j in range(labels[m2.group(1)], idx + 1):
>                 tight.add(code_idx[j][0])
>     for ln, c in code_idx:
>         proc_lines_by_line[f][ln] = 'tight_loop' if (ln in tight and pc != 'cycle_critical') else pc
> for f in files:
>     gen = '/generated/' in f
>     tgt = class_lines_gen if gen else class_lines
>     # non-proc classified by item kind
>     for k, v in kind_lines[f].items():
>         if k == 'proc':
>             continue
>         if k in ('comptime fn', 'comptime test'):
>             tgt['comptime_code'] += v
>         elif k in ('script', 'dispatch'):
>             tgt['script_dispatch'] += v
>         elif k in ('data', 'table', 'offsets'):
>             tgt['data_tables'] += v
>         elif k in ('const', 'equ', 'struct', 'bitfield', 'enum', 'comptime enum', 'vars', 'region', 'newtype'):
>             tgt['decls_layout'] += v
>         elif k in ('use', 'module'):
>             tgt['imports'] += v
>         elif k in ('ensure', 'ensure_fatal'):
>             tgt['item_guards'] += v
>         elif k in ('context', 'interface', 'implement', 'type'):
>             tgt['contracts_decl'] += v
>         else:
>             tgt['other_toplevel'] += v
>     for ln, pc in proc_lines_by_line[f].items():
>         tgt['proc:' + pc] += 1
> 
> # ---- write outputs ----
> cols = ['lines', 'blank', 'comment_only', 'doc_comment', 'code', 'code_with_trailing_comment', 'dated_comment_lines',
>         'history_word_lines', 'as_echo', 'widths_pinned_note', 'asm_file_ref', 'numoff_any', 'numoff_typed_reg',
>         'stride_literal', 'stride_shift', 'literal_with_derivation_comment', 'ensure_item_position',
>         'ensure_pin_name_eq_literal', 'ensure_selftest_call_eq_literal', 'ensure_pin_expr_eq_literal', 'ensure_eq_derived', 'ensure_bound', 'ensure_cycles', 'ensure_other',
>         'static_table_loop', 'branch_sized', 'branch_unsized']
> with open(os.path.join(OUT, 'perfile.tsv'), 'w') as o:
>     o.write('file\tgenerated\t' + '\t'.join(cols) + '\n')
>     for f in files:
>         s = perfile[f]
>         o.write(f + '\t' + str(int(s['generated'])) + '\t' + '\t'.join(str(s[c]) for c in cols) + '\n')
> tot = {'hand': Counter(), 'generated': Counter()}
> for f in files:
>     for c in cols:
>         tot['generated' if perfile[f]['generated'] else 'hand'][c] += perfile[f][c]
> with open(os.path.join(OUT, 'features.tsv'), 'w') as o:
>     o.write('feature\tuses_hand\tfiles_hand\tuses_gen\tfiles_gen\tfirst_hit\n')
>     for k, _ in FEATURES:
>         hand = {f: c for f, c in feat_file[k].items() if '/generated/' not in f}
>         gen = {f: c for f, c in feat_file[k].items() if '/generated/' in f}
>         first = feat_hits[k][0] if feat_hits[k] else ''
>         o.write(f'{k}\t{sum(hand.values())}\t{len(hand)}\t{sum(gen.values())}\t{len(gen)}\t{first}\n')
> for k, v in hits.items():
>     open(os.path.join(OUT, 'hits', k + '.txt'), 'w').write('\n'.join(v) + '\n')
> for k, v in ens_hits.items():
>     open(os.path.join(OUT, 'hits', 'ensure_' + k + '.txt'), 'w').write('\n'.join(v) + '\n')
> for k, v in feat_hits.items():
>     safe = re.sub(r'[^\w]+', '_', k).strip('_')
>     open(os.path.join(OUT, 'hits', 'feat_' + safe + '.txt'), 'w').write('\n'.join(v) + '\n')
> json.dump({'files': len(files), 'hand_files': sum(1 for f in files if not perfile[f]['generated']),
>            'totals': {k: dict(v) for k, v in tot.items()}, 'ensure': dict(ens),
>            'class_lines': dict(class_lines), 'class_lines_generated': dict(class_lines_gen),
>            'proc_class_count': dict(proc_class_count), 'procs': len(proc_records)},
>           open(os.path.join(OUT, 'summary.json'), 'w'), indent=1)
> print(json.dumps({'totals': {k: dict(v) for k, v in tot.items()}, 'ensure': dict(ens), 'class_lines': dict(class_lines),
>                   'class_gen': dict(class_lines_gen), 'proc_class_count': dict(proc_class_count), 'procs': len(proc_records)}, indent=1))
> 
> # per-directory proc-class breakdown
> bydir = defaultdict(Counter)
> for f, d in proc_lines_by_line.items():
>     p = f.split('/')
>     key = '/'.join(p[:3]) if len(p) > 3 else '/'.join(p[:2])
>     for ln, pc in d.items():
>         bydir[key][pc] += 1
> with open(os.path.join(OUT, 'proc_class_by_dir.tsv'), 'w') as o:
>     for k in sorted(bydir):
>         o.write(k + '\t' + json.dumps(dict(bydir[k])) + '\n')
> cc_files = sorted({f for (f, name, a, b, attrs, pl) in proc_records if proc_class(f, name, attrs, pl) == 'cycle_critical'})
> open(os.path.join(OUT, 'cycle_critical_files.txt'), 'w').write('\n'.join(cc_files) + '\n')
> 
> # ---- procs.py ----
> import re,os,sys,collections
> R=sys.argv[1]; C=collections.Counter(); byclass=collections.defaultdict(collections.Counter)
> for top in ('engine','games'):
>   for dp,_,fs in os.walk(os.path.join(R,top)):
>     for fn in fs:
>       if not fn.endswith('.emp') or '/generated/' in dp+'/': continue
>       t=open(os.path.join(dp,fn)).read()
>       t=re.sub(r'//[^\n]*','',t)
>       for m in re.finditer(r'^[ \t]*(?:pub\s+)?proc\s+\w+[^{]*\{',t,re.M):
>         h=m.group(0); C['procs']+=1
>         for k in ('clobbers','preserves','out','inout','requires','grants','falls_into'):
>           if re.search(r'(?<![\w.])'+k+r'\b',h): C[k]+=1
>         if re.search(r'\(\s*(?:[ad][0-7]\s*:)',h): C['typed_params']+=1
>         if not re.search(r'\b(clobbers|preserves|out|inout)\b',h): C['no_register_contract']+=1
> print(dict(C))
> # ---- tables.py ----
> import csv,sys,collections,json
> rows=list(csv.DictReader(open('out/perfile.tsv'),delimiter='\t'))
> def I(r,k): return int(r[k])
> def share(r): c=I(r,'comment_only'); d=I(r,'code'); return 100*c/(c+d) if c+d else 0
> hand=[r for r in rows if r['generated']=='0']; gen=[r for r in rows if r['generated']=='1']
> hdr='| File | Lines | Code | Comment share | Dated | AS echo | Width-pin notes | `.asm` refs | Raw n(aN) | Stride lit/shift | Lit+derivation comment | Ensure pins (name==lit) | Static-table loops | Sized branches |'
> print(hdr); print('|'+'---|'*14)
> def line(name,r):
>   return f"| {name} | {I(r,'lines')} | {I(r,'code')} | {share(r):.0f}% | {I(r,'dated_comment_lines')} | {I(r,'as_echo')} | {I(r,'widths_pinned_note')} | {I(r,'asm_file_ref')} | {I(r,'numoff_any')} | {I(r,'stride_literal')}/{I(r,'stride_shift')} | {I(r,'literal_with_derivation_comment')} | {I(r,'ensure_pin_name_eq_literal')} | {I(r,'static_table_loop')} | {I(r,'branch_sized')} |"
> for r in sorted(hand,key=lambda r:-I(r,'lines')): print(line('`'+r['file']+'`',r))
> def tot(rs):
>   t=collections.Counter()
>   for r in rs:
>     for k,v in r.items():
>       if k not in('file','generated'): t[k]+=int(v)
>   return {k:str(v) for k,v in t.items()}
> print(line(f'**TOTAL hand-written ({len(hand)} files)**',tot(hand)))
> print(line(f'**TOTAL generated/ ({len(gen)} files)**',tot(gen)))
> print()
> # per directory
> dirs=collections.defaultdict(list)
> for r in hand:
>   p=r['file'].split('/'); key='/'.join(p[:2]) if p[0]=='engine' else '/'.join(p[:3]) if len(p)>3 else '/'.join(p[:2])
>   dirs[key].append(r)
> print('| Directory | Files | Lines | Code | Comment share | Dated | AS echo |'); print('|---|---|---|---|---|---|---|')
> for d,rs in sorted(dirs.items()):
>   t=tot(rs); print(f"| `{d}` | {len(rs)} | {t['lines']} | {t['code']} | {share(t):.0f}% | {t['dated_comment_lines']} | {t['as_echo']} |")
> ```
