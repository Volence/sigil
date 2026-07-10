# Golden provenance — Sigil reference gates

## M0 acceptance — the Z80 sound driver (RE-EXPRESSED in M1.D T6)

The M0 milestone proved Sigil reproduces the Aeon Z80 sound driver (Region A =
the resident phase-0 driver; Region B = the phase-`08000h` Moving-Trucks / SFX
bank) byte-for-byte. It was originally gated by a **bounded harness** that
assembled those two regions *in isolation*, stubbing the ~42 68k leaf symbols the
driver referenced but that the isolated build did not define
(`golden/stub-syms.toml`, re-derived by the `regen` bin from a fresh `s4.lst`).

**That whole apparatus was retired in M1.D T6.** Sigil now assembles the entire
`main.asm` include tree byte-exact with **zero stubs** (the `m1d_rom` gate), so
the sound-driver regions fall out of the full build directly. The following were
deleted: `harness_root.asm`, `golden/{stub-syms.toml,windows.toml,region_a.bin,
region_b.bin,sigil_a.bin,sigil_b.bin}`, the `regen` bin, and the
`build_harness`/`assemble_reference_regions` lib helpers.

M0 acceptance is now the `m0_regions` gate
(`crates/sigil-harness/tests/m0_regions.rs`): it runs the full non-debug build
(no stubs), locates the linked sections at LMA `0x3EA` (Region A) and `0x60000`
(Region B), and asserts each is byte-identical to the corresponding window of the
**live** `aeon/s4.bin`. Region lengths are read from the live sections (not
pinned), so the gate tracks driver growth automatically. It is reference-gated
(needs the sibling `aeon` tree) and self-tracks Aeon drift the same way
`m1d_rom` does — no committed golden blob to go stale.

To run: `SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness
--test m0_regions`.

## M1.B reference-ROM pin (acceptance gate)

The `m1b_gate` integration test (`crates/sigil-harness/tests/m1b_gate.rs`) checks
the linker's byte-mutating passes against the live Aeon reference ROM. The pin
below records the exact reference the gate was validated against. This is a
**moving target** — the Aeon ROM (and thus its stored checksum) changes as the
engine evolves; re-baseline this pin *deliberately* when intentionally tracking a
new Aeon build, and re-confirm the gate passes against it.

- Aeon repo commit (reference): **`9bacc939ae7c7c5300fc7e50548d851373128a23`**
- `aeon/s4.bin` length: **458666 bytes** (`0x6FF2A`)
- Stored header checksum at `0x18E`: **`0x5CBE`**

`header_checksum_reproduces_reference_rom_18e` does not hardcode `0x5CBE`: it
reads the stored word from the reference ROM, zeroes `0x18E`, recomputes via
`sigil_link::apply_header_checksum`, and asserts equality — so the gate stays
correct across re-baselines as long as the Sega checksum algorithm holds. The
values above are the observed pin at the time this gate was written.

## M1.D reference pin (T0.0, 2026-07-04) — the authoritative full-ROM target

Re-pinned at the start of M1.D. **The M1.B pin above (458666 B / `0x5CBE`) was
captured with a DIRTY aeon working tree** (BG/editor WIP carried in, per the M0
snapshot note). A *clean* `9bacc93` produces a different, smaller ROM — the
build is non-hermetic (`build.sh` runs python generators that consume the editor
JSON), so the reference depends on the working-tree state, not just the commit.
The gates self-track Aeon drift (they diff against the LIVE `aeon/s4.bin`, no
committed golden blob), so this pin records only the last DELIBERATE re-baseline.

- Aeon repo commit: **`f828406`** (the engine/game split, E1-E7 merged), working
  tree **clean**. Re-baselined 2026-07-08 from the prior `9bacc93` pin.
- `aeon/s4.bin` length: **451198 bytes** (`0x6E23E`; assembled `EndOfRom` =
  `0x658B4`, unchanged from `9bacc93` — the +320 B is the larger post-`convsym`
  symbol-table append, not body growth).
- sha256: **`71a7e24560425d6f00e8885995f1b3d484de8d9ef4b01addc7dd97c58392cae2`**

To reproduce the non-debug reference: stash any aeon WIP → `./build.sh sonic4`.
(The `regen` bin that formerly re-derived the M0 goldens was retired in T6.)

