//! The line table (spec §4.4): which source line wrote each ROM byte.
//!
//! # The walk
//!
//! The walk point is the placed, post-`resolve_layout` sections: every relaxable
//! fragment has been lowered in place to `Data` with its original span, and every
//! address is final. Fragment order is NOT address order, because `Org` moves the
//! write cursor backward as well as forward (the `org Hdr / dc.b n / org End`
//! back-patch). So the walk replays each section's write cursor with exactly the rule
//! [`Section::image_bytes`] uses, and records, for every image byte, the LAST fragment
//! that wrote it. A back-patched byte therefore belongs to the patching line, and rows
//! are cut wherever the owner changes, so no two rows overlap.
//!
//! Bytes the replay creates without a writer get a no-source kind:
//!  * a forward `Org` past everything written so far is gap-filled with the `Org`'s
//!    fill byte: [`NoSourceKind::OrgGap`];
//!  * a `Reserve` followed by a write inside the same section materialises as zero
//!    fill (asl leaves a gap, p2bin fills it): [`NoSourceKind::Fill`];
//!  * a `Fill` fragment is padding (`align`, a trimmed chain pad): [`NoSourceKind::Fill`].
//!
//! # Code or data: the rule, stated once
//!
//! A row is `code` exactly when the fragment that wrote it was emitted as an
//! instruction, and `data` otherwise (a `dc`, a blob, embedded data). The fragment
//! itself cannot say: both front ends push both through one `IrStreamer::emit_data`,
//! and `resolve_layout` lowers the relaxable instruction fragments to plain `Data`. So
//! the front ends record it as they lower (`sigil_ir::Module::code`: the `.emp`
//! lowering around each `CodeItem::Instr`, the AS front end around
//! `lower_instruction`), the driver hands those marks over by section name, and
//! [`row_kind`] is the one place they are read. Nothing is guessed from bytes.
//!
//! # No-source ranges
//!
//! The driver declares the bytes that no source line wrote: section fill between
//! placed sections, the deb2 appendix, and the header checksum. The checksum is folded
//! into the header AFTER the link, over bytes a source line declared (`Checksum: u16 =
//! 0`), so it is the last writer of those two bytes and takes them over. Every other
//! no-source range must land on bytes no section wrote; one that lands on a section
//! byte is an [`ExportError::Overlap`], because it would mean the driver and the
//! sections disagree about the image. Ranges outside every section are rows of the
//! [`IMAGE_SECTION`] line section.
//!
//! # Completeness
//!
//! Section fill (the gaps between placed sections that `sigil_link::flatten` fills)
//! is derived here from the replayed section lengths, so the image is not built twice.
//!
//! Every byte of every byte-emitting section has exactly one row, or the walk fails:
//! a byte whose span names no file is [`ExportError::Uncovered`], never dropped and
//! never attributed to a guess. The whole-image check (every byte of the ROM file
//! covered once) is [`check_image_coverage`], run by the exporter's entry.

use super::locate::SpanLocator;
use super::{ExportError, LineRow, LineSection, LineTable, NoSourceKind, RowKind, Space};
use sigil_ir::{AddressSpace, Cpu, Fragment, Section};

/// The line section that holds the no-source rows outside every placed section (fill
/// between sections, the deb2 appendix). Parentheses keep it apart from every section
/// name a front end makes, and a clash is still refused by the JSON writer.
pub const IMAGE_SECTION: &str = "(image)";

/// Bytes of the image no source line wrote, in image (load) addresses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoSourceRange {
    pub lma: u32,
    pub len: u32,
    pub kind: NoSourceKind,
}

/// Who wrote one byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Owner {
    /// A fragment whose span names no file.
    Unlocated,
    /// A located source line (`file` is the LOCATOR's file id).
    Source { file: u32, line: u32, kind: RowKind },
    NoSource(NoSourceKind),
}

/// THE code-or-data rule (see the module doc): a fragment a front end marked as an
/// instruction emit is `code`; every other `Data` fragment is `data`.
fn row_kind(marked_code: bool) -> RowKind {
    if marked_code {
        RowKind::Code
    } else {
        RowKind::Data
    }
}

