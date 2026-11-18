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
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_export_gates_{what}_{}", std::process::id()));
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
    assert!(out.status.success(), "build {flags:?} failed:\n{}", String::from_utf8_lossy(&out.stderr));
    let sidecar = std::fs::read(rom.with_extension("build.json")).expect("the build wrote no sidecar");
    (sidecar, std::fs::read(rom).expect("rom"))
}

/// One built shape, kept in the shared tree for gates 6 and 8.
struct Built {
    name: &'static str,
    sidecar_path: PathBuf,
    export: Value,
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
                Built { name, sidecar_path: rom.with_extension("build.json"), export }
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
    let Some((_g, all)) = built() else { return skip("no aeon reference tree") };
    assert_eq!(all.len(), SHAPES.len());
    let mut bad = Vec::new();
    for b in all {
        let errs = schema_errors(&b.sidecar_path);
        eprintln!("G6 shape={} schema_errors={}", b.name, errs.len());
        if !errs.is_empty() {
            bad.push(format!("shape {}: {errs:?}", b.name));
        }
    }
    assert!(bad.is_empty(), "exports that do not validate:\n{}", bad.join("\n"));
    eprintln!("G6 {}/{} shapes validate", all.len(), SHAPES.len());
}

/// The control for gate 6: one corrupted field in one shape's sidecar IS refused, so a
/// validator that accepts everything cannot stand in for the gate.
#[test]
fn a_corrupted_sidecar_is_refused_by_the_validator() {
    let Some((_g, all)) = built() else { return skip("no aeon reference tree") };
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
        assert!(!schema_errors(&f).is_empty(), "the corruption `{what}` was accepted by the validator");
    }
}

// ---------------------------------------------------------------- gate 7

#[test]
fn five_builds_one_export_hash_per_shape() {
    let _g = lock();
    let Some(aeon) = aeon() else { return skip("no aeon reference tree") };
    let base = scratch("det");
    let pid = std::process::id();
    // Five directory shapes per build: different names, different depths, different
    // lengths. A leaked absolute path or directory name differs in every one.
    let dir_for = |shape: &str, k: usize| -> PathBuf {
        let mut p = base.join(format!("r{k}_{}", (u64::from(pid) * 7919 + k as u64 * 104_729) % 100_003));
        for d in 0..k {
            p = p.join(format!("{}{d}", ["x", "longer_name_", "q", "deep_directory_", "z"][(d + k) % 5]));
        }
        p.join(format!("{shape}_{}", "w".repeat(k * 3)))
    };
    // At most MAX_PARALLEL shapes at once (each holds one 150 MB tree copy and one build),
    // so a small runner is not asked for seven copies and seven assemblers together.
    let par = std::thread::available_parallelism().map_or(1, |n| n.get()).clamp(1, MAX_PARALLEL);
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
                        let (sidecar, rom) = build(&tree, flags, &dir.join("out").join(format!("{name}.bin")), &cwd);
                        let v: Value = serde_json::from_slice(&sidecar).unwrap();
                        assert_eq!(v["rom"]["sha256"], json!(hex(&rom)), "shape {name} build {k}: rom.sha256 is not the ROM file's hash");
                        roms.push(v["rom"]["sha256"].as_str().unwrap().to_string());
                        sidecars.push(sidecar);
                        std::fs::remove_dir_all(&dir).unwrap();
                    }
                    (name.to_string(), sidecars, roms)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("a shape's builds panicked")).collect::<Vec<_>>()
        }));
    }
    let mut bad = Vec::new();
    for (name, sidecars, roms) in &results {
        let hashes: Vec<String> = sidecars.iter().map(|s| hex(s)).collect();
        eprintln!("G7 shape={name} export_sha256={} rom_sha256={} builds={}", hashes[0], roms[0], hashes.len());
        assert_eq!(sidecars.len(), BUILDS);
        if hashes.iter().any(|h| h != &hashes[0]) {
            bad.push(format!("shape {name}: export hashes differ across builds: {hashes:?}"));
        }
        if roms.iter().any(|r| r != &roms[0]) {
            bad.push(format!("shape {name}: rom.sha256 differs across builds: {roms:?}"));
        }
    }
    assert_eq!(results.len(), SHAPES.len());
    assert!(bad.is_empty(), "nondeterministic export:\n{}", bad.join("\n"));
    eprintln!("G7 {}/{} shapes: {BUILDS} builds, one export hash and one rom.sha256 each", results.len(), SHAPES.len());
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

/// Why `text` names a `games.sonic4` item, if it does: it spells `games.sonic4` or the
/// word `sonic4`, or one of its identifier tokens is the name of a sonic4 item.
fn names_sonic4(text: &str, sonic4_items: &BTreeSet<String>) -> Option<String> {
    if text.contains("games.sonic4") || text.to_ascii_lowercase().contains("sonic4") {
        return Some("spells sonic4".into());
    }
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .find(|t| sonic4_items.contains(*t))
        .map(|t| format!("names the sonic4 item `{t}`"))
}

