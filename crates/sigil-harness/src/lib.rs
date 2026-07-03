//! sigil-harness — recon + integration-harness helpers for the M0 golden-diff
//! pipeline.
//!
//! This crate derives the two Z80 extraction windows (Region A = the resident
//! phase-0 driver, Region B = the phase-08000h Moving-Trucks / SFX bank) and the
//! external 68k leaf-stub set directly from a *fresh* `asl` build of the Aeon
//! reference ROM. Nothing here invents values: every number comes from the
//! `s4.lst` symbol table produced by the same build invocation that emitted
//! `s4.bin`.
//!
//! The pure functions (`parse_lst_symbols`, `derive_region_a`,
//! `derive_region_b`) live here so they are unit-testable; the `regen` bin
//! orchestrates the build + extraction + file writes.

use std::collections::BTreeMap;
use std::path::Path;

use sigil_frontend_as::{assemble_root, Options};
use sigil_ir::{Cpu, Module, SymbolTable, SymbolValue};
use sigil_link::LinkedImage;

/// Region A bracket anchors (68k-context labels around the phase-0 driver).
pub const REGION_A_START_SYM: &str = "Z80_Sound_Start";
pub const REGION_A_END_SYM: &str = "Z80_Sound_End";
/// Region B anchors: LMA of the phase-08000h bank, and the first 68k label
/// after `dephase/restore` (the song include) that ends the region.
pub const REGION_B_START_SYM: &str = "MovingTrucks_Bank_Start";
pub const REGION_B_END_SYM: &str = "Song_MovingTrucks";

/// A physical extraction window into the reference ROM.
///
/// * `vma_base` — the virtual (phased) base the region's labels are relative to
///   (0 for Region A / `phase 0`; `0x8000` for Region B / `phase 08000h`).
/// * `lma`      — the physical ROM byte offset where the region's bytes live.
/// * `len`      — the region length in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionWindow {
    pub vma_base: u64,
    pub lma: u64,
    pub len: u64,
}

/// Parse the `Name : HEXVALUE C` symbol-table cells out of an `asl` `.lst`.
///
/// The listing symbol table packs one or more cells per line, `|`-separated,
/// each shaped `Name :    HEXVALUE C `. We split every line on `|`, then split
/// each cell on the first `:`. The name is the trimmed left side and MUST NOT
/// contain whitespace (this is the guard that rejects source/code listing lines,
/// whose left-of-colon side is an `N/ M` address prefix full of spaces, and
/// header timestamps like `02:24:58`). The value is the first whitespace token
/// on the right, parsed as hexadecimal; a cell whose value is non-hex, or which
/// has no `:`, is silently skipped.
pub fn parse_lst_symbols(lst: &str) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for line in lst.lines() {
        for cell in line.split('|') {
            if let Some((name, value)) = parse_symbol_cell(cell) {
                out.insert(name, value);
            }
        }
    }
    out
}

fn parse_symbol_cell(cell: &str) -> Option<(String, u64)> {
    let (lhs, rhs) = cell.split_once(':')?;
    // A leading `*` marks the symbol in AS's table (e.g. section-local); drop it.
    let name = lhs.trim().trim_start_matches('*');
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return None;
    }
    let tok = rhs.split_whitespace().next()?;
    let value = u64::from_str_radix(tok, 16).ok()?;
    Some((name.to_string(), value))
}

fn sym(syms: &BTreeMap<String, u64>, name: &str) -> Result<u64, String> {
    syms.get(name)
        .copied()
        .ok_or_else(|| format!("symbol `{name}` not found in the .lst symbol table"))
}

/// Region A = the resident phase-0 Z80 driver.
///
/// Both bracketing labels are defined in the 68k context (`Z80_Sound_Start:`
/// sits *before* the driver's `save/cpu z80/phase 0`, and `Z80_Sound_End:` sits
/// *after* its `dephase/restore` — see engine/sound/z80_sound_driver.asm), so
/// their `.lst` values are the real ROM addresses. LMA = `Z80_Sound_Start`;
/// length = `Z80_Sound_End - Z80_Sound_Start` (== the `Z80_SOUND_SIZE` EQU).
/// vma_base = 0 because the driver is phased at 0 (Z80 RAM $0000).
pub fn derive_region_a(syms: &BTreeMap<String, u64>) -> Result<RegionWindow, String> {
    let start = sym(syms, REGION_A_START_SYM)?;
    let end = sym(syms, REGION_A_END_SYM)?;
    if end <= start {
        return Err(format!(
            "Region A end {end:#x} <= start {start:#x} — bad bracket"
        ));
    }
    Ok(RegionWindow {
        vma_base: 0,
        lma: start,
        len: end - start,
    })
}

