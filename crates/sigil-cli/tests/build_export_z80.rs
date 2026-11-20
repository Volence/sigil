//! G0a Task 9, gate 3 and the Z80 bridge (spec 4.4, "The Z80").
//!
//! Seam 1 links the resident sound driver on its own; the ROM embeds the blob as one data
//! fragment. The export states the driver anyway: its symbols and line rows in
//! `space: z80`, with `lma` the ROM byte. These gates read the sidecar as a consumer does
//! and hold it to things the exporter does not compute:
//!
//!  * GATE 3, [`snddrv_setbank_resolves_in_z80_space`]: driver names the `.lst` does not
//!    list (`SndDrv_SetBank`, `Psg_NoteOn`, `Seq_Op_Dac`; 0 listing hits, measured in
//!    Task 0) resolve, in `space: z80`, to a procedure whose row is its declaring line, at
//!    a load address inside the blob the listing brackets; and the sequencer handlers'
//!    VMAs equal what seam 1's own handler table derives (`seam1::native_sound_blob`, a
//!    separate size-only placement);
//!  * [`banked_rows_carry_lma`]: every z80 symbol and row in the banked window `$8000-$FFFF`
//!    carries the load address its bank implies, and every listing `PHASE` label phased
//!    into a `z80_bank` window of the shape's `map.toml` is a z80 symbol (queue row
//!    `G0A-SOUND-BANK-LABEL-SPACE`); the sidecar validates against the vendored schema;
//!  * [`z80_rows_locate_to_driver_files`]: the blob's rows are the five resident modules'
//!    (named by seam 1's own table), on real lines, and a row starting at a driver label
//!    sits at or after that label's declaration in the same file (attribution, with a
//!    floor, as gate 2 does for the 68000 side);
//!  * [`sound_off_shapes_have_no_z80_rows`]: a shape whose defines say the driver is not
//!    built has no row and no symbol from it; one whose defines say it is has both.
//!
//! Real `sigil build` runs over a COPY of the reference tree (`AEON_DIR`), the house
//! pattern for gates that build (before G0b Task 6 it rewrote files in its tree). Shapes are built once and shared.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

static LOCK: Mutex<()> = Mutex::new(());
static TREE: OnceLock<Option<PathBuf>> = OnceLock::new();
static BUILT: Mutex<Option<HashMap<&'static str, Built>>> = Mutex::new(None);

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

/// The define that says whether a shape builds the resident sound driver (the shape's
/// own `shape.defines`, never a list of shape names).
const DRIVER_DEFINE: &str = "SOUND_DRIVER_ENABLED";

/// The driver names gate 3 resolves: absent from the `.lst` (Task 0 baseline: 0 hits).
const GATE3_NAMES: [&str; 3] = ["SndDrv_SetBank", "Psg_NoteOn", "Seq_Op_Dac"];

/// The share of driver labels whose row attribution must be CHECKED, in percent (the
/// bound on the `undeclared` exclusion; gate 2 holds the 68000 side to the same 90).
const ATTRIBUTION_FLOOR_PCT: usize = 90;

fn tree() -> Option<(MutexGuard<'static, ()>, PathBuf)> {
    let guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let path = TREE.get_or_init(|| {
        let profile = sigil_harness::native::sonic4_profile(false);
        let aeon = sigil_harness::test_support::reference_tree_for_profile(&profile)?;
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_export_z80_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tree = dir.join("tree");
        sigil_harness::test_support::copy_tree(&aeon, &tree);
        Some(tree.canonicalize().unwrap())
    });
    path.clone().map(|p| (guard, p))
}

fn skip() {
    eprintln!("{}no aeon reference tree", sigil_harness::test_support::SKIP_MARKER);
}

/// One built shape: the ROM file, its listing, its parsed sidecar and the sidecar's path.
#[derive(Clone)]
struct Built {
    listing: String,
    export: serde_json::Value,
    sidecar: PathBuf,
}

