//! Tranche 4 — the REAL `act_descriptor.emp` port, region-level byte gate.
//!
//! The campaign's biggest port (the OJZ act-1 descriptor + 9-section table,
//! 0x274 bytes) and the first STRUCT-TYPED one — the Tier-1+2 act shape
//! from `docs/superpowers/notes/2026-07-10-act-descriptor-design.md`:
//!
//! - **Typed `Act`/`Sec` struct literals** — module-local struct twins,
//!   layout-pinned against the AS struct-generated `Act_len`/`Sec_len` equs
//!   (the old `* == Act_len` size guard becomes the type itself; field
//!   order drift cannot compile).
//! - **One validating constructor** (`ojz_sec`) — nine sections carry only
//!   their varying facts; the always-default fields collapse to declared
//!   Sec defaults (D2.31 named elision).
//! - **Engine invariants as comptime facts** — the per-act `if/error`
//!   blocks (grid capacity, signed-word camera clamp) fail at COMPTIME.
//! - **`extern()` in VALUE position** — `act_art_pool_pages`/`edge_mode`/
//!   the dict lengths are link-folded `Value16/8` cells (no local mirrors
//!   needed for generated/AS-owned values), and `sec_block_dict` carries
//!   the `extern(Blocks) + extern(BLOCK_INDEX_SIZE)` residual tree
//!   (S2-D13f `Cell::Expr`).
//!
//! ## The cross-seam surface
//!
//! INBOUND: 41 AS-side labels (palette/BG/parallax/pool table + the 36
//! per-section list labels) and 16 equs (pool pages, dict sizes, engine
//! limits, struct lens) — supplied as synthetic link EQUS at each shape's
//! TRUE address (Abs32 fixups bake addresses, so the positions are
//! load-bearing). OUTBOUND: `OJZ_Act1_Descriptor` (the act loader's
//! entry), proven by a `dc.l` consumer.
//!
//! ## Reference windows
//!
//! Plain (map base `$14AE6`): `s4.bin[0x14AE6..0x14D5A]` (0x274 bytes).
//! Debug (map base `$14B4E`): `s4.debug.bin[0x14B4E..0x14DC2]`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test act_descriptor_port
//! ```

use sigil_harness::pins;
use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn aeon_root() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn act_dir() -> PathBuf {
    aeon_root().join("games/sonic4/data/levels/ojz/act1")
}

fn parse_file(path: &Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {pdiags:?}",
        path.display()
    );
    file
}

/// Prepend ambient dependency modules' items to `main` (the `use`-target seam —
/// the region test's single-file lower path does not auto-resolve `use`).
fn with_ambient(
    ambient: Vec<sigil_frontend_emp::ast::File>,
    main: sigil_frontend_emp::ast::File,
) -> sigil_frontend_emp::ast::File {
    let mut items = Vec::new();
    for d in ambient {
        items.extend(d.items);
    }
    items.extend(main.items.clone());
    sigil_frontend_emp::ast::File { module: main.module, attrs: main.attrs, items, docs: main.docs }
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

// Region base/size sourced from `sigil_harness::pins` (regenerate via `repin`).
const PLAIN_BASE: usize = pins::ACT_DESCRIPTOR.plain_base as usize;
const DEBUG_BASE: usize = pins::ACT_DESCRIPTOR.debug_base as usize;
const SIZE: usize = pins::ACT_DESCRIPTOR.plain_len;

fn map_toml(debug: bool) -> String {
    let base = if debug { pins::ACT_DESCRIPTOR.debug_base } else { pins::ACT_DESCRIPTOR.plain_base };
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"act_descriptor\"\n\
         lma_base = {base:#x}\n\
         size = {SIZE:#x}\n\
         kind = \"rom\"\n"
    )
}

