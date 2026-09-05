# A label written in a macro body, and the namespace it belongs to

The binding half of the cell the `#1000` parcel left open. That parcel shipped
`declare_expansion_local_const`, which stops an expansion's PC label and `enum`
member being recorded in the symbol CLASS map — so a second expansion
redeclaring one is not `#1000`. It said the underlying non-localization "wants
its own parcel." This is it: whether such a name becomes a resolvable symbol at
all, and if so, to whom.

Everything below is read off `asl` 1.42 Beta Bld 212 —
`s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098` — with Sonic 1's own flags minus `-E`/`-c`:

```
asl -xx -n -q -A -L -U -i . <root>.asm
```

Probes are `p1.asm`–`p11.asm` under `2026-09-05-as-macro-body-label-probes/`,
run by `all.sh` / `digest.sh` and hashed by `stability.sh`.

**Oracle stability, and a correction to how it was being measured.** All eleven
probes were run **three times** and the whole stream hashed, twice over, in two
independent batches minutes apart. Identical, all six runs each:

```
p1  12080cb158bb   p5  68d67a4d9012   p9  a51bab817f7f
p2  67f255c3782a   p6  b8fd3813a15a   p10 4e29be4b2732
p3  2c5d55726ae7   p7  9acdc359e805   p11 890e0683ea54
p4  4717a201b6e4   p8  f9e93a0acd76
```

> **⚠ EVERY HASH ABOVE IS SUPERSEDED AND NO LONGER REPRODUCES** *(2026-09-05)*.
> The de-clock parcel found a SECOND clock stamp this table's filter left
> standing — the banner's meridiem: asl prints `09/05/2026 04:12:02 AM`, and the
> old rule blanked `NN:NN:NN` while leaving ` AM`/` PM`. A batch straddling noon
> or midnight therefore false-alarms on four banner lines, **guaranteed twice a
> day**, where the duration's tick-straddle is only intermittent. Repairing it
> moves every hash. **The finding this table supports is unchanged** — the probes
> were and are stable — but the numbers are kept rather than deleted so the
> supersession is visible instead of silent. Re-taken with the repaired filter,
> `stability.sh`, three runs each, all identical:
>
> ```
> p1  0cef342db0c8   p5  05aeb381fc2b   p9  4ce3f6197f3c
> p2  a3e4c12a68ab   p6  35d332b166f9   p10 7dbc69f841f9
> p3  4c9d3434bac5   p7  511b0e7ec04a   p11 e95f1cb4ca99
> p4  9f90db17e10a   p8  ad23476fbed7
> ```

The hash is taken over the stream with THREE clock readings blanked, and the
third of them is a correction to the inherited method rather than a refinement
of it. asl stamps the wall clock into the page banner and into the `DATE`/`TIME`
builtins in the symbol table — both of which the earlier parcels' runs also
carried — and it stamps `N.NN seconds assembly time` into the trailer. That last
one reads `0.00` on most runs and `0.01` on some, at no fixed rate.

So a batch of three back-to-back runs comes out identical **by luck**, and a
batch straddling a tick reports an oracle divergence that is a stopwatch. This
parcel's first stability table was the lucky kind: nine probes, three runs, one
hash each, and the instrument had never been shown to be measuring asl. Six
byte-for-byte diffs of consecutive `p1.asm` runs are identical; eight runs
through the pipeline without the seconds rule differ on exactly that one line.
The hashes above are taken with all three blanked, which is why they do not
match the ones an earlier draft of this note carried.

The known real non-determinism — a function call in an immediate whose argument
is a register name — is in none of these probes.

**Every probe invokes its macro MORE THAN ONCE, at addresses that differ.** A
macro invoked once cannot separate "each instance owns the name" from "the last
definition wins", and a label whose address is the same under both readings emits
the same byte either way. That is the same trap the `\{expr}` radix bug lived in
for months.

---

## 1. The rule, as measured

**Every expansion INSTANCE has its own namespace for plain PC labels, and the
namespaces chain inward-to-outward.** An instance is one macro expansion or one
ITERATION of a `rept` / `irp` / `while`.

