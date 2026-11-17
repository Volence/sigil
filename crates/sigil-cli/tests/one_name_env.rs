//! One name environment per declaration (Empyrean v2 step 0, A4).
//!
//! A comptime item imported into another module (a struct, a comptime fn, and the
//! rest of the injectable kinds) resolves its free names where it is DEFINED, never
//! in the importer's scope. The first four fixtures are reviewer A's round-2 probes
//! (`audit-builds/reviewer-a-r2/probes/{p8,p8b,p2d,p2e}`), verbatim; the next three
//! are the plan's Review Focus 5 shapes (a field type from a third module, a private
//! helper chain, and an importer's own same-named items still meaning themselves);
//! the last is the refusal for a name only the importer binds.
//!
//! Each builds with `sigil emp main.emp --root <tmp>` and compares the whole image.
use std::path::Path;
use std::process::Command;

/// Build `main.emp` under a fresh root holding `files`: `(success, stderr, bytes)`.
fn run(files: &[(&str, &str)]) -> (bool, String, Option<Vec<u8>>) {
    let dir = tempfile::tempdir().unwrap();
    for (rel, src) in files {
        std::fs::write(dir.path().join(rel), src).unwrap();
    }
    let root: &Path = dir.path();
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("main.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let bytes = if out.status.success() { std::fs::read(&outbin).ok() } else { None };
    (out.status.success(), stderr, bytes)
}

/// The image of a build that must succeed.
fn build(files: &[(&str, &str)]) -> Vec<u8> {
    let (ok, stderr, bytes) = run(files);
    assert!(ok, "build failed:\n{stderr}");
    bytes.expect("a successful build writes its image")
}

// ---- reviewer A's probes, verbatim ------------------------------------------

const P8_MAIN: &str = "module main
use a.{S}
section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0) {
        moveq   #sizeof(S), d0
        rts
  }
}
";
const P8_A: &str = "module a
pub const N = 4
pub struct S {
    x: [u8; N],
    y: u16,
}
";
const P8: &[(&str, &str)] = &[("main.emp", P8_MAIN), ("a.emp", P8_A)];

const P8B_MAIN: &str = "module main
use a.{S}
const N = 10
section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0) {
        moveq   #sizeof(S), d0
        rts
  }
}
";
const P8B: &[(&str, &str)] = &[("main.emp", P8B_MAIN), ("a.emp", P8_A)];

const P2D_MAIN: &str = "module main
use helper.{count_to_limit, lim}

const LIMIT = 9

section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0-d1) {
        moveq   #count_to_limit(), d0
        moveq   #lim(), d1
        rts
  }
}
";
const P2D_HELPER: &str = "module helper
use k.{LIMIT}
pub comptime fn count_to_limit() -> int {
    comptime var n = 0
    for i in 0..LIMIT {
        n = n + 1
    }
    return n
}
pub comptime fn lim() -> int {
    return LIMIT
}
";
const P2D_K: &str = "module k
pub const LIMIT = 5
";
const P2D: &[(&str, &str)] = &[("main.emp", P2D_MAIN), ("helper.emp", P2D_HELPER), ("k.emp", P2D_K)];

const P2E_MAIN: &str = "module main
use helper.{four}
comptime fn twice(x: int) -> int { return x * 3 }
section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0) {
        moveq   #four(), d0
        rts
  }
}
";
const P2E_HELPER: &str = "module helper
comptime fn twice(x: int) -> int { return x * 2 }
pub comptime fn four() -> int { return twice(2) }
";
const P2E: &[(&str, &str)] = &[("main.emp", P2E_MAIN), ("helper.emp", P2E_HELPER)];

#[test]
fn struct_layout_is_computed_in_its_defining_module() {
    assert_eq!(build(P8B), [0x70, 0x06, 0x4E, 0x75]); // today 70 0C
}

#[test]
fn struct_layout_needs_no_caller_binding() {
    assert_eq!(build(P8), [0x70, 0x06, 0x4E, 0x75]); // today an error at a.emp
}

