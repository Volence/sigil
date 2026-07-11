//! Spec 2, Plan 7 (pitcher_plant tranche) — U4 / D-PP.5: `Item.field`.
//!
//! HALF A — `Item.field` as a straight-line MEMORY OPERAND denotes the FIELD'S
//! ADDRESS: `move.w Player_1.x_pos, d0`, where `Player_1` is a data item of
//! known struct type and `x_pos` a field, lowers EXACTLY like the bare symbolic
//! operand `move.w Foo, d0` (the #2 `RelaxAbsSym` short/long candidate seam) but
//! with fixup target `Player_1 + offsetof(Sst, x_pos)` — a foldable `Add` the
//! linker resolves and widths by `asl_width_rule` on the SUM.
//!
//! HALF B — `Item.field` in comptime VALUE position denotes the field's VALUE:
//! `art: Def.art`, where `Def` is a module-local data item with a struct-literal
//! initializer, reads the field's comptime value (lazy on-demand init eval with
//! cycle detection). `#Item.field` immediate = value; bare = address (assembly
//! convention).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::{build_program, manifest::Manifest};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

// ---- helpers (mirror label_values.rs) --------------------------------------

fn lower(src: &str) -> (sigil_ir::Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] })
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

fn label_bytes(module: &sigil_ir::Module, sec: &str, name: &str, len: usize) -> Vec<u8> {
    let s = section(module, sec);
    let off = label_offset(s, name) as usize;
    let linked = linked_section_bytes(module, sec);
    linked[off..off + len].to_vec()
}

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

fn build(files: &[(&str, &str)], entry: &str, prelude: Option<&str>) -> (Vec<Section>, Vec<Diagnostic>) {
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
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let (sections, _asserts, diags) = build_program(&manifest, entry, prelude, &opts);
    (sections, diags)
}

// `Sst` is a PUB struct (so the consumer sees the type) and `Player_1` a PUB
// data item of it — the type-only stub the resolver injects gives the consumer
// its `(Player_1 -> Sst)` binding. `x_pos` is the field HALF A dereferences.
const PRELUDE_SRC: &str = "\
module m
pub struct Sst { id: u16, x_pos: u16, y_pos: u16 }
pub data Player_1: Sst = Sst{ id: 1, x_pos: 2, y_pos: 3 }
";

// ---- HALF A: field-address memory operands ---------------------------------