| probe | question | asl |
|---|---|---|
| `p1` | read from inside the body that writes it, BACKWARD | `$0000` then `$0004` — per instance |
| `p2` | …FORWARD (the `end-start` shape) | `$0003` then `$0005` — per instance |
| `p6` | the three PC spellings: colon, colon-less column-0, on a data line | all three, per instance |
| `p7` | `rept` / `irp` / `while` | all three, per ITERATION |
| `p4` | read from a NESTED inner expansion | resolves, per instance |
| `p3` | read from the OUTER body after the inner expansion returned | `#1010 symbol undefined` |
| `p5` | read from a DIFFERENT macro's expansion | `#1010` |
| `e12`, `m17`, `m19` | read from FILE level | `#1010`, and the name is absent from asl's symbol table |
| `p9` | a `.local` written in a macro body | resolves inside, per instance; `Sc1.dl`/`Sc2.dl` from outside are `#1010` — the expansion owns the scope, not the enclosing global label |
| `p8` | a FILE-LEVEL label read from INSIDE an expansion | resolves, both directions — the exemption is drawn around the DEFINITION site |

The VALUE-BINDING forms are the other side and were measured by the `#1000`
parcel: `equ`, `=`, `:=`, `set` and the `label` DIRECTIVE are global wherever
they are written (`m18`, `m19` — `Al label $100` inside a macro reads `$0100`
from file level while the PC label beside it does not exist).

### Cells re-derived here, and cells taken from m17/m19

