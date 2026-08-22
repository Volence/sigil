//! The per-game `[defines]` table reader (`games/<g>/map.toml`).
//!
//! A game declares comptime define rows — the game→engine `-D` interface values
//! (the `MAX_RING_BUFFER` class: constants the game-agnostic engine `.emp` reads
//! but no `.emp` module may declare) — in its OWN placement map, beside the
//! placement facts sigil already reads from that file. `map.toml` is the one
//! per-game contract file sigil consumes (`build.conf` is shell sourced by aeon's
//! `build.sh` and never parsed here), so define rows live there too.
//!
//! ONLY the explicit `[defines]` table is read. Unknown keys elsewhere in the map
//! stay the concern of the region/placement readers (which tolerate them), so a
//! typo'd placement key can never silently become a define.
//!
//! Consumption: [`crate::native::shape_defines`] merges these rows with the
//! profile's built-in `emp_defines` rows. The merge is CONFLICT-FREE by
//! construction — a game row whose key matches a built-in row is a loud error in
//! BOTH directions (neither source silently wins), because the built-ins carry
//! shape semantics (`DEBUG`, `SOUND_DRIVER_ENABLED`, …) and a silent override
//! either way would move ROM bytes with no diff at the losing source. Moving a
//! value from a built-in row into a game's `[defines]` therefore has to delete
//! the built-in row in the same change.
//!
//! Coverage: a game-declared row is outside the built-in polarity gate's walk
//! (`tests/shipped_shapes.rs` reads `profile.emp_defines`), so
//! [`audit_game_declared_polarity`] holds the game rows to the same property —
//! a toggle walked in both polarities, a value walked at two sizes, or an
//! exemption naming the reason.

/// Parse the `[defines]` table of a `games/<g>/map.toml` source into define rows.
///
/// `origin` names the source in every diagnostic (the map.toml path). A map with
/// no `[defines]` table parses to an empty row set — the byte-neutral default
/// every shipped map has today. Each value must be a TOML integer (hex literals
/// like `0x3E8` are TOML integers); each key must be define-identifier-shaped.
/// A key declared twice fails loud, naming the key and both lines.
pub fn parse_game_defines(toml_src: &str, origin: &str) -> Result<Vec<(String, i128)>, String> {
    // Deserialize ONLY the `defines` table; every other key in the document is
    // ignored here (serde's unknown-field default), exactly as the placement
    // reader ignores the region keys.
    #[derive(serde::Deserialize)]
    struct MapDefinesDoc {
        #[serde(default)]
        defines: Option<toml::value::Table>,
    }

    let doc: MapDefinesDoc = match toml::from_str(toml_src) {
        Ok(doc) => doc,
        Err(e) => {
            // The PARSER is the duplicate-key authority (the TOML grammar
            // forbids them); the line scan runs only after the parser has
            // reported a duplicate in `[defines]`, to upgrade the message to
            // the both-lines form. A duplicate the scan cannot see (exotic
            // quoting) keeps the parser's own message — so the scan can
            // upgrade a message but never reject a document the parser
            // accepts (a defines-shaped STRING body is data, not rows).
            let msg = e.to_string();
            if msg.contains("duplicate key") && msg.contains("in table `defines`") {
                if let Some(upgraded) = duplicate_define_key_message(toml_src, origin) {
                    return Err(upgraded);
                }
            }
            return Err(format!("{origin}: parse error: {e}"));
        }
    };
    let Some(table) = doc.defines else {
        return Ok(Vec::new());
    };

    let mut out = Vec::with_capacity(table.len());
    for (key, val) in table {
        if !is_define_ident(&key) {
            return Err(format!(
                "{origin}: [defines] key `{key}` is not a define identifier \
                 ([A-Za-z_][A-Za-z0-9_]*)"
            ));
        }
        match val {
            toml::Value::Integer(v) => out.push((key, v as i128)),
            other => {
                return Err(format!(
                    "{origin}: [defines].{key} must be an integer, got a {} ({other})",
                    other.type_str()
                ));
            }
        }
    }
    Ok(out)
}

