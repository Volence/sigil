//! provenance_chain — the §17 re-freeze discipline gate.
//!
//! Validates `golden/provenance.toml` against the committed golden blobs:
//!   (1) TIP-MATCH — every blob's recomputed full + header-neutral anchor CRC equals
//!       the chain tip (a stale golden or a stale chain entry fails here);
//!   (2) ANCHOR-MOVE-NEEDS-A/B — an anchor that moved between consecutive entries
//!       without a real `ab` evidence ref is a HARD failure;
//!   (3) AEON-REV-WELL-FORMED — an entry carrying an `aeon_rev` carries a full 40-char
//!       SHA, wherever it sits;
//!   (4) AEON-REV-MONOTONIC — once any entry names its aeon revision, no later entry may
//!       omit it. The boundary is derived from the chain, never pinned to an entry
//!       number, so a refreeze that lands before the field ships cannot retroactively
//!       become a violation.
//!
//! (1)–(4) need NO aeon tree and NO sigil build — they read only the committed blobs +
//! toml, so they run in the plain `cargo test -p sigil-harness` set (unlike the aeon-gated
//! native_full_rom / native_offcanonical_rom placement gates).
//!
//! `aeon_dir_matches_the_provenance_tip` is the exception: it resolves the reference
//! tree's git HEAD and compares it against the tip's `aeon_rev`. It reads no ROM, but its
//! ORACLE is a committed sigil artifact (`golden/provenance.toml`), which puts it in the
//! third shape `scripts/nightly_source_gates.sh` describes — inputs that would suit the
//! source lane, expectations that would not. The nightly lane points its reference at
//! aeon MASTER, which legitimately runs ahead of the frozen tip between refreezes, so
//! this gate would be red there by design. It is correctly classified artifact-lane by
//! that script's own audit (this file names `golden` throughout) and belongs to the
//! refreeze ritual's trigger, not the clock's.

use sigil_harness::provenance::{self, AppendGate};
use sigil_harness::test_support;
use std::path::{Path, PathBuf};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden")
}

