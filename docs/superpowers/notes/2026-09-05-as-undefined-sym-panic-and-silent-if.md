# Two faults on the undefined-symbol path: an internal error, and a silent wrong answer

2026-09-05 · branch `parcel/as-undefined-sym-panic-and-silent-if` · sigil master base `42633a51`

Two faults found while fixing the register diagnostic, both general rather than
register specific, both confirmed against a register-free control. Every number
below was produced by a command, and each is named at the number.

Reference asl throughout: `/home/volence/sonic_hacks/sonic_hack/tools/as/asl`,
md5 `61e672562465725a8c102288a7da9098`, `-cpu 68000 -L`, **exit status checked on
every invocation and quoted**. s2disasm's own asl (md5
`0dee1f98e6480a4783d27ffd8b90896f`) was not run at all.

---

## FAULT 1: an `if` on an undefined symbol emitted wrong bytes and exited 0

```asm
	if	Nowhere        ; Nowhere is never defined
	dc.l	$11111111
	endif
	dc.l	$22222222
```

| condition | sigil before | sigil after |
|---|---|---|
| `Nowhere` never defined | `22 22 22 22`, exit 0, **no diagnostic** | `probe.asm(2): error: unresolved if condition: ...`, exit 1 |
| `Nowhere equ 0` | `22 22 22 22`, exit 0 | `22 22 22 22`, exit 0 |
| `Nowhere equ 1` | `11 11 11 11 22 22 22 22`, exit 0 | unchanged |

An undefined condition was byte-for-byte indistinguishable from an explicit
false. A typo in a condition name removed code from the ROM and the build
succeeded.

### Root cause, one line

`crates/sigil-frontend-as/src/eval.rs`, `eval_if_expr`:

```rust
self.eval_all(toks, span).map(|v| v != 0).unwrap_or(false)
```

`false` is a verdict. `eval_all` returns `None` for an expression that folds to
Poison (an unresolved symbol) or does not parse, and the `if` answered anyway.

The sibling directives already refuse: `rept` says `unresolved rept count`
(eval.rs:3338), `while` says `unresolved while condition` (eval.rs:3626), `org`
says `org needs a constant expression` (eval.rs:4661). `if` was the one block
directive that answered without a basis, and it is the one whose answer decides
what code exists at all. The file's own `resolve_sym` doc comment had already
written the argument down, about `MOMCPU`/`TRUE`/`FALSE`: *"leaving it undefined
makes every `if notZ80(MOMCPU)` in that file read FALSE ... That is a wrong
branch, not a missing symbol, it emits bytes."*

### The fix

`eval_if_expr` and `eval_cond` return `Option<bool>`; `None` means the condition
produced no verdict. `exec_if` turns `None` into `report_unresolved_cond`, which
names the first symbol with no value and refuses. The arm is still skipped so the
pass has a shape to walk, but the run fails.

`report_unresolved_cond` fires at the site on EVERY pass, and only the CONVERGED
pass's diagnostics are returned by `run_impl` (eval.rs:208 and :241). That is the
same arrangement `rept` and `while` already use, and it is what makes a forward
reference legal.

### THE FORWARD-REFERENCE ANSWER, which is the thing that decided the design

**sigil accepts forward references in `if`; asl refuses them.** Measured both
ways, four shapes, probes archived beside this note:

| probe | shape | sigil before | sigil after | asl | asl exit |
|---|---|---|---|---|---|
| `fwd_equ` | `if Later` … `Later equ 1` later in the file | `11.. 22..` | `11.. 22..` | `expression must be evaluatable in first pass` | **2** |
| `fwd_label` | `if Later>0` … `Later:` later | `11.. 22.. 33..` | same | same refusal | **2** |
| `fwd_include` | `if Later` … `include` defining it later | `11.. 22..` | same | same refusal | **2** |
| `fwd_set` | `if Later` … `Later set 1` later | `11.. 22..` | same | same refusal | **2** |
| `back_include` | `include` BEFORE the `if` | `11.. 22..` | same | assembles | 0 |
| `if_undef` | never defined | `22 22 22 22` exit 0 | **refused** | `expression must be evaluatable in first pass` | **2** |

