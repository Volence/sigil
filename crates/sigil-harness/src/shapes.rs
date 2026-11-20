//! A described shape, built through the game-as-data driver (`sigil_build::driver`), for
//! the harness's own tests and tools (G0b Task 11).
//!
//! A [`Shape`] names a game by its directory under the checkout's `games/` and a
//! `[shapes.*]` row of that game's `game.toml`; everything else (features, defines,
//! placement, gating) is read from the descriptions by the driver, exactly as
//! `sigil build --project <checkout>/games/<game> --shape <row>` reads it. The
//! profile-driven driver these helpers replace is deleted; nothing here names a game.

use std::path::{Path, PathBuf};

use sigil_build::describe::{self, Features, ResolvedShape};
use sigil_build::driver::{self, BuildOptions, BuildOutput, CheckOutput, Project};
use sigil_build::map_placement::AnchorOverlay;
use sigil_frontend_emp::export_side::LayoutPass;
use sigil_ir::Section;

use crate::driver_seams::HarnessSeams;
use crate::module_roots::ModuleSpec;

/// One shape of one described game, and how to build it.
#[derive(Debug, Clone)]
pub struct Shape {
    /// The game's directory name under `<checkout>/games/`.
    pub game: String,
    /// The `[shapes.*]` row.
    pub shape: String,
    /// `--extra-entry` modules.
    pub extra_entries: Vec<String>,
    /// `--anchor-overlay`.
    pub anchor_overlay: Option<AnchorOverlay>,
    /// The build export's layout pass.
    pub layout_pass: LayoutPass,
}

impl Shape {
    pub fn new(game: &str, shape: &str) -> Shape {
        Shape {
            game: game.to_string(),
            shape: shape.to_string(),
            extra_entries: Vec::new(),
            anchor_overlay: None,
            layout_pass: LayoutPass::Off,
        }
    }

    pub fn with_extra_entries<I, S>(mut self, ids: I) -> Shape
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extra_entries = ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_anchor_overlay(mut self, overlay: AnchorOverlay) -> Shape {
        self.anchor_overlay = Some(overlay);
        self
    }

    pub fn with_layout_pass(mut self, pass: LayoutPass) -> Shape {
        self.layout_pass = pass;
        self
    }

    /// `<game> <shape>`, the name a diagnostic gives the shape.
    pub fn name(&self) -> String {
        format!("{} {}", self.game, self.shape)
    }

    /// `<checkout>/games/<game>`, the `--project` argument.
    pub fn game_dir(&self, aeon: &Path) -> PathBuf {
        aeon.join("games").join(&self.game)
    }

    /// The game and the engine it names, located (the engine checkout is the one holding
    /// the game).
    pub fn project(&self, aeon: &Path) -> Result<Project, String> {
        driver::load_project(&self.game_dir(aeon), &|k| std::env::var_os(k)).map_err(|e| e.to_string())
    }

    /// The row with its `extends` chain flattened.
    pub fn resolved(&self, aeon: &Path) -> Result<ResolvedShape, String> {
        let game = describe::load_game(&self.game_dir(aeon).join("game.toml")).map_err(|e| e.to_string())?;
        describe::resolve_shape(&game, &self.shape).map_err(|e| e.to_string())
    }

    /// The row's features; panics naming the shape when the description cannot be read.
    pub fn features(&self, aeon: &Path) -> Features {
        self.resolved(aeon).unwrap_or_else(|e| panic!("shape `{}`: {e}", self.name())).features
    }

    /// The game's placement map (`game.toml` `map`).
    pub fn map_path(&self, aeon: &Path) -> Result<PathBuf, String> {
        Ok(self.project(aeon)?.map_path())
    }

    /// The game's module prefix (`entry.game`'s parent).
    pub fn game_module_prefix(&self, aeon: &Path) -> Result<String, String> {
        Ok(self.project(aeon)?.game_prefix().to_string())
    }

    fn options<'a>(&self, seams: &'a HarnessSeams, env: &'a dyn Fn(&str) -> Option<std::ffi::OsString>) -> BuildOptions<'a> {
        BuildOptions {
            extra_entries: self.extra_entries.clone(),
            anchor_overlay: self.anchor_overlay.clone(),
            layout_pass: self.layout_pass,
            seams,
            env,
            generated: None,
        }
    }
}

/// Every shipped shape the checkout describes: each `[shapes.*]` row that is not a
/// `fixture`, of every `games/*/game.toml`, games in directory-name order and rows in
/// description order.
pub fn shipped(aeon: &Path) -> Result<Vec<Shape>, String> {
    let games = aeon.join("games");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&games)
        .map_err(|e| format!("read {}: {e}", games.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|d| d.join("game.toml").is_file())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    for d in dirs {
        let id = d.file_name().and_then(|n| n.to_str()).ok_or_else(|| format!("{}: not a UTF-8 name", d.display()))?.to_string();
        let game = describe::load_game(&d.join("game.toml")).map_err(|e| e.to_string())?;
        for row in game.shapes.keys() {
            if !describe::resolve_shape(&game, row).map_err(|e| e.to_string())?.fixture {
                out.push(Shape::new(&id, row));
            }
        }
    }
    if out.is_empty() {
        return Err(format!("{}: no game describes a shipped shape", games.display()));
    }
    Ok(out)
}

/// The shape's merged define environment (`driver::merged_defines`), as its build reads it.
pub fn shape_defines(aeon: &Path, s: &Shape) -> Result<Vec<(String, i128)>, String> {
    let p = s.project(aeon)?;
    driver::shape_defines(&p, &s.shape).map(|(_, d)| d).map_err(|e| e.to_string())
}

/// Build the shape (`sigil build --project`).
pub fn build(aeon: &Path, s: &Shape) -> Result<BuildOutput, String> {
    let seams = HarnessSeams;
    let env = |k: &str| std::env::var_os(k);
    driver::build(&s.game_dir(aeon), &s.shape, &s.options(&seams, &env)).map_err(|e| e.to_string())
}

/// Check the shape (`sigil build --project ... --check`).
pub fn check(aeon: &Path, s: &Shape) -> Result<CheckOutput, String> {
    let seams = HarnessSeams;
    let env = |k: &str| std::env::var_os(k);
    let p = s.project(aeon)?;
    driver::check_project(&p, &s.shape, &s.options(&seams, &env)).map_err(|e| e.to_string())
}

/// The assembled image (no appendix; its header folded over the image).
pub fn image(aeon: &Path, s: &Shape) -> Result<Vec<u8>, String> {
    Ok(build(aeon, s)?.image)
}

/// The full file `sigil build` writes (the image, then the deb2 appendix on an island
/// shape).
pub fn full_file(aeon: &Path, s: &Shape) -> Result<Vec<u8>, String> {
    Ok(build(aeon, s)?.rom)
}

/// The resolved sections the shape's ROM was linked from.
pub fn resolved_layout(aeon: &Path, s: &Shape) -> Result<Vec<Section>, String> {
    Ok(build(aeon, s)?.export_inputs.sections)
}

/// The module roots the shape places.
pub fn module_registry(aeon: &Path, s: &Shape) -> Result<Vec<ModuleSpec>, String> {
    Ok(build(aeon, s)?.registry)
}