/// Every cross-seam symbol as a link EQU at its per-shape truth (addresses
/// re-derived from `s4.lst`/`s4.debug.lst` at the port; re-pin on
/// re-baseline). Value equs are shape-invariant.
fn as_seam_equs(debug: bool) -> Vec<Section> {
    // (name, plain, debug) — label addresses from the two symbol tables.
    const LABELS: &[(&str, u32, u32)] = &[
        ("OJZ_Palette", pins::OJZ_PALETTE.plain, pins::OJZ_PALETTE.debug),
        ("OJZ_Act1_BG_Layout", pins::OJZ_ACT1_BG_LAYOUT.plain, pins::OJZ_ACT1_BG_LAYOUT.debug),
        ("OJZ_Act1_BG_Tiles", pins::OJZ_ACT1_BG_TILES.plain, pins::OJZ_ACT1_BG_TILES.debug),
        ("ParallaxConfig_OJZ_Default", pins::PARALLAX_CONFIG_OJZ_DEFAULT.plain, pins::PARALLAX_CONFIG_OJZ_DEFAULT.debug),
        ("OJZ_Act_Pool_PageTable", pins::OJZ_ACT_POOL_PAGE_TABLE.plain, pins::OJZ_ACT_POOL_PAGE_TABLE.debug),
        ("OJZ_Sec0_Blocks", pins::OJZ_SEC0_BLOCKS.plain, pins::OJZ_SEC0_BLOCKS.debug),
        ("OJZ_Sec1_Blocks", pins::OJZ_SEC1_BLOCKS.plain, pins::OJZ_SEC1_BLOCKS.debug),
        ("OJZ_Sec2_Blocks", pins::OJZ_SEC2_BLOCKS.plain, pins::OJZ_SEC2_BLOCKS.debug),
        ("OJZ_Sec3_Blocks", pins::OJZ_SEC3_BLOCKS.plain, pins::OJZ_SEC3_BLOCKS.debug),
        ("OJZ_Sec4_Blocks", pins::OJZ_SEC4_BLOCKS.plain, pins::OJZ_SEC4_BLOCKS.debug), // content-dedup alias of Sec2
        ("OJZ_Sec5_Blocks", pins::OJZ_SEC5_BLOCKS.plain, pins::OJZ_SEC5_BLOCKS.debug),
        ("OJZ_Sec6_Blocks", pins::OJZ_SEC6_BLOCKS.plain, pins::OJZ_SEC6_BLOCKS.debug),
        ("OJZ_Sec7_Blocks", pins::OJZ_SEC7_BLOCKS.plain, pins::OJZ_SEC7_BLOCKS.debug),
        ("OJZ_Sec8_Blocks", pins::OJZ_SEC8_BLOCKS.plain, pins::OJZ_SEC8_BLOCKS.debug),
        ("OJZ_Sec0_Objects", pins::OJZ_SEC0_OBJECTS.plain, pins::OJZ_SEC0_OBJECTS.debug),
        ("OJZ_Sec0_Rings", pins::OJZ_SEC0_RINGS.plain, pins::OJZ_SEC0_RINGS.debug),
        ("OJZ_Sec0_TypeTable", pins::OJZ_SEC0_TYPE_TABLE.plain, pins::OJZ_SEC0_TYPE_TABLE.debug),
        ("OJZ_Sec1_Objects", pins::OJZ_SEC1_OBJECTS.plain, pins::OJZ_SEC1_OBJECTS.debug),
        ("OJZ_Sec1_Rings", pins::OJZ_SEC1_RINGS.plain, pins::OJZ_SEC1_RINGS.debug),
        ("OJZ_Sec1_TypeTable", pins::OJZ_SEC1_TYPE_TABLE.plain, pins::OJZ_SEC1_TYPE_TABLE.debug),
        ("OJZ_Sec2_Objects", pins::OJZ_SEC2_OBJECTS.plain, pins::OJZ_SEC2_OBJECTS.debug),
        ("OJZ_Sec2_Rings", pins::OJZ_SEC2_RINGS.plain, pins::OJZ_SEC2_RINGS.debug),
        ("OJZ_Sec2_TypeTable", pins::OJZ_SEC2_TYPE_TABLE.plain, pins::OJZ_SEC2_TYPE_TABLE.debug),
        ("OJZ_Sec3_Objects", pins::OJZ_SEC3_OBJECTS.plain, pins::OJZ_SEC3_OBJECTS.debug),
        ("OJZ_Sec3_Rings", pins::OJZ_SEC3_RINGS.plain, pins::OJZ_SEC3_RINGS.debug),
        ("OJZ_Sec3_TypeTable", pins::OJZ_SEC3_TYPE_TABLE.plain, pins::OJZ_SEC3_TYPE_TABLE.debug),
        ("OJZ_Sec4_Objects", pins::OJZ_SEC4_OBJECTS.plain, pins::OJZ_SEC4_OBJECTS.debug),
        ("OJZ_Sec4_Rings", pins::OJZ_SEC4_RINGS.plain, pins::OJZ_SEC4_RINGS.debug),
        ("OJZ_Sec4_TypeTable", pins::OJZ_SEC4_TYPE_TABLE.plain, pins::OJZ_SEC4_TYPE_TABLE.debug),
        ("OJZ_Sec5_Objects", pins::OJZ_SEC5_OBJECTS.plain, pins::OJZ_SEC5_OBJECTS.debug),
        ("OJZ_Sec5_Rings", pins::OJZ_SEC5_RINGS.plain, pins::OJZ_SEC5_RINGS.debug),
        ("OJZ_Sec5_TypeTable", pins::OJZ_SEC5_TYPE_TABLE.plain, pins::OJZ_SEC5_TYPE_TABLE.debug),
        ("OJZ_Sec6_Objects", pins::OJZ_SEC6_OBJECTS.plain, pins::OJZ_SEC6_OBJECTS.debug),
        ("OJZ_Sec6_Rings", pins::OJZ_SEC6_RINGS.plain, pins::OJZ_SEC6_RINGS.debug),
        ("OJZ_Sec6_TypeTable", pins::OJZ_SEC6_TYPE_TABLE.plain, pins::OJZ_SEC6_TYPE_TABLE.debug),
        ("OJZ_Sec7_Objects", pins::OJZ_SEC7_OBJECTS.plain, pins::OJZ_SEC7_OBJECTS.debug),
        ("OJZ_Sec7_Rings", pins::OJZ_SEC7_RINGS.plain, pins::OJZ_SEC7_RINGS.debug),
        ("OJZ_Sec7_TypeTable", pins::OJZ_SEC7_TYPE_TABLE.plain, pins::OJZ_SEC7_TYPE_TABLE.debug),
        ("OJZ_Sec8_Objects", pins::OJZ_SEC8_OBJECTS.plain, pins::OJZ_SEC8_OBJECTS.debug),
        ("OJZ_Sec8_Rings", pins::OJZ_SEC8_RINGS.plain, pins::OJZ_SEC8_RINGS.debug),
        ("OJZ_Sec8_TypeTable", pins::OJZ_SEC8_TYPE_TABLE.plain, pins::OJZ_SEC8_TYPE_TABLE.debug),
    ];
    const VALUES: &[(&str, u32)] = &[
        ("OJZ_ACT_POOL_PAGES", pins::OJZ_ACT_POOL_PAGES.plain),
        ("BLOCK_INDEX_SIZE", pins::BLOCK_INDEX_SIZE.plain),
        ("EDGE_CLAMP", pins::EDGE_CLAMP.plain),
        ("MAX_ACT_SECTIONS", pins::MAX_ACT_SECTIONS.plain),
        ("SECTION_SIZE_SHIFT", pins::SECTION_SIZE_SHIFT.plain),
        // Act_len/Sec_len + the Act_*/Sec_* field equs now come from
        // `act_sec_field_equs()` (the shared engine.structs drift wall reads them).
        ("OJZ_SEC0_BLOCK_DICT_LEN", pins::OJZ_SEC0_BLOCK_DICT_LEN.plain),
        ("OJZ_SEC1_BLOCK_DICT_LEN", pins::OJZ_SEC1_BLOCK_DICT_LEN.plain),
        ("OJZ_SEC2_BLOCK_DICT_LEN", pins::OJZ_SEC2_BLOCK_DICT_LEN.plain),
        ("OJZ_SEC3_BLOCK_DICT_LEN", pins::OJZ_SEC3_BLOCK_DICT_LEN.plain),
        ("OJZ_SEC4_BLOCK_DICT_LEN", pins::OJZ_SEC4_BLOCK_DICT_LEN.plain),
        ("OJZ_SEC5_BLOCK_DICT_LEN", pins::OJZ_SEC5_BLOCK_DICT_LEN.plain),
        ("OJZ_SEC6_BLOCK_DICT_LEN", pins::OJZ_SEC6_BLOCK_DICT_LEN.plain),
        ("OJZ_SEC7_BLOCK_DICT_LEN", pins::OJZ_SEC7_BLOCK_DICT_LEN.plain),
        ("OJZ_SEC8_BLOCK_DICT_LEN", pins::OJZ_SEC8_BLOCK_DICT_LEN.plain),
    ];
    let mut asm = String::from("cpu 68000\n");
    for (name, plain, dbg) in LABELS {
        let v = if debug { *dbg } else { *plain };
        asm.push_str(&format!("{name} = ${v:X}\n"));
    }
    for (name, v) in VALUES {
        asm.push_str(&format!("{name} = ${v:X}\n"));
    }
    // The Act_*/Sec_* field equs + Act_len/Sec_len that the prepended
    // engine.structs drift wall reads (shape-invariant offsets).
    for (name, rhs) in sigil_harness::test_support::act_sec_field_equs() {
        asm.push_str(&format!("{name} = {rhs}\n"));
    }
    asm.push_str("Stub:\n\tdc.w 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (seam equs): {d:?}")).sections
}

