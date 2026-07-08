//! External memory map: regions with LMA base/size + default gap fill. A pure
//! type (no I/O); the TOML loader lives in `sigil-link::map_load`.

/// What a region models. Only `Rom` regions contribute image bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionKind {
    Rom,
    M68kRam,
    Z80Bank,
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
                "section `{name}` [{lma:#X},{end:#X}) overflows region `{}` (ends {region_end:#X}) — over by {} bytes",
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
}
