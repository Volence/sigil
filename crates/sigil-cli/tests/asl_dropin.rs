//! sigil under the program name `asl`: the drop-in for a disassembly's own build
//! script (`crates/sigil-cli/src/asl_mode.rs`).
//!
//! Two kinds of check live here.
//!
//! **The command line**, which needs nothing outside this checkout: the program
//! name selects the personality and nothing else does, an option asl's pipeline
//! does not pass is refused by name, and `-U` is required. Their expectations come
//! from the mode's own source (`asl_mode::ACCEPTED`) and from `common.lua`'s
//! command line, which is quoted in that module's note.
//!
//! **The files the script reads**, each measured against the reference asl
//! (md5 `61e672562465725a8c102288a7da9098`) run on the same probe beside it, never
//! against a stored copy of its output: the object file's records (order, CPU,
//! address, bytes, and the separate record just before each Z80 record), what the
//! stock `p2bin` (md5 `4f2fff99c3347bafb93b12d5be1db754`) makes of each, the share
//! file under `-c`, and which of `X.p` and `X.log` exist after an error, a warning
//! and a clean run under `-E`. Both binaries come from `s1disasm` at the revision
//! below, read by `git archive` into this crate's target tmp dir, exactly as
//! `as_driver_placement_corpus.rs` reads its reference toolchain; the checkout's
//! working tree is never read. With no suite root, these follow that test's rule
//! (`suite_root_absent`): a failure under `SIGIL_STRICT_GATE`, a named skip only in
//! a declared partial run, UNMEASURABLE otherwise.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use sigil_harness::test_support::{suite_root_absent, unnamed_default_tree};

const REV: &str = "f6ece657c1cf253404312137dfcb8ec15fa42318";
const ASL_MD5: &str = "61e672562465725a8c102288a7da9098";
const P2BIN_MD5: &str = "4f2fff99c3347bafb93b12d5be1db754";
/// `common.lua`'s `assemble_file` command line, before the optional `-c`.
const PIPELINE: [&str; 10] = ["-xx", "-n", "-q", "-A", "-L", "-U", "-E", "-i", ".", "-c"];

fn dir() -> tempfile::TempDir {
    tempfile::Builder::new().prefix("asl-dropin-").tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a tmp dir")
}

/// This build of sigil, installed in `dir` under `name` as a hard link: the same
/// file under a second name, which is what a copy is to the program-name test,
/// without an open-for-write that a test running in parallel could inherit
/// across its own spawn (`ETXTBSY`, "Text file busy"). A symlink where the two
/// paths are on different file systems.
fn sigil_named(dir: &Path, name: &str) -> PathBuf {
    let to = dir.join(name);
    if std::fs::hard_link(env!("CARGO_BIN_EXE_sigil"), &to).is_err() {
        std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_sigil"), &to).expect("link sigil");
    }
    to
}

