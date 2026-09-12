# AS-MACRO-DOT-SCOPE-AFTER-CALL: a plain label in a macro body is the caller's `.`-local scope after the call

Queue row, as booked by the AS-MACRO-LABEL-LEAK landing (lane log, 2026-09-12T01:26Z):
*"AS-MACRO-DOT-SCOPE-AFTER-CALL (a11/a12, silent)."* Its two committed sources:

* `2026-09-12-as-macro-label-leak.md`, "Left open", first bullet: probes `a11`
  (asl builds `Inner.b`, sigil refused) and `a12` (asl refuses `Base.b`, sigil
  built).
* `2026-09-03-as-maclocal-scope.md`, "What survived from the July probes", the
  second limitation: *"a plain `Inner:` inside a macro body changes the CALLER's
  `.`-local scope for everything after the expansion ... Left alone deliberately,
  it is the same territory as the plain-label-export row the fifth parcel
  booked, and moving it moves whatever depends on it."* That note's probe
  `l1.asm` was never committed (the only committed `l1.asm` belongs to
  `2026-09-09-as-dup-operand-probes` and is unrelated); its three claims are
  re-measured here as `a06`, `a02` and `s01`/`s05`.

Branch `parcel/as-macro-dot-scope-after-call`, base `07edc95f`.

## Provenance

* Oracle: `s1disasm/build_tools/Linux-x86_64/asl`, md5
  `61e672562465725a8c102288a7da9098`, through `asl-reference/asl_ref.sh`'s
  `asl_run`, flags `-xx -n -q -A -L -U -i .`. Bytes from the `p2bin` beside it,
  only from runs that exited 0.
* sigil base: `07edc95f` built into this worktree's own `target/` and copied
  aside before any edit, `sigil 0.1.0 (07edc95f)`, md5 `cea08de4`. sigil fix:
  `75d689d0`, `sigil 0.1.0 (75d689d0)`, "clean-sources at capture", md5
  `950d3f94`. The commits after it change tests, docs and note tools, plus one
  `eval.rs` COMMENT line in the note's commit (an em dash in a comment the fix
  rewrote, made a colon); no code. The final suite and clippy were run at that
  last commit.
* Probes: `gen.py` writes `probes/` (101 shapes, one per file, one suspect line
  marked `; REF`), at `org $100` behind a `$1111` word. Every candidate a
  reference could bind carries its own word: `$5555` Base, `$6666` Base.x,
  `$7777` a file-level Inner, `$8888` its `.x`. So the bytes say WHICH name
  bound, not only that one did.
* Runner: the leak note's `matrix.sh` and `classify.py`, run from a copy with
  `ASL_REF` pointing at the guard, on scratch copies of the committed probes,
  with every sigil binary passed by absolute path.

## The rule as asl answers it

A plain label written anywhere in an expansion is the `.`-local scope for
everything after it, and "after it" crosses every expansion boundary on the way
out: the rest of that body, every enclosing body once the nested call returns,
and the caller after the whole expansion. The label ITSELF stays in its
expansion instance (`a06`: `Inner` read after the call is `#1010`). Only the
scope it opened travels.

* **The caller, after the call.** Body `Inner:`, then `.b := 2` after the call:
  `Inner.b` builds (`a01`, `0002`), `Base.b` is `#1010` (`a02`). The same for a
  `.y:` label (`a04`, `a05`), a call made twice (`a07`), no caller label at all
  (`a08`), all four spellings of the body label, `Inner:` alone, bare `Inner`
  alone, bare `Inner` on a data line, and indented `Inner:` (`l_*`), and a
  body label in a taken `if` (`c08`). With two body labels the LAST wins (`t01`
  builds `In2.b`, `t02` refuses `In1.b`, `t03` refuses `Base.b`).
* **Only a line that runs.** A label in an untaken `if` (`c06`) or after an
  `exitm` (`c07`) moves nothing; a body that writes no plain label (`c01`,
  `c02`) or only a `.q:` (`c03`) leaves the caller's scope where it was.
