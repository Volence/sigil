//! G0b Task 3: the G0b descriptions say exactly what the Rust driver says.
//!
//! G0b moves the per-game build facts out of the Rust profiles (`native.rs`,
//! `module_roots.rs`, `contract_baseline.rs`, `section_align.rs`, `sound_bank_ids.rs`)
//! into description files in the aeon tree: `engine/engine.toml`, `games/<id>/game.toml`
//! and the `baseline.toml` each names. Before any later task switches the driver over to
//! them, this test proves that every (game, shape) row the Rust constructors build has a
//! description shape that resolves to the same facts, field by field, and vice versa.
//!
//! WHERE EACH SIDE COMES FROM
//!
//! * The Rust rows are ENUMERATED FROM SOURCE: every `pub fn *_profile(..) -> GameProfile`
//!   in `native.rs`, a `debug: bool` constructor contributing two rows. A constructor this
//!   file cannot call (a new one) is a failure, never a silent skip. Each row's game is
//!   read from its own `manifest_module`, and its description shape name is the
//!   constructor's stem (`config_a_keystones_flipped_profile` -> `config-a-keystones-flipped`),
//!   or `release`/`debug` for a `debug: bool` constructor. The constructor, not the
//!   profile's `name`, is the key: `config_a_keystones_flipped_profile` returns
//!   `config_a_profile()` verbatim, `name` included.
//! * Rust tables that are `pub` are linked. Tables that are private (`COMPTIME_HELPERS`,
//!   `RAM_HELPERS`, `STRUCT_OFFSET_TWINS`, `GROWABLE`, the sound labels) are read out of
//!   the Rust source as text, never retyped here.
//! * The descriptions are read from the aeon reference tree (`AEON_DIR`) with
//!   `sigil_build::describe`, through the A8 guard (`test_support::reference_tree`): a tree
//!   without them skips, and is a failure under `SIGIL_STRICT_GATE=1`.
//!
//! Each category counts what it compared, the test prints `compared N facts across M
//! shapes` (M computed), and a category that compared nothing is a failure.

use sigil_build::describe::{
    load_baseline, load_engine, load_game, resolve_shape, BaselineDesc, EngineDesc, GameDesc, GateRule, PlaceIn,
    ResolvedShape, DERIVED_DEFINES,
};
use sigil_harness::contract_baseline as cb;
use sigil_harness::module_roots::{gate_of, SHAPE_GATES, SIGIL_PROBE_FIXTURES};
use sigil_harness::native::{self, GameProfile};
use sigil_harness::{section_align, sound_bank_ids, test_support};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ── the Rust source, read as text for the private tables ──

fn harness_src(file: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read the Rust authority {}: {e}", p.display()))
}

/// The string literals on one line (a trailing `//` comment stripped).
fn literals(line: &str) -> Vec<String> {
    let code = line.split("//").next().unwrap_or("");
    code.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

/// The rows of the slice constant whose declaration line contains `decl`: one entry per
/// non-comment line, each the string literals on it. Loud when absent or empty.
fn rust_rows(src: &str, decl: &str) -> Vec<Vec<String>> {
    let mut lines = src.lines().skip_while(|l| !l.contains(decl));
    let head = lines.next().unwrap_or_else(|| panic!("Rust authority lost `{decl}`"));
    if let Some(body) = head.split_once("= &[").map(|(_, b)| b).filter(|b| b.contains("];")) {
        let row = literals(body);
        assert!(!row.is_empty(), "`{decl}` read as empty, which is unmeasured, not empty");
        return vec![row];
    }
    let rows: Vec<Vec<String>> = lines
        .take_while(|l| !l.trim_start().starts_with("];"))
        .filter(|l| !l.trim_start().starts_with("//"))
        .map(literals)
        .filter(|v| !v.is_empty())
        .collect();
    assert!(!rows.is_empty(), "`{decl}` read as empty, which is unmeasured, not empty");
    rows
}

fn rust_list(src: &str, decl: &str) -> Vec<String> {
    rust_rows(src, decl).into_iter().flatten().collect()
}

/// The body of `fn name(` in `src`, up to the next top-level item.
fn fn_body<'a>(src: &'a str, name: &str) -> &'a str {
    let at = src.find(&format!("fn {name}(")).unwrap_or_else(|| panic!("Rust authority lost `fn {name}`"));
    let rest = &src[at..];
    let end = rest.find("\n}\n").unwrap_or(rest.len());
    &rest[..end]
}

/// The one literal of `const NAME: &str = "..";` in `src`.
fn rust_str_const(src: &str, name: &str) -> String {
    let line = src
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("const {name}: &str")))
        .unwrap_or_else(|| panic!("Rust authority lost `const {name}`"));
    let lits = literals(line);
    assert_eq!(lits.len(), 1, "`const {name}` read as {lits:?}");
    lits[0].clone()
}

