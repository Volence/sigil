//! The G0b comparison job in the landing bar (Task 7): every shape the reference tree's game
//! descriptions declare is built by BOTH of Sigil's build drivers, the profile-driven one
//! (`sigil build --legacy --aeon`) and the game-as-data one (`sigil build --project`), and the
//! two must write the same ROM, the same `.lst` and the same G0a build export (`tool` and
//! `inputsSha256` excluded). The job itself is `scripts/compare-drivers.sh`; this file runs it
//! once against the reference tree (`AEON_DIR`) with this test tree's own `sigil` binary and
//! holds its rows.
//!
//! Expectations come from source, not from the script: the set of `(game, shape)` rows the job
//! must print is every `[shapes.*]` row of every `games/*/game.toml` in the tree. A shape a
//! description adds without a row in the script's pairing table is a failure here, named. A row
//! the job could not build is `DID NOT RUN`, which fails, as does a job that never printed its
//! end marker.
//!
//! The listing and the export are compared through the DECLARED DELTA (proxy ruling on QUEUE
//! G0B-COMPARE-IDENTITY-NAMES, option a; see the script's header): the legacy artifacts are
//! transformed into what the game-as-data driver must write (the `[shapes.*]` row name, the two
//! description read rows, the recomputed aggregate) and compared byte for byte. Those columns
//! pass only as `MATCH(declared-delta)`; the ROM is compared exactly.
//!
//! The three artifacts are held by separate tests so a red names the artifact that moved:
//! `rom_bytes_match_on_every_shape`, `listings_match_on_every_shape`,
//! `exports_match_on_every_shape`.
//!
//! Runner: the landing bar (`scripts/landing-run.sh`, which sets `AEON_DIR` and
//! `SIGIL_STRICT_GATE`); the nightly source lane lists it in `SOURCE_GATES`. Without `AEON_DIR`
//! it skips loudly, and under `SIGIL_STRICT_GATE=1` a missing reference is a failure.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// One `SHAPE` row of the job's output.
#[derive(Debug)]
struct Row {
    name: String,
    game: String,
    shape: String,
    rom: String,
    lst: String,
    export: String,
    verdict: String,
}

/// The job's run: its output, its rows, and the expected `(game, shape)` set.
struct Run {
    stdout: String,
    end_rc: Option<i32>,
    rows: Vec<Row>,
    expected: BTreeSet<(String, String)>,
    summary: Option<(usize, usize)>,
}

fn field<'a>(tokens: &'a [&'a str], key: &str) -> Option<&'a str> {
    tokens.iter().find_map(|t| t.strip_prefix(key).and_then(|v| v.strip_prefix('=')))
}

fn parse_row(line: &str) -> Row {
    let did_not_run = line.ends_with(" DID NOT RUN");
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let get = |k: &str| field(&tokens, k).unwrap_or("").to_string();
    Row {
        name: tokens.get(1).copied().unwrap_or("").to_string(),
        game: get("game"),
        shape: get("shape"),
        rom: get("rom"),
        lst: get("lst"),
        export: get("export"),
        verdict: if did_not_run {
            "DID NOT RUN".into()
        } else if line.ends_with(" MATCH (declared delta)") {
            "MATCH (declared delta)".into()
        } else {
            tokens.last().copied().unwrap_or("").to_string()
        },
    }
}

/// Every `[shapes.*]` row of every game description under `<aeon>/games`.
fn described_shapes(aeon: &Path) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    let games = std::fs::read_dir(aeon.join("games")).unwrap_or_else(|e| panic!("read {}/games: {e}", aeon.display()));
    for entry in games {
        let path = entry.unwrap().path().join("game.toml");
        if !path.is_file() {
            continue;
        }
        let game = sigil_build::describe::load_game(&path).unwrap_or_else(|e| panic!("{e}"));
        for shape in game.shapes.keys() {
            out.insert((game.id.clone(), shape.clone()));
        }
    }
    out
}

