//! Deferred link-time assertions (D-H.4/D-H.5): the `ensure`/`ensure_fatal`
//! guards whose condition is a PROVISIONAL `here()` (a value known only after
//! `resolve_layout`). The front-end cannot decide them at lowering time, so it
//! records a [`LinkAssert`] on the [`Module`](crate::Module); the linker
//! evaluates each against the post-relaxation symbol table and fails the build
//! on any that folds to `0`.
//!
//! The same channel carries one check that is not a condition: every evaluated
//! `extern(name)` records an [`AssertKind::ExternDefined`] assert, so the name
//! is refused at its own span when no module in the link defines it, whatever
//! the value went on to feed.

use crate::expr::Expr;
use sigil_span::{Level, Span};

/// What a [`LinkAssert`] checks at link.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AssertKind {
    /// `cond` folds to an integer: `0` fails, nonzero passes. Every `ensure`
    /// guard and every layout congruence/parity check is this kind.
    #[default]
    Condition,
    /// `cond` is a bare [`Expr::Sym`] recorded by an `extern(name)` evaluation.
    /// It passes when the link defines the symbol, whatever its value (an equ of
    /// `0` or a label at address `0` is defined), and is refused by name at the
    /// `extern()` call's span when nothing defines it.
    ExternDefined,
}

/// One piece of a deferred guard's message (D-H.5). The comptime parts are frozen
/// to [`Text`](MsgPart::Text) at DEFER time (the comptime env is about to
/// disappear); a placeholder whose value is itself link-time stays an
/// [`Expr`](MsgPart::Expr), folded and rendered at link on failure — so
/// `"overran: at {here()}"` reports the REAL final address.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MsgPart {
    /// A literal run, already interpolated from the comptime environment.
    Text(String),
    /// A link-time subexpression, folded against the post-relaxation symbol table
    /// and rendered on failure.
    Expr(Expr),
}

/// A deferred link-time assertion (D-H.4): an `ensure`/`ensure_fatal` guard whose
/// condition became a link-time value. The linker folds `cond` against the
/// post-`resolve_layout` symbol table — `0` is a failure (the build fails with
/// the rendered `message`), nonzero is a pass.
///
/// `fatal` records which keyword the source used, for diagnostic wording only:
/// at link, `ensure` and `ensure_fatal` are identical in effect (D-H.7 — a
/// deferred guard cannot stop lowering early because lowering already finished;
/// a failing one is an Error diagnostic that fails the build).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkAssert {
    /// The condition, folded at link. `0` fails the build; nonzero passes.
    pub cond: Expr,
    /// The message parts (D-H.5): comptime-frozen text + link-time subexpressions.
    pub message: Vec<MsgPart>,
    /// Whether the source keyword was `ensure_fatal` (diagnostic wording only).
    pub fatal: bool,
    /// The failure diagnostic's severity. Guards and alignment-congruence
    /// asserts are [`Level::Error`] (they fail the build); the
    /// `[layout.odd-item]` data-item check (D2.29 amendment) is
    /// [`Level::Warning`] — reported, never build-failing.
    pub level: Level,
    /// The guard's source span, for the failure diagnostic.
    pub span: Span,
    /// What the linker checks: a folded condition, or that an `extern()` name is
    /// defined at all.
    pub kind: AssertKind,
}

impl LinkAssert {
    /// The check an `extern(name)` evaluation records: the link must define
    /// `name`. `span` is the `extern()` call itself, so a refusal points at the
    /// reference rather than at whatever consumed its value.
    pub fn extern_defined(name: &str, span: Span) -> LinkAssert {
        LinkAssert {
            cond: Expr::Sym(name.to_string()),
            message: Vec::new(),
            fatal: false,
            level: Level::Error,
            span,
            kind: AssertKind::ExternDefined,
        }
    }

    /// The symbol an [`AssertKind::ExternDefined`] assert requires, or `None`
    /// for a condition.
    pub fn extern_name(&self) -> Option<&str> {
        match (&self.kind, &self.cond) {
            (AssertKind::ExternDefined, Expr::Sym(name)) => Some(name),
            _ => None,
        }
    }
}
