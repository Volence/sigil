---
kind: plan
status: awaiting-owner
description: "Empyrean v2 step 0, task A10: six `.emp` constructs a green build accepts today become refusals (bare number operand, odd instruction after inline data, repeated contract clause, duplicate const, undeclared SR write in the gated build, binding to a proc with no clobbers clause). Sigil only; aeon measured at 0 exposure in all 10 shapes."
---
# A10 Refuse the always-wrong lines Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **AEON PAIR: NOT NEEDED.** Measured for this plan on aeon `9b80db1d` with a scratch sigil carrying all six refusals: every shape of both games (7 shipped + 3 fixture rows) builds and is byte-identical to today, and aeon's expect-fail lane is 56/56. No aeon source line is refused. The only aeon change is one optional doc sentence (Task 8), on its own aeon branch, landed after sigil; it is not a pair.

**Goal:** Six `.emp` lines that are always wrong stop the build with a named diagnostic instead of assembling, and no shape of any game moves a byte.

**Architecture:** Each refusal is one new check at the single place the construct is accepted today, in the `.emp` front end (`sigil-frontend-emp`), except the SR one, which promotes an existing warning inside the gated `sigil build` (`sigil-cli`). No analysis is rewritten and no existing diagnostic changes its text. The AS front end (`sigil-frontend-as`) is untouched: none of the six is an AS-syntax defect.

**Tech Stack:** Rust workspace pinned to 1.99.1 (`rust-toolchain.toml`); `sigil build --project <game dir> --shape <row>`; `scripts/step0_shapes_cmp.sh`; `scripts/landing-run.sh`; `scripts/provision-aeon-ref.sh`.

**Spec:** `/home/volence/sonic_hacks/empyrean/docs/v2/tasks/A10 Refuse the always-wrong lines.md` (the owner-approved card; read it whole, it is short). Evidence: `/home/volence/sonic_hacks/empyrean/docs/v2/language/5 Core verdict.md` rows W1, W5, W7, W11, W13.

## Global Constraints

- Branch `parcel/a10-refuse-always-wrong`, worktree `/home/volence/sonic_hacks/.sigil-a10`, cut from sigil master `1e200e1b`. Run `git branch --show-current` before every commit. Never commit to master, never touch `/home/volence/sonic_hacks/sigil` or another worktree.
- Line numbers in this plan are as of `1e200e1b`. Tasks 1 and 2 both edit `parser.rs`, so Task 2's numbers shift by what Task 1 added: find by the quoted text, not the number.
- **Not byte-changing.** Every task ends with protocol **BN** printing `SHAPES 7/7 IDENTICAL`. A byte that moves is STOP and BLOCKED: never narrow a refusal to get back to green.
- A refusal is `Level::Error` and its message starts with the bracketed id given in the task, copied exactly. No existing message text changes.
- The two judgement calls keep the card's limits (its "Limit" column, verbatim): the SR one is "an error only where the gate runs; the single-file path is a separate question (review B, Q15)"; the binding one is "'Refuse for now', not 'contracts must always be written': whether unwritten contracts are inferred is open for review B (Q3)". Tasks 5 and 6 each carry a test that pins the limit.
- `.emp` only. Do not change `crates/sigil-frontend-as`, `crates/sigil-isa`, or any backend.
- Not in this task (card, verbatim): "The interrupt-mask leak, the flag result, checks through pointers, build-time typing, unchecked build-time code, the nested value `if`, and the cross-CPU operand (Q19, to settle before G0c)."
- Never build inside a live aeon checkout. Reference tree: `/home/volence/sonic_hacks/.aeon-a10-ref` (Task 0). A landing run takes a `cp -a` copy of it placed outside every checkout, with `__pycache__` removed.
- Build with `CARGO_TARGET_DIR=.target-land` in the worktree. Tests need `AEON_DIR=/home/volence/sonic_hacks/.aeon-a10-ref ORACLE_DIR=/home/volence/sonic_hacks/oracle-old`.
- `df -h /` before a landing run or a multi-shape build; under 60 GB free, do not run it: BLOCKED.
- Before reporting a task: `CARGO_TARGET_DIR=.target-land cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- Never use an `mcp__oracle__*` tool. No subagents. Exact-path `git add`; `git show --stat` after each commit; the finding goes in the commit body; no co-author trailer. Never pass a commit message containing backticks through `-m "..."` (the shell runs them): use `git commit -F <file>`.
- Doc sync: discovered work is booked in `docs/QUEUE.md` in the same change. The empyrean checkout is read-only for implementers; the task card's Progress line is the overseer's to write.

## Shared protocols

**BN (byte-neutral).** After building the candidate (`CARGO_TARGET_DIR=.target-land cargo build --release -p sigil-cli`):

```bash
scripts/step0_shapes_cmp.sh --base /home/volence/sonic_hacks/.a10-base-1e200e1b/sigil \
  --cand .target-land/release/sigil --aeon-ref /home/volence/sonic_hacks/.aeon-a10-ref
```

Expected last two lines: `SHAPES 7/7 IDENTICAL` and `STEP0_CMP_END rc=0`. Exit 1 = a byte moved (STOP). Exit 2 = could not run, never a pass. This replaces G0b's cross-driver comparison: both sides now speak the `--project` grammar against one tree, and the base is sigil master `1e200e1b` itself.

**RF (red-first with the mutation shown).** For each refusal, after its commit: (1) `git status --short` shows no tracked change; (2) apply the task's named one-line mutation, show `git diff --stat` and `grep -n` of the mutated line; (3) run the task's named test: it must FAIL on the probe assertion (still green = stale binary or wrong target dir, fix the runner); (4) `git restore --source=HEAD -- <file>`, `git status --short` clean, re-run: PASS; (5) paste steps 2 to 4 into the parcel note `docs/superpowers/notes/2026-11-21-a10-refuse-always-wrong.md`.

**TC (touched crate).** `AEON_DIR=/home/volence/sonic_hacks/.aeon-a10-ref ORACLE_DIR=/home/volence/sonic_hacks/oracle-old CARGO_TARGET_DIR=.target-land cargo test --release -p <crate> --no-fail-fast`, quoting the sum of every `test result:` line, never a tail.

## Review Focus

1. **Z80 code.** A bare number is the legal Z80 immediate (`ld a, 5`: aeon has 154 such lines in 6 sound modules) and Z80 has no alignment rule. Neither refusal may reach a Z80 section. Tests in Tasks 3 and 4.
2. **A spliced integer in a template** (`move.w {n}, d0` inside `asm { }`) is the macro idiom and stays an immediate; only a number written in operand position is refused. Test in Task 3.
3. **Inline data that is legal:** an odd run after the last instruction of a body, two odd runs that sum to even, an even run. Aeon has 61 inline `dc` runs. Tests in Task 4.
4. **`clobbers()` is a contract** (it says "changes nothing"); only a missing clause is refused. A binding inside a comptime group whose condition is false is not bound and is not refused. Tests in Task 5.
5. **`SIGIL_WARNINGS=off` must not hide the SR refusal:** the promotion reads the build's warning list, not what was printed. Test in Task 6.

## Baseline measured for this plan (2026-11-21)

Sigil `1e200e1b` (release build), aeon `9b80db1d`. "Prototype" is a scratch clone of the branch carrying all six refusals as specified below; it was deleted after measuring.

**Today's behaviour, reproduced** (`sigil emp <file> -o out.bin`, exit 0 each):

| Probe | Bytes today |
|---|---|
| `move.w $1234, d0` / `rts` | `30 3C 12 34 4E 75` (control `#$1234` then `($1234).w`: `30 3C 12 34 30 38 12 34`) |
| `dc.b 1` / `nop` / `rts` | `01 4E 71 4E 75`, `nop` at offset 1 |
| `proc P () clobbers(d0) clobbers() { moveq #1, d0 / rts }` | `70 01 4E 75`, lint names `d0` as if `clobbers()` alone |
| `const Foo = 1` / `const Foo = 2` / `moveq #Foo, d0` | `70 02 4E 75` |
| `move.w #$2700, sr` in `clobbers(d0)` | `70 00 46 FC 27 00 4E 75`, `[proc.sr-undeclared]` printed, exit 0 |
| AS route, `move.w $1234,d0` | `30 38 12 34` (a memory read: correct, and why the AS front end is out of scope) |
| `equ Foo = 1` / `equ Foo = 2` | already refused: ``symbol `Foo` redefined by section `text` `` |

