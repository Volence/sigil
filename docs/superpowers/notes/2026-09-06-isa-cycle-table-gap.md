# ISA-CYCLE-TABLE-GAP: pricing every Z80 form the encoder can emit

Branch `parcel/isa-cycle-table-gap`, off master `f67428a0`.
Commits `ce059de8` (the prices and the coverage guard) and `5deb95af` (the two
off-table fixtures that the pricing invalidated).

## The derived set, and how it was derived

The brief framed the gap as "21 previously-unencodable mnemonics the cycle table
does not price". That is a real number about a real thing, 21 mnemonic
spellings were added to the AS front end's map by the coverage parcel `9c813864`
and its predecessor `c2eca83d`, but it is not the size of the gap. The cycle
table was the driver-demand subset from the day it was written, so the
unpriced-but-encodable set was always much larger than whatever the last parcel
happened to add.

The set was not read off a list. It was measured, by running the new coverage
guard against master's cost table:

1. `crates/sigil-frontend-emp/src/z80_cycles.rs`'s new
   `tests::encoder_coverage` module was lifted verbatim into a temporary,
   uncommitted integration test (`tests/tmp_derive_gap.rs`), which reaches
   `instr_cost` through the crate's public API rather than through `super`.
2. `git checkout f67428a0 -- crates/sigil-frontend-emp/src/z80_cycles.rs
   crates/sigil-frontend-emp/src/eval/builtins.rs` put master's table back on
   disk. That command STAGES, so `git diff --stat` reports nothing; the mutation
   was witnessed with `git diff HEAD --stat` (770 lines removed from
   `z80_cycles.rs`, 12 from `builtins.rs`) and with a content grep showing
   `OpaqueCall` count 0 in both restored files, against a canary grep showing
   `Cost` still present 40 times in the same file.
3. The guard ran and went red.

The measurement:

```
320 encodable Z80 form(s) resolve to Cost::Unknown, out of 417 checked
(12 skipped as `(c)` forms; mnemonics with no encoding at all: [LdIA, LdRA])
```

320 forms over **51 distinct mnemonics**:

```
adc add and bit call cp cpd cpdr cpi cpir dec di ei ex im in inc ind indr ini
inir jp ld ldd lddr ldi ldir neg or otdr otir out outd outi pop push res rl rlc
rld rr rrc rrd sbc scf set sla sra srl sub xor
```

Two caveats on the 320, both stated because the number would otherwise flatter
the finding:

* `neg` and `ldir` account for 152 of the 320 between them, because the ENCODER
  has catch-all `(Mnemonic::Neg, _)` and `(Mnemonic::Ldir, _)` arms that accept
  any operand list at all. Excluding those two mnemonics the figure is **168
  forms**. Both numbers are true; 168 is the one to quote when the question is
  "how much real instruction surface".
* 417 is the number of forms the pool DISCOVERED, not the number of Z80 forms.
  It is bounded by the operand-shape pool, which is deliberately over-inclusive
  but is not the whole operand space.

So the brief's "21" was a count of the wrong population, and the twenty-one were
a minority of it. Everything in the list above is now priced except where a
reason is recorded below.

## Each cost, with its source

Every count is the Zilog Z80 CPU User Manual (UM0080) T-state figure from its
instruction-set tables, and each group cites its section at the arm in
`z80_cycles.rs`. Grouped as UM0080 groups them:

**8-Bit Load Group.** `ld r,r'` 4; `ld r,n` 7; `ld r,(hl)` / `ld (hl),r` 7;
`ld a,(bc)` / `ld a,(de)` and the stores 7; `ld (hl),n` 10; `ld a,(nn)` /
`ld (nn),a` 13; `ld r,(ix+d)` / `ld (ix+d),r` 19; `ld (ix+d),n` **19** (four
bytes and five M-cycles, yet the same 19 as the register form, the operand byte
rides an M-cycle the displacement fetch already needs); `ld a,i` / `ld a,r` /
`ld i,a` / `ld r,a` **9** (not the 8 an ED pair usually costs).

**16-Bit Load Group.** `ld dd,nn` 10; `ld ix,nn` 14; `ld hl,(nn)` 16;
`ld dd,(nn)` 20; `ld ix,(nn)` 20; `ld (nn),hl` 16; `ld (nn),dd` 20;
`ld (nn),ix` 20; `ld sp,hl` 6; `push qq` 11; `pop qq` 10; `push ix` 15;
`pop ix` 14.

**Exchange, Block Transfer and Search.** `ex de,hl` 4; `ex af,af'` 4; `exx` 4;
`ex (sp),hl` 19; `ldi` / `ldd` / `cpi` / `cpd` 16; `ldir` / `lddr` / `cpir` /
`cpdr` 21 repeating / 16 final.

