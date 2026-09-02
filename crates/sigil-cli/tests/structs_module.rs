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
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, or
//! `EMPYREAN_SUITE_ROOT`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test structs_module
//! ```

use sigil_harness::native::harvest_engine_struct_offsets;
use sigil_harness::test_support::reference_tree;
use std::collections::HashMap;

/// The harvested `<Struct>_<field>` / `<Struct>_len` equs, or `None` when the
/// reference tree lacks a source the harvest reads. The listed paths are
/// `types.emp` (the ambient type environment) plus every file
/// `STRUCT_OFFSET_TWINS` names.
fn harvested() -> Option<HashMap<String, i64>> {
    let aeon = reference_tree(&[
        "engine/system/types.emp",
        "engine/structs.emp",
        "engine/objects/sst.emp",
        "engine/objects/entity_window.emp",
        "engine/level/parallax.emp",
    ])?;
    Some(
        harvest_engine_struct_offsets(&aeon)
            .expect("struct-offset harvest")
            .into_iter()
            .collect(),
    )
}

#[test]
fn harvest_emits_the_as_field_offsets_and_sizes() {
    let Some(h) = harvested() else { return };
    // A spread across all eight struct twins — the offsets structs.asm's
    // `struct … endstruct` generated, now authored by the `.emp` structs.
    let want: &[(&str, i64)] = &[
        // Act ($26) — grew from $22 when P2b added act_sec_local_maps at $22
        // (art-streaming Task 5); the harvest expectation was stale until the
        // full suite ran under Task 6.
        ("Act_sec_grid_ptr", 0x00),
        ("Act_grid_w", 0x04),
        ("Act_edge_mode", 0x20),
        ("Act_len", 0x28),   // P2 grew Act 0x22->0x28 (art-pool table/pages/budget fields); was stale at 0x26 (P-3 family)
        // Sec ($42)
        ("Sec_sec_block_index", 0x00),
        ("Sec_sec_parallax_config", 0x14),
        ("Sec_sec_block_dict_len", 0x40),
        ("Sec_len", 0x42),
        // DMAEntry (14)
        ("DMAEntry_Reg94", 0),
        ("DMAEntry_Command", 10),
        ("DMAEntry_len", 14),
        // parallax_config ($1E since band-ceiling-16, 2026-08-27 — was $1C).
        // MAX_PARALLAX_BANDS 8 -> 16 forced the one-bit-per-band
        // `pcfg_layer_mask` u8 -> u16, which needs an EVEN slot: it took $02
        // (the reserved `pcfg_v_factor_fg`'s), pushing that field to the tail at
        // $1C and adding a $1D evenness pad. Everything from $04 through $1B is
        // untouched, which is why the two spot-checks below still read 0 and 12.
        ("parallax_config_pcfg_band_count", 0),
        ("parallax_config_pcfg_layer_mask", 2),
        ("parallax_config_pcfg_deform_table_fg", 12),
        ("parallax_config_pcfg_v_factor_fg", 0x1C),
        ("parallax_config_len", 0x1E),
        // Sst ($52 since bug005 — the engine-owned frame_off cache word
        // appended at $50; sprites H1) + the derived tail word
        ("SST_code_addr", 0x00),
        ("SST_x_pos", 0x02),
        ("SST_sst_custom", 0x30),  // sst-fold
        ("SST_frame_off", 0x2E),   // sst-fold
        ("SST_len", 0x50),         // sst-fold
        ("SST_interact", 0x4E),
        // EntityScanState ($1A)
        ("EntityScanState_ess_ring_right_idx", 0x00),
        ("EntityScanState_ess_obj_next_x", 0x18),
        ("EntityScanState_len", 0x1A),
        // band_entry (10) — T7 re-glue: top widened to a u16 PLANE LINE (renamed
        // band_top_cell -> band_top_plane), the two 1-bit factor ops packed into
        // band_factor_ops; RESHAPED NOT RESIZED, so len stays 10 and the tail
        // field keeps its offset.
        ("band_entry_band_top_plane", 0),
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
    let Some(h) = harvested() else { return };
    // The one derived equate structs.asm carried outside a struct block.
    // sst-fold (2026-08-05): frame_off moved into the engine block at $2E, so
    // the custom window is the record tail again and
    // interact_off() = sizeof(Sst) - 2 (bug005 briefly had it frame_off - 2).
    assert_eq!(
        h["SST_interact"],
        h["SST_len"] - 2,
        "SST_interact = SST_len - 2 (the custom window's tail word)"
    );
}
