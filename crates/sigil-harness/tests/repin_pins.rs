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
    // BASES: shifted by the byte-changing wave (items 5/10/11) — dma_queue
    // item-11 carry-return grew the engine block +0xC upstream of everything,
    // and dplc item-11 grew +0xC more. animate's OWN plain LEN shrank −8 (item 5:
    // drop both Sound_PlaySFX saves), so its debug LEN is 0x2A8 (was 0x2B0).
    assert_eq!(pins::ANIMATE.plain_base, 0x2EBA);
    assert_eq!(pins::ANIMATE.debug_base, 0x3494);
    assert_eq!(pins::ANIMATE.plain_len, 0x18A);  // −8: item 5 (drop both Sound_PlaySFX saves)
    assert_eq!(pins::ANIMATE.debug_len, 0x2A8);

    // rings_port.rs: the campaign's first shape-dependent LENGTH. RINGS LEN
    // shrank −6 (item 10: DrawRings camera-bias fold nets −6 B). Bases shifted by
    // the upstream wave.
    assert_eq!(pins::RINGS.plain_base, 0x3244);
    assert_eq!(pins::RINGS.debug_base, 0x3944);
    assert_eq!(pins::RINGS.plain_len, 0x1B8);   // −6: item 10 DrawRings fold
    assert_eq!(pins::RINGS.debug_len, 0x214);

    // core LEN unchanged. Bases shifted +0x18 plain (dma_queue +0xC + dplc
    // item-11 +0xC precede core). debug_len 0x6C8 (unchanged — items 5/10/11 are
    // downstream of / plain-only within core).
    assert_eq!(pins::CORE.plain_base, 0x27AC);
    assert_eq!(pins::CORE.plain_len, 0x2EE);
    assert_eq!(pins::CORE.debug_base, 0x293E);
    assert_eq!(pins::CORE.debug_len, 0x736);
    assert_eq!(pins::DPLC.plain_base, 0x2708);  // +0xC: dma_queue item-11 growth precedes dplc
    assert_eq!(pins::DPLC.debug_base, 0x289A);
    assert_eq!(pins::DPLC.plain_len, 0xA4);     // +0xC: item-11 bcs + post-loop commit (both procs)
    assert_eq!(pins::DPLC.debug_len, 0xA4);   // item 6 REMOVED (soak disproved single-entry) — debug == plain

    // animate_port.rs: the DeleteObject inbound label. Shifted by the upstream
    // wave (dma_queue + dplc item-11); DeleteObject's offset within core stable.
    assert_eq!(pins::DELETE_OBJECT, pins::Pin { plain: 0x2880, debug: 0x2A12 });

    // m1d_rom.rs / m1d_debug_rom.rs / mixed_dac_rom.rs: the END-line pins.
    // +0xCC both shapes from the churn-first ObjectTest scene (test_churn.asm +
    // object_test_state growth), then +0xC debug only from the OJZ scene-pin
    // hook's two `ifdef __DEBUG__` guards (Debug_Scene_Freeze).
    assert_eq!(pins::ASSEMBLED_LEN, 0x65B60);       // +0xCC churn
    assert_eq!(pins::DEBUG_ASSEMBLED_LEN, 0x6765A); // +0xCC churn +0xC hook guards

    // animate_port.rs: `AnimateSprite.cc_delete` − `AnimateSprite`. Shape-
    // DEPENDENT (item 4). Offset stable within animate (.cc_delete precedes the
    // item-5 .evt_sound edit), so plain 0x104 / debug 0x15E hold.
    assert_eq!(pins::CC_DELETE_OFF, pins::ShapeOffset { plain: 0x104, debug: 0x15E });
}

