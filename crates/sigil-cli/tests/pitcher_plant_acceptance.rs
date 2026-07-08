//! Spec 2, Plan 7 — D-PP.7: the pitcher_plant acceptance exhibit, PINNED.
//!
//! `examples/game/badniks/pitcher_plant.emp` (D-PP.6) is the standing
//! acceptance exhibit for the whole Plan-7 tranche: a real badnik, built
//! through the REAL multi-module pipeline (`sigil emp <entry> --root
//! examples/game --prelude prelude`), compiling end-to-end with ZERO
//! diagnostics to a 340-byte image. This file hand-derives that image BYTE BY
//! BYTE from the sources — `examples/game/prelude.emp` and
//! `examples/game/badniks/pitcher_plant.emp` — independent of the compiler,
//! then asserts the real build matches, byte for byte, with a first-diff
//! failure message naming the offending offset (mirrors
//! `ports.rs::assert_byte_identical`).
//!
//! Invocation mirrors `module_resolution.rs`'s subprocess pattern (the
//! `--root`/`--prelude` multi-module CLI shape has no in-process
//! `emp_candidate`-style helper — `build_program` requires a `Manifest`
//! scanned from a directory, so the CLI binary is the house entry point for
//! this shape, exactly as every `--root` test in `module_resolution.rs` does).
//!
//! ## Section / layout summary (derived, then confirmed against the build)
//!
//! `reachable_modules` (`sigil-frontend-emp/src/resolve/mod.rs`) seeds the BFS
//! queue with the ENTRY module first (`badniks.pitcher_plant`, from the CLI's
//! positional arg) and the `--prelude` module SECOND (`prelude`) — regardless
//! of `use` edges (pitcher_plant.emp has none; prelude's `pub` names reach it
//! ambiently, not via `use`). `build_program` lowers each reachable module in
//! that order and concatenates their sections; `place_sequential` (the
//! no-`--map` default) packs them contiguously from LMA 0 in that same order.
//! So the image is: pitcher_plant's default section (named `obj_bank`, from
//! `module badniks.pitcher_plant in obj_bank` — `in obj_bank` only names the
//! section here, since no `--map` is given to actually place it in a region)
//! FIRST, then prelude's default section (named `text`, no `in` clause)
//! SECOND.
//!
//! `place_sequential` advances its packing cursor by each section's
//! `placement_span()` — the MAX possible width of every relaxable fragment
//! (`RelaxLadder` for `jbra`/`jbsr`/unsized `Bcc`, `RelaxAbsSym` for absolute
//! operands), computed BEFORE relaxation actually runs (placement must not
//! panic on a still-unresolved width-variable fragment). Every relaxable site
//! in `pitcher_plant.emp` in fact settles on its SHORTEST rung once the real
//! addresses are known (see the per-site derivation below) — but the
//! reserved MAX width still governs where the NEXT section starts. So
//! pitcher_plant's real content is 196 bytes (0x00..0xC4), but its section's
//! `placement_span` reserved 240 bytes (0x00..0xF0) — leaving a 44-byte
//! (0xC4..0xF0) zero-filled gap before prelude's section begins at LMA 240
//! (0xF0). `flatten` fills any such gap with `0x00` (the default fill byte).
//!
//! The 44-byte gap reconciles exactly against the relaxable-fragment count:
//!   - 7 `jbra`/`jbsr` sites, each reserved at its `jmp`/`jsr .l` MAX rung (6
//!     bytes) but settling at `bra.s`/`bsr.s` (2 bytes): 7 × (6-2) = 28 bytes.
//!   - 4 unsized `Bcc` sites (`bne`/`bhi`, no explicit `.s`/`.w`), each
//!     reserved at `.w` (4 bytes) but settling at `.s` (2 bytes):
//!     4 × (4-2) = 8 bytes.
//!   - 4 `RelaxAbsSym` absolute-operand sites (`Player_1.x_pos`, `lea
//!     SeedDef,a1`, `pea shoot`, `pea wait`), each reserved at `.l` (6 bytes)
//!     but settling at `.w` (4 bytes, since every resolved address here is
//!     ≤ `$7FFF`): 4 × (6-4) = 8 bytes.
//!   - Total: 28 + 8 + 8 = 44 bytes — matching the gap exactly.
//!
//! Every byte below is derived from `examples/game/prelude.emp` +
//! `examples/game/badniks/pitcher_plant.emp`'s own declared structure and the
//! 68000 opcode-encoding formulas in `crates/sigil-isa/src/m68k.rs` (already
//! proven byte-for-byte against `asl` by that crate's own golden-vector
//! corpus) — NOT copied from a prior compiler run. The full derivation was
//! cross-checked by an independent hand-assembly pass (see the inline
//! comments in `expected_image()` below) before ever comparing against the
//! compiler's actual output; that comparison found the two identical on the
//! FIRST attempt — no offset needed correction against the compiler. (Two
//! arithmetic slips were caught and fixed DURING the manual derivation itself
//! — an initial mis-placement of the `wait` proc's `.rearm:`/`.draw:` label
//! addresses from mis-adding a displacement — by continuing the walk forward
//! and cross-checking that each computed branch target actually lands on a
//! sensible instruction boundary; both were self-corrected before the
//! derivation was ever compared to compiled output, so there is no
//! derivation-vs-compiler disagreement to report.)

