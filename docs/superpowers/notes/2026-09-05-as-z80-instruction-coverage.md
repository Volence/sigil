# AS-Z80-INSTRUCTION-COVERAGE

Branch `parcel/as-z80-instruction-coverage`, from master `c38b44fd`.

The row read: *"22 sound-processor instructions we never encoded, now visible
rather than silently swallowed. One is used, a block-copy at 17 sites in Sonic
2's driver."* Two of its three clauses turned out to be wrong in ways the work
had to correct; see the last section.

---

## 1. The enumerated set, and how it was enumerated

**Not from a list of mnemonics. Against the opcode space.** A list of mnemonics
is a list of what I happened to remember, and a gap it omits is invisible; an
opcode page can be checked for holes.

`.probe/forms.py` emits 612 legal Z80 forms by walking the base, CB and ED
pages by OPERAND SHAPE (every `ld r,r'`, every ALU op crossed with every source,
every CB family crossed with every target, every ED register/pair field), plus a
representative DD/FD sweep. `.probe/enum.sh` then assembles each form ONE PER
asl INVOCATION and runs each through the front end, producing a TSV of
`form / asl-exit / asl-bytes / sigil-exit / sigil-bytes`.

`.probe/analyze.py` reads that back two ways.

**(a) Coverage of the opcode space, which is the completeness instrument.** If
the form list forgot an instruction, its opcode shows up here as
unaccounted-for:

| page | legal opcodes produced | unaccounted for |
|---|---|---|
| base | 252 / 252 | none |
| CB | 248 / 256 | `30`..`37` |
| ED | 56 / 80 | 24, listed below |

The CB `30`..`37` column is the undocumented shift. **The reference asl refuses
it under every spelling tried** (`sll`, `sli`, `sl1`, `slia`, `sls`, `swl`; all
exit 2, no bytes), so it has no asl answer and cannot be encoded from one. It is
therefore NOT part of any count of "instructions we could have encoded".

The 24 unaccounted ED opcodes are `4C 4E 54 55 5C 5D 63 64 65 66 6B 6C 6D 6E 70
71 74 75 76 77 7C 7D 7E 7F`. Every one is an undocumented duplicate or a form
asl spells differently: the `neg` mirrors (`4C/54/5C/64/6C/74/7C`), the `retn`
mirrors (`55/5D/65/6D/75/7D`), the `im` aliases (`4E/66/6E`, `76`, `7E`), the
two no-ops (`77/7F`), `63`/`6B` which asl emits as the base `22`/`2A`, and
`70`/`71` (`in f,(c)` / `out (c),0`) which this asl refuses outright. So the
documented instruction set is fully covered.

**(b) The gap.** Of the 612 forms, asl accepted 602 and the front end refused
**55 of them**, across 28 leading mnemonics. Decomposed:

**Twenty documented mnemonics with no encoding at all** (the count in the row
was 22; see section 6):

```
ldd  lddr  cpi  cpd  cpir  cpdr        ini  ind  inir  indr
outi outd  otir otdr                   reti retn  rrd  rld
in   out
```

plus **`ldi`**, which is the twenty-first and is the one with a live population.

**Six operand forms of mnemonics that were already partly encoded:**

| form | why it was missed |
|---|---|
| `sbc hl,rr` (4 pairs) | `add hl,rr` is unprefixed base 09; the carry-aware siblings are ED and were never written |
| `adc hl,rr` (4 pairs) | same |
| `im 0`, `im 2` | only mode 1 was in the original catalog scope |
| `ld a,i`, `ld a,r` | the WRITE direction (`ld i,a`) was encoded and the READ direction was not |

**Deliberately left, and why.** The index page only. Re-running the identical
612-form sweep against the finished binary leaves **8 refused forms across 4
mnemonics**, down from 55 across 28, and every survivor is DD/FD:

```
jp (ix)   jp (iy)        ex (sp),ix   ex (sp),iy
rlc (ix+10)  rlc (iy+10)     ld sp,ix   ld sp,iy
```

