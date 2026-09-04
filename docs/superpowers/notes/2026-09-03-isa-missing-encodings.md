# Five 68000 instruction lines the backend never had — bchg, exg, roxl/roxr, move-to-ccr, usp

Not AS compatibility. Every other row in this arc matches a quirk of another
assembler; these five are real MC68000 instructions the encoder had no row for,
and Sonic 1 uses all five. **Sonic 1 cannot be emitted while they are missing**,
so this is a floor under the corpus rather than a diagnostic count.

## Provenance

| | |
|---|---|
| branch | `isa-missing-encodings` off master `e5cbe258` |
| corpora | `s1disasm` at `f6ece657`, `s2disasm` at `e45ebf3`, each in a detached worktree — the live trees are never written |
| S1 seeding | `sound/dac/{pcm,dpcm}/generated/` copied from the live tree (the `.inc` **and** the `.pcm`/`.dpcm` beside them) |
| S2 seeding | `sound/{music,PCM,DAC}/generated/` copied; the live tree's are EMPTY, so 39 `cannot include` are a constant on both sides of the measurement |
| oracle 1 | `asl`, both shipped builds — S1's upstream 1.42 Beta 212 md5 `61e672562465725a8c102288a7da9098`, s2disasm's flamewing fork md5 `0dee1f98e6480a4783d27ffd8b90896f`; every invocation `-cpu 68000 -q -U` |
| oracle 2 | Capstone 5.0.7 (core 5.0.1280), `CS_MODE_M68K_000` |
| aeon | `/home/volence/sonic_hacks/.aeon-eval-ref` detached at `4f5ad5a1` |

The probe harness and all 448 probe results are committed beside this note in
`2026-09-03-isa-missing-encodings/`.

## The two oracles are genuinely independent, and both were consulted

`asl` produces the bytes; Capstone reads them back. Capstone did not learn the
ISA from asl or from sigil, so a form that clears both is verified against
something outside this project.

| instruction | asl (golden bytes) | Capstone (opcode-space differential) |
|---|---|---|
| `bchg` | ✔ 19 corpus rows minted from asl | ✔ decodes; one counted size class excused, see below |
| `exg` | ✔ 7 rows, all three pairs + the reversed order | ✔ |
| `roxl`/`roxr` | ✔ 18 rows | ✔ except one counted 64-word cell where **capstone is wrong** |
| `move <ea>,ccr` | ✔ 13 rows, the whole DATA source row + both accepted suffixes | ✔ |
| `move.l An,usp` / `usp,An` | ✔ 5 rows | ✔ |

No instruction here is covered by only one oracle.

**The two asl builds agree on all 393 matrix probes** — byte-for-byte on every
accepted form and accept/reject on every rejected one. Since these are ISA
encodings rather than assembler quirks that was the expected answer, and it is
recorded because a disagreement would have been a finding about the tools.

## The addressing-mode matrices, and which cells the corpus reaches

Enumerated from asl's own acceptance over twelve EA modes × four size spellings
per form, not from the corpus. **The corpus exercises a small minority of every
matrix.** Cells it cannot reach are named as unexercised rather than counted as
covered.

### `bchg` — `tt=01`, the fourth bit-op row

Destination row is `bset`'s: DATA ALTERABLE. Size is not encoded; asl derives it
from the destination (long for `Dn`, byte for memory) and rejects the other
suffix on each.

| destination | static `#n,<ea>` | dynamic `Dn,<ea>` | corpus |
|---|---|---|---|
| `Dn` | `0840\|ea` +bit word | `0140\|dn<<9\|ea` | **S1** (2 sites: `bchg #5,d4`, `bchg #5,d0`) |
| `(An)` `(An)+` `-(An)` | ✔ | ✔ | unexercised |
| `(d16,An)` | ✔ | ✔ | **S1 (47 sites), S2 (bulk)** — the `#n,off(a0)` object-flag idiom |
| `(d8,An,Xn)` `(xxx).W` `(xxx).L` | ✔ | ✔ | unexercised |
| `An`, `#imm`, `(d16,PC)`, `(d8,PC,Xn)` | rejected | rejected | — |

The corpus writes only the static form and only two of the eight legal
destinations. **The dynamic `Dn,<ea>` form has zero corpus sites** and is
covered by golden vectors and the capstone sweep alone.

