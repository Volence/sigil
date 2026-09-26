# Which banner-only asl citations actually depend on the binary

2026-09-26 · branch `parcel/asl-banner-dependence` · sigil master base `9212d6e0`

The question `STABILITY-RUNNER-MISSING-WHERE-CLAIMED` left open on 2026-09-16: of the notes that
cite their `asl` oracle by version banner and never by md5, which state a claim that DEPENDS on
which binary produced it? This note answers per note, against what is actually known to differ
between the builds, and re-measures on all four builds wherever a note left an input behind or its
shape could be rebuilt from its own text.

**Answer: 3 DEPENDS, 19 DOES-NOT-DEPEND, 0 UNDETERMINED. Every DEPENDS row is already settled or
superseded; no re-run is owed.** The per-note table is below; the three rows are at the end.

## The population

```sh
./scripts/sweep_asl_citation_form.sh      # at 9212d6e0, run by path from this worktree
```

```text
tree:   9212d6e0  branch: parcel/asl-banner-dependence
cite the banner:            52
cite an identifying md5:    91
positive control OK: the repaired instance cites an md5
count: 22
```

The script is the authority on the population and these figures are its output at that tree. Its
own header states its limits, and one of them matters below: it matches the banner text and the two
FULL 32-hex digests `61e672562465725a8c102288a7da9098` and `0dee1f98e6480a4783d27ffd8b90896f`. It
does not match an 8-hex digest prefix, and it counts a note that merely MENTIONS the banner the same
as one that cites it.

## The binaries, and what is known to differ between them

Measured today with `md5sum` on each path, and the banner by running each binary with no arguments:

| path | md5 | banner line 2 | family |
|---|---|---|---|
| `s1disasm/build_tools/Linux-x86_64/asl` | `61e672562465725a8c102288a7da9098` | `(x86_64-unknown-linux)` | upstream; the REFERENCE build `asl_ref.sh` pins |
| `skdisasm/build_tools/Linux-x86_64/asl` | `61e672562465725a8c102288a7da9098` | same binary | |
| `sonic_hack/tools/as/asl` | `61e672562465725a8c102288a7da9098` | same binary | |
| `s2disasm/build_tools/Linux-x86_64/asl` | `0dee1f98e6480a4783d27ffd8b90896f` | `(x86_64-Linux)`, `(C) 2022-2024 flamewing` | flamewing fork |
| `s1disasm/build_tools/Linux-x86/asl` | `a8cd8b80b765686b2e9266c31ffa6987` | `(i386-unknown-linux)` | upstream, i386 |
| `skdisasm/build_tools/Linux-x86/asl` | `a8cd8b80b765686b2e9266c31ffa6987` | same binary | |
| `s2disasm/build_tools/Linux-x86/asl` | `aa6de52f266cef0a7f60a748919ab1d3` | `(x86_64-Linux)`, `(C) 2022-2024 flamewing` | flamewing fork, ELF 64-bit in the 32-bit slot |

Every one prints `Macro Assembler 1.42 Beta [Bld 212]` as its FIRST line. The second line is not
identical: it separates three families (upstream x86-64, upstream i386, flamewing). Only
`0dee1f98` and `aa6de52f` print a fully identical banner. A note quoting the second line has
therefore narrowed its oracle to a family, which two notes below do.

**Path identity at the dates in question.** The two paths the population names are git-tracked and
clean, and each blob predates every note here:

| path | last commit touching it | blob md5 at HEAD |
|---|---|---|
| `s1disasm/build_tools/Linux-x86_64/asl` | `6ebe25b`, 2026-06-17 | `61e672562465725a8c102288a7da9098` |
| `s2disasm/build_tools/Linux-x86_64/asl` | `fd0c9e8`, 2024-08-21 | `0dee1f98e6480a4783d27ffd8b90896f` |
| `aeon/tools/asl` (deleted at aeon `5e1b2b9d`, 2026-07-30) | added `1c7f81e6`, 2026-04-24, never changed | `61e672562465725a8c102288a7da9098` (`git -C aeon show 5e1b2b9d^:tools/asl \| md5sum`) |

