//! Parcel K4 — the `$100-$1FF` ROM header, region-level byte gate.
//!
//! The header (was `engine/system/header.inc`'s `gameHeader` macro) is a native
//! `.emp` section: ONE `GameHeader: RomHeader` struct literal whose `[u8; N]` field
//! types ARE the exact-width guards (a wrong-length string fails to lower — the nine
//! `strlen … fatal` walls became the types). Per-game (`games.sonic4.header` /
//! `games.demo.header`); boundary key `GameHeader` ($100).
//!
//! Two cells are sigil-patched post-pipeline (derived from the final image size):
//! the checksum ($18E) and the ROM-end pointer ($1A4). The `.emp` emits 0 and
//! `EndOfRom-1`; those 6 bytes are excluded from the region compare (the whole-ROM
//! gates prove the patched values).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test header_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

const BASE: u32 = 0x100;
const LEN: usize = 0x100;
// The sigil-patched cells (region-relative): checksum $18E, ROM-end $1A4.
const PATCHED: &[std::ops::Range<usize>] = &[0x8E..0x90, 0xA4..0xA8];

fn aeon_root() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// EndOfRom seam — the header's `rom_end` reads it, but that cell is patched (and
/// excluded from the compare), so any resolvable value links.
fn endofrom_seam() -> Vec<Section> {
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble("cpu 68000\nEndOfRom = $5DB00\nStub:\n\tdc.w 0\n", &opts)
        .expect("seam assemble")
        .sections
}

fn compile(emp_rel: &str) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    let src = std::fs::read_to_string(aeon.join(emp_rel))
        .unwrap_or_else(|e| panic!("read {emp_rel}: {e}"));
    let (file, pd) = parse_str(&src);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "{emp_rel} parse: {pd:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ld) = lower_module(&file, &opts);
    assert!(ld.iter().all(|d| d.level != sigil_span::Level::Error), "{emp_rel} lower: {ld:?}");

    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"header\"\nlma_base = {BASE:#x}\nsize = {LEN:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map).expect("map");
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "{emp_rel} place: {pd:?}");

    let mut seam = endofrom_seam();
    for s in &mut seam {
        s.lma = 0x0100_0000;
        s.placement = SectionPlacement::Pinned;
        s.group = None;
    }
    sections.extend(seam);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("{emp_rel} resolve: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("{emp_rel} link: {d:?}"))
}

fn gate(emp_rel: &str, rom_name: &str) {
    let rom_path = aeon_root().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    // HEADER pin must agree with this test's window.
    assert_eq!(pins::HEADER.plain_base, BASE);
    assert_eq!(pins::HEADER.plain_len, LEN);

    let linked = compile(emp_rel);
    let sec = linked.section("header").expect("linked image must carry `header`");
    assert_eq!(sec.bytes.len(), LEN, "header must emit {LEN:#x} bytes");
    let expected = &refrom[BASE as usize..BASE as usize + LEN];
    for i in 0..LEN {
        if PATCHED.iter().any(|r| r.contains(&i)) {
            continue; // sigil-patched cell
        }
        if sec.bytes[i] != expected[i] {
            panic!(
                "header ({rom_name}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
                &sec.bytes[i.saturating_sub(4)..(i + 8).min(LEN)],
                &expected[i.saturating_sub(4)..(i + 8).min(LEN)]
            );
        }
    }
}

#[test]
fn sonic4_header_matches_reference() {
    gate("games/sonic4/config/header.emp", "s4.bin");
}

#[test]
fn demo_header_matches_reference() {
    gate("games/demo/config/header.emp", "demo.bin");
}
