//! Tranche 5 — the REAL `game_loop.emp` port, region-level byte gate.
//!
//! `collision_lookup_port.rs`'s sibling for the SEVENTH code port: compiles
//! the ACTUAL ported file from aeon's tree — `engine/system/game_loop.emp` —
//! through the production parse -> lower -> place -> resolve -> link
//! pipeline, and asserts the `game_loop` section's flattened bytes equal the
//! reference ROM window at the pinned addresses, in BOTH build shapes.
//!
//! ## The two hazard classes this port exists to settle (step-0 note:
//! `notes/2026-07-10-tranche5-game-loop-design.md`)
//!
//! - **H1 — `ifdef SOUND_DRIVER_ENABLED` inside a ported file**: the first
//!   CODE port whose body takes a build-shape define (mt_bank's `DEBUG`
//!   pattern, now at proc-statement position). The .emp emits the
//!   `jbsr Sound_DrainSfxRing` drain line only when the define is 1.
//! - **H2 — `gameDebugTick`, the game-contract macro seam**: the .emp mirrors
//!   sonic4's macro EXPANSION (`jsr Debug_MusicToggle` under
//!   `SOUND_DEBUG_HOTKEYS && SOUND_DRIVER_ENABLED`). The mirror's DRIFT GUARD
//!   is `combo_matrix_matches_as_twin` below: the AS-oracle side extracts the
//!   macro body FROM THE REAL `games/sonic4/config/game.asm` at test time, so
//!   a macro-body edit that the .emp doesn't follow fails the matrix.
//!
//! ## Shape defines
//!
//! Both pinned reference shapes are `SOUND_DRIVER_ENABLED` ON (build.sh
//! defaults it), `SOUND_DEBUG_HOTKEYS` OFF (env opt-in, neither pin sets it) —
//! so the reference gates run the (1,0) combo and `gameDebugTick` contributes
//! ZERO bytes to both windows. The other three combos have no pinned ROM to
//! diff against (the engine.inc resume orgs are sound-on-shape addresses), so
//! they are MODULE-LEVEL gates against the AS twin assembled through sigil's
//! own AS front-end with the same defines and synthetic label positions.
//!
//! ## The cross-seam symbols
//!
//! INBOUND, supplied as synthetic AS-side sections at their true per-shape
//! VMAs (read from each shape's listing symbol table):
//!
//! - `VSync_Wait` and `Sound_DrainSfxRing` (per-shape VMAs in `pins`) — both
//!   `jbsr` -> `bsr.w` PC-RELATIVE, so
//!   the positions are load-bearing. (The drain target flips .emp-side when
//!   sound_api ports later this tranche — the port order is deliberate: this
//!   gate exercises the .emp->AS direction first.)
//! - `Logic_Tick` (`$FFFF8004`, ENGINE RAM — shape-invariant) — the I2 addq.l
//!   target (input/replay parcel); pushed `Game_State` +4.
//! - `Game_State` (`$FFFF8008`, ENGINE RAM — shape-invariant) — abs.w EA.
//! - `Debug_MusicToggle` — hotkeys combos only (module matrix; synthetic
//!   position, there is no pinned hotkeys-on reference).
//!
//! OUTBOUND: `boot.asm:220`'s `bra.w GameLoop` — a synthetic consumer
//! asserts the pc-rel fixup resolves to the per-shape `GameLoop`.
//!
//! ## Reference windows
//! (sourced from `sigil_harness::pins` — regenerate via repin)
//!
//! Both windows come from `pins::GAME_LOOP` at run time — base and length, per
//! shape. The numbers are deliberately not restated here: a bound copied into
//! prose is executed by nothing, so nothing can go red when it rots.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, or
//! `EMPYREAN_SUITE_ROOT`). Absent, the reference tests SKIP green —
//! unless `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//! The combo matrix only needs the aeon SOURCE files (game_loop.asm +
//! config/game.asm), not the built ROMs.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test game_loop_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module_with_contracts, LowerOptions};

