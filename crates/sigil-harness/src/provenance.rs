//! The golden provenance chain (`golden/provenance.toml`) — the §17 optimization
//! arc's machine-checkable re-freeze discipline.
//!
//! Post-flip the six golden blobs are sigil's OWN frozen output, so a byte-CHANGING
//! optimization proves correctness on the oracle (the emulator A/B) and then re-freezes
//! the goldens. This module models the append-only chain that records every such
//! re-freeze, and enforces the two invariants:
//!
//!   1. **TIP-MATCH** — each committed golden's recomputed `full_crc`/`full_size` and
//!      header-neutral `anchor_crc` equals the chain tip.
//!   2. **ANCHOR-MOVE-NEEDS-A/B** — a target whose `anchor_crc` differs from the prior
//!      entry forces the newer entry to carry a non-empty `ab` A/B-evidence ref. An
//!      anchor that moved without evidence is a HARD failure.
//!
//! [`check`] validates a loaded chain against the committed blobs (the gate, and the
//! `--check` half of the `refreeze` bin). [`recompute_targets`] rebuilds the per-target
//! CRC set from the blobs (the `--freeze` half). Both reuse [`crate::native::crc32`] and
//! [`crate::native::assembled_anchor_crc`] so the CRC math never diverges from the
//! full-file / anchor gates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The parsed `provenance.toml` — an append-only list of re-freeze entries, oldest
/// (`root-flip-freeze`) first, tip last.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chain {
    pub entry: Vec<Entry>,
}

/// One re-freeze event: a named parcel, its A/B evidence ref, and the per-target CRC
/// set the goldens hold AFTER it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Entry {
    pub name: String,
    /// A/B evidence reference. The root carries the `asl-witness` sentinel; every later
    /// entry that MOVES an anchor must carry a real ref (a design-note / A/B-log path).
    pub ab: String,
    #[serde(default)]
    pub note: String,
    /// target-key -> the golden's frozen CRC set. Keys are stable
    /// (`s4`/`s4_debug`/`demo`/`demo_debug`/`config_a`/`config_b`).
    pub targets: BTreeMap<String, Target>,
}

/// One golden's two CRC layers plus the anchor window.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Target {
    /// The committed blob filename in `golden/`.
    pub golden: String,
    /// Full-file CRC-32 (hex, no `0x`) — the regression pin (drifts on appendix change).
    pub full_crc: String,
    pub full_size: usize,
    /// Header-neutral assembled-anchor CRC-32 (hex) — the PRIMARY, drift-stable bar.
    pub anchor_crc: String,
    /// EndOfRom — the anchor window `[0, anchor_end)`.
    pub anchor_end: usize,
}

/// The `asl-witness` sentinel: the root's `ab`. Any OTHER entry carrying it (or an empty
/// `ab`) while moving an anchor is a discipline violation.
pub const ASL_WITNESS: &str = "asl-witness";

impl Chain {
    pub fn tip(&self) -> Result<&Entry, String> {
        self.entry.last().ok_or_else(|| "provenance.toml has no entries".to_string())
    }
}

/// Load + parse `golden/provenance.toml` from a golden directory. The one-liner the
/// gates use to source their expected CRC/size/anchor_end from the chain tip.
pub fn load(golden_dir: &Path) -> Result<Chain, String> {
    let path = golden_dir.join("provenance.toml");
    let src = std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    parse(&src)
}

/// The tip's target for a key, or an error naming the miss. `full_crc`/`anchor_crc` are
/// hex strings; use [`hex_u32`] to compare against a computed CRC.
pub fn tip_target(golden_dir: &Path, key: &str) -> Result<Target, String> {
    let chain = load(golden_dir)?;
    let tip = chain.tip()?;
    tip.targets
        .get(key)
        .cloned()
        .ok_or_else(|| format!("provenance tip `{}` has no target `{key}`", tip.name))
}

/// Parse a hex CRC string (no `0x`) to u32.
pub fn hex_u32(s: &str) -> Result<u32, String> {
    u32::from_str_radix(s, 16).map_err(|e| format!("bad hex crc `{s}`: {e}"))
}

/// Parse a `provenance.toml` source string.
pub fn parse(src: &str) -> Result<Chain, String> {
    let chain: Chain = toml::from_str(src).map_err(|e| format!("provenance.toml parse: {e}"))?;
    if chain.entry.is_empty() {
        return Err("provenance.toml has no [[entry]] blocks".into());
    }
    if chain.entry[0].ab != ASL_WITNESS {
        return Err(format!(
            "root entry `{}` must carry ab = \"{ASL_WITNESS}\" (got \"{}\")",
            chain.entry[0].name, chain.entry[0].ab
        ));
    }
    Ok(chain)
}

