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
