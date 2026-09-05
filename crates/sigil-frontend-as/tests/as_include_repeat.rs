//! `include` builds a TREE, not a DAG: the same file included twice assembles
//! twice, and the only thing that stops it is DEPTH.
//!
//! Sigil kept a set of every path it had ever included and silently returned
//! from the second `include` of any of them. That was a byte divergence with no
//! diagnostic on either side — asl said nothing because it was doing the normal
//! thing, and sigil said nothing because it thought it was. It also ran the
//! wrong way on the shapes asl REFUSES: a file that includes itself, and two
//! files that include each other, both assembled clean under the old guard and
//! produced a ROM.
//!
//! Expectations derived from asl 1.42 Beta [Bld 212]
//! (`s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`) with Sonic 1's own flags minus `-E`/`-c`.
//! Probes `p1`–`p8` plus the two depth generators are committed under
//! `docs/superpowers/notes/2026-09-05-as-include-repeat-probes/` with their
//! verbatim listings.
//!
//! EVERY FIXTURE HERE USES A HEADER THAT EMITS BYTES, AND ASSERTS THE BYTES AND
//! THE LENGTH. A zero-byte header, or a second copy landing where both readings
//! agree, cannot separate once from twice — the image is identical either way.
//! Each test names, in its own doc comment, the other answer it could have
//! given; a fixture in this file that cannot name one is decoration and should
//! be deleted rather than kept.

use sigil_frontend_as::{assemble, Options, INCLUDE_NEST_MAX};

/// A scratch tree of `(name, contents)` files, assembled from `root`.
///
/// The include population cannot be written as a string literal — `include`
/// reads the filesystem — so every fixture here builds a real directory. The
/// `TempDir` is returned rather than dropped so the files outlive the call.
fn tree(files: &[(&str, &str)]) -> (tempfile::TempDir, Options) {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, body) in files {
        let p = dir.path().join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&p, body).expect("write");
    }
    let opts = Options {
        include_root: Some(dir.path().to_path_buf()),
        ..Options::default()
    };
    (dir, opts)
}

/// Assemble and link the named root of a scratch tree, returning the image.
fn bytes(files: &[(&str, &str)], root: &str) -> Vec<u8> {
    let (_dir, opts) = tree(files);
    let src = files
        .iter()
        .find(|(n, _)| *n == root)
        .expect("root file is in the tree")
        .1;
    let module = assemble(src, &opts).expect("assemble");
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// Assemble the named root expecting a REFUSAL, and hand back the messages.
fn refusal(files: &[(&str, &str)], root: &str) -> Vec<String> {
    let (_dir, opts) = tree(files);
    let src = files
        .iter()
        .find(|(n, _)| *n == root)
        .expect("root file is in the tree")
        .1;
    match assemble(src, &opts) {
        Ok(_) => panic!("expected a refusal, the source assembled"),
        Err(diags) => diags.into_iter().map(|d| d.message).collect(),
    }
}

/// `padding off` so a `dc.b` run is not silently word-aligned: alignment padding
/// is a byte the fixture did not ask for, and it would let a length assertion
/// pass for the wrong reason.
fn head() -> &'static str {
    "\tcpu\t68000\n\tpadding\toff\n\torg\t0\n"
}

/// probe `p1`. One two-byte header, included twice, then a sentinel.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `[0x11, 0x22, 0x99]` — three bytes,
/// which is exactly what the old visited-set guard produced and what asl does
/// not. The header emits two NON-ZERO bytes and the sentinel is distinct from
/// both, so the two readings differ in length AND in content; neither assertion
/// alone is load-bearing, and the length one is what catches a rule that keeps
/// the second copy but emits it over the first.
#[test]
fn a_header_included_twice_assembles_twice() {
    let img = bytes(
        &[
            (
                "root.asm",
                &format!(
                    "{}\tinclude\t\"h.inc\"\n\tinclude\t\"h.inc\"\n\tdc.b\t$99\n",
                    head()
                ),
            ),
            ("h.inc", "\tdc.b\t$11,$22\n"),
        ],
        "root.asm",
    );
    assert_eq!(img, vec![0x11, 0x22, 0x11, 0x22, 0x99]);
    assert_eq!(img.len(), 5);
}

