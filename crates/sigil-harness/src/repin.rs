//! `repin` — the listing-derived pin generator (tranche-10 step 0).
//!
//! Re-pin waves used to be string substitution over ~115 hand-typed layout
//! literals scattered across ~16 test files; every substitution error cost a
//! suite run to find. This module kills that bug class: it parses the AS
//! listings' `Symbol Table` sections (`aeon/s4.lst` + `aeon/s4.debug.lst`),
//! resolves the declarative manifest (`crates/sigil-harness/repin.toml`)
//! against BOTH shapes, and renders the generated `src/pins.rs` the port
//! tests import. Design: `docs/superpowers/notes/
//! 2026-07-10-tranche10-repin-design.md` (D-T10.1..D-T10.9).
//!
//! The binary front-end lives in `src/bin/repin.rs`; the logic lives here so
//! the staleness test (D-T10.5, `tests/repin_pins.rs::pins_rs_is_current`)
//! can regenerate in-memory and compare against the committed file.
//!
//! ## Listing format facts (verified against the 2026-07-10 listings)
//!
//! - The table starts after the line `Symbol Table (* = unused):` and its
//!   dashes underline, and ends at the `NNNN symbols` count line.
//! - Two `NAME : HEXVALUE TYPE |` entries per line normally; long names,
//!   sign-extended values and string values span a full line. Entries are
//!   `|`-separated either way, so the chunk split handles both.
//! - `*` prefix = unused symbol — still parsed (gate regions reference
//!   symbols the reference build may not otherwise use).
//! - Local labels appear parent-qualified (`AnimateSprite.cc_delete`).
//! - RAM symbols are sign-extended 64-bit hex (`FFFFFFFFFFFF89EE`) —
//!   truncated to the u32 VMA every consumer pins.
//! - Non-numeric values exist and are SKIPPED: strings
//!   (`ARCHITECTURE : "x86_64-unknown-linux"`), char literals
//!   (`CROSS_RESET_MAGIC : 'INIT'`), floats (`CONSTPI : 3.14159…`).
//! - Page headers (`AS V1.42 Beta … - Page N - …`) INTERRUPT the table
//!   mid-stream and are skipped.
//! - The `END` line (`     333/   658B4 :                         END`)
//!   carries the assembled ROM end — the `[rom]` pins.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

use serde::Deserialize;

// ── Listing parsing ─────────────────────────────────────────────────────────

/// One parsed listing: the symbol table plus the `END` line address and the
/// page-header date stamp (provenance).
#[derive(Debug)]
pub struct Listing {
    symbols: HashMap<String, u32>,
    /// The `END` line's address — the assembled ROM length.
    pub end_addr: u32,
    /// The page headers' trailing date stamp (`07/10/2026 10:03:54 PM`),
    /// carried into the generated file's provenance header.
    pub stamp: String,
}

impl Listing {
    /// Exact-match lookup. Unknown symbol = HARD ERROR naming it (D-T10.2 —
    /// never a silent 0). `Prof_RunObjects` vs `RunObjects` are DIFFERENT
    /// names; no prefix/suffix matching happens here.
    pub fn get(&self, name: &str) -> Result<u32, String> {
        self.symbols
            .get(name)
            .copied()
            .ok_or_else(|| format!("symbol `{name}` not found in the listing symbol table"))
    }

    /// Number of parsed (numeric) symbols — provenance/debug aid.
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }
}

