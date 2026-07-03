//! Case-sensitive symbol table with global + local(`.foo`) + dotted
//! cross-scope(`Global.local`) resolution and poisoned entries.
//!
//! Names are stored **fully qualified**: a global is `"Foo"`, a local is
//! `"Scope.foo"`. Reference resolution qualifies a leading-`.` name with the
//! current scope; an already-dotted name resolves directly.

use std::collections::BTreeMap;

/// A symbol's resolved value, or a poison marker for a name that was defined
/// but could not be given a concrete value.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SymbolValue {
    Int(i64),
    Poison,
}

/// Case-sensitive symbol table keyed by fully-qualified name.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SymbolTable {
    entries: BTreeMap<String, SymbolValue>,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable { entries: BTreeMap::new() }
    }

    /// Define (or overwrite) a fully-qualified symbol.
    ///
    /// **Precondition:** a bare global name must never contain `.`. The table
    /// keys locals as the composite `"Scope.local"`, so a global literally
    /// named `"Foo.bar"` would alias the local `.bar` defined under scope
    /// `"Foo"` — the table cannot distinguish them. Today's only producer (the
    /// emp front-end) guarantees this because its lexer never lets `.` appear
    /// inside an identifier; callers that construct names by other means must
    /// uphold it.
    ///
    /// Redefinition is currently last-write-wins; `equ`-error / `set`-allowed
    /// redefinition diagnostics (Core spec §6.x) land with the real producer in
    /// Plan 4.
    pub fn define(&mut self, name: &str, value: SymbolValue) {
        self.entries.insert(name.to_string(), value);
    }

    /// Resolve a reference `name` seen inside `scope` (the enclosing global
    /// label, if any) to a concrete integer. Returns `None` for an unknown or
    /// poisoned symbol.
    ///
    /// - `.foo`  → looked up as `"{scope}.foo"` (requires a scope).
    /// - `A.b`   → looked up verbatim (dotted cross-scope).
    /// - `foo`   → looked up verbatim (global).
    pub fn resolve(&self, name: &str, scope: Option<&str>) -> Option<i64> {
        let key = if let Some(local) = name.strip_prefix('.') {
            match scope {
                Some(s) => format!("{s}.{local}"),
                None => return None,
            }
        } else {
            name.to_string()
        };
        match self.entries.get(&key) {
            Some(SymbolValue::Int(v)) => Some(*v),
            Some(SymbolValue::Poison) | None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_lookup() {
        let mut t = SymbolTable::new();
        t.define("SfxBlobWinTab", SymbolValue::Int(0x845F));
        assert_eq!(t.resolve("SfxBlobWinTab", None), Some(0x845F));
        assert_eq!(t.resolve("Missing", None), None);
    }

    #[test]
    fn local_reference_qualifies_with_current_scope() {
        let mut t = SymbolTable::new();
        // `.fetch` defined under scope `Sequencer_NextOpcode`.
        t.define("Sequencer_NextOpcode.fetch", SymbolValue::Int(0x1234));
        // Referenced as `.fetch` from inside that scope.
        assert_eq!(t.resolve(".fetch", Some("Sequencer_NextOpcode")), Some(0x1234));
        // Same local name under a different scope is independent.
        t.define("Other.fetch", SymbolValue::Int(0x9999));
        assert_eq!(t.resolve(".fetch", Some("Other")), Some(0x9999));
    }

    #[test]
    fn dotted_cross_scope_reference() {
        let mut t = SymbolTable::new();
        t.define("Sequencer_NextOpcode.fetch", SymbolValue::Int(0x1234));
        // `jp Sequencer_NextOpcode.fetch` from OUTSIDE the scope.
        assert_eq!(t.resolve("Sequencer_NextOpcode.fetch", Some("SomeOther")), Some(0x1234));
        assert_eq!(t.resolve("Sequencer_NextOpcode.fetch", None), Some(0x1234));
    }

    #[test]
    fn poison_value_resolves_to_none() {
        let mut t = SymbolTable::new();
        t.define("Bad", SymbolValue::Poison);
        assert_eq!(t.resolve("Bad", None), None);
    }

    #[test]
    fn lookup_closure_bridges_to_fold() {
        use crate::expr::{Expr, Fold};
        let mut t = SymbolTable::new();
        t.define("Sfx_33", SymbolValue::Int(0x6569A));
        let e = Expr::Sym("Sfx_33".to_string());
        // The scope-aware closure the linker hands to Expr::fold.
        let scope: Option<&str> = None;
        let folded = e.fold(&|name| t.resolve(name, scope));
        assert_eq!(folded, Fold::Value(0x6569A));
    }
}
