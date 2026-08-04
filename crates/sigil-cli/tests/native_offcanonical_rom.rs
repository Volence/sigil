//! Flip Stage 2 · the SIX-PROOF preamble — the OFF-CANONICAL native == golden gates.
//!
//! Completes the 6×2 matrix beyond canonical sonic4 (`native_rom` + `native_full_rom`):
//! demo (plain/debug), config_a, config_b. Each proves the ASSEMBLED ANCHOR
//! `[0, EndOfRom)` header-neutral == the frozen golden's prefix — the drift-stable
//! PRIMARY-class bar. The chained driver (`build_rom_chained` under `SizeSource::Frozen`)
//! computes every base from the committed listing table; the parallax `:=` capability
//! and the internal-bank-align recompute close the two pre-flip blockers.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_offcanonical_rom
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
fn golden(name: &str) -> Option<Vec<u8>> {
    // The frozen goldens live in the harness crate; the reproduced live artifact is
    // NOT read (Config-A/B clobber s4.bin/s4.debug.bin — the committed blob is truth).
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../sigil-harness/golden/{name}"));
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) => {
            if strict_gate() {
                panic!("golden missing: {}", path.display());
            }
            None
        }
    }
}
fn have_aeon() -> bool {
    let a = aeon_dir();
    if a.join("s4.bin").exists() {
        return true;
    }
    if strict_gate() {
        panic!("SIGIL_STRICT_GATE set but aeon tree missing at {}", a.display());
    }
    eprintln!("skip: aeon tree not present (set AEON_DIR)");
    false
}

// The chained build touches the shared engine/sound/generated dir — serialize.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn is_header_field(i: usize) -> bool {
    sigil_harness::CHECKSUM_FIELD_RANGE.contains(&i) || sigil_harness::ROM_END_FIELD_RANGE.contains(&i)
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/golden")
}

/// EndOfRom for a target, sourced from the provenance chain tip (`anchor_end`) — the
/// hand-typed `assembled_end` literal is retired. `provenance_chain` proves the tip's
/// anchor_end/anchor_crc against the committed blob; here the build's assembled length
/// is independently asserted == that anchor_end, cross-checking it against the real ROM.
fn anchor_end(key: &str) -> usize {
    sigil_harness::provenance::tip_target(&golden_dir(), key)
        .unwrap_or_else(|e| panic!("provenance tip: {e}"))
        .anchor_end
}

/// Prove `profile`'s assembled anchor `[0, eor)` == the golden prefix, header-neutral.
/// `key` selects the provenance target (its `anchor_end` is the window); the byte
/// compare over the prefix is the load-bearing assertion. Determinism: a second build is
/// byte-identical.
fn anchor_matches(profile: &native::GameProfile, golden_name: &str, key: &str) {
    if !have_aeon() {
        return;
    }
    let eor = anchor_end(key);
    let Some(g) = golden(golden_name) else { return };
    let _lk = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let rom = native::build_rom_chained(&aeon, profile).unwrap_or_else(|e| panic!("{}: {e}", profile.name));
    let rom2 = native::build_rom_chained(&aeon, profile).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(rom, rom2, "{}: build non-deterministic", profile.name);
    assert_eq!(rom.len(), eor, "{}: assembled length", profile.name);
    assert!(g.len() >= eor, "{}: golden shorter than anchor", profile.name);
    let bad: Vec<usize> = (0..eor).filter(|&i| !is_header_field(i) && rom[i] != g[i]).collect();
    assert!(
        bad.is_empty(),
        "{}: anchor diverges from golden at {} offset(s); first {:#x} (sig {:#04x} != gold {:#04x})",
        profile.name,
        bad.len(),
        bad.first().copied().unwrap_or(0),
        bad.first().map(|&i| rom[i]).unwrap_or(0),
        bad.first().map(|&i| g[i]).unwrap_or(0),
    );
}

// CRC/size/anchor_end now live in golden/provenance.toml (the chain tip); these gates
// read anchor_end from there via the `key` and prove the fresh build's anchor == the
// frozen golden's prefix.

#[test]
fn config_b_anchor_matches_golden() {
    anchor_matches(&native::config_b_profile(), "config_b.bin", "config_b");
}

#[test]
fn config_a_anchor_matches_golden() {
    anchor_matches(&native::config_a_profile(), "config_a.bin", "config_a");
}

#[test]
fn demo_plain_anchor_matches_golden() {
    anchor_matches(&native::demo_profile(false), "demo.bin", "demo");
}

#[test]
fn demo_debug_anchor_matches_golden() {
    anchor_matches(&native::demo_profile(true), "demo.debug.bin", "demo_debug");
}

/// LEAN (crash-report OFF) — the 7th target. Its whole content delta from the release
/// shape is the fault-handler swap (the 0x10B0 error_handler island out, the 0x2E
/// `ReleaseFault` in, and the 60 vector cells repointed), so this anchor comparison is
/// what proves `CRASH_REPORT=0` is a real, complete shape rather than a partial strip.
#[test]
fn lean_anchor_matches_golden() {
    anchor_matches(&native::lean_profile(), "lean.bin", "lean");
}
