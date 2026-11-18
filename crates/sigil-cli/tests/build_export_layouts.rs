//! G0a Task 8: the export's layouts and constants come from the `.emp` front end.
//!
//! Real `sigil build` runs over a COPY of the reference tree (`AEON_DIR`;
//! `SIGIL_STRICT_GATE` hard-fails a missing tree), because `sigil build` rewrites files
//! in the tree it builds. The tests share the copy and run one at a time (`LOCK`).
//!
//! What is checked is the record, not a table: `Sst`'s expected shape is read from the
//! `.emp` that declares it (`engine/objects/sst.emp`: its `(size: $50)` and its `@`
//! offsets), so a hand table cannot drift in step with a wrong export.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

static LOCK: Mutex<()> = Mutex::new(());
static TREE: OnceLock<Option<PathBuf>> = OnceLock::new();

fn tree() -> Option<(MutexGuard<'static, ()>, PathBuf)> {
    let guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let path = TREE.get_or_init(|| {
        let profile = sigil_harness::native::sonic4_profile(false);
        let aeon = sigil_harness::test_support::reference_tree_for_profile(&profile)?;
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_export_layouts_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tree = dir.join("tree");
        let st = Command::new("cp").arg("-a").arg(&aeon).arg(&tree).status().expect("cp");
        assert!(st.success());
        Some(tree.canonicalize().unwrap())
    });
    path.clone().map(|p| (guard, p))
}

fn skip() {
    eprintln!("{}no aeon reference tree", sigil_harness::test_support::SKIP_MARKER);
}

fn build_plain(tree: &Path) -> serde_json::Value {
    let dir = tree.parent().unwrap().join("out");
    std::fs::create_dir_all(&dir).unwrap();
    let rom = dir.join("plain.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["build", "--aeon"])
        .arg(tree)
        .arg("-o")
        .arg(&rom)
        .output()
        .expect("spawn sigil");
    assert!(out.status.success(), "build failed:\n{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_str(&std::fs::read_to_string(rom.with_extension("build.json")).expect("sidecar")).expect("json")
}

/// The `(size: N)` and every `name: Type @ OFFSET` / `name: Type` field of `pub struct
/// <name>` in `src`, read from the source text. Offsets come from the `@` anchors; a
/// field with none follows the previous field (not needed for `Sst`, whose fields are all
/// anchored except `code_addr` at 0).
fn source_struct(src: &str, name: &str) -> (u64, BTreeMap<String, u64>) {
    let head = format!("pub struct {name} (size: ");
    let at = src.find(&head).unwrap_or_else(|| panic!("no `{head}` in the source"));
    let rest = &src[at + head.len()..];
    let size_txt = rest[..rest.find(')').unwrap()].trim();
    let size = if let Some(h) = size_txt.strip_prefix('$') { u64::from_str_radix(h, 16).unwrap() } else { size_txt.parse().unwrap() };
    let body = &rest[rest.find('{').unwrap() + 1..];
    let body = &body[..body.find("\n}").unwrap()];
    let mut offsets = BTreeMap::new();
    let mut first = true;
    for line in body.lines() {
        let line = line.split("//").next().unwrap().trim();
        let Some((fname, ty)) = line.split_once(':') else { continue };
        if !fname.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || fname.is_empty() {
            continue;
        }
        let off = match ty.split_once('@') {
            Some((_, o)) => {
                let o = o.trim().trim_end_matches(',').trim();
                if let Some(h) = o.strip_prefix('$') { u64::from_str_radix(h, 16).unwrap() } else { o.parse().unwrap() }
            }
            None if first => 0,
            None => continue,
        };
        first = false;
        offsets.insert(fname.to_string(), off);
    }
    (size, offsets)
}

#[test]
fn sst_is_exported_at_size_80() {
    let Some((_g, tree)) = tree() else { return skip() };
    let export = build_plain(&tree);
    let layouts = export["layouts"].as_array().expect("layouts array");
    let sst: Vec<_> = layouts.iter().filter(|l| l["name"] == "Sst").collect();
    assert_eq!(sst.len(), 1, "exactly one `Sst` layout expected, got {}", sst.len());
    let sst = sst[0];
    assert_eq!(sst["kind"], "struct");
    assert_eq!(sst["module"], "engine.objects.sst");
    assert_eq!(sst["size"], 80);

    let fields = sst["fields"].as_array().unwrap();
    let code_addr = fields.iter().find(|f| f["name"] == "code_addr").expect("code_addr field");
    assert_eq!(code_addr["offset"], 0);
    assert_eq!(code_addr["size"], 2);

    // Against the declaring source, not a hand table: every anchored field is at its `@`.
    let src = std::fs::read_to_string(tree.join("engine/objects/sst.emp")).unwrap();
    let (src_size, src_offsets) = source_struct(&src, "Sst");
    assert_eq!(sst["size"].as_u64().unwrap(), src_size);
    assert!(src_offsets.len() > 20, "the source scan found {} fields", src_offsets.len());
    for (name, off) in &src_offsets {
        let f = fields.iter().find(|f| f["name"] == name.as_str()).unwrap_or_else(|| panic!("field {name} not exported"));
        assert_eq!(f["offset"].as_u64().unwrap(), *off, "field {name}");
    }
    assert_eq!(fields.len(), src_offsets.len(), "exported fields vs source fields");

    // The $30 overlay window is present as overlays, all based on Sst at offset $30.
    let overlays = sst["overlays"].as_array().unwrap();
    assert!(!overlays.is_empty(), "no overlays on Sst");
    let at30: Vec<_> = overlays.iter().filter(|o| o["offset"] == 0x30).collect();
    assert!(!at30.is_empty(), "no overlay at $30: {overlays:?}");
    for o in &at30 {
        assert_eq!(o["base"], "Sst", "{o}");
        assert_eq!(o["size"], 32, "{o}");
    }

    // Type spellings: a pointer, a newtype, a fixed-point field are all spelled.
    let ty = |n: &str| fields.iter().find(|f| f["name"] == n).unwrap()["type"].as_str().unwrap().to_string();
    assert_eq!(ty("render_flags"), "u8");
    assert_eq!(ty("parent_ptr"), "u16");
    assert_eq!(ty("code_addr"), "ObjRoutine");
}

#[test]
fn consts_are_integers_only() {
    let Some((_g, tree)) = tree() else { return skip() };
    let export = build_plain(&tree);
    let consts = export["consts"].as_array().expect("consts array");
    assert!(consts.len() > 100, "only {} constants exported", consts.len());
    let mut seen = std::collections::HashSet::new();
    for c in consts {
        let (m, n) = (c["module"].as_str().unwrap(), c["name"].as_str().unwrap());
        assert!(seen.insert((m, n)), "constant {m}::{n} exported twice");
        assert!(c["value"].is_i64(), "constant {m}::{n} is not an integer: {c}");
        assert!(c["type"].as_str().is_some_and(|t| !t.is_empty()), "{c}");
        if c["type"] == "bool" {
            assert!(c["value"] == 0 || c["value"] == 1, "{c}");
        }
    }
    // Nothing whose source is Data, Code or a table is in the export: a `pub const`
    // initialised from `bytes(`/`asm {`/`table`/`import(` is a Data/Code/table value.
    let mut bad = Vec::new();
    for entry in walk(&tree.join("engine")).into_iter().chain(walk(&tree.join("games"))) {
        let text = std::fs::read_to_string(&entry).unwrap_or_default();
        let Some(module) = text.lines().find_map(|l| l.strip_prefix("module ")).map(|m| m.split_whitespace().next().unwrap().to_string()) else { continue };
        for l in text.lines() {
            let Some(rest) = l.strip_prefix("pub const ") else { continue };
            let Some((name, init)) = rest.split_once('=') else { continue };
            let name = name.split(':').next().unwrap().trim();
            let init = init.trim();
            if ["bytes(", "byte(", "asm {", "asm{", "import(", "embed("].iter().any(|p| init.starts_with(p)) && seen.contains(&(module.as_str(), name)) {
                bad.push(format!("{module}::{name} = {init}"));
            }
        }
    }
    assert!(bad.is_empty(), "Data/Code constants were exported: {bad:?}");
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else if p.extension().is_some_and(|x| x == "emp") {
                out.push(p);
            }
        }
    }
    out
}

