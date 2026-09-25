//! `codepage`: AS's NAMED code pages, the selector above `charset`.
//!
//! `charset` edits a 256-entry character-to-byte table (see `as_charset.rs`).
//! `codepage NAME[,BASE]` chooses WHICH table: every `charset` edit, every
//! string in a data directive, every string in an expression and every
//! character constant goes through the page that is selected at that moment.
//! Sonic 3 & Knuckles builds its level-select font as a page called
//! `LEVELSELECT` and switches to it for the menu text and for the character
//! constants of the plane-map code (`sonic3k.macros.asm` 130-150,
//! `sonic3k.asm` 9951 and 10553-10569).
//!
//! An accepted-and-ignored `codepage` would emit that text in ASCII at exit 0,
//! the same silent class the string-escape defect was, so every rule below is
//! pinned with bytes that differ between the plausible readings.
//!
//! ## The rules, each measured
//!
//! - The default page is named `STANDARD`.
//! - A NEW page with no base starts as a copy of the page selected when it is
//!   created (not the identity, not `STANDARD`); with a base it starts as a
//!   copy of the base.
//! - Selecting an EXISTING page brings back its own contents; a base given
//!   then is looked up (an unknown one is refused) but copies nothing.
//! - `charset` edits, including the bare reset, touch the selected page only.
//! - Names are case sensitive, `STANDARD` included, are not symbols (a symbol
//!   of the same name coexists), and are plain text (`.loc` is one page under
//!   any parent label).
//! - `save`/`restore` DO bracket the selection (not the contents: see
//!   `as_charset.rs::save_and_restore_do_not_bracket_the_page`), nested.
//! - The selection leaks out of a macro body, as `charset` edits do.
//! - Every pass starts on `STANDARD` with no other page defined.
//!
//! ## Provenance
//!
//! Every expected byte string below is the hex of asl's own image, never
//! computed: `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
//! md5 `61e672562465725a8c102288a7da9098`, run through
//! `docs/superpowers/notes/asl-reference/asl_ref.sh`'s `asl_run -xx -n -q -A -L
//! -U -i .`, exit 0, then its `p2bin` with `-p=0`. The probe sources are the
//! `*_SRC` constants verbatim, and the table in
//! `docs/superpowers/notes/2026-09-25-s3k-codepage.md` lists every probe with
//! asl's and sigil's bytes. The refusal probes quote asl's diagnostic from a
//! run that exited 2; no byte of those runs is used.

use sigil_frontend_as::{assemble_root_located, Options};

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
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
        assert!(all.contains(n), "refusal does not name {n:?}: {d:?}");
    }
}

// ---------------------------------------------------------------------------
// Selecting pages.
// ---------------------------------------------------------------------------

/// STANDARD maps A and C; `PG1` is created from it, then gets B. Switching
/// changes the bytes, switching back restores each page's own mapping, and
/// the B edit never reaches STANDARD.
#[test]
fn switching_pages_changes_the_bytes_and_switching_back_restores_them() {
    assert_asl(CP01_SRC, CP01_ASL);
}

/// A new page with no base copies the SELECTED page, even when that is not
/// STANDARD (`PG2` made from `PG1` inherits B), and edits to the new page stay
/// on it.
#[test]
fn a_new_page_copies_the_selected_page() {
    assert_asl(CP05_SRC, CP05_ASL);
}

/// A new page with a base copies the base, whichever page is selected.
#[test]
fn a_new_page_with_a_base_copies_the_base() {
    assert_asl(CP02_SRC, CP02_ASL);
}

/// Reselecting an existing page with a base keeps the page's own contents:
/// the base is not copied again (STANDARD's later C edit does not appear).
#[test]
fn a_base_on_an_existing_page_copies_nothing() {
    assert_asl(CP09_SRC, CP09_ASL);
}

/// `charset` edits and the bare `charset` reset act on the selected page only.
#[test]
fn charset_edits_and_the_reset_touch_only_the_selected_page() {
    assert_asl(CP06_SRC, CP06_ASL);
}

