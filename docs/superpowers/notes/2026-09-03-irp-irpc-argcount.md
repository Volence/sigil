# `irp`, `irpc` and `ARGCOUNT` — the three constructs that share one macro

Sonic 1's largest single site and Sonic 2's third largest. `irpc` alone was 615
diagnostics on ONE line (`Macros.asm(317)`), because the macro it sits in is
invoked 615 times.

## Provenance

| | |
|---|---|
| oracle | `s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098` |
| flags | `-xx -n -q -A -L -U -E -i .` — the corpus's own, from `build_tools/lua/common.lua:773` |
| corpora | `s1disasm` `f6ece657` (entry `sonic.asm`), `s2disasm` `e45ebf3` (entry `s2.asm`), both in detached worktrees |
| probes | committed beside this note in `2026-09-03-irp-irpc-probes/` — `p1`…`p9`, `pe/pf/pg/pj/pk/pn/pu/pv`, plus the harnesses |

**S1 and S2 ship different `asl` builds behind one version string.** S1's is
upstream AS; S2's is the flamewing fork (md5 `0dee1f98…`). Every rule here was
measured against **S1's**.

## The semantics

### `irp NAME,<items>` and `irpc NAME,<string>`

Both run their body once per item, substituting `NAME` as text. Both close on
**`endm` OR `endr`**. They differ only in where the item list comes from.

`irp`'s items are the operand's top-level comma groups as **raw source text**,
never evaluated — a comma inside a quoted item does not split it, and whitespace
around an item is dropped:

```text
  16/    100F : 5B31 2B32 5D     dc.b "[1+2]"
  16/    1014 : 5B24 4646 5D     dc.b "[$FF]"
```

`irpc`'s operand is a **string expression, evaluated once**, and the items are
its characters — spaces included. A `set` symbol resolves, escapes decode
(`"A\x5AB"` is three characters `A`, `Z`, `B`), and an integer result renders in
decimal and is then walked digit by digit (`irpc c,65` is `6` then `5`). An
operand that resolves to nothing is asl's error #1010 and runs **zero**
iterations.

**An EMPTY list is ONE EMPTY iteration, not none** — both spellings:

```text
   7/    1002 : 11          dc.b $11     ← irp v,     (one iteration)
   7/    1002 : 3C3E        dc.b "<>"    ← irpc c,""  (one iteration)
```

This is not a curiosity. It is why `s2.macrosetup.asm(301)` guards its
`irp op,ALLARGS` with `if ARGCOUNT>0` — without the guard an argument-less
`jmpTos` would run the loop once over an empty item and execute `label *` with no
name. And it is what S1's most common demo line depends on: `demoinput ,	$8C`
makes `irpc btn,"buttons"` into `irpc btn,""`, which runs the `switch` once
against an empty character and matches no `case`.

A head with **no comma at all** (`irp v`) is a different thing: asl's error
#1110, body skipped.

Substitution is textual, **case-sensitive** under `-U`, and obeys the same
boundary rule macro parameters obey — `"c"` and `_c_` take the value, `xcx` does
not:

```text
  38/    1016 : 4141 7863 785F 415F   dc.b "A", 'A', "xcx", "_A_"
  33/    102A : 3C41 3E3C 6376 3E     dc.b "<A><cv>"        ← irpc Cv,"AB"
```

A loop nested in a macro is substituted **once** where it is entered and then
replayed — exactly as `rept`/`while` are, so a `shift` in the body advances the
frame without changing the body's own text, and the frame has advanced by the
line after the loop:

```text
  35/    1021 : 7031 01     dc.b "p1",1
  35/    1024 : 7031 02     dc.b "p1",2
  35/    1027 : 7031 03     dc.b "p1",3
```

### `ARGCOUNT`

**It is a substitution, not a symbol.** The macro listing shows its digits pasted
into the body line, the way it shows `ALLARGS`'s text. It folds case, obeys the
boundary rule, substitutes inside string literals — and **yields to a parameter
declared with that name**:

```text
   9/    1002 : 315B 325D 2032 ...    dc.b "1[2] 2[xARGCOUNTx] 3[_2_] 4[2] 5[2]"
  15/    1027 : 5B7A 7A5D             dc.b "[zz]"     ← ac3 macro ARGCOUNT / ac3 zz
```

**The owed probe: `ARGCOUNT` under `shift` fits neither vector, and here is the
rule.** Before any shift it is the number of argument groups the call *wrote*.
From the first shift on it answers from the **parameter list**, not the argument
list: `params − shifts`, which can go negative. The decrement stops once
`max(params, args)` shifts have happened, because a shift past the end of the
argument store is a no-op. Sixteen rows over the grid (probes `p3.asm`, `p4.asm`),
five `dc.w ARGCOUNT` each — the two that show the shape:

```text
  one   macro pp       / one 11,22,33               3, 0, -1, -2, -2
  three macro q1,q2,q3 / three 11,22,33,44,55       5, 2,  1,  0, -1
  m2    macro q1,q2    / m2 10,11,12,13,14,15       6, 1,  0, -1, -2
```

The first two, as asl lists them (`p3.asm`, lines 27 and 37):

```text
  27/    1002 : 0003    dc.w 3      37/    103E : 0005    dc.w 5
  27/    1004 : 0000    dc.w 0      37/    1040 : 0002    dc.w 2
  27/    1006 : FFFF    dc.w -1     37/    1042 : 0001    dc.w 1
  27/    1008 : FFFE    dc.w -2     37/    1044 : 0000    dc.w 0
  27/    100A : FFFE    dc.w -2     37/    1046 : FFFF    dc.w -1
```

So the brief's "3 → 0 across one shift of a one-parameter macro called with three
arguments" is `params(1) − shifts(1) = 0`. It is not a count of anything
remaining; it stops being about the arguments the moment a shift happens.

An **empty operand field is 0**, not one empty group — `ac` is 0 while `ac ,` is
2 and `ac 1,,3` is 3. That distinction is load-bearing for the same
`jmpTosInternal2` guard.

**The corpus needs only the unshifted half.** The one `ARGCOUNT` site in either
corpus is `s2.macrosetup.asm(301)`, inside `jmpTosInternal2`, which declares no
parameters and performs no `shift`; the enclosing `jmpTosInternal` shifts in its
own frame and relays `ALLARGS`. The shifted half is implemented anyway because
the rule is measured, and a guess left in its place is what rots.

## Both corpora, before and after

Every class, summing with no remainder, and the unresolved-symbol NAME sets
compared in both directions.

### Sonic 1 — 1,367 → 887

| before | after | delta | class |
|---:|---:|---:|---|
| 728 | 96 | **−632** | `X` is not a recognized 68000 mnemonic |
| 14 | 166 | **+152** | bad word expression |
| 497 | 497 | 0 | unresolved symbol in operand |
| 36 | 36 | 0 | bad operand expression |
| 25 | 25 | 0 | unresolved long expression |
| 18 | 18 | 0 | unexpected character |
| 18 | 18 | 0 | instruction needs an explicit size suffix |
| 8 | 8 | 0 | unresolved rept count |
| 6 | 6 | 0 | case needs a string literal |
| 6 | 6 | 0 | bad immediate expression |
| 4 | 4 | 0 | trailing tokens in operand |
| 2 | 2 | 0 | unsupported form: ccr is not a general EA |
| 2 | 2 | 0 | switch needs a string expression |
| 1 | 1 | 0 | the corpus's own `error` self-check |
| 1 | 1 | 0 | unknown directive or mnemonic |
| 1 | 1 | 0 | org target precedes the current phase base |
| **1367** | **887** | **−480** | |

**The −632** is exactly the predicted family: 615 (`Macros.asm(317)` `irpc`) + 14
(`s1.sounddriver.asm` 1795/2051 `irp`) + 3 stranded `endm` heads. The
`switch`/`case` cascade the S1 baseline note attributed to `demoinput` did NOT
move — those 6+2 belong to `sound/_smps2asm_inc.asm(63)`'s integer `switch`, a
different defect. **That is a correction to the recorded baseline.**

