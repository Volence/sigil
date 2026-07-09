//! Spec 2, Plan 7 (pitcher_plant tranche) — U3 / D-PP.3: first-class LABEL
//! values.
//!
//! A bareword in comptime VALUE position (a data-item field initializer or a
//! call argument) that does NOT resolve to a local/const/comptime-fn name
//! evaluates to a LABEL VALUE — the SAME link-symbol reference the string form
//! produces today (same `Cell::SymRef` cell at emission, same fixup, byte-
//! identical output). `ObjDef{ code: init }` == `ObjDef{ code: "init" }`. Dotted
//! paths (`mod.proc`) resolve when the module is reachable; the qualified STRING
//! spelling (`"mod.proc"`) is fixed to resolve the same way. Precedence: an
//! existing name (const/comptime) SHADOWS the label interpretation; a register
//! still wins in call-arg position; an otherwise-unknown bareword is a LOUD
//! error (a deferred symbol the linker rejects), never a silent string.
//!
//! Comptime fn params type a label with the new comptime-only `Label` type; an
//! `asm{}` splice of a `Label` param (`jsr {p}` / `lea {p}, a1`) produces the
//! symbol operand exactly like a `string`-typed splice does today.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::{build_program, manifest::Manifest};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

// ---- single-file helpers (mirror bare_calls.rs / lower_corpus.rs) -----------

fn lower(src: &str) -> (sigil_ir::Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] })
}

fn section<'a>(module: &'a sigil_ir::Module, name: &str) -> &'a Section {
    module
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section `{name}`"))
}

fn label_offset(sec: &Section, name: &str) -> u32 {
    sec.labels.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("no label `{name}`")).offset
}

fn linked_section_bytes(module: &sigil_ir::Module, name: &str) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

fn errors(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.as_str()).collect()
}

/// The bytes of proc/data label `name` (from its label to `len` bytes onward).
fn label_bytes(module: &sigil_ir::Module, sec: &str, name: &str, len: usize) -> Vec<u8> {
    let s = section(module, sec);
    let off = label_offset(s, name) as usize;
    let linked = linked_section_bytes(module, sec);
    linked[off..off + len].to_vec()
}

// ---- multi-module helper ----------------------------------------------------

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

/// Build a multi-module program from `(rel_path, source)` files rooted in a
/// tempdir, entering at `entry`. Returns only the diagnostics (byte checks that
/// need placement go through the single-file path).
fn build(files: &[(&str, &str)], entry: &str) -> Vec<Diagnostic> {
    let dir = tempfile::tempdir().unwrap();
    for (rel, content) in files {
        write(dir.path(), rel, content);
    }
    let (manifest, mdiags) = Manifest::scan(dir.path());
    assert!(
        mdiags.iter().all(|d| d.level != Level::Error),
        "manifest errors: {:?}",
        mdiags.iter().filter(|d| d.level == Level::Error).collect::<Vec<_>>()
    );
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] };
    let (_sections, _asserts, diags) = build_program(&manifest, entry, None, &opts);
    diags
}

// =============================================================================
// Byte-identity: bareword == string form (same module)
// =============================================================================