asl range-checks nothing: `bchg #255,(a0)` assembles (`08 50 00 FF`); the
hardware masks. sigil matches.

### `exg` — three register pairs, no EA field

| operands | opmode | bytes | corpus |
|---|---|---|---|
| `Dx,Dy` | `01000` | `exg.l d0,d1` = `C1 41` | **S1 (5), S2 (7)** |
| `Ax,Ay` | `01001` | `exg.l a0,a1` = `C1 49` | **unexercised** |
| `Dx,Ay` | `10001` | `exg.l d0,a0` = `C1 88` | **unexercised** |

**asl normalises the mixed pair's written order.** `exg a0,d0` and `exg d0,a0`
both assemble to `C1 88`; `exg a1,d4` is `C9 89`, i.e. Rx = `d4`. The encoding
has one slot per register kind, so the written order carries no bits and cannot
be preserved. Both orders encode; `canonicalize`'s new **Rule E** carries the
equivalence so the round-trip check compares them equal.

Long-only: asl rejects `.b`/`.w`, takes bare and `.l`. Both spellings appear in
the corpora (S1 writes both, S2 writes only the bare form).

### `roxl` / `roxr` — the `tt=10` shift row

| form | bytes | corpus |
|---|---|---|
| register, immediate count 1..8 | `roxl.w #1,d3` = `E3 53` | **S1 (2), S2 (2)** — `.w #1` only |
| register, `Dn` count | `roxl.w d2,d0` = `E5 70` | **unexercised** |
| memory word, MEMORY ALTERABLE | `roxl.w (a0)` = `E5 D0` | **unexercised** |

Sizes: register form takes `.b`/`.w`/`.l`; the memory form is word-only (asl:
`roxl.b (a0)` → "invalid operand size"). Count `#0` is "range underflow" and
`#9` is "range overflow" under asl; sigil already policed `1..=8`.

**Two spellings that are not their own encodings.** asl also accepts
`<shift> Dn` for `<shift> #1,Dn` (`asl d0` = `E3 40`) and `<shift> #1,<mem>` for
the memory form (`roxl #1,(a0)` = `E5 D0`); `<shift> #2,<mem>` is an error
("operand must be one"). Both are now encoded, with **Rule R** in `canonicalize`.
This applies to the whole family, so `asl`/`asr`/`lsl`/`lsr`/`rol`/`ror` gain
the same spellings — see the corrected gate below.

### `move <ea>,ccr` — `44C0 | ea`, source row DATA

Every mode but `An` is legal, **including immediate and both PC-relative forms**
(`move #$12,ccr` = `44 FC 00 12` — the immediate extension is a full WORD even
though only the low byte reaches CCR). Corpus reaches `Dn` (S1 ×2 as `move.w`,
S2 ×2 bare) and `#imm` (S2 ×3 bare). **The other nine modes are unexercised.**

**There is no move-FROM-ccr on the MC68000.** It is a 68010 addition and asl
answers "instruction not supported on 68000" for all twelve destination
spellings. The family is one direction, and a leading `ccr` still fails loud.

asl accepts bare, `.b` and `.w` (identical bytes) and rejects `.l`, and **sigil
now accepts all three**. This was the one cell the enumeration found and the
first implementation did not close — sigil policed `.w` only, and neither corpus
writes the `.b` spelling, so it would have survived the parcel that was supposed
to find it. Accepting `.b` is safe for a specific reason worth stating: the
operand width is not read from `inst.size` at all, `Size::W` goes to `encode_ea`
unconditionally, and an immediate always gets its full word (`move.b #$12,ccr`
= `44 FC 00 12`, asl-confirmed and now a golden row). That is deliberately not
the `move …,sr` shape whose sibling defect keyed the immediate width to
`inst.size` and turned `move.l #$2700,sr` into `sr := $0000`.

### `move.l An,usp` / `move.l usp,An`

There is no matrix: `4E60|An` and `4E68|An`, one word, the address register in
bits 2-0. asl rejects `Dn` on either side and rejects `.w`. `a7`/`sp` are the
same register (`move.l sp,usp` = `4E 67`). The corpus has exactly one site in
each game (`move.l a6,usp`), so **fifteen of the sixteen words are unexercised**
by the corpus and covered by golden vectors and the sweep.

## Both corpora, before and after — every rise accounted for

Same binary, same worktrees, same seeding on both sides.

