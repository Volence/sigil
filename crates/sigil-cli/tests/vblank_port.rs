//! Tranche 21 (file 2) — the REAL `vblank.emp` port, region-level byte gate.
//!
//! Compiles the ACTUAL ported file from aeon's tree —
//! `engine/system/vblank.emp` — through the production parse -> lower ->
//! place -> resolve -> link pipeline, and asserts the `vblank` section's
//! flattened bytes equal the reference ROM window at the pinned addresses, in
//! BOTH build shapes.
//!
//! ## What this port exercises
//!
//! - **`rte` as a proc terminator** (VBlank_Handler — the first .emp proc of
//!   the interrupt-entry class; probe-pinned in `tranche21_spelling_probes`).
//! - **The full-save IRQ contract** (`clobbers()` + movem d0-a6 round-trip,
//!   the hblank HBlankHandler convention) with a computed `jsr (a0) as
//!   VBlankHandler` dispatch inside it.
//! - **The SND_CTRL_DMA_ACTIVE flag bracket** (extern-sum equ, abs.l bare
//!   spelling) + `stop_z80`/`start_z80` template splices.
//! - **Comptime shape arms**: both canonical shapes carry
//!   `SOUND_DRIVER_ENABLED=1`; the sound-OFF arms and the `SOUND_DBG_MIRROR`
//!   nest are proven by the twin-parity arms below (full AS-side assembly at
//!   the same defines — no reference ROM exists for those shapes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, the gates SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test vblank_port
//! ```

