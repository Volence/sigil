//! `sigil <input.asm> -D ...`: asl's command-line define, which makes each name a
//! SET variable bound before every pass.
//!
//! What each test catches:
//!
//! - `values_and_spellings`: the grammar. A bare name defaulting to anything
//!   but 1, a comma list read as one name, a repeated name taking its LAST
//!   value, or a value read as a literal instead of an expression fails here.
//! - `set_rebinds_and_constants_are_refused`: the class. A define seeded with
//!   no class (so an in-file `=`/`equ`/label silently wins, which is what
//!   `Options::defines` does) exits 0 here where asl refuses with #2035.
//! - `rebound_at_every_pass`: a define carried from the previous pass's end
//!   instead of re-bound at each pass start reads `07 07` where asl reads `05 07`.
//! - `defined_and_ifdef_see_it`: a define that reaches the environment but not
//!   the "defined this pass" record answers `defined(FOO)` with 0.
//! - `malformed_arguments_are_usage_errors`: a refusal asl gives at its command
//!   line (`Invalid option: -D`, exit 4) is exit 2 here, before any assembly.
//! - `no_define_no_change`: a program with no `-D` still sees FOO undefined.
//!
//! Expected values come from the reference toolchain: `asl` md5
//! 61e672562465725a8c102288a7da9098 through `asl_ref.sh`'s `asl_run`, flags
//! `-xx -n -q -A -L -U -i .` plus the `-D` under test, exit 0, and `p2bin -p=0`.
//! The table, with every row run through both assemblers, is in
//! `docs/superpowers/notes/2026-09-25-as-cli-define.md`.

use std::path::Path;
use std::process::{Command, Output};

fn run(dir: &Path, extra: &[&str]) -> (Output, std::path::PathBuf) {
    let out_path = dir.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(dir.join("root.asm"))
        .arg("-o")
        .arg(&out_path)
        .args(extra)
        .output()
        .expect("spawn sigil");
    (out, out_path)
}

/// Assemble `src` with `args` and return the image, or panic with what sigil said.
fn image(src: &str, args: &[&str]) -> Vec<u8> {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("root.asm"), src).expect("write root.asm");
    let (out, path) = run(dir.path(), args);
    assert!(
        out.status.success(),
        "{args:?} must assemble.\nstderr:\n{}\nsource:\n{src}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read(path).expect("the image was written")
}

/// Assemble `src` with `args`, require a refusal with no image, and return the
/// exit code and stderr.
fn refused(src: &str, args: &[&str]) -> (i32, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("root.asm"), src).expect("write root.asm");
    let (out, path) = run(dir.path(), args);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!out.status.success(), "{args:?} must be refused.\nstderr:\n{stderr}\nsource:\n{src}");
    assert!(!path.exists(), "a refused run must write no image.\nstderr:\n{stderr}");
    (out.status.code().expect("exit code"), stderr)
}

const USE_FOO: &str = "\tcpu 68000\n\tdc.b\tFOO\n";
const USE_FOO_BAR: &str = "\tcpu 68000\n\tdc.b\tFOO,BAR\n";

#[test]
fn values_and_spellings() {
    let rows: &[(&str, &[&str], &[u8])] = &[
        (USE_FOO, &["-D", "FOO=5"], &[0x05]),
        (USE_FOO, &["-D", "FOO"], &[0x01]),
        (USE_FOO, &["-D", "FOO="], &[0x01]),
        (USE_FOO_BAR, &["-D", "FOO=1,BAR=2"], &[0x01, 0x02]),
        (USE_FOO_BAR, &["-D", "FOO,BAR=2"], &[0x01, 0x02]),
        (USE_FOO_BAR, &["-D", "FOO=1", "-D", "BAR=2"], &[0x01, 0x02]),
        (USE_FOO, &["-D", "FOO=5,"], &[0x05]),
        (USE_FOO, &["-D", "FOO=1", "-D", "FOO=2"], &[0x01]),
        (USE_FOO, &["-D", "FOO=1,FOO=2"], &[0x01]),
        (USE_FOO, &["-D", "FOO", "-D", "FOO=2"], &[0x01]),
        (USE_FOO, &["-D", "FOO=5", "-D", "FOO"], &[0x05]),
        (USE_FOO, &["-D", "FOO=$10"], &[0x10]),
        (USE_FOO, &["-D", "FOO=$ff"], &[0xFF]),
        (USE_FOO, &["-D", "FOO=0x10"], &[0x10]),
        (USE_FOO, &["-D", "FOO=10h"], &[0x10]),
        (USE_FOO, &["-D", "FOO=%101"], &[0x05]),
        (USE_FOO, &["-D", "FOO=-1"], &[0xFF]),
        (USE_FOO, &["-D", "FOO=~1"], &[0xFE]),
        (USE_FOO, &["-D", "FOO=1 + 2"], &[0x03]),
        (USE_FOO, &["-D", "FOO=1<<4"], &[0x10]),
        (USE_FOO, &["-D", "FOO= 5"], &[0x05]),
        ("\tcpu 68000\n\tdc.l\tFOO\n", &["-D", "FOO=$FFFFFFFF"], &[0xFF, 0xFF, 0xFF, 0xFF]),
        ("\tcpu 68000\n\tdc.b\tFOO.BAR\n", &["-D", "FOO.BAR=1"], &[0x01]),
        ("\tcpu 68000\nfoo equ 3\n\tdc.b\tfoo,FOO\n", &["-D", "FOO=5"], &[0x03, 0x05]),
        ("\tcpu 68000\n\tif FOO\n\tdc.b\t1\n\telse\n\tdc.b\t2\n\tendif\n", &["-D", "FOO=0"], &[0x02]),
    ];
    for (src, args, want) in rows {
        assert_eq!(&image(src, args), want, "{args:?} on\n{src}");
    }
    // Case sensitive: `-D foo` does not define `FOO`.
    refused(USE_FOO, &["-D", "foo=5"]);
}

