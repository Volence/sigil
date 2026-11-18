//! The build export's `files` table and the inputs `inputsSha256` hashes, both derived
//! from the read-set snapshot (G0a spec §4.2, §4.6).
//!
//! The recorder (`sigil_span::read_set`) is the one source of truth for what a build
//! read, so everything here is a PROJECTION of it and never a second walk of the tree.
//! Each read is placed in one of exactly four roots, `engine`, `game`, `generated` and
//! `sigil`, with a path relative to that root:
//!
//! | read (under the aeon root unless stated)  | root        | path is relative to        |
//! |-------------------------------------------|-------------|----------------------------|
//! | `engine/sound/generated/...`              | `generated` | `engine/` with `generated/` dropped, so `sound/...` |
//! | `engine/debug/generated/...`              | `generated` | likewise, so `debug/...`   |
//! | `engine/...`                              | `engine`    | `engine/`                  |
//! | `games/<game>/...`                        | `game`      | `games/`, so `<game>/...`  |
//! | under the sigil source checkout           | `sigil`     | the checkout               |
//!
//! `game` paths keep the game's directory name because one build reads more than one
//! game's modules (the module scan reads every `*.emp`), and two games each have a
//! `config/game.emp`: relative to a single game's directory those two would collide.
//!
//! # Two outputs, two scopes
//!
//!  * `files` (the schema's table): SOURCE files only, a read whose extension is
//!    `.emp`, `.asm` or `.z80`, because `lang` is a closed enum of exactly those.
//!  * `inputs` (what `inputsSha256` hashes): EVERY read under a root, sources, art and
//!    sound blobs and `map.toml` alike, by root-relative path and SHA-256. A changed
//!    blob therefore moves `inputsSha256` although no file row changes. A read under
//!    the aeon tree but under no root (the `art/` directory) is still an input; it is
//!    hashed under the hash-only root name `aeon` and never reaches the schema.
//!  * A DATA file read from outside both the aeon tree and the sigil checkout (the
//!    `--anchor-overlay` file) is an input too: hashed under the hash-only root
//!    `external` by file name alone. A SOURCE file from there fits no root and is an
//!    error naming the path.
//!  * `tool_exes`: executables the build ran (`convsym`) are tool identity, recorded
//!    by file name and hash only, wherever they live.
//!
//! # The one named hole: `tools/fixtures`
//!
//! Four `.emp` fixtures under `tools/fixtures/anchor_sweep/` are read by every build,
//! only because the module scan reads every `*.emp` in the tree. They are not part of
//! the program and fit none of the four roots. Controller ruling (G0a Task 3): this is
//! a scan-scope defect, not a missing root, so reads under exactly `tools/fixtures` are
//! left out of both outputs ([`UNROOTED_AEON_DIRS`]), and G0b's two-root resolution
//! deletes the hole by scoping the scan to the engine and game roots (queue row
//! `G0A-SCAN-READS-TOOL-FIXTURES`). Any other read under `tools/` is an error naming
//! the path, and so is any read that fits no root and is not an aeon-tree data file.

use sigil_link::export::{FileRecord, InputRead, Lang, RootName, ToolExe};
use sigil_span::read_set::{FileRead, Snapshot};
use std::path::{Component, Path, PathBuf};

/// Aeon-relative directories whose reads are left out of the export, pending G0b. See
/// the module doc.
pub const UNROOTED_AEON_DIRS: &[&str] = &["tools/fixtures"];

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
    /// Under the aeon tree, under no root, a data file: hashed as `aeon`.
    AeonData(String),
    /// Outside the aeon tree and the sigil checkout, a data file the caller handed the
    /// build (the `--anchor-overlay` file): hashed as `external` by FILE NAME only, since
    /// its directory says where the build ran.
    External(String),
    /// Deliberately left out.
    Skip,
}

fn place(aeon: &Path, sigil: &Path, r: &FileRead) -> Result<Placed, String> {
    let lang = lang_of(&r.path);
    if let Ok(rel) = r.path.strip_prefix(aeon) {
        let first = rel.components().next().and_then(|c| c.as_os_str().to_str()).unwrap_or("");
        let (root, under): (RootName, PathBuf) = match first {
            "engine" => {
                let under = rel.strip_prefix("engine").expect("first component");
                let generated =
                    ["sound", "debug"].iter().find(|area| under.starts_with(Path::new(area).join("generated")));
                match generated {
                    Some(area) => {
                        let tail = under.strip_prefix(Path::new(area).join("generated")).expect("checked");
                        (RootName::Generated, Path::new(area).join(tail))
                    }
                    None => (RootName::Engine, under.to_path_buf()),
                }
            }
            "games" => (RootName::Game, rel.strip_prefix("games").expect("first component").to_path_buf()),
            _ => {
                if UNROOTED_AEON_DIRS.iter().any(|d| rel.starts_with(d)) {
                    return Ok(Placed::Skip);
                }
                if first == "tools" || lang.is_some() {
                    return Err(format!(
                        "{} is read by the build from the aeon tree outside engine/ and games/, and no export root holds it",
                        r.path.display()
                    ));
                }
                return Ok(Placed::AeonData(rel_text(rel, &r.path)?));
            }
        };
        return Ok(Placed::Rooted(root, rel_text(&under, &r.path)?));
    }
    if let Ok(rel) = r.path.strip_prefix(sigil) {
        return Ok(Placed::Rooted(RootName::Sigil, rel_text(rel, &r.path)?));
    }
    if lang.is_none() {
        let name = r
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("{} has no UTF-8 file name", r.path.display()))?;
        return Ok(Placed::External(name.to_string()));
    }
    Err(format!(
        "{} is a source file the build read from outside the aeon tree {} and the sigil checkout {}, so no export root holds it",
        r.path.display(),
        aeon.display(),
        sigil.display()
    ))
}