**8-Bit Arithmetic and Logical.** All eight base ops priced by SOURCE shape,
because UM0080 gives them one row per source: register 4, immediate 7, `(hl)` 7,
`(ix+d)` 19. `inc`/`dec r` 4; `inc`/`dec (hl)` 11; `inc`/`dec (ix+d)` 23.

**General-Purpose Arithmetic and CPU Control.** `daa` / `cpl` / `ccf` / `scf` /
`nop` / `halt` / `di` / `ei` 4; `neg` 8; `im 0` / `im 1` / `im 2` 8.

**16-Bit Arithmetic.** `add hl,ss` 11; `adc hl,ss` 15; `sbc hl,ss` 15;
`add ix,pp` 15; `inc`/`dec ss` 6; `inc`/`dec ix` 10.

**Rotate and Shift.** `rlca` / `rrca` / `rla` / `rra` 4; CB `rlc`/`rrc`/`rl`/
`rr`/`sla`/`sra`/`srl` on `r` 8, on `(hl)` 15; `rld` / `rrd` 18.

**Bit Set, Reset and Test.** `bit b,r` 8, `bit b,(hl)` **12**, `bit b,(ix+d)`
**20**; `set`/`res b,r` 8, `set`/`res b,(hl)` **15**, `set`/`res b,(ix+d)`
**23**. `bit` only reads; `set`/`res` write the byte back.

**Jump, Call and Return.** `jp nn` 10; `jp cc,nn` 10; `jp (hl)` 4; `jr e` 12;
`jr cc,e` 12/7; `djnz` 13/8; `call nn` **17**; `call cc,nn` 17/10; `ret` 10;
`ret cc` 11/5; `reti` / `retn` 14; `rst p` 11.

**Input and Output.** `in a,(n)` 11; `out (n),a` 11; `ini` / `ind` / `outi` /
`outd` 16; `inir` / `indr` / `otir` / `otdr` 21 repeating / 16 final.

Previously-priced values were left exactly where they were. `reti`/`retn` at 14
in particular: they were priced BEFORE they could be assembled, and the encoder
caught up to the price rather than the price moving.

## Deliberately left `Unknown`, with reasons

Three form classes are unpriced, and each says so at its arm. In all three the
reason is the same in kind: **the assembler cannot emit those bytes**, so a cost
would be a number with nothing behind it. Not one of them is unpriced because
the T-state count is in doubt, the counts are known and are given here.

1. **The `(c)` port forms**, `in r,(c)` and `out (c),r` (12 T). This one is not
   even a choice: `CodeOperand` has no variant for the C-addressed port, so no
   `.emp` instruction can carry that operand shape to the cost table, and no
   match arm could be written that would ever fire. The coverage guard asserts
   that this is the ONLY reason any form is skipped.
2. **The DDCB shift column**, `rlc (ix+d)` and its six siblings (23 T).
   `encode_cb_shift` requires an `r` or `(hl)` target, so the encoder refuses
   them. The commit that added the coverage encodings named the index page as
   deliberately left, with zero corpus sites.
3. **The index-page forms the encoder does not reach**, `jp (ix)` (8 T) and
   `ex (sp),ix` (23 T). Same reason: `encode_index` has no arm.

## The coverage test

`crates/sigil-frontend-emp/src/z80_cycles.rs`, `tests::encoder_coverage::
every_encodable_form_is_priced`. It is an INLINE `#[cfg(test)]` module in an
existing file rather than a new `crates/*/tests/*.rs`, so it does not need the
nightly source-gate lane's classification.

**The population is asked of the encoder, not declared.** Every `Mnemonic`
variant is offered every operand shape in a fixed pool; the pairs `z80::encode`
ACCEPTS are the population. Nothing states what is encodable.

**The vocabulary cannot silently lose a member.** A `mnemonics!` macro takes the
spelling list once and expands it into two things: the vector the test walks,
and an exhaustive `match` on the encoder's own `Mnemonic` enum. A variant added
later fails to COMPILE. The ISA-operand-to-emp-operand image is an exhaustive
`match` on the ISA operand enum for the same reason.

**Three non-empty guards**, because an empty population and an unapplied
mutation both print `ok`:

* the pool must hold at least 40 shapes;
* the discovered form set must be non-empty AND must contain an anchor from each
  encoding page, `nop` (base one-byte), `ld` (base two-operand), `srl` (CB),
  `ldir` (ED block), `in` (ED I/O), `push` (16-bit), and `rst` and `im`, whose
  operands only encode in range at all. A pool that had lost every CB or ED
  shape would still be non-empty; it would not still have those anchors;
* the number of forms actually CHECKED must exceed zero, and the failure message
  reports it against the number skipped.

