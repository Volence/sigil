# asl probes: which namespace a macro-body label belongs to

The listings these produce are the evidence behind
`../2026-09-05-as-macro-body-label.md` and behind
`crates/sigil-frontend-as/tests/as_macro_body_label.rs`. Nothing here runs in the
suite.

## Running them

```
./run.sh   p1.asm                              # one probe through asl
./all.sh                                       # every probe, full listings
./digest.sh                                    # the same, minus asl's builtins
./stability.sh                                 # three runs each, hashed
./sigil.sh <path-to-sigil-binary> p1.asm …     # the same sources through sigil
```

`run.sh` is the symbol-class set's runner verbatim: Sonic 1's own flags
(`build_tools/lua/common.lua:773`) minus `-E` and `-c`, so the diagnostics land on
stdout. The reference binary is `s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098` — the upstream build, **not** the flamewing
fork Sonic 2 ships.

`run.sh` leaves a `.lst` (and a `.p`, when the file assembles clean) beside each
probe. Those are build output, not evidence to keep — delete them; only the
sources are committed.

## What each probe asks

| probe | question |
|---|---|
| `p1` | a body's own label, read BACKWARD from inside the same expansion |
| `p2` | the same read FORWARD — the `end-start` shape s2's macrosetup writes |
| `p3` | NESTED: an inner macro's label read by the OUTER body that called it |
| `p4` | NESTED the other way: the outer's label read from inside the inner |
| `p5` | a SIBLING expansion's label — inside an expansion, but not that one |
| `p6` | the three PC spellings (colon, colon-less column-0, on a data line) |
| `p7` | the other three drivers: `rept`, `irp`, `while` — per loop or per iteration? |
| `p8` | the control: a FILE-LEVEL label read from inside an expansion |
| `p9` | a `.local` written in a macro body, and which scope owns it |
| `p10` | `{INTLABEL}`: which `__LABEL__` spellings substitute, and which stay literal |
| `p11` | the `label` DIRECTIVE read from INSIDE the body that declares it |

Every one of them invokes its macro MORE THAN ONCE, at addresses that DIFFER. A
macro invoked once cannot separate "each instance owns the name" from "the last
definition wins", and a label whose address is the same under both readings emits
the same byte either way.

`p11` is the one exception, and it is forced rather than sloppy: `m18` measured a
second `Al label $100` as `#1000 symbol double defined` even with the value
unchanged, so there is no two-instance version of that shape that assembles. Its
discriminator is the VALUE instead — `$100` is nowhere near the PC.

## The measuring scripts

| script | what it does |
|---|---|
| `census.sh` | the population, by compiler: four aeon shapes + both corpora under `SIGIL_CENSUS_EXPLABEL=1` |
| `poscontrol.sh` | the positive control — inject a macro-body label into a COPY of the aeon tree and show the census naming it |
| `fourshapes.sh` | the landing condition: four aeon shapes, CRC32+size per shape |
| `corpora.sh` | both corpora's whole diagnostic stream, for the old-vs-new diff |
| `mutations.sh` | red-first for `as_macro_body_label.rs`, five mutations |

`poscontrol.sh` and `census.sh` write a second aeon tree; **keep it outside the
sigil worktree.** `scripts_name_their_tree` and the drift harness both scan the
whole tree for exactly one `tools/suite_paths.py`, and a second aeon anywhere
under it makes both refuse with `COULD NOT MEASURE` — a failure that looks
nothing like its cause.

`stability.sh` blanks THREE clock readings before hashing: the page banner, the
`DATE`/`TIME` builtins, and `N.NN seconds assembly time` in the trailer. The last
is the one that matters — it reads `0.00` on most runs and `0.01` on some, so a
batch of three can be identical by luck and a batch straddling a tick reports an
oracle divergence that is a stopwatch.
