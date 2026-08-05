//! conv-g — the OJZ parallax block, region-level byte gate.
//!
//! `games/sonic4/data/parallax/configs.emp` (via the `engine.level.parallax_dsl`
//! authoring vocabulary) emits the six deform tables + twenty `parallax_config`
//! records that the old `data/parallax/*.asm` files expanded from
//! `parallax_macros.inc`. This gate lowers the REAL module through the native
//! build path (`build_emp`), links the self-contained `parallax_configs` section
//! (every deform-table pointer is an intra-section label — no externs), and
//! byte-compares against the reference ROM window `[DeformTable_Zero, ObjDef_Static)`.
//!
//! The block is shape-invariant; only its base moves (path_swap's debug guards
//! shift it +0x88). The six-target full-ROM CRC gates prove the same bytes in
//! context; this is the focused regression guard on the macro→comptime mapping
//! (factor encoding, `as.sin`/`as.int` deform tables, band decomposition).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test parallax_configs_port
//! ```

use sigil_harness::native::{self, build_emp};
use sigil_harness::pins;
use sigil_ir::{SectionPlacement, SymbolTable};
use std::path::Path;

// Per-shape span (objtest-gate: the PLAIN successor changed, so the plain span
// carries 2 more align-pad bytes than debug).

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

fn gate(debug: bool, rom_name: &str, base: usize) {
    let region_len: usize = if debug { pins::PARALLAX_CONFIGS.debug_len } else { pins::PARALLAX_CONFIGS.plain_len };
    let aeon_dir =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let aeon = Path::new(&aeon_dir);
    let rom_path = aeon.join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let profile = native::sonic4_profile(debug);
    let native::EmpProgram { sections, .. } =
        build_emp(aeon, &profile).unwrap_or_else(|e| panic!("build_emp: {e}"));

    // The parallax_configs section is self-contained (deform-table pointers are
    // intra-section labels), so it links standalone.
    let mut parallax: Vec<_> =
        sections.into_iter().filter(|s| s.name == "parallax_configs").collect();
    assert_eq!(parallax.len(), 1, "exactly one parallax_configs section");
    // Pin the section at the region base so intra-section deform-table pointers
    // relocate to the golden ROM addresses.
    parallax[0].lma = base as u32;
    parallax[0].placement = SectionPlacement::Pinned;
    parallax[0].group = None;

    let resolved = sigil_link::resolve_layout(&parallax, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link: {d:?}"));
    let section =
        linked.section("parallax_configs").expect("linked image carries parallax_configs");

    let expected = &refrom[base..base + region_len];
    // The pin span runs to the next section's aligned base; the emitted image may
    // stop a short all-zero align pad earlier (objtest-gate: the plain successor
    // changed and left 2 pad bytes). Same tolerance as the region gates.
    let emitted = section.bytes.len();
    assert!(
        emitted <= region_len && region_len - emitted < 16
            && expected[emitted..].iter().all(|&b| b == 0),
        "parallax block emitted {emitted:#x} bytes vs pin span {region_len:#x} — beyond align-pad tolerance"
    );
    if let Some(i) = (0..emitted).find(|&i| section.bytes[i] != expected[i]) {
        panic!(
            "parallax_configs ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
            if debug { "debug" } else { "plain" },
            &section.bytes[i.saturating_sub(4)..(i + 8).min(region_len)],
            &expected[i.saturating_sub(4)..(i + 8).min(region_len)]
        );
    }
}

#[test]
fn parallax_block_plain_matches_reference() {
    gate(false, "s4.bin", pins::PARALLAX_CONFIGS.plain_base as usize);
}

#[test]
fn parallax_block_debug_matches_reference() {
    gate(true, "s4.debug.bin", pins::PARALLAX_CONFIGS.debug_base as usize);
}
