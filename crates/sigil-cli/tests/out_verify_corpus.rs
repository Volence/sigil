//! Contract-grammar v2 §G4.5 — the callee-side `out()` production residue over
//! the REAL aeon corpus, and D1c beside it. The residue is DUMPED for reading and
//! PINNED for enforcement: assert-empty is unavailable while most of the residue
//! is verifier-model gap rather than loose contract, so the gate is a ratchet.
//!
//! BOTH families now have TEETH, against the SHARED baseline constants in
//! `sigil_harness::contract_baseline` that the build-integrated closure gate also
//! reads — one copy, so a pin cannot fork into two halves that disagree.

use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::lexer::{lex, Tok};
use sigil_frontend_emp::out_verify::survives_message;
use sigil_frontend_emp::parse_str;
use sigil_span::SourceId;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn emp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == ".worktrees") {
                continue;
            }
            emp_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

/// The reference aeon tree's `.emp` sources as `(path, text)`, sorted by path, or
/// `None` when that tree is absent and the run is not strict.
///
/// House reference-gate pattern (repin_pins/mt_port, c5505f8): default the
/// sibling aeon tree; under `SIGIL_STRICT_GATE` a missing reference hard-fails so
/// these gates actually run under the standard strict invocation.
fn corpus_sources() -> Option<Vec<(PathBuf, String)>> {
    let aeon = sigil_harness::test_support::aeon_dir();
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return None;
    }
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());
    Some(
        paths
            .into_iter()
            .map(|p| {
                let text = std::fs::read_to_string(&p)
                    .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                (p, text)
            })
            .collect(),
    )
}

/// The whole-corpus contract report over `sources`, each parsed on its own.
fn analyze_sources(sources: &[(PathBuf, String)]) -> ContractReport {
    let files: Vec<_> = sources.iter().map(|(_, text)| parse_str(text).0).collect();
    analyze_corpus(&files)
}

/// The whole-corpus contract report over the reference aeon tree, or `None` when
/// that tree is absent and the run is not strict.
fn corpus_report() -> Option<ContractReport> {
    corpus_sources().map(|sources| analyze_sources(&sources))
}

/// A typed out slot, `(declaration, register)`: the shape of the report's
/// `typed_out_slots` rows and of every list compared with them here.
type Slot = (String, String);

/// The canonical 68k register a slot names, spelled as the report spells it
/// (`d0`..`d7`, `a0`..`a7`, with `sp` as `a7`), or `None` for any other name (a
/// flag such as `carry`, a Z80 register).
fn register_name(name: &str) -> Option<String> {
    if name == "sp" {
        return Some("a7".to_string());
    }
    let b = name.as_bytes();
    (b.len() == 2 && matches!(b[0], b'd' | b'a') && (b'0'..=b'7').contains(&b[1]))
        .then(|| name.to_string())
}

