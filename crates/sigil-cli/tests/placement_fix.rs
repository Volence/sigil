use std::process::Command;

#[test]
fn single_file_growth_overlap_is_fixed() {
    // L-H.1 SILENT-OVERLAP DEFECT (Plan 7 item-7-pre): a single file with two
    // sections, where the FIRST section's code GROWS under linker width relaxation,
    // silently overwrites the SECOND section's baked bytes.
    //
    // WHY IT HAPPENS (baked next_lma chain, no placement pass on the single-file
    // path):
    //   * The emp frontend bakes each section's LMA as a running baseline prefix
    //     sum: on the `section data` boundary it does `next_lma +=
    //     builder.current_offset()` then `switch_section_lma(...)`
    //     (sigil-frontend-emp/src/lower/mod.rs:228-231). It uses each fragment's
    //     BASELINE width; a `jmp <sym>` counts as 4 (`emit_fragment(frag, 4)`,
    //     lower/code.rs:150).
    //   * So `code` (one `jmp p`) is baked at 4 bytes → `data` is baked at LMA 4.
    //   * `jmp p` targets VMA $8000 (the section's `vma:`). The asl width rule
    //     cannot encode $8000 as abs.w (sign-extension makes it $FFFF8000), so
    //     `resolve_layout` GROWS the jmp to abs.l = 6 bytes: `4E F9 00 00 80 00`.
    //   * The single-file CLI tail (`link_to_image` → `flatten`,
    //     sigil-cli/src/main.rs / sigil-link/src/lib.rs:415-432) runs NO placement
    //     pass and flattens UNCHECKED: sections are copied in order at their baked
    //     LMAs, so `data`'s 4 bytes at LMA 4 stomp the jmp operand's last 2 bytes.
    //
    //   MASTER (broken) image, 8 bytes  (VERIFIED by running the CLI on this source):
    //     4E F9 00 00 DE AD BE EF
    //     ^^^^^ ^^^^^ ^^^^^ ^^^^^
    //     opcode  hi    <-- data's DE AD clobbered the jmp's lo operand 80 00 -->
    //
    //   CORRECT image, 10 bytes (what this test asserts; a later task's placement
    //   fix makes it GREEN — the grown code must be accounted for so `data` follows
    //   at LMA 6, not LMA 4):
    //     4E F9 00 00 80 00 DE AD BE EF
    //     \___ jmp p abs.l (=$00008000) __/ \___ data Tail ___/
    //
    // Program-path: spawn the CLI binary, single positional file arg + `-o`, NO
    // --root (mirrors module_resolution.rs's spawn style).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let src = "module m\n\
        section code (vma: $8000) {\n\
        \x20   proc p (a0: *u8) {\n\
        \x20       jmp p\n\
        \x20   }\n\
        }\n\
        section data {\n\
        \x20   data Tail: [u8; 4] = [$DE, $AD, $BE, $EF]\n\
        }\n";
    let emp = root.join("m.emp");
    std::fs::write(&emp, src).unwrap();
    let out = root.join("out.bin");
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            emp.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "single-file two-section compile should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(
        bytes,
        vec![0x4E, 0xF9, 0x00, 0x00, 0x80, 0x00, 0xDE, 0xAD, 0xBE, 0xEF],
        "grown `jmp p` (abs.l = $00008000) must not be overwritten by the `data` \
         section: `data` must follow the FINAL 6-byte code image at LMA 6, not the \
         baked 4-byte baseline"
    );
}
