//! The gate that measures what sigil ACCEPTS and asl REFUSES.
//!
//! ## The gap this closes
//!
//! Every other gate in this repo is one-directional. Byte identity against a
//! known-good ROM and "the corpus still assembles" both go red when sigil gets
//! STRICTER and stay green when it gets LOOSER, because a program the corpora
//! do not contain is a program no corpus gate has an opinion about. So sigil
//! could grow to accept anything at all outside the corpora and the whole suite
//! would report success.
//!
//! That is not hypothetical. `docs/superpowers/notes/2026-09-10-as-circular-split.md`
//! section 6 records three shapes sigil accepts and asl refuses, and they were
//! found by a human writing probe files by hand, one parcel at a time. Section 5
//! of the same note says why: nothing anywhere measures over-refusal or
//! over-acceptance, so the obvious half-fix of that parcel would have passed all
//! 5,034 tests then existing while doing nothing.
//!
//! ## What it measures, and why a count would not do
//!
//! Two sets, both named member by member:
//!
//! - **over-acceptance**: probes asl refuses and sigil accepts.
//! - **over-refusal**: probes asl accepts and sigil refuses.
//!
//! `observed \ ledger` must be EMPTY, in both directions, and that is the hard
//! gate: a divergence nobody wrote down is a regression. `ledger \ observed` is
//! computed and REPORTED and deliberately not asserted, because a ledger row
//! that has quietly started agreeing with asl is good news to be retired on
//! purpose, not a red on arrival.
//!
//! The comparison is a set difference in both directions and never a count.
//! Two populations can differ by one member in each direction and total
//! identically, so a count gate would read green across exactly the swap it
//! exists to catch.
//!
//! ## The probe population, and how it was derived
//!
//! 88 probes in `over_acceptance/probes/`. asl refuses 61 of them, covering 41
//! distinct numbered refusal classes, and accepts the other 27.
//!
//! **A corpus assembled from the shapes we already knew would be a gate that
//! could not come out other than green.** It would pass on the day it landed,
//! pass forever, and measure nothing. So the population is derived from asl
//! rather than from our own findings: asl ships its complete error catalogue in
//! `as.msg` beside the binary, about 200 messages, and the corpus is that
//! catalogue filtered to the classes reachable from 68000 and Z80 source in the
//! AS syntax sigil parses, one minimal probe per class. The three known
//! divergences are in the corpus, but as three members of 88 rather than as the
//! corpus.
//!
//! Six probes come from a second source and are the exception that proves the
//! rule about deriving from asl: `docs/superpowers/notes/campaign-gap-ledger.md`
//! rows 3 and 4 name expression-tier divergences somebody had already measured
//! and nothing watched, and row 5 names "a corpus of asl-REFUSED expressions
//! asserted refused" as its own kill condition. Four of the six reproduce and
//! are ledgered; two do not and are live tripwires instead.
//!
//! The 27 asl-ACCEPTED probes are not filler and the gate is unsound without
//! them. Roughly half are the in-range twin of a refused probe (`dc.b 255`
//! beside `dc.b 256`, `bit 7,a` beside `bit 8,a`, a short branch in range beside
//! one out of range), which makes the corpus a boundary corpus and gives the
//! over-refusal direction something to measure. Without them a sigil that
//! refused every program would produce an empty over-acceptance set and read
//! green.
//!
//! The three disassembly corpora were considered as a probe source and ruled out
//! on measurement rather than on taste: s1disasm, s2disasm and skdisasm all
//! assemble under asl, so by construction they contain no asl-refused construct
//! and no sweep of them can yield a refusal probe.
//!
//! ## Where asl's verdicts come from, and why no byte appears here
//!
//! `over_acceptance/asl_verdicts.txt`, minted by
//! `scripts/mint_over_acceptance_verdicts.sh` from the reference asl selected by
//! md5 `61e672562465725a8c102288a7da9098` through
//! `docs/superpowers/notes/asl-reference/asl_ref.sh`. Seven asl paths on this
//! machine execute under four distinct digests and every one prints
//! `Macro Assembler 1.42 Beta [Bld 212]`, so the digest and not the banner is
//! the identity. The gate reads the committed table and needs no asl, the same
//! split `asl_snippets.rs` uses.
//!
//! **The standing rule is "an asl run carrying any error is not a source of
//! values", and this corpus inverts its usual application.** Most probes here
//! are SUPPOSED to exit nonzero: a nonzero exit is the subject of the
//! measurement, not a fault in it. So the rule cannot be discharged by checking
//! the exit status, and becomes instead: never quote an emitted VALUE out of
//! these runs. The table records accept-or-refuse and the diagnostic text and
//! nothing else, and no expectation anywhere in this file is a byte. That is
//! deliberate and load-bearing, because an error stops asl's pass loop, forward
//! references are left at their unresolved pass-1 placeholder, and the listing
//! prints them looking complete.
//!
//! ## The two controls, and why the canary alone would not have done
//!
//! `positive_control_the_comparison_can_fire` proves the comparison FIRES: it
//! runs the same `divergences` function the gate runs, over a synthetic table
//! carrying an unledgered over-acceptance and an unledgered over-refusal, and
//! requires both to be reported. A canary covers the rule.
//!
//! `feed_control_every_probe_reached_both_assemblers` covers the FEED, which is
//! the half a canary cannot reach. A scanner that silently processed one file
//! instead of 88 prints "clean" with a working comparison behind it. So the
//! probe count the run actually processed is asserted against a literal, and the
//! probe-file name set and the verdict-table name set are asserted equal in both
//! directions, so a probe with no verdict and a verdict with no probe are each a
//! failure rather than a silent drop.
//!
//! ## Would anything go red? The headroom, measured
//!
//! A gate whose ledger already names every divergence in its corpus is green on
//! the day it lands and green forever, and that is the failure mode this parcel
//! was warned about. So the number that matters is not the ledger's size but how
//! many probes are LIVE TRIPWIRES: probes on which sigil and asl currently
//! agree, and where a change of behaviour in either direction therefore reds a
//! gate.
//!
//! Measured on the corpus as it stands: 88 probes, 14 ledgered divergences, so
//! **74 live tripwires**: 51 probes both assemblers refuse, where sigil becoming
//! looser goes red, and 23 both accept, where sigil becoming stricter goes red. `feed_control_the_ledger_is_not_an_escape_hatch` floors both
//! numbers, so the way to get green after a widening is not to ledger it: that
//! reds a second gate whose floor has to be moved by hand in the same diff.
//!
//! Ten of the fourteen ledgered rows were found by this gate rather than
//! inherited. Three were known (the note's `c4`, `c5`, `d2`), four were already
//! written down in the gap ledger with nothing watching them, and three were not
//! known to anyone. Those last three are the evidence that deriving the
//! population from asl's catalogue rather than from our own findings was the
//! load-bearing choice.
//!
//! ## Loud on unmeasurable
//!
//! There is no skip path. A missing probe directory, a missing or empty verdict
//! table, a missing ledger and a probe that panics sigil are each a FAILURE.
//! "Could not measure" is never rendered as zero and never as green.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sigil_frontend_as::{assemble_root_located, Options};

