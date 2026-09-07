//! Expression arithmetic REFUSES a 64-bit overflow, a division by zero and a
//! shift outside `0..=63`, instead of wrapping to a plausible value or folding
//! to a nameless `Poison` that a consumer reads as zero.
//!
//! ## What was silently wrong
//!
//! `sigil_ir::Expr::fold` is the one evaluator behind both the AS front end
//! (assembly time) and the linker (link time). Its arithmetic was `wrapping_*`
//! and a division by zero folded to `Fold::Poison`, the same value an
//! unresolved symbol folds to. Reproduced on the committed release binary:
//!
//! ```text
//!     move.w  #5/0,d0                  ->  303C 0000     exit 0
//!     dc.l    (1<<64)+5                ->  00000006      exit 0
//!     dc.l    ($100<<62)>>62           ->  00000000      exit 0
//!     dc.l    $7FFFFFFFFFFFFFFF*2      ->  FFFFFFFE      exit 0
//!     dc.l    5/0                      ->  "unresolved long expression", exit 1
//! ```
//!
//! The first is the worst shape: `fold_imm` reads a Poison with no dangling
//! symbol names as a 0 placeholder and records nothing, so the run passes.
//! The last is loud with the wrong sentence.
//!
//! The `.emp` comptime evaluator refuses every one of these (D-P2.1, `checked_*`
//! on i128); a link-time expression reached the wrapping evaluator instead.
//! `Fold::Fault` now carries the failed operation, every consumer reports it,
//! and the wide value never reaches a narrowing.
//!
//! ## The reference
//!
//! asl (reference build md5 `61e672562465725a8c102288a7da9098`, exit status
//! checked on every run) refuses `dc.l 5/0` with `error #1310: division by 0`,
//! exit 2. On the overflow rows it is silently wrong in the same way this
//! evaluator was: `(1<<64)+5` assembles to `00000006`, `($100<<62)>>62` to
//! `00000000` and `$7FFFFFFFFFFFFFFF*2` to `FFFFFFFE`, exit 0, which is C's
//! undefined signed overflow being observed, not a semantics. The controls
//! below are values asl folds without wrapping.

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

fn assert_refused(body: &str, line: u32, needles: &[&str]) {
    match assemble(body) {
        Ok(bytes) => panic!(
            "assembled with exit 0 to {:02X?} instead of refusing the arithmetic",
            &bytes[..bytes.len().min(8)]
        ),
        Err(diags) => {
            let hit = diags.iter().find(|(l, m)| *l == line && needles.iter().all(|n| m.contains(n)));
            assert!(hit.is_some(), "no diagnostic at line {line} naming {needles:?}; got {diags:?}");
        }
    }
}

const CPU: &str = "\tcpu 68000\n";

#[test]
fn division_by_zero_in_an_immediate_is_refused_not_a_zero_placeholder() {
    assert_refused(&format!("{CPU}\tmove.w #5/0,d0\n"), 2, &["division by zero", "5 / 0"]);
    assert_refused(&format!("{CPU}\tdc.l 5/0\n"), 2, &["division by zero", "5 / 0"]);
    assert_refused(&format!("{CPU}\tdc.b 7#0\n"), 2, &["division by zero", "7 # 0"]);
}

#[test]
fn a_shift_amount_outside_the_64_bit_value_is_refused() {
    assert_refused(&format!("{CPU}\tdc.l (1<<64)+5\n"), 2, &["shift amount 64 out of range 0..=63"]);
    assert_refused(&format!("{CPU}\tdc.l 1>>64\n"), 2, &["shift amount 64 out of range 0..=63"]);
    assert_refused(&format!("{CPU}\tdc.l 1<<-1\n"), 2, &["shift amount -1 out of range 0..=63"]);
}

#[test]
fn a_left_shift_that_loses_bits_is_refused_as_overflow() {
    assert_refused(&format!("{CPU}\tdc.l ($100<<62)>>62\n"), 2, &["arithmetic overflow", "256 << 62"]);
    assert_refused(&format!("{CPU}\tdc.l 1<<63\n"), 2, &["arithmetic overflow", "1 << 63"]);
}

#[test]
fn multiply_add_and_negate_overflow_are_refused() {
    assert_refused(
        &format!("{CPU}\tdc.l $7FFFFFFFFFFFFFFF*2\n"),
        2,
        &["arithmetic overflow", "9223372036854775807 * 2"],
    );
    assert_refused(
        &format!("{CPU}\tdc.l $7FFFFFFFFFFFFFFF+2\n"),
        2,
        &["arithmetic overflow", "9223372036854775807 + 2"],
    );
    assert_refused(
        &format!("{CPU}\tdc.l -$7FFFFFFFFFFFFFFF-2\n"),
        2,
        &["arithmetic overflow", "-9223372036854775807 - 2"],
    );
}

/// Values asl folds without wrapping keep their bytes: the whole 32-bit
/// space, a signed shift that fills with the sign, and a shift whose result
/// still round-trips.
#[test]
fn controls_fold_as_before() {
    assert_eq!(assemble(&format!("{CPU}\tdc.l 1<<31\n")).unwrap(), vec![0x80, 0x00, 0x00, 0x00]);
    assert_eq!(assemble(&format!("{CPU}\tdc.l $FFFF<<16\n")).unwrap(), vec![0xFF, 0xFF, 0x00, 0x00]);
    assert_eq!(assemble(&format!("{CPU}\tdc.l -1<<31\n")).unwrap(), vec![0x80, 0x00, 0x00, 0x00]);
    assert_eq!(assemble(&format!("{CPU}\tdc.l $7FFFFFFF+1\n")).unwrap(), vec![0x80, 0x00, 0x00, 0x00]);
    assert_eq!(assemble(&format!("{CPU}\tdc.l (-1)>>1\n")).unwrap(), vec![0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(assemble(&format!("{CPU}\tdc.l ($80000000<<1)>>1\n")).unwrap(), vec![0x80, 0x00, 0x00, 0x00]);
    assert_eq!(assemble(&format!("{CPU}\tdc.w 6*7\n")).unwrap(), vec![0x00, 0x2A]);
    assert_eq!(assemble(&format!("{CPU}\tdc.b -5#3\n")).unwrap(), vec![0xFE]);
}
