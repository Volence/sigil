# `include` builds a tree, not a DAG — and the guard was hiding the runaway case

Branch `parcel/as-include-once-vs-twice`, off master `4378716`.

`directive_include` kept a set of every path it had ever included and silently
returned from the second `include` of any of them, commented *"already included
(DAG guard)"*. asl has no re-inclusion guard of any kind. The difference was
silent on both sides: asl said nothing because it was doing the ordinary thing,
sigil said nothing because it thought it was.

Probes, runners and controls are under
`2026-09-05-as-include-repeat-probes/`. Reference asl is 1.42 Beta [Bld 212],
`s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098`, with Sonic 1's own flags minus `-E`/`-c`.
Every asl invocation is under `timeout`, because this set deliberately asks what
asl does with a file that includes itself.

## What asl does

| question | probe | answer |
|---|---|---|
| a 2-byte header included TWICE | `p1` | both copies emit — `1122` at `$1000` AND `$1002`, exit 0 |
| a file that includes ITSELF | `p2` | **not refused by name.** 199 copies emitted, then `error #10008: INCLUDE nested too deeply`, `fatal error, assembly terminated`, `ASL_EXIT=3` |
| mutual recursion, `b`↔`c` | `p3` | the same refusal, at `p3b.inc(2)`, `ASL_EXIT=3` |
| a DIAMOND, `a`→`b,c`→`d` | `p4` | `d` assembles TWICE |
| `x` vs `./x` vs `sub/../x` | `p5` | three inclusions — asl compares nothing, so canonicalization is a question it never asks |
| a repeated include DEFINING a label | `p6` | `error #1000: symbol double defined`, `ASL_EXIT=2` — and it emits the bytes anyway |
| `n set n+1 / dc.b n`, included 3× | `p7` | `01 02 03` — the copies are not required to agree |
| does `NESTMAX` (=100) govern the bound? | `p8` | **no.** `nestmax 5` leaves it at 199 |

**The depth bound is 199 and it is about DEPTH, not repetition.** `depth.sh`
generates N distinct files, each including the next — a chain no re-inclusion
rule of any shape may touch:

```
./depth.sh 199  ->  ASL_EXIT=0, deepest level reached: 199
./depth.sh 200  ->  n199.inc(2):10: error #10008: INCLUDE nested too deeply
                    fatal error, assembly terminated, ASL_EXIT=3
```

The self-including `p2` stops at the same 199. And `siblings.sh 250` — 250
includes IN SEQUENCE, none nested — assembles clean and emits 250 bytes, so the
bound counts what is OPEN AT ONCE and nothing counts how many have run.

No probe timed out. The one shape that could have (`p2`) answered in well under
the 25s limit, by refusing.

## The hypothesis the parcel was dispatched with does not survive

The brief proposed tracking the ACTIVE INCLUDE STACK and refusing only a genuine
cycle. asl has no cycle detection to reproduce, and a stack rule is the wrong
shape twice over:

- **Unfaithful.** It refuses at a different point with a different message. asl
  reaches depth 199 first and says `INCLUDE nested too deeply`.
- **Insufficient, and this is the half that matters.** A generated 5000-deep
  chain of DISTINCT files contains no cycle. `directive_include` re-enters
  `Asm::exec`, so without a depth bound that chain is a native stack overflow —
  a crash instead of a verdict, where asl prints a diagnostic and stops. The
  bound has to be on the depth counter regardless of whether a cycle rule exists,
  and once it is there the cycle rule has nothing left to do.

The brief also framed the risk as "removing the guard would trade a silent
divergence for a hang." **The direction was backwards.** The guard was the thing
preventing the runaway case from being *detected*: `p2` and `p3` are programs asl
terminates on, and under the visited set sigil assembled both clean, exit 0, no
diagnostic, ROM written.

## Before and after, on the probe sources

Bytes from `$1000`; every probe sits at `org $1000`.

