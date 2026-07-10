//! Tranche 3 — the REAL `collision_lookup.emp` port, region-level byte gate.
//!
//! `vdp_init_port.rs`'s sibling for the SIXTH code port: compiles the ACTUAL
//! ported file from aeon's tree — `engine/level/collision_lookup.emp` —
//! through the production parse -> lower -> place -> resolve -> link
//! pipeline, and asserts the `collision_lookup` section's flattened bytes
//! equal the reference ROM window at the pinned addresses, in BOTH build
//! shapes.
//!
//! ## No shape define
//!
//! Like every code port so far, the block's CONTENT is byte-identical plain
//! and debug (0x24 bytes in both since the step-5 tail-call optimize; 0x32
//! as first ported) — only its BASE address shifts (plain `$4C02`, debug
//! `$5426`), so the shape lives entirely in the MAP.
//!
//! ## What this port exercises that the prior five did not
//!
//! - **Cross-seam pc-relative TRANSFER** — `jbra Tile_Cache_GetCollision`
//!   (the step-5 tail call; `jbsr` + `rts` as first ported) targets an
//!   AS-side ROM label (`engine/level/tile_cache.asm`), so a PC-RELATIVE
//!   BRANCH fixup (not just a pc-rel EA) resolves against a link-supplied
//!   symbol at its true per-shape VMA (plain `$431A`, debug `$4A86`). The
//!   jsr/jmp cross-seam DEFERRAL (tranche 2) covered absolute transfers;
//!   this was the first cross-seam pc-relative CALL in a ported body.
//! - **SHAPE-DEPENDENT RAM** — the four `Cache_*` words live in GAME RAM,
//!   which moves between shapes (plain `$FFFFA834+`, debug `$FFFFA856+`) —
//!   the first port whose RAM imports shift with `__DEBUG__` (the engine-RAM
//!   ports were shape-invariant). Both spellings width to abs.w.
//!
//! ## The cross-seam symbols
//!
//! INBOUND references, supplied as synthetic AS-side sections:
//!
//! - Four AS-side RAM labels: `Cache_Left_Col`/`Cache_Head_Col`/
//!   `Cache_Top_Row`/`Cache_Bottom_Row` (`games/sonic4` game RAM, four
//!   consecutive words, phased at the per-shape base — read from each
//!   shape's symbol table).
//! - One AS-side ROM label: `Tile_Cache_GetCollision`, phased at its true
//!   per-shape VMA — the cross-seam `jbra` tail-call target described above.
//!
//! `CTYPE_AIR` comes from the `engine.constants` twin (step 2's migration —
//! `use engine.constants.{CTYPE_AIR}`; the twin lives in the SIBLING
//! `engine/system/` directory, this port being the first outside it), so the
//! twin's EIGHT drift-guard `ensure`s ride this gate: `as_twin_equs`
//! supplies all eight AS-side values and both tests `check_link_asserts`
//! them.
//!
//! OUTBOUND: `Collision_GetType` is called from
//! `games/sonic4/player/player_sensors.asm` (the sensor probe cores) — this
//! test builds a synthetic AS-side `bsr.w Collision_GetType` consumer and
//! asserts the linked fixup resolves to the correct per-shape address.
//!
//! ## Reference windows
//!
//! Plain (map base `$4BFA`): `s4.bin[0x4BFA..0x4C1E]` (0x24 bytes).
//! Debug (map base `$541E`): `s4.debug.bin[0x541E..0x5442]` (0x24 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test collision_lookup_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// The module's own directory in aeon's tree — `collision_lookup.emp` has no
/// `embed`s, but `include_root` is still set for parity with every other
/// port template. NOTE: this is the first port living under `engine/level/`
/// (the prior five were `engine/system/` or sound data).
fn collision_lookup_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("engine/level")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The engine-constants twin's guard count, derived from the shared truth list
/// (test_support) — count literals here broke on every twin growth (the
/// tranche-8 back-prop completing tranche 7's shared-list move).
fn twin_guards() -> usize {
    sigil_harness::test_support::engine_constant_equs().len()
}

