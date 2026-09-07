//! Macro expansion is budgeted in BREADTH as well as depth: the total number of
//! expansions in one pass is capped, so a macro that calls itself more than
//! once per body is refused, naming the macro, in bounded time.
//!
//! ## What was silently wrong
//!
//! ```text
//! m   macro
//!     m
//!     m
//!     endm
//!     m
//! ```
//!
//! The depth cap (64) bounds how DEEP the expansion tree grows, and each leaf
//! at that depth is diagnosed and returned from. Nothing bounded how WIDE it
//! grows: two calls per body is 2^64 leaves, so the run never finished and the
//! only observable outcome was a timeout. The file already argues this exact
//! multiplication for nested `while` (`GLOBAL_WHILE_CAP`); this carries the
//! same per-pass budget to expansion.
//!
//! ## The reference
//!
//! asl (reference build md5 `61e672562465725a8c102288a7da9098`, exit status
//! checked) is itself silently wrong here: on the source above it ran past a
//! 30-second timeout, printing an ever-growing `m_breadth.asm(7) m(1) m(1) ...`
//! expansion trace and never a verdict. Refusing is therefore stricter than the
//! reference, deliberately. The linear shape asl does accept is pinned as the
//! control: `cnt macro n / if n>0 / dc.b n / cnt n-1 / endif / endm / cnt 5`
//! assembles to `05 04 03 02 01`, exit 0.

use sigil_frontend_as::{assemble, Options};
use std::time::Duration;

/// The bound a run must finish inside. The pre-fix code does not finish inside
/// it; that timeout IS the red.
const BOUND: Duration = Duration::from_secs(60);

fn assemble_within_bound(src: &'static str) -> Result<Vec<u8>, Vec<String>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let r = assemble(src, &Options::default())
            .map(|m| {
                m.sections
                    .first()
                    .map(|s| s.image_bytes())
                    .unwrap_or_default()
            })
            .map_err(|ds| ds.into_iter().map(|d| d.message).collect::<Vec<_>>());
        let _ = tx.send(r);
    });
    match rx.recv_timeout(BOUND) {
        Ok(r) => r,
        Err(_) => panic!("assembly did not terminate within {BOUND:?}"),
    }
}

/// The diagnostics of a run that must be refused; an accepted run fails naming
/// only the image size.
fn refusal(src: &'static str) -> Vec<String> {
    match assemble_within_bound(src) {
        Ok(bytes) => panic!(
            "assembled with exit 0 to {} byte(s) instead of refusing",
            bytes.len()
        ),
        Err(diags) => diags,
    }
}

#[test]
fn a_macro_calling_itself_twice_is_refused_in_bounded_time() {
    let src = "\tcpu 68000\n\tpadding off\nm\tmacro\n\tm\n\tm\n\tendm\n\tm\n\tdc.b 1\n";
    let err = refusal(src);
    assert!(
        err.iter()
            .any(|m| m.contains("macro `m`") && m.contains("budget")),
        "expected a macro expansion budget diagnostic naming `m`; got {err:?}"
    );
}

#[test]
fn linear_self_recursion_assembles_as_asl() {
    // asl: `05 04 03 02 01`.
    let src = "\tcpu 68000\n\tpadding off\ncnt\tmacro n\n\tif n>0\n\tdc.b n\n\tcnt n-1\n\tendif\n\tendm\n\tcnt 5\n";
    assert_eq!(
        assemble_within_bound(src).expect("assemble"),
        vec![5, 4, 3, 2, 1]
    );
}

#[test]
fn many_independent_expansions_stay_within_the_budget() {
    // Fifty thousand flat expansions: breadth without recursion, well inside
    // the budget, every one emitting its byte.
    let src = "\tcpu 68000\n\tpadding off\nm\tmacro\n\tdc.b 1\n\tendm\n\trept 50000\n\tm\n\tendr\n";
    assert_eq!(
        assemble_within_bound(src).expect("assemble"),
        vec![1u8; 50000]
    );
}
