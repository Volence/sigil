//! THE MODULE ROOTS ARE THE MAP'S, NOT A LIST'S (`sigil_build::roots::derive_roots`).
//!
//! Two halves. The HERMETIC half builds tiny trees under this test binary's target
//! scratch directory and drives `derive_roots` over each rule on its own: a row resolves
//! by label, by section and by compiler-minted name; a `[[gate]]` rule filters; another
//! game's twin is never a candidate; a duplicate label is settled by the `use` graph or
//! refused; a row nothing defines is refused. The REFERENCE half runs the same derivation
//! over the pinned aeon tree through the driver's own entry point
//! (`sigil_build::driver::module_roots`, via `sigil_harness::shapes::module_registry`) and
//! doctors a copy of it:
//!
//!   * a module whose map row is removed while its SOURCE STAYS drops out of the
//!     derived list, and nothing else moves. A list derived from the files present on
//!     disk would keep it, so this is the witness that presence does not decide;
//!   * a module whose source is removed while its ROW STAYS is refused by name, never
//!     dropped silently.
//!
//! The doctored module is CHOSEN from the tree, never named here: the last game-scope row
//! whose single owner is placed in every shape (`place_in = all`), declares one section,
//! owns no other row, and is `use`d by no module. So the gate follows the tree across a
//! pin advance instead of describing one revision.
//!
//! G0b Task 11 moved this file off the profile-driven driver's `module_roots` (its
//! `SHAPE_GATES` list is deleted); the hermetic trees now state their gates as rules.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sigil_build::describe::{FeatureExpr, Features, GateRule, PlaceIn, ResolvedShape};
use sigil_build::place::ModuleSpec;
use sigil_build::roots::{declared_sections, derive_roots, RootsInput};
use sigil_frontend_emp::resolve::imports::use_decls;
use sigil_frontend_emp::resolve::manifest::Manifest;
use sigil_harness::map_placement::load_placement_map;
use sigil_harness::shapes::{self, Shape};
use sigil_harness::test_support::{reference_tree_for_shape, shadow_aeon_tree};
use sigil_span::roots::RootName;

// ── The hermetic half ─────────────────────────────────────────────────────────────

const TERMINUS: &str = "engine.epilogue";

fn shape(debug: bool, sound: bool) -> ResolvedShape {
    ResolvedShape {
        name: "release".into(),
        features: Features { debug, crash_report: true, sound },
        defines: BTreeMap::new(),
        fixture: false,
        fixture_placement: None,
    }
}

fn plain() -> ResolvedShape {
    shape(false, true)
}

fn rule(modules: &str, place_in: PlaceIn) -> GateRule {
    GateRule { modules: modules.into(), place_in, origin: String::new(), row: 0 }
}

/// Every hermetic module placed in every shape, except `gated` (each with its rule).
fn rules(gated: &[(&str, PlaceIn)]) -> Vec<GateRule> {
    let mut out: Vec<GateRule> = gated.iter().map(|(m, p)| rule(m, p.clone())).collect();
    for m in [
        "engine.epilogue",
        "engine.alpha",
        "engine.twin",
        "games.*.header",
        "games.sonic4.beta",
        "games.sonic4.solid",
        "games.sonic4.solid_copy",
        "games.sonic4.user",
    ] {
        out.push(rule(m, PlaceIn::All));
    }
    out
}

/// A fresh tree of `(relative path, source)` files under the target scratch dir.
fn mini_tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("module_roots_{name}"));
    let _ = std::fs::remove_dir_all(&root);
    for (rel, src) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, src).unwrap();
    }
    root
}

/// The scan a build of `game` makes: the engine root and that game's root, named.
fn scan(root: &Path, game: &str) -> Manifest {
    let (m, diags) = Manifest::scan_roots(&[(RootName::Engine, root.join("engine")), (RootName::Game, root.join("games").join(game))]);
    let errs: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errs.is_empty(), "the fixture tree must parse: {errs:?}");
    m
}

