//! THE GAP BETWEEN THE FREEZE'S STEPS, made self-describing.
//!
//! `refreeze --freeze` runs four regeneration steps in sequence — capture the golden
//! blobs, derive the off-canonical size tables, re-pin `src/pins.rs`, append the ledger
//! entry. `golden/atomic_freeze.sh` made the FIRST step internally atomic; this module is
//! about the three joints BETWEEN the steps, which are the wider exposure.
//!
//! WHY THE JOINTS ARE WORSE THAN THE HALF-WRITE THEY SIT BESIDE. A capture killed
//! part-way leaves a blob that is obviously damaged. A run killed between two steps
//! leaves every artifact individually well-formed: fresh goldens beside a size table, a
//! `pins.rs` and a `provenance.toml` that all parse, all look current, and all describe
//! the PREVIOUS freeze. Nothing in the tree distinguishes that from a completed run —
//! and for a byte-neutral freeze under `--supersede-tip`, a kill at the last joint leaves
//! a tree byte-identical to one where the freeze never ran, with the entry that should
//! have been appended silently absent. The whole capture runs past the time a foreground
//! command can be held open, so this is an ordinary event, not a hypothetical one.
//!
//! WHAT THIS DOES, STATED NARROWLY. It does not prevent the inconsistent state. It
//! RECORDS which steps completed, so the state is nameable afterwards:
//!
//!   * `--freeze` opens a journal before the first step, records each step as it
//!     completes, and removes it only when the run finishes. Its ABSENCE is the only
//!     statement that a freeze completed.
//!   * A leftover found by the next `--freeze` is announced in full and then REPLACED —
//!     never refused. That run regenerates every artifact the leftover names, so a
//!     refusal would obstruct the recovery instead of protecting anything, and a refusal
//!     that can fire on a correct run is worse than the defect it guards.
//!   * `--check` and `--attest` REFUSE over a leftover, because they are the two readers
//!     that would otherwise pronounce on a tree whose artifacts do not correspond to each
//!     other. `--check` is the gate the strict suite runs, so an interrupted freeze
//!     cannot be committed green.
//!
//! LOUD ON UNMEASURABLE. A journal that exists but cannot be read or carries an unknown
//! version is NOT read as "the run completed": it reports `COULD NOT MEASURE` for which
//! steps ran and treats every artifact as possibly-stale, which is the conservative
//! direction. A journal that cannot be CREATED refuses the freeze rather than running one
//! whose interruption could not be detected.
//!
//! THE RECORD LAGS THE WORK, DELIBERATELY. A step is recorded AFTER it returns, so a kill
//! in that instant makes the journal understate progress — it will call a fresh artifact
//! stale. Recovery regenerates it either way, so understating costs a rebuild and
//! overstating would cost a silent inconsistency.
//!
//! WHAT IT STILL DOES NOT SEE, named rather than left for a reader to discover:
//!
//!   * `golden/capture_goldens.sh --write` run BY HAND, outside `refreeze`, writes fresh
//!     goldens that no journal describes. That is a deliberate manual act rather than an
//!     interrupted ritual, and it is not this module's to record — the script's own WRITE
//!     GATE refuses it unless the operator acknowledges it, and an acknowledged one
//!     records itself in `golden/.unjournalled-write`. So a hand write is still outside
//!     this journal; it is no longer silent.
//!   * A kill in the instant between the LAST record and the removal leaves a journal
//!     recording a finished run. Nothing is inconsistent there, so it is reported as a
//!     note and discarded — see [`Leftover::completed`].
//!   * Two concurrent freezes in one tree: the second truncates the first's journal, and
//!     the extent then reads as understated. Concurrency is already refused a step later,
//!     by `freeze_open`'s leftover-staging check, and understating is the safe direction.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The journal's name inside the golden directory. A dot-file beside the capture's
/// `.staging`, ignored by git for the same reason: it is a claim about a run in flight,
/// never durable history, and a shared checkout must not be able to sweep it into a
/// commit.
pub const JOURNAL_NAME: &str = ".freeze-journal";

/// Where the journal lives relative to a harness root.
pub const JOURNAL_SUBPATH: &str = "golden/.freeze-journal";

/// The format this module writes and the only one it will read. An unknown version is
/// unmeasurable, not compatible.
const FORMAT_VERSION: &str = "1";

/// One step of `--freeze`, with the artifacts it is the sole producer of.
///
/// The `produces` lists are what makes a recovery instruction exact: a leftover journal
/// partitions them into the set a killed run regenerated and the set it never reached,
/// and both halves are printed as paths rather than as prose.
pub struct StepSpec {
    /// The key recorded in the journal.
    pub key: &'static str,
    /// What the step runs, in the spelling the operator sees in `--freeze`'s own output.
    pub runs: &'static str,
    /// Paths relative to the harness root that this step, and only this step, writes.
    pub produces: &'static [&'static str],
}

