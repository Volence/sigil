//! Classifying a working tree against the sources this binary is compiled from.
//!
//! `build.rs` derives *which* repository paths cargo compiles the `sigil`
//! binary from; this module decides what a given `git status` entry means once
//! that set is known. The two are split because the derivation needs a cargo
//! subprocess and the decision does not, and the decision is where a wrong
//! answer is dangerous: reporting a real source edit as harmless is the failure
//! that makes a staleness warning worth nothing, and it is reachable through a
//! one-character parsing slip.
//!
//! The file is shared rather than duplicated. `build.rs` reaches it by path,
//! and `main.rs` compiles it under `cfg(test)` so these unit tests run in the
//! normal suite without linking unused code into the shipped executable.
//!
//! Every ambiguity resolves toward "material". A path that cannot be read, a
//! closure that could not be derived: both count as changes to this binary's
//! sources, because a warning that stays quiet when it cannot tell is the
//! defect, not the fix.

use std::collections::BTreeSet;

/// The repository paths this binary is compiled from, or the fact that they
/// could not be determined.
pub struct SourcePaths {
    paths: Vec<String>,
    derived: bool,
}

impl SourcePaths {
    /// A derived set. `paths` are repository-relative, and any path already
    /// covered by an ancestor is dropped so each region is stated once.
    pub fn derived(paths: BTreeSet<String>) -> Self {
        SourcePaths { paths: prune_covered(paths), derived: true }
    }

    /// The set could not be determined. Everything is then material — see the
    /// module note on which direction an unknown resolves toward.
    pub fn undetermined() -> Self {
        SourcePaths { paths: Vec::new(), derived: false }
    }

    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// Whether a repository-relative path is one this binary is compiled from.
    pub fn covers(&self, rel: &str) -> bool {
        if !self.derived {
            return true;
        }
        self.paths
            .iter()
            .any(|p| rel == p || rel.starts_with(&format!("{p}/")))
    }
}

/// Drop any path an ancestor in the same set already covers. Membership is
/// unchanged — [`SourcePaths::covers`] matches by prefix either way — so this
/// only shortens what the banner prints and what a consumer passes to git.
fn prune_covered(paths: BTreeSet<String>) -> Vec<String> {
    let all: Vec<String> = paths.into_iter().collect();
    all.iter()
        .filter(|p| {
            !all.iter()
                .any(|other| other != *p && p.starts_with(&format!("{other}/")))
        })
        .cloned()
        .collect()
}

/// The path from a `git status --porcelain=v1` entry, or `None` when it cannot
/// be read literally.
///
/// A quoted entry carries C-style escapes that are not decoded here, and a path
/// guessed at would be classified against the wrong region, so the caller is
/// told to fall back rather than handed a plausible-looking wrong answer.
pub fn porcelain_path(line: &str) -> Option<String> {
    // The first two columns are the index and worktree status and the third is
    // a separator; the path begins at byte 3 and its leading bytes are part of
    // it. `get`, not slicing, because a status line is external input.
    let rest = line.get(3..)?;
    // A rename or copy renders as `orig -> new`; the new path is the one on
    // disk and the one a classification is about.
    let path = match rest.split_once(" -> ") {
        Some((_, new)) => new,
        None => rest,
    };
    if path.starts_with('"') {
        return None;
    }
    if path.is_empty() {
        return None;
    }
    // An untracked directory is reported with a trailing separator.
    Some(path.trim_end_matches('/').to_string())
}

/// How many working-tree entries fall inside this binary's sources, and how
/// many fall outside them.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub modified: usize,
    pub untracked: usize,
    pub outside: usize,
}

impl Counts {
    pub fn material(&self) -> usize {
        self.modified + self.untracked
    }
}

/// Split `git status --porcelain=v1` output against the sources this binary is
/// compiled from. The input must be the raw stdout: a leading space is the
/// index column of an unstaged modification, so output that has been trimmed at
/// the front describes different files than git reported.
pub fn classify(porcelain: &str, sources: &SourcePaths) -> Counts {
    let mut counts = Counts::default();
    for line in porcelain.lines().filter(|l| !l.trim().is_empty()) {
        let is_untracked = line.starts_with("??");
        let inside = match porcelain_path(line) {
            // An entry whose path could not be read counts as material:
            // "could not classify" must never render as "not a problem".
            None => true,
            Some(path) => sources.covers(&path),
        };
        if !inside {
            counts.outside += 1;
        } else if is_untracked {
            counts.untracked += 1;
        } else {
            counts.modified += 1;
        }
    }
    counts
}

