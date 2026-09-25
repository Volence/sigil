//! `'...'`: AS's character constant is a STRING, and a few consumers read a
//! single-quoted operand as an integer.
//!
//! Sonic 3 alone (`skdisasm` `buildS3.lua`) writes its region message as
//! `dc.b 6,'DEVELOPED FOR USE ONLY WITH',0`; asl emits one byte per character.
//! The rule, measured on the pinned asl:
//!
//! 1. `'...'` is a string exactly as `"..."` is. Every operator treats it as
//!    one: `'A'+'B'` concatenates, `'AB'+1` is the string `"AC"`, `-'AB'` and
//!    `'AB'-1` pack to integers, `strlen('ABC')` is 3, and in an integer slot
//!    (`move.l #'ABCD',d0`) it packs big-endian, 1 to 4 characters, like any
//!    string.
//! 2. A data directive writes a string one element per character at its
//!    width, EXCEPT a single-quoted operand whose string has at most `width`
//!    characters, which is one packed element: `dc.w 'AB'` is `4142`, `dc.w
//!    'ABC'` is `0041 0042 0043`, `dc.b ''` is `00`.
//! 3. "Single-quoted" is the operand's TEXT, after the parentheses enclosing it
//!    are stripped: it begins and ends with `'`. `dc.w 'A'+'B'` is `4142` and
//!    `dc.l 'A'+"B"+'C'` is `0041 4243`; `dc.w ('A')+'B'` and `dc.w 'A'+"B"`
//!    are `0041 0042`. A string symbol named bare carries the quoting of the
//!    operand that defined it.
//! 4. A single-quoted `charset` target is the integer (through the live page)
//!    whatever its length; any other string target is a table of raw bytes.
//! 5. The Z80 `dw` sign-extends a character element; the 68000 `dc.w`/`dc.l`
//!    zero-extend it.
//!
//! ## Provenance
//!
//! Every case below is `head + body + "\tend\n"`, and its `asl` field is the hex
//! of the pinned asl's own image of that exact source (md5
//! `61e672562465725a8c102288a7da9098`, through
//! `docs/superpowers/notes/asl-reference/asl_ref.sh`'s `asl_run -xx -n -q -A -L
//! -U -i .`, exit 0 with `ASL_DIAG=complete`, then its `p2bin -p=0`), or
//! `refused #N` with asl's error number from a run that exited non-zero, in
//! which case no byte of that run is used. The table was generated from those
//! runs, not typed; the probe table is in
//! `docs/superpowers/notes/2026-09-25-as-multichar-squote.md`.

use sigil_frontend_as::{assemble_root_located, Options};

const H68: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const HZ: &str = "\tcpu z80\n\torg 0\n";

struct Case {
    name: &'static str,
    head: &'static str,
    body: &'static str,
    asl: &'static str,
}

fn assemble(src: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, src).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok(sigil_link::flatten(&linked, 0x00).unwrap())
        }
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|b| format!("{b:02x}")).collect()
}

/// Every case, against asl's own answer. The failures are collected and named
/// together, so one run shows the whole shape of a regression.
#[test]
fn every_case_matches_asl() {
    let mut failures = Vec::new();
    for c in CASES {
        let src = format!("{}{}\tend\n", c.head, c.body);
        let got = assemble(&src);
        let refused_by_asl = c.asl.starts_with("refused");
        match (&got, refused_by_asl) {
            (Ok(b), false) if hex(b) == c.asl => {}
            (Err(_), true) => {}
            (Ok(b), _) => failures.push(format!("{}: asl {}, sigil {}", c.name, c.asl, hex(b))),
            (Err(d), false) => failures.push(format!("{}: asl {}, sigil refused {d:?}", c.name, c.asl)),
        }
    }
    assert!(failures.is_empty(), "{} of {} cases differ from asl:\n{}", failures.len(), CASES.len(), failures.join("\n"));
}

/// A refusal asl makes is made here in words that name it, not by a
/// neighbouring path that happens to fail.
#[test]
fn refusals_name_their_reason() {
    for (name, needle) in [
        ("r_imm_empty", "0-character string has no integer value"),
        ("r_imm_five", "5-character string has no integer value"),
        ("r_unterminated", "unterminated character constant"),
        ("r_bad_escape", "invalid escape sequence `\\q`"),
        ("r_charset_two", "out of range"),
        ("r_z80_ld_two", "out of range"),
        ("r_include", "include needs a double-quoted path"),
    ] {
        let c = CASES.iter().find(|c| c.name == name).expect("case exists");
        assert!(c.asl.starts_with("refused"), "{name}: asl accepts it");
        let d = assemble(&format!("{}{}\tend\n", c.head, c.body)).expect_err(name).join("\n");
        assert!(d.contains(needle), "{name}: refusal does not say {needle:?}: {d}");
    }
}

