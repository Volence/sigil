//! Flip Stage 1 · S1.4 — THE SPLIT-GOLDEN FULL-FILE GATES (Option A).
//!
//! The full native ROM = the assembled image (checksum-folded by `emit_rom`) + the
//! SIGIL-CANONICAL deb2 symbol appendix, produced by driving the REAL `tools/convsym`
//! + `tools/fixheader` over sigil's OWN listing (`build.sh:169-175`, fed sigil's
//! `.lst` instead of asl's). Under Volence/overseer Option A the appendix is NOT a
//! byte-imitation of asl's name set — the `.emp` names are the source names going
//! forward. So the bar SPLITS (2026-07-30-flip-stage1-S1.4-appendix-fork.md):
//!
//!   - ASSEMBLED PREFIX `[0, EndOfRom)` == asl (header-neutral) — the correctness
//!     anchor, asl-witnessed, unchanged. Proven by `native_rom_{plain,debug}`.
//!   - FULL FILE == the sigil-canonical golden (deterministic; CRC-pinned here,
//!     blob-frozen in the golden-freeze stage).
//!   - FUNCTIONAL TRUTH (condition 2): determinism, assert-PRESENCE (`de b2` + size
//!     band), convsym rc 0, and a load-bearing spot-check resolving to exact known
//!     addresses through the REAL convsym consumer path — plus a t24 doctored-symbol
//!     negative control.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_full_rom
//! ```

use sigil_harness::{native, pins};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}
fn read_ref(name: &str) -> Option<Vec<u8>> {
    let path = aeon_dir().join(name);
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but reference missing: {}", path.display());
            }
            eprintln!("skip: reference ROM not at {} (set AEON_DIR)", path.display());
            None
        }
    }
}

// The native full-file build touches the shared `engine/sound/generated` dir and
// spawns convsym over temp files — serialize the shapes.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The load-bearing spot-check set (condition 2c): one Game_Entry-class, one
/// ErrorHandler-class, a player proc, a level proc, a Z80-adjacent, plus engine
/// keystones — each MUST resolve to its exact per-shape address through the REAL
/// `convsym -output log` consumer. Addresses are listing-truth (pins.rs / s4.lst).
/// `(name, plain, debug)`.
const LOAD_BEARING: &[(&str, u32, u32)] = &[
    ("EntryPoint", 0x200, 0x200),        // Game_Entry-class
    ("GameLoop", 0x239A, 0x2428),        // the main loop
    ("BusError", 0x5CA50, 0x5E542),      // ErrorHandler-class (error-handler region; −0x60/−0x68 wave-c ojz shrink)
    ("Ground_Move_Cap", 0x10724, 0x10730), // a player proc (object bank — anchored, stable)
    ("Section_Init", 0x560C, 0x63AC),    // a level proc (before parallax — stable)
    ("BG_Init", 0x61CE, 0x6FC0),         // a level proc (after PARALLAX + SECTION, so it rides their growth)
    ("AnimateSprite", 0x2F3C, 0x351A),   // an objects keystone
    ("TouchResponse", 0x30C6, 0x37C2),   // a collision keystone
    ("Z80_Sound_Start", 0x3DE, 0x3E2),   // Z80-adjacent (shape-varying)
];

/// The golden directory (holds the frozen blobs + `provenance.toml`, the single source
/// of the expected CRC/size — the hand-edited const surface is retired).
fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/golden")
}

/// The sigil-canonical full-file CRC-32 + size, sourced from the provenance chain TIP
/// (`golden/provenance.toml`). The ASSEMBLED prefix is asl-identical (the PRIMARY
/// anchor); the FULL-FILE values are sigil-canonical (the appendix is sigil's) and move
/// on any golden re-freeze — a mismatch is the intended "re-freeze the golden" signal,
/// per the split-golden model. `provenance_chain` independently proves the tip equals
/// the committed blobs, so build == tip here means build == the frozen golden.
fn expected_full(key: &str) -> (u32, usize) {
    let t = sigil_harness::provenance::tip_target(&golden_dir(), key)
        .unwrap_or_else(|e| panic!("provenance tip: {e}"));
    let crc = sigil_harness::provenance::hex_u32(&t.full_crc).unwrap_or_else(|e| panic!("{e}"));
    (crc, t.full_size)
}

