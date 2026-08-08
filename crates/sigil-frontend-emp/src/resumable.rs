//! `@resumable` — the STACKLESS-proc scan (bookmark ask 1,
//! `docs/superpowers/2026-08-06-bookmark-implementation-sketch.md` §6).
//!
//! A `@resumable` proc is a supervisor-bookmarkable region: an interrupt may
//! snapshot its whole live state (registers + CCR), rewrite the stacked return
//! PC, and resume it later from any interrupt depth. That is sound ONLY if the
//! body touches the stack pointer NOWHERE — no return address on the stack to be
//! left dangling, no frame to be half-built when preemption lands, all live state
//! in registers. This module is the checked form of that "NO stack access"
//! contract clause.
//!
//! **It scans the evaluated/spliced `CodeBuf`** ([`CodeItem`]s as produced by
//! `eval_proc_body` — post-`with`-splice and post-comptime-template, but
//! pre-backend-encoding), not source tokens. So a stack op that only APPEARS
//! after evaluation (a `with <ctx> { }` bracket splicing an acquire push, a
//! comptime template emitting a `movem`) is caught exactly like a literal one.
//! That is the whole point of scanning `buf.items` rather than the AST.
//!
//! What is forbidden (each is build-fatal — the safety argument of §1 rests on
//! it, so it never softens under `@as_compat`):
//!
//! - a CALL — `bsr` / `jsr` / `jbsr` — pushes a return address;
//! - `pea` pushes an address; `link` / `unlk` build / tear down a frame;
//! - a RETURN — `rts` / `rte` / `rtr` / `rtd` — pops off the stack (a resumable
//!   proc exits by a computed `jmp (aN)` continuation, never a return);
//! - any operand naming sp/a7 — the `-(sp)` push, the `(sp)+` pop, the `(sp)` /
//!   `d(sp)` / `(sp,Xn)` alias forms, a bare `sp` operand, or a `movem` register
//!   list containing a7.
//!
//! What is PERMITTED and must stay permitted: the terminal `jmp (aN)` (aN ≠ a7)
//! continuation exit — a computed transfer through an address register, no stack
//! touch. The typed `jmp (aN) as Type` spelling (bookmark ask 5) is a separate
//! later ask; today the exit rides the plain `jmp (aN)` form, which this scan
//! simply does not flag (a3 is not a7).
//!
//! The register-state set is NOT enforced here: it is the proc's ordinary
//! contract (params + `clobbers` + `out`), and a touch outside it is the existing
//! `[proc.clobber-undeclared]` error. `@resumable` only makes that check
//! mandatory (see `lower/proc.rs`); this module owns the stackless half.

use crate::value::{CodeItem, CodeOperand, Reg};
use sigil_span::Span;

/// One stack-touch a `@resumable` body must not contain.
pub struct StackOpFinding {
    /// The offending instruction's span.
    pub span: Span,
    /// A human description of the offending op, naming it (`bsr`, `` `-(sp)`
    /// push ``, `rts`, …) for the `[resumable.stack-op]` diagnostic.
    pub what: String,
}

/// A7 (== sp) is bit 15 of a `movem` register-list mask (bit0=d0..bit15=a7 — the
/// canonical [`CodeOperand::RegList`] convention).
const A7_MASK_BIT: u16 = 0x8000;

/// Scan an evaluated/spliced proc body (the post-`with`-splice `CodeBuf.items`)
/// for every instruction that touches the stack pointer. Returns a finding per
/// offending instruction (at most one per instruction — the FIRST reason found),
/// in body order. An empty result means the body is provably stackless.
pub fn scan_stack_ops(items: &[CodeItem]) -> Vec<StackOpFinding> {
    let mut out = Vec::new();
    for item in items {
        let CodeItem::Instr { mnemonic, ops, span, .. } = item else { continue };
        if let Some(what) = classify(mnemonic, ops) {
            out.push(StackOpFinding { span: *span, what });
        }
    }
    out
}

/// The reason `mnemonic ops` touches the stack, or `None` if it is stackless.
/// Mnemonic implicit-sp forms win over an operand scan (a `bsr`'s push is the
/// point, not any operand); otherwise the first sp-naming operand is reported.
fn classify(mnemonic: &str, ops: &[CodeOperand]) -> Option<String> {
    match mnemonic {
        // Calls push a return address.
        "bsr" | "jsr" | "jbsr" => {
            return Some(format!("`{mnemonic}` pushes a return address onto the stack"));
        }
        // `pea` pushes an effective address; `link`/`unlk` build/tear a frame.
        "pea" => return Some("`pea` pushes an address onto the stack".to_string()),
        "link" => return Some("`link` allocates a stack frame".to_string()),
        "unlk" => return Some("`unlk` tears down a stack frame".to_string()),
        // Returns pop the return address off the stack. A resumable proc exits by
        // a computed `jmp (aN)` continuation, never a return.
        "rts" | "rte" | "rtr" | "rtd" => {
            return Some(format!(
                "`{mnemonic}` reads a return address off the stack — a resumable proc exits \
                 by `jmp (aN)`, not `{mnemonic}`"
            ));
        }
        _ => {}
    }
    ops.iter().find_map(operand_sp_touch)
}

