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

use sigil_frontend_as::{assemble, Options as AsOptions};
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

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// The VALUE seam: the z80_bus template's bus register (truth:
/// engine/constants.asm). The SND_Z80_BASE / SND_CTRL_DMA_ACTIVE inputs are now
/// authored in engine/sound/sound_constants.emp (prepended in lower_vblank), so
/// they are comptime consts, not link externs.
fn value_equs() -> Vec<Section> {
    // VDP_CTRL: NEW-1 (defect-batch-8) — VInt_Lag's $8F02 re-assert names it.
    let pairs: Vec<(&str, &str)> = vec![("Z80_BUS_REQUEST", "$A11100"), ("VDP_CTRL", "$C00004")];
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
        // input-6button (2026-08-02): the ext latch in VInt_Level references the
        // 6-button ext press cells.
        ("Ctrl_1_Ext_Press", pick(pins::CTRL_1_EXT_PRESS)),
        ("Ctrl_1_Ext_Press_Accum", pick(pins::CTRL_1_EXT_PRESS_ACCUM)),
        ("Ctrl_2_Ext_Press", pick(pins::CTRL_2_EXT_PRESS)),
        ("Ctrl_2_Ext_Press_Accum", pick(pins::CTRL_2_EXT_PRESS_ACCUM)),
        // character-lens-sweep (2026-08-13): VInt_Level now PUBLISHES the held
        // bytes too, latching them once per tick from the IRQ-owned raw shadows.
        // Read_Controllers runs on every physical VBlank including lag frames, so
        // it may not write the bytes game logic reads — a lag VBlank landing
        // mid-tick would otherwise replace the tick's held input with the live pad
        // (under replay playback: $00, i.e. all held input dropped for the rest of
        // the tick). Both sides of that copy are referenced here.
        ("Ctrl_1_Held", pick(pins::CTRL_1_HELD)),
        ("Ctrl_2_Held", pick(pins::CTRL_2_HELD)),
        ("Ctrl_1_Ext_Held", pick(pins::CTRL_1_EXT_HELD)),
        ("Ctrl_2_Ext_Held", pick(pins::CTRL_2_EXT_HELD)),
        ("Ctrl_1_Held_Raw", pick(pins::CTRL_1_HELD_RAW)),
        ("Ctrl_2_Held_Raw", pick(pins::CTRL_2_HELD_RAW)),
        ("Ctrl_1_Ext_Held_Raw", pick(pins::CTRL_1_EXT_HELD_RAW)),
        ("Ctrl_2_Ext_Held_Raw", pick(pins::CTRL_2_EXT_HELD_RAW)),
        ("DMA_Budget_Default", pick(pins::DMA_BUDGET_DEFAULT)),
        ("DMA_Budget_Remaining", pick(pins::DMA_BUDGET_REMAINING)),
        // P2c Task 8 byte cap seam (P-3 family): VInt_Level resets the frame cell.
        ("DMA_Enq_Bytes_Frame", pick(pins::DMA_ENQ_BYTES_FRAME)),
        // m1-budget-fix: VInt_Level charges the plane drain + Critical DMA against
        // the window budget, so it now references these RAM cells directly.
        ("Plane_Buffer_Ptr", pick(pins::PLANE_BUFFER_PTR)),
        ("DMA_Critical", pick(pins::DMA_CRITICAL)),
        ("DMA_Critical_Slot", pick(pins::DMA_CRITICAL_SLOT)),
        // Effects P1: both VInt paths call the raster re-arm, which lives in hblank —
        // an outbound cross-seam call target from vblank's standalone re-lower.
        ("Raster_VBlank", pick(pins::RASTER_V_BLANK)),
        ("Flush_VDP_Shadow", pick(pins::FLUSH_VDP_SHADOW)),
        ("Enqueue_Dirty_Buffers", pick(pins::ENQUEUE_DIRTY_BUFFERS)),
        ("VInt_DrawLevel", pick(pins::V_INT_DRAW_LEVEL)),
        ("Process_DMA_Critical", pick(pins::PROCESS_DMA_CRITICAL)),
        ("Process_DMA_Important", pick(pins::PROCESS_DMA_IMPORTANT)),
        ("Process_DMA_Deferrable", pick(pins::PROCESS_DMA_DEFERRABLE)),
        ("Vscroll_Write", pick(pins::VSCROLL_WRITE)),
        ("Read_Controllers", pick(pins::READ_CONTROLLERS)),
        // Art-streaming P2a Task 3 — the VBlank bookmark hook's cross-seam operands.
        ("PageIn_InFlight", pick(pins::PAGE_IN_IN_FLIGHT)),
        ("PageIn_Saved_PC", pick(pins::PAGE_IN_SAVED_PC)),
        ("PageIn_Process", if debug { pins::PAGE_IN.debug_base } else { pins::PAGE_IN.plain_base }),
        ("PageIn_BankRegs", pick(pins::PAGE_IN_BANK_REGS)),
        ("ZX0R_Decompress", if debug { pins::ZX0_RESUME.debug_base } else { pins::ZX0_RESUME.plain_base }),
        ("ZX0R_Decompress.__end", pick(pins::ZX0R_DECOMPRESS_END)),
        // Art-streaming P2a Task 4 — VInt_Level's Important-drain Staging_Busy release
        // (clears PageIn_Staging_Busy once the Important queue drains to its base).
        ("PageIn_Staging_Busy", pick(pins::PAGE_IN_STAGING_BUSY)),
        ("DMA_Important", pick(pins::DMA_IMPORTANT)),
        ("DMA_Important_Slot", pick(pins::DMA_IMPORTANT_SLOT)),
    ];
    if debug {
        table.push(("Lag_Frame_Count", pins::LAG_FRAME_COUNT));
        table.push(("DMA_Bytes_ThisFrame", pins::DMA_BYTES_THIS_FRAME));
        table.push(("Dbg_PageIn_Preempts", pins::DBG_PAGE_IN_PREEMPTS));
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
    let irq_file = parse_file(&aeon.join("engine/irq.emp"));
    // vblank.emp `use engine.sound_constants.*` for the DMA-window flag; prepend
    // the authority's items so the SND_* consts fold in this standalone lower.
    let snd_file = parse_file(&aeon.join("engine/sound/sound_constants.emp"));
    // m1-budget-fix: VInt_Level's Critical-charge walk uses sizeof(DMAEntry) +
    // DMAEntry.SizeH, so the struct decl must be in scope for this standalone lower.
    let structs_file = parse_file(&aeon.join("engine/structs.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: snd_file
            .items
            .into_iter()
            .chain(structs_file.items.into_iter().filter(|it| !matches!(it, sigil_frontend_emp::ast::Item::Use(_))))
            .chain(z80_file.items)
            .chain(irq_file.items)
            .chain(main.items)
            .collect(),
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
            // Z80_RAM (engine.constants) is the base of SND_Z80_BASE; the auto-glob
            // provides it in the real build, seeded here for the curated lower.
            ("Z80_RAM".to_string(), 0xA0_0000),
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
    // A gate over an EMPTY image proves nothing, and the tolerance below would
    // hide that: with no candidate bytes it shrinks `expected` to zero length, the
    // length assert compares 0 == 0, and the diff loop runs over an empty range —
    // so the test passes if the module emits nothing at all. Confirmed live on
    // OJZ_BG_ANIM, a 14-byte all-zero plain window (lens sweep, seat GATE, S15).
    assert!(
        !candidate.is_empty(),
        "{what}: the module emitted NO BYTES — a region gate over an empty window \
         proves nothing. Either the module stopped emitting, or this pin should not exist."
    );
    // Packed placement (Wave-B B-0) may end a region window in ALIGNMENT FILL: the
    // pins span runs to the next section's aligned base (0x20). Tolerate a short
    // (< 32 B) all-zero tail beyond the lowered image; every real byte still compares.
    // (art-streaming-p2-task4: VInt_Level's growth ends vblank plain 0x10 short of
    // HBLANK's 0x20-aligned base.)
    let expected = if expected.len() > candidate.len()
        && expected.len() - candidate.len() < 32
        && expected[candidate.len()..].iter().all(|&b| b == 0)
    {
        &expected[..candidate.len()]
    } else {
        expected
    };
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