fn run_shape(debug: bool, refname: &str, key: &str) {
    let expect = expected_full(key);
    let Some(refrom) = read_ref(refname) else { return };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let shape = if debug { "debug" } else { "plain" };

    // The assembled prefix stays the asl-witnessed correctness anchor.
    let (assembled, listing) =
        native::build_native_rom_with_listing(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let eor = if debug { pins::DEBUG_ASSEMBLED_LEN } else { pins::ASSEMBLED_LEN };
    assert_eq!(assembled.len(), eor, "{shape}: assembled length");

    // FULL FILE: assembled + sigil-canonical deb2 appendix (presence control lives
    // inside build_native_full_file — a silent convsym failure is a HARD error).
    let full = native::build_native_full_file(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));

    // (a) DETERMINISM — a second build is byte-identical.
    let full2 = native::build_native_full_file(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(full, full2, "{shape}: full file non-deterministic");

    // (b) PRESENCE — `de b2` magic at EndOfRom, non-trivial appendix.
    assert_eq!(&full[eor..eor + 2], &native::DEB2_MAGIC, "{shape}: deb2 magic at EndOfRom");
    let appendix = full.len() - eor;
    assert!(appendix >= 0x2000, "{shape}: appendix {appendix:#x} too small");

    // ASSEMBLED PREFIX == asl (header-neutral): the full file's `[0, EndOfRom)`
    // matches the asl reference modulo the checksum ($18E) and ROM-end ($1A4) fields
    // — the sigil-canonical appendix has a different size than asl's, so those two
    // header fields legitimately differ, but every other assembled byte is identical.
    let asl_prefix = &refrom[..eor];
    let sig_prefix = &full[..eor];
    let is_header_field = |i: usize| {
        sigil_harness::CHECKSUM_FIELD_RANGE.contains(&i) || sigil_harness::ROM_END_FIELD_RANGE.contains(&i)
    };
    let bad: Vec<usize> = (0..eor)
        .filter(|&i| sig_prefix[i] != asl_prefix[i] && !is_header_field(i))
        .collect();
    assert!(
        bad.is_empty(),
        "{shape}: assembled prefix diverges from asl at {} offset(s); first {:#x} (sig {:#04x} != asl {:#04x})",
        bad.len(),
        bad.first().copied().unwrap_or(0),
        bad.first().map(|&i| sig_prefix[i]).unwrap_or(0),
        bad.first().map(|&i| asl_prefix[i]).unwrap_or(0),
    );

    // (c) FUNCTIONAL RESOLVE — load-bearing symbols → exact addresses via convsym.
    let resolved = native::convsym_resolve(&aeon, &listing).unwrap_or_else(|e| panic!("{e}"));
    for (name, plain, dbg) in LOAD_BEARING {
        let want = if debug { *dbg } else { *plain };
        let got = resolved
            .get(*name)
            .unwrap_or_else(|| panic!("{shape}: load-bearing symbol `{name}` absent from convsym output"));
        assert_eq!(*got, want, "{shape}: `{name}` resolved to {got:#X}, expected {want:#X}");
    }

    // FULL-FILE golden (sigil-canonical, CRC-pinned).
    let crc = native::crc32(&full);
    let (want_crc, want_len) = expect;
    eprintln!(
        "S1.4 {shape}: assembled={eor:#x} full={} appendix={appendix:#x} syms={} crc={crc:08x}",
        full.len(),
        listing.len()
    );
    assert_eq!(full.len(), want_len, "{shape}: full-file size (re-freeze the golden?)");
    assert_eq!(crc, want_crc, "{shape}: full-file CRC (re-freeze the golden?)");
}

/// (PLAIN) the split-golden full-file gate.
#[test]
fn native_full_sonic4_plain() {
    run_shape(false, "s4.bin", "s4");
}

/// (DEBUG) the split-golden full-file gate.
#[test]
fn native_full_sonic4_debug() {
    run_shape(true, "s4.debug.bin", "s4_debug");
}

/// t24 NEGATIVE CONTROL: doctoring one symbol's address in the listing MUST change
/// what the real convsym consumer resolves it to — proving the spot-check reads the
/// actual packed table, not a cached/hardcoded value. AND an empty listing MUST be
/// rejected (the assert-PRESENCE size band catches a collapsed/absent appendix).
#[test]
fn deb2_appendix_negative_controls() {
    if read_ref("s4.bin").is_none() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let (_rom, mut listing) =
        native::build_native_rom_with_listing(&aeon, false).unwrap_or_else(|e| panic!("{e}"));

    // Undoctored: BusError resolves to its known address.
    let base = native::convsym_resolve(&aeon, &listing).unwrap();
    assert_eq!(base.get("BusError"), Some(&0x5CA50), "control: undoctored BusError");

    // DOCTOR: move BusError to a bogus in-range address.
    for s in listing.iter_mut() {
        if s.name == "BusError" {
            s.value = 0x00BEEF;
        }
    }
    let doctored = native::convsym_resolve(&aeon, &listing).unwrap();
    assert_eq!(
        doctored.get("BusError"),
        Some(&0x00BEEF),
        "t24: convsym must reflect the doctored address (else the spot-check is vacuous)"
    );

    // A COLLAPSED listing (a handful of symbols) yields a sub-band appendix (~0x27 B)
    // → the size-band presence control HARD-errors, so a silently-appendix-less or
    // truncated ROM can never pass. Exercises the band directly.
    let dummy = vec![0u8; pins::ASSEMBLED_LEN];
    let tiny: Vec<sigil_link::ListingSymbol> = (0..3)
        .map(|i| sigil_link::ListingSymbol {
            name: format!("Sym{i}"),
            value: 0x1000 + i,
            is_equate: false,
            unused: false,
        })
        .collect();
    let err = native::append_deb2_appendix(&aeon, &dummy, &tiny, false, native::SONIC4_APPENDIX_FLOOR)
        .expect_err("collapsed listing must be rejected by the size-band presence control");
    assert!(
        err.contains("appendix size") && err.contains("band"),
        "t24: collapsed-listing rejection should name the size band; got: {err}"
    );

    // An EMPTY listing is ALSO rejected — convsym itself refuses (`No symbols passed`),
    // so the silent-failure path (`2>/dev/null || true` producing an appendix-less
    // ROM) surfaces as a HARD error here, never a pass.
    let empty: Vec<sigil_link::ListingSymbol> = Vec::new();
    native::append_deb2_appendix(&aeon, &dummy, &empty, false, native::SONIC4_APPENDIX_FLOOR)
        .expect_err("empty listing must be rejected (convsym aborts on zero symbols)");
}