/// Character constants and strings in expressions translate through the
/// selected page, not only strings in data directives.
#[test]
fn character_constants_and_expression_strings_use_the_selected_page() {
    assert_asl(CP08_SRC, CP08_ASL);
}

// ---------------------------------------------------------------------------
// Names.
// ---------------------------------------------------------------------------

/// Page names are case sensitive: `pg1` and `Pg1` are new pages, not `PG1`.
#[test]
fn page_names_are_case_sensitive() {
    assert_asl(CP03_SRC, CP03_ASL);
}

/// So is the default page's name: `standard` from `PG1` makes a new page that
/// copies `PG1`, and `STANDARD` is still the original.
#[test]
fn the_default_page_is_named_standard_exactly() {
    assert_asl(CP17_SRC, CP17_ASL);
}

/// A page name is not a symbol: `PG1 equ 5` and `codepage PG1` coexist.
#[test]
fn a_page_name_is_not_a_symbol() {
    assert_asl(CP18_SRC, CP18_ASL);
}

/// Dotted names and names starting with an underscore are page names.
#[test]
fn dotted_and_underscore_names_are_page_names() {
    assert_asl(CP23_SRC, CP23_ASL);
    assert_asl(CP24_SRC, CP24_ASL);
}

/// A `.loc` page name is plain text, not scoped under the last label.
#[test]
fn a_dot_name_is_not_scoped_by_the_label_above_it() {
    assert_asl(CP27_SRC, CP27_ASL);
}

/// A register spelling is an ordinary page name.
#[test]
fn a_register_spelling_is_a_page_name() {
    assert_asl(CP29_SRC, CP29_ASL);
}

/// A label on a `codepage` line is an ordinary address label.
#[test]
fn a_label_on_a_codepage_line_takes_the_address() {
    assert_asl(CP20_SRC, CP20_ASL);
}

// ---------------------------------------------------------------------------
// Scope.
// ---------------------------------------------------------------------------

/// `restore` reselects the page that was selected at `save`.
#[test]
fn restore_brings_back_the_selection() {
    assert_asl(CP04_SRC, CP04_ASL);
}

/// And nested: each `restore` reselects its own `save`'s page.
#[test]
fn nested_save_and_restore_bracket_each_selection() {
    assert_asl(CP19_SRC, CP19_ASL);
}

/// A selection made inside a macro body is still in force after it.
#[test]
fn a_selection_leaks_out_of_a_macro() {
    assert_asl(CP07_SRC, CP07_ASL);
}

/// A character constant passed as a macro argument translates through the
/// page selected INSIDE the body, where it is used.
#[test]
fn a_macro_argument_translates_where_it_is_used() {
    assert_asl(CP30_SRC, CP30_ASL);
}

/// Pages do not survive a pass: `PG1`'s pass-one edit is gone when pass two
/// reselects it, and STANDARD starts clean. asl: `2 passes`.
#[test]
fn pages_are_rebuilt_every_pass() {
    assert_asl(CP10_SRC, CP10_ASL);
}

/// And the selection does not survive a pass either: the file ends on `PG1`,
/// yet pass two's first `charset` lands on STANDARD. asl: `2 passes`.
#[test]
fn every_pass_starts_on_standard() {
    assert_asl(CP11_SRC, CP11_ASL);
}

// ---------------------------------------------------------------------------
// Sonic 3 & Knuckles.
// ---------------------------------------------------------------------------