// ── the Rust rows, enumerated from source ──

/// One Rust (game, shape) row.
struct RustRow {
    ctor: String,
    game: String,
    shape: String,
    profile: GameProfile,
}

/// Call the constructor `ctor` (with `arg` for a `debug: bool` one). `None` is a
/// constructor this file has no dispatch for, which the caller reports as a failure.
fn construct(ctor: &str, arg: Option<bool>) -> Option<GameProfile> {
    Some(match (ctor, arg) {
        ("sonic4_profile", Some(d)) => native::sonic4_profile(d),
        ("demo_profile", Some(d)) => native::demo_profile(d),
        ("config_b_profile", None) => native::config_b_profile(),
        ("config_a_keystones_flipped_profile", None) => native::config_a_keystones_flipped_profile(),
        ("config_a_profile", None) => native::config_a_profile(),
        ("lean_profile", None) => native::lean_profile(),
        ("stress_evict_profile", None) => native::stress_evict_profile(),
        ("stress_art_profile", None) => native::stress_art_profile(),
        _ => return None,
    })
}

/// `games.<g>.<x>` -> `<g>`.
fn game_of_module(id: &str) -> String {
    let mut it = id.split('.');
    assert_eq!(it.next(), Some("games"), "`{id}` is not a game module id");
    it.next().unwrap_or_else(|| panic!("`{id}` names no game")).to_string()
}

fn shape_name(ctor: &str, arg: Option<bool>) -> String {
    match arg {
        Some(true) => "debug".to_string(),
        Some(false) => "release".to_string(),
        None => ctor.trim_end_matches("_profile").replace('_', "-"),
    }
}

/// The constructor lines the row enumeration reads: one-line `pub fn NAME(..) -> GameProfile`
/// signatures at column zero.
fn ctor_lines(src: &str) -> Vec<&str> {
    src.lines().filter(|l| l.starts_with("pub fn ") && l.contains("-> GameProfile")).collect()
}

/// Every free function returning `GameProfile` in `src`, found WHITESPACE-INSENSITIVELY (a
/// signature wrapped over several lines included). Methods (`self` first) are not
/// constructors. Comment lines are dropped first, so prose naming the type is not a site.
fn every_profile_fn(src: &str) -> BTreeSet<String> {
    let code: Vec<&str> = src.lines().filter(|l| !l.trim_start().starts_with("//")).collect();
    let flat = code.join(" ").split_whitespace().collect::<Vec<_>>().join(" ").replace("->GameProfile", "-> GameProfile");
    let mut out = BTreeSet::new();
    let mut from = 0;
    while let Some(off) = flat[from..].find("-> GameProfile") {
        let at = from + off;
        from = at + "-> GameProfile".len();
        if flat[from..].chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue; // a longer type name
        }
        let prefix = &flat[..at];
        let Some(fn_at) = prefix.rfind("fn ").filter(|&i| i == 0 || prefix[..i].ends_with(' ')) else {
            out.insert(format!("<unparsed `-> GameProfile` at flat offset {at}>"));
            continue;
        };
        let sig = &prefix[fn_at + 3..];
        let name = sig.split('(').next().unwrap_or("").trim().to_string();
        let params = sig.split_once('(').map(|(_, r)| r).unwrap_or("").trim_start();
        if params.starts_with("self") || params.starts_with("mut self") || params.starts_with("&self") || params.starts_with("&mut self") {
            continue;
        }
        out.insert(name);
    }
    out
}

/// The constructor census: the one-line enumeration must see every free `-> GameProfile`
/// function the source declares, or a wrapped signature would drop its rows silently.
fn ctor_census(src: &str) -> Result<(), String> {
    let seen: BTreeSet<String> =
        ctor_lines(src).iter().map(|l| l["pub fn ".len()..].split('(').next().unwrap().to_string()).collect();
    let all = every_profile_fn(src);
    if seen != all {
        return Err(format!(
            "native.rs declares {} free `-> GameProfile` function(s) {all:?} but the one-line constructor \
             enumeration saw {} {seen:?}: missed {:?} (a wrapped or reformatted signature)",
            all.len(),
            seen.len(),
            all.difference(&seen).collect::<Vec<_>>()
        ));
    }
    Ok(())
}