/// Parse one AS listing (`s4.lst` shape). Fails loudly on: no symbol table,
/// no `END` line, a duplicate symbol name (exact-name resolution would be
/// ambiguous), or a value chunk in none of the known formats.
pub fn parse_listing(text: &str) -> Result<Listing, String> {
    let mut symbols: HashMap<String, u32> = HashMap::new();
    let mut end_addr: Option<u32> = None;
    let mut stamp = String::new();
    let mut in_table = false;
    let mut saw_terminator = false;

    for line in text.lines() {
        let t = line.trim();

        // Page-header date stamp (first one wins; they are all identical).
        if stamp.is_empty() && t.starts_with("AS V") && t.contains(" - Source File ") {
            if let Some(pos) = t.rfind(" - ") {
                stamp = t[pos + 3..].to_string();
            }
        }

        if !in_table {
            // The `END` line: `NNN/   HHHHH :   END` (source line number /
            // address). Macro-expansion lines carry a `(N)` prefix and so
            // fail the all-digits check on the left side. Last match wins.
            if let Some((left, right)) = t.split_once(':') {
                if right.trim() == "END" {
                    if let Some((lineno, hex)) = left.trim().split_once('/') {
                        if !lineno.is_empty() && lineno.chars().all(|c| c.is_ascii_digit()) {
                            if let Ok(v) = u64::from_str_radix(hex.trim(), 16) {
                                end_addr = Some(v as u32);
                            }
                        }
                    }
                }
            }
            if t == "Symbol Table (* = unused):" {
                in_table = true;
            }
            continue;
        }

        // ── inside the symbol table ──
        if t.is_empty() {
            continue;
        }
        // The dashes underline right after the section header.
        if t.chars().all(|c| c == '-') {
            continue;
        }
        // Page headers interrupt the table mid-stream.
        if t.starts_with("AS V") {
            continue;
        }
        // Terminator: the `NNNN symbols` count line.
        if let Some(count) = t.strip_suffix(" symbols") {
            if !count.is_empty() && count.chars().all(|c| c.is_ascii_digit()) {
                saw_terminator = true;
                break;
            }
        }

        for chunk in line.split('|') {
            let c = chunk.trim();
            if c.is_empty() {
                continue;
            }
            // `*` prefix = unused — still parse it.
            let c = c.strip_prefix('*').map(str::trim_start).unwrap_or(c);
            let Some((name, value)) = c.split_once(':') else {
                return Err(format!("symbol-table chunk without `:`: `{c}`"));
            };
            let (name, value) = (name.trim(), value.trim());
            if name.is_empty() {
                return Err(format!("symbol-table chunk with empty name: `{c}`"));
            }
            // Non-numeric values: strings, char literals, floats — skipped
            // (they are never layout pins).
            if value.starts_with('"') || value.starts_with('\'') || value.contains('.') {
                continue;
            }
            // `HEXVALUE` optionally followed by ONE segment-type mark
            // (`C` = CODE, `-` = untyped equ).
            let mut toks = value.split_whitespace();
            let hex = toks
                .next()
                .ok_or_else(|| format!("symbol `{name}`: empty value"))?;
            if let Some(ty) = toks.next() {
                if ty.len() != 1 || toks.next().is_some() {
                    return Err(format!("symbol `{name}`: unrecognized value shape `{value}`"));
                }
            }
            let v = u64::from_str_radix(hex, 16)
                .map_err(|_| format!("symbol `{name}`: unparseable hex value `{hex}`"))?;
            // RAM symbols are sign-extended 64-bit; the u32 VMA is the pin.
            let v = v as u32;
            if symbols.insert(name.to_string(), v).is_some() {
                return Err(format!(
                    "duplicate symbol `{name}` in the listing symbol table — exact-name \
                     resolution would be ambiguous"
                ));
            }
        }
    }

    if !in_table {
        return Err("no `Symbol Table (* = unused):` section in the listing".to_string());
    }
    if !saw_terminator {
        return Err("symbol table ran off the end of the listing (no `NNNN symbols` line)"
            .to_string());
    }
    let end_addr =
        end_addr.ok_or_else(|| "no `END` line found before the symbol table".to_string())?;
    Ok(Listing { symbols, end_addr, stamp })
}

// ── The manifest (`repin.toml`, D-T10.2) ────────────────────────────────────

/// The declarative pin manifest. Order is load-bearing: pins.rs emits in
/// manifest order (deterministic output).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[derive(Debug)]
pub struct Manifest {
    pub rom: RomSpec,
    #[serde(default, rename = "region")]
    pub regions: Vec<RegionSpec>,
    #[serde(default, rename = "symbol")]
    pub symbols: Vec<SymbolSpec>,
    #[serde(default, rename = "offset")]
    pub offsets: Vec<OffsetSpec>,
}

/// `[rom]` — the assembled-length pins. `end_symbol` must be the `__END__`
/// sentinel: the value comes from the listing `END` line, not the table.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[derive(Debug)]
pub struct RomSpec {
    pub end_symbol: String,
    #[serde(default)]
    pub tests: Vec<String>,
}

/// `[[region]]` — a gated window. `start` is a listing symbol; the extent is
/// EITHER `end` (a listing symbol; per-shape len = end − start) OR `len`
/// (a literal, for the one region whose end address carries no symbol —
/// sound_api). `gate` names the `SIGIL_EMP_*` define whose else-arm org
/// block the tool prints (D-T10.7).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[derive(Debug)]
pub struct RegionSpec {
    pub name: String,
    pub start: String,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub len: Option<u32>,
    /// Per-shape DEBUG length override for a literal-`len` region whose debug
    /// extent differs from plain but whose end carries no listing symbol
    /// (sound_api: DEBUG asserts grow it, but a real end-symbol would ship in
    /// the release convsym appendix and perturb the byte-identical plain ROM).
    /// Only valid alongside `len`; when absent, debug_len = len.
    #[serde(default)]
    pub debug_len: Option<u32>,
    #[serde(default)]
    pub gate: Option<String>,
    #[serde(default)]
    pub tests: Vec<String>,
}

/// `[[symbol]]` — a bare cross-seam name (RAM cell, call target, equ-like
/// value). `debug_only` resolves against the debug listing only and emits a
/// single `u32` (for pins whose sole consumer is a debug-shape test).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[derive(Debug)]
pub struct SymbolSpec {
    pub name: String,
    #[serde(default)]
    pub debug_only: bool,
    #[serde(default)]
    pub tests: Vec<String>,
}

/// `[[offset]]` — `sym − region.start` (dotted locals welcome), asserted
/// shape-INVARIANT unless `per_shape = true`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[derive(Debug)]
pub struct OffsetSpec {
    pub name: String,
    pub sym: String,
    pub region: String,
    #[serde(default)]
    pub per_shape: bool,
    #[serde(default)]
    pub tests: Vec<String>,
}