Re-derived: that a macro-body PC label is absent from the global table (`p3`,
`p5`, and the file-level read confirmed by re-running `e12` and `m17` through
this branch's binary). New here: the inside-the-expansion read in both
directions (`p1`, `p2`), the two nesting directions (`p3`, `p4`), the sibling
case (`p5`), the three spellings read from inside (`p6`), the three loop drivers
read from inside (`p7`), the file-level-read-from-inside control (`p8`), and the
`.local`-inside-a-macro scope (`p9`).

Taken from `m17`/`m19` without re-measuring: that the name is absent from asl's
printed SYMBOL TABLE (as opposed to merely unresolvable), and the `enum` member
row.

### Not probed

- Whether an `include` NESTED inside a macro localizes. Sigil resets the
  namespace stack across an include, on the same evidence `expansion_depth` is
  reset on (`e14`) and on `m14` (a header included twice IS `#1000` on its
  label), but the LABEL question was not put to asl directly.
- `save`/`restore` across an expansion boundary.
- Whether asl's per-instance namespaces can collide with each other.

---

## 2. Census: the population, and how it was enumerated

A grep cannot answer this — the names come out of macro expansion, `\{}`
interpolation and scope qualification, so they are not in the source text. The
enumeration is the compiler: `SIGIL_CENSUS_EXPLABEL=1` makes a build print one
line per plain PC label defined at `expansion_depth > 0`, and per pass the count
of expansion instances that got a non-empty namespace.

| tree | labels defined inside an expansion | distinct names | claimed by the scan |
|---|---|---|---|
| aeon `sonic4` plain | **0** | 0 | — |
| aeon `sonic4` DEBUG | **0** | 0 | — |
| aeon `demo` plain | **0** | 0 | — |
| aeon `demo` DEBUG | **0** | 0 | — |
| s1disasm (`sonic.asm`) | **0** | 0 | — |
| s2disasm (`s2.asm`) | 545 | 12 | 500 yes / 45 no |

s2's twelve: `__LABEL__Plc` ×295, `start` ×185, `end` ×20 — the same three names
the `#1000` parcel's narrowing was drawn around — and
`APM_{ARZ,CNZ,CNZ2P,CPZ,DEZ,EHZ,HPZ,MTZ,OOZ}_Blocks` ×5 each.

**The 45 `scoped=no` are exactly the nine `APM_*_Blocks`, and that is the
fallback working as designed.** They come from `begin_animpat`'s
`__LABEL___Blocks:` under `{INTLABEL}`, so the name the definition carries
(`APM_EHZ_Blocks`) is not the text the body holds, and `scan_plain_labels` cannot
claim it. Those 45 keep the pre-parcel global binding. `__LABEL__Plc` is claimed
because it is NOT substituted — and `p10` puts that asymmetry to asl rather than
assuming it: in one `{INTLABEL}` body carrying both spellings, asl leaves
`__LABEL__Plc:` literal and renders `__LABEL___Blocks:` as `Aint_Blocks:`, which
is what sigil does. The substitution is not a divergence; the scan gap is, and
it is the one place sigil still binds an expansion label globally.

**AEON'S ZERO IS THE HEADLINE, AND IT IS A PROPERTY OF THE TREE, NOT OF THE
INSTRUMENT.** Aeon's AS residual is three files — `games/sonic4/game_root.asm`,
`games/demo/game_root.asm`, and the vendored `engine/debug/debugger.asm` — and
none of them writes a label in a macro body. Everything else is `.emp`.

**Positive control**, because a zero from an instrument that never fires looks
exactly like a zero from a clean tree. The verify tree was COPIED (to a path
OUTSIDE the sigil worktree — see `poscontrol.sh` on why), a macro whose body is a
single label was injected into `games/sonic4/game_root.asm`, and the same build
run:

```text
=== the mutation, shown on disk:
48:SigilExpLabelCtl macro
49:SigilCtlBody:
51:    SigilExpLabelCtl
=== census over the MUTATED tree (MUST name SigilCtlBody):
CENSUS-EXPLABEL	SigilCtlBody	depth=1	scoped=yes
CENSUS-EXPLABEL	instances-with-labels=2      (×3, one per pass)
MUT_HITS=3
=== census over the PRISTINE tree (must be zero):
REF_NAMED=0
```

**The stronger control — inject a macro-body label REFERENCED FROM OUTSIDE and
watch the aeon build refuse it by name — CANNOT BE BUILT, and that is a finding
rather than an omission.** Aeon's AS residual root is declared to emit nothing,
and any PC label in it opens a section, so the build stops at
`[layout.undeclared-alignment]` before symbol resolution is reached. Two
injections were tried (a `dc.w` read and a byte-free `equ` read) and both
produced that red. `poscontrol-refuse.sh` keeps the attempt together with the
measurement that refutes it: a BARE FILE-LEVEL label, no macro anywhere, nothing
this parcel touches, produces the IDENTICAL diagnostic.

```text
STEP 1  macro-body label + outside `equ` read
        error: native build (sonic4 plain): [layout.undeclared-alignment] 1 section(s)
STEP 2  a bare file-level label, nothing else
        error: native build (sonic4 plain): [layout.undeclared-alignment] 1 section(s)
STEP 3  unmutated
        built: sonic4 plain native ROM — crc=1c09fbfc len=819131   BUILD_EXIT=0
```

So the refusal path is unreachable in the aeon build BY CONSTRUCTION, not merely
absent from the source, and a step-1 red could never have been read as evidence
for this rule. The refusal evidence lives entirely in the fixture file and its
mutation gate.

**s1's zero carries a caveat the aeon zero does not.** s1disasm produces 57
front-end diagnostics, four of them on `MacroSetup.asm(98)` — its own macro
definitions do not all survive the parse — so some macro bodies in that tree are
never expanded. Its zero is a floor, not a count.

**The corpus diff.** Both corpora were assembled with the branch-point binary and
with this branch's, and the diagnostic STREAMS diffed:

| tree | old | new | diff |
|---|---|---|---|
| s1disasm | 57 diagnostics | 57 | **empty** |
| s2disasm | 5,761 diagnostics | 5,761 | **empty** |

And s2's zero diff is not vacuous: with the change in, s2 gives **361 expansion
instances per pass** a non-empty label namespace. The mechanism runs, on 545
definitions, and moves nothing.

s2 does not reach the linker at all — 2,610 `bad operand expression` and 2,309
`expected mnemonic, directive, or label` are the bulk of its 5,761 — so it is not
an end-to-end measurement of this change; it is a measurement that the change
adds no new front-end refusals to a tree that has 545 of the construct.

---

## 3. What changed

`crates/sigil-frontend-as/src/eval.rs`:

- `scan_plain_labels` — the twin of `scan_dot_labels` for the non-`.` half of the
  label column. Excludes the value-binding forms (`equ`/`set`/`eval`/`=`/`:=`)
  and the `label` DIRECTIVE, which m18/m19 measured as global; excludes the
  name-first declaration forms and the column-0 directives, neither of which is a
  PC label.
- `ExpansionLabelScope`, `Asm::expansion_labels`, `push_expansion_labels` /
  `pop_expansion_labels` — one namespace per live expansion INSTANCE, pushed by
  `expand_macro_inner` and by `exec_rept` / `exec_irp` / `exec_while` PER
  ITERATION (`p7`).
- `Asm::plain_label_scope` — the innermost live instance that declares a plain
  name, walking outward (`p4`) and stopping at the live stack (`p3`, `p5`).
- `Asm::sym_key` — one function for "the table key a reference builds", covering
  both halves: a `.`-local through `dot_scope`, a plain name through
  `plain_label_scope`. Wired at `qualify_expr` and at the four bare-identifier
  resolve sites (`fold`, `unresolved_names`, the `Tok::Ident` evaluator, and the
  `ifdef`-style definedness probe).
- `define_label` files a plain label under the INNERMOST instance that declares
  it, and under nothing when no instance does.
- `directive_include` saves and clears the namespace stack, beside the
  `expansion_depth` reset it already did.

`resolve_float_sym` and `resolve_str` are deliberately UNCHANGED: they read
`set`-bound float and string symbols, which are the VARIABLE class and global
inside an expansion (`m6`, `m18`), and their writer still keys them with
`qualify`.

**The fallback is what makes this safe, and it is deliberate.** A name the scan
misses — one built by `\{}` interpolation or parameter substitution, so the body
text is not the name the definition ends up with — is in no instance's set, and
BOTH the writer (`define_label`) and the reader (`plain_label_scope`) then treat
it as global, which is exactly what sigil did before. Reader and writer consult
the same set, so they cannot disagree about where a name lives; a miss costs
fidelity on that name and can never strand a definition the reader cannot find.

### What this fixed that was not the stated goal

The parcel was framed as turning names that currently RESOLVE into unresolved
ones — a refusal risk. It is that (`e12`, `m17`, `p3`, `p5`), but it is mostly
the opposite. Sigil refused **eight of the nine probes outright** before this
change, every one with `symbol redefined by section` from the linker, because a
macro invoked twice defined the same global label twice:

```text
=== p1.asm ===  error: symbol `Li` redefined by section `sec0` …
=== p2.asm ===  error: symbol `Sf` redefined by section `sec0` …
=== p6.asm ===  error: symbol `Ca` … `Cb` … `Cc` redefined by section `sec0`
=== p7.asm ===  error: symbol `Ra` … `Ia` … `Wa` redefined by section `sec0`
```

Only `p8` (file-level labels) assembled. After the change all nine match asl
byte for byte and diagnostic for diagnostic.

### Left open

**`enum` members are not localized.** `m19`'s `dc.w Be` still reads `$0005`
where asl says `#1010`. Closing it needs the scan to recover member names from
an `enum` operand list in raw body text, before substitution — the same class of
text-before-substitution reasoning the `\{expr}` radix bug lived in — and it has
no corpus population at all. The class half of that cell is already closed
(`declare_expansion_local_const` records nothing for it).

**A label whose NAME is produced by substitution or `\{}` interpolation still
binds globally.** 45 of s2's 545 sites, measured; `p10` is the probe. Closing it
means scanning the SUBSTITUTED body per expansion instance rather than the macro
body once, which trades the cache away and makes a name's namespace depend on the
call site — the exact property `scan_dot_labels`' own doc says it exists to
prevent. It wants its own decision, not a quiet extension of this one.

**One resolve site is deliberately NOT routed through `sym_key`.** `open_scope`'s
callers and the macro-body TEXT substitution at the `.`-local re-spelling path
must keep producing SPELLABLE names — the result is re-lexed as source — so an
unspellable ` exp#N.` key there would corrupt the body rather than scope it.

## 4. Is the changed code REACHED in the aeon build?

**No, and this must be said plainly: the four-shape byte identity attests
nothing about this parcel.**

`SIGIL_CENSUS_EXPLABEL=1` on the aeon `sonic4` build prints
`instances-with-labels=0` on every pass. The namespace stack is pushed and
popped — the aeon residual does expand macros — but every set is empty, so
`plain_label_scope` returns `None` for every name and `sym_key` returns every
name unchanged. The rule is executed and never engages.

What the four-shape identity DOES attest: that the *plumbing* — the extra
`sym_key` indirection on every symbol reference, the push/pop around every
expansion and loop iteration, the include save/restore — moves no byte on a real
120-module program. That is worth having and it is all it is.

What it DOES NOT attest: any part of the rule this parcel is about. The only
instruments that can see the change are `crates/sigil-frontend-as/tests/
as_macro_body_label.rs` and the s2 corpus's 361 engaged instances.

---

## 5. Landing condition

| shape | branch-point binary | this branch | tree's own ROM |
|---|---|---|---|
| `sonic4` plain | `1c09fbfc` / 819131 | `1c09fbfc` / 819131 | `1c09fbfc` / 819131 |
| `sonic4` DEBUG | `e2144057` / 840324 | `e2144057` / 840324 | `e2144057` / 840324 |
| `demo` plain | `11ebd7ab` / 96602 | `11ebd7ab` / 96602 | `11ebd7ab` / 96602 |
| `demo` DEBUG | `9b0d2ce7` / 102818 | `9b0d2ce7` / 102818 | `9b0d2ce7` / 102818 |

All four assemble, exit 0, and hold CRC32+size. Nothing was refused, nothing
moved.

---

## 6. Gates

`crates/sigil-frontend-as/tests/as_macro_body_label.rs`, ELEVEN fixtures, run by
`cargo test -p sigil-frontend-as` and by the landing run. Acceptance and refusal
are asserted in the same file, so a front end that refuses everything fails
exactly as hard as one that refuses nothing.

Five mutations, each applied to the COMMITTED baseline by a patcher that asserts
its anchor matched exactly once (`mutations.sh`), each shown landed with
`git diff --stat` and the changed lines before the run, each restored with
`git checkout HEAD --` and the restore verified empty. A mutation that fails to
apply runs the ORIGINAL file and prints ok, which is indistinguishable from a
clean restore — which is why the anchor assertion and the printed diff are not
decoration.

| mutation | what it removes | red | must fail |
|---|---|---|---|
| M1 | `plain_label_scope` always `None` — no localization at all | 5 of 11 | a build that binds a macro-body label globally, so a name asl says does not exist resolves and a macro invoked twice is a linker duplicate |
| M4 | every plain name keys under the innermost instance, declared or not | 3 | a build that makes a FILE-LEVEL label unreadable from inside a macro — nothing would assemble |
| M2 | the chain does not walk outward, innermost instance only | 1 | a build where a nested macro cannot see the label its caller's body wrote |
| M3 | `rept` gets no namespace | 1 | a build where two iterations of one loop share a label, so the first iteration reads the second's address |
| M5 | the `label` DIRECTIVE is scanned as a PC label | 1 | a build that hides a GLOBAL declaration inside the expansion that wrote it, unreadable even from the body's own next line |

**M5 WALKED STRAIGHT THROUGH THE FIRST TIME, AND THAT IS THE MOST USEFUL THING
THIS GATE DID.** It applied cleanly and the file stayed green, because the
exclusion is INERT on the read the fixture used: `directive_label` binds through
the builder rather than through `define_label`, so the name is written bare
whichever way the scan goes, and by the time the OUTSIDE read happens the
namespace stack is empty, so the reference is bare too — both halves agree by
accident. The INSIDE read is what breaks. `p11` was measured against asl
(`0100 0100 4444`) and added, and M5 is red on it.

**What other answer could each fixture have given** is written on each test in
the file, one paragraph per test, and each names a concrete alternative BYTE
STRING or refusal rather than "it could have been wrong". The four that carry
the most weight:

* `p3`/`p4` are opposites that no single wrong rule passes together — one demands
  the chain walk outward, the other demands it stop at the live stack.
* `p8` is the only fixture that fails under the
  "reference-evaluated-inside-an-expansion" mis-rule, which passes every other
  test here.
* `a_value_binding_form_in_a_macro_body_stays_global` separates "labels" from
  "the `label` directive", one word apart and on opposite sides of the rule —
  and `p11` is what makes that separation FALSIFIABLE rather than stated.
* every byte fixture invokes its macro twice at DIFFERENT addresses, so
  "resolves" and "resolves to the other expansion" are different byte strings.
  `p11` is the one exception and says why on itself.

**Where this bar is NOT met.** Nothing fixtures the other names in
`scan_plain_labels`' exclusion list (`function`, `struct`, `reg`, `xref`,
`xdef`, `public`, `shared`) or the column-0 directive list. The M5 finding says
exactly what such a fixture would have to look like — an INSIDE-the-body read —
and none of those forms has a corpus site to hang one on. They are reasoned, not
measured, and they are listed here rather than left to look like the measured
rows above them.