* **The silent case.** Where both assemblers build, the bytes differ unless the
  scope moved, and nothing says so:

  | shape | what it asks after the call | asl | sigil at base |
  |---|---|---|---|
  | `s02` | `.x`, with a file-level `Inner:`/`.x:` BEFORE the caller | `$0104`, Inner.x | `$0108`, Base.x |
  | `s03` | the same, file-level Inner after the reference | `$010E` | `$0104` (`$6666`'s address) |
  | `s06` | `.a`, with the file `Inner:` binding `.a := 9` | `9` | `3` (Base.a) |
  | `s07` | `if defined(.x)` | false, `$BBBB` | true, `$AAAA` |
  | `s08` | `ifdef .x` | false, `$BBBB` | true, `$AAAA` |
  | `v04` | inside the body, after `Inner:`, `.x` | `$0104`, the file's Inner.x | `$0108`, Base.x |
  | `n15` | an outer body's `.lp`, after a nested call wrote `Inner:` | `$0104`, the file's Inner.lp | `$0108`, its own |

  The same crossing with no second candidate is a refusal in asl and a build
  here (`s01`, `s05`, `s09`, `s10`). A second `.x:` after the call is a NEW
  name, `Inner.x` (`s12` builds; base said `double defined: Base.x`), and it
  collides with any other `Inner.x` (`s11`), including the one a SECOND caller
  of the same macro writes: two routines that each call the macro and then
  define `.lp` are `#1000 symbol double defined` in asl (`s13`).
* **Inside the body, after its plain label.** A value binding is that label's
  and is GLOBAL (`v01`: `.v := 5` in the body reads as `Inner.v` after the
  call); a reference to a `.`-local the body does not define looks under that
  label (`v03` refused, `v04` the file's). The same lines written BEFORE the
  label use the caller's scope (`v05`, `v06`). A `.lp:` the body wrote before
  its label is out of reach as `.lp` after it (`v08`), one written after it is
  not (`v07`).
* **Expansion first, then outside.** A `.`-local under a body label is found
  in the expansion when the expansion holds one (its own PC `.`-labels), and
  outside it otherwise: the body reads its own global `.v := 5` back as `.v`
  and as `Inner.v` (`v09`, `v10`), reaches a file-level `Inner.x` written
  before or after the call (`v04`, `v13`, `v14`), and an inner body reaches the
  `.x` of a label its caller's body wrote, before or after the call (`v11`,
  `v12`).
* **Nesting.** A nested body's label is the enclosing body's scope for the rest
  of it and the caller's after the nest (`n01`, `n02`, three deep `n13`); an
  enclosing body's own later label moves it again (`n04`, `n05`); an enclosing
  body's `.v := 3` after the nested call is `Inner.v` (`n06`, `n07`), its
  caller-scope `.x` is `Inner.x` (`n10`). An enclosing body's `.y:` written
  after the nested call is QUALIFIED by the nested label (`n12` reads it as
  `Inner.y` inside the body, `n11` cannot read it as `Outer.y`) but STAYS in
  the enclosing expansion (`n09`: `Inner.y` from outside is `#1010`), reads
  back as `.y` (`n08`), forward (`n17`), in two expansions without a collision
  (`n18`), and through `defined(.y)` and `ifdef .y` (`n19`, `n20`, true). A
  `.lp:` written BEFORE the nested call is out of reach as `.lp` after it
  (`n14`) or, with a file-level `Inner.lp`, that one (`n15`). A `label`
  directive in the nested body does the same (`n16`).
* **Loops.** A body label inside a `rept` or `irp` iteration, at file level or
  in a macro body, moves the scope too (`r01`..`r09`; `r08`: the LAST item's
  substituted name, `Bb`); a macro body's `.y:` after such a loop stays in the
  macro's expansion (`r11`) and reads back in it (`r12`).
* **The option twins.** `{GLOBALSYMBOLS}` (`g01`, `g02`, the leak note's `gs5`,
  already matched) and `{INTLABEL}`: a `__LABEL__:` body label is the call
  line's label text (`g03`, `g04`), a literal body label replaces it (`g05`,
  `g06`). A call line's own label (`Lbl mac`) opens its scope first and the
  body label replaces it (`g07`, `g08`; `g09` is the control with no body
  label). A `{GLOBALSYMBOLS}` frame nested with a plain one, either way round,
  carries the label out through both (`g10`..`g13`).

## Before and after

### This parcel's 101 shapes

Base `07edc95f`: **MATCH 34, LEAK 26, OVER-REFUSE 34, VALUE-DIFF 7**
(`compact-base.txt`).

* VALUE-DIFF {n15, s02, s03, s06, s07, s08, v04}
* LEAK {a02, a05, g04, g08, g12, g13, l_bare_alone_read_base,
  l_bare_data_read_base, l_colon_alone_read_base, l_indented_colon_read_base,
  n02, n07, n10, n11, n14, r04, r07, s01, s05, s09, s10, s11, s13, t03, v02,
  v03}