/// The four steps, in the order `--freeze` runs them.
pub const STEPS: [StepSpec; 4] = [
    StepSpec {
        key: STEP_CAPTURE,
        runs: "golden/capture_goldens.sh --write",
        produces: &[
            "golden/s4.bin",
            "golden/s4.debug.bin",
            "golden/demo.bin",
            "golden/demo.debug.bin",
            "golden/config_a.bin",
            "golden/config_b.bin",
            "golden/lean.bin",
        ],
    },
    StepSpec {
        key: STEP_SIZES,
        runs: "golden/derive_offcanonical_sizes.sh",
        produces: &[
            "golden/offcanonical_sizes/s4.txt",
            "golden/offcanonical_sizes/s4_debug.txt",
            "golden/offcanonical_sizes/demo.txt",
            "golden/offcanonical_sizes/demo_debug.txt",
            "golden/offcanonical_sizes/config_a.txt",
            "golden/offcanonical_sizes/config_b.txt",
            "golden/offcanonical_sizes/lean.txt",
        ],
    },
    StepSpec { key: STEP_PINS, runs: "repin", produces: &["src/pins.rs"] },
    StepSpec {
        key: STEP_LEDGER,
        runs: "the provenance append",
        produces: &["golden/provenance.toml"],
    },
];

pub const STEP_CAPTURE: &str = "capture";
pub const STEP_SIZES: &str = "sizes";
pub const STEP_PINS: &str = "pins";
pub const STEP_LEDGER: &str = "ledger";

fn spec(key: &str) -> Option<&'static StepSpec> {
    STEPS.iter().find(|s| s.key == key)
}

/// The journal's path under `root`.
pub fn path(root: &Path) -> PathBuf {
    root.join(JOURNAL_SUBPATH)
}

// ── writing: the run in flight ──────────────────────────────────────────────

/// An open journal. Dropping one WITHOUT [`Journal::close`] leaves the file on disk, and
/// that is the point: the file's survival is what a killed run leaves behind for the next
/// reader, and an ordinary failure return is the same state as a kill.
pub struct Journal {
    file: PathBuf,
    root: PathBuf,
    done: Vec<&'static str>,
}

/// Open a journal for a run about to start, replacing any leftover.
///
/// Refuses if the file cannot be written. That refusal cannot fire on a tree where the
/// freeze itself could work: the capture writes seven blobs into this very directory, so
/// an unwritable golden directory is a run that was going to fail anyway — and running it
/// unjournaled would spend twenty minutes producing exactly the state this module exists
/// to name.
pub fn open(root: &Path, aeon_rev: &str, command: &str) -> Result<Journal, String> {
    let file = path(root);
    let mut text = String::new();
    text.push_str("# A `refreeze --freeze` is IN FLIGHT in this tree, or was killed part-way.\n");
    text.push_str("# DO NOT READ THIS FILE'S PRESENCE DIRECTLY. Run `refreeze --check`, which is\n");
    text.push_str("# the only authority on whether a freeze completed; it refuses while this is\n");
    text.push_str("# here. A direct `ls`/`test -f` reports MISSING for a mistyped path, a relative\n");
    text.push_str("# path under a different cwd, or the wrong root -- indistinguishable from the\n");
    text.push_str("# success it would be read as.\n");
    let _ = writeln!(text, "version {FORMAT_VERSION}");
    let _ = writeln!(text, "started {}", now_secs());
    let _ = writeln!(text, "root {}", one_line(&root.display().to_string()));
    let _ = writeln!(text, "aeon_rev {}", one_line(aeon_rev));
    let _ = writeln!(text, "command {}", one_line(command));
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| {
            format!(
                "COULD NOT MEASURE: {} could not be created for the freeze journal ({e}), so a \
                 kill during this run could not be told apart from a run that finished. Refusing \
                 to freeze unjournaled.",
                dir.display()
            )
        })?;
    }
    std::fs::write(&file, &text).map_err(|e| {
        format!(
            "COULD NOT MEASURE: the freeze journal could not be created at {} ({e}), so a kill \
             during this run could not be told apart from a run that finished. Refusing to \
             freeze unjournaled.",
            file.display()
        )
    })?;
    Ok(Journal { file, root: root.to_path_buf(), done: Vec::new() })
}

