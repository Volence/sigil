//! Shared test-support for the strict gates and the CLI port tests.
//!
//! Two idioms lived hand-copied across ~9 port/probe test files (`sigil-cli`
//! tests and `sigil-harness` tests). Both crates depend on `sigil-harness`, so
//! this is the one seam both can reach without a `#[path]` include or a new
//! test-only crate; the CLI tests call `sigil_harness::test_support::…` and the
//! harness's own integration tests call the same path.
//!
//! ## 1. The AS-truth equ blob for the `engine.constants` twin
//!
//! `engine/system/constants.emp` (aeon tree) is a drift-guarded MIRROR of
//! AS-side constants. Every sigil test that compiles it must synthesise an
//! AS-side `equ` blob supplying the truth values its `ensure(extern(…))` guards
//! read back through the link seam. **`engine/system/constants.asm` (and
//! `structs.asm` for the `SST_*` field pins) is the SOURCE OF TRUTH** — this
//! module carries those values in ONE place.
//!
//! ### Twin-growth procedure
//!
//! When the `constants.emp` twin grows a new guarded constant: (1) grow the twin
//! in the aeon tree, (2) add the matching `(name, rhs)` pair to
//! [`engine_constant_equs`] (or [`sst_field_equs`]) here, (3) done. No per-file
//! blobs — every gate reads this one list.
//!
//! ### Doctoring seam
//!
//! A drift PROBE deliberately doctors ONE value to prove a guard fires. Rather
//! than re-inventing the whole blob, a probe takes the `(name, rhs)` pairs from
//! [`engine_constant_equs`], post-edits the one entry it wants wrong, and
//! assembles via [`assemble_equ_pairs`] — see `with_engine_constant_override`.
//!
//! ## 2. The drift-guard filter
//!
//! `module.link_asserts` carries BOTH the twin drift guards AND the D2.29
//! `[layout.odd-item]` parity asserts. Counting/checking guards means excluding
//! the parity asserts; [`drift_guards_only`] / [`guard_assert_count`] are the
//! shared idiom.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::assert::MsgPart;
use sigil_ir::{Cpu, LinkAssert, Section};

// ── 1. The AS-truth equ blob ────────────────────────────────────────────────

/// The `SST_*` struct-field equs (`structs.asm`'s generated layout) that
/// `sst.emp`'s 30 drift guards read back through `extern()`, plus the
/// supply-only `SST_interact` ($4E) that `collision.emp`'s `interact_off()`
/// guard reads (31 entries; 30 guarded + 1 supply). Ordered as the struct
/// declares them. SOURCE OF TRUTH: `engine/objects/structs.asm`.
pub fn sst_field_equs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("SST_code_addr", "$00"),
        ("SST_x_pos", "$02"),
        ("SST_y_pos", "$06"),
        ("SST_x_vel", "$0A"),
        ("SST_y_vel", "$0C"),
        ("SST_render_flags", "$0E"),
        ("SST_collision_resp", "$0F"),
        ("SST_mappings", "$10"),
        ("SST_art_tile", "$14"),
        ("SST_width_pixels", "$16"),
        ("SST_height_pixels", "$17"),
        ("SST_anim", "$18"),
        ("SST_subtype", "$19"),
        ("SST_anim_table", "$1A"),
        ("SST_status", "$1E"),
        ("SST_angle", "$1F"),
        ("SST_prev_anim", "$20"),
        ("SST_anim_frame", "$21"),
        ("SST_anim_timer", "$22"),
        ("SST_mapping_frame", "$23"),
        ("SST_prev_frame", "$24"),
        ("SST_sprite_piece_count", "$25"),
        ("SST_parent_ptr", "$26"),
        ("SST_sibling_ptr", "$28"),
        ("SST_slot_tag", "$2A"),
        ("SST_entity_section_id", "$2B"),
        ("SST_entity_list_index", "$2C"),
        ("SST_layer", "$2D"),
        ("SST_sst_custom", "$2E"),
        ("SST_len", "$50"),
        // The engine-owned player-slot tail word (structs.asm: SST_sst_custom +
        // SST_CUSTOM_SIZE - 2 = $4E). Not one of sst.emp's 30 field guards —
        // supplied here so `collision.emp`'s `interact_off()` SST_interact guard
        // resolves its `extern("SST_interact")` across the link seam.
        ("SST_interact", "$4E"),
    ]
}

