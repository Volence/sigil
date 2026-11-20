//! The `.lst` source digest for one native build: the files the build read, the module
//! scan's membership, the build configuration and the identity of the ROM it wrote,
//! shaped into a [`sigil_link::SourceDigest`] for [`sigil_link::emit_source_digest`].
//!
//! The rows come from the [`sigil_span::read_set`] snapshot and from nothing else. This
//! module never lists a file kind of its own, so it cannot fall behind what the build
//! opened. What it decides is how each recorded path is written (which root it lies
//! under, which origin it has), and when the snapshot is not a digest this build can
//! vouch for: a file read with two contents, a scan of a root the build does not name, no
//! scan at all, or a scanned module with no read of it.
//!
//! Roots (G0b Task 9): a build names its source roots ([`DigestRoot::Engine`],
//! [`DigestRoot::Game`]) and scans each of them. A path inside the aeon tree is written
//! aeon-relative whichever root it came from, so an in-tree game's rows are the rows it
//! always wrote; a path outside the tree is written under the root that holds it
//! (`root=game`), and a scan member there as `game:<path>`. The profile-driven driver
//! names no roots and scans the tree itself.

use sigil_link::{DigestOrigin, DigestPath, DigestRead, DigestRoot, SourceDigest};
use sigil_span::read_set::Snapshot;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// The root of the sigil checkout this crate was compiled from: the `root=sigil` digest
/// root, where a recorded read under that checkout would be placed.
///
/// No `sigil build` path reads there. The one build input that lived there, the
/// off-canonical size tables, was compiled in (`native::FROZEN_TABLES`) and was deleted
/// with the profile-driven driver (G0b Task 11); `installed_binary_needs_no_build_tree` runs the built binary with this directory
/// hidden. The root stays in the grammar so a digest naming it still parses; this
/// function only computes a path and never requires it to exist.
pub fn sigil_source_root() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = crate_dir.parent().and_then(Path::parent).unwrap_or(crate_dir);
    sigil_span::read_set::canonical(root)
}

/// What the assembler says it is: the facts `sigil --version` reports.
pub struct AssemblerIdentity<'a> {
    pub version: &'a str,
    pub revision: &'a str,
    pub tree_state: &'a str,
}

/// The build configuration a digest records.
pub struct DigestShape<'a> {
    pub target: &'a str,
    pub game: &'a str,
    pub debug: bool,
    pub extra_entries: &'a [String],
}

/// The roots a digest path may be relative to, in the order a path is tried: the aeon
/// tree, the build's named source roots in the order given, the sigil checkout, `/`.
struct Roots {
    aeon: PathBuf,
    named: Vec<(DigestRoot, PathBuf)>,
    sigil: PathBuf,
}

impl Roots {
    fn place(&self, path: &Path) -> Result<DigestPath, String> {
        let filesystem = PathBuf::from("/");
        let order = std::iter::once((DigestRoot::Aeon, &self.aeon))
            .chain(self.named.iter().map(|(r, b)| (*r, b)))
            .chain([(DigestRoot::Sigil, &self.sigil), (DigestRoot::Filesystem, &filesystem)]);
        for (root, base) in order {
            if let Ok(rel) = path.strip_prefix(base) {
                return Ok(DigestPath { root, path: relative_text(rel, path)? });
            }
        }
        Err(format!("{} is not an absolute path, so it lies under no digest root", path.display()))
    }
}

/// `rel` as `/`-joined text, refusing a component the grammar cannot carry.
fn relative_text(rel: &Path, full: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for c in rel.components() {
        match c {
            Component::Normal(s) => parts.push(
                s.to_str().ok_or_else(|| format!("{} is not UTF-8, so the digest cannot name it", full.display()))?,
            ),
            other => {
                return Err(format!("{} has a `{other:?}` component under its root", full.display()));
            }
        }
    }
    Ok(parts.join("/"))
}

/// Where `-o` puts the ROM, as an absolute path with its directory's symlinks
/// resolved, so it is placed under the same roots the reads are.
fn output_location(output: &Path) -> Result<PathBuf, String> {
    let absolute = std::path::absolute(output)
        .map_err(|e| format!("resolve the -o path {}: {e}", output.display()))?;
    match (absolute.parent(), absolute.file_name()) {
        (Some(dir), Some(name)) => match std::fs::canonicalize(dir) {
            Ok(dir) => Ok(dir.join(name)),
            Err(_) => Ok(absolute),
        },
        _ => Ok(absolute),
    }
}

