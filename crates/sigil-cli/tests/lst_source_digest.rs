//! The `.lst` source digest, end to end: real `sigil build` runs over the reference tree.
//!
//! Asserted, each with its control:
//!
//!  * COMPLETENESS, from outside the recorder. Each of the four shipped shapes is built
//!    under an `open(2)`/spawn interposer, and the files the kernel was asked to open
//!    read-only plus the executables spawned must EQUAL the digest's READ rows, in both
//!    directions. The interposer seeing every row is the control that it is not blind to
//!    a read mechanism; a file it saw that is not a row is the omission this parcel exists
//!    to make impossible.
//!  * CONTENT, NOT TIME, on a private copy of the tree: two builds give byte-identical
//!    sections; touching every file the build read leaves the section identical; one
//!    content edit to one input of each kind (an `.emp` module, the AS residual root, an
//!    included `.asm`, a BINCLUDE'd blob, an `embed`ed blob, `map.toml`, a sound source
//!    whose generated file the build reads back, the `convsym` tool) moves exactly the
//!    rows it should and the aggregate; a new module file moves `DIGEST-SCAN`; and undoing
//!    every edit restores the section byte for byte. A half-fix that records the `.emp`
//!    modules but not one of the other kinds goes red on that kind.
//!
//! Reference tree: `AEON_DIR` (the house pattern; `SIGIL_STRICT_GATE` hard-fails a
//! missing tree).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon \
//!   cargo test --release -p sigil-cli --test lst_source_digest
//! ```

use sigil_link::{parse_source_digest, DigestOrigin, DigestRead, DigestRoot, SourceDigest};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

// Every build here writes into a tree or reads one another build writes; one at a time.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const SHAPES: &[(&str, &[&str])] = &[
    ("s4", &[]),
    ("s4.debug", &["--debug"]),
    ("demo", &["--game", "demo"]),
    ("demo.debug", &["--game", "demo", "--debug"]),
];

fn aeon_dir() -> Option<PathBuf> {
    let profile = sigil_harness::native::sonic4_profile(false);
    sigil_harness::test_support::reference_tree_for_profile(&profile)
}

fn fresh_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("lst_source_digest_{case}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir.canonicalize().expect("canonical scratch dir")
}

struct Built {
    lst: String,
}

