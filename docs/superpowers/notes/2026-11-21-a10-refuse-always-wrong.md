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

### Task 5, fix round 1: `[contract.binding-uncontracted]` reaches a full gated `sigil build`

Test: `crates/sigil-cli/tests/binding_uncontracted_gate.rs`. A copy of the reference tree has demo's `implement Game` bind `boot_hook` to `Zz_A10_Bound`, a proc planted in `demo_state.emp` with no `clobbers` clause. Mutation: the same as Task 5, the `refuse_uncontracted` call in `check_hook_sig` disabled with `if false &&`. The probe goes red because the mutated build succeeds; the control (the same binding with `clobbers()`, plus the unplanted tree) stays green.

```
$ git status --short
$ git diff --stat
 crates/sigil-frontend-emp/src/resolve/contract.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
$ grep -n 'refuse_uncontracted(iface' crates/sigil-frontend-emp/src/resolve/contract.rs
381:    if refuse_uncontracted(iface, member, bound_sym, "proc", all_items, span, diags) {
401:    if false && refuse_uncontracted(iface, member, bound_sym, "hook", all_items, span, diags) {
$ AEON_DIR=/home/volence/sonic_hacks/.aeon-a10-ref SIGIL_STRICT_GATE=1 CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-cli --test binding_uncontracted_gate   # mutated
test a_binding_to_a_proc_with_no_clause_stops_the_build ... FAILED
test the_same_binding_with_a_clause_builds_and_so_does_the_unplanted_tree ... ok
thread 'a_binding_to_a_proc_with_no_clause_stops_the_build' (2896397) panicked at crates/sigil-cli/tests/binding_uncontracted_gate.rs:42:5:
the build must refuse:
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.39s
error: test failed, to rerun pass `-p sigil-cli --test binding_uncontracted_gate`
$ git restore --source=HEAD -- crates/sigil-frontend-emp/src/resolve/contract.rs
$ git status --short
$ AEON_DIR=/home/volence/sonic_hacks/.aeon-a10-ref SIGIL_STRICT_GATE=1 CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-cli --test binding_uncontracted_gate   # restored
test a_binding_to_a_proc_with_no_clause_stops_the_build ... ok
test the_same_binding_with_a_clause_builds_and_so_does_the_unplanted_tree ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.42s
```

## Task 7: all shapes byte-identical, and the strict landing run

Candidate: `sigil 0.1.0 (32dc756a)`, built `CARGO_TARGET_DIR=.target-land cargo build --release --locked -p sigil-cli -p sigil-harness --bins`; base `.a10-base-1e200e1b/sigil`; free disk 342G.

### Step 2: BN, the seven shipped shapes

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

(sonic4 release/debug/config-a/config-b/lean, demo release/debug.)

### Step 3: the three fixture rows (copy of the reference tree, `--project games/sonic4 --shape`)

```
FIXTURE config-a-keystones-flipped SAME
FIXTURE stress-evict SAME
FIXTURE stress-art SAME
```

No `COULD NOT RUN`. Copy deleted.

### Step 4: aeon builds with the candidate (copy of the reference tree; SIGIL_BUILD and SIGIL_EMIT at `.target-land/release`)

```
./build.sh              exit 0   emp_expect_fail: OK, 56/56 cases (54 comptime + 2 link)
DEBUG=1 ./build.sh      exit 0   emp_expect_fail: OK, 56/56 cases (54 comptime + 2 link)
./build.sh demo         exit 0   emp_expect_fail: OK, 56/56 cases (54 comptime + 2 link)
DEBUG=1 ./build.sh demo exit 0   emp_expect_fail: OK, 56/56 cases (54 comptime + 2 link)
```

No `NO_LINT`/`FAST` line in any log. Copy deleted.

### Step 5: strict landing run

Tests added, counted from the diff: `git diff 1e200e1b..HEAD -- crates | grep -c '^+#\[test\]'` gives 28 (column 0); two more are indented inside `mod tests` in `crates/sigil-cli/src/main.rs` (the `gated_warning_refusal_*` pair), so 30 added, 0 deleted, 0 new `#[ignore]`. The plan's 28 plus the fix round's.

Command: `ORACLE_DIR=.../oracle-old landing-run.sh --baseline 6186 --aeon <cp -a of reference tree, HEAD 9b80db1d>`.

