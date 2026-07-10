//! Tranche 4 — negative probes for `particle_anims_port.rs` (the house
//! one-file-per-tranche style; `sonic_anims` joins when its port lands).
//!
//! (a) genuineness — a doctored COPY of the local `AF_DELETE` const produces
//!     DIFFERENT linked bytes than the reference (the byte-diff gate is
//!     non-vacuous; the `extern()` drift guard would ALSO catch it — two
//!     independent tripwires).
//! (b) standalone-compile missing-symbol — the drift guard's
//!     `extern("AF_DELETE")` names a genuinely-missing symbol when compiled
//!     without the synthetic AS equ carrier: the assert FAILS LOUD.
//! (c) placement genuineness — a wrong-base map moves the section.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::Level;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    std::env::var("AEON_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/volence/sonic_hacks/aeon"))
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

fn real_src() -> Option<String> {
    let path = aeon_dir().join("games/sonic4/data/animations/particle_anims.emp");
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but {} missing", path.display()),
        Err(_) => {
            eprintln!("skip: {} not found (set AEON_DIR)", path.display());
            None
        }
    }
}

fn read_reference() -> Option<Vec<u8>> {
    let path = aeon_dir().join("s4.bin");
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but {} missing", path.display()),
        Err(_) => {
            eprintln!("skip: {} not found (set AEON_DIR)", path.display());
            None
        }
    }
}

/// Parse + lower `src` and place into a two-region map (probes run
/// plain-shape only — the shape axis is the port gate's job). Returns the
/// placed sections and the module's link asserts.
fn place(src: &str, base: &str) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let (module, ldiags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(aeon_dir()),
            embed_base: None,
            defines: vec![],
        },
    );
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors: {ldiags:?}");
    let map_toml = format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"particle_anims\"\n\
         lma_base = {base}\n\
         size = 0x8\n\
         kind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place errors: {pdiags:?}");
    (sections, module.link_asserts)
}

fn link_bytes(sections: &[Section]) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link: {d:?}"));
    linked.section("particle_anims").expect("particle_anims section").bytes.clone()
}

/// (a) Doctor the local `AF_DELETE` const $FB -> $FA: the inline body's
/// despawn byte changes, so the linked bytes must DIFFER from the reference
/// window. FALSIFIED by the port gate (undoctored == reference).
#[test]
fn doctored_af_delete_produces_different_bytes() {
    let Some(src) = real_src() else { return };
    let Some(refrom) = read_reference() else { return };
    assert!(
        src.contains("const AF_DELETE = $FB"),
        "precondition: the port spells `const AF_DELETE = $FB`"
    );
    let doctored = src.replace("const AF_DELETE = $FB", "const AF_DELETE = $FA");
    let (sections, _asserts) = place(&doctored, "0x309EC");
    assert_ne!(
        link_bytes(&sections),
        refrom[0x309EC..0x309F4].to_vec(),
        "a drifted AF_DELETE const must NOT byte-match the reference"
    );
}

/// (b) The drift guard compiled WITHOUT the AS-side equ carrier: checking
/// the module's link asserts against an empty symbol table must FAIL LOUD,
/// naming the missing `AF_DELETE`.
#[test]
fn standalone_drift_guard_fails_loud_on_the_missing_extern() {
    let Some(src) = real_src() else { return };
    let (sections, asserts) = place(&src, "0x309EC");
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &asserts);
    assert!(
        diags.iter().any(|d| d.level == Level::Error && d.message.contains("AF_DELETE")),
        "the extern drift guard must fail loud without the AS equ, got {diags:?}"
    );
}

/// (c) A wrong-base map moves the section — the placed LMA tracks the map,
/// not an echo. FALSIFIED by the port gate placing at the true `0x309EC`.
#[test]
fn wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_src() else { return };
    let (sections, _asserts) = place(&src, "0x309EE");
    let sec = sections
        .iter()
        .find(|s| s.name == "particle_anims")
        .expect("placed particle_anims section");
    assert_eq!(sec.lma, 0x309EE, "the placed LMA must track the (doctored) map base");
    assert_ne!(sec.lma, 0x309EC, "…and therefore differ from the true pin");
}