/// The outbound consumer — the act loader's `dc.l OJZ_Act1_Descriptor`.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tdc.l   OJZ_Act1_Descriptor\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (consumer): {d:?}")).sections
}

fn compile_real_file(
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = act_dir();
    // act_descriptor.emp `use engine.structs.{Act, Sec}` — prepend the shared
    // struct module (its layout + per-field drift wall) as the `use` target.
    let structs = parse_file(&aeon_root().join("engine/structs.emp"));
    let file = with_ambient(vec![structs], parse_file(&dir.join("act_descriptor.emp")));

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
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = as_seam_equs(debug);
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    let mut consumer = as_outbound_consumer();
    for sec in &mut consumer {
        sec.lma = 0x0300_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(consumer);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// The seven drift/invariant guards (Act_len, Sec_len, the two engine-limit
/// mirrors, the grid-capacity/clamp facts folded at comptime don't reach
/// link — only extern-bearing ones do) must be captured and PASS.
fn assert_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "every link assert must PASS: {diags:?}"
    );
    let drifted = link_asserts
        .iter()
        .filter(|a| {
            a.message.iter().any(|p| {
                matches!(p, sigil_ir::assert::MsgPart::Text(t) if t.contains("drifted"))
            })
        })
        .count();
    // act_descriptor's own 3 limit mirrors (MAX_ACT_SECTIONS/SECTION_SIZE_SHIFT/
    // EDGE_CLAMP) + the prepended engine.structs drift wall (45 per-field —
    // 34 Act/Sec + 11 DMAEntry, tranche 20 — + 3 sizeof = 48) = 51. The
    // Act_len/Sec_len/DMAEntry_len sizeof guards live in structs.emp.
    assert_eq!(drifted, 51, "act limit mirrors + shared struct drift wall must be captured");
}