asl's rule is first-pass evaluatability, and it is STRICTER than what sigil now
does. Sigil resolves `if Later` by iterating passes to a fixpoint, so the name
has a value by the time the returned pass runs. **Refusing only what is still
unresolved at CONVERGENCE is strictly weaker than the reference, so it cannot red
a program asl accepts.** Sigil keeps the extra tolerance deliberately; five of
the fourteen new tests pin it.

That answer is what closed the BLOCKED question in the brief. A refusal keyed on
asl's own rule (first pass) WOULD have red legitimate code, four shapes of it.

### The corpus `if` population

`if_census.py`, archived beside this note, over four trees. Classes are the ones
the refusal can and cannot reach: `ifdef`/`ifndef` never fold a number and are
untouched; a `strcmp` condition is decided before any numeric fold is attempted;
`numeric` is the population the refusal can reach.

| tree | git | `.asm`/`.inc` files | `if`-family lines | numeric | strcmp | ifdef/ifndef |
|---|---|---|---|---|---|---|
| `s2disasm` | `e45ebf3` | 332 | 698 | 673 | 25 | 0 |
| `s1disasm` | `f6ece65` | 459 | 558 | 539 | 19 | 0 |
| `skdisasm` | `2fcd861` | 909 | 418 | 405 | 13 | 0 |
| `aeon` | `7f34863c` | 382 | 6257 | 3007 | 2375 | 875 |

The aeon row is the whole tree's `.asm`/`.inc`, which is an UPPER BOUND on what
reaches this front end, not a measurement of it: only three files are AS roots
(`engine/debug/debugger.asm` and both `games/*/game_root.asm`), and those three
carry **50** `if`-family lines themselves (24 numeric, 19 strcmp, 7
`ifdef`/`ifndef`). Their include closure was not enumerated.

### The corpus effect, decomposed

`corpus.sh`, archived beside this note. Plain `sigil s2.asm` in a detached
s2disasm worktree at `e45ebf33` (0 dirty paths), before and after binaries both
named by md5 in the run log.

```
before  exit=1  5243 diagnostic lines
after   exit=1  5254 diagnostic lines        +11
located: 5243/5243 before, 5254/5254 after   (every row names file(line))
```

| level | before | after | delta | class |
|---|---|---|---|---|
| error | 0 | 8 | +8 | `unresolved if condition: \`X\` has no value, ...` **APPEARED** |
| error | 0 | 1 | +1 | `unresolved elseif condition: \`X\` has no value, ...` **APPEARED** |
| error | 0 | 1 | +1 | `unresolved if condition: it does not evaluate, ...` **APPEARED** |
| error | 0 | 1 | +1 | `unresolved elseif condition: it does not evaluate, ...` **APPEARED** |
| | 5243 | 5254 | +11 | TOTAL |

**No class ROSE. No class went GONE.** Unresolved-symbol name sets, both
directions: before-only **empty**; after-only **{`MOMPASS`}**; in both, 8.
Diagnostic lines present before and absent after: **0**.

The eleven, by cause:

| n | cause | what it is |
|---|---|---|
| 7 | `MOMPASS` | an AS builtin sigil does not implement |
| 2 | `Snd_Sega.size` | downstream of `sound/PCM/generated/SEGA.inc`, absent from the tree and already reported as a `cannot include` |
| 2 | `address < *` | the `org` macro pair, `s2.macrosetup.asm(20)`/`(22)`, reached from `s2.asm(1963)`, the tree's only `org` macro call |

Every one of those is a condition sigil could not evaluate and was choosing an
arm for anyway. **Nothing that assembled before fails now: the corpus did not
assemble before** (exit 1, 5243 diagnostics). The refusal added eleven rows to a
run that already failed, and each row replaced a silent choice.

The `address < *` pair is the one I did NOT attribute. `*` as the program counter
works in isolation (probe `star_pc`), and the same macro shape with a
label-arithmetic argument works both plainly and nested inside
`if notZ80(MOMCPU)` (probes `orgmac`, `orgmac2`, `orgmac3`). Something specific
to that corpus invocation makes the substituted condition fail to parse; the
message says exactly that and does not guess a symbol. Both binaries fail to
evaluate it, so the difference between them is loudness, not correctness.

