//! Flip Stage 2 · the OFF-CANONICAL FULL-FILE gates (the S1.4 split-golden pattern,
//! productionized for demo/config_a/config_b).
//!
//! For each off-canonical target the full native file = the chained assembled image
//! (checksum-folded by `emit_rom`) + the SIGIL-CANONICAL deb2 appendix (the same
//! Option-A `build.sh:169-175` post-pipeline as sonic4, over the Frozen placement).
//! The bar SPLITS exactly as sonic4's (`native_full_rom.rs`):
//!   - ASSEMBLED ANCHOR `[0, EndOfRom)` == the asl golden prefix (header-neutral) —
//!     proven by `native_offcanonical_rom.rs`.
//!   - FULL FILE == the sigil-canonical golden (CRC/size/anchor sourced from the
//!     provenance chain tip in `golden/provenance.toml`).
//!   - FUNCTIONAL TRUTH: determinism, deb2 presence + size band (inside
//!     `append_deb2_appendix`), a convsym load-bearing spot-check, and a per-target
//!     t24 doctored-address negative control.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_offcanonical_full
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

// convsym + the chained build touch shared temp/gen state — serialize the targets.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The golden directory (holds the frozen blobs + `provenance.toml`, the single source
/// of the expected CRC/size/anchor — the hand-edited const surface is retired).
fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/golden")
}

/// EndOfRom anchor + sigil-canonical full-file (crc, len), sourced from the provenance
/// chain TIP (`golden/provenance.toml`) — these move on every golden re-freeze, and
/// `provenance_chain` independently proves the tip equals the committed blobs.
fn expected(key: &str) -> (usize, u32, usize) {
    let t = sigil_harness::provenance::tip_target(&golden_dir(), key)
        .unwrap_or_else(|e| panic!("provenance tip: {e}"));
    let crc = sigil_harness::provenance::hex_u32(&t.full_crc).unwrap_or_else(|e| panic!("{e}"));
    (t.anchor_end, crc, t.full_size)
}

/// A target's full-file proof: profile plus a load-bearing (name, addr) spot set.
/// `name` doubles as the provenance-chain target key.
struct Target {
    name: &'static str,
    profile: native::GameProfile,
    load_bearing: &'static [(&'static str, u32)],
}

fn run(t: &Target) {
    if !have_aeon() {
        return;
    }
    let _lk = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let (eor, want_crc, want_len) = expected(t.name);

    // FULL FILE (presence control + size band live inside build_full_file_chained).
    let full = native::build_full_file_chained(&aeon, &t.profile).unwrap_or_else(|e| panic!("{}: {e}", t.name));

    // (a) DETERMINISM — a second build is byte-identical.
    let full2 = native::build_full_file_chained(&aeon, &t.profile).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(full, full2, "{}: full file non-deterministic", t.name);

    // (b) PRESENCE — deb2 magic at EndOfRom, non-trivial appendix.
    assert_eq!(&full[eor..eor + 2], &native::DEB2_MAGIC, "{}: deb2 magic at EndOfRom", t.name);
    let appendix = full.len() - eor;
    assert!(appendix >= 0x1000, "{}: appendix {appendix:#x} too small", t.name);

    // (c) FUNCTIONAL RESOLVE — load-bearing symbols → exact addresses via the REAL
    // convsym consumer over the chained listing.
    let (_rom, listing) =
        native::build_rom_chained_with_listing(&aeon, &t.profile).unwrap_or_else(|e| panic!("{e}"));
    let resolved = native::convsym_resolve(&aeon, &listing).unwrap_or_else(|e| panic!("{e}"));
    for (name, want) in t.load_bearing {
        let got = resolved
            .get(*name)
            .unwrap_or_else(|| panic!("{}: load-bearing `{name}` absent from convsym output", t.name));
        assert_eq!(got, want, "{}: `{name}` resolved to {got:#X}, expected {want:#X}", t.name);
    }

    // FULL-FILE golden (sigil-canonical, CRC-pinned via the provenance tip).
    let crc = native::crc32(&full);
    eprintln!(
        "S2 {}: assembled={eor:#x} full={} appendix={appendix:#x} syms={} crc={crc:08x}",
        t.name, full.len(), listing.len()
    );
    assert_eq!(full.len(), want_len, "{}: full-file size (re-freeze the golden?)", t.name);
    assert_eq!(crc, want_crc, "{}: full-file CRC (re-freeze the golden?)", t.name);
}

