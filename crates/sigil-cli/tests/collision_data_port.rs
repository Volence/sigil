//! Parcel K4 — the collision/character data island, region-level byte gate.
//!
//! The flat BINCLUDE island at the tail of `games/sonic4/main.asm`'s
//! `gameDataIncludes` (HeightMaps / HeightMapsRot / AngleTable / SolidityTable /
//! Map_Sonic / DPLC_Sonic / Art_Sonic) is now a native `.emp` `embed()` section
//! (`games.sonic4.collision_data`). The two Map_Sonic/DPLC_Sonic word-offset walls
//! became comptime `ensure`s. Boundary key HeightMaps; even-length blobs (no
//! inter-blob padding).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test collision_data_port
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use std::path::{Path, PathBuf};

fn aeon_root() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// Parse a `.emp` file, panicking on parse errors.
fn parse_file(path: &Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let (file, pd) = parse_str(&src);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "parse {}: {pd:?}", path.display());
    file
}

/// The ambient set the module's comptime WALL needs (tails-data parcel, aeon
/// 607fd121). `collision_data.emp` gained
///
///     ensure(VRAM_TEST_SONIC + dplc_peak_tiles(_dplc_sonic) <= VRAM_TEST_OBJ, ...)
///
/// which turns "Sonic's peak DPLC frame fits the character VRAM window" from a
/// stale comment into a build-time fact. Two names have to resolve for the module
/// to lower at all: the window bounds from the game config, and the comptime DPLC
/// parser from `engine.objects.dplc`.
///
/// The DPLC module is filtered to that ONE comptime fn — prepending it wholesale
/// would pull `perform_dplc` and its siblings into a compile whose bytes are being
/// compared against a ROM window. The two constants modules emit nothing, so they
/// ride whole; the engine one is needed because the same parcel gave the game
/// config its own VRAM-window ensures, which read TILE_SIZE / VRAM_SPRITE_TABLE /
/// MAX_VDP_SPRITES / VRAM_HSCROLL_TABLE from engine.constants.
fn wall_ambient(aeon: &Path) -> Vec<sigil_frontend_emp::ast::Item> {
    use sigil_frontend_emp::ast::Item;
    let mut items = parse_file(&aeon.join("engine/system/constants.emp")).items;
    items.extend(parse_file(&aeon.join("games/sonic4/config/constants.emp")).items);
    items.extend(
        parse_file(&aeon.join("engine/objects/dplc.emp"))
            .items
            .into_iter()
            .filter(|it| matches!(it, Item::ComptimeFn(d) if d.name == "dplc_peak_tiles")),
    );
    items
}

fn compile(base: u32, len: usize) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    let path = aeon.join("games/sonic4/data/collision/collision_data.emp");
    let main = parse_file(&path);
    let mut items = wall_ambient(&aeon);
    items.extend(main.items.iter().cloned());
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ld) = lower_module(&file, &opts);
    assert!(ld.iter().all(|d| d.level != sigil_span::Level::Error), "lower: {ld:?}");
    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"collision_data\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map).expect("map");
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "place: {pd:?}");
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link: {d:?}"))
}

fn gate(debug: bool, rom_name: &str) {
    let rom_path = aeon_root().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let base = if debug { pins::COLLISION_DATA.debug_base } else { pins::COLLISION_DATA.plain_base };
    let len = pins::COLLISION_DATA.plain_len;
    assert_eq!(pins::COLLISION_DATA.debug_len, len, "collision len must be shape-invariant");

    let linked = compile(base, len);
    let sec = linked.section("collision_data").expect("linked image must carry collision_data");
    assert_eq!(sec.bytes.len(), len, "collision_data must emit {len:#x} bytes");
    let expected = &refrom[base as usize..base as usize + len];
    if let Some(i) = (0..len).find(|&i| sec.bytes[i] != expected[i]) {
        panic!(
            "collision_data ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
            if debug { "debug" } else { "plain" },
            &sec.bytes[i.saturating_sub(4)..(i + 8).min(len)],
            &expected[i.saturating_sub(4)..(i + 8).min(len)]
        );
    }
    let _ = Path::new("");
}

#[test]
fn collision_data_matches_reference() {
    gate(false, "s4.bin");
}

#[test]
fn collision_data_debug_matches_reference() {
    gate(true, "s4.debug.bin");
}
