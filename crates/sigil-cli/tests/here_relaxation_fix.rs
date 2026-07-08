//! The `here()`-vs-relaxation fix — the design doc's acceptance list (items 1–8),
//! end-to-end (2026-07-08-spec2-plan7-here-relaxation-fix-design.md).
//!
//! A PROVISIONAL `here()` (one after a size-relaxable fragment in its section) is
//! a link-time value: emitted, it becomes a SymRef resolved to the FINAL
//! post-relaxation VMA (D-H.3); guarded, the guard DEFERS to a `LinkAssert` the
//! linker decides against the post-`resolve_layout` symbol table (D-H.4/D-H.6).
//! On master, `here()` folded eagerly to the BASELINE cursor (every relaxable at
//! its smallest rung), so a `jbra` that later GREW made the value stale by the
//! growth delta — the budget-guard idiom checked the wrong number silently.
//!
//! Items 3 and 5 (SymRef byte pin; the `[here.provisional]` refusal matrix) are
//! pinned unit-side in `sigil-frontend-emp/tests/here_provisional.rs`; this file
//! carries the full-pipeline versions of items 1, 2, 4, 6, 7, and 8. Item 6's
//! byte-identity half also rides `ports.rs::example_guards_compiles` (the
//! guards.emp byte pin, untouched by this fix).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;
use std::process::Command;

// ---------------------------------------------------------------------------
// Harness — the FULL emp pipeline including the deferred-assert checker, i.e.
// exactly what the CLI's `link_to_image` runs: parse → lower_module →
// resolve_layout → link → check_link_asserts. Returns the flat image (when the
// build succeeded) plus every diagnostic message, so a test can assert both the
// bytes and the failure text.
// ---------------------------------------------------------------------------

fn compile_full(emp: &str) -> (Option<Vec<u8>>, Vec<String>) {
    let (file, pdiags) = parse_str(emp);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None };
    let (module, ldiags) = lower_module(&file, &opts);
    let mut msgs: Vec<String> = ldiags.iter().map(|d| d.message.clone()).collect();
    if ldiags.iter().any(|d| d.level == Level::Error) {
        return (None, msgs);
    }
    let empty = SymbolTable::new();
    let resolved = match sigil_link::resolve_layout(&module.sections, &empty, true) {
        Ok(r) => r,
        Err(ds) => {
            msgs.extend(ds.into_iter().map(|d| d.message));
            return (None, msgs);
        }
    };
    let image = match sigil_link::link(&resolved, &empty) {
        Ok(img) => img,
        Err(ds) => {
            msgs.extend(ds.into_iter().map(|d| d.message));
            return (None, msgs);
        }
    };
    let assert_diags = sigil_link::check_link_asserts(&resolved, &empty, &module.link_asserts);
    if assert_diags.iter().any(|d| d.level == Level::Error) {
        msgs.extend(assert_diags.into_iter().map(|d| d.message));
        return (None, msgs);
    }
    (Some(sigil_link::flatten(&image, 0x00)), msgs)
}

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

// ---------------------------------------------------------------------------
// Item 1 — the REPRO: a jbra to a far target grows bra.s (2B) → bra.w (4B),
// pushing the guard's position from the stale baseline $8002 to the real $8004.
// The budget `here() <= $8003` PASSES on the stale number (master's silent
// wrong answer) but MUST FAIL on the final one — the build fails at link with
// the guard's message.
// ---------------------------------------------------------------------------

/// Layout at vma $8000: `jbra Far` (baseline rung bra.s = 2 bytes; Far is ~200
/// bytes away so it settles bra.w = 4 bytes), guard at $8004 final ($8002
/// baseline), then 200 pad bytes, then Far. Budget $8003: stale-pass, final-fail.
const BUDGET_FAIL: &str = "module m\n\
    section s (cpu: m68000, vma: $8000) {\n\
      proc p () {\n\
        jbra Far\n\
      }\n\
      ensure_fatal(here() <= $8003, \"overran at {here()}\")\n\
      data Pad = bytes(for i in 0..200 { 0 })\n\
      proc Far () {\n\
        rts\n\
      }\n\
    }\n";

