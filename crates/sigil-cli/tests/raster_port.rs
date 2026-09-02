//! `raster_port` — the standalone byte oracle for `engine/effects/raster.emp`.
//!
//! Written 2026-08-15, after an audit found the `tests = ["raster_port"]` pins in
//! `repin.toml` were consumed by **no test at all**: there was no `raster_port` binary in
//! `crates/`, and `refreeze`'s own rerun hint named a target that did not exist. The pins
//! were regenerated on every byte-moving parcel and read by nothing — coverage-shaped
//! metadata with no coverage behind it, which is the failure class this project keeps
//! re-finding. The rule invoked was "either write the port test or delete the pins".
//!
//! Two of the seven pins turned out NOT to be dead, and finding that out is why the
//! deletion did not happen blind: `RASTER` is a live row in `native.rs`'s module registry,
//! and `HBLANK_UNINSTALL_OFF` is consumed by `hblank_port.rs:276` and was merely
//! MIS-TAGGED. Five symbol pins were genuinely dead. They were also **incomplete** — they
//! described the module's Parcel-P1 surface and were never widened as the dense tier (P2),
//! the ramp op, and P-b's patch channels each added RAM the module names. See the block
//! comment beside them in `repin.toml`.
//!
//! ## What this test is worth, beyond the whole-ROM goldens
//!
//! Stated plainly because a port test that merely re-asserts today's bytes IS a second
//! golden, and this harness has been bitten by decorative gates. Three properties the
//! whole-ROM goldens structurally cannot assert:
//!
//! 1. **Standalone re-lower ≡ in-link bytes.** The full link resolves every name, so a
//!    module that silently acquires a new undeclared cross-seam dependency stays green in
//!    the goldens forever. The port test plus `raster_negative_probes`' carrier-removal
//!    probe is the only mechanism that ENUMERATES this module's seam surface and goes loud
//!    when it grows.
//! 2. **Bare-name export proof for consumerless exports.** Parcel P-b added
//!    `Raster_InstallPatched` and `Effects_SetWorldY`. An export with no in-ROM consumer is
//!    invisible to a golden — the bytes exist either way. The synthetic AS-side consumer
//!    below is what proves those names surface and resolve where the region says.
//! 3. **Placement determinism at the seam** — the standalone lower with true pinned VMAs
//!    must reproduce the relaxation the full link chose; a wrong-base map must fail.
//!
//! ## What this test is NOT worth, recorded so nobody credits it later
//!
//! **It cannot catch the `add.w dN,aM` → ADDX silent mis-encoding class.** Both sides of
//! the comparison go through the SAME sigil encoder, so a systematic mis-encoding
//! reproduces identically in the reference ROM and in the re-lower, and this test passes.
//! The port campaign had that power only while the reference was `asl`-built; the Stage-2
//! flip removed that independence. Hand-decoding the emitted opcodes remains the only
//! check for that class, and `raster_port` does not retire it. This paragraph exists
//! because the tree already carries one ledger entry crediting a gate with a catch it never
//! made (EFX-5/EFX-9); do not mint another.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, or
//! `EMPYREAN_SUITE_ROOT`). Absent, the tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test raster_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module_with_contracts, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

pub fn aeon_dir() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

fn raster_dir() -> PathBuf {
    aeon_dir().join("engine/effects")
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// The map: a `text` carrier region for the module's zero-byte default section, and the
/// real `raster` region pinned at the per-shape reference base.
pub fn map_toml(debug: bool) -> String {
    let base = if debug { pins::RASTER.debug_base } else { pins::RASTER.plain_base };
    let size = pins::RASTER.plain_len;
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
         name = \"raster\"\n\
         lma_base = {base:#x}\n\
         size = {size:#x}\n\
         kind = \"rom\"\n"
    )
}