/// The instruction marks (`(section name, fragment range)`) as a set of fragment
/// indices per section index. A mark must name exactly one placed section that holds a
/// `Data` fragment at every index it marks; anything else means the positions no
/// longer name what the front end lowered, and is refused rather than guessed.
fn code_fragments(
    sections: &[Section],
    code: &[(String, std::ops::Range<usize>)],
) -> Result<Vec<std::collections::HashSet<usize>>, ExportError> {
    let mut by_name: std::collections::HashMap<&str, Vec<usize>> = std::collections::HashMap::new();
    for (k, s) in sections.iter().enumerate() {
        by_name.entry(s.name.as_str()).or_default().push(k);
    }
    let mut out = vec![std::collections::HashSet::new(); sections.len()];
    for (name, range) in code {
        let fits = |k: &usize| {
            let f = &sections[*k].fragments;
            range.end <= f.len() && f[range.clone()].iter().all(|x| matches!(x, Fragment::Data(_)))
        };
        let cands: Vec<usize> = by_name.get(name.as_str()).map_or(vec![], |v| v.iter().copied().filter(fits).collect());
        match cands.as_slice() {
            [k] => out[*k].extend(range.clone()),
            [] => {
                return Err(ExportError::CodeMark {
                    section: name.clone(),
                    why: format!("no placed section of that name holds Data fragments {range:?}"),
                })
            }
            _ => {
                return Err(ExportError::CodeMark {
                    section: name.clone(),
                    why: format!("{} placed sections of that name could hold fragments {range:?}", cands.len()),
                })
            }
        }
    }
    Ok(out)
}

/// The address space a section's rows are stated in: its CPU's, as the symbol table
/// states it ([`super::placed`]).
fn space_of(sec: &Section) -> Space {
    match sec.cpu {
        Cpu::Z80 => Space::Z80,
        _ => Space::M68k,
    }
}

/// Replay `sec`'s write cursor, recording the last writer of every image byte. The
/// shape is [`Section::image_bytes`]'s, step for step.
fn owners(
    sec: &Section,
    loc: &dyn SpanLocator,
    code: &std::collections::HashSet<usize>,
) -> Result<Vec<Owner>, ExportError> {
    let mut own: Vec<Owner> = Vec::new();
    let mut cursor: usize = 0;
    for (i, frag) in sec.fragments.iter().enumerate() {
        match frag {
            Fragment::Data(d) => {
                let end = cursor + d.bytes.len();
                if end > own.len() {
                    // Bytes between the old extent and the cursor exist only because a
                    // `Reserve` advanced it: zero fill.
                    own.resize(end, Owner::NoSource(NoSourceKind::Fill));
                }
                let o = match loc.outermost(d.span) {
                    Some(p) => Owner::Source { file: p.file, line: p.line, kind: row_kind(code.contains(&i)) },
                    None => Owner::Unlocated,
                };
                own[cursor..end].fill(o);
                cursor = end;
            }
            Fragment::Fill { count, .. } => {
                let end = cursor + *count as usize;
                if end > own.len() {
                    own.resize(end, Owner::NoSource(NoSourceKind::Fill));
                }
                own[cursor..end].fill(Owner::NoSource(NoSourceKind::Fill));
                cursor = end;
            }
            Fragment::Reserve { count, .. } => cursor += *count as usize,
            Fragment::Org { target, .. } => {
                let t = *target as usize;
                if t > own.len() {
                    own.resize(t, Owner::NoSource(NoSourceKind::OrgGap));
                }
                cursor = t;
            }
            Fragment::JmpJsrSym { .. } | Fragment::RelaxAbsSym { .. } | Fragment::RelaxLadder { .. } => {
                return Err(ExportError::Unlowered { section: sec.name.clone() });
            }
        }
    }
    Ok(own)
}

/// Run-length rows over `own`, a section's owners from offset 0.
fn rows_of(sec: &Section, own: &[Owner]) -> Result<Vec<LineRow>, ExportError> {
    let mut rows = Vec::new();
    let mut i = 0;
    while i < own.len() {
        let o = own[i];
        let mut j = i + 1;
        while j < own.len() && own[j] == o {
            j += 1;
        }
        let (file, line, kind) = match o {
            Owner::Source { file, line, kind } => (file, line, kind),
            Owner::NoSource(k) => (0, 0, RowKind::NoSource(k)),
            Owner::Unlocated => {
                return Err(ExportError::Uncovered {
                    section: sec.name.clone(),
                    lma: sec.lma.wrapping_add(i as u32),
                    len: (j - i) as u32,
                })
            }
        };
        rows.push(LineRow {
            vma: sec.vma_at(i as u32),
            lma: sec.lma.wrapping_add(i as u32),
            len: (j - i) as u32,
            file,
            line,
            kind,
        });
        i = j;
    }
    Ok(rows)
}

