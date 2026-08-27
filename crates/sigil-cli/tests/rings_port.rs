//! Tranche 8 — the REAL `rings.emp` port, region-level byte gate.
//!
//! `collision_port.rs`'s sibling for the EIGHTH code port: compiles the ACTUAL
//! ported file from aeon's tree — `engine/objects/rings.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline, and asserts
//! the `rings` section's flattened bytes equal the reference ROM window at the
//! pinned addresses, in BOTH build shapes.
//!
//! ## What this port exercises that the prior seven did not
//!
//! - **The FIRST shape-dependent-LENGTH region** — the `__DEBUG__` assert block
//!   in `RingBuffer_Add.full` exists only in the debug shape (plain 0x1B4,
//!   debug 0x210 bytes), so `Shape` carries a per-shape `len`, not the usual
//!   shared constant.
//! - **`dc.b` in a proc body (H8)** — the `assert.b` construct's DEBUG-shape
//!   expansion carries its FSTRING string/flag data as code-embedded `dc.b`
//!   bytes between the `jsr (MDDBG__ErrorHandler).l` and the resume label.
//! - **Comptime-`if` build shapes in a BYTE-GATED engine region** — `DEBUG`
//!   and `SOUND_DRIVER_ENABLED` (`-D NAME=0|1`) mirror the AS twin's `ifdef`s;
//!   the reference gates run (0,1) and (1,1), the combo probe below covers the
//!   SND dimension against a freshly-assembled AS-twin oracle.
//! - **The zero-disp collapse through the F1 splice (row 13's promise)** —
//!   `aabb_axis_test(d4, a0, 0, …)` must emit `sub.w (a0), d1` (mode-(An), no
//!   extension word) for asl parity; collision only exercised NONZERO
//!   `offsetof` displacements through the splice. See
//!   `zero_disp_collapse_probe`.
//! - **A REUSED proc-local label template argument** — both aabb splices in
//!   `RingCollision` take the same `.no_hit`; the .inc twin needed its `utag`
//!   param to disambiguate, hygiene makes the reuse free.
//!
//! ## Cross-seam symbols
//!
//! INBOUND equs (values): the SST_* struct-equ seam + the engine constants
//! twin (24 after this tranche's rings/sprites growth) + the game-owned ring
//! mirrors `rings.emp` guards locally (`MAX_RING_BUFFER`, `RING_WIDTH`,
//! `VRAM_RING_PLACEHOLDER` — truth: `games/sonic4/config/constants.asm`,
//! kill-list row 18). `RING_BUFFER_ENTRY_SIZE` LEFT this set at aeon review item
//! 30: it is an engine-owned FORMAT, not a game knob, so it moved to
//! engine.constants and its local guard retired with the duplication. INBOUND labels at
//! true per-shape VMAs: seven `Ring_*` RAM cells, `Camera_X`/`Camera_Y`,
//! `Player_1` (GAME RAM, moves with `__DEBUG__`), plus the ROM code targets
//! `Collected_MarkRing`, `EntityWindow_EntryForSection`, `EntityLoaded_Clear`,
//! `Sound_PlayRing`, and (debug shape only) the two `MDDBG__ErrorHandler*`
//! entry points the assert construct's expansion jumps into.
//!
//! OUTBOUND: all five procs are `pub` (callers: entity_window.asm,
//! sprites.asm, game states); a synthetic `bsr.w RingCollision` consumer
//! proves the exports surface as bare link symbols at per-shape addresses.
//!
//! ## Reference windows (2026-07-10 pins, from the master listings)
//! (sourced from `sigil_harness::pins` — regenerate via repin)
//!
//! Plain (map base `$3070`): `s4.bin[0x3070..0x3224]` (0x1B4 bytes).
//! Debug (map base `$332A`): `s4.debug.bin[0x332A..0x353A]` (0x210 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, the gates SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test rings_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, lower_module_with_contracts, LowerOptions};
use sigil_harness::pins;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

