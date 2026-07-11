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

fn constants_src() -> Option<String> {
    let path = aeon_dir().join("engine/system/constants.emp");
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

/// Parse + lower `src` (with `constants_src`'s items prepended — AF_DELETE
/// arrives from the constants twin via `use` since the tranche-6 step-4
/// de-mirror, so the twin rides ambient exactly as in the port gate) and
/// place into a two-region map (probes run plain-shape only — the shape
/// axis is the port gate's job). Returns the placed sections and the
/// module's link asserts.
fn place(constants_src: &str, src: &str, base: &str) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let (cfile, cdiags) = parse_str(constants_src);
    assert!(cdiags.iter().all(|d| d.level != Level::Error), "constants parse errors: {cdiags:?}");
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let file = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: cfile.items.into_iter().chain(file.items).collect(),
        docs: file.docs.clone(),
    };
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

/// (a) Doctor the CONSTANTS TWIN's `AF_DELETE` $FB -> $FA (the mirror moved
/// there in tranche-6 step 4): the inline body's despawn byte changes
/// THROUGH the import, so the linked bytes must DIFFER from the reference
/// window. FALSIFIED by the port gate (undoctored == reference).
#[test]
fn doctored_af_delete_produces_different_bytes() {
    let Some(src) = real_src() else { return };
    let Some(constants) = constants_src() else { return };
    let Some(refrom) = read_reference() else { return };
    // (column-aligned since the tranche-9 animation-block growth)
    assert!(
        constants.contains("pub const AF_DELETE    = $FB"),
        "precondition: the twin spells `pub const AF_DELETE    = $FB`"
    );
    let doctored = constants.replace("pub const AF_DELETE    = $FB", "pub const AF_DELETE    = $FA");
    let (sections, _asserts) = place(&doctored, &src, "0x309DE");
    assert_ne!(
        link_bytes(&sections),
        refrom[0x309DE..0x309E6].to_vec(),
        "a drifted AF_DELETE const must NOT byte-match the reference"
    );
}

/// (b) The drift guard compiled WITHOUT the AS-side equ carrier: checking
/// the module's link asserts against an empty symbol table must FAIL LOUD,
/// naming the missing `AF_DELETE`.
#[test]
fn standalone_drift_guard_fails_loud_on_the_missing_extern() {
    let Some(src) = real_src() else { return };
    let Some(constants) = constants_src() else { return };
    let (sections, asserts) = place(&constants, &src, "0x309DE");
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &asserts);
    assert!(
        diags.iter().any(|d| d.level == Level::Error && d.message.contains("AF_DELETE")),
        "the extern drift guard must fail loud without the AS equ, got {diags:?}"
    );
}

/// (c) A wrong-base map moves the section — the placed LMA tracks the map,
/// not an echo. FALSIFIED by the port gate placing at the true `0x309DE`.
#[test]
fn wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = real_src() else { return };
    let Some(constants) = constants_src() else { return };
    let (sections, _asserts) = place(&constants, &src, "0x309E6");
    let sec = sections
        .iter()
        .find(|s| s.name == "particle_anims")
        .expect("placed particle_anims section");
    assert_eq!(sec.lma, 0x309E6, "the placed LMA must track the (doctored) map base");
    assert_ne!(sec.lma, 0x309DE, "…and therefore differ from the true pin");
}

// ===========================================================================
// sonic_anims probes (port #2)
// ===========================================================================

fn sonic_src() -> Option<String> {
    let path = aeon_dir().join("games/sonic4/data/animations/sonic_anims.emp");
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but {} missing", path.display()),
        Err(_) => {
            eprintln!("skip: {} not found (set AEON_DIR)", path.display());
            None
        }
    }
}

/// Place `src` as the sonic_anims module (plain shape) and return
/// (sections, link asserts). The constants twin rides ambient — sonic_anims'
/// command bytes arrive via `use engine.constants` since the tranche-9
/// row-3 consolidation.
fn place_sonic(src: &str) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let csrc = std::fs::read_to_string(aeon_dir().join("engine/system/constants.emp"))
        .unwrap_or_else(|e| panic!("cannot read constants.emp: {e}"));
    let (cfile, cdiags) = parse_str(&csrc);
    assert!(cdiags.iter().all(|d| d.level != Level::Error), "constants parse errors: {cdiags:?}");
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let file = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: cfile.items.into_iter().chain(file.items).collect(),
        docs: file.docs.clone(),
    };
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
    let map_toml = "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"sonic_anims\"\n\
         lma_base = 0x30970\n\
         size = 0x6E\n\
         kind = \"rom\"\n";
    let map = sigil_link::load_map(map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place errors: {pdiags:?}");
    (sections, module.link_asserts)
}

