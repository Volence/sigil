//! Label hygiene (Spec 2, Plan 4 — T5, §5.2/§5.3, D-P4.6): the ONE mechanism
//! that turns an `asm { }` / `proc` body's source-spelled labels into emitted
//! symbols. No scope-sets machinery surfaces to the user (§5.3) — a label's
//! visibility is decided entirely by its `export` flag and its owner scope:
//!
//! - **`export .name:`** → the stable, caller-visible symbol `Owner.name`
//!   (§5.2 "External reference is `ProcName.label`"). For a `proc foo` the owner
//!   is `foo`, so an exported `.entry:` is `foo.entry` — referenceable from other
//!   code as `foo.entry`.
//! - **non-`export` `.name:`** → the fresh, per-instantiation symbol
//!   `$asm{k}$name` (`k` a monotonic instantiation id, D-P4.6): two
//!   instantiations of the same template define DISTINCT internal labels (no
//!   collision), and the mangled name is not something outside code can spell, so
//!   the label stays hidden.
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
}

/// The hidden, fresh-per-instantiation symbol for a non-`export` local label
/// (`$asm{k}$name`, D-P4.6). Two instantiations (`k` distinct) never collide.
fn local_symbol(k: u32, name: &str) -> String {
    format!("$asm{k}${name}")
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
    /// source order. `owner` gives exported labels their `Owner.name` spelling;
    /// `k` gives non-exported labels their fresh `$asm{k}$name` spelling. The
    /// first definition of a name wins (a duplicate keeps the earlier symbol).
    pub fn build<'a>(
        owner: &Owner,
        k: u32,
        labels: impl Iterator<Item = (&'a str, bool)>,
    ) -> Self {
        let mut map = HashMap::new();
        for (name, export) in labels {
            let sym =
                if export { owner.export_symbol(name) } else { local_symbol(k, name) };
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
    /// never resolves (its `$asm{k}$name` is unspellable), which is the point.
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
        let scope = LabelScope::build(&Owner::Proc("foo".into()), 3, [("entry", true)].into_iter());
        assert_eq!(scope.label_def("entry"), "foo.entry");
        // Intra-body and external references both land on the same symbol.
        assert_eq!(scope.resolve_ref(".entry"), "foo.entry");
        assert_eq!(scope.resolve_ref("entry"), "foo.entry");
    }

    #[test]
    fn non_export_label_is_fresh_per_instantiation() {
        let a = LabelScope::build(&Owner::Asm(0), 0, [("wait", false)].into_iter());
        let b = LabelScope::build(&Owner::Asm(1), 1, [("wait", false)].into_iter());
        assert_eq!(a.label_def("wait"), "$asm0$wait");
        assert_eq!(b.label_def("wait"), "$asm1$wait");
        assert_ne!(a.label_def("wait"), b.label_def("wait"));
        // An intra-body reference rewrites to the same fresh symbol.
        assert_eq!(a.resolve_ref(".wait"), "$asm0$wait");
    }

    #[test]
    fn external_reference_passes_through_unchanged() {
        // A `foo.entry` reference from a DIFFERENT body (not in this scope's map)
        // is left as the dot-joined external symbol.
        let scope = LabelScope::build(&Owner::Asm(7), 7, [("wait", false)].into_iter());
        assert_eq!(scope.resolve_ref("foo.entry"), "foo.entry");
        // A reference to a hidden non-export label by an owner spelling does NOT
        // find the fresh symbol — it stays unresolved (passes through).
        assert_eq!(scope.resolve_ref("bar.wait"), "bar.wait");
    }
}
