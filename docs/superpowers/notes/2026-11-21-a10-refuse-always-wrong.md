# A10 Refuse the always-wrong lines: parcel note

Branch `parcel/a10-refuse-always-wrong`, cut from sigil master `1e200e1b`; plan commit `532d371b`
(`docs/superpowers/plans/2026-11-21-a10-refuse-always-wrong.md`). Task card: empyrean
`docs/v2/tasks/A10 Refuse the always-wrong lines.md`.

## Task 0: base binary, reference tree, BN on itself

`git diff --stat 1e200e1b HEAD -- crates scripts contract Cargo.toml Cargo.lock` printed nothing: the
branch carries no code change at the base.

Base, built with `CARGO_TARGET_DIR=.target-land cargo build --release --locked -p sigil-cli -p sigil-harness --bins`
at `532d371b` (`sigil --version`: `sigil 0.1.0 (532d371b)`, `closure-revision: 1071467657f1`, sources clean),
parked in `/home/volence/sonic_hacks/.a10-base-1e200e1b/`:

```
1874f5a88bb06c8566ca4b6708d1cb7fb8626b0d414d70f40ea5d6af5aeb27b7  /home/volence/sonic_hacks/.a10-base-1e200e1b/sigil
7a5501c8ce184e89e835e340d6090b01f5097bd6820fe8aff987e95f2954a58e  /home/volence/sonic_hacks/.a10-base-1e200e1b/emit_sound_blob
```

Reference tree `/home/volence/sonic_hacks/.aeon-a10-ref`, provisioned with
`REF_BUILD_DEMO=1 CARGO_TARGET_DIR=.target-land scripts/provision-aeon-ref.sh /home/volence/sonic_hacks/.aeon-a10-ref`
at aeon `9b80db1d` (rc 0; `REBUILD CONTROL s4.bin 142a39b9/850262 MATCHES THE GOLDEN`,
`REBUILD CONTROL s4.debug.bin 20acdb67/870606 MATCHES THE GOLDEN`). `repin -- --check` last line:
`pins.rs unchanged`. The tree's four ROMs (zlib CRC-32 / size), equal to provenance chain #212:

| ROM | Measured | Control |
|---|---|---|
| `s4.bin` | `142a39b9/850262` | `142a39b9/850262` |
| `s4.debug.bin` | `20acdb67/870606` | `20acdb67/870606` |
| `demo.bin` | `4587df52/99980` | `4587df52/99980` |
| `demo.debug.bin` | `37d66535/106981` | `37d66535/106981` |

BN with the base as both sides (`scripts/step0_shapes_cmp.sh --base <base> --cand <base> --aeon-ref /home/volence/sonic_hacks/.aeon-a10-ref`, rc 0):

```
SHAPE plain 142a39b9/850262 142a39b9/850262 SAME
SHAPE debug 20acdb67/870606 20acdb67/870606 SAME
SHAPE demo 4587df52/99980 4587df52/99980 SAME
SHAPE demo_debug 37d66535/106981 37d66535/106981 SAME
SHAPE config_a 2ce7438f/870972 2ce7438f/870972 SAME
SHAPE config_b 128361b3/617715 128361b3/617715 SAME
SHAPE lean f6b07847/798198 f6b07847/798198 SAME
SHAPES 7/7 IDENTICAL
STEP0_CMP_END rc=0
```

## Baseline measured for this plan (2026-11-21), copied from the plan

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

## RF transcripts

### Task 1: `[proc.repeated-clause]`

Mutation (plan Task 1 Step 7): comment out the `self.note_contract_clause(...)` call in `proc_decl` only. The first run of this mutation against commit 1 did not compile (`E0282: type annotations needed for Vec<_>` on `let mut seen = Vec::new()`), so the declarations were given `Vec<&'static str>` in a follow-up commit and the transcript below is against that commit. (`--untracked-files=no` hides the pre-existing untracked `.target-a10/` build dir, not part of this branch.)

