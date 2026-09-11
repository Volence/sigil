//! `-z` and `-p` on the AS route: the instruction a disassembly's build script
//! gives `p2bin`, taken as written.
//!
//! Every expected image below was written by the p2bin binary (md5
//! `4f2fff99c3347bafb93b12d5be1db754`) from the pinned asl's (md5
//! `61e672562465725a8c102288a7da9098`) object file for the same source and the
//! same arguments (`docs/superpowers/notes/2026-09-11-s1-driver-stage2/`,
//! `mk_expect.py` and `new_probes.out`). Where p2bin refuses, sigil refuses too.
//! Where p2bin places nothing, or writes a stream the game would read back
//! wrong, sigil refuses by name, and each such test says so.
//!
//! What would go red, by half-fix: a `-z` parsed and the blob stored
//! uncompressed (the kosinski and saxman-bugged images); the wrong Kosinski
//! variant (the same images, byte for byte); the blob at the wrong offset,
//! `before` for `after` or at its own Z80 address (every placement image); an
//! overflow accepted (the refusals, with both sizes); `-p` ignored (the pad
//! images); only the first of two `-z` honoured (the two-driver images); a
//! second space no `-z` names placed anyway (the refusals at its `org`).

use std::path::Path;
use std::process::{Command, Output};

fn run(dir: &Path, root: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(dir.join(root))
        .args(extra)
        .output()
        .expect("spawn sigil")
}

/// Write `src` as `root.asm` (and `files` beside it) in a fresh directory, run
/// sigil on it with `args` and `-o out.bin`, and return the run and the image.
fn assemble(src: &str, files: &[(&str, &[u8])], args: &[&str]) -> (Output, Option<Vec<u8>>) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("root.asm"), src).expect("write root.asm");
    for (name, bytes) in files {
        std::fs::write(dir.path().join(name), bytes).expect("write a binclude file");
    }
    let out_path = dir.path().join("out.bin");
    let mut all = vec!["-o", out_path.to_str().expect("utf-8 path")];
    all.extend_from_slice(args);
    let out = run(dir.path(), "root.asm", &all);
    let image = std::fs::read(&out_path).ok();
    (out, image)
}