/// The line table of `sections`. `no_source` declares the image bytes written after
/// the link by no source line (the deb2 appendix, the folded checksum; see the module
/// doc for which ranges may land on section bytes); fill between sections is derived
/// here. `code` is the front ends' instruction marks, by section name. Row `file`
/// values are the LOCATOR's file ids; [`super::entry::build_export`] maps them to rows
/// of `files`.
pub fn walk_lines(
    sections: &[Section],
    loc: &dyn SpanLocator,
    no_source: &[NoSourceRange],
    code: &[(String, std::ops::Range<usize>)],
) -> Result<LineTable, ExportError> {
    let code = code_fragments(sections, code)?;
    // (section index, image owners) for every section that stores a byte.
    let mut placed: Vec<(usize, Vec<Owner>)> = Vec::new();
    for (k, sec) in sections.iter().enumerate() {
        let own = owners(sec, loc, &code[k])?;
        if own.is_empty() {
            continue;
        }
        if sec.space != AddressSpace::Image {
            return Err(ExportError::ForeignSpace { section: sec.name.clone() });
        }
        debug_assert_eq!(own.len(), sec.image_len() as usize, "section `{}`: replay vs image_bytes", sec.name);
        placed.push((k, own));
    }
    placed.sort_by_key(|(k, _)| sections[*k].lma);

    // Section fill: the gaps `flatten` fills between placed sections, from the
    // replayed lengths.
    let mut no_source = no_source.to_vec();
    let mut at = 0u64;
    for (k, own) in &placed {
        let lma = sections[*k].lma as u64;
        if lma > at {
            no_source.push(NoSourceRange { lma: at as u32, len: (lma - at) as u32, kind: NoSourceKind::Fill });
        }
        at = at.max(lma + own.len() as u64);
    }

    // Declared no-source ranges: a checksum fold takes over the section bytes it lands
    // on; every other range must lie outside every section.
    let mut outside: Vec<NoSourceRange> = Vec::new();
    for r in &no_source {
        if r.len == 0 {
            continue;
        }
        let (start, end) = (r.lma as u64, r.lma as u64 + r.len as u64);
        let mut cut = start;
        let mut pieces: Vec<(u64, u64)> = Vec::new(); // parts outside every section
        for (k, own) in placed.iter_mut() {
            let sec = &sections[*k];
            let (s0, s1) = (sec.lma as u64, sec.lma as u64 + own.len() as u64);
            let (a, b) = (start.max(s0), end.min(s1));
            if a >= b {
                continue;
            }
            if r.kind != NoSourceKind::HeaderChecksum {
                return Err(ExportError::Overlap { lma: a as u32 });
            }
            own[(a - s0) as usize..(b - s0) as usize].fill(Owner::NoSource(r.kind));
            if a > cut {
                pieces.push((cut, a));
            }
            cut = cut.max(b);
        }
        if cut < end {
            pieces.push((cut, end));
        }
        for (a, b) in pieces {
            outside.push(NoSourceRange { lma: a as u32, len: (b - a) as u32, kind: r.kind });
        }
    }
    outside.sort_by_key(|r| r.lma);
    for w in outside.windows(2) {
        if w[0].lma as u64 + w[0].len as u64 > w[1].lma as u64 {
            return Err(ExportError::Overlap { lma: w[1].lma });
        }
    }

    let mut table = LineTable::default();
    for (k, own) in &placed {
        let sec = &sections[*k];
        table.sections.push(LineSection { name: sec.name.clone(), space: space_of(sec), rows: rows_of(sec, own)? });
    }
    if !outside.is_empty() {
        let rows = outside
            .iter()
            .map(|r| LineRow { vma: r.lma, lma: r.lma, len: r.len, file: 0, line: 0, kind: RowKind::NoSource(r.kind) })
            .collect();
        table.sections.push(LineSection { name: IMAGE_SECTION.to_string(), space: Space::M68k, rows });
    }
    merge_adjacent(&mut table);
    check_no_overlap(&table)?;
    Ok(table)
}