/// Every Rust row, plus the constructors found (for the report line).
fn rust_rows_from_source(errs: &mut Vec<String>) -> (Vec<RustRow>, usize) {
    let src = harness_src("native.rs");
    if let Err(e) = ctor_census(&src) {
        errs.push(format!("[shapes] {e}"));
    }
    let mut rows = Vec::new();
    let mut ctors = 0usize;
    for line in ctor_lines(&src) {
        let ctor = line["pub fn ".len()..].split('(').next().unwrap().to_string();
        let params = line.split_once('(').unwrap().1.split_once(')').unwrap().0.trim();
        let args: Vec<Option<bool>> = match params {
            "" => vec![None],
            "debug: bool" => vec![Some(false), Some(true)],
            other => {
                errs.push(format!("constructor `{ctor}` takes `{other}`, a signature this test cannot enumerate"));
                continue;
            }
        };
        ctors += 1;
        for arg in args {
            let Some(profile) = construct(&ctor, arg) else {
                errs.push(format!(
                    "constructor `{ctor}` (native.rs) has no dispatch in this test: add it here AND a shape to its game's game.toml"
                ));
                continue;
            };
            let game = game_of_module(profile.manifest_module);
            if arg.is_some() {
                assert_eq!(ctor.trim_end_matches("_profile"), game, "a `debug: bool` constructor is its game's own");
            }
            rows.push(RustRow { shape: shape_name(&ctor, arg), ctor: ctor.clone(), game, profile });
        }
    }
    assert!(!rows.is_empty(), "native.rs read as no profile constructor, which is unmeasured");
    (rows, ctors)
}

