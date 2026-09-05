//! The two out-verifier CFG blind spots (§G4.5): a locally-`bsr`'d helper block,
//! and a computed intra-proc dispatch (`jmp .table(pc,Xn) targets(...)`).
//!
//! Both are driven through the WHOLE corpus walk (`parse → analyze_corpus →
//! [proc.out-unverified]`), not the dataflow directly, because the facts each
//! blind spot turns on have to TRAVEL: the declared `out(dN: T)` width from the
//! parser through the width map to the proc's own obligation, and the local-`bsr`
//! MUST-intersection credit through the same production dataflow a caller's out is
//! charged over. A unit test of the last step alone would pass over machinery that
//! was never wired.
//!
//! **The pairing rule.** Local-`bsr` credit and `out(dN: u8)` width types each
//! close ZERO on their own and only together produce a byte-wide helper result
//! that satisfies a byte-wide claim; the CORE gates are PAIRED (the honest body
//! that must verify, and a one-fact-weaker body that must still fire) so a
//! single-sided assert cannot go quietly true under a checker that stopped
//! charging one of the two. A few gates are positive-only EXTENSIONS of a paired
//! core — the fall-off consumer and the nested-helper composition — where the
//! must-still-fire polarity is already carried by the mutant runs the packet
//! records rather than by a sibling body here.

use sigil_frontend_emp::ast::{File, Item};
use sigil_frontend_emp::contract::InterfaceEnv;
use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{CodeItem, CodeOperand};
use sigil_ir::backend::Cpu;
use std::path::{Path, PathBuf};

/// Parse each source (demanding a clean parse) and run the corpus contract walk.
fn analyze(srcs: &[&str]) -> ContractReport {
    let files: Vec<_> = srcs
        .iter()
        .map(|s| {
            let (f, diags) = parse_str(s);
            assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
            f
        })
        .collect();
    analyze_corpus(&files)
}

/// Does `[proc.out-unverified]` fire on `(proc, reg)`?
fn out_fires(r: &ContractReport, proc: &str, reg: &str) -> bool {
    r.out_firings.iter().any(|f| f.proc == proc && f.reg == reg)
}

/// Is `(proc, reg)` in the VERIFIED-out map — the fixpoint's positive answer?
/// Asserting this rather than "no firing" is what tells a real proof from a walk
/// that never saw the proc.
fn verified(r: &ContractReport, proc: &str, reg: &str) -> bool {
    r.verified_uncond_out.get(proc).is_some_and(|s| s.contains(reg))
}

// ===========================================================================
// BLIND SPOT 1 — a locally-bsr'd helper's productions credit the caller.
// ===========================================================================

/// The `probe_core` shape in miniature: a proc whose ONLY writer of `d1`/`d2` is a
/// helper reached by `jbsr .cell`, both written `.b` under a `u8` claim. A `bsr`
/// gets only its fall-through edge, so without local-`bsr` credit the helper is
/// invisible and `d1`/`d2` are unproduced — WITH it, `.cell`'s MUST-production
/// covers both on the one reachable return, so the caller verifies.
const HELPER_U8: &str = "module m\n\
     proc P (d6: u8) clobbers(d0-d5/a1) out(d1: u8, d2: u8) {\n\
         jbsr    .cell\n\
         rts\n\
     .cell:\n\
         move.b  d6, d1\n\
         move.b  d6, d2\n\
         rts\n\
     }\n";

#[test]
fn local_bsr_helper_credits_the_caller_at_u8() {
    let r = analyze(&[HELPER_U8]);
    assert!(verified(&r, "P", "d1"), "d1 from `.cell` byte write → verified: {:?}", r.out_firings);
    assert!(verified(&r, "P", "d2"), "d2 from `.cell` byte write → verified: {:?}", r.out_firings);
}

