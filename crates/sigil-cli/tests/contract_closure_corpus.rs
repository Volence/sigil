//! Contract-grammar v2 — THE ERROR GATE (§9 tier-timing flip). Runs the transitive
//! clobber closure ([`analyze_corpus_with`]) over the REAL aeon `.emp` corpus and
//! pins zero extern holes, zero §11 Q4 collisions, and an EMPTY firing set. Under
//! `SIGIL_STRICT_GATE` any transitive under-declaration is a BUILD ERROR: an
//! undeclared register effect in `.emp` cannot ship.
//!
//! THE SHAPE AXIS. The three gates below that own a firing set — drops, closure
//! residue, §6 flag results — walk EVERY SHIPPED SHAPE under its own profile
//! defines ([`native::shipped_shapes`]). A define-free walk cannot see inside
//! `if DEBUG == 1 { }` or `if SOUND_DRIVER_ENABLED == 1 { }`: those arms
//! comptime-vanish, so every register effect in them is invisible to the closure
//! and the residue reads empty over code the analysis never reached. Walking each
//! shape under its own defines is what makes the residue a statement about shipped
//! code, and
//! [`a_clobber_undeclared_inside_a_comptime_gate_fires_in_exactly_the_debug_shapes`]
//! is the standing proof that the axis is real and not seven labels on one walk.
//!
//! Two companion axes ride the same walk. The `[comptime.unresolved]` surface
//! ([`corpus_comptime_conditions_all_resolve_the_error_gate`]) pins that every
//! comptime condition RESOLVED — a poisoned condition discards both arms at zero
//! drops, so a lost/misspelled toggle is invisible to every other gate here. And
//! each shape binds its own game's L1 `implement` (the interface env), so an
//! `Iface.MEMBER`-gated arm — `Camera_Update`'s landing lock — is walked exactly
//! when its shape's ROM assembles it;
//! [`an_interface_gated_arm_is_walked_under_each_shapes_own_game`] is that proof.
//!
//! The [`corpus_report`] family below is define-FREE — a separate blind spot with
//! its own pinned censuses, tracked on kill-list row 103.
//!
//! Reference tree: defaults to the sibling aeon checkout (override with `AEON_DIR`).
//! Under `SIGIL_STRICT_GATE` a missing tree HARD-FAILS — these are shipping ERROR
//! gates and must run in the standard strict invocation, not silently skip.

use sigil_frontend_emp::corpus_contracts::{
    analyze_corpus, analyze_corpus_with, analyze_corpus_with_contracts, bind_corpus_interfaces,
    ContractReport,
};
use sigil_frontend_emp::parse_str;
use sigil_harness::contract_baseline;
use sigil_harness::native;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Recursively collect `*.emp` files under `dir`, skipping `.worktrees`.
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

/// The reference tree, or `None` when it is absent and strict mode is off.
///
/// House reference-gate pattern (repin_pins/mt_port, c5505f8): default the sibling
/// aeon tree, and under `SIGIL_STRICT_GATE` a missing reference is a HARD failure.
/// A shipping ERROR gate that silently skips whenever `AEON_DIR` is unset — as the
/// standard strict invocation (`SIGIL_STRICT_GATE=1 cargo test --workspace`, no
/// `AEON_DIR`) leaves it — never actually runs in the gate it exists for.
fn aeon_dir() -> Option<PathBuf> {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return None;
    }
    Some(aeon)
}

/// The whole corpus as `(path, source text)`, so a negative probe can doctor a
/// source before it is parsed.
///
/// COVERAGE IS PINNED, because a gate whose whole content is "this set is empty"
/// proves nothing about a corpus it did not read. Two things can silently shrink the
/// walk: `engine/debug/generated/compression_vectors.emp` is a GENERATED module — gitignored,
/// written by `tools/gen_compression_vectors.py`, absent on a cold checkout — and
/// `engine/debug/compression_selftest.emp` `use`s it at six instruction sites, so
/// without it the walk is 121 files where a warm tree is 122 and nothing else
/// notices. The floor and the named witness below turn both into a loud failure.
///
/// Deliberately NOT done here: emitting build products. `embed(...)` is a
/// `data` item the contract walk never resolves — `Embed` appears nowhere in
/// `sigil-frontend-emp` — so a missing `engine/sound/generated/` blob changes no
/// number this file asserts, and generating one would be a WRITE into `AEON_DIR`
/// from a read-only analysis gate, racing any concurrent build.
fn corpus_sources() -> Option<Vec<(PathBuf, String)>> {
    let aeon = aeon_dir()?;
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    // A FLOOR, not an equality: ordinary corpus growth must not churn this, but a
    // walk that lost a directory — or a generated module — must fail loudly.
    assert!(
        paths.len() >= 120,
        "the corpus walk found only {} .emp files under {} — too few to be the whole \
         corpus, so every assert-empty gate below would pass vacuously",
        paths.len(),
        aeon.display()
    );
    assert!(
        paths.iter().any(|p| p.ends_with("engine/debug/generated/compression_vectors.emp")),
        "engine/debug/generated/compression_vectors.emp is absent, so `engine.compression_vectors` \
         is not in the walk and compression_selftest.emp's uses of it vanish silently. \
         It is gitignored — run tools/gen_compression_vectors.py in {}",
        aeon.display()
    );
    Some(
        paths
            .into_iter()
            .map(|p| {
                let s = std::fs::read_to_string(&p)
                    .unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
                (p, s)
            })
            .collect(),
    )
}

/// The `games.<game>` module-id prefix a shape binds its L1 interfaces under —
/// [`native::GameProfile::game_module_prefix`], the one spelling of the
/// derivation, aliased for the probes' call sites.
fn game_module_prefix(profile: &native::GameProfile) -> &'static str {
    profile.game_module_prefix()
}

/// `(shape label, profile, report)` for every shipped shape — ONE parse, seven
/// analyses, each under the shape's own profile defines AND its own game's L1
/// interface binding. The profile rides along so a shape-partitioning probe reads
/// `profile.debug` / `profile.sound_on` / [`game_module_prefix`] rather than a
/// second, hand-kept list of labels.
///
/// A parse ERROR is fatal here: the closure charges only the instructions it
/// recovered, so a file that half-parses under-reports its register effects and
/// every gate below reads a smaller corpus than the one on disk. A BIND error is
/// equally fatal: a half-bound interface env silently poisons every `Iface.MEMBER`
/// condition it failed to fill, and a poisoned comptime condition discards both
/// arms — the exact invisibility `comptime_unresolved` exists to make loud.
fn analyze_every_shape(
    srcs: &[(PathBuf, String)],
) -> Vec<(&'static str, native::GameProfile, ContractReport)> {
    let files: Vec<_> = srcs
        .iter()
        .map(|(p, s)| {
            let (f, d) = parse_str(s);
            assert!(
                d.iter().all(|x| x.level != sigil_span::Level::Error),
                "{} parse errors: {d:?}",
                p.display()
            );
            f
        })
        .collect();
    native::shipped_shapes()
        .into_iter()
        .map(|(label, profile)| {
            let defines = native::shape_defines(&profile);
            let (iface_env, bind_diags) =
                bind_corpus_interfaces(&files, &defines, game_module_prefix(&profile));
            let bind_errors: Vec<_> = bind_diags
                .iter()
                .filter(|d| d.level == sigil_span::Level::Error)
                .collect();
            assert!(
                bind_errors.is_empty(),
                "shape `{label}`: L1 interface bind errors — the walk's env is \
                 incomplete, so every gate below would read a corpus with silently \
                 discarded comptime arms: {bind_errors:?}"
            );
            let r = analyze_corpus_with_contracts(&files, &defines, &iface_env);
            (label, profile, r)
        })
        .collect()
}

/// The substrate gate — DROPS ARE LOUD. The contract analysis evaluates each
/// `.emp` against the whole-corpus TYPE ENVIRONMENT (every struct/const/type
/// declaration in scope), so no field operand on an imported struct silently
/// vanishes from an analysis buffer. This pins the count of dropped instructions
/// to ZERO across the corpus: a silent under-approximation of any downstream
/// analysis (write set, clobber closure, dead-save, liveness) can no longer
/// return. It is the load-bearing precondition of every other contract gate —
/// before the corpus type environment, ~150 instructions across 24 files were
/// dropping, hiding real register effects beneath the closure/dead-save gates.
#[test]
fn corpus_has_zero_dropped_instructions() {
    let Some(srcs) = corpus_sources() else { return };
    for (label, _profile, r) in analyze_every_shape(&srcs) {
        assert_eq!(
            r.dropped_instrs, 0,
            "shape `{label}`: instructions dropped from analysis buffers (missing \
             import/type in scope, or a define the shape's profile does not carry?): {:?}",
            r.dropped_by_proc
        );
    }
}

