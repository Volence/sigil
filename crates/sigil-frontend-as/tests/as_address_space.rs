//! Which address space each section the AS front end produces is in.
//!
//! A 68000 program that assembles a Z80 sound driver at the Z80's own addresses
//! (`save` / `!org 0` / `CPU Z80` / ... / `restore`, the shape of Sonic 1, Sonic 2
//! and Sonic 3 & Knuckles) produces a section whose `lma` is a Z80 address, not a
//! ROM offset. These tests pin the rule that tells that section apart from every
//! section that IS in the image (`eval::assign_address_spaces`):
//!
//! - the image's CPU is the CPU of the program's first section with content;
//! - only an `org` can change the space (one that leaves the open section, or a
//!   seek back that the section closes behind); after it, the first section with
//!   content decides: the image's CPU is the image, the CPU of the second space
//!   the counter is already in stays there, a `phase` open makes the `org` an
//!   image placement, and anything else enters a new second space at that `org`;
//! - a section opened on a continued counter is in the counter's space.
//!
//! Every source here is written so the rule, not an accident of section
//! numbering, decides the answer: each test first asserts the section shape it
//! depends on, so a changed front end that no longer produces that shape fails
//! loudly instead of passing over nothing.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::{AddressSpace, Cpu, Module, Section};

fn asm(src: &str) -> Module {
    assemble(src, &Options::default()).unwrap_or_else(|d| panic!("must assemble: {d:?}\n{src}"))
}

/// The 1-based line of `src` a byte offset falls on.
fn line_of(src: &str, offset: u32) -> usize {
    src[..offset as usize].matches('\n').count() + 1
}

/// The 1-based line of the first line of `src` containing `needle`.
fn line_containing(src: &str, needle: &str) -> usize {
    src.lines()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("`{needle}` is not in the source"))
        + 1
}

/// The sections that hold content, in order: the only ones the rule decides.
fn with_content(m: &Module) -> Vec<&Section> {
    m.sections.iter().filter(|s| !s.fragments.is_empty()).collect()
}

/// The `org` line a foreign section's space was entered at, or a panic naming
/// what it was instead.
fn entered_line(src: &str, s: &Section) -> (Cpu, usize) {
    match s.space {
        AddressSpace::Foreign { cpu, entered_at } => (cpu, line_of(src, entered_at.start)),
        AddressSpace::Image => panic!("section `{}` (lma {:#X}) is in the image", s.name, s.lma),
    }
}

/// Sonic 1's shape, with a section break before the driver so the open section
/// begins above 0 and `!org 0` leaves it: the driver is a second Z80 space
/// entered at `!org 0`, and the code on either side of it is the image.
#[test]
fn a_driver_org_d_to_z80_zero_is_a_second_space_entered_at_its_org() {
    let src = "\tcpu 68000\n\
               \tdc.l 0, 0\n\
               \torg $100\n\
               \tdc.w $4E71\n\
               DACDriver:\n\
               \tsave\n\
               \t!org 0\n\
               \tcpu z80\n\
               \tdi\n\
               \tld a,1\n\
               \trestore\n\
               \tpadding off\n\
               \t!org DACDriver+$10\n\
               \tdc.w $4E71\n";
    let m = asm(src);
    let secs = with_content(&m);
    assert_eq!(secs.len(), 4, "vectors, $100 code, driver, code after: {:#?}", m.sections);
    assert_eq!((secs[2].cpu, secs[2].lma), (Cpu::Z80, 0), "the driver section is Z80 at 0");
    assert_eq!(entered_line(src, secs[2]), (Cpu::Z80, line_containing(src, "!org 0")));
    for i in [0, 1, 3] {
        assert_eq!(secs[i].space, AddressSpace::Image, "section {i} `{}` is image code", secs[i].name);
    }
}

