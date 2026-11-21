//! The build export's `files` table and the inputs `inputsSha256` hashes, both derived
//! from the read-set snapshot (G0a spec §4.2, §4.6).
//!
//! The recorder (`sigil_span::read_set`) is the one source of truth for what a build
//! read, so everything here is a PROJECTION of it and never a second walk of the tree.
//! Each read is placed in one of exactly four roots, `engine`, `game`, `generated` and
//! `sigil`, by ONE rule for every game (G0b Task 13a): a `files` row is (root name, path
//! relative to that root's directory in this build). The directories are the build's own
//! named roots ([`SourceRoots`]), never a layout of the engine's checkout:
//!
//! | root        | directory                                                       |
//! |-------------|-----------------------------------------------------------------|
//! | `engine`    | the engine root (aeon's `engine/`)                              |
//! | `game`      | the project directory, the one holding `game.toml`              |
//! | `generated` | the build's own generated directory                             |
//! | `sigil`     | the sigil source checkout                                       |
//!
//! A read under more than one root's directory belongs to the LONGEST one, and the engine's
//! checkout competes as well: an engine tree copied INSIDE a root's directory (a test copy
//! under the sigil checkout's target dir) keeps its unrooted reads as the checkout's, never
//! as rows of the root it happens to sit in, which would name where the copy was. No path carries
//! a game id or a checkout-layout segment: the export's `shape.game` names the game, so
//! aeon's `games/sonic4/config/header.emp` and a game in its own repository's
//! `config/header.emp` are both the row `game config/header.emp`.
//!
//! A build writes its generated files into its own generated directory, outside the
//! tree ([`sigil_build::build_dir`], G0b Task 6), and the sources reach them through the
//! build's named `generated` root (`generated/sound/...`). The record layer names a read
//! of that directory at its mount, `<engine>/<sub>/generated/<rest>`
//! ([`mount_generated`], the name the `.lst` digest has always written), and the export
//! states that mount as the `generated` root: under the engine root, `<sub>/generated/<rest>`
//! is the row `generated <sub>/<rest>`, which is also where a read of the generated
//! directory itself lands. So a build's rows do not depend on where its private directory
//! was, and a generator output the engine checkout holds in place (aeon's
//! `engine/debug/generated/`, written by aeon's build.sh) keeps the row it always had.
//!
//! # Two outputs, two scopes
//!
//!  * `files` (the schema's table): SOURCE files only, a read whose extension is
//!    `.emp`, `.asm` or `.z80`, because `lang` is a closed enum of exactly those.
//!  * `inputs` (what `inputsSha256` hashes): EVERY read under a root, sources, art and
//!    sound blobs and `map.toml` alike, by root-relative path and SHA-256. A changed
//!    blob therefore moves `inputsSha256` although no file row changes. A DATA file under
//!    the engine's checkout but under no root (aeon's `art/` directory) is still an input;
//!    it is hashed under the hash-only root name `aeon`, relative to that checkout, and
//!    never reaches the schema.
//!  * A DATA file read from outside every root and the engine's checkout (the
//!    `--anchor-overlay` file) is an input too: hashed under the hash-only root
//!    `external` by file name alone.
//!  * A SOURCE file under no root, inside the engine's checkout or out of it, fits no row
//!    and is an error naming the path and each root with its directory.
//!  * `tool_exes`: executables the build ran (`convsym`) are tool identity, recorded
//!    by file name and hash only, wherever they live.

use sigil_link::export::{FileRecord, InputRead, Lang, RootName, ToolExe};
use sigil_span::read_set::{FileRead, Snapshot};
use sigil_span::roots::SourceRoots;
use std::path::{Component, Path, PathBuf};

/// Everything the export derives from one snapshot.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct ExportInputs {
    /// Source rows for the schema's `files`, sorted by `(root, path)`.
    pub files: Vec<FileRecord>,
    /// Every read `inputsSha256` covers, sorted by `(root, path)`.
    pub inputs: Vec<InputRead>,
    /// Helper executables, by file name.
    pub tool_exes: Vec<ToolExe>,
}

/// The row language of `path`, or `None` for data (no row).
pub fn lang_of(path: &Path) -> Option<Lang> {
    match path.extension()?.to_str()? {
        "emp" => Some(Lang::Emp),
        "asm" => Some(Lang::Asm),
        "z80" => Some(Lang::Z80),
        _ => None,
    }
}

