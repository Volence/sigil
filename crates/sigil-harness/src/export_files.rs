//! The build export's `files` table, derived from the read-set snapshot (G0a spec §4.6).
//!
//! The recorder (`sigil_span::read_set`) is the one source of truth for what a build
//! read, so this table is a PROJECTION of it and never a second walk of the tree. Each
//! read is placed in one of exactly four roots, `engine`, `game`, `generated` and
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
//! # What is a row, and what is not
//!
//! The schema's `lang` is `emp`, `asm` or `z80`, so a row is a SOURCE file: a read whose
//! extension is `.emp`, `.asm` or `.z80`. The rest of what a build reads (art and sound
//! blobs, `map.toml`, the `convsym` executable) is DATA or a TOOL, has no `lang`, and is
//! not a row; a change to one still moves the ROM, so the ROM hash binds it, but it
//! does not move `inputsSha256` or show as a stale source. That is a real gap in the
//! 1.0 shape, booked in `docs/QUEUE.md`.
//!
//! # BLOCKED: source files under `tools/`
//!
//! Four `.emp` fixtures under the aeon `tools/` directory (`tools/fixtures/anchor_sweep/`)
//! are read by every build (the module scan reads every `*.emp` in the tree) and fit no
//! one of the four roots. The standing rule is that a fifth root is not invented, and the
//! controller has not ruled on these. Until it does, reads under `tools/` are NOT rows.
//! This is the one named hole in the table; it is a constant ([`UNROOTED_AEON_DIRS`]) so
//! it is visible, and anything else that fits no root is an error that names the path.

use sigil_link::export::{FileRecord, Lang, RootName};
use sigil_span::read_set::{FileRead, Snapshot};
use std::path::{Component, Path};

/// Aeon top-level directories whose source files fit none of the four roots, pending a
/// controller ruling. See the module doc.
pub const UNROOTED_AEON_DIRS: &[&str] = &["tools"];

/// The row language of `path`, or `None` for data and tools (no row).
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

/// Place one read: `Ok(Some(..))` a row, `Ok(None)` not a row, `Err` a source file that
/// fits no root.
fn place(aeon: &Path, sigil: &Path, r: &FileRead) -> Result<Option<(RootName, String, Lang)>, String> {
    let Some(lang) = lang_of(&r.path).filter(|_| !r.tool) else {
        return Ok(None);
    };
    if let Ok(rel) = r.path.strip_prefix(aeon) {
        let mut comps = rel.components();
        let first = comps.next().and_then(|c| c.as_os_str().to_str()).unwrap_or("");
        let (root, under) = match first {
            "engine" => {
                let under = rel.strip_prefix("engine").expect("first component");
                let generated = ["sound", "debug"].iter().find(|area| under.starts_with(Path::new(area).join("generated")));
                match generated {
                    Some(area) => {
                        let tail = under.strip_prefix(Path::new(area).join("generated")).expect("checked");
                        (RootName::Generated, Path::new(area).join(tail))
                    }
                    None => (RootName::Engine, under.to_path_buf()),
                }
            }
            "games" => (RootName::Game, rel.strip_prefix("games").expect("first component").to_path_buf()),
            d if UNROOTED_AEON_DIRS.contains(&d) => return Ok(None),
            _ => {
                return Err(format!(
                    "{} is a {} source file under the aeon tree outside engine/ and games/, and no export root holds it",
                    r.path.display(),
                    lang.as_str()
                ))
            }
        };
        return Ok(Some((root, rel_text(&under, &r.path)?, lang)));
    }
    if let Ok(rel) = r.path.strip_prefix(sigil) {
        return Ok(Some((RootName::Sigil, rel_text(rel, &r.path)?, lang)));
    }
    Err(format!(
        "{} is a {} source file the build read from outside the aeon tree {} and the sigil checkout {}, so no export root holds it",
        r.path.display(),
        lang.as_str(),
        aeon.display(),
        sigil.display()
    ))
}

/// The `files` table for `snapshot`, sorted by `(root, path)`. `aeon` and `sigil` are the
/// canonical roots. A snapshot with a conflict (a file read with two contents) cannot be
/// stated as one row per file and is an error, as it is for the `.lst` digest.
pub fn export_files(aeon: &Path, sigil: &Path, snapshot: &Snapshot) -> Result<Vec<FileRecord>, String> {
    if !snapshot.conflicts.is_empty() {
        let lines: Vec<String> = snapshot.conflicts.iter().map(|c| format!("  {c}")).collect();
        return Err(format!("the build's read set cannot be exported as one row per file:\n{}", lines.join("\n")));
    }
    let mut out = Vec::new();
    for r in &snapshot.reads {
        if let Some((root, path, lang)) = place(aeon, sigil, r)? {
            out.push(FileRecord { root, path, lang, sha256: r.sha256, size: r.size });
        }
    }
    out.sort_by(|a, b| (a.root, &a.path).cmp(&(b.root, &b.path)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn read(path: &str) -> FileRead {
        FileRead { path: PathBuf::from(path), crc: 0, size: 3, sha256: [9; 32], generated: false, tool: false }
    }

    fn rows(paths: &[&str]) -> Result<Vec<(RootName, String)>, String> {
        let snap = Snapshot { reads: paths.iter().map(|p| read(p)).collect(), ..Default::default() };
        export_files(Path::new("/a"), Path::new("/s"), &snap).map(|v| v.into_iter().map(|f| (f.root, f.path)).collect())
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
    fn data_and_tools_are_not_rows_and_the_unrooted_dir_is_the_one_named_hole() {
        let mut tool = read("/a/engine/x.emp");
        tool.tool = true;
        let snap = Snapshot {
            reads: vec![read("/a/art/x.bin"), read("/a/games/demo/map.toml"), read("/a/tools/fixtures/f.emp"), tool],
            ..Default::default()
        };
        assert!(export_files(Path::new("/a"), Path::new("/s"), &snap).unwrap().is_empty());
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