`rlc (ix+10)` is the interesting one: `bit 1,(ix+10)` and `set 1,(iy+10)` encode
today, so the DDCB group is half built. All eight have zero sites in either live
tree, all sit on a different encoder path (`encode_index` / `encode_cb_shift`),
and none is worth growing the surface for on speculation. Recorded here so the
next person does not have to re-derive the list.

**The same sweep is also a full-ISA differential**, and it comes back clean: of
the 602 forms asl accepts, the number where both assemblers accept and the bytes
DIFFER is **0**, before and after.

**Live population** (`.probe/sites.sh`, mnemonic-column anchored, canary-checked
against `ldir`/`ret` so a zero could not be a broken grep):

| mnemonic | s2disasm `e45ebf33` | .aeon-ref `483b3e12` |
|---|---|---|
| `ldi` | **17**, all in `s2.sounddriver.asm` | 0 |
| `ldir` | 11 (10 driver, 1 `s2.asm`) | 0 |
| `sbc hl,rr` | **1**, `s2.sounddriver.asm:4023` | 0 |
| every other name in the set | 0 | 0 |

aeon's zeroes are a property of the tree and not of the grep: `.aeon-ref` holds
three `.asm`/`.inc` files and none declares `cpu z80`. Its Z80 lives in eleven
`.emp` files, which route through the emp front end whose mnemonic table this
change does not touch.

---

## 2. asl provenance, with exit statuses

Reference assembler `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
**md5 `61e672562465725a8c102288a7da9098`**; `p2bin` beside it, **md5
`4f2fff99c3347bafb93b12d5be1db754`**. Selected through
`docs/superpowers/notes/asl-reference/asl_ref.sh`, sourced with `|| exit $?` at
every call site. The `s2disasm` copy was never invoked.

**The exit status was checked at every call site, and measured to discriminate
before being trusted:**

| input | asl exit |
|---|---|
| `frobnicate` (garbage mnemonic) | **2** |
| `ldir` | **0** |
| `in a,(100h)` (port out of range) | **2**, `range overflow` |
| `im 3` | **2**, `instruction not supported on Z80` |
| `sll a` and five other spellings | **2** |

So a nonzero exit is a real refusal on this build and not a constant.

**One invocation per form** in the enumeration sweep, which is what makes the
exit status meaningful: this build substitutes the last value it computed for an
operand it declines, so a run carrying any error is not a source of values for
any line in it. A single batched file would have let one refused line poison
neighbours that assembled correctly.

**The committed golden minting.** `gen-z80-vectors` asserts `status.success()`
per snippet, so a declined form aborts the mint rather than writing a
substituted byte. Two witnesses:

1. **Instrument witness.** Re-minting the golden **before touching anything**
   reproduced the committed file **byte for byte** (`git diff` empty). So the
   reference asl is demonstrably the build that answered for the vectors already
   in the tree, and the diff after each change is exactly the new lines.
2. **Diff shape.** `ldi` commit: **1 line added, 0 modified**. Remainder commit:
   **32 lines added, 0 modified**. No existing vector moved.

Every added vector, and its independent confirmation from the 612-form sweep
(the two agree line for line):

```
ldi => ED A0     cpi => ED A1     ini => ED A2     outi => ED A3
ldd => ED A8     cpd => ED A9     ind => ED AA     outd => ED AB
ldir => ED B0    cpir => ED B1    inir => ED B2    otir => ED B3
lddr => ED B8    cpdr => ED B9    indr => ED BA    otdr => ED BB
retn => ED 45    reti => ED 4D    rrd => ED 67     rld => ED 6F
im 0 => ED 46    im 1 => ED 56    im 2 => ED 5E
in b,(c) => ED 40      out (c),b => ED 41
in a,(c) => ED 78      out (c),a => ED 79
in a,(0FEh) => DB FE   out (0FEh),a => D3 FE
sbc hl,bc => ED 42     sbc hl,sp => ED 72
adc hl,bc => ED 4A     adc hl,sp => ED 7A
ld i,a => ED 47   ld r,a => ED 4F   ld a,i => ED 57   ld a,r => ED 5F
```

---

## 3. Why these probe cases discriminate

An opcode table is dense with confounds, and this instruction group is the worst
case for a spot check: almost every new instruction has a legal neighbour one
bit away. A test that asserts one exemplar per family passes under a table that
swapped the family's two axes.

**The ED block grid is `ED A0 | family | direction << 3 | repeat << 4`.**
Transpose the two axes and every answer is still a legal block op. So the test
asserts **all sixteen simultaneously AND requires them pairwise distinct**. The
pairwise clause is the half a per-instruction assertion cannot do: a table that
collapsed two neighbours onto one opcode passes every individual check that
happens to look at the survivor.

**`in r,(c)` puts the register in bits 5..3 and the direction in bit 0, and
register A is code SEVEN.** A family exercised only on `b` (code 0) passes with
the register field ignored entirely, because `0 << 3` is `0`. All seven
registers are asserted, both directions.

**`im` is ED 46 / 56 / 5E, which is NOT `0x46 | mode << 4`.** That arithmetic
gives 46 / 56 / **66**, and ED 66 is a genuine undocumented `im 0` alias, so a
table built from the pattern rather than from asl emits bytes a disassembler
reads back as an interrupt-mode instruction. This is the exact shape of the
hex-versus-decimal divergence that cost this lane months: a wrong answer that
looks like a right one.

**`sbc hl,rr` and `adc hl,rr` differ in bit 3 with the pair in bits 5..4** --
the same two-axis confound. All eight are asserted.

**`ld i,a` (ED 47) and `ld a,i` (ED 57) differ in bit 4.** Direction slip swaps a
store for a load with no other symptom, so both directions of both registers are
asserted together.

**The direct port literal is `0FEh` and not a single digit.** A value whose hex
and decimal readings coincide cannot tell a radix error from a correct one, and
a value of zero cannot tell an emitted operand from a dropped one.

**Two refusal tests were rewritten because they passed for the wrong reason.**
`a_port_above_255_is_refused` passed while `in` was not a mnemonic at all, and
`an_out_of_range_interrupt_mode_is_refused` passed back when EVERY mode but 1
was refused. Each now asserts the in-range companion assembles in the same test,
so neither can pass without the boundary it names being the boundary enforced.

**Red-first, with the reason shown.** `ldi` first: red with
`unknown directive or mnemonic \`ldi\``, which is the behavioural reason and not
a compile error. Then the other nine tests: **9 failed, 1 passed** (the already
landed `ldi` one). After implementation: 10 passed.

