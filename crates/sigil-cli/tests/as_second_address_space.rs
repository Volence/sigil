//! The Z80 sound driver of every AS-toolchain Sonic disassembly is assembled at
//! the Z80's own addresses inside the 68000 program (`save` / `!org 0` /
//! `CPU Z80` / ... / `restore`). Its bytes have no place in the ROM until
//! something declares one. The reference toolchain's own answer, when its
//! post-processor flag is withheld, is the driver written over the vector table
//! with exit 0.
//!
//! These tests run the shipped command, because the refusal is a property of the
//! whole route: the front end tags the section, the linker refuses it, and the
//! renderer locates it. Each half can pass alone while the command prints the
//! wrong thing or nothing. What each test catches:
//!
//! - the driver, in both of the ways its `!org 0` can meet the open section:
//!   exit nonzero, exactly ONE error, at the driver's own `!org 0` line, naming
//!   the Z80 and origin 0, never an overlap, and no ROM written. A discriminator
//!   left under a global overlap scan prints the overlap instead. A per-space
//!   scan with no refusal exits 0, and so does a refusal that misses the seek
//!   entry; that is the silent wrong ROM.
//! - a second space on free image ground: refused too, so a refusal that fires
//!   only when the driver happens to collide fails here.
//! - collisions inside one space, the image (68000, and a Z80 program's) and a
//!   second space: still refused, as overlaps.
//! - an `org` with a `phase` open, Sonic 1's phased `SetupValues_Z80` block, and
//!   a Z80 program at `org 0`: still assemble, to the bytes their sources and the
//!   reference assembler give.
//! - a driver whose `!org` opens sections with no content before its code, placed
//!   by `-z`: the reference toolchain's image, so a driver placed anywhere but its
//!   origin is refused by `-z` or lands its bytes somewhere else, and fails here.

use std::path::Path;
use std::process::{Command, Output};

fn run(dir: &Path, root: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(dir.join(root))
        .args(extra)
        .output()
        .expect("spawn sigil")
}

fn error_lines(stderr: &str) -> Vec<&str> {
    stderr.lines().filter(|l| l.contains(": error: ")).collect()
}

/// Assemble `src` as `root.asm` in a fresh directory and return the image, or
/// panic with everything sigil printed.
fn assemble_ok(src: &str) -> Vec<u8> {
    assemble_ok_with(src, &[])
}

/// [`assemble_ok`] with further arguments after `-o`, such as a `-z`.
fn assemble_ok_with(src: &str, extra: &[&str]) -> Vec<u8> {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("root.asm"), src).expect("write root.asm");
    let out_path = dir.path().join("out.bin");
    let mut args = vec!["-o", out_path.to_str().expect("utf-8 path")];
    args.extend_from_slice(extra);
    let out = run(dir.path(), "root.asm", &args);
    assert!(
        out.status.success(),
        "must assemble.\nstderr:\n{}\nsource:\n{src}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read(&out_path).expect("the image was written")
}

/// Assemble `src` as `root.asm`, require it refused with exactly one error, and
/// return that error line. Also requires that no image was written.
fn assemble_refused(src: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("root.asm"), src).expect("write root.asm");
    let out_path = dir.path().join("out.bin");
    let out = run(dir.path(), "root.asm", &["-o", out_path.to_str().expect("utf-8 path")]);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!out.status.success(), "must be refused.\nstderr:\n{stderr}\nsource:\n{src}");
    assert!(!out_path.exists(), "a refused program must write no image.\nstderr:\n{stderr}");
    let errors = error_lines(&stderr);
    assert_eq!(errors.len(), 1, "exactly one error.\nstderr:\n{stderr}");
    errors[0].to_string()
}

/// The driver file, in the shape of `s1disasm/sound/z80.asm`: its `!org 0` is on
/// line 3, and it is three bytes of Z80 code (`di` = F3, `ld a,1` = 3E 01).
const DRIVER: &str = "; a Z80 driver in the shape of s1disasm's sound/z80.asm\n\
                      \tsave\n\
                      \t!org\t0\t\t; z80 Align, handled by the build process\n\
                      \tCPU Z80\n\
                      \tlisting purecode\n\
                      \tdi\n\
                      \tld\ta,1\n\
                      \trestore\n\
                      \tpadding off\n\
                      \t!org\t(DACDriver+Size_of_DAC_driver_guess)\n";