| probe | asl | sigil BEFORE (`4378716`) | sigil AFTER |
|---|---|---|---|
| `p1` | `1122 1122 99 00 00000006` (10 B) | `1122 99 00 00000004` (8 B), exit 0 | matches asl |
| `p2` | refused, exit 3 | **`aa`, exit 0, silent** | `INCLUDE nested too deeply` at `p2self.inc(2)` |
| `p3` | refused, exit 3 | **`b0c0`, exit 0, silent** | same refusal at `p3b.inc(2)` |
| `p4` | `b0 dd c0 dd 99 …` | `b0 dd c0 99 …` | matches asl |
| `p5` | three copies | one copy | three copies |
| `p6` | `#1000 symbol double defined`, exit 2 | **`112299`, exit 0, silent** | `symbol double defined: p6_sym` |
| `p7` | `01 02 03 99` | `01 99` | `01 02 03 99` |

`depth_sigil.sh` runs the generated chain through sigil: clean at 199, refused at
200, at the same site asl names (`n199.inc(2)`).

`p6` is the one row where a difference remains, and it is a diagnostic difference
only: asl raises the double-define AND emits the bytes, exiting 2; sigil raises
the same diagnostic and writes no image, because suppressing output on an error
is a property of the front end rather than of this directive. Sigil's answer is
still strictly better than before, where it was silent.

## What was built

- `visited: BTreeSet<PathBuf>` → `include_depth: u32`, bounded by
  `INCLUDE_NEST_MAX = 199` / `INCLUDE_NEST_TOO_DEEP` in `lib.rs`.
- The refusal sets `aborted`, matching asl's `fatal error, assembly terminated`.
  Measured, not assumed: against a source whose included file carries a trailing
  error line, the abort gives **1 diagnostic** and unwinding gives **200**. asl
  gives 1 on the same source and never reaches line 3 in any of its 199 open
  frames.
- `IncludeCensus` + `SIGIL_CENSUS_INCLUDE=1`, reporting
  `seen / executed / repeats / refused-too-deep` per pass.

## The engagement counter

A before/after diff of two diagnostic streams reads identically whether the
changed code engaged everywhere and agreed, or was never reached at all. The
counter separates them.

| tree | passes | per pass |
|---|---|---|
| aeon sonic4 | 51 | 48× `executed=0`, **3× `executed=1 repeats=0`** |
| aeon sonic4-debug | 51 | 48× `executed=0`, **3× `executed=1 repeats=0`** |
| aeon demo | 3 | **3× `executed=1 repeats=0`** |
| aeon demo-debug | 3 | **3× `executed=1 repeats=0`** |
| corpus s1 | 4 | **`seen=439 executed=439 repeats=0`** |
| corpus s2 | 5 | **`seen=339 executed=300 repeats=0`** |

`refused-too-deep=0` everywhere. s2's `seen` exceeding `executed` by 39 is the
missing-file path; the counters separate resolved-but-unreadable from executed.

**The whole aeon tree holds exactly TWO `include` lines** —
`games/sonic4/game_root.asm:35` and `games/demo/game_root.asm:32`, both
`include "engine/debug/debugger.asm"`. Everything else is `.emp`. So the corpora,
not aeon, are where this code is exercised: 739 include sites per pass between
them, and zero repeats.

## What the four-shape identity does and does not attest

All four aeon shapes hold CRC32 and size across the parcel, and both corpora's
diagnostic streams are byte-identical before and after. **That attests the
plumbing moves no byte. It attests nothing whatever about the behaviour the
parcel changed**, and three controls say so rather than leaving it to be trusted:

- **C1 — `panic!` at the top of `directive_include`.** All four shapes FAIL,
  `BUILD_EXIT=101`, `panics=1`. The directive IS reached, once per game root.
- **C2 — `panic!` on the REPEAT path**, the branch the old guard used to take.
  All four shapes BUILD, `BUILD_EXIT=0`, `panics=0`. aeon's population of
  repeated includes is empirically zero, so the changed behaviour never runs
  there.