/// Region B = the phase-08000h Moving-Trucks / SFX engine-table bank.
///
/// LMA anchor = `MovingTrucks_Bank_Start` (a 68k-context label defined BEFORE
/// the `save/phase 08000h`, so its value is the physical ROM address = the bank
/// start). The phased block (main.asm 283-308) is immediately followed, after
/// `dephase/restore`, by `include song_movingtrucks.asm`, whose first label is
/// `Song_MovingTrucks` (a real ROM address). So the region ends there and its
/// length = `Song_MovingTrucks - MovingTrucks_Bank_Start`. vma_base = 0x8000.
pub fn derive_region_b(syms: &BTreeMap<String, u64>) -> Result<RegionWindow, String> {
    let lma = sym(syms, REGION_B_START_SYM)?;
    let next = sym(syms, REGION_B_END_SYM)?;
    if next <= lma {
        return Err(format!(
            "Region B next-anchor {next:#x} <= bank start {lma:#x} — bad bracket"
        ));
    }
    Ok(RegionWindow {
        vma_base: 0x8000,
        lma,
        len: next - lma,
    })
}

/// Maps a section's `(cpu, vma_base)` to the real ROM LMA it loads at. The
/// front-end sets `lma = vma_base` as a placeholder; the harness overrides it
/// before link so region A lands at its real ROM LMA and region B at $60000.
#[derive(Clone, Debug, Default)]
pub struct LmaMap {
    entries: Vec<(Cpu, Option<u32>, u32)>, // (cpu, vma_base, lma)
}

impl LmaMap {
    pub fn new() -> Self {
        LmaMap { entries: Vec::new() }
    }

    pub fn set(&mut self, cpu: Cpu, vma_base: Option<u32>, lma: u32) {
        self.entries.push((cpu, vma_base, lma));
    }

    fn lookup(&self, cpu: Cpu, vma_base: Option<u32>) -> Option<u32> {
        self.entries.iter().find(|(c, v, _)| *c == cpu && *v == vma_base).map(|(_, _, l)| *l)
    }
}

/// Override each section's LMA from the map. Errors (as a message) if a section
/// has no mapping — the harness must account for every section.
pub fn assign_lmas(module: &mut Module, map: &LmaMap) -> Result<(), String> {
    for sec in &mut module.sections {
        match map.lookup(sec.cpu, sec.vma_base) {
            Some(lma) => sec.lma = lma,
            None => {
                return Err(format!(
                    "no LMA mapping for section `{}` (cpu {:?}, vma_base {:?})",
                    sec.name, sec.cpu, sec.vma_base
                ))
            }
        }
    }
    Ok(())
}

/// The reference build's define set: `SOUND_DRIVER_ENABLED` defined;
/// `__DEBUG__`/`SOUND_DBG_MIRROR` undefined; `SND_REKEY_OFF_THEN_ON = 1`. Plus
/// the derived 68k leaf stubs.
pub fn reference_options(aeon_root: &Path, stubs: &[(String, i64)]) -> Options {
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SND_REKEY_OFF_THEN_ON".to_string(), 1),
    ];
    defines.extend(stubs.iter().cloned());
    Options {
        initial_cpu: Cpu::Z80,
        defines,
        include_root: Some(aeon_root.to_path_buf()),
    }
}

