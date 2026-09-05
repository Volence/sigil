//! The `[name.shadows-*]` pair (D3 / S2-D13(i) + (n)) — two silent name
//! shadows, each made visible at its declaration.
//!
//! `[name.shadows-mnemonic]`: D-PP.1 resolves a leading bareword against the
//! CPU's mnemonic table FIRST and mnemonics win unconditionally, so a
//! `comptime fn` named like an instruction can never be called in the bare
//! statement form — the line assembles the instruction instead.
//!
//! `[name.shadows-import]`: the resolve pass injects each imported `pub data`
//! of struct type as a type-only stub; a local `data` of that name wins for both
//! the base symbol AND the offsets (deliberately — `data_item_struct_name`'s
//! ISSUE-1 rule). Correct, but silent: nothing told the reader which `Player_1`
//! a `Player_1.x_pos` operand addresses.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level};

fn lower(src: &str) -> Vec<Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    diags
}

fn with<'a>(diags: &'a [Diagnostic], lint: &str) -> Vec<&'a Diagnostic> {
    diags.iter().filter(|d| d.message.contains(lint)).collect()
}

// ---- (i) [name.shadows-mnemonic] -------------------------------------------

const MNEM: &str = "[name.shadows-mnemonic]";

/// A comptime fn named `moveq` is unreachable in bare statement form.
#[test]
fn comptime_fn_named_like_a_68k_mnemonic_fires() {
    let diags = lower(
        "\
module m
comptime fn moveq (x: int) -> int { return x }
",
    );
    let f = with(&diags, MNEM);
    assert_eq!(f.len(), 1, "firings: {diags:?}");
    assert_eq!(f[0].level, Level::Warning, "warn tier");
    assert!(f[0].message.contains("`moveq`"), "names the fn: {}", f[0].message);
    assert!(f[0].message.contains("68000"), "names the CPU whose table claims it");
}

/// A comptime fn is CPU-agnostic, so a Z80-only mnemonic shadows too — the fn is
/// in scope for a Z80 section whether or not this module has one.
#[test]
fn a_z80_only_mnemonic_fires_and_names_the_z80() {
    let diags = lower(
        "\
module m
comptime fn djnz (x: int) -> int { return x }
",
    );
    let f = with(&diags, MNEM);
    assert_eq!(f.len(), 1, "firings: {diags:?}");
    assert!(f[0].message.contains("Z80"), "names the CPU: {}", f[0].message);
}

/// The CPU-neutral reserved words (`jbra`/`jbsr`/`dc`) are mnemonics for this
/// purpose too — `is_recognized_mnemonic` reserves them on both CPUs.
#[test]
fn the_cpu_neutral_reserved_words_fire() {
    for name in ["jbra", "jbsr", "dc"] {
        let src = format!("module m\ncomptime fn {name} (x: int) -> int {{ return x }}\n");
        let diags = lower(&src);
        assert_eq!(with(&diags, MNEM).len(), 1, "`{name}` must fire: {diags:?}");
    }
}

/// An ordinary fn name is silent, and a name that merely CONTAINS a mnemonic is
/// not a shadow — the match is on the whole name.
#[test]
fn ordinary_fn_names_are_silent() {
    let diags = lower(
        "\
module m
comptime fn set_timer (x: int) -> int { return x }
comptime fn moveq_helper (x: int) -> int { return x }
comptime fn lea_table (x: int) -> int { return x }
",
    );
    assert!(with(&diags, MNEM).is_empty(), "no shadow: {diags:?}");
}

// ---- (n) [name.shadows-import] ---------------------------------------------

const IMPORT: &str = "[name.shadows-import]";

/// The defining module of the shadowed name — `Player_1` is a `pub data` of a
/// `pub struct`, exactly the shape whose type-only stub the resolver injects.
const PRELUDE_SRC: &str = "\
module m
pub struct Sst { id: u16, x_pos: u16, y_pos: u16 }
pub data Player_1: Sst = Sst{ id: 1, x_pos: 2, y_pos: 3 }
";

/// Write `files` into a temp tree and run the REAL multi-module path (manifest
/// scan → resolve → lower), so the import stub is injected by the resolver
/// rather than hand-stamped. Mirrors `field_operands.rs`'s `build`.
fn build(files: &[(&str, &str)], entry: &str, prelude: Option<&str>) -> Vec<Diagnostic> {
    let dir = tempfile::tempdir().unwrap();
    for (rel, content) in files {
        std::fs::write(dir.path().join(rel), content).unwrap();
    }
    let (manifest, mdiags) = sigil_frontend_emp::resolve::manifest::Manifest::scan(dir.path());
    assert!(
        mdiags.iter().all(|d| d.level != Level::Error),
        "manifest errors: {mdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (_sections, _asserts, diags) =
        sigil_frontend_emp::resolve::build_program(&manifest, entry, prelude, &opts);
    diags
}

/// The end-to-end shadow: the consumer declares its own `data Player_1` while
/// the prelude exports one. The local item wins (that is the ratified ISSUE-1
/// rule and is unchanged) — this only makes the win VISIBLE.
#[test]
fn a_local_data_item_shadowing_an_import_fires() {
    let consumer = "\
module app
struct Local { a: u16, b: u16, c: u16, x_pos: u16 }
data Player_1: Local = Local{ a: 1, b: 2, c: 3, x_pos: $77 }
proc read() {
    move.w Player_1.x_pos, d0
    rts
}
";
    let diags = build(&[("m.emp", PRELUDE_SRC), ("app.emp", consumer)], "app", Some("m"));
    let f = with(&diags, IMPORT);
    assert_eq!(f.len(), 1, "firings: {diags:?}");
    assert_eq!(f[0].level, Level::Warning, "the local item still wins, this only says so");
    assert!(f[0].message.contains("`Player_1`"), "names the item: {}", f[0].message);
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "the shadow resolves cleanly, it is not an error: {diags:?}"
    );
}

/// The same consumer under a DIFFERENT local name is silent — the check is an
/// intersection with the imported names, not "an import exists".
#[test]
fn distinct_names_alongside_an_import_are_silent() {
    let consumer = "\
module app
struct Local { a: u16, b: u16, c: u16, x_pos: u16 }
data Player_2: Local = Local{ a: 1, b: 2, c: 3, x_pos: $77 }
proc read() {
    move.w Player_2.x_pos, d0
    rts
}
";
    let diags = build(&[("m.emp", PRELUDE_SRC), ("app.emp", consumer)], "app", Some("m"));
    assert!(with(&diags, IMPORT).is_empty(), "no shadow: {diags:?}");
}