/// `sigil build --aeon <aeon> --native <args> -o <out>/<name>.bin --emit-lst <out>/<name>.lst`.
fn build(aeon: &Path, out: &Path, name: &str, args: &[&str], env: &[(&str, &Path)]) -> Result<Built, String> {
    let bin = out.join(format!("{name}.bin"));
    let lst = out.join(format!("{name}.lst"));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--aeon"]).arg(aeon).arg("--native").args(args);
    cmd.arg("-o").arg(&bin).arg("--emit-lst").arg(&lst);
    cmd.env("AEON_DIR", aeon);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let o = cmd.output().map_err(|e| format!("spawn sigil: {e}"))?;
    if !o.status.success() {
        return Err(format!(
            "sigil build {name} {args:?} over {} failed:\n{}",
            aeon.display(),
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    let lst = std::fs::read_to_string(&lst).map_err(|e| format!("read {}: {e}", lst.display()))?;
    Ok(Built { lst })
}

/// The section's text: from the listing's first byte through `DIGEST-END` and its blank.
fn section(lst: &str) -> &str {
    let end = lst.find("DIGEST-END\n\n").expect("a listing with a closed digest") + "DIGEST-END\n\n".len();
    &lst[..end]
}

fn digest(lst: &str) -> SourceDigest {
    parse_source_digest(lst).unwrap_or_else(|e| panic!("the listing's digest does not parse: {e}")).0
}

fn aggregate_line(lst: &str) -> String {
    section(lst).lines().find(|l| l.starts_with("DIGEST-AGGREGATE ")).expect("an aggregate").to_string()
}

/// A row's file on disk.
fn resolve(aeon: &Path, r: &DigestRead) -> PathBuf {
    let base = match r.file.root {
        DigestRoot::Aeon => aeon.to_path_buf(),
        DigestRoot::Sigil => sigil_harness::source_digest::sigil_source_root(),
        DigestRoot::Filesystem => PathBuf::from("/"),
    };
    base.join(&r.file.path)
}

/// The aeon-root rows as `path -> (crc, size, origin)`.
fn aeon_rows(d: &SourceDigest) -> BTreeMap<String, (u32, u64, DigestOrigin)> {
    d.reads
        .iter()
        .filter(|r| r.file.root == DigestRoot::Aeon)
        .map(|r| (r.file.path.clone(), (r.crc, r.size, r.origin)))
        .collect()
}

/// The paths whose row differs between two digests, including rows present in one only.
fn changed_rows(a: &SourceDigest, b: &SourceDigest) -> BTreeSet<String> {
    let (a, b) = (aeon_rows(a), aeon_rows(b));
    a.keys().chain(b.keys()).filter(|k| a.get(*k) != b.get(*k)).cloned().collect()
}

// ---------------------------------------------------------------------------
// Completeness, witnessed by the kernel's own record of what was opened.
// ---------------------------------------------------------------------------

/// An `LD_PRELOAD` interposer that appends every successful `open`/`openat` and every
/// spawn to `$SIGIL_OPEN_LOG` as `tag<TAB>pid<TAB>flags<TAB>absolute path`. It logs
/// through a raw `openat` syscall, so logging never re-enters `open`.
const INTERPOSER_C: &str = r#"
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <limits.h>
#include <spawn.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

static void emit(const char *tag, int dirfd, int flags, const char *path) {
    const char *log = getenv("SIGIL_OPEN_LOG");
    if (!log || !path) return;
    char abs[PATH_MAX * 2];
    const char *p = path;
    if (path[0] != '/') {
        char base[PATH_MAX];
        base[0] = 0;
        if (dirfd == AT_FDCWD) {
            if (!getcwd(base, sizeof base)) base[0] = 0;
        } else {
            char link[64];
            snprintf(link, sizeof link, "/proc/self/fd/%d", dirfd);
            ssize_t n = readlink(link, base, sizeof base - 1);
            base[n > 0 ? n : 0] = 0;
        }
        snprintf(abs, sizeof abs, "%s/%s", base, path);
        p = abs;
    }
    char line[PATH_MAX * 2 + 64];
    int len = snprintf(line, sizeof line, "%s\t%d\t%d\t%s\n", tag, (int)getpid(), flags, p);
    if (len <= 0) return;
    if ((size_t)len >= sizeof line) len = (int)sizeof line - 1;
    int fd = (int)syscall(SYS_openat, AT_FDCWD, log, O_WRONLY | O_APPEND | O_CREAT | O_CLOEXEC, 0644);
    if (fd >= 0) {
        ssize_t w = write(fd, line, (size_t)len);
        (void)w;
        close(fd);
    }
}

static mode_t mode_arg(int flags, va_list ap) {
    if ((flags & O_CREAT) || ((flags & O_TMPFILE) == O_TMPFILE)) return (mode_t)va_arg(ap, int);
    return 0;
}

#define OPEN_LIKE(NAME)                                                            \
    int NAME(const char *path, int flags, ...) {                                   \
        static int (*real)(const char *, int, ...);                                \
        if (!real) real = (int (*)(const char *, int, ...))dlsym(RTLD_NEXT, #NAME); \
        va_list ap;                                                                \
        va_start(ap, flags);                                                       \
        mode_t mode = mode_arg(flags, ap);                                         \
        va_end(ap);                                                                \
        int fd = real(path, flags, mode);                                          \
        if (fd >= 0) emit("open", AT_FDCWD, flags, path);                          \
        return fd;                                                                 \
    }
OPEN_LIKE(open)
OPEN_LIKE(open64)

#define OPENAT_LIKE(NAME)                                                            \
    int NAME(int dirfd, const char *path, int flags, ...) {                          \
        static int (*real)(int, const char *, int, ...);                             \
        if (!real) real = (int (*)(int, const char *, int, ...))dlsym(RTLD_NEXT, #NAME); \
        va_list ap;                                                                  \
        va_start(ap, flags);                                                         \
        mode_t mode = mode_arg(flags, ap);                                           \
        va_end(ap);                                                                  \
        int fd = real(dirfd, path, flags, mode);                                     \
        if (fd >= 0) emit("open", dirfd, flags, path);                               \
        return fd;                                                                   \
    }
OPENAT_LIKE(openat)
OPENAT_LIKE(openat64)

int posix_spawn(pid_t *pid, const char *path, const posix_spawn_file_actions_t *fa,
                const posix_spawnattr_t *attr, char *const argv[], char *const envp[]) {
    static int (*real)(pid_t *, const char *, const posix_spawn_file_actions_t *,
                       const posix_spawnattr_t *, char *const[], char *const[]);
    if (!real) real = dlsym(RTLD_NEXT, "posix_spawn");
    emit("exec", AT_FDCWD, 0, path);
    return real(pid, path, fa, attr, argv, envp);
}

int posix_spawnp(pid_t *pid, const char *file, const posix_spawn_file_actions_t *fa,
                 const posix_spawnattr_t *attr, char *const argv[], char *const envp[]) {
    static int (*real)(pid_t *, const char *, const posix_spawn_file_actions_t *,
                       const posix_spawnattr_t *, char *const[], char *const[]);
    if (!real) real = dlsym(RTLD_NEXT, "posix_spawnp");
    emit("exec", AT_FDCWD, 0, file);
    return real(pid, file, fa, attr, argv, envp);
}

int execve(const char *path, char *const argv[], char *const envp[]) {
    static int (*real)(const char *, char *const[], char *const[]);
    if (!real) real = dlsym(RTLD_NEXT, "execve");
    emit("exec", AT_FDCWD, 0, path);
    return real(path, argv, envp);
}

int execvp(const char *file, char *const argv[]) {
    static int (*real)(const char *, char *const[]);
    if (!real) real = dlsym(RTLD_NEXT, "execvp");
    emit("exec", AT_FDCWD, 0, file);
    return real(file, argv);
}
"#;

fn compile_interposer(dir: &Path) -> PathBuf {
    let src = dir.join("openlog.c");
    let so = dir.join("openlog.so");
    std::fs::write(&src, INTERPOSER_C).expect("write the interposer source");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2", "-o"])
        .arg(&so)
        .arg(&src)
        .arg("-ldl")
        .status()
        .expect("run cc: the completeness witness needs a C compiler, and without one it is unmeasurable, not a pass");
    assert!(status.success(), "cc could not build the open(2) interposer");
    so
}

/// What the kernel was asked to open read-only, and to spawn, excluding the system
/// directories every process reads (the loader, locale, `/proc`) and files gone by now
/// (the appendix pass's own temporaries).
fn observed(log: &str) -> (BTreeSet<PathBuf>, BTreeSet<PathBuf>) {
    const O_ACCMODE: i64 = 0o3;
    const O_DIRECTORY: i64 = 0o200000;
    const SYSTEM: &[&str] = &["/proc/", "/sys/", "/dev/", "/etc/", "/usr/", "/lib/", "/lib64/", "/run/"];
    let (mut reads, mut execs) = (BTreeSet::new(), BTreeSet::new());
    for line in log.lines() {
        let f: Vec<&str> = line.splitn(4, '\t').collect();
        let [tag, _pid, flags, path] = f[..] else { panic!("malformed interposer line {line:?}") };
        let Ok(real) = std::fs::canonicalize(path) else { continue };
        if SYSTEM.iter().any(|s| real.to_string_lossy().starts_with(s)) {
            continue;
        }
        match tag {
            "exec" => {
                execs.insert(real);
            }
            "open" => {
                let flags: i64 = flags.parse().expect("numeric flags");
                if flags & O_ACCMODE == 0 && flags & O_DIRECTORY == 0 && real.is_file() {
                    reads.insert(real);
                }
            }
            other => panic!("unknown interposer tag {other:?}"),
        }
    }
    (reads, execs)
}

#[test]
fn every_file_a_build_opens_is_a_digest_row_and_every_row_was_opened() {
    let Some(aeon) = aeon_dir() else { return };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let dir = fresh_dir("witness");
    let interposer = compile_interposer(&dir);
    let aeon = aeon.canonicalize().expect("canonical aeon");
    for (shape, args) in SHAPES {
        let log = dir.join(format!("{shape}.openlog"));
        let built = build(&aeon, &dir, shape, args, &[("LD_PRELOAD", &interposer), ("SIGIL_OPEN_LOG", &log)])
            .unwrap_or_else(|e| panic!("{e}"));
        let d = digest(&built.lst);
        let rows: BTreeSet<PathBuf> =
            d.reads.iter().map(|r| std::fs::canonicalize(resolve(&aeon, r)).expect("a row's file exists")).collect();
        let text = std::fs::read_to_string(&log).expect("the interposer wrote no log, so it never loaded");
        let (reads, execs) = observed(&text);
        let seen: BTreeSet<PathBuf> = reads.union(&execs).cloned().collect();

        let blind: Vec<&PathBuf> = rows.difference(&seen).collect();
        assert!(
            blind.is_empty(),
            "{shape}: the interposer did not see {} file(s) the recorder did, so it is blind to some \
             read mechanism and the omission check below proves nothing about it: {blind:?}",
            blind.len()
        );
        let unlisted: Vec<&PathBuf> = seen.difference(&rows).collect();
        assert!(
            unlisted.is_empty(),
            "{shape}: the build opened {} file(s) its digest does not name, so an edit to one would \
             leave a stale artifact reading as fresh: {unlisted:?}",
            unlisted.len()
        );
        let tool_rows: Vec<&DigestRead> = d.reads.iter().filter(|r| r.origin == DigestOrigin::Tool).collect();
        assert!(
            !tool_rows.is_empty() && !execs.is_empty(),
            "{shape}: no spawn was observed or recorded, so the tool half of this check measured nothing"
        );
        eprintln!("{shape}: {} rows, the kernel saw the same {} files", rows.len(), seen.len());
    }
}

// ---------------------------------------------------------------------------
// Content, not time.
// ---------------------------------------------------------------------------

/// The quoted path after each `directive` in `text` (a `BINCLUDE "x"`, an `embed("x"`).
fn quoted_after(text: &str, directive: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(directive) {
        let after = &rest[i + directive.len()..];
        let after = after.trim_start_matches([' ', '\t', '(']);
        if let Some(body) = after.strip_prefix('"') {
            if let Some(end) = body.find('"') {
                out.push(body[..end].to_string());
            }
        }
        rest = &rest[i + directive.len()..];
    }
    out
}

/// One content edit, and the rows it must move.
struct Edit {
    kind: &'static str,
    file: String,
    mutate: fn(&[u8]) -> Vec<u8>,
    moves: BTreeSet<String>,
}

fn append_line(comment: &'static str) -> impl Fn(&[u8]) -> Vec<u8> {
    move |b: &[u8]| {
        let mut v = b.to_vec();
        v.extend_from_slice(comment.as_bytes());
        v
    }
}

fn flip_last_byte(b: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    let last = v.last_mut().expect("a non-empty blob");
    *last ^= 0x01;
    v
}

fn append_emp_comment(b: &[u8]) -> Vec<u8> {
    append_line("\n// digest probe\n")(b)
}

fn append_asm_comment(b: &[u8]) -> Vec<u8> {
    append_line("\n; digest probe\n")(b)
}

fn append_toml_comment(b: &[u8]) -> Vec<u8> {
    append_line("\n# digest probe\n")(b)
}

fn append_zero_byte(b: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    v.push(0);
    v
}

/// The pitch-table source's first data cell `$00` becomes `$01`: the same edit
/// `seam2::emit_pitchtable_doctored` makes, which moves the generated table's bytes.
fn doctor_pitchtable(b: &[u8]) -> Vec<u8> {
    let mut s = String::from_utf8(b.to_vec()).expect("UTF-8 source");
    let anchor = s.find("dc.b").expect("the pitch table has a dc.b");
    let cell = s[anchor..].find("$00").expect("its first data cell is $00") + anchor;
    s.replace_range(cell..cell + 3, "$01");
    s.into_bytes()
}

/// Apply `edit` to the tree, build, undo it, and return the build (or its failure).
fn with_edit(tree: &Path, out: &Path, file: &str, mutate: &dyn Fn(&[u8]) -> Vec<u8>) -> Result<Built, String> {
    let path = tree.join(file);
    let original = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let edited = mutate(&original);
    assert_ne!(edited, original, "the edit to {file} changed nothing, so its build would test nothing");
    std::fs::write(&path, &edited).expect("write the edit");
    assert_eq!(std::fs::read(&path).expect("read back"), edited, "the edit to {file} did not land");
    let result = build(tree, out, "s4", &[], &[]);
    std::fs::write(&path, &original).expect("undo the edit");
    result
}

#[test]
fn the_digest_moves_with_content_and_never_with_time() {
    let Some(aeon) = aeon_dir() else { return };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let dir = fresh_dir("content");
    let tree = dir.join("aeon");
    std::fs::create_dir_all(&tree).expect("tree dir");
    let status = Command::new("cp")
        .arg("-a")
        .arg(format!("{}/.", aeon.display()))
        .arg(&tree)
        .status()
        .expect("run cp");
    assert!(status.success(), "could not copy the reference tree");
    let git = tree.join(".git");
    if git.is_dir() {
        std::fs::remove_dir_all(&git).expect("drop the copy's .git");
    } else if git.exists() {
        std::fs::remove_file(&git).expect("drop the copy's .git");
    }
    let tree = tree.canonicalize().expect("canonical tree");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).expect("out dir");

    let base = build(&tree, &out, "s4", &[], &[]).unwrap_or_else(|e| panic!("{e}"));
    let s0 = section(&base.lst).to_string();
    let d0 = digest(&base.lst);
    let rows0 = aeon_rows(&d0);

    // Two builds of one tree: byte-identical sections.
    let again = build(&tree, &out, "s4", &[], &[]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(section(&again.lst), s0, "two builds of one tree gave different digests");

    // Every file the build read, touched forward an hour: an identical section.
    let later = SystemTime::now() + Duration::from_secs(3600);
    let mut touched = 0usize;
    for path in rows0.keys() {
        let p = tree.join(path);
        let f = std::fs::File::options().write(true).open(&p).expect("open for touch");
        f.set_modified(later).expect("set mtime");
        let now = std::fs::metadata(&p).expect("stat").modified().expect("mtime");
        assert!(now >= later - Duration::from_secs(1), "the touch of {path} did not land");
        touched += 1;
    }
    assert!(touched > 100, "only {touched} files touched, the digest names too few rows to test");
    let after_touch = build(&tree, &out, "s4", &[], &[]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(section(&after_touch.lst), s0, "touching {touched} files moved the digest; it is reading time, not content");

    // One edit per kind of input. Each kind's file is DERIVED: from the rows, from the
    // profile, or from the source text of the rows (what the build's own reads name).
    let profile = sigil_harness::native::sonic4_profile(false);
    let game_root = profile.game_root_rel.to_string();
    let map = profile
        .map_path(Path::new(""))
        .to_str()
        .expect("UTF-8 map path")
        .trim_start_matches('/')
        .to_string();
    let source_rows: Vec<&String> =
        rows0.iter().filter(|(_, (_, _, o))| *o == DigestOrigin::Source).map(|(p, _)| p).collect();
    let text_of = |p: &str| std::fs::read_to_string(tree.join(p)).unwrap_or_default();
    let emp_module = source_rows
        .iter()
        .find(|p| p.ends_with(".emp"))
        .map(|p| p.to_string())
        .expect("the digest names no .emp module: the manifest read is not recorded");
    let included_asm = source_rows
        .iter()
        .find(|p| p.ends_with(".asm") && **p != &game_root)
        .map(|p| p.to_string())
        .expect("the digest names no included .asm: the AS include read is not recorded");
    // Every BINCLUDE target an .asm the build read names must be a row. At a revision
    // whose AS residual BINCLUDEs nothing there is no corpus file of that kind to edit,
    // and the read site is measured on a synthetic tree instead, in
    // `sigil-frontend-as/tests/read_set_as_reads.rs`; the count is printed either way.
    let binclude_targets: BTreeSet<String> = source_rows
        .iter()
        .filter(|p| p.ends_with(".asm"))
        .flat_map(|p| quoted_after(&text_of(p), "BINCLUDE"))
        .collect();
    let unlisted: Vec<&String> = binclude_targets.iter().filter(|t| !rows0.contains_key(*t)).collect();
    assert!(unlisted.is_empty(), "BINCLUDE targets of an .asm the build read are not rows: {unlisted:?}");
    let bincluded: Vec<String> = binclude_targets
        .iter()
        .filter(|t| rows0.get(*t).is_some_and(|(_, _, o)| *o == DigestOrigin::Source))
        .cloned()
        .collect();
    eprintln!(
        "BINCLUDE: {} target(s) named by the .asm rows, {} of them editable source rows",
        binclude_targets.len(),
        bincluded.len()
    );
    let embedded: Vec<String> = source_rows
        .iter()
        .filter(|p| p.ends_with(".emp"))
        .flat_map(|p| quoted_after(&text_of(p), "embed("))
        .filter(|t| rows0.get(t).is_some_and(|(_, _, o)| *o == DigestOrigin::Source))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    assert!(
        !embedded.is_empty(),
        "no embed() target named by an .emp the build read is a digest row: the .emp embed read is not recorded"
    );
    const PITCH_SOURCE: &str = "games/sonic4/data/sound/movingtrucks_pitchtable.emp";
    const PITCH_GENERATED: &str = "engine/sound/generated/movingtrucks_pitchtable.bin";
    assert_eq!(
        rows0.get(PITCH_GENERATED).map(|r| r.2),
        Some(DigestOrigin::Generated),
        "the generated pitch table is not a generated row"
    );
    assert!(rows0.contains_key(PITCH_SOURCE), "the pitch table's source is not a row");
    let tool = rows0
        .iter()
        .find(|(_, (_, _, o))| *o == DigestOrigin::Tool)
        .map(|(p, _)| p.clone())
        .expect("the digest names no tool: the convsym spawn is not recorded");
    assert!(rows0.contains_key(&game_root), "the AS residual root {game_root} is not a row");
    assert!(rows0.contains_key(&map), "the game map {map} is not a row");

    let one = |p: &str| -> BTreeSet<String> { [p.to_string()].into() };
    let mut edits = vec![
        Edit { kind: "an .emp module", file: emp_module.clone(), mutate: append_emp_comment, moves: one(&emp_module) },
        Edit { kind: "the AS residual root", file: game_root.clone(), mutate: append_asm_comment, moves: one(&game_root) },
        Edit { kind: "an included .asm", file: included_asm.clone(), mutate: append_asm_comment, moves: one(&included_asm) },
        Edit { kind: "map.toml", file: map.clone(), mutate: append_toml_comment, moves: one(&map) },
        Edit {
            kind: "a sound source whose generated file the build reads back",
            file: PITCH_SOURCE.to_string(),
            mutate: doctor_pitchtable,
            moves: [PITCH_SOURCE.to_string(), PITCH_GENERATED.to_string()].into(),
        },
        Edit { kind: "the convsym tool", file: tool.clone(), mutate: append_zero_byte, moves: one(&tool) },
    ];
    // A blob edit can fail the build for reasons of its own (a size guard, a
    // decompressor); the first candidate that builds is the one measured.
    for (kind, candidates) in [("a BINCLUDE'd blob", &bincluded), ("an embed()ed blob", &embedded)] {
        if candidates.is_empty() {
            continue;
        }
        let mut measured = false;
        for file in candidates.iter().take(6) {
            let size = std::fs::metadata(tree.join(file)).map(|m| m.len()).unwrap_or(0);
            if size == 0 {
                continue;
            }
            if with_edit(&tree, &out, file, &flip_last_byte).is_ok() {
                edits.push(Edit { kind, file: file.clone(), mutate: flip_last_byte, moves: one(file) });
                measured = true;
                break;
            }
        }
        assert!(measured, "no {kind} candidate builds after a one-byte edit: {candidates:?}");
    }

    let aggregate0 = aggregate_line(&base.lst);
    for e in &edits {
        let built = with_edit(&tree, &out, &e.file, &e.mutate).unwrap_or_else(|err| panic!("{}: {err}", e.kind));
        let d = digest(&built.lst);
        let moved = changed_rows(&d0, &d);
        assert_eq!(
            moved, e.moves,
            "editing {} ({}) moved rows {:?}; it must move exactly {:?}",
            e.kind, e.file, moved, e.moves
        );
        assert_ne!(aggregate_line(&built.lst), aggregate0, "editing {} left the aggregate unchanged", e.kind);
        eprintln!("{}: {} moved {moved:?}", e.kind, e.file);
    }

    // A module that APPEARS: a new scan member and a new row, nothing else.
    let probe_rel = "engine/zz_digest_probe.emp";
    let probe = tree.join(probe_rel);
    std::fs::write(&probe, "module zz_digest_probe\n").expect("write the probe module");
    let with_probe = build(&tree, &out, "s4", &[], &[]);
    std::fs::remove_file(&probe).expect("remove the probe module");
    let d = digest(&with_probe.unwrap_or_else(|e| panic!("a new empty module failed the build: {e}")).lst);
    assert_eq!(d.scan_files, d0.scan_files + 1, "the new module is not a scan member");
    assert_ne!(d.scan_crc, d0.scan_crc, "the scan identity did not move with its membership");
    assert_eq!(changed_rows(&d0, &d), one(probe_rel), "a new module must add exactly its own row");

    // Every edit undone: the original section, byte for byte.
    let restored = build(&tree, &out, "s4", &[], &[]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(section(&restored.lst), s0, "undoing every edit did not restore the digest");
}
