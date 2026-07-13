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

## Re-baseline: tranche-6 step 5 — test_particle optimize (2026-07-10) — the current pin

Tranche-6 STEP-5 under the RATIFIED loop: two peepholes in
`test_particle` (.emp + AS twin in LOCKSTEP) — `moveq #0,d0` +
`move.b d0, anim(a0)` → `clr.b anim(a0)` (−2 B), and the gravity
register round-trip (`move.w y_vel(a0),d0` / `addi.w` / `move.w`
back) → the read-modify-write `addi.w #PARTICLE_GRAVITY, y_vel(a0)`
(−6 B). Region `test_particle` shrinks 0x5A → **0x52** (base `$10F8A`
UNCHANGED, shape-invariant; end / bank resume org `$10FE4`→`$10FDC`;
`TestParticle_Main` now `$10FCA` both shapes). Everything in
`$10FDC..$5FFFF` slid −8; absorbed at `org $60000` (`EndOfRom` + all
sound/data pins at/after `$60000` UNCHANGED). Re-derived per-shape
positions: act_descriptor plain `$14AE6` / debug `$14B4E` (resume orgs
`$14D5A`/`$14DC2`), sonic_anims plain `$30970` / debug `$309D8`,
particle_anims/`Ani_Particle` plain `$309DE` / debug `$30A46` (resume
orgs `$309E6`/`$30A4E`). Demo unaffected.

- Non-debug `s4.bin`: sha256
  **`588adf815c5a84402981a495e3d96f732e721d3ef5560286d9eeb6ef355f0f3f`**.
- Debug `s4.debug.bin`: sha256
  **`ed96301f5303841a7f12c02ab8dbde5e413b68dca4caed348419ba887504a4f7`**.

## Re-baseline: tranche-7 step 5 — collision per-player standing-bit claim (2026-07-10) — the current pin

Tranche-7 STEP-5 under the RATIFIED loop (`collision.asm` + `collision.emp`
+ `aabb.inc` + `aabb.emp` + `constants.emp` in LOCKSTEP): `Touch_Solid`'s
top-contact now claims the object with THIS player's standing bit
(`moveq #ST_P1/P2_STANDING` selected by `cmpa.l #Player_1` — the ledge
probe scans by player identity; the wrong-bit + stale-bit failures were
live-verified in oracle), four `move.w #0` zero-writes became `clr.w`
(−8 B), the `aabb` zero-copy alias skip dropped the redundant `cdim`
copy (−4 B, BOTH twins), and a dead-path SST reload was elided; net the
`collision` region shrinks `0x170` → **`0x16E`** (bases UNCHANGED — plain
`$308A`, debug `$3344`; `TouchResponse` head unmoved, only `Touch_None`/
`Touch_Hurt`/`Touch_Solid` bodies float within the window). New
`SIGIL_EMP_COLLISION` resume orgs plain **`$31F8`** / debug **`$34B2`**.

`rings.asm` (NOT ported; between collision and `org $10000`) inherited
the shared `aabb.inc` alias-skip (−4) AND slid −2 from collision's
shrink, so **everything between the collision region end and `org $10000`
slid −6 total**. Re-derived per-shape positions: `Collision_GetType`
(collision_lookup base) plain `$4C08`→**`$4C02`** / debug `$542C`→**`$5426`**
(resume orgs `$4C26`/`$544A`), `Tile_Cache_GetCollision` plain
`$4320`→**`$431A`** / debug `$4A8C`→**`$4A86`**, `sound_api` base plain
`$5D94`→**`$5D8E`** / debug `$7252`→**`$724C`** (resume orgs `$5F72`/`$7430`;
`Sound_PlaySFX` = base+`$100`), `Sound_DrainSfxRing` plain
`$5EDA`→**`$5ED4`** / debug `$7398`→**`$7392`** (game_loop's cross-seam
`bsr.w` drain disp `$3BD6`→`$3BD0` plain / `$5006`→`$5000` debug — the
call site is unmoved, the target slid). Regions BEFORE collision
(`vdp_init` `$1C14`/`$1C96`, `hblank` `$227A`/`$2308`, `controllers`
`$228C`/`$231A`, `math` `$2464`/`$25F6`, `game_loop` `$22FE`/`$238C`,
`VSync_Wait` `$2262`/`$22EC`) + the interrupt vectors are UNMOVED;
everything at/after `org $10000` (`act_descriptor`, `sonic_anims`,
`particle_anims`, the test-object bank, `SongTable`/`SongPatchTable`
`$63AE0`/`$65522`) is UNMOVED — the −6 is absorbed at the `org $10000`
sound boundary, so assembled `EndOfRom` is UNCHANGED both shapes
(`$658B4` / `$673A2`). Cross-seam collision-lookup tail (`bra.w
Tile_Cache_GetCollision`, site + target both slid −6) holds its disp
`$F6FA` plain / `$F642` debug — window CONTENT byte-identical.

