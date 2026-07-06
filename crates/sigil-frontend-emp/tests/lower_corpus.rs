//! T7 (Plan 4) — the end-to-end LOWERING corpus: Appendix-C / Appendix-D style
//! `.emp` programs driven all the way through `lower_module` → link → bytes,
//! plus the acceptance diagnostics fired from REAL programs and the D-P4.11
//! comptime-generator provenance note.
//!
//! Scope discipline (T7): this corpus uses ONLY features Plan 4 actually lowers.
//! Natural Appendix-D features that do NOT lower yet are simplified around and
//! called out with a `// GAP:` note here (and enumerated in the T7 report):
//!
//!  - GAP (SST overlay + typed field access): `vars … sst_custom { timer: u8 }`
//!    with `timer(a0)` as a typed field-access-as-displacement does not lower —
//!    the overlay is not threaded into instruction operands. Simplified to an
//!    explicit `36(a0)` displacement (the raw offset the overlay would compute).
//!  - GAP (prelude names): `Collision.Hurt`, `Map_PitcherPlant`, `Draw_Sprite`,
//!    `spawn`/`anim`/`routine` helpers come from a game prelude that does not
//!    exist in a standalone module — replaced with local types / defined labels.
//!  - GAP (proc-name-as-pointer-value): Appendix D's `code: init` names a PROC
//!    label as a pointer value, but a proc name is not a comptime value — only a
//!    comptime-fn name (`FnRef`) or a string symbol reference lowers as a pointer
//!    target. Written `code: "init"` here (a string symbol ref the linker
//!    resolves to the `init` proc's address), which lowers to the same Abs32Be.
//!  - GAP (`data X = <const>` type inference): a `data` whose initializer is a
//!    plain const reference (not a struct literal) is NOT type-inferred, so
//!    Appendix C's `data P68 = Patch` needs an explicit `: [u8; N]` annotation.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Expr, Fixup, FixupKind, Fragment, Module, Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

// ---- shared helpers (mirror the per-feature lower_* test files) -------------

/// Parse + lower `src` for the 68k, asserting a clean parse. Returns the module
/// and lowering diagnostics.
fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 })
}

/// Find a section by name (default `text` sections interleave with placed ones).
fn section<'a>(module: &'a Module, name: &str) -> &'a Section {
    module.sections.iter().find(|s| s.name == name).unwrap_or_else(|| {
        panic!("no section `{name}` in {:?}", module.sections.iter().map(|s| &s.name).collect::<Vec<_>>())
    })
}

/// The section-relative offset of a label in a section (from lowering, pre-link).
fn label_offset(sec: &Section, name: &str) -> u32 {
    sec.labels.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("no label `{name}`")).offset
}

/// Link a module and return a named section's resolved bytes.
fn linked_section_bytes(module: &Module, name: &str) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

fn has_tag(diags: &[Diagnostic], tag: &str) -> bool {
    diags.iter().any(|d| d.message.contains(tag))
}

// =============================================================================
// Part 1 — Appendix-D program: a checked ObjDef table + declared-fallthrough
//          procs + a comptime-fn-generated Code splice, end-to-end.
// =============================================================================

/// The Appendix-D corpus module. Self-contained (no prelude): local `struct` /
/// `bitfield` / `enum` stand in for the game's `ObjDef` / `ArtTile` / `Anim`.
///
/// `ObjDef` layout (declaration order, no padding — struct fields pack tight):
///
/// | field  | type            | bytes | offset |
/// |--------|-----------------|-------|--------|
/// | code   | `*u8` (pointer) | 4     | 0      |  → Abs32Be fixup to `"init"`
/// | art    | `ArtTile` (u16) | 2     | 4      |
/// | size   | `Size` (u8,u8)  | 2     | 6      |
/// | anim   | `Anim` (u8)     | 1     | 8      |
/// | zpri   | u8              | 1     | 9      |
///
/// All word/long fields sit at even offsets → no `[layout.odd-field]` warning.
/// Total = 10 bytes.
const APPENDIX_D: &str = "\
module m