/// Merge a profile's built-in `emp_defines` rows with a game's `[defines]` rows.
///
/// A game row with a fresh key joins the env; a game row whose key matches a
/// built-in row is an ERROR naming the key and both sources — shadowing a
/// built-in is forbidden in both directions (see the module doc for why neither
/// source may silently win).
pub fn merge_builtin_and_game(
    builtin: &[(&'static str, i128)],
    game: Vec<(String, i128)>,
    origin: &str,
) -> Result<Vec<(String, i128)>, String> {
    let mut out: Vec<(String, i128)> =
        builtin.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    for (key, gv) in game {
        if let Some((_, bv)) = builtin.iter().find(|(bk, _)| *bk == key) {
            return Err(format!(
                "define `{key}` is declared twice: {origin} [defines] (= {gv}) and the \
                 built-in profile row in crates/sigil-harness/src/native.rs (= {bv}) — \
                 a game config must not shadow a built-in row (and a built-in must not \
                 silently override a game config); delete one of the two declarations"
            ));
        }
        out.push((key, gv));
    }
    Ok(out)
}

/// Adjudicate the GAME-DECLARED rows of the shipped shapes' merged define envs
/// against the same polarity property the built-in rows are held to.
///
/// `tests/shipped_shapes.rs`'s both-polarities gate reads `profile.emp_defines`
/// — the BUILT-IN rows only — so a game-declared row is outside that walk. This
/// is the other half: it walks [`crate::native::shape_defines`] output, subtracts
/// the built-in keys, and charges the remainder the property that makes a corpus
/// walk mean anything — that no arm is unreachable by every shape.
///
/// The charge mirrors the built-in gate exactly. A key whose observed values are
/// BOOLEAN-shaped is a toggle and must take both `0` and `1`; any other key is a
/// value and must take at least two distinct values. A key that can meet neither
/// is not rejected — it must be NAMED in `exempt` with a reason, so a blind arm
/// is a recorded decision rather than a silent absence. An exemption for a key no
/// shape declares is itself an error, so the list cannot outlive its subject.
///
/// `shapes` is `(label, merged define env)` per shipped shape; `builtin_keys` is
/// the union of those shapes' built-in keys. Returns the game-declared keys it
/// adjudicated, so a caller can assert the walk saw what it expected to.
pub fn audit_game_declared_polarity(
    shapes: &[(&str, Vec<(String, i128)>)],
    builtin_keys: &std::collections::BTreeSet<String>,
    exempt: &[(&str, &str)],
) -> Result<std::collections::BTreeSet<String>, String> {
    use std::collections::{BTreeMap, BTreeSet};

    let mut seen: BTreeMap<String, BTreeMap<i128, Vec<String>>> = BTreeMap::new();
    for (label, defines) in shapes {
        for (key, val) in defines {
            if builtin_keys.contains(key) {
                continue;
            }
            seen.entry(key.clone())
                .or_default()
                .entry(*val)
                .or_default()
                .push((*label).to_string());
        }
    }

    for (key, reason) in exempt {
        if reason.trim().is_empty() {
            return Err(format!(
                "the polarity exemption for game-declared define `{key}` carries no \
                 reason — an exemption records WHY a blind arm is acceptable, so an \
                 empty one is the silent absence it exists to replace"
            ));
        }
        if !seen.contains_key(*key) {
            return Err(format!(
                "the polarity exemption for game-declared define `{key}` is STALE: no \
                 shipped shape declares that key. Delete the exemption row"
            ));
        }
    }

    for (key, by_value) in &seen {
        if exempt.iter().any(|(k, _)| k == key) {
            continue;
        }
        let values: Vec<i128> = by_value.keys().copied().collect();
        let boolean_shaped = values.iter().all(|v| *v == 0 || *v == 1);
        let covered = if boolean_shaped {
            values.contains(&0) && values.contains(&1)
        } else {
            values.len() >= 2
        };
        if covered {
            continue;
        }
        let where_declared: Vec<String> = by_value
            .iter()
            .map(|(v, labels)| format!("{v} in [{}]", labels.join(", ")))
            .collect();
        let kind = if boolean_shaped {
            "is boolean-shaped, so it is a TOGGLE and must take both 0 and 1"
        } else {
            "must take at least two distinct values"
        };
        return Err(format!(
            "game-declared define `{key}` takes {values:?} across the shipped shapes \
             ({}) — it {kind}. Pinned to one value it carries a comptime arm no shape \
             walk reaches, and the built-in polarity gate cannot see it because that \
             gate reads `profile.emp_defines`. Either declare the other value in a \
             second game's `[defines]`, or name `{key}` in the polarity exemption list \
             with the reason its other arm needs no coverage",
            where_declared.join("; ")
        ));
    }

    Ok(seen.keys().cloned().collect::<BTreeSet<String>>())
}

/// A define key: `[A-Za-z_][A-Za-z0-9_]*` — the shape the comptime define env
/// and the AS `-D` seam both address.
fn is_define_ident(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// The both-lines message for a key declared twice inside `[defines]`, when the
/// line scan can locate both rows; `None` when it cannot.
///
/// Called only after `toml::from_str` has itself reported a duplicate key in
/// `[defines]` — the parser is the enforcement, this names the first occurrence
/// the parser's message omits. Because the scan never runs on a document the
/// parser accepts, its line-level naivety (a `#` inside a quoted value, exotic
/// key quoting) can at worst decline the upgrade, never misread data as rows.
fn duplicate_define_key_message(src: &str, origin: &str) -> Option<String> {
    let mut in_defines = false;
    let mut seen: Vec<(String, usize)> = Vec::new();
    for (idx, raw) in src.lines().enumerate() {
        let line = raw.trim();
        let line = line.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            in_defines = line == "[defines]";
            continue;
        }
        if !in_defines || line.is_empty() {
            continue;
        }
        let Some(eq) = line.find('=') else {
            continue;
        };
        let key = line[..eq].trim().trim_matches('"');
        if key.is_empty() {
            continue;
        }
        if let Some((_, first_line)) = seen.iter().find(|(k, _)| k == key) {
            return Some(format!(
                "{origin}: [defines] key `{key}` is declared twice (lines {first_line} \
                 and {}) — a define has exactly one row",
                idx + 1
            ));
        }
        seen.push((key.to_string(), idx + 1));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// A realistic map.toml body: regions/placement keys plus a `[defines]` table.
    /// The non-defines keys must be invisible to this reader.
    const MAP_WITH_DEFINES: &str = r#"
fill = 0x00
order = ["Vectors", "BootData"]

[[region]]
name = "rom"
lma_base = 0
size = 0x400000
kind = "rom"

[defines]
MAX_RING_BUFFER = 16
VRAM_RING_PLACEHOLDER = 0x3E4

[[anchor]]
name = "boot_head"
at = 0x0
"#;

    #[test]
    fn defines_table_parses_rows_and_ignores_placement_keys() {
        let rows = parse_game_defines(MAP_WITH_DEFINES, "fixture/map.toml").unwrap();
        // toml::value::Table iterates in key order; assert as a set-with-values.
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert!(rows.contains(&("MAX_RING_BUFFER".to_string(), 16)), "{rows:?}");
        // Hex literal round-trips as the integer value.
        assert!(rows.contains(&("VRAM_RING_PLACEHOLDER".to_string(), 0x3E4)), "{rows:?}");
    }

    #[test]
    fn map_without_defines_table_parses_to_no_rows() {
        // The byte-neutral default: every shipped map.toml today. A region-only
        // fixture map must also yield no rows.
        let src = "fill = 0x00\n[[region]]\nname=\"rom\"\nlma_base=0\nsize=0x400000\nkind=\"rom\"\n";
        assert_eq!(parse_game_defines(src, "fixture/map.toml").unwrap(), Vec::new());
    }

    #[test]
    fn duplicate_key_in_defines_table_is_a_loud_error_naming_key_and_both_lines() {
        let src = "[defines]\nMAX_RING_BUFFER = 16\nOTHER = 1\nMAX_RING_BUFFER = 32\n";
        let err = parse_game_defines(src, "fixture/map.toml").unwrap_err();
        assert!(err.contains("MAX_RING_BUFFER"), "error must name the key: {err}");
        assert!(err.contains("fixture/map.toml"), "error must name the source: {err}");
        assert!(
            err.contains("lines 2") && err.contains("4"),
            "error must name both rows: {err}"
        );
    }

    #[test]
    fn duplicate_key_in_another_table_does_not_trip_the_defines_scan() {
        // The pre-scan is scoped to `[defines]`; a duplicate elsewhere is the
        // full parser's concern and still fails, but as ITS error, proving the
        // scan does not misattribute other tables' keys to the defines table.
        let src = "[other]\nX = 1\nX = 2\n";
        let err = parse_game_defines(src, "fixture/map.toml").unwrap_err();
        assert!(err.contains("parse error"), "expected the toml parse error, got: {err}");
    }

    #[test]
    fn a_string_body_shaped_like_a_defines_table_is_not_scanned() {
        // A multi-line string is DATA; its lines must never reach the duplicate
        // scan. The scan is consulted only after the parser itself reports a
        // duplicate in `[defines]`, so a defines-shaped string body cannot
        // reject a valid map.
        let src = "[docs]\nbody = \"\"\"\n[defines]\nX = 1\nX = 2\n\"\"\"\n\n[defines]\nREAL = 3\n";
        let rows = parse_game_defines(src, "fixture/map.toml").unwrap();
        assert_eq!(rows, vec![("REAL".to_string(), 3)]);
    }

    #[test]
    fn non_integer_define_value_is_a_loud_error() {
        let src = "[defines]\nMAX_RING_BUFFER = \"16\"\n";
        let err = parse_game_defines(src, "fixture/map.toml").unwrap_err();
        assert!(err.contains("MAX_RING_BUFFER") && err.contains("integer"), "{err}");
    }

    #[test]
    fn non_identifier_define_key_is_a_loud_error() {
        let src = "[defines]\n\"MAX-RING\" = 16\n";
        let err = parse_game_defines(src, "fixture/map.toml").unwrap_err();
        assert!(err.contains("MAX-RING") && err.contains("identifier"), "{err}");
    }

    #[test]
    fn merge_fresh_game_key_joins_the_builtin_rows() {
        // Direction 1 of the precedence rule: a fresh game key LANDS in the env,
        // after the built-ins, with its declared value.
        let builtin: &[(&'static str, i128)] = &[("DEBUG", 1), ("MAX_RING_BUFFER", 128)];
        let merged = merge_builtin_and_game(
            builtin,
            vec![("SCANLINE_CAPS".to_string(), 20)],
            "fixture/map.toml",
        )
        .unwrap();
        assert_eq!(
            merged,
            vec![
                ("DEBUG".to_string(), 1),
                ("MAX_RING_BUFFER".to_string(), 128),
                ("SCANLINE_CAPS".to_string(), 20),
            ]
        );
    }

    #[test]
    fn merge_game_key_shadowing_a_builtin_is_a_loud_error_naming_both_sources() {
        // Direction 2 of the precedence rule: a game row may NEVER shadow a
        // built-in row (and the built-in may never silently override the game's
        // declared value) — the collision is an error naming the key, the game
        // config source, and the built-in source.
        let builtin: &[(&'static str, i128)] = &[("MAX_RING_BUFFER", 128)];
        let err = merge_builtin_and_game(
            builtin,
            vec![("MAX_RING_BUFFER".to_string(), 16)],
            "games/demo/map.toml",
        )
        .unwrap_err();
        assert!(err.contains("MAX_RING_BUFFER"), "error must name the key: {err}");
        assert!(err.contains("games/demo/map.toml"), "error must name the game source: {err}");
        assert!(err.contains("native.rs"), "error must name the built-in source: {err}");
        assert!(err.contains("= 16") && err.contains("= 128"), "error names both values: {err}");
    }

    // ── audit_game_declared_polarity ──────────────────────────────────────────
    //
    // Both directions are asserted for every rule: a reject-everything audit
    // would satisfy the poison arms and be caught only by the control arms, and
    // an audit that saw nothing would satisfy the control arms and be caught only
    // by the poison arms.

    fn builtins(keys: &[&str]) -> BTreeSet<String> {
        keys.iter().map(|k| k.to_string()).collect()
    }

    fn shape(label: &str, rows: &[(&str, i128)]) -> (String, Vec<(String, i128)>) {
        (label.to_string(), rows.iter().map(|(k, v)| (k.to_string(), *v)).collect())
    }

    fn audit(
        shapes: &[(String, Vec<(String, i128)>)],
        builtin: &BTreeSet<String>,
        exempt: &[(&str, &str)],
    ) -> Result<BTreeSet<String>, String> {
        let borrowed: Vec<(&str, Vec<(String, i128)>)> =
            shapes.iter().map(|(l, d)| (l.as_str(), d.clone())).collect();
        audit_game_declared_polarity(&borrowed, builtin, exempt)
    }

    #[test]
    fn a_game_declared_toggle_pinned_to_one_polarity_is_named() {
        // POISON: the exact latent — a game map declares a 0/1 row, every shape
        // that reads that map sees the same value, and the other arm is blind.
        let shapes = [
            shape("sonic4 plain", &[("DEBUG", 0), ("FANCY_ARM", 1)]),
            shape("demo plain", &[("DEBUG", 0)]),
        ];
        let err = audit(&shapes, &builtins(&["DEBUG"]), &[]).unwrap_err();
        assert!(err.contains("FANCY_ARM"), "error must name the key: {err}");
        assert!(err.contains("TOGGLE"), "error must say it is a toggle: {err}");
        assert!(err.contains("sonic4 plain"), "error must name where it is declared: {err}");
        assert!(!err.contains("DEBUG"), "a BUILT-IN key is the other gate's charge: {err}");
    }

    #[test]
    fn a_game_declared_toggle_walked_in_both_polarities_passes() {
        // CONTROL for the toggle rule: the same key at both values is exactly
        // what the polarity net wants, so it must NOT be charged. This is the arm
        // a reject-everything audit fails.
        let shapes = [
            shape("sonic4 plain", &[("DEBUG", 0), ("FANCY_ARM", 1)]),
            shape("demo plain", &[("DEBUG", 0), ("FANCY_ARM", 0)]),
        ];
        let keys = audit(&shapes, &builtins(&["DEBUG"]), &[]).unwrap();
        assert_eq!(keys, builtins(&["FANCY_ARM"]));
    }

    #[test]
    fn a_game_declared_value_pinned_to_one_size_is_named() {
        // POISON for the value rule: a non-boolean row is still a comparison a
        // comptime `if` can branch on, so one size across the corpus is one arm.
        let shapes = [
            shape("sonic4 plain", &[("SCANLINE_CAPS", 20)]),
            shape("demo plain", &[("SCANLINE_CAPS", 20)]),
        ];
        let err = audit(&shapes, &BTreeSet::new(), &[]).unwrap_err();
        assert!(err.contains("SCANLINE_CAPS"), "error must name the key: {err}");
        assert!(
            err.contains("at least two distinct values"),
            "error must say what the value rule is: {err}"
        );
    }

    #[test]
    fn a_game_declared_value_walked_at_two_sizes_passes() {
        // CONTROL for the value rule — the MAX_RING_BUFFER class the mechanism
        // exists for, declared per game.
        let shapes = [
            shape("sonic4 plain", &[("SCANLINE_CAPS", 20)]),
            shape("demo plain", &[("SCANLINE_CAPS", 24)]),
        ];
        let keys = audit(&shapes, &BTreeSet::new(), &[]).unwrap();
        assert_eq!(keys, builtins(&["SCANLINE_CAPS"]));
    }

    #[test]
    fn no_game_declared_rows_is_an_empty_walk_not_an_error() {
        // CONTROL for the whole audit: today's state — every shipped map declares
        // nothing, so the adjudicated key set is empty and nothing is charged.
        // An audit that charged here would fire on the shipped corpus.
        let shapes = [
            shape("sonic4 plain", &[("DEBUG", 1)]),
            shape("demo plain", &[("DEBUG", 0)]),
        ];
        let keys = audit(&shapes, &builtins(&["DEBUG"]), &[]).unwrap();
        assert!(keys.is_empty(), "{keys:?}");
    }

    #[test]
    fn an_exempted_key_is_named_rather_than_absent() {
        // The escape hatch: a blind arm may stand, but only as a row someone
        // wrote a reason on.
        let shapes = [
            shape("sonic4 plain", &[("FANCY_ARM", 1)]),
            shape("demo plain", &[]),
        ];
        let keys = audit(
            &shapes,
            &BTreeSet::new(),
            &[("FANCY_ARM", "sonic4-only capability; the 0 arm is unreachable by design")],
        )
        .unwrap();
        assert_eq!(keys, builtins(&["FANCY_ARM"]), "an exempt key is still REPORTED: {keys:?}");
    }

    #[test]
    fn an_exemption_with_no_reason_is_rejected() {
        let shapes = [shape("sonic4 plain", &[("FANCY_ARM", 1)])];
        let err = audit(&shapes, &BTreeSet::new(), &[("FANCY_ARM", "   ")]).unwrap_err();
        assert!(err.contains("carries no reason"), "{err}");
    }

    #[test]
    fn an_exemption_for_a_key_no_shape_declares_is_stale_and_rejected() {
        // The list cannot outlive its subject: a dropped game row must not leave
        // a standing waiver behind for the next key of that name.
        let shapes = [shape("sonic4 plain", &[("SCANLINE_CAPS", 20)])];
        let err = audit(
            &shapes,
            &BTreeSet::new(),
            &[("SCANLINE_CAPS", "plural"), ("GONE", "was exempt once")],
        )
        .unwrap_err();
        assert!(err.contains("GONE") && err.contains("STALE"), "{err}");
    }
}