/// The map: a `text` region for the zero-byte default-section carrier, and
/// the real `collision_lookup` region pinned at the per-shape reference
/// base, sized to the 0x24-byte block (plain `$4C02`, debug `$5426`).
fn map_toml(debug: bool) -> String {
    let base = if debug { "0x541E" } else { "0x4BFA" };
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
         name = \"collision_lookup\"\n\
         lma_base = {base}\n\
         size = 0x24\n\
         kind = \"rom\"\n"
    )
}

/// The values `engine.constants`'s drift guards read back through
/// `extern()` (the twin rides along via the ambient prepend). A trailing
/// label+`dc.w` opens a section so the equs flush via `pending_equ_syms`.
fn as_twin_equs() -> Vec<Section> {
    // The 20-value constants-twin blob (SOURCE OF TRUTH: `constants.asm`),
    // consolidated in `sigil_harness::test_support`.
    sigil_harness::test_support::as_engine_constants_equs()
}

/// The synthetic AS-side cross-seam unit supplying the four cache-window RAM
/// labels — four consecutive `dc.w` words phased at the per-shape base
/// (GAME RAM moves with `__DEBUG__`: plain `$FFFFA834`, debug `$FFFFA856` —
/// read from each shape's symbol table; `Head_Col` = base+2, `Top_Row` =
/// base+4, `Bottom_Row` = base+6 in both).
fn as_cache_ram_labels(debug: bool) -> Vec<Section> {
    let base = if debug { "$FFFFA856" } else { "$FFFFA834" };
    let asm = format!(
        "cpu 68000\n\
         phase {base}\n\
         Cache_Left_Col:\n\
         \tdc.w 0\n\
         Cache_Head_Col:\n\
         \tdc.w 0\n\
         Cache_Top_Row:\n\
         \tdc.w 0\n\
         Cache_Bottom_Row:\n\
         \tdc.w 0\n"
    );
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (cache ram labels): {d:?}")).sections
}

/// The synthetic AS-side cross-seam unit supplying `Tile_Cache_GetCollision`
/// at its TRUE per-shape VMA (plain `$4312`, debug `$4A7E` —
/// `engine/level/tile_cache.asm`, read from the shape's symbol table). A
/// PC-RELATIVE branch target (`bsr.w`), so the label's absolute position is
/// load-bearing: the `PcRelDisp16` fixup resolves to
/// `target_vma - (site_vma + 2)` and the reference bytes only match when
/// the label sits where the real tile_cache.asm put it.
fn as_tile_cache_label(debug: bool) -> Vec<Section> {
    let base = if debug { "$4A7E" } else { "$4312" };
    let asm = format!(
        "cpu 68000\n\
         phase {base}\n\
         Tile_Cache_GetCollision:\n\
         \tdc.b 0\n"
    );
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (tile cache label): {d:?}")).sections
}

/// The synthetic AS-side OUTBOUND consumer — THE BARE-NAME PROOF. Mirrors
/// the real sensor-core shape (`player_sensors.asm`'s
/// `bsr.w Collision_GetType`), assembled through the AS front-end exactly
/// like a real consumer would be.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tbsr.w   Collision_GetType\n\
               \trts\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// Lower the real `collision_lookup.emp` (NO defines: the block is
