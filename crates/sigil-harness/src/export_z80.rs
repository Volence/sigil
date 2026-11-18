//! The build export's Z80 bridge (G0a spec 4.4, "The Z80").
//!
//! Seam 1 links the resident sound driver on its own ([`crate::seam1`]), writes the blob
//! to `engine/sound/generated/`, and `boot_data.emp` embeds it. So the ROM's placed
//! sections know the driver only as one data fragment on the embedding line. What seam 1
//! knew while it linked (the five modules' placed sections, their symbols and
//! instruction marks, and their source files, each parsed under its own source id) is
//! handed over in process ([`crate::seam1::ResidentLink`]) and turned here into a
//! [`Z80Bridge`]: the driver's symbols and line rows in `space: z80`, rebased to the ROM.
//!
//! Nothing is re-derived a second way. The rows come from the same line walk the ROM's
//! sections take ([`walk_lines`]), the symbols from the same placement rule
//! ([`PlacedSymbols`]) and the same `.emp` side table reader
//! ([`crate::export_symbols::emp_symbols`]). The only new fact is where the ROM embeds
//! the blob ([`embed_site`]), found from the build: the one run of placed data fragments
//! whose bytes ARE the blob, never a label name.

use crate::seam1::ResidentLink;
use sigil_ir::{Cpu, Fragment, Section, SymbolTable};
use sigil_link::export::locate::{FileRef, SourcePos, SpanLocator};
use sigil_link::export::{walk_lines, LineSection, PlacedSymbols, Space, Z80Bridge, IMAGE_SECTION};
use sigil_span::{SourceMap, Span};

/// The resident driver's span authority: its five files at source ids `0..5`, the ids
/// seam 1 parses them under.
struct ResidentSources {
    map: SourceMap,
    files: Vec<FileRef>,
}

impl ResidentSources {
    fn new(link: &ResidentLink) -> ResidentSources {
        let mut map = SourceMap::new();
        let mut files = Vec::with_capacity(link.files.len());
        for (path, text) in &link.files {
            let name = path.display().to_string();
            map.add_named(name.clone(), text.clone());
            files.push(FileRef { path: name });
        }
        ResidentSources { map, files }
    }
}

impl SpanLocator for ResidentSources {
    fn outermost(&self, span: Span) -> Option<SourcePos> {
        if !self.map.contains(span.source) {
            return None;
        }
        let (id, line) = self.map.outermost(span)?;
        Some(SourcePos { file: id.0, line })
    }

    fn file(&self, id: u32) -> &FileRef {
        self.files
            .get(id as usize)
            .unwrap_or_else(|| panic!("file id {id} was never issued by the resident driver's locator"))
    }
}

/// The load address where `resolved` (the ROM's placed sections) embeds `blob`: the
/// start of the ONE run of consecutive, fixup-free `Data` fragments whose bytes are
/// exactly the blob. None, or more than one, is an error: the bridge would describe
/// bytes it cannot place.
pub fn embed_site(resolved: &[Section], blob: &[u8]) -> Result<u32, String> {
    if blob.is_empty() {
        return Err("the resident driver blob is empty: there is nothing to place".into());
    }
    let mut found: Vec<(String, u32)> = Vec::new();
    for sec in resolved {
        // (offset, bytes) of each fixup-free Data fragment, a run broken by anything else.
        let mut runs: Vec<Vec<(u32, &[u8])>> = vec![Vec::new()];
        let mut cursor: u32 = 0;
        for frag in &sec.fragments {
            match frag {
                Fragment::Data(d) if d.fixups.is_empty() => {
                    runs.last_mut().expect("never empty").push((cursor, &d.bytes));
                    cursor += d.bytes.len() as u32;
                }
                other => {
                    runs.push(Vec::new());
                    match other {
                        Fragment::Data(d) => cursor += d.bytes.len() as u32,
                        Fragment::Fill { count, .. } | Fragment::Reserve { count, .. } => cursor += count,
                        Fragment::Org { target, .. } => cursor = *target,
                        _ => {}
                    }
                }
            }
        }
        for run in &runs {
            for i in 0..run.len() {
                let (at, mut got) = (run[i].0, 0usize);
                for (_, bytes) in &run[i..] {
                    if got + bytes.len() > blob.len() || blob[got..got + bytes.len()] != **bytes {
                        break;
                    }
                    got += bytes.len();
                    if got == blob.len() {
                        found.push((sec.name.clone(), sec.lma.wrapping_add(at)));
                        break;
                    }
                }
            }
        }
    }
    match found.as_slice() {
        [(_, lma)] => Ok(*lma),
        [] => Err(format!(
            "no placed section embeds the {}-byte resident driver blob seam 1 linked: the ROM and the seam disagree",
            blob.len()
        )),
        many => Err(format!("the resident driver blob is embedded {} times ({many:?}): the bridge cannot tell which", many.len())),
    }
}

