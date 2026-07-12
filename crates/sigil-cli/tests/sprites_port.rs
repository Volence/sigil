//! Tranche 11 — the REAL `sprites.emp` port, region-level byte gate.
//!
//! The ELEVENTH code port and the first against the finished Spec-2 language.
//! Compiles the ACTUAL ported file from aeon's tree —
//! `engine/objects/sprites.emp` — through the production parse -> lower ->
//! place -> resolve -> link pipeline, and asserts the `sprites` section's
//! flattened bytes equal the reference ROM window at the pinned addresses, in
//! BOTH build shapes.
//!
//! ## What this port exercises
//!
//! - **The heaviest RAM-label surface of the campaign** — twelve engine RAM
//!   cells (`Sprite_Bands`/`Sprite_Band_Counts`/`Sprites_Rendered`/
//!   `Sprite_Table_Buffer`/`Sprite_Table_Dirty`/`Sprite_Cycle_Counter`/
//!   `SpriteMask_{Y,Height,After_Band}`/`Scanline_Band_Sprites` + shared
//!   `Camera_X`/`Camera_Y`), all abs.w EAs. Ten of them SHIFT +$22 in the
//!   debug shape (debug RAM inserted ahead), so the region is same-LENGTH
//!   ($420 both) but the RAM-EA operand bytes DIFFER per shape — each shape
//!   diffs against its own ROM window with its own VMAs.
//! - **`data` interleaved between procs** — `CellOffsets_XFlip` (16-byte flip
//!   width LUT) sits between `Render_Sprites` and `Emit_ObjectPieces`, read
//!   pc-relative (`lea CellOffsets_XFlip(pc), a0`) by the two X-flipping
//!   variants. Decl-order placement lands the data in the region interior.
//! - **One outbound cross-region call** — `Render_Sprites` tail-calls
//!   `DrawRings` (rings region), pinned at its per-shape VMA.
//! - **The row-17 forced flip** — `MAX_VDP_SPRITES`/`VDP_SPRITE_{X,Y}_OFFSET`
//!   were hoisted sprites.asm → engine/constants.asm at this port (the gate
//!   removes sprites.asm's defs, but the gate-off rings.asm twin still reads
//!   them in immediates). The constants twin grew 34 → 49 (render-flag bits,
//!   band/scanline/screen geometry, frame-header offsets).
//!
//! ## Cross-seam symbols
//!
//! INBOUND equs (values): the SST_* struct-equ seam + the engine constants
//! twin (49 after this tranche's 15-const growth). sprites.emp carries NO
//! module-local mirrors (SPRITE_MASK_{SIZE,HEIGHT} are unguarded module
//! consts). INBOUND labels at true per-shape VMAs: `DrawRings` + the twelve
//! RAM cells.
//!
//! OUTBOUND: `InitSpriteSystem`/`Draw_Sprite`/`Render_Sprites` are `pub` —
//! called by core.emp (`Draw_Sprite`) and the game states (`jsr
//! InitSpriteSystem`/`Render_Sprites`). The consumer probe mirrors a game
//! state's bare `jsr InitSpriteSystem` and must land on the abs.w encoding at
//! the region base (`4EB8 base`) for mixed-build parity.
//!
//! ## Reference windows (2026-07-11 pins, from the master listings)
//! (sourced from `sigil_harness::pins` — regenerate via repin)
//!
//! Plain (map base `$2954`): `s4.bin[0x2954..0x2D74]` (0x420 bytes).
//! Debug (map base `$2C0E`): `s4.debug.bin[0x2C0E..0x302E]` (0x420 bytes).
//! Length is shape-INVARIANT (no `__DEBUG__` code in this file); the RAM-EA
//! operand bytes are not.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sprites_port
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

/// Per-shape geometry + TRUE cross-seam VMAs (sourced from
/// `sigil_harness::pins` — regenerate via repin).
struct Shape {
    base: u32,
    len: usize,
    /// `(name, vma)` for every INBOUND label this shape references.
    labels: &'static [(&'static str, u32)],
}

