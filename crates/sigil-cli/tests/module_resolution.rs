use std::process::Command;

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

#[test]
fn two_modules_cross_reference_and_link() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "engine/helpers.emp",
        "module engine.helpers\npub proc Draw_Sprite (a0: *u8) {\n    rts\n}\n",
    );
    write(
        root,
        "badniks/plant.emp",
        "module badniks.plant\nuse engine.helpers.{Draw_Sprite}\nproc init (a0: *u8) {\n    jmp Draw_Sprite\n}\n",
    );
    let out = root.join("out.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("badniks/plant.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "multi-module compile should succeed");
    // No `--map` → sections pack SEQUENTIALLY from 0 (BUG I3 fix): the entry
    // (badniks.plant, discovered first) reserves its MAX span — a `jmp` is 6 bytes
    // at abs.l — so engine.helpers lands at LMA 6, NOT overlapping at 0. The jmp
    // then relaxes to abs.w (target 6 ≤ 0x7FFF, 4 bytes), leaving a 2-byte gap.
    //   plant  @ 0: jmp Draw_Sprite abs.w = 4E F8 00 06   (target = helpers @ 6)
    //   gap    @ 4: 00 00                                  (relax short of the 6-span)
    //   helper @ 6: rts = 4E 75
    assert!(out.exists());
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(bytes.len(), 8, "sequential pack: plant span 6 + helpers span 2");
    assert_eq!(
        &bytes[0..4],
        &[0x4E, 0xF8, 0x00, 0x06],
        "jmp Draw_Sprite → helpers @ LMA 6 (distinct, non-overlapping)"
    );
    assert_eq!(&bytes[6..8], &[0x4E, 0x75], "engine.helpers rts at LMA 6");
}

#[test]
fn transitive_chain_discovers_third_module() {
    // A `use`s B, B `use`s C. A branches to a name imported from B; B branches to
    // a name imported from C. C is only reachable THROUGH B — this proves the
    // `reachable_modules` BFS discovers it transitively (two 2-module tests never
    // exercise transitivity).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "chain/c.emp",
        "module chain.c\npub proc c_fn (a0: *u8) {\n    rts\n}\n",
    );
    write(
        root,
        "chain/b.emp",
        "module chain.b\nuse chain.c.{c_fn}\npub proc b_fn (a0: *u8) {\n    jmp c_fn\n}\n",
    );
    write(
        root,
        "chain/a.emp",
        "module chain.a\nuse chain.b.{b_fn}\nproc init (a0: *u8) {\n    jmp b_fn\n}\n",
    );
    let out = root.join("out.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("chain/a.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "transitive 3-module compile should succeed"
    );
    // No `--map` → sequential packing in BFS discovery order a, b, c. Each of a/b
    // reserves a 6-byte jmp span; c reserves 2 (rts):
    //   a @ 0:  jmp b_fn  → b_fn @ 6  → abs.w 4E F8 00 06
    //   b @ 6:  jmp c_fn  → c_fn @ 12 → abs.w 4E F8 00 0C
    //   c @ 12: rts = 4E 75
    // Both jmps relax to abs.w (4 bytes) inside their 6-byte spans → 2-byte gaps.
    assert!(out.exists());
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(bytes.len(), 14, "sequential pack: 6 + 6 + 2");
    assert_eq!(&bytes[0..4], &[0x4E, 0xF8, 0x00, 0x06], "a: jmp b_fn @ 6");
    assert_eq!(&bytes[6..10], &[0x4E, 0xF8, 0x00, 0x0C], "b: jmp c_fn @ 12");
    assert_eq!(&bytes[12..14], &[0x4E, 0x75], "c: rts @ 12");
}

#[test]
fn unknown_module_id_reports_diagnostic() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "badniks/plant.emp",
        "module badniks.plant\nuse missing.mod.{Foo}\nproc init (a0: *u8) {\n    rts\n}\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("badniks/plant.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no module `missing.mod` found under the scan root"),
        "stderr was: {stderr}"
    );
}

