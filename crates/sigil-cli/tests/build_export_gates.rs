//! G0a Task 10, the landing-bar gates over the finished export.
//!
//!  * gate 6, `every_export_validates_against_the_vendored_schema`: the sidecar of all
//!    seven shapes validates against `contract/build/empyrean.build-1.schema.json`;
//!  * gate 7, `five_builds_one_export_hash_per_shape`: five builds of each shape, each in
//!    its OWN `cp -a` copy of the reference tree at a DIFFERENT directory path, write the
//!    same sidecar bytes and the same `rom.sha256`. A directory name or an absolute path
//!    that leaks into the export, a hash-ordered iteration that feeds a record, a
//!    timestamp or a process id all break it;
//!  * gate 8, `demo_export_validates_and_has_no_sonic_symbol_in_engine_records`: the second
//!    game. The demo shapes validate and carry nothing of `games.sonic4`; and no
//!    `engine.` record of ANY shape names a `games.sonic4` item.
//!  * `tool_revision_names_the_checkout_head`: `tool.revision` is the commit the binary
//!    was built from or the honest `unknown`, never a stale one.
//!
//! Real `sigil build` runs over COPIES of the reference tree (`AEON_DIR`), because
//! `sigil build` rewrites files in the tree it builds. `SIGIL_STRICT_GATE=1` hard-fails a
//! missing tree. python3 + jsonschema are REQUIRED: missing fails, never skips.
//!
//! Runner: these are ordinary tests, so `scripts/landing-run.sh` (`cargo test --workspace`
//! in release and in the CI-shaped debug leg) runs them, and
//! `scripts/nightly_source_gates.sh` lists this file in `SOURCE_GATES` because it reads
//! source only (no committed artifact, golden, ROM or listing). They are not `#[ignore]`:
//! `landing-run.sh` passes no `--include-ignored`, so an ignored test would never run in
//! the landing bar.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

use serde_json::{json, Value};
use sigil_harness::test_support::{copy_tree, schema_errors};

static LOCK: Mutex<()> = Mutex::new(());
static AEON: OnceLock<Option<PathBuf>> = OnceLock::new();
static BUILT: OnceLock<Vec<Built>> = OnceLock::new();

/// The seven shapes `scripts/step0_shapes_cmp.sh` proves byte-identical, by name and flags.
const SHAPES: [(&str, &[&str]); 7] = [
    ("plain", &[]),
    ("debug", &["--debug"]),
    ("demo", &["--game", "demo"]),
    ("demo_debug", &["--game", "demo", "--debug"]),
    ("config_a", &["--config-a"]),
    ("config_b", &["--config-b"]),
    ("lean", &["--lean"]),
];

/// Builds per shape in gate 7.
const BUILDS: usize = 5;

/// The most shapes gate 7 builds at once.
const MAX_PARALLEL: usize = 3;

fn skip(why: &str) {
    eprintln!("{}{why}", sigil_harness::test_support::SKIP_MARKER);
}

