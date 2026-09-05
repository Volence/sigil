# AS-REGISTER-DIAGNOSTIC-RESIDUE

A register written where a value belongs now gets ONE sentence, on the line that
wrote it, at every one of the fifteen places that can be handed one. Before this
it got **seven different outcomes**, one of which named a symbol the author had
actually defined, two of which had no source location, and two of which were
**no diagnostic at all**.

Branch `parcel/as-register-diagnostic-residue`, off master `742c7366`.

| commit | what |
|---|---|
| `392503fe` | the fix |
| `570896d2` | the gate: 25 consumers, red-first |

## Provenance

| | |
|---|---|
| sigil BEFORE | master `742c7366`, `git archive`d to `/home/volence/sonic_hacks/.reg-diag-before` and built fresh with its own `CARGO_TARGET_DIR`, md5 `703a419d3e93542c776cd6d034cde89b` |
| sigil AFTER | branch tip `570896d2`, md5 `c12fb02a702af8c7c8407b29c24467b3`, `--version` reports revision `570896d2`, closure-revision `392503fe` |
| corpus | `s2disasm` at `e45ebf332f39987424ca3102e50c717628f71269`, detached worktree `/home/volence/sonic_hacks/.s2-reg-diag`, `git status --porcelain` empty. The owner's live checkout at `/home/volence/sonic_hacks/s2disasm` was read (for the revision) but never written. |
| reference assembler | NOT INVOKED in this parcel. The asl measurements quoted below are the ones the dispatching brief carried, taken with md5 `61e672562465725a8c102288a7da9098` and exit status checked. `s2disasm`'s own asl build (`0dee1f98e6480a4783d27ffd8b90896f`) was never invoked. |
| suite log | `.runlogs/suite.log` in this worktree, stamped with pwd, HEAD `570896d2` and branch on its first three lines. NOT COMMITTED and it dies with the worktree, so every number taken from it is quoted in full below rather than pointed at. |
| corpus log | `.runlogs/corpus.log`, same, ends `CORPUS-END-MARKER`. `corpus.sh` beside this note regenerates it. |
| scratch trees | Both were created for this run and REMOVED afterwards, so the owner's `s2disasm` ends with the worktree list it started with. Recreate with `git -C /home/volence/sonic_hacks/s2disasm worktree add --detach <path> e45ebf33` and `git -C <sigil> archive 742c7366 \| tar -x -C <path>`. |

**Freshness witness for the BEFORE binary**, which carries no revision because a
`git archive`d tree is not a repository. Two independent ones:

1. It reproduces all four probes from the brief **verbatim**, including variant
   4's missing location. That is a match against a measurement taken by a
   different session with a differently built binary.
2. It reports **5,243 diagnostic rows over 5,112 distinct sites** on this
   corpus, which is exactly the AFTER figure recorded in
   `2026-09-05-as-jmptos-518-block.md` for master at that parcel's tip. A number
   that reproduces an independent earlier measurement is evidence the instrument
   ran and is current; the md5 alone would not have been.

## The root cause, and it is one

A register name is not in the symbol table. So every expression holding one
folds to `Fold::Poison`, and **Poison is the shape of a forward reference**.
AS resolves symbols over several passes, so a name that does not resolve on
THIS pass is ordinarily a name that will resolve on a later one, and every
consumer of a Poison expression accordingly does what a forward reference
deserves: it defers the value to the linker, or it reports it as unresolved and
waits for the converged pass.

The register was never distinguished from a symbol nobody had defined yet,
anywhere except the single strict-argument check landed at `d5b5e8f1`. That is
why variant 1 in the brief reads correctly and the other three do not: variant 1
is the one case that had a bespoke check, and every other case fell through to
the generic Poison machinery.

What separates the two is provable and cheap: **no later pass defines `a0`**. So
the fault is reported at the point of use, on the pass that sees it, with that
line's span. That is also the whole answer to the missing location, and the
answer arrives from a direction the brief's open question did not consider: it
is not that the link stage learns to carry a source location, it is that a
register never reaches the link stage.

