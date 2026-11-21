//! Closed module gating: every module a build would place matches exactly one
//! `[[gate]]` rule, and the rule says which shapes place it.
//!
//! Spec §5.1 ("Module gating ... Every placed module matches exactly one rule, `all`
//! included; no match is an error") and plan Review Focus 1: the old driver's
//! `SHAPE_GATES` (deleted with it, G0b Task 11) listed only the modules SOME shapes drop,
//! so a module added with no row silently shipped in every shape. Here no row is a refusal ([`BuildError::Ungated`](crate::driver::BuildError::Ungated)), and
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
//! (`***`, `?`, `[`), or an empty one, is refused when the description is read
//! ([`check_pattern`](crate::gating::check_pattern), called by the loaders for every row).

use std::path::PathBuf;

use crate::describe::{GateRule, PlaceIn, ResolvedShape};
use crate::driver::{BuildError, RuleRef};

/// A module id (`engine.vdp`, `games.sonic4.test_player`).
pub type ModuleId = String;

/// A module the gate is asked about: its id and the `.emp` file that declares it, so a
/// refusal can name the file to look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedModule {
    pub id: ModuleId,
    pub path: PathBuf,
    /// The named root the module was scanned under (`None` outside every root), which
    /// says which description its rule belongs in.
    pub root: Option<sigil_span::roots::RootName>,
}