```
tree            .sigil-a10 @ 32dc756a (parcel/a10-refuse-always-wrong, clean)
reference       .a10-land-<pid> @ 9b80db1d (HEAD, clean), all four present
started/ended   2026-11-21T14:27:00Z -> 2026-11-21T14:48:01Z (UTC)
CARGO_EXIT      0
CLIPPY_EXIT     0   (lint bar clean)
LEDGER_EXIT     0   (ledger gate clean)
ci-shaped leg   0   (CI shape green: 563 suite(s), 6216 passed, 0 failed)
  suites          563
  passed          6216
  failed          0
  ignored         4
  skip lines      0
  binaries        563 launched, 563 reported
  test targets    563 expected launches (549 runnable + 14 doctest, 0 excluded for required-features; cargo metadata --no-deps), all launched
  reconciles      6186 baseline + 30 new = 6216 observed
  RESULT          GREEN
```

Note: the raw log carries `skip: ... SIGIL_ALLOW_PARTIAL-no-reference-tree-was-named` text lines; those are printed by tests that deliberately run with a nonexistent tree, and the gate's own `skip lines 0` is the verdict. No pin, golden or attestation was touched. `.target-land/tmp` and the copies deleted.

### Per-refusal evidence (probe test / control test / RF transcript above)

- `[proc.repeated-clause]`: probe `a_repeated_clobbers_clause_is_refused`, `every_clause_word_is_refused_when_repeated`, `a_repeated_clause_on_a_signature_is_refused`; control `each_clause_once_stays_legal`; RF "Task 1".
- `[name.duplicate-const]`: probe `a_const_declared_twice_is_refused_at_the_later_one`, `pub_typed_and_section_nested_duplicates_are_refused`; controls `distinct_names_and_other_kinds_stay_legal`, `a_const_beside_an_equ_of_the_same_name_is_not_this_refusal`; RF "Task 2".
- `[operand.bare-number]`: probe `a_bare_number_operand_is_refused`, `a_number_valued_expression_is_refused_too`; controls `the_immediate_and_the_sized_absolute_stay_legal`, `a_spliced_integer_stays_an_immediate` (bytes since the fix wave), `a_z80_bare_number_is_untouched`, `an_unrecognized_mnemonic_keeps_its_own_diagnostic` and `z80_operands.rs` `module_no_cpu_attr_defaults_m68000_and_z80_mnemonic_is_unrecognized` (the fix-wave guard); RF "Task 3" and "Final review fix wave".
- `[layout.odd-instruction]`: probe `an_instruction_after_an_odd_data_run_is_refused`, `as_compat_does_not_exempt_it`; controls `even_and_trailing_data_stay_legal`, `a_z80_body_is_untouched`; RF "Task 4".
- `[contract.binding-uncontracted]`: probe `probe_hook_bound_to_a_proc_with_no_clobbers_clause_is_refused`, `probe_proc_member_and_extern_targets_with_no_clause_are_refused`, gated `a_binding_to_a_proc_with_no_clause_stops_the_build`; controls `a_binding_to_a_proc_declaring_the_empty_clause_conforms`, `the_refusal_reaches_only_a_live_binding_to_a_declared_proc` (the limit pin), `the_same_binding_with_a_clause_builds_and_so_does_the_unplanted_tree`; RF "Task 5" and "Task 5, fix round 1".
- `[proc.sr-undeclared]` (gated build): probe `an_undeclared_sr_write_stops_the_gated_build`, `warnings_off_does_not_hide_the_refusal`, unit `gated_warning_refusal_lists_each_sr_firing_and_nothing_else`; controls `a_declared_sr_write_builds`, `outside_the_gate_it_stays_a_warning`, `the_single_file_path_stays_a_warning` (the limit pin), unit `gated_warning_refusal_is_empty_when_no_gated_id_fired`; RF "Task 6".

## Final review fix wave

The whole-branch review (`1e200e1b..b6b32362`) found two Important items and nine Minor ones. This section is
the fix wave's evidence; the booked residue is in `docs/QUEUE.md` (`STEP0-A10-REFUSALS`,
`A10-FINAL-REVIEW-RESIDUE`, `EMP-CONST-EQU-SAME-NAME`, `EMP-SPLICED-INT-OPERAND`, `G0B-TASK11-RESIDUE` item 2).

### I1: the unknown-mnemonic diagnostic wins over `[operand.bare-number]`