### One verdict per position

`cond_faults_seen` dedupes by span within a pass, mirroring `reg_faults_seen`.
The measurement that motivated it: without dedupe the same run printed **114**
rows, 81 of them one macro's `if MOMPASS=1` at
`sound/_smps2asm_inc.asm(258)` and 23 more from `s2.macros.asm(224)`, and the
eleven distinct positions were unreadable. With dedupe, 11 rows for 11 positions.
Both numbers are from the same corpus script, run before and after adding the
dedupe.

---

## FAULT 2: a `jsr`/`jmp` to an undefined symbol was an internal error, exit 101

```
thread 'main' panicked at crates/sigil-ir/src/lib.rs:424:21:
internal error: entered unreachable code: JmpJsrSym must be lowered by
resolve_layout before layout/link
```

`bsr.w Nowhere` in the same position reported `unresolved symbol ... for fixup`
and exited 1 throughout, so the control says this was specific to the
WIDTH-DEFERRED path.

### Root cause

Three stages, and the fault is in how they were joined.

1. On the convergence bonus pass (`eval.rs:216`, `defer_unresolved_jsr_jmp`) the
   AS front end turns a `jsr`/`jmp` whose bare-symbol target still folds to
   Poison into a `Fragment::JmpJsrSym`, because that is what a genuine cross-seam
   reference to a sibling `.emp` `pub proc` looks like and it is joined at LINK
   time.
2. `resolve_layout` is the stage that lowers it, and it already refuses an
   unresolvable target properly (`relax.rs:775`, `unresolved_abs_target_diag`).
3. The shipped `sigil <file.asm>` command **does not call `resolve_layout`**. It
   goes front end straight to `sigil_link::link`, and `link()` enforced the
   already-lowered contract with `unreachable!` (`lib.rs:159`, reached one line
   later than `Section::image_bytes`'s own at `sigil-ir/src/lib.rs:424`, which is
   why the panic names the IR crate).

Nothing between the deferral and the panic asked what happens when the symbol is
not cross-seam but simply absent.

### The visibility asymmetry, re-derived

The brief's table reproduced exactly, on the pre-change binary:

| | alone | with an unrelated error present |
|---|---|---|
| `jsr Nowhere` | exit 101, panic | exit 1, the unrelated error only, no panic |
| `if Nowhere` | exit 0, silent | exit 1, the unrelated error only, still silent |

The mechanism, which the brief did not name: an unrelated error makes the front
end return `Err`, so the CLI exits at `render_as_diags` and `link()` never runs.
**The panic was visible only when everything else in the file was correct**, so
it struck exactly the person closest to a working build. Both directions are
gated (`an_undefined_jsr_beside_an_unrelated_error_still_never_panics`).

### The fix, at the layer the contract lives on

`link()` gains a Pass 1c that walks every fragment for the three width-deferred
kinds (`JmpJsrSym`, `RelaxAbsSym`, `RelaxLadder`) and turns each into a
diagnostic, splitting the two causes the one `unreachable!` had merged:

* the target does not fold: name the symbol and the section, in
  `resolve_layout`'s voice, because it is an ordinary user mistake;
* the target DOES fold: the caller skipped `resolve_layout`, and the message
  says so.

It runs before `Section::image_bytes`, so the panic is unreachable from `link()`
by construction rather than by convention. The `unreachable!`s stay where they
are: `image_bytes` returns `Vec<u8>` and has no way to report, and its other
callers are internal (17 call sites, all tests and harness internals; `link()` is
the only user-facing one).

The CLI also keeps the front end's `SourceMap` past assembly and renders link
diagnostics through it, via a new `render_located_diags`. A link refusal used to
print a bare `error: ...` naming no file and no line; the span was there the whole
time and only the map was missing.

