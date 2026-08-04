//! `repin` — the listing-derived pin generator (tranche-10 step 0).
//!
//! Re-pin waves used to be string substitution over ~115 hand-typed layout
//! literals scattered across ~16 test files; every substitution error cost a
//! suite run to find. This module kills that bug class: it takes a resolved
//! `name → address` map for BOTH shapes, resolves the declarative manifest
//! (`crates/sigil-harness/repin.toml`), and renders the generated `src/pins.rs`
//! the port tests import. Design: `docs/superpowers/notes/
//! 2026-07-10-tranche10-repin-design.md` (D-T10.1..D-T10.9).
//!
//! ADDRESS SOURCE (Stage-3 P4c, kill-list row 34): the addresses come from
//! SIGIL'S OWN resolved layout (`native::sigil_native_symbol_listing` — the
//! fully-resolved symbol table: labels + folded equates incl. `MDDBG__*`, `.emp`
//! locals demangled, section-END markers synthesized), NOT an asl `.lst`. The
//! `Listing` is now a plain `name → address` map built via
//! [`Listing::from_symbols`]; the asl `Symbol Table` parser is deleted.
//!
//! The binary front-end lives in `src/bin/repin.rs`; the logic lives here so
//! the staleness test (D-T10.5, `tests/repin_pins.rs::pins_rs_is_current`)
//! can regenerate in-memory and compare against the committed file.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

use serde::Deserialize;

// ── The symbol listing (a resolved `name → address` map) ────────────────────

/// A resolved symbol listing: the `name → address` map plus the ROM-end address
/// and a provenance stamp. Built via [`Listing::from_symbols`] from sigil's own
/// resolved layout (Stage-3 P4c).
#[derive(Debug)]
pub struct Listing {
    symbols: HashMap<String, u32>,
    /// PHASE-BANK label LOAD addresses (T4): for a `vma:`-windowed bank section
    /// whose labels resolve at a VMA distinct from their LMA, `name → lma`. A
    /// `phase_bank` region resolves its base HERE, not in `symbols` (which holds the
    /// phase VMA). Empty unless the layout carries a phase-bank section
    /// (`soundbankhead`); populated via [`Listing::with_phase_lma`].
    phase_lma: HashMap<String, u32>,
    /// The ROM end address (`EndOfRom`) — the assembled ROM length.
    pub end_addr: u32,
    /// Provenance text (a build description), carried into the generated file's
    /// provenance header.
    pub stamp: String,
}

impl Listing {
    /// Build a `Listing` directly from a resolved `name → address` map (the
    /// sigil-native source, Stage-3 P4c) instead of parsing an asl `.lst`. The
    /// `stamp` is provenance text (a build date, not an asl page-header). The
    /// phase-bank LMA map is empty — attach it with [`Listing::with_phase_lma`]
    /// when the layout carries a `vma:`-windowed bank.
    pub fn from_symbols(symbols: HashMap<String, u32>, end_addr: u32, stamp: String) -> Listing {
        Listing { symbols, phase_lma: HashMap::new(), end_addr, stamp }
    }

    /// Attach the phase-bank label LMA map (T4, `native::phase_bank_lmas`). A
    /// `phase_bank` region's base resolves against this map (the LMA) instead of the
    /// VMA `symbols` carries.
    pub fn with_phase_lma(mut self, phase_lma: HashMap<String, u32>) -> Listing {
        self.phase_lma = phase_lma;
        self
    }

    /// Exact-match lookup. Unknown symbol = HARD ERROR naming it (D-T10.2 —
    /// never a silent 0). `Prof_RunObjects` vs `RunObjects` are DIFFERENT
    /// names; no prefix/suffix matching happens here.
    pub fn get(&self, name: &str) -> Result<u32, String> {
        self.symbols
            .get(name)
            .copied()
            .ok_or_else(|| format!("symbol `{name}` not found in the listing symbol table"))
    }

    /// The PLACEMENT address of a REGION boundary symbol `name` — the address the
    /// region's base/end pins to (T4). A region's base must be the section's LOAD
    /// address, so a PinnedBaked re-bootstrap (which feeds it straight in as
    /// `lma_base`) places the bytes where they belong.
    ///
    /// PHASE-BANK AUTO-DETECTION: if `name` is a phase-bank section label (present in
    /// [`with_phase_lma`], populated ONLY for `vma:`-windowed banks whose labels
    /// resolve at a VMA distinct from their LMA), return its LMA; otherwise the plain
    /// VMA (which equals the LMA for every non-phase section). The distinction is
    /// driven by the RESOLVED layout, not a hand-maintained manifest flag — so it can
    /// neither be forgotten nor go stale, and `repin.toml` stays frozen. Only region
    /// boundaries route through here; bare symbol pins keep their VMA via [`get`]
    /// (`SongTable`/`SongPatchTable` live in the phase window and are referenced at
    /// their $8000-window VMA, so they must NOT be rebased).
    pub fn placement(&self, name: &str) -> Result<u32, String> {
        if let Some(&lma) = self.phase_lma.get(name) {
            return Ok(lma);
        }
        self.get(name)
    }