| | before | after | Δ |
|---|---:|---:|---:|
| **Sonic 1** (`sonic.asm`) | 368 | **318** | **−50** |
| **Sonic 2** (`s2.asm`) | 9,539 | **9,432** | **−107** |

Per class:

| corpus | class | before | after | Δ |
|---|---|---:|---:|---:|
| S1 | `X is not a recognized 68000 mnemonic` | 94 | 39 | **−55** |
| S1 | `unsupported form: ccr is not a general EA` | 2 | 0 | **−2** |
| S1 | `unresolved symbol X in operand` | 1 | **0** | **−1** |
| S1 | `unresolved long expression` | 5 | 13 | **+8** |
| S2 | `X is not a recognized 68000 mnemonic` | 187 | 120 | **−67** |
| S2 | `instruction needs an explicit size suffix` | 58 | 8 | **−50** |
| S2 | `unresolved symbol X in operand` | 3,374 | 3,384 | **+10** |

Every other class is unchanged in both corpora.

**Sonic 1's unresolved-symbol class is now empty.** `usp` was the last name in
it, and the previous parcel's measurement of "exactly one" is closed.

The row's own sites are **58 in S1** (49 `bchg` + 4 `exg` + 2 `roxl` + 2
`move.w d6,ccr` + 1 `usp`) and **73 in S2** (58 + 7 + 2 + 5 + 1) — figures
re-derived here, not carried from the baseline note.

**S1 reconciles exactly:** −55 mnemonic (49+4+2) −2 ccr −1 usp +8 = −50.

**S2 reconciles too, and the suffix class needs its own line.** S2 writes the
CCR and SR forms without a size suffix, so they were reported as
`instruction needs an explicit size suffix` rather than as unrecognized heads:

| S2 class | sites | why |
|---|---:|---|
| mnemonic | −67 | 58 `bchg`, 7 `exg`, 2 `roxl` |
| size suffix | −50 | 5 bare `move …,ccr` **and 45 bare `move …,sr` / `move sr,…`** |
| unresolved symbol | +10 | +11 `objoff_*` operands behind former `bchg` heads, −1 `usp` |

−67 −50 +10 = **−107**.

**The 45 bare `sr` sites are a side effect of the same defaulting rule**, and a
real one: `m68k_special_reg_size` answers for `ccr`, `sr` and `usp` together
because all three are `move` forms whose size only the operands can decide. All
nine accepted spellings were checked byte-for-byte against asl and sigil now
agrees on every one (`move #$2700,sr` = `46 FC 27 00`, `move sr,d6` = `40 C6`,
`move sr,-(sp)` = `40 E7`, `move (sp)+,sr` = `46 DF`, each identical to its
explicitly-`.w` spelling), and both tools reject `move.l #$2700,sr` and
`move.b sr,d6`.

### Rise 1 — S2 `unresolved symbol` +10

Class MIGRATION, not new breakage. All ten are at `bchg #n,objoff_XX(a0)` sites
that previously reported `` `bchg` is not a recognized 68000 mnemonic `` at the
SAME line. The head now parses, the operand is reached, and a pre-existing
unresolved `objoff_*` (the same class as the other 3,374) surfaces one level
deeper. Ten of the 58 `bchg` lines carry such an operand.

### Rise 2 — S1 `unresolved long expression` +8, and the pre-existing gap it exposed

All eight are one expression — `Map_Ring+(id_Rings<<24)` at
`_incObj/DebugMode.asm(382)`, the `dbug` macro body, once per zone. Measured,
not inferred:

1. The poisoned term is `Map_Ring` (instrumented run: `Map_Ring=Poison
   id_Rings=Value(37)`; its file-neighbours `Map_GRing`, `Map_Flash`,
   `Map_Monitor`, `Map_Crab` all resolve).
2. `Map_Ring` is declared at `sonic.asm(4121)` as
   `Map_Ring:   if Revision=0` — **a label sharing its line with an `if`
   directive.** Standalone, both oracles (`label-on-if-line.asm` beside this
   note): asl assembles it exit 0 and binds the label (`02 00 01 00 00 00`);
   sigil answers `unresolved long expression`. **sigil does not bind a label
   that shares its line with an `if`. That is a pre-existing sigil gap, not this
   parcel's**, and it is booked in the gap ledger.
