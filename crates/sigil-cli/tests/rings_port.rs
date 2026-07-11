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
//! - **`dc.b` in a proc body (H8)** — the transliterated `assert.b` expansion
//!   carries its FSTRING string/flag data as code-embedded `dc.b` statements
//!   between the `jsr (MDDBG__ErrorHandler).l` and the resume label.
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
//! twin (24 after this tranche's rings/sprites growth) + the FOUR game-owned
//! ring mirrors `rings.emp` guards locally (`MAX_RING_BUFFER`,
//! `RING_BUFFER_ENTRY_SIZE`, `RING_WIDTH`, `VRAM_RING_PLACEHOLDER` — truth:
//! `games/sonic4/config/constants.asm`, kill-list row 18). INBOUND labels at
//! true per-shape VMAs: seven `Ring_*` RAM cells, `Camera_X`/`Camera_Y`,
//! `Player_1` (GAME RAM, moves with `__DEBUG__`), plus the ROM code targets
//! `Collected_MarkRing`, `EntityWindow_EntryForSection`, `EntityLoaded_Clear`,
//! `Sound_PlayRing`, and (debug shape only) the two `MDDBG__ErrorHandler*`
//! entry points the assert transliteration jumps into.
//!
//! OUTBOUND: all five procs are `pub` (callers: entity_window.asm,
//! sprites.asm, game states); a synthetic `bsr.w RingCollision` consumer
//! proves the exports surface as bare link symbols at per-shape addresses.
//!
//! ## Reference windows (2026-07-10 pins, from the master listings)
//!
//! Plain (map base `$31F0`): `s4.bin[0x31F0..0x33A4]` (0x1B4 bytes).
//! Debug (map base `$34AA`): `s4.debug.bin[0x34AA..0x36BA]` (0x210 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, the gates SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test rings_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
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

/// Per-shape geometry + TRUE cross-seam VMAs (2026-07-10 pins, both listings).
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
    base: 0x31F0,
    len: 0x1B4,
    ringcol_off: 0x112,
    labels: &[
        ("Ring_Buffer", 0xFFFF_A8F4),
        ("Ring_Count", 0xFFFF_ABF4),
        ("Ring_HighWater", 0xFFFF_ABF5),
        ("Ring_Add_Dropped", 0xFFFF_ABF6),
        ("Ring_Counter", 0xFFFF_AC60),
        ("Ring_Anim_Frame", 0xFFFF_AC62),
        ("Ring_Anim_Timer", 0xFFFF_AC63),
        ("Camera_X", 0xFFFF_A11E),
        ("Camera_Y", 0xFFFF_A122),
        ("Player_1", 0xFFFF_89EE),
        ("Collected_MarkRing", 0x3428),
        ("EntityWindow_EntryForSection", 0x364C),
        ("EntityLoaded_Clear", 0x3638),
        ("Sound_PlayRing", 0x5EFC),
    ],
};

const DEBUG: Shape = Shape {
    base: 0x34AA,
    len: 0x210,
    ringcol_off: 0x16E,
    labels: &[
        ("Ring_Buffer", 0xFFFF_A916),
        ("Ring_Count", 0xFFFF_AC16),
        ("Ring_HighWater", 0xFFFF_AC17),
        ("Ring_Add_Dropped", 0xFFFF_AC18),
        ("Ring_Counter", 0xFFFF_AC82),
        ("Ring_Anim_Frame", 0xFFFF_AC84),
        ("Ring_Anim_Timer", 0xFFFF_AC85),
        ("Camera_X", 0xFFFF_A140),
        ("Camera_Y", 0xFFFF_A144),
        ("Player_1", 0xFFFF_8A10),
        ("Collected_MarkRing", 0x37A0),
        ("EntityWindow_EntryForSection", 0x3C82),
        ("EntityLoaded_Clear", 0x3C0C),
        ("Sound_PlayRing", 0x73BA),
        // Debug shape only: the assert transliteration's error-handler entry
        // points (values read from the reference ROM's own jsr/jmp operands).
        ("MDDBG__ErrorHandler", 0x6_644C),
        ("MDDBG__ErrorHandler_PagesController", 0x6_7212),
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
    let aabb = parse_file(&aeon.join("engine/objects/aabb.emp"));
    let rings = parse_file(&aeon.join("engine/objects/rings.emp"));

    let file = with_ambient(vec![types, sst, constants, aabb], rings);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
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
/// pins + constants.emp's 24 (18 pre-tranche + the rings/sprites block's 6) +
/// rings.emp's own 4 game-owned mirrors = 58.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = sigil_harness::test_support::guard_assert_count(link_asserts);
    assert_eq!(guards, 64, "sst.emp's 30 + constants.emp's 30 (tranche-9 animation block) + rings.emp's 4 drift guards must be captured");
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
    // resolves to base + ringcol_off. The consumer is the LAST synthetic
    // group: equ blob + N labels + consumer.
    let consumer_lma = 0x0100_0000u32 + (1 + shape.labels.len() as u32) * 0x10_0000;
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == consumer_lma)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let expected_disp =
        (shape.base as i64 + shape.ringcol_off as i64 - (consumer.lma as i64 + 2)) as i16;
    assert_eq!(
        disp, expected_disp,
        "bare-name proof: `bsr.w RingCollision` must resolve to base + ringcol_off"
    );
}

