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

## Re-baseline: player fixes — balance-on-solids + spindash anchor (2026-07-10) — the current pin

Volence-reported gameplay fixes (aeon `add02b9` + the TouchResponse
lifecycle commit): the `ST_ON_OBJECT` per-frame clear moved from
player_common's mid-tick spot (which blinded the animation classifier's
ledge probe) to the top of TouchResponse's player loop, and the spindash
charge reverted to classic STANDING-size physics (the curl happens at
release — the donor charge frames are drawn for the standing origin).
The TouchResponse `bclr` is a +6-byte insert in `engine/objects/
collision.asm`, INSIDE the gated span — `tile_cache`/`collision_lookup`
slid +6 (`Tile_Cache_GetCollision` plain `$431A`→`$4320` / debug
`$4A86`→`$4A8C`; collision bases plain `$4C02`→`$4C08` / debug
`$5426`→`$542C`; resume orgs `$4C2C`/`$5450`); vdp_init/hblank/
controllers/math and the interrupt vectors verified UNMOVED. Window
CONTENT is byte-identical (site and target shifted together — disp
`$F6FA` held).

- Non-debug `s4.bin`: sha256
  **`fc69fdbf8d0c8f63d30a10410118775be1c1bd6b1ef70d74b558578fbb73af37`**.
- Debug `s4.debug.bin`: sha256
  **`5e4cbe974007183c652868def207d20a5b72629e0c832755f8dce9d57f42ea58`**.

## Tranche 4 ports #1/#2 — the animation data gates (2026-07-10, overnight)

`particle_anims.emp` + `sonic_anims.emp` (aeon `b66cb4e` + the sonic port
commit): the campaign's first GAME-DATA regions, both past `org $10000` —
engine-block drift cannot move them. Bases/sizes (content shape-invariant;
only the base shifts with `__DEBUG__`):

- `sonic_anims`: plain `$30978`, debug `$309E0`, size `0x74` (11-word
  table + bodies + six align pads).
- `particle_anims`: plain `$309EC`, debug `$30A54`, size `0x8`.

Gate defines live in `games/sonic4/main.asm` (`SIGIL_EMP_SONIC_ANIMS`
resume plain `$309EC`/debug `$30A54`; `SIGIL_EMP_PARTICLE_ANIMS` resume
plain `$309F4`/debug `$30A5C`). Gate-off byte-neutrality sha256 ×3 at the
`755c2c91…` pin (both gates inert without the defines). The TEN-module
mixed gates (`mixed_tranche4_*`) are the acceptance surface; re-pin these
bases on any data-region re-baseline.

## Tranche 4 port #3 — act_descriptor (2026-07-10)