/// Two roots that include the driver the way `s1.sounddriver.asm` does
/// (`DACDriver: include "sound/z80.asm"`). In the first, the open section at the
/// driver's `!org 0` begins at 0, so the org is a seek back that the `CPU Z80`
/// line then closes behind. In the second, a forward `org` has opened a section
/// at $100 first, so the `!org 0` leaves it, which is Sonic 1's own path.
const ROOTS: [(&str, &str); 2] = [
    (
        "the section open at the org begins at 0",
        "\tcpu 68000\n\
         Size_of_DAC_driver_guess equ $10\n\
         \tdc.l 0, 0\n\
         DACDriver:\tinclude \"z80.asm\"\n\
         \tdc.w $4E71\n",
    ),
    (
        "a forward org opened the section at the org",
        "\tcpu 68000\n\
         Size_of_DAC_driver_guess equ $10\n\
         \tdc.l 0, 0\n\
         \torg $100\n\
         \tdc.w $4E71\n\
         DACDriver:\tinclude \"z80.asm\"\n\
         \tdc.w $4E71\n",
    ),
];

#[test]
fn the_driver_is_refused_at_its_own_org_line_and_never_as_an_overlap() {
    for (shape, root) in ROOTS {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("z80.asm"), DRIVER).expect("write z80.asm");
        std::fs::write(dir.path().join("root.asm"), root).expect("write root.asm");
        let out_path = dir.path().join("out.bin");
        let out = run(dir.path(), "root.asm", &["-o", out_path.to_str().expect("utf-8 path")]);
        let stderr = String::from_utf8_lossy(&out.stderr);

        assert!(!out.status.success(), "{shape}: the driver must be refused.\nstderr:\n{stderr}");
        assert!(!out_path.exists(), "{shape}: no image may be written.\nstderr:\n{stderr}");
        assert!(!stderr.contains("overlap"), "{shape}: not an overlap.\nstderr:\n{stderr}");
        let errors = error_lines(&stderr);
        assert_eq!(errors.len(), 1, "{shape}: exactly one error.\nstderr:\n{stderr}");
        let at_org = format!("{}(3):", dir.path().join("z80.asm").display());
        assert!(
            errors[0].starts_with(&at_org),
            "{shape}: located at the driver's `!org 0`, `{at_org}<col>`.\nstderr:\n{stderr}"
        );
        for part in [
            "[0x0, 0x3) is assembled for the Z80 at origin 0x0",
            "in a second address space this org opens outside the ROM image",
            "and no -z instruction places it into the ROM",
        ] {
            assert!(errors[0].contains(part), "{shape}: must say `{part}`.\nstderr:\n{stderr}");
        }
    }
}

/// Z80 code org'd to `$100` in a 68000 program whose image leaves `$8..$200`
/// empty. It collides with nothing, and it is still unplaced: the org gave it
/// Z80 addresses and nothing gave it a place in the ROM.
#[test]
fn a_second_space_on_empty_image_ground_is_refused_too() {
    let src = "\tcpu 68000\n\
               \tdc.l 0, 0\n\
               \torg $200\n\
               \tdc.w 1\n\
               \tsave\n\
               \t!org $100\n\
               \tcpu z80\n\
               \tdb 1\n\
               \trestore\n\
               \t!org $300\n\
               \tdc.w 2\n";
    let error = assemble_refused(src);
    assert!(error.contains("root.asm(6):"), "at the `!org $100` line: {error}");
    assert!(error.contains("[0x100, 0x101) is assembled for the Z80 at origin 0x100"), "{error}");
}

