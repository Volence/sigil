# The reference assembler is a BUILD, and the banner cannot name it

2026-09-05.

Four `asl` binaries in this workspace print

```
Macro Assembler 1.42 Beta [Bld 212]
```

**verbatim**, and they are not the same program. A runner, a note or a test
comment that cites the version string has therefore identified nothing. This
directory holds the guard that identifies the instrument by digest instead, and
the selfcheck that proves the guard refuses.

| path | md5 | fork | second banner line |
|---|---|---|---|
| `s1disasm/build_tools/Linux-x86_64/asl` | `61e672562465725a8c102288a7da9098` | upstream (Arnold, MPC821 additions) | `(x86_64-unknown-linux)` |
| `skdisasm/build_tools/Linux-x86_64/asl` | `61e672562465725a8c102288a7da9098` | same binary | `(x86_64-unknown-linux)` |
| `sonic_hack/tools/as/asl` | `61e672562465725a8c102288a7da9098` | same binary | `(x86_64-unknown-linux)` |
| `s2disasm/build_tools/Linux-x86_64/asl` | `0dee1f98e6480a4783d27ffd8b90896f` | flamewing | `(x86_64-Linux)` |

`61e67256…` is the **reference build**. `0dee1f98…` is the **varying build** and
is refused.

## What the varying build does

For any operand it declined to give a value it substitutes an **uninitialized
memory read**, so it answers differently on every run. Measured on
`../2026-09-04-as-end-probes/wrange.asm`, whose four range-refused immediates
come back, on four consecutive runs:

```
303C 5602    303C 55B1    303C 5655    303C 557F
```

against `303C 8000` on every run of the reference build. `wimm.asm` behaves the
same way (`5570` / `5632` / `555B` / `561E` against a steady `0000`).

**The regime is wider than it looks and the loud case is not the dangerous one.**
`move.w #zz,d0` with `zz` undefined is unstable and exits 2 with a diagnostic;
`#f(<register>)` — a `function` call in an immediate whose argument is a
register name — is unstable with **exit 0 and no diagnostic at all**, which is
the shape a tool will accept and freeze. Under `setarch -R` the value collapses
to a constant `$5555`, which is what identifies the mechanism as an
uninitialized read rather than a hash or a counter. `../2026-09-05-asl-nondeterminism-sweep-probes/`
carries the characterisation and the sweeps.

## The guard

`asl_ref.sh` is sourced, not run:

```sh
. "$(dirname "$0")/../asl-reference/asl_ref.sh" || exit $?
```

The `|| exit $?` is load-bearing — these runners do not set `-e`, so a sourced
guard that merely returns non-zero stops nothing. It exports `$ASLDIR`, `$ASL`
and `AS_MSGPATH`, and refuses (exit 3) any binary whose md5 is not the literal
in the file. `ASLDIR` may be overridden, and pointing it at another build is how
the refusal is demonstrated; there is no escape hatch, because a runner that can
be talked into the varying build has the defect back.

The expected digest is **written out**, never computed from the binary being
checked: a guard whose expectation derives from its subject moves with the
subject and can never come out red.

## Proof it fires

`./selfcheck.sh` — exit 0 means every case held.

```
case 0  both builds print the SAME banner, so a version check cannot discriminate
case 1  the reference build is ACCEPTED
case 2  THE VARYING BUILD IS REFUSED                      ← the load-bearing case
case 3  the refusal names both the wanted and the seen digest
case 4  a missing binary is refused, not silently skipped
case 5  with the comparison STUBBED TO ACCEPT, case 2 goes RED
```

Case 5 is the selfcheck's own honesty check, and it verifies the stub APPLIED
before drawing a conclusion from it — a stub that fails to apply runs the
original guard and produces the same "refused" answer as a working one.

## Which runners are pinned

| runner | before | now |
|---|---|---|
| `../2026-09-04-as-end-probes/run.sh` | `0dee1f98…` | guard |
| `../2026-09-04-as-warning-exitm-probes/run.sh` | `0dee1f98…` | guard |
| `../2026-09-04-as-warning-exitm-probes/img.sh` | `0dee1f98…` | guard |
| `../2026-09-05-as-interp-radix-probes/run.sh` | `0dee1f98…` | guard |
| `../../probes/2026-09-03-align/gen_org.sh` | `0dee1f98…` | guard |
| `../../probes/2026-09-03-align/run.sh` | `0dee1f98…` by default | guard by default; `[asl-dir]` still reaches a second build and prints its md5 |

**Two runners name the varying build and keep it, deliberately.**
`../2026-09-05-asl-nondeterminism-sweep-probes/aslr.sh` defaults to it because
demonstrating the instability is its whole job, and
`../../probes/2026-09-03-align/gen_org_both.sh` puts each question to both
builds by design. Neither is anonymous: both print the md5 of every binary they
run. **A guard on those two would delete the capability, not protect it** — the
defect was never a second build, it was an unidentified instrument.

## What repinning cost

All 71 probes in the four repinned corpora were assembled under BOTH builds and
their emitted code lines compared. **Exactly two differ** — `wrange.asm` and
`wimm.asm`, the only two that carry operands asl refuses. Every other committed
answer in those four directories is identical under either build, so the tables
carry over without being re-taken.

The one committed table repinning invalidates is the asl listing pasted into
`crates/sigil-frontend-as/tests/as_word_immediate_range.rs`'s module doc, whose
four `303C 55F5` words are one draw of uninitialized memory frozen into a
comment. The supersession is written beside it there, and the listing is kept
rather than swapped so it is visible instead of silent. That file's assertions
are unaffected: they assert acceptance and refusal, and the `range overflow`
diagnostics are identical under both builds.

## Left open

Roughly a dozen runners already name the reference build by path without a
digest check (`../2026-09-03-as-struct-probes/`, `../2026-09-03-irp-irpc-probes/`,
`../2026-09-04-as-enum-probes/`, `../2026-09-04-as-symbol-class-probes/`,
`../2026-09-05-as-include-repeat-probes/`, `../2026-09-05-as-macro-body-label-probes/`,
`../../../../scripts/z80_byte_sweep.sh`, and the `.f1probe`/`.s1probe` scratch
copies). They are pinned to the right binary and nothing observed is wrong with
them; sourcing this guard would make that a checked property rather than a
correct path. `docs/OVERSEER.md` also cites `s2disasm/build_tools/Linux-x86_64/asl`
without a digest and is not this parcel's file to edit.
