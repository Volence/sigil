//! `raster_port`'s negative probes — the half that makes it more than a second golden.
//!
//! Shipped WITH `raster_port.rs`, not after it. A port test that only re-asserts today's
//! bytes proves "the bytes did not change", which the whole-ROM goldens already prove; what
//! makes it worth its keep is that these three probes show the gate FIRES. Without them the
//! port test is the decorative outcome the audit that commissioned it was trying to end.
//!
//! Self-contained per house style (`hblank_negative_probes.rs`, `sfx_negative_probes.rs`):
//! the helpers here are local, and the real `raster.emp` is READ but never written — every
//! probe doctors a copy in memory.
//!
//! ## Probes
//!
//! (a) **genuineness** — doctor ONE register field inside one of `Raster_PatchAll`'s indexed
//!     addressing modes (`move.w (a2, d2.w), d2` → `(a2, d3.w)`) and require the linked
//!     bytes to DIFFER from the reference. The indexed modes are chosen deliberately: they
//!     are the two instructions in this module that were hand-decoded from ROM during
//!     Parcel P-b, because this codebase has a recorded case of `add.w dN,aM` silently
//!     encoding as ADDX garbage. This probe proves a single register-field change in
//!     exactly those instructions is VISIBLE to the gate.
//!
//!     It does **not** prove the gate catches the ADDX class itself, and must never be cited
//!     as if it did — both sides of `raster_port`'s comparison run through the same sigil
//!     encoder, so a systematic mis-encoding reproduces on both sides and passes. See the
//!     header of `raster_port.rs`.
//!
//! (b) **seam enumeration** — compile the real module WITHOUT the inbound RAM carriers.
//!     `resolve_layout` must fail LOUD, naming a missing cross-seam symbol. This is the
//!     probe that makes the pins genuinely CONSUMED: it is what turns `raster_port`'s
//!     carrier list from bookkeeping into an assertion that the module's seam surface is
//!     exactly the declared one. A module that silently grows a new cross-seam dependency
//!     stays green in the whole-ROM goldens forever; it cannot stay green here.
//!
//! (c) **placement** — a wrong-base map must make the bytes stop matching, proving the
//!     byte-diff is a real placement check rather than an echo of the reference.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test raster_negative_probes
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module_with_contracts, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_span::Level;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn raster_dir() -> PathBuf {
    aeon_dir().join("engine/effects")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The real sources, or a strict-gate panic / soft skip when the sibling tree is absent.
fn real_sources() -> Option<(String, Vec<String>)> {
    let dir = raster_dir();
    let engine = aeon_dir().join("engine");
    let paths = [
        engine.join("vdp.emp"),
        engine.join("system/constants.emp"),
        engine.join("irq.emp"),
        dir.join("raster_dsl.emp"),
    ];
    let main = match std::fs::read_to_string(dir.join("raster.emp")) {
        Ok(s) => s,
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but raster.emp missing under {}", dir.display());
            }
            eprintln!("skip: raster.emp not found (set AEON_DIR)");
            return None;
        }
    };
    let mut deps = Vec::new();
    for p in paths {
        match std::fs::read_to_string(&p) {
            Ok(s) => deps.push(s),
            Err(_) => {
                if strict_gate() {
                    panic!("SIGIL_STRICT_GATE set but dependency missing: {}", p.display());
                }
                eprintln!("skip: {} not found", p.display());
                return None;
            }
        }
    }
    // The synthesized `CAP_*` block, values read out of scene_dsl.emp at runtime — the
    // same dep `raster_port` prepends. raster.emp's anchors overlay is gated on
    // `Game.SCANLINE_CAPS & CAP_ANCHORS`, so without this every probe below dies at
    // LOWER instead of reaching its own subject.
    let scene_dsl = aeon_dir().join("engine/level/scene_dsl.emp");
    if !scene_dsl.exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but dependency missing: {}", scene_dsl.display());
        }
        eprintln!("skip: {} not found", scene_dsl.display());
        return None;
    }
    deps.push(sigil_harness::test_support::scene_dsl_cap_consts_src(&aeon_dir()));
    Some((main, deps))
}