The refusal ran during operand mapping, before the mnemonic was looked up, and dropped the line, so
`ld a, 5` in a module without `cpu: z80`, `align 2` and `mvoe.w 5, d0` told the author to write `#`. The test
that pinned the first case had been edited (`ld a, 5` to `ld a, b`, plan Task 3 Step 5) so it no longer
exercised it. Now `lower_instr_to_item` computes
`refuse_bare_number = is_recognized_mnemonic(&mnemonic, Cpu::M68000)` once and passes it through
`map_operand` / `map_movem_operands` to `map_plain`; when it is false the operand falls through to
`classify_operand_splice` as on the base, and the lowering prints "not a recognized 68000 mnemonic". Every
recognised 68000 mnemonic (including the reserved `jbra`/`jbsr` and the multiply constructs) still refuses.

Tests written first (`z80_operands.rs` back to `ld a, 5`, plus the absence of the id;
`operand_bare_number.rs` `an_unrecognized_mnemonic_keeps_its_own_diagnostic` over `align 2` and
`mvoe.w 5, d0`), red before the code change:

```
test an_unrecognized_mnemonic_keeps_its_own_diagnostic ... FAILED
thread 'an_unrecognized_mnemonic_keeps_its_own_diagnostic' (2204443) panicked at crates/sigil-frontend-emp/tests/operand_bare_number.rs:105:9:
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test module_no_cpu_attr_defaults_m68000_and_z80_mnemonic_is_unrecognized ... FAILED
thread 'module_no_cpu_attr_defaults_m68000_and_z80_mnemonic_is_unrecognized' (2204690) panicked at crates/sigil-frontend-emp/tests/z80_operands.rs:251:5:
test result: FAILED. 37 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

RF for the guard, after its commit: mutation `(refuse_bare_number, v.as_stored_int())` becomes
`(true, v.as_stored_int())` in `map_plain`.

```
$ git status --short
$ git diff --stat
 crates/sigil-frontend-emp/src/eval/asm.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
$ grep -n "if let (true, Some(n))" crates/sigil-frontend-emp/src/eval/asm.rs
3576:        if let (true, Some(n)) = (true, v.as_stored_int()) {
$ cargo test --release -p sigil-frontend-emp --no-fail-fast --test operand_bare_number --test z80_operands
test an_unrecognized_mnemonic_keeps_its_own_diagnostic ... FAILED
thread 'an_unrecognized_mnemonic_keeps_its_own_diagnostic' (2250420) panicked at crates/sigil-frontend-emp/tests/operand_bare_number.rs:105:9:
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
error: test failed, to rerun pass `-p sigil-frontend-emp --test operand_bare_number`
test module_no_cpu_attr_defaults_m68000_and_z80_mnemonic_is_unrecognized ... FAILED
thread 'module_no_cpu_attr_defaults_m68000_and_z80_mnemonic_is_unrecognized' (2250518) panicked at crates/sigil-frontend-emp/tests/z80_operands.rs:251:5:
test result: FAILED. 37 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
error: test failed, to rerun pass `-p sigil-frontend-emp --test z80_operands`
error: 2 targets failed:
$ git restore --source=HEAD -- crates/sigil-frontend-emp/src/eval/asm.rs
$ git status --short
$ cargo test --release -p sigil-frontend-emp --no-fail-fast --test operand_bare_number --test z80_operands
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
```

Candidate against base, `sigil emp` on a one-line proc body (same rc, same output hash, one line each):

```
== ld a, 5        base rc=1 cf998a03   cand rc=1 cf998a03   `ld` is not a recognized 68000 mnemonic
== align 2        base rc=1 dee29123   cand rc=1 dee29123   `align` is not a recognized 68000 mnemonic
== mvoe.w 5, d0   base rc=1 c08b6c7b   cand rc=1 c08b6c7b   `mvoe` is not a recognized 68000 mnemonic
```

and the candidate still refuses `move.w $1234, d0` ("a bare number (4660)"), `move.w sizeof(u16), d0` (2),
`move.w 2+2, d0` (4) and `move.w K*2, d0` with `const K = 3` (6).

### M1, M2, M3

- M1: `proc_sig` (`parser.rs`) and `push_err` (`lower/code.rs`) get their doc comments back; the A10 helpers
  `note_contract_clause` and `first_odd_instr` had been inserted between each doc and its fn.
- M2: `out_width.rs` `a_narrow_type_on_an_address_result_is_refused_on_every_declaration_form` used
  `lea 4, a0` (now refused) in both arms; both are `lea (4).w, a0`, so the PERMITTED arm is on a source that
  builds.
- M3: `a_spliced_integer_stays_an_immediate` now lowers into a `(cpu: m68000, vma: $0)` section and asserts the
  linked bytes `30 3C 00 05 4E 75` (`move.w #5, d0` / `rts`), the pin `EMP-SPLICED-INT-OPERAND` cites.

## Owed and booked (from the controller's ledger, which is deleted after landing)

- **Aeon `docs/EMP_PITFALLS.md` §6 Trap B sentence, deliberately not written.** No pitfall is retired (none of
  the 14 entries describes one of the six constructs), so the card's third "Done means" line needs no edit. But
  lines 203-204, "(`move.w sr, dN` is fine on 68000 supervisor and does not trip `[proc.sr-undeclared]` — that
  fires only with SR as destination.)", become incomplete. The sentence owed (plan Task 8 Step 1): extend the
  parenthesis to end "— that fires only with SR as destination, and since sigil A10 a firing stops `./build.sh`
  (it stays a warning under `CONTRACTS=0` and for a single file)." It lands with the next aeon change that
  re-pins anyway (landing it alone costs a `--repin-aeon` and an attest). Booked in `STEP0-A10-REFUSALS`.
