//! ENFORCEMENT (queue row SEAM2-TABLE-REGION-UNENFORCED, closed by step-0 A2): a
//! section larger than the region it was placed into is refused on every path that
//! places by region.
//!
//! This file was `seam2_region_unenforced.rs`, which pinned the gaps as current
//! behaviour (`docs/superpowers/notes/2026-09-28-seam2-region-unenforced.md`). Its
//! four `gap_*` tests are inverted here into `enforced_*` tests that assert the
//! refusal:
//! * the seam-2 per-artifact tail (`place_sections` + `resolve_layout` + `link`,
//!   no `emit_rom`) now ends in `seam2::check_placed_regions`, so a single overflow
//!   and the 1,116 B table in a `0x400` region are refused there;
//! * `MemoryMap::validate_section` holds a section to the region its placement
//!   group names, not the first ROM region containing its LMA, so a spill that
//!   starts at the next region's base and an overflow of a region nested in an
//!   earlier `rom` region are refused by `emit_rom`;
//! * a section starting inside a `z80_bank` window must end inside it (two
//!   sections summing past the window are refused).

use sigil_ir::map::MemoryMap;
use sigil_ir::{DataFragment, Fragment, Section, SectionPlacement, SymbolTable};
use sigil_link::{LinkedImage, LinkedSection};
use sigil_span::{Level, SourceId, Span};

fn data_section(name: &str, len: usize) -> Section {
    Section {
        name: name.to_string(),
        cpu: sigil_ir::Cpu::M68000,
        vma_base: None,
        lma: 0,
        space: sigil_ir::AddressSpace::Image,
        labels: vec![],
        fragments: vec![Fragment::Data(DataFragment {
            bytes: vec![0xA5; len],
            fixups: vec![],
            span: Span { source: SourceId(0), start: 0, end: 0 },
        })],
        placement: SectionPlacement::Chained,
        reserved_span: 0,
        group: None,
        bank: None,
        equ_syms: Vec::new(),
    }
}

fn map(src: &str) -> MemoryMap {
    sigil_link::load_map(src).expect("map loads")
}

/// The seam-2 synthetic map shape with the PRE-A2 literal size `0x400` (the size
/// `seam2.rs`'s `sound_tables_z80_linked_from_src` used to declare; it now derives
/// `bank_room(head_lma)`). Kept literal so the overflow is a fixed figure.
fn seam2_tables_map(head_lma: u32) -> MemoryMap {
    map(&format!(
        "fill = 0x00\n\n[[region]]\nname = \"sound_tables_z80\"\nlma_base = 0x{head_lma:X}\nsize = 0x400\nkind = \"rom\"\n"
    ))
}

const N: usize = 0x400;
const HEAD: u32 = 0xB8000;

fn errors(d: &[sigil_span::Diagnostic]) -> usize {
    d.iter().filter(|d| d.level == Level::Error).count()
}

/// The seam-2 tail exactly as the emitters run it after placement: resolve, link,
/// then the region check every seam-2 tail now ends in.
fn seam2_tail(sections: &[Section], m: &MemoryMap) -> Result<LinkedImage, String> {
    let resolved = sigil_link::resolve_layout(sections, &SymbolTable::new(), true)
        .map_err(|d| format!("resolve_layout: {d:?}"))?;
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).map_err(|d| format!("link: {d:?}"))?;
    sigil_harness::seam2::check_placed_regions(&linked, m)?;
    Ok(linked)
}

#[test]
fn enforced_seam2_tail_refuses_single_section_overflow() {
    let m = seam2_tables_map(HEAD);
    let mut secs = vec![data_section("sound_tables_z80", N + 1)];
    let d = place_sections_diags(&mut secs, &m);
    assert!(d.is_empty(), "place_sections emitted {d:?}");
    assert_eq!(secs[0].lma, HEAD);
    assert_eq!(secs[0].group.as_deref(), Some("sound_tables_z80"), "the region is recorded as the group");
    let err = seam2_tail(&secs, &m).expect_err("the seam-2 tail refuses N+1");
    assert_eq!(
        err,
        "section `sound_tables_z80` [0xB8000,0xB8401) overflows region `sound_tables_z80` (ends 0xB8400), over by 1 bytes"
    );
}

