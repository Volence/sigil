//! G0a Task 6, gate 2: every ROM byte is covered by exactly one line row.
//!
//! The rows are what Oracle steps on and what Aurora's go-to-source jumps through, so the
//! gate reads them as a consumer does, from the sidecar beside the ROM file, and checks
//! them against the ROM FILE and the listing, never against the exporter's own idea of
//! the image:
//!
//!  * every byte of `[0, len(rom file))` is in exactly one row: no gap, no overlap,
//!    nothing past the end;
//!  * the deb2 appendix rows are exactly `[EndOfRom, len)`, where `EndOfRom` is the
//!    listing's address for the label the appendix is appended at (the assembled image's
//!    length), and the ROM holds the deb2 magic there; a shape without the appendix has
//!    `EndOfRom == len` and no deb2 row;
//!  * the checksum row is exactly the field `sigil_link::apply_header_checksum` writes,
//!    and the ROM's word there IS the header checksum of the file;
//!  * a fill or org-gap row covers uniform bytes (padding, not code mislabelled);
//!  * a source row names a `files` row and a line from 1; a no-source row names neither;
//!  * a row's `vma` equals its `lma` unless its section is phased (a listing `PHASE`
//!    label lies in it) or is stated in `space: z80` (its `vma` is a Z80 address, at most
//!    `$FFFF`), and one section has one `vma - lma` offset;
//!  * a `code` row sits in a placed 68000 section (not `(image)`) at an even address
//!    with an even length, as every 68000 instruction does; s4 plain has code rows;
//!  * ATTRIBUTION, against the listing and the source text (see [`attribution`]): the
//!    row that starts at a listing label is on a line at or after a declaration of that
//!    label in the same file. The resident Z80 driver's labels are not in the listing (it
//!    is linked by seam 1, not by the ROM link), so for its rows the label source is the
//!    export's own z80 symbols; the check is the same, against the same source text. Plus
//!    the header's declaring lines;
//!  * the Z80 BRIDGE (spec 4.4): over exactly the generated blob's bytes at the listing's
//!    `Z80_Sound_Start`, the rows are the driver's (`space: z80`, more than one, none on
//!    the embedding `boot_data.emp`), and each row's `vma` is its offset into the blob
//!    (the blob loads at Z80 `$0000`).
//!
//! Loud on unmeasurable: a sidecar with no rows, a listing without `EndOfRom`, or a ROM
//! too short for a header fails the gate rather than passing it.
//!
//! Real `sigil build` runs over a COPY of the reference tree (`AEON_DIR`), the house
//! pattern for a gate that may doctor its tree.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

static LOCK: Mutex<()> = Mutex::new(());
static TREE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// The seven shapes `scripts/step0_shapes_cmp.sh` proves byte-identical, by name and
/// `[game, shape row]` (`sigil build --project <tree>/games/<game> --shape <row>`).
const SHAPES: [(&str, &[&str]); 7] = [
    ("plain", &["sonic4", "release"]),
    ("debug", &["sonic4", "debug"]),
    ("demo", &["demo", "release"]),
    ("demo_debug", &["demo", "debug"]),
    ("config_a", &["sonic4", "config-a"]),
    ("config_b", &["sonic4", "config-b"]),
    ("lean", &["sonic4", "lean"]),
];

/// The deb2 appendix's first two bytes (`native::DEB2_MAGIC`).
const DEB2_MAGIC: [u8; 2] = [0xDE, 0xB2];

/// The line rows the G0a spike measured at label granularity on the s4 plain shape; the
/// fragment walk must state more than that.
const SPIKE_LABEL_ROWS: usize = 2285;