/// The same Z80 byte with a `phase` open when it is emitted: the org placed it at
/// `$100` in the ROM and the phase gave it its run address, which is an image
/// placement and assembles. Every expected byte is read off the source.
#[test]
fn an_org_with_a_phase_open_still_places_z80_code_in_the_image() {
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
    let image = assemble_ok(src);
    assert_eq!(image.len(), 0x302, "ends after the `dc.w 2` at $300");
    assert_eq!(&image[0..8], &[0; 8], "the two longwords");
    assert_eq!(image[0x100], 0x01, "the phased Z80 byte, at its org");
    assert_eq!(&image[0x200..0x202], &[0x00, 0x01]);
    assert_eq!(&image[0x300..0x302], &[0x00, 0x02]);
}

/// A collision in the image is still an overlap, for a 68000 program and for a
/// Z80 one, whose image is the Z80's own space.
#[test]
fn a_collision_in_the_image_is_still_refused_as_an_overlap() {
    for src in [
        "\tcpu 68000\n\tdc.l 1\n\torg $10\n\tdc.w 2\n\torg 2\n\tdc.w 3\n",
        "\tcpu z80\n\torg 0\n\tdb 1,2,3,4\n\torg 10h\n\tdb 5\n\torg 2\n\tdb 6\n",
    ] {
        let error = assemble_refused(src);
        assert!(
            error.contains("[0x0, 0x4)") && error.contains("[0x2, ") && error.contains("overlap in the image"),
            "{error}\nsource:\n{src}"
        );
    }
}

/// A collision inside a second space is an overlap in that space, reported as
/// one before the space's missing placement.
#[test]
fn a_collision_inside_a_second_space_is_refused_as_an_overlap() {
    let src = "\tcpu 68000\n\
               \tdc.l 0, 0\n\
               \torg $200\n\
               \tdc.w 1\n\
               \tsave\n\
               \t!org 0\n\
               \tcpu z80\n\
               \tdb 1,2,3,4\n\
               \torg 10h\n\
               \tdb 5\n\
               \torg 2\n\
               \tdb 6\n\
               \trestore\n\
               \t!org $300\n\
               \tdc.w 2\n";
    let error = assemble_refused(src);
    assert!(
        error.contains("[0x0, 0x4)")
            && error.contains("[0x2, 0x3)")
            && error.contains("overlap in a second Z80 address space"),
        "{error}"
    );
}

/// Sonic 1's `SetupValues_Z80` block, `sonic.asm` lines 320-353 at s1disasm
/// f6ece657 copied verbatim into `vectors/s1_setupvalues_z80.asm`, between a
/// longword and a word of 68000 code, with `z80_ram`/`z80_ram_end` from
/// `_Constants.asm`. It is phased Z80 code with no `org`, so it is image bytes.
///
/// The expected image is the reference toolchain's: `asl` md5
/// 61e672562465725a8c102288a7da9098 (through `asl_ref.sh`'s `asl_run`, exit 0,
/// listing complete, 0 errors, 0 warnings) and that corpus's `p2bin`, md5
/// 4f2fff99c3347bafb93b12d5be1db754. sigil at 66d2ed86 produced the same bytes.
#[test]
fn sonic_1_s_phased_setup_block_assembles_to_the_reference_bytes() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/s1_setupvalues_z80.asm");
    let src = std::fs::read_to_string(&fixture).expect("read the fixture");
    let image = assemble_ok(&src);
    let reference: [u8; 44] = [
        0x12, 0x34, 0x56, 0x78, 0xAF, 0x01, 0xD9, 0x1F, 0x11, 0x27, 0x00, 0x21, 0x26, 0x00, 0xF9, 0x77,
        0xED, 0xB0, 0xDD, 0xE1, 0xFD, 0xE1, 0xED, 0x47, 0xED, 0x4F, 0xD1, 0xE1, 0xF1, 0x08, 0xD9, 0xC1,
        0xD1, 0xE1, 0xF1, 0xF9, 0xF3, 0xED, 0x56, 0x36, 0xE9, 0xE9, 0x4E, 0x71,
    ];
    assert_eq!(image, reference);
}

