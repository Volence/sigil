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

/// Prove `profile`'s assembled anchor `[0, eor)` == the golden prefix, header-neutral.
/// `golden_crc` is the FULL-file golden CRC (informational — the byte compare over the
/// prefix is the load-bearing assertion). Determinism: a second build is byte-identical.
fn anchor_matches(profile: &native::GameProfile, golden_name: &str, eor: usize) {
    if !have_aeon() {
        return;
    }
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

#[test]
fn config_b_anchor_matches_golden() {
    // golden config_b.bin 92776720/304961; assembled_end 0x434d0.
    anchor_matches(&native::config_b_profile(), "config_b.bin", 0x434d0);
}

#[test]
fn config_a_anchor_matches_golden() {
    // golden config_a.bin b4a6756d/421898; assembled_end 0x5f65a.
    anchor_matches(&native::config_a_profile(), "config_a.bin", 0x5f65a);
}

#[test]
fn demo_plain_anchor_matches_golden() {
    // demo.bin 18c64002/90776; assembled_end 0x11224.
    anchor_matches(&native::demo_profile(false), "demo.bin", 0x11224);
}

#[test]
fn demo_debug_anchor_matches_golden() {
    // demo.debug.bin b0475a59/91584; assembled_end 0x11224 (demo_debug.txt).
    anchor_matches(&native::demo_profile(true), "demo.debug.bin", 0x11224);
}