/// probe `p4`. A DIAMOND: the root includes `b` and `c`, and both include `d`.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `[0xB0, 0xDD, 0xC0, 0x99]` — the DAG
/// reading, in which `d` is shared. This is the shape the old comment named as
/// the intent ("DAG, not tree"), so it is the fixture that says the intent was
/// wrong rather than merely unimplemented. `b` and `c` emit different bytes, so
/// the assertion also pins the ORDER, not just the count.
#[test]
fn a_diamond_assembles_the_shared_file_twice() {
    let img = bytes(
        &[
            (
                "root.asm",
                &format!(
                    "{}\tinclude\t\"b.inc\"\n\tinclude\t\"c.inc\"\n\tdc.b\t$99\n",
                    head()
                ),
            ),
            ("b.inc", "\tdc.b\t$B0\n\tinclude\t\"d.inc\"\n"),
            ("c.inc", "\tdc.b\t$C0\n\tinclude\t\"d.inc\"\n"),
            ("d.inc", "\tdc.b\t$DD\n"),
        ],
        "root.asm",
    );
    assert_eq!(img, vec![0xB0, 0xDD, 0xC0, 0xDD, 0x99]);
    assert_eq!(img.len(), 5);
}

/// probe `p5`. Three spellings of ONE file — `h.inc`, `./h.inc`,
/// `sub/../h.inc` — in one program.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: three of them. `[0x11, 0x22, 0x99]`
/// is a rule that canonicalizes and dedupes (the old one); `[0x11, 0x22, 0x11,
/// 0x22, 0x11, 0x22, 0x99]` with only the first two spellings collapsed would be
/// a rule that dedupes on the literal string. asl gives all three, and this is
/// the fixture that says there is no path-identity question left to get wrong:
/// the answer does not depend on how the path is spelled because nothing
/// compares paths at all.
#[test]
fn three_spellings_of_one_path_are_three_inclusions() {
    let img = bytes(
        &[
            (
                "root.asm",
                &format!(
                    "{}\tinclude\t\"h.inc\"\n\tinclude\t\"./h.inc\"\n\tinclude\t\"sub/../h.inc\"\n\tdc.b\t$99\n",
                    head()
                ),
            ),
            ("h.inc", "\tdc.b\t$11,$22\n"),
            ("sub/keep", ""),
        ],
        "root.asm",
    );
    assert_eq!(img, vec![0x11, 0x22, 0x11, 0x22, 0x11, 0x22, 0x99]);
    assert_eq!(img.len(), 7);
}

/// probe `p7`. A header that both READS and WRITES a `set` symbol, included
/// three times: `count set count+1` then `dc.b count`.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `[0x01, 0x99]` under the old guard,
/// or `[0x01, 0x01, 0x01, 0x99]` under a rule that re-emitted a CACHED copy of
/// the first inclusion's output instead of re-executing the file. The three
/// copies here are required to differ from each other, which is a strictly
/// stronger claim than "there are three of them" — it is the fixture that says
/// re-inclusion is re-EXECUTION.
#[test]
fn a_re_included_header_re_runs_its_directives() {
    let img = bytes(
        &[
            (
                "root.asm",
                &format!(
                    "{}n\tset\t0\n\tinclude\t\"h.inc\"\n\tinclude\t\"h.inc\"\n\tinclude\t\"h.inc\"\n\tdc.b\t$99\n",
                    head()
                ),
            ),
            ("h.inc", "n\tset\tn+1\n\tdc.b\tn\n"),
        ],
        "root.asm",
    );
    assert_eq!(img, vec![0x01, 0x02, 0x03, 0x99]);
    assert_eq!(img.len(), 4);
}

/// probe `p2`. A file that includes ITSELF.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: three of them, and two were real.
/// (a) The old guard assembled this CLEAN — one `$AA` byte, exit 0, no
/// diagnostic — for a program asl terminates on; that is the answer this test
/// exists to make impossible. (b) With the guard simply deleted and no bound put
/// back, the recursion is unbounded and the answer is a native stack overflow,
/// which is a crash rather than a verdict. (c) A refusal keyed on "this file is
/// already open" would also land here — but it is not what asl does, and the
/// `nesting_is_refused_one_level_past` pair below is where that distinction
/// becomes visible.
#[test]
fn a_self_including_file_is_refused_for_depth() {
    let msgs = refusal(
        &[
            ("root.asm", &format!("{}\tinclude\t\"s.inc\"\n", head())),
            ("s.inc", "\tdc.b\t$AA\n\tinclude\t\"s.inc\"\n"),
        ],
        "root.asm",
    );
    assert!(
        msgs.iter().any(|m| m.contains("INCLUDE nested too deeply")),
        "expected asl's depth refusal, got {msgs:?}"
    );
}

