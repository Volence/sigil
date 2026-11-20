//! G0a Task 8: the `.emp` front end's layout and constant registry, over a small
//! program that has every shape the export names (struct, bitfield, plain enum,
//! overlay; integer, bool, enum, newtype, and the kinds that are skipped).

use sigil_frontend_emp::export_side::{emp_consts, emp_layouts, EmpExportError, EmpLayoutKind, LayoutPass};
use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{build_program_recorded_with_layouts, manifest::Manifest, BuiltProgram};
use sigil_ir::backend::Cpu;
use sigil_span::Level;

fn build(files: &[(&str, &str)]) -> BuiltProgram {
    build_with(files, LayoutPass::Owner)
}

fn build_with(files: &[(&str, &str)], pass: LayoutPass) -> BuiltProgram {
    let dir = tempfile::tempdir().unwrap();
    for (rel, src) in files {
        let p = dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, src).unwrap();
    }
    let (manifest, _) = Manifest::scan(dir.path());
    let opts = LowerOptions { roots: None, initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let built = build_program_recorded_with_layouts(&manifest, "app.main", None, &opts, pass);
    let errs: Vec<_> = built.diags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect();
    assert!(errs.is_empty(), "{errs:?}");
    built
}

const TYPES: &str = "module app.types\n\
    pub newtype Coord = u16\n\
    pub newtype Gain = fixed<8, 8>\n\
    pub enum Anim: u8 { Idle = 0, Run, Jump = 7 }\n\
    pub comptime enum Op { Nop, Lit(int) }\n\
    pub bitfield Flags: u16 { tile: 11 @ 0, pal: 2 @ 11, prio: 1 @ 13 }\n\
    pub struct Rec (size: 14) {\n\
        x: Coord,\n\
        gain: Gain,\n\
        anim: Anim,\n\
        pad: u8,\n\
        next: *Rec,\n\
        flags: Flags,\n\
        win: [u8; 2],\n\
    }\n\
    pub vars RecV: Rec.win { lo: u8, hi: u8 }\n\
    pub const COUNT = 5\n\
    pub const SMALL: u8 = 3\n\
    pub const ON = true\n\
    pub const START = Anim.Run\n\
    pub const SPEED = Coord(40)\n\
    pub const NAME = \"hello\"\n\
    pub const LIST = [1, 2, 3]\n\
    pub const BLOB = bytes([1, 2])\n\
    pub const OPCODE = Op.Nop\n\
    const HIDDEN = 9\n";
const MAIN: &str = "module app.main\n\
    use app.types.{Rec, Anim, Flags, Coord, Gain, COUNT}\n\
    pub proc main () {\n    rts\n}\n";

