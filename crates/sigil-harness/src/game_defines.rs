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
}
