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
/// (sections, link asserts).
fn place_sonic(src: &str) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
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
         name = \"sonic_anims\"\n\
         lma_base = 0x30978\n\
         size = 0x74\n\
         kind = \"rom\"\n";
    let map = sigil_link::load_map(map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place errors: {pdiags:?}");
    (sections, module.link_asserts)
}

/// The AS-side equs at their genuine values (the port gate's set).
fn sonic_equs() -> Vec<Section> {
    let asm = "cpu 68000\n\
               AF_END = $FF\n\
               AF_BACK = $FE\n\
               DUR_DYNAMIC = $FF\n\
               ANIM_WALK = 0\n\
               ANIM_RUN = 1\n\
               ANIM_ROLL = 2\n\
               ANIM_SPINDASH = 3\n\
               ANIM_PUSH = 4\n\
               ANIM_IDLE = 5\n\
               ANIM_BALANCE = 6\n\
               ANIM_LOOKUP = 7\n\
               ANIM_DUCK = 8\n\
               ANIM_SKID = 9\n\
               ANIM_GETUP = 10\n\
               ANIM_COUNT = 11\n\
               Stub:\n\
               \tdc.w 0\n";
    let opts = sigil_frontend_as::Options { initial_cpu: Cpu::M68000, ..Default::default() };
    sigil_frontend_as::assemble(asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (sonic equs): {d:?}"))
        .sections
}

/// THE ORDINALS-STORY NEGATIVE: swap the Walk/Run member order — the table
/// bytes change AND the ordinal drift guards fire (Walk would read ordinal
/// 1 against ANIM_WALK = 0). Declaration position IS the id; a reorder
/// cannot silently pass.
#[test]
fn reordered_members_trip_the_ordinal_guards() {
    let Some(src) = sonic_src() else { return };
    let doctored = src.replace(
        "    Walk:     Ani_Sonic_Walk,\n    Run:      Ani_Sonic_Run,",
        "    Run:      Ani_Sonic_Run,\n    Walk:     Ani_Sonic_Walk,",
    );
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
         name = \"sonic_anims\"\n\
         lma_base = 0x3097A\n\
         size = 0x74\n\
         kind = \"rom\"\n";
    let map = sigil_link::load_map(map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place errors: {pdiags:?}");
    let sec = sections.iter().find(|s| s.name == "sonic_anims").expect("placed sonic_anims");
    assert_eq!(sec.lma, 0x3097A, "the placed LMA must track the (doctored) map base");
    assert_ne!(sec.lma, 0x30978, "…and therefore differ from the true pin");
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
         lma_base = 0x14AEE\n\
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
         lma_base = 0x14AF0\n\
         size = 0x274\n\
         kind = \"rom\"\n";
    let map = sigil_link::load_map(map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place errors: {pdiags:?}");
    let sec = sections.iter().find(|s| s.name == "act_descriptor").expect("placed section");
    assert_eq!(sec.lma, 0x14AF0, "the placed LMA must track the (doctored) map base");
    assert_ne!(sec.lma, 0x14AEE, "…and therefore differ from the true pin");
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
