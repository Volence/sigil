//! Tranche 4 — the REAL `particle_anims.emp` port, region-level byte gate.
//!
//! The SEVENTH code/data port and the campaign's first GAME-DATA module
//! (`games/sonic4/data/animations/` — every prior port was `engine/*` or
//! sound): compiles the ACTUAL ported file from aeon's tree through the
//! production parse -> lower -> place -> resolve -> link pipeline and
//! asserts the `particle_anims` section's flattened bytes equal the
//! reference ROM window at the pinned addresses, in BOTH build shapes.
//!
//! ## What this port exercises that the prior ones did not
//!
//! - **The `offsets` construct in a REAL file** — the first in-tree consumer
//!   of §4.7 (table word + INLINE body): `dc.w Ani_Particle_Flash-
//!   Ani_Particle` becomes the construct's self-relative `RelWord16Be`
//!   word, and the body rides inline as `Flash: [u8; 5] = [...]`. The AS
//!   twin's hand `if (End-Base) > $7FFF` guard is subsumed by the
//!   construct's link-time range check.
//! - **`align 2` at item position after an offsets block** — the twin's
//!   trailing `align 2` pad byte (the 8th byte of the window).
//! - **Data-region placement** — the block lives PAST `org $10000`
//!   (plain `$309EC`, debug `$30A54`), so engine-block drift cannot move
//!   it; only data-region drift re-pins.
//!
//! ## The cross-seam surface
//!
//! INBOUND: `AF_DELETE` ($FB) is an AS-side equ in
//! `engine/objects/animate.asm` — the port carries a LOCAL const mirror
//! plus an `ensure(extern("AF_DELETE") == AF_DELETE)` drift guard, checked
//! here against a synthetic equ carrier (and against the REAL tree by the
//! mixed gate).
//!
//! OUTBOUND: `Ani_Particle` is consumed by
//! `games/sonic4/objects/test_particle.asm` (`move.l #Ani_Particle,
//! SST_anim_table(a0)`) — the bare-name proof builds the same `dc.l`-shaped
//! consumer and asserts the fixup resolves to the per-shape base.
//!
//! ## Reference windows
//!
//! Plain (map base `$309DE`): `s4.bin[0x309DE..0x309E6]` (8 bytes).
//! Debug (map base `$30A46`): `s4.debug.bin[0x30A46..0x30A4E]` (8 bytes).
//! Content is shape-invariant (`00 02 | 04 02 02 02 FB | 00`).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test particle_anims_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn anims_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("games/sonic4/data/animations")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` carrier region plus the real `particle_anims` region
/// pinned at the per-shape reference base, sized to the 8-byte block.
fn map_toml(debug: bool) -> String {
    let base = if debug { "0x30A46" } else { "0x309DE" };
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
         name = \"particle_anims\"\n\
         lma_base = {base}\n\
         size = 0x8\n\
         kind = \"rom\"\n"
    )
}

/// The AS-side truths the CONSTANTS TWIN's guards read back through
/// `extern()` — AF_DELETE now arrives via `use engine.constants.{AF_DELETE}`
/// (tranche-6 step 4 de-mirrored the local copy), and the twin's 11 guards
/// ride the ambient prepend, so all 11 truths are supplied. A trailing
/// label+`dc.w` opens a section so the equs flush via `pending_equ_syms`.
fn as_af_equ() -> Vec<Section> {
    // The 19-value constants-twin blob (SOURCE OF TRUTH: `constants.asm`),
    // consolidated in `sigil_harness::test_support` — supplies AF_DELETE plus
    // every other constant the twin's guards read.
    sigil_harness::test_support::as_engine_constants_equs()
}

/// The synthetic AS-side OUTBOUND consumer — the bare-name proof, mirroring
/// `test_particle.asm`'s `move.l #Ani_Particle, ...` anim-table pointer as a
/// `dc.l` cell.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tdc.l   Ani_Particle\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (consumer): {d:?}")).sections
}

/// Lower the real `particle_anims.emp` -> place into the per-shape map ->
/// append the synthetic cross-seam sections at harness-private LMAs -> ONE
/// `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = anims_dir();
    let src = std::fs::read_to_string(dir.join("particle_anims.emp"))
        .unwrap_or_else(|e| panic!("cannot read particle_anims.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "particle_anims.emp parse errors: {pdiags:?}"
    );
    // AF_DELETE arrives from the constants twin via `use` — prepend the
    // twin's items (the controllers_port ambient technique). Its 11 guards
    // ride along and are checked against `as_af_equ`'s truths.
    let constants_src = std::fs::read_to_string(
        dir.ancestors().nth(4).expect("anims dir is four levels below the aeon root")
            .join("engine/system/constants.emp"),
    )
    .unwrap_or_else(|e| panic!("cannot read constants.emp: {e}"));
    let (constants_file, cdiags) = parse_str(&constants_src);
    assert!(
        cdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "constants.emp parse errors: {cdiags:?}"
    );
    let file = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: constants_file.items.into_iter().chain(file.items).collect(),
        docs: file.docs.clone(),
    };

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = as_af_equ();
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    let mut consumer = as_outbound_consumer();
    for sec in &mut consumer {
        sec.lma = 0x0300_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(consumer);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// The twin's AF_DELETE guard must be captured (riding the ambient prepend)
/// and PASS against `as_af_equ`'s value. The `align 2` congruence assert
/// (D2.29) also rides `module.link_asserts`, so the guard is identified by
/// its own message text rather than by count.
fn assert_drift_guard(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = link_asserts
        .iter()
        .filter(|a| {
            a.message.iter().any(|p| {
                matches!(p, sigil_ir::assert::MsgPart::Text(t) if t.contains("disagree on AF_DELETE"))
            })
        })
        .count();
    assert_eq!(guards, 1, "the AF_DELETE drift guard must be captured");
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "every link assert (drift guard + align congruence) must PASS: {diags:?}"
    );
}

fn gate(debug: bool, rom_name: &str, base: usize) {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(debug);
    assert_drift_guard(&resolved, &link_asserts);

    let expected = &refrom[base..base + 8];
    let section =
        linked.section("particle_anims").expect("linked image must carry particle_anims");
    assert_eq!(
        &section.bytes[..],
        expected,
        "particle_anims ({}) vs {rom_name}[{base:#X}..{:#X}]",
        if debug { "debug" } else { "plain" },
        base + 8
    );

    // Bare-name proof: the `dc.l Ani_Particle` consumer resolves to the
    // per-shape base.
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0300_0000)
        .expect("linked image must carry the outbound consumer");
    let ptr = u32::from_be_bytes([
        consumer.bytes[0],
        consumer.bytes[1],
        consumer.bytes[2],
        consumer.bytes[3],
    ]);
    assert_eq!(
        ptr as usize, base,
        "bare-name proof: `dc.l Ani_Particle` must resolve to {base:#X}"
    );
}

#[test]
fn particle_anims_region_matches_reference() {
    gate(false, "s4.bin", 0x309DE);
}

#[test]
fn particle_anims_debug_region_matches_reference() {
    gate(true, "s4.debug.bin", 0x30A46);
}