`constants.emp` grew `ST_P2_STANDING = 4` (the +1 ensure — the twin's
guard count is now **20**, and its AS-truth mirror joined the SHARED
`test_support::engine_constant_equs()` helper, the single place). Every
gate that compiles the constants twin re-pinned its guard count
(19→20 direct; 49→50 for the `sst`+`constants` gates; 79→80 for the
two-module test-object gate; the `particle_anims` ambient-prepend
gate 20→21).

**Aeon gate-org correction folded in.** The `fbb76f9` engine.inc updated
only the `SIGIL_EMP_COLLISION` resume orgs ($31FA/$34B4 → $31F8/$34B2);
the `SIGIL_EMP_COLLISION_LOOKUP` and `SIGIL_EMP_SOUND_API` gate orgs
(used ONLY by the sigil mixed builds) were left at their pre-shrink
+6 values. They were slid the same −6 the whole tail took —
`$4C2C`→`$4C26` / `$5450`→`$544A` (collision_lookup) and `$5F78`→`$5F72`
/ `$7436`→`$7430` (sound_api) — matching where `Collision_ProbeDown` /
`Sound_FadeIn`'s successor land in the gate-off build. This edits only
the dead `else` branches of `ifndef SIGIL_EMP_*`, so the gate-off
reference ROMs are byte-identical (hashes below unchanged, no rebuild).
The debug convsym-rewritten set re-derived empirically to `{$18E, $1A5,
$1A6, $1A7}` (the `$18F` checksum low byte now coincides with the
reference; `$18F` dropped from the four-byte set).

- Aeon repo commit: **`fbb76f9`** (the tranche-7 step-5 fix+perf),
  working tree carries only the two-gate org correction above.
- Non-debug `s4.bin`: sha256
  **`82aac84d49fbb5ad73956bb6b92545c403523b8487f2cea4c14e434172385b9b`**.
- Debug `s4.debug.bin`: sha256
  **`ff897d0b49bb19583788ab5f4e4184081fe0311b4694b08b1e854dd4bbdca4bc`**.

## Re-baseline: tranche-7b — the interact-pointer staleness fix (2026-07-10) — the current pin

Tranche-7 FOLLOW-UP (`collision.asm` + `collision.emp` + `player_sensors.asm`
+ `structs.asm` + `constants.asm`/`constants.emp` in LOCKSTEP; aeon
**`7138ca3`** on branch `collision-interact`, all shapes freshly captured —
do NOT rebuild). The per-player standing-BIT scheme (object-side bits, prone
to going stale when a player walked off between the mid-frame clear and the
animation classifier's ledge probe) is REPLACED by a single engine-owned
pointer: **`SST_interact`**, a new word at **`$4E`** — the tail of the
player-slot custom window (`SST_sst_custom + SST_CUSTOM_SIZE - 2`;
`structs.asm` equ + `<> $4E` guard). `Touch_Solid`'s top-contact now stores
the claimed solid's address there (replacing the 16-byte per-player
standing-bit block, −12 B); `TouchResponse` clears it at pass start
(`clr.w SST_interact(a2)`, +4 B alongside the `bclr #ST_ON_OBJECT`); the
ledge probe in `player_sensors.asm` reads it directly (slot scan deleted,
−30-class). Claim / transfer / walk-off were LIVE-VERIFIED in oracle. The
two `ST_P1/P2_STANDING` bits had NO remaining consumer and were deleted from
both `constants.asm` (tombstone note) and `constants.emp` — **constants twin
20 → 18**.