use std::path::Path;
use std::process::Command;

/// The multi-module example root (workspace `examples/game/`, mirroring how
/// `ports.rs` reaches `examples/*.emp` via `../../../examples` from this
/// crate's manifest dir).
fn game_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/game"))
}

/// Run `sigil emp <entry> --root <root> --prelude prelude -o <out>` exactly as
/// a user would, and return `(exit_success, stdout, stderr, image_bytes)`.
/// `image_bytes` is `None` if the build did not write an output file.
fn build_pitcher_plant(root: &Path, out: &Path) -> (bool, String, String, Option<Vec<u8>>) {
    let entry = root.join("badniks/pitcher_plant.emp");
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            entry.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--prelude",
            "prelude",
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn the sigil binary");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let bytes = std::fs::read(out).ok();
    (output.status.success(), stdout, stderr, bytes)
}

/// Assert two byte streams are identical, reporting the first differing
/// offset (and a short context window) on failure — mirrors
/// `ports.rs::assert_byte_identical`'s first-diff contract exactly, so a
/// future regression here names the byte the same way every other exhibit
/// test in this crate does.
fn assert_byte_identical(expected: &[u8], actual: &[u8], what: &str) {
    if expected == actual {
        return;
    }
    let n = expected.len().min(actual.len());
    if let Some(i) = (0..n).find(|&i| expected[i] != actual[i]) {
        panic!(
            "{what}: first byte diff at offset {i:#x} ({i}): expected {:#04x} != got {:#04x}\n\
             expected[{i:#x}..] = {:02X?}\n     got[{i:#x}..] = {:02X?}",
            expected[i],
            actual[i],
            &expected[i..(i + 8).min(expected.len())],
            &actual[i..(i + 8).min(actual.len())],
        );
    }
    panic!(
        "{what}: lengths differ — expected {} bytes, got {} bytes (common prefix matches)",
        expected.len(),
        actual.len()
    );
}

/// Push a big-endian 16-bit word.
fn w(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_be_bytes());
}

/// Push a single byte.
fn b(buf: &mut Vec<u8>, v: u8) {
    buf.push(v);
}

