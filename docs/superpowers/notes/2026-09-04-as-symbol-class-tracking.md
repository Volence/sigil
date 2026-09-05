# asl's two symbol classes, and the crossings we were accepting

Everything below is read off `asl` 1.42 Beta Bld 212 —
`s1disasm/build_tools/Linux-x86_64/asl`, the upstream build, md5
`61e672562465725a8c102288a7da9098` — run with **Sonic 1's own flags**
(`build_tools/lua/common.lua:773`) minus `-E` and `-c` so the diagnostics land on
stdout:

```
asl -xx -n -q -A -L -U -i . <root>.asm
```

Probes are `m1.asm`–`m11.asm` under `2026-09-04-as-symbol-class-probes/`, with
`run.sh` (asl) and `sigil.sh` (sigil) beside them. **No expectation here is a
reading of the semantics.** Each is a listing row.

---

## 1. The rule, as measured

asl does not have one "bind a symbol" operation with several spellings. It has
**two classes**, and it refuses every crossing between them.

| | second declaration | asl |
|---|---|---|
| **const → const** | `Ae equ 1` / `Ae equ 2` | `#1000 symbol double defined` |
| **const → var** | `Ce equ 1` / `Ce set 2` | `#2030 constants cannot be redefined as variables` |
| **var → const** | `As set 1` / `As equ 2` | `#2035 variables cannot be redefined as constants` |
| **var → var** | `Cs set 1` / `Cs set 2` | accepted, silently; the value updates |

### Which spelling is in which class

| class | forms | probe |
|---|---|---|
| **CONSTANT** | `equ` · `=` · `NAME:` colon label · column-0 bare `NAME` · `NAME: dc.…` · `NAME label expr` · an `enum`/`nextenum` member | m1, m3, m4, m5 |
| **VARIABLE** | `set` · `eval` · `:=` | m1, m2 |

`=` is a **CONSTANT**, and that is the first thing the booking did not say:

```text
       7/       0 : =$1                  Be	equ	1
> > > m1.asm(8): error #1000: symbol double defined
> > > Be
> > > Be = 2
       8/       0 : =$2                  Be	=	2
```

The three variable spellings are **one class**, not three similar directives —
`set` then `eval` then `:=` over the same name is silent (m2 lines 9-14).

### Three properties the rule does NOT have

**It is not about the value changing.** `Aq equ 1` written twice, same value, is
still `#1000` (m4 line 6).

**It is not about the value's TYPE.** A string `equ` followed by a string `set`
is `#2030`; a float `equ` followed by a float `set` is `#2030` (m5). The class
belongs to the declaring directive.

**It is not about the written spelling of a local name.** asl quotes the
QUALIFIED name, and two scopes' `.loc` are two symbols:

```text
> > > m6.asm(22): error #2030: constants cannot be redefined as variables
> > > Sc1.loc
> > > .loc set 2
```

### It is PER PASS

m7 forces two passes with a forward branch, so its `Ap equ 1` and its `Lab:`
each execute twice. asl reports `2 passes / 0 errors`. Re-running a declaration
on a later pass is not a redefinition.

### A macro expansion declares into the CALLER's class space

```text
> > > m6.asm(12) mset(1): error #2030: constants cannot be redefined as variables
> > > Am
> > > Am       set     9
```

### Recovery: the OLD value survives

After a refused `Ce set 2`, asl's listing column shows the computed `=$2` but its
symbol table still reads `*Ce : 1` (m1). Same for every `var → const` cell in m9
— `*Av : 1`, `*Bv : 1`, `*Cv : 1`, `*Dv : 1`. Assembly continues; asl writes no
`.p` when any error occurred, and exits 2.

### Cells NOT probed

- `NAME macro` / `NAME function` / `NAME struct` against either class. Macros,
  functions and structs live in their own tables in this front end, and asl's
  behaviour for a name declared in two of those namespaces was not measured.
