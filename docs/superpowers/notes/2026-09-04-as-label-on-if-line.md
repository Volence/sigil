# A label on a block-head line — what AS actually does, and what sigil now does

**Parcel:** F6 from `2026-09-04-s1-path-to-rom.md`, tier 1 row 1.
**Branch:** `worktree-agent-a9d0aa550234932be`, on top of sigil master `709bffeb`.
**Oracle:** `asl` 1.42 Beta [Bld 212] (x86_64-Linux) at
`s2disasm/build_tools/Linux-x86_64/asl`, invoked `-cpu 68000 -q -U -L` and, where
bytes are quoted, `p2bin` on the `.p`.

`s1disasm/sonic.asm(4121)` writes

```
Map_Ring:	if Revision=0
```

and sigil bound nothing. `exec` routes a line on its keyword; the keyword here is
`if`; `exec_if` and every other block handler are handed the whole line and none
of them looks at the label column. So the name was neither bound nor diagnosed —
it did not exist, and every reference to it failed far away, at link, with no
file and no line. One line in the corpus, and it was the entire link-stage
unresolved-symbol population.

## What the oracle said

**The value is the PC of the label's own line, and the condition is not part of
the question.** Same source, `Rev` 0 then 1:

```text
       3/     100 : =$0                  Rev:	equ 0
       4/     100 : AA                  	dc.b $AA
       5/     101 : =>TRUE               Lab:	if Rev=0
       6/     101 : 01                  	dc.b 1
       7/     102 : =>FALSE              	else
      10/     102 : 0000 0101           	dc.l Lab            Lab : 101 C

       3/     100 : =$1                  Rev:	equ 1
       5/     101 : =>FALSE              Lab:	if Rev=0
       7/     101 : =>TRUE               	else
       8/     101 : 02                  	dc.b 2
      10/     102 : 0000 0101           	dc.l Lab            Lab : 101 C
```

`101 C` in both: a relocatable code label at the address the `if` line sits on.
An untaken arm does not move it and does not withhold it.

**The rule belongs to the label field, not to `if`.** One probe, one label per
shape, each in an active region at PC `101`:

| line shape | asl symbol table |
|---|---|
| `L:	if 1=1` / `if 1=0` | `L : 101 C` |
| `L:	ifdef Z` , `L:	ifndef Z` | `L : 101 C` |
| `L:	rept 2` | `L : 101 C` |
| `L:	irp X,1,2` , `L:	irpc X,ab` | `L : 101 C` |
| `L:	while W<2` | `L : 101 C` |
| `L:	switch 3` , `L:	case 3` | `L : 101 C` |
| `L:	align 2` , `L:	org $200` , `L:	padding off` | `L : 101 C` (sigil already agreed — these route through `exec_one`) |
| `L:	macro` | **absent** — `macro` CONSUMES the name |
| `L:	endm` (closing a `rept`) | **absent** |

**A closer binds exactly when the arm it closes ran.** This is the part a guess
would have got wrong in both directions:

```text
       5/       1 : =>TRUE               	if 1=1
       6/       1 : BB                  	dc.b $BB
       7/       2 : [5]                  L:	endif               L : 2

       5/       1 : =>FALSE              	if 1=0
       7/       1 : [5]                  L:	endif               (absent)
                                          #1: symbol undefined
```

and the mirror image on `else`: `if 1=1 … L: else` binds `L` at `2`, `if 1=0 …
L: else` binds nothing. Both are the same one rule — **the label field is read
with whatever emitting state the line is reached in** — and it means exactly one
line per `if` region can bind: the `elseif`/`else`/`endif` that terminates the
taken arm. `endm` is not one of them, because it terminates a *captured* body:
the collector eats that line and the expansion never replays it.

**A block head inside a skipped region binds nothing**, because the region is
not executed at all:

```text
       5/       1 : =>FALSE              	if 1=0
       6/       1 :                     N:	if 1=1              (absent)
      10/       1 : DD                  	dc.b $DD
```

**The colon-less spelling obeys AS's column rule.** `L	if 1=1` at column 0
binds `L : 101 C`. Indented:

```text
       5/       1 :                       L	if 1=1
> > > (5):3: error: unknown instruction
> > > (7): error: ELSEIF/ENDIF without IF
```

— not merely "not a label": asl does not process the `if` either.

**The label is an ordinary PC label in every respect**, including opening the
scope its `.local` names hang off:

```text
       5/       1 : =>TRUE               L:	if 1=1              L : 1 C
       7/       2 : CC                  .loc:	dc.b $CC            L.loc : 2 C
```

## What changed

`crates/sigil-frontend-as/src/eval.rs`, two sites and one helper
(`head_label` / `bind_head_label`):

1. `exec`'s seven block-opener arms (`if`/`ifdef`/`ifndef`, `rept`, `irp`,
   `irpc`, `while`, `switch`) bind the line's label before dispatching.
   `macro`/`struct`/`function` are deliberately absent.
2. `exec_if` binds the label of the line that terminates the taken arm, guarded
   on that line's keyword really being `elseif`/`else`/`endif` (`find_block_end`
   falls back to the last line of an unterminated region, which is a body line
   and not a closer).

Binding goes through `define_label`, the same entry point `exec_one` uses, which
is what makes the name relocatable and makes it open the local scope.

Nine tests in `crates/sigil-frontend-as/tests/label_on_directive_line.rs`, every
expectation a byte string or a symbol-table row read off asl's listing for the
identical source text.

## Deliberately NOT changed, and why

* **`case` / `elsecase` / `endcase` labels.** asl binds them by the same rule
  (`L: case 3` → `101`, `L: endcase` → `102`). Left alone because sigil's
  `switch` is string-only and refuses AS's integer form outright (`switch needs a
  string expression` on `switch 3`) — that is F8 in the ranked list, and the
  closers should land with the construct rather than ahead of it. The `switch`
  HEAD's own label does bind, because that arm is in `exec`.
* **`endm` labels.** asl binds none, and neither did sigil. Nothing to do; it is
  a test now so it stays that way.
* **The indented colon-less head.** asl reports `#1200 unknown instruction` and
  does not process the directive; sigil silently processes it as an `if`. The
  new test pins only the half this parcel owns — that the name is not a label.
  The other half (sigil should refuse the line) is a separate divergence, logged
  below.

## Two divergences found and NOT fixed here

1. **A label defined inside a macro expansion.** asl scopes it to the
   expansion — `M: macro / L: dc.b $BB / endm / M / dc.l L` is
   `#1: symbol undefined` — and sigil binds it globally. Found while checking
   whether the if-line case was special inside a macro; the CONTROL (a plain
   `dc.b` label) behaves identically, so this is the general macro-local-label
   rule and not an if-line matter. Out of scope, unmeasured against the corpus.
2. **The indented colon-less directive head** (above): asl refuses the line,
   sigil executes it. Byte-identical on the probe by luck, not by agreement.

## The gates, and the proof they are not vacuous

Red-first, on the committed baseline `709bffeb` with the new test file as the
only working-tree change (`git diff --stat HEAD` named that file and nothing
else): **5 failed, 4 passed.** The four that already passed are the shapes AS
binds NOTHING on, so red-before-fix cannot speak for them — they are what stops
the fix from over-binding, and each was proven by mutating the landed fix and
watching it go red. Each mutation was shown applied with `git diff -U1`, then
restored with `git checkout HEAD --` from the committed baseline:

| mutation (one line unless noted) | tests that went red |
|---|---|
| add `bind_head_label` to `exec`'s `macro` arm | `macro_still_consumes_the_name_in_its_label_field` |
| bind the `endif` unconditionally at the end of `exec_if` | `a_line_closing_a_skipped_arm_binds_nothing`, `the_line_closing_a_taken_arm_binds_its_label` |
| bind nested heads at SCAN time (3 hunks) + bind `exec_rept`'s `endm` | `a_block_head_in_a_skipped_region_binds_nothing`, `a_label_on_endm_is_not_bound` |

All nine green again after the restore, on a clean tree.

## Suite

