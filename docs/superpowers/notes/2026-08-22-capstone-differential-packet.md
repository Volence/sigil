# Parcel packet: the capstone differential — a non-circular 68000 ISA oracle

**Date:** 2026-08-22 · **Branch:** `feat/capstone-differential` (from master
`5c75b5b6`) · **Motivating correction:**
`2026-08-19-m68k-roundtrip-packet.md`, whose soundness claim was narrowed
during review — the 65,536-word opcode sweep's oracle is sigil's own `encode`,
so it proves *decoder ⊆ encoder* and is blind to a defect both halves share.
The `TST` `DATA`-vs-`DATA_ALTERABLE` row was exactly such a defect and
round-tripped green.

**Byte bar:** byte-neutral by construction. No `crates/sigil-harness/golden/`,
`pins.rs`, `repin.toml` or provenance file touched; no `crates/*/src/` file
touched at all. Everything added is a test target, a test-support module, a
Python dump helper and one row in the nightly lane's gate list.

**Oracle:** Capstone 5.0.7 (core 5.0.1280), `CS_ARCH_M68K` /
`CS_MODE_M68K_000`. Independently written, no shared lineage with sigil.

---

## Phase 1 — validate the instrument, gate nothing

The differential ran as a dry run over both reachable corpora before anything
was adopted. **Every disagreement is enumerated below; nothing is sampled.**

### What "disagreement" means here, and why

Capstone's operand text differs from sigil's by formatting in most instructions,
so a text diff would have produced thousands of non-findings. Both sides are
instead normalised into one abstract form and that form is compared:

| compared | why |
|---|---|
| **legality** — sigil decodes ⟹ capstone decodes | this is the `TST` class; the converse is NOT asserted, because sigil's decoder deliberately covers only the forms its encoder emits, so capstone decoding `exg`/`abcd`/`chk` carries no information |
| **length** — consumed byte count | catches an extension-word miscount |
| **family** — mnemonic in sigil's vocabulary, conditions expanded (`scc`+`Eq` → `seq`) | catches an aliased opcode word |
| **operation size** — only where capstone reports one | capstone prints no suffix for `jmp`/`swap`/`trap`/`rts`/`dbcc`/`move …,sr`; that is capstone declining to answer, not agreement, and is not scored |
| **operands** — ordered canonical strings: EA mode, register number, displacement, absolute, immediate (modulo the stored field width), reglist mask, branch/PC targets resolved to an offset from the instruction start | catches a wrong EA field, a wrong register, a wrong displacement |

Capstone's *structured* operand detail (`insn.operands`) is **not** used: in
5.0.7 its m68k backend leaves `mem.base_reg` invalid for `(An)`/`(An)+`/`-(An)`
and `mem.disp` zero for the absolute modes, so a structural comparison would
have been measuring capstone's detail bugs. The rendered `op_str` carries the
real values and is parsed into the canonical form instead.

PC-relative operands need the byte offset of their own extension word to turn
capstone's absolute target back into a displacement, and the decoder does not
report it. It is **derived**, not tabulated: re-encode the instruction with that
one displacement perturbed and diff the bytes — the word that moves is the
extension word. A failed derivation renders as `pc:?`, which matches nothing, so
it surfaces as a disagreement rather than as a pass.

### Corpus 1 — the 65,536-word opcode space

Walked twice, with pads chosen from what each extension-word consumer requires:

- **`$0000`** — the same padding the encoder-oracle sweep uses. sigil decodes
  42,774 words, capstone 45,838.
- **`$00FF`** — nonzero values in every extension-word consumer that admits
  them: `(d8,An,Xn)` displacement `−1` (the disp8 sign path), `$00FF`
  displacements and absolutes, `$00FF00FF` longs, byte immediate `$FF`. sigil
  decodes 42,774, capstone 45,836.

A third pad `$80FF` (negative `(d16,An)`) was measured and **not shipped** — see
the open finding below; it is the one place the two disagree where capstone is
not the wrong side.