/// Validate `instance` against the vendored schema with python3 + jsonschema. A missing
/// python3 or jsonschema FAILS the test; it never skips.
fn schema_errors(instance: &Path) -> Vec<String> {
    let schema = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/build/empyrean.build-1.schema.json");
    let script = "import json,sys\nfrom jsonschema import Draft202012Validator as V\n\
        s=json.load(open(sys.argv[1]));i=json.load(open(sys.argv[2]))\n\
        errs=sorted(V(s).iter_errors(i),key=lambda e:list(e.path))\n\
        [print('/'+'/'.join(map(str,e.path)),e.message[:200]) for e in errs[:10]]\n\
        sys.exit(1 if errs else 0)\n";
    let out = Command::new("python3")
        .args(["-I", "-c", script])
        .arg(schema)
        .arg(instance)
        .output()
        .expect("python3 is required for the schema gate and was not found");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("ModuleNotFoundError") && !stderr.contains("ImportError"),
        "python3 `jsonschema` is required: {stderr}"
    );
    match out.status.code() {
        Some(0) => Vec::new(),
        Some(1) => String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect(),
        _ => panic!("schema validator could not run: {stderr}"),
    }
}

#[test]
fn the_real_sidecar_with_layouts_and_consts_validates() {
    let Some((_g, tree)) = tree() else { return skip() };
    let export = build_plain(&tree);
    let sidecar = tree.parent().unwrap().join("out/plain.build.json");
    let errs = schema_errors(&sidecar);
    assert!(errs.is_empty(), "the sidecar does not validate: {errs:?}");
    let layouts = export["layouts"].as_array().unwrap();
    let consts = export["consts"].as_array().unwrap();
    let overlays: usize = layouts.iter().map(|l| l["overlays"].as_array().unwrap().len()).sum();
    let count = |k: &str| layouts.iter().filter(|l| l["kind"] == k).count();
    eprintln!(
        "T8 layouts={} struct={} bitfield={} enum={} overlays={} consts={}",
        layouts.len(),
        count("struct"),
        count("bitfield"),
        count("enum"),
        overlays,
        consts.len()
    );
    assert!(!layouts.is_empty() && overlays > 0);
}