/// Parse `repin.toml`. Structural validation only — cross-listing resolution
/// happens in [`resolve`].
pub fn load_manifest(src: &str) -> Result<Manifest, String> {
    let m: Manifest = toml::from_str(src).map_err(|e| format!("repin.toml parse error: {e}"))?;
    if m.rom.end_symbol != "__END__" {
        return Err(format!(
            "[rom] end_symbol must be the `__END__` sentinel (the listing END line), got `{}`",
            m.rom.end_symbol
        ));
    }
    for r in &m.regions {
        match (&r.end, r.len) {
            (Some(_), Some(_)) => {
                return Err(format!("region `{}`: give `end` OR `len`, not both", r.name))
            }
            (None, None) => {
                return Err(format!("region `{}`: needs `end` (symbol) or `len` (literal)", r.name))
            }
            _ => {}
        }
        if r.debug_len.is_some() && r.len.is_none() {
            return Err(format!(
                "region `{}`: `debug_len` is a per-shape override for a literal-`len` region — set `len` too (or use an `end` symbol for per-shape lengths)",
                r.name
            ));
        }
    }
    Ok(m)
}

// ── Resolution against the two listings ─────────────────────────────────────

/// A resolved region: per-shape base + len (lens computed `end − start` PER
/// SHAPE — core's debug len ≠ plain len).
#[derive(Debug)]
pub struct RegionPin {
    pub name: String,
    pub const_name: String,
    pub start: String,
    pub end_desc: String,
    pub gate: Option<String>,
    pub tests: Vec<String>,
    pub plain_base: u32,
    pub debug_base: u32,
    pub plain_len: u32,
    pub debug_len: u32,
}

/// A resolved symbol pin.
#[derive(Debug)]
pub struct SymbolPin {
    pub name: String,
    pub const_name: String,
    pub tests: Vec<String>,
    pub value: SymbolValue,
}

/// Per-shape values, or the debug-only single value.
#[derive(Debug)]
pub enum SymbolValue {
    Both { plain: u32, debug: u32 },
    DebugOnly(u32),
}

/// A resolved region-relative offset.
#[derive(Debug)]
pub struct OffsetPin {
    pub const_name: String,
    pub sym: String,
    pub region: String,
    pub tests: Vec<String>,
    pub value: OffsetValue,
}

/// Shape-invariant (the asserted default) or explicitly per-shape.
#[derive(Debug)]
pub enum OffsetValue {
    Invariant(u32),
    PerShape { plain: u32, debug: u32 },
}

/// The fully resolved pin set — everything [`render`] needs, in manifest
/// order.
#[derive(Debug)]
pub struct Resolved {
    pub rom_plain_len: u32,
    pub rom_debug_len: u32,
    pub rom_tests: Vec<String>,
    pub regions: Vec<RegionPin>,
    pub symbols: Vec<SymbolPin>,
    pub offsets: Vec<OffsetPin>,
}

impl Resolved {
    /// `const name → tests` for every emitted const — the rerun-hint map.
    pub fn tests_by_const(&self) -> BTreeMap<String, Vec<String>> {
        let mut map = BTreeMap::new();
        map.insert("ASSEMBLED_LEN".to_string(), self.rom_tests.clone());
        map.insert("DEBUG_ASSEMBLED_LEN".to_string(), self.rom_tests.clone());
        for r in &self.regions {
            map.insert(r.const_name.clone(), r.tests.clone());
        }
        for s in &self.symbols {
            map.insert(s.const_name.clone(), s.tests.clone());
        }
        for o in &self.offsets {
            map.insert(o.const_name.clone(), o.tests.clone());
        }
        map
    }

    /// The gated regions' ready-to-paste `org` blocks (D-T10.7). The org
    /// value is the region END — the else-arm RESUME address.
    pub fn gate_blocks(&self) -> Vec<GateBlock> {
        self.regions
            .iter()
            .filter_map(|r| {
                r.gate.as_ref().map(|g| GateBlock {
                    gate: g.clone(),
                    region: r.name.clone(),
                    const_name: r.const_name.clone(),
                    plain_end: r.plain_base + r.plain_len,
                    debug_end: r.debug_base + r.debug_len,
                })
            })
            .collect()
    }
}

/// One gated region's paste block inputs.
#[derive(Debug)]
pub struct GateBlock {
    pub gate: String,
    pub region: String,
    pub const_name: String,
    pub plain_end: u32,
    pub debug_end: u32,
}

impl GateBlock {
    /// The ready-to-paste else-arm block. Shape-invariant windows (the
    /// object-bank regions) collapse to a single `org`.
    pub fn render(&self) -> String {
        if self.plain_end == self.debug_end {
            format!(
                "; {} — gate {} resume org (shape-invariant window)\n        org     ${:X}\n",
                self.region, self.gate, self.plain_end
            )
        } else {
            format!(
                "; {} — gate {} resume org\n    ifdef __DEBUG__\n        org     ${:X}\n    \
                 else\n        org     ${:X}\n    endif\n",
                self.region, self.gate, self.debug_end, self.plain_end
            )
        }
    }
}