#[test]
fn prelude_types_resolve_without_use() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // Prelude exports a struct type used by the object module with NO `use`.
    write(
        root,
        "prelude.emp",
        "module prelude\npub struct ObjDef (size: 4) { code: *u8 }\n",
    );
    write(
        root,
        "badniks/plant.emp",
        "module badniks.plant\nproc init (a0: *u8) {\n    rts\n}\n\
         pub data Def = ObjDef{ code: \"init\" }\n",
    );
    let out = root.join("out.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("badniks/plant.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--prelude",
            "prelude",
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "prelude struct should resolve without an explicit use"
    );
    assert!(std::fs::metadata(&out).unwrap().len() >= 4); // Def = one *u8 pointer (fixup to init)
}

#[test]
fn prelude_absent_fails_with_unknown_type() {
    // Negative twin of `prelude_types_resolve_without_use`: the SAME module,
    // compiled WITHOUT `--prelude`, must fail because `ObjDef` is not visible.
    // This proves the prelude is load-bearing (not that the type resolves anyway).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "prelude.emp",
        "module prelude\npub struct ObjDef (size: 4) { code: *u8 }\n",
    );
    write(
        root,
        "badniks/plant.emp",
        "module badniks.plant\nproc init (a0: *u8) {\n    rts\n}\n\
         pub data Def = ObjDef{ code: \"init\" }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("badniks/plant.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            // no `--prelude` → ObjDef is not auto-imported.
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "compile must fail without the prelude"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ObjDef"),
        "stderr should name the unknown type ObjDef, was: {stderr}"
    );
}

#[test]
fn use_imported_type_resolves_without_prelude() {
    // Exercises the `use`-path ambient branch (UseNames::List): a `use`d module's
    // pub struct TYPE must be injected so the importing module's `data` literal
    // resolves it — with NO prelude involved. Prior cross-module tests only import
    // procs (labels), which the comptime filter drops, so this branch was untested.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "b.emp",
        "module b\npub struct Foo (size: 4) { p: *u8 }\n",
    );
    write(
        root,
        "a.emp",
        "module a\nuse b.{Foo}\nproc lbl (a0: *u8) {\n    rts\n}\n\
         pub data D = Foo{ p: \"lbl\" }\n",
    );
    let out = root.join("out.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("a.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "a `use`d struct type should resolve without a prelude"
    );
    assert!(std::fs::metadata(&out).unwrap().len() >= 4); // D = one *u8 pointer (fixup to lbl)
}

#[test]
fn missing_use_reports_add_use_fixit() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "engine/helpers.emp",
        "module engine.helpers\npub proc Draw_Sprite (a0: *u8) {\n    rts\n}\n",
    );
    write(
        root,
        "badniks/plant.emp",
        "module badniks.plant\nproc init (a0: *u8) {\n    jmp Draw_Sprite\n}\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("badniks/plant.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("add `use engine.helpers.{Draw_Sprite}`"),
        "stderr was: {stderr}"
    );
}

#[test]
fn module_lands_in_named_section_and_budget_overflow_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("sigil.map.toml"),
        "fill = 0x00\n\n[[region]]\nname = \"obj_bank\"\nlma_base = 0x10000\nsize = 4\nkind = \"rom\"\n",
    )
    .unwrap();
    // 8 bytes into a 4-byte region → overflow.
    write(
        root,
        "big.emp",
        "module big in obj_bank\npub data Blob: [u8; 8] = [1,2,3,4,5,6,7,8]\n",
    );
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("big.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--map",
            root.join("sigil.map.toml").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("over by") && err.contains("obj_bank"),
        "stderr: {err}"
    );
}