use sigil_frontend_as::{assemble, assemble_root, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn region_base(debug: bool) -> u32 {
    if debug { pins::VBLANK.debug_base } else { pins::VBLANK.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::VBLANK.debug_len } else { pins::VBLANK.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The VALUE seam: the extern-sum equ inputs + the z80_bus template's bus
/// register (truths: engine/sound_constants.asm + engine/constants.asm).
fn value_equs() -> Vec<Section> {
    let pairs: Vec<(&str, &str)> = vec![
        ("SND_Z80_BASE", "$A00000"),
        ("SND_CTRL_DMA_ACTIVE", "$1F04"),
        ("Z80_BUS_REQUEST", "$A11100"),
    ];
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols, each a `phase`d one-byte carrier at its
/// pinned per-shape VMA.
fn addr_labels(debug: bool) -> Vec<Section> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let mut table: Vec<(&str, u32)> = vec![
        ("VBlank_Ready", pick(pins::V_BLANK_READY)),
        ("VBlank_Flag", pick(pins::V_BLANK_FLAG)),
        ("VInt_Ptr", pick(pins::V_INT_PTR)),
        ("Frame_Counter", pick(pins::FRAME_COUNTER)),
        ("Ctrl_1_Press", pick(pins::CTRL_1_PRESS)),
        ("Ctrl_1_Press_Accum", pick(pins::CTRL_1_PRESS_ACCUM)),
        ("Ctrl_2_Press", pick(pins::CTRL_2_PRESS)),
        ("Ctrl_2_Press_Accum", pick(pins::CTRL_2_PRESS_ACCUM)),
        ("DMA_Budget_Default", pick(pins::DMA_BUDGET_DEFAULT)),
        ("DMA_Budget_Remaining", pick(pins::DMA_BUDGET_REMAINING)),
        ("Flush_VDP_Shadow", pick(pins::FLUSH_VDP_SHADOW)),
        ("Enqueue_Dirty_Buffers", pick(pins::ENQUEUE_DIRTY_BUFFERS)),
        ("VInt_DrawLevel", pick(pins::V_INT_DRAW_LEVEL)),
        ("Process_DMA_Critical", pick(pins::PROCESS_DMA_CRITICAL)),
        ("Process_DMA_Important", pick(pins::PROCESS_DMA_IMPORTANT)),
        ("Process_DMA_Deferrable", pick(pins::PROCESS_DMA_DEFERRABLE)),
        ("Vscroll_Write", pick(pins::VSCROLL_WRITE)),
        ("Read_Controllers", pick(pins::READ_CONTROLLERS)),
    ];
    if debug {
        table.push(("Lag_Frame_Count", pins::LAG_FRAME_COUNT));
        table.push(("DMA_Bytes_ThisFrame", pins::DMA_BYTES_THIS_FRAME));
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

/// Lower the real `vblank.emp` (prepend the engine.z80_bus templates its
/// `use` line reads) with the given comptime shape, place at `base`.
fn lower_vblank(
    aeon: &Path,
    base: u32,
    len: usize,
    debug: bool,
    sound: bool,
    mirror: bool,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let dir = aeon.join("engine/system");
    let main = parse_file(&dir.join("vblank.emp"));
    let z80_file = parse_file(&aeon.join("engine/z80_bus.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: z80_file.items.into_iter().chain(main.items).collect(),
        docs: main.docs.clone(),
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![
            ("DEBUG".to_string(), i128::from(debug)),
            ("SOUND_DRIVER_ENABLED".to_string(), i128::from(sound)),
            ("SOUND_DBG_MIRROR".to_string(), i128::from(mirror)),
        ],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "vblank.emp lower errors: {ldiags:?}"
    );
    let map_toml = format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"vblank\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );
    (sections, module.link_asserts)
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

/// Canonical-shape gate: `.emp` region bytes vs the shipped reference ROM.
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

    let base = region_base(debug);
    let (mut sections, _asserts) = lower_vblank(&aeon, base, region_len(debug), debug, true, false);
    sections.extend(value_equs());
    sections.extend(addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));

    let expected = &refrom[base as usize..base as usize + region_len(debug)];
    let section = linked.section("vblank").expect("linked image must carry vblank");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("vblank ({shape})"));
}

#[test]
fn vblank_region_matches_reference() {
    run(false);
}

#[test]
fn vblank_debug_region_matches_reference() {
    run(true);
}

// ---------------------------------------------------------------------------
// Off-canonical shape arms — the sound-OFF fence and the SOUND_DBG_MIRROR
// nest have NO reference ROM (build.sh never ships those shapes), so the gate
// is TWIN-PARITY: the full AS-side ROM assembled at the same defines is the
// reference, the region located by its own VBlank_Handler/HBlank_Install
// labels, and every cross-seam symbol fed to the .emp side from the SAME
// AS module's label table — one self-consistent world per shape.
// ---------------------------------------------------------------------------

/// Assemble the FULL AS-side ROM with the given defines; return (module).
fn as_full_module(aeon: &Path, debug: bool, sound: bool, mirror: bool) -> sigil_ir::Module {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines: Vec<(String, i64)> = Vec::new();
    if sound {
        defines.push(("SOUND_DRIVER_ENABLED".to_string(), 1));
    }
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    if mirror {
        defines.push(("SOUND_DBG_MIRROR".to_string(), 1));
    }
    let opts = AsOptions { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts)
        .unwrap_or_else(|d| panic!("full AS assemble failed: {} diagnostics; first: {:?}", d.len(), d.first()))
}

/// Find a label's VMA in an assembled module.
fn module_symbol(module: &sigil_ir::Module, name: &str) -> u32 {
    for sec in &module.sections {
        for l in &sec.labels {
            if l.name == name {
                return sec.vma_origin().wrapping_add(l.offset);
            }
        }
    }
    panic!("symbol {name} not found in AS module");
}

/// Twin-parity gate at an off-canonical shape.
fn run_twin_parity(debug: bool, sound: bool, mirror: bool, what: &str) {
    let aeon = aeon_dir();
    if !strict_gate() && !aeon.join("s4.bin").exists() {
        eprintln!("skip: aeon tree not present");
        return;
    }

    let as_module = as_full_module(&aeon, debug, sound, mirror);
    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&as_module.sections, &stubs, true)
        .unwrap_or_else(|d| panic!("{what}: AS resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &stubs)
        .unwrap_or_else(|d| panic!("{what}: AS link failed: {d:?}"));
    let rom = sigil_link::flatten(&linked, 0x00);

    let base = module_symbol(&as_module, "VBlank_Handler");
    let end = module_symbol(&as_module, "HBlank_Install");
    let expected = &rom[base as usize..end as usize];

    // The .emp side, linked against the SAME world's symbol values.
    let (mut sections, _asserts) =
        lower_vblank(&aeon, base, (end - base) as usize, debug, sound, mirror);
    let pairs: Vec<(&str, String)> = vec![
        ("SND_Z80_BASE", "$A00000".to_string()),
        ("SND_CTRL_DMA_ACTIVE", "$1F04".to_string()),
        ("Z80_BUS_REQUEST", "$A11100".to_string()),
    ];
    let pair_refs: Vec<(&str, &str)> = pairs.iter().map(|(n, v)| (*n, v.as_str())).collect();
    sections.extend(sigil_harness::test_support::assemble_equ_pairs(&pair_refs));

    let mut names: Vec<&str> = vec![
        "VBlank_Ready",
        "VBlank_Flag",
        "VInt_Ptr",
        "Frame_Counter",
        "Ctrl_1_Press",
        "Ctrl_1_Press_Accum",
        "Ctrl_2_Press",
        "Ctrl_2_Press_Accum",
        "DMA_Budget_Default",
        "DMA_Budget_Remaining",
        "Flush_VDP_Shadow",
        "Enqueue_Dirty_Buffers",
        "VInt_DrawLevel",
        "Process_DMA_Critical",
        "Process_DMA_Important",
        "Process_DMA_Deferrable",
        "Vscroll_Write",
        "Read_Controllers",
    ];
    if debug {
        names.push("Lag_Frame_Count");
        names.push("DMA_Bytes_ThisFrame");
    }
    if debug && sound && mirror {
        names.push("Sound_DebugMirror");
    }
    for (i, name) in names.iter().enumerate() {
        let vma = module_symbol(&as_module, name);
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections;
        for mut s in secs.drain(..) {
            s.lma = 0x0200_0000 + (i as u32) * 0x1_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            sections.push(s);
        }
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("{what}: emp resolve failed: {d:?}"));
    let linked_emp = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("{what}: emp link failed: {d:?}"));
    let section = linked_emp.section("vblank").expect("linked image must carry vblank");
    assert_region_matches(&section.bytes, expected, what);
}

#[test]
fn vblank_sound_off_twin_parity_plain() {
    run_twin_parity(false, false, false, "vblank (sound-off plain)");
}

#[test]
fn vblank_sound_off_twin_parity_debug() {
    run_twin_parity(true, false, false, "vblank (sound-off debug)");
}

#[test]
fn vblank_mirror_shape_twin_parity() {
    run_twin_parity(true, true, true, "vblank (mirror shape)");
}
