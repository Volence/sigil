//! t28 lane M — the DEMAND-3 whole-ROM OFF-CANONICAL mixed-build machinery.
//!
//! Three consumers are CANONICALLY EMPTY (`game_debug`, `sound_debug`, `z80_init`)
//! — they emit ZERO bytes in every shipped shape, so `build.sh` never produces a
//! reference ROM containing them. The existing whole-ROM gates (`mixed_dac_rom`)
//! diff against the real asl ROM FILE (`aeon/s4.bin`, which carries the convsym
//! `deb2` append). There is NO off-canonical asl reference file.
//!
//! So the reference here is built through sigil's OWN AS front-end at the
//! off-canonical config (`assemble_root`, byte-identical to asl per M0/M1) — the
//! whole-ROM lift of the `game_debug_port`/`vblank_mirror_shape_twin_parity`
//! window oracles, which already build their own AS reference. Both the reference
//! (pure `.asm`) and the mixed (`.asm` gated + `.emp` placed) go through
//! `assemble_root`+`flatten`, so BOTH are PRE-convsym: their headers MATCH, and
//! the comparison uses `assert_rom_matches` with an EMPTY allowlist — NOT
//! `assert_rom_matches_convsym`, whose "did the reference lose its deb2 append?"
//! guard would FALSE-FAIL two pre-append ROMs. deb2-append coverage stays with
//! the canonical gates (`mixed_dac_rom`); the empty allowlist is a HARD BAR —
//! any needed allowlist entry is a STOP-and-report, not an addition (overseer
//! ruling 1, t28 checkpoint (a)).
//!
//! The window oracles are KEPT (overseer ruling 2): they carry the cheap
//! doctored-twin-diverge falsification the whole-ROM gate can't express; this
//! gate is ADDITIVE placement + cross-seam proof, not a replacement.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness --test mixed_offcanonical_rom
//! ```

use sigil_frontend_as::{assemble_root, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::assert_rom_matches;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// Assemble the WHOLE ROM through sigil's AS front-end at the given defines and
/// flatten to bytes. `gates` adds the `SIGIL_EMP_*` gate defines (empty for the
/// pure reference).
fn assemble_offcanonical(aeon: &Path, defines: &[(&str, i64)], gates: &[&str]) -> sigil_ir::Module {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defs: Vec<(String, i64)> = defines.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    for g in gates {
        defs.push((g.to_string(), 1));
    }
    let opts = AsOptions { initial_cpu: Cpu::M68000, defines: defs, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).unwrap_or_else(|d| {
        panic!("off-canonical AS assemble failed: {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

fn flatten_module(sections: Vec<Section>, label: &str) -> Vec<u8> {
    flatten_with_asserts(sections, &[], label)
}

/// Resolve + check the `.emp` drift-guard link asserts (they MUST all pass — a
/// firing guard is a STOP) + link + flatten.
fn flatten_with_asserts(sections: Vec<Section>, asserts: &[sigil_ir::LinkAssert], label: &str) -> Vec<u8> {
    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &stubs, true)
        .unwrap_or_else(|d| panic!("{label}: resolve_layout failed: {d:?}"));
    let adiags = sigil_link::check_link_asserts(&resolved, &stubs, asserts);
    assert!(
        adiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{label}: drift-guard ensure(s) FIRED: {adiags:?}"
    );
    let linked = sigil_link::link(&resolved, &stubs)
        .unwrap_or_else(|d| panic!("{label}: link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Lower a `.emp` module and place its single named section at `lma_base`
/// (size `region_size`). Returns the placed sections + the module's link
/// asserts (the drift guards, checked against the joint symbol table).
fn placed_emp(
    aeon: &Path,
    rel_path: &str,
    section: &str,
    lma_base: u32,
    region_size: u32,
    emp_defines: &[(&str, i64)],
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let path = aeon.join(rel_path);
    let dir = path.parent().unwrap().to_path_buf();
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {rel_path}: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{rel_path} parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir),
        embed_base: None,
        defines: emp_defines.iter().map(|(k, v)| (k.to_string(), i128::from(*v))).collect(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{rel_path} lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    let link_asserts = module.link_asserts.clone();
    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"{section}\"\nlma_base = {lma_base:#x}\nsize = {region_size:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map).expect("map loads");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{rel_path} place_sections errors: {pdiags:?}"
    );
    (sections, link_asserts)
}

fn maybe_skip(aeon: &Path) -> bool {
    if !strict_gate() && !aeon.join("s4.bin").exists() {
        eprintln!("skip: aeon tree not present (set AEON_DIR / SIGIL_STRICT_GATE)");
        return true;
    }
    false
}

// Config A (debug-side): __DEBUG__ + SOUND_DRIVER_ENABLED + SOUND_DEBUG_HOTKEYS
// + SOUND_DBG_MIRROR. One combined build serves game_debug AND sound_debug
// (t26 intent, ruled). Config A is inherently DEBUG (hotkeys/mirror require it),
// so there is ONE shape — no per-shape org.
const CONFIG_A: &[(&str, i64)] = &[
    ("__DEBUG__", 1),
    ("SOUND_DRIVER_ENABLED", 1),
    ("SOUND_DEBUG_HOTKEYS", 1),
    ("SOUND_DBG_MIRROR", 1),
];
// game_debug.emp reads DEBUG / SOUND_DEBUG_HOTKEYS / SOUND_DRIVER_ENABLED.
const GAME_DEBUG_EMP_DEFS: &[(&str, i64)] =
    &[("DEBUG", 1), ("SOUND_DEBUG_HOTKEYS", 1), ("SOUND_DRIVER_ENABLED", 1)];

// game_debug region at Config A: [Debug_MusicToggle=0x6356, Section_Init=0x6408).
const GAME_DEBUG_BASE: u32 = 0x6356;
const GAME_DEBUG_END: u32 = 0x6408;

/// THE whole-ROM off-canonical gate for game_debug (Config A): game_debug.emp
/// placed in the real ROM == pure-asl game_debug.asm at the SAME config.
#[test]
fn mixed_game_debug_config_a_rom_matches_reference() {
    let aeon = aeon_dir();
    if maybe_skip(&aeon) {
        return;
    }
    // Reference: pure `.asm` at Config A (no gate).
    let refrom = flatten_module(assemble_offcanonical(&aeon, CONFIG_A, &[]).sections, "ref game_debug");

    // Mixed: `.asm` gated (game_debug.asm skipped, org resumes) + game_debug.emp placed.
    let mut sections = assemble_offcanonical(&aeon, CONFIG_A, &["SIGIL_EMP_GAME_DEBUG"]).sections;
    let (emp_sections, asserts) = placed_emp(
        &aeon,
        "games/sonic4/debug/game_debug.emp",
        "game_debug",
        GAME_DEBUG_BASE,
        GAME_DEBUG_END - GAME_DEBUG_BASE,
        GAME_DEBUG_EMP_DEFS,
    );
    sections.extend(emp_sections);
    // The drift-guard ensures (game_debug.emp's 16 game-const guards) run and
    // must pass against the JOINT symbol table (the AS side supplies the truths).
    assert!(!asserts.is_empty(), "game_debug.emp must carry its drift-guard ensures");
    let rom = flatten_with_asserts(sections, &asserts, "mixed game_debug");

    assert_eq!(rom.len(), refrom.len(), "off-canonical ROM lengths must match");
    // Empty allowlist HARD BAR: both sides pre-convsym, headers must match too.
    assert_rom_matches(&rom, &refrom, refrom.len(), &[], "off-canonical Config A: game_debug");
}