/// The TRAP, half one — the SAME helper under a BARE `out(d1, d2)` (32-bit claim).
/// The helper is credited (local-`bsr` credit is live), but a `.b` write is a
/// byte and the claim asks a long, so the width filter still fires. Local-`bsr`
/// credit alone closes nothing without the matching width type.
#[test]
fn local_bsr_credit_without_the_width_type_still_fires() {
    let r = analyze(&[
        "module m\n\
         proc P (d6: u8) clobbers(d0-d5/a1) out(d1, d2) {\n\
             jbsr    .cell\n\
             rts\n\
         .cell:\n\
             move.b  d6, d1\n\
             move.b  d6, d2\n\
             rts\n\
         }\n",
    ]);
    assert!(out_fires(&r, "P", "d1"), "byte helper write under a bare claim → fires: {:?}", r.out_firings);
    assert!(out_fires(&r, "P", "d2"), "byte helper write under a bare claim → fires: {:?}", r.out_firings);
}

/// The red case that a spurious credit would break: a proc that writes the out
/// register NOWHERE — not in its body, not in a helper — must still FIRE even
/// though it contains a local `bsr`. The credit is the helper's own productions,
/// not a blanket "a bsr happened, assume produced".
#[test]
fn a_bsr_to_a_helper_that_writes_nothing_does_not_credit() {
    let r = analyze(&[
        "module m\n\
         proc P () clobbers(d0-d5/a1) out(d1: u8, d2: u8) {\n\
             jbsr    .cell\n\
             rts\n\
         .cell:\n\
             nop\n\
             rts\n\
         }\n",
    ]);
    assert!(out_fires(&r, "P", "d1"), "helper writes nothing → d1 fires: {:?}", r.out_firings);
    assert!(out_fires(&r, "P", "d2"), "helper writes nothing → d2 fires: {:?}", r.out_firings);
}

/// The credit is a MUST-INTERSECTION over the helper's OWN return paths, not a
/// union: a register written on only ONE of the helper's paths is not guaranteed
/// produced. The helper has TWO SEPARATE `rts` exits (no merge before them, as in
/// the real `.cell`), so this exercises the exit-accumulator intersection, not
/// just the in-body join: `d1` is written before the split and verifies; `d2`
/// only on the `!eq` exit and FIRES.
#[test]
fn local_bsr_credit_is_the_must_intersection_over_helper_paths() {
    let r = analyze(&[
        "module m\n\
         proc P (d6: u8) clobbers(d0-d5/a1) out(d1: u8, d2: u8) {\n\
             jbsr    .cell\n\
             rts\n\
         .cell:\n\
             move.b  d6, d1\n\
             tst.w   d0\n\
             beq     .other\n\
             move.b  d6, d2\n\
             rts\n\
         .other:\n\
             rts\n\
         }\n",
    ]);
    assert!(verified(&r, "P", "d1"), "d1 on every helper exit → verified: {:?}", r.out_firings);
    assert!(out_fires(&r, "P", "d2"), "d2 on only one helper exit → fires: {:?}", r.out_firings);
}

/// The credit reaches a `falls_into` (fall-off) exit as well as an `rts` — the
/// helper produces the register, the caller falls into a successor, and the claim
/// is met at the fall-off the same way it is at a return. (A second consumer of
/// the same produced state: `check_return` is charged at `Edge::FallOff` too.)
#[test]
fn local_bsr_credit_reaches_a_fall_off_exit() {
    let r = analyze(&[
        "module m\n\
         proc Succ () clobbers(d0) {\n\
             rts\n\
         }\n\
         proc P (d6: u8) clobbers(d0-d5/a1) out(d1: u8) falls_into Succ {\n\
             jbsr    .cell\n\
             jbra    .end\n\
         .cell:\n\
             move.b  d6, d1\n\
             rts\n\
         .end:\n\
             nop\n\
         }\n",
    ]);
    // The `jbra .end` skips `.cell` on the main path, so `.cell` is reached ONLY by
    // the `jbsr`; `d1` is credited by the local-`bsr` credit and the claim is met at
    // the FALL-OFF into `Succ` (a second consumer of the produced state, distinct
    // from a return).
    assert!(verified(&r, "P", "d1"), "d1 credited before the fall-off → verified: {:?}", r.out_firings);
}