## Every variant, before and after

Both columns come from the same script,
`2026-09-05-as-register-diagnostic-residue-probes/matrix.sh`, run twice: once
bare (branch tip) and once as `SIG=/home/volence/sonic_hacks/.reg-diag-before/.target-land/release/sigil matrix.sh`.
Every row below is quoted from that transcript. `(2)` and `(3)` are the line
numbers the message named.

### Deferred to the linker: no source location, and often no message at all

| written | before | after |
|---|---|---|
| `dc.l a0` | `error: unresolved symbol `a0` for fixup in section sec0 at offset 0` | `m.asm(2): error: `a0` is a register, not a value: expected an integer, floating point number or string` |
| `dc.b a0` | `error: unresolved symbol `a0` for fixup in section sec0 at offset 0` | same, `m.asm(2)` |
| `dc.w a0` | `error: unresolved symbol `a0` for fixup in section sec0 at offset 0` | same, `m.asm(2)` |
| `dc.b a0+1` | `error: unresolved target expression (dangling symbol(s) `a0`) for fixup in section sec0 at offset 0` | same, `m.asm(2)` |
| `dc.w a0+1` | `error: unresolved target expression (dangling symbol(s) `a0`) for fixup in section sec0 at offset 0` | same, `m.asm(2)` |
| `move.l #a0,d0` | `error: unresolved symbol `a0` for fixup in section sec0 at offset 2` | same, `m.asm(2)` |
| `dc.l sp` | `error: unresolved symbol `sp` for fixup in section sec0 at offset 0` | `` `sp` is a register, not a value: … ``, `m.asm(2)` |
| `dc.l A0` | `error: unresolved symbol `A0` for fixup in section sec0 at offset 0` | `` `A0` is a register, not a value: … ``, `m.asm(2)` |
| `X equ a0` then `dc.l X` | `error: unresolved symbol `X` for fixup in section sec0 at offset 0` | `` `a0` is a register, not a value: … ``, `m.asm(2)` |
| `jsr a0` | **PANIC**, exit 101, `internal error: entered unreachable code: JmpJsrSym must be lowered by resolve_layout before layout/link` | `m.asm(2): error: `a0` is a register, not a value: …` |
| `jmp a0+1` | **PANIC**, exit 101, same | `m.asm(2): error: `a0` is a register, not a value: …` |

The `equ` row is worth its own line. It named **`X`**, a symbol the author had
just defined, and said nothing whatever about `a0`.

### Reported on the right line, with the wrong story

| written | before | after |
|---|---|---|
| `dc.l a0+1` | `m.asm(2): error: unresolved long expression` | `m.asm(2): error: `a0` is a register, not a value: …` |
| `dc.l us(a0)` (function body USES its parameter) | `m.asm(3): error: unresolved long expression` | `m.asm(3): …` |
| `dc.l a0+a1` | `m.asm(2): error: unresolved long expression` (one line, naming neither) | two lines, naming `a0` and `a1` |
| `dc.l a0+zz` | `m.asm(2): error: unresolved long expression` | `m.asm(2): error: `a0` is a register, not a value: …` |
| `move.w #a0,d0` | `m.asm(2): error: unresolved symbol `a0` in operand` | `m.asm(2): error: `a0` is a register, not a value: …` |
| `move.w #a0+1,d0` | `m.asm(2): error: unresolved symbol `a0` in operand` | same |
| `moveq #a0,d0` | `m.asm(2): error: unresolved symbol `a0` in operand` | same |
| `move.w a0+1,d0` | `m.asm(2): error: unresolved symbol `a0` in operand` | same |
| `org a0` | `m.asm(2): error: org needs a constant expression` | `m.asm(2): error: `a0` is a register, not a value: …` |
| `ds.b a0` | `m.asm(2): error: unresolved ds count` | same |
| `align a0` | `m.asm(2): error: unresolved align constant` | same |
| `rept a0` | `m.asm(2): error: unresolved rept count` | same |
| `while a0` | `m.asm(2): error: unresolved while condition` | same |

