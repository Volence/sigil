//! Flip Stage 2 · S1.2 — THE FROZEN-CHAINER PLACEMENT GATE (off-canonical).
//!
//! The GameProfile + frozen-table chainer (`native::build_rom_chained` under
//! `SizeSource::Frozen`) computes every off-canonical section's ROM base from the
//! committed listing table (`golden/offcanonical_sizes/*.txt`) instead of the baked
//! sonic4 resume orgs. This gate proves the PLACEMENT half of that mechanism on
//! Config-B (sonic4, sound-off): the chainer builds end-to-end (no overlap, no drift
//! guard fires) AND every frozen-labeled section resolves to its exact frozen address —
//! `frozen_placement_mismatches` returns EMPTY.
//!
//! Why placement, not byte-identity: Config-B's full byte-identity is BLOCKED pre-flip
//! by assembly-time-FOLDED sonic4 constants (`Game_Entry = $5C7EC`,
//! `ErrorHandler: equ $5CC0A` — the brief's rows 52/90), which are numeric constants the
//! chainer cannot re-place; they only "resolve natively" at the flip. See
//! `docs/superpowers/notes/2026-07-30-flip-stage2-drivers-blocker-checkpoint.md`. This
//! gate isolates the chainer's correctness (every DECLARED section placed exactly right)
//! from that independent fold blocker, and keeps the Frozen machinery under test.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_offcanonical_placement
//! ```
use sigil_harness::native;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}
fn have_aeon(aeon: &PathBuf) -> bool {
    if aeon.join("s4.bin").exists() {
        return true;
    }
    if strict_gate() {
        panic!("SIGIL_STRICT_GATE set but aeon tree missing at {}", aeon.display());
    }
    eprintln!("skip: aeon tree not present (set AEON_DIR)");
    false
}

// The frozen build touches the shared engine/sound/generated dir.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Config-B: the frozen chainer places EVERY frozen-labeled section byte-correctly
/// (empty mismatch set). Proves the declared-order computed-base mechanism from the
/// committed `config_b.txt` — TestPlayer/OJZ/error-handler/EndOfRom all land at their
/// config_b addresses, not sonic4's.
#[test]
fn config_b_frozen_placement_exact() {
    let aeon = aeon_dir();
    if !have_aeon(&aeon) {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let profile = native::config_b_profile();
    let mismatches =
        native::frozen_placement_mismatches(&aeon, &profile).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        mismatches.is_empty(),
        "config_b frozen chainer misplaced {} declared section(s); first: {:?}",
        mismatches.len(),
        mismatches.first()
    );
}
