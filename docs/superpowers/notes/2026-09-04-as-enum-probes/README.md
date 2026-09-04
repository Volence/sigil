# AS `enum` / `nextenum` / `enumconf` — the asl-minted semantics

Every value below is read out of an `asl -L` listing, not out of a reading of the
documentation. The oracle is S1's own binary,
`s1disasm/build_tools/Linux-x86_64/asl` — `Macro Assembler 1.42 Beta [Bld 212]
(x86_64-unknown-linux)` — invoked with S1's own flags, `-xx -n -q -A -L -U -i .`.

`run.sh <file.asm>` reruns any probe. The listing annotation `=$X..$Y` on an
`enum`/`nextenum` line is AS reporting the first and last value the line bound;
it is the single most useful thing in the listing and every table row cites it.

## The model, in one sentence

There is one running counter and one running step. A member assigns the counter,
then the counter advances by the step **read at that moment**; an explicit
`name=expr` sets the counter to `expr` first and then advances it the same way.

That is the whole of it. Everything below is a consequence.

## The table

| # | question | probe | asl says | listing line |
|---|---|---|---|---|
| 1 | default start | `enum a,b,c` | `0,1,2` | `=$0..$2` / `0001 02` |
| 2 | default step | `enum a=1,b,c` | `1,2,3` | `=$1..$3` / `0102 03` |
| 3 | does `enum` restart? | `enum a=1,b,c` then `enum d=$10,e,f` | yes — `$10,$11,$12` | `=$10..$12` / `1011 12` |
| 4 | what does `nextenum` continue from? | after `f=$12`, `nextenum g,h` | the running counter — `$13,$14` | `=$13..$14` / `1314` |
| 5 | `enumconf` configures | `enumconf $C` then `enum a=$88,b,c` | the **step** — `$88,$94,$A0` | `=$88..$A0` / `8894 A0` |
| 6 | explicit `=expr` mid-list | `enum a=$80,b,c=b,d,e` | `$80,$81,$81,$82,$83` — the counter follows the explicit value | `=$80..$83` / `8081 8182 83` |
| 7 | step change mid-enumeration | `enumconf 4` / `enum a=0,b` / `enumconf 1` / `nextenum c,d` | `a=0 b=4 c=8 d=9` — **`c` is `$8`, not `$5`** | `=$0..$4` then `=$8..$9` / `0004 0809` |
| 8 | negative step | `enumconf -1` / `enum a=5,b,c` | `5,4,3` — counts down | `=$5..$3` / `0504 03` |
| 9 | zero step | `enumconf 0` / `enum a=5,b,c` | `5,5,5` | `=$5..$5` / `0505 05` |
| 10 | `nextenum` with no prior `enum` | `nextenum q,r` as the first enum line | no diagnostic; counter starts at 0 — `q=0,r=1` | `=$0..$1` / `0001` |
| 11 | does `enum` reset the step? | `enumconf 4` then `enum` | no — the step is global and persists across `enum` | q4 |
| 11a | does `enum` reset the COUNTER? | `enum a=5,b` then `enum c,d` | **yes** — `c=0`, not `$7`. This is the only thing separating `enum` from `nextenum` | `=$5..$6` then `=$0..$1` / `0506 0001` (q11) |
| 11b | both at once | `enumconf 3` / `enum a=5,b` / `enum c,d` / `nextenum e,f` | `5,8 · 0,3 · 6,9` — counter reset, step kept | `=$5..$8`, `=$0..$3`, `=$6..$9` (q12) |
| 12 | redefinition | `enum a=1,b` then `enum a=9,c` | `error #1000: symbol double defined`; **the first value is kept** (`a=1`) but **the counter still takes the new value** (`c=$A`) | `=$9..$A` / `0102 0A` |
| 13 | forward reference in the value | `enum a=fw,b` with `fw EQU 4` below | `error #1820: expression must be evaluatable in first pass`; value folds to 0 | q10 |
| 14 | member referenced above its own `enum` line | `dc.b z` then `enum z=7` | resolves — `07`. It is an ordinary two-pass symbol | q9 |
| 15 | expression as the start | `k EQU 3` / `enum a=k*2,b` | `6,7` — arbitrary expression | `=$6..$7` |
| 16 | member value's kind | symbol table | a plain integer constant, listed exactly as an `EQU` is (`a : 1`) | q1 symbol table |
| 17 | `enumconf` arity | `enumconf` with 0 args | `error #1110: wrong number of operands` / `expected between 1 and 2 arguments but got 0` | — |
| 18 | `enumconf`'s second argument | `enumconf 1,CODE` accepted; `enumconf 2,DATA` and `enumconf 1,2` rejected | a **segment name**, not a second number — `error #1961: unknown segment` | — |
| 19 | `enum` arity | `enum` with 0 args | `error #1110` / `expected between 1 and 476 arguments but got 0` | — |
| 20 | mnemonic case | `ENUM` / `NEXTENUM` | recognized — head folding is case-insensitive even under `-U` | — |