struct Size { w: u8, h: u8 }
bitfield ArtTile: u16 { pri: 1, pal: 2, flip: 2, tile: 11 }
enum Anim: u8 { Idle = 0, Seed = 1, Shoot = 2 }
struct ObjDef { code: *u8, art: ArtTile, size: Size, anim: Anim, zpri: u8 }

data Def: ObjDef = ObjDef{
    code: \"init\",
    art:  ArtTile{ pri: 0, pal: 1, flip: 0, tile: $2AB },
    size: Size{ w: 16, h: 28 },
    anim: Anim.Shoot,
    zpri: 3,
}

comptime fn seed_init() -> Code {
    return asm {
        moveq #1, d1
    }
}

proc init (a0: *u8) falls_into wait {
    move.b #64, 36(a0)
}
proc wait (a0: *u8) clobbers(d0) {
    subq.b #1, 36(a0)
    bne.w  .again
    moveq  #0, d0
.again:
    rts
}
proc seed (a0: *u8) {
    seed_init()
    rts
}
";

#[test]
fn appendix_d_objdef_table_byte_diff() {
    let (module, diags) = lower(APPENDIX_D);
    // Only the default-on undeclared/clobber lints are permissible; there must be
    // NO errors.
    assert!(
        !diags.iter().any(|d| d.level == Level::Error),
        "unexpected lowering errors: {diags:?}"
    );

    let text = section(&module, "text");

    // --- the ObjDef data table, hand-computed (§8.3) -------------------------
    // code:  *u8 → 4-byte Abs32Be hole (00 00 00 00 pre-link) at offset 0.
    // art:   ArtTile{pri:0,pal:1,flip:0,tile:$2AB}
    //        = 0<<15 | 1<<13 | 0<<11 | 0x2AB = 0x2000 + 0x2AB = 0x22AB → BE 22 AB.
    // size:  Size{w:16,h:28} → 10 1C.
    // anim:  Anim.Shoot = 2 → 02.
    // zpri:  3 → 03.
    let def_frag = text
        .fragments
        .iter()
        .find_map(|f| match f {
            Fragment::Data(d) if !d.fixups.is_empty() => Some(d),
            _ => None,
        })
        .expect("the Def data fragment (it carries the code pointer fixup)");
    assert_eq!(
        def_frag.bytes,
        vec![0x00, 0x00, 0x00, 0x00, 0x22, 0xAB, 0x10, 0x1C, 0x02, 0x03],
        "ObjDef table pre-link bytes",
    );
    // The pointer field is an Abs32Be fixup at offset 0 targeting `init`.
    assert_eq!(
        def_frag.fixups,
        vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("init".into()) }],
    );

    // --- proc labels land in declaration order, contiguous after the table ---
    // Def is 10 bytes → init starts at offset 10; the other labels follow.
    assert_eq!(label_offset(text, "Def"), 0);
    assert_eq!(label_offset(text, "init"), 10, "init follows the 10-byte Def table");
    assert!(label_offset(text, "wait") > label_offset(text, "init"));
    assert!(label_offset(text, "seed") > label_offset(text, "wait"));

    // --- link: the code pointer resolves to init's VMA (== its offset, VMA==LMA)
    let init_off = label_offset(text, "init");
    let linked = linked_section_bytes(&module, "text");
    assert_eq!(
        &linked[0..4],
        &(init_off).to_be_bytes(),
        "code pointer resolves to init's VMA (Abs32Be, big-endian)",
    );
    // The rest of the table is unchanged by linking.
    assert_eq!(&linked[4..10], &[0x22, 0xAB, 0x10, 0x1C, 0x02, 0x03]);
}

