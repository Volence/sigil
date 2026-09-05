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

/// One declared island anchor. `when` gates a shape-conditional anchor
/// (`"sound_on"` / `"sound_off"`); `None` ⇒ every shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub name: String,
    pub at: u32,
    pub vma: Option<u32>,
    pub when: Option<String>,
}

/// One declared mid-image hole (K1 = data; K2 enforces). The module named by
/// `filled_by` overlays `[?, at)`; the post-hole data resumes at `at`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hole {
    pub after: String,
    pub at: u32,
    pub filled_by: String,
    pub when: Option<String>,
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
        self.anchors.iter().filter(move |a| when_applies(a.when.as_deref(), sound_on))
    }
    /// The holes whose `when` matches this shape.
    pub fn holes_for(&self, sound_on: bool) -> impl Iterator<Item = &Hole> {
        self.holes.iter().filter(move |h| when_applies(h.when.as_deref(), sound_on))
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

fn when_applies(when: Option<&str>, sound_on: bool) -> bool {
    match when {
        None => true,
        Some("sound_on") => sound_on,
        Some("sound_off") => !sound_on,
        Some(_) => true, // unknown gate: informational, does not exclude
    }
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
    Ok(PlacementMap {
        anchors: doc
            .anchor
            .into_iter()
            .map(|a| Anchor { name: a.name, at: a.at, vma: a.vma, when: a.when })
            .collect(),
        holes: doc
            .hole
            .into_iter()
            .map(|h| Hole { after: h.after, at: h.at, filled_by: h.filled_by, when: h.when })
            .collect(),
        budgets: doc
            .budget
            .into_iter()
            .map(|b| Budget { region: b.region, ceiling: b.ceiling, cursor: b.cursor })
            .collect(),
        order: doc.order,
    })
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