3. What this parcel changed is that `move.l a6,usp` at `sonic.asm(232)` now
   emits its two bytes instead of erroring. Feature-bisected to `usp` alone
   (removing the four new mnemonics one at a time leaves the eight in place;
   removing the `usp` operand arm takes them to zero). Instrumented at the
   `dc.l` site: with the `usp` form disabled there are **13 additional visits
   with `keep_labels_symbolic=true`**, on which the expression takes the
   deferral branch and never folds; with it enabled those visits do not happen
   and the expression folds eagerly onto the unbound `Map_Ring`.

So the gap was masked by a deferral pass and is now reported. No bytes are at
risk: S1 exits 1 either way and the link never runs. Net S1 is still −50.

## What Capstone found, and where Capstone is the one that is wrong

The opcode-space differential walks all 65,536 words twice. Two new disagreement
classes appeared, 136 words in total, and in both the assembler backs sigil.

**`bit-op-size`, widened 504 → 576.** Derived, not measured: capstone answers
byte for every static/dynamic bit op except a dynamic `btst`; the MC68000 PRM
sizes them by the destination. `bchg` adds the 8 `Dn` destinations of the static
form and 8×8 of the dynamic form = **72**, and 504 + 72 = 576. asl agrees with
sigil (`bchg.l d2,d0` assembles; `bchg.b d2,d0` is "invalid operand size").

**`roxr-byte-register-count`, a NEW 64-word class — a capstone defect.** For the
single cell `d=0, ss=00, i=1, tt=10` (`roxr.b Dn,Dn`, words `E030`..`EEF7`)
capstone answers `roxr.l #<ccc>, dN`: it applies the immediate-count rule
(including the 0→8 convention) although bit 5 says register count, and reports
`.l` although the size field says byte. **All eleven other cells of the same
`tt=10` row it reads correctly**, which is what makes this one cell rather than
a different reading of the row.

asl refutes it twice over: `roxr.b d1,d5` = `E2 35`, which is in this cell, and
`roxr.l #8,d7` = `E0 97`, which is not — so `E030` cannot be `roxr.l #8,d0`.
The exclusion reproduces capstone's wrong rule exactly (the destination must
still agree and the first operand must be precisely the immediate the `ccc`
field would denote), so any other answer on those words still fails. Its class
size 64 is derived from the fixed bits and asserted exactly on the zero-pad
pass.

## A gate whose premise the assembler refutes

`ea_class_rejects::memory_shift_is_memory_only` asserted, in prose,
*"The single-operand shift is the MEMORY form; a data register is not it"* and
required `asl.w d0` not to encode. **asl assembles `asl d0` to `E3 40`** — the
count-1 register form, identical bytes to `asl #1,d0`. The rule was sigil's own
limitation written down as an ISA rule, and it survived because nothing in the
suite asked the assembler.

It is now `single_operand_shift_row`: it keeps the genuine exclusions (`An`,
`#imm`, PC-relative), adds the two register acceptances, and pins the aliasing
as an equality of emitted BYTES rather than as a claim.

## What changed

| file | |
|---|---|
| `crates/sigil-isa/src/m68k.rs` | 7 `Mnemonic` variants, `Operand::Usp`, `encode_move_to_ccr`/`encode_move_usp`/`encode_exg`, `bchg` in `encode_bit`, `roxl`/`roxr` in `encode_shift` + its three-spelling reduction, `writes_last_operand`, `family_name`, `ALL_FAMILY_NAMES` |
| `crates/sigil-isa/src/m68k_decode.rs` | the same five lines decoded; canonicalize Rules E and R; the `unknown_real_instructions` list re-scoped to what is still absent, plus a positive decode gate on asl's own bytes |
| `crates/sigil-isa/tests/corpus_m68k/mod.rs` | +62 corpus rows |
| `crates/sigil-isa/tests/m68k_golden_vectors.txt` | regenerated from S1's asl: **114 → 176 rows, all 114 existing rows byte-identical** |
| `crates/sigil-isa/tests/ea_class_rejects.rs` | the corrected gate + 3 new row gates (+4 `#[test]` net) |
| `crates/sigil-isa/tests/support/capstone_diff.rs` | `Usp` canon, special-register family folding, the widened and the new exclusion |
| `crates/sigil-frontend-as/src/eval.rs` | 4 mnemonics, `usp` operand, `m68k_special_reg_size`, three `refine_m68k_mnemonic` arms, default sizes |
| `crates/sigil-frontend-as/tests/snippets_golden.txt` | +8 asl-minted end-to-end blocks (193 → 201; zero churn in the 193) |
| `crates/sigil-harness/tests/m68k_roundtrip_stream.rs` | 7 reasoned `NOT_IN_STREAM` rows |
| `crates/sigil-frontend-emp/src/lower/code.rs` | `bchg`/`roxl`/`roxr`/`exg` |
| `crates/sigil-frontend-emp/src/lower/proc.rs` | the `exg` operand-shape write arm |

