//! A block opener with no closer is REFUSED, never read as closing on the last
//! line of the source.
//!
//! ## What was silently wrong
//!
//! ```text
//!     dc.b    $AA
//!     if      1
//!     dc.b    1,2,3,4
//!     dc.b    5           ; no endif anywhere
//! ```
//!
//! assembled to `AA 01 02 03 04`, exit 0, no diagnostic. `find_block_end` scans
//! for the closer and, finding none, answered with the index of the LAST LINE,
//! so that line was consumed as if it were the `endif` and its bytes vanished.
//! For a `macro` the same fallback swallowed every line after the head as the
//! macro body, so a file missing one `endm` assembled to whatever preceded the
//! definition. This is the class the AS replacement exists to remove: source
//! bytes dropped, exit 0.
//!
//! ## The reference
//!
//! asl (reference build md5 `61e672562465725a8c102288a7da9098`, exit status
//! checked on every run) refuses every one of these shapes with exit 2:
//!
//! ```text
//!     if / switch      INTERNAL: error #1470: missing ENDIF/ENDCASE
//!     rept             INTERNAL: error #1803: REPT without ENDM
//!     while            INTERNAL: error #1804: WHILE without ENDM
//!     irp / irpc       INTERNAL: error #1801: IRP without ENDM
//!     macro            INTERNAL: error #1800: open macro definition
//!     struct           INTERNAL: error #1551: open structure definition
//! ```
//!
//! An `END` directive inside the open block does not close it (asl still raises
//! the same error), while a block opened AFTER `END` is never read at all (exit
//! 0). Both are pinned below. asl names no line for these; sigil names the
//! opener's line, which is the line a reader has to find the partner for.

use sigil_frontend_as::{assemble_root_located, Options};

/// Assemble `body` as a real file: either the linked bytes, or every diagnostic
/// as `(1-based line, message)`.
fn assemble(body: &str) -> Result<Vec<u8>, Vec<(u32, String)>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok(sigil_link::flatten(&linked, 0x00))
        }
        Err(f) => Err(f
            .diags
            .iter()
            .map(|d| {
                let line = f
                    .sources
                    .label(d.primary)
                    .and_then(|l| {
                        l.rsplit_once('(')
                            .and_then(|(_, n)| n.trim_end_matches(')').parse().ok())
                    })
                    .unwrap_or(0);
                (line, d.message.clone())
            })
            .collect()),
    }
}

/// Assert that `body` is refused with a diagnostic AT `line` whose message names
/// every fragment in `names`. The bytes are printed on the failing side so the
/// pre-fix behaviour (a smaller image, exit 0) is visible in the red.
fn assert_refused_at(body: &str, line: u32, names: &[&str]) {
    match assemble(body) {
        Ok(bytes) => panic!(
            "assembled with exit 0 to {} byte(s) {:02X?} instead of refusing the unterminated block",
            bytes.len(),
            bytes
        ),
        Err(diags) => {
            let hit = diags
                .iter()
                .find(|(l, m)| *l == line && names.iter().all(|n| m.contains(n)));
            assert!(
                hit.is_some(),
                "expected a diagnostic at line {line} naming {names:?}; got {diags:?}"
            );
        }
    }
}

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\tdc.b $AA\n";

#[test]
fn an_unterminated_if_is_refused_not_closed_on_the_last_line() {
    // Line 4 opens; lines 5 and 6 are body; nothing closes. Pre-fix: `AA 01 02
    // 03 04`, the `dc.b 5` consumed as the closer.
    let src = format!("{HEAD}\tif 1\n\tdc.b 1,2,3,4\n\tdc.b 5\n");
    assert_refused_at(&src, 4, &["`if`", "`endif`", "never closed"]);
}

#[test]
fn a_terminated_if_still_assembles_every_line() {
    let src = format!("{HEAD}\tif 1\n\tdc.b 1,2,3,4\n\tdc.b 5\n\tendif\n");
    assert_eq!(assemble(&src).expect("assemble"), vec![0xAA, 1, 2, 3, 4, 5]);
}

#[test]
fn an_unterminated_rept_is_refused() {
    // Pre-fix: the body was line 5 alone (line 6 taken as the closer), so this
    // assembled to `AA 01 02 01 02`.
    let src = format!("{HEAD}\trept 2\n\tdc.b 1,2\n\tdc.b 5\n");
    assert_refused_at(&src, 4, &["`rept`", "`endr`", "never closed"]);
}

#[test]
fn a_terminated_rept_still_assembles_every_line() {
    let src = format!("{HEAD}\trept 2\n\tdc.b 1,2\n\tdc.b 5\n\tendr\n");
    assert_eq!(assemble(&src).expect("assemble"), vec![0xAA, 1, 2, 5, 1, 2, 5]);
}

#[test]
fn an_unterminated_macro_definition_is_refused() {
    // Pre-fix: everything after the head became the body of `m`, so the file
    // assembled to `AA` alone and said nothing.
    let src = format!("{HEAD}m\tmacro\n\tdc.b 1,2\n\tdc.b 5\n");
    assert_refused_at(&src, 4, &["`macro`", "`m`", "`endm`", "never closed"]);
}

#[test]
fn an_unterminated_while_is_refused() {
    let src = format!("{HEAD}i\tset 0\n\twhile i<2\n\tdc.b i\ni\tset i+1\n\tdc.b 5\n");
    assert_refused_at(&src, 5, &["`while`", "`endm`", "never closed"]);
}

#[test]
fn an_unterminated_irp_is_refused() {
    let src = format!("{HEAD}\tirp v,1,2\n\tdc.b v\n\tdc.b 5\n");
    assert_refused_at(&src, 4, &["`irp`", "`endm`", "never closed"]);
}

#[test]
fn an_unterminated_switch_is_refused() {
    let src = format!("{HEAD}\tswitch \"a\"\n\tcase \"a\"\n\tdc.b 1\n\tdc.b 5\n");
    assert_refused_at(&src, 4, &["`switch`", "`endcase`", "never closed"]);
}

#[test]
fn an_unterminated_struct_is_refused() {
    let src = format!("{HEAD}S\tstruct\nf\tds.b 2\n\tdc.b 5\n");
    assert_refused_at(&src, 4, &["`struct`", "`endstruct`", "never closed"]);
}

#[test]
fn the_outer_of_two_nested_ifs_is_the_one_reported() {
    // The inner `if` (line 5) is closed by the `endif` on line 7; the outer
    // (line 4) is not. asl reports the same shape as `missing ENDIF/ENDCASE`.
    let src = format!("{HEAD}\tif 1\n\tif 1\n\tdc.b 1\n\tendif\n\tdc.b 5\n");
    assert_refused_at(&src, 4, &["`if`", "`endif`", "never closed"]);
}

#[test]
fn an_end_directive_inside_an_open_if_does_not_close_it() {
    // asl: `INTERNAL: error #1470: missing ENDIF/ENDCASE`, exit 2.
    let src = format!("{HEAD}\tif 1\n\tdc.b 1\n\tend\n");
    assert_refused_at(&src, 4, &["`if`", "`endif`", "never closed"]);
}

#[test]
fn an_if_opened_after_end_is_never_read() {
    // asl assembles this to `AA 01` with exit 0: reading stops at `END`, so the
    // dangling `if` beneath it is not a block at all.
    let src = format!("{HEAD}\tif 1\n\tdc.b 1\n\tendif\n\tend\n\tif 1\n\tdc.b 2\n");
    assert_eq!(assemble(&src).expect("assemble"), vec![0xAA, 1]);
}
