//! Shared struct module: `engine/structs.emp` plus the struct-offset harvest.
//!
//! `engine.structs` (and its siblings sst.emp / entity_window.emp / parallax.emp) is
//! the SOLE AUTHOR of the object/section/DMA/parallax/VDP-shadow layouts, and the
//! residual AS reads the offsets through `harvest_engine_struct_offsets`, which shapes
//! them as the `<Struct>_<field>` / `<Struct>_len` equs `engine/structs.asm` once
//! generated. A wrong offset moves ROM bytes, so the byte gates are the end-to-end
//! check; this file pins the harvest itself, two ways:
//!
//! - over the REFERENCE TREE, the harvest must EQUAL the equs derived from the same
//!   sources by a route that never runs it: each struct's fields enumerated from its
//!   parsed declaration, each value evaluated through the language's own `offsetof` /
//!   `sizeof` (and aeon's `interact_off()` for `SST_interact`), which is what `.emp`
//!   code reading those structs sees. Both routes stand on the same parser and the
//!   same layout arithmetic, so this half checks the harvest's enumeration, naming and
//!   sizing, not the arithmetic;
//! - over a FIXED INPUT, a synthetic tree this file writes, the harvest must equal
//!   literal values true of that input, which pins the arithmetic the first half shares.
//!
//! The first test is REFERENCE-DEPENDENT: it needs the sibling `aeon` tree
//! (`AEON_DIR`, or `EMPYREAN_SUITE_ROOT`) and SKIPS green without it, unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure. The fixed-input
//! test needs no tree.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test structs_module
//! ```

use sigil_frontend_emp::ast::{Expr, File, Item};
use sigil_frontend_emp::eval::eval_expr_in_file_ambient;
use sigil_frontend_emp::parse_str;
use sigil_harness::native::harvest_engine_struct_offsets;
use sigil_harness::test_support::reference_tree;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// The ambient type environment the harvest lays every struct out against.
const TYPES_REL: &str = "engine/system/types.emp";

/// The struct twins the harvest shapes, as `(file, struct, AS prefix)`. The prefix is
/// the AS spelling the residual AS reads (`Sst` carries `SST_*`, `VdpShadow` carries
/// `VDP_Shadow_*`). It is restated here rather than imported from `native`, so a
/// changed spelling or a dropped twin there is a difference this gate sees.
const TWINS: &[(&str, &str, &str)] = &[
    ("engine/structs.emp", "Act", "Act"),
    ("engine/structs.emp", "Sec", "Sec"),
    ("engine/structs.emp", "DMAEntry", "DMAEntry"),
    ("engine/structs.emp", "parallax_config", "parallax_config"),
    ("engine/structs.emp", "VdpShadow", "VDP_Shadow"),
    ("engine/objects/sst.emp", "Sst", "SST"),
    (
        "engine/objects/entity_window.emp",
        "EntityScanState",
        "EntityScanState",
    ),
    ("engine/level/parallax.emp", "band_entry", "band_entry"),
];

/// Every source the harvest reads: the type environment plus each twin's file.
fn sources() -> Vec<&'static str> {
    let mut rels = vec![TYPES_REL];
    for (rel, _, _) in TWINS {
        if !rels.contains(rel) {
            rels.push(rel);
        }
    }
    rels
}

/// One source, parsed, refusing on a parse error.
fn parse(path: &Path) -> File {
    let src =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let (file, diags) = parse_str(&src);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "{}: parse errors {errors:?}",
        path.display()
    );
    file
}

/// The comptime expression `text`, parsed as a const initializer.
fn expr(text: &str) -> Expr {
    let (file, diags) = parse_str(&format!("module probe\nconst PROBE = {text}\n"));
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .collect();
    assert!(errors.is_empty(), "`{text}` does not parse: {errors:?}");
    file.items
        .into_iter()
        .find_map(|item| match item {
            Item::Const(d) => Some(d.value),
            _ => None,
        })
        .unwrap_or_else(|| panic!("`{text}` parsed to no const"))
}

/// `text` evaluated in the scope of `file` (the source at `rel`), with `ambient` as the
/// type environment, as an integer.
fn eval_int(file: &File, ambient: &[Item], rel: &str, text: &str) -> i64 {
    let (value, diags) = eval_expr_in_file_ambient(file, ambient, &expr(text), &[]);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .collect();
    assert!(errors.is_empty(), "`{text}` in {rel}: {errors:?}");
    let int = value
        .as_stored_int()
        .unwrap_or_else(|| panic!("`{text}` in {rel} is not an integer: {value:?}"));
    i64::try_from(int).unwrap_or_else(|_| panic!("`{text}` in {rel} is out of range: {int}"))
}