/// The two description files a build's `[[gate]]` rules come from, as a refusal names
/// them: the engine's `engine.toml` and the game's `game.toml`. Empty for a caller with
/// no description files (rules built in code).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GateHomes {
    pub engine: String,
    pub game: String,
}

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
/// none is [`BuildError::Ungated`], naming the module's file, both description files
/// (`homes`, plus the origin of any other rule) and, by the root the module was scanned
/// under, the one its rule goes in; two or more is [`BuildError::DoubleGated`], naming the first
/// two matching rules by pattern, file and row. The one rule then decides: `all` places it, `shapes = [..]`
/// places it in exactly the listed shapes (`[]` in none), `feature = ".."` places it when
/// the expression holds for the shape's features and defines (`shape.defines` must be the
/// fully merged set, engine defaults included; a define the expression reads that the
/// shape does not have is an error, never false).
pub fn gate_modules(
    scan: &[ScannedModule],
    rules: &[GateRule],
    homes: &GateHomes,
    shape: &ResolvedShape,
) -> Result<Vec<ModuleId>, BuildError> {
    let mut kept = Vec::new();
    for ScannedModule { id, path, root } in scan {
        let mut hits = rules.iter().enumerate().filter(|(_, r)| glob_match(&r.modules, id)).map(|(i, _)| i);
        let Some(first) = hits.next() else {
            // Both description files, whether or not either has a rule yet: a game with no
            // `[[gate]]` rows must still be pointed at its own game.toml (G0b Task 13).
            let mut searched: Vec<String> =
                [&homes.engine, &homes.game].into_iter().filter(|h| !h.is_empty()).cloned().collect();
            for r in rules {
                let o = if r.origin.is_empty() { "(rules built in code)".to_string() } else { r.origin.clone() };
                if !searched.contains(&o) {
                    searched.push(o);
                }
            }
            // Where this module's rule goes, by the root it was scanned under.
            let home = match root {
                Some(sigil_span::roots::RootName::Engine) => Some((false, homes.engine.clone())),
                Some(sigil_span::roots::RootName::Game) => Some((true, homes.game.clone())),
                _ => None,
            }
            .filter(|(_, h)| !h.is_empty());
            return Err(BuildError::Ungated { module: id.clone(), path: path.clone(), searched, home });
        };
        if let Some(second) = hits.next() {
            return Err(BuildError::DoubleGated {
                module: id.clone(),
                path: path.clone(),
                first: Box::new(RuleRef::of(&rules[first])),
                second: Box::new(RuleRef::of(&rules[second])),
            });
        }
        let placed = match &rules[first].place_in {
            PlaceIn::All => true,
            PlaceIn::Shapes(names) => names.contains(&shape.name),
            PlaceIn::Feature(expr) => expr
                .eval(&shape.features, &|d: &str| shape.defines.get(d).copied())
                .map_err(|m| BuildError::Gate(format!("rule {} for `{id}`: {m}", RuleRef::of(&rules[first]))))?,
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
    use sigil_span::roots::RootName;
    use std::collections::BTreeMap;

    fn homes() -> GateHomes {
        GateHomes { engine: "engine/engine.toml".into(), game: "games/g/game.toml".into() }
    }

    fn rule(modules: &str, place_in: PlaceIn) -> GateRule {
        GateRule { modules: modules.into(), place_in, origin: String::new(), row: 0 }
    }

    /// `rules` as the loaders stamp them: each from `origin`, numbered from 1 within it.
    fn from_file(origin: &str, rules: Vec<GateRule>) -> Vec<GateRule> {
        rules
            .into_iter()
            .enumerate()
            .map(|(i, r)| GateRule { origin: origin.into(), row: i + 1, ..r })
            .collect()
    }

    fn scanned(v: &[&str]) -> Vec<ScannedModule> {
        v.iter()
            .map(|s| ScannedModule {
                id: s.to_string(),
                path: format!("src/{}.emp", s.replace('.', "/")).into(),
                root: Some(if s.starts_with("engine.") { RootName::Engine } else { RootName::Game }),
            })
            .collect()
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
        let mut rules = from_file("engine/engine.toml", vec![rule("engine.vdp", PlaceIn::All)]);
        rules.extend(from_file(
            "games/g/game.toml",
            vec![
                rule("games.g.objects.*", PlaceIn::All),
                rule("games.g.test_player", PlaceIn::Feature(FeatureExpr::parse("debug").unwrap())),
            ],
        ));
        let scan = scanned(&["games.g.objects.ring", "games.g.test_player", "games.g.foo"]);
        let err = gate_modules(&scan, &rules, &homes(), &shape("release", false)).unwrap_err();
        assert_eq!(
            err,
            BuildError::Ungated {
                module: "games.g.foo".into(),
                path: "src/games/g/foo.emp".into(),
                searched: vec!["engine/engine.toml".into(), "games/g/game.toml".into()],
                home: Some((true, "games/g/game.toml".into())),
            }
        );
        // The message names the module, its .emp file and both description files searched.
        let msg = err.to_string();
        for want in ["games.g.foo", "src/games/g/foo.emp", "engine/engine.toml", "games/g/game.toml"] {
            assert!(msg.contains(want), "the refusal does not name `{want}`: {msg}");
        }
        // Control: with the module named, the same scan gates.
        let mut named = rules.clone();
        named.push(rule("games.g.foo", PlaceIn::All));
        assert_eq!(
            gate_modules(&scan, &named, &homes(), &shape("release", false)).unwrap(),
            ids(&["games.g.objects.ring", "games.g.foo"])
        );
    }

    /// G0b Task 13 friction item 6: a game with NO `[[gate]]` rows whose map places one of
    /// its own modules. The refusal used to list only the files that already had rules, so
    /// it pointed the game's author at engine.toml alone. It names both descriptions and
    /// says the game module's rule goes in the game's file; an engine module's in the
    /// engine's.
    #[test]
    fn an_ungated_module_is_pointed_at_its_own_description() {
        let rules = from_file("engine/engine.toml", vec![rule("engine.vdp", PlaceIn::All)]);
        let scan = scanned(&["engine.vdp", "games.g.header"]);
        let msg = gate_modules(&scan, &rules, &homes(), &shape("release", false)).unwrap_err().to_string();
        assert!(
            msg.contains("matches no [[gate]] rule in `engine/engine.toml` or `games/g/game.toml`; a game module's rule goes in `games/g/game.toml`"),
            "{msg}"
        );
        let scan = scanned(&["engine.vdp", "engine.ram"]);
        let msg = gate_modules(&scan, &rules, &homes(), &shape("release", false)).unwrap_err().to_string();
        assert!(
            msg.contains("matches no [[gate]] rule in `engine/engine.toml` or `games/g/game.toml`; an engine module's rule goes in `engine/engine.toml`"),
            "{msg}"
        );
    }

    /// A module two rules match is refused naming both rules.
    #[test]
    fn double_gated_module_is_refused() {
        let mut rules = from_file("engine/engine.toml", vec![rule("games.g.**", PlaceIn::All)]);
        rules.extend(from_file(
            "games/g/game.toml",
            vec![
                rule("games.g.ring", PlaceIn::All),
                rule("games.g.test_player", PlaceIn::Feature(FeatureExpr::parse("debug").unwrap())),
            ],
        ));
        let scan = scanned(&["games.g.test_player"]);
        let err = gate_modules(&scan, &rules, &homes(), &shape("debug", true)).unwrap_err();
        assert_eq!(
            err,
            BuildError::DoubleGated {
                module: "games.g.test_player".into(),
                path: "src/games/g/test_player.emp".into(),
                first: Box::new(RuleRef { pattern: "games.g.**".into(), origin: "engine/engine.toml".into(), row: 1 }),
                second: Box::new(RuleRef { pattern: "games.g.test_player".into(), origin: "games/g/game.toml".into(), row: 2 }),
            }
        );
        // The message names each matching rule by pattern, file and row, and the module's file.
        let msg = err.to_string();
        for want in [
            "src/games/g/test_player.emp",
            "`games.g.**` (engine/engine.toml [[gate]] row 1)",
            "`games.g.test_player` (games/g/game.toml [[gate]] row 2)",
        ] {
            assert!(msg.contains(want), "the refusal does not name `{want}`: {msg}");
        }
        // Control: disjoint rules gate the same modules.
        let both = scanned(&["games.g.ring", "games.g.test_player"]);
        let disjoint = rules[1..].to_vec();
        assert_eq!(gate_modules(&both, &disjoint, &homes(), &shape("debug", true)).unwrap(), ids(&["games.g.ring", "games.g.test_player"]));
    }

    #[test]
    fn shapes_feature_and_define_rules_decide() {
        let rules = vec![
            rule("a", PlaceIn::Shapes(vec!["debug".into()])),
            rule("b", PlaceIn::Shapes(vec![])),
            rule("c", PlaceIn::Feature(FeatureExpr::parse("!debug").unwrap())),
            rule("d", PlaceIn::Feature(FeatureExpr::parse("HOTKEYS").unwrap())),
        ];
        let scan = scanned(&["a", "b", "c", "d"]);
        assert_eq!(gate_modules(&scan, &rules, &homes(), &shape("debug", true)).unwrap(), ids(&["a"]));
        assert_eq!(gate_modules(&scan, &rules, &homes(), &shape("release", false)).unwrap(), ids(&["c"]));
        let missing = vec![rule("e", PlaceIn::Feature(FeatureExpr::parse("NOPE").unwrap()))];
        assert!(matches!(gate_modules(&scanned(&["e"]), &missing, &homes(), &shape("x", false)), Err(BuildError::Gate(m)) if m.contains("NOPE")));
    }

    /// G0b final review A-M1 (deferred line 108): a rule that cannot be evaluated is named
    /// by its file and row, not by its index into the engine rules followed by the game's.
    #[test]
    fn an_unevaluable_rule_is_named_by_file_and_row() {
        let mut rules = from_file("engine/engine.toml", vec![rule("engine.vdp", PlaceIn::All), rule("engine.ram", PlaceIn::All)]);
        rules.extend(from_file("games/g/game.toml", vec![rule("games.g.e", PlaceIn::Feature(FeatureExpr::parse("NOPE").unwrap()))]));
        let err = gate_modules(&scanned(&["games.g.e"]), &rules, &homes(), &shape("x", false)).unwrap_err().to_string();
        assert!(err.contains("`games.g.e` (games/g/game.toml [[gate]] row 1)"), "{err}");
        assert!(!err.contains("rule 2"), "no concatenated index: {err}");
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
