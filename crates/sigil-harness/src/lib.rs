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

#[cfg(test)]
mod tests {
    use super::*;

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
