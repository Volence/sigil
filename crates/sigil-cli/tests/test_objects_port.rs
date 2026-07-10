//! Tranche 6 — the object-bank opener: the REAL `test_solid.emp` +
//! `test_particle.emp` ports, region-level byte gates.
//!
//! Compiles the ACTUAL ported files from aeon's tree —
//! `games/sonic4/objects/test_solid.emp` + `test_particle.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline, and
//! asserts each section's flattened bytes equal the reference ROM window at
//! the pinned addresses, in BOTH build shapes.
//!
//! ## What this port opens (step-0 note:
//! `notes/2026-07-10-tranche6-object-bank-design.md`)
//!
//! - **The object code bank gate class**: the modules live past `org $10000`
//!   (`ObjCodeBase`); the gate site is GAME-side (sonic4 `main.asm`'s
//!   `gameObjectBankIncludes`), a first. Bank addresses are SHAPE-INVARIANT
//!   (one region base serves plain and debug; only cross-seam engine/data
//!   targets move between shapes).
//! - **objroutine()** — the table-less dispatch fact: the routine store is a
//!   `.w` LINK-TIME immediate of a symbol DIFFERENCE
//!   (`move.w #(Main - ObjCodeBase), Sst.code_addr(a0)`), this tranche's
//!   demanded ImmLink width. Its destination is an OFFSET-0 field EA, the
//!   zero-disp collapse's demand site (asl's 4-byte `30BC` shape).
//! - **The typed SST twin** (`engine/objects/sst.emp`): both modules resolve
//!   `Sst.field(a0)` displacements off the type-only struct; its 30 drift
//!   guards read the real AS-side struct-generated `SST_*` equs through the
//!   link seam and must PASS here.
//! - **The `.emp`↔`.emp` imm32**: `move.l #ANI_PARTICLE, Sst.anim_table(a0)`
//!   consumes `particle_anims.emp`'s (already-ported) table label through the
//!   link — the tranche-4 imm32-d16 deferral's original consumer, carried by
//!   the existing `.l` ImmLink once the SST offsets fold comptime.
//!
//! ## Compile technique
//!
//! The full multi-module resolver is NOT used (its whole-program closure
//! check would demand the bare cross-seam names — `Draw_Sprite`,
//! `ObjectMove`, `AnimateSprite` — resolve in-program). Instead each object
//! module lowers as ONE synthetic `ast::File` with its `use`-dependencies'
//! items PREPENDED (the `controllers_port.rs` ambient technique):
//! `sst.emp`'s struct+guards into both, `constants.emp`'s consts+guards into
//! test_particle. The prepended guard `ensure`s ride along and are asserted
//! to PASS against the synthetic AS-side truths.
//!
//! ## Cross-seam symbols
//!
//! INBOUND equs (values): the SST_* struct-equ seam + engine constants +
//! `ObjCodeBase`. INBOUND labels at true per-shape VMAs (listing symbol
//! tables, 2026-07-10 pins): `Draw_Sprite` (plain `$2970` / debug `$2C2A`),
//! `ObjectMove` (`$2922`/`$2BDC`), `AnimateSprite` (`$2D78`/`$3032`) — all
//! abs.w width-selected, so positions are load-bearing — and `Ani_Particle`
//! (`$309DE`/`$30A46`), the imm32 value.
//!
//! OUTBOUND: the AS-side consumer shape — `dc.w TestSolid_Init-ObjCodeBase`
//! (ObjDef_Solid's `objdef` word, `data/objdefs/test_objects.asm`) and
//! `dc.w TestParticle-ObjCodeBase` (the emitters' spawn words) — assembled
//! through the AS front-end with the labels UNDEFINED in-unit, resolved by
//! the shared link against the `.emp` `pub proc` exports.
//!
//! ## Reference windows (both shapes — bank addresses shape-invariant)
//!
//! `test_solid`: `[0x10F7C..0x10F8A]` (0xE bytes).
//! `test_particle`: `[0x10F8A..0x10FDC]` (0x52 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, the reference tests SKIP green —
//! unless `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test test_objects_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// Bank geometry — SHAPE-INVARIANT (2026-07-10 pins, both listings agree).
const SOLID_BASE: u32 = 0x10F7C;
const SOLID_LEN: usize = 0xE;
const PARTICLE_BASE: u32 = 0x10F8A;
const PARTICLE_LEN: usize = 0x52;
const OBJ_CODE_BASE: u32 = 0x10000;

/// Per-shape TRUE VMAs of the cross-seam targets (listing symbol tables).
struct Shape {
    draw_sprite: u32,
    object_move: u32,
    animate_sprite: u32,
    ani_particle: u32,
}