/// §6 partial-width — THE WORD-FACET ERROR GATE. Every `preserves(dN.w)` claim in
/// the corpus must round-trip its low word under the closure's verified `effective`
/// oracle, on EVERY shipped shape. The per-file byte gate DEFERS the four witnesses
/// (Player_Main / TestPlayer_Main d7, Collected_ParkSlot d2, EntityWindow_TrySpawnRing
/// d5) — their round-trips cross calls it cannot see through — so this closure gate
/// is their final authority (the analog of `[proc.clobber-undeclared]` catching a
/// full-preserve deferral, which the word facet silences by licensing dN's clobber).
/// Runs per shape so the DEBUG-gated d2/d5 arms are proven by their round-trip in the
/// debug shape AND by never-written in the plain shape. A non-empty set is a false or
/// unprovable partial-width claim.
#[test]
fn corpus_word_facet_preserves_all_verify_the_error_gate() {
    let Some(srcs) = corpus_sources() else { return };

    // The four genuine witnesses — the procs that round-trip a register's LOW word
    // in their OWN body. PINNED, not merely counted: an assert-empty firing gate
    // says nothing about whether any claim was examined, so the CLAIM CENSUS is
    // asserted exactly. (The three transitive EntityWindow callers and `TestPlayer`
    // are deliberately ABSENT — they carry their register only through a call / a
    // fall-through, which under conservative v1 is a full clobber, so they keep
    // `clobbers`. Their absence here is the pin on that ruling.)
    let expected_claims: Vec<(String, String)> = [
        ("Collected_ParkSlot", "d2"),
        ("EntityWindow_TrySpawnRing", "d5"),
        ("Player_Main", "d7"),
        ("TestPlayer_Main", "d7"),
    ]
    .iter()
    .map(|(p, r)| (p.to_string(), r.to_string()))
    .collect();

    let shapes = analyze_every_shape(&srcs);
    let shape_count = shapes.len();
    let mut checked = 0usize;
    let mut report = String::from("\n== §6 word-facet claims per shape ==\n");

    for (label, _profile, r) in shapes {
        report.push_str(&format!("  [{label:<12}] claims {:?}\n", r.word_preserve_claims));
        // The ERROR gate: every declared low word round-trips under the closure's
        // verified `effective` oracle.
        assert!(
            r.word_preserve_firings.is_empty(),
            "shape `{label}`: `preserves(dN.w)` low word not provable under the closure \
             oracle (a false/unprovable partial-width claim): {:?}",
            r.word_preserve_firings
        );
        // NON-VACUITY: the gate above ranged over exactly these claims. The DEBUG-
        // gated d2/d5 arms exist in every shape's DECLARATION (a contract is not
        // comptime-conditional), so the claim set is shape-INDEPENDENT even though
        // what proves each claim is not: in a DEBUG shape the round-trip proves it,
        // in a plain shape the register is never written.
        assert_eq!(
            r.word_preserve_claims, expected_claims,
            "shape `{label}`: the word-facet claim census drifted — the error gate above \
             is only as meaningful as the set it ranged over"
        );
        checked += r.word_preserve_claims.len();
    }
    eprintln!("{report}");

    assert_eq!(
        checked,
        expected_claims.len() * shape_count,
        "expected {} claims across {shape_count} shapes, checked {checked}",
        expected_claims.len()
    );
}

/// THE `[comptime.unresolved]` ERROR GATE — the toggle complement of the drop gate
/// above. The two are disjoint by construction: a missing VALUE define drops the
/// instruction that needed it (loud above), while a missing/misspelled TOGGLE
/// define poisons a comptime `if` condition and discards BOTH arms whole — zero
/// drops, zero firings, the guarded code simply absent from every analysis. This
/// pins the set of names comptime conditions referenced that each shape's define
/// set + interface binding failed to resolve to EMPTY, per shape — so "a profile
/// lost a toggle" and "someone added a toggle no profile carries" both fail
/// loudly here instead of silently re-opening the arm-invisibility hole.
#[test]
fn corpus_comptime_conditions_all_resolve_the_error_gate() {
    let Some(srcs) = corpus_sources() else { return };
    for (label, _profile, r) in analyze_every_shape(&srcs) {
        assert!(
            r.comptime_unresolved.is_empty(),
            "shape `{label}`: [comptime.unresolved] — a comptime condition references \
             a name this shape's define set does not resolve, so its arms are \
             INVISIBLE to every analysis at zero drops: {:?}",
            r.comptime_unresolved
                .iter()
                .map(|(p, n, _)| (p.as_str(), n.as_str()))
                .collect::<Vec<_>>()
        );
    }
}

/// THE LEDGER REPRO, corpus-scale (gap-ledger `[bprime-4-gates lens B]` row): a
/// misspelled toggle in a comptime condition. Doctors `Collected_ParkSlot`'s
/// `if DEBUG == 1` dup-id scan into `if DEBUG_MISSPELLED == 1` and asserts, in
/// EVERY shape: (1) the surface names the exact proc and name — the misspelling
/// resolves in no shape, so this is also the gate's liveness probe (no shape's
/// silence can mean "saw nothing"); (2) `dropped_instrs` stays ZERO — the drop
/// gate is structurally blind to a poisoned condition; zero drops never proves
/// the defines reached the analysis, and this surface is what does.
#[test]
fn a_misspelled_toggle_in_a_condition_is_named_at_zero_drops_in_every_shape() {
    let Some(mut srcs) = corpus_sources() else { return };

    let mut doctored = false;
    for (p, s) in &mut srcs {
        if p.file_name().is_some_and(|n| n == "entity_window.emp") {
            let needle = "if DEBUG == 1 {\n            // no duplicate section ids in the park";
            let weaken = "if DEBUG_MISSPELLED == 1 {\n            // no duplicate section ids in the park";
            assert!(s.contains(needle), "negative probe anchor not found in {}", p.display());
            *s = s.replacen(needle, weaken, 1);
            doctored = true;
        }
    }
    assert!(doctored, "entity_window.emp not found in the corpus");

    for (label, _profile, r) in analyze_every_shape(&srcs) {
        assert!(
            r.comptime_unresolved
                .iter()
                .any(|(p, n, _)| p == "Collected_ParkSlot" && n == "DEBUG_MISSPELLED"),
            "shape `{label}`: a misspelled toggle in a comptime condition MUST land on \
             [comptime.unresolved] in every shape — it resolves in none. got: {:?}",
            r.comptime_unresolved
                .iter()
                .map(|(p, n, _)| (p.as_str(), n.as_str()))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            r.dropped_instrs, 0,
            "shape `{label}`: a poisoned condition discards arms WHOLE — it must not \
             drop instructions, or the drop gate would have covered this class and \
             this surface would be redundant: {:?}",
            r.dropped_by_proc
        );
    }
}

/// A profile that LOSES a toggle outright — the exact failure mode the ledger row
/// measured (`a profile that lost or misspelled SOUND_DRIVER_ENABLED or DEBUG
/// reproduces the blind spot at zero drops`). Strips `DEBUG` from each shape's
/// own define set and asserts the surface names it in EVERY shape, still at zero
/// drops. This is the proof the define set genuinely REACHES the analysis, which
/// zero drops never proved.
#[test]
fn a_profile_that_loses_a_toggle_is_named_by_the_surface_in_every_shape() {
    let Some(srcs) = corpus_sources() else { return };
    let files: Vec<_> = srcs.iter().map(|(_, s)| parse_str(s).0).collect();

    for (label, profile) in native::shipped_shapes() {
        let mut defines = native::shape_defines(&profile);
        defines.retain(|(k, _)| k != "DEBUG");
        let (iface_env, bind_diags) =
            bind_corpus_interfaces(&files, &defines, game_module_prefix(&profile));
        // The stripped toggle is not one the implement groups reference
        // (sonic4's group reads SOUND_DEBUG_HOTKEYS/SOUND_DRIVER_ENABLED), so the
        // bind must stay clean — a bind error here would hand the walk a
        // half-bound env and turn this probe's failure into a downstream riddle.
        assert!(
            bind_diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "shape `{label}`: unexpected L1 bind errors under the stripped define set: {:?}",
            bind_diags
                .iter()
                .filter(|d| d.level == sigil_span::Level::Error)
                .collect::<Vec<_>>()
        );
        let r = analyze_corpus_with_contracts(&files, &defines, &iface_env);
        assert!(
            r.comptime_unresolved.iter().any(|(_, n, _)| n == "DEBUG"),
            "shape `{label}` stripped of DEBUG: the surface must name the lost toggle \
             — this is the profile-lost-a-define blind spot. got: {:?}",
            r.comptime_unresolved
                .iter()
                .map(|(p, n, _)| (p.as_str(), n.as_str()))
                .collect::<Vec<_>>()
        );
        // The Z80 LANE's liveness witness: `Sequencer_NextOpcode` is a
        // `(cpu: z80)` proc with an in-body `if DEBUG == 1` — its row proves the
        // Z80 flag pass's collection reaches the report, not only the 68k
        // PASS 2's (a 68k-only surface would satisfy the assert above).
        assert!(
            r.comptime_unresolved
                .iter()
                .any(|(p, n, _)| p == "Sequencer_NextOpcode" && n == "DEBUG"),
            "shape `{label}` stripped of DEBUG: a Z80 module's comptime conditions \
             read the same defines, and its lane must surface them too. got: {:?}",
            r.comptime_unresolved
                .iter()
                .map(|(p, n, _)| (p.as_str(), n.as_str()))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            r.dropped_instrs, 0,
            "shape `{label}` stripped of DEBUG: still zero drops — the surface, not \
             the drop count, is what catches a lost toggle: {:?}",
            r.dropped_by_proc
        );
    }
}

