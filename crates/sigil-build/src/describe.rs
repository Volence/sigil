//! The description formats: `games/<id>/game.toml`, aeon's `engine/engine.toml`, and the
//! `baseline.toml` each of them names.
//!
//! Spec: `empyrean/docs/v2/specs/2026-11-17-G0b-game-as-data-driver-design.md` §2 (what Sigil
//! knows about Aeon today), §5.1 (what the descriptions contain), §5.4 (the sound seams stay
//! out). Each field holds a fact the Rust driver in `sigil-harness` held until G0b Task 11
//! deleted it; the committed files once cited the Rust line each value was transcribed from.
//!
//! ── CLOSED INPUTS ──
//!
//! Every struct is `deny_unknown_fields`, nested ones included, so a misspelt or invented key
//! is refused BY NAME (the toml error names the key) rather than silently defaulted.
//!
//! ── PATHS ──
//!
//! A path in a `game.toml` is relative to the directory holding that file (the game root); a
//! path in an `engine.toml` is relative to the directory holding it (the engine root). No file
//! names an absolute path: the engine is named by suite path through [`EngineRef`] and found by
//! `empyrean/contract/SUITE_PATHS.md`'s precedence ([`resolve_engine_dir`]).
//!
//! ── WHAT IS NOT A FIELD ──
//!
//! The comptime defines `DEBUG`, `CRASH_REPORT` and `SOUND_DRIVER_ENABLED`, and the residual-AS
//! definedness defines `__DEBUG__` and `__MDDBG__`, are DERIVED from a shape's [`Features`]
//! (`debug`, `crash_report`, `sound`), as the Rust profiles derive them today. Writing one as a
//! define is refused ([`DescError::DerivedDefine`]): two sources for one fact is a fork.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// The one description format version this reader accepts.
///
/// ── FORMAT RULE ──
///
/// `format` names a key set, and every struct here is closed, so a reader can only read the
/// key set it was built for. The number is checked FIRST, from the untyped table, before the
/// closed typed parse ([`check_format_text`]): an older Sigil reading a newer file must be
/// told "this file is format N, this reader accepts format 1", not "unknown field `x`"
/// (G0b final review A-I4). After the first landing, ANY change to the key set of
/// `game.toml`, `engine.toml` or `baseline.toml` (a key added, optional ones included,
/// removed, renamed or re-typed) bumps `format` in the same change, together with every
/// committed description. Before that landing (G0b), the number stayed 1 while Tasks 5,
/// 9a and 11 added required `engine.toml` keys.
pub const FORMAT: u32 = 1;

/// Defines a description may not write, because the driver derives them from [`Features`].
pub const DERIVED_DEFINES: &[&str] = &["DEBUG", "CRASH_REPORT", "SOUND_DRIVER_ENABLED", "__DEBUG__", "__MDDBG__"];

// ───────────────────────────── game.toml ─────────────────────────────

/// `games/<id>/game.toml`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameDesc {
    pub format: u32,
    pub id: String,
    pub name: String,
    pub engine: EngineRef,
    pub sources: Sources,
    pub entry: EntryModules,
    /// The game's own comptime defines, the same in every one of its shapes (a shape may
    /// override one).
    #[serde(default)]
    pub defines: BTreeMap<String, i64>,
    /// Enforce exactly one non-empty `text` section (the objdefs). Added by Task 2: the Rust
    /// profile field `require_one_text` (sonic4 true, demo false) had no home in the brief.
    pub require_one_text: bool,
    /// The deb2 symbol appendix's presence floor, in bytes. Added by Task 2: it replaced
    /// the per-game Rust floor constants the CLI chose between (deleted, G0b Task 11).
    pub appendix_floor: u32,
    pub shapes: BTreeMap<String, ShapeDesc>,
    #[serde(default)]
    pub gate: Vec<GateRule>,
    /// The game's placement map (`map.toml`, format unchanged).
    pub map: PathBuf,
    /// Alignment requirements stronger than the 68000's even-base rule, for the game's labels.
    #[serde(default)]
    pub align: Vec<AlignRow>,
    /// The game's contract-closure baseline rows ([`BaselineDesc`]).
    pub baseline: PathBuf,
    /// Output stem per shape name.
    #[serde(default)]
    pub outputs: BTreeMap<String, String>,
}

/// The engine a game builds on, by suite path name. Resolved by [`resolve_engine_dir`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineRef {
    /// The checkout's suite name (`aeon`), whose `<NAME>_DIR` variable names it.
    pub suite_path: String,
    /// The engine root inside that checkout (`engine`).
    pub root: PathBuf,
}

/// Where the game's sources live.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    /// The game's source root, relative to the game.toml directory.
    pub root: PathBuf,
    /// The game's residual AS root (`game_root.asm`).
    pub residual_as: PathBuf,
}

/// The game's entry modules: the ones the synthetic entry `use`s besides the engine's seeds,
/// and the `.emp` modules whose `pub const`s are harvested.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryModules {
    /// The module that `implement`s `Game` (the contract manifest).
    pub game: String,
    /// The game RAM module.
    pub ram: String,
    /// The game's constants module (a path).
    #[serde(default)]
    pub constants: Option<PathBuf>,
    /// The game's sound-id module (a path).
    #[serde(default)]
    pub sound_ids: Option<PathBuf>,
    /// The game's SFX-bank module (a path).
    #[serde(default)]
    pub sfx_bank: Option<PathBuf>,
}

