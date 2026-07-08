//! Post-lowering IR rewrite: rename top-level labels and every fixup target
//! symbol to its canonical cross-module name, so the flat-symbol-table linker
//! resolves cross-module references. Proc-local `$Proc$name` / `$asm{k}$name`
//! symbols are NOT in the map (they contain `$`), so they pass through
//! unchanged — local hygiene is preserved.
use sigil_ir::expr::Expr;
use sigil_ir::{Fragment, Module};
use std::collections::HashMap;

/// Resolve one symbol/label name through the rename `map`, handling dotted
/// exported labels (`Owner.local`, e.g. `foo.entry` from `export .entry:` in
/// `proc foo`). Precedence:
/// 1. whole-name hit — an ordinary top-level name mapped to its canonical;
/// 2. dotted-owner hit — the segment before the FIRST dot is a mapped name, so
///    module-qualify to `<renamed-owner>.<rest>` (`foo.entry` → `a.foo.entry`).
///    This fixes the false-reject of exported labels AND module-qualifies the
///    owner, so two modules' private `proc foo { export .entry }` no longer
///    collide in the flat link table.
///
/// Returns `None` when neither applies (name passes through unchanged) — e.g.
/// `$`-hygiene locals, which never appear in `map`.
///
/// INVARIANT the first-dot split rests on: an owner is always a SINGLE-segment
/// item name (proc/data names contain no dots), so everything after the first
/// dot is the exported-local part. A future multi-dot label scheme would need
/// this split revisited.
pub fn canonicalize_name(name: &str, map: &HashMap<String, String>) -> Option<String> {
    if let Some(canon) = map.get(name) {
        return Some(canon.clone());
    }
    if let Some((owner, rest)) = name.split_once('.') {
        if let Some(owner_canon) = map.get(owner) {
            return Some(format!("{owner_canon}.{rest}"));
        }
    }
    // D-PP.3 — MODULE-QUALIFIED reference (`pitcher_plant.init`,
    // `badniks.pitcher_plant.init`): the leading segments name (a suffix of) the
    // DEFINING MODULE'S id and the final segment is an in-scope short name. Split
    // at the LAST dot into `(mod_path, item)`; if `item` maps to a canonical
    // `<full.module.id>.item` whose module portion ENDS WITH `mod_path` (segment-
    // aligned), the qualified spelling refers to that same symbol — resolve to
    // the canonical. This fixes the string form's qualified spelling
    // (`"pitcher_plant.init"`, previously `unknown symbol` even when imported) and
    // gives dotted label-value barewords the same resolution, aligning with
    // `examples/main.emp`'s documented `code: pitcher_plant.init` intent. Bare
    // (unqualified) names still take the whole-name hit above, unchanged.
    if let Some((mod_path, item)) = name.rsplit_once('.') {
        if let Some(canon) = map.get(item) {
            // `canon` is `<module.id>.<item>`; strip the trailing `.item` to get
            // the module id, then require `mod_path` to be a segment-aligned
            // SUFFIX of it (so `pitcher_plant` matches `badniks.pitcher_plant`,
            // but `plant` does not match `pitcher_plant`).
            if let Some(canon_mod) = canon.strip_suffix(&format!(".{item}")) {
                if canon_mod == mod_path || canon_mod.ends_with(&format!(".{mod_path}")) {
                    return Some(canon.clone());
                }
            }
        }
    }
    None
}

/// Rewrite `module` in place: rename `Label.name` and every fixup target `Expr`
/// per `map` (short name → canonical), including dotted exported labels via
/// [`canonicalize_name`]. Names absent from `map` are left as-is.
pub fn rename_module(module: &mut Module, map: &HashMap<String, String>) {
    for sec in &mut module.sections {
        for label in &mut sec.labels {
            if let Some(canon) = canonicalize_name(&label.name, map) {
                label.name = canon;
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
        Fragment::RelaxLadder { target, candidates, .. } => {
            rewrite_expr(target, map);
            for c in candidates {
                rewrite_expr(&mut c.fixup.target, map);
            }
        }
        Fragment::Fill { .. } | Fragment::Reserve { .. } | Fragment::Org { .. } => {}
    }
}

fn rewrite_expr(e: &mut Expr, map: &HashMap<String, String>) {
    match e {
        Expr::Sym(name) => {
            if let Some(canon) = canonicalize_name(name, map) {
                *name = canon;
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
        Fragment::RelaxLadder { target, candidates, .. } => {
            collect_expr(target, out);
            for c in candidates {
                collect_expr(&c.fixup.target, out);
            }
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
