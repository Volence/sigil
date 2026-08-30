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
//! per-section list labels) supplied as synthetic link EQUS at each shape's
//! TRUE address (Abs32 fixups bake addresses, so the positions are
//! load-bearing) + a few engine-limit / struct-len value equs. The pool page
//! count and the per-section block-dict lengths retired from the seam at Parcel
//! K3: they are generated `.emp` const modules (ojz_act_pool_manifest.emp /
//! sec_block_dicts.emp) prepended as ambient and consumed at COMPTIME.
//! OUTBOUND: `OJZ_Act1_Descriptor` (the act loader's entry), proven by a `dc.l`
//! consumer.
//!
//! ## Reference windows
//!
//! Both windows come from `pins::ACT_DESCRIPTOR` at run time — base and length,
//! per shape. The numbers are deliberately not restated here: a bound copied
//! into prose is executed by nothing, so nothing can go red when it rots.
//! Regenerate the pins via `repin`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test act_descriptor_port
//! ```

use sigil_harness::pins;
use sigil_harness::test_support::{native_section, NativeSection, ACT_DESCRIPTOR_ASSERT_FILES};
use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn aeon_root() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
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
        // Effects P1: the two per-section gate fixtures act_descriptor now names —
        // OJZ_TestRaster into sec_raster_table, OJZ_TestPal into sec_pal. Both live
        // in parallax_configs, so they are cross-seam DATA operands here.
        ("OJZ_TestRaster", pins::OJZ_TEST_RASTER.plain, pins::OJZ_TEST_RASTER.debug),
        ("OJZ_TestPal", pins::OJZ_TEST_PAL.plain, pins::OJZ_TEST_PAL.debug),
        // Effects P2: the same class two parcels on — OJZ_TestGradient is the dense-tier
        // gate fixture act_descriptor names into section 2's sec_raster_table, and
        // OJZ_ShimmerCycle the Task 8 script it names into sec_pal_cycle. Both live in
        // parallax_configs, so both are cross-seam DATA operands here.
        ("OJZ_TestGradient", pins::OJZ_TEST_GRADIENT.plain, pins::OJZ_TEST_GRADIENT.debug),
        ("OJZ_ShimmerCycle", pins::OJZ_SHIMMER_CYCLE.plain, pins::OJZ_SHIMMER_CYCLE.debug),
        // Effects P3 (vsram parcel): OJZ_TestVsram is the plane B scroll-banding gate
        // fixture act_descriptor names into section 0's sec_raster_table — same class,
        // same home in parallax_configs.
        ("OJZ_TestVsram", pins::OJZ_TEST_VSRAM.plain, pins::OJZ_TEST_VSRAM.debug),
        // Effects ramp parcel: the OP_RUN_RAMP gate fixture, same cross-seam class.
        ("OJZ_TestRamp", pins::OJZ_TEST_RAMP.plain, pins::OJZ_TEST_RAMP.debug),
        // Effects P3 Parcel C2: each section now names ONE EffectsPreset through
        // Sec.sec_effects (total binding) instead of three per-field descriptors, so the
        // five presets are new cross-seam refs this standalone scope must supply — the
        // port-flip rule, and build.sh does not warn about it.
        ("OJZ_Preset_Sec0", pins::OJZ_PRESET_SEC0.plain, pins::OJZ_PRESET_SEC0.debug),
        ("OJZ_Preset_Sec1", pins::OJZ_PRESET_SEC1.plain, pins::OJZ_PRESET_SEC1.debug),
        ("OJZ_Preset_Sec2", pins::OJZ_PRESET_SEC2.plain, pins::OJZ_PRESET_SEC2.debug),
        ("OJZ_Preset_Sec3", pins::OJZ_PRESET_SEC3.plain, pins::OJZ_PRESET_SEC3.debug),
        ("OJZ_Preset_Plain", pins::OJZ_PRESET_PLAIN.plain, pins::OJZ_PRESET_PLAIN.debug),
        // EFFECTS-W1 item 1 step 5: section 5 left the shared Plain record and took its
        // own, so this scope must supply a seventh preset. The split is what MAKES the
        // per-section raster binding possible — the chooser is keyed on section index, and
        // threading it into a record sections 6-8 also point at would band all of them.
        ("OJZ_Preset_Sec5", pins::OJZ_PRESET_SEC5.plain, pins::OJZ_PRESET_SEC5.debug),
        // Aurora's first authored scene: Sec0's `sec_scene` binds the generated
        // `ojz_effects_editor_act1` record, one more cross-seam label the closure
        // now emits. It is the END label of the pinned SCENE_REGISTRY region
        // (`DeformTable_Zero` .. `EditorSceneBinding_OJZ_Act1_Sec0`, pins.rs), so
        // its address is that pin's base + length in each shape.
        (
            "EditorSceneBinding_OJZ_Act1_Sec0",
            pins::SCENE_REGISTRY.plain_base + pins::SCENE_REGISTRY.plain_len as u32,
            pins::SCENE_REGISTRY.debug_base + pins::SCENE_REGISTRY.debug_len as u32,
        ),
        // showcase-effects (2026-08-26, aeon 9dd52471, d-15): section 4's `sec_scene` binds
        // the SECOND generated record (EditorSceneBinding_OJZ_Act1_Sec4, 0x80 B past Sec0
        // now that band_record is 20 B) and its `sec_effects` names a sixth preset,
        // OJZ_Preset_Depth (ojz_effects.emp, after OJZ_Preset_Plain). Two new cross-seam
        // refs, the port-flip rule again: the standalone link said
        // `unresolved symbol EditorSceneBinding_OJZ_Act1_Sec4 for fixup in section
        // act_descriptor at offset 324` / `OJZ_Preset_Depth ... offset 356`. Both are
        // [[symbol]] pins (repin.toml), sourced from the resolve.
        ("EditorSceneBinding_OJZ_Act1_Sec4", pins::EDITOR_SCENE_BINDING_OJZ_ACT1_SEC4.plain, pins::EDITOR_SCENE_BINDING_OJZ_ACT1_SEC4.debug),
        ("OJZ_Preset_Depth", pins::OJZ_PRESET_DEPTH.plain, pins::OJZ_PRESET_DEPTH.debug),
        // editor-raster-preset (2026-08-29, aeon e99a2ca7): Debug_BandDemoHotkey became a
        // CYCLE, and its `.raster_table` second entry is a dc.l to
        // `EditorRaster_OJZ_Act1_authored_probe` — the first EDITOR-authored raster program,
        // lowered from games/sonic4/data/editor/effects/presets/authored_probe.json into
        // ojz_effects_editor_act1. One more cross-seam ref this standalone scope must supply;
        // the port-flip rule, and build.sh does not warn about it. The strict suite on the
        // attest is what caught it, which is where the ritual expects to catch it: the
        // diagnostic was "link assertion condition references symbol(s)
        // `EditorRaster_OJZ_Act1_authored_probe` not defined in this link". A [[symbol]] pin
        // (repin.toml), sourced from the resolve rather than a hand literal that rots at the
        // next slide — note the pin lands at 0x1323C/0x13A80, which is exactly where
        // OJZ_EFFECTS used to start, because the new program is emitted immediately ahead of
        // it and pushed that whole region +0x4E.
        ("EditorRaster_OJZ_Act1_authored_probe", pins::EDITOR_RASTER_OJZ_ACT1_AUTHORED_PROBE.plain, pins::EDITOR_RASTER_OJZ_ACT1_AUTHORED_PROBE.debug),
        // EFFECTS-W1 item 1 step 6 (2026-08-30, aeon c9a462be): section 5's sidecar binds a
        // raster preset, so the generated ojz_effects_editor_act1 (effects_scenes.emp) emits a
        // SECOND authored raster program, `EditorRaster_OJZ_Act1_ojz_sec5_showcase` (one
        // stream_cram band, rows 32..80), and OJZ_Preset_Sec5 (ojz_effects.emp) reaches it
        // through `ojz_act1_sec_raster(sec: 5, hand: Raster_Program_None)`. One more
        // cross-seam dc.l this standalone scope must supply — the port-flip rule, third
        // instance in this table, and build.sh still does not warn about it. Chain 194's
        // strict attest caught it: "link assertion condition references symbol(s)
        // `EditorRaster_OJZ_Act1_ojz_sec5_showcase` not defined in this link". A [[symbol]]
        // pin (repin.toml), sourced from the resolve — it lands 0x4E past authored_probe
        // (0x1329A/0x13ACE), the previous program's exact length, because the generator
        // emits the two programs back to back.
        ("EditorRaster_OJZ_Act1_ojz_sec5_showcase", pins::EDITOR_RASTER_OJZ_ACT1_OJZ_SEC5_SHOWCASE.plain, pins::EDITOR_RASTER_OJZ_ACT1_OJZ_SEC5_SHOWCASE.debug),
        ("OJZ_Act_Pool_PageTable", pins::OJZ_ACT_POOL_PAGE_TABLE.plain, pins::OJZ_ACT_POOL_PAGE_TABLE.debug),
        // art-streaming-p2-task5: the descriptor's Act.act_sec_local_maps field.
        ("OJZ_Sec_LocalMaps", pins::OJZ_SEC_LOCAL_MAPS.plain, pins::OJZ_SEC_LOCAL_MAPS.debug),
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
        // OJZ_ACT_POOL_PAGES + the 9 OJZ_SEC*_BLOCK_DICT_LEN retired from this seam
        // at Parcel K3 — they are generated `.emp` const modules
        // (ojz_act_pool_manifest.emp / sec_block_dicts.emp) act_descriptor.emp `use`s
        // at comptime, prepended as ambient in `compile_real_file` (not injected here).
        ("BLOCK_INDEX_SIZE", pins::BLOCK_INDEX_SIZE.plain),
        ("EDGE_CLAMP", pins::EDGE_CLAMP.plain),
        ("MAX_ACT_SECTIONS", pins::MAX_ACT_SECTIONS.plain),
        ("SECTION_SIZE_SHIFT", pins::SECTION_SIZE_SHIFT.plain),
        // POOL_TILE_CEILING is the comptime ceiling the K3 `ensure` reads — supplied
        // by the prepended engine.constants ambient (not an AS extern).
        // The prepended engine.constants ambient carries its one surviving guard
        // (VDP_Shadow_len, the struct twin); supply its extern target so it PASSES.
        ("VDP_Shadow_len", 19),
        // Act_len/Sec_len + the Act_*/Sec_* field equs now come from
        // `act_sec_field_equs()` (the shared engine.structs drift wall reads them).
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
    // The descriptor's `use` closure — engine.structs, engine.constants, the
    // generated pool-manifest / block-dict / effects_scenes modules, the scene
    // registry and everything THOSE reach (scene_dsl, parallax, the Game contract
    // bind for `Game.SCANLINE_CAPS`) — is followed by the native build itself, the
    // one path the ROM is built by. No hand-listed ambient set: a dependency the
    // descriptor grows is lowered by construction, and its absence reads as the
    // resolver's own `unknown …` diagnostic.
    let profile = sigil_harness::native::sonic4_profile(debug);
    let NativeSection { section, link_asserts } =
        native_section(&aeon_root(), &profile, "act_descriptor", ACT_DESCRIPTOR_ASSERT_FILES)
            .unwrap_or_else(|e| panic!("native act_descriptor closure: {e}"));

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = vec![section];
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
    // No surviving "drifted" mirror: MAX_ACT_SECTIONS flipped to
    // `use engine.constants` at the conv-b constants-tail flip (its drift wall
    // retired — engine.constants is the sole author now), joining
    // SECTION_SIZE_SHIFT/EDGE_CLAMP (flipped at P5). The prepended engine.structs
    // per-field drift wall retired at the conv-a structs flip. The MAX_ACT_SECTIONS
    // *invariant* asserts (grid <= limit, limit*66 <= $7FFF) survive in
    // act_descriptor.emp but carry no "drifted" text.
    assert_eq!(drifted, 0, "no drifted-mirror asserts survive the constants-tail flip");
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
    assert_eq!(section.bytes.len(), SIZE, "act_descriptor must emit exactly {SIZE:#x} bytes");
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
