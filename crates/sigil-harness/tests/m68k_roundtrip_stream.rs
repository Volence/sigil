//! The m68k encode/decode round-trip pass over the FULL emitted stream.
//!
//! For every shipped build shape, a whole-ROM build runs inside a
//! `m68k::capture` session, so every 68000 instruction the build encodes —
//! both front-ends, every module, fixup placeholders included — is captured as
//! an `(Instruction, bytes)` pair at the one point where both are in hand.
//! Each pair must survive `m68k_decode::roundtrip_check`: the independent
//! decoder reads the bytes back and the canonical forms must match. A single
//! encoder bug that emits a wrong EA field or an aliasing neighbour opcode
//! (the `add.w d2,a1` → `D549` = `ADDX -(An),-(An)` class, 2026-08-12) cannot
//! survive this pass, for ANY instruction any shape actually emits.
//!
//! The capture tap is chosen over disassembling the ROM image deliberately:
//! raw-image disassembly needs code/data separation, which is a tar pit; the
//! tap sees exactly the instruction stream with zero heuristics.
//!
//! Coverage is accounted, not assumed:
//! - a shape with ZERO captured instructions fails (a disconnected tap must
//!   never render as green);
//! - per-shape counts must satisfy structural relations derived from what the
//!   shapes ARE (debug ⊇ plain code, sonic4 ⊇ demo code), not a pinned count;
//! - the family union across all shapes is checked TWO-SIDED against the
//!   enum-derived `ALL_FAMILY_NAMES`: a family missing without a
//!   `NOT_IN_STREAM` row fails (coverage silently narrowed), and a
//!   `NOT_IN_STREAM` row that shows up captured fails (the list is stale).

use sigil_harness::native;
use sigil_harness::test_support::aeon_dir;
use sigil_isa::m68k::{capture::CaptureSession, family_name, ALL_FAMILY_NAMES};
use sigil_isa::m68k_decode::roundtrip_check;
use std::collections::{BTreeMap, BTreeSet};

/// Families in sigil's encodable set that NO shipped shape's encode stream
/// contains today. Each row is a claim the test enforces in both directions —
/// a listed family that shows up captured fails as stale, so the list cannot
/// quietly rot into a skip list.
///
/// - `illegal`: the deliberate-trap word; nothing in the shipped games plants
///   one. (Every other family, `tas`/`movep`/`divs`/`divu` included, IS
///   encoded somewhere across the seven shapes.)
const NOT_IN_STREAM: &[&str] = &["illegal"];

#[test]
fn every_emitted_m68k_instruction_roundtrips_in_every_shipped_shape() {
    let aeon = aeon_dir();
    let session = CaptureSession::begin();

    let mut failures: Vec<String> = Vec::new();
    let mut per_shape: Vec<(&'static str, usize)> = Vec::new();
    let mut families: BTreeMap<&'static str, usize> = BTreeMap::new();

    for (label, profile) in native::shipped_shapes() {
        // Discard anything encoded outside this shape's build (prior residue).
        session.drain();
        let rom = native::build_rom_chained(&aeon, &profile)
            .unwrap_or_else(|e| panic!("shape `{label}`: build failed: {e}"));
        assert!(!rom.is_empty(), "shape `{label}`: empty ROM");
        let pairs = session.drain();

        for (inst, bytes) in &pairs {
            *families.entry(family_name(inst.mnemonic)).or_default() += 1;
            if let Err(msg) = roundtrip_check(inst, bytes) {
                failures.push(format!("shape `{label}`: {msg}"));
            }
        }
        per_shape.push((label, pairs.len()));
    }
    drop(session);

    // Failures first, every word named. Cap the rendering, never the count.
    if !failures.is_empty() {
        let shown = failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n");
        panic!(
            "{} instruction(s) failed the round trip (first {} shown):\n{shown}",
            failures.len(),
            failures.len().min(40)
        );
    }

    // Visible accounting.
    println!("per-shape captured m68k instructions:");
    for (label, n) in &per_shape {
        println!("  {label:>14}: {n}");
    }
    println!("per-family counts (union of all shapes): {families:?}");

    // A tap that captured nothing is a broken pass, not a clean one.
    let count = |want: &str| -> usize {
        per_shape
            .iter()
            .find(|(l, _)| *l == want)
            .unwrap_or_else(|| panic!("shape `{want}` missing from the walk"))
            .1
    };
    for (label, n) in &per_shape {
        assert!(
            *n > 0,
            "shape `{label}` captured 0 instructions — the capture tap is disconnected \
             (or the build encoded nothing), which must never read as a pass"
        );
    }

    // Structural relations derived from what the shapes are: DEBUG builds carry
    // the plain code PLUS debug-only modules, and sonic4 carries the full game
    // where demo is the engine-only boot. These are direction checks, not
    // pinned magnitudes, so a legitimately grown/shrunk build stays green while
    // a half-captured stream (e.g. one front-end's encodes lost) fails.
    assert!(
        count("sonic4 debug") > count("sonic4 plain"),
        "sonic4 debug ({}) must emit more instructions than plain ({})",
        count("sonic4 debug"),
        count("sonic4 plain")
    );
    assert!(
        count("demo debug") > count("demo plain"),
        "demo debug ({}) must emit more instructions than plain ({})",
        count("demo debug"),
        count("demo plain")
    );
    assert!(
        count("sonic4 plain") > count("demo plain"),
        "sonic4 plain ({}) must emit more instructions than the engine-only demo ({})",
        count("sonic4 plain"),
        count("demo plain")
    );

    // Two-sided family coverage against the enum-derived list.
    let seen: BTreeSet<&str> = families.keys().copied().collect();
    let expected: BTreeSet<&str> = ALL_FAMILY_NAMES
        .iter()
        .copied()
        .filter(|f| !NOT_IN_STREAM.contains(f))
        .collect();
    let missing: Vec<&str> = expected.difference(&seen).copied().collect();
    assert!(
        missing.is_empty(),
        "families in sigil's encodable set but absent from EVERY shape's stream and \
         not on the NOT_IN_STREAM list: {missing:?} — either the capture narrowed \
         (a bug in this pass) or the corpus stopped using them (move them to the \
         list with a reason)"
    );
    let stale: Vec<&str> = NOT_IN_STREAM.iter().copied().filter(|f| seen.contains(f)).collect();
    assert!(
        stale.is_empty(),
        "NOT_IN_STREAM rows that ARE now emitted: {stale:?} — delete the stale rows \
         so their coverage claim stays true"
    );
}
