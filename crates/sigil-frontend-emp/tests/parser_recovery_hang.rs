//! t25/t28 (sound_debug lane) — a parser ROBUSTNESS bug, captured as a
//! documented repro. Before the t28 P1 fix, `parse_str` HUNG (infinite
//! error-recovery loop, never terminated) on the inputs below instead of
//! emitting a clean parse error. A front-end must error loudly, never spin.
//!
//! ROOT CAUSE (t28 P1): `recover_to_next_decl` lists `extern` among the
//! declaration OPENERS, but `extern` is a CONTEXTUAL opener — only `extern proc`
//! is a real item. When an upstream mis-parse left recovery positioned on a bare
//! `extern` (the `extern("Sym")` comptime read in expression position, e.g. the
//! `equ SND_* = extern(...)` header lines), recovery STOPPED there without
//! consuming; `item()` then failed on the same token (it is not `extern proc`)
//! and returned to recovery, which stopped on the same token again — an infinite
//! loop. The fix extends the existing contextual-opener guard (already present
//! for `ensure`/`ensure_fatal`/`align`) to skip past a non-`proc` `extern`.
//!
//! The `asm_body` zero-progress guard shipped at t25 is a SEPARATE robustness
//! defense (a stuck statement parse inside a proc body) and is kept; the actual
//! hang lived in the top-level recovery loop, not in operand/expr parse.
use sigil_frontend_emp::parse_str;

/// Minimal, stable direct regression for the root cause: a bare `extern(...)`
/// at item position (invalid — a comptime read is not a declaration) must be
/// recovered past, producing a clean error and NEVER hanging. Independent of the
/// extern-in-displacement parse gap (P2) — this stays a valid regression after
/// that form becomes supported.
#[test]
fn recover_does_not_spin_on_bare_extern_at_item_position() {
    let src = "module m in s\n\nextern(\"X\")\n";
    let (_f, diags) = parse_str(src);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "a bare `extern(...)` at item position must be a clean parse error, not a hang"
    );
}

/// The original context-sensitive integration repro (t25 sound_debug body). The
/// `lea (extern(...) - CONST)(a0), a0` line is an extern-in-displacement form
/// (the P2 parse gap): at P1 it is still unsupported, so the accumulated context
/// below reports a clean parse error and RECOVERS — before the P1 fix this
/// recovery hung. (When P2 lands and the extern-in-displacement form parses, the
/// sound_debug port itself becomes the acceptance test for the valid form; this
/// test continues to assert the pre-P2 unsupported-form error path terminates.)
const RECOVERS: &str = r#"module m in s

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

#[test]
fn recovery_terminates_on_extern_disp_in_context() {
    // Before the P1 fix this call DID NOT RETURN (infinite recovery loop). Now
    // it returns with a clean parse error (the extern-in-displacement form is
    // still an unsupported parse gap at P1).
    let (_f, diags) = parse_str(RECOVERS);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "post-fix: the extern-in-displacement form must be a clean parse error, not a hang"
    );
}
