# Parcel packet: m68k encode/decode round-trip self-check

**Date:** 2026-08-19 · **Branch:** `feat/m68k-roundtrip` · **Spec:**
`2026-08-12-encode-decode-roundtrip-selfcheck.md` (shape (a), sigil-native decoder).
Byte bar: byte-neutral by construction — no golden, pins, provenance, or repin file
touched; no emitted ROM byte changes (the only production-code change is a
capture tap on `encode`'s RESULT).

## What shipped

1. **`crates/sigil-isa/src/m68k_decode.rs`** — a sigil-native 68000 decoder,
   written from the opcode-map direction (M68000PRM Section 8 bit patterns),
   covering exactly the forms `m68k::encode` emits. Everything else — real
   68000 instructions sigil never emits (`addx -(An),-(An)`, `subx`, `sbcd`,
   `abcd`, `exg`, `rox*`, `bchg`, `chk`, `moves`, line-A/F) and 68020-only
   extension bits (brief-ext scale/full bits, nonzero high byte on a byte
   immediate) — is a loud, named `DecodeError` (`Unknown` / `Truncated` /
   `BadExtension`), never a best-effort operand. The decoder validates every
   EA against the SAME `EaSet` operand-table row the encoder's family arm
   enforces (the constants went `pub(crate)` for this), so decode-side and
   encode-side legality cannot drift apart silently.
2. **`roundtrip_check(inst, bytes)` / `assert_roundtrip(inst)`** (public in
   `m68k_decode`; test-only in usage). Wired into the existing suites:
   - `tests/encode_m68k.rs` — `check()` round-trips every golden-matched form;
     new `all_forms_roundtrip_through_the_decoder` walks the whole corpus with
     a count assert derived from `corpus.len()`.
   - `tests/ea_class_rejects.rs` — `accept()` round-trips every accepted form,
     so the hand-written spelling matrix now carries the categorical defense too.
3. **`m68k::capture`** — a process-global, session-scoped tap on `encode`:
   while a `CaptureSession` is live, every successful encode records its
   `(Instruction, bytes)` pair. Global (AtomicBool + Mutex), NOT thread-local,
   because both front-ends run lowering on spawned big-stack threads
   (`sigil-frontend-emp/src/eval/mod.rs:1820`, `sigil-frontend-as/src/expr.rs:165`)
   and a thread-local sink would silently lose those encodes. Idle cost: one
   relaxed atomic load per encode.
4. **`crates/sigil-harness/tests/m68k_roundtrip_stream.rs`** — the CI pass over
   the full emitted stream: for each of the seven shipped shapes,
   `build_rom_chained` runs inside a capture session and every captured pair
   must round-trip. Capture-at-encode was chosen over disassembling the ROM
   image per the spec's recommendation (raw-image disassembly needs code/data
   separation — a tar pit); the tap sees the exact instruction stream with
   zero heuristics. The stream is a superset of the final ROM's instructions
   (relaxation-ladder rungs, fixup placeholders, and comptime trial encodes
   are captured too — all must round-trip regardless, so the superset only
   widens coverage).
5. **`family_name` / `ALL_FAMILY_NAMES`** in `m68k.rs` — enum-locked family
   labels (no `_` arm, so a new mnemonic variant fails compilation until
   classified), plus `Cond::from_cc`. Coverage gates derive "all families"
   from the constant, never from a measured run.

## The equivalence relation (doc'd in full at `roundtrip_check`)

Equality is over `canonicalize()`d instructions — exactly the many-to-one
freedom the ENCODING has, nothing looser:

- **Rule M** — `move.w/.l <ea>,An` ≡ `movea`: one opcode layout (dest mode 001).
- **Rule B** — `Bcc(T)` ≡ `bra`, `Bcc(F)` ≡ `bsr`: cc 0/1 ARE bra/bsr.
- **Rule S** — size-less encodings (`moveq`, bit ops, `tas`, `Scc`, SR/CCR
  moves, `jmp`/`jsr`/`lea`/`pea`, fixed words, `dbcc`) normalize to one
  canonical size per form, via a single `canonical_size` table both sides use.
- **Rule I** — width-stored immediates compare modulo the field width
  (`#-1` ≡ `#$FFFF` at `.w`); 3-bit register numbers compare masked.

Everything else — every operand's EA mode AND register, displacements, masks,
conditions, the mnemonic family — must match exactly; that is what catches a
wrong EA field or an aliased opcode word. A decode failure is ALWAYS a check
failure, never a skip.

One slice-delimited leniency, documented at `decode_exact`: a 2-byte slice
`6x00` decodes as the `.s`-branch fixup placeholder (`Disp(0)`), because
`M68kBackend::lower_branch` legitimately encodes that placeholder before the
linker patches `PcRel8`, and a 2-byte slice cannot hold the word form. In a
real stream `6x00` still reads as the word form (4 bytes).

## Coverage numbers (2026-08-19 run, all seven shapes, ~4.4s)

| shape | instructions round-tripped |
|---|---|
| sonic4 plain | 12163 |
| sonic4 debug | 14585 |
| demo plain | 8813 |
| demo debug | 10174 |
| config_a | 14713 |
| config_b | 11883 |
| lean | 12104 |
| **total** | **84435** |

