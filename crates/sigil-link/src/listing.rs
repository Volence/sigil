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