fn scratch(what: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("build_export_gates_{what}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn aeon() -> Option<&'static Path> {
    AEON.get_or_init(|| {
        let profile = sigil_harness::native::sonic4_profile(false);
        sigil_harness::test_support::reference_tree_for_profile(&profile)
    })
    .as_deref()
}

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

/// One `sigil build` of `flags` over `tree`, ROM written to `rom`, run with `cwd` as its
/// working directory. Returns the sidecar bytes and the ROM bytes.
fn build(tree: &Path, flags: &[&str], rom: &Path, cwd: &Path) -> (Vec<u8>, Vec<u8>) {
    std::fs::create_dir_all(rom.parent().unwrap()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["build", "--aeon"])
        .arg(tree)
        .args(flags)
        .arg("-o")
        .arg(rom)
        .current_dir(cwd)
        .output()
        .expect("spawn sigil");
    assert!(
        out.status.success(),
        "build {flags:?} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sidecar =
        std::fs::read(rom.with_extension("build.json")).expect("the build wrote no sidecar");
    (sidecar, std::fs::read(rom).expect("rom"))
}

/// One built shape, kept in the shared tree for gates 6 and 8.
struct Built {
    name: &'static str,
    sidecar_path: PathBuf,
    export: Value,
    /// The tree copy the shape was built in: its `files` rows resolve against it.
    tree: PathBuf,
}

/// All seven shapes built once, sequentially, in one copy of the tree.
fn built() -> Option<(MutexGuard<'static, ()>, &'static [Built])> {
    let guard = lock();
    let aeon = aeon()?;
    let all = BUILT.get_or_init(|| {
        let dir = scratch("shared");
        let tree = dir.join("tree");
        copy_tree(aeon, &tree);
        let tree = tree.canonicalize().unwrap();
        SHAPES
            .iter()
            .map(|(name, flags)| {
                let rom = dir.join("out").join(format!("{name}.bin"));
                let (sidecar, _) = build(&tree, flags, &rom, &dir);
                let export: Value = serde_json::from_slice(&sidecar).expect("the sidecar is JSON");
                Built {
                    name,
                    sidecar_path: rom.with_extension("build.json"),
                    export,
                    tree: tree.clone(),
                }
            })
            .collect()
    });
    Some((guard, all.as_slice()))
}

fn hex(bytes: &[u8]) -> String {
    sigil_link::export::sha256_hex(&sigil_link::export::sha256_of(bytes))
}

// ---------------------------------------------------------------- gate 6

#[test]
fn every_export_validates_against_the_vendored_schema() {
    let Some((_g, all)) = built() else {
        return skip("no aeon reference tree");
    };
    assert_eq!(all.len(), SHAPES.len());
    let mut bad = Vec::new();
    for b in all {
        let errs = schema_errors(&b.sidecar_path);
        eprintln!("G6 shape={} schema_errors={}", b.name, errs.len());
        if !errs.is_empty() {
            bad.push(format!("shape {}: {errs:?}", b.name));
        }
    }
    assert!(
        bad.is_empty(),
        "exports that do not validate:\n{}",
        bad.join("\n")
    );
    eprintln!("G6 {}/{} shapes validate", all.len(), SHAPES.len());
}

/// The control for gate 6: one corrupted field in one shape's sidecar IS refused, so a
/// validator that accepts everything cannot stand in for the gate.
#[test]
fn a_corrupted_sidecar_is_refused_by_the_validator() {
    let Some((_g, all)) = built() else {
        return skip("no aeon reference tree");
    };
    let dir = scratch("corrupt");
    for (what, path, value) in [
        ("rom.sha256 short", "/rom/sha256", json!("abc")),
        ("rom.size as a string", "/rom/size", json!("99")),
        ("version.major", "/version/major", json!("1")),
    ] {
        let mut doc = all[0].export.clone();
        *doc.pointer_mut(path).unwrap() = value;
        let f = dir.join("corrupt.build.json");
        std::fs::write(&f, serde_json::to_vec(&doc).unwrap()).unwrap();
        assert!(
            !schema_errors(&f).is_empty(),
            "the corruption `{what}` was accepted by the validator"
        );
    }
}

// ---------------------------------------------------------------- gate 7

#[test]
fn five_builds_one_export_hash_per_shape() {
    let _g = lock();
    let Some(aeon) = aeon() else {
        return skip("no aeon reference tree");
    };
    let base = scratch("det");
    let pid = std::process::id();
    // Five directory shapes per build: different names, different depths, different
    // lengths. A leaked absolute path or directory name differs in every one.
    let dir_for = |shape: &str, k: usize| -> PathBuf {
        let mut p = base.join(format!(
            "r{k}_{}",
            (u64::from(pid) * 7919 + k as u64 * 104_729) % 100_003
        ));
        for d in 0..k {
            p = p.join(format!(
                "{}{d}",
                ["x", "longer_name_", "q", "deep_directory_", "z"][(d + k) % 5]
            ));
        }
        p.join(format!("{shape}_{}", "w".repeat(k * 3)))
    };
    // At most MAX_PARALLEL shapes at once (each holds one 150 MB tree copy and one build),
    // so a small runner is not asked for seven copies and seven assemblers together.
    let par = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, MAX_PARALLEL);
    let mut results: Vec<(String, Vec<Vec<u8>>, Vec<String>)> = Vec::new();
    for chunk in SHAPES.chunks(par) {
        results.extend(std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|(name, flags)| {
                    let dir_for = &dir_for;
                    s.spawn(move || {
                        let mut sidecars = Vec::new();
                        let mut roms = Vec::new();
                        for k in 0..BUILDS {
                            let dir = dir_for(name, k);
                            std::fs::create_dir_all(&dir).unwrap();
                            let tree = dir.join("tree");
                            copy_tree(aeon, &tree);
                            let tree = tree.canonicalize().unwrap();
                            // Same ROM FILE NAME in every build (it is `rom.path` in the
                            // export), a different directory.
                            // A distinct mtime on every file of the copy, so a leaked
                            // timestamp breaks the hash; and a distinct working directory.
                            let stamp = 1_000_000_000 + k as u64 * 3_187_001 + u64::from(pid) % 997;
                            let st = Command::new("find")
                                .arg(&tree)
                                .args(["-exec", "touch", "-h", "-d"])
                                .arg(format!("@{stamp}"))
                                .args(["{}", "+"])
                                .status()
                                .expect("find/touch");
                            assert!(st.success(), "touch of the copy failed");
                            let cwd = dir.join(format!("cwd_{name}_{k}"));
                            std::fs::create_dir_all(&cwd).unwrap();
                            let (sidecar, rom) = build(
                                &tree,
                                flags,
                                &dir.join("out").join(format!("{name}.bin")),
                                &cwd,
                            );
                            let v: Value = serde_json::from_slice(&sidecar).unwrap();
                            assert_eq!(
                                v["rom"]["sha256"],
                                json!(hex(&rom)),
                                "shape {name} build {k}: rom.sha256 is not the ROM file's hash"
                            );
                            roms.push(v["rom"]["sha256"].as_str().unwrap().to_string());
                            sidecars.push(sidecar);
                            std::fs::remove_dir_all(&dir).unwrap();
                        }
                        (name.to_string(), sidecars, roms)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("a shape's builds panicked"))
                .collect::<Vec<_>>()
        }));
    }
    let mut bad = Vec::new();
    for (name, sidecars, roms) in &results {
        let hashes: Vec<String> = sidecars.iter().map(|s| hex(s)).collect();
        eprintln!(
            "G7 shape={name} export_sha256={} rom_sha256={} builds={}",
            hashes[0],
            roms[0],
            hashes.len()
        );
        assert_eq!(sidecars.len(), BUILDS);
        if hashes.iter().any(|h| h != &hashes[0]) {
            bad.push(format!(
                "shape {name}: export hashes differ across builds: {hashes:?}"
            ));
        }
        if roms.iter().any(|r| r != &roms[0]) {
            bad.push(format!(
                "shape {name}: rom.sha256 differs across builds: {roms:?}"
            ));
        }
    }
    assert_eq!(results.len(), SHAPES.len());
    assert!(
        bad.is_empty(),
        "nondeterministic export:\n{}",
        bad.join("\n")
    );
    eprintln!(
        "G7 {}/{} shapes: {BUILDS} builds, one export hash and one rom.sha256 each",
        results.len(),
        SHAPES.len()
    );
}

// ---------------------------------------------------------------- gate 8

fn sym_field<'a>(s: &'a Value, k: &str) -> &'a str {
    s[k].as_str().unwrap_or("")
}