fn tree() -> Option<(MutexGuard<'static, ()>, PathBuf)> {
    let guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let path = TREE.get_or_init(|| {
        let aeon = sigil_harness::test_support::reference_tree_for_shape(&sigil_harness::shapes::Shape::new("sonic4", "release"))?;
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_export_lines_{}", std::process::id()));
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

/// One built shape: the ROM file, its listing and its parsed sidecar.
struct Built {
    rom: Vec<u8>,
    listing: String,
    export: serde_json::Value,
    sidecar_len: u64,
}

fn build_shape(tree: &Path, name: &str, flags: &[&str]) -> Built {
    let dir = tree.parent().unwrap().join("out");
    std::fs::create_dir_all(&dir).unwrap();
    let rom = dir.join(format!("{name}.bin"));
    let lst = dir.join(format!("{name}.lst"));
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["build", "--project"])
        .arg(tree.join("games").join(flags[0]))
        .args(["--shape", flags[1]])
        .env_remove("AEON_DIR")
        .arg("-o")
        .arg(&rom)
        .arg("--emit-lst")
        .arg(&lst)
        .output()
        .expect("spawn sigil");
    assert!(out.status.success(), "shape {name}: build failed:\n{}", String::from_utf8_lossy(&out.stderr));
    let sidecar = rom.with_extension("build.json");
    let json = std::fs::read_to_string(&sidecar).expect("sidecar");
    Built {
        rom: std::fs::read(&rom).expect("rom"),
        listing: std::fs::read_to_string(&lst).expect("listing"),
        export: serde_json::from_str(&json).expect("valid json"),
        sidecar_len: json.len() as u64,
    }
}

/// `EndOfRom`'s address from the listing's symbol table (`  EndOfRom : C3E7C C |`).
fn end_of_rom(listing: &str) -> Result<u64, String> {
    let row = listing
        .lines()
        .find_map(|l| l.strip_prefix(" EndOfRom : ").and_then(|r| r.strip_suffix(" C |")))
        .ok_or("the listing has no EndOfRom symbol row: the appendix start is unmeasurable")?;
    u64::from_str_radix(row.trim(), 16).map_err(|e| format!("EndOfRom `{row}`: {e}"))
}

/// One row as the gate reads it.
#[derive(Clone, Debug)]
struct Row {
    section: String,
    vma: u64,
    lma: u64,
    len: u64,
    file: u64,
    line: u64,
    kind: String,
}

fn rows_of(export: &serde_json::Value) -> Result<Vec<Row>, String> {
    let cols = export["lines"]["columns"].as_array().ok_or("no lines.columns")?;
    let want = ["vma", "lma", "len", "file", "line", "kind"];
    if cols.len() < 6 || cols.iter().zip(want).any(|(c, w)| c != w) {
        return Err(format!("unexpected columns {cols:?}"));
    }
    let secs = export["lines"]["sections"].as_object().ok_or("no lines.sections")?;
    let mut rows = Vec::new();
    for (name, s) in secs {
        for r in s["rows"].as_array().ok_or_else(|| format!("section {name} has no rows array"))? {
            let n = |i: usize| r[i].as_u64().ok_or_else(|| format!("section {name}: row {r} column {i} is not an integer"));
            rows.push(Row {
                section: name.clone(),
                vma: n(0)?,
                lma: n(1)?,
                len: n(2)?,
                file: n(3)?,
                line: n(4)?,
                kind: r[5].as_str().ok_or_else(|| format!("row {r}: kind is not a string"))?.to_string(),
            });
        }
    }
    if rows.is_empty() {
        return Err("the sidecar has no line rows: nothing to measure".into());
    }
    rows.sort_by_key(|r| (r.lma, r.len));
    Ok(rows)
}

/// The listing's `PHASE NAME VMA $X LMA $Y` rows: the load addresses of phased labels.
fn phased_lmas(listing: &str) -> Vec<u64> {
    listing
        .lines()
        .filter_map(|l| l.strip_prefix("PHASE ")?.split(" LMA $").nth(1).map(|h| u64::from_str_radix(h.trim(), 16)))
        .map(|r| r.expect("a PHASE row's LMA is hex"))
        .collect()
}

/// Gate 2 over one shape. Returns the problems found (empty = green).
fn gate2(rom: &[u8], end_of_rom: u64, phased: &[u64], export: &serde_json::Value, rows: &[Row]) -> Vec<String> {
    let mut bad = Vec::new();
    let len = rom.len() as u64;
    let n_files = export["files"].as_array().map_or(0, |f| f.len()) as u64;

    // Exactly once: walk the sorted rows.
    let mut at = 0u64;
    for r in rows {
        if r.len == 0 {
            bad.push(format!("empty row {r:?}"));
        }
        if r.lma > at {
            bad.push(format!("gap: [{at:#X}, {:#X}) has no row", r.lma));
        } else if r.lma < at {
            bad.push(format!("overlap: {r:?} starts before {at:#X}, where the previous row ends"));
        }
        at = at.max(r.lma + r.len);
    }
    if at < len {
        bad.push(format!("gap: [{at:#X}, {len:#X}) at the end of the ROM has no row"));
    }
    if at > len {
        bad.push(format!("rows run to {at:#X}, past the {len:#X}-byte ROM"));
    }

    // Each row's own claims.
    for r in rows {
        let bytes = &rom[(r.lma.min(len)) as usize..((r.lma + r.len).min(len)) as usize];
        match r.kind.as_str() {
            "code" | "data" => {
                if r.file >= n_files || r.line == 0 {
                    bad.push(format!("source row names no file/line: {r:?} ({n_files} files)"));
                }
            }
            "fill" | "org-gap" => {
                if (r.file, r.line) != (0, 0) {
                    bad.push(format!("no-source row carries a file/line: {r:?}"));
                }
                if bytes.iter().any(|b| *b != bytes[0]) {
                    bad.push(format!("{} row over non-uniform bytes: {r:?}", r.kind));
                }
            }
            "deb2" | "checksum" => {
                if (r.file, r.line) != (0, 0) {
                    bad.push(format!("no-source row carries a file/line: {r:?}"));
                }
            }
            other => bad.push(format!("unknown kind `{other}`: {r:?}")),
        }
    }

    // Addresses: one offset per section; vma == lma unless a PHASE label lies in it.
    let mut by_section: std::collections::BTreeMap<&str, Vec<&Row>> = std::collections::BTreeMap::new();
    for r in rows {
        by_section.entry(r.section.as_str()).or_default().push(r);
    }
    let space_of = |sec: &str| -> String { export["lines"]["sections"][sec]["space"].as_str().unwrap_or("?").to_string() };
    for (sec, rs) in &by_section {
        let delta = |r: &Row| r.vma.wrapping_sub(r.lma);
        if rs.iter().any(|r| delta(r) != delta(rs[0])) {
            bad.push(format!("section {sec}: rows disagree on vma - lma"));
        }
        if space_of(sec) == "z80" {
            if let Some(r) = rs.iter().find(|r| r.vma + r.len > 0x1_0000) {
                bad.push(format!("section {sec} is z80 but a row runs past $FFFF: {r:?}"));
            }
            continue;
        }
        let is_phased = rs.iter().any(|r| phased.iter().any(|p| (r.lma..r.lma + r.len).contains(p)));
        if !is_phased && rs.iter().any(|r| r.vma != r.lma) {
            bad.push(format!("section {sec} is not phased but a row has vma != lma: {:?}", rs.iter().find(|r| r.vma != r.lma)));
        }
    }

    // Code: in a placed 68000 section, whole instructions (even address, even length).
    let spaces = export["lines"]["sections"].as_object();
    for r in rows.iter().filter(|r| r.kind == "code") {
        let space = spaces.and_then(|s| s.get(&r.section)).and_then(|s| s["space"].as_str()).unwrap_or("?");
        if r.section == "(image)" || !matches!(space, "m68k" | "z80") {
            bad.push(format!("code row outside a CPU section: {r:?} (space {space})"));
        } else if space == "m68k" && (r.lma % 2 != 0 || r.len % 2 != 0) {
            bad.push(format!("code row is not whole 68000 instructions (odd address or length): {r:?}"));
        }
    }

    // deb2: exactly [EndOfRom, len).
    let deb2: Vec<&Row> = rows.iter().filter(|r| r.kind == "deb2").collect();
    if end_of_rom > len {
        bad.push(format!("EndOfRom {end_of_rom:#X} lies past the {len:#X}-byte ROM"));
    } else if end_of_rom == len {
        if !deb2.is_empty() {
            bad.push(format!("no appendix (EndOfRom == len) but deb2 rows {deb2:?}"));
        }
    } else {
        if rom[end_of_rom as usize..].get(..2) != Some(&DEB2_MAGIC[..]) {
            bad.push(format!("no deb2 magic at EndOfRom {end_of_rom:#X}"));
        }
        let covered: u64 = deb2.iter().map(|r| r.len).sum();
        let first = deb2.first().map(|r| r.lma);
        if first != Some(end_of_rom) || covered != len - end_of_rom {
            bad.push(format!("deb2 rows {deb2:?} are not exactly [{end_of_rom:#X}, {len:#X})"));
        }
    }

    // checksum: exactly the field the fold writes, holding the file's checksum.
    let ck: Vec<&Row> = rows.iter().filter(|r| r.kind == "checksum").collect();
    match sigil_link::header_checksum_field(rom.len()) {
        None => bad.push(format!("a {len}-byte ROM has no header checksum field: unmeasurable")),
        Some(field) => {
            let want = (field.start as u64, field.len() as u64);
            if ck.len() != 1 || (ck[0].lma, ck[0].len) != want {
                bad.push(format!("checksum rows {ck:?}, want one row at {want:?}"));
            }
            let word = u16::from_be_bytes([rom[field.start], rom[field.start + 1]]);
            if word != sigil_link::header_checksum(rom) {
                bad.push(format!("the ROM's word at {:#X} is not its header checksum", field.start));
            }
        }
    }
    bad
}

/// A `files` row of `export` on disk. Each root's path is relative to that root's
/// directory in the build (G0b Task 13a): a `game` row is under the project directory the
/// shape was built from, `games/<shape.game>`.
fn source_path(tree: &Path, export: &serde_json::Value, f: &serde_json::Value) -> Option<PathBuf> {
    let (root, path) = (f["root"].as_str()?, f["path"].as_str()?);
    Some(match root {
        "engine" => tree.join("engine").join(path),
        "game" => tree.join("games").join(export["shape"]["game"].as_str()?).join(path),
        "generated" => {
            let (area, rest) = path.split_once('/')?;
            tree.join("engine").join(area).join("generated").join(rest)
        }
        "sigil" => Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path),
        _ => return None,
    })
}

/// The identifier a source line declares, if it has the shape of a label or item
/// declaration: optional `pub`, optional item keyword, optional `.`, a name, then end of
/// line or one of `: ( = < {`. (`proc X(`, `pub data X: u16 = 0`, `.loop:`, `Label:`.)
fn declared_name(line: &str) -> Option<&str> {
    let mut s = line.split("//").next()?.trim_start();
    s = s.strip_prefix("pub ").map_or(s, str::trim_start);
    // The Z80 driver exports a proc-local label: `export .fetch:`.
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

/// The 1-based lines of `text` inside a `comptime fn` or `context` block: TEMPLATE
/// bodies. A template body owns no bytes of its own: since G0a Task 7 every splice moves
/// its items to an expansion entered at the splice line, so a row ON one of these lines
/// is a splice that was not attributed, and [`attribution`] refuses it. A label written
/// inside a template body is no declaration a row can be checked against either.
fn template_lines(text: &[String]) -> std::collections::HashSet<usize> {
    let mut out = std::collections::HashSet::new();
    let (mut inside, mut started, mut depth) = (false, false, 0i64);
    for (i, raw) in text.iter().enumerate() {
        let code = raw.split("//").next().unwrap_or("");
        let head = code.trim_start();
        let head = head.strip_prefix("pub ").map_or(head, str::trim_start);
        if !inside && (head.starts_with("comptime fn ") || head.starts_with("context ")) {
            (inside, started, depth) = (true, false, 0);
        }
        if inside {
            out.insert(i + 1);
            depth += code.matches('{').count() as i64 - code.matches('}').count() as i64;
            started |= code.contains('{');
            if started && depth <= 0 {
                inside = false;
            }
        }
    }
    out
}

/// What [`attribution`] measured.
#[derive(Debug, Default)]
struct Attribution {
    /// Label addresses whose row was checked.
    checked: usize,
    /// Label addresses whose row is on a template-body line: each is also a problem
    /// (an unattributed splice), so a green gate has 0.
    template: usize,
    /// Label addresses none of whose labels has a declaration outside a template body.
    undeclared: usize,
    bad: Vec<String>,
}

/// ATTRIBUTION against the listing, the derived set and its rule:
///
/// For every address `A` where the listing places a label (`NAME : HEX C |`) and a
/// `code`/`data` row STARTS, the row's line must not precede a declaration of one of
/// the labels at `A` in the row's own file: some label at `A` is declared
/// ([`declared_name`], by its last `$`/`.` component, outside a template body) in the
/// row's file at a line `<=` the row's line. A row that takes another row's line (a
/// wrong-owner walk, a shifted attribution) lands before the label's declaration or in
/// another file. A label is placed at its load address (a listing `PHASE` row's LMA
/// for a phased label). A row on a template-body line is refused outright: since G0a
/// Task 7 spliced bytes carry the splice line, so one there is an unattributed splice
/// (counted in `template` and reported). Excluded, and counted in `undeclared`:
/// addresses where no label has such a declaration at all (comptime-generated names,
/// and labels a template splice defined, `$module$asmN$local`, whose only declaration
/// is inside the template body while their row is, correctly, the splice line). The
/// exclusion is bounded: [`ATTRIBUTION_FLOOR_PCT`] of the label addresses must be
/// checked, per shape.
///
/// The Z80 driver's labels are not in the listing (seam 1 links it, the ROM only embeds
/// the blob), so every export symbol in `space: z80` that carries an `lma` joins the label
/// set at that load address. They come from the export rather than the listing; what the
/// check holds them to, a declaration in the row's own file at or before the row's line,
/// is still the source text.
fn attribution(tree: &Path, listing: &str, export: &serde_json::Value, rows: &[Row]) -> Attribution {
    let files = export["files"].as_array().cloned().unwrap_or_default();
    let texts: Vec<Vec<String>> = files
        .iter()
        .map(|f| {
            source_path(tree, export, f)
                .and_then(|p| std::fs::read_to_string(p).ok())
                .map(|t| t.lines().map(str::to_string).collect())
                .unwrap_or_default()
        })
        .collect();
    let templates: Vec<_> = texts.iter().map(|t| template_lines(t)).collect();
    // name -> every (file, line) declaring it outside a template body.
    let mut decls: std::collections::HashMap<String, Vec<(usize, usize)>> = std::collections::HashMap::new();
    for (f, t) in texts.iter().enumerate() {
        for (i, l) in t.iter().enumerate() {
            if let Some(n) = declared_name(l) {
                if !templates[f].contains(&(i + 1)) {
                    decls.entry(n.to_string()).or_default().push((f, i + 1));
                }
            }
        }
    }
    // Each label at its LOAD address: the `C` row's VMA, or its `PHASE` row's LMA.
    let phase: std::collections::HashMap<&str, u64> = listing
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("PHASE ")?;
            let (name, lma) = (rest.split(' ').next()?, rest.split(" LMA $").nth(1)?);
            Some((name, u64::from_str_radix(lma.trim(), 16).ok()?))
        })
        .collect();
    let mut at: std::collections::BTreeMap<u64, Vec<&str>> = std::collections::BTreeMap::new();
    for l in listing.lines() {
        if let Some((name, hex)) = l.strip_prefix(' ').and_then(|b| b.strip_suffix(" C |")?.split_once(" : ")) {
            if let Ok(a) = u64::from_str_radix(hex.trim(), 16) {
                at.entry(phase.get(name).copied().unwrap_or(a)).or_default().push(name);
            }
        }
    }
    for s in export["symbols"].as_array().into_iter().flatten().filter(|s| s["space"] == "z80") {
        if let (Some(lma), Some(name)) = (s["lma"].as_u64(), s["name"].as_str()) {
            let names = at.entry(lma).or_default();
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    let starts: std::collections::HashMap<u64, &Row> =
        rows.iter().filter(|r| r.kind == "code" || r.kind == "data").map(|r| (r.lma, r)).collect();
    let mut out = Attribution::default();
    for (a, names) in &at {
        let Some(r) = starts.get(a) else { continue };
        let (f, line) = (r.file as usize, r.line as usize);
        if templates.get(f).is_some_and(|t| t.contains(&line)) {
            out.template += 1;
            let path = files.get(f).and_then(|x| x["path"].as_str()).unwrap_or("?");
            out.bad.push(format!(
                "attribution: row at {a:#X} {names:?} is {path}:{line}, a template-body line: a splice that was not attributed to its call"
            ));
            continue;
        }
        // A label a template splice defined (`$module$asmN$local`) is declared in the
        // template body: it has no declaration outside one.
        let spliced = |n: &&&str| n.split('$').any(|part| part.strip_prefix("asm").is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())));
        let cands: Vec<(usize, usize)> = names
            .iter()
            .filter(|n| !spliced(n))
            .filter_map(|n| decls.get(n.rsplit(['$', '.']).next().unwrap_or(n)))
            .flatten()
            .copied()
            .collect();
        if cands.is_empty() {
            out.undeclared += 1;
            continue;
        }
        out.checked += 1;
        if !cands.iter().any(|&(df, dl)| df == f && dl <= line) {
            let path = files.get(f).and_then(|x| x["path"].as_str()).unwrap_or("?");
            out.bad.push(format!("attribution: row at {a:#X} {names:?} is {path}:{line}, before or outside every declaration {cands:?}"));
        }
    }
    out
}