/// The same driver when the section open at `!org 0` itself begins at 0: the org
/// is an in-section seek, and the `cpu z80` line closes the section behind it.
/// The counter was re-based all the same, so the driver is the same second
/// space, entered at the same line. It must also be `Pinned` at its Z80 origin:
/// left `Chained`, the linker would pack it straight after the vector table.
#[test]
fn a_seek_back_that_its_section_closes_behind_enters_the_space_too() {
    let src = "\tcpu 68000\n\
               \tdc.l 0, 0\n\
               DACDriver:\n\
               \tsave\n\
               \t!org 0\n\
               \tcpu z80\n\
               \tdi\n\
               \tld a,1\n\
               \trestore\n\
               \tpadding off\n\
               \t!org DACDriver+$10\n\
               \tdc.w $4E71\n";
    let m = asm(src);
    let secs = with_content(&m);
    assert_eq!(secs.len(), 3, "vectors, driver, code after: {:#?}", m.sections);
    assert_eq!((secs[1].cpu, secs[1].lma), (Cpu::Z80, 0), "the driver opens on the rewound counter");
    assert_eq!(entered_line(src, secs[1]), (Cpu::Z80, line_containing(src, "!org 0")));
    assert_eq!(secs[1].placement, sigil_ir::SectionPlacement::Pinned, "pinned at its Z80 origin");
    assert_eq!(secs[0].space, AddressSpace::Image);
    assert_eq!(secs[2].space, AddressSpace::Image);
}

/// The address a section binds label `name` at, if the label is in it.
fn label_address(s: &Section, name: &str) -> Option<u32> {
    s.labels.iter().find(|l| l.name == name).map(|l| s.vma_origin() + l.offset)
}

/// The same seek back and close in a program with no second CPU: probe
/// `c01_cpu_same` of `docs/superpowers/notes/2026-09-12-as-backward-seek-split/`.
/// Four marker words at $8, `org Start+2`, one overwrite, and a `cpu` line that
/// closes the section with its cursor at $C and its bytes running to $10. asl
/// binds `L_next` at $C and `L_after` at $12 (listing lines
/// `10/ C : 1234 L_next: dc.w $1234` and `13/ 12 : 5678 L_after: dc.w $5678`,
/// clean exit). The section after the close is in the image, and it is `Pinned`
/// at $C, the address its labels are bound at: left `Chained`, the linker packs
/// its bytes at $10, the end of the section before it. The section after that
/// continues the counter, so it stays `Chained`.
#[test]
fn a_seek_back_that_an_image_section_closes_behind_pins_the_next_section_at_its_labels() {
    let src = "\tcpu 68000\n\
               \torg 0\n\
               \tdc.l L_next\n\
               \tdc.l L_after\n\
               Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\
               \torg Start+2\n\
               \tdc.w $EEEE\n\
               \tcpu 68000\n\
               L_next:\tdc.w $1234\n\
               \tdc.l *\n\
               \tcpu 68000\n\
               L_after:\tdc.w $5678\n";
    let m = asm(src);
    let secs = with_content(&m);
    assert_eq!(secs.len(), 3, "the seek's section, the code at L_next, the code at L_after: {:#?}", m.sections);
    let (next, after) = (secs[1], secs[2]);
    assert_eq!(label_address(next, "L_next"), Some(0xC), "L_next is bound where asl binds it");
    assert_eq!(next.lma, 0xC, "the section's bytes load at its labels");
    assert_eq!(next.placement, sigil_ir::SectionPlacement::Pinned, "and the linker may not move them");
    assert_eq!(label_address(after, "L_after"), Some(0x12));
    assert_eq!((after.lma, after.placement), (0x12, sigil_ir::SectionPlacement::Chained));
    for s in &secs {
        assert_eq!(s.space, AddressSpace::Image, "section `{}` is image bytes", s.name);
    }
}