fn is_engine(module: &str) -> bool {
    module == "engine" || module.starts_with("engine.")
}

/// Every string reachable in `v`, object keys and values both.
fn strings_of<'a>(v: &'a Value, out: &mut Vec<&'a str>) {
    match v {
        Value::String(t) => out.push(t),
        Value::Array(a) => a.iter().for_each(|x| strings_of(x, out)),
        Value::Object(o) => {
            for (k, x) in o {
                out.push(k);
                strings_of(x, out);
            }
        }
        _ => {}
    }
}

/// Every record of `symbols`, `layouts` and `consts` whose module is `engine.*`, held to
/// two rules and no looser:
///  * ANY string in it (name, demangled, kind, const type, layout field and overlay types,
///    overlay base) that spells the fully qualified `games.sonic4.` fails. Bare words and
///    generic item names (`Checksum`, `GameHeader`, `SongTable`) are legitimate engine
///    vocabulary and never fail by themselves.
///  * a SYMBOL's `name` or `demangled` that is the name of a sonic4-module symbol
///    (`sonic4_items`) fails: the engine record resolves to a game item.
fn engine_records_naming_sonic4(export: &Value, sonic4_items: &BTreeSet<String>) -> Vec<String> {
    let mut bad = Vec::new();
    for coll in ["symbols", "layouts", "consts"] {
        for r in export[coll].as_array().map_or(&[][..], |a| a.as_slice()) {
            if !is_engine(sym_field(r, "module")) {
                continue;
            }
            let who = format!(
                "{coll} `{}` (module {})",
                sym_field(r, "name"),
                sym_field(r, "module")
            );
            let mut strs = Vec::new();
            strings_of(r, &mut strs);
            for t in strs.into_iter().filter(|t| t.contains("games.sonic4.")) {
                bad.push(format!("{who}: `{t}` spells a games.sonic4 item"));
            }
            if coll == "symbols" {
                for k in ["name", "demangled"] {
                    if sonic4_items.contains(sym_field(r, k)) {
                        bad.push(format!(
                            "{who}: {k} resolves to the sonic4 item `{}`",
                            sym_field(r, k)
                        ));
                    }
                }
            }
        }
    }
    bad
}

