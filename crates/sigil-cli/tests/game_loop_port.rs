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
//! - `VSync_Wait` (plain `$2262`, debug `$22EC`) and `Sound_DrainSfxRing`
//!   (plain `$5EB0`, debug `$736E`) — both `jbsr` -> `bsr.w` PC-RELATIVE, so
//!   the positions are load-bearing. (The drain target flips .emp-side when
//!   sound_api ports later this tranche — the port order is deliberate: this
//!   gate exercises the .emp->AS direction first.)
//! - `Game_State` (`$FFFF8004`, ENGINE RAM — shape-invariant) — abs.w EA.
//! - `Debug_MusicToggle` — hotkeys combos only (module matrix; synthetic
//!   position, there is no pinned hotkeys-on reference).
//!
//! OUTBOUND: `boot.asm:220`'s `bra.w GameLoop` — a synthetic consumer
//! asserts the pc-rel fixup resolves to the per-shape `GameLoop`.
//!
//! ## Reference windows
//! (sourced from `sigil_harness::pins` — regenerate via repin)
//!
//! Plain (map base `$22FE`): `s4.bin[0x22FE..0x2310]` (0x12 bytes).
//! Debug (map base `$238C`): `s4.debug.bin[0x238C..0x239E]` (0x12 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, the reference tests SKIP green —
//! unless `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//! The combo matrix only needs the aeon SOURCE files (game_loop.asm +
//! config/game.asm), not the built ROMs.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test game_loop_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// Per-shape gate geometry: the region base and the true VMAs of the two
/// pc-relative call targets (sourced from `sigil_harness::pins` — regenerate
/// via repin).
struct Shape {
    base: u32,
    vsync_wait: u32,
    drain: u32,
}

const PLAIN: Shape = Shape {
    base: pins::GAME_LOOP.plain_base,
    vsync_wait: pins::V_SYNC_WAIT.plain,
    drain: pins::SOUND_DRAIN_SFX_RING.plain,
};
const DEBUG: Shape = Shape {
    base: pins::GAME_LOOP.debug_base,
    vsync_wait: pins::V_SYNC_WAIT.debug,
    drain: pins::SOUND_DRAIN_SFX_RING.debug,
};
const REGION_LEN: usize = pins::GAME_LOOP.plain_len;

/// Compile the real `engine/system/game_loop.emp` with the given defines,
/// pinned at `base`, with the synthetic cross-seam labels at the given VMAs.
/// Returns (resolved sections, linked image).
fn compile_emp(
    defines: &[(&str, i128)],
    base: u32,
    vsync_wait: u32,
    drain: u32,
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
    let (module, ldiags) = lower_module(&file, &opts);
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
        ("Debug_MusicToggle", dbg_toggle),
        ("Game_State", pins::GAME_STATE.plain),
    ] {
        let asm = format!(
            "cpu 68000\n\
             phase ${vma:X}\n\
             {name}:\n\
             \tdc.b 0\n"
        );
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
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
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
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

/// The AS-twin oracle: the REAL `game_loop.asm` body, prefaced by the REAL
/// `gameDebugTick` macro extracted from `games/sonic4/config/game.asm` at
/// test time (THE drift guard for the H2 expansion mirror), assembled through
/// sigil's AS front-end with the same synthetic label values. `ifdef` tests
/// DEFINEDNESS, so an off define is OMITTED (the harness maps presence->1).
fn as_twin_bytes(
    drain_on: bool,
    hotkeys_on: bool,
    base: u32,
    vsync_wait: u32,
    drain: u32,
    dbg_toggle: u32,
) -> Vec<u8> {
    let aeon = aeon_dir();
    let loop_src = std::fs::read_to_string(aeon.join("engine/system/game_loop.asm"))
        .unwrap_or_else(|e| panic!("cannot read game_loop.asm: {e}"));
    let game_src = std::fs::read_to_string(aeon.join("games/sonic4/config/game.asm"))
        .unwrap_or_else(|e| panic!("cannot read games/sonic4/config/game.asm: {e}"));

    // Extract sonic4's `gameDebugTick macro ... endm` block verbatim.
    // ASSUMES a flat macro body (the first `endm` closes it) — a nested
    // macro or a bare `endm` on a comment-only line would truncate the
    // oracle; keep the real macro flat or teach this extractor.
    let lines: Vec<&str> = game_src.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with("gameDebugTick") && l.contains("macro"))
        .expect("games/sonic4/config/game.asm must define gameDebugTick (H2 mirror source)");
    let end = lines[start..]
        .iter()
        .position(|l| l.trim() == "endm")
        .map(|i| start + i)
        .expect("gameDebugTick macro must close with endm");
    let macro_text = lines[start..=end].join("\n");

    let game_state = pins::GAME_STATE.plain;
    let src = format!(
        "cpu 68000\n\
         supmode on\n\
         VSync_Wait = ${vsync_wait:X}\n\
         Sound_DrainSfxRing = ${drain:X}\n\
         Debug_MusicToggle = ${dbg_toggle:X}\n\
         Game_State = ${game_state:X}\n\
         {macro_text}\n\
         org ${base:X}\n\
         {loop_src}\n"
    );
    let mut defines: Vec<(String, i64)> = Vec::new();
    if drain_on {
        defines.push(("SOUND_DRIVER_ENABLED".to_string(), 1));
    }
    if hotkeys_on {
        defines.push(("SOUND_DEBUG_HOTKEYS".to_string(), 1));
    }
    let opts = AsOptions { initial_cpu: Cpu::M68000, defines, ..AsOptions::default() };
    let out = assemble(&src, &opts).unwrap_or_else(|d| panic!("AS twin assemble: {d:?}"));
    let mut sections = out.sections;
    for sec in &mut sections {
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("AS twin resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS twin link failed: {d:?}"));
    let sec = linked
        .sections
        .iter()
        .find(|s| s.lma == base && !s.bytes.is_empty())
        .unwrap_or_else(|| panic!("AS twin must emit a section at {base:#x}"));
    sec.bytes.clone()
}