#[test]
fn module_placed_in_region_builds_when_it_fits() {
    // Same shape but the region is large enough (0x10 bytes ≥ 8) → success, and a
    // non-empty binary is written.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("sigil.map.toml"),
        "fill = 0x00\n\n[[region]]\nname = \"obj_bank\"\nlma_base = 0x10000\nsize = 0x10\nkind = \"rom\"\n",
    )
    .unwrap();
    write(
        root,
        "fits.emp",
        "module fits in obj_bank\npub data Blob: [u8; 8] = [1,2,3,4,5,6,7,8]\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("fits.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--map",
            root.join("sigil.map.toml").to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected build success, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The region places the 8 bytes at LMA 0x10000, so the ROM runs 0x10000..0x10008.
    let bytes = std::fs::read(&outbin).unwrap();
    assert_eq!(bytes.len(), 0x10008, "section placed at its region LMA base");
    assert_eq!(&bytes[0x10000..0x10008], &[1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn two_modules_same_region_pack_cumulatively() {
    // The core Task 4 behavior: two modules both `in obj_bank`, each with a small
    // `pub data` block, must pack SEQUENTIALLY within the one region — B lands at
    // `lma_base + A.len`, not overlapping A. Entry `a` `use`s a const from `b` so
    // BFS reaches both (entry is discovered first, so its section packs first).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("sigil.map.toml"),
        "fill = 0x00\n\n[[region]]\nname = \"obj_bank\"\nlma_base = 0x10000\nsize = 0x10\nkind = \"rom\"\n",
    )
    .unwrap();
    // b: a 3-byte block + a pub const the entry references (forces reachability).
    write(
        root,
        "b.emp",
        "module b in obj_bank\npub const Marker: u8 = $EE\npub data BlobB: [u8; 3] = [$AA, $BB, $CC]\n",
    );
    // a (entry): a 4-byte block whose first byte is b's const → both reachable.
    write(
        root,
        "a.emp",
        "module a in obj_bank\nuse b.{Marker}\npub data BlobA: [u8; 4] = [Marker, 1, 2, 3]\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("a.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--map",
            root.join("sigil.map.toml").to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected build success, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&outbin).unwrap();
    // A (entry, discovered first) packs at the region base; B follows at base + 4.
    assert_eq!(&bytes[0x10000..0x10004], &[0xEE, 1, 2, 3], "A at region base");
    assert_eq!(
        &bytes[0x10004..0x10007],
        &[0xAA, 0xBB, 0xCC],
        "B packed sequentially after A (no overlap)"
    );
    // ROM ends right after B — no padding, so total length pins the packing.
    assert_eq!(bytes.len(), 0x10007, "cumulative pack: base + 4 + 3");
}

#[test]
fn section_with_no_matching_region_errors() {
    // A module placed `in <name>` whose name matches NO region in the map is a hard
    // error (`resolve::place_sections`), naming the miss clearly.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("sigil.map.toml"),
        "fill = 0x00\n\n[[region]]\nname = \"obj_bank\"\nlma_base = 0x10000\nsize = 0x10\nkind = \"rom\"\n",
    )
    .unwrap();
    write(
        root,
        "orphan.emp",
        "module orphan in nowhere_region\npub data X: [u8; 2] = [1, 2]\n",
    );
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("orphan.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--map",
            root.join("sigil.map.toml").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("has no region in the map") && err.contains("nowhere_region"),
        "stderr: {err}"
    );
}

#[test]
fn cross_module_offsets_table_bytes_are_exact() {
    // DELIVERABLE 1 (discharges the §4.7 cross-module-target deferral): an
    // `offsets` table in the ENTRY module points at `pub data` targets that live
    // in ANOTHER module. Prove the emitted `dc.w target - base` words are
    // byte-exact after cross-module linking.
    //
    // Both modules land `in data`, so `--map` fixes every address. Packing is in
    // module DISCOVERY order (entry first, then BFS-reached deps), and items pack
    // in declaration order within a module.
    //
    //   LAYOUT ARITHMETIC (computed INDEPENDENTLY of read-back):
    //     region `data` base = 0x20000  (from the map below)
    //     entry `tab` packs first:  T  (offsets, 2 members * 2 bytes = 4 bytes)
    //       T  @ base + 0            (0x20000 .. 0x20004)
    //     `targets` (reached via `use targets`) packs next, decl order A then B:
    //       A  @ base + 4            (0x20004, 1 byte = $AA)
    //       B  @ base + 5            (0x20005, 1 byte = $BB)
    //     offsets words are `dc.w target - T`, big-endian:
    //       First  = addr(A) - addr(T) = (base+4) - (base+0) = 4  -> 0x00 0x04
    //       Second = addr(B) - addr(T) = (base+5) - (base+0) = 5  -> 0x00 0x05
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("sigil.map.toml"),
        "fill = 0x00\n\n[[region]]\nname = \"data\"\nlma_base = 0x20000\nsize = 0x20\nkind = \"rom\"\n",
    )
    .unwrap();
    write(
        root,
        "targets.emp",
        "module targets in data\npub data A: [u8;1] = [$AA]\npub data B: [u8;1] = [$BB]\n",
    );
    write(
        root,
        "tab.emp",
        "module tab in data\nuse targets.{A, B}\npub offsets T {\n    First:  A,\n    Second: B,\n}\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("tab.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--map",
            root.join("sigil.map.toml").to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected build success, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&outbin).unwrap();
    // Independently computed expected words (see LAYOUT ARITHMETIC above).
    let base = 0x20000usize;
    let addr_t = base; // 0x20000
    let addr_a = base + 4; // 0x20004
    let addr_b = base + 5; // 0x20005
    let first = (addr_a - addr_t) as u16; // 4
    let second = (addr_b - addr_t) as u16; // 5
    assert_eq!(
        &bytes[addr_t..addr_t + 4],
        &[
            (first >> 8) as u8,
            (first & 0xFF) as u8,
            (second >> 8) as u8,
            (second & 0xFF) as u8,
        ],
        "offsets words: dc.w (A - T), (B - T) big-endian"
    );
    // Pin the target bytes too, so a mis-packed layout can't accidentally satisfy
    // the offset math.
    assert_eq!(bytes[addr_a], 0xAA, "A target byte");
    assert_eq!(bytes[addr_b], 0xBB, "B target byte");
}

