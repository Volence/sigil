//! THE COMMITTED GOLDEN HEADERS MUST BE WHAT THE GENERATOR WOULD WRITE TODAY.
//!
//! Three committed artifacts carry a header rendered by
//! [`sigil_isa::asl_provenance::Provenance::header`]:
//!
//!   * `crates/sigil-isa/tests/z80_golden_vectors.txt`
//!   * `crates/sigil-isa/tests/m68k_golden_vectors.txt`
//!   * `crates/sigil-frontend-as/tests/snippets_golden.txt`
//!
//! Nothing compared the two, and on 2026-09-05 they drifted. The dash sweep
//! `f6618ec9` correctly swept string literals and correctly left comments alone,
//! but `PREAMBLE` and `push_banner` are string literals that RENDER HEADER PROSE
//! into generated files, so the generator's output moved and these three static
//! files did not. The identical cause hit a fourth artifact,
//! `crates/sigil-harness/src/pins.rs`, which HAD a currency check and went red
//! within hours. One cause, four artifacts; the only difference in how it
//! surfaced was whether somebody had written a comparison. This is that
//! comparison for the other three.
//!
//! WHAT IT COMPARES, AND WHY THAT SHAPE. The check reads the provenance VALUES
//! out of each committed file (minted-by, the two digests, the banners), rebuilds
//! a `Provenance` from them, renders it with the SAME `header()` the generators
//! call, and requires the result to equal the committed header byte for byte.
//!
//! Two properties follow from deriving it that way, and both are the point:
//!
//!   * NO FIXTURE HOLDS A COPY OF THE PROSE. An expectation that quoted
//!     `PREAMBLE` into a test would have to be edited in lockstep with it, which
//!     is a check that goes green because somebody maintained it rather than
//!     because the tree is correct. Here the expectation IS the renderer, so a
//!     `PREAMBLE` edit reds this immediately and there is no fixture to update.
//!   * NO ASL IS RUN AND NO DIGEST IS PINNED. The measured values come from the
//!     file itself, which is what makes this runnable in CI where asl is out of
//!     repo (Stage-3 P4d / OQ-A), and which keeps the check honest about its own
//!     scope: it says the header is CURRENT, never that the build named in it was
//!     the right one. Pinning a digest here would be a different gate, and a
//!     wrong one, because the header exists to record which build answered rather
//!     than to require a particular answer.
//!
//! WHAT IT DOES NOT COVER, stated because a claim of completeness and the check
//! that would establish it are separable:
//!
//!   * The vector rows below the header. Those are asl measurements and their
//!     gates live in `sigil-isa/tests/{z80,m68k}_golden.rs` and
//!     `sigil-frontend-as/tests/asl_snippets.rs`.
//!   * A HAND EDITED PROVENANCE VALUE. Change `asl-md5` in a committed file to
//!     32 zeroes and this gate stays green, MEASURED, not assumed: the values
//!     are read from the file and rendered back, so a lie about which build
//!     answered round trips undisturbed. Catching that needs the binary the
//!     header names, which is out of repo. This gate says the header is what the
//!     generator writes; it never says the header is true.
//!   * A `header()` restructure that changes the LABEL SET or drops the
//!     continuation convention `push_banner` uses for an absent banner. That
//!     would need [`fields`] and [`provenance_from`] below to move with it, and
//!     [`the_inverse_round_trips_both_banner_shapes`] is what would say so.
//!
//! THE FAILURE MODE IT IS AIMED AT is the generator moving underneath a static
//! file, not somebody editing the file. A check that only fired on an edited
//! artifact would have missed the actual 2026-09-05 drift entirely, so
//! [`the_check_fires_when_the_generator_moves`] holds that line: it perturbs the
//! RENDERED PROSE, which is what a `PREAMBLE` edit does, and requires a refusal.

use sigil_isa::asl_provenance::{Provenance, ToolIdentity};
use std::path::PathBuf;

/// One committed artifact and the generator that mints it.
struct Artifact {
    /// Workspace relative path.
    path: &'static str,
    /// The `minted-by` value its header must carry, and the cargo bin that
    /// writes it. This is per artifact metadata, not header prose: nothing here
    /// is a copy of anything the renderer emits.
    minted_by: &'static str,
    regen: &'static str,
}

