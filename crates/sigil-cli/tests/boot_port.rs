//! Tranche 23 — the REAL `boot.emp` port, region-level byte gate.
//!
//! Compiles the ACTUAL ported file from aeon's tree —
//! `engine/system/boot.emp` — through the production parse -> lower ->
//! place -> resolve -> link pipeline, and asserts the `boot` section's
//! flattened bytes equal the reference ROM window at the pinned addresses,
//! in BOTH build shapes.
//!
//! ## What this port exercises
//!
//! - **The FIRST engine region** ([EntryPoint $200, BootData)) — everything
//!   downstream slides with any boot byte-change (gap-ledger row 1257 bar).
//! - **Forward cross-seam pc-rel** — `lea BootData(pc), a5` targets the
//!   .asm data tail immediately AFTER the region (probe-pinned in
//!   `tranche23_spelling_probes::forward_pcrel_lea_into_adjacent_window`).
//! - **The (a5)+ cursor protocol** over boot_data.asm's table (its geometry
//!   is locked AS-side by the assert wall in that file).
//! - **Link-time value immediates**: `#Z80_SOUND_SIZE-1` (imm16 arithmetic,
//!   probe-pinned) and `#GAME_ENTRY_ID` (the tranche-23 demanded `.b`
//!   deferral, `Value8` at the ext word's low byte) — both PARSED from the
//!   live listing here, never hardcoded (they float with the Z80 driver /
//!   game config).
//! - **imm32 symbol stores** `#VInt_Level`/`#Game_Entry` with explicit
//!   `(Sym).w` dests (row-109/row-1046 class).
//! - **Comptime shape arms**: both canonical shapes carry
//!   `SOUND_DRIVER_ENABLED=1`, `SOUND_DEBUG_HOTKEYS=0`; the debug shape adds
//!   the `bsr.w CompressionSelfTest` (+4). The sound-OFF arm
//!   (`moveq #Z80_IDLE_SIZE-1`) is BLOCKED pending the P3 moveq-link-imm8
//!   adjudication — the sound-off twin-parity arms land with that ruling
//!   (see the t23 step-0 design note's probe table).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, the gates SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn region_base(debug: bool) -> u32 {
    if debug { pins::BOOT.debug_base } else { pins::BOOT.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::BOOT.debug_len } else { pins::BOOT.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// Parse a value symbol out of an AS listing's symbol table (`NAME : <hex>`),
/// so build-floating values (Z80 driver size, game config ids) are read from
/// the live tree, never hardcoded (the t22 CSELF lesson).
fn listing_symbol(aeon: &Path, debug: bool, name: &str) -> u64 {
    let lst = aeon.join(if debug { "s4.debug.lst" } else { "s4.lst" });
    let txt = std::fs::read_to_string(&lst)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", lst.display()));
    let needle = format!("{name} :");
    for line in txt.lines() {
        let mut rest = line;
        while let Some(pos) = rest.find(&needle) {
            // Word boundary on the left: line start, whitespace, or '|'.
            let ok_left = pos == 0
                || rest[..pos].ends_with(' ')
                || rest[..pos].ends_with('|')
                || rest[..pos].ends_with('*');
            if ok_left {
                let after = &rest[pos + needle.len()..];
                let tok = after.split_whitespace().next().unwrap_or_default();
                if let Ok(v) = u64::from_str_radix(tok, 16) {
                    return v;
                }
            }
            rest = &rest[pos + needle.len()..];
        }
    }
    panic!("symbol {name} not found in {}", lst.display());
}

/// The VALUE seam: prepended-twin drift-lock truths + boot's own mirrors +
/// the stable constants.asm values + the LISTING-PARSED floating values
/// (Z80 driver size, game entry contract) — one equ blob (one Stub pin).
/// `doctor` overrides ONE static pair (negative probe).
fn value_equs(aeon: &Path, debug: bool, doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = vec![
        // engine.vdp port addresses + target/op bit vocabulary (its ensures)
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        ("VRAM", "%100001"),
        ("CRAM", "%101011"),
        ("VSRAM", "%100101"),
        ("READ", "%001100"),
        ("WRITE", "%000111"),
        ("DMA", "%100111"),
        // boot.emp's own mirrors (engine/constants.asm + engine/structs.asm)
        ("PSG_PORT", "$C00011"),
        ("VDP_Shadow_vdp_mode2", "1"),
        // z80_bus template's bus register
        ("Z80_BUS_REQUEST", "$A11100"),
        // bare link-resolved hardware ports (engine/constants.asm)
        ("HW_PORT_A_CTRL_FULL", "$A10008"),
        ("HW_EXPANSION_CTRL_FULL", "$A1000C"),
        ("HW_VERSION", "$A10001"),
        ("TMSS_REGISTER", "$A14000"),
        ("HW_PORT_1_CTRL", "$A10009"),
        ("HW_PORT_2_CTRL", "$A1000B"),
        ("HW_EXPANSION_CTRL", "$A1000D"),
        ("HW_PORT_1_DATA", "$A10003"),
        ("HW_PORT_2_DATA", "$A10005"),
        ("HW_PORT_EXP_DATA", "$A10007"),
        ("YM2612_A0", "$A04000"),
        // region timing/budget truths (engine/constants.asm)
        ("NTSC_TIMING_STEP", "$0100"),
        ("PAL_TIMING_STEP", "$0133"),
        ("DMA_BUDGET_NTSC", "7200"),
        ("DMA_BUDGET_PAL", "15000"),
    ];
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
    let mut owned: Vec<(&str, String)> =
        pairs.into_iter().map(|(n, v)| (n, v.to_string())).collect();
    owned.push((
        "Z80_SOUND_SIZE",
        format!("${:X}", listing_symbol(aeon, debug, "Z80_SOUND_SIZE")),
    ));
    owned.push((
        "GAME_ENTRY_ID",
        format!("${:X}", listing_symbol(aeon, debug, "GAME_ENTRY_ID")),
    ));
    owned.push(("Game_Entry", format!("${:X}", listing_symbol(aeon, debug, "Game_Entry"))));
    sigil_harness::test_support::assemble_owned_equ_pairs(&owned)
}

/// The cross-seam ADDRESS symbols, each a `phase`d one-byte carrier at its
/// pinned per-shape VMA. BootData is the forward pc-rel target — its position
/// is load-bearing (the vdp_init_port BootData_VDPRegs technique).
fn addr_labels(debug: bool) -> Vec<Section> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let rbase =
        |r: pins::Region| -> u32 { if debug { r.debug_base } else { r.plain_base } };
    let mut table: Vec<(&str, u32)> = vec![
        ("BootData", pick(pins::BOOT_DATA)),
        ("VDP_Shadow_Init", rbase(pins::VDP_INIT)),
        ("Init_DMA_Queue", rbase(pins::DMA_QUEUE)),
        ("Init_SpriteTable", rbase(pins::BUFFERS)),
        ("BuildStaticDMA", pick(pins::BUILD_STATIC_DMA)),
        ("Flush_VDP_Shadow", pick(pins::FLUSH_VDP_SHADOW)),
        ("Sound_Init", pick(pins::SOUND_INIT)),
        ("GameLoop", rbase(pins::GAME_LOOP)),
        ("VInt_Level", pick(pins::V_INT_LEVEL)),
        ("VInt_Ptr", pick(pins::V_INT_PTR)),
        ("Game_State", pick(pins::GAME_STATE)),
        ("Game_State_ID", pick(pins::GAME_STATE_ID)),
        ("Game_State_Init", pick(pins::GAME_STATE_INIT)),
        ("Hardware_Region", pick(pins::HARDWARE_REGION)),
        ("Region_Flags", pick(pins::REGION_FLAGS)),
        ("Timing_Step", pick(pins::TIMING_STEP)),
        ("Frame_Accumulator", pick(pins::FRAME_ACCUMULATOR)),
        ("DMA_Budget_Default", pick(pins::DMA_BUDGET_DEFAULT)),
        ("HBlank_Vector_Slot", pick(pins::H_BLANK_VECTOR_SLOT)),
        ("RAM_Start", pick(pins::RAM_START)),
        ("VDP_Shadow_Table", pick(pins::VDP_SHADOW_TABLE)),
        ("VDP_Dirty_Mask", pick(pins::VDP_DIRTY_MASK)),
    ];
    if debug {
        table.push(("CompressionSelfTest", pins::COMPRESSION_SELFTEST.debug_base));
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
            s.lma = 0x0300_0000 + (i as u32) * 0x1_0000;
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

/// Lower the real `boot.emp` (prepend the engine.vdp + engine.z80_bus twins
/// its `use` lines read) with the given comptime shape, place at `base`.
fn lower_boot(
    aeon: &Path,
    base: u32,
    len: usize,
    debug: bool,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let dir = aeon.join("engine/system");
    let main = parse_file(&dir.join("boot.emp"));
    let vdp_file = parse_file(&aeon.join("engine/vdp.emp"));
    let z80_file = parse_file(&aeon.join("engine/z80_bus.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: vdp_file.items.into_iter().chain(z80_file.items).chain(main.items).collect(),
        docs: main.docs.clone(),
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![
            ("DEBUG".to_string(), i128::from(debug)),
            ("SOUND_DRIVER_ENABLED".to_string(), 1),
            ("SOUND_DEBUG_HOTKEYS".to_string(), 0),
        ],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "boot.emp lower errors: {ldiags:?}"
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
         name = \"boot\"\n\
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
    let (mut sections, asserts) = lower_boot(&aeon, base, region_len(debug), debug);
    sections.extend(value_equs(&aeon, debug, None));
    sections.extend(addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let adiags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &asserts);
    assert!(
        adiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "link asserts failed: {adiags:?}"
    );
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));

    let expected = &refrom[base as usize..base as usize + region_len(debug)];
    let section = linked.section("boot").expect("linked image must carry boot");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("boot ({shape})"));
}

#[test]
fn boot_region_matches_reference() {
    run(false);
}

#[test]
fn boot_debug_region_matches_reference() {
    run(true);
}

/// Negative probe: a doctored PSG_PORT truth must FIRE boot.emp's drift
/// ensure NAMING the constant (the value seam is live, not decorative).
#[test]
fn doctored_psg_port_fires_its_guard() {
    let aeon = aeon_dir();
    if !aeon.join("s4.bin").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon tree not present");
        }
        eprintln!("skip: aeon tree not present");
        return;
    }
    let base = region_base(false);
    let (mut sections, asserts) = lower_boot(&aeon, base, region_len(false), false);
    sections.extend(value_equs(&aeon, false, Some(("PSG_PORT", "$C00013"))));
    sections.extend(addr_labels(false));
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &asserts);
    let fired = diags.iter().any(|d| {
        d.level == sigil_span::Level::Error && d.message.contains("PSG_PORT")
    });
    assert!(fired, "the doctored PSG_PORT truth must fire its ensure: {diags:?}");
}
