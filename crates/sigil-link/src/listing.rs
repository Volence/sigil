//! `s4.lst` symbol-listing emitter. Target: the AS `-L` symbol-table section
//! that `tools/s4budget.py::parse_symbol_table` and the Oracle symbol loader
//! consume. Scope = symbol name, 24-bit hex value, C(code)/-(equate) marker,
//! `|` separator, the `Symbol Table (* = unused):` header, `N symbols` footer.

/// One symbol row. `is_equate` picks the `-` (equate) vs `C` (code) marker.
///
/// `value` is always the symbol's VMA, the address the code RUNS at. `lma`
/// records the address its bytes are STORED at, and ONLY when the two differ:
/// `None` means unphased (VMA == LMA, or a value symbol that has no storage at
/// all), `Some(l)` means the symbol is PHASED and `l` is where its bytes live.
/// The listing's address rows never carried that distinction, so every consumer
/// that needed it had to re-derive it from somewhere else; see [`emit_listing`]'s
/// Phase Table for what the file says about it now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListingSymbol {
    pub name: String,
    pub value: u32,
    pub is_equate: bool,
    pub unused: bool,
    pub lma: Option<u32>,
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

/// Emit the AS-`-L`-compatible symbol-table section. Address symbols are
/// address-sorted; each row is `[*]NAME : HEX C` `|`. One symbol per line keeps it
/// trivially parseable (both consumers iterate matches, so layout is cosmetic).
///
/// # The three sections, and why equates get their own
///
/// The ADDRESS symbols are rendered TWICE, as two views of one table:
///
///  1. the Oracle body listing (`(depth) N/HEXADDR : Name:`), which Oracle's
///     `LoadFromAsListing`/`ParseLineHeader` reads and from which
///     `aeon/tools/scene_spans.py::lst_proc_sizes` derives proc sizes;
///  2. the `Symbol Table (* = unused):` section + its `N symbols` trailer, which
///     `aeon/tools/s4budget.py::parse_listing` reads.
///
/// s4budget CROSS-CHECKS those two views — same length, same `(name, value)`
/// sequence, both equal to the trailer's own count — precisely so a partial parse
/// cannot masquerade as a small program. That invariant is load-bearing, and it is
/// what decides where an EQUATE goes: an equate is a VALUE, not an address, so it
/// belongs in neither view. Putting it only in the symbol table would break the 1:1
/// check; putting it in both would make Oracle resolve a constant as a code address
/// and inject a phantom head into every proc-size window.
///
/// So equates get a THIRD section, appended after the trailer, with a row shape of
/// its own: `EQU <name> = $<8 hex digits>`. That shape matches none of the four
/// consumer grammars in play — s4budget's ` NAME : HEX C|- |` symbol row (no `:`,
/// no `|`), its `(N) i/HEX :` source row and `<N> symbols` trailers, scene_spans'
/// identical `LST_HEAD_RE` address head, and effects_gates' `(0) `-prefixed probe.
/// A tool that wants the value of a published `pub equ` matches
/// `^EQU (\S+) = \$([0-9A-F]{8})$` and cannot collide with an address row.
///
/// The section (and its `N equates` trailer) is OMITTED entirely when there are no
/// equates, so a listing with none is byte-identical to the pre-equate format.
///
/// Values render as the `u32` [`ListingSymbol`] carries: a negative equate appears
/// as its two's-complement pattern, exactly as an address-width AS listing would
/// render it.
///
/// # The Phase Table, and why it is UNCONDITIONAL
///
/// Every address row above prints a VMA. For most symbols that is also the LMA and
/// nothing is lost; for a symbol in a PHASED section (`section … (vma: $8000)`) the
/// printed address is a bank-local runtime address and the bytes are stored
/// somewhere else entirely. The listing said nothing about which was which, so a
/// consumer that needed the distinction had to re-derive it, and re-derivation from
/// the listing alone is impossible: the only recoverable signal is the magnitude of
/// the number, and a phased VMA is an ordinary-looking small address. A fourth
/// section states it instead:
///
/// ```text
///   Phase Table (every address above is a VMA):
///   -------------------------------------------
///
/// PHASE COUNT 6
/// PHASE SoundTablesZ80_Head VMA $00008000 LMA $000B8000
/// ```
///
/// It is emitted ALWAYS, even at count 0, and that is the whole point. The
/// ambiguity being closed is one bit per LISTING, not one per symbol: with the
/// section unconditional, no section at all means an older sigil that does not know
/// about phasing, `PHASE COUNT 0` means this sigil looked and found nothing phased,
/// and rows are the phased set with the storage address each one hides. An
/// omitted-when-empty section would leave those first two cases spelled the same
/// way, which is the one reading a consumer cannot recover from.
///
/// The cost is that an unphased listing is no longer byte-identical to the
/// pre-phase format. That trade is deliberate: the byte identity this project
/// protects is the ROM's, not the listing's.
///
/// The row shape matches none of the four consumer grammars, on the same reasoning
/// as the equate row: no `(depth) N/HEX :` head, no ` NAME : HEX C |` symbol row,
/// and the count line says `PHASE COUNT n`, never `<n> symbols`, so neither
/// s4budget trailer regex sees it.
pub fn emit_listing(symbols: &[ListingSymbol]) -> String {
    let (equates, addrs): (Vec<&ListingSymbol>, Vec<&ListingSymbol>) =
        symbols.iter().partition(|s| s.is_equate);
    let mut rows = addrs;
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
        out.push_str(&format!("{star}{} : {:X} C |\n", s.name, s.value));
    }
    out.push_str(&format!("\n   {} symbols\n", rows.len()));
    out.push_str(&format!("    {unused} unused symbols\n"));

    if !equates.is_empty() {
        let mut eqs = equates;
        // Name-sorted: an equate has no address to order by, and a stable order
        // makes the section diffable across builds.
        eqs.sort_by(|a, b| a.name.cmp(&b.name).then(a.value.cmp(&b.value)));
        out.push_str("\n  Equate Table (name = value; values, not addresses):\n");
        out.push_str("  ---------------------------------------------------\n\n");
        for s in &eqs {
            out.push_str(&format!("EQU {} = ${:08X}\n", s.name, s.value));
        }
        out.push_str(&format!("\n   {} equates\n", eqs.len()));
    }

    // The Phase Table. Address-sorted, matching the two address views above, so a
    // phased row is found at the same place in the ordering as its address row.
    // Emitted unconditionally: `PHASE COUNT 0` is a POSITIVE statement that this
    // build looked and nothing was phased, which absence cannot express.
    let mut phased: Vec<&&ListingSymbol> = rows.iter().filter(|s| s.lma.is_some()).collect();
    phased.sort_by(|a, b| a.value.cmp(&b.value).then(a.name.cmp(&b.name)));
    out.push_str("\n  Phase Table (every address above is a VMA):\n");
    out.push_str("  -------------------------------------------\n\n");
    out.push_str(&format!("PHASE COUNT {}\n", phased.len()));
    for s in &phased {
        let lma = s.lma.expect("filtered to Some above");
        out.push_str(&format!("PHASE {} VMA ${:08X} LMA ${:08X}\n", s.name, s.value, lma));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(name: &str, value: u32, eq: bool, unused: bool) -> ListingSymbol {
        ListingSymbol { name: name.into(), value, is_equate: eq, unused, lma: None }
    }

    /// A PHASED address symbol: runs at `value`, stored at `lma`.
    fn phased(name: &str, value: u32, lma: u32) -> ListingSymbol {
        ListingSymbol { name: name.into(), value, is_equate: false, unused: false, lma: Some(lma) }
    }

    #[test]
    fn emits_s4budget_parseable_rows() {
        // Mirror s4budget's regex: (\*?)([\w.]+)\s*:\s*(hex|"str")\s+([C\-])\s*\|
        let out = emit_listing(&[
            sym("Main", 0x000000, false, false),
            sym("Boot", 0x40, false, false),
            sym("Unused", 0x2000, false, true),
        ]);
        assert!(out.contains("Symbol Table"));
        assert!(out.contains("unused"));
        // address-sorted; every symbol-table row is an address, marker C.
        assert!(out.contains("Main : 0 C |"));
        assert!(out.contains("Boot : 40 C |"));
        assert!(out.contains("*Unused : 2000 C |"));
        assert!(out.contains("3 symbols"));
        assert!(out.contains("1 unused symbols"));
        // No equates in this set → no Equate Table at all (format unchanged).
        assert!(!out.contains("Equate Table"), "an empty equate section was emitted:\n{out}");
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
            sym("Boot", 0x40, false, false),
        ]);
        // Oracle body lines (ParseLineHeader format) come first, address-sorted.
        // `(depth) N/HEXADDR :        Name:`
        assert!(out.contains("(0) 1/40 :        Boot:"), "missing/incorrect body line:\n{out}");
        assert!(out.contains("(0) 2/1000 :        Main:"), "missing/incorrect body line:\n{out}");
        // Every body line must precede the Symbol Table header (s4budget reads only
        // after that header; Oracle reads only the body lines).
        let body_idx = out.find("(0) 1/40").unwrap();
        let tab_idx = out.find("Symbol Table").unwrap();
        assert!(body_idx < tab_idx, "body lines must precede the symbol-table section");
        // The symbol-table section is still present and unchanged.
        assert!(out.contains("Main : 1000 C |"));
        assert!(out.contains("Boot : 40 C |"));
    }

    /// The equate row shape, stated as a contract: an equate lives ONLY in the
    /// Equate Table, as `EQU <name> = $<8 hex>`, and appears in NEITHER of the two
    /// address views.
    ///
    /// This is the half of the equ-listing parcel the emitter owns. `pub equ` mints
    /// a link-level `EquSym` with no label, so before this an equate had no row of
    /// any kind and a comptime-computed constant was unreadable by any tool.
    #[test]
    fn equates_get_a_value_row_and_no_address_row() {
        let out = emit_listing(&[
            sym("Main", 0x1000, false, false),
            sym("SCENE_OJZ_BUDGET", 0x2C, true, false),
            sym("Boot", 0x40, false, false),
        ]);
        // The value row exists, in the Equate Table, with the computed value.
        assert!(out.contains("Equate Table"), "no equate section:\n{out}");
        assert!(out.contains("EQU SCENE_OJZ_BUDGET = $0000002C"), "no equate row:\n{out}");
        assert!(out.contains("1 equates"), "no equate trailer:\n{out}");
        // …and NO address view names it — neither body line nor symbol-table row.
        assert!(
            !out.lines().any(|l| l.starts_with("(0) ") && l.contains("SCENE_OJZ_BUDGET")),
            "an equate reached the Oracle address listing:\n{out}"
        );
        assert!(
            !out.contains("SCENE_OJZ_BUDGET : "),
            "an equate reached the symbol table:\n{out}"
        );
        // The two address views stay 1:1 with each other AND with the trailer —
        // the s4budget cross-check that decides where an equate may live.
        assert!(out.contains("(0) 1/40 :        Boot:"), "body numbering:\n{out}");
        assert!(out.contains("(0) 2/1000 :        Main:"), "body numbering:\n{out}");
        assert!(out.contains("Boot : 40 C |") && out.contains("Main : 1000 C |"), "{out}");
        assert!(out.contains("2 symbols"), "the equate must not inflate the count:\n{out}");
    }

    /// COLLISION CONTROL. An equate row must never parse as an address row under
    /// any of aeon's `.lst` consumer grammars. Those are, verbatim:
    ///
    ///  * `tools/scene_spans.py::LST_HEAD_RE` —
    ///    `^\(\d+\) \d+/([0-9A-F]+) :\s+([A-Za-z_][A-Za-z0-9_]*):\s*$`
    ///    (drives `lst_proc_sizes`, hence `demo_specialization_witness.py`'s
    ///    proc-size differential);
    ///  * `tools/effects_gates.py`'s dense-stream probe —
    ///    `line.startswith("(0) ") and line.rstrip().endswith("<Name>:")`;
    ///  * `tools/s4budget.py`'s `_SYM_ROW_RE`
    ///    `^\s*(\*?)([\w.$]+)\s*:\s*([0-9A-Fa-f]+)\s+([C\-])\s*\|\s*$`, its
    ///    `_SRC_ROW_RE`, and its `<N> symbols` / `<N> unused symbols` trailers.
    ///
    /// Every one of them is an ADDRESS reader. The equate row starts with the
    /// literal `EQU `, carries no `/`, no ` : `, no `|`, and its trailer says
    /// `equates`, not `symbols` — proven here against a hostile equate deliberately
    /// named like a proc and valued like a ROM address.
    #[test]
    fn equate_row_never_parses_as_an_address_row() {
        // Named like a proc, valued like a real ROM address: if the shapes could
        // collide at all, this row is the one that would do it.
        let out = emit_listing(&[
            sym("Anchor", 0x200, false, false),
            sym("OJZ_GradientStream", 0x10AF2, true, false),
        ]);

        // scene_spans.LST_HEAD_RE, transcribed.
        let head_re = |l: &str| -> bool {
            let Some(rest) = l.strip_prefix('(') else { return false };
            let Some((depth, rest)) = rest.split_once(") ") else { return false };
            if depth.is_empty() || !depth.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            let Some((idx, rest)) = rest.split_once('/') else { return false };
            if idx.is_empty() || !idx.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            let Some((hex, rest)) = rest.split_once(" :") else { return false };
            if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return false;
            }
            let name = rest.trim_start();
            let Some(name) = name.strip_suffix(':') else { return false };
            !name.is_empty()
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && !name.as_bytes()[0].is_ascii_digit()
        };
        // effects_gates' dense-stream probe, transcribed.
        let gate_probe =
            |l: &str| l.starts_with("(0) ") && l.trim_end().ends_with("OJZ_GradientStream:");
        // s4budget's `_SYM_ROW_RE`, transcribed.
        let sym_row = |l: &str| -> bool {
            let l = l.trim_start().trim_start_matches('*');
            let Some((name, rest)) = l.split_once(':') else { return false };
            let name = name.trim_end();
            if name.is_empty()
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.$".contains(&b))
            {
                return false;
            }
            let rest = rest.trim_start();
            let Some((hex, rest)) = rest.split_once(' ') else { return false };
            hex.bytes().all(|b| b.is_ascii_hexdigit())
                && !hex.is_empty()
                && matches!(rest.trim().trim_end_matches('|').trim(), "C" | "-")
                && rest.trim_end().ends_with('|')
        };
        // s4budget's two trailers, transcribed.
        let trailer = |l: &str| {
            let t = l.trim();
            t.strip_suffix(" symbols").is_some_and(|n| {
                let n = n.trim_end_matches("unused").trim_end();
                !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())
            })
        };

        // Only the two EQUATE-SECTION lines are under test — the address half of
        // this listing is supposed to match, and does (controls below).
        for line in out.lines().filter(|l| l.starts_with("EQU ") || l.contains("equates")) {
            assert!(!head_re(line), "an equate line parsed as an address head: {line:?}");
            assert!(!gate_probe(line), "an equate line matched the effects gate probe: {line:?}");
            assert!(!sym_row(line), "an equate line parsed as a symbol-table row: {line:?}");
            assert!(!trailer(line), "an equate line parsed as a symbol trailer: {line:?}");
        }
        // Positive controls on the SAME transcriptions: the address rows DO match,
        // so the assertions above test the row shape, not a broken transcription.
        let addr = emit_listing(&[sym("OJZ_GradientStream", 0x10AF2, false, false)]);
        assert!(
            addr.lines().any(head_re),
            "the transcribed LST_HEAD_RE matches no address row, the control is broken:\n{addr}"
        );
        assert!(addr.lines().any(gate_probe), "the transcribed gate probe is broken:\n{addr}");
        assert!(addr.lines().any(sym_row), "the transcribed _SYM_ROW_RE is broken:\n{addr}");
        assert!(addr.lines().any(trailer), "the transcribed trailer regex is broken:\n{addr}");
        // And the equate's value is readable from its own row regardless.
        assert!(out.contains("EQU OJZ_GradientStream = $00010AF2"), "{out}");
    }

    /// The s4budget INVARIANT this design exists to preserve, asserted directly:
    /// the two address views are the same `(name, value)` sequence, of the same
    /// length, equal to the `N symbols` trailer — with equates present.
    #[test]
    fn equates_do_not_disturb_the_two_view_cross_check() {
        let out = emit_listing(&[
            sym("Main", 0x1000, false, false),
            sym("A_EQ", 0x2C, true, false),
            sym("Boot", 0x40, false, false),
            sym("Z_EQ", 0xFFFF0000, true, false),
            sym("Tail", 0x8000, false, true),
        ]);
        let body: Vec<(String, u32)> = out
            .lines()
            .filter_map(|l| l.strip_prefix("(0) "))
            .map(|l| {
                let (head, name) = l.split_once(" :").unwrap();
                let hex = head.split_once('/').unwrap().1;
                (name.trim().trim_end_matches(':').to_string(), u32::from_str_radix(hex, 16).unwrap())
            })
            .collect();
        let table: Vec<(String, u32)> = out
            .lines()
            .filter(|l| l.trim_end().ends_with(" C |"))
            .map(|l| {
                let l = l.trim_start_matches([' ', '*']);
                let (name, rest) = l.split_once(" : ").unwrap();
                let hex = rest.split_once(' ').unwrap().0;
                (name.to_string(), u32::from_str_radix(hex, 16).unwrap())
            })
            .collect();
        assert_eq!(body, table, "the two address views must be one table");
        assert_eq!(body.len(), 3, "only the three address symbols: {body:?}");
        assert!(out.contains("3 symbols") && out.contains("1 unused symbols"), "{out}");
        assert!(out.contains("2 equates"), "{out}");
    }

    /// THE CASE THAT CLOSES THE AMBIGUITY, and the one nothing else exercises.
    ///
    /// A listing of an entirely unphased program still carries the Phase Table,
    /// with `PHASE COUNT 0` and no rows. Without the unconditional header a reader
    /// cannot tell "this sigil looked and found nothing phased" from "this sigil
    /// predates the marker and every address here might be either", and the two would
    /// be spelled identically, as an absent section.
    #[test]
    fn unphased_listing_still_carries_a_count_zero_phase_table() {
        let out = emit_listing(&[
            sym("Main", 0x1000, false, false),
            sym("Boot", 0x40, false, false),
            sym("OBJ_len", 0x40, true, false),
        ]);
        assert!(out.contains("Phase Table"), "no phase section on an unphased listing:\n{out}");
        assert!(out.contains("PHASE COUNT 0"), "no zero count:\n{out}");
        // No rows at all, and in particular no row for the unphased symbols.
        assert!(
            !out.lines().any(|l| l.starts_with("PHASE ") && l.contains(" VMA ")),
            "a row was emitted for an unphased symbol:\n{out}"
        );
        // The rest of the listing is untouched: the marker ADDS a section, it does
        // not reinterpret or renumber any existing row.
        assert!(out.contains("(0) 1/40 :        Boot:"), "{out}");
        assert!(out.contains("(0) 2/1000 :        Main:"), "{out}");
        assert!(out.contains("Boot : 40 C |") && out.contains("Main : 1000 C |"), "{out}");
        assert!(out.contains("2 symbols"), "the phase table must not disturb the count:\n{out}");
        assert!(out.contains("EQU OBJ_len = $00000040"), "{out}");
    }

    /// A phased symbol gets a row naming BOTH addresses, and its address rows are
    /// unchanged: `value` was already the VMA and stays the VMA everywhere.
    #[test]
    fn phased_symbols_get_vma_and_lma_rows() {
        let out = emit_listing(&[
            sym("Anchor", 0x200, false, false),
            phased("SoundTablesZ80_Head", 0x8000, 0xE12C0),
            phased("SfxBlobWinTab", 0x845F, 0xE171F),
            sym("Tail", 0x20000, false, false),
        ]);
        assert!(out.contains("PHASE COUNT 2"), "wrong count:\n{out}");

        // CROSS-LANE CONTRACT, oracle 2026-09-06. Oracle's listing parser keys
        // recognition on `PHASE` being the FIRST CHARACTER of every row in this
        // section, so that its recognition survives a rewording of the header
        // sentence. That is a stronger promise than "first non-whitespace token"
        // and it is the one this emitter actually makes: both write sites emit
        // `PHASE` at column 0 while the header and its rule are indented two
        // spaces. It is pinned HERE rather than left to the format strings
        // because a promise made to another repo in a message rots silently,
        // and the lane that would break it is this one.
        for line in out.lines().filter(|l| l.contains(" VMA $") || l.starts_with("PHASE COUNT")) {
            assert!(
                line.starts_with("PHASE"),
                "a phase-section row must begin with PHASE at column 0, oracle keys on it: {line:?}"
            );
        }
        assert!(
            out.lines().any(|l| l.starts_with("PHASE COUNT ")),
            "the count line must be at column 0 too:\n{out}"
        );
        assert!(
            out.contains("PHASE SoundTablesZ80_Head VMA $00008000 LMA $000E12C0"),
            "missing/incorrect phased row:\n{out}"
        );
        assert!(
            out.contains("PHASE SfxBlobWinTab VMA $0000845F LMA $000E171F"),
            "missing/incorrect phased row:\n{out}"
        );
        // Address-sorted, like the two address views.
        let a = out.find("PHASE SoundTablesZ80_Head").unwrap();
        let b = out.find("PHASE SfxBlobWinTab").unwrap();
        assert!(a < b, "phase rows are not address-sorted:\n{out}");
        // The two address views still carry the phased symbols at their VMA, and
        // still cross-check 1:1 against the trailer, so the marker is ADDITIVE.
        assert!(out.contains("(0) 2/8000 :        SoundTablesZ80_Head:"), "{out}");
        assert!(out.contains("SoundTablesZ80_Head : 8000 C |"), "{out}");
        assert!(out.contains("4 symbols"), "{out}");
        // No unphased symbol acquired a row.
        assert!(!out.contains("PHASE Anchor"), "{out}");
        assert!(!out.contains("PHASE Tail"), "{out}");
    }

    /// COLLISION CONTROL for the phase rows, the same shape as the equate one: the
    /// header, the rule, the count line and a hostile row must parse as an address
    /// row under NONE of aeon's four `.lst` consumer grammars, proven against the
    /// same transcriptions with positive controls so a broken transcription cannot
    /// pass this test by matching nothing.
    #[test]
    fn phase_lines_never_parse_as_an_address_row() {
        // Named like a proc, valued like a real ROM address at both ends.
        let out = emit_listing(&[
            sym("Anchor", 0x200, false, false),
            phased("OJZ_GradientStream", 0x10AF2, 0x2C0FE),
        ]);
        // Same transcriptions the equate control uses.
        let head_re = |l: &str| -> bool {
            let Some(rest) = l.strip_prefix('(') else { return false };
            let Some((depth, rest)) = rest.split_once(") ") else { return false };
            if depth.is_empty() || !depth.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            let Some((idx, rest)) = rest.split_once('/') else { return false };
            if idx.is_empty() || !idx.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            let Some((hex, rest)) = rest.split_once(" :") else { return false };
            if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return false;
            }
            let name = rest.trim_start();
            let Some(name) = name.strip_suffix(':') else { return false };
            !name.is_empty()
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && !name.as_bytes()[0].is_ascii_digit()
        };
        let gate_probe = |l: &str| l.starts_with("(0) ") && l.trim_end().ends_with(':');
        let sym_row = |l: &str| -> bool {
            let l = l.trim_start().trim_start_matches('*');
            let Some((name, rest)) = l.split_once(':') else { return false };
            let name = name.trim_end();
            if name.is_empty()
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.$".contains(&b))
            {
                return false;
            }
            let rest = rest.trim_start();
            let Some((hex, rest)) = rest.split_once(' ') else { return false };
            hex.bytes().all(|b| b.is_ascii_hexdigit())
                && !hex.is_empty()
                && matches!(rest.trim().trim_end_matches('|').trim(), "C" | "-")
                && rest.trim_end().ends_with('|')
        };
        // BOTH s4budget trailers, transcribed as anchored matches.
        let trailer = |l: &str| {
            let t = l.trim();
            [" symbols", " unused symbols"].iter().any(|suffix| {
                t.strip_suffix(suffix)
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
        };

        // Every line the phase section contributes: header, rule, blank, count, row.
        let phase_start = out.find("  Phase Table").expect("no phase section");
        for line in out[phase_start..].lines() {
            assert!(!head_re(line), "a phase line parsed as an address head: {line:?}");
            assert!(!gate_probe(line), "a phase line matched the effects gate probe: {line:?}");
            assert!(!sym_row(line), "a phase line parsed as a symbol-table row: {line:?}");
            assert!(!trailer(line), "a phase line parsed as a symbol trailer: {line:?}");
        }
        // Positive controls on the SAME transcriptions: real rows DO match, so the
        // assertions above test the phase row shape, not a dead transcription.
        let addr = emit_listing(&[sym("OJZ_GradientStream", 0x10AF2, false, false)]);
        assert!(addr.lines().any(head_re), "the transcribed LST_HEAD_RE is broken:\n{addr}");
        assert!(addr.lines().any(gate_probe), "the transcribed gate probe is broken:\n{addr}");
        assert!(addr.lines().any(sym_row), "the transcribed _SYM_ROW_RE is broken:\n{addr}");
        assert!(addr.lines().any(trailer), "the transcribed trailer regex is broken:\n{addr}");
        // And the count line specifically, which is the line that is ALWAYS there.
        assert!(!trailer("PHASE COUNT 0"), "the count line parsed as a symbol trailer");
        assert!(!sym_row("PHASE COUNT 0"), "the count line parsed as a symbol row");
    }

    /// s4budget's cross-check invariant, re-asserted with PHASED symbols present:
    /// the two address views stay one table, at the VMA, and the trailer agrees.
    #[test]
    fn phase_table_does_not_disturb_the_two_view_cross_check() {
        let out = emit_listing(&[
            sym("Main", 0x1000, false, false),
            phased("BankHead", 0x8000, 0xE0000),
            sym("Boot", 0x40, false, false),
            sym("A_EQ", 0x2C, true, false),
        ]);
        let body: Vec<(String, u32)> = out
            .lines()
            .filter_map(|l| l.strip_prefix("(0) "))
            .map(|l| {
                let (head, name) = l.split_once(" :").unwrap();
                let hex = head.split_once('/').unwrap().1;
                (name.trim().trim_end_matches(':').to_string(), u32::from_str_radix(hex, 16).unwrap())
            })
            .collect();
        let table: Vec<(String, u32)> = out
            .lines()
            .filter(|l| l.trim_end().ends_with(" C |"))
            .map(|l| {
                let l = l.trim_start_matches([' ', '*']);
                let (name, rest) = l.split_once(" : ").unwrap();
                let hex = rest.split_once(' ').unwrap().0;
                (name.to_string(), u32::from_str_radix(hex, 16).unwrap())
            })
            .collect();
        assert_eq!(body, table, "the two address views must be one table");
        assert_eq!(body.len(), 3, "only the three address symbols: {body:?}");
        assert!(out.contains("3 symbols"), "{out}");
        // The phased symbol appears in BOTH address views at its VMA, and once more
        // in the phase table with its LMA. Three rows, one truth, no reinterpretation.
        assert!(body.contains(&("BankHead".to_string(), 0x8000)), "{body:?}");
        assert!(out.contains("PHASE COUNT 1") && out.contains("PHASE BankHead VMA $00008000 LMA $000E0000"), "{out}");
    }

    /// An EQUATE is never phased, whatever it carries: it has a value, not storage.
    #[test]
    fn an_equate_never_reaches_the_phase_table() {
        let out = emit_listing(&[
            sym("Anchor", 0x200, false, false),
            ListingSymbol {
                name: "BANK_BASE".into(),
                value: 0x8000,
                is_equate: true,
                unused: false,
                lma: Some(0xE0000),
            },
        ]);
        assert!(out.contains("PHASE COUNT 0"), "an equate was counted as phased:\n{out}");
        assert!(!out.contains("PHASE BANK_BASE"), "an equate got a phase row:\n{out}");
        assert!(out.contains("EQU BANK_BASE = $00008000"), "{out}");
    }
}
