//! `[as.odd-address]`: sigil warns where asl raises `warning #180: address is not
//! properly aligned`, and nowhere else.
//!
//! Bytes cannot show this. The warning changes no byte, so a gate that compares
//! images passes whether the warning fires, is dropped, or fires on the wrong line.
//!
//! The expectations are not written here. Every fixture is a probe file under
//! `docs/superpowers/notes/2026-09-17-asl-warn-180/`, read at test time, and the
//! locations each one must warn at are that directory's `asl_expected.tsv`, which
//! `compare.py --mint` wrote from the reference asl (md5
//! `61e672562465725a8c102288a7da9098`, through `asl_run`, every probe exit 0 with
//! its pass loop complete). So the source sigil is tested on and the answer it is
//! held to are both the oracle's.
//!
//! Locations are compared as SETS. asl prints the warning once per pass that
//! evaluates a line and twice per pass for some mnemonics; sigil prints one per
//! offending operand or instruction start. The count is still checked where it is
//! a property of the source rather than of asl's pass loop: see
//! [`two_odd_operands_on_one_line_are_two_warnings`].

use sigil_frontend_as::{assemble_root_located_warned, assemble_root_relocating_warned, Options};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const ID: &str = "[as.odd-address]";

fn probe_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/superpowers/notes/2026-09-17-asl-warn-180")
}

