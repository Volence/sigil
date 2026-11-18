//! The build export's symbol table (G0a spec 4.5), from the front ends' side tables
//! placed against the resolved sections.
//!
//! The side tables live in the front ends (`sigil_frontend_emp::export_side`,
//! `sigil_frontend_as::export_side`) and say what each front end knows: `.emp` knows
//! modules, procedures (as fragment ranges) and `pub`; AS knows label or equate. They
//! are placed HERE, in the driver, because placing needs the linker's view
//! ([`PlacedSymbols`]) and the crate graph keeps the front ends off `sigil-link`
//! (`sigil-cli/tests/crate_graph.rs`).

use sigil_frontend_as::export_side::AsSymbols;
use sigil_frontend_emp::export_side::{EmpSymbolKind, ProgramSymbols};
use sigil_ir::{Section, SymbolTable};
use sigil_link::export::{PlacedSymbols, SymbolKind, SymbolRecord};
use std::collections::HashSet;

/// The `.emp` program's symbols at their final placement. A procedure is `kind: proc`
/// with `size` from its first byte to the last byte its body stored, in the same
/// section. A label no placed section defines is an error (the side table and the
/// sections disagree); an equate that did not fold is left out, as the listing leaves
/// it out.
pub fn emp_symbols(program: &ProgramSymbols, placed: &PlacedSymbols) -> Result<Vec<SymbolRecord>, String> {
    let mut out = Vec::with_capacity(program.symbols.len());
    for s in &program.symbols {
        let module = Some(s.module.clone());
        if s.kind == EmpSymbolKind::Equ {
            out.extend(placed.equ_record(&s.name, module, s.public));
            continue;
        }
        let p = placed
            .label(&s.name)
            .ok_or_else(|| format!("`.emp` symbol `{}` (module {}) is in no placed section", s.name, s.module))?;
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
    Ok(out)
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

/// The sidecar's symbols: `.emp` first (it knows the most), then the AS residual, then
/// the build-supplied defines, which no front end defines (they come from the shape,
/// `game_defines::define_listing_rows`). A name two populations both claim is kept
/// once, from the first; its address comes from the one placed index either way, so
/// the choice never moves an address.
pub fn export_symbols(
    emp: &ProgramSymbols,
    as_side: &AsSymbols,
    defines: &[sigil_link::ListingSymbol],
    resolved: &[Section],
    stubs: &SymbolTable,
) -> Result<Vec<SymbolRecord>, String> {
    let placed = PlacedSymbols::new(resolved, stubs);
    let mut out = emp_symbols(emp, &placed).map_err(|e| format!("build export: {e}"))?;
    let mut seen: HashSet<String> = out.iter().map(|s| s.name.clone()).collect();
    let as_recs = as_symbols(as_side, &placed).map_err(|e| format!("build export: {e}"))?;
    out.extend(as_recs.into_iter().filter(|s| seen.insert(s.name.clone())));
    for d in defines {
        if seen.insert(d.name.clone()) {
            out.push(SymbolRecord {
                name: d.name.clone(),
                demangled: None,
                space: None,
                vma: d.value,
                lma: None,
                kind: SymbolKind::Equ,
                size: None,
                module: None,
                public: false,
            });
        }
    }
    Ok(out)
}