const ARTIFACTS: &[Artifact] = &[
    Artifact {
        path: "crates/sigil-isa/tests/z80_golden_vectors.txt",
        minted_by: "gen-z80-vectors",
        regen: "ASL_BIN=<asl> cargo run -p sigil-isa --bin gen-z80-vectors",
    },
    Artifact {
        path: "crates/sigil-isa/tests/m68k_golden_vectors.txt",
        minted_by: "gen-m68k-vectors",
        regen: "ASL_BIN=<asl> cargo run -p sigil-isa --bin gen-m68k-vectors",
    },
    Artifact {
        path: "crates/sigil-frontend-as/tests/snippets_golden.txt",
        minted_by: "gen_snippet_vectors",
        // The bin target is named with underscores. The module doc in
        // gen_snippet_vectors.rs spells it with hyphens, which cargo rejects.
        regen: "ASL_BIN=<asl> cargo run -p sigil-frontend-as --bin gen_snippet_vectors",
    },
];

/// `<workspace>`; `CARGO_MANIFEST_DIR` is `<workspace>/crates/sigil-harness`.
fn workspace_root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .and_then(|p| p.parent())
        .expect("crates/sigil-harness sits two levels under the workspace root")
        .to_path_buf()
}

/// The leading `#` comment block of a golden, plus the blank line that closes
/// it, exactly as `Provenance::header` writes it (`#` lines, then a bare `\n`).
///
/// Returns `Err` rather than an empty string when the file does not open with a
/// header: a check that read "no header" as "no mismatch" would pass loudest on
/// the artifact that lost its provenance altogether.
fn header_block(text: &str, path: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut count = 0usize;
    for line in text.lines() {
        if line.starts_with('#') {
            out.push_str(line);
            out.push('\n');
            count += 1;
        } else if line.is_empty() {
            out.push('\n');
            return if count == 0 {
                Err(format!("{path} opens with a blank line, not a provenance header"))
            } else {
                Ok(out)
            };
        } else {
            return Err(format!(
                "{path} has {count} header line(s) and then {line:?} where the blank line \
                 closing the header should be"
            ));
        }
    }
    Err(format!("{path} is {count} header line(s) and nothing else, it has no body"))
}

/// One labelled field parsed back out of a rendered header.
struct Field {
    label: String,
    value: String,
    /// A continuation line carries no label. `push_banner` emits one only in the
    /// no-banner case, which is how [`provenance_from`] tells an absent banner
    /// from a present one without quoting either one's prose.
    continuation: bool,
}

/// Split a header's field lines (`# label        value`, and their unlabelled
/// continuations) out of the fixed prose above them.
///
/// The split is structural, not textual: `push_field` writes `# `, then a left
/// aligned label, then two spaces, then the value. So a field line has a non
/// space character immediately after `# ` and a label with no space in it, and a
/// continuation line has a space there. Prose matches neither and is skipped.
fn fields(header: &str) -> Vec<Field> {
    let mut out = Vec::new();
    for line in header.lines() {
        let Some(rest) = line.strip_prefix("# ") else {
            continue;
        };
        if rest.starts_with(' ') {
            out.push(Field {
                label: String::new(),
                value: rest.trim_start().to_string(),
                continuation: true,
            });
            continue;
        }
        let Some((label, value)) = rest.split_once("  ") else {
            continue;
        };
        if label.contains(' ') || !label.contains(|c: char| c.is_ascii_alphabetic()) {
            continue;
        }
        out.push(Field {
            label: label.to_string(),
            value: value.trim_start().to_string(),
            continuation: false,
        });
    }
    out
}

/// Rebuild the `Provenance` a committed header records.
///
/// This is the inverse of `Provenance::header`, and it quotes none of that
/// function's text. A tool's banner is taken to be ABSENT when its field block
/// carries a continuation line, because `push_banner` emits a continuation only
/// in the empty case; a present banner is one field line per banner line.
fn provenance_from(header: &str, path: &str) -> Result<Provenance, String> {
    let fields = fields(header);
    let one = |label: &str| -> Result<String, String> {
        let mut hits = fields.iter().filter(|f| f.label == label);
        let first = hits
            .next()
            .ok_or_else(|| format!("{path}: no `{label}` line in the provenance header"))?;
        if hits.next().is_some() {
            return Err(format!("{path}: more than one `{label}` line in the header"));
        }
        Ok(first.value.clone())
    };
    let banner = |tool: &str| -> Vec<String> {
        let label = format!("{tool}-banner");
        let mut lines = Vec::new();
        let mut continued = false;
        let mut inside = false;
        for f in &fields {
            if f.continuation {
                continued |= inside;
            } else if f.label == label {
                inside = true;
                lines.push(f.value.clone());
            } else {
                inside = false;
            }
        }
        if continued {
            Vec::new()
        } else {
            lines
        }
    };

    Ok(Provenance {
        minted_by: one("minted-by")?,
        asl: ToolIdentity { md5: one("asl-md5")?, banner: banner("asl") },
        p2bin: ToolIdentity { md5: one("p2bin-md5")?, banner: banner("p2bin") },
    })
}