#[test]
fn three_module_corpus_compiles_end_to_end() {
    // DELIVERABLE 2: the headline #4 mechanisms compose in one image.
    //   prelude.emp  -> `pub struct ObjDef` auto-imported everywhere (no `use`)
    //   art.emp      -> a shared `pub data` label, referenced cross-module
    //   obj.emp      -> ENTRY: `use art.{Map_Thing}`, a `proc`, and a struct-
    //                   literal `data` whose two pointer fields fix up across
    //                   modules (one to a local proc label, one to art's label).
    // Proves: prelude type auto-import + cross-module `use` of a data label +
    // a struct-literal with two cross-module pointer fixups + a proc, all linked
    // into one non-empty binary.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "prelude.emp",
        "module prelude\npub struct ObjDef (size: 8) { code: *u8, map: *u8 }\n",
    );
    write(
        root,
        "art.emp",
        "module art\npub data Map_Thing: [u8; 2] = [$12, $34]\n",
    );
    write(
        root,
        "obj.emp",
        "module obj\nuse art.{Map_Thing}\nproc init (a0: *u8) {\n    rts\n}\n\
         pub data Def = ObjDef{ code: \"init\", map: \"Map_Thing\" }\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("obj.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--prelude",
            "prelude",
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected 3-module build success, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        std::fs::metadata(&outbin).unwrap().len() > 0,
        "expected a non-empty binary"
    );
}

#[test]
fn two_modules_same_proc_local_label_do_not_collide() {
    // CROSS-MODULE PRIVATE-LABEL COLLISION (Plan 7 #4): two DIFFERENT modules each
    // define `pub proc init` containing a non-export `.loop:`. Proc-local labels
    // are owner-scoped as `$init$loop` — but the owner (proc name) is only unique
    // WITHIN a module. Absent module-qualification, both modules mint the SAME
    // `$init$loop`, so the flat linker symbol table sees `$init$loop` redefined.
    // Module-qualifying the hygiene local symbol (`$<modid>$init$loop`) makes the
    // two private labels distinct. The entry `use`s a pub const from each module
    // so BFS reaches both (and thus lowers both `init` procs).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "a.emp",
        "module a\npub const Hook_A: u8 = $AA\n\
         pub proc init (a0: *u8) {\n    nop\n.loop:\n    bra.w .loop\n}\n",
    );
    write(
        root,
        "b.emp",
        "module b\npub const Hook_B: u8 = $BB\n\
         pub proc init (a0: *u8) {\n    nop\n.loop:\n    bra.w .loop\n}\n",
    );
    write(
        root,
        "entry.emp",
        "module entry\nuse a.{Hook_A}\nuse b.{Hook_B}\n\
         pub data Refs: [u8;2] = [Hook_A, Hook_B]\n",
    );
    let out = root.join("out.bin");
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("entry.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "two modules with an identically-named private proc label must link, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out.exists());
}

