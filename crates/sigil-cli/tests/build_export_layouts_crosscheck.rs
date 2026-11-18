//! G0a Task 8 (fix round 1): the layout/constant CROSS-CHECK, A4's two-layouts detector
//! half, over the 7 shipped shapes.
//!
//! The export itself reads each item once, in its owner's view (`LayoutPass::Owner`); a
//! build pays nothing when it writes no sidecar. What the default cannot see is an item
//! that two modules resolved to different answers. This gate runs the full pass
//! (`LayoutPass::Full`: every importer's view of every item) over each shape and requires
//! that nobody disagrees and that the full pass states exactly what the owner pass does.
//!
//! RUNNER: `scripts/nightly_source_gates.sh` (`SOURCE_GATES`, listed as
//! `build_export_layouts_crosscheck`) and `cargo test -p sigil-cli --test
//! build_export_layouts_crosscheck`. Real builds run over a COPY of the reference tree
//! (`AEON_DIR`; `SIGIL_STRICT_GATE` hard-fails a missing tree).
//!
//! The control is part of the gate: `LayoutPass::FullSkewedForTest` plants one
//! disagreement between views, and the gate requires BOTH refusals (`TwoLayouts`,
//! `TwoConsts`) to fire on it, so a cross-check that has gone blind fails here.

use sigil_frontend_emp::export_side::{emp_consts, emp_layouts, EmpExportError, LayoutPass};
use sigil_harness::native::{self, GameProfile};
use std::path::PathBuf;
use std::process::Command;

fn tree() -> Option<PathBuf> {
    let profile = native::sonic4_profile(false);
    let aeon = sigil_harness::test_support::reference_tree_for_profile(&profile)?;
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_export_crosscheck_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let tree = dir.join("tree");
    let st = Command::new("cp").arg("-a").arg(&aeon).arg(&tree).status().expect("cp");
    assert!(st.success());
    Some(tree.canonicalize().unwrap())
}

/// The seven shapes `scripts/step0_shapes_cmp.sh` proves byte-identical.
type Shape = (&'static str, fn() -> GameProfile);

fn shapes() -> Vec<Shape> {
    vec![
        ("plain", || native::sonic4_profile(false)),
        ("debug", || native::sonic4_profile(true)),
        ("demo", || native::demo_profile(false)),
        ("demo_debug", || native::demo_profile(true)),
        ("config_a", native::config_a_profile),
        ("config_b", native::config_b_profile),
        ("lean", native::lean_profile),
    ]
}

#[test]
fn no_two_views_disagree_in_any_shape_and_the_control_is_not_blind() {
    let Some(tree) = tree() else {
        eprintln!("{}no aeon reference tree", sigil_harness::test_support::SKIP_MARKER);
        return;
    };
    let mut census = Vec::new();
    for (name, profile) in shapes() {
        let build = |pass| native::build_emp(&tree, &profile().with_layout_pass(pass)).unwrap_or_else(|e| panic!("{name}: {e}")).layouts;
        let owner = build(LayoutPass::Owner);
        let full = build(LayoutPass::Full);
        let (l_owner, l_full) = (emp_layouts(&owner), emp_layouts(&full));
        let (c_owner, c_full) = (emp_consts(&owner), emp_consts(&full));
        let l = l_full.as_ref().unwrap_or_else(|e| panic!("{name}: two views disagree on a layout: {e:?}"));
        let c = c_full.as_ref().unwrap_or_else(|e| panic!("{name}: two views disagree on a constant: {e:?}"));
        assert!(!l.is_empty() && !c.is_empty(), "{name}: the full pass resolved nothing, so it checked nothing");
        assert_eq!(&l_owner.unwrap(), l, "{name}: the owner pass and the full pass state different layouts");
        assert_eq!(&c_owner.unwrap(), c, "{name}: the owner pass and the full pass state different constants");
        census.push(format!("{name}: {} layouts, {} consts", l.len(), c.len()));

        if name == "plain" {
            let skew = build(LayoutPass::FullSkewedForTest);
            assert!(
                matches!(emp_layouts(&skew), Err(EmpExportError::TwoLayouts { .. })),
                "control: a planted layout disagreement was not refused"
            );
            assert!(
                matches!(emp_consts(&skew), Err(EmpExportError::TwoConsts { .. })),
                "control: a planted constant disagreement was not refused"
            );
        }
    }
    eprintln!("CROSSCHECK 7/7 shapes agree: {}", census.join("; "));
    assert_eq!(census.len(), 7);
}