/// Recompute a target's frozen CRC set from its committed golden blob at a KNOWN
/// `anchor_end`. Used both to validate the tip (anchor_end read from the chain) and to
/// freeze a new tip (anchor_end read from the authoritative pins / size tables).
pub fn recompute_target(golden_dir: &Path, golden: &str, anchor_end: usize) -> Result<(String, usize, String), String> {
    let path = golden_dir.join(golden);
    let bytes = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if bytes.len() < anchor_end {
        return Err(format!(
            "{golden}: blob len {} shorter than anchor_end {anchor_end:#x}",
            bytes.len()
        ));
    }
    let full_crc = format!("{:08x}", crate::native::crc32(&bytes));
    let anchor_crc = format!("{:08x}", crate::native::assembled_anchor_crc(&bytes, anchor_end));
    Ok((full_crc, bytes.len(), anchor_crc))
}

/// Rebuild the per-target set for a fresh freeze from the committed blobs. `golden_map`
/// gives target-key -> golden filename; `anchor_ends` gives target-key -> EndOfRom from
/// the AUTHORITATIVE sources (pins.rs for canonical, `offcanonical_sizes/*.txt` headers
/// for the off-canonical shapes) so a size-changing optimization re-anchors correctly.
pub fn recompute_targets(
    golden_dir: &Path,
    golden_map: &BTreeMap<String, String>,
    anchor_ends: &BTreeMap<String, usize>,
) -> Result<BTreeMap<String, Target>, String> {
    let mut out = BTreeMap::new();
    for (key, golden) in golden_map {
        let end = *anchor_ends
            .get(key)
            .ok_or_else(|| format!("no authoritative anchor_end for target `{key}`"))?;
        let (full_crc, full_size, anchor_crc) = recompute_target(golden_dir, golden, end)?;
        out.insert(
            key.clone(),
            Target { golden: golden.clone(), full_crc, full_size, anchor_crc, anchor_end: end },
        );
    }
    Ok(out)
}

/// Validate a loaded chain against the committed golden blobs. Returns every violation
/// (empty = green). Two classes:
///   * TIP-MATCH — recomputed blob CRCs vs the tip entry.
///   * ANCHOR-MOVE-NEEDS-A/B — swept across EVERY consecutive pair (the whole chain stays
///     disciplined, not just the tip).
pub fn check(golden_dir: &Path, chain: &Chain) -> Vec<String> {
    let mut errs = Vec::new();

    // (2) chain discipline: an anchor that differs from the prior entry's same target
    // forces the newer entry to carry a real A/B ref.
    for pair in chain.entry.windows(2) {
        let (prev, cur) = (&pair[0], &pair[1]);
        let cur_needs_ab = cur.ab.trim().is_empty() || cur.ab == ASL_WITNESS;
        for (key, t) in &cur.targets {
            if let Some(pt) = prev.targets.get(key) {
                if pt.anchor_crc != t.anchor_crc && cur_needs_ab {
                    errs.push(format!(
                        "entry `{}`: target `{key}` anchor moved {} -> {} but ab=\"{}\" carries no A/B evidence (anchor-move-needs-A/B)",
                        cur.name, pt.anchor_crc, t.anchor_crc, cur.ab
                    ));
                }
            }
        }
    }

    // (1) tip-match: the committed blobs ARE the tip.
    let tip = match chain.tip() {
        Ok(t) => t,
        Err(e) => {
            errs.push(e);
            return errs;
        }
    };
    for (key, t) in &tip.targets {
        match recompute_target(golden_dir, &t.golden, t.anchor_end) {
            Ok((full_crc, full_size, anchor_crc)) => {
                if full_crc != t.full_crc {
                    errs.push(format!(
                        "tip `{}`: target `{key}` ({}) full_crc {} != recomputed {full_crc} (re-freeze?)",
                        tip.name, t.golden, t.full_crc
                    ));
                }
                if full_size != t.full_size {
                    errs.push(format!(
                        "tip `{}`: target `{key}` ({}) full_size {} != actual {full_size}",
                        tip.name, t.golden, t.full_size
                    ));
                }
                if anchor_crc != t.anchor_crc {
                    errs.push(format!(
                        "tip `{}`: target `{key}` ({}) anchor_crc {} != recomputed {anchor_crc} (re-freeze?)",
                        tip.name, t.golden, t.anchor_crc
                    ));
                }
            }
            Err(e) => errs.push(e),
        }
    }
    errs
}

