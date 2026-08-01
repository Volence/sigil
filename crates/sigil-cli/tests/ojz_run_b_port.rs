//! Parcel K3 run B — the OJZ act1 interior-island TAIL, region-level byte gate.
//!
//! The contiguous AS run after the native descriptor became three native `.emp`
//! sections (embed-dominated; every island even-length so the old `align 2`s are
//! no-ops):
//!
//! - **sec_block_blobs** (`games.sonic4.ojz_sec_block_blobs_act1`): 8 `embed()`d
//!   per-section block blobs + the `OJZ_Sec4_Blocks = extern("OJZ_Sec2_Blocks")`
//!   content-dedup alias (zero bytes). `OJZ_Sec0_Blocks` .. `OJZ_Palette`.
//! - **ojz_act_assets** (`games.sonic4.ojz_act_assets_act1`): the four palette/BG
//!   `embed()`s that dissolved out of act_descriptor.asm — OJZ_Palette /
//!   BGND_Palette / OJZ_Act1_BG_Layout / OJZ_Act1_BG_Tiles. `OJZ_Palette` .. `BgAnim_Table`.
//! - **ojz_bg_anim** (`games.sonic4.ojz_bg_anim_act1`): the disabled BgAnim_Table
//!   stub (`u16 = 0`) + the empty BgAnim_Banks base. `BgAnim_Table` .. `Map_TestObj`.
//!
//! Each section is byte-compared against the reference ROM at its pin base
//! (sourced from `sigil_harness::pins` — regenerate via repin). Content is
//! shape-invariant; the debug shape is the same bytes at the +0x88 offset.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test ojz_run_b_port
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins::{self, Region};
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

/// (module `.emp` path relative to the aeon tree, its section name, its pin).
const SECTIONS: &[(&str, &str, Region)] = &[
    (
        "games/sonic4/data/generated/ojz/act1/sec_block_blobs.emp",
        "sec_block_blobs",
        pins::SEC_BLOCK_BLOBS,
    ),
    (
        "games/sonic4/data/levels/ojz/act1/act_assets.emp",
        "ojz_act_assets",
        pins::OJZ_ACT_ASSETS,
    ),
    (
        "games/sonic4/data/generated/ojz/act1/bg_anim.emp",
        "ojz_bg_anim",
        pins::OJZ_BG_ANIM,
    ),
];

fn map_toml(section: &str, base: u32, len: usize) -> String {
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"{section}\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

fn compile_section(emp_rel: &str, section: &str, base: u32, len: usize) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    let path = aeon.join(emp_rel);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} parse errors: {pdiags:?}"
    );

    // embed() paths in these modules are aeon-root-relative.
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} lower errors: {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(section, base, len)).expect("map must load");
    let mut sections = module.sections;
    let place = place_sections(&mut sections, &map);
    assert!(
        place.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} place_sections errors: {place:?}"
    );

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("{emp_rel} resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("{emp_rel} link failed: {d:?}"))
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

    for (emp_rel, section, region) in SECTIONS {
        let base = if debug { region.debug_base } else { region.plain_base };
        let len = region.plain_len; // shape-invariant
        assert_eq!(region.debug_len, region.plain_len, "{section} len must be shape-invariant");

        let linked = compile_section(emp_rel, section, base, len);
        let sec = linked
            .section(section)
            .unwrap_or_else(|| panic!("linked image must carry `{section}`"));
        assert_eq!(
            sec.bytes.len(),
            len,
            "`{section}` must emit exactly {len:#x} bytes"
        );
        let expected = &refrom[base as usize..base as usize + len];
        if let Some(i) = (0..len).find(|&i| sec.bytes[i] != expected[i]) {
            panic!(
                "`{section}` ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
                if debug { "debug" } else { "plain" },
                &sec.bytes[i.saturating_sub(4)..(i + 8).min(len)],
                &expected[i.saturating_sub(4)..(i + 8).min(len)]
            );
        }
    }
}

#[test]
fn ojz_run_b_regions_match_reference() {
    gate(false, "s4.bin");
}

#[test]
fn ojz_run_b_debug_regions_match_reference() {
    gate(true, "s4.debug.bin");
}
