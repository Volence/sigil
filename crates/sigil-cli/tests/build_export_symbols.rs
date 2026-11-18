//! G0a Task 5: the export's symbols come from the front ends' side tables, and gate 5,
//! the transition gate of spec 4.7: every symbol the `.lst` lists is in the export at the
//! same address.
//!
//! The listing is the ORACLE here and the comparison is one-directional: the export may
//! carry more than the listing (it is allowed to know things the listing omits), never
//! less, and never a different address. The expectation is the real listing of the same
//! build, parsed, never a pinned count; a listing that parses to no symbols, or whose own
//! trailer counts disagree with the rows parsed, fails the gate as unmeasurable.
//!
//! Real `sigil build` runs over a COPY of the reference tree (`AEON_DIR`;
//! `SIGIL_STRICT_GATE` hard-fails a missing tree), because `sigil build` rewrites files in
//! the tree it builds. The tests share the copy and run one at a time (`LOCK`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

static LOCK: Mutex<()> = Mutex::new(());
static TREE: OnceLock<Option<PathBuf>> = OnceLock::new();

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

fn tree() -> Option<(MutexGuard<'static, ()>, PathBuf)> {
    let guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let path = TREE.get_or_init(|| {
        let profile = sigil_harness::native::sonic4_profile(false);
        let aeon = sigil_harness::test_support::reference_tree_for_profile(&profile)?;
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("build_export_symbols_{}", std::process::id()));
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

/// Build one shape into `<scratch>/<name>.bin` with its listing beside it; returns the
/// listing text and the parsed sidecar.
fn build_shape(tree: &Path, name: &str, flags: &[&str]) -> (String, serde_json::Value) {
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
    let text = std::fs::read_to_string(&lst).expect("listing");
    let json = std::fs::read_to_string(rom.with_extension("build.json")).expect("sidecar");
    (text, serde_json::from_str(&json).expect("valid json"))
}

/// What the listing states, parsed from its three row families.
#[derive(Debug, Default)]
struct Listing {
    /// `NAME : HEX C |` symbol-table rows: name -> VMA.
    addrs: Vec<(String, u32)>,
    /// `EQU NAME = $HEX` rows: name -> value.
    equs: Vec<(String, u32)>,
    /// `PHASE NAME VMA $X LMA $Y` rows: name -> LMA.
    phases: HashMap<String, u32>,
}

fn hex(s: &str) -> Result<u32, String> {
    u32::from_str_radix(s.trim_start_matches('$'), 16).map_err(|e| format!("bad hex `{s}`: {e}"))
}

/// Parse a sigil-canonical listing. Loud on unmeasurable: every trailer count the listing
/// states must equal the rows parsed, and a listing with no address row is refused.
fn parse_listing(text: &str) -> Result<Listing, String> {
    let mut l = Listing::default();
    let (mut n_syms, mut n_equs, mut n_phase) = (None, None, None);
    let mut in_table = false;
    for line in text.lines() {
        if line.starts_with("  Symbol Table") {
            in_table = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("EQU ") {
            let (name, value) = rest.split_once(" = ").ok_or_else(|| format!("bad EQU row `{line}`"))?;
            l.equs.push((name.to_string(), hex(value)?));
        } else if let Some(rest) = line.strip_prefix("PHASE-COUNT ") {
            n_phase = Some(rest.trim().parse::<usize>().map_err(|e| format!("{line}: {e}"))?);
        } else if let Some(rest) = line.strip_prefix("PHASE ") {
            let f: Vec<&str> = rest.split(' ').collect();
            if f.len() != 5 || f[1] != "VMA" || f[3] != "LMA" {
                return Err(format!("bad PHASE row `{line}`"));
            }
            l.phases.insert(f[0].to_string(), hex(f[4])?);
        } else if let Some(n) = line.trim().strip_suffix(" unused symbols") {
            let _ = n;
        } else if let Some(n) = line.trim().strip_suffix(" symbols") {
            n_syms = Some(n.trim().parse::<usize>().map_err(|e| format!("{line}: {e}"))?);
        } else if let Some(n) = line.trim().strip_suffix(" equates") {
            n_equs = Some(n.trim().parse::<usize>().map_err(|e| format!("{line}: {e}"))?);
        } else if in_table {
            if let Some(body) = line.strip_suffix(" C |") {
                let body = body.get(1..).ok_or_else(|| format!("bad symbol row `{line}`"))?;
                let (name, value) =
                    body.split_once(" : ").ok_or_else(|| format!("bad symbol row `{line}`"))?;
                l.addrs.push((name.to_string(), hex(value)?));
            }
        }
    }
    let n_syms = n_syms.ok_or("the listing has no `<n> symbols` trailer")?;
    if n_syms != l.addrs.len() {
        return Err(format!("the listing says {n_syms} symbols, {} rows parsed", l.addrs.len()));
    }
    if n_equs.unwrap_or(0) != l.equs.len() {
        return Err(format!("the listing says {n_equs:?} equates, {} rows parsed", l.equs.len()));
    }
    let n_phase = n_phase.ok_or("the listing has no PHASE-COUNT line")?;
    if n_phase != l.phases.len() {
        return Err(format!("PHASE-COUNT {n_phase}, {} rows parsed", l.phases.len()));
    }
    if l.addrs.is_empty() {
        return Err("the listing parsed to zero symbols: nothing to measure".into());
    }
    Ok(l)
}

/// Gate 5 over one (listing, export) pair: the listing symbols missing from the export
/// or present at another address, as printable lines, plus the number of rows checked.
fn gate5(listing: &Listing, export: &serde_json::Value) -> Result<(usize, Vec<String>), String> {
    let syms = export["symbols"].as_array().ok_or("the export has no symbols array")?;
    let mut by_name: HashMap<&str, &serde_json::Value> = HashMap::new();
    for s in syms {
        let name = s["name"].as_str().ok_or("a symbol with no name")?;
        if by_name.insert(name, s).is_some() {
            return Err(format!("the export names `{name}` twice"));
        }
    }
    let mut bad = Vec::new();
    let vma = |s: &serde_json::Value| s["vma"].as_u64().map(|v| v as u32);
    for (name, value) in &listing.addrs {
        match by_name.get(name.as_str()) {
            None => bad.push(format!("missing: {name} @ {value:#X}")),
            Some(s) if vma(s) != Some(*value) => {
                bad.push(format!("moved: {name} listing {value:#X} export {:?}", vma(s)))
            }
            Some(s) if s["kind"] == "equ" => bad.push(format!("kind: {name} is an address, exported as equ")),
            Some(s) => {
                if let Some(lma) = listing.phases.get(name) {
                    if s["lma"].as_u64() != Some(u64::from(*lma)) {
                        bad.push(format!("lma: {name} listing {lma:#X} export {:?}", s["lma"]));
                    }
                }
            }
        }
    }
    for (name, value) in &listing.equs {
        match by_name.get(name.as_str()) {
            None => bad.push(format!("missing equ: {name} = {value:#X}")),
            Some(s) if vma(s) != Some(*value) => {
                bad.push(format!("moved equ: {name} listing {value:#X} export {:?}", vma(s)))
            }
            Some(s) if s["kind"] != "equ" => bad.push(format!("kind: equ {name} exported as {}", s["kind"])),
            Some(_) => {}
        }
    }
    Ok((listing.addrs.len() + listing.equs.len(), bad))
}

#[test]
fn every_listing_symbol_is_in_the_export_at_the_same_address() {
    let Some((_g, tree)) = tree() else { return skip() };
    let mut failures = Vec::new();
    for (name, flags) in SHAPES {
        let (text, export) = build_shape(&tree, name, flags);
        let listing = parse_listing(&text).unwrap_or_else(|e| panic!("shape {name}: unmeasurable listing: {e}"));
        let (checked, bad) = gate5(&listing, &export).unwrap_or_else(|e| panic!("shape {name}: {e}"));
        eprintln!(
            "GATE5 shape={name} listing_symbols={} listing_equates={} phased={} export_symbols={} bad={}",
            listing.addrs.len(),
            listing.equs.len(),
            listing.phases.len(),
            export["symbols"].as_array().map_or(0, |a| a.len()),
            bad.len()
        );
        assert!(checked > 0);
        if !bad.is_empty() {
            failures.push(format!("shape {name}: {} of {checked} rows wrong; first: {:?}", bad.len(), &bad[..bad.len().min(5)]));
        }

        // Control, on the first shape: the comparison is not blind to the very defect it
        // exists to catch. Shift one exported symbol by 2 in memory; the gate must see it.
        if name == "plain" {
            let mut planted = export.clone();
            let (victim, _) = &listing.addrs[listing.addrs.len() / 2];
            let s = planted["symbols"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|s| s["name"] == victim.as_str())
                .expect("the victim is exported");
            s["vma"] = serde_json::json!(s["vma"].as_u64().unwrap() + 2);
            let (_, bad) = gate5(&listing, &planted).unwrap();
            assert!(bad.iter().any(|b| b.contains(victim.as_str())), "a vma shifted by 2 passed gate 5");
        }
    }
    assert!(failures.is_empty(), "gate 5 failed:\n{}", failures.join("\n"));
}

#[test]
fn queue_dma_deferrable_is_a_proc_with_extent() {
    let Some((_g, tree)) = tree() else { return skip() };
    let (_, export) = build_shape(&tree, "plain", &[]);
    let syms = export["symbols"].as_array().unwrap();
    let q = syms.iter().find(|s| s["name"] == "QueueDMA_Deferrable").expect("QueueDMA_Deferrable exported");
    assert_eq!(q["kind"], "proc", "{q}");
    assert_eq!(q["space"], "m68k", "{q}");
    assert_eq!(q["module"], "engine.dma_queue", "{q}");
    assert_eq!(q["public"], true, "{q}");
    let vma = q["vma"].as_u64().unwrap();
    let size = q["size"].as_u64().expect("a proc carries its extent");
    assert!(size > 0, "{q}");
    // The next symbol in the same section that is not one of the proc's own labels (an
    // exported `QueueDMA_Deferrable.x` or a mangled `$mod$QueueDMA_Deferrable$x`).
    let own = |n: &str| n.starts_with("QueueDMA_Deferrable.") || n.contains("$QueueDMA_Deferrable$");
    let next = syms
        .iter()
        .filter(|s| s["space"] == "m68k" && s["kind"] != "equ")
        .filter(|s| s["vma"].as_u64().unwrap() > vma && !own(s["name"].as_str().unwrap()))
        .min_by_key(|s| s["vma"].as_u64().unwrap())
        .expect("a symbol after QueueDMA_Deferrable");
    assert!(vma + size <= next["vma"].as_u64().unwrap(), "extent runs into {next}: {q}");
    // And the extent covers its own last local label, so it is not a label-to-label gap.
    let last_own = syms
        .iter()
        .filter(|s| own(s["name"].as_str().unwrap()))
        .map(|s| s["vma"].as_u64().unwrap())
        .max()
        .expect("QueueDMA_Deferrable has local labels");
    assert!(vma + size > last_own, "the extent stops before its own label at {last_own:#X}: {q}");
}

#[test]
fn equates_have_no_space() {
    let Some((_g, tree)) = tree() else { return skip() };
    let (text, export) = build_shape(&tree, "plain", &[]);
    let listing = parse_listing(&text).unwrap();
    let syms = export["symbols"].as_array().unwrap();
    let equs: Vec<_> = syms.iter().filter(|s| s["kind"] == "equ").collect();
    assert!(!listing.equs.is_empty(), "the listing has no equates: nothing to measure");
    assert!(equs.len() >= listing.equs.len(), "{} exported equates < {} listed", equs.len(), listing.equs.len());
    for s in &equs {
        assert!(s.get("space").is_none(), "an equate carries a space: {s}");
        assert!(s.get("lma").is_none(), "an equate carries an lma: {s}");
    }
    // RAM labels are addresses the front end knows: kind ram, space m68k.
    let ram: Vec<_> = syms.iter().filter(|s| s["kind"] == "ram").collect();
    assert!(!ram.is_empty(), "no RAM symbol was exported");
    for s in &ram {
        assert_eq!(s["space"], "m68k", "{s}");
        assert!(s["vma"].as_u64().unwrap() >= 0xFF_0000, "{s}");
    }
}
