# `eval` — asl's processor-neutral SET, and the column rule that came with it

`eval` is 8,211 of Sonic 1's 9,739 diagnostics: 84.3% of the baseline, from 59
source lines. It lands in this parcel, and so does a column rule that was not
part of the row as written.

## Provenance

| | |
|---|---|
| sigil | branch `parcel/as-eval` off master `ed06da77` |
| S1 corpus | `s1disasm` `f6ece657`, detached worktree, entry `sonic.asm`, `sigil sonic.asm` from the corpus root |
| S2 corpus | `s2disasm` `e45ebf3`, detached worktree, entry `s2.asm` |
| oracle | `s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098`, `-xx -n -q -A -L -U -E -i .` |

**S1's own binary was used for every S1 claim.** It is upstream AS; S2's copy is
the flamewing fork, md5 `0dee1f98e6480a4783d27ffd8b90896f`, behind an identical
version string. `-U` is on every invocation.

A checkable fact about the two builds: regenerating the 181 pre-existing
`snippets_golden.txt` blocks is a **git-clean no-op under both** — they agree
across that whole corpus, so the difference between them is narrower than the
version string suggests.

**Which S1 tree state was measured.** `sound/dac/*/generated/*` are gitignored
build artifacts and a bare checkout lacks them. Seeding only the four `.inc`
files is not enough — they `BINCLUDE` the `.pcm`/`.dpcm` raw data beside them,
and a tree with the `.inc` but not the raw data reads **9,743**, four
`cannot BINCLUDE` above the real figure. All of `*.inc`, `*.pcm`, `*.dpcm` and
`hashes.lua` were copied in, which reproduces **9,739** exactly.

## What `eval` is

AS's `SET` assigns a redefinable symbol. On a Z80 `set` is a real bit
instruction (`set 3,a` ⇒ `CB DF`), so a sound include shared between CPUs cannot
spell it that way — `EVAL` is the processor-neutral name, and it is what the
disassemblies write.

They are one directive over one symbol class, not two similar ones:

```
       5/     100 : =$3                  b       set 3
       6/     100 : =$4                  b       eval 4
```

and the type check is shared — `a equ 1` followed by `a eval 2` raises the same
`#2030 constants cannot be redefined as variables` that `a set 2` does.

Both spellings, and both are in the corpus:

| form | asl | corpus sites |
|---|---|---|
| `NAME eval VALUE` (label column) | `=$5` | 3 |
| `eval NAME,VALUE` (operand column) | `=$A` | 68 |

**`eval` carries no CPU gate.** sigil's `set NAME,VALUE` dispatch arm is gated to
the 68000 so the Z80 `set BIT,(ix+d)` still routes to instruction lowering.
`eval` needs no such gate and must not have one: asl assembles an indented
`eval j,9` under `CPU Z80` (`=9H` in the listing) while `set 3,a` on the next
line is still `CB DF`. Both `eval` spellings work under both CPUs.

The operand form takes **two** operands. A third is a segment name, not a value
— `eval f,1,2` is `#1961: unknown segment` pointing at the `2`. The corpus
writes no three-operand `eval`; sigil refuses one loudly.

## The column rule — the part the row did not predict

The colon-less `NAME eval VALUE` form requires NAME in asl's **label field**:

| line | asl |
|---|---|
| `i eval 5` at column 0 | assigns, `=$5` |
| `\ti\teval 5` indented | `#1200 unknown instruction` — `i` is the mnemonic |
| `\ti:\teval 5` indented, colon | assigns, `=$5` |

sigil's pre-existing `set` intercept had no such gate, and adding `eval` to it
made that matter. `eval` and `set` are **ordinary symbol names to asl** — a label
called `eval`, referenced as `dc.b eval&$FF`, emits its low byte. That line
arrives at the intercept as head `dc.b` with `eval` in the second token, and
ungated the intercept reads it as the label-column form: it assigns a symbol
named `dc.b` and emits **nothing at all — no bytes, no diagnostic, exit 0**.