#[test]
fn appendix_d_comptime_fn_code_splice_lands_in_proc_body() {
    // The T3 asm→Code path end-to-end: `seed_init()` (a comptime fn returning
    // `asm { moveq #1, d1 }`) is spliced into `proc seed`'s body as a
    // statement-call. The proc's first two emitted bytes must be that instruction
    // (moveq #1,d1 = 72 01), proving the generated Code reached the byte stream.
    let (module, diags) = lower(APPENDIX_D);
    assert!(!diags.iter().any(|d| d.level == Level::Error), "unexpected errors: {diags:?}");

    let text = section(&module, "text");
    let seed_off = label_offset(text, "seed") as usize;
    let linked = linked_section_bytes(&module, "text");
    assert_eq!(
        &linked[seed_off..seed_off + 2],
        &[0x72, 0x01],
        "the spliced `moveq #1, d1` opens proc seed's body",
    );
    // rts (4E 75) terminates the proc right after the splice.
    assert_eq!(&linked[seed_off + 2..seed_off + 4], &[0x4E, 0x75]);
}

#[test]
fn appendix_d_declared_fallthrough_is_clean() {
    // `proc init falls_into wait` with `wait` the immediately-following proc:
    // NO `[proc.fallthrough-separated]`, and the declared fall suppresses init's
    // undeclared-fallthrough warning.
    let (_module, diags) = lower(APPENDIX_D);
    assert!(!has_tag(&diags, "[proc.fallthrough-separated]"), "adjacent fall must be clean: {diags:?}");
    // `init` declares the fall, so it must not warn about undeclared fallthrough.
    // (`wait` and `seed` both end in `rts`, so they don't warn either.)
    assert!(!has_tag(&diags, "[proc.undeclared-fallthrough]"), "no undeclared-fall warnings: {diags:?}");
}

// =============================================================================
// Part 2 — Appendix-C `pbyte` dual-CPU data: one source of truth, two sections.
// =============================================================================

#[test]
fn appendix_c_pbyte_dual_cpu_emits_identical_bytes() {
    // One `[u8; 4]` const included by VALUE into a 68k section and a Z80 section.
    // At width 1 the byte order is irrelevant (§4.5): both sections emit the SAME
    // four bytes, at their respective (continuous) LMAs.
    //
    // GAP: Appendix C writes `data P68 = Patch` (type inferred from the const);
    // emp only infers a data type from a struct-literal initializer, so a plain
    // const reference needs the explicit `: [u8; 4]` annotation used here.
    let src = "\
module m
pub const Patch: [u8; 4] = [4, 113, 32, 9]
section rom_68k (vma: $8000) {
    data P68: [u8; 4] = Patch
}
section z80_driver (cpu: z80, vma: $8000) {
    data PZ80: [u8; 4] = Patch
}
";
    let (module, diags) = lower(src);
    assert!(!diags.iter().any(|d| d.level == Level::Error), "unexpected errors: {diags:?}");

    let want = vec![0x04, 0x71, 0x20, 0x09];
    assert_eq!(linked_section_bytes(&module, "rom_68k"), want, "68k section bytes");
    assert_eq!(linked_section_bytes(&module, "z80_driver"), want, "z80 section bytes (identical at width 1)");

    // Both are placed: the 68k copy at LMA 0, the Z80 copy right after it at LMA 4
    // (the continuous physical counter), each with its own vma:$8000 base.
    assert_eq!(section(&module, "rom_68k").lma, 0);
    assert_eq!(section(&module, "rom_68k").cpu, Cpu::M68000);
    assert_eq!(section(&module, "z80_driver").lma, 4);
    assert_eq!(section(&module, "z80_driver").cpu, Cpu::Z80);
}

// =============================================================================
// Part 3 — ProvFrame::Comptime provenance (§9, D-P4.11): the smallest honest
//          version — a call-site note when a spliced comptime generator errors.
// =============================================================================

