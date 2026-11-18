//! The AS front end's side table for the build export (G0a spec 4.5). AS knows less
//! than `.emp`: a name it defined is a label (bound to a location) or an equate (a
//! value), and nothing about modules, procedures or visibility. Taken from the
//! assembled module's sections BEFORE they join the `.emp` sections, so the table is
//! exactly what this front end defined; resolved to final addresses afterwards.

use sigil_ir::Section;
use sigil_link::export::{PlacedSymbols, SymbolKind, SymbolRecord};
use std::collections::HashSet;

/// Every name the AS assembly defined, in definition order, first definition only.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AsSymbols {
    pub labels: Vec<String>,
    pub equs: Vec<String>,
}

impl AsSymbols {
    /// The names `sections` (one AS assembly's output) define.
    pub fn from_sections(sections: &[Section]) -> AsSymbols {
        let mut seen = HashSet::new();
        let mut t = AsSymbols::default();
        for sec in sections {
            for l in &sec.labels {
                if seen.insert(l.name.clone()) {
                    t.labels.push(l.name.clone());
                }
            }
        }
        for sec in sections {
            for e in &sec.equ_syms {
                if seen.insert(e.name.clone()) {
                    t.equs.push(e.name.clone());
                }
            }
        }
        t
    }
}

/// The AS assembly's symbols at their final placement: `kind: label` (or `ram` in a
/// reserve-only section) and `kind: equ`. A label no placed section defines is an
/// error; an equate that did not fold is left out, as the listing leaves it out.
pub fn as_symbols(table: &AsSymbols, placed: &PlacedSymbols) -> Result<Vec<SymbolRecord>, String> {
    let mut out = Vec::with_capacity(table.labels.len() + table.equs.len());
    for name in &table.labels {
        out.push(
            placed
                .label_record(name, SymbolKind::Label, None, false)
                .ok_or_else(|| format!("AS label `{name}` is in no placed section"))?,
        );
    }
    for name in &table.equs {
        out.extend(placed.equ_record(name, None, false));
    }
    Ok(out)
}