fn derive(m: &Manifest, order: &[String], prefix: &str, sh: &ResolvedShape, rules: &[GateRule], map: &str) -> Result<Vec<ModuleSpec>, String> {
    derive_roots(&RootsInput { manifest: m, order, game_prefix: prefix, rules, shape: sh, seeds: &[], terminus: TERMINUS, map_name: map })
        .map_err(|e| e.to_string())
}

fn rows(r: &[&str]) -> Vec<String> {
    r.iter().map(|s| s.to_string()).collect()
}

fn ids(specs: &[ModuleSpec]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in specs {
        if !out.contains(&s.module_id) {
            out.push(s.module_id.clone());
        }
    }
    out
}

/// The files every hermetic tree shares: the engine terminus, one engine module with
/// a PRIVATE head proc, one with two nested sections, and the sonic4 and demo twins of
/// a header (the cross-game label every real map row `GameHeader` names).
fn base_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("engine/epilogue.emp", "module engine.epilogue in epilogue\n\npub data EndOfRom = Data.empty\n"),
        (
            "engine/alpha.emp",
            "module engine.alpha in alpha\n\nproc Alpha_Private () clobbers() {\n    rts\n}\n",
        ),
        (
            "engine/twin.emp",
            "module engine.twin\n\nsection twin_head (cpu: m68000) {\n    pub data TwinHead: u16 = 0\n}\n\nsection twin_tail (cpu: m68000) {\n    pub data TwinTail: u16 = 0\n}\n",
        ),
        ("games/sonic4/header.emp", "module games.sonic4.header in header\n\npub data GameHeader: u16 = 0\n"),
        ("games/demo/header.emp", "module games.demo.header in header\n\npub data GameHeader: u16 = 1\n"),
        (
            "games/sonic4/beta.emp",
            "module games.sonic4.beta in beta\n\npub data Beta: u16 = 0\n\nalign 2\n",
        ),
    ]
}

/// Every row kind resolves, a private head label included, and roots come out in map
/// order with the terminus appended when no row names it. The two-section module
/// yields one spec per section; another game's twin is never a candidate.
#[test]
fn every_row_kind_resolves_to_its_module_in_map_order() {
    let root = mini_tree("kinds", &base_files());
    let m = scan(&root, "sonic4");
    let r = rules(&[]);
    let order = rows(&["GameHeader", "Alpha_Private", "TwinHead", "section:beta", "TwinTail"]);
    let got = derive(&m, &order, "games.sonic4", &plain(), &r, "map").unwrap();
    assert_eq!(
        ids(&got),
        vec!["games.sonic4.header", "engine.alpha", "engine.twin", "games.sonic4.beta", TERMINUS],
        "{got:?}"
    );
    let twin: Vec<&str> = got.iter().filter(|s| s.module_id == "engine.twin").map(|s| s.section.as_str()).collect();
    assert_eq!(twin, vec!["twin_head", "twin_tail"]);
    // The demo game's twin is the demo header, never sonic4's.
    let dm = scan(&root, "demo");
    let demo = derive(&dm, &rows(&["GameHeader"]), "games.demo", &plain(), &r, "map").unwrap();
    assert_eq!(ids(&demo), vec!["games.demo.header", TERMINUS]);
    // A compiler-minted label names its module between the first two `$`.
    let minted = derive(&m, &rows(&["__align$games.sonic4.beta$0"]), "games.sonic4", &plain(), &r, "map").unwrap();
    assert_eq!(ids(&minted), vec!["games.sonic4.beta", TERMINUS]);
}