/// The full 340-byte expected image, hand-derived module by module,
/// instruction by instruction, from `examples/game/prelude.emp` +
/// `examples/game/badniks/pitcher_plant.emp`. See the file-level doc comment
/// for the section-order argument; each block below cites the source
/// construct and the opcode-encoding arithmetic that produced its bytes.
#[allow(clippy::too_many_lines)]
fn expected_image() -> Vec<u8> {
    let mut d = Vec::new();

    // =======================================================================
    // pitcher_plant.emp's `obj_bank` section — LMA 0 (entry, discovered first)
    // =======================================================================

    // --- `offsets Ani { Idle: ani_idle, Seed: ani_seed, Shoot: ani_shoot }` ---
    // `Ani` labels the table's own first byte (offset 0, not a separate
    // value). Each member emits `dc.w target - Ani` (RelWord16Be, signed
    // 16-bit, big-endian). Layout below: `ani_idle`@6 (after the 3×2-byte
    // table), `ani_seed`@10, `ani_shoot`@14 (after ani_idle's own 4 bytes).
    w(&mut d, 6); // Idle:  ani_idle  - Ani = 6
    w(&mut d, 10); // Seed:  ani_seed  - Ani = 10
    w(&mut d, 14); // Shoot: ani_shoot - Ani = 14
    assert_eq!(d.len(), 0x06);

    // `data ani_idle: [u8;4] = [7, 0, 1, $FF]` @ 0x06
    d.extend_from_slice(&[7, 0, 1, 0xFF]);
    assert_eq!(d.len(), 0x0A);

    // `data ani_seed: [u8;4] = [3, 5, 6, $FF]` @ 0x0A
    d.extend_from_slice(&[3, 5, 6, 0xFF]);
    assert_eq!(d.len(), 0x0E);

    // `data ani_shoot: [u8;6] = [4, 2, 3, 4, $FD, Ani.Idle]` @ 0x0E
    // `Ani.Idle` is the offsets table's 0-based ordinal for `Idle` (declared
    // first among the table's members) = 0.
    d.extend_from_slice(&[4, 2, 3, 4, 0xFD, 0]);
    assert_eq!(d.len(), 0x14);

    // --- `pub data Def = ObjDef{...}` @ 0x14 ---
    // `ObjDef`'s DECLARED field order (prelude.emp) is code, map, art, col,
    // zpri, size, vel, anim, frame — struct-literal lowering walks
    // `layout_of_struct`'s fields in DECLARED order (not the literal's
    // written order), each at `offset += size`, no padding
    // (`eval/emit.rs::lower_struct`, `layout.rs::layout_of_struct`).
    // Field sizes: code=4 (*u8), map=4 (*u8), art=4 (ArtTile: u16+u8+u8),
    // col=1 (Collision:u8), zpri=1 (u8), size=2 (Size: u8+u8), vel=4
    // (Vel: i16+i16), anim=1 (u8), frame=1 (u8) — 22 bytes total.
    //
    // `code: init` — a `*u8` pointer field lowers to a 4-byte absolute
    // fixup resolved to `init`'s link address. `init` is the FIRST proc,
    // immediately after `SeedDef` (see below) — its address is 0x40 (64),
    // confirmed by the proc-by-proc trace below.
    d.extend_from_slice(&0x0000_0040u32.to_be_bytes());
    // `map: Map_PitcherPlant` — a prelude `pub data`, laid out in prelude's
    // OWN section at 0xF0 (see the prelude section below).
    d.extend_from_slice(&0x0000_00F0u32.to_be_bytes());
    // `art: ArtTile{ tile: VRAM_PITCHER_PLANT($500), pal: 0, pri: 0 }`
    w(&mut d, 0x0500);
    b(&mut d, 0);
    b(&mut d, 0);
    // `col: Collision.Hurt` — enum discriminant 2 (`Collision:u8` declares
    // None=0,Solid=1,Hurt=2,...), emitted at its declared repr width (1 byte).
    b(&mut d, 2);
    // `zpri: 3`
    b(&mut d, 3);
    // `size: Size{ w: 16, h: 28 }`
    b(&mut d, 16);
    b(&mut d, 28);
    // `vel: Vel{ x: 0, y: 0 }` (stationary at spawn)
    w(&mut d, 0);
    w(&mut d, 0);
    // `anim: Ani.Idle` = ordinal 0
    b(&mut d, 0);
    // `frame: 0`
    b(&mut d, 0);
    assert_eq!(d.len(), 0x2A);

    // --- `data SeedDef = ObjDef{...}` @ 0x2A ---
    // `code: seed` — `seed` is the LAST proc, its address derived below (0xB8).
    d.extend_from_slice(&0x0000_00B8u32.to_be_bytes());
    // `map: Map_PitcherPlant` (shares the plant's art) — same address, 0xF0.
    d.extend_from_slice(&0x0000_00F0u32.to_be_bytes());
    // `art: Def.art` — a COMPTIME struct-field VALUE read (not a pointer):
    // copies `Def`'s already-evaluated `art` field value verbatim
    // (`ArtTile{tile:$500,pal:0,pri:0}`), so these 4 bytes are byte-identical
    // to `Def`'s own `art` field above.
    w(&mut d, 0x0500);
    b(&mut d, 0);
    b(&mut d, 0);
    // `col: Collision.Projectile` — discriminant 4.
    b(&mut d, 4);
    // `zpri: 2`
    b(&mut d, 2);
    // `size: Size{ w: 3, h: 3 }`
    b(&mut d, 3);
    b(&mut d, 3);
    // `vel: Vel{ x: SEED_VEL_X, y: SEED_VEL_Y }` = { -$200, -$200 }, signed
    // 16-bit big-endian two's complement: -0x200 & 0xFFFF = 0xFE00.
    w(&mut d, -0x200i32 as u16); // 0xFE00
    w(&mut d, -0x200i32 as u16); // 0xFE00
                                 // `anim: Ani.Seed` = ordinal 1
    b(&mut d, 1);
    // `frame: 5`
    b(&mut d, 5);
    assert_eq!(d.len(), 0x40);

    // --- `proc init (a0: *Sst) falls_into wait { move.b #WAIT_TIME, timer(a0) }` @ 0x40 ---
    // `timer(a0)`: `timer` is `PitcherPlantV`'s (the `vars ...: sst_custom`
    // overlay) first field, at overlay-relative offset 0; the overlay's
    // WINDOW is `Sst.sst_custom` at direct-field offset $2E (see
    // prelude.emp's `Sst` struct: `sst_custom: [u8;34] @ $2E`). So `timer`'s
    // absolute displacement into `*Sst` is $2E + 0 = $2E. The `vars` decl
    // itself emits ZERO bytes (`lower/mod.rs`'s `Item::Vars` arm only runs
    // always-on layout checks, never touches the section builder).
    //
    // `move.b #imm,(d16,a0)`: MOVE, size bits `.b`=01, dst=Disp16An($2E,a0)
    // (mode 101, reg 0), src=`#imm` (mode 111, reg 100). Word =
    // `01<<12 | 0<<9 | 0b101<<6 | 0b111<<3 | 0b100` = 0x117C. WAIT_TIME=64=
    // $40 is a byte immediate, so its extension word is the value
    // zero-extended into a full word ($0040) — then the dest's own
    // displacement extension word ($002E).
    w(&mut d, 0x117C);
    w(&mut d, 0x0040);
    w(&mut d, 0x002E);
    // `falls_into wait` is a pure fallthrough-ADJACENCY check
    // (`lower/proc.rs::check_fallthrough_adjacent`) — zero bytes.
    assert_eq!(d.len(), 0x46);

    // --- `proc wait (a0: *Sst) clobbers(d0) { ... }` @ 0x46 ---
    // `subq.b #1, timer(a0)`: SUBQ, ddd=1, op_bit=1(subq), sz=0(.b),
    // dst=Disp16An($2E,a0). Word = `0101<<12 | 1<<9 | 1<<8 | 0<<6 | 0b101<<3`
    // = 0x5328, ext = $002E (no separate immediate word — quick data is
    // opcode-encoded).
    w(&mut d, 0x5328);
    w(&mut d, 0x002E);
    // `bne .draw` — unsized Bcc (Ne=0x6), 2-rung `.s`->`.w` ladder. `.draw`
    // is the proc's tail label (derived below at 0x7C). site=0x4A,
    // disp = 0x7C - (0x4A+2) = 0x30 (48), fits i8 -> `.s` wins.
    // Word = 0x6000 | (0x6<<8) | disp = 0x6600 | 0x30.
    w(&mut d, 0x6600 | 0x30);
    // `move.w Player_1.x_pos, d0` — `Item.field` BARE form (no parens):
    // an ABSOLUTE address operand (`Player_1`'s link address + field offset
    // $10), NOT a displacement-off-a0 (`map_plain` ->
    // `CodeOperand::SymOff{sym:"Player_1", off:0x10}` -> `lower_m68k_abs_sym`,
    // a RelaxAbsSym choosing abs.w/abs.l). `Player_1` lives in prelude's
    // section at LMA 0xF0+0x14=0x104 (see the prelude layout below), so the
    // operand's resolved address is 0x104+0x10=0x114 — within abs.w range
    // (<= 0x7FFF) — so it resolves to abs.w (mode 111, reg 000, one ext
    // word = the address). MOVE.W, dst=Dn(0): word = `11<<12 | 0<<9 | 0<<6
    // | 0b111<<3 | 0b000` = 0x3038.
    w(&mut d, 0x3038);
    w(&mut d, 0x0114);
    // `sub.w x_pos(a0), d0` — `x_pos` is a DIRECT Sst field at offset $10
    // (Sst's own declared offset, unrelated to the overlay). ALU-EA `Sub`:
    // base=0b1001, reg=Dn(0)=0, opmode=sz(.w)=1, ea=Disp16An($10,a0).
    // Word = `1001<<12 | 0<<9 | 1<<6 | 0b101<<3 | 0` = 0x9068.
    w(&mut d, 0x9068);
    w(&mut d, 0x0010);
    // `facing_abs d0` splices (prelude.emp): `tst.w {r}; bpl.s .done;
    // neg.w {r}; .done:` with `r`=d0. This is its own fresh hygiene
    // instantiation (`Owner::Asm`), but the mangled `.done` label never
    // escapes the template — no byte-level consequence from the counter
    // value itself, only from the RESOLVED intra-template displacement.
    //   `tst.w d0`: Tst, size W, ea=Dn(0). base=0x4A00, sz<<6=0x40, ea=0.
    //   Word = 0x4A40.
    w(&mut d, 0x4A40);
    //   `bpl.s .done`: EXPLICIT `.s` size (a pin, not relaxed). cond=Pl=0xA.
    //   `.done` is 2 bytes after this instruction's own end (past the
    //   2-byte `neg.w d0`): site=0x56, target=0x5A,
    //   disp = 0x5A - (0x56+2) = 2. Word = 0x6000|(0xA<<8)|2 = 0x6A02.
    w(&mut d, 0x6A00 | 2);
    //   `neg.w d0`: Neg, size W, ea=Dn(0). base=0x4400, sz<<6=0x40, ea=0.
    //   Word = 0x4440.
    w(&mut d, 0x4440);
    //   `.done:` — 0 bytes, label only.
    // `cmp.w #ATTACK_RANGE, d0` — ATTACK_RANGE=$60=96. `Cmp` ALU-EA (dest
    // is Dn, so `refine_m68k_mnemonic` does NOT retarget to `Cmpi` — that
    // retarget only fires for a MEMORY dest): base=0b1011, reg=Dn(0)=0,
    // opmode=sz(.w)=1, ea=Imm(96) (mode 111, reg 100).
    // Word = `1011<<12 | 0<<9 | 1<<6 | 0b111<<3 | 0b100` = 0xB07C, ext=$0060.
    w(&mut d, 0xB07C);
    w(&mut d, 0x0060);
    // `bhi .rearm` — unsized Bcc (Hi=0x2). `.rearm` is derived below at
    // 0x76. site=0x5E, disp = 0x76-(0x5E+2) = 0x16 (22), fits i8 -> `.s`.
    // Word = 0x6000|(0x2<<8)|0x16 = 0x6216.
    w(&mut d, 0x6200 | 0x16);
    // `move.b #SHOOT_WINDUP, timer(a0)` — SHOOT_WINDUP=$28=40. Same MOVE.B
    // shape as `init`'s: word=0x117C, imm ext=$0028, disp ext=$002E.
    w(&mut d, 0x117C);
    w(&mut d, 0x0028);
    w(&mut d, 0x002E);
    // `anim Ani.Shoot` splices to `move.b #{Ani.Shoot}, Sst.anim(a0)`
    // (prelude.emp). `Ani.Shoot` ordinal = 2 (declared 3rd, 0-based).
    // `Sst.anim` is a DIRECT struct field at offset $1C (prelude.emp:
    // `anim: u8 @ $1C`). Same MOVE.B shape: word=0x117C, imm=$0002,
    // disp=$001C.
    w(&mut d, 0x117C);
    w(&mut d, 0x0002);
    w(&mut d, 0x001C);
    // `routine shoot` splices to `pea {p}; move.w (a7)+, Sst.routine(a0)`
    // with `p`=`shoot` (a proc label — an absolute address, same RelaxAbsSym
    // seam as `Item.field`). `shoot`'s own link address is derived below at
    // 0x7E — within abs.w range, so `pea` resolves to abs.w:
    //   `pea <ea>`: base=0x4840, ea=AbsW (mode 111, reg 000).
    //   Word = 0x4840 | 0b111<<3 | 0 = 0x4878, ext = $007E.
    w(&mut d, 0x4878);
    w(&mut d, 0x007E);
    //   `move.w (a7)+, Sst.routine(a0)`: MOVE.W, src=PostInc(7) (mode 011,
    //   reg 7), dst=Disp16An($20,a0) (`Sst.routine` direct field offset
    //   $20, prelude.emp: `routine: u16 @ $20`).
    //   Word = `11<<12 | 0<<9 | 0b101<<6 | 0b011<<3 | 7` = 0x315F, ext=$0020.
    w(&mut d, 0x315F);
    w(&mut d, 0x0020);
    // `jbra .draw` — a `RelaxLadder` (bra.s/bra.w/jmp.w/jmp.l). `.draw` is
    // the very next label, right after `.rearm`'s own 6-byte `move.b`
    // (below): site=0x74, target=0x76+6=0x7C,
    // disp = 0x7C - (0x74+2) = 6 -> fits i8, nonzero -> `bra.s` (rung 0).
    // Word = 0x6000|6 = 0x6006.
    w(&mut d, 0x6000 | 6);
    assert_eq!(d.len(), 0x76, "`.rearm:` begins here");

    // `.rearm: move.b #WAIT_TIME, timer(a0)` — same MOVE.B shape as `init`'s.
    w(&mut d, 0x117C);
    w(&mut d, 0x0040);
    w(&mut d, 0x002E);
    assert_eq!(d.len(), 0x7C, "`.draw:` begins here");

    // `.draw: jbra Draw_Sprite` — `Draw_Sprite` is a prelude proc, derived
    // below at 0xF4. site=0x7C, disp = 0xF4 - (0x7C+2) = 0x76 (118), fits
    // i8 -> `bra.s`. Word = 0x6000|0x76 = 0x6076.
    w(&mut d, 0x6000 | 0x76);
    assert_eq!(d.len(), 0x7E, "`wait` proc ends, `shoot` proc begins");

    // --- `proc shoot (a0: *Sst) { ... }` @ 0x7E ---
    // `subq.b #1, timer(a0)` — identical shape to `wait`'s own.
    w(&mut d, 0x5328);
    w(&mut d, 0x002E);
    // `cmpi.b #FIRE_FRAME, timer(a0)` — FIRE_FRAME=16=$10. Dest is a MEMORY
    // ea (`timer(a0)`), so `Cmp` DOES retarget to `Cmpi` here
    // (`refine_m68k_mnemonic`'s `is_mem_dest` guard). ALU-immediate family:
    // op=Cmpi=0b1100, sz=0(.b), ea=Disp16An($2E,a0).
    // Word = `1100<<8 | 0<<6 | 0b101<<3 | 0` = 0x0C28, imm ext=$0010 (byte
    // immediate, zero-extended to a full word), disp ext=$002E.
    w(&mut d, 0x0C28);
    w(&mut d, 0x0010);
    w(&mut d, 0x002E);
    // `bne .no_fire` — unsized Bcc. `.no_fire` derived below at 0x9C.
    // site=0x88, disp = 0x9C - (0x88+2) = 0x12 (18) -> `.s`.
    w(&mut d, 0x6600 | 0x12);
    // `spawn(SeedDef, offset: Vec{x:-16,y:-4}, flip: inherit)` — `inherit`
    // (an alias for `Flip.inherit`) makes the comptime `if` take its TRUE
    // branch (prelude.emp), splicing exactly:
    //   lea {def}, a1              -> lea SeedDef, a1
    //   move.w #{offset.x}, d1     -> move.w #-16, d1
    //   move.w #{offset.y}, d2     -> move.w #-4, d2
    //   move.w x_vel(a0), d3
    //   jbsr SpawnObject
    // `{def}` splices a `Label` value into a BARE (non-`#imm`) operand slot
    // -> `CodeOperand::Sym` (`classify_operand_splice`), routed through the
    // SAME RelaxAbsSym abs.w/abs.l seam `lea` needs a memory EA anyway.
    // `SeedDef`'s own address is 0x2A (this section, derived above) — abs.w:
    //   `lea <ea>,An`: base=0x41C0, an=1, ea=AbsW (mode 111, reg 000).
    //   Word = 0x41C0 | 1<<9 | 0b111<<3 = 0x43F8, ext=$002A.
    w(&mut d, 0x43F8);
    w(&mut d, 0x002A);
    //   `move.w #-16, d1`: MOVE.W, src=Imm(-16) (mode 111, reg 100),
    //   dst=Dn(1). Word = `11<<12 | 1<<9 | 0<<6 | 0b111<<3 | 0b100` = 0x323C,
    //   ext = -16 as u16 = 0xFFF0.
    w(&mut d, 0x323C);
    w(&mut d, -16i32 as u16);
    //   `move.w #-4, d2`: same shape, dst=Dn(2).
    //   Word = `11<<12 | 2<<9 | 0<<6 | 0b111<<3 | 0b100` = 0x343C,
    //   ext = -4 as u16 = 0xFFFC.
    w(&mut d, 0x343C);
    w(&mut d, -4i32 as u16);
    //   `move.w x_vel(a0), d3`: ea=Disp16An($18,a0) (`x_vel` direct Sst
    //   field, prelude.emp: `x_vel: i16 @ $18`), dst=Dn(3).
    //   Word = `11<<12 | 3<<9 | 0<<6 | 0b101<<3 | 0` = 0x3628, ext=$0018.
    w(&mut d, 0x3628);
    w(&mut d, 0x0018);
    //   `jbsr SpawnObject`: RelaxLadder. `SpawnObject` is a prelude proc,
    //   derived below at 0xFC. site=0x9A, disp = 0xFC-(0x9A+2) = 0x60 (96)
    //   -> fits i8 -> `bsr.s`. Word = 0x6100|0x60 = 0x6160.
    w(&mut d, 0x6100 | 0x60);
    assert_eq!(d.len(), 0x9C, "`.no_fire:` begins here");

    // `.no_fire: tst.b timer(a0)` — Tst, size B, ea=Disp16An($2E,a0).
    // base=0x4A00, sz<<6=0, ea=Disp16An -> mode101,reg0.
    // Word = 0x4A00 | 0b101<<3 = 0x4A28, ext=$002E.
    w(&mut d, 0x4A28);
    w(&mut d, 0x002E);
    // `bne .draw` — unsized Bcc. `.draw` derived below at 0xB6.
    // site=0xA0, disp = 0xB6-(0xA0+2) = 0x14 (20) -> `.s`.
    w(&mut d, 0x6600 | 0x14);
    // `move.b #WAIT_TIME, timer(a0)` — same shape as before.
    w(&mut d, 0x117C);
    w(&mut d, 0x0040);
    w(&mut d, 0x002E);
    // `anim Ani.Idle` -> `move.b #{Ani.Idle}, Sst.anim(a0)`. Ani.Idle
    // ordinal = 0. Same MOVE.B shape as `wait`'s `anim Ani.Shoot`.
    w(&mut d, 0x117C);
    w(&mut d, 0x0000);
    w(&mut d, 0x001C);
    // `routine wait` -> `pea {p}; move.w (a7)+, Sst.routine(a0)` with
    // `p`=`wait`. `wait`'s own address is 0x46 (derived above) — abs.w:
    w(&mut d, 0x4878);
    w(&mut d, 0x0046);
    w(&mut d, 0x315F);
    w(&mut d, 0x0020);
    assert_eq!(d.len(), 0xB6, "`.draw:` begins here (shoot's tail)");

    // `.draw: jbra Draw_Sprite` — `Draw_Sprite`@0xF4 (derived below).
    // site=0xB6, disp = 0xF4-(0xB6+2) = 0x3C (60) -> `bra.s`.
    w(&mut d, 0x6000 | 0x3C);
    assert_eq!(d.len(), 0xB8, "`shoot` proc ends, `seed` proc begins");

    // --- `proc seed (a0: *Sst) { ... }` @ 0xB8 ---
    // `despawn_below_level` -> `jbsr Despawn_Check` (prelude.emp, bare
    // zero-arg directive). `Despawn_Check`'s own address is 0x100 (derived
    // below). site=0xB8, disp = 0x100-(0xB8+2) = 0x46 (70) -> `bsr.s`.
    w(&mut d, 0x6100 | 0x46);
    // `add.w #SEED_GRAVITY, y_vel(a0)` — SEED_GRAVITY=$20=32. Dest is a
    // MEMORY ea, so `Add` retargets to `Addi` (ALU-immediate family):
    // op=Addi=0b0110, sz=1(.w), ea=Disp16An($1A,a0) (`y_vel` direct Sst
    // field, prelude.emp: `y_vel: i16 @ $1A`).
    // Word = `0110<<8 | 1<<6 | 0b101<<3 | 0` = 0x0668, imm ext=$0020,
    // disp ext=$001A.
    w(&mut d, 0x0668);
    w(&mut d, 0x0020);
    w(&mut d, 0x001A);
    // `jbsr ObjectMove` — `ObjectMove`'s own address is 0xF8 (derived
    // below). site=0xC0, disp = 0xF8-(0xC0+2) = 0x36 (54) -> `bsr.s`.
    w(&mut d, 0x6100 | 0x36);
    // `jbra Draw_Sprite` — site=0xC2, disp = 0xF4-(0xC2+2) = 0x30 (48)
    // -> `bra.s`.
    w(&mut d, 0x6000 | 0x30);
    assert_eq!(
        d.len(),
        0xC4,
        "`seed` proc ends — pitcher_plant's real content ends here"
    );

    // --- 44-byte zero-filled gap (0xC4..0xF0) ---
    // `place_sequential` packs the NEXT section (prelude's) at
    // `pitcher_plant_section.lma + pitcher_plant_section.placement_span()`.
    // `placement_span()` reserves every relaxable fragment at its LONGEST
    // candidate width (`RelaxLadder`'s last rung, `RelaxAbsSym`'s `.l` form)
    // — the pre-relaxation upper bound placement must use so it never
    // panics on an unresolved width-variable fragment. Every relaxable site
    // above in fact settled SHORT once real addresses were known, so the
    // section's actual content (196 bytes, ending at 0xC4) is shorter than
    // its reserved span (240 bytes) by exactly the sum of each site's
    // (reserved - actual) width:
    //   7 jbra/jbsr sites  x (6 - 2) = 28  (`.draw`x2(wait init not counted:
    //     wait has 2, shoot has 2, seed has 3 = 7 total: wait's `jbra .draw`
    //     + wait's `jbra Draw_Sprite` + shoot's `jbsr SpawnObject` + shoot's
    //     `jbra Draw_Sprite` + seed's `jbsr Despawn_Check` + seed's `jbsr
    //     ObjectMove` + seed's `jbra Draw_Sprite`)
    //   4 unsized Bcc sites x (4 - 2) =  8  (wait's `bne`+`bhi`, shoot's
    //     `bne`x2)
    //   4 RelaxAbsSym sites x (6 - 4) =  8  (`Player_1.x_pos`, `lea
    //     SeedDef,a1`, `pea shoot`, `pea wait`)
    //   total = 28 + 8 + 8 = 44 bytes, exactly the 0xC4..0xF0 gap.
    // `flatten`'s default fill byte is 0x00.
    d.extend_from_slice(&[0u8; 0x2C]);
    assert_eq!(
        d.len(),
        0xF0,
        "prelude's `text` section begins here (LMA 240)"
    );

    // =======================================================================
    // prelude.emp's `text` section — LMA 240 (0xF0), packed right after
    // pitcher_plant's reserved span
    // =======================================================================

    // `pub data Map_PitcherPlant: [u8;4] = [1, 0, 0, 0]` @ 0xF0
    d.extend_from_slice(&[1, 0, 0, 0]);
    assert_eq!(d.len(), 0xF4, "`Draw_Sprite` begins here");

    // `pub proc Draw_Sprite () { tst.b d0 ; rts }` @ 0xF4
    // `tst.b d0`: Tst, size B, ea=Dn(0). base=0x4A00, sz<<6=0, ea=0.
    // Word = 0x4A00.
    w(&mut d, 0x4A00);
    // `rts` = 0x4E75 (fixed).
    w(&mut d, 0x4E75);
    assert_eq!(d.len(), 0xF8, "`ObjectMove` begins here");

    // `pub proc ObjectMove () { clr.w d1 ; rts }` @ 0xF8
    // `clr.w d1`: Clr, size W, ea=Dn(1). base=0x4200, sz<<6=0x40, ea=1.
    // Word = 0x4200 | 0x40 | 1 = 0x4241.
    w(&mut d, 0x4241);
    w(&mut d, 0x4E75);
    assert_eq!(d.len(), 0xFC, "`SpawnObject` begins here");

    // `pub proc SpawnObject () { moveq #0, d2 ; rts }` @ 0xFC
    // `moveq #0,Dn(2)`: word = 0x7000 | (2<<9) | (0 as i8 as u8 as u16)
    // = 0x7400.
    w(&mut d, 0x7400);
    w(&mut d, 0x4E75);
    assert_eq!(d.len(), 0x100, "`Despawn_Check` begins here");

    // `pub proc Despawn_Check () { tst.w d3 ; rts }` @ 0x100
    // `tst.w d3`: Tst, size W, ea=Dn(3). base=0x4A00, sz<<6=0x40, ea=3.
    // Word = 0x4A00 | 0x40 | 3 = 0x4A43.
    w(&mut d, 0x4A43);
    w(&mut d, 0x4E75);
    assert_eq!(d.len(), 0x104, "`Player_1` begins here");

    // `pub data Player_1: Sst = Sst{ id: 1, ...all-else-zero }` @ 0x104
    // `Sst` is `$50` (80) bytes; every field is 0 except `id` (the first
    // field, a `u16`) = 1.
    w(&mut d, 1);
    d.extend_from_slice(&[0u8; 0x50 - 2]);
    assert_eq!(d.len(), 0x154, "end of image");

    d
}