```
$ git status --short --untracked-files=no
$ git diff --stat
 crates/sigil-frontend-emp/src/parser.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
$ grep -n 'note_contract_clause' crates/sigil-frontend-emp/src/parser.rs
2100:            // self.note_contract_clause(&mut seen, &name);
2292:    fn note_contract_clause(&mut self, seen: &mut Vec<&'static str>, owner: &str) {
2315:            self.note_contract_clause(&mut seen, "this signature");
$ CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test proc_repeated_clause   # mutated
test a_repeated_clause_on_a_signature_is_refused ... ok
test a_repeated_clobbers_clause_is_refused ... FAILED
test each_clause_once_stays_legal ... ok
test every_clause_word_is_refused_when_repeated ... FAILED
thread 'a_repeated_clobbers_clause_is_refused' (410024) panicked at crates/sigil-frontend-emp/tests/proc_repeated_clause.rs:15:5:
  left: 0
 right: 1
thread 'every_clause_word_is_refused_when_repeated' (410026) panicked at crates/sigil-frontend-emp/tests/proc_repeated_clause.rs:23:9:
  left: 0
 right: 1
test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p sigil-frontend-emp --test proc_repeated_clause`
$ git restore --source=HEAD -- crates/sigil-frontend-emp/src/parser.rs
$ git status --short --untracked-files=no
$ CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test proc_repeated_clause   # restored
test each_clause_once_stays_legal ... ok
test every_clause_word_is_refused_when_repeated ... ok
test a_repeated_clause_on_a_signature_is_refused ... ok
test a_repeated_clobbers_clause_is_refused ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Task 2: `[name.duplicate-const]`

Mutation (plan Task 2 Step 8): comment out the `self.refuse_duplicate_consts(...)` call in `file`.

```
$ git status --short --untracked-files=no
$ git diff --stat
 crates/sigil-frontend-emp/src/parser.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
$ grep -n 'refuse_duplicate_consts' crates/sigil-frontend-emp/src/parser.rs
501:        // self.refuse_duplicate_consts(&items, &mut std::collections::HashSet::new());
512:    fn refuse_duplicate_consts(&mut self, items: &[Item], seen: &mut std::collections::HashSet<String>) {
520:                Item::Section(s) => self.refuse_duplicate_consts(&s.items, seen),
$ CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test name_duplicate_const   # mutated
test a_const_beside_an_equ_of_the_same_name_is_not_this_refusal ... ok
test a_const_declared_twice_is_refused_at_the_later_one ... FAILED
test distinct_names_and_other_kinds_stay_legal ... ok
test pub_typed_and_section_nested_duplicates_are_refused ... FAILED
thread 'a_const_declared_twice_is_refused_at_the_later_one' (563270) panicked at crates/sigil-frontend-emp/tests/name_duplicate_const.rs:16:5:
  left: 0
 right: 1
thread 'pub_typed_and_section_nested_duplicates_are_refused' (563272) panicked at crates/sigil-frontend-emp/tests/name_duplicate_const.rs:23:5:
  left: 0
 right: 1
test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p sigil-frontend-emp --test name_duplicate_const`
$ git restore --source=HEAD -- crates/sigil-frontend-emp/src/parser.rs
$ git status --short --untracked-files=no
$ CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test name_duplicate_const   # restored
test a_const_beside_an_equ_of_the_same_name_is_not_this_refusal ... ok
test a_const_declared_twice_is_refused_at_the_later_one ... ok
test pub_typed_and_section_nested_duplicates_are_refused ... ok
test distinct_names_and_other_kinds_stay_legal ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Task 3: `[operand.bare-number]`

Mutation (plan Task 3 Step 8): `if let Some(n) = v.as_stored_int()` becomes `if let (false, Some(n)) = (true, v.as_stored_int())` in `map_plain`. It compiled as written (no warning surfaced in the filtered output; the plan sample and the real code agree here).