---

## 4. The cycle table, and the other consumers

**`z80_cycles::instr_cost` defaults to `Cost::Unknown`, the loud direction.**
Every mnemonic this work made assemblable lands there, `ldir` included, and is
**left there**: a wrong T-state count is worse than an absent one, and pricing
belongs to whichever timed region first needs it with its own derivation. Pinned
by `the_new_mnemonics_are_unpriced_in_the_cycle_table`, with `nop` = `Fixed(4)`
as the positive control so a lookup that answered Unknown for everything could
not pass.

**TWO EXCEPTIONS, and they went the other way.** `reti` and `retn` were
**already priced at 14 T** in that table with no encoder arm and no name
mapping. That is the same half-built shape `z80_missing_primitives.rs` documents
for the eight one-byte primitives: the analyzers knew about instructions the
encoder could not write. The encoder has now caught up to the price. 14 T is
Zilog's number for both, so nothing was corrected, only connected.

**`z80_preserves.rs` is the consumer that could NOT be left alone,** and it is
the correctness finding of this parcel. Its write-set match defaults to
`vec![]` -- *writes nothing* -- which is the UNSAFE direction for a preserve
proof: an instruction that is assemblable but unlisted is claimed to preserve
every register it destroys, silently. Its `ldir` arm carried the kill condition
in its own comment. So an arm lands in the SAME commit as the encoder that makes
the mnemonic assemblable, never after it:

- `ldi`/`ldd`/`ldir`/`lddr` -> `{b,c,d,e,h,l}`
- `cpi`/`cpd`/`cpir`/`cpdr` -> `{b,c,h,l}` -- **not de**, which is why they are a
  separate arm and not members of the LD one