/// `name` built once per test binary (the caller holds the tree lock).
fn built(tree: &Path, name: &'static str) -> Built {
    let mut cache = BUILT.lock().unwrap_or_else(|p| p.into_inner());
    let cache = cache.get_or_insert_with(HashMap::new);
    if let Some(b) = cache.get(name) {
        return b.clone();
    }
    let flags = SHAPES.iter().find(|(n, _)| *n == name).unwrap_or_else(|| panic!("no shape {name}")).1;
    let dir = tree.parent().unwrap().join("out");
    std::fs::create_dir_all(&dir).unwrap();
    let rom = dir.join(format!("{name}.bin"));
    let lst = dir.join(format!("{name}.lst"));
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["build", "--aeon"])
        .arg(tree)
        .args(flags)
        .arg("-o")
        .arg(&rom)
        .arg("--emit-lst")
        .arg(&lst)
        .output()
        .expect("spawn sigil");
    assert!(out.status.success(), "shape {name}: build failed:\n{}", String::from_utf8_lossy(&out.stderr));
    let sidecar = rom.with_extension("build.json");
    let json = std::fs::read_to_string(&sidecar).expect("sidecar");
    let b = Built {
        listing: std::fs::read_to_string(&lst).expect("listing"),
        export: serde_json::from_str(&json).expect("valid json"),
        sidecar,
    };
    cache.insert(name, b.clone());
    b
}

/// One row as a consumer reads it.
#[derive(Clone, Debug)]
struct Row {
    section: String,
    space: String,
    vma: u64,
    lma: u64,
    len: u64,
    file: u64,
    line: u64,
    kind: String,
}

fn rows_of(export: &serde_json::Value) -> Vec<Row> {
    let secs = export["lines"]["sections"].as_object().expect("lines.sections");
    let mut rows = Vec::new();
    for (name, s) in secs {
        let space = s["space"].as_str().expect("a section space").to_string();
        for r in s["rows"].as_array().expect("rows") {
            let n = |i: usize| r[i].as_u64().unwrap_or_else(|| panic!("row {r} column {i}"));
            rows.push(Row {
                section: name.clone(),
                space: space.clone(),
                vma: n(0),
                lma: n(1),
                len: n(2),
                file: n(3),
                line: n(4),
                kind: r[5].as_str().expect("kind").to_string(),
            });
        }
    }
    assert!(!rows.is_empty(), "the sidecar has no line rows: nothing to measure");
    rows.sort_by_key(|r| r.lma);
    rows
}

/// The listing's address of label `name` (`  NAME : HEX C |`).
fn listing_label(listing: &str, name: &str) -> Option<u64> {
    let hex = listing.lines().find_map(|l| l.strip_prefix(&format!(" {name} : "))?.strip_suffix(" C |"))?;
    u64::from_str_radix(hex.trim(), 16).ok()
}

/// The ROM range the 68000 side brackets the embedded blob with: `[Z80_Sound_Start,
/// Z80_Sound_End)` from the listing (the boot data's own bracket labels, which also fold
/// `Z80_SOUND_SIZE`). Panics when the listing does not state them: unmeasurable.
fn blob_range(listing: &str) -> (u64, u64) {
    let s = listing_label(listing, "Z80_Sound_Start").expect("the listing states Z80_Sound_Start");
    let e = listing_label(listing, "Z80_Sound_End").expect("the listing states Z80_Sound_End");
    assert!(e > s, "an empty blob bracket [{s:#X}, {e:#X})");
    (s, e)
}

/// The five resident modules as seam 1's own table names them: `(files row path under the
/// engine root, module id)`. Seam 1 is the authority for which files are the driver; this
/// gate copies no list of them.
fn driver_modules(tree: &Path, debug: bool) -> Vec<(String, String)> {
    let mods = sigil_harness::seam1::resident_sound_modules(tree, debug, false);
    assert!(!mods.is_empty(), "seam 1 names no resident module: unmeasurable");
    mods.iter()
        .map(|(rel, f, _)| {
            let path = rel.strip_prefix("engine/").unwrap_or_else(|| panic!("{rel} is not under engine/"));
            (path.to_string(), f.module.path.segments.join("."))
        })
        .collect()
}

fn define(export: &serde_json::Value, name: &str) -> Option<i64> {
    export["shape"]["defines"][name].as_i64()
}

fn file_path(export: &serde_json::Value, i: u64) -> (String, String) {
    let f = &export["files"][i as usize];
    (f["root"].as_str().unwrap_or("?").to_string(), f["path"].as_str().unwrap_or("?").to_string())
}

fn source_lines(tree: &Path, export: &serde_json::Value, i: u64) -> Vec<String> {
    let (root, path) = file_path(export, i);
    assert_eq!(root, "engine", "file {i} is not an engine row");
    std::fs::read_to_string(tree.join("engine").join(&path))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .lines()
        .map(str::to_string)
        .collect()
}

