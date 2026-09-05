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
//! - the family union across all shapes is checked in all three directions
//!   against the enum-derived `ALL_FAMILY_NAMES`: a family missing without a
//!   `NOT_IN_STREAM` row fails (coverage silently narrowed), a `NOT_IN_STREAM`
//!   row that shows up captured fails (the list is stale), and a captured
//!   family absent from `ALL_FAMILY_NAMES` fails (an unclassified name).
//!
//! This file must stay SINGLE-TEST (or every added test must open its own
//! capture session): the capture tap is process-global, so any encode this
//! binary performs outside the session's build — another test running in a
//! parallel thread included — lands in the live session's buffer as if the
//! build emitted it.

use sigil_harness::native;
use sigil_harness::test_support::reference_tree;
use sigil_isa::m68k::{capture::CaptureSession, family_name, ALL_FAMILY_NAMES};
use sigil_isa::m68k_decode::roundtrip_check;
use std::collections::{BTreeMap, BTreeSet};

/// Families in sigil's encodable set that NO shipped shape's encode stream
/// contains today. Each row is a claim the test enforces in both directions —
/// a listed family that shows up captured fails as stale, so the list cannot
/// quietly rot into a skip list.
///
/// - `illegal`: the deliberate-trap word; nothing in the shipped games plants
///   one.
///
/// The seven below arrived together when the missing 68000 instruction lines
/// were encoded for the Sonic 1 corpus (2026-09-03). Aeon is a different source
/// tree and uses none of them — verified by grep over the reference tree at
/// `4f5ad5a1`: the only textual hits are a COMMENT (`player_climb.emp:363`
/// describing a `bchg` that is not written) and two `debugger.asm` equates
/// naming the USP register in an exception-screen flag. Their coverage is the
/// asl-minted golden vectors and the 65,536-word capstone sweep, both of which
/// reach every legal form rather than whatever one game happens to compile.
///
/// - `bchg`: aeon's bit work is `bset`/`bclr`/`btst`, all three of which ARE
///   captured; nothing toggles a bit.
/// - `exg`: no register exchange anywhere in the engine or either game.
/// - `roxl` / `roxr`: aeon shifts and rotates without the X bit
///   (`asl`/`asr`/`lsl`/`lsr`/`rol`/`ror` are all captured).
/// - `move-to-ccr`: aeon sets carry with `andi.b #$FE,ccr` / `ori.b #1,ccr`
///   (14 sites, and both `andi-ccr` and `ori-ccr` ARE captured), never by
///   moving a whole EA into CCR.
/// - `move-to-usp` / `move-from-usp`: supervisor-mode-only, and aeon never
///   leaves supervisor mode, so it has no user stack pointer to set.
const NOT_IN_STREAM: &[&str] = &[
    "illegal",
    "bchg",
    "exg",
    "roxl",
    "roxr",
    "move-to-ccr",
    "move-to-usp",
    "move-from-usp",
];

#[test]
fn every_emitted_m68k_instruction_roundtrips_in_every_shipped_shape() {
    // Skip-green without the aeon reference; panic under SIGIL_STRICT_GATE=1.
    // `vblank.emp` is a shape source every shipped shape's build reads.
    let Some(aeon) = reference_tree(&["engine/system/vblank.emp"]) else { return };
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
            "shape `{label}` captured 0 instructions, the capture tap is disconnected \
             (or the build encoded nothing), which must never read as a pass"
        );
    }

    // Structural relations derived from what the shapes are: DEBUG builds carry
    // the plain code PLUS debug-only modules, and sonic4 carries the full game
    // where demo is the engine-only boot. Direction checks, not pinned
    // magnitudes — they catch a shape-selective regression (one shape's build
    // quietly emitting far less). A UNIFORM proportional capture loss preserves
    // these inequalities; the defenses against capture loss itself are the
    // per-shape >0 asserts above and the family-union checks below.
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

    // Family coverage against the enum-derived list, all three directions.
    let seen: BTreeSet<&str> = families.keys().copied().collect();
    let all: BTreeSet<&str> = ALL_FAMILY_NAMES.iter().copied().collect();
    let unclassified: Vec<&str> = seen.difference(&all).copied().collect();
    assert!(
        unclassified.is_empty(),
        "captured families not in ALL_FAMILY_NAMES: {unclassified:?}, unknown family, \
         classify it in ALL_FAMILY_NAMES (sigil-isa m68k.rs) so both coverage \
         directions can see it"
    );
    let expected: BTreeSet<&str> = ALL_FAMILY_NAMES
        .iter()
        .copied()
        .filter(|f| !NOT_IN_STREAM.contains(f))
        .collect();
    let missing: Vec<&str> = expected.difference(&seen).copied().collect();
    assert!(
        missing.is_empty(),
        "families in sigil's encodable set but absent from EVERY shape's stream and \
         not on the NOT_IN_STREAM list: {missing:?}, either the capture narrowed \
         (a bug in this pass) or the corpus stopped using them (move them to the \
         list with a reason)"
    );
    let stale: Vec<&str> = NOT_IN_STREAM.iter().copied().filter(|f| seen.contains(f)).collect();
    assert!(
        stale.is_empty(),
        "NOT_IN_STREAM rows that ARE now emitted: {stale:?}, delete the stale rows \
         so their coverage claim stays true"
    );
}
