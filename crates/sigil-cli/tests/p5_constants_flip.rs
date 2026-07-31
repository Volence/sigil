//! Stage-3 P5 — the constants ownership flip's t24 negative probe (bar 2).
//!
//! `engine.constants` is now the SOLE author of the engine constants; the build
//! harvests its `pub const`s and injects them as GUARDED AS defines. The
//! structural no-silent-shadowing guarantee: if the residual AS ever reintroduces
//! an in-file `=`/`equ` for a flipped constant, the build FAILS LOUD with
//! `[defines.collision]` — it never silently prefers a side. This probe drives
//! that with the REAL harvested set + a REAL flipped name.

use sigil_frontend_as::{assemble, Options};
use sigil_harness::native::harvest_engine_constants;
use sigil_ir::backend::Cpu;
use std::path::Path;

fn aeon() -> String {
    std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string())
}

fn strict() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

#[test]
fn the_real_harvest_owns_the_flipped_constants() {
    let aeon = aeon();
    let Ok(harvested) = harvest_engine_constants(Path::new(&aeon)) else {
        if strict() {
            panic!("SIGIL_STRICT_GATE set but harvest failed (set AEON_DIR to a real aeon)");
        }
        eprintln!("skip: cannot harvest engine.constants (set AEON_DIR)");
        return;
    };
    let map: std::collections::HashMap<_, _> = harvested.iter().cloned().collect();
    // A spread of the flipped set across every block, with the exact frozen values.
    assert_eq!(map.get("HW_PORT_1_DATA"), Some(&0x00A1_0003));
    assert_eq!(map.get("NUM_PLAYERS"), Some(&2));
    assert_eq!(map.get("NUM_TOTAL_SLOTS"), Some(&66), "derived sum must resolve");
    assert_eq!(map.get("SECTION_SIZE_SHIFT"), Some(&11));
    assert_eq!(map.get("TILE_CACHE_NT_SIZE"), Some(&9600), "derived product must resolve");
    assert_eq!(map.get("PHYS_JUMP_RELEASE_CAP"), Some(&-0x400), "negative value preserved");
    // The struct-generated symbol is EXCLUDED (it stays structs.asm's, uncollided).
    assert!(!map.contains_key("VDP_Shadow_len"), "VDP_Shadow_len must not be harvested");
    assert!(harvested.len() >= 110, "the full flipped block, got {}", harvested.len());
}

#[test]
fn reintroducing_an_in_file_definition_is_a_hard_collision() {
    let aeon = aeon();
    let Ok(harvested) = harvest_engine_constants(Path::new(&aeon)) else {
        if strict() {
            panic!("SIGIL_STRICT_GATE set but harvest failed (set AEON_DIR)");
        }
        eprintln!("skip: cannot harvest engine.constants (set AEON_DIR)");
        return;
    };
    // The residual AS reintroduces `NUM_PLAYERS = 2` — the exact silent-shadow the
    // flip forbids. The guard must fail LOUD naming the constant.
    let src = "cpu 68000\nNUM_PLAYERS = 2\nB:\tdc.b NUM_PLAYERS\n";
    let opts = Options {
        initial_cpu: Cpu::M68000,
        defines: vec![],
        guarded_defines: harvested.clone(),
        include_root: None,
    };
    let diags = assemble(src, &opts)
        .expect_err("reintroducing a flipped constant in the residual AS must fail");
    assert!(
        diags.iter().any(|d| {
            d.message.contains("[defines.collision]") && d.message.contains("NUM_PLAYERS")
        }),
        "expected [defines.collision] naming NUM_PLAYERS, got {diags:?}"
    );

    // Positive control (t24): the SAME residual AS without the reintroduced line
    // assembles clean and reads the flipped constant from the guarded define.
    let clean = "cpu 68000\nB:\tdc.b NUM_PLAYERS\n";
    let opts2 = Options {
        initial_cpu: Cpu::M68000,
        defines: vec![],
        guarded_defines: harvested,
        include_root: None,
    };
    let module = assemble(clean, &opts2).expect("undoctored residual AS must assemble");
    let byte = module
        .sections
        .iter()
        .flat_map(|s| s.fragments.iter())
        .find_map(|f| match f {
            sigil_ir::Fragment::Data(d) => d.bytes.first().copied(),
            _ => None,
        });
    assert_eq!(byte, Some(2), "the residual AS must read NUM_PLAYERS=2 from the guarded define");
}
