# Sigil M1.A — 68000 Encoder → Full ISA (`sigil-isa::m68k` + `sigil-backend-m68k`)

**Date:** 2026-07-03
**Status:** Approved (brainstorming), ready for writing-plans
**Track:** Sigil Core backend/linker (M1). *Not* the Spec-2 `.emp` surface language.
**Predecessor:** M0 complete & merged (byte-exact Z80 driver regions A + B); M0.5 complete & merged
(`sigil` master @ `8bd3d1e`): `sigil-isa::m68k` MOVE-only procedural-EA encoder, 22 golden vectors byte-exact vs `asl`.
**Reference pin:** aeon @ `c7aaca6` (frozen for M1; corpus derived from this tree).

## 0. Where M1.A sits

M1 (full 68000 backend → byte-identical `s4.bin`) is too large for one spec. It decomposes into four
sub-projects, each with its own spec → plan → implement cycle:

- **A (this doc)** — 68000 encoder → full ISA + `sigil-backend-m68k` adapter. Ends at: encoder byte-exact on an
  Aeon-derived corpus + §5.5 hazard vectors; adapter produces `Fragment`s. **No full ROM.**
- **B** — full linker: single-image layout, fixup resolution, `jmp`/`jsr` width-selection *resolution*, the
  `HeaderChecksum` synthetic fixup, `s4.lst` emitter, convsym no-op.
- **C** — full AS front-end fidelity (`function`/`struct`/`if`/`fatal`/`set`/`org`-backpatch/`switch`/`lowstring`/
  operator quirks/`!error`/`!align`/bug-for-bug `strstr`/padding+supmode + `deform_table_sine` float folding).
- **D** — wire the full-ROM `sha256` gate, delete the M0 stub table, M1.a–d acceptance.

This doc is **A only**.

## 1. Goal & risk retired

Grow `sigil-isa::m68k` from MOVE-only to **every instruction and EA form the Aeon source at `c7aaca6`
actually uses**, proven byte-identical to `asl 1.42` (`aeon/tools/asl`, native, no Wine) on a corpus derived
from real usage — plus dedicated committed test vectors for each §5.5 byte hazard. Then add the thin
`sigil-backend-m68k` adapter crate.

The M0.5 spike already retired the hardest design risk (the **procedural EA / extension-word machinery**):
`encode_ea` + `brief_ext` are general and reused for both operand positions. M1.A is therefore **additive** —
it grows the opcode-dispatch layer on top of proven EA machinery; it does not restart.

**Success:** a committed golden-vector corpus covering every (mnemonic, size, representative-EA) form Aeon
uses, every §5.5 hazard pinned by its own vectors, all byte-matching `asl`; the full-corpus acceptance gate
green; `sigil-backend-m68k` in place with the crate-graph guard updated and green.

## 2. Corpus (the factual scope — derived, not guessed)

From a read-only enumeration of the non-Z80 tree at `c7aaca6` (Z80 excluded by `save`/`cpu z80`/`phase` region,
*not* per-file — the `phase 08000h` data block lives inside the otherwise-68k `main.asm`):

- **46 base mnemonic families** (~72 counting per-condition Bcc/Scc/DBcc), ~6,884 instruction lines.
- **All 12 standard EA modes** present; **brief-extension indexed form only** — no 68020 full-format extension,
  no scale factor, no An-index / PC-scaled index. Index register is a data register with an explicit `.w`
  (majority) or `.l` (2 sites) suffix; displacements small (0–2, occasional hex).
- **Branches are 100% size-pinned** (`bra` 141 `.s`/81 `.w`; `bsr` 21 `.s`/203 `.w`; every conditional suffixed).
  A targeted grep for any unsized branch returned nothing. ⇒ relaxation is a genuine no-op (§5.4 invariant holds).
- **Operand-width selection collapses to `jmp`/`jsr` bare-symbol targets only.** Every absolute *data* operand
  is explicitly `.w`/`.l`-annotated. The abs.w-vs-abs.l-by-address decision (§5.6) therefore touches only
  jump/call targets (125 `jsr` + bare `jmp`), not data.
- **Absent** (do not implement): EXG, LINK/UNLK, CHK, DIVS/DIVU/MULU, NBCD/ABCD/SBCD, SUBX, NEGX, NEG.L, all
  68010+/68020+ (bitfield, MOVEC/MOVES, RTD, STOP, RESET). TRAPV appears only as a vector-table label.

### 2.1 The mnemonic set to implement (grouped by encoding family — §3)

