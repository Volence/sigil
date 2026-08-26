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
//!   the `bsr.w CompressionSelfTest` (+4). The off-canonical shapes are
//!   twin-parity arms below: sound-OFF (the `moveq #Z80_IDLE_SIZE-1`
//!   ImmSigned8 deferral) and the hotkeys (1,1) shape (the gameBootHook
//!   drift matrix against the REAL game.asm expansion).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, the gates SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module_with_contracts, LowerOptions};
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

/// The frozen reference ROMs (harness `golden/`), NOT the live tree `s4.bin`
/// (post-flip the tree ROM is sigil-canonical — the committed blob is truth,
/// mirroring `native_offcanonical_rom::golden`).
fn golden(name: &str) -> Option<Vec<u8>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../sigil-harness/golden/{name}"));
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) => {
            if strict_gate() {
                panic!("golden missing: {}", path.display());
            }
            None
        }
    }
}

/// The build-floating values boot's imm-links resolve against (Z80 driver
/// size, game entry address). Both come from committed, frozen sources — never
/// from the live tree.
///
/// The tree's `s4.lst` is NOT a usable source: post-flip it is sigil-canonical
/// (any `sigil build` overwrites it, and asl can no longer regenerate it), so
/// parsing it would make this gate depend on whatever was built last. Each
/// value therefore reads a frozen artifact instead:
///
/// - `Z80_SOUND_SIZE` — the resident Z80 driver span, pinned here against the
///   frozen goldens (`golden/s4.bin`, `golden/s4.debug.bin`). It is a driver
///   SIZE, not an address, so it moves only when the Z80 driver itself changes;
///   a cartridge re-layout leaves it alone. `seam1::BLOB_LEN_{PLAIN,DEBUG}`
///   holds the same measurement as its own tripwire — see the note on
///   `seam1_native_link::blob_lengths_are_canonical` for why the two are not
///   automatically equal (the blob is padded to an even length inside the
///   `Z80_Sound_Start`/`_End` brackets).
/// - `GameState_OJZScroll_Init` — read from `pins::OJZ_SCROLL_TEST`, whose
///   `repin.toml` region declares `start = "GameState_OJZScroll_Init"`. The
///   pin's base IS this symbol's address by construction, regenerated from the
///   shipped resolve at every freeze, so there is nothing to re-type here when
///   the cartridge is re-laid-out.
///
/// These are INPUTS to boot's imm-links, not the gate's oracle: the oracle is
/// the frozen golden ROM window that `run` compares against. A wrong value
/// emits wrong bytes and the comparison fails — which is what makes reading the
/// pin safe, and additionally makes this gate an independent alarm on a
/// mis-derived `OJZ_SCROLL_TEST`.
///
/// If the Z80 driver or the game config changes, re-freeze the golden ROMs
/// (`native_full_rom` fires) and re-pin `Z80_SOUND_SIZE` in lockstep.
///
/// `#Game.ENTRY_ID` is a comptime const folded from the game contract env, not
/// a link symbol; `#Game.entry`'s link target is `GameState_OJZScroll_Init`,
/// the game proc.
fn frozen_symbol(debug: bool, name: &str) -> u64 {
    match (name, debug) {
        // The resident Z80 driver span (blob 6164 / 6294 bytes, both already even).
        ("Z80_SOUND_SIZE", false) => 0x1814,
        ("Z80_SOUND_SIZE", true) => 0x1896,
        // `#Game.entry`'s link target. The `ojz_scroll_test` region begins at this
        // symbol, so its base is the symbol's ROM address.
        ("GameState_OJZScroll_Init", false) => u64::from(pins::OJZ_SCROLL_TEST.plain_base),
        ("GameState_OJZScroll_Init", true) => u64::from(pins::OJZ_SCROLL_TEST.debug_base),
        _ => panic!("no frozen value pinned for symbol `{name}` (debug={debug})"),
    }
}