#[test]
fn enforced_seam2_tail_refuses_the_1116_byte_table_in_a_0x400_region() {
    // The 1,116 B aeon figure, through the seam-2 calls in the seam-2 order.
    let m = seam2_tables_map(HEAD);
    let mut secs = vec![data_section("sound_tables_z80", 1116)];
    let d = place_sections_diags(&mut secs, &m);
    assert_eq!(errors(&d), 0);
    let err = seam2_tail(&secs, &m).expect_err("the seam-2 tail refuses 1,116 B in a 0x400 region");
    assert!(err.ends_with("overflows region `sound_tables_z80` (ends 0xB8400), over by 92 bytes"), "{err}");
}

#[test]
fn enforced_emit_rom_refuses_single_section_overflow_by_one() {
    let m = seam2_tables_map(HEAD);
    let mut secs = vec![data_section("sound_tables_z80", N + 1)];
    assert!(place_sections_diags(&mut secs, &m).is_empty());
    let resolved = sigil_link::resolve_layout(&secs, &SymbolTable::new(), true).unwrap();
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).unwrap();
    let err = sigil_link::emit_rom(&linked, &m).expect_err("emit_rom refuses N+1");
    assert_eq!(
        err,
        "section `sound_tables_z80` [0xB8000,0xB8401) overflows region `sound_tables_z80` (ends 0xB8400), over by 1 bytes"
    );
}

#[test]
fn enforced_emit_rom_accepts_exact_fill() {
    let m = seam2_tables_map(HEAD);
    let mut secs = vec![data_section("sound_tables_z80", N)];
    assert!(place_sections_diags(&mut secs, &m).is_empty());
    let linked = seam2_tail(&secs, &m).expect("exact fill is accepted by the seam-2 tail");
    let rom = sigil_link::emit_rom(&linked, &m).expect("exact fill is accepted");
    assert_eq!(rom.len(), HEAD as usize + N);
}

#[test]
fn enforced_emit_rom_refuses_two_sections_summing_past_the_region() {
    // 0x300 + 0x200 = 0x500 in a 0x400 region: the second section starts INSIDE
    // the region and runs past its end.
    let m = seam2_tables_map(HEAD);
    let mut secs = vec![data_section("sound_tables_z80", 0x300), data_section("sound_tables_z80", 0x200)];
    let d = place_sections_diags(&mut secs, &m);
    assert!(d.is_empty(), "place_sections is silent on the sum: {d:?}");
    assert_eq!(secs[1].lma, HEAD + 0x300);
    let err = seam2_tail(&secs, &m).expect_err("the seam-2 tail refuses the sum");
    assert!(err.contains("overflows region `sound_tables_z80`"), "{err}");
    assert!(err.contains("over by 256 bytes"), "{err}");
}

#[test]
fn enforced_emit_rom_refuses_a_spill_that_starts_exactly_at_the_next_region() {
    // Region A is exactly filled by its first section; a second section named into
    // A is packed at A's end, which is B's base. The section is held to A, the
    // region it was placed into, so the 0x10 B spill into B is refused.
    let m = map(
        "fill = 0x00\n\
         [[region]]\nname = \"a\"\nlma_base = 0x1000\nsize = 0x400\nkind = \"rom\"\n\
         [[region]]\nname = \"b\"\nlma_base = 0x1400\nsize = 0x400\nkind = \"rom\"\n",
    );
    let mut secs = vec![data_section("a", 0x400), data_section("a", 0x10)];
    assert!(place_sections_diags(&mut secs, &m).is_empty());
    assert_eq!(secs[1].lma, 0x1400, "the spill lands at region b's base");
    let resolved = sigil_link::resolve_layout(&secs, &SymbolTable::new(), true).unwrap();
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).unwrap();
    let err = sigil_link::emit_rom(&linked, &m).expect_err("the spill into `b` is refused");
    assert_eq!(err, "section `a` [0x1400,0x1410) overflows region `a` (ends 0x1400), over by 16 bytes");
}