Split-baseline notes (what the drift from `9bacc93` required in Sigil, not just
a re-pin): the engine/game split moved the ROM header fields into `equ` string
symbols read via `strlen()`/`substr()` (front-end had to resolve a STRING `equ`,
not just `set`), and it re-expressed game RAM as a phased `align 256` block
(front-end had to reproduce asl's in-phase ALIGN = `round_up(pos + n, n)`, a full
extra `n`). The `m1c_root.asm` bounded fixture's include paths were retargeted to
`engine/` + `games/sonic4/config/`, and the two resident interrupt vectors
(HBlank/VBlank) shifted `+0x114`.

## M1.D T5 — the `__DEBUG__` reference (A2, 2026-07-05)

The debug parity gate (`crates/sigil-harness/tests/m1d_debug_rom.rs`) compares
Sigil's `__DEBUG__` build against a **deliberately-built** debug reference — NOT
the shipped `s4.bin`. It is a separate on-disk artifact (`aeon/s4.debug.bin`, a
gitignored build output), captured so the non-debug reference `s4.bin` (which the
other gates depend on) can be restored alongside it.

Exact capture procedure (from a **clean** aeon at the current `f828406` pin):

```
cd aeon
DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4   # -D __DEBUG__ -D SOUND_DRIVER_ENABLED
cp s4.bin s4.debug.bin && cp s4.lst s4.debug.lst   # capture the debug outputs
./build.sh sonic4                                  # rebuild + restore the non-debug s4.bin/s4.lst
```

- Debug `s4.debug.bin` length: **458982 bytes** (post-`convsym -a` append; the
  deb2 symbol table is larger under `__DEBUG__`), sha256
  `a904b3c9d2e0fe1aec5c0c479b8f6a119b74563b9428d99d73125753101bb4d1`.
- Sigil's **assembled** debug ROM (pre-convsym, what the gate emits): **`0x673A2`
  bytes** (`EndOfRom`), byte-identical to `s4.debug.bin` over `[0, 0x673A2)`
  EXCEPT the `convsym`/`fixheader` header bytes. At this baseline the append
  grew the `EndOfRom-1` pointer past a byte boundary, so it differs in FIVE
  bytes: the checksum `{0x18E,0x18F}` and the 3-byte ROM-end pointer tail
  `{0x1A5,0x1A6,0x1A7}` (the earlier smaller pin differed in `{0x1A6,0x1A7}`
  only) — the same A1/A2 out-of-scope decision as `m1d_rom`.

⚠️ Building debug **clobbers** `aeon/s4.bin`/`s4.lst` (the non-debug pin). Always
restore them afterwards (the final `./build.sh sonic4` above), and confirm
`sha256(s4.bin)` matches the current pin before relying on the non-debug gates.

## Re-baseline: forest-bg restore (2026-07-08) — the current pin

The parked forest-bg work (aeon stash `sigil-m1d: park forest_bg_gen + editor
experiments during byte-exact pin`) was restored: the dual-tree colonnade
generator (`tools/forest_bg_gen.py`, 340 tiles vs 276) plus editor-export tweaks
(`entity_data.asm`, `vram_bases.asm`, section objects/rings JSONs). Both ROMs
were rebuilt per the T5 capture procedure above and all gates re-run green
(harness + full workspace, 1429 tests).