/// probe `p3`. MUTUAL recursion: `b` includes `c`, `c` includes `b`. No file
/// names itself.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `[0xB0, 0xC0]`, exit 0 — which is
/// what the old guard produced, and what a refusal keyed on a file naming ITSELF
/// would also produce. asl terminates on this exactly as it does on `p2`, and at
/// the same kind of site (`p3b.inc(2)`), so the two shapes are one shape.
#[test]
fn mutual_recursion_between_two_files_is_refused_for_depth() {
    let msgs = refusal(
        &[
            ("root.asm", &format!("{}\tinclude\t\"b.inc\"\n", head())),
            ("b.inc", "\tdc.b\t$B0\n\tinclude\t\"c.inc\"\n"),
            ("c.inc", "\tdc.b\t$C0\n\tinclude\t\"b.inc\"\n"),
        ],
        "root.asm",
    );
    assert!(
        msgs.iter().any(|m| m.contains("INCLUDE nested too deeply")),
        "expected asl's depth refusal, got {msgs:?}"
    );
}

/// Build a chain of `n` DISTINCT files, each including the next and emitting one
/// byte. Nothing in it repeats, so no re-inclusion rule of any shape may touch
/// it — the only thing that can refuse it is depth.
fn chain(n: u32) -> Vec<(String, String)> {
    let mut files = vec![(
        "root.asm".to_string(),
        format!("{}\tinclude\t\"n1.inc\"\n", head()),
    )];
    for i in 1..=n {
        let mut body = "\tdc.b\t$AA\n".to_string();
        if i < n {
            body.push_str(&format!("\tinclude\t\"n{}.inc\"\n", i + 1));
        }
        files.push((format!("n{i}.inc"), body));
    }
    files
}

fn borrow(files: &[(String, String)]) -> Vec<(&str, &str)> {
    files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()
}

/// The bound, checked on BOTH sides of itself.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: an off-by-one in either direction,
/// and that is the whole point of asserting the clean case as well as the
/// refused one. A test that only asserts "200 is refused" passes just as well
/// against a bound of 1, which would refuse every real program; a test that only
/// asserts "199 is clean" passes against no bound at all. Both numbers are read
/// off asl:
/// `docs/superpowers/notes/2026-09-05-as-include-repeat-probes/depth.sh 199`
/// exits 0 at deepest level 199, and `depth.sh 200` raises
/// `error #10008: INCLUDE nested too deeply` at `n199.inc(2)` and terminates.
/// Sigil refuses at that same site.
///
/// Note this chain contains no repetition whatever, so it also states the
/// converse of the tests above: the bound is about depth and NOT about a file
/// being seen twice.
#[test]
fn nesting_is_clean_at_the_bound_and_refused_one_level_past() {
    let deep = chain(INCLUDE_NEST_MAX);
    let img = bytes(&borrow(&deep), "root.asm");
    assert_eq!(img.len(), INCLUDE_NEST_MAX as usize);
    assert!(img.iter().all(|&b| b == 0xAA));

    let too_deep = chain(INCLUDE_NEST_MAX + 1);
    let msgs = refusal(&borrow(&too_deep), "root.asm");
    assert!(
        msgs.iter().any(|m| m.contains("INCLUDE nested too deeply")),
        "expected asl's depth refusal at {} levels, got {msgs:?}",
        INCLUDE_NEST_MAX + 1
    );
}

/// The refusal is raised ONCE, not once per open level.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: 199 copies of it. Unwinding one
/// level and carrying on is the obvious implementation of "refuse this include",
/// and it re-raises the same message at every frame on the way out — which is
/// what asl's `fatal error, assembly terminated` exists to avoid. The count is
/// the assertion, so a change that turns the abort back into a return fails here
/// rather than merely making logs noisier.
#[test]
fn the_depth_refusal_is_raised_once_not_once_per_open_level() {
    let too_deep = chain(INCLUDE_NEST_MAX + 1);
    let msgs = refusal(&borrow(&too_deep), "root.asm");
    let n = msgs
        .iter()
        .filter(|m| m.contains("INCLUDE nested too deeply"))
        .count();
    assert_eq!(n, 1, "expected exactly one depth refusal, got {msgs:?}");
}
