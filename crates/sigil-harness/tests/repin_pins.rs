//! The `repin` staleness guard (D-T10.5) + the acceptance baseline (D-T10.8).
//!
//! `pins_rs_is_current` regenerates the pin table IN-MEMORY from the live
//! listings and compares against the committed `src/pins.rs` — a stale
//! pins.rs can no longer hide. REFERENCE-DEPENDENT: needs the sibling `aeon`
//! tree's `s4.lst`/`s4.debug.lst` (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, it SKIPS green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! The acceptance tests are HERMETIC: they pin the committed `pins::*`
//! values against the hand-typed literals the 16 test files carried at the
//! tool's first green (the tranche-10 design note's survey table). If the
//! generator ever mis-derives a value, the mismatch surfaces HERE first,
//! named — not as a byte-diff panic three suites later.

use std::path::PathBuf;

use sigil_harness::pins;
use sigil_harness::repin::{
    diff_pins, load_manifest, parse_listing, render, resolve, strip_provenance, Provenance,
};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The committed pins.rs must equal an in-memory regeneration from the live
/// listings, modulo the `[provenance]` stamp lines (a rebuild that moves no
/// pin is not drift).
#[test]
fn pins_rs_is_current() {
    let aeon = aeon_dir();
    let plain_path = aeon.join("s4.lst");
    let debug_path = aeon.join("s4.debug.lst");
    let (Ok(plain_txt), Ok(debug_txt)) =
        (std::fs::read_to_string(&plain_path), std::fs::read_to_string(&debug_path))
    else {
        if strict_gate() {
            panic!(
                "SIGIL_STRICT_GATE set but listings missing: {} / {}",
                plain_path.display(),
                debug_path.display()
            );
        }
        eprintln!("skip: listings not at {} (set AEON_DIR)", aeon.display());
        return;
    };

    let plain = parse_listing(&plain_txt)
        .unwrap_or_else(|e| panic!("{}: {e}", plain_path.display()));
    let debug = parse_listing(&debug_txt)
        .unwrap_or_else(|e| panic!("{}: {e}", debug_path.display()));
    let manifest = load_manifest(include_str!("../repin.toml")).expect("repin.toml must load");
    let resolved = resolve(&manifest, &plain, &debug).unwrap_or_else(|e| panic!("resolve: {e}"));
    let prov = Provenance {
        plain_path: plain_path.display().to_string(),
        debug_path: debug_path.display().to_string(),
        plain_stamp: plain.stamp.clone(),
        debug_stamp: debug.stamp.clone(),
    };
    let generated = render(&resolved, &prov);
    let committed = include_str!("../src/pins.rs");

    if strip_provenance(committed) != strip_provenance(&generated) {
        let changes = diff_pins(committed, &generated);
        let detail: Vec<String> = changes
            .iter()
            .map(|c| {
                format!(
                    "  {}: {} → {}",
                    c.name,
                    c.old.as_deref().unwrap_or("(new)"),
                    c.new.as_deref().unwrap_or("(removed)")
                )
            })
            .collect();
        panic!(
            "src/pins.rs is STALE against the live listings ({} changed pin(s)):\n{}\n\
             run: cargo run -p sigil-harness --bin repin",
            changes.len(),
            detail.join("\n")
        );
    }
}

