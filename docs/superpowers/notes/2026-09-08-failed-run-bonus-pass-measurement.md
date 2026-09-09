# Skipping the bonus pass on a failed run: the measurement, and the STOP

2026-09-08. Branch `parcel/failed-run-bonus-pass` off master `6f86cb90`. **The change is not in
this branch.** The measurement was the gate, it came back not-clean on one of its three stop
conditions, and the verdict is the delivery.

## The verdict in one screen

| Stop condition | Verdict |
|---|---|
| A diagnostic class that appears ONLY without the bonus pass | **TRIPPED.** `unresolved long expression`, 2 instances, `sonic_hack` root. |
| An error that becomes a deferral (the run gets QUIETER) | **NOT TRIPPED.** Zero diagnostic lines vanish on any of the nine roots, in any class. The delta is additive everywhere. |
| An exit-status flip | **NOT TRIPPED.** All nine AS roots exit 1 both ways; all three aeon shapes exit 0 both ways with byte-identical ROMs. |

And the largest finding is not on that list:

**On Sonic 1 the bonus pass does not run at all, so the 33% it was said to cost is not there to
recover.** `poison` is EMPTY at s1disasm's convergence, so the run already takes the existing
poison-free early return. Its three traversals are three ORDINARY passes: it converges at pass
index 2, not 1. Measured with the cut applied, s1 costs 0.8330 s against a baseline 0.8152 s, a
ratio of 1.022 -- no change, in the direction of noise.

## Provenance

| Instrument | Identity |
|---|---|
| baseline | `sigil 0.1.0 (6f86cb90)`, md5 `5b325616571b50a6b52a87b80835eb46`, built with `CARGO_TARGET_DIR=.worktrees/bonus-pass/.target-parcel` |
| cut | the same tree plus the candidate gate, md5 `b7b7d5ed8ce063b3d868d396b04b9d7d` |
| probe | the cut plus one env-gated `eprintln` at the convergence site (`SIGIL_PROBE_CONVERGE`), used for the poison/error census below and then removed |
| corpora | PRIVATE `rsync -a --exclude .git` copies under `/home/volence/sonic_hacks/.scratch/bonus-pass/corpus/`. No shared checkout was read during a timed run. |
| timing | `.scratch/bonus-pass/timecpu.py`: child CPU from `getrusage(RUSAGE_CHILDREN)` differenced across one call, wall from `perf_counter`, `/proc/loadavg` sampled before every rep, base and cut INTERLEAVED one rep at a time, 7 reps, page cache warmed once per root first |
| load | load1 2.89 to 4.77 across the whole timing run, median 3.38 |

Corpus revisions, and their dirt, which is inherited by BOTH sides equally and so cannot move a
difference: s1disasm `f6ece65` (2 modified `artnem/*.nem` blobs, 2 untracked), s2disasm `e45ebf3`
(clean), s2disasm-mompass-clean `e45ebf3` (clean), skdisasm `2fcd861` (2 untracked save files),
sonic_hack `858af72` (17), Sonic-Clean-Engine `d2f1988` (clean), aeon reference `ec640bcf` (clean).
The Batman disassembly and MD-OS are not git repositories.

## How the roots were enumerated

