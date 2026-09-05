//! S2-D6 item #3, RIDER 1 (the (sp)+ exemption tripwire). The write detector
//! `instr_written_regs` exempts a `movem.l (sp)+, <reglist>` from counting its
//! reglist as a clobber — treating it as a stack RESTORE (preserve-discipline,
//! the parallel of the a7 push/pop exemption). That exemption is SOUND only while
//! every `(sp)+` movem-load in the corpus is the restore-half of a matching
//! `<reglist>, -(sp)` save in the SAME proc (so the reglist genuinely round-trips
//! and is not clobbered). A `(sp)+` movem that popped FRESH values (a
//! stack-argument calling convention) WOULD be a real clobber the detector now
//! silently drops — a flip-blocker-class false negative.
//!
//! This test makes the exemption's soundness a permanent guard, not prose. The
//! PRECISE property (Stage-0 finding: the corpus is NOT literally "0 fresh-pops"):
//! every register a `movem (sp)+, M` LOADS must be either
//!   (a) covered by a matching `-(sp)` SAVE of that register in the same proc
//!       (a genuine restore-to-entry — preserve-discipline), OR
//!   (b) declared in the proc's `clobbers`/`out` (an honestly-declared fresh
//!       load — so the exemption hides NO UNDECLARED clobber, the only unsound
//!       case: an undeclared register would slip the closure error gate).
//! A register in a restore mask that is neither is a genuine hidden fresh-pop —
//! the flip-blocker-class false negative — and fails LOUDLY.
//!
//! `Load_Object` is the live case that forced (b): `movem.l d0-d2/a1, -(sp)` …
//! `movem.l (sp)+, d0-d2/a2` deliberately reloads a2 from a1's saved slot (the
//! object template pointer). a2 is NOT a matching-save restore, but it IS declared
//! `clobbers(a2)` AND independently written via `(a2)+` — so the exemption hides
//! nothing. If a future proc pops a fresh, UNDECLARED register, this fires. (Same
//! pattern as the conditional-external-tail grep-proof that became a regression
//! test.)
//!
//! THIS GATE RUNS PER SHIPPED SHAPE, from sigil-cli. Evaluated with no comptime
//! defines, a `movem (sp)+` restore inside a `DEBUG`/`CRASH_REPORT`/`SOUND_*`-gated
//! arm never lowers, so the exemption over that arm goes unguarded. The gate lives
//! in sigil-cli because that crate depends on `sigil-harness` — the owner of the
//! shape `-D` profiles — as well as `sigil-frontend-emp`; it evaluates every proc
//! under each shipped shape's own `-D` set (`native::shape_defines`) with that
//! shape's bound L1 interface env, so a define-gated stack-restore is scanned exactly
//! when the ROM that ships it turns the arm on. The property holds under EVERY shape.
//! Two non-vacuity guards keep it honest: a floor on the widest shape's visited-
//! restore count, and a WIDENING pin asserting the widest shape strictly exceeds the
//! define-free baseline — the widening is the relocation's whole point, so a
//! define-gated restore that stops lowering must fail loudly, not leave the gate
//! quietly green.
//!
//! Gated on `AEON_DIR` (skips green when the tree is absent, like the port gates).

use sigil_frontend_emp::ast::{File as EmpFile, Item};
use sigil_frontend_emp::corpus_contracts::bind_corpus_interfaces;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::lower::expand_reglist_regs;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{CodeItem, CodeOperand, Reg};
use sigil_harness::native;
use sigil_ir::backend::Cpu;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Expand a canonical movem mask (bit0=D0..bit7=D7, bit8=A0..bit15=A7) to its
/// register-name set (`d0`..`d7`/`a0`..`a7`) — the spelling `expand_reglist_regs`
/// produces, so the two compare directly.
fn mask_to_names(mask: u16) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    for bit in 0..16u32 {
        if mask & (1 << bit) != 0 {
            let name = if bit < 8 { format!("d{bit}") } else { format!("a{}", bit - 8) };
            s.insert(name);
        }
    }
    s
}

fn emp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == ".worktrees") {
                continue;
            }
            emp_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

/// The register-list mask of a `movem <reglist>, -(sp)` SAVE (reglist first,
/// predec a7 last).
fn movem_save_mask(mnem: &str, ops: &[CodeOperand]) -> Option<u16> {
    if mnem != "movem" {
        return None;
    }
    match (ops.first(), ops.last()) {
        (Some(CodeOperand::RegList(mask)), Some(CodeOperand::PreDec(Reg::A7))) => Some(*mask),
        _ => None,
    }
}

/// The register-list mask of a `movem (sp)+, <reglist>` RESTORE (postinc a7
/// first, reglist last) — the exempted load form.
fn movem_restore_mask(mnem: &str, ops: &[CodeOperand]) -> Option<u16> {
    if mnem != "movem" {
        return None;
    }
    match (ops.first(), ops.last()) {
        (Some(CodeOperand::PostInc(Reg::A7)), Some(CodeOperand::RegList(mask))) => Some(*mask),
        _ => None,
    }
}