impl Journal {
    /// Record that one step COMPLETED. Called after the step returns, never before.
    pub fn record(&mut self, key: &'static str) -> Result<(), String> {
        let s = spec(key).ok_or_else(|| format!("freeze journal: `{key}` is not a step"))?;
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.file)
            .map_err(|e| format!("open {} to record `{key}`: {e}", self.file.display()))?;
        writeln!(f, "done {}", s.key)
            .and_then(|()| f.sync_all())
            .map_err(|e| format!("record `{key}` in {}: {e}", self.file.display()))?;
        self.done.push(s.key);
        Ok(())
    }

    /// Close a COMPLETED run: remove the journal.
    ///
    /// Refuses if any step is unrecorded. A journal that can be closed on a partial run is
    /// a journal that says "completed" about something that did not, which is the one
    /// answer this file may never give.
    pub fn close(self) -> Result<(), String> {
        let missing: Vec<&str> =
            STEPS.iter().map(|s| s.key).filter(|k| !self.done.contains(k)).collect();
        if !missing.is_empty() {
            return Err(format!(
                "freeze journal: refusing to close a run that recorded only {}; not reached: {}. \
                 The journal stays at {} so the next reader sees the run did not finish.",
                render_list(&self.done),
                missing.join(", "),
                self.file.display()
            ));
        }
        std::fs::remove_file(&self.file)
            .map_err(|e| format!("remove {}: {e}", self.file.display()))
    }

    /// What is on disk right now, for a run that is about to return an error. Same text
    /// the next reader will get from the leftover, printed while the operator is watching.
    pub fn state_report(&self) -> String {
        read(&self.root).map(|l| l.report()).unwrap_or_else(|| {
            format!(
                "COULD NOT MEASURE: the freeze journal at {} is gone, so which steps of this run \
                 completed cannot be stated.",
                self.file.display()
            )
        })
    }
}

// ── reading: what a killed run left ─────────────────────────────────────────

/// A journal found on disk by a later run.
pub struct Leftover {
    file: PathBuf,
    root: PathBuf,
    /// Why the file could not be understood, if it could not. Every artifact is then
    /// reported as possibly-stale.
    unreadable: Option<String>,
    started: Option<String>,
    aeon_rev: Option<String>,
    command: Option<String>,
    done: Vec<String>,
}

/// Read the journal under `root`. `None` means there is no journal — the only state that
/// says a freeze is not in flight.
pub fn read(root: &Path) -> Option<Leftover> {
    let file = path(root);
    if !file.exists() {
        return None;
    }
    let mut l = Leftover {
        file: file.clone(),
        root: root.to_path_buf(),
        unreadable: None,
        started: None,
        aeon_rev: None,
        command: None,
        done: Vec::new(),
    };
    let src = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            l.unreadable = Some(format!("it could not be read ({e})"));
            return Some(l);
        }
    };
    let mut version = None;
    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, rest) = match line.split_once(' ') {
            Some((k, r)) => (k, r.trim()),
            None => (line, ""),
        };
        match key {
            "version" => version = Some(rest.to_string()),
            "started" => l.started = Some(rest.to_string()),
            "aeon_rev" => l.aeon_rev = Some(rest.to_string()),
            "command" => l.command = Some(rest.to_string()),
            "done" => l.done.push(rest.to_string()),
            _ => {}
        }
    }
    match version.as_deref() {
        Some(FORMAT_VERSION) => {}
        Some(v) => {
            l.unreadable = Some(format!(
                "it is format version `{v}`, which this build does not read (it writes \
                 version {FORMAT_VERSION})"
            ));
            l.done.clear();
        }
        None => {
            l.unreadable = Some("it carries no `version` line".to_string());
            l.done.clear();
        }
    }
    Some(l)
}

impl Leftover {
    /// The journal's own path.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Every step recorded, in journal order.
    pub fn recorded(&self) -> &[String] {
        &self.done
    }

    /// Whether the extent of the previous run could be measured at all.
    pub fn measurable(&self) -> bool {
        self.unreadable.is_none()
    }

    /// A journal recording all four steps: the run finished and only the removal was
    /// missed. Nothing is inconsistent, so this is a note rather than a fault.
    pub fn completed(&self) -> bool {
        self.measurable() && STEPS.iter().all(|s| self.done.iter().any(|d| d == s.key))
    }