/// The first LMA of the harness-private synthetic groups (the equ blob, the
/// cross-seam label pins, the outbound consumer) — far above the mapped regions,
/// one 0x10_0000 stride apart.
const HARNESS_PRIVATE_LMA_BASE: u32 = 0x0100_0000;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// Per-shape geometry + TRUE cross-seam VMAs (sourced from
/// `sigil_harness::pins` — regenerate via repin).
/// Rings is the FIRST region whose LENGTH is shape-dependent (the debug-only
/// assert block), so `len` lives here rather than in a shared constant.
struct Shape {
    base: u32,
    len: usize,
    /// `RingCollision`'s offset from the region base (`lea (Player_1).w, a2`
    /// spot-check + the outbound consumer's target).
    ringcol_off: usize,
    /// `(name, vma)` for every INBOUND label this shape references.
    labels: &'static [(&'static str, u32)],
}

const PLAIN: Shape = Shape {
    base: pins::RINGS.plain_base,
    len: pins::RINGS.plain_len,
    ringcol_off: pins::RINGCOL_OFF.plain,
    labels: &[
        ("Ring_Buffer", pins::RING_BUFFER.plain),
        ("Ring_Count", pins::RING_COUNT.plain),
        ("Ring_HighWater", pins::RING_HIGH_WATER.plain),
        ("Ring_Add_Dropped", pins::RING_ADD_DROPPED.plain),
        ("Ring_Counter", pins::RING_COUNTER.plain),
        ("Ring_Anim_Frame", pins::RING_ANIM_FRAME.plain),
        ("Ring_Anim_Timer", pins::RING_ANIM_TIMER.plain),
        ("Camera_X", pins::CAMERA_X.plain),
        ("Camera_Y", pins::CAMERA_Y.plain),
        ("Player_1", pins::PLAYER_1.plain),
        ("Collected_MarkRing", pins::COLLECTED_MARK_RING.plain),
        ("EntityWindow_EntryForSection", pins::ENTITY_WINDOW_ENTRY_FOR_SECTION.plain),
        ("EntityLoaded_Clear", pins::ENTITY_LOADED_CLEAR.plain),
        ("Sound_PlayRing", pins::SOUND_PLAY_RING.plain),
    ],
};

const DEBUG: Shape = Shape {
    base: pins::RINGS.debug_base,
    len: pins::RINGS.debug_len,
    ringcol_off: pins::RINGCOL_OFF.debug,
    labels: &[
        ("Ring_Buffer", pins::RING_BUFFER.debug),
        ("Ring_Count", pins::RING_COUNT.debug),
        ("Ring_HighWater", pins::RING_HIGH_WATER.debug),
        ("Ring_Add_Dropped", pins::RING_ADD_DROPPED.debug),
        ("Ring_Counter", pins::RING_COUNTER.debug),
        ("Ring_Anim_Frame", pins::RING_ANIM_FRAME.debug),
        ("Ring_Anim_Timer", pins::RING_ANIM_TIMER.debug),
        ("Camera_X", pins::CAMERA_X.debug),
        ("Camera_Y", pins::CAMERA_Y.debug),
        ("Player_1", pins::PLAYER_1.debug),
        ("Collected_MarkRing", pins::COLLECTED_MARK_RING.debug),
        ("EntityWindow_EntryForSection", pins::ENTITY_WINDOW_ENTRY_FOR_SECTION.debug),
        ("EntityLoaded_Clear", pins::ENTITY_LOADED_CLEAR.debug),
        ("Sound_PlayRing", pins::SOUND_PLAY_RING.debug),
        // Debug shape only: the assert construct's error-handler entry
        // points (values read from the reference ROM's own jsr/jmp operands).
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

/// The GAME-owned ring mirrors' truth values (`games/sonic4/config/
/// constants.asm` — engine.inc game-contract symbols, kill-list row 18).
/// Supplied alongside the engine/SST blob so `rings.emp`'s four local
/// `ensure(extern(…))` guards resolve.
fn game_ring_equs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("MAX_RING_BUFFER", "128"),
        ("RING_BUFFER_ENTRY_SIZE", "6"),
        ("RING_WIDTH", "16"),
        ("VRAM_RING_PLACEHOLDER", "$3E8"),
    ]
}