const PLAIN: Shape = Shape {
    base: pins::SPRITES.plain_base,
    len: pins::SPRITES.plain_len,
    labels: &[
        ("DrawRings", pins::DRAW_RINGS.plain),
        ("Sprite_Table_Buffer", pins::SPRITE_TABLE_BUFFER.plain),
        ("Sprite_Table_Dirty", pins::SPRITE_TABLE_DIRTY.plain),
        ("Sprite_Bands", pins::SPRITE_BANDS.plain),
        ("Sprite_Band_Counts", pins::SPRITE_BAND_COUNTS.plain),
        ("Sprites_Rendered", pins::SPRITES_RENDERED.plain),
        ("Sprite_Cycle_Counter", pins::SPRITE_CYCLE_COUNTER.plain),
        ("SpriteMask_Y", pins::SPRITE_MASK_Y.plain),
        ("SpriteMask_Height", pins::SPRITE_MASK_HEIGHT.plain),
        ("SpriteMask_After_Band", pins::SPRITE_MASK_AFTER_BAND.plain),
        ("Scanline_Band_Sprites", pins::SCANLINE_BAND_SPRITES.plain),
        ("Camera_X", pins::CAMERA_X.plain),
        ("Camera_Y", pins::CAMERA_Y.plain),
        ("Camera_X_Biased", pins::CAMERA_X_BIASED.plain),
        ("Camera_Y_Biased", pins::CAMERA_Y_BIASED.plain),
    ],
};

const DEBUG: Shape = Shape {
    base: pins::SPRITES.debug_base,
    len: pins::SPRITES.debug_len,
    labels: &[
        ("DrawRings", pins::DRAW_RINGS.debug),
        ("Sprite_Table_Buffer", pins::SPRITE_TABLE_BUFFER.debug),
        ("Sprite_Table_Dirty", pins::SPRITE_TABLE_DIRTY.debug),
        ("Sprite_Bands", pins::SPRITE_BANDS.debug),
        ("Sprite_Band_Counts", pins::SPRITE_BAND_COUNTS.debug),
        ("Sprites_Rendered", pins::SPRITES_RENDERED.debug),
        ("Sprite_Cycle_Counter", pins::SPRITE_CYCLE_COUNTER.debug),
        ("SpriteMask_Y", pins::SPRITE_MASK_Y.debug),
        ("SpriteMask_Height", pins::SPRITE_MASK_HEIGHT.debug),
        ("SpriteMask_After_Band", pins::SPRITE_MASK_AFTER_BAND.debug),
        ("Scanline_Band_Sprites", pins::SCANLINE_BAND_SPRITES.debug),
        ("Camera_X", pins::CAMERA_X.debug),
        ("Camera_Y", pins::CAMERA_Y.debug),
        ("Camera_X_Biased", pins::CAMERA_X_BIASED.debug),
        ("Camera_Y_Biased", pins::CAMERA_Y_BIASED.debug),
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

/// The AS-side value seam: SST struct equs + the engine constants twin.
/// `override_pair` doctors exactly one entry (the drift-probe seam).
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

/// One synthetic AS-side label phased at `vma` — a `dc.b 0` carrier whose
/// LABEL address is load-bearing (abs.w RAM EAs and the bsr.w DrawRings
/// displacement must resolve to the real per-shape addresses).
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// The AS-side OUTBOUND consumer — mirrors a game state's bare
/// `jsr InitSpriteSystem`, assembled with the label UNDEFINED in-unit (the
/// `.emp` owns it). Proves the `pub proc` export surfaces as a bare link
/// symbol AND that the width relaxation lands on the abs.w encoding at the
/// region base.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tjsr     InitSpriteSystem\n\
               \trts\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// The map: a `text` region for the zero-byte default-section carrier, and the
/// real `sprites` region pinned at the per-shape base.
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
         name = \"sprites\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// Compile the real `sprites.emp` with its ambient dependencies (types + sst +
/// constants), place it at the per-shape base, append the synthetic cross-seam
/// sections, and link. sprites has NO build-shape define dimension.
fn compile_real_file(
    shape: &Shape,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    compile_real_file_with(shape, None)
}

/// `compile_real_file` with the drift-probe equ-override seam exposed.
fn compile_real_file_with(
    shape: &Shape,
    override_pair: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let types = parse_file(&aeon.join("engine/system/types.emp"));
    let sst = parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = parse_file(&aeon.join("engine/system/constants.emp"));
    let sprites = parse_file(&aeon.join("engine/objects/sprites.emp"));

    let file = with_ambient(vec![types, sst, constants], sprites);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: Vec::new(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sprites.emp lower errors: {ldiags:?}"
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
/// pins + constants.emp's growing twin. sprites.emp itself carries ZERO
/// module-local mirrors.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = sigil_harness::test_support::guard_assert_count(link_asserts);
    let want = 30 + sigil_harness::test_support::engine_constant_equs().len();
    assert_eq!(
        guards, want,
        "sst.emp's 30 + constants.emp's {} drift guards must be captured",
        sigil_harness::test_support::engine_constant_equs().len()
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

/// The region reference gate + the outbound bare-name proof + the drift
/// guards, shared body.
fn reference_gate(shape: &Shape, rom_name: &str) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(shape);
    assert_drift_guards(&resolved, &link_asserts);

    let base = shape.base as usize;
    let section = linked.section("sprites").expect("linked image must carry sprites");
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + shape.len],
        &format!("sprites vs {rom_name}[{base:#x}..{:#x}]", base + shape.len),
    );

    // Outbound bare-name proof: the AS-side bare `jsr InitSpriteSystem` must
    // relax to the abs.w encoding (`4EB8 base`) — the game state's shape in
    // the mixed build. The consumer is the LAST synthetic group: equ blob +
    // N labels + consumer.
    let consumer_lma = 0x0100_0000u32 + (1 + shape.labels.len() as u32) * 0x10_0000;
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == consumer_lma)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    assert_eq!(
        &consumer.bytes[0..4],
        &[0x4E, 0xB8, (shape.base >> 8) as u8, shape.base as u8],
        "bare-name proof: `jsr InitSpriteSystem` must relax to abs.w at the region base"
    );
}

/// (plain) the `sprites` region == `s4.bin[0x2954..0x2D74]`.
#[test]
fn sprites_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) the `sprites` region == `s4.debug.bin[0x2C0E..0x302E]`.
#[test]
fn sprites_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}

