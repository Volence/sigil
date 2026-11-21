//! Named source roots (G0b Task 9, spec §5.3): how a path a source file writes (an
//! `.emp` `embed`/`import`, an AS `include`/`BINCLUDE`) names the file it means.
//!
//! A build has up to three roots, named exactly `engine`, `game` and `generated` (the
//! names G0a's export `files` table uses):
//!
//! | root        | directory                                                    |
//! |-------------|--------------------------------------------------------------|
//! | `engine`    | the engine's root (aeon's `engine/`)                         |
//! | `game`      | the game's project directory (the one holding `game.toml`)   |
//! | `generated` | the build's own generated directory, outside every source tree |
//!
//! # The rule
//!
//! A path names its root by its FIRST segment: `engine/system/math.emp` is
//! `system/math.emp` under the `engine` root, `game/test/ring_art.bin` is
//! `test/ring_art.bin` under the `game` root, `generated/sound/sfx_bank.bin` is
//! `sound/sfx_bank.bin` under the `generated` root. The rest of the path resolves inside
//! that root and only there: a `..` that climbs out of it is refused, and so is a
//! `generated/` path in a build that has no generated root. Resolution never searches:
//! the same relative path under two roots (`engine/x.bin` and the game's own `x.bin`) is
//! two different spellings, `engine/x.bin` and `game/x.bin`.
//!
//! A path whose first segment names no root is UNROOTED. It is refused when the same
//! relative path exists under the `engine` or the `game` root, because the spelling does
//! not say which root's file it means (Review Focus 5). Otherwise:
//!
//!  * a build whose game sits inside the engine's checkout (aeon's own `games/*`) may
//!    resolve it against that checkout, the [`tree`](SourceRoots::tree): this is the
//!    transitional spelling of sonic4's checkout-relative paths (`art/...`,
//!    `games/sonic4/data/...`, several of them written by aeon's generators), booked in
//!    sigil `docs/QUEUE.md` `G0B-UNROOTED-TREE-READS`;
//!  * a build with no tree (a game in its own repository) refuses it by name, naming the
//!    three spellings that would resolve.
//!
//! # The retired sound mount
//!
//! The sound seams' output once lived at `engine/sound/generated/<file>` inside the tree.
//! That prefix is refused by name (`[root.stale-generated]`), never read: the tree's copy
//! there is a stale leftover of aeon's `build.sh`, and a build's own output is the
//! `generated` root (`generated/sound/<file>`). The old driver's mount that read it was
//! deleted with that driver (G0b final review C-I4).

use std::path::{Component, Path, PathBuf};

/// A source root's name. Closed: exactly the three G0a's files table also names.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RootName {
    Engine,
    Game,
    Generated,
}

impl RootName {
    /// Every root, in declaration order.
    pub const ALL: [RootName; 3] = [RootName::Engine, RootName::Game, RootName::Generated];

    /// The root's name as a path's first segment spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            RootName::Engine => "engine",
            RootName::Game => "game",
            RootName::Generated => "generated",
        }
    }

    /// The root a first path segment names, if any.
    pub fn from_segment(seg: &str) -> Option<RootName> {
        RootName::ALL.into_iter().find(|r| r.as_str() == seg)
    }
}

impl std::fmt::Display for RootName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The retired location of the sound seams' output, refused by name (module doc).
const LEGACY_SOUND_MOUNT: &str = "engine/sound/generated";

/// A build's named roots. See the module doc for the rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRoots {
    engine: PathBuf,
    game: PathBuf,
    generated: Option<PathBuf>,
    tree: Option<PathBuf>,
}

/// Where a path resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Located {
    /// Under a named root: the root, the file, and the path relative to the root.
    Root { root: RootName, file: PathBuf, rel: String },
    /// An unrooted path resolved against the transitional tree.
    Tree { file: PathBuf },
}

impl Located {
    /// The file the path names.
    pub fn file(&self) -> &Path {
        match self {
            Located::Root { file, .. } | Located::Tree { file } => file,
        }
    }
}

impl SourceRoots {
    /// Roots `engine` and `game`, with no generated root and no tree. Directories are
    /// kept as given (a file's name is the root joined with the rest of the path, so a
    /// caller that names roots the way it named them before sees the same file names).
    pub fn new(engine: impl Into<PathBuf>, game: impl Into<PathBuf>) -> SourceRoots {
        SourceRoots {
            engine: engine.into(),
            game: game.into(),
            generated: None,
            tree: None,
        }
    }

    /// With the build's `generated` root.
    pub fn with_generated(mut self, dir: impl Into<PathBuf>) -> SourceRoots {
        self.generated = Some(dir.into());
        self
    }