/// t24 per-target negative control: doctoring a load-bearing symbol's address in the
/// chained listing MUST change what the REAL convsym consumer resolves it to — proving
/// the spot-check reads the actual packed table, not a cached value.
fn doctored_control(t: &Target) {
    if !have_aeon() {
        return;
    }
    let _lk = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let (_rom, mut listing) =
        native::build_rom_chained_with_listing(&aeon, &t.profile).unwrap_or_else(|e| panic!("{e}"));
    let (probe, real) = t.load_bearing[0];
    let base = native::convsym_resolve(&aeon, &listing).unwrap();
    assert_eq!(base.get(probe), Some(&real), "{}: control undoctored `{probe}`", t.name);
    for s in listing.iter_mut() {
        if s.name == probe {
            s.value = 0x00BEEF;
        }
    }
    let doctored = native::convsym_resolve(&aeon, &listing).unwrap();
    assert_eq!(
        doctored.get(probe),
        Some(&0x00BEEF),
        "{}: t24 — convsym must reflect the doctored `{probe}` (else the spot-check is vacuous)",
        t.name
    );
}

fn config_a() -> Target {
    Target {
        name: "config_a",
        profile: native::config_a_profile(),
        load_bearing: &[
            ("EntryPoint", 0x200),
            ("GameLoop", 0x246e),        // -0x10 pal-ntsc-only boot shrink
            ("BusError", 0x5e542),
            ("HeightMaps", 0x257d6),
            // L1 P2 re-layout: AnimateSprite slid +8 (0x3534 -> 0x353C) with the
            // config_a re-freeze (boot's inline sound-test body became a jsr to the
            // game-side SoundTest_BootPing). The whole-anchor byte compare
            // (native_offcanonical_rom::config_a_anchor_matches_golden) proves the
            // new golden; this spot-check tracks the moved label.
            ("AnimateSprite", 0x356C),   // -0x10 pal-ntsc-only
            ("EndOfRom", 0x5f5f2),
        ],
    }
}
fn config_b() -> Target {
    Target {
        name: "config_b",
        profile: native::config_b_profile(),
        load_bearing: &[
            ("EntryPoint", 0x200),
            ("GameLoop", 0xb80),         // -0x10 pal-ntsc-only boot shrink
            ("BusError", 0x423d0),
            ("HeightMaps", 0x25730),
            ("AnimateSprite", 0x171E),   // -0x10 pal-ntsc-only
            ("EndOfRom", 0x43480),
        ],
    }
}
fn demo_plain() -> Target {
    Target {
        name: "demo",
        profile: native::demo_profile(false),
        load_bearing: &[
            ("EntryPoint", 0x200),
            ("GameLoop", 0xb80),         // -0x10 pal-ntsc-only boot shrink
            ("BusError", 0x10174),
            ("AnimateSprite", 0x171E),   // -0x10 pal-ntsc-only
            ("EndOfRom", 0x11224),
        ],
    }
}
fn demo_debug() -> Target {
    Target {
        name: "demo_debug",
        profile: native::demo_profile(true),
        load_bearing: &[
            ("EntryPoint", 0x200),
            ("GameLoop", 0xb90),         // -0x10 pal-ntsc-only boot shrink
            ("BusError", 0x10174),
            ("AnimateSprite", 0x1C7E),   // -0x10 pal-ntsc-only
            ("EndOfRom", 0x11224),
        ],
    }
}

#[test]
fn config_a_full_file() {
    run(&config_a());
}
#[test]
fn config_b_full_file() {
    run(&config_b());
}
#[test]
fn demo_plain_full_file() {
    run(&demo_plain());
}
#[test]
fn demo_debug_full_file() {
    run(&demo_debug());
}

#[test]
fn config_a_doctored_control() {
    doctored_control(&config_a());
}
#[test]
fn config_b_doctored_control() {
    doctored_control(&config_b());
}
#[test]
fn demo_doctored_control() {
    doctored_control(&demo_plain());
}