**collision region: `$16E` → `$166`** (bases UNCHANGED — plain `$308A`,
debug `$3344`; `TouchResponse` head unmoved, the standing-block removal
shrinks the pass-start clear so the `lea (Dynamic_Slots).w, a3` slides from
region offset `0x1C` to **`0x20`** — its abs.w word moves to offset `0x22`).
New `SIGIL_EMP_COLLISION` resume orgs plain **`$31F0`** / debug **`$34AA`**.

**Two-stage tail slide** (`player_sensors.asm` sits inside
`gameEngineBlockIncludes`, between `collision_lookup` and `section.asm`):
- Regions between collision's end and `player_sensors` slid **−8** (collision
  shrink only): `Collision_GetType` (collision_lookup base) plain
  `$4C02`→**`$4BFA`** / debug `$5426`→**`$541E`** (size `$24` held; resume
  orgs / `SIGIL_EMP_COLLISION_LOOKUP` gate plain **`$4C1E`** / debug
  **`$5442`** = `Collision_ProbeDown`), `Tile_Cache_GetCollision` plain
  `$431A`→**`$4312`** / debug `$4A86`→**`$4A7E`**. The cross-seam
  collision-lookup tail `bra.w Tile_Cache_GetCollision` holds its disp
  `$F6FA` plain / `$F642` debug (site + target both slid −8 equally — window
  CONTENT byte-identical).
