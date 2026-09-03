# A `.`-local's scope inside a macro is decided by how it is written

2026-09-03 · branch `parcel/as-maclocal-scope` · sigil master base `7f78bb53`

Sixth in the AS-frontend arc for the public Sonic 2 disassembly
(`/home/volence/sonic_hacks/s2disasm`, git `e45ebf3`). The fifth parcel landed
`shift` and left one cause standing behind `zoneTableEntry`: `.cur_zone_str` is
`set` in `zoneOrderedTable`'s expansion and read in `zoneTableEntry`'s.

Ground truth throughout is an `asl -L` listing (AS V1.42 Beta Bld 212,
`s2disasm/build_tools/Linux-x86_64/asl`), invoked with the Sonic 2 build's own
flags minus the two that only redirect output: `asl -xx -n -q -A -L -U -i .`
(the build adds `-E` to send errors to a log and `-c` for a shared header;
`common.lua:773`). **`-U` forces case-sensitivity** and every row below carries
it. Every rule is stated with the row that establishes it, and every expected
value in the twelve tests is such a row.

## The split is syntactic, not by value kind

The standing claim was "labels per expansion, `set` variables to the caller."
The first half is right, the second half is right, and the reason is neither:
AS splits on the FORM the definition is written in.

A **plain `name:` label** in a macro body is private to the expansion. Two
expansions under one global label each bind their own, and the name never enters
the symbol table:

```text
  10/    1000 : (MACRO)              	mlab
  10/    1000 : 6702                        beq.s   .done
  10/    1002 : 4E71                        nop
  10/    1004 :                     .done:
  11/    1004 : (MACRO)              	mlab
  11/    1004 : 6702                        beq.s   .done
  11/    1006 : 4E71                        nop
  11/    1008 :                     .done:
 > > > b1.asm(12):7: error #1010: symbol undefined
 > > > .done
 > > >  dc.w .done-Base
```

**Every value-binding form** lands in the CALLER's scope and IS a symbol there.
`equ`, `=`, `set`, `:=` and the `label` directive all behave alike — the last of
those produces an address, so this is not "labels versus constants":

```text
  11/    1000 : (MACRO)              	mkinds
  11/    1000 : =$3                  .eq     equ     3
  11/    1000 : =$1000               .lb     label   *
  11/    1000 : 4E71                        nop
  11/    1002 :                     .pl:
  12/    1002 : 03                  	dc.b	.eq
  13/    1003 : 1000                	dc.w	.lb

   Base.eq :                        3 - |  Base.lb :                     1000 C |
```

`Base.eq` and `Base.lb` are in the table; `.pl` is not. Same file, same
expansion, three definitions, two scopes. `=` and `set` join `:=` on the caller
side (`m1.asm`: `Base.asn : 5`, `Base.eqs : 3`, `.sets` read back as `04`).

### Nesting is transparent to a value binding

An inner macro's `:=` does not stop at the enclosing expansion. It reaches the
nearest scope that is not an expansion at all, in one step:

```text
  12/    1000 : (MACRO)              	outer
  12/    1000 :  (MACRO-2)                   inner
  12/    1000 : =$5                  .v      :=      5
  12/    1000 : 05                          dc.b    .v
  13/    1001 : 05                  	dc.b	.v

   Base.v :                         5 - |
```

This is what makes the corpus's recursion work: `zoneTableEntry` calls itself,
so each recursion's immediate caller is the previous expansion, and the counter
still has to land outside the whole nest.

```text
  15/       0 : (MACRO)              	zte	$11,$22,$33
  15/       0 : 00                              dc.b        .cnt
  15/       2 : =$1                  .cnt    :=      .cnt+1
  15/       2 :  (MACRO-2)                       zte $22,$33
  15/       2 : 01                              dc.b        .cnt
  15/       4 : =$2                  .cnt    :=      .cnt+1
  15/       4 :   (MACRO-3)                      zte $33
  15/       4 : 02                              dc.b        .cnt
  15/       6 : =$3                  .cnt    :=      .cnt+1
  16/       6 : 03                  	dc.b	.cnt
```