/// One named shape.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapeDesc {
    /// The shape this one is built from: its features, defines and placement are inherited,
    /// and this shape's own values override them.
    #[serde(default)]
    pub extends: Option<String>,
    /// Required on a shape with no `extends`; inherited when absent on one that has it.
    /// (The brief spelt this `Features`; `Option` is what lets an `extends` shape inherit.)
    #[serde(default)]
    pub features: Option<Features>,
    #[serde(default)]
    pub defines: BTreeMap<String, i64>,
    /// A shape that may never ship. Sticky: a shape that extends a fixture is one.
    #[serde(default)]
    pub fixture: bool,
    /// Added by Task 2: the Rust profile field `fixture_placement` (greedy packing, no frozen
    /// overrun guard) together with the sections `relocate_fixture_pool` moves. Only the
    /// stress-art fixture sets it.
    #[serde(default)]
    pub fixture_placement: Option<FixturePlacement>,
}

/// The three feature axes a shape is built on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Features {
    pub debug: bool,
    pub crash_report: bool,
    pub sound: bool,
}

/// Fixture placement: pack every non-island section greedily from measured sizes, after moving
/// `relocate` (head labels, in ROM order) to just before the fault-handler island.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixturePlacement {
    pub relocate: Vec<String>,
}

/// One module-gating rule.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateRule {
    /// A module id, or a glob over ids.
    pub modules: String,
    pub place_in: PlaceIn,
    /// The description file the rule was read from (set by [`load_game`] / [`load_engine`]
    /// / [`parse_game`]; empty for a rule built in code), so a refusal can name it.
    #[serde(skip)]
    pub origin: String,
    /// The rule's 1-based position among that file's `[[gate]]` rows (0 when built in code).
    #[serde(skip)]
    pub row: usize,
}

/// Stamp each rule with its file and 1-based row, and refuse a pattern spelling the matcher
/// does not define ([`crate::gating::check_pattern`]) naming the file and the row: matched
/// literally, such a pattern would match nothing (G0b final review A-I6).
fn stamp_gate_rows(path: &Path, rules: &mut [GateRule]) -> Result<(), DescError> {
    for (i, r) in rules.iter_mut().enumerate() {
        r.origin = path.display().to_string();
        r.row = i + 1;
        crate::gating::check_pattern(&r.modules)
            .map_err(|m| DescError::Parse { path: path.to_path_buf(), message: format!("[[gate]] row {}: {m}", r.row) })?;
    }
    Ok(())
}

/// Where a gated module is placed. TOML: `"all"`, `{ shapes = [..] }`, `{ feature = ".." }`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceIn {
    All,
    /// Exactly these shapes; `[]` places the module in none (a sigil probe fixture).
    Shapes(Vec<String>),
    Feature(FeatureExpr),
}

/// A feature predicate: `debug`, `crash_report`, `sound`, or a define name (true when that
/// define is 1 in the shape), combined with `!`, `||` and parentheses. The brief named
/// `debug`, `!debug`, `sound`, `crash_report`; the Rust gates today also need `!sound`
/// (`ShapeGate::SoundOff`), `debug || crash_report` and its negation (`FaultIsland`,
/// `LeanFault`) and a define test (`SoundDebugHotkeys`, `SoundDbgMirror`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum FeatureExpr {
    Debug,
    CrashReport,
    Sound,
    Define(String),
    Not(Box<FeatureExpr>),
    Or(Vec<FeatureExpr>),
}

impl TryFrom<String> for FeatureExpr {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        FeatureExpr::parse(&s)
    }
}

impl FeatureExpr {
    /// Parse `s`; the error names the text and what was wrong with it.
    pub fn parse(s: &str) -> Result<FeatureExpr, String> {
        let toks = tokenize(s)?;
        let mut p = ExprParser { toks: &toks, at: 0 };
        let e = p.or().map_err(|m| format!("feature expression `{s}`: {m}"))?;
        if p.at != toks.len() {
            return Err(format!("feature expression `{s}`: unexpected `{}`", toks[p.at]));
        }
        Ok(e)
    }

    /// Evaluate against `features` and `define` (a define's value in the shape). A define the
    /// lookup does not know is an error naming it, never false.
    pub fn eval(&self, features: &Features, define: &dyn Fn(&str) -> Option<i64>) -> Result<bool, String> {
        Ok(match self {
            FeatureExpr::Debug => features.debug,
            FeatureExpr::CrashReport => features.crash_report,
            FeatureExpr::Sound => features.sound,
            FeatureExpr::Define(n) => {
                define(n).ok_or_else(|| format!("feature expression reads define `{n}`, which this shape does not define"))?
                    == 1
            }
            FeatureExpr::Not(e) => !e.eval(features, define)?,
            FeatureExpr::Or(es) => {
                let mut any = false;
                for e in es {
                    any |= e.eval(features, define)?;
                }
                any
            }
        })
    }
}