const PLAIN: Shape =
    Shape { draw_sprite: 0x2970, object_move: 0x2922, animate_sprite: 0x2D78, ani_particle: 0x309DE };
const DEBUG: Shape =
    Shape { draw_sprite: 0x2C2A, object_move: 0x2BDC, animate_sprite: 0x3032, ani_particle: 0x30A46 };

/// Parse one `.emp` file to an AST, failing loudly on parse errors.
fn parse_file(path: &std::path::Path) -> sigil_frontend_emp::ast::File {
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

/// One synthetic file: `deps`' items prepended to `main`'s own, under
/// `main`'s module header (the ambient-injection technique).
fn with_ambient(
    deps: Vec<sigil_frontend_emp::ast::File>,
    main: sigil_frontend_emp::ast::File,
) -> sigil_frontend_emp::ast::File {
    let mut items = Vec::new();
    for d in deps {
        items.extend(d.items);
    }
    items.extend(main.items);
    sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    }
}

/// The AS-side value seam: the SST struct-generated equs (structs.asm), the
/// engine constants both `.emp` twins guard against, and `ObjCodeBase`. A
/// trailing label+`dc.w` opens a section so the equs flush (the
/// collision_lookup pattern).
fn as_constant_equs() -> Vec<Section> {
    let asm = "cpu 68000\n\
               SST_code_addr = $00\n\
               SST_x_pos = $02\n\
               SST_y_pos = $06\n\
               SST_x_vel = $0A\n\
               SST_y_vel = $0C\n\
               SST_render_flags = $0E\n\
               SST_collision_resp = $0F\n\
               SST_mappings = $10\n\
               SST_art_tile = $14\n\
               SST_width_pixels = $16\n\
               SST_height_pixels = $17\n\
               SST_anim = $18\n\
               SST_subtype = $19\n\
               SST_anim_table = $1A\n\
               SST_status = $1E\n\
               SST_angle = $1F\n\
               SST_prev_anim = $20\n\
               SST_anim_frame = $21\n\
               SST_anim_timer = $22\n\
               SST_mapping_frame = $23\n\
               SST_prev_frame = $24\n\
               SST_sprite_piece_count = $25\n\
               SST_parent_ptr = $26\n\
               SST_sibling_ptr = $28\n\
               SST_slot_tag = $2A\n\
               SST_entity_section_id = $2B\n\
               SST_entity_list_index = $2C\n\
               SST_layer = $2D\n\
               SST_sst_custom = $2E\n\
               SST_len = $50\n\
               BUTTON_UP = 1\n\
               BUTTON_DOWN = 2\n\
               BUTTON_LEFT = 4\n\
               BUTTON_RIGHT = 8\n\
               HW_PORT_1_DATA = $A10003\n\
               HW_PORT_2_DATA = $A10005\n\
               CTYPE_AIR = 0\n\
               VDP_Shadow_len = 19\n\
               RF_COORDMODE = 3\n\
               RF_PRIORITY_SHIFT = 5\n\
               AF_DELETE = $FB\n\
               ObjCodeBase = $10000\n\
               Stub:\n\
               \tdc.w 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (constant equs): {d:?}")).sections
}

/// One synthetic AS-side label phased at `vma` (carrier LMA harness-private,
/// set by the caller).
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// The AS-side OUTBOUND consumer — the REAL reference shapes: ObjDef_Solid's
/// `objdef` emits `dc.w objroutine(TestSolid_Init)` and the emitters emit
/// `dc.w objroutine(TestParticle)`, both `sym - ObjCodeBase` word differences
/// with the sym UNDEFINED in the AS unit (the `.emp` owns it). If the `pub
/// proc` labels don't surface as bare link symbols, or the AS front-end can't
/// defer the difference, this fails at link.
fn as_outbound_consumer() -> Vec<Section> {
    // ObjCodeBase deliberately NOT defined here — BOTH leaves defer, and the
    // link supplies them (the equ blob exports ObjCodeBase; the `.emp`
    // modules export the pub proc labels).
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tdc.w TestSolid_Init-ObjCodeBase\n\
               \tdc.w TestParticle-ObjCodeBase\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// Both regions sized exactly; sections carry only their emitted bytes.
fn map_toml() -> String {
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
         name = \"test_solid\"\n\
         lma_base = {SOLID_BASE:#x}\n\
         size = {SOLID_LEN:#x}\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"test_particle\"\n\
         lma_base = {PARTICLE_BASE:#x}\n\
         size = {PARTICLE_LEN:#x}\n\
         kind = \"rom\"\n"
    )
}

