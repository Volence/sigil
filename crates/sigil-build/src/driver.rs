//! The game-as-data build driver (G0b Task 5): a ROM from a game description
//! (`games/<g>/game.toml`), a shape name, and the engine description it names
//! (`engine/engine.toml`), written beside the old profile-driven driver (sigil-harness
//! `native::build_rom_chained_with_listing`), which it replaces at the cutover.
//!
//! What the descriptions decide, that the old driver hardcodes:
//!
//!   * which modules a shape places: the closed `[[gate]]` rules ([`crate::gating`],
//!     [`crate::roots`]);
//!   * the define environment: derived from the shape's features, then the engine's
//!     defaults, the game's values, the shape's overrides, the map's `[defines]`
//!     ([`merged_defines`](crate::driver::merged_defines));
//!   * section alignment: the 68000 evenness default and the `[[align]]` rows
//!     ([`crate::align`]);
//!   * where sections go: the map's `order` and its `[[anchor]]` rows, each naming the
//!     section it `holds` ([`crate::walk`]), with no frozen size table;
//!   * the deb2 appendix's engine facts (`engine.toml`) and floor (`game.toml`).
//!
//! What it does not own yet: the sound seams (the generated sound artifacts, written into
//! the build's generated root and never into the checkout, [`crate::build_dir`]; the sound
//! bank id and fold checks) and the build-data side tables live in sigil-harness and
//! reach this driver through [`Seams`](crate::driver::Seams), which sigil-cli wires. The pipeline between them
//! is the old driver's, step for step.

use std::any::Any;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use sigil_frontend_emp::export_side::{LayoutPass, ProgramLayouts, ProgramSymbols};
use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve;
use sigil_ir::{Cpu, Section, SymbolTable};
use sigil_link::export::{ExportInputs, SideTables, Z80Window};

use crate::align::AlignTable;
use crate::describe::{self, EngineDesc, GameDesc, ResolvedShape};
use crate::diag::{collect_warnings, collect_warnings_located, BuildWarning, ChainSources};
use crate::gating::ModuleId;
use crate::harvest::{helper_import_errors_in, publicize_helper_comptime, refuse_artifact_contribution, resolve_extra_entry, rewrite_helper_imports};
use crate::map_placement::{AnchorOverlay, PlacementMap};
use crate::place::ModuleSpec;
use crate::walk::{AnchorPin, WalkError};

/// Why a build was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// A module the shape could place matches no `[[gate]]` rule: the module, its source
    /// file, and the description files whose rules were searched.
    Ungated { module: ModuleId, path: PathBuf, searched: Vec<String> },
    /// A module matches two `[[gate]]` rules: the module, its source file, and the first two
    /// matching rules, each by pattern, file and row.
    DoubleGated { module: ModuleId, path: PathBuf, first: Box<RuleRef>, second: Box<RuleRef> },
    /// A gate rule could not be evaluated for the shape.
    Gate(String),
    /// A description is malformed, or the shape or engine cannot be found.
    Desc(String),
    /// The packing walk refused.
    Walk(WalkError),
    /// Any later stage, with its own message.
    Context(String),
}

/// One `[[gate]]` rule as a refusal names it: its pattern, the file it is in, and its
/// 1-based row among that file's `[[gate]]` rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRef {
    pub pattern: String,
    pub origin: String,
    pub row: usize,
}

impl RuleRef {
    pub fn of(rule: &describe::GateRule) -> RuleRef {
        RuleRef { pattern: rule.modules.clone(), origin: rule.origin.clone(), row: rule.row }
    }
}

impl std::fmt::Display for RuleRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.origin.is_empty() {
            write!(f, "`{}` (a rule built in code)", self.pattern)
        } else {
            write!(f, "`{}` ({} [[gate]] row {})", self.pattern, self.origin, self.row)
        }
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::Ungated { module, path, searched } => write!(
                f,
                "[gate.ungated] module `{module}` ({}) is placed by a map row but matches no [[gate]] \
                 rule in {}; name it in one of them (`place_in = \"all\"` if every shape places it)",
                path.display(),
                if searched.is_empty() { "any description (no rules were given)".to_string() } else { searched.join(" or ") }
            ),
            BuildError::DoubleGated { module, path, first, second } => write!(
                f,
                "[gate.double] module `{module}` ({}) matches two [[gate]] rules, {first} and {second}; \
                 one module has one rule, so narrow or delete one",
                path.display()
            ),
            BuildError::Gate(m) => write!(f, "[gate.eval] {m}"),
            BuildError::Desc(m) => write!(f, "{m}"),
            BuildError::Walk(e) => write!(f, "{e}"),
            BuildError::Context(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for BuildError {}

impl From<WalkError> for BuildError {
    fn from(e: WalkError) -> Self {
        BuildError::Walk(e)
    }
}

impl From<describe::DescError> for BuildError {
    fn from(e: describe::DescError) -> Self {
        BuildError::Desc(e.to_string())
    }
}

/// A context message (the generated-root helpers report in `String`).
impl From<String> for BuildError {
    fn from(m: String) -> Self {
        BuildError::Context(m)
    }
}

fn ctx(e: impl std::fmt::Display) -> BuildError {
    BuildError::Context(e.to_string())
}

/// The pieces that still live in sigil-harness, behind a seam this crate can name without
/// depending on it (`tests/no_harness_edge.rs`). sigil-cli passes the harness's.
pub trait Seams {
    /// A sound-on shape: write the generated sound artifacts of `checkout` into
    /// `generated`, the build's generated root directory (never into the checkout:
    /// [`crate::build_dir`]), and return the resident driver link the shape embeds
    /// (handed back to [`Seams::side_tables`]).
    fn emit_sound(
        &self,
        checkout: &Path,
        generated: &Path,
        overlay: Option<&AnchorOverlay>,
        debug: bool,
    ) -> Result<Box<dyn Any>, String>;
    /// The post-link sound checks: the bank id constants and the sound fold.
    fn validate_sound(
        &self,
        checkout: &Path,
        overlay: Option<&AnchorOverlay>,
        resolved: &[Section],
        linked: &sigil_link::LinkedImage,
        sound_on: bool,
        debug: bool,
    ) -> Result<(), String>;
    /// The build-data export's side tables.
    fn side_tables(&self, inputs: SideInputs<'_>) -> Result<SideTables, String>;
}

/// What [`Seams::side_tables`] reads.
pub struct SideInputs<'a> {
    pub emp_symbols: &'a ProgramSymbols,
    pub as_symbols: &'a sigil_frontend_as::export_side::AsSymbols,
    pub define_rows: &'a [sigil_link::ListingSymbol],
    pub resolved: &'a [Section],
    pub stubs: &'a SymbolTable,
    pub z80_windows: &'a [Z80Window],
    pub emp_layouts: &'a ProgramLayouts,
    pub code: Vec<(String, std::ops::Range<usize>)>,
    pub resident: Option<Box<dyn Any>>,
    pub rom: &'a [u8],
}

/// How to build.
pub struct BuildOptions<'a> {
    /// `--extra-entry` modules (ids or `.emp` paths).
    pub extra_entries: Vec<String>,
    /// `--anchor-overlay`.
    pub anchor_overlay: Option<AnchorOverlay>,
    pub layout_pass: LayoutPass,
    pub seams: &'a dyn Seams,
    /// The environment (the engine checkout variable); injected so a test owns it.
    pub env: &'a dyn Fn(&str) -> Option<OsString>,
    /// The build's generated root directory, when the caller owns it (`sigil build`
    /// places the build's reads with it); `None` gives the build a private
    /// [`crate::build_dir::BuildDir`] removed when the build returns.
    pub generated: Option<&'a Path>,
}

/// A built shape.
pub struct BuildOutput {
    /// The full file: the assembled image, then the deb2 appendix when the shape carries
    /// the MD Debugger island.
    pub rom: Vec<u8>,
    /// The assembled image's length (`EndOfRom`).
    pub assembled_len: usize,
    /// The assembled image as the emit wrote it (`assembled_len` bytes, the header's
    /// checksum and ROM-end fields folded over the image alone), before any appendix: the
    /// whole file of a shape without the MD Debugger island, the prefix of one with it
    /// (whose header the appendix step re-folds over the full file).
    pub image: Vec<u8>,
    pub listing: Vec<sigil_link::ListingSymbol>,
    pub export_inputs: ExportInputs,
    pub warnings: Vec<BuildWarning>,
    /// The module roots the shape placed.
    pub registry: Vec<ModuleSpec>,
    /// The engine checkout the build read.
    pub checkout: PathBuf,
    /// The shape's merged define environment ([`merged_defines`]), as the build used it:
    /// what the listing's source digest and the build export state.
    pub defines: Vec<(String, i128)>,
    /// The shape's `debug` feature.
    pub debug: bool,
}