#[test]
fn comptime_fn_free_const_resolves_where_defined() {
    assert_eq!(build(P2D), [0x70, 0x05, 0x72, 0x05, 0x4E, 0x75]); // today 09 09
}

#[test]
fn comptime_fn_private_helper_resolves_where_defined() {
    assert_eq!(build(P2E), [0x70, 0x04, 0x4E, 0x75]); // today 06
}

// ---- Review Focus 5 ---------------------------------------------------------

/// `a.S { t: b.T }`, `b.T`'s length is `b`'s private `N = 2`; `main` has its own
/// `N = 50` AND its own `T` using it. `sizeof(S)` is `b`'s answer: 2 + 2 = 4. Under
/// the importer's scope it was `main.T` (50) + 2 = 52.
#[test]
fn struct_field_type_from_a_third_module_uses_that_modules_names() {
    let main = "module main
use a.{S}
const N = 50
struct T { x: [u8; N] }
section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0) {
        moveq   #sizeof(S), d0
        rts
  }
}
";
    let a = "module a
use b.{T}
pub struct S {
    t: T,
    z: u16,
}
";
    let b = "module b
const N = 2
pub struct T { x: [u8; N] }
";
    assert_eq!(build(&[("main.emp", main), ("a.emp", a), ("b.emp", b)]), [0x70, 0x04, 0x4E, 0x75]);
}

/// `pub f` → private `g` → private `h`, and `main` defines its own `g` and `h`.
/// `f()` is `h() + 1 = 11` from `helper`; the importer's would be 100.
#[test]
fn private_helper_chain_resolves_where_defined() {
    let main = "module main
use helper.{f}
comptime fn g() -> int { return 100 }
comptime fn h() -> int { return 50 }
section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0) {
        moveq   #f(), d0
        rts
  }
}
";
    let helper = "module helper
comptime fn h() -> int { return 10 }
comptime fn g() -> int { return h() + 1 }
pub comptime fn f() -> int { return g() }
";
    assert_eq!(build(&[("main.emp", main), ("helper.emp", helper)]), [0x70, 0x0B, 0x4E, 0x75]);
}

/// The importer's own `N = 10` still means itself in its own code, while the
/// imported `S` is sized by `a.N = 4` beside it.
#[test]
fn an_importers_own_names_still_mean_themselves() {
    let main = "module main
use a.{S}
const N = 10
section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0-d1) {
        moveq   #N, d0
        moveq   #sizeof(S), d1
        rts
  }
}
";
    assert_eq!(build(&[("main.emp", main), ("a.emp", P8_A)]), [0x70, 0x0A, 0x72, 0x06, 0x4E, 0x75]);
}

/// A helper reads `LIMIT` it never imports, and the caller has one: refused at the
/// definition, naming the name, the item and the module to fix.
#[test]
fn a_name_unbound_at_definition_is_refused() {
    let main = "module main
use helper.{lim}
const LIMIT = 9
section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0) {
        moveq   #lim(), d0
        rts
  }
}
";
    let helper = "module helper
pub comptime fn lim() -> int {
    return LIMIT
}
";
    let (ok, stderr, _) = run(&[("main.emp", main), ("helper.emp", helper)]);
    assert!(!ok, "a name bound only by the importer must be refused; stderr:\n{stderr}");
    assert!(
        stderr.contains(
            "[name.unbound-at-definition] `LIMIT` in `lim` is not bound in module `helper`; add a `use` there"
        ),
        "stderr:\n{stderr}"
    );
}

// ---- fix round 1 (task-8 review I1-I4, M2) -----------------------------------

const PROC_MOVEQ_F: &str = "section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0) {
        moveq   #f(), d0
        rts
  }
}
";

