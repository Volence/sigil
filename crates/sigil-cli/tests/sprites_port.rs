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
//!   `Camera_X`/`Camera_Y`), all abs.w EAs — plus `Sprite_Owner`, a DEBUG-only
//!   thirteenth that the plain shape never names. Ten of them SHIFT +$22 in the
//!   debug shape (debug RAM inserted ahead), so the RAM-EA operand bytes DIFFER
//!   per shape — each shape diffs against its own ROM window with its own VMAs.
//!   Region length is shape-dependent too (see the reference-window block below,
//!   and `pins::SPRITES.{plain,debug}_len`, which is where the numbers live).
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
//! ## Reference windows
//! (sourced from `sigil_harness::pins` — regenerate via repin)
//!
//! Both windows come from `pins::SPRITES` at run time — base and length, per
//! shape. The numbers are deliberately not restated here: a bound copied into
//! prose is executed by nothing, so nothing can go red when it rots.
//!
//! Length went shape-DEPENDENT at the bug005-sprites parcel: the H1 staleness
//! net + the BUG-005 chain-walk are `if DEBUG == 1 {}` blocks with `assert.w`
//! expansions (rings/core precedent), so the module now takes the `DEBUG`
//! build-shape define and the debug shape carries the two MDDBG__* error-handler
//! seams. The same parcel's H3 (partial SAT DMA length) patches the
//! `Static_Sprite_DMA` queue entry via `movep.w d0, DMAEntry.SizeH(a0)` — the
//! DMAEntry struct rides ambient `engine/structs.emp` (buffers_port precedent)
//! and Static_Sprite_DMA is a new inbound RAM label.
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
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
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
        ("Sprite_Emit_Active", pins::SPRITE_EMIT_ACTIVE.plain),
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
        // bug005 H3: the partial-SAT DMA length patch writes the queue entry.
        ("Static_Sprite_DMA", pins::STATIC_SPRITE_DMA.plain),
    ],
};

const DEBUG: Shape = Shape {
    base: pins::SPRITES.debug_base,
    len: pins::SPRITES.debug_len,
    labels: &[
        ("DrawRings", pins::DRAW_RINGS.debug),
        ("Sprite_Table_Buffer", pins::SPRITE_TABLE_BUFFER.debug),
        ("Sprite_Table_Dirty", pins::SPRITE_TABLE_DIRTY.debug),
        ("Sprite_Emit_Active", pins::SPRITE_EMIT_ACTIVE.debug),
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
        // bug005 H3: the partial-SAT DMA length patch writes the queue entry.
        ("Static_Sprite_DMA", pins::STATIC_SPRITE_DMA.debug),
        // SPRITE-OWNER: engine/ram.emp declares Sprite_Owner inside
        // `if DEBUG == 1 @shape_divergent`, and Render_Sprites holds `lea
        // Sprite_Owner, a6` for the whole render (abs.w — the VMA is in these
        // bytes). DEBUG shape only; the plain shape never names it.
        ("Sprite_Owner", pins::SPRITE_OWNER),
        // DEBUG-only: the H1-staleness + BUG-005 chain-walk assert.w
        // expansions jsr/jmp these (core_port precedent).
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
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
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
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
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

/// Compile the real `sprites.emp` with its ambient dependencies (types +
/// structs + sst + constants) and the given build-shape defines, place it at
/// the per-shape base, append the synthetic cross-seam sections, and link.
/// The `DEBUG` define dimension arrived with the bug005-sprites parcel (the
/// H1-staleness + BUG-005 chain-walk `if DEBUG == 1 {}` blocks); the structs
/// dep carries `DMAEntry` for the H3 `movep.w d0, DMAEntry.SizeH(a0)` patch.
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
    let structs = parse_file(&aeon.join("engine/structs.emp"));
    let sst = parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = parse_file(&aeon.join("engine/system/constants.emp"));
    let sprites = parse_file(&aeon.join("engine/objects/sprites.emp"));

    let file = with_ambient(vec![types, structs, sst, constants], sprites);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
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
    // sst.emp's SST_* drift wall retired at the conv-a structs flip; sprites.emp
    // carries no module-local mirrors. constants.emp's bug005
    // PHYS_JUMP_SKIP_CLEARANCE ensure is comptime-folded (no extern), so it
    // contributes no captured guard — the count stays the twin list's.
    let want = sigil_harness::test_support::engine_constant_equs().len();
    assert_eq!(
        guards, want,
        "constants.emp's {} drift guards must be captured",
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
    // A gate over an EMPTY image proves nothing, and the tolerance below would
    // hide that: with no candidate bytes it shrinks `expected` to zero length, the
    // length assert compares 0 == 0, and the diff loop runs over an empty range —
    // so the test passes if the module emits nothing at all. Confirmed live on
    // OJZ_BG_ANIM, a 14-byte all-zero plain window (lens sweep, seat GATE, S15).
    assert!(
        !candidate.is_empty(),
        "{what}: the module emitted NO BYTES, a region gate over an empty window \
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
        "{what}: length mismatch, candidate {} bytes, expected {} bytes",
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

/// (plain) the `sprites` region == `s4.bin` at the pinned window.
#[test]
fn sprites_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin", false);
}

/// (debug) the `sprites` region == `s4.debug.bin` at the pinned window.
#[test]
fn sprites_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin", true);
}

// The AS-twin lockstep oracle RETIRED (flip Stage-2): sprites.asm is deleted —
// the .emp is the only source. Coverage is subsumed by the region gates above
// (sprites == frozen-golden slice) + the native whole-ROM golden gates; the t24
// negative probe below (doctored SCREEN_WIDTH) keeps the golden non-vacuous.

// ── The twin-mirror drift probe (negative test) ─────────────────────────────

// The `doctored_twin_mirror_fires_its_guard` negative probe (SCREEN_WIDTH)
// retired with the Stage-3 P5 ownership flip: `SCREEN_WIDTH` and the other
// constants-twin values are now SOLE-authored by `constants.emp` (harvested into
// guarded AS defines), so their mirror drift guards were deleted — nothing for a
// doctored AS-side truth to fire. The undoctored reference gates above remain the
// proof these values are load-bearing.