#[test]
fn every_shape_is_exported_with_resolved_types() {
    let built = build(&[("app/types.emp", TYPES), ("app/main.emp", MAIN)]);
    let layouts = emp_layouts(&built.layouts).unwrap();
    let find = |n: &str| layouts.iter().find(|l| l.name == n).unwrap_or_else(|| panic!("no layout {n}: {:?}", layouts.iter().map(|l| &l.name).collect::<Vec<_>>()));

    let rec = find("Rec");
    assert_eq!((rec.kind, rec.size, rec.module.as_str()), (EmpLayoutKind::Struct, 14, "app.types"));
    let f = |n: &str| rec.fields.iter().find(|f| f.name == n).unwrap();
    assert_eq!((f("x").offset, f("x").size, f("x").ty.as_str()), (0, 2, "Coord"));
    assert_eq!((f("gain").offset, f("gain").ty.as_str()), (2, "Gain"));
    assert_eq!((f("anim").offset, f("anim").size, f("anim").ty.as_str()), (4, 1, "Anim"));
    assert_eq!((f("next").offset, f("next").size, f("next").ty.as_str()), (6, 4, "ptr<Rec>"));
    assert_eq!(f("flags").ty, "Flags");
    assert_eq!(f("win").ty, "[u8; 2]");
    assert_eq!(rec.overlays.len(), 1);
    let ov = &rec.overlays[0];
    assert_eq!((ov.name.as_str(), ov.base.as_str(), ov.offset, ov.size), ("RecV", "Rec", 12, 2));
    assert_eq!(ov.fields.iter().map(|f| (f.name.as_str(), f.offset, f.size, f.ty.as_str())).collect::<Vec<_>>(), [("lo", 0, 1, "u8"), ("hi", 1, 1, "u8")]);

    let flags = find("Flags");
    assert_eq!((flags.kind, flags.size), (EmpLayoutKind::Bitfield, 2));
    assert_eq!(flags.fields.iter().map(|f| (f.name.as_str(), f.offset, f.size)).collect::<Vec<_>>(), [("tile", 0, 11), ("pal", 11, 2), ("prio", 13, 1)]);

    let anim = find("Anim");
    assert_eq!((anim.kind, anim.size), (EmpLayoutKind::Enum, 1));
    assert_eq!(anim.fields.iter().map(|f| (f.name.as_str(), f.offset)).collect::<Vec<_>>(), [("Idle", 0), ("Run", 1), ("Jump", 7)]);

    // A comptime enum has no data layout; a private or non-layout item is not a layout.
    assert!(layouts.iter().all(|l| l.name != "Op"));
    // Each layout once, though three modules' views saw `Rec`.
    assert_eq!(layouts.iter().filter(|l| l.name == "Rec").count(), 1);
}

#[test]
fn constants_are_integer_bool_and_enum_and_the_rest_are_skipped_by_kind() {
    let built = build(&[("app/types.emp", TYPES), ("app/main.emp", MAIN)]);
    let consts = emp_consts(&built.layouts).unwrap();
    let got: Vec<(String, String, i64)> = consts.iter().map(|c| (c.name.clone(), c.ty.clone(), c.value)).collect();
    let want = |n: &str, t: &str, v: i64| assert!(got.contains(&(n.into(), t.into(), v)), "missing {n}: {got:?}");
    want("COUNT", "int", 5);
    want("SMALL", "u8", 3);
    want("ON", "bool", 1);
    want("START", "Anim", 1);
    want("SPEED", "Coord", 40);
    for skipped in ["NAME", "LIST", "BLOB", "OPCODE", "HIDDEN"] {
        assert!(got.iter().all(|(n, _, _)| n != skipped), "{skipped} was exported: {got:?}");
    }
    assert!(consts.iter().all(|c| c.module == "app.types"));
    let counts = built.layouts.const_shape_counts();
    assert_eq!(counts.get("int").map(|c| c.0), Some(1), "{counts:?}");
    assert_eq!(counts.get("typed-or-enum").map(|c| c.0), Some(3), "{counts:?}");
    assert_eq!(counts.get("comptime-enum").map(|c| c.1), Some(1), "{counts:?}");
    assert_eq!(counts.get("bool").map(|c| c.0), Some(1), "{counts:?}");
    assert_eq!(counts.get("string").map(|c| c.1), Some(1), "{counts:?}");
    assert_eq!(counts.get("array").map(|c| c.1), Some(1), "{counts:?}");
    assert_eq!(counts.get("data").map(|c| c.1), Some(1), "{counts:?}");
}

#[test]
fn a_constant_past_i64_is_an_error_naming_it() {
    let built = build(&[("app/big.emp", "module app.big\npub const HUGE = 1 << 63\npub const OK = 1 << 62\n"), ("app/main.emp", "module app.main\nuse app.big.{OK}\npub proc main () {\n    rts\n}\n")]);
    match emp_consts(&built.layouts) {
        Err(EmpExportError::ConstOutOfRange { module, name, value }) => {
            assert_eq!((module.as_str(), name.as_str(), value), ("app.big", "HUGE", 1i128 << 63));
        }
        other => panic!("expected ConstOutOfRange, got {other:?}"),
    }
}