#[test]
fn enforced_emit_rom_refuses_overflow_of_a_region_nested_in_an_earlier_rom_region() {
    // A whole-cartridge ROM region listed FIRST (the shape of games/sonic4/map.toml,
    // whose first region is `rom` [0, 0x400000)) used to win the LMA lookup, so the
    // nested region was never the one checked. Now the named region is.
    let m = map(&format!(
        "fill = 0x00\n\
         [[region]]\nname = \"rom\"\nlma_base = 0x0\nsize = 0x400000\nkind = \"rom\"\n\
         [[region]]\nname = \"sound_tables_z80\"\nlma_base = 0x{HEAD:X}\nsize = 0x400\nkind = \"rom\"\n"
    ));
    let mut secs = vec![data_section("sound_tables_z80", 1116)];
    assert!(place_sections_diags(&mut secs, &m).is_empty());
    let resolved = sigil_link::resolve_layout(&secs, &SymbolTable::new(), true).unwrap();
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).unwrap();
    let err = sigil_link::emit_rom(&linked, &m).expect_err("checked against `sound_tables_z80`, not `rom`");
    assert_eq!(
        err,
        "section `sound_tables_z80` [0xB8000,0xB845C) overflows region `sound_tables_z80` (ends 0xB8400), over by 92 bytes"
    );
}

/// Reviewer B's case: two sections, each inside the Z80 bank window on its own
/// start, whose sum runs past the window top. The native chain names no region
/// (`declchain{n}`), so the bank window is what holds them.
#[test]
fn z80_bank_two_sections_summing_past_the_window_are_refused() {
    let m = map(
        "fill = 0x00\n\
         [[region]]\nname = \"rom\"\nlma_base = 0x0\nsize = 0x400000\nkind = \"rom\"\n\
         [[region]]\nname = \"z80_sound_bank\"\nlma_base = 0xB8000\nsize = 0x8000\nkind = \"z80_bank\"\nvma_base = 0x8000\n",
    );
    let sec = |name: &str, lma: u32, len: usize| LinkedSection {
        name: name.into(),
        lma,
        bytes: vec![0x5A; len],
        region: Some("declchain7".into()),
    };
    let image = LinkedImage { sections: vec![sec("tables", 0xB8000, 0x5000), sec("sfx", 0xBD000, 0x4000)] };
    let err = sigil_link::emit_rom(&image, &m).expect_err("the second section crosses $C0000");
    assert_eq!(
        err,
        "section `sfx` [0xBD000,0xC1000) overflows region `z80_sound_bank` (ends 0xC0000), over by 4096 bytes"
    );
    let fits = LinkedImage { sections: vec![sec("tables", 0xB8000, 0x5000), sec("sfx", 0xBD000, 0x3000)] };
    sigil_link::emit_rom(&fits, &m).expect("the pair that ends exactly at $C0000 fits");
}

fn place_sections_diags(secs: &mut [Section], m: &MemoryMap) -> Vec<sigil_span::Diagnostic> {
    sigil_frontend_emp::resolve::place_sections(secs, m)
}

/// The real table, through the real seam-2 emitter, in the aeon tree named by
/// `SEAM2_REGION_AEON`. Ignored by default (it needs an aeon tree); run with
/// `--ignored`. It reads only: lowering + linking one self-contained module.
/// Panics when the variable is unset, so an unconfigured run cannot read as a pass.
/// The table now places into the DERIVED room `bank_room(head_lma)` (the rest of
/// its `$8000` window), which must hold the measured bytes.
#[test]
#[ignore = "needs SEAM2_REGION_AEON=<aeon tree>; run explicitly with --ignored"]
fn measure_real_sound_tables_z80_through_seam2() {
    let aeon = std::env::var("SEAM2_REGION_AEON").expect("set SEAM2_REGION_AEON to an aeon tree");
    let aeon = std::path::Path::new(&aeon);
    let bytes = sigil_harness::seam2::emit_sound_tables_z80(aeon).expect("the seam-2 emitter returns Ok");
    let head = sigil_harness::seam2::sound_layout(aeon).expect("sound layout").sound_tables_z80_lma;
    let room = sigil_harness::seam2::bank_room(head);
    eprintln!(
        "SEAM2_REGION_MEASURE aeon={} sound_tables_z80_len={} head_lma={head:#X} derived_room={room}",
        aeon.display(),
        bytes.len()
    );
    assert!(bytes.len() as u32 <= room, "the derived room {room} holds the {} B table", bytes.len());
}