    /// With the transitional tree unrooted paths resolve against (the checkout holding
    /// the engine root, for a game inside it).
    pub fn with_tree(mut self, tree: impl Into<PathBuf>) -> SourceRoots {
        self.tree = Some(tree.into());
        self
    }

    /// The directory of `root`, when this build has it.
    pub fn dir(&self, root: RootName) -> Option<&Path> {
        match root {
            RootName::Engine => Some(&self.engine),
            RootName::Game => Some(&self.game),
            RootName::Generated => self.generated.as_deref(),
        }
    }

    /// The transitional tree, when this build has one.
    pub fn tree(&self) -> Option<&Path> {
        self.tree.as_deref()
    }

    /// Resolve `path` as a source file wrote it. `Err` is the refusal, worded for a
    /// diagnostic (it starts with a bracketed code).
    pub fn locate(&self, path: &str) -> Result<Located, String> {
        let p = Path::new(path);
        if p.is_absolute() || path.is_empty() {
            return Err(format!(
                "[root.absolute] `{path}` is not a relative path; a source path names a root (`engine/...`, `game/...` or `generated/...`)"
            ));
        }
        let mut segs: Vec<&str> = Vec::new();
        for c in p.components() {
            match c {
                Component::Normal(s) => segs.push(s.to_str().unwrap_or("")),
                Component::CurDir => {}
                Component::ParentDir => segs.push(".."),
                Component::RootDir | Component::Prefix(_) => {}
            }
        }
        // The retired prefix is refused by name, however it is spelled (judged on the
        // `..`-folded path): the tree's copy there is rewritten by aeon's build.sh, so
        // resolving it as an engine path would let a stale tree file stand in for this
        // build's own seam output (Task 6's guarantee).
        let mount: Vec<&str> = LEGACY_SOUND_MOUNT.split('/').collect();
        let mut folded: Vec<&str> = Vec::new();
        for s in &segs {
            if *s == ".." {
                folded.pop();
            } else {
                folded.push(s);
            }
        }
        if folded.len() >= mount.len() && folded[..mount.len()] == mount[..] {
            let rest = folded[mount.len()..].join("/");
            return Err(format!(
                "[root.stale-generated] `{path}` names the retired `{LEGACY_SOUND_MOUNT}/` location, which \
                 is not this build's output (the tree's copy there is a stale leftover); the sound seams' \
                 output is the `generated` root: spell it `generated/sound/{rest}`"
            ));
        }
        if let Some(root) = segs.first().and_then(|s| RootName::from_segment(s)) {
            return self.inside(root, path, &segs[1..]);
        }
        // Unrooted. Ambiguous first: the spelling names no root, and a root holds it.
        let rel: PathBuf = segs.iter().collect();
        let mut holders = Vec::new();
        for root in [RootName::Engine, RootName::Game] {
            if let Some(dir) = self.dir(root) {
                if let Some(f) = join_inside(dir, &segs) {
                    if f.exists() {
                        holders.push(root);
                    }
                }
            }
        }
        if !holders.is_empty() {
            let named: Vec<String> = holders.iter().map(|r| format!("`{r}/{}`", rel.display())).collect();
            return Err(format!(
                "[root.ambiguous] `{path}` names no root, and the {} root{} hold{} that path; spell the one meant: {}",
                holders.iter().map(|r| format!("`{r}`")).collect::<Vec<_>>().join(" and "),
                if holders.len() == 1 { "" } else { "s" },
                if holders.len() == 1 { "s" } else { "" },
                named.join(" or ")
            ));
        }
        let Some(tree) = &self.tree else {
            return Err(format!(
                "[root.unrooted] `{path}` names no root; a source path starts with `engine/`, `game/` or `generated/`"
            ));
        };
        match join_inside(tree, &segs) {
            Some(file) => Ok(Located::Tree { file }),
            None => Err(format!("[root.escape] `{path}` climbs out of the checkout it is relative to")),
        }
    }

    fn inside(&self, root: RootName, path: &str, rest: &[&str]) -> Result<Located, String> {
        let Some(dir) = self.dir(root) else {
            return Err(format!("[root.missing] `{path}` names the `{root}` root, and this build has no `{root}` root"));
        };
        if rest.is_empty() {
            return Err(format!("[root.escape] `{path}` names the `{root}` root itself, not a file under it"));
        }
        let Some(file) = join_inside(dir, rest) else {
            return Err(format!("[root.escape] `{path}` climbs out of the `{root}` root"));
        };
        let rel = rest.join("/");
        Ok(Located::Root { root, file, rel })
    }
}