### Accepted silently

| written | before | after |
|---|---|---|
| `if a0` | **no diagnostic, exit 0**, the condition taken as false | `m.asm(2): error: `a0` is a register, not a value: …` |

### The variant the brief did not have, and it is the worst one

Injecting `dc.l a0` at line 86 of the corpus's `s2.asm` (a 91k-line program that
already carries other diagnostics) produced, on the BEFORE binary, **5,243
diagnostic rows and not one of them naming line 86**. The register was deferred
as a fixup, the front end returned an error for unrelated reasons, and the link
stage that would have refused the fixup was therefore never reached.

So in the realistic case, which is a program with more than one thing wrong with
it, `dc.l a0` was not a badly-worded diagnostic. It was **no diagnostic**.

On the AFTER binary the same tree gives 5,244 rows, and `diff` of the two stderr
streams is exactly one added line:

```
10a11
> s2.asm(86): error: `a0` is a register, not a value: expected an integer, floating point number or string
```

## The message, and the wording call

```
`a0` is a register, not a value: expected an integer, floating point number or string
```

The tail is asl's own message catalogue (`as.msg` #1145, *"expected integer,
floating point number or string but got register"*). The head names the offending
token, which asl's does not.

A reasonable person could have worded this differently and the alternative
considered was keeping the function-argument variant's more specific tail (*"a
function argument must be an integer, floating point number or string"*) for
that one context, on the grounds that it is more informative there. That was
rejected: the parcel's whole subject is that one fault had many stories, and
keeping a context-specific tail keeps two of them alive for no gain a reader can
use. The context is already carried by the source line the message points at.

What the message must never contain is **"unresolved"** or **"undefined"**. Both
are true statements about sigil's symbol table and false ones about the program,
and both send a reader hunting for a definition of `a0` that was never missing.
`no_consumer_calls_a_register_a_missing_definition` enforces that as a property
over the whole table rather than as a wording preference.

## What is NOT the goal here

asl fidelity, deliberately, and this is worth stating because the shape looks
like an asl-parity row and is not one.

Per the brief's measurement (reference asl md5
`61e672562465725a8c102288a7da9098`, exit status checked, positive control
`dc.l $12345678` showing `1234 5678` and advancing the PC by 4):

* `dc.l a0+1` is the ONE variant asl diagnoses: exit 2, `expected integer,
  floating point number or string but got register`.
* `dc.l a0`, `dc.l ig(a0)` and `dc.l us(a0)`: asl exits **0** and **silently
  emits nothing**. No bytes in the listing, and the program counter does not
  advance across the `dc.l`.

A `dc.l` that produces zero bytes and says nothing is the silent-wrong-answer
class. sigil's refusal is correct and stays. Only the wording moved, and in two
cases the wording moved from "nothing at all".

## The design, and why the population is the consuming end

The register-ness of a name is established at ONE place (the expression) and
acted on at FIFTEEN (every consumer of a Poison expression). A check at the
producer proves nothing about any consumer, which is why the fix is a pair of
helpers called from every consumer rather than a filter somewhere central:

| helper | what |
|---|---|
| `report_register_values(e, span) -> bool` | names every unresolved REGISTER in `e` at `span`, deduplicated per (span, name) for the pass, and answers whether there was one. `true` is the caller's signal to take an error path instead of emitting a placeholder and deferring a fixup the linker can only refuse. |
| `route_poison_names(e, span) -> bool` | splits the unresolved names between their two stories: registers to the above, everything else to `poison_refs` where a forward reference belongs. `dc.l a0+Later` says both. |
| `register_reported_at(span) -> bool` | asked by a directive that has its own word for not getting a constant, before it adds that word. |

Call sites:

| consumer | helper |
|---|---|
| `eval_all` Poison arm | `report_register_values`. This is the choke point for `org`, `ds`, `align`, `rept`, `while`, `if`, `irpc` and eleven more callers. |
| `fold_imm` Poison arm | `route_poison_names` |
| `abs_ea_from_expr` Poison arm | `route_poison_names` |
| `jmp`/`jsr` symbolic-target deferral, and its Poison arm | `report_register_values` guard, then `route_poison_names` |
| `try_defer_imm` (the imm16/imm32 cross-seam deferral) | `report_register_values` guard |
| `directive_db`, `directive_dw`, `directive_dc_w`, `directive_dc_l` Poison arms | `report_register_values` guard, then the placeholder alone with no fixup |
| `check_ignored_arg` | `route_poison_names`; its bespoke bare-identifier register branch was DELETED, so the strict-argument path now tells the same story as everything else rather than its own. |
| `directive_org`, `directive_ds`, `directive_align`, `rept`, `while` | `register_reported_at` guard around their generic message |

### The property that keeps this from being a ban on a spelling

The check fires only on names that **do not resolve**. A program that defines a
symbol called `a0` is untouched:

```
a0  equ 5
    dc.l a0     ->  00 00 00 05, exit 0, on both binaries
```

That is pinned by
`a_symbol_that_happens_to_be_spelled_like_a_register_still_assembles`.

`(a0)` as a register-indirect effective address is a register in a REGISTER
position and is not an expression at all: `lea (a0),a1` assembles to `43 D0` on
both binaries, pinned by `a_register_in_a_register_position_is_untouched`.

z80 is out of scope by measurement, not by omission. `is_expr_register_name` is
already 68000-only, because a z80 program is free to define a symbol called `sp`
and asl answers `#1010 symbol undefined` for `dw hl`. `dw hl` still leaves the
front end as a deferred fixup, pinned by
`z80_register_names_are_not_expression_level_registers`.

## The test-contract enumeration (bar 2)

### Messages this parcel changed or made conditional

| message | change | asserted on by |
|---|---|---|
| `` `X` is a register: a function argument must be an integer, floating point number or string`` | DELETED, replaced by the shared sentence | `as_strict_function_args.rs:107` (substring) |
| `unresolved symbol `X` in operand` | text unchanged; register names no longer route here | `imm32_defer.rs` (x4, all naming `ExternalSym`), `as_warning_exitm.rs:253` (`LC`), `as_disp_vs_function.rs` (doc only), `db_dw_defer.rs` (doc only), `tranche5_negative_probes.rs`, `act_descriptor_port.rs`, `label_values.rs`, `tranche7_code_concat.rs`, `extern_builtin.rs` |
| `unresolved long expression` | text unchanged; register names no longer route here | `imm32_defer.rs:15` (doc only), and now `as_register_in_value_position.rs:307` as a CONTROL |
| `unresolved symbol `X` for fixup in section …` (link) | code untouched; registers no longer reach it | `act_descriptor_port.rs:142` (comment), and the general link tests |
| `unresolved target expression (dangling symbol(s) …)` (link) | code untouched; registers no longer reach it | none found |
| `org needs a constant expression` | now suppressed when a register was reported at the same span | none found |
| `unresolved rept count` / `unresolved while condition` / `unresolved ds count` / `unresolved align constant` | same | none found |

### Both directions, as the bar asks

**RED LOUDLY (asserted the full string, would break on a rewording):** none in
the pre-existing suite. Nothing asserted the old function-argument sentence by
equality. The only full-string assertions on the new sentence are the ones this
parcel adds.

**STAYS GREEN WHILE NO LONGER TESTING WHAT IT NAMES:** exactly one candidate,
and it was **measured rather than assumed**.
`as_strict_function_args::a_register_argument_is_refused_as_a_register_not_as_a_missing_symbol`
asserts `m.contains("register") && m.contains("a1")` and
`!m.contains("unresolved symbol")`. Under the wording-only mutation described
below it stays green at 5/5 while the message it is nominally pinning has
changed under it. It is **not broken**: it still tests exactly what its name
says, because "says register, does not say unresolved symbol" is the whole of
its claim and remains true. It is left as it is, and it is the demonstration of
why the new rows assert whole strings.

