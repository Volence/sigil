//! Label hygiene (Spec 2, Plan 4 — T5, §5.2/§5.3, D-P4.6): the ONE mechanism
//! that turns an `asm { }` / `proc` body's source-spelled labels into emitted
//! symbols. No scope-sets machinery surfaces to the user (§5.3) — a label's
//! visibility is decided entirely by its `export` flag and its owner scope:
//!
//! - **`export .name:`** → the stable, caller-visible symbol `Owner.name`
//!   (§5.2 "External reference is `ProcName.label`"). For a `proc foo` the owner
//!   is `foo`, so an exported `.entry:` is `foo.entry` — referenceable from other
//!   code as `foo.entry`.
//! - **non-`export` `.name:`** → a hidden symbol carrying the OWNER so it is
//!   globally unique (D-P4.6): a proc's local label is `$Owner$name` (the proc
//!   name is unique across the module and label names are unique within a proc,
//!   so a `.loop` in `foo` and a `.loop` in `bar` become `$foo$loop` /
//!   `$bar$loop` — no collision, and no counter needed); a raw `asm { }`
//!   instantiation's local label is `$asm{k}$name` with `k` a GLOBALLY-monotonic
//!   instantiation id (threaded across the whole module by lowering, not reset
//!   per proc — see `eval_proc_body`). Either way the mangled name is not
//!   something outside code can spell, so the label stays hidden.
//!
//! Why the owner and NOT `k` alone for the proc case: `k` lives on the evaluator,
//! and lowering builds a FRESH evaluator per proc (so `k` restarts at 0 each
//! proc). Mangling a proc's local with `k` alone therefore collided across procs
//! (`$asm0$loop` for both `foo` and `bar`) — the whole-branch review's CRITICAL.
//! The owner prefix is what makes proc-local labels globally distinct.
//!
//! Kept CPU-neutral / Core-free (pure string computation) so `eval/asm.rs` — the
//! evaluator side — can call it while `lower/` remains the only Core-importing
//! module (D-P4.1). It lives under `lower/` because that is where the plan's file
//! structure homes the hygiene model; nothing here touches `sigil_ir`.

use std::collections::HashMap;

/// The owner scope of an `asm { }` / `proc` body — what an exported label's
/// caller-visible `Owner.name` symbol is derived from. Every owner carries the
/// enclosing MODULE id (`a.b.c`, the dotted `module` path) so a hidden local
/// symbol is unique across the whole multi-module program, not just within one
/// module (Plan 7 #4): the proc name / instantiation id `k` are only unique
/// within a module, so two modules with a `proc init` (or an `asm {}` whose `k`
/// reset to the same value) would otherwise mint colliding `$init$loop` /
/// `$asm{k}$wait` symbols into the flat linker table.
#[derive(Clone, Debug)]
pub enum Owner {
    /// A `proc foo` in module `modid`: an exported `.name:` is caller-visible as
    /// `foo.name`; a private `.name:` is the module-scoped `$modid$foo$name`.
    Proc { module: String, name: String },
    /// A raw `asm { }` instantiation `k` in module `modid`: an exported label is
    /// still stable per §5.3, spelled `$asm{k}.name`; a private label is the
    /// module-scoped `$modid$asm{k}$name`. (The common case is exported labels in
    /// procs; this keeps the model consistent for the rare exported-in-`asm{}`
    /// case without a caller having a name to reach it by.)
    Asm { module: String, k: u32 },
}

impl Owner {
    /// The exported label's stable, caller-visible symbol `Owner.name` (§5.2).
    /// NOT module-qualified: exported (top-level) symbols are the raw names the
    /// resolve/rename pass maps to their canonical `module.id.name` form — this
    /// mangling is only for the hidden `$…$` locals that pass skips.
    fn export_symbol(&self, name: &str) -> String {
        match self {
            Owner::Proc { name: p, .. } => format!("{p}.{name}"),
            Owner::Asm { k, .. } => format!("$asm{k}.{name}"),
        }
    }

    /// The hidden, globally-unique symbol for a non-`export` local label
    /// (D-P4.6, Plan 7 #4). The MODULE id leads so the symbol is unique across the
    /// whole program; then a proc owner adds its (module-unique) name —
    /// `$modid$foo$loop` — so the same `.loop` in another proc or another module
    /// is a DISTINCT symbol, and a raw `asm { }` owner adds its instantiation id
    /// `k` — `$modid$asm{k}$name`. The `$`-wrapped spelling is unspellable from
    /// source (so the label stays hidden) and already unique (so the rename pass
    /// correctly skips it). Distinct from [`export_symbol`](Self::export_symbol)'s
    /// dot spelling, so an internal and an exported label never collide.
    ///
    /// `pub(super)` so `lower/script.rs` can compute a script's hidden resume
    /// table rows' final names through this SAME one-source-of-truth mangling
    /// (R9b.11): the table's `Expr::Str` targets must equal the emitted symbol
    /// of the body's `__resume$<k>` label definitions, or the link fails.
    pub(super) fn local_symbol(&self, name: &str) -> String {
        match self {
            Owner::Proc { module, name: p } => format!("${module}${p}${name}"),
            Owner::Asm { module, k } => format!("${module}$asm{k}${name}"),
        }
    }
}

/// The resolved label environment for ONE `asm { }` / `proc` instantiation: a map
/// from each source-spelled label name to the symbol it emits. Built once per
/// body (from its label statements) and consulted both when a label is DEFINED
/// and when an operand REFERENCES a label.
pub struct LabelScope {
    /// Source label name (bare, no leading dot) → emitted symbol.
    map: HashMap<String, String>,
}

