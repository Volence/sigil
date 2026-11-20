//! The contract-closure baseline a game-as-data shape's gate diffs against, read from the
//! descriptions (G0b Task 8): the `baseline.toml` the engine description names, the one the
//! game description names, and (until Task 9) the rows of every other game whose sources the
//! closure analyses.
//!
//! WHY OTHER GAMES' ROWS AT ALL. The closure analyses every module the checkout scan finds,
//! both games and any fixture, on every shape (the profile-driven driver's census, kept as is:
//! narrowing it to the game's own scope is NOT neutral today, because an engine module's
//! comptime condition names a game-side id, `Sfx_BeginSound`'s `SFXID_REV_LOOP`, which then
//! stops resolving and blinds the gate). So a demo shape fires sonic4's rows too. Which of
//! another game's rows join follows the split the baseline files themselves state: rows naming
//! a proc that LIVES in the game's own sources fire wherever those sources are analysed, so
//! they join; a row naming a proc elsewhere (demo's `Parallax_Update` row, an engine proc that
//! only demo's `Game.SCANLINE_CAPS` fires) is a fact about building that game, so it does not.
//! Task 9's two-root corpus makes a build's corpus its own game plus the engine, and this rule
//! then reduces to the engine's rows and the game's (sigil `docs/QUEUE.md`
//! G0B-DESCRIBE-TASK2-RESIDUE item 3).
//!
//! A scanned module outside the engine and this game that sits under no game description
//! (aeon's `tools/fixtures/`) contributes no rows: any firing in it is a new violation.

use std::path::{Path, PathBuf};

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::resolve::manifest::Manifest;

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
    /// The `baseline.toml` files read, engine first, then the game, then any other game.
    pub files: Vec<PathBuf>,
}

impl ClosureBaseline {
    fn add(&mut self, b: &BaselineDesc, debug: bool, keep: &dyn Fn(&str) -> bool) {
        let pair = |rows: &[(String, String)]| rows.iter().filter(|(p, _)| keep(p)).cloned().collect::<Vec<_>>();
        self.z80_out_unverified.extend(pair(&b.z80_out_unverified));
        self.out_unverified.extend(pair(&b.out_unverified));
        self.inout_unverified.extend(pair(&b.inout_unverified));
        let debug_rows: &[(String, String, String)] = if debug { &b.d1c_debug } else { &[] };
        self.d1c.extend(b.d1c.iter().chain(debug_rows).filter(|(caller, _, _)| keep(caller)).cloned());
    }
}

fn under(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('.'))
}

/// The game description directory a scanned module sits in: the nearest ancestor of its file,
/// inside `checkout`, holding a `game.toml`.
fn owning_game_dir(file: &Path, checkout: &Path) -> Option<PathBuf> {
    file.ancestors().skip(1).take_while(|a| a.starts_with(checkout)).find(|a| a.join("game.toml").is_file()).map(Path::to_path_buf)
}

/// The baseline `shape` of `p` is gated on, over the scanned `manifest` the closure analyses.
pub fn closure_baseline(p: &Project, shape: &ResolvedShape, manifest: &Manifest) -> Result<ClosureBaseline, BuildError> {
    let debug = shape.features.debug;
    let mut out = ClosureBaseline::default();
    let engine_file = p.engine_dir.join(&p.engine.baseline);
    let game_file = p.game_dir.join(&p.game.baseline);
    for f in [&engine_file, &game_file] {
        out.add(&describe::load_baseline(f)?, debug, &|_| true);
        out.files.push(f.clone());
    }

    // Every other game whose sources the scan pulls in, once each, in scan order.
    let checkout = std::fs::canonicalize(&p.checkout).unwrap_or_else(|_| p.checkout.clone());
    let own_prefix = p.game_prefix().to_string();
    let mut others: Vec<PathBuf> = Vec::new();
    for m in &manifest.modules {
        if crate::roots::in_candidate_scope(&m.id, &own_prefix) {
            continue;
        }
        let file = std::fs::canonicalize(&m.path).unwrap_or_else(|_| m.path.clone());
        if let Some(dir) = owning_game_dir(&file, &checkout) {
            if dir != p.game_dir && !others.contains(&dir) {
                others.push(dir);
            }
        }
    }
    for dir in others {
        let game_path = dir.join("game.toml");
        let other = describe::load_game(&game_path)?;
        let prefix = other.entry.game.rsplit_once('.').map_or(other.entry.game.as_str(), |(pre, _)| pre).to_string();
        // The procs that live in that game's own sources.
        let procs: std::collections::BTreeSet<&str> = manifest
            .modules
            .iter()
            .filter(|m| under(&m.id, &prefix))
            .flat_map(|m| m.file.items.iter())
            .filter_map(|it| match it {
                Item::Proc(pd) => Some(pd.name.as_str()),
                _ => None,
            })
            .collect();
        let file = dir.join(&other.baseline);
        out.add(&describe::load_baseline(&file)?, debug, &|proc| procs.contains(proc));
        out.files.push(file);
    }
    Ok(out)
}
