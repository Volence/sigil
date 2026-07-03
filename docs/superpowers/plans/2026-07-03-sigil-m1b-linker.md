# Sigil M1.B — Full Linker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Grow `sigil-link` from the M0 Z80-blob linker into the full linker that turns a converged 68k `Module` into a correct single-image ROM: memory map, 68k fixup resolution, `jmp`/`jsr` width fixpoint, header checksum, `convsym` no-op, and the `s4.lst` symbol listing.

**Architecture:** Additive on the working M0 spine. New `resolve_layout` fixpoint lowers the one length-variable fragment (bare-symbol `jmp`/`jsr`) into concrete `DataFragment`s **before** the existing `link()` runs, so `link()`/`Section`/`Fragment` invariants stay clean. 68k fixups mirror the Z80 placeholder+patch idiom (`Z80JrRel8`/`BankPtr16Le`). `emit_rom` replaces `p2bin`+`fixheader`; `emit_listing` replaces the `s4.lst` half of the toolchain. asl is the oracle for every byte.

**Tech Stack:** Rust workspace (10 crates). `serde`+`toml` added to `sigil-link` for the map loader. asl 1.42 (`aeon/tools/asl`) + `p2bin` as the golden oracle (native, no Wine).

**Design doc:** `docs/superpowers/specs/2026-07-03-sigil-m1b-linker-design.md`

---

## File Structure

| File | Responsibility | Tasks |
|---|---|---|
| `crates/sigil-ir/src/fixup.rs` (modify) | New `FixupKind` variants + `byte_width` | 1 |
| `crates/sigil-ir/src/map.rs` (create) | `MemoryMap`/`Region`/`RegionKind` pure types | 4 |
| `crates/sigil-ir/src/lib.rs` (modify) | `Fragment::JmpJsrSym` variant + helper arms; `pub mod map` | 4, 7 |
| `crates/sigil-link/src/lib.rs` (modify) | 68k `apply_fixup` arms; `emit_rom`; `emit_listing` re-export | 2, 3, 5, 6 |
| `crates/sigil-link/src/relax.rs` (create) | `asl_width_rule`, `resolve_layout` fixpoint | 9, 10 |
| `crates/sigil-link/src/listing.rs` (create) | `ListingSymbol`, `emit_listing` (s4.lst) | 11 |
| `crates/sigil-link/src/map_load.rs` (create) | TOML → `MemoryMap` loader | 12 |
| `crates/sigil-backend-m68k/src/lib.rs` (modify) | `lower_jmp_jsr_sym`, `lower_branch`, `lower_pcrel_ea` | 8 |
| `crates/sigil-isa/tests/corpus_m68k/mod.rs` (modify) | width-selection + branch snippets in the shared corpus | 9 |
| `crates/sigil-harness/**` (modify) | integration gate: 68k corpus vs asl, checksum vs pinned ref, s4.lst vs tools | 13 |
| `.github/workflows/ci.yml` (create) | test + clippy + crate-graph | 13 |
| `sigil.map.toml` (create, repo root) | canonical Aeon map example | 12 |

**Naming locked (used across tasks):** `FixupKind::{PcRel8, PcRelDisp16, PcRelDisp8, HeaderChecksum}`; `MemoryMap`, `Region`, `RegionKind::{Rom, M68kRam, Z80Bank}`; `Fragment::JmpJsrSym { is_jsr, target, span }`; `AbsWidth::{W, L}`; `asl_width_rule(target: i64, dash_a: bool) -> AbsWidth`; `resolve_layout(sections, stubs, dash_a) -> Result<Vec<Section>, Vec<Diagnostic>>`; `emit_rom(image, map) -> Result<Vec<u8>, String>`; `apply_header_checksum(rom: &mut [u8])`; `ListingSymbol { name, value, is_equate, unused }`; `emit_listing(&[ListingSymbol]) -> String`.

**68k reference rules (encoded once, cited by tasks):**
- `abs.w` operand = 2 bytes (`4EF8`+word for `jmp`, `4EB8`+word for `jsr`); `abs.l` operand = 4 bytes (`4EF9`+long / `4EB9`+long). So `jmp`/`jsr` fragment length is 4 (`.w`) or 6 (`.l`).
- PC-relative displacement reference = **VMA of the extension word holding the displacement**. For `PcRelDisp16`/`PcRelDisp8`: `disp = target − site_vma` (the disp bytes' own VMA). For `PcRel8` (branch disp in the *opcode* word's low byte): PC ref is `op+2` but the byte is at `op+1`, so `disp = target − (site_vma + 1)` — identical to the existing `Z80JrRel8` formula.

---

## Task 1: New `FixupKind` variants