/// Every `out(rN: T)` slot the sources DECLARE, as `(declaration, register)`,
/// sorted. This is the expectation half of the coverage check in
/// [`no_corpus_out_type_is_unresolvable`], so it is read off the LEXER's token
/// stream and runs neither the parser nor the report's type walk, the two things
/// whose product it is compared with.
///
/// A slot is a register name followed by `:` at the start of a top-level segment
/// of an `out(...)` group, where segments separate on `,` and `/` as the out
/// clause's grammar has them. The flag form `carry: name` names no register and
/// is not a slot, and `inout(...)` is a different keyword that is not read. The
/// declaration owning a group is the nearest head before it: `proc NAME` (plain
/// or `extern`), `hook NAME`, or `type NAME = proc` for a contract type. An
/// interface `hook` is a head here although the report's walk visits no
/// interface member, so a typed hook result reads as a slot the walk did not
/// see. A typed register with no head before it refuses, naming the file, rather
/// than being dropped.
fn declared_typed_out_slots(sources: &[(PathBuf, String)]) -> Vec<Slot> {
    let mut slots = Vec::new();
    for (path, text) in sources {
        let (tokens, _) = lex(text, SourceId(0));
        let toks: Vec<&Tok> = tokens
            .iter()
            .map(|t| &t.tok)
            .filter(|t| !matches!(t, Tok::Newline | Tok::DocLine(_)))
            .collect();
        let ident = |i: usize| match toks.get(i) {
            Some(Tok::Ident(s)) => Some(s.as_str()),
            _ => None,
        };
        let mut owner: Option<&str> = None;
        for i in 0..toks.len() {
            match ident(i) {
                Some("proc" | "hook") if ident(i + 1).is_some() => owner = ident(i + 1),
                Some("proc") if i >= 2 && *toks[i - 1] == Tok::Eq => owner = ident(i - 2),
                Some("out") if toks.get(i + 1) == Some(&&Tok::LParen) => {
                    let mut depth = 1usize;
                    let mut at_segment_start = true;
                    let mut j = i + 2;
                    while depth > 0 {
                        let Some(t) = toks.get(j) else {
                            panic!("{}: an `out(` group never closes", path.display())
                        };
                        if depth == 1 && at_segment_start {
                            if let (Tok::Ident(name), Some(Tok::Colon)) = (t, toks.get(j + 1)) {
                                if let Some(reg) = register_name(name) {
                                    let Some(owner) = owner else {
                                        panic!(
                                            "{}: `out({name}: T)` has no `proc`, `hook` or \
                                             `type = proc` head before it",
                                            path.display()
                                        )
                                    };
                                    slots.push((owner.to_string(), reg));
                                }
                            }
                        }
                        match t {
                            Tok::LParen | Tok::LBracket => depth += 1,
                            Tok::RParen | Tok::RBracket => depth -= 1,
                            _ => {}
                        }
                        at_segment_start = depth == 1 && matches!(t, Tok::Comma | Tok::Slash);
                        j += 1;
                    }
                }
                _ => {}
            }
        }
    }
    slots.sort();
    slots
}

/// `(in a and not in b, in b and not in a)`, counting repeats, so the two lists
/// hold the same slots exactly when both halves are empty.
fn slot_difference(a: &[Slot], b: &[Slot]) -> (Vec<Slot>, Vec<Slot>) {
    let mut count: BTreeMap<&Slot, i64> = BTreeMap::new();
    for s in a {
        *count.entry(s).or_default() += 1;
    }
    for s in b {
        *count.entry(s).or_default() -= 1;
    }
    let (mut only_a, mut only_b) = (Vec::new(), Vec::new());
    for (s, n) in count {
        for _ in 0..n.max(0) {
            only_a.push(s.clone());
        }
        for _ in 0..(-n).max(0) {
            only_b.push(s.clone());
        }
    }
    (only_a, only_b)
}

#[test]
fn dump_out_unverified_residue() {
    let Some(r) = corpus_report() else { return };

    eprintln!("=== [proc.out-unverified] residue: {} firing(s) ===", r.out_firings.len());
    for f in &r.out_firings {
        eprintln!("  {} :: out({}), {}", f.proc, f.reg, f.reason);
    }
    eprintln!("=== [call.live-clobbered] D1c: {} firing(s) ===", r.live_clobbered_firings.len());
    for f in &r.live_clobbered_firings {
        eprintln!("  {} @ {} :: {}", f.proc, f.callee, f.reg);
    }
    eprintln!(
        "=== [proc.out-cond-survives-unverifiable]: {} firing(s) ===",
        r.survives_firings.len()
    );
    for f in &r.survives_firings {
        eprintln!("  {} :: out({} if {}), {}", f.proc, f.reg, f.cc, f.reason);
    }
    eprintln!(
        "=== Z80 [proc.out-unverified] residue: {} firing(s), over {} out claim(s) ===",
        r.z80_out_firings.len(),
        r.z80_out_claims.len()
    );
    for f in &r.z80_out_firings {
        eprintln!("  {} :: out({}), {}", f.proc, f.unit, f.reason);
    }
}

