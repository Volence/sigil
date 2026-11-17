//! `resolve::name_env_report` (step-0 A4): the rows `sigil build --report name-env`
//! prints, on the two shapes the report exists to find.
use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{hygiene::NameEnvRow, manifest::Manifest, name_env_report};
use sigil_ir::backend::Cpu;
use sigil_span::Level;

/// Write `files` under a fresh root and run the name-env report from `main`.
fn report(files: &[(&str, &str)]) -> Vec<NameEnvRow> {
    let dir = tempfile::tempdir().unwrap();
    for (rel, src) in files {
        std::fs::write(dir.path().join(rel), src).unwrap();
    }
    let (manifest, d) = Manifest::scan(dir.path());
    assert!(d.iter().all(|d| d.level != Level::Error), "{d:?}");
    let opts =
        LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let (rows, diags) = name_env_report(&manifest, "main", None, &opts);
    assert!(diags.iter().all(|d| d.level != Level::Error), "{diags:?}");
    rows
}

const P8B_A: &str = "module a\npub const N = 4\npub struct S {\n    x: [u8; N],\n    y: u16,\n}\n";
const P8B_MAIN: &str = "module main\nuse a.{S}\nconst N = 10\nsection s (cpu: m68000, vma: $0) {\n  proc P () clobbers(d0) {\n        moveq   #sizeof(S), d0\n        rts\n  }\n}\n";

#[test]
fn report_names_a_struct_size_captured_by_the_importer() {
    let rows = report(&[("a.emp", P8B_A), ("main.emp", P8B_MAIN)]);
    let hit: Vec<_> = rows.iter().filter(|r| r.differs).collect();
    assert_eq!(hit.len(), 1, "{rows:#?}");
    assert_eq!((hit[0].importer.as_str(), hit[0].item.as_str(), hit[0].name.as_str()), ("main", "S", "N"));
    assert_eq!((hit[0].at_def.as_str(), hit[0].caller.as_str()), ("a.N", "main.N"));
    assert!(rows.iter().all(|r| !r.unbound));
}

#[test]
fn report_names_a_helper_name_only_the_importer_binds() {
    let rows = report(&[
        ("helper.emp", "module helper\npub comptime fn lim() -> int {\n    return LIMIT\n}\n"),
        ("main.emp", "module main\nuse helper.{lim}\nconst LIMIT = 9\nsection s (cpu: m68000, vma: $0) {\n  proc P () clobbers(d0) {\n        moveq   #lim(), d0\n        rts\n  }\n}\n"),
    ]);
    let un: Vec<_> = rows.iter().filter(|r| r.unbound).collect();
    assert_eq!(un.len(), 1, "{rows:#?}");
    assert_eq!((un[0].item.as_str(), un[0].name.as_str(), un[0].at_def.as_str()), ("lim", "LIMIT", "-"));
    assert!(rows.iter().all(|r| !r.differs));
}
