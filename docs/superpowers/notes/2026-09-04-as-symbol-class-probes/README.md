# asl probes: the constant-vs-variable symbol class

The listings these produce are the evidence behind
`../2026-09-04-as-symbol-class-tracking.md` and behind
`crates/sigil-frontend-as/tests/as_symbol_class.rs`. Nothing here runs in the
suite.

## Running them

```
./run.sh   m1.asm                              # asl 1.42 Beta Bld 212
./sigil.sh <path-to-sigil-binary> m1.asm …     # the same source through sigil
```

`run.sh` uses Sonic 1's own flags (`build_tools/lua/common.lua:773`) minus `-E`
(which redirects the error listing to a file) and minus `-c`, so the diagnostics
land on stdout. The reference binary is
`s1disasm/build_tools/Linux-x86_64/asl` — the upstream build, md5
`61e672562465725a8c102288a7da9098`, **not** the flamewing fork that Sonic 2
ships.

`m8.asm` is the one exception: it must be run with command-line defines, which
`run.sh` does not pass. Its own invocation is

```
AS_MSGPATH=$ASLDIR $ASLDIR/asl -xx -n -q -A -L -U -D Dv=1 -D Dw=1 -i . m8.asm
```

## What each probe asks

| probe | question |
|---|---|
| `m1` | `equ` followed by each of the five assignment spellings |
| `m2` | `set` followed by each of the five |
| `m3` | a colon label followed by each of the five, and by a second colon label |
| `m4` | is the refusal about the VALUE changing? and: bare label, the `label` directive, a label on a data line |
| `m5` | `enum` members, and whether a STRING or FLOAT value changes the rule |
| `m6` | a declaration inside a macro expansion, inside a `rept`, and a `.local` under two scopes |
| `m7` | does re-executing a declaration on a SECOND PASS count as a redefinition? |
| `m8` | what class does a command-line `-D` define carry? |
| `m9` | `set` first, then each of the four constant-MAKING forms |
| `m10` | which line sets the local-label scope — a label, or a `set`? |
| `m11` | the divergence in its sharpest form: both assemblers, one source, different bytes |
| `m12` | a constant declared inside a `rept`: does each iteration count? |
| `m13` | a macro invoked twice, each expansion declaring the same name |
| `m14` | the same header `include`d twice (needs `m14hdr.inc` beside it) |
| `m15` | a label redeclared across `phase`/`dephase` |
| `m16` | a second declaration the pass never EXECUTES (`if 0`), and its executed twin |
| `m17` | is an expansion's PC label global at all? (it is not) |
| `m18` | the remaining constant-making forms inside an expansion, and `irp`/`while` |
| `m19` | reading an expansion's `enum` member and `label` directive from OUTSIDE |
| `m20` | the mixed order: an expansion-local constant first, a file-level one second |

`m12`–`m20` are the `#1000` parcel's, and the note behind them is
`../2026-09-05-as-duplicate-definition.md`.

`run.sh` leaves a `.lst` (and `.p`, when the file assembles clean) beside each
probe. Those are build output, not evidence to keep — delete them; only the
sources are committed.