/// The AS-side value seam: SST struct equs + the engine constants twin's 24 +
/// the four game-owned ring mirrors. `override_pair` doctors exactly one
/// entry (the drift-probe seam — see `doctored_game_mirror_fires_its_guard`).
fn as_constant_equs_with(override_pair: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs = sigil_harness::test_support::sst_field_equs();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(game_ring_equs());
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
/// address is load-bearing (abs.w RAM EAs and bsr.w/jsr targets must sit at the
/// real per-shape addresses).
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// The AS-side OUTBOUND consumer — mirrors a game state's `bsr.w RingCollision`,
/// assembled with the label UNDEFINED in-unit (the `.emp` owns it). Proves the
/// `pub proc` exports surface as bare link symbols.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tbsr.w   RingCollision\n\
               \trts\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// The map: a `text` region for the zero-byte default-section carrier, and the
/// real `rings` region pinned at the per-shape base, sized to the per-shape
/// length.
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
         name = \"rings\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// Compile the real `rings.emp` with its ambient dependencies (types + sst +
/// constants + aabb) and the given build-shape defines, place it at the
/// per-shape base, append the synthetic cross-seam sections, and link.
fn compile_real_file(
    shape: &Shape,
    defines: &[(&str, i128)],
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    compile_real_file_with(shape, defines, None)
}

/// This oracle's build shape, read off the defines it is lowering with — the
/// `DEBUG` value the caller already supplies, not a second flag to keep in step.
fn shape_is_debug(ds: &[(String, i128)]) -> bool {
    ds.iter().find(|(n, _)| n == "DEBUG").is_some_and(|(_, v)| *v != 0)
}

/// The comptime environment the game manifest's binding groups are resolved
/// against: the profile's OWN `emp_defines` (what the reference ROM was built
/// with — `SOUND_DEBUG_HOTKEYS` and friends live only there), with this oracle's
/// own defines layered on top so a shape/probe variation reaches the manifest too.
fn contract_defines(
    profile: &sigil_harness::native::GameProfile,
    ds: &[(String, i128)],
) -> Vec<(String, i128)> {
    let mut out: Vec<(String, i128)> =
        profile.emp_defines.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    for (n, v) in ds {
        match out.iter_mut().find(|(o, _)| o == n) {
            Some(slot) => slot.1 = *v,
            None => out.push((n.clone(), *v)),
        }
    }
    out
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
    // objdef.emp is the home of the `vram_art` comptime fn that rings.emp now
    // imports (`use engine.objects.objdef.{vram_art}` for RING_ART_ATTR) — pull
    // it into the ambient so the cross-module import resolves.
    let objdef = parse_file(&aeon.join("engine/objects/objdef.emp"));
    let aabb = parse_file(&aeon.join("engine/objects/aabb.emp"));
    let rings = parse_file(&aeon.join("engine/objects/rings.emp"));

    let file = with_ambient(vec![types, sst, constants, objdef, aabb], rings);

    // rings.emp takes the game-config values as -D (engine/game split); supply
    // sonic4's canonical values for any a caller didn't already set (a doctored-
    // override caller keeps its value — first-set wins).
    let mut ds: Vec<(String, i128)> = defines.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    for (k, v) in [("MAX_RING_BUFFER", 128i128), ("VRAM_RING_PLACEHOLDER", 0x3E8)] {
        if !ds.iter().any(|(n, _)| n == k) {
            ds.push((k.to_string(), v));
        }
    }
    // rings.emp's `invoke Game.ring_collected` (the collect visual) needs the L1
    // game-contract env the whole-program build binds. It is DERIVED from aeon's
    // own `engine/system/game_contract.emp` + sonic4's `implement Game`, never a
    // stub written here: this oracle compares against the SONIC4 reference ROM,
    // so the binding must be sonic4's actual one, and a hand-written interface
    // cannot see a member the engine grows (`ring_collected` is exactly that).
    let is_debug = shape_is_debug(&ds);
    let profile = sigil_harness::native::sonic4_profile(is_debug);
    let env = sigil_harness::test_support::game_contract_env_from_aeon(
        &aeon,
        &profile,
        &contract_defines(&profile, &ds),
    );

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: ds,
    };
    let (module, ldiags) = lower_module_with_contracts(&file, &opts, &env);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "rings.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(shape.base, shape.len)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut lma = HARNESS_PRIVATE_LMA_BASE;
    let mut groups: Vec<Vec<Section>> = vec![as_constant_equs_with(override_pair)];
    for (name, vma) in shape.labels {
        groups.push(as_label_at(name, *vma));
    }
    // The contract's own bound targets (`invoke Game.ring_collected` lowers to
    // `jsr RingSparkle_Spawn`). The NAMES come from the env, not a list here, and
    // each address comes from the reference ROM's OWN sibling listing — the same
    // build, so the operand this oracle encodes and the one in the reference
    // cannot disagree. A hook the game binds later arrives pinned by construction.
    let listing = aeon.join(if is_debug { "s4.debug.lst" } else { "s4.lst" });
    for sym in sigil_harness::test_support::game_contract_bound_symbols(&env) {
        if let Some(vma) = sigil_harness::test_support::listing_symbol_addr(&listing, &sym) {
            groups.push(as_label_at(&sym, vma));
        }
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

/// All prepended drift guards must be captured and PASS: the engine constants
/// twin (derived via the shared truth list) + rings.emp's own 1 engine-invariant
/// game-owned mirror (RING_WIDTH). RING_BUFFER_ENTRY_SIZE's guard retired at aeon
/// review item 30: it is an engine-owned FORMAT, so it moved to engine.constants and
/// the game configs stopped restating it — one authority, nothing left to cross-check
/// (RING_WIDTH stays a genuine game knob and keeps its guard). sst.emp's SST_* wall
/// retired at the conv-a structs flip; the game-VARYING MAX_RING_BUFFER /
/// VRAM_RING_PLACEHOLDER guards retired at conv-f (they are native.rs `-D`
/// interface values with no `.emp`/AS authority to cross-check — byte-identity is
/// their drift net).
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = sigil_harness::test_support::guard_assert_count(link_asserts);
    let want = sigil_harness::test_support::engine_constant_equs().len() + 1;
    assert_eq!(guards, want, "the constants twin's {} + rings.emp's 1 drift guard must be captured", sigil_harness::test_support::engine_constant_equs().len());
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );
}

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