/// Merge rows of one section that are contiguous in both address columns and agree on
/// `(file, line, kind)`. Rows are sorted by `lma` first.
pub fn merge_adjacent(table: &mut LineTable) {
    for s in &mut table.sections {
        s.rows.sort_by_key(|r| r.lma);
        let mut out: Vec<LineRow> = Vec::with_capacity(s.rows.len());
        for r in s.rows.drain(..) {
            if let Some(p) = out.last_mut() {
                if (p.file, p.line, p.kind) == (r.file, r.line, r.kind)
                    && p.lma.wrapping_add(p.len) == r.lma
                    && p.vma.wrapping_add(p.len) == r.vma
                {
                    p.len += r.len;
                    continue;
                }
            }
            out.push(r);
        }
        s.rows = out;
    }
}

/// No two rows of the table, across every section, share an image byte.
pub fn check_no_overlap(table: &LineTable) -> Result<(), ExportError> {
    let mut spans: Vec<(u64, u64)> = table
        .sections
        .iter()
        .flat_map(|s| s.rows.iter().map(|r| (r.lma as u64, r.lma as u64 + r.len as u64)))
        .collect();
    spans.sort();
    for w in spans.windows(2) {
        if w[0].1 > w[1].0 {
            return Err(ExportError::Overlap { lma: w[1].0 as u32 });
        }
    }
    Ok(())
}