/// A game description with the engine it names, located.
#[derive(Debug, Clone)]
pub struct Project {
    pub game_path: PathBuf,
    pub game_dir: PathBuf,
    pub game: GameDesc,
    /// The checkout holding the engine root (the module scan root, `include` root).
    pub checkout: PathBuf,
    pub engine_dir: PathBuf,
    pub engine_path: PathBuf,
    pub engine: EngineDesc,
}

impl Project {
    /// The game's module prefix (`games.sonic4`), from its `entry.game` module.
    pub fn game_prefix(&self) -> &str {
        self.game.entry.game.rsplit_once('.').map_or(self.game.entry.game.as_str(), |(p, _)| p)
    }
    /// The game's sources root.
    /// The game's source root, `sources.root` under the game's directory, spelled without
    /// `.` components: `sources.root = "."` must not put a `games/<g>/./` into every path a
    /// diagnostic renders (G0b Task 11; the digest already spelled rows without them).
    pub fn sources_root(&self) -> PathBuf {
        self.game_dir.join(&self.game.sources.root).components().filter(|c| !matches!(c, std::path::Component::CurDir)).collect()
    }
    pub fn map_path(&self) -> PathBuf {
        self.game_dir.join(&self.game.map)
    }
    /// The roots whose modules a shape's contract closure analyses (G0b Task 9a): the
    /// engine root, then the game's sources root, canonical. Another game's sources, and a
    /// fixture outside both (aeon's `tools/fixtures/`), are not this game's build.
    pub fn closure_roots(&self) -> Vec<PathBuf> {
        [self.engine_dir.clone(), self.sources_root()].into_iter().map(|d| std::fs::canonicalize(&d).unwrap_or(d)).collect()
    }
    /// The roots a build scans for modules, named (G0b Task 9): the engine root, then the
    /// game's sources root. The same two directories [`closure_roots`](Self::closure_roots)
    /// analyses, so the modules a build may place and the modules its contract gate
    /// analyses come from one scan rule.
    pub fn scan_roots(&self) -> Vec<(sigil_span::roots::RootName, PathBuf)> {
        vec![(sigil_span::roots::RootName::Engine, self.engine_dir.clone()), (sigil_span::roots::RootName::Game, self.sources_root())]
    }
    /// The build's module corpus: every `*.emp` under [`scan_roots`](Self::scan_roots)
    /// and nothing else in the checkout (another game's sources, aeon's
    /// `tools/fixtures/`), each module tagged with its root.
    pub fn scan(&self) -> (resolve::manifest::Manifest, Vec<sigil_span::Diagnostic>) {
        resolve::manifest::Manifest::scan_roots(&self.scan_roots())
    }
    /// The build's named roots (G0b Task 9, [`sigil_span::roots`]): `engine` is the
    /// engine's root, `game` the game's directory, `generated` the build's own generated
    /// directory when the build has one. A game inside the engine's checkout (aeon's own
    /// `games/*`) also gets that checkout as the transitional tree its unrooted paths
    /// resolve against; a game in its own repository gets none, so an unrooted path is
    /// refused by name there.
    pub fn source_roots(&self, generated: Option<&Path>) -> std::sync::Arc<sigil_span::roots::SourceRoots> {
        let mut r = sigil_span::roots::SourceRoots::new(self.engine_dir.clone(), self.game_dir.clone());
        if let Some(g) = generated {
            r = r.with_generated(g.to_path_buf());
        }
        if self.game_dir.starts_with(&self.checkout) {
            r = r.with_tree(self.checkout.clone());
        }
        std::sync::Arc::new(r)
    }
}

/// Does `path` (a scanned module's file) sit under one of `roots` (canonical directories)?
pub fn under_roots(path: &Path, roots: &[PathBuf]) -> bool {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    roots.iter().any(|r| path.starts_with(r))
}

/// The `pub const` names `items` declare, sections included.
fn declared_pub_consts<'a>(items: &'a [sigil_frontend_emp::ast::Item], out: &mut std::collections::BTreeSet<&'a str>) {
    use sigil_frontend_emp::ast::Item;
    for item in items {
        match item {
            Item::Const(c) if c.public => {
                out.insert(c.name.as_str());
            }
            Item::Section(sec) => declared_pub_consts(&sec.items, out),
            _ => {}
        }
    }
}

/// Refuse a placed module that the contract closure does not analyse (G0b Task 9:
/// placed ⊆ analysed). The closure analyses the modules under
/// [`Project::closure_roots`] ([`under_roots`], the predicate the gate's corpus uses); a
/// module a shape places from anywhere else would ship code no contract ever checked.
/// The build scans exactly those roots ([`Project::scan`]), so in a sound build this
/// never fires; it is the check that ties the two scopes together, so that widening
/// either one alone is refused by name instead of shipping silently.
pub fn check_placed_in_analysed(
    placed: &[ModuleSpec],
    manifest: &resolve::manifest::Manifest,
    analysed_roots: &[PathBuf],
) -> Result<(), BuildError> {
    let mut outside: Vec<String> = Vec::new();
    for spec in placed {
        let row = match manifest.by_id.get(&spec.module_id) {
            Some(&i) => {
                let m = &manifest.modules[i];
                if m.root.is_some() && under_roots(&m.path, analysed_roots) {
                    continue;
                }
                format!("`{}` ({})", m.id, m.path.display())
            }
            None => format!("`{}` (in no scanned file)", spec.module_id),
        };
        if !outside.contains(&row) {
            outside.push(row);
        }
    }
    if outside.is_empty() {
        return Ok(());
    }
    Err(BuildError::Context(format!(
        "[gate.placed-not-analysed] {} placed module(s) lie outside the roots the contract closure analyses ({}), \
         so they would ship unchecked:\n  - {}",
        outside.len(),
        analysed_roots.iter().map(|r| r.display().to_string()).collect::<Vec<_>>().join(", "),
        outside.join("\n  - ")
    )))
}

/// Refuse a game that does not declare every constant the engine description's
/// `required_consts` names (G0b Task 9a), naming each missing constant, the engine.toml row
/// and the game. A constant is declared when a module in the game's sources root holds a
/// `pub const` of that name. There is no default: the engine names these by bare name
/// (aeon's `Sfx_BeginSound` reads `SFXID_REV_LOOP`), so a game that leaves one out would
/// otherwise reach the contract closure as an unresolved condition, or resolve only because
/// another game's sources happen to be scanned.
pub fn check_required_consts(p: &Project, manifest: &resolve::manifest::Manifest) -> Result<(), BuildError> {
    if p.engine.required_consts.is_empty() {
        return Ok(());
    }
    // No engine default (G0b Task 9): a required constant is the GAME's. An engine module
    // that declares one would make every game's declaration a redeclaration, or let a
    // game that forgot it build on the engine's value; neither may pass silently.
    let engine_root = std::fs::canonicalize(&p.engine_dir).unwrap_or_else(|_| p.engine_dir.clone());
    let mut defaults: Vec<String> = Vec::new();
    for m in manifest.modules.iter().filter(|m| under_roots(&m.path, std::slice::from_ref(&engine_root))) {
        let mut names = std::collections::BTreeSet::new();
        declared_pub_consts(&m.file.items, &mut names);
        for c in p.engine.required_consts.iter().filter(|c| names.contains(c.as_str())) {
            defaults.push(format!("`{c}` in `{}` ({})", m.id, m.path.display()));
        }
    }
    if !defaults.is_empty() {
        return Err(BuildError::Desc(format!(
            "[engine.required-const-default] the engine declares {} as a `pub const`; {} `required_consts` names it a \
             constant every game declares, with no engine default, so the engine may not declare it",
            defaults.join(", "),
            p.engine_path.display(),
        )));
    }
    let root = p.sources_root();
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    let mut declared = std::collections::BTreeSet::new();
    for m in manifest.modules.iter().filter(|m| under_roots(&m.path, std::slice::from_ref(&root))) {
        declared_pub_consts(&m.file.items, &mut declared);
    }
    let missing: Vec<&str> =
        p.engine.required_consts.iter().map(String::as_str).filter(|c| !declared.contains(c)).collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(BuildError::Desc(format!(
        "[engine.required-const] game `{}` ({}) declares no `pub const` named {} in its sources ({}); \
         {} `required_consts` requires every game on this engine to declare it, with no default",
        p.game.id,
        p.game_path.display(),
        missing.iter().map(|c| format!("`{c}`")).collect::<Vec<_>>().join(", "),
        root.display(),
        p.engine_path.display(),
    )))
}