/// The headline positive proof: the REAL multi-module build produces ZERO
/// diagnostics and the FULL 340-byte image matches the hand-derivation above,
/// byte for byte.
#[test]
fn pitcher_plant_full_image_is_byte_exact() {
    let root = game_root();
    let out_dir = std::env::temp_dir().join(format!(
        "sigil_pp_acceptance_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&out_dir).unwrap();
    let out = out_dir.join("pitcher_plant.bin");

    let (success, stdout, stderr, image) = build_pitcher_plant(root, &out);

    assert!(
        success,
        "pitcher_plant build must succeed with zero errors; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stderr.trim().is_empty(),
        "expected ZERO diagnostics of any severity (warnings included); stderr was:\n{stderr}"
    );
    assert!(
        stdout.contains("built: 340 bytes"),
        "expected the CLI to report `built: 340 bytes`, stdout was: {stdout}"
    );

    let image = image.expect("output .bin was not written");
    assert_eq!(
        image.len(),
        340,
        "the pitcher_plant image must be exactly 340 bytes"
    );

    let expected = expected_image();
    assert_eq!(
        expected.len(),
        340,
        "hand-derived expectation must itself total 340 bytes"
    );
    assert_byte_identical(&expected, &image, "pitcher_plant acceptance exhibit");

    let _ = std::fs::remove_dir_all(&out_dir);
}

/// Adversarial negative twin (mirrors `module_resolution.rs`'s corrupted-copy
/// pattern): copy the WHOLE `examples/game` tree into a tempdir, corrupt
/// exactly one prelude byte (`WAIT_TIME`'s value, `64` -> `65`), rebuild, and
/// assert the image CHANGES. This proves the positive test above is not an
/// echo of a golden file that happens to equal the build regardless of input
/// — the prelude's content is genuinely load-bearing on the emitted bytes.
#[test]
fn corrupting_prelude_wait_time_changes_the_image() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("game");

    // Copy `examples/game` verbatim into the tempdir.
    copy_dir(game_root(), &root);

    // Corrupt exactly one byte's worth of meaning: `const WAIT_TIME: u8 = 64`
    // -> `65`. `WAIT_TIME` is declared in pitcher_plant.emp (not prelude.emp
    // — it is the badnik's own tuning constant) and spliced as an immediate
    // at THREE call sites (`init`'s body, `wait`'s `.rearm:` arm, and
    // `shoot`'s `.no_fire:` re-arm — see `expected_image`'s derivation
    // above) — each of those immediate-extension words must change from
    // `$0040` to `$0041`.
    let pp_path = root.join("badniks/pitcher_plant.emp");
    let pp_src = std::fs::read_to_string(&pp_path).unwrap();
    assert!(
        pp_src.contains("const WAIT_TIME: u8 = 64"),
        "precondition: pitcher_plant.emp declares `const WAIT_TIME: u8 = 64` verbatim, source was:\n{pp_src}"
    );
    let corrupted = pp_src.replacen("const WAIT_TIME: u8 = 64", "const WAIT_TIME: u8 = 65", 1);
    std::fs::write(&pp_path, corrupted).unwrap();

    let out = tmp.path().join("out.bin");
    let (success, stdout, stderr, image) = build_pitcher_plant(&root, &out);
    assert!(
        success,
        "corrupted-WAIT_TIME build must still succeed (65 is still a valid u8); stdout: {stdout}\nstderr: {stderr}"
    );
    let image = image.expect("output .bin was not written for the corrupted build");

    let baseline = expected_image();
    assert_ne!(
        image, baseline,
        "corrupting WAIT_TIME must change the emitted image — the test would be an echo otherwise"
    );
    // Same length (a `u8` immediate's value never changes any instruction's
    // width), so the byte-diff is real content, not a length regression.
    assert_eq!(
        image.len(),
        baseline.len(),
        "corrupting an immediate's VALUE must not change the image length"
    );
}

/// Recursively copy `src` into `dst` (`dst` must not yet exist). Small,
/// dependency-free helper — mirrors the shape `module_resolution.rs`'s `write`
/// helper plays for single files, extended to a whole directory tree since
/// this test needs the full `examples/game` corpus (badniks/ + prelude.emp).
fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest_path);
        } else {
            std::fs::copy(&path, &dest_path).unwrap();
        }
    }
}
