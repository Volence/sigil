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
//!
//! ## 3. The REFERENCE-DEPENDENT guard
//!
//! A port gate compiles sources out of the sibling `aeon` working copy, which a
//! checkout of this repo alone does not have. [`reference_tree`] is the one
//! guard those tests open with: skip green when the named paths are absent, and
//! panic instead under `SIGIL_STRICT_GATE=1` so the pre-merge run cannot skip.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::assert::MsgPart;
use sigil_ir::{Cpu, LinkAssert, Section};
use std::path::PathBuf;

// ── 1. The AS-truth equ blob ────────────────────────────────────────────────

/// The `SST_*` struct-field equs that a standalone gate SUPPLIES so a module's
/// legitimate field-address `extern()`s resolve across the link seam — chiefly
/// `ojz_scroll_test.emp`'s `Player_1 + SST_x_pos` player-field addresses. Post
/// the conv-a structs flip, `sst.emp` OWNS this layout (its 30 drift guards
/// retired; the build harvests the offsets from the struct), so this is a
/// SUPPLY-ONLY blob — the equs match what the harvest injects. `SST_interact`
/// ($4E) is the derived tail word. Ordered as the struct declares them.
/// SOURCE OF TRUTH: `engine/objects/sst.emp`.
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
        // sst-fold (2026-08-05): frame_off moved into the engine block at $2E
        // (its bug005 bolt-on position was $50), the custom window is the record
        // tail at $30-$4F, and the record is back to $50.
        ("SST_frame_off", "$2E"),
        ("SST_sst_custom", "$30"),
        ("SST_len", "$50"),
        // The engine-owned player-slot tail word (structs.asm: SST_sst_custom +
        // SST_CUSTOM_SIZE - 2 = $4E). Not one of sst.emp's 30 field guards —
        // supplied here so `collision.emp`'s `interact_off()` SST_interact guard
        // resolves its `extern("SST_interact")` across the link seam.
        ("SST_interact", "$4E"),
    ]
}

/// The `Act_*` / `Sec_*` / `DMAEntry_*` / `parallax_config_*` struct-field equs
/// a standalone gate may SUPPLY. Post the conv-a structs flip, `engine.structs`
/// OWNS these layouts (the per-field drift wall retired; the build harvests the
/// offsets from the structs), so this is a SUPPLY-ONLY blob matching what the
/// harvest injects — the values track the struct declarations. Act's `$21` pad is
/// anonymous (covered by `Act_len`). SOURCE OF TRUTH: `engine/structs.emp`.
pub fn act_sec_field_equs() -> Vec<(&'static str, &'static str)> {
    vec![
        // Act (40 bytes / $28) — tracked by `tests/act_fixture_drift.rs` against
        // `harvest_engine_struct_offsets`, which reads the live struct.
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
        ("Act_pad_21", "$21"),
        ("Act_act_sec_local_maps", "$22"),
        ("Act_act_art_budget", "$26"),
        ("Act_len", "$28"),
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
        ("Sec_sec_effects", "$34"),
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
        ("parallax_config_pcfg_v_factor_fg", "2"),
        ("parallax_config_pcfg_layer_mask", "3"),
        ("parallax_config_pcfg_v_center_y", "4"),
        ("parallax_config_pcfg_v_offset", "6"),
        ("parallax_config_pcfg_transition", "8"),
        ("parallax_config_pcfg_deform_speed_fg", "9"),
        ("parallax_config_pcfg_deform_speed_bg", "10"),
        ("parallax_config_pcfg_anchor_ch", "11"),
        ("parallax_config_pcfg_deform_table_fg", "12"),
        ("parallax_config_pcfg_deform_table_bg", "16"),
        ("parallax_config_pcfg_v_deform_table_bg", "20"),
        ("parallax_config_pcfg_v_deform_speed_bg", "24"),
        ("parallax_config_pcfg_v_deform_shift_bg", "25"),
        ("parallax_config_pcfg_anchor_dsa", "26"),
        ("parallax_config_pcfg_anchor_dsb", "27"),
        ("DMAEntry_len", "14"),
    ]
}

