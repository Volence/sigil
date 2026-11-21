//! The driver's unit-level gates, moved here from `sigil-harness/src/native.rs` by G0b
//! Task 11 when the profile-driven driver was deleted. Every function these gates call is
//! a `sigil_build` function the game-as-data driver runs (the guard census and the
//! inapplicable allowlist, the overshoot trim, placement validation and the hole
//! interior, the error-handler-last check, helper imports, `--extra-entry`, the warn tier,
//! the width-flip report, the listing's phase markers, the chained
//! source locator, the region budgets). They moved verbatim; only their imports changed
//! (the module bodies' `use super::{..}` resolve against the re-exports below, which are
//! the ones `native.rs` carried).
//!
//! The native-local placement walk (`packed_true_bases` / `true_bases_by_index`), its
//! section_align-table alignment validator, its synthetic-entry writer and its
//! provisional-drift warning were deleted with that driver. The gates whose subject held
//! for the game-as-data driver moved onto its units (`align.rs`, `walk.rs`, `driver.rs`
//! unit tests); the rest went with that driver (the Task 11 report accounts for each by
//! name).

#![allow(clippy::too_many_lines)]

pub use sigil_build::appendix::*;
pub use sigil_build::diag::*;
pub use sigil_build::emit::*;
pub use sigil_build::guards::*;
pub use sigil_build::harvest::*;
pub use sigil_build::place::*;
use sigil_build::diag::render as diag_render;
use sigil_build::map_placement;
pub use sigil_ir::{Fragment, Section};

/// The inapplicable-guard allowlist the game-as-data driver enforces: `build_project_in`
/// passes `&[]` to `enforce_inapplicable_allowlist_against` (it was `native.rs`'s
/// `STAGE1_INAPPLICABLE_GUARDS`, already empty, before Task 11).
pub const STAGE1_INAPPLICABLE_GUARDS: &[(&str, &str)] = &[];

#[cfg(test)]
mod allowlist_tests {
    //! t24 both-directions negative controls for the Stage-1 inapplicable-drift-guard
    //! allowlist: the enforcement rejects an UNKNOWN Poison guard (a typo'd/renamed
    //! extern or a new twin-parity guard) AND a STALE allowlist (an entry that no
    //! longer folds to Poison), so the native gates can never silently vacate a guard.
    use super::{
        enforce_inapplicable_allowlist_against,
        STAGE1_INAPPLICABLE_GUARDS,
    };
    use sigil_ir::{Expr, LinkAssert, MsgPart};
    use sigil_span::{Diagnostic, Level, Span, SourceId};

    fn span(n: u32) -> Span {
        Span { source: SourceId(0), start: n, end: n }
    }
    /// A synthetic Poison diagnostic + its matching LinkAssert (site text) for one
    /// `(extern, site_token)` allowlist-shaped guard.
    fn guard(n: u32, ext: &str, site: &str) -> (Diagnostic, LinkAssert) {
        let d = Diagnostic {
            level: Level::Error,
            message: format!(
                "link assertion condition references symbol(s) `{ext}` not defined in this link"
            ),
            primary: span(n),
        };
        let a = LinkAssert {
            cond: Expr::Int(0),
            message: vec![MsgPart::Text(format!("engine/level/twin.asm and {site} disagree"))],
            fatal: false,
            level: Level::Error,
            span: span(n),
            kind: sigil_ir::AssertKind::Condition,
        };
        (d, a)
    }

    fn build(entries: &[(&str, &str)]) -> (Vec<Diagnostic>, Vec<LinkAssert>) {
        let mut ds = Vec::new();
        let mut as_ = Vec::new();
        for (i, (e, s)) in entries.iter().enumerate() {
            let (d, a) = guard(i as u32, e, s);
            ds.push(d);
            as_.push(a);
        }
        (ds, as_)
    }

    #[test]
    fn empty_allowlist_passes_with_no_poison() {
        // Post the bg/camera flip STAGE1_INAPPLICABLE_GUARDS is empty; the
        // enforcement asserts NO Poison drift guard exists → OK on an empty set.
        assert!(STAGE1_INAPPLICABLE_GUARDS.is_empty(), "the allowlist retired to empty");
        assert!(
            enforce_inapplicable_allowlist_against(&[], &[], STAGE1_INAPPLICABLE_GUARDS).is_ok()
        );
    }

    #[test]
    fn any_poison_guard_is_rejected_against_the_empty_allowlist() {
        // With the allowlist empty, ANY Poison-folding drift guard is a HARD FAIL
        // (the strengthened no-Poison invariant — a new twin-parity guard needs a
        // ruling, not a silent vacation).
        let (ds, as_) = build(&[("SOME_NEW_TWIN_CONST", "camera.emp")]);
        let refs: Vec<&Diagnostic> = ds.iter().collect();
        let err =
            enforce_inapplicable_allowlist_against(&refs, &as_, STAGE1_INAPPLICABLE_GUARDS)
                .unwrap_err();
        assert!(err.contains("NOT in the allowlist"), "got: {err}");
    }

    #[test]
    fn stale_allowlist_is_rejected() {
        // The staleness direction still holds (an allowlisted guard that no longer
        // folds to Poison → HARD FAIL), proven against a SYNTHETIC 1-entry allowlist
        // with no matching Poison diag (the empty shipped list can't be stale).
        let synthetic: &[(&str, &str)] = &[("SYNTHETIC_TWIN_CONST", "twin.emp")];
        let err = enforce_inapplicable_allowlist_against(&[], &[], synthetic).unwrap_err();
        assert!(err.contains("STALE"), "got: {err}");
    }
}

#[cfg(test)]
mod align_recompute_tests {
    //! Fix 3 — the trailing-`align` overshoot recompute-on-relocation. A pure-data
    //! section that ends in a stale `align` `Fill` (baked for its AS-residual position)
    //! must have that padding trimmed to its relocated reserved span, so its image no
    //! longer overshoots the next hard-org anchor. Byte-neutral when already in-bounds.
    use super::trim_trailing_align_overshoot;
    use sigil_ir::{Cpu, DataFragment, Fragment, Section, SectionPlacement};
    use sigil_span::Span;

    fn span0() -> Span {
        Span { source: sigil_span::SourceId(0), start: 0, end: 0 }
    }
    /// A pure-data section: `data_len` bytes of data, then a trailing zero-`Fill` of
    /// `pad` bytes (the align padding).
    fn data_then_pad(data_len: u32, pad: u32) -> Section {
        Section {
            name: "heightmaps".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0xAB; data_len as usize], fixups: vec![], span: span0() }),
                Fragment::Fill { value: 0, count: pad, span: span0() },
            ],
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            space: sigil_ir::AddressSpace::Image,
            equ_syms: vec![],
        }
    }

    #[test]
    fn overshoot_trims_trailing_pad_to_span() {
        // Image = 0x100 data + 0x40 pad = 0x140; relocated span clamps to 0x134
        // (0xC shorter). The 0xC overshoot must come off the trailing pad.
        let mut s = data_then_pad(0x100, 0x40);
        assert_eq!(s.image_len(), 0x140);
        trim_trailing_align_overshoot(&mut s, 0x134);
        assert_eq!(s.image_len(), 0x134, "image trimmed to the relocated span");
        // The data is untouched; only the pad shrank (0x40 → 0x34).
        match s.fragments.last().unwrap() {
            Fragment::Fill { count, value: 0, .. } => assert_eq!(*count, 0x34),
            other => panic!("expected trailing zero-fill, got {other:?}"),
        }
    }

    #[test]
    fn in_bounds_is_byte_neutral() {
        // image_len (0x140) <= span (0x140): nothing trimmed (the pinned-path case).
        let mut s = data_then_pad(0x100, 0x40);
        let before = s.image_bytes();
        trim_trailing_align_overshoot(&mut s, 0x140);
        assert_eq!(s.image_bytes(), before, "in-bounds section untouched");
    }

    #[test]
    fn overshoot_not_trailing_pad_is_left_for_the_linker() {
        // A section whose overshoot is real DATA (no trailing zero-fill) is NOT
        // trimmed — resolve_layout must reject it loudly, never silent corruption.
        let mut s = Section {
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0xAB; 0x140],
                fixups: vec![],
                span: span0(),
            })],
            ..data_then_pad(0, 0)
        };
        trim_trailing_align_overshoot(&mut s, 0x134);
        assert_eq!(s.image_len(), 0x140, "real data is never trimmed");
    }
}

#[cfg(test)]
mod placement_validation_tests {
    //! Parcel K5 — negative probes proving the map DRIVE-CONFIRMATION has teeth: each lint
    //! (undeclared-island, anchor-absent, order-diverged, order-undeclared) fires on a
    //! doctored map, and the correct map passes. Synthetic resolved layout:
    //! boot head (label-less) @0x0, GameLoop @0x100, ObjCodeBase @0x10000 (ANCHOR_GAP island).
    //! The SEMANTICS inverted at K5 (the map now DRIVES): `order-undeclared` is the
    //! completeness guard (the map must name every emitter), and `order-diverged` is the
    //! drive's own guard (the resolved layout must follow the declared sequence). The
    //! `drives_order_by_map_rank` probe below proves the packer consumes the DECLARATION,
    //! not the frozen provisional bases.
    use super::{hole_interior_faults, validate_placement};
    use crate::map_placement::{load_placement_map, PlacementMap};
    use sigil_ir::{Cpu, DataFragment, Fragment, Label, Section, SectionPlacement};
    use sigil_span::Span;

    fn span0() -> Span { Span { source: sigil_span::SourceId(0), start: 0, end: 0 } }

    fn sec(label: &str, lma: u32, len: usize) -> Section {
        Section {
            name: format!("sec{lma}"),
            cpu: Cpu::M68000,
            vma_base: None,
            lma,
            labels: if label.is_empty() { vec![] } else { vec![Label { name: label.into(), offset: 0 }] },
            fragments: vec![Fragment::Data(DataFragment { bytes: vec![0u8; len], fixups: vec![], span: span0() })],
            placement: SectionPlacement::Pinned,
            reserved_span: len as u32,
            group: None,
            bank: None,
            space: sigil_ir::AddressSpace::Image,
            equ_syms: Vec::new(),
        }
    }

    fn layout() -> Vec<Section> {
        vec![sec("", 0x0, 0x100), sec("GameLoop", 0x100, 0x50), sec("ObjCodeBase", 0x10000, 0x10)]
    }

    fn good_map() -> PlacementMap {
        load_placement_map(
            "order = [\"GameLoop\", \"ObjCodeBase\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n",
        )
        .unwrap()
    }

    #[test]
    fn correct_map_passes() {
        assert!(validate_placement(&layout(), &good_map(), false, &[]).is_ok());
    }