- `ini`/`ind`/`inir`/`indr`/`outi`/`outd`/`otir`/`otdr` -> `{b,h,l}` -- B alone
  is decremented, so `c` (the port) survives
- `rrd`/`rld` -> `{a}`
- `in` -> its destination register; `out` -> nothing, named rather than left to
  the default so it reads as a decision

**And the flag model needed a carve-out the module's own doc had predicted.**
`ld` is on the flag-neutral allowlist, but `ld a,i` and `ld a,r` load `a` and set
S/Z while copying IFF2 into P/V: they are flag WRITERS. The doc said they "MUST
enter the F-writer set the day those forms land". They landed here, and the
carve-out is by OPERAND SHAPE, so `ld i,a` stays neutral and an ordinary `ld`
stays neutral. That kill condition is now spent and the paragraph rewritten.

**`flag_check.rs` needed nothing, and that is measured rather than skipped.**
Its Z80 carry-writer allowlist leans false-negative (an unmodeled mnemonic is
CC-transparent). Checked against the Zilog flag table: **not one** of the added
instructions writes carry. The LD and CP block ops, the IN/OUT block transfers,
`in`, `out`, `rrd`/`rld`, `reti`/`retn`, `im`, and `ld a,i`/`ld a,r` all leave C
unaffected; `sbc`/`adc` were already on the list.

**`context.rs`'s Z80 terminator set** already listed `reti`/`retn` before either
was assemblable. Now correct rather than aspirational.

**A new mnemonic reserves a word,** and `in`/`out` are ordinary English.
`.probe/collision.sh` asked both live trees whether any of the 21 names already
appears as an indented head or as a `macro`/`equ` name: **the only hit is the 17
`ldi` sites this parcel is about**. Canary-checked (`ret` = 115 in the corpus),
and the aeon zero is explained above rather than trusted.

---

## 5. Corpus decomposition

Plain `sigil s2.asm` over a detached `s2disasm` worktree at `e45ebf33` under a
run-unique path; the owner's live checkout was never written to. Both binaries
answer the parcel's own probe first, as a freshness witness:

```
BEFORE ldi: exit=1 : error: unknown directive or mnemonic `ldi`
AFTER  ldi: exit=0 : ED A0
```

**Measured, never predicted by subtraction.**

| | before | after | delta |
|---|---|---|---|
| after the `ldi` commit alone | 5247 | 5230 | **-17** |
| after the whole parcel | 5247 | **5229** | **-18** |

Per class, the only two rows that moved:

```
error   18 -> 1   -17   unknown directive or mnemonic `X`
error    1 -> 0    -1   unsupported form: Sbc, ops: [Pair(Hl), Pair(Bc)]
       5247 -> 5229  -18  TOTAL
```

**No class ROSE. No class APPEARED.** The APPEARED set is empty (count 0), and
the unresolved-symbol name sets are identical in both directions:
`before-only 0`, `after-only 0`, `in both 8`.

The 18 removed lines are the 17 `ldi` sites plus `s2.sounddriver.asm:4023`'s
`sbc hl,bc`. **The one surviving `unknown directive or mnemonic` in the whole
corpus is `s2.sounddriver.asm:251`'s `listing`,** which is an AS directive and
not a Z80 instruction: after this parcel, no Z80 mnemonic in the corpus is
unknown to the front end.

---

## 6. Anything in this brief you concluded was wrong

**1. The row's "22" is 21, and the missing one has no asl answer.** The set of
documented Z80 mnemonics the front end could not name is **21** (`ldi` plus the
20 in section 1), not 22. The obvious twenty-second is the undocumented `sll`,
and **the reference asl refuses it under all six spellings tried**, so it could
not have been encoded from asl at all. A count of what a probe list happened to
contain is not a count of what is missing -- the brief said so, and it was right
to.

