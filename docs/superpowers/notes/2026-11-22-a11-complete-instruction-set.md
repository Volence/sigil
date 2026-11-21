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