**Aeon exposure, per refusal** (207 tracked `.emp`: 72 engine, 131 games incl. 47 poison fixtures, 4 tool fixtures):

| Refusal | Lines refused | Text scan | Build measure |
|---|---|---|---|
| Bare number operand | **0** | 15,555 instruction-shaped lines; 154 literal-number operands, all in the 6 `cpu: z80` modules; 0 in 68000 code | prototype builds all 10 shapes |
| Odd instruction after inline data | **0** | 61 inline `dc` runs, 0 of odd length followed by code | same |
| Repeated contract clause | **0** | 0 headers repeat any of the 7 clause words | parser check runs on every scanned file, poison fixtures included |
| Duplicate const | **0** | 0 names declared twice per file over 207 files | same |
| Undeclared SR write | **0** | n/a | `proc.sr-undeclared` absent from the warn tally of all 10 shapes |
| Binding with no clause | **0** | 8 bindings (sonic4: `entry` + 6 hooks, 2 of them in a comptime group; demo: `entry`); all 8 bound procs declare `clobbers` | same |

What the text scans can miss: operands that reach an instruction through a template, number-valued expressions (`sizeof(T)`), and anything in a module no shape places. The build measure covers the first two for every placed module. A module no shape places is covered for the two parser refusals (the scan parses every file under the roots) and not for the two lowering refusals.

**Build measure.** Base and prototype, `sigil build --project <copy>/games/<g> --shape <row>`, sha256 of the ROM equal on both sides for all 10: sonic4 `release`, `debug`, `config-a`, `config-a-keystones-flipped`, `config-b`, `lean`, `stress-evict`, `stress-art`; demo `release`, `debug`. Aeon `tools/emp_expect_fail.py` with the prototype: `OK — 56/56 cases (54 comptime + 2 link)`. A planted `move.w #$2700, sr` in `games/demo/demo_state.emp` builds with the base (exit 0, tally `proc.sr-undeclared 1`) and stops the prototype (exit 1, no ROM); with `SIGIL_CONTRACTS=0`, with `--check`, and with `clobbers(d0/sr)` the prototype exits 0.

**Sigil's own tests and fixtures broken by each refusal** (full strict suite on the prototype against a provisioned tree whose four ROMs match the provenance tip):

