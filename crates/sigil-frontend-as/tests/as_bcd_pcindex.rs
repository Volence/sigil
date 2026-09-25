//! The 68000's extended and decimal family (`addx`, `subx`, `abcd`, `sbcd`,
//! `negx`, `nbcd`) and the `(d8,PC,Xn)` / `(d16,PC)` memory operand of `movem`.
//!
//! Sonic 3 & Knuckles writes `abcd -(a1),-(a2)` (`sonic3k.asm` 62886-62888),
//! `subx.w` (100959, 119898, 119919) and `movem.w label(pc,d0.w),dN-dM`
//! (174875, 174882). The pair family encodes `base | Rx<<9 | ss<<6 | rm<<3 | Ry`
//! with the destination in bits 11-9, so a swapped register field or a dropped
//! `rm` bit is a different, valid instruction; every probe below uses distinct
//! non-zero registers for source and destination so those readings differ.
//!
//! ## The rules, each measured
//!
//! - `addx`/`subx`/`negx` take `.b`/`.w`/`.l` and default to `.w`;
//!   `abcd`/`sbcd`/`nbcd` are byte only (`.b` or bare) and refuse `.w`/`.l`
//!   (`#1130 invalid operand size`).
//! - The pair forms are exactly `Dy,Dx` and `-(Ay),-(Ax)`; a mixed pair or any
//!   other mode is `#1350 addressing mode not allowed here`.
//! - `negx`/`nbcd` take the data-alterable modes; `An`, `#imm` and PC-relative
//!   are refused.
//! - `(d8,PC,Xn)` is a SOURCE mode wherever the EA class admits it, `movem`'s
//!   memory-to-register direction included; the displacement is measured from
//!   the EA extension word, which for `movem` and `btst #n` follows another
//!   word. -128 and 127 assemble; -129 and 128 are refused, which asl reports as
//!   `#1505 addressing mode not supported on 68000` (it falls through to the
//!   68020's full-format mode). The index is `dN`/`aN`/`sp`, `.w` (the default)
//!   or `.l`, in any case; `(disp,pc,Xn)` and `disp(pc,Xn)` are the same mode.
//! - As a destination it is refused (`#1350`), `movem`'s register-to-memory
//!   direction included.
//!
//! ## Provenance
//!
//! Every expected byte string below is the hex of asl's own image, never
//! computed: `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
//! md5 `61e672562465725a8c102288a7da9098`, run through
//! `docs/superpowers/notes/asl-reference/asl_ref.sh`'s `asl_run -xx -n -q -A -L
//! -U -i .`, exit 0 with its pass loop complete, then its `p2bin` with `-p=0`.
//! The sources are the probe files verbatim (`s3k-bcd-pcindex/probes/` beside
//! the note of the same date). The `sk*` sources are `sonic3k.asm` lines at the
//! census revision, phased at the address the reference build's listing gives
//! them, and asl's image of each equals the reference ROM (`4ea493ea...`) there.
//! The refusal tests quote asl's diagnostic from a run that exited 2; no byte of
//! those runs is used.

use sigil_frontend_as::{assemble_root_located, Options};

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    let m = match assemble_root_located(&path, &Options::default()) {
        Ok(m) => m,
        Err(f) => return Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    };
    let msgs = |d: Vec<sigil_span::Diagnostic>| d.iter().map(|d| d.message.clone()).collect::<Vec<_>>();
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(msgs)?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).map_err(msgs)?;
    Ok(sigil_link::flatten(&linked, 0x00).unwrap())
}

/// Assemble `src` and require asl's bytes, given as asl's own hex.
fn assert_asl(src: &str, asl_hex: &str) {
    let got = match assemble(src) {
        Ok(b) => b,
        Err(d) => panic!("expected asl's bytes {asl_hex}, got diagnostics: {d:?}"),
    };
    let got_hex: String = got.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(got_hex, asl_hex, "sigil's image differs from asl's");
}

/// Assemble `src` and require a refusal whose text contains every one of
/// `needles`, so a neighbouring refusal path cannot stand in for this one.
fn assert_refused(src: &str, needles: &[&str]) {
    let d = match assemble(src) {
        Ok(b) => panic!("expected a refusal, got bytes: {b:02X?}"),
        Err(d) => d,
    };
    let all = d.join("\n");
    for n in needles {
        assert!(all.contains(n), "refusal does not name `{n}`: {all}");
    }
}