## Rows 7 and 12 are the two that a plausible implementation gets wrong

**Row 7** is the one that separates "advance after assign" from "compute from a
base". A reading where `nextenum` resumes at `last + current_step` gives `c=$5`.
AS gives `c=$8`, because the counter was already advanced past `b` by the step
that was in force *on the `enum` line*, and the later `enumconf 1` cannot reach
back into it. The step is read at the moment of each advance, and never re-applied.

**Row 12** splits the symbol from the counter. A redefinition is refused *as a
definition* — `a` keeps `1` — while the counter still accepts `9`, so the member
after it is `$A` and not `$3`. The two are not the same piece of state.

## What the corpus actually uses

`sound/_smps2asm_inc.asm` — one file, shared verbatim by S1 and S2 — exercises
rows 1-6, 8 and 11. The pitch table opens `enumconf $C` (row 5) and closes
`enumconf 1` (row 11); the note table is one `enum nRst=$80` followed by
sixteen `nextenum` continuations (row 4) whose enharmonics are all explicit
`nDb0=nCs0` mid-list assignments (row 6).

Rows 7, 12 and 13 are **not** reached by S1 or S2. They are implemented to the
listing anyway, because the cost of being wrong about them later is a silently
shifted note table.

Row 11a is not reached either, and for a reason worth writing down: **every
`enum` line in the corpus carries an explicit start value** (`enum
smpsPitch10lo=$88`, `enum nRst=$80`, `enum objoff_30=$30`), so the reset is
overwritten before it can be observed. A build of sigil with the reset deleted
outright still reproduces the corpus's whole pitch and note table byte for byte.
The corpus is therefore **not** a test of that row, and the two probes that are
(q11, q12) exist because of it.

## Where the construct's real weight sits

S1 spends the enum names almost entirely in `dc.b`/`dc.w` data, which sigil
defers to a link fixup rather than resolving in the front end — so an unbound
name there is silent until link, and S1 does not currently reach link.

S2 is the opposite and it is where the mass is. `s2.constants.asm` builds the
entire object-RAM offset vocabulary out of three `enum` lines:

```
 134: enum objoff_30=$30,objoff_31=$31,…,objoff_37=$37
```

aliases it into hundreds of semantic names (`boss_hurt_sonic = objoff_38`), and
references those in **thousands of instruction operands** — which the front end
*does* resolve. So the three unimplemented lines were not hiding a population;
they were generating one, loudly, in a different diagnostic class:

| | before | after |
|---|---|---|
| `X is not a recognized 68000 mnemonic` (enum/nextenum/enumconf) | 31 | 0 |
| `unresolved symbol X in operand` | 3,384 | 184 |
| S2 total | 9,266 | 6,035 |

190 distinct symbol names left the unresolved set and **0 entered it**.

Claude-Session: https://claude.ai/code/session_01QU6arHjqorA3eMNhsorP3H
