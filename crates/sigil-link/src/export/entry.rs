//! The exporter's entry: what a driver hands over after a build (`ExportInputs`),
//! and `build_export`, which turns it plus the ROM's identity into a `BuildExport`.
//! Driver-independent on purpose: the driver supplies placed sections, a span
//! locator and its side tables; nothing here knows a front end.

use super::lines::{check_image_coverage, merge_adjacent, walk_lines, NoSourceRange};
use super::locate::{FileRef, SpanLocator};
use super::placed::Z80Window;
use super::{
    BuildExport, ConstRecord, ExportError, FileRecord, LayoutRecord, LineRow, LineSection, LineTable, RomRecord,
    RootName, RowKind, ShapeRecord, Space, SpaceRecord, SymbolRecord, ToolRecord,
};
use sigil_ir::Section;
use std::collections::HashMap;

/// The Z80 sound driver as the export states it (spec 4.4, "The Z80"). Seam 1 links
/// the driver on its own and the ROM embeds the result as ONE data fragment, so the
/// placed sections know the blob only as bytes on its embedding line. The driver (the
/// harness) hands over what seam 1 knew in process: the driver's own symbols and line
/// rows, in `space: z80`, already rebased to the ROM. [`build_export`] puts the rows in
/// place of the embedding line's row over exactly the blob's bytes, so the completeness
/// check covers them like any other row.
///
/// When the driver becomes ordinary sections (G0c), its rows come from the same walk and
/// this bridge is deleted.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Z80Bridge {
    /// Where the ROM embeds the blob: the load address of its first byte.
    pub blob_lma: u32,
    /// The driver's source files as seam 1 opened them. A bridge row's `file` is an
    /// index into this (not into the main locator), mapped to its `files` row by
    /// [`build_export`] through the same [`FileKey`].
    pub files: Vec<FileRef>,
    /// The driver's symbols: `space: z80`, `vma` the Z80 address, `lma` the ROM byte.
    pub symbols: Vec<SymbolRecord>,
    /// The driver's line sections: `space: z80`, rows rebased to ROM load addresses,
    /// covering `[blob_lma, blob_lma + blob length)` exactly once.
    pub sections: Vec<LineSection>,
}

/// Tables the front ends know and the placed sections do not. Empty fields are legal
/// until the tasks that fill them land.
#[derive(Clone, Debug, Default)]
pub struct SideTables {
    pub symbols: Vec<SymbolRecord>,
    pub layouts: Vec<LayoutRecord>,
    pub consts: Vec<ConstRecord>,
    pub z80: Option<Z80Bridge>,
    /// The fragments the front ends emitted as instructions, by section name and
    /// fragment range (`sigil_ir::Module::code`): the line table's `code` rows.
    pub code: Vec<(String, std::ops::Range<usize>)>,
}

/// Everything a build keeps for its export, taken before the sections are consumed.
pub struct ExportInputs {
    /// The placed, post-resolve sections the ROM was linked from.
    pub sections: Vec<Section>,
    /// Maps those sections' spans to source lines.
    pub locator: Box<dyn SpanLocator + Send>,
    pub side: SideTables,
    /// The shape's Z80 bank windows, from its memory map: they decide the space of a
    /// section phased into one ([`super::placed::section_space`]).
    pub z80_windows: Vec<Z80Window>,
}

/// The root-relative key of a source file, given the name its front end opened it
/// under: the `(root, path)` its `files` row carries. Supplied by the driver, which
/// alone knows where the roots are; `Err` says why the file fits no root.
pub type FileKey<'a> = &'a dyn Fn(&str) -> Result<(RootName, String), String>;