#[test]
fn two_modules_asm_splice_local_label_do_not_collide() {
    // The COUNTER-RESET twin: two modules each splice a comptime-generated `asm {}`
    // (owning a non-export `.wait:`) inside a proc. The `asm {}` instantiation
    // counter `k` restarts at 0 per `lower_module` call, so both modules mint the
    // SAME `$asm0$wait`. Distinct proc names (`go_a`/`go_b`) isolate this to the
    // counter collision (not the proc-name collision above). Module-qualifying the
    // asm local symbol (`$<modid>$asm{k}$wait`) resolves it.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "a.emp",
        "module a\npub const Tag_A: u8 = $AA\n\
         comptime fn spin() -> Code {\n    return asm {\n.wait:\n    bra.w .wait\n    }\n}\n\
         pub proc go_a (a0: *u8) {\n    spin()\n    rts\n}\n",
    );
    write(
        root,
        "b.emp",
        "module b\npub const Tag_B: u8 = $BB\n\
         comptime fn spin() -> Code {\n    return asm {\n.wait:\n    bra.w .wait\n    }\n}\n\
         pub proc go_b (a0: *u8) {\n    spin()\n    rts\n}\n",
    );
    write(
        root,
        "entry.emp",
        "module entry\nuse a.{Tag_A}\nuse b.{Tag_B}\n\
         pub data Refs: [u8;2] = [Tag_A, Tag_B]\n",
    );
    let out = root.join("out.bin");
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("entry.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "two modules splicing an asm{{}} with the same local label must link, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out.exists());
}

#[test]
fn map_placement_on_code_module_does_not_panic() {
    // BUG C1: a `--map` build whose module contains ANY `jmp`/`jsr`-to-symbol used
    // to PANIC — `place_sections` advanced its cursor by `vma_len()`, which hits
    // `unreachable!` on the still-unlowered `JmpJsrSym` fragment (placement runs
    // BEFORE `resolve_layout`). With the panic-safe `placement_span`, placement
    // succeeds and the section lands at its region base.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("sigil.map.toml"),
        "fill = 0x00\n\n[[region]]\nname = \"code\"\nlma_base = 0x100\nsize = 0x10\nkind = \"rom\"\n",
    )
    .unwrap();
    write(
        root,
        "solo.emp",
        "module solo in code\npub proc p (a0: *u8) {\n    jmp p\n}\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("solo.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--map",
            root.join("sigil.map.toml").to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "code module with `--map` must not panic, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&outbin).unwrap();
    // Section placed at region base 0x100. `p` @ 0x100 → jmp abs.w (0x100 ≤
    // 0x7FFF) = 4E F8 01 00.
    assert!(bytes.len() >= 0x104, "image reaches the region base at 0x100");
    assert_eq!(
        &bytes[0x100..0x104],
        &[0x4E, 0xF8, 0x01, 0x00],
        "jmp p at region base 0x100 (abs.w to 0x100)"
    );
}

#[test]
fn no_map_multi_module_places_at_distinct_lmas() {
    // BUG I3: without `--map` no placement happened, so every module's section kept
    // `lma == 0` and silently OVERLAPPED at the origin. `place_sequential` now packs
    // them contiguously, so the second module's label resolves to a DISTINCT,
    // non-overlapping address (not 0). Entry `main` `jmp`s a pub proc from `helper`.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "helper.emp",
        "module helper\npub proc target (a0: *u8) {\n    rts\n}\n",
    );
    write(
        root,
        "main.emp",
        "module main\nuse helper.{target}\nproc init (a0: *u8) {\n    jmp target\n}\n",
    );
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
    assert!(
        out.status.success(),
        "no-map multi-module compile must succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&outbin).unwrap();
    // main (entry, discovered first) reserves a 6-byte jmp span at LMA 0; helper
    // lands at LMA 6 — NOT overlapping at 0. `target` therefore resolves to 6, and
    // the jmp relaxes to abs.w within its span (2-byte gap at bytes 4..6).
    assert_eq!(bytes.len(), 8, "sequential pack: main span 6 + helper span 2");
    assert_eq!(
        &bytes[0..4],
        &[0x4E, 0xF8, 0x00, 0x06],
        "jmp target → helper @ LMA 6 (proves I3 fixed: not overlapping at 0)"
    );
    assert_eq!(&bytes[6..8], &[0x4E, 0x75], "helper `target` rts at LMA 6");
}