/// The share of label addresses (`checked + undeclared`) whose row attribution must be
/// CHECKED, per shape, in percent: the bound on the `undeclared` exclusion. Measured
/// at G0a Task 7, when template rows stopped being excluded: 97.25% (demo_debug, 1310 of
/// 1347) to 98.31% (demo, 1166 of 1186). Falling under it means the exclusion has grown
/// into a real share of what the check is for.
const ATTRIBUTION_FLOOR_PCT: usize = 90;

/// The row starting at the listing address of `label`.
fn row_at_label<'a>(listing: &str, rows: &'a [Row], label: &str) -> Option<&'a Row> {
    let hex = listing.lines().find_map(|l| l.strip_prefix(&format!(" {label} : "))?.strip_suffix(" C |"))?;
    let a = u64::from_str_radix(hex.trim(), 16).ok()?;
    rows.iter().find(|r| r.lma == a && (r.kind == "code" || r.kind == "data"))
}

/// The source text of a row's line.
fn line_text(tree: &Path, export: &serde_json::Value, r: &Row) -> String {
    let f = &export["files"][r.file as usize];
    let text = source_path(tree, export, f).and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    // Line 0 (a no-source line) is no text: a row wrongly carrying it fails the caller's check.
    (r.line as usize).checked_sub(1).and_then(|i| text.lines().nth(i)).unwrap_or("").to_string()
}

