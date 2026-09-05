//! THE WARN TIER over the real aeon corpus.
//!
//! `sigil build` prints a one-line tally of every warn-tier lint that fires. This
//! gate is the machine that watches the same thing when nobody is reading the build
//! log: the SET of firing lint ids per shape must equal [`WARN_ID_BASELINE`], so a
//! newly-merged lint cannot fire across the whole corpus and leave no trace.
//!
//! WHY IDS AND NOT COUNTS. The house baseline shape (`out_verify_corpus`'s
//! `D1C_BASELINE`) pins identified rows, and the natural analogue here would be a
//! per-`(shape, id)` count. It is deliberately NOT pinned, for two reasons:
//!   - The counts are SHAPE-DEPENDENT and move on ordinary engine work — a
//!     comptime-gated proc entering or leaving a shape moves them. A baseline
//!     that churns on unrelated work gets rubber-stamped, and a rubber-stamped
//!     gate asserts nothing.
//!   - The id SET is shape-INVARIANT and moves only when a lint starts or stops
//!     firing on the corpus — which is exactly the event that must never pass
//!     unnoticed. It also has teeth in the retirement direction: clearing a class
//!     fails this gate until the win is recorded here.
//!
//! The limitation is honest and stated: the id gate does NOT catch growth WITHIN an
//! already-firing class. [`CORPUS_OPEN_FINDINGS`] is where a class stops having that
//! hiding place — it pins `(shape, id, file, symbol)` with a count, so a registered id
//! firing anywhere else, or one more time, fails. Ids not in the register still rely on
//! the build's tally line.
//!
//! WHERE A NEW FIRING GOES. Into the register, with an owner, a tracking anchor and a
//! kill condition — not into [`WARN_ID_BASELINE`], which admits an id corpus-wide.
//! Widening the baseline to quiet a firing is the move that made this whole file
//! necessary.
//!
//! Reference tree: defaults to the sibling aeon checkout (override with `AEON_DIR`);
//! under `SIGIL_STRICT_GATE` a missing tree HARD-FAILS, the house pattern.

use sigil_harness::native;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The warn-tier lint classes the aeon corpus fires. Named once so retiring a
/// class — or admitting a new one — is the ONE-LINE diff it should be, which is the
/// most conspicuous form this baseline's escape hatch can take.
///
/// Every id here is a KNOWN-OPEN class, triaged in
/// `docs/superpowers/notes/2026-08-04-warning-tier.md` — which also records which
/// firings are true positives and which are lint gaps. Listing an id says "known",
/// not "fine".
const CORPUS_LINTS: &[&str] = &[
    // ADJUDICATED 2026-08-14 (lens sweep, seat COMPTIME, finding S21). Fires on
    // every shipped shape, 14 modules / 53 guards on sonic4 plain. It is NOT a
    // regression and NOT noise to be silenced: an item-position `ensure` is
    // evaluated iff its module is in the profile's `use` closure, so these guards
    // are structurally incapable of firing for this target. Most of the 14 are the
    // Z80 sound modules, which DO evaluate theirs via seam-1/seam-2. The ones that
    // do not are the finding: `engine.z80_init` (whose
    // `ensure(extern("Z80_IDLE_SIZE") == 40)` neither shipped sonic4 shape
    // evaluates or even defers), `engine.debug.sound_debug`,
    // `engine.compression_selftest`, and `games.demo.constants`. Retiring those is
    // engine work; this row is the standing record that they are unevaluated.
    "module.unreachable",
    "module.path-mismatch",
    "proc.clobber-undeclared",
    "proc.out-unwritten",
    "proc.undeclared-fallthrough",
];

