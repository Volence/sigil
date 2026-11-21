---
kind: plan
status: awaiting-owner
description: "Empyrean v2 step 0, task A11: `.emp` can write every 68000 and Z80 instruction the shared encoder encodes (12 refused mnemonics, `move usp`, five SR/CCR forms, bare `exg`, `movem` from a PC-relative source, Z80 `(nn)` with a link-time symbol), proven by two tests that walk the encoder. Sigil only; aeon measured at 0 exposure in all 10 shapes."
---
# A11 Complete the instruction set Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **AEON PAIR: NOT NEEDED.** Measured for this plan on aeon `9b80db1d` with a scratch sigil carrying every change below: the 7 shipped shapes and the 3 fixture rows of both games are byte-identical to sigil `55c6ec32`, the warning tallies of the two release builds are unchanged (sonic4 `161 warnings`, same ids and counts), and aeon's expect-fail lane is 56/56. No aeon `.emp` file names a comptime function, label or statement after any of the 12 new words, and aeon uses none of the new forms (one `exg.l d4, d5`, already legal). Nothing in aeon changes.

> **SIZE, stated up front.** The card calls this encoder wiring, and for the bytes that is true: the shared encoder, the flag roles, the stack model for `link`/`unlk` and the return tables already know all 12 (the whole prototype is 149 added lines of non-test Rust in six files). Five things are bigger than "should be small": (1) the two walk tests (Task 1, about 380 lines, given whole); (2) the walk found a form the card does not list, `movem` from a PC-relative source, 4 instructions (Task 5); (3) three soundness holes that open the moment the words exist: `rtr` is not in the terminator set and the stack model does not know it pops a word, `stop` is invisible to the SR checks, and `usp` misuse must keep its named refusal (Tasks 3, 4, 7); (4) the Z80 `(nn)` fix needs a new operand kind that every Z80 analysis must be shown (Task 11); (5) the front-end differential and four other tests pin today's refusals and move with every task. **`sigil-isa` must change** in one file, `m68k_cycles.rs`, rows only (Task 10); no encoder byte changes anywhere.

**Goal:** Every 68000 and Z80 instruction the shared encoder can encode can be written in `.emp`, with the checks seeing what each one does, and no shape of any game moves a byte.

**Architecture:** The 12 names are added to the `.emp` mnemonic table and routed to encoder variants that already exist; `usp` becomes an operand kind; a Z80 `(Label)` becomes a link-time operand kind. Two tests in `sigil-harness` brute-force the shared encoder for everything it accepts and require `.emp` to assemble each one; they land first with a table of today's refusals, and each task deletes its rows. The four groups that touch a checker each land alone, in the narrowest form that is certainly right, and the rest is an Open question.

**Tech Stack:** Rust workspace pinned to 1.99.1 (`rust-toolchain.toml`); `sigil build --project <game dir> --shape <row>`; `scripts/step0_shapes_cmp.sh`; `scripts/landing-run.sh`.

**Spec:** `/home/volence/sonic_hacks/empyrean/docs/v2/tasks/A11 Complete the instruction set.md` (the owner-approved card; read it whole, it is short). Evidence: `/home/volence/sonic_hacks/empyrean/docs/v2/language/EMP language review A part 1.md` card 2a; `/home/volence/sonic_hacks/empyrean/docs/v2/language/6 Open questions for B.md` Q7 (the interrupt-mask model, open) and Q17 (is the refused set design cost or encoding work: this plan's measurement is the answer).

## Global Constraints

- Branch `parcel/a11-complete-instruction-set`, worktree `/home/volence/sonic_hacks/.sigil-a11`, cut from sigil master `55c6ec32`. Run `git branch --show-current` before every commit. Never commit to master, never touch `/home/volence/sonic_hacks/sigil` or another worktree.
- Line numbers in this plan are as of `55c6ec32`. Several tasks edit `lower/code.rs`, `lower/proc.rs` and `tests/support/frontend_diff.rs`, so numbers shift: **find by the quoted text, not the number.**
- **Not byte-changing.** Every task ends with protocol **BN** printing `SHAPES 7/7 IDENTICAL`. A byte that moves is STOP and BLOCKED.
- **`sigil-isa` may change in exactly one way:** new arms in `instr_cycles` (`crates/sigil-isa/src/m68k_cycles.rs`) and their tests, Task 10 only. No task changes `m68k.rs`, `m68k_decode.rs`, `z80.rs` or any backend's encoding. (A10 forbade touching `sigil-isa` at all; A11 cannot meet the card's cycle-table line without it.)
- The AS front end (`crates/sigil-frontend-as`) is not changed. It is the reference: where `.emp` gains a form, its size rule is the AS front end's (`m68k_default_size`, `m68k_suffix_refusal`, `refine_m68k_mnemonic` in `sigil-frontend-as/src/eval.rs`), which copies asl.
- Every new instruction gets: a golden vector (bytes equal on three sides: the literal, `sigil_isa::m68k::encode`, and both front ends), the deletion of its row in the Task 1 table, and a test of what the CHECKS say about it. A test that only asserts "no error" is not a test of a check.
- A new function is inserted BEFORE the doc comment of the item it precedes, never between a doc comment and its item. After each edit, read the three lines above the insertion.
- Every code block in this plan was compiled and run in a scratch clone at `55c6ec32` plus the changes of the earlier tasks. If one does not compile, the tree has moved: report it, do not improvise a different design.
- A test that breaks because it pinned an old refusal is updated to pin the new behaviour with the same strength (bytes, or the named diagnostic). Never delete an assertion to get to green; never edit a test to dodge a diagnostic.
- Not in this task (card, verbatim): "Deciding the interrupt-mask model (review B). 68020+ instructions." Also not here: a default size for `subx`/`negx`/`bchg` and the rest of `EMP-FRONTEND-DIFF-GAPS`; a symbolic 8-bit Z80 operand.
- Never build inside a live aeon checkout. Reference tree: `/home/volence/sonic_hacks/.aeon-a10-ref` (aeon `9b80db1d`, provisioned by A10). A build or landing run takes a `cp -a` copy of it placed outside every checkout, with `__pycache__` removed, deleted when done.
- Build with `CARGO_TARGET_DIR=.target-land` in the worktree. Tests need `AEON_DIR=/home/volence/sonic_hacks/.aeon-a10-ref ORACLE_DIR=/home/volence/sonic_hacks/oracle-old`.
- `df -h /` before a landing run or a multi-shape build; under 60 GB free, do not run it: BLOCKED.
- Before reporting a task: `CARGO_TARGET_DIR=.target-land cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- Never use an `mcp__oracle__*` tool. No subagents. Exact-path `git add`; `git show --stat` after each commit; the finding goes in the commit body; no co-author trailer. Never pass a commit message containing backticks through `-m "..."`: use `git commit -F <file>`.
- Doc sync: discovered work is booked in `docs/QUEUE.md` in the same change, and Task 12 checks every "Booked" name in this plan has a row. The empyrean checkout is read-only for implementers.

## Shared protocols

**BN (byte-neutral).** After building the candidate (`CARGO_TARGET_DIR=.target-land cargo build --release -p sigil-cli`):

```bash
scripts/step0_shapes_cmp.sh --base /home/volence/sonic_hacks/.a11-base-55c6ec32/sigil \
  --cand .target-land/release/sigil --aeon-ref /home/volence/sonic_hacks/.aeon-a10-ref
```

Expected last two lines: `SHAPES 7/7 IDENTICAL` and `STEP0_CMP_END rc=0`. Exit 1 = a byte moved (STOP). Exit 2 = could not run, never a pass.

**RF (red-first with the mutation shown).** The walk table gives every task its red for free, and it is recorded: (1) BEFORE implementing, delete the task's rows from `KNOWN_REFUSED` (or `SYM_REFUSED`) and add the task's golden and check tests; run them; paste the failing output (the walk prints the refused forms and their diagnostics) into the parcel note. (2) Implement; same commands PASS. (3) After the commit, `git status --short` clean, apply the task's named one-line mutation, show `git diff --stat`, run the named test: it must FAIL (still green = stale binary or wrong target dir); `git restore --source=HEAD -- <file>`, re-run: PASS. Paste (1) and (3) into `docs/superpowers/notes/2026-11-22-a11-complete-instruction-set.md`.

**TC (touched crate).** `AEON_DIR=/home/volence/sonic_hacks/.aeon-a10-ref ORACLE_DIR=/home/volence/sonic_hacks/oracle-old CARGO_TARGET_DIR=.target-land cargo test --release -p <crate> --no-fail-fast`, quoting the sum of every `test result:` line, never a tail. Tasks 2 to 9 run TC for `sigil-frontend-emp`, then `-p sigil-cli --test frontend_differential` (90 s) and `-p sigil-harness --test emp_isa_walk_m68k --test emp_a11_golden --test frontend_differential_coverage`.

**FD (differential tables).** `crates/sigil-cli/tests/support/frontend_diff.rs` holds the list of forms `.emp` refuses and the AS route assembles. Three of its tests move with this plan: `emp_and_as_front_ends_agree_on_every_generated_m68k_case` (a new disagreement fails it), `every_exclusion_is_used` (a row nothing matches any more fails it) and `every_family_with_a_legal_emp_form_compared_bytes` (a family listed as having no `.emp` form that now assembles fails it). Each task names its FD edit exactly; make no other. Only the FINAL state of these tables was run for this plan (green, 7 tests, 90 s). If an intermediate task's differential prints a disagreement its FD step did not predict, read it before touching the table: it is either a form a later task closes (then say so in the note and carry a temporary named row, deleted by that task) or a finding to report.

## Review Focus

1. **A word that used to be free is now a mnemonic.** "Mnemonics win": after this plan a comptime function named `link`, `stop`, `reset`, `chk` (or any of the 12) is refused by `[name.shadows-mnemonic]`, and a statement line starting with one is an instruction. Aeon has none (measured); fields, locals and labels named `link`/`stop`/`reset` are unaffected. Test in Task 9 (`a_const_or_label_named_after_a_new_mnemonic_is_still_a_name`).
2. **`rtr` on correct code.** `move.w sr, -(sp)` ... `rtr` is the idiom; today's stack model would call it 2 bytes unbalanced. Test in Task 3.
3. **`stop` under `preserves(sr)`.** It writes SR with no `sr` operand, so every operand-shaped SR check misses it. Test in Task 7.
4. **`usp` anywhere but the two moves** (`lea usp, a0`, `move.l usp, d0`, `move.w a0, usp`) must stay a named refusal and never become a symbol read; a `proc usp` beside `move.l a2, usp` once assembled `21CA 0000`. Tests in Task 4.
5. **A Z80 `(Label)` that is not a 16-bit address field:** `in a, (Label)` / `out (Label), a` (8-bit port) must refuse with a diagnostic, not truncate; a `(Label)` carrying a bank mask must refuse. Tests in Task 11.

## Baseline measured for this plan (2026-11-21)

Sigil `55c6ec32` (release build; master is `10dcf37a`, which adds one plan document and no code), aeon `9b80db1d`. "Prototype" is a scratch clone of the branch carrying everything below; it was deleted after measuring.

**What `.emp` refuses today.** The Task 1 walk on the unmodified tree: the encoder accepts 2439 (mnemonic, size, operands) forms, which decode to 1527 distinct instructions in all 85 families of `ALL_FAMILY_NAMES`; `.emp` assembles 1447 and refuses 80, in 20 families:

| Family | Refused | `.emp` says | Card lists it |
|---|---|---|---|
| `abcd` 2, `sbcd` 2, `nbcd` 8, `negx` 24, `subx` 6, `chk` 11, `link` 1, `unlk` 1, `rtr` 1, `trapv` 1, `stop` 1, `reset` 1 | 59 | `` `x` is not a recognized 68000 mnemonic `` | yes, all 12 |
| `move-to-usp` 1, `move-from-usp` 1 | 2 | `[operand.usp-unsupported]` | yes |
| `andi-sr` 1, `ori-sr` 1, `eori-sr` 1, `eori-ccr` 1, `move-to-ccr` 11 | 15 | `unsupported form: sr\|ccr is not a general EA` | yes, the five forms |
| `movem` (`.w`/`.l`, source `(d16,PC)` or `(d8,PC,Xn)`) | 4 | `internal: a PC-relative operand reached the generic operand mapper` | **no** |

The card's list is exact; the one extra is `movem` from a PC-relative source. `exg`: all three register pairings assemble with `.l`; only the bare spelling is refused (`instruction needs an explicit size suffix`), and `exg` has no cycle row. Unsized spellings `.emp` refuses and the AS route defaults (`addx`, `bchg`, memory shifts, `move` to sr/ccr) are the existing `emp-requires-a-size-suffix` row, not refused instructions, and stay.

**Z80.** 538 distinct encodings from 67 of the 69 `Mnemonic` variants; `.emp` assembles all 538. The two variants `Mnemonic::LdIA`/`LdRA` are never accepted by `z80::encode` (`ld i, a` / `ld r, a` go through `Mnemonic::Ld`). With every 16-bit field replaced by a link-time symbol: the 24 immediate forms (`ld rr, Sym`, `jp`/`call [cc,] Sym`) assemble; all 28 `ld` forms with `(nn)` are refused (`ld a,(nn)`, `ld (nn),a`, `ld hl/bc/de/sp/ix/iy,(nn)` and the reverse, two pool addresses each), with `unknown name` then `[lower.z80-unsupported] a Z80 indirect base must be ... got poison` (or `got link-expr` for `extern("Foo")`); `in a,(Sym)` and `out (Sym),a` are refused (the port is one byte). Cause: `map_ind_z80` (`eval/asm.rs`, about `:2660`) evaluates the base outside the link context and accepts only a stored integer.

**The golden vectors** (prototype `.emp`, the AS route at `55c6ec32`, and `m68k::encode` agree on every row; Task 2's `golden` helper checks all three):

| Line | Bytes | Line | Bytes |
|---|---|---|---|
| `abcd d0, d1` | `C3 00` | `link a6, #-8` | `4E 56 FF F8` |
| `abcd -(a0), -(a1)` | `C3 08` | `unlk a6` | `4E 5E` |
| `sbcd d0, d1` | `83 00` | `rtr` | `4E 77` |
| `nbcd d0` / `nbcd (a0)` | `48 00` / `48 10` | `trapv` | `4E 76` |
| `negx.w d0` / `negx.b (a0)` | `40 40` / `40 10` | `reset` | `4E 70` |
| `subx.w d0, d1` | `93 40` | `stop #$2700` | `4E 72 27 00` |
| `subx.l -(a0), -(a1)` | `93 88` | `chk.w d0, d1` / `chk.w (a0), d1` | `43 80` / `43 90` |
| `exg d0, d1` / `exg a0, a1` / `exg d0, a1` | `C1 41` / `C1 49` / `C1 89` | `move.l a0, usp` / `move.l usp, a0` | `4E 60` / `4E 68` |
| `ori.w #$0700, sr` | `00 7C 07 00` | `move.w d0, ccr` | `44 C0` |
| `andi.w #$F8FF, sr` | `02 7C F8 FF` | `move.w #$1F, ccr` | `44 FC 00 1F` |
| `eori.w #$0700, sr` | `0A 7C 07 00` | `eori.b #$01, ccr` | `0A 3C 00 01` |
| `movem.w .tgt(pc), d0-d1` (`.tgt` 4 bytes back) | `4C BA 00 03 FF FA` | Z80 `ld a, (q)`, `q` at 4 | `3A 04 00` |

**What the checks do with the new words once they assemble** (prototype, wiring only, before the fixes of Tasks 3, 4, 7):