/// Assemble regions A+B together and link them at their real LMAs. `stubs` seeds
/// BOTH the front-end env (defines) AND the link `SymbolTable` (fallback for
/// surviving `BankPtr16Le` fixup targets).
pub fn build_harness(
    aeon_root: &Path,
    harness_root: &Path,
    stubs: &[(String, i64)],
    map: &LmaMap,
) -> Result<LinkedImage, String> {
    let opts = reference_options(aeon_root, stubs);
    let mut module = assemble_root(harness_root, &opts)
        .map_err(|d| format!("assemble: {} diagnostics; first: {:?}", d.len(), d.first()))?;
    // Drop empty label-only sections. Region A's driver source brackets itself
    // with `Z80_Sound_Start:` (just before its `save/cpu z80/phase 0`) and
    // `Z80_Sound_End:` (just after its `dephase/restore`); in the 68000-context
    // preamble those two labels each open an empty `(M68000, vma_base None)`
    // section carrying zero fragments. They contribute no image bytes (nothing
    // in A/B references them — the tail `Z80_SOUND_SIZE` equate is evaluated at
    // assemble time), so dropping them is byte-neutral and leaves exactly the
    // two real output regions: A `sec0` (Z80) and B `sec32768` (Z80).
    //
    // A `Reserve`/`ds` region is NOT empty (Reserve is a fragment), so this is
    // safe for BSS-style sections. This is a blanket drop, though: if a future
    // harness adds a genuinely label-only section whose labels ARE referenced by
    // a surviving fixup, it would be silently dropped — revisit the filter then.
    module.sections.retain(|s| !s.fragments.is_empty());
    assign_lmas(&mut module, map)?;
    let mut stub_table = SymbolTable::new();
    for (name, value) in stubs {
        stub_table.define(name, SymbolValue::Int(*value));
    }
    sigil_link::link(&module.sections, &stub_table)
        .map_err(|d| format!("link: {} diagnostics; first: {:?}", d.len(), d.first()))
}

/// Compare a linked section's bytes against a golden blob. On mismatch, report
/// the first differing byte offset. The `LinkedSection` here carries only
/// name/lma/bytes; to map that offset back to source, correlate it against the
/// original IR `Section`'s fragment spans (the pre-link representation), not this
/// `LinkedImage`. `name` is the section name in the `LinkedImage`.
pub fn diff_region(img: &LinkedImage, name: &str, golden: &[u8]) -> Result<(), String> {
    let sec = img.section(name).ok_or_else(|| format!("no linked section `{name}`"))?;
    if sec.bytes.len() != golden.len() {
        return Err(format!("`{name}` length {} != golden {}", sec.bytes.len(), golden.len()));
    }
    if let Some(i) = (0..golden.len()).find(|&i| sec.bytes[i] != golden[i]) {
        return Err(format!(
            "`{name}` first diff at offset {i:#x}: sigil {:#04x} != golden {:#04x}",
            sec.bytes[i], golden[i]
        ));
    }
    Ok(())
}

/// Parse the `name = value` lines of a `stub-syms.toml` body (value may be
/// `0xHEX` or decimal). Blank lines and `#` comments are skipped, but any
/// present, non-comment line that is malformed — no `=`, or a value that parses
/// as neither hex nor decimal — is a hard error (returns `Err` naming the
/// offending line). We never silently drop a stub: a fat-fingered value in the
/// hand-maintained table must surface here, not resurface as a baffling
/// link-time "unresolved symbol".
pub fn parse_stub_syms(text: &str) -> Result<Vec<(String, i64)>, String> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            return Err(format!("malformed stub line (no `=`): {raw:?}"));
        };
        let name = name.trim().to_string();
        let value = value.trim();
        let parsed = if let Some(hex) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X"))
        {
            i64::from_str_radix(hex, 16).ok()
        } else {
            value.parse::<i64>().ok()
        };
        match parsed {
            Some(v) => out.push((name, v)),
            None => return Err(format!("malformed stub value in line: {raw:?}")),
        }
    }
    Ok(out)
}