fn tokenize(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '!' || c == '(' || c == ')' {
            out.push(c.to_string());
            i += 1;
        } else if c == '|' && cs.get(i + 1) == Some(&'|') {
            out.push("||".into());
            i += 2;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let st = i;
            while i < cs.len() && (cs[i].is_ascii_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            out.push(cs[st..i].iter().collect());
        } else {
            return Err(format!("feature expression `{s}`: unexpected character `{c}`"));
        }
    }
    Ok(out)
}

struct ExprParser<'a> {
    toks: &'a [String],
    at: usize,
}

impl ExprParser<'_> {
    fn peek(&self) -> Option<&str> {
        self.toks.get(self.at).map(String::as_str)
    }
    fn or(&mut self) -> Result<FeatureExpr, String> {
        let mut terms = vec![self.unary()?];
        while self.peek() == Some("||") {
            self.at += 1;
            terms.push(self.unary()?);
        }
        Ok(if terms.len() == 1 { terms.pop().unwrap() } else { FeatureExpr::Or(terms) })
    }
    fn unary(&mut self) -> Result<FeatureExpr, String> {
        match self.peek() {
            Some("!") => {
                self.at += 1;
                Ok(FeatureExpr::Not(Box::new(self.unary()?)))
            }
            Some("(") => {
                self.at += 1;
                let e = self.or()?;
                if self.peek() != Some(")") {
                    return Err("missing `)`".into());
                }
                self.at += 1;
                Ok(e)
            }
            Some(id) => {
                let e = match id {
                    "debug" => FeatureExpr::Debug,
                    "crash_report" => FeatureExpr::CrashReport,
                    "sound" => FeatureExpr::Sound,
                    d if d.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') => {
                        FeatureExpr::Define(d.to_string())
                    }
                    other => {
                        return Err(format!(
                            "`{other}` is neither a feature (debug, crash_report, sound) nor an \
                             UPPER_CASE define name"
                        ))
                    }
                };
                self.at += 1;
                Ok(e)
            }
            None => Err("ends where an operand was expected".into()),
        }
    }
}

/// An alignment requirement stronger than "even" for the section whose head label is `label`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlignRow {
    pub label: String,
    pub align: u32,
}

// ───────────────────────────── engine.toml ─────────────────────────────

/// aeon `engine/engine.toml`: the facts true for every game built on the engine.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineDesc {
    pub format: u32,
    /// The pure-comptime helper modules, IN ORDER: the later helper wins a duplicate name.
    pub helpers: Vec<String>,
    /// The helpers the focused RAM-address harvest publishes.
    pub ram_helpers: Vec<String>,
    /// `(file, struct, AS prefix)`: the brief spelt this a pair; the Rust table
    /// `STRUCT_OFFSET_TWINS` carries the file too, so it is a triple. `file` is relative to
    /// the engine root.
    pub struct_twins: Vec<(String, String, String)>,
    /// The engine's synthetic-entry seeds (the game's `entry.game` and `entry.ram` follow
    /// each in turn: contract, game manifest, engine RAM, game RAM).
    pub entry_seeds: Vec<String>,
    /// The ROM terminus module, rooted in every build.
    pub terminus: String,
    /// The MD Debugger island module (placed when `debug || crash_report`).
    pub error_handler: String,
    /// The lean fault handler module (the island's other arm).
    pub release_fault: String,
    #[serde(default)]
    pub gate: Vec<GateRule>,
    /// Labels every build on the engine must define: after the layout resolves, a name here
    /// that no section labels is refused by name ([`crate::driver::check_required_labels`],
    /// `[engine.required-label]`).
    pub required_labels: Vec<String>,
    /// Added by G0b Task 9a: comptime constants the ENGINE's own sources name by bare name and
    /// every game must declare (a `pub const` in the game's sources), with no engine default.
    /// A game that declares one of them nowhere is refused by name
    /// ([`crate::driver::check_required_consts`]) before the closure or the build reads the
    /// name; a silent default is how a game could ship without ever stating the value.
    pub required_consts: Vec<String>,
    pub sound: SoundDesc,
    /// The engine's contract-closure baseline rows ([`BaselineDesc`]).
    pub baseline: PathBuf,
    /// Added by Task 2: engine comptime defines every game builds with, at their default (a
    /// shape may override one). Today's dev opt-ins, 0 in every Rust profile of both games.
    #[serde(default)]
    pub defines: BTreeMap<String, i64>,
    /// Added by Task 2: alignment requirements of ENGINE labels (`ObjCodeBase` is defined in
    /// `engine/objects/objcodebase.emp`, not in a game).
    #[serde(default)]
    pub align: Vec<AlignRow>,
    /// Added by Task 2: the engine constants module harvested for the residual AS and the
    /// link EquSyms (`harvest_engine_constants`), relative to the engine root.
    pub constants: PathBuf,
    /// Added by Task 2: the shared type vocabulary the struct-twin layout reads
    /// (`harvest_engine_struct_offsets`), relative to the engine root.
    pub types: PathBuf,
    /// Added by Task 2: the label the vendored MD Debugger blob is emitted under
    /// (read by `appendix::AppendixFacts::of`; no Rust spelling since G0b Task 11).
    pub error_handler_blob: String,
    /// Added by Task 5: the vendored MD Debugger blob's length in bytes; the deb2 appendix
    /// must start exactly at `error_handler_blob + error_handler_blob_len`
    /// (read by `appendix::AppendixFacts::of`).
    pub error_handler_blob_len: u32,
    /// Added by Task 5: the `convsym` binary the deb2 appendix runs, relative to the engine
    /// root (it lives in the checkout's `tools/`, beside the engine).
    pub convsym: PathBuf,
    /// Added by Task 2: the head labels of the two fault-handler arms, the anchor
    /// `relocate_fixture_pool` inserts before.
    pub fault_island_heads: Vec<String>,
    /// Added by G0b Task 11: the defines the residual AS side receives (each = 1) on a shape
    /// whose features satisfy the row's `when`. The engine's AS sources test them with
    /// `ifdef` (aeon's `game_root.asm` includes the MD Debugger glue under `__MDDBG__`). An
    /// engine whose residual AS tests none declares none.
    #[serde(default)]
    pub as_feature_defines: Vec<AsFeatureDefine>,
    /// Added by G0b Task 11: the lowest address of work RAM. The focused RAM harvest hands
    /// the residual AS every label and equate of a section placed at or above it.
    pub ram_floor: u32,
    /// Added by G0b Task 11: residual-AS equates derived from a struct twin's size, each
    /// `name = sizeof(of) - from_end`, emitted right after that twin's `<prefix>_len`.
    #[serde(default)]
    pub struct_tail_equates: Vec<StructTailEquate>,
}

