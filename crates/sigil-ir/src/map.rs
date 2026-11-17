//! External memory map: regions with LMA base/size + default gap fill. A pure
//! type (no I/O); the TOML loader lives in `sigil-link::map_load`.

/// What a region models. Only `Rom` regions contribute image bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionKind {
    Rom,
    M68kRam,
    /// A Z80 `SetBank` window (`$8000` bytes of cartridge the Z80 reads through its
    /// bank register). A CONTAINER for whatever starts in it: a section whose first
    /// byte lies in the window must end in it (`validate_section` rule 2). It places
    /// no image bytes of its own; the enclosing `Rom` region does.
    Z80Bank,
    /// A BUDGET sub-window (e.g. the 64KB object code bank at `$10000`). It does NOT
    /// participate in section containment (`region_for` matches only `Rom`; the sections
    /// live inside the enclosing `Rom` region) — it declares a `[lma_base, lma_base+size)`
    /// budget the link checks a `[[budget]]` row's cursor against (`check_budget`). This is the
    /// map-owned form of the AS-side `if * > $20000 / error` object-bank overflow guard.
    ObjectBank,
}

impl RegionKind {
    /// The map-file spelling of this kind (the `kind = "..."` value `map_load` reads).
    pub fn label(self) -> &'static str {
        match self {
            RegionKind::Rom => "rom",
            RegionKind::M68kRam => "m68k_ram",
            RegionKind::Z80Bank => "z80_bank",
            RegionKind::ObjectBank => "object_bank",
        }
    }
}

/// The 68000 cartridge window, `0x000000..=0x3FFFFF`: the address space a flat ROM
/// image occupies, and the bound every flat image is validated against. The
/// shipped game maps declare their `rom` region with this same base and size.
pub const CARTRIDGE_LMA_BASE: u32 = 0x00_0000;
/// Size of [`CARTRIDGE_LMA_BASE`]'s window (4 MiB).
pub const CARTRIDGE_SIZE: u32 = 0x40_0000;
/// The 68000 work RAM window, `0xFF0000..=0xFFFFFF`. A byte placed here has no
/// image to land in.
pub const M68K_RAM_LMA_BASE: u32 = 0xFF_0000;
/// Size of [`M68K_RAM_LMA_BASE`]'s window (64 KiB).
pub const M68K_RAM_SIZE: u32 = 0x1_0000;

/// One declared region. `vma_base` records a phased VMA≠LMA relationship
/// (informational in B; sections still carry their own `vma_base`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Region {
    pub name: String,
    pub lma_base: u32,
    pub size: u32,
    pub kind: RegionKind,
    pub vma_base: Option<u32>,
}

impl Region {
    /// Whether `lma` lies in this region's `[lma_base, lma_base+size)` window.
    pub fn contains(&self, lma: u32) -> bool {
        lma >= self.lma_base && (lma - self.lma_base) < self.size
    }
}

/// The whole map, in ROM output order, plus the default gap-fill byte.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryMap {
    pub regions: Vec<Region>,
    pub fill: u8,
}

impl MemoryMap {
    pub fn new(regions: Vec<Region>, fill: u8) -> Self {
        MemoryMap { regions, fill }
    }

    /// The Mega Drive's fixed address windows as a map: the cartridge
    /// (`cartridge`, the one ROM region) and 68000 work RAM (`work_ram`, which
    /// holds no image bytes), gap fill `0x00`. This is the bound a flat image is
    /// validated against when no game map names its regions.
    pub fn mega_drive() -> Self {
        MemoryMap::new(
            vec![
                Region {
                    name: "cartridge".to_string(),
                    lma_base: CARTRIDGE_LMA_BASE,
                    size: CARTRIDGE_SIZE,
                    kind: RegionKind::Rom,
                    vma_base: None,
                },
                Region {
                    name: "work_ram".to_string(),
                    lma_base: M68K_RAM_LMA_BASE,
                    size: M68K_RAM_SIZE,
                    kind: RegionKind::M68kRam,
                    vma_base: None,
                },
            ],
            0x00,
        )
    }

