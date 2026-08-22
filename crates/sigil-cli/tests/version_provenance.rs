//! `sigil --version`, observed through the `sigil` BINARY.
//!
//! The property under test is not "a version string is printed" — it is "the
//! executable that aeon actually invokes can be asked which source it came
//! from, and the answer is true". Those differ: a build script that bakes a
//! revision and is then not re-run by cargo keeps reporting the old SHA while
//! the binary relinks around it, which is the same staleness failure one level
//! down, wearing the costume of a fix. So the load-bearing test here is the
//! HEAD-equality one — it is the assertion that goes red if the rerun triggers
//! in `build.rs` ever stop firing.
//!
//! This file reads no aeon input of any kind — no environment pointer to an
//! engine tree, no reference tree, no built ROM, no listing, no golden. It
//! drives the built binary and asks git about the checkout the test itself was
//! compiled from, and that is its whole input set.
//!
//! That is stated in this negative form deliberately. `scripts/nightly_source_gates.sh`
//! classifies every file under `crates/*/tests/` by grepping for the names of
//! those aeon inputs, and refuses to run the whole lane if a match is neither in
//! its `SOURCE_GATES` list nor derivably artifact-dependent. The detector cannot
//! read English, so a file that names an aeon input only to disclaim it is
//! indistinguishable from one that uses it — and the cost is not a false
//! positive on this file, it is the nightly backstop exiting "COULD NOT RUN".
//! Prose in `crates/*/tests/` should therefore describe aeon inputs by
//! description rather than by identifier.
//!
//! ## Residual gap, named rather than papered over
//!
//! Working-tree dirtiness is *not* cross-checkable. `build.rs` captures it as a
//! snapshot and cargo has no trigger that follows it, so a mismatch between the
//! reported tree state and `git status` at test time is legitimate in both
//! directions (the tree may have been edited after capture, or cleaned after
//! it). Asserting either direction would be a flake, so these tests assert the
//! *shape* of the tree claim and that the banner discloses the limitation —
//! and the disclosure itself is asserted, so it cannot be quietly dropped.

use std::process::Command;

/// The checkout this test was compiled from. Every expectation below is
/// derived from asking git about *this* directory at test time, never from a
/// SHA pinned in a fixture.
const REPO: &str = env!("CARGO_MANIFEST_DIR");

/// `sigil --version` stdout. A non-zero exit is a failure in itself: the
/// version banner is what a build script would call to decide whether the
/// assembler is current, so it must not need a success check bolted on.
fn version_stdout(flag: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(flag)
        .output()
        .expect("spawn sigil");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "`sigil {flag}` must exit 0; got {:?}\nstdout: {stdout}\nstderr: {stderr}",
        out.status.code()
    );
    stdout
}