/// I1: an `equ` is read as a comptime value, so a private equ of the defining
/// module binds there; the importer's own `E = 9` does not capture it.
#[test]
fn a_private_equ_resolves_where_defined() {
    let main = format!("module main\nuse helper.{{f}}\nconst E = 9\n{PROC_MOVEQ_F}");
    let helper = "module helper
equ E = 3
pub comptime fn f() -> int { return E }
";
    assert_eq!(build(&[("main.emp", &main), ("helper.emp", helper)]), [0x70, 0x03, 0x4E, 0x75]);
}

/// I1: the same for a struct field length spelled with a private equ.
#[test]
fn a_private_equ_sizes_a_struct_where_defined() {
    let main = "module main
use a.{S}
const N = 9
section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0) {
        moveq   #sizeof(S), d0
        rts
  }
}
";
    let a = "module a
equ N = 3
pub struct S { x: [u8; N] }
";
    assert_eq!(build(&[("main.emp", main), ("a.emp", a)]), [0x70, 0x03, 0x4E, 0x75]);
}

/// I1: the refusal sees an importer binding spelled as an `equ` too.
#[test]
fn a_name_the_importer_binds_only_by_equ_is_refused() {
    let main = "module main
use helper.{lim}
equ LIMIT = 9
section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0) {
        moveq   #lim(), d0
        rts
  }
}
";
    let helper = "module helper
pub comptime fn lim() -> int {
    return LIMIT
}
";
    let (ok, stderr, _) = run(&[("main.emp", main), ("helper.emp", helper)]);
    assert!(!ok, "an equ bound only by the importer must be refused; stderr:\n{stderr}");
    assert!(
        stderr.contains(
            "[name.unbound-at-definition] `LIMIT` in `lim` is not bound in module `helper`; add a `use` there"
        ),
        "stderr:\n{stderr}"
    );
}

/// I2: a discriminant is evaluated in the module environment, where a variant is
/// only `K.A`, so `A = A` names `a.A = 7` and `K.B` is 8, not the importer's 20 + 1.
#[test]
fn an_enum_discriminant_names_the_module_const_not_a_variant() {
    let a = "module a
const A = 7
pub enum K: u8 { A = A, B }
";
    let main = "module main
use a.{K}
const A = 20
section s (cpu: m68000, vma: $0) {
  data X: K = K.B
}
";
    assert_eq!(build(&[("main.emp", main), ("a.emp", a)]), [0x08]);
}

const OVL_S: &str = "module s
pub struct Sst { id: u16, sst_custom: [u8; 8] }
";
const OVL_TICK: &str = "proc tick () {
    tst.b V.timer(a1)
    rts
}
";

/// I3: a dotted overlay window `Sst.sst_custom` names `s.Sst` (window at `$2`) where
/// the overlay is defined, not the importer's own `Sst` (window at `$A`).
#[test]
fn a_dotted_overlay_window_resolves_where_defined() {
    let lib = "module lib\nuse s.{Sst}\npub vars V: Sst.sst_custom { timer: u8 }\n";
    let main = format!(
        "module main\nuse lib.{{V}}\nstruct Sst {{ hp: u32, filler: [u8; 6], sst_custom: [u8; 4] }}\n{OVL_TICK}"
    );
    let img = build(&[("main.emp", &main), ("lib.emp", lib), ("s.emp", OVL_S)]);
    assert_eq!(img, [0x4A, 0x29, 0x00, 0x02, 0x4E, 0x75]);
    // The importer needs no `Sst` of its own (the retired EMP_PITFALLS §8 rule).
    let main = format!("module main\nuse lib.{{V}}\n{OVL_TICK}");
    let img = build(&[("main.emp", &main), ("lib.emp", lib), ("s.emp", OVL_S)]);
    assert_eq!(img, [0x4A, 0x29, 0x00, 0x02, 0x4E, 0x75]);
}