/// The export's file rows, hashed inputs and helper executables for `snapshot`. `aeon`
/// and `sigil` are the canonical roots. A snapshot with a conflict (a file read with two
/// contents) cannot be stated as one row per file and is an error, as it is for the
/// `.lst` digest.
pub fn export_files(aeon: &Path, sigil: &Path, snapshot: &Snapshot) -> Result<ExportInputs, String> {
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
        match place(aeon, sigil, r)? {
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
            Placed::Skip => {}
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

    fn run(paths: &[&str]) -> Result<ExportInputs, String> {
        let snap = Snapshot { reads: paths.iter().map(|p| read(p)).collect(), ..Default::default() };
        export_files(Path::new("/a"), Path::new("/s"), &snap)
    }

    fn rows(paths: &[&str]) -> Result<Vec<(RootName, String)>, String> {
        run(paths).map(|e| e.files.into_iter().map(|f| (f.root, f.path)).collect())
    }

    #[test]
    fn each_read_lands_in_its_root_with_a_relative_path() {
        let got = rows(&[
            "/a/engine/vdp.emp",
            "/a/engine/sound/generated/seq.z80",
            "/a/engine/debug/generated/compression_vectors.emp",
            "/a/games/demo/config/game.emp",
            "/a/games/sonic4/config/game.emp",
            "/a/engine/debug/debugger.asm",
            "/s/crates/prelude.emp",
        ])
        .unwrap();
        assert_eq!(
            got,
            vec![
                (RootName::Engine, "debug/debugger.asm".to_string()),
                (RootName::Engine, "vdp.emp".to_string()),
                (RootName::Game, "demo/config/game.emp".to_string()),
                (RootName::Game, "sonic4/config/game.emp".to_string()),
                (RootName::Generated, "debug/compression_vectors.emp".to_string()),
                (RootName::Generated, "sound/seq.z80".to_string()),
                (RootName::Sigil, "crates/prelude.emp".to_string()),
            ]
        );
    }

    #[test]
    fn blobs_and_toml_are_hashed_inputs_but_not_file_rows() {
        let e = run(&["/a/engine/data/sine.bin", "/a/games/demo/map.toml", "/a/engine/sound/generated/x.bin", "/a/art/pal/s.bin", "/a/engine/vdp.emp"]).unwrap();
        assert_eq!(e.files.len(), 1, "only the .emp is a row");
        let got: Vec<(&str, &str)> = e.inputs.iter().map(|i| (i.root.as_str(), i.path.as_str())).collect();
        assert_eq!(
            got,
            vec![
                ("aeon", "art/pal/s.bin"),
                ("engine", "data/sine.bin"),
                ("engine", "vdp.emp"),
                ("game", "demo/map.toml"),
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
        let e = export_files(Path::new("/a"), Path::new("/s"), &snap).unwrap();
        assert_eq!(e.tool_exes, vec![ToolExe { name: "convsym".into(), sha256: [9; 32] }]);
        assert!(e.inputs.is_empty() && e.files.is_empty());
    }

    #[test]
    fn tools_fixtures_is_the_one_named_hole() {
        let e = run(&["/a/tools/fixtures/anchor_sweep/f.emp", "/a/tools/fixtures/x.bin"]).unwrap();
        assert_eq!(e, ExportInputs::default());
    }

    #[test]
    fn any_other_read_under_tools_is_an_error_naming_the_path() {
        for p in ["/a/tools/other.emp", "/a/tools/gen.bin", "/a/tools/fixturesque/x.emp"] {
            let e = run(&[p]).unwrap_err();
            assert!(e.contains(p), "{e}");
        }
    }

    #[test]
    fn a_source_read_outside_every_root_is_an_error_naming_the_path() {
        let e = rows(&["/a/engine/ok.emp", "/a/other/x.emp"]).unwrap_err();
        assert!(e.contains("/a/other/x.emp"), "{e}");
        let e = rows(&["/elsewhere/y.asm"]).unwrap_err();
        assert!(e.contains("/elsewhere/y.asm"), "{e}");
    }

    #[test]
    fn a_conflict_is_an_error() {
        let snap = Snapshot {
            conflicts: vec![sigil_span::read_set::Conflict::Read { path: "/a/engine/m.emp".into(), observations: vec![(1, 2), (3, 4)] }],
            ..Default::default()
        };
        assert!(export_files(Path::new("/a"), Path::new("/s"), &snap).is_err());
    }
}
