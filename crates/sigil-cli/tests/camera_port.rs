//! Tranche 19 — the REAL `camera.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/camera.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline and asserts the
//! `camera` region's flattened bytes equal the reference ROM window at the
//! pinned base, in BOTH build shapes. The §4 camera: act-descriptor init +
//! per-frame deadzone follow with spindash freeze, landing lock, and the
//! continuous-scroll grid clamps.
//!
//! ## Shape
//! Both the base and the LENGTH are shape-dependent, so each shape diffs
//! against its own ROM window. The four numbers live in
//! `pins::CAMERA.{plain,debug}_{base,len}` — regenerate via `repin` — and are
//! deliberately not restated here: a bound copied into prose is executed by
//! nothing, so nothing can go red when it rots.
//!
//! ## Game-contract seam
//! camera.emp takes `-D GAME_CAMERA_JUMP_LOCK` (0|1) — the game_loop.emp
//! define mechanism. The sonic4 reference shape is =1 (byte gates below). The
//! =0 shape has no reference ROM (demo builds are not gated); its gate is
//! `jump_lock_off_compiles_without_game_symbols`: lower+link must succeed with
//! NO `_pl_state`/`PSTATE_*` symbols provided — the twin's "games without
//! those symbols set the flag to 0" promise as a real link-time property.
//!
//! ## Cross-seam symbols
//! - RAM (abs.w operands): `Camera_X/Y`, `Camera_Pan_Offset`,
//!   `Camera_Deadzone_Base`, `Camera_Hold_Frames`, `Camera_Target`,
//!   `Current_Act_Ptr` — each a `phase`d one-byte carrier at its pinned
//!   per-shape VMA (pins.rs, repin-derived).
//! - Game-owned values: `_pl_state` (address-sum equ — defers through the
//!   link) and `PSTATE_JUMP`/`PSTATE_ROLLJUMP` (cmpi.b imm8 — mirrored consts,
//!   drift-locked; the negative probe doctors one).
//! - `engine.constants`/`engine.structs`/`sst` twins ride the ambient prepend;
//!   their drift guards ride this gate.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test camera_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module_with_contracts, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// SYNTHETIC BY INTENT: a one-member `Game` binding `CAMERA_JUMP_LOCK` to a
/// CHOSEN bool. `jump_lock_off_compiles_without_game_symbols` needs the `false`
/// arm — the specialisation no shipped game declares — so the binding here is a
/// probe INPUT, not a restatement of anybody's manifest. The reference gates do
/// not use it: they bind sonic4's real contract via
/// [`sonic4_contract_env`].
fn camera_contract_env(jump_lock: bool) -> sigil_frontend_emp::contract::InterfaceEnv {
    let b = if jump_lock { "true" } else { "false" };
    sigil_harness::test_support::game_contract_env(
        "module engine.game_contract\npub interface Game {\n    const CAMERA_JUMP_LOCK: bool\n}\n",
        &format!("module games.g.game\npub implement Game {{\n    const CAMERA_JUMP_LOCK = {b}\n}}\n"),
        &[],
    )
}

/// sonic4's OWN contract, bound from aeon's interface and manifest at the
/// canonical shape — what the reference gates lower against, since the ROM they
/// compare to was built with exactly these bindings. Replaces a hardcoded
/// `jump_lock = 1` that meant "sonic4 declares true" without saying so.
fn sonic4_contract_env() -> sigil_frontend_emp::contract::InterfaceEnv {
    let profile = sigil_harness::native::sonic4_profile(false);
    let defines: Vec<(String, i128)> =
        profile.emp_defines.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    sigil_harness::test_support::game_contract_env_from_aeon(&aeon_dir(), &profile, &defines)
}