* OVER-REFUSE {a01, a04, a07, a08, c08, f01, g03, g05, g07, g10, g11,
  l_bare_alone_read_inner, l_bare_data_read_inner, l_colon_alone_read_inner,
  l_indented_colon_read_inner, n01, n03, n04, n06, n12, n13, n16, r03, r06,
  r08, r09, s12, t01, v01, v10, v11, v12, v13, v14}

Fix `75d689d0`: **MATCH 101** (`compact-fix.txt`).

The first cut of the fix (pieces 1 to 3 below, without piece 4), run on the
99 shapes that existed then: MATCH 94, OVER-REFUSE {v04, v09, v10, v13, v14}.
`v09` was MATCH at base: that cut REGRESSED it. A body's `.v := 5` after its
label wrote the global `Inner.v` while the reader looked only at the
expansion's private key. The suite did not see it; a probe written to ask what
the new lookup did with a value binding did. Piece 4 closed all five.

### The leak note's 150 shapes

Re-run on scratch copies of `probes/`, `probes2/`, `probes3/`, `probes4/`.

* Base `07edc95f`: MATCH 147, LEAK {a12}, OVER-REFUSE {a11, gs8}. Reproduces the
  brief's figures exactly.
* Fix `75d689d0`: MATCH 149, OVER-REFUSE {gs8}.
* Changed: exactly {a11: OVER-REFUSE to MATCH, a12: LEAK to MATCH}. No other
  shape moved. `gs8` is the leak note's separate `{GLOBALSYMBOLS}`-in-a-plain-
  outer reader gap and is untouched.

## The fix, established from code

All in `crates/sigil-frontend-as/src/eval.rs`.