/// The probe count this gate is known to cover. A literal, and the feed control
/// asserts the run reached it. It is not derived from the directory listing,
/// because an expectation read off its own subject moves with the subject and
/// can never disagree with it.
const EXPECTED_PROBE_COUNT: usize = 88;

/// The floor on distinct numbered asl refusal classes the corpus exercises.
/// This is what stops the corpus being quietly gutted down to the handful of
/// shapes that happen to be ledgered: a gate over four probes and a gate over
/// eighty-two are different instruments with the same green.
const MIN_ASL_REFUSAL_CLASSES: usize = 41;

/// Probes both assemblers currently REFUSE. Each is a live tripwire for sigil
/// becoming looser, and their number is the gate's real headroom.
const MIN_AGREED_REFUSALS: usize = 51;

/// Probes both assemblers currently ACCEPT. Each is a live tripwire for sigil
/// becoming stricter.
const MIN_AGREED_ACCEPTANCES: usize = 23;

// ---------------------------------------------------------------------------
// Reading the committed inputs
// ---------------------------------------------------------------------------

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/over_acceptance")
}

/// One row of the committed asl verdict table.
#[derive(Debug, Clone)]
struct AslVerdict {
    accepted: bool,
    /// asl's numbered class, e.g. `#1320 range overflow`, or the recorded
    /// reason there is none. Never a byte.
    class: String,
}

