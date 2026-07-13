//! Tranche 10 (1b) — the REAL `core.emp` port, region-level byte gate.
//!
//! `collision_port.rs`'s / `animate_port.rs`'s sibling for the object-system
//! core: compiles the ACTUAL ported file from aeon's tree —
//! `engine/objects/core.emp` — through the production parse -> lower -> place
//! -> resolve -> link pipeline, and asserts the `core` section's flattened
//! bytes equal the reference ROM window at the pinned addresses, in BOTH build
//! shapes.
//!
//! ## What this port exercises that the prior nine did not
//!
//! - **imm-link + pinned-abs.w in ONE instruction** (the tranche-10 shipped
//!   feature, commit 080aba5): the four free-stack SP writes —
//!   `move.w #extern("Dynamic_Free_Stack")+NUM_DYNAMIC*2, (Dynamic_Free_SP).w`
//!   and the two `cmpi.w #extern("Free_Stack"), (Free_SP).w` bound-checks —
//!   emit TWO independent fixups (Value16Be imm @2, Abs16Be dest @4).
//! - **Shape-DEPENDENT region LENGTH from a whole debug-only proc** — plain
//!   0x1C4, debug 0x2EC. The 0x128 surplus is the two `if DEBUG == 1 { bsr.w
//!   Debug_AssertObjLoop }` call sites + the `Debug_AssertObjLoop` proc (three
//!   `assert` construct expansions, rings.emp `.full` precedent).
//!   The proc emits ZERO bytes in the plain shape (its whole body is inside
//!   `if DEBUG == 1 {}`).
//! - **The largest cross-seam RAM surface of the campaign** — Object_RAM,
//!   Dynamic/System/Effect_Slots, Dynamic/Effect_Free_{Stack,SP},
//!   Object_RAM_End, Spawn_Count, Game_Paused, Camera_X/Y (bare abs.w EAs) +
//!   the proc seams Draw_Sprite / MDDBG__* (debug).
//! - **`#(extern("A")-extern("B"))/4-1` link-time symbol-difference /division**
//!   as a `.w` immediate (the clear-loop count).
//! - **`sizeof(Sst)` as both a displacement (`sizeof(Sst)(a0)`) and inside a
//!   comptime immediate (`#extern("Effect_Slots")+sizeof(Sst)*NUM_EFFECTS`).**
//!
//! ## Reference windows
//! (sourced from `sigil_harness::pins` — regenerate via repin)
//!
//! Plain (map base `$2794`): `s4.bin[0x2794..0x2958]` (0x1C4 bytes).
//! Debug (map base `$2926`): `s4.debug.bin[0x2926..0x2C12]` (0x2EC bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, the gates SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test core_port
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

/// The engine-constants twin's guard count, derived from the shared truth list.
fn twin_guards() -> usize {
    sigil_harness::test_support::engine_constant_equs().len()
}

/// Per-shape geometry + TRUE cross-seam VMAs (sourced from
/// `sigil_harness::pins` — regenerate via repin). The DEBUG shape references
/// the two MDDBG__* error-handler entries the assert construct expansions jump to;
/// the plain shape does not (the whole proc is elided).
struct Shape {
    base: u32,
    len: usize,
    /// `(name, vma)` for every INBOUND label this shape references.
    labels: &'static [(&'static str, u32)],
}

const PLAIN: Shape = Shape {
    base: pins::CORE.plain_base,
    len: pins::CORE.plain_len,
    labels: &[
        ("Object_RAM", pins::OBJECT_RAM.plain),
        ("Dynamic_Slots", pins::DYNAMIC_SLOTS.plain),
        ("System_Slots", pins::SYSTEM_SLOTS.plain),
        ("Effect_Slots", pins::EFFECT_SLOTS.plain),
        ("Object_RAM_End", pins::OBJECT_RAM_END.plain),
        ("Dynamic_Free_Stack", pins::DYNAMIC_FREE_STACK.plain),
        ("Dynamic_Free_SP", pins::DYNAMIC_FREE_SP.plain),
        ("Effect_Free_Stack", pins::EFFECT_FREE_STACK.plain),
        ("Effect_Free_SP", pins::EFFECT_FREE_SP.plain),
        ("Player_1", pins::PLAYER_1.plain),
        ("Spawn_Count", pins::SPAWN_COUNT.plain),
        ("Game_Paused", pins::GAME_PAUSED.plain),
        ("Camera_X", pins::CAMERA_X.plain),
        ("Camera_Y", pins::CAMERA_Y.plain),
        ("Draw_Sprite", pins::DRAW_SPRITE.plain),
        // object-pool occupancy — the dynamic live-list (spawn-order maintenance)
        ("Dynamic_Live", pins::DYNAMIC_LIVE.plain),
        ("Dynamic_Live_Count", pins::DYNAMIC_LIVE_COUNT.plain),
        ("Dynamic_Live_Dirty", pins::DYNAMIC_LIVE_DIRTY.plain),
    ],
};

