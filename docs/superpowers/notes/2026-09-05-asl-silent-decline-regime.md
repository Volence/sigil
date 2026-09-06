# The regime asl declines in silence, and where the right answer comes from

2026-09-05 · branch `parcel/asl-silent-wrong-measurement` · sigil master base `e0109109`

The measurement half of **ASL-SILENT-WRONG-ON-BOTH-BUILDS**. It changes no
behaviour and recommends one fix; the ruling is not this note's to make.

Probes, runners and the shapes table: `2026-09-05-asl-silent-decline-regime-probes/`.
Every number below is produced by a committed script, named at the number.

```sh
cd docs/superpowers/notes/2026-09-05-asl-silent-decline-regime-probes
./sigil_today.sh <path-to-sigil> 4 | ./classify.py    # the regime table
./both.sh 4                                           # every runnable asl build
./corpus.sh                                           # the population
./run.sh r11_earlier_error_swallows_undef.asm         # the instrument finding
```

Ground truth is `asl -xx -n -q -A -L -U -i .`, the Sonic 2 build's own flags
minus the two that only redirect output.

---

## THE INSTRUMENT, and a correction to the count

`../asl-reference/README.md` says four `asl` binaries in this workspace print
`Macro Assembler 1.42 Beta [Bld 212]` verbatim. **Six of them run on this
machine, under FOUR distinct digests**, and the directory name misleads as badly
as the banner does:

| path | md5 | second banner line | actual ELF |
|---|---|---|---|
| `s1disasm/build_tools/Linux-x86_64/asl` | `61e67256…` | `(x86_64-unknown-linux)` | x86-64 |
| `skdisasm/build_tools/Linux-x86_64/asl` | `61e67256…` | same binary | x86-64 |
| `sonic_hack/tools/as/asl` | `61e67256…` | same binary | x86-64 |
| `s2disasm/build_tools/Linux-x86_64/asl` | `0dee1f98…` | `(x86_64-Linux)` | x86-64 |
| `s1disasm/build_tools/Linux-x86/asl` | `a8cd8b80…` | `(i386-unknown-linux)` | i386 |
| `skdisasm/build_tools/Linux-x86/asl` | `a8cd8b80…` | same binary | i386 |
| **`s2disasm/build_tools/Linux-x86/asl`** | **`aa6de52f…`** | **`(x86_64-Linux)`** | **x86-64** |

That last row is the one worth staring at. It sits in the directory named for
the 32-bit platform, is a 64-bit binary, announces itself `(x86_64-Linux)`, and
has a **different digest** from the `Linux-x86_64` build beside it. Neither the
banner, nor the second banner line, nor the path is the identity. Only the
digest is. `both.sh` walks all four digests and prints each one's banner beside
its md5 so the collision is visible rather than asserted.

They split 2–2 on what they substitute for an operand they declined to value:
`61e67256` and `a8cd8b80` carry the last value they computed (stable, and
therefore the dangerous one); `0dee1f98` and `aa6de52f` read uninitialized
memory (different every run). **A stable value is still not an answer** — both
halves of that rule are now measured on two builds each rather than one.

Two of them — `0dee1f98` and `aa6de52f` — **abort** on `dc.w a1`:

```text
asl: /home/runner/work/asl-releases/asl-releases/motpseudo.c:969:
     DecodeMotoDC: Assertion `0' failed.
Aborted (core dumped)                                       exit 134
```

`r10_dcw_reg_abort.asm` is that line and nothing else.

---

## AN INSTRUMENT FINDING THAT OUTRANKS THE ROW

**An error found earlier in a file suppresses every later `symbol undefined`
report.** asl says so itself, in the listing footer:

> **WIDENED 2026-09-05: POSITION IS IRRELEVANT.** "Earlier" is a narrower rule
> than the one that holds. An error placed *below* the undefined symbols
> suppresses them just the same, because what stops is the pass LOOP and not the
> reading of the file. Measured on
> `2026-09-05-asl-pass-loop-probes/error_first.asm` and `error_last.asm`, which
> carry the same three undefined symbols with one unrelated error above and
> below them respectively: both report **zero** of the three, against three from
> the same file with no other error. Everything this section concludes stands;
> only its scope grows. `2026-09-05-asl-pass-loop-swallows-diagnostics.md`
> carries the sweep this section booked.

```text
      1 pass
        Additional necessary passes not started due to
        errors, listing possibly incorrect.
