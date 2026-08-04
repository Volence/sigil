//! provenance_chain — the §17 re-freeze discipline gate.
//!
//! Validates `golden/provenance.toml` against the committed golden blobs:
//!   (1) TIP-MATCH — every blob's recomputed full + header-neutral anchor CRC equals
//!       the chain tip (a stale golden or a stale chain entry fails here);
//!   (2) ANCHOR-MOVE-NEEDS-A/B — an anchor that moved between consecutive entries
//!       without a real `ab` evidence ref is a HARD failure.
//!
//! Needs NO aeon tree and NO sigil build — it reads only the committed blobs + toml, so
//! it runs in the plain `cargo test -p sigil-harness` set (unlike the aeon-gated
//! native_full_rom / native_offcanonical_rom placement gates).

use sigil_harness::provenance;
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden")
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