| Family | Mnemonics (from corpus) |
|---|---|
| move / movea | `move` (done), `movea` |
| ALU-EA (opmode + Dn + EA) | `add adda`, `sub suba`, `and`, `or`, `eor`, `cmp cmpa`, `muls` |
| ALU-immediate (`#imm,EA`) | `addi`, `subi`, `andi`, `ori`, `eori`, `cmpi` — incl. `andi/ori` to `ccr` and `move #imm,sr` sibling |
| quick (embedded data) | `moveq`, `addq`, `subq` |
| shift/rotate (reg-form + mem-form) | `asl asr`, `lsl lsr`, `rol ror` |
| bit (static `#n` + dynamic `Dn`) | `btst`, `bset`, `bclr` (`bchg` absent — implement if trivial, else skip) |
| single-EA (size + EA) | `clr`, `neg`, `not`, `tst`, `tas`, `Scc` (`st sf sgt`) |
| branch / DBcc | `bra bsr Bcc(16 conds)`, `DBcc` (`dbf dbeq`) |
| control / misc (EA-only or no-op) | `jmp`, `jsr`, `lea`, `pea`, `nop`, `rts`, `rte`, `trap`, `swap`, `ext` |
| specials (irregular encoding) | `movem`, `movep`, `addx` (Dn,Dn only), `cmpm` |

**Blind-spot instructions flagged for explicit vectors:** `movep`, `tas`, `addx`, `cmpm`, Scc `t/f/gt`, and the
debugger-macro-synthesized `move.ATTRIBUTE`/`cmp.ATTRIBUTE`/`tst.ATTRIBUTE` (real move/cmp/tst at `.b/.w/.l` —
the macro layer is a front-end (C) concern, but the encoder must handle every size these resolve to).

## 3. Architecture

Follows the existing MOVE code and spec §5.2 ("declarative fixed-field table + procedural EA escape hatch").

**Chosen: per-family encoder functions, not a generic table-interpreter.** Right-sized for ~46 mnemonics,
keeps the code shaped like the existing `encode_move`; a `#ruledef`-style interpreter is more machinery than
payoff here. Each family function reads a small fixed-field opcode const and assembles the instruction word(s)
using the **shared** `encode_ea` / `brief_ext` (unchanged). The `encode` dispatch grows one arm per family.

New `sigil-isa::m68k` vocabulary (still zero workspace deps — crate-graph rule (a) preserved):
- **`Cond` enum** (16 conditions t/f/hi/ls/cc/cs/ne/eq/vc/vs/pl/mi/ge/lt/gt/le) so `Bcc(Cond)`, `Scc(Cond)`,
  `DBcc(Cond)` collapse the per-condition explosion into one arm each. Condition code = bits 11–8.
- **`RegList(u16)`** operand for MOVEM — a 16-bit register mask in canonical d0..d7,a0..a7 order; the encoder
  applies the **predecrement bit-order reversal** (§5.5) inside `encode_movem`, keyed on the memory-operand mode.
- New `Mnemonic` variants for every family member in §2.1; new `Size`/operand handling as needed (`ext.w/.l`,
  `swap` word-only, Scc byte-only, shift count-vs-Dn form).

Encoding-hazard notes baked into the family functions:
- **MOVEM mask reversal** — mask is `d0`-lsb for all modes *except* `-(An)`, where it is bit-reversed. Both the
  register-store (to `-(An)`) and register-load (from `(An)+`) directions, plus `(An)`/`(d16,An)`/`abs.w`.
- **DBcc = non-relaxable** — single fixed 16-bit displacement; the family emits fixed 4-byte `Data`, never a
  `Relaxable`. Out-of-range displacement is an error, never a widen.
- **Branches** — `{.s, .w}` only (no `.l`); given an explicit size (always, per §2). `.s` = opcode + 8-bit disp
  in the low byte; `.w` = opcode with 8-bit field `= 0` + a 16-bit displacement extension word.
- **MOVE to/from SR & andi/ori to CCR** — fixed special opcodes; `sr`/`ccr` are not general EAs.

## 4. The `sigil-backend-m68k` adapter (thin, mirrors `sigil-backend-z80`)

New crate `crates/sigil-backend-m68k` (deps `sigil-ir` + `sigil-isa` + `sigil-span`). Implements the `Backend`
trait (§5.1). Re-exports the isa vocabulary so the AS front-end (C) takes **no** direct `sigil-isa` edge.

