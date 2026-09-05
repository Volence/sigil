# asl probes: what a repeated `include` does, and what bounds it

The listings these produce are the evidence behind
`crates/sigil-frontend-as/tests/as_include_repeat.rs` and behind the rewritten
`directive_include` doc comment in `crates/sigil-frontend-as/src/eval.rs`.
Nothing here runs in the suite.

## Running them

```
./run.sh   p1.asm                                  # one probe through asl
./depth.sh 199                                     # asl: a chain of N distinct files
./siblings.sh 250 [sigil]                          # N includes IN SEQUENCE, both tools
./sigil.sh <sigil-binary>                          # the same sources through sigil
./depth_sigil.sh <sigil-binary> 199                # sigil: the same chain
./fourshapes.sh <sigil> <aeon> <out>               # landing condition: 4 shapes, CRC32+size
./corpora.sh <sigil> <out>                         # both corpora, CRC/size/diagnostic count
./census.sh <sigil> <aeon> <out>                   # the ENGAGEMENT COUNTER over the population
./census_selfcheck.sh <sigil>                      # calibrate that counter against known values
./mutations.sh <worktree> <target-dir>             # red-first, six mutations
./reach.sh <worktree> <target-dir> <aeon> <out>    # the two panic! reachability controls
./poscontrol.sh <base> <parcel> <aeon> <out>       # the divergence, on real aeon source
```

`census.sh`, `reach.sh` and `poscontrol.sh` write a second aeon tree or build
ROMs; **keep their out-dirs outside the sigil worktree and outside any cargo
target dir.** `scripts_name_their_tree` and the drift harness both scan for
exactly one `tools/suite_paths.py`, and a second aeon under either makes both
answer `COULD NOT MEASURE` — a failure that looks nothing like its cause.

`mutations.sh` and `reach.sh` build a MUTATED assembler into the target dir and
rebuild it from restored source before exiting. If either is interrupted, rebuild
before trusting that binary.

`run.sh` is the macro-body-label set's runner plus one thing: **every asl
invocation is under `timeout`.** This set deliberately asks asl what it does
with a file that includes itself, and an assembler that answered by recursing
forever would hang the run. `ASL_EXIT=124` here is the measurement "asl did not
stop", not a flake to retry.

The reference binary is `s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098` — the upstream build, **not** the flamewing
fork Sonic 2 ships.

`run.sh` leaves a `.lst` (and a `.p`, when the file assembles clean) beside each
probe. Those are build output, not evidence to keep — delete them; only the
sources are committed. `depth.sh`, `depth_sigil.sh` and `sigil.sh` write into a
scratch dir under `/home/volence/sonic_hacks/.parcel-include-scratch/`, never
`/tmp` (tmpfs on this machine) and never inside a cargo target dir.

## What each probe asks

| probe | question | asl's answer |
|---|---|---|
| `p1` | one 2-byte header, included TWICE, pure data | both copies emit: `1122` at `$1000` and `$1002` |
| `p2` | a file that includes ITSELF | 199 copies, then `error #10008: INCLUDE nested too deeply`, fatal, `ASL_EXIT=3` |
| `p3` | MUTUAL recursion — `b` includes `c`, `c` includes `b` | same refusal, `ASL_EXIT=3`, at `p3b.inc(2)` |
| `p4` | a DIAMOND — `a` includes `b` and `c`, both include `d` | `d` assembles TWICE |
| `p5` | three spellings of one path: `x`, `./x`, `sub/../x` | three inclusions; asl compares nothing |
| `p6` | a repeated include that DEFINES a PC label | `error #1000: symbol double defined`, `ASL_EXIT=2` — and it emits the bytes anyway |
| `p7` | a header holding `n set n+1 / dc.b n`, included 3× | `01 02 03` — the copies are not required to agree |
| `p8` | does the `NESTMAX` builtin (100) govern the include bound? | **no** — `nestmax 5` leaves the bound at 199 |

Every probe that asks "once or twice" uses a header that EMITS BYTES and asserts
the bytes *and* the length. A zero-byte header, or a second copy landing where
both readings agree, cannot separate the two — the image is identical either way.

`p5` is the weakest of the set and is kept with its weakness stated: since asl
does not dedupe at all, path spelling turns out to be a question asl never asks,
so `p5` confirms `p1` rather than adding to it. It earns its place on the SIGIL
side, where the old rule keyed on the canonicalized path and all three spellings
collapsed to one inclusion.

## The bound

`depth.sh N` generates `N` DISTINCT files, each including the next — a chain no
re-inclusion rule of any shape may touch, so the only thing that can refuse it is
depth. Measured:

```
./depth.sh 199   ->  ASL_EXIT=0, deepest level reached: 199
./depth.sh 200   ->  n199.inc(2):10: error #10008: INCLUDE nested too deeply
                     fatal error, assembly terminated, ASL_EXIT=3
```

The self-including `p2` stops at the same 199. So **asl's bound is about DEPTH
and not about repetition**, and there is no cycle detection to reproduce: a
genuine cycle is refused because it is unbounded, at the same wall a legitimate
200-deep chain of distinct files hits.

`depth_sigil.sh` runs the identical chain through sigil, and it refuses at the
same site asl names (`n199.inc(2)`).

## The before/after matrix

Sigil at the branch point (`master 4378716`) against sigil with the parcel, on
the same probe sources. Bytes are from `$1000`; every probe sits at `org $1000`.

| probe | asl | sigil BEFORE | sigil AFTER |
|---|---|---|---|
| `p1` | `1122 1122 99 00 00000006` | `1122 99 00 00000004`, exit 0 | matches asl |
| `p2` | refused, exit 3 | **`aa`, exit 0, silent** | `INCLUDE nested too deeply` at `p2self.inc(2)` |
| `p3` | refused, exit 3 | **`b0c0`, exit 0, silent** | `INCLUDE nested too deeply` at `p3b.inc(2)` |
| `p4` | `b0 dd c0 dd 99 …` | `b0 dd c0 99 …` | matches asl |
| `p5` | three copies | one copy | three copies |
| `p6` | `#1000 symbol double defined`, exit 2 | **`112299`, exit 0, silent** | `symbol double defined: p6_sym` |
| `p7` | `01 02 03 99` | `01 99` | `01 02 03 99` |

The two rows to read twice are `p2` and `p3`. The old guard did not merely drop
bytes: it accepted, silently and with a ROM written, two programs asl terminates
on. The guard's stated purpose was to stop runaway inclusion, and it was the
thing preventing the runaway case from being *detected*.

`p6` is the one row where a difference remains. asl raises the double-define AND
emits the bytes, exiting 2; sigil raises the same diagnostic and emits no image,
because refusing to write output on an error is a property of the whole front end
rather than of this directive. Sigil's answer here is nonetheless strictly better
than before, where it was silent.