/// Scan every proc in `files`, evaluated under `defines` + `iface_env`, for the
/// hidden-fresh-pop property. Returns `(violations, restore_count)` — the count is
/// how many `movem (sp)+, …` restores the walk VISITED (the non-vacuity witness).
fn scan(
    files: &[(PathBuf, EmpFile)],
    defines: &[(String, i128)],
    iface_env: &sigil_frontend_emp::contract::InterfaceEnv,
) -> (Vec<String>, usize) {
    let mut violations: Vec<String> = Vec::new();
    let mut restore_count = 0usize;
    let mut counter = 0u32;
    for (path, file) in files {
        for item in &file.items {
            let Item::Proc(p) = item else { continue };
            let (buf, _d, next) = eval_proc_body(
                file, &p.name, &p.params, &p.body, p.span, counter, Cpu::M68000, defines, iface_env,
            );
            counter = next;
            let Some(buf) = buf else { continue };

            // Registers ever SAVED via `movem M, -(sp)` (per-register union of all
            // save masks in the proc) — a matching restore-to-entry.
            let mut saved: BTreeSet<String> = BTreeSet::new();
            for it in &buf.items {
                if let CodeItem::Instr { mnemonic, ops, .. } = it {
                    if let Some(m) = movem_save_mask(mnemonic, ops) {
                        saved.extend(mask_to_names(m));
                    }
                }
            }
            // Registers the proc DECLARES it may destroy or return (clobbers ∪ out)
            // — an exemption that hides one of THESE hides nothing (already visible
            // to the closure's firing check via `allowed`).
            let mut declared = expand_reglist_regs(p.clobbers.as_deref().unwrap_or(&[]));
            declared.extend(expand_reglist_regs(p.out.as_deref().unwrap_or(&[])));

            // Every register LOADED by a `movem (sp)+, M` restore must be a matching
            // save (a) or declared (b); anything else is a hidden fresh-pop.
            for it in &buf.items {
                if let CodeItem::Instr { mnemonic, ops, .. } = it {
                    if let Some(m) = movem_restore_mask(mnemonic, ops) {
                        restore_count += 1;
                        for r in mask_to_names(m) {
                            if !saved.contains(&r) && !declared.contains(&r) {
                                violations.push(format!(
                                    "{}::{}, `movem (sp)+, …` loads `{r}` with a FRESH value \
                                     (no matching `-(sp)` save, not in clobbers/out): the (sp)+ \
                                     clobber-lint exemption would silently drop a real clobber",
                                    path.display(),
                                    p.name,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    (violations, restore_count)
}

/// The reference tree, or `None` when it is absent and strict mode is off.
fn aeon_dir() -> Option<PathBuf> {
    let aeon = sigil_harness::test_support::aeon_dir();
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return None;
    }
    Some(aeon)
}

#[test]
fn every_stack_movem_restore_has_a_matching_save() {
    let Some(aeon) = aeon_dir() else { return };

    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());
    let files: Vec<(PathBuf, EmpFile)> =
        paths.iter().map(|p| (p.clone(), parse_str(&std::fs::read_to_string(p).unwrap()).0)).collect();
    let just_files: Vec<EmpFile> = files.iter().map(|(_, f)| f.clone()).collect();

    // The DEFINE-FREE baseline (this gate's earlier no-`-D` coverage) — reported
    // beside the per-shape scan so a shape that stops widening is visible.
    let empty_env = sigil_frontend_emp::contract::InterfaceEnv::empty();
    let (base_viol, base_count) = scan(&files, &[], &empty_env);
    assert!(base_viol.is_empty(), "define-free baseline hidden fresh-pop(s):\n{}", base_viol.join("\n"));

    // Per shape: evaluate every proc under the shape's own `-D` set + bound L1
    // interface env, so DEBUG/CRASH_REPORT/SOUND-gated stack restores are scanned
    // exactly when the shipped ROM turns their arm on.
    let mut widest = 0usize;
    let mut census = format!("define-free baseline: {base_count} restores");
    let aeon = aeon_dir().expect("aeon tree present");
    for (label, profile) in native::shipped_shapes() {
        let defines =
            native::shape_defines(&profile, &aeon).expect("shape defines");
        let (iface_env, bind_diags) =
            bind_corpus_interfaces(&just_files, &defines, profile.game_module_prefix());
        assert!(
            bind_diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "shape `{label}`: L1 bind errors: {bind_diags:?}"
        );
        let (viol, count) = scan(&files, &defines, &iface_env);
        assert!(
            viol.is_empty(),
            "(sp)+ movem-restore exemption tripwire under shape `{label}`, {} hidden fresh-pop(s); \
             re-adjudicate the exemption before shipping:\n{}",
            viol.len(),
            viol.join("\n")
        );
        census.push_str(&format!("; {label}: {count}"));
        widest = widest.max(count);
    }
    eprintln!("== movem-restore census ==\n{census}");

    // NON-VACUOUS: the WIDEST shape must actually exercise the guarded form. The
    // live aeon tree visits 32 `movem (sp)+, …` restores in the define-free arms
    // and 33 in the widest shape; the per-shape scan can only add. A floor of 20
    // tolerates minor churn while catching a sweep that silently stops visiting the
    // guarded instructions (an eval/walker regression that would make this pass
    // emptily).
    assert!(
        widest >= 20,
        "expected 32+ `movem (sp)+, …` restores in the widest shape, visited only {widest}, \
         the guard has gone (near-)vacuous"
    );

    // WIDENING PIN: the widest shape must scan STRICTLY MORE restores than the
    // define-free baseline (33 > 32 today — the `SOUND_DRIVER_ENABLED`-gated
    // restore). The widening is the whole point of running per shape; without this,
    // a define-gated restore that stops lowering leaves the gate quietly green while
    // its exemption goes back to unguarded.
    assert!(
        widest > base_count,
        "the per-shape scan visited {widest} restores, no more than the define-free baseline \
         {base_count}, the define-gated arms are no longer widening coverage; a gated \
         `movem (sp)+` restore has stopped lowering and its exemption is unguarded again"
    );
}