| probe | before | after |
|---|---|---|
| `jsr Nowhere` | exit **101**, `internal error: entered unreachable code`, no location | `root.asm(2): error: unresolved jmp/jsr target in section sec0 references symbol \`Nowhere\` not defined in this link`, exit 1 |
| `jmp Nowhere` | exit 101, same panic | same shape, exit 1 |
| `bsr.w Nowhere` (control) | `error: unresolved symbol \`Nowhere\` for fixup in section sec0 at offset 2`, exit 1, **no location** | same message, now `root.asm(2): error: ...`, exit 1 |
| `jsr Later` to a label defined later | `4E B8 00 04 4E 75`, exit 0 | **unchanged** |

asl on `jsr Nowhere`: `jsr_undef.asm(2):6: error: symbol undefined`, **exit 2**.
On `jsr Later` with `Later:` after it: `4EB8 0004`, exit 0. Reference and target
agree in both directions.

---

## RED-FIRST PROOFS, with the mutation shown applied on disk

Both fixes were committed FIRST, so every restore is from a committed baseline
rather than `git checkout --` over uncommitted work. `git checkout <rev> -- path`
STAGES the change, so `git diff HEAD --stat` is the one that reports it, and a
content grep is shown beside it.

### Fault 2, whole-fix mutation (the CLI rows)

```
$ git checkout 42633a51 -- crates/sigil-cli/src/main.rs crates/sigil-link/src/lib.rs crates/sigil-link/src/relax.rs
$ git diff HEAD --stat
 crates/sigil-cli/src/main.rs   |  31 ++-------
 crates/sigil-link/src/lib.rs   | 119 ----------------------------
 crates/sigil-link/src/relax.rs |   4 +-
 3 files changed, 7 insertions(+), 147 deletions(-)
$ grep -c "Pass 1c" crates/sigil-link/src/lib.rs            -> 0
$ grep -c "render_located_diags" crates/sigil-cli/src/main.rs -> 0
```

```
test result: FAILED. 2 passed; 3 failed
    jsr_to_an_undefined_symbol_is_a_located_diagnostic
    jmp_to_an_undefined_symbol_is_a_located_diagnostic
    bsr_to_an_undefined_symbol_stays_loud_and_gains_a_location
```

with the first one's output quoting the panic verbatim
(`internal error: entered unreachable code`, `left: Some(101)`). **What each row
MUST fail on:** the two rows that stayed green are the over-firing gate
(`a_forward_jsr_to_a_defined_label_still_assembles`) and the second direction of
the visibility asymmetry, which was never a panic. Both are supposed to pass on
both sides; that is what they are for.

### Fault 2, surgical mutation (the two link unit tests)

The whole-fix revert deletes the tests too, so a second mutation removed ONLY the
guard body from `link()` and left the tests in place. `.runlogs/mutate_link.py`
printed the 30 removed lines, and:

```
$ git diff HEAD --stat
 crates/sigil-link/src/lib.rs | 30 ------------------------------
$ grep -c "no_overlay" crates/sigil-link/src/lib.rs                             -> 0
$ grep -c "an_unlowered_jmpjsr_with_an_absent_target_is_a_diagnostic" .../lib.rs -> 1
```

```
test result: FAILED. 0 passed; 2 failed
```

both by panicking at `sigil-ir/src/lib.rs:424:21`. Restored, then re-run: 2
passed.

### Fault 1

```
$ git checkout 42633a51 -- crates/sigil-frontend-as/src/eval.rs
$ git diff HEAD --stat
 crates/sigil-frontend-as/src/eval.rs | 111 +++--------------------------------
$ grep -c "report_unresolved_cond" crates/sigil-frontend-as/src/eval.rs -> 0
$ grep -n "self.eval_all(toks, span).map(|v| v != 0)" .../eval.rs
3983:        self.eval_all(toks, span).map(|v| v != 0).unwrap_or(false)
```

```
test result: FAILED. 11 passed; 3 failed
    an_if_on_an_undefined_symbol_is_refused_and_names_it
    an_elseif_on_an_undefined_symbol_is_refused_and_names_it
    one_condition_reached_many_times_is_reported_once
```

The first row's failure message is the fault itself, printed:
`expected a refusal, the unit assembled to [22, 22, 22, 22] and said nothing`.
**What each row must fail on:** the eleven that stayed green are the over-firing
gates, which must pass on both sides. Restored from the commit, then re-run: 14
passed.

