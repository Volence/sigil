# The two biggest Sonic 2 complaint blocks are one unimplemented feature

**AS-S2-TOP-BLOCKS-DECOMPOSE: measurement only. Nothing under `crates/` was
touched, and this note proposes no change.** The last column of the
decomposition table is a *sizing note*: what a reader would have to build. It is
not a plan and not an approval to build it.

> Written under the 2026-09-05 owner ruling booked at `docs/OVERSEER.md`: no em
> or en dashes. That ruling landed on master at `bf236f10` while this parcel was
> running, so the two commits carrying this work were rewritten to conform.

The parcel exists because a complaint count is a count of a MESSAGE, not a count
of a DEFECT. This lane has been burned by that twice: a block booked at 492 sites
delivered 5,381, and a 2,624-row count was once attributed to the wrong
construct. So every figure below is a distinct-site count beside its row count,
and the residual is counted rather than named.

## Provenance

| | |
|---|---|
| sigil revision | `297dcd8f5763153b3154b79a1b2c378196e2e41c` (`crates/` clean, no source change) |
| sigil binary md5 | `46cf243340dd6f90d600a59e5d883d97` (`target/release/sigil`, built into a worktree-local `CARGO_TARGET_DIR`) |
| corpus | `s2disasm` at `e45ebf332f39987424ca3102e50c717628f71269`, detached worktree `/home/volence/sonic_hacks/.s2-decompose`, `git status --porcelain` empty |
| command | `cd <corpus> && <bin>/sigil s2.asm` (no flags) |
| result | exit 1, **5,761 rows** over **5,113 distinct sites** |
| wall clock | 11:12:56 to 11:13:02 EDT, 2026-09-05: **6 s** (build: 15.57 s) |
| reference assembler | `s1disasm/build_tools/Linux-x86_64/asl`, md5 verified `61e672562465725a8c102288a7da9098` before each use. The `s2disasm` build (`0dee1f9...`) was never invoked. |
| re-run | `docs/superpowers/notes/2026-09-05-s2-top-blocks-decompose-probes/run.sh <sigil-bin> <corpus> <out-dir>` reproduces every number here, and prints the md5 and both revisions before it prints a count |

Every one of the 5,761 rows parses as `file(line): error: ...` and every
`(file, line)` resolves to a real source line: 0 unparsed, 0 unresolvable. There
is no diagnostic cap in this output and no suppression line.

## The class totals, beside the ones the brief quoted

| class | brief quoted | measured today | delta |
|---|---|---|---|
| `bad operand expression` | ~2,624 | **2,624 rows / 2,602 sites** | **0** |
| `expected mnemonic, directive, or label` | ~2,309 | **2,309 rows / 2,309 sites** | **0** |
| whole-run total | 9,432 (`2026-09-04-s1-path-to-rom.md:400`) | **5,761** | -3,671, mostly explained |

The two subject classes did not move at all. The whole-run drop of 3,671 is
**mostly** explained by one class named in that same note: it recorded 3,384
`unresolved symbol in operand` rows, and today that class is **24**, a fall of
3,360. The remaining **311** is **not explained here**. Attributing it would mean
re-running the 2026-09-04 binary, which this parcel did not do. Do not read the
311 as anything but unexplained.

Full class table at this revision, rows and distinct sites:

| rows | sites | message |
|---|---|---|
| 2624 | 2602 | `bad operand expression` |
| 2309 | 2309 | `expected mnemonic, directive, or label` |
| 518 | **1** | `bad absolute address expression` |
| 89 | 89 | `` `X` is not a recognized 68000 mnemonic `` |
| 49 | **1** | `bad word expression` |
| 39 | 39 | `cannot include <file>` |
| 30 | **1** | `bad byte expression` |
| 24 | 24 | ``unresolved symbol `X` in operand`` |
| 23 | **2** | `int(): could not evaluate float expression` |
| 18 | 18 | ``unknown directive or mnemonic `X` `` |
| 11 | **1** | `unexpected character` |
| 8 | 8 | `instruction needs an explicit size suffix` |
| 6 | 6 | `case needs a string literal` |
| 4 | 4 | `malformed number` |
| 3 | 3 | ``bad displacement expression in `disp(An)` `` |
| 2 | 2 | `switch needs a string expression` |
| 2 | 2 | `trailing tokens in operand` |
| 1 | 1 | ``struct `X` has a member line this cannot read`` |
| 1 | 1 | `unsupported form: <insn>` |

The third, fifth, seventh and eleventh largest classes are **one site each**.
That is the parcel's thesis with no analysis required: 518 rows of
`bad absolute address expression` are one line, `s2.macrosetup.asm:304`.

## The decomposition

### RC-1: AS's nameless temporary labels (`+`, `-`, `/`) are not implemented