**The +152 is the loops working.** It lands entirely on two lines:
`s1.sounddriver.asm(1796)` 8 → 96 and `(2052)` 6 → 70. Both bodies call a float
`function` sigil cannot fold; before, the `irp` never ran and the body errored
once per macro invocation, now it errors once per item. The counts are exact and
independently derivable from the source: `MakeFMFrequenciesOctave` is called 8
times over a 12-item list (**96**), and `MakePSGFrequencies` is called 6 times
over lists of 12,12,12,12,12,10 (**70**). Same defect, more iterations of it —
which is also an independent witness that `irp` iterated the right number of
times.

Unresolved-symbol names: 86 distinct before, 86 after, **zero in either
direction**.

### Sonic 2 — 9,625 → 9,317

| before | after | delta | class |
|---:|---:|---:|---|
| 593 | 219 | **−374** | `X` is not a recognized 68000 mnemonic |
| 10 | 1 | **−9** | unknown directive or mnemonic |
| 56 | 131 | **+75** | bad word expression |
| 3745 | 3745 | 0 | unresolved symbol in operand |
| 2622 | 2622 | 0 | bad operand expression |
| 2307 | 2307 | 0 | expected mnemonic, directive, or label |
| 114 | 114 | 0 | absolute address needs an explicit width suffix |
| 58 | 58 | 0 | instruction needs an explicit size suffix |
| 30 | 30 | 0 | bad byte expression |
| 23 | 23 | 0 | `int()`: could not evaluate float expression |
| 11 | 11 | 0 | unexpected character |
| 41 | 41 | 0 | cannot include (the gitignored generated sound data) |
| 6 | 6 | 0 | case needs a string literal |
| 3 | 3 | 0 | malformed number |
| 3 | 3 | 0 | bad displacement expression |
| 2 | 2 | 0 | trailing tokens in operand |
| 2 | 2 | 0 | switch needs a string expression |
| 1 | 1 | 0 | unsupported form: `sbc hl,bc` |
| **9625** | **9317** | **−308** | |

Removals by site: 272 (`s2.macros.asm(289)`), 83 (`s2.asm(14493)`), 14
(`s2.asm(10284)`), 6 (`s2.sounddriver.asm(1055)`), 1 each at `s2.asm(4682)`,
`(4688)`, `(10296)`, `(14512)`, `s2.macros.asm(311)`,
`s2.sounddriver.asm(1058)`, `(1387)`, `(1390)`. The −9 `unknown directive` are
the `irp` heads reached under `CPU Z80`, where an unrecognized head reports under
that spelling instead.

Rises: `s2.sounddriver.asm(1056)` 6 → 70 and `(1388)` 1 → 12 — the same float
`function` gap as S1's, now once per item. Unresolved-symbol names: 291 distinct
before, 291 after, **zero in either direction**.

The 41 `cannot include` rows are the gitignored generated sound data, absent from
the live `s2disasm` too (it has never been built). Constant across the pair.

### No new diagnostic SITE, in either corpus

A class table can hide a rise as a count on a line that was already red, so the
`file(line)` sets were compared too, both directions:

| | sites before | sites after | now clean | **NEW** |
|---|---:|---:|---:|---:|
| S1 | 615 | 609 | 6 | **0** |
| S2 | 8600 | 8588 | 12 | **0** |

The six S1 sites are the three loop heads and the three `endm` lines stranded by
them; the twelve S2 sites are the same shape. Every count that rose rose on a line
that was already diagnosing.

## The silent half — what was done to look

A loop that expands wrongly emits **wrong bytes rather than a complaint**, and
`demoinput` emits demo input data, so its bytes fail silently and late. Counting
diagnostics cannot see any of that. Four things were done instead.

**1. A 13-file byte sweep against `asl`, and it is proven able to fail.** Each S1
demo script was wrapped in the corpus's own `demoinput` macro and its own button
constants, assembled by both tools, and the images compared by CRC32+size
(`2026-09-03-irp-irpc-probes/mkharness.sh`, `diff_bytes.sh`). **13 of 13 identical** — the whole
`irpc` → `switch`/`case` → `dc.b` path across all 615 invocations.