/// A single-quoted `charset` target with no packed value is refused by name.
///
/// asl reads it as an integer and does not refuse it: `charset $41,''` and
/// `charset $41,'BCDEF'` exit 0 and leave `dc.b "AB"` / `dc.b "ABCDE"` reading
/// `41 42` / `41 42 43 44 45`. What else they do to the page is not
/// established, so sigil refuses rather than calling them no-ops.
#[test]
fn a_single_quoted_charset_target_with_no_packed_value_is_refused() {
    for target in ["''", "'BCDEF'"] {
        let d = assemble(&format!("{H68}\tcharset $41,{target}\n\tend\n")).expect_err(target).join("\n");
        assert!(d.contains("single-quoted `charset` target must have 1 to 4 characters"), "{target}: {d}");
    }
}

const CASES: &[Case] = &[
    Case { name: "s3_region_block", head: H68, body: "MessageData:\nCountryCodes:\n\t\tdc.b\t'J',0,'UE'\n\nMsgPtrs:\n\t\tdc.b\t0,'J'\n\t\tdc.l\tMsgJapan-MessageData\n\n\t\tdc.b\t0,'U'\n\t\tdc.l\tMsgUSA-MessageData\n\n\t\tdc.b\t0,'E'\n\t\tdc.l\tMsgEurope-MessageData\n\n\t\tdc.w\t0\n\n;                               0123456789012345678901234567890123456789\nMsgDevelopedFor:\n\t\tdc.b\t6,           'DEVELOPED FOR USE ONLY WITH',0\nMsgAnd:\n\t\tdc.b\t18,                       '&',0\nMsgSystems:\n\t\tdc.b\t15,                   'SYSTEMS.',0\nMsgJapan:\n\t\tdc.b\t12,                'NTSC MEGA DRIVE',0\nMsgUSA:\n\t\tdc.b\t13,                 'NTSC GENESIS',0\nMsgEurope:\n\t\tdc.b\t4,         'PAL AND FRENCH SECAM MEGA DRIVE',0\n", asl: "4a005545004a00000042005500000053004500000061000006444556454c4f50454420464f5220555345204f4e4c592057495448001226000f53595354454d532e000c4e545343204d454741204452495645000d4e5453432047454e45534953000450414c20414e44204652454e434820534543414d204d45474120445249564500" },
    Case { name: "b_five", head: H68, body: "\tdc.b 'ABCDE'\n", asl: "4142434445" },
    Case { name: "b_list", head: H68, body: "\tdc.b 'J',0,'UE'\n\tdc.b 6,'ABC',0\n\tdc.b 'Q'\n", asl: "4a005545064142430051" },
    Case { name: "b_plus_int", head: H68, body: "\tdc.b 'AB'+1\n", asl: "4143" },
    Case { name: "b_int_plus", head: H68, body: "\tdc.b 1+'AB'\n", asl: "4143" },
    Case { name: "b_plus_carry", head: H68, body: "\tdc.b 'AB'+$100\n", asl: "4242" },
    Case { name: "b_concat", head: H68, body: "\tdc.b 'A'+'B'\n", asl: "4142" },
    Case { name: "b_mixed_quotes", head: H68, body: "\tdc.b 'AB',\"CD\",'E'\n", asl: "4142434445" },
    Case { name: "b_empty", head: H68, body: "\tdc.b ''\n", asl: "00" },
    Case { name: "b_empty_then_char", head: H68, body: "\tdc.b '','A'\n", asl: "0041" },
    Case { name: "b_semicolon_comma", head: H68, body: "\tdc.b 'A;B','A,B',1\n", asl: "413b42412c4201" },
    Case { name: "b_escapes", head: H68, body: "\tdc.b 'A\\x42C','A\\'B','A\"B'\n", asl: "414243412742412242" },
    Case { name: "b_interpolation", head: H68, body: "\tdc.b 'A\\{1+1}B'\n", asl: "413242" },
    Case { name: "b_sum_zero_is_empty", head: H68, body: "\tdc.b 'AB'+(0-$4142),$EE\n", asl: "ee" },
    Case { name: "b_dup", head: H68, body: "\tdc.b [2]'AB'\n", asl: "41424142" },
    Case { name: "b_rept", head: H68, body: "\trept 2\n\tdc.b 'AB'\n\tendm\n", asl: "41424142" },
    Case { name: "b_macro_arg", head: H68, body: "m macro a\n\tdc.b a\n\tendm\n\tm 'ABC'\n", asl: "414243" },
    Case { name: "b_strlen", head: H68, body: "\tdc.b strlen('ABC')\n", asl: "03" },
    Case { name: "w_fits", head: H68, body: "\tdc.w 'AB'\n", asl: "4142" },
    Case { name: "w_one", head: H68, body: "\tdc.w 'A'\n", asl: "0041" },
    Case { name: "w_three", head: H68, body: "\tdc.w 'ABC'\n", asl: "004100420043" },
    Case { name: "w_empty", head: H68, body: "\tdc.w ''\n", asl: "0000" },
    Case { name: "w_plus_int", head: H68, body: "\tdc.w 'AB'+1\n", asl: "00410043" },
    Case { name: "w_plus_int_zero", head: H68, body: "\tdc.w 'AB'+(1-1)\n", asl: "00410042" },
    Case { name: "w_minus_int", head: H68, body: "\tdc.w 'AB'-1\n", asl: "4141" },
    Case { name: "w_concat", head: H68, body: "\tdc.w 'A'+'B'\n", asl: "4142" },
    Case { name: "w_concat_parens", head: H68, body: "\tdc.w ('A'+'B')\n", asl: "4142" },
    Case { name: "w_paren_head", head: H68, body: "\tdc.w ('A')+'B'\n", asl: "00410042" },
    Case { name: "w_paren_tail", head: H68, body: "\tdc.w 'A'+('B')\n", asl: "00410042" },
    Case { name: "w_double_tail", head: H68, body: "\tdc.w 'A'+\"B\"\n", asl: "00410042" },
    Case { name: "w_parens", head: H68, body: "\tdc.w ('AB')\n", asl: "4142" },
    Case { name: "w_char_plus", head: H68, body: "\tdc.w 'A'+$100\n", asl: "00010041" },
    Case { name: "w_high_char", head: H68, body: "\tdc.w 'A\\x99\\x41'\n", asl: "004100990041" },
    Case { name: "w_substr", head: H68, body: "\tdc.w substr('ABC',0,2)\n", asl: "00410042" },
    Case { name: "w_dup", head: H68, body: "\tdc.w [2]'ABC'\n", asl: "004100420043004100420043" },
    Case { name: "w_list", head: H68, body: "\tdc.w \"A\",'BCD',$1234\n", asl: "00410042004300441234" },
    Case { name: "l_three", head: H68, body: "\tdc.l 'ABC'\n", asl: "00414243" },
    Case { name: "l_five", head: H68, body: "\tdc.l 'ABCDE'\n", asl: "0000004100000042000000430000004400000045" },
    Case { name: "l_ends_quoted", head: H68, body: "\tdc.l 'A'+\"B\"+'C'\n", asl: "00414243" },
    Case { name: "l_ends_quoted_int", head: H68, body: "\tdc.l 'A'+1+'C'\n", asl: "00004243" },
    Case { name: "l_ends_quoted_paren", head: H68, body: "\tdc.l 'A'+('B')+'C'\n", asl: "00414243" },
    Case { name: "l_ends_quoted_call", head: H68, body: "\tdc.l 'A'+substr(\"BC\",0,1)+'D'\n", asl: "00414244" },
    Case { name: "l_group_head", head: H68, body: "\tdc.l ('A'+'B')+'C'\n", asl: "000000410000004200000043" },
    Case { name: "l_minus", head: H68, body: "\tdc.l 'ABCD'-1\n", asl: "41424343" },
    Case { name: "sym_equ", head: H68, body: "X equ 'AB'\n\tdc.w X\n\tdc.b X\n\tdc.w (X)\n", asl: "414241424142" },
    Case { name: "sym_equ_long", head: H68, body: "X equ 'ABC'\n\tdc.w X\n", asl: "004100420043" },
    Case { name: "sym_equ_concat", head: H68, body: "X equ 'A'+'B'\n\tdc.w X\n", asl: "4142" },
    Case { name: "sym_equ_plus_int", head: H68, body: "X equ 'AB'+1\n\tdc.w X\n", asl: "00410043" },
    Case { name: "sym_in_concat", head: H68, body: "X equ 'A'\n\tdc.w X+'B'\n", asl: "00410042" },
    Case { name: "sym_copy", head: H68, body: "X equ 'AB'\nY equ (X)\n\tdc.w Y\n", asl: "4142" },
    Case { name: "sym_set", head: H68, body: "X := 'AB'\n\tdc.w X\n\tmove.l #X,d0\n", asl: "4142203c00004142" },
    Case { name: "sym_double", head: H68, body: "X equ \"AB\"\n\tdc.w X\n", asl: "00410042" },
    Case { name: "cs_bytes", head: H68, body: "\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\n\tdc.b 'ABC'\n\tdc.b 'CBA',0,'B'\n", asl: "1122999922110022" },
    Case { name: "cs_word", head: H68, body: "\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\n\tdc.w 'AB'\n\tdc.w 'ABC'\n", asl: "1122001100220099" },
    Case { name: "cs_long_imm", head: H68, body: "\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\n\tdc.l 'CAB'\n\tmove.l #'CAB',d0\n", asl: "00991122203c00991122" },
    Case { name: "cs_concat", head: H68, body: "\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\n\tdc.w 'A'+'B'\n\tdc.b 'AB'+'C'\n", asl: "1122112299" },
    Case { name: "cs_plus_int_dropped", head: H68, body: "\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\n\tdc.b 'AB'+1\n", asl: "11" },
    Case { name: "cs_plus_int_kept", head: H68, body: "\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\n\tdc.b 'AB'+2\n", asl: "1124" },
    Case { name: "cs_symbol_before_page", head: H68, body: "X equ 'CA'\n\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\n\tdc.b X\n\tdc.w X\n", asl: "99119911" },
    Case { name: "cs_symbol_after_reset", head: H68, body: "\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\nX equ 'CA'\n\tcharset\n\tdc.b X\n\tdc.w X\n", asl: "43414341" },
    Case { name: "cs_escape", head: H68, body: "\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\n\tdc.b 'A\\x42C'\n", asl: "112299" },
    Case { name: "cs_target_char", head: H68, body: "\tcharset 'B',$77\n\tcharset 'A','B'\n\tcharset 'C',('B')\n\tdc.b \"AC\"\n", asl: "7777" },
    Case { name: "cs_target_string", head: H68, body: "\tcharset 'B',$77\n\tcharset 'A','B'+0\n\tdc.b \"A\"\n", asl: "42" },
    Case { name: "cs_range_target", head: H68, body: "\tcharset 'A',$99\n\tcharset 'a','c','A'\n\tdc.b \"abc\"\n", asl: "999a9b" },
    Case { name: "z_db", head: HZ, body: "\tdb 'AB',0,'C'\n\tdb 'AB'+1\n\tdb ''\n", asl: "41420043414300" },
    Case { name: "z_dw", head: HZ, body: "\tdw 'AB'\n\tdw 'A'\n\tdw ''\n\tdw 'ABC'\n", asl: "424141000000410042004300" },
    Case { name: "z_dw_plus", head: HZ, body: "\tdw 'AB'+1\n\tdw 'A'+80h\n", asl: "41004300c1ff" },
    Case { name: "z_dw_sign", head: HZ, body: "\tdw \"\\x99\\x41\"\n\tdw \"\\x7f\\x80\"\n", asl: "99ff41007f0080ff" },
    Case { name: "z_dw_page", head: HZ, body: "\tcharset 'A',11h\n\tcharset 'B',22h\n\tcharset 'C',99h\n\tdw 'CAB'\n\tdw 'CA'\n\tdb 'CAB'\n\tld hl,'BC'\n", asl: "99ff110022001199991122219922" },
    Case { name: "z_ld", head: HZ, body: "\tld hl,'AB'\n\tld a,'A'\n\tcp 'A'\n", asl: "2142413e41fe41" },
    Case { name: "imm", head: H68, body: "\tmove.l #'ABCD',d0\n\tmove.w #'A'+'B',d0\n\tmove.w #'AB'+1,d0\n", asl: "203c41424344303c4142303c4143" },
    Case { name: "fn_substitutes", head: H68, body: "f function a,'a'\n\tdc.b f(66)\n", asl: "28363629" },
    Case { name: "switch_case", head: H68, body: "\tswitch 'A'\n\tcase 65\n\tdc.b 1\n\tcase \"A\"\n\tdc.b 2\n\telsecase\n\tdc.b 3\n\tendcase\n", asl: "02" },
    Case { name: "if_compare", head: H68, body: "\tif 65='A'\n\tdc.b 1\n\telse\n\tdc.b 2\n\tendif\n\tif 'AB'<>'AC'\n\tdc.b 3\n\tendif\n", asl: "0103" },
    Case { name: "r_imm_empty", head: H68, body: "\tmove.w #'',d0\n", asl: "refused #1141" },
    Case { name: "r_imm_five", head: H68, body: "\tmove.l #'ABCDE',d0\n", asl: "refused #1141" },
    Case { name: "r_doubled_quote", head: H68, body: "\tdc.b 'A''B'\n", asl: "refused #1020" },
    Case { name: "r_unterminated", head: H68, body: "\tdc.b 'AB\n", asl: "refused #1020" },
    Case { name: "r_bad_escape", head: H68, body: "\tdc.b '\\q'\n", asl: "refused #2010" },
    Case { name: "r_charset_two", head: H68, body: "\tcharset $41,'BC'\n", asl: "refused #1320" },
    Case { name: "r_z80_ld_two", head: HZ, body: "\tld a,'AB'\n", asl: "refused #1320" },
    Case { name: "r_include", head: H68, body: "\tinclude 'p.inc'\n", asl: "refused #10001" },
];