/// The level-select page and text, extracted VERBATIM from skdisasm
/// `2fcd861c`: the `levselstr` macro and the `LEVELSELECT` block
/// (`sonic3k.macros.asm` 130-150), `make_art_tile` (114), the character
/// constant lines of the plane-map code (`sonic3k.asm` 9950-9995, branches and
/// loads dropped), and `LevelSelectText` (10552-10569). The three equates and
/// `planeLocH28` at the top stand in for definitions elsewhere in the source;
/// none of them touches a character. The `dc.b "*AZaz09:. "` lines around the
/// menu are STANDARD-page controls: they read as ASCII in asl's image, which
/// proves both `restore`s reselected STANDARD.
#[test]
fn the_sonic_3_level_select_text_matches_asl() {
    assert_asl(S3KLEVSEL_SRC, S3KLEVSEL_ASL);
}

// ---------------------------------------------------------------------------
// Refusals, each asl's own.
// ---------------------------------------------------------------------------

/// asl `cp12.asm(2): error #1110: wrong number of operands` for a bare
/// `codepage`, and the same for three operands (`cp13`).
#[test]
fn one_or_two_operands_only() {
    assert_refused(CP12_SRC, &["codepage", "operand"]);
    assert_refused(CP13_SRC, &["codepage", "operand"]);
}

/// asl `error #1610: unknown codepage` for a base that does not exist
/// (`cp14`), an empty base (`cp22`), a base naming the page being created
/// (`cp26`), and an unknown base on an EXISTING page (`cp28`).
#[test]
fn an_unknown_base_is_refused() {
    assert_refused(CP14_SRC, &["codepage", "unknown", "NOPE"]);
    assert_refused(CP22_SRC, &["codepage", "unknown"]);
    assert_refused(CP26_SRC, &["codepage", "unknown", "PG1"]);
    assert_refused(CP28_SRC, &["codepage", "unknown", "NOPE"]);
}

/// asl `error #1020: invalid symbol name` for a quoted name (`cp15`), a number
/// (`cp16`) and an expression (`cp21`).
#[test]
fn a_name_that_is_not_a_symbol_name_is_refused() {
    assert_refused(CP15_SRC, &["codepage", "name"]);
    assert_refused(CP16_SRC, &["codepage", "name"]);
    assert_refused(CP21_SRC, &["codepage", "name"]);
}

// ---------------------------------------------------------------------------
// The probe sources, verbatim, and asl's images as hex.
// ---------------------------------------------------------------------------

const CP01_SRC: &str = r#"	cpu 68000
	charset $41,$11
	charset $43,$33
	dc.b "ABC"
	codepage PG1
	dc.b "ABC"
	charset $42,$22
	dc.b "ABC"
	codepage STANDARD
	dc.b "ABC"
	codepage PG1
	dc.b "ABC"
"#;
const CP01_ASL: &str = "114233114233112233114233112233";
const CP02_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage PG1
	charset $42,$22
	codepage PG2,PG1
	dc.b "ABC"
	codepage PG3,STANDARD
	dc.b "ABC"
"#;
const CP02_ASL: &str = "112243114243";
const CP03_SRC: &str = r#"	cpu 68000
	codepage PG1
	charset $41,$11
	codepage STANDARD
	dc.b "AB"
	codepage pg1
	dc.b "AB"
	codepage Pg1
	dc.b "AB"
"#;
const CP03_ASL: &str = "414241424142";
const CP04_SRC: &str = r#"	cpu 68000
	charset $41,$11
	save
	codepage PG1
	charset $42,$22
	dc.b "AB"
	restore
	dc.b "AB"
"#;
const CP04_ASL: &str = "11221142";
const CP05_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage PG1
	charset $42,$22
	codepage PG2
	dc.b "ABC"
	charset $43,$33
	codepage PG1
	dc.b "ABC"
	codepage STANDARD
	dc.b "ABC"
	codepage PG2
	dc.b "ABC"
"#;
const CP05_ASL: &str = "112243112243114243112233";
const CP06_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage PG1
	charset $42,$22
	dc.b "AB"
	charset
	dc.b "AB"
	codepage STANDARD
	dc.b "AB"
"#;
const CP06_ASL: &str = "112241421142";
const CP07_SRC: &str = r#"	cpu 68000
sel	macro
	codepage PG1
	endm
	charset $41,$11
	codepage PG1
	charset $42,$22
	codepage STANDARD
	dc.b "AB"
	sel
	dc.b "AB"
