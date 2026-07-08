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

// Test helper: collect every symbol name appearing in a fragment's fixup targets.
fn fixup_targets(f: &sigil_ir::Fragment) -> Vec<String> {
    let mut out = Vec::new();
    sigil_frontend_emp::resolve::rename::collect_target_syms(f, &mut out);
    out
}