/// The firing set per build shape, spelled as [`CORPUS_LINTS`] plus whatever that
/// shape adds. Every SHIPPED shape has its own row, so a lint that fires in one
/// shape only cannot hide behind a shape nobody watches — and because the shared
/// ids live in ONE list, retiring a class corpus-wide stays the one-line diff it
/// should be while a shape-only class is visible as its own entry.
///
/// NO shape adds anything today. `proc.sr-undeclared` in particular: the
/// `assert` desugar's SR restore is `AssertDesugar`-authored and exempt (its
/// balance is pinned at the emission site, `diag_desugar.rs`;
/// [`debug_shape_sr_writes_are_author_checked`] holds the exemption's ground
/// here), so a `DEBUG == 1` row is empty — which gives this gate teeth in its
/// favorite direction: the FIRST hand-written undeclared SR write in any
/// debug-gated proc makes `proc.sr-undeclared` APPEAR in that row and fails
/// the id-set gate loudly, with no compiler-emitted crowd to hide in.
/// (History and the measured retirement numbers: campaign-gap-ledger.md and
/// notes/2026-08-04-warning-tier.md.)
const WARN_ID_BASELINE: &[(&str, &[&str])] = &[
    // ADJUDICATED 2026-08-18 (scanline-P1): `import.no-names` fires TWICE on sonic4, once
    // per hand-written closure edge — `use games.sonic4.scene_registry` in ojz_effects.emp
    // and `use games.sonic4.scene_equiv_proof` in ojz_scroll_test.emp. Both are DELIBERATE
    // and neither can be spelled another way: a name list does not create the closure edge
    // (measured — the registry's capability ensure and all of scene_dsl.emp behind it go
    // dark without the bare use), and a glob on the witness would re-evaluate its twenty
    // EQ_* consts in the consumer's scope. Demo authors no scenes, so its rows stay empty
    // and remain a live control on this id.
    // OPEN, booked in aeon docs/DEFERRED_WORK.md: the lint cannot express "this bare use is
    // a closure edge", so a real accidental bare use in sonic4 now hides behind these two.
    // The fix is a spelling for the idiom, not a wider baseline.
    ("sonic4 plain", &["import.no-names"]),
    ("sonic4 debug", &["import.no-names"]),
    ("demo plain", &[]),
    ("demo debug", &[]),
    // Same two closure edges — config_a/config_b/lean are sonic4 profiles and carry the
    // same files. Added together and confirmed by re-running: the gate is bidirectional,
    // so a shape that did NOT fire would have failed as "NO LONGER FIRING".
    ("config_a", &["import.no-names"]),
    ("config_b", &["import.no-names"]),
    ("lean", &["import.no-names"]),
];

/// One unadjudicated warn-tier firing, pinned SITE-for-SITE.
///
/// WHY THIS EXISTS ALONGSIDE [`WARN_ID_BASELINE`] AND NOT INSIDE IT. The id baseline
/// answers "may this lint fire at all on this shape", which is deliberately blunt —
/// this file's own header states it "does NOT catch growth WITHIN an already-firing
/// class". So an id admitted there is admitted EVERYWHERE, any number of times, and a
/// genuinely new instance of the same class lands in the crowd its predecessor made.
/// This register answers the sharper question: exactly WHICH sites, and how MANY
/// times each. A firing of a registered id anywhere else, or a different number of
/// times, is a failure.
///
/// It is therefore the place an unadjudicated firing goes INSTEAD of the id baseline.
/// Widening the baseline to silence a firing is the failure mode this whole gate was
/// installed against; a row here silences nothing — it names the finding, its owner,
/// and what would remove it, and it goes red the moment the finding changes shape.
///
/// EVERY FIELD IS REQUIRED BY THE TYPE. An entry with no owner, no anchor or no kill
/// condition is a dumping ground with extra steps, so those are not `Option`.
///
/// LEAVING THE REGISTER GOES BOTH WAYS. A row is removed when the firing is fixed, and
/// equally when it is RULED DELIBERATE — in which case the id moves into
/// [`WARN_ID_BASELINE`] with the ruling cited in the same commit. Either way the gate
/// fails until the register is updated, so neither exit can happen silently.
struct OpenFinding {
    /// The lint id, as the diagnostic carries it.
    id: &'static str,
    /// The source file, relative to the aeon tree root. Derived from the diagnostic's
    /// own `location` field, never scraped out of the message.
    file: &'static str,
    /// A SYMBOL the message names — a module path, a field, a proc. Symbolic on
    /// purpose: a register entry outlives the thing it points at, which is its whole
    /// job, so a line number or a byte offset in one rots into a confident lie. The
    /// message must name this symbol for a firing to count as this site.
    detail: &'static str,
    /// `(shape, occurrences)` — MEASURED, not typed from a design document. The count
    /// is carried so that a second firing at the same site is red too: set semantics
    /// would collapse it into the first and pass, reintroducing within one site the
    /// growth-blindness this register exists to fix across sites.
    sites: &'static [(&'static str, usize)],
    /// Who adjudicates it. Not "someone".
    owner: &'static str,
    /// Where the adjudication is tracked. A path or a symbol, never a line number.
    anchor: &'static str,
    /// What removes this row.
    kill: &'static str,
    /// `YYYY-MM-DD`, taken from the repo's own record of the adjudication. The lane
    /// renders each row's AGE from this, so an entry left open for months is
    /// self-evident in ordinary green output rather than resting quietly in a file
    /// nobody is obliged to open.
    first_observed: &'static str,
}