/// The map without a module's row derives the list without that module, the file
/// still on disk. A list read off the files present would keep it.
#[test]
fn a_map_without_a_row_derives_the_smaller_list_with_the_file_still_present() {
    let root = mini_tree("smaller", &base_files());
    let m = scan(&root, "sonic4");
    let r = rules(&[]);
    let full = rows(&["GameHeader", "Alpha_Private", "TwinHead", "section:beta"]);
    let before = derive(&m, &full, "games.sonic4", &plain(), &r, "map").unwrap();
    let without: Vec<String> = full.iter().filter(|r| *r != "section:beta").cloned().collect();
    let after = derive(&m, &without, "games.sonic4", &plain(), &r, "map").unwrap();
    assert!(root.join("games/sonic4/beta.emp").exists(), "control: the file stays");
    let expected: Vec<ModuleSpec> = before.iter().filter(|s| s.module_id != "games.sonic4.beta").cloned().collect();
    assert_eq!(after, expected);
}

/// A row nothing in scope defines is refused and named, not dropped.
#[test]
fn a_row_no_module_defines_is_refused_by_name() {
    let root = mini_tree("orphan", &base_files());
    let m = scan(&root, "sonic4");
    let e = derive(&m, &rows(&["GameHeader", "ObjDef_Gone"]), "games.sonic4", &plain(), &rules(&[]), "games/sonic4/map.toml")
        .unwrap_err();
    assert!(e.contains("[map.order-orphan]"), "{e}");
    assert!(e.contains("ObjDef_Gone"), "must name the row: {e}");
    assert!(e.contains("games/sonic4/map.toml"), "must name the map: {e}");
    assert!(!e.contains('\u{2014}') && !e.contains('\u{2013}'), "no em/en dash in tool text: {e}");
}

/// A gated module is placed exactly in the shapes its rule admits; a row whose only
/// owner is gated out is dropped for that shape without an error.
#[test]
fn a_shape_gate_filters_the_derived_list() {
    let mut files = base_files();
    files.push((
        "engine/debug/selftest.emp",
        "module engine.compression_selftest in compression_selftest\n\npub data CompressionSelfTest: u16 = 0\n",
    ));
    let root = mini_tree("gate", &files);
    let m = scan(&root, "sonic4");
    let r = rules(&[("engine.compression_selftest", PlaceIn::Feature(FeatureExpr::Debug))]);
    let order = rows(&["GameHeader", "CompressionSelfTest"]);
    let plain_roots = derive(&m, &order, "games.sonic4", &plain(), &r, "map").unwrap();
    assert!(!ids(&plain_roots).contains(&"engine.compression_selftest".to_string()), "{plain_roots:?}");
    let dbg = derive(&m, &order, "games.sonic4", &shape(true, true), &r, "map").unwrap();
    assert!(ids(&dbg).contains(&"engine.compression_selftest".to_string()), "{dbg:?}");
}

/// A module gated on `sound` under both ids its song bank may carry: a sound-off shape
/// roots neither, a sound-on shape roots each.
#[test]
fn the_song_bank_is_sound_on_only_under_both_ids() {
    let mut files = base_files();
    files.push((
        "games/sonic4/data/sound/mt_bank.emp",
        "module games.sonic4.mt_bank in mt_bank\n\npub data MtBankHead: u16 = 0\n",
    ));
    files.push((
        "games/sonic4/data/sound/mt_bank_blob.emp",
        "module games.sonic4.mt_bank_blob in mt_bank_blob\n\npub data MtBankBlobHead: u16 = 0\n",
    ));
    let root = mini_tree("song_bank_ids", &files);
    let m = scan(&root, "sonic4");
    let r = rules(&[("games.sonic4.mt_bank*", PlaceIn::Feature(FeatureExpr::Sound))]);
    let order = rows(&["GameHeader", "MtBankHead", "MtBankBlobHead"]);
    let off = ids(&derive(&m, &order, "games.sonic4", &shape(false, false), &r, "map").unwrap());
    let on = ids(&derive(&m, &order, "games.sonic4", &plain(), &r, "map").unwrap());
    for id in ["games.sonic4.mt_bank", "games.sonic4.mt_bank_blob"] {
        assert!(!off.contains(&id.to_string()), "a sound-off shape must not root `{id}`: {off:?}");
        assert!(on.contains(&id.to_string()), "a sound-on shape must root `{id}`: {on:?}");
    }
}

