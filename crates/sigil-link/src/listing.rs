//! `s4.lst` symbol-listing emitter. Target: the AS `-L` symbol-table section
//! that `tools/s4budget.py::parse_symbol_table` and the Oracle symbol loader
//! consume. Scope = symbol name, 24-bit hex value, C(code)/-(equate) marker,
//! `|` separator, the `Symbol Table (* = unused):` header, `N symbols` footer.

/// One symbol row. `is_equate` picks the `-` (equate) vs `C` (code) marker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListingSymbol {
    pub name: String,
    pub value: u32,
    pub is_equate: bool,
    pub unused: bool,
}

/// Is `part` a synthetic compiler block scope (`asm0`, `asm1`, …)? Those are
/// block-internal names with no source meaning — pure backtrace noise.
fn is_asm_block_scope(part: &str) -> bool {
    part.strip_prefix("asm").is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Rewrite the sigil-canonical mangled `.emp` symbol names into debugger-friendly
/// `Parent.local` names AND drop pure compiler-plumbing synthetics — the appendix
/// filter policy (Stage-3 P2b, OQ-B):
///
///   1. `$<module.path>$<Parent>$<local>` (a `.emp` proc-local, e.g.
///      `$engine.boot$EntryPoint$wait_dma`) → `EntryPoint.wait_dma`. KEEP.
///   2. `__offsets$<module.path>$<Parent>$<local>` (a comptime offset-table entry
///      a debugger user names, e.g. `__offsets$…$Ani_Sonic$Walk`) → `Ani_Sonic.Walk`.
///      KEEP.
///   3. `__align$…` internals and any name carrying an `asm<N>` block scope
///      (e.g. `$engine.boot$asm1$wait_z80`) → DROPPED.
///
/// Plain (unmangled) names pass through untouched. The mangled form uses `$` which
/// `convsym`'s `as_lst` name parser rejects (so mangled locals never reach the deb2
/// table); demangling to a `$`-free `Parent.local` lets the source-meaningful ones
/// survive it, while the plumbing stays dropped by removal here.
pub fn demangle_symbols(symbols: &[ListingSymbol]) -> Vec<ListingSymbol> {
    let mut out = Vec::with_capacity(symbols.len());
    for s in symbols {
        // Plain names (no mangling separator) are already source names.
        if !s.name.contains('$') {
            out.push(s.clone());
            continue;
        }
        let parts: Vec<&str> = s.name.split('$').filter(|p| !p.is_empty()).collect();
        // `__align$module$N` and any name with an `asm<N>` synthetic scope are
        // plumbing — dropped (not emitted → convsym never sees them).
        if parts.first() == Some(&"__align") || parts.iter().any(|p| is_asm_block_scope(p)) {
            continue;
        }
        // `$module$Parent$local` and `__offsets$module$Parent$local` both demangle
        // to their trailing `Parent.local` (the two most-specific components).
        if parts.len() >= 2 {
            let parent = parts[parts.len() - 2];
            let local = parts[parts.len() - 1];
            out.push(ListingSymbol { name: format!("{parent}.{local}"), ..s.clone() });
        }
        // A degenerate single-component mangled name (should not occur — top-level
        // procs emit unmangled) is dropped rather than emit a bare `$`-form.
    }
    out
}

/// Emit the AS-`-L`-compatible symbol-table section. Symbols are address-sorted;
/// each row is `[*]NAME : HEX C|-` `|`. One symbol per line keeps it trivially
/// parseable (both consumers iterate matches, so layout is cosmetic).
pub fn emit_listing(symbols: &[ListingSymbol]) -> String {
    let mut rows: Vec<&ListingSymbol> = symbols.iter().collect();
    rows.sort_by(|a, b| a.value.cmp(&b.value).then(a.name.cmp(&b.name)));
    let unused = rows.iter().filter(|s| s.unused).count();

    let mut out = String::new();

    // Oracle's `LoadFromAsListing` reads the per-line BODY listing (via
    // `ParseLineHeader`: `(depth) num/hexaddr :  ... Name:`), NOT the symbol-table
    // section that s4budget reads. Emit one Oracle-parseable body line per symbol
    // first — verified against the real Oracle Symbols.cpp AND s4budget: the body
    // lines precede s4budget's `Symbol Table` header (so it ignores them) and the
    // symbol-table rows below fail Oracle's `ParseLineHeader` (so Oracle ignores
    // them). Each consumer reads exactly its own half of one file.
    for (i, s) in rows.iter().enumerate() {
        out.push_str(&format!("(0) {}/{:X} :        {}:\n", i + 1, s.value, s.name));
    }

    out.push_str("  Symbol Table (* = unused):\n");
    out.push_str("  --------------------------\n\n");
    for s in &rows {
        let star = if s.unused { "*" } else { " " };
        let marker = if s.is_equate { "-" } else { "C" };
        out.push_str(&format!("{star}{} : {:X} {marker} |\n", s.name, s.value));
    }
    out.push_str(&format!("\n   {} symbols\n", rows.len()));
    out.push_str(&format!("    {unused} unused symbols\n"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(name: &str, value: u32, eq: bool, unused: bool) -> ListingSymbol {
        ListingSymbol { name: name.into(), value, is_equate: eq, unused }
    }

    #[test]
    fn emits_s4budget_parseable_rows() {
        // Mirror s4budget's regex: (\*?)([\w.]+)\s*:\s*(hex|"str")\s+([C\-])\s*\|
        let out = emit_listing(&[
            sym("Main", 0x000000, false, false),
            sym("OBJ_len", 0x40, true, false),
            sym("Unused", 0x2000, false, true),
        ]);
        assert!(out.contains("Symbol Table"));
        assert!(out.contains("unused"));
        // address-sorted; code marker C, equate marker -.
        assert!(out.contains("Main : 0 C |"));
        assert!(out.contains("OBJ_len : 40 - |"));
        assert!(out.contains("*Unused : 2000 C |"));
        assert!(out.contains("3 symbols"));
        assert!(out.contains("1 unused symbols"));
    }

    #[test]
    fn regex_intersection_matches_each_row() {
        // A pure-Rust stand-in for s4budget's regex to prove the grammar holds.
        let out = emit_listing(&[sym("Air_LandState", 0x10AF2, false, false)]);
        let re_ok = out.lines().any(|l| {
            let l = l.trim_start();
            // [*]name : HEX (C|-) |
            l.contains(" : ") && l.trim_end().ends_with('|')
                && (l.contains(" C |") || l.contains(" - |"))
        });
        assert!(re_ok, "no parseable row in:\n{out}");
    }

    #[test]
    fn demangler_keeps_proc_local_and_offsets_drops_plumbing() {
        let out = demangle_symbols(&[
            // (1) a .emp proc-local → Parent.local, KEPT.
            sym("$engine.boot$EntryPoint$wait_dma", 0x210, false, false),
            // (2) a source-meaningful comptime offset entry → Parent.local, KEPT.
            sym("__offsets$games.sonic4.sonic_anims$Ani_Sonic$Walk", 0x256F2, false, false),
            // (3a) an asm<N> block scope → DROPPED.
            sym("$engine.boot$asm1$wait_z80", 0x260, false, false),
            // (3b) an __align internal → DROPPED.
            sym("__align$games.sonic4.sonic_anims$0", 0x2574A, false, false),
            // plain → untouched.
            sym("EntryPoint", 0x200, false, false),
        ]);
        let names: Vec<&str> = out.iter().map(|s| s.name.as_str()).collect();
        // KEPT + demangled, value preserved.
        assert!(names.contains(&"EntryPoint.wait_dma"), "proc-local demangle: {names:?}");
        assert!(names.contains(&"Ani_Sonic.Walk"), "__offsets demangle: {names:?}");
        assert_eq!(out.iter().find(|s| s.name == "EntryPoint.wait_dma").unwrap().value, 0x210);
        // plain pass-through.
        assert!(names.contains(&"EntryPoint"));
        // DROPPED — the t24 must-NOT-survive control.
        assert!(!names.iter().any(|n| n.contains("asm1")), "asm block scope leaked: {names:?}");
        assert!(!names.iter().any(|n| n.contains("__align") || *n == "sonic_anims.0"), "align internal leaked: {names:?}");
        // No `$` survives into the demangled set (convsym would drop those).
        assert!(!names.iter().any(|n| n.contains('$')), "a mangled `$` name survived: {names:?}");
    }

    #[test]
    fn demangler_is_asm_block_scope_precise() {
        assert!(is_asm_block_scope("asm0"));
        assert!(is_asm_block_scope("asm12"));
        // NOT block scopes — real names beginning with `asm` keep their locals.
        assert!(!is_asm_block_scope("asm"));
        assert!(!is_asm_block_scope("asmName"));
        assert!(!is_asm_block_scope("assemble"));
    }

    #[test]
    fn emits_oracle_body_lines_before_symbol_table() {
        let out = emit_listing(&[
            sym("Main", 0x1000, false, false),
            sym("OBJ_len", 0x40, true, false),
        ]);
        // Oracle body lines (ParseLineHeader format) come first, address-sorted.
        // `(depth) N/HEXADDR :        Name:`
        assert!(out.contains("(0) 1/40 :        OBJ_len:"), "missing/incorrect body line:\n{out}");
        assert!(out.contains("(0) 2/1000 :        Main:"), "missing/incorrect body line:\n{out}");
        // Every body line must precede the Symbol Table header (s4budget reads only
        // after that header; Oracle reads only the body lines).
        let body_idx = out.find("(0) 1/40").unwrap();
        let tab_idx = out.find("Symbol Table").unwrap();
        assert!(body_idx < tab_idx, "body lines must precede the symbol-table section");
        // The symbol-table section is still present and unchanged.
        assert!(out.contains("Main : 1000 C |"));
        assert!(out.contains("OBJ_len : 40 - |"));
    }
}