/// (plain) the `rings` region == `s4.bin[0x31F0..0x33A4]` — DEBUG=0.
#[test]
fn rings_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin", 0);
}

/// (debug) the `rings` region == `s4.debug.bin[0x34AA..0x36BA]` — DEBUG=1,
/// including the transliterated assert block and its `dc.b` FSTRING data.
#[test]
fn rings_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin", 1);
}

// ── The SND combo probe ─────────────────────────────────────────────────────

/// The AS-twin oracle for the SOUND_DRIVER_ENABLED dimension: aabb.inc +
/// rings.asm (the include line replaced by the .inc text), assembled through
/// the sigil AS front-end at the PLAIN base with the same equ prelude the .emp
/// gets, per-combo defines. DEBUG stays OFF in the matrix — expanding the
/// `assert.b` needs the whole debugger.asm macro tower, and the DEBUG
/// dimension is already byte-gated by the debug REFERENCE gate above; the one
/// uncovered combo is (DEBUG=1, SND=0), a debug-silent build no pin exists
/// for. Recorded, not hidden.
fn as_twin_bytes(snd_on: bool) -> Vec<u8> {
    let aeon = aeon_dir();
    let inc = std::fs::read_to_string(aeon.join("engine/objects/aabb.inc"))
        .expect("aabb.inc must be readable");
    let rings_src = std::fs::read_to_string(aeon.join("engine/objects/rings.asm"))
        .expect("rings.asm must be readable");
    let rings_body = rings_src.replace("    include \"engine/objects/aabb.inc\"", "");

    let mut prelude = String::from("cpu 68000\nsupmode on\n");
    let mut pairs = sigil_harness::test_support::sst_field_equs();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(game_ring_equs());
    for (name, rhs) in pairs {
        prelude.push_str(&format!("{name} = {rhs}\n"));
    }
    for (name, vma) in PLAIN.labels {
        prelude.push_str(&format!("{name} = ${vma:X}\n"));
    }
    let src = format!("{prelude}{inc}\norg ${:X}\n{rings_body}\n", PLAIN.base);

    let mut defines: Vec<(String, i64)> = Vec::new();
    if snd_on {
        defines.push(("SOUND_DRIVER_ENABLED".to_string(), 1));
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
        .find(|s| s.lma == PLAIN.base && !s.bytes.is_empty())
        .unwrap_or_else(|| panic!("AS twin must emit a section at {:#x}", PLAIN.base));
    sec.bytes.clone()
}

/// SND 0/1 at DEBUG=0: the .emp vs the AS-twin oracle, module-level. This is
/// the conditional-MIRRORING drift guard (the oracle re-reads the real
/// rings.asm every run).
#[test]
fn snd_combo_matches_as_twin() {
    let aeon = aeon_dir();
    if !aeon.join("engine/objects/rings.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    for snd_on in [true, false] {
        let defines: Vec<(&str, i128)> =
            vec![("DEBUG", 0), ("SOUND_DRIVER_ENABLED", i128::from(snd_on))];
        let (_, linked, _) = compile_real_file(&PLAIN, &defines);
        let section = linked.section("rings").expect("linked image must carry rings");
        let expected = as_twin_bytes(snd_on);
        assert_region_matches(
            &section.bytes,
            &expected,
            &format!("rings combo (snd={snd_on}) vs AS twin"),
        );
    }
}

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

// ── The game-mirror drift probe (kill-list row 18's guard) ──────────────────

/// A DOCTORED game-owned mirror truth (`MAX_RING_BUFFER` = 64 AS-side while
/// rings.emp says 128) must fire rings.emp's own `ensure(extern(…))` guard
/// NAMING the constant — paired with the undoctored control that passes
/// through the same plumbing (the reference gates above).
#[test]
fn doctored_game_mirror_fires_its_guard() {
    let aeon = aeon_dir();
    if !aeon.join("engine/objects/rings.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let defines: Vec<(&str, i128)> = vec![("DEBUG", 0), ("SOUND_DRIVER_ENABLED", 1)];
    let (resolved, _, link_asserts) =
        compile_real_file_with(&PLAIN, &defines, Some(("MAX_RING_BUFFER", "64")));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(
        !fired.is_empty(),
        "the doctored MAX_RING_BUFFER truth must fire rings.emp's drift guard"
    );
    assert!(
        fired.iter().any(|d| d.message.contains("MAX_RING_BUFFER")),
        "the fired guard must NAME the drifted constant: {fired:?}"
    );
}
