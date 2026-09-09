// A TAB in these quotations is the AS mnemonic COLUMN, and the column is
// load-bearing in this crate: an indented head is an instruction, a column-0
// head is a label. Spacing them to please a prose lint would falsify the
// quoted source and the quoted asl listing, so the lint is off HERE and the
// text stays verbatim.
#![allow(clippy::tabs_in_doc_comments)]

//! A user `function` call in a **Z80 instruction operand**.
//!
//! Two separable defects live at this one site, and the second is only reachable
//! once the first is fixed.
//!
//! 1. `lower_z80` handed its operands straight to `parse_operands`, with none of
//!    the `expand_operand_builtins` layer that `dc.b`/`dc.w`/`dc.l` and the
//!    68000 path both run. There is no call syntax in the expression parser, so
//!    `zmake68kPtr(SegaPCM)` parsed as the bare symbol `zmake68kPtr` with
//!    `(SegaPCM)` left over and was refused as `trailing tokens in operand`.
//! 2. `expand_calls` wraps every expansion in parentheses so the body binds
//!    tighter than whatever surrounds the call. On the 68000 that is invisible:
//!    an immediate there carries a `#`, which `classify` settles before it looks
//!    at parens. On the Z80 an operand that is one whole paren group is an
//!    INDIRECTION, so the expansion silently rewrote `ld de,f(x)` (3 bytes,
//!    `11 nn nn`) as `ld de,(nn)` (4 bytes, `ED 5B nn nn`), and rewrote
//!    `ld b,f(x)` into `ld b,(nn)`, which is not a Z80 instruction at all. The
//!    written shape, not the expanded one, decides the addressing mode.
//!
//! Sonic 1's `sound/z80.asm` lands on defect 1 four times (lines 51, 55, 188 and
//! 197) and on defect 2 twice (188 and 197). Those four instructions encode to
//! `3E 01`, `3E 0E`, `11 C0 8F` and `06 0B`, so 2 + 2 + 3 + 2 = **9 bytes**, which
//! is exactly the amount by which sigil's reported uncompressed driver size
//! (`1BBDh`) fell short of the reference assembler's (`1BC6h`). With defect 1
//! fixed and defect 2 live the figure is `1BC5h`: the 4-byte `ld de,(nn)` pays
//! back one of the two bytes that the unencodable `ld b,(nn)` loses.
//!
//! # Provenance of every expected value here
//!
//! Reference assembler `s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, invoked
//! `asl -xx -n -q -A -L -U -E -i .` and **checked for exit status 0** before any
//! byte was read out of the listing. Probe and verbatim listing are committed
//! under `docs/superpowers/notes/2026-09-09-as-z80-function-operand/`.
//!
//! # Why these particular values
//!
//! * `zmake68kBank(SegaPCM)` is `$1D` (29), so `&1` is `1` and `>>1` is `$0E`.
//!   Both halves are non-zero and they differ from each other, so neither a
//!   fold that dropped the trailing operator nor one that dropped the call can
//!   pass: a fixture whose two answers agreed would clear both.
//! * `zmake68kPtr(SegaPCM)` is `$8FC0`, which has a distinct high and low byte
//!   and is not `0`. A call that folded to nothing would read as `00 00`.
//! * The written indirections `(zmake68kPtr(SegaPCM))` are in the fixture
//!   BESIDE their bare twins, on the same mnemonics (`ld de`, `ld hl`). That
//!   pair is the only shape that can tell defect 2's fix from simply refusing
//!   to read any paren as an indirection: the two lines must encode
//!   DIFFERENTLY (`11`/`ED 5B`, `21`/`2A`) and a fix that over-reached would
//!   make them the same.
//! * `ld (ix+zmake68kBank(SegaPCM)),c` puts a call inside an index
//!   displacement, which reaches the paren branch with a call nested in it.
//! * `bit zmake68kBank(SegaPCM)&7,a` puts one in a bit-number operand, the
//!   Z80's other non-addressing value position.

use sigil_frontend_as::{assemble, Options};

/// Assemble a Z80 fixture at `org 0` and flatten it. Loud on failure: the
/// subject of this file is an operand sigil used to REFUSE, so a refusal must
/// name itself rather than surface as a length mismatch.
fn bytes(src: &str) -> Vec<u8> {
    let module = assemble(src, &Options::default()).unwrap_or_else(|d| {
        panic!(
            "expected a successful assembly, got {:?}",
            d.iter().map(|x| &x.message).collect::<Vec<_>>()
        )
    });
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    let image = sigil_link::flatten(&linked, 0x00).unwrap();
    assert!(
        !image.is_empty(),
        "the fixture emitted nothing, so no expectation below was actually tested"
    );
    image
}

/// The declarations Sonic 1's driver writes, verbatim from `sound/z80.asm:21`,
/// `:22` and `s1.sounddriver.asm:22`, `:23`, with `Z80_Clock` and `SegaPCM`
/// pinned to literals so the fixture stands alone.
const DECLS: &str = concat!(
    "\tcpu\tz80\n",
    "\torg\t0\n",
    "zROMWindow:\tequ\t8000h\n",
    "Z80_Clock:\tequ\t3579545\n",
    "SegaPCM:\tequ\t0E8FC0h\n",
    "zmake68kPtr  function addr,zROMWindow+(addr&7FFFh)\n",
    "zmake68kBank function addr,(((addr&0FF8000h)/zROMWindow))\n",
    "pcmLoopCounterBase function sampleRate,baseCycles,",
    " 1+(Z80_Clock/(sampleRate)-(baseCycles)+(13/2))/13\n",
    "pcmLoopCounter function sampleRate, pcmLoopCounterBase(sampleRate,90)\n",
);