/// The remaining pin classes the migration will lean on: per-shape offsets
/// (rings), literal-len regions (sound_api), debug-only symbols (MDDBG),
/// and a RAM-cell Pin — all against the hand-typed sources.
#[test]
fn secondary_pin_classes_match_the_hand_typed_baseline() {
    // rings_port.rs: ringcol_off, the one per-shape offset. −6 (item 10:
    // DrawRings shrinks ahead of RingCollision within the region).
    assert_eq!(pins::RINGCOL_OFF, pins::ShapeOffset { plain: 0x116, debug: 0x172 });

    // sound_api_port.rs: base + literal len (no end symbol in the listing).
    // Bases slid −4 (t10), −8 (t11), +8 (A1), +4/+2 (C-A1/Bug-1), +0xA (ring-art
    // DrawRings), +0x22 (object-pool occupancy core growth), then −0x1C plain /
    // −0xC debug (tranche-12 entity_window step-2 branch shrink), then the whole
    // retro-fix-audit-1 batch. Item 11's dma_queue +0xC shifts BOTH shapes;
    // items 5 (−8) / 10 (−6) net into the plain base too. Plain 0x5D46 / debug
    // 0x770E. Then −0x6 both shapes (tranche-13 load_object: step-2 `bne.w
    // .alloc_fail` → bne.s −2, step-5 Load_ObjectList redundant a0 save/restore
    // removed −4; both upstream of sound_api). Then −0x16 both shapes (t13
    // step-5 second look: Load_Object burst copy movem-pairs → 6× move.l −0x10,
    // d4 push/pop eliminated −0x4, Load_ObjectList `bsr.w Load_Object` → bsr.s
    // −0x2 as the −0x14 shrink pulled the backward target into .s range). Then
    // −0xE both shapes (tranche-15 section.emp step-2: the modernization to bare
    // Bcc / jbra / jbsr relaxed 7-8 of section.asm's conservatively-.w branches
    // to .s at asl's fixpoint, shrinking the section region 0x3EA→0x3DC; section
    // is upstream of sound_api in the pre-$10000 engine bank). Then −0xE plain /
    // −0x6 debug (tranche-16 tile_cache.emp step-2: the same bare-Bcc/jbra/jbsr
    // modernization relaxed 7 plain / 3 debug of tile_cache.asm's conservative-.w
    // branches to .s — 4 of them shape-divergent (ifdef __DEBUG__, the assert
    // block blocks .s in debug), shrinking the tile_cache region 0x924→0x916 /
    // 0x9DC→0x9D6; tile_cache is upstream of sound_api in the engine bank).
    assert_eq!(pins::SOUND_API.plain_base, 0x5D78);
    assert_eq!(pins::SOUND_API.debug_base, 0x769C);
    assert_eq!(pins::SOUND_API.plain_len, 0x1E4);
    // debug_len grew 0x1E4 -> 0x2DA (retro-fix batch 2: the PlayMusic song-id +
    // PlaySFX ring-full DEBUG asserts, +0xF6); plain unchanged (release ROM
    // byte-IDENTICAL — literal len + debug_len override, no end-symbol shipped).
    // SOUND_PLAY_SFX_OFF became per-shape (PlayMusic's asserts precede Sound_PlaySFX).
    assert_eq!(pins::SOUND_API.debug_len, 0x2DA);
    assert_eq!(pins::SOUND_PLAY_SFX_OFF, pins::ShapeOffset { plain: 0x100, debug: 0x1B0 });

    // rings_port.rs DEBUG.labels: the debug-only error-handler entries.
    // +0xCC (churn) +0xC (hook guards) both in the debug ROM, like DEBUG_ASSEMBLED_LEN.
    assert_eq!(pins::MDDBG_ERROR_HANDLER, 0x6_6704);
    assert_eq!(pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER, 0x6_74CA);

    // collision_port.rs: sign-extended RAM labels truncated to u32. debug +0x2:
    // Debug_Scene_Freeze's RAM byte+pad shifts every __DEBUG__-block-downstream
    // RAM symbol (Player_1 among them); plain shape unchanged.
    assert_eq!(pins::PLAYER_1, pins::Pin { plain: 0xFFFF_89EE, debug: 0xFFFF_8A12 });
    // DYNAMIC_SLOTS also debug +0x2 (downstream of the __DEBUG__ block).
    assert_eq!(pins::DYNAMIC_SLOTS, pins::Pin { plain: 0xFFFF_8A8E, debug: 0xFFFF_8AB2 });
}