#[test]
fn budget_guard_fails_at_link_after_growth() {
    let (image, msgs) = compile_full(BUDGET_FAIL);
    assert!(image.is_none(), "the overrun budget must FAIL the build, got an image");
    // Item 4 rides the same assertion: the {here()} placeholder renders the REAL
    // final address $8004 = 32772 — not the stale baseline $8002 = 32770.
    assert!(
        msgs.iter().any(|m| m.contains("overran at 32772")),
        "expected the guard message with the FINAL address (32772), got: {msgs:?}"
    );
}

/// The same repro through the REAL binary: exit 1, guard message on stderr.
#[test]
fn budget_guard_fails_via_cli_binary() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "m.emp", BUDGET_FAIL);
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", tmp.path().join("m.emp").to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success(), "overrun budget must exit non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("overran at 32772"), "stderr: {stderr}");
}

// ---------------------------------------------------------------------------
// Item 2 — the mirror positive: the SAME growth, but the budget is still met
// after it → the build passes with zero diagnostics.
// ---------------------------------------------------------------------------

#[test]
fn budget_guard_passes_when_met_after_growth() {
    let src = BUDGET_FAIL.replace("$8003", "$9000");
    let (image, msgs) = compile_full(&src);
    assert!(msgs.is_empty(), "passing budget must be silent, got: {msgs:?}");
    let image = image.expect("passing budget must build");
    // The jbra settled bra.w: 60 00 <disp16>; the guard emitted zero bytes.
    assert_eq!(&image[0..2], &[0x60, 0x00], "jbra grew to bra.w");
}

// ---------------------------------------------------------------------------
// Item 4 — `{here()}` in a deferred message renders the FINAL address. The
// failing test above already pins 32772; this one isolates the message shape:
// comptime parts frozen eagerly, the link-time placeholder folded lazily.
// ---------------------------------------------------------------------------

#[test]
fn deferred_message_mixes_frozen_text_and_final_address() {
    // A comptime const in the message freezes NOW; {here()} folds at link.
    let src = "module m\n\
        const LIMIT = $8003\n\
        section s (cpu: m68000, vma: $8000) {\n\
          proc p () {\n\
            jbra Far\n\
          }\n\
          ensure(here() <= LIMIT, \"limit {LIMIT} exceeded: pos {here()}\")\n\
          data Pad = bytes(for i in 0..200 { 0 })\n\
          proc Far () {\n\
            rts\n\
          }\n\
        }\n";
    let (image, msgs) = compile_full(src);
    assert!(image.is_none(), "the exceeded limit must fail the build");
    // {LIMIT} froze at defer time (32771); {here()} folded at link (32772).
    assert!(
        msgs.iter().any(|m| m.contains("limit 32771 exceeded: pos 32772")),
        "expected eager text + lazy final address, got: {msgs:?}"
    );
}

// ---------------------------------------------------------------------------
// Item 6 (committed half) — the documented examples/guards.emp (exact-position
// guards, all passing) still compiles through the REAL binary with zero
// diagnostics and unchanged output; the byte pin itself lives in
// ports.rs::example_guards_compiles and is untouched by this fix.
// ---------------------------------------------------------------------------

#[test]
fn example_guards_compiles_via_cli_binary_with_zero_diagnostics() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/guards.emp");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", path])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "guards.emp must compile, stderr: {stderr}");
    assert!(stderr.is_empty(), "guards.emp must produce ZERO diagnostics, got: {stderr}");
}

// ---------------------------------------------------------------------------
// Item 7 — two modules, EACH deferring a guard: the anonymous anchors are
// module-qualified (`__here$<module>$<n>`), so the whole-program link (which
// has duplicate-label detection) must not collide.
// ---------------------------------------------------------------------------

