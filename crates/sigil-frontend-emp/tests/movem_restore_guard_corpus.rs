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
//! Gated on `AEON_DIR` (skips green when the tree is absent, like the port gates).

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::lower::expand_reglist_regs;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{CodeItem, CodeOperand, Reg};
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

#[test]
fn every_stack_movem_restore_has_a_matching_save() {
    let Ok(aeon) = std::env::var("AEON_DIR") else {
        eprintln!("skip: AEON_DIR not set");
        return;
    };
    let aeon = PathBuf::from(aeon);
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());

    let mut violations: Vec<String> = Vec::new();
    let mut restore_count = 0usize;
    let mut counter = 0u32;
    for path in &paths {
        let src = std::fs::read_to_string(path).unwrap();
        let (file, _diags) = parse_str(&src);
        for item in &file.items {
            let Item::Proc(p) = item else { continue };
            let (buf, _d, next) = eval_proc_body(
                &file, &p.name, &p.params, &p.body, p.span, counter, Cpu::M68000, &[],
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
                                    "{}::{} — `movem (sp)+, …` loads `{r}` with a FRESH value \
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

    assert!(
        violations.is_empty(),
        "(sp)+ movem-restore exemption tripwire — {} hidden fresh-pop(s) found; re-adjudicate \
         the exemption before shipping:\n{}",
        violations.len(),
        violations.join("\n")
    );
    // NON-VACUOUS: the corpus must actually exercise the guarded form. The live
    // aeon tree has 26 `movem (sp)+, …` restores (the Stage-0 census figure); a
    // floor of 20 tolerates minor churn while catching a sweep that silently stops
    // visiting the guarded instructions (an eval/walker regression that would make
    // this pass emptily).
    assert!(
        restore_count >= 20,
        "expected ~26 `movem (sp)+, …` restores in the corpus, visited only {restore_count} — \
         the guard has gone (near-)vacuous"
    );
}
