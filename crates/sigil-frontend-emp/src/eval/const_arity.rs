//! Array-arity checking for typed comptime `const` bindings.
//!
//! A `data NAME: T = expr` binding gets its shape checked as a side effect of
//! BYTE EMISSION — [`lower_array`](super::Evaluator::lower_to_data)'s length
//! guard runs while the initializer is being turned into cells. A `const` binds a
//! comptime value and emits nothing, so that path never runs for it and its
//! declared type constrains nothing. This module is the const half of that
//! contract: every typed `const` in the module has its value walked against its
//! declared type and every array-shaped mismatch reported, at the declaration
//! site, whether or not anything ever reads the value.
//!
//! WHY A ONCE-PER-COMPILE VALIDATOR AND NOT A CHECK INSIDE `resolve_const`.
//! Every data item and every proc builds its own [`Evaluator`], so a diagnostic
//! raised where a const RESOLVES fires once per evaluator that touches it — and
//! never at all for a const nothing references. Both are wrong for a declaration
//! error. [`validate_const_arity`] runs once from the lowering funnel and reads
//! declarations, so a never-referenced const is checked exactly like a hot one.
//! This mirrors the sibling `validate_option_newtypes` / `validate_defines`
//! drivers.
//!
//! SCOPE: array LENGTH only. The walk descends through struct fields, tuple
//! elements, arrays-of-arrays and newtype/refinement wrappers to reach every
//! array position a value can occupy, and reports a length mismatch with the same
//! wording the emitting path uses. It deliberately reports nothing else — a
//! wrong-TYPE field or an out-of-range scalar in a const is a separate contract
//! that emission owns today.
use super::{run_on_eval_stack, Evaluator};
use crate::ast;
use crate::layout::Ty;
use crate::value::Value;
use sigil_span::{Diagnostic, Span};

/// Check every typed `const` declaration in `file` for array-arity mismatches
/// (once per compile — see the module banner). `defines`/`contracts`/
/// `include_root` are threaded so a const initializer that reads a `-D` define,
/// an interface member, or an `import(...)` resolves exactly as it does during
/// lowering; a value that cannot be resolved here is skipped rather than guessed
/// at.
pub fn validate_const_arity(
    file: &ast::File,
    defines: &[(String, i128)],
    include_root: Option<&std::path::Path>,
    contracts: &crate::contract::InterfaceEnv,
) -> Vec<Diagnostic> {
    run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.seed_defines(defines);
        ev.seed_interfaces(contracts);
        if let Some(root) = include_root {
            ev.set_include_root(root.to_path_buf());
        }
        let mut seen: Vec<Span> = Vec::new();
        let mut targets: Vec<(&str, &ast::Type, Span)> = Vec::new();
        collect(&file.items, &mut targets);
        for (name, ty, span) in targets {
            // The same declaration can appear twice in one composed file: the
            // resolve pass ambient-prepends a defining module's `pub const`
            // clones, which keep their ORIGINAL span. Report each declaration
            // once per file, keyed by that span.
            if seen.contains(&span) {
                continue;
            }
            seen.push(span);
            ev.check_const_arity(name, ty, span);
        }
        ev.diags
    })
}

/// Gather every typed `const` in `items`, recursing into `section {}` bodies so a
/// section-nested const is checked exactly like a top-level one (§7.1's flat
/// namespace, matching `Evaluator::index_items`).
fn collect<'a>(items: &'a [ast::Item], out: &mut Vec<(&'a str, &'a ast::Type, Span)>) {
    for item in items {
        match item {
            ast::Item::Const(c) => {
                if let Some(ty) = &c.ty {
                    out.push((c.name.as_str(), ty, c.span));
                }
            }
            ast::Item::Section(s) => collect(&s.items, out),
            _ => {}
        }
    }
}

