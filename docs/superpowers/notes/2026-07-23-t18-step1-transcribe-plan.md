# t18 — parallax step-1 transcribe: RESOLVED EXECUTION PLAN

Gate PASS'd step-0 and authorized step 1. This note records the fully-resolved
transcribe idioms (every one cross-checked against a shipped ported file) so the
transcribe + wiring + byte-gate executes mechanically and verifiably. NO code cut
yet in this file; parallax.emp does not exist.

## File scaffold (mirrors plane_buffer.emp / section.emp)

```
module engine.parallax in parallax          // section name = parallax

use engine.constants.{SECTION_SIZE_SHIFT}   // proven-shared import
use engine.structs.{Act, Sec}               // Act.act_parallax_config($16) / Sec.sec_parallax_config($14) confirmed present
use engine.vdp.{VdpTarget, VdpOp, vdp_comm}
```

## Const mirrors + drift guards (parallax-specific consts not in the shared twin)

`SCREEN_WIDTH=320`, `SCREEN_HEIGHT=224`, `MAX_PARALLAX_BANDS=8`,
`PARALLAX_TRANS_DEFAULT=16`, `PARALLAX_LERP_SHIFT=4` — each `const` + `ensure(extern("X")==X, "...")`.
(engine/system/constants.emp is the shared twin; SECTION_SIZE_SHIFT already lives
there and is imported. The rest mirror-local per the byte-isolation idiom; hoist to
the shared twin when a 2nd .emp consumer appears.)

VDP addressing (mirror like plane_buffer/section):
```
const VDP_DATA = $C00000
const VDP_CTRL = $C00004
const VDP_DATA_OFF = VDP_DATA - VDP_CTRL    // = -4; a5 = VDP_CTRL, so -4(a5) hits VDP_DATA
ensure(extern("VDP_DATA")==VDP_DATA,...) ; ensure(extern("VDP_CTRL")==VDP_CTRL,...)
```

## parallax-private ROM data structs (declared LOCAL — not shared; only parallax uses them)

`struct band_entry` (10 bytes) + `struct parallax_config` (28-byte header) declared
in parallax.emp, field order/sizes copied from structs.asm:166-205. Access sugar:
`band_entry.band_top_cell(a1)`, `parallax_config.pcfg_band_count(a0)` (→ AS twin
symbols `band_entry_band_top_cell` / `parallax_config_pcfg_band_count`).
- **Lengths:** use `sizeof(band_entry)` / `sizeof(parallax_config)` directly for
  `#band_entry_len` / `parallax_config_len(a0)`; `ensure(sizeof(band_entry)==extern("band_entry_len"))`
  + `ensure(sizeof(parallax_config)==extern("parallax_config_len"))`.
- **field-in-disp** (`band_top_cell+band_entry_len(a1)`, peek next band): the
  `.field`-in-disp sugar does NOT compose in displacement arithmetic (step-2 item 5)
  → spell as `(offsetof(band_entry, band_top_cell) + sizeof(band_entry))(a1)`.
- **Drift wall:** per-field `ensure(offsetof(struct, field)==extern("struct_field"))`
  for every field (mirrors structs.emp's discipline; guards the local decl vs
  structs.asm truth).

## setVDPReg (donor AS macro, engine/macros.asm:251) — INLINE at step 1

Expansion of `setVDPReg VDP_Shadow_vdp_mode3, d0`:
```
move.b  d0, VDP_Shadow_Table + VDP_MODE3_OFF   // (VDP_Shadow_Table+$0B).w
ori.l   #(1 << VDP_MODE3_OFF), VDP_Dirty_Mask   // (VDP_Dirty_Mask).w
```
with `const VDP_MODE3_OFF = $0B` + `ensure(VDP_MODE3_OFF==extern("VDP_Shadow_vdp_mode3"))`.
`VDP_Shadow_Table` / `VDP_Dirty_Mask` are bare RAM externs (abs.w by width rule).
**`set_vdp_reg` comptime-fn counterpart = STEP-4 macro-port build** (demand: parallax
1 site + trampoline 2 sites [reg $00 IE1, reg $0A counter] = 3; first-consumer duty
lands the typed interface once, all 3 sites sweep). Not built at step 1 — inline is
byte-identical; the interface design belongs in the construct pass.

## RAM-span immediate (Parallax_Init clear)

`moveq #(Parallax_State_End-Parallax_State)/4-1, d0` →
`moveq #(extern("Parallax_State_End")-extern("Parallax_State"))/4-1, d0`
(core.emp:53 is the precedent — it uses `move.w` for a larger span; parallax's
span ≈30 fits imm8, so `moveq` should hold; **the byte gate is the arbiter** — if
the link-time extern-diff can't encode as moveq imm8, that's a demanded imm8-defer
finding, resolve at the gate).

## Faithful-transcribe rules (step 1 = FAITHFUL, NOT modernized)

- Widths preserved EXACTLY: `bra.w`/`bra.s`/`bsr.w`/`bsr.s`/`beq.s`/`bne.s`/`blo.s`/
  `bhs.s`/`bhi.s`/`bgt.s`/`ble.s`/`bge.s` stay as-is. Bare-Bcc + `jbra`/`jbsr` = step 2.
- Fall-through chain preserved: `Parallax_Update` → (`bra.w Parallax_Step5_Vscroll`)
  → (`bra.w Parallax_Step4_Fill`); Step 5 runs before Step 4 by design.