    #[test]
    fn undeclared_island_fires() {
        // Drop the 0x10000 anchor — the inferred island is now undeclared.
        let m = load_placement_map(
            "order = [\"GameLoop\", \"ObjCodeBase\"]\n[[anchor]]\nname=\"boot_head\"\nat=0x0\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.undeclared-island") && e.contains("0x10000"), "{e}");
    }

    #[test]
    fn anchor_absent_fires() {
        // Declare an anchor the layout has no island for.
        let m = load_placement_map(
            "order = [\"GameLoop\", \"ObjCodeBase\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n\
             [[anchor]]\nname=\"ghost\"\nat=0x99999\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.anchor-absent") && e.contains("0x99999"), "{e}");
    }

    #[test]
    fn order_diverged_fires() {
        // K5 drive-confirmation: the resolved layout (GameLoop@0x100 before ObjCodeBase)
        // must contradict a map declaring ObjCodeBase before GameLoop — the packer did not
        // honour the driving order (a packer bug the confirmation catches).
        let m = load_placement_map(
            "order = [\"ObjCodeBase\", \"GameLoop\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.order-diverged"), "{e}");
    }

    #[test]
    fn order_undeclared_fires() {
        // K5 completeness: omit GameLoop from the order — since the map DRIVES, a
        // byte-emitting section it fails to declare is rejected loud.
        let m = load_placement_map(
            "order = [\"ObjCodeBase\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n",
        ).unwrap();
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.order-undeclared") && e.contains("GameLoop"), "{e}");
    }

    // ── `section:<name>` rows (parcel SECTION-ROW) ──
    // `sec()` names a section `sec{lma}`: GameLoop's section is `sec256`, ObjCodeBase's
    // is `sec65536`. The rows below declare GameLoop by NAME instead of by head label.

    fn anchors() -> &'static str {
        "[[anchor]]\nname=\"boot_head\"\nat=0x0\n[[anchor]]\nname=\"object_bank\"\nat=0x10000\n"
    }

    fn map_with_order(order: &str) -> PlacementMap {
        load_placement_map(&format!("order = [{order}]\n{}", anchors())).unwrap()
    }

    #[test]
    fn section_row_satisfies_completeness() {
        // GameLoop emits bytes and is declared ONLY as `section:sec256` — that row satisfies
        // `[map.order-undeclared]` (the same map with the row dropped fails, per
        // `order_undeclared_fires`).
        let m = map_with_order("\"section:sec256\", \"ObjCodeBase\"");
        validate_placement(&layout(), &m, false, &[]).unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn section_row_ranks_where_it_sits_in_validation() {
        // The row sits AFTER ObjCodeBase, but the layout places GameLoop before it: the
        // drive-confirmation must name the row by its `section:` spelling.
        let m = map_with_order("\"ObjCodeBase\", \"section:sec256\"");
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.order-diverged") && e.contains("`section:sec256`"), "{e}");
    }


    #[test]
    fn zero_byte_section_row_is_inert() {
        // A zero-byte section (`sec336`, no label, no bytes) declared by `section:` row is a
        // reserved slot: it trips neither completeness nor divergence, wherever the row sits
        // — including a position the resolved layout contradicts.
        let mut secs = layout();
        secs.push(sec("", 0x150, 0));
        for order in [
            "\"GameLoop\", \"section:sec336\", \"ObjCodeBase\"",
            "\"section:sec336\", \"GameLoop\", \"ObjCodeBase\"",
            "\"GameLoop\", \"ObjCodeBase\", \"section:sec336\"",
        ] {
            let m = map_with_order(order);
            validate_placement(&secs, &m, false, &[]).unwrap_or_else(|e| panic!("order [{order}]: {e}"));
        }
    }

    #[test]
    fn unknown_section_row_fires() {
        // A `section:` row naming a section absent from the build is a named error, and the
        // message carries the row as written so the author can find it.
        let m = map_with_order("\"GameLoop\", \"section:ghost\", \"ObjCodeBase\"");
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(e.contains("map.order-unknown-section") && e.contains("`section:ghost`"), "{e}");
    }

    #[test]
    fn double_declared_section_fires() {
        // GameLoop declared by label AND by `section:sec256`: two rows, one section.
        let m = map_with_order("\"GameLoop\", \"section:sec256\", \"ObjCodeBase\"");
        let e = validate_placement(&layout(), &m, false, &[]).unwrap_err();
        assert!(
            e.contains("map.order-double-declared") && e.contains("`GameLoop`") && e.contains("`section:sec256`"),
            "{e}"
        );
    }


    #[test]
    fn shape_gated_sound_bank_anchor() {
        // A sound_on-gated anchor must be absent-checked only in sound_on shapes.
        let mut secs = layout();
        secs.push({
            let mut s = sec("SoundTablesZ80_Head", 0x58000, 0x20);
            s.vma_base = Some(0x8000); // phase bank
            s
        });
        let m = load_placement_map(
            "order = [\"GameLoop\", \"ObjCodeBase\", \"SoundTablesZ80_Head\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"object_bank\"\nat=0x10000\n\
             [[anchor]]\nname=\"sound_bank\"\nat=0x58000\nvma=0x8000\nwhen=\"sound_on\"\n",
        ).unwrap();
        // sound_on: the phase-bank island is declared → ok.
        assert!(validate_placement(&secs, &m, true, &[]).is_ok());
        // sound_off with the phase bank still present → it's an undeclared island (gate excludes it).
        let e = validate_placement(&secs, &m, false, &[]).unwrap_err();
        assert!(e.contains("map.undeclared-island") && e.contains("0x58000"), "{e}");
    }


    // ── The `[[hole]]` half (HOLE-INTERIOR-RESERVED) ────────────────────────────────
    //
    // Synthetic geometry modelled on the real one: a head section, then the hole's
    // filler at the `after` label, then the post-hole section. `sec()` names a section
    // `sec{lma}`, so the filler at 0x3D0 is section `sec976` — which is the name the
    // registry rows below bind to the filler MODULE id, exactly as a shipped profile's
    // registry binds `engine.z80_init` to `z80_idle`.

    /// The filler section's synthetic name (`sec()`'s naming rule at lma 0x3D0).
    const FILLER_SECTION: &str = "sec976";
    const FILLER_MODULE: &str = "engine.filler";

    fn filler_registry() -> Vec<super::ModuleSpec> {
        vec![super::ModuleSpec {
            module_id: FILLER_MODULE.to_string(),
            section: FILLER_SECTION.to_string(),
        }]
    }

    /// A map declaring one always-on hole: opens at `Filler`, runs to `at`, filled by
    /// `FILLER_MODULE`. `at` is a parameter so a probe can move the declared right edge
    /// without touching the layout.
    fn hole_map(at: u32) -> PlacementMap {
        load_placement_map(&format!(
            "order = [\"Head\", \"Filler\", \"PostHole\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[hole]]\nafter = \"Filler\"\nat = {at}\nfilled_by = \"{FILLER_MODULE}\"\n",
        ))
        .unwrap()
    }

    /// Head @0x0 (0x3D0 B) — filler @0x3D0 (0x28 B) — post-hole @0x3F8 (0xE B).
    /// One contiguous run, so the whole layout is a single island at 0x0.
    fn hole_layout() -> Vec<Section> {
        vec![sec("Head", 0x0, 0x3D0), sec("Filler", 0x3D0, 0x28), sec("PostHole", 0x3F8, 0xE)]
    }

    /// THE PIN FOR `[map.hole-anchor-missing]` — the presence half of the shipped path's
    /// hole arm. A layout whose `after` label is nowhere (the filler section dropped, as
    /// a shape gate or a rename would drop it) must be refused by name, and by THIS name
    /// rather than the interior half's `[map.hole-anchor-unresolved]`, which the same
    /// layout would otherwise reach.
    #[test]
    fn hole_anchor_missing_fires() {
        let layout: Vec<Section> =
            hole_layout().into_iter().filter(|s| s.name != FILLER_SECTION).collect();
        let m = load_placement_map(&format!(
            "order = [\"Head\", \"PostHole\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[hole]]\nafter = \"Filler\"\nat = 0x3F8\nfilled_by = \"{FILLER_MODULE}\"\n",
        ))
        .unwrap();
        let e = validate_placement(&layout, &m, false, &filler_registry()).unwrap_err();
        assert!(
            e.contains("map.hole-anchor-missing") && e.contains("`Filler`") && e.contains("0x3F8"),
            "{e}"
        );
        // CONTROL: the same map over the layout that DOES carry the label passes, so the
        // red above is the absent label and not the doctored order/anchor set.
        let full = hole_layout();
        let m_full = hole_map(0x3F8);
        validate_placement(&full, &m_full, false, &filler_registry())
            .unwrap_or_else(|e| panic!("control: {e}"));
    }

    /// THE CONTROL: a hole whose interior holds nothing but its declared filler.
    #[test]
    fn a_hole_holding_only_its_filler_has_no_faults() {
        let faults =
            hole_interior_faults(&hole_layout(), &hole_map(0x3F8), false, &filler_registry())
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(faults.is_empty(), "{faults:?}");
    }

    /// RED-FIRST: a byte-emitting section inside the declared interior is named — the
    /// probe shape that returned `Ok` before this predicate existed.
    #[test]
    fn a_section_inside_the_hole_interior_is_named() {
        // The hole is declared to run to 0x406, so the post-hole section at 0x3F8 sits
        // inside it. Nothing about the LAYOUT changes — only the declared right edge.
        let faults =
            hole_interior_faults(&hole_layout(), &hole_map(0x406), false, &filler_registry())
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(faults.len(), 1, "one intruder, one fault: {faults:?}");
        let f = &faults[0];
        assert!(f.contains("map.hole-interior-occupied"), "{f}");
        assert!(f.contains("`Filler`"), "the fault must name the hole it is about: {f}");
        assert!(f.contains("`sec1016`") && f.contains("`PostHole`"), "must name the intruding section and its head label: {f}");
        assert!(f.contains("[0x3F8,0x406)"), "must name the occupied span, not just the hole: {f}");
    }

    /// The occupied span is the INTERSECTION, so a section that only partly reaches into
    /// the hole is reported by the part that is inside it.
    #[test]
    fn a_partial_overlap_reports_the_overlapping_bytes_only() {
        // Declared right edge 0x400: the post-hole section spans [0x3F8,0x406) and only
        // [0x3F8,0x400) of it is inside.
        let faults =
            hole_interior_faults(&hole_layout(), &hole_map(0x400), false, &filler_registry())
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(faults.len(), 1, "{faults:?}");
        assert!(faults[0].contains("[0x3F8,0x400)"), "{}", faults[0]);
    }

    /// LOUD WHEN IT CANNOT MEASURE — the `after` label resolves nowhere. Driven directly
    /// (the shipped path refuses this first with `[map.hole-anchor-missing]`), so the
    /// function cannot report a clean sheet over a hole it never located.
    #[test]
    fn a_hole_whose_after_label_is_absent_cannot_be_measured() {
        let layout: Vec<Section> =
            hole_layout().into_iter().filter(|s| s.name != FILLER_SECTION).collect();
        let e = hole_interior_faults(&layout, &hole_map(0x406), false, &filler_registry())
            .unwrap_err();
        assert!(e.contains("map.hole-anchor-unresolved") && e.contains("`Filler`"), "{e}");
    }

    /// LOUD WHEN IT CANNOT MEASURE — one `after` label defined in two sections has no
    /// single left edge, so the interior is two different spans.
    #[test]
    fn a_hole_whose_after_label_resolves_twice_cannot_be_measured() {
        let mut layout = hole_layout();
        layout.push(sec("Filler", 0x1000, 0x10));
        let e = hole_interior_faults(&layout, &hole_map(0x406), false, &filler_registry())
            .unwrap_err();
        assert!(e.contains("map.hole-anchor-ambiguous") && e.contains("0x1000"), "{e}");
    }

    /// LOUD WHEN IT CANNOT MEASURE — a hole whose `at` is at or before its `after` label
    /// has an empty interior. That is the vacuous pass this predicate exists to remove,
    /// so it is a refusal rather than a clean sheet.
    #[test]
    fn a_hole_with_no_interior_is_refused_rather_than_passing_empty() {
        for at in [0x3D0u32, 0x100] {
            let e = hole_interior_faults(&hole_layout(), &hole_map(at), false, &filler_registry())
                .unwrap_err();
            assert!(
                e.contains("map.hole-bounds-degenerate") && e.contains("0x3D0"),
                "at={at:#X}: {e}"
            );
        }
    }

    /// LOUD WHEN IT CANNOT MEASURE — `filled_by` naming no module in the shape's registry
    /// leaves an empty permitted set, under which the intended filler itself reads as an
    /// intruder and the answer is a fault about the wrong thing.
    #[test]
    fn a_hole_whose_filler_module_is_unknown_is_refused() {
        let e = hole_interior_faults(&hole_layout(), &hole_map(0x406), false, &[]).unwrap_err();
        assert!(e.contains("map.hole-filler-unknown") && e.contains(FILLER_MODULE), "{e}");
    }

    /// LOUD WHEN IT CANNOT MEASURE — the filler module is declared but this build placed
    /// none of its sections, so an empty interior would mean UNFILLED, not reserved.
    #[test]
    fn a_hole_whose_filler_is_not_placed_is_refused() {
        // The `after` label survives on a section that is NOT the filler, so the left edge
        // still resolves and the refusal is specifically about the missing filler.
        let layout = vec![sec("Head", 0x0, 0x3D0), sec("Filler", 0x3D0, 0x28)];
        let registry = vec![super::ModuleSpec {
            module_id: FILLER_MODULE.to_string(),
            section: "a_section_this_build_does_not_place".to_string(),
        }];
        let e = hole_interior_faults(&layout, &hole_map(0x406), false, &registry).unwrap_err();
        assert!(e.contains("map.hole-filler-absent") && e.contains(FILLER_MODULE), "{e}");
    }

    /// A hole gated out by `when` contributes nothing — and the emptiness is the SHAPE's,
    /// not the predicate declining to look. The two shapes disagree over one layout.
    #[test]
    fn a_shape_gated_hole_is_absent_from_that_shape() {
        let m = load_placement_map(&format!(
            "order = [\"Head\", \"Filler\", \"PostHole\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[hole]]\nafter = \"Filler\"\nat = 0x406\nfilled_by = \"{FILLER_MODULE}\"\nwhen = \"sound_off\"\n",
        ))
        .unwrap();
        let off = hole_interior_faults(&hole_layout(), &m, false, &filler_registry()).unwrap();
        assert_eq!(off.len(), 1, "sound_off: the hole is live and occupied: {off:?}");
        let on = hole_interior_faults(&hole_layout(), &m, true, &filler_registry()).unwrap();
        assert!(on.is_empty(), "sound_on: the shape declares no hole at all: {on:?}");
    }

    // ── THROUGH `validate_placement` — the shipped ROM build path ────────────────────
    //
    // The predicate above is reached from the build, so each of its answers must be a
    // BUILD error. Four of its five refusals are reachable here; the fifth,
    // `[map.hole-anchor-unresolved]`, is shadowed by the presence arm that runs first
    // (`hole_anchor_missing_fires` pins that shadowing), so it is deliberately absent.

    /// A byte-emitting section inside the declared interior fails the build.
    #[test]
    fn the_build_path_refuses_an_occupied_hole_interior() {
        let e = validate_placement(&hole_layout(), &hole_map(0x406), false, &filler_registry())
            .unwrap_err();
        assert!(e.contains("map.hole-interior-occupied"), "{e}");
        assert!(e.contains("`PostHole`") && e.contains("[0x3F8,0x406)"), "{e}");
        // CONTROL: the identical layout under the declaration that matches it passes, so
        // the red is the declared right edge and not the fixture.
        validate_placement(&hole_layout(), &hole_map(0x3F8), false, &filler_registry())
            .unwrap_or_else(|e| panic!("control: {e}"));
    }

    /// EVERY occupant is reported, not the first — a stale `at` that swallows two
    /// sections is one declaration to correct, and a caller shown one name fixes it twice.
    #[test]
    fn the_build_path_names_every_occupant_of_one_hole() {
        // `hole_layout` ends at 0x406; a second post-hole section abuts it, and a right
        // edge past both puts both inside. Both bounds are read off the fixture.
        let mut layout = hole_layout();
        layout.push(sec("Trailer", 0x406, 0x8));
        let end = 0x406 + 0x8;
        let m = load_placement_map(&format!(
            "order = [\"Head\", \"Filler\", \"PostHole\", \"Trailer\"]\n\
             [[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[hole]]\nafter = \"Filler\"\nat = {end}\nfilled_by = \"{FILLER_MODULE}\"\n",
        ))
        .unwrap();
        let e = validate_placement(&layout, &m, false, &filler_registry()).unwrap_err();
        assert!(e.contains("`PostHole`") && e.contains("`Trailer`"), "both occupants: {e}");
        assert_eq!(e.lines().count(), 2, "one line per occupant: {e}");
    }

    /// LOUD ON UNMEASURABLE, through the build path: a hole whose declared interior is
    /// empty would read as checked while checking nothing.
    #[test]
    fn the_build_path_refuses_a_degenerate_hole() {
        let e = validate_placement(&hole_layout(), &hole_map(0x3D0), false, &filler_registry())
            .unwrap_err();
        assert!(e.contains("map.hole-bounds-degenerate"), "{e}");
    }

    /// LOUD ON UNMEASURABLE, through the build path: a `filled_by` naming no module in
    /// the shape's registry leaves nothing permitted, so the answer would be a fault
    /// about the wrong thing. An EMPTY registry is the shape of that mistake.
    #[test]
    fn the_build_path_refuses_an_unknown_filler_module() {
        let e = validate_placement(&hole_layout(), &hole_map(0x406), false, &[]).unwrap_err();
        assert!(e.contains("map.hole-filler-unknown") && e.contains(FILLER_MODULE), "{e}");
    }

    /// LOUD ON UNMEASURABLE, through the build path: the filler module is known but this
    /// build placed none of its sections, so the interior is UNFILLED, not reserved. The
    /// `after` label still resolves, so the presence arm passes and this is the refusal.
    #[test]
    fn the_build_path_refuses_a_filler_the_build_did_not_place() {
        let registry = vec![super::ModuleSpec {
            module_id: FILLER_MODULE.to_string(),
            section: "a_section_this_build_does_not_place".to_string(),
        }];
        let e =
            validate_placement(&hole_layout(), &hole_map(0x406), false, &registry).unwrap_err();
        assert!(e.contains("map.hole-filler-absent") && e.contains(FILLER_MODULE), "{e}");
    }

    /// LOUD ON UNMEASURABLE, through the build path: one `after` label defined in two
    /// sections gives the hole two left edges. The presence arm is satisfied by either,
    /// so only the interior half can catch it.
    #[test]
    fn the_build_path_refuses_an_ambiguous_hole_anchor() {
        let mut layout = hole_layout();
        layout.push(sec("Filler", 0x1000, 0x10));
        // No `order` (the duplicate head label is not the subject here) and the second
        // island declared, so the refusal is the ambiguity and not a map-completeness lint.
        let m = load_placement_map(&format!(
            "[[anchor]]\nname=\"boot_head\"\nat=0x0\n\
             [[anchor]]\nname=\"second\"\nat=0x1000\n\
             [[hole]]\nafter = \"Filler\"\nat = 0x406\nfilled_by = \"{FILLER_MODULE}\"\n",
        ))
        .unwrap();
        let e = validate_placement(&layout, &m, false, &filler_registry()).unwrap_err();
        assert!(e.contains("map.hole-anchor-ambiguous") && e.contains("0x1000"), "{e}");
    }
}