/// I3: a BARE window over an imported struct binds in the defining module's
/// environment too: not the importer's unrelated `Other.sst_custom` (`$A`), and
/// with no struct in the importer at all.
#[test]
fn a_bare_overlay_window_resolves_where_defined() {
    let lib = "module lib\nuse s.{Sst}\npub vars V: sst_custom { timer: u8 }\n";
    let main = format!(
        "module main\nuse lib.{{V}}\nstruct Other {{ hp: u32, filler: [u8; 6], sst_custom: [u8; 4] }}\n{OVL_TICK}"
    );
    let img = build(&[("main.emp", &main), ("lib.emp", lib), ("s.emp", OVL_S)]);
    assert_eq!(img, [0x4A, 0x29, 0x00, 0x02, 0x4E, 0x75]);
    let main = format!("module main\nuse lib.{{V}}\n{OVL_TICK}");
    let img = build(&[("main.emp", &main), ("lib.emp", lib), ("s.emp", OVL_S)]);
    assert_eq!(img, [0x4A, 0x29, 0x00, 0x02, 0x4E, 0x75]);
}

const WIN_A: &str = "module a
struct T { id: u32, w: [u8; 4] }
pub comptime fn f() -> int { return sizeof(T) }
";

/// I4: an injected hidden struct (`$a$T`, a dependency of `f`) is not a bare-window
/// candidate. With the importer's own `M` the window binds `M.w` (`$2`)...
#[test]
fn a_hidden_struct_is_not_a_bare_window_candidate() {
    let main = "module main
use a.{f}
struct M { q: u16, w: [u8; 4] }
vars V: w { t: u8 }
proc tick () {
    moveq #f(), d0
    tst.b V.t(a1)
    rts
}
";
    assert_eq!(build(&[("main.emp", main), ("a.emp", WIN_A)]), [0x70, 0x08, 0x4A, 0x29, 0x00, 0x02, 0x4E, 0x75]);
}

/// I4: ...and with no struct of its own, the window is unknown, loudly, rather than
/// binding silently to `$a$T.w`, a struct the importer never named.
#[test]
fn a_hidden_struct_never_binds_a_bare_window_silently() {
    let main = "module main
use a.{f}
vars V: w { t: u8 }
proc tick () {
    moveq #f(), d0
    tst.b V.t(a1)
    rts
}
";
    let (ok, stderr, _) = run(&[("main.emp", main), ("a.emp", WIN_A)]);
    assert!(!ok, "a window only a hidden struct has must not bind; stderr:\n{stderr}");
    assert!(
        stderr.contains("[overlay.unknown-window] no in-scope struct has a `[u8; N]` field named `w`"),
        "stderr:\n{stderr}"
    );
}

const MAP_A: &str = "module a
pub newtype Px = u16
pub comptime fn add1(p: Px) -> Px { return p + Px(1) }
";

/// M2: an importer that imports `Px` itself gets `add1`'s hidden `$a$Px` mapped
/// back to its own `Px` (the same item), so the two values mix: 3 + 1 + 2 = 6.
#[test]
fn a_hidden_dependency_the_importer_also_imports_maps_back() {
    let main = "module main
use a.{Px, add1}
section s (cpu: m68000, vma: $0) {
  data X: [u8; 1] = [add1(Px(3)) + Px(2)]
}
";
    assert_eq!(build(&[("main.emp", main), ("a.emp", MAP_A)]), [0x06]);
}

/// M2, the negative: a DIFFERENT item under the same plain name is never mapped
/// back (`f` sizes `a.S`, 3; the importer's `S` is `b.S`, 8).
#[test]
fn a_different_item_of_the_same_name_is_not_mapped_back() {
    let a = "module a
pub struct S { x: [u8; 3] }
pub comptime fn f() -> int { return sizeof(S) }
";
    let b = "module b\npub struct S { x: [u8; 8] }\n";
    let main = "module main
use a.{f}
use b.{S}
section s (cpu: m68000, vma: $0) {
  proc P () clobbers(d0-d1) {
        moveq   #f(), d0
        moveq   #sizeof(S), d1
        rts
  }
}
";
    assert_eq!(
        build(&[("main.emp", main), ("a.emp", a), ("b.emp", b)]),
        [0x70, 0x03, 0x72, 0x08, 0x4E, 0x75]
    );
}