/// Every byte of a `len`-byte image is covered by exactly one row: no overlap, no gap,
/// nothing past the end.
pub fn check_image_coverage(table: &LineTable, len: u64) -> Result<(), ExportError> {
    check_no_overlap(table)?;
    let mut spans: Vec<(u64, u64)> = table
        .sections
        .iter()
        .flat_map(|s| s.rows.iter().map(|r| (r.lma as u64, r.lma as u64 + r.len as u64)))
        .collect();
    spans.sort();
    let mut at = 0u64;
    for (a, b) in spans {
        if a > at {
            return Err(ExportError::Uncovered { section: IMAGE_SECTION.into(), lma: at as u32, len: (a - at) as u32 });
        }
        at = b;
    }
    if at < len {
        return Err(ExportError::Uncovered { section: IMAGE_SECTION.into(), lma: at as u32, len: (len - at) as u32 });
    }
    if at > len {
        return Err(ExportError::Internal(format!("rows run to {at:#X}, past the {len:#X}-byte image")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::locate::{FileRef, SourcePos};
    use super::*;
    use sigil_ir::{DataFragment, SectionPlacement};
    use sigil_span::{SourceId, Span};

    /// A locator where a span's source id is its file and its `start` is its line;
    /// `SourceId(u32::MAX)` names no file.
    struct Lines(Vec<FileRef>);

    impl SpanLocator for Lines {
        fn outermost(&self, span: Span) -> Option<SourcePos> {
            (span.source.0 != u32::MAX).then_some(SourcePos { file: span.source.0, line: span.start })
        }
        fn file(&self, id: u32) -> &FileRef {
            &self.0[id as usize]
        }
    }

    fn loc() -> Lines {
        Lines(vec![FileRef { path: "a.emp".into() }, FileRef { path: "b.emp".into() }])
    }

    fn at(line: u32) -> Span {
        Span { source: SourceId(0), start: line, end: line }
    }

    fn data(n: usize, line: u32) -> Fragment {
        Fragment::Data(DataFragment { bytes: vec![0xAB; n], fixups: vec![], span: at(line) })
    }

    fn org(target: u32) -> Fragment {
        Fragment::Org { target, fill: 0xFF, span: at(1) }
    }

    fn sec(name: &str, lma: u32, fragments: Vec<Fragment>) -> Section {
        Section {
            name: name.into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma,
            space: AddressSpace::Image,
            labels: vec![],
            fragments,
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![],
        }
    }

    /// `(lma, len, line, kind)` of every row of section `name`.
    fn rows(t: &LineTable, name: &str) -> Vec<(u32, u32, u32, RowKind)> {
        let s = t.sections.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("no section {name}"));
        s.rows.iter().map(|r| (r.lma, r.len, r.line, r.kind)).collect()
    }

    const D: RowKind = RowKind::Data;

    #[test]
    fn org_backpatch_last_writer_wins() {
        // data 4 B at 0 from line 10, org to 1, data 1 B from line 20, org to 4.
        let s = sec("s", 0, vec![data(4, 10), org(1), data(1, 20), org(4)]);
        let t = walk_lines(&[s], &loc(), &[], &[]).unwrap();
        assert_eq!(rows(&t, "s"), vec![(0, 1, 10, D), (1, 1, 20, D), (2, 2, 10, D)]);
    }

    #[test]
    fn forward_org_makes_a_gap_row() {
        let s = sec("s", 0x100, vec![data(2, 5), org(6), data(2, 7)]);
        let t = walk_lines(&[s], &loc(), &[], &[]).unwrap();
        assert_eq!(
            rows(&t, "s"),
            vec![(0x100, 2, 5, D), (0x102, 4, 0, RowKind::NoSource(NoSourceKind::OrgGap)), (0x106, 2, 7, D)]
        );
    }

    #[test]
    fn uncovered_byte_is_refused() {
        let lost = Fragment::Data(DataFragment {
            bytes: vec![1, 2, 3],
            fixups: vec![],
            span: Span { source: SourceId(u32::MAX), start: 0, end: 0 },
        });
        let s = sec("s", 0x200, vec![data(2, 5), lost, data(1, 6)]);
        assert_eq!(
            walk_lines(&[s], &loc(), &[], &[]),
            Err(ExportError::Uncovered { section: "s".into(), lma: 0x202, len: 3 })
        );
    }

    #[test]
    fn adjacent_rows_of_one_line_merge_and_others_do_not() {
        let s = sec("s", 0, vec![data(2, 5), data(2, 5), data(2, 6), data(2, 5)]);
        let t = walk_lines(&[s], &loc(), &[], &[]).unwrap();
        assert_eq!(rows(&t, "s"), vec![(0, 4, 5, D), (4, 2, 6, D), (6, 2, 5, D)]);
    }

    #[test]
    fn a_phased_section_states_both_addresses() {
        let mut s = sec("s", 0x4000, vec![data(2, 5), data(3, 6)]);
        s.vma_base = Some(0x8000);
        let t = walk_lines(&[s], &loc(), &[], &[]).unwrap();
        let r = &t.sections[0].rows;
        assert_eq!((r[1].vma, r[1].lma, r[1].len), (0x8002, 0x4002, 3));
    }

    #[test]
    fn fill_and_reserve_gaps_are_fill_rows() {
        let pad = Fragment::Fill { value: 0, count: 2, span: at(9) };
        let res = Fragment::Reserve { count: 3, span: at(9) };
        let s = sec("s", 0, vec![data(2, 5), pad, res, data(1, 6)]);
        let t = walk_lines(&[s], &loc(), &[], &[]).unwrap();
        let f = RowKind::NoSource(NoSourceKind::Fill);
        assert_eq!(rows(&t, "s"), vec![(0, 2, 5, D), (2, 5, 0, f), (7, 1, 6, D)]);
    }

    #[test]
    fn the_checksum_fold_takes_over_its_source_line() {
        let s = sec("hdr", 0x180, vec![data(0xE, 3), data(2, 4), data(0x70, 5)]);
        let ck = NoSourceRange { lma: 0x18E, len: 2, kind: NoSourceKind::HeaderChecksum };
        let t = walk_lines(&[s], &loc(), &[ck], &[]).unwrap();
        assert_eq!(
            rows(&t, "hdr"),
            vec![(0x180, 0xE, 3, D), (0x18E, 2, 0, RowKind::NoSource(NoSourceKind::HeaderChecksum)), (0x190, 0x70, 5, D)]
        );
    }

    #[test]
    fn a_non_checksum_range_on_section_bytes_is_an_overlap() {
        let s = sec("s", 0x100, vec![data(4, 3)]);
        let fill = NoSourceRange { lma: 0xFE, len: 4, kind: NoSourceKind::Fill };
        assert_eq!(walk_lines(&[s], &loc(), &[fill], &[]), Err(ExportError::Overlap { lma: 0x100 }));
    }

    #[test]
    fn ranges_outside_sections_are_image_rows_and_the_image_is_covered_once() {
        let a = sec("a", 0, vec![data(4, 3)]);
        let b = sec("b", 8, vec![data(4, 4)]);
        // The gap [4, 8) between the sections is fill, derived by the walk.
        let ns = vec![NoSourceRange { lma: 12, len: 4, kind: NoSourceKind::Deb2Appendix }];
        let t = walk_lines(&[a, b], &loc(), &ns, &[]).unwrap();
        assert_eq!(
            rows(&t, IMAGE_SECTION),
            vec![(4, 4, 0, RowKind::NoSource(NoSourceKind::Fill)), (12, 4, 0, RowKind::NoSource(NoSourceKind::Deb2Appendix))]
        );
        check_image_coverage(&t, 16).unwrap();
        assert_eq!(
            check_image_coverage(&t, 18),
            Err(ExportError::Uncovered { section: IMAGE_SECTION.into(), lma: 16, len: 2 })
        );
        // Drop a row: the gap is named.
        let mut dropped = t.clone();
        dropped.sections[0].rows.clear();
        assert_eq!(
            check_image_coverage(&dropped, 16),
            Err(ExportError::Uncovered { section: IMAGE_SECTION.into(), lma: 0, len: 4 })
        );
        // Duplicate a row over another: the overlap is named.
        let mut dup = t.clone();
        let r = dup.sections[0].rows[0];
        dup.sections[1].rows.push(r);
        assert_eq!(check_image_coverage(&dup, 16), Err(ExportError::Overlap { lma: 0 }));
    }

    fn mark(name: &str, r: std::ops::Range<usize>) -> (String, std::ops::Range<usize>) {
        (name.to_string(), r)
    }

    #[test]
    fn marked_fragments_are_code_rows_and_unmarked_are_data() {
        // instr(2, line 5), instr(2, line 5), dc(2, line 6), instr(2, line 7)
        let s = sec("s", 0, vec![data(2, 5), data(2, 5), data(2, 6), data(2, 7)]);
        let t = walk_lines(&[s], &loc(), &[], &[mark("s", 0..2), mark("s", 3..4)]).unwrap();
        let c = RowKind::Code;
        assert_eq!(rows(&t, "s"), vec![(0, 4, 5, c), (4, 2, 6, D), (6, 2, 7, c)]);
    }

    #[test]
    fn a_code_and_a_data_row_of_one_line_do_not_merge() {
        let s = sec("s", 0, vec![data(2, 5), data(2, 5)]);
        let t = walk_lines(&[s], &loc(), &[], &[mark("s", 0..1)]).unwrap();
        assert_eq!(rows(&t, "s"), vec![(0, 2, 5, RowKind::Code), (2, 2, 5, D)]);
    }

    #[test]
    fn a_mark_that_names_no_data_fragment_is_refused() {
        let s = sec("s", 0, vec![data(2, 5), org(2)]);
        // Past the end, on a non-Data fragment, and in a section that is not there.
        for m in [mark("s", 0..3), mark("s", 1..2), mark("t", 0..1)] {
            let err = walk_lines(&[s.clone()], &loc(), &[], &[m.clone()]).unwrap_err();
            assert!(matches!(&err, ExportError::CodeMark { section, .. } if *section == m.0), "{m:?}: {err:?}");
        }
    }

    #[test]
    fn a_mark_two_same_named_sections_could_hold_is_refused() {
        let a = sec("s", 0, vec![data(2, 5)]);
        let b = sec("s", 2, vec![data(2, 6)]);
        let err = walk_lines(&[a.clone(), b], &loc(), &[], &[mark("s", 0..1)]).unwrap_err();
        assert!(matches!(err, ExportError::CodeMark { .. }), "{err:?}");
        // One that only one of them can hold is placed there.
        let c = sec("s", 2, vec![org(0)]);
        let t = walk_lines(&[a, c], &loc(), &[], &[mark("s", 0..1)]).unwrap();
        assert_eq!(rows(&t, "s"), vec![(0, 2, 5, RowKind::Code)]);
    }

    #[test]
    fn an_unlowered_fragment_is_refused() {
        let j = Fragment::JmpJsrSym { is_jsr: true, target: sigil_ir::Expr::Int(0), span: at(1) };
        let s = sec("s", 0, vec![j]);
        assert_eq!(walk_lines(&[s], &loc(), &[], &[]), Err(ExportError::Unlowered { section: "s".into() }));
    }
}
