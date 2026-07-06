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
/// caller-visible `Owner.name` symbol is derived from.
#[derive(Clone, Debug)]
pub enum Owner {
    /// A `proc foo`: an exported `.name:` is caller-visible as `foo.name`.
    Proc(String),
    /// A raw `asm { }` instantiation `k`: an exported label is still stable per
    /// §5.3, spelled `$asm{k}.name`. (The common case is exported labels in
    /// procs; this keeps the model consistent for the rare exported-in-`asm{}`
    /// case without a caller having a name to reach it by.)
    Asm(u32),
}

impl Owner {
    /// The exported label's stable, caller-visible symbol `Owner.name` (§5.2).
    fn export_symbol(&self, name: &str) -> String {
        match self {
            Owner::Proc(p) => format!("{p}.{name}"),
            Owner::Asm(k) => format!("$asm{k}.{name}"),
        }
    }

    /// The hidden, globally-unique symbol for a non-`export` local label
    /// (D-P4.6). A proc owner prefixes with its (unique) name — `$foo$loop` — so
    /// the same `.loop` in another proc is a DISTINCT symbol; a raw `asm { }`
    /// owner prefixes with its globally-monotonic instantiation id `k` —
    /// `$asm{k}$name`. The `$`-wrapped spelling is unspellable from source, so the
    /// label stays hidden. Distinct from [`export_symbol`](Self::export_symbol)'s
    /// dot spelling, so an internal and an exported label never collide.
    fn local_symbol(&self, name: &str) -> String {
        match self {
            Owner::Proc(p) => format!("${p}${name}"),
            Owner::Asm(k) => format!("$asm{k}${name}"),
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

    #[test]
    fn export_label_is_owner_dot_name_for_a_proc() {
        let scope = LabelScope::build(&Owner::Proc("foo".into()), [("entry", true)].into_iter());
        assert_eq!(scope.label_def("entry"), "foo.entry");
        // Intra-body and external references both land on the same symbol.
        assert_eq!(scope.resolve_ref(".entry"), "foo.entry");
        assert_eq!(scope.resolve_ref("entry"), "foo.entry");
    }

    #[test]
    fn non_export_proc_label_is_scoped_to_its_owner() {
        // The SAME `.loop` in two different procs gets DISTINCT symbols — the
        // owner (proc name) is what makes them unique (no counter involved).
        let foo = LabelScope::build(&Owner::Proc("foo".into()), [("loop", false)].into_iter());
        let bar = LabelScope::build(&Owner::Proc("bar".into()), [("loop", false)].into_iter());
        assert_eq!(foo.label_def("loop"), "$foo$loop");
        assert_eq!(bar.label_def("loop"), "$bar$loop");
        assert_ne!(foo.label_def("loop"), bar.label_def("loop"));
        // An intra-body reference rewrites to the same owner-scoped symbol.
        assert_eq!(foo.resolve_ref(".loop"), "$foo$loop");
    }

    #[test]
    fn non_export_asm_label_is_scoped_to_its_instantiation() {
        // Raw `asm { }` owners use the globally-monotonic instantiation id `k`.
        let a = LabelScope::build(&Owner::Asm(0), [("wait", false)].into_iter());
        let b = LabelScope::build(&Owner::Asm(1), [("wait", false)].into_iter());
        assert_eq!(a.label_def("wait"), "$asm0$wait");
        assert_eq!(b.label_def("wait"), "$asm1$wait");
        assert_ne!(a.label_def("wait"), b.label_def("wait"));
        assert_eq!(a.resolve_ref(".wait"), "$asm0$wait");
    }

    #[test]
    fn external_reference_passes_through_unchanged() {
        // A `foo.entry` reference from a DIFFERENT body (not in this scope's map)
        // is left as the dot-joined external symbol.
        let scope = LabelScope::build(&Owner::Asm(7), [("wait", false)].into_iter());
        assert_eq!(scope.resolve_ref("foo.entry"), "foo.entry");
        // A reference to a hidden non-export label by an owner spelling does NOT
        // find the fresh symbol — it stays unresolved (passes through).
        assert_eq!(scope.resolve_ref("bar.wait"), "bar.wait");
    }
}