Two further substring matchers on the word "register" exist and are unrelated:
`diag_assert_vector.rs:286` and `diag_desugar.rs:393` both also require
`"move"`, and neither path is touched.

### Bar 3: is the matcher unique?

The new sentence occurs in exactly one producer in the whole tree
(`eval.rs:9164`, `register_in_value_position`), verified by grep, and the table
rows assert message AND `file(line)` label by **equality**, not substring. A
guard deletion cannot produce that string from another rule.

This was checked the other way round too, because a grep over the code under
test cannot see it. Mutating only the WORDING (`floating point number` to
`float`, one line, shown on disk by `git diff HEAD --stat`) reds 3 of the 9 new
tests. That binds the assertions to this producer rather than to any phrase that
happens to be shared.

## Red-first proofs, with the mutation shown applied

### Proof 1: the guard

Mutation: `git checkout 742c7366 -- crates/sigil-frontend-as/src/eval.rs`.

Shown applied on disk two ways, because `git checkout <rev> -- <path>` STAGES
the change and plain `git diff --stat` reports nothing:

```
$ git diff HEAD --stat
 crates/sigil-frontend-as/src/eval.rs | 193 ++++++-----------------------------
 1 file changed, 30 insertions(+), 163 deletions(-)
$ grep -c "report_register_values\|route_poison_names\|register_reported_at" crates/sigil-frontend-as/src/eval.rs
0
$ grep -c "is a register: a function argument must be an" crates/sigil-frontend-as/src/eval.rs
1
```

Result: `test result: FAILED. 4 passed; 5 failed`.

```
25 of 25 consumers disagree
13 answers carry no source location, all of them <no refusal>:
   dc.b bare, dc.b compound, dc.w bare, dc.w compound, dc.l bare,
   move.l #imm, jsr, jmp, equ, if, sp, d7, uppercase A0
10 consumers still say it is a missing definition:
   dc.l compound            unresolved long expression
   move.w #imm              unresolved symbol `a0` in operand
   moveq #imm               unresolved symbol `a0` in operand
   absolute EA              unresolved symbol `a0` in operand
   ds count                 unresolved ds count
   align                    unresolved align constant
   rept count               unresolved rept count
   while condition          unresolved while condition
   function arg, body uses  unresolved long expression
   nested ignoring call     unresolved symbol `a1` in operand
```

The 4 that stayed green under the mutation are **exactly the four controls**,
which is the right answer: each pins behaviour the fix must not change.

Restored with `git checkout HEAD -- crates/sigil-frontend-as/src/eval.rs` from
the committed baseline `392503fe`, never `git checkout --` on a dirty tree.
`git diff HEAD --stat` empty afterwards, guard grep back to 22 hits, 9/9 green.

### Proof 2: the matcher

Mutation: one string literal, `floating point number` to `float`. Shown applied:

```
$ git diff HEAD --stat
 crates/sigil-frontend-as/src/eval.rs | 2 +-
$ grep -n "expected an integer, float or string" crates/sigil-frontend-as/src/eval.rs
9164:    format!("`{name}` is a register, not a value: expected an integer, float or string")
```

Result: `as_register_in_value_position` FAILED, 6 passed 3 failed;
`as_strict_function_args` ok, 5 passed 0 failed. Restored the same way.

### What a run MUST FAIL

Both proofs run the SAME binary the parcel changes (`sigil-frontend-as`
rebuilt from the mutated source, confirmed by cargo recompiling the crate in
each run), and both reds name the specific rows and the specific pre-fix
message text. A proof that printed a generic failure, or that failed on all 9
including the four controls, would mean the runner was not measuring what it
names.

## Corpus decomposition

`s2disasm` at `e45ebf33`, `<bin> s2.asm` from the corpus root, no flags.

**Nothing moved. Nothing rose. Nothing appeared. Nothing went.**