/// The region reference gate + cross-seam label pins + the outbound bare-name
/// proof + the drift guards, shared body. Reference shapes always run
/// SOUND_DRIVER_ENABLED=1 (both pinned ROMs have sound on).
fn reference_gate(shape: &Shape, rom_name: &str, debug_define: i128) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let defines: Vec<(&str, i128)> =
        vec![("DEBUG", debug_define), ("SOUND_DRIVER_ENABLED", 1)];
    let (resolved, linked, link_asserts) = compile_real_file(shape, &defines);
    assert_drift_guards(&resolved, &link_asserts);

    let base = shape.base as usize;
    let section = linked.section("rings").expect("linked image must carry rings");
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + shape.len],
        &format!("rings vs {rom_name}[{base:#x}..{:#x}]", base + shape.len),
    );

    // Cross-seam label pins: `RingBuffer_Add` opens `moveq #0, d4` +
    // `move.b (Ring_Count).w, d4` — the abs.w word at region offset 4 must be
    // Ring_Count's low half; `RingCollision` opens `lea (Player_1).w, a2` at
    // `ringcol_off` — abs.w word at +2.
    let ring_count = shape.labels.iter().find(|(n, _)| *n == "Ring_Count").unwrap().1;
    let count_word = u16::from_be_bytes([section.bytes[4], section.bytes[5]]);
    assert_eq!(
        count_word,
        (ring_count & 0xFFFF) as u16,
        "`move.b (Ring_Count).w, d4` must carry Ring_Count's abs.w address"
    );
    let player_1 = shape.labels.iter().find(|(n, _)| *n == "Player_1").unwrap().1;
    let player_word = u16::from_be_bytes([
        section.bytes[shape.ringcol_off + 2],
        section.bytes[shape.ringcol_off + 3],
    ]);
    assert_eq!(
        player_word,
        (player_1 & 0xFFFF) as u16,
        "`lea (Player_1).w, a2` must carry Player_1's abs.w address at RingCollision"
    );

    // Outbound bare-name proof: the AS-side `bsr.w RingCollision` fixup
    // resolves to base + ringcol_off. The consumer is appended LAST among the
    // harness-private groups, so it is the one at the highest private LMA —
    // found by that construction rather than by counting the groups ahead of it
    // (a count the contract's own bound-symbol pins now vary).
    let consumer = linked
        .sections
        .iter()
        .filter(|s| s.lma >= HARNESS_PRIVATE_LMA_BASE)
        .max_by_key(|s| s.lma)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let expected_disp =
        (shape.base as i64 + shape.ringcol_off as i64 - (consumer.lma as i64 + 2)) as i16;
    assert_eq!(
        disp, expected_disp,
        "bare-name proof: `bsr.w RingCollision` must resolve to base + ringcol_off"
    );
}

