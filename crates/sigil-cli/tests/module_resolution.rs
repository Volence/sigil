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
    // The happy path emits a deterministic image: one `rts` (Draw_Sprite) + one
    // `jmp Draw_Sprite`. Pin the exact length so a mis-linked cross-module fixup
    // (which would change the emitted width/bytes) can't pass silently.
    assert!(out.exists());
    assert_eq!(
        std::fs::metadata(&out).unwrap().len(),
        4,
        "expected a 4-byte image"
    );
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
    assert!(out.exists() && std::fs::metadata(&out).unwrap().len() > 0);
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