#[test]
fn set_rebinds_and_constants_are_refused() {
    assert_eq!(image("\tcpu 68000\nFOO set 7\n\tdc.b\tFOO\n", &["-D", "FOO=5"]), [0x07]);
    assert_eq!(image("\tcpu 68000\nFOO := 7\n\tdc.b\tFOO\n", &["-D", "FOO=5"]), [0x07]);
    assert_eq!(image("\tcpu 68000\nFOO eval 7\n\tdc.b\tFOO\n", &["-D", "FOO=5"]), [0x07]);
    assert_eq!(
        image("\tcpu 68000\n\tdc.b\tFOO\nFOO set 7\n\tdc.b\tFOO\n", &["-D", "FOO=5"]),
        [0x05, 0x07]
    );
    let guarded = "\tcpu 68000\n\tifndef FOO\nFOO set 3\n\tendif\n\tdc.b\tFOO\n";
    assert_eq!(image(guarded, &["-D", "FOO=5"]), [0x05]);
    assert_eq!(image(guarded, &[]), [0x03]);
    for src in [
        "\tcpu 68000\nFOO = 7\n\tdc.b\tFOO\n",
        "\tcpu 68000\nFOO equ 7\n\tdc.b\tFOO\n",
        "\tcpu 68000\nFOO:\n\tdc.b\t1\n",
    ] {
        let (code, stderr) = refused(src, &["-D", "FOO=5"]);
        assert_eq!(code, 1, "{stderr}");
        assert!(
            stderr.contains("root.asm(2):1: error: variables cannot be redefined as constants: `FOO`"),
            "asl's #2035 at line 2:\n{stderr}"
        );
    }
}

#[test]
fn rebound_at_every_pass() {
    // The forward branch forces a second pass (asl: `2 passes`).
    let src = "\tcpu 68000\n\tdc.b\tFOO\nFOO set 7\n\tdc.b\tFOO\n\tbra.w\tfwd\nfwd:\n";
    assert_eq!(image(src, &["-D", "FOO=5"]), [0x05, 0x07, 0x60, 0x00, 0x00, 0x02]);
}

#[test]
fn defined_and_ifdef_see_it() {
    let defd = "\tcpu 68000\n\tdc.b\tdefined(FOO)\n";
    assert_eq!(image(defd, &["-D", "FOO=0"]), [0x01]);
    assert_eq!(image(defd, &[]), [0x00]);
    let ifdef = "\tcpu 68000\n\tifdef FOO\n\tdc.b\t1\n\telse\n\tdc.b\t2\n\tendif\n";
    assert_eq!(image(ifdef, &["-D", "FOO=0"]), [0x01]);
    let ifndef = "\tcpu 68000\n\tifndef FOO\n\tdc.b\t1\n\telse\n\tdc.b\t2\n\tendif\n";
    assert_eq!(image(ifndef, &["-D", "FOO=5"]), [0x02]);
}

#[test]
fn malformed_arguments_are_usage_errors() {
    for arg in [
        "", "1FOO=1", "=1", "FO-O=1", ",FOO=5", "FOO==5", "FOO=5x", "FOO=1+", "FOO=ff",
        "FOO=1, BAR=2", "FOO =5", "FOO=1,,BAR=2", "FOO=BAR", "FOO=1.5", "FOO=\"A\"", "FOO='A'",
        ".FOO=1",
    ] {
        let (code, stderr) = refused(USE_FOO, &["-D", arg]);
        assert_eq!(code, 2, "`-D {arg}`:\n{stderr}");
        assert!(stderr.starts_with("error: -D"), "`-D {arg}`:\n{stderr}");
    }
    let (code, stderr) = refused(USE_FOO, &["-D"]);
    assert_eq!(code, 2, "{stderr}");
}

#[test]
fn no_define_no_change() {
    let (code, stderr) = refused(USE_FOO, &[]);
    assert_eq!(code, 1, "{stderr}");
}