/// Nested local `bsr` — a helper that itself `bsr`s a helper — composes: the inner
/// credit flows through the outer helper to the caller. Exercises `transfer`'s
/// local-`bsr` arm from INSIDE the sub-walk (the second caller of `transfer`).
#[test]
fn nested_local_bsr_credit_composes() {
    let r = analyze(&[
        "module m\n\
         proc P (d6: u8) clobbers(d0-d5/a1) out(d1: u8) {\n\
             jbsr    .outer\n\
             rts\n\
         .outer:\n\
             jbsr    .inner\n\
             rts\n\
         .inner:\n\
             move.b  d6, d1\n\
             rts\n\
         }\n",
    ]);
    assert!(verified(&r, "P", "d1"), "d1 from a nested helper → verified: {:?}", r.out_firings);
}

// ===========================================================================
// BLIND SPOT 2 — a computed intra-proc dispatch with `targets(...)` is verified
// INSIDE the proc: its landing blocks are reachable, and it charges no false out
// obligation as a transfer OUT.
// ===========================================================================

/// Every landing of a `targets(...)`-annotated `jmp .tbl(pc,Xn)` produces the out,
/// so the dispatch verifies — the transfer stays inside the proc, its landings are
/// reachable to the production dataflow.
#[test]
fn targets_dispatch_verifies_inside_the_proc() {
    let r = analyze(&[
        "module m\n\
         proc P (d0: u8) clobbers(d0-d3/a1) out(d1: u8) {\n\
             move.w  .tbl(pc,d0.w), d0\n\
             jmp     .tbl(pc,d0.w) targets(.a, .b)\n\
         .tbl:\n\
             dc.w    .a-.tbl\n\
             dc.w    .b-.tbl\n\
         .a:\n\
             move.b  d0, d1\n\
             rts\n\
         .b:\n\
             move.b  d0, d1\n\
             rts\n\
         }\n",
    ]);
    assert!(verified(&r, "P", "d1"), "d1 produced on every landing → verified: {:?}", r.out_firings);
}

/// WITHOUT the `targets(...)` clause the SAME body fires: the computed `jmp` is one
/// opaque transfer OUT, so it charges an out obligation at a point control never
/// leaves and its landings are unreachable. This is the blind spot the clause
/// closes — the paired contrast to the test above.
#[test]
fn the_same_dispatch_without_targets_fires() {
    let r = analyze(&[
        "module m\n\
         proc P (d0: u8) clobbers(d0-d3/a1) out(d1: u8) {\n\
             move.w  .tbl(pc,d0.w), d0\n\
             jmp     .tbl(pc,d0.w)\n\
         .tbl:\n\
             dc.w    .a-.tbl\n\
             dc.w    .b-.tbl\n\
         .a:\n\
             move.b  d0, d1\n\
             rts\n\
         .b:\n\
             move.b  d0, d1\n\
             rts\n\
         }\n",
    ]);
    assert!(out_fires(&r, "P", "d1"), "an opaque computed transfer charges a false out obligation: {:?}", r.out_firings);
}

/// The probe the brief names: verification stays INSIDE the proc, so removing ONE
/// landing's write of the out makes the claim FIRE — the dispatch is not treated
/// as a transfer out that skips the landing. `.b` no longer writes `d1`, and the
/// MUST-intersection over the two landings drops `d1`.
#[test]
fn removing_one_targets_landings_write_fires() {
    let r = analyze(&[
        "module m\n\
         proc P (d0: u8) clobbers(d0-d3/a1) out(d1: u8) {\n\
             move.w  .tbl(pc,d0.w), d0\n\
             jmp     .tbl(pc,d0.w) targets(.a, .b)\n\
         .tbl:\n\
             dc.w    .a-.tbl\n\
             dc.w    .b-.tbl\n\
         .a:\n\
             move.b  d0, d1\n\
             rts\n\
         .b:\n\
             rts\n\
         }\n",
    ]);
    assert!(out_fires(&r, "P", "d1"), "one landing produces no d1 → fires (verification stays inside): {:?}", r.out_firings);
}