/// `CamelCase`/`Mixed_Snake` → `UPPER_SNAKE` const name. Deterministic:
/// underscores are inserted at lower→upper transitions and before an upper
/// followed by a lower (acronym tail), then everything is uppercased and
/// runs of `_` collapse (`MDDBG__ErrorHandler` → `MDDBG_ERROR_HANDLER`).
pub fn upper_snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::with_capacity(name.len() + 8);
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase());
            if prev.is_ascii_lowercase() || prev.is_ascii_digit() || (prev.is_ascii_uppercase() && next_lower)
            {
                out.push('_');
            }
        }
        out.push(c.to_ascii_uppercase());
    }
    // Collapse `_` runs (dunder names) and dots (locals never name consts,
    // but keep the function total).
    let mut collapsed = String::with_capacity(out.len());
    let mut prev_us = false;
    for c in out.chars() {
        let c = if c == '.' { '_' } else { c };
        if c == '_' {
            if !prev_us {
                collapsed.push('_');
            }
            prev_us = true;
        } else {
            collapsed.push(c);
            prev_us = false;
        }
    }
    collapsed
}

/// Resolve the manifest against both listings. Every failure names the
/// symbol/region — never a silent zero (D-T10.2).
pub fn resolve(m: &Manifest, plain: &Listing, debug: &Listing) -> Result<Resolved, String> {
    let mut regions = Vec::new();
    for r in &m.regions {
        let plain_base = plain.get(&r.start).map_err(|e| format!("region `{}` start: {e}", r.name))?;
        let debug_base = debug.get(&r.start).map_err(|e| format!("region `{}` start (debug): {e}", r.name))?;
        let (plain_len, debug_len, end_desc) = match (&r.end, r.len) {
            (Some(end), None) => {
                let pe = plain.get(end).map_err(|e| format!("region `{}` end: {e}", r.name))?;
                let de = debug.get(end).map_err(|e| format!("region `{}` end (debug): {e}", r.name))?;
                if pe < plain_base || de < debug_base {
                    return Err(format!(
                        "region `{}`: end `{end}` precedes start `{}` ({pe:#X} < {plain_base:#X} \
                         or {de:#X} < {debug_base:#X})",
                        r.name, r.start
                    ));
                }
                (pe - plain_base, de - debug_base, format!("`{end}`"))
            }
            (None, Some(len)) => {
                let dl = r.debug_len.unwrap_or(len);
                (len, dl, format!("start + {len:#X} plain / {dl:#X} debug (literal — no end symbol)"))
            }
            // load_manifest already rejected the other arms.
            _ => unreachable!("load_manifest validates end/len exclusivity"),
        };
        regions.push(RegionPin {
            name: r.name.clone(),
            const_name: upper_snake(&r.name),
            start: r.start.clone(),
            end_desc,
            gate: r.gate.clone(),
            tests: r.tests.clone(),
            plain_base,
            debug_base,
            plain_len,
            debug_len,
        });
    }

    let mut symbols = Vec::new();
    for s in &m.symbols {
        let value = if s.debug_only {
            SymbolValue::DebugOnly(
                debug.get(&s.name).map_err(|e| format!("debug_only symbol: {e}"))?,
            )
        } else {
            SymbolValue::Both {
                plain: plain.get(&s.name)?,
                debug: debug.get(&s.name).map_err(|e| format!("{e} (debug listing)"))?,
            }
        };
        symbols.push(SymbolPin {
            name: s.name.clone(),
            const_name: upper_snake(&s.name),
            tests: s.tests.clone(),
            value,
        });
    }

    let mut offsets = Vec::new();
    for o in &m.offsets {
        let region = regions
            .iter()
            .find(|r| r.name == o.region)
            .ok_or_else(|| format!("offset `{}`: unknown region `{}`", o.name, o.region))?;
        let pv = plain.get(&o.sym).map_err(|e| format!("offset `{}`: {e}", o.name))?;
        let dv = debug.get(&o.sym).map_err(|e| format!("offset `{}` (debug): {e}", o.name))?;
        if pv < region.plain_base || dv < region.debug_base {
            return Err(format!(
                "offset `{}`: `{}` precedes region `{}` start",
                o.name, o.sym, o.region
            ));
        }
        let (po, dofs) = (pv - region.plain_base, dv - region.debug_base);
        let value = if o.per_shape {
            OffsetValue::PerShape { plain: po, debug: dofs }
        } else if po == dofs {
            OffsetValue::Invariant(po)
        } else {
            return Err(format!(
                "offset `{}` (`{}` − `{}` start) is NOT shape-invariant: plain {po:#X} vs debug \
                 {dofs:#X}; set `per_shape = true` if that is intended",
                o.name, o.sym, o.region
            ));
        };
        offsets.push(OffsetPin {
            const_name: o.name.clone(),
            sym: o.sym.clone(),
            region: o.region.clone(),
            tests: o.tests.clone(),
            value,
        });
    }

    // Const-name collisions would shadow silently at the use site — reject.
    let mut seen: HashSet<&str> = HashSet::from(["ASSEMBLED_LEN", "DEBUG_ASSEMBLED_LEN"]);
    for name in regions
        .iter()
        .map(|r| r.const_name.as_str())
        .chain(symbols.iter().map(|s| s.const_name.as_str()))
        .chain(offsets.iter().map(|o| o.const_name.as_str()))
    {
        if !seen.insert(name) {
            return Err(format!("const name collision: `{name}` emitted twice"));
        }
    }

    Ok(Resolved {
        rom_plain_len: plain.end_addr,
        rom_debug_len: debug.end_addr,
        rom_tests: m.rom.tests.clone(),
        regions,
        symbols,
        offsets,
    })
}