/// The identifier a source line declares, if it has the shape of a label or item
/// declaration (gate 2's reader: optional `pub`, an item keyword, an optional `.`, a name,
/// then end of line or one of `: ( = < {`), plus the Z80 driver's exported local label,
/// `export .fetch:`.
fn declared_name(line: &str) -> Option<&str> {
    let mut s = line.split("//").next()?.trim_start();
    s = s.strip_prefix("pub ").map_or(s, str::trim_start);
    s = s.strip_prefix("export ").map_or(s, str::trim_start);
    for kw in ["proc ", "data ", "table ", "dispatch ", "script ", "offsets ", "region "] {
        if let Some(rest) = s.strip_prefix(kw) {
            s = rest.trim_start();
            break;
        }
    }
    s = s.strip_prefix('.').unwrap_or(s);
    let end = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(s.len());
    if end == 0 || s.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let (name, rest) = s.split_at(end);
    let rest = rest.trim_start();
    (rest.is_empty() || rest.starts_with([':', '(', '=', '<', '{'])).then_some(name)
}

/// GATE 3. Red at Task 0: the export had no driver symbol at all.
#[test]
fn snddrv_setbank_resolves_in_z80_space() {
    let Some((_g, tree)) = tree() else { return skip() };
    for (name, debug) in [("plain", false), ("debug", true)] {
        let b = built(&tree, name);
        let (start, end) = blob_range(&b.listing);
        let rows = rows_of(&b.export);
        let syms = b.export["symbols"].as_array().expect("symbols");
        for want in GATE3_NAMES {
            assert!(listing_label(&b.listing, want).is_none(), "{want} is in the listing now: gate 3 no longer measures the bridge");
            let found: Vec<&serde_json::Value> = syms.iter().filter(|s| s["name"] == want).collect();
            assert_eq!(found.len(), 1, "shape {name}: `{want}` is {} export symbols: {found:?}", found.len());
            let s = found[0];
            assert_eq!(s["space"], "z80", "shape {name}: `{want}` is not in space z80: {s}");
            assert_eq!(s["kind"], "proc", "shape {name}: `{want}` is not a proc: {s}");
            let vma = s["vma"].as_u64().expect("vma");
            let lma = s["lma"].as_u64().unwrap_or_else(|| panic!("shape {name}: `{want}` carries no lma: {s}"));
            assert!(vma <= 0xFFFF, "shape {name}: `{want}` vma {vma:#X} is not a Z80 address");
            assert!((start..end).contains(&lma), "shape {name}: `{want}` lma {lma:#X} is outside the blob [{start:#X}, {end:#X})");
            assert_eq!(lma - vma, start, "shape {name}: `{want}`: lma - vma is not the blob's ROM base");
            // A step to the symbol lands on its declaring line, in a code row.
            let r = rows
                .iter()
                .find(|r| r.lma == lma && (r.kind == "code" || r.kind == "data"))
                .unwrap_or_else(|| panic!("shape {name}: no row starts at `{want}` ({lma:#X})"));
            assert_eq!((r.space.as_str(), r.kind.as_str(), r.vma), ("z80", "code", vma), "shape {name}: the row at `{want}`: {r:?}");
            let text = source_lines(&tree, &b.export, r.file);
            let decl = text.iter().position(|l| declared_name(l) == Some(want)).map(|i| i as u64 + 1);
            let decl = decl.unwrap_or_else(|| panic!("shape {name}: `{want}` is declared nowhere in {:?}", file_path(&b.export, r.file)));
            assert!(r.line >= decl, "shape {name}: the row at `{want}` is line {}, before its declaration at {decl}", r.line);
        }
        // An independent derivation: seam 1's handler table (a size-only placement of its
        // own) gives every sequencer handler's VMA, which the banked seq table bakes.
        let handlers = sigil_harness::seam1::native_sound_blob(&tree, debug).symbols;
        assert!(!handlers.is_empty(), "seam 1 derived no handler: unmeasurable");
        for (h, vma) in &handlers {
            let s = syms
                .iter()
                .find(|s| s["name"] == h.as_str() && s["space"] == "z80")
                .unwrap_or_else(|| panic!("shape {name}: handler `{h}` is not a z80 export symbol"));
            assert_eq!(s["vma"].as_u64(), Some(*vma as u64), "shape {name}: handler `{h}`: export vs seam 1 handler table");
        }
        eprintln!("GATE3 shape={name} blob=[{start:#X},{end:#X}) names={GATE3_NAMES:?} handlers_matched={}", handlers.len());
    }
}