fn map_toml(base: u32) -> String {
    let size = pins::RASTER.plain_len;
    format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n\
         [[region]]\nname = \"raster\"\nlma_base = {base:#x}\nsize = {size:#x}\nkind = \"rom\"\n"
    )
}

fn carriers(labels: &[(&str, u32)], lma_base: u32) -> Vec<Section> {
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let mut out = Vec::new();
    for (i, (name, vma)) in labels.iter().enumerate() {
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        for mut s in assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections
        {
            s.lma = lma_base + (i as u32) * 0x1_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            out.push(s);
        }
    }
    out
}

fn inbound() -> Vec<Section> {
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
        // R1 Task 3: OP_PAL_RESTORE's body reads Palette_Ship_Snap directly — same
        // carrier raster_port.rs added, needed here for the same reason (this probe
        // must resolve cleanly before it can prove doctoring changes the bytes).
        ("Palette_Ship_Snap", pins::PALETTE_SHIP_SNAP.plain),
        // The off-screen frame-top ship's cross-seam references — the same three raster_port
        // declares. A negative probe has to RESOLVE cleanly before it can prove that doctoring
        // one thing changes the bytes, so a missing carrier here fails the probe as a harness
        // error rather than as the defect it is hunting.
        ("Effects_Screen_L", pins::EFFECTS_SCREEN_L.plain),
        ("Effects_Offscreen_Entry", pins::EFFECTS_OFFSCREEN_ENTRY.plain),
        ("Static_Pal_Ship", pins::STATIC_PAL_SHIP.plain),
        ("Build_DMA_Entry", pins::BUILD_DMA_ENTRY.plain),
        ("Camera_Y", pins::CAMERA_Y.plain),
    ];
    carriers(&labels, 0x0400_0000)
}

fn hblank_targets() -> Vec<Section> {
    let base = pins::HBLANK.plain_base;
    carriers(
        &[
            ("HBlank_Install", base),
            ("HBlank_Uninstall", base + pins::HBLANK_UNINSTALL_OFF as u32),
        ],
        0x0500_0000,
    )
}

/// Compile a (possibly doctored) `raster.emp` source. `with_inbound = false` is probe (b).
/// Returns the linked image, or the resolve diagnostics when the link legitimately fails.
fn compile(
    main_src: &str,
    deps: &[String],
    base: u32,
    with_inbound: bool,
) -> Result<sigil_link::LinkedImage, Vec<sigil_span::Diagnostic>> {
    let parse = |src: &str, what: &str| {
        let (f, d) = parse_str(src);
        assert!(d.iter().all(|x| x.level != Level::Error), "{what} parse errors: {d:?}");
        f
    };
    let main = parse(main_src, "raster.emp");
    let mut items = Vec::new();
    for (i, d) in deps.iter().enumerate() {
        items.extend(parse(d, &format!("dep {i}")).items);
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
        include_root: Some(raster_dir()),
        embed_base: None,
        defines: vec![],
    };
    // Bound to sonic4's declared SCANLINE_CAPS (read from its game.emp), matching
    // `raster_port`: probe (a) and (c) compare against the sonic4 reference window, and
    // probe (b)'s missing carriers include RAM only the anchors overlay names.
    let contracts = sigil_harness::test_support::scanline_caps_contract_env(&aeon_dir());
    let (module, ldiags) = lower_module_with_contracts(&file, &opts, &contracts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors: {ldiags:?}");

    let map = sigil_link::load_map(&map_toml(base)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place errors: {pdiags:?}");

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

    if with_inbound {
        sections.extend(inbound());
    }
    sections.extend(hblank_targets());

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)?;
    Ok(sigil_link::link(&resolved, &SymbolTable::new()).expect("link must succeed once resolved"))
}

fn reference_window() -> Option<Vec<u8>> {
    let p = aeon_dir().join("s4.bin");
    let Ok(rom) = std::fs::read(&p) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", p.display());
        }
        eprintln!("skip: reference ROM not at {}", p.display());
        return None;
    };
    let base = pins::RASTER.plain_base as usize;
    Some(rom[base..base + pins::RASTER.plain_len].to_vec())
}

