//! `scripts/s8_seam_size.sh`: THE SIZE REPORT, HELD TO ITS SHAPE AND NEVER TO ITS SIZE.
//!
//! The script exists because S8's cost was written down as prose: "~9,300 LOC" in
//! August 2026, re-derived as "~15,800" a month later, with nothing in between able
//! to notice the drift. It replaced the sentence with a command, and then the
//! command was wired into no runner: its loud-when-a-module-goes-missing path was
//! proven once by hand and nothing re-proved it, and its silent-undercount failure
//! (16,195 where the crate is 18,257) was demonstrated once and then left to the
//! same rot the prose had.
//!
//! **NOTHING HERE ASSERTS A LINE COUNT.** Every number the script prints grows with
//! ordinary work on the harness crate, so pinning one would put a check on a
//! trajectory and red it on correct code, which is the failure mode the script was
//! written to escape. What is asserted is that the report's own arithmetic agrees
//! with itself: each seam total against the rows it totals, the headline against the
//! seam totals, and (the one that catches the undercount class directly) the
//! partition, that the move plus the unassigned remainder is the whole crate. A
//! module that fell out of the seam table without falling out of the crate breaks
//! that equation at any crate size.
//!
//! The refusal path gets a fixture rather than a mutation of the real tree: a
//! throwaway `scripts/` + `crates/sigil-harness/src/` skeleton, run once whole (the
//! control, which must exit 0) and once with a single seam-named module removed
//! (which must exit 2). Without the control the exit-2 run would be evidence only
//! that the fixture was broken somehow, not that the absence was what refused it.
//!
//! Scratch is on disk, derived from this binary's own location inside the target
//! directory, never the system temp directory (tmpfs here).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the harness crate sits two levels under the repo root")
        .to_path_buf()
}

fn script() -> PathBuf {
    let p = repo_root().join("scripts/s8_seam_size.sh");
    assert!(
        p.is_file(),
        "COULD NOT MEASURE: the seam sizer is not at {}, so nothing below would be judging \
         the real script",
        p.display()
    );
    p
}

fn scratch(tag: &str) -> PathBuf {
    let exe = std::env::current_exe().expect("COULD NOT MEASURE: this test has no path");
    let target = exe
        .ancestors()
        .nth(3)
        .expect("COULD NOT MEASURE: the test binary is not nested under a target directory")
        .to_path_buf();
    let dir = target.join("s8-seam-size").join(format!("{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir {}: {e}", dir.display()));
    dir
}

/// One `  <label>   <number> [trailing prose]` row, as `(label, number)`.
///
/// A row is an INDENTED line, because that is how the script prints one
/// (`printf '  %-22s %6d'`), and nothing else in the report is indented: the
/// title, the `tree:` and `counting:` header, the seam and section heads and the
/// `THE MOVE:` headline all start in column 0. Keying on the row's own shape
/// rather than on which header lines happen to hold a number keeps every header
/// field out of the sums whatever it holds. The `tree:` line carries HEAD's short
/// SHA, and a SHA made only of decimal digits (`tree: 64156924`) would otherwise
/// read as a row worth 64,156,924 and be summed into the first seam.
///
/// The number is the FIRST integer token on the line, not the last: the `lib` row
/// carries an explanatory clause after its count, and reading from the right would
/// silently take a number out of prose the day another row grows one.
fn row(line: &str) -> Option<(String, i64)> {
    if !line.starts_with(char::is_whitespace) {
        return None;
    }
    let toks: Vec<&str> = line.split_whitespace().collect();
    let at = toks.iter().position(|t| t.parse::<i64>().is_ok())?;
    if at == 0 {
        return None;
    }
    Some((toks[..at].join(" "), toks[at].parse().ok()?))
}

/// The seam module names the script's own table declares, read out of the script
/// text so the fixture below names what the plan currently names rather than a copy
/// of it that could drift.
fn seam_modules(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let Some(rest) = line.strip_prefix("SEAM_") else { continue };
        let Some((_, list)) = rest.split_once("=(") else { continue };
        let Some((list, _)) = list.split_once(')') else { continue };
        out.extend(list.split_whitespace().map(str::to_string));
    }
    out
}

fn run(script_path: &Path) -> Output {
    Command::new("bash")
        .arg(script_path)
        .output()
        .unwrap_or_else(|e| panic!("run {}: {e}", script_path.display()))
}

