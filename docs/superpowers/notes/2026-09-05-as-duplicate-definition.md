# `#1000 symbol double defined`, and the cell asl does not have

The same-class half of the symbol matrix, and the row the crossing parcel
(`2026-09-04-as-symbol-class-tracking.md`) deliberately left open. Everything
below is read off `asl` 1.42 Beta Bld 212 —
`s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098` — with Sonic 1's own flags minus `-E`/`-c`:

```
asl -xx -n -q -A -L -U -i . <root>.asm
```

Probes are `m12.asm`–`m20.asm` under `2026-09-04-as-symbol-class-probes/`,
beside the crossing parcel's `m1`–`m11`.

**Oracle stability.** Every probe this note relies on — `m1`–`m6`, `m9`,
`m12`–`m20`, sixteen files — was run **three times** and the whole diagnostic
stream hashed. One hash, `544d9b7ecf094dd1fa0dc71ccd63cce3`, all three runs.
No measurement here sits on an unstable shape.

> **⚠ THAT HASH IS UNVERIFIABLE AS WRITTEN, and the defect is provenance rather
> than arithmetic** *(2026-09-05, found by the de-clock parcel)*. **This note named
> no runner**, so "the whole diagnostic stream" is not a defined quantity. Three
> plausible readings were hashed and NONE reproduces it — stderr alone
> `350912c01482`, stderr+exit `5a03c71bca12`, `run.sh`-shaped with the listing
> `7bc5cc26506d`. **The claim is not contradicted; it is uncheckable**, which is
> the weaker and more corrosive state, because it reads exactly like a verified
> one. THE CONCLUSION STANDS on its own logic — identical output across runs
> implies identical content, whatever was hashed — so nothing measured here is
> withdrawn. What is withdrawn is this line's status as EVIDENCE anyone can
> re-run. A committed `stability.sh` now exists beside these probes (19 of the 20,
> `m8` excepted for needing `-D` defines the runner does not pass): all STABLE,
> zero unstable, and re-runnable. **Cite that, not this paragraph.** (The known non-determinism, a
function call in an immediate whose argument is a register name, is not in any
of these probes.)

---

## 1. The rule, as measured

A second **executed** declaration of a name in the CONSTANT class is
`#1000 symbol double defined`. The first value survives; assembly continues;
asl writes no `.p` and exits 2.

Three properties it does NOT have:

**It is not about the value changing.** `Aq equ 1` twice, same value, is `#1000`
(m4). So is `Fq label $100` twice (m18-shaped), and so is a header included
twice whose `equ` binds 5 both times (m14). And the converse: `m15.asm` refuses
a `Bp:` whose logical AND physical address both moved, so a moving value is
neither necessary nor sufficient — it simply is not the question.

**It is not about the name appearing twice.** m16 puts the second `equ` under
`if 0` and asl is silent; the `if 1` twin is `#1000`. What counts is a
declaration the pass EXECUTES.

**`phase`/`dephase` does not exempt it.** m15, above.

### The cell that decided the shape of the whole parcel

**A PC label and an `enum` member written inside a macro / `rept` / `irp` /
`while` expansion are LOCAL TO THAT EXPANSION.** They never enter asl's global
symbol table, so a second expansion redeclaring one is silent.

| form, inside an expansion, declared twice | asl |
|---|---|
| `Bm:` colon label (macro ×2, m13) | **silent** |
| `Cl` colon-less column-0 label (macro ×2, m18) | **silent** |
| `Dl: dc.w` label on a data line (macro ×2, m18) | **silent** |
| `Br:` colon label (`rept 2`, m12) | **silent** |
| `El:` colon label (`irp` over two items, m18) | **silent** |
| `Fl:` colon label (`while`, two iterations, m18) | **silent** |
| `enum Be=5` (macro ×2, m18) | **silent** |
| `Al label $100` (macro ×2, m18) | `#1000` |
| `Am equ 7` (macro ×2, m13) · `Ar equ 7` (`rept 2`, m12) | `#1000` |

It is not that asl forgives the redeclaration — the name is not there at all:

```text
> > > m17.asm(11):7: error #1010: symbol undefined
> > > Xr
> > >  dc.w Xr  ; if the rept label is global this resolves
> > >       ~~
> > > m17.asm(17):7: error #1010: symbol undefined
> > > Ym
```

m19 does the same for the `enum` member and, in the same file, shows the
`label` directive's `Al` resolving to `$100` from outside — so the split is
real and not an artefact of one spelling. m18's symbol table lists `Al` and
nothing else from the expansions.

