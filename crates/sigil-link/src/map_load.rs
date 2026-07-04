//! TOML → `MemoryMap`. The external-config seam (`sigil.map.toml`). The pure
//! `MemoryMap` type stays in sigil-ir; deserialization lives here.

use serde::Deserialize;
use sigil_ir::map::{MemoryMap, Region, RegionKind};

#[derive(Deserialize)]
struct MapDoc {
    #[serde(default = "default_fill")]
    fill: u8,
    #[serde(default)]
    region: Vec<RegionDoc>,
}

#[derive(Deserialize)]
struct RegionDoc {
    name: String,
    lma_base: u32,
    size: u32,
    kind: String,
    #[serde(default)]
    vma_base: Option<u32>,
}

fn default_fill() -> u8 { 0x00 }

/// Parse a `sigil.map.toml` string into a `MemoryMap`. Regions keep source order
/// (the ROM output order).
pub fn load_map(toml_src: &str) -> Result<MemoryMap, String> {
    let doc: MapDoc = toml::from_str(toml_src).map_err(|e| format!("map parse error: {e}"))?;
    let mut regions = Vec::new();
    for r in doc.region {
        let kind = match r.kind.as_str() {
            "rom" => RegionKind::Rom,
            "m68k_ram" => RegionKind::M68kRam,
            "z80_bank" => RegionKind::Z80Bank,
            other => return Err(format!("region `{}`: unknown kind `{other}`", r.name)),
        };
        regions.push(Region { name: r.name, lma_base: r.lma_base, size: r.size, kind, vma_base: r.vma_base });
    }
    Ok(MemoryMap::new(regions, doc.fill))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_regions_in_order_with_default_fill() {
        let src = r#"
fill = 0x00
[[region]]
name = "rom"
lma_base = 0
size = 0x400000
kind = "rom"
[[region]]
name = "z80_bank"
lma_base = 0x60000
size = 0x8000
kind = "z80_bank"
vma_base = 0x8000
"#;
        let m = load_map(src).unwrap();
        assert_eq!(m.fill, 0x00);
        assert_eq!(m.regions.len(), 2);
        assert_eq!(m.regions[0].name, "rom");
        assert_eq!(m.regions[1].vma_base, Some(0x8000));
    }

    #[test]
    fn rejects_unknown_kind() {
        let src = "[[region]]\nname=\"x\"\nlma_base=0\nsize=1\nkind=\"bogus\"\n";
        assert!(load_map(src).is_err());
    }
}