// ---- refusals ----------------------------------------------------------------

/// asl: `#1130 invalid operand size` (probes xr01, xr02, xr14, xr19, xr22).
#[test]
fn the_decimal_forms_are_byte_only() {
    for src in ["abcd.w d3,d5", "abcd.l d3,d5", "sbcd.w d3,d5", "nbcd.w d3", "nbcd.l (a3)"] {
        assert_refused(&format!("\tcpu 68000\n\t{src}\n"), &["byte-only"]);
    }
}

/// asl: `#1350 addressing mode not allowed here` (probes xr03 to xr13, xr28, xr29).
#[test]
fn the_pair_forms_are_dn_dn_or_predec_predec_only() {
    for src in [
        "abcd d3,-(a5)",
        "abcd -(a3),d5",
        "abcd (a3),(a5)",
        "abcd (a3)+,(a5)+",
        "addx d3,-(a5)",
        "addx -(a3),d5",
        "addx (a3),(a5)",
        "subx d3,-(a5)",
        "subx.w (a3)+,(a5)+",
        "subx.b a3,d5",
        "sbcd d3,a5",
    ] {
        assert_refused(&format!("\tcpu 68000\n\t{src}\n"), &["Dy,Dx or -(Ay),-(Ax)"]);
    }
}

/// asl: `#1350 addressing mode not allowed here` (probes xr15, xr16, xr17,
/// xr20, xr21, pcr16, pcr17).
#[test]
fn negx_and_nbcd_refuse_non_data_alterable_operands() {
    for src in ["negx a3", "nbcd a3"] {
        assert_refused(&format!("\tcpu 68000\n\t{src}\n"), &["An is not a legal addressing mode"]);
    }
    for src in ["negx #1", "nbcd #1", "negx 4(pc)"] {
        assert_refused(&format!("\tcpu 68000\n\t{src}\n"), &["illegal destination EA"]);
    }
    for src in ["nbcd T(pc,d3.w)\nT: dc.w 1", "negx.w T(pc,d3.w)\nT: dc.w 1"] {
        assert_refused(&format!("\tcpu 68000\n\t{src}\n"), &["illegal destination EA: (d8,PC,Xn)"]);
    }
}

/// asl: `#1505 addressing mode not supported on 68000` for each (probes pcbr01
/// to pcbr08). The base is the EA extension word: address + 2, or + 4 for
/// `movem` and `btst #n`, so each of these is exactly one past the boundary.
#[test]
fn pc_indexed_displacement_one_past_either_end_is_refused() {
    for (src, disp) in [
        ("move.w *+2+128(pc,d3.w),d5", "(128)"),
        ("move.w *+2-129(pc,d3.w),d5", "(-129)"),
        ("movem.w *+4+128(pc,d3.w),d2-d3", "(128)"),
        ("movem.w *+4-129(pc,d3.w),d2-d3", "(-129)"),
        ("btst #3,*+4+128(pc,d3.w)", "(128)"),
        ("btst #3,*+4-129(pc,d3.w)", "(-129)"),
        ("lea *+2+128(pc,a3.l),a5", "(128)"),
        ("lea *+2-129(pc,a3.l),a5", "(-129)"),
    ] {
        assert_refused(
            &format!("\tcpu 68000\n\t{src}\n"),
            &["(d8,PC,Xn) displacement", "out of range", disp],
        );
    }
}

/// asl: `#1350 addressing mode not allowed here` (probes pcr07, pcr23).
#[test]
fn movem_refuses_a_pc_relative_destination() {
    for src in ["movem.w d2-d3,T(pc,d0.w)\nT: dc.w 1", "movem.l d0-d2,T(pc)\nT: dc.w 1"] {
        assert_refused(&format!("\tcpu 68000\n\t{src}\n"), &["illegal destination EA"]);
    }
}

/// asl: `#1350 addressing mode not allowed here` (probes pcr21, pcr22).
#[test]
fn a_second_pc_relative_operand_is_named_as_a_destination() {
    for src in ["move.w T(pc),T(pc)\nT: dc.w 1", "move.w T(pc,d3.w),T(pc,d4.w)\nT: dc.w 1"] {
        assert_refused(&format!("\tcpu 68000\n\t{src}\n"), &["PC-relative", "destination position"]);
    }
}

