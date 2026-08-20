//! Tranche 21 (file 1) — the REAL `buffers.emp` port, region-level byte gate.
//!
//! Compiles the ACTUAL ported file from aeon's tree —
//! `engine/system/buffers.emp` — through the production parse -> lower ->
//! place -> resolve -> link pipeline, and asserts the `buffers` section's
//! flattened bytes equal the reference ROM window at the pinned addresses, in
//! BOTH build shapes.
//!
//! ## What this port exercises
//!
//! - **The DMAEntry struct adoption** (`.build_entry` writes the entry via
//!   `DMAEntry.field(a0)` displacements incl. both movep widths — the t20
//!   ledger ride retired here).
//! - **queue_static_dma** — the .emp counterpart of macros.asm's
//!   `queueStaticDMA` (entry-only, Critical-bound interface; spliced CCR
//!   carry result consumed by caller-side bcs — probe-pinned in
//!   `tranche21_spelling_probes`).
//! - **The dmaSource link-time arm** — `equ SRC_* = (extern(sym) >> 1) &
//!   $7FFFFF` over RAM labels (the row-1004 comptime/link boundary).
//! - **engine.vdp derivation fns** (`vdp_comm`, `vdp_comm_delta`,
//!   `plane_loc`, `dma_length`) folding into immediates.
//! - **The shared parallax_config struct twin** (moved to engine.structs at
//!   this port — 2nd .emp consumer).
//! - Region length is shape-INVARIANT (buffers.asm has no ifdef arms).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, the gates SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test buffers_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module_with_contracts, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn region_base(debug: bool) -> u32 {
    if debug { pins::BUFFERS.debug_base } else { pins::BUFFERS.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::BUFFERS.debug_len } else { pins::BUFFERS.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

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
         name = \"buffers\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// The VALUE seam: the prepended twins' drift-lock truths. `doctor` overrides
/// ONE pair (the negative probe).
fn value_equs(doctor: Option<(&str, &str)>) -> Vec<Section> {
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
    ];
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
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
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols, each a `phase`d one-byte carrier at its
/// pinned per-shape VMA.
fn addr_labels(debug: bool) -> Vec<Section> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let table: Vec<(&str, u32)> = vec![
        ("Palette_Buffer", pick(pins::PALETTE_BUFFER)),
        ("Sprite_Table_Buffer", pick(pins::SPRITE_TABLE_BUFFER)),
        ("Hscroll_Buffer", pick(pins::HSCROLL_BUFFER)),
        ("Static_Pal_Line0", pick(pins::STATIC_PAL_LINE0)),
        ("Static_Pal_Line1", pick(pins::STATIC_PAL_LINE1)),
        ("Static_Pal_Line2", pick(pins::STATIC_PAL_LINE2)),
        ("Static_Pal_Line3", pick(pins::STATIC_PAL_LINE3)),
        // The off-screen frame-top ship (effects P3). Enqueue_Dirty_Buffers gained a block
        // that ships a patched channel's colours when its anchor leaves the top of the screen,
        // and these are its THREE cross-seam references — declare them here or this gate stops
        // resolving, which is exactly how it failed when the parcel first built green.
        ("Static_Pal_Ship", pick(pins::STATIC_PAL_SHIP)),
        ("Effects_Offscreen_Entry", pick(pins::EFFECTS_OFFSCREEN_ENTRY)),
        ("Effects_Screen_L", pick(pins::EFFECTS_SCREEN_L)),
        ("Static_Sprite_DMA", pick(pins::STATIC_SPRITE_DMA)),
        ("Static_Hscroll_Cell", pick(pins::STATIC_HSCROLL_CELL)),
        ("Static_Hscroll_Line", pick(pins::STATIC_HSCROLL_LINE)),
        ("Palette_Dirty", pick(pins::PALETTE_DIRTY)),
        // R1 Task 2: the four snapshot splices in Enqueue_Dirty_Buffers reference this
        // RAM-tail field directly — declare it here or this gate stops resolving, same
        // as the Effects P3 trio above.
        ("Palette_Ship_Snap", pick(pins::PALETTE_SHIP_SNAP)),
        ("Sprite_Table_Dirty", pick(pins::SPRITE_TABLE_DIRTY)),
        ("Sprite_Emit_Active", pick(pins::SPRITE_EMIT_ACTIVE)),
        ("DMA_Critical_Slot", pick(pins::DMA_CRITICAL_SLOT)),
        ("DMA_Critical_End", pick(pins::DMA_CRITICAL_END)),
        ("Parallax_Active_Config", pick(pins::PARALLAX_ACTIVE_CONFIG)),
    ];
    let mut out = Vec::new();
    for (i, (name, vma)) in table.iter().enumerate() {
        let vma = *vma;
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

/// Lower the real `buffers.emp` (prepend the engine.structs, engine.constants
/// and engine.vdp twins its `use` lines read), place into the per-shape map,
/// append the value equs + address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
    doctor: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/system");
    let main = parse_file(&dir.join("buffers.emp"));
    let structs_file = parse_file(&aeon.join("engine/structs.emp"));
    let consts_file = parse_file(&aeon.join("engine/system/constants.emp"));
    let vdp_file = parse_file(&aeon.join("engine/vdp.emp"));
    // The SYNTHESIZED `CAP_*` block standing in for
    // `use engine.level.scene_dsl.{CAP_PER_LINE, CAP_ANCHORS}`: a single-module lower has
    // no module to follow, and the bit values are read out of scene_dsl.emp at test
    // runtime (test_support §4) so this gate can never bind a stale mask.
    let caps_src = sigil_harness::test_support::scene_dsl_cap_consts_src(&aeon);
    let (caps_file, caps_diags) = parse_str(&caps_src);
    assert!(
        caps_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "synthesized CAP_* block parse errors: {caps_diags:?}"
    );
    // T6's forcer single-sourcing: `use engine.level.scene_dsl.{parallax_mode_key}` —
    // the shared comptime template both mode twins splice. Extracted VERBATIM from the
    // parsed scene_dsl.emp at test runtime (same never-stale rationale as the CAP_*
    // block above; the fn's free names resolve at the CALL SITE, which this chained
    // file provides). Only the one item — scene_dsl.emp as a whole is CODE and would
    // emit bytes into the compared section.
    let scene_dsl_file = parse_file(&aeon.join("engine/level/scene_dsl.emp"));
    let mode_key_fn: Vec<sigil_frontend_emp::ast::Item> = scene_dsl_file
        .items
        .into_iter()
        .filter(|it| matches!(it, sigil_frontend_emp::ast::Item::ComptimeFn(d) if d.name == "parallax_mode_key"))
        .collect();
    assert!(
        !mode_key_fn.is_empty(),
        "parallax_mode_key not found in scene_dsl.emp — the port shim is stale against the tree"
    );
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: structs_file
            .items
            .into_iter()
            .chain(consts_file.items)
            .chain(vdp_file.items)
            .chain(caps_file.items)
            .chain(mode_key_fn)
            .chain(main.items)
            .collect(),
        docs: main.docs.clone(),
    };

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    // The game-contract env: buffers.emp's HScroll fill gates two blocks on
    // `Game.SCANLINE_CAPS & CAP_PER_LINE` / `CAP_ANCHORS`, which the whole-program bind
    // pass resolves and a single-module lower does not. Bound to SONIC4's declared mask,
    // read from its game.emp — the reference windows below are sonic4-shaped.
    let contracts = sigil_harness::test_support::scanline_caps_contract_env(&aeon);
    let (module, ldiags) = lower_module_with_contracts(&file, &opts, &contracts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "buffers.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    sections.extend(value_equs(doctor));
    sections.extend(addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
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
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "buffers.emp drift guards must all PASS: {diags:?}"
    );

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("buffers").expect("linked image must carry buffers");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("buffers ({shape})"));
}

#[test]
fn buffers_region_matches_reference() {
    run(false);
}

#[test]
fn buffers_debug_region_matches_reference() {
    run(true);
}

// The `doctored_vram_sprite_table_fires_its_guard` negative probe retired with
// the Stage-3 P5 ownership flip: `VRAM_SPRITE_TABLE` is now SOLE-authored by
// `constants.emp` (harvested into guarded AS defines), so its mirror drift guard
// was deleted. The undoctored reference gates above remain the proof.