- **`lower_instruction(m, ops, hint)`** maps IR `Mnemonic`/`Operand` → isa `Mnemonic`/`Operand` and returns
  `Vec<Fragment>`:
  - Pinned forms (everything except bare-symbol `jmp`/`jsr`) → `Data` (or a single-candidate `Relaxable` with
    `fixed = Some(n)`), since the operand carries its explicit resolved EA form.
  - Bare-symbol `jmp`/`jsr` target → a **two-candidate `Relaxable` `{abs.w, abs.l}`** with a `fits` predicate.
    A's `fits` is the first-approximation rule (target ∈ signed-16 sign-extended range ⇒ abs.w); the
    **authoritative boundary — including `-A`'s byte effect (§5.6) — is pinned in sub-project B** by reading
    `asl`'s open-source 68000 encoder, not assumed here. **Resolution** of the candidate needs layout, so it is
    *exercised* in B; A only *produces* the candidate set and unit-tests the predicate shape.
- **`encode(cand, resolved)`** calls the isa encoder and returns `(bytes, residual_fixups)`.
- **Crate-graph guard:** add `sigil-backend-m68k` to `crate_graph.rs` rule (d) mirroring `sigil-backend-z80`
  (deps = exactly `sigil-ir` + `sigil-isa` + `sigil-span`); `sigil-isa` stays zero-workspace-dep (rule (a)).

## 5. Test strategy (asl-oracle TDD — the M0/M0.5 pattern)

The `asl` oracle is the spec. Reuse the `gen_m68k_vectors.rs` generator pattern (asl builds natively).

- **Corpus vectors** — one golden per (mnemonic, size, representative-EA) form drawn from §2. Generated by
  assembling a snippet through `asl`, extracting the bytes, committing `(source, expected-bytes)` to a golden
  file (extends `tests/m68k_golden_vectors.txt`). The generator is regenerable; the committed file is the gate.
- **§5.5 hazard vectors** (dedicated, called out so a corpus curator can't silently drop them):
  - **MOVEM**: masks across `-(An)`, `(An)+`, `(An)`, `(d16,An)`, `abs.w`, with register lists that are
    singletons, ranges, D/A-boundary-crossing ranges (`d0-a6` = all 16), and slash-joined mixes — proving the
    predecrement reversal on both store and load sides. Both `.w` and `.l`.
  - **MOVE SR/CCR**: `move.w #imm,sr`, `move.w sr,-(sp)`, `move.w (sp)+,sr`, `andi.b #$FE,ccr`, `ori.b #1,ccr`.
  - **Specials**: `movep.w/.l`, `tas.b`, `addx.b/.l` (Dn,Dn), `cmpm.w (a0)+,(a1)+`, Scc `st/sf/sgt`.
  - **DBcc**: `dbf`, `dbeq` — assert 4-byte fixed encoding.
  - **Branches**: `bra.s`/`bra.w`/`bsr.s`/`bsr.w` and a representative `Bcc.s`/`Bcc.w`, asserting the 2-wide-only
    candidate set and the `.w` extension-word form.
- **Acceptance gate** (extends `tests/encode_m68k.rs`): `all_forms_match_golden` over the grown corpus, plus a
  hazard-coverage assertion (every §5.5 vector present). Optional **stretch** (not a gate): the §12 R4
  differential fuzzer — random valid `(mnemonic, EA)` combos assembled through both `asl` and Sigil and diffed;
  the vector-oracle machinery generalizes directly, but the fixed corpus is the M1.A gate.

## 6. Explicitly out of scope (deferred to B / C / D)

- **Width-selection *resolution*** (which of `{abs.w, abs.l}` a bare `jmp`/`jsr` gets) and **Pcd16 target→disp**
  — both need layout ⇒ **sub-project B** (mirrors the `Z80JrRel8` linker-fixup pattern). A produces the
  candidate + unit-tests `fits`; B resolves it against real addresses.
- **All AS front-end fidelity** (the `.ATTRIBUTE`/`pbyte` macro layer, `struct`, `function`, `strstr`, float
  folding, operator quirks) — **sub-project C**. A takes already-resolved integers + explicit EA forms, exactly
  as the M0.5 seed does.
- **Full-ROM `sha256`** and stub-table deletion — **sub-project D**.
- **Decode / disassembly** (the ISA-sharing dual facet) — noted-but-deferred, as Z80 disasm was in M0.

## 7. Acceptance (M1.A gate)

1. `cargo test --workspace` green, `cargo clippy --workspace --all-targets -- -D warnings` clean.
2. Every §2 corpus form and every §5 hazard vector byte-matches `asl` (`all_forms_match_golden` + hazard-coverage
   assertion green).
3. `sigil-backend-m68k` present; `crate_graph.rs` updated and green; `sigil-isa` still zero-workspace-dep.
4. `sigil-isa::m68k::encode` dispatches every mnemonic in §2.1; unsupported/illegal forms return a typed
   `IsaError` (no panics, no silent wrong bytes).