#[cfg(test)]
mod error_handler_placement_tests {
    //! The MDDBG blob-end contract guard (`check_error_handler_is_last`): the deb2
    //! appendix must start at `ErrorHandlerBlob + 0xF56`, because the vendored blob's
    //! two baked `lea` displacements point there. Negative probe = the historical bug
    //! (`Replay_OJZ_Fixture`, 0x140 B, placed between the blob and `EndOfRom`), which
    //! made every crash-screen line print `<unknown>` with no build-time signal.
    /// This probe's blob facts (a shipped engine states them in `engine.toml`).
    const ERROR_HANDLER_BLOB_LABEL: &str = "ErrorHandlerBlob";
    const ERROR_HANDLER_BLOB_LEN: u32 = 0xF56;

    fn check_error_handler_is_last(
        listing: &[sigil_link::ListingSymbol],
        appendix_start: usize,
        expect_island: bool,
    ) -> Result<(), String> {
        super::check_error_handler_is_last(ERROR_HANDLER_BLOB_LABEL, ERROR_HANDLER_BLOB_LEN, listing, appendix_start, expect_island)
    }

    fn sym(name: &str, value: u32) -> sigil_link::ListingSymbol {
        sigil_link::ListingSymbol { name: name.into(), value, is_equate: false, unused: false, lma: None }
    }

    /// The real debug-shape geometry: blob @0x5E688, so EndOfRom must be 0x5F5DE.
    fn listing() -> Vec<sigil_link::ListingSymbol> {
        vec![sym("BusError", 0x5E52E), sym("ErrorHandlerBlob", 0x5E688)]
    }

    #[test]
    fn blob_last_passes() {
        let end = (0x5E688 + ERROR_HANDLER_BLOB_LEN) as usize;
        assert!(check_error_handler_is_last(&listing(), end, true).is_ok());
    }

    #[test]
    fn section_after_blob_fires() {
        // The shipped bug: the 0x140-byte replay fixture between blob end and EndOfRom.
        let end = (0x5E688 + ERROR_HANDLER_BLOB_LEN) as usize + 0x140;
        let e = check_error_handler_is_last(&listing(), end, true).unwrap_err();
        assert!(e.contains("MDDBG blob-end contract VIOLATED"), "{e}");
        assert!(e.contains("+320"), "drift must be reported in bytes: {e}");
        assert!(e.contains("<unknown>"), "must name the silent runtime symptom: {e}");
        assert!(e.contains("map.toml"), "must name the fix site: {e}");
    }

    #[test]
    fn appendix_short_of_blob_end_fires() {
        let end = (0x5E688 + ERROR_HANDLER_BLOB_LEN) as usize - 2;
        let e = check_error_handler_is_last(&listing(), end, true).unwrap_err();
        assert!(e.contains("MDDBG blob-end contract VIOLATED") && e.contains("-2"), "{e}");
    }

    #[test]
    fn inert_for_a_shape_that_declares_no_island() {
        // The `lean` shape places the loud-failure handler instead of the island and
        // consults no symbol table, so the placement contract has no subject and the
        // guard must not fire on an arbitrary EndOfRom.
        let lean = vec![sym("ReleaseFault", 0x5CA40)];
        assert!(check_error_handler_is_last(&lean, 0x5CBAE, false).is_ok());
    }

    #[test]
    fn a_declared_island_with_no_blob_label_fires() {
        // The fail-open the `expect_island` parameter exists to close: the same
        // islandless listing, under a shape that DOES declare the island, is a renamed
        // label / dropped island / lost harvest row, not a satisfied contract.
        let no_blob = vec![sym("ReleaseFault", 0x5CA40)];
        let e = check_error_handler_is_last(&no_blob, 0x5CBAE, true).unwrap_err();
        assert!(e.contains("MDDBG island MEMBERSHIP violated"), "{e}");
        assert!(e.contains("<unknown>"), "must name the silent runtime symptom: {e}");
    }

