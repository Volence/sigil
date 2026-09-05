# `disp(An)` versus `function(args)` — the probes

The measurements behind `../2026-09-05-disp-or-call.md`. **Reconstructed, not
original.** That note took its rows on the s2disasm build and cited it by
version banner; its probe files (`ta.asm`, `te.asm`, and the rest) were never
committed, so every row it states was, until this directory existed, a value
with no input that reproduces it. Each file below was rebuilt from the note's
own listings — its line numbers, addresses and byte columns are quoted there,
and reproducing them exactly is what says the reconstruction is faithful.

## The instrument

Two builds, deliberately, and both named by MD5 because the banner cannot tell
them apart — all four `asl` binaries in this workspace print
`Macro Assembler 1.42 Beta [Bld 212]` verbatim (`../asl-reference/`).

| role | path | md5 |
|---|---|---|
| reference | `s1disasm/build_tools/Linux-x86_64/asl` | `61e672562465725a8c102288a7da9098` |
| varying | `s2disasm/build_tools/Linux-x86_64/asl` | `0dee1f98e6480a4783d27ffd8b90896f` |

Flags are Sonic 2's own minus the two that only redirect output, exactly as the
parent note states them: `asl -xx -n -q -A -L -U -i .`.

## What each probe establishes

| probe | subject | build-independent? |
|---|---|---|
| `d1` | the structural peel: `dsp(a1)`, `1+dsp(a1)` take the EQUATE while `#dsp(k)` takes the FUNCTION | yes |
| `d2` | the same rule as a refusal — the caret sits under `konst`, not under `a1` | yes (diagnostic) |
| `d3` | a two-element trailing group is an addressing mode with a bad index, never a two-argument call | yes (diagnostic) |
| `d4` | the exclusion is the trailing GROUP, not the operand: a call in the displacement still expands | yes |
| `d5` | `1+konst(a1)` with `konst` both an equate and a parameter-free function — six bytes, exit 0, silently a different program from sigil's | yes |
| `d6` | the immediate `#konst(a1)`, the shape the note declined to build a rule on | **no** |
| `d7` | whether `#f(<reg>)` has an answer when the body USES its parameter, or only when it ignores it | **no** |
| `d8` | whether an equate of the same name is what decides `#name(<reg>)` — four cells, one discriminator each way | **no** |
| `d9` | what the reference build actually substitutes for an operand it declined to value | **no** |

`d7`, `d8` and `d9` are new here; the note asserted a rule about `#f(<reg>)`
without a probe that could separate the candidate causes.

## The result that matters

**A declined operand is not answered by either build. The difference is only
whether the wrong answer repeats.**

`d9` is the decisive one. Three `#f(<register>)` immediates, each preceded by a
successful `#f(5)` call with a distinct value:

```text
reference build 61e67256                    varying build 0dee1f98
 17/ 1000 : 303C 0111  move.w #one(5),d0      303C 0111
 18/ 1004 : 303C 0111  move.w #one(a1),d0     303C 5630
 19/ 1008 : 303C 0222  move.w #two(5),d0      303C 0222
 20/ 100C : 303C 0222  move.w #two(a1),d0     303C 5630
 21/ 1010 : 303C 0333  move.w #three(5),d0    303C 0333
 22/ 1014 : 303C 0333  move.w #three(a1),d0   303C 5630
```

Each declined line on the reference build **echoes the last value that build
computed** — three different echoes, tracking the three different predecessors.
It is not a zero and it is not an answer; `d8`'s uniform `0000` was only the
initial state of that slot, because in `d8` no call had succeeded yet. The
varying build reads uninitialized memory instead, which is why its three lines
agree with each other within a run and differ between runs.

So the reference build's reproducibility on this shape is a property of SOURCE
ORDER, not of the operand. Exit 0, no diagnostic, on both builds. **A value
frozen off the reference build for an operand asl declined is a stale-buffer
artifact of the lines above it** — the most freezable shape there is, because it
survives a re-run.

## Running them

```sh
./run.sh d1.asm                 # the reference build, digest-pinned
./run.sh d1.asm /path/to/other  # a second build, md5 announced
./both.sh 4                     # every probe, both builds, 4 runs each
```

Four runs, not one: a single run of an unstable row looks exactly like a stable
one. `both.sh` names both builds by md5 above their blocks and guards neither,
because comparing them is the job.