/// A struct with one `*u8` pointer field. `code: init` (bareword) must lower
/// byte-identically to `code: "init"` (string) — same 4-byte absolute pointer
/// to the same in-module label.
const SAME_MODULE: &str = "\
module m
struct E { code: *u8 }
data Bare = E{ code: init }
data Str  = E{ code: \"init\" }
proc init (a0: *u8) { rts }
";

#[test]
fn bareword_label_matches_string_form_bytes() {
    let (module, diags) = lower(SAME_MODULE);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // No `in <section>` header → data AND procs all land in `text`.
    let bare = label_bytes(&module, "text", "Bare", 4);
    let strf = label_bytes(&module, "text", "Str", 4);
    assert_eq!(bare, strf, "bareword and string label forms must be byte-identical");
    // Both point at `init` — resolved to a concrete absolute address (whatever
    // `init` lands at); the point is they AGREE. A bare `Value::Str` in a u8
    // field would instead have type-errored, so equal 4-byte pointers is the
    // proof the bareword became a label, not a string.
    assert_eq!(bare.len(), 4);
}

/// The bareword also resolves to a DATA item (not just a proc): `art: Map` where
/// Map is a `data` item lowers to a pointer to it, same as `art: "Map"`.
const DATA_LABEL: &str = "\
module m
struct E { art: *u8 }
data Bare = E{ art: Map }
data Str  = E{ art: \"Map\" }
data Map: [u8; 2] = [1, 2]
";

#[test]
fn bareword_naming_data_item_matches_string_form() {
    let (module, diags) = lower(DATA_LABEL);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    let bare = label_bytes(&module, "text", "Bare", 4);
    let strf = label_bytes(&module, "text", "Str", 4);
    assert_eq!(bare, strf, "bareword naming a data item must match the string form");
}

// =============================================================================
// Precedence: existing name resolution wins (const shadows label)
// =============================================================================

/// A `const` named like a would-be label SHADOWS the label interpretation: the
/// pointer field takes the CONST's integer value, not a symbol reference. Here
/// the field is a plain `u16` so the const's value emits directly and the label
/// path is never taken.
const CONST_SHADOWS: &str = "\
module m
const init: u16 = $1234
struct E { code: u16 }
data D = E{ code: init }
proc other (a0: *u8) { rts }
";

#[test]
fn const_shadows_label_value() {
    let (module, diags) = lower(CONST_SHADOWS);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // `code` is a u16 taking the const's value $1234 — a 2-byte big-endian word.
    let d = label_bytes(&module, "text", "D", 2);
    assert_eq!(d, vec![0x12, 0x34], "const wins: the field is the const value, not a label");
}

// =============================================================================
// The `label_ctx` propagation boundary (U4 stacks on this exact line)
// =============================================================================

/// The label context PROPAGATES into expressions nested under a wrapped
/// position: an array literal INSIDE a struct field resolves its elements as
/// labels — `E{ table: [a, b] }` == `E{ table: ["a", "b"] }` byte-for-byte.
const NESTED_ARRAY_IN_FIELD: &str = "\
module m
struct E { table: [*u8; 2] }
data Bare = E{ table: [x, y] }
data Str  = E{ table: [\"x\", \"y\"] }
proc x (a0: *u8) { rts }
proc y (a0: *u8) { rts }
";

#[test]
fn nested_array_elements_in_struct_field_resolve_as_labels() {
    let (module, diags) = lower(NESTED_ARRAY_IN_FIELD);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // Two 4-byte absolute pointers per item; bareword and string elements agree.
    let bare = label_bytes(&module, "text", "Bare", 8);
    let strf = label_bytes(&module, "text", "Str", 8);
    assert_eq!(bare, strf, "nested array elements must resolve as labels, matching the string form");
}

/// The boundary's other side: a TOP-LEVEL data-item array initializer is never
/// wrapped in the label context, so its bare elements keep the loud
/// `unknown name` error (they do NOT silently become labels).
#[test]
fn top_level_data_array_bare_elements_keep_unknown_name() {
    let src = "\
module m
data D: [*u8; 2] = [x, y]
proc x (a0: *u8) { rts }
proc y (a0: *u8) { rts }
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("unknown name `x`"))
            && errs.iter().any(|e| e.contains("unknown name `y`")),
        "top-level data array bare elements must keep the loud unknown-name error, got: {errs:?}"
    );
}

// =============================================================================
// Errors — the scope guard (what label values are NOT)
// =============================================================================

/// A Label in a NON-pointer data field is a type error naming `label`.
#[test]
fn label_in_u8_field_is_type_error_naming_label() {
    let src = "\
module m
struct E { n: u8 }
data D = E{ n: init }
proc init (a0: *u8) { rts }
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("label") && (e.contains("u8") || e.contains("integer"))),
        "a label into a u8 field must be a type error naming label, got: {errs:?}"
    );
}

/// No address arithmetic on a label at comptime: `init + 2` is an error.
#[test]
fn label_arithmetic_is_an_error() {
    let src = "\
module m
struct E { code: *u8 }
data D = E{ code: init + 2 }
proc init (a0: *u8) { rts }
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.to_lowercase().contains("label")),
        "arithmetic on a label value must be a loud error naming label, got: {errs:?}"
    );
}

