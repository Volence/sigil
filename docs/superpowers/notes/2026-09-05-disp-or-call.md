# A name spelled two ways, and the parenthesis that decides which

2026-09-05 · branch `parcel/as-insn2op-disp-operand` · sigil master base `7dcac58`

Ninth in the AS-frontend arc for the public Sonic 2 disassembly
(`/home/volence/sonic_hacks/s2disasm`, git `e45ebf3`). The booked row was
`AS-INSN2OP-DISP-OPERAND`, 162 diagnostics, described as *unexamined*: one arm
of `insn2op`, the two-operand wrapper, newly reachable since the setup file's
conditionals began picking the right processor.

Ground truth throughout is an `asl -L` listing run with the Sonic 2 build's own
flags minus the two that only redirect output: `asl -xx -n -q -A -L -U -i .`.
Every rule below is stated with the row that establishes it.

> **THE INSTRUMENT, CORRECTED (2026-09-05).** This note first cited its
> assembler as *AS V1.42 Beta [Bld 212], `s2disasm/build_tools/Linux-x86_64/asl`*
> — a version banner and a path. **The banner identifies nothing**: four `asl`
> binaries in this workspace print it verbatim and are not the same program, and
> the one this note used, md5 `0dee1f98e6480a4783d27ffd8b90896f`, answers any
> operand it declined to value from an uninitialized word. Cite the digest.
>
> Every row below has since been **re-measured on the reference build, md5
> `61e672562465725a8c102288a7da9098`** (`s1disasm/build_tools/Linux-x86_64/asl`),
> four runs each, against the same flags. The probes are committed at
> `2026-09-05-disp-or-call-probes/` — they were not, when this note was written,
> which is why re-measuring meant reconstructing them from the listings quoted
> here. Where a row moved it is marked SUPERSEDED in place and the old value is
> left visible.
>
> **No rule in this note rested on an unstable draw.** Every row that carries a
> rule — the structural peel, the `1+konst(a1)` bytes, `dsp(k)+2(a1)`, and both
> refusals with their carets — is byte-identical on both builds and stable
> across four runs. The two rows that move are the two the note already declined
> to build a rule on, and they move for a reason that makes that decision
> stronger rather than weaker: see *An asl defect found on the way*.

## What the arm is

`s2.macrosetup.asm:122` defines `insn2op`, which defeats AS's zero-offset
optimization without patching AS: assemble the operand as `1+y`, so the
displacement word is emitted, then rewind and overwrite the low byte with zero.

```text
122 insn2op  macro oper,x,y
123   if (chkop("x","0(") && chkop("x","id(") && …)
124     if (chkop("y","0(") && chkop("y","id(") && …)
125       !oper   x,y
126     else
127       !oper   x,1+y
128       !org    *-1
129       !dc.b   0
```

`chkop` is TRUE when the operand does NOT begin with the given prefix, so
`:127` is the arm where the SOURCE is ordinary and the DESTINATION is one of the
zero-offset forms. The corpus's dominant call site is
`_move.b #ObjID_X,id(a1)` — 100-odd of them — and what reaches asl there is

```text
!move.b   #ObjID_Ring,1+id(a1)
```

## What it does today, and what asl does

The two differ, and the reason is not the arm.

`id` is spelled BOTH ways in this corpus. `s2.constants.asm:15` makes it an
object-record field offset (`id = 0`); `:438` makes it a user function
(`id function ptr,((ptr-offset)/ptrsize+idstart)`) used to number pointer
tables. So `id(a1)` is ambiguous on its face: a displacement of `id` over `a1`,
or a call with `a1` as the argument.

**asl resolves it structurally, by peeling the addressing mode first.** With a
name deliberately given both spellings and values far apart:

```text
       2/       0 : =$2A                 dsp	=	$2A
       3/       0 :                     dsp	function p,(p*7)+$100
       7/    1000 : 337C 1234 002A      	move.w	#$1234,dsp(a1)
       9/    1006 : 337C 1234 002B      	move.w	#$1234,1+dsp(a1)
      11/    100C : 303C 0115           	move.w	#dsp(k),d0
```

Displacement `$2A` and `$2B` — the EQUATE — with the function untouched. The
same file's immediate on line 11 gives `$115`, so the function is live; the
operand rows are a choice, not an absence.

Stated as a refusal it is the same rule, and the caret is the whole message —
it sits under the NAME, not under the register:

```text
> > > ta.asm(5):16: error #1010: symbol undefined
> > > konst
> > >  move.w #$1234,konst(a1)
> > >                ~~~~~
```

The peel is driven by the BASE alone. A two-element group is an addressing mode
with a bad index register, never a two-argument call:

```text
> > > te.asm(6): error #1350: addressing mode not allowed here
> > >  move.w #$1234,1+dsp(a1,zz)
```

And the exclusion is the trailing GROUP, not the operand — a call inside the
displacement expression still expands, with `(a1)` still the base:

```text
       9/    1004 : 337C 1234 0117      	move.w	#$1234,dsp(k)+2(a1)
      11/    100A : 33BC 1234 202B      	move.w	#$1234,1+dsp(a1,d2.w)
```

**Sigil expanded first and peeled afterwards.** `Evaluator::lower_m68k` ran
`expand_calls` over the whole operand token list before `parse_operands`, so
`id(a1)` was a call and `a1` was substituted into the body. Hence the booked
row's message, `unresolved symbol `a1` in operand`, at a line whose text
contains no `a1` at all. The comment above the call stated the assumption that
failed: *"register-indirect EAs (`(a0)`, `(4,a0,d1.w)`) pass through
untouched"* — true, and true only while no function name abuts the paren.

## It was not only a refusal

The same path emits wrong bytes silently when the function's body does not
mention its parameter, because the call then folds instead of failing:

```text
konst	=	$2A
konst	function p,$3C7
       5/    1000 : 337C 1234 002B      	move.w	#$1234,1+konst(a1)     ; asl
                    31FC 1234 03C8                                       ; sigil, before
```

Six bytes either way, exit 0, no diagnostic: `move.w #$1234,$2B(a1)` against
`move.w #$1234,($3C8).w`. Not present in any corpus checked — it needs a name
that is both a symbol and a parameter-free function — and recorded because the
class is the hazard, not the instance.

## The fix

`operands::m68k_ea_base_spans` returns the index ranges of every trailing
`(An)` / `(An,Xn)` / `(pc)` / `(sp)` group in an operand list;
`Evaluator::expand_calls_m68k_operands` expands around them. It can only
NARROW the window, and only for an operand ending in a register base group; an
operand with no such group takes the unchanged whole-slice path. Immediates are
excluded (see the asl defect below).

## Corpus

`s2.asm` through `assemble_root_located_warned`: **6035 → 5761**, −274. Sets
compared BOTH directions — **zero new diagnostics anywhere**, 122 rows removed.

| class | removed |
|---|---|
| `unresolved symbol `a1` in operand` | 123 |
| `unresolved symbol `a0` in operand` | 37 |
| `absolute address operand `(expr)` needs an explicit width suffix` | 114 |

The booked row is fully retired: `s2.macrosetup.asm` `:127` (98), `:141` (54),
`:148` (6), `:115` (1) → **0**, which is 159 of the 274.

The other 115 are the SAME cause reached WITHOUT the wrapper, at sites the
booked row does not mention — the real mnemonic, `id(a1)` written straight into
`s2.asm`:

- **114 width-suffix rows**, every one of them a whole-operand `id(aN)`: 67
  `move.b` (`move.b #ObjID_CutScene,id(a1)`, `:13099`), 25 `cmpi.b`, 19
  `tst.b`, 3 `cmp.b`. Sigil folded `id(a1)` to `(((a1)-offset)/ptrsize+idstart)`
  — a bare parenthesised expression — and demanded a `.w`/`.l`. Enumerated, not
  sampled: all 114 lines contain `id(`, and none contains anything else.
- **1 unresolved-symbol row**, `s2.asm:5069`,
  `tst.b TitleCard_ActNumber-TitleCard+id(a1)`. Here the displacement is a real
  expression, so the fold put `a1` in front of the resolver exactly as the
  `1+y` arm did.

The booked row named one arm of one macro. The cause was one line in
`lower_m68k`, and it predates the wrapper entirely — `insn2op` only made it
visible, by putting a `1+` in front of the name.

## Aeon

Structurally unreachable there. **Aeon defines no AS `function` at all** — three
`.asm` files remain (`games/demo/game_root.asm`, `games/sonic4/game_root.asm`,
`engine/debug/debugger.asm`) and none carries the directive, so the ambiguity
cannot arise. All four shapes deleted and rebuilt anyway, byte-identical to the
frozen provenance tip.

