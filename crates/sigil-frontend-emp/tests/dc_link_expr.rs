//! `dc.<w>` element naming an equ whose value is a link-time expression
//! (`extern("L") + off`) — the equ-off-link-external-base form on the `.emp`
//! side. A bare-symbol equ keeps the `Cell::SymRef` address lowering; an
//! arithmetic residual tree emits a general link-expr VALUE cell (`Cell::Expr`),
//! the same machinery the typed-data emit path (`lower_link_expr`) and the
//! `offsets`/`dispatch` constructs use. The linker folds it post-placement.
//!
//! This is the surface the MD Debugger `MDDBG__*` `pub equ` table needs: each
//! entry is `extern("ErrorHandlerBlob") + off`, and the blob's own two
//! extension-button `dc.l` cells reference two of them.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

fn lower(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    (module, diags.into_iter().map(|d| d.message).collect())
}

fn linked_bytes(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

// `dc.l <equ = extern("L") + off>` folds to the target label's address + off.
// Layout (section `text`, lma 0): Foo holds one `dc.l` (4 bytes) at offset 0, so
// Target lands at offset 4. Entry = extern("Target") + $10 = 4 + 0x10 = 0x14.
#[test]
fn dc_l_link_expr_equ_folds_to_label_plus_offset() {
    let src = "\
module m in text
pub equ Entry = extern(\"Target\") + $10
proc Foo () clobbers() falls_into Target {
        dc.l    Entry
}
proc Target () clobbers() {
        rts
}
";
    let (m, diags) = lower(src);
    assert!(diags.is_empty(), "unexpected diags: {diags:?}");
    let bytes = linked_bytes(&m);
    // 4-byte big-endian fold of Target(4) + 0x10 = 0x14, then Target's `rts`.
    assert_eq!(&bytes[0..4], &[0x00, 0x00, 0x00, 0x14], "dc.l link-expr fold");
}