/// A Z80 program at `org 0` is a Z80 image and assembles, to its own encodings:
/// `di` F3, `ld a,1` 3E 01, `jp 0` C3 00 00.
#[test]
fn a_z80_program_at_org_0_still_assembles() {
    let image = assemble_ok("\tcpu z80\n\torg 0\n\tdi\n\tld a,1\n\tjp 0\n");
    assert_eq!(image, [0xF3, 0x3E, 0x01, 0xC3, 0x00, 0x00]);
}

/// A driver whose `!org 0` opens sections with no content before its code,
/// placed by `-z` where p2bin places it. The expected images are the reference
/// toolchain's for the same sources and the same `-z` (`asl` md5
/// 61e672562465725a8c102288a7da9098 through `asl_ref.sh`'s `asl_run`, exit 0,
/// 0 errors; `p2bin` md5 4f2fff99c3347bafb93b12d5be1db754), and every byte reads
/// off the source: the listing binds the driver's labels at Z80 0, `di` F3 at 0
/// and `ld a,1` 3E 01 at 1, and p2bin stores those three bytes right after the
/// image code before the driver, at $102. The third source enters a second space
/// again at `!org $1300` the same way, and its `db 7,8` follows the code at $110.
/// The sources are probes p12, p16 and p11 of
/// `docs/superpowers/notes/2026-09-12-second-space-pin/`.
#[test]
fn a_driver_behind_sections_with_no_content_is_placed_where_p2bin_places_it() {
    const HEAD: &str = "\tcpu 68000\nSize1 equ $10\nSize2 equ $10\n\tdc.l 0, 0\n\torg $100\n\tdc.w $4E71\n";
    let two_labels = format!(
        "{HEAD}\tsave\n\t!org 0\nDriverStart:\n\tcpu z80\nInner:\n\tcpu z80\n\tdi\n\tld a,1\n\
         \trestore\n\tpadding off\n\t!org $110\n\tdc.w $4E71\n"
    );
    let no_section_open: &str = "\tcpu 68000\nSize1 equ $10\n\tdc.l 0, 0\n\torg $100\n\tdc.w $4E71\n\tcpu 68000\n\
                                 \tsave\n\t!org 0\nDriverStart:\n\tcpu z80\n\tdi\n\tld a,1\n\
                                 \trestore\n\tpadding off\n\t!org $110\n\tdc.w $4E71\n";
    let two_spaces = format!(
        "{HEAD}\tsave\n\t!org 0\nD1:\n\tcpu z80\n\tdi\n\tld a,1\n\trestore\n\tpadding off\n\
         \t!org $110\n\tdc.w $4E71\n\tsave\n\t!org $1300\nD2:\n\tcpu z80\n\tdb 7,8\n\
         \trestore\n\tpadding off\n\t!org $120\n\tdc.w $4E71\n"
    );
    // One driver: the vectors, the code at $100, the driver after it, the code at $110.
    let mut one = vec![0u8; 0x112];
    one[0x100..0x105].copy_from_slice(&[0x4E, 0x71, 0xF3, 0x3E, 0x01]);
    one[0x110..0x112].copy_from_slice(&[0x4E, 0x71]);
    // Two spaces: the same, then `db 7,8` after the code at $110, and the code at $120.
    let mut two = one.clone();
    two.resize(0x122, 0);
    two[0x112..0x114].copy_from_slice(&[0x07, 0x08]);
    two[0x120..0x122].copy_from_slice(&[0x4E, 0x71]);
    let z0 = "-z=0,uncompressed,Size1,after";
    let cases: [(&str, &str, &[&str], &[u8]); 3] = [
        ("two labels between the org and the code", &two_labels, &[z0], &one),
        ("the org finds no section open", no_section_open, &[z0], &one),
        ("two spaces, each behind a label", &two_spaces, &[z0, "-z=1300h,uncompressed,Size2,after"], &two),
    ];
    for (shape, src, z, want) in cases {
        let image = assemble_ok_with(src, z);
        assert_eq!(image, want, "{shape}: the reference toolchain's image");
    }
}