/// The game-contract env for the isolated game_loop oracle (L1 P2): game_loop.emp
/// names `invoke Game.debug_tick`. DERIVED from aeon's own contract and sonic4's
/// own manifest under the canonical shape's defines — hotkeys OFF, so the hook
/// binds `= empty` and the `invoke` emits nothing, which is exactly what the
/// reference ROM this oracle byte-gates against carries. A hand-written stub
/// would restate the interface here and go stale the day the engine grows a
/// member game_loop names.
fn game_loop_contract_env() -> sigil_frontend_emp::contract::InterfaceEnv {
    let profile = sigil_harness::native::sonic4_profile(false);
    let defines: Vec<(String, i128)> =
        profile.emp_defines.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    sigil_harness::test_support::game_contract_env_from_aeon(&aeon_dir(), &profile, &defines)
}
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// Per-shape gate geometry: the region base and the true VMAs of the two
/// pc-relative call targets (sourced from `sigil_harness::pins` — regenerate
/// via repin).
struct Shape {
    base: u32,
    len: usize,
    vsync_wait: u32,
    drain: u32,
    // Parcel I3: game_loop's `jbsr Input_Tick` (bsr.w pc-relative) target — the
    // replay seam, a shape-specific code VMA (unlike the shape-invariant RAM cells).
    input_tick: u32,
    // Effects P2: game_loop's `jbsr Palette_Compose` — the once-per-frame palette
    // composition. Same class as `input_tick`: a pc-relative call into another
    // section, so the VMA is shape-specific and must come from pins.
    palette_compose: u32,
}

const PLAIN: Shape = Shape {
    base: pins::GAME_LOOP.plain_base,
    len: pins::GAME_LOOP.plain_len,
    vsync_wait: pins::V_SYNC_WAIT.plain,
    drain: pins::SOUND_DRAIN_SFX_RING.plain,
    input_tick: pins::INPUT_TICK.plain,
    palette_compose: pins::PALETTE_COMPOSE.plain,
};
const DEBUG: Shape = Shape {
    base: pins::GAME_LOOP.debug_base,
    // input-6button: the +0xB0 shift landed S4LZ_DecompressDict misaligned in
    // PLAIN only, so the two lens now differ (plain carries a 2-byte align
    // pad the tolerant compare zero-verifies) — the region len must be
    // per-shape, not the old shared plain_len const.
    len: pins::GAME_LOOP.debug_len,
    vsync_wait: pins::V_SYNC_WAIT.debug,
    drain: pins::SOUND_DRAIN_SFX_RING.debug,
    input_tick: pins::INPUT_TICK.debug,
    palette_compose: pins::PALETTE_COMPOSE.debug,
};

/// Compile the real `engine/system/game_loop.emp` with the given defines,
/// pinned at `base`, with the synthetic cross-seam labels at the given VMAs.
/// Returns (resolved sections, linked image).
// One argument per synthetic cross-seam VMA; a struct would just relocate the
// list without making any call site clearer.
#[allow(clippy::too_many_arguments)]
fn compile_emp(
    defines: &[(&str, i128)],
    base: u32,
    vsync_wait: u32,
    drain: u32,
    input_tick: u32,
    palette_compose: u32,
    dbg_toggle: u32,
    with_consumer: bool,
) -> (Vec<Section>, sigil_link::LinkedImage) {
    let dir = aeon_dir().join("engine/system");
    let src = std::fs::read_to_string(dir.join("game_loop.emp"))
        .unwrap_or_else(|e| panic!("cannot read game_loop.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "game_loop.emp parse errors: {pdiags:?}"
    );

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
    };
    let (module, ldiags) =
        lower_module_with_contracts(&file, &opts, &game_loop_contract_env());
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors: {ldiags:?}"
    );

    // The region is sized to the LARGEST combo (drain + hotkeys jsr) so the
    // same map serves the whole matrix; the linked section carries only its
    // emitted bytes, so region slack never pads the comparison.
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
         name = \"game_loop\"\n\
         lma_base = {base:#x}\n\
         size = 0x18\n\
         kind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    // Synthetic AS-side cross-seam labels, each phased at its true (or
    // matrix-chosen) VMA; carrier LMAs are harness-private.
    let mut lma = 0x0200_0000u32;
    for (name, vma) in [
        ("VSync_Wait", vsync_wait),
        ("Sound_DrainSfxRing", drain),
        ("Input_Tick", input_tick),              // I3: jbsr Input_Tick (replay seam)
        ("Palette_Compose", palette_compose), // Effects P2: jbsr Palette_Compose
        ("Debug_MusicToggle", dbg_toggle),
        ("Logic_Tick", pins::LOGIC_TICK.plain),  // I2: addq.l #1, Logic_Tick (shape-invariant RAM)
        ("Game_State", pins::GAME_STATE.plain),
    ] {
        let asm = format!(
            "cpu 68000\n\
             phase ${vma:X}\n\
             {name}:\n\
             \tdc.b 0\n"
        );
        let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}"))
            .sections;
        for sec in &mut secs {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.extend(secs);
        lma += 0x10_0000;
    }

    if with_consumer {
        // boot.asm:220's shape — the outbound bare-name proof. The consumer
        // is PHASED at $8000 — its PC (vma, what the displacement measures)
        // sits INSIDE bra.w's ±32K of both shapes' bases, so the asserted
        // displacement is a real reachable one. (sigil-link does not
        // range-check pc-rel16 fixups today — gap-ledger jot — and an
        // unphased carrier "passes" mod 2^16 regardless of its LMA: the
        // review caught this test's first version doing exactly that.)
        let asm = "cpu 68000\n\
                   phase $8000\n\
                   Consumer:\n\
                   \tbra.w   GameLoop\n";
        let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
        let mut secs = assemble(asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}"))
            .sections;
        for sec in &mut secs {
            sec.lma = 0x8000;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.extend(secs);
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked)
}