/// Compile BOTH real object modules with their ambient dependencies, place
/// them at the bank addresses, append the synthetic cross-seam sections, and
/// link. Returns (resolved sections, linked image, all captured link asserts).
fn compile_real_files(
    shape: &Shape,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = || parse_file(&aeon.join("engine/system/constants.emp"));
    let solid = parse_file(&aeon.join("games/sonic4/objects/test_solid.emp"));
    let particle = parse_file(&aeon.join("games/sonic4/objects/test_particle.emp"));

    let solid_file = with_ambient(vec![sst()], solid);
    let particle_file = with_ambient(vec![sst(), constants()], particle);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/objects")),
        embed_base: None,
        defines: vec![],
    };
    let mut sections = Vec::new();
    let mut link_asserts = Vec::new();
    for (file, what) in [(solid_file, "test_solid"), (particle_file, "test_particle")] {
        let (module, ldiags) = lower_module(&file, &opts);
        assert!(
            ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
            "{what} lower errors: {ldiags:?}"
        );
        sections.extend(module.sections);
        link_asserts.extend(module.link_asserts);
    }

    let map = sigil_link::load_map(&map_toml()).expect("map must load");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    // Synthetic cross-seam sections at harness-private LMAs, clear of the
    // bank regions.
    let mut lma = 0x0100_0000u32;
    let mut synth = as_constant_equs();
    for group in [
        &mut synth,
        &mut as_label_at("Draw_Sprite", shape.draw_sprite),
        &mut as_label_at("ObjectMove", shape.object_move),
        &mut as_label_at("AnimateSprite", shape.animate_sprite),
        &mut as_label_at("Ani_Particle", shape.ani_particle),
        &mut as_outbound_consumer(),
    ] {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.append(group);
        lma += 0x10_0000;
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// All prepended drift guards must be captured and PASS against the
/// synthetic AS-side truths: sst.emp's 30 ride with BOTH modules (60) plus
/// constants.emp's 11 with test_particle = 71.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    // The four pub proc labels each carry an always-recorded
    // `[layout.odd-item]` even-address parity assert — not drift guards;
    // exclude them from the count (they still ride the check below).
    let guards = link_asserts
        .iter()
        .filter(|a| {
            !a.message.iter().any(|p| {
                matches!(p, sigil_ir::assert::MsgPart::Text(t) if t.contains("[layout.odd-item]"))
            })
        })
        .count();
    assert_eq!(guards, 71, "the ambient drift guards must all be captured");
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );
}

/// On mismatch, report the first differing offset plus context on each side.
fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch — candidate {} bytes, expected {} bytes\n  candidate: {candidate:02x?}\n  expected:  {expected:02x?}",
        candidate.len(),
        expected.len()
    );
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(candidate.len());
        panic!(
            "{what}: first diff at offset {i:#x} (region-relative)\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

/// Both regions' reference gate + the outbound objroutine-word proofs +
/// the drift guards, shared body.
fn reference_gate(shape: &Shape, rom_name: &str) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_files(shape);
    assert_drift_guards(&resolved, &link_asserts);

    for (name, base, len) in [
        ("test_solid", SOLID_BASE as usize, SOLID_LEN),
        ("test_particle", PARTICLE_BASE as usize, PARTICLE_LEN),
    ] {
        let section = linked
            .section(name)
            .unwrap_or_else(|| panic!("linked image must carry {name}"));
        assert_region_matches(
            &section.bytes,
            &refrom[base..base + len],
            &format!("{name} vs {rom_name}[{base:#x}..{:#x}]", base + len),
        );
    }

    // Outbound proof: the AS-side objroutine words resolve to the bank
    // offsets of the `.emp`-owned pub labels.
    // The consumer is the SIXTH synthetic group: 0x0100_0000 + 5 × 0x10_0000.
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0150_0000)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let solid_word = u16::from_be_bytes([consumer.bytes[0], consumer.bytes[1]]);
    let particle_word = u16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    assert_eq!(
        solid_word,
        (SOLID_BASE - OBJ_CODE_BASE) as u16,
        "objdef's `dc.w objroutine(TestSolid_Init)` must resolve to the bank offset"
    );
    assert_eq!(
        particle_word,
        (PARTICLE_BASE - OBJ_CODE_BASE) as u16,
        "the emitters' `dc.w objroutine(TestParticle)` must resolve to the bank offset"
    );
}

/// (plain) both regions == `s4.bin` windows.
#[test]
fn test_objects_regions_match_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) both regions == `s4.debug.bin` windows.
#[test]
fn test_objects_debug_regions_match_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}