1. **`define_label` writes the real scope.** A plain label also writes
   `outer_scope` whenever a non-transparent frame is live. The outermost exit
   already handed `outer_scope` back to the caller (that is how a `label`
   directive's scope outlives the call); a plain label now feeds it too. Value
   bindings and caller-scope references after the label in the body read it
   through `real_scope` (`v01`, `v03`).
2. **`scope_epoch`.** Every real-scope write (`define_label`, `open_scope`) bumps
   a counter. A NESTED exit whose body opened a scope hands the real scope to
   the enclosing body as its `self.scope`; one whose body opened none restores
   the enclosing body's own scope, private ` macro#N` name included, exactly as
   before (`n12`, `n14`, `n15`, `n16`).
3. **`file_in_body_instance` and the whole-name arm of `owned_by_head`.** After
   such a nested return the scope is a label no LIVE instance owns (the nested
   one is gone), so an enclosing body's own `.y:` would key as the global
   `Inner.y` and leak (`n09`). A PC `.`-label the body text writes
   (`scan_dot_labels`), under a scope no live instance owns, is filed in the
   instance of the macro whose body writes it and recorded in that instance's
   `written` set; `owned_by_head` asks about the whole dotted name after the
   head, so the reader finds it, and `prev_owned` serves a forward reference on
   the next pass. `MacroFrame` records its instance key for this.
4. **`sym_key` looks in the expansion, then outside.** A key privatised by head
   ownership falls back to the global name when the instance never filed it
   (absent from both the seeded environment and this pass's definitions). Only a
   PC `.`-label is ever filed in an instance; a value binding under the same
   head is global, and so is a file-level `Inner:`'s `.x:`.

**The pass-count race the maclocal parcel designed out is untouched.** A body's
own `.x` under its private ` macro#N` scope never reaches piece 4: that key is
space-led, `owned_by_head` returns it unchanged, so `k == q` and there is no
fallback. Piece 4 applies only under a spellable head a live instance owns, and
its presence test reads the environment, which carries the previous pass, so a
forward definition of the instance's own name wins from the next pass on and the
answer cannot differ between the two passes `run_impl` returns on.

## Tests and red-first evidence

`crates/sigil-frontend-as/tests/as_macro_dot_scope_after_call.rs`, 8 tests. Every
fixture is a committed probe read at test time; every expected byte string is
that probe's asl `p2bin` output from an exit-0 run; every refusal is a probe asl
refused.

| test | carries |
|---|---|
| `a_plain_label_in_a_macro_body_is_the_callers_dot_scope_after_the_call` | a01 a02 a04 a05 a07 a08 c08 f01 l_* t01..t03 |
| `a_dot_local_that_crosses_the_call_binds_the_name_asl_binds` | s01..s13 |
| `inside_the_body_a_binding_or_reference_after_its_plain_label_uses_that_label` | v01..v08 |
| `a_dot_local_under_a_body_label_is_looked_up_in_the_expansion_then_outside` | v09..v14 |
| `a_nested_body_label_moves_the_scope_of_every_body_it_returns_to` | n01..n20 |
| `a_body_label_inside_a_loop_iteration_moves_the_scope_too` | r01..r12 |
| `the_option_twins_and_a_labelled_call_follow_the_same_rule` | g01..g13 |
| `a_body_that_writes_no_plain_label_leaves_the_callers_scope_alone` | a03 a06 c01..c07 f02 (controls) |

**Red at base.** Committed first (`779cbfcc`) and run against base `eval.rs`: 6
of the 7 tests then in the file failed, each on its first fix-carrying cell
(a01, s01, v01, n01, r03, g03). The seventh, the control test, was also red on
the first cut, because it carried `f01`, which is not a control (it is `a01`
with a forced extra pass, and base refuses it); `f01` moved to the first test.

**Half-fix matrix.** `mutations.sh` (in this directory) removes one piece of the
COMMITTED fix at a time: each anchor asserted to match exactly once, the change
shown on disk with `git diff` before the run, the WHOLE `sigil-frontend-as`
suite run, `eval.rs` restored from HEAD and verified clean. It refuses to start a
mutation unless `eval.rs` is clean against HEAD. Run at `75d689d0` (862 tests),
M4 re-run at `591c06fc`, M9 and M10 at the note commit. "red" counts failing
tests out of 862.

| mutation | removes | red | failing test, and the cell that shows it |
|---|---|---|---|
| M0 | the whole fix (`eval.rs` at base) | 7 | every test but the control |
| M1 | piece 1: a plain label does not write the real scope | 7 | every test but the control (a01, s01, v01, n01, r03, g03, v10 refused or built wrong) |
| M2 | piece 2: a nested exit ignores the epoch | 1 | nested (`n11` BUILDS, asl refuses) |
| M3 | piece 3, writer: `file_in_body_instance` files nothing | 1 | nested (`n09` BUILDS: the enclosing body's `.y:` leaks as a global `Inner.y`) |
| M4 | piece 3, reader: `owned_by_head` asks only about the head | **0** at `75d689d0`, **1** at `591c06fc` | nested (`n19`: `defined(.y)` answers `$BBBB`, asl `$AAAA`) |
| M5 | piece 4: no expansion-then-outside fallback | 2 | inside-the-body (`v04` refused), expansion-then-outside (`v09` refused) |
| M6 | `open_scope` does not bump the epoch | 1 | nested (`n16` refused) |
| M7 | a plain label does not bump the epoch | 1 | nested (`n11` builds) |
| M8 | the frame never learns its instance key | 1 | nested (`n09` builds) |
| M9 | WRONG FIX: the outermost exit hands back the body's own scope | 13 | the control (`c01`: `Base.b` refused after a label-free body), the option twins (`g09`), and 11 existing tests across `eval.rs`, `as_binder_opens_scope.rs` and `as_macro_label_leak.rs` that pin the maclocal and `label`-directive scope rules |
| M10 | WRONG FIX: the body SCAN's label moves the scope whether or not it runs | 2 | the control (`c06`: an untaken `if`'s `Inner:` moved the scope), inside-the-body (`v05`: the scope moved before the label line ran) |

**M4 walked through the first run, and the reason is a property of the front
end, not of the test.** An operand reference is keyed TWICE: the operand path
qualifies `.y` to `Inner.y`, and the fold's own symbol closure calls `sym_key`
again on the result, whose PLAIN branch asks `plain_label_scope("Inner.y")` and
finds the name the enclosing instance wrote. So every operand shape reached the
expansion's key without the arm. `defined()` and `ifdef` key the name ONCE
(`sym_defined_now`), so there the arm is the only route. `n19` and `n20` put
exactly that to asl (true, `$AAAA`); base also answers `$AAAA` (it kept the
enclosing body's private scope), so they are pins of piece 3's reader rather
than red-at-base cells, and M4 answers `$BBBB`, silently.

**The control test is red only under a wrong fix, by construction.** Base and
asl already agree on every control shape, so neither base nor any piece removed
from the fix can red it. M9 and M10 are the two plausible WRONG designs it
exists to refuse: handing back whatever scope the body left rather than the real
scope, and moving the scope on the body scan rather than on the line that runs.

Proof method, stated once: the red-at-base run used the test file as committed at
`779cbfcc`; the half-fix runs used it as committed at `75d689d0` and, for M4,
`591c06fc` (which only ADDS two cells to the nested test, so a red set measured
at `75d689d0` can only grow there, never shrink).

## Exposure, measured

**Corpora.** Clones of `s1disasm` (`f6ece65`), `s2disasm` (`e45ebf3`) and
`skdisasm` (`2fcd861`) inside this worktree's scratch, never the checkouts
(`corpus-prepare.sh` writes generated includes into the tree it is given, and
`s1-census.sh` makes a git worktree of the repo it is given; both were given
the clones). Base binary `cea08de4`, fix binary `950d3f94`.

| corpus | instrument | base | fix | moved |
|---|---|---|---|---|
| s1 | `s1-census.sh`, pristine and stubbed | 1 line, exit 1 | 1 line, exit 1 | 0 lines, 0 unresolved names either way |
| s2 | `corpus-baseline.sh` over `s2.asm` | 2 lines, exit 1 (layout: no `-z`) | 2 lines | 0 lines, 0 names |
| sk | `corpus-baseline.sh` over `sonic3k.asm` | 137 lines, exit 1 (front end) | 137 lines | 0 lines, 0 names |
| s1 | whole image, `build.lua`'s p2bin instruction (`-p=FF -z=0,kosinski,...`) | md5 `09dadb50`, CRC `afe05eee`, 524288 | identical (`cmp`) | **0 bytes** |
| s2 | whole image, `build.lua`'s instruction (`-p=0 -z=0,saxman-bugged,...`) | md5 `9feeb724`, CRC `7b905383`, 1048576 | identical (`cmp`) | **0 bytes** |
| s2 | final-pass symbol environment | 23142 keys | 23142 keys | **0 keys, 0 values** |
| sk | final-pass symbol environment | 39519 keys | 39519 keys | **0 keys, 0 values** |

Both whole images equal the references the workspace byte gates pin
(`as_driver_placement_corpus.rs`: CRC `0xafe05eee`, 524288;
`as_sonic2_whole_rom.rs`: CRC `0x7b905383`, 1048576, census md5 `9feeb724...`).
sk cannot emit an image (it stops at the front end on both binaries), which is
why the symbol environment was measured for it.

`corpus_bytediff.sh` was read and NOT run: it compares `examples/*.emp` and the
two `.emp` game invocations, which go through the `.emp` front end this change
does not touch, and it builds a second binary in the MAIN checkout's `target/`,
which this parcel may not write.

**The zeros are not blind, and the code under test ran.**

* Engagement (`SIGIL_CENSUS_EXPLABEL=1`, fix binary): s2 prints 436 plain-label
  definitions inside expansions over its passes (`__LABEL__Plc` 236, `start`
  148, `end` 16, nine `APM_*_Blocks`), sk 1314 (`__LABEL__Plc` 384, `start`
  213, `DAC_*_Setup`, ...). Every one is a site where piece 1 now moves the
  caller's scope. s1 prints `instances-with-labels=0` on every pass: it writes no
  label in a body at all, so its zero is a floor.
* The environment diff: an instrumented copy of each tree (`dumppatch.py`,
  applied to a `git archive` of each revision, never this worktree) writes the
  whole environment per pass, to one file per `mompass` value, so a later pass
  with the same value overwrites it and the highest file holds the final pass;
  `envdiff.py` compares the highest. Positive control
  on the same instrument: probe `a01` under the two instrumented binaries gives
  `only-A "Base.b" Int(2)`, `only-B "Inner.b" Int(2)`. And the instrumented
  binaries were shown to be what they claim: base refuses `a01`, fix builds it
  to asl's bytes.
* A catch in that instrument, recorded because it would have faked the result:
  the first fix build shared a target directory with the base build, finished in
  0.01 s, and produced a byte-identical binary (both md5 `65b9b92b`). A diff
  between those two would have been zero by construction. Rebuilt in its own
  target directory it compiled `sigil-frontend-as` and came out `1116cb0f`; only
  that pair was used.

Why nothing moved, and why it could not have: the fix changes a `.`-local's
key only between such a call and the next file-level plain label. asl refuses
most of those shapes (`s01`, `s05`, `s09`), and `s13` shows two routines that
each call such a macro and then define the same `.`-local collide outright, so
a corpus that builds under asl carries no `.`-local across these calls. The
fix then moves no key in s2 or sk, which is what the environment diff measured.

**aeon**, read-only through `git -C /home/volence/sonic_hacks/aeon ... origin/master`
(`104134ca`). Its AS inputs are `engine/debug/debugger.asm` and the two
`games/*/game_root.asm`. `bodyscan.py` over `debugger.asm`: 12 macro bodies, 537
body lines, **0 plain labels** (every body label is `.skip:`, `.__data:`,
`.__leave:` or a `.__x: set`); positive controls, the same scanner over probes
`a01` and `l_bare_alone_read_inner` finds 1 each, over `c03` (a `.q:` only) 0.
And no macro is ever EXPANDED there: both `game_root.asm` files define and invoke
no macro, and every invocation in `debugger.asm` (`git grep` finds 21, the
positive control for that instrument) sits inside another macro's body. The
changed code runs on no aeon line.

## The plain-label-export row

`AS-MACRO-LABEL-EXPORT`, last recorded in `docs/OVERSEER-LOG.md` (the
2026-09-05 cut) as state `doing`, narrowed: *"BOTH halves as written are
DISCHARGED: a plain label in a macro body read from outside is now refused by us
and by the reference alike ... and the explicit-export spelling ... is the
'label' directive, which we accept and resolve ... WHAT IS ACTUALLY LEFT ... I
confirmed both assemblers RESOLVE the exported form but did not confirm they
resolve it to the SAME VALUE ... this is now a pin-it parcel."* That pin landed
the same day as `3d9322c5` ("pin the `label` directive's escape from a macro body,
at an address that can disagree": `the_label_directive_with_a_pc_operand_escapes_the_macro_body`
and the snippet vector `as_label_directive_star_escapes_the_macro_body`). The row
is not on the live board (`docs/lane-status.json` names
AS-MACRO-DOT-SCOPE-AFTER-CALL and not LABEL-EXPORT) nor in `docs/QUEUE.md`.

So it is **closed in substance, and not in conflict with this change.** Its two
claims are what a body label NAME does; this change is what a body label's SCOPE
does. `a06` keeps the name unreachable from outside, the `label` directive's
tests are among the 862 that pass, and `c04` pins that directive's scope after a
call. The maclocal note's worry, *"moving it moves whatever depends on it"*, is
answered by measurement above: in s1, s2, sk and aeon, nothing did.

## Left open

* **`gs8`**, the leak note's `{GLOBALSYMBOLS}` macro called from a plain one,
  unchanged, a separate reader gap.
* **A body `.q:` qualified by the CALLER's label, spelled out inside the body.**
  Before any plain label in a body, sigil files the body's own `.q:` under the
  private ` macro#N` scope; asl qualifies it with the caller's label and keeps
  it in the expansion. The two differ only when the body spells `Base.q`
  explicitly. Not probed, not this row; recorded because piece 3 uses the asl
  naming only after a nested return and the private naming everywhere else.
* **`.`-labels in a `rept` inside a macro body** are filed in the macro's
  instance by piece 3, as the private-scope rule already did before any body
  label; whether asl files them per ITERATION (as it does nameless and plain
  labels) was not probed.
* Not probed: `save`/`restore` or `phase` across the expansion boundary; a body
  label written in a file `include`d from the body (the leak note's include rule
  makes it the body's; its scope effect after the call was not asked).

## Things in the brief that turned out different

1. **"`l1.asm` (find where that note's probes live)."** The maclocal note's
   `l1.asm` was never committed; the only committed `l1.asm` is another note's.
   Its three recorded claims are re-measured here (`a06`, `a02`, `s01`/`s05`) and
   hold.
2. **The plain-label-export row is not in `docs/QUEUE.md`,** and `git log -S`
   finds it only in the commit that moved the console payload into
   `OVERSEER-LOG.md`. Its standing had to be read from that log and the pin
   commit, as above.
3. **`corpus_bytediff.sh` does not measure the AS corpora at all** (it compares
   `.emp` examples), and running it as written would build in the main
   checkout's `target/`. It was not run; the AS corpora were measured by the
   three AS scripts and by whole images and symbol environments instead.
4. **The corpus scripts could not answer "did any output byte move"** for any
   corpus: all three stop before an image (s1 and s2 at layout for want of the
   build scripts' `-z` instruction, sk at the front end). The byte answer for s1
   and s2 came from building with each build script's own p2bin instruction; for
   sk there is no image, so the answer is the environment diff.
