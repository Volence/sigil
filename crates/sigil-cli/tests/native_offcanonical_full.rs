//! Flip Stage 2 · the OFF-CANONICAL FULL-FILE gates (the S1.4 split-golden pattern,
//! productionized for demo/config_a/config_b/lean).
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

/// A target's full-file proof: profile plus a load-bearing label spot set.
/// `name` doubles as the provenance-chain target key AND the frozen size-table
/// stem (`golden/offcanonical_sizes/<name>.txt`). Expected addresses are read
/// from that table — it regenerates at every refreeze, so the old hand-typed
/// literals (which rode two waves in two parcels) cannot rot again
/// (input-6button, the t24 rule).
struct Target {
    name: &'static str,
    profile: native::GameProfile,
    load_bearing: &'static [&'static str],
}

/// Parse the target's frozen size table into name -> address.
fn size_table(name: &str) -> std::collections::HashMap<String, u32> {
    let stem = if name == "demo" { "demo" } else { name };
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../sigil-harness/golden/offcanonical_sizes/{stem}.txt"));
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read size table {}: {e}", path.display()));
    src.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let n = it.next()?;
            let a = it.next()?;
            Some((n.to_string(), u32::from_str_radix(a.trim_start_matches("0x"), 16).ok()?))
        })
        .collect()
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

    // (b) PRESENCE / ABSENCE — split on the CRASH-REPORT axis, not on `debug`
    // (owner-ruled 2026-08-04). Any profile carrying the MD Debugger island ships its
    // deb2 symbol table: that is debug AND release (config_a, config_b, demo plain,
    // demo debug). The `lean` profile is the one that ships the assembled image ALONE.
    // Both directions are asserted, so either failure mode is caught by name: a symbol
    // table silently DROPPED from a shipped release (the crash screen degrades to
    // `<unknown>` on a player's cartridge), or one leaking into lean, whose entire
    // point is not to have it.
    let appendix = full.len() - eor;
    if t.profile.debug || t.profile.crash_report {
        assert_eq!(&full[eor..eor + 2], &native::DEB2_MAGIC, "{}: deb2 magic at EndOfRom", t.name);
        assert!(appendix >= 0x1000, "{}: appendix {appendix:#x} too small", t.name);
    } else {
        assert_eq!(
            appendix, 0,
            "{}: a CRASH_REPORT=0 profile must ship NOTHING past EndOfRom, found \
             {appendix:#x} bytes (has the deb2 symbol appendix leaked into lean?)",
            t.name
        );
    }

    // (c) FUNCTIONAL RESOLVE — load-bearing symbols → exact addresses via the REAL
    // convsym consumer over the chained listing.
    let native::RomBuild { listing, .. } =
        native::build_rom_chained_with_listing(&aeon, &t.profile).unwrap_or_else(|e| panic!("{e}"));
    let resolved = native::convsym_resolve(&aeon, &listing).unwrap_or_else(|e| panic!("{e}"));
    let table = size_table(t.name);
    for name in t.load_bearing {
        let want = table
            .get(*name)
            .unwrap_or_else(|| panic!("{}: `{name}` absent from the frozen size table", t.name));
        let got = resolved
            .get(*name)
            .unwrap_or_else(|| panic!("{}: load-bearing `{name}` absent from convsym output", t.name));
        assert_eq!(got, want, "{}: `{name}` resolved to {got:#X}, size table says {want:#X}", t.name);
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
    let native::RomBuild { mut listing, .. } =
        native::build_rom_chained_with_listing(&aeon, &t.profile).unwrap_or_else(|e| panic!("{e}"));
    let probe = t.load_bearing[0];
    let real = *size_table(t.name).get(probe).expect("probe in size table");
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
            "EntryPoint",
            "GameLoop",
            "BusError",
            "HeightMaps",
            "AnimateSprite",
            "EndOfRom",
        ],
    }
}
fn config_b() -> Target {
    Target {
        name: "config_b",
        profile: native::config_b_profile(),
        load_bearing: &[
            "EntryPoint",
            "GameLoop",
            "BusError", // release carries the error_handler island (crash-report ruling)
            "HeightMaps",
            "AnimateSprite",
            "EndOfRom",
        ],
    }
}
fn demo_plain() -> Target {
    Target {
        name: "demo",
        profile: native::demo_profile(false),
        load_bearing: &[
            "EntryPoint",
            "GameLoop",
            "BusError", // demo's release shape carries the debugger too (owner-ruled: no exclusion)
            "AnimateSprite",
            "EndOfRom",
        ],
    }
}
fn demo_debug() -> Target {
    Target {
        name: "demo_debug",
        profile: native::demo_profile(true),
        load_bearing: &[
            "EntryPoint",
            "GameLoop",
            "BusError",
            "AnimateSprite",
            "EndOfRom",
        ],
    }
}

/// LEAN — the 7th target and the ONLY one whose full file is the assembled image
/// alone: CRASH_REPORT=0 means no error_handler island and no deb2 appendix, and every
/// fault vector routes at `ReleaseFault`. So its load-bearing set probes `ReleaseFault`
/// where every other target probes `BusError` — that pair of spot-checks is what proves
/// the axis actually swaps the fault handler rather than just eliding bytes.
fn lean() -> Target {
    Target {
        name: "lean",
        profile: native::lean_profile(),
        load_bearing: &[
            "EntryPoint",
            "GameLoop",
            "ReleaseFault", // the lean fault handler — absent from every other target
            "HeightMaps",
            "AnimateSprite",
            "EndOfRom",
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
fn lean_full_file() {
    run(&lean());
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
#[test]
fn lean_doctored_control() {
    doctored_control(&lean());
}