**One root cause. Four message classes. 4,985 of the run's 5,761 rows (86.5%),
over 4,915 reported coordinates and 5,003 source constructs.**

| # | sub-shape | message class | reported sites | rows | source constructs | example |
|---|---|---|---|---|---|---|
| a | definition: a lone `+` / `-` / `/` in column 1 | `expected mnemonic, directive, or label` | 2309 | 2309 | 2309 | `s2.asm:90822`, `-\tmove.b\t#0,(a5)+` |
| b | reference: a bare `+`...`+++` / `-`...`---` as a branch or `dbf` operand | `bad operand expression` | 2599 | 2599 | 2599 | `s2.asm:90846`, `\tbne.s\t+` |
| c | reference written *inside* a macro body (`nosignpost`) | `bad operand expression` | 1 | 11 | 1 body, 11 calls | body `s2.asm:6120`, `\tbeq.ATTRIBUTE\t+\t; rts`, called from `s2.asm:6128` to `6138` |
| d | reference passed as a macro **argument** to `_beq` / `_bne` | `bad operand expression` | 2 | 14 | 14 calls | body `s2.macrosetup.asm:256`, `\tbpl.ATTRIBUTE\tx`, called at `s2.asm:5560`, `\t_beq.s\t+\t; rts` |
| e | reference passed as a macro **argument** to `offsetTableEntry` | `bad word expression` | 1 | 49 | 49 calls | body `s2.macros.asm:162`, `\tdc.ATTRIBUTE ptr-current_offset_table`, called at `s2.asm:75728`, `\toffsetTableEntry.w +\t; 0` |
| f | reference as the displacement in `disp(PC,Xn)` | ``bad displacement expression in `disp(An)` `` | 3 | 3 | 3 | `s2.asm:9410`, `\tmove.b\t++(pc,d2.w),d1` |
| g | reference on a line whose *definition* already failed, so it **emits no row at all** | *(none)* | 0 | 0 | 18 | `s2.asm:497`, `-\tdbf\td0,-` |
| | **RC-1 total** | **4 classes** | **4915** | **4985** | **5003** | |

**Sizing note (not a plan, not a fix).** This is one feature in the AS front end,
not 4,985 fixes. It is absent, not partial (see H2 below), and it fails at two
independent emit sites, which is the whole reason it reads as two headline
classes:

* `crates/sigil-frontend-as/src/eval.rs:2697`: the statement dispatcher requires
  `Tok::Ident` in head position and errors otherwise. Sub-shape (a) dies here.
* `crates/sigil-frontend-as/src/operands.rs:345`: `parse_expr` on a bare operand.
  Sub-shapes (b) through (e) die here or in the `dc` / `disp(An)` paths beside it.

What a reader building it would have to cover, from the shapes actually present:
recognition of a lone `+`/`-`/`/` in label position; a position-ordered anonymous
symbol table (references reach depth 3, both `+++` and `---` occur); recognition
of a run of `+` or `-` in *expression* position, not only as a branch target;
survival of macro **argument** substitution (63 rows arrive that way) and of a
literal in a macro **body** (11 rows); and the parenthesized forms `(+)` and
`(++)`, one occurrence each, both as `offsetTableEntry` arguments. A regex over
bare tokens misses those two, which is how they were nearly lost from this count.
No cost in hours is given: costing it means reading and changing the parser,
which this parcel forbids.

**A prohibition, not a caveat: `5,761 - 4,985 = 776` is NOT the post-fix count,
and must not be quoted as a prediction of it.** Closing RC-1 resolves symbols
that are currently unresolved and lets sigil reach code it currently abandons,
which can remove rows *and* add them. The only way to know the number after is to
measure after.

### The residual

| class | rows | attributed to RC-1 | **residual** |
|---|---|---|---|
| `bad operand expression` | 2624 | 2624 | **0 rows / 0 sites** |
| `expected mnemonic, directive, or label` | 2309 | 2309 | **0 rows / 0 sites** |

Both subject classes are **entirely** one cause. There is no misc bucket, because
there is nothing in it.

How that was established rather than assumed:

* Class B's source lines carry exactly three distinct first tokens across all
  2,309 rows: `+` (1983), `-` (312), `/` (14). Nothing else.
* Class A's 2,610 direct rows carry 29 distinct mnemonics, all branches or
  `dbf`/`dbeq`, and 18 distinct operand tails, every one a bare nameless token
  (`+`, `++`, `+++`, `-`, `--`, `---`, or the `dN,-` forms). Zero false positives.
* Class A's 14 remaining rows sit at two macro-body lines whose operand is the
  macro parameter `x`. The corpus contains exactly 12 `_beq` and 2 `_bne` calls
  whose argument is a bare nameless token: 12 and 2, matching the two sites' row
  counts exactly.

### Attribution of the other one-site blocks (checked, and **not** RC-1)