/// (a) A one-register-field change inside `Raster_PatchAll`'s indexed load must change the
/// linked bytes. If this passed silently, `raster_port`'s byte-diff would be matching by
/// construction rather than checking anything.
#[test]
fn a_doctored_indexed_mode_changes_the_bytes() {
    let Some((main, deps)) = real_sources() else { return };
    let Some(expected) = reference_window() else { return };

    let needle = "move.w  (a2, d2.w), d2";
    assert!(
        main.contains(needle),
        "probe (a) has gone stale: `{needle}` is no longer in raster.emp. This probe must \
         doctor one of the module's INDEXED addressing modes — the instructions hand-decoded \
         from ROM in Parcel P-b — so re-point it rather than deleting it."
    );
    let doctored = main.replacen(needle, "move.w  (a2, d3.w), d2", 1);

    let clean = compile(&main, &deps, pins::RASTER.plain_base, true)
        .expect("clean compile must resolve");
    let dirty = compile(&doctored, &deps, pins::RASTER.plain_base, true)
        .expect("doctored compile must still resolve");

    let clean_b = &clean.section("raster").expect("clean raster section").bytes;
    let dirty_b = &dirty.section("raster").expect("doctored raster section").bytes;

    assert_eq!(clean_b.as_slice(), expected.as_slice(), "the CLEAN build must match the reference");
    assert_ne!(
        dirty_b, clean_b,
        "probe (a): doctoring the index register of `{needle}` produced IDENTICAL bytes — \
         raster_port's byte-diff would then be vacuous for this instruction class"
    );
}

/// (b) Without the inbound RAM carriers the link must fail loudly, naming a missing
/// cross-seam symbol. This is what makes the carrier list an assertion about the module's
/// seam surface rather than a list of names somebody once typed.
#[test]
fn b_missing_cross_seam_carriers_fail_loud() {
    let Some((main, deps)) = real_sources() else { return };

    let err = compile(&main, &deps, pins::RASTER.plain_base, false)
        .expect_err("probe (b): compiling WITHOUT the inbound carriers must FAIL");

    assert!(
        err.iter().any(|d| d.level == Level::Error),
        "probe (b): diagnostics must include an Error, got {err:?}"
    );
    let joined = err.iter().map(|d| d.message.clone()).collect::<Vec<_>>().join("\n");
    assert!(
        joined.contains("not defined in this link"),
        "probe (b): expected the cross-seam-standalone framing, got:\n{joined}"
    );
    assert!(
        joined.contains("Raster_") || joined.contains("Camera_Y") || joined.contains("Pal"),
        "probe (b): the diagnostic must NAME the missing cross-seam symbol, got:\n{joined}"
    );
}

/// (c) A wrong-base map must stop the bytes matching — the byte-diff is a placement check,
/// not an echo of the reference.
#[test]
fn c_wrong_base_stops_matching() {
    let Some((main, deps)) = real_sources() else { return };
    let Some(expected) = reference_window() else { return };

    let wrong = compile(&main, &deps, pins::RASTER.plain_base + 0x100, true)
        .expect("wrong-base compile must still resolve");
    let bytes = &wrong.section("raster").expect("raster section").bytes;

    assert_ne!(
        bytes.as_slice(),
        expected.as_slice(),
        "probe (c): the module linked at a WRONG base still matched the reference window — \
         raster_port would then not be checking placement at all"
    );
}