fn gate(debug: bool, rom_name: &str, base: usize) {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(debug);
    assert_guards(&resolved, &link_asserts);

    let expected = &refrom[base..base + SIZE];
    let section =
        linked.section("act_descriptor").expect("linked image must carry act_descriptor");
    assert_eq!(section.bytes.len(), SIZE, "act_descriptor must emit exactly 0x274 bytes");
    if let Some(i) = (0..SIZE).find(|&i| section.bytes[i] != expected[i]) {
        panic!(
            "act_descriptor ({}) first diff at region offset {i:#x} (item {}): got {:02x?}, expected {:02x?}",
            if debug { "debug" } else { "plain" },
            if i < 0x22 { "descriptor".to_string() } else { format!("Sec{}+{:#x}", (i - 0x22) / 0x42, (i - 0x22) % 0x42) },
            &section.bytes[i.saturating_sub(4)..(i + 8).min(SIZE)],
            &expected[i.saturating_sub(4)..(i + 8).min(SIZE)]
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
    assert_eq!(
        ptr as usize, base,
        "bare-name proof: `dc.l OJZ_Act1_Descriptor` must resolve to {base:#X}"
    );
}

#[test]
fn act_descriptor_region_matches_reference() {
    gate(false, "s4.bin", PLAIN_BASE);
}

#[test]
fn act_descriptor_debug_region_matches_reference() {
    gate(true, "s4.debug.bin", DEBUG_BASE);
}
