//! The flat image `sigil <file.asm>` writes is bounded by the cartridge window, and
//! a section placed outside it is refused BY NAME, at its line, with exit 1.
//!
//! ## Why this has its own gate
//!
//! The image buffer used to be sized from the highest section's LMA plus its
//! length. One `org` past the cartridge, and one byte after it, therefore sized
//! the buffer from the ADDRESS: `org -1` asked the allocator for 4 GiB and, with
//! `--hex`, for 96 GiB of rendered text, which is an allocator abort (SIGABRT,
//! exit 134) before any diagnostic is printed. An abort is not an error a caller
//! can read, and `catch_unwind` cannot turn one into one. `org $FF0000` (68000 work
//! RAM) did not abort: it wrote a 16 MiB image with a RAM byte at its end and
//! exited 0, which is the quieter failure of the same sizing.
//!
//! The assertions are about the CLI PROCESS: the refusal must be a rendered
//! `file(line): error:` line on stderr and a status of exactly 1 (a signal shows
//! up as `code() == None`, which these assertions reject by name), and the
//! allocator's own message must be absent. A control case places the LAST byte of
//! the cartridge and must still succeed, so the bound is the window's end and not
//! one short of it.

use std::process::{Command, Output};

/// Assemble `body` (after a `cpu 68000` line) from `name` in a fresh directory and
/// return the process output plus the directory (kept alive for `-o` files).
fn run(name: &str, body: &str, extra: &[&str]) -> (Output, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    std::fs::write(&path, format!("\tcpu 68000\n{body}")).expect("write source");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(&path)
        .args(extra)
        .output()
        .expect("spawn sigil");
    (out, dir)
}

/// The refusal shape every out-of-window case must take: exit status exactly 1
/// (not a signal, not the allocator's 134), a located error line naming the file
/// and the byte-emitting line, `needle` in that line, and no allocator message.
fn assert_refused(out: &Output, file: &str, line: u32, needle: &str) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit 1, got {:?} (signal-terminated when None)\nstderr: {stderr}",
        out.status
    );
    assert!(
        !stderr.contains("memory allocation"),
        "the allocator spoke instead of the linker\nstderr: {stderr}"
    );
    let prefix = format!("{file}({line}): error: section `");
    let located = stderr.lines().find(|l| l.contains(&prefix));
    let Some(located) = located else {
        panic!("no line starts with {prefix:?}\nstderr: {stderr}");
    };
    assert!(located.contains(needle), "expected {needle:?} in {located:?}");
    assert!(
        !located.contains('\u{2014}') && !located.contains('\u{2013}'),
        "dash in diagnostic: {located:?}"
    );
}

#[test]
fn org_minus_one_is_refused_not_a_four_gib_allocation() {
    let (out, _dir) = run("org_neg1.asm", "\torg -1\n\tdc.b 1\n", &[]);
    assert_refused(&out, "org_neg1.asm", 3, "LMA 0xFFFFFFFF is in no ROM region");
    assert!(out.stdout.is_empty(), "stdout: {}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn org_minus_one_with_hex_is_refused_before_rendering() {
    let (out, _dir) = run("org_neg1.asm", "\torg -1\n\tdc.b 1\n", &["--hex"]);
    assert_refused(&out, "org_neg1.asm", 3, "LMA 0xFFFFFFFF is in no ROM region");
    assert!(out.stdout.is_empty(), "stdout: {}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn a_byte_in_work_ram_is_refused_by_region_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("org_ram.asm");
    let bin = dir.path().join("ram.bin");
    std::fs::write(&src, "\tcpu 68000\n\torg $FF0000\n\tdc.b 1\n").expect("write source");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(&src)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("spawn sigil");
    assert_refused(&out, "org_ram.asm", 3, "lies in non-ROM region `work_ram` (m68k_ram)");
    assert!(!bin.exists(), "an image was written for a refused program");
}

#[test]
fn a_byte_just_past_the_cartridge_is_refused() {
    let (out, _dir) = run("org_past.asm", "\torg $400000\n\tdc.b 1\n", &[]);
    assert_refused(&out, "org_past.asm", 3, "LMA 0x400000 is in no ROM region");
}

#[test]
fn a_word_straddling_the_cartridge_end_is_refused_with_the_overshoot() {
    let (out, _dir) = run("org_straddle.asm", "\torg $3FFFFF\n\tdc.w 1\n", &[]);
    assert_refused(&out, "org_straddle.asm", 3, "overflows region `cartridge` (ends 0x400000), over by 1 bytes");
}

/// Control: the last byte of the window is inside it. Without this the four
/// refusals above could pass against a bound one byte too tight.
#[test]
fn the_last_cartridge_byte_is_still_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("org_last.asm");
    let bin = dir.path().join("last.bin");
    std::fs::write(&src, "\tcpu 68000\n\torg $3FFFFF\n\tdc.b 1\n").expect("write source");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(&src)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("spawn sigil");
    assert_eq!(out.status.code(), Some(0), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let image = std::fs::read(&bin).expect("read image");
    assert_eq!(image.len(), 0x40_0000, "the image ends at the cartridge window's end");
    assert_eq!(image[0x3F_FFFF], 1);
}
