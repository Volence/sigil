# Banked: encode/decode round-trip self-check (categorical ISA defense)

**Date:** 2026-08-12
**Context:** the effects-P2 `add.w dN,aM` corruption (silent `D549` = `ADDX -(An),-(An)`).
The targeted fix (reject An-destination in the ALU-EA family, name `adda`/`suba`) +
spelling probes shipped in the same parcel. This note banks the CATEGORICAL defense the
user endorsed evaluating, and records why it is NOT shipped inline.

## The idea

After `encode(inst) -> bytes`, **decode `bytes` with an INDEPENDENT 68K decoder and assert
the decode is equivalent to the parsed instruction.** A single encoder bug (wrong EA field,
a mnemonic that aliases another opcode's bit pattern) cannot survive a round trip through a
decoder written from the other direction. This is the class-level guarantee the
`add`/`sub`/`cmp`-An-dest holes each needed a hand-probe for: the round trip would have
caught all three (and any future sibling) for free — the emitted `D549` decodes as `ADDX`,
not `ADD d2,a1`, so the equivalence assert fails.

Two shapes:
- **(a) CI-only pass over the full emitted stream** — decode every instruction sigil emits
  for the six shapes and assert round-trip equivalence. Broadest coverage; needs a decoder
  that spans the whole ISA sigil emits.
- **(b) spelling-matrix test** — decode each probe's bytes only. Cheap; covers exactly the
  forms the matrix enumerates (so it is only as good as the matrix — it would NOT have
  caught `add.w d,a` unless someone thought to add that row, which is the same gap the
  hand-probe has).

(a) is the one with categorical value; (b) is a nicer spelling probe, not a class defense.

## Decoder options (surveyed 2026-08-12)

| Option | Verdict | Why |
|---|---|---|
| **capstone** (Rust crate, dev-dependency) | **Not usable now** | Not in `Cargo.lock`, not in the local registry cache (`~/.cargo/registry/cache`). Adding it needs network the build env does not have. Also a C-library FFI dep — a heavier build/CI surface than a pure-Rust test dep. |
| **oracle-next `oracle-core::m68000::decode`** | **Viable but heavy** | A complete, hardware-exercised 68000 decoder already exists at `oracle-next/crates/oracle-core/src/m68000/decode.rs`. BUT importing it couples the ASSEMBLER's test suite to the EMULATOR core (a large crate with its own deps and its own release cadence), across repos. That cross-repo coupling is an architecture decision, not a test tweak — and the decoder's output type (its own `DecodedInst`) needs an equivalence mapping onto sigil's `Instruction`, which is real adapter code. |
| **sigil-native m68k decoder** | **A project, not a test** | sigil-isa has no m68k decoder today (only `z80.rs` has decode-adjacent code). Writing a faithful 68000 decoder purely to self-check the encoder is a substantial new component — worth doing as the encoder's mirror, but it is its own parcel with its own golden coverage, not a rider on a bugfix. |

## Recommendation (why banked, not shipped)

Ship the **targeted fix + spelling probes** now (done). The round-trip self-check is real
leverage but every path to it is **more than a modest test-side addition**: capstone is
unavailable offline, the oracle-next decoder is a cross-repo coupling, and a sigil-native
decoder is its own component. Banking it here per the "do not balloon the parcel" ruling.

## When to pick it up, and how

Do it as **shape (a)** when one of these is true:
1. A sigil-native m68k decoder is wanted anyway (e.g. for a `sigil disasm` subcommand, or a
   `.lst` annotator) — then the round-trip check is a near-free consumer of it, and the
   right home is a `sigil-isa` `#[cfg(test)]` `roundtrip` module plus a CI pass in the
   harness over the emitted stream.
2. The oracle-next core is already a sigil dependency for another reason (unlikely — the
   dependency direction is deliberately the other way today).

Concrete first step when picked up: a `decode_word(u16, &[u16 ext]) -> Instruction` in
`sigil-isa`, `#[cfg(test)]`-gated `assert_roundtrip(inst)` used by the existing encoder
tests, then a harness pass decoding the full six-shape image. Start with the ALU-EA and
MOVE families (where aliasing bugs bite hardest), grow to full coverage.

## Interim mitigation already in place

Until then, the encoder's own **loud rejection of non-encodable forms** (this parcel's fix,
plus the pre-existing `An`/`Dn` destination guards on `cmpa`/`movea`/`muls`/`divs`/`eor`) is
the line of defense: any form that would alias another opcode must be an explicit error, not
a silent fall-through. The lesson from this bug is that the bidirectional `Dn,<ea>` arm
silently accepted `An` as the EA — the audit that closed it should be repeated for any
future bidirectional/EA-flexible family added to `encode_alu_ea`.