/// The §7.1 SURVIVES claim over the real corpus, under the closure's
/// callee-preserves oracle — the FINAL authority behind the per-file gate's
/// call-blocked deferrals, so it is an assert-EMPTY gate, not a dump.
///
/// Non-vacuity is asserted, not asserted-about: [`SURVIVES_CLAIM_SITES`] names
/// the procs that must still be MAKING a claim, so the empty assert cannot go
/// quietly true by a contract edit that deletes the claim instead of proving it.
/// (The obvious mutation — reverting `TileCache_FindStagedBlock` to
/// `clobbers(d3-d4)` — lives in the aeon tree, so no sigil-side test can perform
/// it; it was run by hand and the gate failed as designed.)
#[test]
fn cond_out_survives_claims_all_prove() {
    let Some(r) = corpus_report() else { return };

    let rows: Vec<String> = r.survives_firings.iter().map(survives_message).collect();
    assert!(
        rows.is_empty(),
        "[proc.out-cond-survives-unverifiable] over the aeon corpus, a conditional out \
         claims its register survives the failure edges and the proof does not carry:\n  {}",
        rows.join("\n  ")
    );
    // The gate must have subjects. A cond-out register ABSENT from the proc's
    // declared clobbers is what makes the claim, and `survives_claim_sites` is
    // exactly that set — NOT `verified_cond_out`, which is the PRODUCTION half's
    // output and stays true even if every claim were downgraded away.
    let claim_sites: Vec<&str> = r.survives_claim_sites.iter().map(String::as_str).collect();
    assert_eq!(
        claim_sites, SURVIVES_CLAIM_SITES,
        "the set of procs MAKING a survives claim moved. The assert above proves nothing \
         about a proc that stopped claiming; adjudicate and update SURVIVES_CLAIM_SITES"
    );
}

/// No `out(rN: T)` in the corpus names a type the corpus cannot resolve to a
/// width. An unresolvable type is not unsound — it answers the bare 32-bit claim
/// on both sides, exactly as the declaration would with no type at all — but it
/// silently means something other than what its author wrote, and a width cannot
/// be guessed from a name that resolves to nothing.
///
/// Assert-EMPTY rather than baselined: there is no adjudication to make. Either
/// the name is a typo, or the module declaring it is missing from the walk, and
/// both are fixed rather than pinned.
#[test]
fn no_corpus_out_type_is_unresolvable() {
    let Some(sources) = corpus_sources() else { return };
    let r = analyze_sources(&sources);
    let rows: Vec<String> = r
        .unresolvable_out_types
        .iter()
        .map(|(p, reg, ty)| format!("{p} :: out({reg}: {ty})"))
        .collect();
    assert!(
        rows.is_empty(),
        "an `out(rN: T)` names a type the corpus cannot resolve to a width, so it \
         silently claims all 32 bits instead of what was written:\n  {}",
        rows.join("\n  ")
    );
    // NON-VACUITY, measured on the SCAN's own subject matter. `typed_out_slots` is
    // what the type walk saw; a map keyed by proc NAME cannot serve here, because
    // it carries a key whether or not that proc declares a type, so stripping every
    // type off the corpus would leave it unchanged.
    let slots: Vec<(&str, &str)> =
        r.typed_out_slots.iter().map(|(p, g)| (p.as_str(), g.as_str())).collect();
    for exemplar in [
        ("Collision_GetType", "d0"),
        ("GetSineCosine", "d0"),
        ("Tile_Cache_GetTile", "d2"),
    ] {
        assert!(
            slots.contains(&exemplar),
            "the walk did not see `{} :: out({}: T)`, so the assert above ranges \
             over less than it should. slots: {slots:?}",
            exemplar.0,
            exemplar.1
        );
    }
    // COVERAGE, in both directions: the walk sees exactly the typed out slots the
    // corpus declares, so a slot it stops visiting and a slot it invents are each
    // named. The expectation is `declared_typed_out_slots`, an enumeration of the
    // same sources off the lexer's token stream that runs neither the parser nor
    // the walk, so it moves when the corpus moves and stays put when the walk
    // does. The declared set is printed, so a changed population is readable in
    // the run's output.
    let declared = declared_typed_out_slots(&sources);
    let (unseen, invented) = slot_difference(&declared, &r.typed_out_slots);
    assert!(
        unseen.is_empty() && invented.is_empty(),
        "the type walk's typed out slots are not the ones the corpus declares, so the \
         assert above ranges over the wrong set.\n  declared, not seen by the walk: \
         {unseen:?}\n  seen by the walk, not declared: {invented:?}"
    );
    eprintln!("=== typed out slots, declared and seen by the walk: {} ===", declared.len());
    for (p, g) in &declared {
        eprintln!("  {p} :: out({g}: T)");
    }
}

