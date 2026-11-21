//! `[proc.repeated-clause]` (A10, W13): a contract clause written twice on one header is
//! refused. Before, the parser kept the last one silently.
use sigil_frontend_emp::parse_str;
use sigil_span::Level;

const ID: &str = "[proc.repeated-clause]";

fn refusals(src: &str) -> Vec<String> {
    parse_str(src).1.into_iter().filter(|d| d.level == Level::Error && d.message.contains(ID)).map(|d| d.message).collect()
}

#[test]
fn a_repeated_clobbers_clause_is_refused() {
    let r = refusals("module m\nproc P () clobbers(d0) clobbers() {\n    moveq #1, d0\n    rts\n}\n");
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].contains("`clobbers` is written twice on `P`"), "{}", r[0]);
}

#[test]
fn every_clause_word_is_refused_when_repeated() {
    let r = refusals("module m\nproc P () requires(v) requires(w) grants(v) grants(w) out(d0) out(d1) inout(d1) inout(d2) preserves(d2) preserves(d3) falls_into Q falls_into R {\n    rts\n}\n");
    for kw in ["requires", "grants", "out", "inout", "preserves", "falls_into"] {
        assert_eq!(r.iter().filter(|m| m.contains(&format!("`{kw}` is written twice"))).count(), 1, "{kw}: {r:?}");
    }
    assert_eq!(r.len(), 6, "{r:?}");
}

#[test]
fn a_repeated_clause_on_a_signature_is_refused() {
    let r = refusals("module m\nextern proc E () clobbers(d0) clobbers()\npub type T = proc () preserves(d0) preserves(d1)\npub interface G {\n    hook h () out(d0) out(d1) = empty\n}\n");
    assert_eq!(r.len(), 3, "{r:?}");
    assert!(r.iter().all(|m| m.contains("on `this signature`")), "{r:?}");
}

/// The control: every clause once, in an order the grammar allows, parses clean.
#[test]
fn each_clause_once_stays_legal() {
    let (_, diags) = parse_str("module m\ncontext v { granted }\nproc Q () clobbers() {\n    rts\n}\nproc P (d1: u8) requires(v) grants(v) clobbers(d0) out(d0) preserves(d2) inout(d1) falls_into Q {\n    moveq #0, d0\n}\n");
    assert!(diags.iter().all(|d| d.level != Level::Error), "{diags:?}");
}