Every `function` name in s1disasm and s2disasm was swept for a `name(<reg>)`
use. `id` is the only one.

## An asl defect found on the way, and why no rule was built on it

`#f(<register>)` — a function call in an IMMEDIATE whose argument is a register
name — makes AS V1.42 Beta 212 emit a **different value on every run**, with
`0 errors` and `0 warnings`:

> **⚠ THIS PARAGRAPH NAMES THE WRONG THING AS THE CAUSE, TWICE OVER**
> *(corrected 2026-09-05 by the nondeterminism sweep; the observation above is
> real and reproduces, the attribution does not).*
>
> **It is a property of a BUILD, not of a version.** Four asl binaries in this
> workspace print `1.42 Beta [Bld 212]` verbatim; only s2disasm's
> (md5 `0dee1f98`) varies, while s1disasm's (`61e67256`) is stable —
> confirmed here by md5 against identical banners. **`ASL_BIN` accepts any of
> them and the version string cannot tell them apart**, so citing the version
> has not identified the instrument. Cite the md5.
>
> *(Amended 2026-09-05: this said "a stable ZERO". It is stable but it is not a
> zero — probe `d9` shows the reference build returning the last value it
> computed, so what it substitutes depends on the lines above. The zero that
> was observed was the initial state of that slot. Nothing else in this
> paragraph changes.)*
>
> **And the trigger is neither the function, the register, nor the immediate.**
> Minimal form is `move.w #zz,d0` with `zz` UNDEFINED; `#zz(qq)`, `#zz(5)`,
> `#(a1)` and bare `#zz` are equally unstable, while `#(5)`, `#f(5)` and a
> defined `#zz` are stable. The class is wider than symbols — range-refused
> immediates containing no symbol at all vary too. **The rule is ANY OPERAND
> ASL DECLINED TO GIVE A VALUE**, and the mechanism is an uninitialized read,
> collapsing to a constant `$5555` under `setarch -R`.
>
> The `1+dsp(d3).w` → `F83C` F-line claim below is subject to the same
> correction: it is build-specific AND itself unstable (alternating
> `F83C`/`783C`), while the stable build emits `343C 1234`. *(Re-measured
> 2026-09-05 from `2026-09-05-asl-nondeterminism-sweep-probes/s_fline_shape.asm`
> — reference `343C 1234` 4/4, varying `783C` ×3 then `F83C` ×1. Confirmed. The
> reference build's `343C` is a substituted stale value, not an F-line and not
> an answer; see `d9`.)*
>
> **The decision this note recorded — build no rule on the shape — stands, and
> stands more firmly**: the silent arm (`#f(<reg>)` with `f` defined) is exit 0
> with no diagnostic on BOTH builds, so it is silently wrong even where it is
> reproducible. That is a correctness defect, not a reproducibility one.

```text
       6/    1004 : 303C 55A2           	move.w	#konst(a1),d0     ; run 1
       6/    1004 : 303C 55B7           	move.w	#konst(a1),d0     ; run 2
       6/    1004 : 303C 5612           	move.w	#konst(a1),d0     ; run 3
```

`#konst(5)` on the line above gives the correct `$03C7` in all three. The value
does not depend on which register (`a1`, `a3`, `d5` all agree with each other
within a run) nor on whether the function body uses its parameter. There is no
answer here to match, so immediates are excluded from the peel and sigil's
existing behaviour for that shape is left alone rather than pinned.

A second degenerate shape, `1+dsp(d3).w`, has asl emitting `F83C 1234` — an
F-line word — silently. Also excluded, for the same reason.