```

A forward reference is legal, so an undefined symbol is a provisional value in
the first pass and is only reported when a later pass finds it still undefined.
An earlier error stops the pass loop, and the provisional value is then emitted
with no diagnostic at all.

Measured on a matched pair that differs in exactly one line —
`r11_earlier_error_swallows_undef.asm` carries a loud `move.w #1.5,d0` above two
undefined symbols; `r11b_no_earlier_error.asm` carries an accepted
`move.w #$B0B0,d0` in its place and is otherwise identical:

| file | earlier error | `#zz` | `dc.w zz` | passes |
|---|---|---|---|---|
| `r11` | `#1133` float | **silent** | **silent** | 1, "not started" |
| `r11b` | none | `#1010 symbol undefined` | `#1010 symbol undefined` | 2 |

**This is how a multi-shape probe file lies.** It was found because
`r08_other_arg_kinds.asm` reported nothing for its two undefined-symbol lines
while `r09_strict_or_lazy.asm` reported both — the two files differ only in
whether anything above them had already failed.

Two consequences, and the second is the one that generalises:

- Every measurement in this note comes from `sigil_today.sh`, which puts **one
  shape per file**. That is the structural defence, and `classify.py` verifies
  it held: **0 of 40** shape files stopped their pass loop early. The
  multi-shape `r*.asm` probes beside them are illustrative, not the authority,
  and **2 of the 12 do trip it** — `r11`, which exists to, and `r08`, which
  fell into it and is how this was found. `both.sh` names both, on all four
  builds.
- `run.sh`, `both.sh` and `sigil_today.sh` now grep every listing for that footer
  line and print `INCOMPLETE` when it appears. It fires on `r11` (2 lines) and
  not on `r11b` (0) — a red/green pair from committed inputs, no mutation
  needed. **Any probe corpus in this tree that puts several deliberate errors in
  one file has this defect and does not know it.** Not swept here; booked.

---

## THE DERIVED HANDLE AGREES, AND ITS POPULATION MOVED

`../2026-09-05-asl-nondeterminism-sweep-probes/sweep_probes.sh` enumerates the
declined-operand set by running the varying build over every `*-probes`
directory and reporting what changes between runs. It was not guessed at here,
it was run:

```sh
ASLDIR=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64 ./sweep_probes.sh 3
```

220 probes swept, **30 UNSTABLE**, up from the 22 that directory's README
records. This parcel's probe directory matches `*-probes`, so it joined the
corpus without being told to, and **8 of its 12 files came back UNSTABLE**:
`r02`, `r03`, `r04`, `r05`, `r08`, `r09`, `r11`, `r11b`. That is an independent
classification of the same shapes by a runner written before this parcel, and it
agrees — every probe here that carries a declined operand is in its set.

**The four absentees are the four that should be.** `r07_z80.asm` carries no
declined operand, because z80 is outside the regime. `r01`, `r06` and `r10` all
contain `dc.w a1` and so ABORT the varying build, identically on every run — a
crash is perfectly stable, and the sweep quite correctly files them under "not
seen to vary". One more reason a stable stream is not an answer. (The two
`SIGSEGV` rows in that run's totals are pre-existing, in
`2026-09-04-as-warning-exitm-probes`, and are exit 139, not this abort.)

The sweep's own positive control fires on this run, so a zero from it would have
been a zero it could have failed to report.

---

## THE REGIME

33 shapes plus 7 controls, one shape per file, both assemblers, four runs of the
varying build each. `shapes.tsv` is the population; `classify.py` reads the run.

**asl is SILENT — exit 0, no diagnostic, wrong output — on 16 of 33.**
**asl is LOUD on 17 of 33.** Every one of the 7 controls answers on both
assemblers, which is what makes the table a measurement rather than a mood.

The boundary is sharper than "a function call with a register argument", and it
is one sentence:

> **asl declines in silence exactly when a value it cannot print is produced by
> the substitution step of a user `function` call, or is the whole operand of a
> `dc.<size>`. Everything one step away is loud.**