    /// Number of parsed (numeric) symbols — provenance/debug aid.
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }
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
    /// Per-shape DEBUG end symbol: when the region's END is a DIFFERENT listing
    /// symbol in the debug shape than in plain (the fault-handler shape split of
    /// review item 29 part 4 — `ojz_scroll_test` ends at `ReleaseFault` in the
    /// RELEASE shape but at `BusError` in DEBUG, and neither symbol exists in the
    /// other shape). Only valid alongside `end`; when absent, debug end = `end`.
    #[serde(default)]
    pub debug_end: Option<String>,
    #[serde(default)]
    pub len: Option<u32>,
    /// Per-shape DEBUG length override for a literal-`len` region whose debug
    /// extent differs from plain but whose end carries no listing symbol
    /// (sound_api: DEBUG asserts grow it, but a real end-symbol would enter the
    /// convsym appendix and perturb the frozen full-file goldens — pre-item-29
    /// that hit the plain ROM too; the appendix is DEBUG-only now, so the blast
    /// radius is the debug full file, and the override stays the honest way to
    /// state a per-shape extent without inventing a symbol).
    /// Only valid alongside `len`; when absent, debug_len = len.
    #[serde(default)]
    pub debug_len: Option<u32>,
    #[serde(default)]
    pub gate: Option<String>,
    #[serde(default)]
    pub tests: Vec<String>,
    /// Region that exists ONLY in the debug shape (the twin is whole-file
    /// `ifdef __DEBUG__` — compression_selftest): `start`/`end` resolve
    /// against the DEBUG listing only; plain_len = 0; plain_base = the plain
    /// address of `plain_anchor` (the next placement — keeps the emitted
    /// `Region` shape unchanged for every consumer). Requires `plain_anchor`.
    #[serde(default)]
    pub debug_only: bool,
    /// The plain-listing symbol whose address becomes a `debug_only` region's
    /// plain_base (the next placement). Only valid with `debug_only`.
    #[serde(default)]
    pub plain_anchor: Option<String>,
    /// Region that exists ONLY in the RELEASE (plain) shape — the mirror of
    /// `debug_only`, for `release_fault` (review item 29 part 4: the DEBUG shape
    /// strips it, exactly as the RELEASE shape strips the `debug_only`
    /// error_handler island). `start`/`end` resolve against the PLAIN listing
    /// only; debug_len = 0; debug_base = the debug address of `debug_anchor` (the
    /// next placement — keeps the emitted `Region` shape unchanged for every
    /// consumer). Requires `debug_anchor`.
    #[serde(default)]
    pub plain_only: bool,
    /// The debug-listing symbol whose address becomes a `plain_only` region's
    /// debug_base (the next placement). Only valid with `plain_only`.
    #[serde(default)]
    pub debug_anchor: Option<String>,
}