/// The open findings. Empty is a legitimate state and means every warn-tier firing on
/// the corpus is accounted for by [`CORPUS_LINTS`] and [`WARN_ID_BASELINE`].
const CORPUS_OPEN_FINDINGS: &[OpenFinding] = &[
    // The two hand-written closure edges. Both are deliberate: a whole-path `use` pulls
    // a zero-emitting guard/witness module into a profile's use closure, and the lint
    // cannot tell that from an accidental bare use, which is why `import.no-names` sits
    // in WARN_ID_BASELINE's shape rows. The explicit spelling for the idiom EXISTS —
    // `use base._` binds no names and the lint is silent on it — so what these rows now
    // track is the ADOPTION, which is aeon's to make in aeon's tree. That row's own comment names the cost: "a real
    // accidental bare use in sonic4 now hides behind these two". These entries take
    // that cost back. A third bare use anywhere in the corpus, or a second one in
    // either of these files, now fails.
    OpenFinding {
        id: "import.no-names",
        file: "games/sonic4/data/effects/ojz_effects.emp",
        detail: "games.sonic4.scene_registry",
        sites: &[
            ("sonic4 plain", 1),
            ("sonic4 debug", 1),
            ("config_a", 1),
            ("config_b", 1),
            ("lean", 1),
        ],
        owner: "sigil language lane (the spelling); aeon adopts it",
        anchor: "aeon docs/DEFERRED_WORK.md, the `[import.no-names]` closure-edge gap",
        kill: "the spelling exists (`use base._`); when this `use` line adopts it the lint \
               stops firing here, at which point this row goes red and is deleted. Widening \
               the baseline instead is the move this register replaces.",
        first_observed: "2026-08-18",
    },
    OpenFinding {
        id: "import.no-names",
        file: "games/sonic4/test/ojz_scroll_test.emp",
        detail: "games.sonic4.scene_equiv_proof",
        sites: &[
            ("sonic4 plain", 1),
            ("sonic4 debug", 1),
            ("config_a", 1),
            ("config_b", 1),
            ("lean", 1),
        ],
        owner: "sigil language lane (the spelling); aeon adopts it",
        anchor: "aeon docs/DEFERRED_WORK.md, the `[import.no-names]` closure-edge gap",
        kill: "the spelling exists (`use base._`); when this `use` line adopts it the lint \
               stops firing here, at which point this row goes red and is deleted. Widening \
               the baseline instead is the move this register replaces.",
        first_observed: "2026-08-18",
    },
];

/// Days since 1970-01-01 for a `YYYY-MM-DD` date, or `None` if it does not parse.
///
/// Proleptic Gregorian, era-based (Howard Hinnant's `days_from_civil`). Written out
/// rather than pulled in so the register's age line costs the workspace no dependency.
fn days_from_civil(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Today, in days since the epoch, from the system clock.
fn today_days() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before the epoch")
        .as_secs() as i64;
    secs / 86_400
}

/// The reference tree, or `None` when it is absent and strict mode is off.
fn aeon_dir() -> Option<PathBuf> {
    let aeon = sigil_harness::test_support::aeon_dir();
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return None;
    }
    Some(aeon)
}