- **Pre-existing tests leave pid-suffixed folders under the target tmp dir:** `build_export_*`,
  `extra_entry_probe_*`, `lst_source_digest_*`. Out of A10's scope; booked in `G0B-TASK11-RESIDUE` item 2.
- **Width of `[operand.bare-number]`, for the owner.** Any integer-valued, non-path expression written without
  `#` in a 68000 operand position is refused, not only a literal: `move.w sizeof(u16), d0`, `move.w 2+2, d0`,
  `move.w K*2, d0`. Ruled to stand (below); aeon exposure 0 by build measure. Booked in `STEP0-A10-REFUSALS`.

## Rulings

One line per ruling in the controller's ledger.

- Start A10 now: the task card is owner-approved (2026-11-19, card 1), its only blocker was G0b landing, and the
  owner said to keep going; cost if wrong, a branch to discard (nothing merged or pushed).
- The plan's nine open questions are taken at the plan author's recommendation (table below): each is the narrow
  reading of the card; cost if wrong, each is a one-task follow-up. All nine go to the owner.
- Open question 6: Task 8 (the aeon pitfalls sentence) is not run now: no pitfall is retired, and landing it alone
  costs a `--repin-aeon`; booked above and in `STEP0-A10-REFUSALS`.
- Batching: three implementer dispatches (Tasks 0+1+2, 3+4, 5+6) then Task 7 alone, one review per batch; each task
  still has its own commit, RF transcript and BN line.
- The bare-number refusal's width (any integer-valued operand expression without `#`) stands: the same defect as
  the literal case, and aeon is unaffected by build measure; narrowing to literals is a small follow-up.
- Fix wave scope: I1, I2, M1, M2, M3 and the deferred lines marked FIX BEFORE MERGE are fixed; BOOK lines are booked in
  `docs/QUEUE.md`; M4-M9 are booked under `A10-FINAL-REVIEW-RESIDUE` unless trivially fixed in a file already open.

## Open questions (plan), and the recommendation taken

| # | Question | Taken |
|---|---|---|
| 1 | SR refusal: where does "the gate" promote it, and does the context form count? | (a): the gated `sigil build` refuses its own `[proc.sr-undeclared]` warnings after the link, both emission sites |
| 2 | Is a spliced integer (`move.w {n}, d0`) refused too? | Leave; booked `EMP-SPLICED-INT-OPERAND` |
| 3 | Does `@as_compat` exempt `[layout.odd-instruction]`? | Refuse (no exemption) |
| 4 | Duplicate const: does `const` against `equ` count? | `const`/`const` only; mixed pair booked `EMP-CONST-EQU-SAME-NAME` |
| 5 | Binding: is a target declared nowhere refused too? | Leave; booked `EMP-BINDING-TARGET-UNDECLARED` |
| 6 | Is the aeon pitfalls sentence (Task 8) wanted now? | With the next aeon change that re-pins; not written in A10 (owed, above) |
| 7 | Do the single-file paths get the five non-SR refusals? | All paths; only the SR one is limited to the gated build |
| 8 | AS front end | Leave for all six |
| 9 | Diagnostic wording and ids | As written |

## Final proof (fix-wave tip)

Recorded in the next commit, after the strict landing run at this tip.
