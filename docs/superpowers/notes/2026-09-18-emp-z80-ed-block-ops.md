# The 21 Z80 mnemonics `.emp` could not spell

Branch `parcel/emp-z80-ed-block-ops`, off master `515be271`.

## What was missing

`.emp`'s Z80 mnemonic table ended at `ldir`. The AS front end's table carried 21
names beyond it, all of them already present and already ENCODED in the shared
ISA crate. An author writing a Z80 routine in `.emp` simply had no way to say
them; a routine ported between the two source languages changed instruction
because one reader had no name for it.

Re-derived rather than taken from the brief: extracting both tables and diffing
gives 67 names on the AS side, 46 on the emp side, difference exactly

    ldi ldd lddr cpi cpd cpir cpdr ini ind inir indr outi outd otir otdr
    in out reti retn rrd rld

with no emp-only name in the other direction. After the change both tables
extract to the SAME 67-name set (diff empty both ways).

## The design half: `(c)`

The port operand had no `.emp` representation. `CodeOperand::Z80IndC` is new,
and is kept DISTINCT from `Z80Mem` on purpose: `in a,(c)` is the 2-byte ED form
and `in a,(n)` the 2-byte unprefixed form, so collapsing them would be a wrong
answer of the same length that reads as plausible.

**It is recognised only under `in`/`out`**, mirroring the AS front end's own
`"c" if matches!(m, In | Out)`. This is what makes the spelling byte-neutral
rather than merely new: `c` is an ordinary name everywhere else, so an
unconditional arm would retarget any existing `(c)` meaning "the address the
const `c` holds". Unlike the neighbouring condition-code rule it is NOT
position-conditioned, because the port is operand 1 of `in r,(c)` and operand 0
of `out (c),r`.

Surface spellings implemented:

| form | `.emp` |
|---|---|
| C-addressed port | `in a, (c)` / `out (c), a` |
| direct port | `in a, (127)` / `out (127), a` |
| everything else | the bare mnemonic (`ldi`, `otdr`, `reti`, `rld`, …) |

The direct port takes a BARE numeral, which is `.emp`'s own absolute-address
spelling (`ld a, ($4000)`). The brief proposed `(#$7f)`; `#` inside an indirect
is not an `.emp` operand form at all, so the bare numeral is what the language
already reads. Hex works in `.emp`; the tests use decimal only because one
snippet string drives BOTH front ends and `$` is the program counter in the AS
source language.

## Byte-neutrality: why, not just whether

Three independent arguments, because the sweep alone would only say that
nothing in the corpus noticed.

1. **By construction.** `z80_mnemonic` returning `None` makes lowering
   `push_err("… is not a recognized Z80 mnemonic")` and emit nothing
   (`lower/code.rs`). So no program that assembles today can contain any of the
   21. Adding them cannot change one. The `(c)` operand is reachable only under
   `in`/`out`, and the two analyzer fixes are reachable only under `in` — all
   three therefore inherit the same argument.
2. **The exposure the instruction argument misses.** The 21 words become
   RESERVED in mnemonic position (`is_recognized_mnemonic`), so a `comptime fn`
   named one of them would stop being callable in bare statement position inside
   a Z80 proc, and would newly draw a `[name.shadows-mnemonic]` warning at its
   declaration. Scanned the 7262-file aeon corpus: no `fn`/`proc`/`comptime fn`
   is named like any of the 21. The identical loop finds 36 and 53 files for two
   control names, so the zero is a real zero and not a broken command. Note the
   failure mode if it ever happens is a LOUD error, never different bytes.