The guard is `body[0].span.start == line.base`, mirroring the column rule the
bare-label fallback seventeen lines below already applies. The colon arm keeps no
gate, because asl peels a colon label at any indentation.

The same guard closes the `set` twin, which was wrong on master for the same
reason: an indented `\ti\tset 5` bound `i` where asl reports `#1200`.

## Before and after — both corpora, every class

Sonic 1, `9,739 → 1,367`. Two classes moved; nothing else changed by one count,
and no class appeared.

| counts before | after | class |
|---|---|---|
| 8,939 | 728 | `X` is not a recognized 68000 mnemonic |
| 161 | **0** | operand out of range -128..=255 |
| 497 | 497 | unresolved symbol in operand |
| 36 | 36 | bad operand expression |
| 25 | 25 | unresolved long expression |
| 18 | 18 | unexpected character |
| 18 | 18 | instruction needs an explicit size suffix |
| 14 | 14 | bad word expression |
| 8 | 8 | unresolved rept count |
| 6 | 6 | case needs a string literal |
| 6 | 6 | bad immediate expression |
| 4 | 4 | trailing tokens in operand |
| 2 | 2 | unsupported form: ccr is not a general EA |
| 2 | 2 | switch needs a string expression |
| 1 | 1 | the corpus's own `error` text (`_Variables.asm(430)`) |
| 1 | 1 | unknown directive or mnemonic |
| 1 | 1 | org target precedes the current phase base |
| **9,739** | **1,367** | |

`9,739 − 8,211 − 161 = 1,367`, no remainder. The 8,211 is the whole of `eval`:
the unrecognized-head histogram loses that row entirely and every other head
keeps its exact count. The 161 is baseline cause 4, `dc.b ALLARGS` inside
`smpsDcb`, which the baseline predicted was downstream of `eval` and is.

Sonic 2, `13,109 → 9,625`, the same two classes and no others:
`13,109 − 3,417 (eval) − 67 (out of range) = 9,625`. S2's numbers remain true for
S2; the corpus worktree lacks its 60 gitignored generated sound includes, and
those 60 `cannot include` rows are constant across before and after.

Set-level, compared **both directions**:

- unresolved-symbol NAME sets: 86 before, 86 after on S1 (291/291 on S2), **no
  name added or removed in either corpus**.
- distinct `file(line)` sites: S1 675 → 615, S2 8,660 → 8,600. **Nothing only in
  after** — no new ground broke.
- every output line matches `file(line): severity: …` in all four captures. No
  unclassified remainder.

## The silent half — what was searched

The count falling is not the result. `eval` newly *assembling* means lines that
used to refuse now emit bytes, and the question is whether they are the right
bytes.

**1. The whole corpus's FM voice data, against asl, per file.** Every one of the
59 music and SFX files that defines voices was extracted with the corpus's own
voice macro block, assembled by **both** assemblers, and compared with `cmp`.

```
files identical=59  differing/failed=0   voices covered=162   bytes compared=4025
```

Not vacuous, on both counts that matter: no output was empty (25–825 bytes per
file), and running the identical sweep with the **pre-fix** binary gives
`identical=0 differing/failed=59`.

**2. A third, independent oracle agrees.** The disassembly annotates each voice
with its expected bytes. GHZ Voice `$00`:

```
;   $08
;   $0A, $70, $30, $00,  $1F, $1F, $5F, $5F,  $12, $0E, $0A, $0A
;   $00, $04, $04, $03,  $2F, $2F, $2F, $2F,  $24, $2D, $13, $80
```

sigil and asl both produce exactly that. These 25 bytes are the product of
nothing but `eval` accumulators.

