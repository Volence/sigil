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

### Task 1: the two walks (code commit 456f9e64; mutations applied to that clean HEAD)

The walks are the plan's code blocks verbatim; they compiled and passed as given at the first try:
`emp_isa_walk_m68k` 3 passed, `emp_isa_walk_z80` 4 passed, `skip_marker_lint` 6 passed. Measured
counts (read by raising each floor by one, then restored from HEAD): the encoder accepts **2439**
68000 forms that decode to **1527** distinct instructions; Z80 **538** distinct encodings, **54** of
which carry a 16-bit field (24 immediates, 30 `(nn)`). Each equals the plan's floor exactly.

Each mutation below was made with `git status --short` clean, shown from disk with `git diff`, run,
and undone with `git restore --source=HEAD -- <file>`; after the last one both walks are green again
(`3 passed`, `4 passed`).

**RF-1: a refused form missing from the table fails, naming it.** Delete `("subx", 6)`:

```
 crates/sigil-harness/tests/emp_isa_walk_m68k.rs | 2 +-
-    ("abcd", 2), ("sbcd", 2), ("nbcd", 8), ("negx", 24), ("subx", 6), ("rtr", 1),
+    ("abcd", 2), ("sbcd", 2), ("nbcd", 8), ("negx", 24), ("rtr", 1),
```

`cargo test --release -p sigil-harness --test emp_isa_walk_m68k emp_assembles`:

```
assertion `left == right` failed: refused instructions per family differ from KNOWN_REFUSED:
  left: {"abcd": 2, "andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-from-usp": 1, "move-to-ccr": 11, "move-to-usp": 1, "movem": 4, "nbcd": 8, "negx": 24, "ori-sr": 1, "reset": 1, "rtr": 1, "sbcd": 2, "stop": 1, "subx": 6, "trapv": 1, "unlk": 1}
 right: {"abcd": 2, "andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-from-usp": 1, "move-to-ccr": 11, "move-to-usp": 1, "movem": 4, "nbcd": 8, "negx": 24, "ori-sr": 1, "reset": 1, "rtr": 1, "sbcd": 2, "stop": 1, "trapv": 1, "unlk": 1}
    "subx": [
        "`subx.b d1, d1`: `subx` is not a recognized 68000 mnemonic  ||  `subx d1, d1`: `subx` is not a recognized 68000 mnemonic",
        "`subx.b -(a5), -(a5)`: `subx` is not a recognized 68000 mnemonic  ||  `subx -(a5), -(a5)`: `subx` is not a recognized 68000 mnemonic",
        "`subx.l d1, d1`: `subx` is not a recognized 68000 mnemonic  ||  `subx d1, d1`: `subx` is not a recognized 68000 mnemonic",
        "`subx.l -(a5), -(a5)`: `subx` is not a recognized 68000 mnemonic  ||  `subx -(a5), -(a5)`: `subx` is not a recognized 68000 mnemonic",
        "`subx.w d1, d1`: `subx` is not a recognized 68000 mnemonic  ||  `subx d1, d1`: `subx` is not a recognized 68000 mnemonic",
        "`subx.w -(a5), -(a5)`: `subx` is not a recognized 68000 mnemonic  ||  `subx -(a5), -(a5)`: `subx` is not a recognized 68000 mnemonic",
    ],
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 4.86s
```

**RF-2: a table row that is no longer true fails, naming it.** Add a bogus `("tas", 1)` (`tas` assembles in `.emp`):

```
-    ("chk", 11), ("trapv", 1),
+    ("chk", 11), ("trapv", 1), ("tas", 1),
```

```
  left: {"abcd": 2, "andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-from-usp": 1, "move-to-ccr": 11, "move-to-usp": 1, "movem": 4, "nbcd": 8, "negx": 24, "ori-sr": 1, "reset": 1, "rtr": 1, "sbcd": 2, "stop": 1, "subx": 6, "trapv": 1, "unlk": 1}
 right: {"abcd": 2, "andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-from-usp": 1, "move-to-ccr": 11, "move-to-usp": 1, "movem": 4, "nbcd": 8, "negx": 24, "ori-sr": 1, "reset": 1, "rtr": 1, "sbcd": 2, "stop": 1, "subx": 6, "tas": 1, "trapv": 1, "unlk": 1}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 4.97s
```

`"tas": 1` is in `right` (the table) and absent from `left` (what the walk measured).

**RF-3 (brief step 4): a wrong count fails.** `("movem", 4)` → `("movem", 3)`:

```
-    ("move-to-usp", 1), ("move-from-usp", 1), ("move-to-ccr", 11), ("eori-ccr", 1), ("movem", 4),
+    ("move-to-usp", 1), ("move-from-usp", 1), ("move-to-ccr", 11), ("eori-ccr", 1), ("movem", 3),
```

```
  left: {..., "movem": 4, ...}
 right: {..., "movem": 3, ...}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 4.59s
```

**RF-4 (brief step 4): the Z80 symbol table.** Delete `("ld mem", 28)`:

```
 crates/sigil-harness/tests/emp_isa_walk_z80.rs | 1 -
-    ("ld mem", 28),
```

`cargo test --release -p sigil-harness --test emp_isa_walk_z80 every_16`:

```
  left: {"in mem": 1, "ld mem": 28, "out mem": 1}
 right: {"in mem": 1, "out mem": 1}
(28 `ld ... ZSym ...` lines listed; the first three:)
        "`ld (ZSym), hl`: unknown name `ZSym` | [lower.z80-unsupported] a Z80 indirect base must be `hl`/`bc`/`de`, an index register `(ix±d)`, or a comptime address `(nn)`, got poison | instruction dropped, a mnemonic, size, or operand did not resolve (a missing import or type in scope?)",
        "`ld (ZSym), hl`: unknown name `ZSym` | [lower.z80-unsupported] a Z80 indirect base must be `hl`/`bc`/`de`, an index register `(ix±d)`, or a comptime address `(nn)`, got poison | instruction dropped, a mnemonic, size, or operand did not resolve (a missing import or type in scope?)",
        "`ld hl, (ZSym)`: unknown name `ZSym` | [lower.z80-unsupported] a Z80 indirect base must be `hl`/`bc`/`de`, an index register `(ix±d)`, or a comptime address `(nn)`, got poison | instruction dropped, a mnemonic, size, or operand did not resolve (a missing import or type in scope?)",
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.05s
```

**The 80 refused 68000 instructions at `55c6ec32`** (family, sized / bare spelling, first diagnostic),
from the RF-1 run's full dump:

```
abcd           `abcd.b d1, d1` / `abcd d1, d1` -> `abcd` is not a recognized 68000 mnemonic
abcd           `abcd.b -(a5), -(a5)` / `abcd -(a5), -(a5)` -> `abcd` is not a recognized 68000 mnemonic
andi-sr        `andi.w #1, sr` / `andi #1, sr` -> unsupported form: sr is not a general EA
chk            `chk.w ($00FF8000).l, d1` / `chk ($00FF8000).l, d1` -> `chk` is not a recognized 68000 mnemonic
chk            `chk.w ($1234).w, d1` / `chk ($1234).w, d1` -> `chk` is not a recognized 68000 mnemonic
chk            `chk.w 8(a1), d1` / `chk 8(a1), d1` -> `chk` is not a recognized 68000 mnemonic
chk            `chk.w 4(a2,d3.w), d1` / `chk 4(a2,d3.w), d1` -> `chk` is not a recognized 68000 mnemonic
chk            `chk.w d1, d1` / `chk d1, d1` -> `chk` is not a recognized 68000 mnemonic
chk            `chk.w #1, d1` / `chk #1, d1` -> `chk` is not a recognized 68000 mnemonic
chk            `chk.w (a3), d1` / `chk (a3), d1` -> `chk` is not a recognized 68000 mnemonic
chk            `chk.w .tgt(pc), d1` / `chk .tgt(pc), d1` -> `chk` is not a recognized 68000 mnemonic
chk            `chk.w .tgt(pc,d0.w), d1` / `chk .tgt(pc,d0.w), d1` -> `chk` is not a recognized 68000 mnemonic
chk            `chk.w (a4)+, d1` / `chk (a4)+, d1` -> `chk` is not a recognized 68000 mnemonic
chk            `chk.w -(a5), d1` / `chk -(a5), d1` -> `chk` is not a recognized 68000 mnemonic
eori-ccr       `eori.b #1, ccr` / `eori #1, ccr` -> unsupported form: ccr is not a general EA
eori-sr        `eori.w #1, sr` / `eori #1, sr` -> unsupported form: sr is not a general EA
link           `link.w a2, #1` / `link a2, #1` -> `link` is not a recognized 68000 mnemonic
move-from-usp  `move.l usp, a2` / `move usp, a2` -> [operand.usp-unsupported] `usp` is the 68000 user stack pointer, never a symbol; `.emp` has no `move` to or fr
move-to-ccr    `move.w ($00FF8000).l, ccr` / `move ($00FF8000).l, ccr` -> unsupported form: ccr is not a general EA
move-to-ccr    `move.w ($1234).w, ccr` / `move ($1234).w, ccr` -> unsupported form: ccr is not a general EA
move-to-ccr    `move.w 8(a1), ccr` / `move 8(a1), ccr` -> unsupported form: ccr is not a general EA
move-to-ccr    `move.w 4(a2,d3.w), ccr` / `move 4(a2,d3.w), ccr` -> unsupported form: ccr is not a general EA
move-to-ccr    `move.w d1, ccr` / `move d1, ccr` -> unsupported form: ccr is not a general EA
move-to-ccr    `move.w #1, ccr` / `move #1, ccr` -> unsupported form: ccr is not a general EA
move-to-ccr    `move.w (a3), ccr` / `move (a3), ccr` -> unsupported form: ccr is not a general EA
move-to-ccr    `move.w .tgt(pc), ccr` / `move .tgt(pc), ccr` -> unsupported form: ccr is not a general EA
move-to-ccr    `move.w .tgt(pc,d0.w), ccr` / `move .tgt(pc,d0.w), ccr` -> unsupported form: ccr is not a general EA
move-to-ccr    `move.w (a4)+, ccr` / `move (a4)+, ccr` -> unsupported form: ccr is not a general EA
move-to-ccr    `move.w -(a5), ccr` / `move -(a5), ccr` -> unsupported form: ccr is not a general EA
move-to-usp    `move.l a2, usp` / `move a2, usp` -> [operand.usp-unsupported] `usp` is the 68000 user stack pointer, never a symbol; `.emp` has no `move` to or fr
movem          `movem.l .tgt(pc), d0/a0/a1` / `movem .tgt(pc), d0/a0/a1` -> internal: a PC-relative operand reached the generic operand mapper
movem          `movem.l .tgt(pc,d0.w), d0/a0/a1` / `movem .tgt(pc,d0.w), d0/a0/a1` -> internal: a PC-relative operand reached the generic operand mapper
movem          `movem.w .tgt(pc), d0/a0/a1` / `movem .tgt(pc), d0/a0/a1` -> internal: a PC-relative operand reached the generic operand mapper
movem          `movem.w .tgt(pc,d0.w), d0/a0/a1` / `movem .tgt(pc,d0.w), d0/a0/a1` -> internal: a PC-relative operand reached the generic operand mapper
nbcd           `nbcd.b ($00FF8000).l` / `nbcd ($00FF8000).l` -> `nbcd` is not a recognized 68000 mnemonic
nbcd           `nbcd.b ($1234).w` / `nbcd ($1234).w` -> `nbcd` is not a recognized 68000 mnemonic
nbcd           `nbcd.b 8(a1)` / `nbcd 8(a1)` -> `nbcd` is not a recognized 68000 mnemonic
nbcd           `nbcd.b 4(a2,d3.w)` / `nbcd 4(a2,d3.w)` -> `nbcd` is not a recognized 68000 mnemonic
nbcd           `nbcd.b d1` / `nbcd d1` -> `nbcd` is not a recognized 68000 mnemonic
nbcd           `nbcd.b (a3)` / `nbcd (a3)` -> `nbcd` is not a recognized 68000 mnemonic
nbcd           `nbcd.b (a4)+` / `nbcd (a4)+` -> `nbcd` is not a recognized 68000 mnemonic
nbcd           `nbcd.b -(a5)` / `nbcd -(a5)` -> `nbcd` is not a recognized 68000 mnemonic
negx           `negx.b ($00FF8000).l` / `negx ($00FF8000).l` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.b ($1234).w` / `negx ($1234).w` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.b 8(a1)` / `negx 8(a1)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.b 4(a2,d3.w)` / `negx 4(a2,d3.w)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.b d1` / `negx d1` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.b (a3)` / `negx (a3)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.b (a4)+` / `negx (a4)+` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.b -(a5)` / `negx -(a5)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.l ($00FF8000).l` / `negx ($00FF8000).l` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.l ($1234).w` / `negx ($1234).w` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.l 8(a1)` / `negx 8(a1)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.l 4(a2,d3.w)` / `negx 4(a2,d3.w)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.l d1` / `negx d1` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.l (a3)` / `negx (a3)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.l (a4)+` / `negx (a4)+` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.l -(a5)` / `negx -(a5)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.w ($00FF8000).l` / `negx ($00FF8000).l` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.w ($1234).w` / `negx ($1234).w` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.w 8(a1)` / `negx 8(a1)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.w 4(a2,d3.w)` / `negx 4(a2,d3.w)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.w d1` / `negx d1` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.w (a3)` / `negx (a3)` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.w (a4)+` / `negx (a4)+` -> `negx` is not a recognized 68000 mnemonic
negx           `negx.w -(a5)` / `negx -(a5)` -> `negx` is not a recognized 68000 mnemonic
ori-sr         `ori.w #1, sr` / `ori #1, sr` -> unsupported form: sr is not a general EA
reset          `reset.w ` / `reset ` -> `reset` is not a recognized 68000 mnemonic
rtr            `rtr.w ` / `rtr ` -> `rtr` is not a recognized 68000 mnemonic
sbcd           `sbcd.b d1, d1` / `sbcd d1, d1` -> `sbcd` is not a recognized 68000 mnemonic
sbcd           `sbcd.b -(a5), -(a5)` / `sbcd -(a5), -(a5)` -> `sbcd` is not a recognized 68000 mnemonic
stop           `stop.w #1` / `stop #1` -> `stop` is not a recognized 68000 mnemonic
subx           `subx.b d1, d1` / `subx d1, d1` -> `subx` is not a recognized 68000 mnemonic
subx           `subx.b -(a5), -(a5)` / `subx -(a5), -(a5)` -> `subx` is not a recognized 68000 mnemonic
subx           `subx.l d1, d1` / `subx d1, d1` -> `subx` is not a recognized 68000 mnemonic
subx           `subx.l -(a5), -(a5)` / `subx -(a5), -(a5)` -> `subx` is not a recognized 68000 mnemonic
subx           `subx.w d1, d1` / `subx d1, d1` -> `subx` is not a recognized 68000 mnemonic
subx           `subx.w -(a5), -(a5)` / `subx -(a5), -(a5)` -> `subx` is not a recognized 68000 mnemonic
trapv          `trapv.w ` / `trapv ` -> `trapv` is not a recognized 68000 mnemonic
unlk           `unlk.w a2` / `unlk a2` -> `unlk` is not a recognized 68000 mnemonic
```

TC `sigil-harness` (after the last edit): 69 suites, 511 passed, 0 failed, 2 ignored (both pre-existing:
`secondary_pin_classes_match_the_hand_typed_baseline` retired, `measure_real_sound_tables_z80_through_seam2`
needs `SEAM2_REGION_AEON`). Clippy `--workspace --all-targets -D warnings`: clean. BN:

```
SHAPES 7/7 IDENTICAL
STEP0_CMP_END rc=0
```

### Task 2: `abcd`, `sbcd`, `nbcd`, `negx`, `subx`, bare `exg` (code commit df161434)

The brief's code blocks compiled and ran as given.

**RF step 1, red before implementing.** Rows `("abcd", 2), ("sbcd", 2), ("nbcd", 8), ("negx", 24), ("subx", 6)`
deleted from `KNOWN_REFUSED`, `emp_a11_golden.rs` and `a11_checks.rs` created, nothing else changed:

```
test exg_assembles_with_and_without_its_suffix ... FAILED
test the_extended_and_decimal_arithmetic_assembles ... FAILED
thread 'exg_assembles_with_and_without_its_suffix' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:53:9:
assertion `left == right` failed: `.emp`, `exg d0, d1`
  left: Err(["instruction needs an explicit size suffix (.b/.w/.l)"])
 right: Ok([78, 113, 193, 65, 78, 117])
thread 'the_extended_and_decimal_arithmetic_assembles' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:38:5:
assertion `left == right` failed: `.emp`, `abcd d0, d1`
  left: Err(["`abcd` is not a recognized 68000 mnemonic"])
 right: Ok([78, 113, 195, 0, 78, 117])
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
test emp_assembles_every_68000_instruction_the_shared_encoder_accepts ... FAILED
assertion `left == right` failed: refused instructions per family differ from KNOWN_REFUSED:
  left: {"abcd": 2, "andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-from-usp": 1, "move-to-ccr": 11, "move-to-usp": 1, "movem": 4, "nbcd": 8, "negx": 24, "ori-sr": 1, "reset": 1, "rtr": 1, "sbcd": 2, "stop": 1, "subx": 6, "trapv": 1, "unlk": 1}
 right: {"andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-from-usp": 1, "move-to-ccr": 11, "move-to-usp": 1, "movem": 4, "ori-sr": 1, "reset": 1, "rtr": 1, "stop": 1, "trapv": 1, "unlk": 1}
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
test the_new_arithmetic_writes_its_destination_register ... FAILED
thread 'the_new_arithmetic_writes_its_destination_register' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:30:5:
[(true, "`abcd` is not a recognized 68000 mnemonic")]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