// ===========================================================================
// NON-VACUITY — the real corpus HAS the computed-dispatch idiom the wiring is for.
// ===========================================================================

/// Collect every `.emp` file under `dir`.
fn emp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            emp_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

/// A computed INTRA-PROC dispatch site is a `jmp` whose operand is the `(d8,PC,Xn)`
/// (`PcRelIdx`) or symbol-`(An)` (`DispSymInd`) form — the shapes the out-verifier
/// now sees through via `targets(...)`. The `(An,Xn)` external-hook form
/// (`jmp (a1,d1.w) as PlayerHook`) is a genuine transfer out and is NOT one of
/// these operands, so it is excluded by construction.
fn is_computed_dispatch_op(o: &CodeOperand) -> bool {
    matches!(o, CodeOperand::PcRelIdx { .. } | CodeOperand::DispSymInd { .. })
}

/// The census the brief names, so the targets-wiring gates above cannot pass by
/// finding nothing: the reference corpus carries EXACTLY 6 computed intra-proc
/// dispatch sites across 5 procs. A drift in either number is a corpus change the
/// wiring's assumptions must be re-checked against.
#[test]
fn corpus_computed_dispatch_census_is_six_sites_five_procs() {
    let aeon = sigil_harness::test_support::aeon_dir();
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());

    let mut sites = 0usize;
    let mut procs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for p in &paths {
        let src = std::fs::read_to_string(p).unwrap();
        let (file, _diags) = parse_str(&src);
        let mut counter = 0u32;
        for item in &file.items {
            let Item::Proc(pr) = item else { continue };
            let (buf, _d, next) = eval_proc_body(
                &file,
                &pr.name,
                &pr.params,
                &pr.body,
                pr.span,
                counter,
                Cpu::M68000,
                &[],
                &InterfaceEnv::empty(),
            );
            counter = next;
            let Some(buf) = buf else { continue };
            for it in &buf.items {
                if let CodeItem::Instr { mnemonic, ops, .. } = it {
                    if mnemonic == "jmp" && ops.iter().any(is_computed_dispatch_op) {
                        sites += 1;
                        procs.insert(pr.name.clone());
                    }
                }
            }
        }
    }
    assert_eq!(sites, 6, "computed intra-proc dispatch sites (procs: {procs:?})");
    assert_eq!(procs.len(), 5, "distinct enclosing procs: {procs:?}");
    // Named-exemplar membership, not counts alone: the two procs whose dispatches
    // this parcel adopted `targets(...)` on must be IN the set, so a corpus change
    // that swapped one dispatch for an unrelated one could not keep the count at 6.
    for exemplar in ["Player_SensorSurface", "Player_SensorWallDir"] {
        assert!(procs.contains(exemplar), "census must contain `{exemplar}`: {procs:?}");
    }
}

// ===========================================================================
// EXHAUSTIVENESS TRIPWIRE — `out_edges` makes a `targets(...)` list SOUNDNESS-
// load-bearing for out-verify (a verified out feeds must-def), and the list is
// unvalidated in general (nothing proves it names EVERY reachable landing). A
// `targets` list that OMITS a reachable landing in an OUT-declaring proc would
// verify a false out. The corpus is safe today only by accident — the two
// dispatch-carrying procs declare no `out`, and both tables are `andi`-clamped.
// This pins that accident: NO out-declaring proc may carry a `targets(...)`
// dispatch until the exhaustiveness question is answered (ledger, the
// "exhaustiveness trust" row).
// ===========================================================================

/// Does this proc declare ANY output (a register out, a flag out, or a
/// conditional out)? An `out(dN: T)` shows up in `out` with its width in
/// `out_types`, so the register list is the one to read.
fn declares_out(p: &sigil_frontend_emp::ast::ProcDecl) -> bool {
    p.out.as_ref().is_some_and(|v| !v.is_empty()) || !p.out_flags.is_empty() || !p.out_cond.is_empty()
}