/// Assemble the export for `rom` (the bytes as installed).
///
/// `post_link` declares the image bytes written after the link, by no source line:
/// the deb2 appendix and the folded header checksum, as the driver measured them on
/// this build. Section fill between placed sections is derived by the walk.
/// `file_key` maps a located file to its `files` row (see [`FileKey`]); a located file
/// with no row is an error naming it, never dropped and never row 0.
///
/// `files` is sorted here by `(root, path)`, the order the JSON writer states it in,
/// so a row's `file` index means the same row in the model and in the file.
///
/// `inputs_sha256` is left zero: it is a hash of the build's read set, which the
/// driver owns and this entry does not see, so the caller sets it before rendering.
pub fn build_export(
    inputs: &ExportInputs,
    rom: &RomRecord,
    tool: ToolRecord,
    shape: ShapeRecord,
    mut files: Vec<FileRecord>,
    post_link: &[NoSourceRange],
    file_key: FileKey<'_>,
) -> Result<BuildExport, ExportError> {
    files.sort_by(|a, b| (a.root, &a.path).cmp(&(b.root, &b.path)));
    for w in files.windows(2) {
        if (w[0].root, &w[0].path) == (w[1].root, &w[1].path) {
            return Err(ExportError::DuplicateFile { root: w[0].root.as_str(), path: w[0].path.clone() });
        }
    }
    let mut lines =
        walk_lines(&inputs.sections, &*inputs.locator, post_link, &inputs.side.code, &inputs.z80_windows)?;
    map_files(&mut lines, &|id| Some(inputs.locator.file(id)), &files, file_key)?;
    let mut symbols = inputs.side.symbols.clone();
    if let Some(z) = &inputs.side.z80 {
        let mut driver = LineTable { sections: z.sections.clone() };
        map_files(&mut driver, &|id| z.files.get(id as usize), &files, file_key)?;
        splice_z80(&mut lines, driver, z.blob_lma)?;
        symbols.extend(z.symbols.iter().cloned());
    }
    merge_adjacent(&mut lines);
    check_image_coverage(&lines, rom.size)?;
    Ok(BuildExport {
        rom: rom.clone(),
        inputs_sha256: [0; 32],
        tool,
        shape,
        spaces: vec![
            SpaceRecord { id: Space::M68k, addr_bits: 24 },
            SpaceRecord { id: Space::Z80, addr_bits: 16 },
        ],
        files,
        symbols,
        layouts: inputs.side.layouts.clone(),
        consts: inputs.side.consts.clone(),
        lines,
    })
}

/// Put the Z80 bridge's rows (`driver`, files already mapped) in place of the embedding
/// line's row over the blob's bytes.
///
/// Refused, never patched over: driver rows that are not `space: z80`, that do not tile
/// one contiguous range from `blob_lma`, or a range the placed sections do not cover
/// with `data` rows of ONE source line lying wholly inside it (the embedding line). Any
/// of those means the bridge and the ROM disagree about where the blob is.
fn splice_z80(lines: &mut LineTable, driver: LineTable, blob_lma: u32) -> Result<(), ExportError> {
    let bad = |m: String| Err(ExportError::Z80Bridge(m));
    let mut rows: Vec<&LineRow> = Vec::new();
    for s in &driver.sections {
        if s.space != Space::Z80 {
            return bad(format!("driver section `{}` is stated in {}, not z80", s.name, s.space.as_str()));
        }
        rows.extend(s.rows.iter());
    }
    rows.sort_by_key(|r| r.lma);
    let mut end = blob_lma as u64;
    for r in &rows {
        if r.lma as u64 != end {
            return bad(format!("driver rows do not tile the blob: a row starts at ${:X}, the previous ends at ${end:X}", r.lma));
        }
        end += r.len as u64;
    }
    if end == blob_lma as u64 {
        return bad("the bridge carries no driver rows".into());
    }
    let (start, end) = (blob_lma as u64, end);
    let mut embedding: Option<(u32, u32)> = None;
    let mut covered = 0u64;
    for s in &mut lines.sections {
        let mut kept = Vec::with_capacity(s.rows.len());
        for r in s.rows.drain(..) {
            let (a, b) = (r.lma as u64, r.lma as u64 + r.len as u64);
            if b <= start || a >= end {
                kept.push(r);
                continue;
            }
            if a < start || b > end {
                return bad(format!(
                    "the row [${a:X}, ${b:X}) of section `{}` crosses the blob's edge [${start:X}, ${end:X})",
                    s.name
                ));
            }
            if r.kind != RowKind::Data {
                return bad(format!("the blob's bytes at ${a:X} are a `{}` row, not embedded data", r.kind.as_str()));
            }
            match embedding {
                None => embedding = Some((r.file, r.line)),
                Some(at) if at != (r.file, r.line) => {
                    return bad(format!("the blob's bytes come from two source lines ({at:?} and {:?})", (r.file, r.line)))
                }
                Some(_) => {}
            }
            covered += r.len as u64;
        }
        s.rows = kept;
    }
    if covered != end - start {
        return bad(format!(
            "the embedding line covers {covered} of the blob's {} bytes at ${start:X}",
            end - start
        ));
    }
    lines.sections.retain(|s| !s.rows.is_empty());
    lines.sections.extend(driver.sections);
    Ok(())
}

