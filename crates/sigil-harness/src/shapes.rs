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
        driver::load_project(&self.game_dir(aeon), &no_env).map_err(|e| e.to_string())
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
    let env = no_env;
    driver::build(&s.game_dir(aeon), &s.shape, &s.options(&seams, &env)).map_err(|e| e.to_string())
}

/// Check the shape (`sigil build --project ... --check`).
pub fn check(aeon: &Path, s: &Shape) -> Result<CheckOutput, String> {
    let seams = HarnessSeams;
    let env = no_env;
    let p = s.project(aeon)?;
    driver::check_project(&p, &s.shape, &s.options(&seams, &env)).map_err(|e| e.to_string())
}

/// The shape's lowered `.emp` program (`driver::emp_program`), unplaced and unlinked.
pub fn emp_program(aeon: &Path, s: &Shape) -> Result<sigil_build::harvest::EmpProgram, String> {
    let seams = HarnessSeams;
    let env = no_env;
    let p = s.project(aeon)?;
    driver::emp_program(&p, &s.shape, &s.options(&seams, &env)).map_err(|e| e.to_string())
}

/// The shape's residual AS side (`driver::as_side`).
pub fn as_side(aeon: &Path, s: &Shape) -> Result<sigil_build::harvest::AsSide, String> {
    let seams = HarnessSeams;
    let env = no_env;
    let p = s.project(aeon)?;
    driver::as_side(&p, &s.shape, &s.options(&seams, &env)).map_err(|e| e.to_string())
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

/// The driver's environment for a [`Shape`] build: none. A shape names a game inside the
/// checkout it is built from, so the engine is always the one observed beside the game,
/// and the build reads no checkout variable (`AEON_DIR` naming another tree, as it does
/// for a shadow copy of the reference tree, must not refuse the build).
fn no_env(_: &str) -> Option<std::ffi::OsString> {
    None
}

/// The engine struct twins' AS offsets of the checkout's engine (`<aeon>/engine/engine.toml`
/// `struct_twins` and `struct_tail_equates`), the harvest every residual AS side reads.
pub fn engine_struct_offsets(aeon: &Path) -> Result<Vec<(String, i64)>, String> {
    let dir = aeon.join("engine");
    let path = dir.join("engine.toml");
    let engine = describe::load_engine(&path).map_err(|e| e.to_string())?;
    driver::engine_struct_offsets(&engine, &dir, &path).map_err(|e| e.to_string())
}

/// The residual AS side's inputs for the shape (`driver::as_inputs`), the RAM harvest in a
/// private build directory.
pub fn as_inputs(aeon: &Path, s: &Shape) -> Result<driver::AsInputs, String> {
    let p = s.project(aeon)?;
    let (shape, defines) = driver::shape_defines(&p, &s.shape).map_err(|e| e.to_string())?;
    let dir = sigil_build::build_dir::BuildDir::create()?;
    driver::as_inputs(&p, &shape, &defines, Some(dir.generated())).map_err(|e| e.to_string())
}

/// For a standalone gate that lowers ONE sound module with no build around it: the seams'
/// sound artifacts emitted into a fresh build directory, and the shape's source roots with
/// that directory as their `generated` root, so the module's `embed("generated/sound/...")`
/// reads them and the tree is not written. Keep the [`BuildDir`](sigil_build::build_dir::BuildDir)
/// alive while lowering; dropping it removes the files.
pub fn standalone_sound_roots(
    aeon: &Path,
    s: &Shape,
) -> (sigil_build::build_dir::BuildDir, std::sync::Arc<sigil_span::roots::SourceRoots>) {
    let dir = sigil_build::build_dir::BuildDir::create().unwrap_or_else(|e| panic!("{e}"));
    let sound = dir.sound().unwrap_or_else(|e| panic!("{e}"));
    crate::native::emit_generated_in(aeon, &sound, None).unwrap_or_else(|e| panic!("emit the sound artifacts: {e}"));
    let roots = s.project(aeon).unwrap_or_else(|e| panic!("shape `{}`: {e}", s.name())).source_roots(Some(dir.generated()));
    (dir, roots)
}

/// The linker memory map the shape's build reads (its game's placement map, `game.toml` `map`).
pub fn memory_map(aeon: &Path, s: &Shape) -> Result<sigil_ir::map::MemoryMap, String> {
    let path = s.map_path(aeon)?;
    let src = sigil_span::read_set::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    sigil_link::load_map(&src).map_err(|e| format!("load {}: {e}", path.display()))
}