| rows before | sites before | rows after | sites after | delta | class |
|---|---|---|---|---|---|
| 2624 | 2602 | 2624 | 2602 | +0 | `bad operand expression` |
| 2309 | 2309 | 2309 | 2309 | +0 | `expected mnemonic, directive, or label` |
| 89 | 89 | 89 | 89 | +0 | `` `X` is not a recognized 68000 mnemonic`` |
| 49 | 1 | 49 | 1 | +0 | `bad word expression` |
| 39 | 39 | 39 | 39 | +0 | `cannot include <file>: no such file` |
| 30 | 1 | 30 | 1 | +0 | `bad byte expression` |
| 24 | 24 | 24 | 24 | +0 | `` unresolved symbol `X` in operand `` |
| 23 | 2 | 23 | 2 | +0 | `int(): could not evaluate float expression` |
| 18 | 18 | 18 | 18 | +0 | `` unknown directive or mnemonic `X` `` |
| 11 | 1 | 11 | 1 | +0 | `unexpected character` |
| 8 | 8 | 8 | 8 | +0 | `instruction needs an explicit size suffix (.b/.w/.l)` |
| 6 | 6 | 6 | 6 | +0 | `case needs a string literal` |
| 4 | 4 | 4 | 4 | +0 | `` malformed number (hex needs a trailing `X`) `` |
| 3 | 3 | 3 | 3 | +0 | `` bad displacement expression in `X` `` |
| 2 | 2 | 2 | 2 | +0 | `trailing tokens in operand` |
| 2 | 2 | 2 | 2 | +0 | `switch needs a string expression` |
| 1 | 1 | 1 | 1 | +0 | `unsupported form: <insn>` |
| 1 | 1 | 1 | 1 | +0 | `struct `X` has a member line this cannot read; …` |
| **5243** | **5112** | **5243** | **5112** | **+0** | **TOTAL** |

Locationless split, measured separately because `join_source.py` silently drops
a line with no `file(line)` prefix and this parcel is about exactly that class:
`before: total=5243 with file(line)=5243 without=0`, `after: identical`. Both
joins report `unparsed lines: 0`.

Unresolved-symbol NAME SETS, **both directions**: before-only 0, after-only 0,
in both 8. Every AFTER diagnostic line absent from the BEFORE run: **0**.
Register-message rows in the AFTER run: **0**.

**A zero delta here is not a measurement of the fix.** The corpus contains no
instance of the fault, which is unsurprising: `s2disasm` is a real disassembly
that assembles under asl, and asl would refuse (or silently swallow) a register
in value position, so nobody writes one. What this table establishes is
**inertness**: the fix changed no diagnostic on 5,243 real rows, and no
legitimate cross-seam deferral was turned into a refusal.

The ENGAGEMENT witness is separate and is the injection above: one register
placed in one real `dc.l` value position in the corpus root, before 5,243 rows
with none naming that line, after 5,244 with exactly one added. Without that
injection this section would read the same whether the changed code ran on the
corpus and agreed, or never ran at all, and here it plainly never ran.

The injection was done in a throwaway `cp -a` copy at
`/home/volence/sonic_hacks/.s2-reg-diag-inject`, since deleted. The corpus
worktree itself ends at `git status --porcelain` empty.

## Suite

`cargo test --workspace --release --no-fail-fast` with `SIGIL_ALLOW_PARTIAL=1`,
from this worktree, log stamped with pwd / HEAD `570896d2` / branch:

**4577 passed, 0 failed, 2 ignored, across 403 test binaries. Exit 0.**
Zero ` FAILED` lines in the log, so there are no failure names to list.

## What did NOT execute

* **The byte-identity golden gates did NOT run.** They did not pass; they were
  not measured. `SIGIL_ALLOW_PARTIAL=1` names no reference tree, and the harness
  derives that **127 test binaries** are reference-dependent and were therefore
  left unmeasured (floor 20, so the derivation is above its own confidence bar).
  A green result from this run says nothing about any row in those 127.
* **No aeon tree was touched.** No `AEON_DIR`, no aeon build, as the brief
  required.
* **No emulator.** No `mcp__oracle__*` call was made.
* **asl was not invoked.** The reference measurements quoted are the brief's.
* **The `.emp` front end was NOT examined.** Whether `.emp` has the same fault in
  its own expression path is unknown; a syntax probe was attempted and abandoned
  rather than guessed at. This is a gap, not a clearance.