// The AS-twin oracle (the REAL game_loop.asm body + the gameDebugTick macro
// extracted from config/game.asm) RETIRED (flip Stage-2): game_loop.asm is
// deleted — the .emp is the only source. The gameDebugTick H2-mirror drift guard
// (kill-list row 9, still OPEN as the Stage-3 game-contract-hook mechanism) is
// now gated by the native whole-ROM golden gates: Config-A (hotkeys+mirror ON)
// and the canonical/Config-B shapes together cover the define matrix, each with
// a t24 control. config/game.asm survives as residual game config.

/// On mismatch, report the first differing offset plus context on each side.
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
    // pins span runs to the next section's aligned base. Sections align to 0x20, so
    // the fill runs 0..31 bytes (art-streaming-p2-task4: VInt_Level's growth ended
    // vblank's plain code at 0x2290, a full 0x10 short of HBLANK's 0x20-aligned
    // 0x22A0 base). Tolerate a short (< 32 B) all-zero tail beyond the lowered
    // image; every real byte still compares.
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

// The H1/H2 combo-matrix AS-twin acceptance test RETIRED (flip Stage-2) with
// game_loop.asm — see the note above `assert_region_matches`. The four-combo
// coverage is subsumed by the native whole-ROM golden gates.

/// Both pinned shapes' reference gate + the outbound bare-name proof, shared
/// body.
fn reference_gate(shape: &Shape, rom_name: &str) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let defines: Vec<(&str, i128)> =
        vec![("SOUND_DRIVER_ENABLED", 1), ("SOUND_DEBUG_HOTKEYS", 0)];
    // Debug_MusicToggle is unreferenced in the (1,0) combo; any synthetic
    // position satisfies the link without touching the bytes.
    let (_, linked) =
        compile_emp(
            &defines,
            shape.base,
            shape.vsync_wait,
            shape.drain,
            shape.input_tick,
            shape.palette_compose,
            0x3000,
            true,
        );

    let lo = shape.base as usize;
    let expected = &refrom[lo..lo + shape.len];
    let section = linked.section("game_loop").expect("linked image must carry game_loop");
    assert_region_matches(
        &section.bytes,
        expected,
        &format!("game_loop vs {rom_name}[{lo:#x}..{:#x}]", lo + shape.len),
    );

    // Outbound proof: boot.asm's `bra.w GameLoop` resolves to the region base.
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x8000)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let expected_disp = (shape.base as i64 - (consumer.lma as i64 + 2)) as i16;
    assert_eq!(
        disp, expected_disp,
        "bare-name proof: `bra.w GameLoop` must resolve to {:#x}",
        shape.base
    );
}