/// The INBOUND cross-seam surface: every RAM label `raster.emp` names, as a `phase`d
/// one-byte carrier at its true VMA.
///
/// THIS LIST IS THE POINT OF THE TEST, not bookkeeping. It is the module's declared seam
/// surface; if the module grows a new one and nobody adds it here, the lower fails loud
/// instead of the goldens staying quiet. `Raster_State`/`Raster_State_End` are the region
/// MARKS — `raster.emp`'s
/// `ensure((extern("Raster_State_End") - extern("Raster_State")) == RASTER_STATE_SIZE)`
/// evaluates at lower time, so both must be present at their true VMAs (0x132 = 306 apart)
/// or the guard fails here for a reason unrelated to the port.
pub fn cross_seam_labels(debug: bool) -> Vec<Section> {
    let labels: [(&str, u32); 25] = [
        ("Raster_Program", pins::RASTER_PROGRAM.plain),
        ("Raster_Cursor", pins::RASTER_CURSOR.plain),
        ("Raster_Pending", pins::RASTER_PENDING.plain),
        ("Raster_Line", pins::RASTER_LINE.plain),
        ("Raster_Buf_A", pins::RASTER_BUF_A.plain),
        ("Raster_Buf_B", pins::RASTER_BUF_B.plain),
        ("Raster_Active_Buf", pins::RASTER_ACTIVE_BUF.plain),
        ("Raster_Dense_Lines", pins::RASTER_DENSE_LINES.plain),
        ("Raster_Dense_Cursor", pins::RASTER_DENSE_CURSOR.plain),
        ("Raster_Dense_Cmd", pins::RASTER_DENSE_CMD.plain),
        ("Raster_Dense_Mode", pins::RASTER_DENSE_MODE.plain),
        ("Raster_Ramp_Acc", pins::RASTER_RAMP_ACC.plain),
        ("Raster_Ramp_Step", pins::RASTER_RAMP_STEP.plain),
        ("Effects_World_Y", pins::EFFECTS_WORLD_Y.plain),
        ("Raster_Patch_Tab", pins::RASTER_PATCH_TAB.plain),
        ("Raster_State", pins::RASTER_STATE.plain),
        ("Raster_State_End", pins::RASTER_STATE_END.plain),
        ("Palette_Dirty", pins::PALETTE_DIRTY.plain),
        ("Pal_Variant_Stage", pins::PAL_VARIANT_STAGE.plain),
        // R1 Task 3: OP_PAL_RESTORE's body reads Palette_Ship_Snap directly (the ship
        // snapshot buffers.emp's splices fill) — declare it here or this gate stops
        // resolving, same as the Palette_Ship_Snap entry Task 2 added to buffers_port.
        // Shape-dependent like Camera_Y/Build_DMA_Entry below: RAM past __DEBUG__.
        (
            "Palette_Ship_Snap",
            if debug { pins::PALETTE_SHIP_SNAP.debug } else { pins::PALETTE_SHIP_SNAP.plain },
        ),
        // The off-screen frame-top ship (effects P3). Effects_Screen_L is the latch
        // Raster_PatchAll now READS instead of deriving `world_y - Camera_Y` itself;
        // Effects_Offscreen_Entry and Static_Pal_Ship are Raster_InstallPatched's and
        // Raster_BuildShipEntry's targets. All three are engine RAM ahead of the __DEBUG__
        // block, so they are shape-invariant like their neighbours above.
        ("Effects_Screen_L", pins::EFFECTS_SCREEN_L.plain),
        ("Effects_Offscreen_Entry", pins::EFFECTS_OFFSCREEN_ENTRY.plain),
        ("Static_Pal_Ship", pins::STATIC_PAL_SHIP.plain),
        // THE ONE SHAPE-DEPENDENT CARRIER. Every raster/palette RAM label above sits
        // ahead of the __DEBUG__ block and so has an identical plain and debug VMA;
        // Camera_Y sits past it and moves ($FFFFA43A -> $FFFFA4C8). Supplying `.plain`
        // for both shapes passed the PLAIN test and failed debug at region offset
        // 0x1c7 — which is Raster_PatchAll's `move.w Camera_Y, d3`. Worth noting the
        // failure was legible only because the plain twin passed: a shared bug would
        // have looked like a port-harness problem rather than a carrier VMA.
        ("Camera_Y", if debug { pins::CAMERA_Y.debug } else { pins::CAMERA_Y.plain }),
        // SECOND shape-dependent carrier, and a ROM one: Raster_BuildShipEntry tail-calls
        // engine.buffers' Build_DMA_Entry — the single DMAEntry builder — so the operand is a
        // code address in another region and moves between shapes like any ROM symbol.
        // (Camera_Y above is still needed: Raster_PatchAll's read of it went away with the
        // latch, but Effects_LatchWorldLines in this same module is what does the reading now.)
        ("Build_DMA_Entry", if debug { pins::BUILD_DMA_ENTRY.debug } else { pins::BUILD_DMA_ENTRY.plain }),
    ];
    carriers(&labels, 0x0400_0000)
}