/// Read `project` (a game directory holding `game.toml`, or the file) and find its engine.
/// A game that does not declare every constant the engine's `required_consts` names is
/// refused here ([`check_required_consts`]), so every `--project` run (build, check, each
/// report, the closure gate) refuses it by name before anything reads the name.
///
/// The engine is found by observation first: the nearest ancestor of the game directory
/// that holds `<engine.root>/engine.toml` is the game's own checkout. Only a game with no
/// such ancestor falls back to SUITE_PATHS ([`describe::resolve_engine_dir`]). When the
/// checkout variable is set and names a different tree than the one observed, the build is
/// refused: two answers to "which engine" is a fork, and neither may silently win.
pub fn load_project(project: &Path, env: &dyn Fn(&str) -> Option<OsString>) -> Result<Project, BuildError> {
    let game_path = if project.is_dir() { project.join("game.toml") } else { project.to_path_buf() };
    let game = describe::load_game(&game_path)?;
    let game_dir = game_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let game_dir = std::fs::canonicalize(&game_dir)
        .map_err(|e| BuildError::Desc(format!("game directory {}: {e}", game_dir.display())))?;
    let observed = game_dir.ancestors().find(|a| a.join(&game.engine.root).join("engine.toml").is_file()).map(Path::to_path_buf);
    let var = format!("{}_DIR", game.engine.suite_path.to_ascii_uppercase());
    let checkout = match observed {
        Some(dir) => {
            if let Some(v) = env(&var).filter(|v| !v.is_empty()) {
                let named = std::fs::canonicalize(&v).unwrap_or_else(|_| PathBuf::from(&v));
                if named != dir {
                    return Err(BuildError::Desc(format!(
                        "[engine.conflict] {var} names {}, but the game {} sits in the checkout {}; \
                         build the game with the engine beside it, or unset {var}",
                        named.display(),
                        game_path.display(),
                        dir.display()
                    )));
                }
            }
            dir
        }
        None => describe::resolve_engine_dir(&game.engine, env)?.0,
    };
    let engine_dir = checkout.join(&game.engine.root);
    let engine_path = engine_dir.join("engine.toml");
    let engine = describe::load_engine(&engine_path)?;
    let project = Project { game_path, game_dir, game, checkout, engine_dir, engine_path, engine };
    if !project.engine.required_consts.is_empty() {
        // Description time, before any front end or the closure reads a required name: the
        // scan's own errors are reported by the stage that uses it.
        let (manifest, sdiags) = project.scan();
        if let Some(e) = sdiags.iter().find(|d| d.level == sigil_span::Level::Error) {
            return Err(BuildError::Desc(format!("[engine.required-const] the module scan failed before the required constants could be checked: {}", e.message)));
        }
        check_required_consts(&project, &manifest)?;
    }
    Ok(project)
}

/// The shape's define environment, in order: the three the shape's features DERIVE
/// (`SOUND_DRIVER_ENABLED`, `DEBUG`, `CRASH_REPORT`), the engine's defaults, the game's
/// values, each overridden by the shape where it says so, then the map's `[defines]` rows.
///
/// Refused: a description that declares a derived name; a game define that redeclares an
/// engine one; a shape override of a name neither description declares (a typo there
/// would otherwise mint a define silently); a map row that redeclares any earlier one.
pub fn merged_defines(p: &Project, shape: &ResolvedShape, map_src: &str) -> Result<Vec<(String, i128)>, BuildError> {
    let f = shape.features;
    let derived: [(&str, i64); 3] =
        [("SOUND_DRIVER_ENABLED", f.sound as i64), ("DEBUG", f.debug as i64), ("CRASH_REPORT", f.crash_report as i64)];
    let is_derived = |k: &str| derived.iter().any(|(d, _)| *d == k);
    let eng = &p.engine.defines;
    let game = &p.game.defines;
    for k in eng.keys() {
        if is_derived(k) {
            return Err(BuildError::Desc(format!(
                "[define.derived] {} declares `{k}`, which the shape's features derive",
                p.engine_path.display()
            )));
        }
    }
    for k in game.keys() {
        if is_derived(k) || eng.contains_key(k) {
            return Err(BuildError::Desc(format!(
                "[define.redeclared] {} declares `{k}`, which {} already declares; a game sets an \
                 engine define per shape, it does not redeclare it",
                p.game_path.display(),
                if is_derived(k) { "the shape's features derive".to_string() } else { p.engine_path.display().to_string() }
            )));
        }
    }
    let mut out: BTreeMap<&str, i64> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    for (k, v) in derived {
        order.push(k.to_string());
        out.insert(k, v);
    }
    for (k, v) in eng.iter().chain(game.iter()) {
        order.push(k.clone());
        out.insert(k, *v);
    }
    for (k, v) in &shape.defines {
        if !out.contains_key(k.as_str()) || is_derived(k) {
            return Err(BuildError::Desc(format!(
                "[define.undeclared-override] shape `{}` sets `{k}`, which neither {} nor {} declares \
                 (or which the shape's features derive); declare it with its default first",
                shape.name,
                p.engine_path.display(),
                p.game_path.display()
            )));
        }
        out.insert(k, *v);
    }
    let mut env: Vec<(String, i128)> = order.iter().map(|k| (k.clone(), out[k.as_str()] as i128)).collect();
    let origin = p.map_path().display().to_string();
    for (k, v) in crate::game_defines::parse_game_defines(map_src, &origin).map_err(ctx)? {
        if env.iter().any(|(e, _)| *e == k) {
            return Err(BuildError::Desc(format!(
                "[define.redeclared] {origin} [defines] declares `{k}`, which the descriptions already declare"
            )));
        }
        env.push((k, v));
    }
    Ok(env)
}

// ───────────────────────────── the AS side ─────────────────────────────

fn parse_emp(path: &Path, what: &str) -> Result<sigil_frontend_emp::ast::File, BuildError> {
    let src = sigil_span::read_set::read_to_string(path).map_err(|e| ctx(format!("{what}: read {}: {e}", path.display())))?;
    let (file, d) = sigil_frontend_emp::parse_str(&src);
    if let Some(e) = d.iter().find(|d| d.level == sigil_span::Level::Error) {
        return Err(ctx(format!("{what}: parse {}: {e:?}", path.display())));
    }
    Ok(file)
}

fn pub_consts(p: &Project, path: &Path, seed: &[(String, i128)], what: &str) -> Result<Vec<(String, i64)>, BuildError> {
    let file = parse_emp(path, what)?;
    // A constants module embeds nothing generated: no generated root, so a `generated/`
    // path here is refused by name rather than read from anywhere.
    let sandbox = sigil_frontend_emp::Sandbox::Roots(p.source_roots(None));
    let (vals, d) = sigil_frontend_emp::eval::eval_all_pub_consts(&file, Some(&sandbox), seed);
    if let Some(e) = d.iter().find(|d| d.level == sigil_span::Level::Error) {
        return Err(ctx(format!("{what}: resolve {}: {e:?}", path.display())));
    }
    Ok(vals)
}

/// The engine's public constants (the AS side's guarded defines), evaluated in `defines`,
/// less every name a struct twin also produces (`twins`, [`struct_offsets`]): the twin is
/// the AS side's spelling of that name (aeon: `VDP_Shadow_len`, a `pub const` of the
/// constants module and the `VdpShadow` twin's `<prefix>_len`).
fn engine_constants(p: &Project, defines: &[(String, i128)], twins: &[(String, i64)]) -> Result<Vec<(String, i64)>, BuildError> {
    let vals = pub_consts(p, &p.engine_dir.join(&p.engine.constants), defines, "engine constants")?;
    let twin_names: std::collections::HashSet<&str> = twins.iter().map(|(n, _)| n.as_str()).collect();
    Ok(vals.into_iter().filter(|(n, _)| !twin_names.contains(n.as_str())).collect())
}

/// The engine structs' field offsets, under the AS spelling `<prefix>_<field>` and
/// `<prefix>_len`, for each `struct_twins` row, each twin followed by the
/// `struct_tail_equates` of its struct (`name = sizeof - from_end`).
pub fn struct_offsets(p: &Project) -> Result<Vec<(String, i64)>, BuildError> {
    engine_struct_offsets(&p.engine, &p.engine_dir, &p.engine_path)
}