/// (plain) the `rings` region == `s4.bin[0x3070..0x3224]` — DEBUG=0.
#[test]
fn rings_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin", 0);
}

/// (debug) the `rings` region == `s4.debug.bin[0x332A..0x353A]` — DEBUG=1,
/// including the assert construct's DEBUG-shape expansion and its `dc.b`
/// FSTRING data.
#[test]
fn rings_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin", 1);
}

// The SND-combo AS-twin oracle RETIRED (flip Stage-2): rings.asm is deleted —
// the .emp is the only source. The SOUND_DRIVER_ENABLED-dimension coverage is
// subsumed by the native whole-ROM golden gates (sound-ON canonical + Config-B
// sound-OFF); the region gates above pin rings == frozen-golden slice, and the
// t24 game-mirror drift probe below keeps the golden non-vacuous. aabb.inc (not
// a twin) survives as rings.emp's data source. The zero-disp collapse probe
// below still exercises the F1 splice against the real aabb.emp.

// ── The zero-disp collapse probe (row 13's promise) ─────────────────────────

/// `aabb_axis_test(…, a0, 0, …)` must emit `sub.w (a0), d1` — the 2-byte
/// mode-(An) EA, NOT the 4-byte `0(a0)` d16 form — through the F1 splice path
/// (asl collapses zero displacements; byte parity requires the same here).
/// Collision's calls only exercised NONZERO `offsetof` displacements, so this
/// is the splice path's first zero-disp consumer. The probe compiles a
/// synthetic caller against the REAL aabb.emp and asserts the collapsed
/// encoding: `sub.w (a0), d1` (0x9250) directly followed by
/// `move.w d1, d2` (0x3401) — the d16 form would interpose a zero extension
/// word.
#[test]
fn zero_disp_collapse_probe() {
    let aeon = aeon_dir();
    let aabb_path = aeon.join("engine/objects/aabb.emp");
    if !aabb_path.exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let aabb = parse_file(&aabb_path);
    let probe_src = "module probe.zero_disp in probe\n\
                     pub proc Probe () {\n\
                     \taabb_axis_test(d4, a0, 0, d0, d1, d0, d1, d2, .miss)\n\
                     \tnop\n\
                     .miss:\n\
                     \trts\n\
                     }\n";
    let (probe, pdiags) = parse_str(probe_src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "probe parse errors: {pdiags:?}"
    );
    let file = with_ambient(vec![aabb], probe);
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "probe lower errors: {ldiags:?}"
    );
    let map = "fill = 0x00\n[[region]]\nname = \"probe\"\nlma_base = 0x1000\nsize = 0x40\nkind = \"rom\"\n";
    let mapv = sigil_link::load_map(map).expect("probe map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &mapv);
    assert!(pdiags.iter().all(|d| d.level != sigil_span::Level::Error), "{pdiags:?}");
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("probe resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("probe link failed: {d:?}"));
    let bytes = &linked.section("probe").expect("probe section").bytes;
    let collapsed: &[u8] = &[0x92, 0x50, 0x34, 0x01]; // sub.w (a0),d1; move.w d1,d2
    assert!(
        bytes.windows(4).any(|w| w == collapsed),
        "zero-disp splice must collapse `sub.w 0(a0), d1` to `sub.w (a0), d1` — got {bytes:02x?}"
    );
}

// ── The game-mirror drift probe (kill-list row 18's guard) — RETIRED at conv-f ─
//
// The `doctored_game_mirror_fires_its_guard` negative probe doctored MAX_RING_BUFFER
// and asserted rings.emp's `ensure(extern("MAX_RING_BUFFER") == …)` fired. Parcel F
// flipped sonic4's config constants to `.emp`, which cannot declare a `-D` name, so
// MAX_RING_BUFFER / VRAM_RING_PLACEHOLDER stay native.rs `-D` interface values with
// no `.emp`/AS authority — the extern guards were retired. A wrong `-D` MOVES ROM
// bytes, so the undoctored region-match gates above (`rings_region_matches_reference`
// / `_debug`) are the surviving proof (six-target byte-identity is the drift net).
// RING_BUFFER_ENTRY_SIZE / RING_WIDTH keep their guards + could carry a doctored
// probe; MAX_RING_BUFFER's probe has no guard left to fire.