Families: 61 of the 62 encodable families appear in the union (everything
except `illegal`, which is on the two-sided `NOT_IN_STREAM` list — the test
fails if a listed family appears OR an unlisted family vanishes). Counts are
printed per family on every run (`move` 23248, `bcc` 16854, `lea` 5158, …
down to `cmpm` 5).

Derived-expectation notes: the per-shape floor is `> 0` plus structural
relations (debug > plain per game, sonic4 > demo) — direction checks derived
from what the shapes are, not pinned magnitudes; the corpus test's count
assert derives from `corpus.len()`; the family list derives from the
`Mnemonic` enum via a compiler-forced exhaustive match.

## Red-first mutant records

All four run against the shipped tests, then restored; exact failure text below.

**MUTANT A — the motivating `D549` class** (encoder emits the ADDX alias for
`add.w d2,a1`). Mutation in `m68k.rs::encode_alu_ea`: the An-destination
reject arm `(Operand::Dn(_), Operand::An(_)) => {` guarded with `if false`,
and the fall-through arm's `EaSet::MEMORY_ALTERABLE` changed to `EaSet::ALL`.
Probe `assert_roundtrip(add.w d2,a1)` failed:

```
round-trip DECODE failed: unknown opcode D549: addx -(Ay),-(Ax) is not in sigil's emitted set
  encoded from: Instruction { mnemonic: Add, size: W, ops: [Dn(2), An(1)] }
  bytes: [D5, 49]
```

**MUTANT B — aliased opcode family on an emitted form.** Mutation:
`Mnemonic::Suba => 0b1001,` → `Mnemonic::Suba => 0b1011,` (suba now emits
cmpa's base word). `all_forms_roundtrip_through_the_decoder` failed:

```
1 of 114 corpus forms failed the round trip:
suba.l a2,a3: round-trip MISMATCH:
  encoded:   Instruction { mnemonic: Suba, size: L, ops: [An(2), An(3)] }
  bytes:     [B7, CA]
  decoded:   Instruction { mnemonic: Cmpa, size: L, ops: [An(2), An(3)] }
```

**MUTANT C — wrong EA field** (the §5.5 MOVE dest-field swap hazard).
Mutation in `encode_move`: `(dst_reg << 9) | (dst_mode << 6)` swapped to
`(dst_mode << 9) | (dst_reg << 6)`. Same test failed on 7 of 114 forms, e.g.:

```
move.w d1,(a0): round-trip MISMATCH:
  encoded:   Instruction { mnemonic: Move, size: W, ops: [Dn(1), Ind(0)] }
  bytes:     [34, 01]
  decoded:   Instruction { mnemonic: Move, size: W, ops: [Dn(1), Dn(2)] }
```

**MUTANT D — capture tap disconnected** (`capture::record(inst, bytes)`
replaced with a no-op). The harness stream pass failed:

```
shape `sonic4 plain` captured 0 instructions — the capture tap is disconnected
(or the build encoded nothing), which must never read as a pass
```

**MUTANT E — mutant B against the STREAM pass** (proves the harness gate is
red-able on real emitted code, independent of the golden corpus): 98
instructions failed across the shapes, first:

```
shape `sonic4 plain`: round-trip MISMATCH:
  encoded:   Instruction { mnemonic: Suba, size: W, ops: [Imm(0), An(1)] }
  bytes:     [B2, FC, 00, 00]
  decoded:   Instruction { mnemonic: Cmpa, size: W, ops: [Imm(0), An(1)] }
```

## Runners

- `cargo test -p sigil-isa` — decoder unit tests (`m68k_decode::tests`), the
  corpus round-trip gate, the wired golden/spelling suites, the family-map test.
- `AEON_DIR=… cargo test -p sigil-harness --test m68k_roundtrip_stream` — the
  full-stream pass (part of the workspace suite CI runs).

## Banked / deferred

- **`decode_one` streaming use** (a `sigil disasm` subcommand, `.lst`
  annotation): the decoder returns `(Instruction, consumed)` and is stream-ready,
  but no CLI surface ships in this parcel — out of scope.
- **Z80 round-trip over the emitted stream**: `z80::disassemble` already
  inverts the Plan-1 subset; extending to the full Z80 ISA + a capture tap is
  its own parcel (this one is the m68k defense the D549 incident motivated).
- **Linker-lowered words** (`lower_jmp_jsr_abs`, relax-ladder rung bytes built
  by hand in `sigil-backend-m68k`): the LADDER rungs' branch/jmp forms are
  covered because the same encodings also flow through `encode` elsewhere in
  every shape, but the hand-built byte blocks themselves bypass the tap. A
  follow-up could decode those four fixed shapes at construction; banked as a
  small hardening, since their bytes are pinned by the backend's own tests.
- **`PcRel8` zero-displacement soundness — ESTABLISHED, nothing to do**: the
  decoder's placeholder rule (2-byte `6x00` slice = pre-link placeholder) is
  closed by `sigil-link/src/lib.rs`'s `PcRel8` arm, which already rejects a
  RESOLVED displacement of 0 with a loud diagnostic ("the 68000 word-form
  escape, not a branch to the next instruction"), so a final `.s` branch can
  never carry the byte pattern the leniency tolerates.