/// Run git in the crate's checkout. Returns `Err` with a reason rather than an
/// empty string, so a caller cannot mistake "could not ask" for "the answer is
/// nothing".
fn git(args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(REPO)
        .output()
        .map_err(|e| format!("git unavailable: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The value of a `  <label>:  <value>` line from the banner body.
fn field(stdout: &str, label: &str) -> String {
    let prefix = format!("{label}:");
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            return rest.trim().to_string();
        }
    }
    panic!("`sigil --version` printed no `{label}:` line; got:\n{stdout}");
}

/// THE INCIDENT TEST. Ties the running binary's reported revision to the tree
/// it is being tested from.
///
/// If `build.rs` fails to re-run when HEAD moves, this binary keeps reporting
/// the revision it was first built at while cargo relinks it against newer
/// code — and that is exactly the state this assertion refuses. Note it cannot
/// be satisfied by a build script that bakes a plausible-looking constant: the
/// expectation is fetched from git at test time.
#[test]
fn version_reports_the_head_of_the_tree_it_was_built_from() {
    let head = git(&["rev-parse", "HEAD"]).unwrap_or_else(|why| {
        // Loud on unmeasurable: a cross-check that cannot run is not a pass.
        panic!(
            "cannot verify the binary's revision against this tree: {why}. \
             This test asserts provenance and has no meaningful weakened form; \
             it needs a git checkout and a `git` on PATH."
        )
    });

    let stdout = version_stdout("--version");
    let reported = field(&stdout, "revision");

    assert_eq!(
        reported, head,
        "the `sigil` binary reports revision {reported} but this checkout's HEAD is {head}. \
         Either build.rs did not re-run when HEAD moved (the rerun triggers are the fix), \
         or HEAD moved while the suite was running (re-run to distinguish).\n\
         full banner:\n{stdout}"
    );
}

/// The branch is captured from the same `.git/HEAD` the revision is, so it is
/// a second, independent reading of the same trigger — a branch switch that
/// lands on the same commit still moves HEAD, and this catches a stamp that
/// missed it.
#[test]
fn version_reports_the_branch_this_tree_is_on() {
    let expected = git(&["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|why| {
        panic!("cannot verify the binary's branch against this tree: {why}")
    });

    let stdout = version_stdout("--version");
    assert_eq!(
        field(&stdout, "branch"),
        expected,
        "reported branch disagrees with this checkout\nfull banner:\n{stdout}"
    );
}

/// `-V` is the same banner, not a truncated one. A short flag that prints less
/// invites scripts to read the cheap one and miss the caveat.
#[test]
fn short_flag_prints_the_same_banner() {
    assert_eq!(
        version_stdout("-V"),
        version_stdout("--version"),
        "`sigil -V` and `sigil --version` must print identical output"
    );
}

/// The first line is the greppable identity: `sigil <semver> (<tag>)`, with the
/// semver derived from this package's own `CARGO_PKG_VERSION` rather than a
/// copied literal.
#[test]
fn first_line_carries_the_crate_version_and_a_revision_tag() {
    let stdout = version_stdout("--version");
    let first = stdout.lines().next().expect("banner has a first line");

    let expected_head = format!("sigil {} (", env!("CARGO_PKG_VERSION"));
    assert!(
        first.starts_with(&expected_head),
        "first line must start with `{expected_head}`; got `{first}`"
    );
    assert!(
        first.ends_with(')'),
        "first line must close its revision tag; got `{first}`"
    );

    let tag = &first[expected_head.len()..first.len() - 1];
    assert!(!tag.is_empty(), "the revision tag must never be empty; got `{first}`");
}

/// No field may render an unknown as blank. An empty value reads as "fine" to
/// a human skimming and passes a grep looking for a SHA, which is precisely the
/// confident-wrong-answer failure this feature exists to prevent.
#[test]
fn no_banner_field_is_blank_or_a_bare_placeholder() {
    let stdout = version_stdout("--version");

    for line in stdout.lines().skip(1) {
        let trimmed = line.trim_start();
        // Continuation lines of the freshness paragraph carry no label.
        let Some((label, value)) = trimmed.split_once(':') else {
            continue;
        };
        if label.contains(' ') || label.is_empty() {
            continue;
        }
        let value = value.trim();
        assert!(
            !value.is_empty(),
            "field `{label}` rendered empty — an unknown must be a word, not a blank\n{stdout}"
        );
        assert!(
            !matches!(value, "-" | "n/a" | "N/A" | "0" | "null" | "none"),
            "field `{label}` rendered the placeholder `{value}` instead of stating what it is\n{stdout}"
        );
        // A dangling em-dash means a reason was promised and not supplied.
        assert!(
            !value.ends_with('—'),
            "field `{label}` promises a reason and gives none: `{value}`\n{stdout}"
        );
    }
}

/// The tag on line one and the `tree:` line are two renderings of one fact and
/// must not disagree — a dirty build tagged with the bare short SHA would read
/// as the clean commit it was built next to.
#[test]
fn the_revision_tag_agrees_with_the_reported_tree_state() {
    let stdout = version_stdout("--version");
    let first = stdout.lines().next().expect("banner has a first line");
    let tag = first
        .rsplit_once('(')
        .map(|(_, t)| t.trim_end_matches(')').to_string())
        .expect("first line carries a parenthesised tag");

    let revision = field(&stdout, "revision");
    let tree = field(&stdout, "tree");

    if revision.starts_with("unknown") || tree.starts_with("unknown") {
        assert!(
            tag == "revision-unknown" || tag.ends_with("-tree-unknown"),
            "an undetermined revision or tree must be tagged as such; tag `{tag}`\n{stdout}"
        );
        return;
    }

    let short = tag.trim_end_matches("-dirty");
    assert!(
        revision.starts_with(short),
        "the tag's short revision `{short}` is not a prefix of `{revision}`\n{stdout}"
    );

    if tree.starts_with("dirty") {
        assert!(
            tag.ends_with("-dirty"),
            "the tree was dirty at capture but the tag `{tag}` does not say so\n{stdout}"
        );
    } else {
        assert_eq!(
            tag, short,
            "a clean tree must tag the bare revision; got `{tag}`\n{stdout}"
        );
    }
}

/// A clean tree is exactly the case where `git status --porcelain` prints
/// nothing, so a capture that reads empty output as a failed probe reports the
/// healthiest possible state as `unknown`. That is the inverse of the rule this
/// feature is built on: loud-on-unmeasurable is a duty owed to states that
/// genuinely cannot be measured, and turning a measured "clean" into "unknown"
/// spends the reader's attention on a non-problem until they stop reading it.
#[test]
fn an_empty_porcelain_reads_as_clean_not_as_unknown() {
    let stdout = version_stdout("--version");
    let tree = field(&stdout, "tree");

    assert!(
        !tree.contains("produced no output"),
        "the tree probe treated empty porcelain output as a failure; empty output IS the \
         clean answer\ntree: {tree}\n{stdout}"
    );

    // A revision proves git answered at capture time, so a `status` that could
    // not answer in the same run is not a plausible environment difference.
    if field(&stdout, "revision").starts_with("unknown") {
        return;
    }
    let porcelain = git(&[
        "--no-optional-locks",
        "status",
        "--porcelain=v1",
        "--untracked-files=normal",
    ])
    .unwrap_or_else(|why| panic!("cannot read this tree's status: {why}"));
    if porcelain.is_empty() {
        assert!(
            !tree.starts_with("unknown"),
            "this checkout is clean and git answered for the revision, so the tree state had \
             no reason to be unknown\ntree: {tree}\n{stdout}"
        );
    }
}

/// The banner must disclose which of its claims cargo re-captures and which it
/// cannot. A witness that admits a limit is a witness; one that silently claims
/// freshness it cannot back is the defect. Asserting the disclosure keeps it
/// from being dropped as noise in a later tidy-up.
#[test]
fn the_banner_discloses_what_it_cannot_track() {
    let stdout = version_stdout("--version");
    let revision = field(&stdout, "revision");

    if revision.starts_with("unknown") {
        assert!(
            stdout.contains("NO revision"),
            "a binary with no revision must say so in capitals, not merely omit it\n{stdout}"
        );
        return;
    }

    let freshness = field(&stdout, "freshness");
    assert!(
        freshness.contains("re-captured"),
        "the banner must state that the revision is re-captured\n{stdout}"
    );
    assert!(
        stdout.contains("snapshot"),
        "the banner must label the tree state a snapshot, not present it as live\n{stdout}"
    );
    assert!(
        stdout.contains("under-report"),
        "the banner must name the direction in which the tree state can be wrong\n{stdout}"
    );
    assert!(
        stdout.contains("git rev-parse HEAD"),
        "the banner must tell a reader how to check this binary against a tree\n{stdout}"
    );
}

/// The rerun triggers are named in the output, so a build that could track
/// nothing cannot present itself the same way as one that tracks HEAD. This
/// asserts what cargo was *told*; the HEAD-equality test above is what proves
/// cargo acted on it.
#[test]
fn the_banner_names_the_rerun_triggers_backing_the_revision() {
    let stdout = version_stdout("--version");
    if field(&stdout, "revision").starts_with("unknown") {
        return;
    }
    let freshness = field(&stdout, "freshness");
    assert!(
        freshness.contains("cargo tracks HEAD"),
        "a revision captured from a git checkout must name `.git/HEAD` as tracked\n{stdout}"
    );
    assert!(
        !freshness.contains("cargo tracks none"),
        "a revision was reported while nothing was tracked — that stamp cannot stay true\n{stdout}"
    );
}