/// The image sigil builds from `src` with `args`, or a panic with what it said.
fn build(src: &str, files: &[(&str, &[u8])], args: &[&str]) -> Vec<u8> {
    let (out, image) = assemble(src, files, args);
    assert!(
        out.status.success(),
        "must build with {args:?}.\nstderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    image.expect("the image was written")
}

/// The error lines of a refused run, after requiring the refusal and that no
/// image was written.
fn refused(src: &str, files: &[(&str, &[u8])], args: &[&str]) -> String {
    let (out, image) = assemble(src, files, args);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!out.status.success(), "must be refused with {args:?}.\nstderr:\n{stderr}");
    assert!(image.is_none(), "a refused run must write no image.\nstderr:\n{stderr}");
    stderr
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex")).collect()
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// A 68000 program with a ten-byte Z80 driver at Z80 0 and a 16-byte gap after
/// the code before it. `Small` is a second constant the tests name instead.
const P_AFTER: &str = "\tcpu 68000\n\tpadding off\nGuess = 16\nSmall = 4\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\
\tsave\n\t!org 0\n\tcpu z80\n\tdb 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h\n\trestore\n\tpadding off\n\
\t!org Drv+Guess\n\tdc.b $AA,$BB\n";

/// The same driver after a gap: the label is at $20, the last byte written at 8.
const P_GAP: &str = "\tcpu 68000\n\tpadding off\nGuess = 16\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\n\t!org $20\nDrv:\n\
\tsave\n\t!org 0\n\tcpu z80\n\tdb 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h\n\trestore\n\tpadding off\n\
\t!org Drv+Guess\n\tdc.b $AA,$BB\n";

/// The S3K shape: the source emits the reservation itself, as padding.
const P_BEFORE: &str = "\tcpu 68000\n\tpadding off\nGuess = 16\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\tdc.b [16]$EE\n\
\tsave\n\t!org 0\n\tcpu z80\n\tdb 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h\n\trestore\n\tpadding off\n\
\t!org Drv+Guess\n\tdc.b $AA,$BB\n";

/// Two drivers, at Z80 0 and 1300h, each reserved by padding before it. The
/// second driver's `!org 1300h` is on line 20.
const P_TWO: &str = "\tcpu 68000\n\tpadding off\nGuess = 16\nGuess2 = 12\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\
\tdc.b [16]$EE\n\tsave\n\t!org 0\n\tcpu z80\n\tdb 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h\n\trestore\n\
\tpadding off\n\t!org Drv+Guess\nDrv2:\n\tdc.b [12]$DD\n\tsave\n\tcpu z80\n\t!org 1300h\n\
\tdb 20h,21h,22h,23h,24h,25h\n\trestore\n\tpadding off\n\t!org Drv2+Guess2\n\tdc.b $AA,$BB\n";

/// A driver with nothing written after it.
const P_LAST: &str = "\tcpu 68000\n\tpadding off\nGuess = 16\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\
\tsave\n\t!org 0\n\tcpu z80\n\tdb 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h\n\trestore\n";

/// A driver at Z80 $10.
const P_HEX: &str = "\tcpu 68000\n\tpadding off\nGuess = $30\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\
\tsave\n\t!org $10\n\tcpu z80\n\tdb 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h\n\trestore\n\tpadding off\n\
\t!org Drv+Guess\n\tdc.b $AA,$BB\n";

/// A twenty-byte driver in a 16-byte gap.
const P_BIG: &str = "\tcpu 68000\n\tpadding off\nGuess = 16\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\
\tsave\n\t!org 0\n\tcpu z80\n\tdb 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h\n\
\tdb 1Ah,1Bh,1Ch,1Dh,1Eh,1Fh,20h,21h,22h,23h\n\trestore\n\tpadding off\n\t!org Drv+Guess\n\tdc.b $AA,$BB\n";

/// A driver whose second half is written at a Z80 address that does not follow
/// its first half.
const P_SEG: &str = "\tcpu 68000\n\tpadding off\nGuess = 16\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\
\tsave\n\t!org 0\n\tcpu z80\n\tdb 10h,11h,12h\n\t!org 8\n\tdb 20h,21h\n\trestore\n\tpadding off\n\
\t!org Drv+Guess\n\tdc.b $AA,$BB\n";

/// 68000 code written right after the driver's `restore`, at the ROM address
/// that equals the driver's Z80 counter. asl gives it a record of its own.
const P_CONT68K: &str = "\tcpu 68000\n\tpadding off\nGuess = 16\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\
\tsave\n\t!org 0\n\tcpu z80\n\tdb 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h\n\trestore\n\tpadding off\n\
\t!org 10\n\tdc.b $C0,$C1\n\t!org Drv+Guess\n\tdc.b $AA,$BB\n";

/// A 140-byte driver with repeats, in a $200 gap.
const P_KOS: &str = "\tcpu 68000\n\tpadding off\nGuess = $200\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\
\tsave\n\t!org 0\n\tcpu z80\n\
\tdb \"The quick brown fox jumps over the lazy dog. The quick brown fox jumps again.\"\n\
\tdb 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0\n\
\tdb 1,2,3,4,5,1,2,3,4,5,1,2,3,4,5,0F3h,0F3h,0F3h,31h,0FCh,1Fh,0DDh,21h,0,40h\n\
\trestore\n\tpadding off\n\t!org Drv+Guess\n\tdc.b $AA,$BB\n";

/// Two drivers that both start at Z80 0.
const P_SAMEORIGIN: &str = "\tcpu 68000\n\tpadding off\nGuess = 16\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\
\tsave\n\t!org 0\n\tcpu z80\n\tdb 10h,11h,12h,13h,14h,15h,16h,17h,18h,19h\n\trestore\n\tpadding off\n\
\t!org Drv+Guess\n\tdc.b $AA,$BB\n\tsave\n\t!org 0\n\tcpu z80\n\tdb 20h,21h,22h,23h\n\trestore\n\
\tpadding off\n\t!org $40\n\tdc.b $CC,$DD\n";

/// No driver: a gap between sections, and a reservation's gap inside one.
const P_PAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n\tdc.b 1,2\n\t!org 8\n\tdc.b 3\n\tds.b 3\n\tdc.b 4\n";

/// A 100-byte driver included from `p1.bin`, whose authentic Saxman stream
/// holds a match that starts in the compressor's zero-filled ring buffer.
const P_STRADDLE: &str = "\tcpu 68000\n\tpadding off\nGuess = $80\n\torg 0\n\tdc.b 1,2,3,4,5,6,7,8\nDrv:\n\
\tsave\n\t!org 0\n\tcpu z80\n\tbinclude \"p1.bin\"\n\trestore\n\tpadding off\n\t!org Drv+Guess\n\tdc.b $AA,$BB\n";

/// `p1.bin`: the vector `p1` of `sigil-clownlzss-sys`'s `accurate_vectors.rs`.
const P1_BIN: &str = "01000100020201020102000200010002020102010200020001000202010201020002000100020201020102020102010200020001000202010201020002000100020201020102020102010201000100000102000101000201000201000201000201000201";

/// A probe built from skdisasm's own text (2fcd861c): `notZ80` and the `org`
/// macro from sonic3k.macrosetup.asm, the driver file's RAM phase blocks and
/// rewind, and both driver exits, verbatim, with small Z80 bodies. The second
/// driver's `!org 1300h` is on line 83.
const P_S3K: &str = r#"; A probe built from skdisasm's own text (2fcd861c): `notZ80` and the `org`
; macro from sonic3k.macrosetup.asm, the driver file's RAM phase blocks and
; rewind (Sound/Z80 Sound Driver.asm 18, 115, 199-227), and both driver exits
; (4433-4444 and 5313-5315), verbatim. Small Z80 bodies stand in for the
; driver and its data, and the two guesses are this probe's own.
	cpu 68000
	padding off

notZ80 function cpu,(cpu<>128)&&(cpu<>32988)

; make org safer (impossible to overwrite previously assembled bytes)
org macro address
	if notZ80(MOMCPU)
.diff := address - *
		if .diff < 0
			error "too much stuff before org $\{address} ($\{(-.diff)} bytes)"
		else
			while .diff > 1024
				; AS can only generate 1 kb of code on a single line
				dc.b [1024]$FF
.diff := .diff - 1024
			endm
			dc.b [.diff]$FF
		endif
	else
		if address < $
			error "too much stuff before org 0\{address}h (0\{($-address)}h bytes)"
		else
			while address > $
				db 0
			endm
		endif
	endif
    endm

Size_of_Snd_driver_guess = $A0
Size_of_Snd_driver2_guess = $60
zDataStart = $1C00

	!org 0
	dc.l 0,0
	dc.w $4E71
z80_SoundDriverStart:
		phase zDataStart
zSongBank:	ds.b 2
zTempVariablesStart:	ds.b $20
		dephase

		phase $1C40
zTracksSFXStart:	ds.b $30
		dephase
; ---------------------------------------------------------------------------
		!org z80_SoundDriverStart	; Rewind the ROM address to where we were earlier (allocating the RAM above messes with it)
; z80_SoundDriver:
Z80_SoundDriver:
		org Z80_SoundDriver+Size_of_Snd_driver_guess	; This 'org' inserts some padding that we can paste the compressed sound driver over later (see the 's3p2bin' tool)

		save
		!org 0	; z80 Align, handled by the build process
		CPU Z80
		listing purecode
		di
		im 1
		ld sp,1FFEh
		jp 38h
		org 38h
		ld a,(1C00h)
		or a
		jr z,38h
		db "SMPS Z80 driver stand-in", 0
		db 1,2,3,4,5,6,7,8,1,2,3,4,5,6,7,8,1,2,3,4,5,6,7,8
		restore
		padding off
		!org Z80_SoundDriver+Size_of_Snd_driver_guess	; The assembler still thinks we're in Z80 memory, so use an 'org' to switch back to the cartridge

; Z80_Snd_Driver2:
Z80_SoundDriverData:
		org Z80_SoundDriverData+Size_of_Snd_driver2_guess	; Once again, create some padding that we can paste the compressed data over later
; ---------------------------------------------------------------------------
		save
		CPU Z80
		listing purecode
		!org 1300h	; z80 Align, handled by the build process
		dw 1310h, 1320h, 1330h
		db 0F2h, 80h, 0E0h, 0F2h, 80h, 0E0h, 0F2h, 80h, 0E0h
		db "music data stand-in", 0
		restore
		padding off
		!org Z80_SoundDriverData+Size_of_Snd_driver2_guess	; The assembler still thinks we're in Z80 memory, so use an 'org' to switch back to the cartridge
		dc.w $4E75
"#;

#[test]
fn after_stores_the_blob_where_the_code_before_it_ends_and_pads_the_rest() {
    let got = build(P_AFTER, &[], &["-p=FF", "-z=0,uncompressed,Guess,after"]);
    assert_eq!(got, hex("010203040506070810111213141516171819ffffffffffffaabb"));
}

/// p2bin never reads the constant's value: it only names it in its overflow
/// message. A different constant, or one that does not exist, gives the same
/// image (measured: `Small` and `Nope` in place of `Guess`).
#[test]
fn the_constant_is_a_name_and_its_value_is_never_read() {
    let want = hex("010203040506070810111213141516171819ffffffffffffaabb");
    assert_eq!(build(P_AFTER, &[], &["-p=FF", "-z=0,uncompressed,Small,after"]), want);
    assert_eq!(build(P_AFTER, &[], &["-p=FF", "-z=0,uncompressed,Nope,after"]), want);
}

#[test]
fn the_pad_byte_is_p2bins_and_zero_without_it() {
    let zero = hex("010203040506070810111213141516171819000000000000aabb");
    assert_eq!(build(P_AFTER, &[], &["-p=0", "-z=0,uncompressed,Guess,after"]), zero);
    assert_eq!(build(P_AFTER, &[], &["-z=0,uncompressed,Guess,after"]), zero);
    // Between two sections, and inside one where a reservation leaves a gap.
    assert_eq!(build(P_PAD, &[], &["-p=FF"]), hex("0102ffffffffffff03ffffff04"));
    assert_eq!(build(P_PAD, &[], &["-p=0"]), hex("01020000000000000300000004"));
    assert_eq!(build(P_PAD, &[], &[]), hex("01020000000000000300000004"));
}

/// `after` is measured from the last byte written before the driver, not from
/// the driver's label: the label here is at $20 and the blob lands at 8.
#[test]
fn after_is_measured_from_the_last_byte_written_not_from_the_label() {
    let got = build(P_GAP, &[], &["-p=FF", "-z=0,uncompressed,Guess,after"]);
    assert_eq!(
        got,
        hex("010203040506070810111213141516171819ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffaabb")
    );
}

#[test]
fn before_stores_the_blob_over_the_start_of_the_code_before_it() {
    let got = build(P_BEFORE, &[], &["-p=FF", "-z=0,uncompressed,Guess,before"]);
    assert_eq!(got, hex("10111213141516171819eeeeeeeeeeeeeeeeeeeeeeeeeeeeaabb"));
}

#[test]
fn with_nothing_written_after_it_the_blob_is_unbounded() {
    let got = build(P_LAST, &[], &["-p=FF", "-z=0,uncompressed,Guess,after"]);
    assert_eq!(got, hex("010203040506070810111213141516171819"));
}

#[test]
fn the_address_is_hexadecimal() {
    let want = hex(
        "010203040506070810111213141516171819ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffaabb",
    );
    assert_eq!(build(P_HEX, &[], &["-p=FF", "-z=10,uncompressed,Guess,after"]), want);
    assert_eq!(build(P_HEX, &[], &["-p=FF", "-z=0x10,uncompressed,Guess,after"]), want);
}

/// The stored streams are p2bin's own: a stream stored uncompressed, or by
/// clownlzss's optimal Kosinski, differs from these.
#[test]
fn kosinski_and_saxman_bugged_store_the_bytes_p2bin_stores() {
    assert_eq!(
        build(P_AFTER, &[], &["-p=FF", "-z=0,kosinski,Guess,after"]),
        hex("0102030405060708ff0b1011121314151617181900f00000aabb")
    );
    assert_eq!(
        build(P_AFTER, &[], &["-p=FF", "-z=0,saxman-bugged,Guess,after"]),
        hex("0102030405060708ff101112131415161703181900ffffffaabb")
    );
    for (format, want) in [("uncompressed", 0x42b2bfe6), ("kosinski", 0xe004a55a), ("saxman-bugged", 0xd72a6d1a)] {
        let got = build(P_KOS, &[], &["-p=FF", &format!("-z=0,{format},Guess,after")]);
        assert_eq!((got.len(), crc32(&got)), (522, want), "{format}");
    }
}

#[test]
fn both_blobs_of_a_two_driver_program_are_placed_in_either_order() {
    let want = hex("10111213141516171819eeeeeeeeeeeeeeeeeeeeeeeeeeee202122232425ddddddddddddaabb");
    let first = ["-p=FF", "-z=0,uncompressed,Guess,before", "-z=1300,uncompressed,Guess2,before"];
    let second = ["-p=FF", "-z=1300,uncompressed,Guess2,before", "-z=0,uncompressed,Guess,before"];
    assert_eq!(build(P_TWO, &[], &first), want);
    assert_eq!(build(P_TWO, &[], &second), want);
    // The Sonic 3 & Knuckles shape, from its own text, as buildSK.lua and
    // buildS3.lua instruct.
    for (format, want) in [("kosinski", 0xeacc2d60), ("uncompressed", 0xc3956dd1)] {
        let args = [
            "-p=FF".to_string(),
            format!("-z=0,{format},Size_of_Snd_driver_guess,before"),
            format!("-z=1300,{format},Size_of_Snd_driver2_guess,before"),
        ];
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let got = build(P_S3K, &[], &args);
        assert_eq!((got.len(), crc32(&got)), (268, want), "{format}");
    }
}

/// One `-z` places every blob that starts at its address, each against the code
/// before it, as p2bin does.
#[test]
fn one_instruction_places_every_blob_that_starts_at_its_address() {
    let got = build(P_SAMEORIGIN, &[], &["-p=FF", "-z=0,uncompressed,Guess,after"]);
    assert_eq!(
        got,
        hex("010203040506070810111213141516171819ffffffffffffaabb20212223ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffccdd")
    );
}

/// p2bin refuses these too ("Space reserved for the compressed Z80 segments is
/// too small. Set 'Guess' to at least $14."). sigil names both sizes and where
/// the reservation is.
#[test]
fn a_blob_larger_than_its_reservation_is_refused_with_both_sizes() {
    let e = refused(P_BIG, &[], &["-p=FF", "-z=0,uncompressed,Guess,after"]);
    for part in ["is 0x14 bytes stored uncompressed", "only 0x10 are reserved", "[0x8, 0x18)", "set `Guess` to at least $14"] {
        assert!(e.contains(part), "must say `{part}`:\n{e}");
    }
    let e = refused(P_BIG, &[], &["-p=FF", "-z=0,uncompressed,Guess,before"]);
    assert!(e.contains("only 0x8 are reserved") && e.contains("which it overwrites"), "{e}");
    let e = refused(P_KOS, &[], &["-p=0", "-z=0,kosinski,Guess,before"]);
    assert!(e.contains("is 0x60 bytes compressed as kosinski") && e.contains("at least $60"), "{e}");
}

/// asl gives the 68000 bytes after the driver's `restore` a record of their
/// own, so the gap after the driver is two bytes and p2bin refuses. If they
/// joined the driver's section they would be compressed into the blob and the
/// program would build.
#[test]
fn code_written_after_a_restore_is_not_part_of_the_blob() {
    let e = refused(P_CONT68K, &[], &["-p=FF", "-z=0,uncompressed,Guess,after"]);
    assert!(e.contains("the blob [0x0, 0xA)") && e.contains("only 0x2 are reserved"), "{e}");
}

/// p2bin, told about one driver of two, writes the other at its Z80 address,
/// over whatever the image holds there. sigil refuses it at its own `org`.
#[test]
fn a_second_space_no_instruction_names_is_refused_at_its_org() {
    let e = refused(P_TWO, &[], &["-p=FF", "-z=0,uncompressed,Guess,before"]);
    assert!(e.contains("root.asm(20):") && e.contains("[0x1300, 0x1306)") && e.contains("no -z instruction places it"), "{e}");
    let e = refused(P_TWO, &[], &["-p=FF", "-z=1300,uncompressed,Guess2,before"]);
    assert!(e.contains("root.asm(10):") && e.contains("[0x0, 0xA)") && e.contains("no -z instruction places it"), "{e}");
    let e = refused(P_S3K, &[], &["-p=FF", "-z=0,kosinski,Size_of_Snd_driver_guess,before"]);
    assert!(e.contains("root.asm(83):") && e.contains("origin 0x1300"), "{e}");
}

/// p2bin compresses only the first run and refuses for want of space; a blob
/// that does not continue is refused before its size is weighed.
#[test]
fn code_in_the_space_that_does_not_continue_the_blob_is_refused() {
    let e = refused(P_SEG, &[], &["-p=FF", "-z=0,uncompressed,Guess,after"]);
    assert!(e.contains("also holds [0x8, 0xA), which does not continue it"), "{e}");
}

/// p2bin ignores an instruction that matches nothing.
#[test]
fn an_instruction_that_names_no_second_space_is_refused() {
    let e = refused(P_PAD, &[], &["-p=FF", "-z=0,kosinski,Guess,after"]);
    assert!(e.contains("`-z=0,kosinski,Guess,after` names no second address space"), "{e}");
}

/// p2bin writes this stream. Sonic 2's decompressor reads a match whose source
/// starts before the output as zeros throughout, so the game would load
/// different bytes than were assembled; sigil's round-trip check refuses it.
/// The same driver as Kosinski round-trips and is placed as p2bin places it.
#[test]
fn a_stream_the_game_would_read_back_wrong_is_refused() {
    let p1 = hex(P1_BIN);
    let files: [(&str, &[u8]); 1] = [("p1.bin", &p1)];
    let e = refused(P_STRADDLE, &files, &["-p=FF", "-z=0,saxman-bugged,Guess,after"]);
    assert!(e.contains("does not decompress to the blob that was assembled"), "{e}");
    let got = build(P_STRADDLE, &files, &["-p=FF", "-z=0,kosinski,Guess,after"]);
    let mut want = hex("0102030405060708c3110100fe020201fe9400f9f5f81ee5f81fb5fd5101ebeafdf80c00f0000000");
    want.resize(0x88, 0xFF);
    want.extend_from_slice(&[0xAA, 0xBB]);
    assert_eq!(got, want);
}

/// p2bin prints its complaint and goes on building; sigil stops.
#[test]
fn a_p2bin_grammar_error_stops_the_run() {
    for (args, part) in [
        (vec!["-p=$FF"], "not a hexadecimal number"),
        (vec!["-p=100"], "FF or lower"),
        (vec!["-z=0,kosinski-optimised,Guess,after"], "is a p2bin format sigil does not implement"),
        (vec!["-z=0,Kosinski,Guess,after"], "is not a p2bin format"),
        (vec!["-z=0,kosinski,Guess"], "is written `-z=<address>,<format>,<constant>,<before|after>`"),
        (vec!["-pFF"], "unexpected argument '-pFF'"),
    ] {
        let (out, image) = assemble(P_AFTER, &[], &args);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}:\n{stderr}");
        assert!(image.is_none(), "{args:?}");
        assert!(stderr.contains(part), "{args:?} must say `{part}`:\n{stderr}");
    }
}