/// Symbols with no `module` must be plain AS-side equates: kind `equ`, non-public, no
/// space. Anything else without a module is returned as a problem, so `is_engine` cannot
/// silently skip it. WHOSE equate each one is is [`equate_origins`]' question.
fn unattributed_symbol_problems(export: &Value) -> Vec<String> {
    export["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s.get("module").is_none())
        .filter(|s| s["kind"] != "equ" || s["public"] != false || s.get("space").is_some())
        .map(|s| {
            format!(
                "symbol `{}` has no module but is not a plain AS equate: {s}",
                sym_field(s, "name")
            )
        })
        .collect()
}

/// Where a module-less equate comes from, derived from the shape's own export and the
/// sources its `files` rows name (never a name list).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Origin {
    /// A `-D`-style define of the shape (`shape.defines`).
    Define,
    /// Declared (`const NAME`, `NAME = `, `NAME: equ`) in a file the shape read; true when
    /// EVERY such file is under `games/sonic4/`.
    Declared { sonic4_only: bool },
    /// `<Layout>_<member>`: the struct-offset and size equates a layout generates; true
    /// when the layout is in a `games.sonic4` module.
    Layout { sonic4_only: bool },
    /// Found nowhere.
    Unresolved,
}

fn declared_equate(line: &str) -> Option<&str> {
    let s = line.split("//").next()?.split(';').next()?.trim_start();
    let s = s.strip_prefix("pub ").map_or(s, str::trim_start);
    let (s, is_const) = s
        .strip_prefix("const ")
        .map_or((s, false), |r| (r.trim_start(), true));
    let end = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    if end == 0 || s.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let (name, rest) = s.split_at(end);
    let rest = rest.strip_prefix(':').unwrap_or(rest).trim_start();
    let equ = rest
        .strip_prefix("equ")
        .is_some_and(|r| !r.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_'));
    let assign = rest.starts_with('=') && !rest.starts_with("==");
    (is_const || equ || assign).then_some(name)
}

fn source_path(tree: &Path, f: &Value) -> Option<PathBuf> {
    let (root, path) = (f["root"].as_str()?, f["path"].as_str()?);
    Some(match root {
        "engine" => tree.join("engine").join(path),
        "game" => tree.join("games").join(path),
        "generated" => {
            let (area, rest) = path.split_once('/')?;
            tree.join("engine").join(area).join("generated").join(rest)
        }
        "sigil" => Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
        _ => return None,
    })
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The [`Origin`] of every module-less symbol of `export`, resolved against `tree`.
fn equate_origins(tree: &Path, export: &Value) -> Vec<(String, Origin)> {
    // name -> (declared in a sonic4 file, declared elsewhere)
    let mut declared: std::collections::HashMap<String, (bool, bool)> =
        std::collections::HashMap::new();
    for f in export["files"].as_array().unwrap() {
        let Some(path) = source_path(tree, f) else {
            continue;
        };
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "a `files` row's source {} is unreadable: {e}",
                path.display()
            )
        });
        let sonic4 = f["root"] == "game" && sym_field(f, "path").starts_with("sonic4/");
        for line in text.lines() {
            if let Some(n) = declared_equate(line) {
                let e = declared.entry(n.to_string()).or_default();
                if sonic4 {
                    e.0 = true;
                } else {
                    e.1 = true;
                }
            }
        }
    }
    let mut layouts: std::collections::HashMap<String, (bool, bool)> =
        std::collections::HashMap::new();
    for l in export["layouts"].as_array().unwrap() {
        let e = layouts.entry(norm(sym_field(l, "name"))).or_default();
        if sym_field(l, "module").starts_with("games.sonic4") {
            e.0 = true;
        } else {
            e.1 = true;
        }
    }
    let defines = export["shape"]["defines"].as_object().unwrap();
    export["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s.get("module").is_none())
        .map(|s| {
            let n = sym_field(s, "name");
            let origin = if defines.contains_key(n) {
                Origin::Define
            } else if let Some(&(s4, other)) = declared.get(n) {
                Origin::Declared {
                    sonic4_only: s4 && !other,
                }
            } else if let Some(&(s4, other)) = n
                .match_indices('_')
                .filter_map(|(i, _)| layouts.get(&norm(&n[..i])))
                .next()
            {
                Origin::Layout {
                    sonic4_only: s4 && !other,
                }
            } else {
                Origin::Unresolved
            };
            (n.to_string(), origin)
        })
        .collect()
}

