//! Closed module gating: every module a build would place matches exactly one
//! `[[gate]]` rule, and the rule says which shapes place it.
//!
//! Spec §5.1 ("Module gating ... Every placed module matches exactly one rule, `all`
//! included; no match is an error") and plan Review Focus 1: the old driver's
//! `SHAPE_GATES` lists only the modules SOME shapes drop, so a module added with no row
//! silently ships in every shape. Here no row is a refusal ([`BuildError::Ungated`](crate::driver::BuildError::Ungated)), and
//! so is a module two rows match ([`BuildError::DoubleGated`](crate::driver::BuildError::DoubleGated)): two rules for one module
//! is two answers to one question, and rule order is not an answer.
//!
//! ── PATTERNS ──
//!
//! A rule's `modules` is a module id or a glob over ids, matched against the WHOLE id:
//!
//!   * `*` matches any run of characters inside one dotted segment (never a `.`);
//!   * `**` matches any run of characters, dots included;
//!   * every other character matches itself.
//!
//! So `games.sonic4.ojz_*_act1` names the generated act modules and nothing deeper, and
//! `engine.**` names the whole engine tree. A pattern with any other wildcard spelling
//! (`***`, `?`, `[`) is refused when the rule is read ([`check_pattern`](crate::gating::check_pattern)).

use crate::describe::{GateRule, PlaceIn, ResolvedShape};
use crate::driver::BuildError;

/// A module id (`engine.vdp`, `games.sonic4.test_player`).
pub type ModuleId = String;

/// Refuse a pattern spelling the matcher does not define. The error names the pattern.
pub fn check_pattern(pattern: &str) -> Result<(), String> {
    if pattern.is_empty() {
        return Err("a [[gate]] rule's `modules` is empty".into());
    }
    if pattern.contains("***") {
        return Err(format!("[[gate]] pattern `{pattern}`: `***` is not a wildcard (use `*` or `**`)"));
    }
    if let Some(c) = pattern.chars().find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '*'))) {
        return Err(format!(
            "[[gate]] pattern `{pattern}`: `{c}` is not part of a module id or a wildcard (`*` within one \
             segment, `**` across segments)"
        ));
    }
    Ok(())
}

/// Does `pattern` match the whole of `id`? See the module header for the syntax.
pub fn glob_match(pattern: &str, id: &str) -> bool {
    fn go(p: &[u8], s: &[u8]) -> bool {
        match p {
            [] => s.is_empty(),
            [b'*', b'*', rest @ ..] => (0..=s.len()).any(|k| go(rest, &s[k..])),
            [b'*', rest @ ..] => {
                // One segment: never consume a dot.
                let mut k = 0;
                loop {
                    if go(rest, &s[k..]) {
                        return true;
                    }
                    if k == s.len() || s[k] == b'.' {
                        return false;
                    }
                    k += 1;
                }
            }
            [c, rest @ ..] => s.first() == Some(c) && go(rest, &s[1..]),
        }
    }
    go(pattern.as_bytes(), id.as_bytes())
}