(The walk's dump listed the 42 refused forms of the five families, each `` `x` is not a recognized 68000 mnemonic ``, as in Task 1's table.)

**RF step 2, green.** Same three files after the wiring: golden 2 passed, walk 3 passed, checks 1 passed.

**RF step 3, mutation on the clean committed HEAD.** `git status --short` empty; comment out the `subx` line:

```
 crates/sigil-frontend-emp/src/lower/code.rs | 2 +-
-        "subx" => Subx,
+        // "subx" => Subx,
```

`cargo test --release -p sigil-harness --test emp_isa_walk_m68k --test emp_a11_golden --no-fail-fast`:

```
test exg_assembles_with_and_without_its_suffix ... ok
test the_extended_and_decimal_arithmetic_assembles ... FAILED
thread 'the_extended_and_decimal_arithmetic_assembles' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:45:5:
  left: Err(["`subx` is not a recognized 68000 mnemonic"])
 right: Ok([78, 113, 147, 64, 78, 117])
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
test emp_assembles_every_68000_instruction_the_shared_encoder_accepts ... FAILED
    "subx": [ ... 6 rows, e.g. "`subx.w -(a5), -(a5)`: `subx` is not a recognized 68000 mnemonic  ||  `subx -(a5), -(a5)`: ..." ],
  left: {..., "stop": 1, "subx": 6, "trapv": 1, "unlk": 1}
 right: {..., "stop": 1, "trapv": 1, "unlk": 1}
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

`git restore --source=HEAD -- crates/sigil-frontend-emp/src/lower/code.rs`, re-run: golden 2 passed, walk 3 passed.

TC (after the last code edit): `sigil-frontend-emp` 148 suites, 2868 passed, 0 failed, 0 ignored;
`sigil-cli --test frontend_differential` 7 passed; `sigil-harness --test emp_isa_walk_m68k --test emp_a11_golden
--test frontend_differential_coverage` 3 suites, 6 passed. Clippy clean. BN: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.

### Task 3: `rtr` (code commit 9095953f)

The brief's code blocks compiled and ran as given. One placement note: the `rtr` block in `preserves.rs`
`transfer` went immediately before the `// FRAME OPEN / CLOSE` comment that heads `if mnem == "link" {`,
not between that comment and its `if` (the plan's "insert before the item's comment" rule).

**RF step 1, red before implementing.** `("rtr", 1)` deleted from `KNOWN_REFUSED`, the two golden tests and
the check test appended, nothing else changed:

```
test rtr_assembles ... FAILED
test a_size_suffix_the_instruction_lacks_is_refused ... FAILED
thread 'rtr_assembles' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:63:5:
  left: Err(["`rtr` is not a recognized 68000 mnemonic"])
 right: Ok([78, 113, 78, 119, 78, 117])
thread 'a_size_suffix_the_instruction_lacks_is_refused' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:71:9:
`rtr.w`: ["`rtr` is not a recognized 68000 mnemonic"]
test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
test emp_assembles_every_68000_instruction_the_shared_encoder_accepts ... FAILED
    "rtr": [
        "`rtr.w `: `rtr` is not a recognized 68000 mnemonic  ||  `rtr `: `rtr` is not a recognized 68000 mnemonic",
    ],
  left: {"andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-from-usp": 1, "move-to-ccr": 11, "move-to-usp": 1, "movem": 4, "ori-sr": 1, "reset": 1, "rtr": 1, "stop": 1, "trapv": 1, "unlk": 1}
 right: {"andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-from-usp": 1, "move-to-ccr": 11, "move-to-usp": 1, "movem": 4, "ori-sr": 1, "reset": 1, "stop": 1, "trapv": 1, "unlk": 1}
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
test rtr_ends_a_proc_and_pops_the_saved_ccr_word ... FAILED
thread 'rtr_ends_a_proc_and_pops_the_saved_ccr_word' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:43:5:
  left: [(true, "`rtr` is not a recognized 68000 mnemonic"), (false, "[proc.undeclared-fallthrough] `P` can reach its closing `}` without an unconditional terminator and does not declare `falls_into`, it will run into whatever follows it"), (true, "[stack.unbalanced] in `P`: this path returns with 2 bytes still on the stack, sp is below its entry value, so the return reads its address from the wrong word")]
 right: []
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

The red already shows both wrong verdicts on correct code (Review Focus 2): the fall-through warning and
the 2-byte `[stack.unbalanced]` error.

**RF step 2, green.** golden 4 passed, walk 3 passed, `a11_checks` 2 passed, `as_author_forms_stay_off_emp` 4 passed.

**RF step 3, mutation 1 on the clean committed HEAD** (`git status --short` empty). Delete `"rtr"` from `is_terminator`:

```
 crates/sigil-frontend-emp/src/lower/proc.rs | 2 +-
-        Cpu::M68000 => matches!(mnemonic, "rts" | "rtr" | "rte" | "bra" | "jmp" | "jbra" | "illegal"),
+        Cpu::M68000 => matches!(mnemonic, "rts" | "rte" | "bra" | "jmp" | "jbra" | "illegal"),
```

`cargo test --release -p sigil-frontend-emp --test a11_checks --no-fail-fast`:

```
test rtr_ends_a_proc_and_pops_the_saved_ccr_word ... FAILED
thread 'rtr_ends_a_proc_and_pops_the_saved_ccr_word' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:43:5:
  left: [(false, "[proc.undeclared-fallthrough] `P` can reach its closing `}` without an unconditional terminator and does not declare `falls_into`, it will run into whatever follows it")]
 right: []
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Restored with `git restore --source=HEAD -- crates/sigil-frontend-emp/src/lower/proc.rs`: 2 passed.

**Mutation 2.** `apply_sp_cleanup(st, 2)` to `(st, 0)`:

```
 crates/sigil-frontend-emp/src/preserves.rs | 2 +-
-        return apply_sp_cleanup(st, 2);
+        return apply_sp_cleanup(st, 0);
```

```
test rtr_ends_a_proc_and_pops_the_saved_ccr_word ... FAILED
thread 'rtr_ends_a_proc_and_pops_the_saved_ccr_word' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:43:5:
  left: [(true, "[stack.unbalanced] in `P`: this path returns with 2 bytes still on the stack, sp is below its entry value, so the return reads its address from the wrong word")]
 right: []
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Restored with `git restore --source=HEAD -- crates/sigil-frontend-emp/src/preserves.rs`: 2 passed.

TC (after the last code edit): `sigil-frontend-emp` 148 suites, 2869 passed, 0 failed, 0 ignored;
`sigil-cli --test frontend_differential` 7 passed; `sigil-harness --test emp_isa_walk_m68k --test emp_a11_golden
--test frontend_differential_coverage` 3 suites, 8 passed. Clippy clean. BN: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.

### Task 4: `move` to and from `usp` (code commit d87bddf4)

The brief's code blocks compiled. One differs from the brief, because the brief contradicted itself: Step 5's
guard required `size == Some(Width::L)`, so bare `move a2, usp` was refused `[operand.usp-unsupported]`, while
Step 7 moves both families into `SIZE_DEFAULT_GAP_FAMILIES`, whose row matches only `instruction needs an explicit
size suffix (.b/.w/.l)`. The differential printed exactly that:

```
test emp_and_as_front_ends_agree_on_every_generated_m68k_case ... FAILED
2 front-end disagreement(s) not covered by EXCLUSIONS:
  `move a2, usp`: emp refused ["[operand.usp-unsupported] `usp` is the 68000 user stack pointer, an operand of `move.l aN, usp` and `move.l usp, aN` only"] | AS bytes 4E714E624E75
  `move usp, a4`: emp refused ["[operand.usp-unsupported] `usp` is the 68000 user stack pointer, an operand of `move.l aN, usp` and `move.l usp, aN` only"] | AS bytes 4E714E6C4E75
```

No later task closes it (Tasks 6-12 do not touch `usp` or this row). `is_usp_move` now takes
`matches!(size, Some(Width::L) | None)`: an unsized `move aN, usp` passes the guard and is refused for its missing
suffix, as every `move` is. Still refused, never a symbol; every other `usp` operand keeps the named refusal.

Probed beside the guard (frontend_diff link environment, `usp = $1234`; not committed as a test): `jsr usp`,
`jmp usp`, `bra.w usp`, `bsr.s usp`, `beq.s usp`, `dbra d0, usp`, `pea usp`, `movem.l usp, d0-d1` all refuse
`[operand.usp-unsupported]`; `move.l usp, a7` / `move.l sp, usp` assemble `4E6F` / `4E67`. One hole that predates
this task: `move.l #usp, d0` assembles `203C 0000 1234` on BOTH routes (the immediate evaluator reads the
symbol). Booked `EMP-IMM-REGISTER-WORD`.

**RF step 1, red before implementing.** `("move-to-usp", 1), ("move-from-usp", 1)` deleted from `KNOWN_REFUSED`,
`the_usp_moves_assemble` and `usp_is_an_operand_of_the_two_moves_only` appended, nothing else changed:

```
test the_usp_moves_assemble ... FAILED
thread 'the_usp_moves_assemble' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:79:5:
assertion `left == right` failed: `.emp`, `move.l a0, usp`
  left: Err(["[operand.usp-unsupported] `usp` is the 68000 user stack pointer, never a symbol; `.emp` has no `move` to or from `usp` yet (EMP-USP-CCR-SURFACE)", "instruction dropped, a mnemonic, size, or operand did not resolve (a missing import or type in scope?)"])
 right: Ok([78, 113, 78, 96, 78, 117])
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out
test emp_assembles_every_68000_instruction_the_shared_encoder_accepts ... FAILED
    "move-from-usp": [
        "`move.l usp, a2`: [operand.usp-unsupported] ... `.emp` has no `move` to or from `usp` yet (EMP-USP-CCR-SURFACE) | instruction dropped, ...  ||  `move usp, a2`: [operand.usp-unsupported] ...",
    ],
    "move-to-usp": [
        "`move.l a2, usp`: [operand.usp-unsupported] ... | instruction dropped, ...  ||  `move a2, usp`: [operand.usp-unsupported] ...",
    ],
  left: {"andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-from-usp": 1, "move-to-ccr": 11, "move-to-usp": 1, "movem": 4, "ori-sr": 1, "reset": 1, "stop": 1, "trapv": 1, "unlk": 1}
 right: {"andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-to-ccr": 11, "movem": 4, "ori-sr": 1, "reset": 1, "stop": 1, "trapv": 1, "unlk": 1}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out
test usp_is_an_operand_of_the_two_moves_only ... FAILED
thread 'usp_is_an_operand_of_the_two_moves_only' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:51:5:
assertion `left == right` failed
  left: [(true, "[operand.usp-unsupported] `usp` is the 68000 user stack pointer, never a symbol; `.emp` has no `move` to or from `usp` yet (EMP-USP-CCR-SURFACE)"), (true, "instruction dropped, a mnemonic, size, or operand did not resolve (a missing import or type in scope?)")]
 right: []
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out
```

**RF step 2, green.** golden 5 passed, walk 3 passed, `a11_checks` 3 passed, `usp_reserved` 6 passed, differential 7 passed.

**RF step 3, mutation on the clean committed HEAD** (`git status --short` empty). The guard's condition to `false`:

```
 crates/sigil-frontend-emp/src/lower/code.rs | 2 +-
-    if ops.iter().any(|o| matches!(o, CodeOperand::Usp)) && !is_usp_move(mnemonic, size, ops) {
+    if ops.iter().any(|o| matches!(o, CodeOperand::Usp)) && false {
```

`cargo test --release -p sigil-frontend-emp --test a11_checks usp_is`:

```
test usp_is_an_operand_of_the_two_moves_only ... FAILED
thread 'usp_is_an_operand_of_the_two_moves_only' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:56:9:
`move.l usp, d0`: [(true, "unsupported form: MoveFromUsp requires An,usp / usp,An operands, got [Usp, Dn(0)]")]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out
```

Red on `move.l usp, d0`, as predicted. Without the guard the encoder still refuses it, with an unnamed message:
the guard is what makes it the named `[operand.usp-unsupported]`. Restored with
`git restore --source=HEAD -- crates/sigil-frontend-emp/src/lower/code.rs`: 1 passed.

TC (after the last code edit): `sigil-frontend-emp` 150 suites, 2870 passed, 0 failed, 0 ignored;
`sigil-cli --test frontend_differential` 7 passed; `sigil-harness --test emp_isa_walk_m68k --test emp_a11_golden
--test frontend_differential_coverage` 3 suites, 9 passed. Clippy clean. BN: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.

### Task 5: `move <ea>, ccr`, `eori #, ccr`, `movem` from a PC-relative source (code commit bfb520d3)

The brief's code blocks compiled and ran as given (the `movem` block's locals `mem_op`, `mask`, `list_first`,
`size` exist under those names). One addition the brief did not list: two check tests in `a11_checks.rs`, because
the plan requires a test of what the checks say for every new instruction and the brief gave only golden vectors.
What the checks say, probed and then pinned: a CCR-only write is a flag write, not a mask write.
`[proc.sr-undeclared]` stays silent for `move.w d0, ccr`, `move.w #$1F, ccr` and `eori.b #$01, ccr` under
`clobbers()`, while `move.w d0, sr` fires; `preserves(sr.ccr)` refuses each unbracketed write
`[proc.preserves-unverifiable]`; bare `preserves(sr)` warns `[proc.ccr-advisory]`; inside the
`move.w sr, -(sp)` ... `move.w (sp)+, sr` bracket all are clean. The flag tables (`carry_role` / `zero_role` in
`flag_check.rs`) already answered both forms (`Move` to ccr `Writes`; `EoriCcr` by its immediate, e.g. `#$01`
inverts C). A `movem` load from `.t(pc)` writes its list (`[proc.clobber-undeclared]` names `d1` under `clobbers(d0)`).
`lower_module` does not run `[call.flag-result-unused]`, so no module-level test of it was possible here.

FD: the `emp-lacks-sr-ccr-form` predicate still accepts `unsupported form: ccr is not a general EA` as an
alternative; the brief named only the families and the prose, so it stays (no family it now covers can produce it).

**RF step 1, red before implementing.** `("move-to-ccr", 11), ("eori-ccr", 1), ("movem", 4)` deleted from
`KNOWN_REFUSED`, the two golden tests appended:

```
test the_ccr_forms_assemble ... FAILED
test movem_loads_from_a_pc_relative_source ... FAILED
thread 'the_ccr_forms_assemble' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:86:5:
assertion `left == right` failed: `.emp`, `move.w d0, ccr`
  left: Err(["unsupported form: ccr is not a general EA"])
 right: Ok([78, 113, 68, 192, 78, 117])
thread 'movem_loads_from_a_pc_relative_source' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:94:5:
assertion `left == right` failed
  left: Err(["internal: a PC-relative operand reached the generic operand mapper"])
 right: Ok([78, 113, 76, 186, 0, 3, 255, 250, 78, 117])
test result: FAILED. 5 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
test emp_assembles_every_68000_instruction_the_shared_encoder_accepts ... FAILED
    "eori-ccr": [
        "`eori.b #1, ccr`: unsupported form: ccr is not a general EA  ||  `eori #1, ccr`: instruction needs an explicit size suffix (.b/.w/.l)",
    ],
    "move-to-ccr": [ 11 rows, e.g.
        "`move.w d1, ccr`: unsupported form: ccr is not a general EA  ||  `move d1, ccr`: instruction needs an explicit size suffix (.b/.w/.l)",
    ],
    "movem": [ 4 rows, e.g.
        "`movem.l .tgt(pc), d0/a0/a1`: internal: a PC-relative operand reached the generic operand mapper  ||  `movem .tgt(pc), d0/a0/a1`: movem needs an explicit size suffix (.w or .l)",
    ],
  left: {"andi-sr": 1, "chk": 11, "eori-ccr": 1, "eori-sr": 1, "link": 1, "move-to-ccr": 11, "movem": 4, "ori-sr": 1, "reset": 1, "stop": 1, "trapv": 1, "unlk": 1}
 right: {"andi-sr": 1, "chk": 11, "eori-sr": 1, "link": 1, "ori-sr": 1, "reset": 1, "stop": 1, "trapv": 1, "unlk": 1}
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

The two check tests and the `as_author_forms_stay_off_emp` edit were written after the implementation (after
probing what the checks say). Their red was then shown against the pre-implementation `code.rs` (the edited file
copied aside, `code.rs` restored from the Task 4 HEAD, tests run, the copy put back):

```
test movem_from_a_pc_relative_source_writes_its_list ... FAILED
test the_ccr_writes_are_flag_writes_not_mask_writes ... FAILED
thread 'movem_from_a_pc_relative_source_writes_its_list' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:90:5:
  left: [(true, "internal: a PC-relative operand reached the generic operand mapper")]
 right: []
thread 'the_ccr_writes_are_flag_writes_not_mask_writes' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:73:9:
assertion `left == right` failed: `move.w d0, ccr` under clobbers()
  left: [(true, "unsupported form: ccr is not a general EA")]
 right: []
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
thread 'the_other_as_rewrites_do_not_reach_emp' panicked at crates/sigil-frontend-emp/tests/as_author_forms_stay_off_emp.rs:41:5:
assertion `left == right` failed: `.emp` answer for `eori.b #1,ccr` moved
  left: ["unsupported form: ccr is not a general EA"]
 right: []
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

**RF step 2, green.** golden 7 passed, walk 3 passed, `a11_checks` 5 passed, `as_author_forms_stay_off_emp` 4
passed, differential 7 passed.

**RF step 3, mutation on the clean committed HEAD** (`git status --short` empty). Delete the `MoveToCcr` arm:

```
 crates/sigil-frontend-emp/src/lower/code.rs | 1 -
-        (Move, [_, M68kOperand::Ccr]) => MoveToCcr,
```

`cargo test --release -p sigil-harness --test emp_a11_golden --test emp_isa_walk_m68k --no-fail-fast`:

```
test the_ccr_forms_assemble ... FAILED
thread 'the_ccr_forms_assemble' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:86:5:
assertion `left == right` failed: `.emp`, `move.w d0, ccr`
  left: Err(["unsupported form: ccr is not a general EA"])
 right: Ok([78, 113, 68, 192, 78, 117])
test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
test emp_assembles_every_68000_instruction_the_shared_encoder_accepts ... FAILED
  left: {"andi-sr": 1, "chk": 11, "eori-sr": 1, "link": 1, "move-to-ccr": 11, "ori-sr": 1, "reset": 1, "stop": 1, "trapv": 1, "unlk": 1}
 right: {"andi-sr": 1, "chk": 11, "eori-sr": 1, "link": 1, "ori-sr": 1, "reset": 1, "stop": 1, "trapv": 1, "unlk": 1}
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

As predicted: `the_ccr_forms_assemble` and the walk (`move-to-ccr`, 11). Restored with
`git restore --source=HEAD -- crates/sigil-frontend-emp/src/lower/code.rs`: golden 7 passed, walk 3 passed.

**Mutation 2 (extra, for the `movem` block).** Its guard to `false`:

```
 crates/sigil-frontend-emp/src/lower/code.rs | 2 +-
-    if matches!(mem_op, CodeOperand::PcRel { .. } | CodeOperand::PcRelIdx { .. }) {
+    if false {
```

```
test movem_loads_from_a_pc_relative_source ... FAILED
  left: Err(["internal: a PC-relative operand reached the generic operand mapper"])
 right: Ok([78, 113, 76, 186, 0, 3, 255, 250, 78, 117])
test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
test movem_from_a_pc_relative_source_writes_its_list ... FAILED
  left: [(true, "internal: a PC-relative operand reached the generic operand mapper")]
 right: []
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Restored from HEAD: golden 7 passed, `a11_checks` 5 passed.

TC (after the last code edit): `sigil-frontend-emp` 150 suites, 2872 passed, 0 failed, 0 ignored;
`sigil-cli --test frontend_differential` 7 passed; `sigil-harness --test emp_isa_walk_m68k --test emp_a11_golden
--test frontend_differential_coverage` 3 suites, 11 passed. Clippy clean. BN: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.

### Batch B fix round 1 (Tasks 2-3 review; commits 0d7470d3 rtr, 1dc70ae2 rte)

**I1: `rtr` replaced the condition codes, but `out_verify` called it transparent.** Red before the fix
(the reviewer's two probes in `a11_checks.rs`, and the production half in `tests/out_verify.rs`):

```
test rtr_replaces_the_condition_codes_a_conditional_out_reads ... FAILED
panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:103:5:
  left: [(true, "[proc.out-cond-survives-unverifiable] `P` declares `out(d0 if eq)` with `d0` absent from `clobbers(...)`, which claims `d0` still holds its entry value on every `!eq` return path, but `d0` does not provably hold its entry value on a `!eq` exit (written and not restored, or destroyed by a call / tail transfer there). Add `d0` to `clobbers(...)` if it is destroyed on the failure edges (the conditional result stands either way), or save/restore it across them")]
 right: []
test rtr_does_not_carry_the_computed_flags_to_the_caller ... FAILED
panicked at crates/sigil-frontend-emp/tests/out_verify.rs:1150:5:
the cc at `rtr` is the saved word's, unknown → Unverified, got Produced
```

(`right: []` is the `tst.w d1` probe, which was already silent.) Fix: `rtr` out of `is_branch_or_return`.
After it, both probes print nothing (`[]`). That is right: the return's flags are the saved word's,
unknown to the checker, and the survives half is obligated only on provably-`!eq` exits (the documented
"unknown does not obligate" rule in `not_cc_exit_sites`); the production half now OWES `d0` at that
return (Unverified, the sigil-build `[proc.out-unverified]` input), which is the unsafe direction closed.

**Minor 1: `rtr` closes the `preserves(sr.ccr)` bracket.** Red before the fix:

```
test rtr_restores_the_saved_condition_codes ... FAILED
panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:112:5:
  left: [(true, "[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)` but the condition codes are not provably the caller's at return: the `move.w sr, -(sp)` save is never restored (the slice accepts only flag effects bracketed by the `move.w sr, -(sp)` … `move.w (sp)+, sr` pair; path-sensitive flag liveness is S2-D7)")]
 right: []
```

After the fix the first control (`addq.l #2, sp; rts`) printed `` `rts` returns between the save and the
restore, skipping it `` rather than the "never restored" text my first draft pinned; the test pins the
real refusal text (same id, same strength).

**RF step 3, on the clean committed HEAD (rtr commit)**, each restored with `git restore --source=HEAD`:

```
== M1: rtr back in is_branch_or_return
-    matches!(mnem, "rts" | "rte" | "rtd" | "jmp" | "jra" | "jbra")
+    matches!(mnem, "rts" | "rte" | "rtr" | "rtd" | "jmp" | "jra" | "jbra")
test rtr_replaces_the_condition_codes_a_conditional_out_reads ... FAILED   (left: the survives firing above, right: [])
test rtr_does_not_carry_the_computed_flags_to_the_caller ... FAILED   (got Produced)
== M2: rtr restore recognition off
-        if mnemonic == "rtr" && saved {
+        if mnemonic == "rtr" && saved && false {
test rtr_restores_the_saved_condition_codes ... FAILED   (left: "... the `move.w sr, -(sp)` save is never restored ...")
== M-sbcd / M-nbcd / M-negx / M-subx: `if m == "<x>" { return false; }` at the top of writes_dest_register
test each_new_arithmetic_word_is_charged_with_its_destination ... FAILED   `sbcd d0, d1`: []
test each_new_arithmetic_word_is_charged_with_its_destination ... FAILED   `nbcd d1`: []
test each_new_arithmetic_word_is_charged_with_its_destination ... FAILED   `negx.w d1`: []
test each_new_arithmetic_word_is_charged_with_its_destination ... FAILED   `subx.w d0, d1`: []
== restored: a11_checks 8 passed, out_verify 38 passed
```

**`rte`, its own commit.** Red before: `rte_does_not_carry_the_computed_flags_to_the_caller ... got Produced`.
Fixed the same way. Gate: BN `SHAPES 7/7 IDENTICAL`; seven shape builds with the base binary and the rte
candidate, warning tallies side by side (second summary line of each build; the first, `15 warnings,
module.path-mismatch 15`, is identical too on every sonic4 shape):

| Shape | Base `55c6ec32` | Candidate (rte fix) |
|---|---|---|
| plain | 161: clobber-undeclared 63, unreachable 59, undeclared-fallthrough 21, path-mismatch 15, no-names 2, out-unwritten 1 | identical |
| debug | 148: 67, 58, path-mismatch 15, fallthrough 5, 2, 1 | identical |
| demo | 84: 62, fallthrough 13, unreachable 8, 1 | identical |
| demo_debug | 80: 66, unreachable 7, fallthrough 6, 1 | identical |
| config_a | 147: 67, 57, 15, 5, 2, 1 | identical |
| config_b | 163: 63, 61, 21, 15, 2, 1 | identical |
| lean | 160: 63, 59, 20, 15, 2, 1 | identical |

The whole build logs are identical once the scratch paths are normalised (`diff` empty). Mutation on the
committed rte HEAD (`"rte"` back in the list): `rte_does_not_carry_the_computed_flags_to_the_caller` FAILED
(got Produced); restored: 39 passed.

TC (after the last code edit): `sigil-frontend-emp` 148 suites, 2877 passed, 0 failed, 0 ignored;
`frontend_differential` 7 passed; harness walk + golden + coverage 3 suites, 11 passed. Clippy clean.
BN on both commits: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.

### Batch C fix round 1: the usp moves and the flag checks (I1), `movem usp(pc)` (I2), minors (code commit 1d48a0b8)

**I1.** `.emp` gives the checks the written word `move`, and `m68k_mnemonic` has no spelling that maps to
`MoveToUsp`/`MoveFromUsp`, so there is no refined mnemonic to hand over. The operand carries the distinction
instead. `out_verify::is_usp_move` is true for a `move` holding `CodeOperand::Usp`, which the lowering allows
only in the two moves. It becomes a `Move => Untouched` arm in `carry_role` and `zero_role`, and it is added to
`cc_inert_data_op`, which now takes the operands. That one allowlist feeds `cc_transparent` (the `out_verify`
flag tracker, the `flag_check` guard walk) and `ccr_effect` (the `preserves(sr.ccr)` slice). `branch_const`'s
separate copy of the allowlist gets the same clause.

**I2 (ruling branch B).** An operand name reaches a symbol by more than one route: the immediate evaluator's label
fallback (`fallback_label`, `#usp`) and the spelled PC-relative target (`map_pcrel_operand`,
`scope.resolve_ref`, `lea usp(pc)`). The indirect spellings stop at `unknown name`. Since there is no single choke
point, I fixed only the new `movem` path: a `PcRel`/`PcRelIdx` target named `usp` is refused with the Task 4 guard's text.
`EMP-IMM-REGISTER-WORD` is rewritten from a probe of 31 spellings, run in the frontend_diff frame with `usp = $1234`:

```
move.l #usp, d0                => Ok([20, 3C, 00, 00, 12, 34, 4E, 75])
move.l #usp+1, d0              => Ok([20, 3C, 00, 00, 12, 35, 4E, 75])
lea usp(pc), a0                => Ok([41, FA, 12, 32, 4E, 75])
jmp usp(pc)                    => Ok([4E, FA, 12, 32, 4E, 75])
move.l usp(pc), d0             => Ok([20, 3A, 12, 32, 4E, 75])
pea usp(pc)                    => Ok([48, 7A, 12, 32, 4E, 75])
lea usp(pc,d0.w), a0           => Err(link: (d8,PC,Xn) displacement to `usp` out of range (4658))
movem.l usp(pc), d0-d1         => Err(["[operand.usp-unsupported] ..."])
movem.l usp(pc,d2.w), d0-d1    => Err(["[operand.usp-unsupported] ..."])
lea (usp), a0 / -(usp) / (usp)+ / usp(a0) / 4(usp) / usp+0 / 0+usp / dc.l usp / dc.w usp
                               => Err(["unknown name `usp`", "instruction dropped, ..."])
data p: u32 = usp / *u8 = usp  => Err(["unknown name `usp`"])
move.l (usp).w, d0 / (usp).l   => Err(["[operand.usp-unsupported] ... never a symbol ...", "instruction dropped, ..."])
move.l usp, d0 / lea usp, a0 / jsr usp => Err(["[operand.usp-unsupported] ..."])
move a0, usp                   => Err(["instruction needs an explicit size suffix (.b/.w/.l)"])
move.l a0, usp / move.l usp, a0 => Ok([4E, 60, 4E, 75]) / Ok([4E, 68, 4E, 75])
```

**Minors.** The golden usp/ccr vectors now use distinct non-zero registers (`a3`/`a5` → `4E63`/`4E6D`;
`move.w d3, ccr` → `44C3`). Both `movem` vectors are three-sided: `(d16,pc)` is `4CBA 0018 FFFA` and
`(d8,pc,a2.w)` is `4CFB 0018 A0FA`, and the shared encoder agreed with both literals on the first run. A new pin covers
the PC-relative `movem` destination (`illegal destination EA: (d...` on `.emp`, refused on the AS route). That
pin and the register changes are test-only, on behaviour that already existed, so they have no red. The dead ccr
alternative in the `emp-lacks-sr-ccr-form` predicate is removed, and FD is still green (7 passed). The stale
`EMP-USP-CCR-SURFACE` line is corrected: it listed bare `move a0, usp` and said the ccr half was Task 5's.

**RF step 1, red before implementing** (the four new tests, against tip 88112c25 plus the tests):

```
test a_flag_result_survives_a_usp_move ... FAILED
test preserves_ccr_holds_across_a_usp_move ... FAILED
test a_conditional_out_reads_the_flags_through_a_usp_move ... FAILED
thread 'a_flag_result_survives_a_usp_move' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:163:13:
assertion `left == right` failed: carry across `move.l a0, usp`: [FlagFiring { proc: "P", callee: "Queue", flag: "carry", ..., kind: Unused, name: None }]
  left: true
 right: false
thread 'preserves_ccr_holds_across_a_usp_move' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:189:5:
  left: [(true, "[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)` but the condition codes are not provably the caller's at return: `move` affects the condition codes outside the bracket (...)")]
 right: []
thread 'a_conditional_out_reads_the_flags_through_a_usp_move' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:181:5:
flags across `move.l a0, usp`
test result: FAILED. 8 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
test a_movem_pc_relative_source_named_usp_is_refused ... FAILED
thread 'a_movem_pc_relative_source_named_usp_is_refused' panicked at crates/sigil-frontend-emp/tests/usp_reserved.rs:50:18:
assembled to [4C, FA, 00, 03, 12, 30, 4E, 75]; expected a refusal:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 6 filtered out
```

The movem red is the bug itself: `movem.l usp(pc), d0-d1` linked against the outside `usp`, giving `4CFA 0003 1230`.

**RF step 2, green.** `a11_checks` 11 passed, `usp_reserved` 7 passed, golden 8 passed.

**RF step 3, mutations on the clean committed HEAD** (`git status --short` empty before each one; each restored with
`git restore --source=HEAD -- <file>` and re-run green: a11_checks 11, usp_reserved 7).

M1, the carry arm:
```
 crates/sigil-frontend-emp/src/flag_check.rs | 3 ---
-        // MOVE USP (`move.l aN, usp` / `move.l usp, aN`, spelled `move`):
-        // "Condition Codes: Not affected."
-        Move if crate::out_verify::is_usp_move("move", ops) => Untouched,
test a_flag_result_survives_a_usp_move ... FAILED
assertion `left == right` failed: carry across `move.l a0, usp`: [FlagFiring { ..., flag: "carry", ..., kind: Unused, name: None }]
test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

M1b, the zero arm:
```
 crates/sigil-frontend-emp/src/flag_check.rs | 1 -
-        Move if crate::out_verify::is_usp_move("move", ops) => Untouched,
test a_flag_result_survives_a_usp_move ... FAILED
assertion `left == right` failed: zero across `move.l a0, usp`: [FlagFiring { ..., flag: "zero", ..., kind: Unused, name: None }]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out
```

M2, the shared allowlist clause:
```
 crates/sigil-frontend-emp/src/out_verify.rs | 2 +-
-    ) || is_usp_move(mnem, ops)
+    )
test preserves_ccr_holds_across_a_usp_move ... FAILED
test a_conditional_out_reads_the_flags_through_a_usp_move ... FAILED
  left: [(true, "[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)` but ... `move` affects the condition codes outside the bracket ...")]
 right: []
flags across `move.l a0, usp`
test result: FAILED. 9 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

M3, the movem refusal:
```
 crates/sigil-frontend-emp/src/lower/code.rs | 2 +-
-            if target == "usp" {
+            if false {
test a_movem_pc_relative_source_named_usp_is_refused ... FAILED
thread 'a_movem_pc_relative_source_named_usp_is_refused' panicked at crates/sigil-frontend-emp/tests/usp_reserved.rs:50:18:
test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

TC (after the last code edit): `sigil-frontend-emp` 150 suites, 2881 passed, 0 failed, 0 ignored;
`sigil-cli --test frontend_differential` 7 passed; `sigil-harness --test emp_isa_walk_m68k --test emp_a11_golden
--test frontend_differential_coverage` 3 suites, 12 passed. Clippy clean. BN: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.

### Task 6: `link` / `unlk` (code commit d771447b)

The brief's code blocks compiled. One did not run as written: the suffix test's AS-route assertion built its
expected text from the whole line (`` `unlk.l a6` is refused ``), and the AS route names the written mnemonic only:
`` `unlk.l` is refused, as asl refuses it: `unlk` has no size field, write `unlk` with no suffix ``. The test now
takes the first word of the line (same strength: the named refusal on both routes).

One addition the brief did not list (the batch C lesson, a correct answer in every flag table): `link` and `unlk`
join the shared flag-inert allowlist `out_verify::cc_inert_data_op` (LINK / UNLK: "Condition Codes: Not
affected."). Without it `preserves(sr.ccr)` refused a closed frame and a conditional out lost its flags across
`unlk`; `branch_const`'s copy already listed both. Pinned by `a_frame_op_leaves_the_condition_codes_alone`.

What each table says: written registers, `link aN` writes `aN` (first operand, new arm) and `unlk aN` writes `aN`
(last operand); a7 is the stack model's (`apply_link` pushes 4 and moves sp by the displacement, `apply_unlk` closes
it), and neither instruction is charged a7 by the clobber lint, except `link a7, #d`, which is charged `a7` (the
model bails on it). Carry and zero: Untouched (ISA enum arms, now reached). Terminator: neither. Cycles: Task 10.

Probed (not committed): `link a6, #$FFF8` assembles `4E56 FFF8` on both routes, and the stack model reads it as
+65528 and bails, so `link a6, #$FFF8` then `rts` is silent where `link a6, #-8` then `rts` is `[stack.unbalanced]`;
`link a6, #8` assembles `4E56 0008` and bails; `link a6, #-$8001` is refused by the encoder (`operand -32769 out of
range`), and on `.emp` also prints a `[stack.unbalanced]` for the same line. Booked in `EMP-LINK-UNSIGNED-DISPLACEMENT`.

Golden: the brief's `a6, #-8` vectors plus `link a3, #-$1234` / `unlk a3`, derived: LINK.W is `0100 1110 0101 0rrr`
then the displacement word, so a3 is `$4E53` and -$1234 is `$EDCC`; UNLK is `0100 1110 0101 1rrr`, a3 is `$4E5B`.
The shared encoder and both front ends agreed with the literal on the first run.

**RF step 1, red before implementing.** `("link", 1), ("unlk", 1)` deleted from `KNOWN_REFUSED`, the golden test,
the suffix lines and the two check tests appended:

```
test link_and_unlk_assemble ... FAILED
thread 'link_and_unlk_assemble' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:128:5:
  left: Err(["`link` is not a recognized 68000 mnemonic", "`unlk` is not a recognized 68000 mnemonic"])
 right: Ok([78, 113, 78, 86, 255, 248, 78, 94, 78, 117])
test a_size_suffix_the_instruction_lacks_is_refused ... FAILED
thread 'a_size_suffix_the_instruction_lacks_is_refused' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:71:9:
`unlk.l a6`: ["`unlk` is not a recognized 68000 mnemonic"]
test result: FAILED. 7 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
test emp_assembles_every_68000_instruction_the_shared_encoder_accepts ... FAILED
    "link": [
        "`link.w a2, #1`: `link` is not a recognized 68000 mnemonic  ||  `link a2, #1`: `link` is not a recognized 68000 mnemonic",
    ],
    "unlk": [
        "`unlk.w a2`: `unlk` is not a recognized 68000 mnemonic  ||  `unlk a2`: `unlk` is not a recognized 68000 mnemonic",
    ],
  left: {"andi-sr": 1, "chk": 11, "eori-sr": 1, "link": 1, "ori-sr": 1, "reset": 1, "stop": 1, "trapv": 1, "unlk": 1}
 right: {"andi-sr": 1, "chk": 11, "eori-sr": 1, "ori-sr": 1, "reset": 1, "stop": 1, "trapv": 1}
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
test a_frame_is_balanced_only_when_it_is_closed ... FAILED
test a_frame_op_leaves_the_condition_codes_alone ... FAILED
thread 'a_frame_is_balanced_only_when_it_is_closed' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:198:5:
  left: [(true, "`link` is not a recognized 68000 mnemonic"), (true, "`unlk` is not a recognized 68000 mnemonic")]
 right: []
thread 'a_frame_op_leaves_the_condition_codes_alone' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:220:5:
flags across `unlk a6`
test result: FAILED. 11 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

**RF step 2, green.** golden 9 passed, walk 3 passed, `a11_checks` 13 passed, `as_author_forms_stay_off_emp` 4
passed, differential 7 passed (`link` compares `SameBytes` in the allowed frame).

**RF step 3, mutations on the clean committed HEAD** (`git status --short` empty before each; restored with
`git restore --source=HEAD -- <file>`).

M1 (the brief's), `link` out of the first-operand arm:
```
 crates/sigil-frontend-emp/src/lower/proc.rs | 2 +-
-    if mnemonic == "exg" || mnemonic == "link" {
+    if mnemonic == "exg" {
test a_frame_is_balanced_only_when_it_is_closed ... FAILED
assertion `left == right` failed: one for `link`, one for `unlk`: [(false, "[proc.clobber-undeclared] `P` writes `a6`, ...")]
  left: 1
 right: 2
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out
```
Restored: 1 passed.

M2 (extra), `link` / `unlk` out of the flag-inert allowlist:
```
 crates/sigil-frontend-emp/src/out_verify.rs | 2 +-
-        "movea" | "lea" | "pea" | "movem" | "exg" | "nop" | "adda" | "suba" | "link" | "unlk"
+        "movea" | "lea" | "pea" | "movem" | "exg" | "nop" | "adda" | "suba"
test a_frame_op_leaves_the_condition_codes_alone ... FAILED
thread 'a_frame_op_leaves_the_condition_codes_alone' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:220:5:
flags across `unlk a6`
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out
```
Restored: `a11_checks` 13 passed.

TC (after the last code edit): `sigil-frontend-emp` 148 suites, 2883 passed, 0 failed, 0 ignored;
`sigil-cli --test frontend_differential` 7 passed; `sigil-harness --test emp_isa_walk_m68k --test emp_a11_golden
--test frontend_differential_coverage` 3 suites, 13 passed. Clippy clean. BN: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.

### Task 7: `stop` and the SR immediates (code commit c1cabe26)

The brief's code blocks compiled and ran as given. Additions: a third check test,
`the_sr_writes_reach_the_flags_as_their_immediate_says` (the flag consequence of each form, firing), and the
golden vectors' literals derived: STOP is `$4E72` then the immediate word (`#$2700` → `4E72 2700`); ORI / ANDI /
EORI to SR are `$007C` / `$027C` / `$0A7C` then the word (`0700`, `F8FF`, `0700`). The shared encoder and both front
ends agreed with the literals on the first run.

What each table says. Mask (`[proc.sr-undeclared]`, `preserves(sr)` round trip, the gated build): all four are mask
writes, the immediates through their `sr` operand, `stop` by name. Carry / zero (`flag_check.rs`, reached through the
ISA enum once `m68k_mnemonic` knows `stop`): `stop` Writes (STOP: "Set according to the immediate operand"); `andi` /
`ori` / `eori #, sr` move C as the immediate's bit 0 says and Z as its bit 2 says (the existing `to_ccr_or_sr` arms),
so `ori.w #$0700, sr` leaves both and `ori.w #$0705, sr` writes both. Flag-inert allowlist: none of the four (all
rewrite the CCR), so `preserves(sr.ccr)` refuses each unbracketed and `out(.. if cc)` loses its flags across them.
Stack model: no effect. Terminator: none (execution continues after the interrupt). Cycles: Task 10.

Walk frame (lesson 4): no change. `[proc.sr-undeclared]` is a warning outside the gate, so `stop #1` in the bare
frame is accepted and the walk reads it as written.

`grep -n "CodeOperand::Sr" crates/sigil-frontend-emp/src -r` after the change, one word each:

```
corpus_contracts.rs:1321  write site, has stop
branch_const.rs:90        read (the constant-flag writer skips an sr/ccr destination; `stop` is no constant writer, falls to Dyn)
flag_check.rs:239, :518   read (flag tables, the to-ccr/sr operand refinement)
flag_check.rs:247, :525   read (MOVE from SR)
flag_check.rs:1797-1807   operand mapping (unit-test spellings)
lower/code.rs:1611        operand mapping
lower/mod.rs:2065         write site, has stop (the name list beside it)
lower/proc.rs:1381        write site, has stop
lower/proc.rs:2825, :2834 read (the CCR bracket's exact save / restore shapes; `stop` inside a bracket is a Writes, allowed)
lower/proc.rs:2966        read (the round trip's save)
lower/proc.rs:2972        write site, has stop (the block before it)
lower/proc.rs:2975        read (the restore shape)
eval/asm.rs:3495          operand mapping
```

**RF step 1, red before implementing.** `("stop", 1), ("andi-sr", 1), ("ori-sr", 1), ("eori-sr", 1)` deleted from
`KNOWN_REFUSED`, the golden test, the suffix line and the three check tests appended:

```
test stop_and_the_sr_immediates_assemble ... FAILED
test a_size_suffix_the_instruction_lacks_is_refused ... FAILED
thread 'stop_and_the_sr_immediates_assemble' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:147:5:
  left: Err(["`stop` is not a recognized 68000 mnemonic"])
 right: Ok([78, 113, 78, 114, 39, 0, 78, 117])
thread 'a_size_suffix_the_instruction_lacks_is_refused' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:71:9:
`stop.w #$2700`: ["`stop` is not a recognized 68000 mnemonic"]
test result: FAILED. 8 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
test emp_assembles_every_68000_instruction_the_shared_encoder_accepts ... FAILED
    "andi-sr": [
        "`andi.w #1, sr`: unsupported form: sr is not a general EA  ||  `andi #1, sr`: instruction needs an explicit size suffix (.b/.w/.l)",
    "eori-sr": [
        "`eori.w #1, sr`: unsupported form: sr is not a general EA  ||  `eori #1, sr`: instruction needs an explicit size suffix (.b/.w/.l)",
    "ori-sr": [
        "`ori.w #1, sr`: unsupported form: sr is not a general EA  ||  `ori #1, sr`: instruction needs an explicit size suffix (.b/.w/.l)",
    "stop": [
        "`stop.w #1`: `stop` is not a recognized 68000 mnemonic  ||  `stop #1`: `stop` is not a recognized 68000 mnemonic",
  left: {"andi-sr": 1, "chk": 11, "eori-sr": 1, "ori-sr": 1, "reset": 1, "stop": 1, "trapv": 1}
 right: {"chk": 11, "reset": 1, "trapv": 1}
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
test the_sr_writes_reach_the_flags_as_their_immediate_says ... FAILED
test every_sr_write_form_is_an_undeclared_mask_write ... FAILED
test stop_cannot_hide_behind_preserves_sr ... FAILED
thread 'the_sr_writes_reach_the_flags_as_their_immediate_says' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:269:13:
assertion `left == right` failed: carry across `stop #$2700`: []
  left: false
 right: true
thread 'every_sr_write_form_is_an_undeclared_mask_write' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:233:9:
`stop #$2000`: [(true, "`stop` is not a recognized 68000 mnemonic")]
thread 'stop_cannot_hide_behind_preserves_sr' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:247:5:
[(true, "`stop` is not a recognized 68000 mnemonic"), (false, "[proc.ccr-advisory] `P` declares bare `preserves(sr)` (both halves), but its condition codes are not provably the caller's at return: `stop` affects the condition codes outside the bracket, ...")]
test result: FAILED. 13 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

The gate test was written with the implementation; its red is Mutation A below.

**RF step 2, green.** golden 10 passed, walk 3 passed, `a11_checks` 16 passed, `as_author_forms_stay_off_emp` 4
passed, `flag_check` unit tests 18 passed, differential 7 passed, `sr_undeclared_gate` 6 passed.

**RF step 3, mutations on the clean committed HEAD** (`git status --short` empty before each; restored with
`git restore --source=HEAD -- <file>`).

Mutation A (the brief's), `stop` out of `check_clobbers`:
```
 crates/sigil-frontend-emp/src/lower/proc.rs | 3 +--
-        if ((writes_dest_register(mnemonic) && matches!(ops.last(), Some(CodeOperand::Sr)))
-            || mnemonic == "stop")
+        if ((writes_dest_register(mnemonic) && matches!(ops.last(), Some(CodeOperand::Sr))))
test every_sr_write_form_is_an_undeclared_mask_write ... FAILED
thread 'every_sr_write_form_is_an_undeclared_mask_write' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:233:9:
`stop #$2000`: []
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 15 filtered out
test a_stop_stops_the_gated_build ... FAILED
thread 'a_stop_stops_the_gated_build' panicked at crates/sigil-cli/tests/sr_undeclared_gate.rs:64:5:
assertion `left == right` failed: contract closure gate: passed (demo release: 80 module(s) analysed; ...)
  left: Some(0)
 right: Some(1)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out
```
Red on `stop #$2000` and at the build level (the gated build wrote a ROM). Restored: 1 passed, 1 passed.

Mutation B (the brief's), the `stop` block out of `sr_writes_round_trip`:
```
 crates/sigil-frontend-emp/src/lower/proc.rs | 7 -------
-        if mnemonic == "stop" {
-            saved_before_first_write.get_or_insert(saved);
-            last_write_is_restore = false;
-            continue;
-        }
test stop_cannot_hide_behind_preserves_sr ... FAILED
thread 'stop_cannot_hide_behind_preserves_sr' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:247:5:
[(false, "[proc.ccr-advisory] `P` declares bare `preserves(sr)` (both halves), but its condition codes are not provably the caller's at return: `stop` affects the condition codes outside the bracket, ...")]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 15 filtered out
```
Red on the first assertion: only the CCR advisory is left, the mask half passes. Restored: 1 passed.

Mutation D (extra), `stop` to Untouched in the carry table:
```
 crates/sigil-frontend-emp/src/flag_check.rs | 3 ++-
-        Rtr | Stop | Chk => Writes,
+        Rtr | Chk => Writes,
+        Stop => Untouched,
test the_sr_writes_reach_the_flags_as_their_immediate_says ... FAILED
assertion `left == right` failed: carry across `stop #$2700`: []
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 15 filtered out
```
Restored: `a11_checks` 16 passed.

TC (after the last code edit): `sigil-frontend-emp` 148 suites, 2886 passed, 0 failed, 0 ignored;
`sigil-cli --test frontend_differential` 7 passed; `sigil-cli --test sr_undeclared_gate` 6 passed;
`sigil-harness --test emp_isa_walk_m68k --test emp_a11_golden --test frontend_differential_coverage` 3 suites, 14
passed. Clippy clean. BN: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.

### Task 8: `reset` (code commit 7ff27bcf)

The brief's code blocks compiled and ran as given. Golden literal derived: RESET is the fixed word
`0100 1110 0111 0000` = `$4E70` with no operand field; the shared encoder and both front ends agreed with it on the
first green run. No register can vary, so lesson 3's distinct-register rule has nothing to apply to.

Beyond the brief (the dispatch's lesson 1: every table that classifies instructions): `reset` added to the flag-inert
allowlist `cc_inert_data_op` (`out_verify.rs`: the `out(.. if cc)` tracker, the flag_check guard walk, the
`preserves(sr.ccr)` slice) and to `branch_const`'s `cc_transparent`. A new check test,
`reset_leaves_registers_flags_and_flow_alone`, has a firing case and a control for each table.

What each table says for `reset`, and why (RESET pulses the external RESET line for 124 clocks; "Condition Codes: Not
affected"; no register, flag or stack word changes and execution continues):
- written registers: none (ISA `writes_last_operand(Reset)` = false; no operand for the auto-inc/dbcc/exg rules).
- carry_role / zero_role: Untouched (the ISA enum arm, reached once `m68k_mnemonic` knows the word).
- flag-inert allowlists (`cc_inert_data_op`, `branch_const`): inert. Without this, a conditional out lost its flags
  across `reset`, `preserves(sr.ccr)` refused a body with one, and the constant-branch net went blind.
- stack model (`preserves.rs::transfer`): no effect (no operand, no sp).
- terminator list: not a terminator; a body ending in `reset` warns `[proc.undeclared-fallthrough]` (pinned).

**RF step 1, red** (before the implementation: `("reset", 1)` deleted from `KNOWN_REFUSED`, golden, suffix row and
check test added):
```
test reset_leaves_registers_flags_and_flow_alone ... FAILED
thread 'reset_leaves_registers_flags_and_flow_alone' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:299:5:
assertion `left == right` failed
  left: [(true, "`reset` is not a recognized 68000 mnemonic")]
 right: []
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 16 filtered out
test reset_assembles ... FAILED
test a_size_suffix_the_instruction_lacks_is_refused ... FAILED
thread 'reset_assembles' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:156:5:
assertion `left == right` failed: `.emp`, `reset`
  left: Err(["`reset` is not a recognized 68000 mnemonic"])
 right: Ok([78, 113, 78, 112, 78, 117])
thread 'a_size_suffix_the_instruction_lacks_is_refused' panicked at crates/sigil-harness/tests/emp_a11_golden.rs:71:9:
`reset.w`: ["`reset` is not a recognized 68000 mnemonic"]
test result: FAILED. 9 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
test emp_assembles_every_68000_instruction_the_shared_encoder_accepts ... FAILED
    "reset": [
        "`reset.w `: `reset` is not a recognized 68000 mnemonic  ||  `reset `: `reset` is not a recognized 68000 mnemonic",
    ],
  left: {"chk": 11, "reset": 1, "trapv": 1}
 right: {"chk": 11, "trapv": 1}
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

**RF step 2, green.** `a11_checks` 17 passed, `as_author_forms_stay_off_emp` 4 passed, golden 11 passed, walk 3 passed.

**RF step 3, mutations on the clean committed HEAD** (`git status --short` empty before each; restored with
`git restore --source=HEAD -- <file>`).

Mutation A (the brief's), `"reset" => Reset,` commented out:
```
 crates/sigil-frontend-emp/src/lower/code.rs | 2 +-
-        "reset" => Reset,
+        // "reset" => Reset,
test reset_assembles ... FAILED
test a_size_suffix_the_instruction_lacks_is_refused ... FAILED
  left: Err(["`reset` is not a recognized 68000 mnemonic"])
 right: Ok([78, 113, 78, 112, 78, 117])
test result: FAILED. 9 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
test emp_assembles_every_68000_instruction_the_shared_encoder_accepts ... FAILED
  left: {"chk": 11, "reset": 1, "trapv": 1}
 right: {"chk": 11, "trapv": 1}
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
test reset_leaves_registers_flags_and_flow_alone ... FAILED
  left: [(true, "`reset` is not a recognized 68000 mnemonic")]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 16 filtered out
```
Restored: golden 11 passed, walk 3 passed, check 1 passed.

Mutation B (extra), `reset` out of `cc_inert_data_op`:
```
 crates/sigil-frontend-emp/src/out_verify.rs | 2 +-
-            | "reset" | "assume_some"
+            | "assume_some"
test reset_leaves_registers_flags_and_flow_alone ... FAILED
thread 'reset_leaves_registers_flags_and_flow_alone' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:318:5:
flags across `reset`
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 16 filtered out
```
Restored: `a11_checks` 17 passed.

Mutation C (extra), `reset` out of `branch_const`'s `cc_transparent`:
```
 crates/sigil-frontend-emp/src/branch_const.rs | 2 +-
-            | "unlk" | "reset"
+            | "unlk"
test reset_leaves_registers_flags_and_flow_alone ... FAILED
thread 'reset_leaves_registers_flags_and_flow_alone' panicked at crates/sigil-frontend-emp/tests/a11_checks.rs:328:5:
[]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 16 filtered out
```
Restored: `a11_checks` 17 passed.

TC (after the last code edit): `sigil-frontend-emp` 148 suites, 2887 passed, 0 failed, 0 ignored;
`sigil-cli --test frontend_differential` 7 passed; `sigil-harness --test emp_isa_walk_m68k --test emp_a11_golden --test
frontend_differential_coverage` 3 suites, 15 passed. Clippy clean. BN: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.
No FD edit (no operand; `reset` is a `NO_OPERAND_FAMILIES` member the generator skips).