const DEBUG: Shape = Shape {
    base: pins::CORE.debug_base,
    len: pins::CORE.debug_len,
    labels: &[
        ("Object_RAM", pins::OBJECT_RAM.debug),
        ("Dynamic_Slots", pins::DYNAMIC_SLOTS.debug),
        ("System_Slots", pins::SYSTEM_SLOTS.debug),
        ("Effect_Slots", pins::EFFECT_SLOTS.debug),
        ("Object_RAM_End", pins::OBJECT_RAM_END.debug),
        ("Dynamic_Free_Stack", pins::DYNAMIC_FREE_STACK.debug),
        ("Dynamic_Free_SP", pins::DYNAMIC_FREE_SP.debug),
        ("Effect_Free_Stack", pins::EFFECT_FREE_STACK.debug),
        ("Effect_Free_SP", pins::EFFECT_FREE_SP.debug),
        ("Player_1", pins::PLAYER_1.debug),
        ("Spawn_Count", pins::SPAWN_COUNT.debug),
        ("Game_Paused", pins::GAME_PAUSED.debug),
        ("Camera_X", pins::CAMERA_X.debug),
        ("Camera_Y", pins::CAMERA_Y.debug),
        ("Draw_Sprite", pins::DRAW_SPRITE.debug),
        // object-pool occupancy — the dynamic live-list (spawn-order maintenance)
        ("Dynamic_Live", pins::DYNAMIC_LIVE.debug),
        ("Dynamic_Live_Count", pins::DYNAMIC_LIVE_COUNT.debug),
        ("Dynamic_Live_Dirty", pins::DYNAMIC_LIVE_DIRTY.debug),
        // A2 walk-live rail (item 1) — DEBUG-only flag set/cleared by the walkers
        // and asserted clear at CompactDynamicLive entry.
        ("Dynamic_Live_Walking", pins::DYNAMIC_LIVE_WALKING),
        // DEBUG-only: the assert construct expansions jsr/jmp these.
        ("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER),
        ("MDDBG__ErrorHandler_PagesController", pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER),
    ],
};

/// Parse one `.emp` file to an AST, failing loudly on parse errors.
fn parse_file(path: &std::path::Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, diags) = parse_str(&src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {diags:?}",
        path.display()
    );
    file
}

/// One synthetic file: `deps`' items prepended to `main`'s own, under `main`'s
/// module header (the ambient-injection technique).
fn with_ambient(
    deps: Vec<sigil_frontend_emp::ast::File>,
    main: sigil_frontend_emp::ast::File,
) -> sigil_frontend_emp::ast::File {
    let mut items = Vec::new();
    for d in deps {
        items.extend(d.items);
    }
    items.extend(main.items);
    sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    }
}

/// The AS-side value seam: SST struct equs + the engine constants twin's 34
/// (the tranche-10 object-core block grew it 30 → 34). `override_pair` doctors
/// exactly one entry (the drift-probe seam).
fn as_constant_equs_with(override_pair: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs = sigil_harness::test_support::sst_field_equs();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    if let Some((name, rhs)) = override_pair {
        let slot = pairs
            .iter_mut()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("override: `{name}` is not in the equ blob"));
        slot.1 = rhs;
    }
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// One synthetic AS-side label phased at `vma` — a `dc.b 0` carrier whose LABEL
/// address is load-bearing (all the abs.w RAM EAs and the proc jsr/jmp targets
/// must resolve to the real per-shape addresses).
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// The AS-side OUTBOUND consumers — bare `jsr RunObjects` / `jsr DeleteObject`
/// from an AS unit (undefined in-unit; the `.emp` owns them). Proves the
/// `pub proc` exports surface as bare link symbols relaxing to abs.w.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tjsr     RunObjects\n\
               \tjsr     DeleteObject\n\
               \trts\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// The map: a `text` region for the zero-byte default-section carrier, and the