/// The shape's `z80_bank` windows, from its own `map.toml` (`(lma, size, vma)`).
fn z80_windows(tree: &Path) -> Vec<(u64, u64, u64)> {
    let src = std::fs::read_to_string(tree.join("games/sonic4/map.toml")).expect("map.toml");
    let map = sigil_link::load_map(&src).expect("the map loads");
    let w: Vec<(u64, u64, u64)> = map
        .regions
        .iter()
        .filter(|r| r.kind == sigil_ir::map::RegionKind::Z80Bank)
        .map(|r| (r.lma_base as u64, r.size as u64, r.vma_base.expect("a z80_bank window has a vma") as u64))
        .collect();
    assert!(!w.is_empty(), "the sonic4 map declares no z80_bank window: unmeasurable");
    w
}

/// Review Focus 3: banked z80 addresses carry their load address, and the bank it implies.
#[test]
fn banked_rows_carry_lma() {
    let Some((_g, tree)) = tree() else { return skip() };
    let windows = z80_windows(&tree);
    for name in ["plain", "debug"] {
        let b = built(&tree, name);
        let syms = b.export["symbols"].as_array().expect("symbols");
        let rows = rows_of(&b.export);
        let banked = |vma: u64| (0x8000..=0xFFFF).contains(&vma);
        let window_of = |lma: u64| windows.iter().find(|(l, s, _)| (*l..l + s).contains(&lma));
        let mut problems = Vec::new();
        // Symbols: every z80 symbol in the window has an lma in a bank window, at the VMA
        // that window maps it to.
        let mut n_sym = 0;
        for s in syms.iter().filter(|s| s["space"] == "z80") {
            let vma = s["vma"].as_u64().unwrap();
            if !banked(vma) {
                continue;
            }
            n_sym += 1;
            match s["lma"].as_u64() {
                None => problems.push(format!("banked z80 symbol without lma: {s}")),
                Some(lma) => match window_of(lma) {
                    Some((l, _, v)) if v + (lma - l) == vma => {}
                    w => problems.push(format!("banked z80 symbol {s}: lma {lma:#X} (bank {:#X}) is not window {w:?}'s byte for vma {vma:#X}", lma >> 15)),
                },
            }
        }
        // Rows: the same for every z80 row in the window, and one bank per section.
        let mut n_row = 0;
        let mut banks: HashMap<&str, BTreeSet<u64>> = HashMap::new();
        for r in rows.iter().filter(|r| r.space == "z80" && banked(r.vma)) {
            n_row += 1;
            banks.entry(&r.section).or_default().insert(r.lma >> 15);
            match window_of(r.lma) {
                Some((l, _, v)) if v + (r.lma - l) == r.vma => {}
                w => problems.push(format!("banked z80 row {r:?} is not window {w:?}'s byte")),
            }
        }
        for (sec, bs) in &banks {
            if bs.len() != 1 {
                problems.push(format!("section {sec}'s banked rows span banks {bs:?}"));
            }
        }
        // Every listing label phased into a z80_bank window is a z80 symbol at that lma
        // (G0A-SOUND-BANK-LABEL-SPACE), derived from the listing and the map, not a name.
        let mut n_phase = 0;
        for l in b.listing.lines() {
            let Some(rest) = l.strip_prefix("PHASE ") else { continue };
            let label = rest.split(' ').next().unwrap();
            let hex = |key: &str| rest.split(key).nth(1).and_then(|h| u64::from_str_radix(h.split(' ').next()?.trim(), 16).ok());
            let (Some(vma), Some(lma)) = (hex(" VMA $"), hex(" LMA $")) else {
                problems.push(format!("unparsed PHASE row `{l}`"));
                continue;
            };
            let Some((wl, _, wv)) = window_of(lma) else { continue };
            if wv + (lma - wl) != vma {
                continue;
            }
            n_phase += 1;
            let ok = syms.iter().any(|s| s["name"] == label && s["space"] == "z80" && s["vma"] == vma && s["lma"] == lma);
            if !ok {
                problems.push(format!("listing label `{label}` is phased into a z80 bank window (VMA {vma:#X} LMA {lma:#X}) but is no z80 symbol there"));
            }
        }
        eprintln!("BANKED shape={name} banked_z80_symbols={n_sym} banked_z80_rows={n_row} phased_window_labels={n_phase} sections={banks:?} problems={}", problems.len());
        assert!(n_sym > 0 && n_row > 0 && n_phase > 0, "shape {name}: nothing banked to measure ({n_sym} symbols, {n_row} rows, {n_phase} phased labels)");
        assert!(problems.is_empty(), "shape {name}:\n{}", problems.join("\n"));
    }
    // The real s4 sidecar validates, banked z80 symbols included.
    let b = built(&tree, "plain");
    let errs = sigil_harness::test_support::schema_errors(&b.sidecar);
    assert!(errs.is_empty(), "the s4 sidecar does not validate: {errs:?}");
}