The sweep was then run under two mutations, each applied to a committed baseline:
`irpc` dropping its last character → **13 DIFFER**; the loop-variable boundary
rule removed → **13 DIFFER**; restored → 13 SAME.

**2. A GREEN mutation, chased rather than banked.** Making an empty `irpc` string
produce ZERO iterations instead of one left all 13 **SAME**. That is real
uncovered ground and it is worth stating precisely: `demoinput`'s empty case
falls through to an empty `elsecase` either way, so the corpus cannot tell the
two apart. **The empty-iteration rule is pinned by nothing in either corpus** —
only by the asl probe and the unit test derived from it. The corpus authors knew:
the `if ARGCOUNT>0` guard exists because the rule bites where a body emits
unconditionally.

**3. The probe corpus itself was byte-compared, not exit-compared.** `p1.asm`
(the `irp`/`irpc` basics) is byte-identical to asl. That comparison is what
caught the first real defect in this parcel's own code — see below.

**4. The reverse direction was checked.** An early run of the demo harness had
sigil at **exit 0 with plausible bytes** while asl refused; the harness, not
sigil, was wrong (missing constants). Reading only sigil's exit code would have
recorded a pass. Both tools' exit codes are in every row of the sweep for that
reason.

### What the hunt found

**a. In this parcel's own code, caught by bytes and not by any count.**
`irp`'s items were first rendered from tokens, which prints `$FF` as `255` — the
loop expanding to the wrong text, silently, with the diagnostic count unchanged.
Fixed by slicing the items back out of the head's source text through the token
spans. Pinned by `irp_items_are_raw_text_while_irpc_evaluates_its_operand`.

**b. `~~` is asl's LOGICAL NOT and sigil computes double bitwise NOT.**
Not this row, but found by it — the S2 `jmpTos` byte harness (`pj.asm`) emitted
**nothing at all**, silently, exit 0, because `if ~~removeJmpTos` folded false.
Probe `pn.asm`:

| source | asl | sigil |
|---|---|---|
| `dc.b ~~0,~~1,~~5` | `01 00 00` | `00 01 05` |
| `dc.b ~0&$FF,~1&$FF` | `FF FE` | `FF FE` (agree) |

**96 sites in `s2disasm`, 0 in `s1disasm`, 0 in aeon.** It gates
`jmpTosInternal`, i.e. every one of S2's 30-plus `jmpTos` call sites. Booked, not
fixed: it is an operator change with its own red-first gate and its own aeon
sweep to earn. Its falsifier is one command — `2026-09-03-irp-irpc-probes/pn.asm`.

**c. `NAME := <expression containing an undefined symbol>` binds 0 silently.**
Probe `pv.asm` (committed): asl reports #1010 and refuses; sigil exits 0 and emits `AA 00 BB`.
Reachable in principle through `demoinput`'s own `btns_mask := btns_mask|btnX`,
though not in the real corpus where the constants are defined. Booked.

**d. A recorded, asl-verified project rule is WRONG, and it is one this parcel's
machinery sits on top of.** `MacroFrame`'s doc says a `shift` empty-fills the
vacated parameter slot, citing `<2|3||2,3,4>` → `<3|||3,4>`. asl does not
empty-fill: it leaves AS's internal placeholder, which renders as the two control
bytes `\001\00N`. Probe `pg.asm`, `t3 macro pp,qq,rr` over `dc.b "n<pp|qq|rr>"`:

```text
asl    0<a1|a2|a3>  1<a2|a3|\001\004>  2<a3|\001\003|\001\004>  3<\001\002|\001\003|\001\004>
sigil  0<a1|a2|a3>  1<a2|a3|>          2<a3||>                  3<||>
```

A slot never SUPPLIED does render empty (`t3 a1` → `0<a1||>`), which is what the
recursion guards in `zoneTableEntry` and `creditsPtrs` depend on and what sigil
gets right. Only a slot a shift VACATED differs. The earlier note read the
placeholder bytes off a listing where they are unprintable and transcribed them
as nothing.