**Skipping is bounded.** A form is skipped only when it has no emp operand
image, and the test asserts that every skipped form contains `(c)`. Anything
else skipped would be the demand shrinking silently.

### Red-first proof

Three runs, each with the mutation shown applied on disk before it was run.

**(a) Against master's table**, the derivation above. Restored from the
committed baseline `f67428a0`, witnessed with `git diff HEAD --stat` and a
content grep (not `git diff --stat`, which reports nothing because `git checkout
<rev> -- <path>` stages). RED, naming 320 forms.

**(b) One arm deleted from the landed table.** The `push`/`pop` arm was removed
by a script that asserts its target string is present first, so a mutation that
failed to apply would have aborted rather than run the original file and printed
`ok`. Witnessed on disk: `git diff HEAD --stat` = 1 file changed, and a grep for
`"push", [Z80Pair` returning 0 against a grep for the mutation marker returning
1. RED, naming exactly the 12 push/pop forms and nothing else, out of the same
417 checked:

```
12 encodable Z80 form(s) resolve to Cost::Unknown, out of 417 checked
  push [Pair(Bc)] … push [Pair(Iy)]
  pop  [Pair(Bc)] … pop  [Pair(Iy)]
```

**(c) The non-empty guard itself.** `shape_pool` was mutated to return an empty
vector, again with a presence assertion on the target string. RED with
`the operand-shape pool has 0 entries; a shrunken pool makes this guard pass by
examining almost nothing`, which is the point: the empty population is a
failure here, not a pass.

After each, the file was restored with `git checkout HEAD -- <path>` and the
suite re-run green.

## Two consumer changes that fall out of the prices

**`call nn` is 17 T, and `span_cost` must not sum through it.** The instruction's
own cost is known, but the callee is not in the slice, so a straight-line sum
would state a true T-state count of less code than actually runs. `span_cost`
gains `CycleBail::OpaqueCall`, keyed on the same `context::is_call_mnemonic` the
budget walk uses, so both consumers agree on what a call is. This also closes a
hole that predates the parcel: `rst p` was already priced at 11 and already
summed silently.

**The eight repeating block ops give `[cycles.ambiguous-branch]` its first real
input.** That variant's doc asserted it had none, on the strength of a counted
enumeration of split-cost TERMINATORS. A block repeat is a split-cost
NON-terminator presenting one edge, because it re-executes itself instead of
branching, so it lands on the `two_way` guard. Refusing is correct, the true
cost is `16 + 21*(BC-1)` with `BC` a run-time value, and charging either number
to the single edge would be wrong by up to 21 T per iteration, of which there
may be 65 535. The doc, the inline comment and the crate-side edge sweep are
corrected, and the sweep gains all eight repeats as a third polarity.

## Landing run, failures first

Run twice, and both are recorded because a verdict belongs to the tip it was
measured on. `5deb95af` is the last CODE tip; `90cd287e` adds this note and the
comment punctuation fix and moves no logic. Both GREEN at the same 4683.

```
pwd    /home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a44f1861f39ea8d03
HEAD   5deb95af  (code tip)      GREEN, 4683 passed / 0 failed, cargo 0, clippy 0
HEAD   90cd287e  (note + prose)  GREEN, 4683 passed / 0 failed, cargo 0, clippy 0
```

The commit carrying this line is not itself named above, for the obvious reason;
it changes this file only.

FAILING TESTS: none, in either run.

```
  tree            …/agent-a44f1861f39ea8d03 @ 5deb95af (parcel/isa-cycle-table-gap, clean)
  reference       /home/volence/sonic_hacks/.aeon-ref @ 483b3e12 (HEAD, clean), all four present
  started/ended   2026-09-06T05:10:31Z -> 2026-09-06T05:15:05Z (UTC)
  CARGO_EXIT      0
  CLIPPY_EXIT     0   (lint bar clean)
  suites          412
  passed          4683
  failed          0
  ignored         2
  skip lines      0
  reconciles      4670 baseline + 13 new = 4683 observed
  RESULT          GREEN
```

The 13 new tests are twelve in `z80_cycles.rs` (ten discriminating cost tests,
`call_and_rst_bail_opaque_in_a_span`, and the coverage guard) and one in
`cycle_budget.rs` (`a_block_repeat_is_an_ambiguous_branch_not_an_unknown_op`).
The renamed pin in `z80_new_mnemonic_write_sets.rs` is a rename, so it is net
zero.

The FIRST landing attempt was RED, at `ce059de8`, with two failures:
`an_off_table_op_is_refused` and `cycles_off_table_bails`. Both used `ldir` as
their off-table fixture and the pricing invalidated them. Recorded here rather
than quietly fixed, because both had ALREADY been moved once from `rlca` for the
same reason and both carried a comment saying to keep the fixture a real unpriced
op, advice that cannot survive a pass whose purpose is to price everything real.
The fixture is now `rlc (ix+0)`, which cannot go stale a third time: the coverage
guard is keyed on what the encoder accepts, and the encoder does not accept it.