/// Two in-scope definitions of one label: the one the rest of the build `use`s is the
/// one placed; with neither reached, the row is refused naming both.
#[test]
fn a_duplicate_label_is_settled_by_the_use_graph_or_refused() {
    let mut files = base_files();
    files.push(("games/sonic4/solid.emp", "module games.sonic4.solid in solid\n\npub data Solid_Init: u16 = 0\n"));
    files.push(("games/sonic4/copy.emp", "module games.sonic4.solid_copy in solid\n\npub data Solid_Init: u16 = 0\n"));
    let unreached = mini_tree("dup_unreached", &files);
    let m = scan(&unreached, "sonic4");
    let r = rules(&[]);
    let order = rows(&["GameHeader", "Solid_Init"]);
    let e = derive(&m, &order, "games.sonic4", &plain(), &r, "map").unwrap_err();
    assert!(e.contains("[map.order-ambiguous]"), "{e}");
    assert!(e.contains("games.sonic4.solid") && e.contains("games.sonic4.solid_copy"), "{e}");

    files.push(("games/sonic4/user.emp", "module games.sonic4.user in user\n\nuse games.sonic4.solid\n\npub data User: u16 = 0\n"));
    let reached = mini_tree("dup_reached", &files);
    let m = scan(&reached, "sonic4");
    let order = rows(&["GameHeader", "User", "Solid_Init"]);
    let got = derive(&m, &order, "games.sonic4", &plain(), &r, "map").unwrap();
    assert_eq!(ids(&got), vec!["games.sonic4.header", "games.sonic4.user", "games.sonic4.solid", TERMINUS]);
}

// ── The reference half ────────────────────────────────────────────────────────────

/// `map_src` with `row` removed from its root `order` array. The edit is textual; the
/// proof it removed exactly that row is the parse after it, compared to the original
/// order minus the row.
fn remove_order_row(map_src: &str, row: &str) -> String {
    let start = map_src.find("\norder = [").expect("the map has a root `order` array") + 1;
    let end = start + map_src[start..].find("\n]").expect("the `order` array closes");
    let body = &map_src[start..end];
    let q = format!("\"{row}\"");
    let doctored_body = [format!("{q}, "), format!("{q},"), format!(", {q}"), q.clone()]
        .iter()
        .find(|pat| body.matches(pat.as_str()).count() == 1)
        .map(|pat| body.replacen(pat.as_str(), "", 1))
        .unwrap_or_else(|| panic!("`{row}` is not exactly one entry of the order array"));
    let doctored = format!("{}{}{}", &map_src[..start], doctored_body, &map_src[end..]);
    let parsed = load_placement_map(&doctored).expect("the doctored map still parses");
    let original = load_placement_map(map_src).unwrap();
    let expected: Vec<String> = original.order.iter().filter(|r| *r != row).cloned().collect();
    assert_eq!(parsed.order, expected, "the doctored map must differ by exactly the one row");
    doctored
}

/// The doctored module and its row, chosen from the tree (see the file header).
struct Victim {
    module: String,
    row: String,
    source_rel: String,
}

/// The top-level labels a module defines (`data`/`proc`/`offsets`/`dispatch`/`script`/
/// `table` names, sections included), the labels an `order` row names.
fn labels_of(items: &[sigil_frontend_emp::ast::Item], out: &mut Vec<String>) {
    use sigil_frontend_emp::ast::Item;
    for item in items {
        let named = match item {
            Item::Data(d) => Some(&d.name),
            Item::Proc(p) => Some(&p.name),
            Item::Offsets(o) => Some(&o.name),
            Item::Dispatch(d) => Some(&d.name),
            Item::Script(s) => Some(&s.name),
            Item::Table(t) => Some(&t.name),
            _ => None,
        };
        if let Some(n) = named {
            out.push(n.clone());
        }
        if let Item::Section(sec) = item {
            labels_of(&sec.items, out);
        }
    }
}