/// Render one `[[entry]]` block (append-only text) in the file's house style. Kept as
/// text rendering (not `toml::to_string` of the whole chain) so appending preserves the
/// header comments and every existing entry verbatim.
pub fn render_entry(name: &str, ab: &str, note: &str, targets: &BTreeMap<String, Target>) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "\n[[entry]]");
    let _ = writeln!(s, "name = \"{name}\"");
    let _ = writeln!(s, "ab = \"{ab}\"");
    if !note.is_empty() {
        let _ = writeln!(s, "note = \"{note}\"");
    }
    for (key, t) in targets {
        let _ = writeln!(s, "\n[entry.targets.{key}]");
        let _ = writeln!(s, "golden = \"{}\"", t.golden);
        let _ = writeln!(s, "full_crc = \"{}\"", t.full_crc);
        let _ = writeln!(s, "full_size = {}", t.full_size);
        let _ = writeln!(s, "anchor_crc = \"{}\"", t.anchor_crc);
        let _ = writeln!(s, "anchor_end = {:#x}", t.anchor_end);
    }
    s
}

/// Whether a freshly recomputed target set is IDENTICAL to the current tip — the
/// fixpoint predicate: a re-freeze that produces the tip again is a no-op (append
/// nothing).
pub fn equals_tip(chain: &Chain, fresh: &BTreeMap<String, Target>) -> bool {
    match chain.tip() {
        Ok(tip) => &tip.targets == fresh,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(golden: &str, full: &str, size: usize, anchor: &str, end: usize) -> Target {
        Target {
            golden: golden.into(),
            full_crc: full.into(),
            full_size: size,
            anchor_crc: anchor.into(),
            anchor_end: end,
        }
    }

    #[test]
    fn root_ab_sentinel_required() {
        let src = "[[entry]]\nname=\"root\"\nab=\"nope\"\n[entry.targets.s4]\ngolden=\"s4.bin\"\nfull_crc=\"0\"\nfull_size=1\nanchor_crc=\"0\"\nanchor_end=1\n";
        assert!(parse(src).is_err(), "non-sentinel root ab must be rejected");
    }

    #[test]
    fn anchor_move_without_ab_is_flagged() {
        let mut root_t = BTreeMap::new();
        root_t.insert("s4".to_string(), t("s4.bin", "aaaa", 10, "1111", 8));
        let mut moved_t = BTreeMap::new();
        // anchor changed 1111 -> 2222, but ab is the sentinel -> violation.
        moved_t.insert("s4".to_string(), t("s4.bin", "bbbb", 10, "2222", 8));
        let chain = Chain {
            entry: vec![
                Entry { name: "root".into(), ab: ASL_WITNESS.into(), note: String::new(), targets: root_t },
                Entry { name: "bad".into(), ab: ASL_WITNESS.into(), note: String::new(), targets: moved_t },
            ],
        };
        // discipline sweep alone (no blobs) should flag the moved anchor.
        let errs = check(Path::new("/nonexistent"), &chain);
        assert!(
            errs.iter().any(|e| e.contains("anchor-move-needs-A/B")),
            "expected anchor-move-needs-A/B violation, got: {errs:?}"
        );
    }

    #[test]
    fn anchor_move_with_ab_ok_full_crc_free() {
        let mut root_t = BTreeMap::new();
        root_t.insert("s4".to_string(), t("s4.bin", "aaaa", 10, "1111", 8));
        let mut moved_t = BTreeMap::new();
        moved_t.insert("s4".to_string(), t("s4.bin", "bbbb", 10, "2222", 8));
        let chain = Chain {
            entry: vec![
                Entry { name: "root".into(), ab: ASL_WITNESS.into(), note: String::new(), targets: root_t },
                Entry { name: "g9".into(), ab: "notes/g9-ab.md".into(), note: String::new(), targets: moved_t },
            ],
        };
        let errs = check(Path::new("/nonexistent"), &chain);
        // no discipline violation (full_crc moved freely; anchor move carries ab).
        assert!(
            !errs.iter().any(|e| e.contains("anchor-move-needs-A/B")),
            "anchor move with a real ab must be allowed: {errs:?}"
        );
    }

    #[test]
    fn render_roundtrips() {
        let mut targets = BTreeMap::new();
        targets.insert("s4".to_string(), t("s4.bin", "7f071417", 412306, "e5765873", 0x5db60));
        let block = render_entry("p", "ref", "n", &targets);
        // raw deserialize (bypasses parse()'s root-sentinel guard — this is a lone entry).
        let chain: Chain = toml::from_str(&block).unwrap();
        assert_eq!(chain.entry[0].targets["s4"].anchor_end, 0x5db60);
        assert_eq!(chain.entry[0].ab, "ref");
        assert_eq!(chain.entry[0].targets["s4"].full_crc, "7f071417");
    }
}