## Why the test cases discriminate

Most Z80 instructions cost 4, 7, 8 or 11 T, so a transposed table entry agrees
with a right one by coincidence more often than not. Each new test names the
mistake it separates from:

* `inc` answers **4, 6, 11, 23 and 10** on five operand shapes. A table that
  treated a mnemonic as an instruction would give one of them five times.
* `bit b,(hl)` is 12 where `set`/`res b,(hl)` are 15, and `bit b,(ix+d)` is 20
  where `set`/`res` are 23, but all three agree at 8 on the register column,
  which is where a casual test would look. The register column is asserted too,
  so the test cannot pass by only exercising the axis that agrees.
* `add hl,ss` is 11 where `adc`/`sbc hl,ss` are 15: the ED prefix is worth 4, and
  `add hl,ss` is the only unprefixed member. Copying its 11 across the family
  would be wrong three times and look internally consistent.
* `push` is 11 where `pop` is 10, the one stack pair that is NOT symmetric.
* `ld hl,(nn)` is 16 where every other pair is 20, which is the same asymmetry
  that makes the encoder never emit ED 63 / ED 6B.
* `in a,(0FEh)` is 11 where `ld a,(00FEh)` is 13: two instructions that differ
  only by mnemonic over an identical operand shape.
* `rlc b` 8 / `rlc (hl)` 15 / `rlca` 4, three numbers, none reachable from
  another by a plausible slip.
* The block grid is asserted as two whole FAMILIES with distinct shapes
  (`Fixed(16)` against `Split { 21, 16 }`), never one member, because its members
  are one bit apart in two independent axes.

## Anything in this brief you concluded was wrong

**1. "The block instructions (sixteen of them) carry two costs, repeat and
final."** Only eight do. `ldi`, `ldd`, `cpi`, `cpd`, `ini`, `ind`, `outi` and
`outd` do not repeat: they step once and stop, at a flat 16 T. Giving all sixteen
a `Cost::Split` would have made BOTH consumers refuse an instruction whose cost
is a constant, `span_cost` with `[cycles.ambiguous-branch]` and the budget walk
with the same, since a single-step block op also presents one edge. That is a
refusal manufactured out of nothing, on eight instructions the model can price
exactly. The eight that DO repeat are `Split { taken: 21, not_taken: 16 }`.

**2. "21 previously-unencodable mnemonics ... `z80_cycles.rs` prices none of
them."** The 21 is correct as a count of what the coverage parcel added, and the
brief was right to say I should not inherit it unverified. Verified: 21 spellings
were added to the AS front end's mnemonic map. But the population the parcel
asked me to close is not that one. Measured against master's table, the
unpriced-but-encodable set is 320 forms over 51 mnemonics (168 forms excluding
the two mnemonics whose encoder arms accept any operands), and the twenty-one
are a minority of it. `push`, `pop`, `ex`, `scf`, `ei`, `di`, `call nn`,
`add hl,ss`, the entire CB rotate and bit group, every `(nn)` pair load and
store, `inc (hl)`, `inc (ix+d)`, and the eight-op ALU family in every source
shape but register-and-immediate were all unpriced and all encodable long before
that parcel ran.

**3. "Then a test that fails if any ENCODABLE mnemonic resolves to `Unknown`."**
Mnemonic-level coverage is too weak to hold this, and following it literally
would have produced a guard that passed over most of the gap. `inc` was already
priced, at 4 and 6, while `inc (hl)` and `inc (ix+d)` were not, and they are
11 and 23. A mnemonic-level test sees `inc` as covered. The guard is FORM-level:
every accepted `(mnemonic, operand shape)` pair must price. That is what turned
up 320 rather than a few dozen.

**4. "reti/retn are ALREADY priced at `Cost::Fixed(14)` (`z80_cycles.rs:191`)."**
The fact is right and the line number was right at the time of writing, which I
checked rather than assumed: `git show f67428a0:.../z80_cycles.rs | sed -n
'191p'` is that arm. It is at line 385 after this parcel. Recorded only as the
general point that a line number in a brief is a fact about one revision.

**5. Not wrong, but the brief's framing hid it.** "A wrong cycle count is worse
than an absent one" is what motivated leaving the 21 unpriced, and it is right.
But pricing `call nn` correctly at 17 would have made `span_cost` sum through a
call and produce a wrong SPAN count from an entirely right instruction count.
The dangerous direction was not in the table at all; it was in a consumer whose
contract the new price silently broke. `rst` was already in that state before
this parcel. The guard against it is `CycleBail::OpaqueCall`, and it had to be
added in the same commit as the price.
