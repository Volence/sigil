//! Substrate parcel — the cross-file corpus TYPE ENVIRONMENT and the LOUD drop
//! count. A field operand on an IMPORTED struct (`S.b(a0)` where `S` is declared
//! in another module) must resolve when the declaring struct is supplied as
//! ambient, and — critically — must be COUNTED as a dropped instruction when it
//! is NOT, so the silent under-approximation that sat beneath the contract gates
//! can be pinned to zero.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::eval::eval_proc_body_env;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::CodeItem;
use sigil_ir::backend::Cpu;

/// Count `move`-mnemonic instructions in a proc's evaluated buffer.
fn count_moves(buf: &Option<sigil_frontend_emp::value::CodeBuf>) -> usize {
    buf.as_ref()
        .map(|b| {
            b.items
                .iter()
                .filter(|it| matches!(it, CodeItem::Instr { mnemonic, .. } if mnemonic == "move"))
                .count()
        })
        .unwrap_or(0)
}

const DEPS: &str = "module deps\nstruct S {\n  a: u16,\n  b: u16,\n}\n";
const USER: &str = "module m\nproc P (a0: *S) clobbers(d0) {\n  move.w S.b(a0), d0\n  rts\n}\n";

fn user_proc() -> sigil_frontend_emp::ast::File {
    let (f, d) = parse_str(USER);
    assert!(d.is_empty(), "parse USER: {d:?}");
    f
}

/// WITHOUT the ambient struct, `S.b(a0)` cannot resolve `S` → the `move.w`
/// instruction is DROPPED, and the drop is COUNTED (was silent before this
/// parcel).
#[test]
fn cross_file_field_drops_and_is_counted_without_ambient() {
    let f = user_proc();
    let p = f.items.iter().find_map(|i| match i {
        Item::Proc(p) => Some(p),
        _ => None,
    }).unwrap();
    let (buf, _diags, _n, dropped) =
        eval_proc_body_env(&f, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[], &[]);
    assert_eq!(dropped, 1, "the unresolved field instruction must be COUNTED as dropped");
    assert_eq!(count_moves(&buf), 0, "the dropped move must be absent from the buffer");
}

/// WITH the declaring struct supplied as ambient, `S.b(a0)` resolves → the
/// instruction is present and ZERO drops.
#[test]
fn cross_file_field_resolves_with_ambient() {
    let deps = parse_str(DEPS).0;
    let ambient: Vec<Item> = deps.items;
    let f = user_proc();
    let p = f.items.iter().find_map(|i| match i {
        Item::Proc(p) => Some(p),
        _ => None,
    }).unwrap();
    let (buf, diags, _n, dropped) =
        eval_proc_body_env(&f, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[], &ambient);
    assert_eq!(dropped, 0, "S resolved via ambient → nothing drops: {diags:?}");
    assert_eq!(count_moves(&buf), 1, "the field instruction is present in the buffer");
}

/// A file-local declaration SHADOWS a same-named ambient one (ambient only fills
/// cross-file gaps; it never changes local resolution). Here the local `S` has
/// `b` at a different offset than the ambient `S`; the local layout must win.
#[test]
fn file_local_struct_shadows_ambient() {
    // Ambient S: b at offset 2 (after a: u16). Local S: b at offset 4 (after a
    // u32-sized pad + a2). Resolution must use the LOCAL S.
    let ambient = parse_str("module deps\nstruct S {\n  a: u16,\n  b: u16,\n}\n").0.items;
    let (f, d) = parse_str(
        "module m\nstruct S {\n  a: u16,\n  pad: u16,\n  b: u16,\n}\n\
         proc P (a0: *S) clobbers(d0) {\n  move.w S.b(a0), d0\n  rts\n}\n",
    );
    assert!(d.is_empty(), "parse: {d:?}");
    let p = f.items.iter().find_map(|i| match i {
        Item::Proc(p) => Some(p),
        _ => None,
    }).unwrap();
    let (buf, diags, _n, dropped) =
        eval_proc_body_env(&f, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[], &ambient);
    assert_eq!(dropped, 0, "resolves with either S: {diags:?}");
    // The local S puts `b` at offset 4; assert the emitted displacement is 4.
    let disp = buf.as_ref().unwrap().items.iter().find_map(|it| match it {
        CodeItem::Instr { mnemonic, ops, .. } if mnemonic == "move" => Some(ops.clone()),
        _ => None,
    });
    let ops = disp.expect("the move instruction");
    let d = ops.iter().find_map(|o| match o {
        sigil_frontend_emp::value::CodeOperand::DispInd { disp, .. } => Some(*disp),
        _ => None,
    });
    assert_eq!(d, Some(4), "the LOCAL struct's b offset (4) must win over the ambient's (2)");
}
