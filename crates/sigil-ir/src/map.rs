//! External memory map: regions with LMA base/size + default gap fill. A pure
//! type (no I/O); the TOML loader lives in `sigil-link::map_load`.

/// What a region models. Only `Rom` regions contribute image bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionKind {
    Rom,
    M68kRam,
    Z80Bank,
    /// A BUDGET sub-window (e.g. the 64KB object code bank at `$10000`). It does NOT
    /// participate in section containment (`region_for` matches only `Rom`; the sections
    /// live inside the enclosing `Rom` region) — it declares a `[lma_base, lma_base+size)`
    /// budget the link checks a named cursor against (`check_budget`). This is the
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

    /// Enforce a named BUDGET region's cap: `cursor_lma` (a link-resolved end marker,
    /// e.g. the object bank's `__BUDGET_DATA`) must not exceed `lma_base + size`. Returns
    /// the used byte count (`cursor_lma - lma_base`) on success. A region that is absent
    /// or not an `ObjectBank` is a NO-OP (`Ok(0)`) — the check is additive, so a map that
    /// declares no budget simply isn't guarded. Overflow is an actionable Err naming the
    /// region, its cap, and the overshoot (the AS `\{*-$20000}` message, map-owned).
    pub fn check_budget(&self, region_name: &str, cursor_lma: u32) -> Result<u32, String> {
        let Some(r) = self.region_by_name(region_name) else {
            return Ok(0);
        };
        if r.kind != RegionKind::ObjectBank {
            return Ok(0);
        }
        let cap = r.lma_base as u64 + r.size as u64;
        if cursor_lma as u64 > cap {
            return Err(format!(
                "budget region `{}` [{:#X},{cap:#X}) overflows, cursor {cursor_lma:#X} over by {} bytes",
                r.name, r.lma_base, cursor_lma as u64 - cap
            ));
        }
        Ok(cursor_lma.saturating_sub(r.lma_base))
    }

    /// Verify a section `[lma, lma+len)` lies entirely within one `Rom` region.
    /// A refusal names what the LMA did land in: the non-ROM region holding it
    /// when one does (work RAM, a Z80 bank), else every ROM window it could have
    /// used.
    pub fn validate_section(&self, name: &str, lma: u32, len: u32) -> Result<(), String> {
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
        let end = lma as u64 + len as u64;
        let region_end = r.lma_base as u64 + r.size as u64;
        if end > region_end {
            // §7.3: report the overflow amount ("over by N bytes") so the budget
            // miss is actionable, naming the region and its end address.
            return Err(format!(
                "section `{name}` [{lma:#X},{end:#X}) overflows region `{}` (ends {region_end:#X}), over by {} bytes",
                r.name, end - region_end
            ));
        }
        Ok(())
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
        assert!(m.validate_section("ok", 0, 0x1000).is_ok());
        assert!(m.validate_section("over", 0xF00, 0x200).is_err());
        assert!(m.validate_section("outside", 0x2000, 4).is_err());
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
        let err = m.validate_section("s", M68K_RAM_LMA_BASE, 1).unwrap_err();
        assert_eq!(
            err,
            "section `s` LMA 0xFF0000 lies in non-ROM region `work_ram` (m68k_ram) [0xFF0000,0x1000000); its 1 byte(s) have no place in the image"
        );
    }

    #[test]
    fn a_refusal_outside_every_region_lists_the_rom_windows() {
        let m = MemoryMap::mega_drive();
        let err = m.validate_section("s", 0xFFFF_FFFF, 1).unwrap_err();
        assert_eq!(err, "section `s` LMA 0xFFFFFFFF is in no ROM region; ROM regions: `cartridge` [0x0,0x400000)");
        let bare = MemoryMap::new(vec![], 0x00);
        assert_eq!(bare.validate_section("s", 0, 1).unwrap_err(), "section `s` LMA 0x0 is in no ROM region; ROM regions: (none)");
    }

    #[test]
    fn the_last_cartridge_byte_fits_and_the_next_overflows_by_one() {
        let m = MemoryMap::mega_drive();
        let last = CARTRIDGE_LMA_BASE + CARTRIDGE_SIZE - 1;
        assert!(m.validate_section("s", last, 1).is_ok());
        let err = m.validate_section("s", last, 2).unwrap_err();
        assert_eq!(err, "section `s` [0x3FFFFF,0x400001) overflows region `cartridge` (ends 0x400000), over by 1 bytes");
    }

    fn objbank(base: u32, size: u32) -> Region {
        Region { name: "object_bank".into(), lma_base: base, size, kind: RegionKind::ObjectBank, vma_base: None }
    }

    #[test]
    fn check_budget_reports_usage_and_catches_overflow() {
        // The object bank at $10000, 64KB — a cursor at $1128C uses $128C, well within.
        let m = MemoryMap::new(vec![rom("rom", 0, 0x400000), objbank(0x10000, 0x10000)], 0x00);
        assert_eq!(m.check_budget("object_bank", 0x1128C).unwrap(), 0x128C);
        assert_eq!(m.check_budget("object_bank", 0x20000).unwrap(), 0x10000); // exact fit
        let err = m.check_budget("object_bank", 0x20002).unwrap_err();
        assert!(err.contains("over by 2 bytes"), "{err}");
        // An ObjectBank region does NOT participate in section containment.
        assert_eq!(m.region_for(0x10000).unwrap().name, "rom");
        // A map with no budget region is a no-op.
        let bare = MemoryMap::new(vec![rom("rom", 0, 0x400000)], 0x00);
        assert_eq!(bare.check_budget("object_bank", 0x99999).unwrap(), 0);
    }
}
