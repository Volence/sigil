//! The `.emp` front end's side table for the build export (G0a spec 4.5): what the
//! lowering knows about every symbol it defined and the placed sections do not, namely
//! which module defined it, whether it is a procedure and how far its body runs, and
//! whether it is `pub`. Recorded while [`crate::resolve`] lowers each module (after the
//! module's rename, so every name is spelled as its label is, which is the listing's
//! spelling), then resolved to final addresses against the placed sections.

use sigil_ir::Module;
use sigil_link::export::{PlacedSymbols, SymbolKind, SymbolRecord};
use std::collections::{HashMap, HashSet};

/// What the lowering knew about one symbol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmpSymbolKind {
    /// A label that is not a procedure's entry (a data item, a proc local, a RAM var).
    Label,
    /// A procedure entry, with its body as fragment indices into the label's section
    /// (see [`sigil_ir::ProcMark`]).
    Proc { start_fragment: usize, end_fragment: usize },
    /// A link-level `equ`.
    Equ,
}

/// One symbol the `.emp` lowering defined.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmpSymbol {
    /// The spelling its label (or equ) carries after the module's rename.
    pub name: String,
    /// The dotted id of the module that defined it.
    pub module: String,
    pub kind: EmpSymbolKind,
    /// `pub proc` / `pub equ`.
    pub public: bool,
}

/// Every symbol a `.emp` program build defined, in lowering order (first definition
/// of a name only).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramSymbols {
    pub symbols: Vec<EmpSymbol>,
    seen: HashSet<String>,
}

impl ProgramSymbols {
    /// Record what module `module_id` defined, from its lowered (and renamed) IR.
    /// `pub_equs` is the module's `pub equ` names.
    pub fn record_module(&mut self, module_id: &str, module: &Module, pub_equs: &HashSet<String>) {
        let procs: HashMap<&str, &sigil_ir::ProcMark> =
            module.procs.iter().map(|p| (p.name.as_str(), p)).collect();
        for sec in &module.sections {
            for l in &sec.labels {
                if !self.seen.insert(l.name.clone()) {
                    continue;
                }
                let (kind, public) = match procs.get(l.name.as_str()) {
                    Some(p) => (
                        EmpSymbolKind::Proc { start_fragment: p.start_fragment, end_fragment: p.end_fragment },
                        p.public,
                    ),
                    None => (EmpSymbolKind::Label, false),
                };
                self.symbols.push(EmpSymbol { name: l.name.clone(), module: module_id.to_string(), kind, public });
            }
            for e in &sec.equ_syms {
                if !self.seen.insert(e.name.clone()) {
                    continue;
                }
                self.symbols.push(EmpSymbol {
                    name: e.name.clone(),
                    module: module_id.to_string(),
                    kind: EmpSymbolKind::Equ,
                    public: pub_equs.contains(&e.name),
                });
            }
        }
    }

    /// Whether the program defined `name`.
    pub fn defines(&self, name: &str) -> bool {
        self.seen.contains(name)
    }
}

/// The `.emp` program's symbols at their final placement. A procedure is `kind: proc`
/// with `size` from its first byte to the last byte its body stored, in the same
/// section. A label no placed section defines is an error (the side table and the
/// sections disagree); an equate that did not fold is left out, as the listing leaves
/// it out.
pub fn emp_symbols(program: &ProgramSymbols, placed: &PlacedSymbols) -> Result<Vec<SymbolRecord>, String> {
    let mut out = Vec::with_capacity(program.symbols.len());
    for s in &program.symbols {
        let module = Some(s.module.clone());
        match &s.kind {
            EmpSymbolKind::Equ => out.extend(placed.equ_record(&s.name, module, s.public)),
            EmpSymbolKind::Label | EmpSymbolKind::Proc { .. } => {
                let p = placed.label(&s.name).ok_or_else(|| {
                    format!("`.emp` symbol `{}` (module {}) is in no placed section", s.name, s.module)
                })?;
                let kind = if matches!(s.kind, EmpSymbolKind::Proc { .. }) { SymbolKind::Proc } else { SymbolKind::Label };
                let mut r = placed.label_record(&s.name, kind, module, s.public).expect("looked up above");
                if let EmpSymbolKind::Proc { start_fragment, end_fragment } = s.kind {
                    r.size = Some(
                        placed
                            .extent(&p, start_fragment, end_fragment)
                            .map_err(|e| format!("proc `{}` (module {}): {e}", s.name, s.module))?,
                    );
                }
                out.push(r);
            }
        }
    }
    Ok(out)
}