| SILENT (16) | LOUD (17), and what asl says |
|---|---|
| `#fu(a1)` body uses `p` | `#fu(pc)`, `#fu(sr)`, `#fu(ccr)`, `#fu(usp)` — `#1010 symbol undefined` |
| `#fi(a1)` body ignores `p` | `#fu(a1.w)` — `#1010`, the size suffix takes it out of the register set |
| `#fu(d3)`, `#fu(sp)`, `#fu(a7)` | `#fu(1+a1)` — `#1145 …but got register` |
| `#fu(A1)` — uppercase, under `-U` | `#a1` — `#1146 expected integer or string` |
| `#fu(g(a1))`, `#g(fu(a1))` — either nesting | `#1+a1` — `#1145` |
| `#f2(a1,5)`, `#f2(5,a1)` — either position | `dc.w a1+0` — `#1145` |
| `#fu(a1)+fu(5)` — one bad call poisons the expression | `eq = fu(a1)` then `dc.w eq` — `#1010` at the USE, blaming `eq` |
| `dc.w`/`dc.b`/`dc.l fu(a1)` — **no bytes emitted at all** | `move.w #$1234,fu(a1)` and `1+fu(a1)` — `#1010`, blaming `fu` |
| `dc.w a1` — **no bytes**, and the abort on two builds | `move.w fu(a1),d0` — `#1010`, blaming `fu` |
| `rs = a1` then `dc.w rs` — a register ALIAS, then no bytes | `#fu('ab')` `#1320`, `#fu(1.5)` `#1133`, `#fu()` / `#fu(5,6)` `#1490`, `#nofn(a1)` `#1860` |

Four boundaries worth naming separately, because each one refutes a plausible
description of the regime:

1. **Which registers.** `a0`–`a7`, `d0`–`d7` and `sp` are silent; `pc`, `sr`,
   `ccr`, `usp` and any `a1.w` are `#1010 symbol undefined`. The silent set is
   exactly the names asl's *expression* parser resolves to a register value.
   Case does not narrow it: `A1` is silent too, under `-U`.
2. **Whether the body uses its parameter is irrelevant** to asl. `fu` and `fi`
   behave identically. asl evaluates the argument before it looks at the body.
3. **The argument must be exactly a register token.** `fu(1+a1)` is LOUD. One
   `+` moves the shape out of the silent regime and into a named diagnostic.
4. **It is 68000-only.** `r07_z80.asm` puts the same shape to `cpu z80`:
   `ld bc,fu(hl)` and `dw hl` are both `#1010 symbol undefined`. Z80 register
   names are not expression-level register symbols, so they never reach the
   silent path.

And one that is not a boundary: a name that is **also** an equate does not
change anything. `r06_shadowed_name.asm` defines `a1 = $77` — asl accepts the
definition, lists it, and still reads `a1` as the register everywhere it
matters.

---

## WHAT THE RIGHT ANSWER IS

**It is to refuse, naming the register.** That is not a preference; five
independent sources outside the two binaries say it, and none says otherwise.

**1. AS's manual says `FUNCTION` is strict and its substitution is textual.**
`doc_EN/as.tex`, section `FUNCTION`:

> When the function is called, all parameters are calculated once and are then
> inserted into the function's formula. […] The result's type may depend on the
> type of the input arguments **as the arguments are textually inserted into the
> function's formula**. For example […] may have an integer, a float, or even a
> string as result, depending on the argument's type!

Three types are named. A register is not one of them, and has no textual form to
insert.

**2. AS's own message catalogue has the diagnostic.** `as.msg` beside the
binary — a separate data file, not the executable — carries

> `expected integer, floating point number or string but got register`

and asl fires it (`#1145`) for `#1+a1` and `#fu(1+a1)`, which is the same
situation one syntactic layer away.

**3. The shipped build's source shows the bail, and it is a missing `WrError`.**
`asmpars.c`, the user-`function` arm of `EvalStrExpression`, has four early
exits in one loop. Three report; the fourth does not:

```c
        EvalStrExpression(&FArg, &InVals[0]);
        if (InVals[0].Relocs)
        {
          WrStrErrorPos(ErrNum_NoRelocs, &FArg);        /* reports */
          FreeRelocs(&InVals[0].Relocs);
          LEAVE2;
        }
        …
        as_sdprintf(&stemp, "(");
        if (as_tempres_append_dynstr(&stemp, &InVals[0]))
          LEAVE2;                                       /* REPORTS NOTHING */
```