### Corpus 2 — the shipped shapes' emitted stream

All seven shapes built inside a `m68k::capture` session against aeon
`1ee8f8e6`: 86,401 captured `(Instruction, bytes)` pairs
(sonic4 plain 12473 · sonic4 debug 14927 · demo plain 8973 · demo debug 10366 ·
config_a 15055 · config_b 12193 · lean 12414), **2,948 distinct byte strings**
after dedup. Capstone decoded 2,948 of 2,948. These carry the operand VALUES a
padded sweep cannot reach — real displacements of both signs, real long
immediates, real absolute addresses.

### THE COMPLETE INVENTORY

Every disagreement class the two corpora produced, with counts and
classification. "Class size" is words for the sweep, distinct byte strings for
the stream.

#### Class A — disassembly SPELLING, not a semantic disagreement (4 classes)

Both sides carry the same value; only the notation differs. Handled by
normalisation in the canonical form, **not** by exclusions — nothing is being
excused. Each rule can only fire when the two values already agree.

| # | class | pad `$0000` | stream | what it is |
|---|---|---|---|---|
| A1 | `moveq` immediate | 1024 | 10 | the data field is a stored signed byte the CPU sign-extends; sigil reports `Imm(-128)`, capstone prints the stored `#$80`. Compared modulo the 8-bit stored field — the same Rule I the existing round-trip relation uses, applied to BOTH sides |
| A2 | `movem` empty register list | 140 | 0 | mask `$0000` has no registers to name, so capstone prints the raw mask word as an immediate (`movem.w #$0, (a0)`). Rewritten only when sigil has a `RegList` in that slot AND the numbers agree |
| A3 | `movem` single-register list | 0 | 8 | a one-bit mask prints as the bare register (`movem.l a3, -(a7)`). Rewritten only when the mask has exactly one bit and it is the bit that register names |
| A4 | `illegal` | 1 | 0 | the fixed word `$4AFC` with no operands; capstone prints the word itself as an immediate. Dropped only for the `illegal` family and only when the printed immediate IS `$4AFC` |

Totals with normalisation disabled: **1165** at pad `$0000`, **18** in the
stream. With it: 0.

#### Class B — CAPSTONE is the wrong side (5 classes, each an exclusion)

Each derivation is stated with its authority, and each was independently
corroborated where `asl` could answer. `asl` is a third implementation, not
sigil's mirror, and it disagrees with sigil elsewhere, so its agreement here is
evidence rather than restatement.

| # | exclusion | pad `$0000` | pad `$00FF` | stream | kind excused |
|---|---|---|---|---|---|
| B1 | `branch-ff` | 16 | 16 | 0 | legality |
| B2 | `btst-immediate-destination` | 1 | 1 | 0 | legality |
| B3 | `bit-op-size` | 504 | 504 | 43 | size |
| B4 | `btst-dynamic-immediate-length` | 8 | 8 | 0 | length |
| B5 | `pc-base-after-extension` | 6 | 6 | 0 | operands |