---

## ENGAGEMENT WITNESS

The last parcel's corpus table read "nothing moved", which was inertness rather
than a measurement. Here the corpus DOES contain instances of fault 1 (eleven
positions, above), so the corpus itself is the witness for that half. For fault 2
it does not, so the witness is injected and both binaries are run in the same
script, side by side:

```
== freshness witness: each binary answers the two parcel probes ==
  BEFORE if_undef : exit=0   : 22 22 22 22
  BEFORE jsr_undef: exit=101 : thread 'main' panicked at crates/sigil-ir/src/lib.rs:424:21:
  AFTER  if_undef : exit=1   : if_undef.asm(2): error: unresolved if condition: `Nowhere` ...
  AFTER  jsr_undef: exit=1   : jsr_undef.asm(2): error: unresolved jmp/jsr target ... `Nowhere` ...
```

Both binaries named by md5 in the same log (`77d8469c...` before,
`175875878a...` after). The exit status is captured from the binary, not from a
pipeline: the first version of this witness piped to `head` and reported `exit=0`
for a run that exits 1, which is a check that reads as a pass. Fixed and re-run.

---

## TEST-CONTRACT ENUMERATION, both directions

**No existing diagnostic's TEXT was changed.** Everything added is additive:
four new messages, plus a location prefix on CLI link diagnostics.

### Would red loudly (a full-string assertion on something I changed)

**None.** Searched for `"error: ` string literals and `starts_with("error`
/ `contains("error: ` across `crates/`: the only hits are
`sigil-harness/src/bin/refreeze.rs:1923` (a cargo error line, unrelated) and
`sigil-span/src/lib.rs:234` (`Diagnostic::to_string`, which this parcel does not
touch). No test asserts on the CLI's bare `error: ` prefix for a link
diagnostic, which is the one rendering that changed.

### Would stay GREEN while no longer testing what it names (the silent one)

Three tests match a substring that my NEW `link()` messages also contain:

* `crates/sigil-cli/tests/dplc_negative_probes.rs:204` -- `contains("unresolved branch/ladder target")`
* `crates/sigil-cli/tests/hblank_negative_probes.rs:243` -- `contains("unresolved symbolic absolute operand")`
* `crates/sigil-cli/tests/tranche2_negative_probes.rs:436` -- `contains("unresolved symbolic absolute operand")`