```
$ git status --short --untracked-files=no
$ git diff --stat
 crates/sigil-frontend-emp/src/eval/asm.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
$ grep -n 'if let (false, Some(n))\|if let Some(n) = v.as_stored_int()' crates/sigil-frontend-emp/src/eval/asm.rs
3562:        if let (false, Some(n)) = (true, v.as_stored_int()) {
$ CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test operand_bare_number   # mutated
test a_z80_bare_number_is_untouched ... ok
test a_number_valued_expression_is_refused_too ... FAILED
test a_bare_number_operand_is_refused ... FAILED
test the_immediate_and_the_sized_absolute_stay_legal ... ok
test a_spliced_integer_stays_an_immediate ... ok
thread 'a_number_valued_expression_is_refused_too' (669114) panicked at crates/sigil-frontend-emp/tests/operand_bare_number.rs:61:5:
  left: 0
 right: 1
thread 'a_bare_number_operand_is_refused' (669113) panicked at crates/sigil-frontend-emp/tests/operand_bare_number.rs:53:5:
  left: 0
 right: 1
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p sigil-frontend-emp --test operand_bare_number`
$ git restore --source=HEAD -- crates/sigil-frontend-emp/src/eval/asm.rs
$ git status --short --untracked-files=no
$ CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test operand_bare_number   # restored
test a_bare_number_operand_is_refused ... ok
test the_immediate_and_the_sized_absolute_stay_legal ... ok
test a_number_valued_expression_is_refused_too ... ok
test a_z80_bare_number_is_untouched ... ok
test a_spliced_integer_stays_an_immediate ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Task 4: `[layout.odd-instruction]`

Mutation (plan Task 4 Step 8): the guard `cpu == Cpu::M68000` on the new `first_odd_instr` call in `lower_code_buf` becomes `cpu == Cpu::Z80`. As the plan predicts, the two probes go red and `a_z80_body_is_untouched` goes red too (the check now fires on the Z80 body), so the Z80 control has teeth. The `a_z80_body_is_untouched` panic is a plain `assert!` and prints its diagnostics on one long line, filtered out here.

```
$ git status --short --untracked-files=no
$ git diff --stat
 crates/sigil-frontend-emp/src/lower/code.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
$ grep -n -A1 'if cpu == Cpu::Z80 {' crates/sigil-frontend-emp/src/lower/code.rs | grep -B1 first_odd_instr
78:    if cpu == Cpu::Z80 {
79-        if let Some((span, n)) = first_odd_instr(&code.items) {
$ CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test layout_odd_instruction   # mutated
test an_instruction_after_an_odd_data_run_is_refused ... FAILED
test as_compat_does_not_exempt_it ... FAILED
test a_z80_body_is_untouched ... FAILED
test even_and_trailing_data_stay_legal ... ok
thread 'an_instruction_after_an_odd_data_run_is_refused' (2733245) panicked at crates/sigil-frontend-emp/tests/layout_odd_instruction.rs:47:5:
  left: 0
 right: 1
thread 'as_compat_does_not_exempt_it' (2733246) panicked at crates/sigil-frontend-emp/tests/layout_odd_instruction.rs:60:5:
  left: 0
 right: 1
thread 'a_z80_body_is_untouched' (2733244) panicked at crates/sigil-frontend-emp/tests/layout_odd_instruction.rs:82:5:
test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p sigil-frontend-emp --test layout_odd_instruction`
$ git restore --source=HEAD -- crates/sigil-frontend-emp/src/lower/code.rs
$ git status --short --untracked-files=no
$ CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test layout_odd_instruction   # restored
test as_compat_does_not_exempt_it ... ok
test a_z80_body_is_untouched ... ok
test an_instruction_after_an_odd_data_run_is_refused ... ok
test even_and_trailing_data_stay_legal ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Task 5: `[contract.binding-uncontracted]`

Mutation (plan Task 5 Step 8): the `refuse_uncontracted` call in `check_hook_sig` is disabled. The call is a three-line `if ... { return; }` block, so the one-line form is `if false && refuse_uncontracted(...)` rather than commenting it out (same effect, still compiles). Both probes go red: the hook probe finds no refusal, and the mixed probe keeps its `proc` member refusal (still wired in `check_proc_type`, shown in the panic with the exact message) but loses the `hook` one bound to the `extern proc`. The `clobbers()` twin and the limit test stay green.