/// Outbound CALL targets: `raster.emp` `jbsr`s into the hblank trampoline. Supplied at the
/// hblank region's per-shape base so the emitted call operands match the reference.
pub fn hblank_call_targets(debug: bool) -> Vec<Section> {
    let base = if debug { pins::HBLANK.debug_base } else { pins::HBLANK.plain_base };
    let labels: [(&str, u32); 2] = [
        ("HBlank_Install", base),
        ("HBlank_Uninstall", base + pins::HBLANK_UNINSTALL_OFF as u32),
    ];
    carriers(&labels, 0x0500_0000)
}

fn carriers(labels: &[(&str, u32)], lma_base: u32) -> Vec<Section> {
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let mut out = Vec::new();
    for (i, (name, vma)) in labels.iter().enumerate() {
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections;
        for mut s in secs {
            s.lma = lma_base + (i as u32) * 0x1_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            out.push(s);
        }
    }
    out
}

/// The synthetic AS-side OUTBOUND consumer — THE BARE-NAME PROOF.
///
/// `Raster_InstallPatched` and `Effects_SetWorldY` are exactly the exports a golden cannot
/// see: P-b added them, and `Effects_SetWorldY`'s only call site in the whole tree is a
/// DEBUG-gated hotkey in a test scaffold. If either name stops surfacing as a bare link
/// symbol, this is what notices.
pub fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tdc.l Raster_Install\n\
               \tdc.l Raster_VBlank\n\
               \tdc.l Raster_HInt\n\
               \tdc.l Raster_InstallPatched\n\
               \tdc.l Effects_SetWorldY\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}"))
        .sections
}

