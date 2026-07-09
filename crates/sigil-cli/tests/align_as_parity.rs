//! D2.29 AS-parity vectors: the same logical layout through the AS
//! front-end's `align` and through `.emp` `align` must produce identical
//! bytes (§4.8 — "$00 fill, exact AS parity by construction"; AS `even`
//! ports as `align 2`). Lives in sigil-cli because only sigil-cli /
//! sigil-harness may depend on sigil-frontend-as (crate_graph.rs (c)).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;

fn as_image(src: &str) -> Vec<u8> {
    let opts = sigil_frontend_as::Options::default();
    let module = sigil_frontend_as::assemble(src, &opts).expect("AS assemble");
    let linked = sigil_link::link(&module.sections, &SymbolTable::new()).expect("AS link");
    sigil_link::flatten(&linked, 0x00)
}

fn emp_image(src: &str) -> Vec<u8> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(diags.is_empty(), "clean lower: {diags:?}");
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

#[test]
fn align_4_parity() {
    let as_img = as_image("\tcpu 68000\n\tdc.b 1,2,3\n\talign 4\n\tdc.b 9\n");
    let emp_img = emp_image("module m\ndata D1: [u8; 3] = [1, 2, 3]\nalign 4\ndata D2: [u8; 1] = [9]\n");
    assert_eq!(emp_img, as_img);
    assert_eq!(as_img, vec![1, 2, 3, 0, 9], "and both match the hand-derivation");
}

#[test]
fn align_2_word_data_parity() {
    // NOTE: sigil-frontend-as does not implement `even` at all (probed at
    // tranche 0) — the D2.29 "AS `even` ports as `align 2`" translation is
    // MANDATORY at port time, on both frontends. Parity is over `align 2`.
    let as_img = as_image("\tcpu 68000\n\tdc.b 7\n\talign 2\n\tdc.w $1234\n");
    let emp_img = emp_image("module m\ndata D1: [u8; 1] = [7]\nalign 2\ndata D2: [u16; 1] = [$1234]\n");
    assert_eq!(emp_img, as_img);
    assert_eq!(as_img, vec![7, 0, 0x12, 0x34]);
}

#[test]
fn already_aligned_parity() {
    let as_img = as_image("\tcpu 68000\n\tdc.b 1,2\n\talign 2\n\tdc.b 9\n");
    let emp_img = emp_image("module m\ndata D1: [u8; 2] = [1, 2]\nalign 2\ndata D2: [u8; 1] = [9]\n");
    assert_eq!(emp_img, as_img);
    assert_eq!(as_img, vec![1, 2, 9], "no pad when already on the boundary");
}