- Aeon repo state: **`e5b256c` + the bg-restore working tree** (dirty at capture
  time — the aeon commit lands after Volence's boot-check).
- Non-debug `s4.bin`: **451198 bytes** (unchanged; assembled `EndOfRom` =
  `0x658B4`, also unchanged — the +64 bg tiles fit inside existing `align`
  padding), sha256
  **`8ce6dd7e30553b8525ddda19ebe3365cc5d24cc62dccfb9c0e6a227d70bc25ef`**.
- Debug `s4.debug.bin`: **458982 bytes** (unchanged; assembled `EndOfRom` =
  `0x673A2`, unchanged), sha256
  **`13c7b06355b658ee299756840a80b566005cdbbd5192755e8eae506a5f4fd22f`**.
- Sound-block layout UNCHANGED: DAC banks at `$50000`–`$60000`, MT bank at
  `$60000` — no dac/MT pin shift; `dac_port.rs` goldens untouched.
- Collision `.bin` files verified byte-identical across the rebuild (still the
  pinned S&K-import tables; OJZ collision switch remains deferred).

## Re-baseline: Collision_GetType step-5 optimize (2026-07-10) — the current pin

First tranche-3 STEP-5 (post-merge optimize) commit: `Collision_GetType`
drops the world-column stack push (Y shifts in place in d1; the
`move.w d1,d2` save, push/pop round-trip, and `.cgt_air_pop` discard path
all deleted) and tail-calls `Tile_Cache_GetCollision` (`jbsr`+`rts` →
`jbra`). The routine shrinks `0x32` → `0x24` bytes; both shapes rebuilt
per the T5 capture procedure and the whole gate surface re-pinned
(`engine.inc` resume orgs plain `$4C38`→`$4C2A` / debug `$545C`→`$544E`;
the tranche-3 map size + both byte windows in `mixed_dac_rom.rs`,
`collision_lookup_port.rs`, `tranche3_negative_probes.rs`). The shrink is
absorbed by the `org $10000` sound boundary, so assembled `EndOfRom` is
UNCHANGED in both shapes — the file-length deltas below are the smaller
`convsym` symbol tables (the deleted `.cgt_air_pop` local).

- Aeon repo commit: **`4352a40`** (the step-5 optimize itself), working tree
  clean at capture; gate-off byte-neutrality sha256 ×3 verified.
- Non-debug `s4.bin`: **451176 bytes** (assembled `EndOfRom` = `0x658B4`,
  unchanged), sha256
  **`36b2e3038e76439ca77fbbed3602b25899eaa7c352db07ce737f6e5a91606439`**.
- Debug `s4.debug.bin`: **458960 bytes** (assembled `EndOfRom` = `0x673A2`,
  unchanged), sha256
  **`993339f82cf81d9be0b6d6356e0b5de885d9f9903f634e741a99d19fea373fe6`**.
- Collision region bases UNCHANGED (plain `$4C06`, debug `$542A`);
  `Tile_Cache_GetCollision` UNCHANGED (plain `$431E`, debug `$4A8A`);
  sound-block layout UNCHANGED.

## Re-baseline: vdp_init `clr.l` step-5 optimize (2026-07-10) — the current pin

Second tranche-3 STEP-5 commit: both `VDP_Dirty_Mask` zero-writes in
`vdp_init` become `clr.l` (RAM operand — the 68000 `clr` read-before-write
hazard only matters on I/O; and note it is a SIZE win, not a speed win:
`moveq`+`move.l (abs.w)` and `clr.l (abs.w)` are both 20 cycles, but the
pair is 6 bytes to `clr.l`'s 4 and burns a scratch register). The region
shrinks `0x4C` → `0x48`, and since vdp_init sits at the FRONT of the gated
chain, every region between it and the `org $10000` boundary slid −4:
hblank `$227E`→`$227A` / `$230C`→`$2308`, controllers `$2290`→`$228C` /
`$231E`→`$231A`, math `$2468`→`$2464` / `$25FA`→`$25F6`, collision
`$4C06`→`$4C02` / `$542A`→`$5426`, `Tile_Cache_GetCollision`
`$431E`→`$431A` / `$4A8A`→`$4A86`, and the two resident interrupt
handlers (`HBlank_Dispatch` `$227E`→`$227A`, `VBlank_Handler`
`$2156`→`$2152` — the m1c vector-table stubs). All five `engine.inc`
resume orgs, every port-gate map base/window, both probe files, and the
mixed maps re-derived from the rebuilt listings. `EndOfRom` again
UNCHANGED both shapes (absorbed by `org $10000`); module CONTENT bytes
outside vdp_init are unchanged (no pc-rel crosses the vdp_init boundary —
collision's tail `bra.w` site and target slid together, disp `$F6FA`
held).

- Aeon repo commit: **`9eb2101`** (the vdp_init step-5 optimize, child of
  `4352a40`), working tree clean at capture; gate-off byte-neutrality
  sha256 ×3 verified.
- Non-debug `s4.bin`: **451176 bytes** (assembled `EndOfRom` = `0x658B4`,
  unchanged), sha256
  **`57ff6b0d66596fd8a72c08027e1cc3bf3a8563d4f888926fc1f8be8e97a89904`**.
- Debug `s4.debug.bin`: **458960 bytes** (assembled `EndOfRom` = `0x673A2`,
  unchanged), sha256
  **`3cb6679299d4fdba287506986b3f713ad5fdedefd18966868231c74f514b7ee2`**.
- vdp_init region bases UNCHANGED (plain `$1C14`, debug `$1C96`);
  `BootData_VDPRegs` UNCHANGED (`$3CE`/`$3D2`); sound-block layout
  UNCHANGED.
