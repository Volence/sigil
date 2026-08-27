//! supersede_fixpoint — the one thing an explicit `--supersede-tip` is allowed to do to
//! the fixpoint, and the four things it still is not.
//!
//! `refreeze --freeze` normally appends nothing when the regenerated goldens reproduce
//! the chain tip: there is no new bytes-fact to record, and that no-op is the machinery's
//! own regression test. But an entry turns RED for reasons that do not live in the
//! emitted bytes — a harness-side defect, a wrong declaration, a check the frozen shapes
//! never satisfied — and the fix for those moves no bytes at all. The re-freeze then
//! lands on the fixpoint, appends nothing, and the tip stays red; `--supersede-tip` never
//! gets consulted, because the fixpoint returns first. The chain cannot advance past a
//! red entry until some unrelated byte-MOVING parcel happens along.
//!
//! So an explicit `--supersede-tip` passes the fixpoint and appends an ORDINARY
//! `[[entry]]` whose goldens equal its predecessor's. This file gates both halves: that
//! the abandonment gets through, and that everything guarding it did not move —
//!
//!   * a byte-neutral freeze WITH the flag on a red tip appends one entry;
//!   * a byte-neutral freeze WITHOUT the flag appends nothing and leaves
//!     `provenance.toml` BYTE-IDENTICAL;
//!   * a green tip is refused;
//!   * a tip with no strict run is refused, both on a chain where the rule has armed and
//!     on one where it has not;
//!   * the appended entry repeats its predecessor's CRC set exactly, and the resulting
//!     chain parses and passes `provenance::check`.
//!
//! These drive [`provenance::freeze_into`] — the real ledger half of `--freeze`, the same
//! function the binary calls — against a scratch `golden/` holding throwaway blobs. The
//! binary itself cannot be driven here: `--freeze` rebuilds seven ROMs and rewrites the
//! REAL chain.
//!
//! Runner: `cargo test -p sigil-harness --test supersede_fixpoint` (and the plain
//! `cargo test -p sigil-harness` set — no aeon tree, no sigil build, no strict gate).

use sigil_harness::provenance::{
    self, Applied, Chain, StrictRun, Target, ASL_WITNESS, OUTCOME_FAILED, OUTCOME_PASSED,
    SUPERSEDE_OF_A_GREEN_TIP, SUPERSEDE_WITHOUT_A_RED_RUN,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Two 40-char SHAs that are only ever compared for equality and well-formedness.
const AEON_REV: &str = "9bba8700d09d3c87bdad3e2e0f13c7389f2da4cc";
const SIGIL_REV: &str = "e671a5b6383f82a20683ca838035e655b2c803c9";

/// The scratch shapes. Two, not one: a single-target fixture cannot show that the
/// byte-neutral entry repeats the WHOLE set.
fn golden_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("alpha".to_string(), "alpha.bin".to_string()),
        ("beta".to_string(), "beta.bin".to_string()),
    ])
}

/// Write the throwaway blobs and DERIVE the target set from them with the same
/// `recompute_targets` a real freeze uses — never a hand-written CRC. `fill` is what
/// makes one generation of blobs differ from the next.
fn write_blobs(golden: &Path, fill: u8) -> BTreeMap<String, Target> {
    let mut ends = BTreeMap::new();
    for (i, (key, file)) in golden_map().iter().enumerate() {
        let len = 512 + i * 64;
        let bytes: Vec<u8> = (0..len).map(|n| fill.wrapping_add(n as u8)).collect();
        std::fs::write(golden.join(file), &bytes).expect("write scratch blob");
        // Inside the blob, so `assembled_anchor_crc` has a real prefix to fold.
        ends.insert(key.clone(), len - 16);
    }
    provenance::recompute_targets(golden, &golden_map(), &ends).expect("recompute scratch targets")
}

/// A well-formed strict record for `targets`, of the given outcome. Every cross-checked
/// field is derived from the entry it will sit on, exactly as `--attest` derives them.
fn strict_run(outcome: &str, targets: &BTreeMap<String, Target>) -> StrictRun {
    let failed = usize::from(outcome == OUTCOME_FAILED);
    StrictRun {
        outcome: outcome.to_string(),
        sigil_rev: SIGIL_REV.to_string(),
        aeon_rev: AEON_REV.to_string(),
        strict_bodies: 29,
        suites: 349,
        passed: 4037,
        failed,
        ignored: 4,
        skips: 0,
        ran_at: "unix:1787834118".to_string(),
        failing: if failed == 1 { vec!["a_test_that_was_red".to_string()] } else { Vec::new() },
        expected_tests: Vec::new(),
        goldens: targets
            .iter()
            .map(|(k, t)| (k.clone(), format!("{}/{}", t.full_crc, t.full_size)))
            .collect(),
    }
}

