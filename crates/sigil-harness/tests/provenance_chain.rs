//! provenance_chain — the §17 re-freeze discipline gate.
//!
//! Validates `golden/provenance.toml` against the committed golden blobs:
//!   (1) TIP-MATCH — every blob's recomputed full + header-neutral anchor CRC equals
//!       the chain tip (a stale golden or a stale chain entry fails here);
//!   (2) ANCHOR-MOVE-NEEDS-A/B — an anchor that moved between consecutive entries
//!       without a real `ab` evidence ref is a HARD failure;
//!   (3) AEON-REV-PRESENT — every entry written since the `aeon_rev` field shipped names
//!       the aeon revision its bytes were built from, as a full 40-char SHA.
//!
//! (1)–(3) need NO aeon tree and NO sigil build — they read only the committed blobs +
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

use sigil_harness::provenance;
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
    // than strengthening it. The teeth live in `check`'s AEON-REV-PRESENT rule instead:
    // that one is hard TODAY, in every mode, and makes a field-less entry #167
    // impossible. So this branch is a ratchet that disarms itself, permanently, at the
    // next refreeze. Not `skip:` — that sentinel means "reference missing", and the
    // strict full-suite bar requires zero of those.
    if number <= provenance::AEON_REV_FROM_ENTRY {
        eprintln!(
            "ratchet: provenance tip `{}` is entry #{number}, at or below the \
             AEON_REV_FROM_ENTRY={} cutoff, so it carries no aeon_rev and the AEON_DIR \
             pairing cannot be checked. Historical entries are deliberately not \
             backfilled. This ratchet disarms at the next byte-moving refreeze, which \
             writes the field and makes this assertion live.",
            tip.name,
            provenance::AEON_REV_FROM_ENTRY
        );
        return;
    }

    // Past the cutoff the field is guaranteed well-formed by AEON-REV-PRESENT, which
    // `provenance_chain_holds` enforces; assert it rather than assume it, so this gate
    // cannot be the one that reads a malformed value as a pass.
    assert!(
        provenance::is_full_sha(&tip.aeon_rev),
        "tip `{}` (entry #{number}) is past the AEON_REV_FROM_ENTRY cutoff but carries \
         aeon_rev = \"{}\", which is not a full 40-char SHA",
        tip.name,
        tip.aeon_rev
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

    if head == tip.aeon_rev {
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
        tip.aeon_rev,
        tip.name,
        tip.aeon_rev
    );
    assert!(!test_support::strict_gate(), "{msg}");
    eprintln!("notice: {msg}");
}
