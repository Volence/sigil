//! AS's NAMELESS TEMPORARY LABELS: the bare `+`, `++`, `-`, `/` a source writes
//! in column 1 and branches to with a bare `+`/`++`/`-` operand.
//!
//! The whole construct is two counters and a slot name. This module owns both,
//! so the definition side (`eval.rs`) and the reference side (`expr.rs`) cannot
//! disagree about which slot a given ordinal means -- the same
//! "a reader can never disagree with its writer about where a name lives" rule
//! `sym_key` states for scoped names.
//!
//! # The rules, every one measured against `asl` 1.42 Beta Bld 212
//!
//! (md5 `61e672562465725a8c102288a7da9098`; the probe sources and listings are
//! quoted in `docs/superpowers/notes/2026-09-09-as-nameless-labels.md`. Every
//! listing quoted there is from an `exit 0` run: asl stops iterating its passes
//! once a line errors, so a run carrying ANY error prints unconverged forward
//! branches -- `60FE`, a branch to itself -- for lines that are perfectly fine.
//! Reading values off an errored listing is how the first draft of these rules
//! got the forward direction backwards.)
//!
//! ## Definition
//!
//! A definition is a run of `+`, a single `-`, or a single `/`, **in column 1**.
//! Indentation is not cosmetic: an indented `+` is `error: unknown instruction`,
//! not a label.
//!
//! ```text
//!   4/    1000 : 60FE                bra.s  +
//!   6/    1004 :                      +           ; indented
//!   > > > q1.asm(6):2: error: unknown instruction
//! ```
//!
//! Each definition ADVANCES a counter and then defines the slot at the counter's
//! new value:
//!
//! * `+` × m advances the FORWARD counter by m and defines forward slot `fwd`.
//! * `-` advances the BACKWARD counter by 1 and defines backward slot `bwd`.
//! * `/` advances BOTH by 1 and defines both slots -- it is the bidirectional
//!   form, reachable from either side.
//!
//! The `m > 1` case is not a curiosity invented here, it is measured, and it is
//! the case that distinguishes "a `++` definition is an alias for `+`" from
//! "a `++` definition consumes two slots". It consumes two:
//!
//! ```text
//!   4/    1000 : 6004                bra.s  +          ; -> $1006, slot 1
//!   5/    1002 : 6004                bra.s  +++        ; -> $1008, slot 3
//!   7/    1006 :                     +                 ; slot 1
//!   9/    1008 :                     ++                ; slot 3, NOT slot 2
//! ```
//!
//! -- and slot 2 is then never defined at all, which is why a `bra.s ++` in that
//! file is `error: symbol undefined`.
//!
//! The multi-character form is a `+` privilege and not a general one. `--` and
//! `//` in column 1 are both `error: invalid symbol name`, so [`classify_def`]
//! accepts a run only for `+`.
//!
//! ## Reference
//!
//! * `+` × k is forward slot `fwd + k`.
//! * `-` × k is backward slot `bwd - k + 1`.
//!
//! Both read the counters as they stand when the REFERENCING STATEMENT is
//! parsed, which is what makes the ordinals positional. It also settles the
//! same-line case in both directions at once, with no rule of its own: a
//! definition in column 1 has already advanced its counter by the time the rest
//! of that line is dispatched, so
//!
//! ```text
//!   6/    1004 : 60FE                -  bra.s  -       ; its OWN line's label
//!   4/    1000 : 6004                +  bra.s  +       ; NOT its own; the next one
//! ```
//!
//! -- backward `-` with k=1 lands on slot `bwd`, which the line just defined,
//! while forward `+` with k=1 wants slot `fwd + 1`, one past it.
//!
//! ## Scope: the counters are global, the definitions are not
//!
//! asl keeps ONE pair of counters for the whole pass, but files each definition
//! in the namespace of the expansion instance (a macro expansion, or one
//! iteration of a `rept` / `irp` / `irpc` / `while`) that is innermost where it
//! is written -- the same namespace a plain label written there lands in. A
//! reference names its slot by the global counters exactly as above, and then
//! reaches it only if the instance that owns the slot is live around the
//! reference. So a `+` written in a macro body is `error: symbol undefined` to a
//! reference made before the call, even when a later definition outside the
//! macro sits one slot further on (`c03`), and a `-` written in a body is out of
//! reach once the expansion has returned (`c01`). Both directions from INSIDE a
//! body to a definition outside it resolve (`c06`, `c07`), as do a nested
//! expansion reaching its caller's definition (`n04`, `n05`). Probes and asl's
//! verdicts: `docs/superpowers/notes/2026-09-12-as-macro-label-leak.md`.
//!
//! This module does not implement that half, `Asm::define_nameless_slot` and
//! `Asm::sym_key` do: a slot is just a name, and it is filed and looked up by
//! the same rule a plain label is.
//!
//! ## What this module deliberately does NOT model
//!
//! asl's `+` run DEFINITION of length m >= 2 does NOT advance the forward
//! counter: it defines slot `c + m - 1` (zero-based, asl's own `__forwN`) and a
//! following single `+` still defines slot `c`. Read off asl's symbol table,
//! `+`, `++`, `+` at $102/$104/$106 is `__forw0 = 102, __forw2 = 104,
//! __forw1 = 106`. [`classify_def`]'s `Forward(m)` advances by m, which agrees
//! on every reference measured when this module was written (none had a `+`
//! after a `++`) and disagrees on a reference spanning both. A separate row,
//! recorded in the 2026-09-12 note; not changed here.