/// An otherwise-unknown bareword in value position is a LOUD error (a deferred
/// symbol the linker rejects), never a silent string.
#[test]
fn unknown_bareword_label_is_a_loud_link_error() {
    let src = "\
module m
struct E { code: *u8 }
data D = E{ code: nonesuch }
";
    let (module, diags) = lower(src);
    // Either eval reports it, or it becomes a deferred symbol the linker rejects
    // — either way there must be an ERROR, and NO silent string emission.
    let mut errs = errors(&diags).into_iter().map(|s| s.to_string()).collect::<Vec<_>>();
    if errs.is_empty() {
        // Nothing at eval time → it is a deferred symbol; the FULL link pipeline
        // (resolve_layout then link, where data fixups resolve) rejects the
        // unresolved reference loudly.
        match sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true) {
            Ok(resolved) => {
                if let Err(ds) = sigil_link::link(&resolved, &SymbolTable::new()) {
                    errs.extend(ds.iter().map(|d| d.message.clone()));
                }
            }
            Err(ds) => errs.extend(ds.iter().map(|d| d.message.clone())),
        }
    }
    assert!(
        errs.iter().any(|e| e.contains("nonesuch")),
        "an unknown bareword label must be a loud error naming the symbol, got: {errs:?}"
    );
}

// =============================================================================
// Call arguments — `routine shoot` / `routine(shoot)` binding a Label param
// =============================================================================

/// A `Label` param, spliced into `jsr {p}`, from a bare-spelling statement call
/// `routine shoot`, must lower byte-identically to the `string`-param
/// equivalent `jsr {t}` with `t = "shoot"`.
const LABEL_PARAM_JSR: &str = "\
module m

comptime fn routine(p: Label) -> Code {
    return asm { jsr {p} }
}
comptime fn routine_str(t: string) -> Code {
    return asm { jsr {t} }
}

proc via_label (a0: *u8) {
    routine shoot
    rts
}
proc via_string (a0: *u8) {
    routine_str(\"shoot\")
    rts
}
proc shoot (a0: *u8) { rts }
";

#[test]
fn label_param_jsr_splice_matches_string_param() {
    let (module, diags) = lower(LABEL_PARAM_JSR);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // The near target relaxes to `jsr <abs>.w` = 4E B8 xx xx (4 bytes) then
    // rts (2) = 6. Read exactly the proc's own 6 bytes (a longer read would
    // spill into the next proc, which differs in length).
    let via_label = label_bytes(&module, "text", "via_label", 6);
    let via_string = label_bytes(&module, "text", "via_string", 6);
    assert_eq!(
        via_label, via_string,
        "a Label-param jsr splice must match the string-param jsr splice byte-for-byte"
    );
    // Sanity: it really is a jsr (4E B8, abs.w after relaxation) — a link
    // symbol reference, not an int folded into the stream.
    assert_eq!(&via_label[0..2], &[0x4E, 0xB8], "must be jsr <abs>.w");
}

/// The `lea {p}, a1` splice of a Label param, from the paren spelling
/// `routine(shoot)`, matches the `string`-param `lea {t}, a1`.
const LABEL_PARAM_LEA: &str = "\
module m

comptime fn load(p: Label) -> Code {
    return asm { lea {p}, a1 }
}
comptime fn load_str(t: string) -> Code {
    return asm { lea {t}, a1 }
}

proc via_label (a0: *u8) {
    load(target)
    rts
}
proc via_string (a0: *u8) {
    load_str(\"target\")
    rts
}
proc target (a0: *u8) { rts }
";

#[test]
fn label_param_lea_splice_matches_string_param() {
    let (module, diags) = lower(LABEL_PARAM_LEA);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // The near target relaxes to `lea <abs>.w, a1` = 43 F8 xx xx (4) then
    // rts (2) = 6; read the proc's own 6 bytes exactly.
    let via_label = label_bytes(&module, "text", "via_label", 6);
    let via_string = label_bytes(&module, "text", "via_string", 6);
    assert_eq!(via_label, via_string, "Label-param lea splice must match string-param lea splice");
    assert_eq!(&via_label[0..2], &[0x43, 0xF8], "must be lea <abs>.w, a1");
}

/// A register still wins over a label in call-arg position (U1 precedence
/// preserved): `facing(d0)` binds a Reg, not a label named `d0`.
const REG_WINS: &str = "\
module m
comptime fn facing(r: Reg) -> Code { return asm { neg.w {r} } }
proc p (a0: *u8) {
    facing d0
    rts
}
";

#[test]
fn register_still_wins_over_label_in_call_arg() {
    let (module, diags) = lower(REG_WINS);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // neg.w d0 = 44 40, rts = 4E 75.
    let bytes = label_bytes(&module, "text", "p", 4);
    assert_eq!(bytes, vec![0x44, 0x40, 0x4E, 0x75], "register wins: neg.w d0");
}

// =============================================================================
// Label-param type errors
// =============================================================================

/// A Label argument into a `u8` param is a type error naming the expected type.
#[test]
fn label_into_u8_param_is_type_error() {
    let src = "\
module m
comptime fn takes(v: u8) -> Code { return asm { move.b #{v}, d0 } }
proc p (a0: *u8) {
    takes shoot
    rts
}
proc shoot (a0: *u8) { rts }
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.to_lowercase().contains("label")),
        "a label into a u8 param must name the label class, got: {errs:?}"
    );
}

/// A non-label (an int) into a `Label` param is a type error naming the expected
/// `Label` type.
#[test]
fn int_into_label_param_is_type_error() {
    let src = "\
module m
comptime fn routine(p: Label) -> Code { return asm { jsr {p} } }
proc p (a0: *u8) {
    routine 5
    rts
}
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("Label")),
        "an int into a Label param must name the expected Label type, got: {errs:?}"
    );
}

