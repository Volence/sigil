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

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn act_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("games/sonic4/data/levels/ojz/act1")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

const PLAIN_BASE: usize = 0x14AE6;
const DEBUG_BASE: usize = 0x14B4E;
const SIZE: usize = 0x274;

fn map_toml(debug: bool) -> String {
    let base = if debug { "0x14B4E" } else { "0x14AE6" };
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
         lma_base = {base}\n\
         size = 0x274\n\
         kind = \"rom\"\n"
    )
}

/// Every cross-seam symbol as a link EQU at its per-shape truth (addresses
/// re-derived from `s4.lst`/`s4.debug.lst` at the port; re-pin on
/// re-baseline). Value equs are shape-invariant.
fn as_seam_equs(debug: bool) -> Vec<Section> {
    // (name, plain, debug) — label addresses from the two symbol tables.
    const LABELS: &[(&str, u32, u32)] = &[
        ("OJZ_Palette", 0x1FDE4, 0x1FE4C),
        ("OJZ_Act1_BG_Layout", 0x1FE64, 0x1FECC),
        ("OJZ_Act1_BG_Tiles", 0x21E64, 0x21ECC),
        ("ParallaxConfig_OJZ_Default", 0x11348, 0x113B0),
        ("OJZ_Act_Pool_PageTable", 0x14ADA, 0x14B42),
        ("OJZ_Sec0_Blocks", 0x14D5A, 0x14DC2),
        ("OJZ_Sec1_Blocks", 0x1694A, 0x169B2),
        ("OJZ_Sec2_Blocks", 0x17CC6, 0x17D2E),
        ("OJZ_Sec3_Blocks", 0x1945E, 0x194C6),
        ("OJZ_Sec4_Blocks", 0x17CC6, 0x17D2E), // content-dedup alias of Sec2
        ("OJZ_Sec5_Blocks", 0x1A5AA, 0x1A612),
        ("OJZ_Sec6_Blocks", 0x1B3D0, 0x1B438),
        ("OJZ_Sec7_Blocks", 0x1CFD0, 0x1D038),
        ("OJZ_Sec8_Blocks", 0x1E244, 0x1E2AC),
        ("OJZ_Sec0_Objects", 0x11D40, 0x11DA8),
        ("OJZ_Sec0_Rings", 0x11D48, 0x11DB0),
        ("OJZ_Sec0_TypeTable", 0x11D3A, 0x11DA2),
        ("OJZ_Sec1_Objects", 0x11D72, 0x11DDA),
        ("OJZ_Sec1_Rings", 0x11D86, 0x11DEE),
        ("OJZ_Sec1_TypeTable", 0x11D68, 0x11DD0),
        ("OJZ_Sec2_Objects", 0x11DB8, 0x11E20),
        ("OJZ_Sec2_Rings", 0x11DC6, 0x11E2E),
        ("OJZ_Sec2_TypeTable", 0x11DAE, 0x11E16),
        ("OJZ_Sec3_Objects", 0x11DFC, 0x11E64),
        ("OJZ_Sec3_Rings", 0x11DFE, 0x11E66),
        ("OJZ_Sec3_TypeTable", 0x11DFA, 0x11E62),
        ("OJZ_Sec4_Objects", 0x11E04, 0x11E6C),
        ("OJZ_Sec4_Rings", 0x11E06, 0x11E6E),
        ("OJZ_Sec4_TypeTable", 0x11E02, 0x11E6A),
        ("OJZ_Sec5_Objects", 0x11E3C, 0x11EA4),
        ("OJZ_Sec5_Rings", 0x11E3E, 0x11EA6),
        ("OJZ_Sec5_TypeTable", 0x11E3A, 0x11EA2),
        ("OJZ_Sec6_Objects", 0x11E64, 0x11ECC),
        ("OJZ_Sec6_Rings", 0x11E66, 0x11ECE),
        ("OJZ_Sec6_TypeTable", 0x11E62, 0x11ECA),
        ("OJZ_Sec7_Objects", 0x11E6C, 0x11ED4),
        ("OJZ_Sec7_Rings", 0x11E6E, 0x11ED6),
        ("OJZ_Sec7_TypeTable", 0x11E6A, 0x11ED2),
        ("OJZ_Sec8_Objects", 0x11E94, 0x11EFC),
        ("OJZ_Sec8_Rings", 0x11E96, 0x11EFE),
        ("OJZ_Sec8_TypeTable", 0x11E92, 0x11EFA),
    ];
    const VALUES: &[(&str, u32)] = &[
        ("OJZ_ACT_POOL_PAGES", 3),
        ("BLOCK_INDEX_SIZE", 1024),
        ("EDGE_CLAMP", 0),
        ("MAX_ACT_SECTIONS", 48),
        ("SECTION_SIZE_SHIFT", 11),
        ("Act_len", 34),
        ("Sec_len", 66),
        ("OJZ_SEC0_BLOCK_DICT_LEN", 0),
        ("OJZ_SEC1_BLOCK_DICT_LEN", 768),
        ("OJZ_SEC2_BLOCK_DICT_LEN", 768),
        ("OJZ_SEC3_BLOCK_DICT_LEN", 768),
        ("OJZ_SEC4_BLOCK_DICT_LEN", 768),
        ("OJZ_SEC5_BLOCK_DICT_LEN", 768),
        ("OJZ_SEC6_BLOCK_DICT_LEN", 768),
        ("OJZ_SEC7_BLOCK_DICT_LEN", 768),
        ("OJZ_SEC8_BLOCK_DICT_LEN", 768),
    ];
    let mut asm = String::from("cpu 68000\n");
    for (name, plain, dbg) in LABELS {
        let v = if debug { *dbg } else { *plain };
        asm.push_str(&format!("{name} = ${v:X}\n"));
    }
    for (name, v) in VALUES {
        asm.push_str(&format!("{name} = ${v:X}\n"));
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
    let src = std::fs::read_to_string(dir.join("act_descriptor.emp"))
        .unwrap_or_else(|e| panic!("cannot read act_descriptor.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "act_descriptor.emp parse errors: {pdiags:?}"
    );

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
    assert_eq!(drifted, 5, "Act_len/Sec_len/limits/EDGE_CLAMP drift guards must be captured");
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
