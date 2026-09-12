//! A section closed with its write cursor behind its end: an `org` seek back
//! into bytes the section already holds, then a line that ends the section
//! (`cpu`, `phase`, `dephase`, a `restore` that changes the CPU) before the
//! cursor returns to the end. asl binds the code after it at the cursor and
//! writes that code's bytes over the section's tail. sigil binds the labels at
//! the cursor too, places the bytes there, and refuses the overlap by name, at
//! the `org`. Where the code after it writes nothing into the tail, the program
//! assembles to asl's image.
//!
//! What each test catches:
//!
//! - the refusal, for every closer: a front end that leaves the next section
//!   `Chained` exits 0 with its bytes `extent - cursor` past its labels (the
//!   silent wrong image); a linker that locates the overlap at the first line of
//!   the seek's section points at a line that can be anywhere above the `org`.
//! - nothing written into the tail: still assembles, to asl's bytes, so a fix
//!   that refuses every seek its section closes behind fails here.
//! - an overlap that is not this shape keeps the plain overlap diagnostic, so a
//!   linker that blames the seek for an `org` the author wrote fails here.
//!
//! Expected values come from the reference toolchain: `asl` md5
//! 61e672562465725a8c102288a7da9098 through `asl_ref.sh`'s `asl_run`, flags
//! `-xx -n -q -A -L -U -i .`, exit 0, listing complete, and the `p2bin` beside it,
//! md5 4f2fff99c3347bafb93b12d5be1db754. Each source is the probe of the same
//! shape name in `docs/superpowers/notes/2026-09-12-as-backward-seek-split/probes/`
//! without its first (comment) line, so every line number below is one less
//! than the probe listing's.

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
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("root.asm"), src).expect("write root.asm");
    let out_path = dir.path().join("out.bin");
    let out = run(dir.path(), "root.asm", &["-o", out_path.to_str().expect("utf-8 path")]);
    assert!(
        out.status.success(),
        "must assemble.\nstderr:\n{}\nsource:\n{src}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read(&out_path).expect("the image was written")
}

/// Assemble `src` as `root.asm`, require it refused with exactly one error and
/// no image written, and return that error line.
fn assemble_refused(src: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("root.asm"), src).expect("write root.asm");
    let out_path = dir.path().join("out.bin");
    let out = run(dir.path(), "root.asm", &["-o", out_path.to_str().expect("utf-8 path")]);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!out.status.success(), "must be refused.\nstderr:\n{stderr}\nsource:\n{src}");
    assert!(!out_path.exists(), "a refused program must write no image.\nstderr:\n{stderr}");
    let errors = error_lines(&stderr);
    assert_eq!(errors.len(), 1, "exactly one error.\nstderr:\n{stderr}\nsource:\n{src}");
    errors[0].to_string()
}

/// The 1-based line of the only line of `src` containing `needle`.
fn line_of(src: &str, needle: &str) -> usize {
    let hits: Vec<usize> =
        src.lines().enumerate().filter(|(_, l)| l.contains(needle)).map(|(i, _)| i + 1).collect();
    assert_eq!(hits.len(), 1, "`{needle}` must be on exactly one line of:\n{src}");
    hits[0]
}

/// Probe c01: four marker words at $8, a seek back to $A, one overwrite, then a
/// `cpu` naming the CPU already selected. asl's listing:
///
/// ```text
///  6/  8 : AAAA BBBB CCCC   Start:  dc.w $AAAA,$BBBB,$CCCC,$DDDD
///      E : DDDD
///  7/  A :                          org Start+2
///  8/  A : EEEE                     dc.w $EEEE
///  9/  C :                          cpu 68000
/// 10/  C : 1234             L_next: dc.w $1234
/// 11/  E : 0000 000E                dc.l *
/// ```
///
/// So the code after the `cpu` line is at $C, over the tail `CCCC DDDD` that
/// ends at $10, and it ends at $12.
const C01: &str = "\tcpu 68000\n\
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

#[test]
fn a_seek_its_section_closes_behind_is_refused_at_the_org_when_the_next_bytes_land_on_the_tail() {
    let error = assemble_refused(C01);
    let at = format!("root.asm({}):", line_of(C01, "org Start+2"));
    assert!(error.contains(&at), "located at the seek, `{at}`: {error}");
    for part in [
        "sections `sec0` [0x0, 0x10) and `sec12` [0xC, 0x12) overlap in the image",
        "this `org` left the write position at 0xC, inside bytes already written up to 0x10",
        "(asl writes the later bytes over the earlier ones; sigil refuses the overlap)",
    ] {
        assert!(error.contains(part), "must say `{part}`: {error}");
    }
}