Every site in the run that emits more than one row was inspected. There are nine.
Four are RC-1 (sub-shapes c, d, e). The other five are different causes, recorded
so nobody re-attributes them later:

| site | rows | class | construct |
|---|---|---|---|
| `s2.macrosetup.asm:304` | 518 | `bad absolute address expression` | `jmp (extractJmpToName("op")).l`, a symbol name built by a string function inside `irp` |
| `s2.sounddriver.asm:3817` | 30 | `bad byte expression` | `music_metadata`, a user `function` (`withinSameZ80Bank`) over struct-member fields |
| `s2.sounddriver.asm:3905`, `s2.asm:87677` | 23 | `int(): could not evaluate float expression` | already booked as F1 |
| `s2.macros.asm:246` | 11 | `unexpected character` | `zoneanimcount_{"\{zoneanimcur}"}`, AS's `{...}` name composition |

## Verdict on H1: ONE cause, and it is bigger than H1 supposed

H1 asked whether the two classes share one root cause, with the mechanism "one
message at the reference site, a different message at the definition site."

**They share one root cause, and that mechanism is right as far as it goes, but
it is four classes, not two.** H3 (causes cross class boundaries) is the operative
fact here, not a caveat on it.

The evidence that settles it is a controlled probe, not an inference from the
corpus. `...-probes/nameless_shapes.asm` contains one instance of each corpus
shape. sigil refuses all of them and produces all four messages from the one
missing feature:

```
nameless_shapes.asm(15): error: expected mnemonic, directive, or label    <- a '/' definition
nameless_shapes.asm(18): error: bad operand expression                    <- 'bne.s -'
nameless_shapes.asm(7):  error: bad operand expression                    <- macro body, arg was '+'
nameless_shapes.asm(23): error: expected mnemonic, directive, or label    <- a '+' definition
nameless_shapes.asm(10): error: bad word expression                       <- 'dc.w ptr-Base', arg '+'
nameless_shapes.asm(10): error: bad word expression                       <- same body, arg '(+)'
nameless_shapes.asm(28): error: bad displacement expression in `disp(An)`  <- '+(pc,d0.w)'
nameless_shapes.asm(29): error: expected mnemonic, directive, or label     <- a '+' definition
sigil exit=1
```

The reference `asl` (md5 `61e672562465725a8c102288a7da9098`) assembles the same
file with **exit 0 and no diagnostic**, and, because a probe whose answers all
coincide proves nothing, its four values are distinct and non-degenerate: `66FA`
(backward branch to the `/`), `6702` (a forward branch resolved through a macro
argument), `0016` (a `dc.w` difference), `0002` (a PC displacement). A probe using
one target address could not have told a correct implementation from a broken one.

### Neither cited note is wrong, but one is incomplete

* `docs/superpowers/notes/2026-09-04-as-macro-default-params.md:201`: "AS's
  anonymous relative labels ... over 2,602 distinct sites". **Reproduces exactly.**
  Small addition: 2,602 is the count of *reported coordinates*, and three of them
  are macro bodies standing in for 25 call sites, so the construct count behind
  that class is 2,624, which happens to equal the row count because each of those
  25 calls emits exactly one row.
* `docs/superpowers/notes/2026-09-04-s1-path-to-rom.md:400`: "2,309 of them are
  the nameless `+`/`-` local labels". **Also reproduces exactly**, and is also
  correct. But it books only the *definition* class. The same construct is also
  the whole of `bad operand expression` (2,624), the whole of `bad word
  expression` (49) and the whole of `bad displacement expression` (3). Reading
  that line as "the nameless-label cost is 2,309" understates it by a factor of
  2.2: the real footprint is 4,985 rows, 86.5% of the run.

So the answer to "which note is wrong" is **neither**. The two notes were
describing the two ends of the same construct and neither said so.

## Verdict on H2: ruled, and **not started**, rather than half-implemented

Decision `d-22` (`docs/decisions.jsonl:25`, 2026-09-03) ruled *accept nameless
labels in the AS-compatibility surface*, on the ground that the corpus is
unassemblable without them. Two days later, **zero of the 5,003 assembled
nameless-label constructs in `s2.asm` are accepted**. Not a subset: none.

That was checked against the corpus, not inferred from the diagnostic stream:

* **Definitions.** `s2.asm` contains 2,337 lines whose column-1 token is a lone
  `+`/`-`/`/`. 2,309 are reported. The 28 that are not were each read: all 28 sit
  inside `if fixBugs` (and similar) arms that are not assembled, for example
  `s2.asm:3148`, `+\tlea (Underwater_palette+4).w,a1`, inside the `if fixBugs` at
  3140. There are **zero** reported definitions that are not real ones.