fn rel_text(rel: &Path, full: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for c in rel.components() {
        match c {
            Component::Normal(s) => parts.push(
                s.to_str().ok_or_else(|| format!("{} is not UTF-8, so the export cannot name it", full.display()))?,
            ),
            other => return Err(format!("{} has a `{other:?}` component under its root", full.display())),
        }
    }
    if parts.is_empty() {
        return Err(format!("{} is a root itself, not a file under one", full.display()));
    }
    Ok(parts.join("/"))
}

enum Placed {
    /// Under a schema root.
    Rooted(RootName, String),
    /// Under the engine's checkout, under no root, a data file: hashed as `aeon`.
    AeonData(String),
    /// Outside every root and the engine's checkout, a data file the caller handed the
    /// build (the `--anchor-overlay` file): hashed as `external` by FILE NAME only, since
    /// its directory says where the build ran.
    External(String),
}

/// The directories a build's export places its reads against: the build's named roots,
/// the engine's checkout (for the hash-only `aeon` data rows) and the sigil checkout.
/// Every directory canonical, as the read set records paths.
#[derive(Clone, Copy, Debug)]
pub struct ExportRoots<'a> {
    /// The build's named roots: `engine` = the engine root, `game` = the project
    /// directory (the one holding `game.toml`), `generated` = the build's generated
    /// directory.
    pub roots: &'a SourceRoots,
    /// The checkout holding the engine root.
    pub checkout: &'a Path,
    /// The sigil source checkout.
    pub sigil: &'a Path,
}

impl ExportRoots<'_> {
    /// Each schema root with its directory, in schema order (a build with no generated
    /// root has three).
    fn dirs(&self) -> Vec<(RootName, &Path)> {
        use sigil_span::roots::RootName as R;
        let mut out = vec![(RootName::Engine, self.roots.dir(R::Engine).expect("every build has an engine root"))];
        out.push((RootName::Game, self.roots.dir(R::Game).expect("every build has a game root")));
        if let Some(g) = self.roots.dir(R::Generated) {
            out.push((RootName::Generated, g));
        }
        out.push((RootName::Sigil, self.sigil));
        out
    }

    fn named(&self) -> String {
        self.dirs().iter().map(|(r, d)| format!("`{}` = {}", r.as_str(), d.display())).collect::<Vec<_>>().join(", ")
    }
}

fn place(roots: &ExportRoots<'_>, r: &FileRead) -> Result<Placed, String> {
    place_path(roots, &r.path)
}

/// The `files` row key `(root, path)` of a source file the build located a span in,
/// given the name its front end opened it under (resolved like the read set resolves
/// it: [`sigil_span::read_set::canonical`]). The line table maps each located byte to
/// its row through this, so it is the same placement the `files` rows came from.
pub fn source_row_key(roots: &ExportRoots<'_>, name: &str) -> Result<(RootName, String), String> {
    let path = sigil_span::read_set::canonical(Path::new(name));
    if lang_of(&path).is_none() {
        return Err(format!("{} is not a source file (.emp, .asm or .z80), so it has no `files` row", path.display()));
    }
    match place_path(roots, &path)? {
        Placed::Rooted(root, rel) => Ok((root, rel)),
        Placed::AeonData(_) | Placed::External(_) => Err(format!("{} fits no export root", path.display())),
    }
}

/// Under the engine root, the generated root's mount `<sub>/generated/<rest>` (module
/// doc): `Some(<sub>/<rest>)`, else `None`.
fn generated_mount(under_engine: &Path) -> Option<PathBuf> {
    let mut comps = under_engine.components();
    let sub = comps.next()?;
    let rest = comps.as_path().strip_prefix("generated").ok()?;
    if rest.as_os_str().is_empty() {
        return None;
    }
    Some(Path::new(sub.as_os_str()).join(rest))
}