/// real `core` region pinned at the per-shape base + per-shape len.
fn map_toml(base: u32, len: usize) -> String {
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
         name = \"core\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// Compile the real `core.emp` with its ambient dependencies (types + sst +
/// constants) and the given build-shape defines, place it at the per-shape
/// base, append the synthetic cross-seam sections, and link.
fn compile_real_file(
    shape: &Shape,
    defines: &[(&str, i128)],
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    compile_real_file_with(shape, defines, None)
}

/// `compile_real_file` with the drift-probe equ-override seam exposed.
fn compile_real_file_with(
    shape: &Shape,
    defines: &[(&str, i128)],
    override_pair: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let types = parse_file(&aeon.join("engine/system/types.emp"));
    let sst = parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = parse_file(&aeon.join("engine/system/constants.emp"));
    let core = parse_file(&aeon.join("engine/objects/core.emp"));

    let file = with_ambient(vec![types, sst, constants], core);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "core.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(shape.base, shape.len)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut lma = 0x0100_0000u32;
    let mut groups: Vec<Vec<Section>> = vec![as_constant_equs_with(override_pair)];
    for (name, vma) in shape.labels {
        groups.push(as_label_at(name, *vma));
    }
    groups.push(as_outbound_consumer());
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
    (resolved, linked, link_asserts)
}

/// All prepended drift guards must be captured and PASS: sst.emp's 30 SST_*
/// pins + constants.emp's 34 (30 pre-tranche + the object-core block's 4).
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = sigil_harness::test_support::guard_assert_count(link_asserts);
    let want = 30 + twin_guards();
    assert_eq!(
        guards, want,
        "sst.emp's 30 + constants.emp's {} drift guards must be captured",
        twin_guards()
    );
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );
}

/// On mismatch, report the first differing offset plus context on each side.
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

/// The region reference gate + cross-seam pins + the outbound bare-name proof +
/// the drift guards, shared body.
fn reference_gate(shape: &Shape, rom_name: &str, debug_on: bool) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let defines: Vec<(&str, i128)> = vec![("DEBUG", i128::from(debug_on))];
    let (resolved, linked, link_asserts) = compile_real_file(shape, &defines);
    assert_drift_guards(&resolved, &link_asserts);

    let base = shape.base as usize;
    let section = linked.section("core").expect("linked image must carry core");
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + shape.len],
        &format!("core vs {rom_name}[{base:#x}..{:#x}]", base + shape.len),
    );

    // Cross-seam RAM pin: the first instruction of InitObjectRAM is
    // `lea Object_RAM, a0` (Object_RAM == Player_1), abs.w word at region
    // offset 2 must equal the low half of Object_RAM's VMA.
    let obj_ram = shape.labels.iter().find(|(n, _)| *n == "Object_RAM").unwrap().1;
    let obj_word = u16::from_be_bytes([section.bytes[2], section.bytes[3]]);
    assert_eq!(
        obj_word,
        (obj_ram & 0xFFFF) as u16,
        "`lea Object_RAM, a0` must carry Object_RAM's abs.w address"
    );

    // Outbound bare-name proof: `jsr RunObjects` / `jsr DeleteObject` must
    // relax to the abs.w encoding (`4EB8 vma`). RunObjects/DeleteObject are NOT
    // the region base (InitObjectRAM/AllocDynamic/AllocEffect precede them), so
    // resolve their VMAs from the resolved `core` section's labels. The
    // consumer is the LAST synthetic group: equ blob + N labels + consumer.
    let consumer_lma = 0x0100_0000u32 + (1 + shape.labels.len() as u32) * 0x10_0000;
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == consumer_lma)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let core_sec = resolved.iter().find(|s| s.name == "core").expect("resolved core section");
    let label_vma = |name: &str| -> u32 {
        let off = core_sec
            .labels
            .iter()
            .find(|l| l.name == name)
            .unwrap_or_else(|| panic!("core section must define `{name}`"))
            .offset;
        core_sec.vma_origin() + off
    };
    let run_objects = label_vma("RunObjects");
    assert_eq!(
        &consumer.bytes[0..4],
        &[0x4E, 0xB8, (run_objects >> 8) as u8, run_objects as u8],
        "bare-name proof: `jsr RunObjects` must relax to abs.w at RunObjects' VMA"
    );
    let delete_object = label_vma("DeleteObject");
    assert_eq!(
        &consumer.bytes[4..8],
        &[0x4E, 0xB8, (delete_object >> 8) as u8, delete_object as u8],
        "bare-name proof: `jsr DeleteObject` must relax to abs.w at DeleteObject's VMA"
    );
}