fn read_asl_verdicts() -> BTreeMap<String, AslVerdict> {
    let path = data_dir().join("asl_verdicts.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read the asl verdict table at {}: {e}. This gate has no skip \
             path: without the table there is no oracle and the answer is not zero \
             divergences, it is no measurement. Re-mint with \
             scripts/mint_over_acceptance_verdicts.sh.",
            path.display()
        )
    });
    let mut out = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        assert!(
            f.len() >= 4,
            "malformed verdict row (want name/verdict/exit/diag/message): {line:?}"
        );
        let accepted = match f[1] {
            "accept" => true,
            "refuse" => false,
            other => panic!("unknown verdict {other:?} in row: {line:?}"),
        };
        let class = f.get(4).copied().unwrap_or("").to_string();
        out.insert(f[0].to_string(), AslVerdict { accepted, class });
    }
    assert!(
        !out.is_empty(),
        "the asl verdict table parsed to zero rows. An empty oracle produces an \
         empty divergence set, which is the shape of a clean run."
    );
    out
}

/// One ledgered divergence: a shape where sigil and asl are known to disagree
/// and the disagreement is currently tolerated, with the reason it is.
#[derive(Debug, Clone)]
struct LedgerRow {
    direction: String,
    reason: String,
}

fn read_ledger() -> BTreeMap<String, LedgerRow> {
    let path = data_dir().join("ledger.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read the divergence ledger at {}: {e}. Without it every known \
             divergence reads as new and every new one reads as known, depending \
             on which way the missing file is defaulted; neither is measured.",
            path.display()
        )
    });
    let mut out = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        assert!(
            f.len() >= 3,
            "malformed ledger row (want name/direction/reason): {line:?}"
        );
        assert!(
            f[1] == "over-acceptance" || f[1] == "over-refusal",
            "unknown ledger direction {:?} in row: {line:?}",
            f[1]
        );
        out.insert(
            f[0].to_string(),
            LedgerRow {
                direction: f[1].to_string(),
                reason: f[2].to_string(),
            },
        );
    }
    out
}

fn probe_paths() -> Vec<(String, PathBuf)> {
    let dir = data_dir().join("probes");
    let rd = std::fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!(
            "cannot read the probe directory at {}: {e}. This gate has no skip path.",
            dir.display()
        )
    });
    let mut out: Vec<(String, PathBuf)> = rd
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().map(|x| x == "asm").unwrap_or(false))
        .map(|p| {
            (
                p.file_stem().unwrap().to_string_lossy().into_owned(),
                p,
            )
        })
        .collect();
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// Running sigil
// ---------------------------------------------------------------------------

/// What sigil did with one probe. `Panic` is a third state on purpose: a panic
/// is neither an acceptance nor a refusal, and folding it into either would let
/// a crash read as agreement with asl.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SigilVerdict {
    Accept,
    Refuse(String),
    Panic(String),
}

