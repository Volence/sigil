//! Byte-exact end-to-end test: shells the built `sigil` binary and asserts
//! the output image and `--hex` string match the golden sample exactly.

use std::process::Command;

const GOLDEN_SRC: &str = "        cpu z80\n        phase 0\n        nop\n        ld a,5\n        ld b,10\n        ld b,c\n        add a,b\n        jp 1234h\n";

const GOLDEN_BYTES: [u8; 10] = [0x00, 0x3E, 0x05, 0x06, 0x0A, 0x41, 0x80, 0xC3, 0x34, 0x12];

const GOLDEN_HEX: &str = "00 3E 05 06 0A 41 80 C3 34 12";

fn unique_temp_dir() -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    dir.push(format!("sigil_e2e_{}_{}", std::process::id(), nanos));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn golden_sample_end_to_end() {
    let dir = unique_temp_dir();
    let asm_path = dir.join("golden.asm");
    let bin_path = dir.join("golden.bin");
    std::fs::write(&asm_path, GOLDEN_SRC).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(&asm_path)
        .arg("-o")
        .arg(&bin_path)
        .arg("--hex")
        .output()
        .expect("failed to spawn the sigil binary");

    assert!(
        output.status.success(),
        "sigil exited with failure; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let image = std::fs::read(&bin_path).expect("output .bin was not written");
    assert_eq!(image, GOLDEN_BYTES.to_vec(), "output image bytes mismatch");

    let stdout = String::from_utf8(output.stdout).expect("stdout was not valid utf8");
    assert_eq!(stdout.trim_end(), GOLDEN_HEX, "--hex output mismatch");

    // Clean up temp files so the test is re-runnable and parallel-safe.
    let _ = std::fs::remove_dir_all(&dir);
}