/// A module-less equate that resolves to nothing is a problem in every shape (loud on
/// unmeasurable); one that only a `games/sonic4/` file (or layout) defines is a problem in
/// a DEMO export. Returns the problems and the per-origin counts.
fn equate_problems(tree: &Path, export: &Value, demo: bool) -> (Vec<String>, String) {
    let origins = equate_origins(tree, export);
    let mut bad = Vec::new();
    let (mut define, mut declared, mut layout, mut sonic4_only) = (0, 0, 0, 0);
    for (n, o) in &origins {
        match o {
            Origin::Define => define += 1,
            Origin::Declared { sonic4_only: false } => declared += 1,
            Origin::Layout { sonic4_only: false } => layout += 1,
            Origin::Declared { sonic4_only: true } | Origin::Layout { sonic4_only: true } => {
                sonic4_only += 1;
                if demo {
                    bad.push(format!(
                        "equate `{n}` is defined only under games/sonic4/ and is in a demo export"
                    ));
                }
            }
            Origin::Unresolved => bad.push(format!(
                "equate `{n}`: no defining file, define or layout could be found"
            )),
        }
    }
    let counts = format!(
        "equates={} define={define} declared={declared} layout={layout} sonic4_only={sonic4_only}",
        origins.len()
    );
    (bad, counts)
}