- `NAME reg <register>` (register aliases).
- Any interaction with `save`/`restore`, `pushv`/`popv`, or `section`-scoped
  symbol namespaces.
- `#1000`'s own population beyond the four probes above — how asl treats a label
  redefined inside `rept`, `phase`/`dephase`, or a twice-included header.
- Whether a REFUSED label still moves asl's local-label scope anchor. m9 looked
  like evidence that it does (`Dv.aft` after a refused `enum Dv=5`), but m10
  shows a plain `set` ALSO opens a scope (`Av set 1` then `.loc1 equ 7` lists as
  `Av.loc1`), so the two readings are indistinguishable and neither is asserted.

---

## 2. What sigil did, and the sharpest shape of it

**Nothing at either crossing.** Probe `m11.asm`, run against the binary built at
the parcel's own branch point:

```
X	equ	1
X	set	2
	dc.b	X
Y	set	1
Y	equ	2
	dc.b	Y
```

| | result |
|---|---|
| **asl** | 2 errors (`#2030`, `#2035`), no `.p`, exit 2; its listing emits `01` and `01` — the OLD values |
| **sigil, before** | **no diagnostic of any kind, exit 0, bytes `02 02`** — the NEW values |
| **sigil, after** | both crossings refused, exit 1 |

Two assemblers, one source, two different byte streams, and nothing anywhere
said so. This is the same shape as the macro-argument finding in
`2026-09-04-as-silent-acceptance-probes.md` §1a.

The `const → const` cells were NOT entirely silent before: `equ` twice reached
`sigil-link`'s duplicate-equ_sym check and produced `symbol redefined by section`
— from the LINKER, not the front end, and only for the forms that export an
equ_sym (`Fq label $100` then `Fq equ 2` was silent, where asl says `#1000`).

---

## 3. What changed

`crates/sigil-frontend-as/src/eval.rs`:

- `enum SymClass { Const, Var }` and a **per-pass** `Asm::sym_class` map.
- `Asm::declare_class(q, class, span)` — records the class, reports the crossing
  with asl's own two sentences, returns `false` when refused.
- Wired at five declaration sites: `directive_equate` (Const), `directive_set`
  (Var, and so `set`/`eval`/`:=` and the `NAME,VALUE` comma forms),
  `define_label` (Const), `directive_label` (Const), `enum_members` (Const).
- `define_label` gained a `span` parameter; `label_span()` reconstructs it from
  the line's base column.

**Recovery.** `directive_equate` and `directive_set` return before binding, which
reproduces asl's surviving old value exactly. The three label-ish sites report
and change nothing else — a label is also a scope anchor and a placed section
label, and the probes cannot say what asl's scope does after a refusal (see the
un-probed cell above). Inventing that on a path that already fails the build was
not worth the risk.

**Deviation, taken knowingly.** A command-line `-D` define is a **VARIABLE** to
asl (m8 run with `-D Dw=1` makes `Dw equ 2` a `#2035`). sigil's seeded defines
enter with **no class**, so the first in-source declaration establishes one.
`Options::defines`' own doc requires this — *"an in-file `=`/`equ` of the same
name wins (the code-gate / game-config-override defines rely on this)"* — and
`guarded_defines` are `.emp`-owned CONSTANTS with their own loud
`[defines.collision]` refusal, which classing them as variables would break.

**Not closed: `#1000`.** The same-class constant redefinition is a different
rule with a different population — every duplicated label, every twice-included
header — and it wants its own measurement rather than a half-covering patch
bolted onto this one. Named in the code and here rather than closed silently.

> **CLOSED 2026-09-05** by `2026-09-05-as-duplicate-definition.md`, and the
> guessed population above is wrong in a way worth reading: a PC label and an
> `enum` member inside a macro/`rept`/`irp`/`while` expansion are LOCAL to that
> expansion in asl and are not in `#1000`'s population at all. "Every duplicated
> label" would have refused 97 sites in the s2 corpus that asl assembles.