/// What strict record, if any, the fixture's TIP carries.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tip {
    /// A well-formed RED run — the state that unlocks abandonment.
    Red,
    /// A well-formed GREEN run.
    Green,
    /// No run at all, on a chain where an EARLIER entry has one, so the rule has armed.
    UnattestedArmed,
    /// No run anywhere in the chain — the rule has not armed at all.
    UnattestedRatchet,
}

/// A scratch `golden/` whose blobs and chain agree, with the tip in the requested state.
/// Returns the temp dir (kept alive by the caller) and the target set the blobs hold.
fn fixture(tip: Tip) -> (tempfile::TempDir, BTreeMap<String, Target>) {
    let dir = tempfile::tempdir().expect("scratch golden dir");
    let golden = dir.path();
    let targets = write_blobs(golden, 0x10);

    // Root + tip carry the SAME targets: no anchor moves, so the fixture needs no A/B
    // evidence and cannot fail the discipline sweep for a reason this file is not about.
    let mut src = provenance::render_entry("root", ASL_WITNESS, AEON_REV, "the scratch root", &targets);
    if tip == Tip::UnattestedArmed {
        src.push_str(&provenance::render_strict(&strict_run(OUTCOME_PASSED, &targets)));
    }
    src.push_str(&provenance::render_entry("tip", "ab/scratch", AEON_REV, "the scratch tip", &targets));
    match tip {
        Tip::Red => src.push_str(&provenance::render_strict(&strict_run(OUTCOME_FAILED, &targets))),
        Tip::Green => src.push_str(&provenance::render_strict(&strict_run(OUTCOME_PASSED, &targets))),
        Tip::UnattestedArmed | Tip::UnattestedRatchet => {}
    }
    std::fs::write(golden.join("provenance.toml"), &src).expect("write scratch chain");

    // POSITIVE CONTROL. A fixture that does not actually hold the state the test names
    // would make every assertion below vacuous, so it is measured, not assumed — and the
    // measurement is the module's own predicates, not this file's opinion.
    let chain = load(golden);
    assert!(
        provenance::check(golden, &chain).is_empty(),
        "the fixture chain must be clean before the test acts: {:?}",
        provenance::check(golden, &chain)
    );
    let t = chain.tip().expect("fixture tip");
    match tip {
        Tip::Red => assert!(t.is_red(), "fixture asked for a RED tip but the tip is not red"),
        Tip::Green => assert!(t.is_attested(), "fixture asked for a GREEN tip but it is not attested"),
        Tip::UnattestedArmed | Tip::UnattestedRatchet => assert!(
            !t.is_red() && !t.is_attested(),
            "fixture asked for a tip with NO strict run but it carries one"
        ),
    }
    (dir, targets)
}

fn load(golden: &Path) -> Chain {
    let src = std::fs::read_to_string(golden.join("provenance.toml")).expect("read scratch chain");
    provenance::parse(&src).expect("parse scratch chain")
}

fn chain_bytes(golden: &Path) -> Vec<u8> {
    std::fs::read(golden.join("provenance.toml")).expect("read scratch chain bytes")
}

/// THE DEADLOCK CASE. A red tip, a re-freeze that reproduces its goldens exactly, and an
/// explicit abandonment: the entry must land.
#[test]
fn a_byte_neutral_supersede_of_a_red_tip_appends_an_entry() {
    let (dir, fresh) = fixture(Tip::Red);
    let golden = dir.path();
    let before = load(golden).entry.len();

    let applied = provenance::freeze_into(
        golden,
        "the-successor",
        "ab/scratch",
        AEON_REV,
        "",
        &fresh,
        Some("the fix that closed the red run moved no bytes"),
    )
    .expect("a byte-neutral supersede of a red tip must append");

    match applied {
        Applied::Appended { abandoned, byte_neutral, chain_len, .. } => {
            assert_eq!(abandoned.as_deref(), Some("tip"), "the abandoned entry must be named");
            assert!(byte_neutral, "the goldens were unchanged; the append must know it");
            assert_eq!(chain_len, before + 1, "exactly one entry appended");
        }
        other => panic!("expected an append past the fixpoint, got {other:?}"),
    }
}

/// THE PROPERTY BEING PRESERVED. Same fixture, same goldens, no flag: the fixpoint is
/// untouched, and `provenance.toml` is not merely un-appended-to but BYTE-IDENTICAL.
#[test]
fn a_byte_neutral_refreeze_without_the_flag_appends_nothing_and_leaves_the_file_untouched() {
    let (dir, fresh) = fixture(Tip::Red);
    let golden = dir.path();
    let before = chain_bytes(golden);

    let applied = provenance::freeze_into(golden, "the-successor", "ab/scratch", AEON_REV, "", &fresh, None)
        .expect("a no-op re-freeze must succeed");

    assert_eq!(
        applied,
        Applied::Fixpoint { tip: "tip".to_string() },
        "without the flag a byte-neutral re-freeze must report the fixpoint"
    );
    assert_eq!(before, chain_bytes(golden), "the fixpoint must leave provenance.toml byte-identical");
}

