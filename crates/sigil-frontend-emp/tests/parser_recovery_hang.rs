//! t25 (sound_debug lane) — a parser ROBUSTNESS bug, captured as a documented
//! repro. `parse_str` HANGS (infinite error-recovery loop, never terminates) on
//! the input below instead of emitting a clean parse error. A front-end must
//! error loudly, never spin — so this is a bug independent of the missing
//! feature that triggers it.
//!
//! TRIGGER: `lea (extern("X") - CONST)(a0), a0` — an `extern(...)` inside a
//! displacement expression is an UNSUPPORTED form (a parse gap; ledgered as a
//! demanded feature). In ISOLATION the parser reports "expected end of line" and
//! recovers cleanly. But in the ACCUMULATED context below (a `{stop_z80()}` Code
//! splice opening the body + a nested `.copy3c`/`.copy3cb` label loop, with the
//! module's `equ = extern(...)` / `ensure(...)` header), the recovery from that
//! error LOOPS forever. Minimization could not reduce it below this shape — the
//! loop is context-sensitive (removing the splice, the nested labels, or the
//! header each makes the same trigger error cleanly), which is why it is
//! ledgered as a DEEP parser-recovery bug rather than an obvious one-line fix.
//!
//! Kept `#[ignore]` so the suite never hangs; run explicitly (WITH A TIMEOUT) to
//! reproduce: `timeout 20 cargo test -p sigil-frontend-emp --test
//! parser_recovery_hang -- --ignored recovery_loops` → the process is killed by
//! the timeout (it does not return). When the parse gap or the recovery loop is
//! fixed, this returns and the test can be un-ignored to assert a clean error.
use sigil_frontend_emp::parse_str;

const HANGS: &str = r#"module m in s

use engine.z80_bus.{stop_z80, start_z80}

equ SND_MIRROR_DEST = extern("Sound_Dbg_Mirror")
equ SND_REQ_SRC     = extern("Z80_RAM") + extern("SND_REQ_BASE")
equ SND_STATE_SRC   = extern("Z80_RAM") + extern("SND_STATE_BASE")
equ SND_SEQ_SRC     = extern("Z80_RAM") + extern("SND_SEQ_BASE")
equ SND_TRACE_SRC   = extern("Z80_RAM") + extern("SND_SEQ_TRACE")

const SEQ_MIRROR_CHANNELS = 3
const SEQ_MIRROR_CHBYTES  = 20
const SEQ_MIRROR_HDRCH    = 8 + SEQ_MIRROR_CHANNELS * SEQ_MIRROR_CHBYTES

ensure(SEQ_MIRROR_CHBYTES <= extern("SeqChannel_len"), "a")
ensure(64 + SEQ_MIRROR_HDRCH + extern("SND_SEQ_TRACE_LEN") <= 176, "b")

pub proc Sound_DebugMirror () clobbers(d0/d1/a0/a1) {
    {stop_z80()}
    .copy3c:
        moveq   #SEQ_MIRROR_CHBYTES-1, d0
        .copy3cb:
            move.b  (a0)+, (a1)+
            dbf     d0, .copy3cb
        lea     (extern("SeqChannel_len")-SEQ_MIRROR_CHBYTES)(a0), a0
    rts
}
"#;

#[ignore = "captures a parser infinite-loop bug — parse_str never returns on this input (t25 sound_debug lane, ledgered)"]
#[test]
fn recovery_loops_forever_on_extern_disp_in_context() {
    // This call DOES NOT RETURN today (infinite error-recovery loop). Kept
    // ignored so the suite never hangs; the assertion documents the intended
    // post-fix behavior (a clean parse error, no hang).
    let (_f, diags) = parse_str(HANGS);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "post-fix: the extern-in-displacement form must be a clean parse error, not a hang"
    );
}
