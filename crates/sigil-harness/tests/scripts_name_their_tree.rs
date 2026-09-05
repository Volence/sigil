//! THE SHELL AND PYTHON RESOLVERS OBEY `contract/SUITE_PATHS.md`, PROVEN BY RUNNING THEM.
//!
//! `scripts/lib/suite_paths.sh` and its Python twin implement one precedence for naming
//! another checkout: the explicit variable, then the suite-root variable joined with the
//! repo's directory name, then a derivation from this repo's own location, then a refusal
//! that names everything it consulted and everything it tried. A variable that is SET BUT
//! WRONG is a hard error at its own step, never a null that lets the next step run.
//!
//! Every case below runs the real file, in a subprocess, in a SCRUBBED environment,
//! against a bed this test builds. Reading the sources and agreeing with them would prove
//! nothing: the defect class here is a resolver that looks right and answers wrongly from
//! a linked worktree, which is the shape every agent in this repo runs in.
//!
//! WHY THE BED IS NESTED THE WAY IT IS. Step 3's whole content is that the derivation uses
//! the MAIN checkout's common git directory rather than the working tree's own root. Those
//! two answers are identical whenever the worktree sits BESIDE its repo, so a bed shaped
//! that way would pass with the wrong implementation. The bed therefore puts the linked
//! worktree INSIDE the checkout, exactly where this repo puts its own, where the two
//! answers differ and only the correct one finds the sibling.
//!
//! WHAT THIS FILE DELIBERATELY DOES NOT CONTAIN. `scripts/nightly_source_gates.sh`
//! classifies test files by grepping their text, and a file naming a built artifact is
//! bucketed as artifact-dependent whether or not it reads one. This file obtains no
//! reference tree at all — every path it uses points into a bed it built — so it names no
//! artifact and calls no harness accessor, which lands it in that lane's `no-reference`
//! bucket where it belongs. The one literal it must not grow is the one it is policing,
//! so the detector's needle is assembled rather than written.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

// ── where things are ────────────────────────────────────────────────────────────

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the harness crate sits two levels under the repo root")
        .to_path_buf()
}

fn shell_include() -> PathBuf {
    let p = repo_root().join("scripts/lib/suite_paths.sh");
    assert!(
        p.is_file(),
        "COULD NOT MEASURE: the shell resolver is not at {}, every case below would be \
         asserting about a file that is not there",
        p.display()
    );
    p
}

/// The Python twin, FOUND rather than spelled: writing its directory would put this file
/// in the wrong bucket of the nightly lane's classifier (see the module doc).
fn python_helper() -> PathBuf {
    let mut hits = Vec::new();
    find_named(&repo_root(), "suite_paths.py", &mut hits);
    assert_eq!(
        hits.len(),
        1,
        "COULD NOT MEASURE: expected exactly one Python resolver in the tree, found {:?}. \
         Two copies of one precedence is the defect this parcel exists to end.",
        hits
    );
    hits.remove(0)
}

fn find_named(dir: &Path, name: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let base = e.file_name();
        let base = base.to_string_lossy();
        if base.starts_with('.') || base == "target" {
            continue;
        }
        if p.is_dir() {
            find_named(&p, name, out);
        } else if base == name {
            out.push(p);
        }
    }
}

/// A scratch directory on disk and NEVER under the system temp directory: it is tmpfs on
/// this machine, and work sent there has wedged the shell before. Derived from this test
/// binary's own location, which is inside the target directory whatever the caller set it
/// to, so it needs no environment variable to be right.
fn scratch(tag: &str) -> PathBuf {
    let exe = std::env::current_exe().expect("COULD NOT MEASURE: this test has no path");
    let target = exe
        .ancestors()
        .nth(3)
        .expect("COULD NOT MEASURE: the test binary is not nested under a target directory")
        .to_path_buf();
    let dir = target
        .join("suite-paths-beds")
        .join(format!("{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir {}: {e}", dir.display()));
    dir
}

// ── the bed ─────────────────────────────────────────────────────────────────────

/// A stand-in suite root: a `sigil` checkout carrying a COPY of the real resolver, an
/// engine checkout beside it, and a linked worktree nested inside the first.
struct Bed {
    root: PathBuf,
}