/// The digest of one build, from the recorder's `snapshot` taken after the build's last
/// read. `rom` is the full shipped file exactly as it is about to be written to
/// `rom_output`. `named` is the build's named source roots ([`DigestRoot::Engine`],
/// [`DigestRoot::Game`]), each of which it scanned; empty for a build that scans the aeon
/// tree itself.
#[allow(clippy::too_many_arguments)]
pub fn source_digest(
    aeon: &Path,
    named: &[(DigestRoot, PathBuf)],
    assembler: &AssemblerIdentity<'_>,
    shape: &DigestShape<'_>,
    defines: Vec<(String, i128)>,
    rom: &[u8],
    rom_output: Option<&Path>,
    snapshot: &Snapshot,
) -> Result<SourceDigest, String> {
    let aeon_root = std::fs::canonicalize(aeon)
        .map_err(|e| format!("resolve the aeon root {}: {e}", aeon.display()))?;
    let mut canon_named = Vec::with_capacity(named.len());
    for (root, dir) in named {
        if root.is_external() || *root == DigestRoot::Aeon {
            return Err(format!("{root:?} is not a named source root, so a build cannot name {} as one", dir.display()));
        }
        let dir = std::fs::canonicalize(dir).map_err(|e| format!("resolve the {root:?} root {}: {e}", dir.display()))?;
        canon_named.push((*root, dir));
    }
    let roots = Roots { aeon: aeon_root.clone(), named: canon_named.clone(), sigil: sigil_source_root() };

    if !snapshot.conflicts.is_empty() {
        let lines: Vec<String> = snapshot.conflicts.iter().map(|c| format!("  {c}")).collect();
        return Err(format!(
            "the build's read set cannot be stated as one digest:\n{}",
            lines.join("\n")
        ));
    }

    // The scans: a build that names roots scans each of them and nothing else; one that
    // names none scans exactly its aeon tree.
    let wanted: Vec<&PathBuf> =
        if canon_named.is_empty() { vec![&aeon_root] } else { canon_named.iter().map(|(_, d)| d).collect() };
    for s in &snapshot.scans {
        // A walk of a directory inside a wanted root that found nothing that root's own
        // walk did not is covered by it (the profile-driven driver's sound seams resolve
        // their imports over the engine and sonic4 roots, inside the tree it scans whole):
        // it adds no member and moves no row.
        let covered = wanted.iter().any(|w| {
            s.root.starts_with(w)
                && snapshot.scans.iter().find(|o| &&o.root == w).is_some_and(|whole| {
                    let have: BTreeSet<PathBuf> = whole.files.iter().map(|f| w.join(f)).collect();
                    s.files.iter().all(|f| have.contains(&s.root.join(f)))
                })
        });
        if !wanted.contains(&&s.root) && !covered {
            return Err(format!(
                "the build scanned {}, which is none of the roots it names ({}), and a digest names \
                 only the build's own roots",
                s.root.display(),
                wanted.iter().map(|w| w.display().to_string()).collect::<Vec<_>>().join(", ")
            ));
        }
    }
    let read_paths: BTreeSet<&Path> = snapshot.reads.iter().map(|r| r.path.as_path()).collect();
    let mut scanned = Vec::new();
    for root_dir in &wanted {
        let scan = snapshot.scans.iter().find(|s| &&s.root == root_dir).ok_or_else(|| {
            format!(
                "no module scan of {} was recorded, so the read set missed the walk every build performs",
                root_dir.display()
            )
        })?;
        // Every scanned module was read, or a module could change without moving a row.
        for member in &scan.files {
            let full = root_dir.join(member);
            let on_disk = sigil_span::read_set::canonical(&full);
            if !read_paths.contains(on_disk.as_path()) {
                return Err(format!(
                    "the module scan found {} and no read of it was recorded, so the digest would not \
                     name a module the build parsed",
                    full.display()
                ));
            }
            // In the tree, aeon-relative (the rows an in-tree build always wrote); outside
            // it, under the named root that holds it.
            let placed = roots.place(&full)?;
            scanned.push(match placed.root.name() {
                None => placed.path,
                Some(token) => format!("{token}:{}", placed.path),
            });
        }
    }
    let (scan_files, scan_crc) = sigil_link::digest_scan_identity(&scanned);

    let mut reads = Vec::with_capacity(snapshot.reads.len());
    for r in &snapshot.reads {
        let file = roots.place(&r.path)?;
        let origin = if file.root.is_external() {
            DigestOrigin::External
        } else if r.tool {
            DigestOrigin::Tool
        } else if r.generated {
            DigestOrigin::Generated
        } else {
            DigestOrigin::Source
        };
        reads.push(DigestRead { crc: r.crc, size: r.size, origin, file });
    }

    let rom_output = match rom_output {
        Some(p) => Some(roots.place(&output_location(p)?)?),
        None => None,
    };

    Ok(SourceDigest {
        assembler_version: assembler.version.to_string(),
        revision: assembler.revision.to_string(),
        tree_state: assembler.tree_state.to_string(),
        target: shape.target.to_string(),
        game: shape.game.to_string(),
        debug: shape.debug,
        extra_entries: shape.extra_entries.to_vec(),
        defines,
        scan_files,
        scan_crc,
        reads,
        rom_crc: sigil_span::read_set::crc32(rom),
        rom_size: rom.len() as u64,
        rom_output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_span::read_set::{FileRead, Scan};

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn read(path: PathBuf, crc: u32) -> FileRead {
        FileRead { path, crc, size: 4, sha256: [0; 32], generated: false, tool: false }
    }

    /// The digest names each of a build's roots (G0b Task 9): an engine root inside the
    /// aeon tree keeps its aeon-relative rows (what an in-tree build always wrote), a game
    /// root outside the tree writes `root=game` rows that are sources, not external, and
    /// scan members there are spelled `game:<path>`. A scan of a root the build does not
    /// name is refused by name, and so is a named root the build never scanned. The text
    /// round-trips through the listing grammar.
    #[test]
    fn digest_names_each_root() {
        let tmp = Tmp(std::env::temp_dir().join(format!("sigil_digest_roots_{}", std::process::id())));
        let _ = std::fs::remove_dir_all(&tmp.0);
        let (aeon, game, stray) = (tmp.0.join("aeon"), tmp.0.join("cherub/games/c"), tmp.0.join("aeon/games/other"));
        for d in [aeon.join("engine"), game.clone(), stray.clone()] {
            std::fs::create_dir_all(&d).unwrap();
        }
        for f in [aeon.join("engine/a.emp"), game.join("g.emp"), game.join("game.toml"), stray.join("o.emp")] {
            std::fs::write(&f, "x").unwrap();
        }
        let (aeon, game, stray) =
            (aeon.canonicalize().unwrap(), game.canonicalize().unwrap(), stray.canonicalize().unwrap());
        let engine = aeon.join("engine");
        let named = vec![(DigestRoot::Engine, engine.clone()), (DigestRoot::Game, game.clone())];
        let scans = vec![
            Scan { root: engine.clone(), files: vec![PathBuf::from("a.emp")] },
            Scan { root: game.clone(), files: vec![PathBuf::from("g.emp")] },
        ];
        let snapshot = Snapshot {
            reads: vec![read(engine.join("a.emp"), 1), read(game.join("g.emp"), 2), read(game.join("game.toml"), 3)],
            scans: scans.clone(),
            conflicts: Vec::new(),
        };
        let ident = AssemblerIdentity { version: "0", revision: "r", tree_state: "clean" };
        let shape = DigestShape { target: "project", game: "c", debug: false, extra_entries: &[] };
        let digest = |named: &[(DigestRoot, PathBuf)], snapshot: &Snapshot| {
            source_digest(&aeon, named, &ident, &shape, Vec::new(), b"ROM!", None, snapshot)
        };

        let d = digest(&named, &snapshot).expect("a build over its two roots is one digest");
        let rows: Vec<(DigestRoot, &str, DigestOrigin)> =
            d.reads.iter().map(|r| (r.file.root, r.file.path.as_str(), r.origin)).collect();
        assert_eq!(
            rows,
            [
                (DigestRoot::Aeon, "engine/a.emp", DigestOrigin::Source),
                (DigestRoot::Game, "g.emp", DigestOrigin::Source),
                (DigestRoot::Game, "game.toml", DigestOrigin::Source),
            ]
        );
        let want = sigil_link::digest_scan_identity(&["engine/a.emp".to_string(), "game:g.emp".to_string()]);
        assert_eq!((d.scan_files, d.scan_crc), want, "the scan names each member under its root");
        let text = sigil_link::emit_source_digest(&d).expect("renders");
        assert!(text.contains("root=game") && text.contains("path=g.emp"), "{text}");
        let (back, _) = sigil_link::parse_source_digest(&text).expect("parses back");
        assert_eq!(back, d);

        // A stray scan: a root the build does not name.
        let mut with_stray = snapshot.clone();
        with_stray.scans.push(Scan { root: stray.clone(), files: vec![PathBuf::from("o.emp")] });
        with_stray.reads.push(read(stray.join("o.emp"), 4));
        let err = digest(&named, &with_stray).expect_err("a scan of an unnamed root is refused");
        assert!(err.contains(&stray.display().to_string()) && err.contains("none of the roots it names"), "{err}");

        // A sub-scan inside a scanned root, finding only what that root's walk found, is
        // covered and moves nothing; one that finds a file the root's walk did not is not.
        let mut covered = snapshot.clone();
        covered.scans.push(Scan { root: game.join("sub"), files: Vec::new() });
        std::fs::create_dir_all(game.join("sub")).unwrap();
        let c = digest(&named, &covered).expect("a covered sub-scan is part of its root's walk");
        assert_eq!((c.scan_files, c.scan_crc), (d.scan_files, d.scan_crc));
        let mut uncovered = snapshot.clone();
        uncovered.scans.push(Scan { root: game.join("sub"), files: vec![PathBuf::from("new.emp")] });
        assert!(digest(&named, &uncovered).is_err(), "a sub-scan finding a file its root's walk did not is refused");

        // A named root with no scan of it.
        let mut no_game_scan = snapshot.clone();
        no_game_scan.scans.retain(|s| s.root != game);
        let err = digest(&named, &no_game_scan).expect_err("a named root never scanned is refused");
        assert!(err.contains(&format!("no module scan of {}", game.display())), "{err}");

        // The profile-driven build names no roots and scans the tree: the game root's
        // scan is then a stray, and an aeon-tree scan is the one it wants.
        let err = digest(&[], &snapshot).expect_err("an unnamed build scanning two roots is refused");
        assert!(err.contains("none of the roots it names"), "{err}");
    }
}