/// The `Act_*` / `Sec_*` struct-field equs (`structs.asm`'s generated layout)
/// that `engine/structs.emp`'s per-field drift wall reads back through
/// `extern()`, plus the `Act_len`/`Sec_len` totals. Ordered as the structs
/// declare them. Act's `$21` pad is anonymous in structs.asm (no equ), covered
/// by `Act_len`. SOURCE OF TRUTH: `engine/structs.asm`.
pub fn act_sec_field_equs() -> Vec<(&'static str, &'static str)> {
    vec![
        // Act (34 bytes / $22)
        ("Act_sec_grid_ptr", "$00"),
        ("Act_grid_w", "$04"),
        ("Act_grid_h", "$06"),
        ("Act_start_local_x", "$08"),
        ("Act_start_local_y", "$0A"),
        ("Act_start_sec_x", "$0C"),
        ("Act_start_sec_y", "$0D"),
        ("Act_act_bg_layout", "$0E"),
        ("Act_act_bg_tiles", "$12"),
        ("Act_act_parallax_config", "$16"),
        ("Act_act_art_pool_table", "$1A"),
        ("Act_act_art_pool_pages", "$1E"),
        ("Act_edge_mode", "$20"),
        ("Act_len", "$22"),
        // Sec (66 bytes / $42)
        ("Sec_sec_block_index", "$00"),
        ("Sec_sec_objects", "$04"),
        ("Sec_sec_rings", "$08"),
        ("Sec_sec_plc", "$0C"),
        ("Sec_sec_pal", "$10"),
        ("Sec_sec_parallax_config", "$14"),
        ("Sec_sec_raster_table", "$18"),
        ("Sec_sec_bg_layout", "$1C"),
        ("Sec_sec_type_table", "$20"),
        ("Sec_sec_pal_cycle", "$24"),
        ("Sec_sec_sound_bank", "$28"),
        ("Sec_sec_block_dict", "$2C"),
        ("Sec_sec_anim_blocks", "$30"),
        ("Sec_sec_collision_s4lz", "$34"),
        ("Sec_sec_flags", "$38"),
        ("Sec_sec_music", "$3A"),
        ("Sec_sec_pcfg_pad_3C", "$3C"),
        ("Sec_sec_camera_lookahead", "$3D"),
        ("Sec_sec_pcfg_pad_3E", "$3E"),
        ("Sec_sec_pcfg_pad_3F", "$3F"),
        ("Sec_sec_block_dict_len", "$40"),
        ("Sec_len", "$42"),
        // DMAEntry (the 14-byte DMA-queue entry twin, tranche 20) — the
        // structs.emp per-field wall reads these like the Act/Sec walls.
        ("DMAEntry_Reg94", "0"),
        ("DMAEntry_SizeH", "1"),
        ("DMAEntry_Reg93", "2"),
        ("DMAEntry_SizeL", "3"),
        ("DMAEntry_Reg97", "4"),
        ("DMAEntry_SrcH", "5"),
        ("DMAEntry_Reg96", "6"),
        ("DMAEntry_SrcM", "7"),
        ("DMAEntry_Reg95", "8"),
        ("DMAEntry_SrcL", "9"),
        ("DMAEntry_Command", "10"),
        // parallax_config (28 bytes / $1C) — moved to engine.structs at the
        // tranche-21 buffers port (2nd .emp consumer).
        ("parallax_config_len", "$1C"),
        ("parallax_config_pcfg_band_count", "0"),
        ("parallax_config_pcfg_v_factor_bg", "1"),
        ("parallax_config_pcfg_layer_mask", "3"),
        ("parallax_config_pcfg_v_center_y", "4"),
        ("parallax_config_pcfg_v_offset", "6"),
        ("parallax_config_pcfg_transition", "8"),
        ("parallax_config_pcfg_deform_speed_fg", "9"),
        ("parallax_config_pcfg_deform_speed_bg", "10"),
        ("parallax_config_pcfg_deform_table_fg", "12"),
        ("parallax_config_pcfg_deform_table_bg", "16"),
        ("parallax_config_pcfg_v_deform_table_bg", "20"),
        ("parallax_config_pcfg_v_deform_speed_bg", "24"),
        ("parallax_config_pcfg_v_deform_shift_bg", "25"),
        ("DMAEntry_len", "14"),
    ]
}