---

## 4. Population: nothing anywhere trips the new refusal

| tree | how measured | result |
|---|---|---|
| aeon, all 7 shipped shapes | `corpus_builds` under `SIGIL_STRICT_GATE=1`, `AEON_DIR=.aeon-verify-483` | 2 passed / 0 failed — every shape `Ok` with zero error-level diagnostics |
| aeon, 4 ROMs | `sigil build --aeon` for each shape, CRC32+size | **identical**: `1c09fbfc`/819131, `e2144057`/840324, `11ebd7ab`/96602, `9b0d2ce7`/102818 |
| s1disasm | `sigil sonic.asm` from the corpus root, before-binary vs after-binary | 57 diagnostics each, output **byte-identical** |
| s2disasm | `sigil s2.asm`, same | 6,035 diagnostics each, output **byte-identical** |

Both corpus diffs are empty, so the newly-unresolved and newly-resolved symbol
name sets are both **empty in both directions**, and the per-class decomposition
is unchanged with no class rising and no class appearing. The aeon ROMs prove the
change **reaches LINK** — a front-end-only measurement never gets there, and the
four images were flattened, not merely assembled.

s2disasm's live checkout lacks its gitignored `sound/*/generated/*.inc`, so 22 of
its 6,035 rows are `cannot include`. That is a property of the tree, identical in
both runs, and it does not affect the comparison.

---

## 5. Found while probing, not fixed here

**asl's `set` OPENS the local-label scope.** m10:

```text
*Av :                             1 - | *Av.loc1 :                        7 - |
*Scope1 :                         0 C |
```

`Scope1:` then `Av set 1` then `.loc1 equ 7` lists as `Av.loc1`, not
`Scope1.loc1`. sigil's `directive_set` qualifies against `real_scope()` but never
SETS it, so sigil binds `Scope1.loc1`. Neither assembler complains, so this is a
silent name divergence, not a diagnostic one — a local written after a `set` in a
scope resolves to a different symbol in the two assemblers. It has no population
in aeon (the ROMs are byte-identical) and it did not move either corpus count,
but it is a real disagreement and it wants its own parcel with `save`/`restore`
and macro-scope probes, not a same-day patch.

**`NAME label expr` exports no equ_sym**, so the `#1000` the linker catches for
`equ`-then-`equ` does not fire for `label`-then-`equ` (m4's `Fq`). Folded into
the `#1000` parcel.

---

## 6. Gates

`crates/sigil-frontend-as/tests/as_symbol_class.rs`, 8 tests, run by
`cargo test -p sigil-frontend-as` and by the landing run. Acceptance and refusal
are asserted **as a pair**, so a front end that refuses everything fails as hard
as one that refuses nothing.

Four mutations, each shown applied on disk with `git diff` and each restored with
`git checkout HEAD --` from a committed baseline:

| mutation | what it removed | red |
|---|---|---|
| A | the whole `declare_class` call in `directive_set` | 5 of 8 |
| B | the `prev != class` guard — refuse EVERY redeclaration | 3 of 8, all acceptance rows |
| C | the Const recording in `define_label` + `directive_label` | 2, naming `colon label then set` and `set then a colon label` |
| D | the Const recording in `enum_members` | 2, naming `enum member then set` and `set then an enum member` |

**What each gate MUST FAIL:** a build that binds a `set` over an `equ`'d name
(A); a build that refuses a legal `set` reassignment (B); a build that forgets a
label or an `enum` member is a constant (C, D).

**One test is labelled as NOT a gate**, in its own doc:
`re_executing_a_declaration_on_a_later_pass_is_not_a_redefinition`. The per-pass
class map is the right design, but under a crossing-only rule it is **not
observable** — re-executing a declaration gives the same class twice, and only a
crossing is checked — so no mutation could have turned that test red. It is
regression coverage and a record of asl's `2 passes / 0 errors`, and it becomes a
real gate the day `#1000` lands.