/// (plain) the `core` region == `s4.bin[0x2794..0x2958]` (0x1C4 bytes).
#[test]
fn core_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin", false);
}

/// (debug) the `core` region == `s4.debug.bin[0x2926..0x2C12]` (0x2EC bytes).
#[test]
fn core_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin", true);
}

// ── The DEBUG-shape divergence proof ─────────────────────────────────────────
//
// NO AS-twin oracle for core (unlike animate_port). core.asm uses two asl
// constructs the sigil AS FRONT-END (`sigil_frontend_as`) does not model — the
// `ifdebug` macro and a `$FFFFxxxx` RAM-address word immediate — so it cannot
// be re-assembled through the harness AS path the way include-free animate.asm
// was. That oracle's purpose (a drift guard re-reading the reference source
// every run) is instead served — MORE strongly — by the two reference gates
// above: they diff against the REAL asl-built ROMs in BOTH shapes, so any
// core.asm change moves the ROM and trips the gate. The proof below pins the
// shape-dependent LENGTH itself: plain 0x1C4, debug 0x2EC (the 0x128 surplus is
// the two `if DEBUG == 1 { bsr.w Debug_AssertObjLoop }` call sites + the
// Debug_AssertObjLoop proc), i.e. the .emp's `if DEBUG == 1 {}` blocks mirror
// core.asm's `ifdef __DEBUG__` divergence exactly.

/// The `DEBUG` build-shape input drives the region length: the plain shape
/// elides the whole `Debug_AssertObjLoop` proc + its two call sites (the proc's
/// body is entirely inside `if DEBUG == 1 {}`, so it emits ZERO bytes).
#[test]
fn debug_shape_length_diverges() {
    let aeon = aeon_dir();
    if !aeon.join("engine/objects/core.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let plain = {
        let (_, linked, _) = compile_real_file(&PLAIN, &[("DEBUG", 0)]);
        linked.section("core").expect("core").bytes.len()
    };
    let debug = {
        let (_, linked, _) = compile_real_file(&DEBUG, &[("DEBUG", 1)]);
        linked.section("core").expect("core").bytes.len()
    };
    assert_eq!(plain, pins::CORE.plain_len, "plain shape emits 0x1C4 bytes");
    assert_eq!(debug, pins::CORE.debug_len, "debug shape emits 0x2EC bytes");
    assert!(
        debug > plain,
        "the DEBUG shape must be longer — the Debug_AssertObjLoop proc + its two \
         call sites exist only under `if DEBUG == 1 {{}}`"
    );
}

// ── The twin-mirror drift probe (the 4 new constants ride core's gate) ───────

/// A DOCTORED twin truth (`CULL_DISTANCE_X` = $301 AS-side while constants.emp
/// says $300) must fire the twin's `ensure(extern(…))` guard NAMING the
/// constant — proving the tranche's FOUR new object-core-block guards ride
/// core's gate like the originals, paired with the undoctored control (the
/// reference gates above).
#[test]
fn doctored_twin_mirror_fires_its_guard() {
    let aeon = aeon_dir();
    if !aeon.join("engine/objects/core.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let defines: Vec<(&str, i128)> = vec![("DEBUG", 0)];
    let (resolved, _, link_asserts) =
        compile_real_file_with(&PLAIN, &defines, Some(("CULL_DISTANCE_X", "$301")));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(
        !fired.is_empty(),
        "the doctored CULL_DISTANCE_X truth must fire constants.emp's drift guard"
    );
    assert!(
        fired.iter().any(|d| d.message.contains("CULL_DISTANCE_X")),
        "the fired guard must NAME the drifted constant: {fired:?}"
    );
}
