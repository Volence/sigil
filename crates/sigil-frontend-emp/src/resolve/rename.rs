//! Post-lowering IR rewrite: rename top-level labels and every fixup target
//! symbol to its canonical cross-module name, so the flat-symbol-table linker
//! resolves cross-module references. Proc-local `$Proc$name` / `$asm{k}$name`
//! symbols are NOT in the map (they contain `$`), so they pass through
//! unchanged — local hygiene is preserved.
use sigil_ir::expr::Expr;
use sigil_ir::{Fragment, Module};
use std::collections::HashMap;

/// Rewrite `module` in place: rename `Label.name` and every fixup target `Expr`
/// per `map` (short name → canonical). Names absent from `map` are left as-is.
pub fn rename_module(module: &mut Module, map: &HashMap<String, String>) {
    for sec in &mut module.sections {
        for label in &mut sec.labels {
            if let Some(canon) = map.get(&label.name) {
                label.name = canon.clone();
            }
        }
        for frag in &mut sec.fragments {
            rename_fragment(frag, map);
        }
    }
}

fn rename_fragment(frag: &mut Fragment, map: &HashMap<String, String>) {
    match frag {
        Fragment::Data(df) => {
            for fx in &mut df.fixups {
                rewrite_expr(&mut fx.target, map);
            }
        }
        Fragment::JmpJsrSym { target, .. } => rewrite_expr(target, map),
        Fragment::RelaxAbsSym { target, short, long, .. } => {
            rewrite_expr(target, map);
            rewrite_expr(&mut short.fixup.target, map);
            rewrite_expr(&mut long.fixup.target, map);
        }
        Fragment::Fill { .. } | Fragment::Reserve { .. } | Fragment::Org { .. } => {}
    }
}

fn rewrite_expr(e: &mut Expr, map: &HashMap<String, String>) {
    match e {
        Expr::Sym(name) => {
            if let Some(canon) = map.get(name) {
                *name = canon.clone();
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            rewrite_expr(lhs, map);
            rewrite_expr(rhs, map);
        }
        Expr::Unary { operand, .. } => rewrite_expr(operand, map),
        Expr::Int(_) => {}
    }
}

/// Test/diagnostic helper: collect every symbol name in a fragment's fixup targets.
pub fn collect_target_syms(frag: &Fragment, out: &mut Vec<String>) {
    match frag {
        Fragment::Data(df) => {
            for fx in &df.fixups {
                collect_expr(&fx.target, out);
            }
        }
        Fragment::JmpJsrSym { target, .. } => collect_expr(target, out),
        Fragment::RelaxAbsSym { target, short, long, .. } => {
            collect_expr(target, out);
            collect_expr(&short.fixup.target, out);
            collect_expr(&long.fixup.target, out);
        }
        Fragment::Fill { .. } | Fragment::Reserve { .. } | Fragment::Org { .. } => {}
    }
}

fn collect_expr(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Sym(n) => out.push(n.clone()),
        Expr::Binary { lhs, rhs, .. } => {
            collect_expr(lhs, out);
            collect_expr(rhs, out);
        }
        Expr::Unary { operand, .. } => collect_expr(operand, out),
        Expr::Int(_) => {}
    }
}
