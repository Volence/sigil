//! The installed `sigil` must build every shipped shape with the checkout it was COMPILED
//! in gone.
//!
//! The binary aeon builds every ROM with is compiled in a sigil worktree and then used
//! from elsewhere. On 2026-09-26 that worktree was removed and every aeon build of every
//! shape failed, because the assembler read its off-canonical size tables at run time
//! from a path fixed at compile time. The tables are compiled in now
//! (`sigil_harness::native::FROZEN_TABLES`), and this gate holds the property rather
//! than the mechanism: it runs the real binary with the whole compiling checkout HIDDEN
//! (`bwrap`, an empty tmpfs mounted over it) and requires all four shapes to build, byte
//! for byte the ROMs the same binary writes with the checkout visible.
//!
//! Its controls, each asserted:
//!
//!  * the hiding is real: a committed file of the checkout is readable outside the
//!    sandbox and absent inside it, checked with the same `bwrap` arguments the builds use;
//!  * the builds measured something: each sandboxed ROM is compared with a ROM from an
//!    unsandboxed build of the same shape, and both must exist and be non-empty;
//!  * the digest agrees: no `DIGEST-READ` row of a sandboxed build is rooted at the sigil
//!    checkout.
//!
//! Unmeasurable is loud: with no `bwrap`, or with the reference tree or the binary itself
//! inside the checkout (hiding the checkout would hide the input under test), the test
//! fails naming why. With no reference tree it follows the house pattern (skip, or a
//! failure under `SIGIL_STRICT_GATE=1`).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon \
//!   cargo test --release -p sigil-cli --test installed_binary_needs_no_build_tree
//! ```

use sigil_link::{parse_source_digest, DigestRoot};
use std::path::{Path, PathBuf};
use std::process::Command;

const SHAPES: &[(&str, &[&str])] = &[
    ("s4", &[]),
    ("s4.debug", &["--debug"]),
    ("demo", &["--game", "demo"]),
    ("demo.debug", &["--game", "demo", "--debug"]),
];

/// A file every checkout of this repository carries, used as the hiding control.
const CONTROL_FILE: &str = "crates/sigil-harness/golden/provenance.toml";

fn fresh_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("installed_binary_needs_no_build_tree_{case}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir.canonicalize().expect("canonical scratch dir")
}

/// A `bwrap` command that shows the whole filesystem except `hidden`, which is an empty
/// tmpfs, with each of `keep` bound back at its own path (the binary under test and the
/// output directory, when either lies inside `hidden`).
fn sandboxed(hidden: &Path, keep: &[&Path]) -> Command {
    let mut cmd = Command::new("bwrap");
    cmd.args(["--dev-bind", "/", "/", "--tmpfs"]).arg(hidden);
    for k in keep {
        if k.starts_with(hidden) {
            cmd.arg("--bind").arg(k).arg(k);
        }
    }
    cmd.args(["--die-with-parent", "--"]);
    cmd
}

fn run_build(mut cmd: Command, sigil: &Path, aeon: &Path, out: &Path, name: &str, args: &[&str]) -> (Vec<u8>, String) {
    let bin = out.join(format!("{name}.bin"));
    let lst = out.join(format!("{name}.lst"));
    cmd.arg(sigil).args(["build", "--aeon"]).arg(aeon).arg("--native").args(args);
    cmd.arg("-o").arg(&bin).arg("--emit-lst").arg(&lst);
    cmd.env("AEON_DIR", aeon);
    let o = cmd.output().unwrap_or_else(|e| panic!("spawn {name} build: {e}"));
    assert!(
        o.status.success(),
        "{name}: `sigil build` failed ({}):\n{}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
    let rom = std::fs::read(&bin).unwrap_or_else(|e| panic!("{name}: read {}: {e}", bin.display()));
    let lst = std::fs::read_to_string(&lst).unwrap_or_else(|e| panic!("{name}: read {}: {e}", lst.display()));
    assert!(!rom.is_empty(), "{name}: the build wrote an empty ROM, so nothing was measured");
    (rom, lst)
}

/// `env` as the program word of a plain command, so the sandboxed and unsandboxed
/// builds are spawned by one function.
fn unsandboxed() -> Command {
    Command::new("env")
}

#[test]
fn installed_binary_needs_no_build_tree() {
    let profile = sigil_harness::native::sonic4_profile(false);
    let Some(aeon) = sigil_harness::test_support::reference_tree_for_profile(&profile) else { return };
    let aeon = aeon.canonicalize().expect("canonical aeon");
    let checkout = sigil_harness::source_digest::sigil_source_root();
    let sigil = PathBuf::from(env!("CARGO_BIN_EXE_sigil")).canonicalize().expect("canonical sigil binary");
    let out = fresh_dir("builds");

    let probe = Command::new("bwrap").arg("--version").output();
    assert!(
        matches!(&probe, Ok(o) if o.status.success()),
        "UNMEASURABLE: `bwrap` is not runnable here ({probe:?}), and it is what hides the compiling \
         checkout from the binary. This is not a pass."
    );
    assert!(
        !aeon.starts_with(&checkout),
        "UNMEASURABLE: the reference tree {} lies inside the compiling checkout {}, so hiding the \
         checkout would hide the build's own input. Point AEON_DIR outside it.",
        aeon.display(),
        checkout.display()
    );

    // The hiding control, with the arguments the builds use.
    let control = checkout.join(CONTROL_FILE);
    assert!(control.is_file(), "the control file {} is not in this checkout", control.display());
    let seen = sandboxed(&checkout, &[&sigil, &out])
        .arg("test")
        .arg("-e")
        .arg(&control)
        .status()
        .expect("spawn bwrap for the hiding control");
    assert!(
        !seen.success(),
        "the sandbox still shows {}, so it does not hide the compiling checkout and the builds \
         below would prove nothing",
        control.display()
    );

    for (shape, args) in SHAPES {
        let open_dir = out.join("visible");
        let hidden_dir = out.join("hidden");
        std::fs::create_dir_all(&open_dir).expect("visible out dir");
        std::fs::create_dir_all(&hidden_dir).expect("hidden out dir");

        let (rom_open, _) = run_build(unsandboxed(), &sigil, &aeon, &open_dir, shape, args);
        let (rom_hidden, lst_hidden) =
            run_build(sandboxed(&checkout, &[&sigil, &out]), &sigil, &aeon, &hidden_dir, shape, args);

        assert!(
            rom_open == rom_hidden,
            "{shape}: the ROM built with the compiling checkout hidden ({} bytes, crc {:08x}) differs \
             from the one built with it visible ({} bytes, crc {:08x}), so the binary still reads \
             something from it",
            rom_hidden.len(),
            sigil_harness::native::crc32(&rom_hidden),
            rom_open.len(),
            sigil_harness::native::crc32(&rom_open)
        );
        let (digest, _) = parse_source_digest(&lst_hidden)
            .unwrap_or_else(|e| panic!("{shape}: the sandboxed listing's digest does not parse: {e}"));
        let rooted: Vec<&str> = digest
            .reads
            .iter()
            .filter(|r| r.file.root == DigestRoot::Sigil)
            .map(|r| r.file.path.as_str())
            .collect();
        assert!(
            rooted.is_empty(),
            "{shape}: the build recorded reads under the compiling checkout: {rooted:?}"
        );
        eprintln!(
            "{shape}: built with {} hidden, {} bytes crc {:08x}, identical to the visible build",
            checkout.display(),
            rom_hidden.len(),
            sigil_harness::native::crc32(&rom_hidden)
        );
    }
    let _ = std::fs::remove_dir_all(&out);
}