/// Parse the module, prepend the items of every module it `use`s (plus `raster_dsl`, whose
/// consts reach `raster.emp` by COMPTIME_HELPERS glob injection in the real build and so
/// have no `use` line to follow), lower, place, link.
pub fn compile_real_file(debug: bool, carriers_override: Option<Vec<Section>>) -> sigil_link::LinkedImage {
    let dir = raster_dir();
    let engine = aeon_dir().join("engine");

    let read = |p: PathBuf| -> String {
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
    };
    let parse = |src: &str, what: &str| {
        let (f, d) = parse_str(src);
        assert!(
            d.iter().all(|x| x.level != sigil_span::Level::Error),
            "{what} parse errors: {d:?}"
        );
        f
    };

    let main = parse(&read(dir.join("raster.emp")), "raster.emp");
    // Order matters only in that every name must be defined before the item that reads it
    // at comptime; these five are leaves. The last is the SYNTHESIZED `CAP_*` block —
    // raster.emp's `use engine.level.scene_dsl.{CAP_ANCHORS}` has no module to follow in a
    // single-file lower, and its values are read out of scene_dsl.emp at test runtime (see
    // test_support §4) so this gate can never bind a stale mask.
    let deps = [
        parse(&read(engine.join("vdp.emp")), "vdp.emp"),
        parse(&read(engine.join("system/constants.emp")), "constants.emp"),
        parse(&read(engine.join("irq.emp")), "irq.emp"),
        parse(&read(dir.join("raster_dsl.emp")), "raster_dsl.emp"),
        parse(
            &sigil_harness::test_support::scene_dsl_cap_consts_src(&aeon_dir()),
            "scene_dsl CAP_* block",
        ),
    ];

    let mut items = Vec::new();
    for d in deps {
        items.extend(d.items);
    }
    items.extend(main.items.clone());
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    };

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
    };
    // The game-contract env: raster.emp's anchors overlay sits under
    // `if (Game.SCANLINE_CAPS & CAP_ANCHORS) != 0`, which the whole-program bind pass
    // resolves and a single-module lower does not. Bound to SONIC4's declared mask, read
    // from its game.emp — the reference windows below are sonic4-shaped, so any other
    // binding would compare a specialisation the reference never took.
    let contracts = sigil_harness::test_support::scanline_caps_contract_env(&aeon_dir());
    let (module, ldiags) = lower_module_with_contracts(&file, &opts, &contracts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors: {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    // VDP port addresses. They are `pub const`s in engine/system/constants.emp, but the
    // lower emits them as SYMBOLIC absolute operands, so prepending that module's items is
    // not enough — the link still needs the names. Supplied as equ pairs, the
    // parallax_port/hblank_port idiom.
    let mut equs = sigil_harness::test_support::assemble_equ_pairs(&[
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
    ]);
    for sec in &mut equs {
        sec.lma = 0x0300_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    sections.extend(carriers_override.unwrap_or_else(|| cross_seam_labels(debug)));
    sections.extend(hblank_call_targets(debug));

    let mut consumer = as_outbound_consumer();
    for sec in &mut consumer {
        sec.lma = 0x0200_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
        sec.name = "outbound".to_string();
    }
    sections.extend(consumer);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"))
}

/// On mismatch, report the first differing offset plus context. Carries `hblank_port`'s
/// empty-window guard verbatim: without it, a module that emits NOTHING passes, because the
/// alignment tolerance shrinks `expected` to zero and the diff loop runs over an empty
/// range. Confirmed live on a 14-byte all-zero window during the lens sweep.
pub fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    assert!(
        !candidate.is_empty(),
        "{what}: the module emitted NO BYTES — a region gate over an empty window \
         proves nothing. Either the module stopped emitting, or this pin should not exist."
    );
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

/// The bare-name proof: every consumerless `pub proc` resolves inside the raster region.
///
/// Asserted as a RANGE rather than against per-symbol offset pins on purpose. Pinning each
/// one's exact offset would make this test fail on every parcel that reorders procs inside
/// the module — churn that says nothing, since the region byte-compare above already
/// catches any real content change. What is not otherwise checked, and is checked here, is
/// that each name EXPORTS AT ALL and lands in this module rather than resolving to 0 or to
/// some other section.
fn assert_outbound(linked: &sigil_link::LinkedImage, base: u32, len: u32) {
    let names = [
        "Raster_Install",
        "Raster_VBlank",
        "Raster_HInt",
        "Raster_InstallPatched",
        "Effects_SetWorldY",
    ];
    let consumer = linked
        .section("outbound")
        .expect("linked image must carry the outbound consumer");
    for (i, name) in names.iter().enumerate() {
        let off = i * 4;
        let v = u32::from_be_bytes(consumer.bytes[off..off + 4].try_into().unwrap());
        assert!(
            v >= base && v < base + len,
            "bare-name proof: `dc.l {name}` resolved to {v:#010x}, outside the raster region \
             [{base:#010x}, {:#010x}). Either the name stopped exporting or it moved module.",
            base + len
        );
    }
}

/// (plain) The `raster` section's linked bytes equal the plain reference window, and every
/// consumerless export surfaces inside the region.
#[test]
fn raster_region_matches_reference() {
    let rom_path = aeon_dir().join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let linked = compile_real_file(false, None);
    let base = pins::RASTER.plain_base as usize;
    let expected = &refrom[base..base + pins::RASTER.plain_len];
    let section = linked.section("raster").expect("linked image must carry raster");
    assert_region_matches(&section.bytes, expected, "raster (plain) vs reference window");
    assert_outbound(&linked, pins::RASTER.plain_base, pins::RASTER.plain_len as u32);
}

/// (debug) Same, against the debug reference at the debug base.
#[test]
fn raster_debug_region_matches_reference() {
    let rom_path = aeon_dir().join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but debug reference missing: {}", rom_path.display());
        }
        eprintln!("skip: debug reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let linked = compile_real_file(true, None);
    let base = pins::RASTER.debug_base as usize;
    let expected = &refrom[base..base + pins::RASTER.debug_len];
    let section = linked.section("raster").expect("linked image must carry raster");
    assert_region_matches(&section.bytes, expected, "raster (debug) vs reference window");
    assert_outbound(&linked, pins::RASTER.debug_base, pins::RASTER.debug_len as u32);
}
