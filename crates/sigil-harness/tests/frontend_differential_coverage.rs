//! A6 (step 0 soundness): the front-end differential's generator
//! (`sigil-cli/tests/support/frontend_diff.rs`) reaches every
//! `(family, size, EA class per operand)` triple the aeon build emits.
//!
//! The differential in `sigil-cli/tests/frontend_differential.rs` is only as
//! good as its tables: a family, size or addressing mode the build uses and
//! the generator never spells is a shape on which the two front ends were
//! never compared. This test builds every shipped shape inside an encode
//! capture session (`sigil_isa::m68k::capture`), maps each encoded
//! `Instruction` to its triple, and asserts the set is a subset of the
//! generator's [`case_triples`]. A missing triple is fixed by extending the
//! generator's tables, never this assertion.
//!
//! The triple's size is the `Instruction`'s size for an emitted instruction
//! and the SPELLED size for a generated case. Every family is generated at
//! every size in `SIZES`, so a family whose size the source leaves implicit
//! (`lea` is `.l`, `jmp` is `.w`) is matched by its explicitly sized
//! spelling, and the unsized spelling is a case of its own.
//!
//! # This file must stay single-test
//!
//! The capture tap is process-global, so any encode this binary performs
//! outside the live session's build lands in that session's buffer as if the
//! build emitted it (`m68k_capstone_stream.rs`). Reference-dependent: skips
//! without the aeon tree, panics under `SIGIL_STRICT_GATE=1`.

use sigil_isa::m68k;

#[path = "../../sigil-cli/tests/support/frontend_diff.rs"]
mod frontend_diff;

use frontend_diff::{case_triples, NO_OPERAND_FAMILIES};
use m68k::capture::CaptureSession;
use m68k::{family_name, Instruction, Operand, Size};
use sigil_harness::native;
use sigil_harness::test_support::reference_tree;
use std::collections::{BTreeMap, BTreeSet};

/// The class name of an emitted operand, in `frontend_diff::ea_class`'s
/// vocabulary. Exhaustive over `Operand` with no `_` arm, so a new operand
/// kind fails to compile here instead of mapping to nothing.
fn class(op: &Operand) -> &'static str {
    match op {
        Operand::Dn(_) => "Dn",
        Operand::An(_) => "An",
        Operand::Ind(_) => "(An)",
        Operand::PostInc(_) => "(An)+",
        Operand::PreDec(_) => "-(An)",
        Operand::Disp16An(..) => "(d16,An)",
        Operand::Disp8AnXn { .. } => "(d8,An,Xn)",
        Operand::Pcd8Xn { .. } => "(d8,PC,Xn)",
        Operand::AbsW(_) => "(xxx).w",
        Operand::AbsL(_) => "(xxx).l",
        Operand::Pcd16(_) => "(d16,PC)",
        Operand::Imm(_) => "#imm",
        Operand::RegList(_) => "reglist",
        Operand::Disp(_) => "disp",
        Operand::Ccr => "ccr",
        Operand::Sr => "sr",
        Operand::Usp => "usp",
    }
}

fn size_char(s: Size) -> char {
    match s {
        Size::B => 'b',
        Size::W => 'w',
        Size::L => 'l',
        Size::S => 's',
    }
}

type Triple = (String, Option<char>, Vec<&'static str>);

fn triple(i: &Instruction) -> Triple {
    (
        family_name(i.mnemonic).to_string(),
        Some(size_char(i.size)),
        i.ops.iter().map(class).collect(),
    )
}

#[test]
fn generator_covers_every_family_size_ea_the_aeon_build_emits() {
    // `vblank.emp` is a shape source every shipped shape's build reads.
    let Some(aeon) = reference_tree(&["engine/system/vblank.emp"]) else { return };

    // Each emitted triple, with how many encodes carried it and one shape that
    // emitted it, so a failure names where to look.
    let mut emitted: BTreeMap<Triple, (usize, &'static str)> = BTreeMap::new();
    let session = CaptureSession::begin();
    for (label, profile) in native::shipped_shapes() {
        session.drain();
        let rom = native::build_rom_chained(&aeon, &profile)
            .unwrap_or_else(|e| panic!("shape `{label}`: build failed: {e}"));
        assert!(!rom.is_empty(), "shape `{label}`: empty ROM");
        let pairs = session.drain();
        assert!(
            !pairs.is_empty(),
            "shape `{label}` captured 0 instructions: the capture tap is disconnected, \
             which must never read as covered"
        );
        for (inst, _) in &pairs {
            let fam = family_name(inst.mnemonic);
            if NO_OPERAND_FAMILIES.contains(&fam) {
                // The generator leaves these out because they take no operand;
                // hold that claim to the build.
                assert!(
                    inst.ops.is_empty(),
                    "`{fam}` is in NO_OPERAND_FAMILIES but `{label}` emitted it with operands: {inst:?}"
                );
                continue;
            }
            let e = emitted.entry(triple(inst)).or_insert((0, label));
            e.0 += 1;
        }
    }
    drop(session);

    let generated = case_triples();
    let missing: Vec<String> = emitted
        .iter()
        .filter(|(t, _)| !generated.contains(*t))
        .map(|((f, z, ops), (n, shape))| {
            format!("({f}, {z:?}, [{}]) x{n}, e.g. in `{shape}`", ops.join(", "))
        })
        .collect();
    eprintln!(
        "frontend-differential coverage: {} emitted triples ({} encodes), {} generated triples, {} missing",
        emitted.len(),
        emitted.values().map(|(n, _)| n).sum::<usize>(),
        generated.len(),
        missing.len()
    );
    let families: BTreeSet<&str> = emitted.keys().map(|(f, _, _)| f.as_str()).collect();
    eprintln!("emitted families with operands: {}", families.len());
    assert!(
        missing.is_empty(),
        "the generator misses {} triple(s) the aeon build emits (extend its tables):\n{}",
        missing.len(),
        missing.join("\n")
    );
}
