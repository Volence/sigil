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

fn compile(base: u32, len: usize) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    let path = aeon.join("games/sonic4/data/collision/collision_data.emp");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read: {e}"));
    let (file, pd) = parse_str(&src);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {pd:?}");
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