/// [`struct_offsets`] of an engine description alone (`engine_dir` holds `engine_path`,
/// the description).
pub fn engine_struct_offsets(engine: &EngineDesc, engine_dir: &Path, engine_path: &Path) -> Result<Vec<(String, i64)>, BuildError> {
    for t in &engine.struct_tail_equates {
        if !engine.struct_twins.iter().any(|(_, sname, _)| *sname == t.of) {
            return Err(BuildError::Desc(format!(
                "{}: `struct_tail_equates` row `{}` names struct `{}`, which no `struct_twins` row lays out",
                engine_path.display(),
                t.name,
                t.of
            )));
        }
    }
    let types = parse_emp(&engine_dir.join(&engine.types), "struct offsets")?;
    let mut out = Vec::new();
    for (rel, sname, prefix) in &engine.struct_twins {
        let file = parse_emp(&engine_dir.join(rel), "struct offsets")?;
        let (layout, d) = sigil_frontend_emp::layout::layout_struct_ambient(&file, &types.items, sname);
        if let Some(e) = d.iter().find(|d| d.level == sigil_span::Level::Error) {
            return Err(ctx(format!("struct offsets: layout {sname}: {e:?}")));
        }
        let layout = layout.ok_or_else(|| ctx(format!("struct offsets: no struct `{sname}` in {rel}")))?;
        for f in &layout.fields {
            out.push((format!("{prefix}_{}", f.name), f.offset as i64));
        }
        out.push((format!("{prefix}_len"), layout.size as i64));
        for t in engine.struct_tail_equates.iter().filter(|t| t.of == *sname) {
            if t.from_end as u64 > layout.size as u64 {
                return Err(BuildError::Desc(format!(
                    "{}: `struct_tail_equates` row `{}` reaches {} bytes back from the end of `{sname}`, which is {} bytes",
                    engine_path.display(),
                    t.name,
                    t.from_end,
                    layout.size
                )));
            }
            out.push((t.name.clone(), layout.size as i64 - t.from_end as i64));
        }
    }
    Ok(out)
}

/// The RAM labels and equates the engine RAM and the game RAM place (the AS side's plain
/// defines), from a focused build of those two modules.
fn ram_addresses(p: &Project, ram_seed: &str, defines: &[(String, i128)], generated: Option<&Path>) -> Result<Vec<(String, i64)>, BuildError> {
    let (mut manifest, md) = p.scan();
    if let Some(e) = md.iter().find(|d| d.level == sigil_span::Level::Error) {
        return Err(ctx(format!("ram harvest: manifest scan: {e:?}")));
    }
    let helpers: Vec<&str> = p.engine.ram_helpers.iter().map(String::as_str).collect();
    publicize_helper_comptime(&mut manifest, &helpers);
    let entry_id = "__ram_harvest_entry__".to_string();
    let src = format!("module __ram_harvest_entry__\n\nuse {ram_seed}\nuse {}\n", p.game.entry.ram);
    push_synthetic(&mut manifest, &entry_id, &src, &p.checkout)?;
    let opts = lower_options(p, defines, generated);
    let (sections, _, bd) = resolve::build_program_open_embed(&manifest, &entry_id, None, &opts, &|_| None);
    if let Some(e) = bd.iter().find(|d| d.level == sigil_span::Level::Error) {
        return Err(ctx(format!("ram harvest: build: {e:?}")));
    }
    let mut out = Vec::new();
    for sec in sections.iter().filter(|s| s.vma_origin() >= p.engine.ram_floor) {
        for l in &sec.labels {
            out.push((l.name.clone(), sec.vma_origin().wrapping_add(l.offset) as i64));
        }
        for e in &sec.equ_syms {
            if let sigil_ir::expr::Expr::Int(v) = e.expr {
                out.push((e.name.clone(), v));
            }
        }
    }
    if out.is_empty() {
        return Err(ctx("ram harvest: no RAM labels found (the RAM modules are not reachable?)"));
    }
    Ok(out)
}

/// The lowering options of every `.emp` lowering a build makes: the shape's defines and
/// the build's named roots ([`Project::source_roots`]), with `generated` the build's
/// generated root when it has one.
fn lower_options(p: &Project, defines: &[(String, i128)], generated: Option<&Path>) -> LowerOptions {
    LowerOptions {
        roots: Some(p.source_roots(generated)),
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: defines.to_vec(),
    }
}

fn push_synthetic(manifest: &mut resolve::manifest::Manifest, id: &str, src: &str, checkout: &Path) -> Result<sigil_span::SourceId, BuildError> {
    let source = sigil_span::SourceId(manifest.modules.len() as u32);
    if manifest.sources.contains_key(&source) {
        return Err(ctx(format!("synthetic entry {source:?} collides with a scanned module")));
    }
    let (file, pd) = sigil_frontend_emp::parse_file(src, source);
    if let Some(e) = pd.iter().find(|d| d.level == sigil_span::Level::Error) {
        return Err(ctx(format!("synthetic entry `{id}` parse: {e:?}")));
    }
    let path = checkout.join(format!("__{id}__.emp"));
    manifest.by_id.insert(id.to_string(), manifest.modules.len());
    manifest.sources.insert(source, path.clone());
    manifest.modules.push(resolve::manifest::ParsedModule { root: None, id: id.to_string(), file, path });
    Ok(source)
}

/// What the residual AS side is assembled with, each harvest by name: the guarded defines
/// are `engine_constants`, then `struct_offsets`, then `game_constants`; the plain defines
/// are `ram_addresses`, then `feature_defines`.
pub struct AsInputs {
    /// The engine's public constants ([`EngineDesc::constants`]), less the struct twins' names.
    pub engine_constants: Vec<(String, i64)>,
    /// The struct twins' field offsets and sizes, each followed by its tail equates.
    pub struct_offsets: Vec<(String, i64)>,
    /// The game's public constants (its `entry` constants, sound ids and SFX bank modules).
    pub game_constants: Vec<(String, i64)>,
    /// The RAM labels and equates the engine RAM and the game RAM place.
    pub ram_addresses: Vec<(String, i64)>,
    /// The shape's `as_feature_defines`.
    pub feature_defines: Vec<(String, i64)>,
}

/// The residual AS side's inputs for `shape` under `defines` (see [`AsInputs`]).
pub fn as_inputs(p: &Project, shape: &ResolvedShape, defines: &[(String, i128)], generated: Option<&Path>) -> Result<AsInputs, BuildError> {
    let seeds = entry_seeds(p)?;
    let struct_offsets = struct_offsets(p)?;
    let engine_constants = engine_constants(p, defines, &struct_offsets)?;
    let mut seed: Vec<(String, i128)> = engine_constants.iter().map(|(n, v)| (n.clone(), *v as i128)).collect();
    seed.push(("DEBUG".to_string(), shape.features.debug as i128));
    let e = &p.game.entry;
    let mut game_constants = Vec::new();
    for rel in [&e.constants, &e.sound_ids, &e.sfx_bank].into_iter().flatten() {
        game_constants.extend(pub_consts(p, &p.game_dir.join(rel), &seed, "game constants")?);
    }
    let ram_addresses = ram_addresses(p, &seeds[2], defines, generated)?;
    let feature_defines = as_feature_defines(p, &shape.features, defines)?;
    Ok(AsInputs { engine_constants, struct_offsets, game_constants, ram_addresses, feature_defines })
}

/// The residual AS side, assembled with the defines the descriptions derive.
fn assemble_as(p: &Project, shape: &ResolvedShape, defines: &[(String, i128)], generated: &Path) -> Result<crate::harvest::AsSide, BuildError> {
    let AsInputs { engine_constants, struct_offsets, game_constants, ram_addresses, feature_defines } =
        as_inputs(p, shape, defines, Some(generated))?;
    let mut guarded = engine_constants;
    guarded.extend(struct_offsets);
    guarded.extend(game_constants);
    let mut plain = ram_addresses;
    plain.extend(feature_defines);
    let opts = sigil_frontend_as::Options {
        initial_cpu: Some(Cpu::M68000),
        defines: plain,
        include_root: None,
        roots: Some(p.source_roots(Some(generated))),
        guarded_defines: guarded,
        cli_defines: Vec::new(),
        share_file: false,
    };
    let root = p.sources_root().join(&p.game.sources.residual_as);
    sigil_frontend_as::assemble_root_relocating_warned(&root, &opts)
        .map(|a| crate::harvest::AsSide {
            warnings: a.warnings.iter().map(|d| BuildWarning::from_as(d, &a.sources)).collect(),
            module: a.module,
            sources: a.sources,
        })
        .map_err(|f| ctx(format!("assemble {}: {} diagnostic(s); first: {:?}", root.display(), f.diags.len(), f.diags.first())))
}

