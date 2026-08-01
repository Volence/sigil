//! The resolved game-contract environment (L1): the bind pass
//! ([`crate::resolve::contract`]) turns an `interface` + its one `implement`
//! into a [`ResolvedInterface`], and the evaluator consumes it to resolve
//! `Iface.MEMBER` references and lower `invoke Iface.hook`.
//!
//! Core-free by construction — it holds only [`Value`]s and `String` symbols, so
//! the evaluator (which is Core-free, D-P4.1) can carry it without importing the
//! backend. The bind pass that PRODUCES it lives in `resolve` (which may use the
//! evaluator); the types PRODUCED live here so both layers name one vocabulary.
use crate::value::Value;
use std::collections::HashMap;

/// Every resolved interface in a build, keyed by interface name. Empty is the
/// no-contract default (a build with no `interface`/`implement`) — the evaluator
/// then resolves no `Iface.MEMBER` and lowers no `invoke`, so a contract-free
/// module is byte-identical to one lowered without this env.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InterfaceEnv {
    /// Interface name → its resolved members.
    pub interfaces: HashMap<String, ResolvedInterface>,
}

impl InterfaceEnv {
    /// The empty environment (no interfaces).
    pub fn empty() -> Self {
        InterfaceEnv::default()
    }

    /// Resolve `iface.member`, if both are known.
    pub fn member(&self, iface: &str, member: &str) -> Option<&ResolvedMember> {
        self.interfaces.get(iface).and_then(|i| i.members.get(member))
    }
}

/// One interface's resolved members (the outcome of binding its single
/// `implement` block against its declaration).
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedInterface {
    /// Member name → its resolved binding.
    pub members: HashMap<String, ResolvedMember>,
}

/// A resolved interface member — what a consumer site sees.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedMember {
    /// A `const` member folded to its comptime value (carrying its declared type
    /// via [`Value::Typed`] when the member is a newtype). Read as `Iface.NAME`.
    Const(Value),
    /// A `proc` member bound to a game proc's link symbol. `#Iface.name` yields
    /// the SAME link-deferred immediate a bare symbol reference would.
    Proc(String),
    /// A `hook` member: `Some(symbol)` when bound (`invoke` → `jsr symbol`),
    /// `None` when `empty`/unbound (`invoke` emits nothing).
    Hook(Option<String>),
}