/// The procs in `files` that BOTH declare an out AND carry a `targets(...)`
/// dispatch — the blessing shape the tripwire forbids. Empty is the pass.
fn out_declaring_targets_dispatches(files: &[File]) -> Vec<String> {
    let mut hits = Vec::new();
    for file in files {
        let mut counter = 0u32;
        for item in &file.items {
            let Item::Proc(pr) = item else { continue };
            if !declares_out(pr) {
                counter = eval_proc_body(
                    file, &pr.name, &pr.params, &pr.body, pr.span, counter, Cpu::M68000, &[],
                    &InterfaceEnv::empty(),
                ).2;
                continue;
            }
            let (buf, _d, next) = eval_proc_body(
                file, &pr.name, &pr.params, &pr.body, pr.span, counter, Cpu::M68000, &[],
                &InterfaceEnv::empty(),
            );
            counter = next;
            let Some(buf) = buf else { continue };
            let has_targets = buf.items.iter().any(|it| {
                matches!(it, CodeItem::Instr { targets, .. } if !targets.is_empty())
            });
            if has_targets {
                hits.push(pr.name.clone());
            }
        }
    }
    hits
}

/// The failure text: names the trust boundary and points at the ledger row, so
/// whoever writes the forbidden combination confronts exhaustiveness instead of
/// silently inheriting a blessing path.
fn tripwire_message(hits: &[String]) -> String {
    format!(
        "TRUST BOUNDARY: out-verify's soundness through a computed `targets(...)` \
         dispatch rests on the author-supplied landing list being EXHAUSTIVE, a \
         list omitting a reachable landing verifies a false `out()`, which then \
         feeds must-def as a definition. Nothing validates exhaustiveness in \
         general; the corpus was safe only because no out-declaring proc used the \
         combination. These now do: {hits:?}. Before adopting it, add a \
         clamp-provability or table-length guard, see campaign-gap-ledger.md, the \
         \"exhaustiveness trust\" row (lane-cfg, 2026-08-08)."
    )
}

/// The invariant: NO out-declaring proc in the corpus carries a `targets(...)`
/// dispatch, so out-verify never blesses an out through an unvalidated
/// enumeration.
#[test]
fn no_out_declaring_proc_carries_a_targets_dispatch() {
    let aeon = sigil_harness::test_support::aeon_dir();
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());
    let files: Vec<File> =
        paths.iter().map(|p| parse_str(&std::fs::read_to_string(p).unwrap()).0).collect();

    let hits = out_declaring_targets_dispatches(&files);
    assert!(hits.is_empty(), "{}", tripwire_message(&hits));
}

/// Negative probe: an out-declaring proc WITH a `targets(...)` dispatch trips the
/// scan, and the message names the trust boundary and the ledger row.
#[test]
fn an_out_declaring_targets_dispatch_trips_the_tripwire() {
    let (file, diags) = parse_str(
        "module m\n\
         proc P (d0: u8) clobbers(d0-d3/a1) out(d1: u8) {\n\
             move.w  .tbl(pc,d0.w), d0\n\
             jmp     .tbl(pc,d0.w) targets(.a, .b)\n\
         .tbl:\n\
             dc.w    .a-.tbl\n\
             dc.w    .b-.tbl\n\
         .a:\n\
             move.b  d0, d1\n\
             rts\n\
         .b:\n\
             move.b  d0, d1\n\
             rts\n\
         }\n",
    );
    assert!(diags.is_empty(), "parse: {diags:?}");
    let hits = out_declaring_targets_dispatches(&[file]);
    assert_eq!(hits, vec!["P".to_string()], "an out-declaring targets dispatch must trip the scan");
    let msg = tripwire_message(&hits);
    assert!(msg.contains("TRUST BOUNDARY"), "message names the trust boundary: {msg}");
    assert!(msg.contains("EXHAUSTIVE"), "message states the exhaustiveness rest: {msg}");
    assert!(msg.contains("exhaustiveness trust"), "message points at the ledger row: {msg}");
}