/// The names of every `games.sonic4.*` symbol in `export`.
fn sonic4_item_names(export: &Value) -> BTreeSet<String> {
    export["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| sym_field(s, "module").starts_with("games.sonic4"))
        .map(|s| sym_field(s, "name").to_string())
        .collect()
}

/// The second-game checks over one demo export: it says `demo`, carries no symbol in a
/// `games.sonic4` module, no engine record names a sonic4 item, and no module-less equate
/// is a sonic4 one.
fn demo_problems(tree: &Path, demo: &Value, sonic4_items: &BTreeSet<String>) -> Vec<String> {
    let mut bad = Vec::new();
    if demo["shape"]["game"] != "demo" {
        bad.push(format!("shape.game is {}, not demo", demo["shape"]["game"]));
    }
    for s in demo["symbols"].as_array().unwrap() {
        let m = sym_field(s, "module");
        if m.starts_with("games.sonic4") {
            bad.push(format!(
                "symbol `{}` is in module {m}",
                sym_field(s, "name")
            ));
        }
    }
    bad.extend(engine_records_naming_sonic4(demo, sonic4_items));
    bad.extend(unattributed_symbol_problems(demo));
    bad.extend(equate_problems(tree, demo, true).0);
    bad
}

#[test]
fn demo_export_validates_and_has_no_sonic_symbol_in_engine_records() {
    let Some((_g, all)) = built() else {
        return skip("no aeon reference tree");
    };
    let get = |n: &str| all.iter().find(|b| b.name == n).unwrap();
    let items = sonic4_item_names(&get("plain").export);
    assert!(
        items.len() > 100,
        "only {} games.sonic4 items in the sonic4 export: nothing to hold the demo to",
        items.len()
    );

    for name in ["demo", "demo_debug"] {
        let b = get(name);
        let errs = schema_errors(&b.sidecar_path);
        assert!(errs.is_empty(), "{name} does not validate: {errs:?}");
        let engine_syms = b.export["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| is_engine(sym_field(s, "module")))
            .count();
        assert!(
            engine_syms > 100,
            "{name}: only {engine_syms} engine symbols, nothing to measure"
        );
        let bad = demo_problems(&b.tree, &b.export, &items);
        let (_, counts) = equate_problems(&b.tree, &b.export, true);
        eprintln!("G8 shape={name} engine_symbols={engine_syms} sonic4_items_held_against={} {counts} problems={}", items.len(), bad.len());
        assert!(bad.is_empty(), "{name}: {bad:?}");
    }

    // The same rules over the sonic4 shapes: an `engine.` record never spells a
    // `games.sonic4` item, and every module-less equate has a source (a sonic4-only equate
    // is of course fine in a sonic4 shape).
    for name in ["plain", "debug", "config_a", "config_b", "lean"] {
        let b = get(name);
        let own = sonic4_item_names(&b.export);
        let engine_syms = b.export["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| is_engine(sym_field(s, "module")))
            .count();
        assert!(
            engine_syms > 100 && !own.is_empty(),
            "{name}: nothing to measure"
        );
        let mut bad = engine_records_naming_sonic4(&b.export, &own);
        bad.extend(unattributed_symbol_problems(&b.export));
        let (eq_bad, counts) = equate_problems(&b.tree, &b.export, false);
        bad.extend(eq_bad);
        eprintln!(
            "G8 shape={name} engine_symbols={engine_syms} sonic4_items={} {counts} problems={}",
            own.len(),
            bad.len()
        );
        assert!(bad.is_empty(), "{name}: {bad:?}");
    }
}

/// The controls for gate 8: what must go red under the tightened rule, and what must NOT.
#[test]
fn a_sonic4_symbol_injected_into_a_demo_export_is_refused() {
    let Some((_g, all)) = built() else {
        return skip("no aeon reference tree");
    };
    let plain = all.iter().find(|b| b.name == "plain").unwrap();
    let items = sonic4_item_names(&plain.export);
    let demo_b = all.iter().find(|b| b.name == "demo").unwrap();
    let (demo, tree) = (&demo_b.export, &demo_b.tree);
    assert!(
        demo_problems(tree, demo, &items).is_empty(),
        "the pristine demo export is not clean"
    );

    let row = |name: &str, module: &str| json!({"name": name, "vma": 0, "kind": "label", "module": module, "public": true});
    let refused = |what: &str, mutated: &Value| {
        assert!(
            !demo_problems(tree, mutated, &items).is_empty(),
            "gate 8 accepted {what}"
        )
    };
    let accepted = |what: &str, mutated: &Value| {
        let bad = demo_problems(tree, mutated, &items);
        assert!(bad.is_empty(), "gate 8 refused {what}: {bad:?}");
    };
    let with_symbol = |sym: Value| {
        let mut m = demo.clone();
        m["symbols"].as_array_mut().unwrap().push(sym);
        m
    };
    refused(
        "a games.sonic4 module row",
        &with_symbol(row("Sonic_Init", "games.sonic4.player_common")),
    );
    refused(
        "an engine row spelling games.sonic4",
        &with_symbol(row("games.sonic4.Sonic_Init", "engine.objects")),
    );
    refused(
        "an engine row named like a sonic4 item",
        &with_symbol(row(items.iter().next().unwrap(), "engine.objects")),
    );
    let mut demangled = row("Plain_Engine_Label", "engine.objects");
    demangled["demangled"] = json!(items.iter().next().unwrap());
    refused(
        "an engine row whose demangled name is a sonic4 item",
        &with_symbol(demangled),
    );

    // The non-name strings of an engine record: only the QUALIFIED spelling fails.
    let first_engine = |coll: &str| {
        demo[coll]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| is_engine(sym_field(r, "module")))
            .expect("an engine record")
    };
    let mutate = |coll: &str, f: &dyn Fn(&mut Value)| {
        let mut m = demo.clone();
        f(&mut m[coll][first_engine(coll)]);
        m
    };
    refused(
        "an engine const typed games.sonic4.Foo",
        &mutate("consts", &|r| r["type"] = json!("games.sonic4.Foo")),
    );
    refused(
        "an engine layout field typed games.sonic4.Foo",
        &mutate("layouts", &|r| {
            r["fields"][0]["type"] = json!("games.sonic4.Foo")
        }),
    );
    refused(
        "an engine layout overlay based on games.sonic4.Foo",
        &mutate(
            "layouts",
            &|r| {
                r["overlays"] =
                    json!([{"name": "o", "offset": 0, "base": "games.sonic4.Foo", "fields": []}])
            },
        ),
    );
    // Generic vocabulary a sonic4 item happens to share is NOT a leak.
    assert!(
        items.contains("Checksum"),
        "the sonic4 item set no longer holds the generic `Checksum`: pick another control"
    );
    accepted(
        "an engine const typed `Checksum`",
        &mutate("consts", &|r| r["type"] = json!("Checksum")),
    );
    accepted(
        "an engine layout field typed `Checksum`",
        &mutate("layouts", &|r| {
            r["fields"][0]["type"] = json!("ptr<Checksum>")
        }),
    );
    accepted(
        "an engine const typed with the word sonic4",
        &mutate("consts", &|r| r["type"] = json!("sonic4")),
    );

    // A moduleless symbol that is not a plain equate is refused, not skipped.
    refused(
        "a moduleless label",
        &with_symbol(json!({"name": "Orphan", "vma": 4, "kind": "label", "public": true})),
    );

    // Module-less equates: whose are they? A sonic4-only one in a demo export is refused,
    // as is one with no source at all.
    let sonic4_only: Vec<String> = equate_origins(&plain.tree, &plain.export)
        .into_iter()
        .filter(|(_, o)| matches!(o, Origin::Declared { sonic4_only: true }))
        .map(|(n, _)| n)
        .collect();
    assert!(
        sonic4_only.len() > 50,
        "only {} sonic4-only equates in the sonic4 export",
        sonic4_only.len()
    );
    let equ = |n: &str| json!({"name": n, "vma": 1, "kind": "equ", "public": false});
    refused("a sonic4-only equate", &with_symbol(equ(&sonic4_only[0])));
    refused(
        "an equate with no defining file",
        &with_symbol(equ("NoSuchEquateAnywhere")),
    );
    // (The pristine demo export, with its engine-declared equates, is asserted clean above.)
}

// ---------------------------------------------------------------- tool.revision

/// `tool.revision` is baked by `build.rs`. A test-profile binary can come out of a cargo
/// invocation that did not re-run the build script and so name an older commit than the
/// checkout it was built from. The export must not carry that: it names the checkout's
/// HEAD, or the honest `unknown` (a build outside a git checkout).
#[test]
fn tool_revision_names_the_checkout_head() {
    let Some((_g, all)) = built() else {
        return skip("no aeon reference tree");
    };
    let rev = all[0].export["tool"]["revision"]
        .as_str()
        .expect("tool.revision")
        .to_string();
    for b in all {
        assert_eq!(
            b.export["tool"]["revision"],
            json!(rev),
            "shape {}: tool.revision differs between shapes",
            b.name
        );
    }
    // The honest `unknown` (a build outside a git checkout) is allowed and checks no more.
    if rev != "unknown" {
        assert!(
            rev.len() == 40
                && rev
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "tool.revision `{rev}` is not a full lowercase commit id"
        );
        let root = sigil_harness::source_digest::sigil_source_root();
        let head = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["rev-parse", "HEAD"])
            .output();
        match head {
            Ok(o) if o.status.success() => {
                let head = String::from_utf8_lossy(&o.stdout).trim().to_string();
                assert_eq!(rev, head, "STALE tool.revision: the binary was built from {rev} but the checkout is at {head}; its build script did not re-run");
                eprintln!("T10 tool.revision {rev} == checkout HEAD");
            }
            _ => {
                let why = format!("tool.revision is {rev} but `git rev-parse HEAD` cannot be asked in {}: cannot verify it", root.display());
                // A source tree that moved or has no git is an honest skip; a strict run
                // (the landing bar) must measure.
                assert!(!sigil_harness::test_support::strict_gate(), "{why}");
                skip(&why);
            }
        }
    } else {
        eprintln!("T10 tool.revision is the honest `unknown`");
    }
}