3. **The sweep, run as an A/B.** A green workspace suite was not available to
   compare against: the aeon main tree is live-edited, so 169 reference-reading
   tests in `sigil-cli`/`sigil-harness` already fail against it at master
   (`repin_pins`: "442 pin values moved", `ASSEMBLED_LEN` Δ -0x284). A sweep
   that merely reported those would have said nothing about this parcel. So the
   whole workspace was run TWICE under one `AEON_DIR` — once at the branch tip,
   once with `crates/sigil-frontend-emp` alone reverted to `515be271`:

   | | suites | passed | failed |
   |---|---|---|---|
   | baseline (crate reverted) | 492 | 5422 | 169 |
   | parcel | 493 | 5430 | 170 |

   The failing TEST-NAME SETS are identical, 159 names each, except for one:
   `version_reports_the_head_of_the_tree_it_was_built_from`. That one is an
   artifact of this session, not of the parcel — the docs commit landed WHILE
   the first sweep was in flight, so the binary's baked revision (`bbaf22e4`)
   was one commit behind the checkout's HEAD (`f2f14b78`). The test names that
   possibility itself and says to re-run to distinguish; re-run against a stable
   HEAD it is 19 passed, 0 failed.

   So: the same 169 pre-existing reference-tree failures on both sides, and +8
   net passes (nine new tests, less the one transient above).

## The analyzer holes

Both were self-documented, each with a comment asserting a premise this parcel
falsifies, and NEITHER was found by reading around — each came from a gate going
red.

* **`z80_cycles`** said the `(c)` forms were "12 T on the machine and are NOT
  priced here, because they have no spelling to price". Mapping `IsaOp::IndC` in
  the coverage test's `emp_image` turned `every_encodable_form_is_priced` red on
  exactly four forms out of 272 checked, 0 skipped. Priced at 12 T (Zilog
  UM0080: 3 M cycles of 4+4+4, against the direct port's 4+3+4).

* **`flag_check::z80_zero_role`** had `In` as an unconditional Z writer, its own
  comment noting that `IN A,(n)` "affects no flag, and the `.emp` operand model
  spells neither form". The second clause stopped being true here, which made
  the first clause a live defect. Split by operand shape.

  Worth being explicit that this was not a hole on the safe side of a polarity.
  `FlagRole::Writes` ends the must-use window (a genuinely unread result goes
  unreported) AND answers `may_change` (the invalid-edge diagnostic fires on
  correct code). Wrong in both directions.

Checked and found already complete: `z80_preserves` names all 21 with correct
write sets; `flag_check`'s CARRY role covers all 21 through an exhaustive match
on the ISA enum; `context.rs` already counts `reti`/`retn` as terminators;
`cycle_budget` already treats the eight repeating block ops as split-cost. A
grep for these names as STRINGS returns zero in `flag_check.rs` and says nothing
at all about its coverage — the match is over the enum.

## Left open

`z80_preserves::z80_flag_neutral` treats `in a,(n)` as a flag writer. That is
wrong on the manual, but it sits on that module's DECLARED conservative side (an
unmodelled mnemonic over-fires visibly rather than false-passing a
`preserves(f)`). Correcting it would LOOSEN a preserve proof, which is a
risk-bearing change that does not belong in a mnemonic-surface parcel.

## Why the tests are shaped the way they are

Expectations are DERIVED, never typed: each snippet is one string assembled
through BOTH front ends and the byte strings required to agree. For this
instruction group a typed table fails silently — the ED block ops are a 4x4 grid
at `ED A0 | family | direction << 3 | repeat << 4`, so transposing the axes
still lands on a legal member.

Cross-frontend agreement alone would not catch a collapse both tables shared, so
the grid is ALSO required pairwise distinct, and the `(c)` forms are exercised on
both `a` (register code 7) and `b` (code 0) so the register shift is not left
unexercised.

**That grid test earned its place immediately.** The first draft of the new table
block dropped the pre-existing `"ldir" => Ldir` arm while inserting the fifteen
around it. `ldir` has a live population in the sound-driver corpus, so that would
have broken assembling programs — and the cross-frontend case table does not
cover `ldir`, deliberately, since this parcel is about what was MISSING. Only the
grid test, which covers the whole 4x4 including the members that already worked,
spelled it. Generalisable: when a parcel extends a family, gate the WHOLE family,
not the delta.