/// asl: `#1350 addressing mode not allowed here` (probes pcr14, pcr15).
#[test]
fn movem_pc_indexed_refuses_a_bad_index_register() {
    for src in [
        "movem.w T(pc,d3.b),d2-d3\nT: dc.w 1",
        "movem.w T(pc,x.w),d2-d3\nT: dc.w 1",
    ] {
        assert_refused(&format!("\tcpu 68000\n\t{src}\n"), &["not a valid index register"]);
    }
}

// ---- accepted probes: asl's image, byte for byte -----------------------------

/// Probe `x01`.
#[test]
fn addx_and_subx_every_size_both_forms_and_the_bare_word_default() {
    assert_asl(
        r#"	cpu 68000
	addx.b d3,d5
	addx.w d3,d5
	addx.l d3,d5
	addx d3,d5
	addx.b -(a3),-(a5)
	addx.w -(a3),-(a5)
	addx.l -(a3),-(a5)
	addx -(a3),-(a5)
	addx.w d6,d1
	addx.l -(sp),-(a6)
	addx.w -(a1),-(a7)
	subx.b d3,d5
	subx.w d3,d5
	subx.l d3,d5
	subx d3,d5
	subx.b -(a3),-(a5)
	subx.w -(a3),-(a5)
	subx.l -(a3),-(a5)
	subx -(a3),-(a5)
	subx.w d6,d1
	subx.l -(sp),-(a6)
	ADDX.W D3,D5
	SUBX.L -(A3),-(A5)
"#,
        "db03db43db83db43db0bdb4bdb8bdb4bd346dd8fdf499b039b439b839b439b0b9b4b9b8b9b4b93469d8fdb439b8b",
    );
}

/// Probe `x02`.
#[test]
fn abcd_and_sbcd_both_forms_bare_and_byte_suffixed() {
    assert_asl(
        r#"	cpu 68000
	abcd d3,d5
	abcd.b d3,d5
	abcd -(a3),-(a5)
	abcd.b -(a3),-(a5)
	abcd d6,d1
	abcd -(a1),-(a2)
	abcd -(sp),-(a6)
	sbcd d3,d5
	sbcd.b d3,d5
	sbcd -(a3),-(a5)
	sbcd.b -(a3),-(a5)
	sbcd d6,d1
	sbcd -(a1),-(sp)
	ABCD -(A3),-(A5)
"#,
        "cb03cb03cb0bcb0bc306c509cd0f8b038b038b0b8b0b83068f09cb0b",
    );
}

/// Probe `x03`.
#[test]
fn negx_and_nbcd_over_the_data_alterable_modes() {
    assert_asl(
        r#"	cpu 68000
	negx.b d3
	negx.w d3
	negx.l d3
	negx d5
	negx.b (a3)
	negx.w (a3)+
	negx.l -(a3)
	negx.w $1234(a5)
	negx.b -$12(a5,d6.w)
	negx.l $12(a5,a2.l)
	negx.w ($1234).w
	negx.l ($123456).l
	nbcd d3
	nbcd.b d5
	nbcd (a3)
	nbcd (a3)+
	nbcd -(a3)
	nbcd $1234(a5)
	nbcd -$12(a5,d6.w)
	nbcd $12(a5,a2.l)
	nbcd ($1234).w
	nbcd ($123456).l
"#,
        "40034043408340454013405b40a3406d1234403560ee40b5a8124078123440b900123456480348054813481b4823482d1234483560ee4835a81248381234483900123456",
    );
}

