//! The Parcel-K1 placement map reader (`games/<g>/map.toml`).
//!
//! The declared, reviewed ROM placement contract — island ANCHORS, the boot_data
//! HOLE (data; K2 enforces), the object-bank BUDGET, and the byte-emitting section
//! ORDER (VALIDATED against the chainer's derived order — R2). The REGION geometry in
//! the same file is parsed by `sigil_link::load_map` (the unchanged region reader the
//! ~68 port-test fixtures share); this reader adds ONLY the placement facts, so a
//! region-only map (a fixture) parses here to an all-empty `PlacementMap` and the
//! fixtures need no migration.
//!
//! Consumption (K1): `native::validate_placement` checks the chainer's resolved layout
//! against these facts — the `[map.undeclared-island]` lint (an ANCHOR_GAP-inferred
//! island absent from `anchors` fails loud), the order validation (the derived
//! byte-emitting order must be a subsequence of `order`), and the budget. The map DRIVES
//! order once the synthetic-named AS residual is gone (K5, post-K4).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// The shape gate a `when` key names. The vocabulary is closed: a value outside it is
/// refused at load (`[map.when-unknown]`), because a gate the reader does not know
/// cannot be told to exclude anything, and applying the row to every shape instead
/// is how a mistyped gate would move a canonical anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    /// `"sound_on"`: only the shapes that build the sound driver.
    SoundOn,
    /// `"sound_off"`: only the shapes that do not.
    SoundOff,
}

impl When {
    /// Parse a `when` value, naming the row it came from on refusal.
    fn parse(value: &str, row: &str) -> Result<When, String> {
        match value {
            "sound_on" => Ok(When::SoundOn),
            "sound_off" => Ok(When::SoundOff),
            other => Err(format!(
                "[map.when-unknown] {row} has `when = \"{other}\"`, but `when` takes only \
                 `sound_on` or `sound_off`; an unknown gate is refused rather than applied to \
                 every shape"
            )),
        }
    }

    /// The spelling `map.toml` writes this gate as.
    pub fn as_str(self) -> &'static str {
        match self {
            When::SoundOn => "sound_on",
            When::SoundOff => "sound_off",
        }
    }

    /// Whether a row with this gate belongs to a shape with sound `sound_on`.
    pub fn applies(self, sound_on: bool) -> bool {
        match self {
            When::SoundOn => sound_on,
            When::SoundOff => !sound_on,
        }
    }
}

/// One declared island anchor. `when` gates a shape-conditional anchor; `None` means
/// every shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub name: String,
    pub at: u32,
    pub vma: Option<u32>,
    pub when: Option<When>,
}

/// One declared mid-image hole (K1 = data; K2 enforces). The module named by
/// `filled_by` overlays `[?, at)`; the post-hole data resumes at `at`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hole {
    pub after: String,
    pub at: u32,
    pub filled_by: String,
    pub when: Option<When>,
}

/// One declared per-region byte ceiling (checked at pack time). `cursor` names the
/// head-label of the first section PAST the budget region's content (the object bank's
/// data-region head, e.g. `DeformTable_Zero`); its resolved LMA is the used-cursor the
/// ceiling is checked against — the map-owned successor to engine.inc's `__BUDGET_DATA`
/// marker (K4 inc-6B). `None` ⇒ no cursor declared (the check is a no-op for that region).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    pub region: String,
    pub ceiling: u32,
    pub cursor: Option<String>,
}

/// The placement facts of a `games/<g>/map.toml` (regions live in the `MemoryMap`
/// that `sigil_link::load_map` returns from the same source).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlacementMap {
    pub anchors: Vec<Anchor>,
    pub holes: Vec<Hole>,
    pub budgets: Vec<Budget>,
    /// Byte-emitting section head-labels / module-ids, canonical union order.
    pub order: Vec<String>,
}

impl PlacementMap {
    /// The anchors whose `when` matches this shape (`sound_on`).
    pub fn anchors_for(&self, sound_on: bool) -> impl Iterator<Item = &Anchor> {
        self.anchors.iter().filter(move |a| when_applies(a.when, sound_on))
    }
    /// The holes whose `when` matches this shape.
    pub fn holes_for(&self, sound_on: bool) -> impl Iterator<Item = &Hole> {
        self.holes.iter().filter(move |h| when_applies(h.when, sound_on))
    }
}