* **References.** 2,709 lines match a bare nameless operand. 2,600 are reported
  directly; 47 are `offsetTableEntry` calls reported at the macro body instead;
  14 are `_beq`/`_bne` calls likewise; and the remaining 48 sit in unassembled
  conditional arms (`s2.asm:84639`, `71112` and `12196` were each read in context
  to confirm). 2600 + 47 + 14 + 48 = 2709, exactly.

The brief's pointer to `crates/sigil-frontend-as/src/eval.rs` "mentioning" them is
a false lead worth killing: the six matches there are about *anonymous `ds.b`
reserve fields* and about a guard stopping `irp` over an empty item from
accidentally defining a nameless label. None of them is nameless-temporary-label
support. A reader following that pointer would go looking for a partial
implementation to finish and find nothing.

## Why the ratio is the finding

The parcel asked for rows beside sites because the ratio is where the deception
lives. In this corpus it deceives in **both** directions at once:

* **Rows overstate the work.** 74 rows come from 4 coordinates (sub-shapes c, d
  and e). One line, `s2.macros.asm:162`, is a whole message class. Anyone sizing
  `bad word expression` at "49 things to fix" would be sizing 1.
* **Rows understate the population.** 18 nameless-label *references* produce no
  row at all, because the definition on the same line failed first and the
  statement never reached its operand. `s2.asm:497`, `-\tdbf\td0,-`, is one line
  carrying both halves of the construct and yielding exactly one message.

Hence three different honest numbers for the same cause, and a report that gives
one of them without saying which is a report that can be off by 4x:

| | |
|---|---|
| message rows | **4,985** |
| distinct reported coordinates | **4,915** |
| source constructs in assembled code | **5,003** |

## Anything in the brief I concluded was wrong

1. **"Do not trust either number."** Right as method, wrong as prediction. Both
   reproduced to the digit: 2,624 and 2,309, delta zero. The classes are frozen.
   What moved between 2026-09-04 and today was `unresolved symbol in operand`
   (3,384 down to 24). The distrust cost nothing, but a reader should know the two
   quoted figures are current, not stale.
2. **"If they are not both right, one of those notes is wrong and I want to know
   which."** Both are right. There is no note to correct and no withdrawal owed.
   What there is instead is an *incompleteness*: `s1-path-to-rom.md:400` books the
   definition half only, and reading it as the nameless-label total understates by
   2.2x. That is a different defect from a wrong attribution and wants a different
   fix, an addition rather than a retraction.
3. **H1's mechanism is right but too small.** "Two headline classes, one cause" is
   true; the actual shape is **four** classes, one cause. The brief offered H3
   (causes cross class boundaries) as a caution against assuming a partition. Here
   H3 is not a caution, it is the main result, and the two classes it names *are*
   a clean partition of coordinates (zero overlap), which is exactly the
   observation that would have made a lazier decomposition stop early.
4. **H2 leans the wrong way.** "A half-implemented state" implies something to
   finish. Nothing is implemented; the ruling has not been started. The practical
   difference is real: half-implemented means find and fix the gaps, absent means
   design the feature.
5. **The `eval.rs` pointer is a false lead** (see H2 above). It names a file whose
   matches are all unrelated.
6. **The reference-assembler warning applied, but not for the reason offered.**
   Method note 6 warns that a stable `asl` value can be an artifact of source
   order for shapes it *declines*. Nameless labels are not such a shape, so that
   trap did not bite. A different one did, and it is worth writing down: my first
   version of the shapes probe contained one invalid line, `bra.s /`. (`/` is a
   definition-only form in AS; it is referenced with `+` or `-`, never written as
   an operand.) `asl` rejected that one line with `wrong number of operands`, and
   **an unrelated value elsewhere in the same file changed**: the macro-expanded
   `beq.s +` came back `67FE`, a branch to itself, instead of the correct `6702`.
   Both runs printed a listing that looked complete. **An `asl` run with any error
   in it is not a source of values for the lines that did assemble**, and the
   digest guard does not protect against that. It selects the right build, not a
   clean run. Check the exit status too.
7. **One thing the brief got exactly right and worth keeping.** "A path is not a
   state": the corpus was pre-verified clean and stayed clean (`git status
   --porcelain` empty at both ends), and the isolated `CARGO_TARGET_DIR` meant no
   other lane's pinned `target/release/sigil` was relinked. Neither cost anything
   and both are re-checked by `run.sh` on every future run.

## What I could not measure

* **The post-fix diagnostic count.** It requires implementing RC-1, which this
  parcel forbids. See the prohibition above: 776 is not it.
* **The remaining 311 rows of the 3,671 whole-run delta** against the 2026-09-04
  figure. Attributing it needs the 2026-09-04 binary, which was not rebuilt.
* **Whether the 76 nameless-label constructs inside unassembled conditional arms**
  (28 definitions and 48 references) would report if `fixBugs` were on. They were
  read and classified by eye, not measured, because flipping a corpus assembly
  option changes the corpus.