/// Probe `pcx01`.
#[test]
fn pc_indexed_source_over_every_instruction_that_admits_it() {
    assert_asl(
        r#"	cpu 68000
	move.b T(pc,d3.w),d5
	move.w T(pc,d3.l),d5
	move.l T(pc,a3.w),d5
	move.w T(pc,a3.l),(a5)
	move.l T(pc,d6.w),$1234(a5)
	movea.w T(pc,d3.w),a5
	movea.l T(pc,a2.l),a5
	move.w T(pc,d3.w),a5
	add.b T(pc,d3.w),d5
	add.w T(pc,d3.l),d5
	add.l T(pc,a3.w),d5
	adda.w T(pc,d3.w),a5
	adda.l T(pc,d3.l),a5
	sub.w T(pc,d3.w),d5
	suba.l T(pc,a4.w),a5
	and.w T(pc,d3.w),d5
	or.l T(pc,d3.l),d5
	cmp.b T(pc,d3.w),d5
	cmp.w T(pc,a3.l),d5
	cmpa.w T(pc,d3.w),a5
	cmpa.l T(pc,d3.w),a5
	muls.w T(pc,d3.w),d5
	mulu.w T(pc,d3.l),d5
	divs.w T(pc,a3.w),d5
	divu.w T(pc,d3.w),d5
	btst d5,T(pc,d3.w)
	btst #3,T(pc,d3.l)
T: dc.w 1
	lea T(pc,d3.w),a5
	lea T(pc,a3.l),a5
	pea T(pc,d3.w)
	jmp T(pc,d3.w)
	jsr T(pc,a3.l)
	movem.w T(pc,d0.w),d2-d3
	movem.l T(pc,d3.l),d0-d2/a4
	movem.w T(pc,a3.w),a0/d6
	movem.l T(pc,a2.l),d0-d7/a0-a6
	move.w T(pc,d3.w),ccr
	move.w T(pc,d3.w),sr
	move T(pc,d3.w),ccr
	move.w T(pc,d3),d5
	move.w T(PC,D3.W),d5
	move.w T(Pc,d3.W),d5
	move.w (T,pc,d3.w),d5
	move.w (T,PC,a3.L),d5
	move.w T(pc,sp.w),d5
	move.w T(pc,a7.l),d5
	movem.w (T,pc,d3.w),d2/d4
	movem.w T(pc,d3),d2/d4
"#,
        "1a3b306e3a3b386a2a3bb0663abbb8622b7b605e12343a7b30582a7ba8543a7b3050da3b304cda7b3848dabbb044dafb3040dbfb383c9a7b30389bfbc034ca7b30308abb382cba3b3028ba7bb824bafb3020bbfb301ccbfb3018cafb38148bfbb0108afb300c0b3b3008083b0003380200014bfb30fc4bfbb8f8487b30f44efb30f04ebbb8ec4cbb000c00e64cfb100738e04cbb0140b0da4cfb7fffa8d444fb30d046fb30cc44fb30c83a3b30c43a3b30c03a3b30bc3a3b30b83a3bb8b43a3bf0b03a3bf8ac4cbb001430a64cbb001430a0",
    );
}

/// Probe `pcx02`.
#[test]
fn pc_indexed_backward_targets() {
    assert_asl(
        r#"	cpu 68000
B: dc.w $1111,$2222
	move.w B(pc,d3.w),d5
	movem.w B(pc,d0.w),d2-d3
	lea B(pc,a3.l),a5
	jmp B(pc,d6.w)
	btst #7,B(pc,d3.w)
"#,
        "111122223a3b30fa4cbb000c00f44bfbb8f04efb60ec083b000730e6",
    );
}

/// Probe `pcx03`.
#[test]
fn pc_indexed_movem_the_s3k_shape() {
    assert_asl(
        r#"	cpu 68000
	movem.w word_82872(pc,d0.w),d2-d3
	movem.w word_82832(pc,d0.w),d4-d5
word_82832: dc.w 1,2,3,4
word_82872: dc.w 5,6,7,8
"#,
        "4cbb000c00104cbb0030000200010002000300040005000600070008",
    );
}

/// Probe `pcb01`.
#[test]
fn pc_indexed_displacement_boundaries_127_and_minus_128_accepted() {
    assert_asl(
        r#"	cpu 68000
	move.w *+2+127(pc,d3.w),d5
	move.w *+2-128(pc,d3.w),d5
	movem.w *+4+127(pc,d3.w),d2-d3
	movem.w *+4-128(pc,d3.w),d2-d3
	lea *+2+127(pc,a3.l),a5
	lea *+2-128(pc,a3.l),a5
	btst #3,*+4+127(pc,d3.w)
	btst #3,*+4-128(pc,d3.w)
	btst d5,*+2+127(pc,d3.w)
	jmp *+2-1(pc,d3.w)
"#,
        "3a3b307f3a3b30804cbb000c307f4cbb000c30804bfbb87f4bfbb880083b0003307f083b000330800b3b307f4efb30ff",
    );
}

/// Probe `pcd01`.
#[test]
fn movem_through_d16_pc() {
    assert_asl(
        r#"	cpu 68000
	movem.w T(pc),d2-d3
	movem.l (T,pc),d0/a1
T: dc.w 1
	movem.w T(pc),a2/d6
"#,
        "4cba000c00084cfa0201000200014cba0440fffa",
    );
}

