//! The exporter's entry: what a driver hands over after a build (`ExportInputs`),
//! and `build_export`, which turns it plus the ROM's identity into a `BuildExport`.
//! Driver-independent on purpose: the driver supplies placed sections, a span
//! locator and its side tables; nothing here knows a front end.

use super::lines::{check_image_coverage, merge_adjacent, walk_lines, NoSourceRange};
use super::locate::SpanLocator;
use super::{
    BuildExport, ConstRecord, ExportError, FileRecord, LayoutRecord, LineTable, RomRecord, RootName, RowKind,
    ShapeRecord, Space, SpaceRecord, SymbolRecord, ToolRecord,
};
use sigil_ir::Section;
use std::collections::HashMap;

/// The Z80 sound driver's bridge facts (bank windows). Empty until the task that
/// fills it (G0a Task 9).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Z80Bridge {}

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
    let mut lines = walk_lines(&inputs.sections, &*inputs.locator, post_link, &inputs.side.code)?;
    map_files(&mut lines, &*inputs.locator, &files, file_key)?;
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
        symbols: inputs.side.symbols.clone(),
        layouts: inputs.side.layouts.clone(),
        consts: inputs.side.consts.clone(),
        lines,
    })
}

/// Rewrite every source row's `file` from the locator's file id to its index in the
/// sorted `files`.
fn map_files(
    lines: &mut LineTable,
    loc: &dyn SpanLocator,
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
        let path = &loc.file(row.file).path;
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
