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

/// The engine-constant equs that `engine.constants`'s 30 drift guards read back
/// through `extern()`. SOURCE OF TRUTH: `engine/system/constants.asm`.
///
/// (The four `BUTTON_*` are written as plain magnitudes here; some hand-copied
/// call sites used `1<<0`-form RHSs, but the drift guards compare the RESOLVED
/// value, so `1` and `1<<0` are interchangeable — this list uses one form.)
pub fn engine_constant_equs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("BUTTON_UP", "1"),
        ("BUTTON_DOWN", "2"),
        ("BUTTON_LEFT", "4"),
        ("BUTTON_RIGHT", "8"),
        ("HW_PORT_1_DATA", "$A10003"),
        ("HW_PORT_2_DATA", "$A10005"),
        ("CTYPE_AIR", "0"),
        ("VDP_Shadow_len", "19"),
        ("RF_COORDMODE", "3"),
        ("RF_PRIORITY_SHIFT", "5"),
        // Animation block (tranche 9 — AF_* truth re-homed from animate.asm to
        // engine/constants.asm at the animate port so script data files survive
        // the SIGIL_EMP_ANIMATE gate; consumed-only mirroring, kill-list row 2).
        ("AF_END", "$FF"),
        ("AF_BACK", "$FE"),
        ("AF_DELETE", "$FB"),
        ("AF_SET_FIELD", "$F7"),
        ("DUR_DYNAMIC", "$FF"),
        ("OBJ_CODE_BANK", "1"),
        ("FRAME_PIECE_COUNT", "4"),
        ("NUM_PLAYERS", "2"),
        ("NUM_DYNAMIC", "40"),
        ("NUM_SYSTEM", "8"),
        ("NUM_EFFECTS", "16"),
        // Object-core block (tranche 10 — NUM_TOTAL_SLOTS is the pool sum,
        // culling geometry + the untagged-slot sentinel; source of truth
        // engine/constants.asm).
        ("NUM_TOTAL_SLOTS", "66"),
        ("NUM_DYNAMIC_PENDING", "8"),
        ("CULL_DISTANCE_X", "$300"),
        ("CULL_DISTANCE_Y", "$200"),
        ("SLOT_TAG_UNTAGGED", "$FF"),
        ("COLLISION_TOUCH", "12"),
        ("ST_IN_AIR", "3"),
        ("ST_ON_OBJECT", "5"),
        // Ring geometry/animation (constants.asm:401-403) + VDP sprite-table
        // geometry (truth: engine/objects/sprites.asm:6-8 — kill-list row 17),
        // tranche 8. The GAME-owned ring capacity constants (MAX_RING_BUFFER
        // etc.) are rings.emp-local mirrors, supplied by rings_port.rs, not
        // this engine twin.
        ("RING_HEIGHT", "16"),
        ("RING_ANIM_FRAMES", "4"),
        ("RING_ANIM_SPEED", "8"),
        ("MAX_VDP_SPRITES", "80"),
        ("VDP_SPRITE_X_OFFSET", "128"),
        ("VDP_SPRITE_Y_OFFSET", "128"),
        // Sprite rendering geometry + render-flag bits + frame-header offsets
        // (constants.asm — first consumed at the tranche-11 sprites.emp port).
        ("RF_ONSCREEN", "0"),
        ("RF_XFLIP", "1"),
        ("RF_YFLIP", "2"),
        ("RF_MULTISPRITE", "4"),
        ("PRIORITY_BANDS", "8"),
        ("SPRITES_PER_BAND", "32"),
        ("SCANLINE_BANDS", "7"),
        ("SCANLINE_SPRITE_LIMIT", "24"),
        ("SCREEN_WIDTH", "320"),
        ("SCREEN_HEIGHT", "224"),
        ("FRAME_BBOX_X_MIN", "0"),
        ("FRAME_BBOX_X_MAX", "1"),
        ("FRAME_BBOX_Y_MIN", "2"),
        ("FRAME_BBOX_Y_MAX", "3"),
        ("FRAME_PIECES", "6"),
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
    fn engine_constants_blob_assembles_and_defines_all_50() {
        let secs = as_engine_constants_equs();
        // Non-empty: the `Stub:` carrier flushed the equs into a real section.
        assert!(!secs.is_empty(), "the equ blob must produce at least the Stub section");
        assert_eq!(
            engine_constant_equs().len(),
            50,
            "the twin guards 50 engine constants (34 + tranche-11 sprites block of 15 + NUM_DYNAMIC_PENDING, A2)"
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

    #[test]
    fn override_doctors_exactly_one_and_keeps_the_rest() {
        let doctored = with_engine_constant_override("BUTTON_UP", "1<<4");
        let up: Vec<_> = doctored.iter().filter(|(n, _)| *n == "BUTTON_UP").collect();
        assert_eq!(up.len(), 1);
        assert_eq!(up[0].1, "1<<4", "the named constant is doctored");
        // Every other constant retains its truth value.
        let down = doctored.iter().find(|(n, _)| *n == "BUTTON_DOWN").unwrap();
        assert_eq!(down.1, "2", "untouched constants keep their truth value");
        // It still assembles (the probe needs real sections to link against).
        let _ = assemble_owned_equ_pairs(&doctored);
    }

    #[test]
    #[should_panic(expected = "is not an engine constant")]
    fn override_of_unknown_constant_panics() {
        let _ = with_engine_constant_override("NOT_A_CONSTANT", "0");
    }
}
