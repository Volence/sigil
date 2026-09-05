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
    /// The placed ROM SECTION table (REPIN-END): every byte-carrying section's name,
    /// LMA and one-past-end (`lma + image_len`, the same derivation that synthesizes
    /// the `<Base>_End` markers). A region boundary spelled `section:<name>` resolves
    /// HERE — `start` to the section's LMA, `end` to the section's OWN end — so a
    /// region can end where its bytes end instead of where the successor's head label
    /// lands after alignment pad. Also drives the pad-inclusion warning for bare-label
    /// ends ([`Listing::pad_past_content`]). Empty unless attached via
    /// [`Listing::with_sections`].
    sections: Vec<SectionExtent>,
    /// Which ROM section DEFINES each label (parcel R6, `native::section_label_owners`).
    /// The address cannot tell a region's own end from a successor's head label that
    /// happens to sit flush against it; ownership can. Empty unless attached via
    /// [`Listing::with_label_owners`], and an empty map means the check is skipped —
    /// documented at its one use in `judge_end`.
    label_owners: HashMap<String, String>,
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
        Listing {
            symbols,
            phase_lma: HashMap::new(),
            sections: Vec::new(),
            label_owners: HashMap::new(),
            end_addr,
            stamp,
        }
    }

    /// Attach the placed section table (REPIN-END, `native::section_extents`). Enables
    /// the `section:<name>` boundary spelling and the bare-label pad warning.
    pub fn with_sections(mut self, sections: Vec<SectionExtent>) -> Listing {
        self.sections = sections;
        self
    }

    /// Attach the label→owning-section map (parcel R6, `native::section_label_owners`).
    /// Enables the flush-neighbour check in `judge_end`; without it that check is skipped
    /// and only the pad check runs (exactly the pre-R6 reach).
    pub fn with_label_owners(mut self, owners: HashMap<String, String>) -> Listing {
        self.label_owners = owners;
        self
    }

    /// The ROM section that DEFINES `name`, when the map is attached and the name is
    /// unambiguously owned.
    pub fn label_owner(&self, name: &str) -> Option<&str> {
        self.label_owners.get(name).map(String::as_str)
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
    /// region's base/end pins to (T4). A region's base is the section's LOAD address,
    /// so every consumer of a pin (a port gate windowing the reference ROM, `repin`'s
    /// own re-derivation) reads the address the bytes are actually at.
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

    /// The named section's extent. HARD ERROR naming the section when the listing
    /// carries no section table (the spelling is unmeasurable without one) or when
    /// the name matches no section / more than one.
    fn section(&self, name: &str) -> Result<&SectionExtent, String> {
        if self.sections.is_empty() {
            return Err(format!(
                "`{SECTION_PREFIX}{name}`: the listing carries no section table (attach one \
                 with `Listing::with_sections`), the spelling cannot be measured"
            ));
        }
        let hits: Vec<&SectionExtent> = self.sections.iter().filter(|s| s.name == name).collect();
        match hits.as_slice() {
            [one] => Ok(one),
            [] => Err(format!("section `{name}` not found in the resolved layout")),
            many => Err(format!("section `{name}` names {} sections, ambiguous", many.len())),
        }
    }

    /// A region's START boundary: `section:<name>` ⇒ the named section's LMA;
    /// otherwise the label's placement address ([`placement`]).
    pub fn region_start(&self, spec: &str) -> Result<u32, String> {
        match spec.strip_prefix(SECTION_PREFIX) {
            Some(name) => self.section(name).map(|s| s.lma),
            None => self.placement(spec),
        }
    }

    /// A region's END boundary: `section:<name>` ⇒ the named section's OWN one-past-end
    /// (`lma + image_len`) — never the successor's base, so alignment pad after the
    /// section's last byte is not measured into the region; otherwise the label's
    /// placement address ([`placement`]). A gap between labels is an allotment, not a
    /// size: a bare label that is the NEXT section's head measures the pad too, which
    /// [`pad_past_content`] reports.
    pub fn region_end(&self, spec: &str) -> Result<u32, String> {
        match spec.strip_prefix(SECTION_PREFIX) {
            Some(name) => self.section(name).map(|s| s.end),
            None => self.placement(spec),
        }
    }

    /// How many bytes at the tail of `[start, end)` belong to NO section's image — the
    /// placer pad a bare-label `end` sitting on the successor's head sweeps into a
    /// region. See [`PadVerdict`] for the four outcomes; the one that matters is that
    /// "no section overlaps the window" is [`PadVerdict::Unmeasurable`], NOT a pass
    /// (parcel R6, invariant "loud on unmeasurable"). A listing with no section table
    /// at all is [`PadVerdict::NoTable`] — silent by design, because such a listing
    /// still measures bare labels exactly as it always did.
    pub fn pad_past_content(&self, start: u32, end: u32) -> PadVerdict {
        if self.sections.is_empty() {
            return PadVerdict::NoTable;
        }
        let mut covered: Option<(&SectionExtent, u32)> = None;
        for s in &self.sections {
            if s.lma >= end || s.end <= start {
                continue;
            }
            let reach = s.end.min(end);
            if covered.map(|(_, c)| reach > c).unwrap_or(true) {
                covered = Some((s, reach));
            }
        }
        let Some((last, reach)) = covered else {
            return PadVerdict::Unmeasurable;
        };
        if reach < end {
            PadVerdict::Pad { section: last.name.clone(), pad: end - reach, content_end: reach }
        } else {
            PadVerdict::Exact { section: last.name.clone() }
        }
    }

    /// Number of parsed (numeric) symbols — provenance/debug aid.
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }
}