    #[test]
    fn an_undeclared_island_whose_label_appears_fires() {
        // The other direction: a shape that declares no island whose listing defines
        // the blob label anyway has lost the registry's exclusive fault-handler split.
        let e = check_error_handler_is_last(&listing(), 0x5F5DE, false).unwrap_err();
        assert!(e.contains("MDDBG island MEMBERSHIP violated"), "{e}");
        assert!(e.contains("0x5e688"), "must name where the unexpected label sits: {e}");
    }
}

#[cfg(test)]
mod helper_import_tests {
    //! The helper-import rewrite keeps the import rule. `build_emp` replaces every
    //! author `use` of a comptime helper with a glob before the resolve pass runs,
    //! so the pass that owns "module `X` has no `pub` name `n`" never saw those
    //! lines, and a helper `use` naming nothing that exists built clean whenever
    //! nothing read the name (EMP-UNUSED-IMPORT-UNCHECKED). Each test builds a
    //! scratch tree the way `build_emp` does: [`rewrite_helper_imports`], the
    //! whole-program build, then [`helper_import_errors_in`] over what it lowered.
    use super::{helper_import_errors_in, rewrite_helper_imports};
    use sigil_frontend_emp::lower::LowerOptions;
    use sigil_frontend_emp::resolve::{self, manifest::Manifest};

    const HELPER: &str = "module pkg.h\npub const W = 4\nconst PRIV = 2\n";

    /// A consumer of the helper `pkg.h` whose import list is `names`, and nothing
    /// in it reads an imported name.
    fn consumer(id: &str, names: &str) -> String {
        format!("module {id}\nuse pkg.h.{{{names}}}\nproc init (a0: *u8) {{\n    rts\n}}\n")
    }

    /// Every Error message a `build_emp`-shaped build of `entry` produces.
    fn build_errors(files: &[(&str, &str)], entry: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        for (rel, src) in files {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, src).unwrap();
        }
        let (mut manifest, sdiags): (Manifest, _) = Manifest::scan(dir.path());
        assert!(
            sdiags.iter().all(|d| d.level != sigil_span::Level::Error),
            "the scratch tree must scan clean: {sdiags:?}"
        );
        let removed = rewrite_helper_imports(&mut manifest, &["pkg.h"]);
        let opts = LowerOptions {
            roots: None,
            initial_cpu: sigil_ir::Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        };
        let resolve::BuiltProgram { mut diags, lowered, .. } =
            resolve::build_program_open_embed_counted(&manifest, entry, None, &opts, sigil_frontend_emp::export_side::LayoutPass::Off, &|_| None);
        let extra = helper_import_errors_in(&manifest, &removed, &lowered, &diags);
        diags.extend(extra);
        diags
            .into_iter()
            .filter(|d| d.level == sigil_span::Level::Error)
            .map(|d| d.message)
            .collect()
    }

    /// THE ROW: a helper `use` naming an item the helper does not have, read by
    /// nothing, is refused with the rule's own wording.
    #[test]
    fn an_unread_helper_import_of_a_missing_name_is_refused() {
        let c = consumer("pkg.c", "NOPE_UNUSED_IMPORT");
        let errors = build_errors(&[("pkg/h.emp", HELPER), ("pkg/c.emp", &c)], "pkg.c");
        assert_eq!(errors, vec!["module `pkg.h` has no `pub` name `NOPE_UNUSED_IMPORT`"]);
    }

    /// A private helper item is refused too, although the rewrite publicizes it: the
    /// rule is decided against the tree as written.
    #[test]
    fn an_unread_helper_import_of_a_private_name_is_refused() {
        let c = consumer("pkg.c", "PRIV");
        let errors = build_errors(&[("pkg/h.emp", HELPER), ("pkg/c.emp", &c)], "pkg.c");
        assert_eq!(errors, vec!["module `pkg.h` has no `pub` name `PRIV`"]);
    }

    /// The accept arm: an unread import of a real `pub` helper item builds clean. A
    /// check that refused every removed import fails here and nowhere else.
    #[test]
    fn an_unread_helper_import_of_a_pub_name_is_accepted() {
        let c = consumer("pkg.c", "W");
        let errors = build_errors(&[("pkg/h.emp", HELPER), ("pkg/c.emp", &c)], "pkg.c");
        assert!(errors.is_empty(), "{errors:?}");
    }

    /// The scope arm: a module outside the build's `use` closure contributes no code,
    /// so its stale import is not this build's error, as the resolve pass also
    /// judges only the modules it lowers.
    #[test]
    fn a_stale_helper_import_outside_the_build_is_not_reported() {
        let c = consumer("pkg.c", "W");
        let other = consumer("pkg.other", "NOPE_UNUSED_IMPORT");
        let files = [("pkg/h.emp", HELPER), ("pkg/c.emp", c.as_str()), ("pkg/other.emp", other.as_str())];
        assert!(build_errors(&files, "pkg.c").is_empty());
        assert_eq!(
            build_errors(&files, "pkg.other"),
            vec!["module `pkg.h` has no `pub` name `NOPE_UNUSED_IMPORT`"],
            "the same module built as the entry is refused, so the silence above is scope"
        );
    }
}

#[cfg(test)]
mod extra_entry_tests {
    //! `--extra-entry`'s NAME RESOLUTION and its byte-neutrality REFUSAL, over a
    //! two-module scratch tree — so the per-kind verdicts are pinned without a
    //! reference tree, and the aeon-facing gate (`sigil-cli/tests/extra_entry.rs`)
    //! is free to assert the end-to-end contract instead of enumerating kinds.
    use super::{refuse_artifact_contribution, resolve_extra_entry};
    use sigil_frontend_emp::resolve::manifest::Manifest;