/// The residual AS side's feature defines for a shape: each `as_feature_defines` row whose
/// `when` holds for `features` (a define it reads is looked up in `defines`), as `name = 1`,
/// in description order.
fn as_feature_defines(p: &Project, features: &describe::Features, defines: &[(String, i128)]) -> Result<Vec<(String, i64)>, BuildError> {
    let mut out = Vec::new();
    for d in &p.engine.as_feature_defines {
        let on = d.when.eval(features, &|n| defines.iter().find(|(k, _)| k == n).map(|(_, v)| *v as i64)).map_err(|e| {
            BuildError::Desc(format!("{}: `as_feature_defines` row `{}`: {e}", p.engine_path.display(), d.name))
        })?;
        if on {
            out.push((d.name.clone(), 1));
        }
    }
    Ok(out)
}

// ───────────────────────────── the .emp side ─────────────────────────────

struct EmpSide {
    program: crate::harvest::EmpProgram,
}

/// The `.emp` program a shape lowers, ready to lower: the scanned manifest with the
/// helper imports rewritten and the synthetic entry pushed, and what the build reports
/// against it.
struct PreparedEmp {
    manifest: resolve::manifest::Manifest,
    mdiags: Vec<sigil_span::Diagnostic>,
    removed: Vec<(usize, sigil_span::Diagnostic)>,
    registry: Vec<ModuleSpec>,
    source: sigil_span::SourceId,
    entry_id: &'static str,
    lopts: LowerOptions,
}

fn prepare_emp(
    p: &Project,
    gate_shape: &ResolvedShape,
    order: &[String],
    defines: &[(String, i128)],
    opts: &BuildOptions<'_>,
    generated: Option<&Path>,
) -> Result<PreparedEmp, BuildError> {
    let (mut manifest, mdiags) = p.scan();
    if let Some(e) = mdiags.iter().find(|d| d.level == sigil_span::Level::Error) {
        return Err(ctx(format!("manifest scan: {e:?}")));
    }
    let seeds = entry_seeds(p)?;
    let rules: Vec<describe::GateRule> = p.engine.gate.iter().chain(p.game.gate.iter()).cloned().collect();
    let map_name = p.map_path().display().to_string();
    let registry = crate::roots::derive_roots(&crate::roots::RootsInput {
        manifest: &manifest,
        order,
        game_prefix: p.game_prefix(),
        rules: &rules,
        shape: gate_shape,
        seeds: &seeds,
        terminus: &p.engine.terminus,
        map_name: &map_name,
    })?;
    check_placed_in_analysed(&registry, &manifest, &p.closure_roots())?;
    let helpers: Vec<&str> = p.engine.helpers.iter().map(String::as_str).collect();
    let removed = rewrite_helper_imports(&mut manifest, &helpers);
    let extra: Vec<String> = opts
        .extra_entries
        .iter()
        .map(|arg| {
            let id = resolve_extra_entry(&manifest, &p.checkout, arg)?;
            refuse_artifact_contribution(&manifest, &id, arg)?;
            Ok(id)
        })
        .collect::<Result<_, String>>()
        .map_err(ctx)?;
    let entry_id = "native_flip_entry";
    let mut src = format!("module {entry_id}\n\n");
    for s in seeds.iter().map(String::as_str).chain(registry.iter().map(|m| m.module_id.as_str())).chain(extra.iter().map(String::as_str)) {
        src.push_str(&format!("use {s}\n"));
    }
    let source = push_synthetic(&mut manifest, entry_id, &src, &p.checkout)?;
    let lopts = lower_options(p, defines, generated);
    Ok(PreparedEmp { manifest, mdiags, removed, registry, source, entry_id, lopts })
}

fn build_emp(
    p: &Project,
    gate_shape: &ResolvedShape,
    order: &[String],
    defines: &[(String, i128)],
    opts: &BuildOptions<'_>,
    generated: &Path,
) -> Result<EmpSide, BuildError> {
    let PreparedEmp { manifest, mdiags, removed, registry, source, entry_id, lopts } =
        prepare_emp(p, gate_shape, order, defines, opts, Some(generated))?;
    let resolve::BuiltProgram { mut sections, link_asserts, comptime_guards, diags: mut bdiags, lowered, symbols, layouts, expansions } =
        resolve::build_program_open_embed_counted(&manifest, entry_id, None, &lopts, opts.layout_pass, &|_| None);
    bdiags.extend(helper_import_errors_in(&manifest, &removed, &lowered, &bdiags));
    let berr: Vec<_> = bdiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    if !berr.is_empty() {
        let index = resolve::manifest::SourceIndex::with_expansions(&manifest, &expansions);
        return Err(ctx(format!(
            "build_program: {} error(s);\n{}",
            berr.len(),
            crate::diag::render::render_diag_lines(&berr, &|span| index.locate(span))
        )));
    }
    if p.game.require_one_text {
        let n = sections.iter().filter(|s| s.name == "text" && !s.image_bytes().is_empty()).count();
        if n != 1 {
            return Err(ctx(format!("expected exactly 1 non-empty `text` section (game.toml require_one_text), found {n}")));
        }
    }
    let map = sigil_link::load_map(&crate::place::emp_map_frozen(&sections)).map_err(|e| ctx(format!("emp map load: {e}")))?;
    let place_diags = resolve::place_sections(&mut sections, &map);
    if let Some(e) = place_diags.iter().find(|d| d.level == sigil_span::Level::Error) {
        return Err(ctx(format!("place_sections: {e:?}")));
    }
    let index = resolve::manifest::SourceIndex::with_expansions(&manifest, &expansions);
    let pdiags: Vec<sigil_span::Diagnostic> = Vec::new();
    let warnings = collect_warnings(&index, &[&mdiags, &pdiags, &bdiags, &place_diags], Some(source));
    Ok(EmpSide {
        program: crate::harvest::EmpProgram { sections, link_asserts, comptime_guards, warnings, sources: index, registry, symbols, layouts },
    })
}

/// The synthetic entry's leading `use`s: the engine's `entry_seeds` paired with the game's
/// entry modules (seed 0, `entry.game`, seed 1, `entry.ram`).
fn entry_seeds(p: &Project) -> Result<Vec<String>, BuildError> {
    match p.engine.entry_seeds.as_slice() {
        [contract, ram] => Ok(vec![contract.clone(), p.game.entry.game.clone(), ram.clone(), p.game.entry.ram.clone()]),
        other => Err(BuildError::Desc(format!(
            "{}: `entry_seeds` must name two modules (the game contract, then the engine RAM), found {}",
            p.engine_path.display(),
            other.len()
        ))),
    }
}

// ───────────────────────────── the build ─────────────────────────────

/// Build `shape` of the game at `project`.
pub fn build(project: &Path, shape: &str, opts: &BuildOptions<'_>) -> Result<BuildOutput, BuildError> {
    let p = load_project(project, opts.env)?;
    build_project(&p, shape, opts)
}

/// [`build`] over a loaded [`Project`]. The build reads the checkout and writes what it
/// generates into its generated root: [`BuildOptions::generated`] when the caller owns
/// it, otherwise a private build directory removed when the build returns.
pub fn build_project(p: &Project, shape_name: &str, opts: &BuildOptions<'_>) -> Result<BuildOutput, BuildError> {
    match with_generated(p, opts, |generated| build_project_in(p, shape_name, opts, generated, Run::Build))? {
        Outcome::Built(out) => Ok(*out),
        Outcome::Checked(_) => unreachable!("Run::Build returns Outcome::Built"),
    }
}

/// The shape's lowered `.emp` program, as [`build_project`] lowers it before placement
/// (the sound artifacts emitted into the build's generated root first on a sound-on
/// shape): every placed `.emp` section, the deferred link asserts, the warnings and the
/// location authority. For a gate or a port test that reads one section of the real
/// program without linking the ROM.
pub fn emp_program(p: &Project, shape_name: &str, opts: &BuildOptions<'_>) -> Result<crate::harvest::EmpProgram, BuildError> {
    with_generated(p, opts, |generated| {
        let Plan { shape, defines, gate_shape, pmap, .. } = plan(p, shape_name, opts)?;
        if shape.features.sound {
            let sound_dir = crate::build_dir::sound_dir(generated).map_err(ctx)?;
            opts.seams.emit_sound(&p.checkout, &sound_dir, opts.anchor_overlay.as_ref(), shape.features.debug).map_err(ctx)?;
        }
        Ok(build_emp(p, &gate_shape, &pmap.order, &defines, opts, generated)?.program)
    })
}