/// The first differing line of two headers, rendered for a person.
fn first_difference(committed: &str, generated: &str) -> String {
    let c: Vec<&str> = committed.lines().collect();
    let g: Vec<&str> = generated.lines().collect();
    let show = |l: Option<&&str>| -> String {
        l.map(|s| format!("{s:?}")).unwrap_or_else(|| "(no such line)".to_string())
    };
    for i in 0..c.len().max(g.len()) {
        if c.get(i) != g.get(i) {
            return format!(
                "  first difference at header line {}:\n    committed: {}\n    generator: {}",
                i + 1,
                show(c.get(i)),
                show(g.get(i)),
            );
        }
    }
    "  (the headers agree line for line and differ only in trailing bytes)".to_string()
}

/// The refusal message, so the gate and its own red-first proof render the same
/// text rather than two texts that are asserted to resemble each other.
fn stale_message(art: &Artifact, committed: &str, generated: &str) -> String {
    format!(
        "STALE GENERATED HEADER: {}\n\n\
         Its committed header is not what sigil_isa::asl_provenance::Provenance::header \
         renders today, for the very provenance values this file itself records. The vector \
         rows below the header are not in question here; this is the header prose alone, and \
         the usual cause is an edit to PREAMBLE or push_banner in \
         crates/sigil-isa/src/asl_provenance.rs moving the generator's output while this \
         static file stayed where it was. That is exactly how these three files drifted on \
         2026-09-05 unnoticed, while pins.rs took the same edit and went red the same day \
         because it had a check.\n\n\
         {}\n\n\
         FIX BY REGENERATING, never by hand editing the header:\n    {}\n\
         A header freshened by hand sits directly above rows that are real asl measurements, \
         and at read time an edit that freshened prose is indistinguishable from an edit that \
         adjusted a measurement.",
        art.path,
        first_difference(committed, generated),
        art.regen,
    )
}

/// Compare one artifact's committed header against a freshly rendered one.
fn check(art: &Artifact) -> Result<(), String> {
    let path = workspace_root().join(art.path);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let committed = header_block(&text, art.path)?;
    let prov = provenance_from(&committed, art.path)?;

    if prov.minted_by != art.minted_by {
        return Err(format!(
            "{}: its header says it was minted by {:?}, this check expects {:?}. Either the \
             wrong generator wrote this file or this test's table is out of date.",
            art.path, prov.minted_by, art.minted_by
        ));
    }

    let generated = prov.header();
    if generated == committed {
        return Ok(());
    }
    Err(stale_message(art, &committed, &generated))
}

/// THE GATE. Every committed golden header is what its generator writes today.
#[test]
fn committed_golden_headers_are_current() {
    assert!(!ARTIFACTS.is_empty(), "this gate enumerated no artifacts, so it measured nothing");
    let mut bad = Vec::new();
    for art in ARTIFACTS {
        if let Err(e) = check(art) {
            bad.push(e);
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} generated golden header(s) are stale:\n\n{}",
        bad.len(),
        ARTIFACTS.len(),
        bad.join("\n\n")
    );
}

/// THE FAILURE MODE THIS EXISTS FOR, held open.
///
/// The 2026-09-05 drift was the GENERATOR moving under files nobody touched, so
/// a check that only noticed edited artifacts would have slept through it. This
/// renders a header, perturbs the rendered PROSE (what a `PREAMBLE` edit does),
/// and requires the comparison to call it stale and say how to fix it.
///
/// The perturbation is positional rather than a quoted phrase, so this test does
/// not have to be maintained alongside the prose it is about.
#[test]
fn the_check_fires_when_the_generator_moves() {
    let prov = Provenance {
        minted_by: "gen-z80-vectors".to_string(),
        asl: ToolIdentity {
            md5: "0".repeat(32),
            banner: vec!["Macro Assembler 1.42 Beta [Bld 212]".to_string()],
        },
        p2bin: ToolIdentity { md5: "1".repeat(32), banner: Vec::new() },
    };
    let current = prov.header();

    // A committed file whose header was rendered before the prose moved: the
    // first prose line ends one character differently.
    let cut = current.find('\n').expect("a rendered header has lines");
    let stale = format!("{};{}", &current[..cut], &current[cut..]);
    assert_ne!(stale, current, "the perturbation must actually change the header");

    let committed = header_block(&stale, "fixture").expect("the perturbed header still parses");
    let parsed = provenance_from(&committed, "fixture").expect("its fields still parse");
    assert_eq!(parsed, prov, "the inverse must recover the values it was given");

    let generated = parsed.header();
    assert_ne!(generated, committed, "a moved preamble must not compare equal");

    // The message a person gets names the drift, quotes both lines and points at
    // the generator. This is the gate's own text, not a lookalike.
    let art = &ARTIFACTS[0];
    let msg = stale_message(art, &committed, &generated);
    let stale_line = &stale[..cut + 1];
    let current_line = &current[..cut];
    assert!(msg.starts_with("STALE GENERATED HEADER: "), "{msg}");
    assert!(msg.contains(art.path), "message must name the file: {msg}");
    assert!(msg.contains(art.regen), "message must give the regeneration command: {msg}");
    assert!(msg.contains("never by hand editing"), "message must forbid the hand edit: {msg}");
    assert!(
        msg.contains(&format!("committed: {stale_line:?}")),
        "message must quote the committed line: {msg}"
    );
    assert!(
        msg.contains(&format!("generator: {current_line:?}")),
        "message must quote the line the generator writes today: {msg}"
    );
}