/// The AS-side equs at their genuine values (the port gate's set).
fn sonic_equs() -> Vec<Section> {
    // The full constants-twin blob rides too (its 30 ensures come ambient
    // with the tranche-9 consolidation), plus the ANIM_* ordinal truths.
    let mut pairs = sigil_harness::test_support::engine_constant_equs();
    pairs.extend([
        ("ANIM_WALK", "0"),
        ("ANIM_RUN", "1"),
        ("ANIM_ROLL", "2"),
        ("ANIM_SPINDASH", "3"),
        ("ANIM_PUSH", "4"),
        ("ANIM_IDLE", "5"),
        ("ANIM_BALANCE", "6"),
        ("ANIM_LOOKUP", "7"),
        ("ANIM_DUCK", "8"),
        ("ANIM_SKID", "9"),
        ("ANIM_GETUP", "10"),
        ("ANIM_COUNT", "11"),
    ]);
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// THE ORDINALS-STORY NEGATIVE: swap the Walk/Run member order — the table
/// bytes change AND the ordinal drift guards fire (Walk would read ordinal
/// 1 against ANIM_WALK = 0). Declaration position IS the id; a reorder
/// cannot silently pass.
#[test]
fn reordered_members_trip_the_ordinal_guards() {
    let Some(src) = sonic_src() else { return };
    let walk = "    Walk:     [u8; 10] = [DUR_DYNAMIC, 7, 8, 1, 2, 3, 4, 5, 6, AF_END],\n";
    let run = "    Run:      [u8; 6]  = [DUR_DYNAMIC, $21, $22, $23, $24, AF_END],\n";
    let pair: String = format!("{walk}{run}");
    let swapped: String = format!("{run}{walk}");
    let doctored = src.replace(&pair, &swapped);
    assert_ne!(doctored, src, "precondition: the swap must apply");
    let (mut sections, asserts) = place_sonic(&doctored);
    let mut equs = sonic_equs();
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = sigil_ir::SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &asserts);
    assert!(
        diags.iter().any(|d| d.level == Level::Error
            && (d.message.contains("Walk ordinal drifted") || d.message.contains("Run ordinal drifted"))),
        "a member reorder must trip the ordinal drift guards, got {diags:?}"
    );
}

/// Placement genuineness for the second region.
#[test]
fn sonic_wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = sonic_src() else { return };
    let csrc = std::fs::read_to_string(aeon_dir().join("engine/system/constants.emp"))
        .unwrap_or_else(|e| panic!("cannot read constants.emp: {e}"));
    let (cfile, cdiags) = parse_str(&csrc);
    assert!(cdiags.iter().all(|d| d.level != Level::Error), "constants parse errors: {cdiags:?}");
    let (file, pdiags) = parse_str(&src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let file = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: cfile.items.into_iter().chain(file.items).collect(),
        docs: file.docs.clone(),
    };
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
    let map_toml = "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"sonic_anims\"\n\
         lma_base = 0x30972\n\
         size = 0x6E\n\
         kind = \"rom\"\n";
    let map = sigil_link::load_map(map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place errors: {pdiags:?}");
    let sec = sections.iter().find(|s| s.name == "sonic_anims").expect("placed sonic_anims");
    assert_eq!(sec.lma, 0x30972, "the placed LMA must track the (doctored) map base");
    assert_ne!(sec.lma, 0x30970, "…and therefore differ from the true pin");
}

// ===========================================================================
// act_descriptor probes (port #3)
// ===========================================================================

fn act_src() -> Option<String> {
    let path = aeon_dir().join("games/sonic4/data/levels/ojz/act1/act_descriptor.emp");
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but {} missing", path.display()),
        Err(_) => {
            eprintln!("skip: {} not found (set AEON_DIR)", path.display());
            None
        }
    }
}

