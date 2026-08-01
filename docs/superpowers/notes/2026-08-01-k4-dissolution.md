# Parcel K4 — the skeleton dissolution (in progress)

**Porter: Opus (k4-dissolution branch, aeon + sigil worktree). Overseer/spec owner: Fable.**
Spec: `specs/2026-08-01-k-capstone-design.md` §2 (K4) + §0 end-state. Survey:
`notes/2026-08-01-k-capstone-survey.md` §2/§3/§4/§8.

This is the largest K parcel. Committed incrementally; honest partials. Increments
1–2 DONE below; the skeleton dissolution proper (engine.inc + main.asm ×2 + the 4
remaining BINCLUDE islands) is inventoried in §3 as the remaining work.

## Increment 1 — `engine/macros.asm` DELETED (byte-neutral)

The object/VDP helper library had **zero live residual-AS consumers** (grep-proof
over all 25 helpers; the last two, objentry/objend, died with the K3 run-A
entity_data port; objdef/objroutine appear only in comments). Deleted;
`engine.inc` + the `m1c_root.asm` test harness drop the `include`. **Six targets
byte-identical** (48049c03/…). Census #4 → DELETED.

## Increment 2 — `engine/system/header.inc` → `games/*/config/header.emp` (appendix re-freeze)

The `$100-$1FF` ROM header (gameHeader macro) is now a native `.emp` section,
per-game (`games.sonic4.header` / `games.demo.header`):

- **The nine `strlen … fatal` walls became the field TYPES.** Each game-declared
  string is a `[u8; N]` struct field — a wrong-length string fails to lower with
  `array length mismatch: expected N, got M` (proven: too-short AND too-long both
  error). The type IS the width assertion, checked at COMPTIME — the correctness
  hardening the campaign prefers.
- **Structure:** a `HeaderTop` struct ($100-$18D) + `pub data Checksum: u16` ($18E,
  kept a real boundary label — the derive guard requires it to resolve) + a
  `_header_tail: HeaderTail` ($190-$1FF). `rom_end` = `extern("EndOfRom") - 1`.
- **The GAME_* strings moved from game.asm to header.emp** (only gameHeader read
  them — grep-proof); game.asm keeps its irreducible -D/macros/equalates.
- engine.inc drops `include header.inc` + the `gameHeader` invocation (org $100
  inert); `header.inc` DELETED. Registry: `games.sonic4.header` (registry) +
  `games.demo.header` (demo_registry); pin HEADER ($100/0x100); repin.toml region
  (start GameHeader, end EntryPoint); map.toml order Checksum→GameHeader ×2;
  golden-gate `header_port` (both games, the sigil-patched $18E/$1A4 cells excluded).

**Identity: appendix-only re-freeze — assembled ANCHOR identical ×6.** The header
content is byte-identical (strings match, $18E/$1A4 are sigil-patched by address —
fixheader left the pipeline). The label set shifted (Checksum stays; GameHeader +
_header_tail enter), so the deb2 appendix + full CRCs move. `refreeze --freeze
k4-header` (no `--ab` — anchors did not move) appended provenance chain entry
**#13**; all six goldens re-frozen; the 6 frozen tables re-derived (GameHeader is
the header head); `repin` 0 changed.

| target | anchor (UNCHANGED) | full_crc (was → now) |
|---|---|---|
| s4 | `658c623b` | 48049c03 → **f1b78974** |
| s4_debug | `b137c411` | 4b0fc1bc → **cbeeec69** |
| demo | `e2dc9207` | fc072efe → **55b70266** |
| demo_debug | `b7d81931` | 9f735dc3 → **6487a47c** |
| config_a | `19b793ec` | 76ade14c → **0c08425d** |
| config_b | `8b490cfb` | 38215ead → **947e4c57** |

Gates: strict **2893/0/4** (baseline 2891 + 2 header_port); `refreeze --check` OK
(tip k4-header, chain 13); `repin --check` clean; K1 order-validation green.

Two `.emp` mechanics recorded: (1) `[u8; N]` string data is exact-width (the header
guard); (2) the derive_offcanon guard rejects dropping a committed boundary label —
Checksum had to stay a real section LABEL (a `pub equ` alias does NOT satisfy it),
which is why the header splits at $18E rather than being one struct.

## Increment 3a — the collision/character island → `.emp` (+ the DAC anchor)