/// What [`Listing::pad_past_content`] found at the tail of a pinned window (parcel R6).
///
/// The split that earns its keep is [`Unmeasurable`](PadVerdict::Unmeasurable) vs
/// [`Exact`](PadVerdict::Exact): before R6 both returned `None` and read as "fine". A
/// window that overlaps NO section's image has an unknown width — the resolve cannot say
/// whether it holds the region's bytes, and rendering that as a pass is the one thing the
/// R6 gate is forbidden to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PadVerdict {
    /// The window ends exactly at `section`'s last byte — the region's content extent.
    Exact { section: String },
    /// `pad` bytes at the tail belong to no section's image; `section` is the region's
    /// last covered section and `content_end` its one-past-end.
    Pad { section: String, pad: u32, content_end: u32 },
    /// A section table is present and NO section overlaps the window — the width cannot
    /// be judged. Never a pass.
    Unmeasurable,
    /// The listing carries no section table; nothing to judge (a table-less listing
    /// measures bare labels exactly as before). Silent by design.
    NoTable,
}

/// The `section:<name>` boundary prefix (REPIN-END) — the same spelling the map's
/// `order` rows use (parcel SECTION-ROW). Unambiguous: no identifier in either
/// front-end admits `:`.
pub const SECTION_PREFIX: &str = "section:";

/// `end_measures = "content"` (parcel R6) — the DEFAULT and the strict contract: the
/// pinned window holds the region's own section bytes and nothing past them.
pub const END_MEASURES_CONTENT: &str = "content";

/// `end_measures = "allotment"` (parcel R6) — the OPT-IN contract: the end is wherever
/// the next placement begins, so the pin's width is the gap to a neighbour rather than
/// a size of this region's own. Declared, never inferred.
pub const END_MEASURES_ALLOTMENT: &str = "allotment";