fn choose_victim(aeon: &Path, s: &Shape, map_src: &str) -> Victim {
    let p = s.project(aeon).expect("project");
    let (m, _) = p.scan();
    let prefix = p.game_prefix().to_string();
    let rules: Vec<GateRule> = p.engine.gate.iter().chain(p.game.gate.iter()).cloned().collect();
    let placed_everywhere = |id: &str| {
        let hits: Vec<&GateRule> = rules.iter().filter(|r| sigil_build::gating::glob_match(&r.modules, id)).collect();
        matches!(hits.as_slice(), [r] if matches!(r.place_in, PlaceIn::All))
    };
    let mut owners_of: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for (i, pm) in m.modules.iter().enumerate() {
        if pm.root.is_none() {
            continue;
        }
        let mut labels = Vec::new();
        labels_of(&pm.file.items, &mut labels);
        for l in labels {
            let o = owners_of.entry(l).or_default();
            if !o.contains(&i) {
                o.push(i);
            }
        }
    }
    let order = load_placement_map(map_src).unwrap().order;
    let used: Vec<String> = m
        .modules
        .iter()
        .flat_map(|pm| use_decls(&pm.file.items).into_iter().map(|u| u.base.segments.join(".")))
        .collect();
    // A row's single owner, by each row kind the derivation resolves: a compiler-minted
    // row names its module between the first two `$`, a `section:` row every module that
    // declares the section, any other row the modules defining the label.
    let owner = |row: &str| -> Option<usize> {
        if let Some(rest) = row.strip_prefix("__") {
            let module = rest.split('$').nth(1)?;
            return m.by_id.get(module).copied();
        }
        if let Some(sec) = row.strip_prefix("section:") {
            let hits: Vec<usize> = (0..m.modules.len())
                .filter(|&i| m.modules[i].root.is_some() && declared_sections(&m.modules[i].file).iter().any(|d| d == sec))
                .collect();
            return if let [one] = hits.as_slice() { Some(*one) } else { None };
        }
        match owners_of.get(row).map(Vec::as_slice) {
            Some([one]) => Some(*one),
            _ => None,
        }
    };
    for row in order.iter().rev() {
        if row.starts_with("section:") || row.starts_with("__") {
            continue;
        }
        let Some(i) = owner(row) else { continue };
        let pm = &m.modules[i];
        let rows_owned = order.iter().filter(|r| owner(r) == Some(i)).count();
        if pm.id.starts_with(&format!("{prefix}."))
            && placed_everywhere(&pm.id)
            && rows_owned == 1
            && declared_sections(&pm.file).len() == 1
            && !used.contains(&pm.id)
        {
            let canon_aeon = std::fs::canonicalize(aeon).unwrap_or_else(|_| aeon.to_path_buf());
            let path = std::fs::canonicalize(&pm.path).unwrap_or_else(|_| pm.path.clone());
            let source_rel = path.strip_prefix(&canon_aeon).unwrap().to_string_lossy().into_owned();
            return Victim { module: pm.id.clone(), row: row.clone(), source_rel };
        }
    }
    panic!("no ungated, unused, single-row, single-section game module in {}", aeon.display());
}

