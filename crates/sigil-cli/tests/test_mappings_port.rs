//! conv-h #35 — the `test_mappings.emp` port, region-level byte gate.
//!
//! The test-object sprite mapping index: `Map_TestObj` is a §4.7 `offsets`
//! word-offset table over three frame records (two 16x16 color variants + an
//! 8x8 particle), each a typed `MapFrame1` struct (signed bbox + piece count +
//! one 8-byte `MapPiece`). What it exercises:
//!
//! - **The `offsets` table over STRUCT-typed inline bodies** — the frame index
//!   emits `dc.w Frame - Map_TestObj` per entry then the three `MapFrame1`
//!   records back-to-back (0x30 bytes: 6-byte table + 3×14-byte frames).
//! - **The `spr_size` comptime helper** — the AS `sprSize(w,h) >> 8` VDP
//!   size-nibble pack ((h-1)<<2 | (w-1)), proven by the emitted size bytes
//!   (05 for the 2x2 frames, 00 for the 1x1 particle).
//!
//! ## Reference window
//! (sourced from `sigil_harness::pins` — regenerate via repin)
//!
//! Both windows come from `pins::TEST_MAPPINGS` at run time — base and length,
//! per shape; the content is shape-invariant. The numbers are deliberately not
//! restated here: a bound copied into prose is executed by nothing, so nothing
//! can go red when it rots.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test test_mappings_port
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

const REGION_LEN: usize = pins::TEST_MAPPINGS.plain_len;

fn region_base(debug: bool) -> u32 {
    if debug { pins::TEST_MAPPINGS.debug_base } else { pins::TEST_MAPPINGS.plain_base }
}

fn aeon_root() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

fn mappings_dir() -> PathBuf {
    aeon_root().join("games/sonic4/data/mappings")
}

fn parse_file(path: &Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, diags) = parse_str(&src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {diags:?}",
        path.display()
    );
    file
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

fn map_toml(debug: bool) -> String {
    let base = region_base(debug);
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"test_mappings\"\n\
         lma_base = {base:#x}\n\
         size = {REGION_LEN:#x}\n\
         kind = \"rom\"\n"
    )
}

fn compile_real_file(debug: bool) -> sigil_link::LinkedImage {
    let dir = mappings_dir();
    let main = parse_file(&dir.join("test_mappings.emp"));

    // The port now `use`s the shared mapping-DSL module (MapPiece/MapFrame1/
    // centered). It is a self-contained types + comptime-fn module (no imports of
    // its own), so its items inline whole ahead of the port's — the standalone
    // lower then resolves the `use` locally (the g2 with_ambient pattern).
    let dsl = parse_file(&aeon_root().join("engine/objects/mapping_dsl.emp"));
    let mut items = dsl.items;
    items.extend(main.items);
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    };

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors: {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    // A synthetic outbound consumer — the real `move.l #Map_TestObj` shape as a
    // `dc.l` cell — proves the bare-name pointer resolves to the region base.
    let asm = "cpu 68000\nConsumer:\n\tdc.l   Map_TestObj\n";
    let opts_as = sigil_frontend_as::Options { initial_cpu: Some(Cpu::M68000), ..Default::default() };
    let mut consumer = sigil_frontend_as::assemble(asm, &opts_as)
        .unwrap_or_else(|d| panic!("AS assemble (consumer): {d:?}"))
        .sections;
    for sec in &mut consumer {
        sec.lma = 0x0300_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(consumer);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"))
}

fn gate(debug: bool, rom_name: &str, base: usize) {
    let aeon =
        sigil_harness::test_support::aeon_dir();
    let rom_path = Path::new(&aeon).join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let linked = compile_real_file(debug);
    let expected = &refrom[base..base + REGION_LEN];
    let section = linked.section("test_mappings").expect("linked image must carry test_mappings");
    assert_eq!(
        section.bytes.len(),
        REGION_LEN,
        "test_mappings must emit exactly {REGION_LEN:#x} bytes (table + 3 frames)"
    );
    if let Some(i) = (0..REGION_LEN).find(|&i| section.bytes[i] != expected[i]) {
        panic!(
            "test_mappings ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
            if debug { "debug" } else { "plain" },
            &section.bytes[i.saturating_sub(4)..(i + 8).min(REGION_LEN)],
            &expected[i.saturating_sub(4)..(i + 8).min(REGION_LEN)]
        );
    }

    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0300_0000)
        .expect("linked image must carry the outbound consumer");
    let ptr = u32::from_be_bytes([
        consumer.bytes[0],
        consumer.bytes[1],
        consumer.bytes[2],
        consumer.bytes[3],
    ]);
    assert_eq!(ptr as usize, base, "bare-name proof: `dc.l Map_TestObj` must resolve to {base:#X}");
}

#[test]
fn test_mappings_region_matches_reference() {
    gate(false, "s4.bin", region_base(false) as usize);
}

#[test]
fn test_mappings_debug_region_matches_reference() {
    gate(true, "s4.debug.bin", region_base(true) as usize);
}