/// One shape's label and the warn tier its lowering produced.
type ShapeWarnings = (&'static str, Vec<native::BuildWarning>);

/// `(label, warnings)` for each of the SEVEN shipped shapes, lowered ONCE per test
/// binary — the gates below share one measurement rather than each paying for its
/// own lowering.
///
/// All seven are watched, not a canonical subset: the shapes differ by comptime
/// defines that DO gate lint-bearing code, and the divergence can reach the ID
/// SET this gate pins — a lint whose triggers live only in `DEBUG == 1` code
/// fires in exactly three shapes, so a shape-only lint is a real escape route.
/// (Shapes also diverge below the id set, which this gate deliberately does
/// not watch: `lean` fires one fewer `[proc.undeclared-fallthrough]` than the
/// canonical shapes because it drops `ErrorHandlerBlob`, and the class stays
/// in its row.)
fn corpus_warnings() -> Option<&'static [ShapeWarnings]> {
    static CACHE: OnceLock<Option<Vec<ShapeWarnings>>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let aeon = aeon_dir()?;
            // The `.emp` corpus embeds the seam-emitted sound blobs, so they must
            // exist before any shape lowers.
            native::ensure_generated(&aeon);
            Some(
                native::shipped_shapes()
                    .into_iter()
                    .map(|(label, profile)| {
                        let built = native::build_emp(&aeon, &profile)
                            .unwrap_or_else(|e| panic!("build_emp({label}): {e}"));
                        (label, built.warnings)
                    })
                    .collect(),
            )
        })
        .as_deref()
}

/// The firing id set per shape must equal [`WARN_ID_BASELINE`] exactly.
#[test]
fn warn_tier_lint_ids_match_the_frozen_baseline() {
    let Some(shapes) = corpus_warnings() else { return };

    for (label, warnings) in shapes {
        let got: BTreeSet<&str> = warnings.iter().map(|w| w.id.as_str()).collect();
        let extra = WARN_ID_BASELINE
            .iter()
            .find(|(l, _)| l == label)
            .unwrap_or_else(|| panic!("no baseline row for shape `{label}`"))
            .1;
        let want: BTreeSet<&str> = CORPUS_LINTS.iter().chain(extra).copied().collect();

        let appeared: Vec<_> = got.difference(&want).collect();
        let vanished: Vec<_> = want.difference(&got).collect();
        assert!(
            appeared.is_empty() && vanished.is_empty(),
            "the warn-tier lint id set moved for `{label}`.\n  \
             NEWLY FIRING (a lint fires on the corpus and nobody decided that): {appeared:?}\n  \
             NO LONGER FIRING (a class was retired, or a lint lost its teeth): {vanished:?}\n  \
             Adjudicate each id and update WARN_ID_BASELINE in the same commit."
        );
    }
}

/// Every [`CORPUS_OPEN_FINDINGS`] row is complete and its date parses.
///
/// The type already makes owner / anchor / kill non-`Option`; this is the other half —
/// a field cannot be discharged by typing `""` into it. A register whose rows can be
/// blank is the dumping ground it was built to replace.
#[test]
fn every_open_finding_is_completely_specified() {
    for f in CORPUS_OPEN_FINDINGS {
        let missing: Vec<&str> = [
            ("id", f.id),
            ("file", f.file),
            ("detail", f.detail),
            ("owner", f.owner),
            ("anchor", f.anchor),
            ("kill", f.kill),
            ("first_observed", f.first_observed),
        ]
        .iter()
        .filter(|(_, v)| v.trim().is_empty())
        .map(|(k, _)| *k)
        .collect();
        assert!(missing.is_empty(), "open finding `{}` @ {}: blank {missing:?}", f.id, f.file);
        assert!(
            !f.sites.is_empty(),
            "open finding `{}` @ {}: no shape sites, an entry that pins nothing gates nothing",
            f.id,
            f.file
        );
        assert!(
            f.sites.iter().all(|(_, n)| *n > 0),
            "open finding `{}` @ {}: a site with zero occurrences is a row that should have \
             been deleted, not pinned",
            f.id,
            f.file
        );
        assert!(
            days_from_civil(f.first_observed).is_some(),
            "open finding `{}` @ {}: `first_observed` is not a YYYY-MM-DD date: {:?}",
            f.id,
            f.file,
            f.first_observed
        );
        // The anchor names a document or a symbol. A line or column in one rots into a
        // confident lie the moment the file it points at is edited, which is the exact
        // defect class this register was built after.
        assert!(
            !f.anchor.contains(".md:") && !f.anchor.contains(".emp:") && !f.anchor.contains(".rs:"),
            "open finding `{}` @ {}: the anchor carries a line number ({:?}). Anchor to a \
             symbol or a path and let a SHA date the claim, an anchor outlives what it \
             points at, so a coordinate in one is guaranteed to go stale.",
            f.id,
            f.file,
            f.anchor
        );
    }
}