/// (plain) `game_loop` bytes == the `s4.bin` window at `pins::GAME_LOOP`'s
/// plain base/len.
#[test]
fn game_loop_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) `game_loop` bytes == the `s4.debug.bin` window at
/// `pins::GAME_LOOP`'s debug base/len.
#[test]
fn game_loop_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}

// ---------------------------------------------------------------------------
// Tranche-21 ownership flip (kill-list row 29): `VSync_Wait` moved from an
// extern decl to the .emp-owned `engine.vblank` proc. This persisted
// two-module link test compiles game_loop.emp + vblank.emp TOGETHER — the
// extern decl is GONE from game_loop.emp, the call resolves module-to-module,
// and BOTH regions byte-match the shipped reference ROM (the
// plane_buffer/entity_window flip-test template, t20 dplc/bg_anim shape).
// ---------------------------------------------------------------------------

fn flip_parse(path: &std::path::Path) -> sigil_frontend_emp::ast::File {
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

fn flip_lower(
    main: sigil_frontend_emp::ast::File,
    ambient: Vec<sigil_frontend_emp::ast::File>,
    include_root: std::path::PathBuf,
    region: &str,
    base: u32,
    len: usize,
    defines: Vec<(String, i128)>,
) -> Vec<Section> {
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
    let (module, ldiags) =
        lower_module_with_contracts(&file, &opts, &game_loop_contract_env());
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
    sections
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
    let gl_base = if debug { pins::GAME_LOOP.debug_base } else { pins::GAME_LOOP.plain_base };
    let gl_len = if debug { pins::GAME_LOOP.debug_len } else { pins::GAME_LOOP.plain_len };
    let vb_base = if debug { pins::VBLANK.debug_base } else { pins::VBLANK.plain_base };
    let vb_len = if debug { pins::VBLANK.debug_len } else { pins::VBLANK.plain_len };

    let dbg = i128::from(debug);
    let mut sections = flip_lower(
        flip_parse(&aeon.join("engine/system/game_loop.emp")),
        vec![],
        aeon.join("engine/system"),
        "game_loop",
        gl_base,
        gl_len,
        vec![
            ("DEBUG".to_string(), dbg),
            ("SOUND_DRIVER_ENABLED".to_string(), 1),
            ("SOUND_DEBUG_HOTKEYS".to_string(), 0),
        ],
    );
    sections.extend(flip_lower(
        flip_parse(&aeon.join("engine/system/vblank.emp")),
        vec![
            // vblank.emp `use engine.sound_constants.*` for the DMA-window flag —
            // prepend the authority so the SND_* consts fold in this lower.
            flip_parse(&aeon.join("engine/sound/sound_constants.emp")),
            flip_parse(&aeon.join("engine/z80_bus.emp")),
            flip_parse(&aeon.join("engine/irq.emp")),
            // m1-budget-fix: VInt_Level's Critical-charge walk uses DMAEntry
            // (sizeof + SizeH field), so engine.structs must be in scope.
            flip_parse(&aeon.join("engine/structs.emp")),
        ],
        aeon.join("engine/system"),
        "vblank",
        vb_base,
        vb_len,
        vec![
            ("DEBUG".to_string(), dbg),
            ("SOUND_DRIVER_ENABLED".to_string(), 1),
            ("SOUND_DBG_MIRROR".to_string(), 0),
            // Z80_RAM (engine.constants) is the base of SND_Z80_BASE.
            ("Z80_RAM".to_string(), 0xA0_0000),
        ],
    ));

    // Value seam (ONE equ blob — a second assemble would redefine `Stub:`).
    // SND_Z80_BASE / SND_CTRL_DMA_ACTIVE are authored in sound_constants.emp now
    // (prepended above), so only the z80_bus register stays a link extern.
    let pairs: Vec<(&str, &str)> = vec![
        ("Z80_BUS_REQUEST", "$A11100"),
        // NEW-1 (defect-batch-8): VInt_Lag's $8F02 re-assert names VDP_CTRL.
        ("VDP_CTRL", "$C00004"),
    ];
    sections.extend(sigil_harness::test_support::assemble_equ_pairs(&pairs));

    // Address seam: everything both modules still read cross-seam.
    // NO VSync_Wait carrier — the flip means the name resolves to vblank.emp's
    // proc; a stale carrier here would be the §11 Q4 collision.
    let mut table: Vec<(&str, u32)> = vec![
        ("Sound_DrainSfxRing", pick(pins::SOUND_DRAIN_SFX_RING)),
        ("Input_Tick", pick(pins::INPUT_TICK)),  // I3: game_loop's replay-seam jbsr target
        ("Logic_Tick", pick(pins::LOGIC_TICK)),  // I2: game_loop's addq target
        ("Game_State", pick(pins::GAME_STATE)),
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
        // character-lens-sweep (2026-08-13): VInt_Level publishes the held bytes,
        // latching them once per tick from the IRQ-owned raw shadows (a lag VBlank
        // must not overwrite a running tick's input). Both sides of that copy.
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
        // m1-budget-fix: VInt_Level now charges the plane drain + Critical DMA.
        ("Plane_Buffer_Ptr", pick(pins::PLANE_BUFFER_PTR)),
        ("DMA_Critical", pick(pins::DMA_CRITICAL)),
        ("DMA_Critical_Slot", pick(pins::DMA_CRITICAL_SLOT)),
        // Effects P1: both VInt paths call the raster re-arm, which lives in hblank —
        // an outbound cross-seam call target from vblank's standalone re-lower.
        ("Raster_VBlank", pick(pins::RASTER_V_BLANK)),
        // Effects P2: GameLoop composes the palette once per frame. It is arithmetic,
        // not VDP work, so it hangs off the main loop rather than VBlank — an outbound
        // cross-seam call target from game_loop's standalone re-lower, peer of the above.
        ("Palette_Compose", pick(pins::PALETTE_COMPOSE)),
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
        ("PageIn_Process", if debug { pins::PAGE_IN.debug_base } else { pins::PAGE_IN.plain_base }),  // VSync_Wait's idle-slice jbsr target
        ("PageIn_BankRegs", pick(pins::PAGE_IN_BANK_REGS)),
        ("ZX0R_Decompress", if debug { pins::ZX0_RESUME.debug_base } else { pins::ZX0_RESUME.plain_base }),
        ("ZX0R_Decompress.__end", pick(pins::ZX0R_DECOMPRESS_END)),
        // Art-streaming P2a Task 4 — VInt_Level's Important-drain Staging_Busy release.
        ("PageIn_Staging_Busy", pick(pins::PAGE_IN_STAGING_BUSY)),
        ("DMA_Important", pick(pins::DMA_IMPORTANT)),
        ("DMA_Important_Slot", pick(pins::DMA_IMPORTANT_SLOT)),
    ];
    if debug {
        table.push(("Lag_Frame_Count", pins::LAG_FRAME_COUNT));
        table.push(("DMA_Bytes_ThisFrame", pins::DMA_BYTES_THIS_FRAME));
        // The hook's DEBUG-only Preempts counter bump.
        table.push(("Dbg_PageIn_Preempts", pins::DBG_PAGE_IN_PREEMPTS));
    }
    for (i, (name, vma)) in table.iter().enumerate() {
        let vma = *vma;
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections;
        for sec in &mut secs {
            sec.lma = 0x0300_0000 + (i as u32) * 0x1_0000;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.extend(secs);
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("flip resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("flip link failed: {d:?}"));

    let shape = if debug { "debug" } else { "plain" };
    // Tolerant compares (the dma_queue-flip precedent): the plain game_loop
    // window now ends in a 2-byte align pad (input-6button +0xB0 shift).
    let gl = linked.section("game_loop").expect("game_loop region");
    let gr = &refrom[gl_base as usize..gl_base as usize + gl_len];
    assert_region_matches(&gl.bytes, gr, &format!("game_loop ({shape} flip)"));
    let vb = linked.section("vblank").expect("vblank region");
    let vr = &refrom[vb_base as usize..vb_base as usize + vb_len];
    assert_region_matches(&vb.bytes, vr, &format!("vblank ({shape} flip)"));
}

#[test]
fn two_module_ownership_flip_plain() {
    two_module_flip(false, "s4.bin");
}

#[test]
fn two_module_ownership_flip_debug() {
    two_module_flip(true, "s4.debug.bin");
}
