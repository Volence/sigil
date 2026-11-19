//! The emitted artifact's glue: the build's result type ([`RomBuild`]), the
//! sigil-canonical listing derived from a resolved layout, the Z80 bank windows the
//! export reads, the Sega header fields the post-steps rewrite, and the CRCs.
//! `emit_rom` itself is `sigil_link::emit_rom`.
//!
//! Moved from sigil-harness (`native.rs`, `lib.rs`) unchanged by G0b Task 4.

use sigil_ir::{Section, SymbolTable};
use sigil_link::export::ExportInputs;

use crate::diag::BuildWarning;
use crate::place::ModuleSpec;

/// The Genesis header's checksum word (`$18E..$190`) — a SEMANTIC header
/// fact, not a layout pin. One of the only two fields the out-of-scope
/// `convsym -a`/`fixheader` post-steps rewrite (`convsym -a` appends the
/// MD-Debugger `deb2` symbol table; `fixheader` re-checksums the appended
/// image — M1.B models `convsym` as a no-op, so Sigil's `emit_rom` target is
/// the pre-append ASSEMBLED ROM).
pub const CHECKSUM_FIELD_RANGE: std::ops::Range<usize> = 0x18E..0x190;

/// The header's `dc.l EndOfRom-1` ROM-end pointer (`$1A4..$1A8`) — the other
/// convsym-rewritten field (bumped to the POST-append end). Which of its four
/// bytes actually differ shifts whenever the deb2 append changes size (the
/// tranche-9 PerFrame deletion flipped `$1A5` back), which is why the
/// rewritten set is DERIVED per comparison rather than pinned — see
/// [`derive_convsym_rewritten`] (tranche-10 step 0, D-T10.6).
pub const ROM_END_FIELD_RANGE: std::ops::Range<usize> = 0x1A4..0x1A8;

/// An assembled ROM image with the artefacts derived from the same build: the
/// sigil-canonical symbol listing (the deb2-appendix source) and the `.emp`
/// lowering's [`BuildWarning`]s, carried out to whichever caller renders them.
pub struct RomBuild {
    /// The assembled ROM image (checksum already folded by `emit_rom`).
    pub rom: Vec<u8>,
    /// One `C` row per resolved section label at its final VMA.
    pub listing: Vec<sigil_link::ListingSymbol>,
    /// Every non-error diagnostic the `.emp` lowering produced.
    pub warnings: Vec<BuildWarning>,
    /// The module roots the build derived from the game's map and placed.
    pub registry: Vec<ModuleSpec>,
    /// What the build-data export reads: the placed sections, the span locator and the
    /// side tables ([`sigil_link::export::build_export`]).
    pub export_inputs: ExportInputs,
}

/// The shape's Z80 bank windows, from its memory map (the `z80_bank` regions in force,
/// each where its anchor placed it): what decides the space of a section phased into
/// one ([`sigil_link::export::section_space`]). A window the map gives no VMA cannot
/// say what a phased section in it maps to, so one holding a phased section is refused
/// rather than left to state that section in `m68k`.
pub fn z80_windows(
    map: &sigil_ir::map::MemoryMap,
    resolved: &[Section],
) -> Result<Vec<sigil_link::export::Z80Window>, String> {
    let mut out = Vec::new();
    for r in map.regions.iter().filter(|r| r.kind == sigil_ir::map::RegionKind::Z80Bank) {
        match r.vma_base {
            Some(vma) => out.push(sigil_link::export::Z80Window { lma: r.lma_base, vma, size: r.size }),
            None => {
                if let Some(s) =
                    resolved.iter().find(|s| r.contains(s.lma) && s.vma_origin() != s.lma && s.image_len() > 0)
                {
                    return Err(format!(
                        "build export: z80_bank region `{}` declares no vma, so the space of section `{}`, \
                         phased into it, cannot be stated",
                        r.name, s.name
                    ));
                }
            }
        }
    }
    Ok(out)
}

/// CRC-32 (IEEE, the campaign provenance standard alongside byte-size): the one
/// implementation the build's read set hashes with, so a ROM's `built:` CRC and its
/// digest rows come from the same function.
pub fn crc32(data: &[u8]) -> u32 {
    sigil_span::read_set::crc32(data)
}