/// THE OPEN-FINDINGS GATE: every registered site fires exactly as registered, and a
/// registered id fires NOWHERE else.
///
/// Four ways to be red, and all four are the point:
///   - a registered id fires at an UNREGISTERED site (new file, or a symbol no row
///     names) — the property that makes this strictly stronger than a baseline entry,
///     which tolerates the id anywhere;
///   - a registered site fires MORE times than pinned — growth within one site;
///   - a registered site fires FEWER times than pinned;
///   - a registered site stops firing entirely — the finding was resolved, or the file
///     left the shape's closure, and the register must move in the same commit.
///
/// The register is RENDERED on every run with each row's age, so the lane's ordinary
/// green output names its open items instead of leaving them to a file nobody is
/// obliged to open. Invisibility was the failure this whole parcel is about; a green
/// that says "two open, 4 days" is a different object from a silent green.
#[test]
fn corpus_open_findings_fire_exactly_where_registered() {
    let Some(aeon) = aeon_dir() else { return };
    let Some(shapes) = corpus_warnings() else { return };
    assert!(!shapes.is_empty(), "no shape was lowered, measure before trusting");

    // The rendered register, printed whether or not anything is wrong.
    let today = today_days();
    eprintln!("open warn-tier findings: {}", CORPUS_OPEN_FINDINGS.len());
    for f in CORPUS_OPEN_FINDINGS {
        let age = days_from_civil(f.first_observed)
            .map(|d| today - d)
            .expect("first_observed parses; every_open_finding_is_completely_specified holds it");
        let leaf = f.file.rsplit('/').next().unwrap_or(f.file);
        let stale = if age >= 30 { "  <-- open a month or more" } else { "" };
        eprintln!(
            "  {} · {leaf} · {} · owner: {} · open {age} days{stale}",
            f.id, f.detail, f.owner
        );
    }

    let registered_ids: BTreeSet<&str> = CORPUS_OPEN_FINDINGS.iter().map(|f| f.id).collect();

    // What the register claims, keyed `(shape, id, file, detail)`.
    let mut want: std::collections::BTreeMap<(&str, &str, &str, &str), usize> = Default::default();
    for f in CORPUS_OPEN_FINDINGS {
        for (shape, n) in f.sites {
            want.insert((shape, f.id, f.file, f.detail), *n);
        }
    }

    let mut got: std::collections::BTreeMap<(&str, &str, &str, &str), usize> = Default::default();
    let mut unregistered: Vec<String> = Vec::new();
    let root = format!("{}/", aeon.display());
    for (label, warnings) in shapes {
        for w in warnings.iter().filter(|w| registered_ids.contains(w.id.as_str())) {
            // The file comes from the diagnostic's own `location`, with the `:line:col`
            // suffix dropped: the file is the stable half of a coordinate and the line
            // is the half that moves on every edit above it.
            let loc = w.location.as_deref().unwrap_or("<no location>");
            let file = loc.rsplitn(3, ':').last().unwrap_or(loc);
            let rel = file.strip_prefix(&root).unwrap_or(file);
            // A row claims a firing when it names the same file AND the message names
            // its symbol. Matching on the symbol keeps two distinct findings in one
            // file distinguishable without pinning a coordinate.
            let hit = CORPUS_OPEN_FINDINGS.iter().find(|f| {
                f.id == w.id
                    && f.file == rel
                    // `detail` is documented as "a SYMBOL the message names", and this
                    // matched it as "`use <detail>`" — the exact phrasing of
                    // `import.no-names`, the only id that had ever been registered. That
                    // coupled the register to one lint's wording: a legitimate row for any
                    // other id could not be claimed, and the failure presented as the ROW
                    // being wrong rather than the matcher being narrow. Generalised
                    // 2026-09-02 to the field's actual contract. Precision is unaffected in
                    // practice: (id, file, shape) already narrows a firing to one lint in
                    // one file, and `detail` only has to separate sites WITHIN that.
                    && w.message.contains(f.detail)
                    && f.sites.iter().any(|(s, _)| s == label)
            });
            match hit {
                Some(f) => *got.entry((label, f.id, f.file, f.detail)).or_default() += 1,
                None => unregistered.push(format!("[{label}] {rel} :: {}", w.message)),
            }
        }
    }

    assert!(
        unregistered.is_empty(),
        "a registered lint id fired at {} site(s) NO open-findings row claims. The id is \
         admitted in WARN_ID_BASELINE, so the id-set gate stays green and this is the only \
         thing that sees it, which is the whole reason the register is site-pinned. \
         Adjudicate each and either fix it or add a CORPUS_OPEN_FINDINGS row (owner, anchor \
         and kill condition included) in the same commit:\n{}",
        unregistered.len(),
        unregistered.join("\n")
    );

    let mut drift: Vec<String> = Vec::new();
    for (key, n) in &want {
        let actual = got.get(key).copied().unwrap_or(0);
        if actual != *n {
            let (shape, id, file, detail) = key;
            drift.push(if actual == 0 {
                format!(
                    "  RESOLVED: [{shape}] {id} @ {file} ({detail}) no longer fires (was {n}). \
                     Delete the row, or, if it was ruled DELIBERATE, move the id into \
                     WARN_ID_BASELINE citing the ruling. Same commit either way."
                )
            } else {
                format!("  COUNT MOVED: [{shape}] {id} @ {file} ({detail}): pinned {n}, measured {actual}")
            });
        }
    }
    assert!(
        drift.is_empty(),
        "the open-findings register no longer describes the corpus:\n{}",
        drift.join("\n")
    );
}