"#;
const CP07_ASL: &str = "11421122";
const CP08_SRC: &str = r#"	cpu 68000
	codepage PG1
	charset $41,$11
	charset $42,$22
	dc.b 'A'
	move.w	#'B',d0
	move.w	#"AB",d1
	dc.w	'A'+1
	codepage STANDARD
	dc.b 'A'
	move.w	#'B',d0
"#;
const CP08_ASL: &str = "1100303c0022323c112200124100303c0042";
const CP09_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage PG1
	charset $42,$22
	codepage STANDARD
	charset $43,$33
	codepage PG1,STANDARD
	dc.b "ABC"
"#;
const CP09_ASL: &str = "112243";
const CP10_SRC: &str = r#"	cpu 68000
	dc.b "AB"
	codepage PG1
	dc.b "AB"
	codepage STANDARD
	dc.w	fwd
	codepage PG1
	charset $41,$11
	codepage STANDARD
	charset $42,$22
fwd:
	dc.b "AB"
"#;
const CP10_ASL: &str = "4142414200064122";
const CP11_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage STANDARD
	dc.b "AB"
	dc.w	fwd
	codepage PG1
	charset $42,$22
fwd:
	dc.b "AB"
"#;
const CP11_ASL: &str = "114200041122";
const CP17_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage PG1
	charset $42,$22
	codepage standard
	dc.b "AB"
	codepage STANDARD
	dc.b "AB"
"#;
const CP17_ASL: &str = "11221142";
const CP18_SRC: &str = r#"	cpu 68000
PG1	equ	5
	charset $41,$11
	codepage PG1
	charset $42,$22
	dc.b "AB",PG1
	codepage STANDARD
	dc.b "AB"
"#;
const CP18_ASL: &str = "1122051142";
const CP19_SRC: &str = r#"	cpu 68000
	charset $41,$11
	save
	codepage PG1
	charset $42,$22
	save
	codepage PG2
	charset $43,$33
	dc.b "ABC"
	restore
	dc.b "ABC"
	restore
	dc.b "ABC"
"#;
const CP19_ASL: &str = "112233112243114243";
const CP20_SRC: &str = r#"	cpu 68000
	charset $41,$11
lbl	codepage PG1
	charset $42,$22
	dc.b "AB"
	dc.w lbl
"#;
const CP20_ASL: &str = "11220000";
const CP23_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage A.B
	charset $42,$22
	codepage _x
	dc.b "AB"
	codepage A.B
	dc.b "AB"
"#;
const CP23_ASL: &str = "11221122";
const CP24_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage .loc
	dc.b "AB"
"#;
const CP24_ASL: &str = "1142";
const CP27_SRC: &str = r#"	cpu 68000
	charset $41,$11
first:
	codepage .loc
	charset $42,$22
	codepage STANDARD
second:
	codepage .loc
	dc.b "AB"
"#;
const CP27_ASL: &str = "1122";
const CP29_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage d0
	charset $42,$22
	dc.b "AB"
	codepage STANDARD
	dc.b "AB"
"#;
const CP29_ASL: &str = "11221142";
const CP30_SRC: &str = r#"	cpu 68000
	codepage PG1
	charset $41,$11
	charset $42,$22
	codepage STANDARD
chr	macro c,s
	save
	codepage PG1
	dc.b c,s
	restore
	dc.b c,s
	endm
	chr 'A',"AB"
	dc.b 'A',"AB"
"#;
const CP30_ASL: &str = "111122414142414142";
const S3KLEVSEL_SRC: &str = r#"	cpu 68000
tile_mask = $7FF
RAM_start = $FFFF0000
planeLocH28 function col,line,(line*$50)+(col*2)
make_art_tile function addr,pal,pri,((pri&1)<<15)|((pal&3)<<13)|(addr&tile_mask)
; macro for generating level select strings
levselstr macro str
	save
	codepage	LEVELSELECT
	dc.b strlen(str)-1, str
	restore
    endm