/// What [`driver_attribution`] measured.
#[derive(Debug, Default)]
struct Attribution {
    checked: usize,
    undeclared: usize,
    bad: Vec<String>,
}

/// For every driver label (a z80 symbol inside the blob) where a row starts, the row's
/// line is at or after a declaration of that label (its last `.` component) in the row's
/// own file. A label declared nowhere (a generated name) is counted, not checked.
fn driver_attribution(tree: &Path, export: &serde_json::Value, rows: &[Row], blob: (u64, u64)) -> Attribution {
    let mut texts: HashMap<u64, Vec<String>> = HashMap::new();
    let starts: HashMap<u64, &Row> =
        rows.iter().filter(|r| r.space == "z80" && (r.kind == "code" || r.kind == "data")).map(|r| (r.lma, r)).collect();
    let mut out = Attribution::default();
    for s in export["symbols"].as_array().unwrap().iter().filter(|s| s["space"] == "z80") {
        let Some(lma) = s["lma"].as_u64() else { continue };
        if !(blob.0..blob.1).contains(&lma) {
            continue;
        }
        let Some(r) = starts.get(&lma) else { continue };
        let name = s["name"].as_str().unwrap();
        let short = name.rsplit(['.', '$']).next().unwrap_or(name);
        let text = texts.entry(r.file).or_insert_with(|| source_lines(tree, export, r.file));
        let decls: Vec<u64> =
            text.iter().enumerate().filter(|(_, l)| declared_name(l) == Some(short)).map(|(i, _)| i as u64 + 1).collect();
        if decls.is_empty() {
            out.undeclared += 1;
            continue;
        }
        out.checked += 1;
        if !decls.iter().any(|d| *d <= r.line) {
            out.bad.push(format!("`{name}` at {lma:#X}: row {:?}:{} is before every declaration {decls:?}", file_path(export, r.file), r.line));
        }
    }
    out
}

#[test]
fn z80_rows_locate_to_driver_files() {
    let Some((_g, tree)) = tree() else { return skip() };
    for (name, debug) in [("plain", false), ("debug", true)] {
        let b = built(&tree, name);
        let blob = blob_range(&b.listing);
        let rows = rows_of(&b.export);
        let modules = driver_modules(&tree, debug);
        let driver_paths: BTreeSet<&str> = modules.iter().map(|(p, _)| p.as_str()).collect();
        // Every byte of the blob bracket is a z80 row of a driver file, on a real line.
        let inside: Vec<&Row> = rows.iter().filter(|r| r.lma < blob.1 && r.lma + r.len > blob.0).collect();
        let mut at = blob.0;
        let mut used: BTreeSet<String> = BTreeSet::new();
        let mut problems = Vec::new();
        for r in &inside {
            if r.lma != at || r.lma + r.len > blob.1 {
                problems.push(format!("row {r:?} does not tile the blob from {at:#X}"));
            }
            at = r.lma + r.len;
            let (root, path) = file_path(&b.export, r.file);
            if r.space != "z80" || root != "engine" || !driver_paths.contains(path.as_str()) {
                problems.push(format!("blob row {r:?} is {root}/{path} in space {}, not a driver row", r.space));
                continue;
            }
            let n = source_lines(&tree, &b.export, r.file).len() as u64;
            if r.line == 0 || r.line > n {
                problems.push(format!("blob row {r:?}: line {} of a {n}-line file", r.line));
            }
            used.insert(path);
        }
        if at != blob.1 {
            problems.push(format!("the blob's rows end at {at:#X}, the bracket at {:#X}", blob.1));
        }
        let unused: Vec<&&str> = driver_paths.iter().filter(|p| !used.contains(**p)).collect();
        if !unused.is_empty() {
            problems.push(format!("driver files with no row in the blob: {unused:?}"));
        }
        // No driver row outside the blob.
        for r in rows.iter().filter(|r| r.lma >= blob.1 || r.lma + r.len <= blob.0) {
            if r.kind == "code" || r.kind == "data" {
                let (root, path) = file_path(&b.export, r.file);
                if root == "engine" && driver_paths.contains(path.as_str()) {
                    problems.push(format!("a driver row outside the blob: {r:?}"));
                }
            }
        }
        let attr = driver_attribution(&tree, &b.export, &rows, blob);
        let measured = attr.checked + attr.undeclared;
        eprintln!(
            "Z80ROWS shape={name} blob_rows={} files={} attribution_checked={} undeclared={} ratio={:.2}% floor={ATTRIBUTION_FLOOR_PCT}% problems={}",
            inside.len(),
            used.len(),
            attr.checked,
            attr.undeclared,
            100.0 * attr.checked as f64 / measured.max(1) as f64,
            problems.len() + attr.bad.len()
        );
        assert!(attr.checked > 0, "shape {name}: driver attribution checked nothing");
        if attr.checked * 100 < ATTRIBUTION_FLOOR_PCT * measured {
            problems.push(format!("attribution checked {} of {measured}, under the {ATTRIBUTION_FLOOR_PCT}% floor", attr.checked));
        }
        problems.extend(attr.bad.iter().cloned());
        assert!(problems.is_empty(), "shape {name}: {} problems:\n{}", problems.len(), problems[..problems.len().min(10)].join("\n"));
        // Control: the attribution is not blind. Each blob row takes the previous row's line.
        let mut shifted = rows.clone();
        for i in (1..shifted.len()).rev() {
            if shifted[i].space == "z80" && shifted[i - 1].space == "z80" {
                (shifted[i].file, shifted[i].line) = (shifted[i - 1].file, shifted[i - 1].line);
            }
        }
        let red = driver_attribution(&tree, &b.export, &shifted, blob);
        assert!(!red.bad.is_empty(), "shape {name}: driver attribution shifted by one row passed: {red:?}");
    }
}