/// THE REPORT'S ARITHMETIC AGREES WITH ITSELF, at whatever size the crate is.
///
/// Five relations, none of which names a number:
///   - every seam's declared total is the sum of the rows it printed above it;
///   - the headline `THE MOVE` is the sum of the seam totals;
///   - the headline's module count is the number of module rows actually printed;
///   - the unassigned total is the sum of the unassigned rows;
///   - THE PARTITION: the move plus the unassigned remainder is the whole crate.
///
/// The last is the one that catches the class the script's own header names. A
/// module dropping out of the seam table without dropping out of `src/` makes the
/// report undercount the move silently, and nothing about the printed figure looks
/// wrong; the partition stops balancing the moment it happens.
#[test]
fn the_seam_report_arithmetic_agrees_with_itself() {
    let out = run(&script());
    assert!(
        out.status.success(),
        "the seam sizer could not measure: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_report_agrees(&text);
}

/// The five relations of `the_seam_report_arithmetic_agrees_with_itself`, over any
/// report text, so a fixture report goes through exactly the checks the real one
/// does.
fn assert_report_agrees(text: &str) {
    let mut seam_totals: Vec<i64> = Vec::new();
    let mut seam_rows: Vec<i64> = Vec::new();
    let mut module_rows = 0usize;
    let mut unassigned_rows: Vec<i64> = Vec::new();
    let mut unassigned_total: Option<i64> = None;
    let mut move_lines: Option<i64> = None;
    let mut move_modules: Option<i64> = None;
    let mut whole_crate: Option<i64> = None;
    let mut in_unassigned = false;

    for line in text.lines() {
        if line.starts_with("UNASSIGNED by the seam plan") {
            in_unassigned = true;
            continue;
        }
        if line.starts_with("CONTEXT") {
            in_unassigned = false;
            continue;
        }
        if let Some(rest) = line.strip_prefix("THE MOVE: ") {
            let toks: Vec<&str> = rest.split_whitespace().collect();
            // `<lines> lines across <modules> modules.`
            move_lines = toks.first().and_then(|t| t.parse().ok());
            move_modules = toks.get(3).and_then(|t| t.parse().ok());
            continue;
        }
        let Some((label, value)) = row(line) else { continue };
        match label.as_str() {
            "-- seam total" => {
                seam_totals.push(value);
                let summed: i64 = seam_rows.iter().sum();
                assert_eq!(
                    value, summed,
                    "a seam declares a total of {value} over rows summing to {summed} -- the \
                     report contradicts itself:\n{text}"
                );
                module_rows += seam_rows.len();
                seam_rows.clear();
            }
            "-- unassigned total" => unassigned_total = Some(value),
            "src/*.rs whole crate" => whole_crate = Some(value),
            _ if in_unassigned => unassigned_rows.push(value),
            _ if label.starts_with("src/") || label.starts_with("tests/") => {}
            _ => seam_rows.push(value),
        }
    }

    // A floor of one seam, and only a floor: with no seam section at all every
    // relation below holds vacuously over empty sums, which is a report and a
    // deleted report agreeing with each other.
    assert!(
        !seam_totals.is_empty(),
        "the report printed no seam section, so every total below it is a sum over \
         nothing:\n{text}"
    );

    let move_lines = move_lines.expect("a `THE MOVE:` headline");
    let move_modules = move_modules.expect("a module count in the `THE MOVE:` headline");
    let unassigned_total = unassigned_total.expect("an `-- unassigned total` row");
    let whole_crate = whole_crate.expect("a `src/*.rs whole crate` context row");

    let summed_seams: i64 = seam_totals.iter().sum();
    assert_eq!(
        move_lines, summed_seams,
        "the headline says the move is {move_lines} lines while its seam totals sum to \
         {summed_seams}:\n{text}"
    );
    assert_eq!(
        move_modules, module_rows as i64,
        "the headline says the move spans {move_modules} modules while the seams printed \
         {module_rows} module rows -- a module the plan names is not in the report:\n{text}"
    );
    let summed_unassigned: i64 = unassigned_rows.iter().sum();
    assert_eq!(
        unassigned_total, summed_unassigned,
        "the unassigned remainder declares {unassigned_total} over rows summing to \
         {summed_unassigned}:\n{text}"
    );

    // THE PARTITION. Every `src/*.rs` module is either claimed by the seam table or
    // left in the remainder, so the two must exhaust the crate. A module counted in
    // neither is exactly the silent undercount this script was written after.
    assert_eq!(
        move_lines + unassigned_total,
        whole_crate,
        "the move ({move_lines}) plus the unassigned remainder ({unassigned_total}) is not the \
         whole crate ({whole_crate}) -- {} lines of `src/` are counted in neither, so the move \
         is being reported at a size nothing accounts for:\n{text}",
        whole_crate - (move_lines + unassigned_total)
    );
}

/// A report in the script's own shape whose HEADER FIELDS ARE ALL DIGITS: the
/// `tree:` line an all-digit short SHA prints, and a second numeric header field.
/// Its arithmetic balances (seams 30 + 5 = 35 over 3 module rows, remainder
/// 7 + 3 = 10, and 35 + 10 = 45, the whole crate), so every relation holds and
/// only a parser that reads a header line as a row can refuse it.
const DIGIT_HEADER_REPORT: &str = "\
S8 seam sizing, measured 2026-09-12T08:32:40Z
tree: 64156924
revision: 20260912
counting: physical lines of crates/sigil-harness/src/<module>.rs

sigil-build   <-
  native                     10
  seam1                      20
  -- seam total              30

sigil-pins    <-
  pins                        5
  -- seam total               5

THE MOVE: 35 lines across 3 modules.

UNASSIGNED by the seam plan (stays in sigil-harness, or wants a fourth seam):
  atomic_write                7
  lib                         3  (the module tree itself; rewritten by any split)
  -- unassigned total        10

CONTEXT (not part of the move):
  src/*.rs whole crate       45
  src/bin/*.rs                0
  tests/*.rs                 99
";

/// A header line is never a row, whatever number it carries. The real report's
/// `tree:` line holds HEAD's short SHA, which is all decimal digits for about one
/// commit in fifty (`64156924` was one).
#[test]
fn a_header_field_of_digits_is_never_read_as_a_row() {
    assert_report_agrees(DIGIT_HEADER_REPORT);
}

/// The control for the test above: the same report with one seam row off by one
/// is still refused, at the seam it breaks. A parser that stopped reading rows
/// altogether would pass the test above over empty sums and fail this one on a
/// different message.
#[test]
#[should_panic(expected = "a seam declares a total of 30 over rows summing to 31")]
fn a_seam_total_that_does_not_balance_is_still_refused() {
    let broken = DIGIT_HEADER_REPORT
        .replace("  seam1                      20", "  seam1                      21");
    assert_ne!(broken, DIGIT_HEADER_REPORT, "the fixture edit did not apply");
    assert_report_agrees(&broken);
}

/// THE REFUSAL PATH RUNS, AND IT REFUSES FOR THE REASON CLAIMED.
///
/// The script promises to exit 2 rather than print a figure it cannot stand behind
/// when a module its seam table names has moved out from under it. Proven on a
/// fixture tree, in two runs that differ by exactly one file:
///
///   - the CONTROL, with every named module present, must exit 0. Without it an
///     exit 2 from the second run would be evidence that the fixture was wrong
///     somehow, not that the absence was what refused it.
///   - the same tree with one seam-named module removed must exit 2 AND name it.
///
/// The dropped module is read out of the script's own seam table, so this follows
/// the plan rather than carrying a second copy of it that could drift.
#[test]
fn a_module_missing_from_under_the_seam_table_refuses_instead_of_undercounting() {
    let script_path = script();
    let script_src = std::fs::read_to_string(&script_path).expect("read the seam sizer");
    let seams = seam_modules(&script_src);
    assert!(
        !seams.is_empty(),
        "COULD NOT MEASURE: no `SEAM_*=(...)` table was found in {}, so this test does not \
         know what the script claims to count",
        script_path.display()
    );
    let dropped = seams[0].clone();

    let dir = scratch("refusal");
    let fixture_src = dir.join("crates/sigil-harness/src");
    std::fs::create_dir_all(&fixture_src).expect("mkdir fixture src");
    std::fs::create_dir_all(dir.join("scripts")).expect("mkdir fixture scripts");
    let fixture_script = dir.join("scripts/s8_seam_size.sh");
    std::fs::copy(&script_path, &fixture_script).expect("copy the seam sizer");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fixture_script, std::fs::Permissions::from_mode(0o755))
            .expect("chmod the fixture script");
    }

    // A stub for every module the real crate has, so the fixture's shape follows the
    // crate instead of a list written here that would go stale.
    let real_src = repo_root().join("crates/sigil-harness/src");
    let mut stubbed = 0usize;
    for entry in std::fs::read_dir(&real_src).expect("read the harness src").flatten() {
        let p = entry.path();
        if p.extension().is_some_and(|x| x == "rs") {
            let name = p.file_name().expect("a file name").to_owned();
            std::fs::write(fixture_src.join(&name), "// stub\n").expect("write stub");
            stubbed += 1;
        }
    }
    assert!(stubbed > 0, "COULD NOT MEASURE: no modules in {}", real_src.display());
    assert!(
        fixture_src.join(format!("{dropped}.rs")).is_file(),
        "COULD NOT MEASURE: the seam table names `{dropped}` but the crate has no such module, \
         so this fixture cannot remove it"
    );

    let control = run(&fixture_script);
    assert!(
        control.status.success(),
        "the CONTROL run refused a fixture in which every named module is present, so an exit \
         2 below would say nothing about the absence:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&control.stdout),
        String::from_utf8_lossy(&control.stderr)
    );

    std::fs::remove_file(fixture_src.join(format!("{dropped}.rs"))).expect("remove the module");
    let refused = run(&fixture_script);
    assert_eq!(
        refused.status.code(),
        Some(2),
        "with the seam-named module `{dropped}` absent the sizer must exit 2 rather than print \
         a figure counting one module fewer than the plan names:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    let stderr = String::from_utf8_lossy(&refused.stderr).into_owned();
    assert!(
        stderr.contains("COULD NOT MEASURE") && stderr.contains(&dropped),
        "the refusal must name what went missing, or a reader cannot tell the plan from the \
         crate: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
