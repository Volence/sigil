//! `sigil-build`: the game-as-data build driver (G0b).
//!
//! A game is described by `games/<id>/game.toml`, the engine it builds on by
//! aeon's `engine/engine.toml`, and each names a `baseline.toml` with its
//! contract-closure baseline rows. [`describe`] holds those formats and their
//! loaders; [`driver`] builds every shape from them (`sigil build --project`).
//!
//! The game-independent build code lives here too (G0b Task 4): [`diag`], [`place`],
//! [`map_placement`], [`guards`], [`emit`], [`appendix`] and [`harvest`]. This crate
//! never depends on sigil-harness (`tests/no_harness_edge.rs`).

pub mod describe;

/// Section base alignment: the 68000 evenness default and the declared `[[align]]` rows.
pub mod align;

/// The driver's packing walk: anchors from the map, no frozen size table.
pub mod walk;

/// Closed module gating: every placed module matches exactly one `[[gate]]` rule.
pub mod gating;

/// The driver's module roots: the map's `order` through the closed gate.
pub mod roots;

/// The game-as-data build driver (G0b Task 5): a ROM from a game description, a shape
/// name and the engine description.
pub mod driver;

/// The contract-closure baseline a game-as-data shape is gated on, from its descriptions.
pub mod baseline;

/// The build directory: a private directory outside the tree for what a build
/// generates, and the `generated` root mounted from it (G0b Task 6).
pub mod build_dir;

/// The per-game `[defines]` table reader and the define listing rows (G0b Task 5, from
/// sigil-harness, re-exported there).
pub mod game_defines;

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