- Regions AT/after `player_sensors` slid **−36** (collision −8 + the ledge
  probe's scan-to-read shrink): `sound_api` base (`Sound_PostByte`) plain
  `$5D8E`→**`$5D6A`** / debug `$724C`→**`$7228`** (size `$1E4` held;
  `SIGIL_EMP_SOUND_API` gate plain **`$5F4E`** / debug **`$740C`** =
  `Sound_FadeIn`'s successor; `Sound_PlaySFX` = base+`$100` INVARIANT),
  `Sound_DrainSfxRing` plain `$5ED4`→**`$5EB0`** / debug `$7392`→**`$736E`**.
  `game_loop`'s cross-seam `bsr.w` drain (site UNMOVED, target slid) re-derives
  disp plain `$3BD0`→**`$3BAC`** / debug `$5000`→**`$4FDC`**. (The prompt's
  −38 estimate for this stage read −36 in the listings — the ledge-probe scan
  collapsed 28 B, not 30; the listings are truth.)

Regions BEFORE collision (`vdp_init`, `hblank`, `controllers`, `math`,
`game_loop` `$22FE`/`$238C`, `VSync_Wait` `$2262`/`$22EC`) + the vectors are
UNMOVED; everything at/after `org $10000` (`act_descriptor`, `sonic_anims`,
`particle_anims`, the test-object bank, `SongTable` `$63AE0` /
`SongPatchTable` `$63AE4`) is UNMOVED — the −36 is absorbed at the
`org $10000` sound boundary, so assembled `EndOfRom` is UNCHANGED both shapes.
Demo unaffected.

**Guard-count re-derivation** (constants twin 20→18 rippled everywhere it
compiles, and `collision.emp` gained ONE new ensure — `comptime fn
interact_off()` drift-locks `$4E` against `extern("SST_interact")`):
constants-only gates 20→**18** (controllers / vdp_init / collision_lookup),
`collision.emp` 50→**49** (sst 30 + constants 18 + collision.emp's own 1),
`test_particle.emp` 50→**48** (sst 30 + constants 18), `test_objects_port`
80→**78** (sst 30 ×2 + constants 18), `particle_anims.emp` 21→**19**
(constants 18 + its own AF_DELETE prepend guard). `sigil_harness::test_support`:
`engine_constant_equs()` dropped both STANDING entries (20→18 pairs);
`sst_field_equs()` gained a SUPPLY-ONLY `("SST_interact", "$4E")` (31 pairs —
30 guarded + 1 supply so `collision.emp`'s new guard resolves its extern).

**Debug convsym-rewritten set re-derived** to `{$18E, $18F, $1A5, $1A6,
$1A7}` (5 bytes): the collision content change moves the checksum, so `$18F`
DIVERGES again (it had coincided at t7-step5). Plain set UNCHANGED
(`{$18E, $18F, $1A6, $1A7}`).

**Aeon engine.inc gate-org VERIFIED (not edited).** The `7138ca3` engine.inc
already re-derived all three downstream gate orgs this time; cross-checked
against my listing reads and CONSISTENT: `SIGIL_EMP_COLLISION` resume
`$31F0`/`$34AA`, `SIGIL_EMP_COLLISION_LOOKUP` gate `$4C1E`/`$5442`,
`SIGIL_EMP_SOUND_API` gate `$5F4E`/`$740C`.

Full strict workspace (`SIGIL_STRICT_GATE=1`) = **2034 passed / 0 failed**;
`clippy -D warnings` clean; `corpus_bytediff` all-identical (the fix is
test-pin-only — no engine/lowering change).

- Aeon repo commit: **`7138ca3`** (branch `collision-interact`).
- Non-debug `s4.bin`: sha256
  **`e22a82b397525d8021e6facdd4f307ed1886ac7f497c08fc95f19f7182f61f0e`**.
- Debug `s4.debug.bin`: sha256
  **`0c9f1952b50e4bec8f02cf0fb57195c8c73b7ce98a4dcaedb87ae2d9aca6869d`**.

## Tranche 8 — the rings region (2026-07-10) — NEW REGION, reference UNCHANGED

`engine/objects/rings.asm` → `rings.emp` (step-1 transcribe): byte-exact
against the UNCHANGED tranche-7b reference ROMs (pins above still current —
the port adds a gate, no AS-side content changed; gate-off plain build
re-verified `e22a82b3…`).

**rings region (NEW):** plain `s4.bin[$31F0..$33A8]` (0x1B8), debug
`s4.debug.bin[$34AA..$36BE]` (0x214) — the campaign's FIRST
shape-dependent-LENGTH region (the `__DEBUG__` assert block in
`RingBuffer_Add.full` exists only in the debug shape; its FSTRING data is
transliterated `dc.b`, kill-list row 16). `SIGIL_EMP_RINGS` resume orgs:
plain `$33A8`, debug `$36BE` (engine.inc; from the 2026-07-10 listings).

**Guard-count surface:** `engine_constant_equs()` grew the rings/sprites
block 18→**24** (RING_HEIGHT/RING_ANIM_FRAMES/RING_ANIM_SPEED +
MAX_VDP_SPRITES/VDP_SPRITE_X_OFFSET/VDP_SPRITE_Y_OFFSET — the latter three's
truth is `engine/objects/sprites.asm`, kill-list row 17). Every count
assertion is now DERIVED from the shared list (`twin_guards()` — the
tranche-8 back-prop completing tranche 7's shared-list move), so future twin
growth stops breaking counts. `rings.emp` carries FOUR module-local
game-owned mirrors (kill-list row 18): gate total 30+24+4 = **58**.

Full strict workspace (`SIGIL_STRICT_GATE=1`, `AEON_DIR` at the tranche-8
worktree) = **2048 passed / 0 failed**; clippy clean.

- Aeon worktree branch: `sigil-emp-tranche8`.
- Reference sha256 pins UNCHANGED from tranche 7b (above).

## Re-baseline: tranche-8 step-5 — the RingCollision rolling pointer (2026-07-10) — the current pin

Step-5 optimization wave (`rings.asm` + `rings.emp` in LOCKSTEP), all shapes
freshly captured:

**RingCollision rolling entry pointer** — the per-ring `×6 index chain + lea
pair` (~36 c/ring-test) is replaced by ONE `subq.w #6, a3` (8 c): the entry
pointer is computed once per player and decremented per iteration. Correct
across removals because swap-with-last only rewrites the removed slot from an
already-visited HIGHER index (entries below the cursor never move); a3
survives the collect path (all callees clobber d0-d2/a0-a1 only — contracts
verified). ~28 c/ring-test/player/frame at the hot loop; net-ZERO region
bytes (the chain moved out of the loop). **LIVE-VERIFIED in oracle**: draw,
collect, counter, high-water, and swap-with-last removal (twice — including
a MID-BUFFER collect with live entries below the cursor).

**RingBuffer_Remove `lea (aN,dN.w)` → `adda.w dN, aN`** ×2 (−4 B, −4 c each;
matches RingBuffer_Add's existing idiom).

**rings region: `0x1B8/0x214` → `0x1B4/0x210`** (bases unchanged). The −4
shrink slid every downstream engine-block pin (absorbed at `org $10000`):
- rings resume orgs `$33A4`/`$36BA`; collision_lookup gate `$4C1A`/`$543E`;
  sound_api gate `$5F4A`/`$7408` (engine.inc, from listings).
- collision_lookup base `$4BF6`/`$541A`; sound_api base `$5D66`/`$7224`;
  `Tile_Cache_GetCollision` `$430E`/`$4A7A`; `Sound_DrainSfxRing`
  `$5EAC`/`$736A`; `Sound_PlayRing` `$5EFC`/`$73BA`; rings-port labels
  (`Collected_MarkRing` `$3428`/`$37A0` etc.) — all listings-derived.
- RingCollision region offset `0x112`/`0x16E` (RingBuffer_Remove shrank
  ahead of it).
- MDDBG__* UNMOVED (past the `org $10000` boundary), verified from the
  rebuilt debug ROM's own jsr/jmp operands.

Full strict workspace = **2048 passed / 0 failed**; clippy clean.

- Aeon worktree branch: `sigil-emp-tranche8`.
- Non-debug `s4.bin`: sha256
  **`c973091d14c5cb5657f7e900b08584ce876c13da2bb95bc0f4e5f291537aad18`**.
- Debug `s4.debug.bin`: sha256
  **`6a0f9c3f44986916ddd971cbc541cce1b199c6c26b4e16bf27073c28ffaf15d0`**.

## 2026-07-10 — tranche 9 (animate.emp) step-1/step-2 re-baseline

Tranche 9 ported `engine/objects/animate.asm` → `animate.emp`
(`SIGIL_EMP_ANIMATE` gate; region base plain `$2D78` / debug `$3032`,
length shape-INVARIANT — no `__DEBUG__` code).

Step 1 was byte-exact against the tranche-8 pins (`c973091d…`/`6a0f9c3f…`)
at len 0x312, including the AF_* equ re-home animate.asm →
engine/constants.asm (equ moves emit nothing; script data files keep their
truth when the gate strips animate.asm from the AS side).

Step 2 (house format) changed bytes TWICE over:

- `.cc_delete`'s `jmp DeleteObject` (`4EF8`, abs.w) → `jbra DeleteObject`
  relaxing to `bra.w` — the static-tail-call house spelling (jmp reserved
  for computed targets). Length-neutral.
- **The bare-Bcc relaxation found FIVE suboptimal hand widths** (`bhi.w`
  ×2, `bhs.w` ×2, one `bra.w` tail-call — all reached short):
  **region 0x312 → 0x308**, the first time the rule's width-selection
  actually shrank a port (t7/t8 hand widths were optimal). The −10 slid
  every downstream engine pin (absorbed at `org $10000`):
  - gate orgs (engine.inc, from listings): animate resume `$3080`/`$333A`;
    collision resume `$31E6`/`$34A0`; rings resume `$339A`/`$36B0`;
    collision_lookup resume `$4C10`/`$5434`; sound_api resume
    `$5F40`/`$73FE`.
  - region bases: collision `$3080`/`$333A`; rings `$31E6`/`$34A0`;
    collision_lookup `$4BEC`/`$5410`; sound_api `$5D5C`/`$721A`.
  - labels: `Collected_MarkRing` `$341E`/`$3796`; `EntityLoaded_Clear`
    `$362E`/`$3C02`; `EntityWindow_EntryForSection` `$3642`/`$3C78`;
    `Sound_PlaySFX` `$5E5C`/`$731A`; `Sound_DrainSfxRing` `$5EA2`/`$7360`;
    `Sound_PlayRing` `$5EF2`/`$73B0`; `Tile_Cache_GetCollision`
    `$4304`/`$4A70`. `DeleteObject` (upstream) and all RAM cells unmoved;
    `MDDBG__*` unmoved (past `org $10000`).
  - byte-pin arrays: tranche-5's game_loop `bsr.w Sound_DrainSfxRing`
    displacement `$3BA8`→`$3B9E` plain / `$4FD8`→`$4FCE` debug.

The AS twin LOCKSTEP spells the five new widths EXPLICITLY (`.s`,
commented) — the sigil AS front-end deliberately pins branch widths
(no relaxation on the .asm side), so bare Bcc is an `.emp`-only surface;
asl was verified to select the identical widths (same hashes from the
bare and explicit spellings). Pinned exceptions, commented in place: the
two 9-entry `bra.w` dispatch tables (load-bearing 4-byte slots under a
pc-indexed jmp) and the `reload_anim_timer` template's `bne.s`
(byte-locked to the macro twin).

Full strict workspace = **2054 passed / 0 failed**.

- Aeon worktree branch: `sigil-emp-tranche9`.
- Non-debug `s4.bin`: sha256
  **`3b0357ad651152e10886ac6e4d1da3ea457d6acfc52b077a8e80fb9359a55927`**.
- Debug `s4.debug.bin`: sha256
  **`8cd33561714d4fc95ce3508c05df6d90fc6ca54478c1f4a7e9de292bffd272e3`**.

## 2026-07-10 — tranche 9 gate ruling: AnimateSprite_PerFrame DELETED

Volence's gate call: the dead per-frame-duration interpreter (zero callers,
no DUR_DYNAMIC support, ~2× script bytes for the uniform case) is deleted
from BOTH twins; uneven timing is answered by sonic_anims.emp's documented
`rep()` comptime helper (probe-tested in sonic_anims_port.rs) with
AF_DURATION recorded as the fallback design. Region 0x308 → **0x192**
(`$2D78..$2F0A` plain / `$3032..$31C4` debug); the −0x176 slid every
downstream engine pin (second sweep this tranche, all values from
listings). The debug convsym allowlist shrank to the plain set's shape
(the deb2 symbol append lost the PerFrame symbols; `$1A5` matches again).
`export .cc_delete` reverted (its only consumer was PerFrame's table).

Full strict workspace = **2055 passed / 0 failed**, clippy clean.

- Non-debug `s4.bin`: sha256
  **`50f92f57b112966df9ab836cad8971296decab6e6fe8aee2da62b37b51dc9f2c`**.
- Debug `s4.debug.bin`: sha256
  **`1dfe4a4c3767a3ada2a18b5bbc4cb0810d41bb94bc7cfaee29a1a53f56c05edf`**.

## Tranche 10 — core.emp + dplc.emp (2026-07-11, MERGED)

The object-system spine ported (`RunObjects`/`DeleteObject`/pools/DPLC).
Step 0 shipped the `repin` tool (generated `pins.rs`); step 1 transcribed
byte-exact (constants twin 30→34; two shipped language features — imm-link +
one pinned-abs operand, and `FixupKind::ImmWord16Be` = AS's word-immediate
rule); step 2 modernized to house format and took core's −4 shrink
(`bsr.w .run_culled`/`bsr.w Draw_Sprite`→`bsr.s`, both shapes, `.asm` twins
in lockstep), re-pinned via the tool. `org $10000` absorbs the shrink so
`EndOfRom` is UNCHANGED (`$658B4`/`$673A2`); only engine-block-downstream
regions moved −4. RunObjects live-profiled (9.3% frame budget; the empty-slot
occupancy redesign is deferred to its own tranche).

Full strict workspace = **2086 passed / 0 failed**, clippy clean.

- Non-debug `s4.bin`: sha256
  **`15f2d69e428f64b5f5c887fd57364fa06826b636eae2df20efbeff6f1bb4cbed`**.
- Debug `s4.debug.bin`: sha256
  **`2d095a44d7fbb061b39ddc999106e406ab88f823056b46b70cf533c395052cb0`**.

## Object-pool occupancy — dynamic live-list (2026-07-12, MERGED) — the current pin

The tranche-9-class PerFrame-deletion engine-arch item: a word-address live
list for the DYNAMIC pool (`Dynamic_Live[NUM_DYNAMIC]` + count + dirty, RAM
tail — `Engine_RAM_End` grew to `$FFFFB044` plain / `$FFFFB066` debug, zero
ripple to existing RAM). Walkers (`RunObjects` `.run_culled` / `_Frozen`
dynamic segment, `TouchResponse`, `EntityWindow_DespawnObjects`) walk the live
list in SPAWN order instead of the fixed 40/66-slot sweeps; AllocDynamic
appends, DeleteObject flags dirty + A1-zeroes its entry (duplicate-free under
same-frame LIFO recycle), frame-end `CompactDynamicLive` reconciles. DEBUG-only
§6 invariant asserts (self-gating — the plain shape carries ZERO of them).
Spawn-order dispatch is Volence's ruling (§3a); code_addr stays the single
truth. Built as spec build-order steps 1-8 + amendment A1.

Region growth (all absorbed at `org $10000` — assembled `EndOfRom` UNCHANGED
both shapes at `$65A94` / `$67582`, = the tranche-10 pin):
- PLAIN: core +0x22 (step 1 structure) +0x8 (step 2) +0x6A (A1) +0x2A (step 3)
  net through step 6's +0x8 compaction call; entity_window +0x8 (step 5).
  Step 7's DEBUG asserts add ZERO plain bytes (self-gate) — the plain ROM is
  byte-identical whether or not step 7 is present. Every downstream engine
  region (sprites/animate/collision/rings/collision_lookup/sound_api) slid by
  the cumulative plain growth; the tranche-5 game_loop `bsr.w Sound_DrainSfxRing`
  disp tracked its target.
- DEBUG: the same, PLUS step 7's +0x19E of self-gating asserts in core — so the
  debug engine regions slid further than plain, and the `bsr CompactDynamicLive`
  at the RunObjects tail is `bsr.w` in debug / `bsr.s` in plain (jbsr auto-selects
  per shape).

All harness pins re-derived via `cargo run -p sigil-harness --bin repin` +
hand-typed baselines (`repin_pins.rs`), engine.inc resume orgs (7 regions ×
both shapes), and the tranche5 disp. Full strict workspace
(`SIGIL_STRICT_GATE=1`) = **2208 passed / 0 failed**; clippy clean. Live-verified
in oracle: null-guard walk, forced-despawn cursor survival, compact-on-full
a1 survival, self-cleaning compaction (Count == true live count), DEBUG asserts
never fire (error-handler hits 0). Profiler (caching fix confirmed live via
jitter check): **RunObjects 11,841 cyc (9.3%) → 2,428 cyc (1.9%), −79.5%** in
the light OJZScroll scene — the empty-slot tax eliminated (packet:
`notes/2026-07-12-object-pool-occupancy-profile-packet.md`).

- Aeon repo commit: **`f64ebf7`** (merge of `object-pool-occupancy`), working
  tree clean apart from an untracked concurrent doc; sigil merge `fdf8d36`.
- Non-debug `s4.bin`: **451861 bytes** (assembled `EndOfRom` = `0x65A94`,
  unchanged), sha256
  **`514361b743af4a04b8d5b38be74c15d1affd6906b6cf2d883611172a4e9be0e7`**.
- Debug `s4.debug.bin`: **459735 bytes** (assembled `EndOfRom` = `0x67582`,
  unchanged), sha256
  **`0f03dd2e87ce1f4aeda4f2385aa8581701e84934d9ef3fa860ef2fe0b89e3cc0`**.

## Re-baseline 2026-07-12 — tranche 12 (entity_window.asm → .emp)

The 12th code port modernizes `entity_window.asm` in lockstep with its `.emp`
port (steps 1-5): control flow → jbsr/jbra/bare-Bcc; the Init→Scan tail branch
DELETED (fall-through via `falls_into EntityWindow_Scan`); the DEBUG-conditional-
width branches hand-set per-shape (`ifdef __DEBUG__ .w / else .s`; `197`'s bsr
stays `.w`); 4 backward-near `bsr.w`→`bsr.s`; `clear_slot_bitmasks` comptime-fn.
entity_window shrank **-0x1C plain / -0xC debug**; collision_lookup + sound_api
slid; engine.inc resume orgs re-pinned; the `SIGIL_EMP_ENTITY_WINDOW` gate wired
(resume orgs plain `$3C5A` / debug `$4570`).

Region growth absorbed at `org $10000` — assembled `EndOfRom` UNCHANGED both
shapes (`$65A94` / `$67582`); ROM lengths unchanged, engine-block CONTENT changed
(hence the new hashes). Harness pins re-derived via `repin` + hand-typed
baselines; the mixed_dac_rom game_loop `bsr.w Sound_DrainSfxRing` disp is now
pin-spliced (survives future target slides).

- Aeon repo commit: **`2751a27`** (merge of `port-tranche12`) + **`281198f`**
  (gate-wiring integration); sigil merge **`e2ad6d7`**.
- Non-debug `s4.bin`: **451861 bytes** (`EndOfRom` = `0x65A94`, unchanged), sha256
  **`e55a010ce6470f3f4caca8c51cad9b795888fb9f39b32417ea87c79dff760046`**.
- Debug `s4.debug.bin`: **459735 bytes** (`EndOfRom` = `0x67582`, unchanged), sha256
  **`6c21b56cb2f68390c94fbcc548faf307e23f32981af4c292fe0e233f375708e5`**.

## Re-baseline 2026-07-12 — retro-fix-audit-1 (steps-2-5 audit batch)

The ratified retro-fix batch off the 2026-07-12 steps-2-5 retro-audit. Three
items change ROM BYTES (plain + debug); the rest are DEBUG-shape-only asserts
(self-gating) or comment/ensure (byte-neutral in both shapes):

- **item 5** (animate): drop both `Sound_PlaySFX` movem saves (exhaustive-license
  ruling — a1/d1 contractually preserved). −8 B.
- **item 10** (rings): DrawRings camera-bias fold (A1 class) — pre-bias the cached
  camera regs, drop the per-ring subi/addi, compensate the cull addi immediate to
  reproduce the EXACT pre-fold d0 (wraparound-safe). SAT bytes identical; net
  −6 B (the cull addi compensates, can't be dropped — hence −6 not the audit's
  −16).
- **item 11** (dplc + dma_queue): `QueueDMATransfer` now HONORS its long-documented
  carry-on-full contract (+12 B, 3 CCR ops — shifts the whole engine block +0xC);
  `perform_dplc` commits `prev_frame` only after a successful enqueue.
- **item 6 REMOVED**: the DPLC single-entry assert — the A2 ObjectTest oracle soak
  disproved the invariant (`DPLC_Sonic` carries multi-entry frames), so it was
  removed (debug-shape shrinks back).

Engine-block growth absorbed at `org $10000` — assembled `EndOfRom` UNCHANGED both
shapes (`$65A94` / `$67582`). engine.inc's 14 `SIGIL_EMP_*` gate resume orgs
re-derived; the mixed_dac_rom pre-pin hardcoded engine-block addresses bumped
+0xC; harness pins re-derived via `repin` + hand-typed baselines. New RAM
symbol `Dynamic_Live_Walking` (DEBUG-only, reuses the ram.asm pad → Engine_RAM_End
shape-invariant). The debug `s4.debug.bin` file grew +18 B (the added DEBUG
symbols in the post-`convsym -a` table; body up to `EndOfRom` unchanged length).

- Aeon repo commit: **`5e946ca`** (merge of `retro-fix-audit-1`); sigil merge
  **`a17e0b7`**.
- Non-debug `s4.bin`: **451861 bytes** (`EndOfRom` = `0x65A94`, unchanged), sha256
  **`65c3681cc7118fc120332894a0404090784192911afe72456ba26a75e1eb4013`**.
- Debug `s4.debug.bin`: **459753 bytes** (`EndOfRom` = `0x67582`, unchanged), sha256
  **`7d24abbf2de46a1f5941e33a543308be79683f755f943e912897c69299d911e2`**.
