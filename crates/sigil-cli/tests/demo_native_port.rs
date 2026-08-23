//! Parcel H-demo — the demo game modules' native-placement byte gate.
//!
//! The demo is the "start here" template game. Its three game modules —
//! `demo_box` (the static object proc), `demo_data` (objdef + `offsets` mapping +
//! spawn list + art + palette + BG-anim header), `demo_state` (the two GameState
//! procs) — were the FIRST demo ROM-data/code to place natively (the demo profile
//! had zero native game modules before this parcel). This gate proves each module
//! emits the EXACT golden bytes, at its exact ROM address, through the real demo
//! frozen-chain build.
//!
//! The demo game island is shape-invariant (no DEBUG blocks), so plain and debug
//! carry identical bytes in this window:
//!
//! - `demo_box`   `$10002..$10006` (DemoBox_Main — `jbra Draw_Sprite`, 4 bytes)
//! - `demo_data`  `$10006..$10100` (ObjDef_DemoBox .. BgAnim_Table, 0xFA bytes)
//! - `demo_state` `$10100..$10172` (GameState_Demo_Init .. GameState_Demo, 0x72)
//!
//! The whole demo game island is `[0x10002, 0x10172)` — byte-compared against the
//! committed golden ROM's assembled region (well inside the header-neutral
//! assembled anchor `[0, EndOfRom)` the offcanonical gates prove).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test demo_native_port
//! ```
use sigil_harness::native;
use sigil_harness::test_support::reference_tree_for_profile;
use std::path::PathBuf;

/// The three demo game modules, each `(name, base, len)`. Demo-shape addresses
/// (shape-invariant, plain == debug); the demo modules carry no `pins` constant
/// (repin resolves only the sonic4 shape), so the window is named here. Together
/// they tile the contiguous island `[0x10002, 0x10172)` in the object-code bank.
const WINDOWS: &[(&str, usize, usize)] = &[
    ("demo_box", 0x10002, 0x4),    // DemoBox_Main
    ("demo_data", 0x10006, 0xFA),  // ObjDef_DemoBox .. BgAnim_Table
    ("demo_state", 0x10100, 0x72), // GameState_Demo_Init .. GameState_Demo
];

/// The committed golden ROMs live in sigil's OWN `golden/` directory, so the aeon
/// tree this gate needs is source-only: it builds the demo shape from `.emp` and
/// compares here.
fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/golden")
}

// The chained build touches shared temp/gen state — serialize plain vs debug.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gate(debug: bool, golden_name: &str) {
    let profile = native::demo_profile(debug);
    let Some(aeon) = reference_tree_for_profile(&profile) else {
        return;
    };
    let _lk = LOCK.lock().unwrap_or_else(|p| p.into_inner());

    // The committed golden ROM (its assembled region is the byte reference).
    let golden = std::fs::read(golden_dir().join(golden_name))
        .unwrap_or_else(|e| panic!("read golden {golden_name}: {e}"));

    // Build the demo ROM through the real frozen chain (native placement of the
    // demo game modules), then byte-compare each module window vs the golden.
    let native::RomBuild { rom, listing, .. } =
        native::build_rom_chained_with_listing(&aeon, &profile).unwrap_or_else(|e| {
            panic!("demo{} chained build: {e}", if debug { "_debug" } else { "" })
        });

    for (name, base, len) in WINDOWS {
        let got = &rom[*base..*base + *len];
        let want = &golden[*base..*base + *len];
        if let Some(i) = (0..*len).find(|&i| got[i] != want[i]) {
            let lo = i.saturating_sub(4);
            let hi = (i + 8).min(*len);
            panic!(
                "demo{} module `{name}` first diff at window offset {i:#x} (ROM {:#x}): got {:02x?}, want {:02x?}",
                if debug { "_debug" } else { "" },
                base + i,
                &got[lo..hi],
                &want[lo..hi]
            );
        }
    }

    // The frozen anchors resolve to their exact ROM addresses (the chainer placed
    // each section head where the demo golden has it).
    let resolve = |sym: &str| -> u32 {
        listing
            .iter()
            .find(|s| s.name == sym)
            .unwrap_or_else(|| panic!("`{sym}` absent from the demo listing"))
            .value
    };
    assert_eq!(resolve("DemoBox_Main"), 0x10002, "DemoBox_Main placement");
    assert_eq!(resolve("ObjDef_DemoBox"), 0x10006, "ObjDef_DemoBox placement");
    assert_eq!(resolve("GameState_Demo_Init"), 0x10100, "GameState_Demo_Init placement");
}

#[test]
fn demo_plain_game_modules_match_golden() {
    gate(false, "demo.bin");
}

#[test]
fn demo_debug_game_modules_match_golden() {
    gate(true, "demo.debug.bin");
}