**Files:**
- Modify: `crates/sigil-ir/src/fixup.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `crates/sigil-ir/src/fixup.rs`:

```rust
    #[test]
    fn byte_width_of_new_68k_kinds() {
        assert_eq!(FixupKind::PcRel8.byte_width(), 1);
        assert_eq!(FixupKind::PcRelDisp16.byte_width(), 2);
        assert_eq!(FixupKind::PcRelDisp8.byte_width(), 1);
        assert_eq!(FixupKind::HeaderChecksum.byte_width(), 2);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sigil-ir fixup::`
Expected: FAIL — `no variant named PcRel8`.

- [ ] **Step 3: Add the variants and widths**

In the `FixupKind` enum (after `Abs32Be`):

```rust
    /// 8-bit branch displacement in the opcode low byte (`bra.s`/`bsr.s`/`Bcc.s`).
    /// `disp = target - (site_vma + 1)` (PC ref = op+2, byte at op+1); range i8.
    PcRel8,
    /// 16-bit displacement in an extension word (`bra.w`/`bsr.w`/`Bcc.w`, `(d16,PC)`).
    /// `disp = target - site_vma` (the disp word's own VMA); range i16, big-endian.
    PcRelDisp16,
    /// 8-bit displacement in a brief extension word (`(d8,PC,Xn)`).
    /// `disp = target - site_vma`; range i8.
    PcRelDisp8,
    /// Synthetic Sega header checksum. Applied as a final post-image pass over
    /// the whole file, NOT through `apply_fixup`; present here for `byte_width`
    /// bookkeeping and future in-fragment modelling.
    HeaderChecksum,
```

In `byte_width`:

```rust
            FixupKind::BankPtr16Le | FixupKind::Abs16Be => 2,
            FixupKind::PcRelDisp16 | FixupKind::HeaderChecksum => 2,
            FixupKind::Z80JrRel8 | FixupKind::PcRel8 | FixupKind::PcRelDisp8 => 1,
            FixupKind::Abs32Be => 4,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sigil-ir fixup::`
Expected: PASS (both `byte_width_matches_kind` and `byte_width_of_new_68k_kinds`).

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-ir/src/fixup.rs
git commit -m "feat(sigil-ir): add PcRel8/PcRelDisp16/PcRelDisp8/HeaderChecksum fixup kinds"
```

---

## Task 2: Resolve `Abs16Be` / `Abs32Be`

**Files:**
- Modify: `crates/sigil-link/src/lib.rs` (the `Abs16Be | Abs32Be` arm of `apply_fixup`, currently a "not supported in M0" diagnostic)

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `crates/sigil-link/src/lib.rs`:

```rust
    #[test]
    fn abs32be_writes_big_endian_target_vma() {
        // A 4-byte data fragment; Abs32Be fixup at offset 0 targeting VMA 0x00123456.
        let mut stubs = SymbolTable::new();
        stubs.define("T", SymbolValue::Int(0x0012_3456));
        let sec = Section {
            name: "s".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x400,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0, 0, 0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("T".into()) }],
                span: span(),
            })],
        };
        let linked = link(&[sec], &stubs).unwrap();
        assert_eq!(linked.section("s").unwrap().bytes, vec![0x00, 0x12, 0x34, 0x56]);
    }

    #[test]
    fn abs16be_writes_big_endian_and_rejects_overflow() {
        let mut stubs = SymbolTable::new();
        stubs.define("Ok", SymbolValue::Int(0x1234));
        stubs.define("Big", SymbolValue::Int(0x1_0000)); // does not fit abs.w sign-extension
        let ok = Section {
            name: "ok".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x400, labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs16Be, offset: 0, target: Expr::Sym("Ok".into()) }],
                span: span(),
            })],
        };
        assert_eq!(link(&[ok], &stubs).unwrap().section("ok").unwrap().bytes, vec![0x12, 0x34]);

        let bad = Section {
            name: "bad".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x400, labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs16Be, offset: 0, target: Expr::Sym("Big".into()) }],
                span: span(),
            })],
        };
        let err = link(&[bad], &stubs).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("abs.w")), "got: {:?}", err);
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sigil-link abs`
Expected: FAIL — current arm pushes "not supported in M0".

- [ ] **Step 3: Replace the `Abs16Be | Abs32Be` arm**

Replace the whole `FixupKind::Abs16Be | FixupKind::Abs32Be => { ... }` arm in `apply_fixup` with:

```rust
        FixupKind::Abs16Be => {
            // abs.w holds a sign-extended 16-bit address: the VMA must fit i16
            // (asl errors otherwise; matching that keeps us byte-exact).
            let v = value as i64;
            if !(-0x8000..=0x7FFF).contains(&v) && !(0xFF_8000..=0xFF_FFFF).contains(&(v & 0xFF_FFFF)) {
                diags.push(diag(
                    format!("value {v:#X} does not fit abs.w (16-bit sign-extended) in section {section}"),
                    span,
                ));
                return;
            }
            let w = (value & 0xFFFF) as u16;
            bytes[site_abs as usize] = (w >> 8) as u8;
            bytes[site_abs as usize + 1] = (w & 0xFF) as u8;
        }
        FixupKind::Abs32Be => {
            let w = value as u32;
            bytes[site_abs as usize] = (w >> 24) as u8;
            bytes[site_abs as usize + 1] = (w >> 16) as u8;
            bytes[site_abs as usize + 2] = (w >> 8) as u8;
            bytes[site_abs as usize + 3] = (w & 0xFF) as u8;
        }
        FixupKind::PcRel8 | FixupKind::PcRelDisp16 | FixupKind::PcRelDisp8 => {
            // Implemented in Task 3.
            diags.push(diag(format!("PC-relative fixup {:?} not yet implemented", fx.kind), span));
        }
        FixupKind::HeaderChecksum => {
            diags.push(diag("HeaderChecksum is a post-image pass, not an in-fragment fixup".into(), span));
        }
```

> The `abs.w` fit check accepts both low addresses (`0..=0x7FFF`) and high sign-extended addresses (`0xFF8000..=0xFFFFFF`); the width-selection rule (Task 9) normally only routes fitting values here, but the guard makes a mis-route loud rather than silently truncating.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p sigil-link`
Expected: PASS. The pre-existing `abs16be_unsupported_in_m0_diagnoses` test now fails on its assertion (it expects "not supported in M0"). **Delete that obsolete test** (`abs16be_unsupported_in_m0_diagnoses`) — it asserted the stub we just replaced.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-link/src/lib.rs
git commit -m "feat(sigil-link): resolve Abs16Be/Abs32Be fixups (big-endian, abs.w fit check)"
```

---

## Task 3: Resolve PC-relative fixups

**Files:**
- Modify: `crates/sigil-link/src/lib.rs` (the `PcRel8 | PcRelDisp16 | PcRelDisp8` arm)

- [ ] **Step 1: Write failing tests**

```rust
    #[test]
    fn pcrel_disp16_measured_from_extension_word() {
        // bra.w at op VMA 0x1000: [0x60,0x00, hi,lo]. Disp word at offset 2 (VMA 0x1002).
        // target 0x1080 → disp = 0x1080 - 0x1002 = 0x7E.
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x1000,
            labels: vec![Label { name: "t".into(), offset: 0x80 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00, 0x00, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRelDisp16, offset: 2, target: Expr::Sym("t".into()) }],
                span: span(),
            })],
        };
        let linked = link(&[sec], &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x60, 0x00, 0x00, 0x7E]);
    }

    #[test]
    fn pcrel8_measured_from_op_plus_two() {
        // bra.s at op VMA 0x2000: [0x60, disp]. disp byte at offset 1 (VMA 0x2001).
        // target 0x2010 → disp = 0x2010 - (0x2001 + 1) = 0x0E.
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x2000,
            labels: vec![Label { name: "t".into(), offset: 0x10 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRel8, offset: 1, target: Expr::Sym("t".into()) }],
                span: span(),
            })],
        };
        assert_eq!(link(&[sec], &SymbolTable::new()).unwrap().section("c").unwrap().bytes, vec![0x60, 0x0E]);
    }

    #[test]
    fn pcrel8_out_of_range_diagnoses() {
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x2000,
            labels: vec![Label { name: "far".into(), offset: 0x200 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRel8, offset: 1, target: Expr::Sym("far".into()) }],
                span: span(),
            })],
        };
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("out of range")), "got: {:?}", err);
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sigil-link pcrel`
Expected: FAIL — "not yet implemented".

- [ ] **Step 3: Implement the PC-relative arm**

Replace the `PcRel8 | PcRelDisp16 | PcRelDisp8` placeholder arm from Task 2 with:

```rust
        FixupKind::PcRel8 => {
            // disp measured from op+2; the disp byte sits at op+1 = site_vma.
            let disp = value - (site_vma as i64 + 1);
            if !(-128..=127).contains(&disp) {
                diags.push(diag(format!("bra.s/Bcc.s displacement out of range ({disp}) in section {section}"), span));
                return;
            }
            bytes[site_abs as usize] = disp as i8 as u8;
        }
        FixupKind::PcRelDisp16 => {
            // disp measured from the extension word's own VMA = site_vma.
            let disp = value - site_vma as i64;
            if !(-0x8000..=0x7FFF).contains(&disp) {
                diags.push(diag(format!("(d16,PC)/bra.w displacement out of range ({disp}) in section {section}"), span));
                return;
            }
            let w = disp as i16 as u16;
            bytes[site_abs as usize] = (w >> 8) as u8;
            bytes[site_abs as usize + 1] = (w & 0xFF) as u8;
        }
        FixupKind::PcRelDisp8 => {
            let disp = value - site_vma as i64;
            if !(-128..=127).contains(&disp) {
                diags.push(diag(format!("(d8,PC,Xn) displacement out of range ({disp}) in section {section}"), span));
                return;
            }
            bytes[site_abs as usize] = disp as i8 as u8;
        }
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p sigil-link`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-link/src/lib.rs
git commit -m "feat(sigil-link): resolve PcRel8/PcRelDisp16/PcRelDisp8 fixups (extension-word reference)"
```

---

## Task 4: `MemoryMap` type

**Files:**
- Create: `crates/sigil-ir/src/map.rs`
- Modify: `crates/sigil-ir/src/lib.rs` (add `pub mod map;`)

- [ ] **Step 1: Write the failing test**

Create `crates/sigil-ir/src/map.rs`:

```rust
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
        if end > (r.lma_base as u64 + r.size as u64) {
            return Err(format!(
                "section `{name}` [{lma:#X},{end:#X}) overflows region `{}` end {:#X}",
                r.name, r.lma_base + r.size
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
```

Add to `crates/sigil-ir/src/lib.rs` after `pub mod fixup;`:

```rust
pub mod map;
```

- [ ] **Step 2: Run to verify failure then pass**

Run: `cargo test -p sigil-ir map::`
Expected: PASS (this task is pure, so tests pass immediately on compile — acceptable; the value is the committed contract).

- [ ] **Step 3: Commit**

```bash
git add crates/sigil-ir/src/map.rs crates/sigil-ir/src/lib.rs
git commit -m "feat(sigil-ir): MemoryMap/Region types with region-containment + overflow validation"
```

---

## Task 5: `emit_rom` — map-aware single image + convsym no-op

**Files:**
- Modify: `crates/sigil-link/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
    #[test]
    fn emit_rom_places_sections_and_validates_regions() {
        use sigil_ir::map::{MemoryMap, Region, RegionKind};
        let map = MemoryMap::new(
            vec![Region { name: "rom".into(), lma_base: 0, size: 0x1_0000, kind: RegionKind::Rom, vma_base: None }],
            0x00,
        );
        let img = LinkedImage {
            sections: vec![
                LinkedSection { name: "a".into(), lma: 2, bytes: vec![0xAA, 0xBB] },
                LinkedSection { name: "b".into(), lma: 6, bytes: vec![0xCC] },
            ],
        };
        // head 0,1 filled; bytes at 2..4; gap at 4,5; byte at 6. Terminus = 7 (no padding).
        assert_eq!(emit_rom(&img, &map).unwrap(), vec![0x00, 0x00, 0xAA, 0xBB, 0x00, 0x00, 0xCC]);
    }

    #[test]
    fn emit_rom_rejects_section_outside_region() {
        use sigil_ir::map::{MemoryMap, Region, RegionKind};
        let map = MemoryMap::new(
            vec![Region { name: "rom".into(), lma_base: 0, size: 4, kind: RegionKind::Rom, vma_base: None }],
            0x00,
        );
        let img = LinkedImage { sections: vec![LinkedSection { name: "a".into(), lma: 8, bytes: vec![1] }] };
        assert!(emit_rom(&img, &map).is_err());
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sigil-link emit_rom`
Expected: FAIL — `emit_rom` not found.

- [ ] **Step 3: Implement `emit_rom`**

Add near `flatten` in `crates/sigil-link/src/lib.rs`:

```rust
use sigil_ir::map::MemoryMap;

/// The single-image ROM output (`p2bin` + `fixheader` replacement):
/// validate each section against the map, place bytes at LMA, gap-fill with the
/// map default, append NOTHING (the `convsym` no-op), then apply the header
/// checksum as the final pass. The ROM ends at the last section byte — no
/// power-of-two padding.
pub fn emit_rom(image: &LinkedImage, map: &MemoryMap) -> Result<Vec<u8>, String> {
    for s in &image.sections {
        map.validate_section(&s.name, s.lma, s.bytes.len() as u32)?;
    }
    let mut rom = flatten_checked(image, map.fill)?;
    // convsym no-op: append nothing.
    apply_header_checksum(&mut rom); // Task 6
    Ok(rom)
}
```

Add a temporary stub for `apply_header_checksum` so this compiles (replaced in Task 6):

```rust
/// TEMP stub — real implementation in Task 6.
pub fn apply_header_checksum(_rom: &mut [u8]) {}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p sigil-link emit_rom`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-link/src/lib.rs
git commit -m "feat(sigil-link): emit_rom — map-validated single image + convsym no-op (checksum stubbed)"
```

---

## Task 6: Header checksum (final pass)

**Files:**
- Modify: `crates/sigil-link/src/lib.rs` (replace the `apply_header_checksum` stub)

- [ ] **Step 1: Write failing tests**

```rust
    #[test]
    fn header_checksum_is_be_wordsum_over_200_to_eof_at_18e() {
        // Build a >0x200-byte ROM; put known words after 0x200; assert the
        // checksum word at 0x18E equals the BE word-sum over [0x200, EOF).
        let mut rom = vec![0u8; 0x210];
        rom[0x200] = 0x12; rom[0x201] = 0x34; // word 0x1234
        rom[0x202] = 0x00; rom[0x203] = 0x01; // word 0x0001
        // remaining 0x204..0x210 are zero words → sum = 0x1235.
        apply_header_checksum(&mut rom);
        assert_eq!(rom[0x18E], 0x12);
        assert_eq!(rom[0x18F], 0x35);
    }

    #[test]
    fn header_checksum_handles_odd_trailing_byte() {
        // Odd length: last lone byte forms a word with a 0x00 low half (BE hi-byte).
        let mut rom = vec![0u8; 0x203];
        rom[0x200] = 0x00; rom[0x201] = 0x10; // word 0x0010
        rom[0x202] = 0x05;                    // lone byte → word 0x0500
        apply_header_checksum(&mut rom);
        assert_eq!(((rom[0x18E] as u16) << 8) | rom[0x18F] as u16, 0x0510);
    }
```

> The odd-trailing-byte convention is verified against the reference ROM in Task 13; the current on-disk `aeon/s4.bin` is even-length, but modelling the lone byte as a high-half word keeps `emit_rom` correct for any terminus.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sigil-link header_checksum`
Expected: FAIL — stub leaves `0x18E` at 0.

- [ ] **Step 3: Implement**

Replace the `apply_header_checksum` stub:

```rust
/// Sega header checksum: 16-bit big-endian additive word-sum over `[0x200, EOF)`,
/// written big-endian at `0x18E`. The genuinely-last byte-mutating pass. An odd
/// trailing byte is summed as the high half of a word (low half 0x00).
pub fn apply_header_checksum(rom: &mut [u8]) {
    if rom.len() < 0x200 {
        return;
    }
    let mut sum: u16 = 0;
    let mut i = 0x200;
    while i + 1 < rom.len() {
        sum = sum.wrapping_add(((rom[i] as u16) << 8) | rom[i + 1] as u16);
        i += 2;
    }
    if i < rom.len() {
        sum = sum.wrapping_add((rom[i] as u16) << 8);
    }
    rom[0x18E] = (sum >> 8) as u8;
    rom[0x18F] = (sum & 0xFF) as u8;
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p sigil-link header_checksum`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-link/src/lib.rs
git commit -m "feat(sigil-link): header checksum final pass (BE word-sum over 0x200..EOF at 0x18E)"
```

---

## Task 7: `Fragment::JmpJsrSym` variant

**Files:**
- Modify: `crates/sigil-ir/src/lib.rs` (the `Fragment` enum + the three helper methods `image_len`/`vma_len`/`image_bytes`)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `crates/sigil-ir/src/lib.rs`:

```rust
    #[test]
    fn jmpjsr_sym_variant_constructs() {
        let f = Fragment::JmpJsrSym {
            is_jsr: true,
            target: Expr::Sym("Sub".into()),
            span: Span { source: SourceId(0), start: 0, end: 0 },
        };
        match f {
            Fragment::JmpJsrSym { is_jsr, .. } => assert!(is_jsr),
            _ => panic!("wrong variant"),
        }
    }
```

(Ensure `use crate::Expr;` / `sigil_span` imports exist in the test module; add `use crate::expr::Expr;` if needed.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sigil-ir jmpjsr`
Expected: FAIL — no variant `JmpJsrSym`.

- [ ] **Step 3: Add the variant + helper arms**

In the `Fragment` enum, after `Reserve`:

```rust
    /// A bare-symbol `jmp`/`jsr` whose operand width (`abs.w`/`abs.l`) is not yet
    /// chosen — the ONLY length-variable fragment. `resolve_layout` (sigil-link)
    /// picks the width and lowers this to a `Data` fragment (opcode word +
    /// Abs16Be/Abs32Be operand fixup) BEFORE `link()` runs, so the helpers below
    /// never see it at link time.
    JmpJsrSym { is_jsr: bool, target: crate::expr::Expr, span: Span },
```

Add a match arm to each of `image_len`, `vma_len`, and `image_bytes`:

```rust
                Fragment::JmpJsrSym { .. } => {
                    unreachable!("JmpJsrSym must be lowered by resolve_layout before layout/link")
                }
```

(For `image_len`/`vma_len` this is `n += match ... { ... Fragment::JmpJsrSym { .. } => unreachable!(...) }`; for `image_bytes` it is a statement arm.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p sigil-ir`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-ir/src/lib.rs
git commit -m "feat(sigil-ir): Fragment::JmpJsrSym — the length-variable jmp/jsr placeholder"
```

---

## Task 8: m68k backend deferred lowering

**Files:**
- Modify: `crates/sigil-backend-m68k/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module (extend imports: `use sigil_ir::{Expr, Fixup, FixupKind, Fragment};`):

```rust
    #[test]
    fn lower_jmp_jsr_sym_builds_placeholder_fragment() {
        let f = M68kBackend.lower_jmp_jsr_sym(true, Expr::Sym("Sub".into()), span());
        match f {
            Fragment::JmpJsrSym { is_jsr, target, .. } => {
                assert!(is_jsr);
                assert_eq!(target, Expr::Sym("Sub".into()));
            }
            _ => panic!("expected JmpJsrSym"),
        }
    }

    #[test]
    fn lower_branch_short_emits_opcode_plus_pcrel8() {
        // bra.s → 0x60 + placeholder disp, PcRel8 fixup at offset 1.
        let frag = M68kBackend
            .lower_branch(Mnemonic::Bra, Size::S, Expr::Sym(".t".into()), span())
            .unwrap();
        assert_eq!(frag.bytes, vec![0x60, 0x00]);
        assert_eq!(frag.fixups.len(), 1);
        assert_eq!(frag.fixups[0].kind, FixupKind::PcRel8);
        assert_eq!(frag.fixups[0].offset, 1);
    }

    #[test]
    fn lower_branch_word_emits_opcode_plus_pcreldisp16() {
        // bra.w → 0x60 0x00, ext word placeholder, PcRelDisp16 at offset 2.
        let frag = M68kBackend
            .lower_branch(Mnemonic::Bra, Size::W, Expr::Sym(".t".into()), span())
            .unwrap();
        assert_eq!(frag.bytes, vec![0x60, 0x00, 0x00, 0x00]);
        assert_eq!(frag.fixups[0].kind, FixupKind::PcRelDisp16);
        assert_eq!(frag.fixups[0].offset, 2);
    }

    #[test]
    fn lower_branch_bcc_uses_condition_opcode() {
        // beq.w → 0x67 0x00 + ext word. (Bcc opcode = 0x6000 | cc<<8; eq cc = 0x7.)
        let frag = M68kBackend
            .lower_branch(Mnemonic::Bcc(Cond::Eq), Size::W, Expr::Sym(".t".into()), span())
            .unwrap();
        assert_eq!(&frag.bytes[..2], &[0x67, 0x00]);
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sigil-backend-m68k lower_`
Expected: FAIL — methods not found.

- [ ] **Step 3: Implement the methods**

Extend the imports at the top:

```rust
use sigil_ir::{DataFragment, Expr, Fixup, FixupKind, Fragment};
use sigil_isa::m68k::{Cond, Instruction, Mnemonic, Operand, Size};
```

Add to `impl M68kBackend`:

```rust
    /// Lower a bare-symbol `jmp`/`jsr` to the length-variable placeholder the
    /// linker's `resolve_layout` will width-select and lower. `is_jsr` picks
    /// `jsr` (true) vs `jmp` (false).
    pub fn lower_jmp_jsr_sym(&self, is_jsr: bool, target: Expr, span: Span) -> Fragment {
        Fragment::JmpJsrSym { is_jsr, target, span }
    }

    /// Lower a symbolic `bra`/`bsr`/`Bcc` at an explicit size (`.s` or `.w`) to
    /// the opcode word + placeholder displacement + a PC-relative fixup. Aeon
    /// pins branch sizes, so the size is always known here (never selected).
    pub fn lower_branch(
        &self,
        mnemonic: Mnemonic,
        size: Size,
        target: Expr,
        span: Span,
    ) -> Result<DataFragment, LowerError> {
        // Encode with a zero displacement to get the opcode word bytes, then
        // attach the fixup over the displacement bytes.
        let disp_op = match size {
            Size::S => Operand::Disp(0),
            Size::W => Operand::Disp(0),
            other => return Err(LowerError { message: format!("branch size {other:?} illegal on 68000") }),
        };
        let inst = Instruction { mnemonic, size, ops: vec![disp_op] };
        let encoded = m68k::encode(&inst).map_err(|e| LowerError { message: e.to_string() })?;
        match size {
            Size::S => {
                // [opcode_hi, disp]; disp byte at offset 1.
                if encoded.len() != 2 {
                    return Err(LowerError { message: format!("bra.s expected 2 bytes, got {}", encoded.len()) });
                }
                Ok(DataFragment {
                    bytes: vec![encoded[0], 0x00],
                    fixups: vec![Fixup { kind: FixupKind::PcRel8, offset: 1, target }],
                    span,
                })
            }
            Size::W => {
                // [opcode_hi, 0x00, disp_hi, disp_lo]; ext word at offset 2.
                if encoded.len() != 4 {
                    return Err(LowerError { message: format!("bra.w expected 4 bytes, got {}", encoded.len()) });
                }
                Ok(DataFragment {
                    bytes: vec![encoded[0], encoded[1], 0x00, 0x00],
                    fixups: vec![Fixup { kind: FixupKind::PcRelDisp16, offset: 2, target }],
                    span,
                })
            }
            _ => unreachable!(),
        }
    }

    /// Lower an instruction carrying a symbolic `(d16,PC)` operand: encode with a
    /// `Pcd16(0)` placeholder, then attach a `PcRelDisp16` fixup at the byte
    /// offset of that extension word. `pcd16_offset` is that offset within the
    /// encoded bytes (the caller/front-end knows the operand layout).
    pub fn lower_pcrel_ea(
        &self,
        inst: &Instruction,
        pcd16_offset: u32,
        target: Expr,
        span: Span,
    ) -> Result<DataFragment, LowerError> {
        let bytes = m68k::encode(inst).map_err(|e| LowerError { message: e.to_string() })?;
        if pcd16_offset as usize + 2 > bytes.len() {
            return Err(LowerError { message: "pcd16 offset past instruction end".into() });
        }
        Ok(DataFragment {
            bytes,
            fixups: vec![Fixup { kind: FixupKind::PcRelDisp16, offset: pcd16_offset, target }],
            span,
        })
    }
```

> If `m68k::encode` for `Bra`/`Bsr`/`Bcc` with `Operand::Disp(0)` and `Size::S` does not yield a 2-byte `.s` form (or `Size::W` a 4-byte form), fix the call to match the M1.A encoder's branch contract (see `corpus_m68k` branch vectors) — the encoder is byte-golden, so mirror exactly what it expects. Do not change the encoder.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p sigil-backend-m68k`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-backend-m68k/src/lib.rs
git commit -m "feat(sigil-backend-m68k): deferred lowering — jmp/jsr placeholder, branch + (d16,PC) fixups"
```

---

## Task 9: `asl_width_rule` + width-selection golden vectors

**Files:**
- Create: `crates/sigil-link/src/relax.rs` (the `AbsWidth` enum + `asl_width_rule`)
- Modify: `crates/sigil-link/src/lib.rs` (add `mod relax; pub use relax::{AbsWidth, asl_width_rule};`)
- Modify: `crates/sigil-isa/tests/corpus_m68k/mod.rs` (add width + branch snippets — these regenerate golden bytes and document asl's width choices)

- [ ] **Step 1: Add width-probe snippets to the shared corpus**

Append to the `corpus_m68k()` list in `crates/sigil-isa/tests/corpus_m68k/mod.rs` (follow the file's existing `(snippet, Instruction)` tuple shape; for width probes we only need the byte oracle, so use the simplest existing constructor the file expects, or a `jmp`/`jsr` at explicit width whose bytes we already encode). Add explicit-width `jmp`/`jsr` probes:

```rust
    // Width-selection oracle probes: asl's abs.w/abs.l choice for bare jmp/jsr
    // is inferred from these explicit-width encodings + the boundary sweep in
    // gen-m68k-vectors (see relax.rs). These specific forms are byte-golden.
    ("jmp ($7FFE).w",     inst(Mnemonic::Jmp, Size::W, vec![Operand::AbsW(0x7FFE)])),
    ("jmp ($12345678).l", inst(Mnemonic::Jmp, Size::L, vec![Operand::AbsL(0x1234_5678)])),
    ("jsr ($7FFE).w",     inst(Mnemonic::Jsr, Size::W, vec![Operand::AbsW(0x7FFE)])),
    ("jsr ($12345678).l", inst(Mnemonic::Jsr, Size::L, vec![Operand::AbsL(0x1234_5678)])),
```

(Use the corpus file's existing `inst(...)`/`Operand`/`Mnemonic`/`Size` helpers and imports; if the file names them differently, match its convention. If a form already exists in the corpus, skip the duplicate.)

- [ ] **Step 2: Regenerate golden vectors and DERIVE the width rule**

Run: `cargo run -p sigil-isa --bin gen-m68k-vectors`
Then run the boundary sweep to observe asl's bare-`jmp` width choice under the exact ASFLAGS (`-A` on). Assemble each of these with `aeon/tools/asl -cpu 68000 -A` (mirror the generator's assemble path) and record the resulting width from the byte length (4 bytes = abs.w, 6 bytes = abs.l):

```
jmp $0000  ;  jmp $7FFF  ;  jmp $8000  ;  jmp $FFFE  ;  jmp $FFFF
jmp $10000 ;  jmp $FF8000 ; jmp $FFFFFF
```

**Hypothesis to confirm (standard 68k abs.w rule):** `asl` picks `abs.w` iff the address sign-extends losslessly from 16 bits — i.e. `addr ∈ [0x0, 0x7FFF] ∪ [0xFF_8000, 0xFF_FFFF]` (24-bit space) — else `abs.l`. Record the actual observed behaviour (and any `-A` effect) as a comment block in `relax.rs`. **The observed bytes are the spec; the hypothesis is only a starting guess.**

- [ ] **Step 3: Write the failing test**

Create `crates/sigil-link/src/relax.rs`:

```rust
//! Width selection + the bounded layout fixpoint (spec §5.4/§5.6). The only
//! length-variable fragment in Aeon is bare-symbol `jmp`/`jsr`.

/// The chosen absolute-addressing width for a `jmp`/`jsr`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsWidth {
    /// `abs.w`: opcode word + 2-byte operand (4 bytes total).
    W,
    /// `abs.l`: opcode word + 4-byte operand (6 bytes total).
    L,
}

impl AbsWidth {
    /// Total instruction length in bytes for this width.
    pub fn inst_len(self) -> u32 {
        match self {
            AbsWidth::W => 4,
            AbsWidth::L => 6,
        }
    }
}

/// asl's `abs.w` vs `abs.l` selection for a `jmp`/`jsr` target address, under
/// the Aeon ASFLAGS (`-A` on). Byte-verified by the gen-m68k-vectors boundary
/// sweep (Task 9). `abs.w` iff the 24-bit address sign-extends losslessly from
/// 16 bits: `[0, 0x7FFF] ∪ [0xFF_8000, 0xFF_FFFF]`.
pub fn asl_width_rule(target: i64, _dash_a: bool) -> AbsWidth {
    let a = (target & 0xFF_FFFF) as u32;
    if a <= 0x7FFF || a >= 0xFF_8000 {
        AbsWidth::W
    } else {
        AbsWidth::L
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_rule_matches_asl_boundary_sweep() {
        assert_eq!(asl_width_rule(0x0000, true), AbsWidth::W);
        assert_eq!(asl_width_rule(0x7FFF, true), AbsWidth::W);
        assert_eq!(asl_width_rule(0x8000, true), AbsWidth::L);
        assert_eq!(asl_width_rule(0xFFFF, true), AbsWidth::L);
        assert_eq!(asl_width_rule(0x1_0000, true), AbsWidth::L);
        assert_eq!(asl_width_rule(0xFF_8000, true), AbsWidth::W);
        assert_eq!(asl_width_rule(0xFF_FFFF, true), AbsWidth::W);
    }
}
```

Add to `crates/sigil-link/src/lib.rs`:

```rust
mod relax;
pub use relax::{asl_width_rule, resolve_layout, AbsWidth};
```

(`resolve_layout` lands in Task 10; if it does not exist yet, temporarily export only `{asl_width_rule, AbsWidth}` and add `resolve_layout` to the `pub use` in Task 10.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p sigil-link relax:: && cargo test -p sigil-isa --test encode_m68k`
Expected: PASS. **If Step 2's observed asl behaviour differs from the hypothesis, edit `asl_width_rule` and the test to the observed truth before this passes.**

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-link/src/relax.rs crates/sigil-link/src/lib.rs \
        crates/sigil-isa/tests/corpus_m68k/mod.rs crates/sigil-isa/tests/m68k_golden_vectors.txt
git commit -m "feat(sigil-link): asl_width_rule (abs.w/abs.l) verified against asl boundary sweep"
```

---

## Task 10: `resolve_layout` — the bounded width fixpoint

**Files:**
- Modify: `crates/sigil-link/src/relax.rs` (add `resolve_layout`)

- [ ] **Step 1: Write failing tests**

Add to `crates/sigil-link/src/relax.rs` tests:

```rust
    use sigil_ir::{Cpu, DataFragment, Expr, Fragment, Label, Section, SymbolTable, SymbolValue};
    use sigil_span::{SourceId, Span};

    fn sp() -> Span { Span { source: SourceId(0), start: 0, end: 0 } }

    #[test]
    fn resolve_lowers_jmp_to_absw_for_low_target() {
        // Section at LMA 0: [jmp Low] then Low: nop (0x4E71). Low VMA = 4 (abs.w).
        let sec = Section {
            name: "c".into(), cpu: Cpu::M68000, vma_base: None, lma: 0,
            labels: vec![Label { name: "Low".into(), offset: 4 }],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Low".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
            ],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        // jmp abs.w = 4EF8 + word(0x0004) = 4E F8 00 04, then nop.
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x4E, 0xF8, 0x00, 0x04, 0x4E, 0x71]);
    }

    #[test]
    fn resolve_lowers_jmp_to_absl_for_high_target() {
        // jmp to a stubbed high address → abs.l (4EF9 + long).
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let sec = Section {
            name: "c".into(), cpu: Cpu::M68000, vma_base: None, lma: 0, labels: vec![],
            fragments: vec![Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Hi".into()), span: sp() }],
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        let linked = crate::link(&out, &stubs).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x4E, 0xF9, 0x00, 0x12, 0x34, 0x56]);
    }

    #[test]
    fn resolve_width_shift_moves_downstream_label_but_converges() {
        // A jmp whose own .w→.l growth pushes a later label across the 0x7FFF
        // boundary must converge (the fixpoint re-lays-out). Here targets stay
        // low, so it converges to abs.w in one extra pass — just assert success.
        let sec = Section {
            name: "c".into(), cpu: Cpu::M68000, vma_base: None, lma: 0,
            labels: vec![Label { name: "End".into(), offset: 4 }],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: true, target: Expr::Sym("End".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
            ],
        };
        assert!(resolve_layout(&[sec], &SymbolTable::new(), true).is_ok());
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sigil-link resolve_`
Expected: FAIL — `resolve_layout` not found.

- [ ] **Step 3: Implement `resolve_layout`**

Add to `crates/sigil-link/src/relax.rs`:

```rust
use sigil_ir::expr::Fold;
use sigil_ir::{DataFragment, Expr, Fixup, FixupKind, Fragment, Section, SymbolTable, SymbolValue};
use sigil_span::{Diagnostic, Level, Span};

const MAX_PASSES: usize = 64;

/// Size of a fragment given the current width choices for its JmpJsrSym members.
/// `widths[(sec_idx, frag_idx)]` holds the current AbsWidth guess.
fn frag_vma_len(sec_i: usize, frag_i: usize, frag: &Fragment, widths: &[Vec<AbsWidth>]) -> u32 {
    match frag {
        Fragment::Data(d) => d.bytes.len() as u32,
        Fragment::Fill { count, .. } => *count,
        Fragment::Reserve { count, .. } => *count,
        Fragment::JmpJsrSym { .. } => widths[sec_i][frag_i].inst_len(),
    }
}

/// Resolve the width of every `JmpJsrSym` via a bounded, monotone (grow-only)
/// fixpoint, then lower each to a concrete `Data` fragment (opcode word +
/// Abs16Be/Abs32Be operand fixup). The returned sections contain only
/// Data/Fill/Reserve, so `link()` runs on them unchanged.
pub fn resolve_layout(
    sections: &[Section],
    stubs: &SymbolTable,
    dash_a: bool,
) -> Result<Vec<Section>, Vec<Diagnostic>> {
    // Start every JmpJsrSym at its minimum width (abs.w). Grow-only ⇒ terminates.
    let mut widths: Vec<Vec<AbsWidth>> = sections
        .iter()
        .map(|s| s.fragments.iter().map(|_| AbsWidth::W).collect())
        .collect();

    for _pass in 0..MAX_PASSES {
        // (a) Provisional layout: build the symbol table at current widths.
        let mut syms = stubs.clone();
        for (si, sec) in sections.iter().enumerate() {
            let origin = sec.vma_origin();
            // Byte offset of each label = sum of preceding fragment VMA lengths.
            // Labels carry an absolute offset already, but JmpJsrSym growth shifts
            // everything after it, so recompute label offsets from fragment order.
            // We map each label's *original* offset onto the current layout by
            // walking fragments and tracking the delta from width growth.
            let mut cur: u32 = 0;      // running current offset
            let mut orig: u32 = 0;     // running original (all-abs.w) offset
            let mut deltas: Vec<(u32, i64)> = vec![(0, 0)]; // (orig_offset, delta) breakpoints
            for (fi, frag) in sec.fragments.iter().enumerate() {
                let cur_len = frag_vma_len(si, fi, frag, &widths);
                let orig_len = match frag {
                    Fragment::JmpJsrSym { .. } => AbsWidth::W.inst_len(),
                    other => frag_vma_len(si, fi, other, &widths),
                };
                cur += cur_len;
                orig += orig_len;
                deltas.push((orig, cur as i64 - orig as i64));
            }
            let shift = |orig_off: u32| -> u32 {
                // delta applied to a label = delta at the last breakpoint <= orig_off
                let mut d = 0i64;
                for &(bo, bd) in &deltas {
                    if bo <= orig_off { d = bd; } else { break; }
                }
                (orig_off as i64 + d) as u32
            };
            for label in &sec.labels {
                syms.define(&label.name, SymbolValue::Int((origin + shift(label.offset)) as i64));
            }
        }

        // (b) Re-select each width from the resolved target; monotone max.
        let mut grew = false;
        for (si, sec) in sections.iter().enumerate() {
            for (fi, frag) in sec.fragments.iter().enumerate() {
                if let Fragment::JmpJsrSym { target, span, .. } = frag {
                    let v = match target.fold(&|n| syms.resolve(n, None)) {
                        Fold::Value(v) => v,
                        Fold::Poison => {
                            return Err(vec![Diagnostic {
                                level: Level::Error,
                                message: format!("unresolved jmp/jsr target in section {}", sec.name),
                                primary: *span,
                            }]);
                        }
                    };
                    let want = asl_width_rule(v, dash_a);
                    if want == AbsWidth::L && widths[si][fi] == AbsWidth::W {
                        widths[si][fi] = AbsWidth::L; // grow only
                        grew = true;
                    }
                }
            }
        }
        if !grew {
            return Ok(lower_all(sections, &widths));
        }
    }
    Err(vec![Diagnostic {
        level: Level::Error,
        message: format!("jmp/jsr width selection did not converge within {MAX_PASSES} passes"),
        primary: sections.first().and_then(|s| s.fragments.first()).map(frag_span).unwrap_or(zero_span()),
    }])
}

fn zero_span() -> Span { Span { source: sigil_span::SourceId(0), start: 0, end: 0 } }
fn frag_span(f: &Fragment) -> Span {
    match f {
        Fragment::Data(d) => d.span,
        Fragment::Fill { span, .. } | Fragment::Reserve { span, .. } | Fragment::JmpJsrSym { span, .. } => *span,
    }
}

/// Lower every JmpJsrSym to a concrete Data fragment at its chosen width.
fn lower_all(sections: &[Section], widths: &[Vec<AbsWidth>]) -> Vec<Section> {
    sections
        .iter()
        .enumerate()
        .map(|(si, sec)| {
            let fragments = sec
                .fragments
                .iter()
                .enumerate()
                .map(|(fi, frag)| match frag {
                    Fragment::JmpJsrSym { is_jsr, target, span } => {
                        lower_jmp_jsr(*is_jsr, target.clone(), widths[si][fi], *span)
                    }
                    other => other.clone(),
                })
                .collect();
            Section { fragments, ..sec.clone() }
        })
        .collect()
}

/// jmp abs.w = 4EF8, jmp abs.l = 4EF9, jsr abs.w = 4EB8, jsr abs.l = 4EB9.
fn lower_jmp_jsr(is_jsr: bool, target: Expr, w: AbsWidth, span: Span) -> Fragment {
    let base: u16 = if is_jsr { 0x4EB8 } else { 0x4EF8 };
    match w {
        AbsWidth::W => {
            let op = base; // .w form
            Fragment::Data(DataFragment {
                bytes: vec![(op >> 8) as u8, (op & 0xFF) as u8, 0x00, 0x00],
                fixups: vec![Fixup { kind: FixupKind::Abs16Be, offset: 2, target }],
                span,
            })
        }
        AbsWidth::L => {
            let op = base | 0x0001; // .l form
            Fragment::Data(DataFragment {
                bytes: vec![(op >> 8) as u8, (op & 0xFF) as u8, 0, 0, 0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs32Be, offset: 2, target }],
                span,
            })
        }
    }
}
```

> The label-shift logic re-maps each label's original (all-`abs.w`) offset onto the current layout using running deltas. This is the general form; because Aeon's `jmp`/`jsr` targets are effectively pinned, in practice it converges in ≤2 passes. If simpler correct code is clear to you, prefer it — the tests are the contract.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p sigil-link`
Expected: PASS (all `resolve_` tests + existing suite).

- [ ] **Step 5: Add the non-convergence guardrail test + commit**

Add this test (a synthetic input that oscillates is hard to construct with a monotone rule, so instead assert the cap constant is wired by forcing a target that is exactly boundary-crossing is unnecessary; the grow-only rule cannot oscillate, so the cap is a backstop). Assert the cap path is reachable code by unit-testing `asl_width_rule` monotonicity is already covered — **document that grow-only guarantees termination and the cap is a backstop**, and commit:

```bash
git add crates/sigil-link/src/relax.rs
git commit -m "feat(sigil-link): resolve_layout — bounded grow-only jmp/jsr width fixpoint + lowering"
```

---

## Task 11: `s4.lst` emitter

**Files:**
- Create: `crates/sigil-link/src/listing.rs`
- Modify: `crates/sigil-link/src/lib.rs` (`mod listing; pub use listing::{emit_listing, ListingSymbol};`)

- [ ] **Step 1: Write failing tests**

Create `crates/sigil-link/src/listing.rs`:

```rust
//! `s4.lst` symbol-listing emitter. Target: the AS `-L` symbol-table section
//! that `tools/s4budget.py::parse_symbol_table` and the Oracle symbol loader
//! consume. Scope = symbol name, 24-bit hex value, C(code)/-(equate) marker,
//! `|` separator, the `Symbol Table (* = unused):` header, `N symbols` footer.

/// One symbol row. `is_equate` picks the `-` (equate) vs `C` (code) marker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListingSymbol {
    pub name: String,
    pub value: u32,
    pub is_equate: bool,
    pub unused: bool,
}

/// Emit the AS-`-L`-compatible symbol-table section. Symbols are address-sorted;
/// each row is `[*]NAME : HEX C|-` `|`. One symbol per line keeps it trivially
/// parseable (both consumers iterate matches, so layout is cosmetic).
pub fn emit_listing(symbols: &[ListingSymbol]) -> String {
    let mut rows: Vec<&ListingSymbol> = symbols.iter().collect();
    rows.sort_by(|a, b| a.value.cmp(&b.value).then(a.name.cmp(&b.name)));
    let unused = rows.iter().filter(|s| s.unused).count();

    let mut out = String::new();
    out.push_str("  Symbol Table (* = unused):\n");
    out.push_str("  --------------------------\n\n");
    for s in &rows {
        let star = if s.unused { "*" } else { " " };
        let marker = if s.is_equate { "-" } else { "C" };
        out.push_str(&format!("{star}{} : {:X} {marker} |\n", s.name, s.value));
    }
    out.push_str(&format!("\n   {} symbols\n", rows.len()));
    out.push_str(&format!("    {unused} unused symbols\n"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(name: &str, value: u32, eq: bool, unused: bool) -> ListingSymbol {
        ListingSymbol { name: name.into(), value, is_equate: eq, unused }
    }

    #[test]
    fn emits_s4budget_parseable_rows() {
        // Mirror s4budget's regex: (\*?)([\w.]+)\s*:\s*(hex|"str")\s+([C\-])\s*\|
        let out = emit_listing(&[
            sym("Main", 0x000000, false, false),
            sym("OBJ_len", 0x40, true, false),
            sym("Unused", 0x2000, false, true),
        ]);
        assert!(out.contains("Symbol Table"));
        assert!(out.contains("unused"));
        // address-sorted; code marker C, equate marker -.
        assert!(out.contains("Main : 0 C |"));
        assert!(out.contains("OBJ_len : 40 - |"));
        assert!(out.contains("*Unused : 2000 C |"));
        assert!(out.contains("3 symbols"));
        assert!(out.contains("1 unused symbols"));
    }

    #[test]
    fn regex_intersection_matches_each_row() {
        // A pure-Rust stand-in for s4budget's regex to prove the grammar holds.
        let out = emit_listing(&[sym("Air_LandState", 0x10AF2, false, false)]);
        let re_ok = out.lines().any(|l| {
            let l = l.trim_start();
            // [*]name : HEX (C|-) |
            l.contains(" : ") && l.trim_end().ends_with('|')
                && (l.contains(" C |") || l.contains(" - |"))
        });
        assert!(re_ok, "no parseable row in:\n{out}");
    }
}
```

Add to `crates/sigil-link/src/lib.rs`:

```rust
mod listing;
pub use listing::{emit_listing, ListingSymbol};
```

- [ ] **Step 2: Run to verify failure then pass**

Run: `cargo test -p sigil-link listing::`
Expected: PASS after adding the module (write test first mentally; the module + test land together, run to confirm green).

- [ ] **Step 3: Commit**

```bash
git add crates/sigil-link/src/listing.rs crates/sigil-link/src/lib.rs
git commit -m "feat(sigil-link): emit_listing — s4.lst symbol-table section (s4budget/Oracle grammar)"
```

> The real Oracle load-path format check (`LoadFromAsListing` vs `Symbols.cpp`) and `s4budget.py --summary` acceptance are the Task 13 integration gate (M1.d). If Oracle needs the body-line format rather than the symbol-table section, extend `emit_listing` there with the format the actual load path parses.

---

## Task 12: TOML map loader + canonical `sigil.map.toml`

**Files:**
- Modify: `crates/sigil-link/Cargo.toml` (add `serde`, `toml`)
- Create: `crates/sigil-link/src/map_load.rs`
- Modify: `crates/sigil-link/src/lib.rs` (`mod map_load; pub use map_load::load_map;`)
- Create: `sigil.map.toml` (repo root)

- [ ] **Step 1: Add dependencies**

In `crates/sigil-link/Cargo.toml` `[dependencies]`:

```toml
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

- [ ] **Step 2: Write the failing test**

Create `crates/sigil-link/src/map_load.rs`:

```rust
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
```

Add to `crates/sigil-link/src/lib.rs`:

```rust
mod map_load;
pub use map_load::load_map;
```

- [ ] **Step 3: Run to verify pass**

Run: `cargo test -p sigil-link map_load::`
Expected: PASS.

- [ ] **Step 4: Create the canonical Aeon map**

Create `sigil.map.toml` at the repo root (values match the Aeon layout; the ROM region spans the whole cart, the z80 bank is the `phase 08000h` window — cross-check bases against `crates/sigil-harness/golden/windows.toml`):

```toml
# Sigil external memory map for Aeon (the p2bin layout the source assumes).
# Default gap fill matches p2bin invoked without -p.
fill = 0x00

[[region]]
name = "rom"
lma_base = 0
size = 0x400000        # 4 MiB address space; the image ends at the source terminus
kind = "rom"

[[region]]
name = "z80_moving_trucks_bank"
lma_base = 0x60000     # cross-check against golden/windows.toml Region B lma
size = 0x8000
kind = "z80_bank"
vma_base = 0x8000
```

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-link/Cargo.toml crates/sigil-link/src/map_load.rs crates/sigil-link/src/lib.rs sigil.map.toml Cargo.lock
git commit -m "feat(sigil-link): TOML map loader + canonical sigil.map.toml; commit Cargo.lock"
```

---

## Task 13: Integration gate (M1.B acceptance) + CI

**Files:**
- Modify: `crates/sigil-harness/src/lib.rs` (or a new `crates/sigil-harness/tests/m1b_gate.rs`) — the three acceptance checks
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the checksum-vs-reference test**

Create `crates/sigil-harness/tests/m1b_gate.rs`:

```rust
//! M1.B acceptance gate: prove the linker's byte-mutating passes reproduce the
//! reference ROM's checksum and that emit_listing is tool-parseable. The full
//! sha256 identity is D; here we gate the pieces B owns.

use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()))
}

#[test]
fn header_checksum_reproduces_reference_rom_18e() {
    // Read the pinned reference ROM, recompute the checksum over [0x200,EOF)
    // with the linker's algorithm, and assert it equals the stored word at 0x18E.
    let rom_path = aeon_dir().join("s4.bin");
    let Ok(mut rom) = std::fs::read(&rom_path) else {
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let stored = ((rom[0x18E] as u16) << 8) | rom[0x18F] as u16;
    // Zero the stored word first so recomputation is independent of it
    // (the checksum range starts at 0x200, so 0x18E is outside it — but zero
    // anyway to prove the algorithm derives the value, not reads it back).
    rom[0x18E] = 0; rom[0x18F] = 0;
    sigil_link::apply_header_checksum(&mut rom);
    let got = ((rom[0x18E] as u16) << 8) | rom[0x18F] as u16;
    assert_eq!(got, stored, "checksum mismatch: got {got:#06X}, ref {stored:#06X}");
}
```

- [ ] **Step 2: Run to verify it passes against the real ROM**

Run: `cargo test -p sigil-harness --test m1b_gate header_checksum`
Expected: PASS (checksum equals the pinned ROM's `$18E`; currently `$5CBE`). If the ROM is absent, the test skips with a message.

- [ ] **Step 3: Write the hand-built multi-section fixup gate**

Add to `crates/sigil-harness/tests/m1b_gate.rs` a test that builds a small 68k `Section` set exercising a cross-section `jsr` (width-selected) + a `bra.w` (PcRelDisp16), runs `resolve_layout` → `link` → `emit_rom`, and asserts the bytes. Use a map covering `[0, 0x1000)`:

```rust
#[test]
fn multi_section_jsr_and_branch_link_correctly() {
    use sigil_ir::map::{MemoryMap, Region, RegionKind};
    use sigil_ir::{Cpu, DataFragment, Expr, Fragment, Label, Section, SymbolTable};
    use sigil_span::{SourceId, Span};
    fn sp() -> Span { Span { source: SourceId(0), start: 0, end: 0 } }

    // Section "code" @0: jsr Target ; bra.w Done ; Done: nop
    let code = Section {
        name: "code".into(), cpu: Cpu::M68000, vma_base: None, lma: 0,
        labels: vec![Label { name: "Done".into(), offset: 8 }], // 6 (jsr abs.w? no—low target→abs.w=4) ... see note
        fragments: vec![
            Fragment::JmpJsrSym { is_jsr: true, target: Expr::Sym("Target".into()), span: sp() },
            Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00, 0x00, 0x00],
                fixups: vec![sigil_ir::Fixup { kind: sigil_ir::FixupKind::PcRelDisp16, offset: 2, target: Expr::Sym("Done".into()) }],
                span: sp(),
            }),
            Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
        ],
    };
    // Section "target" @0x100: Target: rts (0x4E75)
    let target = Section {
        name: "target".into(), cpu: Cpu::M68000, vma_base: None, lma: 0x100,
        labels: vec![Label { name: "Target".into(), offset: 0 }],
        fragments: vec![Fragment::Data(DataFragment { bytes: vec![0x4E, 0x75], fixups: vec![], span: sp() })],
    };
    let map = MemoryMap::new(
        vec![Region { name: "rom".into(), lma_base: 0, size: 0x1000, kind: RegionKind::Rom, vma_base: None }],
        0x00,
    );
    let resolved = sigil_link::resolve_layout(&[code, target], &SymbolTable::new(), true).unwrap();
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).unwrap();
    let c = &linked.section("code").unwrap().bytes;
    // jsr Target(0x100) → abs.w: 4E B8 01 00. Target VMA 0x100 fits abs.w.
    assert_eq!(&c[..4], &[0x4E, 0xB8, 0x01, 0x00]);
    // Then bra.w Done: Done is right after the branch. jsr(4) + bra(4) = 8 → Done@8; nop@8.
    // bra.w at op VMA 4: ext word @6; disp = 8 - 6 = 2 → 60 00 00 02.
    assert_eq!(&c[4..8], &[0x60, 0x00, 0x00, 0x02]);
    let _rom = sigil_link::emit_rom(&linked, &map).unwrap();
}
```

> **Adjust the `Done` label offset and expected disp to whatever the actual widths produce** — run once, read the bytes, and pin them. The point is a real cross-section width-selected `jsr` + a `bra.w` resolving byte-correctly through the full `resolve_layout → link → emit_rom` chain. Where feasible, add an asl-diff variant that assembles the equivalent snippet through `aeon/tools/asl` and compares (reuse the generator's assemble path).

- [ ] **Step 4: Write the s4.lst tool-acceptance gate (M1.d)**

Add a test that writes `emit_listing` output to a temp file and runs `aeon/tools/s4budget.py` (or directly applies its `_SYM_ENTRY_RE`) to confirm it parses; and, if the Oracle symbol-loader binary/entrypoint is available, loads it and resolves a known symbol. Because invoking Oracle may need its build, gate the Python check unconditionally and the Oracle check behind availability:

```rust
#[test]
fn s4budget_parses_emit_listing() {
    use sigil_link::{emit_listing, ListingSymbol};
    let lst = emit_listing(&[
        ListingSymbol { name: "Main".into(), value: 0x1000, is_equate: false, unused: false },
        ListingSymbol { name: "OBJ_len".into(), value: 0x40, is_equate: true, unused: false },
    ]);
    let dir = std::env::temp_dir().join("sigil_m1b_lst");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("s4.lst");
    std::fs::write(&path, &lst).unwrap();
    let script = aeon_dir().join("tools/s4budget.py");
    if !script.is_file() {
        eprintln!("skip: s4budget.py not found");
        return;
    }
    // s4budget's parse_symbol_table must find both symbols. Run --summary; expect exit 0.
    let out = std::process::Command::new("python3")
        .arg(&script).arg("--summary").arg("--lst").arg(&path)
        .output();
    if let Ok(o) = out {
        assert!(o.status.success(), "s4budget failed: {}", String::from_utf8_lossy(&o.stderr));
    } else {
        eprintln!("skip: python3 unavailable");
    }
}
```

> Check `s4budget.py`'s actual CLI (`build.sh` line ~171) for the correct flag to point it at a listing; adapt the args. If it only accepts a fixed path, write to that path in a temp CWD. The gate is: s4budget parses Sigil's listing without error.

- [ ] **Step 5: Run the full gate**

Run: `cargo test -p sigil-harness --test m1b_gate`
Expected: PASS (checks that can run) / SKIP (with message) where a tool is absent. Pin any adjusted expected bytes.

- [ ] **Step 6: Add CI**

Create `.github/workflows/ci.yml`:

```yaml
name: ci
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - name: test
        run: cargo test --workspace
      - name: clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: crate-graph
        run: cargo test -p sigil-ir crate_graph
```

> Adjust the crate-graph step to however the existing `crate_graph.rs` guard is invoked (it may be a test in another crate — grep `crate_graph` and match). asl-oracle steps are NOT in CI (asl not available there); CI reads committed golden files only.

- [ ] **Step 7: Verify full workspace green + commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS, clean.

```bash
git add crates/sigil-harness/tests/m1b_gate.rs .github/workflows/ci.yml
git commit -m "test(sigil-m1b): M1.B acceptance gate (checksum vs ref, multi-section link, s4.lst) + CI"
```

- [ ] **Step 8: Pin the reference commit in PROVENANCE**

Append the aeon reference commit hash + `s4.bin` length + `$18E` checksum to `crates/sigil-harness/golden/PROVENANCE.md` (get the hash: `git -C ../aeon rev-parse HEAD`). Commit:

```bash
git add crates/sigil-harness/golden/PROVENANCE.md
git commit -m "docs(sigil-harness): pin aeon reference commit + ROM length/checksum for M1.B"
```

---

## Self-Review

**Spec coverage** (design doc §→task):
- §3.1 MemoryMap → Task 4, 12 ✓
- §3.2 layout fixpoint (jmp/jsr width) → Task 9, 10 ✓
- §3.3 68k fixup resolution (Abs16/32, PcRel family, target→disp) → Task 2, 3, 8 ✓
- §3.4 width rule + `-A` → Task 9 ✓
- §3.5 single image + convsym no-op → Task 5 ✓
- §3.6 header checksum → Task 6, 13 ✓
- §3.7 s4.lst → Task 11, 13 ✓
- §4 IR changes (fixup kinds, JmpJsrSym, MemoryMap) → Task 1, 4, 7 ✓
- §5 test strategy (asl-oracle, hand-built corpus, checksum vs ref, s4.lst gate, guardrail) → Task 9, 13; guardrail termination argued in Task 10 ✓
- §6 reference pin + CI + Cargo.lock → Task 12 (lock), 13 (CI, pin) ✓
- §8 acceptance gate → Task 13 ✓

**Placeholder scan:** No "TBD"/"add error handling" — each step has real code. Two intentional interim stubs (`apply_header_checksum` in Task 5, `PcRel*` arm in Task 2) are *replaced within the next task* and labelled as such. Empirical steps (Task 9 Step 2, Task 13 byte-pinning) show the exact commands and the hypothesis to confirm — not vague.

**Type consistency:** `FixupKind` variants, `MemoryMap`/`Region`/`RegionKind`, `Fragment::JmpJsrSym { is_jsr, target, span }`, `AbsWidth::{W,L}`, `asl_width_rule(i64, bool)->AbsWidth`, `resolve_layout(&[Section], &SymbolTable, bool)->Result<Vec<Section>,Vec<Diagnostic>>`, `emit_rom(&LinkedImage,&MemoryMap)->Result<Vec<u8>,String>`, `apply_header_checksum(&mut [u8])`, `emit_listing(&[ListingSymbol])->String` — names match across all tasks and the design doc's "Naming locked" block.

**Known soft spots flagged for the implementer** (not gaps — verification points): asl width rule is derived, not assumed (Task 9); the Oracle load-path format is confirmed at the M1.d gate (Task 11/13); hand-built expected bytes in Task 13 are pinned after a first run. These are inherent to asl-oracle TDD and are called out at their steps.
