//! t28 P2 — the wrapped-paren displacement parse gap (ledger row 1582, the
//! sound_debug mirror lane's first blocker).
//!
//! `lea (extern("SeqChannel_len") - CONST)(a0), a0` — a parenthesized
//! displacement EXPRESSION followed by `(a0)` — failed to parse ("expected end
//! of line"). The leading `(` routed the operand to the indirect-addressing
//! `paren_operand` path, which only looked for a trailing `+` (post-increment),
//! never a trailing `(` that makes the preceding parenthesized group a
//! DISPLACEMENT. (The UNWRAPPED form `extern("X") - CONST(a0)` already parsed via
//! the bare-expression displacement arm; the gap was specific to the wrapped
//! parens.) The extern framing was incidental — the wrapped-paren displacement
//! failed for any displacement expression.
//!
//! Byte-parity of the extern form against the AS front-end is proven by the
//! sound_debug port (lane S, the demand site). Here: the parse-structure fix and
//! a self-contained byte-equivalence positive control (the wrapping parens are
//! pure grouping — the wrapped and unwrapped forms emit identical bytes).

use sigil_frontend_emp::ast::*;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::Level;

fn first_instr_operands<'a>(f: &'a File, proc: &str) -> &'a [Operand] {
    for it in &f.items {
        if let Item::Proc(p) = it {
            if p.name == proc {
                for s in &p.body {
                    if let AsmStmt::Instr(i) = s {
                        return &i.operands;
                    }
                }
            }
        }
    }
    panic!("no instruction in proc `{proc}`");
}

/// The gap: a wrapped-paren displacement expression over `(a0)` parses, and the
/// first operand is a `DispInd` whose displacement is the parenthesized
/// expression (here `extern("X") - 4`).
#[test]
fn wrapped_paren_extern_displacement_parses_as_dispind() {
    let src = "module m\n\
               pub proc P () clobbers(a0) {\n\
               \x20   lea     (extern(\"SeqChannel_len\") - 4)(a0), a0\n\
               \x20   rts\n\
               }\n";
    let (file, diags) = parse_str(src);
    assert!(
        !diags.iter().any(|d| d.level == Level::Error),
        "wrapped-paren displacement must parse cleanly, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let ops = first_instr_operands(&file, "P");
    match &ops[0] {
        Operand::DispInd { disp, inner, disp_spliced, .. } => {
            assert!(!disp_spliced, "a literal wrapped disp is not a splice");
            assert!(
                matches!(disp, Expr::Binary { op: BinOp::Sub, .. }),
                "displacement should be the `extern(..) - 4` expression, got {disp:?}"
            );
            assert!(
                matches!(**inner, Operand::Ind { .. }),
                "inner should be the `(a0)` indirect, got {inner:?}"
            );
        }
        other => panic!("expected DispInd, got {other:?}"),
    }
}

fn read_bytes(disp_form: &str) -> Vec<u8> {
    // A self-contained `move.w <disp>(a0), d0` with a comptime const base, so no
    // link resolution is needed — the displacement folds at comptime.
    let src = format!(
        "module m\nconst K = 20\nproc read() {{\n    move.w {disp_form}(a0), d0\n    rts\n}}\n"
    );
    let (file, perrs) = parse_str(&src);
    assert!(
        !perrs.iter().any(|d| d.level == Level::Error),
        "parse `{disp_form}`: {:?}",
        perrs.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(
        !diags.iter().any(|d| d.level == Level::Error),
        "lower `{disp_form}`: {:?}",
        diags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
    let sec: &Section = module.sections.iter().find(|s| s.name == "text").expect("text section");
    let off = sec.labels.iter().find(|l| l.name == "read").expect("read label").offset as usize;
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section("text").expect("linked").bytes[off..off + 4].to_vec()
}

/// POSITIVE CONTROL: the wrapping parens are pure grouping. The wrapped form
/// `(K - 4)(a0)` emits exactly the same bytes as the unwrapped `K - 4(a0)` — and
/// both equal the literal `move.w 16(a0), d0`.
#[test]
fn wrapped_and_unwrapped_displacement_emit_identical_bytes() {
    let wrapped = read_bytes("(K - 4)");
    let unwrapped = read_bytes("K - 4");
    let literal = read_bytes("16");
    assert_eq!(wrapped, unwrapped, "wrapped parens must be pure grouping");
    assert_eq!(wrapped, literal, "K-4 = 16, so both must equal `move.w 16(a0), d0`");
}

/// BOUNDARY: P2 closed the PARSE gap only. The wrapped `(Struct.field + 1)(An)`
/// form now parses (no "expected end of line"), but `.field` still does not
/// compose inside arithmetic at LOWERING — the wrapped form fails with the same
/// unknown-name diagnostic as the unwrapped `Struct.field + 1(An)`
/// (`struct_field_disp_plus_n::natural_field_plus_n_does_not_compose_today`).
/// Documents that P2 did NOT widen the field-in-arithmetic gap.
#[test]
fn wrapped_field_arithmetic_disp_parses_but_does_not_lower() {
    let src = "module m\n\
               struct Act { sec_grid_ptr: *u8, grid_w: u16, grid_h: u16 }\n\
               proc read() {\n\
               \x20   move.b (Act.grid_w + 1)(a2), d1\n\
               \x20   rts\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(
        !perrs.iter().any(|d| d.level == Level::Error),
        "P2: the wrapped field-arithmetic displacement must PARSE, got: {:?}",
        perrs.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let (_module, ldiags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(
        ldiags.iter().any(|d| d.level == Level::Error && d.message.contains("Act.grid_w")),
        "the `.field`-in-arithmetic gap is a LOWERING gap: expected an unknown-name error, got: {:?}",
        ldiags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
}