    /// The ROM region whose `[lma_base, lma_base+size)` contains `lma`.
    pub fn region_for(&self, lma: u32) -> Option<&Region> {
        self.regions.iter().find(|r| r.kind == RegionKind::Rom && r.contains(lma))
    }

    /// A region by exact name (any kind).
    pub fn region_by_name(&self, name: &str) -> Option<&Region> {
        self.regions.iter().find(|r| r.name == name)
    }

    /// Enforce one declared budget: `cursor_lma` (a link-resolved end marker, e.g. the
    /// object bank's data-region head) must not exceed `ceiling`, the `[[budget]]`
    /// row's declared cap. Returns the used byte count (`cursor_lma - lma_base`) on
    /// success. The region must exist, the ceiling must lie inside it, and the cursor must
    /// not lie below its base: each of those budgets cannot be measured, and is refused
    /// (`[map.budget-unmeasurable]`) rather than read as within budget.
    /// Overflow is an actionable Err naming the region, its cap, and the overshoot
    /// (the AS `\{*-$20000}` message, map-owned).
    pub fn check_budget(&self, region_name: &str, ceiling: u32, cursor_lma: u32) -> Result<u32, String> {
        let Some(r) = self.region_by_name(region_name) else {
            return Err(format!(
                "[map.budget-unmeasurable] budget row `{region_name}` names no map region, so its ceiling {ceiling:#X} cannot be measured"
            ));
        };
        let region_end = r.lma_base as u64 + r.size as u64;
        if ceiling as u64 > region_end {
            return Err(format!(
                "[map.budget-unmeasurable] budget row `{}` ceiling {ceiling:#X} is past its region's end {region_end:#X}; a budget caps bytes inside its region",
                r.name
            ));
        }
        if cursor_lma < r.lma_base {
            return Err(format!(
                "[map.budget-unmeasurable] budget row `{}` cursor {cursor_lma:#X} is below its region's base {:#X}, so its use cannot be measured",
                r.name, r.lma_base
            ));
        }
        if cursor_lma > ceiling {
            return Err(format!(
                "budget region `{}` [{:#X},{ceiling:#X}) overflows, cursor {cursor_lma:#X} over by {} bytes",
                r.name,
                r.lma_base,
                cursor_lma - ceiling
            ));
        }
        Ok(cursor_lma - r.lma_base)
    }