So a note citing `s1disasm/.../Linux-x86_64/asl` in September ran the reference build, one citing
`s2disasm/.../Linux-x86_64/asl` ran `0dee1f98`, and the five July notes citing `aeon/tools/asl` ran
the reference build: aeon carried exactly one blob at that path for its whole life. (A working copy
modified without a commit is not excluded by history; nothing suggests one.)

**What is known to differ.** From `docs/OVERSEER-REFERENCE.md` ("Selecting and citing the `asl`
oracle"), `asl-reference/README.md`, `2026-09-05-asl-silent-decline-regime.md` and
`2026-09-11-s2-residual-census.md`:

| # | difference | builds |
|---|---|---|
| K1 | the value substituted for an operand asl DECLINED to value: the last value computed (stable) vs uninitialized memory (varies per run) | stable on `61e67256`, `a8cd8b80`; varies on `0dee1f98`, `aa6de52f` |
| K2 | `dc.w <register>` aborts on an `assert` vs emits nothing | aborts on `0dee1f98`, `aa6de52f` |
| K3 | the second banner line, and so `*ARCHITECTURE` in a listing | three families, above |
| K4 | macro argument text truncated at 255 characters | `61e67256` truncates, so it cannot assemble whole Sonic 2 (274 x `#1010`); `0dee1f98` does not. The i386 and `aa6de52f` builds were not measured on this |
| K5 | everything else measured so far agrees: 71 probes across four corpora compared under the two x86-64 builds differ only on the two carrying declined operands (K1) | |

So the differences are narrow and every one has a recognisable shape: a declined operand's value, a
`dc.<size>` of a register, the architecture string, a macro argument over 255 characters, or a
whole-corpus Sonic 2 run (which reaches K4). **Timing is a sixth, by nature**: two different
programs have no reason to run in the same time. Each note is classified against those, not against
"could differ".

## Re-measured here, on all four builds

`2026-09-26-asl-banner-citation-dependence-probes/all4.sh <probe.asm>` runs a probe through every
build, three runs each, prints each build's md5 above its listing, and says `stable` or `VARIES`
across the three runs. Unguarded by design: the build is the subject.

**Positive control first**, because an instrument that says "all four agree" must be able to say
otherwise: `all4.sh ../2026-09-04-as-end-probes/wrange.asm` reports `VARIES` on `0dee1f98` (`55AD`,
`5578`, `55F5` on the four refused lines across three runs) and on `aa6de52f` (`564F`, `557B`,
`5611`), and `stable` `303C 8000` on `61e67256` and `a8cd8b80`. That is K1, reproduced, so the
agreements below are agreements an instrument able to show a difference produced.

| probe | from note | result on all four builds |
|---|---|---|
| `../2026-09-04-as-enum-probes/q10.asm` (committed) | enum README row 13 | identical, stable: exit 2, `#1820`, `dc.b a,b` lists `0000`, footer "listing possibly incorrect" |
| `enum_fwd_stale.asm` | same row, with `dc.b $5A` above | identical, stable: `dc.b a,b` lists `0101`. **Not 0, and not the carried `$5A`** |
| `empty_operand.asm` | macro-default-params, "Logged, not fixed" | identical, stable, exit 0: `21FC 0000 1234`, `2038 0000`, and still `2038 0000` directly after `dc.w $7777`. Zero is not a carry-over |
| `default_params.asm` | macro-default-params, "The rule, off the oracle" | identical, stable, exit 0; every row the note quotes reproduces (`00`/`<\|DEF2\|>`/`[]` ... `01`/`<\|DEF2\|99>`) |
| `shift_argcount.asm` | shift walk, main table and BLOCKED/ARGCOUNT | identical, stable, exit 0: `ARGCOUNT` `03` then `00`; the four-shift table renders as the note says |
| `shift_placeholder.asm` | shift walk, "Deliberately NOT replicated" | identical, stable, exit 0: placeholder bytes leak into strings and `strlen` identically on every build |
| `maclocal_untaken_arm.asm` | maclocal scope, "the untaken arm" | identical, stable, exit 0: `mcond 0` `67FE`, `mcond 1` `6704` |
| `../2026-09-04-as-silent-acceptance-probes/w3.asm` (committed) | silent acceptance 1a | identical, stable, exit 0: warning `#320`, `0902` |
| `../2026-09-04-as-silent-acceptance-probes/k3.asm` (committed) | silent acceptance 1 | identical, stable, exit 2: same `#1811`/`#1812`/`#320`/`#2050` lines |
| `align_zero.asm` | align-in-phase, "What changed" | identical: SIGFPE, exit 136, on every build and run |
| `align_negative.asm` | same | identical, stable, exit 0: `align -256` from `$101` lands at `$FF00` |
| `float_numeric.asm` | float handoff's compatibility target | identical, stable, exit 0: eleven scaled `sin`/`cos`/`sqrt`/`ln`/`exp`/`^`/`atan`/`log`/division results, same 32-bit words on all four, the i386 build included |

Ten probes written for this parcel sit beside the runner. Where a note's own probe was not
committed, the probe is RECONSTRUCTED from the note's text and says so in its header: it measures
the same mechanism, not the note's own file. Build-to-build agreement across 12 more shapes, with
the positive control live, is consistent with K5.

## Per note

DND is DOES-NOT-DEPEND. "Known-diff" means classified against K1 to K5 without a re-run;
"re-run" means a row of the table above.

| # | note | claim, briefly | verdict | reason |
|---|---|---|---|---|
| 1 | `2026-07-04-m1d-t0.1-padding-probes.md` | `restore` resets padding only on a CPU change (rows b, c, d) | DND | Accepted, fully valued shapes, exit 0; none of K1 to K4. Instrument recovered anyway: `aeon/tools/asl` was `61e67256` |
| 2 | `2026-07-04-m1d-t1-string-set-probes.md` | string `set`, infix `!` is XOR (`5!3` = `06`) | DND | Accepted shapes. Also carried as `t1_*` blocks in `crates/sigil-frontend-as/tests/snippets_golden.txt`, whose header records `asl-md5 61e672562465725a8c102288a7da9098` |
| 3 | `2026-07-04-m1d-t2-abs-ea-end-probes.md` | abs.w iff `[0,$7FFF] ∪ [$FF8000,…]`; `END` emits nothing | DND | Accepted shapes; `t2_*` golden blocks under the same md5 header |
| 4 | `2026-07-04-m1d-t3-jmpjsr-width-probes.md` | least fixpoint: `org $7FFA; jmp T` = `4EF8 7FFE` | DND | Accepted shapes; `t3_*` golden blocks except the boundary self-reference, which rests on the recovered `61e67256` identity |
| 5 | `2026-07-04-m1d-t4-macro-local-scope-probes.md` | macro `.`-local private per expansion (P1 `6702`) | DND | Accepted shapes; `t4_maclocal_twice` golden; P1/P3/P4 re-tested on `0dee1f98` by note 9 ("survive verbatim") |
| 6 | `2026-07-05-spec2-plan5-sandbox-float-handoff.md` | `as.*` = "bit-compatible with asl 1.42 Bld 212's numeric routines" | DND | A requirement, not a measurement. Re-run `float_numeric.asm`: all four builds give identical float results, so the target names one behaviour on that sample |
| 7 | `2026-09-03-align-in-phase-contradiction.md` | align rounds on the signed low 32 bits; `align 0` SIGFPE; `-256` acts as `$FF00` | DND | The rule was measured on both x86-64 builds by the note itself ("The binary is not the variable"), and `docs/superpowers/probes/2026-09-03-align/RESULTS.md` carries both md5s. The two corner claims re-run: identical on all four |
| 8 | `2026-09-03-as-intlabel-capture.md` | `{INTLABEL}` capture suppresses the definition; boundary rule; `label` opens the caller's scope | DND | Known-diff: `0dee1f98` by path, but every quoted row is an accepted, fully valued operand and every refusal is a verdict (`#1010`), not a value |
| 9 | `2026-09-03-as-maclocal-scope.md` | split is syntactic; asl order-unstable (`67FE` vs `6704`) | DND | Known-diff, plus re-run `maclocal_untaken_arm.asm`: identical on all four |
| 10 | `2026-09-03-as-macrosetup-three-sites.md` | probe rows (`MOMCPU` `0006 8000`, paren-transparent strings) and one corpus claim: "asl ... reports nothing at any of" 34 `.w` RAM-label lines | **DEPENDS** | The probe rows are DND. The corpus claim is a whole-`s2.asm` run on `0dee1f98`, the corpus K4 makes the builds disagree on. **Superseded, not open**: row `AS-WORD-IMM-RAM-LABEL` closed at `2026-09-04-as-end-probes/README.md` ("never a divergence ... asl refuses the shape"), measured under both builds with identical `range overflow` verdicts |
| 11 | `2026-09-03-as-name-composition-braces.md` | `{expr}` composes a name; `\{expr}` folds at binding | DND | Known-diff: accepted shapes; the `.here-Outer` refusal is a verdict |
| 12 | `2026-09-03-as-shift-macro-argument-walk.md` | two-vector shift; empty-parameter placeholder leaks; `ARGCOUNT` 3 to 0 | DND | Re-run `shift_argcount.asm` and `shift_placeholder.asm`: identical on all four. The macro-argument path is where the fork is known to differ (K4), which is why this one was re-run rather than reasoned |
| 13 | `2026-09-03-bang-forces-the-builtin.md` | a macro beats the builtin; `!` forces the builtin; `#1200` on `!mym` | DND | Known-diff: accepted shapes and verdicts. The one note here naming no path at all |
| 14 | `2026-09-03-tilde-tilde-logical-not.md` | `~~` is logical NOT, atom tier | DND | Measured on both x86-64 builds by the note ("agree on every byte column AND every error line"); probes and runners committed, runners print md5 |
| 15 | `2026-09-03-tilde-tilde-probes/README.md` | how to run; the two builds agree | DND | Mentions the banner to say it identifies nothing; `run.sh` is `ref` (guarded) by default |
| 16 | `2026-09-04-as-enum-probes/README.md` | 20-row enum model; row 13 "value folds to 0" | DND | Oracle `s1disasm/.../Linux-x86_64`, so `61e67256`; `run.sh` is guarded. Row 13 is a K1-shaped value in an errored run, so it was re-run: identical on all four, **not build-dependent**. See "Found on the way" for what it IS |
| 17 | `2026-09-04-as-label-on-if-line.md` | a block-head label binds the line's PC; closers bind when their arm ran | DND | Known-diff: `0dee1f98` by path (the quoted `(x86_64-Linux)` agrees), accepted shapes and verdicts |
| 18 | `2026-09-04-as-macro-default-params.md` | default substituted when no text is supplied; missing operand reads as `0` | DND | Re-run `default_params.asm` and `empty_operand.asm`: identical on all four, and the zero is not a carry-over |
| 19 | `2026-09-04-as-silent-acceptance-probes.md` | keyword/positional rules; `09 02`; `#1820` takes THEN | DND | Oracle `61e67256` by path; `w3` and `k3` re-run identical on all four; the `#1820` controls were already run on both builds by the note |
| 20 | `2026-09-05-asl-silent-decline-regime.md` | the silent-decline regime across all four builds | DND | **A false positive of the sweep**: it identifies every build by 8-hex digest prefix (`61e67256…`, `0dee1f98…`, `a8cd8b80…`, `aa6de52f…`), which the sweep's full-digest match cannot see. Its build-dependent claims (K1, K2) are attributed per digest |
| 21 | `2026-09-07-as-nesting-relex.md` | asl's column in a timing ladder (`0.007` s flat) | **DEPENDS** | Timing. **Settled in the record**: the column was taken "through `asl_ref.sh`'s `asl_run`", which refuses any digest but `61e67256` (literal pinned since `048ea0b8`, 2026-09-05; the note landed `8b3d7755`, 2026-09-07) |
| 22 | `2026-09-08-extra-traversal-measure.md` | asl timings and pass counts on s1disasm and s2disasm; "per traversal 1.24x / 1.13x" | **DEPENDS** | Timing, and a whole-`s2.asm` run that succeeds only on the fork (K4). **Settled from surviving artifacts**: the corpus copies in `/home/volence/sonic_hacks/.scratch/traversal/corpus/` hold `61e67256` (s1disasm) and `0dee1f98` (s2disasm) today, and that directory's `asl-s1-verbose.txt` / `asl-s2-verbose.txt` banners carry `(x86_64-unknown-linux)` and `(x86_64-Linux)` + flamewing respectively, the matching families |

## Summary

| verdict | count |
|---|---|
| DEPENDS | 3 |
| DOES-NOT-DEPEND | 19 |
| UNDETERMINED | 0 |

Of the 19: **9 rest on a re-run or a cross-build measurement** (6, 7, 9, 12, 14, 16, 18, 19, and 20,
which attributes per digest); **5 on a recovered identity, most also on md5-stamped committed
goldens** (1 to 5); **5 on K1 to K5 alone** (8, 11, 13, 17, and 15, which only mentions the banner):
accepted, fully valued, short-argument shapes that no known difference reaches. That last group is a
classification, not a measurement, and is labelled "Known-diff" in the table for that reason.

**The three DEPENDS rows, and what settles each:**

| note | depends on | what settles it | state |
|---|---|---|---|
| `2026-09-03-as-macrosetup-three-sites.md` | a whole-Sonic-2 run's silence at 34 lines, on `0dee1f98` | Nothing left to settle: the claim was refuted as a divergence at `2026-09-04-as-end-probes/README.md` (`wrange.asm`, `wimm.asm`, run on both builds, identical verdicts) and its row closed | superseded |
| `2026-09-07-as-nesting-relex.md` | asl wall-clock time | The binary is fixed by the guard it ran through: `asl_run` accepts only `61e672562465725a8c102288a7da9098`. A re-run would re-time, not re-identify | settled |
| `2026-09-08-extra-traversal-measure.md` | asl wall-clock time; asl succeeding on whole `s2.asm` | Identified today from its own scratch: corpus copies md5 `61e672562465725a8c102288a7da9098` (s1) and `0dee1f98e6480a4783d27ffd8b90896f` (s2), banners agreeing. That directory is untracked and outside the repo, so this note is now the durable record of it. A stronger settle, if ever wanted, is re-running `.scratch/traversal/timeit.py` against those copies with the md5 printed; the ratios would be re-timed, not re-identified | settled |

No note was edited: they are historical records. This note is where the identities now live.

## Found on the way, not acted on

* **Enum row 13's "value folds to 0" is not a rule.** With one accepted byte above the refused
  `enum` line (`enum_fwd_stale.asm`), `dc.b a,b` lists `0101` instead of `0000`, identically on all
  four builds. The run exits 2 with an incomplete-pass footer, so this is the "a run carrying any
  error is not a source of values" face, not a build difference. The listed value moves with the
  source above it (here it tracks the PC of the `enum` line; one pair does not establish that it IS
  the PC).
* **`asl-reference/README.md`'s table gives `a8cd8b80`'s second banner line as
  `(x86_64-unknown-linux)`.** Running it prints `(i386-unknown-linux)`, as
  `2026-09-05-asl-silent-decline-regime.md` already records. Not edited: that file carries its own
  standing rule.
* **The sweep counts mentions and misses digest prefixes.** Notes 15 and 20 are in the population
  only because of its one spelling: 15 mentions the banner to say it identifies nothing, 20
  identifies every build by an 8-hex prefix. Neither is a defect the sweep exists to find. Changing
  the sweep would move a figure other records quote, so it is noted rather than changed.

## Reproduce

```sh
./scripts/sweep_asl_citation_form.sh
cd docs/superpowers/notes/2026-09-26-asl-banner-citation-dependence-probes
./all4.sh ../2026-09-04-as-end-probes/wrange.asm     # positive control: VARIES on two builds
./all4.sh default_params.asm                         # any probe here, or any committed one
md5sum /home/volence/sonic_hacks/{s1disasm,s2disasm}/build_tools/Linux-x86{,_64}/asl
git -C /home/volence/sonic_hacks/aeon show 5e1b2b9d^:tools/asl | md5sum
```

`all4.sh` writes under `.work/` beside itself, never `/tmp`, and nothing under `.work/` is committed.
