//! Tranche 19 — the REAL `bg_anim.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/bg_anim.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline and asserts
//! the `bg_anim` region's flattened bytes equal the reference ROM window at
//! the pinned base, in BOTH build shapes. BgAnim (HCZ-pillar technique):
//! driver-keyed tile-band animation walking the 44-byte band records of the
//! game-emitted BgAnim_Table, queueing wrapped-pair deferrable DMAs.
//!
//! ## Shape
//! SHAPE-INVARIANT length ($A0 both shapes — bg_anim.asm has NO `__DEBUG__`
//! code, no asserts); only the base shifts.
//!
//! ## Cross-seam symbols
//! - RAM (abs.w operands): `BgAnim_LastStep`, `Frame_Counter`, `Camera_X`,
//!   `Camera_Y` — `phase`d one-byte carriers at their pinned per-shape VMAs.
//! - ROM: `BgAnim_Table` (GAME data — gameDataIncludes emits it; the header
//!   law "nothing here may conditionally assemble on its symbols" holds: the
//!   .emp reads it as a link-time address only) and `QueueDMA_Deferrable`
//!   (the dma_queue.asm enqueue — extern-proc seam, kill-list row 32 class).
//! - No value mirrors: bg_anim.emp declares no extern-locked constants, so
//!   this gate carries no doctored-truth probe (nothing to doctor — the
//!   camera/bg gates cover their mirror classes).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test bg_anim_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn region_base(debug: bool) -> u32 {
    if debug { pins::BG_ANIM.debug_base } else { pins::BG_ANIM.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::BG_ANIM.debug_len } else { pins::BG_ANIM.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn level_dir() -> PathBuf {
    aeon_dir().join("engine/level")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` carrier for the zero-byte default section, and the
/// `bg_anim` region pinned at the per-shape reference base + length.
fn map_toml(debug: bool) -> String {
    let base = region_base(debug);
    let len = region_len(debug);
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
         name = \"bg_anim\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// The cross-seam ADDRESS symbols — RAM state, the game-emitted table, and
/// the DMA-queue enqueue — each a `phase`d one-byte carrier at its pinned
/// per-shape VMA.
fn bg_anim_addr_labels(debug: bool) -> Vec<Section> {
    let table: [(&str, pins::Pin); 6] = [
        ("BgAnim_LastStep", pins::BG_ANIM_LAST_STEP),
        ("Frame_Counter", pins::FRAME_COUNTER),
        ("Camera_X", pins::CAMERA_X),
        ("Camera_Y", pins::CAMERA_Y),
        ("BgAnim_Table", pins::BG_ANIM_TABLE),
        ("QueueDMA_Deferrable", pins::QUEUE_DMA_DEFERRABLE),
    ];
    let mut out = Vec::new();
    for (i, (name, pin)) in table.iter().enumerate() {
        let vma = if debug { pin.debug } else { pin.plain };
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections;
        for mut s in secs.drain(..) {
            s.lma = 0x0200_0000 + (i as u32) * 0x1_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            out.push(s);
        }
    }
    out
}

/// Parse a .emp file, panicking on parse errors.
fn parse_file(path: &Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {pdiags:?}",
        path.display()
    );
    file
}

/// Lower the real `bg_anim.emp` (self-contained — no shared-module prepend),
/// place into the per-shape map, append the cross-seam address labels, one
/// `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = level_dir();
    let file = parse_file(&dir.join("bg_anim.emp"));

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "bg_anim.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    sections.extend(bg_anim_addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

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

fn run(debug: bool) {
    let aeon = aeon_dir();
    let rom_name = if debug { "s4.debug.bin" } else { "s4.bin" };
    let rom_path = aeon.join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(debug);
    // bg_anim.emp has no extern-locked mirrors; any guards that do exist
    // (future ensures) must pass.
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "bg_anim.emp drift guards must all PASS: {diags:?}"
    );

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("bg_anim").expect("linked image must carry bg_anim");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("bg_anim ({shape})"));
}

#[test]
fn bg_anim_region_matches_reference() {
    run(false);
}

#[test]
fn bg_anim_debug_region_matches_reference() {
    run(true);
}