/// The `tree:` state word and its detail.
///
/// Three cases a single dirty flag conflates: nothing changed, something changed
/// that this binary is compiled from, and something changed that it is not. Only
/// the middle one is a reason to distrust the binary, and only it yields a word
/// beginning `dirty`, and a warning that fires on a note left in a documentation
/// directory is a warning nobody reads.
///
/// A CONSUMER MUST NOT KEY ON THE `dirty` PREFIX. That test fails OPEN: any word
/// this vocabulary does not yet contain — a state added later, a typo, an empty
/// capture — does not begin `dirty` and so reads as trustworthy. The correct shape
/// is a POSITIVE match on the trusted words with everything else treated as
/// suspect, which fails CLOSED. The vocabulary is this function's to define
/// (`clean`, `clean-sources`, `dirty`, `unknown` — pinned by
/// `version_provenance.rs`); the fail-safe direction is the consumer's to keep, and
/// a consumer enumerating the trusted words must be told when a word is added.
pub fn state_and_detail(counts: &Counts) -> (String, String) {
    if counts.material() == 0 && counts.outside == 0 {
        return ("clean".to_string(), "no uncommitted changes".to_string());
    }
    let (modified, untracked, outside) = (counts.modified, counts.untracked, counts.outside);
    if counts.material() == 0 {
        return (
            "clean-sources".to_string(),
            format!(
                "{outside} uncommitted change(s), none of them in the sources this binary is \
                 compiled from"
            ),
        );
    }
    let detail = if outside == 0 {
        format!(
            "{modified} modified, {untracked} untracked in the sources this binary is compiled from"
        )
    } else {
        format!(
            "{modified} modified, {untracked} untracked in the sources this binary is compiled \
             from, and {outside} outside them"
        )
    };
    ("dirty".to_string(), detail)
}

/// What cargo must be told to watch so that a tree-state capture describes the
/// tree the binary was actually compiled from.
///
/// [`state_and_detail`] is only ever true *as of the moment it ran*, and a build
/// script runs when cargo decides it is stale. Keying that decision on the
/// revision alone reproduces, one level down, the staleness the whole banner
/// exists to detect: cargo tracks sources for COMPILATION, so a closure source
/// edited and not committed recompiles the crate and relinks the binary while
/// the build script — whose triggers are all revision-shaped — keeps its
/// previous answer. The banner then reports `clean` about a binary built from
/// uncommitted code, which is the one wrong answer it must never give. Keying
/// the capture on the closure's own paths makes it follow the content it
/// describes.
///
/// # The existence filter is load-bearing, and the asymmetry behind it is a trap
///
/// A `cargo:rerun-if-changed` naming a path that does not exist is read by cargo
/// as "always dirty", which forces the build script to re-run on every single
/// build — the unconditional-rerun cost this design refuses on measured grounds.
/// So a derived path is emitted only when it is there.
///
/// The same list is handed out for a second purpose, as git pathspecs, and there
/// a pathspec matching nothing is genuinely harmless — which is why the closure
/// deliberately lists `.cargo` and `rust-toolchain*` whether or not this
/// workspace has them. Harmless as a pathspec is NOT harmless as a rerun
/// trigger, and the two uses do not share the property. A path in `absent` is a
/// real, named hole in the tracking rather than something to quietly emit.
pub enum Triggers {
    /// The closure was derived. `emitted` are the paths cargo must be told to
    /// watch — every derived path that exists — and `absent` are the derived
    /// paths that do not exist and therefore cannot be watched without making
    /// every build dirty.
    Derived { emitted: Vec<String>, absent: Vec<String> },
    /// The closure could not be derived, so every path in the repository is
    /// material ([`SourcePaths::covers`] says so) and no finite trigger set
    /// covers them. Nothing is emitted and a caller must not claim source
    /// tracking. This is not a silent hole: with an underivable closure any dirt
    /// at all already yields `dirty`, which is the loud direction, and the
    /// banner says `NOT DERIVED` in place.
    Undetermined,
}