use crate::token::{Punct, Tok, Token};

/// The two nameless-label counters, as they stand at one point in the pass.
///
/// `Copy` and plain: a reference resolves against a SNAPSHOT, so nothing here
/// can be mutated through the parser by accident.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct NamelessCounts {
    /// Forward-capable definitions (`+`, `/`) advanced past so far.
    pub fwd: u32,
    /// Backward-capable definitions (`-`, `/`) advanced past so far.
    pub bwd: u32,
}

/// The symbol-table name of forward slot `n`.
///
/// The leading space is what keeps these out of the user's namespace: no AS
/// source can spell a symbol with one, and the front end already relies on that
/// for expansion scopes (`" exp#N"`). It also keeps them sorted together and
/// obviously synthetic in any symbol listing that shows them.
///
/// The name SAYS `nameless` because it is not purely internal: a reference
/// deeper than the definitions behind it (`bra.s ---` with two `-` above)
/// reaches the linker as an unresolved symbol, and the linker prints the name.
/// `unresolved symbol ` -#0`` sends the reader looking for a typo; `unresolved
/// symbol ` nameless-#0`` names the construct they wrote. asl's own answer here
/// is `error: symbol undefined`, so a refusal is right either way and only the
/// wording was in question.
pub fn fwd_slot(n: u32) -> String {
    format!(" nameless+#{n}")
}

/// The symbol-table name of backward slot `n`. See [`fwd_slot`].
pub fn bwd_slot(n: u32) -> String {
    format!(" nameless-#{n}")
}

/// What a column-1 token run defines, if anything.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Def {
    /// `+` × m: advance the forward counter by m, define the slot it lands on.
    Forward(u32),
    /// `-`: advance the backward counter, define the slot it lands on.
    Backward,
    /// `/`: advance both counters, define both slots.
    Both,
    /// A run asl refuses outright: `-` × m or `/` × m for m > 1
    /// (`error: invalid symbol name`).
    Invalid,
}

/// Read a line's leading tokens as a nameless DEFINITION, or `None` when they
/// are not one.
///
/// `col1` is the caller's column-rule answer for the first token; a run that is
/// indented is not a definition at all and this returns `None` so the caller
/// reports whatever it reports for an indented `+` today.
///
/// Returns the [`Def`] and the number of tokens it consumed, so the caller can
/// dispatch the rest of the line -- `-\tdbf\td0,-` is one line carrying both a
/// definition and a reference, and it is 18 of the corpus's references.
pub fn classify_def(body: &[Token], col1: bool) -> Option<(Def, usize)> {
    if !col1 {
        return None;
    }
    let kind = match body.first()?.tok {
        Tok::Punct(p @ (Punct::Plus | Punct::Minus | Punct::Slash)) => p,
        _ => return None,
    };
    let n = body
        .iter()
        .take_while(|t| matches!(t.tok, Tok::Punct(p) if p == kind))
        .count();
    let def = match kind {
        Punct::Plus => Def::Forward(n as u32),
        Punct::Minus if n == 1 => Def::Backward,
        Punct::Slash if n == 1 => Def::Both,
        _ => Def::Invalid,
    };
    Some((def, n))
}

/// Whether a token can begin a primary expression.
///
/// This is the whole of the operator-versus-label disambiguation, and it is a
/// LOCAL test because AS's own rule is local. AS splits an expression at the
/// RIGHTMOST operator of the loosest precedence tier present, so in a leading
/// run of `+`/`-` the LAST one is the binary operator and everything before it
/// is the left-hand side -- which, being a bare run, is a nameless reference.
/// Only when nothing an operand could apply to follows the run is the whole run
/// the reference.
///
/// The pair that shows this is not a guess:
///
/// ```text
///   dc.l -Base    ->  FFFF EFE0   ; unary negation: one `-`, nothing to its left
///   dc.l --Base   ->  FFFF FFE0   ; (-) - Base:     the LAST `-` is the operator
///   dc.l +-Base   ->  0000 0010   ; (+) - Base
///   dc.l 1+-2     ->  error: wrong number of operands
/// ```
///
/// with `-` = $1000, `Base` = $1020, `+` = $1030. The last row is the one that
/// rules out "a `-` after a binary operator is unary": AS splits `1+-2` at the
/// rightmost `-`, is left with `1+` on the left, and refuses it. sigil accepts
/// that expression as `1 + (-2)` and has since before this feature; closing
/// that divergence is a separate, byte-changing question and is NOT touched
/// here.
pub(crate) fn starts_atom(t: &Tok) -> bool {
    match t {
        Tok::Int(_) | Tok::Float(_) | Tok::Str(_) | Tok::Dollar | Tok::Ident(_) => true,
        // `*` in atom position is AS's other spelling of the program counter
        // (see `expr.rs::parse_atom`), so it does begin an operand.
        Tok::Punct(p) => matches!(
            p,
            Punct::LParen | Punct::Tilde | Punct::TildeTilde | Punct::Star
        ),
    }
}