    /// Verify a section `[lma, lma+len)` against the map. Three rules, each one a
    /// refusal in the form ``section `{name}` [{lma},{end}) overflows region
    /// `{region}` (ends {region_end}), over by {n} bytes``, checked in order:
    ///
    /// 1. `placed_in` names a map region (any kind): the section must lie inside THAT
    ///    region. `place_sections` records the region a section was placed into as
    ///    its `group`; a group that names no region (the native declared chain's
    ///    `declchain{n}`) is ignored. Without this a spill packed at the region's end
    ///    is checked against whatever region starts there, and an overflow of a region
    ///    nested in an earlier, wider `rom` region is checked against the wider one.
    /// 2. every `Z80Bank` region whose window holds `lma` must hold the section's
    ///    end: a section that starts in a Z80 `SetBank` window and crosses its top is
    ///    read through the wrong bank at runtime. A section starting exactly at the
    ///    window's end is outside it.
    /// 3. the first `Rom` region holding `lma` must hold the end (only `Rom` regions
    ///    place image bytes).
    ///
    /// `ObjectBank` regions are budgets ([`Self::check_budget`]), never containers:
    /// apart from rule 1 they hold nothing (data runs on past the object bank's end).
    /// A refusal under rule 3 for an LMA no ROM region holds names what it did land
    /// in: the non-ROM region holding it when one does (work RAM, a Z80 bank), else
    /// every ROM window it could have used.
    pub fn validate_section(&self, name: &str, placed_in: Option<&str>, lma: u32, len: u32) -> Result<(), String> {
        let end = lma as u64 + len as u64;
        let held = |r: &Region| -> Result<(), String> {
            let region_end = r.lma_base as u64 + r.size as u64;
            if lma < r.lma_base {
                return Err(format!(
                    "section `{name}` [{lma:#X},{end:#X}) starts below region `{}` (base {:#X}), under by {} bytes",
                    r.name,
                    r.lma_base,
                    r.lma_base - lma
                ));
            }
            if end > region_end {
                // §7.3: report the overflow amount ("over by N bytes") so the budget
                // miss is actionable, naming the region and its end address.
                return Err(format!(
                    "section `{name}` [{lma:#X},{end:#X}) overflows region `{}` (ends {region_end:#X}), over by {} bytes",
                    r.name,
                    end - region_end
                ));
            }
            Ok(())
        };
        // Rule 1: the region the section was placed into.
        if let Some(r) = placed_in.and_then(|g| self.region_by_name(g)) {
            held(r)?;
        }
        // Rule 2: every Z80 bank window the section starts in.
        for r in self.regions.iter().filter(|r| r.kind == RegionKind::Z80Bank && r.contains(lma)) {
            held(r)?;
        }
        // Rule 3: the ROM region that holds the first byte.
        let Some(r) = self.region_for(lma) else {
            if let Some(other) = self.regions.iter().find(|r| r.kind != RegionKind::Rom && r.contains(lma)) {
                return Err(format!(
                    "section `{name}` LMA {lma:#X} lies in non-ROM region `{}` ({}) [{:#X},{:#X}); its {len} byte(s) have no place in the image",
                    other.name,
                    other.kind.label(),
                    other.lma_base,
                    other.lma_base as u64 + other.size as u64
                ));
            }
            let windows: Vec<String> = self
                .regions
                .iter()
                .filter(|r| r.kind == RegionKind::Rom)
                .map(|r| format!("`{}` [{:#X},{:#X})", r.name, r.lma_base, r.lma_base as u64 + r.size as u64))
                .collect();
            let windows = if windows.is_empty() { "(none)".to_string() } else { windows.join(", ") };
            return Err(format!("section `{name}` LMA {lma:#X} is in no ROM region; ROM regions: {windows}"));
        };
        held(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom(name: &str, base: u32, size: u32) -> Region {
        Region { name: name.into(), lma_base: base, size, kind: RegionKind::Rom, vma_base: None }
    }

    #[test]
    fn region_for_finds_containing_rom_region() {
        let m = MemoryMap::new(vec![rom("main", 0, 0x8000), rom("bank", 0x8000, 0x8000)], 0x00);
        assert_eq!(m.region_for(0x100).unwrap().name, "main");
        assert_eq!(m.region_for(0x9000).unwrap().name, "bank");
        assert!(m.region_for(0x1_0000).is_none());
    }

    #[test]
    fn validate_section_rejects_region_overflow() {
        let m = MemoryMap::new(vec![rom("main", 0, 0x1000)], 0x00);
        assert!(m.validate_section("ok", None, 0, 0x1000).is_ok());
        assert!(m.validate_section("over", None, 0xF00, 0x200).is_err());
        assert!(m.validate_section("outside", None, 0x2000, 4).is_err());
    }

    #[test]
    fn mega_drive_map_names_the_cartridge_and_work_ram_windows() {
        let m = MemoryMap::mega_drive();
        assert_eq!(m.region_for(CARTRIDGE_LMA_BASE).unwrap().name, "cartridge");
        assert_eq!(m.region_for(CARTRIDGE_LMA_BASE + CARTRIDGE_SIZE - 1).unwrap().name, "cartridge");
        assert!(m.region_for(CARTRIDGE_LMA_BASE + CARTRIDGE_SIZE).is_none());
        assert!(m.region_for(M68K_RAM_LMA_BASE).is_none(), "work RAM is not a ROM region");
        assert_eq!(m.region_by_name("work_ram").unwrap().kind, RegionKind::M68kRam);
        assert_eq!(m.fill, 0x00);
    }

    #[test]
    fn a_refusal_names_the_non_rom_region_holding_the_lma() {
        let m = MemoryMap::mega_drive();
        let err = m.validate_section("s", None, M68K_RAM_LMA_BASE, 1).unwrap_err();
        assert_eq!(
            err,
            "section `s` LMA 0xFF0000 lies in non-ROM region `work_ram` (m68k_ram) [0xFF0000,0x1000000); its 1 byte(s) have no place in the image"
        );
    }

    #[test]
    fn a_refusal_outside_every_region_lists_the_rom_windows() {
        let m = MemoryMap::mega_drive();
        let err = m.validate_section("s", None, 0xFFFF_FFFF, 1).unwrap_err();
        assert_eq!(err, "section `s` LMA 0xFFFFFFFF is in no ROM region; ROM regions: `cartridge` [0x0,0x400000)");
        let bare = MemoryMap::new(vec![], 0x00);
        assert_eq!(bare.validate_section("s", None, 0, 1).unwrap_err(), "section `s` LMA 0x0 is in no ROM region; ROM regions: (none)");
    }

    #[test]
    fn the_last_cartridge_byte_fits_and_the_next_overflows_by_one() {
        let m = MemoryMap::mega_drive();
        let last = CARTRIDGE_LMA_BASE + CARTRIDGE_SIZE - 1;
        assert!(m.validate_section("s", None, last, 1).is_ok());
        let err = m.validate_section("s", None, last, 2).unwrap_err();
        assert_eq!(err, "section `s` [0x3FFFFF,0x400001) overflows region `cartridge` (ends 0x400000), over by 1 bytes");
    }

    fn objbank(base: u32, size: u32) -> Region {
        Region { name: "object_bank".into(), lma_base: base, size, kind: RegionKind::ObjectBank, vma_base: None }
    }

    #[test]
    fn check_budget_reports_usage_and_catches_overflow() {
        // The object bank at $10000, ceiling $20000 — a cursor at $1128C uses $128C.
        let m = MemoryMap::new(vec![rom("rom", 0, 0x400000), objbank(0x10000, 0x10000)], 0x00);
        assert_eq!(m.check_budget("object_bank", 0x20000, 0x1128C).unwrap(), 0x128C);
        assert_eq!(m.check_budget("object_bank", 0x20000, 0x20000).unwrap(), 0x10000); // exact fit
        let err = m.check_budget("object_bank", 0x20000, 0x20002).unwrap_err();
        assert_eq!(err, "budget region `object_bank` [0x10000,0x20000) overflows, cursor 0x20002 over by 2 bytes");
        // The cap is the declared ceiling, not the region's end.
        assert!(m.check_budget("object_bank", 0x18000, 0x18001).is_err());
        // An ObjectBank region does NOT participate in section containment.
        assert_eq!(m.region_for(0x10000).unwrap().name, "rom");
        // A budget against a region the map does not declare is unmeasurable: refused.
        let bare = MemoryMap::new(vec![rom("rom", 0, 0x400000)], 0x00);
        let err = bare.check_budget("object_bank", 0x20000, 0x99999).unwrap_err();
        assert!(err.starts_with("[map.budget-unmeasurable] budget row `object_bank`"), "{err}");
    }

    #[test]
    fn a_budget_cursor_below_its_region_is_unmeasurable() {
        let m = MemoryMap::new(vec![rom("rom", 0, 0x400000), objbank(0x10000, 0x10000)], 0x00);
        let err = m.check_budget("object_bank", 0x20000, 0xFFFE).unwrap_err();
        assert_eq!(
            err,
            "[map.budget-unmeasurable] budget row `object_bank` cursor 0xFFFE is below its region's base 0x10000, so its use cannot be measured"
        );
        assert_eq!(m.check_budget("object_bank", 0x20000, 0x10000).unwrap(), 0, "a cursor at the base uses 0");
    }

    #[test]
    fn a_budget_ceiling_past_its_region_is_unmeasurable() {
        let m = MemoryMap::new(vec![rom("rom", 0, 0x400000), objbank(0x10000, 0x10000)], 0x00);
        let err = m.check_budget("object_bank", 0x20001, 0x11000).unwrap_err();
        assert_eq!(
            err,
            "[map.budget-unmeasurable] budget row `object_bank` ceiling 0x20001 is past its region's end 0x20000; a budget caps bytes inside its region"
        );
        assert!(m.check_budget("object_bank", 0x20000, 0x11000).is_ok(), "the region's end is a legal ceiling (both games)");
    }

    #[test]
    fn a_placed_section_below_its_region_is_refused() {
        let m = MemoryMap::new(vec![rom("rom", 0, 0x8000), rom("drv_pcm", 0x300, 4)], 0x00);
        assert_eq!(
            m.validate_section("drv_pcm", Some("drv_pcm"), 0x2FE, 4).unwrap_err(),
            "section `drv_pcm` [0x2FE,0x302) starts below region `drv_pcm` (base 0x300), under by 2 bytes"
        );
    }

    fn z80bank(name: &str, base: u32, size: u32) -> Region {
        Region { name: name.into(), lma_base: base, size, kind: RegionKind::Z80Bank, vma_base: Some(0x8000) }
    }

    /// The `z80probe` z3 shape: a whole-window `rom` region listed FIRST, then the
    /// small `drv_pcm` region nested inside it. The named region is the one held.
    #[test]
    fn validate_placed_checks_the_named_region() {
        let m = MemoryMap::new(vec![rom("rom", 0, 0x8000), rom("drv_pcm", 0x300, 4)], 0x00);
        assert_eq!(
            m.validate_section("drv_pcm", Some("drv_pcm"), 0x300, 7).unwrap_err(),
            "section `drv_pcm` [0x300,0x307) overflows region `drv_pcm` (ends 0x304), over by 3 bytes"
        );
        assert!(m.validate_section("drv_pcm", Some("drv_pcm"), 0x300, 4).is_ok(), "exact fill");
        // A group that names no region (the native chain's `declchain{n}`) is no region.
        assert!(m.validate_section("drv_pcm", Some("declchain3"), 0x300, 7).is_ok());
    }

    /// A spill packed at the named region's end, which is the next region's base, is
    /// outside the region it was placed in.
    #[test]
    fn validate_placed_refuses_a_spill_into_the_next_region() {
        let m = MemoryMap::new(vec![rom("a", 0x1000, 0x400), rom("b", 0x1400, 0x400)], 0x00);
        assert_eq!(
            m.validate_section("a", Some("a"), 0x1400, 0x10).unwrap_err(),
            "section `a` [0x1400,0x1410) overflows region `a` (ends 0x1400), over by 16 bytes"
        );
    }

    /// Bank 1 at aeon HEAD runs `$B8000-$BDC5C`; a section starting at its end that
    /// crosses `$C0000` would be read through the wrong Z80 window at runtime.
    #[test]
    fn z80_bank_containment_refuses_a_section_crossing_the_window() {
        let m = MemoryMap::new(vec![rom("rom", 0, 0x400000), z80bank("z80_sound_bank", 0xB8000, 0x8000)], 0x00);
        let err = m.validate_section("sfx", None, 0xBDC5C, 0x3000).unwrap_err();
        assert!(
            err.ends_with("overflows region `z80_sound_bank` (ends 0xC0000), over by 3164 bytes"),
            "{err}"
        );
        assert_eq!(err, "section `sfx` [0xBDC5C,0xC0C5C) overflows region `z80_sound_bank` (ends 0xC0000), over by 3164 bytes");
    }

    #[test]
    fn z80_bank_exact_fill_is_accepted() {
        let m = MemoryMap::new(vec![rom("rom", 0, 0x400000), z80bank("z80_sound_bank", 0xB8000, 0x8000)], 0x00);
        assert!(m.validate_section("b1", None, 0xB8000, 0x8000).is_ok());
        assert!(m.validate_section("b1", None, 0xB8000, 0x8001).is_err(), "one byte more crosses");
    }

    #[test]
    fn a_section_starting_at_the_window_end_is_outside_it() {
        let m = MemoryMap::new(vec![rom("rom", 0, 0x400000), z80bank("z80_sound_bank", 0xB8000, 0x8000)], 0x00);
        assert!(m.validate_section("SongBank2_Head", None, 0xC0000, 0x100).is_ok());
    }

    /// `OJZ_Sec8_Blocks` starts at `$1FF86` and runs past `$20000` today: the object
    /// bank is a budget (`check_budget`), never a container.
    #[test]
    fn object_bank_is_a_budget_not_a_container() {
        let m = MemoryMap::new(vec![rom("rom", 0, 0x400000), objbank(0x10000, 0x10000)], 0x00);
        assert!(m.validate_section("OJZ_Sec8_Blocks", None, 0x1FF86, 0x1660).is_ok());
    }
}