### The `.emp` boundary, drawn deliberately

`bchg`/`roxl`/`roxr`/`exg` are in the `.emp` mnemonic table (no new operand
kind, mirrors the AS table). `usp` and `move <ea>,ccr` are **not**: `usp` needs
a new `CodeOperand` variant and both are language surface, which is the owner's
to rule on. `.emp` source naming them gets today's unknown-mnemonic answer.

### `exg` and the clobber lint

`writes_last_operand` is a last-operand model and `exg Rx,Ry` writes BOTH. It
returns `true` (correct for `Ry`) and the `Rx` write gets the operand-shape arm
in `instr_written_regs` that the ISA doc says `link`/`unlk` would need — without
it a proc swapping a caller register through `exg` escapes
`[proc.clobber-undeclared]` on one of the two.

## Booked, not done

- **A label sharing its line with an `if` is not bound** (`Map_Ring:   if …`).
  Standalone repro beside this note; asl binds it, sigil does not. Pre-existing.
- **The cycle table prices none of the five.** `m68k_cycles`'s final arm is
  `_ => CycleCost::Unmodeled`, which `cycle_budget` renders `WalkCost::Unknown`
  — the loud direction, never a wrong price. The M68000UM rows for `bchg`,
  `roxl`/`roxr` and `exg` are a bounded follow-up; they were not added here
  because a UM number with no second source is worse than a refusal.
- **68000 instructions still absent** and each a named `Unknown` in the decoder:
  `abcd`/`sbcd`/`nbcd`, `negx`, `subx`, `chk`, `link`/`unlk`, `stop`, `reset`,
  `trapv`, `rtr`. None appears in either corpus. `moves` is 68010+ and out of
  scope by CPU.
- **The bare-suffix default for the shift family.** asl reads bare `asl #1,d0`
  as word; sigil requires an explicit suffix for the whole family. Untouched
  here (S1/S2 both write `roxl.w`), and it is a family-wide question rather than
  a roxl one.

## Aeon byte-neutrality — no shape moved

This row was genuinely at risk: instruction encoding is what aeon's ROMs are
built from. All four artifacts were **deleted before the builds**, so a stale
file could not answer for a build that did not run, and each shape was one
invocation of its own.

```
SIGIL_BUILD=<target>/release/sigil SIGIL_EMIT=<target>/release/emit_sound_blob \
SIGIL_VERSION_STRICT=1 AEON_DIR=/home/volence/sonic_hacks/.aeon-eval-ref ./build.sh
  … then DEBUG=1 ./build.sh, ./build.sh demo, DEBUG=1 ./build.sh demo
```

| shape | crc32 | size | vs the tree's frozen value |
|---|---|---:|---|
| `s4.bin` | `14ee2440` | 719,700 | **unchanged** |
| `s4.debug.bin` | `142294b3` | 737,683 | **unchanged** |
| `demo.bin` | `0c456778` | 96,474 | **unchanged** |
| `demo.debug.bin` | `2e603d53` | 101,339 | **unchanged** |

**Zero shapes moved.** That is the expected result and the reason for it is
structural rather than lucky: every change here ADDS an encoding where the
encoder previously returned `Err`, and aeon writes none of the five (see the
`NOT_IN_STREAM` rows). The one place an existing encoding could have shifted is
`encode_shift`, whose operand match was restructured — and its two new arms are
`[Dn]` and `[Imm(1), non-Dn]`, both of which previously errored.

Nothing under `golden/`, `pins.rs` or `repin.toml` was touched, and no refreeze
was performed.

### The freshness witness, and the way it nearly lied

Byte identity is silent on provenance, so the build banner is the witness: the
final run reads `Assembler: sigil 98f1b7393e93 (clean at capture — no
uncommitted changes)` under `SIGIL_VERSION_STRICT=1`, naming the exact commit.