#[test]
fn a_build_with_the_pass_off_computes_nothing() {
    let built = build_with(&[("app/types.emp", TYPES), ("app/main.emp", MAIN)], LayoutPass::Off);
    assert_eq!(built.layouts.layout_counts(), (0, 0, 0, 0));
    assert!(built.layouts.const_shape_counts().is_empty());
    assert_eq!(emp_layouts(&built.layouts).unwrap().len(), 0);
    assert_eq!(emp_consts(&built.layouts).unwrap().len(), 0);
    // And the same program with the pass on is not empty, so the assertion above bites.
    let on = build_with(&[("app/types.emp", TYPES), ("app/main.emp", MAIN)], LayoutPass::Owner);
    assert_ne!(on.layouts.layout_counts(), (0, 0, 0, 0));
}

#[test]
fn two_modules_with_a_same_named_struct_keep_their_overlays_apart() {
    let a = "module app.a\npub struct Foo (size: 6) { x: u8, pad: u8, w: [u8; 4] }\n\
             pub vars AV: Foo.w { p: u16, q: u16 }\npub const A = 1\n";
    let b = "module app.b\npub struct Foo (size: 2) { y: u16 }\npub const B = 2\n";
    let m = "module app.main\nuse app.a.{A}\nuse app.b.{B}\npub proc main () {\n    rts\n}\n";
    let built = build(&[("app/a.emp", a), ("app/b.emp", b), ("app/main.emp", m)]);
    let layouts = emp_layouts(&built.layouts).expect("legal code is not refused");
    let foo = |module: &str| layouts.iter().find(|l| l.name == "Foo" && l.module == module).unwrap();
    assert_eq!(foo("app.a").overlays.len(), 1, "{layouts:?}");
    assert_eq!(foo("app.a").overlays[0].name, "AV");
    assert_eq!(foo("app.a").overlays[0].base_module, "app.a");
    assert!(foo("app.b").overlays.is_empty(), "the overlay landed on the other module's Foo");
}

#[test]
fn the_default_pass_reads_only_the_owners_view_and_the_full_pass_cross_checks() {
    let files = [("app/types.emp", TYPES), ("app/main.emp", MAIN)];
    let owner = build_with(&files, LayoutPass::Owner);
    let full = build_with(&files, LayoutPass::Full);
    assert_eq!(emp_layouts(&owner.layouts), emp_layouts(&full.layouts));
    assert_eq!(emp_consts(&owner.layouts), emp_consts(&full.layouts));
    // The control: with a planted disagreement between views, Owner cannot see it (it
    // reads one view) and Full refuses it by key.
    let skew = build_with(&files, LayoutPass::FullSkewedForTest);
    assert!(matches!(emp_layouts(&skew.layouts), Err(EmpExportError::TwoLayouts { .. })), "{:?}", emp_layouts(&skew.layouts));
    assert!(matches!(emp_consts(&skew.layouts), Err(EmpExportError::TwoConsts { .. })), "{:?}", emp_consts(&skew.layouts));
}

#[test]
fn an_owner_constant_that_does_not_evaluate_is_a_fault_not_a_gap() {
    let bad = "module app.bad\npub const OK = 1\npub const BROKEN = NOT_DEFINED_ANYWHERE + 1\n";
    let m = "module app.main\nuse app.bad.{OK}\npub proc main () {\n    rts\n}\n";
    let dir = tempfile::tempdir().unwrap();
    for (rel, src) in [("app/bad.emp", bad), ("app/main.emp", m)] {
        let p = dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, src).unwrap();
    }
    let (manifest, _) = Manifest::scan(dir.path());
    let opts = LowerOptions { roots: None, initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let built = build_program_recorded_with_layouts(&manifest, "app.main", None, &opts, LayoutPass::Owner);
    match emp_consts(&built.layouts) {
        Err(EmpExportError::Const { module, name, .. }) => assert_eq!((module.as_str(), name.as_str()), ("app.bad", "BROKEN")),
        other => panic!("expected a Const fault naming BROKEN, got {other:?}"),
    }
}
