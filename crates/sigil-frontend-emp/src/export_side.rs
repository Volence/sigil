//! The `.emp` front end's side table for the build export (G0a spec 4.5): what the
//! lowering knows about every symbol it defined and the placed sections do not, namely
//! which module defined it, whether it is a procedure and how far its body runs, and
//! whether it is `pub`. Recorded while [`crate::resolve`] lowers each module (after the
//! module's rename, so every name is spelled as its label is, which is the listing's
//! spelling). Placing it (final addresses, extents in bytes) is the driver's step
//! (`sigil-harness` `export_symbols`): this crate does not depend on the linker, and
//! the crate graph (`sigil-cli/tests/crate_graph.rs`) keeps it that way.

use crate::ast;
use sigil_ir::Module;
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
    /// `pub_names` is every name the module declares `pub`, spelled as its label is
    /// after the rename ([`pub_item_names`]); a label or equ named there is public.
    pub fn record_module(&mut self, module_id: &str, module: &Module, pub_names: &HashSet<String>) {
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
                    None => (EmpSymbolKind::Label, pub_names.contains(&l.name)),
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
                    public: pub_names.contains(&e.name),
                });
            }
        }
    }

    /// Whether the program defined `name`.
    pub fn defines(&self, name: &str) -> bool {
        self.seen.contains(name)
    }
}

/// Every name `file` declares `pub` that can become a link symbol: the names the
/// resolver exports (`imports::exported_names`: `pub data`/`proc`/`dispatch`/`script`/
/// `offsets`/`equ`, the labels of a region-form `pub vars` block, and the comptime-only
/// names, which no label carries), plus the two `pub` item kinds that index does not
/// list, `pub table` (its base label) and `pub region`. Recurses into `section {}`.
pub fn pub_item_names(file: &ast::File) -> HashSet<String> {
    fn walk(items: &[ast::Item], out: &mut HashSet<String>) {
        for it in items {
            match it {
                ast::Item::Table(t) if t.public => {
                    out.insert(t.name.clone());
                }
                ast::Item::Region(r) if r.public => {
                    out.insert(r.name.clone());
                }
                ast::Item::Section(sec) => walk(&sec.items, out),
                _ => {}
            }
        }
    }
    let mut out: HashSet<String> = crate::resolve::imports::exported_names(file).into_iter().collect();
    walk(&file.items, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_ir::{AddressSpace, Cpu, EquSym, Expr, Label, Section, SectionPlacement};
    use sigil_span::{SourceId, Span};

    fn file(src: &str) -> ast::File {
        let (f, d) = crate::parse_str(src);
        assert!(d.iter().all(|d| d.level != sigil_span::Level::Error), "{d:?}");
        f
    }

    #[test]
    fn pub_item_names_lists_every_pub_label_kind_and_no_private_one() {
        let f = file(
            "module m\n\
             pub data PubData: [u8; 1] = [1]\n\
             data PrivData: [u8; 1] = [2]\n\
             pub proc PubProc() { rts }\n\
             proc PrivProc() { rts }\n\
             pub equ PUB_EQU = 3\n\
             equ PRIV_EQU = 4\n",
        );
        let names = pub_item_names(&f);
        for n in ["PubData", "PubProc", "PUB_EQU"] {
            assert!(names.contains(n), "{n} missing from {names:?}");
        }
        for n in ["PrivData", "PrivProc", "PRIV_EQU"] {
            assert!(!names.contains(n), "{n} is private: {names:?}");
        }
    }

    #[test]
    fn record_module_marks_pub_labels_and_equs_public() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let module = Module {
            sections: vec![Section {
                name: "text".into(),
                cpu: Cpu::M68000,
                vma_base: None,
                lma: 0,
                space: AddressSpace::Image,
                labels: vec![Label { name: "PubData".into(), offset: 0 }, Label { name: "PrivData".into(), offset: 1 }],
                fragments: vec![],
                placement: SectionPlacement::Pinned,
                reserved_span: 0,
                group: None,
                bank: None,
                equ_syms: vec![EquSym { name: "PUB_EQU".into(), expr: Expr::Int(3), span }],
            }],
            link_asserts: vec![],
            comptime_guards: 0,
            procs: vec![],
        };
        let pubs: HashSet<String> = ["PubData", "PUB_EQU"].iter().map(|s| s.to_string()).collect();
        let mut t = ProgramSymbols::default();
        t.record_module("m", &module, &pubs);
        let public = |n: &str| t.symbols.iter().find(|s| s.name == n).unwrap().public;
        assert!(public("PubData"));
        assert!(!public("PrivData"));
        assert!(public("PUB_EQU"));
    }
}