**Corpus-unreachable in both**, because all four `shift` sites read their
parameters before shifting and then recurse into a fresh frame. Not fixed —
matching it means emitting control bytes, and it looks like an AS bug rather than
a feature. Booked with the probe so the next reader measures rather than inherits.

## Verification

- 10 new `#[test]` rows in `crates/sigil-frontend-as/src/eval.rs`, every
  expectation a byte column read off an asl listing.
- **Five red-first mutations**, each applied to a committed baseline with the
  patch read back from disk, each **actually red**, each restored
  (`2026-09-03-irp-irpc-probes/mutate.sh`): empty list → zero iterations; `ARGCOUNT` counting
  arguments down instead of parameters; `ARGCOUNT` tried before the parameters;
  `irp` items rendered from tokens; loop variable folded case-insensitively.
- Two further mutations against the 13-file byte sweep, plus the GREEN one above.
- Aeon: all four shapes deleted and rebuilt one per invocation under
  `SIGIL_VERSION_STRICT=1`. **Byte-identical**, CRC32+size:
  `s4 14ee2440/719700`, `s4.debug 142294b3/737683`, `demo 0c456778/96474`,
  `demo.debug 2e603d53/101339`.
- `cargo clippy --release --workspace --all-targets -- -D warnings` — exit 0.
- Corpus figures re-derived with the final committed binary and identical to the
  measurement above, line for line.

### The suite baseline, derived rather than taken

Master `e91f649f`: **378 suites / 4,329 passed / 0 failed / 2 ignored**, CARGO_EXIT
0 — measured in a detached worktree of that SHA, matching the figure the lane was
given.

**It did not match on the first attempt, and the reason is worth recording.** Run
CONCURRENTLY with this branch's landing run, master returned 4,328 passed and one
red: `deny_todo_promotes_to_error`, which spawns `sigil emp … --deny-todo` and
saw the build succeed. Alone it is green 3 times out of 3, and the full suite
alone reconciles at 4,329/0. This branch's run was green for that test even under
the same concurrency.

**The mechanism is NOT established here and should not be banked from this note as
"a flake".** What was measured is: one failure under two simultaneous full-suite
runs, and zero failures in four subsequent isolated runs of the same tree. The
falsifier is one command — run the full suite twice concurrently on master.

## Booked, not done

- `~~` logical NOT (finding b) — the largest of these, 96 S2 sites, silent.
- `NAME :=` binding an unresolved expression to 0 silently (finding c).
- `shift`'s vacated-slot placeholder (finding d) — corpus-unreachable; the
  `MacroFrame` doc comment still carries the old reading.
- `eval_str` has no `+` concatenation, so `irpc c,sa+"OP"` is refused where asl
  accepts it (probe `p7.asm` case 7e). A general `eval_str` gap, not an `irpc`
  one; unreachable in both corpora.
- `ARGCOUNT` outside a macro: **asl reports an unresolved symbol, and so does
  sigil. There is no divergence here.**

  **CORRECTED 2026-09-05.** This row used to read "asl resolves it to something
  (13 in `p2.asm`), sigil reports an unresolved symbol", and recorded a
  behavioural gap that does not exist. `p2.asm` is a five-shape file whose
  `three` macro raises six `#1107 undefined attribute` errors well above the
  `dc.b $BB,ARGCOUNT` line, and those errors stopped asl's pass loop, so the
  pass that judges an unresolved symbol never ran. The `BB0D` in that listing is
  a pass-1 artifact, not the 13 asl computed. The same construct alone in a file

  ```text
  cpu 68000 / padding off / org $1000 / dc.b $BB,ARGCOUNT / end
  ```

  assembles in **2 passes** with a complete diagnostic set and says
  `error #1010: symbol undefined  ARGCOUNT`, emitting no byte for the line. The
  two front ends agree. See `2026-09-05-asl-pass-loop-swallows-diagnostics.md`.