/// Every record of `symbols`, `layouts` and `consts` whose module is `engine.*` and in
/// which ANY string (name, demangled, kind, const type, layout field type, overlay base,
/// overlay fields and their types) names a `games.sonic4` item.
fn engine_records_naming_sonic4(export: &Value, sonic4_items: &BTreeSet<String>) -> Vec<String> {
    let mut bad = Vec::new();
    for coll in ["symbols", "layouts", "consts"] {
        for r in export[coll].as_array().map_or(&[][..], |a| a.as_slice()) {
            if !is_engine(sym_field(r, "module")) {
                continue;
            }
            let mut strs = Vec::new();
            strings_of(r, &mut strs);
            for t in strs {
                if let Some(why) = names_sonic4(t, sonic4_items) {
                    bad.push(format!("{coll} `{}` (module {}): `{t}` {why}", sym_field(r, "name"), sym_field(r, "module")));
                }
            }
        }
    }
    bad
}

/// Symbols with no `module`. These are legitimate only as AS-side equates: the `.emp`
/// front end attributes every label/data/ram/proc symbol (and every z80 symbol) to its
/// module, while an equate a `.asm` file defines has no `.emp` module to name. Measured:
/// 543 on demo, 675 on sonic4 plain, all `equ`, non-public, no space. Anything else
/// without a module is returned as a problem, so `is_engine` cannot silently skip it.
fn unattributed_symbol_problems(export: &Value) -> Vec<String> {
    export["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s.get("module").is_none())
        .filter(|s| s["kind"] != "equ" || s["public"] != false || s.get("space").is_some())
        .map(|s| format!("symbol `{}` has no module but is not a plain AS equate: {s}", sym_field(s, "name")))
        .collect()
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
/// `games.sonic4` module, and no engine record names a sonic4 item.
fn demo_problems(demo: &Value, sonic4_items: &BTreeSet<String>) -> Vec<String> {
    let mut bad = Vec::new();
    if demo["shape"]["game"] != "demo" {
        bad.push(format!("shape.game is {}, not demo", demo["shape"]["game"]));
    }
    for s in demo["symbols"].as_array().unwrap() {
        let m = sym_field(s, "module");
        if m.starts_with("games.sonic4") {
            bad.push(format!("symbol `{}` is in module {m}", sym_field(s, "name")));
        }
    }
    bad.extend(engine_records_naming_sonic4(demo, sonic4_items));
    bad.extend(unattributed_symbol_problems(demo));
    // The demo has no sonic4 half, so an unattributed equate must not name one either.
    for s in demo["symbols"].as_array().unwrap().iter().filter(|s| s.get("module").is_none()) {
        if let Some(why) = names_sonic4(sym_field(s, "name"), sonic4_items) {
            bad.push(format!("unattributed equate `{}` {why}", sym_field(s, "name")));
        }
    }
    bad
}

#[test]
fn demo_export_validates_and_has_no_sonic_symbol_in_engine_records() {
    let Some((_g, all)) = built() else { return skip("no aeon reference tree") };
    let get = |n: &str| all.iter().find(|b| b.name == n).unwrap();
    let items = sonic4_item_names(&get("plain").export);
    assert!(items.len() > 100, "only {} games.sonic4 items in the sonic4 export: nothing to hold the demo to", items.len());

    for name in ["demo", "demo_debug"] {
        let b = get(name);
        let errs = schema_errors(&b.sidecar_path);
        assert!(errs.is_empty(), "{name} does not validate: {errs:?}");
        let engine_syms = b.export["symbols"].as_array().unwrap().iter().filter(|s| is_engine(sym_field(s, "module"))).count();
        assert!(engine_syms > 100, "{name}: only {engine_syms} engine symbols, nothing to measure");
        let bad = demo_problems(&b.export, &items);
        eprintln!("G8 shape={name} engine_symbols={engine_syms} sonic4_items_held_against={} problems={}", items.len(), bad.len());
        assert!(bad.is_empty(), "{name}: {bad:?}");
    }

    // The same rule over the sonic4 shapes: an `engine.` record never names a
    // `games.sonic4` item. (Within one export a name is unique, so the by-name arm cannot
    // fire there; the spelling arm is the live one.)
    for name in ["plain", "debug", "config_a", "config_b", "lean"] {
        let b = get(name);
        let own = sonic4_item_names(&b.export);
        let engine_syms = b.export["symbols"].as_array().unwrap().iter().filter(|s| is_engine(sym_field(s, "module"))).count();
        assert!(engine_syms > 100 && !own.is_empty(), "{name}: nothing to measure");
        let mut bad = engine_records_naming_sonic4(&b.export, &own);
        bad.extend(unattributed_symbol_problems(&b.export));
        eprintln!("G8 shape={name} engine_symbols={engine_syms} sonic4_items={} problems={}", own.len(), bad.len());
        assert!(bad.is_empty(), "{name}: {bad:?}");
    }
}

/// The control for gate 8: a `games.sonic4.Sonic_Init` row injected into a demo export,
/// and an engine-module row that spells it, are both refused.
#[test]
fn a_sonic4_symbol_injected_into_a_demo_export_is_refused() {
    let Some((_g, all)) = built() else { return skip("no aeon reference tree") };
    let items = sonic4_item_names(&all.iter().find(|b| b.name == "plain").unwrap().export);
    let demo = &all.iter().find(|b| b.name == "demo").unwrap().export;
    assert!(demo_problems(demo, &items).is_empty(), "the pristine demo export is not clean");

    let row = |name: &str, module: &str| json!({"name": name, "vma": 0, "kind": "label", "module": module, "public": true});
    for (what, injected) in [
        ("a games.sonic4 module row", row("Sonic_Init", "games.sonic4.player_common")),
        ("an engine row spelling games.sonic4", row("games.sonic4.Sonic_Init", "engine.objects")),
        ("an engine row named like a sonic4 item", row(items.iter().next().unwrap(), "engine.objects")),
    ] {
        let mut mutated = demo.clone();
        mutated["symbols"].as_array_mut().unwrap().push(injected);
        assert!(!demo_problems(&mutated, &items).is_empty(), "gate 8 accepted {what}");
    }

    // The non-name fields of an engine record: a const's type, a layout field's type, an
    // overlay's base and field types, a symbol's kind.
    let some_item = items.iter().next().unwrap().clone();
    let first_engine = |coll: &str| {
        demo[coll].as_array().unwrap().iter().position(|r| is_engine(sym_field(r, "module"))).expect("an engine record")
    };
    type Mutation = Box<dyn Fn(&mut Value)>;
    let cases: [(&str, &str, Mutation); 5] = [
        ("an engine const typed games.sonic4.Foo", "consts", Box::new(|r| r["type"] = json!("games.sonic4.Foo"))),
        ("an engine const typed with a sonic4 item", "consts", { let it = some_item.clone(); Box::new(move |r| r["type"] = json!(format!("ptr<{it}>"))) }),
        ("an engine layout field typed games.sonic4.Foo", "layouts", Box::new(|r| r["fields"][0]["type"] = json!("games.sonic4.Foo"))),
        ("an engine layout field typed with a sonic4 item", "layouts", { let it = some_item.clone(); Box::new(move |r| r["fields"][0]["type"] = json!(it.clone())) }),
        ("an engine layout overlay based on a sonic4 type", "layouts", Box::new(|r| {
            r["overlays"] = json!([{"name": "o", "offset": 0, "base": "games.sonic4.Foo", "fields": []}])
        })),
    ];
    for (what, coll, mutate) in cases {
        let mut mutated = demo.clone();
        let at = first_engine(coll);
        mutate(&mut mutated[coll][at]);
        assert!(!demo_problems(&mutated, &items).is_empty(), "gate 8 accepted {what}");
    }
    // A moduleless symbol that is not a plain equate is refused, not skipped.
    let mut mutated = demo.clone();
    mutated["symbols"].as_array_mut().unwrap().push(json!({"name": "Orphan", "vma": 4, "kind": "label", "public": true}));
    assert!(!demo_problems(&mutated, &items).is_empty(), "gate 8 accepted a moduleless label");
}

// ---------------------------------------------------------------- tool.revision

/// `tool.revision` is baked by `build.rs`. A test-profile binary can come out of a cargo
/// invocation that did not re-run the build script and so name an older commit than the
/// checkout it was built from. The export must not carry that: it names the checkout's
/// HEAD, or the honest `unknown` (a build outside a git checkout).
#[test]
fn tool_revision_names_the_checkout_head() {
    let Some((_g, all)) = built() else { return skip("no aeon reference tree") };
    let rev = all[0].export["tool"]["revision"].as_str().expect("tool.revision").to_string();
    for b in all {
        assert_eq!(b.export["tool"]["revision"], json!(rev), "shape {}: tool.revision differs between shapes", b.name);
    }
    // The honest `unknown` (a build outside a git checkout) is allowed and checks no more.
    if rev != "unknown" {
        assert!(
            rev.len() == 40 && rev.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "tool.revision `{rev}` is not a full lowercase commit id"
        );
        let root = sigil_harness::source_digest::sigil_source_root();
        let head = Command::new("git").arg("-C").arg(&root).args(["rev-parse", "HEAD"]).output();
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