/// Probe `sk01`.
#[test]
fn s3k_extract_both_movem_sites_and_their_tables() {
    assert_asl(
        r#"	cpu 68000
x_vel = $18
y_vel = $1A
	phase $8280C
		movem.w	word_82872(pc,d0.w),d2-d3
		move.w	d2,x_vel(a0)
		move.w	d3,y_vel(a0)

loc_8281A:
		tst.w	d0
		beq.s	loc_82828
		movem.w	word_82832(pc,d0.w),d4-d5
		add.w	d4,d2
		add.w	d5,d3

loc_82828:
		move.w	d2,x_vel(a1)
		move.w	d3,y_vel(a1)
		rts
; End of function sub_82772

; ---------------------------------------------------------------------------
word_82832:	; neither A, B, nor C are pressed
		dc.w      0,     0	; no D-pad
		dc.w      0, -$300	; up
		dc.w      0,  $300	; down
		dc.w      0,     0	; up + down (shouldn't happen)
		dc.w  -$300,     0	; left
		dc.w  -$21F, -$21F	; left + up
		dc.w  -$21F,  $21F	; left + down
		dc.w      0,     0	; left + up + down (shouldn't happen)
		dc.w   $300,     0	; right
		dc.w   $21F, -$21F	; right + up
		dc.w   $21F,  $21F	; right + down
		dc.w      0,     0	; right + up + down (shouldn't happen)
		dc.w      0,     0	; left + right(shouldn't happen)
		dc.w      0,     0	; left + right + up (shouldn't happen)
		dc.w      0,     0	; left + right + down (shouldn't happen)
		dc.w      0,     0	; left + right + up + down (shouldn't happen)
word_82872:	; either A, B, or C is pressed
		dc.w   $600,     0	; no D-pad
		dc.w      0, -$600	; up
		dc.w      0,  $600	; down
		dc.w      0,     0	; up + down (shouldn't happen)
		dc.w  -$600,     0	; left
		dc.w  -$43E, -$43E	; left + up
		dc.w  -$43E,  $43E	; left + down
		dc.w      0,     0	; left + up + down (shouldn't happen)
		dc.w   $600,     0	; right
		dc.w   $43E, -$43E	; right + up
		dc.w   $43E,  $43E	; right + down
		dc.w      0,     0	; right + up + down (shouldn't happen)
		dc.w      0,     0	; left + right (shouldn't happen)
		dc.w      0,     0	; left + right + up (shouldn't happen)
		dc.w      0,     0	; left + right + down (shouldn't happen)
		dc.w      0,     0	; left + right + up + down (shouldn't happen)
"#,
        "4cbb000c0062314200183143001a4a40670a4cbb00300010d444d645334200183343001a4e75000000000000fd000000030000000000fd000000fde1fde1fde1021f0000000003000000021ffde1021f021f0000000000000000000000000000000000000000060000000000fa000000060000000000fa000000fbc2fbc2fbc2043e0000000006000000043efbc2043e043e0000000000000000000000000000000000000000",
    );
}

/// Probe `sk02`.
#[test]
fn s3k_extract_the_three_abcd_sites() {
    assert_asl(
        r#"	cpu 68000
x_vel = $18
y_vel = $1A
	phase $2DE6A
		abcd	-(a1),-(a2)
		abcd	-(a1),-(a2)
		abcd	-(a1),-(a2)
"#,
        "c509c509c509",
    );
}

/// Probe `sk03`.
#[test]
fn s3k_extract_subx_at_4d252() {
    assert_asl(
        r#"	cpu 68000
x_vel = $18
y_vel = $1A
	phase $4D252
		subx.w	d2,d0
"#,
        "9142",
    );
}

/// Probe `sk04`.
#[test]
fn s3k_extract_subx_at_5a036() {
    assert_asl(
        r#"	cpu 68000
x_vel = $18
y_vel = $1A
	phase $5A036
		subx.w	d0,d2
"#,
        "9540",
    );
}

/// Probe `sk05`.
#[test]
fn s3k_extract_subx_at_5a05a() {
    assert_asl(
        r#"	cpu 68000
x_vel = $18
y_vel = $1A
	phase $5A05A
		subx.w	d0,d2
"#,
        "9540",
    );
}
