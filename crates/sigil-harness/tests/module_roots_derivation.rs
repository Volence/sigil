//! THE MODULE ROOTS ARE THE MAP'S, NOT A LIST'S (`sigil_harness::module_roots`).
//!
//! Two halves. The HERMETIC half builds tiny trees under this test binary's target
//! scratch directory and drives `derive_module_roots` over each rule on its own: a
//! row resolves by label, by section and by compiler-minted name; a shape gate
//! filters; another game's twin is never a candidate; a duplicate label is settled by
//! the `use` graph or refused; a row nothing defines is refused. The REFERENCE half
//! runs the same derivation over the pinned aeon tree through the build's own entry
//! point (`native::module_registry`) and doctors a copy of it:
//!
//!   * a module whose map row is removed while its SOURCE STAYS drops out of the
//!     derived list, and nothing else moves. A list derived from the files present on
//!     disk would keep it, so this is the witness that presence does not decide;
//!   * a module whose source is removed while its ROW STAYS is refused by name, never
//!     dropped silently.
//!
//! The doctored module is CHOSEN from the tree, never named here: the last ungated
//! game-scope row whose single owner declares one section, owns no other row, and is
//! `use`d by no module. So the gate follows the tree across a pin advance instead of
//! describing one revision.

use std::path::{Path, PathBuf};

use sigil_frontend_emp::resolve::imports::use_decls;
use sigil_frontend_emp::resolve::manifest::Manifest;
use sigil_harness::map_placement::load_placement_map;
use sigil_harness::module_roots::{
    derive_module_roots, gate_of, ModuleSpec, RowIndex, Shape, ENGINE_TERMINUS,
};
use sigil_harness::native;
use sigil_harness::test_support::{reference_tree_for_profile, shadow_aeon_tree};

// ── The hermetic half ─────────────────────────────────────────────────────────────

const PLAIN: Shape = Shape {
    debug: false,
    crash_report: true,
    sound_on: true,
    sound_debug_hotkeys: false,
    sound_dbg_mirror: false,
};

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

fn scan(root: &Path) -> Manifest {
    let (m, diags) = Manifest::scan(root);
    let errs: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errs.is_empty(), "the fixture tree must parse: {errs:?}");
    m
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
/// contributes one spec per section.
#[test]
fn every_row_kind_resolves_to_its_module_in_map_order() {
    let root = mini_tree("kinds", &base_files());
    let m = scan(&root);
    let order = rows(&["GameHeader", "Alpha_Private", "TwinHead", "section:beta", "TwinTail"]);
    let got = derive_module_roots(&m, &order, "games.sonic4", &PLAIN, &[], "map").unwrap();
    assert_eq!(
        ids(&got),
        vec!["games.sonic4.header", "engine.alpha", "engine.twin", "games.sonic4.beta", ENGINE_TERMINUS],
        "{got:?}"
    );
    let twin: Vec<&str> =
        got.iter().filter(|s| s.module_id == "engine.twin").map(|s| s.section.as_str()).collect();
    assert_eq!(twin, vec!["twin_head", "twin_tail"]);
    // The demo game's twin is the demo header, never sonic4's.
    let demo = derive_module_roots(&m, &rows(&["GameHeader"]), "games.demo", &PLAIN, &[], "map").unwrap();
    assert_eq!(ids(&demo), vec!["games.demo.header", ENGINE_TERMINUS]);
    // A compiler-minted label names its module between the first two `$`.
    let minted = derive_module_roots(
        &m,
        &rows(&["__align$games.sonic4.beta$0"]),
        "games.sonic4",
        &PLAIN,
        &[],
        "map",
    )
    .unwrap();
    assert_eq!(ids(&minted), vec!["games.sonic4.beta", ENGINE_TERMINUS]);
}