#[test]
fn two_modules_deferring_guards_do_not_collide() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // Each module: a jbra (making its guard position provisional) + a deferred,
    // PASSING budget guard. If the two minted anchors collided, link() would
    // report `symbol … redefined` and the build would fail.
    write(
        root,
        "guards/lib.emp",
        "module guards.lib\n\
         pub proc entry () {\n\
           jbra LibFar\n\
         }\n\
         ensure_fatal(here() <= $FFFFF, \"lib overran at {here()}\")\n\
         proc LibFar () {\n\
           rts\n\
         }\n",
    );
    write(
        root,
        "guards/main.emp",
        "module guards.main\n\
         use guards.lib.{entry}\n\
         proc init () {\n\
           jbra Away\n\
         }\n\
         ensure_fatal(here() <= $FFFFF, \"main overran at {here()}\")\n\
         proc Away () {\n\
           jmp entry\n\
         }\n",
    );
    let out_bin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("guards/main.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            out_bin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "two deferred guards across modules must link (no anchor collision), stderr: {stderr}"
    );
    assert!(!stderr.contains("redefined"), "anchor collision: {stderr}");
    assert!(out_bin.exists());
}

// ---------------------------------------------------------------------------
// Item 8 — deferred `ensure` and `ensure_fatal` are identical in effect at link
// (D-H.7: both fail the build), ALL failures are collected, and a deferred
// fatal does NOT suppress lowering of the items after it.
// ---------------------------------------------------------------------------

#[test]
fn deferred_ensure_and_ensure_fatal_both_fail_and_all_collect() {
    // BOTH guards fail their (deferred) budgets; both messages must surface —
    // not first-failure — and the build must fail.
    let src = "module m\n\
        section s (cpu: m68000, vma: $8000) {\n\
          proc p () {\n\
            jbra Far\n\
          }\n\
          ensure(here() <= $8000, \"plain ensure failed\")\n\
          ensure_fatal(here() <= $8000, \"fatal ensure failed\")\n\
          data Pad = bytes(for i in 0..200 { 0 })\n\
          proc Far () {\n\
            rts\n\
          }\n\
        }\n";
    let (image, msgs) = compile_full(src);
    assert!(image.is_none(), "both failing guards must fail the build");
    assert!(msgs.iter().any(|m| m.contains("plain ensure failed")), "got: {msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("fatal ensure failed")), "got: {msgs:?}");
}

#[test]
fn deferred_fatal_does_not_suppress_later_items() {
    // A deferred (provisional) ensure_fatal that will FAIL at link, followed by
    // MORE items. D-H.7: deferral cannot stop lowering — the later items' bytes
    // must exist in the linked image even though the assert then fails the build.
    let src = "module m\n\
        section s (cpu: m68000, vma: $8000) {\n\
          proc p () {\n\
            jbra Far\n\
          }\n\
          ensure_fatal(here() <= $8000, \"will fail at link\")\n\
          data Tail: [u8; 2] = [$AB, $CD]\n\
          proc Far () {\n\
            rts\n\
          }\n\
        }\n";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower: {ldiags:?}");
    // The later items lowered: Tail's label exists and one deferred assert rides
    // the module.
    assert!(
        module.sections.iter().flat_map(|s| &s.labels).any(|l| l.name == "Tail"),
        "the item AFTER the deferred fatal guard must still lower"
    );
    assert_eq!(module.link_asserts.len(), 1);
    // Link the image (pre-check): Tail's bytes are physically present…
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&module.sections, &empty, true).expect("resolve");
    let linked = sigil_link::link(&resolved, &empty).expect("link");
    let bytes = &linked.section("s").expect("section s").bytes;
    // jbra settled bra.s (Far is near: 2 + 2 = offset 4): 60 02; Tail at 2..4.
    assert_eq!(&bytes[2..4], &[0xAB, 0xCD], "Tail's bytes must be present, got {bytes:02X?}");
    // …and the deferred fatal still fails the build at the checker.
    let ds = sigil_link::check_link_asserts(&resolved, &empty, &module.link_asserts);
    assert_eq!(ds.len(), 1, "the deferred fatal must fail at link: {ds:?}");
    assert!(ds[0].message.contains("will fail at link"));
}