Result: 559 `test result:` lines, 6182 passed, 4 failed, 4 ignored (6182 + 4 = the 6186 baseline of chain #212, so nothing else moved). The 4 reds, each read from its panic text:

| Refusal | Tests broken | Which |
|---|---|---|
| Bare number operand | **3** | `crates/sigil-frontend-emp/tests/out_width.rs:856` `an_out_type_the_corpus_cannot_resolve_is_reported` (`lea 4, a0`); `tests/z80_operands.rs:245` `module_no_cpu_attr_defaults_m68000_and_z80_mnemonic_is_unrecognized` (`ld a, 5` in a 68000 module); `tests/as_author_forms_stay_off_emp.rs:84` `a_width_suffix_on_a_name_is_what_it_was` (`jmp $1234.w`) |
| Odd instruction after inline data | **1** | `crates/sigil-cli/tests/tranche8_negative_probes.rs:173` `dc_cannot_be_shadowed_by_comptime_fn` (`dc.b 7` then `rts`) |
| Repeated contract clause | **0** | |
| Duplicate const | **0** | |
| Binding with no clause | **0** | |
| Undeclared SR write | **0** | warnings are unchanged; only the gated CLI build reads them |

No `.emp` fixture file, golden or corpus file in the sigil repo is refused: all four are inline source strings in tests.

## Order

| Order | Task | Row | File the refusal lives in | Depends on |
|---|---|---|---|---|
| 0 | Base binary, reference tree, parcel note | none | none | none |
| 1 | Repeated contract clause | W13 | `parser.rs` (clause loops) | 0 |
| 2 | Duplicate const | W13 | `parser.rs` (`file`) | 1 (same file) |
| 3 | Bare number operand | W1 | `eval/asm.rs` | 0 |
| 4 | Odd instruction after inline data | W11 | `lower/code.rs` | 0 |
| 5 | Binding with no clause (judgement call) | W5 | `resolve/contract.rs` | 0 |
| 6 | Undeclared SR write (judgement call) | W7 | `sigil-cli/src/main.rs` | 0 |
| 7 | All shapes identical, strict landing run | all | none | 1 to 6 |
| 8 | Aeon pitfalls sentence (aeon branch, optional) | W7 | aeon `docs/EMP_PITFALLS.md` | 7 landed |

Tasks 1 to 6 touch disjoint regions; only 1 and 2 share a file, so they run in that order and nothing else rebases.

---

### Task 0: Base binary, reference tree, parcel note

**Files:**
- Create: `docs/superpowers/notes/2026-11-21-a10-refuse-always-wrong.md`

**Interfaces:**
- Produces: `/home/volence/sonic_hacks/.a10-base-1e200e1b/sigil` (the base for BN), `/home/volence/sonic_hacks/.aeon-a10-ref` (the reference tree), and the parcel note every later task appends its RF transcript to.

- [ ] **Step 1: Confirm the branch holds no code change.** `git diff --stat 1e200e1b HEAD -- crates scripts contract Cargo.toml Cargo.lock` prints nothing.
- [ ] **Step 2: Build and park the base.** `df -h /`, then `CARGO_TARGET_DIR=.target-land cargo build --release --locked -p sigil-cli -p sigil-harness --bins`; `mkdir /home/volence/sonic_hacks/.a10-base-1e200e1b && cp .target-land/release/sigil .target-land/release/emit_sound_blob /home/volence/sonic_hacks/.a10-base-1e200e1b/`; record `sha256sum` of both in the note.
- [ ] **Step 3: Provision the reference tree.** `REF_BUILD_DEMO=1 CARGO_TARGET_DIR=.target-land scripts/provision-aeon-ref.sh /home/volence/sonic_hacks/.aeon-a10-ref` (the default revision is the provenance tip's `aeon_rev`, `9b80db1d`). Then run the `repin -- --check` command the script prints; expected last line `pins.rs unchanged`. Control, from `crates/sigil-harness/golden/provenance.toml` chain #212: the tree's ROMs are `s4.bin 142a39b9/850262`, `s4.debug.bin 20acdb67/870606`, `demo.bin 4587df52/99980`, `demo.debug.bin 37d66535/106981` (zlib CRC-32 / size). The live `/home/volence/sonic_hacks/aeon` checkout is NOT a reference tree: its `s4.bin` is `193507d9/850296`, and a strict run against it fails every `*_region_matches_reference` test for that reason alone. If the script cannot run (it adds a detached worktree to the aeon repo and fetches `origin`), report BLOCKED on this step with its output.
- [ ] **Step 4: Prove BN on itself.** Run BN with `--cand /home/volence/sonic_hacks/.a10-base-1e200e1b/sigil`. Expected `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`. Paste the seven `SHAPE` rows into the note.
- [ ] **Step 5: Write the note** with: the two sha256 lines, the seven rows, the "Baseline measured for this plan" tables above copied in, and an empty `## RF transcripts` heading.
- [ ] **Step 6: Commit** `docs/superpowers/notes/2026-11-21-a10-refuse-always-wrong.md`.

---

### Task 1 (W13): A repeated contract clause is refused

**Where it is accepted today.** `crates/sigil-frontend-emp/src/parser.rs`: `proc_decl` (`:2082`, clause loop `:2098-2145`) and `proc_sig` (`:2285`, loop `:2296-2323`, shared by `extern proc` `:2358`, `type X = proc` `:2402` and interface `hook` signatures `:2458`). Each `eat_kw("<clause>")` arm assigns its variable, so a second clause overwrites the first.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/parser.rs:2082-2147` and `:2285-2325`
- Create: `crates/sigil-frontend-emp/tests/proc_repeated_clause.rs`

**Interfaces:**
- Produces, in `impl Parser`: `fn note_contract_clause(&mut self, seen: &mut Vec<&'static str>, owner: &str)`. It looks at the next token without consuming it; if it is one of `requires`, `grants`, `clobbers`, `out`, `inout`, `preserves`, `falls_into` and that word is already in `seen`, it calls `self.diag_at(self.span(), ...)` with the message below; otherwise it records the word. Called as the first statement of both clause loops. `owner` is the proc's name in `proc_decl` and the literal `this signature` in `proc_sig`.
- Message (exact): ``[proc.repeated-clause] `{kw}` is written twice on `{owner}`: a contract clause may appear once, and the later one used to replace the earlier silently. Merge them into one clause``

- [ ] **Step 1: Write the failing tests.**

```rust
//! `[proc.repeated-clause]` (A10, W13): a contract clause written twice on one header is
//! refused. Before, the parser kept the last one silently.
use sigil_frontend_emp::parse_str;
use sigil_span::Level;

const ID: &str = "[proc.repeated-clause]";

fn refusals(src: &str) -> Vec<String> {
    parse_str(src).1.into_iter().filter(|d| d.level == Level::Error && d.message.contains(ID)).map(|d| d.message).collect()
}

#[test]
fn a_repeated_clobbers_clause_is_refused() {
    let r = refusals("module m\nproc P () clobbers(d0) clobbers() {\n    moveq #1, d0\n    rts\n}\n");
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].contains("`clobbers` is written twice on `P`"), "{}", r[0]);
}

#[test]
fn every_clause_word_is_refused_when_repeated() {
    let r = refusals("module m\nproc P () requires(v) requires(w) grants(v) grants(w) out(d0) out(d1) inout(d1) inout(d2) preserves(d2) preserves(d3) falls_into Q falls_into R {\n    rts\n}\n");
    for kw in ["requires", "grants", "out", "inout", "preserves", "falls_into"] {
        assert_eq!(r.iter().filter(|m| m.contains(&format!("`{kw}` is written twice"))).count(), 1, "{kw}: {r:?}");
    }
    assert_eq!(r.len(), 6, "{r:?}");
}

#[test]
fn a_repeated_clause_on_a_signature_is_refused() {
    let r = refusals("module m\nextern proc E () clobbers(d0) clobbers()\npub type T = proc () preserves(d0) preserves(d1)\npub interface G {\n    hook h () out(d0) out(d1) = empty\n}\n");
    assert_eq!(r.len(), 3, "{r:?}");
    assert!(r.iter().all(|m| m.contains("on `this signature`")), "{r:?}");
}

/// The control: every clause once, in an order the grammar allows, parses clean.
#[test]
fn each_clause_once_stays_legal() {
    let (_, diags) = parse_str("module m\ncontext v { granted }\nproc Q () clobbers() {\n    rts\n}\nproc P (d1: u8) requires(v) grants(v) clobbers(d0) out(d0) preserves(d2) inout(d1) falls_into Q {\n    moveq #0, d0\n}\n");
    assert!(diags.iter().all(|d| d.level != Level::Error), "{diags:?}");
}
```

- [ ] **Step 2: Run them red.** `CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test proc_repeated_clause`. Expected: 3 FAILED (`left: 0`), `each_clause_once_stays_legal` ok.
- [ ] **Step 3: Implement** `note_contract_clause` and the two call sites. Declare `let mut seen = Vec::new();` beside the clause variables in each function.
- [ ] **Step 4: Run them green.** Same command: `test result: ok. 4 passed`.
- [ ] **Step 5: Fix what the refusal breaks in sigil's own tests.** Measured: none. The prototype suite shows no test or fixture that uses the construct; if TC shows one, STOP and report it (the measurement was wrong) instead of editing the test.
- [ ] **Step 6: TC** for `sigil-frontend-emp`, clippy, **BN**.
- [ ] **Step 7: Commit** the two files (and any fixture from Step 5), then **RF**. Mutation: in `proc_decl` only, comment out the `self.note_contract_clause(...)` call line. Expected: `a_repeated_clobbers_clause_is_refused` and `every_clause_word_is_refused_when_repeated` FAIL, the signature test still passes (it reads the other call site).

---

### Task 2 (W13): A constant declared twice in one module is refused

**Where it is accepted today.** `crates/sigil-frontend-emp/src/eval/mod.rs:887-889` (`index_items`): `self.consts.insert(c.name.as_str(), c)` keeps the last. Its own doc (`:822-824`) says "Later duplicate names (a parse-level concern) are resolved last-wins by the index build; duplicate diagnosis is not this task's job." The parser is the one place that sees a module's own declarations before the resolve pass injects imported ones into `file.items`, so the check goes there.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (`pub fn file`, `:432-502`; the `File { module, attrs, items, .. }` line is `:501` at `1e200e1b`)
- Create: `crates/sigil-frontend-emp/tests/name_duplicate_const.rs`

**Interfaces:**
- Produces, in `impl Parser`: `fn refuse_duplicate_consts(&mut self, items: &[Item], seen: &mut std::collections::HashSet<String>)`, called once in `file` just before the `File { .. }` is built. It walks `Item::Const` and recurses into `Item::Section` bodies (one flat namespace, as `index_items` treats it). On a name already in `seen` it calls `self.diag_at(c.span, ...)`: the diagnostic sits on the LATER declaration.
- Message (exact): ``[name.duplicate-const] constant `{name}` is declared twice in this module, and the later one used to win silently; remove or rename one``
- Scope, decided (Open question 4): `const` against `const` only. `equ` twice is already a link error; `const` against `equ` is booked, not refused here.

- [ ] **Step 1: Write the failing tests.**

```rust
//! `[name.duplicate-const]` (A10, W13): a `const` declared twice in one module is refused.
//! Before, the evaluator's index kept the last one.
use sigil_frontend_emp::parse_str;
use sigil_span::Level;

const ID: &str = "[name.duplicate-const]";

fn refusals(src: &str) -> Vec<sigil_span::Diagnostic> {
    parse_str(src).1.into_iter().filter(|d| d.level == Level::Error && d.message.contains(ID)).collect()
}

#[test]
fn a_const_declared_twice_is_refused_at_the_later_one() {
    let src = "module m\nconst Foo = 1\nconst Foo = 2\n";
    let r = refusals(src);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].message.contains("constant `Foo` is declared twice"), "{}", r[0].message);
    assert_eq!(r[0].primary.start as usize, src.rfind("const Foo").unwrap(), "anchored at the later declaration");
}

#[test]
fn pub_typed_and_section_nested_duplicates_are_refused() {
    assert_eq!(refusals("module m\npub const Foo = 1\nconst Bar = 2\nconst Foo: u8 = 3\n").len(), 1);
    assert_eq!(refusals("module m\nconst Foo = 1\nsection text {\n    const Foo = 2\n}\n").len(), 1);
}

/// Controls: distinct names; a const beside an enum, a fn and an imported name.
#[test]
fn distinct_names_and_other_kinds_stay_legal() {
    assert!(refusals("module m\nconst Foo = 1\nconst Bar = 2\n").is_empty());
    assert!(refusals("module m\nconst Foo = 1\nenum Foo: u8 { A }\ncomptime fn Bar() -> int {\n    return 1\n}\nconst Baz = 2\n").is_empty());
    assert!(refusals("module m\nuse other.{Foo}\nconst Foo = 1\n").is_empty());
}

/// The limit of this task: `const` against `equ` is not refused here (booked in docs/QUEUE.md).
#[test]
fn a_const_beside_an_equ_of_the_same_name_is_not_this_refusal() {
    assert!(refusals("module m\nconst Foo = 1\nequ Foo = 2\n").is_empty());
}
```

- [ ] **Step 2: Run them red.** `CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test name_duplicate_const`. Expected: the first two FAILED, the two controls ok. If `Span.start`'s type makes the `as usize` cast a clippy lint, compare in the span's own type.
- [ ] **Step 3: Implement** `refuse_duplicate_consts` and its call.
- [ ] **Step 4: Run them green:** `test result: ok. 4 passed`.
- [ ] **Step 5: Fix what the refusal breaks in sigil's own tests.** Measured: none. The prototype suite shows no test or fixture that uses the construct; if TC shows one, STOP and report it (the measurement was wrong) instead of editing the test.
- [ ] **Step 6: Book the leftover.** Add to `docs/QUEUE.md`: `## EMP-CONST-EQU-SAME-NAME` (state open, size `S`, project `STEP0-A10`): a `const` and an `equ` of one name in one module build, and the `const` wins for a comptime read (`const Foo = 1` / `equ Foo = 2` / `moveq #Foo, d0` gives `70 01`, measured at `1e200e1b`); the same last-wins shape exists for repeated `data`/`table` attributes and `context` fields (language review A, stage I1 row 8). Also correct the stale doc sentence at `eval/mod.rs:822-824` to say the parser refuses a duplicate `const`.
- [ ] **Step 7: TC** for `sigil-frontend-emp`, clippy, **BN**.
- [ ] **Step 8: Commit**, then **RF**. Mutation: comment out the `self.refuse_duplicate_consts(...)` call in `file`. Expected: the first two tests FAIL.

---

### Task 3 (W1): A bare number as a 68000 operand is refused

**Where it is accepted today.** `crates/sigil-frontend-emp/src/eval/asm.rs`, `map_plain` (`:3455-3558`). A `Plain` operand that is not a path and not a `Sym ± const` sum falls to the last two lines (`:3556-3557`): `let v = self.eval_expr(expr, env); self.classify_operand_splice(v, expr_span(expr))`, and `classify_operand_splice` (`:3649`) turns an integer into `CodeOperand::Imm`. That function is shared with `{splice}` operands (`:2440`), which must keep doing so.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/eval/asm.rs:3556-3557`
- Create: `crates/sigil-frontend-emp/tests/operand_bare_number.rs`
- Modify: `crates/sigil-frontend-emp/tests/out_width.rs:865`, `tests/z80_operands.rs:247-248`, `tests/as_author_forms_stay_off_emp.rs:109` (Step 5)

**Interfaces:**
- The change: between those two lines, `if let Some(n) = v.as_stored_int() { self.error(expr_span(expr), <message>); return None; }`. Nothing else moves. `map_plain` is reached only for 68000 sections (`map_operand` sends Z80 to `map_operand_z80` at `:2174`).
- Message (exact): ``[operand.bare-number] a bare number ({n}) is not a 68000 operand: write `#` before it for the value, or `(addr).w` / `(addr).l` for the memory at that address`` with `{n}` the evaluated integer in decimal.
- The existing follow-on error "instruction dropped, a mnemonic, size, or operand did not resolve" still prints after it; do not suppress it.

- [ ] **Step 1: Write the failing tests.** Use the `lower` helper of `tests/operand_const_as_address.rs:19-33` (copy it: `parse_str`, then `lower_module` with `initial_cpu: Cpu::M68000`), plus a second `lower_z80` with `initial_cpu: Cpu::Z80`.

```rust
const ID: &str = "[operand.bare-number]";
fn firings(d: &[Diagnostic]) -> Vec<&Diagnostic> { d.iter().filter(|d| d.message.contains(ID)).collect() }
fn errors(d: &[Diagnostic]) -> Vec<&Diagnostic> { d.iter().filter(|d| d.level == Level::Error).collect() }

#[test]
fn a_bare_number_operand_is_refused() {
    let d = lower("module m\nproc p () clobbers(d0) {\n    move.w $1234, d0\n    rts\n}\n");
    let f = firings(&d);
    assert_eq!(f.len(), 1, "{d:?}");
    assert_eq!(f[0].level, Level::Error);
    assert!(f[0].message.contains("a bare number (4660)"), "{}", f[0].message);
}

#[test]
fn a_number_valued_expression_is_refused_too() {
    let d = lower("module m\nproc p () clobbers(d0) {\n    move.w sizeof(u16), d0\n    rts\n}\n");
    assert_eq!(firings(&d).len(), 1, "{d:?}");
}

/// The two spellings the message offers both assemble.
#[test]
fn the_immediate_and_the_sized_absolute_stay_legal() {
    let d = lower("module m\nproc p () clobbers(d0) {\n    move.w #$1234, d0\n    move.w ($1234).w, d0\n    rts\n}\n");
    assert!(errors(&d).is_empty(), "{d:?}");
}

/// Review Focus 2: a spliced integer in a template is the macro idiom and stays an immediate.
#[test]
fn a_spliced_integer_stays_an_immediate() {
    let d = lower("module m\ncomptime fn put(n: int) -> Code {\n    return asm {\n        move.w  {n}, d0\n    }\n}\nproc p () clobbers(d0) {\n    put(5)\n    rts\n}\n");
    assert!(errors(&d).is_empty(), "{d:?}");
}

/// Review Focus 1: on the Z80 a bare number IS the immediate.
#[test]
fn a_z80_bare_number_is_untouched() {
    let d = lower_z80("module m\nproc p () clobbers(a) {\n    ld a, 5\n    ret\n}\n");
    assert!(firings(&d).is_empty() && errors(&d).is_empty(), "{d:?}");
}
```

- [ ] **Step 2: Run them red.** `CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test operand_bare_number`. Expected: the first two FAILED (`left: 0`), three ok.
- [ ] **Step 3: Implement** the three-line refusal.
- [ ] **Step 4: Run them green:** `test result: ok. 5 passed`.
- [ ] **Step 5: Fix what the refusal breaks in sigil's own tests.** Measured: exactly three tests, all inline strings. Any other red is a STOP.
  - `tests/out_width.rs:865`: `lea 4, a0` becomes `lea (4).w, a0` (the test is about the `out` type, not the operand; `a0` must still be produced).
  - `tests/z80_operands.rs:247-248`: `ld a, 5` becomes `ld a, b`, so the 68000 path still reaches "not a recognized 68000 mnemonic" with no number in it. (Probed on the prototype: it does.)
  - `tests/as_author_forms_stay_off_emp.rs:109`: the baseline for `jmp $1234.w` becomes the two strings the front end now gives: this task's message with `(4660)`, then `instruction dropped, a mnemonic, size, or operand did not resolve (a missing import or type in scope?)`. Add a comment: moved by A10 on purpose, was `illegal destination EA: #imm`.
- [ ] **Step 6: Book the leftover.** `docs/QUEUE.md`: `## EMP-SPLICED-INT-OPERAND` (open, `S`, `STEP0-A10`): `move.w {n}, d0` with an integer `n` still assembles as an immediate with no `#` in the template; refused nowhere, by decision in this plan (Open question 2); count aeon's uses before changing it.
- [ ] **Step 7: TC** for `sigil-frontend-emp`, clippy, **BN**.
- [ ] **Step 8: Commit**, then **RF**. Mutation: change `if let Some(n) = v.as_stored_int()` to `if let (false, Some(n)) = (true, v.as_stored_int())`. Expected: the first two tests FAIL.

---

### Task 4 (W11): An instruction at an odd address inside a body is refused

**Where it is accepted today.** `crates/sigil-frontend-emp/src/lower/code.rs`, `lower_code_buf` (`:54`), the one function every 68000 code stream goes through (named proc `lower/proc.rs:149`, script `lower/script.rs:177`, dispatch body `lower/mod.rs:1748`, and its own recursion for `mul_*` at `:140`). Its `CodeItem::Inline` arm (`:77-83`) emits a `dc` run's bytes and the next `CodeItem::Instr` follows at whatever parity results. The only parity check is on the item's own label (`[layout.odd-item]`, `lower/mod.rs:1164-1218`, recorded at `lower/proc.rs:122-130`).

**Why a static check is exact.** Every 68000 instruction is an even number of bytes at every relaxation, so inside one body the parity of an instruction's offset is the parity of the inline data before it. The body's start is already asserted even by `[layout.odd-item]`.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/code.rs:54-72`
- Create: `crates/sigil-frontend-emp/tests/layout_odd_instruction.rs`
- Modify: `crates/sigil-cli/tests/tranche8_negative_probes.rs:173-186` (Step 5)

**Interfaces:**
- Produces, private in `code.rs`: `fn first_odd_instr(items: &[CodeItem]) -> Option<(Span, usize)>`: sums `DataBuf::size` over `CodeItem::Inline` in order and returns the span of the first `CodeItem::Instr` reached while the sum is odd, with that sum. It skips the zero-byte `assume_some` mnemonic (see `:105-107`). One firing per body: everything after it is shifted by the same byte.
- Called in `lower_code_buf` right after the `check_dispatch_targets` loop (`:69-71`), only when `cpu == Cpu::M68000`, pushing with `push_err` (`:2349`).
- `@as_compat` does NOT exempt it (Open question 3): AS pads an instruction to even, so an odd one is never what a faithful port means.
- Message (exact): ``[layout.odd-instruction] this instruction lands on an odd address: {n} byte(s) of inline `dc` data precede it in this body, and an odd instruction address is a guaranteed address-error crash on 68k; pad the data to an even length``

- [ ] **Step 1: Write the failing tests** (the `lower` and `lower_z80` helpers of Task 3's file, copied).

```rust
const ID: &str = "[layout.odd-instruction]";

#[test]
fn an_instruction_after_an_odd_data_run_is_refused() {
    let src = "module m\nproc p () clobbers() {\n    dc.b 1\n    nop\n    rts\n}\n";
    let d = lower(src);
    let f: Vec<_> = d.iter().filter(|d| d.message.contains(ID)).collect();
    assert_eq!(f.len(), 1, "one firing per body: {d:?}");
    assert_eq!(f[0].level, Level::Error);
    assert!(f[0].message.contains("1 byte(s) of inline `dc` data"), "{}", f[0].message);
    assert_eq!(f[0].primary.start as usize, src.find("nop").unwrap(), "anchored at the first odd instruction");
}

#[test]
fn as_compat_does_not_exempt_it() {
    let d = lower("module m\n@as_compat\nproc p () {\n    dc.b 1\n    nop\n    rts\n}\n");
    assert_eq!(d.iter().filter(|d| d.message.contains(ID)).count(), 1, "{d:?}");
}

/// Review Focus 3: even runs, odd runs that sum to even across a label, and odd data
/// after the last instruction are all legal.
#[test]
fn even_and_trailing_data_stay_legal() {
    for src in [
        "module m\nproc p () clobbers() {\n    dc.b 1, 2\n    nop\n    rts\n}\n",
        "module m\nproc p () clobbers() {\n    dc.b 1\n.x:\n    dc.b 2\n    nop\n    rts\n}\n",
        "module m\nproc p () clobbers() {\n    rts\n    dc.b \"abc\"\n}\n",
        "module m\nproc p () clobbers() {\n    dc.w 1\n    dc.l 2\n    nop\n    rts\n}\n",
    ] {
        let d = lower(src);
        assert!(!d.iter().any(|d| d.message.contains(ID)), "{src}\n{d:?}");
    }
}

/// Review Focus 1: the Z80 has no alignment rule.
#[test]
fn a_z80_body_is_untouched() {
    let d = lower_z80("module m\nproc p () clobbers(a) {\n    dc.b 1\n    nop\n    ld a, 5\n    ret\n}\n");
    assert!(!d.iter().any(|d| d.message.contains(ID)), "{d:?}");
}
```

- [ ] **Step 2: Run them red.** `CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test layout_odd_instruction`. Expected: the first two FAILED, two ok.
- [ ] **Step 3: Implement** `first_odd_instr` and its call.
- [ ] **Step 4: Run them green:** `test result: ok. 4 passed`.
- [ ] **Step 5: Fix what the refusal breaks in sigil's own tests.** Measured: exactly one test. Any other red is a STOP.
  - `crates/sigil-cli/tests/tranche8_negative_probes.rs:173-186`: `dc.b 7` becomes `dc.b 7, 8`, and the byte assertion becomes `&bytes[..4]` equal to `[0x07, 0x08, 0x4E, 0x75]`. The test's point (a comptime fn named `dc` does not shadow the statement) is unchanged. This file is in `sigil-cli`, so run TC for `sigil-cli` as well.
- [ ] **Step 6: Book the leftover.** `docs/QUEUE.md`: `## EMP-DISPATCH-BODY-PARITY` (open, `S`, `STEP0-A10`): the check is per code stream, and only a named proc, a script and a dispatch TABLE have their start asserted even; dispatch body k starts where body k-1 ended, so a body ending in an odd `dc` run after its last instruction shifts the next body with no diagnostic (read at `lower/mod.rs:1722-1752`, not probed).
- [ ] **Step 7: TC** for `sigil-frontend-emp`, clippy, **BN**.
- [ ] **Step 8: Commit**, then **RF**. Mutation: change the guard `cpu == Cpu::M68000` on the new call to `cpu == Cpu::Z80`. Expected: the first two tests FAIL; `a_z80_body_is_untouched` FAILS too (it now fires there), which shows the control has teeth.

---

### Task 5 (W5, judgement call): A binding to a proc with no contract clause is refused

**The card's limit (do not widen):** "'Refuse for now', not 'contracts must always be written': whether unwritten contracts are inferred is open for review B (Q3)". So: only a proc that is BOUND to an interface member is refused, only when it is declared with no `clobbers` clause, and the message says "for now". A proc nobody binds may still have no clause.

**Where it is skipped today.** `crates/sigil-frontend-emp/src/resolve/contract.rs`: `check_hook_sig` (`:389-401`, called at `:187`) and `check_proc_type` (`:371-386`, called at `:177`) both start with `let Some(bound) = find_proc_contract(all_items, bound_sym) else { return };`, and `find_proc_contract` (`:470-501`) returns `None` at `:488` (`p.clobbers.as_ref()?;`, comment "skip the subset check") and at `:493` for an `extern proc`. Both the production resolve pass and the gate's `bind_corpus_interfaces` call this bind, so the refusal reaches `sigil build` and `sigil emp --root` alike.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/resolve/contract.rs:371-401`
- Modify: `crates/sigil-frontend-emp/tests/game_contract.rs` (append; its helpers `bind_srcs`, `has_tag`, `errors` are at `:28-36` and `:73-79`)

**Interfaces:**
- Produces, private in `contract.rs`: `fn refuse_uncontracted(iface: &str, member: &str, bound_sym: &str, what: &str, all_items: &[&ast::Item], span: Span, diags: &mut Vec<Diagnostic>) -> bool`. True, with one diagnostic pushed, when `all_items` holds an `ast::Item::Proc` named `bound_sym` whose `clobbers` is `None`, or an `ast::Item::ExternProc` named `bound_sym` whose `sig.clobbers` is `None`. Called as the first statement of `check_hook_sig` (`what = "hook"`) and, in `check_proc_type`, right after its `let ast::Type::Named(p) = ptype else { return };` line (`what = "proc"`); each returns when it is true.
- `find_proc_contract` is not changed. A `bound_sym` declared nowhere in the module set stays unchecked and unrefused (Open question 5).
- Message (exact): ``[contract.binding-uncontracted] {what} `{iface}.{member}` is bound to `{bound_sym}`, which declares no `clobbers(...)` clause, so its conformance to the declared signature cannot be checked. Refused for now: state `clobbers(...)` on `{bound_sym}` (`clobbers()` if it changes no register)``
- Put a one-line comment on the function naming the limit: review B Q3 decides whether an unwritten contract is inferred instead.

- [ ] **Step 1: Write the failing tests** (append to `game_contract.rs`).

```rust
const ENGINE_TICK: &str = "module engine.c\npub interface Game {\n    hook tick () clobbers(d0-d1) = empty\n}\n";
const UNCONTRACTED: &str = "[contract.binding-uncontracted]";

#[test]
fn probe_hook_bound_to_a_proc_with_no_clobbers_clause_is_refused() {
    let game = "module games.a.m\npub implement Game {\n    hook tick = Bare\n}\nproc Bare() {\n    moveq #0, d5\n    rts\n}\n";
    let diags = bind_srcs(&[ENGINE_TICK, game], &[]);
    let hits: Vec<_> = errors(&diags).into_iter().filter(|m| m.contains(UNCONTRACTED)).collect();
    assert_eq!(hits.len(), 1, "{:?}", errors(&diags));
    assert!(hits[0].contains("hook `Game.tick` is bound to `Bare`") && hits[0].contains("Refused for now"), "{}", hits[0]);
}

#[test]
fn probe_proc_member_and_extern_targets_with_no_clause_are_refused() {
    let engine = "module engine.c\npub type St = proc () clobbers(d0)\npub interface Game {\n    proc entry: St\n    hook tick () clobbers(d0) = empty\n}\n";
    let game = "module games.a.m\npub implement Game {\n    proc entry = Bare\n    hook tick = Ext\n}\nproc Bare() {\n    rts\n}\nextern proc Ext ()\n";
    let diags = bind_srcs(&[engine, game], &[]);
    let e = errors(&diags);
    assert!(e.iter().any(|m| m.contains(UNCONTRACTED) && m.contains("proc `Game.entry` is bound to `Bare`")), "{e:?}");
    assert!(e.iter().any(|m| m.contains(UNCONTRACTED) && m.contains("hook `Game.tick` is bound to `Ext`")), "{e:?}");
}

/// Review Focus 4: `clobbers()` is a contract. This is the twin of the probe.
#[test]
fn a_binding_to_a_proc_declaring_the_empty_clause_conforms() {
    let game = "module games.a.m\npub implement Game {\n    hook tick = Quiet\n}\nproc Quiet() clobbers() {\n    rts\n}\n";
    let diags = bind_srcs(&[ENGINE_TICK, game], &[]);
    assert!(errors(&diags).is_empty(), "{:?}", errors(&diags));
}

/// The limit: only a BOUND proc is refused. An unbound proc with no clause, a binding in a
/// comptime group whose condition is false, and a target declared nowhere are left alone.
#[test]
fn the_refusal_reaches_only_a_live_binding_to_a_declared_proc() {
    let game = "module games.a.m\npub implement Game {\n    if OFF == 1 {\n        hook tick = Bare\n    }\n}\nproc Bare() {\n    rts\n}\n";
    let diags = bind_srcs(&[ENGINE_TICK, game], &[("OFF".to_string(), 0)]);
    assert!(!has_tag(&diags, UNCONTRACTED), "{:?}", errors(&diags));
    let game = "module games.a.m\npub implement Game {\n    hook tick = Elsewhere\n}\n";
    let diags = bind_srcs(&[ENGINE_TICK, game], &[]);
    assert!(!has_tag(&diags, UNCONTRACTED), "{:?}", errors(&diags));
}
```

- [ ] **Step 2: Run them red.** `CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test game_contract`. Expected: the two `probe_` tests FAILED, the other two ok, and the file's existing 23 tests ok.
- [ ] **Step 3: Implement** `refuse_uncontracted` and the two calls.
- [ ] **Step 4: Run them green:** `test result: ok. 27 passed`.
- [ ] **Step 5: Fix what the refusal breaks in sigil's own tests.** Measured: none. The prototype suite shows no test or fixture that uses the construct; if TC shows one, STOP and report it (the measurement was wrong) instead of editing the test.
- [ ] **Step 6: Book the leftover.** `docs/QUEUE.md`: `## EMP-BINDING-TARGET-UNDECLARED` (open, `S`, `STEP0-A10`): a binding whose target is declared nowhere in the bound module set (`find_proc_contract` finds no item) still skips the conformance check with no diagnostic; A10 refused only the declared-without-a-clause case, by the card's wording.
- [ ] **Step 7: TC** for `sigil-frontend-emp`, clippy, **BN**.
- [ ] **Step 8: Commit**, then **RF**. Mutation: comment out the `refuse_uncontracted` call in `check_hook_sig`. Expected: `probe_hook_bound_to_a_proc_with_no_clobbers_clause_is_refused` FAILS.

---

### Task 6 (W7, judgement call): An undeclared SR write stops the gated build

**The card's limit (do not widen):** "It is an error only where the gate runs; the single-file path is a separate question (review B, Q15)". The gate runs in exactly one arm: `sigil build --project ... -o ...` with `SIGIL_CONTRACTS` not `0` (`crates/sigil-cli/src/main.rs:3095-3101`). `--check`, `--report`, `SIGIL_CONTRACTS=0` and single-file `sigil emp` keep the warning.

**Where it is a warning today.** `crates/sigil-frontend-emp/src/lower/proc.rs:1378-1399` pushes `[proc.sr-undeclared]` at `Level::Warning`, and `eval/asm.rs:981-1001` pushes the same id for a context whose acquire and release do not round-trip `sr`. Both reach `sigil build` as `BuildWarning { id: "proc.sr-undeclared", .. }` in `BuildOutput::warnings`, printed by `report_warnings(&out.warnings)` at `main.rs:3230`. **Neither emission site changes**: the level stays `Warning`, so `sigil emp` and every harness build behave as today.

**The change.** The gated build refuses when its own warning list holds that id. Keyed on the id, so both emission sites are refused (Open question 1).

**Files:**
- Modify: `crates/sigil-cli/src/main.rs` (`run_contract_gate` `:2212-2530`, `run_project` `:3099-3100`, `run_project_build` `:3191-3232`, and the `#[cfg(test)]` module, beside `warning_summary_tallies_by_id_most_frequent_first` near `:4366`)
- Create: `crates/sigil-cli/tests/sr_undeclared_gate.rs`

**Interfaces:**
- `fn run_contract_gate(subject: impl FnOnce() -> ClosureSubject) -> bool`: `false` on the `SIGIL_CONTRACTS=0` early return (`:2215-2221`), `true` after the `contract closure gate: passed` line (`:2524-2529`). The failing paths still `process::exit(1)`.
- `fn run_project_build(project: &sigil_build::driver::Project, opts: &ProjectOpts, gated: bool)`; `run_project` passes the gate's return value.
- `const GATED_WARNING_IDS: [&str; 1] = ["proc.sr-undeclared"];`
- `fn gated_warning_refusal(warnings: &[sigil_harness::native::BuildWarning]) -> Vec<String>`: empty when no warning's `id` is in `GATED_WARNING_IDS`; otherwise the header line then one line per firing, `  {location}: {message}` or `  (no span recorded): {message}` when `location` is `None` (the shape `located_refusal` prints, `:2555`).
- Header (exact): ``error: [proc.sr-undeclared], {n} firing(s); an undeclared write to the status register stops a gated `sigil build`:``
- In `run_project_build`, right after `report_warnings(&out.warnings);` and before anything is written: when `gated` and the refusal is non-empty, print its lines to stderr, then (exact) `error: gated build FAILED on [proc.sr-undeclared]. Set SIGIL_CONTRACTS=0 (aeon: CONTRACTS=0) to build anyway, that is an emergency hatch, not a fix.` and `process::exit(1)`. No ROM, listing or export is written.

- [ ] **Step 1: Write the failing unit tests** in `main.rs`'s test module, building `BuildWarning`s with the closure that test at `:4368-4375` uses.

```rust
#[test]
fn gated_warning_refusal_lists_each_sr_firing_and_nothing_else() {
    // w(level, id) as in warning_summary_tallies_by_id_most_frequent_first, plus a location
    let mut a = w(warn, "proc.sr-undeclared");
    a.location = Some("engine/x.emp:4:5".into());
    let lines = crate::gated_warning_refusal(&[w(warn, "proc.clobber-undeclared"), a, w(warn, "proc.sr-undeclared")]);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(lines[0], "error: [proc.sr-undeclared], 2 firing(s); an undeclared write to the status register stops a gated `sigil build`:");
    assert_eq!(lines[1], "  engine/x.emp:4:5: [proc.sr-undeclared] whatever");
    assert_eq!(lines[2], "  (no span recorded): [proc.sr-undeclared] whatever");
}

#[test]
fn gated_warning_refusal_is_empty_when_no_gated_id_fired() {
    assert!(crate::gated_warning_refusal(&[]).is_empty());
    assert!(crate::gated_warning_refusal(&[w(warn, "proc.clobber-undeclared")]).is_empty());
}
```

- [ ] **Step 2: Write the failing whole-build tests** in `crates/sigil-cli/tests/sr_undeclared_gate.rs`. The gate fixture `write_project_fixture` cannot be used: its build stops before lowering, so no warning list exists. Follow `crates/sigil-cli/tests/build_check.rs:56-78` and `:206-213` instead: reference tree from `sigil_harness::test_support::reference_tree_for_shape(&sigil_harness::shapes::Shape::new("demo", "release"))` (return when `None`), ONE `copy_tree` of it per test binary under `env!("CARGO_TARGET_TMPDIR")` held in a `static OnceLock<PathBuf>`, a `static LOCK: Mutex<()>` taken by every test, `AEON_DIR` removed from the child. Each test rewrites `games/demo/demo_state.emp` in the copy as the pristine text (read once from the reference) plus its plant.

```rust
const PLANT: &str = "\npub proc Zz_A10_SrProbe () clobbers(d0) {\n        moveq   #0, d0\n        move.w  #$2700, sr\n        rts\n}\n";
const TWIN: &str = "\npub proc Zz_A10_SrProbe () clobbers(d0/sr) {\n        moveq   #0, d0\n        move.w  #$2700, sr\n        rts\n}\n";
const HEADER: &str = "error: [proc.sr-undeclared], 1 firing(s); an undeclared write to the status register stops a gated `sigil build`:";

// fn build(plant: &str, args: &[&str], env: &[(&str, &str)]) -> Option<(std::process::Output, String, PathBuf)>
//   runs `sigil build --project <copy>/games/demo --shape release <args>` and returns (output, stderr, rom path);
//   `args` is `["-o", <rom>]` for a build and `["--check"]` for a check. The rom path is removed before each run.

#[test]
fn an_undeclared_sr_write_stops_the_gated_build() {
    let Some((out, err, rom)) = build(PLANT, &["-o"], &[]) else { return };
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains(HEADER), "{err}");
    let line = err.lines().find(|l| l.contains("[proc.sr-undeclared] `Zz_A10_SrProbe` writes `sr`")).unwrap_or_else(|| panic!("{err}"));
    assert!(line.contains("games/demo/demo_state.emp:"), "located: {line}");
    assert!(err.contains("contract closure gate: passed"), "the closure gate itself ran and passed first:\n{err}");
    assert!(!rom.exists(), "a refused build writes no ROM");
}

/// Review Focus 5: silencing the warn tier does not silence the refusal.
#[test]
fn warnings_off_does_not_hide_the_refusal() {
    let Some((out, err, _)) = build(PLANT, &["-o"], &[("SIGIL_WARNINGS", "off")]) else { return };
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains(HEADER), "{err}");
}

/// The control: the same write, declared, builds.
#[test]
fn a_declared_sr_write_builds() {
    let Some((out, err, rom)) = build(TWIN, &["-o"], &[]) else { return };
    assert!(out.status.success(), "{err}");
    assert!(!err.contains("proc.sr-undeclared"), "{err}");
    assert!(rom.exists());
}

/// The card's limit, pinned: where the gate does not run it stays a warning.
#[test]
fn outside_the_gate_it_stays_a_warning() {
    let Some((out, err, rom)) = build(PLANT, &["-o"], &[("SIGIL_CONTRACTS", "0")]) else { return };
    assert!(out.status.success() && rom.exists(), "{err}");
    assert!(err.contains("proc.sr-undeclared 1") && !err.contains(HEADER), "{err}");
    let Some((out, err, _)) = build(PLANT, &["--check"], &[]) else { return };
    assert!(out.status.success(), "{err}");
    assert!(err.contains("proc.sr-undeclared 1") && !err.contains(HEADER), "{err}");
}

/// The single-file path (review B, Q15) is not tightened by A10. No reference tree needed.
#[test]
fn the_single_file_path_stays_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("w7.emp");
    std::fs::write(&src, "module w7\nproc P () clobbers(d0) {\n    moveq   #0, d0\n    move.w  #$2700, sr\n    rts\n}\n").unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_sigil")).arg("emp").arg(&src).arg("-o").arg(dir.path().join("w7.bin")).output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{err}");
    assert!(err.contains("[proc.sr-undeclared] `P` writes `sr`"), "{err}");
    assert_eq!(std::fs::read(dir.path().join("w7.bin")).unwrap(), [0x70, 0x00, 0x46, 0xFC, 0x27, 0x00, 0x4E, 0x75]);
}
```

- [ ] **Step 3: Run them red.** `AEON_DIR=/home/volence/sonic_hacks/.aeon-a10-ref SIGIL_STRICT_GATE=1 CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-cli --test sr_undeclared_gate` and `... -p sigil-cli --bin sigil gated_warning_refusal`. Expected: the unit tests do not compile (the function is missing; add an empty-returning stub to see them FAIL on `left: 0`), `an_undeclared_sr_write_stops_the_gated_build` and `warnings_off_does_not_hide_the_refusal` FAILED (exit code `Some(0)`), the three others ok.
- [ ] **Step 4: Implement** the signature changes, the constant, `gated_warning_refusal` and the refusal block.
- [ ] **Step 5: Run them green:** `test result: ok. 5 passed` and `ok. 2 passed`.
- [ ] **Step 6: Fix what the refusal breaks in sigil's own tests.** Measured: none. The prototype suite shows no test or fixture that fires the warning in a gated build; if TC shows one, STOP and report it (the measurement was wrong) instead of editing the test.
- [ ] **Step 7: TC** for `sigil-cli` (with `SIGIL_STRICT_GATE=1`), clippy, **BN**.
- [ ] **Step 8: Commit**, then **RF**. Mutation: in `run_project`, pass `false` instead of the gate's return value. Expected: `an_undeclared_sr_write_stops_the_gated_build` and `warnings_off_does_not_hide_the_refusal` FAIL with exit code `Some(0)`.

---

### Task 7: All shapes byte-identical, and the strict landing run

**Files:**
- Modify: `docs/superpowers/notes/2026-11-21-a10-refuse-always-wrong.md`, `docs/QUEUE.md` (a `## STEP0-A10-REFUSALS` row, state "built, awaiting landing")

- [ ] **Step 1: Rebuild the candidate** from the branch tip: `df -h /`; `CARGO_TARGET_DIR=.target-land cargo build --release --locked -p sigil-cli -p sigil-harness --bins`; `.target-land/release/sigil --version` names the tip.
- [ ] **Step 2: The seven shipped shapes.** Run **BN**. Expected `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`: sonic4 `release`, `debug`, `config-a`, `config-b`, `lean` and demo `release`, `debug`. This is the card's "DEBUG and demo included".
- [ ] **Step 3: The three fixture rows** `step0_shapes_cmp.sh` does not list. In a `cp -a` copy of the reference tree at `/home/volence/sonic_hacks/.a10-fx-$$` (remove its `.git` link file and every `__pycache__`):

```bash
for s in config-a-keystones-flipped stress-evict stress-art; do
  for side in base cand; do
    bin=/home/volence/sonic_hacks/.a10-base-1e200e1b/sigil; [ $side = cand ] && bin=$PWD/.target-land/release/sigil
    "$bin" build --project /home/volence/sonic_hacks/.a10-fx-$$/games/sonic4 --shape $s -o /home/volence/sonic_hacks/.a10-fx-$$/$s.$side.bin || echo "COULD NOT RUN $s $side"
  done
  cmp /home/volence/sonic_hacks/.a10-fx-$$/$s.base.bin /home/volence/sonic_hacks/.a10-fx-$$/$s.cand.bin && echo "FIXTURE $s SAME"
done
```

Expected: three `FIXTURE <row> SAME` lines and no `COULD NOT RUN`. Delete the copy.
- [ ] **Step 4: Aeon stays green with the candidate.** In a second copy of the reference tree outside every checkout, with `SIGIL_BUILD` and `SIGIL_EMIT` set to `.target-land/release/{sigil,emit_sound_blob}`: `./build.sh`, `DEBUG=1 ./build.sh`, `./build.sh demo`, `DEBUG=1 ./build.sh demo`, each exit 0 with every lane run (no `NO_LINT`, no `FAST`). The expect-fail lane must end `emp_expect_fail: OK`. Delete the copy.
- [ ] **Step 5: The strict landing run.** Count the tests added: `git diff 1e200e1b..HEAD -- crates | grep -c '^+#\[test\]'` (this plan adds 28; subtract any test Step 5 of a task deleted). Copy `scripts/landing-run.sh` to a run-unique path, make a fresh `cp -a` copy of the reference tree at `/home/volence/sonic_hacks/.a10-land-$$` (it must keep its git metadata: the pairing gate reads its HEAD), remove `__pycache__`, then detached, polling your own log for the verdict:

```bash
ORACLE_DIR=/home/volence/sonic_hacks/oracle-old <copy of landing-run.sh> --baseline 6186 --aeon /home/volence/sonic_hacks/.a10-land-$$
```

Expected: `RESULT GREEN`, `reconciles 6186 baseline + <added> new = <total> observed`, `CARGO_EXIT=0`, `CLIPPY_EXIT=0`, `LEDGER_EXIT=0`, zero `skip:` lines. No red is expected: the provenance tip (chain #212) already pins aeon `9b80db1d`, so `provenance_chain::aeon_dir_matches_the_provenance_tip` passes, and no byte moved, so there is nothing to refreeze or attest. If that pairing test is red, the copy's HEAD is not `9b80db1d`: fix the copy, do not touch `golden/provenance.toml`. Quote the whole RESULT block (suites, passed, failed, ignored, skips, binaries) in the note.
- [ ] **Step 6: Record and commit.** In the note: the seven BN rows, the three fixture lines, the four aeon build exits, the RESULT block, and one line per refusal naming its probe test, its control test and its RF transcript. Commit the note and `docs/QUEUE.md`.
- [ ] **Step 7: Stop.** The branch is ready. Merging to master and pushing are the overseer's and owner-gated; so is the card's Progress line in empyrean. Report: commits, aggregate totals, the `SHAPES 7/7 IDENTICAL` line, the RESULT block.

---

### Task 8 (aeon, optional by ruling): the pitfalls page

**Measured:** none of the 14 entries of `/home/volence/sonic_hacks/aeon/docs/EMP_PITFALLS.md` describes one of the six constructs, so no pitfall is retired and the card's third "Done means" line is met with no edit. One sentence becomes incomplete: §6 Trap B, line 203-204, "(`move.w sr, dN` is fine on 68000 supervisor and does not trip `[proc.sr-undeclared]` — that fires only with SR as destination.)".

**This is an aeon change on its own feature branch, never in this worktree, and not a pair:** sigil lands first and needs nothing from it. It moves aeon master off the provenance tip, so the next strict sigil run after it lands needs `refreeze --freeze NAME --ab REF --repin-aeon` (`docs/OVERSEER-REFERENCE.md`, "T-REPIN"). Do it only if the controller says so (Open question 6).

**Files:**
- Modify (aeon worktree `/home/volence/sonic_hacks/.aeon-a10`, branch `docs/a10-pitfalls` cut from aeon master): `docs/EMP_PITFALLS.md:203-204`

- [ ] **Step 1:** Extend the parenthesis to end: "— that fires only with SR as destination, and since sigil A10 a firing stops `./build.sh` (it stays a warning under `CONTRACTS=0` and for a single file)."
- [ ] **Step 2:** Commit that one file on the branch. Do not merge or push.

---

## Open questions

Each has a recommendation, and the tasks above already follow it.

| # | Question | Options | Recommendation | Cost if wrong |
|---|---|---|---|---|
| 1 | SR refusal: where does "the gate" promote it, and does the context form count? | (a) the gated `sigil build` refuses its own `proc.sr-undeclared` warnings after the link (Task 6), both emission sites; (b) a new family computed inside `run_contract_gate` from the corpus closure, before the build | **(a).** It is exactly "the warning becomes an error": the exposure number is the warn tally (0 in all 10 shapes), there is no second copy of the predicate to drift, and a context that leaves `sr` changed is the same defect. (b) would also fire in modules no shape places, which no build warns about today | (a) costs about one build before the refusal prints. Moving to (b) later is one task; the tests in Task 6 keep their assertions |
| 2 | Bare number: is a spliced integer (`move.w {n}, d0`) refused too? | refuse / leave | **Leave**, booked as `EMP-SPLICED-INT-OPERAND`. The card names a bare number in a line; a splice is how templates pass a value, and aeon's use of it was not counted | If the owner wants it refused: one task plus an aeon exposure count, possibly an aeon pair |
| 3 | Odd instruction: does `@as_compat` exempt it, as it exempts `[layout.odd-item]`? | exempt / refuse | **Refuse.** AS pads an instruction to even, so an odd one is never what the reference did. Aeon exposure 0 either way | An exemption is a one-line guard and one test |
| 4 | Duplicate const: does `const` against `equ` count? | `const`/`const` only; or any two of `const`/`equ` | **`const`/`const` only**, the form the card measured; `equ`/`equ` is already a link error; the mixed pair is booked as `EMP-CONST-EQU-SAME-NAME` | One small follow-up parcel |
| 5 | Binding: is a target declared NOWHERE refused too? | refuse / leave | **Leave**, booked as `EMP-BINDING-TARGET-UNDECLARED`. The card says "with no contract clause"; an undeclared target is a different hole and may be a legitimate cross-seam symbol | One follow-up task; no aeon exposure expected (all 8 targets are declared) |
| 6 | Is the aeon pitfalls sentence (Task 8) wanted now? | now / with the next aeon landing / never | **Write it on the branch, land it with the next aeon change that re-pins anyway.** No pitfall is retired, so the card does not require it | Landing it alone costs one `--repin-aeon` and an attest |
| 7 | Do the single-file paths get the five non-SR refusals? | all paths / build only | **All paths.** They are errors in the front end, so `sigil emp`, `sigil parse`, `sigil test` and `sigil build` all refuse; only the SR one is path-limited, by the card | None expected: narrowing one would need a path flag nobody asked for |
| 8 | AS front end | change / leave | **Leave for all six.** `move.w $1234,d0` on the AS route is already a memory read (`30 38 12 34`), AS pads odd instructions, and reassignment is AS semantics | None |
| 9 | Diagnostic wording and ids | as written / owner's wording | **As written** (`[operand.bare-number]`, `[layout.odd-instruction]`, `[proc.repeated-clause]`, `[name.duplicate-const]`, `[contract.binding-uncontracted]`, and the existing `[proc.sr-undeclared]`), each in an existing id family | A rename is a string and its test, per id; do it before landing, since aeon tools may come to match on them |