/// The map without a module's row derives the list without that module, the file
/// still on disk. A list read off the files present would keep it.
#[test]
fn a_map_without_a_row_derives_the_smaller_list_with_the_file_still_present() {
    let root = mini_tree("smaller", &base_files());
    let m = scan(&root);
    let full = rows(&["GameHeader", "Alpha_Private", "TwinHead", "section:beta"]);
    let before = derive_module_roots(&m, &full, "games.sonic4", &PLAIN, &[], "map").unwrap();
    let without: Vec<String> = full.iter().filter(|r| *r != "section:beta").cloned().collect();
    let after = derive_module_roots(&m, &without, "games.sonic4", &PLAIN, &[], "map").unwrap();
    assert!(root.join("games/sonic4/beta.emp").exists(), "control: the file stays");
    let expected: Vec<ModuleSpec> =
        before.iter().filter(|s| s.module_id != "games.sonic4.beta").cloned().collect();
    assert_eq!(after, expected);
}

/// A row nothing in scope defines is refused and named, not dropped.
#[test]
fn a_row_no_module_defines_is_refused_by_name() {
    let root = mini_tree("orphan", &base_files());
    let m = scan(&root);
    let e = derive_module_roots(
        &m,
        &rows(&["GameHeader", "ObjDef_Gone"]),
        "games.sonic4",
        &PLAIN,
        &[],
        "games/sonic4/map.toml",
    )
    .unwrap_err();
    assert!(e.contains("[map.order-orphan]"), "{e}");
    assert!(e.contains("ObjDef_Gone"), "must name the row: {e}");
    assert!(e.contains("games/sonic4/map.toml"), "must name the map: {e}");
    assert!(!e.contains('\u{2014}') && !e.contains('\u{2013}'), "no em/en dash in tool text: {e}");
}

/// A gated module is placed exactly in the shapes its gate admits; a row whose only
/// owner is gated out is dropped for that shape without an error.
#[test]
fn a_shape_gate_filters_the_derived_list() {
    let mut files = base_files();
    files.push((
        "engine/debug/selftest.emp",
        "module engine.compression_selftest in compression_selftest\n\npub data CompressionSelfTest: u16 = 0\n",
    ));
    let root = mini_tree("gate", &files);
    let m = scan(&root);
    assert!(gate_of("engine.compression_selftest").is_some(), "control: the module is gated");
    let order = rows(&["GameHeader", "CompressionSelfTest"]);
    let plain = derive_module_roots(&m, &order, "games.sonic4", &PLAIN, &[], "map").unwrap();
    assert!(!ids(&plain).contains(&"engine.compression_selftest".to_string()), "{plain:?}");
    let debug = Shape { debug: true, ..PLAIN };
    let dbg = derive_module_roots(&m, &order, "games.sonic4", &debug, &[], "map").unwrap();
    assert!(ids(&dbg).contains(&"engine.compression_selftest".to_string()), "{dbg:?}");
}