fn run(exe: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(exe).current_dir(cwd).args(args).output().expect("spawn")
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

const CLEAN: &str = "\tcpu 68000\n\tdc.w $1234\n";

#[test]
fn only_the_program_name_asl_selects_the_personality() {
    let d = dir();
    std::fs::write(d.path().join("probe.asm"), CLEAN).unwrap();
    let args = ["-xx", "-n", "-q", "-A", "-L", "-U", "-E", "-i", ".", "probe.asm"];
    for name in ["asl", "asl.exe", "asl.EXE"] {
        let exe = sigil_named(d.path(), name);
        let _ = std::fs::remove_file(d.path().join("probe.p"));
        let o = run(&exe, d.path(), &args);
        assert!(o.status.success(), "as {name}: {}", text(&o));
        let p = std::fs::read(d.path().join("probe.p")).unwrap_or_else(|_| panic!("as {name}: no probe.p"));
        let (records, _) = sigil_link::decode_code_file(&p).expect("an asl object file");
        assert_eq!(records, vec![sigil_link::CodeRecord { cpu: 0x01, start: 0, bytes: vec![0x12, 0x34] }]);
    }
    // A symlink named asl is the drop-in too: the name is the one it was run as.
    let link = d.path().join("linkdir");
    std::fs::create_dir(&link).unwrap();
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_sigil"), link.join("asl")).unwrap();
    let _ = std::fs::remove_file(d.path().join("probe.p"));
    let o = run(&link.join("asl"), d.path(), &args);
    assert!(o.status.success() && d.path().join("probe.p").exists(), "as a symlink: {}", text(&o));
    // Under any other name it is sigil, and asl's first flag is refused.
    for name in ["sigil", "ASL", "asl.bin"] {
        let exe = sigil_named(d.path(), name);
        let _ = std::fs::remove_file(d.path().join("probe.p"));
        let o = run(&exe, d.path(), &args);
        assert_eq!(o.status.code(), Some(2), "as {name}: {}", text(&o));
        assert!(String::from_utf8_lossy(&o.stderr).contains("unexpected argument '-xx'"), "as {name}: {}", text(&o));
        assert!(!d.path().join("probe.p").exists(), "as {name}, an object file was written");
    }
}

#[test]
fn an_option_asls_pipeline_does_not_pass_is_refused_by_name() {
    let d = dir();
    std::fs::write(d.path().join("probe.asm"), CLEAN).unwrap();
    let asl = sigil_named(d.path(), "asl");
    for opt in ["-x", "-o", "-P", "-g", "-h", "-Z", "-DFOO=1", "--help", "-cpu", "-olist", "-r"] {
        // A stale object file must not survive a refusal: the script reads its
        // presence as success.
        std::fs::write(d.path().join("probe.p"), b"stale").unwrap();
        let _ = std::fs::remove_file(d.path().join("probe.log"));
        let o = run(&asl, d.path(), &["-U", "-E", opt, "probe.asm"]);
        assert_eq!(o.status.code(), Some(4), "{opt}: {}", text(&o));
        let named = format!("'{opt}'");
        assert!(String::from_utf8_lossy(&o.stderr).contains(&named), "{opt}: {}", text(&o));
        assert!(!d.path().join("probe.p").exists(), "{opt}: the stale object file survived");
        let log = std::fs::read_to_string(d.path().join("probe.log")).unwrap_or_default();
        assert!(log.contains(&named), "{opt}: under -E the log must name it, so the script prints it: {log:?}");
    }
    // Every option the pipeline passes is accepted.
    for opt in ["-xx", "-n", "-q", "-A", "-L", "-c"] {
        let o = run(&asl, d.path(), &["-U", opt, "probe.asm"]);
        assert!(o.status.success(), "{opt}: {}", text(&o));
    }
}

#[test]
fn u_is_required_because_sigils_symbols_are_case_sensitive() {
    let d = dir();
    std::fs::write(d.path().join("probe.asm"), CLEAN).unwrap();
    let asl = sigil_named(d.path(), "asl");
    let o = run(&asl, d.path(), &["-xx", "-q", "-E", "-i", ".", "probe.asm"]);
    assert_eq!(o.status.code(), Some(4), "{}", text(&o));
    let log = std::fs::read_to_string(d.path().join("probe.log")).expect("the refusal is in the log");
    assert!(log.contains("-U") && log.contains("case"), "{log}");
    assert!(!d.path().join("probe.p").exists());
}

/// asl reads the argument after `-E` as the log's name when it does not start
/// with `-`, so `-E probe.asm` names the SOURCE as the log. asl then has no source,
/// and a run with nothing to report removes its log: measured on the reference
/// build, `asl -xx -n -q -A -U -E sub/in2.asm` asked for a source on stdin, got
/// none, exited 0, and `sub/in2.asm` was gone. sigil reads the command line the
/// same way and refuses without touching the file.
#[test]
fn e_naming_the_source_as_its_log_leaves_the_source_alone() {
    let d = dir();
    std::fs::write(d.path().join("probe.asm"), CLEAN).unwrap();
    let asl = sigil_named(d.path(), "asl");
    for args in [&["-U", "-E", "probe.asm"][..], &["-U", "-E", "probe.asm", "probe.asm"][..]] {
        let o = run(&asl, d.path(), args);
        assert_eq!(o.status.code(), Some(4), "{args:?}: {}", text(&o));
        assert_eq!(std::fs::read_to_string(d.path().join("probe.asm")).unwrap(), CLEAN, "{args:?}");
    }
}

#[test]
fn an_include_directory_other_than_the_sources_own_is_refused() {
    let d = dir();
    std::fs::write(d.path().join("probe.asm"), CLEAN).unwrap();
    std::fs::create_dir(d.path().join("elsewhere")).unwrap();
    let asl = sigil_named(d.path(), "asl");
    assert!(run(&asl, d.path(), &["-U", "-i", ".", "probe.asm"]).status.success());
    let o = run(&asl, d.path(), &["-U", "-i", "elsewhere", "probe.asm"]);
    assert_eq!(o.status.code(), Some(4), "{}", text(&o));
    assert!(String::from_utf8_lossy(&o.stderr).contains("-i elsewhere"), "{}", text(&o));
}

// ---------------------------------------------------------------------------
// Against the reference asl and the stock p2bin
// ---------------------------------------------------------------------------

/// The reference toolchain: `s1disasm`'s `build_tools/Linux-x86_64` at [`REV`],
/// each binary checked against its md5. `None` after `suite_root_absent` when
/// there is no suite root to read it from.
fn reference_tools(what: &str) -> Option<(tempfile::TempDir, PathBuf)> {
    let root = match unnamed_default_tree() {
        Ok(tree) => tree.path.parent().expect("a tree under the suite root has a parent").to_path_buf(),
        Err(why) => {
            suite_root_absent(what, &why);
            return None;
        }
    };
    let s1 = root.join("s1disasm");
    if !s1.join(".git").exists() {
        suite_root_absent(what, &format!("the suite root {} holds no s1disasm checkout", root.display()));
        return None;
    }
    let work = dir();
    let archive = Command::new("git")
        .arg("-C")
        .arg(&s1)
        .args(["archive", "--format=tar", REV, "build_tools/Linux-x86_64"])
        .output()
        .expect("spawn git");
    assert!(
        archive.status.success(),
        "{} has no revision {REV}: {}",
        s1.display(),
        String::from_utf8_lossy(&archive.stderr)
    );
    let mut tar = Command::new("tar").arg("-x").arg("-C").arg(work.path()).stdin(Stdio::piped()).spawn().unwrap();
    tar.stdin.take().unwrap().write_all(&archive.stdout).unwrap();
    assert!(tar.wait().unwrap().success(), "tar could not unpack the archive");
    let tools = work.path().join("build_tools/Linux-x86_64");
    assert_eq!(md5_of(&tools.join("asl")), ASL_MD5, "the revision's asl is not the reference build");
    assert_eq!(md5_of(&tools.join("p2bin")), P2BIN_MD5, "the revision's p2bin is not the pinned build");
    Some((work, tools))
}

fn md5_of(path: &Path) -> String {
    let out = Command::new("md5sum").arg(path).output().expect("spawn md5sum");
    String::from_utf8_lossy(&out.stdout).split_whitespace().next().unwrap_or("").to_string()
}

/// Run `probe` (file name, source) through the reference asl and through sigil
/// as asl, each in its own directory, with `args`. Returns the two directories.
fn both(tools: &Path, name: &str, source: &str, args: &[&str]) -> (tempfile::TempDir, Output, tempfile::TempDir, Output) {
    let a = dir();
    std::fs::write(a.path().join(name), source).unwrap();
    let ao = Command::new(tools.join("asl"))
        .current_dir(a.path())
        .env("AS_MSGPATH", tools)
        .args(args)
        .arg(name)
        .output()
        .expect("spawn asl");
    let s = dir();
    std::fs::write(s.path().join(name), source).unwrap();
    let asl = sigil_named(s.path(), "asl");
    let so = Command::new(asl).current_dir(s.path()).args(args).arg(name).output().expect("spawn sigil as asl");
    (a, ao, s, so)
}

fn p2bin(tools: &Path, cwd: &Path, instruction: &[&str], p: &str, out: &str) -> Vec<u8> {
    let o = Command::new(tools.join("p2bin")).current_dir(cwd).args(instruction).args([p, out]).output().unwrap();
    assert!(o.status.success(), "p2bin: {}", text(&o));
    std::fs::read(cwd.join(out)).expect("p2bin's image")
}

/// Two Z80 blobs stored `before` over filler, each filler run starting at a
/// `!org` that rewinds the ROM address (skdisasm's shape, `Sound/Z80 Sound
/// Driver.asm`), and one stored `after` into a gap (s1disasm's shape): the
/// filler record before each Z80 record is contiguous with the code before it,
/// and only the record boundary says where a `before` blob goes.
const RECORDS: &str = "\tcpu 68000
\tpadding off
\torg 0
\tdc.w $1111,$2222,$3333,$4444
DrvRam:\tphase $FF0000
\tds.b $20
\tdephase
\t!org DrvRam
Drv:\tdc.b [$10]$EE
\tsave
\t!org 0
\tcpu z80
\tld a,1
\tld b,2
\tnop
\trestore
\tpadding off
\t!org Drv+$10
Drv2:\tdc.b [$8]$DD
\tsave
\t!org $100
\tcpu z80
\tld c,3
\trestore
\tpadding off
\t!org Drv2+8
After:\tdc.w $5555,$6666
DacGap:
\tsave
\t!org $200
\tcpu z80
\tld d,4
\tld e,5
\trestore
\tpadding off
\t!org DacGap+$10
\tdc.w $7777
Size_Drv = $10
Size_Drv2 = 8
Size_Dac = $10
";

const RECORDS_P2BIN: [&str; 4] = [
    "-p=FF",
    "-z=0,uncompressed,Size_Drv,before",
    "-z=100,uncompressed,Size_Drv2,before",
    "-z=200,uncompressed,Size_Dac,after",
];

#[test]
fn the_object_file_has_asls_records_in_asls_order() {
    let Some((_keep, tools)) = reference_tools("asl object-file records against the reference asl") else { return };
    let (a, ao, s, so) = both(&tools, "rec.asm", RECORDS, &PIPELINE);
    assert!(ao.status.success(), "the reference asl failed, so its output is not a source of values: {}", text(&ao));
    assert!(so.status.success(), "sigil as asl: {}", text(&so));
    let (asl_recs, _) = sigil_link::decode_code_file(&std::fs::read(a.path().join("rec.p")).unwrap()).unwrap();
    let (sig_recs, creator) = sigil_link::decode_code_file(&std::fs::read(s.path().join("rec.p")).unwrap()).unwrap();
    assert!(creator.starts_with("sigil "), "{creator}");
    // The shape the claim is about must be present in the oracle's own output,
    // or this compare could not have failed on it: a record contiguous with the
    // one before it, of the same CPU, immediately followed by a Z80 record.
    let separate_before_z80 = asl_recs.windows(3).any(|w| {
        w[0].cpu == 0x01 && w[1].cpu == 0x01 && w[2].cpu == 0x51 && w[0].start + w[0].bytes.len() as u32 == w[1].start
    });
    assert!(separate_before_z80, "the probe no longer has a separate record before a Z80 record: {asl_recs:?}");
    assert_eq!(sig_recs, asl_recs, "sigil's records are not asl's");
    // And the stock p2bin makes the same ROM of both, which is the route's point,
    // and the same ROM sigil's own direct route makes from the same instruction.
    let from_asl = p2bin(&tools, a.path(), &RECORDS_P2BIN, "rec.p", "rec.bin");
    let from_sigil = p2bin(&tools, s.path(), &RECORDS_P2BIN, "rec.p", "rec.bin");
    assert_eq!(from_sigil, from_asl, "p2bin places sigil's object file differently");
    let direct = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .current_dir(s.path())
        .args(["rec.asm", "-o", "direct.bin"])
        .args(RECORDS_P2BIN)
        .output()
        .unwrap();
    assert!(direct.status.success(), "{}", text(&direct));
    assert_eq!(std::fs::read(s.path().join("direct.bin")).unwrap(), from_asl, "the two sigil routes disagree");
}

/// A run longer than one record, just before a Z80 blob stored `after` it:
/// s1disasm with `AllOptimizations = 1` is this shape (0x71E9A bytes). The
/// records split differently from asl's, and p2bin places the blob identically.
#[test]
fn a_run_longer_than_a_record_splits_and_places_as_asls() {
    let Some((_keep, tools)) = reference_tools("a long run before a Z80 blob against the reference asl") else { return };
    let src = "\tcpu 68000
\tpadding off
\torg 0
\trept $9000
\tdc.w $4E71
\tendm
DacGap:
\tsave
\t!org 0
\tcpu z80
\tld a,1
\trestore
\tpadding off
\t!org DacGap+$10
\tdc.w $7777
Size_Dac = $10
";
    let (a, ao, s, so) = both(&tools, "long.asm", src, &PIPELINE);
    assert!(ao.status.success(), "{}", text(&ao));
    assert!(so.status.success(), "{}", text(&so));
    let (sig_recs, _) = sigil_link::decode_code_file(&std::fs::read(s.path().join("long.p")).unwrap()).unwrap();
    assert!(sig_recs.iter().all(|r| r.bytes.len() <= sigil_link::MAX_RECORD));
    assert!(sig_recs.len() > 3, "the long run was not split: {} records", sig_recs.len());
    let instruction = ["-p=FF", "-z=0,uncompressed,Size_Dac,after"];
    assert_eq!(
        p2bin(&tools, s.path(), &instruction, "long.p", "long.bin"),
        p2bin(&tools, a.path(), &instruction, "long.p", "long.bin"),
        "p2bin places sigil's split records differently from asl's"
    );
}

#[test]
fn the_share_file_is_asls_byte_for_byte() {
    let Some((_keep, tools)) = reference_tools("the -c share file against the reference asl") else { return };
    // A label, a label defined after the line, a `set` symbol rebound after the
    // line, a negative value, a value with hex letters, a name listed twice, and
    // a bare `shared`: asl writes the value AT the line, in the order written.
    let src = "\tcpu 68000
\tdc.w 1
Here:\tdc.w 2
S1 set 3
\tshared Here, Later, S1
S1 set 4
Neg = -2
Hexy = $ABCDEF
\tshared Neg, Hexy, Here
\tshared
\tdc.w S1
Later:\tdc.w 5
";
    let (a, ao, s, so) = both(&tools, "sh.asm", src, &PIPELINE);
    assert!(ao.status.success(), "{}", text(&ao));
    assert!(so.status.success(), "{}", text(&so));
    let want = std::fs::read_to_string(a.path().join("sh.h")).expect("asl's share file");
    assert!(want.contains("#define S1 0x3\n") && want.contains("#define Here 0x2\n"), "the oracle changed: {want}");
    let got = std::fs::read_to_string(s.path().join("sh.h")).expect("sigil's share file");
    assert_eq!(got, want);
    // Without -c neither writes one, and both warn per `shared` line.
    let args: Vec<&str> = PIPELINE[..9].to_vec();
    let (a, ao, s, so) = both(&tools, "sh.asm", src, &args);
    assert!(ao.status.success() && so.status.success(), "{}{}", text(&ao), text(&so));
    for d in [&a, &s] {
        assert!(!d.path().join("sh.h").exists());
        let log = std::fs::read_to_string(d.path().join("sh.log")).expect("a log of the warnings");
        assert_eq!(log.lines().filter(|l| l.contains("SHARED ignored")).count(), 3, "{log}");
    }
}

/// What `common.lua` reads: `X.p` present is success, `X.p` absent with `X.log`
/// present is an error it prints, and a log beside an object file is warnings.
#[test]
fn e_leaves_the_files_the_script_reads_as_asl_leaves_them() {
    let Some((_keep, tools)) = reference_tools("-E's files against the reference asl") else { return };
    let cases = [
        ("error", "\tcpu 68000\n\tdc.w 1\n\tdc.w NotDefinedHere\n"),
        ("warning", "\tcpu 68000\n\tdc.w 1\n\twarning \"from the author\"\n\tdc.w 2\n"),
        ("clean", CLEAN),
    ];
    let args: Vec<&str> = PIPELINE[..9].to_vec();
    for (what, src) in cases {
        let a = dir();
        let s = dir();
        for d in [&a, &s] {
            std::fs::write(d.path().join("e.asm"), src).unwrap();
            // Both leftovers of an earlier run, so each tool must decide about each.
            std::fs::write(d.path().join("e.p"), b"stale").unwrap();
            std::fs::write(d.path().join("e.log"), b"stale").unwrap();
        }
        let ao = Command::new(tools.join("asl"))
            .current_dir(a.path())
            .env("AS_MSGPATH", &tools)
            .args(&args)
            .arg("e.asm")
            .output()
            .unwrap();
        let asl = sigil_named(s.path(), "asl");
        let so = Command::new(&asl).current_dir(s.path()).args(&args).arg("e.asm").output().unwrap();
        let state = |d: &Path| {
            let p = std::fs::read(d.join("e.p")).ok();
            let log = std::fs::read_to_string(d.join("e.log")).ok();
            (p.map(|b| b != b"stale"), log.map(|l| l != "stale"))
        };
        let want = state(a.path());
        assert_eq!(state(s.path()), want, "{what}: (fresh object file?, fresh log?) differ; asl {}, sigil {}", text(&ao), text(&so));
        assert_eq!(ao.status.success(), so.status.success(), "{what}: exit status class differs");
        if what == "error" {
            assert_eq!(want, (None, Some(true)), "the oracle changed for {what}");
            let log = std::fs::read_to_string(s.path().join("e.log")).unwrap();
            assert!(log.contains("e.asm(3)") && log.contains("NotDefinedHere"), "{log}");
        }
    }
}