fn run() -> Option<&'static Run> {
    static RUN: OnceLock<Option<Run>> = OnceLock::new();
    RUN.get_or_init(|| {
        let aeon: PathBuf = sigil_harness::test_support::reference_tree(&[
            "build.sh",
            "engine/engine.toml",
            "games/sonic4/game.toml",
            "games/demo/game.toml",
        ])?;
        let expected = described_shapes(&aeon);
        assert!(!expected.is_empty(), "UNMEASURABLE: {} describes no shape", aeon.display());
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/compare-drivers.sh");
        let out = Command::new("bash")
            .arg(&script)
            .arg("--sigil")
            .arg(env!("CARGO_BIN_EXE_sigil"))
            .arg("--aeon")
            .arg(&aeon)
            .env_remove("AEON_DIR")
            .output()
            .unwrap_or_else(|e| panic!("run {}: {e}", script.display()));
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        eprintln!("{stdout}");
        let end_rc = stdout
            .lines()
            .find_map(|l| l.strip_prefix("COMPARE_DRIVERS_END rc="))
            .and_then(|v| v.trim().parse().ok());
        let rows = stdout.lines().filter(|l| l.starts_with("SHAPE ")).map(parse_row).collect();
        let summary = stdout.lines().find_map(|l| {
            let (n, total) = l.strip_prefix("SHAPES ")?.strip_suffix(" MATCH")?.split_once('/')?;
            Some((n.parse().ok()?, total.parse().ok()?))
        });
        Some(Run { stdout, end_rc, rows, expected, summary })
    })
    .as_ref()
}

/// The job ran to its end and built every described shape with both drivers.
fn complete() -> Option<&'static Run> {
    let r = run()?;
    let rc = r.end_rc.unwrap_or_else(|| panic!("the comparison job printed no end marker (DID NOT RUN):\n{}", r.stdout));
    assert!(rc == 0 || rc == 1, "the comparison job could not run every shape (rc={rc}):\n{}", r.stdout);
    let dnr: Vec<&str> = r.rows.iter().filter(|x| x.verdict == "DID NOT RUN").map(|x| x.name.as_str()).collect();
    assert!(dnr.is_empty(), "DID NOT RUN: {dnr:?}\n{}", r.stdout);
    let got: BTreeSet<(String, String)> = r.rows.iter().map(|x| (x.game.clone(), x.shape.clone())).collect();
    let missing: Vec<_> = r.expected.difference(&got).collect();
    let extra: Vec<_> = got.difference(&r.expected).collect();
    assert!(
        missing.is_empty() && extra.is_empty() && got.len() == r.rows.len(),
        "the job's rows are not the described shapes: missing {missing:?} (add a row to scripts/compare-drivers.sh), \
         not described {extra:?}, {} row(s) for {} distinct shape(s)\n{}",
        r.rows.len(),
        got.len(),
        r.stdout
    );
    let matched = r.rows.iter().filter(|x| x.verdict == "MATCH (declared delta)").count();
    assert_eq!(r.summary, Some((matched, r.rows.len())), "the SHAPES line disagrees with the rows\n{}", r.stdout);
    Some(r)
}

/// A column passes only with the value the ruling requires: `MATCH` for the ROM (exact), and
/// `MATCH(declared-delta)` for the listing and the export, whose declared delta is REQUIRED (a
/// bare `MATCH` there would mean the job compared without it).
fn hold(column: &str, want: &str, pick: fn(&Row) -> &str) {
    let Some(r) = complete() else { return };
    let bad: Vec<String> = r.rows.iter().filter(|x| pick(x) != want).map(|x| format!("{} ({}={})", x.name, column, pick(x))).collect();
    assert!(bad.is_empty(), "{} of {} shape(s) differ in the {column}: {bad:?}\n{}", bad.len(), r.rows.len(), r.stdout);
    eprintln!("driver_comparison: {column} identical on {} shape(s)", r.rows.len());
}

#[test]
fn the_job_builds_every_described_shape_with_both_drivers() {
    if let Some(r) = complete() {
        eprintln!("driver_comparison: {} shape(s) built by both drivers", r.rows.len());
    }
}

#[test]
fn rom_bytes_match_on_every_shape() {
    hold("rom", "MATCH", |x| &x.rom);
}

#[test]
fn listings_match_on_every_shape() {
    hold("lst", "MATCH(declared-delta)", |x| &x.lst);
}

#[test]
fn exports_match_on_every_shape() {
    hold("export", "MATCH(declared-delta)", |x| &x.export);
}