/// A green tip is not being abandoned, byte-neutral or not.
#[test]
fn a_green_tip_is_still_refused() {
    let (dir, fresh) = fixture(Tip::Green);
    let golden = dir.path();
    let before = chain_bytes(golden);

    let err = provenance::freeze_into(golden, "the-successor", "ab/scratch", AEON_REV, "", &fresh, Some("why"))
        .expect_err("superseding a green tip must be refused");

    assert_eq!(err, SUPERSEDE_OF_A_GREEN_TIP, "the refusal must be the green-tip one");
    assert_eq!(before, chain_bytes(golden), "a refusal must not touch the ledger");
}

/// A tip with no strict run at all cannot be abandoned — abandoning it would be a way to
/// SKIP the run rather than to record one that came back red. Both chains: the one where
/// the rule has armed and the one where it has not.
#[test]
fn a_tip_with_no_strict_run_is_still_refused() {
    for (state, expect) in [
        (Tip::UnattestedArmed, None),
        (Tip::UnattestedRatchet, Some(SUPERSEDE_WITHOUT_A_RED_RUN)),
    ] {
        let (dir, fresh) = fixture(state);
        let golden = dir.path();
        let before = chain_bytes(golden);

        let err = provenance::freeze_into(golden, "the-successor", "ab/scratch", AEON_REV, "", &fresh, Some("why"))
            .expect_err("superseding a tip with no strict run must be refused");

        match expect {
            Some(want) => assert_eq!(err, want, "the unarmed chain must refuse by name"),
            // The armed chain's refusal is `append_gate`'s own prose, derived from the
            // chain; assert the fact it must carry rather than a copy of the sentence.
            None => assert!(
                err.contains("carries no strict run"),
                "the armed chain must refuse for the missing run, got: {err}"
            ),
        }
        assert_eq!(before, chain_bytes(golden), "a refusal must not touch the ledger");
    }
}

/// The appended entry is an ORDINARY entry that happens to repeat its predecessor's CRC
/// set — no second kind of record — and the chain it produces still parses and validates.
#[test]
fn the_appended_entry_repeats_its_predecessors_goldens_and_the_chain_still_validates() {
    let (dir, fresh) = fixture(Tip::Red);
    let golden = dir.path();

    provenance::freeze_into(
        golden,
        "the-successor",
        "ab/scratch",
        AEON_REV,
        "",
        &fresh,
        Some("the fix that closed the red run moved no bytes"),
    )
    .expect("a byte-neutral supersede of a red tip must append");

    let chain = load(golden);
    let n = chain.entry.len();
    let (abandoned, successor) = (&chain.entry[n - 2], &chain.entry[n - 1]);

    assert_eq!(abandoned.name, "tip", "the entry before the successor is the abandoned tip");
    assert_eq!(successor.name, "the-successor");
    assert_eq!(
        abandoned.targets, successor.targets,
        "the byte-neutral entry must repeat its predecessor's whole CRC set"
    );
    assert_eq!(
        successor.targets, fresh,
        "and that set must be what the blobs on disk actually hold"
    );
    let sup = abandoned.superseded.as_ref().expect("the abandoned entry must carry the record");
    assert_eq!(sup.by, successor.name, "the abandonment must name the entry that followed it");
    assert!(
        successor.note.contains(&abandoned.name) && successor.note.contains("byte-identical"),
        "the successor must be legible as a byte-neutral abandonment from its own fields, got: {:?}",
        successor.note
    );
    assert!(
        provenance::check(golden, &chain).is_empty(),
        "the resulting chain must validate: {:?}",
        provenance::check(golden, &chain)
    );
}

/// The byte-MOVING supersede is unchanged: it still appends, and it is NOT marked
/// byte-neutral, so the new note never attaches to an entry whose bytes really moved.
#[test]
fn a_byte_moving_supersede_still_appends_and_is_not_byte_neutral() {
    let (dir, _) = fixture(Tip::Red);
    let golden = dir.path();
    // A real byte-moving freeze: the blobs are regenerated, so the fresh set differs.
    let moved = write_blobs(golden, 0x77);

    let applied = provenance::freeze_into(
        golden,
        "the-successor",
        "ab/scratch",
        AEON_REV,
        "an ordinary byte-moving parcel",
        &moved,
        Some("the red run's fix moved bytes"),
    )
    .expect("a byte-moving supersede must still append");

    match applied {
        Applied::Appended { abandoned, byte_neutral, .. } => {
            assert_eq!(abandoned.as_deref(), Some("tip"));
            assert!(!byte_neutral, "the goldens moved; this append is not byte-neutral");
        }
        other => panic!("expected an append, got {other:?}"),
    }
    let chain = load(golden);
    let successor = chain.tip().expect("tip");
    assert_eq!(successor.note, "an ordinary byte-moving parcel", "the operator's note stands alone");
    assert!(
        provenance::check(golden, &chain).is_empty(),
        "the resulting chain must validate: {:?}",
        provenance::check(golden, &chain)
    );
}