/// Rewrite every source row's `file` from the locator's file id to its index in the
/// sorted `files`.
///
/// `file_of` names the file a row's id stands for (the main locator's, or the Z80
/// bridge's own list); an id it does not know is an internal error, never row 0.
fn map_files<'f>(
    lines: &mut LineTable,
    file_of: &dyn Fn(u32) -> Option<&'f FileRef>,
    files: &[FileRecord],
    file_key: FileKey<'_>,
) -> Result<(), ExportError> {
    let mut rows: HashMap<u32, u32> = HashMap::new();
    for row in lines.sections.iter_mut().flat_map(|s| s.rows.iter_mut()) {
        if !matches!(row.kind, RowKind::Code | RowKind::Data) {
            continue;
        }
        if let Some(&r) = rows.get(&row.file) {
            row.file = r;
            continue;
        }
        let path = &file_of(row.file)
            .ok_or_else(|| ExportError::Internal(format!("a line row names file id {}, which no file list issued", row.file)))?
            .path;
        let (root, rel) =
            file_key(path).map_err(|why| ExportError::FileNotInTable { path: path.clone(), why })?;
        let r = files
            .binary_search_by(|f| (f.root, f.path.as_str()).cmp(&(root, rel.as_str())))
            .map_err(|_| ExportError::FileNotInTable {
                path: path.clone(),
                why: format!("its root-relative name `{}/{rel}` is not a row of the read set", root.as_str()),
            })? as u32;
        rows.insert(row.file, r);
        row.file = r;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::locate::{FileRef, SourcePos};
    use super::super::{Lang, NoSourceKind};
    use super::*;
    use sigil_ir::{AddressSpace, Cpu, DataFragment, Fragment, SectionPlacement};
    use sigil_span::{SourceId, Span};

    /// Source id = locator file id, `start` = line.
    struct Loc(Vec<FileRef>);
    impl SpanLocator for Loc {
        fn outermost(&self, span: Span) -> Option<SourcePos> {
            Some(SourcePos { file: span.source.0, line: span.start })
        }
        fn file(&self, id: u32) -> &FileRef {
            &self.0[id as usize]
        }
    }

    fn data(n: usize, file: u32, line: u32) -> Fragment {
        Fragment::Data(DataFragment {
            bytes: vec![0; n],
            fixups: vec![],
            span: Span { source: SourceId(file), start: line, end: line },
        })
    }

    fn inputs(paths: &[&str], fragments: Vec<Fragment>) -> ExportInputs {
        let sec = Section {
            name: "s".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            space: AddressSpace::Image,
            labels: vec![],
            fragments,
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![],
        };
        ExportInputs {
            sections: vec![sec],
            locator: Box::new(Loc(paths.iter().map(|p| FileRef { path: p.to_string() }).collect())),
            side: SideTables::default(),
            z80_windows: vec![],
        }
    }

    fn row(root: RootName, path: &str) -> FileRecord {
        FileRecord { root, path: path.into(), lang: Lang::Emp, sha256: [0; 32], size: 0 }
    }

    /// `/abs/engine/x.emp` -> `(engine, x.emp)`; anything else fits no root.
    fn key(p: &str) -> Result<(RootName, String), String> {
        if let Some(r) = p.strip_prefix("/abs/engine/") {
            return Ok((RootName::Engine, r.into()));
        }
        if let Some(r) = p.strip_prefix("/abs/games/") {
            return Ok((RootName::Game, r.into()));
        }
        Err(format!("{p} fits no root"))
    }

    fn build(i: &ExportInputs, size: u64, files: Vec<FileRecord>, post: &[NoSourceRange]) -> Result<BuildExport, ExportError> {
        let rom = RomRecord { path: "r.bin".into(), size, sha256: [0; 32] };
        let tool = ToolRecord { name: "sigil".into(), version: "0".into(), revision: "r".into(), tree: "clean".into() };
        let shape = ShapeRecord { game: "g".into(), shape: "s".into(), defines: vec![] };
        build_export(i, &rom, tool, shape, files, post, &key)
    }

    /// The locator's ids are not the files rows: a row's `file` indexes the SORTED files.
    #[test]
    fn a_rows_file_is_its_index_in_the_sorted_files() {
        let i = inputs(&["/abs/games/s4/a.emp", "/abs/engine/z.emp"], vec![data(2, 0, 3), data(2, 1, 4)]);
        // Given unsorted: game before engine. Sorted: engine/z.emp (0), engine/zz.emp (1), game/s4/a.emp (2).
        let files = vec![row(RootName::Game, "s4/a.emp"), row(RootName::Engine, "zz.emp"), row(RootName::Engine, "z.emp")];
        let e = build(&i, 4, files, &[]).unwrap();
        assert_eq!(e.files[0].path, "z.emp");
        let r = &e.lines.sections[0].rows;
        assert_eq!((r[0].file, r[0].line), (2, 3));
        assert_eq!((r[1].file, r[1].line), (0, 4));
    }

    #[test]
    fn a_located_file_with_no_row_is_an_error_naming_it() {
        let i = inputs(&["/abs/engine/z.emp", "/abs/engine/gone.emp"], vec![data(2, 0, 3), data(2, 1, 4)]);
        let err = build(&i, 4, vec![row(RootName::Engine, "z.emp")], &[]).unwrap_err();
        assert!(
            matches!(&err, ExportError::FileNotInTable { path, .. } if path == "/abs/engine/gone.emp"),
            "{err:?}"
        );
        let i = inputs(&["/elsewhere/q.emp"], vec![data(2, 0, 3)]);
        let err = build(&i, 2, vec![row(RootName::Engine, "z.emp")], &[]).unwrap_err();
        assert!(matches!(&err, ExportError::FileNotInTable { path, .. } if path == "/elsewhere/q.emp"), "{err:?}");
    }

    /// A Z80 bridge whose blob is embedded at `blob_lma`: one section `drv` of `rows`
    /// `(vma, lma, len, line)`, every row in file 0 (`/abs/engine/sound/drv.emp`).
    fn bridge(blob_lma: u32, space: Space, rows: &[(u32, u32, u32, u32)]) -> Z80Bridge {
        let rows = rows
            .iter()
            .map(|&(vma, lma, len, line)| LineRow { vma, lma, len, file: 0, line, kind: RowKind::Code })
            .collect();
        Z80Bridge {
            blob_lma,
            files: vec![FileRef { path: "/abs/engine/sound/drv.emp".into() }],
            symbols: vec![SymbolRecord {
                name: "Drv".into(),
                demangled: None,
                space: Some(Space::Z80),
                vma: 0,
                lma: Some(blob_lma),
                kind: super::super::SymbolKind::Proc,
                size: Some(1),
                module: Some("engine.drv".into()),
                public: true,
            }],
            sections: vec![LineSection { name: "drv".into(), space, rows }],
        }
    }

    /// `(a.emp line 3: 2 B) (a.emp line 7: the 4-byte blob) (a.emp line 9: 2 B)`.
    fn embedding() -> ExportInputs {
        inputs(&["/abs/engine/a.emp"], vec![data(2, 0, 3), data(4, 0, 7), data(2, 0, 9)])
    }

    fn files() -> Vec<FileRecord> {
        vec![row(RootName::Engine, "sound/drv.emp"), row(RootName::Engine, "a.emp")]
    }

    #[test]
    fn the_z80_bridge_replaces_the_embedding_row_over_the_blob() {
        let mut i = embedding();
        i.side.z80 = Some(bridge(2, Space::Z80, &[(0, 2, 3, 5), (3, 5, 1, 6)]));
        let e = build(&i, 8, files(), &[]).unwrap();
        let s = e.lines.sections.iter().find(|s| s.name == "s").unwrap();
        assert_eq!(s.rows.iter().map(|r| (r.lma, r.len, r.line)).collect::<Vec<_>>(), vec![(0, 2, 3), (6, 2, 9)]);
        let d = e.lines.sections.iter().find(|s| s.name == "drv").unwrap();
        assert_eq!(d.space, Space::Z80);
        // Files are sorted: `a.emp` (0), `sound/drv.emp` (1); the bridge's file 0 is row 1.
        assert_eq!(
            d.rows.iter().map(|r| (r.vma, r.lma, r.len, r.file, r.line)).collect::<Vec<_>>(),
            vec![(0, 2, 3, 1, 5), (3, 5, 1, 1, 6)]
        );
        assert!(e.symbols.iter().any(|s| s.name == "Drv" && s.space == Some(Space::Z80) && s.lma == Some(2)));
    }

    #[test]
    fn a_bridge_that_disagrees_with_the_embedding_is_refused() {
        let refused = |z: Z80Bridge| {
            let mut i = embedding();
            i.side.z80 = Some(z);
            match build(&i, 8, files(), &[]) {
                Err(ExportError::Z80Bridge(m)) => m,
                other => panic!("not refused: {other:?}"),
            }
        };
        // Wider than the embedding line: its bytes come from two lines.
        assert!(refused(bridge(0, Space::Z80, &[(0, 0, 6, 5)])).contains("two source lines"));
        // Starting inside a row: that row crosses the blob's edge.
        assert!(refused(bridge(1, Space::Z80, &[(0, 1, 2, 5)])).contains("crosses"));
        // Shorter than the embedding line: the embedding row runs past the blob's end.
        assert!(refused(bridge(2, Space::Z80, &[(0, 2, 3, 5)])).contains("crosses"));
        // Rows that leave a hole.
        assert!(refused(bridge(2, Space::Z80, &[(0, 2, 1, 5), (2, 4, 2, 6)])).contains("do not tile"));
        // Rows in the wrong space.
        assert!(refused(bridge(2, Space::M68k, &[(0, 2, 4, 5)])).contains("not z80"));
    }

    #[test]
    fn the_whole_image_must_be_covered() {
        let i = inputs(&["/abs/engine/z.emp"], vec![data(4, 0, 3)]);
        let files = vec![row(RootName::Engine, "z.emp")];
        assert_eq!(
            build(&i, 6, files.clone(), &[]).unwrap_err(),
            ExportError::Uncovered { section: crate::export::IMAGE_SECTION.into(), lma: 4, len: 2 }
        );
        let deb2 = NoSourceRange { lma: 4, len: 2, kind: NoSourceKind::Deb2Appendix };
        build(&i, 6, files.clone(), &[deb2]).unwrap();
        let dup = vec![row(RootName::Engine, "z.emp"), row(RootName::Engine, "z.emp")];
        assert!(matches!(build(&i, 4, dup, &[]), Err(ExportError::DuplicateFile { .. })));
    }
}
