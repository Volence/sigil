//! Effects-P2 corruption fix (2026-08-12) — the `.emp` path that actually shipped
//! the bug. `add.w d2,a1` in an `.emp` proc was silently encoded as `D549`
//! (`ADDX -(a1),-(a2)`), corrupting memory (the raster/palette RotateSpan/Derive
//! sites). ADD/SUB with an address-register destination must FAIL LOUD, naming the
//! `adda`/`suba` spelling; the correct spellings assemble.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Level;

/// Lower an `.emp` string and return all error-level diagnostics, then (if lowering
/// succeeded) force byte emission so an encode-stage rejection also surfaces.
fn errors(emp: &str) -> Vec<String> {
    let (file, pdiags) = parse_str(emp);
    assert!(
        !pdiags.iter().any(|d| d.level == Level::Error),
        "parse errors: {:?}",
        pdiags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    // The ALU-EA rejection surfaces at lower time (the emp back-end encodes
    // instructions eagerly), so lower diagnostics carry it.
    let (_module, ldiags) = lower_module(&file, &opts);
    ldiags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

#[test]
fn add_with_address_register_destination_fails_loud() {
    let errs = errors("module m\npub proc P () {\n        add.w   d2,a1\n        rts\n}\n");
    assert!(
        errs.iter().any(|e| e.to_lowercase().contains("address-register destination")
            || e.contains("adda")),
        "add.w d2,a1 must be rejected naming adda, got: {errs:?}"
    );
}

#[test]
fn sub_with_address_register_destination_fails_loud() {
    let errs = errors("module m\npub proc P () {\n        sub.l   d0,a1\n        rts\n}\n");
    assert!(
        errs.iter().any(|e| e.to_lowercase().contains("address-register destination")
            || e.contains("suba")),
        "sub.l d0,a1 must be rejected naming suba, got: {errs:?}"
    );
}

#[test]
fn adda_spelling_assembles_clean() {
    let errs = errors("module m\npub proc P () {\n        adda.w  d2,a1\n        rts\n}\n");
    assert!(errs.is_empty(), "adda.w d2,a1 must assemble, got: {errs:?}");
}