/// A placed ROM section's extent: `name` is the `module … in <name>` section name,
/// `lma` its load address, `end` one-past its last image byte (`lma + image_len`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionExtent {
    pub name: String,
    pub lma: u32,
    pub end: u32,
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
    /// What the region's END VALUE is a statement ABOUT (parcel R6). Two values:
    ///
    /// - `"content"` (the DEFAULT, and the strict one): the window holds this region's
    ///   own section bytes and nothing past them. Any placer pad between the last
    ///   content byte and `end` is a HARD ERROR naming the region, the shape, the
    ///   successor section and the `section:<name>` remedy.
    /// - `"allotment"`: the end is deliberately "wherever the next placement begins".
    ///   The pin's WIDTH IS NOT A SIZE — it is the gap to the neighbour, and it moves
    ///   when the neighbour moves. Declared per region so the dependency is greppable
    ///   and countable instead of being a convention nobody can see.
    ///
    /// No BYTE COUNT is declared, deliberately. The pad is an accident of where the
    /// successor landed, not a property of this region; writing today's number down
    /// would enshrine the accident as a requirement (the mistake R7 measured its way
    /// out of). What is declared is the KIND of contract, which is stable.
    ///
    /// Only meaningful with `end` (a `len` region declares its own width) and never
    /// with a `section:` end (there is nothing to tolerate — the section's own end IS
    /// the content end).
    #[serde(default)]
    pub end_measures: Option<String>,
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
                "region `{}`: `debug_len` is a per-shape override for a literal-`len` region, set `len` too (or use an `end` symbol for per-shape lengths)",
                r.name
            ));
        }
        if r.debug_only && r.plain_anchor.is_none() {
            return Err(format!(
                "region `{}`: `debug_only` needs `plain_anchor` (the plain-listing symbol of the next placement, a debug-only region's start never appears in the plain listing)",
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
                "region `{}`: `plain_only` needs `debug_anchor` (the debug-listing symbol of the next placement, a plain-only region's start never appears in the debug listing)",
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
                "region `{}`: `debug_end` is a per-shape end SYMBOL override, set `end` too (it is the plain-shape end)",
                r.name
            ));
        }
        // R6: `end_measures` declares WHAT the end value is a statement about.
        match r.end_measures.as_deref() {
            None | Some(END_MEASURES_CONTENT) | Some(END_MEASURES_ALLOTMENT) => {}
            Some(other) => {
                return Err(format!(
                    "region `{}`: `end_measures` must be `{END_MEASURES_CONTENT}` (the default, \
                     the window holds this region's bytes and nothing past them) or \
                     `{END_MEASURES_ALLOTMENT}` (the end is the next placement; the width is not a \
                     size), got `{other}`",
                    r.name
                ))
            }
        }
        if r.end_measures.is_some() && r.end.is_none() {
            return Err(format!(
                "region `{}`: `end_measures` describes an `end` symbol, a `len` region already \
                 declares its own width",
                r.name
            ));
        }
        if r.end_measures.as_deref() == Some(END_MEASURES_ALLOTMENT) {
            for spec in [r.end.as_deref(), r.debug_end.as_deref()].into_iter().flatten() {
                if spec.starts_with(SECTION_PREFIX) {
                    return Err(format!(
                        "region `{}`: `end_measures = \"{END_MEASURES_ALLOTMENT}\"` contradicts \
                         `end = \"{spec}\"`, a `{SECTION_PREFIX}` end IS the content end, so there \
                         is no allotment to declare",
                        r.name
                    ));
                }
            }
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
    /// Loud-but-not-fatal findings (REPIN-END): a bare-label `end` that measures
    /// placer pad past the region's last section byte, one line per shape, naming
    /// the region, the label, the section and the pad. The `repin` bin prints them.
    pub warnings: Vec<String>,
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

/// Judge ONE region/shape window's tail against its declared `end_measures` contract
/// (parcel R6). `Ok(Some(advisory))` is a non-fatal note; `Err` refuses the resolve.
///
/// WHICH INSTRUMENT EACH HALF USES, stated because a gate that asks the same resolver
/// the pin file asked cannot notice that resolver being wrong (R7's lesson):
/// - the window `[lo, hi)` comes from the SYMBOL listing (`Listing::region_start` /
///   `region_end` over `native::sigil_native_symbol_listing`);
/// - the content extent comes from the SECTION TABLE of the resolved layout the ROM is
///   emitted from (`native::section_extents` → `resolve_canonical_sections`).
///
/// Those are two derivations, so a symbol address that disagrees with the section
/// geometry is visible here. What NEITHER half can catch is a wrong resolve — both come
/// from one link of one tree — and this function does not claim to.
fn judge_end(
    region: &str,
    shape: &str,
    spec: &str,
    measures: Option<&str>,
    listing: &Listing,
    lo: u32,
    hi: u32,
) -> Result<Option<String>, String> {
    let allotment = measures == Some(END_MEASURES_ALLOTMENT);
    // A `section:` end IS the content end by construction; there is nothing to judge,
    // and `load_manifest` already refused `allotment` alongside one.
    if spec.starts_with(SECTION_PREFIX) {
        return Ok(None);
    }
    match listing.pad_past_content(lo, hi) {
        PadVerdict::NoTable => Ok(None),
        PadVerdict::Unmeasurable => Err(format!(
            "region `{region}` ({shape}): end `{spec}` spans {:#X} byte(s) at {lo:#X}..{hi:#X} that \
             overlap NO placed section, the width cannot be measured, so it is not asserted. A \
             region whose bytes the section table cannot find is a manifest error, not a pass: \
             check `start`/`end` name this region's own labels.",
            hi - lo
        )),
        // FLUSH BUT STILL THE NEIGHBOUR'S: zero pad does not mean the end is this
        // region's own. When the end label is DEFINED in a different section, the value
        // is the successor's placement and moves with it — the same contract as a padded
        // end, wearing better clothes. Refuse it undeclared, exactly as the padded case.
        // Skipped when no ownership map is attached (the pre-R6 reach) or when the name
        // is ambiguously owned (`native::section_label_owners` drops those, so an
        // unknown name is never read as a match).
        PadVerdict::Exact { section }
            if !allotment
                && listing.label_owner(spec).is_some_and(|owner| owner != section) =>
        {
            let owner = listing.label_owner(spec).unwrap_or_default();
            Err(format!(
                "region `{region}` ({shape}): end `{spec}` is defined in section `{owner}`, not in \
                 `{section}` where this region's bytes end. The window is flush TODAY ({hi:#X}), so \
                 nothing is mis-measured yet, but the value is the neighbour's placement and moves \
                 with it. Spell `end = \"{SECTION_PREFIX}{section}\"` to pin this region's own extent \
                 (the pin does not move), or declare `end_measures = \"{END_MEASURES_ALLOTMENT}\"` if \
                 the gap to the neighbour really is what this pin means."
            ))
        }
        PadVerdict::Exact { section } if allotment => Ok(Some(format!(
            "region `{region}` ({shape}): `end_measures = \"{END_MEASURES_ALLOTMENT}\"` but the \
             window ends exactly at section `{section}`'s last byte, the allotment is zero-width \
             IN THIS SHAPE. If every shape says so, re-spell `end = \"{SECTION_PREFIX}{section}\"` \
             and drop the declaration; the pin does not move."
        ))),
        PadVerdict::Exact { .. } => Ok(None),
        PadVerdict::Pad { section, pad, content_end } if allotment => Ok(Some(format!(
            "region `{region}` ({shape}): declared allotment, end `{spec}` is the next placement, \
             {pad:#X} byte(s) past section `{section}`'s last byte ({hi:#X} vs {content_end:#X}). \
             The pin's WIDTH IS NOT A SIZE and moves when the neighbour moves."
        ))),
        PadVerdict::Pad { section, pad, content_end } => Err(format!(
            "region `{region}` ({shape}): end `{spec}` measures {pad:#X} byte(s) of placer pad past \
             section `{section}`'s last byte ({hi:#X} vs {content_end:#X}); the gap between labels \
             is an allotment, not a size. Either spell `end = \"{SECTION_PREFIX}{section}\"` to pin \
             the content (preferred, the pin then states this region's own extent), or declare \
             `end_measures = \"{END_MEASURES_ALLOTMENT}\"` to say in the manifest that this width is \
             the gap to a neighbour and not a size."
        )),
    }
}

/// Resolve the manifest against both listings. Every failure names the
/// symbol/region — never a silent zero (D-T10.2).
pub fn resolve(m: &Manifest, plain: &Listing, debug: &Listing) -> Result<Resolved, String> {
    let mut regions = Vec::new();
    let mut warnings = Vec::new();
    for r in &m.regions {
        if r.debug_only {
            // Debug-only region (whole-file `ifdef __DEBUG__` twin): the
            // debug listing carries start/end; the plain shape has ZERO bytes
            // at plain_anchor's address.
            let anchor = r.plain_anchor.as_ref().expect("load_manifest validated plain_anchor");
            let plain_base = plain
                .get(anchor)
                .map_err(|e| format!("region `{}` plain_anchor: {e}", r.name))?;
            let debug_base = debug
                .region_start(&r.start)
                .map_err(|e| format!("region `{}` start (debug): {e}", r.name))?;
            let (debug_len, end_desc) = match (&r.end, r.len) {
                (Some(end), None) => {
                    // R6: `section:` and the end-contract check reach this arm too. Before
                    // R6 they did not, and three pad-carrying `debug_only` pairs
                    // (compression_selftest, test_parent, test_stress_emitter) were the
                    // only ones in the corpus that no warning could name.
                    let de = debug
                        .region_end(end)
                        .map_err(|e| format!("region `{}` end (debug): {e}", r.name))?;
                    if de < debug_base {
                        return Err(format!(
                            "region `{}`: end `{end}` precedes start `{}` ({de:#X} < {debug_base:#X})",
                            r.name, r.start
                        ));
                    }
                    if let Some(note) = judge_end(
                        &r.name,
                        "debug",
                        end,
                        r.end_measures.as_deref(),
                        debug,
                        debug_base,
                        de,
                    )? {
                        warnings.push(note);
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
            let plain_base = plain
                .region_start(&r.start)
                .map_err(|e| format!("region `{}` start: {e}", r.name))?;
            let (plain_len, end_desc) = match (&r.end, r.len) {
                (Some(end), None) => {
                    // R6: same reach as the `debug_only` arm above — `section:` ends and
                    // the end-contract check apply here too.
                    let pe =
                        plain.region_end(end).map_err(|e| format!("region `{}` end: {e}", r.name))?;
                    if pe < plain_base {
                        return Err(format!(
                            "region `{}`: end `{end}` precedes start `{}` ({pe:#X} < {plain_base:#X})",
                            r.name, r.start
                        ));
                    }
                    if let Some(note) = judge_end(
                        &r.name,
                        "plain",
                        end,
                        r.end_measures.as_deref(),
                        plain,
                        plain_base,
                        pe,
                    )? {
                        warnings.push(note);
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
            plain.region_start(&r.start).map_err(|e| format!("region `{}` start: {e}", r.name))?;
        let debug_base = debug
            .region_start(&r.start)
            .map_err(|e| format!("region `{}` start (debug): {e}", r.name))?;
        let (plain_len, debug_len, end_desc) = match (&r.end, r.len) {
            (Some(end), None) => {
                // `debug_end` overrides the END SYMBOL for the debug shape (the
                // fault-handler shape split: ojz_scroll_test ends at ReleaseFault in
                // plain, BusError in debug — neither exists in the other listing).
                let debug_end = r.debug_end.as_deref().unwrap_or(end);
                let pe = plain.region_end(end).map_err(|e| format!("region `{}` end: {e}", r.name))?;
                let de = debug
                    .region_end(debug_end)
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
                // A bare label sitting past the last section byte measures placer pad
                // into the pin — name it (region, label, section, bytes) per shape.
                for (shape, listing, spec, lo, hi) in
                    [("plain", plain, end.as_str(), plain_base, pe), ("debug", debug, debug_end, debug_base, de)]
                {
                    if let Some(note) = judge_end(
                        &r.name,
                        shape,
                        spec,
                        r.end_measures.as_deref(),
                        listing,
                        lo,
                        hi,
                    )? {
                        warnings.push(note);
                    }
                }
                (pe - plain_base, de - debug_base, desc)
            }
            (None, Some(len)) => {
                let dl = r.debug_len.unwrap_or(len);
                (len, dl, format!("start + {len:#X} plain / {dl:#X} debug (literal, no end symbol)"))
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
        warnings,
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
    let _ = writeln!(w, "//! GENERATED FILE, DO NOT EDIT BY HAND.");
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
    let _ = writeln!(w, "/// A gated region's geometry. Slice as `base..base + len`, the lens are");
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
            .map(|g| format!(", gate `{g}`"))
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
                    "/// `{}`, debug-shape consumer only (`debug_only`).{}",
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

// ── Drift reporting: the ONE verdict, and a message that owns both comparisons ──

/// What differs between the committed `pins.rs` and a fresh render, kept in the
/// buckets a reader has to be able to tell apart.
///
/// TWO COMPARISONS, AND THEY ARE NOT THE SAME ONE. The staleness VERDICT is
/// whole-file: [`strip_provenance`] drops only the `[provenance]` stamp lines, so
/// every other line the generator emits, doc comments and struct definitions
/// included, is part of "is the committed file what `repin` now writes". The pin
/// COUNT is not whole-file: [`diff_pins`] reads `pub const` declarations and is
/// blind to everything else. A sweep that rewrites the generator's comment strings
/// therefore leaves every pin value standing and still makes the committed file
/// stale, and a report carrying only the count renders that true pair as
/// `STALE ... (0 changed pin(s))`, which reads as a self-contradiction and sends
/// the reader hunting a bug in the gate. Measured once already: a dash sweep moved
/// 38 string literals in this file and drifted 108 comment lines of `pins.rs` with
/// no pin value moving at all.
///
/// THE VERDICT IS THE STRICT ONE ON PURPOSE. Narrowing it to what [`diff_pins`]
/// models would make exactly that case invisible, which is why the fix here was to
/// widen the MESSAGE and never the comparison. The buckets below partition the
/// whole line difference, so the report can name what it found in every case
/// rather than printing a zero next to the word STALE.
#[derive(Debug)]
pub struct DriftReport {
    /// Pins whose initializer text changed, or that were added or removed.
    pub pin_changes: Vec<PinChange>,
    /// `pub const` lines that differ WITHOUT their pin appearing in `pin_changes`:
    /// the declaration's rendering moved while its value stood still.
    pub reformatted_declarations: usize,
    /// Non-declaration lines present only in the committed file.
    pub other_removed: Vec<String>,
    /// Non-declaration lines present only in the regenerated text.
    pub other_added: Vec<String>,
}

/// How many of the listed one-sided lines a message prints before summarising the
/// rest. A comment sweep can move hundreds; a panic that scrolls past the verdict
/// is a message nobody reads to the end of.
const DRIFT_LINES_SHOWN: usize = 8;

impl DriftReport {
    /// Non-declaration lines present on exactly one side, both directions.
    pub fn other_lines(&self) -> usize {
        self.other_removed.len() + self.other_added.len()
    }

    /// True when every bucket is empty. The texts still differ (that is what built
    /// this report), so the only remaining explanation is that the same lines appear
    /// in a different ORDER, and the message says so rather than printing zeroes.
    pub fn only_line_order_differs(&self) -> bool {
        self.pin_changes.is_empty() && self.reformatted_declarations == 0 && self.other_lines() == 0
    }

    /// The one-line answer to "what differs", covering every bucket.
    fn headline(&self) -> String {
        if self.only_line_order_differs() {
            return "the same lines in a different ORDER; no line was added or removed".into();
        }
        let mut parts = Vec::new();
        parts.push(match self.pin_changes.len() {
            0 => "NO pin value moved".to_string(),
            1 => "1 pin value moved".to_string(),
            n => format!("{n} pin values moved"),
        });
        if self.reformatted_declarations > 0 {
            parts.push(format!(
                "{} declaration line(s) differ without their value changing",
                self.reformatted_declarations
            ));
        }
        if self.other_lines() > 0 {
            parts.push(format!(
                "{} line(s) of surrounding text differ ({} committed-only, {} regenerated-only)",
                self.other_lines(),
                self.other_removed.len(),
                self.other_added.len()
            ));
        } else {
            parts.push("the surrounding text is identical".to_string());
        }
        parts.join("; ")
    }
}

/// What a report says when the pins all stood still. Spelled out because the pair
/// it describes is the one that reads as a contradiction, and because the tempting
/// repair is the wrong one.
const PINS_ALL_STOOD_STILL: &str = "\
Every pin value in the committed file is still the value the generator produces, and
the file is STILL stale: the verdict compares the whole rendered file minus the
`[provenance]` stamp lines, so the text the generator writes AROUND the pins counts.
That is not a false alarm and not a gate bug. The committed file is no longer what
`repin` emits, so regenerate it; do NOT narrow the comparison to pin values, which
would hide exactly this case.";

impl std::fmt::Display for DriftReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "WHAT DIFFERS: {}", self.headline())?;
        if self.pin_changes.is_empty() && !self.only_line_order_differs() {
            writeln!(f)?;
            writeln!(f, "{PINS_ALL_STOOD_STILL}")?;
        }
        if !self.pin_changes.is_empty() {
            writeln!(f)?;
            writeln!(f, "pin values that moved (name: committed -> regenerated):")?;
            for c in &self.pin_changes {
                let old = c.old.as_deref().unwrap_or("(new)");
                let new = c.new.as_deref().unwrap_or("(removed)");
                writeln!(
                    f,
                    "  {}: {old} -> {new}{}",
                    c.name,
                    delta_suffix(c.old.as_deref(), c.new.as_deref())
                )?;
            }
        }
        for (label, lines) in [
            ("surrounding text, committed side only", &self.other_removed),
            ("surrounding text, regenerated side only", &self.other_added),
        ] {
            if lines.is_empty() {
                continue;
            }
            writeln!(f)?;
            writeln!(f, "{label} ({} line(s)):", lines.len())?;
            for line in lines.iter().take(DRIFT_LINES_SHOWN) {
                writeln!(f, "  {line}")?;
            }
            if lines.len() > DRIFT_LINES_SHOWN {
                writeln!(f, "  ... and {} more", lines.len() - DRIFT_LINES_SHOWN)?;
            }
        }
        Ok(())
    }
}

/// THE staleness verdict, asked in exactly one place so the gate and the `repin`
/// binary cannot answer it differently: `None` when the committed text is current,
/// `Some(report)` when it is stale.
///
/// The comparison is [`strip_provenance`] equality over the WHOLE text and stays
/// that way; the report exists so the message can explain a verdict the pin count
/// alone cannot.
pub fn drift_report(committed: &str, generated: &str) -> Option<DriftReport> {
    let a = strip_provenance(committed);
    let b = strip_provenance(generated);
    if a == b {
        return None;
    }
    let pin_changes = diff_pins(committed, generated);
    let moved: HashSet<&str> = pin_changes.iter().map(|c| c.name.as_str()).collect();
    let (removed, added) = one_sided_lines(&a, &b);
    let mut reformatted = 0usize;
    let mut other_removed: Vec<String> = Vec::new();
    let mut other_added: Vec<String> = Vec::new();
    for (lines, out) in [(removed, &mut other_removed), (added, &mut other_added)] {
        for line in lines {
            match const_name_of(&line) {
                // A declaration line that differs while `diff_pins` reports its pin
                // unchanged: the VALUE stood still and only the rendering moved.
                Some(name) if !moved.contains(name.as_str()) => reformatted += 1,
                // Already named, with its old and new value, under `pin_changes`.
                Some(_) => {}
                None => out.push(line),
            }
        }
    }
    Some(DriftReport {
        pin_changes,
        reformatted_declarations: reformatted,
        other_removed,
        other_added,
    })
}

/// The constant a `pub const` line declares, or `None` for any other line. The same
/// prefix [`const_lines`] keys on, so the two agree about what a declaration is.
fn const_name_of(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("pub const ")?;
    let (name, _) = rest.split_once(':')?;
    Some(name.trim().to_string())
}

/// Lines present on exactly one side, as a MULTISET difference in each file's own
/// order: `(only in a, only in b)`.
///
/// Multiset rather than positional, because a single inserted line shifts every line
/// after it and a positional diff would report the whole tail as changed. File order
/// rather than sorted, because the first differing line is the one a reader looks at.
fn one_sided_lines(a: &str, b: &str) -> (Vec<String>, Vec<String>) {
    let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
    for l in a.lines() {
        *counts.entry(l).or_default() += 1;
    }
    for l in b.lines() {
        *counts.entry(l).or_default() -= 1;
    }
    let mut left = counts.clone();
    let mut removed = Vec::new();
    for l in a.lines() {
        if let Some(c) = left.get_mut(l) {
            if *c > 0 {
                *c -= 1;
                removed.push(l.to_string());
            }
        }
    }
    let mut right = counts;
    let mut added = Vec::new();
    for l in b.lines() {
        if let Some(c) = right.get_mut(l) {
            if *c < 0 {
                *c += 1;
                added.push(l.to_string());
            }
        }
    }
    (removed, added)
}

/// ` (Δ …)` for single-value numeric pins where a delta is meaningful; empty for
/// added/removed pins and for multi-field initializers whose field counts differ.
fn delta_suffix(old: Option<&str>, new: Option<&str>) -> String {
    let (Some(old), Some(new)) = (old, new) else { return String::new() };
    let nums = |s: &str| -> Vec<i64> {
        let mut out = Vec::new();
        for tok in s.split(|c: char| !c.is_ascii_alphanumeric()) {
            if let Some(hex) = tok.strip_prefix("0x").or_else(|| tok.strip_prefix("0X")) {
                if let Ok(v) = i64::from_str_radix(hex, 16) {
                    out.push(v);
                }
            }
        }
        out
    };
    let (o, n) = (nums(old), nums(new));
    if o.is_empty() || o.len() != n.len() {
        return String::new();
    }
    let deltas: Vec<String> = o
        .iter()
        .zip(&n)
        .filter(|(a, b)| a != b)
        .map(|(a, b)| {
            let d = b - a;
            if d >= 0 { format!("+{d:#X}") } else { format!("-{:#X}", -d) }
        })
        .collect();
    if deltas.is_empty() { String::new() } else { format!(" (Δ {})", deltas.join(", ")) }
}

/// The EXACT text `tests/repin_pins.rs::pins_rs_is_current` fails with. Assembled
/// here rather than at the panic site so the message itself is a testable artifact:
/// the gate needs a reference tree to reach its panic, and the wording is the subject
/// of `tests/repin_gate_message.rs`, which has none.
pub fn stale_pins_message(
    report: &DriftReport,
    build_dir: Option<&std::path::Path>,
    aeon: Option<&std::path::Path>,
) -> String {
    format!(
        "src/pins.rs is STALE against the live listings.\n\n{report}\n{}",
        regenerate_command(build_dir, aeon)
    )
}

/// The cargo build directory the RUNNING executable was compiled into, derived from
/// its own path rather than guessed: an explicit `CARGO_TARGET_DIR` first (a caller
/// who chose a directory keeps it), else `<target>/<profile>[/deps]/<exe>` walked
/// back. `None` when neither is readable, so a caller prints a placeholder instead
/// of a wrong path.
pub fn build_dir_of_this_run() -> Option<std::path::PathBuf> {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR") {
        if !dir.is_empty() {
            return Some(std::path::PathBuf::from(dir));
        }
    }
    let exe = std::env::current_exe().ok()?;
    let holder = exe.parent()?;
    // An integration-test binary sits one level deeper, in `deps/`.
    let profile = if holder.file_name()? == "deps" { holder.parent()? } else { holder };
    Some(profile.parent()?.to_path_buf())
}

/// The remediation a staleness report prints: THE COMMAND THAT ACTUALLY WORKS.
///
/// `cargo run -p sigil-harness --bin repin` on its own does not regenerate anything.
/// The resolve builds the sound-on shape, so `repin` refuses without `SIGIL_EMIT`:
/// measured, it exits 2 and prints `repin: set SIGIL_EMIT to ...`, writing nothing.
/// That is a clear failure with a pointer rather than a silent no-op, so the old
/// one-line hint cost a reader a round trip rather than misleading them into a bad
/// state; it is still the wrong command to print, and printing the right one costs
/// two lines.
///
/// `build_dir` and `aeon` are filled in when the caller knows them (the gate does:
/// it has just resolved both), and stand in as named placeholders when it does not.
pub fn regenerate_command(
    build_dir: Option<&std::path::Path>,
    aeon: Option<&std::path::Path>,
) -> String {
    let emit = match build_dir {
        Some(d) => d.join("release").join("emit_sound_blob").display().to_string(),
        None => "$CARGO_TARGET_DIR/release/emit_sound_blob".to_string(),
    };
    let aeon = match aeon {
        Some(a) => a.display().to_string(),
        None => "$AEON_DIR".to_string(),
    };
    format!(
        "TO FIX: regenerate the file. SIGIL_EMIT IS PART OF THE COMMAND, not an optional\n\
         extra: the resolve builds the sound-on shape, and `repin` with SIGIL_EMIT unset\n\
         exits 2 naming the variable and writes nothing. From the sigil checkout whose\n\
         pins.rs is stale:\n\
         \n  \
         cargo build --release -p sigil-harness --bin emit_sound_blob\n  \
         SIGIL_EMIT={emit} \\\n  \
         AEON_DIR={aeon} \\\n    \
         cargo run --release -p sigil-harness --bin repin"
    )
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
    /// stale. The soundness catch: a base holding the phase VMA ($8000) would name a
    /// ROM window at $8000 rather than the bank's true LMA ($58000), so every pin
    /// consumer would read the wrong bytes.
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

    /// REPIN-END: a region whose `end` label is the NEXT section's head measures the
    /// successor's alignment pad into the pin (the ACT_DESCRIPTOR 0x27C-vs-0x27A
    /// failure); `end = "section:<name>"` measures the owning section's OWN end
    /// (`lma + image_len`). Expectations are DERIVED from the synthetic section
    /// geometry, never copied from the live 0x27A.
    #[test]
    fn section_end_spelling_measures_the_owning_section_not_the_successor_base() {
        // A synthetic section table: `desc` carries IMAGE_LEN bytes at LMA; its
        // successor `blocks` is placed on the next even boundary, 2 bytes past
        // desc's last byte (the plain-shape act_descriptor geometry; the debug shape
        // pads 6 — any positive pad exercises the same arithmetic).
        const LMA: u32 = 0x15A34;
        const IMAGE_LEN: u32 = 0x28 + 9 * 0x42; // header + 9 Sec records — odd, so pad follows
        const PAD: u32 = 2;
        let desc_end = LMA + IMAGE_LEN;
        let blocks_lma = desc_end + PAD;
        let sections = vec![
            SectionExtent { name: "desc".into(), lma: LMA, end: desc_end },
            SectionExtent { name: "blocks".into(), lma: blocks_lma, end: blocks_lma + 0x100 },
        ];
        let syms = &[("Desc_Head", LMA), ("Blocks_Head", blocks_lma)];
        let plain = test_listing(syms, 0x60000).with_sections(sections.clone());
        let debug = test_listing(syms, 0x62000).with_sections(sections);

        let manifest = load_manifest(
            r#"
[rom]
end_symbol = "__END__"

[[region]]
name = "by_section"
start = "Desc_Head"
end = "section:desc"

[[region]]
name = "by_successor_label"
start = "Desc_Head"
end = "Blocks_Head"
end_measures = "allotment"

[[region]]
name = "start_by_section"
start = "section:blocks"
end = "section:blocks"
"#,
        )
        .unwrap();
        let resolved = resolve(&manifest, &plain, &debug).unwrap();

        // `section:desc` at `end` = the section's own end: exactly IMAGE_LEN, no pad.
        let by_section = &resolved.regions[0];
        assert_eq!((by_section.plain_base, by_section.debug_base), (LMA, LMA));
        assert_eq!(
            (by_section.plain_len, by_section.debug_len),
            (IMAGE_LEN, IMAGE_LEN),
            "`end = section:<name>` must measure lma + image_len of the owning section"
        );
        assert_eq!(by_section.end_desc, "`section:desc`");

        // The successor's head label still measures the allotment (labels keep their
        // meaning unchanged) — and the pad is DETECTED and NAMED, per shape. R6: the
        // region must DECLARE `end_measures = "allotment"` to be resolvable at all; the
        // note then says the width is not a size.
        let by_label = &resolved.regions[1];
        assert_eq!((by_label.plain_len, by_label.debug_len), (IMAGE_LEN + PAD, IMAGE_LEN + PAD));
        let pad_warnings: Vec<&String> =
            resolved.warnings.iter().filter(|w| w.contains("region `by_successor_label`")).collect();
        assert_eq!(pad_warnings.len(), 2, "one pad note per shape: {:?}", resolved.warnings);
        for w in &pad_warnings {
            assert!(
                w.contains("`Blocks_Head`")
                    && w.contains("section `desc`")
                    && w.contains(&format!("{PAD:#X} byte(s)")),
                "{w}"
            );
            assert!(w.contains("WIDTH IS NOT A SIZE"), "the note says what it is not: {w}");
        }
        // The `section:` region raised no warning (its end is content, not allotment).
        assert!(!resolved.warnings.iter().any(|w| w.contains("region `by_section`")), "{:?}", resolved.warnings);

        // `section:<name>` at `start` = the section's LMA; start == end ⇒ the whole
        // section, derived from the same table.
        let by_start = &resolved.regions[2];
        assert_eq!(by_start.plain_base, blocks_lma);
        assert_eq!(by_start.plain_len, 0x100);

        // LOUD on unmeasurable: an unknown section name, and a listing that carries no
        // section table at all, both fail NAMING the spelling — never a silent 0.
        let bad = load_manifest(
            r#"[rom]
end_symbol = "__END__"
[[region]]
name = "x"
start = "Desc_Head"
end = "section:nope"
"#,
        )
        .unwrap();
        let err = resolve(&bad, &plain, &debug).unwrap_err();
        assert!(err.contains("region `x` end") && err.contains("section `nope` not found"), "{err}");
        let no_table = test_listing(syms, 0x60000);
        let err = resolve(&manifest, &no_table, &no_table).unwrap_err();
        assert!(err.contains("`section:desc`") && err.contains("no section table"), "{err}");
        // …and a table-less listing still measures bare labels exactly as before,
        // silently (no table ⇒ nothing to judge, not a false warning).
        let labels_only = load_manifest(
            r#"[rom]
end_symbol = "__END__"
[[region]]
name = "y"
start = "Desc_Head"
end = "Blocks_Head"
"#,
        )
        .unwrap();
        let r = resolve(&labels_only, &no_table, &no_table).unwrap();
        assert_eq!(r.regions[0].plain_len, IMAGE_LEN + PAD);
        assert!(r.warnings.is_empty());
    }

    /// R6: what a region's `end` is a statement ABOUT is DECLARED, and the strict
    /// reading is the default. Four behaviours, every expectation derived from the
    /// synthetic geometry (`LMA`/`IMAGE_LEN`/`PAD`), never copied off a live pin:
    ///
    /// 1. an undeclared bare-label end that sweeps a neighbour's pad REFUSES the
    ///    resolve, naming region, shape, label, section, both addresses and both fixes;
    /// 2. `end_measures = "allotment"` accepts the same manifest and says the width is
    ///    not a size;
    /// 3. an `allotment` declaration whose pad has gone to zero is an advisory to
    ///    convert — not a failure, because a shrinking pad is good news;
    /// 4. a window overlapping NO placed section is UNMEASURABLE and refuses. Before
    ///    R6 that case returned `None` and read as a pass.
    #[test]
    fn end_measures_declares_the_contract_and_the_strict_reading_is_the_default() {
        const LMA: u32 = 0x8000;
        const IMAGE_LEN: u32 = 0x37;
        const PAD: u32 = 9;
        let a_end = LMA + IMAGE_LEN;
        let b_lma = a_end + PAD;
        let sections = vec![
            SectionExtent { name: "a".into(), lma: LMA, end: a_end },
            SectionExtent { name: "b".into(), lma: b_lma, end: b_lma + 0x40 },
        ];
        let syms = &[("A_Head", LMA), ("B_Head", b_lma), ("Far", b_lma + 0x1000)];
        let plain = test_listing(syms, 0x60000).with_sections(sections.clone());
        let debug = test_listing(syms, 0x62000).with_sections(sections);

        // 1. Undeclared pad-sweeping end ⇒ REFUSED, naming everything needed to fix it.
        let undeclared = load_manifest(
            "[rom]\nend_symbol = \"__END__\"\n[[region]]\nname = \"r\"\nstart = \"A_Head\"\nend = \"B_Head\"\n",
        )
        .unwrap();
        let err = resolve(&undeclared, &plain, &debug).unwrap_err();
        for needle in [
            "region `r` (plain)",
            "`B_Head`",
            "section `a`",
            &format!("{PAD:#X} byte(s)"),
            &format!("{b_lma:#X} vs {a_end:#X}"),
            "section:a",
            "allotment",
        ] {
            assert!(err.contains(needle), "refusal must name `{needle}`: {err}");
        }

        // 2. Declared ⇒ resolves, with a note that the width is not a size.
        let declared = load_manifest(
            "[rom]\nend_symbol = \"__END__\"\n[[region]]\nname = \"r\"\nstart = \"A_Head\"\nend = \"B_Head\"\nend_measures = \"allotment\"\n",
        )
        .unwrap();
        let r = resolve(&declared, &plain, &debug).unwrap();
        assert_eq!((r.regions[0].plain_len, r.regions[0].debug_len), (IMAGE_LEN + PAD, IMAGE_LEN + PAD));
        assert_eq!(r.warnings.len(), 2, "one note per shape: {:?}", r.warnings);
        assert!(r.warnings[0].contains("WIDTH IS NOT A SIZE"), "{:?}", r.warnings);

        // 3. A declaration whose pad has gone to zero: advisory to convert, not a
        //    failure. Modelled by moving `b` flush against `a`.
        let flush = vec![
            SectionExtent { name: "a".into(), lma: LMA, end: a_end },
            SectionExtent { name: "b".into(), lma: a_end, end: a_end + 0x40 },
        ];
        let flush_syms = &[("A_Head", LMA), ("B_Head", a_end)];
        let fp = test_listing(flush_syms, 0x60000).with_sections(flush.clone());
        let fd = test_listing(flush_syms, 0x62000).with_sections(flush);
        let r = resolve(&declared, &fp, &fd).unwrap();
        assert_eq!(r.regions[0].plain_len, IMAGE_LEN, "flush ⇒ the pin is the content");
        assert!(
            r.warnings.iter().all(|w| w.contains("zero-width") && w.contains(SECTION_PREFIX)),
            "a stale allotment advises the conversion: {:?}",
            r.warnings
        );

        // 4. LOUD ON UNMEASURABLE: a window overlapping no placed section refuses, and
        //    the `allotment` declaration does NOT buy it a pass — an undeclared width is
        //    not a tolerated width.
        let off_map = load_manifest(
            "[rom]\nend_symbol = \"__END__\"\n[[region]]\nname = \"ghost\"\nstart = \"Far\"\nend = \"Far\"\nend_measures = \"allotment\"\n",
        )
        .unwrap();
        let err = resolve(&off_map, &plain, &debug).unwrap_err();
        assert!(
            err.contains("region `ghost`") && err.contains("overlap NO placed section"),
            "{err}"
        );

        // 5. FLUSH IS NOT SAFE: the same manifest as (3) — pad zero in both shapes — is
        //    REFUSED once the listing knows `B_Head` is defined in section `b` while the
        //    region's bytes end in `a`. Zero pad hid a dependency the address cannot see.
        let owners: HashMap<String, String> =
            [("A_Head".to_string(), "a".to_string()), ("B_Head".to_string(), "b".to_string())]
                .into_iter()
                .collect();
        let flush2 = vec![
            SectionExtent { name: "a".into(), lma: LMA, end: a_end },
            SectionExtent { name: "b".into(), lma: a_end, end: a_end + 0x40 },
        ];
        let owned_p = test_listing(&[("A_Head", LMA), ("B_Head", a_end)], 0x60000)
            .with_sections(flush2.clone())
            .with_label_owners(owners.clone());
        let owned_d = test_listing(&[("A_Head", LMA), ("B_Head", a_end)], 0x62000)
            .with_sections(flush2)
            .with_label_owners(owners);
        let err = resolve(&undeclared, &owned_p, &owned_d).unwrap_err();
        assert!(
            err.contains("is defined in section `b`")
                && err.contains("not in `a`")
                && err.contains("flush TODAY")
                && err.contains("section:a"),
            "a flush successor-head end must be refused, naming the owner: {err}"
        );
        // …and the pad-free geometry alone (no ownership map attached) still passes —
        // the check is skipped, not silently satisfied, when the map is absent.
        let no_owner_p = test_listing(&[("A_Head", LMA), ("B_Head", a_end)], 0x60000)
            .with_sections(vec![
                SectionExtent { name: "a".into(), lma: LMA, end: a_end },
                SectionExtent { name: "b".into(), lma: a_end, end: a_end + 0x40 },
            ]);
        let no_owner_d = test_listing(&[("A_Head", LMA), ("B_Head", a_end)], 0x62000)
            .with_sections(vec![
                SectionExtent { name: "a".into(), lma: LMA, end: a_end },
                SectionExtent { name: "b".into(), lma: a_end, end: a_end + 0x40 },
            ]);
        assert!(resolve(&undeclared, &no_owner_p, &no_owner_d).is_ok());

        // Manifest validation: an unknown word, `end_measures` on a `len` region, and
        // `allotment` alongside a `section:` end are all refused by name.
        for (src, needle) in [
            ("[rom]\nend_symbol = \"__END__\"\n[[region]]\nname = \"r\"\nstart = \"A_Head\"\nend = \"B_Head\"\nend_measures = \"maybe\"\n", "got `maybe`"),
            ("[rom]\nend_symbol = \"__END__\"\n[[region]]\nname = \"r\"\nstart = \"A_Head\"\nlen = 4\nend_measures = \"allotment\"\n", "already declares its own width"),
            ("[rom]\nend_symbol = \"__END__\"\n[[region]]\nname = \"r\"\nstart = \"A_Head\"\nend = \"section:a\"\nend_measures = \"allotment\"\n", "no allotment to declare"),
        ] {
            let err = load_manifest(src).unwrap_err();
            assert!(err.contains(needle), "expected `{needle}`: {err}");
        }
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
