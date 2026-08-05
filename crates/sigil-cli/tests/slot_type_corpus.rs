//! G5 §7 tier 5 — the `[call.slot-type-mismatch]` domain-newtype slot check over
//! the REAL aeon `.emp` corpus, walked under EVERY SHIPPED SHAPE. Three pins:
//!
//! 1. **POSITIVE** — the retrofitted seam (`Section_FlatIDXY` / `Section_GetSecPtrXY`
//!    typed `d2: GridX, d3: GridY`, callers `as`-blessed) produces an EMPTY firing
//!    set in all seven shapes: every domain-typed call slot is satisfied on every
//!    path, in every `-D` set a ROM ships under.
//! 2. **NEGATIVE** (the class-closure pin, `struct_field_disp_plus_n.rs` precedent) —
//!    doctoring ONE call site to swap the axis bless (`move.w d4, d2 as GridX`
//!    → `as GridY`) makes the build FAIL, naming that site: a `SectionId`/`GridX`
//!    swap can no longer ship silently.
//! 3. **SHAPE-SENSITIVITY** — a bless stripped from a site inside
//!    `if SOUND_DRIVER_ENABLED == 1 { }` fires in exactly the four sound-ON shapes
//!    and in none of the three sound-OFF ones.
//!
//! WHY ALL SEVEN. The walk's define set decides which code the walk can SEE.
//! Analyzed with no defines, every `if SOUND_DRIVER_ENABLED == 1 { }` block
//! comptime-vanishes, and this gate reported an empty firing set over a corpus in
//! which six real mismatches shipped — `PState_Ground` ×2, `PState_Spindash` ×2,
//! `Player_Animate`, `Player_Jump`, each a `move.b #SFXID_*, d0` handed to
//! `Sound_PlaySFX`'s `SfxId` slot. Pin 3 is what keeps that from returning: it is
//! false the moment the walk stops reading the shape's defines, because a stripped
//! bless inside a vanished block fires nowhere.
//!
//! REFERENCE-DEPENDENT: needs the sibling aeon tree (`AEON_DIR`). Under
//! `SIGIL_STRICT_GATE` a missing tree HARD-FAILS (shipping ERROR gate); otherwise
//! it skips green.

use sigil_frontend_emp::corpus_contracts::{analyze_corpus_with, ContractReport};
use sigil_frontend_emp::parse_str;
use sigil_harness::native;
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

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