/// The shape's residual AS side, assembled with the defines the descriptions derive (the
/// first front end [`build_project`] runs).
pub fn as_side(p: &Project, shape_name: &str, opts: &BuildOptions<'_>) -> Result<crate::harvest::AsSide, BuildError> {
    with_generated(p, opts, |generated| {
        let Plan { shape, defines, .. } = plan(p, shape_name, opts)?;
        if shape.features.sound {
            let sound_dir = crate::build_dir::sound_dir(generated).map_err(ctx)?;
            opts.seams.emit_sound(&p.checkout, &sound_dir, opts.anchor_overlay.as_ref(), shape.features.debug).map_err(ctx)?;
        }
        assemble_as(p, &shape, &defines, generated)
    })
}

/// What `sigil build --project ... --check` reports: the guard census of a resolve that
/// stopped before the link, and the warn tier it raised.
pub struct CheckOutput {
    pub guards: crate::guards::GuardCensus,
    pub warnings: Vec<BuildWarning>,
}

/// Decide every `ensure` and `LinkAssert` of `shape_name` against the final
/// post-relaxation placement, then stop: [`build_project`]'s pipeline step for step
/// (the sound artifacts into the generated root for a sound-on shape, the residual AS,
/// the `.emp` lowering, the walk, `resolve_layout`, the link-assert tally, the drift
/// verdict, the inapplicable allowlist, the census) and nothing after it: no link, no
/// listing, no appendix, no checksum, no ROM. The profile-driven driver's `--check`
/// (sigil-harness `native::check_chained`) is the same cut of its own pipeline.
pub fn check_project(p: &Project, shape_name: &str, opts: &BuildOptions<'_>) -> Result<CheckOutput, BuildError> {
    match with_generated(p, opts, |generated| build_project_in(p, shape_name, opts, generated, Run::Check))? {
        Outcome::Checked(out) => Ok(out),
        Outcome::Built(_) => unreachable!("Run::Check returns Outcome::Checked"),
    }
}

/// Run `f` with the build's generated root directory: the caller's
/// ([`BuildOptions::generated`], refused when it lies inside the checkout), else a fresh
/// private one that lives exactly as long as `f`.
fn with_generated<T>(p: &Project, opts: &BuildOptions<'_>, f: impl FnOnce(&Path) -> Result<T, BuildError>) -> Result<T, BuildError> {
    if let Some(g) = opts.generated {
        crate::build_dir::check_outside(&p.checkout, g).map_err(ctx)?;
        return f(g);
    }
    let dir = crate::build_dir::BuildDir::create().map_err(ctx)?;
    f(dir.generated())
}

/// How far [`build_project_in`] runs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Run {
    Build,
    Check,
}

enum Outcome {
    Built(Box<BuildOutput>),
    Checked(CheckOutput),
}

fn build_project_in(p: &Project, shape_name: &str, opts: &BuildOptions<'_>, generated: &Path, run: Run) -> Result<Outcome, BuildError> {
    let Plan { shape, defines, gate_shape, map, pmap } = plan(p, shape_name, opts)?;
    let sound = shape.features.sound;
    let debug = shape.features.debug;
    let map_path = p.map_path();
    let mut pins: Vec<AnchorPin> = Vec::new();
    for a in pmap.anchors_for(sound) {
        let holds = a.holds.clone().ok_or_else(|| {
            BuildError::Desc(format!(
                "[map.anchor-holds-missing] {} anchor `{}` at {:#x} names no `holds` row; the driver \
                 places the section an anchor holds, so each anchor in force must say which",
                map_path.display(),
                a.name,
                a.at
            ))
        })?;
        pins.push(AnchorPin { name: a.name.clone(), holds, at: a.at });
    }
    let engine_origin = p.engine_path.display().to_string();
    let game_origin = p.game_path.display().to_string();
    let align = AlignTable::new([(engine_origin.as_str(), &p.engine.align[..]), (game_origin.as_str(), &p.game.align[..])]).map_err(ctx)?;

    // ── Front ends. ──
    let resident = if sound {
        let sound_dir = crate::build_dir::sound_dir(generated).map_err(ctx)?;
        Some(opts.seams.emit_sound(&p.checkout, &sound_dir, opts.anchor_overlay.as_ref(), debug).map_err(ctx)?)
    } else {
        None
    };
    let crate::harvest::AsSide { module: as_module, warnings: mut as_warnings, sources: as_map } =
        assemble_as(p, &shape, &defines, generated)?;
    let EmpSide { program } = build_emp(p, &gate_shape, &pmap.order, &defines, opts, generated)?;
    let crate::harvest::EmpProgram { sections: emp_sections, link_asserts, comptime_guards, mut warnings, sources, registry, symbols: emp_symbols, layouts: emp_layouts } = program;
    let as_symbols = sigil_frontend_as::export_side::AsSymbols::from_sections(&as_module.sections);
    let as_code = as_module.named_code_marks();
    let (sources, sections) = ChainSources::join(sources, as_map, as_module.sections, emp_sections).map_err(ctx)?;
    for w in &mut as_warnings {
        sources.rebase_warning(w);
    }
    warnings.extend(as_warnings);
    let island = declares_island(p, &shape, &registry)?;

    // ── Place. ──
    let true_bases = crate::walk::true_bases(&sections, &pmap.order, &pins, &align, &|s| sources.locate(s))?;
    let spans = crate::place::declared_spans_by_index(&sections, &true_bases).map_err(ctx)?;
    let all = crate::place::apply_declared_chain(sections, &true_bases, &spans);
    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout_owned(all, &stubs, true)
        .map_err(|d| ctx(crate::guards::render_declared_chain("resolve_layout", &d, &sources)))?;
    let (adiags, tally) = sigil_link::check_link_asserts_tallied(&resolved, &stubs, &link_asserts);
    warnings.extend(collect_warnings_located(&|s| sources.locate(s), &[&adiags], None));
    let inapplicable = crate::guards::declared_chain_drift_verdict(&adiags, &|s| sources.locate(s)).map_err(ctx)?;
    crate::guards::enforce_inapplicable_allowlist_against(&inapplicable, &link_asserts, &[]).map_err(ctx)?;
    let census = crate::guards::GuardCensus::from_verdict(comptime_guards, &tally, &inapplicable).map_err(ctx)?;
    if run == Run::Check {
        return Ok(Outcome::Checked(CheckOutput { guards: census, warnings }));
    }

    // ── Link, check, emit. ──
    let mut listing = crate::emit::listing_from_resolved(&resolved, &stubs);
    let define_rows = crate::game_defines::define_listing_rows(&defines, &[], &listing, &map_path.display().to_string()).map_err(ctx)?;
    listing.extend(define_rows.iter().cloned());
    let z80_windows = crate::emit::z80_windows(&map, &resolved).map_err(ctx)?;
    let linked = sigil_link::link(&resolved, &stubs).map_err(|d| ctx(crate::guards::render_declared_chain("link", &d, &sources)))?;
    opts.seams.validate_sound(&p.checkout, opts.anchor_overlay.as_ref(), &resolved, &linked, sound, debug).map_err(ctx)?;
    crate::guards::validate_placement(&resolved, &pmap, sound, &registry).map_err(ctx)?;
    align.validate_resolved(&resolved).map_err(ctx)?;
    crate::guards::check_budgets(&resolved, &map, &pmap).map_err(ctx)?;
    let rom = sigil_link::emit_rom(&linked, &map).map_err(|e| ctx(format!("emit_rom: {e}")))?;
    let mut code = emp_symbols.code.clone();
    code.extend(as_code);
    let side = opts
        .seams
        .side_tables(SideInputs {
            emp_symbols: &emp_symbols,
            as_symbols: &as_symbols,
            define_rows: &define_rows,
            resolved: &resolved,
            stubs: &stubs,
            z80_windows: &z80_windows,
            emp_layouts: &emp_layouts,
            code,
            resident,
            rom: &rom,
        })
        .map_err(ctx)?;
    let assembled_len = rom.len();
    let image = rom.clone();
    let full = if island {
        let facts = crate::appendix::AppendixFacts {
            blob_label: &p.engine.error_handler_blob,
            blob_len: p.engine.error_handler_blob_len,
            convsym: p.engine_dir.join(&p.engine.convsym),
        };
        crate::appendix::append_deb2_appendix_with(&facts, &rom, &listing, debug, p.game.appendix_floor as usize, true).map_err(ctx)?
    } else {
        crate::appendix::check_error_handler_is_last_with(&p.engine.error_handler_blob, p.engine.error_handler_blob_len, &listing, rom.len(), false)
            .map_err(ctx)?;
        rom
    };
    let export_inputs = ExportInputs { sections: resolved, locator: Box::new(sources), side, z80_windows };
    Ok(Outcome::Built(Box::new(BuildOutput {
        rom: full,
        assembled_len,
        image,
        listing,
        export_inputs,
        warnings,
        registry,
        checkout: p.checkout.clone(),
        defines,
        debug,
    })))
}

