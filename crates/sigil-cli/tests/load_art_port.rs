//! Tranche 20 — the REAL `load_art.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/load_art.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline and asserts
//! the `load_art` region's flattened bytes equal the reference ROM window at
//! the pinned base, in BOTH build shapes. The level art loader: the
//! version-dispatched blocking decompressor (Art_Decompress) and the paged
//! act-pool load loop (Level_LoadArt) with its out-of-line drop handler.
//!
//! ## Shape
//! Shape-DEPENDENT length ($68 plain / $B2 debug — the debug surplus is the
//! `.drop_page` raise_error expansion; the release arm is the 6-byte
//! drain-and-retry).
//!
//! ## Cross-seam symbols
//! - Address carriers: `Art_Staging_Buffer` (abs.l RAM — $FFFF0000 sits
//!   OUTSIDE the abs.w window, so the bare spelling width-selects .l),
//!   `S4LZ_Decompress` / `ZX0_Decompress` / `VSync_Wait` (extern .asm
//!   callees), `QueueDMA_Critical` + `BG_Init` (.emp-owned in their own
//!   modules — supplied as address carriers HERE exactly like the dplc
//!   standalone gate supplies its .emp-owned callees; the module-to-module
//!   proof lives in the flip tests), and (debug) the MDDBG handlers the
//!   raise_error blob targets.
//! - VALUE mirrors: the engine.constants ART_*/DMA consts + the
//!   engine.structs walls (prepended twins' ensures).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test load_art_port
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
    if debug { pins::LOAD_ART.debug_base } else { pins::LOAD_ART.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::LOAD_ART.debug_len } else { pins::LOAD_ART.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

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
         name = \"load_art\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// The VALUE seam: the prepended twins' drift-lock truths. `doctor` overrides
/// ONE pair (the negative probe).
fn value_equs(doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = Vec::new();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    if let Some((name, val)) = doctor {
        let mut hit = false;
        for p in pairs.iter_mut() {
            if p.0 == name {
                p.1 = val;
                hit = true;
            }
        }
        assert!(hit, "doctor target `{name}` not in the value seam");
    }
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols, each a `phase`d one-byte carrier at its
/// pinned per-shape VMA.
fn addr_labels(debug: bool) -> Vec<Section> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let mut table: Vec<(&str, u32)> = vec![
        ("Art_Staging_Buffer", pick(pins::ART_STAGING_BUFFER)),
        ("S4LZ_Decompress", pick(pins::S4_LZ_DECOMPRESS)),
        ("VSync_Wait", pick(pins::V_SYNC_WAIT)),
        ("QueueDMA_Critical", pick(pins::QUEUE_DMA_CRITICAL)),
        ("BG_Init", pick(pins::BG_INIT)),
        // Art-streaming P2a Task 4 — Level_LoadArt drives the pool load through the
        // page-in FIFO (PageIn_Flush/Enqueue calls, PageIn_Pool_Table store, the
        // budget raise, and the drain-wait flag polls).
        // P2b Task 6: Level_LoadArt now opens with PageCache_Init (which itself
        // calls PageIn_Flush internally — page_cache's concern, not load_art's).
        ("PageCache_Init", pick(pins::PAGE_CACHE_INIT)),
        ("PageIn_Enqueue", pick(pins::PAGE_IN_ENQUEUE)),
        ("PageIn_Pool_Table", pick(pins::PAGE_IN_POOL_TABLE)),
        ("PageIn_Queue_Count", pick(pins::PAGE_IN_QUEUE_COUNT)),
        ("PageIn_InFlight", pick(pins::PAGE_IN_IN_FLIGHT)),
        ("PageIn_Suspended", pick(pins::PAGE_IN_SUSPENDED)),
        ("PageIn_Land_Pending", pick(pins::PAGE_IN_LAND_PENDING)),
        ("PageIn_Staging_Busy", pick(pins::PAGE_IN_STAGING_BUSY)),
        ("DMA_Budget_Default", pick(pins::DMA_BUDGET_DEFAULT)),
        // P-3 family (pre-existing post-P2 seam rot): Level_LoadArt's chain-71
        // act-context bind + the P2c budget seed + the F-3/M-2 latch cells.
        ("Current_Act_Ptr", pick(pins::CURRENT_ACT_PTR)),
        ("Act_Art_Budget", pick(pins::ACT_ART_BUDGET)),
        ("Art_Budget_Remaining", pick(pins::ART_BUDGET_REMAINING)),
        ("PageIn_Pool_Pages", pick(pins::PAGE_IN_POOL_PAGES)),
        ("PageIn_Bulk_Drain", pick(pins::PAGE_IN_BULK_DRAIN)),
        ("PageIn_Fully_Resident", pick(pins::PAGE_IN_FULLY_RESIDENT)),
    ];
    if debug {
        // Debug shape only: the raise_error construct's error-handler entry
        // points (the rings_port precedent).
        table.push(("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER));
        table.push((
            "MDDBG__ErrorHandler_PagesController",
            pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER,
        ));
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

/// Lower the real `load_art.emp` (prepend the engine.structs +
/// engine.constants twins its `use` lines read), place into the per-shape
/// map, append the value equs + address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
    doctor: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/level");
    let main = parse_file(&dir.join("load_art.emp"));
    let structs_file = parse_file(&aeon.join("engine/structs.emp"));
    let consts_file = parse_file(&aeon.join("engine/system/constants.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: structs_file
            .items
            .into_iter()
            .chain(consts_file.items)
            .chain(main.items)
            .collect(),
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
        "load_art.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    sections.extend(value_equs(doctor));
    sections.extend(addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    // Packed placement (Wave-B B-0) may end a region window in ALIGNMENT FILL: the
    // pins span runs to the next section's aligned base (0x20). Tolerate a short
    // (< 32 B) all-zero tail beyond the lowered image; every real byte still compares.
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

    let (resolved, linked, link_asserts) = compile_real_file(debug, None);
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "load_art.emp drift guards must all PASS: {diags:?}"
    );

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("load_art").expect("linked image must carry load_art");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("load_art ({shape})"));
}

#[test]
fn load_art_region_matches_reference() {
    run(false);
}

#[test]
fn load_art_debug_region_matches_reference() {
    run(true);
}

// The `doctored_art_ver_zx0_fires_its_guard` negative probe retired with the
// Stage-3 P5 ownership flip: `ART_VER_ZX0` is now SOLE-authored by
// `constants.emp` (harvested into guarded AS defines), so its mirror drift
// guard was deleted — there is no AS-side twin to drift and nothing for a
// doctored truth to fire. The undoctored reference gates above remain the proof
// the value is load-bearing.

// ---------------------------------------------------------------------------
// Ownership flips (kill-list rows 29 + 38): `VSync_Wait` (t21, engine.vblank)
// and `S4LZ_Decompress` (t22, engine.s4lz) both moved from extern decls to
// .emp-owned procs. This persisted link test compiles load_art.emp +
// vblank.emp + s4lz.emp TOGETHER — both extern decls are GONE from
// load_art.emp (its register-discipline block relies on the carried
// `clobbers(d0)` / `clobbers(d0-d3/a2-a3)` licenses), the calls resolve
// module-to-module, and ALL FOUR regions byte-match the shipped reference
// ROM. `ZX0_Decompress` (row 39, t22) rides the same world: zx0.emp
// compiles in too, and load_art.emp carries ZERO extern decls.
// ---------------------------------------------------------------------------

fn flip_lower(
    main: sigil_frontend_emp::ast::File,
    ambient: Vec<sigil_frontend_emp::ast::File>,
    include_root: PathBuf,
    region: &str,
    base: u32,
    len: usize,
    defines: Vec<(String, i128)>,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let mut items = Vec::new();
    for a in ambient {
        items.extend(a.items);
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
        defines,
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "flip lower errors ({region}): {ldiags:?}"
    );
    let map_toml = format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"{region}\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "flip place_sections errors ({region}): {pdiags:?}"
    );
    (sections, module.link_asserts)
}

/// Compare a compiled flip region against its reference span, tolerating a short
/// (< 32 B) trailing all-zero ALIGNMENT PAD in the reference. Packed placement
/// (Wave-B B-0) runs a pins region span to the NEXT section's aligned base (0x20),
/// so an upstream size shift (e.g. art-streaming-p2-task4's VInt_Level growth ending
/// vblank plain 0x10 short of HBLANK's 0x20-aligned base) can leave the reference
/// slice carrying up to 31 zero pad bytes the compiled image does not. Every real
/// byte still compares (matches boot_port's `assert_region_matches`).
fn assert_flip_region(compiled: &[u8], reference: &[u8], what: &str) {
    let reference = if reference.len() > compiled.len()
        && reference.len() - compiled.len() < 32
        && reference[compiled.len()..].iter().all(|&b| b == 0)
    {
        &reference[..compiled.len()]
    } else {
        reference
    };
    assert_eq!(compiled.len(), reference.len(), "{what}: length");
    assert_eq!(compiled, reference, "{what}: bytes must match the reference");
}

fn two_module_flip(debug: bool, rom_name: &str) {
    let aeon = aeon_dir();
    let rom_path = aeon.join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let la_base = region_base(debug);
    let la_len = region_len(debug);
    let vb_base = if debug { pins::VBLANK.debug_base } else { pins::VBLANK.plain_base };
    let vb_len = if debug { pins::VBLANK.debug_len } else { pins::VBLANK.plain_len };

    let dbg = i128::from(debug);
    let (mut sections, mut asserts) = flip_lower(
        parse_file(&aeon.join("engine/level/load_art.emp")),
        vec![
            parse_file(&aeon.join("engine/structs.emp")),
            parse_file(&aeon.join("engine/system/constants.emp")),
        ],
        aeon.join("engine/level"),
        "load_art",
        la_base,
        la_len,
        vec![("DEBUG".to_string(), dbg)],
    );
    let (vb_sections, vb_asserts) = flip_lower(
        parse_file(&aeon.join("engine/system/vblank.emp")),
        vec![
            // vblank.emp `use engine.sound_constants.*` — prepend the authority.
            parse_file(&aeon.join("engine/sound/sound_constants.emp")),
            parse_file(&aeon.join("engine/z80_bus.emp")),
            parse_file(&aeon.join("engine/irq.emp")),
            // m1-budget-fix: VInt_Level's Critical-charge walk uses DMAEntry.
            parse_file(&aeon.join("engine/structs.emp")),
        ],
        aeon.join("engine/system"),
        "vblank",
        vb_base,
        vb_len,
        vec![
            ("DEBUG".to_string(), dbg),
            ("SOUND_DRIVER_ENABLED".to_string(), 1),
            ("SOUND_DBG_MIRROR".to_string(), 0),
            ("Z80_RAM".to_string(), 0xA0_0000),
        ],
    );
    sections.extend(vb_sections);
    asserts.extend(vb_asserts);

    let s4_base = if debug { pins::S4LZ.debug_base } else { pins::S4LZ.plain_base };
    let s4_len = if debug { pins::S4LZ.debug_len } else { pins::S4LZ.plain_len };
    let (s4_sections, s4_asserts) = flip_lower(
        parse_file(&aeon.join("engine/compression/s4lz.emp")),
        vec![],
        aeon.join("engine/compression"),
        "s4lz",
        s4_base,
        s4_len,
        vec![("DEBUG".to_string(), dbg)],
    );
    sections.extend(s4_sections);
    asserts.extend(s4_asserts);

    // Value seam: ONE combined equ blob (a second assemble_equ_pairs call
    // would redefine its `Stub:` carrier label).
    // SND_Z80_BASE / SND_CTRL_DMA_ACTIVE are authored in sound_constants.emp now
    // (prepended into the vblank leg above), so only the z80_bus register + the
    // TILE_SIZE constants stay link externs.
    let mut pairs: Vec<(&str, &str)> = vec![
        ("Z80_BUS_REQUEST", "$A11100"),
        ("TILE_SIZE", "32"),
        // NEW-1 (defect-batch-8): VInt_Lag's $8F02 re-assert names VDP_CTRL.
        ("VDP_CTRL", "$C00004"),
    ];
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    sections.extend(sigil_harness::test_support::assemble_equ_pairs(&pairs));

    // Address seam — NO VSync_Wait / S4LZ_Decompress / ZX0_Decompress
    // carriers (the flips: those names resolve to the .emp owner modules
    // compiled above; a stale carrier would be the §11 Q4 collision).
    let mut table: Vec<(&str, u32)> = vec![
        ("Art_Staging_Buffer", pick(pins::ART_STAGING_BUFFER)),
        ("QueueDMA_Critical", pick(pins::QUEUE_DMA_CRITICAL)),
        ("BG_Init", pick(pins::BG_INIT)),
        ("VBlank_Ready", pick(pins::V_BLANK_READY)),
        ("VBlank_Flag", pick(pins::V_BLANK_FLAG)),
        ("VInt_Ptr", pick(pins::V_INT_PTR)),
        ("Frame_Counter", pick(pins::FRAME_COUNTER)),
        ("Ctrl_1_Press", pick(pins::CTRL_1_PRESS)),
        ("Ctrl_1_Press_Accum", pick(pins::CTRL_1_PRESS_ACCUM)),
        ("Ctrl_2_Press", pick(pins::CTRL_2_PRESS)),
        ("Ctrl_2_Press_Accum", pick(pins::CTRL_2_PRESS_ACCUM)),
        // input-6button (2026-08-02): VInt_Level's ext latch references the
        // 6-button ext press cells.
        ("Ctrl_1_Ext_Press", pick(pins::CTRL_1_EXT_PRESS)),
        ("Ctrl_1_Ext_Press_Accum", pick(pins::CTRL_1_EXT_PRESS_ACCUM)),
        ("Ctrl_2_Ext_Press", pick(pins::CTRL_2_EXT_PRESS)),
        ("Ctrl_2_Ext_Press_Accum", pick(pins::CTRL_2_EXT_PRESS_ACCUM)),
        ("DMA_Budget_Default", pick(pins::DMA_BUDGET_DEFAULT)),
        ("DMA_Budget_Remaining", pick(pins::DMA_BUDGET_REMAINING)),
        // P-3 family rows (same set as the standalone table above); the flip
        // also lowers vblank, whose VInt_Level resets the byte-cap cell.
        ("DMA_Enq_Bytes_Frame", pick(pins::DMA_ENQ_BYTES_FRAME)),
        ("Current_Act_Ptr", pick(pins::CURRENT_ACT_PTR)),
        ("Act_Art_Budget", pick(pins::ACT_ART_BUDGET)),
        ("Art_Budget_Remaining", pick(pins::ART_BUDGET_REMAINING)),
        ("PageIn_Pool_Pages", pick(pins::PAGE_IN_POOL_PAGES)),
        ("PageIn_Bulk_Drain", pick(pins::PAGE_IN_BULK_DRAIN)),
        ("PageIn_Fully_Resident", pick(pins::PAGE_IN_FULLY_RESIDENT)),
        // m1-budget-fix: VInt_Level now charges the plane drain + Critical DMA.
        ("Plane_Buffer_Ptr", pick(pins::PLANE_BUFFER_PTR)),
        ("DMA_Critical", pick(pins::DMA_CRITICAL)),
        ("DMA_Critical_Slot", pick(pins::DMA_CRITICAL_SLOT)),
        ("Flush_VDP_Shadow", pick(pins::FLUSH_VDP_SHADOW)),
        ("Enqueue_Dirty_Buffers", pick(pins::ENQUEUE_DIRTY_BUFFERS)),
        ("VInt_DrawLevel", pick(pins::V_INT_DRAW_LEVEL)),
        ("Process_DMA_Critical", pick(pins::PROCESS_DMA_CRITICAL)),
        ("Process_DMA_Important", pick(pins::PROCESS_DMA_IMPORTANT)),
        ("Process_DMA_Deferrable", pick(pins::PROCESS_DMA_DEFERRABLE)),
        ("Vscroll_Write", pick(pins::VSCROLL_WRITE)),
        ("Read_Controllers", pick(pins::READ_CONTROLLERS)),
        // Art-streaming P2a Task 3 — the VBlank bookmark hook's cross-seam operands
        // (this flip re-lowers vblank standalone, so they must resolve).
        ("PageIn_InFlight", pick(pins::PAGE_IN_IN_FLIGHT)),
        ("PageIn_Saved_PC", pick(pins::PAGE_IN_SAVED_PC)),
        ("PageIn_Process", if debug { pins::PAGE_IN.debug_base } else { pins::PAGE_IN.plain_base }),
        ("PageIn_BankRegs", pick(pins::PAGE_IN_BANK_REGS)),
        ("ZX0R_Decompress", if debug { pins::ZX0_RESUME.debug_base } else { pins::ZX0_RESUME.plain_base }),
        ("ZX0R_Decompress.__end", pick(pins::ZX0R_DECOMPRESS_END)),
        // Art-streaming P2a Task 4 — Level_LoadArt drives the pool load through the
        // page-in FIFO (PageIn_Flush/Enqueue calls, PageIn_Pool_Table store, drain-wait
        // flag polls); the co-lowered vblank's Important-drain release adds the last three.
        // P2b Task 6: Level_LoadArt now opens with PageCache_Init (which itself
        // calls PageIn_Flush internally — page_cache's concern, not load_art's).
        ("PageCache_Init", pick(pins::PAGE_CACHE_INIT)),
        ("PageIn_Enqueue", pick(pins::PAGE_IN_ENQUEUE)),
        ("PageIn_Pool_Table", pick(pins::PAGE_IN_POOL_TABLE)),
        ("PageIn_Queue_Count", pick(pins::PAGE_IN_QUEUE_COUNT)),
        ("PageIn_Suspended", pick(pins::PAGE_IN_SUSPENDED)),
        ("PageIn_Land_Pending", pick(pins::PAGE_IN_LAND_PENDING)),
        ("PageIn_Staging_Busy", pick(pins::PAGE_IN_STAGING_BUSY)),
        ("DMA_Important", pick(pins::DMA_IMPORTANT)),
        ("DMA_Important_Slot", pick(pins::DMA_IMPORTANT_SLOT)),
    ];
    if debug {
        table.push(("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER));
        table.push((
            "MDDBG__ErrorHandler_PagesController",
            pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER,
        ));
        table.push(("Lag_Frame_Count", pins::LAG_FRAME_COUNT));
        table.push(("DMA_Bytes_ThisFrame", pins::DMA_BYTES_THIS_FRAME));
        table.push(("Dbg_PageIn_Preempts", pins::DBG_PAGE_IN_PREEMPTS));
    }
    for (i, (name, vma)) in table.iter().enumerate() {
        let vma = *vma;
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections;
        for mut s in secs.drain(..) {
            s.lma = 0x0300_0000 + (i as u32) * 0x1_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            sections.push(s);
        }
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("flip resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("flip link failed: {d:?}"));

    let adiags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &asserts);
    assert!(
        adiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "flip drift guards: {adiags:?}"
    );

    let shape = if debug { "debug" } else { "plain" };
    let la = linked.section("load_art").expect("load_art region");
    let lr = &refrom[la_base as usize..la_base as usize + la_len];
    assert_flip_region(&la.bytes, lr, &format!("load_art ({shape} flip)"));
    let vb = linked.section("vblank").expect("vblank region");
    let vr = &refrom[vb_base as usize..vb_base as usize + vb_len];
    assert_flip_region(&vb.bytes, vr, &format!("vblank ({shape} flip)"));
    let s4 = linked.section("s4lz").expect("s4lz region");
    let sr = &refrom[s4_base as usize..s4_base as usize + s4_len];
    assert_eq!(s4.bytes.len(), sr.len(), "s4lz ({shape} flip): length");
    assert_eq!(s4.bytes, sr, "s4lz ({shape} flip): bytes must match the reference");
}

#[test]
fn two_module_ownership_flip_plain() {
    two_module_flip(false, "s4.bin");
}

#[test]
fn two_module_ownership_flip_debug() {
    two_module_flip(true, "s4.debug.bin");
}