/// `[[symbol]]` — a bare cross-seam name (RAM cell, call target, equ-like
/// value). `debug_only` resolves against the debug listing only and emits a
/// single `u32` (for pins whose sole consumer is a debug-shape test).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[derive(Debug)]
pub struct SymbolSpec {
    pub name: String,
    /// Override for the emitted const name (default: `upper_snake(name)`). Needed
    /// when a symbol's snake-upper would collide with a region const — e.g. the
    /// `Plane_Buffer` RAM base vs the `plane_buffer` region (both → PLANE_BUFFER);
    /// the base pins as `PLANE_BUFFER_BASE`.
    #[serde(default)]
    pub const_name: Option<String>,
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
        if r.debug_only && r.plain_anchor.is_none() {
            return Err(format!(
                "region `{}`: `debug_only` needs `plain_anchor` (the plain-listing symbol of the next placement — a debug-only region's start never appears in the plain listing)",
                r.name
            ));
        }
        if r.plain_anchor.is_some() && !r.debug_only {
            return Err(format!(
                "region `{}`: `plain_anchor` is only meaningful with `debug_only = true`",
                r.name
            ));
        }
        if r.plain_only && r.debug_anchor.is_none() {
            return Err(format!(
                "region `{}`: `plain_only` needs `debug_anchor` (the debug-listing symbol of the next placement — a plain-only region's start never appears in the debug listing)",
                r.name
            ));
        }
        if r.debug_anchor.is_some() && !r.plain_only {
            return Err(format!(
                "region `{}`: `debug_anchor` is only meaningful with `plain_only = true`",
                r.name
            ));
        }
        if r.debug_only && r.plain_only {
            return Err(format!(
                "region `{}`: `debug_only` and `plain_only` are mutually exclusive",
                r.name
            ));
        }
        if r.debug_end.is_some() && r.end.is_none() {
            return Err(format!(
                "region `{}`: `debug_end` is a per-shape end SYMBOL override — set `end` too (it is the plain-shape end)",
                r.name
            ));
        }
        if r.debug_end.is_some() && (r.debug_only || r.plain_only) {
            return Err(format!(
                "region `{}`: `debug_end` is for a both-shapes region with a shape-split end; a `debug_only`/`plain_only` region resolves against ONE listing",
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
    /// Debug-only region (plain shape empty; gate block prints only the
    /// `ifdef __DEBUG__` resume arm).
    pub debug_only: bool,
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
        if r.debug_only {
            // Debug-only region (whole-file `ifdef __DEBUG__` twin): the
            // debug listing carries start/end; the plain shape has ZERO bytes
            // at plain_anchor's address.
            let anchor = r.plain_anchor.as_ref().expect("load_manifest validated plain_anchor");
            let plain_base = plain
                .get(anchor)
                .map_err(|e| format!("region `{}` plain_anchor: {e}", r.name))?;
            let debug_base =
                debug.get(&r.start).map_err(|e| format!("region `{}` start (debug): {e}", r.name))?;
            let (debug_len, end_desc) = match (&r.end, r.len) {
                (Some(end), None) => {
                    let de = debug
                        .get(end)
                        .map_err(|e| format!("region `{}` end (debug): {e}", r.name))?;
                    if de < debug_base {
                        return Err(format!(
                            "region `{}`: end `{end}` precedes start `{}` ({de:#X} < {debug_base:#X})",
                            r.name, r.start
                        ));
                    }
                    (de - debug_base, format!("`{end}` (debug-only region; plain empty at `{anchor}`)"))
                }
                (None, Some(len)) => {
                    (len, format!("start + {len:#X} (debug-only literal; plain empty at `{anchor}`)"))
                }
                _ => unreachable!("load_manifest validates end/len exclusivity"),
            };
            regions.push(RegionPin {
                name: r.name.clone(),
                const_name: upper_snake(&r.name),
                debug_only: true,
                start: r.start.clone(),
                end_desc,
                gate: r.gate.clone(),
                tests: r.tests.clone(),
                plain_base,
                debug_base,
                plain_len: 0,
                debug_len,
            });
            continue;
        }
        if r.plain_only {
            // Plain-only region (the mirror of debug_only — review item 29 part 4's
            // release_fault): the PLAIN listing carries start/end; the debug shape
            // has ZERO bytes at debug_anchor's address.
            let anchor = r.debug_anchor.as_ref().expect("load_manifest validated debug_anchor");
            let debug_base = debug
                .get(anchor)
                .map_err(|e| format!("region `{}` debug_anchor: {e}", r.name))?;
            let plain_base =
                plain.get(&r.start).map_err(|e| format!("region `{}` start: {e}", r.name))?;
            let (plain_len, end_desc) = match (&r.end, r.len) {
                (Some(end), None) => {
                    let pe =
                        plain.get(end).map_err(|e| format!("region `{}` end: {e}", r.name))?;
                    if pe < plain_base {
                        return Err(format!(
                            "region `{}`: end `{end}` precedes start `{}` ({pe:#X} < {plain_base:#X})",
                            r.name, r.start
                        ));
                    }
                    (pe - plain_base, format!("`{end}` (plain-only region; debug empty at `{anchor}`)"))
                }
                (None, Some(len)) => {
                    (len, format!("start + {len:#X} (plain-only literal; debug empty at `{anchor}`)"))
                }
                _ => unreachable!("load_manifest validates end/len exclusivity"),
            };
            regions.push(RegionPin {
                name: r.name.clone(),
                const_name: upper_snake(&r.name),
                debug_only: false,
                start: r.start.clone(),
                end_desc,
                gate: r.gate.clone(),
                tests: r.tests.clone(),
                plain_base,
                debug_base,
                plain_len,
                debug_len: 0,
            });
            continue;
        }
        let plain_base =
            plain.placement(&r.start).map_err(|e| format!("region `{}` start: {e}", r.name))?;
        let debug_base = debug
            .placement(&r.start)
            .map_err(|e| format!("region `{}` start (debug): {e}", r.name))?;
        let (plain_len, debug_len, end_desc) = match (&r.end, r.len) {
            (Some(end), None) => {
                // `debug_end` overrides the END SYMBOL for the debug shape (the
                // fault-handler shape split: ojz_scroll_test ends at ReleaseFault in
                // plain, BusError in debug — neither exists in the other listing).
                let debug_end = r.debug_end.as_deref().unwrap_or(end);
                let pe = plain.placement(end).map_err(|e| format!("region `{}` end: {e}", r.name))?;
                let de = debug
                    .placement(debug_end)
                    .map_err(|e| format!("region `{}` end (debug): {e}", r.name))?;
                if pe < plain_base || de < debug_base {
                    return Err(format!(
                        "region `{}`: end (`{end}` plain / `{debug_end}` debug) precedes start `{}` ({pe:#X} < {plain_base:#X} \
                         or {de:#X} < {debug_base:#X})",
                        r.name, r.start
                    ));
                }
                let desc = if r.debug_end.is_some() {
                    format!("`{end}` plain / `{debug_end}` debug")
                } else {
                    format!("`{end}`")
                };
                (pe - plain_base, de - debug_base, desc)
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
            debug_only: false,
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
            const_name: s.const_name.clone().unwrap_or_else(|| upper_snake(&s.name)),
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
    let _ = writeln!(w, "//! + SIGIL'S OWN resolved layout (Stage-3 P4c; the asl-`.lst` parse retired).");
    let _ = writeln!(w, "//! Edit the MANIFEST, then regenerate; `tests/repin_pins.rs::");
    let _ = writeln!(w, "//! pins_rs_is_current` guards staleness. All values are per-shape VMAs/lengths");
    let _ = writeln!(w, "//! from sigil's native canonical resolve (plain + `__DEBUG__`).");
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

    /// A test `Listing` from `(name, addr)` pairs + the ROM-end address (the
    /// sigil-native `Listing::from_symbols` path — the asl `.lst` parser is gone).
    fn test_listing(pairs: &[(&str, u32)], end: u32) -> Listing {
        let map: HashMap<String, u32> = pairs.iter().map(|(n, v)| (n.to_string(), *v)).collect();
        Listing::from_symbols(map, end, "test".into())
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
        // A second "debug" listing: the region + local slid +0x10, END larger.
        let plain = test_listing(
            &[("AnimateSprite", 0x2D78), ("AnimateSprite.cc_delete", 0x2E7C), ("Player_1", 0xFFFF_89EE)],
            0x658B4,
        );
        let debug = test_listing(
            &[("AnimateSprite", 0x2D88), ("AnimateSprite.cc_delete", 0x2E8C), ("Player_1", 0xFFFF_8A10)],
            0x673A2,
        );

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
    }

    /// A region that exists ONLY in the debug shape (the twin is whole-file
    /// `ifdef __DEBUG__` — compression_selftest): `debug_only = true` resolves
    /// start/end against the DEBUG listing only; plain_len = 0; plain_base =
    /// `plain_anchor`'s plain address (the next placement).
    #[test]
    fn debug_only_region_resolves_debug_listing_only() {
        let plain = test_listing(&[("AnimateSprite", 0x2D78)], 0x658B4);
        let debug =
            test_listing(&[("SelfTest", 0x6FDC), ("SelfTest.done", 0x7204)], 0x673A2);
        assert!(plain.get("SelfTest").is_err(), "premise: SelfTest absent from plain");
        let manifest = load_manifest(
            r#"
[rom]
end_symbol = "__END__"

[[region]]
name = "selftest"
start = "SelfTest"
end = "SelfTest.done"
debug_only = true
plain_anchor = "AnimateSprite"
gate = "SIGIL_EMP_SELFTEST"
tests = ["selftest_port"]
"#,
        )
        .unwrap();
        let resolved = resolve(&manifest, &plain, &debug).unwrap();
        let reg = &resolved.regions[0];
        assert_eq!((reg.plain_base, reg.debug_base), (0x2D78, 0x6FDC));
        assert_eq!((reg.plain_len, reg.debug_len), (0, 0x228));

        // A debug_only region WITHOUT plain_anchor is a manifest error.
        let err = load_manifest(
            r#"
[rom]
end_symbol = "__END__"

[[region]]
name = "selftest"
start = "SelfTest"
end = "SelfTest.done"
debug_only = true
"#,
        )
        .unwrap_err();
        assert!(err.contains("plain_anchor"), "error names the missing key: {err}");

        // plain_anchor WITHOUT debug_only is a manifest error too.
        let err = load_manifest(
            r#"
[rom]
end_symbol = "__END__"

[[region]]
name = "selftest"
start = "AnimateSprite"
end = "AnimateSprite.cc_delete"
plain_anchor = "AnimateSprite"
"#,
        )
        .unwrap_err();
        assert!(err.contains("debug_only"), "error names the missing flag: {err}");
    }

    #[test]
    fn non_invariant_offset_without_per_shape_is_rejected() {
        // Debug side: the LOCAL slides +0x20 but the base only +0x10 —
        // the offset is not invariant.
        let plain =
            test_listing(&[("AnimateSprite", 0x2D78), ("AnimateSprite.cc_delete", 0x2E7C)], 0x658B4);
        let debug =
            test_listing(&[("AnimateSprite", 0x2D88), ("AnimateSprite.cc_delete", 0x2EAC)], 0x673A2);
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

    /// T4 — a phase-bank region pins its base to the section's LMA (the placement
    /// address), NOT the phase VMA the symbol listing carries. Auto-detected: the
    /// region's start being a phase-bank label (present in the phase-LMA map) IS the
    /// signal — no manifest flag, so `repin.toml` stays frozen and the rule can't go
    /// stale. The soundness catch: a PinnedBaked re-bootstrap feeds `Region::plain_base`
    /// into `emp_map_toml` as the bank's `lma_base`, so a base holding the phase VMA
    /// ($8000) would place the bank there instead of its true LMA ($58000).
    #[test]
    fn phase_bank_region_pins_the_lma_not_the_vma() {
        // The listing resolves the phase-bank head label at its VMA ($8000 window),
        // exactly as `resolved_symbols` does for a `vma:`-windowed section. A second,
        // ordinary region (Foo) shares the manifest to prove non-phase starts keep the
        // VMA — only the phase-bank label rebases.
        let base_syms = &[("SoundTablesZ80_Head", 0x8000u32), ("Foo", 0x1234u32)];
        // The phase-bank LMA map carries the LOAD address ($58000) for the phase label
        // ONLY (native::phase_bank_lmas populates only vma!=lma bank sections).
        let phase: HashMap<String, u32> =
            [("SoundTablesZ80_Head".to_string(), 0x58000u32)].into_iter().collect();
        let plain = test_listing(base_syms, 0x60000).with_phase_lma(phase.clone());
        let debug = test_listing(base_syms, 0x62000).with_phase_lma(phase);

        let manifest = load_manifest(
            r#"
[rom]
end_symbol = "__END__"

[[region]]
name = "soundbankhead"
start = "SoundTablesZ80_Head"
len = 0x607
gate = "SIGIL_EMP_SOUNDBANKHEAD"
tests = ["soundbankhead_port"]

[[region]]
name = "foo"
start = "Foo"
len = 0x10
"#,
        )
        .unwrap();
        let resolved = resolve(&manifest, &plain, &debug).unwrap();
        let bank = &resolved.regions[0];
        // The phase-bank base pins the LMA in BOTH shapes — never the $8000 phase VMA.
        assert_eq!(bank.plain_base, 0x58000, "phase-bank base must pin the LMA, not the VMA");
        assert_eq!(bank.debug_base, 0x58000, "phase-bank base must pin the LMA, not the VMA");
        assert_eq!((bank.plain_len, bank.debug_len), (0x607, 0x607));
        // An ordinary region keeps its VMA base (auto-detection does not over-reach).
        let foo = &resolved.regions[1];
        assert_eq!((foo.plain_base, foo.debug_base), (0x1234, 0x1234));

        // Without the phase-LMA map (a listing that carries no phase-bank section),
        // the same manifest resolves the region at its VMA — no crash, no rebase. This
        // is the byte-identity path for any layout with no `vma:`-windowed bank.
        let plain_no_phase = test_listing(base_syms, 0x60000);
        let debug_no_phase = test_listing(base_syms, 0x62000);
        let resolved = resolve(&manifest, &plain_no_phase, &debug_no_phase).unwrap();
        assert_eq!(resolved.regions[0].plain_base, 0x8000, "no phase map ⇒ base is the plain VMA");
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