/// The namespace prefix of an `order` row that names a SECTION rather than a head label.
pub const SECTION_ROW_PREFIX: &str = "section:";

/// The section name an `order` row names, when the row is spelled
/// `"section:<name>"` — the `<name>` of a `module … in <name>` declaration. A plain
/// row (a head label) yields `None`. The prefix cannot collide with a label: neither
/// frontend's identifier grammar admits `:` (`.emp`: `[A-Za-z_][A-Za-z0-9_]*`; AS:
/// alphanumerics, `_`, `.`, `'`), so any row containing `section:` is a section row.
pub fn section_row(row: &str) -> Option<&str> {
    row.strip_prefix(SECTION_ROW_PREFIX)
}

/// The `order` key a section is declared under: its head label when the row is a
/// label, else its `section:<name>` spelling.
pub fn section_row_key(name: &str) -> String {
    format!("{SECTION_ROW_PREFIX}{name}")
}

fn when_applies(when: Option<When>, sound_on: bool) -> bool {
    when.is_none_or(|w| w.applies(sound_on))
}

#[derive(Deserialize)]
struct MapDoc {
    #[serde(default)]
    anchor: Vec<AnchorDoc>,
    #[serde(default)]
    hole: Vec<HoleDoc>,
    #[serde(default)]
    budget: Vec<BudgetDoc>,
    #[serde(default)]
    order: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnchorDoc {
    #[serde(default)]
    name: String,
    at: u32,
    #[serde(default)]
    vma: Option<u32>,
    #[serde(default)]
    when: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HoleDoc {
    after: String,
    at: u32,
    filled_by: String,
    #[serde(default)]
    when: Option<String>,
}
#[derive(Deserialize)]
struct BudgetDoc {
    region: String,
    ceiling: u32,
    #[serde(default)]
    cursor: Option<String>,
}

/// Parse a `games/<g>/map.toml` string's PLACEMENT facts (anchors/holes/budgets/order).
/// Region-only sources (the port-test fixtures) parse to an empty `PlacementMap`. Unknown
/// keys in the region tables are ignored (they are `MemoryMap`'s concern).
pub fn load_placement_map(toml_src: &str) -> Result<PlacementMap, String> {
    let doc: MapDoc =
        toml::from_str(toml_src).map_err(|e| format!("placement map parse error: {e}"))?;
    // A `section:` row must name something; the bare prefix is a typo, not a row.
    if let Some(row) = doc.order.iter().find(|r| section_row(r).is_some_and(|n| n.is_empty())) {
        return Err(format!(
            "[map.order-section-row-empty] `order` row `{row}` names no section, spell it `section:<name>`"
        ));
    }
    let when = |w: Option<String>, row: String| w.map(|v| When::parse(&v, &row)).transpose();
    let anchors = doc
        .anchor
        .into_iter()
        .map(|a| {
            let when = when(a.when, format!("anchor `{}`", a.name))?;
            Ok(Anchor { name: a.name, at: a.at, vma: a.vma, when })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let holes = doc
        .hole
        .into_iter()
        .map(|h| {
            let when = when(h.when, format!("hole after `{}`", h.after))?;
            Ok(Hole { after: h.after, at: h.at, filled_by: h.filled_by, when })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(PlacementMap {
        anchors,
        holes,
        budgets: doc
            .budget
            .into_iter()
            .map(|b| Budget { region: b.region, ceiling: b.ceiling, cursor: b.cursor })
            .collect(),
        order: doc.order,
    })
}

/// The cartridge bank grid an overlay anchor must sit on: one `$8000` SetBank window.
pub const OVERLAY_GRID: u32 = 0x8000;

/// An anchor overlay (`sigil build --anchor-overlay <path>`): `[[anchor]]` rows, each
/// written as `map.toml` writes that anchor, each replacing the `at` of the map anchor
/// of the same name for one invocation. The contract is
/// `docs/superpowers/notes/2026-09-25-clip-overlay-contract.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorOverlay {
    /// Where the rows came from, for every diagnostic that names the overlay.
    pub origin: String,
    /// The rows, one per anchor name, in file order.
    pub rows: Vec<Anchor>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayDoc {
    #[serde(default)]
    anchor: Vec<OverlayAnchorDoc>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayAnchorDoc {
    name: String,
    at: u32,
    #[serde(default)]
    vma: Option<u32>,
    #[serde(default)]
    when: Option<String>,
}

/// Read and parse the anchor overlay at `path`. The read goes through the build's read
/// set, so the file is a row of the listing's source digest.
pub fn load_anchor_overlay(path: &Path) -> Result<AnchorOverlay, String> {
    let origin = path.display().to_string();
    let src = sigil_span::read_set::read_to_string(path)
        .map_err(|e| format!("[map.overlay-read] anchor overlay {origin}: {e}"))?;
    parse_anchor_overlay(&src, &origin)
}

/// Parse an anchor overlay's source. Refuses what the file alone can show is wrong:
/// a parse error or a key outside the grammar, no rows, two rows of one name, an
/// unknown `when`, an `at` off the bank grid. What needs the map is refused by
/// [`PlacementMap::with_overlay`].
pub fn parse_anchor_overlay(src: &str, origin: &str) -> Result<AnchorOverlay, String> {
    let doc: OverlayDoc = toml::from_str(src).map_err(|e| {
        format!(
            "[map.overlay-parse] anchor overlay {origin}: {e}\n(an overlay holds only \
             [[anchor]] rows, each with name and at, and optionally vma and when)"
        )
    })?;
    if doc.anchor.is_empty() {
        return Err(format!(
            "[map.overlay-empty] anchor overlay {origin} has no [[anchor]] row, so it moves \
             nothing; drop the switch or add the rows"
        ));
    }
    let mut rows: Vec<Anchor> = Vec::with_capacity(doc.anchor.len());
    for a in doc.anchor {
        let row = format!("anchor overlay {origin} row `{}`", a.name);
        let when = a.when.map(|v| When::parse(&v, &row)).transpose()?;
        if rows.iter().any(|r| r.name == a.name) {
            return Err(format!(
                "[map.overlay-duplicate] anchor overlay {origin} has two rows for `{}`; one \
                 anchor takes one position",
                a.name
            ));
        }
        if a.at % OVERLAY_GRID != 0 {
            return Err(format!(
                "[map.overlay-off-grid] {row} puts it at {:#x}, which is not on the {:#x} bank \
                 grid",
                a.at, OVERLAY_GRID
            ));
        }
        rows.push(Anchor { name: a.name, at: a.at, vma: a.vma, when });
    }
    Ok(AnchorOverlay { origin: origin.to_string(), rows })
}

impl PlacementMap {
    /// This map with `overlay` applied: each overlay row replaces the `at` of the one
    /// anchor of its name. Every row applies against THIS map in one pass, so an overlay
    /// address equal to another anchor's original address never moves anything twice.
    /// The anchor list keeps its length and order, so `self.anchors[i]` and the result's
    /// `anchors[i]` are the same anchor before and after.
    pub fn with_overlay(&self, overlay: &AnchorOverlay) -> Result<PlacementMap, String> {
        let origin = &overlay.origin;
        let mut out = self.clone();
        for row in &overlay.rows {
            let idx: Vec<usize> = self
                .anchors
                .iter()
                .enumerate()
                .filter(|(_, a)| a.name == row.name)
                .map(|(i, _)| i)
                .collect();
            let base = match idx.as_slice() {
                [] => {
                    let known: Vec<&str> = self.anchors.iter().map(|a| a.name.as_str()).collect();
                    return Err(format!(
                        "[map.overlay-unknown-anchor] anchor overlay {origin} row `{}` names no \
                         map.toml anchor (the map declares: {})",
                        row.name,
                        known.join(", ")
                    ));
                }
                [i] => *i,
                _ => {
                    return Err(format!(
                        "[map.overlay-ambiguous-base] anchor overlay {origin} row `{}`: map.toml \
                         declares {} anchors of that name, so which one it replaces cannot be said",
                        row.name,
                        idx.len()
                    ))
                }
            };
            let b = &self.anchors[base];
            if b.vma != row.vma || b.when != row.when {
                let show = |a: &Anchor| {
                    format!(
                        "vma = {}, when = {}",
                        a.vma.map_or("(none)".to_string(), |v| format!("{v:#x}")),
                        a.when.map_or("(none)", When::as_str)
                    )
                };
                return Err(format!(
                    "[map.overlay-row-differs] anchor overlay {origin} row `{}` has {}, but the \
                     map.toml row it replaces has {}. An overlay moves an anchor's `at` only; \
                     copy the other keys from map.toml",
                    row.name,
                    show(row),
                    show(b)
                ));
            }
            out.anchors[base].at = row.at;
        }
        for sound_on in [false, true] {
            let mut seen: HashMap<u32, &str> = HashMap::new();
            for a in out.anchors_for(sound_on) {
                if let Some(other) = seen.insert(a.at, a.name.as_str()) {
                    return Err(format!(
                        "[map.overlay-anchor-collision] with anchor overlay {origin}, anchors \
                         `{other}` and `{}` both sit at {:#x} in the sound-{} shapes",
                        a.name,
                        a.at,
                        if sound_on { "on" } else { "off" }
                    ));
                }
            }
        }
        Ok(out)
    }
}

/// The frozen provisional table with each island moved to its overlay address.
///
/// `base` is the map as written and `overlaid` is `base.with_overlay(..)`. For every
/// anchor that applies to this shape and whose address the overlay changed, the one
/// frozen row at the base address (the island that anchor holds) takes the overlay
/// address. Every row is looked up in the ORIGINAL table, so the substitution is one
/// pass: when an overlay address equals another anchor's base address, the row found
/// there is that other anchor's own island and moves with it, never twice.
///
/// Refused (`[map.overlay-island-ambiguous]`) when a moved anchor's base address holds
/// no frozen row or more than one, or when a row the overlay did not move already sits
/// at a moved anchor's new address: in each case which section is the island cannot be
/// said.
pub fn move_island_rows(
    frozen: &HashMap<String, u32>,
    base: &PlacementMap,
    overlaid: &PlacementMap,
    sound_on: bool,
    origin: &str,
) -> Result<HashMap<String, u32>, String> {
    let mut moves: Vec<(&str, u32, u32)> = Vec::new();
    for (b, o) in base.anchors.iter().zip(&overlaid.anchors) {
        if b.name != o.name {
            return Err(format!(
                "internal: the overlaid map's anchor list is not the base list in order (`{}` \
                 against `{}`)",
                b.name, o.name
            ));
        }
        if b.at != o.at && when_applies(b.when, sound_on) {
            moves.push((b.name.as_str(), b.at, o.at));
        }
    }
    let mut out = frozen.clone();
    let mut moved: Vec<&str> = Vec::new();
    for &(name, from, to) in &moves {
        let mut at_base: Vec<&str> =
            frozen.iter().filter(|(_, &v)| v == from).map(|(k, _)| k.as_str()).collect();
        at_base.sort_unstable();
        let [label] = at_base.as_slice() else {
            return Err(format!(
                "[map.overlay-island-ambiguous] anchor overlay {origin} moves `{name}` from \
                 {from:#x} to {to:#x}, but the frozen table holds {} section(s) at {from:#x} \
                 ({}), so the island it holds cannot be said",
                at_base.len(),
                at_base.join(", ")
            ));
        };
        out.insert((*label).to_string(), to);
        moved.push(label);
    }
    for &(name, _, to) in &moves {
        let mut squatters: Vec<&str> = out
            .iter()
            .filter(|(k, &v)| v == to && !moved.contains(&k.as_str()))
            .map(|(k, _)| k.as_str())
            .collect();
        if !squatters.is_empty() {
            squatters.sort_unstable();
            return Err(format!(
                "[map.overlay-island-ambiguous] anchor overlay {origin} moves `{name}` to \
                 {to:#x}, where the frozen table already holds {}, so the island at that address \
                 cannot be said",
                squatters.join(", ")
            ));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_only_map_parses_to_empty_placement() {
        // A port-test fixture (regions only) must yield an empty PlacementMap.
        let src = "fill = 0x00\n[[region]]\nname=\"rom\"\nlma_base=0\nsize=0x400000\nkind=\"rom\"\n";
        let p = load_placement_map(src).unwrap();
        assert_eq!(p, PlacementMap::default());
    }

    #[test]
    fn parses_anchors_holes_budget_order_with_when() {
        // Root scalar/array keys (`order`) MUST precede the `[[table]]` arrays.
        let src = r#"
order = ["Vectors", "BootData"]
[[anchor]]
name = "object_bank"
at = 0x10000
[[anchor]]
name = "sound_bank"
at = 0x58000
vma = 0x8000
when = "sound_on"
[[hole]]
after = "Z80_IdleProgram"
at = 0x3FE
filled_by = "engine.z80_init"
when = "sound_off"
[[budget]]
region = "object_bank"
ceiling = 0x20000
"#;
        let p = load_placement_map(src).unwrap();
        assert_eq!(p.anchors.len(), 2);
        assert_eq!(p.order, vec!["Vectors", "BootData"]);
        assert_eq!(p.budgets[0].ceiling, 0x20000);
        // shape filtering
        assert_eq!(p.anchors_for(false).count(), 1); // sound_bank gated out
        assert_eq!(p.anchors_for(true).count(), 2);
        assert_eq!(p.holes_for(false).count(), 1);
        assert_eq!(p.holes_for(true).count(), 0);
    }

    #[test]
    fn unknown_when_value_is_refused_not_applied_everywhere() {
        let e = load_placement_map("[[anchor]]\nname = \"x\"\nat = 0x8000\nwhen = \"s2clip\"\n")
            .unwrap_err();
        assert!(e.contains("[map.when-unknown]") && e.contains("\"s2clip\""), "{e}");
        let e = load_placement_map(
            "[[hole]]\nafter = \"A\"\nat = 0x400\nfilled_by = \"m\"\nwhen = \"sound\"\n",
        )
        .unwrap_err();
        assert!(e.contains("[map.when-unknown]") && e.contains("\"sound\""), "{e}");
    }

    #[test]
    fn unknown_anchor_or_hole_key_is_refused_not_dropped() {
        let e = load_placement_map("[[anchor]]\nname = \"x\"\nat = 0x8000\nvariant = \"s2clip\"\n")
            .unwrap_err();
        assert!(e.contains("unknown field") && e.contains("variant"), "{e}");
        let e = load_placement_map(
            "[[hole]]\nafter = \"A\"\nat = 0x400\nfilled_by = \"m\"\nsize = 4\n",
        )
        .unwrap_err();
        assert!(e.contains("unknown field") && e.contains("size"), "{e}");
    }

    // ── anchor overlay ──
    // A fixture shaped like a sound-on game map: two ungated anchors and the two bank
    // anchors, the sound bank's `at` one bank pair above the DAC banks' so that a clip
    // overlay moving the DAC banks up by that pair lands exactly on the base sound bank.
    const BASE_DAC: u32 = 0xA8000;
    const BASE_SND: u32 = 0xB8000;
    const PAIR: u32 = BASE_SND - BASE_DAC;

    fn base_map() -> PlacementMap {
        load_placement_map(&format!(
            "[[anchor]]\nname = \"boot_head\"\nat = 0x0\n\
             [[anchor]]\nname = \"object_bank\"\nat = 0x10000\n\
             [[anchor]]\nname = \"dac_banks\"\nat = {BASE_DAC:#x}\nwhen = \"sound_on\"\n\
             [[anchor]]\nname = \"sound_bank\"\nat = {BASE_SND:#x}\nvma = 0x8000\nwhen = \"sound_on\"\n"
        ))
        .unwrap()
    }

    fn overlay_src(dac: u32, snd: u32) -> String {
        format!(
            "[[anchor]]\nname = \"dac_banks\"\nat = {dac:#x}\nwhen = \"sound_on\"\n\
             [[anchor]]\nname = \"sound_bank\"\nat = {snd:#x}\nvma = 0x8000\nwhen = \"sound_on\"\n"
        )
    }

    fn frozen() -> HashMap<String, u32> {
        HashMap::from([
            ("Vectors".to_string(), 0x0),
            ("ObjCodeBase".to_string(), 0x10000),
            ("Dac_Temp_Blip".to_string(), BASE_DAC),
            ("SoundTablesZ80_Head".to_string(), BASE_SND),
            ("Song_MovingTrucks".to_string(), BASE_SND + 0x630),
        ])
    }

    fn overlay_err(src: &str) -> String {
        match parse_anchor_overlay(src, "fx/anchors.toml").and_then(|o| base_map().with_overlay(&o)) {
            Ok(m) => panic!("overlay accepted: {m:?}"),
            Err(e) => e,
        }
    }

    /// Trap 1: the overlay's DAC address IS the base sound-bank address. One pass puts
    /// the DAC island there and the sound head one pair above; a sequential rewrite
    /// would carry the DAC island on to the sound head's new address as well.
    #[test]
    fn overlay_onto_the_base_sound_bank_moves_each_island_once() {
        let (dac, snd) = (BASE_SND, BASE_SND + PAIR);
        let base = base_map();
        let ov = parse_anchor_overlay(&overlay_src(dac, snd), "fx").unwrap();
        let eff = base.with_overlay(&ov).unwrap();
        let at = |m: &PlacementMap, n: &str| m.anchors.iter().find(|a| a.name == n).unwrap().at;
        assert_eq!((at(&eff, "dac_banks"), at(&eff, "sound_bank")), (dac, snd));
        assert_eq!(base, base_map(), "the base map is not modified");
        let f = frozen();
        let moved = move_island_rows(&f, &base, &eff, true, "fx").unwrap();
        assert_eq!(moved["Dac_Temp_Blip"], dac);
        assert_eq!(moved["SoundTablesZ80_Head"], snd);
        for k in ["Vectors", "ObjCodeBase", "Song_MovingTrucks"] {
            assert_eq!(moved[k], f[k], "{k} is not an island the overlay moves");
        }
        // A sound-off shape carries neither gated anchor, so nothing moves.
        assert_eq!(move_island_rows(&f, &base, &eff, false, "fx").unwrap(), f);
    }

    #[test]
    fn overlay_row_keeps_the_map_anchor_order_and_other_keys() {
        let ov = parse_anchor_overlay(&overlay_src(0xC0000, 0xD0000), "fx").unwrap();
        let base = base_map();
        let eff = base.with_overlay(&ov).unwrap();
        assert_eq!(eff.anchors.len(), base.anchors.len());
        for (b, e) in base.anchors.iter().zip(&eff.anchors) {
            assert_eq!((&b.name, b.vma, b.when), (&e.name, e.vma, e.when));
        }
    }

    #[test]
    fn overlay_refusals_each_fire_with_their_id() {
        let e = overlay_err("[[anchor]]\nname = \"dac_banks\"\nat = 0xB8000\nvariant = \"x\"\n");
        assert!(e.contains("[map.overlay-parse]") && e.contains("variant"), "{e}");
        let e = overlay_err("order = [\"A\"]\n");
        assert!(e.contains("[map.overlay-parse]") && e.contains("order"), "{e}");
        let e = overlay_err("[[anchor]]\nat = 0xB8000\n");
        assert!(e.contains("[map.overlay-parse]") && e.contains("name"), "{e}");
        let e = overlay_err("not toml at all [");
        assert!(e.contains("[map.overlay-parse]"), "{e}");
        let e = overlay_err("# nothing\n");
        assert!(e.contains("[map.overlay-empty]"), "{e}");
        let e = overlay_err(&format!("{}{}", overlay_src(0xC0000, 0xD0000), overlay_src(0xC0000, 0xD0000)));
        assert!(e.contains("[map.overlay-duplicate]") && e.contains("`dac_banks`"), "{e}");
        let e = overlay_err("[[anchor]]\nname = \"dac_bank\"\nat = 0xB8000\nwhen = \"sound_on\"\n");
        assert!(e.contains("[map.overlay-unknown-anchor]") && e.contains("`dac_bank`"), "{e}");
        let e = overlay_err("[[anchor]]\nname = \"dac_banks\"\nat = 0xB8000\nwhen = \"clip\"\n");
        assert!(e.contains("[map.when-unknown]") && e.contains("\"clip\""), "{e}");
        let e = overlay_err("[[anchor]]\nname = \"dac_banks\"\nat = 0xB8000\n");
        assert!(e.contains("[map.overlay-row-differs]") && e.contains("when = sound_on"), "{e}");
        let e = overlay_err("[[anchor]]\nname = \"sound_bank\"\nat = 0xC8000\nwhen = \"sound_on\"\n");
        assert!(e.contains("[map.overlay-row-differs]") && e.contains("vma = 0x8000"), "{e}");
        let e = overlay_err("[[anchor]]\nname = \"dac_banks\"\nat = 0xB8100\nwhen = \"sound_on\"\n");
        assert!(e.contains("[map.overlay-off-grid]") && e.contains("0xb8100"), "{e}");
        // Moving only the DAC banks onto the unmoved sound bank puts two anchors at one address.
        let e = overlay_err(&format!(
            "[[anchor]]\nname = \"dac_banks\"\nat = {BASE_SND:#x}\nwhen = \"sound_on\"\n"
        ));
        assert!(e.contains("[map.overlay-anchor-collision]"), "{e}");
        // A base map with the name twice.
        let mut twice = base_map();
        twice.anchors.push(twice.anchors[2].clone());
        let ov = parse_anchor_overlay(&overlay_src(0xC0000, 0xD0000), "fx").unwrap();
        let e = twice.with_overlay(&ov).unwrap_err();
        assert!(e.contains("[map.overlay-ambiguous-base]"), "{e}");
        let e = load_anchor_overlay(Path::new("/nonexistent/clip/anchors.toml")).unwrap_err();
        assert!(e.contains("[map.overlay-read]") && e.contains("/nonexistent/clip/anchors.toml"), "{e}");
    }

    #[test]
    fn island_rows_refuse_an_unidentifiable_island() {
        let base = base_map();
        let eff = base
            .with_overlay(&parse_anchor_overlay(&overlay_src(0xC0000, 0xD0000), "fx").unwrap())
            .unwrap();
        // No frozen row at the DAC anchor's base address.
        let mut f = frozen();
        f.remove("Dac_Temp_Blip");
        let e = move_island_rows(&f, &base, &eff, true, "fx").unwrap_err();
        assert!(e.contains("[map.overlay-island-ambiguous]") && e.contains("0 section(s)"), "{e}");
        // Two rows at it.
        let mut f = frozen();
        f.insert("Dac_Twin".into(), BASE_DAC);
        let e = move_island_rows(&f, &base, &eff, true, "fx").unwrap_err();
        assert!(e.contains("[map.overlay-island-ambiguous]") && e.contains("Dac_Twin"), "{e}");
        // An unmoved row already at the new address.
        let mut f = frozen();
        f.insert("Squatter".into(), 0xC0000);
        let e = move_island_rows(&f, &base, &eff, true, "fx").unwrap_err();
        assert!(e.contains("[map.overlay-island-ambiguous]") && e.contains("Squatter"), "{e}");
    }

    #[test]
    fn section_row_is_the_prefixed_spelling_only() {
        assert_eq!(section_row("section:ojz_effects_editor_act1"), Some("ojz_effects_editor_act1"));
        assert_eq!(section_row("EditorSceneBinding_OJZ_Act1_Sec0"), None);
        assert_eq!(section_row_key("x"), "section:x");
        // A section row round-trips through the loader as a plain string (no struct change).
        let p = load_placement_map("order = [\"A\", \"section:b\"]\n").unwrap();
        assert_eq!(p.order, vec!["A", "section:b"]);
    }

    #[test]
    fn bare_section_prefix_is_rejected_at_load() {
        let e = load_placement_map("order = [\"A\", \"section:\"]\n").unwrap_err();
        assert!(e.contains("map.order-section-row-empty") && e.contains("`section:`"), "{e}");
    }

    /// The prefix is unambiguous because no label can contain `:` — witnessed on the
    /// `.emp` lexer (the AS lexer's identifier set is `[A-Za-z0-9_.']`, likewise
    /// colon-free): `section:foo` lexes as ident, colon, ident, never one identifier.
    #[test]
    fn no_label_can_spell_the_section_prefix() {
        use sigil_frontend_emp::lexer::{lex, Tok};
        let (toks, errs) = lex("section:foo", sigil_span::SourceId(0));
        assert!(errs.is_empty(), "{errs:?}");
        let idents: Vec<&str> = toks
            .iter()
            .filter_map(|t| match &t.tok {
                Tok::Ident(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(idents, ["section", "foo"]);
        assert!(toks.iter().any(|t| matches!(t.tok, Tok::Colon)));
    }
}
