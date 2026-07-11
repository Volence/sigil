//! T1 (Plan 4) — the `Code` value model: `CodeBuf` monoid + operand-class
//! comptime values (`Width`/`Cc`/`Reg`). Unit-tested at the Rust level; `asm{}`
//! evaluation that PRODUCES these is T3.
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{Cc, CodeBuf, CodeItem, CodeOperand, DataBuf, Reg, Value, Width};
use sigil_span::{SourceId, Span};

fn dummy_span() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

/// A non-empty `CodeBuf` carrying a single label named `n`.
fn label(n: &str) -> CodeBuf {
    let mut b = CodeBuf::empty();
    b.push(CodeItem::Label { name: n.into(), export: false, span: dummy_span() });
    b
}

#[test]
fn code_empty_is_empty() {
    assert!(CodeBuf::empty().items.is_empty());
    let e = CodeBuf::concat(CodeBuf::empty(), CodeBuf::empty());
    assert!(e.items.is_empty());
}

#[test]
fn code_monoid_associativity() {
    let a = label("a");
    let b = label("b");
    let c = label("c");
    let left = CodeBuf::concat(CodeBuf::concat(a.clone(), b.clone()), c.clone());
    let right = CodeBuf::concat(a, CodeBuf::concat(b, c));
    assert_eq!(left, right);
    assert_eq!(left.items.len(), 3);
}

#[test]
fn code_empty_is_identity() {
    let a = label("a");
    assert_eq!(CodeBuf::concat(CodeBuf::empty(), a.clone()), a);
    assert_eq!(CodeBuf::concat(a.clone(), CodeBuf::empty()), a);
}

#[test]
fn type_names() {
    assert_eq!(Value::Width(Width::W).type_name(), "width");
    assert_eq!(Value::Cc(Cc::Ne).type_name(), "cc");
    assert_eq!(Value::Reg(Reg::D0).type_name(), "reg");
    assert_eq!(Value::Code(CodeBuf::empty()).type_name(), "code");
}

#[test]
fn display_width() {
    assert_eq!(Value::Width(Width::B).to_string(), "b");
    assert_eq!(Value::Width(Width::W).to_string(), "w");
    assert_eq!(Value::Width(Width::L).to_string(), "l");
    assert_eq!(Value::Width(Width::S).to_string(), "s");
}

#[test]
fn display_cc() {
    assert_eq!(Value::Cc(Cc::Ne).to_string(), "ne");
    assert_eq!(Value::Cc(Cc::Eq).to_string(), "eq");
    assert_eq!(Value::Cc(Cc::Hi).to_string(), "hi");
}

#[test]
fn display_reg() {
    assert_eq!(Value::Reg(Reg::D0).to_string(), "d0");
    assert_eq!(Value::Reg(Reg::A7).to_string(), "a7");
}

#[test]
fn display_code() {
    let mut b = CodeBuf::empty();
    b.push(CodeItem::Inline(DataBuf::empty(), dummy_span()));
    b.push(CodeItem::Label { name: "x".into(), export: false, span: dummy_span() });
    assert_eq!(Value::Code(b).to_string(), "code[2 items]");
}

#[test]
fn instr_with_operands_survives_concat_and_displays() {
    // Locks the T3 vocabulary shape: an `Instr` carrying resolved operands
    // (an immediate and a register) round-trips through `concat` unchanged and
    // Displays with the right item count.
    let mut b = CodeBuf::empty();
    b.push(CodeItem::Instr {
        mnemonic: "move".into(),
        size: Some(Width::W),
        ops: vec![CodeOperand::Imm(1), CodeOperand::Reg(Reg::D0)],
        span: dummy_span(),
    });
    let joined = CodeBuf::concat(b.clone(), label("done"));
    assert_eq!(joined.items.len(), 2);
    assert_eq!(joined.items[0], b.items[0]); // Instr survives concat intact
    let CodeItem::Instr { mnemonic, size, ops, .. } = &joined.items[0] else {
        panic!("expected Instr");
    };
    assert_eq!(mnemonic, "move");
    assert_eq!(*size, Some(Width::W));
    assert_eq!(ops, &[CodeOperand::Imm(1), CodeOperand::Reg(Reg::D0)]);
    assert_eq!(Value::Code(joined).to_string(), "code[2 items]");
}

// ---- parse-confirm (verification only; grammar already exists) -----------

fn parses(src: &str) {
    let (_file, diags) = parse_str(src);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn asm_patch_bind_section_still_parse() {
    // `asm { }` with a bare register + `.w` size + immediate.
    parses("module m\ncomptime fn f() -> Code {\n    return asm {\n        move.w #1, d0\n    }\n}\n");
    // `patch` and `bind` at statement position inside a comptime fn.
    parses("module m\ncomptime fn f() -> int {\n    patch p: u16\n    bind p = 5\n    return 0\n}\n");
    // `section (...)` with attributes and an empty body.
    parses("module m\nsection s (cpu: z80, vma: $8000) {\n}\n");
}
