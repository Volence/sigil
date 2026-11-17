//! MEASUREMENT (queue row SEAM2-TABLE-REGION-UNENFORCED): what the region-map
//! placement path does with a section larger than its region's `size`.
//!
//! These tests assert CURRENT behaviour, including behaviour that is a gap. They
//! are a record of what the code does today, not a statement that it is right.
//! Each test name says which it is. See
//! `docs/superpowers/notes/2026-09-28-seam2-region-unenforced.md`.
//!
//! Findings pinned here:
//! * `place_sections` never compares a section's span against its region's
//!   `size`; overflow is deferred to `emit_rom` (`MemoryMap::validate_section`).
//! * The seam-2 per-artifact tail (`place_sections` + `resolve_layout` + `link`,
//!   no `emit_rom`) therefore never checks a region size at all.
//! * `emit_rom` does refuse a single section that overruns its region, and
//!   accepts one that exactly fills it.
//! * `emit_rom` checks each section against the FIRST ROM region that contains
//!   the section's LMA, not the region `place_sections` named it into. Two shapes
//!   follow in which even `emit_rom` accepts a by-name overflow: a region spill
//!   that starts exactly at the next region's base, and a small region nested in
//!   a larger ROM region listed before it.

use sigil_ir::map::MemoryMap;
use sigil_ir::{DataFragment, Fragment, Section, SectionPlacement, SymbolTable};
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

/// The seam-2 synthetic map shape, verbatim from `seam2.rs`'s
/// `sound_tables_z80_linked_from_src` (region `sound_tables_z80`, size 0x400).
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

/// The seam-2 tail exactly as the emitter runs it after placement.
fn seam2_tail(sections: &[Section]) -> Result<sigil_link::LinkedImage, String> {
    let resolved = sigil_link::resolve_layout(sections, &SymbolTable::new(), true)
        .map_err(|d| format!("resolve_layout: {d:?}"))?;
    sigil_link::link(&resolved, &SymbolTable::new()).map_err(|d| format!("link: {d:?}"))
}

#[test]
fn gap_place_sections_is_silent_on_single_section_overflow() {
    let m = seam2_tables_map(HEAD);
    let mut secs = vec![data_section("sound_tables_z80", N + 1)];
    let d = place_sections_diags(&mut secs, &m);
    assert!(d.is_empty(), "place_sections emitted {d:?}");
    assert_eq!(secs[0].lma, HEAD);
    assert_eq!(secs[0].reserved_span, (N + 1) as u32);
}

#[test]
fn gap_seam2_tail_links_an_overflowing_section_with_no_diagnostic() {
    // The 1,116 B aeon figure, through the seam-2 calls in the seam-2 order.
    let m = seam2_tables_map(HEAD);
    let mut secs = vec![data_section("sound_tables_z80", 1116)];
    let d = place_sections_diags(&mut secs, &m);
    assert_eq!(errors(&d), 0);
    assert!(d.is_empty(), "no warning either: {d:?}");
    let linked = seam2_tail(&secs).expect("the seam-2 tail accepts 1,116 B in a 0x400 region");
    assert_eq!(linked.section("sound_tables_z80").unwrap().bytes.len(), 1116);
}

#[test]
fn enforced_emit_rom_refuses_single_section_overflow_by_one() {
    let m = seam2_tables_map(HEAD);
    let mut secs = vec![data_section("sound_tables_z80", N + 1)];
    assert!(place_sections_diags(&mut secs, &m).is_empty());
    let linked = seam2_tail(&secs).unwrap();
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
    let linked = seam2_tail(&secs).unwrap();
    let rom = sigil_link::emit_rom(&linked, &m).expect("exact fill is accepted");
    assert_eq!(rom.len(), HEAD as usize + N);
}