**B1 — `6xFF`, re-derived (this was a starting hypothesis; it confirms).**
MC68000 (M68000 8-/16-/32-Bit Microprocessor User's Manual, BRA/BSR/Bcc): the
8-bit displacement is used as written, and the ONLY escape is `$00`, which
selects the following 16-bit displacement word. The `$FF` escape to a 32-bit
displacement is an MC68020 addition; the MC68000 predates it. So on an MC68000
`6xFF` is a well-formed 2-byte branch with displacement `−1`. Capstone, even in
`CS_MODE_M68K_000`, applies the 68020 escape and then refuses the long form,
answering `dc.w`. **Class size 16, derived:** the condition field (bits 11–8) is
free and the displacement byte is fixed, so the class is `0x60FF..=0x6FFF` step
`0x0100` — `bra`, `bsr` and the 14 conditional branches.

**B2 — `083C` = `btst #<data>,#<data>`.** MC68000 PRM, BTST: the destination
effective-address field takes the data addressing modes, and the immediate and
PC-relative rows carry the footnote that they are valid **for BTST only** —
BTST reads a bit and writes nothing, so unlike BCHG/BCLR/BSET it does not need
an alterable destination. `asl` implements exactly that differentiated rule:

```
btst #1,#$ff  →  083C 0001 00FF
bclr #1,#$ff  →  x.asm(4):10: error: addressing mode not allowed here
```

sigil agrees with asl; capstone omits the immediate row from its BTST
destination table. **Class size 1, derived:** the static form fixes every bit —
`0000 1000 0011 1100`.

**B3 — bit-op operation size (this was the second starting hypothesis; it
confirms, and is broader than stated).** MC68000 PRM, BTST/BCHG/BCLR/BSET —
"Operand Size: Byte, Long": long when the destination is a data register, byte
when it is a memory location. The size is a function of the DESTINATION, which
is what sigil's `canonical_size` implements. Capstone answers a different
function entirely: byte for everything except the DYNAMIC form of `btst`, where
it answers long. It is therefore wrong in both directions — `btst d0,$1234.l` is
byte-sized on the MC68000 and capstone says long; `bset #1,d0` is long-sized and
capstone says byte. The parcel named this as "`btst Dn,#imm` sizing"; the real
class is every bit-op form whose destination register-ness disagrees with
capstone's source-keyed rule.

The predicate reproduces capstone's rule exactly and excuses only the size that
rule produces — any other answer (say `.w`, or `.l` on a `bset`) still fails.

**Class size 504, derived from the encoder's destination rows.** BTST's
destination row is DATA: `Dn`(8) `(An)`(8) `(An)+`(8) `-(An)`(8) `(d16,An)`(8)
`(d8,An,Xn)`(8) `(xxx).W` `(xxx).L` `(d16,PC)` `(d8,PC,Xn)` `#<data>` = 53
forms; BSET/BCLR take DATA_ALTERABLE = the same list minus the three read-only
rows = 50.
- `btst` dynamic: 45 non-`Dn` destinations disagree, but one of them
  (`#<data>`) disagrees on LENGTH first (B4) and never reaches the size
  comparison → 44 × 8 source registers = **352**;
- `btst` static: the 8 `Dn` destinations = **8**;
- `bset`/`bclr`, both forms: the 8 `Dn` destinations, × 8 source registers for
  the dynamic form → 2 × (64 + 8) = **144**.

352 + 8 + 144 = **504**, which is what the run counts.

**B4 — `btst Dn,#<data>` length.** Because capstone believes this form is
long-sized it reads the immediate as a long and consumes two extension words
where the MC68000's byte-sized form has one: sigil 4 bytes, capstone 6. `asl`
assembles `btst d0,#$ff` to `013C 00FF` — 4 bytes, sigil's answer. The predicate
demands the exact `+2` over-read. **Class size 8, derived:**
`0000 rrr 1 00 111 100`, bits 11–9 free.

**B5 — PC base when an extension word precedes the displacement.** MC68000 PRM
§2.6/§2.7: for `(d16,PC)` and `(d8,PC,Xn)` the PC value is "the address of the
extension word" — the displacement's OWN extension word. `asl` confirms it on
both shapes:

```
btst #1,target(pc)      target at 6  →  083A 0001 0002   (disp word at 4; 4+2 = 6)
movem.w target(pc),d0   target at 8  →  4CBA 0001 0002   (disp word at 4; 4+2 = 6)
```

Capstone resolves from the address of the FIRST extension word, so whenever one
precedes the displacement its answer is short by exactly that many bytes. The
predicate demands precisely that shortfall in precisely one operand slot: same
family, same operand count, every other operand identical, both sides
PC-relative in the differing slot, and capstone's target = sigil's target −
(ext_off − 2).

**Class size 6, derived** — and this is the derivation that had to be corrected
mid-parcel. Two shapes place an extension word before a PC-relative
displacement, and PC-relative is read-only so only a source/tested operand can
carry it: the STATIC bit ops put the bit-number word first and their destination
row admits PC-relative for BTST only (`083A`, `083B`); and `movem <ea>,reglist`
puts the mask word first (`4CBA`/`4CBB` at `.w`, `4CFA`/`4CFB` at `.l`). The
first draft said 2 — the four `movem` words were hidden behind the A2 spelling
class until that was normalised. **The exact-class-size assertion is what caught
it**, which is the property it exists for.

#### Class C — SIGIL is the wrong side, or the ISA is ambiguous (1 class)

**C1 — a nonzero high byte in the static bit-ops' bit-number extension word.
152 words at pad `$80FF`. NOT excluded, NOT fixed, reported for adjudication.**

Under a pad whose high byte is set, sigil decodes `0800 80FF` as
`btst #33023,d0` (and 151 sibling words: 52 `btst` + 50 `bclr` + 50 `bset`
destination rows). Capstone rejects all 152.

The MC68000 instruction format for the static bit ops specifies the extension
word as `00000000 bbbbbbbb` — the high byte is zeros, the bit number is the low
byte. `asl` enforces it in both directions:

```
btst #255,d0    →  0800 00FF   (with "warning: bit number will be truncated")
btst #-1,d0     →  0800 00FF   (truncated to the 8-bit field)
btst #256,d0    →  error: range overflow
btst #$80ff,d0  →  error: range overflow
```

sigil's `encode_bit` instead does `u16::try_from(*v)` and writes the whole word
(`m68k.rs`, static-form arm), so it accepts any bit number up to 65535 and emits
it verbatim; the decoder reads it back; the round trip is byte-exact and every
existing gate stays green. **This is a defect both halves share — precisely the
class this differential exists to find** — but it is NOT the trapping kind:
the MC68000 takes the bit number modulo 32 (data-register destination) or
modulo 8 (memory), so those high bits are ignored and the instruction executes
correctly. What sigil loses is asl's diagnostic: a source asking for
`btst #$100,d0` assembles silently under sigil and errors under asl.

Disposition, and why: this cannot be classified as capstone-side, so per the
sequencing rule it is neither excluded nor matched-to. It is also not fixed
here — a range check in `encode_bit` is an encoder-row change whose byte
neutrality I cannot establish from inside a gate parcel. **Adjudication is the
overseer's.** The shipped gate is built around it by not carrying the `$80FF`
pad; the cost is that a negative `(d16,An)` displacement is unreachable in the
sweep (every pad word with the high bit set also has a nonzero high byte), and
that coverage comes from the emitted-stream pass instead, whose displacements
are whatever the shipped games compile to.

### Nothing else

At pad `$0000`, pad `$00FF` and over the 2,948-string emitted stream, after the
four spelling normalisations and the five named exclusions, the count of
remaining disagreements is **zero**. There is no residual bucket and no
unclassified case.

---

## Phase 2 — what shipped

| file | what |
|---|---|
| `scripts/capstone_m68k_dump.py` | the oracle adapter. Makes NO judgement: disassembles buffers with capstone and prints what capstone said, verbatim TSV. Two modes — `sweep` (all 65,536 words, `--pad2=0xHHLL`) and `bytes` (hex buffers on stdin). Import failure exits 3 with a message on stderr |
| `crates/sigil-isa/tests/support/capstone_diff.rs` | the shared machinery: the canonical form, the capstone-rendering parser, the four spelling normalisations, the five named exclusions with their derivations, the comparison and the reporting. Under `tests/support/` so cargo does not build it as a target; the harness side reaches it with an explicit `#[path]`, because ONE definition of "disagreement" and ONE exclusion set is the point — two copies would drift, and a drifted exclusion is a hole |
| `crates/sigil-isa/tests/m68k_capstone_differential.rs` | gate 1: the opcode-space sweep, two pads |
| `crates/sigil-harness/tests/m68k_capstone_stream.rs` | gate 2: the emitted stream of all seven shipped shapes |
| `scripts/nightly_source_gates.sh` | one row: `m68k_capstone_stream` in `SOURCE_GATES` |

### Runners — named

- **Gate 1** runs in the default suite: `cargo test --release --workspace
  --no-fail-fast`. No aeon dependency, 0.53 s wall for both passes (the capstone
  dump itself is 0.25 s per 65,536-word pass), so it is cheap enough to live
  there unconditionally and is not opt-in.
- **Gate 2** runs in the default suite too (skip-green without `AEON_DIR`, like
  its `m68k_roundtrip_stream` peer) **and** in the nightly source-gate lane
  (`scripts/nightly_source_gates.sh`, systemd timer `sigil-source-gates.timer`,
  05:17 daily), which sets `AEON_DIR` and `SIGIL_STRICT_GATE=1`. It belongs in
  that lane and not the artifact lane: it reads aeon SOURCE and sigil's own
  compilation of it, and builds no ROM, reads no listing and touches no golden.
  Wall clock 5.3 s / 9.5 s over two runs, against its peer's 5.1 s — the cost is
  the seven ROM builds, which that lane already pays.

The lane's two self-audits were replicated against this tree: 35 gate rows named
/ 35 result blocks produced, zero unclassified aeon-reading test targets, every
row resolving to a real target. Gate 1 does not trip the lane's aeon-reading
grep and correctly stays out of the list.

### The exclusions cannot silently widen

Three properties, all enforced every run:

1. **Predicates are value-aware, not word-ranges.** B3 reproduces capstone's
   wrong size rule and excuses only that answer; B4 demands the exact `+2`
   over-read; B5 demands the exact shortfall in exactly one operand slot with
   every other operand identical. A different defect in the same word is not
   excused.
2. **Each exclusion covers exactly one disagreement KIND** (`legality`,
   `size`, `length`, `operands`), so a carve-out written for a size quirk
   cannot swallow a legality disagreement in the same word class.
3. **Exact class sizes are asserted**, derived from the encoding, on the
   `$0000` pass: 16 / 1 / 504 / 8 / 6. An exclusion that matches nothing fails
   with "it has outlived its cause, delete it"; one that matches more than its
   derivation says fails with "the class moved". This fired twice during
   development and was right both times (B5's 2 → 6, B3's 512 → 504).

### Unmeasurable is loud

`capstone_or_skip` prints `skip: capstone oracle unavailable (…)` to both
stdout and stderr, and under `SIGIL_STRICT_GATE=1` panics instead — the
repo-wide `reference_tree` convention applied to a tool rather than a source
tree. The nightly lane already treats any surviving `skip:` line as
COULD-NOT-RUN, so a capstone-less nightly reports as a lane failure, never as
coverage. Proven below.

---

## How it was verified

### Red-first — MUTANT T (the historically real one)

`TST`'s destination row widened from `DATA_ALTERABLE` to `DATA` on **both**
sides (encoder `m68k.rs::encode_single_ea`, decoder `m68k_decode.rs` line 420),
which is what the real defect looked like:

```
9 capstone disagreement(s) in the opcode sweep, pad $0000 (first 9 shown; …):
  [legality] 4A3A: sigil decodes Instruction { mnemonic: Tst, size: B, ops: [Pcd16(0)] } from [4A, 3A, 00, 00] but capstone rejects the word
  [legality] 4A3B: sigil decodes Instruction { mnemonic: Tst, size: B, ops: [Pcd8Xn { d: 0, xn: D(0), long: false }] } from [4A, 3B, 00, 00] but capstone rejects the word
  [legality] 4A3C: sigil decodes Instruction { mnemonic: Tst, size: B, ops: [Imm(0)] } from [4A, 3C, 00, 00] but capstone rejects the word
  [legality] 4A7A: … 4A7B: … 4A7C: …
  [legality] 4ABA: … 4ABB: …
  [legality] 4ABC: sigil decodes Instruction { mnemonic: Tst, size: L, ops: [Imm(0)] } from [4A, BC, 00, 00, 00, 00] but capstone rejects the word
```

All nine words, named, without anyone having had to think of them. Under the
same mutant the encoder-oracle sweep stays green —

```
--- encoder-oracle sweep (should stay GREEN):
test every_decodable_word_reencodes_to_the_consumed_bytes ... ok
test result: ok. 1 passed; 0 failed; …
```

— which is the non-circularity claim, demonstrated rather than asserted. The
whole `sigil-isa` suite under MUTANT T: two tests move, the hand-written
`ea_class_rejects::tst_destination_is_data_alterable` (which exists only because
the TST fix added it, and names one form) and this gate (which names the class).

### Red-first — MUTANT Q (a shared defect on emitted code)

`addq`/`subq` swapped in **both** encoder and decoder together, so the round
trip cannot see it:

```
--- encoder-oracle sweep (expect GREEN):    test result: ok. 1 passed; 0 failed
--- roundtrip stream    (expect GREEN):     test result: ok. 1 passed; 0 failed   (19.62s)
--- capstone opcode sweep (expect RED):
2656 capstone disagreement(s) in the opcode sweep, pad $0000 …
  [family] 5000: sigil `subq` vs capstone `addq` (Instruction { mnemonic: Subq, size: B, ops: [Imm(8), Dn(0)] } / `addq.b #$8, d0`)
--- capstone stream (expect RED):
107 capstone disagreement(s) in the emitted stream …
  [family] 5041000000000000000000000000 (first emitted by `sonic4 plain`): sigil `subq` vs capstone `addq` (… / `addq.w #$8, d1`)
  [family] 5100000000000000000000000000 (first emitted by `sonic4 plain`): sigil `addq` vs capstone `subq` (… / `subq.b #$8, d0`)
```

Both new gates are red-able independently, on a defect class every pre-existing
m68k gate is blind to. Both mutants restored; `git status` clean of source
changes afterwards.

### Red-first — the exclusion guards

Both fired for real during development, before any of this was committed:

```
assertion `left == right` failed: exclusion `pc-base-after-extension` covered 6 words
but its derivation says the class is 2 words — the class moved and the derivation no longer describes it
  left: 6   right: 2

assertion `left == right` failed: exclusion `bit-op-size` covered 504 words
but its derivation says the class is 512 words — the class moved and the derivation no longer describes it
  left: 504  right: 512
```

Each was a wrong derivation, not a wrong measurement, and each was corrected by
redoing the derivation (B5's `movem` shapes; B3's B4 short-circuit).

### Unmeasurable — the strict-gate proof

With a `python3` on `PATH` that reports capstone missing and exits 3:

```
=== oracle unavailable, SIGIL_STRICT_GATE unset (expect skip: + pass)
skip: capstone oracle unavailable (capstone dump failed (exit status: 3): capstone import failed: No module named 'capstone')
test opcode_sweep_agrees_with_capstone ... ok
test result: ok. 1 passed; 0 failed; …

=== oracle unavailable, SIGIL_STRICT_GATE=1 (expect panic)
SIGIL_STRICT_GATE set but the capstone oracle is unavailable: capstone dump failed (exit status: 3): capstone import failed: No module named 'capstone'
test opcode_sweep_agrees_with_capstone ... FAILED
test result: FAILED. 0 passed; 1 failed; …
```

### Suite totals — failures first

Zero failures in every run below.

| run | passed | failed | ignored | result blocks | wall |
|---|---|---|---|---|---|
| master `5c75b5b6`, this tree with the new files held aside | 3810 | **0** | 4 | 333 | — |
| this branch, pre-commit | 3812 | **0** | 4 | 335 | 101 s |
| this branch at `d38f655b`, clean tree | 3812 | **0** | 4 | 335 | 111 s |

Both with `AEON_DIR=/home/volence/sonic_hacks/.aeon-landing` and
`SIGIL_STRICT_GATE=1`; `--release --workspace --no-fail-fast`. `+2` is exactly
the two new test targets. `skip:` lines in the branch run: **0**.

The nightly lane's own cargo invocation, replicated against this tree
(35 `--test` selections, `AEON_DIR` + `SIGIL_STRICT_GATE=1`, `--nocapture`):
exit 0, 35 gates named / 35 result blocks, 158 passed / 0 failed, 0 `skip:`
lines, 27 s wall.

Per-gate wall clock, warm binaries: gate 1 0.53 / 0.53 / 0.54 s over three runs;
the capstone dump alone 0.25 s per pass; gate 2 5.28 / 9.49 s over two runs
against `m68k_roundtrip_stream`'s 5.13 s on the same corpus.

**One reporting correction, recorded because it nearly went into this packet as
a fact:** the first workspace run's log showed 3863/0/3 across 340 result
blocks. That log was a corrupted capture — two output streams interleaved,
19 test binaries appearing twice, `Running` lines spliced mid-token. The clean
re-run gives 3812/0/4 across 335 blocks, which reconciles exactly with the
3810/0/4 baseline. The corrupted figure was never a real measurement.

---

## Open / BLOCKED — for the overseer

1. **BLOCKED (adjudication): the static bit-ops' bit-number high byte.**
   Class C1 above, 152 words. sigil's `encode_bit` writes a full 16-bit word
   where the MC68000 format specifies `00000000 bbbbbbbb`, and its decoder
   mirrors that, so encoder and decoder agree and every existing gate is green.
   Both `asl` (range-overflow error above 255, truncation to the 8-bit field
   below it) and capstone (rejects the word) are stricter than sigil.
   Consequence is a lost diagnostic, not a wrong ROM — hardware ignores those
   bits. A fix is a range check in `encode_bit`'s static-form arm; whether it is
   byte-neutral needs the byte-mover sequencing you own, so I neither applied it
   nor excluded the class. Re-enabling the `$80FF` pad in
   `m68k_capstone_differential.rs` is a one-line change once this is settled,
   and it buys negative-`(d16,An)` coverage in the sweep.

2. **LATENT (unrelated to this parcel's gate, found while deriving B1): the
   encoder accepts an ODD `.s` branch displacement.**
   `encode_branch`'s `Size::S` arm checks only `i8::try_from(disp)`, and the
   linker's `PcRel8` arm rejects a resolved displacement of `0` and out-of-range
   values but not an odd one. An odd byte displacement branches to an odd
   address, which is an address-error trap on an MC68000 — the same severity as
   the `TST` class. It is currently unreachable in practice (branch targets and
   the post-opcode PC are both even, so the difference is always even), which is
   why it is a latent and not a finding; `6xFF` is the word that made me look.
   A one-line `disp % 2 != 0` diagnostic in the linker arm would close it by
   construction.

3. **Not attempted, TAGGED for foreground: no runtime confirmation.** Nothing
   here was checked on an emulator; no `mcp__oracle__*` tool was touched. None
   of the findings need it — every claim is settled by the MC68000 reference
   plus `asl` — but the C1 hardware-ignores-the-high-bits claim is the one that
   could be confirmed on hardware/emulator if you want it belt-and-braces.

## Banked

- **Z80.** Capstone has no Z80 architecture, so this technique does not extend
  there. The Z80 side's non-circular oracle would have to be a different tool
  (or the existing `z80::disassemble` self-check, which is circular in the same
  way the m68k round trip is).
- **Capstone version drift.** The run prints the oracle's version banner every
  time (`capstone oracle: capstone 5.0.7 5.0.1280`) but does not pin it. A
  capstone upgrade that fixes B3 or B5 will fail the "exclusion matched
  nothing" assertion, which is the correct outcome — the carve-out should then
  be deleted, and the loud failure is what prompts it.
- **A `sigil disasm` surface.** The canonical-form renderer here is most of a
  disassembler's operand printer; the decoder has been stream-ready since the
  round-trip parcel. Still out of scope.