/// The VALUE seam: prepended-twin drift-lock truths + boot's own mirrors +
/// the stable constants.asm values + the FROZEN floating values (Z80 driver
/// size, game entry contract) — one equ blob (one Stub pin).
/// `doctor` overrides ONE static pair (negative probe).
fn value_equs(debug: bool, doctor: Option<(&str, &str)>) -> Vec<Section> {
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
        // VDP_Shadow offset twins (engine.vdp shadow-offset block)
        ("VDP_Shadow_vdp_mode1", "$00"),
        ("VDP_Shadow_vdp_mode2", "$01"),
        ("VDP_Shadow_vdp_mode3", "$0B"),
        ("VDP_Shadow_vdp_hint_rate", "$0A"),
        // boot.emp's own mirror (engine/constants.asm)
        ("PSG_PORT", "$C00011"),
        // The initial supervisor stack (engine/system/constants.emp). EntryPoint
        // reloads sp from it (`lea (SYSTEM_STACK).w, sp`) so a software
        // `jmp EntryPoint` soft-reset does not run the RAM clear on a stale
        // stack — review item 27, finding 7 (2026-08-04). The source pins the
        // `.w` width, so this equ cannot change the encoding.
        ("SYSTEM_STACK", "$FFFFFF00"),
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
        // region budget truths (engine/system/constants.emp). The PAL fixed-timestep
        // consts (NTSC_TIMING_STEP/PAL_TIMING_STEP) were deleted 2026-08-02 (NTSC-only
        // ruling B); only the region-adaptive DMA budget survives the region branch.
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
        format!("${:X}", frozen_symbol(debug, "Z80_SOUND_SIZE")),
    ));
    // The `Game.entry` link target (the game-side proc). `Game.ENTRY_ID` is no
    // longer a link symbol — it folds from the contract env as a comptime const.
    owned.push((
        "GameState_OJZScroll_Init",
        format!("${:X}", frozen_symbol(debug, "GameState_OJZScroll_Init")),
    ));
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
        // boot re-anchors its table cursor on the tail with `lea
        // BootData_PostBlob(pc), a5` before reading the PSG bytes and the post-DMA
        // VDP commands. It used to reach them by WALKING a5 out of the blob, which
        // silently ate the chainer's inter-section alignment pad — 4 bytes in debug
        // (survivable) and 6 in release, where the auto-increment slot read $0000,
        // the VDP took it as a command's first word, and the control-port flip-flop
        // stranded so that no VDP write in the whole ROM ever landed again. Naming
        // the label makes it an outbound cross-seam ref this scope must supply; it
        // is the boot_tail region's base.
        ("BootData_PostBlob", rbase(pins::BOOT_TAIL)),
        ("VDP_Shadow_Init", rbase(pins::VDP_INIT)),
        ("Init_DMA_Queue", rbase(pins::DMA_QUEUE)),
        ("Init_SpriteTable", rbase(pins::BUFFERS)),
        ("BuildStaticDMA", pick(pins::BUILD_STATIC_DMA)),
        // Effects P1: both VInt paths call the raster re-arm, which lives in hblank —
        // an outbound cross-seam call target from vblank's standalone re-lower.
        ("Raster_VBlank", pick(pins::RASTER_V_BLANK)),
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
        // Timing_Step/Frame_Accumulator deleted 2026-08-02 (NTSC-only ruling B).
        ("DMA_Budget_Default", pick(pins::DMA_BUDGET_DEFAULT)),
        ("HBlank_Vector_Slot", pick(pins::H_BLANK_VECTOR_SLOT)),
        ("RAM_Start", pick(pins::RAM_START)),
        // VDP_Dirty_Mask left this table with the blanket-restore parcel: boot's
        // VInt-enable is a shadow write-through only, so the symbol is gone.
        ("VDP_Shadow_Table", pick(pins::VDP_SHADOW_TABLE)),
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
    // boot.emp reads VDP_DATA/VDP_CTRL/PSG_PORT from engine.constants (the
    // hardware-address authority) via `use` — prepend the constants twin so those
    // imports resolve standalone, exactly as the real ambient supplies them.
    let constants_file = parse_file(&dir.join("constants.emp"));
    let vdp_file = parse_file(&aeon.join("engine/vdp.emp"));
    let z80_file = parse_file(&aeon.join("engine/z80_bus.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: constants_file
            .items
            .into_iter()
            .chain(vdp_file.items)
            .chain(z80_file.items)
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
            ("SOUND_DRIVER_ENABLED".to_string(), 1),
            ("SOUND_DEBUG_HOTKEYS".to_string(), 0),
        ],
    };
    // The game-contract env (L1 P2): boot.emp names `#Game.entry`,
    // `#Game.ENTRY_ID`, and `invoke Game.boot_hook`. DERIVED from aeon's own
    // contract and sonic4's own manifest under the canonical shape's defines, so
    // ENTRY_ID and the entry symbol are read out of the manifest instead of copied
    // here (a copy the day sonic4 re-points its entry would gate the wrong value),
    // and boot_hook stays unbound because hotkeys are off in that shape.
    let profile = sigil_harness::native::sonic4_profile(debug);
    let contract_defines: Vec<(String, i128)> =
        profile.emp_defines.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    let env = sigil_harness::test_support::game_contract_env_from_aeon(
        &aeon_dir(),
        &profile,
        &contract_defines,
    );
    let (module, ldiags) = lower_module_with_contracts(&file, &opts, &env);
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
    // pins span runs to the next section's aligned base. Tolerate a short (< 16 B)
    // all-zero tail beyond the lowered image; every real byte still compares.
    let expected = if expected.len() > candidate.len()
        && expected.len() - candidate.len() < 16
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
    let Some(refrom) = golden(rom_name) else {
        eprintln!("skip: golden {rom_name} not present");
        return;
    };
    if !aeon.join("engine/system/boot.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but boot.emp source missing (set AEON_DIR)");
        }
        eprintln!("skip: boot.emp source not present (set AEON_DIR)");
        return;
    }

    let base = region_base(debug);
    let (mut sections, asserts) = lower_boot(&aeon, base, region_len(debug), debug);
    sections.extend(value_equs(debug, None));
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

// `doctored_psg_port_fires_its_guard` RETIRED at the conv-b constants-tail flip:
// PSG_PORT (with VDP_DATA/VDP_CTRL) flipped from boot.emp's local mirror to
// `use engine.constants`, so boot.emp no longer carries an `ensure(extern("PSG_PORT")
// == …)` wall for the doctored probe to fire. Its protection re-homes to the
// six-target byte-identity: a wrong PSG_PORT moves the emitted PSG displacement.

// Off-canonical twin-parity arms RETIRED (flip Stage-2 D2): the sound-OFF and
// HOTKEYS shapes assembled the full AS-side ROM as their oracle — AS-reassembly
// machinery the flip removes. Their coverage is subsumed by the whole-ROM golden
// gates (config_b == sound-off, config_a == hotkeys+mirror; native_offcanonical_*),
// each with its own t24 control. The 3 golden-backed region tests above survive.