/// What a shape's export says of the driver: rows on its files, symbols of its modules.
fn driver_presence(export: &serde_json::Value, modules: &[(String, String)]) -> (usize, usize) {
    let paths: BTreeSet<&str> = modules.iter().map(|(p, _)| p.as_str()).collect();
    let ids: BTreeSet<&str> = modules.iter().map(|(_, m)| m.as_str()).collect();
    let rows = rows_of(export)
        .iter()
        .filter(|r| r.kind == "code" || r.kind == "data")
        .filter(|r| {
            let (root, path) = file_path(export, r.file);
            root == "engine" && paths.contains(path.as_str())
        })
        .count();
    let syms = export["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["module"].as_str().is_some_and(|m| ids.contains(m)))
        .count();
    (rows, syms)
}

/// Driver absent (by the shape's own define) means no driver row and no driver symbol;
/// driver present means both. The shape list is the 7 shapes; which half applies comes
/// from each sidecar's `shape.defines`.
#[test]
fn sound_off_shapes_have_no_z80_rows() {
    let Some((_g, tree)) = tree() else { return skip() };
    let modules = driver_modules(&tree, false);
    let mut absent = 0;
    let mut present = 0;
    let mut problems = Vec::new();
    for (name, _) in SHAPES {
        let b = built(&tree, name);
        let on = define(&b.export, DRIVER_DEFINE)
            .unwrap_or_else(|| panic!("shape {name}: the sidecar states no {DRIVER_DEFINE}: unmeasurable"));
        let (rows, syms) = driver_presence(&b.export, &modules);
        eprintln!("SOUNDOFF shape={name} {DRIVER_DEFINE}={on} driver_rows={rows} driver_symbols={syms}");
        match on {
            0 => {
                absent += 1;
                if rows != 0 || syms != 0 {
                    problems.push(format!("shape {name} builds no driver but states {rows} driver rows, {syms} driver symbols"));
                }
            }
            1 => {
                present += 1;
                if rows == 0 || syms == 0 {
                    problems.push(format!("shape {name} builds the driver but states {rows} driver rows, {syms} driver symbols"));
                }
            }
            v => problems.push(format!("shape {name}: {DRIVER_DEFINE}={v} is neither 0 nor 1")),
        }
    }
    assert!(absent > 0 && present > 0, "the 7 shapes do not exercise both halves ({absent} absent, {present} present)");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    // Control: the absent half is not blind. The plain shape's export, read as if its
    // driver were absent, must be flagged.
    let (rows, syms) = driver_presence(&built(&tree, "plain").export, &modules);
    assert!(rows > 0 && syms > 0, "the presence reader finds no driver in s4 plain: blind ({rows} rows, {syms} symbols)");
}