fn place_act(src: &str) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
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
    let map_toml = "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"act_descriptor\"\n\
         lma_base = 0x14AE6\n\
         size = 0x274\n\
         kind = \"rom\"\n";
    let map = sigil_link::load_map(map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place errors: {pdiags:?}");
    (sections, module.link_asserts)
}

/// THE STRUCT-TYPING NEGATIVE: swap two same-width Sec fields in a doctored
/// copy of the STRUCT twin (`sec_objects`/`sec_rings`). `sizeof(Sec)` is
/// unchanged — the old `* == Act_len`-style size guard would PASS — but the
/// named literals now emit the pointers in the swapped order, so the byte
/// gate catches what the size guard structurally cannot. This is Tier 1's
/// whole argument in one probe.
#[test]
fn swapped_sec_fields_produce_different_bytes() {
    let Some(src) = act_src() else { return };
    let doctored = src.replace(
        "    sec_objects:         *u8,           // $04",
        "    sec_rings_swapped_probe: *u8,       // $04",
    );
    // rename-then-swap: give the doctored struct rings-before-objects by
    // swapping the two field LINES wholesale.
    let doctored = doctored.replace(
        "    sec_rings:           *u8,           // $08",
        "    sec_objects:         *u8,           // $08",
    );
    let doctored = doctored.replace(
        "    sec_rings_swapped_probe: *u8,       // $04",
        "    sec_rings:           *u8,           // $04",
    );
    assert_ne!(doctored, src, "precondition: the swap must apply");
    // The struct literals are NAMED, so they still compile — the bytes move.
    let (sections, _asserts) = place_act(&doctored);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    // Cross-seam symbols unresolved — compare the SECTION's fixup TARGETS
    // instead of linked bytes: the first Sec's field order determines which
    // symbol's fixup lands at region offset 0x22+0x04 vs 0x22+0x08. Simpler
    // and just as conclusive: the placed section's fixup list ORDER changed.
    let sec = resolved.iter().find(|s| s.name == "act_descriptor").expect("section");
    let names: Vec<String> = sec
        .fragments
        .iter()
        .filter_map(|f| match f {
            sigil_ir::Fragment::Data(d) => Some(d.fixups.iter()),
            _ => None,
        })
        .flatten()
        .filter_map(|f| match &f.target {
            sigil_ir::expr::Expr::Sym(n) => Some(n.clone()),
            _ => None,
        })
        .take(12)
        .collect();
    let obj_pos = names
        .iter()
        .position(|n| n.contains("Sec0_Objects"))
        .expect("Sec0_Objects fixup must be present");
    let ring_pos = names
        .iter()
        .position(|n| n.contains("Sec0_Rings"))
        .expect("Sec0_Rings fixup must be present");
    assert!(
        obj_pos > ring_pos,
        "the doctored struct order must emit Rings before Objects (got {names:?})"
    );
}

/// Placement genuineness for the act region.
#[test]
fn act_wrong_base_map_places_the_section_at_a_different_address() {
    let Some(src) = act_src() else { return };
    let (file, pdiags) = parse_str(&src);
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
    let map_toml = "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"act_descriptor\"\n\
         lma_base = 0x14AE8\n\
         size = 0x274\n\
         kind = \"rom\"\n";
    let map = sigil_link::load_map(map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place errors: {pdiags:?}");
    let sec = sections.iter().find(|s| s.name == "act_descriptor").expect("placed section");
    assert_eq!(sec.lma, 0x14AE8, "the placed LMA must track the (doctored) map base");
    assert_ne!(sec.lma, 0x14AE6, "…and therefore differ from the true pin");
}

/// The Act_len twin pin compiled WITHOUT the AS equs: the link-assert check
/// against an empty table must FAIL LOUD naming the missing symbol.
#[test]
fn act_standalone_twin_pins_fail_loud_on_missing_externs() {
    let Some(src) = act_src() else { return };
    let (sections, asserts) = place_act(&src);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &asserts);
    assert!(
        diags.iter().any(|d| d.level == Level::Error && d.message.contains("Act_len")),
        "the Act_len twin pin must fail loud without the AS equs, got {diags:?}"
    );
}