/// The bridge for `link`, embedded in the ROM at `blob_lma`.
///
/// Each module's load address becomes `blob_lma` plus its VMA origin (a module's VMA is
/// its offset into the blob, [`crate::seam1`]'s running cursor), so a row's and a
/// symbol's `lma` is the ROM byte and its `vma` the Z80 address. The walk is the ROM's
/// own: last writer per byte, the `.emp` instruction marks deciding `code` or `data`, and
/// a byte no driver line wrote is an error. The walk also derives the fill below the
/// first module, `[0, blob_lma)`, as it would for a ROM; that row is the ROM's, not the
/// driver's, and is dropped. Any other no-module byte is a gap inside the blob and is
/// refused.
pub fn z80_bridge(link: &ResidentLink, blob_lma: u32) -> Result<Z80Bridge, String> {
    let fail = |m: String| format!("build export: the Z80 bridge: {m}");
    let mut sections = link.sections.clone();
    let mut expected = blob_lma as u64;
    for s in &mut sections {
        if s.cpu != Cpu::Z80 {
            return Err(fail(format!("resident section `{}` is not a Z80 section", s.name)));
        }
        let lma = blob_lma as u64 + s.vma_origin() as u64;
        if lma != expected {
            return Err(fail(format!(
                "resident section `{}` starts at blob offset ${:X}, but the previous module ends at ${:X}",
                s.name,
                s.vma_origin(),
                expected - blob_lma as u64
            )));
        }
        s.lma = u32::try_from(lma).map_err(|_| fail(format!("resident section `{}` lands past 4 GiB", s.name)))?;
        expected = lma + s.image_len() as u64;
    }
    if expected - blob_lma as u64 != link.bytes.len() as u64 {
        return Err(fail(format!(
            "the resident sections store {} bytes, the blob is {}",
            expected - blob_lma as u64,
            link.bytes.len()
        )));
    }
    let loc = ResidentSources::new(link);
    let table = walk_lines(&sections, &loc, &[], &link.symbols.code, &[]).map_err(|e| fail(e.to_string()))?;
    let mut out: Vec<LineSection> = Vec::with_capacity(table.sections.len());
    for s in table.sections {
        if s.name != IMAGE_SECTION {
            out.push(s);
            continue;
        }
        for r in &s.rows {
            if (r.lma, r.len) != (0, blob_lma) {
                return Err(fail(format!("{} byte(s) at ${:X} inside the blob belong to no driver module", r.len, r.lma)));
            }
        }
    }
    let placed = PlacedSymbols::new(&sections, &SymbolTable::new(), &[]);
    let symbols = crate::export_symbols::emp_symbols(&link.symbols, &placed).map_err(fail)?;
    for s in &symbols {
        if s.space.is_some() && (s.space != Some(Space::Z80) || s.vma > 0xFFFF || s.lma.is_none()) {
            return Err(fail(format!("driver symbol `{}` is not a z80 address with a load address: {s:?}", s.name)));
        }
    }
    Ok(Z80Bridge { blob_lma, files: loc.files, symbols, sections: out })
}