- **The positive control (`poscontrol.sh`)** — duplicate that one `include` line
  in a COPY of the aeon tree and build it with both binaries:

  | binary | result |
  |---|---|
  | branch-point | `BUILD_EXIT=0`, **crc32=`1c09fbfc`, size=819131 — byte-identical to the UNDOUBLED tree** |
  | parcel | `BUILD_EXIT=1`, `symbol double defined: DEBUGGER__EXTENSIONS__ENABLE`, 35 diagnostics, no image |

  The branch-point row is the divergence on real aeon source: a source edit that
  must move the ROM, silently producing the identical ROM. The parcel row is asl's
  own answer (`p6`). The census on the doubled tree reads
  `executed=2 repeats=1` in the three passes that see the AS root, against
  `executed=1 repeats=0` untouched.

## Gates

`mutations.sh`, six mutations, each shown applied on disk, each restored with
`git checkout --` from the committed baseline. The script reports
APPLIED-AND-STILL-GREEN separately from RED, and **that clause earned its keep on
the first run: three of six mutations landed and left every test passing.** All
six are RED now.

| mutation | must fail | result |
|---|---|---|
| m1 restore the visited-set DAG guard | the four byte fixtures + both cycle fixtures | RED, 7 failed |
| m2 bound 199 → 198 | `the_bound_constant_matches_what_asl_measured`, the clean half of the boundary pair | RED, 2 failed |
| m3 no depth bound at all | both cycle fixtures + the refused half — as a process abort, which is the crash this bound prevents | RED |
| m4 refusal returns instead of aborting | `the_depth_refusal_terminates_the_assembly` | RED, 1 failed |
| m5 `include_depth` never decremented | `sibling_includes_do_not_accumulate_depth` | RED, 1 failed |
| m6 census never counts a repeat | `census_selfcheck.sh`, all four rows | RED, `SELFCHECK_EXIT=1` |

### The three fixtures that could not disagree with the code

- **m2** — the fixtures built their chains FROM `INCLUDE_NEST_MAX`, so moving the
  constant moved the input too. asl's 199/200 are now local constants in the test
  file; the crate constant is imported in exactly one test, to be compared.
- **m4** — the old fixture counted the refusal and got 1 either way: its included
  file ENDED at the failing include, so there was nothing after it to re-run on
  the way out. The replacement's included file carries a trailing error line.
  *A vacuity inside that fix, kept because it will recur:* the first draft used an
  UNDEFINED SYMBOL as the trailing line and stayed green — undefined names are
  poisoned and promoted at the end of the converged pass, and a run already
  holding an error never gets there. The trailing line must be an error the front
  end raises immediately.
- **m5** — every NESTED fixture stays green: the diamond reaches depth 2, the
  cycles are refused anyway, the 199-chain is at the bound either way. The whole
  decrement was unpinned until `sibling_includes_do_not_accumulate_depth`.

m6 was originally declared MUST-FAIL-NOTHING, which is not a gate. The counter is
evidence in this note — *"an empty diff beside `repeats=0`"* means nothing unless
`repeats` would have been non-zero had there been repeats — so it now has
`census_selfcheck.sh`, calibrating it against four sources whose counts are
readable off the file (`p1`→1, `p4`→1, `p5`→2, `p7`→2; executed 2/4/3/3).

## Left open

- **`p6`'s byte behaviour under an error.** asl emits the bytes of a
  double-defining repeated include and exits 2; sigil raises the same diagnostic
  and writes nothing. This is the front end's general no-output-on-error rule,
  not this directive's, and changing it is a much wider parcel.
- **`IncludeCensus::seen` is a measurement, not a rule.** Nothing consults it to
  decide anything and it is only allocated under `SIGIL_CENSUS_INCLUDE`. Kill
  condition: when a report no longer needs the repeated-include population
  enumerated, the field and the `repeats` counter go, and
  `census_selfcheck.sh` goes with them.