`as_tempres_append_dynstr` (`tempresult.c`) switches on `TempInt`, `TempFloat`
and `TempString` and `return -1` in its `default:` — so a `TempReg` argument
takes that branch. `LEAVE2` leaves `pErg` at the `as_tempres_set_none(pErg)` the
evaluator set on entry, and `TempNone` is the *"an error has already been
reported"* sentinel:

```c
    default:
      if (t.Typ != TempNone)
        WrStrErrorPos(DeduceExpectTypeErrMsgMask(TempInt | TempString, t.Typ), pComp);
```

So a register is downgraded to "already reported" and then not reported. **That
is a bug, not a semantics.** It also explains the whole silent set at once: any
argument whose value has no printable form takes it, which is why registers go
silent while strings and floats stay loud, and why the body's use of its
parameter is irrelevant.

**4. Upstream has already removed the mechanism.** Current
`Macroassembler-AS/asl-releases`, branch `upstream`, no longer substitutes
argument text at all — `as_tempres_append_dynstr` does not appear in `asmpars.c`
any more. Arguments are bound through a callback that `as_tempres_copy`s the
whole `TempResult`, registers included, into the body's evaluation; and
`DeduceExpectTypeErrMsgMask` has gained an explicit

```c
    case TempReg:
      switch (Mask)
      {
        case TempInt:                            return ErrNum_ExpectInt;
        …
        case TempInt | TempFloat | TempString:   return ErrNum_StringOrIntOrFloatButReg;
```

The behaviour measured here is a 2021-vintage build's defect that upstream's own
author has since designed out — in the direction of a named refusal.

**5. `dc.w <register>` carries an `assert(0)`.** `motpseudo.c`'s `DecodeMotoDC`
switches on `TempInt`/`TempFloat`/`TempString`/`TempNone` and asserts in the
default. Upstream marked the case unreachable. Two of the four shipped builds
have the assert enabled and abort; the other two have it compiled out and
silently emit nothing.

> **CAVEAT, stated rather than buried.** Sources 3–5 are read from
> `Clownacy/asl-releases` and `Macroassembler-AS/asl-releases` at their
> **present-day master/upstream**, fetched 2026-09-05. The shipped binaries are
> Bld 212, and both trees have moved since — `motpseudo.c`'s assert is at line
> 1071 today and the binary names line 969. The *mechanism* is corroborated by
> behaviour, not taken from the source alone: all four builds agree on the whole
> silent set, and the assert message the binary itself prints names the file and
> function. What the source adds is WHY, and the direction upstream chose. It is
> not a claim that these files compile to these binaries.

**None of the brief's three candidate readings is right as stated.** It is not a
syntax error — asl parses `#f(a1)` correctly as a call, and the `ctl_` rows show
the same parse answering when the argument is a value. It is not a mis-parsed
displacement — that story belongs to the *addressing-mode* position, which is
separately and correctly loud (`#1010`, blaming the function name), and the
`disp-or-call` parcel already closed it. The closest reading is the second, and
it needs sharpening: it is a **type error at argument evaluation**, which asl
detects and then fails to report.

---

## WHAT SIGIL DOES TODAY

Per shape, from `sigil_today.sh | classify.py`, sigil at `e0109109`.

**Sigil refuses 31 of the 33 shapes** — including 15 of the 16 asl is silent on,
where it is therefore already strictly better than every asl build. It accepts
exactly two:

| shape | asl | sigil | |
|---|---|---|---|
| `#fi(a1)` — body ignores `p`, argument a register | SILENT, `$A101` (a carry-over) | **accepts, emits `03 C7`** | neither refuses |
| `#fi(zz)` — body ignores `p`, `zz` UNDEFINED | **LOUD, `#1010 symbol undefined`** | **accepts, emits `03 C7`** | **sigil is the more permissive** |

**Both rows have one cause: sigil expands function arguments LAZILY.** When the
body never mentions its parameter, sigil folds the body without evaluating the
actual argument, so nothing about that argument is ever checked. The AS manual
states the opposite rule in as many words — *"all parameters are calculated
once"* — so this is a divergence from documented AS behaviour, not from a quirk.