/// One residual-AS feature define ([`EngineDesc::as_feature_defines`]).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsFeatureDefine {
    pub name: String,
    pub when: FeatureExpr,
}

/// One struct-tail equate ([`EngineDesc::struct_tail_equates`]): `of` names a struct a
/// `struct_twins` row lays out.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructTailEquate {
    pub name: String,
    pub of: String,
    pub from_end: u32,
}

/// What `sound_bank_ids.rs` holds: the resident blob's bank-id consts, the labels each must
/// bank, and the labels the check reads. Checked on every sound-on build, so every label here
/// is one a sound-on game's layout must carry: game-defined labels (and the engine's two blob
/// brackets) the sound engine requires when sound is on. They live here, not in
/// `required_labels`, because this table carries the sound-on condition (ruling M4). The seam facts (driver files, generated names,
/// the driver constant names, the bank chain heads) are NOT here: they stay in Rust until G0c.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundDesc {
    pub blob_bank_ids: Vec<BankIdConst>,
    pub blob_start: String,
    pub blob_end: String,
    pub dac_head: String,
    pub dac_blip_bank: String,
    pub dac_shared_bank: String,
}

/// One bank-id const baked into the resident blob, and the placed labels whose bank it must
/// select.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankIdConst {
    pub name: String,
    pub banks: Vec<String>,
}

// ───────────────────────────── baseline.toml ─────────────────────────────

/// A contract-closure baseline file: the frozen residue rows an engine or a game contributes.
/// Every list may be absent (empty). The gate for a shape is the multiset union of the
/// engine's rows and the game's, with `d1c_debug` added on a debug shape.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineDesc {
    pub format: u32,
    /// `[proc.out-unverified]` for Z80 procs: `(proc, unit)`.
    #[serde(default)]
    pub z80_out_unverified: Vec<(String, String)>,
    /// `[proc.out-unverified]` for 68000 procs: `(proc, register)`.
    #[serde(default)]
    pub out_unverified: Vec<(String, String)>,
    /// `[proc.inout-unverified]`: `(proc, register)`.
    #[serde(default)]
    pub inout_unverified: Vec<(String, String)>,
    /// `[call.live-clobbered]` on every shape: `(caller, callee, register)`, a multiset.
    #[serde(default)]
    pub d1c: Vec<(String, String, String)>,
    /// `[call.live-clobbered]` on debug shapes only.
    #[serde(default)]
    pub d1c_debug: Vec<(String, String, String)>,
}

// ───────────────────────────── errors ─────────────────────────────

/// A description that could not be read, or says something refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescError {
    Io { path: PathBuf, message: String },
    /// A TOML or schema error; an unknown key's message names the key.
    Parse { path: PathBuf, message: String },
    Format { path: PathBuf, found: u32 },
    UnknownShape(String),
    /// The `extends` chain, from the shape asked for to the repeated name.
    ExtendsCycle(Vec<String>),
    /// A shape with no `extends` and no `features`.
    MissingFeatures(String),
    /// A define the driver derives from features, written as a define (`where`, `name`).
    DerivedDefine { place: String, name: String },
    /// A reference to a shape the file does not declare (`where`, `name`).
    UndeclaredShape { place: String, name: String },
    BadAlign { label: String, align: u32 },
    /// The engine checkout could not be found (or a variable naming it is wrong).
    EngineNotFound(String),
}

