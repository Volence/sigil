# AS-PERF-FIX-PASSES: adversarial review of stages A and B

Reviewed: `df96a977` (stage A `61f66464`, stage B `446519d2`) against base `911ebb9c`,
both built release in this worktree (`.target-new`, `.target-base`), and against the
reference asl md5 `61e67256` + p2bin md5 `4f2fff99` (copied from s1disasm's
`build_tools/Linux-x86_64`). Probes and the runner are in
`2026-09-28-passes-adversarial-review/` (`run.sh` expects the probe tree at `.probe/`
and the tools at `.probe/bin/`; `summary.txt` is the raw output of the last full run).
Pass counts for sigil are the number of `passN` phase lines; for asl, the listing's
`N pass`.

## Verdict

Stage B (converge on the read record) held on every probe: no source gave base and new
different bytes through a read-set convergence. Stage A (first-pass placeholders)
breaks: it follows asl's rule for a bare forward symbol but not for an expression, and
it changes which pass-0 layout sigil's persistent environment remembers. Three sources
where base agreed with asl and new does not, one of them not a multiple-fixpoint
source.

## Findings

### F1. REAL DEFECT (stage A): the placeholder is taken per expression, asl takes it per symbol

`fold_disp16` gives the WHOLE unknown displacement the location counter. asl gives each
unknown SYMBOL the location counter and then evaluates the expression. With `*` = 2,
`Fwd-2(a1)` is 0 under asl on pass 1 (collapses to `(An)`) and 2 under sigil (long).

Reproducer `a2_multifix_d16_offset.asm`:

```
	cpu 68000
	nop
	move.b d0,Fwd-2(a1)
After:	nop
Fwd	equ After-4
```

base `4e71 1280 4e71` (2 passes), new `4e71 1340 0002 4e71` (2 passes), asl
`4e71 1280 4e71` (2 passes). The source has two self-consistent layouts and the first
pass picks which one is kept, so it is contrived; the same split with the struct-offset
idiom `Fld-Base(a1)`, both forward (`a14`), gives the same result (asl `PC-PC` = 0).
On single-fixpoint sources the split shows only as passes: `a9` and `a15` take 3 passes
under asl (collapse, then grow) and 2 under new, bytes equal. So the note's "the size
then matches asl's first pass exactly" is true for a bare symbol only.

### F2. REAL DEFECT (stage A exposing a pre-existing semantic): a pass-0 name that asl never sees

Not a multiple-fixpoint source. `a16_stale_name_new_pass0.asm`:

```
	cpu 68000
	nop
	move.b d0,Fwd(a1)
After:
	if After=6
Stale = 1
	endif
	ifdef Stale
	dc.b $AA
	endif
	dc.b 0
Fwd	equ 0
```

base `4e71 1280 00` (2 passes), new `4e71 1280 aa 00` (2 passes, converged by reads),
asl `4e71 1280 00` (3 passes). New's pass 0 lays the move out long (placeholder = `*` =
2), so `After` = 6 and `Stale` is bound on pass 0 only. Sigil's environment keeps every
name any earlier pass bound (a pass's table starts as a copy of its seed and nothing
removes an entry), and sigil's `ifdef` answers from that environment, so every later
pass sees `Stale` defined. asl's first pass is also long here and also binds `Stale`,
but asl's `ifdef` is positional within the pass (probes `c1`, `c3`: `ifdef B` above
`B = 1` is false under asl on every pass), so the stale name is invisible to it.

Root cause is older than the change: `ifdef` reading the seeded (and stale) environment
differs from asl (`c1` asl `00`, sigil `bb00`; `c2` asl `01`, sigil `02`; `a12`, where
base and new are equally wrong). Stage A changes WHICH programs have a pass 0 that
differs from the final layout: base was wrong where a forward displacement is nonzero
(`a10`, `a11`: base emits `$AA`, new and asl do not), new is wrong where it is zero
(`a16`). The planted controls in `pass_count_tests` (`control_an_added_name_read_by_ifdef_...`,
`control_a_value_decided_by_an_added_name_...`) pin the non-asl `ifdef` answer: asl
gives `00` and `01` on those sources, 1 pass.

### F3. BEHAVIOUR DIFFERENCE (stage A): error where base and asl succeed