/// `dir` joined with `segs` lexically, or `None` when a `..` climbs above `dir`.
fn join_inside(dir: &Path, segs: &[&str]) -> Option<PathBuf> {
    let mut stack: Vec<&str> = Vec::new();
    for s in segs {
        if *s == ".." {
            stack.pop()?;
        } else {
            stack.push(s);
        }
    }
    let mut out = dir.to_path_buf();
    for s in stack {
        out.push(s);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tree {
        root: PathBuf,
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn tree(tag: &str) -> Tree {
        let root = std::env::temp_dir().join(format!("sigil-roots-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (rel, body) in [
            ("aeon/engine/x.bin", "engine"),
            ("aeon/games/g/x.bin", "game"),
            ("aeon/art/a.bin", "art"),
            ("gen/sound/s.bin", "generated"),
        ] {
            let f = root.join(rel);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(&f, body).unwrap();
        }
        Tree { root }
    }

    fn roots(t: &Tree) -> SourceRoots {
        SourceRoots::new(t.root.join("aeon/engine"), t.root.join("aeon/games/g")).with_generated(t.root.join("gen"))
    }

    fn body(l: &Located) -> String {
        std::fs::read_to_string(l.file()).unwrap()
    }

    /// Review Focus 5: `x.bin` exists under both roots; each spelling reads its own
    /// root's file, and the unrooted spelling is refused naming both.
    #[test]
    fn same_relpath_under_two_roots_resolves_by_root() {
        let t = tree("rf5");
        for r in [roots(&t), roots(&t).with_tree(t.root.join("aeon"))] {
            let e = r.locate("engine/x.bin").unwrap();
            assert_eq!(body(&e), "engine");
            assert!(matches!(e, Located::Root { root: RootName::Engine, ref rel, .. } if rel == "x.bin"));
            let g = r.locate("game/x.bin").unwrap();
            assert_eq!(body(&g), "game");
            assert!(matches!(g, Located::Root { root: RootName::Game, .. }));
            let err = r.locate("x.bin").unwrap_err();
            assert!(err.starts_with("[root.ambiguous]"), "{err}");
            assert!(err.contains("`engine/x.bin`") && err.contains("`game/x.bin`"), "{err}");
        }
    }

    #[test]
    fn a_root_path_cannot_climb_out_of_its_root() {
        let t = tree("escape");
        let r = roots(&t).with_tree(t.root.join("aeon"));
        let err = r.locate("game/../../../art/a.bin").unwrap_err();
        assert!(err.starts_with("[root.escape]"), "{err}");
        assert!(r.locate("game/../x.bin").is_err(), "`..` to the root's own parent climbs out");
        assert_eq!(body(&r.locate("game/sub/../x.bin").unwrap()), "game", "a `..` inside the root stays");
        assert!(r.locate("/etc/passwd").unwrap_err().starts_with("[root.absolute]"));
        assert!(r.locate("engine").unwrap_err().starts_with("[root.escape]"));
    }

    #[test]
    fn generated_names_the_generated_root_and_only_when_there_is_one() {
        let t = tree("gen");
        assert_eq!(body(&roots(&t).locate("generated/sound/s.bin").unwrap()), "generated");
        let none = SourceRoots::new(t.root.join("aeon/engine"), t.root.join("aeon/games/g"));
        let err = none.locate("generated/sound/s.bin").unwrap_err();
        assert!(err.starts_with("[root.missing]"), "{err}");
        // The old prefix is refused by name: the tree's
        // copy there is rewritten by aeon's build.sh and must never stand in for this
        // build's own output (Task 6's guarantee). `..` spellings of it are refused too.
        for old in ["engine/sound/generated/s.bin", "engine/x/../sound/generated/s.bin", "engine/sound/generated"] {
            let err = roots(&t).locate(old).unwrap_err();
            assert!(err.starts_with("[root.stale-generated]"), "{old}: {err}");
            assert!(err.contains("`generated/sound/"), "{old}: the remedy names the generated root: {err}");
        }
        let err = none.locate("engine/sound/generated/s.bin").unwrap_err();
        assert!(err.starts_with("[root.stale-generated]"), "with no generated root too: {err}");
        // A sibling of the old prefix is an ordinary engine path.
        let sib = roots(&t).locate("engine/sound/generated_not/s.bin").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(sib.file(), t.root.join("aeon/engine/sound/generated_not/s.bin"));
    }

    #[test]
    fn an_unrooted_path_needs_a_tree() {
        let t = tree("tree");
        let err = roots(&t).locate("art/a.bin").unwrap_err();
        assert!(err.starts_with("[root.unrooted]"), "{err}");
        let r = roots(&t).with_tree(t.root.join("aeon"));
        assert_eq!(body(&r.locate("art/a.bin").unwrap()), "art");
        assert!(matches!(r.locate("art/a.bin").unwrap(), Located::Tree { .. }));
        assert!(r.locate("../outside.bin").unwrap_err().starts_with("[root.escape]"));
    }
}