#[test]
fn section_nested_items_resolve_under_root() {
    // AUDIT FIX (Task 0.5): `exported_names`/`defined_names` iterated ONLY
    // top-level `file.items` — no recursion into `section {}` bodies — so any
    // data/proc/offsets nested in a section never entered the rename map and
    // `report_unresolved` rejected references to it (including the offsets table's
    // OWN base label) as `unknown symbol`. Single-file mode worked; `--root`
    // failed. Here an offsets table + its two data targets are ALL section-nested;
    // `--root` must now produce the SAME bytes single-file mode gives:
    //   00 04 00 05 AA BB  (dc.w A-T, B-T then the two target bytes).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "m.emp",
        "module m\nsection s (vma: $0) {\n\
         offsets T {\n    A: X,\n    B: Y,\n}\n\
         data X: [u8;1] = [$AA]\n\
         data Y: [u8;1] = [$BB]\n}\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("m.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "section-nested offsets/data must resolve under --root, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&outbin).unwrap();
    assert_eq!(
        bytes,
        vec![0x00, 0x04, 0x00, 0x05, 0xAA, 0xBB],
        "section-nested table must byte-match single-file mode"
    );
}

#[test]
fn section_nested_proc_cross_reference_resolves_under_root() {
    // The proc twin of the audit repro: `proc go { jmp Helper }` and its sibling
    // `proc Helper` are BOTH section-nested. Single-file gives `4E F8 00 04 4E 75`
    // (jmp relaxes to abs.w → Helper @ 4). `--root` must match.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "p.emp",
        "module p\nsection s (vma: $0) {\n\
         proc go (a0: *u8) {\n    jmp Helper\n}\n\
         proc Helper (a0: *u8) {\n    rts\n}\n}\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("p.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "section-nested proc cross-reference must resolve under --root, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&outbin).unwrap();
    assert_eq!(
        bytes,
        vec![0x4E, 0xF8, 0x00, 0x04, 0x4E, 0x75],
        "section-nested proc jmp must byte-match single-file mode"
    );
}

#[test]
fn cross_module_offsets_target_section_nested_bytes_are_exact() {
    // The §4.7 cross-module-target deferral, but with BOTH the offsets table and
    // its cross-module targets section-nested (the top-level variant is covered by
    // `cross_module_offsets_table_bytes_are_exact`). The `data` region packs
    // sections cumulatively in module-discovery order: entry `tab`'s `data` section
    // (offsets T, 4 bytes) at LMA base 0x20000, then `targets`'s `data` section at
    // LMA 0x20004. Each section's explicit VMA is set to its resulting LMA so labels
    // resolve distinctly (A @ 0x20004, B @ 0x20005). Words: dc.w (A-T)=4, (B-T)=5.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("sigil.map.toml"),
        "fill = 0x00\n\n[[region]]\nname = \"data\"\nlma_base = 0x20000\nsize = 0x20\nkind = \"rom\"\n",
    )
    .unwrap();
    write(
        root,
        "targets.emp",
        "module targets in data\nsection data (vma: $20004) {\n\
         pub data A: [u8;1] = [$AA]\n\
         pub data B: [u8;1] = [$BB]\n}\n",
    );
    write(
        root,
        "tab.emp",
        "module tab in data\nuse targets.{A, B}\nsection data (vma: $20000) {\n\
         pub offsets T {\n    First:  A,\n    Second: B,\n}\n}\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("tab.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--map",
            root.join("sigil.map.toml").to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "cross-module section-nested offsets target must resolve, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&outbin).unwrap();
    let base = 0x20000usize;
    assert_eq!(
        &bytes[base..base + 4],
        &[0x00, 0x04, 0x00, 0x05],
        "offsets words: dc.w (A - T), (B - T) big-endian"
    );
    assert_eq!(bytes[base + 4], 0xAA, "A target byte");
    assert_eq!(bytes[base + 5], 0xBB, "B target byte");
}

#[test]
fn exported_dotted_label_resolves_under_root() {
    // AUDIT FIX (Task 0.6): an exported proc label (`export .entry:` → emitted as
    // dotted `foo.entry`) is neither a `$`-hygiene local nor a rename-map key, so
    // `report_unresolved` rejected ANY `--root` reference to it as
    // `unknown symbol foo.entry`. Single-file gives `60 00 FF FE` (bra.w to self);
    // `--root` must now match after teaching the rename pass dotted symbols.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "f.emp",
        "module f\nproc foo (a0: *u8) {\nexport .entry:\n    bra.w foo.entry\n}\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("f.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "exported dotted label must resolve under --root, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&outbin).unwrap();
    assert_eq!(
        bytes,
        vec![0x60, 0x00, 0xFF, 0xFE],
        "bra.w foo.entry to self must byte-match single-file mode"
    );
}

