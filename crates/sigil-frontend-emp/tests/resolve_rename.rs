use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::rename::rename_module;
use sigil_ir::backend::Cpu;
use std::collections::HashMap;

#[test]
fn renames_labels_and_fixup_targets() {
    // A module whose data points at a proc label; rename both to canonicals.
    let (file, d) = parse_str("module m.a\ndata Def: [*u8; 1] = [init]\nproc init (a0: *u8) {}\n");
    assert!(d.iter().all(|x| x.level != sigil_span::Level::Error), "{d:?}");
    let (mut module, _) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });

    let mut map = HashMap::new();
    map.insert("Def".to_string(), "m.a.Def".to_string());
    map.insert("init".to_string(), "m.a.init".to_string());
    rename_module(&mut module, &map);

    // The proc's entry label is now canonical.
    let has_canon_label =
        module.sections.iter().flat_map(|s| &s.labels).any(|l| l.name == "m.a.init");
    assert!(has_canon_label, "expected renamed label m.a.init");
    // The data fixup target is now canonical (no bare `init` remains).
    let bare_init_target = module
        .sections
        .iter()
        .flat_map(|s| &s.fragments)
        .any(|f| fixup_targets(f).iter().any(|t| t == "init"));
    assert!(!bare_init_target, "bare `init` fixup target should have been renamed");
}

#[test]
fn renames_cross_module_branch_target() {
    // `jmp other` (bare symbol) lowers to a `JmpJsrSym` fragment whose target is
    // the cross-module label — the whole reason rename.rs exists. Renaming must
    // canonicalize that branch target so the flat-symbol linker resolves it.
    let (file, d) = parse_str("module x\nproc a (a0: *u8) { jmp other }\nproc other (a0: *u8) { rts }\n");
    assert!(d.iter().all(|x| x.level != sigil_span::Level::Error), "{d:?}");
    let (mut module, _) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });

    // Confirm the fragment we rely on is actually a JmpJsrSym (not, say, a
    // relaxed abs) — the invariant this test is meant to pin.
    let has_jmpjsr = module
        .sections
        .iter()
        .flat_map(|s| &s.fragments)
        .any(|f| matches!(f, sigil_ir::Fragment::JmpJsrSym { .. }));
    assert!(has_jmpjsr, "expected a JmpJsrSym fragment for `jmp other`");

    let mut map = HashMap::new();
    map.insert("a".to_string(), "x.a".to_string());
    map.insert("other".to_string(), "x.other".to_string());
    rename_module(&mut module, &map);

    // Labels renamed.
    assert!(module.sections.iter().flat_map(|s| &s.labels).any(|l| l.name == "x.other"));
    // Branch target renamed; no bare `other` survives.
    let all_targets: Vec<String> =
        module.sections.iter().flat_map(|s| &s.fragments).flat_map(fixup_targets).collect();
    assert!(all_targets.iter().any(|t| t == "x.other"), "branch target should be canonical, got {all_targets:?}");
    assert!(!all_targets.iter().any(|t| t == "other"), "bare `other` branch target should be gone, got {all_targets:?}");
}

#[test]
fn proc_local_symbols_pass_through_unchanged() {
    // A proc-local `.loop` is hygiene-mangled to `$a$loop`. The rename map holds
    // only bare top-level names, so the mangled local must survive VERBATIM in
    // both the label list and the branch fixup target — hygiene preservation.
    let (file, d) = parse_str("module x\nproc a (a0: *u8) {\n.loop:\n  bra.w .loop\n}\n");
    assert!(d.iter().all(|x| x.level != sigil_span::Level::Error), "{d:?}");
    let (mut module, _) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });

    // Map contains ONLY the bare top-level name — never the mangled local.
    let mut map = HashMap::new();
    map.insert("a".to_string(), "x.a".to_string());
    rename_module(&mut module, &map);

    // Top-level label renamed, mangled local untouched, in labels...
    let labels: Vec<String> =
        module.sections.iter().flat_map(|s| &s.labels).map(|l| l.name.clone()).collect();
    assert!(labels.iter().any(|l| l == "x.a"), "top-level label renamed, got {labels:?}");
    assert!(labels.iter().any(|l| l == "$a$loop"), "mangled local label must survive, got {labels:?}");

    // ...and in the branch fixup target.
    let all_targets: Vec<String> =
        module.sections.iter().flat_map(|s| &s.fragments).flat_map(fixup_targets).collect();
    assert!(all_targets.iter().any(|t| t == "$a$loop"), "mangled local target must survive, got {all_targets:?}");
}

// Test helper: collect every symbol name appearing in a fragment's fixup targets.
fn fixup_targets(f: &sigil_ir::Fragment) -> Vec<String> {
    let mut out = Vec::new();
    sigil_frontend_emp::resolve::rename::collect_target_syms(f, &mut out);
    out
}