/// Every closer the measurement found splitting labels from bytes, and the two
/// ways of writing into the tail that are not plain code. Per shape: the probe
/// source, the seek line, and from asl's listing the address the code after
/// the closer is placed at (the `L_next` bytes) and the end of the bytes
/// already written there (the end of the marker run). Every one of these
/// exited 0 from sigil before the next section was pinned.
const CLOSERS: [(&str, &str, &str, u32, u32); 9] = [
    (
        // listing: 7/ A org Start+2, 9/ C phase $8000, 10/ C(phys) 1234 L_next.
        "c02 phase",
        "\tcpu 68000\n\torg 0\n\tdc.l L_next\n\tdc.l L_after\n\
         Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\torg Start+2\n\tdc.w $EEEE\n\
         \tphase $8000\nL_next:\tdc.w $1234\n\tdc.l *\n\tdephase\nL_after:\tdc.w $5678\n",
        "org Start+2",
        0xC,
        0x10,
    ),
    (
        // listing: the phased run at phys 8..10, 8/ org $8002, 10/ dephase,
        // 11/ C 1234 L_next.
        "c03 dephase",
        "\tcpu 68000\n\torg 0\n\tdc.l L_next\n\tdc.l L_after\n\tphase $8000\n\
         Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\torg $8002\n\tdc.w $EEEE\n\tdephase\n\
         L_next:\tdc.w $1234\n\tdc.l *\n\tcpu 68000\nL_after:\tdc.w $5678\n",
        "org $8002",
        0xC,
        0x10,
    ),
    (
        // listing: Z80 bytes AA BB CC DD at phys 8..C, org 8001h, EE at 9,
        // restore, dephase, A 1234 L_next.
        "c04 cpu-changing restore closing a phased Z80 section",
        "\tcpu 68000\n\torg 0\n\tdc.l L_next\n\tdc.l L_after\n\tsave\n\tcpu z80\n\
         \tphase 8000h\n\tdb 0AAh,0BBh,0CCh,0DDh\n\torg 8001h\n\tdb 0EEh\n\trestore\n\
         \tdephase\nL_next:\tdc.w $1234\n\tdc.l *\n\tcpu 68000\nL_after:\tdc.w $5678\n",
        "org 8001h",
        0xA,
        0xC,
    ),
    (
        // listing: org Start, EEEE at 8, org Start+4 (C), cpu, C 1234 L_next.
        "c08 two seeks, the second short of the end",
        "\tcpu 68000\n\torg 0\n\tdc.l L_next\n\tdc.l L_after\n\
         Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\torg Start\n\tdc.w $EEEE\n\torg Start+4\n\
         \tcpu 68000\nL_next:\tdc.w $1234\n\tdc.l *\n\tcpu 68000\nL_after:\tdc.w $5678\n",
        "org Start+4",
        0xC,
        0x10,
    ),
    (
        // listing: org Start+2 (A), cpu, A 1234 L_next.
        "c09 a seek with no bytes after it",
        "\tcpu 68000\n\torg 0\n\tdc.l L_next\n\tdc.l L_after\n\
         Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\torg Start+2\n\tcpu 68000\n\
         L_next:\tdc.w $1234\n\tdc.l *\n\tcpu 68000\nL_after:\tdc.w $5678\n",
        "org Start+2",
        0xA,
        0x10,
    ),
    (
        // listing: cpu z80, phase 8000h, C(phys) 12 34 L_next.
        "c11 cpu z80 with a phase",
        "\tcpu 68000\n\torg 0\n\tdc.l L_next\n\tdc.l L_after\n\
         Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\torg Start+2\n\tdc.w $EEEE\n\tcpu z80\n\
         \tphase 8000h\nL_next:\tdb 12h,34h\n\tdw $\n\tdephase\n\tcpu 68000\n\
         L_after:\tdc.w $5678\n",
        "org Start+2",
        0xC,
        0x10,
    ),
    (
        // listing: AA BB CC DD at 4..8, org Start+1, EE at 5, cpu z80,
        // 6 12 34 L_next.
        "c13 a Z80 program",
        "\tcpu z80\n\torg 0\n\tdw L_next\n\tdw L_after\nStart:\tdb 0AAh,0BBh,0CCh,0DDh\n\
         \torg Start+1\n\tdb 0EEh\n\tcpu z80\nL_next:\tdb 12h,34h\n\tdw $\n\tcpu z80\n\
         L_after:\tdb 56h,78h\n",
        "org Start+1",
        0x6,
        0x8,
    ),
    (
        // listing: org Start+2 (A), EEEE, restore to the Z80, phase 8000h,
        // C(phys) 12 34 L_next.
        "c14 cpu-changing restore out of the host section",
        "\tcpu z80\n\tsave\n\tcpu 68000\n\torg 0\n\tdc.l L_next\n\tdc.l L_after\n\
         Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\torg Start+2\n\tdc.w $EEEE\n\trestore\n\
         \tphase 8000h\nL_next:\tdb 12h,34h\n\tdw $\n\tdephase\n\tcpu 68000\n\
         L_after:\tdc.w $5678\n",
        "org Start+2",
        0xC,
        0x10,
    ),
    (
        // listing: cpu, C L_next: ds.b 4, 10 1234. asl leaves `CCCC DDDD` in
        // place under the reservation; sigil fills a reservation that has bytes
        // after it, so the section still writes into the tail and is refused.
        "c18 a reservation over the tail",
        "\tcpu 68000\n\torg 0\n\tdc.l L_next\n\tdc.l L_after\n\
         Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\torg Start+2\n\tdc.w $EEEE\n\tcpu 68000\n\
         L_next:\tds.b 4\n\tdc.w $1234\n\tdc.l *\n\tcpu 68000\nL_after:\tdc.w $5678\n",
        "org Start+2",
        0xC,
        0x10,
    ),
];

