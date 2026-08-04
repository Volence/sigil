//! Parcel K3 run A — the OJZ act1 interior-island HEAD, region-level byte gate.
//!
//! The contiguous AS run BEFORE the native descriptor became two native `.emp`
//! sections:
//!
//! - **entity_data** (`games.sonic4.ojz_entity_data_act1`): the 9-section type
//!   tables (count-prefixed `dc.l ObjDef_*` pointer arrays — the objentry/objend
//!   macros replaced by packed 3-word records + a `$FFFF` terminator; ring lists
//!   are `dc.w X,Y` pairs + a longword-0 terminator). `OJZ_Sec0_TypeTable` ..
//!   `OJZ_Act_Pool_Page0`. The `ObjDef_*` archetypes are cross-module link labels
//!   (test_objects / path_swap), injected here as a synthetic seam at their ROM
//!   addresses (Abs32 fixups bake addresses, so the pointer cells are load-bearing).
//! - **ojz_act_pool** (`games.sonic4.ojz_act_pool_act1`): 3 ZX0 page `embed()`s +
//!   the `OJZ_Act_Pool_PageTable` (`dc.l` page addresses, same-module `extern()`s
//!   resolved at link). `OJZ_Act_Pool_Page0` .. `OJZ_Act1_Descriptor`.
//!
//! Each section is byte-compared against the reference ROM at its pin base.
//! Content is shape-invariant (debug = same bytes at +0x88).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test ojz_run_a_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins::{self, Region};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn aeon_root() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// One `ObjDef` record — the stride between `ObjDef_Static` and `ObjDef_Solid`.
const OBJDEF_LEN: u32 = 0x1A;

/// The `ObjDef_*` seam entity_data's type-table pointers resolve against — the
/// per-shape ROM addresses the Abs32 pointer cells bake in.
///
/// PIN-DERIVED as of `cheat-flag` (2026-08-05). These were three hand-typed literal
/// pairs that had to be re-shifted by hand on every re-baseline, and they silently
/// rotted whenever someone forgot — this parcel moved the object bank +0x20 and the
/// gate failed with a one-byte diff deep inside a data blob, which is an expensive
/// way to learn that a constant is stale. The old comment already recorded the exact
/// relations (`ObjDef_Static == OBJDEFS base`, `ObjDef_Solid == base + one ObjDef`,
/// `ObjDef_PathSwap == PATH_SWAP base`), so they are now simply computed.
///
/// This is NOT circular: the seam is an INPUT to the standalone compile, and the
/// assertion compares that compile's bytes against the real built ROM. Deriving the
/// input from pins removes hand-maintenance without weakening the check.
fn objdef_seam(debug: bool) -> Vec<(&'static str, u32)> {
    let (objdefs, path_swap) = if debug {
        (pins::OBJDEFS.debug_base, pins::PATH_SWAP.debug_base)
    } else {
        (pins::OBJDEFS.plain_base, pins::PATH_SWAP.plain_base)
    };
    vec![
        ("ObjDef_Solid", objdefs + OBJDEF_LEN),
        ("ObjDef_Static", objdefs),
        ("ObjDef_PathSwap", path_swap),
    ]
}

fn seam_sections(debug: bool) -> Vec<Section> {
    let mut asm = String::from("cpu 68000\n");
    for (name, addr) in objdef_seam(debug) {
        asm.push_str(&format!("{name} = ${addr:X}\n"));
    }
    asm.push_str("Stub:\n\tdc.w 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (seam): {d:?}")).sections
}

fn map_toml(section: &str, base: u32, len: usize) -> String {
    format!(
        "fill = 0x00\n\n[[region]]\nname = \"{section}\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    )
}

fn compile_section(
    emp_rel: &str,
    section: &str,
    base: u32,
    len: usize,
    seam: bool,
    debug: bool,
) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    let path = aeon.join(emp_rel);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} parse errors: {pdiags:?}"
    );

    // embed() paths in these modules are aeon-root-relative.
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} lower errors: {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(section, base, len)).expect("map must load");
    let mut sections = module.sections;
    let place = place_sections(&mut sections, &map);
    assert!(
        place.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} place_sections errors: {place:?}"
    );

    if seam {
        let mut equs = seam_sections(debug);
        for s in &mut equs {
            s.lma = 0x0100_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
        }
        sections.extend(equs);
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("{emp_rel} resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("{emp_rel} link failed: {d:?}"))
}

/// (module `.emp` path relative to aeon, section name, pin, needs the ObjDef seam).
fn sections() -> Vec<(&'static str, &'static str, Region, bool)> {
    vec![
        (
            "games/sonic4/data/generated/ojz/act1/entity_data.emp",
            "entity_data",
            pins::ENTITY_DATA,
            true,
        ),
        (
            "games/sonic4/data/generated/ojz/act1/ojz_act_pool.emp",
            "ojz_act_pool",
            pins::OJZ_ACT_POOL,
            false,
        ),
    ]
}

fn gate(debug: bool, rom_name: &str) {
    let rom_path = aeon_root().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    for (emp_rel, section, region, seam) in sections() {
        let base = if debug { region.debug_base } else { region.plain_base };
        let len = region.plain_len; // shape-invariant
        assert_eq!(region.debug_len, region.plain_len, "{section} len must be shape-invariant");

        let linked = compile_section(emp_rel, section, base, len, seam, debug);
        let sec = linked
            .section(section)
            .unwrap_or_else(|| panic!("linked image must carry `{section}`"));
        assert_eq!(sec.bytes.len(), len, "`{section}` must emit exactly {len:#x} bytes");
        let expected = &refrom[base as usize..base as usize + len];
        if let Some(i) = (0..len).find(|&i| sec.bytes[i] != expected[i]) {
            panic!(
                "`{section}` ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
                if debug { "debug" } else { "plain" },
                &sec.bytes[i.saturating_sub(4)..(i + 8).min(len)],
                &expected[i.saturating_sub(4)..(i + 8).min(len)]
            );
        }
    }
}

#[test]
fn ojz_run_a_regions_match_reference() {
    gate(false, "s4.bin");
}

#[test]
fn ojz_run_a_debug_regions_match_reference() {
    gate(true, "s4.debug.bin");
}