; codepage for level select
	save
	codepage LEVELSELECT
	charset '0','9', 16
	charset 'A','Z', 30
	charset 'a','z', 30
	charset '*', 26
	charset $A9, 27	; '?'
	charset ':', 28
	charset '.', 29
	charset ' ',  0
	restore

		save
		codepage	LEVELSELECT	; This is here so we can use '*' instead of '$1A'

	.blankloop:
		move.w	#make_art_tile(' ',0,0),(a2)+	; Full the remaining space with blank characters

	.stringfull:
		move.w	#make_art_tile('1',0,0),(a2)	; Write (act) '1'
		move.w	#make_art_tile('2',0,0),(a2)	; Write (act) '2'

		; Assuming the last line was the sound test...
		move.w	#make_art_tile(' ',0,0),(a2)	; Get rid of (act) '2'
		move.w	#make_art_tile('*',0,0),(a2)	; Replace that with '*'

		; Overwrite duplicate LAVA REEF 1/2 with 3/4 (obviously, S3 didn't do this)
		move.w	#make_art_tile('3',0,0),(RAM_start+planeLocH28($25,4)).l
		move.w	#make_art_tile('4',0,0),(RAM_start+planeLocH28($25,5)).l

		restore
	dc.b "*AZaz09:. "
LevelSelectText:
		levselstr "ANGEL ISLAND"
		levselstr "HYDROCITY"
		levselstr "MARBLE GARDEN"
		levselstr "CARNIVAL NIGHT"
		levselstr "ICECAP"
		levselstr "LAUNCH BASE"
		levselstr "MUSHROOM HILL"
		levselstr "FLYING BATTERY"
		levselstr "SANDOPOLIS"
		levselstr "LAVA REEF"
		levselstr "LAVA REEF"
		levselstr "SKY SANCTUARY"
		levselstr "DEATHEGG"
		levselstr "THE DOOMSDAY"
		levselstr "BONUS"
		levselstr "SPECIAL STAGE"
		levselstr "SOUND TEST  *"
	dc.b "*AZaz09:. "
"#;
const S3KLEVSEL_ASL: &str = "34fc000034bc001134bc001234bc000034bc001a33fc0013ffff018a33fc0014ffff01da2a415a617a30393a2e200b1e2b242229002630291e2b21082536212f2c202631360c2a1e2f1f292200241e2f21222b0d201e2f2b26331e29002b2624253105262022201e2d0a291e322b2025001f1e30220c2a3230252f2c2c2a00252629290d232936262b24001f1e3131222f3609301e2b212c2d2c29263008291e331e002f22222308291e331e002f2222230c30283600301e2b2031321e2f360721221e31252224240b31252200212c2c2a30211e36041f2c2b32300c302d2220261e290030311e24220c302c322b21003122303100001a2a415a617a30393a2e20";
const CP12_SRC: &str = r#"	cpu 68000
	codepage
	dc.b "A"
"#;
const CP13_SRC: &str = r#"	cpu 68000
	codepage PG1,STANDARD,PG2
	dc.b "A"
"#;
const CP14_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage PG2,NOPE
	dc.b "AB"
"#;
const CP15_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage "PG1"
	charset $42,$22
	codepage STANDARD
	dc.b "AB"
	codepage PG1
	dc.b "AB"
"#;
const CP16_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage 1
	dc.b "AB"
"#;
const CP21_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage PG1+1
	dc.b "AB"
"#;
const CP22_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage PG1,
	dc.b "AB"
"#;
const CP26_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage PG1,PG1
	dc.b "AB"
"#;
const CP28_SRC: &str = r#"	cpu 68000
	charset $41,$11
	codepage PG1
	codepage STANDARD,NOPE
	dc.b "AB"
"#;