/// The engine-constant equs that `engine.constants`'s SURVIVING drift guards
/// read back through `extern()`. Post the Stage-3 P5 ownership flip,
/// `engine.constants` is the SOLE author of the engine constants (the build
/// harvests its `pub const`s and injects them as guarded AS defines), so its 114
/// mirror drift guards RETIRED — only `VDP_Shadow_len` survives, because it is
/// STRUCT-GENERATED by `structs.asm`'s VDP_Shadow struct (not injected, to avoid
/// colliding with that struct symbol), so it stays a genuine twin until the
/// structs ownership flip. This blob is what the surviving guard's `extern()`
/// reads; a module consuming an engine constant now imports it via
/// `use engine.constants` (comptime), not through this blob.
pub fn engine_constant_equs() -> Vec<(&'static str, &'static str)> {
    vec![
        // The one surviving twin: struct-generated AS-side (VDP_Shadow struct),
        // mirrored + guarded in constants.emp until the structs flip.
        ("VDP_Shadow_len", "19"),
    ]
}


/// Assemble a list of `(name, rhs)` equ pairs into `Vec<Section>`, appending a
/// `Stub:` label + `dc.w 0` so the equs (defined before any section) flush via
/// the AS front-end's `pending_equ_syms` into a real section. The universal
/// pattern behind every AS-truth-equ helper below.
pub fn assemble_equ_pairs(pairs: &[(&str, &str)]) -> Vec<Section> {
    let mut asm = String::from("cpu 68000\n");
    for (name, rhs) in pairs {
        asm.push_str(name);
        asm.push_str(" = ");
        asm.push_str(rhs);
        asm.push('\n');
    }
    asm.push_str("Stub:\n\tdc.w 0\n");
    let opts = Options { initial_cpu: Cpu::M68000, ..Options::default() };
    assemble(&asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (equ pairs): {d:?}"))
        .sections
}

/// The complete AS-truth equ blob for the `engine.constants` twin: the 24
/// engine constants its guards read. For gates that ALSO compile `sst.emp` (its
/// 30 `SST_*` guards), use [`as_engine_constants_and_sst_equs`].
pub fn as_engine_constants_equs() -> Vec<Section> {
    assemble_equ_pairs(&engine_constant_equs())
}

/// The AS-truth equ blob for gates that compile BOTH `constants.emp` and
/// `sst.emp` (e.g. the `collision.emp` / test-object gates): the 30 `SST_*`
/// field pins followed by the 24 engine constants.
pub fn as_engine_constants_and_sst_equs() -> Vec<Section> {
    let mut pairs = sst_field_equs();
    pairs.extend(engine_constant_equs());
    assemble_equ_pairs(&pairs)
}

/// The engine-constant pairs with EXACTLY ONE constant's RHS overridden — the
/// drift-probe seam. A probe passes the constant it wants wrong and a doctored
/// RHS; the guard for that constant must then fire loud (naming it), while every
/// other guard still passes. Panics if `name` isn't a real engine constant (so a
/// renamed constant can't silently turn the probe into a no-op).
pub fn with_engine_constant_override(name: &str, rhs: &str) -> Vec<(&'static str, String)> {
    let mut pairs: Vec<(&'static str, String)> =
        engine_constant_equs().into_iter().map(|(n, r)| (n, r.to_string())).collect();
    let slot = pairs
        .iter_mut()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("with_engine_constant_override: `{name}` is not an engine constant"));
    slot.1 = rhs.to_string();
    pairs
}

/// Assemble owned `(name, rhs)` pairs (the shape [`with_engine_constant_override`]
/// returns) — same flush pattern as [`assemble_equ_pairs`].
pub fn assemble_owned_equ_pairs(pairs: &[(&str, String)]) -> Vec<Section> {
    let borrowed: Vec<(&str, &str)> = pairs.iter().map(|(n, r)| (*n, r.as_str())).collect();
    assemble_equ_pairs(&borrowed)
}

// ── 2. The drift-guard filter ────────────────────────────────────────────────

/// `true` iff `a` is a twin DRIFT GUARD (not a D2.29 `[layout.odd-item]` parity
/// assert). Drift guards and parity asserts both ride `module.link_asserts`;
/// this is the predicate that tells them apart.
pub fn is_drift_guard(a: &LinkAssert) -> bool {
    // Exclude the D2.29 STRUCTURAL alignment asserts — both the `[layout.odd-item]`
    // odd-address parity asserts and the `[layout.align]` congruence asserts that
    // an `align` / `table item_align:` pad records. Neither is a user DRIFT guard
    // (an `ensure`/twin-mirror co-residency check); they are layout invariants.
    !a.message.iter().any(|p| {
        matches!(p, MsgPart::Text(t) if t.contains("[layout.odd-item]") || t.contains("[layout.align]"))
    })
}

/// The drift guards among `asserts`, excluding the `[layout.odd-item]` parity
/// asserts.
pub fn drift_guards_only(asserts: &[LinkAssert]) -> impl Iterator<Item = &LinkAssert> {
    asserts.iter().filter(|a| is_drift_guard(a))
}

/// Count the twin drift guards in `asserts` (excludes `[layout.odd-item]` parity
/// asserts). The established `guard_assert_count` idiom, now shared.
pub fn guard_assert_count(asserts: &[LinkAssert]) -> usize {
    drift_guards_only(asserts).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_constants_blob_provides_the_surviving_twin_guard() {
        let secs = as_engine_constants_equs();
        // Non-empty: the `Stub:` carrier flushed the equs into a real section.
        assert!(!secs.is_empty(), "the equ blob must produce at least the Stub section");
        assert_eq!(
            engine_constant_equs().len(),
            1,
            "post the P5 ownership flip only VDP_Shadow_len's guard survives (the \
             struct-generated twin); the other 114 constants are now .emp-owned and \
             injected as guarded defines, so their mirror drift guards retired"
        );
    }

    #[test]
    fn sst_and_constants_blob_carries_both_layers() {
        let _ = as_engine_constants_and_sst_equs();
        assert_eq!(
            sst_field_equs().len(),
            31,
            "sst.emp guards 30 SST_* fields + 1 supply-only SST_interact for collision.emp"
        );
    }

    // The `override_doctors_exactly_one_and_keeps_the_rest` test retired with the
    // P5 ownership flip: it doctored `BUTTON_UP` and asserted the OTHER engine
    // constants kept their truth values, but `engine_constant_equs` now carries a
    // single entry (`VDP_Shadow_len`) — there is no "rest" to keep, and BUTTON_UP
    // is no longer an engine-constant equ. The `override_of_unknown_constant_panics`
    // test below still exercises `with_engine_constant_override`'s typo guard.

    #[test]
    #[should_panic(expected = "is not an engine constant")]
    fn override_of_unknown_constant_panics() {
        let _ = with_engine_constant_override("NOT_A_CONSTANT", "0");
    }
}