/// The reason `op` names sp/a7, or `None`. Covers every addressing form the 68k
/// operand model can spell over a7, plus a bare-register `sp` and a `movem` list
/// containing a7.
fn operand_sp_touch(op: &CodeOperand) -> Option<String> {
    let msg = match op {
        CodeOperand::PreDec(Reg::A7) => "a `-(sp)` push",
        CodeOperand::PostInc(Reg::A7) => "an `(sp)+` pop",
        CodeOperand::Ind(Reg::A7) => "an `(sp)` stack access",
        CodeOperand::DispInd { reg: Reg::A7, .. } => "a `d(sp)` displaced stack access",
        CodeOperand::DispSymInd { reg: Reg::A7, .. } => {
            "a `Sym(sp)` symbolic-displacement stack access"
        }
        CodeOperand::IndIdx { reg: Reg::A7, .. } | CodeOperand::IndIdx { xn: Reg::A7, .. } => {
            "an `(sp,Xn)` indexed stack access"
        }
        // sp as the index register of a PC-relative access (`Sym(pc,sp.size)`).
        CodeOperand::PcRelIdx { xn: Reg::A7, .. } => "sp used as a PC-relative index register",
        CodeOperand::Reg(Reg::A7) => "a bare `sp`/`a7` operand (stack-pointer arithmetic)",
        CodeOperand::RegList(mask) if mask & A7_MASK_BIT != 0 => {
            "a `movem` register list containing sp/a7"
        }
        _ => return None,
    };
    Some(msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_span::{SourceId, Span};

    fn sp() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    fn instr(mnemonic: &str, ops: Vec<CodeOperand>) -> CodeItem {
        CodeItem::Instr {
            mnemonic: mnemonic.to_string(),
            size: None,
            ops,
            span: sp(),
            as_type: None,
            targets: vec![],
            author: crate::value::ItemAuthor::User,
        }
    }

    #[test]
    fn a_stackless_body_is_clean() {
        // moveq #0,d0 ; move.b (a0)+,(a1)+ ; jmp (a3) — the resumable idiom.
        let body = vec![
            instr("moveq", vec![CodeOperand::Imm(0), CodeOperand::Reg(Reg::D0)]),
            instr("move", vec![CodeOperand::PostInc(Reg::A0), CodeOperand::PostInc(Reg::A1)]),
            instr("jmp", vec![CodeOperand::Ind(Reg::A3)]),
        ];
        assert!(scan_stack_ops(&body).is_empty());
    }

    #[test]
    fn every_call_and_frame_mnemonic_fires() {
        for m in ["bsr", "jsr", "jbsr", "pea", "link", "unlk"] {
            let f = scan_stack_ops(&[instr(m, vec![])]);
            assert_eq!(f.len(), 1, "{m} should fire");
            assert!(f[0].what.contains(m), "{m}: {}", f[0].what);
        }
    }

    #[test]
    fn every_return_mnemonic_fires() {
        for m in ["rts", "rte", "rtr", "rtd"] {
            let f = scan_stack_ops(&[instr(m, vec![])]);
            assert_eq!(f.len(), 1, "{m} should fire");
            assert!(f[0].what.contains(m) && f[0].what.contains("jmp (aN)"), "{}", f[0].what);
        }
    }

    #[test]
    fn every_sp_operand_form_fires() {
        let forms = [
            CodeOperand::PreDec(Reg::A7),
            CodeOperand::PostInc(Reg::A7),
            CodeOperand::Ind(Reg::A7),
            CodeOperand::DispInd { disp: 4, reg: Reg::A7 },
            CodeOperand::DispSymInd { target: "T".to_string(), reg: Reg::A7 },
            CodeOperand::IndIdx { reg: Reg::A7, disp: 0, xn: Reg::D0, xlong: false },
            CodeOperand::IndIdx { reg: Reg::A0, disp: 0, xn: Reg::A7, xlong: false },
            CodeOperand::PcRelIdx { target: "T".to_string(), addend: 0, xn: Reg::A7, xlong: false },
            CodeOperand::Reg(Reg::A7),
            CodeOperand::RegList(A7_MASK_BIT | 0x0001),
        ];
        for op in forms {
            let f = scan_stack_ops(&[instr("move", vec![op.clone(), CodeOperand::Reg(Reg::D0)])]);
            assert_eq!(f.len(), 1, "{op:?} should fire");
        }
    }

    #[test]
    fn a_symbolic_displacement_over_sp_fires() {
        // `Sym(sp)` — the symbolic-d16 dispatch idiom pointed at the stack pointer.
        let f = scan_stack_ops(&[instr(
            "jmp",
            vec![CodeOperand::DispSymInd { target: "Tbl".to_string(), reg: Reg::A7 }],
        )]);
        assert_eq!(f.len(), 1, "Sym(sp) is a forbidden stack access");
        assert!(f[0].what.contains("sp"), "{}", f[0].what);
    }

    #[test]
    fn sp_as_a_pc_relative_index_fires() {
        // `Sym(pc,sp.w)` — sp used as the index register of a PC-relative access.
        let f = scan_stack_ops(&[instr(
            "lea",
            vec![
                CodeOperand::PcRelIdx {
                    target: "Tbl".to_string(),
                    addend: 0,
                    xn: Reg::A7,
                    xlong: false,
                },
                CodeOperand::Reg(Reg::A0),
            ],
        )]);
        assert_eq!(f.len(), 1, "sp as a PC-rel index is a forbidden stack access");
        assert!(f[0].what.contains("sp"), "{}", f[0].what);
    }

    #[test]
    fn a_movem_list_without_a7_is_clean_unless_it_touches_sp() {
        // The list itself (d0-d2/a2, no a7) is clean; only the `-(sp)` push fires.
        let push = scan_stack_ops(&[instr(
            "movem",
            vec![CodeOperand::RegList(0x0007), CodeOperand::PreDec(Reg::A7)],
        )]);
        assert_eq!(push.len(), 1);
        assert!(push[0].what.contains("sp"), "{}", push[0].what);
        // A movem into memory with no a7 anywhere is clean.
        let clean = scan_stack_ops(&[instr(
            "movem",
            vec![CodeOperand::RegList(0x0007), CodeOperand::Ind(Reg::A0)],
        )]);
        assert!(clean.is_empty());
    }
}