**2. The row's "now visible rather than silently swallowed" describes a state
that a previous lane already closed, and `ldi` was that lane's exemplar.** Every
one of these was already LOUD: `unknown directive or mnemonic \`ldi\``, exit 1.
The silence was fixed earlier, by the change that made an indented head under
`CPU Z80` a diagnostic instead of a label, and that change's own unit test used
`ldi` as its example of "a real Z80 instruction this assembler does not encode".
This parcel therefore had to edit that test, which is a thing the brief did not
anticipate. Its docstring already recorded asl's answer for the case in prose
(`00 ED A0 00`), so the row became a **positive control** asserting exactly
that, which is strictly stronger than the refusal it replaced.

**3. The brief's split of `ldi` and `ldir` is right, and its "17 sites" is
right, but the sharpening understates the parcel: `ldi` is not the only form
with a live population.** `sbc hl,bc` has one site, `s2.sounddriver.asm:4023`,
and it is a form-level gap inside a mnemonic the front end already knew --
invisible to any sweep that enumerates mnemonics rather than forms. It is the
eighteenth corpus row this parcel removes.

**4. The brief's `ISA-CYCLE-TABLE-GAP` framing ("new instructions resolve to
Unknown there") is right for 20 of 22 and backwards for two.** `reti` and `retn`
were already PRICED at 14 T with no encoder. The gap ran the other way for them:
the analyzer was ahead of the encoder, not behind it.

**5. The landing gate could not be reached, and the reason was master's, not
this branch's.** `cargo clippy --workspace --all-targets -- -D warnings` exited
101 on master with 36 `tabs_in_doc_comments` findings in two test files this
parcel does not touch (both byte-identical to master, both from `33ceea3a`,
confirmed an ancestor of master by `git merge-base --is-ancestor`). This is the
same clippy red the brief said had "been failing for hours". Fixed in its own
commit, silenced per item rather than crate-wide per the script's own rule.
**The one judgement call to argue with:** `as_insn_operand_builtins.rs` gets the
inner `#![allow]` form, which for an integration test file IS crate-wide,
because its MODULE doc comment carries one of the tabs and a `//!` run has no
outer-attribute spelling.

**6. One thing the brief did not raise and I decided against.** The `.emp` front
end's own Z80 mnemonic table (`lower/code.rs`) is deliberately NOT extended, so
none of these is writable in `.emp` yet. That is language surface, and language
surface is the owner's to rule on. The consequence is stated where it matters:
`crates/sigil-frontend-emp/tests/z80_new_mnemonic_write_sets.rs` says in its own
header that it exercises the TABLE and not the pipeline. Extending the emp
table would additionally need a `Z80IndC` operand for `in`/`out`. **TAGGED for
the owner.**

**7. `LdIA` and `LdRA` are dead.** Two `Mnemonic` variants that nothing
constructs and no encoder arm matches; `ld i,a` lowers through `Ld` with a
`RegI` operand. Left alone -- removing them is a different change with a
different blast radius -- but noted so the next reader does not assume they are
the path.

---

## 7. Files

- `crates/sigil-isa/src/z80.rs` -- `Operand::IndC`, 21 `Mnemonic` variants,
  the encoder arms, `port8`
- `crates/sigil-isa/tests/corpus/mod.rs` + `tests/z80_golden_vectors.txt` -- 33
  asl-minted vectors
- `crates/sigil-frontend-as/src/eval.rs` -- names, the `(c)` operand, and the
  column-rule test's row 4
- `crates/sigil-frontend-as/tests/as_z80_instruction_coverage.rs` -- new
- `crates/sigil-frontend-emp/src/z80_preserves.rs` -- write sets and the flag
  carve-out
- `crates/sigil-frontend-emp/tests/z80_new_mnemonic_write_sets.rs` -- new
- `crates/sigil-frontend-as/tests/as_insn_operand_builtins.rs`,
  `as_str_builtin_nesting.rs` -- master's clippy red, separate commit

Probes kept under `.probe/` in the worktree: `forms.py`, `enum.sh`,
`analyze.py`, `extra.sh`, `portrange.sh`, `sites.sh`, `collision.sh`,
`corpus.sh`, `mint.sh`, `aslcheck.sh`.