// ── The AS-twin lockstep oracle ─────────────────────────────────────────────

/// The AS twin, assembled through the sigil AS front-end at the PLAIN base
/// with the same equ prelude the `.emp` gets. sprites.asm is include-free, so
/// no splicing. This is the LOCKSTEP gate ("the continuous gate is only
/// `.emp == AS twin`") on an independent path — a macro/edit AS-side the
/// `.emp` doesn't mirror fails here naming the first diverging byte.
fn as_twin_bytes() -> Vec<u8> {
    let aeon = aeon_dir();
    let sprites_src = std::fs::read_to_string(aeon.join("engine/objects/sprites.asm"))
        .expect("sprites.asm must be readable");

    let mut prelude = String::from("cpu 68000\nsupmode on\n");
    let mut pairs = sigil_harness::test_support::sst_field_equs();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    for (name, rhs) in pairs {
        prelude.push_str(&format!("{name} = {rhs}\n"));
    }
    for (name, vma) in PLAIN.labels {
        prelude.push_str(&format!("{name} = ${vma:X}\n"));
    }
    let src = format!("{prelude}org ${:X}\n{sprites_src}\n", PLAIN.base);

    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
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

/// The `.emp` vs the AS-twin oracle (PLAIN shape), byte-for-byte.
#[test]
fn sprites_matches_as_twin() {
    let aeon = aeon_dir();
    if !aeon.join("engine/objects/sprites.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let (_, linked, _) = compile_real_file(&PLAIN);
    let section = linked.section("sprites").expect("linked image must carry sprites");
    let expected = as_twin_bytes();
    assert_region_matches(&section.bytes, &expected, "sprites vs AS twin (plain)");
}

// ── The twin-mirror drift probe (negative test) ─────────────────────────────

/// A DOCTORED twin truth (`SCREEN_WIDTH` = 319 AS-side while constants.emp
/// says 320) must fire the twin's `ensure(extern(…))` guard NAMING the
/// constant — proving the tranche's 15 new constants-twin guards ride
/// sprites' gate, paired with the undoctored control (the reference gates
/// above). SCREEN_WIDTH's drift would silently shift the exact X-cull
/// boundary in Draw_Sprite.
#[test]
fn doctored_twin_mirror_fires_its_guard() {
    let aeon = aeon_dir();
    if !aeon.join("engine/objects/sprites.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let (resolved, _, link_asserts) = compile_real_file_with(&PLAIN, Some(("SCREEN_WIDTH", "319")));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(
        !fired.is_empty(),
        "the doctored SCREEN_WIDTH truth must fire constants.emp's drift guard"
    );
    assert!(
        fired.iter().any(|d| d.message.contains("SCREEN_WIDTH")),
        "the fired guard must NAME the drifted constant: {fired:?}"
    );
}