/// A shape's inputs before any front end runs: the resolved shape, its define
/// environment (and the shape the gate rules evaluate, carrying it), and the placement
/// facts (the linker map for the shape, and the placement map with any anchor overlay
/// and fixture relocation applied).
struct Plan {
    shape: ResolvedShape,
    defines: Vec<(String, i128)>,
    gate_shape: ResolvedShape,
    map: sigil_ir::map::MemoryMap,
    pmap: PlacementMap,
}

fn plan(p: &Project, shape_name: &str, opts: &BuildOptions<'_>) -> Result<Plan, BuildError> {
    let shape = describe::resolve_shape(&p.game, shape_name)?;
    let sound = shape.features.sound;
    let map_path = p.map_path();
    let map_src = sigil_span::read_set::read_to_string(&map_path).map_err(|e| ctx(format!("read {}: {e}", map_path.display())))?;
    let defines = merged_defines(p, &shape, &map_src)?;
    let gate_shape = ResolvedShape { defines: defines.iter().map(|(k, v)| (k.clone(), *v as i64)).collect(), ..shape.clone() };

    // ── Placement facts. ──
    let base_pmap = crate::map_placement::load_placement_map(&map_src).map_err(|e| ctx(format!("placement {}: {e}", map_path.display())))?;
    let map = sigil_link::load_map(&map_src).map_err(|e| ctx(format!("load {}: {e}", map_path.display())))?;
    let map = crate::map_placement::map_for_shape(&map, &base_pmap, sound);
    let mut pmap: PlacementMap = match &opts.anchor_overlay {
        Some(ov) => base_pmap.with_overlay(ov).map_err(ctx)?,
        None => base_pmap.clone(),
    };
    let map = crate::map_placement::move_bank_regions(&map, &base_pmap, &pmap, sound);
    let map = crate::map_placement::bank_windows_for_shape(&map, &pmap, sound).map_err(ctx)?;
    if let Some(fp) = &shape.fixture_placement {
        relocate_rows(&mut pmap.order, &fp.relocate, &p.engine.fault_island_heads)?;
    }
    Ok(Plan { shape, defines, gate_shape, map, pmap })
}

/// `sigil build --project ... --report name-env` (A4): the name environment of the
/// program `shape_name` lowers, over the same prepared program the build lowers (the
/// placed modules, the helper rewrite, the synthetic entry). Nothing is emitted, so the
/// lowering names no generated root: an `embed` of a generated artifact is refused by
/// name rather than read from anywhere.
pub fn name_env_report(
    p: &Project,
    shape_name: &str,
    opts: &BuildOptions<'_>,
) -> Result<(Vec<resolve::hygiene::NameEnvRow>, Vec<sigil_span::Diagnostic>), BuildError> {
    let Plan { defines, gate_shape, pmap, .. } = plan(p, shape_name, opts)?;
    let PreparedEmp { manifest, entry_id, lopts, .. } = prepare_emp(p, &gate_shape, &pmap.order, &defines, opts, None)?;
    Ok(resolve::name_env_report(&manifest, entry_id, None, &lopts))
}

/// The shape's define environment as the build uses it ([`merged_defines`] over the
/// game's map), for the reports and the contract closure gate that run beside a build.
pub fn shape_defines(p: &Project, shape_name: &str) -> Result<(ResolvedShape, Vec<(String, i128)>), BuildError> {
    let shape = describe::resolve_shape(&p.game, shape_name)?;
    let map_path = p.map_path();
    let map_src = sigil_span::read_set::read_to_string(&map_path).map_err(|e| ctx(format!("read {}: {e}", map_path.display())))?;
    let defines = merged_defines(p, &shape, &map_src)?;
    Ok((shape, defines))
}

/// Move the `relocate` rows (a fixture shape's growable sections) to just before the
/// first fault island head in the order, keeping their own order.
fn relocate_rows(order: &mut Vec<String>, relocate: &[String], fault_heads: &[String]) -> Result<(), BuildError> {
    let mut moved = Vec::new();
    for head in relocate {
        match order.iter().position(|s| s == head) {
            Some(at) => moved.push(order.remove(at)),
            None => return Err(BuildError::Desc(format!("[fixture.relocate] `{head}` is not in the map order"))),
        }
    }
    let at = order
        .iter()
        .position(|s| fault_heads.iter().any(|h| h == s))
        .ok_or_else(|| BuildError::Desc(format!("[fixture.relocate] no fault island head ({}) in the map order", fault_heads.join(", "))))?;
    for (k, head) in moved.into_iter().enumerate() {
        order.insert(at + k, head);
    }
    Ok(())
}