#[test]
fn every_closer_is_refused_at_the_seek_when_the_next_bytes_land_on_the_tail() {
    for (shape, src, seek, at, end) in CLOSERS {
        let error = assemble_refused(src);
        let loc = format!("root.asm({}):", line_of(src, seek));
        assert!(error.contains(&loc), "{shape}: located at `{seek}`, `{loc}`: {error}");
        let what = format!("this `org` left the write position at {at:#X}, inside bytes already written up to {end:#X}");
        assert!(error.contains(&what), "{shape}: must say `{what}`: {error}");
    }
}

/// Probe c17: the code after the `cpu` line writes nothing into the tail. Its
/// label is at the cursor, and an `org` moves past the old end before its first
/// byte. asl's listing:
///
/// ```text
///  9/  C :                          cpu 68000
/// 10/  C :                  L_next:
/// 11/ 10 :                          org Start+8
/// 12/ 10 : 1234                     dc.w $1234
/// 13/ 12 : 0000 0012                dc.l *
/// 14/ 16 :                          cpu 68000
/// 15/ 16 : 5678             L_after: dc.w $5678
/// ```
///
/// and its image, through `p2bin`, 24 bytes:
/// `0000000c 00000016 aaaa eeee cccc dddd 1234 00000012 5678`.
#[test]
fn a_seek_its_section_closes_behind_assembles_to_asl_s_image_when_nothing_lands_on_the_tail() {
    let src = "\tcpu 68000\n\
               \torg 0\n\
               \tdc.l L_next\n\
               \tdc.l L_after\n\
               Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\
               \torg Start+2\n\
               \tdc.w $EEEE\n\
               \tcpu 68000\n\
               L_next:\n\
               \torg Start+8\n\
               \tdc.w $1234\n\
               \tdc.l *\n\
               \tcpu 68000\n\
               L_after:\tdc.w $5678\n";
    let asl: [u8; 24] = [
        0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x16, 0xAA, 0xAA, 0xEE, 0xEE, 0xCC, 0xCC, 0xDD, 0xDD,
        0x12, 0x34, 0x00, 0x00, 0x00, 0x12, 0x56, 0x78,
    ];
    assert_eq!(assemble_ok(src), asl);
}

/// Two overlaps next to a seek its section closes behind that the seek did not
/// cause. The author's own `org` placed the colliding code, so the diagnostic is
/// the plain overlap at the first line of the section overlapped, as every
/// backward `org` onto closed ground is.
///
/// - c19: the `org` lands inside the tail but not at the cursor. asl: `L_next`
///   at $E (listing `10/ E : 1234 L_next: dc.w $1234`).
/// - c20: other code at $20 first, then an `org` back to exactly the cursor.
///   asl: `$9999` at $20, `L_next` at $C.
#[test]
fn an_overlap_the_seek_did_not_place_keeps_the_plain_overlap_diagnostic() {
    let cases = [
        (
            "c19",
            "\tcpu 68000\n\torg 0\n\tdc.l L_next\n\tdc.l L_after\n\
             Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\torg Start+2\n\tdc.w $EEEE\n\tcpu 68000\n\
             \torg Start+6\nL_next:\tdc.w $1234\n\tdc.l *\n\tcpu 68000\nL_after:\tdc.w $5678\n",
            "sections `sec0` [0x0, 0x10) and `sec14` [0xE, 0x14) overlap in the image (colliding pins)",
        ),
        (
            "c20",
            "\tcpu 68000\n\torg 0\n\tdc.l L_next\n\tdc.l L_after\n\
             Start:\tdc.w $AAAA,$BBBB,$CCCC,$DDDD\n\torg Start+2\n\tdc.w $EEEE\n\tcpu 68000\n\
             \torg $20\n\tdc.w $9999\n\torg Start+4\nL_next:\tdc.w $1234\n\tdc.l *\n\
             \tcpu 68000\nL_after:\tdc.w $5678\n",
            "sections `sec0` [0x0, 0x10) and `sec12` [0xC, 0x12) overlap in the image (colliding pins)",
        ),
    ];
    for (shape, src, message) in cases {
        let error = assemble_refused(src);
        let at = format!("root.asm({}):", line_of(src, "dc.l L_next"));
        assert!(error.contains(&at), "{shape}: at the first line of `sec0`, `{at}`: {error}");
        assert!(error.ends_with(message), "{shape}: must end `{message}`: {error}");
    }
}