/// The equs the harvest must emit for the tree at `aeon`, derived WITHOUT running it.
///
/// Field names come from each struct's parsed declaration (`StructDecl::fields`, in
/// declaration order), where the harvest walks the layout engine's field list. Each
/// `<prefix>_<field>` is `offsetof(<Struct>, <field>)` and each `<prefix>_len` is
/// `sizeof(<Struct>)`, evaluated in the struct's own file with `types.emp` as the
/// ambient. `SST_interact` is aeon's own `interact_off()` from sst.emp, the function
/// the harvest's derived equ mirrors.
fn expected_equs(aeon: &Path) -> BTreeMap<String, i64> {
    let types = parse(&aeon.join(TYPES_REL));
    let mut want = BTreeMap::new();
    let mut put = |name: String, value: i64| {
        let prior = want.insert(name.clone(), value);
        assert!(prior.is_none(), "the derivation produced `{name}` twice");
    };
    for (rel, sname, prefix) in TWINS {
        let file = parse(&aeon.join(rel));
        let decl = file
            .items
            .iter()
            .find_map(|item| match item {
                Item::Struct(d) if d.name == *sname => Some(d),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{rel} declares no struct `{sname}`"));
        assert!(
            !decl.fields.is_empty(),
            "{rel}: struct `{sname}` declares no fields, so this derivation would compare \
             nothing but its size"
        );
        for field in &decl.fields {
            let text = format!("offsetof({sname}, {})", field.name);
            put(
                format!("{prefix}_{}", field.name),
                eval_int(&file, &types.items, rel, &text),
            );
        }
        put(
            format!("{prefix}_len"),
            eval_int(&file, &types.items, rel, &format!("sizeof({sname})")),
        );
        if *sname == "Sst" {
            put(
                "SST_interact".to_string(),
                eval_int(&file, &types.items, rel, "interact_off()"),
            );
        }
    }
    want
}

/// A harvest as a name-to-value map, refusing a name it emits twice.
fn harvest_map(harvest: Vec<(String, i64)>) -> BTreeMap<String, i64> {
    let mut map = BTreeMap::new();
    for (name, value) in harvest {
        let prior = map.insert(name.clone(), value);
        assert!(prior.is_none(), "the harvest emits `{name}` twice");
    }
    map
}

/// `got` EQUALS `want`, every difference named in both directions.
fn assert_same_equs(got: &BTreeMap<String, i64>, want: &BTreeMap<String, i64>, what: &str) {
    let missing: Vec<String> = want
        .iter()
        .filter(|(name, _)| !got.contains_key(*name))
        .map(|(name, value)| format!("{name} = {value:#x}"))
        .collect();
    let extra: Vec<String> = got
        .iter()
        .filter(|(name, _)| !want.contains_key(*name))
        .map(|(name, value)| format!("{name} = {value:#x}"))
        .collect();
    let differ: Vec<String> = want
        .iter()
        .filter_map(|(name, w)| {
            got.get(name)
                .filter(|g| *g != w)
                .map(|g| format!("{name}: harvest {g:#x}, expected {w:#x}"))
        })
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty() && differ.is_empty(),
        "{what}: expected but not emitted {missing:?}; emitted but not expected {extra:?}; \
         values differ {differ:?}"
    );
}

#[test]
fn harvest_emits_the_as_field_offsets_and_sizes() {
    let Some(aeon) = reference_tree(&sources()) else {
        return;
    };
    let got = harvest_map(harvest_engine_struct_offsets(&aeon).expect("struct-offset harvest"));
    let want = expected_equs(&aeon);
    assert_same_equs(
        &got,
        &want,
        &format!(
            "the harvest of {} vs its structs' own offsetof/sizeof",
            aeon.display()
        ),
    );
}

/// The synthetic tree [`harvest_shapes_a_fixed_struct_set_exactly`] harvests: every file
/// the harvest reads, each declaring its twin with a layout small enough to add up by
/// hand. It mixes pointers, `u16`, `u8`, a `fixed<16,16>` newtype from the ambient, an
/// array, explicit `@` offsets and a `(size: N)` record.
const FIXED_TREE: &[(&str, &str)] = &[
    ("engine/system/types.emp", "module engine.types\npub newtype Coord = fixed<16,16>\n"),
    (
        "engine/structs.emp",
        "module engine.structs\n\
         pub struct Act {\n    sec_grid_ptr: *u8,\n    grid_w: u16,\n    edge_mode: u8,\n    pad_07: u8,\n}\n\
         pub struct Sec {\n    sec_block_index: *u8,\n    sec_block_dict_len: u16,\n}\n\
         pub struct DMAEntry {\n    Reg94: u8,\n    SizeH: u8,\n}\n\
         pub struct parallax_config {\n    pcfg_band_count: u8,\n    pcfg_v_factor_bg: u8,\n    pcfg_layer_mask: u16,\n}\n\
         pub struct VdpShadow {\n    vdp_mode1: u8,\n    vdp_mode2: u8,\n}\n",
    ),
    (
        "engine/objects/sst.emp",
        "module engine.objects.sst\nuse engine.types.{Coord}\n\
         pub struct Sst (size: $0C) {\n    code_addr: u16,\n    x_pos: Coord @ $02,\n    frame_off: u16 @ $06,\n    sst_custom: [u8; 4] @ $08,\n}\n",
    ),
    (
        "engine/objects/entity_window.emp",
        "module engine.objects.entity_window\n\
         struct EntityScanState (size: $04) {\n    ess_ring_right_idx: u16 @ $00,\n    ess_obj_next_x: u16 @ $02,\n}\n",
    ),
    (
        "engine/level/parallax.emp",
        "module engine.level.parallax\n\
         pub struct band_entry {\n    band_top_plane: u16,\n    band_phase_offset: u8,\n    band_pad: u8,\n}\n",
    ),
];

#[test]
fn harvest_shapes_a_fixed_struct_set_exactly() {
    let tmp = tempfile::tempdir().expect("tempdir");
    for (rel, src) in FIXED_TREE {
        let path = tmp.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, src).unwrap();
    }
    let got =
        harvest_map(harvest_engine_struct_offsets(tmp.path()).expect("struct-offset harvest"));
    let want: BTreeMap<String, i64> = [
        ("Act_sec_grid_ptr", 0x00),
        ("Act_grid_w", 0x04),
        ("Act_edge_mode", 0x06),
        ("Act_pad_07", 0x07),
        ("Act_len", 0x08),
        ("Sec_sec_block_index", 0x00),
        ("Sec_sec_block_dict_len", 0x04),
        ("Sec_len", 0x06),
        ("DMAEntry_Reg94", 0x00),
        ("DMAEntry_SizeH", 0x01),
        ("DMAEntry_len", 0x02),
        ("parallax_config_pcfg_band_count", 0x00),
        ("parallax_config_pcfg_v_factor_bg", 0x01),
        ("parallax_config_pcfg_layer_mask", 0x02),
        ("parallax_config_len", 0x04),
        ("VDP_Shadow_vdp_mode1", 0x00),
        ("VDP_Shadow_vdp_mode2", 0x01),
        ("VDP_Shadow_len", 0x02),
        ("SST_code_addr", 0x00),
        ("SST_x_pos", 0x02),
        ("SST_frame_off", 0x06),
        ("SST_sst_custom", 0x08),
        ("SST_len", 0x0C),
        ("SST_interact", 0x0A),
        ("EntityScanState_ess_ring_right_idx", 0x00),
        ("EntityScanState_ess_obj_next_x", 0x02),
        ("EntityScanState_len", 0x04),
        ("band_entry_band_top_plane", 0x00),
        ("band_entry_band_phase_offset", 0x02),
        ("band_entry_band_pad", 0x03),
        ("band_entry_len", 0x04),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_string(), value))
    .collect();
    assert_same_equs(&got, &want, "the harvest of the fixed synthetic tree");
}

/// The harvested `<Struct>_<field>` / `<Struct>_len` equs of the reference tree, or
/// `None` when it lacks a source the harvest reads.
fn harvested() -> Option<HashMap<String, i64>> {
    let aeon = reference_tree(&sources())?;
    Some(
        harvest_engine_struct_offsets(&aeon)
            .expect("struct-offset harvest")
            .into_iter()
            .collect(),
    )
}

#[test]
fn sst_interact_is_the_record_tail_word() {
    let Some(h) = harvested() else { return };
    // The one derived equate structs.asm carried outside a struct block. frame_off
    // sits in the engine block at $2E, so the custom window is the record tail and
    // interact_off() = sizeof(Sst) - 2.
    assert_eq!(
        h["SST_interact"],
        h["SST_len"] - 2,
        "SST_interact = SST_len - 2 (the custom window's tail word)"
    );
}