impl Evaluator<'_> {
    /// Resolve one typed `const`'s declared type and value, then walk them
    /// together reporting array-length mismatches.
    ///
    /// The VALUE resolution is a probe: any diagnostic it provokes is discarded
    /// and the const un-memoized, exactly as `check_const_as_address` does. A
    /// const whose initializer does not resolve in this module (a cross-module
    /// reference the composed file cannot see, a cycle, a genuine error) tells us
    /// nothing about its arity, and its real failure is reported by whoever
    /// actually reads it — this validator must never be the second voice on it.
    fn check_const_arity(&mut self, name: &str, ty: &ast::Type, span: Span) {
        let before = self.diags.len();
        let resolved = self.resolve_type(ty);
        if self.diags.len() > before {
            // An unresolvable annotation is reported at the const's real use
            // site (or by the layout pass); drop it here and give up on arity.
            self.diags.truncate(before);
            return;
        }
        if matches!(resolved, Ty::Poison) || !ty_has_array(&resolved) {
            return;
        }
        let fresh = !self.const_memo.contains_key(name);
        let value = self.resolve_const(name, span);
        if self.diags.len() > before {
            self.diags.truncate(before);
            if fresh {
                self.const_memo.remove(name);
            }
            return;
        }
        self.walk_arity(&value, &resolved, span);
    }

    /// Walk `value` against `ty`, reporting every array position whose element
    /// count disagrees with the declared length. Descends only where an array can
    /// still be found; a leaf whose shape does not match its type is left to the
    /// contracts that own type mismatches.
    fn walk_arity(&mut self, value: &Value, ty: &Ty, span: Span) {
        if matches!(value, Value::Poison) {
            return;
        }
        match ty {
            Ty::Array(elem, n) => {
                match value {
                    Value::Array(elems) => {
                        if elems.len() != *n {
                            self.error(
                                span,
                                format!(
                                    "array length mismatch: expected {n} element(s), got {}",
                                    elems.len()
                                ),
                            );
                        }
                        for el in elems {
                            self.walk_arity(el, elem, span);
                        }
                    }
                    // A string in a byte-array slot is the byte-context reading
                    // (`lower_str_as_byte_array`): the author sizes `n`, and
                    // there is no implicit terminator. Non-ASCII poisons the
                    // conversion there and is not an arity question here, so
                    // compare CHARACTER count and stay silent on the rest.
                    Value::Str(s) if matches!(**elem, Ty::Prim { width: 1, .. }) => {
                        let len = s.chars().count();
                        if len != *n {
                            self.error(
                                span,
                                format!("array length mismatch: expected {n} element(s), got {len}"),
                            );
                        }
                    }
                    _ => {}
                }
            }
            Ty::Tuple(elem_tys) => {
                let Value::Tuple(vals) = value else { return };
                for (v, t) in vals.iter().zip(elem_tys.iter()) {
                    self.walk_arity(v, t, span);
                }
            }
            Ty::Struct(sname) => {
                let Value::Struct { fields, .. } = value else { return };
                let layout = self.layout_of_struct(sname, span);
                for fl in &layout.fields {
                    if let Some((_, v)) = fields.iter().find(|(n, _)| n == &fl.name) {
                        let fty = fl.ty.clone();
                        self.walk_arity(v, &fty, span);
                    }
                }
            }
            // A newtype/refinement erases to its underlying for storage; an array
            // can sit under either wrapper.
            Ty::Newtype(nname) => {
                let Some(decl) = self.newtypes.get(nname.as_str()).copied() else { return };
                let underlying = self.resolve_type(&decl.underlying);
                self.walk_arity(value, &underlying, span);
            }
            Ty::Refined { inner, .. } => self.walk_arity(value, inner, span),
            _ => {}
        }
    }
}

/// Whether `ty` can contain an array anywhere reachable by [`walk_arity`]'s
/// descent. Consulted before the value is resolved so a const with no array in
/// its type — the overwhelming majority — pays no evaluation at all.
///
/// [`Ty::Struct`] and [`Ty::Newtype`] answer `true` without unfolding: reaching
/// their field types / underlying needs `&mut Evaluator`, and the walk itself
/// unfolds both correctly. The cost of that conservatism is one const resolution
/// per struct- or newtype-typed const, paid once per compile.
fn ty_has_array(ty: &Ty) -> bool {
    match ty {
        Ty::Array(..) => true,
        Ty::Tuple(elems) => elems.iter().any(ty_has_array),
        Ty::Struct(_) | Ty::Newtype(_) => true,
        Ty::Refined { inner, .. } => ty_has_array(inner),
        _ => false,
    }
}