/// Collect the whole corpus source, honoring `AEON_DIR`. Returns `None` (with the
/// strict-gate hard-fail already applied) when the tree is absent.
fn corpus_sources() -> Option<Vec<(PathBuf, String)>> {
    let aeon = aeon_dir();
    if !aeon.exists() {
        if strict_gate() {
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
    Some(paths.into_iter().map(|p| {
        let s = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        (p, s)
    }).collect())
}

fn parse_all(srcs: &[(PathBuf, String)]) -> Vec<sigil_frontend_emp::ast::File> {
    srcs.iter()
        .map(|(p, s)| {
            let (f, d) = parse_str(s);
            assert!(
                d.iter().all(|x| x.level != sigil_span::Level::Error),
                "{} parse errors: {d:?}",
                p.display()
            );
            f
        })
        .collect()
}

/// `(shape label, report)` for every shipped shape — ONE parse, seven analyses.
/// The define set is the shape's own [`native::shape_defines`], the exact set
/// `--report contracts` and the build read, so this gate and the ROM see the same
/// code.
fn analyze_every_shape(srcs: &[(PathBuf, String)]) -> Vec<(&'static str, ContractReport)> {
    let files = parse_all(srcs);
    native::shipped_shapes()
        .into_iter()
        .map(|(label, profile)| {
            (label, analyze_corpus_with(&files, &native::shape_defines(&profile)))
        })
        .collect()
}

/// POSITIVE: the retrofitted corpus fires ZERO slot-type mismatches — in every
/// shape, not just in the define-free one no ROM ships.
#[test]
fn retrofitted_corpus_has_zero_slot_mismatches() {
    let Some(srcs) = corpus_sources() else { return };
    for (label, r) in analyze_every_shape(&srcs) {
        assert!(
            r.slot_firings.is_empty(),
            "[call.slot-type-mismatch] firings on the retrofitted corpus in shape \
             `{label}`: {:#?}",
            r.slot_firings
        );
    }
}

/// SHAPE-SENSITIVITY — the anti-vacuity pin, and the reason this file walks seven
/// shapes instead of one.
///
/// Strips the `as SfxId` bless from `Player_Jump`'s `moveq #SFXID_JUMP, d0`, which
/// sits inside `if SOUND_DRIVER_ENABLED == 1 { }`. The doctored corpus must fire in
/// exactly the four shapes that assemble that block and in none of the three that
/// do not. Both halves are load-bearing: the fires-here half proves the gate has
/// teeth on comptime-gated code (the class that shipped six real mismatches past a
/// define-free walk), and the silent-there half proves the defines genuinely reach
/// the analysis rather than every shape being one walk under seven labels.
#[test]
fn a_bless_stripped_inside_a_comptime_gate_fires_in_exactly_the_sound_on_shapes() {
    let Some(mut srcs) = corpus_sources() else { return };

    let mut doctored = false;
    for (p, s) in &mut srcs {
        if p.file_name().is_some_and(|n| n == "player_ground.emp") {
            let needle = "moveq   #SFXID_JUMP, d0 as SfxId";
            let strip = "moveq   #SFXID_JUMP, d0";
            assert!(s.contains(needle), "negative probe anchor not found in {}", p.display());
            *s = s.replacen(needle, strip, 1);
            doctored = true;
        }
    }
    assert!(doctored, "player_ground.emp not found in the corpus");

    // The shapes whose profile sets `SOUND_DRIVER_ENABLED = 1`; the other three
    // compile the block away entirely.
    const SOUND_ON: [&str; 4] = ["sonic4 plain", "sonic4 debug", "config_a", "lean"];

    for (label, r) in analyze_every_shape(&srcs) {
        let hit = r.slot_firings.iter().find(|f| {
            f.proc == "Player_Jump" && f.callee == "Sound_PlaySFX" && f.reg == "d0"
        });
        if SOUND_ON.contains(&label) {
            assert!(
                hit.is_some(),
                "shape `{label}` assembles the `SOUND_DRIVER_ENABLED == 1` block, so the \
                 stripped bless MUST fire — an empty firing set here means the walk is \
                 not reading the shape's defines. firings: {:#?}",
                r.slot_firings
            );
            assert_eq!(hit.unwrap().expected, "SfxId");
            assert_eq!(hit.unwrap().found, None, "the bless was stripped, not swapped");
        } else {
            assert!(
                hit.is_none(),
                "shape `{label}` is sound-OFF, so the doctored site compiles away and must \
                 NOT fire — a firing here means every shape is walking one define set: {:#?}",
                r.slot_firings
            );
        }
    }
}

/// NEGATIVE: swap the axis bless at ONE call site → the build fails naming that
/// site. The genuineness pin: the check actually fires, it does not trivially
/// pass by there being no typed slots.
///
/// The doctored site is UNGATED code, so the firing is shape-invariant and every
/// shape must show it — asserted per shape rather than once, so a shape whose
/// walk silently stopped analyzing this module could not hide behind the others.
#[test]
fn swapped_axis_bless_fires_naming_the_site() {
    let Some(mut srcs) = corpus_sources() else { return };

    // Doctor the unique site 1630 in entity_window.emp: bless d3 (sec_y) as GridX
    // instead of GridY, so Section_FlatIDXY's `d3: GridY` slot receives a GridX —
    // the axis-swap class the split is FOR.
    let mut doctored = false;
    for (p, s) in &mut srcs {
        if p.file_name().is_some_and(|n| n == "entity_window.emp") {
            let needle = "move.w  d5, d3 as GridY        // sec_y";
            let swap = "move.w  d5, d3 as GridX        // sec_y";
            assert!(s.contains(needle), "negative probe anchor not found in {}", p.display());
            *s = s.replacen(needle, swap, 1);
            doctored = true;
        }
    }
    assert!(doctored, "entity_window.emp not found in the corpus");

    for (label, r) in analyze_every_shape(&srcs) {
        let hit = r
            .slot_firings
            .iter()
            .find(|f| f.callee == "Section_FlatIDXY" && f.reg == "d3" && f.expected == "GridY");
        assert!(
            hit.is_some(),
            "shape `{label}`: the swapped bless must fire [call.slot-type-mismatch] on \
             d3/GridY; firings: {:#?}",
            r.slot_firings
        );
        // The found state must be the WRONG axis (GridX), not merely untyped — the
        // check proves it caught a swap, not a missing bless.
        assert_eq!(hit.unwrap().found.as_deref(), Some("GridX"), "shape `{label}`");
    }
}

/// NEGATIVE (item-13 wave-1, FAMILY 1): doctor `Sound_PlayRing`'s first bless
/// from `as SfxId` to `as SongId` — a SongId reaching `Sound_PlaySFX`'s SfxId
/// slot at the (ungated) `jbra Sound_PlaySFX` tail-call. The wrong-sound class
/// the split is FOR; the build must fail naming that site with the wrong newtype
/// (SongId), not merely "untyped".
#[test]
fn sound_id_swap_fires_naming_the_site() {
    let Some(mut srcs) = corpus_sources() else { return };

    let mut doctored = false;
    for (p, s) in &mut srcs {
        if p.file_name().is_some_and(|n| n == "sound_api.emp") {
            let needle = "moveq   #SFXID_RING_RIGHT, d0 as SfxId";
            let swap = "moveq   #SFXID_RING_RIGHT, d0 as SongId";
            assert!(s.contains(needle), "negative probe anchor not found in {}", p.display());
            *s = s.replacen(needle, swap, 1);
            doctored = true;
        }
    }
    assert!(doctored, "sound_api.emp not found in the corpus");

    for (label, r) in analyze_every_shape(&srcs) {
        let hit = r
            .slot_firings
            .iter()
            .find(|f| f.callee == "Sound_PlaySFX" && f.reg == "d0" && f.expected == "SfxId");
        assert!(
            hit.is_some(),
            "shape `{label}`: the swapped bless must fire [call.slot-type-mismatch] on \
             d0/SfxId; firings: {:#?}",
            r.slot_firings
        );
        // Caught a swap (SongId), not merely a missing bless.
        assert_eq!(hit.unwrap().found.as_deref(), Some("SongId"), "shape `{label}`");
    }
}