    /// Scan a scratch tree of `(relative path, source)` files.
    fn scan(files: &[(&str, &str)]) -> (tempfile::TempDir, Manifest) {
        let dir = tempfile::tempdir().unwrap();
        for (rel, src) in files {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, src).unwrap();
        }
        let (manifest, diags) = Manifest::scan(dir.path());
        assert!(
            diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "the scratch tree must scan clean: {diags:?}"
        );
        (dir, manifest)
    }

    const GUARD_ONLY: &str = "module pkg.guard_only\nconst K = 1\nensure(K == 1, \"holds\")\n";

    /// A dotted id, an aeon-relative path, and an absolute path all name the same
    /// module — the AUTHORING spelling and the on-disk spelling are both accepted,
    /// and the resolved id is the module's DECLARED one either way.
    #[test]
    fn an_extra_entry_resolves_by_id_and_by_path() {
        let (dir, m) = scan(&[("pkg/guard_only.emp", GUARD_ONLY)]);
        let root = dir.path();
        let abs = root.join("pkg/guard_only.emp");
        for arg in ["pkg.guard_only", "pkg/guard_only.emp", abs.to_str().unwrap()] {
            assert_eq!(
                resolve_extra_entry(&m, root, arg).unwrap(),
                "pkg.guard_only",
                "spelling `{arg}` must resolve"
            );
        }
    }

    /// A name that resolves to nothing is an error naming the argument — never a
    /// silent skip, which would let a lane pass with its subject deleted.
    #[test]
    fn an_unresolvable_extra_entry_is_an_error() {
        let (dir, m) = scan(&[("pkg/guard_only.emp", GUARD_ONLY)]);
        for arg in ["pkg.gone", "pkg/gone.emp", "/nowhere/gone.emp"] {
            let e = resolve_extra_entry(&m, dir.path(), arg).unwrap_err();
            assert!(e.contains(arg), "the error must name the argument: {e}");
            assert!(e.contains("no such module under the scan root"), "{e}");
        }
    }

    /// The byte-neutrality line, EVERY declaration kind the refusal ranges over —
    /// both directions. The ACCEPT rows are the load-bearing half: they are what fails
    /// if the refusal is ever widened by accident (collapsing the region-form `vars`
    /// arm to a bare `Item::Vars(_)` would take the overlay form with it, and an
    /// overlay is a comptime view over bytes that already exist).
    #[test]
    fn only_a_comptime_only_extra_entry_is_accepted() {
        // Each body is appended to a `module pkg.m` line. `(body, refused-kind)`.
        let cases: &[(&str, Option<&str>)] = &[
            // ACCEPTED — comptime-only, zero bytes and zero link symbols.
            ("const K = 1\nensure(K == 1, \"holds\")\n", None),
            ("struct S { a: u8 }\n", None),
            ("enum E: u8 { A = 0, B = 1 }\n", None),
            ("comptime fn f(x: u8) -> u8 { return x }\n", None),
            ("newtype N = u8\n", None),
            ("context ints_off { granted }\n", None),
            ("comptime test \"holds\" { }\n", None),
            // The overlay form of `vars` — a comptime view over an existing window,
            // NOT an allocation. Refusing it would refuse every SST-carrying module.
            ("struct W { pad: [u8; 4] }\nvars V: W.pad { t: u8 }\n", None),
            // `interface`/`implement` reach the whole-program contract bind pass; a
            // duplicate of either is a hard error there, so neither can rebind.
            ("interface I {\n    const C: u8\n}\n", None),
            ("implement I {\n    const C = 1\n}\n", None),
            // REFUSED — ROM producers.
            ("data D: [u8; 2] = [1, 2]\n", Some("data")),
            ("proc P () clobbers() { rts }\n", Some("proc")),
            ("section scratch { }\n", Some("section")),
            ("align 2\n", Some("align")),
            ("offsets O {\n    F0: F = 1\n}\n", Some("offsets")),
            ("table T (cell: *u8, hole: 0) {\n    A: a,\n}\n", Some("table")),
            ("dispatch R (encoding: word_offsets) {\n    A: a,\n}\n", Some("dispatch")),
            (
                "struct S { n: u8 }\nscript B (a0: *S) (encoding: word_offsets) shows done { }\n",
                Some("script"),
            ),
            // REFUSED — a link symbol.
            ("equ Q = 1\n", Some("equ")),
            // REFUSED — RAM allocators (the compiler's own predicate).
            ("region upper_ram @ $FFFF8000 .. $FFFFFFFF\n", Some("region")),
            ("vars upper_ram { a: u8 }\n", Some("vars")),
        ];
        for (body, refused) in cases {
            let src = format!("module pkg.m\n{body}");
            let (_dir, m) = scan(&[("pkg/m.emp", &src)]);
            let got = refuse_artifact_contribution(&m, "pkg.m", "pkg.m");
            match refused {
                None => assert!(got.is_ok(), "`{body}` must be accepted: {got:?}"),
                Some(kind) => {
                    let e = got.expect_err(&format!("`{body}` must be refused"));
                    assert!(e.contains(&format!("`{kind}`")), "must name `{kind}`: {e}");
                    assert!(e.contains("byte-neutral by contract"), "{e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod warn_tier_tests {
    //! THE WARN TIER's collection contract. Every non-error diagnostic the `.emp`
    //! build produces reaches [`EmpProgram::warnings`] located and deduplicated,
    //! and nothing else does: errors abort the build through `Err`, and the
    //! synthetic entry module the harness writes itself is not the reader's code.
    use super::{collect_warnings, BuildWarning};
    use sigil_frontend_emp::resolve::manifest::{Manifest, SourceIndex};
    use sigil_span::{Diagnostic, Level, SourceId, Span};

    /// SourceId(0) is a real fixture on disk whose third line starts at offset 10;
    /// SourceId(1) is the generated entry — a path that does not exist, as
    /// `build_emp` records its synthetic module.
    const REAL: SourceId = SourceId(0);
    const GENERATED: SourceId = SourceId(1);

    /// The index over a manifest whose SourceId(0) is a real file and whose
    /// SourceId(1) is a generated module with no file on disk.
    fn index_over(dir: &std::path::Path) -> SourceIndex {
        SourceIndex::new(&manifest_over(dir))
    }

    fn manifest_over(dir: &std::path::Path) -> Manifest {
        let real = dir.join("real.emp");
        std::fs::write(&real, "module m\n\nproc P () {\n").unwrap();
        let mut sources = std::collections::HashMap::new();
        sources.insert(REAL, real);
        sources.insert(GENERATED, dir.join("__native_flip_entry__.emp"));
        Manifest { modules: Vec::new(), by_id: std::collections::HashMap::new(), sources }
    }

    fn diag(level: Level, source: SourceId, at: u32, message: &str) -> Diagnostic {
        Diagnostic {
            level,
            message: message.to_string(),
            primary: Span { source, start: at, end: at },
        }
    }

    /// The four-source collection: errors are excluded (the build already failed on
    /// them), the generated entry is excluded, and a real warning survives with its
    /// `path:line:col` and its bracketed id parsed out.
    #[test]
    fn collects_only_reportable_non_errors() {
        let dir = tempfile::tempdir().unwrap();
        let m = index_over(dir.path());
        // Offset 10 is on line 3 ("module m\n" = 9 bytes, "\n" = 1).
        let mdiags = vec![diag(Level::Warning, REAL, 10, "[module.path-mismatch] renamed")];
        let pdiags = vec![diag(Level::Error, REAL, 0, "[parse.bad] boom")];
        let bdiags = vec![diag(Level::Warning, GENERATED, 0, "[import.no-names] whole-module")];
        let place = vec![diag(Level::Note, REAL, 0, "[place.fyi] noted")];

        let got = collect_warnings(&m, &[&mdiags, &pdiags, &bdiags, &place], Some(GENERATED));

        let shown: Vec<&str> = got.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(
            shown,
            ["module.path-mismatch", "place.fyi"],
            "errors and generated-entry diagnostics must not be reported: {got:?}"
        );
        assert_eq!(got[0].level, Level::Warning);
        assert_eq!(got[1].level, Level::Note, "the Note tier rides the same channel");
        assert!(
            got[0].location.as_deref().is_some_and(|l| l.ends_with("real.emp:3:1")),
            "a real source must locate to path:line:col, got {:?}",
            got[0].location
        );
    }

    /// One lint firing at one site is ONE warning however many sources replay it —
    /// the reported count is a count of distinct problems. Two firings of the same
    /// lint at DIFFERENT offsets stay two.
    #[test]
    fn deduplicates_a_replayed_diagnostic_but_not_distinct_sites() {
        let dir = tempfile::tempdir().unwrap();
        let m = index_over(dir.path());
        let a = diag(Level::Warning, REAL, 0, "[proc.sr-undeclared] `P` writes `sr`");
        let b = diag(Level::Warning, REAL, 10, "[proc.sr-undeclared] `P` writes `sr`");

        let one = std::slice::from_ref(&a);
        let got = collect_warnings(&m, &[one, one, one, std::slice::from_ref(&b)], Some(GENERATED));

        assert_eq!(got.len(), 2, "replays collapse, distinct sites do not: {got:?}");
        assert_ne!(got[0].location, got[1].location);
    }

    /// Errors alone produce an EMPTY tier: the build fails on them through `Err`,
    /// so reporting them a second time as warnings would double-count.
    #[test]
    fn errors_alone_produce_an_empty_tier() {
        let dir = tempfile::tempdir().unwrap();
        let m = index_over(dir.path());
        let errs = vec![diag(Level::Error, REAL, 0, "[x.y] boom")];
        assert!(collect_warnings(&m, &[&errs], Some(GENERATED)).is_empty());
    }

    /// A FAILING warn-level `LinkAssert` reaches the warn tier.
    ///
    /// `[layout.odd-item]`'s data-item check is `Level::Warning`
    /// (`sigil_ir::LinkAssert::level`) and is evaluated at LINK time, past every
    /// lowering diagnostic source — so the ROM drivers must feed
    /// `check_link_asserts`' output through the collector, or the whole class is
    /// reported nowhere.
    ///
    /// NOT VACUOUS, and it is the only proof available: the aeon corpus records 75
    /// warn-level asserts per sonic4 build and every one of them PASSES, so a
    /// corpus measurement of this path reads identically whether or not the wiring
    /// exists. This drives the failure directly.
    #[test]
    fn a_failing_warn_level_link_assert_survives_the_error_filter() {
        let dir = tempfile::tempdir().unwrap();
        let index = index_over(dir.path());
        let assert_at_odd = sigil_ir::LinkAssert {
            // Folds to 0 — a failure.
            cond: sigil_ir::Expr::Int(0),
            message: vec![sigil_ir::MsgPart::Text(
                "[layout.odd-item] `T` lands at odd address".to_string(),
            )],
            fatal: false,
            level: Level::Warning,
            span: Span { source: REAL, start: 10, end: 10 },
            kind: sigil_ir::AssertKind::Condition,
        };
        let adiags = sigil_link::check_link_asserts(
            &[],
            &sigil_ir::SymbolTable::new(),
            std::slice::from_ref(&assert_at_odd),
        );
        assert_eq!(adiags.len(), 1, "a false cond must produce one diagnostic: {adiags:?}");
        assert_eq!(adiags[0].level, Level::Warning, "the assert's own level must ride through");

        let got = collect_warnings(&index, &[&adiags], None);
        assert_eq!(got.len(), 1, "the warn-tier assert must survive the error filter: {got:?}");
        assert_eq!(got[0].id, "layout.odd-item");
        assert!(got[0].location.as_deref().is_some_and(|l| l.ends_with("real.emp:3:1")));

        // The ERROR tier still aborts instead of being reported: the drivers' own
        // `filter(… == Level::Error)` partition must still see it.
        let fatal = sigil_ir::LinkAssert { level: Level::Error, ..assert_at_odd };
        let ediags = sigil_link::check_link_asserts(
            &[],
            &sigil_ir::SymbolTable::new(),
            std::slice::from_ref(&fatal),
        );
        assert!(collect_warnings(&index, &[&ediags], None).is_empty());
    }

    /// The id is the leading bracketed group; a message without one classifies as
    /// the empty id, which the summary shows as `unclassified` and the corpus gate
    /// refuses. Rendering names the level and reads like the error tier.
    #[test]
    fn an_unbracketed_message_has_no_id_and_an_unlocatable_span_renders_bare() {
        let dir = tempfile::tempdir().unwrap();
        let m = index_over(dir.path());
        let idless = vec![diag(Level::Warning, REAL, 0, "no bracket here")];
        let got = collect_warnings(&m, &[&idless], Some(GENERATED));
        assert_eq!(got[0].id, "");
        assert!(
            got[0].to_string().ends_with("real.emp:1:1: warning: no bracket here"),
            "rendered: {}",
            got[0]
        );

        // An unlocatable span degrades to `<level>: <message>`, never to a
        // fabricated position.
        let unlocatable = BuildWarning {
            level: Level::Note,
            id: "a.b".into(),
            location: None,
            message: "[a.b] hello".into(),
            primary: Span { source: GENERATED, start: 0, end: 0 },
        };
        assert_eq!(unlocatable.to_string(), "note: [a.b] hello");
    }
}

#[cfg(test)]
mod derived_layout_tests {
    //! Derived placement (2026-08-26, decision d-7): the frozen provisional bases are a
    //! CHECK on the last refreeze, never a build input that can refuse content growth.
    //! Every expectation below is DERIVED from the synthetic sections' sizes and the
    //! declared anchor set — no address is copied from a pin.
    use sigil_ir::{Cpu, DataFragment, Fragment, Label, Section, SectionPlacement};
    use sigil_span::Span;

    fn span0() -> Span {
        Span { source: sigil_span::SourceId(0), start: 0, end: 0 }
    }

    /// A pure-DATA ROM section (position-independent image) headed by `label`.
    fn data(label: &str, lma: u32, len: usize) -> Section {
        Section {
            name: format!("sec_{label}"),
            cpu: Cpu::M68000,
            vma_base: None,
            lma,
            labels: vec![Label { name: label.into(), offset: 0 }],
            fragments: vec![Fragment::Data(DataFragment { bytes: vec![0u8; len], fixups: vec![], span: span0() })],
            placement: SectionPlacement::Pinned,
            reserved_span: len as u32,
            group: None,
            bank: None,
            space: sigil_ir::AddressSpace::Image,
            equ_syms: Vec::new(),
        }
    }

    /// The fixtures' run head.
    const HEAD: u32 = 0x1000;


    // The fixtures' head labels were rows of the deleted `section_align::DECLARED` table
    // (the deleted walk refused a head label without one). The constants keep each
    // section's ROLE in the fixture readable.
    const L_CODE: &str = "EntryPoint";
    const L_T: &str = "BootData";



    // ── measure-at-packed-base (2026-08-26): a section whose LENGTH DEPENDS ON ITS
    // BASE — one `lea Table, aN`-shaped relaxable (RelaxAbsSym) whose operand width
    // is abs.w below $8000 and abs.l above it. Every expectation is DERIVED from the
    // sizes and the $8000 boundary; no address is copied from a run of the code.

    /// A CODE section holding exactly one RelaxAbsSym (`lea (T).w/.l, a0`): 4 bytes
    /// when `T` resolves under $8000 on the 24-bit bus, 6 above.
    fn code_abs(label: &str, lma: u32, target: &str) -> Section {
        use sigil_ir::{Expr, Fixup, FixupKind, RelaxCandidate};
        let sym = || Expr::Sym(target.into());
        let short = RelaxCandidate {
            bytes: vec![0x41, 0xF8, 0, 0],
            fixup: Fixup { kind: FixupKind::Abs16Be, offset: 2, target: sym() },
        };
        let long = RelaxCandidate {
            bytes: vec![0x41, 0xF9, 0, 0, 0, 0],
            fixup: Fixup { kind: FixupKind::Abs32Be, offset: 2, target: sym() },
        };
        Section {
            name: format!("sec_{label}"),
            cpu: Cpu::M68000,
            vma_base: None,
            lma,
            labels: vec![Label { name: label.into(), offset: 0 }],
            fragments: vec![Fragment::RelaxAbsSym { short, long, target: sym(), span: span0() }],
            placement: SectionPlacement::Pinned,
            reserved_span: 4,
            group: None,
            bank: None,
            space: sigil_ir::AddressSpace::Image,
            equ_syms: Vec::new(),
        }
    }

    /// The width-flip report (the non-convergence diagnostic's payload) names the
    /// section, both lengths, the base, and each flipping site as `file:line` with
    /// both encodings' widths.
    #[test]
    fn width_flip_report_names_the_relaxing_site() {
        use super::width_flip_report;
        let secs = vec![code_abs(L_CODE, 0x1000, L_T)];
        let order = vec![0usize];
        let report = width_flip_report(
            &secs,
            &order,
            &[0x4],
            &[vec![(span0(), 4)]],
            &[0x6],
            &[vec![(span0(), 6)]],
            &[Some(0x1000)],
            &|_| Some("games/sonic4/player/player_sensors.emp:202:17".to_string()),
        );
        for needle in [
            &format!("`sec_{L_CODE}`"),
            &format!("(`{L_CODE}`)"),
            "measures 0x4 then 0x6 at base 0x1000",
            "games/sonic4/player/player_sensors.emp:202:17 (4 B -> 6 B)",
        ] {
            assert!(report.contains(needle), "missing {needle:?} in {report:?}");
        }
    }

    // ── the far scratch at every slot ordinal: a never-pinned section measures at a
    // scratch slot drawn by its ordinal among the never-pinned sections, so anything that
    // shifts that ordinal (a zero-byte, label-less section the `.emp` lowering opens for a
    // run of top-level items such as an `ensure` after a file's last `section {}` block)
    // must decide no width and move no section.

    /// A zero-byte, label-less ROM section: no label names it, so nothing can reference
    /// it, and it emits nothing.
    fn zero_byte_blob() -> Section {
        Section {
            name: "text".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: Vec::new(),
            fragments: Vec::new(),
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            space: sigil_ir::AddressSpace::Image,
            equ_syms: Vec::new(),
        }
    }

    /// The largest count of zero-byte sections the two tests below put ahead of the
    /// never-pinned target: its slot ordinal then runs 0..=41, which covers every ordinal
    /// whose raw slot address is a multiple of 0x100_0000 (k = 9, 25 and 41), the ones a
    /// 24-bit mask turns into 0x0.
    const MAX_ZERO_BYTE_AHEAD: usize = 41;

    /// A reference from a PINNED section into a NEVER-PINNED one takes its width from the
    /// scratch slot the target measures at. `lea T, a0` with `T` in a never-pinned section
    /// must measure the abs.l form, 6 bytes (opcode 0x41F9 and a 4-byte address), whatever
    /// slot `T` draws: that is the width asl gives a reference whose target it cannot place,
    /// and the reason the far scratch exists.
    #[test]
    fn a_reference_into_the_far_scratch_measures_abs_l_at_every_slot_ordinal() {
        use super::measure_pinned;
        for ahead in 0..=MAX_ZERO_BYTE_AHEAD {
            let mut secs = vec![code_abs(L_CODE, HEAD, L_T)];
            secs.extend((0..ahead).map(|_| zero_byte_blob()));
            secs.push(data(L_T, 0, 0x10));
            let mut pins = vec![Some(HEAD)];
            pins.extend(std::iter::repeat_n(None, ahead + 1));
            let m = measure_pinned(&secs, &pins).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                m.img[0],
                6,
                "`lea T, a0` measured {} B with T's never-pinned section at scratch ordinal {ahead} \
                 (behind {ahead} zero-byte label-less sections): the far scratch must encode abs.l \
                 (6 B) at every ordinal, or a pinned referrer's length depends on how many \
                 never-pinned sections precede its target",
                m.img[0]
            );
        }
    }

}

#[cfg(test)]
mod phase_marker_tests {
    //! The PHASE derivation, at the only place it exists: the section.
    //!
    //! `value` in a listing row has always been the VMA. Nothing in the file said
    //! so, so a consumer needing the distinction re-derived it, and the only signal
    //! recoverable from the listing alone is the magnitude of the number, which is
    //! exactly why re-derivation by address range returns 68000 routines at low ROM
    //! addresses instead of phased ones. Here the assembler answers from the
    //! section that declared the phase.
    use super::listing_from_resolved;
    use sigil_ir::{Cpu, DataFragment, Fragment, Label, Section, SectionPlacement, SymbolTable};
    use sigil_span::{SourceId, Span};

    fn sp() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    fn data(n: usize) -> Fragment {
        Fragment::Data(DataFragment { bytes: vec![0u8; n], fixups: vec![], span: sp() })
    }

    fn sec(
        name: &str,
        vma_base: Option<u32>,
        lma: u32,
        frags: Vec<Fragment>,
        labels: &[(&str, u32)],
    ) -> Section {
        Section {
            name: name.into(),
            cpu: Cpu::M68000,
            vma_base,
            lma,
            labels: labels
                .iter()
                .map(|(n, o)| Label { name: (*n).into(), offset: *o })
                .collect(),
            fragments: frags,
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            space: sigil_ir::AddressSpace::Image,
            equ_syms: vec![],
        }
    }

    /// A phased section's labels carry BOTH addresses; an unphased section's carry
    /// no `lma` at all, so `Some` in a row always means a real phase.
    #[test]
    fn a_phased_section_yields_vma_and_lma_and_a_plain_one_yields_neither() {
        let out = listing_from_resolved(
            &[
                // Unphased: placed where it says it runs.
                sec("boot", None, 0x200, vec![data(0x40)], &[("EntryPoint", 0), ("Wait", 0x10)]),
                // Phased: runs at $8000, stored at $B8000 (aeon's sound-bank head).
                sec(
                    "soundbankhead",
                    Some(0x8000),
                    0xB8000,
                    vec![data(0x500)],
                    &[("SoundTablesZ80_Head", 0), ("SfxBlobWinTab", 0x45F)],
                ),
            ],
            &SymbolTable::new(),
        );
        let by = |n: &str| out.iter().find(|s| s.name == n).unwrap().clone();
        assert_eq!((by("EntryPoint").value, by("EntryPoint").lma), (0x200, None));
        assert_eq!((by("Wait").value, by("Wait").lma), (0x210, None));
        // The VMA is what the address rows have always printed; the LMA is new.
        assert_eq!(
            (by("SoundTablesZ80_Head").value, by("SoundTablesZ80_Head").lma),
            (0x8000, Some(0xB8000))
        );
        // The offset rides along, so a phased row names the byte, not just the base.
        assert_eq!((by("SfxBlobWinTab").value, by("SfxBlobWinTab").lma), (0x845F, Some(0xB845F)));
    }

    /// A RESERVE-ONLY section is NOT phased, whatever its VMA and LMA say.
    ///
    /// Aeon's RAM blocks are exactly this shape: `vma: $FFFFxxxx` over an LMA that
    /// anchors at the physical counter and places ZERO image bytes. An `lma` on one
    /// of those would name a storage address for bytes that were never stored, and
    /// a consumer following it would land in unrelated ROM. It is also by far the
    /// larger population, so admitting it would bury the rows that matter under
    /// every RAM label in the build.
    #[test]
    fn a_reserve_only_ram_block_is_not_reported_as_phased() {
        let out = listing_from_resolved(
            &[
                sec(
                    "ram",
                    Some(0xFFFF_0000),
                    0,
                    vec![Fragment::Reserve { count: 0x1000, span: sp() }],
                    &[("Camera_X", 0xA728)],
                ),
                sec("boot", None, 0x200, vec![data(4)], &[("EntryPoint", 0)]),
            ],
            &SymbolTable::new(),
        );
        let ram = out.iter().find(|s| s.name == "Camera_X").unwrap();
        assert_eq!(ram.value, 0xFFFF_A728, "a RAM label still lists at its RAM VMA");
        assert_eq!(ram.lma, None, "a reserve-only block must not claim a storage address");
        // NON-VACUITY. The same function DOES mark a byte-emitting phased section,
        // so this cannot pass by the derivation having been switched off entirely.
        let phased = listing_from_resolved(
            &[sec("bank", Some(0x8000), 0xB8000, vec![data(4)], &[("Head", 0)])],
            &SymbolTable::new(),
        );
        assert_eq!(phased[0].lma, Some(0xB8000));
    }
}

#[cfg(test)]
mod mixed_front_end_location_tests {
    //! THE TWO LOCATION AUTHORITIES OF A CHAINED (MIXED `.asm` + `.emp`) BUILD.
    //!
    //! [`resolve_chained`] builds ONE section list out of BOTH front ends, and a
    //! diagnostic raised over that list carries a bare [`sigil_span::Span`] — an id
    //! and a byte range, with nothing in it saying which map can read it. The AS
    //! front end's ids count its root and its `include` splices; the `.emp`
    //! manifest's ids count the scanned `.emp` files. The ranges OVERLAP, so
    //! locating an AS span through the `.emp` index does not fail: it names a
    //! different file, confidently, and the build says so with no error anywhere.
    //! (The rule is already stated on [`BuildWarning::from_as`]; these gates are
    //! that rule for the chained path's own consumers.)
    //!
    //! The fixture is the smallest thing that can tell the two apart: an `.emp`
    //! index whose id 1 is `second.emp`, and an AS map whose id 1 is `sound.asm`.
    //! A span with id 1 therefore renders a DIFFERENT PATH under each authority,
    //! so a gate here reads a path and never an id.
    use super::{check_joined_expansions, render_declared_chain, ChainSources, Fragment, Section};
    use sigil_frontend_emp::resolve::manifest::{Manifest, SourceIndex};
    use sigil_span::{Diagnostic, Level, SourceId, SourceMap, Span};

    const EMP_TEXT: &str = "module m\n\nproc P () {\n}\n";
    const ASM_TEXT: &str = "\tmove.w\td0,d1\n\n\tbra.s\t*\n";

    /// The byte offset of `text`'s THIRD line — DERIVED from the fixture text
    /// rather than counted by hand into a literal. Both fixtures have three lines
    /// of different lengths, and hand-counting one of them wrong is how a gate ends
    /// up asserting a column it never meant to.
    fn third_line(text: &str) -> u32 {
        text.match_indices('\n').nth(1).expect("the fixture has three lines").0 as u32 + 1
    }

    /// `first.emp` (id 0) and `second.emp` (id 1), both on disk: `SourceIndex`
    /// READS each file to answer `path:line:col`, so a fixture that does not
    /// exist would only exercise the degraded (`None`) arm and prove nothing.
    fn emp_index(dir: &std::path::Path) -> SourceIndex {
        let mut sources = std::collections::HashMap::new();
        for (id, name) in [(0u32, "first.emp"), (1, "second.emp")] {
            let p = dir.join(name);
            std::fs::write(&p, EMP_TEXT).unwrap();
            sources.insert(SourceId(id), p);
        }
        SourceIndex::new(&Manifest {
            modules: Vec::new(),
            by_id: std::collections::HashMap::new(),
            sources,
        })
    }

    /// The AS front end's own map: `boot.asm` (id 0) and `sound.asm` (id 1).
    fn as_map() -> SourceMap {
        let mut m = SourceMap::new();
        m.add_named("boot.asm".to_string(), ASM_TEXT.to_string());
        m.add_named("sound.asm".to_string(), ASM_TEXT.to_string());
        m
    }

    fn diag(source: u32, at: u32, message: &str) -> Diagnostic {
        Diagnostic {
            level: Level::Error,
            message: message.to_string(),
            primary: Span { source: SourceId(source), start: at, end: at },
        }
    }

    /// The chained authority under test, plus the fixture directory it reads from
    /// (dropping the directory deletes the `.emp` files, so it must outlive the
    /// index — `SourceIndex` reads each file to answer `path:line:col`).
    fn chained(dir: &std::path::Path) -> ChainSources {
        ChainSources::join(emp_index(dir), as_map(), Vec::new(), Vec::new())
            .expect("two files plus two files must join")
            .0
    }

    /// An AS-side span as it looks AFTER the seam rebased it: file `k` of the AS
    /// map lives at `k + as_base` in the joined space, and `as_base` is the number
    /// of ids the `.emp` index covers (2 here).
    fn rebased_as_diag(chain: &ChainSources, as_file: u32, at: u32, message: &str) -> Diagnostic {
        let d = diag(as_file, at, message);
        Diagnostic { primary: chain.rebase(d.primary), ..d }
    }

    /// THE DEFECT. A `resolve_layout` diagnostic whose span came from an AS-side
    /// section must name the `.asm` file it was written in. Before 2026-09-18 the
    /// chained path located it through the `.emp` manifest's index, which answered
    /// `second.emp` — a real path, a real line, and the wrong file.
    ///
    /// DERIVED, not copied from a neighbouring pin: `sound.asm` is AS id 1 and the
    /// offset is `third_line(ASM_TEXT)`, computed from the fixture, so the expected
    /// render is `sound.asm(3)` and nothing else. The `.emp` fixture's id 1 is a
    /// DIFFERENT PATH, which is what makes this gate read a path rather than an id —
    /// an id would be equal under both authorities and prove nothing.
    #[test]
    fn an_as_side_layout_diagnostic_names_its_own_asm_file() {
        let dir = tempfile::tempdir().unwrap();
        let chain = chained(dir.path());
        let ds = vec![rebased_as_diag(
            &chain,
            1,
            third_line(ASM_TEXT),
            "[layout.overlap] section `snd` overlaps `obj`",
        )];

        let got = render_declared_chain("resolve_layout", &ds, &chain);

        assert!(
            got.contains("sound.asm(3)"),
            "an AS-side span must render through the AS front end's own map, got:\n{got}"
        );
        assert!(
            !got.contains(".emp"),
            "an AS-side span must not be attributed to any `.emp` file, got:\n{got}"
        );
    }

    /// THE CASE THAT ALREADY WORKED, gated so the fix cannot buy the AS side by
    /// spending the `.emp` side. An `.emp` span keeps `path:line:col` against the
    /// manifest's own file.
    #[test]
    fn an_emp_diagnostic_still_names_its_own_emp_file() {
        let dir = tempfile::tempdir().unwrap();
        let chain = chained(dir.path());
        let ds = vec![diag(1, third_line(EMP_TEXT), "[link.unresolved] `Foo`")];

        let got = render_declared_chain("resolve_layout", &ds, &chain);

        assert!(
            got.contains("second.emp:3:1"),
            "an `.emp` span must keep its `path:line:col` under the joined authority, got:\n{got}"
        );
        assert!(!got.contains(".asm"), "an `.emp` span must not reach the AS map, got:\n{got}");
    }

    /// THE MIXED CASE — the normal one for this port, and the one neither
    /// single-front-end gate can see. Two diagnostics that carried THE SAME raw id
    /// on opposite sides of the seam each reach their own file in one render.
    #[test]
    fn mixed_diagnostics_each_reach_their_own_front_end() {
        let dir = tempfile::tempdir().unwrap();
        let chain = chained(dir.path());
        let ds = vec![
            diag(1, third_line(EMP_TEXT), "[link.unresolved] from the emp side"),
            rebased_as_diag(&chain, 1, third_line(ASM_TEXT), "[layout.overlap] from the AS side"),
        ];

        let got = render_declared_chain("resolve_layout", &ds, &chain);

        assert!(got.contains("second.emp:3:1"), "the emp half: {got}");
        assert!(got.contains("sound.asm(3)"), "the AS half: {got}");
        // NON-VACUITY: `render_diag_lines` falls back to `@ Span { .. }` for a span
        // no authority reads, so its absence is the proof that BOTH lines located
        // rather than one locating and the other degrading past the assertions.
        assert!(!got.contains("@ Span"), "both diagnostics must locate: {got}");
    }

    /// The ledger's second half: an AS MACRO EXPANSION id. It is not a file id in
    /// either map — it is in the expansion range by construction — so it is not
    /// rebased, and the AS map renders asl's call-site trail for it. Through the
    /// `.emp` index it named nothing at all.
    #[test]
    fn an_as_expansion_span_renders_its_call_site_trail() {
        let dir = tempfile::tempdir().unwrap();
        let mut asm = as_map();
        // `sound.asm` (id 1) is the body's backing file; the call is on its line 1.
        let run = asm.add_expansion(
            SourceId(1),
            third_line(ASM_TEXT),
            Span { source: SourceId(1), start: 0, end: 0 },
            sigil_span::Frame::Macro("dbgout".into()),
        );
        let chain = ChainSources::join(emp_index(dir.path()), asm, Vec::new(), Vec::new()).unwrap().0;
        assert!(run.is_expansion(), "an expansion id must be in the expansion range");
        let ds = vec![Diagnostic {
            level: Level::Error,
            message: "[layout.overlap] inside a macro".to_string(),
            primary: Span { source: run, start: third_line(ASM_TEXT), end: third_line(ASM_TEXT) },
        }];

        let got = render_declared_chain("resolve_layout", &ds, &chain);

        assert!(got.contains("sound.asm(1)"), "the OUTERMOST call site: {got}");
        assert!(got.contains("dbgout("), "the expansion frame: {got}");
        assert!(!got.contains(".emp"), "an expansion is never an `.emp` file: {got}");
    }

    /// AN UNATTRIBUTED DIAGNOSTIC still renders as a bare message. The no-source id
    /// is in the expansion range, so it routes to the AS map; that map answers
    /// `None` for it exactly as the `.emp` index did, and the rendered line is the
    /// `@ Span { .. }` fallback either way.
    #[test]
    fn an_unattributed_diagnostic_renders_the_same_bare_line() {
        let dir = tempfile::tempdir().unwrap();
        let chain = chained(dir.path());
        let ds = vec![diag(u32::MAX, 0, "[link.internal] no line")];

        let got = render_declared_chain("resolve_layout", &ds, &chain);

        assert!(got.contains("@ Span"), "must degrade to the bare form, got:\n{got}");
        assert!(chain.locate(ds[0].primary).is_none(), "and locate must say so");
    }

    /// THE SINGLE-FRONT-END INVARIANCE, byte-for-byte on the rendered text: a build
    /// with NO AS side joins an EMPTY AS map, `as_base` swallows nothing, and every
    /// `.emp` diagnostic renders the string the `.emp` index alone produced. This is
    /// the gate that says the fix is inert where there is nothing to fix.
    #[test]
    fn with_no_as_side_the_text_is_exactly_what_the_emp_index_renders() {
        let dir = tempfile::tempdir().unwrap();
        let ds = [
            diag(0, third_line(EMP_TEXT), "[a.b] one"),
            diag(1, 0, "[c.d] two"),
            diag(9, 0, "[e.f] off-index"),
        ];

        let before = crate::diag_render::render_diag_lines(
            &ds.iter().collect::<Vec<_>>(),
            &|span| emp_index(dir.path()).locate(span),
        );
        let chain = ChainSources::join(emp_index(dir.path()), SourceMap::new(), Vec::new(), Vec::new())
            .unwrap()
            .0;
        let after = crate::diag_render::render_diag_lines(
            &ds.iter().collect::<Vec<_>>(),
            &|span| chain.locate(span),
        );

        assert_eq!(after, before, "a build with one front end must render identically");
        assert!(before.contains("first.emp:3:1"), "non-vacuity: the comparison had content");
    }

    /// REBASING IS INJECTIVE — the property the warn tier's `(level, message, span)`
    /// dedup key depends on. Distinct AS spans stay distinct, and an AS span can no
    /// longer equal an `.emp` span it used to collide with.
    #[test]
    fn rebasing_separates_the_two_id_spaces_without_merging_anything() {
        let dir = tempfile::tempdir().unwrap();
        let chain = chained(dir.path());
        let emp0 = Span { source: SourceId(0), start: 4, end: 4 };
        let emp1 = Span { source: SourceId(1), start: 4, end: 4 };
        let as0 = chain.rebase(emp0);
        let as1 = chain.rebase(emp1);

        assert_ne!(as0, emp0, "an AS span must not keep an id the `.emp` index answers for");
        assert_ne!(as1, emp1);
        assert_ne!(as0, as1, "distinct AS files stay distinct");
        assert_eq!(as0.start, emp0.start, "only the id moves");
        // And each now reaches its own authority.
        assert!(chain.locate(emp0).unwrap().contains("first.emp"));
        assert!(chain.locate(as0).unwrap().contains("boot.asm"));
    }

    /// LOUD ON UNMEASURABLE: a joined id space that would reach the expansion range
    /// is REFUSED by name. It is never truncated, never wrapped, and never located
    /// against whichever map the arithmetic happens to land in.
    ///
    /// Exercised through `check_joined_space`, which IS the decision
    /// `ChainSources::join` makes, taken as a function of the two sizes — no
    /// `SourceIndex` can be built at 2^31 files, and a gate that could only assert
    /// the underlying predicate would never execute the refusal itself.
    #[test]
    fn a_joined_space_reaching_the_expansion_range_is_refused() {
        use super::check_joined_space;
        assert!(check_joined_space(2, 2).is_ok(), "the fixture-sized join must be accepted");
        // The last accepted total, then the first refused one.
        assert!(check_joined_space(0x7FFF_FFFF, 0).is_ok(), "one below the range is a file id");
        let err = check_joined_space(0x7FFF_FFFF, 1)
            .expect_err("a join reaching the expansion range must be refused");
        assert!(err.contains("[chain.source-id-space]"), "{err}");
        assert!(err.contains("expansion range"), "the refusal must say why: {err}");
        assert!(check_joined_space(usize::MAX, 1).is_err(), "and it cannot be reached by wrapping");
    }

    /// A one-fragment section at `span`, named `name` — the smallest thing that can
    /// stand in for a front end's contribution to the chained list.
    fn one_frag_section(name: &str, span: Span) -> Section {
        use sigil_ir::DataFragment;
        Section {
            name: name.to_string(),
            cpu: sigil_ir::Cpu::M68000,
            vma_base: None,
            lma: 0,
            space: sigil_ir::AddressSpace::Image,
            labels: Vec::new(),
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0],
                fixups: Vec::new(),
                span,
            })],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    /// THE SEAM'S OWN CONTRACT, over both halves at once. `join` returns the AS
    /// sections first and the `.emp` sections after, the AS half moved into the
    /// joined id space and the `.emp` half untouched — so a diagnostic raised over
    /// the RETURNED list reaches the right file whichever half it came from.
    ///
    /// This is the gate that would go red if the rebase were dropped from the seam
    /// while every isolated `ChainSources` gate stayed green: the two spans start
    /// out EQUAL (same id, same range, one per side), and only the seam tells them
    /// apart.
    #[test]
    fn the_seam_returns_the_as_half_rebased_and_the_emp_half_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let shared = Span { source: SourceId(1), start: 0, end: 0 };
        let (chain, all) = ChainSources::join(
            emp_index(dir.path()),
            as_map(),
            vec![one_frag_section("as_side", shared)],
            vec![one_frag_section("emp_side", shared)],
        )
        .expect("the fixture-sized join must be accepted");

        let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["as_side", "emp_side"], "AS sections come first, then `.emp`");
        let span_of = |i: usize| match &all[i].fragments[0] {
            Fragment::Data(d) => d.span,
            other => panic!("fixture fragment changed shape: {other:?}"),
        };
        assert_ne!(span_of(0), shared, "the AS half must have moved");
        assert_eq!(span_of(1), shared, "the `.emp` half must not have moved");
        assert!(
            chain.locate(span_of(0)).is_some_and(|l| l.contains("sound.asm")),
            "the AS half must reach the AS map, got {:?}",
            chain.locate(span_of(0))
        );
        assert!(
            chain.locate(span_of(1)).is_some_and(|l| l.contains("second.emp")),
            "the `.emp` half must reach the `.emp` index, got {:?}",
            chain.locate(span_of(1))
        );
    }

    /// G0a Task 7: BOTH front ends make expansions now, and both number them from 0,
    /// so an `.emp` template expansion and an AS macro expansion start out with THE
    /// SAME id. The seam moves the AS one past the `.emp` ones (as it moves AS files
    /// past the `.emp` files), and each then reaches its own authority: the `.emp`
    /// one locates where its line was written (diagnostics) and its outermost line
    /// is the splice; the AS one renders asl's trail and its outermost line is the
    /// macro call.
    #[test]
    fn emp_and_as_expansions_share_one_joined_range() {
        use sigil_link::export::locate::{SourcePos, SpanLocator};
        let dir = tempfile::tempdir().unwrap();
        let at = third_line(EMP_TEXT);
        // `.emp`: a template body on first.emp's line 3, spliced at second.emp's line 3.
        let mut emp_runs = SourceMap::new();
        let emp_run = emp_runs.add_expansion(
            SourceId(0),
            at,
            Span { source: SourceId(1), start: at, end: at },
            sigil_span::Frame::Template("t".into()),
        );
        let mut sources = std::collections::HashMap::new();
        for (id, name) in [(0u32, "first.emp"), (1, "second.emp")] {
            let p = dir.path().join(name);
            std::fs::write(&p, EMP_TEXT).unwrap();
            sources.insert(SourceId(id), p);
        }
        let manifest = Manifest { modules: Vec::new(), by_id: std::collections::HashMap::new(), sources };
        let index = SourceIndex::with_expansions(&manifest, &emp_runs);
        // AS: a macro body in sound.asm (id 1) called from its line 1.
        let mut asm = as_map();
        let as_run = asm.add_expansion(
            SourceId(1),
            third_line(ASM_TEXT),
            Span { source: SourceId(1), start: 0, end: 0 },
            sigil_span::Frame::Macro("dbgout".into()),
        );
        assert_eq!(emp_run, as_run, "both front ends number expansions from 0: the collision");
        let emp_span = Span { source: emp_run, start: at, end: at };
        let raw_as = Span { source: as_run, start: third_line(ASM_TEXT), end: third_line(ASM_TEXT) };
        let (chain, all) = ChainSources::join(
            index,
            asm,
            vec![one_frag_section("as_side", raw_as)],
            vec![one_frag_section("emp_side", emp_span)],
        )
        .unwrap();
        let span_of = |i: usize| match &all[i].fragments[0] {
            Fragment::Data(d) => d.span,
            other => panic!("fixture fragment changed shape: {other:?}"),
        };
        let as_span = span_of(0);
        assert!(as_span.source.is_expansion() && as_span.source != emp_run, "the AS expansion moved: {as_span:?}");
        assert_eq!(span_of(1), emp_span, "the `.emp` expansion did not move");
        // `.emp`: written in first.emp, spliced at second.emp line 3 (joined file 1).
        assert!(chain.locate(emp_span).is_some_and(|l| l.contains("first.emp:3:1")), "{:?}", chain.locate(emp_span));
        assert_eq!(chain.outermost(emp_span), Some(SourcePos { file: 1, line: 3 }));
        // AS: asl's trail from sound.asm's line 1; sound.asm is joined file 2 + 1.
        let got = chain.locate(as_span).unwrap_or_default();
        assert!(got.contains("sound.asm(1)") && got.contains("dbgout("), "{got}");
        assert_eq!(chain.outermost(as_span), Some(SourcePos { file: 3, line: 1 }));
        // The no-source id still reaches neither.
        let none = Span { source: SourceId(u32::MAX), start: 0, end: 0 };
        assert_eq!((chain.locate(none), chain.outermost(none)), (None, None));
    }

    /// The expansion half of the joined-space refusal, at its boundary: one index
    /// past the last the no-source id leaves free is refused, the last one is not.
    #[test]
    fn the_joined_expansion_range_is_refused_past_the_no_source_id() {
        let limit = (u32::MAX >> 1) as usize; // expansion indices `0 .. limit`
        assert!(check_joined_expansions(limit - 1, 1).is_ok());
        assert!(check_joined_expansions(limit, 1).is_err());
        assert!(check_joined_expansions(0, 0).is_ok());
    }

    /// THE SEAM, and THE FRAGMENT FAMILY in one gate. Handing AS-side sections to
    /// `ChainSources::join` must move EVERY span they carry into the joined space —
    /// every span-carrying `Fragment` variant and the section's equates, not just
    /// the variant the defect was found through.
    ///
    /// It goes through `join` rather than calling `rebase_section` directly because
    /// that is the seam's own contract: a `ChainSources` and un-rebased AS sections
    /// must not be able to coexist. A variant added later is caught by
    /// `rebase_section`'s exhaustive `match` at COMPILE time; this is the run-time
    /// half, and it fails if a variant is added to this list but not to the rebase.
    #[test]
    fn joining_rebases_every_span_an_as_side_section_carries() {
        use sigil_ir::{DataFragment, EquSym, RelaxCandidate};
        use sigil_ir::expr::Expr;
        use sigil_ir::fixup::{Fixup, FixupKind};
        let dir = tempfile::tempdir().unwrap();
        let raw =
            Span { source: SourceId(1), start: third_line(ASM_TEXT), end: third_line(ASM_TEXT) };
        let cand = || RelaxCandidate {
            bytes: vec![0, 0],
            fixup: Fixup { kind: FixupKind::Abs16Be, offset: 0, target: Expr::Int(0) },
        };
        let sec = Section {
            name: "as_side".to_string(),
            cpu: sigil_ir::Cpu::M68000,
            vma_base: None,
            lma: 0,
            space: sigil_ir::AddressSpace::Image,
            labels: Vec::new(),
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0], fixups: Vec::new(), span: raw }),
                Fragment::Fill { value: 0, count: 1, span: raw },
                Fragment::Reserve { count: 1, span: raw },
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Int(0), span: raw },
                Fragment::RelaxAbsSym {
                    short: cand(),
                    long: cand(),
                    target: Expr::Int(0),
                    span: raw,
                },
                Fragment::RelaxLadder { candidates: vec![cand()], target: Expr::Int(0), span: raw },
                Fragment::Org { target: 0, fill: 0, span: raw },
            ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![EquSym { name: "E".to_string(), expr: Expr::Int(0), span: raw }],
        };
        let variants = sec.fragments.len();

        let (chain, out) = ChainSources::join(emp_index(dir.path()), as_map(), vec![sec], Vec::new())
            .expect("the fixture-sized join must be accepted");
        let sec = &out[0];

        let spans: Vec<Span> = sec
            .fragments
            .iter()
            .map(|f| match f {
                Fragment::Data(d) => d.span,
                Fragment::Fill { span, .. }
                | Fragment::Reserve { span, .. }
                | Fragment::JmpJsrSym { span, .. }
                | Fragment::RelaxAbsSym { span, .. }
                | Fragment::RelaxLadder { span, .. }
                | Fragment::Org { span, .. } => *span,
            })
            .chain(sec.equ_syms.iter().map(|e| e.span))
            .collect();
        assert_eq!(spans.len(), variants + 1, "every fragment plus the equate");
        for (i, sp) in spans.iter().enumerate() {
            assert_eq!(
                chain.locate(*sp).as_deref(),
                Some("sound.asm(3):1"),
                "span {i} must reach the AS map after the rebase, not {:?}",
                chain.locate(*sp)
            );
        }
    }
}

