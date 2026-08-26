//! The DERIVED game-contract env covers aeon's whole `interface Game` — the gate
//! that makes the NEXT hook a non-event.
//!
//! ## What went wrong, and why a gate exists here
//!
//! aeon's ring-sparkle parcel added ONE member to
//! `engine/system/game_contract.emp` (`hook ring_collected`) and one
//! `invoke Game.ring_collected` to `engine/objects/rings.emp`. Two sigil port
//! gates went red with `[contract.unknown-member]`, because the oracle that
//! lowers `rings.emp` STANDALONE saw no contract at all, and its sibling oracles
//! see only a HAND-WRITTEN interface stub — a second copy of the contract that
//! cannot grow when the contract grows. The same class as CLOSURE-2: a
//! hand-maintained list in a test rig that cannot see new content.
//!
//! `test_support::game_contract_env_from_aeon` parses aeon's own contract
//! instead. This file is the proof that the derived env actually COVERS it, for
//! every shipped game, with the expectation READ OUT OF THE CONTRACT — a
//! hand-listed member count here would be the very defect one layer up.
//!
//! ## Source-only
//!
//! Every gate here reads aeon `.emp` SOURCE (`engine/system/game_contract.emp`
//! and each game's `games/<g>/config/game.emp`). Nothing is built, nothing is
//! compared to a committed artifact — this belongs in the nightly source lane.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test game_contract_env_coverage
//! ```

use sigil_frontend_emp::contract::{InterfaceEnv, ResolvedInterface};
use sigil_harness::native::{demo_profile, sonic4_profile, GameProfile};
use sigil_harness::test_support::{
    game_contract_declared_members, game_contract_env_from_aeon, game_contract_missing_members,
    game_manifest_path, reference_tree_for_profile, GAME_CONTRACT_IFACE_REL,
};

/// The shipped games, each at the build shape whose manifest bindings are the
/// plainest (canonical, hotkeys off). Both must bind the SAME interface — the
/// contract is engine-owned, one declaration for every game.
fn shipped() -> Vec<GameProfile> {
    vec![sonic4_profile(false), sonic4_profile(true), demo_profile(false), demo_profile(true)]
}

/// A profile's comptime environment, as the profile itself declares it.
fn defines(profile: &GameProfile) -> Vec<(String, i128)> {
    profile.emp_defines.iter().map(|(n, v)| (n.to_string(), *v)).collect()
}

/// THE GATE. For every shipped game, the env derived from aeon's own contract
/// carries EVERY member `interface Game` declares — expectation derived by
/// parsing the interface, never a count written here.
///
/// A member the engine adds and a game leaves at its `= empty` default is still
/// COVERED: the bind pass resolves it to an unbound hook, which is what an
/// `invoke` at a zero-byte call site needs. What this refuses is a member the
/// env does not know at all — the shape that made `rings.emp` fail at lower.
#[test]
fn derived_env_covers_every_declared_member() {
    let Some(aeon) = reference_tree_for_profile(&sonic4_profile(false)) else { return };
    let declared = game_contract_declared_members(&aeon);
    for profile in shipped() {
        let env = game_contract_env_from_aeon(&aeon, &profile, &defines(&profile));
        let missing = game_contract_missing_members(&aeon, &env);
        assert!(
            missing.is_empty(),
            "`{}`: the derived env misses {:?} of the {} members declared in {}",
            profile.name,
            missing,
            declared.len(),
            aeon.join(GAME_CONTRACT_IFACE_REL).display()
        );
    }
}

/// NON-VACUITY, standing red-first. The coverage predicate above passes on a
/// COMPLETE env; this proves it FAILS on an incomplete one, and names the member
/// it lost. Without this, a predicate that silently answered "nothing missing"
/// for every input would read exactly like the gate above passing.
///
/// The member removed is not named here either: it is the FIRST one the contract
/// declares, so this probe follows the contract too.
#[test]
fn a_member_filtered_out_of_the_env_is_reported_by_name() {
    let Some(aeon) = reference_tree_for_profile(&sonic4_profile(false)) else { return };
    let declared = game_contract_declared_members(&aeon);
    let profile = sonic4_profile(false);
    let env = game_contract_env_from_aeon(&aeon, &profile, &defines(&profile));

    for victim in &declared {
        let game = env.interfaces.get("Game").expect("the derived env carries `Game`");
        let members = game
            .members
            .iter()
            .filter(|(n, _)| *n != victim)
            .map(|(n, m)| (n.clone(), m.clone()))
            .collect();
        let mut doctored = InterfaceEnv::empty();
        doctored.interfaces.insert("Game".to_string(), ResolvedInterface { members });

        let missing = game_contract_missing_members(&aeon, &doctored);
        assert_eq!(
            missing,
            vec![victim.clone()],
            "filtering `{victim}` out of the env must be reported as exactly that member missing"
        );
    }
    assert!(
        !declared.is_empty(),
        "the probe swept no members — an empty contract would make it vacuous"
    );
}

/// The paths the derived env reads are the ones the REAL BUILD reads: each
/// profile's manifest is the file its declared `manifest_module` names, at the
/// place the profile's own residual root puts it. A manifest that moved without
/// the derivation moving is caught here rather than as a mystery bind failure.
#[test]
fn every_shipped_manifest_is_where_the_profile_says() {
    let Some(aeon) = reference_tree_for_profile(&sonic4_profile(false)) else { return };
    assert!(
        aeon.join(GAME_CONTRACT_IFACE_REL).is_file(),
        "the engine contract must be at {}",
        aeon.join(GAME_CONTRACT_IFACE_REL).display()
    );
    for profile in shipped() {
        let path = game_manifest_path(&aeon, &profile);
        assert!(
            path.is_file(),
            "`{}`: no manifest at {} (derived from game_root_rel `{}`)",
            profile.name,
            path.display(),
            profile.game_root_rel
        );
        let src = std::fs::read_to_string(&path).expect("manifest reads");
        let (file, _) = sigil_frontend_emp::parse_str(&src);
        assert_eq!(
            file.module.path.segments.join("."),
            profile.manifest_module,
            "`{}`: {} declares a different module than the profile names",
            profile.name,
            path.display()
        );
    }
}