impl LabelScope {
    /// Build the scope from this instantiation's labels: `(name, export)` pairs in
    /// source order. `owner` gives exported labels their caller-visible
    /// `Owner.name` spelling and non-exported labels their hidden, owner-scoped
    /// symbol. The first definition of a name wins (a duplicate keeps the earlier
    /// symbol).
    pub fn build<'a>(owner: &Owner, labels: impl Iterator<Item = (&'a str, bool)>) -> Self {
        let mut map = HashMap::new();
        for (name, export) in labels {
            let sym =
                if export { owner.export_symbol(name) } else { owner.local_symbol(name) };
            map.entry(name.to_string()).or_insert(sym);
        }
        LabelScope { map }
    }

    /// The symbol to EMIT for a label definition named `name` (bare, no dot). A
    /// label always appears in the scope (it was collected from the same body),
    /// so this normally hits the map; the passthrough is a defensive fallback.
    pub fn label_def(&self, name: &str) -> String {
        self.map.get(name).cloned().unwrap_or_else(|| name.to_string())
    }

    /// Resolve an operand's label REFERENCE to its emitted symbol. A `.local`
    /// reference (leading dot) or a bare name that names a label defined in THIS
    /// instantiation maps to that label's emitted symbol (the fresh mangling for
    /// a non-export label, so an intra-body branch resolves; the `Owner.name`
    /// spelling for an exported one). Any other reference — an external
    /// `Owner.label` (already dot-joined), a global symbol — passes through
    /// unchanged, so the linker resolves it against the defining owner's export
    /// symbol. A reference to a hidden non-export label from OUTSIDE therefore
    /// never resolves (its `$Owner$name` is unspellable), which is the point.
    pub fn resolve_ref(&self, name: &str) -> String {
        let key = name.strip_prefix('.').unwrap_or(name);
        self.map.get(key).cloned().unwrap_or_else(|| name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `Proc` owner in module `m` (the common single-file test module id).
    fn proc_owner(name: &str) -> Owner {
        Owner::Proc { module: "m".into(), name: name.into() }
    }

    /// An `Asm` owner (instantiation `k`) in module `m`.
    fn asm_owner(k: u32) -> Owner {
        Owner::Asm { module: "m".into(), k }
    }

    #[test]
    fn export_label_is_owner_dot_name_for_a_proc() {
        // Exported labels are NOT module-qualified (the rename pass handles them).
        let scope = LabelScope::build(&proc_owner("foo"), [("entry", true)].into_iter());
        assert_eq!(scope.label_def("entry"), "foo.entry");
        // Intra-body and external references both land on the same symbol.
        assert_eq!(scope.resolve_ref(".entry"), "foo.entry");
        assert_eq!(scope.resolve_ref("entry"), "foo.entry");
    }

    #[test]
    fn non_export_proc_label_is_scoped_to_its_owner() {
        // The SAME `.loop` in two different procs gets DISTINCT symbols — the
        // owner (proc name), under its module id, is what makes them unique.
        let foo = LabelScope::build(&proc_owner("foo"), [("loop", false)].into_iter());
        let bar = LabelScope::build(&proc_owner("bar"), [("loop", false)].into_iter());
        assert_eq!(foo.label_def("loop"), "$m$foo$loop");
        assert_eq!(bar.label_def("loop"), "$m$bar$loop");
        assert_ne!(foo.label_def("loop"), bar.label_def("loop"));
        // An intra-body reference rewrites to the same owner-scoped symbol.
        assert_eq!(foo.resolve_ref(".loop"), "$m$foo$loop");
    }

    #[test]
    fn non_export_local_label_is_scoped_to_its_module() {
        // Plan 7 #4: the SAME proc name `init` in two DIFFERENT modules gets
        // DISTINCT symbols — the module id is what makes them unique. (Within a
        // module the proc name already disambiguates; across modules only the
        // module prefix does.)
        let a = LabelScope::build(
            &Owner::Proc { module: "a".into(), name: "init".into() },
            [("loop", false)].into_iter(),
        );
        let b = LabelScope::build(
            &Owner::Proc { module: "b".into(), name: "init".into() },
            [("loop", false)].into_iter(),
        );
        assert_eq!(a.label_def("loop"), "$a$init$loop");
        assert_eq!(b.label_def("loop"), "$b$init$loop");
        assert_ne!(a.label_def("loop"), b.label_def("loop"));
    }

    #[test]
    fn non_export_asm_label_is_scoped_to_its_instantiation() {
        // Raw `asm { }` owners use the instantiation id `k`, under the module id.
        let a = LabelScope::build(&asm_owner(0), [("wait", false)].into_iter());
        let b = LabelScope::build(&asm_owner(1), [("wait", false)].into_iter());
        assert_eq!(a.label_def("wait"), "$m$asm0$wait");
        assert_eq!(b.label_def("wait"), "$m$asm1$wait");
        assert_ne!(a.label_def("wait"), b.label_def("wait"));
        assert_eq!(a.resolve_ref(".wait"), "$m$asm0$wait");
        // Plan 7 #4: the SAME `k` in two modules is still distinct (counter reset).
        let other = LabelScope::build(
            &Owner::Asm { module: "other".into(), k: 0 },
            [("wait", false)].into_iter(),
        );
        assert_ne!(a.label_def("wait"), other.label_def("wait"));
    }

    #[test]
    fn external_reference_passes_through_unchanged() {
        // A `foo.entry` reference from a DIFFERENT body (not in this scope's map)
        // is left as the dot-joined external symbol.
        let scope = LabelScope::build(&asm_owner(7), [("wait", false)].into_iter());
        assert_eq!(scope.resolve_ref("foo.entry"), "foo.entry");
        // A reference to a hidden non-export label by an owner spelling does NOT
        // find the fresh symbol — it stays unresolved (passes through).
        assert_eq!(scope.resolve_ref("bar.wait"), "bar.wait");
    }
}
