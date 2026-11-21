---
kind: note
description: "A11 parcel note: base binary, BN on itself, the plan's baseline tables, and every task's RF transcript."
---
# A11 Complete the instruction set: parcel note

## Task 0: base binary

Built at `55c6ec32` (branch head `ec2ecaf4` carries no code change: `git diff --stat 55c6ec32 HEAD -- crates scripts contract Cargo.toml Cargo.lock` prints nothing) with `CARGO_TARGET_DIR=.target-land cargo build --release --locked -p sigil-cli -p sigil-harness --bins`, parked in `/home/volence/sonic_hacks/.a11-base-55c6ec32/`:

```
ed0292396f6cdd67e8e1c86d69bc8e8a7dd641acde9f33d51a8dd186bfe434d6  /home/volence/sonic_hacks/.a11-base-55c6ec32/sigil
42ecb50844f314e4446262de47e8b690838a41bcba01cbe594da0324d1369750  /home/volence/sonic_hacks/.a11-base-55c6ec32/emit_sound_blob
```

Reference tree `/home/volence/sonic_hacks/.aeon-a10-ref`: HEAD `9b80db1d`; ROMs `s4.bin 142a39b9/850262`, `s4.debug.bin 20acdb67/870606`, `demo.bin 4587df52/99980`, `demo.debug.bin 37d66535/106981` (zlib CRC-32 / size), as A10 provisioned. Reused, not re-provisioned.

BN on itself (`--base` and `--cand` both the parked base):

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

## Baseline measured for this plan (copied from the plan, 2026-11-21)

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

## RF transcripts