/// D-T10.8 acceptance: the generated values byte-match the CURRENT
/// hand-typed pins for a representative spread of every pin class —
/// per-shape bases, shape-INVARIANT lens (animate), shape-DEPENDENT lens
/// (rings/core), literal-len regions (sound_api implicitly via SOUND_API in
/// the migration), symbol Pins, dotted-local offsets, and the ROM end pins.
#[test]
fn generated_pins_match_the_hand_typed_baseline() {
    // animate_port.rs: PLAIN/DEBUG Shape { base, len } — len shape-invariant.
    // Bases slid −4 (t10 core), −8 (t11 sprites), +8 (t11 A1 camera-bias),
    // −2 plain/−4 debug (C-A1 core shrink), +0x22 both (object-pool occupancy
    // grew the core region) — net.
    // debug_base moved by retro-fix-audit-1's DEBUG asserts UPSTREAM of animate:
    // dplc item-6 assert (+0xB0), core item-1 A2 rail + item-2 DeleteObject
    // asserts. animate's OWN debug LEN grew +0x11E (item-4 AF_* asserts). Plain
    // is byte-identical to baseline (every assert self-gates in release).
    assert_eq!(pins::ANIMATE.plain_base, 0x2E38);
    assert_eq!(pins::ANIMATE.debug_base, 0x34BE);
    assert_eq!(pins::ANIMATE.plain_len, 0x192);
    assert_eq!(pins::ANIMATE.debug_len, 0x2B0);

    // rings_port.rs: the campaign's first shape-dependent LENGTH.
    // Bases slid as animate above, incl. +0x22 (object-pool occupancy core
    // growth). LEN unchanged: R-A1's addi #16→#8 is same-size (immediate only).
    // debug_base moved again by items 1/2/4/6 (all upstream DEBUG growth). RINGS
    // LEN unchanged (rings itself untouched by this batch — items 10-11 pending).
    assert_eq!(pins::RINGS.plain_base, 0x31CA);
    assert_eq!(pins::RINGS.debug_base, 0x3976);
    assert_eq!(pins::RINGS.plain_len, 0x1BE);   // +0xA: ring-art DrawRings frame-tile calc
    assert_eq!(pins::RINGS.debug_len, 0x21A);

    // Object-pool occupancy geometry: core LEN +0x22 both shapes. debug_base
    // +0xB0 (dplc item-6 assert precedes core). debug_len +0x116 (item-2
    // DeleteObject bounds + double-delete asserts) on top of item-1's +0x6C.
    // Plain unchanged (asserts self-gate).
    assert_eq!(pins::CORE.plain_base, 0x2794);
    assert_eq!(pins::CORE.plain_len, 0x284);    // +0x8: step-6 frame-end compaction call
    assert_eq!(pins::CORE.debug_base, 0x29D6);
    assert_eq!(pins::CORE.debug_len, 0x6C8);    // step-7 asserts + item-1 A2 rail (+0x6C) + item-2 DeleteObject asserts (+0x116) — all self-gate in plain
    assert_eq!(pins::DPLC.plain_base, 0x26FC);
    assert_eq!(pins::DPLC.debug_base, 0x288E);  // dplc is first — nothing upstream grows it
    assert_eq!(pins::DPLC.plain_len, 0x98);
    assert_eq!(pins::DPLC.debug_len, 0x148);    // +0xB0: item-6 single-entry assert (×2 procs, self-gates in plain)

    // animate_port.rs: the DeleteObject inbound label. Plain unchanged; debug
    // +0xB0 (dplc item-6 assert shifts core's base; DeleteObject's offset within
    // core is unchanged — item-2's asserts sit AT its entry, inside it).
    assert_eq!(pins::DELETE_OBJECT, pins::Pin { plain: 0x284E, debug: 0x2A90 });

    // m1d_rom.rs / m1d_debug_rom.rs / mixed_dac_rom.rs: the END-line pins. The
    // batch's DEBUG growth is absorbed before the ROM end (unchanged both shapes).
    assert_eq!(pins::ASSEMBLED_LEN, 0x65A94);
    assert_eq!(pins::DEBUG_ASSEMBLED_LEN, 0x67582);

    // animate_port.rs: `AnimateSprite.cc_delete` − `AnimateSprite`. Shape-
    // DEPENDENT as of item 4 (the AF_* asserts sit between the region start and
    // .cc_delete, so debug 0x15E > plain 0x104).
    assert_eq!(pins::CC_DELETE_OFF, pins::ShapeOffset { plain: 0x104, debug: 0x15E });
}

/// The remaining pin classes the migration will lean on: per-shape offsets
/// (rings), literal-len regions (sound_api), debug-only symbols (MDDBG),
/// and a RAM-cell Pin — all against the hand-typed sources.
#[test]
fn secondary_pin_classes_match_the_hand_typed_baseline() {
    // rings_port.rs: ringcol_off, the one per-shape offset.
    assert_eq!(pins::RINGCOL_OFF, pins::ShapeOffset { plain: 0x11C, debug: 0x178 });

    // sound_api_port.rs: base + literal len (no end symbol in the listing).
    // Bases slid −4 (t10), −8 (t11), +8 (A1), +4/+2 (C-A1/Bug-1), +0xA (ring-art
    // DrawRings), +0x22 (object-pool occupancy core growth), then −0x1C plain /
    // −0xC debug (tranche-12 entity_window step-2 branch shrink), then the whole
    // retro-fix-audit-1 DEBUG batch (items 1/2/4/6/12 asserts + rails in dplc/
    // core/animate/collision/entity_window) grows the debug shape upstream —
    // debug_base lands at 0x7704. Plain unchanged (asserts self-gate in release).
    assert_eq!(pins::SOUND_API.plain_base, 0x5D3C);
    assert_eq!(pins::SOUND_API.debug_base, 0x7704);
    assert_eq!(pins::SOUND_API.plain_len, 0x1E4);
    assert_eq!(pins::SOUND_API.debug_len, 0x1E4);
    assert_eq!(pins::SOUND_PLAY_SFX_OFF, 0x100);

    // rings_port.rs DEBUG.labels: the debug-only error-handler entries.
    assert_eq!(pins::MDDBG_ERROR_HANDLER, 0x6_662C);
    assert_eq!(pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER, 0x6_73F2);

    // collision_port.rs: sign-extended RAM labels truncated to u32.
    assert_eq!(pins::PLAYER_1, pins::Pin { plain: 0xFFFF_89EE, debug: 0xFFFF_8A10 });
    assert_eq!(pins::DYNAMIC_SLOTS, pins::Pin { plain: 0xFFFF_8A8E, debug: 0xFFFF_8AB0 });
}