## Booked, not fixed

Four things found while measuring that are outside this parcel's subject. None
is register-specific except the third, and each was confirmed against a control.

1. **`jsr <undefined symbol>` PANICS the CLI.** `jsr zz` (no register anywhere)
   gives `internal error: entered unreachable code: JmpJsrSym must be lowered by
   resolve_layout before layout/link`, exit 101. The plain `sigil <file>` path
   links without calling `resolve_layout` first. The register case no longer
   reaches it, which is a side effect of this parcel and not a fix for the
   underlying fault. **Reproduce:** `printf '\tcpu 68000\n\tjsr zz\n' > t.asm &&
   sigil t.asm`.
2. **`if <undefined symbol>` is silently false.** `if zz` produces no diagnostic
   and exit 0, the body skipped. Whether that is asl-faithful was not measured.
   The register case is now caught, the general case is not.
3. **A nested ignoring function call points at the definition, not the call.**
   `hu function p,gi(p)` on line 3, `dc.l hu(a1)` on line 4, reports line 3. The
   argument is dropped by the inner call, whose tokens come from `hu`'s body, so
   that is the span the substituted expression carries. It is a real line in the
   file, so the location claim holds, but the call site would serve a reader
   better. Pinned as line 3 in the gate with the reasoning written down, so a
   future change to it is deliberate.
4. **`lea (a0).l,a1` says `trailing tokens in operand`.** Not this fault:
   `(a0)` is parsed as a register-indirect EA and `.l` is then trailing, which is
   a syntax fault refused with a source location. Listed only so a reader who
   probes it is not surprised.

## Anything in this brief you concluded was wrong

Five things, one of them substantive.

1. **"There are at least three stories" is an undercount, and the framing of
   variant 4 understates the defect by a category.** There are seven distinct
   pre-fix outcomes, and two of them are not stories at all: `if a0` produced no
   diagnostic and exit 0, and `dc.l a0` in a program with any other front-end
   error produced no diagnostic either, because the link stage is never reached.
   The brief describes variant 4 as "loses the source location entirely". In the
   isolated probe that is exactly right. In a real program it is worse than that:
   the diagnostic does not exist. The corpus injection is the proof, 5,243 rows
   and not one naming the injected line.

2. **The brief's open question about variant 4 carries a false premise.** It asks
   "whether variant 4 can carry a source location at all without restructuring
   the link stage". Both branches of that question assume the answer has to come
   from the link stage. It does not. The register never needs to get there, so no
   part of the link stage changed and the location came for free. The question
   as posed would have sent a reader looking in the wrong crate.

3. **The four probes are all correct, and none was refutable.** The brief
   flagged them as "exactly the sort of thing that is cleanly refutable" and
   asked for them to be re-derived rather than trusted. They were, on a
   freshly built BEFORE binary, and all four reproduce verbatim including the
   exact message text and the missing location on variant 4. The brief's caution
   was right in method and its measurements were right in fact.

4. **The row's phrase "same fault, two stories" is right about the fault and
   wrong about the number**, but the brief's own instruction not to assume "same
   fault" turned out to be over-cautious in one direction: it IS one fault, and
   the evidence is that a single predicate (does this unresolved name spell a
   68000 register) placed at every consumer of one value (a Poison expression)
   closes all seven outcomes with no case analysis.

5. **Bar 5's shape does not fit this parcel's corpus result, and saying so is
   part of the finding.** It asks which classes moved, whether any rose or
   appeared, and for the symbol name sets in both directions, all of which
   presuppose a corpus that contains the fault. This one does not, and cannot:
   a register in value position is not something a working disassembly contains.
   The bar was answered literally (every figure above is measured, all zero) but
   the number that carries information is the injection, which the bar does not
   ask for. A corpus decomposition on a diagnostic-wording parcel measures
   inertness, and inertness needs an engagement witness beside it or it is
   indistinguishable from code that never ran.
