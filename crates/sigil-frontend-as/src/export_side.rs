//! The AS front end's side table for the build export (G0a spec 4.5). AS knows less
//! than `.emp`: a name it defined is a label (bound to a location) or an equate (a
//! value), and nothing about modules, procedures or visibility. Taken from the
//! assembled module's sections BEFORE they join the `.emp` sections, so the table is
//! exactly what this front end defined. Placing it is the driver's step (`sigil-harness`
//! `export_symbols`): the crate graph (`sigil-cli/tests/crate_graph.rs`) forbids this
//! front end a dependency on the linker.

use sigil_ir::Section;
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