/// The pinned tree, doctored: the victim's map row removed, its source KEPT. The
/// derived list loses exactly the victim.
#[test]
fn the_pinned_tree_without_a_map_row_derives_exactly_the_smaller_list() {
    let profile = Shape::new("sonic4", "release");
    let Some(aeon) = reference_tree_for_shape(&profile) else { return };
    let map_rel = "games/sonic4/map.toml";
    let map_src = std::fs::read_to_string(aeon.join(map_rel)).unwrap();
    let victim = choose_victim(&aeon, &profile, &map_src);
    eprintln!("doctored: row `{}` of module `{}` ({})", victim.row, victim.module, victim.source_rel);

    let before = shapes::module_registry(&aeon, &profile).expect("the pinned tree derives");
    assert!(
        before.iter().any(|s| s.module_id == victim.module),
        "control: `{}` is placed before the row is removed",
        victim.module
    );

    let doctored = remove_order_row(&map_src, &victim.row);
    let shadow = shadow_aeon_tree(&aeon, &[(map_rel, &doctored)]).unwrap();
    assert!(
        shadow.root().join(&victim.source_rel).exists(),
        "control: the source `{}` is still on disk",
        victim.source_rel
    );
    let after = shapes::module_registry(shadow.root(), &profile).expect("the doctored tree derives");
    let expected: Vec<ModuleSpec> = before.iter().filter(|s| s.module_id != victim.module).cloned().collect();
    let gone: Vec<&ModuleSpec> = expected.iter().filter(|s| !after.contains(s)).collect();
    let new: Vec<&ModuleSpec> = after.iter().filter(|s| !expected.contains(s)).collect();
    assert_eq!(
        after, expected,
        "removing row `{}` must remove `{}` and move nothing else (missing {gone:?}, extra {new:?})",
        victim.row, victim.module
    );
}

/// The pinned tree, doctored the other way: the victim's SOURCE removed, its row
/// KEPT. The derivation refuses, naming the row and the map.
#[test]
fn the_pinned_tree_with_a_row_but_no_source_is_refused_by_name() {
    let profile = Shape::new("sonic4", "release");
    let Some(aeon) = reference_tree_for_shape(&profile) else { return };
    let map_src = std::fs::read_to_string(aeon.join("games/sonic4/map.toml")).unwrap();
    let victim = choose_victim(&aeon, &profile, &map_src);

    let shadow = shadow_aeon_tree(&aeon, &[]).unwrap();
    let file = shadow.root().join(&victim.source_rel);
    std::fs::remove_file(&file).unwrap_or_else(|e| panic!("remove {}: {e}", file.display()));
    assert!(!file.exists(), "control: the source is gone from the copy");
    let e = shapes::module_registry(shadow.root(), &profile).expect_err("a row whose module is gone must not derive");
    assert!(e.contains("[map.order-orphan]"), "{e}");
    assert!(e.contains(&format!("- {}", victim.row)), "must name row `{}`: {e}", victim.row);
    assert!(e.contains("map.toml"), "must name the map: {e}");
}

/// The pinned tree with the song bank declared `module games.sonic4.mt_bank`, the id
/// its path names: the sound-off shape (config-b) does not root it, the sound-on shape
/// (release) does, through the driver's own derivation.
#[test]
fn the_pinned_tree_roots_the_song_bank_by_its_path_id_only_when_sound_is_on() {
    let plain = Shape::new("sonic4", "release");
    let Some(aeon) = reference_tree_for_shape(&plain) else { return };
    let bank_rel = "games/sonic4/data/sound/mt_bank.emp";
    let new_id = "games.sonic4.mt_bank";
    let src = std::fs::read_to_string(aeon.join(bank_rel))
        .unwrap_or_else(|e| panic!("the song bank `{bank_rel}` must be in the tree: {e}"));
    let old_decl = "\nmodule games.sonic4.mt_bank_blob\n";
    let new_decl = format!("\nmodule {new_id}\n");
    let doctored = if src.contains(old_decl) { src.replacen(old_decl, &new_decl, 1) } else { src.clone() };
    assert!(doctored.contains(&new_decl), "control: `{bank_rel}` declares `{new_id}` in the copy");
    let shadow = shadow_aeon_tree(&aeon, &[(bank_rel, &doctored)]).unwrap();

    let on = shapes::module_registry(shadow.root(), &plain).expect("the sound-on shape derives");
    assert!(on.iter().any(|s| s.module_id == new_id), "control: the sound-on shape roots `{new_id}`");
    let off = shapes::module_registry(shadow.root(), &Shape::new("sonic4", "config-b")).expect("the sound-off shape derives");
    let rooted: Vec<&str> = off.iter().filter(|s| s.module_id == new_id).map(|s| s.section.as_str()).collect();
    assert!(rooted.is_empty(), "the sound-off shape must not root `{new_id}`, it placed {rooted:?}");
}