/// The whole fixture, every Z80 operand position a `function` call can occupy.
///
/// asl (md5 above, exit 0),
/// `.../2026-09-09-as-z80-function-operand/z80_function_operand.lst`:
///
/// ```text
///      10/       0 : 3E 01               	ld	a,zmake68kBank(SegaPCM)&1
///      11/       2 : 3E 0E               	ld	a,zmake68kBank(SegaPCM)>>1
///      12/       4 : 11 C0 8F            	ld	de,zmake68kPtr(SegaPCM)
///      13/       7 : 06 0B               	ld	b,pcmLoopCounter(16000)
///      14/       9 : ED 5B C0 8F         	ld	de,(zmake68kPtr(SegaPCM))
///      15/       D : 3A C0 8F            	ld	a,(zmake68kPtr(SegaPCM))
///      16/      10 : DD 71 1D            	ld	(ix+zmake68kBank(SegaPCM)),c
///      17/      13 : 21 C0 8F            	ld	hl,zmake68kPtr(SegaPCM)
///      18/      16 : 2A C0 8F            	ld	hl,(zmake68kPtr(SegaPCM))
///      19/      19 : C3 C0 8F            	jp	zmake68kPtr(SegaPCM)
///      20/      1C : CD C0 8F            	call	zmake68kPtr(SegaPCM)
///      21/      1F : CB 6F               	bit	zmake68kBank(SegaPCM)&7,a
///      22/      21 : 00                  	nop
///      23/      22 : C9                  	ret
/// ```
#[test]
fn a_function_call_in_every_z80_operand_position() {
    let src = format!(
        "{DECLS}{}",
        concat!(
            "\tld\ta,zmake68kBank(SegaPCM)&1\n",
            "\tld\ta,zmake68kBank(SegaPCM)>>1\n",
            "\tld\tde,zmake68kPtr(SegaPCM)\n",
            "\tld\tb,pcmLoopCounter(16000)\n",
            "\tld\tde,(zmake68kPtr(SegaPCM))\n",
            "\tld\ta,(zmake68kPtr(SegaPCM))\n",
            "\tld\t(ix+zmake68kBank(SegaPCM)),c\n",
            "\tld\thl,zmake68kPtr(SegaPCM)\n",
            "\tld\thl,(zmake68kPtr(SegaPCM))\n",
            "\tjp\tzmake68kPtr(SegaPCM)\n",
            "\tcall\tzmake68kPtr(SegaPCM)\n",
            "\tbit\tzmake68kBank(SegaPCM)&7,a\n",
            "\tnop\n",
            "\tret\n",
        )
    );
    assert_eq!(
        bytes(&src),
        vec![
            0x3E, 0x01, // ld a,$1D&1
            0x3E, 0x0E, // ld a,$1D>>1
            0x11, 0xC0, 0x8F, // ld de,$8FC0       IMMEDIATE, 3 bytes
            0x06, 0x0B, // ld b,11
            0xED, 0x5B, 0xC0, 0x8F, // ld de,($8FC0)     INDIRECT, 4 bytes
            0x3A, 0xC0, 0x8F, // ld a,($8FC0)
            0xDD, 0x71, 0x1D, // ld (ix+$1D),c
            0x21, 0xC0, 0x8F, // ld hl,$8FC0       IMMEDIATE
            0x2A, 0xC0, 0x8F, // ld hl,($8FC0)     INDIRECT
            0xC3, 0xC0, 0x8F, // jp $8FC0
            0xCD, 0xC0, 0x8F, // call $8FC0
            0xCB, 0x6F, // bit 5,a
            0x00, // nop
            0xC9, // ret
        ]
    );
}

/// The four Sonic 1 lines on their own, and the arithmetic that ties them to the
/// driver size. Their nine bytes are the whole of the `1BBDh` versus `1BC6h` gap,
/// so this asserts the COUNT as well as the content: a future change that keeps
/// the bytes legal but moves their total moves the reported driver size with it.
#[test]
fn the_four_sonic_1_driver_lines_are_nine_bytes() {
    let src = format!(
        "{DECLS}{}",
        concat!(
            "\tld\ta,zmake68kBank(SegaPCM)&1\n",
            "\tld\ta,zmake68kBank(SegaPCM)>>1\n",
            "\tld\tde,zmake68kPtr(SegaPCM)\n",
            "\tld\tb,pcmLoopCounter(16000)\n",
        )
    );
    let image = bytes(&src);
    assert_eq!(
        image,
        vec![0x3E, 0x01, 0x3E, 0x0E, 0x11, 0xC0, 0x8F, 0x06, 0x0B]
    );
    assert_eq!(image.len(), 9, "0x1BC6 - 0x1BBD is 9");
}

/// A name that is both an ordinary symbol and a `function` is left as the symbol
/// when no `(` follows it, which is what keeps the expansion from reaching past
/// its own call.
#[test]
fn a_function_name_without_a_call_is_still_a_symbol() {
    let src = concat!(
        "\tcpu\tz80\n",
        "\torg\t0\n",
        "val:\tequ\t42h\n",
        "f    function x,x+1\n",
        "\tld\ta,val\n",
        "\tld\thl,val\n",
        "\tld\ta,(val)\n",
    );
    assert_eq!(
        bytes(src),
        vec![
            0x3E, 0x42, // ld a,$42
            0x21, 0x42, 0x00, // ld hl,$42
            0x3A, 0x42, 0x00, // ld a,($42)
        ]
    );
}