```
$ git status --short --untracked-files=no
$ git diff --stat
 crates/sigil-frontend-emp/src/resolve/contract.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
$ grep -n 'refuse_uncontracted(iface' crates/sigil-frontend-emp/src/resolve/contract.rs
381:    if refuse_uncontracted(iface, member, bound_sym, "proc", all_items, span, diags) {
401:    if false && refuse_uncontracted(iface, member, bound_sym, "hook", all_items, span, diags) {
$ CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test game_contract   # mutated
test a_binding_to_a_proc_declaring_the_empty_clause_conforms ... ok
test probe_hook_bound_to_a_proc_with_no_clobbers_clause_is_refused ... FAILED
test probe_proc_member_and_extern_targets_with_no_clause_are_refused ... FAILED
test the_refusal_reaches_only_a_live_binding_to_a_declared_proc ... ok
thread 'probe_hook_bound_to_a_proc_with_no_clobbers_clause_is_refused' (2921830) panicked at crates/sigil-frontend-emp/tests/game_contract.rs:595:5:
  left: 0
 right: 1
thread 'probe_proc_member_and_extern_targets_with_no_clause_are_refused' (2921844) panicked at crates/sigil-frontend-emp/tests/game_contract.rs:606:5:
["[contract.binding-uncontracted] proc `Game.entry` is bound to `Bare`, which declares no `clobbers(...)` clause, so its conformance to the declared signature cannot be checked. Refused for now: state `clobbers(...)` on `Bare` (`clobbers()` if it changes no register)"]
test result: FAILED. 25 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p sigil-frontend-emp --test game_contract`
$ git restore --source=HEAD -- crates/sigil-frontend-emp/src/resolve/contract.rs
$ git status --short --untracked-files=no
$ CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --test game_contract   # restored
test a_binding_to_a_proc_declaring_the_empty_clause_conforms ... ok
test probe_hook_bound_to_a_proc_with_no_clobbers_clause_is_refused ... ok
test probe_proc_member_and_extern_targets_with_no_clause_are_refused ... ok
test the_refusal_reaches_only_a_live_binding_to_a_declared_proc ... ok
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Task 6: `[proc.sr-undeclared]` in the gated build

Mutation (plan Task 6 Step 8): in `run_project`, `run_project_build` is passed `false` instead of the gate's return value. The two refusal tests go red with exit code `Some(0)`, as the plan predicts; the declared-`sr` control and the two limit tests (`--check` / `SIGIL_CONTRACTS=0`, single-file `sigil emp`) stay green.

```
$ git status --short --untracked-files=no
$ git diff --stat
 crates/sigil-cli/src/main.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
$ grep -n 'run_project_build(&project' crates/sigil-cli/src/main.rs
3104:            run_project_build(&project, opts, false);
$ AEON_DIR=/home/volence/sonic_hacks/.aeon-a10-ref SIGIL_STRICT_GATE=1 CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-cli --test sr_undeclared_gate   # mutated
test the_single_file_path_stays_a_warning ... ok
test a_declared_sr_write_builds ... ok
test an_undeclared_sr_write_stops_the_gated_build ... FAILED
test warnings_off_does_not_hide_the_refusal ... FAILED
test outside_the_gate_it_stays_a_warning ... ok
thread 'an_undeclared_sr_write_stops_the_gated_build' (2742442) panicked at crates/sigil-cli/tests/sr_undeclared_gate.rs:85:5:
  left: Some(0)
 right: Some(1)
thread 'warnings_off_does_not_hide_the_refusal' (2742445) panicked at crates/sigil-cli/tests/sr_undeclared_gate.rs:100:5:
  left: Some(0)
 right: Some(1)
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.95s
error: test failed, to rerun pass `-p sigil-cli --test sr_undeclared_gate`
$ git restore --source=HEAD -- crates/sigil-cli/src/main.rs
$ git status --short --untracked-files=no
$ AEON_DIR=/home/volence/sonic_hacks/.aeon-a10-ref SIGIL_STRICT_GATE=1 CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-cli --test sr_undeclared_gate   # restored
test the_single_file_path_stays_a_warning ... ok
test a_declared_sr_write_builds ... ok
test warnings_off_does_not_hide_the_refusal ... ok
test an_undeclared_sr_write_stops_the_gated_build ... ok
test outside_the_gate_it_stays_a_warning ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.84s
```