**An earlier run of the same check produced a stale banner and it would have
read as a witness.** After an uncommitted edit to `crates/sigil-isa/src/m68k.rs`
— squarely inside the binary's own closure — a rebuilt `sigil` still reported
`clean at capture — no uncommitted changes`, because cargo re-runs the version
capture when HEAD or refs move and an uncommitted edit moves neither. `sigil
--version` says so itself, under `freshness:`; the banner does not.

So the tree-state WORD in that banner is not a freshness witness for an
uncommitted edit, only the revision is. What actually proved the binary current
was a behavioural discriminator — `move.b d6,ccr` assembling to `44 C6`, a form
that binary's predecessor refused. The rule for the next parcel: **commit
before the byte-neutrality run**, so the stamp re-captures, and if you cannot,
prove currency with something the new code does and the old code did not.

## Verification

- **Capstone opcode sweep** (65,536 words × two pads): green, with all six
  exclusion classes at their derived sizes exactly — 16 / 1 / **576** / 8 /
  **64** / 6. sigil decodes 44,337 words.
- **Capstone emitted-stream pass** over all seven aeon shapes: 3,050 distinct
  padded byte strings, capstone decoded 3,050 of 3,050, zero unexcused
  disagreements.
- **Golden vectors**: 176 rows, all minted by S1's own `asl`; the 114 pre-existing
  rows are byte-identical to before.
- **Clippy** `--release --workspace --all-targets -- -D warnings`: **exit 0**.
- **`m68k_roundtrip_stream`'s family-union check fired** on the first landing
  run — seven families encodable but emitted by no shape. That is the gate
  working: each now carries a `NOT_IN_STREAM` row with its reason, and the list
  is enforced in both directions (a listed family that shows up captured fails
  as stale).
- **AS-front-end snippet golden**, eight new blocks minted by S1's asl and read
  end-to-end (`assemble` → `resolve_layout` → `link` → `flatten`). Regenerating
  the file churns **only** the new blocks — the 193 pre-existing goldens come
  back byte-identical, which is the file's own non-circularity invariant.

### The red-first proof for the bare-suffix rule

The bare special-register sizes are 50 of Sonic 2's 107 and had no gate at all.
The mutation and its restore, both shown from disk:

```
# applied — read back from crates/sigil-frontend-as/src/eval.rs
6660:    return None; // MUTATION: red-first proof
6661:    #[allow(unreachable_code)]
6662:    atoms.iter().find_map(|a| match a {

test snippets_match_golden ... FAILED
  assemble: [ … "instruction needs an explicit size suffix (.b/.w/.l)" ×5 … ]
```

That is the exact diagnostic class the rule removes from the corpus, so the gate
fails for the reason it exists. Restored with `git checkout --` from the
committed baseline (grep for the marker returns 0), the **binary rebuilt** as
well as the source, and the gate green again. The corpora were then re-measured
with the restored binary and are byte-identical to the pre-mutation run
(S1 318, S2 9,432), and the four aeon shapes rebuilt from scratch a second time
with it and did not move.

### The full-suite landing run

`/home/volence/sonic_hacks/.isa-cov-work/landing2.log`, stamped with pwd, HEAD,
branch and the reference tree + its HEAD.

```
tree        /home/volence/sonic_hacks/.wt-isa-cov @ 88fee191 (isa-missing-encodings)
reference   /home/volence/sonic_hacks/.aeon-eval-ref @ 4f5ad5a1 (clean) — all four present
CARGO_EXIT  0
suites 378   passed 4366   failed 0   ignored 2   skip lines 0
RESULT      GREEN
```

(That run is the one at `88fee191`. The tree moved twice after it — the asl
snippet blocks and the `move.b …,ccr` cell — and the run was repeated on each,
green both times; the last is `landing-final.log` at `d4e03909` and the final
one is recorded below.)

Master's baseline is 378 / 4,362 / 0 / 2. This parcel adds **four** `#[test]`
functions — one in `m68k_decode`'s unit tests and three in `ea_class_rejects`
(a fourth, `single_operand_shift_row`, is the renamed `memory_shift_is_memory_only`
and is net zero) — so 4,362 + 4 = **4,366**, which is what the run observed.
Suite count is unchanged because no new test TARGET was added. All five
`--expect-test` names executed.