#[test]
fn field_operand_lowers_like_abs_sym_low_address() {
    // `move.w Player_1.x_pos, d0` — MOVE.w abs.w src, d0 dst → 30 38 + addr16.
    // `Player_1` at its (low) placed address, `x_pos @ $10`, so the abs.w
    // address is `Player_1 + $10`, derived from the label offset.
    let src = "\
module m
struct Sst { id: u16, x_pos: u16, y_pos: u16 }
data Player_1: Sst = Sst{ id: 1, x_pos: 2, y_pos: 3 }
proc read() {
    move.w Player_1.x_pos, d0
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    let s = section(&module, "text");
    let base = label_offset(s, "Player_1");
    let addr = base + 0x02;
    let read_bytes = label_bytes(&module, "text", "read", 4);
    assert_eq!(
        read_bytes,
        vec![0x30, 0x38, (addr >> 8) as u8, (addr & 0xFF) as u8],
        "field operand must encode abs.w of Player_1 + $10 = {addr:#x}"
    );
}

#[test]
fn field_operand_high_address_selects_abs_l() {
    // A section with a HIGH vma pushes `Player_1 + $10` above the abs.w range,
    // so the SUM (an `Add` fixup target) folds and selects the abs.l candidate:
    // 30 39 + addr32. Proves Add folding through resolve_layout width selection.
    let src = "\
module m
struct Sst { id: u16, x_pos: u16, y_pos: u16 }
section hi (vma: $00FF0000) {
    data Player_1: Sst = Sst{ id: 1, x_pos: 2, y_pos: 3 }
    proc read() {
        move.w Player_1.x_pos, d0
        rts
    }
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    let s = section(&module, "hi");
    let base = 0x00FF_0000 + label_offset(s, "Player_1");
    let addr = base + 0x02;
    let read_bytes = label_bytes(&module, "hi", "read", 6);
    assert_eq!(
        read_bytes,
        vec![
            0x30,
            0x39,
            (addr >> 24) as u8,
            (addr >> 16) as u8,
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
        ],
        "high field operand must encode abs.l of Player_1 + $10 = {addr:#x}"
    );
}

#[test]
fn field_operand_unknown_field_is_loud_comptime_error() {
    // `Player_1.bogus` on a KNOWN struct-typed item names the struct and field —
    // a comptime error, NOT a silent link-symbol pass-through.
    let src = "\
module m
struct Sst { id: u16, x_pos: u16 }
data Player_1: Sst = Sst{ id: 1, x_pos: 2 }
proc read() {
    move.w Player_1.bogus, d0
    rts
}
";
    let (_module, diags) = lower(src);
    assert!(
        errors(&diags).iter().any(|e| e.contains("Sst") && e.contains("bogus")),
        "expected a loud unknown-field error naming Sst and bogus, got: {:?}",
        errors(&diags)
    );
}

#[test]
fn non_struct_item_falls_through_to_link_symbol() {
    // `helper.entry` where `helper` is a PROC (not a struct-typed data item):
    // the operand keeps today's module-qualified link-symbol pass-through — a
    // deferred reference resolved at link, byte-identical to the bare form. This
    // pins that Half A does NOT hijack the `Owner.label` operand case.
    let src = "\
module m
proc helper() {
    rts
}
proc caller() {
    move.w helper.entry, d0
    rts
}
";
    let (module, _diags) = lower(src);
    // The operand encodes as a RelaxAbsSym targeting the dotted symbol
    // `helper.entry` (unresolved here — link would reject it, but lowering must
    // not error, matching the bare Owner.label behavior).
    use sigil_ir::{Expr, Fragment};
    let s = section(&module, "text");
    let has_dotted = s.fragments.iter().any(|f| {
        matches!(f, Fragment::RelaxAbsSym { target: Expr::Sym(n), .. } if n == "helper.entry")
    });
    assert!(has_dotted, "expected a RelaxAbsSym targeting the dotted `helper.entry` symbol");
}

#[test]
fn jbra_dotted_target_untouched() {
    // `jbra X.y` is a branch target, NOT a memory operand — Half A must leave it
    // exactly as today (a JmpJsrSym-style auto-reaching branch to `X.y`).
    let src = "\
module m
proc caller() {
    jbra other.entry
}
proc other() {
    rts
}
";
    let (_module, diags) = lower(src);
    // Whatever jbra does today it must not gain a new error from Half A.
    assert!(errors(&diags).is_empty(), "jbra dotted target must be untouched: {:?}", errors(&diags));
}

#[test]
fn cross_module_field_operand_resolves_via_prelude() {
    // `Player_1` lives in the prelude (`pub data`). A consumer references
    // `Player_1.x_pos` as a memory operand; the type is visible cross-module
    // (stamped, per T0a), so lowering emits a field-address operand with NO
    // error — the deferred `Player_1 + $10` reference resolves at link.
    let consumer = "\
module app
proc read() {
    move.w Player_1.x_pos, d0
    rts
}
";
    let (sections, diags) = build(
        &[("m.emp", PRELUDE_SRC), ("app.emp", consumer)],
        "app",
        Some("m"),
    );
    assert!(
        errors(&diags).is_empty(),
        "cross-module field operand must resolve cleanly, got: {:?}",
        errors(&diags)
    );
    // Prove the mechanism: the `read` proc's field operand is a RelaxAbsSym whose
    // fixup target is `Player_1 + 2` (an `Add`), and its inner symbol has been
    // canonicalized to the prelude's `game.prelude.Player_1`-style qualified name
    // — NOT a raw dotted `Player_1.x_pos` link symbol. This distinguishes the
    // field-ADDRESS lowering from a fall-through pass-through.
    use sigil_ir::{Expr, Fragment};
    let read_frag = sections
        .iter()
        .flat_map(|s| s.fragments.iter())
        .find_map(|f| match f {
            Fragment::RelaxAbsSym { target: Expr::Binary { lhs, rhs, .. }, .. } => {
                Some((lhs.clone(), rhs.clone()))
            }
            _ => None,
        })
        .expect("expected a RelaxAbsSym with an Add target for the field operand");
    let (lhs, rhs) = read_frag;
    assert!(
        matches!(&*lhs, Expr::Sym(n) if n.ends_with("Player_1")),
        "field operand's base symbol must be the canonicalized Player_1, got {lhs:?}"
    );
    assert_eq!(*rhs, Expr::Int(2), "field operand offset must be offsetof(Sst, x_pos) = 2");
}

#[test]
fn local_data_item_shadows_imported_type_stub() {
    // NAME SHADOW coherence (spec-review ISSUE-1): the consumer declares its OWN
    // `data Player_1: Local` while the prelude also exports `pub data Player_1:
    // Sst` (whose type-only stub is injected). The LOCAL item must win for BOTH
    // the base symbol AND the struct type/offsets — mixing the local base with
    // the imported struct's offset would emit a silently-wrong address. Local
    // `x_pos` sits at offset 6 (`a,b,c: u16` precede it); imported `Sst.x_pos`
    // sits at 2 — the operand's Add target must carry 6.
    let consumer = "\
module app
struct Local { a: u16, b: u16, c: u16, x_pos: u16 }
data Player_1: Local = Local{ a: 1, b: 2, c: 3, x_pos: $77 }
data Copy: Local = Local{ a: 1, b: 2, c: 3, x_pos: Player_1.x_pos }
proc read() {
    move.w Player_1.x_pos, d0
    rts
}
";
    let (sections, diags) = build(
        &[("m.emp", PRELUDE_SRC), ("app.emp", consumer)],
        "app",
        Some("m"),
    );
    // HALF B coherence companion: `Copy`'s `Player_1.x_pos` VALUE read must hit
    // the LOCAL item (a clean comptime read of $77) — an imported-stub win would
    // be the loud [value.cross-module] error instead. Checked via the diag set.
    assert!(
        errors(&diags).is_empty(),
        "local shadow must resolve coherently (both halves local), got: {:?}",
        errors(&diags)
    );
    use sigil_ir::{Expr, Fragment};
    let (lhs, rhs) = sections
        .iter()
        .flat_map(|s| s.fragments.iter())
        .find_map(|f| match f {
            Fragment::RelaxAbsSym { target: Expr::Binary { lhs, rhs, .. }, .. } => {
                Some((lhs.clone(), rhs.clone()))
            }
            _ => None,
        })
        .expect("expected a RelaxAbsSym with an Add target for the field operand");
    assert!(
        matches!(&*lhs, Expr::Sym(n) if n.ends_with("Player_1")),
        "shadowed operand's base symbol must still be Player_1, got {lhs:?}"
    );
    assert_eq!(
        *rhs,
        Expr::Int(6),
        "shadowed operand must use the LOCAL struct's offsetof(Local, x_pos) = 6, not the imported Sst's 2"
    );
}

// ---- HALF B: comptime field VALUE reads ------------------------------------

#[test]
fn data_item_field_value_read_matches_inline() {
    // `Def.art` (a data item with a struct-lit initializer) reads the field's
    // comptime VALUE, byte-identical to writing that value inline.
    let via_field = "\
module m
struct ArtTile { art: u16, pal: u16 }
struct SeedDef { art: u16 }
data Def: ArtTile = ArtTile{ art: $1234, pal: 0 }
data Seed: SeedDef = SeedDef{ art: Def.art }
";
    let inline = "\
module m
struct SeedDef { art: u16 }
data Seed: SeedDef = SeedDef{ art: $1234 }
";
    let (mf, df) = lower(via_field);
    let (mi, di) = lower(inline);
    assert!(errors(&df).is_empty(), "field-read errors: {:?}", errors(&df));
    assert!(errors(&di).is_empty(), "inline errors: {:?}", errors(&di));
    assert_eq!(
        label_bytes(&mf, "text", "Seed", 2),
        label_bytes(&mi, "text", "Seed", 2),
        "Def.art field read must equal the inline value $1234"
    );
}

#[test]
fn data_item_enum_field_value_read() {
    // An enum-typed field read: `Def.dir` yields the enum value, byte-identical
    // to naming the variant inline.
    let via_field = "\
module m
enum Dir : u8 { Left, Right }
struct S { d: Dir }
data Def: S = S{ d: Dir.Right }
data Use: S = S{ d: Def.d }
";
    let inline = "\
module m
enum Dir : u8 { Left, Right }
struct S { d: Dir }
data Use: S = S{ d: Dir.Right }
";
    let (mf, df) = lower(via_field);
    let (mi, di) = lower(inline);
    assert!(errors(&df).is_empty(), "field-read errors: {:?}", errors(&df));
    assert!(errors(&di).is_empty(), "inline errors: {:?}", errors(&di));
    assert_eq!(label_bytes(&mf, "text", "Use", 1), label_bytes(&mi, "text", "Use", 1));
}

#[test]
fn immediate_field_read_is_value_not_address() {
    // `#Def.art` in an immediate operand is the field's VALUE ($1234), while the
    // bare form is the address — assembly convention. `move.w #Def.art, d0` →
    // 30 3C 12 34.
    let src = "\
module m
struct ArtTile { art: u16, pal: u16 }
data Def: ArtTile = ArtTile{ art: $1234, pal: 0 }
proc load() {
    move.w #Def.art, d0
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    assert_eq!(
        label_bytes(&module, "text", "load", 4),
        vec![0x30, 0x3C, 0x12, 0x34],
        "#Def.art must be the field VALUE $1234, not an address"
    );
}

#[test]
fn data_item_field_cycle_is_loud_error() {
    // `data A = S{x: B.x}` + `data B = S{x: A.x}` — a value-read cycle must be a
    // loud cyclic-definition error naming the chain, not a hang.
    let src = "\
module m
struct S { x: u16 }
data A: S = S{ x: B.x }
data B: S = S{ x: A.x }
";
    let (_module, diags) = lower(src);
    assert!(
        errors(&diags).iter().any(|e| e.contains("cyclic") && e.contains("A") && e.contains("B")),
        "expected a loud cyclic value-read error naming A and B, got: {:?}",
        errors(&diags)
    );
}

#[test]
fn value_read_wins_over_label_fallback() {
    // Precedence: a data item whose field is read via the dotted form takes the
    // VALUE-read, NOT the U3 label fallback. `Def.art` is a concrete int here, so
    // it must fit a u16 field (a label value would be a 4-byte address and would
    // size-mismatch). This pins value-read winning before the label ladder.
    let src = "\
module m
struct ArtTile { art: u16 }
data Def: ArtTile = ArtTile{ art: $0042 }
data Copy: ArtTile = ArtTile{ art: Def.art }
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "value-read must win cleanly: {:?}", errors(&diags));
    assert_eq!(label_bytes(&module, "text", "Copy", 2), vec![0x00, 0x42]);
}

#[test]
fn offsets_member_wins_over_data_value_read() {
    // Precedence guard (D-PP.5): a name that is ALSO an `offsets` table keeps its
    // ordinal meaning for `Name.Member` — the data-value read must NOT shadow the
    // enum/offsets/dispatch member steps (they run after it in eval_path). Here
    // `T.B` is the offsets ordinal 1, not a struct-field read. (Distinct names for
    // the offsets table and data item — a same-named collision is a naming error;
    // this pins the guard regardless.)
    let src = "\
module m
offsets T { A: t_a, B: t_b }
proc t_a() { rts }
proc t_b() { rts }
data Ord: [u8; 1] = [T.B]
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "offsets member must resolve: {:?}", errors(&diags));
    // `T.B` is the 0-based ordinal of member B = 1.
    assert_eq!(label_bytes(&module, "text", "Ord", 1), vec![0x01]);
}

#[test]
fn const_wins_over_data_value_read() {
    // Precedence pin (U3 ladder, folded from a review probe): a CONST and a data
    // item sharing a name — `D.f` in value position reads the CONST's field
    // ($AAAA), never the data item's initializer ($1234). Step 1's value tier is
    // local -> const -> data-value; the data-value read slots BEHIND const.
    let src = "\
module m
struct A { f: u16 }
const D: A = A{ f: $AAAA }
data D: A = A{ f: $1234 }
data Use: [u16;1] = [D.f]
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "const-shadowed read must resolve: {:?}", errors(&diags));
    assert_eq!(
        label_bytes(&module, "text", "Use", 2),
        vec![0xAA, 0xAA],
        "D.f must read the CONST's field value, not the same-named data item's"
    );
}

