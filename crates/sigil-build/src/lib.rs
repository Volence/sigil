//! `sigil-build`: the game-as-data build driver (G0b).
//!
//! A game is described by `games/<id>/game.toml`, the engine it builds on by
//! aeon's `engine/engine.toml`, and each names a `baseline.toml` with its
//! contract-closure baseline rows. [`describe`] holds those formats and their
//! loaders. The driver that reads them arrives in later G0b tasks; until then
//! the descriptions are inert and the old driver in sigil-harness ignores them.

pub mod describe;

/// Test support: the aeon reference tree for this crate's reference-dependent tests.
#[doc(hidden)]
pub mod reference;