I reused `resolve_layout`'s `what` strings on purpose, so a user sees one voice
for one fault. **The hazard is closed by construction, and I checked it rather
than assuming it:** all three call `sigil_link::resolve_layout(...).expect_err(...)`
directly and never reach `link()`, so the string they match can only have come
from the stage they name. The discriminating tails, if a future test needs them:
`resolve_layout`'s message continues `, expected when compiling a cross-seam
module standalone; supply the map/harness composition that defines it`, and the
ladder one says `(symbol \`X\`)`; the `link()` Pass-1c message stops at `not
defined in this link`.

`eval_if_expr` and `eval_cond` changed signature; both are private to `eval.rs`
and have exactly two call sites, both in `exec_if`.

---

## WHAT DID NOT EXECUTE

* **The byte-identity golden gates did not run.** The suite was run with
  `SIGIL_ALLOW_PARTIAL=1`, and the harness's own count, quoted from its banner:
  *"127 test binaries are reference-dependent and every row in them will SKIP. A
  green result from this run does NOT mean those rows passed, it means they were
  not run."* **I am saying plainly that the byte gates did not execute, not that
  they passed.**
* **No aeon build.** Per the brief. TAGGED for the overseer: fault 1 makes the
  assembler refuse something it used to accept, and the residual risk it cannot
  close from here is an aeon `if` that (a) sigil cannot evaluate, (b) asl reads
  as false, and (c) emits no bytes either way, which would red the aeon build
  with no byte difference existing to have caught it earlier. The bounding facts
  I could establish read-only: `MOMPASS` appears in **0** aeon files; the three
  AS-routed roots carry 50 `if`-family lines. **The settling command is a strict
  landing run with `AEON_DIR` pointed at a provisioned tree, both ROM shapes
  built.**
* **No emulator.** Nothing here wants runtime confirmation.
* `MOMPASS` itself is NOT implemented by this parcel. Its value in a
  convergence-iterating assembler is a design question with byte consequences,
  and this parcel cannot run the byte gates. It is the headline defect the
  refusal exposed and it is left open deliberately, with 7 of the 11 corpus rows
  naming it.

---

## SUITE

```
CARGO_TARGET_DIR=<worktree>/.target-land SIGIL_ALLOW_PARTIAL=1 \
  cargo test --workspace --no-fail-fast
```

**4598 passed, 0 failed, 2 ignored, exit 0**, end marker written and found.
(4584 before the fault-1 tests were added, same shape.) No failing names to
report. See the caveat above about which rows were not measured.

---

## ANYTHING IN THIS BRIEF I CONCLUDED WAS WRONG

**1. Nothing in the brief's measurements was wrong.** Both faults reproduced
exactly as described, including the exit statuses (0, 101), the byte strings, the
panic text and file:line, the `bsr.w` control, and both rows of the visibility
asymmetry table. The asl refusal text and its exit status 2 also reproduced.

**2. The brief's reading of asl's rule was right, and probing it was still what
decided the parcel.** The brief said not to take its reading of asl's
first-pass rule as asl's behaviour. Probed: the reading was correct. But the
probe returned something the brief did not have, and it is the finding that
chose the design: **sigil is the MORE PERMISSIVE assembler here.** Sigil resolves
forward references in `if` by iterating to a fixpoint and asl refuses all four
shapes with exit 2. So the dangerous direction the brief was worried about had a
safe answer available: keying the refusal on convergence rather than on the first
pass makes it strictly weaker than the reference. Without that measurement the
defensible design would have been asl's rule, and asl's rule reds four
legitimate shapes.

**3. The brief's framing of the BLOCKED question needs one correction.** It asks
whether the refusal "reds legitimate corpus code", and treats a yes as BLOCKED.
The corpus answer is yes-and-no in a way the framing does not separate: the
refusal fires 11 times on s2disasm, and every one of those is legitimate AS that
asl accepts. But **s2disasm does not assemble under sigil today** (exit 1, 5243
diagnostics before the change), so nothing that worked now fails. What the
refusal actually did is convert eleven silent arm-choices into eleven loud ones
in a build that was already failing. I judged that not-BLOCKED and shipped it.
The thing that would be BLOCKED, and that I could not measure, is the aeon build,
which IS green today; that is the TAG above, and it is a real open item rather
than a formality.

**4. One thing the brief did not ask about, found on the way, out of scope.**
Operator precedence between comparison and `&&` diverges, and it is the same
silent-wrong-branch class:

```asm
K equ 3
J equ 4
	dc.l	(K*2)=6&&(J<>3)        ; asl: 00000000   sigil: 00000001
	dc.l	((K*2)=6)&&((J<>3))    ; asl: 00000001   sigil: 00000001
	dc.l	(K*2)=6                ; asl: 00000001   sigil: 00000001
	dc.l	6&&(J<>3)              ; asl: 00000001   sigil: 00000001
```

asl exit 0 for that file, so its values are usable. asl binds `&&` TIGHTER than
`=`, reading the first row as `6 = (6 && 1)` = `6 = 1` = 0; sigil binds `=`
tighter. The three parenthesised rows agree, which isolates it to precedence
rather than to any operator's own semantics. Corpus population is effectively
zero: the disassemblies parenthesise both sides of `&&`/`||` throughout. Probe
`prec.asm` is archived beside this note; this belongs in the gap ledger, not in
this parcel.

**5. The brief's own advice caught me once, at the check rather than at the
subject.** The dash sweep's first instrument was one `grep -c` for both codepoints,
spelled with a `\|` alternation inside a `$'...'` quote, under
zsh, which returned a confident **0** for a file that plainly contains five em
dashes. Re-run as two separate greps with a positive control (a file known to
hold 659), it returned the real counts. Every dash this parcel wrote was then
removed, verified against added-lines-only extracts of both commits plus a
control.