The mixed orders follow from that, and m20 measures them: a `rept` label
`Pr:` followed by a file-level `Pr equ $99` is **silent** and `dc.w Pr` reads
`$0099`; likewise a macro's `enum Qe=5` then `Qe equ $99`. The same shape with
the two GLOBAL forms (`Rl label`, `Sl equ`) is `#1000` in both cases.

### Not probed

- Whether asl's per-expansion locals collide with EACH OTHER across nested
  expansions, or what `save`/`restore` does to them.
- `NAME macro` / `NAME function` / `NAME struct` / `NAME reg` against the
  constant class (still open from the crossing parcel).
- Whether an `include` nested inside a macro localizes — sigil's
  `expansion_depth` is reset across an include, which is what the crossing
  parcel's `exitm` probe `e14` measured for the same counter, but the LABEL
  question was not probed directly.

---

## 2. Census: the population, and how it was enumerated

The enumeration is the compiler itself. A grep cannot answer this question —
the names that matter are produced by macro expansion, `\{}` name
interpolation and scope qualification, none of which exist in the text — so
each tree was assembled with the branch-point binary and with the new one and
the DIAGNOSTIC STREAMS DIFFED.

| tree | how | result |
|---|---|---|
| aeon, 4 ROM shapes | `sigil build --aeon $AEON_DIR` per shape, `AEON_DIR=.aeon-verify-483` | all four build, **CRC32+size unchanged** |
| aeon, 7 shipped shapes | `corpus_builds` under `SIGIL_STRICT_GATE=1` | see the landing run |
| s1disasm | `sigil sonic.asm`, old binary vs new | 57 diagnostics each, **diff empty** |
| s2disasm | `sigil s2.asm`, old binary vs new | 5,761 diagnostics each, **diff empty** |

**Positive control**, because a zero from a rule that never ran looks exactly
like a zero from a clean tree. The verify tree was copied, one duplicated `equ`
was injected into `games/sonic4/game_root.asm` **with the two values EQUAL**
(so a refusal cannot be a value-change check firing), and the same command run:

```text
=== injected, shown on disk:
50:SigilDupCtl:  equ 7
51:SigilDupCtl:  equ 7
=== build sonic4 plain against the MUTATED tree (MUST be red):
error: native build (sonic4 plain): assemble (native AS side, sonic4): 1 diagnostics;
  first: … message: "symbol double defined: `SigilDupCtl`" …
BUILD_EXIT=1
=== and the SAME command against the UNMUTATED tree (must be green):
built: sonic4 plain native ROM — crc=1c09fbfc len=819131
BUILD_EXIT=0
```

**The s2 number the census actually produced, before the narrowing.** The first
implementation was the obvious one — any second constant declaration is
`#1000` — and it refused **97 sites in s2disasm**, in three symbols:

| symbol | site | shape |
|---|---|---|
| `start` ×36 | `s2.macrosetup.asm(134)`, `s2.macros.asm(253)` | colon label in a twice-expanded macro |
| `end` ×3 | `s2.macrosetup.asm(136)` | same |
| `__LABEL__Plc` ×58 | `s2.asm(88690)` (`plrlistheader`) | same |

Every one of them is the cell asl exempts. s1disasm and aeon were at zero under
both shapes of the rule, so **s2 is the only tree that measured the
difference** — and it measured it decisively. That is what the rule is narrowed
to, and the narrowing is not a concession: it is what asl does.

---

## 3. What changed

`crates/sigil-frontend-as/src/eval.rs`:

- `Asm::declare_class` gains the `Some(SymClass::Const)` arm — asl's own
  `symbol double defined` wording. `Var`-over-`Var` stays legal.
- `Asm::declare_expansion_local_const` — the narrowing, one place, wired at the
  two sites whose form asl localizes: `define_label` (all three PC-label
  spellings reach it) and `enum_members`. Inside an expansion it records
  nothing and refuses nothing.
- `directive_equate`, `directive_set` and `directive_label` are unchanged and
  keep calling `declare_class` directly.

**It records nothing**, not merely "refuses nothing", and that half is what
carries m20's mixed orders: recording the class and only skipping the refusal
would make a file-level `Pr equ $99` after a `rept`'s `Pr:` a redefinition of a
symbol asl says does not exist.