| Probe | Result | Verdict |
|---|---|---|
| `abcd d0, d1` in `clobbers()` | `[proc.clobber-undeclared]` names `d1` | right, no work |
| `link a6, #-8` / `unlk a6` / `rts` | balanced; `link` alone: `[stack.unbalanced] ... 12 bytes`; `a6` charged at `unlk` only | `link` must charge `a6` too (Task 6) |
| `move.w sr, -(sp)` / `rtr` | `[stack.unbalanced] ... 2 bytes` (error) and `[proc.undeclared-fallthrough]` | **wrong on correct code** (Task 3) |
| `ori`/`andi`/`eori #, sr` in `clobbers()` | `[proc.sr-undeclared]` each | right, no work |
| `stop #$2000` in `clobbers()` | nothing | **hole** (Task 7) |
| `stop #$2700` in `preserves(sr)` | builds, advisory only | **hole** (Task 7) |
| `move.l usp, d0`, `lea usp, a0`, `move.w a0, usp` | refused, but by three unnamed encoder messages | keep the id (Task 4) |
| `stop.w #0`, `unlk.l a6`, `rtr.w`, bare `chk d0, d1` | assemble; the AS route refuses each, as asl does | refuse (Tasks 3, 6 to 9) |

**Sigil's own tests that pin a refusal this plan removes** (measured two ways. The full strict workspace suite on a prototype carrying the wiring only: 565 suites, 6207 passed, 10 failed, 4 ignored; nine failures are the tests below and the tenth was the prototype's own scratch walk file tripping `skip_marker_lint` with an `eprintln!` the Task 1 files do not have (they pass that lint). Then, on the prototype carrying everything in this plan: all of `sigil-frontend-emp` and `sigil-isa` (167 suites, 3029 tests, the same six failures below and no other; both backends were green on the prototype one step earlier), the differential (7 of 7 green once its tables are edited as the tasks say), its coverage test, and every test file this plan adds. What was NOT re-run on the final prototype is the rest of the workspace (`sigil-cli`'s build tests, the harness corpus tests), so Task 12's landing run is the complete answer):

| Test | Why it moves | Task |
|---|---|---|
| `frontend_differential`: the three FD tests | rows `emp-lacks-mnemonic`, `emp-lacks-sr-ccr-form`, `emp-lacks-usp` | 2 to 9 |
| `as_author_forms_stay_off_emp.rs::the_other_as_rewrites_do_not_reach_emp` | pins `andi.w #,sr`, `eori.b #,ccr`, and six of the mnemonics as refused | 3, 5 to 9 |
| `usp_reserved.rs`: 3 of 6 tests | expect `[operand.usp-unsupported]` on the two legal moves | 4 |
| `sigil-frontend-emp` unit `m68k_cycles::tests::off_table_forms_refuse`, `cycle_budget::tests::an_off_table_68k_op_is_unknown` | use `link` as the example of an unpriced instruction | 10 |
| `flag_check.rs` `UNSPELLABLE` (unit-test const, 9 families) | not a failure: a skip list that would silently keep skipping | 2 to 9 |

**Aeon exposure.** 207 tracked `.emp` files: no comptime function, label or statement line begins with any of the 12 words; `link`, `stop`, `reset` appear only as field names, locals and comments. Base against prototype, `sigil build --project <copy>/games/<g> --shape <row>`: `SHAPES 7/7 IDENTICAL` (`plain 142a39b9/850262`, `debug 20acdb67/870606`, `demo 4587df52/99980`, `demo_debug 37d66535/106981`, `config_a 2ce7438f/870972`, `config_b 128361b3/617715`, `lean f6b07847/798198`) and `FIXTURE config-a-keystones-flipped|stress-evict|stress-art SAME`. Expect-fail lane with the prototype: `emp_expect_fail: OK — 56/56 cases (54 comptime + 2 link)`.

**Test count.** A10's landing run at `55c6ec32` reported `563 suites / 6217 passed / 0 failed / 4 ignored`; master has added no code since. This plan's own plain `cargo test --workspace --no-fail-fast` on the wiring prototype counted 565 suites and 6217 tests INCLUDING its 2 scratch tests, so 6215 from the tree: the two counts differ by 2 and this plan did not chase why. Task 12 therefore takes the baseline from the landing script's own record on master at the time, not from this paragraph.

## Order

| Order | Task | Size | Group | Files the change lives in | Depends on |
|---|---|---|---|---|---|
| 0 | Base binary, parcel note | S | none | none | none |
| 1 | The two walk tests, with today's refusals as a table | M | proof | `sigil-harness/tests` (new) | 0 |
| 2 | `abcd` `sbcd` `nbcd` `negx` `subx`, bare `exg` | S | wiring | `lower/code.rs` | 1 |
| 3 | `rtr` | S | wiring | `lower/code.rs`, `lower/proc.rs`, `preserves.rs` | 2 |
| 4 | `move` to and from `usp` | M | wiring | `value.rs`, `eval/asm.rs`, `lower/code.rs` | 3 |
| 5 | `move <ea>, ccr`, `eori #, ccr`, `movem` from a PC-relative source | S | wiring | `lower/code.rs` | 4 |
| 6 | `link` / `unlk` (decision) | S | stack balance | `lower/code.rs`, `lower/proc.rs` | 5 |
| 7 | `stop` and the SR immediates (decision) | M | mask | `lower/code.rs`, `lower/proc.rs`, `lower/mod.rs`, `corpus_contracts.rs` | 6 |
| 8 | `reset` (decision) | S | reset | `lower/code.rs` | 7 |
| 9 | `chk` / `trapv` (decision) | S | exceptions | `lower/code.rs` | 8 |
| 10 | Cycle rows (`sigil-isa`) | M | cycles | `sigil-isa/src/m68k_cycles.rs` | 9 |
| 11 | Z80 `(nn)` takes a link-time symbol | M | Z80 | `value.rs`, `eval/asm.rs`, `lower/code.rs`, `z80_cycles.rs`, `flag_check.rs` | 1 |
| 12 | All shapes identical, strict landing run | M (wall clock) | proof | none | 1 to 11 |

Tasks 2 to 9 are a chain because they share `lower/code.rs`, the differential table and two test files; each is one dispatch. Task 11 touches different functions of the same files and needs only Task 1, but there is one worktree: run it in sequence, after Task 10.

---

### Task 0: Base binary, parcel note

**Files:**
- Create: `docs/superpowers/notes/2026-11-22-a11-complete-instruction-set.md`

**Interfaces:**
- Produces: `/home/volence/sonic_hacks/.a11-base-55c6ec32/sigil` (the base for BN) and the parcel note every later task appends its RF transcript to.

- [ ] **Step 1: Confirm the branch holds no code change.** `git diff --stat 55c6ec32 HEAD -- crates scripts contract Cargo.toml Cargo.lock` prints nothing.
- [ ] **Step 2: Build and park the base.** `df -h /`, then `CARGO_TARGET_DIR=.target-land cargo build --release --locked -p sigil-cli -p sigil-harness --bins`; `mkdir /home/volence/sonic_hacks/.a11-base-55c6ec32 && cp .target-land/release/sigil .target-land/release/emit_sound_blob /home/volence/sonic_hacks/.a11-base-55c6ec32/`; record `sha256sum` of both in the note.
- [ ] **Step 3: Check the reference tree is the one A10 provisioned.** `git -C /home/volence/sonic_hacks/.aeon-a10-ref rev-parse --short HEAD` prints `9b80db1d`, and its four ROMs are `s4.bin 142a39b9/850262`, `s4.debug.bin 20acdb67/870606`, `demo.bin 4587df52/99980`, `demo.debug.bin 37d66535/106981` (zlib CRC-32 / size). If the tree is missing or differs, provision it as A10's Task 0 Step 3 did (`REF_BUILD_DEMO=1 CARGO_TARGET_DIR=.target-land scripts/provision-aeon-ref.sh <dir>`), into a new directory, and use that path wherever this plan names `.aeon-a10-ref`.
- [ ] **Step 4: Prove BN on itself.** Run BN with `--cand /home/volence/sonic_hacks/.a11-base-55c6ec32/sigil`. Expected `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`. Paste the seven `SHAPE` rows into the note.
- [ ] **Step 5: Write the note** with the two sha256 lines, the seven rows, this plan's "Baseline measured" tables copied in, and an empty `## RF transcripts` heading. Commit it.

---

### Task 1: The two walk tests, with today's refusals as a table

**Why first:** these are the card's "a test walks the shared instruction table and fails if `.emp` refuses any 68000 instruction; the same check for Z80". Landing them first, green, with today's 80 refusals written down as a table, turns every later task into "delete my rows, watch it go red, make it green".