/// On mismatch, report the first differing offset plus context on each side.
fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
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

/// The H1/H2 define matrix — all four combos, module-level, .emp vs the
/// AS-twin oracle at matrix-chosen synthetic positions. This is BOTH hazard
/// rulings' acceptance test AND the H2 mirror's drift guard (the oracle
/// re-extracts the macro body from the real config/game.asm every run).
#[test]
fn combo_matrix_matches_as_twin() {
    let aeon = aeon_dir();
    if !aeon.join("engine/system/game_loop.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    // Matrix positions: VSync behind the block (negative disp), the drain
    // ahead (positive), the toggle in abs.w reach (the real one sits there).
    let (base, vsync, drain, toggle) = (0x1000u32, 0x0F00u32, 0x2000u32, 0x3000u32);
    for (drain_on, hotkeys_on) in [(true, false), (true, true), (false, false), (false, true)] {
        let defines: Vec<(&str, i128)> = vec![
            ("SOUND_DRIVER_ENABLED", i128::from(drain_on)),
            ("SOUND_DEBUG_HOTKEYS", i128::from(hotkeys_on)),
        ];
        let (_, linked) = compile_emp(&defines, base, vsync, drain, toggle, false);
        let section = linked.section("game_loop").expect("linked image must carry game_loop");
        let expected = as_twin_bytes(drain_on, hotkeys_on, base, vsync, drain, toggle);
        assert_region_matches(
            &section.bytes,
            &expected,
            &format!("game_loop combo (drain={drain_on}, hotkeys={hotkeys_on}) vs AS twin"),
        );
    }
}

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
        compile_emp(&defines, shape.base, shape.vsync_wait, shape.drain, 0x3000, true);

    let lo = shape.base as usize;
    let expected = &refrom[lo..lo + REGION_LEN];
    let section = linked.section("game_loop").expect("linked image must carry game_loop");
    assert_region_matches(
        &section.bytes,
        expected,
        &format!("game_loop vs {rom_name}[{lo:#x}..{:#x}]", lo + REGION_LEN),
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

/// (plain) `game_loop` bytes == `s4.bin[0x22FE..0x2310]`.
#[test]
fn game_loop_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) `game_loop` bytes == `s4.debug.bin[0x238C..0x239E]`.
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
            flip_parse(&aeon.join("engine/z80_bus.emp")),
            flip_parse(&aeon.join("engine/irq.emp")),
        ],
        aeon.join("engine/system"),
        "vblank",
        vb_base,
        vb_len,
        vec![
            ("DEBUG".to_string(), dbg),
            ("SOUND_DRIVER_ENABLED".to_string(), 1),
            ("SOUND_DBG_MIRROR".to_string(), 0),
        ],
    ));

    // Value seam (ONE equ blob — a second assemble would redefine `Stub:`).
    let pairs: Vec<(&str, &str)> = vec![
        ("SND_Z80_BASE", "$A00000"),
        ("SND_CTRL_DMA_ACTIVE", "$1F04"),
        ("Z80_BUS_REQUEST", "$A11100"),
    ];
    sections.extend(sigil_harness::test_support::assemble_equ_pairs(&pairs));

    // Address seam: everything both modules still read cross-seam.
    // NO VSync_Wait carrier — the flip means the name resolves to vblank.emp's
    // proc; a stale carrier here would be the §11 Q4 collision.
    let mut table: Vec<(&str, u32)> = vec![
        ("Sound_DrainSfxRing", pick(pins::SOUND_DRAIN_SFX_RING)),
        ("Game_State", pick(pins::GAME_STATE)),
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
    for (i, (name, vma)) in table.iter().enumerate() {
        let vma = *vma;
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
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
    let gl = linked.section("game_loop").expect("game_loop region");
    let gr = &refrom[gl_base as usize..gl_base as usize + gl_len];
    assert_eq!(gl.bytes.len(), gr.len(), "game_loop ({shape} flip): length");
    assert_eq!(gl.bytes, gr, "game_loop ({shape} flip): bytes must match the reference");
    let vb = linked.section("vblank").expect("vblank region");
    let vr = &refrom[vb_base as usize..vb_base as usize + vb_len];
    assert_eq!(vb.bytes.len(), vr.len(), "vblank ({shape} flip): length");
    assert_eq!(vb.bytes, vr, "vblank ({shape} flip): bytes must match the reference");
}

#[test]
fn two_module_ownership_flip_plain() {
    two_module_flip(false, "s4.bin");
}

#[test]
fn two_module_ownership_flip_debug() {
    two_module_flip(true, "s4.debug.bin");
}
