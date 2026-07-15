//! Tranche 14 — the objdef data file (the ObjDef-twin driver).
//!
//! This gate covers the DATA-emission side of object spawning: the `ObjDef`
//! struct-twin (co-located with `Sst` in `engine/objects/sst.emp`), its
//! burst-copy ensure-chain, and (later tests) the `objdef()` emitter + the
//! `test_objects.emp` consumer byte-gated against the AS macro.
//!
//! REFERENCE-DEPENDENT: reads the sibling aeon tree via `AEON_DIR`. For the
//! t14 branch this must point at the worktree
//! (`aeon/.worktrees/sigil-emp-tranche14`) per the paired-state gate.

use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;
use sigil_ir::backend::Cpu;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn parse_file(rel: &str) -> sigil_frontend_emp::ast::File {
    let path = aeon_dir().join(rel);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, diags) = parse_str(&src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {diags:?}",
        path.display()
    );
    file
}

/// A synthetic module carrying `sst.emp`'s items (which need `engine.types`
/// prepended) plus any extra source appended — so `offsetof(ObjDef, …)` /
/// `offsetof(Sst, …)` resolve against the real struct decls.
fn sst_ambient_with(extra: &str) -> sigil_frontend_emp::ast::File {
    let types = parse_file("engine/system/types.emp");
    let sst = parse_file("engine/objects/sst.emp");
    let (extra_file, diags) = parse_str(&format!("module probe\n{extra}"));
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "probe parse errors: {diags:?}"
    );
    let mut items = Vec::new();
    items.extend(types.items);
    items.extend(sst.items);
    items.extend(extra_file.items);
    sigil_frontend_emp::ast::File {
        module: sst.module.clone(),
        attrs: sst.attrs.clone(),
        items,
        docs: sst.docs.clone(),
    }
}

/// The eager offsetof ensure-chain (and the whole ObjDef struct layout) must
/// lower WITHOUT error — a broken burst-copy correspondence is a build error.
/// The `extern("SST_*")` drift guards defer to link and do not fire here.
#[test]
fn objdef_burst_copy_ensure_chain_passes() {
    let file = sst_ambient_with("");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (_module, diags) = lower_module(&file, &opts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "ObjDef ensure-chain / layout must lower clean, got: {diags:?}"
    );
}

/// ObjDef is the compact 26-byte record: spot-check the offsets that define
/// the burst-copy correspondence (code_addr@0, x_vel@2, mappings@8, pad@$18).
#[test]
fn objdef_compact_layout_offsets() {
    for (probe, want) in [
        ("const R = offsetof(ObjDef, code_addr)", 0),
        ("const R = offsetof(ObjDef, x_vel)", 2),
        ("const R = offsetof(ObjDef, render_flags)", 6),
        ("const R = offsetof(ObjDef, mappings)", 8),
        ("const R = offsetof(ObjDef, art_tile)", 0xC),
        ("const R = offsetof(ObjDef, anim_table)", 0x12),
        ("const R = offsetof(ObjDef, pad)", 0x18),
        ("const R = sizeof(ObjDef)", 26),
    ] {
        let file = sst_ambient_with(probe);
        let (v, diags) = eval_const(&file, "R");
        assert!(diags.is_empty(), "{probe}: unexpected diagnostics: {diags:?}");
        assert_eq!(v, Some(Value::Int(want)), "{probe}");
    }
}