fn place_path(roots: &ExportRoots<'_>, path: &Path) -> Result<Placed, String> {
    let lang = lang_of(path);
    // The longest root directory holding the path owns it. The engine checkout competes
    // too: when it is nested in a root (an engine tree copied under the sigil checkout)
    // and holds the path, the root does not own it; it is the checkout's unrooted read.
    let under_checkout = path.starts_with(roots.checkout).then(|| roots.checkout.components().count());
    let owner = roots
        .dirs()
        .into_iter()
        .filter(|(_, d)| path.starts_with(d))
        .max_by_key(|(_, d)| d.components().count())
        .filter(|(_, d)| under_checkout.is_none_or(|c| d.components().count() > c));
    if let Some((root, dir)) = owner {
        let rel = path.strip_prefix(dir).expect("filtered on starts_with");
        if root == RootName::Engine {
            if let Some(mounted) = generated_mount(rel) {
                return Ok(Placed::Rooted(RootName::Generated, rel_text(&mounted, path)?));
            }
        }
        return Ok(Placed::Rooted(root, rel_text(rel, path)?));
    }
    if lang.is_some() {
        return Err(format!(
            "{} is a source file the build read from under no export root ({}), so no `files` row can name it",
            path.display(),
            roots.named()
        ));
    }
    if let Ok(rel) = path.strip_prefix(roots.checkout) {
        return Ok(Placed::AeonData(rel_text(rel, path)?));
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("{} has no UTF-8 file name", path.display()))?;
    Ok(Placed::External(name.to_string()))
}

/// A build's `generated` root as the record layer names its reads (G0b Task 9).
///
/// A build reads its generated files from its own generated directory (`<dir>/sound/...`,
/// named in the source as `generated/sound/...`, [`sigil_span::roots`]). G0a's export
/// already holds the convention that the `generated` root's `<sub>/<rest>` is the
/// engine's `<sub>/generated/<rest>` (the table in the module doc), and the `.lst`
/// digest has written a generated read under that engine-tree name since the seams wrote
/// into the tree. [`mounted`](Self::mounted) states that equivalence, so the digest and
/// the export keep naming each generated file as before (`origin=generated
/// path=engine/sound/generated/...`, root `generated` path `sound/...`) and a build's
/// rows do not depend on where its private directory was. This is a NAME, not a
/// resolution: no source path is redirected through it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedRoot {
    /// The generated directory itself (canonical).
    pub dir: PathBuf,
    /// The engine root (canonical).
    pub engine: PathBuf,
}

impl GeneratedRoot {
    /// The generated root `dir` of a build on the engine root `engine`, both canonical.
    pub fn new(dir: &Path, engine: &Path) -> Result<GeneratedRoot, String> {
        let dir = std::fs::canonicalize(dir).map_err(|e| format!("resolve the generated root {}: {e}", dir.display()))?;
        let engine = std::fs::canonicalize(engine).map_err(|e| format!("resolve the engine root {}: {e}", engine.display()))?;
        Ok(GeneratedRoot { dir, engine })
    }

    /// The engine-tree name of a file in the generated directory: `<dir>/<sub>/<rest>` is
    /// `<engine>/<sub>/generated/<rest>`. `None` for a path outside the directory, or one
    /// directly in it (no subsystem directory, so no name under the convention).
    pub fn mounted(&self, physical: &Path) -> Option<PathBuf> {
        let rest = physical.strip_prefix(&self.dir).ok()?;
        let mut comps = rest.components();
        let sub = comps.next()?;
        let tail = comps.as_path();
        if tail.as_os_str().is_empty() {
            return None;
        }
        Some(self.engine.join(sub).join("generated").join(tail))
    }
}