The flat BINCLUDE island at the tail of sonic4/main.asm's gameDataIncludes
(HeightMaps / HeightMapsRot / AngleTable / SolidityTable / Map_Sonic / DPLC_Sonic /
Art_Sonic) is now `games.sonic4.collision_data` — 7 `embed()`s via the `const embed
→ ensure(.len) → pub data = const` pattern; the two Map_Sonic/DPLC_Sonic word-offset
walls became comptime `ensure`s. Boundary key HeightMaps (existed); pin
COLLISION_DATA (literal len 0x1C480 — the blobs are fixed-size, no end label at the
$8000-pad-to-DAC gap); golden-gate `collision_data_port`.

**The coupling finding (the reason this island isn't isolated):** the collision
island used to FILL the AS stream up to $41BDA, so the DAC bank's `align $8000`
landed at $48000. With collision native, the AS cursor before that align is at
$25752, so the relative align would land the DAC early — the label-less DAC section
provisioned at $25752 with a 76 KB image and overran particle_anims (proven with a
packer trace). Fix: the DAC blip `align $8000` → **absolute `org $48000`** (the
bank's fixed Z80-SetBank LMA), which makes it an explicit ANCHOR — declared as
`[[anchor]] dac_blip_bank at=0x48000 when="sound_on"` in the map (dac_shared packs
contiguously after it, needs none). Its head `Dac_Temp_Blip` joins the map order.

**Identity: appendix-only re-freeze — anchors identical ×6.** `refreeze --freeze
k4-collision` (no `--ab`), chain entry **#14**; demo/demo_debug/config_b full CRCs
UNCHANGED (demo has no collision; config_b's appendix coincides), s4/s4_debug/
config_a re-frozen (collision labels re-home AS→`.emp`). Strict **2895/0/4** (+2
collision_data_port); refreeze/repin --check clean; K1 order green.

## §3 — Remaining work (the skeleton dissolution proper) — INVENTORY

engine.inc + the two main.asm files still carry these, counted against the CURRENT
tree (the survey was pre-K2/K3):

**engine.inc (441 ln)** — includes `sound_bank.inc` (74, data island), `debugger.asm`
(81, VENDORED-KEEP), invokes the 7 game*Includes macros + the org ladder (66 orgs,
inert). **sonic4/main.asm (357 ln)** — `include game.asm` (SURVIVES) + 6 BINCLUDE
islands: the collision island (heightmaps/heightmaps_rot/angles/solidity +
sonic map/dplc/art, 7 BINCLUDEs, 183-208), the DAC banks (dac_blip/dac_shared,
phase bank, 230/234), the MT bank (+ mt_syms, 291-295), the SFX bank (322/324) +
`include engine.inc`. **demo/main.asm (59 ln)** — `include game.asm` + `include
engine.inc`.

**The remaining ports, in the coordinator's order:**
1. **sound_bank.inc → `.emp`** — the $8000-window phase-bank head: 5 `embed()`s
   (SoundTablesZ80_Head / MovingTrucks_PitchTable / SfxBlobWinTab / SeqOpcodeTable /
   DacSampleTable) at fixed VMAs, each span-wall → `ensure`. A native phase-bank
   section (vma_base 0x8000). Golden-extraction per block.
2. **The 4 main.asm BINCLUDE islands** (collision / DAC / MT / SFX) → native `embed()`
   sections — the collision island is the tractable one (7 flat embeds); DAC/MT/SFX
   are phase banks.
3. **engine.inc + main.asm ×2 DELETE** — every surviving include native. **Design
   point (coordinator #4):** game.asm (SURVIVES) is `include`d at main.asm:20 via
   gameConfigIncludes; deleting main.asm strands its include site. The kill-row-9/45
   matrices re-extract game.asm's macro bodies every run, so game.asm must keep
   assembling exactly as today — the dissolution needs a mechanism (a minimal AS
   entry that includes game.asm + debugger.asm, or a harvest of the macro bodies).
   **EndOfRom (engine.inc:425)** — the ROM terminus label the header + others read —
   must relocate (a native epilogue label) when engine.inc goes.

**Status: increments 1–2 committed; the §3 work (sound_bank + 4 islands + the
skeleton delete with the game.asm/EndOfRom design points) remains — multi-session,
each island a run-A/B-shaped native port + the two skeleton design points to STOP
on with options if they resist.**