// =============================================================================
// Cross-module + dotted paths + the qualified-string fix (multi-module)
// =============================================================================

/// A bareword naming a proc imported via `use` resolves to that proc's label,
/// with no error (the SCE-continuation half of R1).
#[test]
fn cross_module_bareword_after_use_resolves() {
    let helper = "module engine.helper\npub proc init (a0: *u8) { rts }\n";
    let cons = "\
module obj.plant
use engine.helper.{init}
struct E { code: *u8 }
pub data Def = E{ code: init }
";
    let diags = build(
        &[("engine/helper.emp", helper), ("obj/plant.emp", cons)],
        "obj.plant",
    );
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "cross-module bareword after use must resolve cleanly, got: {:?}",
        diags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// The DOTTED bareword `pitcher_plant.init` (module-qualified) resolves when the
/// module is reachable — this is `examples/main.emp`'s documented usage.
#[test]
fn dotted_bareword_module_qualified_resolves() {
    let helper = "module badniks.pitcher_plant\npub proc init (a0: *u8) { rts }\n";
    let cons = "\
module main
use badniks.pitcher_plant.{init}
struct E { code: *u8 }
data Entry = E{ code: pitcher_plant.init }
";
    let diags = build(
        &[("badniks/pitcher_plant.emp", helper), ("main.emp", cons)],
        "main",
    );
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "dotted module-qualified bareword must resolve, got: {:?}",
        diags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// The qualified STRING spelling `"pitcher_plant.init"` — which fails today with
/// `unknown symbol` even when imported — must resolve the same way as the
/// dotted bareword (D-PP.3 folds in the string-form fix).
#[test]
fn qualified_string_spelling_resolves_when_imported() {
    let helper = "module badniks.pitcher_plant\npub proc init (a0: *u8) { rts }\n";
    let cons = "\
module main
use badniks.pitcher_plant.{init}
struct E { code: *u8 }
data Entry = E{ code: \"pitcher_plant.init\" }
";
    let diags = build(
        &[("badniks/pitcher_plant.emp", helper), ("main.emp", cons)],
        "main",
    );
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "qualified string spelling must resolve when imported, got: {:?}",
        diags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
}