The second row is the more serious of the two and is **not** in this parcel's
booked shape at all: it involves no register, asl refuses it loudly, and sigil
accepts it silently. It was found by `r09_strict_or_lazy.asm`, which exists only
because the manual's word "calculated" invited the question.

Three message-quality gaps, none of which changes an accept/refuse verdict:

- Sigil says ``unresolved symbol `a1` in operand`` where asl says *"expected
  integer, floating point number or string but got register"*. A register is not
  an unresolved symbol, and a reader who takes sigil's message at face value
  looks for a missing definition.
- `move.w #$1234,fu(a1)` and `1+fu(a1)` each emit the **same diagnostic twice**.
- `#nofn(a1)` with `nofn` undefined: asl says `#1860 unknown function`; sigil
  says `trailing tokens in #immediate`, which describes the parse and not the
  program.

Per `docs/OVERSEER.md` the AS frontend renders `file(line):` while `.emp` renders
`path:line:col:`; that split is a ruling and nothing above asks to change it.
Two sigil diagnostics here carry no location at all (`error: unresolved symbol
`a1` for fixup in section sec0 at offset 4`) because they are raised at link
time rather than parse time — noted, not booked.

---

## THE CORPUS POPULATION IS ZERO

`corpus.sh`, four parameters, each printing the command that produced it.
**2056 `.asm` files across four trees; 116 `function` definitions; not one site
in the regime.**

| tree | git | `.asm` | `function` defs | param-ignoring | `f(reg)` as a CALL | `dc.x reg` | `#name(reg)` |
|---|---|---|---|---|---|---|---|
| `s1disasm` | `f6ece65` | 455 | 25 | 0 | 0 | 0 | 0 |
| `s2disasm` | `e45ebf3` | 332 | 49 | 0 | **0** | 0 | 0 |
| `skdisasm` | `2fcd861` | 909 | 42 | 0 | 0 | 0 | 0 |
| `aeon` | `1f2aab07` | 360 | **0** | 0 | 0 | 0 | 0 |

The one number that needs its qualifier: s2disasm has **289** occurrences of
`id(aN)`, every one of them `id`, all in one file. **All 289 sit at the end of an
operand** — 244 end the operand outright, 45 are followed by a comma, which ends
an operand just as whitespace does, and **0** are followed by anything else.
asl peels a trailing `(An)` group as an addressing mode
*before* evaluating anything, so none of them is a call and none is in this
regime; they are the sites the `disp-or-call` parcel closed, and they are loud
when they are wrong.

```sh
# the whole discriminator, and it returns nothing:
grep -rEo 'id\(([aAdD][0-7]|[sS][pP])\)[^[:space:];,]' --include='*.asm' s2disasm
```

`param-ignoring: 0` is the population for the lazy-expansion defect too, and it
is the more interesting zero: sigil's two accepting rows both need a `function`
whose body never mentions its parameter, and no such definition exists in any of
the four trees. **The defect is unreachable from today's corpora.** It is not
unreachable from tomorrow's — nothing prevents such a definition, and sigil
accepting an undefined symbol is a class of failure worth closing on its own
terms rather than on its current count.

`corpus.sh` reports an unreadable directory as UNMEASURABLE. No zero in this
table is a directory that could not be read.

---

## RECOMMENDATION FOR THE FIX HALF — for ruling, not a decision

**Refuse. Do not match either asl build, and do not pin any byte of the silent
regime.** All five outside sources point the same way and the corpus cost of
refusing is zero.

Concretely, and in priority order:

1. **Make function-argument expansion STRICT** — evaluate every actual argument
   once, before substituting, whether or not the body mentions it. This is the
   AS manual's stated rule, it closes **both** of sigil's accepting rows with one
   change, and the second of them (`#fi(zz)`, an undefined symbol accepted) is
   the one that matters, because it is silent acceptance of a broken program
   with no register involved.
2. **Give a register in a value position its own diagnostic**, distinct from
   `unresolved symbol`. Sigil already refuses these 31 shapes; the gap is that it
   describes a register as a missing definition. asl's own wording — *"expected
   integer, floating point number or string but got register"* — is the model,
   and matching asl's *category* here costs nothing because asl's refusals are
   the loud rows, which agree on all four builds.