/// The header-neutral ASSEMBLED ANCHOR CRC over `bytes[0, eor)`: the checksum (`$18E`)
/// and ROM-end-pointer (`$1A4`) header fields are zeroed before the CRC, so it is the
/// drift-stable invariant the PROVENANCE anchors record (`e5765873` &c). `eor` is the
/// target's `EndOfRom`; `bytes` may be a full golden file (only its prefix is read).
pub fn assembled_anchor_crc(bytes: &[u8], eor: usize) -> u32 {
    let mut prefix = bytes[..eor.min(bytes.len())].to_vec();
    for i in crate::emit::CHECKSUM_FIELD_RANGE.chain(crate::emit::ROM_END_FIELD_RANGE) {
        if i < prefix.len() {
            prefix[i] = 0;
        }
    }
    crc32(&prefix)
}

/// The sigil-canonical symbol listing of a `resolve_layout` output — the single
/// derivation both native drivers (chained + full-file) use.
///
/// TWO row kinds, because the listing describes two different kinds of name:
///
///  * every section LABEL → an as-`-L` `C` (code/ADDRESS) row at its final VMA.
///    De-duplicated (a label is defined once) and deterministic (`emit_listing`
///    address-sorts). RAM labels (`$FFFFxxxx`) are kept — convsym's
///    `-range 0 FFFFFF` drops them from the deb2 table, but they belong in the
///    full listing for the `s4budget` RAM consumer.
///  * every folded EQUATE → an as-`-L` `-` (VALUE) row. `pub equ` mints a
///    link-level `EquSym` and no label, so before this these names existed in the
///    linker's symbol table and nowhere a tool could read them; a `.emp` module
///    could compute a constant at comptime but not publish it. The `-` marker is
///    the AS form for a value symbol and the discriminator every consumer keys on
///    — see `sigil_link::emit_listing`, which additionally keeps equates out of
///    the Oracle address-listing half.
///
/// Label rows win a name collision (`seen` is seeded by the label pass): a label
/// is an address the debugger must resolve, and `link()` already rejects a genuine
/// equ-vs-label duplicate through its `defined_here` channel, so this is a
/// belt-and-braces tie-break, not a policy.
pub fn listing_from_resolved(
    resolved: &[sigil_ir::Section],
    stubs: &SymbolTable,
) -> Vec<sigil_link::ListingSymbol> {
    let mut listing: Vec<sigil_link::ListingSymbol> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for sec in resolved {
        let origin = sec.vma_origin();
        // PHASE, decided at the SECTION, which is the only place it exists. A
        // section with an explicit `vma:` that differs from where the linker
        // actually placed it runs at one address and is stored at another, and its
        // labels' listing values are the former. Anything else is unphased and
        // carries no `lma` at all, so `Some` in a row always means a real phase.
        //
        // A RESERVE-ONLY section is excluded even when its VMA differs from its
        // LMA. Aeon's RAM blocks are exactly that shape (`vma: $FFFFxxxx` over an
        // LMA that anchors at the physical counter and places ZERO image bytes), so
        // an `lma` for one of them would name a storage address for bytes that were
        // never stored: a number a consumer could follow into unrelated ROM. RAM
        // symbols are already recognised by every consumer here from their own
        // `$FFFFxxxx` value; what the marker exists to disambiguate is a phased
        // address that looks ORDINARY.
        let phased = sec.vma_origin() != sec.lma && sec.image_len() > 0;
        for label in &sec.labels {
            if seen.insert(label.name.clone()) {
                listing.push(sigil_link::ListingSymbol {
                    name: label.name.clone(),
                    value: origin.wrapping_add(label.offset),
                    is_equate: false,
                    unused: false,
                    lma: phased.then(|| sec.lma.wrapping_add(label.offset)),
                });
            }
        }
    }
    for (name, value) in sigil_link::resolved_equates(resolved, stubs) {
        if seen.insert(name.clone()) {
            listing.push(sigil_link::ListingSymbol {
                name,
                value: value as u32,
                is_equate: true,
                unused: false,
                // An equate is a value, not storage: never phased.
                lma: None,
            });
        }
    }
    listing
}