impl fmt::Display for DescError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DescError::Io { path, message } => write!(f, "{}: {message}", path.display()),
            DescError::Parse { path, message } => write!(f, "{}: {message}", path.display()),
            DescError::Format { path, found } => {
                write!(f, "{}: format = {found}, this reader accepts format = {FORMAT}", path.display())
            }
            DescError::UnknownShape(s) => write!(f, "no shape named `{s}`"),
            DescError::ExtendsCycle(chain) => write!(f, "`extends` cycle: {}", chain.join(" -> ")),
            DescError::MissingFeatures(s) => {
                write!(f, "shape `{s}` has neither `extends` nor `features`")
            }
            DescError::DerivedDefine { place, name } => write!(
                f,
                "{place}: define `{name}` is derived from the shape's features and may not be written"
            ),
            DescError::UndeclaredShape { place, name } => {
                write!(f, "{place}: names shape `{name}`, which is not declared")
            }
            DescError::BadAlign { label, align } => {
                write!(f, "align row `{label}`: {align} is not a power of two above 1")
            }
            DescError::EngineNotFound(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for DescError {}

// ───────────────────────────── loaders ─────────────────────────────

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, DescError> {
    let text = sigil_span::read_set::read_to_string(path)
        .map_err(|e| DescError::Io { path: path.to_path_buf(), message: e.to_string() })?;
    parse_typed(path, &text)
}

/// The format check ([`check_format_text`]), then the closed typed parse.
fn parse_typed<T: for<'de> Deserialize<'de>>(path: &Path, text: &str) -> Result<T, DescError> {
    check_format_text(path, text)?;
    toml::from_str(text).map_err(|e| DescError::Parse { path: path.to_path_buf(), message: e.to_string() })
}

/// Refuse a description whose `format` is not [`FORMAT`], read from the untyped table so the
/// refusal comes before any unknown or missing key of the closed typed parse (the
/// [`FORMAT`] rule). A file that is not TOML, or has no integer `format`, is left to the typed
/// parse, which names what is wrong.
fn check_format_text(path: &Path, text: &str) -> Result<(), DescError> {
    let Ok(table) = text.parse::<toml::Table>() else { return Ok(()) };
    match table.get("format").and_then(toml::Value::as_integer) {
        Some(n) if n != i64::from(FORMAT) => {
            Err(DescError::Format { path: path.to_path_buf(), found: u32::try_from(n).unwrap_or(u32::MAX) })
        }
        _ => Ok(()),
    }
}

fn check_format(path: &Path, found: u32) -> Result<(), DescError> {
    if found == FORMAT {
        Ok(())
    } else {
        Err(DescError::Format { path: path.to_path_buf(), found })
    }
}

fn check_defines(place: &str, defines: &BTreeMap<String, i64>) -> Result<(), DescError> {
    match defines.keys().find(|k| DERIVED_DEFINES.contains(&k.as_str())) {
        Some(k) => Err(DescError::DerivedDefine { place: place.to_string(), name: k.clone() }),
        None => Ok(()),
    }
}

fn check_align(rows: &[AlignRow]) -> Result<(), DescError> {
    match rows.iter().find(|r| r.align < 2 || !r.align.is_power_of_two()) {
        Some(r) => Err(DescError::BadAlign { label: r.label.clone(), align: r.align }),
        None => Ok(()),
    }
}

/// Parse `text` as a `game.toml` (`path` names it in errors) and check what the file alone can
/// check: the format, no derived define, every shape reference declared, a root shape has
/// features, alignments are powers of two.
pub fn parse_game(path: &Path, text: &str) -> Result<GameDesc, DescError> {
    let mut g: GameDesc = parse_typed(path, text)?;
    stamp_gate_rows(path, &mut g.gate)?;
    check_format(path, g.format)?;
    check_defines("[defines]", &g.defines)?;
    let declared = |place: String, name: &str| -> Result<(), DescError> {
        if g.shapes.contains_key(name) {
            Ok(())
        } else {
            Err(DescError::UndeclaredShape { place, name: name.to_string() })
        }
    };
    for (name, s) in &g.shapes {
        check_defines(&format!("shape `{name}`"), &s.defines)?;
        match &s.extends {
            Some(parent) => declared(format!("shape `{name}` extends"), parent)?,
            None if s.features.is_none() => return Err(DescError::MissingFeatures(name.clone())),
            None => {}
        }
    }
    for rule in &g.gate {
        if let PlaceIn::Shapes(names) = &rule.place_in {
            for n in names {
                declared(format!("gate `{}`", rule.modules), n)?;
            }
        }
    }
    for n in g.outputs.keys() {
        declared("[outputs]".to_string(), n)?;
    }
    check_align(&g.align)?;
    Ok(g)
}

/// Load and check a `game.toml`.
pub fn load_game(path: &Path) -> Result<GameDesc, DescError> {
    let text = sigil_span::read_set::read_to_string(path)
        .map_err(|e| DescError::Io { path: path.to_path_buf(), message: e.to_string() })?;
    parse_game(path, &text)
}

/// Load and check an `engine.toml`.
pub fn load_engine(path: &Path) -> Result<EngineDesc, DescError> {
    let mut e: EngineDesc = read_toml(path)?;
    stamp_gate_rows(path, &mut e.gate)?;
    check_format(path, e.format)?;
    check_defines("[defines]", &e.defines)?;
    check_align(&e.align)?;
    for rule in &e.gate {
        if let PlaceIn::Shapes(names) = &rule.place_in {
            // The engine knows no game's shape names; only a feature or `all` can gate here.
            if let Some(n) = names.first() {
                return Err(DescError::UndeclaredShape { place: format!("engine gate `{}`", rule.modules), name: n.clone() });
            }
        }
    }
    Ok(e)
}

/// Load and check a `baseline.toml`.
pub fn load_baseline(path: &Path) -> Result<BaselineDesc, DescError> {
    let b: BaselineDesc = read_toml(path)?;
    check_format(path, b.format)?;
    Ok(b)
}

// ───────────────────────────── shapes ─────────────────────────────

/// A shape with its `extends` chain flattened over the game's `[defines]`. Engine defines are
/// not merged here (the game file does not know them); the driver lays these over the
/// engine's.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedShape {
    pub name: String,
    pub features: Features,
    pub defines: BTreeMap<String, i64>,
    pub fixture: bool,
    pub fixture_placement: Option<FixturePlacement>,
}