`shift` does not interact with any of this. Shift state is per-frame argument
bookkeeping; the scope question is answered by the name, not by the frame.

### `label` opens a scope, and it opens the CALLER's

`Second label *` makes subsequent `.`-locals `Second.…`, and one written inside
an expansion is still in force after the expansion returns (`j2.asm`, the
`{INTLABEL}` shape the corpus uses: `Table label *` inside `zot`, then
`Table.cnt` read at top level as `06`). A plain `Third:` label ends the previous
scope, so `.a` after it is `error #1010`.

## The half nobody had probed: a reference reaching OUT

A macro body that does NOT define the name reaches the caller's label, and this
is load-bearing rather than incidental:

```text
   9/    1000 : (MACRO)              	mref
   9/    1000 : 6704                        beq.s   .tgt
   9/    1002 : 4E71                        nop
  10/    1004 : 4E71                	nop
  11/    1006 :                     .tgt:

   Base.tgt :                    1006 C |
```

The July note listed exactly this as a known limitation of the per-expansion
scope ("a macro body that references a caller-scope `.`-local WITHOUT it being
passed as an argument … would diverge"). It is a real divergence, and it is
the reason the fix cannot be "route `set` to the caller and leave references
alone."

## Why the design is a namespace and not a fall-back chain

The obvious rule — look in the expansion, fall back to the caller — has a
silent-wrong mode: a macro's own forward branch to `.done` reaches a caller's
`.done` whenever the expansion has not defined its own yet.

**asl does exactly that, and is not self-consistent about it.** Same macro, same
caller, the only difference being an unrelated forward reference that costs a
second pass:

```text
  ; d1.asm — 1 pass
  12/    1000 : (MACRO)              	mown
  12/    1000 : 67FE                        beq.s   .tgt      ← Base.tgt
  12/    1006 :                     .tgt:

  ; d2.asm — 2 passes, identical construction plus `dc.w Later`
  12/       0 : (MACRO)              	mown
  12/       0 : 6704                        beq.s   .tgt      ← its own
  12/       6 :                     .tgt:
```

`67FE` is two bytes backward; `6704` is four forward. The written program did
not change.

So "match asl" does not name a behaviour here, and the choice is ours. The rule
implemented is:

> A `.`-local that the innermost expansion's body declares as a PLAIN LABEL
> belongs to that expansion, for the whole expansion. Every other `.`-local in
> that body belongs to the caller's nearest non-expansion scope. Value-binding
> definitions always write the caller's.

The set is computed from the body ONCE, before the body runs
(`scan_dot_labels`), so a name's scope is a property of the MACRO, not of where
in the body a reference sits or of which pass is running. **There is no lookup
order, so there is nothing for a race to change**: a body that declares `.done`
owns `.done` everywhere inside itself, and a body that does not, never owns it.
The fall-through is not merely unobserved — the state in which it happens does
not exist.

This reproduces every probe row above, and every row where asl is order-stable.
It diverges in exactly the shapes where asl disagrees with itself.

### What it costs: the untaken arm

The one reachable shape where this REFUSES what asl assembles is a label
declared in a conditional arm the call did not take. asl reaches the caller's,
silently, and which label a written branch means then depends on an argument —
`mcond 0` and `mcond 1`, both two-pass:

```text
  14/       2 :                     .done:
  15/       2 : (MACRO)              	mcond	0
  15/       2 : 67FE                        beq.s   .done      ← Base.done
  15/       6 : =>FALSE                      if 0

  14/       2 :                     .done:
  15/       2 : (MACRO)              	mcond	1
  15/       2 : 6704                        beq.s   .done      ← its own
  15/       8 :                     .done:
```

One written branch, two destinations, no diagnostic. Here the name is the
macro's, so the `mcond 0` reference stays in the expansion and dangles loudly.
Nothing in the Sonic 2 disassembly or in aeon writes this shape (the corpus
decomposition below is the measurement, not an assertion). Should a real
consumer need asl's answer, the change is to narrow what the body-label scan
claims — never to add a fall-back, which brings the order-dependence back with
it.

### Why the scan is exact for both trees

`scan_dot_labels` reads the raw body, so a label whose NAME came from a
parameter would be scanned under its unsubstituted spelling. Across every macro
body in `s2disasm/**/*.asm` and `.aeon-as-fold/**/*.{asm,inc}` there are 513
column-0 dotted plain labels (390 in `s2.asm`, 117 in `s2.sounddriver.asm`, 6 in
`s2.macros.asm`, none in aeon — aeon's macros are `.emp` now) and **not one of
them contains a parameter name of its own macro**.

## The adjacent finding, folded in

The fifth parcel booked that surplus positional arguments skip
`bind_macro_arg`. The reference-side rule above hides that for the ordinary
case — an unqualified `.val` arriving through `ALLARGS` is resolved by the
callee against the caller's real scope, which is the same scope — so it does
NOT fall out correctly on its own. The shape that separates them is a callee
whose own body declares the same name:

```text
  12/       2 :                     .val:
  14/       4 : (MACRO)              	sp	1,.val
  14/       4 :                             shift
  14/       4 : 0002                        dc.w    .val
  14/       6 : 4E71                        nop
  14/       8 :                     .val:
```

`0002` is the caller's label at two, not the callee's at eight. The argument
means what the caller wrote, so it is bound at the call, surplus or not.

Reaching that surfaced a **panic**. A `.`-local argument passed from INSIDE an
expansion is qualified against the expansion's deliberately-unspellable scope
name (a leading space and a `#`, so no source label can alias it), and if the
callee pastes it back through `ALLARGS` the line reads `dc.w  macro#1.val` —
whose second token is the `macro` keyword. `capture_macro` then took it as a
definition with no `endm` after it and sliced a body backwards. It now says so.
asl has no such limit (`q1.asm`: `0002`, the caller expansion's private label),
so the loud refusal is a limitation, not a rule; see BLOCKED below.

## Corpus movement

Same command both times, `sigil s2.asm` from `s2disasm`. The `before` column is
master `7f78bb53` built in its own worktree and measured with the same script,
not a quoted figure. **Every older figure in this lane's records for this row —
32,385 — predates `shift` and is wrong by ~25×.**

| | before | after |
|---|---|---|
| diagnostics | 22,328 | **21,003** |
| distinct unresolved symbols, all files | 298 | 293 |
| distinct source lines carrying a diagnostic | 8,696 | 8,683 |

The fall is 1,325 and it decomposes exactly:

| class | before | after |
|---|---|---|
| `org needs a constant expression` (`s2.macros.asm`) | 669 | **0** |
| `` `{…}` in a symbol name did not resolve `` (`s2.macros.asm`) | 646 | **0** |
| `unresolved symbol … in operand` (`s2.asm`) | 3,179 | 3,169 |
| **every other class** (73 of them) | — | **unchanged, to the count** |

**No new class appeared and no class rose.** Per line: `s2.macros.asm:191`
(`zoneTableEntry`'s `!org`) 1,224 → 0, `:208` (`zoneTableBinEntry`) 68 → 0,
`:227` (`zoneTableEnd`) 23 → 0. That is 1,315; the remaining 10 are the
`Hud_*.loop_counter` sites.

The unresolved-symbol SETS were compared sorted, not just their sizes: **zero
newly unresolved**, five newly resolved, and they are one construct —

```
hud_counter macro {INTLABEL},number
__LABEL__ label *
.loop_counter = int(log(number))
```

`.loop_counter` is `=`-bound inside the expansion and read from outside as
`Hud_1.loop_counter`, `Hud_10.loop_counter`, … (`s2.asm:87677`, read at eleven
sites). It is the same rule as `zoneTableEntry`'s, in the `=` spelling.

`s2.macros.asm:157` (492, `offsetTableEntry`) is untouched: it needs
`{INTLABEL}`/`__LABEL__`, which sigil does not implement.

## What survived from the July probes

`docs/superpowers/notes/2026-07-04-m1d-t4-macro-local-scope-probes.md` probed
this before the oracle was known to be here. Re-tested:

* **P1/P3/P4 (plain labels per expansion, invisible to the caller) — survive
  verbatim.** They are the first table row above.
* **Its stated rule generalized to "a `.`-local defined inside a macro
  expansion" — REFUTED.** All four of its probes used plain labels; none used a
  value-binding form, and the two go opposite ways.
* **Its first known limitation (a body referencing a caller-scope `.`-local not
  passed as an argument) — CONFIRMED real, and closed here.**
* **Its second (a body defining a NON-dotted global label meant to become the
  outer scope) — CONFIRMED real, still open.** asl: a plain `Inner:` inside a
  macro body changes the CALLER's `.`-local scope for everything after the
  expansion, though `Inner` itself never enters the symbol table
  (`l1.asm`: `.b := 2` after the call becomes `Inner.b`, `Base.b` is
  `error #1010`, and `.a` set before the call is no longer reachable as `.a`).
  sigil restores the caller's scope. Left alone deliberately — it is the same
  territory as the plain-label-export row the fifth parcel booked, and moving
  it moves whatever depends on it.

## Verification

* Twelve tests, every expected value a listing row quoted at the test.
* Each proven red by a mutation shown applied from disk (`git diff` quoting the
  changed line) and restored from the committed baseline between runs. Eight
  mutations, each redding a named set: routing `set` to the expansion scope reds
  five; routing `equ`/`=` there reds exactly the all-forms test; resolving every
  reference in the expansion reds four; resolving every reference in the caller
  reds the three label tests; **replacing the namespace with a fall-back chain
  reds exactly one** — the untaken-arm gate, and nothing else, which is the
  finding that gate exists for; dropping the surplus binding reds two; letting a
  nested expansion reset the real scope reds the two nesting tests; dropping the
  unclosed-definition refusal reds exactly the panic test.
* The fall-back mutation was GREEN against the first nine tests. sigil's env
  survives a pass, so its second pass hands a fall-back the expansion's own
  definition and the two rules agree on every unconditional shape. The
  untaken-arm gate was written because of that, not before it.
* aeon four shapes rebuilt from `/home/volence/sonic_hacks/.aeon-as-fold`
  (detached at aeon `4f5ad5a1`), all four artifacts DELETED first, one shape per
  invocation, all exit 0 under `SIGIL_VERSION_STRICT=1`, every log stamped
  `Assembler: sigil 48f268da0a0d (clean at capture)`. CRC32+size unchanged on
  every shape: s4 `14ee2440`/719700, s4.debug `142294b3`/737683, demo
  `0c456778`/96474, demo.debug `2e603d53`/101339. mtimes moved on all four. The
  one commit after that build changes 58 lines, all of them `///` doc comments
  (`git diff 48f268da c52c83a7 | grep -v '^[+-]\s*///'` over the `+`/`-` lines
  is empty).

## BLOCKED

### An expansion-scoped name cannot be pasted into text

Reachable, refused loudly, not implemented. A `.`-local argument passed from
inside an expansion carries the synthetic scope name, and any callee that puts
it back into a line through `ALLARGS` produces text that cannot re-lex. The
scope name is unspellable ON PURPOSE — that is what makes a user label unable to
alias it — so this is a genuine trade, not an oversight, and closing it means
choosing an identifier-legal reserved prefix and accepting that a source label
could in principle collide. No corpus or aeon call does this. Sizing: one
parcel, byte-moving for anything that dumps section symbol names.

### `{INTLABEL}` / `__LABEL__` / the `label` directive

Unimplemented, and the largest remaining block in `s2.macros.asm`
(`:157`, 492 diagnostics; `:162`, 49; `:168`, 23). `label` is now known to open
a caller scope (`i1.asm`), which is the part of it this parcel needed to know
and did not have to implement, because the corpus's call sites carry an ordinary
column-0 label on the macro-call line and sigil opens the scope from that.
Sizing: one parcel with `__LABEL__`.