/// Whether the shape carries the MD Debugger island: the engine's `error_handler` and
/// `release_fault` modules are the two arms of one gate, so exactly one is placed, and it
/// is the one the features say (`debug || crash_report`).
fn declares_island(p: &Project, shape: &ResolvedShape, registry: &[ModuleSpec]) -> Result<bool, BuildError> {
    let axis = shape.features.debug || shape.features.crash_report;
    let island = registry.iter().any(|m| m.module_id == p.engine.error_handler);
    let lean = registry.iter().any(|m| m.module_id == p.engine.release_fault);
    if island == lean {
        return Err(ctx(format!(
            "[gate.fault-split] shape `{}` places {} of `{}` and `{}`; the fault handler is exactly one",
            shape.name,
            if island { "both" } else { "neither" },
            p.engine.error_handler,
            p.engine.release_fault
        )));
    }
    if axis != island {
        return Err(ctx(format!(
            "[gate.fault-split] shape `{}`: `debug || crash_report` is {axis} but the gate {} `{}`",
            shape.name,
            if island { "places" } else { "drops" },
            p.engine.error_handler
        )));
    }
    Ok(island)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// placed ⊆ analysed (G0b Task 9): a placed module scanned under an analysed root
    /// passes; one tagged with no root, one under a root the closure does not analyse,
    /// and one in no scanned file are each refused, by name, in one refusal.
    #[test]
    fn a_placed_module_outside_the_analysed_roots_is_refused_by_name() {
        use crate::place::ModuleSpec;
        use resolve::manifest::{Manifest, ParsedModule};
        use sigil_span::roots::RootName;
        let tmp = crate::build_dir::BuildDir::create().unwrap();
        let root = tmp.path().to_path_buf();
        let (engine, game, other) = (root.join("engine"), root.join("games/g"), root.join("games/h"));
        let module = |id: &str, dir: &Path, tag: Option<RootName>| {
            std::fs::create_dir_all(dir).unwrap();
            let path = dir.join(format!("{}.emp", id.rsplit('.').next().unwrap()));
            let src = format!("module {id}\n");
            std::fs::write(&path, &src).unwrap();
            ParsedModule { id: id.to_string(), file: sigil_frontend_emp::parse_str(&src).0, path, root: tag }
        };
        let modules = vec![
            module("engine.eng", &engine, Some(RootName::Engine)),
            module("games.g.gmod", &game, Some(RootName::Game)),
            module("games.g.synth", &game, None),
            module("games.h.hmod", &other, Some(RootName::Game)),
        ];
        let by_id = modules.iter().enumerate().map(|(i, m)| (m.id.clone(), i)).collect();
        let manifest = Manifest { modules, by_id, sources: Default::default() };
        let spec = |id: &str| ModuleSpec { module_id: id.to_string(), section: "s".to_string() };
        let analysed = vec![engine.clone(), game.clone()];

        check_placed_in_analysed(&[spec("engine.eng"), spec("games.g.gmod")], &manifest, &analysed)
            .expect("modules under the analysed roots are placed");
        let err = check_placed_in_analysed(
            &[spec("engine.eng"), spec("games.g.synth"), spec("games.h.hmod"), spec("games.g.gone"), spec("games.h.hmod")],
            &manifest,
            &analysed,
        )
        .expect_err("a placed module outside the analysed roots must be refused")
        .to_string();
        assert!(err.contains("[gate.placed-not-analysed] 3 placed module(s)"), "{err}");
        assert!(err.contains("`games.g.synth`"), "a module tagged with no root, {err}");
        assert!(err.contains(&format!("`games.h.hmod` ({})", other.join("hmod.emp").display())), "{err}");
        assert!(err.contains("`games.g.gone` (in no scanned file)"), "{err}");
        assert!(!err.contains("`engine.eng`"), "{err}");
    }

    const ENGINE: &str = r#"format = 1
        helpers = []
        ram_helpers = []
        struct_twins = []
        entry_seeds = ["engine.game_contract", "engine.ram"]
        terminus = "engine.epilogue"
        error_handler = "engine.debug.error_handler"
        release_fault = "engine.system.release_fault"
        required_labels = []
        required_consts = []
        baseline = "b"
        constants = "c"
        types = "t"
        error_handler_blob = "x"
        error_handler_blob_len = 1
        convsym = "c"
        fault_island_heads = []
        ram_floor = 0xF00000
        as_feature_defines = [
            { name = "SND_ON", when = "sound" },
            { name = "ISLAND", when = "debug || crash_report" },
            { name = "KEYS", when = "HOTKEYS" },
        ]
        [defines]
        HOTKEYS = 0
        [sound]
        blob_bank_ids = []
        blob_start = "a"
        blob_end = "b"
        dac_head = "c"
        dac_blip_bank = "d"
        dac_shared_bank = "e"
    "#;

    /// `required_consts` is a closed, required key (G0b Task 9a): a misspelling is refused by
    /// name, and an engine description that leaves the key out is refused naming it, so no
    /// engine is read as "requires nothing" by accident.
    #[test]
    fn required_consts_is_a_closed_required_key() {
        assert!(ENGINE.contains("required_consts = []\n"), "the fixture lost its row");
        let typo = ENGINE.replace("required_consts = []", "required_const = []");
        let err = toml::from_str::<describe::EngineDesc>(&typo).expect_err("a misspelt key").to_string();
        assert!(err.contains("required_const"), "the refusal names the key: {err}");
        let absent = ENGINE.replace("required_consts = []\n", "");
        let err = toml::from_str::<describe::EngineDesc>(&absent).expect_err("an absent key").to_string();
        assert!(err.contains("required_consts"), "the refusal names the missing key: {err}");
    }

    const GAME: &str = r#"format = 1
        id = "g"
        name = "g"
        require_one_text = false
        appendix_floor = 4096
        map = "map.toml"
        baseline = "baseline.toml"
        engine = { suite_path = "aeon", root = "engine" }
        sources = { root = ".", residual_as = "game_root.asm" }
        entry = { game = "games.g.game", ram = "games.g.ram" }
        [defines]
        RINGS = 16
        [shapes.release]
        features = { debug = false, crash_report = true, sound = false }
        [shapes.keys]
        extends = "release"
        defines = { HOTKEYS = 1, RINGS = 8 }
        [shapes.typo]
        extends = "release"
        defines = { HOTKYES = 1 }
    "#;

    fn project(game: &str, engine: &str) -> Project {
        Project {
            game_path: "games/g/game.toml".into(),
            game_dir: "games/g".into(),
            game: describe::parse_game(Path::new("games/g/game.toml"), game).unwrap(),
            checkout: ".".into(),
            engine_dir: "engine".into(),
            engine_path: "engine/engine.toml".into(),
            engine: toml::from_str(engine).unwrap(),
        }
    }

    /// The residual AS's feature defines come from `as_feature_defines` (G0b Task 11; the
    /// driver held `SOUND_DRIVER_ENABLED`/`__DEBUG__`/`__MDDBG__` as literals before): each
    /// row whose `when` holds is defined, in description order, a define-reading `when`
    /// reads the shape's merged defines, and a `when` naming a define the shape lacks is
    /// refused naming the row.
    #[test]
    fn the_residual_as_feature_defines_follow_the_description() {
        let p = project(GAME, ENGINE);
        let names = |shape: &str| -> Vec<String> {
            let rs = describe::resolve_shape(&p.game, shape).unwrap();
            let env = merged_defines(&p, &rs, "").unwrap();
            as_feature_defines(&p, &rs.features, &env).unwrap().into_iter().map(|(n, v)| {
                assert_eq!(v, 1, "a feature define is defined as 1");
                n
            }).collect()
        };
        assert_eq!(names("release"), ["ISLAND"], "release: crash_report, no sound, HOTKEYS 0");
        assert_eq!(names("keys"), ["ISLAND", "KEYS"], "keys overrides HOTKEYS = 1");
        let rs = describe::resolve_shape(&p.game, "release").unwrap();
        let err = as_feature_defines(&p, &rs.features, &[]).expect_err("KEYS reads HOTKEYS, absent here").to_string();
        assert!(err.contains("`as_feature_defines` row `KEYS`") && err.contains("HOTKEYS"), "names the row and the define: {err}");
        let none = project(GAME, &ENGINE.replace("as_feature_defines = [", "unused = [").replace("unused = [\n            { name = \"SND_ON\", when = \"sound\" },\n            { name = \"ISLAND\", when = \"debug || crash_report\" },\n            { name = \"KEYS\", when = \"HOTKEYS\" },\n        ]\n", ""));
        assert!(none.engine.as_feature_defines.is_empty(), "an engine whose AS tests no feature declares none");
        assert!(as_feature_defines(&none, &rs.features, &[]).unwrap().is_empty());
    }

    /// `struct_tail_equates` rows (G0b Task 11; `SST_interact` was a literal) must name a
    /// struct a `struct_twins` row lays out, and may not reach past its start: both refused
    /// by name before any file is read.
    #[test]
    fn a_struct_tail_equate_must_name_a_twin() {
        let p = project(GAME, &ENGINE.replace("fault_island_heads = []\n", "fault_island_heads = []\n        struct_tail_equates = [{ name = \"T_tail\", of = \"Nope\", from_end = 2 }]\n"));
        assert_eq!(p.engine.struct_tail_equates.len(), 1, "the fixture row parsed");
        let err = struct_offsets(&p).expect_err("no twin lays out `Nope`").to_string();
        assert!(err.contains("`struct_tail_equates` row `T_tail` names struct `Nope`"), "names the row: {err}");
    }

    fn defines(p: &Project, shape: &str, map: &str) -> Result<Vec<(String, i128)>, BuildError> {
        merged_defines(p, &describe::resolve_shape(&p.game, shape).unwrap(), map)
    }

    #[test]
    fn defines_merge_in_order_and_a_shape_overrides_what_is_declared() {
        let p = project(GAME, ENGINE);
        let env = defines(&p, "keys", "[defines]\nCAPS = 3\n").unwrap();
        let names: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(names, ["SOUND_DRIVER_ENABLED", "DEBUG", "CRASH_REPORT", "HOTKEYS", "RINGS", "CAPS"]);
        let get = |k: &str| env.iter().find(|(n, _)| n == k).unwrap().1;
        assert_eq!((get("SOUND_DRIVER_ENABLED"), get("DEBUG"), get("CRASH_REPORT")), (0, 0, 1));
        assert_eq!((get("HOTKEYS"), get("RINGS"), get("CAPS")), (1, 8, 3));
        assert_eq!(defines(&p, "release", "").unwrap().iter().find(|(n, _)| n == "HOTKEYS").unwrap().1, 0);
    }

    /// QUEUE G0B-DESCRIBE-TASK2-RESIDUE item 5: a shape override of a define neither
    /// description declares is refused, not minted.
    #[test]
    fn a_shape_override_of_an_undeclared_define_is_refused() {
        let p = project(GAME, ENGINE);
        let err = defines(&p, "typo", "").unwrap_err().to_string();
        assert!(err.contains("[define.undeclared-override]") && err.contains("HOTKYES"), "{err}");
        // A game redeclaring an engine define, and a map row redeclaring one, are refused too.
        let redecl = project(&GAME.replace("RINGS = 16", "RINGS = 16\n        HOTKEYS = 1"), ENGINE);
        assert!(defines(&redecl, "release", "").unwrap_err().to_string().contains("[define.redeclared]"));
        assert!(defines(&p, "release", "[defines]\nRINGS = 2\n").unwrap_err().to_string().contains("[define.redeclared]"));
        // (A game `[defines]` row naming a derived define is refused by the reader, describe.rs.)
    }

    #[test]
    fn fixture_rows_move_before_the_first_fault_island() {
        let mut order: Vec<String> = ["A", "Pool", "B", "ReleaseFault", "BusError", "EndOfRom"].iter().map(|s| s.to_string()).collect();
        relocate_rows(&mut order, &["Pool".into()], &["ReleaseFault".into(), "BusError".into()]).unwrap();
        assert_eq!(order, ["A", "B", "Pool", "ReleaseFault", "BusError", "EndOfRom"]);
        assert!(relocate_rows(&mut order, &["Gone".into()], &["BusError".into()]).is_err());
    }
}