/// Flatten `shape`'s `extends` chain: the root's values first, each shape down the chain
/// overriding what it states. A cycle is refused with the chain that closes it.
pub fn resolve_shape(g: &GameDesc, shape: &str) -> Result<ResolvedShape, DescError> {
    let mut chain: Vec<(&str, &ShapeDesc)> = Vec::new();
    let mut cur = shape;
    loop {
        let s = g.shapes.get(cur).ok_or_else(|| DescError::UnknownShape(cur.to_string()))?;
        if chain.iter().any(|(n, _)| *n == cur) {
            let mut names: Vec<String> = chain.iter().map(|(n, _)| n.to_string()).collect();
            names.push(cur.to_string());
            return Err(DescError::ExtendsCycle(names));
        }
        chain.push((cur, s));
        match &s.extends {
            Some(p) => cur = p,
            None => break,
        }
    }
    let mut features: Option<Features> = None;
    let mut defines = g.defines.clone();
    let mut fixture = false;
    let mut fixture_placement = None;
    for (_, s) in chain.iter().rev() {
        if let Some(f) = s.features {
            features = Some(f);
        }
        defines.extend(s.defines.iter().map(|(k, v)| (k.clone(), *v)));
        fixture |= s.fixture;
        if s.fixture_placement.is_some() {
            fixture_placement = s.fixture_placement.clone();
        }
    }
    let root = chain.last().map(|(n, _)| n.to_string()).unwrap_or_default();
    let features = features.ok_or(DescError::MissingFeatures(root))?;
    Ok(ResolvedShape { name: shape.to_string(), features, defines, fixture, fixture_placement })
}

// ───────────────────────────── engine resolution ─────────────────────────────

/// Which precedence step of `empyrean/contract/SUITE_PATHS.md` found the engine checkout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStep {
    /// Step 1: the checkout variable (`AEON_DIR`).
    CheckoutVar,
    /// Step 2: `EMPYREAN_SUITE_ROOT` joined with the suite path.
    SuiteRoot,
}

