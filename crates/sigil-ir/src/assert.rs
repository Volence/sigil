//! Deferred link-time assertions (D-H.4/D-H.5): the `ensure`/`ensure_fatal`
//! guards whose condition is a PROVISIONAL `here()` (a value known only after
//! `resolve_layout`). The front-end cannot decide them at lowering time, so it
//! records a [`LinkAssert`] on the [`Module`](crate::Module); the linker
//! evaluates each against the post-relaxation symbol table and fails the build
//! on any that folds to `0`.

use crate::expr::Expr;
use sigil_span::{Level, Span};

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
}