`scripts/landing-run.sh --aeon /home/volence/sonic_hacks/.aeon-as-fold
--baseline 4375`, log
`landing-20260904T052106Z.log`, tree stamped
`…/worktrees/agent-a9d0aa550234932be @ 35a679df (worktree-agent-a9d0aa550234932be, clean)`:

```
  CARGO_EXIT      0
  suites          379
  passed          4375     failed 0     ignored 2     skip lines 0
  RESULT          GREEN
```

Coarse tripwire: `git grep -c '#[test]' HEAD -- '*.rs'` sums to 4376 against
4375+0+2 = 4377 observed. A difference of one is bookkeeping, not a different
tree.

**The first run of this suite went red on one test, and it was self-inflicted:**
`version_reports_the_head_of_the_tree_it_was_built_from` compares the binary's
stamped revision against `git rev-parse HEAD`, and the docs commit landed while
the suite was in flight. The test's own message names that case and says to
re-run to distinguish it from a build.rs trigger fault; the re-run above, with
HEAD held still, is green. Recorded because "one red test, ignore it, it's
timing" is exactly what a real stale-stamp defect would also look like.

## Measurements

**Sonic 1 corpus** (`s1disasm` `f6ece657`, the live checkout: two modified
`.nem` files, `.aurora/` and `Test.hsproject` untracked — the same tree state the
baseline note describes; read-only run, `sigil sonic.asm -o /dev/null`):

| | before (baseline note) | after |
|---|---|---|
| frontend diagnostics | 318 | **305** |
| `unresolved long expression` | 13 | **0** |
| rows at `_inc/Special Stage Mappings & VRAM Pointers.asm(10)` | 5 | **0** |
| rows at `_incObj/DebugMode.asm(382)` | 8 | **0** |

−13, matching the predicted count exactly, and both sites now produce no rows at
all.

**The 16 link failures are NOT measured here, and must not be read as closed.**
The frontend still exits 1 on the other 305 diagnostics, so the link never runs
in this tree; the baseline's 16 was measured in a *stubbed* corpus this parcel
does not have. What IS measured is the recorded standalone repro,
`.s1recon-out/probe/p7.asm`, which is the same construct with the same forward
references and which previously failed at link:

```text
       4/       0 : 0000 000C           	dc.l	LblOnIf
       5/       4 : 0000 000D           	dc.l	LblOnRept
       6/       8 : 0000 000F           	dc.l	LblOnInclude
   asl:   00 00 00 0c 00 00 00 0d 00 00 00 0f 01 02 02 03
   sigil: 00 00 00 0C 00 00 00 0D 00 00 00 0F 01 02 02 03
```

**Aeon byte identity — all four canonical shapes, built with this sigil against
the prepared reference tree** `/home/volence/sonic_hacks/.aeon-as-fold` at aeon
`4f5ad5a146b799c13aedabbba9da23fce370b63c`, each deleted and rebuilt behind a
pre-build mtime marker:

| shape | built | golden | |
|---|---|---|---|
| `s4.bin` | `14ee2440` / 719700 | `14ee2440` / 719700 | identical |
| `s4.debug.bin` | `142294b3` / 737683 | `142294b3` / 737683 | identical |
| `demo.bin` | `0c456778` / 96474 | `0c456778` / 96474 | identical |
| `demo.debug.bin` | `2e603d53` / 101339 | `2e603d53` / 101339 | identical |

`sigil --version` in that run reported revision `97f5368d…`, this parcel's own
commit — the binary that produced those bytes carried the change.

Reachability, run first and separately because it bounds the risk on its own:
the construct does not appear at all in the three `.asm` files aeon's `build.sh`
still routes through `sigil-frontend-as` (`engine/debug/debugger.asm`,
`games/demo/game_root.asm`, `games/sonic4/game_root.asm`) — zero hits for a
column-0 name followed by any of `if`/`ifdef`/`ifndef`/`else`/`elseif`/`endif`/
`rept`/`irp`/`irpc`/`while`/`switch`/`case`/`endcase`/`endm`/`endr`/`end`. Those
files DO carry 20-odd column-0 `NAME: macro` and `NAME: equ` lines, which is
exactly the population the `macro`/`struct`/`function` exclusion protects, and
the byte result is what proves the exclusion held.
