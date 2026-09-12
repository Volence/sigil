# AS-MACRO-LABEL-LEAK: a label made inside a macro, used outside it

Queue row, verbatim: *"Silent: a label made inside a macro can be used outside
it here, which the old assembler forbids. A program could build here that is
wrong there. Needs a reproduction first."*

Branch `parcel/as-macro-label-leak`, base `3bc0d81a`.

## Provenance

* Oracle: `s1disasm/build_tools/Linux-x86_64/asl`, md5
  `61e672562465725a8c102288a7da9098`, selected and run through
  `asl-reference/asl_ref.sh`'s `asl_run`, flags `-xx -n -q -A -L -U -i .`
  (the corpus's own minus `-E`/`-c`). Bytes come from the `p2bin` beside it, and
  only from runs that exited 0.
* sigil: built from this worktree into its own target dir, `sigil <f> --hex`.
  Base binary for the census and aeon comparisons: a `git archive` of
  `3bc0d81a` built into a scratch target dir (it reports
  `revision-unknown`, having no `.git`; its provenance is the archive).
* Every probe is placed at `org $100` behind a `$1111` filler word, so a bound
  label reads as a non-zero address and a zero cannot pass for an answer.
* One shape per file, one suspect line per file (marked `; REF`), because an
  asl run carrying any error is not a source of values.

Probes and runners are in `2026-09-12-as-macro-label-leak/`: `gen.py` (84
shapes, `probes/`), `gen2.py` (54, `probes2/`), `gen3.py` (8, `probes3/`),
`gen4.py` (4, `probes4/`), `genx.py` (9 counter probes, `xprobes/`),
`matrix.sh` (both assemblers, one row per shape), `classify.py` (the verdict
column), `xrun.sh` (asl's nameless symbol names), `mutations.sh` (red-first).

## Stage 0: reproduced

The row has a committed source after all: `src/nameless.rs` ("What this
module deliberately does NOT model") and `2026-09-09-as-nameless-labels.md`
("Measured and deliberately NOT implemented: macro-body scoping") record the
nameless half as a measured, unmodelled divergence. Two open items in
`2026-09-05-as-macro-body-label.md` ("Left open": `enum` members; names
produced by substitution) are the same row in other spellings.

At base `3bc0d81a`:

| batch | shapes | MATCH | LEAK (asl refuses, sigil builds) | OVER-REFUSE (asl builds, sigil refuses) |
|---|---|---|---|---|
| `probes/` | 84 | 51 | 25 | 8 |
| `probes2/` | 54 | 36 | 10 | 8 |
| `probes3/` | 8 | 5 | 2 | 1 |
| `probes4/` | 4 | 4 | 0 | 0 |
| **total** | **150** | **96** | **37** | **17** |

Per-shape tables: `compact-base1.txt` .. `compact-base3.txt`. `probes4/` was
written after the fix, to pin two transparency rules the first mutation run
left green; the base binary matches asl on all four as well, so it guards
the new code rather than recording a leak.

**A correction to commit `6928125f`'s message**, which gives the base as
`MATCH 94 LEAK 35 OVER-REFUSE 17` over 146 shapes. It left out batch 3's two
leaks (en5, en6); the 146-shape base is `MATCH 92 LEAK 37 OVER-REFUSE 17`,
as the table above says. The commit is not rewritten; this is the record.

The leaks, by mechanism:

* **Nameless labels** (`+`, `-`, `/`): c01 c02 c03 c04 c05 c08 c10 c11 c12
  c14 c15 i03 in6 n02 n03 n10 n13 n20 n21. asl keeps one nameless counter per
  pass but files each definition in the namespace of the expansion instance (a
  macro expansion or one loop iteration) that is innermost when it is written.
  sigil filed every one globally.
* **A plain label whose name the body text does not spell**: g01 g02
  (`__LABEL__`), g04 g05 (`{expr}`), g06 g07 g08 s06 s07 (the name arrives as
  argument text), g14 (`ALLARGS`). The body scan reads raw text, misses the
  name, and the fallback filed it globally.
* **`enum` / `nextenum` members** written in a body or a loop: h01 h04 en5 en6.
* **A label in a file `include`d from a body**: i01 (and in6 in a loop). The
  include cleared the namespace stack.
* **A `.x` under a plain label of the body**: a10 (`Lp.x` read from outside).
* **A `{GLOBALSYMBOLS}` body's `.dl:` written twice**: gs7 (asl `#1000`).
* a12 (a `.b :=` written after the call, read as `Base.b`) is the maclocal
  parcel's known open scope-change divergence, not this row; see "Left open".

The over-refusals are mostly the other face of the same leaks: a name filed
globally collides when a second expansion writes it again (g09 g10 i02 in1 in2
in4 dx2), and the include clearing the stack also stranded a read of the
body's own label from inside the included file (in3). The rest were
`{GLOBALSYMBOLS}`, which sigil did not recognise at all (f_globalsymbols,
_dot, _forward, _lower, gs1 gs5 gs6 gs8), and a11.

### asl's nameless counters are not what sigil models, and not only in macros

`xprobes/` read asl's own symbol names (`__forwN`, `__backN`, zero-based):

```text
x1  file: +, ++, +        __forw0 = 102   __forw2 = 104   __forw1 = 106
x5  file: ++, +           __forw1 = 102   __forw0 = 104
x2  body ++ ; file +, +   __forw0 = 104   __forw1 = 106
x3  body +  ; file +, +   __forw1 = 104   __forw2 = 106
```

A `+` run of length m >= 2 defines `__forw(c + m - 1)` and does NOT advance the
forward counter; only a single `+` advances it. sigil advances by m. The two
agree on every reference the 2026-09-09 probes made (`q8` has no `+` after its
`++`), and disagree on a `+` defined after a `++` with a reference spanning
both. A separate row, recorded here and in `nameless.rs`, not changed; it is
why n21 stays divergent after the scope fix. Also measured and not modelled: a
`/` written in a macro body advances asl's forward counter and not its
backward one (x7: after body `/`, file `+` is `__forw1`, file `-` is
`__back0`).

## After the fix

At `9c84159b` (and unchanged at `a454e714`, whose only change is tests), all
four batches re-run on scratch copies of the committed probes (tables
`compact-fix1.txt` .. `compact-fix4.txt`):

| batch | shapes | MATCH | LEAK | OVER-REFUSE |
|---|---|---|---|---|
| `probes/` | 84 | 82 | 1 (a12) | 1 (a11) |
| `probes2/` | 54 | 53 | 1 (n21) | 0 |
| `probes3/` | 8 | 7 | 0 | 1 (gs8) |
| `probes4/` | 4 | 4 | 0 | 0 |
| **total** | **150** | **146** | **2** | **2** |

The four left are not this mechanism; see "Left open".

## The fix, established from code

All in `crates/sigil-frontend-as/src/eval.rs` (plus the module doc of
`src/nameless.rs`, which recorded the nameless half as unmodelled).

**One writer.** `Asm::file_in_innermost` files a name under the innermost live
expansion instance (` exp#N.name`) and records it in that instance's new
`written` set. Three definers call it: `define_label` (plain labels, whatever
spelling produced the name), `define_nameless_slot` (the slot keeps its global
NUMBER and is filed like a label) and `enum_members`. Before, `define_label`
filed a name in the instance only when `scan_plain_labels` had claimed it from
the raw body text, and `define_nameless_slot` and `enum_members` never did.
That "fallback" was the row.

**One reader.** `plain_label_scope` owns a name for an instance on any of three
pieces of evidence: the body scan claimed it (unchanged), the instance has
`written` it this pass, or the instance filed it on the PREVIOUS pass
(`prev_owned`, built per pass by `index_instance_owned` from the seed
environment's ` exp#N.` keys). The last is what serves a forward reference
inside a body to a name the scan cannot see (`s01`..`s04`, `n01`, `n05`,
`in4`). It is sound for the reason every forward reference in this front end
is: `run_impl` returns only after two consecutive passes with EQUAL
environments, and the environment carries the key each name was filed under,
so on the returned pass the previous pass's owner and this pass's owner are
the same instance.

**`include` keeps the namespace stack.** `directive_include` still zeroes
`expansion_depth` (asl refuses `exitm` in an included file, `e14`) but no
longer clears `expansion_labels`, so an included file's names land in the
instance the `include` line runs in (`i01`, `i02`, `in1`..`in6`, `en6`).
The old comment's evidence, `m14.asm`, was a FILE-LEVEL double include, where
no instance is live either way.

**`owned_by_head`.** `Lp.x` where `Lp` is owned by a live instance is filed
under that instance, in the writer (`define_label` for `.`-names) and in
`sym_key` (for both `.x` after qualification and a spelled-out `Lp.x`). The
scope `Lp:` opens stays its bare spellable name, which is what keeps `dx5`
working: a `.x` argument is pasted into the nested macro as the text `Lp.x`,
re-lexed, and keyed here. A first design made the scope itself private, and
`dx5` measured that it would have turned a shape both assemblers accept into
the known "expansion-scoped name pasted into text" refusal.

**`{GLOBALSYMBOLS}`.** `head_declares_option` (the former
`head_declares_int_label`, generalised) records it on `MacroDef`. Such an
expansion is TRANSPARENT: it pushes no namespace, swaps in no `.`-local scope,
records and restores no `outer_scope`, and hands back whatever scope its body
left (`gs5`). `dot_scope` answers a transparent top frame with the scope in
force, `real_scope` treats a stack of transparent frames as file level, and
`declare_expansion_local_const` records the class of a name written under
transparent frames alone, so a second such expansion is the front end's
`symbol double defined`, asl's `#1000` (`f_globalsymbols_twice`, `gs7`).

## Tests and their red-first evidence

`crates/sigil-frontend-as/tests/as_macro_label_leak.rs`, 13 tests. Every
fixture is a committed probe file read at test time, so the text sigil is
tested on is the text asl answered; every expected byte string is that probe's
asl `p2bin` output from an exit-0 run; every expected refusal is a probe asl
refused.

| test | carries |
|---|---|
| `a_nameless_label_written_in_a_macro_body_is_out_of_reach_after_the_expansion` | c01 c04 c08 c15 i03 n03 n10 n20 |
| `a_nameless_reference_made_before_a_call_cannot_reach_a_definition_in_the_body` | c02 c03 c05 c14 n02 n13; n22 n23 accepted |
| `a_macro_body_reaches_nameless_definitions_outside_it_in_both_directions` | c06 c07 c09 n01 n04 n05 n11 n12 n16 n17 n18 |
| `a_nameless_label_in_a_loop_iteration_belongs_to_that_iteration` | c10 c11 c12; n06 n07 n08 n09 n14 n15 |
| `a_label_whose_name_the_body_does_not_spell_stays_in_its_expansion` | g01 g02 g04 g05 g06 g07 g08 g14 s06 s07 |
| `a_substituted_label_reads_back_inside_its_own_body_before_and_after_it` | s01..s05 g11 g12 g13 |
| `two_expansions_may_write_the_same_substituted_label` | g09 g10 |
| `an_enum_member_written_in_a_body_or_a_loop_stays_there` | h01 h04 en5 en6; en1..en4 |
| `a_label_in_a_file_included_from_a_body_belongs_to_that_expansion` | i01 in6; i02 in1..in5 |
| `a_globalsymbols_expansion_opens_no_namespace_of_its_own` | f_globalsymbols* c13 n19 gs1..gs7 gs9 gs10 gs11 |
| `a_globalsymbols_expansion_nested_with_a_plain_one_is_transparent_there_too` | gs12..gs15 |
| `a_dot_label_under_a_body_label_stays_in_the_expansion` | a10; dx1..dx5 |
| `the_shapes_asl_and_sigil_already_agreed_on_stay_agreed` | the controls: value bindings, definedness, 36 prior refusals |

`mutations.sh` removes one mechanism at a time from the COMMITTED baseline
(each anchor asserted to match exactly once, the change shown landed with
`git diff` before the run, the WHOLE `sigil-frontend-as` suite run, restored
from HEAD and verified clean). Run 1 at `9c84159b` (17 mutations), run 2 at
`a454e714` (`ONLY="M12 M14"`). "red" is the count of failing tests out of
847 (run 1) / 848 (run 2).

| mutation | removes | red | failing tests |
|---|---|---|---|
| M0 | the whole fix (eval.rs, nameless.rs at base) | 9 | every new test except the two acceptance-only ones and the controls |
| M1 | `file_in_innermost` files nothing | 19 | 11 new + 8 in `as_macro_body_label.rs` (the per-instance cells the 2026-09-05 parcel pinned) |
| M2 | nameless slots filed globally | 3 | the three nameless-refusal tests |
| M3 | a plain label filed only if the scan claimed it (the old rule) | 4 | substituted names, twice-substituted, include, GLOBALSYMBOLS |
| M4 | enum members global | 1 | enum |
| M5 | include clears the namespace stack | 3 | include, nameless (i03), enum (en6) |
| M6 | no previous-pass owner index | 4 | substituted forward reads, nameless forward reads, include (in4), loops |
| M7 | the reader ignores `written` | **0** | by construction, see below |
| M6+M7 | both | 6 | M6's four + GLOBALSYMBOLS + enum |
| M8 | `owned_by_head` disabled | 1 | the `.x` under a body label |
| M9 | `{GLOBALSYMBOLS}` not recognised | 1 | GLOBALSYMBOLS |
| M10 | class recording back to the depth rule | 1 | GLOBALSYMBOLS (the `double defined` fragment) |
| M11 | a transparent frame restores the scope | 1 | GLOBALSYMBOLS (gs5) |
| M12 | `dot_scope`'s transparent arm | **0** in run 1, **1** in run 2 | nested GLOBALSYMBOLS (gs13) |
| M13 | `real_scope` ignores transparency | 1 | GLOBALSYMBOLS |
| M14 | `outermost` ignores transparency | **0** in run 1, **1** in run 2 | nested GLOBALSYMBOLS (gs14, gs15) |
| M15 | a transparent frame still swaps in a private scope | 1 | GLOBALSYMBOLS |

**Three mutations walked through the first run, and that is the useful
output.** M12 and M14 were equivalent to their replacements at file level
and differ only when a `{GLOBALSYMBOLS}` expansion and a plain one NEST;
nothing nested them. `probes4/` (gs12..gs15) put the nesting to asl, all four
match the fix, and both mutations are red on the test that carries them.

**M7 stays green, and the reason is stated rather than hidden.** On every pass
`run_impl` can return, the previous pass filed the same names under the same
keys (convergence demands equal environments, and keys are part of the
environment), so the previous-pass index answers every question `written`
answers. M6+M7 is the control that the pair carries those cells. `written` is
kept because it makes a BACKWARD reference right on the pass that makes it,
without leaning on convergence; a test that could see it would have to observe
a non-returned pass, which no public instrument does.

The M10 cell exists because the first cut of the test accepted the fragment
`defined`, which the LINKER's `redefined by section` also satisfies; it was
tightened to `double defined` (the front end's wording, asl's `#1000`) before
the mutation run, so M10 is red on the stage, not only on the verdict.

## Corpus census, by compiler

`census.sh` assembles each disassembly root with one binary and keeps the
whole diagnostic stream and, separately, the `SIGIL_CENSUS_EXPLABEL` lines.
Base binary: a `git archive` of `3bc0d81a` built into its own target dir. Fixed
binary: `9c84159b`. Roots `s1disasm/sonic.asm`, `s2disasm/s2.asm`,
`skdisasm/sonic3k.asm`, each run from its own directory.

| tree | exit / stage, both binaries | diagnostic lines base / fix | differing lines |
|---|---|---|---|
| s1 | 1, stopped at layout (Z80 driver placement, needs `-z`) | 1 / 1 | **0** |
| s2 | 1, stopped at the front end | 79 / 79 | **0** |
| sk | 1, stopped at the front end | 137 / 137 | **0** |

No site newly refused, none newly accepted, in any tree. The diff instrument
is not blind: the same `diff` over two streams known to differ (s2 against sk)
reports 218 lines.

The zero is not vacuous for s2 and sk. The census shows the changed writer
engaging on real code: 9 names in s2 (`APM_{ARZ,CNZ,CNZ2P,CPZ,DEZ,EHZ,HPZ,MTZ,OOZ}_Blocks`,
the 45 `scoped=no` sites of the 2026-09-05 note, now filed in their instance)
and 30 in sk (`Debug_*_End`) are labels the body text does not spell, 36 and
90 definition events over the passes. Base filed every one of them globally.
The census streams differ ONLY in the per-pass reachability witness
(`instances-with-labels` 365 to 374 in s2, 644 to 674 in sk), which counts
exactly those newly filed names.

Why no reference to them moved: neither tree spells any of them literally
(`Debug_SSZ_End` 0 occurrences in skdisasm against 4 for `Debug_SSZ`;
`APM_EHZ_Blocks` 0 in s2disasm). They exist only as `__LABEL___End` /
`__LABEL___Blocks` inside the macro that writes them (`sonic3k.macros.asm:202`
reads the `:204` definition of the same `dbglistinclude` body; `s2.asm:86229`
reads `:86232` in the same body). `watertransheader` and `plrlistheader` read
`__LABEL___End` without defining it, and their `_End` labels are written at
file level, where they stay global.

What the census cannot see: s2 and sk stop at the front end on both binaries,
so a LINK-level refusal of an outside reference to a re-filed name would not
appear in these streams. For s2 that is covered by the whole-ROM byte gate
(green). For sk it is covered only by the literal-spelling argument above.

s1 writes no label in a macro body at all (`instances-with-labels=0` on every
pass), so its zero is a floor, as the 2026-09-05 note already said.

## aeon

Own reference tree `/home/volence/sonic_hacks/.aeon-ref-as-macro-label-leak`,
provisioned by `scripts/provision-aeon-ref.sh` at aeon `ec640bcf` (golden
control required). The provisioning built its assembler from this worktree,
`sigil 0.1.0 (9c84159b)`, "clean at capture", closure equal to HEAD, and its
rebuild control printed `s4.bin b09ccd65/820229 MATCHES THE GOLDEN` and
`s4.debug.bin 1b7fe316/846529 MATCHES THE GOLDEN`. `repin --check` (run at
`9c84159b`, tree clean) ends `pins.rs unchanged`.

All four shapes built into scratch (never over the tree's own ROMs), `sigil
build --aeon . --native --game <g> [--debug]`, the tree clean before and after:

| shape | base `3bc0d81a` | fix `9c84159b` |
|---|---|---|
| sonic4 | `b09ccd65` / 820229 | `b09ccd65` / 820229 |
| sonic4 debug | `1b7fe316` / 846529 | `1b7fe316` / 846529 |
| demo | `0ad17404` / 96863 | `0ad17404` / 96863 |
| demo debug | `2565ece2` / 103185 | `2565ece2` / 103185 |

**The byte identity attests plumbing only, and the census says why.** The
sonic4 builds with `SIGIL_CENSUS_EXPLABEL=1` print `instances-with-labels=0`
on every one of 51 census lines, plain and debug: in aeon's AS residual no
name is ever filed in an instance, and no body scan claims one. Its three AS
files carry no `{GLOBALSYMBOLS}`, no `enum`, no column-1 nameless label, and
the one `include` in each `game_root.asm` sits at file level inside
`ifdef __MDDBG__`, a conditional, not an instance. The changed writer is
executed on no aeon line.

## Workspace suite under the strict gate

Two runs, both detached, both at HEAD `a454e714` with the tree clean, both
against this lane's aeon tree (`ec640bcf`, clean). Completeness is the count
of test binaries launched against the count reported: 459 `Running` plus 13
`Doc-tests` against 472 `test result:` lines, in both runs.

| run | environment | passed | failed | ignored | exit |
|---|---|---|---|---|---|
| 1 | the brief's command: `SIGIL_STRICT_GATE=1 AEON_DIR=...`, default target dir | 5272 | 2 | 2 | 101 |
| 2 | the same plus `ORACLE_DIR=/home/volence/sonic_hacks/oracle-old` (`1eb09a98`, clean) and a target dir outside the checkout's `target/` | **5274** | **0** | 2 | **0** |

Run 1's two failures are the strict gate refusing the environment, and each
says so by name: `refreeze_tells_its_children_which_build_directory_to_use`
("this test binary was built into the checkout's DEFAULT target/ ... A strict
run builds into a named CARGO_TARGET_DIR outside the checkout's target/") and
`oracle_loadfromaslisting_resolves_emit_listing` ("NO REFERENCE TREE IS
NAMED ... DECLINED to use /home/volence/sonic_hacks/oracle-old, which step 3
derived"). Both pass in run 2, and 5272 + 2 = 5274 reconciles.

The two ignored tests carry their own reasons and predate this parcel:
`secondary_pin_classes_match_the_hand_typed_baseline` (retired by Wave-B B-0)
and `sigil_diff_reports_byte_identity` (reads the aeon source tree, opt-in).

All 13 tests of `as_macro_label_leak.rs` and both whole-ROM gates,
`sonic_1_builds_to_the_reference_rom_byte_for_byte` and
`sonic_2_builds_to_the_reference_rom_byte_for_byte`, appear in run 2's log by
exact name as `... ok`. Under `SIGIL_STRICT_GATE` a missing suite root is an
assertion failure (`suite_root_absent`), so neither gate can have passed by
skipping.

Also run: the `sigil-frontend-as` suite alone at the fix (847 passed, 0
failed, 0 ignored, 75 `Running` + 1 `Doc-tests` = 76 result lines), and
`cargo clippy --release -p sigil-frontend-as --all-targets -- -D warnings`
(exit 0, zero diagnostics, after one `manual_contains` fix).

## Left open

* **a11 / a12**, a plain label written in a macro body changes the CALLER's
  `.`-local scope for everything after the expansion (asl: `.b := 2` after the
  call is `Inner.b`; sigil restores the caller's scope, so it accepts `Base.b`
  and refuses `Inner.b`). This is the 2026-09-03 maclocal note's second known
  limitation, left alone there deliberately; a12 is a silent over-acceptance,
  but of a name written OUTSIDE the macro, so not this row. Worth its own row.
  (The `{GLOBALSYMBOLS}` twin, `gs5`, IS fixed, because a transparent frame has
  no scope to restore.)
* **n21**, asl's `+`-run definition counter (section above). A separate row.
* **gs8**, a `{GLOBALSYMBOLS}` macro called from a PLAIN one writes a `.dl:`
  that the plain body reads back in asl (`$0104`); sigil files it in the plain
  expansion's private scope but the plain body's reader, whose static
  `dot_labels` never saw it, looks in the caller's. Loud (refused), no corpus
  or aeon use of `{GLOBALSYMBOLS}` at all (`/usr/bin/grep` over s1disasm,
  s2disasm, skdisasm and `git grep` over aeon's `.asm`/`.inc`/`.emp`: zero,
  with `INTLABEL` found in all three corpora as the positive control). Closing
  it wants the dot-label reader to take `written`-style evidence too.
* **`written` is outcome-redundant with `prev_owned` on every returned pass**
  (see the mutation table): convergence guarantees the previous pass filed the
  same names under the same keys. It is kept because it makes every BACKWARD
  reference right on the pass that makes it, without leaning on convergence,
  and costs one set insert per definition. Stated rather than left looking
  like an untested mechanism.
* Not probed: `save`/`restore` across an expansion boundary; `phase` inside a
  body; `struct` declared in a body.

## Things in the brief that turned out wrong

1. **"This row has NO committed source finding."** It has two.
   `src/nameless.rs` (the "What this module deliberately does NOT model"
   section) and `2026-09-09-as-nameless-labels.md` ("Measured and deliberately
   NOT implemented: macro-body scoping") record the nameless half as measured
   and unmodelled, with the `mb.asm` listing; `2026-09-05-as-macro-body-label.md`
   ("Left open") records the `enum` and substituted-name halves. The row's
   wording ("a label made inside a macro can be used outside it") matches the
   nameless note's almost exactly.
2. **The two measurements "point the OTHER way".** They do not conflict with
   the row; they cover different shapes. `3d9322c5` and the maclocal lane entry
   measured names the body text SPELLS (a literal `lbl12:`, a literal `.name:`),
   which the body scan claims; every leak here is a name the scan cannot claim
   (nameless slots, substituted names, enum members, included files).
3. **The regression hypotheses** (nameless labels at `6b26f7f7`, argument
   pasting at `f8052111`). The nameless leak is not a regression: it dates
   from the feature itself, which recorded it as known. For the argument-text
   leak (`g06`..`g08`) no commit was measured, because the base here postdates
   both; what the code shows is that the cause does not involve argument
   pasting at all. `scan_plain_labels` reads the macro body as captured, where
   the parameter is still the word `nm`, so the definition `Foo:` was never in
   the claimed set however the argument text reached it.
4. **The strict command as given cannot come out green here.** Run exactly as
   written (`SIGIL_STRICT_GATE=1 AEON_DIR=... cargo test --release --workspace
   --no-fail-fast`, default target dir), it reports 5272 passed, 2 failed:
   `refreeze_tells_its_children_which_build_directory_to_use` refuses a test
   binary built into the checkout's default `target/` under the strict gate,
   and `oracle_loadfromaslisting_resolves_emit_listing` refuses to use an
   oracle-old tree nobody named. Both are the gate refusing the ENVIRONMENT,
   by design. The second run names `ORACLE_DIR` and builds outside the
   checkout's `target/`, and both pass.
