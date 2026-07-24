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
    let mut table: Vec<(&str, u32)> = vec![
        ("BgAnim_LastStep", if debug { pins::BG_ANIM_LAST_STEP.debug } else { pins::BG_ANIM_LAST_STEP.plain }),
        ("Frame_Counter", if debug { pins::FRAME_COUNTER.debug } else { pins::FRAME_COUNTER.plain }),
        ("Camera_X", if debug { pins::CAMERA_X.debug } else { pins::CAMERA_X.plain }),
        ("Camera_Y", if debug { pins::CAMERA_Y.debug } else { pins::CAMERA_Y.plain }),
        ("BgAnim_Table", if debug { pins::BG_ANIM_TABLE.debug } else { pins::BG_ANIM_TABLE.plain }),
        ("QueueDMA_Deferrable", if debug { pins::QUEUE_DMA_DEFERRABLE.debug } else { pins::QUEUE_DMA_DEFERRABLE.plain }),
    ];
    if debug {
        // Debug shape only: the assert construct's error-handler entry points
        // (the rings_port precedent).
        table.push(("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER));
        table.push(("MDDBG__ErrorHandler_PagesController", pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER));
    }
    let mut out = Vec::new();
    for (i, (name, vma)) in table.iter().enumerate() {
        let vma = *vma;
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

/// Lower the real `bg_anim.emp` (prepend `engine.types` — the `bganim_band`
/// record's `vram_dest: VramAddr` vocabulary), place into the per-shape map,
/// append the cross-seam address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = level_dir();
    let main = parse_file(&dir.join("bg_anim.emp"));
    let types_file = parse_file(&dir.parent().unwrap().join("system/types.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: types_file.items.into_iter().chain(main.items).collect(),
        docs: main.docs.clone(),
    };

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

// ===========================================================================
// Tranche 20 — the QueueDMA_Deferrable ownership FLIP (proof-mechanism
// feed-forward): bg_anim.emp's two `jbsr QueueDMA_Deferrable` re-resolve from
// the old .asm owner to the newly-ported engine.dma_queue module. Both modules
// compile together — NO QueueDMA_Deferrable address carrier, NO extern decl
// (deleted this tranche; kill-list row 32) — and BOTH regions must still
// byte-match the reference ROM.
// ===========================================================================

fn flip_lower_and_place(
    emp_path: &Path,
    ambient: Vec<sigil_frontend_emp::ast::File>,
    include_root: PathBuf,
    region: &str,
    base: u32,
    len: usize,
    debug: bool,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let main = parse_file(emp_path);
    let mut items = Vec::new();
    for d in ambient {
        items.extend(d.items);
    }
    items.extend(main.items);
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(include_root),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} lower errors: {ldiags:?}",
        emp_path.display()
    );
    let mt = format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"{region}\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&mt).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} place errors: {pdiags:?}",
        emp_path.display()
    );
    (sections, module.link_asserts)
}

fn flip_value_equs() -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = vec![
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        ("VRAM", "%100001"),
        ("CRAM", "%101011"),
        ("VSRAM", "%100101"),
        ("READ", "%001100"),
        ("WRITE", "%000111"),
        ("DMA", "%100111"),
    ];
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

fn two_module_flip(debug: bool, rom_name: &str) {
    let aeon = aeon_dir();
    let Ok(refrom) = std::fs::read(aeon.join(rom_name)) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {rom_name}");
        }
        eprintln!("skip: reference ROM {rom_name} missing");
        return;
    };

    let ba_base = region_base(debug);
    let ba_len = region_len(debug);
    let (mut ba_sections, ba_asserts) = flip_lower_and_place(
        &level_dir().join("bg_anim.emp"),
        vec![parse_file(&level_dir().parent().unwrap().join("system/types.emp"))],
        level_dir(),
        "bg_anim",
        ba_base,
        ba_len,
        debug,
    );

    let dq_base = if debug { pins::DMA_QUEUE.debug_base } else { pins::DMA_QUEUE.plain_base };
    let dq_len = if debug { pins::DMA_QUEUE.debug_len } else { pins::DMA_QUEUE.plain_len };
    let (mut dq_sections, dq_asserts) = flip_lower_and_place(
        &aeon.join("engine/system/dma_queue.emp"),
        vec![
            parse_file(&aeon.join("engine/structs.emp")),
            parse_file(&aeon.join("engine/system/constants.emp")),
            parse_file(&aeon.join("engine/vdp.emp")),
        ],
        aeon.join("engine/system"),
        "dma_queue",
        dq_base,
        dq_len,
        debug,
    );

    let mut sections = Vec::new();
    sections.append(&mut ba_sections);
    sections.append(&mut dq_sections);

    // bg_anim's address seam MINUS QueueDMA_Deferrable (the flip), PLUS the
    // dma_queue RAM family.
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let mut labels: Vec<(&str, u32)> = vec![
        ("BgAnim_LastStep", pick(pins::BG_ANIM_LAST_STEP)),
        ("Frame_Counter", pick(pins::FRAME_COUNTER)),
        ("Camera_X", pick(pins::CAMERA_X)),
        ("Camera_Y", pick(pins::CAMERA_Y)),
        ("BgAnim_Table", pick(pins::BG_ANIM_TABLE)),
        ("DMA_Queue", pick(pins::DMA_CRITICAL)),
        ("DMA_Critical", pick(pins::DMA_CRITICAL)),
        ("DMA_Critical_End", pick(pins::DMA_CRITICAL_END)),
        ("DMA_Important", pick(pins::DMA_IMPORTANT)),
        ("DMA_Important_End", pick(pins::DMA_IMPORTANT_END)),
        ("DMA_Deferrable", pick(pins::DMA_DEFERRABLE)),
        ("DMA_Deferrable_End", pick(pins::DMA_DEFERRABLE_END)),
        ("DMA_Critical_Slot", pick(pins::DMA_CRITICAL_SLOT)),
        ("DMA_Important_Slot", pick(pins::DMA_IMPORTANT_SLOT)),
        ("DMA_Deferrable_Slot", pick(pins::DMA_DEFERRABLE_SLOT)),
        ("DMA_Budget_Remaining", pick(pins::DMA_BUDGET_REMAINING)),
    ];
    if debug {
        labels.push(("DMA_Overflow_Count", pins::DMA_OVERFLOW_COUNT));
        labels.push(("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER));
        labels.push((
            "MDDBG__ErrorHandler_PagesController",
            pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER,
        ));
    }

    let mut lma = 0x0100_0000u32;
    let mut groups: Vec<Vec<Section>> = vec![flip_value_equs()];
    for (name, vma) in labels {
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        groups.push(
            assemble(&asm, &opts)
                .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
                .sections,
        );
    }
    for group in &mut groups {
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

    let mut all = ba_asserts;
    all.extend(dq_asserts);
    let adiags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &all);
    assert!(
        adiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "drift guards: {adiags:?}"
    );

    let shape = if debug { "debug" } else { "plain" };
    let br = &refrom[ba_base as usize..ba_base as usize + ba_len];
    assert_region_matches(
        &linked.section("bg_anim").expect("bg_anim region").bytes,
        br,
        &format!("bg_anim ({shape} flip)"),
    );
    let qr = &refrom[dq_base as usize..dq_base as usize + dq_len];
    assert_region_matches(
        &linked.section("dma_queue").expect("dma_queue region").bytes,
        qr,
        &format!("dma_queue ({shape} flip)"),
    );
}

#[test]
fn two_module_ownership_flip_plain() {
    two_module_flip(false, "s4.bin");
}

#[test]
fn two_module_ownership_flip_debug() {
    two_module_flip(true, "s4.debug.bin");
}