fn region_base(debug: bool) -> u32 {
    if debug { pins::CAMERA.debug_base } else { pins::CAMERA.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::CAMERA.debug_len } else { pins::CAMERA.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn level_dir() -> PathBuf {
    aeon_dir().join("engine/level")
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// The map: a `text` carrier for the zero-byte default section, and the
/// `camera` region pinned at the per-shape reference base + length.
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
         name = \"camera\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// camera.emp's OWN mirrored constants + the game-contract values its
/// jump-lock block reads back through `extern()`. `doctor` overrides ONE pair
/// (the negative probes). Engine constants + Act_*/SST_* field offsets ride
/// the prepended twins via `engine_constant_equs` / `act_sec_field_equs` /
/// `sst_field_equs`.
fn camera_value_equs(doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = vec![
        // camera.emp local mirror (truth: engine/constants.asm:392)
        ("CAM_MAX_Y_STEP", "16"),
        // game-contract values (truth: games/sonic4 player_common.asm + config)
        ("_pl_state", "$32"),  // sst-fold: the PlayerV overlay window moved $2E -> $30
        ("PSTATE_JUMP", "8"),
        ("PSTATE_ROLLJUMP", "10"),
    ];
    if let Some((name, val)) = doctor {
        for p in pairs.iter_mut() {
            if p.0 == name {
                p.1 = val;
            }
        }
    }
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    pairs.extend(sigil_harness::test_support::sst_field_equs());
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols — camera RAM state + `Camera_Target` +
/// `Current_Act_Ptr` — each a `phase`d one-byte carrier at its pinned
/// per-shape VMA (label position selects abs.w width and the low-word bytes).
fn camera_addr_labels(debug: bool) -> Vec<Section> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let mut table: Vec<(&str, u32)> = vec![
        ("Camera_X", pick(pins::CAMERA_X)),
        ("Camera_Y", pick(pins::CAMERA_Y)),
        ("Camera_Deadzone_Base", pick(pins::CAMERA_DEADZONE_BASE)),
        ("Camera_Pan_Offset", pick(pins::CAMERA_PAN_OFFSET)),
        ("Camera_Hold_Frames", pick(pins::CAMERA_HOLD_FRAMES)),
        // P2c Task 10: the soft-clamp axis-hold bits Camera_Update reads.
        ("Camera_Art_Hold", pick(pins::CAMERA_ART_HOLD)),
        ("Camera_X_Max", pick(pins::CAMERA_X_MAX)),
        ("Camera_Y_Max", pick(pins::CAMERA_Y_MAX)),
        // C1: the camera follows `Camera_Target` — an engine RAM word holding the
        // LEADER's SST pointer, seeded by level init — where it used to name
        // `Player_1` outright. That is what makes the follow character-agnostic,
        // and it is why `Player_1` is no longer a cross-seam symbol of this module.
        ("Camera_Target", pick(pins::CAMERA_TARGET)),
        // tails-flight (aeon 89837e3e): the curl compensation. The game publishes
        // it per character (Player_RefreshPhysics, from the record's two box
        // heights) and the camera consumes it, which is what keeps the follow
        // correct when a balled-up character's centre drops.
        ("Camera_Curl_Offset", pick(pins::CAMERA_CURL_OFFSET)),
        ("Current_Act_Ptr", pick(pins::CURRENT_ACT_PTR)),
    ];
    if debug {
        // P2c Task 10: the DEBUG-only soft-clamp frame counter (abs.w in the DEBUG
        // counter bump at Camera_Update entry).
        table.push(("Dbg_Cam_Clamp_Frames", pins::DBG_CAM_CLAMP_FRAMES));
    }
    let mut out = Vec::new();
    for (i, (name, vma)) in table.iter().enumerate() {
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

/// Lower the real `camera.emp` (prepend the `engine.constants` twin +
/// `engine.structs` + `sst`), place into the per-shape map, append the value
/// equs + cross-seam address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
    doctor: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = level_dir();
    let main = parse_file(&dir.join("camera.emp"));
    let types_file = parse_file(&dir.parent().unwrap().join("system/types.emp"));
    let constants_file = parse_file(&dir.parent().unwrap().join("system/constants.emp"));
    let structs_file = parse_file(&dir.parent().unwrap().join("structs.emp"));
    let sst_file = parse_file(&dir.parent().unwrap().join("objects/sst.emp"));
    let coords_file = parse_file(&dir.parent().unwrap().join("coords.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: types_file
            .items
            .into_iter()
            .chain(constants_file.items)
            .chain(structs_file.items)
            .chain(sst_file.items)
            .chain(coords_file.items)
            .chain(main.items)
            .collect(),
        docs: main.docs.clone(),
    };

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        // engine.constants (chained as an ambient above) computes PAGE_FRAMES_CLAMP
        // from the STRESS_EVICT build define and comptime-ensures it. The native
        // default (STRESS_EVICT=0) is now seeded by `lower_module_inner` itself (the
        // one lowering funnel), so this single-region lower needs no per-port inject.
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) =
        lower_module_with_contracts(&file, &opts, &sonic4_contract_env());
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "camera.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = camera_value_equs(doctor);
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    sections.extend(camera_addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// camera.emp's drift guards + the prepended twins' guards must PASS.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "camera.emp drift guards must all PASS: {diags:?}"
    );
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
    assert_drift_guards(&resolved, &link_asserts);

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("camera").expect("linked image must carry camera");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("camera ({shape})"));
}

#[test]
fn camera_region_matches_reference() {
    run(false);
}

#[test]
fn camera_debug_region_matches_reference() {
    run(true);
}

// `doctored_cam_max_y_step_fires_its_guard` RETIRED at the conv-b constants-tail
// flip: CAM_MAX_Y_STEP flipped from camera.emp's local mirror to `use engine.constants`,
// so no `ensure(extern("CAM_MAX_Y_STEP") == …)` wall survives for the doctored probe to
// fire. Its protection re-homes to the six-target byte-identity gate.

/// Negative probe (game-mirror class, kill-list row 18): a DOCTORED
/// `PSTATE_JUMP` truth (9 game-side while camera.emp's gated mirror says 8)
/// must fire the jump-lock block's drift guard NAMING the constant.
#[test]
fn doctored_pstate_jump_fires_its_guard() {
    let aeon = aeon_dir();
    if !aeon.join("engine/level/camera.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let (resolved, _, link_asserts) = compile_real_file(false, Some(("PSTATE_JUMP", "9")));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(!fired.is_empty(), "the doctored PSTATE_JUMP truth must fire a drift guard");
    assert!(
        fired.iter().any(|d| d.message.contains("PSTATE_JUMP")),
        "the fired guard must NAME the drifted constant: {fired:?}"
    );
}

/// The engine/game-split property gate: with `-D GAME_CAMERA_JUMP_LOCK=0` the
/// module must lower AND link with NO game symbols provided — the equ pairs
/// below deliberately OMIT `_pl_state`/`PSTATE_JUMP`/`PSTATE_ROLLJUMP`, so any
/// ungated reference to them fails resolution here. (No byte gate: no
/// reference ROM builds the =0 shape; compile+link-clean is the bar.)
#[test]
fn jump_lock_off_compiles_without_game_symbols() {
    let aeon = aeon_dir();
    if !aeon.join("engine/level/camera.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let dir = level_dir();
    let main = parse_file(&dir.join("camera.emp"));
    let types_file = parse_file(&dir.parent().unwrap().join("system/types.emp"));
    let constants_file = parse_file(&dir.parent().unwrap().join("system/constants.emp"));
    let structs_file = parse_file(&dir.parent().unwrap().join("structs.emp"));
    let sst_file = parse_file(&dir.parent().unwrap().join("objects/sst.emp"));
    let coords_file = parse_file(&dir.parent().unwrap().join("coords.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: types_file
            .items
            .into_iter()
            .chain(constants_file.items)
            .chain(structs_file.items)
            .chain(sst_file.items)
            .chain(coords_file.items)
            .chain(main.items)
            .collect(),
        docs: main.docs.clone(),
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        // STRESS_EVICT=0 (the chained engine.constants ambient comptime-ensures a
        // STRESS_EVICT-derived const) is now seeded by `lower_module_inner` itself.
        defines: vec![("DEBUG".to_string(), 0)],
    };
    let (module, ldiags) = lower_module_with_contracts(&file, &opts, &camera_contract_env(false));
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "camera.emp (JUMP_LOCK=0) lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(false)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    // NO game equs: only the engine twins' truth values + the RAM carriers.
    let mut pairs: Vec<(&str, &str)> = vec![("CAM_MAX_Y_STEP", "16")];
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    pairs.extend(sigil_harness::test_support::sst_field_equs());
    let mut equs = sigil_harness::test_support::assemble_equ_pairs(&pairs);
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);
    sections.extend(camera_addr_labels(false));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed (JUMP_LOCK=0): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed (JUMP_LOCK=0 must not need game symbols): {d:?}"));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "JUMP_LOCK=0 drift guards must not reference game symbols: {diags:?}"
    );
    // The =0 shape must be SHORTER than the =1 reference region (the gated
    // block is genuinely elided, not merely unreferenced).
    let section = linked.section("camera").expect("linked image must carry camera");
    assert!(
        section.bytes.len() < region_len(false),
        "JUMP_LOCK=0 shape must elide the gated block (got {} bytes, =1 region is {})",
        section.bytes.len(),
        region_len(false)
    );
}