/// Find the directory holding `r`'s engine root, by SUITE_PATHS.md's precedence: (1) the
/// checkout variable `<SUITE_PATH>_DIR`; (2) `EMPYREAN_SUITE_ROOT/<suite_path>`; (4) refuse,
/// naming what was looked for. A variable that is set but names a tree without the engine root
/// is a hard error at its own step, never a fall-through. Step 3 (derivation from the calling
/// repo's location) is not taken: an installed `sigil` has no repo location to derive from,
/// and deriving from the game's location would find the owner's live tree for a game in a
/// worktree. `env` is the environment lookup (injected so a test owns its environment).
///
/// Returns the CHECKOUT directory (the engine root is `<it>/<r.root>`) and the step.
pub fn resolve_engine_dir(
    r: &EngineRef,
    env: &dyn Fn(&str) -> Option<std::ffi::OsString>,
) -> Result<(PathBuf, EngineStep), DescError> {
    let var = format!("{}_DIR", r.suite_path.to_ascii_uppercase());
    let check = |dir: PathBuf, how: String, step: EngineStep| -> Result<(PathBuf, EngineStep), DescError> {
        if dir.join(&r.root).is_dir() {
            Ok((dir, step))
        } else {
            Err(DescError::EngineNotFound(format!(
                "{how} names {}, which has no engine root `{}`",
                dir.display(),
                r.root.display()
            )))
        }
    };
    if let Some(v) = env(&var).filter(|v| !v.is_empty()) {
        return check(PathBuf::from(v), var, EngineStep::CheckoutVar);
    }
    if let Some(v) = env("EMPYREAN_SUITE_ROOT").filter(|v| !v.is_empty()) {
        return check(
            PathBuf::from(v).join(&r.suite_path),
            format!("EMPYREAN_SUITE_ROOT joined with `{}`", r.suite_path),
            EngineStep::SuiteRoot,
        );
    }
    Err(DescError::EngineNotFound(format!(
        "engine `{}` (root `{}`) not found: set {var} to its checkout, or EMPYREAN_SUITE_ROOT to \
         the directory holding `{}`",
        r.suite_path,
        r.root.display(),
        r.suite_path
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── a minimal game for the logic tests ──

    const MINI: &str = r#"
        format = 1
        id = "mini"
        name = "mini"
        require_one_text = false
        appendix_floor = 4096
        map = "map.toml"
        baseline = "baseline.toml"
        engine = { suite_path = "aeon", root = "engine" }
        sources = { root = ".", residual_as = "game_root.asm" }
        entry = { game = "games.mini.game", ram = "games.mini.ram" }
        [defines]
        A = 1
        B = 2
        [shapes.release]
        features = { debug = false, crash_report = true, sound = false }
        [shapes.debug]
        features = { debug = true, crash_report = true, sound = false }
        [shapes.dev]
        extends = "debug"
        defines = { B = 7 }
    "#;

    fn mini() -> GameDesc {
        parse_game(Path::new("mini/game.toml"), MINI).expect("the mini game parses")
    }

    #[test]
    fn unknown_key_refused_by_name() {
        // Top level, a nested table, and an inline table inside a shape: each is refused, and
        // the message names the key.
        for (bad, key) in [
            (MINI.replace("format = 1", "format = 1\nbogus_top = 1"), "bogus_top"),
            (MINI.replace("residual_as = \"game_root.asm\"", "residual_as = \"g.asm\", bogus_src = 1"), "bogus_src"),
            (MINI.replace("sound = false }\n        [shapes.debug]", "sound = false, bogus_feat = true }\n        [shapes.debug]"), "bogus_feat"),
            (MINI.replace("extends = \"debug\"", "extends = \"debug\"\n        bogus_shape = 1"), "bogus_shape"),
        ] {
            assert!(bad.contains(key), "the mutation for `{key}` did not apply");
            let err = parse_game(Path::new("mini/game.toml"), &bad).expect_err(key).to_string();
            assert!(err.contains(key), "error for `{key}` does not name it: {err}");
        }
        // An engine nested table (`[sound]`) is closed too.
        let e: Result<EngineDesc, _> = toml::from_str(
            r#"format = 1
            helpers = []
            ram_helpers = []
            struct_twins = []
            entry_seeds = []
            terminus = "t"
            error_handler = "e"
            release_fault = "r"
            required_labels = []
            required_consts = []
            baseline = "b"
            constants = "c"
            types = "t"
            error_handler_blob = "x"
            error_handler_blob_len = 1
            convsym = "c"
            fault_island_heads = []
            ram_floor = 0
            [sound]
            blob_bank_ids = []
            blob_start = "a"
            blob_end = "b"
            dac_head = "c"
            dac_blip_bank = "d"
            dac_shared_bank = "e"
            bogus_sound = 1
            "#,
        );
        let msg = e.expect_err("unknown [sound] key").to_string();
        assert!(msg.contains("bogus_sound"), "engine error does not name the key: {msg}");
    }

    #[test]
    fn extends_overrides_one_define() {
        let g = mini();
        let dev = resolve_shape(&g, "dev").unwrap();
        let debug = resolve_shape(&g, "debug").unwrap();
        assert_eq!(dev.defines["B"], 7, "the override wins");
        assert_eq!(dev.defines["A"], g.defines["A"], "an untouched define inherits");
        assert_eq!(dev.features, debug.features, "features inherit through extends");
        assert_eq!(dev.defines.len(), debug.defines.len(), "nothing added, nothing lost");
        assert_eq!(debug.defines["B"], 2, "the parent keeps its own value");
    }

    #[test]
    fn extends_cycle_refused() {
        let text = MINI.replace(
            "[shapes.dev]\n        extends = \"debug\"",
            "[shapes.dev]\n        extends = \"loop\"\n        [shapes.loop]\n        extends = \"dev\"",
        );
        assert!(text.contains("[shapes.loop]"));
        let g = parse_game(Path::new("mini/game.toml"), &text).unwrap();
        assert_eq!(
            resolve_shape(&g, "dev"),
            Err(DescError::ExtendsCycle(vec!["dev".into(), "loop".into(), "dev".into()]))
        );
        // A self-extending shape is the one-element cycle.
        let selfish = MINI.replace("extends = \"debug\"", "extends = \"dev\"");
        let g = parse_game(Path::new("mini/game.toml"), &selfish).unwrap();
        assert_eq!(resolve_shape(&g, "dev"), Err(DescError::ExtendsCycle(vec!["dev".into(), "dev".into()])));
    }

    #[test]
    fn closed_references_and_derived_defines_refused() {
        let undeclared = MINI.replace("extends = \"debug\"", "extends = \"nope\"");
        assert!(matches!(
            parse_game(Path::new("g"), &undeclared),
            Err(DescError::UndeclaredShape { name, .. }) if name == "nope"
        ));
        let derived = MINI.replace("A = 1", "DEBUG = 1");
        assert!(matches!(
            parse_game(Path::new("g"), &derived),
            Err(DescError::DerivedDefine { name, .. }) if name == "DEBUG"
        ));
        let rootless = MINI.replace("features = { debug = false, crash_report = true, sound = false }\n", "");
        assert_eq!(parse_game(Path::new("g"), &rootless), Err(DescError::MissingFeatures("release".into())));
        let fmt2 = MINI.replace("format = 1", "format = 2");
        assert!(matches!(parse_game(Path::new("g"), &fmt2), Err(DescError::Format { found: 2, .. })));
    }

    /// G0b final review A-I6: a `[[gate]]` pattern the matcher does not define (`?`, `[`,
    /// `***`, an empty `modules`) is refused when the file is read, naming the file, the row
    /// and the pattern. It used to be matched literally, so it matched nothing and the
    /// module its author meant surfaced later as `[gate.ungated]`.
    #[test]
    fn a_gate_pattern_the_matcher_does_not_define_is_refused_at_read() {
        for bad in ["games.mini.te?t", "games.mini.***", "games.mini.[ab]", ""] {
            let text = format!(
                "{MINI}\n        [[gate]]\n        modules = \"games.mini.ok\"\n        place_in = \"all\"\n        [[gate]]\n        modules = \"{bad}\"\n        place_in = \"all\"\n"
            );
            let err = parse_game(Path::new("mini/game.toml"), &text).expect_err(bad).to_string();
            assert!(err.contains("mini/game.toml") && err.contains("[[gate]] row 2"), "names the file and row: {err}");
            if !bad.is_empty() {
                assert!(err.contains(&format!("`{bad}`")), "names the pattern: {err}");
            }
        }
        // Control: the same file with a defined pattern reads.
        let ok = format!("{MINI}\n        [[gate]]\n        modules = \"games.mini.**\"\n        place_in = \"all\"\n");
        assert_eq!(parse_game(Path::new("mini/game.toml"), &ok).unwrap().gate.len(), 1);
    }

    /// G0b final review A-I4: an older reader meeting a newer file is told the format, not
    /// an unknown or missing key. A `format = 2` file that also adds a key, or drops one, is
    /// refused as a format mismatch; so is a newer engine.toml and baseline.toml.
    #[test]
    fn a_format_mismatch_is_named_before_any_key_error() {
        let added = MINI.replace("format = 1", "format = 2\nnew_in_format_two = true");
        let dropped = MINI.replace("format = 1", "format = 2").replace("require_one_text = false\n", "");
        for (what, text) in [("an added key", added), ("a dropped key", dropped)] {
            assert!(text.contains("format = 2"), "{what}: the mutation did not apply");
            let err = parse_game(Path::new("mini/game.toml"), &text).expect_err(what);
            assert!(matches!(err, DescError::Format { found: 2, .. }), "{what}: {err}");
            assert!(err.to_string().contains("this reader accepts format = 1"), "{what}: {err}");
        }
        let dir = crate::build_dir::BuildDir::create().unwrap();
        for (name, text) in [("engine.toml", "format = 3\nfrom_the_future = 1\n"), ("baseline.toml", "format = 3\nnew_list = []\n")] {
            let path = dir.path().join(name);
            std::fs::write(&path, text).unwrap();
            let err = if name == "engine.toml" { load_engine(&path).map(|_| ()) } else { load_baseline(&path).map(|_| ()) };
            assert!(matches!(err, Err(DescError::Format { found: 3, .. })), "{name}: {err:?}");
        }
        // Control: format 1 with an unknown key is still refused by the key's name.
        let unknown = MINI.replace("format = 1", "format = 1\nnew_in_format_two = true");
        assert!(parse_game(Path::new("g"), &unknown).unwrap_err().to_string().contains("new_in_format_two"));
    }

    #[test]
    fn feature_expressions_parse_and_evaluate() {
        let f = Features { debug: false, crash_report: true, sound: false };
        let defs = |n: &str| (n == "HOTKEYS").then_some(1);
        let ev = |s: &str| FeatureExpr::parse(s).unwrap().eval(&f, &defs).unwrap();
        assert!(ev("debug || crash_report"));
        assert!(!ev("!(debug || crash_report)"));
        assert!(ev("!sound"));
        assert!(ev("HOTKEYS"));
        assert!(!ev("debug"));
        assert!(FeatureExpr::parse("debug ||").is_err());
        assert!(FeatureExpr::parse("Debug").is_err());
        assert!(FeatureExpr::parse("MISSING").unwrap().eval(&f, &defs).is_err(), "an unknown define is loud");
    }

    #[test]
    fn engine_dir_follows_suite_paths_precedence() {
        let tmp = Path::new(env!("CARGO_MANIFEST_DIR"));
        // The crate dir has a `src` subdirectory: stand it in for an engine root.
        let r = EngineRef { suite_path: "aeon".into(), root: "src".into() };
        let set = |k: &'static str, v: &'static Path| move |n: &str| (n == k).then(|| v.as_os_str().to_owned());
        assert_eq!(resolve_engine_dir(&r, &set("AEON_DIR", tmp)).unwrap(), (tmp.to_path_buf(), EngineStep::CheckoutVar));
        let parent = tmp.parent().unwrap();
        let r2 = EngineRef { suite_path: "sigil-build".into(), root: "src".into() };
        assert_eq!(
            resolve_engine_dir(&r2, &set("EMPYREAN_SUITE_ROOT", parent)).unwrap(),
            (tmp.to_path_buf(), EngineStep::SuiteRoot)
        );
        // Set but wrong is a hard error at its own step, not a fall-through.
        let wrong = |n: &str| match n {
            "AEON_DIR" => Some(parent.as_os_str().to_owned()),
            "EMPYREAN_SUITE_ROOT" => Some(parent.as_os_str().to_owned()),
            _ => None,
        };
        let err = resolve_engine_dir(&r, &wrong).unwrap_err().to_string();
        assert!(err.contains("AEON_DIR"), "{err}");
        assert!(resolve_engine_dir(&r, &|_| None).unwrap_err().to_string().contains("AEON_DIR"));
    }
}