/// Read `golden/stub-syms.toml` and parse it via [`parse_stub_syms`]. Panics
/// (with the parse error) on a malformed table — acceptable for this test
/// harness helper.
pub fn load_stub_syms(golden_dir: &Path) -> Vec<(String, i64)> {
    let text = std::fs::read_to_string(golden_dir.join("stub-syms.toml"))
        .expect("read golden/stub-syms.toml");
    parse_stub_syms(&text).expect("parse golden/stub-syms.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img_with(name: &str, bytes: Vec<u8>) -> LinkedImage {
        LinkedImage {
            sections: vec![sigil_link::LinkedSection { name: name.to_string(), lma: 0, bytes }],
        }
    }

    #[test]
    fn diff_region_ok_on_match() {
        let img = img_with("sec0", vec![0x01, 0x02, 0x03]);
        assert!(diff_region(&img, "sec0", &[0x01, 0x02, 0x03]).is_ok());
    }

    #[test]
    fn diff_region_reports_first_diff_offset() {
        let img = img_with("sec0", vec![0x01, 0x02, 0xFF, 0x04]);
        let err = diff_region(&img, "sec0", &[0x01, 0x02, 0x03, 0x04]).unwrap_err();
        assert!(err.contains("offset 0x2"), "got: {err}");
        assert!(err.contains("0xff"), "got: {err}");
        assert!(err.contains("0x03"), "got: {err}");
    }

    #[test]
    fn diff_region_reports_length_mismatch() {
        let img = img_with("sec0", vec![0x01, 0x02]);
        let err = diff_region(&img, "sec0", &[0x01, 0x02, 0x03]).unwrap_err();
        assert!(err.contains("length"), "got: {err}");
    }

    #[test]
    fn parse_stub_syms_reads_hex_decimal_comments_and_blanks() {
        let text = "\
# a comment line
SND_ENGINE_TABLE_BANK = 0xc

  Sfx_33 = 0x6553a
DEC_VALUE = 1543
";
        let out = parse_stub_syms(text).unwrap();
        assert_eq!(
            out,
            vec![
                ("SND_ENGINE_TABLE_BANK".to_string(), 0xc),
                ("Sfx_33".to_string(), 0x6553a),
                ("DEC_VALUE".to_string(), 1543),
            ]
        );
    }

    #[test]
    fn parse_stub_syms_errs_on_malformed_value() {
        let err = parse_stub_syms("GOOD = 0x10\nBAD = 0xZZ\n").unwrap_err();
        assert!(err.contains("0xZZ"), "error should name the offending line: {err}");
    }

    #[test]
    fn parse_stub_syms_errs_on_missing_equals() {
        let err = parse_stub_syms("NoEqualsHere\n").unwrap_err();
        assert!(err.contains("NoEqualsHere"), "error should name the line: {err}");
    }

    #[test]
    fn parses_symbol_cells_from_a_listing_row() {
        let lst = " Mod_ReArm :    86F C |  MovingTrucks_Bank_Start :    60000 C |\n";
        let syms = parse_lst_symbols(lst);
        assert_eq!(syms.get("MovingTrucks_Bank_Start"), Some(&0x60000));
        assert_eq!(syms.get("Mod_ReArm"), Some(&0x86F));
    }

    #[test]
    fn skips_non_symbol_lines() {
        // A real code-listing line (address prefix, opcode bytes, colon in it)
        // and a page-header timestamp line must NOT produce phantom symbols.
        let lst = "(2)  122/       0 : C3 3B 00                    jp      SndDrv_Init\n\
                    AS V1.42 Beta [Bld 212] - Page 1 - 07/03/2026 02:24:58 AM\n";
        let syms = parse_lst_symbols(lst);
        assert!(syms.is_empty(), "got phantom symbols: {syms:?}");
    }

    #[test]
    fn skips_cells_with_no_colon_or_bad_hex() {
        let lst = " NoColonHere  |  BadVal : ZZZ C |  Good : 1F C |\n";
        let syms = parse_lst_symbols(lst);
        assert_eq!(syms.len(), 1);
        assert_eq!(syms.get("Good"), Some(&0x1F));
    }

    #[test]
    fn strips_leading_star_marker() {
        let lst = "*Local_Sym :  10 C |\n";
        let syms = parse_lst_symbols(lst);
        assert_eq!(syms.get("Local_Sym"), Some(&0x10));
    }

    #[test]
    fn dotted_and_wide_names_parse() {
        let lst = " Mod_Advance.changed :          94A C |  MovingTrucks_PitchTable_Stream :   636B8 C |\n";
        let syms = parse_lst_symbols(lst);
        assert_eq!(syms.get("Mod_Advance.changed"), Some(&0x94A));
        assert_eq!(syms.get("MovingTrucks_PitchTable_Stream"), Some(&0x636B8));
    }

    #[test]
    fn derives_region_a_from_bracket_labels() {
        let mut syms = BTreeMap::new();
        syms.insert("Z80_Sound_Start".to_string(), 0x3EA);
        syms.insert("Z80_Sound_End".to_string(), 0x1B02);
        let w = derive_region_a(&syms).unwrap();
        assert_eq!(w, RegionWindow { vma_base: 0, lma: 0x3EA, len: 0x1718 });
    }

    #[test]
    fn derives_region_b_from_bank_and_song_anchors() {
        let mut syms = BTreeMap::new();
        syms.insert("MovingTrucks_Bank_Start".to_string(), 0x60000);
        syms.insert("Song_MovingTrucks".to_string(), 0x60607);
        let w = derive_region_b(&syms).unwrap();
        assert_eq!(w, RegionWindow { vma_base: 0x8000, lma: 0x60000, len: 0x607 });
    }
}

#[cfg(test)]
mod lma_tests {
    use super::*;
    use sigil_ir::{Cpu, Section};