#[test]
fn jmp_field_operand_is_absolute_address_transfer() {
    // Behavior pin (review M2): `jmp Item.field` — a SymOff operand on jmp/jsr —
    // routes through the abs-sym seam (the `[CodeOperand::Sym]` jmp guard matches
    // only a BARE symbol), emitting a RelaxAbsSym absolute-address transfer:
    // jmp abs.w = 4E F8 + addr16 (abs.l = 4E F9 + addr32). Low address selects
    // abs.w; the target is Player_1 + offsetof(Sst, x_pos) = base + 2.
    let src = "\
module m
struct Sst { id: u16, x_pos: u16, y_pos: u16 }
data Player_1: Sst = Sst{ id: 1, x_pos: 2, y_pos: 3 }
proc go() {
    jmp Player_1.x_pos
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    let s = section(&module, "text");
    let addr = label_offset(s, "Player_1") + 0x02;
    assert_eq!(
        label_bytes(&module, "text", "go", 4),
        vec![0x4E, 0xF8, (addr >> 8) as u8, (addr & 0xFF) as u8],
        "jmp Item.field must encode jmp abs.w of Player_1 + 2 = {addr:#x}"
    );
}

// ---- `Sym ± const` absolute-address operands (tranche 11 — sprites.emp
//      demanded `btst #0, Sprite_Cycle_Counter+1`, the odd byte of a word
//      RAM cell). A bare symbol plus a comptime byte offset rides the SAME
//      `SymOff`/RelaxAbsSym seam as `Item.field`, widthed by asl on the SUM.

#[test]
fn sym_plus_const_operand_lowers_as_abs_sym_sum() {
    // `move.w Foo+2, d0` — abs.w src (30 38) of `Foo + 2`, the bare-label
    // idiom extended with a constant byte offset. Foo is a low-placed data
    // label, so the sum stays in abs.w range.
    let src = "\
module m
data Foo: [u8; 8] = [0,0,0,0,0,0,0,0]
proc read() {
    move.w Foo+2, d0
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    let s = section(&module, "text");
    let addr = label_offset(s, "Foo") + 2;
    assert_eq!(
        label_bytes(&module, "text", "read", 4),
        vec![0x30, 0x38, (addr >> 8) as u8, (addr & 0xFF) as u8],
        "Sym+const operand must encode abs.w of Foo + 2 = {addr:#x}"
    );
}

#[test]
fn sym_minus_const_operand_subtracts_the_offset() {
    // `move.w Foo-2, d0` — the `-` form subtracts: abs.w of `Foo - 2`. Foo is
    // placed past a leading pad so the difference stays non-negative + low.
    let src = "\
module m
data Pad: [u8; 8] = [0,0,0,0,0,0,0,0]
data Foo: [u8; 4] = [0,0,0,0]
proc read() {
    move.w Foo-2, d0
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    let s = section(&module, "text");
    let addr = label_offset(s, "Foo") - 2;
    assert_eq!(
        label_bytes(&module, "text", "read", 4),
        vec![0x30, 0x38, (addr >> 8) as u8, (addr & 0xFF) as u8],
        "Sym-const operand must encode abs.w of Foo - 2 = {addr:#x}"
    );
}

#[test]
fn const_plus_sym_operand_commutes() {
    // `move.w 4+Foo, d0` — address addition commutes for `+`, so the symbol
    // may sit on the right. Same abs.w encoding as `Foo+4`.
    let src = "\
module m
data Foo: [u8; 8] = [0,0,0,0,0,0,0,0]
proc read() {
    move.w 4+Foo, d0
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    let s = section(&module, "text");
    let addr = label_offset(s, "Foo") + 4;
    assert_eq!(
        label_bytes(&module, "text", "read", 4),
        vec![0x30, 0x38, (addr >> 8) as u8, (addr & 0xFF) as u8],
        "const+Sym operand must encode abs.w of 4 + Foo = {addr:#x}"
    );
}