asl DOES keep a pass-1-only name for VALUE reads and macro calls (`a17`, `a18`: asl and
new succeed with the stale value, base errors). So when new's pass 0 and asl's pass 1
differ (F1's per-symbol split), a stale label exists under asl and not under new.
`a19_stale_label_per_symbol.asm`:

```
	cpu 68000
	nop
	move.b d0,Fwd-2(a1)
After:
	if After=4
StaleLab:
	endif
	dc.w StaleLab
Fwd	equ $40
```

base `4e71 1340 003e 0004` (3 passes), asl the same (3 passes), new fails:
`unresolved symbol StaleLab`.

### Pre-existing, not introduced (both revisions equal, both differ from asl)

- abs.w/abs.l: sigil stays optimistic abs.w for an unknown address; asl's first-pass
  `*` stand-in picks abs.l when `*` is above `$7FFF`. `a13` (`phase $9000`, two
  fixpoints): sigil `3038 7fff`, asl `3039 0000 8001`. The note lists this as not done.
- `b3_set_twice`: a `set` symbol read above its first `set`: sigil `05 01 05`, asl
  `01 01 05`.
- `b18_string_forward`: a forward string equate errors in sigil, asl emits `41`.
- sigil accepts forward values in `if`, `rept`, `switch`, `include` selection,
  `binclude` offsets, `org`, `save`/`restore` guards and nameless/local labels under an
  `if`, where asl errors ("expression must be evaluatable in first pass").

## Probe table

base and new are passes / bytes; "same" means cmp-identical images or identical error.

| probe | what | base | new | asl | verdict |
|---|---|---|---|---|---|
| a1 | two-fixpoint `Fwd(a1)` at `*`=2 | 2, `12 80` | 2, `1340 0002` | 2, `1340 0002` | base was wrong; new = asl (fixed by A) |
| a2 | two-fixpoint `Fwd-2(a1)` at `*`=2 | 2, `12 80` | 2, `1340 0002` | 2, `12 80` | REAL DEFECT F1 |
| a3 | two-fixpoint `Fwd+2(a1)` | 2, `12 80` | 2, `1340 0002` | `1340 0002` | new = asl |
| a4 | `(Fwd-2)*2(a1)`, one fixpoint | 2 | 2 reads | 2 | same, HELD |
| a5 | unsized `bra` to forward | error (size required) | same | 2, `607e` | pre-existing policy |
| a6 | two-fixpoint abs.w at low `*` | 2 | 2 | 2 | same, HELD |
| a7 | `Fwd(a1)` at `phase $10000` | 3, error (probe's own dc.w) | 2, same error | error | HELD |
| a8 | `addq #9-0` forward | error | error | error | HELD |
| a9 | `Fwd-2(a1)` one fixpoint | 3 | 2 | 3 | same bytes, pass count only |
| a10 | stale name, per-symbol split | 3, `aa` | 2, no `aa` | 3, no `aa` | base wrong; new = asl |
| a11 | stale name, bare symbol | 3, `aa` | 2, no `aa` | 2, no `aa` | base wrong; new = asl |
| a12 | stale name at `*`=0 | 3, `aa` | 2 reads, `aa` | 3, no `aa` | pre-existing (ifdef), same |
| a13 | two-fixpoint abs at `phase $9000` | abs.w | abs.w | abs.l | pre-existing |
| a14 | two-fixpoint `Fld-Base(a1)` | `12 80` | `1340 0002` | `12 80` | REAL DEFECT F1 |
| a15 | `Fld-Base(a1)` one fixpoint | 3 | 2 | 3 | same bytes |
| a16 | stale name, `Fwd equ 0` | 2, no `aa` | 2 reads, `aa` | 3, no `aa` | REAL DEFECT F2 |
| a17 | stale label value, bare symbol | error | 2 reads, `0006` | 3, `0006` | base wrong; new = asl |
| a18 | stale macro call | error | 2 reads, `aa` | 3, `aa` | base wrong; new = asl |
| a19 | stale label value, per-symbol split | 3, `0004` | error | 3, `0004` | BEHAVIOUR DIFFERENCE F3 |
| b1 | `DEFINED(B)` after a conditional `B` | 3 | 2 reads | 1, `00` | same; ifdef pre-existing |
| b2 | `DEFINED(B)` above `B` | 3 | 2 reads | 1 | same, HELD |
| b3 | `set` twice, read above first | 3 | 3 | 2, differs | same; pre-existing |
| b4 | `rept` count forward | 3 | 2 reads | error | same, HELD |
| b5 | `switch` on forward | 2 | 2 | error | same, HELD |
| b6 | macro `if` on forward, labels in two instances | 2 | 2 | error | same, HELD |
| b7 | nameless under `if` (probe typo) | error | error | error | HELD |
| b10 | `align` forward | 2 | 2 | asl crashes (SIGFPE) | same, HELD |
| b11 | `org` forward | 2 | 2 | error | same, HELD |
| b12 | macro defined under `ifdef A`, A later | 2 | 2 | error | same, HELD |
| b13 | function defined under `ifdef A` | 2 | 2 | error | same, HELD |
| b16 | `fatal` under `if` on forward | 2 | 2 | error | same, HELD |
| b17 | label defined in both `if` arms by forward value | 3 | 3 | error | same, HELD |
| b18 | forward string equate | error | error | `41` | same; pre-existing |
| b20 | three-link `ifdef` chain | 3 | 2 reads | 1, `00` | same, HELD |
| b22 | `jmp (X).l`, `X equ Label` later | 3 | 3 | error | same, HELD |
| b30 | `include` chosen by forward | 2 | 2 | error | same, HELD |
| b31 | `binclude` offset forward | 3 | 2 reads | error | same, HELD |
| b32 | `charset` forward | 2 | 2 | 2 | same, HELD |
| b33 | `.local` forward past an `if` | 3 | 3 | error | same, HELD |
| b34 | macro `.local` forward, two instances | 3 | 3 | error | same, HELD |
| b35 | macro plain label forward, two instances (owners index) | 3 | 3 | error | same, HELD |
| b36 | `rept` labels forward | 3 | 3 | error | same, HELD |
| b37 | `while` (probe typo) | error | error | error | HELD |
| b38 | `phase` forward | 3 | 2 reads | 3 | same, HELD |
| b39 | `padding on` with forward | 2 | 2 | 2 | same, HELD |
| b40 | `save`/`restore` around a forward `if` | 3 | 2 reads | error | same, HELD |
| b41 | nameless `+`/`-` past an `if` | 3 | 3 | error | same, HELD |
| c1 | planted control, `ifdef` | 3, `bb00` | 3, `bb00` | 1, `00` | same; controls pin non-asl ifdef |
| c2 | planted control, `ifndef` value | 4, `02` | 3 reads, `02` | 1, `01` | same; as c1 |
| c3 | `ifdef B` above `B = 1` | 2, `bb00` | 2, `bb00` | 1, `00` | pre-existing |

## Is the read log complete

Read of the code at `df96a977`, not only probes. The seeded tables (`env`, `macros`,
`functions`, `known_labels`, `label_ref_equs`) are the only things `one_pass_with_defer`
takes from the previous pass, and their fields are private to `seed.rs`; eval.rs calls
`finish()` only at the end of the pass, and constructs `Seeded`/`SeededEnv` only when
seeding and in `Asm::new`. Every accessor records before answering (the owners index
records even after this pass's writes, which is conservative). The answers carry
everything a reader can learn: `SymbolTable::resolve` collapses Poison and absent, and
nothing in eval.rs can tell them apart through the seed.

Things that read previous-pass state OUTSIDE `seed.rs`, none of which break the
argument:

- `HeadMemo` on `SrcLine` returns a cached "is this head a macro" answer without calling
  `macros.contains_key`. The key includes `macros_gen`, renewed per pass and on every
  macro insert (`12456`), so a memo hit repeats an answer that was recorded or that came
  from this pass's own write.
- `ever_exported`, `carried_fatals`, `carried_author_warnings` accumulate across passes
  and shape the returned result, but an extra identical pass adds nothing to them.
- `str_env` and `float_env` are not seeded at all, so they cannot carry a previous pass.
- The pass number: pass 0 is `FIRST_PASS`, every later pass `LATER_PASS`, and `MOMPASS`
  reads 2 on all of them, so the first pass eligible for rule (b) (pass 1) and the pass
  it stands in for (pass 2) see the same pass number.
- The environment never loses a name (a pass's table starts as a copy of its seed).
  That is what F2 and F3 are made of, but it is the same under both rules.

The weak premise is not the record but what it records: `ifdef` reads the persistent
environment, where asl answers positionally. The record faithfully proves "the next pass
would do the same", which is all rule (b) claims.

## Addendum: round 2, the rework at `9664f887` (rework commit `6bfa6a89`)

Tip built release into `.target-new`; base `911ebb9c` unchanged in `.target-base`; asl and
p2bin as above. Probes and scripts in `2026-09-28-passes-adversarial-review/round2/`
(`summary-round1-probes-at-tip.txt` is the round 1 set rerun, `summary3.txt` the round 2
set). Verdict: nothing broke. No probe, old or new, where base matched asl and the tip
does not; no probe where the tip's read-set convergence gave different bytes from base.

### Round 1 probes rerun at the tip

Comparing the image (or error text) with asl's: base matches on 12 of 50, the tip on 24 (last field of each line compared; `summary-round1-probes-at-tip.txt`).
Every base match is kept. The tip newly matches a1, a3, a10, a11, a12, a17, a18, b1, b20,
c1, c2, c3; a16 and a19 (the F2 and F3 regressions of `df96a977`) match again. The
author's "23 of 57 / 37 of 57" counts a different set (7 of their own probes), so the
figures differ from mine but the claim that matters, no base match lost, holds on my 50.
Error-versus-error pairs count as mismatches in my comparison (the texts differ); b12
and b13 now error as asl does, where base accepted them.

### Round 2 probes (48); base / tip / asl

Per-symbol first-pass stand-in, each built as a two-layout source so the first pass
decides the bytes, unless noted. All: tip = asl.

| probe | what | result |
|---|---|---|
| d10 | three unknowns `Fa+Fb-Fc*2(a1)` | all `1280`, HELD |
| d11 | nested parens `((Fa-2)*(Fb+1))(a1)` | all `1280`, HELD |
| d12 | unknown through a function `ff(Fa)(a1)` | all `1280`, HELD |
| d13 | unknown in a macro argument | all `1280`, HELD |
| d14 | known minus unknown `K-Fa(a1)` | all `1280`, HELD |
| d15 | `(d8,An,Xn)` forward | same bytes, HELD |
| d17 | `ds.b Fwd` | base 3, tip 2 passes, same bytes; asl errors (pre-existing tolerance) |
| d18 | `movem.l` to `Fa-2(a1)` | all 3 passes, same bytes, HELD |
| d19 | `move.l #Ptr,Fa-2(a1)` (the deferred path) | all 3 passes, same bytes, HELD |
| d20 | `Fa-Fa(a1)` | all collapse, HELD |
| d21 | `Fwd>>16(a1)` | base 3, tip 2, asl 3, same bytes, HELD |
| d22 | per-symbol at `phase $10002` (low 16 bits of `$10000`) | all collapse, HELD |
| d23 | `lea Fa-2(a1),a2` | all `45d1`, HELD |
| d24 | `Fa-2(pc)` | same, HELD |
| d25 | `addq #Fwd+20` (probe's own range error) | all error, HELD |
| d26 | `Fwd-*(a1)` | all `1280`, HELD |
| d27 | `Fwd-$(a1)` | all refuse the spelling, HELD |
| d28 | local `.fwd` defined after a new global label | all error, HELD |
| d29 | unknown defined by `set` | all `1280`, HELD |

Positional `ifdef`/`ifndef`. All: tip = asl except where asl has no comparable result or
the row says pre-existing.

| probe | what | base | tip | asl |
|---|---|---|---|---|
| e10 | `set` and `=` names, before and after | `a1a2a3a4` | `a2a4` | `a2a4` |
| e11 | macro-local label, ifdef above/below it in two expansions | `a100a2a100a2` | `00a200a2` | `00a200a2` |
| e12 | `.local` above/below, then a new scope | `a100a2` | `00a2` | `00a2` |
| e13 | `-D Dx=3`, ifdef above and below `Dx set 5` (`rund.sh`) | `a1a205` | `a1a205` | `a1a205` |
| e14 | name defined in an included file; ifdef inside it of a later name | `a1b1a200` | `a200` | `a200` |
| e15 | ifdef in a macro expanded before and after the definition | `0101` | `0201` | `0201` |
| e16 | name bound between `save` and `restore` | `a100` | `a100` | `a100` |
| e17 | label on the ifdef's own line | `a100` | `a100` | `a100` |
| e18 | `X equ Fwd`, Fwd later | `a100` (3) | `a100` (2) | `a100` (2) |
| e19 | `X equ Fwd`, two-link chain | `a100` (4) | `a100` (3) | error |
| e20 | include-guard idiom | `07` | `07` | `07` |
| e21 | `dc.w L` above `ifndef L / L:` | `0004` (block dropped) | `00044e714e71` | `00044e714e71` |
| e22 | name bound only by pass 0's layout, read through ifdef then by value | error | `4e71128002` | `4e71128002` |
| e23 | macro label read from outside | error | error | error |
| e24 | `ifdef TRUE`/`MOMCPU` | `00` | `00` | `a2a300` (pre-existing) |
| e25 | `ifdef x` for `X` (asl case-insensitive by default) | `00` | `00` | `a100` (pre-existing) |
| e26 | `reg` alias (sigil lacks `reg`) | error | error | error |
| e27 | string equate | `00` | `00` | `a200` (pre-existing) |
| e28 | `enum` members | `a1a200` | `a100` | `a100` |
| e29 | inside `rept`, `set` after the ifdef | `a1a100` | `a100` | `a100` |
| e30 | ifdef of a name whose value needs the block it guards | error | error | error |
| e31 | `-D` name read by ifdef inside a macro (`rund.sh`) | `a1a100` | `a1a100` | `a1a100` |
| e32 | ifdef of an equ of a forward, arms define the forward | `a101` (4) | `a101` (3) | `a101` (4) |
| e33 | name defined only under an ifdef of a later name | `a100` | `00` | `00` |
| e34 | ifdef of an equ of a forward label | same bytes (4) | (3) | (4) |

Stale-value and pass-count channels (f1 to f4): `f1` builds a stale name through the
abs.w/abs.l stand-in (sigil optimistic abs.w, asl `*` at `phase $9000` picks abs.l on
pass 1): base and tip error `unresolved symbol StaleLab`, asl succeeds. Pre-existing,
the same in both revisions; the note's "not done" item. `f2` (`jsr` to a far forward
label): base 3, tip 2 passes by reads, bytes same as asl. `f3` near `jsr`: same. `f4`
(`ifndef Seen / Seen = After` capturing layout): all `0004`.

Caller-predefined names (`Options::defines`, `guarded_defines`, the path aeon's native
driver uses for `__DEBUG__` and `__MDDBG__`) cannot be passed on the command line, so
they were checked with a scratch integration test run against the tip's source and then
deleted (not committed): `ifdef PD`/`ifdef GD` true at file level and inside a macro
expanded twice, `ifndef PD` false, and with no defines both false and `ifndef` true.
1 passed.

### The b12/b13 shape in real sources

A b12/b13 refusal needs an `ifdef`/`ifndef` (or `DEFINED()`) whose answer changes when
it stops reading earlier passes. The three corpora, as extracted at the SHAs of the perf
note, contain no `ifdef` or `ifndef` in any `.asm` file; `DEFINED(` appears only as
`(MOMPASS=1)&&(DEFINED(loc))` in each `_smps2asm_inc.asm`, and `DEFINED()` did not change.
aeon has 3 tracked `.asm` files; their `ifdef`s name only `__DEBUG__` (five sites, in
the `ifdebug` and assert macros in `engine/debug/debugger.asm`) and `__MDDBG__`
(`games/*/game_root.asm`), both pushed through `defines` by
`sigil-harness/src/native.rs` and so predefined, which the scratch test covers.
The tip builds all three corpora cmp-identical to their stock ROMs, 2 passes each
(`corpora.sh`, outputs written only to this worktree): S1 `09dadb50`, S2 `9feeb724`,
S3K `4ea493ea` (md5 prefixes, equal to the reference). Not rebuilt: aeon itself, and the
S3K `Sonic3_Complete=1` variant; neither has an `ifdef` beyond those named here.

### Judgement on the new code

`fold_unknown_as_pc` substitutes per name inside `Expr::fold`, so functions (expanded to
tokens before the fold), macro arguments (substituted before the fold), parens and
several unknowns all reach it as plain names, which the d probes confirm. It is used only
for the first-pass `(d16,An)` displacement; the other size-deciding first-pass value,
abs.w/abs.l, still differs from asl (a13, f1), as the note already says, and did before
this change.

`cond_defined` now reads `defined_this_pass` (this pass's own writes) and `predefined`
(fixed per run); its `resolve_sym` is reached only for a key this pass wrote (not a seed
read) or a predefined key (recorded, and a later redefinition in the source moves the
recorded answer, so it forces the next pass as it should). So `ifdef` no longer depends
on the seed except through `sym_key`, whose `owners`/`resolve` reads are recorded. The
read-log argument of round 1 still holds. Remaining ifdef differences from asl are
pre-existing and the same at base: builtins (`TRUE`, `MOMCPU`), string equates, and asl's
default case-insensitivity.
