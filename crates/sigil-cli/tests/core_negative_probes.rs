//! Tranche 10 (1b) — negative probes: `core_port.rs`'s gate must fail LOUDLY
//! when violated, mirroring `dplc_negative_probes.rs` / `hblank_negative_probes.rs`.
//!
//! Each probe doctors ONE input so ONE specific guard fires, and — implicitly,
//! by the test PASSING rather than aborting — that no probe itself panics
//! uncontrolled. The real `core.emp` file is read but never written; every
//! probe doctors a COPY. Probes compile the PLAIN shape (`DEBUG=0`).
//!
//! ## Probes
//!
//! (a) genuineness — a doctored COPY (`asl.l   #8, d0` -> `asl.l   #7, d0`, a
//!     shift-count field change) produces DIFFERENT linked bytes than the
//!     reference, proving `core_port.rs`'s byte-diff actually fires.
//! (b) standalone-compile diagnostic — compile the real `core.emp` WITHOUT the
//!     synthetic RAM-label / proc cross-seam sections: `resolve_layout` fails
//!     LOUD naming a missing symbol.
//! (c) placement genuineness — a wrong-base map places the section at a
//!     different address, so a byte-diff against the FIXED reference window
//!     would fail.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_span::Level;
use std::path::PathBuf;

fn aeon() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn core_dir() -> PathBuf {
    aeon().join("engine/objects")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The plain-shape cross-seam labels core references (RAM abs.w EAs + the
/// Draw_Sprite proc). The plain shape elides Debug_AssertObjLoop, so no MDDBG__*.
const PLAIN_LABELS: &[(&str, u32)] = &[
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
    ("Dynamic_Live_Pending", pins::DYNAMIC_LIVE_PENDING.plain),
    ("Dynamic_Live_Pending_Count", pins::DYNAMIC_LIVE_PENDING_COUNT.plain),
];

/// The ambient deps prepended so `Sst.<field>(a0)` + the engine constants
/// resolve — types + sst + constants, under core.emp's module header.
fn core_with_ambient(core_src: &str) -> sigil_frontend_emp::ast::File {
    let read = |p: PathBuf| {
        let s = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        let (f, d) = parse_str(&s);
        assert!(d.iter().all(|x| x.level != Level::Error), "{} parse: {d:?}", p.display());
        f
    };
    let types = read(aeon().join("engine/system/types.emp"));
    let sst = read(aeon().join("engine/objects/sst.emp"));
    let constants = read(aeon().join("engine/system/constants.emp"));
    let (core, cdiags) = parse_str(core_src);
    assert!(cdiags.iter().all(|x| x.level != Level::Error), "core parse: {cdiags:?}");
    let mut items = Vec::new();
    items.extend(types.items);
    items.extend(sst.items);
    items.extend(constants.items);
    items.extend(core.items);
    sigil_frontend_emp::ast::File {
        module: core.module.clone(),
        attrs: core.attrs.clone(),
        items,
        docs: core.docs.clone(),
    }
}

fn real_core_src() -> Option<String> {
    let path = core_dir().join("core.emp");
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but missing: {}", path.display()),
        Err(_) => {
            eprintln!("skip: core.emp not at {} (set AEON_DIR)", path.display());
            None
        }
    }
}

fn map_toml(base: &str) -> String {
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
         lma_base = {base}\n\
         size = {:#x}\n\
         kind = \"rom\"\n",
        pins::CORE.plain_len
    )
}

/// The synthetic AS-side plain-shape cross-seam labels, phased to their VMAs +
/// the AS equ blob (types/sst/constants guards read it).
fn cross_seam_groups() -> Vec<Vec<Section>> {
    let mut groups: Vec<Vec<Section>> =
        vec![sigil_harness::test_support::as_engine_constants_and_sst_equs()];
    for (name, vma) in PLAIN_LABELS {
        let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        groups.push(
            assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}")).sections,
        );
    }
    groups
}

/// Parse -> lower (DEBUG=0) -> place `src` at `base`, WITHOUT cross-seam
/// sections appended.
fn place_core(src: &str, base: &str) -> Vec<Section> {
    let file = core_with_ambient(src);
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(core_dir()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), 0)],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors: {ldiags:?}");

    let map = sigil_link::load_map(&map_toml(base)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place_sections: {pdiags:?}");
    sections
}