3. **Do not add a gate over an asl byte column for any shape in the silent
   table.** Sixteen of them have no answer on any build; the numbers in this
   note's tables are carry-over artifacts and are printed only to show that they
   are.

Deliberately **not** recommended: matching asl's silence anywhere, or matching
`dc.w a1`'s zero-byte emission. Two of four builds treat that as an assertion
failure.

---

## THE ROW IS MIS-BOOKED, and how

`ASL-SILENT-WRONG-ON-BOTH-BUILDS` names one construct. It is two, and only one
of them is about asl.

- **(a) asl's silent decline.** Real, wider than `#f(<register>)` (it takes
  `dc.<size>` too), narrower than "any declined operand" (a register one `+`
  deep is loud, and the whole thing is 68000-only). **Sigil already refuses all
  of it but one shape.** This half is nearly closed and the booking did not know
  it.
- **(b) sigil's lazy function-argument expansion.** Independent of registers,
  independent of asl's bug, and the only place sigil is *worse* than asl. It is
  what the fix half should actually be about.

They intersect at exactly one shape — `#fi(a1)` — which is why one row looked
like enough.

---

## WHAT IS LEFT OPEN

- ~~**The pass-loop suppression is unswept.**~~ **SWEPT 2026-09-05**, in
  `2026-09-05-asl-pass-loop-swallows-diagnostics.md`: 348 tracked `.asm`, 24
  carrying the warning, 18 more whose run died before writing a footer at all,
  two conclusions actually affected (`q8.asm` in `2026-09-03-as-struct-dots.md`
  and `p2.asm` in `2026-09-03-irp-irpc-argcount.md`, both corrected). The tell is
  no longer per-runner: it moved into `../asl-reference/asl_ref.sh` as
  `asl_diag_state`, and `asl_run` now writes `ASL_DIAG=<state>` beside
  `ASL_EXIT` for every caller. **Two things the sweep found that this row's
  recipe would have missed**: the grep has to be ANCHORED, because a listing
  echoes its source and a file whose comments discuss the warning matches an
  unanchored one; and absence of the warning is not completeness, because a
  fatal or a crash writes a listing with no footer at all.
- **`../asl-reference/README.md`'s binary table is short by two rows and by two
  digests**, and its "the path is context beside it" line is too kind to a path
  that actively lies. Not edited here: that file is the subject of its own
  standing rule and an edit to it wants its own review.
- **`../asl-reference/README.md` records the declined-operand population as "22
  of them as of 2026-09-05".** This parcel's probes joined that corpus and the
  number is now **30**. Not edited there, for the same reason as the row above.
- **The four builds' substitution behaviour is characterised only on this
  regime.** `a8cd8b80` and `aa6de52f` were measured on the shapes in this note,
  not on the 22-probe declined-operand set the nondeterminism sweep enumerates.
- **`#fu(zz)` under an earlier error** emits a masked provisional value
  (`303C 71C4` in `r08`). What that mask is derived from is not established here.
- **Sigil's duplicated diagnostic** for `#$1234,fu(a1)` — one line, two identical
  messages. Cosmetic, unbooked.

## WHAT IN THE BRIEF TURNED OUT WRONG

- *"Two asl builds here"* and the reference README's *"four binaries"*: there are
  **six runnable, four digests**, and one of them lives in a directory named for
  a platform it is not built for.
- *"We match neither behaviour today."* Sigil refuses 16 of the 17 shapes asl is
  loud on, and 15 of the 16 it is silent on — 31 of 33. The true mismatch is
  **two shapes**, both from one cause, and one of them is a shape the brief never
  names and where asl is loud, not silent.
- The three candidate readings for "what the right answer is" are all off: not a
  syntax error, not a mis-parsed displacement, and the "call should refuse"
  reading is right only once sharpened to *a type error at argument evaluation
  that asl detects and fails to report*.
- The brief's *"a `function` call in an immediate"* framing is too narrow in one
  direction (`dc.<size>` is in the regime, and the immediate is not required) and
  too wide in another (z80 is not in it at all).

Everything else in the brief reproduced: the carry-over mechanism on
`61e67256`, the uninitialized read on `0dee1f98`, the silence and exit 0 on both,
and `sweep_probes.sh` as a derived handle on the declined-operand population.