Not from the campaign's "six roots" phrase. Every directory under `/home/volence/sonic_hacks/` was
listed and kept if it held a top-level `sonic|s1|s2|s3|sk.asm` or a `build.{lua,bat,sh}`; the
survivors were then read to find what assembler each build script actually invokes. That found
**nine** AS-frontend roots where the campaign's phrase names three, because skdisasm carries two
roots rather than one, `s2disasm-mompass-clean` is a second s2 checkout, and three trees outside
the Sonic disassemblies (`sonic_hack`, the S.C.E. engine, and a Batman & Robin disassembly that
shares sonic_hack's `asl`) are AS corpora nobody had run this question against. MD-OS is included
as an adversarial tenth-of-a-kind: its `_Assemble.bat` runs `Asm68k.exe`, a DIFFERENT assembler, so
sigil's AS front end sees a dialect it was never meant to read. A failing run is the population
under test, so that is a feature.

The three aeon shapes were measured too, because they are the shipping path.

## What the run actually does at the convergence site, per root

`SIGIL_PROBE_CONVERGE=1`, one line per root, printed at the moment convergence is detected.

| Root | converged at pass | `poison` | errors so far | takes the bonus pass? |
|---|---|---|---|---|
| s1disasm `sonic.asm` | 2 | **0** | 50 | **no** |
| s2disasm `s2.asm` | 3 | 24 | 5205 | yes |
| s2disasm-mompass-clean `s2.asm` | 3 | 24 | 5205 | yes |
| skdisasm `sonic3k.asm` | 2 | 1 | 2423 | yes |
| skdisasm `s3.asm` | 2 | **0** | 1433 | **no** |
| sonic_hack `S4.asm` | 3 | 19 | 3092 | yes |
| S.C.E. `Engine/Includes.asm` | 1 | **0** | 87 | **no** |
| Batman `batman.asm` | 1 | 1348 | 9352 | yes |
| MD-OS `Source.asm` | 1 | **0** | 908 | **no** |

Four of the nine failing roots never reach the bonus pass, and one of the four is the root the
whole proposal was costed against. The traversal counts confirm it independently
(`SIGIL_CENSUS_BUDGET=1`, lines counted): s1 is 3 both ways; batman 3 -> 2, skdisasm 4 -> 3,
s2disasm 5 -> 4, sonic_hack 5 -> 4.

## The diagnostic sets, compared in both directions

Exact-line multiset compare of stderr, sorted, `comm` both ways. Stdout compared with `cmp`.

| Root | exit b/c | lines base | lines cut | vanished | appeared | new classes | stdout |
|---|---|---|---|---|---|---|---|
| s1disasm | 1/1 | 50 | 50 | 0 | 0 | none | same |
| s2disasm | 1/1 | 5229 | 5229 | 0 | 0 | none | same |
| s2disasm-mompass-clean | 1/1 | 5229 | 5229 | 0 | 0 | none | same |
| skdisasm sonic3k | 1/1 | 2424 | 2424 | 0 | 0 | none | same |
| skdisasm s3 | 1/1 | 1433 | 1433 | 0 | 0 | none | same |
| sonic_hack | 1/1 | 3108 | 3111 | **0** | **3** | **`unresolved long expression`** | same |
| S.C.E. | 1/1 | 87 | 87 | 0 | 0 | none | same |
| Batman | 1/1 | 9466 | 10700 | **0** | **1234** | none | same |
| MD-OS | 1/1 | 908 | 908 | 0 | 0 | none | same |
| aeon sonic4 | 0/0 | ROM `B09CCD65`, 820229 B, byte-identical | | 0 | 0 | none | same |
| aeon sonic4 --debug | 0/0 | ROM `1B7FE316`, 846529 B, byte-identical | | 0 | 0 | none | same |
| aeon demo | 0/0 | ROM `0AD17404`, 96863 B, byte-identical | | 0 | 0 | none | same |

**Nothing goes quiet anywhere.** The dangerous direction, an error becoming a silent deferral, does
not occur on any root: the `vanished` column is zero across the board, and it is zero at the level
of exact lines, not of counts.

The Batman +1234 are all one already-present class, `unresolved symbol \`X\` in operand` (base emits
114 of them, cut 1348 -- exactly the 1348 `poison` entries the probe counted). That is the class the
brief predicted: a `jsr`/`jmp` bare-symbol target the bonus pass would have deferred as
`Fragment::JmpJsrSym`. In this corpus they are targets like `loc_005406.l`, an operand syntax the
front end does not parse, so it reads the whole thing as one symbol name.

## The class that trips the gate, and the mechanism the brief did not have

sonic_hack gains 3 lines: one `unresolved symbol \`LoadLevelLayout\` in operand`, which is the
predicted class, and **two of a class that does not otherwise exist in that run**:

```
code/engines/art_data.asm(23): error: unresolved long expression
code/engines/art_data.asm(24): error: unresolved long expression
```

Those are the `dc.l (plc1<<24)|art` lines inside the `levartptrs` macro. The mechanism is NOT the
`jsr`/`jmp` deferral arm. It is `keep_labels_symbolic()`, which is true on the bonus pass and only
there, and which short-circuits `directive_dc_l` BEFORE the fold:

```rust
if self.keep_labels_symbolic() && self.expr_refs_label(&qe) {
    self.emit(&[0,0,0,0], vec![Fixup { kind: Abs32Be, offset: 0, target: self.relax_safe_fold(&qe) }], span);
    continue;
}
match self.fold(&qe) { ... Fold::Poison => { ... self.err(span, "unresolved long expression") } }
```

A compound label-referencing operand that folds to Poison therefore raises nothing on the bonus
pass and raises an error on an ordinary one. `keep_labels_symbolic()` guards **seven** sites in
`eval.rs` at master `6f86cb90` (5725, 6394, 6472, 6863, 6896, 7133, 7980), each a place where the bonus pass takes a
symbolic short-circuit that skips a fold and so skips whatever diagnostic that fold would have
raised. The suppression surface is therefore seven sites wide plus the `jsr`/`jmp` arm, not one
class, and only one corpus in nine happened to land on the second kind. That is the reason the stop
condition exists and the reason a nine-root sweep was worth more than a three-root one.

It is worth saying plainly which behaviour is the defensible one: **the bonus pass is currently
suppressing real errors on runs that are going to fail anyway.** Dropping it does not invent the
two `unresolved long expression` lines, it stops hiding them. But that is an argument for a
deliberate change of what a failing run reports, made by the owner, and not something to take as a
side effect of a performance cut.

## What the cut buys, per root

7 interleaved reps, CPU (user+sys) seconds, load1 2.89 to 4.77 (median 3.38) throughout. Wall is
printed beside CPU only so the contamination stays visible; every ratio is over CPU.

| Root | base CPU med | cut CPU med | ratio | base CPU range | cut CPU range | base wall med | cut wall med |
|---|---|---|---|---|---|---|---|
| s1disasm | 0.8152 | 0.8330 | **1.022** | 0.7894-0.8961 | 0.8093-1.0684 | 0.8207 | 0.8555 |
| s2disasm | 2.5684 | 2.0641 | 0.804 | 2.5378-2.6077 | 2.0230-2.0925 | 2.6144 | 2.1117 |
| s2disasm-mompass-clean | 2.5592 | 2.0854 | 0.815 | 2.5111-2.6713 | 2.0540-2.1299 | 2.5836 | 2.0929 |
| skdisasm sonic3k | 2.5724 | 1.9353 | 0.752 | 2.5340-2.6365 | 1.8884-1.9977 | 2.6167 | 1.9869 |
| skdisasm s3 | 1.3957 | 1.4120 | 1.012 | 1.3727-1.4579 | 1.3934-1.4494 | 1.4063 | 1.4539 |
| sonic_hack | 0.3942 | 0.3128 | 0.794 | 0.3869-0.5321 | 0.3071-0.3699 | 0.3997 | 0.3165 |
| S.C.E. | 0.0012 | 0.0010 | 0.806 | 0.0012-0.0014 | 0.0009-0.0012 | 0.0013 | 0.0011 |
| Batman | 0.3311 | 0.2263 | **0.684** | 0.3239-0.3461 | 0.2194-0.2332 | 0.3380 | 0.2270 |
| MD-OS | 0.0044 | 0.0041 | 0.947 | 0.0043-0.0045 | 0.0040-0.0042 | 0.0045 | 0.0043 |

Every ratio is `(n-1)/n` over the traversal counts, which is the whole model and it fits without
being fitted: Batman 2/3 = 0.667 measured 0.684, skdisasm 3/4 = 0.750 measured 0.752, s2disasm
4/5 = 0.800 measured 0.804, sonic_hack 4/5 = 0.800 measured 0.794. Of the four roots that take no
bonus pass, the two with measurable runtime (s1disasm, skdisasm s3) sit at 1.02 and 1.01, which is the
control: the change costs nothing where it does nothing. S.C.E. and MD-OS are 1.2 ms and 4.4 ms
runs; their ratios are noise and are printed only so no root is silently dropped.

So the honest headline is: **the cut buys 20% to 32% on the five roots that carry leftover poison,
and 0% on Sonic 1**, which is the opposite of how the proposal was costed.

## The shipping path, established rather than asserted

Three independent ways, in increasing strength:

1. **Enumeration.** `force_relocate` is a parameter of one private function, `eval::run_impl`, with
   exactly three callers: `run`, `run_located` (both `false`) and `run_relocating` (`true`).
   `run_relocating` has one wrapper pair in `lib.rs`, and outside tests exactly ONE consumer in
   `crates/*/src`: `sigil-harness/src/native.rs:1538`, whose comment says why ("Every build CHAINS").
   The non-relocating route has exactly one consumer outside tests, `sigil-cli/src/main.rs:111`,
   the bare `sigil <file>.asm` convenience command. So the shipping ROM path and the corpus path
   are disjoint entry points, not two flags on one.
2. **The gate's shape.** The candidate condition is `!force_relocate && (poison.is_empty() ||
   already_failed)`, and `force_relocate` is the FIRST conjunct, so the whole arm is unreachable on
   any relocating build regardless of what the rest evaluates to.
3. **Measurement.** All three aeon shapes built with both binaries from a private copy of the
   reference tree: `sonic4` CRC32 `B09CCD65` / 820229 B, `sonic4 --debug` `1B7FE316` / 846529 B,
   `demo` `0AD17404` / 96863 B, `cmp` byte-identical in every case, exit 0 both ways, stdout and
   stderr byte-identical.

One caveat that belongs with this, because the obvious stronger claim is false. It is tempting to
say the change cannot affect any run that exits 0. It cannot affect any run that exits 0 **today**,
but it is not structurally guaranteed: a run whose ONLY errors are ones the bonus pass suppresses
(the `keep_labels_symbolic` family above) would flip from `Ok` to `Err`. No root in the corpus is
such a run, and no aeon shape can be one because it goes through `force_relocate`. The claim to
make is the measured one, not the structural one.

## What the existing suite says

`cargo test --release -p sigil-frontend-as` with the change applied: **644 passed, 0 failed, 0
ignored**, exit 0. That is a finding rather than a reassurance: nothing in the suite pins the
behaviour of the bonus pass on a failing run in either direction, so the suite would not have
caught the sonic_hack regression either. Whatever is decided, the decision needs its own test.

## What in the brief turned out to be wrong

1. **"On Sonic 1 this bonus pass is 33 percent of the run."** It is 0 percent of the run: s1disasm's
   `poison` is empty at convergence and the bonus pass never runs. Its third traversal is an
   ordinary pass, and it exists because s1 converges at pass index 2 rather than 1. The 33% figure
   in `2026-09-08-s1-build-profile.md` (Item 2's "Pass 3 is the bonus pass after convergence, and on
   this input it is forced by `poison`", and Item 6's first row) is wrong, and Item 8's whole
   costing of the fix rests on it. Item 6's "both of the above -> parity with asl" does not survive
   either: the 0.292 s it wanted to delete is a convergence pass sigil needs.
2. **"Its work is dropped, because the build fails and there is no linker"** is right for the five
   roots that reach it and vacuous for the four that do not.
3. **The predicted risk was the right risk but the wrong mechanism, and one mechanism short.** The
   brief expected loudness from the `jsr`/`jmp` deferral arm, and that is where 1235 of the 1237 new
   lines come from. The two that trip the gate come from `keep_labels_symbolic()`, a second and
   separate suppression the brief does not mention, guarding seven sites.
4. **"Every corpus root available, at least s1disasm, s2disasm, skdisasm"** understates what is on
   disk by a factor of three; the enumeration is in its own section above.
5. Not wrong, but worth recording: the brief's own warning about wall time was earned twice over.
   The four roots where the change does nothing sit at ratios 1.02, 1.01, 0.95 and 0.81 in CPU; the
   0.81 is a 1.2 ms run and the 0.95 a 4.4 ms one. Read as speedups they would be three false
   positives.

## The verdict

**STOP, not landed.** One of the three stop conditions fired. The change is a real 20% to 32% cut
on five of nine corpus roots, it makes no run quieter anywhere, and it cannot touch a shipping ROM
-- but it makes one root report a class of error it does not report today, which is exactly the
outcome the gate was written to catch.

The decision it needs from the owner is not a performance decision. It is: **should a failing AS
run report the diagnostics of an ordinary pass, or the quieter set the deferral pass produces?**
The measurement says the ordinary set is strictly larger and never smaller. If the answer is "the
ordinary set", the change lands with a test pinning both the gating and the sonic_hack lines. If
the answer is "keep today's output exactly", the cut is not available in this form, and the 20% to
32% would have to come from somewhere else.