/// The engine-constant equs that `engine.constants`'s SURVIVING drift guards
/// read back through `extern()`. Post the Stage-3 P5 constants flip AND the
/// conv-a structs flip, `engine.constants` is the SOLE author of every engine
/// constant and the struct twins own the layouts, so EVERY mirror drift guard
/// retired — including `VDP_Shadow_len`'s (its bridge dissolved when the VdpShadow
/// struct became the length author; vdp_init.emp now pins the const to
/// `sizeof(VdpShadow)` in-file). The list is empty: a module consuming an engine
/// constant imports it via `use engine.constants` (comptime), never through a
/// blob. Kept as the zero-length base so `twin_guards()` derivations stay 0.
pub fn engine_constant_equs() -> Vec<(&'static str, &'static str)> {
    vec![]
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

// ── 1b. The game-contract env for isolated port oracles (L1 P2) ──────────────

/// Build a resolved game-contract [`InterfaceEnv`](sigil_frontend_emp::contract::InterfaceEnv)
/// for a single-module port oracle. The whole-program build runs the bind pass
/// over its reachable module set; an oracle that lowers ONE engine module naming
/// `Game.MEMBER` / `invoke Game.hook` must synthesize the same env. `iface_src`
/// and `impl_src` are `.emp` sources for the interface and its one `implement`;
/// `defines` feeds any comptime-`if` binding group (the hotkeys shape). Asserts a
/// clean parse + bind — a malformed contract stub is a test bug, not a silent
/// empty env.
pub fn game_contract_env(
    iface_src: &str,
    impl_src: &str,
    defines: &[(String, i128)],
) -> sigil_frontend_emp::contract::InterfaceEnv {
    use sigil_frontend_emp::resolve::contract::{bind, ContractModule};
    let (ef, ed) = sigil_frontend_emp::parse_str(iface_src);
    let (gf, gd) = sigil_frontend_emp::parse_str(impl_src);
    assert!(
        ed.iter().chain(&gd).all(|d| d.level != sigil_span::Level::Error),
        "game_contract_env parse diags: iface={ed:?} impl={gd:?}"
    );
    let eid = ef.module.path.segments.join(".");
    let gid = gf.module.path.segments.join(".");
    let mods =
        [ContractModule { id: &eid, file: &ef }, ContractModule { id: &gid, file: &gf }];
    let (env, diags) = bind(&mods, defines);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "game_contract_env bind diags: {diags:?}"
    );
    env
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

// ── 3. The REFERENCE-DEPENDENT guard ────────────────────────────────────────

/// The aeon reference tree: `AEON_DIR`, or the workspace default.
pub fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

/// `true` when `SIGIL_STRICT_GATE` is set — the pre-merge fidelity run, where a
/// missing reference is a FAILURE rather than a skip.
pub fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The guard every REFERENCE-DEPENDENT test opens with: `Some(aeon)` when the
/// tree carries every path in `rels`, `None` — skip green — when it does not.
///
/// The skip exists for environments that check out THIS repo alone (CI): the
/// aeon tree is a sibling working copy, not a dependency sigil can vendor. It
/// must never quietly swallow a real gate, so `SIGIL_STRICT_GATE=1` turns a
/// missing reference into a panic naming the absent path — that is the flag the
/// pre-merge run sets, and under it these tests cannot skip.
///
/// `rels` are the aeon-relative paths the caller actually reads (`.emp` sources,
/// fixtures, ROMs). Naming them — rather than probing the tree root — is what
/// makes the guard honest against an `AEON_DIR` pointed at an incomplete tree.
pub fn reference_tree(rels: &[&str]) -> Option<PathBuf> {
    let aeon = aeon_dir();
    if let Some(missing) = rels.iter().find(|rel| !aeon.join(rel).exists()) {
        let path = aeon.join(missing);
        assert!(
            !strict_gate(),
            "SIGIL_STRICT_GATE set but reference missing: {}",
            path.display()
        );
        eprintln!("skip: reference not at {} (set AEON_DIR)", path.display());
        return None;
    }
    Some(aeon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_constant_equs_is_empty_after_the_flips() {
        // Post the P5 constants flip AND the conv-a structs flip every engine
        // constant is .emp-owned and injected as a guarded define, so no mirror
        // drift guard reads a blob value — the list is the zero-length base.
        assert_eq!(
            engine_constant_equs().len(),
            0,
            "all engine-constant + struct-len mirror guards retired; VDP_Shadow_len \
             is now pinned in-file against sizeof(VdpShadow)"
        );
        // The `Stub:` carrier still flushes an (empty) section — a real Vec.
        let _ = as_engine_constants_equs();
    }

    #[test]
    fn sst_supply_blob_carries_the_field_addresses() {
        let _ = as_engine_constants_and_sst_equs();
        assert_eq!(
            sst_field_equs().len(),
            32,
            "the SUPPLY-ONLY SST_* blob carries 31 field offsets (bug005 added the \
             frame_off tail word) + SST_interact for a standalone gate's legitimate \
             field-address externs (ojz_scroll_test)"
        );
    }

    // The `override_doctors_exactly_one_and_keeps_the_rest` test retired with the
    // P5 ownership flip (it doctored an engine constant and asserted the rest kept
    // their truth values; `engine_constant_equs` is now empty — no "rest"). The
    // `override_of_unknown_constant_panics` test below still exercises
    // `with_engine_constant_override`'s typo guard against the empty list.

    #[test]
    #[should_panic(expected = "is not an engine constant")]
    fn override_of_unknown_constant_panics() {
        let _ = with_engine_constant_override("NOT_A_CONSTANT", "0");
    }

    /// EFX-6: the `act_sec_field_equs` supply-only blob had nothing cross-checking
    /// it against the live structs, so a renamed `Act`/`Sec` field could leave a
    /// STALE name in the blob — a dead equ that standalone port test oracles then
    /// resolve against nothing. `act_fixture_drift.rs` already checks the fixture's
    /// VALUES against the harvest and that the harvest's fields are all COVERED by
    /// the fixture, but neither direction catches a name the fixture still supplies
    /// that the live structs no longer declare. This is that missing direction.
    #[test]
    fn sec_field_equ_names_match_the_harvest() {
        let Some(aeon) = reference_tree(&["engine/structs.emp"]) else { return };
        let harvested = crate::native::harvest_engine_struct_offsets(&aeon)
            .expect("harvest_engine_struct_offsets must succeed");
        let names: std::collections::BTreeSet<&str> =
            harvested.iter().map(|(n, _)| n.as_str()).collect();
        for (name, _) in act_sec_field_equs() {
            assert!(
                names.contains(name),
                "test_support.rs supplies `{name}`, which no longer exists in \
                 engine/structs.emp — a renamed Act/Sec/DMAEntry/parallax_config field \
                 leaves this blob supplying a DEAD equ that standalone port test oracles \
                 then resolve against nothing (EFX-6)"
            );
        }
    }
}