/// A second space's code, as the link-time placement pass places it, when
/// sections with no content open between the `org` and the code: a label between
/// the `org` and the `cpu` line, a second label under the new CPU, a label before
/// a `phase`. The section the builder opens at the `org` is `Pinned` at the
/// counter the `org` set. A section with no fragments spans nothing, so every
/// section after it up to the code is placed at that same counter, and so is the
/// code: its bytes load where its labels are bound. Each case names the label the
/// reference assembler binds at that counter, `asl` md5
/// 61e672562465725a8c102288a7da9098 through `asl_ref.sh`'s `asl_run` (exit 0,
/// 0 errors), with the listing line quoted. The sources are the probes of
/// `docs/superpowers/notes/2026-09-12-second-space-pin/`. The shipped command's
/// image for the placed shapes is `as_second_address_space`'s
/// `a_driver_behind_sections_with_no_content_is_placed_where_p2bin_places_it`.
#[test]
fn a_second_space_behind_sections_with_no_content_is_placed_at_its_org() {
    let cases: [(&str, &str, &str, u32, &str); 5] = [
        (
            // p01: `9/ 0 : DriverStart:`; the `!org 0` leaves the section open at $100.
            "one label between the org and the cpu line",
            "\tcpu 68000\nSize1 equ $10\nSize2 equ $10\n\tdc.l 0, 0\n\torg $100\n\tdc.w $4E71\n\
             \tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tdi\n\tld a,1\n\
             \trestore\n\tpadding off\n\t!org $110\n\tdc.w $4E71\n",
            "DriverStart",
            0,
            "!org 0",
        ),
        (
            // p03: `9/ 0 : DriverStart:` and `11/ 0 : Inner:`.
            "a second label under the new cpu",
            "\tcpu 68000\nSize1 equ $10\nSize2 equ $10\n\tdc.l 0, 0\n\torg $100\n\tdc.w $4E71\n\
             \tsave\n\t!org 0\nDriverStart:\n\tcpu z80\nInner:\n\tcpu z80\n\tdi\n\tld a,1\n\
             \trestore\n\tpadding off\n\t!org $110\n\tdc.w $4E71\n",
            "DriverStart",
            0,
            "!org 0",
        ),
        (
            // p16: `9/ 0 : DriverStart:`; a `cpu` line closed the section, so the
            // `!org 0` finds none open.
            "the org finds no section open",
            "\tcpu 68000\nSize1 equ $10\n\tdc.l 0, 0\n\torg $100\n\tdc.w $4E71\n\tcpu 68000\n\
             \tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tdi\n\tld a,1\n\
             \trestore\n\tpadding off\n\t!org $110\n\tdc.w $4E71\n",
            "DriverStart",
            0,
            "!org 0",
        ),
        (
            // p04: `12/ 40 : L40:`, then `14/ 1000 : 05 db 5` under the phase. The
            // `org 40h` stays in the driver's space, so the space is still the one
            // entered at `!org 0`, and the phased byte loads at $40.
            "an org inside the driver, a label, then a phase",
            "\tcpu 68000\nSize1 equ $10\nSize2 equ $10\n\tdc.l 0, 0\n\torg $100\n\tdc.w $4E71\n\
             \tsave\n\t!org 0\n\tcpu z80\n\tdi\n\torg 40h\nL40:\n\tphase 1000h\n\tdb 5\n\tdephase\n\
             \trestore\n\tpadding off\n\t!org $110\n\tdc.w $4E71\n",
            "L40",
            0x40,
            "!org 0",
        ),
        (
            // p09: `5/ 100 : Mark:` and `7/ 100 : 1234 dc.w $1234`. A Z80 program
            // entering a 68000 second space.
            "a Z80 image entering a 68000 space",
            "\tcpu z80\n\torg 0\n\tdb 1\n\torg 100h\nMark:\n\tcpu 68000\n\tdc.w $1234\n",
            "Mark",
            0x100,
            "org 100h",
        ),
    ];
    for (shape, src, label, at, entered) in cases {
        let m = asm(src);
        let at_label = m
            .sections
            .iter()
            .position(|s| label_address(s, label).is_some())
            .unwrap_or_else(|| panic!("{shape}: `{label}` is bound in no section: {:#?}", m.sections));
        assert_eq!(label_address(&m.sections[at_label], label), Some(at), "{shape}: `{label}` is bound where asl binds it");
        let code = (at_label..m.sections.len())
            .find(|&i| !m.sections[i].fragments.is_empty())
            .unwrap_or_else(|| panic!("{shape}: no code after `{label}`: {:#?}", m.sections));
        assert!(
            code > at_label,
            "{shape}: `{label}` opens a section with no content ahead of the code (the shape this test depends on): {:#?}",
            m.sections
        );
        let (_, line) = entered_line(src, &m.sections[code]);
        assert_eq!(line, line_containing(src, entered), "{shape}: the code is a second space entered at `{entered}`");
        let placed = sigil_link::resolve_layout_measuring(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .unwrap_or_else(|d| panic!("{shape}: placement: {d:?}"));
        assert_eq!(
            placed[code].lma, at,
            "{shape}: the code is placed at the org's counter, where `{label}` is bound; `{}` [{:#X}] was placed at {:#X}",
            placed[code].name, m.sections[code].lma, placed[code].lma
        );
    }
}

/// Sonic 1's `SetupValues_Z80` shape: Z80 code phased to 0 with no `org`. The
/// counter runs on through it, so it is image bytes, and so is everything after.
#[test]
fn a_phased_z80_block_with_no_org_is_in_the_image() {
    let src = "\tcpu 68000\n\
               \tdc.l 0\n\
               \tsave\n\
               \tcpu z80\n\
               \tphase 0\n\
               \tdi\n\
               \tjp 0\n\
               \tdephase\n\
               \trestore\n\
               \tpadding off\n\
               \tdc.w $4E71\n";
    let m = asm(src);
    let secs = with_content(&m);
    assert!(
        secs.iter().any(|s| s.cpu == Cpu::Z80 && s.lma == 4 && s.vma_base == Some(0)),
        "the Z80 block is loaded at 4 and runs at 0: {:#?}",
        m.sections
    );
    for s in &secs {
        assert_eq!(s.space, AddressSpace::Image, "section `{}` is image bytes", s.name);
    }
}

/// Z80 bytes inline in a 68000 image with neither an `org` nor a `phase`: the
/// `cpu z80` line breaks the section but the counter runs on, so the Z80 section
/// is image bytes. Only an `org` can move the counter out of the image.
#[test]
fn an_inline_z80_block_with_no_org_and_no_phase_is_in_the_image() {
    let src = "\tcpu 68000\n\
               \tdc.l 0\n\
               \tsave\n\
               \tcpu z80\n\
               \tdb 1,2\n\
               \trestore\n\
               \tpadding off\n\
               \tdc.w $4E71\n";
    let m = asm(src);
    let secs = with_content(&m);
    assert!(
        secs.iter().any(|s| s.cpu == Cpu::Z80 && s.lma == 4 && s.vma_base == Some(4)),
        "the Z80 bytes continue the counter at 4, unphased: {:#?}",
        m.sections
    );
    for s in &secs {
        assert_eq!(s.space, AddressSpace::Image, "section `{}` is image bytes", s.name);
    }
}

/// An `org` that places Z80 code and a `phase` that gives it its run address:
/// the load/run split the image models, so the image.
#[test]
fn an_org_with_a_phase_open_is_an_image_placement() {
    let src = "\tcpu 68000\n\
               \tdc.l 0, 0\n\
               \torg $200\n\
               \tdc.w 1\n\
               \tsave\n\
               \t!org $100\n\
               \tcpu z80\n\
               \tphase 0\n\
               \tdb 1\n\
               \tdephase\n\
               \trestore\n\
               \t!org $300\n\
               \tdc.w 2\n";
    let m = asm(src);
    let secs = with_content(&m);
    let z80 = secs.iter().find(|s| s.cpu == Cpu::Z80).expect("a Z80 section");
    assert_eq!((z80.lma, z80.vma_base), (0x100, Some(0)), "loaded at $100, run at 0");
    for s in &secs {
        assert_eq!(s.space, AddressSpace::Image, "section `{}` is image bytes", s.name);
    }
}

/// A Z80 program is a Z80 image: its `org 0` is the image's origin, and a later
/// `org` under the same CPU is an image placement.
#[test]
fn a_z80_program_at_org_0_is_the_image() {
    let src = "\tcpu z80\n\
               \torg 0\n\
               \tdi\n\
               \tld a,1\n\
               \torg 100h\n\
               \tdb 2\n";
    let m = asm(src);
    let secs = with_content(&m);
    assert_eq!(secs.len(), 2, "{:#?}", m.sections);
    for s in &secs {
        assert_eq!((s.cpu, s.space), (Cpu::Z80, AddressSpace::Image), "section `{}`", s.name);
    }
}

/// A label between the `org` and the `cpu` line opens a section under the old
/// CPU with nothing in it. It must not decide: the code after it is still the
/// second space, entered at the `org`.
#[test]
fn a_label_between_the_org_and_the_cpu_line_does_not_decide() {
    let src = "\tcpu 68000\n\
               \tdc.l 0, 0\n\
               \torg $100\n\
               \tdc.w 1\n\
               \tsave\n\
               \t!org 0\n\
               DriverStart:\n\
               \tcpu z80\n\
               \tdi\n\
               \trestore\n\
               \t!org $200\n\
               \tdc.w 2\n";
    let m = asm(src);
    assert!(
        m.sections.iter().any(|s| s.fragments.is_empty() && s.cpu == Cpu::M68000 && s.lma == 0),
        "the label opens an empty 68000 section at 0 (the shape this test depends on): {:#?}",
        m.sections
    );
    let secs = with_content(&m);
    let z80 = secs.iter().find(|s| s.cpu == Cpu::Z80).expect("a Z80 section");
    assert_eq!(entered_line(src, z80), (Cpu::Z80, line_containing(src, "!org 0")));
}

/// Sonic 2's driver has an `org 38h` inside it. Under the same CPU, in the space
/// the counter is already in, that lays the driver out: one space, not two.
#[test]
fn an_org_inside_a_driver_stays_in_the_driver_s_space() {
    let src = "\tcpu 68000\n\
               \tdc.l 0, 0\n\
               \torg $100\n\
               \tdc.w 1\n\
               \tsave\n\
               \t!org 0\n\
               \tcpu z80\n\
               \tdi\n\
               \torg 38h\n\
               \tei\n\
               \trestore\n\
               \t!org $200\n\
               \tdc.w 2\n";
    let m = asm(src);
    let z80: Vec<&Section> = with_content(&m).into_iter().filter(|s| s.cpu == Cpu::Z80).collect();
    assert_eq!(z80.len(), 2, "the `org 38h` splits the driver in two sections: {:#?}", m.sections);
    assert_eq!(z80[1].lma, 0x38);
    assert_eq!(z80[0].space, z80[1].space, "one driver, one space");
    assert_eq!(entered_line(src, z80[1]), (Cpu::Z80, line_containing(src, "!org 0")));
}

/// Sonic 3 & Knuckles has two Z80 blobs, at 0 and at `1300h`, with a return to
/// the cartridge between them. Each entry from the image is a space of its own.
#[test]
fn two_entries_from_the_image_are_two_spaces() {
    let src = "\tcpu 68000\n\
               \tdc.l 0, 0\n\
               \torg $100\n\
               \tdc.w 1\n\
               \tsave\n\
               \t!org 0\n\
               \tcpu z80\n\
               \tdi\n\
               \trestore\n\
               \t!org $200\n\
               \tdc.w 2\n\
               \tsave\n\
               \tcpu z80\n\
               \t!org 1300h\n\
               \tdb 7\n\
               \trestore\n\
               \t!org $300\n\
               \tdc.w 3\n";
    let m = asm(src);
    let secs = with_content(&m);
    let z80: Vec<&Section> = secs.iter().copied().filter(|s| s.cpu == Cpu::Z80).collect();
    assert_eq!(z80.len(), 2, "{:#?}", m.sections);
    assert_eq!(entered_line(src, z80[0]), (Cpu::Z80, line_containing(src, "!org 0")));
    assert_eq!(entered_line(src, z80[1]), (Cpu::Z80, line_containing(src, "!org 1300h")));
    assert_ne!(z80[0].space, z80[1].space);
    let image: Vec<&&Section> = secs.iter().filter(|s| s.cpu == Cpu::M68000).collect();
    assert_eq!(image.len(), 4, "{:#?}", m.sections);
    for s in image {
        assert_eq!(s.space, AddressSpace::Image, "section `{}` (lma {:#X})", s.name, s.lma);
    }
}

/// The image's CPU is decided by the first section WITH CONTENT. A label before
/// the `cpu` line opens an empty section under the provisional processor, and a
/// 68000 program must not become a Z80 image because of it.
#[test]
fn the_image_cpu_is_the_first_section_with_content() {
    let src = "Start:\n\
               \tcpu 68000\n\
               \tdc.w 1\n\
               \torg $100\n\
               \tdc.w 2\n";
    let m = asm(src);
    assert!(
        m.sections.first().is_some_and(|s| s.fragments.is_empty() && s.cpu == Cpu::Z80),
        "the label opens an empty section under the provisional Z80 (the shape this test depends on): {:#?}",
        m.sections
    );
    let secs = with_content(&m);
    assert_eq!(secs.len(), 2, "{:#?}", m.sections);
    for s in secs {
        assert_eq!((s.cpu, s.space), (Cpu::M68000, AddressSpace::Image), "section `{}`", s.name);
    }
}