#[test]
fn enforced_emit_rom_refuses_two_sections_summing_past_the_region() {
    // 0x300 + 0x200 = 0x500 in a 0x400 region: the second section starts INSIDE
    // the region and runs past its end, so `validate_section` sees it.
    let m = seam2_tables_map(HEAD);
    let mut secs = vec![data_section("sound_tables_z80", 0x300), data_section("sound_tables_z80", 0x200)];
    let d = place_sections_diags(&mut secs, &m);
    assert!(d.is_empty(), "place_sections is silent on the sum too: {d:?}");
    assert_eq!(secs[1].lma, HEAD + 0x300);
    let linked = seam2_tail(&secs).unwrap();
    let err = sigil_link::emit_rom(&linked, &m).expect_err("emit_rom refuses the sum");
    assert!(err.contains("overflows region `sound_tables_z80`"), "{err}");
    assert!(err.contains("over by 256 bytes"), "{err}");
}

#[test]
fn gap_emit_rom_accepts_a_spill_that_starts_exactly_at_the_next_region() {
    // Region A is exactly filled by its first section; a second section named
    // into A is packed at A's end, which is B's base. `validate_section` finds
    // the region by LMA, so it checks the spill against B and accepts it. B has
    // no section of its own here, so `flatten_checked` sees no overlap either.
    let m = map(
        "fill = 0x00\n\
         [[region]]\nname = \"a\"\nlma_base = 0x1000\nsize = 0x400\nkind = \"rom\"\n\
         [[region]]\nname = \"b\"\nlma_base = 0x1400\nsize = 0x400\nkind = \"rom\"\n",
    );
    let mut secs = vec![data_section("a", 0x400), data_section("a", 0x10)];
    assert!(place_sections_diags(&mut secs, &m).is_empty());
    assert_eq!(secs[1].lma, 0x1400, "the spill lands at region b's base");
    let linked = seam2_tail(&secs).unwrap();
    let rom = sigil_link::emit_rom(&linked, &m).expect("today: accepted, 0x10 B of `a` sit in `b`");
    assert_eq!(rom.len(), 0x1410);
}

#[test]
fn gap_emit_rom_accepts_overflow_of_a_region_nested_in_an_earlier_rom_region() {
    // A whole-cartridge ROM region listed FIRST (the shape of games/sonic4/map.toml,
    // whose first region is `rom` [0, 0x400000)) wins `region_for`, so a by-name
    // region nested inside it is never the one checked.
    let m = map(&format!(
        "fill = 0x00\n\
         [[region]]\nname = \"rom\"\nlma_base = 0x0\nsize = 0x400000\nkind = \"rom\"\n\
         [[region]]\nname = \"sound_tables_z80\"\nlma_base = 0x{HEAD:X}\nsize = 0x400\nkind = \"rom\"\n"
    ));
    let mut secs = vec![data_section("sound_tables_z80", 1116)];
    assert!(place_sections_diags(&mut secs, &m).is_empty());
    let linked = seam2_tail(&secs).unwrap();
    let rom = sigil_link::emit_rom(&linked, &m).expect("today: accepted, checked against `rom` not `sound_tables_z80`");
    assert_eq!(rom.len(), HEAD as usize + 1116);
}

fn place_sections_diags(secs: &mut [Section], m: &MemoryMap) -> Vec<sigil_span::Diagnostic> {
    sigil_frontend_emp::resolve::place_sections(secs, m)
}

/// The real table, through the real seam-2 emitter, in the aeon tree named by
/// `SEAM2_REGION_AEON`. Ignored by default (it needs an aeon tree); run with
/// `--ignored`. It reads only: lowering + linking one self-contained module.
/// Panics when the variable is unset, so an unconfigured run cannot read as a pass.
#[test]
#[ignore = "needs SEAM2_REGION_AEON=<aeon tree>; run explicitly with --ignored"]
fn measure_real_sound_tables_z80_through_seam2() {
    let aeon = std::env::var("SEAM2_REGION_AEON").expect("set SEAM2_REGION_AEON to an aeon tree");
    let bytes = sigil_harness::seam2::emit_sound_tables_z80(std::path::Path::new(&aeon))
        .expect("the seam-2 emitter returns Ok");
    eprintln!("SEAM2_REGION_MEASURE aeon={aeon} sound_tables_z80_len={} region_size={N}", bytes.len());
}