    fn sec(name: &str, vma_base: Option<u32>) -> Section {
        Section { name: name.into(), cpu: Cpu::Z80, vma_base, lma: vma_base.unwrap_or(0),
                  labels: vec![], fragments: vec![] }
    }

    #[test]
    fn assigns_region_lmas_by_vma_base() {
        let mut m = Module { sections: vec![sec("sec0", Some(0)), sec("sec32768", Some(0x8000))] };
        let mut map = LmaMap::new();
        map.set(Cpu::Z80, Some(0), 0x400);
        map.set(Cpu::Z80, Some(0x8000), 0x60000);
        assign_lmas(&mut m, &map).unwrap();
        assert_eq!(m.sections[0].lma, 0x400);
        assert_eq!(m.sections[1].lma, 0x60000);
    }

    #[test]
    fn unmapped_section_errors() {
        let mut m = Module { sections: vec![sec("sec0", Some(0))] };
        let err = assign_lmas(&mut m, &LmaMap::new()).unwrap_err();
        assert!(err.contains("no LMA mapping"), "got: {err}");
    }
}

#[cfg(test)]
mod harness_tests {
    use super::*;

    #[test]
    #[ignore = "reads the aeon source tree; run with --ignored"]
    fn harness_assembles_regions_a_and_b_together() {
        let aeon = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../aeon");
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("harness_root.asm");
        let stubs = load_stub_syms(&Path::new(env!("CARGO_MANIFEST_DIR")).join("golden"));
        let mut map = LmaMap::new();
        map.set(Cpu::Z80, Some(0), 0x3EA);
        map.set(Cpu::Z80, Some(0x8000), 0x60000);
        let img = build_harness(&aeon, &root, &stubs, &map).expect("build");
        assert_eq!(img.sections.len(), 2, "expected region A + region B");

        // Shape guard: pin BOTH regions by name + LMA + byte length so a
        // regression that drops a real section and admits a phantom cannot pass.
        // Region A = sec0 (Z80, lma 0x3EA); region B = sec32768 (Z80, lma
        // 0x60000). The exact lengths track the current aeon source (they drift
        // as the driver/tables change), so pin them to the committed golden blobs
        // — which `regen` keeps in lockstep with the same build — rather than to
        // a hard-coded number that silently rots. (`LinkedSection` carries no
        // `cpu`, so the two real Z80 regions are pinned by their names + real
        // LMAs + exact byte lengths; an empty M68000 phantom — also named `sec0`
        // — could satisfy neither the golden length nor the 0x3EA LMA.)
        let g = Path::new(env!("CARGO_MANIFEST_DIR")).join("golden");
        let golden_a = std::fs::read(g.join("region_a.bin")).expect("region_a.bin (run regen)");
        let golden_b = std::fs::read(g.join("region_b.bin")).expect("region_b.bin (run regen)");
        let a = img.section("sec0").expect("region A section sec0");
        let b = img.section("sec32768").expect("region B section sec32768");
        assert_eq!(a.lma, 0x3EA, "region A LMA");
        assert_eq!(b.lma, 0x60000, "region B LMA");
        // The real gate: the LIVE Sigil output must byte-match the committed
        // reference blobs (not merely have the right length). Unlike the hermetic
        // `m0_acceptance` test — which compares two files written by the same
        // regen run and so only proves internal consistency — this asserts the
        // freshly-assembled bytes against the reference extraction, with a
        // first-diff offset on failure.
        diff_region(&img, "sec0", &golden_a).expect("region A diverged from reference");
        diff_region(&img, "sec32768", &golden_b).expect("region B diverged from reference");
    }
}

#[cfg(test)]
mod acceptance_tests {
    use super::*;

    #[test]
    fn m0_acceptance_sigil_matches_reference_blobs() {
        let g = Path::new(env!("CARGO_MANIFEST_DIR")).join("golden");
        let ref_a = std::fs::read(g.join("region_a.bin")).expect("region_a.bin (run regen)");
        let ref_b = std::fs::read(g.join("region_b.bin")).expect("region_b.bin (run regen)");
        let sig_a = std::fs::read(g.join("sigil_a.bin")).expect("sigil_a.bin (run regen)");
        let sig_b = std::fs::read(g.join("sigil_b.bin")).expect("sigil_b.bin (run regen)");
        assert_eq!(sig_a, ref_a, "region A diverged");
        assert_eq!(sig_b, ref_b, "region B diverged");
    }
}
