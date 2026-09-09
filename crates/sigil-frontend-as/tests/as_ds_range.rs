//! `ds.b`/`ds.w`/`ds.l` REFUSE a count the reservation cannot hold, instead of
//! narrowing it to `u32` after a sign check made on the wide `i64`.
//!
//! ## What was silently wrong
//!
//! The directive tested `v >= 0` on the folded `i64` and reserved
//! `v as u32 * unit`. Two counts pass the wide test and reserve nothing:
//!
//! ```text
//!     ds.b $100000000     ->  v as u32 == 0: image `11 22`, exit 0
//!     ds.w $80000000      ->  2^31 * 2 wraps to 0: image `11 11 22`, exit 0
//! ```
//!
//! ## The reference
//!
//! asl (reference build md5 `61e672562465725a8c102288a7da9098`, exit status
//! checked on every run) refuses `ds.b $100000000` with `error #1320: range
//! overflow`, exit 2: its count is a 32-bit window. It is itself silent on the
//! other rows: `ds.w $80000000` assembles with exit 0 and the PC advances by
//! ZERO (listing `3/ 2 : ds.w $80000000` then `4/ 2 : 22`), `ds.b -1` moves
//! the PC BACKWARDS to 0, and `ds.b $FFFFFFFF` wraps it to 0. Those three are
//! asl being wrong, not a verdict to match; every one is refused here.

use sigil_frontend_as::{assemble_root_located, Options};

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
            Ok(sigil_link::flatten(&linked, 0x00).expect("flatten"))
        }
        Err(f) => Err(f
            .diags
            .iter()
            .map(|d| {
                let line = f
                    .sources
                    .label(d.primary)
                    .and_then(|l| {
                        // `file(line):col`: the line is what sits BETWEEN the
                        // parens. Reading to the end of the string instead used
                        // to work only because nothing followed the `)`, and it
                        // returned 0 (a line no source has) the moment the
                        // column arrived.
                        l.rsplit_once('(')
                            .and_then(|(_, rest)| rest.split_once(')'))
                            .and_then(|(n, _)| n.parse().ok())
                    })
                    .unwrap_or(0);
                (line, d.message.clone())
            })
            .collect()),
    }
}

/// Refused at `line`, with a message carrying every needle. The bytes are
/// printed on the failing side so a silent pre-fix reservation is visible.
fn assert_refused(body: &str, line: u32, needles: &[&str]) {
    match assemble(body) {
        Ok(bytes) => panic!(
            "assembled with exit 0 to {} byte(s) {:02X?} instead of refusing the ds count",
            bytes.len(),
            &bytes[..bytes.len().min(8)]
        ),
        Err(diags) => {
            let hit = diags
                .iter()
                .find(|(l, m)| *l == line && needles.iter().all(|n| m.contains(n)));
            assert!(hit.is_some(), "no diagnostic at line {line} naming {needles:?}; got {diags:?}");
        }
    }
}

const DS_WINDOW: &str = "0..=4294967295";

#[test]
fn ds_b_two_to_the_32_is_refused_not_a_zero_reservation() {
    assert_refused(
        "\tcpu 68000\n\tdc.b $11\n\tds.b $100000000\n\tdc.b $22\n",
        3,
        &["ds count", "4294967296", DS_WINDOW],
    );
}

/// The count fits `u32`; the BYTE total (`count * unit`) does not.
#[test]
fn ds_w_two_to_the_31_is_refused_where_the_byte_total_overflows() {
    assert_refused(
        "\tcpu 68000\n\tdc.w $1111\n\tds.w $80000000\n\tdc.b $22\n",
        3,
        &["ds.w", "2147483648", "4294967296 bytes"],
    );
    assert_refused(
        "\tcpu 68000\n\tds.l $40000000\n\tdc.b $22\n",
        2,
        &["ds.l", "1073741824", "4294967296 bytes"],
    );
}

/// The count and total fit; the reservation would run the section cursor off
/// the end of the 32-bit address space (asl wraps its PC to 0 here).
#[test]
fn ds_b_ffffffff_at_offset_one_is_refused_where_the_cursor_would_wrap() {
    assert_refused(
        "\tcpu 68000\n\tdc.b $11\n\tds.b $FFFFFFFF\n",
        3,
        &["ds.b", "4294967295 bytes", "32-bit address space"],
    );
}

#[test]
fn ds_negative_is_refused_with_the_window() {
    assert_refused("\tcpu 68000\n\tdc.b $11\n\tds.b -1\n\tdc.b $22\n", 3, &["ds count", "-1", DS_WINDOW]);
}

#[test]
fn ds_controls_still_reserve_exactly_their_bytes() {
    assert_eq!(
        assemble("\tcpu 68000\n\tdc.b $11\n\tds.b 3\n\tdc.b $22\n").expect("ds.b 3"),
        vec![0x11, 0, 0, 0, 0x22]
    );
    assert_eq!(
        assemble("\tcpu 68000\n\tdc.w $1111\n\tds.w 2\n\tdc.b $22\n").expect("ds.w 2"),
        vec![0x11, 0x11, 0, 0, 0, 0, 0x22]
    );
    // A zero count is inside the window and reserves nothing (at an even PC,
    // so the word-or-larger even-pad does not enter).
    assert_eq!(
        assemble("\tcpu 68000\n\tdc.w $1111\n\tds.l 0\n\tdc.b $22\n").expect("ds.l 0"),
        vec![0x11, 0x11, 0x22]
    );
}

/// The largest reservation the cursor can take from offset 0: the byte total
/// is `u32::MAX` exactly and the count check accepts it. Only the front end
/// runs, since materialising a 4 GiB image is not the point.
#[test]
fn ds_b_ffffffff_at_offset_zero_is_accepted_by_every_check() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, "\tcpu 68000\n\tds.b $FFFFFFFF\n").expect("write probe");
    let m = assemble_root_located(&path, &Options::default());
    assert!(m.is_ok(), "the top of the window is refused: {:?}", m.err().map(|f| f.diags));
}