// ── Rendering `pins.rs` ─────────────────────────────────────────────────────

/// Provenance strings for the generated header. The stamp lines carry the
/// `[provenance]` token and are STRIPPED by [`strip_provenance`] before any
/// staleness comparison — a rebuild that moves no pin must not read as drift.
#[derive(Debug)]
pub struct Provenance {
    pub plain_path: String,
    pub debug_path: String,
    pub plain_stamp: String,
    pub debug_stamp: String,
}

fn tests_suffix(tests: &[String]) -> String {
    if tests.is_empty() { String::new() } else { format!(" tests: {}", tests.join(", ")) }
}

/// Render the full `pins.rs` text. Deterministic: manifest order, stable
/// formatting; the only run-varying lines carry the `[provenance]` token.
pub fn render(r: &Resolved, prov: &Provenance) -> String {
    let mut s = String::new();
    let w = &mut s;
    let _ = writeln!(w, "//! GENERATED FILE — DO NOT EDIT BY HAND.");
    let _ = writeln!(w, "//!");
    let _ = writeln!(w, "//! Emitted by `cargo run -p sigil-harness --bin repin` from `repin.toml`");
    let _ = writeln!(w, "//! + the aeon listings (D-T10.3, tranche-10 step 0). Edit the MANIFEST,");
    let _ = writeln!(w, "//! then regenerate; `tests/repin_pins.rs::pins_rs_is_current` guards");
    let _ = writeln!(w, "//! staleness. All values are LISTING truth — per-shape VMAs/lengths from");
    let _ = writeln!(w, "//! `s4.lst` (plain) and `s4.debug.lst` (`__DEBUG__`).");
    let _ = writeln!(w, "//!");
    let _ = writeln!(w, "//! [provenance] plain: {} ({})", prov.plain_path, prov.plain_stamp);
    let _ = writeln!(w, "//! [provenance] debug: {} ({})", prov.debug_path, prov.debug_stamp);
    let _ = writeln!(
        w,
        "//! [provenance] {} regions, {} symbols, {} offsets",
        r.regions.len(),
        r.symbols.len(),
        r.offsets.len()
    );
    let _ = writeln!(w);
    let _ = writeln!(w, "/// A per-shape address pin: one cross-seam symbol's VMA in each shape.");
    let _ = writeln!(w, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
    let _ = writeln!(w, "pub struct Pin {{");
    let _ = writeln!(w, "    pub plain: u32,");
    let _ = writeln!(w, "    pub debug: u32,");
    let _ = writeln!(w, "}}");
    let _ = writeln!(w);
    let _ = writeln!(w, "/// A gated region's geometry. Slice as `base..base + len` — the lens are");
    let _ = writeln!(w, "/// computed `end − start` at generation, PER SHAPE (core's debug len ≠");
    let _ = writeln!(w, "/// plain len), so the slice-end bug class is unwritable.");
    let _ = writeln!(w, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
    let _ = writeln!(w, "pub struct Region {{");
    let _ = writeln!(w, "    pub plain_base: u32,");
    let _ = writeln!(w, "    pub debug_base: u32,");
    let _ = writeln!(w, "    pub plain_len: usize,");
    let _ = writeln!(w, "    pub debug_len: usize,");
    let _ = writeln!(w, "}}");
    let _ = writeln!(w);
    let _ = writeln!(w, "/// A region-relative offset that is genuinely shape-DEPENDENT (the");
    let _ = writeln!(w, "/// invariant ones emit a bare `usize`).");
    let _ = writeln!(w, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
    let _ = writeln!(w, "pub struct ShapeOffset {{");
    let _ = writeln!(w, "    pub plain: usize,");
    let _ = writeln!(w, "    pub debug: usize,");
    let _ = writeln!(w, "}}");
    let _ = writeln!(w);

    let _ = writeln!(w, "// ── ROM end (the listing `END` line address, per shape) ──");
    let _ = writeln!(w);
    let _ = writeln!(w, "/// Assembled (pre-convsym) ROM length, plain shape.{}", tests_suffix(&r.rom_tests));
    let _ = writeln!(w, "pub const ASSEMBLED_LEN: usize = {:#X};", r.rom_plain_len);
    let _ = writeln!(w, "/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape.{}", tests_suffix(&r.rom_tests));
    let _ = writeln!(w, "pub const DEBUG_ASSEMBLED_LEN: usize = {:#X};", r.rom_debug_len);
    let _ = writeln!(w);

    let _ = writeln!(w, "// ── Regions (manifest order) ──");
    for reg in &r.regions {
        let _ = writeln!(w);
        let gate = reg
            .gate
            .as_ref()
            .map(|g| format!(" — gate `{g}`"))
            .unwrap_or_default();
        let _ = writeln!(
            w,
            "/// `{}` .. {}{gate}.{}",
            reg.start,
            reg.end_desc,
            tests_suffix(&reg.tests)
        );
        let _ = writeln!(
            w,
            "pub const {}: Region = Region {{ plain_base: {:#X}, debug_base: {:#X}, plain_len: {:#X}, debug_len: {:#X} }};",
            reg.const_name, reg.plain_base, reg.debug_base, reg.plain_len, reg.debug_len
        );
    }
    let _ = writeln!(w);

    let _ = writeln!(w, "// ── Symbols (manifest order) ──");
    for sym in &r.symbols {
        let _ = writeln!(w);
        match sym.value {
            SymbolValue::Both { plain, debug } => {
                let _ = writeln!(w, "/// `{}`.{}", sym.name, tests_suffix(&sym.tests));
                let _ = writeln!(
                    w,
                    "pub const {}: Pin = Pin {{ plain: {:#X}, debug: {:#X} }};",
                    sym.const_name, plain, debug
                );
            }
            SymbolValue::DebugOnly(v) => {
                let _ = writeln!(
                    w,
                    "/// `{}` — debug-shape consumer only (`debug_only`).{}",
                    sym.name,
                    tests_suffix(&sym.tests)
                );
                let _ = writeln!(w, "pub const {}: u32 = {:#X};", sym.const_name, v);
            }
        }
    }
    let _ = writeln!(w);

    let _ = writeln!(w, "// ── Region-relative offsets (manifest order) ──");
    for off in &r.offsets {
        let _ = writeln!(w);
        match off.value {
            OffsetValue::Invariant(v) => {
                let _ = writeln!(
                    w,
                    "/// `{}` − `{}` start (shape-invariant, asserted at generation).{}",
                    off.sym,
                    off.region,
                    tests_suffix(&off.tests)
                );
                let _ = writeln!(w, "pub const {}: usize = {:#X};", off.const_name, v);
            }
            OffsetValue::PerShape { plain, debug } => {
                let _ = writeln!(
                    w,
                    "/// `{}` − `{}` start (per-shape).{}",
                    off.sym,
                    off.region,
                    tests_suffix(&off.tests)
                );
                let _ = writeln!(
                    w,
                    "pub const {}: ShapeOffset = ShapeOffset {{ plain: {:#X}, debug: {:#X} }};",
                    off.const_name, plain, debug
                );
            }
        }
    }
    s
}

/// Drop the `[provenance]` lines — the staleness/`--check` comparison basis.
/// A listing rebuild that moves NO pin changes only the date stamps; that
/// must not read as drift (and the committed provenance keeps naming the
/// listings that last CHANGED a value).
pub fn strip_provenance(text: &str) -> String {
    text.lines()
        .filter(|l| !l.contains("[provenance]"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Pin-level diff (D-T10.4 review surface) ─────────────────────────────────

/// One changed const between two renderings of `pins.rs`.
#[derive(Debug)]
pub struct PinChange {
    pub name: String,
    /// The old initializer text (`None` = newly added pin).
    pub old: Option<String>,
    /// The new initializer text (`None` = pin removed).
    pub new: Option<String>,
}

/// Extract `const name → initializer text` from a `pins.rs` rendering.
fn const_lines(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("pub const ") else { continue };
        let Some((name, tail)) = rest.split_once(':') else { continue };
        let Some((_, init)) = tail.split_once('=') else { continue };
        out.push((name.trim().to_string(), init.trim().trim_end_matches(';').to_string()));
    }
    out
}

/// Diff two renderings pin-by-pin: changed, added, and removed consts, in
/// new-text order (removed ones last).
pub fn diff_pins(old_text: &str, new_text: &str) -> Vec<PinChange> {
    let old: BTreeMap<String, String> = const_lines(old_text).into_iter().collect();
    let new = const_lines(new_text);
    let new_names: HashSet<&str> = new.iter().map(|(n, _)| n.as_str()).collect();
    let mut changes = Vec::new();
    for (name, init) in &new {
        match old.get(name) {
            Some(o) if o == init => {}
            other => changes.push(PinChange {
                name: name.clone(),
                old: other.cloned(),
                new: Some(init.clone()),
            }),
        }
    }
    for (name, init) in &old {
        if !new_names.contains(name.as_str()) {
            changes.push(PinChange { name: name.clone(), old: Some(init.clone()), new: None });
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A vendored excerpt of the real `s4.lst` shape: the END line, the table
    /// header, a page-header INTERRUPTION mid-table, a `*`-prefixed (unused)
    /// entry, a sign-extended RAM value, a string value, a char-literal
    /// value, a float value, a dotted local, and the count terminator.
    const EXCERPT: &str = "\
(1)  332/   658B4 : [330]                    endif
     333/   658B4 :                         END
 AS V1.42 Beta [Bld 212] - Source File main.asm - Page 796 - 07/10/2026 10:03:54 PM


  Symbol Table (* = unused):
  --------------------------

 ACCELERATION :                   C - |  AF_BACK :                       FE - |
*AF_CALLBACK :                   FA - |
*ARCHITECTURE :                                      \"x86_64-unknown-linux\" - |
 AnimateSprite :               2D78 C |  AnimateSprite.cc_delete :     2E7C C |
 AS V1.42 Beta [Bld 212] - Source File main.asm - Page 797 - 07/10/2026 10:03:54 PM


 Air_FloorLandBanded.grounded :                                       10A9A C |
 Player_1 :                                                FFFFFFFFFFFF89EE C |
*CONSTPI :        3.141592653589793 - | *CROSS_RESET_MAGIC :         'INIT' - |
 GAME_CONSOLE :  \"SEGA GENESIS    \" - |

   3296 symbols
    426 unused symbols

 AS V1.42 Beta [Bld 212] - Source File main.asm - Page 830 - 07/10/2026 10:03:54 PM


  Defined Macros:
  ---------------
";

    #[test]
    fn excerpt_parses_every_format_fact() {
        let l = parse_listing(EXCERPT).expect("excerpt must parse");
        // The END line address.
        assert_eq!(l.end_addr, 0x658B4);
        // Two-per-line entries + the equ type mark.
        assert_eq!(l.get("ACCELERATION").unwrap(), 0xC);
        assert_eq!(l.get("AF_BACK").unwrap(), 0xFE);
        // `*` = unused — still parsed.
        assert_eq!(l.get("AF_CALLBACK").unwrap(), 0xFA);
        // CODE entries + a dotted local.
        assert_eq!(l.get("AnimateSprite").unwrap(), 0x2D78);
        assert_eq!(l.get("AnimateSprite.cc_delete").unwrap(), 0x2E7C);
        // An entry AFTER the page-header interruption, full-line form.
        assert_eq!(l.get("Air_FloorLandBanded.grounded").unwrap(), 0x10A9A);
        // Sign-extended RAM value truncates to the u32 VMA.
        assert_eq!(l.get("Player_1").unwrap(), 0xFFFF_89EE);
        // Non-numeric values are skipped, not mangled.
        assert!(l.get("ARCHITECTURE").is_err());
        assert!(l.get("CONSTPI").is_err());
        assert!(l.get("CROSS_RESET_MAGIC").is_err());
        assert!(l.get("GAME_CONSOLE").is_err());
        // Provenance stamp from the page header.
        assert_eq!(l.stamp, "07/10/2026 10:03:54 PM");
        assert_eq!(l.symbol_count(), 7);
    }

    #[test]
    fn unknown_symbol_is_a_hard_error_naming_it() {
        let l = parse_listing(EXCERPT).unwrap();
        let err = l.get("RunObjects").unwrap_err();
        assert!(err.contains("RunObjects"), "error must name the symbol: {err}");
        // Exact match only — a prefix of a known name resolves nothing.
        assert!(l.get("AnimateSprite.cc").is_err());
    }

    #[test]
    fn duplicate_symbol_name_is_a_hard_error() {
        let dup = "\
     10/    100 :                         END

  Symbol Table (* = unused):
  --------------------------

 Twice :                          1 - |  Twice :                          2 - |

   2 symbols
";
        let err = parse_listing(dup).unwrap_err();
        assert!(err.contains("duplicate symbol `Twice`"), "{err}");
    }

    #[test]
    fn missing_table_or_end_line_fail_loudly() {
        assert!(parse_listing("just noise\n").unwrap_err().contains("Symbol Table"));
        let no_end = "\
  Symbol Table (* = unused):
  --------------------------
 A :                              1 - |

   1 symbols
";
        assert!(parse_listing(no_end).unwrap_err().contains("END"));
    }

    #[test]
    fn upper_snake_covers_the_inventory_shapes() {
        assert_eq!(upper_snake("DeleteObject"), "DELETE_OBJECT");
        assert_eq!(upper_snake("Sound_PlaySFX"), "SOUND_PLAY_SFX");
        assert_eq!(upper_snake("MDDBG__ErrorHandler"), "MDDBG_ERROR_HANDLER");
        assert_eq!(upper_snake("OJZ_Sec0_Blocks"), "OJZ_SEC0_BLOCKS");
        assert_eq!(upper_snake("BootData_VDPRegs"), "BOOT_DATA_VDP_REGS");
        assert_eq!(upper_snake("Ring_Sfx_Speaker"), "RING_SFX_SPEAKER");
        assert_eq!(upper_snake("Act_len"), "ACT_LEN");
        assert_eq!(upper_snake("animate"), "ANIMATE");
    }

    /// End-to-end over the excerpt: manifest → resolve → render, checking
    /// the computed len, the offset subtraction, and determinism.
    #[test]
    fn resolve_and_render_from_the_excerpt() {
        // A second "debug" listing: everything slid by +0x10, END larger.
        let debug_excerpt = EXCERPT
            .replace("2D78", "2D88")
            .replace("2E7C", "2E8C")
            .replace("658B4", "673A2")
            .replace("FFFFFFFFFFFF89EE", "FFFFFFFFFFFF8A10");
        let plain = parse_listing(EXCERPT).unwrap();
        let debug = parse_listing(&debug_excerpt).unwrap();

        let manifest = load_manifest(
            r#"
[rom]
end_symbol = "__END__"
tests = ["m1d_rom"]

[[region]]
name = "animate"
start = "AnimateSprite"
end = "AnimateSprite.cc_delete"
gate = "SIGIL_EMP_ANIMATE"
tests = ["animate_port"]

[[symbol]]
name = "Player_1"
tests = ["rings_port"]

[[offset]]
name = "CC_DELETE_OFF"
sym = "AnimateSprite.cc_delete"
region = "animate"
tests = ["animate_port"]
"#,
        )
        .unwrap();

        let resolved = resolve(&manifest, &plain, &debug).unwrap();
        assert_eq!(resolved.rom_plain_len, 0x658B4);
        assert_eq!(resolved.rom_debug_len, 0x673A2);
        let reg = &resolved.regions[0];
        assert_eq!((reg.plain_base, reg.debug_base), (0x2D78, 0x2D88));
        assert_eq!((reg.plain_len, reg.debug_len), (0x104, 0x104));
        match resolved.offsets[0].value {
            OffsetValue::Invariant(v) => assert_eq!(v, 0x104),
            _ => panic!("offset must resolve shape-invariant"),
        }

        let prov = Provenance {
            plain_path: "s4.lst".into(),
            debug_path: "s4.debug.lst".into(),
            plain_stamp: plain.stamp.clone(),
            debug_stamp: debug.stamp.clone(),
        };
        let text = render(&resolved, &prov);
        assert!(text.contains("pub const ANIMATE: Region = Region { plain_base: 0x2D78, debug_base: 0x2D88, plain_len: 0x104, debug_len: 0x104 };"));
        assert!(text.contains("pub const PLAYER_1: Pin = Pin { plain: 0xFFFF89EE, debug: 0xFFFF8A10 };"));
        assert!(text.contains("pub const CC_DELETE_OFF: usize = 0x104;"));
        // Deterministic: same inputs, same bytes.
        assert_eq!(text, render(&resolved, &prov));
        // Gate block prints the per-shape RESUME orgs (the region ENDS).
        let blocks = resolved.gate_blocks();
        assert_eq!(blocks.len(), 1);
        let block = blocks[0].render();
        assert!(block.contains("SIGIL_EMP_ANIMATE"));
        assert!(block.contains("org     $2E7C"), "{block}");
        assert!(block.contains("org     $2E8C"), "{block}");
    }

    #[test]
    fn non_invariant_offset_without_per_shape_is_rejected() {
        // Debug side: the LOCAL slides +0x20 but the base only +0x10 —
        // the offset is not invariant.
        let debug_excerpt = EXCERPT.replace("2D78", "2D88").replace("2E7C", "2EAC");
        let plain = parse_listing(EXCERPT).unwrap();
        let debug = parse_listing(&debug_excerpt).unwrap();
        let manifest = load_manifest(
            r#"
[rom]
end_symbol = "__END__"

[[region]]
name = "animate"
start = "AnimateSprite"
end = "AnimateSprite.cc_delete"

[[offset]]
name = "CC_DELETE_OFF"
sym = "AnimateSprite.cc_delete"
region = "animate"
"#,
        )
        .unwrap();
        let err = resolve(&manifest, &plain, &debug).unwrap_err();
        assert!(err.contains("NOT shape-invariant"), "{err}");
        // With per_shape = true the same geometry resolves.
        let manifest = load_manifest(
            r#"
[rom]
end_symbol = "__END__"

[[region]]
name = "animate"
start = "AnimateSprite"
end = "AnimateSprite.cc_delete"

[[offset]]
name = "CC_DELETE_OFF"
sym = "AnimateSprite.cc_delete"
region = "animate"
per_shape = true
"#,
        )
        .unwrap();
        let resolved = resolve(&manifest, &plain, &debug).unwrap();
        match resolved.offsets[0].value {
            OffsetValue::PerShape { plain, debug } => {
                assert_eq!((plain, debug), (0x104, 0x124));
            }
            _ => panic!("per_shape offset must keep both values"),
        }
    }

    #[test]
    fn manifest_validation_rejects_bad_shapes() {
        // end AND len.
        let err = load_manifest(
            "[rom]\nend_symbol = \"__END__\"\n[[region]]\nname = \"x\"\nstart = \"A\"\nend = \"B\"\nlen = 4\n",
        )
        .unwrap_err();
        assert!(err.contains("not both"), "{err}");
        // Neither end nor len.
        let err = load_manifest(
            "[rom]\nend_symbol = \"__END__\"\n[[region]]\nname = \"x\"\nstart = \"A\"\n",
        )
        .unwrap_err();
        assert!(err.contains("needs `end`"), "{err}");
        // Wrong sentinel.
        let err = load_manifest("[rom]\nend_symbol = \"EndOfRom\"\n").unwrap_err();
        assert!(err.contains("__END__"), "{err}");
        // Unknown key (typo guard).
        assert!(load_manifest("[rom]\nend_symbol = \"__END__\"\nbogus = 1\n").is_err());
    }

    #[test]
    fn diff_reports_changed_added_and_removed_pins() {
        let old = "pub const A: usize = 0x10;\npub const B: usize = 0x20;\npub const GONE: usize = 0x30;\n";
        let new = "pub const A: usize = 0x10;\npub const B: usize = 0x24;\npub const NEW: usize = 0x40;\n";
        let changes = diff_pins(old, new);
        let names: Vec<&str> = changes.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["B", "NEW", "GONE"]);
        assert_eq!(changes[0].old.as_deref(), Some("0x20"));
        assert_eq!(changes[0].new.as_deref(), Some("0x24"));
        assert!(changes[1].old.is_none());
        assert!(changes[2].new.is_none());
    }

    #[test]
    fn strip_provenance_drops_only_the_stamp_lines() {
        let text = "//! header\n//! [provenance] plain: x (stamp)\npub const A: usize = 1;\n";
        let stripped = strip_provenance(text);
        assert!(!stripped.contains("[provenance]"));
        assert!(stripped.contains("pub const A"));
        assert!(stripped.contains("//! header"));
    }
}