/// Every SR write in a `DEBUG == 1` shape's item streams carries an author with
/// a RECEIVING obligation — asserted TYPED, off the compiler's own
/// classification (`ContractReport::sr_writes`), never by re-reading source
/// text behind a diagnostic's location.
///
/// The exemption ledger this asserts, author by author:
///   - `AssertDesugar` — exempt from `[proc.sr-undeclared]`; the balance proof
///     lives at the emission site (`diag_desugar.rs`'s
///     `the_assert_expansion_is_desugar_authored_and_sr_balanced`).
///   - `Context` — exempt; the round-trip proof lives at the context
///     DEFINITION (`lower_with`'s once-per-context check).
///   - `User` / `Splice` — NOT exempt: the lint charges the containing proc,
///     and the id-set gate above pins every row to zero surviving firings, so
///     the first undeclared one fails that gate with its id named.
///   - `EntrySynth` — must not appear: the synthesis emits no instructions
///     (pinned in `sigil-harness`), so an EntrySynth SR write would be an
///     authored effect with NO obligation home yet — the one defect class the
///     author field must not admit.
///
/// A future `ItemAuthor` variant fails the match at compile time, so a new
/// author cannot ship without declaring where its obligation lands.
///
/// NON-VACUITY: desugar-authored SR writes must be SEEN (`seen > 0`) — the
/// DEBUG shapes compile asserts today, so a walk that stops reaching them
/// cannot silently pass.
#[test]
fn debug_shape_sr_writes_are_author_checked() {
    use sigil_frontend_emp::corpus_contracts::{
        analyze_corpus_with_contracts, bind_corpus_interfaces,
    };
    use sigil_frontend_emp::value::ItemAuthor;
    let Some(aeon) = aeon_dir() else { return };

    // The corpus source walk — the house per-file pattern (`slot_type_corpus`).
    fn emp_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
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
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    let files: Vec<sigil_frontend_emp::ast::File> = paths
        .iter()
        .map(|p| {
            let s = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
            let (f, d) = sigil_frontend_emp::parse_str(&s);
            assert!(
                d.iter().all(|x| x.level != sigil_span::Level::Error),
                "{} parse errors: {d:?}",
                p.display()
            );
            f
        })
        .collect();

    let mut seen_desugar = 0usize;
    let mut seen_context = 0usize;
    let mut walked_debug_shapes = 0usize;
    let aeon = aeon_dir().expect("aeon tree present");
    for (label, profile) in native::shipped_shapes() {
        let defines =
            native::shape_defines(&profile, &aeon).expect("shape defines");
        if !defines.iter().any(|(k, v)| k == "DEBUG" && *v == 1) {
            continue;
        }
        walked_debug_shapes += 1;
        let (iface_env, bind_diags) =
            bind_corpus_interfaces(&files, &defines, profile.game_module_prefix());
        assert!(
            bind_diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "shape `{label}`: L1 bind errors: {bind_diags:?}"
        );
        let r = analyze_corpus_with_contracts(&files, &defines, &iface_env);
        assert!(!r.sr_writes.is_empty(), "`{label}`: no SR write examined, measure before trusting");
        for (proc, author, _span) in &r.sr_writes {
            match author {
                ItemAuthor::AssertDesugar => seen_desugar += 1,
                ItemAuthor::Context { .. } => seen_context += 1,
                // Charged by the lint; the id-set baseline pins zero firings.
                // `IrqFrame` (bookmark ask 3) authors an `irq_frame.pc` MEMORY
                // access, never an SR write, so it is treated like `User` here (the
                // corpus has none today; the arm keeps the match exhaustive).
                ItemAuthor::User | ItemAuthor::Splice { .. } | ItemAuthor::IrqFrame => {}
                ItemAuthor::EntrySynth => panic!(
                    "`{label}`: an EntrySynth-authored SR write in `{proc}`, the entry \
                     synthesis emits no instructions today, and no obligation home exists \
                     for one that writes SR; build the receiving contract before shipping it"
                ),
            }
        }
    }
    assert_eq!(walked_debug_shapes, 3, "the three DEBUG == 1 shapes are the class's home");
    assert!(seen_desugar > 0, "no desugar-authored SR write was seen, the exemption is vacuous");
    // Context-authored SR traffic is corpus-real too (`ints_off` brackets); its
    // count is reported, not pinned (adoption moves it).
    eprintln!("sr-write census: desugar-authored {seen_desugar}, context-authored {seen_context}");
}