**Recovery.** `directive_equate` already returns before binding when
`declare_class` refuses, so the first value survives — which is what asl's
symbol table shows (`Bi : 7` after a refused `Bi equ 9`, m16; `Bp : 1000` after
a refused `Bp:`, m15). The label and enum sites stay report-only for the reason
the crossing parcel gave.

---

## 4. Three divergences found while measuring, none of them closed here

**(a) sigil does not localize expansion labels at all.** It binds them globally,
so `sigil-link` refuses exactly the names asl exempts — m17's `Zr` is
`symbol redefined by section` from the linker, measured identically **before**
this parcel, and m19's `dc.w Be` resolves to `$0005` where asl says `#1010`.
This parcel neither creates nor worsens that: the front-end exemption restores
the pre-parcel behaviour for that cell exactly. Closing it properly means making
such a label unreferenceable from outside its expansion, which moves symbol
resolution and wants its own parcel.

**(b) `include` is include-ONCE in sigil and every-time in asl.** m14 includes
one header twice: asl executes it twice (two `#1000`s, and `33 33 33 33 44 44`),
sigil's `visited` DAG guard drops the second include entirely and emits
`33 33 44 44`, exit 0. A silent four-vs-six-byte divergence, documented in
`directive_include`'s own doc as a deliberate DAG guard. It also means the
twice-included header — the population the `#1000` rule most plausibly has in a
real source — **cannot arise inside sigil at all**, so it is measurable only
against asl.

**(c) `local` is not a directive in this asl build.** `local Cm` inside a macro
is `#1200 unknown instruction` (m13), so the documented AS escape for
macro-local labels is not available in the reference and cannot be the
explanation for (a). sigil refuses it too, with its own wording.

---

## 5. Gates

`crates/sigil-frontend-as/tests/as_symbol_class.rs`, now 11 tests, run by
`cargo test -p sigil-frontend-as` and by the landing run. Three are new
(`a_constant_may_not_be_redefined_as_a_constant`,
`a_declaration_the_pass_never_reaches_is_not_a_redefinition`,
`an_expansion_localizes_a_pc_label_and_an_enum_member_but_not_an_equ`) and one
existing test stops being decorative:
`re_executing_a_declaration_on_a_later_pass_is_not_a_redefinition` was labelled
NOT A GATE by the crossing parcel because no mutation could turn it red under a
crossing-only rule. With `#1000` in, a class map threaded across passes makes
the second pass of every multi-pass program a wall of refusals, and the test
catches it. It is relabelled.

Acceptance and refusal are asserted as a pair in every new test, so a front end
that refuses everything fails exactly as hard as one that refuses nothing.

Four mutations, each applied to the COMMITTED baseline by a patcher that asserts
its anchor matched exactly once, each shown landed with `git diff --stat` before
the run, each restored with `git checkout HEAD --` and the restore verified
empty:

| mutation | what it removes | red |
|---|---|---|
| M1 | the `Some(SymClass::Const)` arm — no `#1000` at all | 3 of 11 |
| M2 | the narrowing — refuse EVERY second constant declaration | 1, naming the expansion test |
| M3 | the narrowing RECORDS the class instead of recording nothing | 1, on the mixed-order rows alone |
| M4 | the per-pass reset — seed each pass's class map from the last | 4, incl. the multi-pass test |

**What each MUST FAIL:** a build that silently rebinds a name asl leaves at its
first value (M1); a build that refuses the 97 s2 sites asl assembles (M2); a
build that treats an expansion-local name as a symbol that exists, so a later
file-level constant of that name is a redefinition (M3); a build that turns the
second pass of every forward-referencing program into a wall of refusals (M4).

M3 is the one worth keeping: it is a ONE-LINE mutation that leaves every other
row green, and the three mixed-order fixtures in the expansion test are the only
thing standing between the narrowing and a half-right version of it.

**What other answer could each fixture have given.** Every `#1000` shape is
written twice, once with the value CHANGED and once with it IDENTICAL, so a
front end that refused only on a changed value passes one row and fails the
other — the two are not decoration for each other. The `if 0` / `if 1` pair
distinguishes "a declaration ran" from "the name appears twice". The expansion
test's two loops are opposites: the accepting loop fails under the un-narrowed
rule, the refusing loop fails under an exemption drawn around "anything inside
an expansion", and no single wrong rule passes both. The one place this bar is
NOT met is the phase/dephase row, which can only distinguish `#1000` from
nothing — `phase` has no candidate rule of its own that would accept it; it is
in the table as the record of a probe, not as a discriminator.
