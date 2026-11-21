//! A10 W5 at the `sigil build` level: a game's interface binding to a proc that declares no
//! `clobbers(...)` clause stops a full gated build with `[contract.binding-uncontracted]`.
//!
//! `crates/sigil-frontend-emp/tests/game_contract.rs` pins the refusal and its limits against
//! the bind pass. This file proves that the refusal REACHES the real driver: a copy of the
//! reference tree with one planted `hook boot_hook = ...` binding in demo's `implement Game`
//! block (the bound proc planted in `demo_state.emp`), built with `sigil build --project ... -o ...`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon \
//!   cargo test --release -p sigil-cli --test binding_uncontracted_gate
//! ```

#[path = "support/gate_tree.rs"]
mod gate_tree;

const GAME: &str = "games/demo/config/game.emp";
/// Where the bound proc is declared: demo's state module, where a planted proc is placed
/// with the rest of the game's code (a proc appended to `game.emp` would head the map).
const STATE: &str = "games/demo/demo_state.emp";
/// The last binding of demo's `implement Game` block and its closing brace.
const ANCHOR: &str = "    proc entry = GameState_Demo_Init\n}\n";
const UNCONTRACTED: &str = "[contract.binding-uncontracted]";

/// A fresh tree copy whose demo binds `boot_hook` to `Zz_A10_Bound`, declared with `clause`
/// (empty for none). Deleted when the guard drops. `None` when no reference tree is present.
fn planted(clause: &str) -> Option<gate_tree::GateTree> {
    let tree = gate_tree::GateTree::new("binding_uncontracted_gate")?;
    let src = tree.read(GAME);
    assert_eq!(src.matches(ANCHOR).count(), 1, "{GAME}: the plant's anchor moved");
    let src = src.replace(ANCHOR, "    proc entry = GameState_Demo_Init\n    hook boot_hook = Zz_A10_Bound\n}\n");
    tree.write(GAME, &src);
    let state = tree.read(STATE);
    tree.write(STATE, &format!("{state}\npub proc Zz_A10_Bound (){clause} {{\n        rts\n}}\n"));
    Some(tree)
}

#[test]
fn a_binding_to_a_proc_with_no_clause_stops_the_build() {
    let Some(tree) = planted("") else { return };
    let (out, err, rom) = tree.build(&["-o"], &[]);
    assert!(!out.status.success(), "the build must refuse:\n{err}");
    assert!(
        err.lines().any(|l| l.contains(UNCONTRACTED) && l.contains("hook `Game.boot_hook` is bound to `Zz_A10_Bound`")),
        "{err}"
    );
    assert!(!rom.exists(), "a refused build writes no ROM");
}

/// The control: the same binding to a proc declaring `clobbers()` builds, as does the
/// unplanted tree.
#[test]
fn the_same_binding_with_a_clause_builds_and_so_does_the_unplanted_tree() {
    let Some(tree) = planted(" clobbers()") else { return };
    let (out, err, rom) = tree.build(&["-o"], &[]);
    assert!(out.status.success(), "{err}");
    assert!(!err.contains(UNCONTRACTED), "{err}");
    assert!(rom.exists());
    drop(tree);

    let Some(tree) = gate_tree::GateTree::new("binding_uncontracted_gate") else { return };
    let (out, err, rom) = tree.build(&["-o"], &[]);
    assert!(out.status.success(), "{err}");
    assert!(!err.contains(UNCONTRACTED), "{err}");
    assert!(rom.exists());
}