/// Rows of a tab-separated file in the probe directory, comments and blanks dropped.
fn rows(name: &str) -> Vec<Vec<String>> {
    let path = probe_dir().join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

/// A diagnostic's location in asl's spelling: `file(line)` plus any macro or
/// `rept` trail, without sigil's trailing `:col` and without the directory the
/// probe was opened from, which asl never saw.
fn asl_location(label: &str) -> String {
    let (head, _col) = label.rsplit_once(':').expect("a label ends in :col");
    let open = head.find('(').expect("a label carries (line)");
    let file = &head[..open];
    let base = file.rsplit('/').next().unwrap_or(file);
    format!("{base}{}", &head[open..])
}

/// Every `[as.odd-address]` location sigil reports for a probe, with multiplicity,
/// or the refusal messages if sigil does not assemble it.
fn sigil_locations(probe: &str) -> Result<Vec<String>, Vec<String>> {
    let dir = probe_dir();
    let path = dir.join(format!("{probe}.asm"));
    let opts = Options { include_root: Some(dir), ..Default::default() };
    match assemble_root_located_warned(&path, &opts) {
        Ok(a) => Ok(a
            .warnings
            .iter()
            .filter(|d| d.message.starts_with(ID))
            .map(|d| {
                asl_location(
                    &a.sources
                        .label(d.primary)
                        .unwrap_or_else(|| panic!("{probe}: warning with no location: {}", d.message)),
                )
            })
            .collect()),
        Err(f) => Err(f.diags.into_iter().map(|d| d.message).collect()),
    }
}

fn warnings_for(src: &str, relocating: bool) -> Vec<String> {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir()
        .join("sigil_as_odd_address")
        .join(format!("{n}-{:?}", std::thread::current().id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("root.asm");
    std::fs::write(&path, src).expect("write fixture");
    let got = if relocating {
        assemble_root_relocating_warned(&path, &Options::default())
    } else {
        assemble_root_located_warned(&path, &Options::default())
    };
    std::fs::remove_dir_all(&dir).ok();
    match got {
        Ok(a) => a.warnings.into_iter().map(|d| d.message).filter(|m| m.starts_with(ID)).collect(),
        Err(f) => panic!(
            "expected the fixture to assemble, got {:?}",
            f.diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        ),
    }
}

/// THE PROPERTY, over every probe asl answered: sigil's set of warning locations
/// equals asl's, in both directions, on every probe sigil assembles; and every
/// probe sigil does not assemble is listed in `sigil_refuses.tsv` and refused for
/// the stated reason.
///
/// MUST FAIL if the warning stops firing anywhere asl fires it (a location
/// vanishes), if it fires anywhere asl does not (a location appears, for example
/// on `lea`, a byte access, a branch, a displacement or data at an odd address),
/// or if a probe starts or stops being refused without the list saying so.
///
/// Loud on vacuity: the minted table must name at least one firing and one silent
/// probe that sigil assembles, and every probe file must have a row.
#[test]
fn every_probe_warns_exactly_where_asl_does() {
    let mut expected: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in rows("asl_expected.tsv") {
        assert_eq!(r.len(), 3, "malformed asl_expected.tsv row {r:?}");
        let set = expected.entry(r[0].clone()).or_default();
        if r[1] != "-" {
            set.insert(r[1].clone());
        }
    }
    let refuses: BTreeMap<String, String> =
        rows("sigil_refuses.tsv").into_iter().map(|r| (r[0].clone(), r[1].clone())).collect();

    let mut on_disk: Vec<String> = std::fs::read_dir(probe_dir())
        .expect("probe dir")
        .filter_map(|e| {
            let name = e.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".asm").map(str::to_string)
        })
        .collect();
    on_disk.sort();
    assert_eq!(
        on_disk,
        expected.keys().cloned().collect::<Vec<_>>(),
        "asl_expected.tsv and the probe files disagree: re-mint with compare.py --mint"
    );
    for p in refuses.keys() {
        assert!(expected.contains_key(p), "sigil_refuses.tsv names `{p}`, which is no probe");
    }

    let (mut compared_firing, mut compared_silent, mut refused) = (0, 0, 0);
    let mut failures = Vec::new();
    for (probe, want) in &expected {
        match (sigil_locations(probe), refuses.get(probe)) {
            (Ok(got), None) => {
                let got: BTreeSet<String> = got.into_iter().collect();
                if &got != want {
                    failures.push(format!(
                        "{probe}: asl warns at {want:?}, sigil at {got:?}\n    \
                         only asl: {:?}\n    only sigil: {:?}",
                        want.difference(&got).collect::<Vec<_>>(),
                        got.difference(want).collect::<Vec<_>>()
                    ));
                } else if want.is_empty() {
                    compared_silent += 1;
                } else {
                    compared_firing += 1;
                }
            }
            (Err(msgs), Some(reason)) => {
                if msgs.iter().any(|m| m.contains(reason.as_str())) {
                    refused += 1;
                } else {
                    failures.push(format!("{probe}: refused, but not for `{reason}`: {msgs:?}"));
                }
            }
            (Ok(_), Some(reason)) => failures.push(format!(
                "{probe}: listed as refused (`{reason}`) but sigil assembled it; drop the row"
            )),
            (Err(msgs), None) => {
                failures.push(format!("{probe}: sigil refused a probe asl assembled: {msgs:?}"))
            }
        }
    }
    eprintln!(
        "odd-address parity: {compared_firing} firing and {compared_silent} silent probes \
         compared, {refused} refused as listed, {} failure(s)",
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(compared_firing > 0 && compared_silent > 0, "a vacuous comparison proves nothing");
    assert_eq!(
        compared_firing + compared_silent + refused,
        expected.len(),
        "every probe must be accounted for"
    );
}

/// The line that started this: Sonic 2's `move.w (1).w,d0` (`s2.asm`, inside
/// `if gameRevision=0`), which asl warns about and sigil did not. Probe `p01`
/// line 5 is that operand shape.
///
/// MUST FAIL if the operand rule is dropped. Also pins the wording's substance:
/// it names the address and says why it matters.
#[test]
fn sonic_2s_word_read_at_address_one_warns() {
    let w = warnings_for("\tcpu 68000\n\tpadding off\n\torg 0\n\tmove.w\t(1).w,d0\n", false);
    assert_eq!(w.len(), 1, "one warning, got {w:?}");
    assert!(w[0].contains("$1"), "names the address: {}", w[0]);
    assert!(w[0].contains("address error"), "says why it matters: {}", w[0]);
    assert!(w[0].contains("asl #180"), "names asl's counterpart: {}", w[0]);
    assert!(!w[0].contains('\u{2014}') && !w[0].contains('\u{2013}'), "no em or en dash");
}

/// asl prints one warning per odd operand (probe `p02` line 7, `move.w
/// (1).w,(3).w`, twice on a one-pass file), and so does sigil.
///
/// MUST FAIL if sigil collapses a line's warnings into one, or checks only the
/// first operand.
#[test]
fn two_odd_operands_on_one_line_are_two_warnings() {
    let w = warnings_for("\tcpu 68000\n\tpadding off\n\torg 0\n\tmove.w\t(1).w,(3).w\n", false);
    assert_eq!(w.len(), 2, "two warnings, got {w:?}");
    assert!(w[0].contains("$1") && w[1].contains("$3"), "one per address: {w:?}");
}

/// An instruction at an odd address warns once for its start (probe `p23a`),
/// and a word access at an odd address on the same line warns again (`p23d`).
///
/// MUST FAIL if the instruction-start rule is dropped, or if it is folded into the
/// operand rule.
#[test]
fn an_instruction_starting_at_an_odd_address_warns() {
    let w = warnings_for("\tcpu 68000\n\tpadding off\n\torg 0\n\tdc.b\t0\n\tnop\n", false);
    assert_eq!(w.len(), 1, "one warning, got {w:?}");
    assert!(w[0].contains("starts at odd address $1"), "{}", w[0]);
    let w = warnings_for("\tcpu 68000\n\tpadding off\n\torg 0\n\tdc.b\t0\n\tmove.w\t(1).w,d0\n", false);
    assert_eq!(w.len(), 2, "instruction start and operand, got {w:?}");
}

/// The does-not-fire half, one line per excluded class, each measured silent on
/// asl: `lea`/`pea` (`p05`), byte access (`p01`), a bit operation on memory and
/// `tas` (`p09`), a branch to an odd label (`p19`), a PC-relative read of an odd
/// label (`p06`), a displacement (`p07`), an immediate (`p08`), data at an odd
/// address (`p13b`), and an even address (`p01`).
///
/// MUST FAIL if any of those classes starts warning, which is the over-firing
/// direction the corpus-wide set comparison above would also catch but could not
/// name as precisely.
#[test]
fn the_classes_asl_leaves_silent_stay_silent() {
    let src = "\tcpu 68000\n\tpadding off\n\torg 0\n\
               \tlea\t(1).w,a0\n\
               \tpea\t(1).w\n\
               \tmove.b\t(1).w,d0\n\
               \tbtst\t#0,(1).w\n\
               \ttas\t(1).w\n\
               \tbra.w\todd\n\
               \tmove.w\todd(pc),d0\n\
               \tmove.w\t1(a0),d0\n\
               \tmove.w\t#1,d0\n\
               \tmove.w\t(2).w,d0\n\
               \tdc.b\t0\n\
               odd:\n\
               \tdc.w\t1\n\
               \tdc.b\t0\n";
    let w = warnings_for(src, false);
    assert!(w.is_empty(), "asl is silent on every line here, sigil said {w:?}");
}

/// On the relocating path (every chained aeon build) sections still move after
/// assembly, so an address taken from a label or from the location counter is
/// provisional and is NOT guessed at: neither an odd instruction start nor an odd
/// label operand warns there. A constant address is not provisional and still
/// does. Gap row `AS-ODD-ADDRESS-RELOCATING-UNDECIDED` books the undecided half.
///
/// MUST FAIL if the relocating path starts deciding label parity (a guess), or if
/// it stops deciding constants (a real loss).
#[test]
fn the_relocating_path_decides_constants_and_leaves_labels_undecided() {
    let src = "\tcpu 68000\n\tpadding off\n\torg 0\n\
               \tmove.w\t(1).w,d0\n\
               \tmove.w\t(OddLabel).l,d0\n\
               \tdc.b\t0\n\
               OddLabel:\n\
               \tnop\n";
    let pinned = warnings_for(src, false);
    assert_eq!(pinned.len(), 3, "pinned: constant, label and instruction start, got {pinned:?}");
    let relocating = warnings_for(src, true);
    assert_eq!(relocating.len(), 1, "relocating: the constant only, got {relocating:?}");
    assert!(relocating[0].contains("access at odd address $1"), "{}", relocating[0]);
}