/// Two in-scope definitions of one label: the one the rest of the build `use`s is the
/// one placed; with neither reached, the row is refused naming both.
#[test]
fn a_duplicate_label_is_settled_by_the_use_graph_or_refused() {
    let mut files = base_files();
    files.push(("games/sonic4/solid.emp", "module games.sonic4.solid in solid\n\npub data Solid_Init: u16 = 0\n"));
    files.push((
        "games/sonic4/copy.emp",
        "module games.sonic4.solid_copy in solid\n\npub data Solid_Init: u16 = 0\n",
    ));
    let unreached = mini_tree("dup_unreached", &files);
    let m = scan(&unreached);
    let order = rows(&["GameHeader", "Solid_Init"]);
    let e = derive_module_roots(&m, &order, "games.sonic4", &PLAIN, &[], "map").unwrap_err();
    assert!(e.contains("[map.order-ambiguous]"), "{e}");
    assert!(e.contains("games.sonic4.solid") && e.contains("games.sonic4.solid_copy"), "{e}");

    files.push(("games/sonic4/user.emp", "module games.sonic4.user in user\n\nuse games.sonic4.solid\n\npub data User: u16 = 0\n"));
    let reached = mini_tree("dup_reached", &files);
    let m = scan(&reached);
    let order = rows(&["GameHeader", "User", "Solid_Init"]);
    let got = derive_module_roots(&m, &order, "games.sonic4", &PLAIN, &[], "map").unwrap();
    assert_eq!(
        ids(&got),
        vec!["games.sonic4.header", "games.sonic4.user", "games.sonic4.solid", ENGINE_TERMINUS]
    );
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

fn choose_victim(aeon: &Path, profile: &native::GameProfile, map_src: &str) -> Victim {
    let (m, _) = Manifest::scan(aeon);
    let prefix = profile.game_module_prefix();
    let index = RowIndex::new(&m, prefix);
    let order = load_placement_map(map_src).unwrap().order;
    let used: Vec<String> = m
        .modules
        .iter()
        .flat_map(|pm| use_decls(&pm.file.items).into_iter().map(|u| u.base.segments.join(".")))
        .collect();
    for row in order.iter().rev() {
        let owners = index.owners(row);
        let [module] = owners.as_slice() else { continue };
        let pm = &m.modules[m.by_id[*module]];
        let rows_owned = order.iter().filter(|r| index.owners(r) == vec![*module]).count();
        if module.starts_with(&format!("{prefix}."))
            && gate_of(module).is_none()
            && rows_owned == 1
            && sigil_harness::module_roots::declared_sections(&pm.file).len() == 1
            && !used.iter().any(|u| u == module)
            && !row.starts_with("section:")
            && !row.starts_with("__")
        {
            let source_rel = pm.path.strip_prefix(aeon).unwrap().to_string_lossy().into_owned();
            eprintln!("victim: row `{row}` -> module `{module}` ({source_rel}) in {}", aeon.display());
            return Victim { module: module.to_string(), row: row.clone(), source_rel };
        }
    }
    panic!("no ungated, unused, single-row, single-section game module in {}", aeon.display());
}

/// The pinned tree, doctored: the victim's map row removed, its source KEPT. The
/// derived list loses exactly the victim.
#[test]
fn the_pinned_tree_without_a_map_row_derives_exactly_the_smaller_list() {
    let profile = native::sonic4_profile(false);
    let Some(aeon) = reference_tree_for_profile(&profile) else { return };
    let map_rel = "games/sonic4/map.toml";
    let map_src = std::fs::read_to_string(aeon.join(map_rel)).unwrap();
    let victim = choose_victim(&aeon, &profile, &map_src);

    let before = native::module_registry(&aeon, &profile).expect("the pinned tree derives");
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
    let after = native::module_registry(shadow.root(), &profile).expect("the doctored tree derives");
    let expected: Vec<ModuleSpec> = before.iter().filter(|s| s.module_id != victim.module).cloned().collect();
    assert_eq!(
        after, expected,
        "removing row `{}` must remove `{}` and move nothing else",
        victim.row, victim.module
    );
}

/// The pinned tree, doctored the other way: the victim's SOURCE removed, its row
/// KEPT. The build refuses, naming the row and the map.
#[test]
fn the_pinned_tree_with_a_row_but_no_source_is_refused_by_name() {
    let profile = native::sonic4_profile(false);
    let Some(aeon) = reference_tree_for_profile(&profile) else { return };
    let map_src = std::fs::read_to_string(aeon.join("games/sonic4/map.toml")).unwrap();
    let victim = choose_victim(&aeon, &profile, &map_src);

    let shadow = shadow_aeon_tree(&aeon, &[]).unwrap();
    let file = shadow.root().join(&victim.source_rel);
    std::fs::remove_file(&file).unwrap_or_else(|e| panic!("remove {}: {e}", file.display()));
    assert!(!file.exists(), "control: the source is gone from the copy");
    let e = native::module_registry(shadow.root(), &profile)
        .expect_err("a row whose module is gone must not derive");
    assert!(e.contains("[map.order-orphan]"), "{e}");
    assert!(e.contains(&format!("- {}", victim.row)), "must name row `{}`: {e}", victim.row);
    assert!(e.contains("map.toml"), "must name the map: {e}");
}