/// The typed out slots of a FIXED input, from both routes the coverage check in
/// [`no_corpus_out_type_is_unresolvable`] compares. The two share the lexer and
/// the file list, and the real corpus exercises only some declaration forms, so
/// this holds them to an input sigil owns: every head the walk visits (`proc`,
/// `extern proc`, a contract `type`, a proc inside a `section`), a conditional
/// typed result, an address register, a type with a comma inside it, a
/// `/`-separated segment, and three things that are not slots (a bare register, a
/// flag result, a typed `inout`). The literal cannot go stale, because the input
/// is written here.
#[test]
fn typed_out_slots_of_a_fixed_input() {
    let sources: Vec<(PathBuf, String)> = [
        "module m\n\
         pub newtype Id = u8\n\
         extern proc Ext () out(d0: Id, d1)\n\
         type Probe = proc () clobbers(d0) out(d4: i16, a1: *u8)\n\
         proc Plain () out(d2: u16, carry: failed) {\n move.w #1, d2\n rts\n}\n\
         proc Cond () out(d3: u8 if cc) {\n move.b #1, d3\n rts\n}\n\
         proc Wide () out(d0: fixed<8,8>, d1: fixed<8,8>) {\n move.w #1, d0\n move.w #1, d1\n rts\n}\n\
         proc Moves () out(d0/d5: u16) {\n moveq #0, d0\n move.w #1, d5\n rts\n}\n\
         proc InOut (d6: u16) inout(d6: u16) {\n addq.w #1, d6\n rts\n}\n",
        "module n\n\
         section s {\n\
         proc InSection () out(d7: u8) {\n move.b #1, d7\n rts\n}\n\
         }\n",
    ]
    .iter()
    .enumerate()
    .map(|(i, text)| (PathBuf::from(format!("fixed{i}.emp")), text.to_string()))
    .collect();
    for (path, text) in &sources {
        let (_, diags) = parse_str(text);
        assert!(diags.is_empty(), "{}: the fixed input must parse cleanly: {diags:?}", path.display());
    }
    let want: Vec<Slot> = [
        ("Cond", "d3"),
        ("Ext", "d0"),
        ("InSection", "d7"),
        ("Moves", "d5"),
        ("Plain", "d2"),
        ("Probe", "a1"),
        ("Probe", "d4"),
        ("Wide", "d0"),
        ("Wide", "d1"),
    ]
    .iter()
    .map(|(p, g)| (p.to_string(), g.to_string()))
    .collect();
    assert_eq!(
        analyze_sources(&sources).typed_out_slots,
        want,
        "the report's type walk over the fixed input"
    );
    assert_eq!(
        declared_typed_out_slots(&sources),
        want,
        "the token enumeration over the fixed input"
    );
}

/// Every proc that declares `out(rN if cc)` with rN ABSENT from its `clobbers` —
/// i.e. every proc whose survives claim the gate above actually proves.
/// `AllocDynamic` is deliberately not here: it names a1 in `clobbers` and makes
/// no claim.
const SURVIVES_CLAIM_SITES: &[&str] = &["AllocEffect"];

// The `[call.live-clobbered]` and `[proc.out-unverified]` baseline gates do NOT
// live here, and the reason is load-bearing: this file's `corpus_report()` walks
// DEFINE-FREE, and a define-free walk discards BOTH arms of every
// `if DEBUG == 1 { }` / `if SOUND_DRIVER_ENABLED == 1 { }`. It is a strict
// under-approximation of every shipped shape, not the view of any one of them —
// see `contract_closure_corpus.rs`'s header, which says so directly. Pinning a
// frozen baseline against it would make the pin unsatisfiable the moment a gated
// arm gained or lost a firing: the define-free walk would report one count and
// the per-shape builds another, with no baseline value satisfying both. Those
// gates walk every shipped shape, in `contract_closure_corpus.rs`.