    /// Artifacts the killed run REGENERATED — fresh, and described by no ledger entry.
    /// Empty when the extent is unmeasurable, where nothing may be claimed fresh.
    pub fn fresh(&self) -> Vec<&'static str> {
        if !self.measurable() {
            return Vec::new();
        }
        STEPS
            .iter()
            .filter(|s| self.done.iter().any(|d| d == s.key))
            .flat_map(|s| s.produces.iter().copied())
            .collect()
    }

    /// Artifacts the killed run never reached — still the previous freeze's output,
    /// sitting beside the fresh ones. Under an unmeasurable journal this is EVERY
    /// artifact, because none of them can be shown current.
    pub fn stale(&self) -> Vec<&'static str> {
        if !self.measurable() {
            return STEPS.iter().flat_map(|s| s.produces.iter().copied()).collect();
        }
        STEPS
            .iter()
            .filter(|s| !self.done.iter().any(|d| d == s.key))
            .flat_map(|s| s.produces.iter().copied())
            .collect()
    }

    /// The note for a journal that records a finished run. Not a fault: the artifacts
    /// agree with each other and with the ledger.
    pub fn completed_note(&self) -> String {
        format!(
            "a previous `--freeze` recorded all {} steps but did not remove its journal at \
             {}. Every artifact it names is current and the ledger describes them; the journal \
             is discarded.",
            STEPS.len(),
            self.file.display()
        )
    }

    /// The full state report: what ran, what did not, which paths are fresh, which are
    /// stale, and the two exact ways back.
    pub fn report(&self) -> String {
        let mut out = String::new();
        if let Some(why) = &self.unreadable {
            let _ = writeln!(
                out,
                "COULD NOT MEASURE, a freeze journal is present but {why}, so which steps of \
                 the previous `--freeze` completed cannot be stated. Every artifact below is \
                 treated as possibly stale; that is the conservative reading, not a finding \
                 that they are."
            );
        } else {
            let _ = writeln!(
                out,
                "A PREVIOUS `--freeze` DID NOT COMPLETE in this tree. Its artifacts \
                 individually parse and look current; they do not describe one run."
            );
        }
        let _ = writeln!(out, "          journal:  {}", self.file.display());
        if let Some(s) = &self.started {
            let _ = writeln!(out, "          started:  {s} (unix seconds)");
        }
        if let Some(r) = &self.aeon_rev {
            let _ = writeln!(out, "          aeon_rev: {r}");
        }

        if self.measurable() {
            let done: Vec<&StepSpec> = STEPS
                .iter()
                .filter(|s| self.done.iter().any(|d| d == s.key))
                .collect();
            let left: Vec<&StepSpec> = STEPS
                .iter()
                .filter(|s| !self.done.iter().any(|d| d == s.key))
                .collect();
            let _ = writeln!(out, "          COMPLETED:   {}", render_steps(&done));
            let _ = writeln!(out, "          NOT REACHED: {}", render_steps(&left));
        }

        let fresh = self.fresh();
        if !fresh.is_empty() {
            let _ = writeln!(
                out,
                "\n          FRESH, regenerated by the interrupted run, described by no ledger \
                 entry:"
            );
            for p in &fresh {
                let _ = writeln!(out, "            {}", self.root.join(p).display());
            }
        }
        let stale = self.stale();
        if !stale.is_empty() {
            let _ = writeln!(
                out,
                "\n          STALE, {}:",
                if fresh.is_empty() {
                    "not shown to have been regenerated, so possibly the PREVIOUS freeze's output"
                } else {
                    "still the PREVIOUS freeze's output, sitting beside those"
                }
            );
            for p in &stale {
                let _ = writeln!(out, "            {}", self.root.join(p).display());
            }
        }

        let _ = writeln!(out, "\n          RECOVER by ONE of:");
        match &self.command {
            Some(c) => {
                let _ = writeln!(
                    out,
                    "          (1) Re-run the interrupted freeze. It regenerates every artifact \
                     above and\n              replaces this journal; nothing needs deleting \
                     first:\n\n                {c}\n"
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "          (1) Re-run the interrupted freeze, it regenerates every artifact \
                     above and\n              replaces this journal. COULD NOT MEASURE: the \
                     journal records no command,\n              so the exact invocation is not \
                     recoverable from it.\n"
                );
            }
        }
        if fresh.is_empty() {
            let _ = writeln!(
                out,
                "          (2) Discard the interrupted run: no artifact is known to have been \
                 regenerated,\n              so only the journal is removed.\n\n                \
                 rm {}",
                self.file.display()
            );
        } else {
            let _ = writeln!(
                out,
                "          (2) Discard the interrupted run's output and return to the committed \
                 state:\n\n                git -C {} checkout -- {}\n                rm {}",
                self.root.display(),
                fresh.join(" "),
                self.file.display()
            );
        }
        out
    }
}

// ── small helpers ───────────────────────────────────────────────────────────

fn render_steps(steps: &[&StepSpec]) -> String {
    if steps.is_empty() {
        return "(none)".to_string();
    }
    steps.iter().map(|s| format!("{} ({})", s.key, s.runs)).collect::<Vec<_>>().join(", ")
}

fn render_list(keys: &[&'static str]) -> String {
    if keys.is_empty() {
        "no steps".to_string()
    } else {
        keys.join(", ")
    }
}

/// Collapse anything that would break the one-record-per-line format. A value that spans
/// lines would make the next line parse as a record.
fn one_line(s: &str) -> String {
    s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().trim().to_string()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Quote one word for a shell, so a recorded invocation can be pasted back verbatim.
pub fn sh_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-/=:+@".contains(c))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', r"'\''"))
}