#[cfg(test)]
mod budget_tests {
    //! A2: every `[[budget]]` row is enforced against its own `ceiling`, not only the
    //! row whose region is spelled `object_bank`.
    use super::check_budgets;
    use crate::map_placement::load_placement_map;
    use sigil_ir::map::{MemoryMap, Region, RegionKind};
    use sigil_ir::{Cpu, DataFragment, Fragment, Label, Section, SectionPlacement};

    fn sec(label: &str, lma: u32) -> Section {
        Section {
            name: format!("sec{lma}"),
            cpu: Cpu::M68000,
            vma_base: None,
            lma,
            labels: vec![Label { name: label.into(), offset: 0 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0u8; 2],
                fixups: vec![],
                span: sigil_span::Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
            })],
            placement: SectionPlacement::Pinned,
            reserved_span: 2,
            group: None,
            bank: None,
            space: sigil_ir::AddressSpace::Image,
            equ_syms: Vec::new(),
        }
    }

    fn region(name: &str, base: u32, size: u32, kind: RegionKind) -> Region {
        Region { name: name.into(), lma_base: base, size, kind, vma_base: None }
    }

    fn two_budget_map() -> MemoryMap {
        MemoryMap::new(
            vec![
                region("rom", 0, 0x400000, RegionKind::Rom),
                region("object_bank", 0x10000, 0x10000, RegionKind::ObjectBank),
                region("art_bank", 0x40000, 0x8000, RegionKind::ObjectBank),
            ],
            0x00,
        )
    }

    const TWO_ROWS: &str = "[[budget]]\nregion = \"object_bank\"\nceiling = 0x20000\ncursor = \"ObjEnd\"\n\
                            [[budget]]\nregion = \"art_bank\"\nceiling = 0x48000\ncursor = \"ArtEnd\"\n";

    #[test]
    fn every_budget_row_is_enforced() {
        let pmap = load_placement_map(TWO_ROWS).unwrap();
        // Both rows in budget: each reports its used bytes.
        let ok = check_budgets(&[sec("ObjEnd", 0x1128C), sec("ArtEnd", 0x48000)], &two_budget_map(), &pmap)
            .expect("both rows fit");
        assert_eq!(ok, vec![("object_bank".to_string(), 0x128C), ("art_bank".to_string(), 0x8000)]);
        // The second row overflows by one byte: refused, naming the second row's region.
        let err = check_budgets(&[sec("ObjEnd", 0x1128C), sec("ArtEnd", 0x48001)], &two_budget_map(), &pmap)
            .expect_err("the second row overflows by 1");
        assert_eq!(err, "budget region `art_bank` [0x40000,0x48000) overflows, cursor 0x48001 over by 1 bytes");
    }

    /// The cap is the row's declared `ceiling`, not the region's end.
    #[test]
    fn the_cap_is_the_rows_ceiling() {
        let pmap = load_placement_map(
            "[[budget]]\nregion = \"object_bank\"\nceiling = 0x18000\ncursor = \"ObjEnd\"\n",
        )
        .unwrap();
        let err = check_budgets(&[sec("ObjEnd", 0x18002)], &two_budget_map(), &pmap).unwrap_err();
        assert_eq!(err, "budget region `object_bank` [0x10000,0x18000) overflows, cursor 0x18002 over by 2 bytes");
    }

    /// A budget that cannot be measured is refused, never read as within budget: a row
    /// naming no map region, a row with no cursor, a cursor label the layout lacks.
    #[test]
    fn an_unmeasurable_budget_row_is_refused() {
        let layout = [sec("ObjEnd", 0x11000)];
        let m = two_budget_map();
        let no_region =
            load_placement_map("[[budget]]\nregion = \"nope\"\nceiling = 0x20000\ncursor = \"ObjEnd\"\n").unwrap();
        let e = check_budgets(&layout, &m, &no_region).unwrap_err();
        assert!(e.starts_with("[map.budget-unmeasurable] budget row `nope`"), "{e}");
        let no_cursor = load_placement_map("[[budget]]\nregion = \"object_bank\"\nceiling = 0x20000\n").unwrap();
        let e = check_budgets(&layout, &m, &no_cursor).unwrap_err();
        assert!(e.starts_with("[map.budget-unmeasurable] budget row `object_bank`"), "{e}");
        let absent = load_placement_map(
            "[[budget]]\nregion = \"object_bank\"\nceiling = 0x20000\ncursor = \"Missing\"\n",
        )
        .unwrap();
        let e = check_budgets(&layout, &m, &absent).unwrap_err();
        assert!(e.contains("`Missing`"), "{e}");
    }
}
