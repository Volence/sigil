//! `sigil-isa` — instruction-set encoders/decoders for the Sigil assembler.
//!
//! # Z80 encoder (table-driven)
//!
//! `z80::encode` turns a decoded `z80::Instruction` (a `z80::Mnemonic` plus zero
//! to two `z80::Operand`s carrying **already-resolved** integers — displacements,
//! immediates and addresses) into its exact Z80 machine-code bytes. It covers the
//! full sound-driver ISA subset catalogued in `SIGIL_M0_CATALOG.md` §2: the base
//! group plus the CB, ED, DD, FD, DDCB and FDCB prefix groups (~74 distinct
//! `(mnemonic, operand-form)` encodings). Dispatch is driven by a declarative
//! description of each form rather than one hand-written arm per instruction.
//!
//! The encoder does **no relaxation and no peephole rewriting**: every Z80
//! `(mnemonic, operands)` pair has exactly one length, so `jp cc` is never
//! shortened to `jr cc`. Symbol resolution is *not* this crate's job — the
//! front-end resolves symbols and hands the codec concrete integers. Relative
//! operands (`z80::Operand::Rel`) already hold the resolved displacement measured
//! from the end of the instruction.
//!
//! # Testing: the asl vector oracle
//!
//! Correctness is proven byte-for-byte against `asl` (the AS macro assembler, the
//! reference Sigil must reproduce). A committed golden-vector file
//! (`tests/z80_golden_vectors.txt`) pairs each one-line Z80 snippet in the catalog
//! corpus with the exact bytes `asl` emits; `tests/completeness.rs` maps every
//! snippet back to a `z80::Instruction`, encodes it, and asserts the bytes match —
//! so no catalog form can silently go uncovered. CI needs no `asl`: the vectors are
//! committed and are only regenerated when the corpus changes.
//!
//! # Disassembler (limited subset; full-ISA disassembly deferred)
//!
//! `z80::disassemble` is the exact inverse of `z80::encode` over the original
//! Plan-1 subset only, and that round-trip invariant is preserved. A full-ISA
//! disassembler covering every encoded form is **deferred to a later plan**; M0
//! only requires the encoder to be complete.
pub mod z80;

/// # 68000 encoder (M0.5 spike)
///
/// `m68k::encode` turns a resolved `m68k::Instruction` into big-endian bytes via a
/// procedural EA/extension-word encoder. Scope is the MOVE EA matrix; proven against
/// `asl` golden vectors (`tests/m68k_golden_vectors.txt`). See `src/m68k.rs`.
pub mod m68k;