/// The interface axis's own failure shape, held as an executable record: a walk
/// WITHOUT an interface env cannot resolve `Game.CAMERA_JUMP_LOCK`, so
/// `Camera_Update`'s landing-lock condition poisons and the surface names it —
/// through the MULTI-SEGMENT miss site (`eval_path`'s dotted fallthrough),
/// which no synthetic test reaches. This is the exact blind spot the per-shape
/// binding closes (the arm ships in five of the seven ROMs and an env-free
/// analysis never reads it), and it doubles as the tripwire for a half-bound
/// env: an interface whose member goes missing falls through the same dotted
/// fallthrough to this same surface.
#[test]
fn an_env_free_walk_names_the_interface_member_it_cannot_resolve() {
    let Some(srcs) = corpus_sources() else { return };
    let files: Vec<_> = srcs.iter().map(|(_, s)| parse_str(s).0).collect();
    let (label, profile) = native::shipped_shapes().remove(0);
    let r = sigil_frontend_emp::corpus_contracts::analyze_corpus_with(
        &files,
        &native::shape_defines(&profile),
    );
    assert!(
        r.comptime_unresolved
            .iter()
            .any(|(p, n, _)| p == "Camera_Update" && n == "Game.CAMERA_JUMP_LOCK"),
        "shape `{label}`, empty interface env: the interface-gated condition must \
         land on the surface — silence here means an env-free walk can silently \
         discard `Iface.MEMBER`-gated arms again. got: {:?}",
        r.comptime_unresolved
            .iter()
            .map(|(p, n, _)| (p.as_str(), n.as_str()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn corpus_closure_residue_is_empty_the_error_gate() {
    let Some(srcs) = corpus_sources() else { return };
    for (label, _profile, r) in analyze_every_shape(&srcs) {
        // Boundary decls resolve every extern call — no holes.
        assert!(
            r.closure.unresolved_callees.is_empty(),
            "shape `{label}`: unexpected extern holes (missing extern proc?): {:?}",
            r.closure.unresolved_callees
        );
        // No name declared both extern proc and proc (§11 Q4).
        assert!(
            r.extern_collisions.is_empty(),
            "shape `{label}`: extern/proc collisions: {:?}",
            r.extern_collisions
        );

        // THE ERROR GATE (WARN→ERROR flip, §9): the residue is ZERO in every shipped
        // shape. ANY firing here — an undeclared transitive register effect, or an
        // unbounded indirect — is a build error under the strict gate. This is the
        // permanent gate: an undeclared register effect in `.emp` can no longer ship,
        // and no longer ship behind a comptime arm one define set happens to elide.
        let residue: Vec<(String, String)> = r
            .firings
            .iter()
            .map(|f| (f.proc.clone(), f.reg.clone().unwrap_or_else(|| "<unbounded>".into())))
            .collect();
        assert!(
            r.firings.is_empty(),
            "shape `{label}`: closure firing(s) — an undeclared register effect must be \
             declared or verified-preserved before it can ship: {residue:?}"
        );
    }
}

/// LIVENESS — the closure analysis is a live analysis WITH REAL SUBJECTS in every
/// shape, not only in the three that assemble the shape-sensitivity probe's arm.
///
/// The shape-sensitivity probe below partitions: it demands a firing in the `DEBUG`
/// shapes and SILENCE in the plain ones. Silence is also what a walk that analyzed
/// nothing at all produces, so on its own it leaves the four plain shapes' halves of
/// `corpus_closure_residue_is_empty_the_error_gate` unproven — an empty residue there
/// would mean "clean" and "saw nothing" equally well.
///
/// So this drops `d1` from `Collected_UnparkSlot`'s `clobbers`, in code no comptime
/// condition guards. The firing is shape-invariant by construction, so EVERY shape
/// must show it, and any shape that stopped producing firings fails here instead of
/// passing the ERROR gate for the wrong reason.
#[test]
fn an_undeclared_clobber_in_ungated_code_fires_in_every_shape() {
    let Some(mut srcs) = corpus_sources() else { return };

    let mut doctored = false;
    for (p, s) in &mut srcs {
        if p.file_name().is_some_and(|n| n == "entity_window.emp") {
            let needle = "proc Collected_UnparkSlot () clobbers(d1, a1) preserves(a0) {";
            let weaken = "proc Collected_UnparkSlot () clobbers(a1) preserves(a0) {";
            assert!(s.contains(needle), "negative probe anchor not found in {}", p.display());
            *s = s.replacen(needle, weaken, 1);
            doctored = true;
        }
    }
    assert!(doctored, "entity_window.emp not found in the corpus");

    for (label, _profile, r) in analyze_every_shape(&srcs) {
        assert!(
            r.firings.iter().any(|f| {
                f.proc == "Collected_UnparkSlot" && f.reg.as_deref() == Some("d1")
            }),
            "shape `{label}`: an undeclared clobber in UNGATED code must fire in every \
             shape. It did not, so this shape's walk produced no firings and its half of \
             the residue gate proves nothing: {:?}",
            r.firings.iter().map(|f| (f.proc.as_str(), f.reg.as_deref())).collect::<Vec<_>>()
        );
    }
}

/// SHAPE-SENSITIVITY — the anti-vacuity pin for the closure gate, and the reason it
/// walks seven shapes instead of one.
///
/// Drops `d2` from `Collected_ParkSlot`'s `clobbers`, which the DEBUG duplicate-id
/// scan writes. The doctored corpus must fire in exactly the `DEBUG == 1` shapes and
/// in none of the plain ones. Both halves are load-bearing: the fires-here half
/// proves the ERROR gate has teeth on comptime-gated code — the code a define-free
/// walk cannot reach at all — and the silent-there half proves the defines genuinely
/// reach the analysis rather than every shape being one walk under seven labels.
#[test]
fn a_clobber_undeclared_inside_a_comptime_gate_fires_in_exactly_the_debug_shapes() {
    let Some(mut srcs) = corpus_sources() else { return };

    let mut doctored = false;
    for (p, s) in &mut srcs {
        if p.file_name().is_some_and(|n| n == "entity_window.emp") {
            // Drop the `d2.w` word facet: Collected_ParkSlot's DEBUG dup-id scan
            // writes d2, so with neither a `clobbers(d2)` nor the `preserves(d2.w)`
            // facet licensing it, the write is an undeclared clobber — in exactly the
            // DEBUG shapes (§6 partial-width: the facet is the license the probe removes).
            let needle = "proc Collected_ParkSlot () clobbers(d0-d1, a1) preserves(a0, d2.w) {";
            let weaken = "proc Collected_ParkSlot () clobbers(d0-d1, a1) preserves(a0) {";
            assert!(s.contains(needle), "negative probe anchor not found in {}", p.display());
            *s = s.replacen(needle, weaken, 1);
            doctored = true;
        }
    }
    assert!(doctored, "entity_window.emp not found in the corpus");

    // `profile.debug` IS the `DEBUG` value the shape lowers under, so the partition
    // cannot drift from the profile it is testing.
    let mut debug_shapes = 0;
    let mut plain_shapes = 0;

    for (label, profile, r) in analyze_every_shape(&srcs) {
        let hit = r.firings.iter().find(|f| {
            f.proc == "Collected_ParkSlot" && f.reg.as_deref() == Some("d2")
        });
        if profile.debug {
            debug_shapes += 1;
            assert!(
                hit.is_some(),
                "shape `{label}` assembles the `DEBUG == 1` block that writes d2, so the \
                 dropped declaration MUST fire — an empty residue here means the walk is \
                 not reading the shape's defines. firings: {:?}",
                r.firings.iter().map(|f| (f.proc.as_str(), f.reg.as_deref())).collect::<Vec<_>>()
            );
        } else {
            plain_shapes += 1;
            assert!(
                hit.is_none(),
                "shape `{label}` is DEBUG=0, so the doctored arm compiles away and must NOT \
                 fire — a firing here means every shape is walking one define set: {:?}",
                r.firings.iter().map(|f| (f.proc.as_str(), f.reg.as_deref())).collect::<Vec<_>>()
            );
        }
    }

    // Both halves must have RUN. A partition that lands every shape on one side
    // asserts nothing on the other, and the silent-there half is the load-bearing one.
    assert!(debug_shapes > 0 && plain_shapes > 0,
        "the shipped shapes no longer straddle DEBUG ({debug_shapes} debug / \
         {plain_shapes} plain) — this probe proves nothing");
}

/// GAME-AXIS SHAPE-SENSITIVITY — the L1 interface binding is real, per shape.
///
/// `Camera_Update`'s landing-lock arm is gated on `Game.CAMERA_JUMP_LOCK`, an L1
/// interface member — sonic4 binds it `true`, demo binds it `false`. Without an
/// interface env this condition poisons and BOTH arms are invisible in every
/// shape, while the arm ships in the sonic4 ROM — the invisibility this walk's
/// binding closes. This probe plants an undeclared `d5` write INSIDE the arm and
/// demands the partition: fires in exactly the five sonic4-family shapes (whose
/// game binds the lock on), silent in the two demo shapes (whose game compiles
/// the arm away). Both halves are load-bearing — the fires-here half proves the
/// arm genuinely enters the analysis buffer, and the silent-there half proves
/// each shape binds ITS OWN game's implement rather than one env under seven
/// labels.
#[test]
fn an_interface_gated_arm_is_walked_under_each_shapes_own_game() {
    let Some(mut srcs) = corpus_sources() else { return };

    let mut doctored = false;
    for (p, s) in &mut srcs {
        if p.file_name().is_some_and(|n| n == "camera.emp") {
            let needle = "if Game.CAMERA_JUMP_LOCK {\n            move.b  PL_STATE_ADDR, d2";
            let weaken = "if Game.CAMERA_JUMP_LOCK {\n            moveq   #0, d5\n            move.b  PL_STATE_ADDR, d2";
            assert!(s.contains(needle), "negative probe anchor not found in {}", p.display());
            *s = s.replacen(needle, weaken, 1);
            doctored = true;
        }
    }
    assert!(doctored, "camera.emp not found in the corpus");

    let (mut lock_on_shapes, mut lock_off_shapes) = (0, 0);
    for (label, profile, r) in analyze_every_shape(&srcs) {
        let hit = r
            .firings
            .iter()
            .find(|f| f.proc == "Camera_Update" && f.reg.as_deref() == Some("d5"));
        if game_module_prefix(&profile) == "games.sonic4" {
            lock_on_shapes += 1;
            assert!(
                hit.is_some(),
                "shape `{label}` binds sonic4's `CAMERA_JUMP_LOCK = true`, so the \
                 doctored write inside the interface-gated arm MUST fire — silence \
                 here means the arm is still invisible to the analysis: {:?}",
                r.firings.iter().map(|f| (f.proc.as_str(), f.reg.as_deref())).collect::<Vec<_>>()
            );
        } else {
            lock_off_shapes += 1;
            assert!(
                hit.is_none(),
                "shape `{label}` binds demo's `CAMERA_JUMP_LOCK = false`, so the arm \
                 compiles away and must NOT fire — a firing here means every shape \
                 is walking one game's binding: {:?}",
                r.firings.iter().map(|f| (f.proc.as_str(), f.reg.as_deref())).collect::<Vec<_>>()
            );
        }
    }
    assert!(
        lock_on_shapes > 0 && lock_off_shapes > 0,
        "the shipped shapes no longer straddle the game axis ({lock_on_shapes} sonic4 / \
         {lock_off_shapes} demo) — this probe proves nothing"
    );
}

/// `CRASH_REPORT` reaches the analysis — the first of the three toggles the
/// define-gate flip left with no proof at all. Its only condition sites are
/// `Vectors`' four `if DEBUG == 1 || CRASH_REPORT == 1` fault-cell arms, and
/// `||` short-circuits: a DEBUG=1 shape decides on the left operand and never
/// evaluates `CRASH_REPORT` at all. So stripping it from each shape's defines
/// must surface it in exactly the four PLAIN shapes and stay silent in the three
/// DEBUG ones — the partition is evaluation-order truth, pinned so a rewrite of
/// the vectors condition (or a lost short-circuit) re-measures loudly.
#[test]
fn a_lost_crash_report_define_is_named_in_exactly_the_plain_shapes() {
    let Some(srcs) = corpus_sources() else { return };
    let files: Vec<_> = srcs.iter().map(|(_, s)| parse_str(s).0).collect();

    let (mut plain_shapes, mut debug_shapes) = (0, 0);
    for (label, profile) in native::shipped_shapes() {
        let mut defines = native::shape_defines(&profile);
        defines.retain(|(k, _)| k != "CRASH_REPORT");
        let (iface_env, bind_diags) =
            bind_corpus_interfaces(&files, &defines, game_module_prefix(&profile));
        // The stripped toggle is not one the implement groups reference
        // (sonic4's group reads SOUND_DEBUG_HOTKEYS/SOUND_DRIVER_ENABLED), so the
        // bind must stay clean — a bind error here would hand the walk a
        // half-bound env and turn this probe's failure into a downstream riddle.
        assert!(
            bind_diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "shape `{label}`: unexpected L1 bind errors under the stripped define set: {:?}",
            bind_diags
                .iter()
                .filter(|d| d.level == sigil_span::Level::Error)
                .collect::<Vec<_>>()
        );
        let r = analyze_corpus_with_contracts(&files, &defines, &iface_env);
        let hit = r
            .comptime_unresolved
            .iter()
            .any(|(p, n, _)| p == "Vectors" && n == "CRASH_REPORT");
        if profile.debug {
            debug_shapes += 1;
            assert!(
                !hit,
                "shape `{label}` (DEBUG=1) short-circuits `DEBUG == 1 || …` before \
                 CRASH_REPORT evaluates — a hit here means evaluation order changed; \
                 re-measure this partition: {:?}",
                r.comptime_unresolved
                    .iter()
                    .map(|(p, n, _)| (p.as_str(), n.as_str()))
                    .collect::<Vec<_>>()
            );
        } else {
            plain_shapes += 1;
            assert!(
                hit,
                "shape `{label}` (DEBUG=0) must evaluate CRASH_REPORT in `Vectors`' \
                 fault-cell conditions — silence means the toggle no longer reaches \
                 the analysis: {:?}",
                r.comptime_unresolved
                    .iter()
                    .map(|(p, n, _)| (p.as_str(), n.as_str()))
                    .collect::<Vec<_>>()
            );
        }
    }
    assert!(
        plain_shapes > 0 && debug_shapes > 0,
        "the shipped shapes no longer straddle DEBUG ({debug_shapes} debug / \
         {plain_shapes} plain) — this probe proves nothing"
    );
}

/// `SOUND_DBG_MIRROR` reaches the analysis — its one condition site is
/// `if DEBUG == 1 && SOUND_DRIVER_ENABLED == 1 && SOUND_DBG_MIRROR == 1` in
/// vblank.emp, and `&&` short-circuits: only a DEBUG=1 sound-ON shape evaluates
/// the third operand. Stripping the define must surface it in exactly those
/// shapes (`sonic4 debug`, `config_a`) and nowhere else.
#[test]
fn a_lost_mirror_define_is_named_in_exactly_the_debug_sound_on_shapes() {
    let Some(srcs) = corpus_sources() else { return };
    let files: Vec<_> = srcs.iter().map(|(_, s)| parse_str(s).0).collect();

    let (mut reached_shapes, mut skipped_shapes) = (0, 0);
    for (label, profile) in native::shipped_shapes() {
        let mut defines = native::shape_defines(&profile);
        defines.retain(|(k, _)| k != "SOUND_DBG_MIRROR");
        let (iface_env, bind_diags) =
            bind_corpus_interfaces(&files, &defines, game_module_prefix(&profile));
        // The stripped toggle is not one the implement groups reference
        // (sonic4's group reads SOUND_DEBUG_HOTKEYS/SOUND_DRIVER_ENABLED), so the
        // bind must stay clean — a bind error here would hand the walk a
        // half-bound env and turn this probe's failure into a downstream riddle.
        assert!(
            bind_diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "shape `{label}`: unexpected L1 bind errors under the stripped define set: {:?}",
            bind_diags
                .iter()
                .filter(|d| d.level == sigil_span::Level::Error)
                .collect::<Vec<_>>()
        );
        let r = analyze_corpus_with_contracts(&files, &defines, &iface_env);
        let hit = r.comptime_unresolved.iter().any(|(_, n, _)| n == "SOUND_DBG_MIRROR");
        if profile.debug && profile.sound_on {
            reached_shapes += 1;
            assert!(
                hit,
                "shape `{label}` (DEBUG=1, sound ON) must evaluate SOUND_DBG_MIRROR — \
                 silence means the toggle no longer reaches the analysis: {:?}",
                r.comptime_unresolved
                    .iter()
                    .map(|(p, n, _)| (p.as_str(), n.as_str()))
                    .collect::<Vec<_>>()
            );
        } else {
            skipped_shapes += 1;
            assert!(
                !hit,
                "shape `{label}` short-circuits before SOUND_DBG_MIRROR evaluates — a \
                 hit here means evaluation order changed; re-measure this partition: {:?}",
                r.comptime_unresolved
                    .iter()
                    .map(|(p, n, _)| (p.as_str(), n.as_str()))
                    .collect::<Vec<_>>()
            );
        }
    }
    assert!(
        reached_shapes > 0 && skipped_shapes > 0,
        "the shipped shapes no longer straddle DEBUG+sound ({reached_shapes} reached / \
         {skipped_shapes} skipped) — this probe proves nothing"
    );
}

/// `SOUND_DEBUG_HOTKEYS` reaches the BIND, not the walk — its one condition site
/// is the comptime-`if` GROUP inside sonic4's `implement Game` block (the hook
/// bindings), which [`bind_corpus_interfaces`] evaluates while the proc walk
/// never sees it. Stripping the define must therefore fail the BIND with an
/// error naming the toggle in every sonic4-family shape; the demo shapes bind
/// demo's implement, which never references it, so they stay clean — and that
/// asymmetry is the honest residual: a demo-only corpus would carry no proof of
/// this toggle at all. (`analyze_every_shape` asserts bind errors empty, so the
/// lost define fails the same gate run, through the bind's mouth.)
#[test]
fn a_lost_hotkeys_define_fails_the_interface_bind_in_the_sonic4_shapes() {
    let Some(srcs) = corpus_sources() else { return };
    let files: Vec<_> = srcs.iter().map(|(_, s)| parse_str(s).0).collect();

    let (mut sonic4_shapes, mut demo_shapes) = (0, 0);
    for (label, profile) in native::shipped_shapes() {
        let mut defines = native::shape_defines(&profile);
        defines.retain(|(k, _)| k != "SOUND_DEBUG_HOTKEYS");
        let (_iface_env, bind_diags) =
            bind_corpus_interfaces(&files, &defines, game_module_prefix(&profile));
        let errors: Vec<&str> = bind_diags
            .iter()
            .filter(|d| d.level == sigil_span::Level::Error)
            .map(|d| d.message.as_str())
            .collect();
        if game_module_prefix(&profile) == "games.sonic4" {
            sonic4_shapes += 1;
            assert!(
                errors.iter().any(|m| m.contains("SOUND_DEBUG_HOTKEYS")),
                "shape `{label}`: stripping SOUND_DEBUG_HOTKEYS must fail sonic4's \
                 implement-group bind naming the toggle. bind errors: {errors:?}"
            );
        } else {
            demo_shapes += 1;
            assert!(
                errors.is_empty(),
                "shape `{label}`: demo's implement never references the toggle, so \
                 its bind must stay clean: {errors:?}"
            );
        }
    }
    assert!(
        sonic4_shapes > 0 && demo_shapes > 0,
        "the shipped shapes no longer straddle the game axis ({sonic4_shapes} sonic4 / \
         {demo_shapes} demo) — this probe proves nothing"
    );
}

/// Contract-grammar v2 G2 — the §6 flag-result must-use pin: every `.emp` caller
/// of a flag-result callee (`out(carry:)`) CONSUMES the carry, so the corpus has
/// ZERO `[call.flag-result-unused]` / `[call.result-invalid-path]` firings. The
/// three retrofitted callees (QueueDMA_Important/_Deferrable `dropped`,
/// RingBuffer_Add `full`) are all consumed via a `bcs` — no `@discards` anywhere.
/// This pin is the G2 regression guard (mirrors the G1 residue pin); a future
/// caller that drops a flag result breaks it.
#[test]
fn corpus_flag_results_are_all_consumed() {
    let Some(srcs) = corpus_sources() else { return };
    for (label, _profile, r) in analyze_every_shape(&srcs) {
        assert!(
            r.flag_firings.is_empty(),
            "shape `{label}`: unexpected flag-result firings (a dropped carry?): {:?}",
            r.flag_firings
        );
    }
}

/// Load the aeon corpus + run the contract analysis, or `None` (skip) when the
/// reference tree is absent — hard-failing under `SIGIL_STRICT_GATE` (the house
/// reference-gate pattern). Shared by the D1b/§3.2/§3.3 gates below.
///
/// DEFINE-FREE, and knowingly so: this walk resolves NO comptime define, so every
/// `if DEBUG == 1 { }` and `if SOUND_DRIVER_ENABLED == 1 { }` arm is absent from
/// the five gates it feeds — two of them ERROR gates. Its pinned censuses are
/// calibrated to that walk (`context_regions.len() == 17`; the same measurement
/// under the shipped profiles is 23 for the sonic4-family shapes and 20 for the
/// demo/config_b ones), so flipping it to [`analyze_every_shape`] is a per-shape
/// re-baselining, not a one-line change. Tracked as the highest-leverage remaining
/// row on kill-list row 103.
fn corpus_report() -> Option<ContractReport> {
    let aeon = aeon_dir()?;
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());
    let files: Vec<_> =
        paths.iter().map(|p| parse_str(&std::fs::read_to_string(p).unwrap()).0).collect();
    Some(analyze_corpus(&files))
}

/// THE D1b ERROR GATE (Phase-1 item #4 flip). Every register param of every
/// callee has a reaching definition on EVERY path at each `.emp` call site — the
/// corpus has ZERO `[call.input-undefined]` firings. This shipped WARN through G4;
/// now, with the credit source switched to the VERIFIED-out fixpoint (an out is a
/// definition only once PROVEN honest — the FindStagedBlock existence-lie can no
/// longer silently satisfy an input), it is the permanent ERROR gate: under the
/// strict invocation, ANY call passing an undefined register input is a build error
/// — the exact mistake a pass-3 contract-trusting register hoist could make.
#[test]
fn corpus_input_undefined_is_empty_the_error_gate() {
    let Some(r) = corpus_report() else { return };
    assert!(
        r.input_firings.is_empty(),
        "[call.input-undefined] (D1b): a callee register-param input has no reaching \
         definition on some path — it must be defined before the call: {:?}",
        r.input_firings
            .iter()
            .map(|f| (f.proc.as_str(), f.callee.as_str(), f.reg.as_str()))
            .collect::<Vec<_>>()
    );
}

/// §6 divergence TRIPWIRE (the honest-residual guard for keeping §6 on DECLARED
/// credit). §6 result-invalid-path uses the declared out maps (redefine-kill
/// semantics — a width-unverified out still redefines its register). This asserts
/// the §6 firings computed with DECLARED credit EQUAL those under VERIFIED credit
/// TODAY. The day a corpus change makes them diverge, declared credit is
/// suppressing a real firing that verified credit would surface on this ERROR gate
/// — the test fails and forces adjudication (move §6 to verified, or a per-lie-class
/// credit) at the moment it matters, instead of a silent miss. See the gap-ledger
/// row + the dividing-line table in the residue note.
#[test]
fn corpus_flag_results_declared_vs_verified_credit_agree() {
    let Some(r) = corpus_report() else { return };
    assert_eq!(
        r.flag_firings, r.flag_firings_verified_credit,
        "§6 invalid-path DIVERGES between declared and verified out-credit — declared \
         credit is suppressing a firing verified credit would show on the ERROR gate. \
         Adjudicate (the define-vs-redefine boundary may need §6 moved to verified). \
         declared={:?} verified={:?}",
        r.flag_firings, r.flag_firings_verified_credit
    );
}

/// CONSISTENCY (brief §2.6): the out-verify residue surface and D1b must-def read
/// ONE fixpoint source, so they cannot disagree on whether an out is honest —
/// every residue firing names an out ABSENT from the verified map, i.e. the
/// residue IS the verified complement.
///
/// The loop is guarded against vacuity IN ITS OWN WALK. Deferring that to
/// `contract_baselines_hold_for_every_shipped_shape` would not do: that gate runs
/// `analyze_every_shape` under each shape's defines, while `corpus_report()` walks
/// DEFINE-FREE and is a strict under-approximation of every shipped shape — it can
/// go empty while all seven still fire. `out_verify_corpus.rs` refuses the same
/// inference in the same direction, and for the same reason.
///
/// THE SECOND HALF OF THIS TEST NOW LIVES IN A SYNTHETIC, deliberately. It
/// asserted a corpus row that exists ONLY under verified credit, proving the
/// surface reads the VERIFIED map and not the DECLARED one. Two successive corpus
/// witnesses decayed out from under it: `Collision_GetType::out(d0)` when aeon
/// `49b7f3d` fused its callee into a leaf — it kept passing while detecting
/// nothing — and `Art_Decompress::out(a1)` when the decompressors' unproduced
/// `out(a1)` was retired to `clobbers(a1)`. No row in the present residue carries
/// the shape at all: every one writes its own register and sources none of it from
/// a callee, tail or successor, so a corpus witness is not merely fragile here but
/// unavailable. The fact is guarded instead — in both polarities, and against a
/// declared-credit mutant — by `sigil-frontend-emp/tests/corpus_contracts.rs`.
/// TWO tests, because `check_out` takes TWO credit maps and a guard over one says
/// nothing about the other: `the_out_residue_surface_uses_verified_credit_not_declared`
/// for the unconditional map, `the_conditional_out_credit_surface_also_uses_verified_credit`
/// for the conditional one. A witness whose
/// discriminating power lives in another repo needs re-checking whenever that repo
/// moves; a synthetic one does not.
#[test]
fn corpus_out_residue_is_the_verified_complement() {
    let Some(r) = corpus_report() else { return };
    assert!(
        !r.out_firings.is_empty(),
        "the residue is empty — the loop below would pass by measuring nothing"
    );
    for f in &r.out_firings {
        let marked_verified =
            r.verified_uncond_out.get(&f.proc).is_some_and(|s| s.contains(&f.reg));
        assert!(
            !marked_verified,
            "{}::out({}) is in the out-verify residue yet marked VERIFIED — the residue \
             surface and must-def credit have drifted apart",
            f.proc, f.reg
        );
    }
}

/// EDGE-SENSITIVE CONDITIONAL-OUT CREDIT, stated as a corpus witness rather than
/// left as an absence. `out_verify`'s header names `Load_Object`←`AllocDynamic` as
/// the worked example of an out sourced from a conditional callee and credited
/// only on the caller's cc-success edge. Nothing asserted it: the capability was
/// visible only as `Load_Object` NOT appearing in the residue, and an absence is
/// not a claim — a credit that regressed would surface as an unexplained new
/// baseline violation rather than as the named capability breaking.
///
/// The witness is TWO-part, because the interesting half is the conditional
/// sourcing, not the credit. If `AllocDynamic` were ever relabeled to an
/// unconditional `out(a1)`, `Load_Object`'s credit would still land — through the
/// trivial path — and a one-part witness would keep passing while testing nothing
/// about edge-sensitivity. So the source must still be genuinely conditional
/// (`a1 if eq`) AND the credit must still reach the caller.
///
/// D1c fires `Load_Object @ AllocDynamic :: a1` on this same triple and that is
/// NOT a contradiction: D1c's close is edge-BLIND by a deliberate ruling
/// (`calls.rs`'s `destroys_value` header, §4 Finding 5 — coupling D1c to the edge
/// primitive risks a degrade-to-miss on a `valid_edge` bail, judged worse than the
/// documented false positive). Same proc, same register, same callee, two lints,
/// opposite verdicts, both intended.
#[test]
fn corpus_conditional_callee_out_is_credited_edge_sensitively() {
    let Some(srcs) = corpus_sources() else { return };

    for (label, _profile, r) in analyze_every_shape(&srcs) {
        let cond = r.verified_cond_out.get("AllocDynamic");
        assert!(
            cond.is_some_and(|cs| {
                cs.iter().any(|(reg, cc)| reg.as_str() == "a1" && cc.as_str() == "eq")
            }),
            "shape `{label}`: AllocDynamic no longer verifies a CONDITIONAL out(a1 if eq) \
             (got {cond:?}). The Load_Object witness below only tests edge-sensitive credit \
             while its source is conditional — adjudicate before re-pointing it",
        );
        assert!(
            r.verified_uncond_out.get("Load_Object").is_some_and(|s| s.contains("a1")),
            "shape `{label}`: Load_Object::out(a1) is no longer credited from AllocDynamic's \
             cc-success edge — the edge-sensitive conditional-out credit named in \
             out_verify's header regressed. verified={:?}",
            r.verified_uncond_out.get("Load_Object")
        );
    }
}

/// §3.2 THE BRACKET GATE. Every `with` region in the corpus must prove its
/// pairing: no path leaves the body without the release, no branch enters it past
/// the acquire, no acquired context is taken twice. The per-file gate already
/// FAILS THE BUILD on these; this is the corpus-wide statement of the same fact,
/// so the class cannot regress behind a shape nobody builds.
#[test]
fn corpus_context_brackets_prove_the_error_gate() {
    let Some(r) = corpus_report() else { return };
    assert!(
        r.context_firings.is_empty(),
        "[context.escape]/[context.entry-skip]/[context.reacquire]: {:?}",
        r.context_firings
            .iter()
            .map(|f| (f.proc.as_str(), f.ctx.as_str(), f.kind))
            .collect::<Vec<_>>()
    );
}

/// §3.3 THE REQUIREMENT GATE, with its own anti-vacuity pins. An assert-empty over
/// `context_unsatisfied` is only as meaningful as the claims it ranged over and
/// the brackets that could discharge them, so both censuses are pinned here:
/// deleting a `requires`, or un-adopting the brackets, fails this test rather than
/// quietly emptying the gate.
#[test]
fn corpus_context_requirements_are_satisfied_the_error_gate() {
    let Some(r) = corpus_report() else { return };
    assert!(
        r.context_unsatisfied.is_empty(),
        "[context.unsatisfied]: a call site lacks a context its callee requires: {:?}",
        r.context_unsatisfied
            .iter()
            .map(|f| (f.proc.as_str(), f.callee.as_str(), f.ctx.as_str()))
            .collect::<Vec<_>>()
    );
    assert!(
        r.unknown_context_refs.is_empty(),
        "[context.unknown]: a requires/grants names a context no module declares: {:?}",
        r.unknown_context_refs
            .iter()
            .map(|(p, c, _)| (p.as_str(), c.as_str()))
            .collect::<Vec<_>>()
    );

    // Anti-vacuity 1 — the CLAIM census. A `requires`/`grants` clause is a
    // DECLARATION, not code, so this list is exact and shape-independent.
    assert_eq!(
        r.context_claim_sites.len(),
        10,
        "the vblank claim census moved — 1 grant root (VBlank_Handler) + 9 requiring \
         procs. Update deliberately: {:?}",
        r.context_claim_sites
    );
    assert!(
        r.context_claim_sites
            .contains(&("VBlank_Handler".into(), "grants".into(), "vblank".into())),
        "the vblank grant ROOT is gone — every requirement below it would then be \
         discharged by nothing and the gate would still read empty: {:?}",
        r.context_claim_sites
    );

    // Anti-vacuity 2 — the BRACKET census. The bus contexts must still be adopted;
    // an un-adoption would empty `context_firings` above without anyone noticing.
    //
    // SHAPE NOTE: this is the no-`-D` walk, in which `SOUND_DRIVER_ENABLED` does
    // not resolve — so both comptime-gated arms are inert and the three WIDE
    // sound-OFF fences (VInt_Level / VInt_Lag / Section_RedrawPlanes) lower their
    // bodies bare, contributing no region. Those three are proven by the PER-FILE
    // gate in every shape the ×7 byte bar builds, where the flag has a value.
    assert_eq!(
        r.context_regions.len(),
        17,
        "the `with` bracket census moved — corpus adoption changed. Update deliberately: {:?}",
        r.context_regions
    );
    // …with a NAMED witness, so an un-adoption that coincidentally preserves the
    // count still fails (the house pattern: pin content, not only cardinality).
    for witness in [("Read_Controllers", "z80_stopped"), ("Sound_PostByte", "ints_off")] {
        assert!(
            r.context_regions
                .iter()
                .any(|(p, c)| p == witness.0 && c == witness.1),
            "the `with {}` bracket in `{}` is gone: {:?}",
            witness.1,
            witness.0,
            r.context_regions
        );
    }
    // Anti-vacuity 3 — the DISCHARGED census. `context_unsatisfied` being empty
    // means nothing unless call sites were EXAMINED, and `call_target_sym`
    // resolves a DIRECT call only: a refactor to indirect dispatch would empty
    // the examined set while the firing set stayed (correctly) empty.
    assert!(
        !r.context_discharged.is_empty(),
        "no call site's `requires` was DISCHARGED — the satisfaction gate examined \
         nothing, so its emptiness proves nothing"
    );

    // Bus-context identification, pinned by EQUALITY. A `contains` would pass
    // with a wrong SET, and a wrongly-identified context hands the first proc
    // requiring it a bogus held entry — which silences `[bus.vdp-write-unstopped]`
    // (the crash class) for that whole proc. `ints_off` brackets a bus bracket
    // in four corpus sites and is the concrete near-miss.
    let bus: Vec<&str> = r.bus_contexts.iter().map(|s| s.as_str()).collect();
    assert_eq!(
        bus,
        vec!["z80_stopped"],
        "the bus-context set moved — it is read off what each bracket's ACQUIRE \
         splices, so this changes only when a context's acquire does"
    );
}

// ---------------------------------------------------------------------------
// The `--report contracts` surface.
// ---------------------------------------------------------------------------

/// THE SURFACE IS WIRED, AND IT IS SHAPE-PARAMETERIZED.
///
/// Every gate above calls the analysis in-process — the shape-walking ones through
/// [`analyze_every_shape`], the [`corpus_report`] family with no defines at all — so
/// all of them stay green if `run_contract_report` is deleted or stops reading the
/// target's profile. This drives the real binary and reads its real stdout.
///
/// The last two assertions pin zero drops AND zero `[comptime.unresolved]` names
/// through the binary, and they are complementary classes: a missing VALUE define
/// drops the instruction that needed it (without `MAX_RING_BUFFER`, `DrawRings`
/// cannot lower its `vram_art(...)` operand), while a missing TOGGLE define
/// poisons an `if` condition and discards both arms whole at zero drops — visible
/// only on the unresolved surface. The corpus-side gate for the latter is
/// [`corpus_comptime_conditions_all_resolve_the_error_gate`]; this test pins that
/// the REPORT renders the same surface, so a pasted report carries the evidence.
#[test]
fn the_contracts_report_is_wired_and_carries_the_targets_defines() {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    // The `.emp` corpus embeds the seam-emitted sound blobs, so they must exist
    // before any shape lowers — a missing embed target drops instructions, which is
    // precisely this gate's load-bearing assertion.
    sigil_harness::native::ensure_generated(&aeon);

    let run = |extra: &[&str]| -> String {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"));
        cmd.args(["build", "--aeon", aeon.to_str().unwrap(), "--native"]);
        cmd.args(extra);
        cmd.args(["--report", "contracts"]);
        let out = cmd.output().expect("run sigil build --report contracts");
        assert!(out.status.success(), "sigil build --report contracts failed: {out:?}");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    let plain = run(&["--game", "sonic4"]);
    let debug = run(&["--game", "sonic4", "--debug"]);
    let demo = run(&["--game", "demo"]);

    assert!(plain.starts_with("contract closure — sonic4 plain\n"), "header: {plain:.80}");
    assert!(debug.starts_with("contract closure — sonic4 debug\n"), "header: {debug:.80}");
    assert!(demo.starts_with("contract closure — demo plain\n"), "header: {demo:.80}");

    // The shapes differ in the defines the walk RAN under, not merely in a label:
    // DEBUG is the build-shape axis and MAX_RING_BUFFER is the game→engine axis.
    let defines = |s: &str| -> String {
        s.lines().find(|l| l.starts_with("defines: ")).expect("a defines line").to_string()
    };
    assert!(defines(&plain).contains("DEBUG=0"), "{}", defines(&plain));
    assert!(defines(&debug).contains("DEBUG=1"), "{}", defines(&debug));
    assert!(defines(&plain).contains("MAX_RING_BUFFER=128"), "{}", defines(&plain));
    assert!(defines(&demo).contains("MAX_RING_BUFFER=16"), "{}", defines(&demo));

    // The defines REACH the analysis: with them the corpus lowers whole, and no
    // comptime condition references a name the shape fails to resolve. The two
    // lines are the two complementary classes (value define / toggle define).
    for (label, out) in [("sonic4 plain", &plain), ("sonic4 debug", &debug), ("demo", &demo)] {
        assert!(
            out.contains("-- dropped instructions (must be 0): 0 --"),
            "{label}: the report dropped instructions, so its walk is missing defines:\n{out:.400}"
        );
        assert!(
            out.contains("-- [comptime.unresolved] condition names (must be 0): 0 --"),
            "{label}: the report carries unresolved comptime-condition names — a \
             define or interface binding is missing from its walk:\n{out:.400}"
        );
    }
}


/// THE INVOKE EDGE IS LIVE. `invoke Iface.hook` lowers to `jsr (bound).l`
/// (an `AbsSym`), the only corpus source of an AbsSym call edge, and the closure
/// must collect it so the bound proc's clobbers charge the invoker. Pinned from
/// the exposed edge census, NOT inferred from a silent residue: both corpus
/// invokers declare full-universe clobbers, so charging them moves no firing and
/// the edge's existence would otherwise be invisible. The hooks bind only under
/// SOUND_DEBUG_HOTKEYS==1 (config_a); every other shape carries the `= empty`
/// default and emits no invoke — both halves pinned.
#[test]
fn the_corpus_invoke_edges_are_collected() {
    let Some(srcs) = corpus_sources() else { return };
    let debug_edge = ("GameLoop".to_string(), "Debug_MusicToggle".to_string());
    let boot_edge = ("EntryPoint".to_string(), "SoundTest_BootPing".to_string());
    let mut bound_shapes = 0usize;
    for (label, profile, r) in analyze_every_shape(&srcs) {
        let hotkeys = native::shape_defines(&profile)
            .iter()
            .any(|(k, v)| k == "SOUND_DEBUG_HOTKEYS" && *v == 1);
        let has_debug = r.abs_call_edges.contains(&debug_edge);
        let has_boot = r.abs_call_edges.contains(&boot_edge);
        if hotkeys {
            assert!(
                has_debug && has_boot,
                "shape `{label}`: SOUND_DEBUG_HOTKEYS binds both hooks, so both invoke \
                 edges must be collected — got {:?}",
                r.abs_call_edges
            );
            bound_shapes += 1;
        } else {
            assert!(
                !has_debug && !has_boot,
                "shape `{label}`: no hotkeys, so the hooks stay `= empty` and emit no \
                 invoke edge — got {:?}",
                r.abs_call_edges
            );
        }
    }
    assert_eq!(
        bound_shapes, 1,
        "exactly one shipped shape (config_a) binds the hooks; if this count moves, \
         the invoke-edge pin's non-vacuity must be re-confirmed"
    );
}

/// THE AUTHORSHIP EXCLUSION, and its revert probe. The `assert`/`raise`
/// desugars lower to `jsr (MDDBG__…).l` naming contractless equ boundaries; they
/// are excluded from the closure by AUTHORSHIP (`ItemAuthor::AssertDesugar`), so
/// they never enter the edge census — their absence there proves the exclusion —
/// and the residue stays empty. The REVERT PROBE is `authored_rail_holes`:
/// exactly the `unresolved_callees` the gate WOULD report if the exclusion were
/// stripped. It names precisely the two desugar
/// boundaries in every shape (asserts ship everywhere), so a lost exclusion fails
/// here naming the exact sites.
#[test]
fn the_assert_rails_are_excluded_by_authorship_the_revert_probe() {
    let Some(srcs) = corpus_sources() else { return };
    let rails: BTreeSet<String> =
        ["MDDBG__ErrorHandler", "MDDBG__ErrorHandler_PagesController"]
            .into_iter()
            .map(String::from)
            .collect();
    for (label, _profile, r) in analyze_every_shape(&srcs) {
        for (proc, callee) in &r.abs_call_edges {
            assert!(
                !rails.contains(callee),
                "shape `{label}`: an AssertDesugar rail leaked into the edge census as \
                 ({proc}, {callee}) — the authorship exclusion is not applied"
            );
        }
        assert_eq!(
            r.authored_rail_holes, rails,
            "shape `{label}`: the would-be holes the exclusion suppresses must be exactly \
             the two desugar boundaries (strip the exclusion → the residue fails naming \
             these) — got {:?}",
            r.authored_rail_holes
        );
        // And the live residue stays empty — the exclusion actually holds.
        assert!(
            r.closure.unresolved_callees.is_empty(),
            "shape `{label}`: residue must be empty with the exclusion applied — got {:?}",
            r.closure.unresolved_callees
        );
    }
}

/// NON-VACUITY: the invoke edge does not just exist, it CHARGES. A
/// synthetic invoker declares only `clobbers(d0)` but its bound hook clobbers d5;
/// the transitive-clobber diagnostic must FIRE on the invoker. The corpus
/// invokers declare full-universe clobbers, so corpus silence proves the edge
/// harmless, not that the charge lands — this proves it lands, and the unbound
/// half proves the edge vanishes with the binding.
#[test]
fn a_synthetic_invoke_charges_the_bound_hooks_clobbers() {
    let engine = "\
module engine.synth
pub interface H {
    hook tick () clobbers(d0-d7/a0-a6) = empty
}
section s (cpu: m68000) {
    pub proc Invoker () clobbers(d0) {
        invoke H.tick
        rts
    }
    pub proc Boom () clobbers(d0-d7/a0-a6) {
        moveq #0, d5
        rts
    }
}
";
    let bound_game = "\
module games.synth.game
pub implement H {
    hook tick = Boom
}
";
    let files: Vec<_> = [engine, bound_game].iter().map(|s| parse_str(s).0).collect();
    let (env, bind_diags) = bind_corpus_interfaces(&files, &[], "games.synth");
    assert!(
        bind_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "bind errors: {bind_diags:?}"
    );
    let r = analyze_corpus_with_contracts(&files, &[], &env);
    assert!(
        r.abs_call_edges.contains(&("Invoker".to_string(), "Boom".to_string())),
        "the synthetic invoke edge must be collected: {:?}",
        r.abs_call_edges
    );
    assert!(
        r.firings
            .iter()
            .any(|f| f.proc == "Invoker" && f.reg.as_deref() == Some("d5")),
        "the invoke edge must charge the bound hook's d5 into the invoker (which declares \
         only d0) — got {:?}",
        r.firings
            .iter()
            .map(|f| (f.proc.as_str(), f.reg.as_deref()))
            .collect::<Vec<_>>()
    );

    // Unbound half: the demo-shaped implement binds no hook, so `invoke` emits
    // nothing — no edge, no charge, no firing on the invoker.
    let empty_game = "module games.synth.game\npub implement H {}\n";
    let files2: Vec<_> = [engine, empty_game].iter().map(|s| parse_str(s).0).collect();
    let (env2, bind2) = bind_corpus_interfaces(&files2, &[], "games.synth");
    assert!(
        bind2.iter().all(|d| d.level != sigil_span::Level::Error),
        "bind errors (unbound): {bind2:?}"
    );
    let r2 = analyze_corpus_with_contracts(&files2, &[], &env2);
    assert!(
        r2.abs_call_edges.is_empty(),
        "an unbound hook emits no invoke edge: {:?}",
        r2.abs_call_edges
    );
    assert!(
        !r2.firings.iter().any(|f| f.proc == "Invoker"),
        "an unbound hook charges nothing into the invoker: {:?}",
        r2.firings
            .iter()
            .map(|f| (f.proc.as_str(), f.reg.as_deref()))
            .collect::<Vec<_>>()
    );
}

/// HAND-WRITTEN vs AUTHORED, both polarities of the same `jsr (sym).l`
/// shape. A USER-authored abs call to an equ/link boundary is an unresolved HOLE
/// (its honest exit is an `extern proc` contract). The SAME shape emitted by the
/// `assert` desugar (authored AssertDesugar) is NOT a hole — excluded by authorship —
/// while still counted a would-be hole in `authored_rail_holes`, so the exclusion
/// is provably doing the work rather than the rail simply not existing.
#[test]
fn a_hand_written_abs_call_is_a_hole_the_authored_rail_is_not() {
    let src = "\
module test.polarity
pub equ Boundary = extern(\"BoundaryBlob\")
section s (cpu: m68000) {
    pub proc HandCall () clobbers(d0-d7/a0-a6) {
        jsr (Boundary).l
        rts
    }
    pub proc Asserter () clobbers(d0-d7/a0-a6) {
        assert.b d0, eq, #0
        rts
    }
}
";
    let files = vec![parse_str(src).0];
    let r = analyze_corpus_with(&files, &[("DEBUG".to_string(), 1)]);
    // Polarity 1 (User) — a hole.
    assert!(
        r.closure.unresolved_callees.contains("Boundary"),
        "a hand-written `jsr (Boundary).l` (User-authored) must be an unresolved hole: {:?}",
        r.closure.unresolved_callees
    );
    assert!(
        r.abs_call_edges.contains(&("HandCall".to_string(), "Boundary".to_string())),
        "the hand-written abs call must be a collected edge: {:?}",
        r.abs_call_edges
    );
    // Polarity 2 (AssertDesugar) — NOT a hole, but a would-be hole.
    assert!(
        !r.closure.unresolved_callees.contains("MDDBG__ErrorHandler"),
        "the assert desugar's rail (AssertDesugar) must be excluded from the residue: {:?}",
        r.closure.unresolved_callees
    );
    assert!(
        r.authored_rail_holes.contains("MDDBG__ErrorHandler"),
        "the excluded rail must still register as a would-be hole (else the exclusion is \
         vacuous here): {:?}",
        r.authored_rail_holes
    );
}

/// THE FROZEN CONTRACT BASELINES, PER SHIPPED SHAPE — the CI half of the same
/// pin the build-integrated closure gate enforces, reading the SAME constants
/// from `sigil_harness::contract_baseline` so the two cannot fork.
///
/// Per SHAPE, not once: `[call.live-clobbered]` is shape-dependent (the debug
/// family assembles debug-gated code the plain family never sees), and a
/// define-free walk would see neither arm of any gated `if`. Walking every shape
/// under its own profile is what makes this pin a statement about shipped code —
/// and it is what exercises `d1c_baseline(true)`, which no define-free gate can
/// reach.
///
/// RATCHET, both directions: a firing outside the baseline is a new violation; a
/// baseline row that stops firing is a stale pin, and possibly an analysis that
/// narrowed — the destructive direction, since the same closure feeds the
/// dead-save walk.
#[test]
fn contract_baselines_hold_for_every_shipped_shape() {
    let Some(srcs) = corpus_sources() else { return };

    for (label, profile, r) in analyze_every_shape(&srcs) {
        let d1c: Vec<(String, String, String)> = r
            .live_clobbered_firings
            .iter()
            .map(|f| (f.proc.clone(), f.callee.clone(), f.reg.clone()))
            .collect();
        let d = contract_baseline::diff_d1c(&d1c, profile.debug);
        assert!(
            d.is_clean(),
            "shape `{label}`: {}",
            contract_baseline::adjudication_message("[call.live-clobbered] (D1c)", &d)
        );

        let out: Vec<(String, String)> =
            r.out_firings.iter().map(|f| (f.proc.clone(), f.reg.clone())).collect();
        let d = contract_baseline::diff_out_unverified(&out);
        assert!(
            d.is_clean(),
            "shape `{label}`: {}",
            contract_baseline::adjudication_message("[proc.out-unverified]", &d)
        );

        // Non-vacuity that is not implied by `is_clean()`: the walk must have
        // produced firings at all. An analysis that silently returned nothing
        // would match an empty baseline cleanly — but these baselines are not
        // empty, so a zero result fails above. What this catches instead is the
        // shape LIST going empty, which `is_clean()` over zero iterations cannot.
        assert!(!d1c.is_empty(), "shape `{label}`: no D1c firings at all");
    }
    assert_eq!(
        native::shipped_shapes().len(),
        7,
        "the shape list shrank — a baseline that walks fewer shapes pins less"
    );
}
