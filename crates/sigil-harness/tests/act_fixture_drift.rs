//! The `Act_*`/`Sec_*`/`DMAEntry_*`/`parallax_config_*` supply fixture must track
//! the `.emp` structs it claims as its source of truth (lens sweep, seat B2, S16).
//!
//! `test_support::act_sec_field_equs` says "SOURCE OF TRUTH: engine/structs.emp"
//! and is hand-maintained, and it drifted: `Act` gained `act_sec_local_maps` ($22)
//! and `act_art_budget` ($26), so `Act_len` became $28 — while the fixture still
//! said 34 bytes / $22, six bytes and two fields stale. It is consumed by 12 port
//! gates. It stayed inert only because no live `.emp` externs `Act_len` today.
//!
//! `harvest_engine_struct_offsets` already computes these offsets from the live
//! structs for the real build. This gate simply makes the fixture answer to it —
//! the same shape as `pins.rs` + `repin_pins.rs`: a generated authority plus a
//! staleness gate over the hand-written copy.
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()))
}
fn strict() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The struct prefixes the fixture claims to supply.
const PREFIXES: &[&str] = &["Act_", "Sec_", "DMAEntry_", "parallax_config_"];

fn harvested() -> Option<Vec<(String, i64)>> {
    let aeon = aeon_dir();
    if !aeon.join("engine/structs.emp").exists() {
        assert!(!strict(), "SIGIL_STRICT_GATE set but aeon tree not at {}", aeon.display());
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return None;
    }
    Some(sigil_harness::native::harvest_engine_struct_offsets(&aeon).expect("harvest must succeed"))
}

/// The fixture spells values BOTH ways — `"$0A"` and `"10"` — because it grew in
/// two eras. A `$` prefix means hex; a bare literal is decimal, exactly as the AS
/// equs it supplies would read them. Assuming hex throughout turns every decimal
/// entry into a phantom drift report.
fn parse_equ(s: &str) -> i64 {
    let t = s.trim();
    match t.strip_prefix('$') {
        Some(h) => i64::from_str_radix(h, 16)
            .unwrap_or_else(|e| panic!("fixture value `{s}` is not hex: {e}")),
        None => t.parse().unwrap_or_else(|e| panic!("fixture value `{s}` is not decimal: {e}")),
    }
}

/// Every value the fixture supplies must equal the live struct's.
#[test]
fn fixture_values_match_the_live_structs() {
    let Some(live) = harvested() else { return };
    let mut wrong = Vec::new();
    for (name, val) in sigil_harness::test_support::act_sec_field_equs() {
        if !PREFIXES.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        if let Some((_, want)) = live.iter().find(|(n, _)| n == name) {
            let got = parse_equ(val);
            if got != *want {
                wrong.push(format!("{name}: fixture {got:#04x}, struct {want:#04x}"));
            }
        }
    }
    assert!(wrong.is_empty(), "the supply fixture has drifted from engine/structs.emp: {wrong:?}");
}

/// ...and it must not OMIT a field the structs declare. This is the half that
/// matters: the drift happened by two fields being ADDED to `Act` and never
/// reaching the fixture, which a value-only comparison cannot see.
#[test]
fn fixture_covers_every_field_the_structs_declare() {
    let Some(live) = harvested() else { return };
    let have: std::collections::BTreeSet<&str> =
        sigil_harness::test_support::act_sec_field_equs().into_iter().map(|(n, _)| n).collect();
    let missing: Vec<&String> = live
        .iter()
        .map(|(n, _)| n)
        .filter(|n| PREFIXES.iter().any(|p| n.starts_with(p)))
        .filter(|n| !have.contains(n.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "engine/structs.emp declares fields the supply fixture does not carry: {missing:?}"
    );
}
