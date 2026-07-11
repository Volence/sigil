//! Tranche 4 — the REAL `sonic_anims.emp` port, region-level byte gate.
//!
//! `particle_anims_port.rs`'s big sibling: the ELEVEN-script animation
//! index, ordered by the shared `ANIM_*` ids. What it exercises beyond
//! port #1:
//!
//! - **Fully-INLINE offsets members at scale** — eleven `Name: [u8; N] =
//!   [...]` entries packed back-to-back (the step-5 rewrite dropped the AS
//!   twin's dead inter-body pads — AnimateSprite reads scripts BYTE-wise —
//!   so the construct's inline form became expressible; 0x6E bytes).
//! - **The ordinals-replace-hand-synced-constants story** — twelve drift
//!   guards prove `Ani_Sonic.Walk == ANIM_WALK` .. `.count == ANIM_COUNT`
//!   against the AS-side config equs: declaration position IS the id, and
//!   the pairing is a checked fact.
//!
//! ## Reference windows
//!
//! Plain (map base `$30970`): `s4.bin[0x30970..0x309DE]` (0x6E bytes).
//! Debug (map base `$309D8`): `s4.debug.bin[0x309D8..0x30A46]`.
//! Content is shape-invariant.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sonic_anims_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn anims_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("games/sonic4/data/animations")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

fn map_toml(debug: bool) -> String {
    let base = if debug { "0x309D8" } else { "0x30970" };
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
         name = \"sonic_anims\"\n\
         lma_base = {base}\n\
         size = 0x6E\n\
         kind = \"rom\"\n"
    )
}

/// The AS-side equs the drift guards read back through `extern()`: the
/// twelve `ANIM_*` ids (`games/sonic4/config/constants.asm` truth) plus the
/// full engine-constants twin blob — sonic_anims' three former local command
/// mirrors consolidated into the twin at the animate port (tranche 9,
/// kill-list row 3), so the twin's 30 ensures now ride this module's
/// lowering and need their truths present.
fn as_equs() -> Vec<Section> {
    let mut pairs = sigil_harness::test_support::engine_constant_equs();
    pairs.extend([
        ("ANIM_WALK", "0"),
        ("ANIM_RUN", "1"),
        ("ANIM_ROLL", "2"),
        ("ANIM_SPINDASH", "3"),
        ("ANIM_PUSH", "4"),
        ("ANIM_IDLE", "5"),
        ("ANIM_BALANCE", "6"),
        ("ANIM_LOOKUP", "7"),
        ("ANIM_DUCK", "8"),
        ("ANIM_SKID", "9"),
        ("ANIM_GETUP", "10"),
        ("ANIM_COUNT", "11"),
    ]);
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The synthetic outbound consumer — sonic.asm's real anim-table pointer
/// write shape, as a `dc.l` cell.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tdc.l   Ani_Sonic\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (consumer): {d:?}")).sections
}

fn compile_real_file(
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = anims_dir();
    let src = std::fs::read_to_string(dir.join("sonic_anims.emp"))
        .unwrap_or_else(|e| panic!("cannot read sonic_anims.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sonic_anims.emp parse errors: {pdiags:?}"
    );
    // `use engine.constants.{AF_END, AF_BACK, DUR_DYNAMIC}` (the tranche-9
    // row-3 consolidation) — plain `lower_module` never resolves cross-module
    // `use`, so the twin's items ride in front (the ambient technique).
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let csrc = std::fs::read_to_string(Path::new(&aeon).join("engine/system/constants.emp"))
        .unwrap_or_else(|e| panic!("cannot read constants.emp: {e}"));
    let (cfile, cdiags) = parse_str(&csrc);
    assert!(
        cdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "constants.emp parse errors: {cdiags:?}"
    );
    let file = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: cfile.items.into_iter().chain(file.items).collect(),
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
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = as_equs();
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

/// All twelve ordinal/count drift guards must be captured (identified by
/// their message text — the constants twin's 30 "disagree" ensures and the
/// trailing `align 2` congruence assert also ride `link_asserts`) and PASS
/// against the equs. The three former command-byte guards died in the
/// tranche-9 row-3 consolidation (the twin's own ensures cover the values).
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = link_asserts
        .iter()
        .filter(|a| {
            a.message.iter().any(|p| {
                matches!(p, sigil_ir::assert::MsgPart::Text(t) if t.contains("drifted"))
            })
        })
        .count();
    assert_eq!(guards, 12, "all twelve ordinal drift guards must be captured");
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "every link assert must PASS: {diags:?}"
    );
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
    assert_drift_guards(&resolved, &link_asserts);

    let expected = &refrom[base..base + 0x6E];
    let section = linked.section("sonic_anims").expect("linked image must carry sonic_anims");
    assert_eq!(
        section.bytes.len(),
        0x6E,
        "sonic_anims must emit exactly 0x6E bytes (table + packed bodies)"
    );
    if let Some(i) = (0..0x6E).find(|&i| section.bytes[i] != expected[i]) {
        panic!(
            "sonic_anims ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
            if debug { "debug" } else { "plain" },
            &section.bytes[i.saturating_sub(4)..(i + 8).min(0x6E)],
            &expected[i.saturating_sub(4)..(i + 8).min(0x6E)]
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
    assert_eq!(ptr as usize, base, "bare-name proof: `dc.l Ani_Sonic` must resolve to {base:#X}");
}

#[test]
fn sonic_anims_region_matches_reference() {
    gate(false, "s4.bin", 0x30970);
}

#[test]
fn sonic_anims_debug_region_matches_reference() {
    gate(true, "s4.debug.bin", 0x309D8);
}