/// The inverse must round trip, or the gate compares a header against a
/// misparse of itself and its green means nothing.
///
/// Both banner shapes are covered because they render differently: a present
/// banner is one field line per line, an absent one is a field line plus an
/// unlabelled continuation, and telling those apart is the only judgement the
/// inverse makes.
#[test]
fn the_inverse_round_trips_both_banner_shapes() {
    let cases = [
        (
            "both banners present",
            Provenance {
                minted_by: "gen-m68k-vectors".to_string(),
                asl: ToolIdentity {
                    md5: "a".repeat(32),
                    banner: vec![
                        "Macro Assembler 1.42 Beta [Bld 212]".to_string(),
                        "(x86_64-unknown-linux)".to_string(),
                    ],
                },
                p2bin: ToolIdentity { md5: "b".repeat(32), banner: vec!["p2bin".to_string()] },
            },
        ),
        (
            "p2bin banner absent, the shape all three real files have",
            Provenance {
                minted_by: "gen_snippet_vectors".to_string(),
                asl: ToolIdentity {
                    md5: "c".repeat(32),
                    banner: vec![
                        "Macro Assembler 1.42 Beta [Bld 212]".to_string(),
                        "(x86_64-unknown-linux)".to_string(),
                    ],
                },
                p2bin: ToolIdentity { md5: "d".repeat(32), banner: Vec::new() },
            },
        ),
        (
            "both banners absent",
            Provenance {
                minted_by: "gen-z80-vectors".to_string(),
                asl: ToolIdentity { md5: "e".repeat(32), banner: Vec::new() },
                p2bin: ToolIdentity { md5: "f".repeat(32), banner: Vec::new() },
            },
        ),
    ];
    let mut bad = Vec::new();
    for (name, prov) in &cases {
        let rendered = prov.header();
        let block = match header_block(&rendered, name) {
            Ok(b) => b,
            Err(e) => {
                bad.push(format!("{name}: header_block refused a rendered header: {e}"));
                continue;
            }
        };
        match provenance_from(&block, name) {
            Ok(back) if back == *prov => {}
            Ok(back) => bad.push(format!("{name}: round trip lost data: {back:?} != {prov:?}")),
            Err(e) => bad.push(format!("{name}: the inverse refused a rendered header: {e}")),
        }
    }
    assert!(bad.is_empty(), "{} round trip failure(s):\n{}", bad.len(), bad.join("\n"));
}

/// A header that is missing or truncated must be a loud failure, never a quiet
/// pass. A gate that treats "nothing to compare" as "nothing wrong" is at its
/// most silent on the artifact that lost its provenance.
#[test]
fn a_missing_header_is_a_failure_not_a_pass() {
    let mut bad = Vec::new();
    let cases: &[(&str, &str)] = &[
        ("no header at all", "nop => 00\n"),
        ("opens on a blank line", "\nnop => 00\n"),
        ("header and nothing else", "# minted-by     x\n"),
    ];
    for (name, text) in cases {
        if header_block(text, "fixture").is_ok() {
            bad.push(format!("header_block accepted {name}: {text:?}"));
        }
    }
    if provenance_from("# PROVENANCE\n#\n", "fixture").is_ok() {
        bad.push("the inverse accepted a header carrying no fields".to_string());
    }
    assert!(bad.is_empty(), "{} vacuous-pass hole(s):\n{}", bad.len(), bad.join("\n"));
}