/// Split the derived closure into the paths cargo can be told to watch and the
/// paths it cannot. `exists` is injected so the decision is testable without a
/// filesystem — the rule, not the disk, is what can be got wrong here.
pub fn source_triggers(sources: &SourcePaths, exists: impl Fn(&str) -> bool) -> Triggers {
    if !sources.derived {
        return Triggers::Undetermined;
    }
    let (emitted, absent) = sources.paths.iter().cloned().partition(|p| exists(p));
    Triggers::Derived { emitted, absent }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources(paths: &[&str]) -> SourcePaths {
        SourcePaths::derived(paths.iter().map(|s| s.to_string()).collect())
    }

    /// The regression that motivates reading raw stdout. `git status
    /// --porcelain=v1` renders an unstaged modification with a leading space,
    /// so a reader that trims the front loses the first entry's first
    /// character — and `argo.lock` matches no source region, which reports a
    /// real edit to a compiled file as harmless dirt.
    #[test]
    fn a_leading_status_column_is_not_stripped_from_the_path() {
        assert_eq!(porcelain_path(" M Cargo.lock").as_deref(), Some("Cargo.lock"));
        assert_eq!(porcelain_path("M  Cargo.lock").as_deref(), Some("Cargo.lock"));
        assert_eq!(porcelain_path("?? docs/note.md").as_deref(), Some("docs/note.md"));

        let counts = classify(" M Cargo.lock\n M crates/x/src/a.rs\n", &sources(&["Cargo.lock", "crates/x/src"]));
        assert_eq!(counts, Counts { modified: 2, untracked: 0, outside: 0 });
    }

    /// A rename is about where the file is now, not where it was.
    #[test]
    fn a_rename_is_classified_by_its_destination() {
        assert_eq!(
            porcelain_path("R  docs/old.md -> crates/x/src/new.rs").as_deref(),
            Some("crates/x/src/new.rs")
        );
        let counts = classify(
            "R  docs/old.md -> crates/x/src/new.rs\n",
            &sources(&["crates/x/src"]),
        );
        assert_eq!(counts.modified, 1, "a file renamed INTO the sources is a source change");
    }

    /// `--untracked-files=normal` collapses an untracked directory to a single
    /// entry with a trailing separator, and it must classify by the directory.
    #[test]
    fn an_untracked_directory_classifies_by_its_prefix() {
        let counts = classify("?? crates/x/src/gen/\n?? docs/scratch/\n", &sources(&["crates/x/src"]));
        assert_eq!(counts, Counts { modified: 0, untracked: 1, outside: 1 });
    }

    /// A quoted path carries escapes this module does not decode, so it is
    /// counted as a source change rather than classified on a guess.
    #[test]
    fn an_unreadable_path_counts_as_a_source_change() {
        assert_eq!(porcelain_path(" M \"odd\\nname\""), None);
        let counts = classify(" M \"odd\\nname\"\n", &sources(&["crates/x/src"]));
        assert_eq!(
            counts,
            Counts { modified: 1, untracked: 0, outside: 0 },
            "a path that could not be read must not be reported as harmless"
        );
    }

    /// THE failure direction of this whole feature. When the source set could
    /// not be derived there is nothing to classify against, and the answer must
    /// be the loud one — never a clean or outside-the-sources verdict.
    #[test]
    fn an_underivable_source_set_makes_every_change_material() {
        let unknown = SourcePaths::undetermined();
        assert!(unknown.covers("docs/OVERSEER.md"));
        assert!(unknown.covers("anything/at/all"));

        let counts = classify(" M docs/OVERSEER.md\n?? notes/x\n", &unknown);
        assert_eq!(counts.outside, 0, "an undetermined source set may not place anything outside");
        assert_eq!(counts.material(), 2);

        let (state, detail) = state_and_detail(&counts);
        assert_eq!(state, "dirty", "an unclassifiable tree must read as dirty, not as clean");
        assert!(
            !detail.contains("none of them in the sources"),
            "an undetermined source set cannot claim a change is outside the sources; got \
             `{detail}`"
        );
    }

    /// A word beginning `dirty` exactly when a compiled source changed, and never
    /// otherwise. This pins the producer's half; a consumer positively matches the
    /// trusted words rather than keying on this prefix, because the prefix test
    /// fails open on any word added after the consumer was written.
    #[test]
    fn only_a_source_change_yields_a_dirty_state() {
        let (clean, detail) = state_and_detail(&Counts::default());
        assert_eq!(clean, "clean");
        assert!(!detail.is_empty(), "every state carries a reason");

        let (outside_only, detail) =
            state_and_detail(&Counts { modified: 0, untracked: 0, outside: 4 });
        assert!(
            !outside_only.starts_with("dirty"),
            "changes outside this binary's sources must not read as dirty; got `{outside_only}`"
        );
        assert!(
            detail.contains('4') && detail.contains("none of them in the sources"),
            "the state must still report what changed; got `{detail}`"
        );

        let (source_change, detail) =
            state_and_detail(&Counts { modified: 1, untracked: 0, outside: 9 });
        assert_eq!(
            source_change, "dirty",
            "one source change outranks any number of changes outside the sources"
        );
        assert!(
            detail.contains("1 modified") && detail.contains("9 outside"),
            "a mixed tree must report both sides separately; got `{detail}`"
        );
    }

    /// A path is covered by an ancestor region, not by sharing its spelling.
    #[test]
    fn coverage_is_by_path_component_not_by_string_prefix() {
        let s = sources(&["crates/sigil-cli"]);
        assert!(s.covers("crates/sigil-cli"));
        assert!(s.covers("crates/sigil-cli/src/main.rs"));
        assert!(
            !s.covers("crates/sigil-cli-extra/src/main.rs"),
            "a sibling whose name merely starts the same is a different package"
        );
    }

    /// Pruning shortens the printed list without changing what it covers.
    #[test]
    fn pruning_removes_only_paths_an_ancestor_already_covers() {
        let s = sources(&["crates/x", "crates/x/src", "crates/x/src/bin", "crates/y/src"]);
        assert_eq!(s.paths(), &["crates/x".to_string(), "crates/y/src".to_string()]);
        assert!(s.covers("crates/x/src/bin/tool.rs"));
        assert!(!s.covers("crates/y/tests/t.rs"));
    }

    fn triggers(sources: &SourcePaths, present: &[&str]) -> (Vec<String>, Vec<String>) {
        match source_triggers(sources, |p| present.contains(&p)) {
            Triggers::Derived { emitted, absent } => (emitted, absent),
            Triggers::Undetermined => panic!("a derived source set must yield derived triggers"),
        }
    }

    /// THE defect this trigger set closes. Every region the classifier will
    /// later be asked about must be one cargo was told to watch, or the capture
    /// is keyed on something other than the content it reports on.
    #[test]
    fn every_watchable_closure_path_becomes_a_trigger() {
        let s = sources(&["Cargo.toml", "crates/x", "crates/y/src"]);
        let (emitted, absent) = triggers(&s, &["Cargo.toml", "crates/x", "crates/y/src"]);
        assert_eq!(emitted, vec!["Cargo.toml", "crates/x", "crates/y/src"]);
        assert!(absent.is_empty(), "nothing was missing, so nothing may be reported as a hole");

        for path in s.paths() {
            assert!(
                emitted.contains(path),
                "`{path}` is classified as a source of this binary but cargo was never told to \
                 watch it, so an edit there would not re-capture the tree state"
            );
        }
    }

    /// The trap. The closure lists workspace-level inputs whether or not they
    /// exist, because a git pathspec matching nothing is harmless — and a rerun
    /// trigger naming a missing path is the opposite of harmless: cargo reads it
    /// as permanently dirty and re-runs the build script on every build, which
    /// is precisely the cost this design refuses.
    #[test]
    fn a_path_that_does_not_exist_is_never_emitted_as_a_trigger() {
        let s = sources(&["Cargo.toml", ".cargo", "rust-toolchain", "rust-toolchain.toml"]);
        let (emitted, absent) = triggers(&s, &["Cargo.toml"]);

        assert_eq!(emitted, vec!["Cargo.toml"]);
        assert_eq!(absent, vec![".cargo", "rust-toolchain", "rust-toolchain.toml"]);
        for missing in &absent {
            assert!(
                !emitted.contains(missing),
                "`{missing}` does not exist; emitting it makes every build dirty forever"
            );
        }
    }

    /// A hole must be reported, not silently dropped. `absent` is what the
    /// banner prints, and a path that vanished from both lists would be a gap in
    /// the tracking that nothing in the output confesses.
    #[test]
    fn every_derived_path_lands_in_exactly_one_of_the_two_lists() {
        let s = sources(&["a", "b", "c", "d"]);
        let (emitted, absent) = triggers(&s, &["a", "c"]);
        let mut all: Vec<String> = emitted.iter().chain(absent.iter()).cloned().collect();
        all.sort();
        assert_eq!(all, s.paths(), "a derived path went missing from both lists");
    }

    /// An underivable closure has no finite trigger set — everything in the
    /// repository is material — so the answer is the loud one rather than an
    /// empty `Derived` that would read as "nothing needed watching".
    #[test]
    fn an_underivable_source_set_yields_no_trigger_claim() {
        assert!(
            matches!(
                source_triggers(&SourcePaths::undetermined(), |_| true),
                Triggers::Undetermined
            ),
            "an undetermined source set must not present itself as fully tracked"
        );
    }
}