/// `HEAD` of the git repository at `dir`, or an error naming why it could not be read.
fn head_sha(dir: &Path) -> Result<String, String> {
    if !dir.is_dir() {
        return Err(format!("{} is not a directory", dir.display()));
    }
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("spawn git in {}: {e}", dir.display()))?;
    if !out.status.success() {
        return Err(format!(
            "{} is not a git repository, or HEAD is unresolvable: {}",
            dir.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !provenance::is_full_sha(&sha) {
        return Err(format!("{}: HEAD resolved to `{sha}`, not a 40-char SHA", dir.display()));
    }
    Ok(sha)
}

#[test]
fn provenance_chain_holds() {
    let dir = golden_dir();
    let src = std::fs::read_to_string(dir.join("provenance.toml"))
        .expect("golden/provenance.toml must exist");
    let chain = provenance::parse(&src).expect("provenance.toml parses + root sentinel");
    let errs = provenance::check(&dir, &chain);
    assert!(
        errs.is_empty(),
        "provenance chain violations ({}):\n  {}",
        errs.len(),
        errs.join("\n  ")
    );
}

/// The tip must name all seven golden targets (the 7-ROM matrix — the six canonical/
/// config shapes plus `lean`, the crash-report-OFF profile added 2026-08-04) — a
/// truncated tip would silently drop a target from the discipline.
#[test]
fn tip_covers_all_seven_targets() {
    let dir = golden_dir();
    let src = std::fs::read_to_string(dir.join("provenance.toml")).unwrap();
    let chain = provenance::parse(&src).unwrap();
    let tip = chain.tip().unwrap();
    let mut keys: Vec<&str> = tip.targets.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["config_a", "config_b", "demo", "demo_debug", "lean", "s4", "s4_debug"],
        "tip must cover all seven golden targets"
    );
}

/// THE PAIRING GATE: the aeon tree a run points at must be the tree the goldens were
/// frozen from. Until `aeon_rev` existed this was not a queryable question — the SHA
/// lived only in `ab`/`note` prose, so every check of it was a human reading English, and
/// a parcel lost a run to exactly that (it matched four ROMs against CRCs quoted in a
/// note a later refreeze had invalidated, and concluded a wrong tree was the right one).
///
/// TWO MODES, and the split is deliberate — see the module note below on why the
/// mismatch case is not hard in both.
#[test]
fn aeon_dir_matches_the_provenance_tip() {
    let dir = golden_dir();
    let src = std::fs::read_to_string(dir.join("provenance.toml")).unwrap();
    let chain = provenance::parse(&src).unwrap();
    let tip = chain.tip().unwrap();
    let number = chain.entry.len();

    // RATCHET. `refreeze` appends nothing when nothing moved, so the tip cannot acquire
    // an `aeon_rev` until the next genuinely byte-moving parcel — no parcel can force
    // one. Failing closed here would put master red under the documented pre-merge bar
    // for an indefinite period, which retires the strict gate as a merge tool rather
    // than strengthening it. The teeth live in `check`'s AEON-REV-WELL-FORMED and
    // AEON-REV-MONOTONIC rules instead: both are hard TODAY, in every mode. So this
    // branch is a ratchet that disarms itself, permanently, at the next refreeze that
    // names a revision. Not `skip:` — that sentinel means "reference missing", and the
    // strict full-suite bar requires zero of those.
    //
    // The condition is the FIELD's absence, never an entry number: a pinned cutoff has a
    // merge race (see `check`), and reading the tip itself has none.
    let Some(tip_rev) = tip.aeon_rev.as_deref() else {
        eprintln!(
            "ratchet: provenance tip `{}` (entry #{number}) carries no aeon_rev, so the \
             AEON_DIR pairing cannot be checked. Entries predating the field are \
             deliberately not backfilled. This ratchet disarms at the next refreeze that \
             names a revision, which makes this assertion live permanently.",
            tip.name
        );
        return;
    };

    // A present field is guaranteed well-formed by AEON-REV-WELL-FORMED, which
    // `provenance_chain_holds` enforces; assert it rather than assume it, so this gate
    // cannot be the one that reads a malformed value as a pass.
    assert!(
        provenance::is_full_sha(tip_rev),
        "tip `{}` (entry #{number}) carries aeon_rev = \"{tip_rev}\", which is not a \
         full 40-char SHA",
        tip.name
    );

    let aeon = test_support::aeon_dir();
    let head = match head_sha(&aeon) {
        Ok(h) => h,
        Err(e) => {
            // A reference that cannot be read at all is the `reference_tree` case:
            // skip green, hard under strict, naming the path.
            assert!(!test_support::strict_gate(), "SIGIL_STRICT_GATE set but {e}");
            eprintln!("skip: cannot read aeon revision — {e} (set AEON_DIR)");
            return;
        }
    };

    if head == tip_rev {
        return;
    }

    // MISMATCH. Hard under strict, loud otherwise — and NOT hard in both modes, on
    // measurement: at the time this shipped aeon master was already two commits past
    // the frozen tip and both were documentation-only, moving zero bytes. An
    // unconditional assertion therefore goes red on a tree that is byte-correct in
    // every way, including the owner's default `cargo test` with AEON_DIR unset, which
    // resolves his live checkout. A gate that is red for a benign reason gets disabled.
    //
    // Strict is the right home for the hard bar: `SIGIL_STRICT_GATE=1` IS the
    // pre-merge/landing run, the landing lane already requires AEON_DIR to be a clean
    // checkout of a committed SHA, and the tip's revision is the only one whose ROMs
    // match these goldens. So the tolerance cannot survive a landing unnoticed.
    let msg = format!(
        "AEON_DIR pairing: {} is at aeon {head}, but the goldens were frozen from aeon \
         {} (provenance tip `{}`, entry #{number}). Byte comparisons made against this \
         tree are comparisons against a DIFFERENT revision than the record describes. \
         Point AEON_DIR at a clean checkout of {}.",
        aeon.display(),
        tip_rev,
        tip.name,
        tip_rev
    );
    assert!(!test_support::strict_gate(), "{msg}");
    eprintln!("notice: {msg}");
}


/// THE APPEND GATE, exercised against the REAL chain rather than a fixture.
///
/// `append_gate`'s rules are unit-tested exhaustively on synthetic chains; this is the
/// one place they meet `golden/provenance.toml` as it actually is, so a shape the real
/// file has and no fixture models cannot slip past. It also REPORTS the arming state on
/// every run, which is the whole point of a self-disarming ratchet: an operator should
/// never have to open the file to find out whether the rule is in force.
///
/// It asserts COHERENCE — that the verdict agrees with the tip's own records — rather
/// than demanding a particular verdict. `Refused` is a legitimate state for master to be
/// in: it is exactly the window between a freeze landing and its `--attest`, and failing
/// here would put master red for a reason that is not a defect. The teeth are in
/// `provenance_chain_holds` (which enforces rules 5 and 6 on every entry the chain has
/// been BUILT ON) and in `refreeze --freeze`, which refuses the append itself.
#[test]
fn the_append_gate_agrees_with_the_chains_own_records() {
    let dir = golden_dir();
    let src = std::fs::read_to_string(dir.join("provenance.toml")).unwrap();
    let chain = provenance::parse(&src).unwrap();
    let tip = chain.tip().unwrap();
    let n = chain.entry.len();

    match provenance::append_gate(&chain) {
        AppendGate::Ratchet(m) => {
            // Not `skip:` — the strict full-suite bar requires zero of those, and an
            // unarmed rule is not a missing reference.
            assert!(m.starts_with("ratchet:"), "an unarmed rule must say so as a ratchet: {m}");
            assert!(
                chain.entry.iter().all(|e| e.strict.is_none()),
                "the rule reported itself unarmed, but some entry carries a strict record"
            );
            eprintln!("{m}");
        }
        AppendGate::Allowed => assert!(
            tip.is_attested(),
            "the gate allowed an append, but tip `{}` (entry #{n}) records no passing run",
            tip.name
        ),
        AppendGate::NeedsSupersede(m) => {
            assert!(tip.is_red(), "a supersede was demanded without a red run on the tip: {m}");
            eprintln!("notice: {m}");
        }
        AppendGate::Refused(m) => {
            assert!(
                !tip.is_attested() && !tip.is_red(),
                "the gate refused an append, but tip `{}` (entry #{n}) does record a run",
                tip.name
            );
            // The normal window between a freeze landing and its `--attest`. Loud, not red.
            eprintln!("notice: {m}");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LEDGER INTEGRITY
//
// The ledger is the ONLY copy of the freeze history, and unlike a bad blob a bad
// ledger does not look bad: it looks authoritative and fails every later `--check`,
// `--attest` and byte gate with a line number for a cause. So the bar is not "this
// one input works" but:
//
//     after any refreeze run that reports an error, provenance.toml still parses.
//
// These gates hold that line at the append boundary every write goes through.
// ─────────────────────────────────────────────────────────────────────────────

/// A/B evidence prose carrying two literal quotes — the shape the field exists to hold,
/// and the shape a TOML basic string ends at.
const AB_WITH_QUOTES: &str = r#"("after ~512 frames", "~4,000px of unbroken descent")"#;

const A_SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn a_target_set() -> std::collections::BTreeMap<String, provenance::Target> {
    let mut m = std::collections::BTreeMap::new();
    m.insert(
        "s4".to_string(),
        provenance::Target {
            golden: "s4.bin".into(),
            full_crc: "deadbeef".into(),
            full_size: 0x40000,
            anchor_crc: "feedface".into(),
            anchor_end: 0x3f000,
        },
    );
    m
}

/// A tempdir holding a root-only ledger; returns the path and its exact text.
fn a_seeded_ledger(dir: &Path) -> (PathBuf, String) {
    let root = provenance::render_entry("root", provenance::ASL_WITNESS, A_SHA, "", &a_target_set());
    let path = dir.join("provenance.toml");
    std::fs::write(&path, &root).expect("seed ledger");
    (path, root)
}

/// THE INVARIANT. An append that reports an error leaves a ledger that still parses —
/// and, here, one that is byte-identical to what was there before.
#[test]
fn an_erroring_append_leaves_a_ledger_that_still_parses() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (path, before) = a_seeded_ledger(tmp.path());

    // Unterminated basic string: the exact syntactic shape a quote in `--ab` produced.
    let err = provenance::append_block(&path, &before, "\n[[entry]]\nname = \"broken\n")
        .expect_err("an unparseable block must not be installed");

    let after = std::fs::read_to_string(&path).expect("read ledger back");
    if let Err(e) = provenance::parse(&after) {
        panic!(
            "INVARIANT VIOLATED: the append reported `{err}` and left a provenance.toml \
             that no longer parses: {e}"
        );
    }
    assert_eq!(after, before, "a refused append must leave the ledger untouched");
}

/// The prose fields carry prose. A quote, a backslash and the characters a sentence is
/// made of survive the round trip through the file byte-for-byte.
#[test]
fn ledger_prose_survives_the_quotes_a_human_types() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (path, before) = a_seeded_ledger(tmp.path());

    let note = r#"a backslash \ and a quote " in one note"#;
    let block =
        provenance::render_entry("descent-parcel", AB_WITH_QUOTES, A_SHA, note, &a_target_set());
    let (_, chain) = provenance::append_block(&path, &before, &block)
        .unwrap_or_else(|e| panic!("prose with a quote broke the ledger: {e}"));

    let tip = chain.tip().expect("tip");
    assert_eq!(tip.ab, AB_WITH_QUOTES, "`ab` did not round-trip");
    assert_eq!(tip.note, note, "`note` did not round-trip");

    // And from the bytes actually on disk, not just the in-memory copy.
    let reread = provenance::parse(&std::fs::read_to_string(&path).expect("read back"))
        .expect("the installed ledger must parse");
    let tip = reread.tip().expect("tip");
    assert_eq!(tip.ab, AB_WITH_QUOTES);
    assert_eq!(tip.name, "descent-parcel");
}

/// The refusal half: what the ledger cannot show verbatim is named — the character and
/// where it is — and what it CAN show is not refused.
#[test]
fn unrepresentable_prose_is_refused_by_name() {
    let two_lines = "first\nsecond";
    let f = provenance::fault_in_prose("ab", two_lines)
        .expect("a raw newline in a one-line ledger field must be refused");
    assert!(f.contains("newline"), "the refusal must NAME the character, got: {f}");
    let at = two_lines.find('\n').expect("the newline");
    assert!(f.contains(&format!("byte {at}")), "the refusal must locate it, got: {f}");
    assert!(f.contains("ab"), "the refusal must name the field, got: {f}");

    // A quote is escapable, round-trips exactly, and reads correctly in the file, so
    // refusing it would cost the author their sentence for nothing.
    assert!(
        provenance::fault_in_prose("ab", AB_WITH_QUOTES).is_none(),
        "a quote is representable and must not be refused"
    );

    // And the whole-entry sweep reaches every field, not just `ab`.
    assert!(
        !provenance::entry_faults("n", "fine", A_SHA, "a\ttab", &a_target_set()).is_empty(),
        "entry_faults must reach `note`"
    );
    assert!(
        provenance::entry_faults("n", AB_WITH_QUOTES, A_SHA, "", &a_target_set()).is_empty(),
        "a faithful entry must not be refused"
    );
}

/// A target key becomes a TOML table HEADER, which has no escaping to fall back on.
#[test]
fn a_target_key_that_is_not_a_bare_key_is_refused() {
    assert!(provenance::fault_in_key("s4_debug").is_none());
    assert!(provenance::fault_in_key("config-a").is_none());
    for bad in ["", "s4.bin", "a b", "\"q\""] {
        assert!(
            provenance::fault_in_key(bad).is_some(),
            "`{bad}` is not a TOML bare key but was accepted"
        );
    }
}

/// A refusal that rejects the ledger's OWN history would be a new way to break the
/// freeze, not a fix. Every entry the chain already holds must pass the check that now
/// stands in front of every new one.
#[test]
fn the_refusal_accepts_every_entry_the_chain_already_holds() {
    let chain = provenance::load(&golden_dir()).expect("load provenance.toml");
    for (n, e) in chain.entry.iter().enumerate() {
        let faults = provenance::entry_faults(
            &e.name,
            &e.ab,
            e.aeon_rev.as_deref().unwrap_or(""),
            &e.note,
            &e.targets,
        );
        assert!(
            faults.is_empty(),
            "entry #{} `{}` is already in the ledger but the refusal rejects it: {faults:?}",
            n + 1,
            e.name
        );
    }
}

/// The install is a rename, so nothing is left behind for the next reader to trip over.
#[test]
fn the_ledger_install_leaves_no_temporary_behind() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (path, before) = a_seeded_ledger(tmp.path());
    let block = provenance::render_entry("second", "ref", A_SHA, "", &a_target_set());
    provenance::append_block(&path, &before, &block).expect("append");

    let left: Vec<String> = std::fs::read_dir(tmp.path())
        .expect("read tempdir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n != "provenance.toml")
        .collect();
    assert!(left.is_empty(), "the install left files behind: {left:?}");
}
