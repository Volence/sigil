//! `align n` REFUSES a count the pad function cannot take, instead of
//! narrowing it to `u32` after a check made on the wide `i64`.
//!
//! ## What was silently wrong
//!
//! The directive tested `n > 0` on the folded `i64` and then handed `n as u32`
//! to `asl_align_pad`. Two counts that pass the wide test change meaning in
//! the narrow cast:
//!
//! ```text
//!     align $100000000     ->  n as u32 == 0: `wrapping_rem(0)`, a panic
//!                              ("attempt to calculate the remainder with a
//!                              divisor of zero", exit 101) in the release
//!                              binary
//!     align $100000001     ->  n as u32 == 1: no pad at all, `11 22`, exit 0
//! ```
//!
//! ## The reference
//!
//! asl (reference build md5 `61e672562465725a8c102288a7da9098`, exit status
//! checked on every run) refuses all of `align $100000000`, `align $100000001`
//! and `align $10000` with `error #1320: range overflow`, exit 2: its count is
//! a 16-bit word. `align $FFFF` assembles (exit 0, the next byte lands at
//! `$FFFF`). The accepted window here is `1..=$FFFFFFFF`, the domain of
//! `asl_align_pad`, so `align $10000` (a 64 KiB boundary) stays accepted where
//! asl's word type refuses it; the two refusals above are the ones that matter.

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
                        l.rsplit_once('(')
                            .and_then(|(_, n)| n.trim_end_matches(')').parse().ok())
                    })
                    .unwrap_or(0);
                (line, d.message.clone())
            })
            .collect()),
    }
}

/// Refused at `line`, with a message carrying every needle. The bytes are
/// printed on the failing side so a silent pre-fix pad is visible.
fn assert_refused(body: &str, line: u32, needles: &[&str]) {
    match assemble(body) {
        Ok(bytes) => panic!(
            "assembled with exit 0 to {} byte(s) {:02X?} instead of refusing the align count",
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

const ALIGN_WINDOW: &str = "1..=4294967295";

#[test]
fn align_two_to_the_32_is_refused_not_a_panic() {
    assert_refused(
        "\tcpu 68000\n\tdc.b $11\n\talign $100000000\n\tdc.b $22\n",
        3,
        &["align", "4294967296", ALIGN_WINDOW],
    );
}

#[test]
fn align_two_to_the_32_plus_one_is_refused_not_a_no_op() {
    assert_refused(
        "\tcpu 68000\n\tdc.b $11\n\talign $100000001\n\tdc.b $22\n",
        3,
        &["align", "4294967297", ALIGN_WINDOW],
    );
}

#[test]
fn align_zero_and_negative_are_refused_with_the_same_window() {
    assert_refused("\tcpu 68000\n\tdc.b $11\n\talign 0\n\tdc.b $22\n", 3, &["align", "0", ALIGN_WINDOW]);
    assert_refused("\tcpu 68000\n\tdc.b $11\n\talign -256\n\tdc.b $22\n", 3, &["align", "-256", ALIGN_WINDOW]);
}

/// The largest count asl accepts still assembles, and pads to exactly asl's
/// address (`$FFFF`, so the second byte is the 65536th).
#[test]
fn align_ffff_is_the_asl_control_and_still_assembles() {
    let bytes = assemble("\tcpu 68000\n\tdc.b $11\n\talign $FFFF\n\tdc.b $22\n").expect("align $FFFF assembles");
    assert_eq!(bytes.len(), 0x10000);
    assert_eq!(bytes[0], 0x11);
    assert_eq!(bytes[0xFFFF], 0x22);
}

/// The top of the accepted window: a count of `$FFFFFFFF` at PC 1 is a pad the
/// function defines (it wraps to zero, the address-space top), and the run does
/// not panic. Only the count check is under test, so the byte image is not
/// materialised.
#[test]
fn align_ffffffff_is_accepted_by_the_count_check() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, "\tcpu 68000\n\tdc.b $11\n\talign $FFFFFFFF\n").expect("write probe");
    let m = assemble_root_located(&path, &Options::default());
    assert!(m.is_ok(), "the top of the window is refused: {:?}", m.err().map(|f| f.diags));
}
