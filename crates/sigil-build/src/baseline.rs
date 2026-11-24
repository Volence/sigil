//! The contract-closure baseline a game-as-data shape's gate diffs against, read from the
//! descriptions (G0b Task 8): the `baseline.toml` the engine description names, and the one
//! the game description names. Nothing else.
//!
//! WHY ONLY THOSE TWO (G0b Task 9a). The closure a `--project` shape is gated on analyses the
//! engine root and the game's own sources root ([`crate::driver::Project::closure_roots`]),
//! not every module the checkout scan finds, so no other game's proc can fire and no other
//! game's rows can be wanted. Until Task 9a the closure analysed the whole scan, and this
//! function also pulled in each other game's rows naming a proc in that game's own sources:
//! narrowing was not neutral while aeon's engine module `engine/sound/sound_sfx.emp` named
//! `SFXID_REV_LOOP`, a constant only sonic4 declared. Aeon now states it as a required game
//! constant (`engine/engine.toml` `required_consts`, refused by name when a game omits it,
//! [`crate::driver::check_required_consts`]) and demo declares it feature-off, so the rule
//! and its reads of the other games' `game.toml` and `baseline.toml` are gone.

use std::path::PathBuf;

use crate::describe::{self, BaselineDesc, ResolvedShape};
use crate::driver::{BuildError, Project};

/// One shape's frozen closure rows, unioned from the description files, with the files they
/// came from (where an adjudicated row is written).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClosureBaseline {
    /// `[proc.out-unverified]` for Z80 procs: `(proc, unit)`.
    pub z80_out_unverified: Vec<(String, String)>,
    /// `[proc.out-unverified]` for 68000 procs: `(proc, register)`.
    pub out_unverified: Vec<(String, String)>,
    /// `[proc.inout-unverified]`: `(proc, register)`.
    pub inout_unverified: Vec<(String, String)>,
    /// `[call.live-clobbered]`, with every file's `d1c_debug` rows on a debug shape:
    /// `(caller, callee, register)`, a multiset.
    pub d1c: Vec<(String, String, String)>,
    /// `[proc.clobber-undeclared]` unbounded (top) procs, with every file's
    /// `clobber_unbounded_debug` rows on a debug shape.
    pub clobber_unbounded: Vec<String>,
    /// `[proc.preserves-sr-unbalanced]` / `[proc.with-sr-level-broken]` procs.
    pub sr_level: Vec<String>,
    /// `[proc.preserves-unverifiable]` word-facet firings: `(proc, reg_facet)`.
    pub word_preserve_unverifiable: Vec<(String, String)>,
    /// The `baseline.toml` files read: the engine's, then the game's.
    pub files: Vec<PathBuf>,
}

impl ClosureBaseline {
    fn add(&mut self, b: &BaselineDesc, debug: bool) {
        self.z80_out_unverified.extend(b.z80_out_unverified.iter().cloned());
        self.out_unverified.extend(b.out_unverified.iter().cloned());
        self.inout_unverified.extend(b.inout_unverified.iter().cloned());
        let debug_rows: &[(String, String, String)] = if debug { &b.d1c_debug } else { &[] };
        self.d1c.extend(b.d1c.iter().chain(debug_rows).cloned());
        let unbounded_debug: &[String] = if debug { &b.clobber_unbounded_debug } else { &[] };
        self.clobber_unbounded.extend(b.clobber_unbounded.iter().chain(unbounded_debug).cloned());
        self.sr_level.extend(b.sr_level.iter().cloned());
        self.word_preserve_unverifiable.extend(b.word_preserve_unverifiable.iter().cloned());
    }
}

/// The baseline `shape` of `p` is gated on: the engine's rows, then the game's.
pub fn closure_baseline(p: &Project, shape: &ResolvedShape) -> Result<ClosureBaseline, BuildError> {
    let debug = shape.features.debug;
    let mut out = ClosureBaseline::default();
    for f in [p.engine_dir.join(&p.engine.baseline), p.game_dir.join(&p.game.baseline)] {
        out.add(&describe::load_baseline(&f)?, debug);
        out.files.push(f);
    }
    Ok(out)
}