/// The modules of `scan` that `shape` places, in `scan` order.
///
/// Each module must match exactly one of `rules` (by [`glob_match`] on its `modules`):
/// none is [`BuildError::Ungated`], two or more is [`BuildError::DoubleGated`] naming the
/// first two rule indices. The one rule then decides: `all` places it, `shapes = [..]`
/// places it in exactly the listed shapes (`[]` in none), `feature = ".."` places it when
/// the expression holds for the shape's features and defines (`shape.defines` must be the
/// fully merged set, engine defaults included; a define the expression reads that the
/// shape does not have is an error, never false).
pub fn gate_modules(scan: &[ModuleId], rules: &[GateRule], shape: &ResolvedShape) -> Result<Vec<ModuleId>, BuildError> {
    let mut kept = Vec::new();
    for id in scan {
        let mut hits = rules.iter().enumerate().filter(|(_, r)| glob_match(&r.modules, id)).map(|(i, _)| i);
        let Some(first) = hits.next() else {
            return Err(BuildError::Ungated(id.clone()));
        };
        if let Some(second) = hits.next() {
            return Err(BuildError::DoubleGated(id.clone(), first, second));
        }
        let placed = match &rules[first].place_in {
            PlaceIn::All => true,
            PlaceIn::Shapes(names) => names.contains(&shape.name),
            PlaceIn::Feature(expr) => expr
                .eval(&shape.features, &|d: &str| shape.defines.get(d).copied())
                .map_err(|m| BuildError::Gate(format!("rule {first} (`{}`) for `{id}`: {m}", rules[first].modules)))?,
        };
        if placed {
            kept.push(id.clone());
        }
    }
    Ok(kept)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::describe::{FeatureExpr, Features};
    use std::collections::BTreeMap;

    fn rule(modules: &str, place_in: PlaceIn) -> GateRule {
        GateRule { modules: modules.into(), place_in }
    }

    fn shape(name: &str, debug: bool) -> ResolvedShape {
        ResolvedShape {
            name: name.into(),
            features: Features { debug, crash_report: true, sound: false },
            defines: BTreeMap::from([("HOTKEYS".to_string(), 0)]),
            fixture: false,
            fixture_placement: None,
        }
    }

    fn ids(v: &[&str]) -> Vec<ModuleId> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// Review Focus 1: a module with no rule is refused by name, never shipped in every shape.
    #[test]
    fn unmatched_module_is_refused() {
        let rules = vec![
            rule("games.g.objects.*", PlaceIn::All),
            rule("games.g.test_player", PlaceIn::Feature(FeatureExpr::parse("debug").unwrap())),
        ];
        let scan = ids(&["games.g.objects.ring", "games.g.test_player", "games.g.foo"]);
        assert_eq!(gate_modules(&scan, &rules, &shape("release", false)), Err(BuildError::Ungated("games.g.foo".into())));
        // Control: with the module named, the same scan gates.
        let mut named = rules.clone();
        named.push(rule("games.g.foo", PlaceIn::All));
        assert_eq!(
            gate_modules(&scan, &named, &shape("release", false)).unwrap(),
            ids(&["games.g.objects.ring", "games.g.foo"])
        );
    }

    /// A module two rules match is refused naming both rules.
    #[test]
    fn double_gated_module_is_refused() {
        let rules = vec![
            rule("games.g.**", PlaceIn::All),
            rule("games.g.test_player", PlaceIn::Feature(FeatureExpr::parse("debug").unwrap())),
        ];
        let scan = ids(&["games.g.ring", "games.g.test_player"]);
        assert_eq!(
            gate_modules(&scan, &rules, &shape("debug", true)),
            Err(BuildError::DoubleGated("games.g.test_player".into(), 0, 1))
        );
        // Control: disjoint rules gate the same scan.
        let disjoint = vec![rules[1].clone(), rule("games.g.ring", PlaceIn::All)];
        assert_eq!(gate_modules(&scan, &disjoint, &shape("debug", true)).unwrap(), scan);
    }

    #[test]
    fn shapes_feature_and_define_rules_decide() {
        let rules = vec![
            rule("a", PlaceIn::Shapes(vec!["debug".into()])),
            rule("b", PlaceIn::Shapes(vec![])),
            rule("c", PlaceIn::Feature(FeatureExpr::parse("!debug").unwrap())),
            rule("d", PlaceIn::Feature(FeatureExpr::parse("HOTKEYS").unwrap())),
        ];
        let scan = ids(&["a", "b", "c", "d"]);
        assert_eq!(gate_modules(&scan, &rules, &shape("debug", true)).unwrap(), ids(&["a"]));
        assert_eq!(gate_modules(&scan, &rules, &shape("release", false)).unwrap(), ids(&["c"]));
        let missing = vec![rule("e", PlaceIn::Feature(FeatureExpr::parse("NOPE").unwrap()))];
        assert!(matches!(gate_modules(&ids(&["e"]), &missing, &shape("x", false)), Err(BuildError::Gate(m)) if m.contains("NOPE")));
    }

    #[test]
    fn glob_segments() {
        assert!(glob_match("games.s.ojz_*_act1", "games.s.ojz_sec_grid_act1"));
        assert!(!glob_match("games.s.*", "games.s.data.x"));
        assert!(glob_match("games.s.**", "games.s.data.x"));
        assert!(glob_match("engine.vdp", "engine.vdp"));
        assert!(!glob_match("engine.vdp", "engine.vdp_init"));
        assert!(glob_match("*", "engine") && !glob_match("*", "engine.x"));
        assert!(check_pattern("games.s.**").is_ok());
        assert!(check_pattern("games.s.***").is_err());
        assert!(check_pattern("games.s.te?t").is_err());
    }
}