#[test]
fn every_rom_byte_is_covered_once() {
    let Some((_g, tree)) = tree() else { return skip() };
    let mut failures = Vec::new();
    for (name, flags) in SHAPES {
        let b = build_shape(&tree, name, flags);
        let eor = end_of_rom(&b.listing).unwrap_or_else(|e| panic!("shape {name}: {e}"));
        let rows = rows_of(&b.export).unwrap_or_else(|e| panic!("shape {name}: {e}"));
        let phased = phased_lmas(&b.listing);
        let mut bad = gate2(&b.rom, eor, &phased, &b.export, &rows);
        let attr = attribution(&tree, &b.listing, &b.export, &rows);
        assert!(attr.checked > 0, "shape {name}: attribution checked nothing: {attr:?}");
        let measured = attr.checked + attr.undeclared;
        eprintln!(
            "GATE2-ATTRIBUTION shape={name} checked={} undeclared={} ratio={:.2}% floor={ATTRIBUTION_FLOOR_PCT}%",
            attr.checked,
            attr.undeclared,
            100.0 * attr.checked as f64 / measured as f64
        );
        if attr.checked * 100 < ATTRIBUTION_FLOOR_PCT * measured {
            bad.push(format!(
                "attribution checked {} of {measured} label addresses, under the {ATTRIBUTION_FLOOR_PCT}% floor ({} undeclared)",
                attr.checked, attr.undeclared
            ));
        }
        bad.extend(attr.bad.iter().cloned());
        // The header's declaring lines: what a step into the header lands on.
        for label in ["GameHeader", "_header_tail"] {
            match row_at_label(&b.listing, &rows, label) {
                Some(r) if line_text(&tree, &b.export, r).contains(&format!("data {label}")) => {}
                other => bad.push(format!("the row at {label} is not its declaring line: {other:?}")),
            }
        }
        let count = |k: &str| rows.iter().filter(|r| r.kind == k).count();
        eprintln!(
            "GATE2 shape={name} rom={} end_of_rom={eor} rows={} code={} data={} fill={} org-gap={} deb2={} checksum={} sections={} sidecar_bytes={} attribution_checked={} attribution_template={} attribution_undeclared={} bad={} first_bad={:?}",
            b.rom.len(),
            rows.len(),
            count("code"),
            count("data"),
            count("fill"),
            count("org-gap"),
            count("deb2"),
            count("checksum"),
            b.export["lines"]["sections"].as_object().map_or(0, |s| s.len()),
            b.sidecar_len,
            attr.checked,
            attr.template,
            attr.undeclared,
            bad.len(),
            bad.first()
        );
        if !bad.is_empty() {
            failures.push(format!("shape {name}: {} problems; first: {:?}", bad.len(), &bad[..bad.len().min(5)]));
        }

        if name == "plain" {
            // The fragment walk states far more than the spike's label-granularity rows.
            assert!(
                rows.len() > SPIKE_LABEL_ROWS,
                "s4 plain has {} line rows, not more than the spike's {SPIKE_LABEL_ROWS}",
                rows.len()
            );
            // Instruction rows exist: the front ends' marks reached the table.
            assert!(count("code") > 0, "s4 plain has no code rows");
            // The Z80 bridge: over exactly the generated blob's bytes, the driver's own rows
            // replace the embedding line's single data row.
            // The build wrote the blob into its own build directory, not the tree (G0b
            // Task 6), so it is emitted again here, from the same tree, for its length.
            let gen = sigil_build::build_dir::BuildDir::create().expect("a build directory");
            let blob = sigil_harness::native::emit_generated_in(&tree, gen.generated(), None)
                .expect("emit the sound artifacts")
                .for_shape(false)
                .bytes
                .len() as u64;
            let start = b
                .listing
                .lines()
                .find_map(|l| l.strip_prefix(" Z80_Sound_Start : ")?.strip_suffix(" C |"))
                .and_then(|h| u64::from_str_radix(h.trim(), 16).ok())
                .expect("the listing places Z80_Sound_Start");
            let z80 = |r: &Row| b.export["lines"]["sections"][&r.section]["space"] == "z80";
            let inside: Vec<&Row> = rows.iter().filter(|r| r.lma < start + blob && r.lma + r.len > start).collect();
            let tiled = inside.first().map(|r| r.lma) == Some(start)
                && inside.windows(2).all(|w| w[0].lma + w[0].len == w[1].lma)
                && inside.last().map(|r| r.lma + r.len) == Some(start + blob);
            let on_embed = inside
                .iter()
                .filter(|r| b.export["files"][r.file as usize]["path"].as_str() == Some("system/boot_data.emp"))
                .count();
            let offsets = inside.iter().all(|r| r.vma == r.lma - start);
            assert!(
                inside.len() > 1 && tiled && inside.iter().all(|r| z80(r)) && on_embed == 0 && offsets,
                "the Z80 blob [{start:#X}, +{blob}) is not the bridge's driver rows: {} rows, tiled={tiled}, \
                 all z80={}, {on_embed} on boot_data.emp, vma = blob offset: {offsets}; first {:?}",
                inside.len(),
                inside.iter().all(|r| z80(r)),
                inside.first()
            );
            // Controls: the gate is not blind to the defects it exists to catch.
            // (1) Drop one row.
            let mut dropped = rows.clone();
            let gone = dropped.remove(dropped.len() / 2);
            let red = gate2(&b.rom, eor, &phased, &b.export, &dropped);
            assert!(red.iter().any(|m| m.contains("gap")), "a dropped row {gone:?} passed gate 2: {red:?}");
            // (2) A source row duplicated over the header back-patch: the checksum bytes
            // claimed again by a data row (the source line that declared them).
            let ck = rows.iter().find(|r| r.kind == "checksum").expect("a checksum row").clone();
            let mut dup = rows.clone();
            dup.push(Row { section: ck.section.clone(), kind: "data".into(), file: 0, line: 1, ..ck });
            dup.sort_by_key(|r| (r.lma, r.len));
            let red = gate2(&b.rom, eor, &phased, &b.export, &dup);
            assert!(red.iter().any(|m| m.contains("overlap")), "a duplicated header row passed gate 2: {red:?}");
            // (3) The deb2 row off by one.
            let mut off = rows.clone();
            let d = off.iter_mut().find(|r| r.kind == "deb2").expect("a deb2 row");
            d.lma += 1;
            d.len -= 1;
            let red = gate2(&b.rom, eor, &phased, &b.export, &off);
            assert!(red.iter().any(|m| m.starts_with("deb2 rows")), "a deb2 row off by one passed gate 2: {red:?}");
            // (4) A data row over non-uniform bytes relabelled fill.
            let mut relabel = rows.clone();
            let v = relabel
                .iter_mut()
                .find(|r| r.kind == "data" && { let s = &b.rom[r.lma as usize..(r.lma + r.len) as usize]; s.iter().any(|x| *x != s[0]) })
                .expect("a data row over non-uniform bytes");
            (v.kind, v.file, v.line) = ("fill".into(), 0, 0);
            let red = gate2(&b.rom, eor, &phased, &b.export, &relabel);
            assert!(red.iter().any(|m| m.contains("non-uniform")), "a data row relabelled fill passed gate 2: {red:?}");
            // (5) A row's vma moved off its lma in an unphased section.
            let mut moved = rows.clone();
            let v = moved.iter_mut().find(|r| r.kind == "code").unwrap();
            v.vma += 2;
            let red = gate2(&b.rom, eor, &phased, &b.export, &moved);
            assert!(red.iter().any(|m| m.contains("vma")), "a vma moved off its lma passed gate 2: {red:?}");
            // (7) A row outside every placed section relabelled code (the appendix).
            let mut stray = rows.clone();
            let v = stray.iter_mut().find(|r| r.kind == "deb2").unwrap();
            (v.kind, v.file, v.line) = ("code".into(), 0, 1);
            let red = gate2(&b.rom, eor, &phased, &b.export, &stray);
            assert!(red.iter().any(|m| m.contains("outside a CPU section")), "code in (image) passed gate 2: {red:?}");
            // (6) Attribution shifted by one row: each row takes the previous row's line.
            let mut shifted = rows.clone();
            for i in (1..shifted.len()).rev() {
                (shifted[i].file, shifted[i].line) = (shifted[i - 1].file, shifted[i - 1].line);
            }
            let red = attribution(&tree, &b.listing, &b.export, &shifted);
            assert!(!red.bad.is_empty(), "attribution shifted by one row passed: {red:?}");
        }
    }
    assert!(failures.is_empty(), "gate 2 failed:\n{}", failures.join("\n"));
}

/// `SHAPES` is every shipped shape and nothing else (`shapes::shipped`), so a new
/// shipped row cannot go unwatched by this file.
#[test]
fn the_shapes_table_is_every_shipped_shape() {
    sigil_harness::test_support::assert_lists_every_shipped_shape(
        SHAPES.iter().map(|(_, gs)| (gs[0], gs[1])),
        "this file's SHAPES table",
    );
}