/// The (game, shape) pairs `shipped_shapes` builds, read from its body's constructor calls.
fn shipped_from_source() -> BTreeSet<(String, String)> {
    let src = harness_src("native.rs");
    let body = fn_body(&src, "shipped_shapes");
    let mut out = BTreeSet::new();
    for line in body.lines().skip(1) {
        for piece in line.split(", ").filter(|p| p.contains("_profile(")) {
            let call = piece.trim().trim_end_matches("),").trim_end_matches(')');
            let ctor = call.split('(').next().unwrap().rsplit(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap();
            let argtxt = call.split_once('(').map(|(_, a)| a.trim()).unwrap_or("");
            let arg = match argtxt {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
            let p = construct(ctor, arg).unwrap_or_else(|| panic!("shipped_shapes calls `{ctor}`, unknown here"));
            out.insert((game_of_module(p.manifest_module), shape_name(ctor, arg)));
        }
    }
    assert!(!out.is_empty(), "shipped_shapes read as empty, which is unmeasured");
    out
}

// ── the description side ──

struct GameSide {
    dir_rel: PathBuf,
    desc: GameDesc,
    baseline: BaselineDesc,
}

/// The driver's own matcher (G0b QUEUE 7b: this file used an exact-or-single-`*` stand-in
/// until the driver's landed), so this test and the build read one pattern one way.
use sigil_build::gating::glob_match;

/// The description's placement of module `id` in `shape`: `Ok(None)` when no rule names it
/// (placed in every shape), `Ok(Some(b))` from the one rule that does.
fn desc_place(rules: &[&GateRule], id: &str, shape: &ResolvedShape, engine: &EngineDesc) -> Result<Option<bool>, String> {
    let hits: Vec<&&GateRule> = rules.iter().filter(|r| glob_match(&r.modules, id)).collect();
    match hits.as_slice() {
        [] => Ok(None),
        [r] => Ok(Some(match &r.place_in {
            PlaceIn::All => true,
            PlaceIn::Shapes(s) => s.contains(&shape.name),
            PlaceIn::Feature(e) => {
                let define = |k: &str| shape.defines.get(k).or_else(|| engine.defines.get(k)).copied();
                e.eval(&shape.features, &define)?
            }
        })),
        many => Err(format!("{} gate rules match `{id}`: {:?}", many.len(), many.iter().map(|r| &r.modules).collect::<Vec<_>>())),
    }
}

/// The full comptime define set a description shape builds with: the engine's defaults,
/// the game's rows and the shape's overrides (`resolve_shape`), then the defines derived
/// from features (`describe::DERIVED_DEFINES`).
fn desc_defines(engine: &EngineDesc, shape: &ResolvedShape) -> BTreeMap<String, i64> {
    let mut d = engine.defines.clone();
    d.extend(shape.defines.iter().map(|(k, v)| (k.clone(), *v)));
    let derived = [
        ("DEBUG", shape.features.debug),
        ("CRASH_REPORT", shape.features.crash_report),
        ("SOUND_DRIVER_ENABLED", shape.features.sound),
    ];
    for (k, on) in derived {
        assert!(DERIVED_DEFINES.contains(&k), "`{k}` is no longer a derived define in describe.rs");
        d.insert(k.to_string(), on as i64);
    }
    d
}

fn norm(p: &Path) -> String {
    let s: Vec<String> = p
        .components()
        .filter(|c| !matches!(c, std::path::Component::CurDir))
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    s.join("/")
}

fn sorted<T: Ord + Clone>(v: &[T]) -> Vec<T> {
    let mut v = v.to_vec();
    v.sort();
    v
}

fn pairs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

/// Facts compared, by category, and every disagreement.
#[derive(Default)]
struct Tally {
    counts: BTreeMap<&'static str, usize>,
    errs: Vec<String>,
}

impl Tally {
    /// One fact compared in `cat`; `ok` false records `why`.
    fn check(&mut self, cat: &'static str, ok: bool, why: impl FnOnce() -> String) {
        *self.counts.entry(cat).or_default() += 1;
        if !ok {
            self.errs.push(format!("[{cat}] {}", why()));
        }
    }

    /// A multiset of rows: one fact per row on either side (an empty pair of lists is one fact).
    fn rows<T: Ord + Clone + std::fmt::Debug>(&mut self, cat: &'static str, what: &str, rust: &[T], desc: &[T]) {
        let (r, d) = (sorted(rust), sorted(desc));
        *self.counts.entry(cat).or_default() += r.len().max(d.len()).max(1);
        if r != d {
            let only_r: Vec<&T> = r.iter().filter(|x| r.iter().filter(|y| y == x).count() > d.iter().filter(|y| y == x).count()).collect();
            let only_d: Vec<&T> = d.iter().filter(|x| d.iter().filter(|y| y == x).count() > r.iter().filter(|y| y == x).count()).collect();
            self.errs.push(format!("[{cat}] {what}: Rust-only {only_r:?}, description-only {only_d:?}"));
        }
    }
}

const CATEGORIES: &[&str] = &[
    "shapes", "features", "defines", "entry", "game", "gate rows", "placement", "align", "baseline", "engine",
];

#[test]
fn descriptions_agree_with_rust() {
    let mut t = Tally::default();
    let (rows, ctors) = rust_rows_from_source(&mut t.errs);
    let games: BTreeSet<String> = rows.iter().map(|r| r.game.clone()).collect();

    let mut needed: Vec<String> = vec!["engine/engine.toml".to_string()];
    needed.extend(games.iter().map(|g| format!("games/{g}/game.toml")));
    let needed_ref: Vec<&str> = needed.iter().map(String::as_str).collect();
    let Some(aeon) = test_support::reference_tree(&needed_ref) else {
        return;
    };

    // ── the descriptions ──
    let engine_dir = aeon.join("engine");
    let engine = load_engine(&engine_dir.join("engine.toml")).unwrap_or_else(|e| panic!("engine.toml: {e}"));
    let engine_base = load_baseline(&engine_dir.join(&engine.baseline)).unwrap_or_else(|e| panic!("engine baseline: {e}"));

    // Every game the tree describes, so a description game with no Rust row is caught too.
    let mut described: BTreeSet<String> = BTreeSet::new();
    for ent in std::fs::read_dir(aeon.join("games")).expect("read games/") {
        let ent = ent.expect("games/ entry");
        if ent.path().join("game.toml").is_file() {
            described.insert(ent.file_name().to_string_lossy().into_owned());
        }
    }
    t.rows("shapes", "games (Rust profile games vs described games)", &games.iter().cloned().collect::<Vec<_>>(), &described.iter().cloned().collect::<Vec<_>>());

    let mut sides: BTreeMap<String, GameSide> = BTreeMap::new();
    for g in &games {
        let dir_rel = PathBuf::from("games").join(g);
        let desc = load_game(&aeon.join(&dir_rel).join("game.toml")).unwrap_or_else(|e| panic!("games/{g}/game.toml: {e}"));
        let baseline =
            load_baseline(&aeon.join(&dir_rel).join(&desc.baseline)).unwrap_or_else(|e| panic!("games/{g} baseline: {e}"));
        t.check("game", desc.id == *g, || format!("games/{g}/game.toml has id `{}`", desc.id));
        t.check("game", norm(&desc.engine.root) == "engine", || format!("games/{g}: engine root `{}`, the Rust reads `engine/`", desc.engine.root.display()));
        sides.insert(g.clone(), GameSide { dir_rel, desc, baseline });
    }

    // ── shapes: a bijection by name, per game ──
    for g in &games {
        let rust: Vec<String> = rows.iter().filter(|r| &r.game == g).map(|r| r.shape.clone()).collect();
        let desc: Vec<String> = sides[g].desc.shapes.keys().cloned().collect();
        t.rows("shapes", &format!("{g} shapes (Rust constructors vs game.toml)"), &rust, &desc);
    }
    let shipped = shipped_from_source();

    // ── the full module scan, once ──
    let (manifest, mdiags) = sigil_frontend_emp::resolve::manifest::Manifest::scan(&aeon);
    let merr: Vec<_> = mdiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(merr.is_empty(), "manifest scan of {}: {} error(s); first: {:?}", aeon.display(), merr.len(), merr.first());
    assert!(!manifest.modules.is_empty(), "the module scan found nothing, which is unmeasured");

    let mut shapes_compared = 0usize;
    for row in &rows {
        let p = &row.profile;
        let g = &row.game;
        let side = &sides[g];
        let at = format!("{g}/{} ({}, profile `{}`)", row.shape, row.ctor, p.name);
        let Ok(shape) = resolve_shape(&side.desc, &row.shape) else {
            t.errs.push(format!("[shapes] {at}: no description shape `{}` resolves", row.shape));
            continue;
        };
        shapes_compared += 1;

        // features
        let f = shape.features;
        t.check("features", f.debug == p.debug, || format!("{at}: debug Rust {} vs description {}", p.debug, f.debug));
        t.check("features", f.crash_report == p.crash_report, || format!("{at}: crash_report Rust {} vs description {}", p.crash_report, f.crash_report));
        t.check("features", f.sound == p.sound_on, || format!("{at}: sound Rust {} vs description {}", p.sound_on, f.sound));

        // defines: every key on either side
        let mut rust_defs: BTreeMap<String, i64> = BTreeMap::new();
        for (k, v) in &p.emp_defines {
            t.check("defines", rust_defs.insert(k.to_string(), *v as i64).is_none(), || format!("{at}: Rust repeats define `{k}`"));
        }
        let desc_defs = desc_defines(&engine, &shape);
        let keys: BTreeSet<&String> = rust_defs.keys().chain(desc_defs.keys()).collect();
        for k in keys {
            let (r, d) = (rust_defs.get(k), desc_defs.get(k));
            t.check("defines", r == d, || format!("{at}: define `{k}` Rust {r:?} vs description {d:?}"));
        }

        // entry modules and the game's source paths
        let e = &side.desc.entry;
        let gdir = &side.dir_rel;
        let rel = |o: &Option<PathBuf>| o.as_ref().map(|r| norm(&gdir.join(r)));
        t.check("entry", e.game == p.manifest_module, || format!("{at}: manifest module Rust `{}` vs entry.game `{}`", p.manifest_module, e.game));
        t.check("entry", e.ram == p.game_ram_module, || format!("{at}: game RAM module Rust `{}` vs entry.ram `{}`", p.game_ram_module, e.ram));
        for (what, r, d) in [
            ("constants", p.game_constants_rel, rel(&e.constants)),
            ("sound_ids", p.game_sound_ids_rel, rel(&e.sound_ids)),
            ("sfx_bank", p.game_sfx_bank_rel, rel(&e.sfx_bank)),
        ] {
            let r = r.map(str::to_string);
            t.check("entry", r == d, || format!("{at}: {what} Rust {r:?} vs entry.{what} {d:?}"));
        }
        let residual = norm(&gdir.join(&side.desc.sources.root).join(&side.desc.sources.residual_as));
        t.check("entry", residual == p.game_root_rel, || format!("{at}: residual AS root Rust `{}` vs sources `{residual}`", p.game_root_rel));
        let map_desc = aeon.join(gdir).join(&side.desc.map);
        t.check("entry", norm(&map_desc) == norm(&p.map_path(&aeon)), || format!("{at}: map Rust {} vs description {}", p.map_path(&aeon).display(), map_desc.display()));
        let prefix_desc = e.game.rsplit_once('.').map(|(a, _)| a).unwrap_or("");
        t.check("entry", prefix_desc == p.game_module_prefix(), || format!("{at}: module scope Rust `{}` vs description `{prefix_desc}`", p.game_module_prefix()));

        // game scalars
        t.check("game", side.desc.require_one_text == p.require_one_text, || format!("{at}: require_one_text Rust {} vs description {}", p.require_one_text, side.desc.require_one_text));
        // native.rs build_full_file_chained: the deb2 floor by game path.
        let floor = if p.game_root_rel.contains("/demo/") { native::DEMO_APPENDIX_FLOOR } else { native::SONIC4_APPENDIX_FLOOR };
        t.check("game", side.desc.appendix_floor as usize == floor, || format!("{at}: appendix floor Rust {floor:#x} vs description {:#x}", side.desc.appendix_floor));
        let is_shipped = shipped.contains(&(g.clone(), row.shape.clone()));
        t.check("game", shape.fixture == !is_shipped, || format!("{at}: in shipped_shapes: {is_shipped}, so fixture must be {}, description fixture {}", !is_shipped, shape.fixture));
        t.check("game", shape.fixture_placement.is_some() == p.fixture_placement, || format!("{at}: fixture_placement Rust {} vs description {:?}", p.fixture_placement, shape.fixture_placement));
        if let Some(fp) = &shape.fixture_placement {
            let growable = rust_list(fn_body(&harness_src("native.rs"), "relocate_fixture_pool"), "const GROWABLE");
            t.check("game", fp.relocate == growable, || format!("{at}: relocate Rust GROWABLE {growable:?} vs description {:?}", fp.relocate));
        }
        if row.shape == "release" {
            t.check("game", side.desc.name == p.name, || format!("{at}: name Rust `{}` vs description `{}`", p.name, side.desc.name));
        }

        // placement over the FULL module scan: Rust SHAPE_GATES + SIGIL_PROBE_FIXTURES + scope,
        // description gate rules + scope. No rule = every shape, on both sides.
        let rshape = native::module_shape(p);
        let rules: Vec<&GateRule> = engine.gate.iter().chain(&side.desc.gate).collect();
        let game_scope = format!("{}.", p.game_module_prefix());
        let desc_scope = format!("{prefix_desc}.");
        for m in &manifest.modules {
            let id = m.id.as_str();
            let rust_in = (id.starts_with("engine.") || id.starts_with(&game_scope)) && !SIGIL_PROBE_FIXTURES.contains(&id);
            let rust = rust_in && gate_of(id).is_none_or(|gt| gt.admits(&rshape));
            let desc_in = id.starts_with("engine.") || id.starts_with(&desc_scope);
            // Another game's module (outside both scopes) is no fact about this shape.
            if !(rust_in || desc_in || id.starts_with(&game_scope)) {
                continue;
            }
            let desc = match desc_place(&rules, id, &shape, &engine) {
                Ok(v) => desc_in && v.unwrap_or(true),
                Err(err) => {
                    t.errs.push(format!("[placement] module `{id}` in shape {at}: {err}"));
                    continue;
                }
            };
            t.check("placement", rust == desc, || format!("module `{id}` in shape {at}: Rust places it {rust}, the description {desc}"));
        }

        // gate rows: the two gate tables, row by row, so a gated id the tree no longer has
        // is still compared (the scan above cannot see it).
        let in_game = |id: &str| id.starts_with("engine.") || id.starts_with(&game_scope);
        let rust_ids: BTreeSet<String> = SHAPE_GATES
            .iter()
            .map(|(id, _)| *id)
            .chain(SIGIL_PROBE_FIXTURES.iter().copied())
            .filter(|id| in_game(id))
            .map(str::to_string)
            .collect();
        // SHAPE_GATES lists only modules some shape DROPS; the closed description gate also
        // names every always-placed module (`place_in = "all"`, G0b Task 5), and those rows
        // have no Rust twin. The placement scan above still compares every module.
        let desc_ids: BTreeSet<String> =
            rules.iter().filter(|r| r.place_in != PlaceIn::All).map(|r| r.modules.clone()).collect();
        t.rows("gate rows", &format!("{at}: gated module ids (SHAPE_GATES + SIGIL_PROBE_FIXTURES vs gate rules)"), &rust_ids.iter().cloned().collect::<Vec<_>>(), &desc_ids.iter().cloned().collect::<Vec<_>>());
        for id in rust_ids.intersection(&desc_ids) {
            let rust = !SIGIL_PROBE_FIXTURES.contains(&id.as_str()) && gate_of(id).is_none_or(|gt| gt.admits(&rshape));
            match desc_place(&rules, id, &shape, &engine) {
                Ok(v) => {
                    let desc = v.unwrap_or(true);
                    t.check("gate rows", rust == desc, || format!("gate for module `{id}` in shape {at}: Rust admits {rust}, the description {desc}"));
                }
                Err(err) => t.errs.push(format!("[gate rows] module `{id}` in shape {at}: {err}")),
            }
        }

        // alignment: every description row of this shape's engine + game is a Rust
        // exception with the same quantum (the global bijection is below).
        for a in engine.align.iter().chain(&side.desc.align) {
            let r = section_align::DECLARED.iter().find(|d| d.label == a.label).map(|d| d.required);
            t.check("align", r == Some(a.align), || format!("{at}: align `{}` Rust {r:?} vs description {:#x}", a.label, a.align));
        }

        // baseline: Rust's per-shape gate vs the union of the description files.
        // UNTIL TASK 9 (sigil docs/QUEUE.md G0B-DESCRIBE-TASK2-RESIDUE item 3): today's
        // closure analyses the WHOLE aeon corpus on every shape, so a demo shape's gate is
        // engine + sonic4 + demo rows. The Task 9 two-root corpus removes this union, and
        // demo then compares against engine + demo alone, like every other game.
        let mut files: Vec<&BaselineDesc> = vec![&engine_base, &side.baseline];
        if g == "demo" {
            files.insert(1, &sides["sonic4"].baseline);
        }
        let z80: Vec<(String, String)> = files.iter().flat_map(|b| b.z80_out_unverified.clone()).collect();
        let out: Vec<(String, String)> = files.iter().flat_map(|b| b.out_unverified.clone()).collect();
        let inout: Vec<(String, String)> = files.iter().flat_map(|b| b.inout_unverified.clone()).collect();
        let mut d1c: Vec<(String, String, String)> = files.iter().flat_map(|b| b.d1c.clone()).collect();
        if f.debug {
            d1c.extend(files.iter().flat_map(|b| b.d1c_debug.clone()));
        }
        let rust_d1c: Vec<(String, String, String)> = cb::d1c_baseline(p.debug, cb::is_demo_family(p))
            .into_iter()
            .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()))
            .collect();
        t.rows("baseline", &format!("{at}: z80 out-unverified"), &pairs(cb::Z80_OUT_UNVERIFIED_BASELINE), &z80);
        t.rows("baseline", &format!("{at}: out-unverified"), &pairs(cb::OUT_UNVERIFIED_BASELINE), &out);
        t.rows("baseline", &format!("{at}: inout-unverified"), &pairs(cb::INOUT_UNVERIFIED_BASELINE), &inout);
        t.rows("baseline", &format!("{at}: d1c live-clobbered"), &rust_d1c, &d1c);
    }

    // ── alignment, globally: the Rust exceptions (stronger than the 68000 word rule, which
    // the descriptions leave to the driver) are exactly the descriptions' rows ──
    let rust_align: Vec<(String, u32)> =
        section_align::DECLARED.iter().filter(|d| d.required != 2).map(|d| (d.label.to_string(), d.required)).collect();
    let desc_align: Vec<(String, u32)> = engine
        .align
        .iter()
        .chain(sides.values().flat_map(|s| &s.desc.align))
        .map(|a| (a.label.clone(), a.align))
        .collect();
    t.rows("align", "alignment exceptions (section_align DECLARED vs engine + games)", &rust_align, &desc_align);

    // ── engine lists ──
    let nsrc = harness_src("native.rs");
    t.check("engine", rust_list(&nsrc, "const COMPTIME_HELPERS") == engine.helpers, || {
        format!("helpers (ORDER is the fact) Rust {:?} vs engine.toml {:?}", rust_list(&nsrc, "const COMPTIME_HELPERS"), engine.helpers)
    });
    *t.counts.entry("engine").or_default() += engine.helpers.len().saturating_sub(1);
    t.check("engine", rust_list(&nsrc, "const RAM_HELPERS") == engine.ram_helpers, || {
        format!("ram_helpers Rust {:?} vs engine.toml {:?}", rust_list(&nsrc, "const RAM_HELPERS"), engine.ram_helpers)
    });
    *t.counts.entry("engine").or_default() += engine.ram_helpers.len().saturating_sub(1);
    let twins: Vec<(String, String, String)> = rust_rows(&nsrc, "const STRUCT_OFFSET_TWINS")
        .into_iter()
        .map(|r| (r[0].strip_prefix("engine/").unwrap_or(&r[0]).to_string(), r[1].clone(), r[2].clone()))
        .collect();
    t.check("engine", twins == engine.struct_twins, || format!("struct_twins Rust {twins:?} vs engine.toml {:?}", engine.struct_twins));
    *t.counts.entry("engine").or_default() += engine.struct_twins.len().saturating_sub(1);
    let seeds_line = fn_body(&nsrc, "registry_from_manifest").lines().find(|l| l.contains("let seeds")).expect("registry_from_manifest lost `let seeds`");
    let seeds = literals(seeds_line);
    t.check("engine", seeds == engine.entry_seeds, || format!("entry seeds Rust {seeds:?} vs engine.toml {:?}", engine.entry_seeds));
    t.check("engine", engine.terminus == sigil_harness::module_roots::ENGINE_TERMINUS, || format!("terminus Rust `{}` vs `{}`", sigil_harness::module_roots::ENGINE_TERMINUS, engine.terminus));
    t.check("engine", engine.error_handler == sigil_harness::module_roots::ERROR_HANDLER_MODULE_ID, || format!("error_handler `{}`", engine.error_handler));
    t.check("engine", engine.release_fault == sigil_harness::module_roots::RELEASE_FAULT_MODULE_ID, || format!("release_fault `{}`", engine.release_fault));
    t.check("engine", engine.error_handler_blob == native::ERROR_HANDLER_BLOB_LABEL, || format!("error_handler_blob `{}`", engine.error_handler_blob));
    let heads_line = fn_body(&nsrc, "relocate_fixture_pool").lines().find(|l| l.contains(".position(|s| s ==") && l.contains('"')).expect("relocate_fixture_pool lost its island test");
    let heads = literals(heads_line);
    t.check("engine", heads == engine.fault_island_heads, || format!("fault_island_heads Rust {heads:?} vs engine.toml {:?}", engine.fault_island_heads));
    let lst = fn_body(&nsrc, "sigil_native_symbol_listing");
    let req: Vec<String> = lst.lines().filter(|l| l.contains(".get(\"")).flat_map(literals).collect();
    t.check("engine", req == engine.required_labels, || format!("required_labels Rust {req:?} vs engine.toml {:?}", engine.required_labels));
    let first_join = |body: &str| body.split("aeon.join(\"").nth(1).and_then(|s| s.split('"').next()).map(|s| s.trim_start_matches("engine/").to_string());
    let consts = first_join(fn_body(&nsrc, "harvest_engine_constants"));
    t.check("engine", consts.as_deref() == Some(&*norm(&engine.constants)), || format!("constants Rust {consts:?} vs `{}`", engine.constants.display()));
    let types = first_join(fn_body(&nsrc, "harvest_engine_struct_offsets"));
    t.check("engine", types.as_deref() == Some(&*norm(&engine.types)), || format!("types Rust {types:?} vs `{}`", engine.types.display()));
    // [sound]
    let ssrc = harness_src("sound_bank_ids.rs");
    for (name, desc) in [
        ("BLOB_START", &engine.sound.blob_start),
        ("BLOB_END", &engine.sound.blob_end),
        ("DAC_HEAD", &engine.sound.dac_head),
        ("DAC_BLIP_BANK", &engine.sound.dac_blip_bank),
        ("DAC_SHARED_BANK", &engine.sound.dac_shared_bank),
    ] {
        let r = rust_str_const(&ssrc, name);
        t.check("engine", &r == desc, || format!("sound {name} Rust `{r}` vs engine.toml `{desc}`"));
    }
    let ids: Vec<&str> = engine.sound.blob_bank_ids.iter().map(|b| b.name.as_str()).collect();
    t.check("engine", ids == sound_bank_ids::BLOB_BANK_ID_CONSTS, || format!("blob bank ids Rust {:?} vs engine.toml {ids:?}", sound_bank_ids::BLOB_BANK_ID_CONSTS));
    let banks_body = fn_body(&ssrc, "blob_const_bank_labels");
    for b in &engine.sound.blob_bank_ids {
        let r: Vec<String> = banks_body
            .lines()
            .find(|l| l.trim_start().starts_with(&format!("\"{}\" =>", b.name)))
            .map(|l| literals(l).into_iter().skip(1).collect())
            .unwrap_or_default();
        t.check("engine", r == b.banks, || format!("sound bank labels of `{}` Rust {r:?} vs engine.toml {:?}", b.name, b.banks));
    }

    // ── the verdict ──
    let total: usize = t.counts.values().sum();
    for (cat, n) in &t.counts {
        eprintln!("  {cat:<10} {n} facts");
    }
    for cat in CATEGORIES {
        if t.counts.get(cat).copied().unwrap_or(0) == 0 {
            t.errs.push(format!("category `{cat}` compared ZERO facts, which is unmeasured, not agreement"));
        }
    }
    println!(
        "compared {total} facts across {shapes_compared} shapes ({} Rust rows from {ctors} constructors)",
        rows.len()
    );
    assert!(
        t.errs.is_empty(),
        "the descriptions disagree with the Rust driver in {} place(s):\n  - {}",
        t.errs.len(),
        t.errs.join("\n  - ")
    );
}

/// The constructor census sees a wrapped signature the one-line enumeration misses. A unit
/// fixture: the real `native.rs` text with `config_b_profile`'s signature wrapped by hand.
#[test]
fn ctor_census_refuses_a_wrapped_signature() {
    let src = harness_src("native.rs");
    ctor_census(&src).expect("the real native.rs passes the census");
    let one_line = "pub fn config_b_profile() -> GameProfile {";
    assert!(src.contains(one_line), "native.rs lost `{one_line}`, the fixture needs a new victim");
    let wrapped = src.replacen(one_line, "pub fn config_b_profile(\n) -> GameProfile {", 1);
    assert!(!ctor_lines(&wrapped).iter().any(|l| l.contains("config_b_profile")), "the fixture is not wrapped");
    let err = ctor_census(&wrapped).expect_err("a wrapped constructor must fail the census");
    assert!(err.contains("missed [\"config_b_profile\"]"), "{err}");
}