/// Assemble one probe end to end, exactly as `circular_layout.rs` and
/// `as_mompass_builtin.rs` do (front end, then layout resolution, then link,
/// then flatten), because a program is only ACCEPTED if it survives all four.
/// A refusal at any stage is a refusal.
fn sigil_verdict(path: &Path) -> SigilVerdict {
    let p = path.to_path_buf();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let r = std::panic::catch_unwind(move || {
        match assemble_root_located(&p, &Options::default()) {
            Ok(m) => {
                let resolved = match sigil_link::resolve_layout(
                    &m.sections,
                    &sigil_ir::SymbolTable::new(),
                    true,
                ) {
                    Ok(s) => s,
                    Err(d) => {
                        return SigilVerdict::Refuse(
                            d.iter()
                                .map(|x| x.message.clone())
                                .collect::<Vec<_>>()
                                .join(" || "),
                        )
                    }
                };
                let linked = match sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()) {
                    Ok(l) => l,
                    Err(d) => {
                        return SigilVerdict::Refuse(
                            d.iter()
                                .map(|x| x.message.clone())
                                .collect::<Vec<_>>()
                                .join(" || "),
                        )
                    }
                };
                match sigil_link::flatten(&linked, 0x00) {
                    Ok(_) => SigilVerdict::Accept,
                    Err(e) => SigilVerdict::Refuse(e),
                }
            }
            Err(f) => SigilVerdict::Refuse(
                f.diags
                    .iter()
                    .map(|d| d.message.clone())
                    .collect::<Vec<_>>()
                    .join(" || "),
            ),
        }
    });
    std::panic::set_hook(hook);
    match r {
        Ok(v) => v,
        Err(e) => {
            let m = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "<non-string panic payload>".to_string());
            SigilVerdict::Panic(m)
        }
    }
}

// ---------------------------------------------------------------------------
// The comparison
// ---------------------------------------------------------------------------

/// Both divergence directions over one pair of verdict maps.
///
/// Factored out as a pure function on purpose: the positive control drives THIS
/// function over a synthetic table, so the control exercises the comparison the
/// gate actually runs rather than a re-implementation of it that could agree
/// with a broken original.
///
/// Returns `(over_acceptance, over_refusal)`.
fn divergences(
    asl: &BTreeMap<String, bool>,
    sigil: &BTreeMap<String, bool>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut over_acceptance = BTreeSet::new();
    let mut over_refusal = BTreeSet::new();
    for (name, &asl_accepted) in asl {
        let Some(&sigil_accepted) = sigil.get(name) else {
            continue;
        };
        if !asl_accepted && sigil_accepted {
            over_acceptance.insert(name.clone());
        }
        if asl_accepted && !sigil_accepted {
            over_refusal.insert(name.clone());
        }
    }
    (over_acceptance, over_refusal)
}

/// The whole measurement, run once and shared by the tests that read it.
struct Run {
    asl: BTreeMap<String, AslVerdict>,
    sigil: BTreeMap<String, SigilVerdict>,
    ledger: BTreeMap<String, LedgerRow>,
    over_acceptance: BTreeSet<String>,
    over_refusal: BTreeSet<String>,
    probes_processed: usize,
}

fn measure() -> Run {
    let asl = read_asl_verdicts();
    let ledger = read_ledger();
    let probes = probe_paths();
    let mut sigil = BTreeMap::new();
    for (name, path) in &probes {
        sigil.insert(name.clone(), sigil_verdict(path));
    }
    let asl_flat: BTreeMap<String, bool> = asl.iter().map(|(k, v)| (k.clone(), v.accepted)).collect();
    let sigil_flat: BTreeMap<String, bool> = sigil
        .iter()
        .filter_map(|(k, v)| match v {
            SigilVerdict::Accept => Some((k.clone(), true)),
            SigilVerdict::Refuse(_) => Some((k.clone(), false)),
            // A panic is not a verdict, so it is not compared. It is asserted
            // separately, by name, so it cannot vanish into a filter.
            SigilVerdict::Panic(_) => None,
        })
        .collect();
    let (over_acceptance, over_refusal) = divergences(&asl_flat, &sigil_flat);
    Run {
        asl,
        sigil,
        ledger,
        over_acceptance,
        over_refusal,
        probes_processed: probes.len(),
    }
}