**3. A 27-case matrix, asl vs sigil, exit code and emitted bytes.** Both
spellings on both CPUs, decorative colon, case folding (`EVAL`/`Eval`),
reassignment chains, string values, macro-local cursors, `rept` and `while`
cursors, column-0 placement, `eval` as a label name, a user macro named `eval`,
the `!eval` forced-builtin escape, and the arity failures. **26 match.** The
single divergence is pre-existing and is booked below.

**4. Whether the Z80 silent-label class is next door — it is, and it was hit.**
The S1 baseline found that under `CPU Z80` an unrecognized bare head is silently
bound as a label. `eval` sits in exactly that territory: it is not a mnemonic on
any target, so under Z80 an indented `eval j,9` would have been bound as a label
and assigned nothing. That is why the dispatch reach is widened to every CPU
rather than left on the 68000 branch.

And the hunt paid: the column-rule defect in the section above **was found this
way, not by the count**. The count was already correct — 1,367 either way. It was
found by a probe asking what a line that is *not* an assignment does, and its
signature was the silent one: exit 0, no diagnostic, a byte quietly missing.

Where `eval` sits in the corpus: all 71 lines are in `sound/_smps2asm_inc.asm`,
included from `s1.sounddriver.asm:2639` under `CPU 68000` — every one of the
8,211 diagnostics said `68000`. No `eval` line in S1 is reachable under `CPU Z80`
today.

## Gates

Twelve `t6_eval_*` blocks in `crates/sigil-frontend-as/tests/snippets_golden.txt`,
bytes generated by real asl. The regeneration is a pure append — 152 lines added,
**none removed** — so every pre-existing golden is untouched.

`crates/sigil-frontend-as/tests/as_eval_directive.rs`, five `#[test]`s, carrying
what the golden corpus cannot because asl *refuses* those inputs and the
generator only records successes.

Red-first, both mutations applied from disk and shown applied:

| mutation | `as_eval_directive` | `asl_snippets` |
|---|---|---|
| none | 5 passed, 0 failed | ok |
| **M1** `eval.rs` reverted to `ed06da77` (13+/56−) | **3 FAILED**, 2 passed | **FAILED** |
| **M2** the `name_in_label_field` guard deleted (1+/3−) | **3 FAILED**, 2 passed | **FAILED** |

Every test reddens under at least one mutation; the two that survive M1 are the
column-rule pair, whose mutation is M2, and the test file says so. Both restores
were from the committed baseline and left the tree clean.

One gate was **applied-and-still-green** on the first attempt and was chased
rather than accepted: `eval_operand_form_needs_exactly_a_name_and_a_value`
matched on the substring `` `eval` ``, which the
`` `eval` is not a recognized 68000 mnemonic `` of a no-eval build also contains.
It now requires the directive's own arity text.

## Booked, not done

- **`equ` then `set`/`eval` is not refused.** asl raises `#2030 constants cannot
  be redefined as variables`; sigil reassigns silently and emits the new value.
  **Pre-existing** — the pre-fix binary does the same for `a equ 1` / `a set 2`.
  `eval` only gives it a second spelling. Closing it needs symbol-class tracking
  (constant vs variable), which is its own parcel.
- **A column-0 bare label named `set`.** asl binds it; sigil dispatches it as the
  directive and reports `` `set` directive expects `NAME, value` ``, because
  `set` is in the Z80 mnemonic table. **Pre-existing and `set`-only** — `eval` is
  in no mnemonic table, so the `eval` twin works (`t6_eval_is_an_ordinary_symbol_name`
  pins both, the `set` half with its colon).
- **The three-operand `eval NAME,VALUE,SEGMENT`** is refused rather than
  implemented. Loud, not silent; zero corpus sites.

## What this leaves at the front of the S1 queue

With `eval` and its downstream gone, the 1,367 remaining diagnostics are led by
`irpc` (615 counts on one line), the `STRUCT` family (~499), and 728 unrecognized
heads in total. `s1.sounddriver.asm`'s Z80 section stays under-measured for the
reason the baseline gave — its six diagnostics are a floor, and that is still a
soundness question rather than a count.