/// Every warn-tier diagnostic the corpus fires carries a `[area.name]` id.
///
/// NOT VACUOUS: the id is what makes the tally line legible and what
/// [`warn_tier_lint_ids_match_the_frozen_baseline`] keys on. A lint that ships
/// without one tallies into a single meaningless `unclassified` bucket and is
/// invisible to that gate, so it has to fail here instead.
#[test]
fn every_corpus_warning_carries_a_lint_id() {
    let Some(shapes) = corpus_warnings() else { return };

    for (label, warnings) in shapes {
        assert!(!warnings.is_empty(), "`{label}` reported no warnings, measure before trusting");
        let bare: Vec<&str> =
            warnings.iter().filter(|w| w.id.is_empty()).map(|w| w.message.as_str()).collect();
        assert!(
            bare.is_empty(),
            "`{label}`: {} warn-tier diagnostic(s) carry no `[area.name]` id, so they tally as \
             `unclassified` and the baseline gate cannot key on them: {bare:#?}",
            bare.len()
        );
    }
}

/// The build reports on code its reader can EDIT.
///
/// `build_emp` writes a synthetic entry module to drive the reachability BFS, whose
/// every line is a bare `use <module>` — the exact shape `[import.no-names]` warns
/// about. Reporting those would put unactionable rows against a file that does not
/// exist on disk.
///
/// PREMISE CORRECTED 2026-08-18 (scanline-P1). This test used to filter on the lint id
/// alone, on the stated grounds that "the corpus hand-writes no bare `use` statement, so
/// an `[import.no-names]` firing can only come from the generated module". THAT IS NO
/// LONGER TRUE: a bare whole-path `use` is the CLOSURE-EDGE idiom — the only way to pull a
/// zero-emitting module (a guard/witness module, which can never take a registry row) into
/// a profile's use closure. `ojz_effects.emp` and `ojz_scroll_test.emp` each hand-write one.
/// So the discriminator is now the LOCATION, not the id: the generated entry is not a file
/// on disk and its diagnostics carry `location: None`, while a hand-written one names a file
/// the reader can open. Still not vacuous in either direction — a leaked entry diagnostic
/// has no location and is caught here; the lint's teeth against real code are proven in
/// `module_resolution.rs`, so a green here cannot mean the lint is dead.
#[test]
fn the_generated_entry_module_is_not_reported() {
    let Some(shapes) = corpus_warnings() else { return };

    for (label, warnings) in shapes {
        let generated: Vec<&str> = warnings
            .iter()
            .filter(|w| w.id == "import.no-names" && w.location.is_none())
            .map(|w| w.message.as_str())
            .collect();
        assert!(
            generated.is_empty(),
            "`{label}`: the synthetic entry's own diagnostics reached the build report: \
             {generated:#?}"
        );
        // Every reported warning names a file the reader can open.
        let unlocated: Vec<&str> =
            warnings.iter().filter(|w| w.location.is_none()).map(|w| w.message.as_str()).collect();
        assert!(
            unlocated.is_empty(),
            "`{label}`: {} reported warning(s) carry no source location: {unlocated:#?}",
            unlocated.len()
        );
    }
}