/// The bed is REMOVED AFTER, including on a panic — `contract/SUITE_PATHS.md` asks for the
/// temporary worktree to be cleaned up, and a bed left behind is a `git worktree` registration
/// in a repository that no longer exists once the scratch tree is swept.
///
/// Nothing is lost by removing it on failure: every assertion in this file quotes the
/// subprocess's whole merged output, so the evidence is in the panic message rather than on
/// disk.
impl Drop for Bed {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Bed {
    /// `with_engine` false builds the step-4 bed: a suite root with no engine checkout in
    /// it, which is the state a refusal is supposed to describe.
    fn plant(tag: &str, with_engine: bool) -> Bed {
        let root = scratch(tag);
        let bed = Bed { root };

        // The calling repo, with the real resolver copied in at its real relative path.
        let sigil = bed.root.join("sigil");
        std::fs::create_dir_all(sigil.join("scripts/lib")).expect("mkdir");
        std::fs::create_dir_all(sigil.join("crates/sigil-harness")).expect("mkdir");
        std::fs::write(sigil.join("Cargo.toml"), "# bed\n").expect("write");
        std::fs::copy(shell_include(), sigil.join("scripts/lib/suite_paths.sh")).expect("copy");
        // The Python twin sits at the same relative depth from the checkout root as the
        // real one, so its own `--git-common-dir` walk is measured, not simulated.
        let pydir = sigil.join("crates/sigil-harness/fixtures/ab");
        std::fs::create_dir_all(&pydir).expect("mkdir");
        std::fs::copy(python_helper(), pydir.join("suite_paths.py")).expect("copy");
        git_init(&sigil);

        if with_engine {
            let aeon = bed.root.join("aeon");
            std::fs::create_dir_all(aeon.join("engine")).expect("mkdir");
            std::fs::write(aeon.join("build.sh"), "#!/bin/sh\nexit 0\n").expect("write");
            git_init(&aeon);
        }

        // A directory that is a checkout and is NOT the engine: the set-but-wrong case
        // needs a plausible wrong answer, not an obviously absent one.
        let decoy = bed.root.join("decoy");
        std::fs::create_dir_all(&decoy).expect("mkdir");
        std::fs::write(decoy.join("README"), "not an engine checkout\n").expect("write");
        git_init(&decoy);

        // THE LINKED WORKTREE, NESTED INSIDE THE CHECKOUT. See the module doc: beside it,
        // the wrong implementation and the right one agree.
        let wt = sigil.join("nested/wt");
        std::fs::create_dir_all(sigil.join("nested")).expect("mkdir");
        let out = Command::new("git")
            .args(["-C", &sigil.display().to_string(), "worktree", "add", "--detach"])
            .arg(&wt)
            .output()
            .expect("COULD NOT MEASURE: git could not run");
        assert!(
            out.status.success(),
            "COULD NOT MEASURE: the bed's linked worktree was not created:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        bed
    }

    fn root(&self) -> &Path {
        &self.root
    }
    fn checkout(&self) -> PathBuf {
        self.root.join("sigil")
    }
    fn worktree(&self) -> PathBuf {
        self.checkout().join("nested/wt")
    }
    fn engine(&self) -> PathBuf {
        self.root.join("aeon")
    }
    fn decoy(&self) -> PathBuf {
        self.root.join("decoy")
    }
}

fn git_init(dir: &Path) {
    let d = dir.display().to_string();
    for args in [
        vec!["-C", &d, "init", "-q"],
        vec!["-C", &d, "add", "-A"],
        vec![
            "-C",
            &d,
            "-c",
            "user.email=bed@example.invalid",
            "-c",
            "user.name=bed",
            "commit",
            "-q",
            "-m",
            "bed",
            "--allow-empty",
        ],
    ] {
        let out = Command::new("git")
            .args(&args)
            .output()
            .expect("COULD NOT MEASURE: git could not run");
        assert!(
            out.status.success(),
            "COULD NOT MEASURE: `git {}` failed in the bed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

// ── running the resolvers ───────────────────────────────────────────────────────

/// stdout and stderr together, because the announce and the answer travel on different
/// streams on purpose and a caller reads both.
fn merged(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Run the SHELL resolver from inside `from`, with only the named variables set.
fn ask_shell(from: &Path, include: &Path, env: &[(&str, &str)]) -> (Output, String) {
    let script = format!(
        "set -u\n. {inc}\np=$(suite_resolve_checkout aeon AEON_DIR)\nrc=$?\n\
         printf 'RC=%s\\nANSWER=%s\\n' \"$rc\" \"$p\"\nexit $rc\n",
        inc = shell_quote(include)
    );
    let mut cmd = Command::new("bash");
    cmd.arg("-c").arg(&script).current_dir(from).env_clear();
    // The floor a scrubbed environment still needs to run bash and git at all.
    cmd.env("PATH", std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into()));
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("COULD NOT MEASURE: bash could not run");
    let text = merged(&out);
    (out, text)
}

/// Run the PYTHON resolver the same way. Asked for the engine checkout so both halves are
/// answering the identical question about the identical bed.
fn ask_python(from: &Path, helper_dir: &Path, env: &[(&str, &str)]) -> (Output, String) {
    let body = format!(
        "import sys\nsys.path.insert(0, {dir})\nimport suite_paths\n\
         try:\n    print('ANSWER=' + suite_paths.resolve_checkout('aeon', 'AEON_DIR'))\n\
         except suite_paths.SuitePathError as e:\n    print('REFUSED'); print(e); sys.exit(3)\n",
        dir = python_quote(helper_dir)
    );
    let mut cmd = Command::new("python3");
    cmd.arg("-c").arg(&body).current_dir(from).env_clear();
    cmd.env("PATH", std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into()));
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("COULD NOT MEASURE: python3 could not run");
    let text = merged(&out);
    (out, text)
}

fn shell_quote(p: &Path) -> String {
    format!("'{}'", p.display().to_string().replace('\'', r"'\''"))
}
fn python_quote(p: &Path) -> String {
    format!("r'{}'", p.display())
}

fn bed_python_dir(bed: &Bed, from_worktree: bool) -> PathBuf {
    let base = if from_worktree { bed.worktree() } else { bed.checkout() };
    base.join("crates/sigil-harness/fixtures/ab")
}

// ── the gates ───────────────────────────────────────────────────────────────────

/// STEP 1 WINS, AND SAYS SO.
///
/// The explicit variable outranks a suite root that is also set and also correct, and the
/// announce names the step — a resolved path with no step beside it leaves a reader unable
/// to tell a deliberate override from a lucky derivation.
#[test]
fn step_one_wins_and_is_announced() {
    let bed = Bed::plant("step1", true);
    let engine = bed.engine().display().to_string();
    let root = bed.root().display().to_string();
    let inc = bed.worktree().join("scripts/lib/suite_paths.sh");

    let (out, text) = ask_shell(
        &bed.worktree(),
        &inc,
        &[("AEON_DIR", &engine), ("EMPYREAN_SUITE_ROOT", &root)],
    );
    assert!(out.status.success(), "step 1 did not resolve:\n{text}");
    assert!(
        text.contains(&format!("ANSWER={engine}")),
        "step 1 answered something other than the variable's own value:\n{text}"
    );
    assert!(
        text.contains(&format!("# AEON_DIR={engine} (step 1: explicit AEON_DIR)")),
        "step 1 resolved without announcing itself as step 1:\n{text}"
    );

    let (pout, ptext) = ask_python(
        &bed.worktree(),
        &bed_python_dir(&bed, true),
        &[("AEON_DIR", &engine), ("EMPYREAN_SUITE_ROOT", &root)],
    );
    assert!(pout.status.success(), "the Python half did not resolve step 1:\n{ptext}");
    assert!(
        ptext.contains(&format!("# AEON_DIR={engine} (step 1: explicit AEON_DIR)")),
        "the two halves must announce identically; the Python half said:\n{ptext}"
    );
}

/// SET BUT WRONG IS A HARD ERROR, NOT A FALLTHROUGH.
///
/// The variable names a real checkout that is not the engine, and a correct suite root is
/// ALSO set — so a resolver that treated the wrong value as a null would answer correctly
/// and leave the caller's environment wrong for everything downstream. The proof that it
/// did not is negative and is asserted as such: no step-2 announce appears.
#[test]
fn a_set_but_wrong_variable_stops_the_resolution_by_name() {
    let bed = Bed::plant("wrong", true);
    let decoy = bed.decoy().display().to_string();
    let root = bed.root().display().to_string();
    let inc = bed.worktree().join("scripts/lib/suite_paths.sh");

    let (out, text) = ask_shell(
        &bed.worktree(),
        &inc,
        &[("AEON_DIR", &decoy), ("EMPYREAN_SUITE_ROOT", &root)],
    );
    assert!(!out.status.success(), "a wrong variable was accepted:\n{text}");
    assert_eq!(
        out.status.code(),
        Some(4),
        "set-but-wrong has its own exit code so a caller can tell it from a plain \
         refusal:\n{text}"
    );
    assert!(
        text.contains(&format!("AEON_DIR={decoy}")),
        "the refusal must name the variable AND the path it held:\n{text}"
    );
    assert!(
        !text.contains("(step 2"),
        "THE FALLTHROUGH: a set-but-wrong variable was treated as a null and the next step \
         answered anyway, which resolves the tree correctly while leaving the environment \
         wrong:\n{text}"
    );

    let (pout, ptext) = ask_python(
        &bed.worktree(),
        &bed_python_dir(&bed, true),
        &[("AEON_DIR", &decoy), ("EMPYREAN_SUITE_ROOT", &root)],
    );
    assert!(!pout.status.success(), "the Python half accepted a wrong variable:\n{ptext}");
    assert!(
        ptext.contains(&format!("AEON_DIR={decoy}")) && !ptext.contains("(step 2"),
        "the Python half must refuse by name and must not fall through:\n{ptext}"
    );
}

/// STEP 2 JOINS THE ROOT.
#[test]
fn step_two_joins_the_suite_root() {
    let bed = Bed::plant("step2", true);
    let root = bed.root().display().to_string();
    let engine = bed.engine().display().to_string();
    let inc = bed.worktree().join("scripts/lib/suite_paths.sh");

    let (out, text) = ask_shell(&bed.worktree(), &inc, &[("EMPYREAN_SUITE_ROOT", &root)]);
    assert!(out.status.success(), "step 2 did not resolve:\n{text}");
    assert!(
        text.contains(&format!("ANSWER={engine}")),
        "step 2 did not join the root with the repo's directory name:\n{text}"
    );
    assert!(
        text.contains("(step 2: EMPYREAN_SUITE_ROOT/aeon)"),
        "step 2 resolved without announcing itself as step 2:\n{text}"
    );

    let (pout, ptext) = ask_python(
        &bed.worktree(),
        &bed_python_dir(&bed, true),
        &[("EMPYREAN_SUITE_ROOT", &root)],
    );
    assert!(pout.status.success(), "the Python half did not resolve step 2:\n{ptext}");
    assert!(
        ptext.contains("(step 2: EMPYREAN_SUITE_ROOT/aeon)"),
        "the two halves must announce identically; the Python half said:\n{ptext}"
    );
}

/// STEP 3 DERIVES FROM THE COMMON GIT DIRECTORY, FROM INSIDE A LINKED WORKTREE.
///
/// This is the case the contract writes in capitals. The working tree's own root answers
/// the wrong question here — the bed's worktree is nested inside the checkout, so a
/// resolver built on it looks for the sibling under `nested/`, finds nothing, and refuses
/// a tree that is present.
#[test]
fn step_three_derives_the_sibling_from_inside_a_linked_worktree() {
    let bed = Bed::plant("step3", true);
    let engine = bed.engine().display().to_string();
    let inc = bed.worktree().join("scripts/lib/suite_paths.sh");

    let (out, text) = ask_shell(&bed.worktree(), &inc, &[]);
    assert!(
        out.status.success(),
        "step 3 refused a sibling that is present, the derivation is answering from the \
         worktree's own root rather than from the common git directory:\n{text}"
    );
    assert!(
        text.contains(&format!("ANSWER={engine}")),
        "step 3 derived a path that is not the sibling checkout:\n{text}"
    );
    assert!(
        text.contains("(step 3: sibling of this checkout via git --git-common-dir)"),
        "step 3 resolved without announcing itself as step 3:\n{text}"
    );

    let (pout, ptext) = ask_python(&bed.worktree(), &bed_python_dir(&bed, true), &[]);
    assert!(
        pout.status.success(),
        "the Python half refused a sibling that is present from inside a linked \
         worktree:\n{ptext}"
    );
    assert!(
        ptext.contains("(step 3: sibling of this checkout via git --git-common-dir)"),
        "the two halves must announce identically; the Python half said:\n{ptext}"
    );
}

/// STEP 4 REFUSES BY NAME.
///
/// A refusal is only worth the fix it makes readable, so it must carry BOTH variable names
/// and the path it tried. This bed has no engine checkout at all, which is the state the
/// message is supposed to describe.
#[test]
fn step_four_refuses_naming_every_variable_and_every_path() {
    let bed = Bed::plant("step4", false);
    let inc = bed.worktree().join("scripts/lib/suite_paths.sh");
    let expected_try = bed.engine().display().to_string();

    let (out, text) = ask_shell(&bed.worktree(), &inc, &[]);
    assert!(
        !out.status.success(),
        "there is no engine checkout in this bed, and something answered anyway, a silent \
         fallback is the defect this step exists to end:\n{text}"
    );
    assert_eq!(out.status.code(), Some(3), "step 4 has its own exit code:\n{text}");
    // EACH VARIABLE MUST BE NAMED AS CONSULTED, not merely mentioned. Both names also
    // appear in the message's closing advice, so a bare `contains` passes a refusal that
    // never says it looked at them — measured: dropping the `consulted` line for the
    // suite-root variable left this gate green until the check was tightened to the line.
    for var in ["AEON_DIR", "EMPYREAN_SUITE_ROOT"] {
        assert!(
            consulted_line(&text, var),
            "the refusal has no line saying it CONSULTED `{var}`. Naming it only in the \
             closing advice tells a reader what to set, not what was already looked \
             at:\n{text}"
        );
    }
    assert!(
        text.contains(&expected_try),
        "the refusal does not name the path it tried, so the fix is not readable from the \
         message:\n{text}"
    );

    let (pout, ptext) = ask_python(&bed.worktree(), &bed_python_dir(&bed, false), &[]);
    assert!(!pout.status.success(), "the Python half answered with no checkout present:\n{ptext}");
    for var in ["AEON_DIR", "EMPYREAN_SUITE_ROOT"] {
        assert!(
            consulted_line(&ptext, var),
            "the Python refusal has no line saying it CONSULTED `{var}`:\n{ptext}"
        );
    }
    assert!(
        ptext.contains(&expected_try),
        "the Python refusal does not name the path it tried:\n{ptext}"
    );
}

/// One line of `text` both says `consulted` and names `var`.
fn consulted_line(text: &str, var: &str) -> bool {
    text.lines()
        .any(|l| l.contains("consulted") && l.contains(var))
}

/// THE ANNOUNCE MUST NOT READ AS AN UNMEASURED GATE.
///
/// `scripts/landing-run.sh` counts both spellings of a skipped gate out of its own log. A
/// successful resolution prints a line on every run, and a line matching that counter
/// would add one to it every time — a witness that inflates on success is worse than none.
#[test]
fn the_announce_is_not_counted_as_a_skipped_gate() {
    let bed = Bed::plant("bar", true);
    let inc = bed.worktree().join("scripts/lib/suite_paths.sh");
    let (_, text) = ask_shell(&bed.worktree(), &inc, &[]);
    // The two spellings the bar matches, assembled so this file is not itself a hit when
    // the bar is ever pointed at the test tree.
    for spelling in [format!("{}:", "sk".to_owned() + "ip"), "skip".to_owned() + "ping"] {
        assert!(
            !text.contains(&spelling),
            "the resolver's own output contains `{spelling}`, which the landing bar counts \
             as a gate that measured nothing:\n{text}"
        );
    }
}

// ── the lint ────────────────────────────────────────────────────────────────────

/// NO FILE THAT USES THE RESOLVER MAY RE-GROW A HOME LITERAL.
///
/// The population is DERIVED TWICE OVER: the resolver's own entry points are read out of
/// the resolver's source, and the callers are every runnable file in the tree that names
/// one of them. A hand-written list is the failure mode this whole parcel is about — it
/// goes stale the moment somebody adds the twenty-fifth caller, and a lint over a stale
/// list reports green about files it never opened.
///
/// WHY THE ENTRY POINTS AND NOT THE FILE NAME. The predecessor of this population matched
/// files whose `source` line spelled the include's path — and MISSED the two scripts that
/// hold that path in a variable first, which is every caller that has to check whether the
/// include is reachable at all. Measured, not supposed: a literal planted in one of them
/// left this lint green. A function name cannot be held in a variable the same way.
///
/// THE POPULATION IS RUNNABLE FILES ONLY — `.sh`, `.py`, `.conf`. A document describing a
/// path is not a resolver, and the note explaining this parcel quotes refusals full of the
/// literal it is about; judging prose by a resolver's standard would make the rule
/// unusable and the first thing it broke would be its own explanation.
#[test]
fn no_resolver_caller_regrows_a_home_literal() {
    // Assembled, so this file does not match its own rule and does not have to be
    // exempted from it — an exemption mechanism is a hole, and the first user of a hole
    // is always the file that opened it.
    let needle = format!("/{}/", "ho".to_owned() + "me");

    let mut files = Vec::new();
    walk(&repo_root(), &mut files);
    assert!(
        files.len() > 100,
        "COULD NOT MEASURE: the walk examined only {} file(s), so its silence says nothing \
         about whether a caller carries a literal",
        files.len()
    );

    let entries = resolver_entry_points();
    assert!(
        entries.len() >= 2,
        "COULD NOT MEASURE: only {} entry point(s) could be read out of the resolver, so \
         the caller population is derived from almost nothing: {entries:?}",
        entries.len()
    );
    let announced = announced_variables();
    assert!(
        !announced.is_empty(),
        "COULD NOT MEASURE: the resolver announces no variable under a literal name, so a \
         file that consumes a resolution without calling anything, a sourced config, \
         cannot be reached by this population at all"
    );

    let me = Path::new(file!()).file_name().map(|n| n.to_owned());
    let mut callers = Vec::new();
    let mut offenders = Vec::new();
    for f in &files {
        if f.file_name().map(|n| n.to_owned()) == me {
            continue;
        }
        let runnable = f
            .extension()
            .is_some_and(|e| e == "sh" || e == "py" || e == "conf");
        if !runnable {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        if !calls_the_resolver(&text, &entries, &announced) {
            continue;
        }
        callers.push(f.clone());
        for (i, line) in text.lines().enumerate() {
            if line.contains(&needle) {
                offenders.push(format!(
                    "{}:{}: {}",
                    f.strip_prefix(repo_root()).unwrap_or(f).display(),
                    i + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        !callers.is_empty(),
        "COULD NOT MEASURE: no file in the tree sources or imports the resolver, so this \
         lint judged nothing. Either the resolver has no callers or the detector no longer \
         matches how they reach it."
    );
    assert!(
        offenders.is_empty(),
        "{} file(s) that use the resolver still name one person's home directory. That is \
         the literal the resolver exists to replace, and a caller carrying both resolves \
         correctly on one machine and by accident everywhere else:\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
}

/// The resolver's public entry points, read out of the resolver itself — `suite_*()`
/// function definitions in the shell include. A second spelling of this list is a second
/// thing to keep in step, and this repo has been bitten by exactly that before.
fn resolver_entry_points() -> Vec<String> {
    let text = std::fs::read_to_string(shell_include())
        .unwrap_or_else(|e| panic!("COULD NOT MEASURE: read the resolver: {e}"));
    let mut names: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let t = l.trim_end();
            let name = t.strip_suffix("() {")?.trim();
            (name.starts_with("suite_") && !name.contains(char::is_whitespace))
                .then(|| name.to_string())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The variables the resolver ANNOUNCES under a literal name, read out of the resolver.
///
/// Today that is the suite-root variable. A file does not have to call a function to
/// consume a resolution: `scripts/drift-nightly.conf` is SOURCED by the job after the job
/// has resolved and exported the root, and expands it. That file holds a resolved path and
/// calls nothing, so an entry-point-only population would never judge it — which is exactly
/// how the literal this parcel removed from it sat there in the first place.
fn announced_variables() -> Vec<String> {
    let text = std::fs::read_to_string(shell_include())
        .unwrap_or_else(|e| panic!("COULD NOT MEASURE: read the resolver: {e}"));
    let mut names: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("suite_paths_announce ")?;
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
                .collect();
            (name.len() > 1).then_some(name)
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// A file REACHES the resolver, as opposed to discussing it: it names one of the resolver's
/// entry points, imports the Python module by name, or EXPANDS a variable the resolver
/// announces — the third being how a sourced config consumes a resolution without calling
/// anything. Expansion syntax is required, so prose naming the variable is not swept in.
fn calls_the_resolver(text: &str, entries: &[String], announced: &[String]) -> bool {
    entries.iter().any(|e| text.contains(e.as_str()))
        || announced.iter().any(|v| text.contains(&format!("${{{v}")))
        || text.lines().any(|l| {
            let t = l.trim();
            t.starts_with("from suite_paths ") || t.starts_with("import suite_paths")
        })
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let base = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if base.starts_with('.') || base == "target" {
            continue;
        }
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}