> **SUPERSEDED, and the decision survives on better evidence (2026-09-05).**
> Those three `$55xx` words are the VARYING build's uninitialized read and are
> kept above so the supersession is visible rather than silent. Re-measured on
> the reference build (md5 `61e67256…`, probes `d6`/`d7`/`d8`/`d9`):
>
> | row | varying `0dee1f98` | reference `61e67256` |
> |---|---|---|
> | `#konst(a1)` after a `#konst(5)` line | `55A2`/`55B7`/`5612`, new each run | `03C7`, 20/20 runs and 5/5 under `setarch -R` |
> | `1+dsp(d3).w` | `783C 1234` ×3, `F83C 1234` ×1 | `343C 1234`, 4/4 runs |
>
> **The reference build's stability here is not an answer.** `d9` puts three
> declined `#f(<reg>)` immediates after three successful `#f(5)` calls holding
> `$0111`, `$0222`, `$0333`, and the reference build returns `0111`, `0222`,
> `0333` — each declined operand **echoes the last value that build computed**.
> `d8` reads `0000` in all four cells only because no call had succeeded above
> them. Which is to say: the reference build substitutes a STALE VALUE and the
> varying build substitutes an UNINITIALIZED one, and the paragraph above was
> right that there is no answer to match — it was wrong only in thinking the
> reference build had one. Exit 0 and no diagnostic on both.
>
> That is the worse of the two failure modes to inherit, because a stale value
> is reproducible: re-run it and it agrees with itself, so it reads as a
> measurement. **A value taken off the reference build for an operand asl
> declined is an artifact of the lines above it, not of the line it is on.**
>
> `d8` also refutes a plausible reading of `d6` that I held for one measurement:
> that the presence of an EQUATE of the same name is what makes `#name(<reg>)`
> expand. All four of `d8`'s cells — function-only and function-plus-equate,
> body using its parameter and body ignoring it — return the same word. The
> discriminator is source order, nothing about the name.
>
> The exclusion of immediates from the peel therefore stands, and stands on the
> stronger ground that NEITHER build answers this shape.

## Gates

| name | file | must fail when |
|---|---|---|
| `as_disp_name_that_is_also_a_function` | `snippets_golden.txt` | the trailing group is expanded — the displacement becomes the function's value, or the line is refused |
| `as_insn2op_zero_offset_arms_with_id_also_a_function` | `snippets_golden.txt` | the corpus's own `insn2op` shape stops assembling, or its trailing `dc.w id($1C6)` stops being `00 C9` |
| `a_function_only_name_over_an_address_register_is_an_undefined_symbol` | `tests/as_disp_vs_function.rs` | the refusal blames `a1`, or (with a parameter-free body) there is no refusal at all |
| `the_insn2op_displacement_arm_takes_the_equate_not_the_function` | same | `1+id(a1)` is expanded as a call |
| `a_two_element_trailing_group_is_an_addressing_mode_not_a_two_arg_call` | same | the complaint moves off the index register |
| `a_call_in_the_displacement_still_expands` | same | the fix is widened to "never expand a call in an operand" |

Both snippet blocks are minted from real asl by `gen_snippet_vectors`;
regeneration churned only the new blocks (59 insertions, **0** deletions), so
every pre-existing golden reproduced byte-identically — the non-circularity
invariant.

**On the fixture rule.** Every value here is multi-digit and non-round, and the
equate and the function are chosen to share no digits: `dsp = $2A` against
`dsp(k) = $14D`, `id = 0` against `id($1C6) = $C9`, immediate `$1234`,
`ObjID_Ring = $B7`. The one deliberate identity is `id = 0` in the `insn2op`
block, and it cannot hide anything because it is not the discriminator: the two
readings there are "six bytes" and "a refusal".

**Red-first**, from the committed baseline `8e346abf`, both mutations shown
applied on disk:

- *M1* — `expand_calls_m68k_operands` → `expand_calls`: 3 of 4 diagnostic tests
  RED plus the snippet gate. `a_call_in_the_displacement_still_expands` stays
  green, by design.
- *M2* — the method body → `toks.to_vec()` (never expand in an operand): that
  one test RED (`trailing tokens in `disp(An)` displacement`) plus the snippet
  gate, the other three green.
- Restored with `git checkout` from the commit, all five green again.

Neither mutation leaves every gate green, and neither leaves every gate red —
which is the point: the two failure directions are separated.

## One misattribution, caught by enumerating instead of sampling

The corpus decomposition above was first written the other way round — the 114
width rows described as `+id(` displacement expressions and the single
unresolved row as a bare `id(a1)`. The cause was a two-range `sed -n
'13095,13101p;5065,5072p'`: sed emits in FILE order, not argument order, so the
`5065` block printed first and was read as the `13095` one. Both blocks were
real, both were relevant, and the labels were swapped.

What caught it was declining to generalise from the two lines that had been
read: `grep -c "+id("` over all 114 returned **0**, which cannot be true of a
class described as `+id` expressions. The check that found it is the same one
that would have prevented it — enumerate the population and let it disagree
with you, rather than reading two members and naming the set.