#[test]
fn comptime_generated_table_error_names_the_generator_call_site() {
    // A comptime fn whose body errors (an out-of-range enum cast, `[enum.out-of-
    // range]`, fired at the generator's DEF site) is spliced into a proc body.
    // The splice site adds a `[prov.comptime]` NOTE naming the generator CALL
    // site — so the error inside the comptime-generated table is traceable back
    // to where it was instantiated (D-P4.11).
    //
    // FLAGGED remainder: this is a `Note`, not a structured `ProvFrame::Comptime
    // { call_site, def_site }` — Core reserves no provenance stack on a
    // `Diagnostic`/`DataFragment`, so the full frame is deferred (see T7 report).
    let src = "\
module m
enum Anim: u8 { Idle = 0, Seed = 1, Shoot = 2 }
comptime fn bad_gen(n: int) -> Code {
    let bad = Anim(n)
    return asm { moveq #0, d0 }
}
proc p (a0: *u8) {
    bad_gen(99)
    rts
}
";
    let (_module, diags) = lower(src);
    // The generator's own error (def-site provenance) fires.
    assert!(has_tag(&diags, "[enum.out-of-range]"), "expected the generator's error: {diags:?}");
    // The call-site provenance note (call_site provenance) fires and is a Note.
    let note = diags
        .iter()
        .find(|d| d.message.contains("[prov.comptime]"))
        .expect("expected a [prov.comptime] call-site note");
    assert_eq!(note.level, Level::Note, "provenance is a follow-up note, not an error");
}

#[test]
fn clean_comptime_splice_emits_no_provenance_note() {
    // Guard: a comptime splice that generates a WELL-FORMED table produces no
    // provenance note (the note is strictly error-follow-up, D-P4.11).
    let (_module, diags) = lower(APPENDIX_D);
    assert!(!has_tag(&diags, "[prov.comptime]"), "a clean splice must not note: {diags:?}");
}

// =============================================================================
// Part 4 — the acceptance diagnostics, each fired from a REAL lowered program.
// =============================================================================

#[test]
fn acceptance_wrong_kind_asm_splice_from_a_real_program() {
    // `[asm.splice-kind]`: a comptime fn splices a STRING into a size position
    // (`cmp.{w}` expects a `Width`). Fired through `lower_module` (the generator
    // is called from a proc-body splice).
    let src = "\
module m
comptime fn g(w: string) -> Code {
    return asm { cmp.{w} #1, d0 }
}
proc p (a0: *u8) {
    g(\"oops\")
    rts
}
";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().any(|d| d.message.contains("[asm.splice-kind]") && d.message.contains("Width")),
        "expected [asm.splice-kind] naming Width, got: {diags:?}"
    );
}

#[test]
fn acceptance_falls_into_separated_from_a_real_program() {
    // `[proc.fallthrough-separated]`: `init falls_into wait` with a proc BETWEEN
    // them breaks physical adjacency.
    let src = "\
module m
proc init (a0: *u8) falls_into wait {
    move.b #64, 36(a0)
}
proc middle (a0: *u8) {
    rts
}
proc wait (a0: *u8) {
    rts
}
";
    let (_module, diags) = lower(src);
    let d = diags
        .iter()
        .find(|d| d.message.contains("[proc.fallthrough-separated]"))
        .expect("expected [proc.fallthrough-separated]");
    assert_eq!(d.level, Level::Error);
}

#[test]
fn acceptance_cross_cpu_unwindowed_pointer_from_a_real_program() {
    // `[cross-cpu.unwindowed-pointer]`: a plain (un-windowed) 68k-address pointer
    // in a Z80 section — the pointer needs an explicit `winptr(sym)`.
    let src = "\
module m
section z (cpu: z80, vma: $8000) {
    data BadP: *u8 = \"Target\"
}
";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().any(|d| d.message.contains("[cross-cpu.unwindowed-pointer]") && d.message.contains("Target")),
        "expected [cross-cpu.unwindowed-pointer] naming Target, got: {diags:?}"
    );
}

// NOTE on `[patch.unbound]`: T5 established that comptime `patch`/`bind`
// statements have NO section-emission position in the current surface (the
// `Stmt::Patch`/`Stmt::Bind` arms are no-ops until a section-emission context
// exists — see `lower/patch.rs`'s module doc). So an unbound `patch` CANNOT be
// fired from a real `.emp` program yet; the acceptance coverage for
// `[patch.unbound]` is the T5 standalone-primitive test
// `lower_patch.rs::unbound_patch_is_patch_unbound`, which drives the
// `PatchTable` lowering primitive directly. This is stated explicitly rather
// than contrived here.
