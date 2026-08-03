//! Shared struct module — `engine/structs.emp` + the struct-offset harvest.
//!
//! Post the conv-a structs flip, `engine.structs` (and its siblings sst.emp /
//! entity_window.emp / parallax.emp) is the SOLE AUTHOR of the object/section/
//! DMA/parallax/VDP-shadow layouts: the per-field `offsetof == extern` drift
//! walls retired (they became tautologies once the `.emp` owns the layout), and
//! the residual AS reads the offsets through `harvest_engine_struct_offsets`.
//!
//! This gate replaces the old drift-wall test with the POSITIVE re-home: it
//! asserts the harvester emits the exact `<Struct>_<field>` / `<Struct>_len` equs
//! the AS side used to generate (a wrong offset would move ROM bytes — the
//! six-target byte-identity is the ultimate check; this pins the mechanism).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test structs_module
//! ```

use sigil_harness::native::harvest_engine_struct_offsets;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn skip() -> bool {
    std::env::var("AEON_DIR").is_err() && !Path::new("/home/volence/sonic_hacks/aeon").exists()
}

fn harvested() -> HashMap<String, i64> {
    harvest_engine_struct_offsets(&aeon_dir())
        .expect("struct-offset harvest")
        .into_iter()
        .collect()
}

#[test]
fn harvest_emits_the_as_field_offsets_and_sizes() {
    if skip() {
        eprintln!("skip: AEON_DIR unset");
        return;
    }
    let h = harvested();
    // A spread across all eight struct twins — the offsets structs.asm's
    // `struct … endstruct` generated, now authored by the `.emp` structs.
    let want: &[(&str, i64)] = &[
        // Act ($22)
        ("Act_sec_grid_ptr", 0x00),
        ("Act_grid_w", 0x04),
        ("Act_edge_mode", 0x20),
        ("Act_len", 0x22),
        // Sec ($42)
        ("Sec_sec_block_index", 0x00),
        ("Sec_sec_parallax_config", 0x14),
        ("Sec_sec_block_dict_len", 0x40),
        ("Sec_len", 0x42),
        // DMAEntry (14)
        ("DMAEntry_Reg94", 0),
        ("DMAEntry_Command", 10),
        ("DMAEntry_len", 14),
        // parallax_config ($1C)
        ("parallax_config_pcfg_band_count", 0),
        ("parallax_config_pcfg_deform_table_fg", 12),
        ("parallax_config_len", 0x1C),
        // Sst ($52 since bug005 — the engine-owned frame_off cache word
        // appended at $50; sprites H1) + the derived tail word
        ("SST_code_addr", 0x00),
        ("SST_x_pos", 0x02),
        ("SST_sst_custom", 0x2E),
        ("SST_frame_off", 0x50),
        ("SST_len", 0x52),
        ("SST_interact", 0x4E),
        // EntityScanState ($1A)
        ("EntityScanState_ess_ring_right_idx", 0x00),
        ("EntityScanState_ess_obj_next_x", 0x18),
        ("EntityScanState_len", 0x1A),
        // band_entry (10)
        ("band_entry_band_top_cell", 0),
        ("band_entry_band_phase_offset", 9),
        ("band_entry_len", 10),
        // VdpShadow (19)
        ("VDP_Shadow_vdp_mode1", 0x00),
        ("VDP_Shadow_vdp_mode2", 0x01),
        ("VDP_Shadow_vdp_mode3", 0x0B),
        ("VDP_Shadow_vdp_hint_rate", 0x0A),
        ("VDP_Shadow_len", 19),
    ];
    for (name, off) in want {
        assert_eq!(
            h.get(*name),
            Some(off),
            "harvest must emit {name} = {off:#x} (got {:?})",
            h.get(*name)
        );
    }
}

#[test]
fn sst_interact_is_the_record_tail_word() {
    if skip() {
        eprintln!("skip: AEON_DIR unset");
        return;
    }
    let h = harvested();
    // The one derived equate structs.asm carried outside a struct block.
    // bug005: the record's tail is now the engine's frame_off cache ($50-$51),
    // so the custom window's tail word sits immediately BELOW frame_off —
    // interact_off() = offsetof(Sst, frame_off) - 2, no longer sizeof - 2.
    assert_eq!(
        h["SST_interact"],
        h["SST_frame_off"] - 2,
        "SST_interact = SST_frame_off - 2 (the word below the H1 cache)"
    );
}