**Design, so a reviewer can judge its honesty.**
- **What is walked.** Not `ALL_FAMILY_NAMES` alone (85 names say nothing about operands) and not a hand list of forms (that is the thing that drifts). The demand is the ENCODER'S OWN ANSWER: every `Mnemonic` variant (a `match` with no wildcard arm, so a new variant does not compile until it is listed; `Scc`/`Bcc`/`Dbcc` take all 16 conditions) x 4 sizes x every 0-, 1- and 2-operand combination of a 17-operand pool (one of each `Operand` variant) is offered to `m68k::encode`. Whatever it accepts is an instruction: 2439 forms.
- **What counts as one instruction.** The bytes are read back with `m68k_decode::decode_exact` and canonicalized; two accepted forms that decode to the same thing (modulo a link-resolved displacement) are one instruction: 1527.
- **How `.emp` is asked.** Each instruction is spelled from its DECODED form, with a size suffix and again with none, inside `module m / @allow("stack.unbalanced") / proc f() { .tgt: nop / <line> / rts }` (the allow is for `link` alone and `-(a7)` forms; the frame's `nop` and `rts` prove the line is where the bytes came from).
- **What "refused" means.** Neither spelling yields bytes that decode to the same instruction. A diagnostic and "assembled something else" both count.
- **What keeps it honest.** (a) `the_walk_demands_every_family_of_the_shared_table`: every name in `ALL_FAMILY_NAMES` has at least one accepted form, nothing the encoder emits is undecodable, and the counts have a measured floor (a pool that shrinks fails instead of passing quietly). (b) `the_walk_can_see_a_refusal`: the oracle is shown a real refusal and a wrong-instruction case and must report both. (c) `KNOWN_REFUSED` is compared for EQUALITY with what was refused, per family with counts: a new refusal fails, and a row that is no longer true fails.
- **`sigil-isa` is not changed for this.** The mnemonic list is locked by the test's own macro, the precedent `z80_cycles.rs` `encoder_coverage` sets. `sigil-harness` already depends on `sigil-isa` directly.
- **Z80** is the same shape, simpler: distinct BYTES are distinct instructions (538), the `.emp` line must give exactly those bytes, and a second pass replaces every 16-bit field (`Mem`, `Imm16`) with a link-time symbol defined to the same value.

**Files:**
- Create: `crates/sigil-harness/tests/emp_isa_walk_m68k.rs`, `crates/sigil-harness/tests/emp_isa_walk_z80.rs`
- Modify: `docs/QUEUE.md` (book `ISA-Z80-DEAD-MNEMONICS`: `Mnemonic::LdIA`/`LdRA` are variants `z80::encode` never accepts; and `EMP-Z80-SYMBOLIC-BYTE-OPERAND`: `in a,(Sym)`, `out (Sym),a`, `ld a, Sym`, `cp Sym`, `rst Sym` refuse a link-time symbol)

**Interfaces:**
- Consumes: `frontend_diff::emp_assemble(&str) -> Result<Vec<u8>, Vec<String>>` and `frontend_diff::mnemonic(&str) -> &str` from `crates/sigil-cli/tests/support/frontend_diff.rs` (included by `#[path]`; it needs `m68k` bound at the including crate root, which `use sigil_isa::m68k::{self, ...}` does).
- Produces: `KNOWN_REFUSED: &[(&str, usize)]` (68000, keyed by family name) and `SYM_REFUSED: &[(&str, usize)]` (Z80, keyed `"<mnemonic> <mem|imm16>"`). Tasks 2 to 9 and 11 delete rows from them.

- [ ] **Step 1: Create `crates/sigil-harness/tests/emp_isa_walk_m68k.rs`** with exactly this content:

```rust
//! A11: `.emp` can write every 68000 instruction the shared encoder can encode.
//!
//! DEMAND is brute force over the shared encoder, not a hand list: every
//! `sigil_isa::m68k::Mnemonic` (a `match` with no wildcard arm locks the list to
//! the enum) x four sizes x every 0-, 1- and 2-operand shape of [`pool`] goes
//! through `m68k::encode`; whatever it accepts is an instruction. Two accepted
//! forms are the same instruction when `m68k_decode::decode_exact` reads their
//! bytes back to the same thing, modulo a link-resolved displacement.
//!
//! SUPPLY is the `.emp` front end: each demanded instruction is spelled from its
//! DECODED form (sized, then with no suffix) and assembled. REFUSED means neither
//! spelling produces bytes that decode to the same instruction: a diagnostic, or
//! different bytes, both count.
use sigil_isa::m68k::{self, encode, family_name, Cond, Instruction, Mnemonic, Operand, Size, Xn, ALL_FAMILY_NAMES};
use sigil_isa::m68k_decode::{canonicalize, decode_exact};
use std::collections::{BTreeMap, BTreeSet};

#[path = "../../sigil-cli/tests/support/frontend_diff.rs"]
mod frontend_diff;

const CONDS: [Cond; 16] = [
    Cond::T, Cond::F, Cond::Hi, Cond::Ls, Cond::Cc, Cond::Cs, Cond::Ne, Cond::Eq, Cond::Vc,
    Cond::Vs, Cond::Pl, Cond::Mi, Cond::Ge, Cond::Lt, Cond::Gt, Cond::Le,
];
fn cond_name(c: Cond) -> &'static str {
    match c {
        Cond::T => "t", Cond::F => "f", Cond::Hi => "hi", Cond::Ls => "ls", Cond::Cc => "cc",
        Cond::Cs => "cs", Cond::Ne => "ne", Cond::Eq => "eq", Cond::Vc => "vc", Cond::Vs => "vs",
        Cond::Pl => "pl", Cond::Mi => "mi", Cond::Ge => "ge", Cond::Lt => "lt", Cond::Gt => "gt",
        Cond::Le => "le",
    }
}

macro_rules! unit_mnemonics {
    ($($v:ident),* $(,)?) => {
        #[allow(dead_code)]
        fn locked(m: Mnemonic) {
            match m { $(Mnemonic::$v => {}),* Mnemonic::Scc(_) | Mnemonic::Bcc(_) | Mnemonic::Dbcc(_) => {} }
        }
        fn all_mnemonics() -> Vec<Mnemonic> {
            let mut v = vec![$(Mnemonic::$v),*];
            for c in CONDS { v.push(Mnemonic::Scc(c)); v.push(Mnemonic::Bcc(c)); v.push(Mnemonic::Dbcc(c)); }
            v
        }
    };
}
unit_mnemonics! {
    Move, Movea, Add, Adda, Sub, Suba, And, Or, Eor, Cmp, Cmpa, Muls, Mulu, Divs, Divu,
    Addi, Subi, Andi, Ori, Eori, Cmpi, Moveq, Addq, Subq, Asl, Asr, Lsl, Lsr, Rol, Ror, Roxl, Roxr,
    Btst, Bset, Bclr, Bchg, Clr, Neg, Not, Tst, Tas, Jmp, Jsr, Lea, Pea, Nop, Rts, Rte, Trap, Swap,
    Ext, Illegal, Bra, Bsr, Movem, Movep, Addx, Cmpm, Subx, Abcd, Sbcd, Negx, Nbcd, MoveToSr,
    MoveFromSr, MoveToCcr, AndiCcr, OriCcr, EoriCcr, AndiSr, OriSr, EoriSr, MoveToUsp, MoveFromUsp,
    Exg, Chk, Link, Unlk, Reset, Rtr, Trapv, Stop,
}

fn pool() -> Vec<Operand> {
    use Operand::*;
    vec![
        Dn(1), An(2), Ind(3), PostInc(4), PreDec(5), Disp16An(8, 1),
        Disp8AnXn { d: 4, an: 2, xn: Xn::D(3), long: false },
        Pcd8Xn { d: -4, xn: Xn::D(0), long: false },
        AbsW(0x1234), AbsL(0x00FF_8000), Pcd16(-4), Imm(1), RegList(0x0301), Disp(-4), Ccr, Sr, Usp,
    ]
}

fn mnem_text(m: Mnemonic) -> String {
    match m {
        Mnemonic::Scc(c) => format!("s{}", cond_name(c)),
        Mnemonic::Bcc(c) => format!("b{}", cond_name(c)),
        Mnemonic::Dbcc(c) => format!("db{}", cond_name(c)),
        other => frontend_diff::mnemonic(family_name(other)).to_string(),
    }
}

fn op_text(o: &Operand) -> String {
    match *o {
        Operand::Dn(n) => format!("d{n}"),
        Operand::An(n) => format!("a{n}"),
        Operand::Ind(n) => format!("(a{n})"),
        Operand::PostInc(n) => format!("(a{n})+"),
        Operand::PreDec(n) => format!("-(a{n})"),
        Operand::Disp16An(d, n) => format!("{d}(a{n})"),
        Operand::Disp8AnXn { d, an, xn, long } => format!("{d}(a{an},{}.{})", xn_text(xn), if long { 'l' } else { 'w' }),
        Operand::Pcd8Xn { xn, long, .. } => format!(".tgt(pc,{}.{})", xn_text(xn), if long { 'l' } else { 'w' }),
        Operand::AbsW(w) if w < 0 => format!("(${:08X}).w", w as i32 as u32),
        Operand::AbsW(w) => format!("(${w:04X}).w"),
        Operand::AbsL(a) => format!("(${:08X}).l", a as u32),
        Operand::Pcd16(_) => ".tgt(pc)".into(),
        Operand::Imm(v) => format!("#{v}"),
        Operand::RegList(m) => reglist(m),
        Operand::Disp(_) => ".tgt".into(),
        Operand::Ccr => "ccr".into(),
        Operand::Sr => "sr".into(),
        Operand::Usp => "usp".into(),
    }
}
fn xn_text(x: Xn) -> String {
    match x { Xn::D(n) => format!("d{n}"), Xn::A(n) => format!("a{n}") }
}
fn reglist(mask: u16) -> String {
    let mut p = Vec::new();
    for i in 0..8 { if mask & (1 << i) != 0 { p.push(format!("d{i}")); } }
    for i in 0..8 { if mask & (1 << (8 + i)) != 0 { p.push(format!("a{i}")); } }
    p.join("/")
}

fn size_char(s: Size) -> char {
    match s { Size::B => 'b', Size::W => 'w', Size::L => 'l', Size::S => 's' }
}

/// The instruction with every link-resolved displacement zeroed.
fn modulo_disp(i: &Instruction) -> Instruction {
    let mut c = canonicalize(i);
    for o in &mut c.ops {
        match o {
            Operand::Pcd16(d) => *d = 0,
            Operand::Pcd8Xn { d, .. } => *d = 0,
            Operand::Disp(d) => *d = 0,
            _ => {}
        }
    }
    c
}

fn emp_src(line: &str) -> String {
    format!("module m\n@allow(\"stack.unbalanced\")\nproc f() {{\n.tgt:\n    nop\n    {line}\n    rts\n}}\n")
}

/// One demanded instruction: the form first given to the encoder, and what its bytes decode to.
struct Demand {
    by_decode: BTreeMap<String, Instruction>,
    families: BTreeSet<&'static str>,
    accepted: usize,
    undecodable: Vec<String>,
}

fn demand() -> Demand {
    let pool = pool();
    let mut shapes: Vec<Vec<Operand>> = vec![vec![]];
    for a in &pool {
        shapes.push(vec![*a]);
    }
    for a in &pool {
        for b in &pool {
            shapes.push(vec![*a, *b]);
        }
    }
    let mut out = Demand { by_decode: BTreeMap::new(), families: BTreeSet::new(), accepted: 0, undecodable: Vec::new() };
    for m in all_mnemonics() {
        for size in [Size::B, Size::W, Size::L, Size::S] {
            for ops in &shapes {
                let inst = Instruction { mnemonic: m, size, ops: ops.clone() };
                let Ok(bytes) = encode(&inst) else { continue };
                out.accepted += 1;
                out.families.insert(family_name(m));
                match decode_exact(&bytes) {
                    Ok(d) => {
                        out.by_decode.entry(format!("{:?}", modulo_disp(&d))).or_insert(d);
                    }
                    Err(e) => out.undecodable.push(format!("{inst:?}: {e:?}")),
                }
            }
        }
    }
    out
}

/// Assemble `line` inside the frame and require the bytes between the frame's
/// `nop` and `rts` to decode to `want` (modulo displacement).
fn emp_writes(line: &str, want: &Instruction) -> Result<(), String> {
    let bytes = frontend_diff::emp_assemble(&emp_src(line)).map_err(|e| e.join(" | "))?;
    if bytes.len() < 6 || bytes[..2] != [0x4E, 0x71] || bytes[bytes.len() - 2..] != [0x4E, 0x75] {
        return Err(format!("frame lost: {bytes:02X?}"));
    }
    let mid = &bytes[2..bytes.len() - 2];
    let got = decode_exact(mid).map_err(|e| format!("emp bytes {mid:02X?} do not decode: {e:?}"))?;
    if modulo_disp(&got) == modulo_disp(want) {
        Ok(())
    } else {
        Err(format!("DIFFERENT INSTRUCTION {mid:02X?} = {got:?}"))
    }
}

/// Families `.emp` still refuses, with the number of refused instructions in
/// each. A11 empties this table one task at a time; a row whose count is no
/// longer true fails the walk, so a closed gap cannot keep its row.
const KNOWN_REFUSED: &[(&str, usize)] = &[
    ("abcd", 2), ("sbcd", 2), ("nbcd", 8), ("negx", 24), ("subx", 6), ("rtr", 1),
    ("move-to-usp", 1), ("move-from-usp", 1), ("move-to-ccr", 11), ("eori-ccr", 1), ("movem", 4),
    ("link", 1), ("unlk", 1),
    ("stop", 1), ("andi-sr", 1), ("ori-sr", 1), ("eori-sr", 1),
    ("reset", 1),
    ("chk", 11), ("trapv", 1),
];

/// The floor under the demand, measured at sigil `55c6ec32`. A pool or mnemonic
/// list that shrinks fails here instead of passing with less work done.
const MIN_ACCEPTED: usize = 2439;
const MIN_DISTINCT: usize = 1527;

#[test]
fn the_walk_demands_every_family_of_the_shared_table() {
    let d = demand();
    assert!(d.undecodable.is_empty(), "encoder output the decoder cannot read: {:#?}", d.undecodable);
    let missing: Vec<&str> = ALL_FAMILY_NAMES.iter().copied().filter(|f| !d.families.contains(f)).collect();
    assert!(missing.is_empty(), "families of ALL_FAMILY_NAMES with no encodable form in the pool: {missing:?}");
    assert!(d.accepted >= MIN_ACCEPTED, "encoder accepted {} forms, floor {MIN_ACCEPTED}", d.accepted);
    assert!(d.by_decode.len() >= MIN_DISTINCT, "{} distinct instructions, floor {MIN_DISTINCT}", d.by_decode.len());
}

#[test]
fn emp_assembles_every_68000_instruction_the_shared_encoder_accepts() {
    let d = demand();
    let mut refused: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for inst in d.by_decode.values() {
        let mn = mnem_text(inst.mnemonic);
        let ops = inst.ops.iter().map(op_text).collect::<Vec<_>>().join(", ");
        let sized = format!("{mn}.{} {ops}", size_char(inst.size));
        let bare = format!("{mn} {ops}");
        if let (Err(a), Err(b)) = (emp_writes(&sized, inst), emp_writes(&bare, inst)) {
            refused.entry(family_name(inst.mnemonic)).or_default().push(format!("`{sized}`: {a}  ||  `{bare}`: {b}"));
        }
    }
    let got: BTreeMap<&str, usize> = refused.iter().map(|(f, rows)| (*f, rows.len())).collect();
    let want: BTreeMap<&str, usize> = KNOWN_REFUSED.iter().copied().collect();
    assert_eq!(got, want, "refused instructions per family differ from KNOWN_REFUSED:\n{refused:#?}");
}

#[test]
fn the_walk_can_see_a_refusal() {
    let nop = Instruction { mnemonic: Mnemonic::Nop, size: Size::W, ops: vec![] };
    let rts = Instruction { mnemonic: Mnemonic::Rts, size: Size::W, ops: vec![] };
    assert_eq!(emp_writes("nop", &nop), Ok(()));
    let unknown = emp_writes("frobnicate d0", &nop).unwrap_err();
    assert!(unknown.contains("not a recognized 68000 mnemonic"), "{unknown}");
    let wrong = emp_writes("nop", &rts).unwrap_err();
    assert!(wrong.starts_with("DIFFERENT INSTRUCTION"), "{wrong}");
}
```

- [ ] **Step 2: Create `crates/sigil-harness/tests/emp_isa_walk_z80.rs`** with exactly this content:

```rust
//! A11: `.emp` can write every Z80 instruction the shared encoder can encode, and
//! every 16-bit field of one can be a link-time symbol.
//!
//! DEMAND is brute force over `sigil_isa::z80::encode`: every `Mnemonic` (the
//! `mnemonics!` match has no wildcard arm, so the list is locked to the enum) x
//! every 0-, 1- and 2-operand shape of [`pool`]. Distinct BYTES are distinct
//! instructions. SUPPLY: the same instruction spelled for `.emp` must assemble
//! to exactly those bytes. REFUSED is a diagnostic or any other bytes.
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_isa::z80::{encode, Cond, IndexReg, Instruction, Mnemonic, Operand, Reg16, Reg8};
use sigil_span::Level;
use std::collections::BTreeMap;

macro_rules! mnemonics {
    ($($v:ident => $s:expr),* $(,)?) => {
        #[allow(dead_code)]
        fn spelling(m: Mnemonic) -> &'static str { match m { $(Mnemonic::$v => $s),* } }
        fn all_mnemonics() -> Vec<(Mnemonic, &'static str)> { vec![$((Mnemonic::$v, $s)),*] }
    };
}
mnemonics! {
    Nop => "nop", Ld => "ld", Add => "add", Adc => "adc", Sub => "sub",
    Sbc => "sbc", And => "and", Or => "or", Xor => "xor", Cp => "cp",
    Inc => "inc", Dec => "dec", Push => "push", Pop => "pop", Ex => "ex",
    Exx => "exx", Ret => "ret", Jr => "jr", Jp => "jp", Call => "call",
    Djnz => "djnz", Rrca => "rrca", Scf => "scf", Ei => "ei", Di => "di",
    Bit => "bit", Res => "res", Set => "set", Srl => "srl", Rr => "rr",
    Sla => "sla", Rlc => "rlc", Rrc => "rrc", Rl => "rl", Sra => "sra",
    Neg => "neg", Im => "im",
    Ldi => "ldi", Ldir => "ldir", Ldd => "ldd", Lddr => "lddr",
    Cpi => "cpi", Cpd => "cpd", Cpir => "cpir", Cpdr => "cpdr",
    Ini => "ini", Ind => "ind", Inir => "inir", Indr => "indr",
    Outi => "outi", Outd => "outd", Otir => "otir", Otdr => "otdr",
    In => "in", Out => "out", Reti => "reti", Retn => "retn",
    Rrd => "rrd", Rld => "rld",
    LdIA => "ld", LdRA => "ld",
    Rlca => "rlca", Rla => "rla", Rra => "rra", Daa => "daa",
    Cpl => "cpl", Ccf => "ccf", Halt => "halt", Rst => "rst",
}

fn pool() -> Vec<Operand> {
    use Operand::*;
    let mut v = vec![];
    for r in [Reg8::A, Reg8::B, Reg8::C, Reg8::D, Reg8::E, Reg8::H, Reg8::L] { v.push(Reg(r)); }
    for p in [Reg16::Bc, Reg16::De, Reg16::Hl, Reg16::Sp, Reg16::Af, Reg16::Ix, Reg16::Iy] { v.push(Pair(p)); }
    v.extend([IndHl, IndBc, IndDe, Indexed { reg: IndexReg::Ix, disp: 5 }, Indexed { reg: IndexReg::Iy, disp: -1 }]);
    v.extend([Imm8(0x12), Imm8(0), Imm8(1), Imm8(2), Imm8(0x18), Imm16(0x1234), Mem(0x4000), Mem(0x00FE)]);
    for c in [Cond::Nz, Cond::Z, Cond::Nc, Cond::C, Cond::Po, Cond::Pe, Cond::P, Cond::M] { v.push(Cc(c)); }
    v.extend([Bit(3), Rel(-3), AfShadow, RegI, RegR, IndC]);
    v
}

fn op_text(m: Mnemonic, o: &Operand, sym: Option<&str>) -> String {
    match *o {
        Operand::Reg(r) => format!("{r:?}").to_lowercase(),
        Operand::Pair(Reg16::Sp) if m == Mnemonic::Ex => "(sp)".into(),
        Operand::Pair(p) => format!("{p:?}").to_lowercase(),
        Operand::IndHl => "(hl)".into(),
        Operand::IndBc => "(bc)".into(),
        Operand::IndDe => "(de)".into(),
        Operand::Indexed { reg, disp } => format!("({}{:+})", format!("{reg:?}").to_lowercase(), disp),
        Operand::Imm8(n) => format!("{n}"),
        Operand::Imm16(n) => sym.map(|s| s.to_string()).unwrap_or(format!("{n}")),
        Operand::Mem(a) => sym.map(|s| format!("({s})")).unwrap_or(format!("({a})")),
        Operand::Cc(c) => format!("{c:?}").to_lowercase(),
        Operand::Bit(b) => format!("{b}"),
        Operand::Rel(_) => ".tgt".into(),
        Operand::AfShadow => "af'".into(),
        Operand::RegI => "i".into(),
        Operand::RegR => "r".into(),
        Operand::IndC => "(c)".into(),
    }
}

fn assemble(src: &str, env: &SymbolTable) -> Result<Vec<u8>, Vec<String>> {
    let (file, perrs) = parse_str(src);
    let perrs: Vec<String> = perrs.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
    if !perrs.is_empty() { return Err(perrs); }
    let (m, diags) = lower_module(&file, &LowerOptions { roots: None, initial_cpu: Cpu::Z80, include_root: None, embed_base: None, defines: vec![] });
    let errs: Vec<String> = diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
    if !errs.is_empty() { return Err(errs); }
    let resolved = sigil_link::resolve_layout(&m.sections, env, true).map_err(|e| vec![format!("resolve_layout: {e:?}")])?;
    let linked = sigil_link::link(&resolved, env).map_err(|e| vec![format!("link: {e:?}")])?;
    sigil_link::flatten(&linked, 0x00).map_err(|e| vec![format!("flatten: {e}")])
}

fn frame(line: &str) -> String {
    format!("module m (cpu: z80)\nproc f() {{\n.tgt:\n    nop\n    {line}\n    ret\n}}\n")
}

fn line(m: Mnemonic, s: &str, ops: &[Operand], sym: Option<&str>) -> String {
    let o = ops.iter().map(|o| op_text(m, o, sym)).collect::<Vec<_>>().join(", ");
    format!("{s} {o}")
}

/// Assemble `l` in the frame and require exactly `nop`, `bytes`, `ret`.
fn emp_writes(l: &str, bytes: &[u8], env: &SymbolTable) -> Result<(), String> {
    let want: Vec<u8> = [&[0x00u8][..], bytes, &[0xC9][..]].concat();
    match assemble(&frame(l), env) {
        Ok(b) if b == want => Ok(()),
        Ok(b) => Err(format!("WRONG BYTES {b:02X?} want {want:02X?}")),
        Err(e) => Err(e.join(" | ")),
    }
}

type Demand = BTreeMap<Vec<u8>, (Mnemonic, &'static str, Vec<Operand>)>;

fn demand() -> (Demand, Vec<String>) {
    let pool = pool();
    let mut shapes: Vec<Vec<Operand>> = vec![vec![]];
    for a in &pool {
        shapes.push(vec![*a]);
    }
    for a in &pool {
        for b in &pool {
            shapes.push(vec![*a, *b]);
        }
    }
    let mut demand: Demand = BTreeMap::new();
    let mut reached: Vec<Mnemonic> = Vec::new();
    for (m, s) in all_mnemonics() {
        for ops in &shapes {
            if let Ok(b) = encode(&Instruction { mnemonic: m, ops: ops.clone() }) {
                reached.push(m);
                demand.entry(b).or_insert((m, s, ops.clone()));
            }
        }
    }
    let unreached = all_mnemonics().into_iter().filter(|(m, _)| !reached.contains(m)).map(|(m, _)| format!("{m:?}")).collect();
    (demand, unreached)
}

/// The floor under the demand, measured at sigil `55c6ec32`.
const MIN_DISTINCT: usize = 538;

/// 16-bit-field forms that still refuse a link-time symbol, as
/// `("<mnemonic> <mem|imm16>", count)`. A row whose count is no longer true fails.
const SYM_REFUSED: &[(&str, usize)] = &[
    // `in a, (n)` / `out (n), a`: the port is ONE byte, so there is no 16-bit
    // field to carry a symbol. Booked: EMP-Z80-SYMBOLIC-BYTE-OPERAND.
    ("ld mem", 28),
    ("in mem", 1),
    ("out mem", 1),
];

#[test]
fn the_walk_demands_every_z80_mnemonic() {
    let (demand, unreached) = demand();
    // `ld i, a` / `ld r, a` encode through `Mnemonic::Ld` (`ED 47`, `ED 4F`, both
    // in the demand); the two dedicated variants are arms `encode` never accepts.
    // Booked: ISA-Z80-DEAD-MNEMONICS. Any other name here is a hole in the pool.
    assert_eq!(unreached, ["LdIA", "LdRA"], "Z80 mnemonics with no encodable form in the pool");
    assert!(demand.len() >= MIN_DISTINCT, "{} distinct encodings, floor {MIN_DISTINCT}", demand.len());
}

#[test]
fn emp_assembles_every_z80_instruction_the_shared_encoder_accepts() {
    let (demand, _) = demand();
    let empty = SymbolTable::new();
    let refused: Vec<String> = demand
        .iter()
        .filter_map(|(bytes, (m, s, ops))| {
            let l = line(*m, s, ops, None);
            emp_writes(&l, bytes, &empty).err().map(|e| format!("`{l}`: {e}"))
        })
        .collect();
    assert!(refused.is_empty(), "{} Z80 instruction(s) `.emp` refuses:\n{refused:#?}", refused.len());
}

#[test]
fn every_16_bit_z80_field_takes_a_link_time_symbol() {
    let (demand, _) = demand();
    let mut tried = 0usize;
    let mut refused: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (bytes, (m, s, ops)) in &demand {
        let field = ops.iter().find_map(|o| match o {
            Operand::Mem(a) => Some(("mem", *a)),
            Operand::Imm16(n) => Some(("imm16", *n)),
            _ => None,
        });
        let Some((kind, v)) = field else { continue };
        tried += 1;
        let mut env = SymbolTable::new();
        env.define("ZSym", sigil_ir::SymbolValue::Int(i64::from(v)));
        let l = line(*m, s, ops, Some("ZSym"));
        if let Err(e) = emp_writes(&l, bytes, &env) {
            refused.entry(format!("{s} {kind}")).or_default().push(format!("`{l}`: {e}"));
        }
    }
    assert!(tried >= 54, "only {tried} forms carry a 16-bit field, floor 54");
    let got: BTreeMap<&str, usize> = refused.iter().map(|(k, rows)| (k.as_str(), rows.len())).collect();
    let want: BTreeMap<&str, usize> = SYM_REFUSED.iter().copied().collect();
    assert_eq!(got, want, "symbol refusals differ from SYM_REFUSED:\n{refused:#?}");
}

#[test]
fn the_walk_can_see_a_refusal() {
    let empty = SymbolTable::new();
    assert_eq!(emp_writes("nop", &[0x00], &empty), Ok(()));
    let wrong = emp_writes("nop", &[0x76], &empty).unwrap_err();
    assert!(wrong.starts_with("WRONG BYTES"), "{wrong}");
    let unknown = emp_writes("frobnicate a", &[0x00], &empty).unwrap_err();
    assert!(unknown.contains("frobnicate"), "{unknown}");
    // An undefined link-time symbol is a refusal, not a zero.
    assert!(emp_writes("ld hl, ZSym", &[0x21, 0x00, 0x00], &empty).is_err());
}
```

- [ ] **Step 3: Run both.** `CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-harness --test emp_isa_walk_m68k --test emp_isa_walk_z80`. Expected: `3 passed` and `4 passed` (measured at `55c6ec32`, about 3 s and 1 s). Then `--test skip_marker_lint`: `6 passed`. That lint reads every test source for prints that announce a skipped check; the two files pass it as given, and an added `eprintln!` about "no form" or "missing" will fail it (the prototype's did).
- [ ] **Step 4: Prove the table is not decoration (RF).** Delete the `("subx", 6)` entry: `emp_assembles_every_68000_instruction_the_shared_encoder_accepts` must FAIL, printing six `subx` lines each with `` `subx` is not a recognized 68000 mnemonic ``. Restore. Change `("movem", 4)` to `("movem", 3)`: FAIL. Restore. Delete `("ld mem", 28)` from the Z80 table: `every_16_bit_z80_field_takes_a_link_time_symbol` must FAIL listing 28 forms. Restore. Paste the three failures into the note.
- [ ] **Step 5:** Book the two QUEUE rows, clippy, commit the two test files, `docs/QUEUE.md` and the note.

---

### Task 2: `abcd`, `sbcd`, `nbcd`, `negx`, `subx`, and bare `exg`

Pure wiring: the encoder, `writes_last_operand`, the flag roles (`Addx | Subx | Negx | Abcd | Sbcd | Nbcd => ClearsOnly`) and `is_sp_arithmetic` already know these.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/code.rs` (`m68k_mnemonic` `:1737`, `m68k_default_size` `:1851`), `crates/sigil-frontend-emp/src/flag_check.rs` (`UNSPELLABLE` doc, two string arms), `crates/sigil-cli/tests/support/frontend_diff.rs`, `crates/sigil-harness/tests/emp_isa_walk_m68k.rs`
- Create: `crates/sigil-harness/tests/emp_a11_golden.rs`, `crates/sigil-frontend-emp/tests/a11_checks.rs`

**Interfaces:**
- Produces, for Tasks 3 to 9: in `emp_a11_golden.rs`, `emp(body) -> Result<Vec<u8>, Vec<String>>`, `asl(body) -> Result<Vec<u8>, Vec<String>>`, `framed(&[u8]) -> Vec<u8>`, `golden(line: &str, inst: Instruction, hex: &[u8])`, `i(Mnemonic, Size, &[Operand]) -> Instruction`; in `a11_checks.rs`, `diags(header: &str, body: &str) -> Vec<(bool, String)>` (`true` = error level) and `has(&[(bool, String)], id: &str) -> bool`.

- [ ] **Step 1: Red.** In `KNOWN_REFUSED` delete `("abcd", 2), ("sbcd", 2), ("nbcd", 8), ("negx", 24), ("subx", 6)`. Create `crates/sigil-harness/tests/emp_a11_golden.rs`:

```rust
//! A11 golden vectors: each instruction A11 makes writable, three ways. The
//! literal bytes, the shared encoder (`sigil_isa::m68k::encode`) and both front
//! ends (`.emp`, and the asl-compatible AS route) must agree.
use m68k::{encode, Instruction, Mnemonic, Operand, Size};
use sigil_isa::m68k;

#[path = "../../sigil-cli/tests/support/frontend_diff.rs"]
mod frontend_diff;

fn emp(body: &str) -> Result<Vec<u8>, Vec<String>> {
    frontend_diff::emp_assemble(&format!("module m\nproc f() {{\n.tgt:\n    nop\n    {body}\n    rts\n}}\n"))
}

fn asl(body: &str) -> Result<Vec<u8>, Vec<String>> {
    frontend_diff::as_assemble(&format!("\tcpu 68000\ntgt:\n\tnop\n\t{body}\n\trts\n"))
}

fn framed(mid: &[u8]) -> Vec<u8> {
    [&[0x4E, 0x71][..], mid, &[0x4E, 0x75][..]].concat()
}

/// `line` assembles to `hex` on both routes, and `hex` is what the shared
/// encoder gives for `inst`.
#[track_caller]
fn golden(line: &str, inst: Instruction, hex: &[u8]) {
    assert_eq!(encode(&inst).as_deref(), Ok(hex), "shared encoder, `{line}`");
    assert_eq!(emp(line), Ok(framed(hex)), "`.emp`, `{line}`");
    assert_eq!(asl(line), Ok(framed(hex)), "AS route, `{line}`");
}

fn i(mnemonic: Mnemonic, size: Size, ops: &[Operand]) -> Instruction {
    Instruction { mnemonic, size, ops: ops.to_vec() }
}

#[test]
fn the_extended_and_decimal_arithmetic_assembles() {
    use Operand::*;
    golden("abcd d0, d1", i(Mnemonic::Abcd, Size::B, &[Dn(0), Dn(1)]), &[0xC3, 0x00]);
    golden("abcd -(a0), -(a1)", i(Mnemonic::Abcd, Size::B, &[PreDec(0), PreDec(1)]), &[0xC3, 0x08]);
    golden("sbcd d0, d1", i(Mnemonic::Sbcd, Size::B, &[Dn(0), Dn(1)]), &[0x83, 0x00]);
    golden("nbcd d0", i(Mnemonic::Nbcd, Size::B, &[Dn(0)]), &[0x48, 0x00]);
    golden("nbcd (a0)", i(Mnemonic::Nbcd, Size::B, &[Ind(0)]), &[0x48, 0x10]);
    golden("negx.w d0", i(Mnemonic::Negx, Size::W, &[Dn(0)]), &[0x40, 0x40]);
    golden("negx.b (a0)", i(Mnemonic::Negx, Size::B, &[Ind(0)]), &[0x40, 0x10]);
    golden("subx.w d0, d1", i(Mnemonic::Subx, Size::W, &[Dn(0), Dn(1)]), &[0x93, 0x40]);
    golden("subx.l -(a0), -(a1)", i(Mnemonic::Subx, Size::L, &[PreDec(0), PreDec(1)]), &[0x93, 0x88]);
}

#[test]
fn exg_assembles_with_and_without_its_suffix() {
    use Operand::*;
    for line in ["exg d0, d1", "exg.l d0, d1"] {
        golden(line, i(Mnemonic::Exg, Size::L, &[Dn(0), Dn(1)]), &[0xC1, 0x41]);
    }
    golden("exg a0, a1", i(Mnemonic::Exg, Size::L, &[An(0), An(1)]), &[0xC1, 0x49]);
    golden("exg d0, a1", i(Mnemonic::Exg, Size::L, &[Dn(0), An(1)]), &[0xC1, 0x89]);
    let e = emp("exg.w d0, d1").unwrap_err();
    assert!(e[0].contains("exg is long-only"), "{e:?}");
}
```

Create `crates/sigil-frontend-emp/tests/a11_checks.rs`:

```rust
//! A11: what the checks say about the instructions A11 makes writable. The
//! bytes are pinned in `sigil-harness/tests/emp_a11_golden.rs`; this file pins
//! that each new instruction is SEEN by the contract checks, not merely encoded.
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Level;

/// Every diagnostic of a one-proc module, as `(is_error, message)`.
fn diags(header: &str, body: &str) -> Vec<(bool, String)> {
    module_diags(&format!("module m\nproc P () {header} {{\n{body}\n}}\n"))
}

/// Every diagnostic of a whole module source, as `(is_error, message)`.
fn module_diags(src: &str) -> Vec<(bool, String)> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let opts = LowerOptions { roots: None, initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let (_m, d) = lower_module(&file, &opts);
    d.iter().map(|d| (d.level == Level::Error, d.message.clone())).collect()
}

fn has(d: &[(bool, String)], id: &str) -> bool {
    d.iter().any(|(_, m)| m.starts_with(id))
}

#[test]
fn the_new_arithmetic_writes_its_destination_register() {
    let d = diags("clobbers()", "    abcd d0, d1\n    rts");
    assert!(has(&d, "[proc.clobber-undeclared] `P` writes `d1`"), "{d:?}");
    for line in ["abcd d0, d1", "sbcd d0, d1", "nbcd d1", "negx.w d1", "subx.w d0, d1"] {
        let d = diags("clobbers(d1)", &format!("    {line}\n    rts"));
        assert_eq!(d, [], "`{line}` under clobbers(d1)");
    }
    let d = diags("clobbers(d0)", "    exg d0, d1\n    rts");
    assert!(has(&d, "[proc.clobber-undeclared] `P` writes `d1`"), "{d:?}");
}
```

Run the three test files; all fail on the five mnemonics and on bare `exg`. Keep the output for RF.
- [ ] **Step 2: Mnemonic table.** In `m68k_mnemonic`, beside the line `"addx" => Addx,`, add `"subx" => Subx`, `"abcd" => Abcd`, `"sbcd" => Sbcd`, `"negx" => Negx`, `"nbcd" => Nbcd`.
- [ ] **Step 3: Default sizes.** In `m68k_default_size` (the `.emp` one, `lower/code.rs`), add arms `Abcd | Sbcd | Nbcd => Some(M68kSize::B)` and `Exg => Some(M68kSize::L)`. `subx` and `negx` get NO default, exactly like `addx` today (Open question 8). The encoder already refuses a wrong size (`exg is long-only on the 68000`, `abcd is byte-only on the 68000`).
- [ ] **Step 4: Flag-role string arms.** In `flag_check.rs`, `m68k_zero_role` has an arm `"subx" | "negx" | "abcd" | "sbcd" | "nbcd" => return Some(FlagRole::ClearsOnly)` whose comment says `.emp`'s table spells only ADDX; `m68k_carry_role` has its sibling (returning `FlagRole::Writes`). Delete both arms and their comments: the enum arms below now answer, with the same roles. The existing tests `the_emp_words_have_a_zero_role_and_an_unknown_word_has_none` (and the carry twin) assert `zero_role("subx", &[], ...) == Some(ClearsOnly)` and must stay green UNCHANGED: that is the proof the roles did not move.
- [ ] **Step 5: FD.** In `frontend_diff.rs`: remove `"abcd", "sbcd", "nbcd", "negx", "subx"` from `EMP_ABSENT_MNEMONICS` and from the prose of the `emp-lacks-mnemonic` row; in `SIZE_DEFAULT_GAP_FAMILIES` add `"subx", "negx"` and remove `"exg"`; in the `emp-requires-a-size-suffix` prose replace `` `exg`, `addx` `` with `` `addx`, `subx`, `negx` `` and drop the `` `exg` long `` clause.
- [ ] **Step 6: Green.** The three files from Step 1, then TC. Expected: all pass; the differential's table shows `SameBytes` for the five families.
- [ ] **Step 7: BN,** clippy, commit, then RF step 3. Mutation: comment out the `"subx" => Subx,` line. Expected failures: `the_extended_and_decimal_arithmetic_assembles` and the walk (naming `subx`, 6).

---

### Task 3: `rtr`

`rtr` pops a word into the condition codes and then returns. Three tables are wrong the moment it is a word: it is not a terminator (so a proc ending in `rtr` warns that it falls through), the stack model does not know it pops 2 bytes (so the correct idiom is an ERROR), and it accepts a size suffix the AS route refuses.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/code.rs`, `crates/sigil-frontend-emp/src/lower/proc.rs` (`is_terminator` `:1178`), `crates/sigil-frontend-emp/src/preserves.rs` (`transfer`, `:960`), `crates/sigil-frontend-emp/src/flag_check.rs` (`UNSPELLABLE`), `crates/sigil-frontend-emp/tests/as_author_forms_stay_off_emp.rs`, the walk table, `emp_a11_golden.rs`, `a11_checks.rs`

**Interfaces:**
- Produces, for Tasks 6 to 9: `fn m68k_suffix_refusal(m: M68kMnemonic, written: &str, size: Option<Width>) -> Option<String>` in `lower/code.rs`, complete for all of A11's instructions and called at the top of `lower_m68k_instr`; and the golden test `a_size_suffix_the_instruction_lacks_is_refused`, whose line list Tasks 6 to 9 extend.

- [ ] **Step 1: Red.** Delete `("rtr", 1)` from `KNOWN_REFUSED`. Append to `emp_a11_golden.rs`:

```rust

#[test]
fn rtr_assembles() {
    golden("rtr", i(Mnemonic::Rtr, Size::W, &[]), &[0x4E, 0x77]);
}

/// A suffix the instruction does not have is refused, on both routes.
#[test]
fn a_size_suffix_the_instruction_lacks_is_refused() {
    for line in ["rtr.w", "rtr.l"] {
        let e = emp(line).unwrap_err();
        assert!(e[0].starts_with("[lower.size-suffix] "), "`{line}`: {e:?}");
        assert!(asl(&line.replace("\n    ", "\n\t")).is_err(), "AS route accepts `{line}`");
    }
}
```

Append to `a11_checks.rs`:

```rust

#[test]
fn rtr_ends_a_proc_and_pops_the_saved_ccr_word() {
    // The idiom: save the condition codes, work, return restoring them.
    let d = diags("clobbers()", "    move.w sr, -(sp)\n    rtr");
    assert_eq!(d, [], "a balanced `rtr` body");
    // A leftover long under the saved word is still an imbalance.
    let d = diags("clobbers()", "    move.l d0, -(sp)\n    move.w sr, -(sp)\n    rtr");
    assert!(has(&d, "[stack.unbalanced]"), "{d:?}");
}
```

- [ ] **Step 2:** `m68k_mnemonic`: `"rtr" => Rtr`. `m68k_default_size`: add `Rtr` to the `Swap | Nop | Rts | Rte | Tas | Trap | Illegal` word arm.
- [ ] **Step 3: Terminator.** `is_terminator` in `lower/proc.rs`: the 68000 arm `matches!(mnemonic, "rts" | "rte" | "bra" | "jmp" | "jbra" | "illegal")` gains `"rtr"`.
- [ ] **Step 4: The pop.** In `preserves.rs` `transfer`, immediately before the line `if mnem == "link" {`, add:

```rust
    // `rtr` pops the saved condition-code WORD, then returns: model the pop so
    // the balance the return is judged at is the one the machine has.
    if mnem == "rtr" {
        return apply_sp_cleanup(st, 2);
    }
```

`transfer` runs before the walk's return observer, so the return sees the popped state. With nothing pushed, `apply_sp_cleanup` reports an underflow and the path bails to silence (the model has no negative depth: its documented blind spot, unchanged). With a long pushed under the word, the 2-byte pop lands on the word and the long remains: `[stack.unbalanced]`, which Step 1's second assertion pins.
- [ ] **Step 5: The suffix rule.** In `lower/code.rs`, BEFORE the doc comment that begins `/// The implicit size for mnemonics that real 68k syntax never suffixes.`, add:

```rust
/// The refusal for a size suffix one of the A11 instructions does not have, or
/// `None`. The same rule, instruction by instruction, as the AS front end's
/// `m68k_suffix_refusal` (which copies asl): `reset`, `rtr`, `trapv`, `stop` and
/// `unlk` take no suffix, `link` is word only, and the 68000 has only `chk.w`
/// (`chk.l` is an MC68020 opcode a 68000 does not decode).
fn m68k_suffix_refusal(m: M68kMnemonic, written: &str, size: Option<Width>) -> Option<String> {
    use M68kMnemonic::*;
    let size = size?;
    match m {
        Reset | Rtr | Trapv | Stop | Unlk => Some(format!(
            "[lower.size-suffix] `{written}` has no size field, write `{written}` with no suffix"
        )),
        Link if size != Width::W => Some(format!(
            "[lower.size-suffix] `{written}` is word only on the 68000, write `{written}` or `{written}.w`"
        )),
        Chk if size != Width::W => Some(format!(
            "[lower.size-suffix] the 68000 has only `{written}.w`"
        )),
        _ => None,
    }
}
```

and in `lower_m68k_instr`, directly after the `let Some(m) = m68k_mnemonic(mnemonic) else { ... };` block:

```rust
    if let Some(msg) = m68k_suffix_refusal(m, mnemonic, size) {
        push_err(diags, span, msg);
        return;
    }
```

The function is given whole, as compiled in the prototype: the arms for `reset`, `trapv`, `stop`, `unlk`, `link` and `chk` cannot be reached until Tasks 6 to 9 add those words, and each of those tasks adds its own lines to the suffix test. Existing words (`nop.w`, `swap.w`) keep today's behaviour: the rule names only A11's instructions (Open question 6).
- [ ] **Step 6: Lists.** `flag_check.rs` `UNSPELLABLE`: remove `"rtr"` (and from its doc comment). `as_author_forms_stay_off_emp.rs`, `the_other_as_rewrites_do_not_reach_emp`: remove `"rtr"` from the loop's list. No FD edit (`rtr` takes no operand; the generator skips it).
- [ ] **Step 7:** Green, TC, BN, clippy, commit, RF. Mutation: delete `"rtr"` from `is_terminator`. Expected failure: `rtr_ends_a_proc_and_pops_the_saved_ccr_word` on its first assertion, showing `[proc.undeclared-fallthrough]`. Second mutation: `apply_sp_cleanup(st, 2)` to `(st, 0)`: same test fails showing `[stack.unbalanced] ... 2 bytes`.

---

### Task 4: `move` to and from `usp`

`usp` is refused at three places in the operand evaluator so that it can never be read as a symbol (A6 fix rounds; `usp_reserved.rs`). This task lets it through at ONE of them, as a real operand, and keeps the named refusal everywhere the two moves are not.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/value.rs` (`CodeOperand`, `:554`), `crates/sigil-frontend-emp/src/eval/asm.rs` (`:3504`, `:3816`), `crates/sigil-frontend-emp/src/lower/code.rs` (`m68k_operand` `:1569`, `refine_m68k_mnemonic` `:1897`, `lower_m68k_instr`), `crates/sigil-frontend-emp/src/flag_check.rs` (`UNSPELLABLE`, `spellings`), `crates/sigil-frontend-emp/tests/usp_reserved.rs`, `frontend_diff.rs`, the walk table, the two A11 test files, `docs/QUEUE.md` (`EMP-USP-CCR-SURFACE`: usp half closed)

**Interfaces:**
- Produces: `CodeOperand::Usp` (a unit variant). Every existing `match` on `CodeOperand` has a wildcard arm (measured: the variant compiles with no other edit), so analyses treat it as "not a register, not memory".

- [ ] **Step 1: Red.** Delete `("move-to-usp", 1), ("move-from-usp", 1)` from `KNOWN_REFUSED`. Append to `emp_a11_golden.rs`:

```rust

#[test]
fn the_usp_moves_assemble() {
    use Operand::*;
    golden("move.l a0, usp", i(Mnemonic::MoveToUsp, Size::L, &[An(0), Usp]), &[0x4E, 0x60]);
    golden("move.l usp, a0", i(Mnemonic::MoveFromUsp, Size::L, &[Usp, An(0)]), &[0x4E, 0x68]);
}
```

Append to `a11_checks.rs`:

```rust

#[test]
fn usp_is_an_operand_of_the_two_moves_only() {
    assert_eq!(diags("clobbers()", "    move.l a0, usp\n    rts"), []);
    let d = diags("clobbers()", "    move.l usp, a1\n    rts");
    assert!(has(&d, "[proc.clobber-undeclared] `P` writes `a1`"), "{d:?}");
    for line in ["move.l usp, d0", "lea usp, a0", "move.w a0, usp", "move.l #1, usp", "tst.l usp"] {
        let d = diags("clobbers(d0, a0)", &format!("    {line}\n    rts"));
        assert!(errors(&d).iter().any(|m| m.starts_with("[operand.usp-unsupported] ")), "`{line}`: {d:?}");
    }
}

/// The error-level messages only.
fn errors(d: &[(bool, String)]) -> Vec<&str> {
    d.iter().filter(|(e, _)| *e).map(|(_, m)| m.as_str()).collect()
}
```

- [ ] **Step 2: The operand.** In `value.rs`, after the `Ccr` variant of `CodeOperand` (after the variant, not after its doc comment), add `Usp` with the doc comment `/// \`usp\`, the user stack pointer: an operand of \`move.l aN, usp\` and \`move.l usp, aN\` only.`
- [ ] **Step 3: Let it through at one place.** `eval/asm.rs` has three `self.error(..., USP_UNSUPPORTED.to_string())` sites. The one in the plain-operand mapper (about `:3505`, beside the two lines that return `CodeOperand::Sr` / `CodeOperand::Ccr` for the words `sr` / `ccr`) becomes `return Some(CodeOperand::Usp);`. The other two (`(usp)`-style and splice paths, about `:2931` and `:3760`) stay. Reword the const (`:3816`) to: `"[operand.usp-unsupported] \`usp\` is the 68000 user stack pointer, never a symbol; it is an operand of \`move.l aN, usp\` and \`move.l usp, aN\` only"`.
- [ ] **Step 4: Encoder mapping.** `m68k_operand`: beside `CodeOperand::Sr => Ok(M68kOperand::Sr),` add `CodeOperand::Usp => Ok(M68kOperand::Usp),`. `refine_m68k_mnemonic`: add `(Move, [_, M68kOperand::Usp]) => MoveToUsp,` and `(Move, [M68kOperand::Usp, _]) => MoveFromUsp,`.
- [ ] **Step 5: The guard.** In `lower/code.rs`, before the doc comment of `m68k_suffix_refusal`, add:

```rust
/// The refusal for `usp` anywhere but the two instructions that take it.
const USP_ONLY_IN_MOVE: &str = "[operand.usp-unsupported] `usp` is the 68000 user stack pointer, an operand of `move.l aN, usp` and `move.l usp, aN` only";

/// Is this `move.l aN, usp` or `move.l usp, aN`?
fn is_usp_move(mnemonic: &str, size: Option<Width>, ops: &[CodeOperand]) -> bool {
    let an = |o: &CodeOperand| matches!(o, CodeOperand::Reg(r) if reg_kind(*r).0);
    mnemonic == "move"
        && size == Some(Width::L)
        && match ops {
            [a, CodeOperand::Usp] | [CodeOperand::Usp, a] => an(a),
            _ => false,
        }
}
```

and in `lower_m68k_instr`, directly before the `m68k_suffix_refusal` call:

```rust
    if ops.iter().any(|o| matches!(o, CodeOperand::Usp)) && !is_usp_move(mnemonic, size, ops) {
        push_err(diags, span, USP_ONLY_IN_MOVE.to_string());
        return;
    }
```

(`reg_kind(r) -> (is_address, number)` is the helper `lower_m68k_pcrel` already uses in this file.) Bare `move a0, usp` is refused here too: `.emp` has no default size for `move`.
- [ ] **Step 6: `usp_reserved.rs`.** Three tests asserted `has(&e, OPERAND)` on the two legal moves. Rewrite them to the new truth, keeping their point (a declared or outside `usp` never becomes the operand's value): `a_proc_named_usp_...` and `move_from_usp_is_refused_beside_a_data_item_named_usp` still assert `has(&e, DECL)` and now assert `!has(&e, OPERAND)`; `an_outside_symbol_named_usp_never_resolves_as_the_operand` asserts, with `usp = 0x1234` in the link environment, `emp("...move.l a2, usp...") == Ok(vec![0x4E, 0x62, 0x4E, 0x75])` and `move.l usp, a3` `== Ok(vec![0x4E, 0x6B, 0x4E, 0x75])` (the user stack pointer, not `21CA 1234`), and keeps `has(&e, OPERAND)` for `move.l usp, d0` and `lea usp, a0`. Rename the two test functions whose names say "refused" to say what they now prove. Update the file's header comment.
- [ ] **Step 7: Lists.** `flag_check.rs`: remove both families from `UNSPELLABLE`; in `spellings`, beside the `"move-from-sr"` arm, add `"move-to-usp" => vec![("move".into(), vec![CodeOperand::Reg(Reg::A0), CodeOperand::Usp])]` and `"move-from-usp" => vec![("move".into(), vec![CodeOperand::Usp, CodeOperand::Reg(Reg::A0)])]`. FD: delete the `emp-lacks-usp` row (its `EXCLUSIONS` entry and its `match` arm) and `USP_FAMILIES` (also from `families_without_an_emp_form`); add `"move-to-usp", "move-from-usp"` to `SIZE_DEFAULT_GAP_FAMILIES` (bare `move a2, usp`: the AS route defaults to long, `.emp` asks for the suffix, as for every `move`).
- [ ] **Step 8:** Green, TC, BN, clippy, QUEUE, commit, RF. Mutation: in the guard, replace `!is_usp_move(mnemonic, size, ops)` with `false`. Expected failure: `usp_is_an_operand_of_the_two_moves_only` on `move.l usp, d0`.

---

### Task 5: `move <ea>, ccr`, `eori #, ccr`, and `movem` from a PC-relative source

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/code.rs` (`refine_m68k_mnemonic` `:1897`, `lower_m68k_movem` `:712`), `as_author_forms_stay_off_emp.rs`, `frontend_diff.rs`, the walk table, `emp_a11_golden.rs`

- [ ] **Step 1: Red.** Delete `("move-to-ccr", 11), ("eori-ccr", 1), ("movem", 4)` from `KNOWN_REFUSED`. Append to `emp_a11_golden.rs`:

```rust

#[test]
fn the_ccr_forms_assemble() {
    use Operand::*;
    golden("move.w d0, ccr", i(Mnemonic::MoveToCcr, Size::W, &[Dn(0), Ccr]), &[0x44, 0xC0]);
    golden("move.w #$1F, ccr", i(Mnemonic::MoveToCcr, Size::W, &[Imm(0x1F), Ccr]), &[0x44, 0xFC, 0x00, 0x1F]);
    golden("eori.b #$01, ccr", i(Mnemonic::EoriCcr, Size::B, &[Imm(1), Ccr]), &[0x0A, 0x3C, 0x00, 0x01]);
}

#[test]
fn movem_loads_from_a_pc_relative_source() {
    // `.tgt` is the frame's first byte: the displacement is from the extension word at +4.
    assert_eq!(emp("movem.w .tgt(pc), d0-d1"), Ok(framed(&[0x4C, 0xBA, 0x00, 0x03, 0xFF, 0xFA])));
    assert_eq!(asl("movem.w tgt(pc),d0-d1"), Ok(framed(&[0x4C, 0xBA, 0x00, 0x03, 0xFF, 0xFA])));
    assert_eq!(emp("movem.l .tgt(pc,d0.w), d0-d1"), asl("movem.l tgt(pc,d0.w),d0-d1"));
    assert!(emp("movem.l .tgt(pc,d0.w), d0-d1").is_ok());
}
```

- [ ] **Step 2: CCR.** `refine_m68k_mnemonic`: add `(Move, [_, M68kOperand::Ccr]) => MoveToCcr,` and `(Eori, [_, M68kOperand::Ccr]) => EoriCcr,` beside the existing `AndiCcr`/`OriCcr`/`MoveToSr` arms. The encoder owns the size rules (measured: `move.l d0, ccr` is `move to ccr takes .b or .w`, `eori.w #1, ccr` is `an immediate to \`ccr\` is byte only`). A CCR-only write does not fire `[proc.sr-undeclared]` (`every_sr_write_form_is_an_undeclared_mask_write` in Task 7 pins that).
- [ ] **Step 3: `movem`.** `lower_m68k_instr` routes every `movem` to `lower_m68k_movem` before its PC-relative branch, and `lower_m68k_movem` hands the memory operand to the generic mapper, which answers `internal: a PC-relative operand reached the generic operand mapper`. In `lower_m68k_movem`, immediately before the comment that begins `// A width-PINNED symbolic absolute memory EA`, add (`mem_op`, `mask`, `list_first`, `size` are that function's existing locals):

```rust
    if matches!(mem_op, CodeOperand::PcRel { .. } | CodeOperand::PcRelIdx { .. }) {
        let (pc_op, target, idx) = match mem_op {
            CodeOperand::PcRel { target: t, addend } => {
                (M68kOperand::Pcd16(0), pcrel_target_expr(t, *addend), false)
            }
            CodeOperand::PcRelIdx { target: t, addend, xn, xlong } => {
                let (is_a, n) = reg_kind(*xn);
                let xn = if is_a { M68kXn::A(n) } else { M68kXn::D(n) };
                (M68kOperand::Pcd8Xn { d: 0, xn, long: *xlong }, pcrel_target_expr(t, *addend), true)
            }
            _ => unreachable!(),
        };
        let list_op = M68kOperand::RegList(mask);
        let ops = if list_first { vec![list_op, pc_op] } else { vec![pc_op, list_op] };
        let inst = M68kInst { mnemonic: M68kMnemonic::Movem, size, ops };
        let result = if idx {
            M68kBackend.lower_pcrel_idx_ea(&inst, target, span)
        } else {
            M68kBackend.lower_pcrel_ea(&inst, target, span)
        };
        match result {
            Ok(df) => emit_data_frag(builder, df),
            Err(e) => push_err(diags, span, e.message),
        }
        return;
    }
```

If a local has a different name in the tree, read `lower_m68k_movem` and `lower_m68k_pcrel` (which this mirrors) and adapt the names only. A PC-relative DESTINATION (`movem d0-d1, .tgt(pc)`) is not a 68000 instruction; the encoder refuses it, and Step 1 does not ask for it.
- [ ] **Step 4: Lists.** `as_author_forms_stay_off_emp.rs`: the line `assert_unchanged("eori.b #1,ccr", &["unsupported form: ccr is not a general EA"]);` becomes `assert_unchanged("eori.b #1,ccr", &[]);` with a comment `// A11: a real instruction, the bytes are pinned in emp_a11_golden.rs`. FD: remove `"move-to-ccr", "eori-ccr"` from `SR_CCR_GAP_FAMILIES` and from the row's prose; in the `emp-label-operand-gap` row remove the `internal: a PC-relative operand reached the generic operand mapper` alternative from the predicate and the `movem` pc-relative clause from the prose.
- [ ] **Step 5:** Green, TC, BN, clippy, commit, RF. Mutation: delete the `(Move, [_, M68kOperand::Ccr]) => MoveToCcr,` arm. Expected: `the_ccr_forms_assemble` and the walk (`move-to-ccr`, 11).

---

### Task 6 (decision group): `link` / `unlk`

**Decision taken, the narrowest that is certainly right:** assemble both, and lean on the stack model that already exists (`preserves.rs` `apply_link` / `apply_unlk`, with unit tests): a paired frame is balanced; a `link` whose frame is open at a return is `[stack.unbalanced]`; an `unlk` with no matching `link` on the path, a positive displacement, or `link a7` bails that path to silence. This task adds only what the model cannot infer: `link` writes its frame register (its FIRST operand), and the suffix rule. It does NOT check that `d(a6)` accesses stay inside the frame, and does not add a frame construct (Open question 1).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/code.rs`, `crates/sigil-frontend-emp/src/lower/proc.rs` (`instr_written_regs`, `:1543`), `flag_check.rs` (`UNSPELLABLE`), `as_author_forms_stay_off_emp.rs`, `frontend_diff.rs`, the walk table, the two A11 test files, `docs/QUEUE.md` (book `EMP-LINK-FRAME-BOUNDS`: frame-relative accesses are not checked against the frame size; and `EMP-LINK-UNSIGNED-DISPLACEMENT`: `link a6, #$FFF8` assembles as `-8`, as on the AS route, and `link a6, #8` assembles and bails the model)

- [ ] **Step 1: Red.** Delete `("link", 1), ("unlk", 1)` from `KNOWN_REFUSED`. Append to `emp_a11_golden.rs`:

```rust

#[test]
fn link_and_unlk_assemble() {
    use Operand::*;
    assert_eq!(encode(&i(Mnemonic::Link, Size::W, &[An(6), Imm(-8)])).as_deref(), Ok(&[0x4E, 0x56, 0xFF, 0xF8][..]));
    assert_eq!(encode(&i(Mnemonic::Unlk, Size::W, &[An(6)])).as_deref(), Ok(&[0x4E, 0x5E][..]));
    let pair = framed(&[0x4E, 0x56, 0xFF, 0xF8, 0x4E, 0x5E]);
    assert_eq!(emp("link a6, #-8\n    unlk a6"), Ok(pair.clone()));
    assert_eq!(emp("link.w a6, #-8\n    unlk a6"), Ok(pair.clone()));
    assert_eq!(asl("link a6,#-8\n\tunlk a6"), Ok(pair));
}
```

and add `"unlk.l a6", "link.l a6, #-8\n    unlk a6"` to the list in `a_size_suffix_the_instruction_lacks_is_refused`. Append to `a11_checks.rs`:

```rust

#[test]
fn a_frame_is_balanced_only_when_it_is_closed() {
    let d = diags("clobbers() preserves(a6)", "    link a6, #-8\n    move.w d0, -4(a6)\n    unlk a6\n    rts");
    assert_eq!(d, [], "a paired frame that restores a6");
    let d = diags("clobbers(a6)", "    link a6, #-8\n    rts");
    assert_eq!(errors(&d).len(), 1, "{d:?}");
    assert!(errors(&d)[0].starts_with("[stack.unbalanced] in `P`: this path returns with 12 bytes"), "{d:?}");
    // `link` writes its frame register even though it is the FIRST operand.
    let d = diags("clobbers()", "    link a6, #-8\n    unlk a6\n    rts");
    let charged = d.iter().filter(|(_, m)| m.starts_with("[proc.clobber-undeclared] `P` writes `a6`")).count();
    assert_eq!(charged, 2, "one for `link`, one for `unlk`: {d:?}");
}
```

- [ ] **Step 2:** `m68k_mnemonic`: `"link" => Link, "unlk" => Unlk`. `m68k_default_size`: add `Link | Unlk` to the word arm.
- [ ] **Step 3: `link` writes `aN`.** `instr_written_regs` has `if mnemonic == "exg" {` pushing the first operand; change the condition to `mnemonic == "exg" || mnemonic == "link"` and extend its comment (`link aN, #d` writes `aN`, its first operand). `unlk aN` already charges `aN` through the last-operand rule.
- [ ] **Step 4: Suffix rule.** Nothing to write: Task 3's `m68k_suffix_refusal` already has the `Unlk` and `Link` arms, and Step 1's two new lines in the suffix test now reach them.
- [ ] **Step 5: Lists.** `UNSPELLABLE`: remove `"link"`, `"unlk"`. `as_author_forms_stay_off_emp.rs`: remove `"unlk a6"` from the loop. FD: remove `"link", "unlk"` from `EMP_ABSENT_MNEMONICS` and the prose, delete the `|| (mn == "link" && s.starts_with("[stack.unbalanced]"))` alternative and its comment from the `emp-lacks-mnemonic` predicate, and in `emp_spelling` change the frame from `"module m\nproc f() {{..."` to `"module m\n@allow(\"stack.unbalanced\")\nproc f() {{..."` (the differential compares one instruction at a time, so `link` is necessarily alone in its frame; measured: with the allow the whole differential is green and `link` compares `SameBytes`).
- [ ] **Step 6:** Green, TC, BN, clippy, QUEUE, commit, RF. Mutation: revert Step 3's condition to `mnemonic == "exg"`. Expected failure: `a_frame_is_balanced_only_when_it_is_closed`, last assertion, `left: 1, right: 2` (only the `unlk` line is charged).

---

### Task 7 (decision group): `stop` and the SR immediates

**Decision taken:** review B's Q7 (is the interrupt level a per-path value) is open, so nothing here models what a mask VALUE means. Every one of these is treated as A10 treats `move #imm, sr`: an undeclared write of the mask, `[proc.sr-undeclared]`, which the gated build already turns into an error. That holds for `andi.w #$F8FF, sr` (lowering the mask) exactly as for `ori.w #$0700, sr` (raising it): the direction is the model Q7 decides (Open question 2). `stop #imm` loads SR and has no `sr` operand, so it is added by name at each place an SR write is recognised. `stop` is not a terminator (execution continues after the interrupt) and gets no cycle row (Task 10).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/code.rs`, `crates/sigil-frontend-emp/src/lower/proc.rs` (`check_clobbers` `:1378`, `sr_writes_round_trip` `:2934`), `crates/sigil-frontend-emp/src/lower/mod.rs` (`:2064`), `crates/sigil-frontend-emp/src/corpus_contracts.rs` (`:1320`), `flag_check.rs` (`UNSPELLABLE`), `as_author_forms_stay_off_emp.rs`, `frontend_diff.rs`, the walk table, the two A11 test files, `crates/sigil-cli/tests/sr_undeclared_gate.rs`, `docs/QUEUE.md` (`EMP-USP-CCR-SURFACE` closed)

- [ ] **Step 1: Red.** Delete `("stop", 1), ("andi-sr", 1), ("ori-sr", 1), ("eori-sr", 1)` from `KNOWN_REFUSED`. Append to `emp_a11_golden.rs`:

```rust

#[test]
fn stop_and_the_sr_immediates_assemble() {
    use Operand::*;
    golden("stop #$2700", i(Mnemonic::Stop, Size::W, &[Imm(0x2700)]), &[0x4E, 0x72, 0x27, 0x00]);
    golden("ori.w #$0700, sr", i(Mnemonic::OriSr, Size::W, &[Imm(0x0700), Sr]), &[0x00, 0x7C, 0x07, 0x00]);
    golden("andi.w #$F8FF, sr", i(Mnemonic::AndiSr, Size::W, &[Imm(0xF8FF), Sr]), &[0x02, 0x7C, 0xF8, 0xFF]);
    golden("eori.w #$0700, sr", i(Mnemonic::EoriSr, Size::W, &[Imm(0x0700), Sr]), &[0x0A, 0x7C, 0x07, 0x00]);
}
```

and add `"stop.w #$2700"` to the suffix test's list. Append to `a11_checks.rs`:

```rust

#[test]
fn every_sr_write_form_is_an_undeclared_mask_write() {
    for line in ["stop #$2000", "ori.w #$0700, sr", "andi.w #$F8FF, sr", "eori.w #$0700, sr"] {
        let d = diags("clobbers()", &format!("    {line}\n    rts"));
        assert!(has(&d, "[proc.sr-undeclared] `P` writes `sr`"), "`{line}`: {d:?}");
        let d = diags("clobbers(sr)", &format!("    {line}\n    rts"));
        assert!(!has(&d, "[proc.sr-undeclared]"), "`{line}` declared: {d:?}");
    }
    // A condition-code-only write is not a mask write.
    for line in ["eori.b #1, ccr", "move.w d0, ccr"] {
        let d = diags("clobbers()", &format!("    {line}\n    rts"));
        assert!(!has(&d, "[proc.sr-undeclared]"), "`{line}`: {d:?}");
    }
}

#[test]
fn stop_cannot_hide_behind_preserves_sr() {
    let d = diags("clobbers() preserves(sr)", "    stop #$2700\n    rts");
    assert!(has(&d, "[proc.preserves-sr-unbalanced]"), "{d:?}");
    let d = diags("clobbers() preserves(sr)", "    move.w sr, -(sp)\n    stop #$2000\n    move.w (sp)+, sr\n    rts");
    assert!(!has(&d, "[proc.preserves-sr-unbalanced]"), "{d:?}");
}
```

- [ ] **Step 2:** `m68k_mnemonic`: `"stop" => Stop`. `m68k_default_size`: `Stop` joins the word arm. `refine_m68k_mnemonic`: `(Andi, [_, M68kOperand::Sr]) => AndiSr,`, `(Ori, [_, M68kOperand::Sr]) => OriSr,`, `(Eori, [_, M68kOperand::Sr]) => EoriSr,`. The suffix rule for `stop` is already in Task 3's function. The encoder owns the rest (measured: `ori.b #1, sr` is `OriSr is word only`).
- [ ] **Step 3: `stop` is an SR write, four places.** After Step 2 the three immediates already fire (`ops.last()` is `Sr`, measured). For `stop`:
  1. `lower/proc.rs` `check_clobbers`, the condition `if writes_dest_register(mnemonic) && matches!(ops.last(), Some(CodeOperand::Sr)) && !mask_covered && ...` becomes `if ((writes_dest_register(mnemonic) && matches!(ops.last(), Some(CodeOperand::Sr))) || mnemonic == "stop") && !mask_covered && ...`.
  2. `lower/proc.rs` `sr_writes_round_trip`, directly after the `if mnemonic == "rte" { return false; }` block:

```rust
        // `stop #imm` loads SR from its immediate with no `sr` operand: an SR
        // write that is never the restore.
        if mnemonic == "stop" {
            saved_before_first_write.get_or_insert(saved);
            last_write_is_restore = false;
            continue;
        }
```

  3. `lower/mod.rs` (`safe_entry_labels`): `let touches_sr = matches!(mnemonic.as_str(), "rte" | "rtr")` gains `"stop"`.
  4. `corpus_contracts.rs` `:1320`: the census condition `matches!(ops.last(), Some(crate::value::CodeOperand::Sr))` is the copy of site 1; give it the same `|| mnemonic == "stop"` (read the surrounding condition and keep its other terms).
  Then `grep -n "CodeOperand::Sr" crates/sigil-frontend-emp/src -r` and list every hit in the note with one word each: "write site, has stop", "read", or "operand mapping". Two more recognisers at `lower/proc.rs:2821` and `:2830` match the exact save and restore shapes and need nothing.
- [ ] **Step 4: The gated build.** In `crates/sigil-cli/tests/sr_undeclared_gate.rs`, add `const PLANT_STOP: &str = "\npub proc Zz_A11_StopProbe () clobbers(d0) {\n        moveq   #0, d0\n        stop    #$2000\n        rts\n}\n";` beside `PLANT`, and a test `a_stop_stops_the_gated_build` that is `an_undeclared_sr_write_stops_the_gated_build` with `planted(PLANT_STOP)` and the probe name `Zz_A11_StopProbe` in the `find`: same exit code 1, same `HEADER`, same "closure gate passed first", same "no ROM". The fixture guard (`gate_tree::GateTree`) deletes its tree copy on drop; do not add a second copy mechanism. This is the build-level proof A10's review asked for.
- [ ] **Step 5: Lists.** `UNSPELLABLE`: remove `"stop"`. `as_author_forms_stay_off_emp.rs`: `assert_unchanged("andi.w #$F8FF,sr", &["unsupported form: sr is not a general EA"]);` becomes `assert_unchanged("andi.w #$F8FF,sr", &[]);` with the A11 comment; remove `"stop #$2700"` from the loop. FD: remove `"stop"` from `EMP_ABSENT_MNEMONICS` and the prose; delete the `emp-lacks-sr-ccr-form` row, its arm and `SR_CCR_GAP_FAMILIES` (also from `families_without_an_emp_form`).
- [ ] **Step 6:** Green, TC (also `-p sigil-cli --test sr_undeclared_gate`), BN, clippy, QUEUE, commit, RF. Mutation A: drop `|| mnemonic == "stop"` from `check_clobbers`: `every_sr_write_form_is_an_undeclared_mask_write` fails on `stop #$2000`. Mutation B: delete the `stop` block from `sr_writes_round_trip`: `stop_cannot_hide_behind_preserves_sr` fails on its first assertion.

---

### Task 8 (decision group): `reset`

**Decision taken:** `reset` asserts the RESET line to external devices and changes no CPU register, flag or stack word. It is assembled as a plain instruction with no effect in any model, and no suffix. Whether a `reset` outside boot code deserves a warning is a policy the card leaves open (Open question 4).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/code.rs`, `flag_check.rs` (`UNSPELLABLE`), `as_author_forms_stay_off_emp.rs`, the walk table, `emp_a11_golden.rs`

- [ ] **Step 1: Red.** Delete `("reset", 1)` from `KNOWN_REFUSED`. Append to `emp_a11_golden.rs`:

```rust

#[test]
fn reset_assembles() {
    golden("reset", i(Mnemonic::Reset, Size::W, &[]), &[0x4E, 0x70]);
}
```

and add `"reset.w"` to the suffix test's list.
- [ ] **Step 2:** `m68k_mnemonic`: `"reset" => Reset`. `m68k_default_size`: word arm. The suffix rule is already in Task 3's function.
- [ ] **Step 3: Lists.** `UNSPELLABLE`: remove `"reset"`. `as_author_forms_stay_off_emp.rs`: remove `"reset"` from the loop. No FD edit (no operand).
- [ ] **Step 4:** Green, TC, BN, clippy, commit, RF. Mutation: comment out `"reset" => Reset,`. Expected: `reset_assembles` and the walk (`reset`, 1). The check-level assertion for `reset` is in Task 9's test.

---

### Task 9 (decision group): `chk` / `trapv`

**Decision taken:** both can raise an exception, which is a transfer to a handler: "a kind of call". The existing language already has two instructions of that kind, `trap #n` and `divu`/`divs` (divide by zero), and gives neither a call edge: the handler is assumed to return with the registers intact or not at all. `chk` and `trapv` get exactly that treatment, no more: no call edge, no clobber charge, flag roles as the ISA table has them (`Chk => Writes`, `Trapv => Untouched`), and no cycle row, so a `@budget` proc containing one is refused as unpriceable (the data-dependent trap is exactly what a budget cannot bound). Size rule from the AS front end: the 68000 has only `chk.w`, and bare `chk` is refused on both routes (Open questions 5 and 7).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/code.rs`, `flag_check.rs` (`UNSPELLABLE` deleted), `as_author_forms_stay_off_emp.rs`, `frontend_diff.rs`, `crates/sigil-cli/tests/frontend_differential.rs`, the walk test, the two A11 test files, `docs/QUEUE.md` (book `EMP-EXCEPTION-EDGE`: `chk`, `trapv`, `trap`, `divu`/`divs` have no edge to their handler)

- [ ] **Step 1: Red.** Delete `("chk", 11), ("trapv", 1)` from `KNOWN_REFUSED`: the table is now empty. Append to `emp_a11_golden.rs`:

```rust

#[test]
fn chk_and_trapv_assemble() {
    use Operand::*;
    golden("chk.w d0, d1", i(Mnemonic::Chk, Size::W, &[Dn(0), Dn(1)]), &[0x43, 0x80]);
    golden("chk.w (a0), d1", i(Mnemonic::Chk, Size::W, &[Ind(0), Dn(1)]), &[0x43, 0x90]);
    golden("trapv", i(Mnemonic::Trapv, Size::W, &[]), &[0x4E, 0x76]);
    // `chk` has no default size on either route.
    assert_eq!(emp("chk d0, d1").unwrap_err(), ["instruction needs an explicit size suffix (.b/.w/.l)"]);
    assert!(asl("chk d0,d1").is_err());
}
```

and add `"trapv.w", "chk.l d0, d1", "chk.b d0, d1"` to the suffix test's list. Append to `a11_checks.rs`:

```rust

#[test]
fn chk_trapv_and_reset_are_plain_instructions_to_the_checks() {
    assert_eq!(diags("clobbers()", "    chk.w d1, d0\n    trapv\n    reset\n    rts"), []);
}

/// Review Focus 1: a new mnemonic does not take a name away from data.
#[test]
fn a_const_or_label_named_after_a_new_mnemonic_is_still_a_name() {
    let d = module_diags("module m\nconst stop = 3\nconst link = 4\nproc P () clobbers(d0) {\n.chk:\n    moveq #stop + link, d0\n    bra.s .chk\n}\n");
    assert_eq!(d, [], "{d:?}");
}
```

The second test is a control: it passes today and must pass after (a constant or a label may keep a name that is now also a mnemonic). The first, and the golden, are the red.

- [ ] **Step 2:** `m68k_mnemonic`: `"chk" => Chk, "trapv" => Trapv`. `m68k_default_size`: `Trapv` joins the word arm; `Chk` gets NO default (bare `chk` is refused on both routes). The suffix rules for both are already in Task 3's function.
- [ ] **Step 3: Lists, now empty.** `flag_check.rs`: `UNSPELLABLE` has no entry left; delete the const, its doc comment and the two `if UNSPELLABLE.contains(family) { continue; }` skips, so the zero and carry sweeps cover all 85 families (the default `plain` arm of `spellings` yields each new word with no operands, which is all a role lookup needs); print `checked` once and raise both sweeps' `checked > 60` floor to the measured count minus 5. `as_author_forms_stay_off_emp.rs`: the loop over refused mnemonics is now empty; delete it and say in the test's comment that A11 made those words instructions. FD: delete the `emp-lacks-mnemonic` row, its arm, `EMP_ABSENT_MNEMONICS` and `families_without_an_emp_form`; in `frontend_differential.rs` (`every_family_with_a_legal_emp_form_compared_bytes`, `:107`) drop the `none` set so EVERY family is held to at least one `SameBytes` case. Bare `chk` is `BothRefuse` and needs no row.
- [ ] **Step 4: Retire the table.** In `emp_isa_walk_m68k.rs` delete `KNOWN_REFUSED` and its doc comment, and end `emp_assembles_every_68000_instruction_the_shared_encoder_accepts` with `assert!(refused.is_empty(), "{} family(ies) with an instruction `.emp` refuses:\n{refused:#?}", refused.len());` in place of the `got`/`want` comparison. From here on the test is the card's sentence, with no exception list.
- [ ] **Step 5:** Green, TC, BN, clippy, QUEUE, commit, RF. Mutation: comment out `"trapv" => Trapv,`. Expected: the walk fails with one `trapv` family, and `chk_and_trapv_assemble` fails.

---

### Task 10: Cycle rows (`sigil-isa`)

The card: "`exg` ... is missing from the cycle table". Measured in `crates/sigil-isa/src/m68k_cycles.rs` (`instr_cycles`, `:162`): no arm names `Exg`, nor any of `Subx Abcd Sbcd Negx Nbcd Link Unlk Rtr Reset Chk Trapv Stop`, nor `Roxl Roxr Bchg` (pre-existing gaps, not A11's). A missing arm is `CycleCost::Unmodeled`, which `@budget` and `cycles()` refuse loudly, so a gap is never a wrong number, only a refusal.

**Decision taken:** price `exg` (the card) and the A11 instructions whose cost is a fixed manual number; leave `chk`, `trapv` (data-dependent trap) and `stop` (waits for an interrupt) deliberately `Unmodeled`, pinned by test; book the rest (Open question 10).

**Files:**
- Modify: `crates/sigil-isa/src/m68k_cycles.rs` (arms and unit tests), `crates/sigil-frontend-emp/src/m68k_cycles.rs` (`tests::off_table_forms_refuse`, `:332`), `crates/sigil-frontend-emp/src/cycle_budget.rs` (`tests::an_off_table_68k_op_is_unknown`, `:1408`), `docs/QUEUE.md` (book `ISA-CYCLE-TABLE-GAPS`: `roxl`/`roxr`/`bchg`, the SR/CCR/`usp` moves and immediates have no row; no test sweeps the table by family)

**Interfaces:**
- Consumes: `pub fn instr_cycles(m: Mnemonic, size: Size, ops: &[CatOp]) -> CycleCost`; the file's own helpers (`exact(n)`, the `long` flag, the effective-address time helper the `Neg`/`Not`/`Clr` arm uses).

- [ ] **Step 1: Source the numbers.** Open the M68000 User's Manual, section 8 ("Instruction Execution Times": tables 8-4 single operand, 8-7 multiprecision, 8-10 miscellaneous, 8-11 exception processing) and cross-read `oracle-old/Devices/M68000` for the same instructions. The values this plan expects, each to be confirmed against the manual and cited by table in the arm's comment: `exg` 6; `link` 16; `unlk` 12; `rtr` 20; `reset` 132; `subx` the `addx` row (Dn,Dn 4 byte/word and 8 long; -(An),-(An) 18 and 30); `abcd`/`sbcd` Dn,Dn 6 and -(An),-(An) 18; `negx` the `neg` row (Dn 4 and 6; memory 8 and 12 plus EA time); `nbcd` Dn 6 and memory 8 plus EA time. If the manual disagrees with a number here, the manual wins: record the difference in the note.
- [ ] **Step 2: Write the failing tests** in the unit-test module of `sigil-isa/src/m68k_cycles.rs`, in the style of its existing tests (read the nearest one that prices `Addx`): one assertion per value in Step 1, and `the_trapping_and_waiting_instructions_are_unmodeled` asserting `instr_cycles` returns `CycleCost::Unmodeled` for `Chk` (`.w`, Dn,Dn), `Trapv` and `Stop`. Run: the priced ones FAIL (`Unmodeled`), the last passes.
- [ ] **Step 3: Add the arms.** `Subx` joins the `Addx` arms and `Negx` joins the `Neg` single-operand arm (same shapes); the others are new arms beside them. Measured to compile in the prototype for the fixed ones: `(Exg, _) => exact(6)`, `(Link, _) => exact(16)`, `(Unlk, _) => exact(12)`, `(Rtr, _) => exact(20)`, `(Reset, _) => exact(132)`, `(Abcd | Sbcd, [CatOp::Ea(EaCat::Dn), CatOp::Ea(EaCat::Dn)]) => exact(6)`, `(Abcd | Sbcd, [CatOp::Ea(EaCat::PreDec), CatOp::Ea(EaCat::PreDec)]) => exact(18)`. Put them before the final `_ => CycleCost::Unmodeled` arm, not after a more general arm that would shadow them (rustc's `unreachable pattern` warning is the check; clippy is `-D warnings`).
- [ ] **Step 4: Two tests in `sigil-frontend-emp` use `link` as "an instruction with no price".** `m68k_cycles::tests::off_table_forms_refuse`: change the `link` assertion to `instr_cost("stop", None, &[CodeOperand::Imm(0x2000)])`, still `CycleCost::Unmodeled`, and add beside it `assert_eq!(instr_cost("exg", Some(Width::L), &[<d0>, <d1>]), <the fixed 6-cycle cost>)` written with that module's own constructors. `cycle_budget::tests::an_off_table_68k_op_is_unknown`: replace `"link"` with `"trapv"` in both the instruction and the expected `UnknownOp { mnemonic }`.
- [ ] **Step 5:** TC for `sigil-isa` and `sigil-frontend-emp`, BN (the cycle table feeds strength-reduction tie-breaks; measured identical with these rows), clippy, QUEUE, commit, RF. Mutation: delete the `Exg` arm. Expected: the `exg` test in `sigil-isa` and the new `exg` assertion in `off_table_forms_refuse` fail.

---

### Task 11: Z80 `(nn)` takes a link-time symbol

**Reproduction (in the note first):** `module m (cpu: z80)` / `proc f() { ld a, (q) / ret }` / `q: ret`-style forward label, `sigil emp` at `55c6ec32`: `unknown name \`q\`` and `[lower.z80-unsupported] a Z80 indirect base must be ... got poison`. `ld hl, q` in the same file assembles.

**Cause:** `map_ind_z80` (`eval/asm.rs`, its last arm, find the text `Some(addr) => Some(CodeOperand::Z80Mem { addr }),`) evaluates the base with plain `self.eval_expr`, where an unresolved name is an error, and accepts only a stored integer. `map_plain_z80` (find `in_imm_link_ctx` near `:2570`) evaluates under the link context and gets a `Value::Label` / `Value::LinkExpr`.

**Fix:** a second operand kind for a link-time address, lowered through the existing symbolic 16-bit path (`lower_z80_abs16_sym`), whose fixup is `Value16Le` over the last two bytes: in every `ld` form with `(nn)`, `nn` IS the last two bytes (opcodes `3A 32 2A 22`, `ED 4B/5B/7B`, `ED 43/53/73`, `DD/FD 2A`, `DD/FD 22`).

**Files:**
- Modify: `crates/sigil-frontend-emp/src/value.rs`, `crates/sigil-frontend-emp/src/eval/asm.rs` (`map_ind_z80`), `crates/sigil-frontend-emp/src/lower/code.rs` (`lower_z80_instr` `:1939`, `lower_z80_abs16_sym`, the Z80 arm list at `:1670`), `crates/sigil-frontend-emp/src/z80_cycles.rs` (`:260-294`), `crates/sigil-harness/tests/emp_isa_walk_z80.rs`
- Test: `crates/sigil-frontend-emp/tests/a11_z80_mem_sym.rs` (new)

**Interfaces:**
- Produces: `CodeOperand::Z80MemSym { target: sigil_ir::expr::Expr }`.

- [ ] **Step 1: Red.** Delete `("ld mem", 28)` from `SYM_REFUSED` in `emp_isa_walk_z80.rs` (the `in`/`out` rows stay: Review Focus 5). Create `crates/sigil-frontend-emp/tests/a11_z80_mem_sym.rs` with exactly this content (every byte below was measured on the prototype):

```rust
//! A11: a Z80 `(nn)` operand takes a link-time symbol in every `ld` form that
//! has a 16-bit address field, and nowhere else.
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

fn assemble(src: &str, env: &SymbolTable) -> Result<Vec<u8>, Vec<String>> {
    let (file, perrs) = parse_str(src);
    let perrs: Vec<String> = perrs.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
    if !perrs.is_empty() {
        return Err(perrs);
    }
    let opts = LowerOptions { roots: None, initial_cpu: Cpu::Z80, include_root: None, embed_base: None, defines: vec![] };
    let (m, diags) = lower_module(&file, &opts);
    let errs: Vec<String> = diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
    if !errs.is_empty() {
        return Err(errs);
    }
    let resolved = sigil_link::resolve_layout(&m.sections, env, true).map_err(|e| vec![format!("resolve_layout: {e:?}")])?;
    let linked = sigil_link::link(&resolved, env).map_err(|e| vec![format!("link: {e:?}")])?;
    sigil_link::flatten(&linked, 0x00).map_err(|e| vec![format!("flatten: {e}")])
}

/// `line` then `ret` in `f`, followed by a one-byte proc `q`.
fn with_q(line: &str) -> Result<Vec<u8>, Vec<String>> {
    let src = format!("module m (cpu: z80)\nproc f() {{\n    {line}\n    ret\n}}\nproc q() {{\n    ret\n}}\n");
    assemble(&src, &SymbolTable::new())
}

#[test]
fn a_forward_label_is_a_z80_memory_operand() {
    // 3-byte forms: `q` is at 4. Prefixed forms: `q` is at 5.
    assert_eq!(with_q("ld a, (q)"), Ok(vec![0x3A, 0x04, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld (q), a"), Ok(vec![0x32, 0x04, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld hl, (q)"), Ok(vec![0x2A, 0x04, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld (q), hl"), Ok(vec![0x22, 0x04, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld (q), bc"), Ok(vec![0xED, 0x43, 0x05, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld sp, (q)"), Ok(vec![0xED, 0x7B, 0x05, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld ix, (q)"), Ok(vec![0xDD, 0x2A, 0x05, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld (q), iy"), Ok(vec![0xFD, 0x22, 0x05, 0x00, 0xC9, 0xC9]));
    assert_eq!(with_q("ld (q+1), a"), Ok(vec![0x32, 0x05, 0x00, 0xC9, 0xC9]));
}

#[test]
fn a_literal_address_is_unchanged() {
    assert_eq!(with_q("ld a, ($1234)"), Ok(vec![0x3A, 0x34, 0x12, 0xC9, 0xC9]));
}

#[test]
fn a_symbolic_port_is_refused_not_truncated() {
    for line in ["in a, (q)", "out (q), a"] {
        let e = with_q(line).unwrap_err();
        assert!(e[0].starts_with("[lower.z80-unsupported] "), "`{line}`: {e:?}");
    }
}

#[test]
fn an_undefined_symbol_is_a_link_error_not_a_zero() {
    let src = "module m (cpu: z80)\nproc f() {\n    ld a, (extern(\"Foo\"))\n    ret\n}\n";
    let e = assemble(src, &SymbolTable::new()).unwrap_err();
    assert!(e[0].contains("Foo"), "{e:?}");
    let mut env = SymbolTable::new();
    env.define("Foo", sigil_ir::SymbolValue::Int(0x4000));
    assert_eq!(assemble(src, &env), Ok(vec![0x3A, 0x00, 0x40, 0xC9]));
}
```

Run it and the Z80 walk: `a_forward_label_is_a_z80_memory_operand`, `an_undefined_symbol_is_a_link_error_not_a_zero` and the walk's symbol test fail; the other two pass today and must still pass after (they are the controls).
- [ ] **Step 2: The operand.** In `value.rs`, before the doc comment of the `Z80AfShadow` variant (find `/// \`af'\` (only in \`ex af,af'\`).`), add:

```rust
    /// `(Label)`: a LINK-TIME absolute memory address.
    Z80MemSym {
        /// The residual link-time expression.
        target: sigil_ir::expr::Expr,
    },
```

- [ ] **Step 3: Evaluate under the link context.** In `map_ind_z80`, replace `let v = self.eval_expr(e, env);` (the one directly above `match v.as_stored_int() {` and `Some(addr) => Some(CodeOperand::Z80Mem { addr }),`) with:

```rust
        let v = self.in_imm_link_ctx(|this| this.eval_expr(e, env));
        if let Value::Label(n) = &v {
            return Some(CodeOperand::Z80MemSym { target: sigil_ir::expr::Expr::Sym(n.clone()) });
        }
        if let Value::LinkExpr(le) = &v {
            if !crate::eval::expr::expr_carries_bank_mask(le) {
                return Some(CodeOperand::Z80MemSym { target: le.clone() });
            }
        }
```

A bank-masked expression falls through to the existing refusal, as it does for an immediate.
- [ ] **Step 4: Lower it.** In `lower/code.rs`: (a) add `| CodeOperand::Z80MemSym { .. }` beside `| CodeOperand::Z80Mem { .. }` in the 68000 operand mapper's Z80-operand arm (`:1670`); (b) in `lower_z80_instr`, the dispatch `if ops.iter().any(|o| matches!(o, CodeOperand::Sym(_) | CodeOperand::ImmLink { .. })) {` gains `| CodeOperand::Z80MemSym { .. }`; (c) in `lower_z80_abs16_sym`, before the arm `(Z80Mnemonic::Jp | Z80Mnemonic::Call, [CodeOperand::Sym(name)]) => {`, add:

```rust
        (Z80Mnemonic::Ld, [dst, CodeOperand::Z80MemSym { target }]) => {
            match map_z80_operands(m, std::slice::from_ref(dst)) {
                Ok(mut z) => {
                    z.push(Z80Operand::Mem(0));
                    (z, target.clone())
                }
                Err(msg) => {
                    push_err(diags, span, msg);
                    return;
                }
            }
        }
        (Z80Mnemonic::Ld, [CodeOperand::Z80MemSym { target }, src]) => {
            match map_z80_operands(m, std::slice::from_ref(src)) {
                Ok(z) => {
                    let mut v = vec![Z80Operand::Mem(0)];
                    v.extend(z);
                    (v, target.clone())
                }
                Err(msg) => {
                    push_err(diags, span, msg);
                    return;
                }
            }
        }
```

Any other mnemonic with a `Z80MemSym` (`in`, `out`) falls to that function's existing `[lower.z80-unsupported] a symbolic operand is only supported as ...` refusal; extend that message's list with `` `ld r, (Label)` / `ld (Label), r` ``.
- [ ] **Step 5: Show it to the analyses.** `z80_cycles.rs` prices `ld` by operand shape: at each of the six `Z80Mem { .. }` patterns in `:260-294` add `| Z80MemSym { .. }` (13, 13, 16, 20, 16, 20 cycles: a symbol costs what a number costs). Leave the `in`/`out` rows (`:427-428`) and `flag_check.rs:642` alone: those forms are refused before any analysis. In the cycles unit tests, beside the test that prices `instr("ld", vec![a(), CodeOperand::Z80Mem { addr: 0x4000 }])` (`:583`), add the same assertion with `CodeOperand::Z80MemSym { target: sigil_ir::expr::Expr::Sym("q".into()) }` and the same expected cost. Then `grep -rn "Z80Mem" crates/sigil-frontend-emp/src` and account for every hit in the note.
- [ ] **Step 6:** Green (the new file, the Z80 walk, TC for `sigil-frontend-emp`), **BN** (aeon's six Z80 sound modules go through `map_ind_z80`; measured: all 10 shapes identical with this change, because a comptime integer still takes the `Z80Mem` path), clippy, commit, RF. Mutation: in Step 3, replace `self.in_imm_link_ctx(|this| this.eval_expr(e, env))` with `self.eval_expr(e, env)`. Expected: `a_forward_label_is_a_z80_memory_operand` and the walk's `every_16_bit_z80_field_takes_a_link_time_symbol` (28 `ld mem` forms) fail.

---

### Task 12: All shapes identical, strict landing run

**Files:**
- Modify: `docs/superpowers/notes/2026-11-22-a11-complete-instruction-set.md`, `docs/QUEUE.md` (a `## STEP0-A11-INSTRUCTION-SET` row, state "built, awaiting landing"; `EMP-USP-CCR-SURFACE` closed; `EMP-FRONTEND-DIFF-GAPS` rewritten to the rows that remain, with re-measured counts from the differential's printed table)

- [ ] **Step 1: The booked items exist.** `grep -n` in `docs/QUEUE.md` for each: `ISA-Z80-DEAD-MNEMONICS`, `EMP-Z80-SYMBOLIC-BYTE-OPERAND`, `EMP-LINK-FRAME-BOUNDS`, `EMP-LINK-UNSIGNED-DISPLACEMENT`, `EMP-EXCEPTION-EDGE`, `ISA-CYCLE-TABLE-GAPS`, and `EMP-ROXL-SYMBOLIC-COUNT-MESSAGE` (found by this plan's measurement: `roxl.w #Sym, d1` is refused with the untagged `Roxl immediate count must be 1..=8, got 0` where its siblings say `[lower.imm-link]`; book it here). A missing row is added now.
- [ ] **Step 2: Rebuild the candidate** from the branch tip: `df -h /`; `CARGO_TARGET_DIR=.target-land cargo build --release --locked -p sigil-cli -p sigil-harness --bins`; `.target-land/release/sigil --version` names the tip.
- [ ] **Step 3: The seven shipped shapes.** Run **BN**. Expected `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`: sonic4 `release`, `debug`, `config-a`, `config-b`, `lean` and demo `release`, `debug`.
- [ ] **Step 4: The three fixture rows** `step0_shapes_cmp.sh` does not list. In a `cp -a` copy of the reference tree at `/home/volence/sonic_hacks/.a11-fx-$$` (remove its `.git` link file and every `__pycache__`):

```bash
for s in config-a-keystones-flipped stress-evict stress-art; do
  for side in base cand; do
    bin=/home/volence/sonic_hacks/.a11-base-55c6ec32/sigil; [ $side = cand ] && bin=$PWD/.target-land/release/sigil
    "$bin" build --project /home/volence/sonic_hacks/.a11-fx-$$/games/sonic4 --shape $s -o /home/volence/sonic_hacks/.a11-fx-$$/$s.$side.bin || echo "COULD NOT RUN $s $side"
  done
  cmp /home/volence/sonic_hacks/.a11-fx-$$/$s.base.bin /home/volence/sonic_hacks/.a11-fx-$$/$s.cand.bin && echo "FIXTURE $s SAME"
done
```

Expected: three `FIXTURE <row> SAME` lines and no `COULD NOT RUN`. Delete the copy.
- [ ] **Step 5: Aeon stays green with the candidate.** In a second copy of the reference tree outside every checkout, with `SIGIL_BUILD` and `SIGIL_EMIT` set to `.target-land/release/{sigil,emit_sound_blob}`: `./build.sh`, `DEBUG=1 ./build.sh`, `./build.sh demo`, `DEBUG=1 ./build.sh demo`, each exit 0 with every lane run (no `NO_LINT`, no `FAST`). The expect-fail lane must end `emp_expect_fail: OK`. Delete the copy.
- [ ] **Step 6: The strict landing run.** Count the tests added: `git diff 55c6ec32..HEAD -- crates | grep -c '^+#\[test\]'` minus `grep -c '^-#\[test\]'` of the same diff. Take the baseline from master AT THE TIME: the last landing run recorded on master (A10 recorded 6217 at `55c6ec32`); if master has moved past `10dcf37a` with code, rebase the branch first and use the count that landing recorded. **Use the landing-run invocation current at the time, including the Oracle lane if present** (a parallel parcel is adding an `ORACLE_G0A_DIR` environment variable to it; read `scripts/landing-run.sh --help` and `docs/OVERSEER.md` on master before running, and pass every variable they name). As of this plan the invocation is: copy `scripts/landing-run.sh` to a run-unique path, make a fresh `cp -a` copy of the reference tree at `/home/volence/sonic_hacks/.a11-land-$$` (it must keep its git metadata: the pairing gate reads its HEAD), remove `__pycache__`, then detached, polling your own log for the verdict:

```bash
ORACLE_DIR=/home/volence/sonic_hacks/oracle-old <copy of landing-run.sh> --baseline 6217 --aeon /home/volence/sonic_hacks/.a11-land-$$
```

Expected: `RESULT GREEN`, `reconciles 6217 baseline + <added> new = <total> observed`, `CARGO_EXIT=0`, `CLIPPY_EXIT=0`, `LEDGER_EXIT=0`, zero `skip:` lines. No byte moved, so there is nothing to refreeze or attest. A red test outside the list in "Baseline measured" is a finding this plan did not predict (its full-suite measurement was incomplete): fix it under the Global Constraints rule for pinned refusals, and name it in the note. Delete the copy of the tree and the copied script when done (A10 leaked a 139 MB copy per run).
- [ ] **Step 7: Record and commit.** In the note: the seven BN rows, the three fixture lines, the four aeon build exits, the RESULT block, the two walk tests' final counts (`emp assembles N of N` is the tests passing: quote the three and four `ok` lines), and one line per task naming its golden test, its check test and its RF transcript. Commit the note and `docs/QUEUE.md`.
- [ ] **Step 8: Stop.** The branch is ready. Merging to master and pushing are the overseer's and owner-gated; so is the card's Progress line in empyrean. Report: commits, aggregate totals, the `SHAPES 7/7 IDENTICAL` line, the RESULT block.

---

## Open questions (none blocks; each task ships the "Chosen" column)

| # | Question | Options | Chosen (and why) | Cost if wrong |
|---|---|---|---|---|
| 1 | What does the stack model owe `link`/`unlk`? | (a) assemble, use the existing frame model, charge `aN`; (b) also check `d(aN)` stays inside the frame; (c) refuse until review B designs frames | (a): the model exists and is tested; (b) is new analysis | A wild frame access is unseen, as any `d(aN)` is today. Booked `EMP-LINK-FRAME-BOUNDS` |
| 2 | How are `ori`/`andi`/`eori #, sr` and `stop` checked while Q7 is open? | (a) every one is an undeclared mask write unless `clobbers(sr)` / a proven `preserves(sr)`; (b) lowering the mask (`andi`) is exempt; (c) wait for Q7 | (a): the card's own wording, and A10's rule for `move #, sr` | When Q7 lands, sites declared `clobbers(sr)` to lower the mask get a finer contract. Aeon has 0 such lines |
| 3 | Is `stop` a terminator, and does it have a cost? | terminator or not; priced or not | Not a terminator (it resumes); `Unmodeled` (it waits) | A `@budget` proc with `stop` is refused: conservative |
| 4 | Should `reset` be restricted? | (a) plain instruction; (b) warn outside a boot proc; (c) refuse without an attribute | (a): there is no "boot proc" notion to hang (b) on | A stray `reset` is not flagged |
| 5 | Is a possibly-trapping instruction a call? | (a) as `trap #n` and `divu` today: no edge; (b) an edge to the handler vector with its contract | (a): (b) needs typed vectors (review B Q9) | A handler that returns with changed registers is unseen, as for divide-by-zero today. Booked `EMP-EXCEPTION-EDGE` |
| 6 | Refuse a meaningless size suffix (`rtr.w`, `stop.l`, `unlk.w`, `link.l`, `chk.l`)? | (a) refuse, new id `[lower.size-suffix]`, A11's instructions only; (b) ignore it as `.emp` does for `swap.w`; (c) refuse for every instruction | (a): the AS route and asl refuse each; `chk.l` is a 68020 opcode; (c) changes existing words | A new diagnostic id to keep. Removing it later breaks nothing |
| 7 | Default size for `chk`? | none (AS/asl) or `.w` | None: both routes then agree | Authors write `chk.w` |
| 8 | Default size for `subx`/`negx`? | none (like `.emp`'s `addx`) or `.w` (AS) | None: consistent inside `.emp`; the gap is the existing `emp-requires-a-size-suffix` row | Booked with `addx` in `EMP-FRONTEND-DIFF-GAPS` |
| 9 | `movem` from a PC-relative source is not on the card | (a) fix it (26 lines, measured); (b) leave it as the walk's one named exception | (a): otherwise the card's "fails if `.emp` refuses any instruction" test ships with an exception | 26 lines of lowering that mirror `lower_m68k_pcrel` |
| 10 | How much of the cycle table is A11's? | (a) `exg` only (the card's words); (b) plus A11's fixed-cost instructions; (c) every gap | (b); `chk`/`trapv`/`stop` stay unpriced on purpose; `roxl`/`roxr`/`bchg` and the SR/CCR/`usp` forms are booked | A wrong number mis-proves a budget: Task 10 Step 1 makes the manual the authority |
| 11 | Symbolic 8-bit Z80 operands (`in a,(Sym)`, `ld a, Sym`, `cp Sym`) | fix now or name and book | Named rows in the walk, booked: it needs an 8-bit link fixup with a range check | Authors use a comptime constant, as today |
| 12 | The two dead Z80 enum variants (`LdIA`, `LdRA`) | delete from `sigil-isa`, or pin and book | Pin and book: A11's licence to touch `sigil-isa` is the cycle rows | None |