/// THE SURFACE IS WIRED: `sigil build` prints the tally on stderr, and
/// `SIGIL_WARNINGS=off` silences it.
///
/// NOT VACUOUS, and nothing else covers this: every other gate here calls
/// `build_emp` and reads `EmpProgram::warnings` directly, so all of them stay green
/// if the `report_warnings` call in `run_build_native` is deleted. This drives the
/// real binary and reads its real stderr, which is the only thing that can tell the
/// difference between a collected warn tier and a REPORTED one.
///
/// One `--report ram` invocation per view rather than a full ROM build: it runs the
/// same `report_warnings` under the same `WarningView`, and costs seconds instead of
/// minutes. Both report kinds share one prologue, so this covers `contracts` too.
#[test]
fn the_build_binary_prints_the_tally_and_off_silences_it() {
    let Some(aeon) = aeon_dir() else { return };

    let run = |view: Option<&str>| -> String {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"));
        cmd.args(["build", "--aeon", aeon.to_str().unwrap(), "--native", "--report", "ram"]);
        match view {
            Some(v) => cmd.env("SIGIL_WARNINGS", v),
            None => cmd.env_remove("SIGIL_WARNINGS"),
        };
        let out = cmd.output().expect("run sigil build");
        assert!(out.status.success(), "sigil build --report ram failed: {out:?}");
        String::from_utf8_lossy(&out.stderr).into_owned()
    };

    // A report shows the SAME warn tier the build shows over the same tree — the
    // manifest scan's `[module.path-mismatch]` family included, which no later stage
    // re-reports. A report that renders a cleaner tree than the build is a report
    // nobody can trust, so the DEFAULT view carries the tally here too.
    let default = run(None);
    assert!(
        default.contains("warning: ") && default.contains("module.path-mismatch"),
        "a report must show the manifest scan's own warn tier, got: {default:?}"
    );
    assert!(
        !run(Some("off")).contains("warning: "),
        "`SIGIL_WARNINGS=off` must silence the tier on the real corpus too"
    );

    // And the printer IS reachable from this path: a `--report ram` over a tree with
    // no RAM-region module warns, and the tally names the class.
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("engine")).unwrap();
    std::fs::write(tmp.path().join("engine/ram.emp"), "module engine.ram\n").unwrap();
    // A synthetic tree still owes the game config `shape_defines` reads: a
    // missing `games/<g>/map.toml` is a MISSING config and fails loud, so this
    // fixture supplies an empty one — no `[defines]` table, hence no game rows,
    // which is the state every shipped map is in.
    std::fs::create_dir_all(tmp.path().join("games/sonic4")).unwrap();
    std::fs::write(tmp.path().join("games/sonic4/map.toml"), "").unwrap();
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--aeon", tmp.path().to_str().unwrap(), "--native", "--report", "ram"]);
    let warned = String::from_utf8_lossy(&cmd.output().expect("run").stderr).into_owned();
    assert!(
        warned.contains("warning: ") && warned.contains("ram.no-region"),
        "the tally must reach stderr and name the firing class, got: {warned:?}"
    );
    assert!(
        warned.contains("SIGIL_WARNINGS=full to list"),
        "the summary view must point at the full view, got: {warned:?}"
    );

    // `off` on the SAME tree prints no warn-tier line — proof the view is consulted
    // rather than the tier being empty.
    let mut off = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"));
    off.args(["build", "--aeon", tmp.path().to_str().unwrap(), "--native", "--report", "ram"])
        .env("SIGIL_WARNINGS", "off");
    let silenced = String::from_utf8_lossy(&off.output().expect("run").stderr).into_owned();
    assert!(
        !silenced.contains("warning: "),
        "`SIGIL_WARNINGS=off` must silence the tier, got: {silenced:?}"
    );
}
