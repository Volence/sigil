//! `sigil-build`: the game-as-data build driver (G0b).
//!
//! A game is described by `games/<id>/game.toml`, the engine it builds on by
//! aeon's `engine/engine.toml`, and each names a `baseline.toml` with its
//! contract-closure baseline rows. [`describe`] holds those formats and their
//! loaders. The driver that reads them arrives in later G0b tasks; until then
//! the descriptions are inert and the old driver in sigil-harness ignores them.

pub mod describe;

/// The build's diagnostics: the warn tier, the chained build's location authority,
/// and the one error renderer (G0b Task 4, from sigil-harness).
pub mod diag;

/// The placement map reader (`games/<g>/map.toml`): anchors, holes, budgets, the
/// driving `order` (G0b Task 4, from sigil-harness).
pub mod map_placement;

/// The packing walk's generic half: pinned measurement, the declared chain, align
/// recomputation, section predicates (G0b Task 4, from sigil-harness).
pub mod place;

/// The verdicts on a placed program: drift guards, the guard census, the placement
/// contract, budgets, region fit (G0b Task 4, from sigil-harness).
pub mod guards;

/// The emitted artifact's glue: the build result, the canonical listing, the Z80 bank
/// windows, the header fields and CRCs (G0b Task 4, from sigil-harness).
pub mod emit;

/// The deb2 appendix: convsym over the canonical listing, the blob-end contract, the
/// header re-fix and the presence controls (G0b Task 4, from sigil-harness).
pub mod appendix;

/// What the build gathers before it places: the AS side's result, the lowered `.emp`
/// program, the comptime-helper rewrites, `--extra-entry` (G0b Task 4, from sigil-harness).
pub mod harvest;

/// Test support: the aeon reference tree for this crate's reference-dependent tests.
#[doc(hidden)]
pub mod reference;