/// Link `sections` plus the synthetic cross-seam groups (probes (a)/(c) need
/// every RAM/proc operand to resolve to compile at all).
fn link_placed(mut sections: Vec<Section>) -> sigil_link::LinkedImage {
    let mut lma = 0x0100_0000u32;
    for mut group in cross_seam_groups() {
        for sec in &mut group {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.append(&mut group);
        lma += 0x10_0000;
    }
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link: {d:?}"))
}

// ===========================================================================
// Probe (a) — GENUINENESS
// ===========================================================================

#[test]
fn doctored_shift_count_produces_different_bytes_than_genuine() {
    let Some(src) = real_core_src() else { return };
    assert!(src.contains("asl.l   #8, d0"), "precondition: the real file spells `asl.l   #8, d0`");
    let doctored = src.replacen("asl.l   #8, d0", "asl.l   #7, d0", 1);
    assert_ne!(src, doctored, "doctoring must actually change the source");

    let genuine = link_placed(place_core(&src, &format!("{:#x}", pins::CORE.plain_base)));
    let doctored = link_placed(place_core(&doctored, &format!("{:#x}", pins::CORE.plain_base)));

    let genuine_bytes = &genuine.section("core").expect("core section").bytes;
    let doctored_bytes = &doctored.section("core").expect("core section").bytes;
    assert_ne!(
        genuine_bytes, doctored_bytes,
        "a doctored `asl.l #7` must emit different bytes than the genuine `asl.l #8` — \
         else the byte gate could never catch this transcription class"
    );
}

// ===========================================================================
// Probe (b) — STANDALONE-COMPILE DIAGNOSTIC
// ===========================================================================

#[test]
fn standalone_compile_without_cross_seam_labels_is_a_loud_missing_symbol_error() {
    let Some(src) = real_core_src() else { return };
    let sections = place_core(&src, &format!("{:#x}", pins::CORE.plain_base));
    // NO cross-seam labels appended — the RAM/proc symbols are genuinely absent.
    let err = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true).expect_err(
        "compiling core.emp standalone (no RAM/proc cross-seam sections) \
         must be a loud resolve_layout error, not a silent/panicking one",
    );
    assert!(
        err.iter().any(|d| {
            d.level == Level::Error
                && d.message.contains("core")
                && d.message.contains("not defined in this link")
        }),
        "expected a loud cross-seam-standalone diagnostic naming `core`, got: {err:?}"
    );
}

// ===========================================================================
// Probe (c) — PLACEMENT GENUINENESS
// ===========================================================================

#[test]
fn wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_core_src() else { return };

    let real_sections = place_core(&src, &format!("{:#x}", pins::CORE.plain_base));
    let wrong_sections = place_core(&src, "0x2798");

    let real_core = real_sections.iter().find(|s| s.name == "core").expect("real core section");
    let wrong_core = wrong_sections.iter().find(|s| s.name == "core").expect("wrong core section");

    assert_eq!(real_core.lma, pins::CORE.plain_base, "the real map must place core at $2794");
    assert_eq!(wrong_core.lma, 0x2798, "the doctored map must place core at $2798");
    assert_ne!(
        real_core.lma, wrong_core.lma,
        "placement must genuinely move with the map base — not be an echo/hardcode"
    );

    let real_linked = link_placed(real_sections);
    let wrong_linked = link_placed(wrong_sections);
    let real_bytes = &real_linked.section("core").expect("core").bytes;
    let wrong_bytes = &wrong_linked.section("core").expect("core").bytes;
    // core is NOT placement-invariant: its `bsr.w Draw_Sprite` (and the
    // `.run_culled` bsr.w) resolve to FIXED external/backward VMAs, so their
    // pc-relative displacements shift when the section base moves. A wrong base
    // therefore emits DIFFERENT bytes — a stronger placement-genuineness proof
    // than the position-independent modules (a byte-diff against the FIXED
    // reference window would fail loudly).
    assert_ne!(
        real_bytes, wrong_bytes,
        "a wrong base must shift the pc-relative branch displacements — placement is real"
    );
    assert_ne!(
        real_linked.section("core").unwrap().lma,
        wrong_linked.section("core").unwrap().lma,
        "the LMA must differ between the two placements — placement is real, not an echo"
    );
}