/// shape-invariant; the shape lives in the map AND in the synthetic RAM/ROM
/// label positions) -> place into the per-shape map -> append the THREE
/// synthetic cross-seam sections at harness-private LMAs -> ONE
/// `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = collision_lookup_dir();
    let src = std::fs::read_to_string(dir.join("collision_lookup.emp"))
        .unwrap_or_else(|e| panic!("cannot read collision_lookup.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "collision_lookup.emp parse errors: {pdiags:?}"
    );
    // `collision_lookup.emp` `use`s the `engine.constants` twin (step 2) —
    // prepend its items. The twin lives in the SIBLING `engine/system/`
    // directory (this is the first port outside it).
    let constants_path =
        dir.parent().expect("engine/level has a parent").join("system/constants.emp");
    let constants_src = std::fs::read_to_string(&constants_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", constants_path.display()));
    let (constants_file, cdiags) = parse_str(&constants_src);
    assert!(
        cdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "constants.emp parse errors: {cdiags:?}"
    );
    let file = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: constants_file.items.into_iter().chain(file.items).collect(),
        docs: file.docs.clone(),
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
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors (region-per-section): {pdiags:?}"
    );

    let mut equs = as_twin_equs();
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    let mut ram_labels = as_cache_ram_labels(debug);
    for sec in &mut ram_labels {
        sec.lma = 0x0200_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(ram_labels);

    // Tile_Cache_GetCollision is a PC-RELATIVE branch target: its section is
    // `phase`d at the true per-shape VMA, so its LABEL address is already
    // correct — the LMA of the carrier byte is harness-private.
    let mut tile_cache = as_tile_cache_label(debug);
    for sec in &mut tile_cache {
        sec.lma = 0x0280_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(tile_cache);

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

/// The twin's drift guards must be captured and PASS against
/// `as_twin_equs`' values (excluding the D2.29 `[layout.odd-item]` parity
/// asserts that also ride `module.link_asserts`).
fn assert_twin_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = sigil_harness::test_support::guard_assert_count(link_asserts);
    assert_eq!(guards, twin_guards(), "the engine.constants twin's drift guards must be captured");
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "engine.constants's drift guards must all PASS: {diags:?}"
    );
}

/// On mismatch, report the first differing offset plus 8 bytes of context on
/// each side.
fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch — candidate {} bytes, expected {} bytes",
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

/// (plain) The `collision_lookup` section's linked bytes equal
/// `s4.bin[0x4BFA..0x4C1E]`, AND the outbound `bsr.w` consumer's fixup
/// resolves to the correct per-shape address ($4C02) — the bare-name proof.
#[test]
fn collision_lookup_region_matches_reference() {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(false);
    assert_twin_guards(&resolved, &link_asserts);

    let expected = &refrom[0x4BFA..0x4C1E];
    let section =
        linked.section("collision_lookup").expect("linked image must carry collision_lookup");
    assert_region_matches(
        &section.bytes,
        expected,
        "collision_lookup (plain) vs s4.bin[0x4BFA..0x4C1E]",
    );

    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0300_0000)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let expected_disp = (0x4BFAi64 - (consumer.lma as i64 + 2)) as i16;
    assert_eq!(
        disp, expected_disp,
        "bare-name proof: `bsr.w Collision_GetType` must resolve to $4C02 (plain)"
    );
}

/// (debug) The `collision_lookup` section's linked bytes equal
/// `s4.debug.bin[0x541E..0x5442]`, AND the outbound consumer's fixup
/// resolves to the correct per-shape address ($5426).
#[test]
fn collision_lookup_debug_region_matches_reference() {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but debug reference missing: {}", rom_path.display());
        }
        eprintln!("skip: debug reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(true);
    assert_twin_guards(&resolved, &link_asserts);

    let expected = &refrom[0x541E..0x5442];
    let section =
        linked.section("collision_lookup").expect("linked image must carry collision_lookup");
    assert_region_matches(
        &section.bytes,
        expected,
        "collision_lookup (debug) vs s4.debug.bin[0x541E..0x5442]",
    );

    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0300_0000)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let expected_disp = (0x541Ei64 - (consumer.lma as i64 + 2)) as i16;
    assert_eq!(
        disp, expected_disp,
        "bare-name proof: `bsr.w Collision_GetType` must resolve to $5426 (debug)"
    );
}