/// `snapshot` with every read of `root`'s directory written under its engine-tree name
/// ([`GeneratedRoot::mounted`]). The `.lst` source digest and the export both place reads
/// through this, so a generated file is `origin=generated path=engine/sound/generated/...`
/// in the digest and root `generated` in the export.
///
/// A read of that engine-tree path as well is refused naming both: the build would then
/// have read two different files under one name. A build resolves `generated/...` to the
/// generated directory only, so this does not happen in a build; the check is what keeps
/// the rewrite from merging two rows silently if it ever did.
pub fn mount_generated(snapshot: &Snapshot, root: &GeneratedRoot) -> Result<Snapshot, String> {
    let in_tree: std::collections::BTreeSet<&Path> = snapshot.reads.iter().map(|r| r.path.as_path()).collect();
    let mut out = snapshot.clone();
    for r in &mut out.reads {
        if r.path.starts_with(&root.dir) && root.mounted(&r.path).is_none() {
            return Err(format!(
                "the build read {} directly in its generated root {}; a generated file lives in a subsystem directory (`sound/`), which is what names it",
                r.path.display(),
                root.dir.display()
            ));
        }
        if let Some(mounted) = root.mounted(&r.path) {
            if in_tree.contains(mounted.as_path()) {
                return Err(format!(
                    "the build read {} and also the generated file {} mounted at that path, so one name \
                     would stand for two files",
                    mounted.display(),
                    r.path.display()
                ));
            }
            r.path = mounted;
        }
    }
    out.reads.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// The export's file rows, hashed inputs and helper executables for `snapshot`, placed
/// against `roots` (module doc). A snapshot with a conflict (a file read with two
/// contents) cannot be stated as one row per file and is an error, as it is for the
/// `.lst` digest.
pub fn export_files(roots: &ExportRoots<'_>, snapshot: &Snapshot) -> Result<ExportInputs, String> {
    if !snapshot.conflicts.is_empty() {
        let lines: Vec<String> = snapshot.conflicts.iter().map(|c| format!("  {c}")).collect();
        return Err(format!("the build's read set cannot be exported as one row per file:\n{}", lines.join("\n")));
    }
    let mut out = ExportInputs::default();
    for r in &snapshot.reads {
        if r.tool {
            let name = r
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("{} has no UTF-8 file name", r.path.display()))?;
            out.tool_exes.push(ToolExe { name: name.to_string(), sha256: r.sha256 });
            continue;
        }
        match place(roots, r)? {
            Placed::Rooted(root, path) => {
                out.inputs.push(InputRead { root: root.as_str().to_string(), path: path.clone(), sha256: r.sha256 });
                if let Some(lang) = lang_of(&r.path) {
                    out.files.push(FileRecord { root, path, lang, sha256: r.sha256, size: r.size });
                }
            }
            Placed::AeonData(path) => {
                out.inputs.push(InputRead { root: "aeon".to_string(), path, sha256: r.sha256 })
            }
            Placed::External(name) => {
                out.inputs.push(InputRead { root: "external".to_string(), path: name, sha256: r.sha256 })
            }
        }
    }
    out.files.sort_by(|a, b| (a.root, &a.path).cmp(&(b.root, &b.path)));
    out.inputs.sort_by(|a, b| (&a.root, &a.path).cmp(&(&b.root, &b.path)));
    out.tool_exes.sort_by(|a, b| (&a.name, a.sha256).cmp(&(&b.name, b.sha256)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(path: &str) -> FileRead {
        FileRead { path: PathBuf::from(path), crc: 0, size: 3, sha256: [9; 32], generated: false, tool: false }
    }

    /// An in-tree game: engine `/a/engine`, game `/a/games/demo`, checkout `/a`.
    fn in_tree() -> SourceRoots {
        SourceRoots::new("/a/engine", "/a/games/demo").with_generated("/b/generated")
    }

    fn with<T>(roots: &SourceRoots, f: impl FnOnce(&ExportRoots<'_>) -> T) -> T {
        f(&ExportRoots { roots, checkout: Path::new("/a"), sigil: Path::new("/s") })
    }

    fn run_on(roots: &SourceRoots, paths: &[&str]) -> Result<ExportInputs, String> {
        let snap = Snapshot { reads: paths.iter().map(|p| read(p)).collect(), ..Default::default() };
        with(roots, |r| export_files(r, &snap))
    }

    fn run(paths: &[&str]) -> Result<ExportInputs, String> {
        run_on(&in_tree(), paths)
    }

    fn rows(paths: &[&str]) -> Result<Vec<(RootName, String)>, String> {
        run(paths).map(|e| e.files.into_iter().map(|f| (f.root, f.path)).collect())
    }

    #[test]
    fn each_read_lands_in_its_root_with_a_relative_path() {
        let got = rows(&[
            "/a/engine/vdp.emp",
            "/a/engine/debug/generated/compression_vectors.emp",
            "/b/generated/sound/seq.z80",
            "/a/games/demo/config/game.emp",
            "/a/engine/debug/debugger.asm",
            "/s/crates/prelude.emp",
        ])
        .unwrap();
        assert_eq!(
            got,
            vec![
                (RootName::Engine, "debug/debugger.asm".to_string()),
                (RootName::Engine, "vdp.emp".to_string()),
                (RootName::Game, "config/game.emp".to_string()),
                (RootName::Generated, "debug/compression_vectors.emp".to_string()),
                (RootName::Generated, "sound/seq.z80".to_string()),
                (RootName::Sigil, "crates/prelude.emp".to_string()),
            ]
        );
    }

    /// One rule for every game: a game in its own repository gets the same rows as the
    /// same game inside the engine's checkout, and no row names the game.
    #[test]
    fn a_game_outside_the_engine_checkout_is_placed_by_its_own_directory() {
        let own = SourceRoots::new("/a/engine", "/elsewhere/cherub");
        let e = run_on(&own, &["/elsewhere/cherub/game_root.asm", "/elsewhere/cherub/src/ship.emp", "/elsewhere/cherub/map.toml", "/a/engine/vdp.emp"])
            .unwrap();
        let got: Vec<(RootName, &str)> = e.files.iter().map(|f| (f.root, f.path.as_str())).collect();
        assert_eq!(got, vec![(RootName::Engine, "vdp.emp"), (RootName::Game, "game_root.asm"), (RootName::Game, "src/ship.emp")]);
        assert!(e.inputs.iter().any(|i| i.root == "game" && i.path == "map.toml"), "{:?}", e.inputs);
        let inside = run(&["/a/games/demo/src/ship.emp"]).unwrap();
        assert_eq!(inside.files[0].path, "src/ship.emp", "the in-tree spelling is the same row");
    }

    /// Longest root wins: a game directory under the engine root's directory is still
    /// placed by the root nearest the file.
    #[test]
    fn the_longest_root_holding_a_read_owns_it() {
        let game_in_engine = SourceRoots::new("/a/engine", "/a/engine/sub/game");
        let got: Vec<(RootName, String)> = run_on(&game_in_engine, &["/a/engine/sub/game/g.emp", "/a/engine/sub/e.emp"])
            .unwrap()
            .files
            .into_iter()
            .map(|f| (f.root, f.path))
            .collect();
        assert_eq!(got, vec![(RootName::Engine, "sub/e.emp".to_string()), (RootName::Game, "g.emp".to_string())]);
    }

    /// An engine checkout INSIDE the sigil checkout (a test copy under the target dir, as
    /// `build_export_gates`' determinism gate makes them): a read under the checkout but under
    /// no engine/game root is the checkout's, never a `sigil` row named by where the copy sits.
    #[test]
    fn a_checkout_inside_the_sigil_checkout_owns_its_unrooted_reads() {
        let roots = SourceRoots::new("/s/target/tmp/r1/tree/engine", "/s/target/tmp/r1/tree/games/demo");
        let er = ExportRoots { roots: &roots, checkout: Path::new("/s/target/tmp/r1/tree"), sigil: Path::new("/s") };
        let snap = Snapshot {
            reads: vec![read("/s/target/tmp/r1/tree/art/pal/s.bin"), read("/s/crates/x/std.emp")],
            ..Default::default()
        };
        let e = export_files(&er, &snap).unwrap();
        let got: Vec<(&str, &str)> = e.inputs.iter().map(|i| (i.root.as_str(), i.path.as_str())).collect();
        assert_eq!(got, vec![("aeon", "art/pal/s.bin"), ("sigil", "crates/x/std.emp")]);
        let src = "/s/target/tmp/r1/tree/other/x.emp";
        let err = export_files(&er, &Snapshot { reads: vec![read(src)], ..Default::default() }).unwrap_err();
        assert!(err.contains(src) && err.contains("under no export root"), "{err}");
    }

    #[test]
    fn blobs_and_toml_are_hashed_inputs_but_not_file_rows() {
        let e = run(&["/a/engine/data/sine.bin", "/a/games/demo/map.toml", "/b/generated/sound/x.bin", "/a/art/pal/s.bin", "/a/engine/vdp.emp"]).unwrap();
        assert_eq!(e.files.len(), 1, "only the .emp is a row");
        let got: Vec<(&str, &str)> = e.inputs.iter().map(|i| (i.root.as_str(), i.path.as_str())).collect();
        assert_eq!(
            got,
            vec![
                ("aeon", "art/pal/s.bin"),
                ("engine", "data/sine.bin"),
                ("engine", "vdp.emp"),
                ("game", "map.toml"),
                ("generated", "sound/x.bin"),
            ]
        );
    }

    #[test]
    fn an_external_data_file_is_hashed_by_name_only() {
        let e = run(&["/tmp/x/anchors.toml"]).unwrap();
        assert_eq!(e.inputs, vec![InputRead { root: "external".into(), path: "anchors.toml".into(), sha256: [9; 32] }]);
        assert!(e.files.is_empty());
    }

    #[test]
    fn the_tool_executable_is_identity_by_name_only() {
        let mut tool = read("/a/tools/convsym");
        tool.tool = true;
        let snap = Snapshot { reads: vec![tool], ..Default::default() };
        let e = with(&in_tree(), |r| export_files(r, &snap)).unwrap();
        assert_eq!(e.tool_exes, vec![ToolExe { name: "convsym".into(), sha256: [9; 32] }]);
        assert!(e.inputs.is_empty() && e.files.is_empty());
    }

    /// A source file under no root is refused naming it and every root with its
    /// directory, inside the engine's checkout (another game's file, a tool fixture) or
    /// out of it.
    #[test]
    fn a_source_read_outside_every_root_is_an_error_naming_the_path_and_the_roots() {
        for p in ["/a/other/x.emp", "/a/games/sonic4/config/game.emp", "/a/tools/fixtures/anchor_sweep/f.emp", "/elsewhere/y.asm"] {
            let e = rows(&["/a/engine/ok.emp", p]).unwrap_err();
            for want in [p, "`engine` = /a/engine", "`game` = /a/games/demo", "`generated` = /b/generated", "`sigil` = /s"] {
                assert!(e.contains(want), "the refusal does not name `{want}`: {e}");
            }
        }
    }

    #[test]
    fn a_read_of_the_generated_root_is_placed_at_its_mount() {
        let root = GeneratedRoot { engine: PathBuf::from("/a/engine"), dir: PathBuf::from("/b/generated") };
        let mut gen = read("/b/generated/sound/z80_sound_blob.bin");
        gen.generated = true;
        let snap = Snapshot { reads: vec![gen, read("/a/engine/vdp.emp")], ..Default::default() };
        let mounted = mount_generated(&snap, &root).unwrap();
        let paths: Vec<&Path> = mounted.reads.iter().map(|r| r.path.as_path()).collect();
        assert_eq!(paths, [Path::new("/a/engine/sound/generated/z80_sound_blob.bin"), Path::new("/a/engine/vdp.emp")]);
        assert!(mounted.reads[0].generated, "the generated mark survives the rename");
        let want = vec![("engine", "vdp.emp"), ("generated", "sound/z80_sound_blob.bin")];
        let e = with(&in_tree(), |r| export_files(r, &mounted)).unwrap();
        let got: Vec<(&str, &str)> = e.inputs.iter().map(|i| (i.root.as_str(), i.path.as_str())).collect();
        assert_eq!(got, want);
        // Unmounted, the read is under the build's generated root directly: the same row.
        let raw = with(&in_tree(), |r| export_files(r, &snap)).unwrap();
        let got: Vec<(&str, &str)> = raw.inputs.iter().map(|i| (i.root.as_str(), i.path.as_str())).collect();
        assert_eq!(got, want);
    }

    #[test]
    fn a_tree_read_at_the_mount_beside_its_generated_file_is_refused() {
        let root = GeneratedRoot { engine: PathBuf::from("/a/engine"), dir: PathBuf::from("/b/generated") };
        let snap = Snapshot {
            reads: vec![read("/b/generated/sound/x.bin"), read("/a/engine/sound/generated/x.bin")],
            ..Default::default()
        };
        let e = mount_generated(&snap, &root).unwrap_err();
        assert!(e.contains("/a/engine/sound/generated/x.bin") && e.contains("/b/generated/sound/x.bin"), "{e}");
        let flat = Snapshot { reads: vec![read("/b/generated/x.bin")], ..Default::default() };
        let e = mount_generated(&flat, &root).unwrap_err();
        assert!(e.contains("directly in its generated root"), "{e}");
    }

    #[test]
    fn a_conflict_is_an_error() {
        let snap = Snapshot {
            conflicts: vec![sigil_span::read_set::Conflict::Read { path: "/a/engine/m.emp".into(), observations: vec![(1, 2), (3, 4)] }],
            ..Default::default()
        };
        assert!(with(&in_tree(), |r| export_files(r, &snap)).is_err());
    }

    #[test]
    fn the_row_key_is_the_files_placement() {
        let roots = in_tree();
        with(&roots, |r| {
            assert_eq!(source_row_key(r, "/a/games/demo/config/game.emp").unwrap(), (RootName::Game, "config/game.emp".into()));
            assert_eq!(source_row_key(r, "/b/generated/sound/d.z80").unwrap(), (RootName::Generated, "sound/d.z80".into()));
            assert!(source_row_key(r, "/a/art/x.bin").is_err());
            assert!(source_row_key(r, "/a/other/x.emp").unwrap_err().contains("`game` = /a/games/demo"));
        });
    }
}