- Comments carried verbatim (codename cleanup `T6 stub`/`T8+`/`T12` = step 2).
- RAM abs.w → bare symbol + `// (X).w` comment.
- VDP: `#vdpComm(0, VSRAM, WRITE)` → `#vdp_comm(0, VdpTarget.Vsram, VdpOp.Write)`.
- Cross-module: `jsr Section_GetSecPtrXY` kept as `jsr` (faithful; jbsr = step 2).
  section.emp OWNS the symbol (already ported) → normal cross-.emp link, NOT an
  ownership flip. Step-1 gate exercises the real mixed-link path.
- `rept 20 { move.l (a0)+, VDP_DATA_OFF(a5) }` (Vscroll_Write per-column): no .emp
  `rept` primitive exists → write 20 explicit lines at step 1 (byte-identical);
  **`rep`/unroll construct = step-4 ask** (construct inventory names it; no builder yet).
- GridX/GridY bless at CheckBoundary = **STEP 2** (item-6), NOT step 1.

## Wiring (RESOLVED — concrete inventory, mirroring plane_buffer tranche 17)

**Net site-list to touch (delta vs plane_buffer):**
1. **New** `aeon-t18/engine/level/parallax.emp` — `module engine.parallax in parallax`.
2. **Edit `aeon-t18/engine/engine.inc:384`** — the bare `include "engine/level/parallax.asm"`
   (right after camera.asm :383, before load_art.asm :385) becomes the 3-arm gate:
   `ifndef SIGIL_EMP_PARALLAX / include "…parallax.asm" / else / <comment + shape
   note> / ifdef __DEBUG__ / org <debug resume> / else / org <plain resume> / endif /
   endif`. **Resume-org = load_art.asm's first placed symbol address, per shape**
   (read from `aeon-t18/s4.lst` + `s4.debug.lst` — the region END = next region start).
   Carry the "sonic4-shape addresses — never set the gate for other games" NOTE.
3. **Edit `sigil-t18/crates/sigil-harness/repin.toml`** — new `[[region]] name="parallax"
   start=<first proc, `Parallax_Init`> end=<load_art first symbol> gate="SIGIL_EMP_PARALLAX"
   tests=["parallax_port"]` (symbol `end`, next-placement idiom — NOT a len literal).
   Add `[[symbol]]` entries ONLY for NEW cross-seam labels parallax reads that aren't
   already pinned (Section_GetSecPtrXY is already a section symbol; Sec/Act via twins;
   Camera_X/Y/Current_Act_Ptr already pinned under section_port — audit at wire time).
4. **Regenerate `sigil-t18/crates/sigil-harness/src/pins.rs`** via
   `cargo run -p sigil-harness --bin repin -- --aeon /home/volence/sonic_hacks/aeon-t18`
   (adds `pub const PARALLAX: Region`; do NOT hand-edit; it reads aeon-t18/s4.lst +
   s4.debug.lst; prints the engine.inc org paste-block for hand-pasting into site 2).
5. **Edit `sigil-t18/crates/sigil-harness/tests/repin_pins.rs`** — extend the hand-typed
   baselines (`generated_pins_match_the_hand_typed_baseline` +
   `secondary_pin_classes_match_the_hand_typed_baseline`) to the new PARALLAX pins.
6. **New `sigil-t18/crates/sigil-cli/tests/parallax_port.rs`** — mirror
   `plane_buffer_port.rs`: `region_base/len` from `pins::PARALLAX`; `map_toml`;
   `parallax_value_equs` (the .emp's mirrored consts + engine_constant_equs +
   act_sec_field_equs); `parallax_addr_labels` (cross-seam RAM carriers at per-shape
   VMAs); `compile_real_file` PREPENDS `system/constants.emp` + `structs.emp` +
   `vdp.emp`; plain + debug `#[test]` byte gates; a drift-guard negative probe
   (doctor a const → fires naming it); a **standard cross-module link test** compiling
   parallax.emp + section.emp together resolving `Section_GetSecPtrXY` (NOT a
   two-module ownership flip — parallax is a new caller, section already owns it).
7. **parallax.asm → kill-list row 5** (gate-off twin) same commit.

**CORRECTION to step-0 note §8:** parallax does **NOT** touch `mixed_dac_rom.rs`.
That whole-ROM composition tops out at **tranche 9** and contains NO engine/level
port (plane_buffer/tile_cache/section are all absent — their mixed acceptance is the
per-port cross-module test, not a tranche entry). So the ripple is pins.rs +
repin_pins.rs + engine.inc + repin.toml — NOT mixed_dac_rom. (The 5-site ripple's
mixed_dac_rom leg applies to files that ARE in that composition; engine/level ports
aren't, yet.)

**build.sh:** no change — it passes no `SIGIL_EMP_*` defines; parallax.asm stays
included via the gate's `ifndef` arm in the normal build.

## Strict commands (RESOLVED)

- Per-port: `SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/aeon-t18 cargo test -p sigil-cli --test parallax_port`
- Full paired strict (commit boundary): `SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/aeon-t18 cargo test` (workspace) — **AEON_DIR pinned to aeon-t18, never master** (§D phantom-red lesson).

## Verification gates (step-1 artifact discipline — every gate names its test/commit)

1. Region byte-identity BOTH shapes (plain + debug) — `parallax_port` test.
2. Mixed-build acceptance (real sigil-link resolves Section_GetSecPtrXY across the seam).
3. Gate-off neutrality — CRCs `ab787bd1`/421122 · `6a19669f`/429165 UNCHANGED with
   the gate off (parallax.asm path).
4. Negative probe (doctored offset/width fails naming the site).
5. Full paired strict failures-first, **AEON_DIR pinned to aeon-t18** (branch tree).

**Status: plan resolved; transcribe + wiring + gate is the next mechanical push
(awaiting the wiring-inventory subagent). This is a step-1 that must land VERIFIED
— byte gates green — not as a raw draft.**