`act_descriptor.emp` (the OJZ act-1 descriptor + 9-section table, the
campaign's biggest and first STRUCT-TYPED port — the Tier-1+2 act shape).
Bases/size (content shape-invariant modulo per-shape fixup addresses):

- `act_descriptor`: plain `$14AEE`, debug `$14B56`, size `0x274`
  (`Act` descriptor `0x22` + 9 × `Sec` `0x42`).

Gate define `SIGIL_EMP_ACT_DESCRIPTOR` lives INSIDE
`games/sonic4/data/levels/ojz/act1/act_descriptor.asm` (the generated
includes at the file top stay AS-side in BOTH shapes; resume org plain
`$14D62` / debug `$14DCA`). The scroll test's four consumers were re-spelled
`lea (OJZ_Act1_Descriptor).l, aN` (byte-neutral — asl already picked abs.l)
so the new pinned-width lea deferral carries them across the seam. Gate-off
byte-neutrality sha256 ×3 at the `755c2c91…` pin. The ELEVEN-module mixed
gates are the acceptance surface; the port test pins 41 cross-seam label
addresses from both symbol tables — re-derive them on any re-baseline.

## Re-baseline: sonic_anims pad-drop + inline rewrite (2026-07-10) — the current pin

Tranche-4 STEP-5 (post-merge, Volence-approved): the five inter-body
`align 2` pads in `sonic_anims` were dead weight (AnimateSprite reads
scripts BYTE-wise; verified — only the TABLE is word-read), so the bodies
pack and the offsets construct's fully-INLINE form becomes expressible —
the .emp rewrote to `Name: [u8; N] = [...]` members (−6 bytes, region
`0x74` → `0x6E`), the AS twin dropped the same pads in LOCKSTEP (the
trailing align stays: it guards Ani_Particle's word-read table evenness).
`Ani_Particle` slid −6 (plain `$309EC`→`$309E6`, debug `$30A54`→`$30A4E`;
resume orgs `$309EE`/`$30A56`); `EndOfRom` UNCHANGED both shapes (absorbed
at `org $50000`); act_descriptor and everything below `$30978` unmoved.
Gate-off neutrality sha256 ×3. Behavior check: walk cycle advancing
through correct frame bytes live in oracle.

- Non-debug `s4.bin`: sha256
  **`907a902966efc0dccf09339a10da3dc949560983fc442c8bd302ed696bd2fbd7`**.
- Debug `s4.debug.bin`: sha256
  **`7148f938b1d0e4b0f465e8204566ce598c23cac93381fadbc46a67c0452c5d78`**.

## Tranche-5 port #1: game_loop (2026-07-10)

`engine/system/game_loop.asm` → `engine/system/game_loop.emp` under
`SIGIL_EMP_GAME_LOOP` at `engine/engine.inc:136` (the sixth engine-side
gate; resume org plain `$2310` / debug `$239E`). Region plain
`$22FE..$2310` / debug `$238C..$239E` (0x12 bytes: GameLoop +
GameState_Idle). The FIRST code module taking build-shape defines — the
.emp requires `-D SOUND_DRIVER_ENABLED` and `-D SOUND_DEBUG_HOTKEYS`
(0|1); both pinned shapes are the (1,0) combo (build.sh defaults:
sound on, hotkeys env-opt-in off), where sonic4's `gameDebugTick`
expansion contributes ZERO bytes. The other three combos are gated
module-level against the AS twin assembled through sigil's AS front-end
(`game_loop_port.rs`'s matrix — ALSO the drift guard for the H2
expansion mirror, kill-list row 9: it re-extracts the macro body from
the real `games/sonic4/config/game.asm` every run). Cross-seam reads:
`VSync_Wait` (plain `$2262` / debug `$22EC`) and `Sound_DrainSfxRing`
(plain `$5EDE` / debug `$739C`) as pc-relative `bsr.w` targets,
`Game_State` (`$FFFF8004`, engine RAM, shape-invariant); outbound
consumer `boot.asm:220`'s `bra.w GameLoop`. The TWELVE-module mixed
gates are the acceptance surface. Gate-off byte-neutrality sha256 ×3 at
the `907a9029…` pin (+ debug `7148f938…`, + demo.bin builds clean —
the engine-side gate must never define for other games). Reference pins
UNCHANGED.

## Tranche-5 port #2: sound_api (2026-07-10)

`engine/sound/sound_api.asm` → `engine/sound/sound_api.emp` under
`SIGIL_EMP_SOUND_API` INSIDE engine.inc's `ifdef SOUND_DRIVER_ENABLED`
block (resume org plain `$5F7C` / debug `$743A`). Region plain
`$5D94..$5F7C` / debug `$7252..$743A` (0x1E8 bytes, twelve Sound_* procs).
Three language features shipped mid-port: (1) the abs-sym ext-word fence
RELAXED to positional (`move.w #$0100, (Z80_BUS_REQUEST).l` — the stopZ80
shape; ext words BEFORE the sym operand precede the abs field, which
stays last), (2) LINK-TIME imm32 (`ImmLink`, `Value32Be` at offset 2 —
the emp mirror of the AS side's `try_defer_long_imm`; `.l` only, the
`.b`/`.w` gap stays ledgered), (3) `sr`/`ccr` operands. Slot ADDRESSES
stay AS-owned as extern-equ sums (`equ *_SLOT = extern("SND_Z80_BASE") +
extern("SND_REQ_*")` — the MUSIC_PARAM block derives from a Z80-driver
RAM label and floats with driver resizes, so no comptime mirror); only
the 7 immediate-position values are mirrored, drift-guarded (kill-list
row 10). `SongTable`/`SongPatchTable` read as imm-link equs — .emp-side
under `SIGIL_EMP_MT`, so the mixed build exercises .emp-defines/
.emp-consumes. Cross-seam positions (listing symbol tables): RAM
`Ring_Sfx_Speaker`/`Sfx_Ring_Buf`/`Wr`/`Rd` plain `$FFFFAF30/32/3A/3B`,
debug `$FFFFAF52/54/5C/5D`; ROM `SongTable` plain `$63AE0` / debug
`$65522`, `SongPatchTable` plain `$63AE4` / debug `$6552E`; the SND_*
equ values are shape-invariant (MUSIC_PARAM base `$1CA6`). The
THIRTEEN-module mixed gates are the acceptance surface. Gate-off
byte-neutrality sha256 ×3 at the `907a9029…` pin (+ debug + demo).
Reference pins UNCHANGED.

## Re-baseline: tranche-5 step 2 — the modernize pass (2026-07-10, the current pin

Tranche-5 STEP-2 under the RATIFIED loop (Volence, 2026-07-10 — see
`notes/campaign-port-loop.md`: the byte gate is a step-1 transcribe
verifier; step 2 converts to the complete house format and MAY change
bytes, paying lockstep + re-pin). sound_api.emp: all eight `bra.w`
tail-calls → `jbra` (only Sound_Ping/Sound_PlaySample relax to `.s` —
−2 B each, −4 B total), the four inline stopZ80/startZ80 expansions →
`stop_z80()`/`start_z80()` comptime-fn templates (hygienic per-site
`.wait_z80`; byte-identical), pinned `(X).w/.l` spellings → bare
width-rule idiom (byte-identical). AS twin lockstep: the two `bra.s`
sites only. game_loop.emp was born-modern (no changes).

Region `sound_api` shrinks 0x1E8 → **0x1E4** (plain `$5D94..$5F78`,
debug `$7252..$7436`; engine.inc resume orgs re-pinned). Everything
after Sound_PlaySample slid −4: `Sound_PlaySFX` plain `$5E94` / debug
`$7352` (outbound proofs re-anchored to base+0x100), `Sound_DrainSfxRing`
plain `$5EDA` / debug `$7398` (game_loop's cross-seam drain position +
the mixed-gate head-pin displacements re-derived: plain `3BD6`, debug
`5006`). EndOfRom unchanged (org-anchored); demo unaffected.

- Non-debug `s4.bin`: sha256
  **`bcd4e3a5f42d63a7994fb989d076435a5242b4cb48203a99edfb01ac34189ee4`**.
- Debug `s4.debug.bin`: sha256
  **`634fea687f6ebe44fca4cc50a9e2e9cfaeaa6c4740fcaffbc429f96bc6305184`**.
