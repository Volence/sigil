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

    /// The ROM region whose `[lma_base, lma_base+size)` contains `lma`.
    pub fn region_for(&self, lma: u32) -> Option<&Region> {
        self.regions.iter().find(|r| {
            r.kind == RegionKind::Rom && lma >= r.lma_base && (lma - r.lma_base) < r.size
        })
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
    pub fn validate_section(&self, name: &str, lma: u32, len: u32) -> Result<(), String> {
        let Some(r) = self.region_for(lma) else {
            return Err(format!("section `{name}` LMA {lma:#X} is in no ROM region"));
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