fn ledgered(run: &Run, direction: &str) -> BTreeSet<String> {
    run.ledger
        .iter()
        .filter(|(_, r)| r.direction == direction)
        .map(|(n, _)| n.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// THE FEED CONTROL
// ---------------------------------------------------------------------------

/// A working comparison over an empty feed prints "clean". So the count the run
/// processed is asserted against a literal, and the three name sets are asserted
/// to agree, which is what makes a dropped probe a failure rather than a
/// silently smaller population.
#[test]
fn feed_control_every_probe_reached_both_assemblers() {
    let run = measure();
    assert_eq!(
        run.probes_processed, EXPECTED_PROBE_COUNT,
        "the run processed {} probe files, not {EXPECTED_PROBE_COUNT}. A scanner \
         that silently processes fewer files than it should reports a clean \
         divergence set for the same reason a correct one does. If probes were \
         added or removed deliberately, move EXPECTED_PROBE_COUNT and re-mint.",
        run.probes_processed
    );
    assert_eq!(
        run.sigil.len(),
        EXPECTED_PROBE_COUNT,
        "sigil produced {} verdicts for {EXPECTED_PROBE_COUNT} probes",
        run.sigil.len()
    );

    let probe_names: BTreeSet<String> = run.sigil.keys().cloned().collect();
    let table_names: BTreeSet<String> = run.asl.keys().cloned().collect();
    let no_verdict: Vec<&String> = probe_names.difference(&table_names).collect();
    let no_probe: Vec<&String> = table_names.difference(&probe_names).collect();
    assert!(
        no_verdict.is_empty(),
        "probe files with no row in the asl verdict table (they were assembled by \
         sigil and compared against NOTHING): {no_verdict:?}. Re-mint with \
         scripts/mint_over_acceptance_verdicts.sh."
    );
    assert!(
        no_probe.is_empty(),
        "verdict rows with no probe file (the table is stale): {no_probe:?}"
    );
}

/// The other half of the feed: a corpus can be the right SIZE and still measure
/// almost nothing if it has been narrowed to a few refusal classes. This is the
/// floor on how much of asl's refusal catalogue the corpus actually exercises.
#[test]
fn feed_control_the_corpus_still_spans_asl_refusal_classes() {
    let asl = read_asl_verdicts();
    let classes: BTreeSet<&str> = asl
        .values()
        .filter(|v| !v.accepted)
        .map(|v| v.class.as_str())
        .filter(|c| c.starts_with('#'))
        .collect();
    assert!(
        classes.len() >= MIN_ASL_REFUSAL_CLASSES,
        "the corpus covers {} distinct numbered asl refusal classes, below the \
         floor of {MIN_ASL_REFUSAL_CLASSES}. Covered: {classes:?}",
        classes.len()
    );
    let accepted = asl.values().filter(|v| v.accepted).count();
    assert!(
        accepted >= 20,
        "only {accepted} probes are ACCEPTED by asl. Those are the probes that \
         give the over-refusal direction anything to measure; with too few of \
         them a sigil that refused everything would still read green."
    );
}

/// The ledger must not be a way to buy green.
///
/// `no_unledgered_over_acceptance` can always be satisfied by adding a row, and
/// a gate whose only failure mode is fixable by editing a text file beside it is
/// a gate people edit. So the count of probes on which the two assemblers still
/// AGREE is floored: ledgering a new divergence lowers one of these numbers and
/// reds this test, and the floor then has to be moved by hand, in the same diff,
/// where a reviewer sees it beside the row that caused it.
///
/// This is deliberately not a count of the ledger. A ledger count would read
/// identically whether a row was added for a real new divergence or swapped for
/// one that had been retired, and those are not the same event.
#[test]
fn feed_control_the_ledger_is_not_an_escape_hatch() {
    let run = measure();
    let agreed_refusals = run
        .asl
        .iter()
        .filter(|(n, v)| !v.accepted && !run.over_acceptance.contains(*n))
        .filter(|(n, _)| matches!(run.sigil.get(*n), Some(SigilVerdict::Refuse(_))))
        .count();
    let agreed_acceptances = run
        .asl
        .iter()
        .filter(|(n, v)| v.accepted && !run.over_refusal.contains(*n))
        .filter(|(n, _)| matches!(run.sigil.get(*n), Some(SigilVerdict::Accept)))
        .count();
    assert!(
        agreed_refusals >= MIN_AGREED_REFUSALS,
        "only {agreed_refusals} probes are refused by BOTH assemblers, below the \
         floor of {MIN_AGREED_REFUSALS}. Each of those is a live tripwire for \
         sigil becoming looser, so this number falling means the gate now \
         measures less than it did. If a divergence was ledgered on purpose, \
         lower this floor in the same change and say why."
    );
    assert!(
        agreed_acceptances >= MIN_AGREED_ACCEPTANCES,
        "only {agreed_acceptances} probes are accepted by BOTH assemblers, below \
         the floor of {MIN_AGREED_ACCEPTANCES}. Each is a live tripwire for sigil \
         becoming stricter."
    );
}

// ---------------------------------------------------------------------------
// THE POSITIVE CONTROL
// ---------------------------------------------------------------------------

/// Proof that the comparison can come out red, driving the same `divergences`
/// the gate drives.
///
/// Four rows, one per quadrant, so the control also shows the comparison does
/// NOT fire on the two agreeing quadrants: a check that reported every row would
/// be red on correct input, which is its own defect and trains people to weaken
/// it.
#[test]
fn positive_control_the_comparison_can_fire() {
    let asl: BTreeMap<String, bool> = [
        ("both_accept".to_string(), true),
        ("both_refuse".to_string(), false),
        ("planted_over_acceptance".to_string(), false),
        ("planted_over_refusal".to_string(), true),
    ]
    .into_iter()
    .collect();
    let sigil: BTreeMap<String, bool> = [
        ("both_accept".to_string(), true),
        ("both_refuse".to_string(), false),
        ("planted_over_acceptance".to_string(), true),
        ("planted_over_refusal".to_string(), false),
    ]
    .into_iter()
    .collect();

    let (over_acceptance, over_refusal) = divergences(&asl, &sigil);
    assert_eq!(
        over_acceptance,
        BTreeSet::from(["planted_over_acceptance".to_string()]),
        "the over-acceptance comparison did not report a planted divergence"
    );
    assert_eq!(
        over_refusal,
        BTreeSet::from(["planted_over_refusal".to_string()]),
        "the over-refusal comparison did not report a planted divergence"
    );

    // And the set difference against a ledger, which is the assertion the gate
    // makes rather than the raw sets.
    let ledger: BTreeSet<String> = BTreeSet::from(["something_else".to_string()]);
    let unledgered: BTreeSet<String> = over_acceptance.difference(&ledger).cloned().collect();
    assert_eq!(
        unledgered,
        BTreeSet::from(["planted_over_acceptance".to_string()]),
        "a ledger naming an unrelated probe swallowed a real divergence"
    );
}

// ---------------------------------------------------------------------------
// THE GATE
// ---------------------------------------------------------------------------

/// The hard gate. A shape sigil accepts and asl refuses, that nobody has written
/// down, is a regression.
#[test]
fn no_unledgered_over_acceptance() {
    let run = measure();
    let known = ledgered(&run, "over-acceptance");
    let new: Vec<String> = run
        .over_acceptance
        .difference(&known)
        .map(|n| {
            let class = run.asl.get(n).map(|v| v.class.as_str()).unwrap_or("?");
            format!("  {n}\n    asl refuses: {class}\n    sigil accepts it")
        })
        .collect();
    assert!(
        new.is_empty(),
        "sigil ACCEPTS {} program(s) asl REFUSES that are not in the ledger:\n{}\n\n\
         This is the direction no other gate in the repo can see. Either tighten \
         sigil to refuse them, or add a row to \
         crates/sigil-frontend-as/tests/over_acceptance/ledger.txt saying why the \
         divergence is tolerated. Adding a row is a decision and it is supposed to \
         cost a sentence.",
        new.len(),
        new.join("\n")
    );
}

/// The mirror. A program asl accepts and sigil refuses is the direction the
/// corpus gates already cover for constructs the corpora contain, and cover for
/// nothing else.
#[test]
fn no_unledgered_over_refusal() {
    let run = measure();
    let known = ledgered(&run, "over-refusal");
    let new: Vec<String> = run
        .over_refusal
        .difference(&known)
        .map(|n| {
            let why = match run.sigil.get(n) {
                Some(SigilVerdict::Refuse(d)) => d.clone(),
                other => format!("{other:?}"),
            };
            format!("  {n}\n    asl accepts it\n    sigil refuses: {why}")
        })
        .collect();
    assert!(
        new.is_empty(),
        "sigil REFUSES {} program(s) asl ACCEPTS that are not in the ledger:\n{}\n\n\
         Either accept them, or ledger the divergence with its reason.",
        new.len(),
        new.join("\n")
    );
}

/// A panic is neither verdict, and a crash must not be able to read as agreement.
#[test]
fn no_probe_panics_sigil() {
    let run = measure();
    let panics: Vec<String> = run
        .sigil
        .iter()
        .filter_map(|(n, v)| match v {
            SigilVerdict::Panic(m) => Some(format!("  {n}: {m}")),
            _ => None,
        })
        .collect();
    assert!(
        panics.is_empty(),
        "sigil panicked on {} probe(s):\n{}",
        panics.len(),
        panics.join("\n")
    );
}

/// `ledger \ observed`, computed and REPORTED and not asserted.
///
/// A ledger row that has started agreeing with asl is good news. Reddening on it
/// would make the arrival of good news look like a defect and would train the
/// next reader to delete rows to get green, which is exactly how a ledger stops
/// describing anything.
///
/// What IS asserted is that every ledger row names a probe that exists. A row
/// pointing at a deleted probe is dead weight that can never be retired by
/// observation, because the observation that would retire it can never be made.
#[test]
fn ledger_rows_that_have_started_agreeing_are_reported() {
    let run = measure();

    let probe_names: BTreeSet<String> = run.sigil.keys().cloned().collect();
    let orphans: Vec<&String> = run
        .ledger
        .keys()
        .filter(|n| !probe_names.contains(*n))
        .collect();
    assert!(
        orphans.is_empty(),
        "ledger rows naming probes that do not exist: {orphans:?}. Such a row can \
         never be retired by observation, because the observation is impossible."
    );

    let mut stale = Vec::new();
    for (name, row) in &run.ledger {
        let observed = match row.direction.as_str() {
            "over-acceptance" => run.over_acceptance.contains(name),
            "over-refusal" => run.over_refusal.contains(name),
            _ => unreachable!("direction validated at read time"),
        };
        if !observed {
            stale.push(format!(
                "  {name} ({}) now AGREES with asl. Reason on file: {}",
                row.direction, row.reason
            ));
        }
    }
    if stale.is_empty() {
        println!("ledger: every row still diverges; nothing to retire.");
    } else {
        println!(
            "ledger: {} row(s) have started agreeing with asl and can be retired \
             from over_acceptance/ledger.txt:\n{}",
            stale.len(),
            stale.join("\n")
        );
    }
}

/// The measurement itself, printed. NOT AN ASSERTION, and it has no red-first
/// proof because it has no red: do not read its green as a measurement of
/// anything. It exists so the numbers in the module doc can be checked against a
/// run rather than taken on the doc's word, and so a reader has the full picture
/// beside whichever gate above went red.
#[test]
fn report_the_divergence_sets() {
    let run = measure();
    let asl_refuses = run.asl.values().filter(|v| !v.accepted).count();
    let asl_accepts = run.asl.len() - asl_refuses;
    println!(
        "probes {} | asl refuses {asl_refuses}, accepts {asl_accepts}",
        run.probes_processed
    );
    println!("over-acceptance ({}):", run.over_acceptance.len());
    for n in &run.over_acceptance {
        let led = if run.ledger.contains_key(n) { "ledgered" } else { "NEW" };
        println!("  [{led}] {n} -- asl: {}", run.asl[n].class);
    }
    println!("over-refusal ({}):", run.over_refusal.len());
    for n in &run.over_refusal {
        let led = if run.ledger.contains_key(n) { "ledgered" } else { "NEW" };
        println!("  [{led}] {n}");
    }
}