#[test]
fn two_modules_same_exported_dotted_label_do_not_collide() {
    // Latent finding #2: an exported `.entry:` is emitted `Owner.name` (`foo.entry`),
    // NOT module-qualified. Two modules each with a private `proc foo` exporting
    // `.entry:` therefore both mint `foo.entry` → duplicate symbol in the flat link
    // table. Module-qualifying the dotted label on BOTH def and ref sides
    // (`a.foo.entry`, `b.foo.entry`) makes them distinct. Entry `use`s each module's
    // pub wrapper so BFS lowers both `foo` procs.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "a.emp",
        "module a\nproc foo (a0: *u8) {\nexport .entry:\n    rts\n}\n\
         pub proc wrap_a (a0: *u8) {\n    jmp foo.entry\n}\n",
    );
    write(
        root,
        "b.emp",
        "module b\nproc foo (a0: *u8) {\nexport .entry:\n    rts\n}\n\
         pub proc wrap_b (a0: *u8) {\n    jmp foo.entry\n}\n",
    );
    write(
        root,
        "entry.emp",
        "module entry\nuse a.{wrap_a}\nuse b.{wrap_b}\n\
         proc init (a0: *u8) {\n    jsr wrap_a\n    jmp wrap_b\n}\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("entry.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "two modules exporting the same dotted label must link, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(outbin.exists());
}

#[test]
fn cross_module_exported_dotted_label_reference_links() {
    // The importing side of the dotted-label fix: `use a.{foo}` + `jmp foo.entry`
    // must resolve the exported label across modules. The importer has `foo` in its
    // rename map (→ `a.foo`), so `foo.entry` module-qualifies to `a.foo.entry`,
    // matching a's own label. Byte-check the resolved jmp target.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "a.emp",
        "module a\npub proc foo (a0: *u8) {\n    nop\nexport .entry:\n    rts\n}\n",
    );
    write(
        root,
        "b.emp",
        "module b\nuse a.{foo}\nproc init (a0: *u8) {\n    jmp foo.entry\n}\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("b.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "cross-module exported dotted label must link, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&outbin).unwrap();
    // b (entry) discovered first reserves a 6-byte jmp span at LMA 0; a's `foo`
    // lands at LMA 6 with `nop` (4E 71) then `.entry` at LMA 8. `jmp foo.entry`
    // relaxes to abs.w (target 8 ≤ 0x7FFF) → 4E F8 00 08.
    assert_eq!(
        &bytes[0..4],
        &[0x4E, 0xF8, 0x00, 0x08],
        "jmp foo.entry → a.foo.entry @ LMA 8"
    );
}

#[test]
fn item_guard_sees_prelude_const_across_modules() {
    // Plan 7 #5: an item-position guard in the game module references a `pub const`
    // auto-imported from the prelude (no explicit `use`). The prelude's comptime
    // defs are prepended as ambient items before lowering, so the guard resolves
    // `MAX_OBJS` and passes. Build succeeds and the data byte is written.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "prelude.emp", "module prelude\npub const MAX_OBJS = 32\n");
    write(
        root,
        "game.emp",
        "module game\nensure(MAX_OBJS % 8 == 0, \"objs {MAX_OBJS}\")\n\
         pub data D: [u8;1] = [$42]\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("game.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--prelude",
            "prelude",
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "cross-module prelude-const guard must compile, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&outbin).unwrap();
    assert_eq!(bytes, vec![0x42], "guard is zero-byte; only the data byte lands");
}

#[test]
fn whole_module_use_warns_it_imports_nothing() {
    // M5: `use other` (whole-module, no `.{…}`/`.*`) binds no names, so it is a
    // silent no-op today. Emit a warning at the `use` decl so a later `other.Name`
    // reference isn't mysterious. The module still compiles (warning, not error).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "other.emp", "module other\npub data X: [u8;1] = [$22]\n");
    write(
        root,
        "entry.emp",
        "module entry\nuse other\npub data D: [u8;1] = [$11]\n",
    );
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("entry.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "whole-module use is a warning, not an error, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("whole-module `use other`") && stderr.contains("imports no names"),
        "expected a whole-module-use warning, stderr: {stderr}"
    );
}
