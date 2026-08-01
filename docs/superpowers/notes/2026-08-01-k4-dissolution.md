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

## §4 — Inc-4 design points (the skeleton-deletion prerequisites) — RULING NEEDED

Deleting `games/sonic4/main.asm` + `games/demo/main.asm` + `engine/engine.inc`
strands two things that must survive. Each below: the mechanisms, trade-offs, my
recommendation, and the kill-row-9/45 impact. **These want a ruling BEFORE the
deletion work (inc-4) starts.**

### The architecture as-is
`main.asm` DEFINES the 8 `game*Includes` macros (one of which, `gameConfigIncludes`,
`include`s `game.asm`) + the collision/sound BINCLUDE content, then `include`s
`engine.inc`, then `END`. `engine.inc` INVOKES those 8 macros at the org-ladder
positions, `include`s `debugger.asm` (VENDORED-KEEP) + `sound_bank.inc`, defines
`EndOfRom` + the 3 epilogue walls, and is the file the sigil AS frontend assembles
as the root. So `main.asm`+`engine.inc` are the AS root + the ROM-layout owner.

### Design point A — game.asm's include site (kill-row-9/45 depend on it)

game.asm SURVIVES (the -D `GAME_CAMERA_JUMP_LOCK`, the `Game_Entry`/`GAME_ENTRY_ID`
equalates, and the `gameBootHook`/`gameDebugTick` macro BODIES). `game_loop_port.rs`
(kill row 9) + `boot_port.rs` (kill row 45) RE-EXTRACT those macro bodies from the
REAL game.asm every run and byte-diff their AS EXPANSION across the four
SOUND_DEBUG_HOTKEYS×SOUND_DRIVER_ENABLED combos. So game.asm must keep assembling
through `sigil-frontend-as` EXACTLY as today — a mechanism that changes its text,
its define environment, or its macro-expansion is a kill-row break.

- **A1 — a minimal AS root stub (`games/<g>/game_root.asm`, ~5 lines):** `include
  game.asm` + `include debugger.asm` + `END`, with the harvested -D defines the
  build already passes. Deletes main.asm+engine.inc; game.asm's text + expansion are
  UNTOUCHED (kill-rows unaffected). The stub is a NAMED survivor beside game.asm,
  documented against the game-contract-hook ledger row. **Cost:** one tiny AS file
  survives per game (not zero-AS, but the spec §0 already names game.asm + debugger
  as survivors — the stub is their loader).
- **A2 — game.asm becomes the AS root itself** (append `include debugger.asm` + `END`
  to it). Zero extra files. **Cost:** game.asm gains non-contract lines (the debugger
  include + END); the kill-row extractors parse it fine (they target the macro
  bodies, not the whole file), but game.asm stops being purely the contract — a
  readability regression, and the demo/sonic4 game.asm files diverge structurally
  from "just the contract."
- **A3 — harvest the macro bodies + drop game.asm from the AS entirely:** the
  gameBootHook/gameDebugTick expansions become `.emp` (game_loop.emp already MIRRORS
  gameDebugTick). **Cost:** BLOCKED — this IS the game-contract-hook construct the
  spec §4 explicitly defers to the language round; and the kill-row tests re-extract
  from the AS game.asm, so removing it deletes their witness. Not available in K4.

**Recommendation: A1** (the minimal AS root stub). It's the least-surprise path: the
spec already blesses game.asm + debugger.asm as survivors, so a 5-line loader for
them is in-scope; game.asm's text is byte-untouched so kill-rows 9/45 stay green by
construction; and it cleanly separates "the game contract" (game.asm) from "the AS
entry" (game_root.asm). A2 works but muddies game.asm; A3 waits for the language round.

### Design point B — EndOfRom relocation

`EndOfRom` (engine.inc:425) is the ROM terminus label + 3 walls (`EndOfRom & 1`,
`EndOfRom > $3FFFFF`, the plane-cells wall). Consumers: header.emp's `rom_end =
extern("EndOfRom") - 1` (K4 inc-2) — a cross-seam link ref. It is NOT in the
kill-row matrices.

- **B1 — a native epilogue `.emp` module** (`engine.epilogue in epilogue`) that
  emits `pub data EndOfRom = Data.empty` (a zero-length terminus label at the ROM
  end) + the 3 walls as comptime `ensure`s. Placed LAST by the map order (its
  boundary key is EndOfRom). header.emp's extern resolves against it unchanged.
  **Cost:** a new native section whose placement must land at the true image end —
  but "last section" is exactly what the packer already computes; EndOfRom is
  already a frozen boundary key in all 6 tables, so the pin exists.
- **B2 — the map/linker owns the terminus** (emit_rom exposes the image-end as a
  synthetic `EndOfRom` link symbol). **Cost:** more machinery (a linker feature) for
  one label; and the `EndOfRom > $3FFFFF` / evenness walls would move from a comptime
  ensure to a linker check — a reasonable home, but a bigger change than B1.
- **B3 — EndOfRom stays in the A1 root stub** (`EndOfRom: ` + the walls, after the
  native content). **Cost:** couples the epilogue to the AS stub; the stub would need
  an org/position for EndOfRom to land at the true end, reintroducing an org the
  dissolution is trying to retire.

**Recommendation: B1** (the native epilogue module). EndOfRom is already a frozen
boundary key, the packer already places a last section, header.emp already resolves
it as an extern, and the walls become the comptime ensures the campaign prefers. B1
is the natural `.emp` home; B2 over-builds; B3 keeps an org alive.

**Kill-row-9/45 impact summary:** A1 + B1 leave game.asm's text byte-UNCHANGED and
touch nothing the extractors read — the combo matrices stay green by construction.
The only kill-row risk in inc-4 is if the mechanism alters game.asm's define
environment (the -D set passed when it assembles); A1 preserves it exactly (the same
harvested defines the current build passes to the AS root).

## §5 — Inc-4 STOP finding: the sound-bank native placement is a seam-2 undertaking

**STOP (not a hack, not a fatigued gamble on the "ANY anchor move = STOP" surface).**
Grounded investigation of the sound-bank islands (the coordinator's inc-4 item 1):

### The reality (why this is not a run-A/B-shaped embed)
The collision island (inc-3) was tractable because its `.bin` are COMMITTED and it is
flat, non-phased data — a plain `embed()` section. The sound banks are NOTHING like
that:

1. **The `.bin` are gitignored BUILD ARTIFACTS.** `dac_blip_bank.bin` /
   `dac_shared_bank.bin` / `mt_bank.bin` / `sfx_bank.bin` / `sound_tables_z80.bin`
   (+ the pitchtable/seq/dac-sample-tab/sfx-win-tab heads) are all
   `git check-ignore`-confirmed, regenerated every build by `ensure_generated()`
   (seam1 `emit_sound_blob` + seam2 `emit_{dac,mt,sfx,seq_opcode,sound_tables}_artifacts`).
2. **The composing `.emp` modules are NOT in the main `registry()`.** `dac_samples.emp`
   / `mt_bank.emp` / `sfx_bank.emp` / `sound_tables_z80.emp` are lowered by a SEPARATE
   pipeline (`seam2.rs`) into their own one-region maps (`dac_blip_bank` @ 0x48000,
   `dac_shared_bank` @ 0x50000, …) and WRITTEN to `.bin`. The main native build then
   AS-BINCLUDEs those `.bin`. This is the shipped "emit-tool architecture" (MEMORY:
   the entire sound stack is sigil-native via emit-to-`.bin`, ~10k `.asm` deleted).
3. **Phase banks + Z80 pointers.** SoundTablesZ80_Head/MovingTrucks_PitchTable/
   SfxBlobWinTab/SeqOpcodeTable/DacSampleTable sit at FIXED VMAs ($8000/$8357/$845F/
   $856D/$85AD, LMA $58000+) inside the `phase 08000h` bracket; the resident Z80
   driver holds banked carriers at those exact addresses. The map already anchors
   `sound_bank` @ 0x58000 vma 0x8000. The B-0 bank-anchor rule is ARMED: a labeled
   phase-bank head must NEVER repack (the mt-gate catch class).
4. **DSM-harness STUB arms.** `SIGIL_EMP_{DAC,MT,SFX}_BODY_STUB` compose the banks
   IN-MEMORY for the mixed harness (`mixed_dac_rom` / `dac_port`/`mt_port`/`sfx_port`);
   the real build takes the BINCLUDE arm. A native bank section would double-place
   against the STUB composition — the harness path must be reconciled, not just the
   BINCLUDE removed.

### The two native-placement paths (both real, both risky)
- **P1 — move the seam-2 `.emp` modules into the main `registry()`** (native-placed
  phase-bank sections with vma_base 0x8000, replacing emit-to-`.bin`+BINCLUDE). The
  cleanest END-STATE (true native), but it REWORKS the emit-tool architecture: the
  seam-2 `emit_*_artifacts` pipeline, the STUB arms, the `mixed_*_rom` harness tests,
  and `ensure_generated` all change. Largest blast radius on the most fragile stack.
- **P2 — native sections that `embed()` the seam-2-emitted `.bin`** (circular but
  smaller): keep `ensure_generated` emitting the `.bin`, add native `embed()` sections
  placing them at the bank anchors, delete the BINCLUDE arms. Less invasive, but (a)
  the golden gate can't rely on committed `.bin` (they're emitted, so the port test
  must run emit first or gate at build level), and (b) the STUB arms still need
  reconciling (the native section vs. the harness in-memory composition).

### Recommendation + the dependency it creates
**Recommend P2 first for the DAC** (non-phased, an inc-3 anchor already exists) as the
lowest-risk probe, THEN the phased MT/SFX/soundBankHead — but ALL of it wants a
DEDICATED, fresh pass with the seam-2 harness in full view, not the tail of this
session. **This BLOCKS the skeleton deletion (inc-4 item 2):** deleting
`games/sonic4/main.asm` requires its `gameSoundDataIncludes` BINCLUDEs to leave AS,
which requires the sound banks native (P1/P2). The A1 root stub + B1 epilogue land
game.asm/debugger/EndOfRom, but the sound BINCLUDEs have no home until the banks are
native — so the skeleton cannot delete until the sound-bank pass completes.

**Independently landable NOW (does NOT touch the sound banks):** the **B1
engine.epilogue** (EndOfRom + the 3 walls → a native `.emp`; EndOfRom is already a
frozen boundary key and header.emp already externs it). If a partial inc-4 is wanted
before the sound-bank pass, B1 is the clean piece; the A1 stub + the main.asm/
engine.inc deletion stay blocked on the banks.

**Decision for this session: STOP on the sound-bank ports** (the ruled discipline —
the fragile phase-bank surface is not for a fatigued session), deliver this finding as
the map for the dedicated sound-bank pass, and hold the skeleton deletion behind it.

## §6 — K spec §0 end-state checklist — what is now TRUE (after K4 inc 1–3 + this)

- **Placement authority = the declared map (not bootstrapped tables):** TRUE since K1;
  the K4 islands (header, collision, DAC anchor) all declare map order/anchors.
- **`engine/macros.asm` DELETED:** ✅ TRUE (K4 inc-1).
- **`games/sonic4/main.asm` + `games/demo/main.asm` + `engine/engine.inc` DELETED:**
  ❌ NOT YET — blocked on the sound-bank native placement (§5): main.asm's
  `gameSoundDataIncludes` BINCLUDEs (DAC/MT/SFX) have no `.emp` home until the seam-2
  banks are natively placed. engine.inc no longer owns EndOfRom (B1 DONE, inc-5
  Stage 1 — `engine.epilogue` native) but still carries the org ladder + the
  `sound_bank.inc`/`debugger.asm` includes.
- **`engine/system/header.inc` ported:** ✅ TRUE (K4 inc-2 → games/*/config/header.emp).
- **`engine/sound/sound_bank.inc` ported:** ✅ TRUE (K4 inc-5 Stage 4b →
  games/sonic4/data/sound/soundbankhead.emp, the first native phase-bank section;
  sound_bank.inc DELETED). The whole sound bank is native (DAC+MT+SFX bodies + head).
- **The 106 inert orgs die with their files:** PARTIAL — the OJZ orgs (K3), the header
  org, and the collision org are retired/inert; the sound + engine-ladder orgs remain
  until the skeleton deletion.
- **Named survivors — game.asm ×2 + the vendored debugger tree:** on track; the ruled
  A1 root stub (game_root.asm) is their loader-to-be, the B1 epilogue owns EndOfRom.
- **Every other included module/data island is manifest-placed `.emp`:** the DATA
  islands are done EXCEPT the sound banks (§5); the CODE is long done.

**Net K4 state:** macros.asm gone; header + collision native; the DAC anchor explicit;
the two skeleton-deletion design points RULED (A1+B1). The one remaining wall is the
sound-bank seam-2 native-placement pass (§5), which gates the main.asm/engine.inc
deletion. Recommended inc-5 shape: (1) the dedicated sound-bank pass (P2 DAC probe →
phased MT/SFX/soundBankHead, seam-2 harness in view), then (2) B1 epilogue + A1 stubs
+ the skeleton deletion.

## Increment 5 — the sound-bank pass + B1 epilogue (in progress)

**Porter: Opus (k4-inc5 branch, aeon + sigil worktree). Overseer/spec owner: Fable.**
Spec §6 (the ruled P2 staged path). Sequenced: Stage 1 B1 epilogue (bank-independent
warm-up) → Stage 2 P2 DAC probe → Stage 3 MT bank → Stage 4 SFX bank + soundBankHead.
Each its own commit; STOP anywhere the fragile phase-bank surface fights.

### Stage 1 — B1 epilogue (`engine.epilogue`) — DONE, six-target byte-identical

`EndOfRom` + the three ROM-tail walls left `engine.inc` for a native `.emp` module,
`engine/system/epilogue.emp` (`module engine.epilogue in epilogue`):

- **`pub data EndOfRom = Data.empty`** — the zero-length terminus label, placed LAST.
  Its boundary key `EndOfRom` is already frozen in all six tables, so the chainer keys
  the terminus off it; it packs a zero-length section abutting `error_handler`'s tail
  (`error_handler` still ends exactly at `EndOfRom`). `header.emp`'s `rom_end =
  extern("EndOfRom") - 1` resolves cross-seam to the native label unchanged.
- **The walls split by their nature (the B1 ruling: link-time, not comptime):**
  - `PLANE_H_CELLS * PLANE_V_CELLS <= 4096` is a COMPTIME fact (pure constants) → a
    comptime `ensure` (`use engine.constants.{PLANE_H_CELLS, PLANE_V_CELLS}`).
  - `EndOfRom` evenness and the 4 MB ceiling depend on the PLACED address → LINK-time
    asserts: `ensure((extern("EndOfRom") & 1) == 0, …)` and `ensure(extern("EndOfRom")
    <= $3FFFFF, …)` defer to `LinkAssert`s (D-H.4), checked by the chainer against the
    resolved layout (`check_link_asserts`, the vdp/parallax/sound_api ensure-over-extern
    precedent). They evaluate true in every whole-ROM build (implicit non-vacuity: a
    malformed assert fails the build).
- **engine.inc:** the `EndOfRom:` label + the three `if … error` guards + the inert
  `org $5DB60/$5F65A` resume are holed out behind `ifndef SIGIL_EMP_EPILOGUE` (the
  gate is set in every native build; the block is the asl-less fallback, never taken).
- **Registry/pins:** `engine.epilogue`/`epilogue` in `registry()` (rides into
  demo_registry via the `engine.*` filter; config_a/b via `registry()`); code gate
  `SIGIL_EMP_EPILOGUE` (sonic4 + demo gate lists); pin `EPILOGUE` (repin.toml region
  `start = EndOfRom`, `len = 0`; pins.rs `plain_base 0x5DB00 / debug_base 0x5F5F2 /
  len 0`). No map.toml change — `EndOfRom` is a zero-byte marker, excluded from the
  order-validation subsequence (its tie position is byte-neutral), so the K1 order
  list is untouched and stays green.

**Identity: SIX-TARGET FULL-CRC byte-identical — NO re-freeze (cleaner than the note's
predicted appendix re-freeze).** The label set shifted (EndOfRom AS→native + the new
`epilogue` section head) but the deb2 appendix orders by address and EndOfRom's address
is unchanged (0x5DB00 plain), so every full CRC held:

| target | full_crc / len (UNCHANGED, = chain-14) |
|---|---|
| s4 | `4d28dc6b` / 412151 |
| s4_debug | `0d3cd198` / 421982 |
| demo | `55b70266` / 90576 |
| demo_debug | `6487a47c` / 93073 |
| config_a | `0b384578` / 422321 |
| config_b | `947e4c57` / 303555 |

Gates: strict **2895/0/4** (unchanged — no new test binary; epilogue emits zero bytes,
so there is no region-window golden to gate, and the walls are build-path link asserts).
`refreeze --check` OK (tip `k4-collision`, chain len 14 — untouched). `repin` **0 pins
changed** (67→68 regions; the hand-authored EPILOGUE pin matched the tool's derivation
exactly). K1 order-validation green (exercised by the chained strict gates).

Step-3/step-5: the epilogue is a pure authority move (no behavior). Step-5 note — a
dedicated NEGATIVE probe (an odd/over-4MB EndOfRom fires the link asserts; a plane-cell
over-count fires the comptime ensure) would harden the walls beyond the implicit
build-path coverage; ledgered as a gap candidate, not built this stage (the walls are
not easily forced without doctoring the terminus). engine.inc's `EndOfRom` orgs +
label are the row-6-class scaffolding the skeleton deletion (inc-6) removes.

### Stage 2 — P2 DAC probe — STOP (seam-2 model surprise; overseer countersign wanted)

**STOP per the absolute seam-2 discipline** ("any surprise in the seam-2 pipeline is a
STOP with a finding"). Grounded investigation of the DAC bank machinery found the
ruling's core premise inaccurate — the `_BODY_STUB` "mixed harness" the exclusivity
mechanism was to reconcile is VESTIGIAL. Nothing byte-corrupted; the STOP is at the
design-premise level, before touching the fragile main.asm sound block.

**The surprise (the corrected seam-2 model):**
- `SIGIL_EMP_DAC_BODY_STUB` / `SIGIL_EMP_MT_BODY_STUB` / `SIGIL_EMP_SFX_BODY_STUB` are
  referenced ONLY in `games/sonic4/main.asm`'s `ifdef` arms and are **set by NOTHING**
  in sigil (grep-proof across `crates/`). Every whole-ROM build takes the `else`
  (BINCLUDE) arm. The `_BODY_STUB` arms are dead code.
- `native.rs` (assemble_as_side, sound-on) sets the BARE gates `SIGIL_EMP_DAC` /
  `SIGIL_EMP_MT` / `SIGIL_EMP_SFX` = 1 — but main.asm consumes NONE of them (it checks
  the `_BODY_STUB` variants). So the bare gates are set-but-unconsumed.
- The note §5's description of the STUB arms as a live "mixed harness (mixed_dac_rom /
  dac_port) in-memory composition" is inaccurate: `mixed_dac_rom.rs` does NOT exist;
  `dac_port.rs` compiles `dac_samples.emp` in ISOLATION (its own two-region map), NOT
  through main.asm's STUB arm in a whole-ROM build. So there is NO live in-memory
  whole-ROM DAC composition to double-place against.
- What the `_BODY_STUB` arm's CODE actually is: just `org $58000` — the AS-SKIP half
  (position the cursor past the two-bank hole so the MT bank's `align $8000` lands at
  $58000). It is exactly the "AS skips, native fills" path P2 needs, but gated by a
  never-set symbol and MISSING its other half (the native section that fills $48000/
  $50000). P2's real job is: add the native fill + activate the skip, NOT reconcile a
  live in-memory composition.

**Read-only viability PROVEN (so the recommended path is de-risked for countersign):**
- `ensure_generated` emits `dac_blip_bank.bin` (2880 B) + `dac_shared_bank.bin`
  (30908 B) (gitignored build artifacts, present at build time). Both are
  BYTE-IDENTICAL to the reference ROM at $48000 / $50000; the inter/post-bank gaps
  ($48B40..$50000, $578BC..$58000) are all-zero (map `fill = 0x00` reproduces them).
- `build_emp`'s `include_root` + `embed_base` are the aeon root, so a native section
  `embed("engine/sound/generated/dac_blip_bank.bin")` resolves; `ensure_generated`
  runs before `build_emp` (build_rom_chained_with_listing:2105), so the .bin exists at
  lower time — no cycle (dac_samples.emp → emit → .bin; dac_banks.emp → embed .bin).

**Recommended P2 DAC wiring (for the overseer to countersign):**
1. New module `games/sonic4/data/sound/dac_banks.emp` — two sections
   `dac_blip_bank` (`pub data Dac_Temp_Blip = embed("…/dac_blip_bank.bin")`) and
   `dac_shared_bank` (`Dac_SharedBank_Start` / `Dac_Kick = embed("…/dac_shared_bank.bin")`),
   placed at the existing map anchors ($48000 / $50000, `when="sound_on"`).
2. Registry + pins (DAC_BLIP_BANK / DAC_SHARED_BANK, anchored) + a gate + map order
   heads (Dac_Temp_Blip already in the order list).
3. main.asm DAC block: the `org $58000` skip becomes the ACTIVE native path; the
   BINCLUDE arm DELETES (spec §6). The vestigial `_BODY_STUB` gate is retired.
4. **The exclusivity mechanism (reframed):** with the BINCLUDE arm deleted and the
   native section unconditional in sound-on, exclusivity is STRUCTURAL — can't-both
   (no BINCLUDE to double-place), can't-neither (native section always present in
   sound-on). The LOUD enforcement is the map anchor validation (`dac_blip_bank`
   anchor @ $48000 already declared) + the whole-ROM byte gate + an emit-first
   `dac_bank_port` golden gate (runs `ensure_generated`, then diffs [$48000,$58000)).
   This is simpler than the ruling's "reconcile two live placements" because there is
   only ONE live placement path.

**Why STOP and not proceed:** (a) the ruling named "STUB-arm reconciliation" THE CORE
Stage-2 deliverable and made the exclusivity mechanism a design requirement — but the
reconciliation target is vestigial, so the mechanism's shape changes (structural, not
a live-vs-live guard); this reframing should be blessed before it is baked into main.asm
AND inherited by Stages 3/4 (MT/SFX, which share the pattern). (b) The change edits the
most fragile surface (main.asm's `phase 08000h` sound block, the Z80-pointer banks);
the ruled discipline is countersign-first there. (c) The bare-`SIGIL_EMP_DAC`-vs-
`_BODY_STUB` gate choice + whether to fully delete vs keep-dead the BINCLUDE arm are
real calls the exclusivity-mechanism ruling should own. Stages 3/4 (phased MT/SFX +
soundBankHead) are held behind this — they inherit whatever mechanism Stage 2 sets.

### Stage 2 — P2 DAC probe — DONE under the ruling (six-target byte-identical)

The coordinator ruled the reframe accepted in full (structural exclusivity; delete the
dead `_BODY_STUB` arms; re-key to the bare `SIGIL_EMP_DAC` gate; `dac_banks.emp` embeds
the artifacts). Implemented on the same k4-inc5 branches:

- **`games/sonic4/data/sound/dac_banks.emp`** (`module games.sonic4.dac_banks in
  dac_banks`) — ONE section embedding the seam-2 `dac_blip_bank.bin` @ $48000 +
  `dac_shared_bank.bin` @ $50000, with an intra-section `align $8000` for the inter-bank
  pad (the AS twin's structure exactly; the pad bytes are $00, matching the reference).
  Head label `Dac_Temp_Blip` (section head + map-order + frozen boundary key);
  `Dac_SharedBank_Start` / `Dac_Kick` are mid-section labels (coincident at $50000). The
  SND_* triples stay folded in the DacSampleTable head at emit — the main build's Dac_*
  labels are placement heads with no runtime consumer.
- **STRUCTURAL EXCLUSIVITY (the ruled mechanism):** the AS BINCLUDE arm is DELETED
  (can't-both — nothing else places the DAC), the native section is unconditional in
  the sound-on registry (can't-neither), and `main.asm`'s `ifdef SIGIL_EMP_DAC` skip
  (`org $58000`) moves the AS cursor past the native region so the MT bank lands at
  $58000. Loud enforcement = the map anchor validation (`dac_banks` @ $48000,
  `when="sound_on"`) + the whole-ROM byte gate + the emit-first `dac_bank_port` gate.
  The dead `SIGIL_EMP_DAC_BODY_STUB` arm (a never-set gate on a dead org-skip) is gone.
- **Wiring:** registry `games.sonic4.dac_banks`/`dac_banks` (sound-ON only —
  filtered out of config_b, and demo excludes it via the `engine.*` filter); pin
  `DAC_BANKS` (repin.toml `start=Dac_Temp_Blip len=0xF8BC`; pins.rs base $48000 both
  shapes, len $F8BC = blip 0xB40 + pad to 0x8000 + shared 0x78BC); the bare
  `SIGIL_EMP_DAC` gate holes out the AS skip; map anchor renamed `dac_blip_bank`→
  `dac_banks` (matched by address, transparent). **Frozen-key seed** (the K3-run-B
  pattern — s4 uses the Frozen chainer, so a keyless section sorts to base 0 and
  collides): `Dac_Temp_Blip 0x48000` added to the THREE sound-on tables (s4.txt /
  s4_debug.txt / config_a.txt; shape-invariant; config_b/demo are sound-off, no DAC).
- **Emit-first golden gate** `dac_bank_port` (spec §6): runs `ensure_generated` (the
  .bin are gitignored artifacts) then diffs the linked `dac_banks` section against the
  reference [$48000, $578BC) both shapes. 2/2.

**Identity: SIX-TARGET FULL-CRC byte-identical — NO golden re-freeze.** The DAC bytes
land at the same LMAs and the same three labels persist at the same addresses, so the
deb2 appendix is unchanged. Only the frozen TABLES gained one boundary key (Dac_Temp_Blip,
seeded ×3) — the goldens themselves are byte-identical.

| target | full_crc / len | vs chain-14 |
|---|---|---|
| s4 | `4d28dc6b` / 412151 | identical |
| s4_debug | `0d3cd198` / 421982 | identical |
| demo | `55b70266` / 90576 | identical (sound-off, no DAC) |
| demo_debug | `6487a47c` / 93073 | identical |
| config_a | `0b384578` / 422321 | identical |
| config_b | `947e4c57` / 303555 | identical (sound-off, no DAC) |

Gates: strict **2897 / 0 / 4** (baseline 2895 + 2 dac_bank_port). `refreeze --check` OK
(tip `k4-collision`, chain 14). `repin` 0 pins changed (68→69 regions). K1 order green
(Dac_Temp_Blip already in the map order). No assembled-anchor move — the DAC LMAs are
byte-exact.

Step-3/step-5: pure placement move (no behavior). The reframe simplified the ruling's
"reconcile two live placements" to structural exclusivity — the byte gate + anchor
validation are the loud guards. The pattern (native embed of the emitted .bin, delete
the BINCLUDE arm, re-key the bare gate, frozen-key seed, emit-first golden gate) is the
template Stages 3/4 (MT / SFX) inherit — with the added phase-bank-anchor rule (a labeled
$8000-window head NEVER repacks; the mt-gate catch class).

### Stage 3 — P2 MT bank probe — DONE (anchors-proven flagged re-freeze)

The Moving-Trucks streaming bank BODY (the BINCLUDE at $58607, inside the sound bank
after the phased soundBankHead) is native — `games/sonic4/data/sound/mt_bank_blob.emp`
embeds the seam-2 `mt_bank{,_debug}.bin`. Inherits the Stage-2 template + adds the
shape-dependent + cross-seam-label wrinkles:

- **Non-phased LMA embed** at $58607 (the MT body is emitted AFTER `dephase/restore`,
  so its labels are plain LMA — like the DAC, NOT a `bank: $8000` phase section). The
  start $58607 is shape-INVARIANT (soundBankHead is $607 both shapes); only the body
  SIZE differs (plain 0x34E1 → $5BAE8, debug 0x4F33 → $5D53A, debug adds DrumTest +
  HCZ2). The embed selects on DEBUG.
- **`SongTable`/`SongPatchTable` stay AS-provided** by the emitted `mt_syms{,_debug}.asm`
  (kept `include`d): they sit at mid-blob offsets (len − SONG_COUNT*8/4, SONG_COUNT 1
  plain / 3 debug) a single `embed()` cannot label, and sound_api.emp externs them. This
  is the pragmatic P2 — mt_syms.asm IS an emitted artifact (like the .bin). The native
  section owns the BYTES + the head label `Song_MovingTrucks` (placement head; no runtime
  consumer).
- **Structural exclusivity** (inherited): the AS BINCLUDE deleted (can't-both), the native
  section unconditional in sound-on (can't-neither); main.asm's `ifdef SIGIL_EMP_MT`
  per-shape skip org ($5BAE8 plain / $5D53A debug) moves the AS cursor past the native
  body so the SFX block lands correctly. The dead `SIGIL_EMP_MT_BODY_STUB` arm deleted.
- **Wiring:** registry `games.sonic4.mt_bank_blob`/`mt_bank_blob` (sound-ON only — config_b
  filtered); pin `MT_BANK_BLOB` (start=Song_MovingTrucks, len 0x34E1 plain / debug_len
  0x4F33 — the shape-dependent literal-len form; the repin region name drives the CONST
  name, K3-run-A); gate `SIGIL_EMP_MT`; map order + anchor (Song_MovingTrucks — not an
  island, abuts soundBankHead at gap 0, so no new anchor); frozen-key seed
  `Song_MovingTrucks 0x58607` ×3 sound-on tables (shape-invariant head); emit-first
  `mt_bank_port` golden gate (2/2).

**Identity: ANCHORS IDENTICAL ×6 — appendix-only flagged re-freeze (the inc-2 header
precedent).** The MT body is byte-exact at $58607, `EndOfRom` is unchanged ($5DB00), and
the assembled region `[0, EndOfRom)` is byte-identical EXCEPT the sigil-patched header
checksum ($18E) + rom_end ($1A4) — the derived cells that MUST move when the image grows.
The +18 bytes (plain) is the new `Song_MovingTrucks` label entering the deb2 symbol
appendix (the AS twin had no head label — just BINCLUDE bytes — so this label is genuinely
new; it cannot be avoided, the listing is pub-agnostic over all section labels). Proven by
diff: the only `[0,EndOfRom)` diffs are $18E (checksum) + $1A4 (rom_end).

| target | full_crc (was → now) | anchor_crc (UNCHANGED) / anchor_end |
|---|---|---|
| s4 | `4d28dc6b` → **`95428d52`** / 412169 | `658c623b` / 0x5DB00 |
| s4_debug | `0d3cd198` → **`74e16b34`** / 422000 | `b137c411` / 0x5F5F2 |
| config_a | `0b384578` → **`cfbd235b`** / 422337 | (anchor unchanged) |
| demo / demo_debug / config_b | UNCHANGED (sound-off, no MT) | — |

`refreeze --freeze k4-mt-bank` (no `--ab` — anchors did not move) appended provenance
chain entry **#15**; the 3 sound-on goldens re-frozen; frozen tables re-derived (gained
Song_MovingTrucks). Gates: strict **2899 / 0 / 4** (2897 + 2 mt_bank_port); `refreeze
--check` OK (tip k4-mt-bank, chain 15); `repin --check` clean; K1 order green.

Step-5: the `mt_syms.asm` retention is the one non-full-native residue (the mid-blob
cross-seam labels). A future emit that emits `SongTable`/`SongPatchTable` as separate
tiny artifacts, OR a split-embed with the emitted offset, would let the .emp own them —
ledgered, not built (P2 keeps the emit pipeline untouched by design).

### Stage 4a — P2 SFX block probe — DONE (anchors-proven flagged re-freeze)

The SFX block (the BINCLUDE at $5BAE8/$5D53A, after the native MT body) is native —
`games/sonic4/data/sound/sfx_bank_blob.emp` embeds the seam-2 `sfx_bank{,_debug}.bin`.
The CLEANEST of the three bodies: non-phased LMA embed, **NO cross-seam labels** (no
surviving AS/emp reads SfxTable — sound_sfx.emp's SfxBlobWinTab reads are native, in the
head), so no syms file. Shape-INVARIANT size (0x748 both), shape-DEPENDENT start ($5BAE8
plain / $5D53A debug — the MT body before it differs) + content (the SfxTable pointer
cells). Head label Sfx_33 (placement head only).

- **Wiring:** registry `games.sonic4.sfx_bank_blob` (sound-ON only, config_b filtered);
  pin `SFX_BANK_BLOB` (base $5BAE8 plain / $5D53A debug, len 0x748); gate `SIGIL_EMP_SFX`;
  map order + Sfx_33 (abuts the MT body at gap 0 — not an island); frozen-key seed
  `Sfx_33` per-shape ($5BAE8 / $5D53A) ×3 sound-on tables; emit-first `sfx_bank_port`
  golden gate (2/2). The AS BINCLUDE deleted; per-shape skip org; dead
  `SIGIL_EMP_SFX_BODY_STUB` arm deleted.

**Identity: ANCHORS IDENTICAL ×6 — appendix-only flagged re-freeze.** SFX body byte-exact,
`EndOfRom` unchanged ($5DB00); the only `[0,EndOfRom)` diffs are the derived header
checksum ($18E) + rom_end ($1A4); the +8/+10 bytes is the new Sfx_33 head label in the
deb2 appendix. `refreeze --freeze k4-sfx-bank` (no `--ab`) → chain **#16**.

| target | full_crc (was → now) | anchor_crc (UNCHANGED) |
|---|---|---|
| s4 | `95428d52` → **`f788dae7`** / 412177 | `658c623b` |
| s4_debug | `74e16b34` → **`d7af41e5`** / 422010 | `b137c411` |
| config_a | `cfbd235b` → **`58c570a0`** / 422349 | (unchanged) |
| demo / demo_debug / config_b | UNCHANGED (sound-off) | — |

Gates: strict **2901 / 0 / 4** (2899 + 2 sfx_bank_port); `refreeze --check` OK (chain 16);
`repin --check` clean; K1 order green. The MT + SFX bank BODIES are now native; the
remaining sound-bank piece is the `soundBankHead` phase-08000h head (sound_bank.inc) —
Stage 4b, the phase-bank surface.

### Stage 4b — P2 soundBankHead probe — DONE (the phase bank; sound_bank.inc DELETED)

The engine-table bank HEAD (the `soundBankHead` macro, sound_bank.inc — the 5 heads
inside the `phase 08000h` bracket) is native — `games/sonic4/data/sound/soundbankhead.emp`
places them as the **FIRST native PHASE-BANK section** (vma $8000 / lma $58000) embedding
the seam-2 head artifacts. sound_bank.inc + its macro are DELETED (the last sound-bank
piece; the `phase 08000h` bracket goes with them — the section's `vma: $8000` carries the
window addressing).

- **The phase-bank-anchor rule (armed, first exercise):** a labeled $8000-window head is
  a HARD org (`is_phase_bank`), never repacks. The 5 heads are byte-exact + contiguous
  ($8000..$8607 = $607); a size drift would slide a downstream head off its fixed carrier
  VMA and desync the resident Z80 blob — the AS `fatal` walls are now the module's comptime
  `ensure`s ($357/$108/$10E/$40/$5A). NO whole-ROM-link consumer (the resident Z80 code
  that reads these labels is the seam-1 BLOB, resolved separately via banked_carriers +
  the DacSampleTable -D — this module re-homes the same labels the AS soundBankHead defined).
- **Wiring:** registry `games.sonic4.soundbankhead` (sound-ON only, config_b filtered); gate
  `SIGIL_EMP_SOUNDBANKHEAD` (added to the sound-on bare-gate set); main.asm replaces the
  `save/cpu z80/phase/soundBankHead/dephase/restore` block with an `ifdef SIGIL_EMP_SOUNDBANKHEAD`
  skip org $58607 (keeping MovingTrucks_Bank_Start + the SND_ENGINE_TABLE_BANK/SFX_BLOB_BANK
  equs, which read its LMA>>15); m1c_root.asm drops the sound_bank.inc include; emit-first
  `soundbankhead_port` golden gate (2/2). **Two phase-bank firsts recorded:**
  (1) `SoundTablesZ80_Head` was already in s4.txt/s4_debug.txt but NOT config_a.txt — seeded
  it there (config_a placed the keyless phase section at base 0 and collided with `vectors`
  until seeded). (2) **repin derives the pin `SOUNDBANKHEAD` as the phase VMA ($8000), not
  the LMA** (it resolves the phased label's value); the golden gate therefore reads the LMA
  = VMA + $50000 = $58000 explicitly, and placement is driven by the frozen key (LMA $58000,
  Frozen path) — the VMA pin is cosmetic for the shipped (Frozen) builds.

**Identity: ANCHORS IDENTICAL ×6 — appendix flagged re-freeze.** Head byte-exact at the
phase VMAs, `EndOfRom` unchanged ($5DB00); the only `[0,EndOfRom)` diffs are the derived
header checksum ($18E) + rom_end ($1A4). The appendix RE-HOMES labels (the AS soundBankHead's
`_End` span labels retire; the 6 head labels re-home AS→native). `refreeze --freeze
k4-soundbankhead` (no `--ab`) → chain **#17**.

| target | full_crc (was → now) | anchor_crc (UNCHANGED) |
|---|---|---|
| s4 | `f788dae7` → **`94708cfb`** / 412154 | `658c623b` |
| s4_debug | `d7af41e5` → **`6f0a1948`** / 421990 | `b137c411` |
| config_a | `58c570a0` → **`b3ce9e67`** / 422325 | (unchanged) |
| demo / demo_debug / config_b | UNCHANGED (sound-off) | — |

Gates: strict **2903 / 0 / 4** (2901 + 2 soundbankhead_port); `refreeze --check` OK
(chain 17); `repin --check` clean; K1 order green. **The entire sound bank is now native —
DAC + MT + SFX bodies + the soundBankHead head; sound_bank.inc DELETED.**

## Increment 6 — the skeleton dissolution (in progress, k4-inc6 off the merged masters)

Deletes `games/sonic4/main.asm` + `games/demo/main.asm` + `engine/engine.inc` (the AS
root + ROM-layout owner) under the ruled A1 (a minimal `game_root.asm` stub per game).
Committed incrementally; the true first step is re-homing the last byte-bearing AS
residual native (nothing can drop the orgs until these go native).

### Inc-6 A — the last byte-bearing AS residual → native (byte-neutral)

engine.inc's two remaining hand-written byte emitters are native:
- **`ObjCodeBase` (rts @ $10000)** → `engine/objects/objcodebase.emp` (`pub proc
  ObjCodeBase () { rts }`), the object-bank base + the offset-0 safety net. Consumed
  cross-module by every player/test `.emp`'s `label - ObjCodeBase` code_addr. Placed
  at $10000 by the map `object_bank` anchor + the frozen key ObjCodeBase — **the
  `org $10000` retires**. player_common abuts it at $10002 (len 2, exact).
- **`NullInterrupt` (rte)** → `engine/system/null_interrupt.emp` (`pub proc
  NullInterrupt () { rte }`), the no-op IRQ handler vectors.emp points IRQ1/2/3/5/7 at
  (`dc.l NullInterrupt`, cross-seam). Placed at its frozen key ($5CA42/$5E540).
- `__BUDGET_OBJBANK` / `__BUDGET_ENGINE` (no consumers) retire with the org block.
  `__BUDGET_DATA` (the object-bank budget cursor) stays for now — inc-6 B adapts it.

Both gated out of engine.inc behind `ifndef SIGIL_EMP_{OBJCODEBASE,NULL_INTERRUPT}`.
Registry + pins (`OBJCODEBASE`; `NULLINT` — the region const, renamed off the existing
`NULL_INTERRUPT` test-carrier SYMBOL pin vectors_port injects) + gates (sonic4 + demo).
**SIX-TARGET FULL byte-identical (= chain-17), no re-freeze; strict 2903/0/4; repin 0
changed; refreeze --check OK.** The last hand-written AS bytes have left the tree — the
AS residual now emits ZERO bytes (game.asm + debugger.asm are defines/externs only).

**Remaining (inc-6 B):** the A1 stubs (`game_root.asm` ×2), the budget-cursor adaptation
(compute from object_bank sections, retiring `__BUDGET_DATA`), the game_root_rel flip,
and DELETE main.asm ×2 + engine.inc. Then the §0 end-state final annotation.

### Inc-6 B — the skeleton deletion (DONE; main.asm ×2 + engine.inc gone)

**Porter: Opus (k4-inc6b branch, aeon + sigil worktree). Overseer/spec owner: Fable.**
The six ruled steps landed, each its own commit; anchors held ×6 throughout.

1. **Budget-cursor adaptation (byte-neutral).** The object bank and the data region pack
   CONTIGUOUSLY in `[$10000,$20000)` and the data extends BEYOND the window (collision_data
   ends at $41BDA), so an LMA-window scan cannot separate object code from data — only a
   declared boundary can. `native::object_bank_cursor` now takes the map-declared cursor
   label and resolves its LMA (the object-code terminus = where data begins). The
   `[[budget]]` table gains `cursor` (`map_placement::Budget`): sonic4/sigil.map.toml
   `= "DeformTable_Zero"`, demo `= "ObjDef_DemoBox"`. Reports used = 0x1294 (the true
   object-code size; a naive window scan mis-reported the data-inclusive 0xFE30). The AS
   `__BUDGET_DATA` marker the cursor used to key off retires with engine.inc.
2. **The A1 stubs `games/<g>/game_root.asm`.** `cpu 68000 / padding off / supmode on`,
   `include game.asm`, `include debugger.asm`, `END` — no orgs. sonic4's stub also re-homes
   the ONE cross-seam artifact include the deleted `gameSoundDataIncludes` carried: the
   SIGIL_EMP_MT-gated `mt_syms{,_debug}.asm` (SongTable/SongPatchTable absolute equs,
   externed by `sound_api.emp`). demo is sound-OFF → no sound include.
3. **`game_root_rel` flipped** (native.rs, all 4 sonic4-rooted profiles + demo →
   `games/<g>/game_root.asm`). `map_path` still resolves (parent `games/<g>/` unchanged);
   the `/demo/` appendix-floor discriminator still matches. Proven byte-identical BEFORE
   deleting the skeleton (game_root.asm is a byte-perfect replacement).
4. **DELETED** `games/sonic4/main.asm` + `games/demo/main.asm` + `engine/engine.inc`.
   build.sh `MAIN_ASM` → game_root.asm; `__BUDGET_DATA` removed from s4/s4_debug frozen
   tables (regenerated clean by the re-freeze); stale comments swept (player_air/ground.emp,
   build.sh, repin.rs). The 7 game*Includes macros die with the files.

**The residual re-homing inventory (nothing died silently).** The AS residual emitted 4
zero-length sections; each accounted for:
- `__BUDGET_ENGINE` @ $100 — no consumer (inc-6A finding); dies.
- `__BUDGET_DATA` @ ~$11294 — the object-bank cursor (re-keyed to the map, step 1);
  removed from s4/s4_debug frozen tables; dies.
- `MovingTrucks_Bank_Start` @ $58000 — its `>>15` fed `SND_ENGINE_TABLE_BANK`/`SFX_BLOB_BANK`
  (consumed ONLY by seam-1 Z80 modules, which the seam provides independently —
  `seam1.rs:760` hardcodes the bank ids, `seam2.rs` synthesizes its own head). No whole-ROM
  `.emp` externs it; dies harmlessly.
- the label-less epilogue-resume tail @ EndOfRom — dies.
- `SongTable`/`SongPatchTable` (via `mt_syms`) — the ONE genuine whole-ROM cross-seam dep
  (`sound_api.emp` `movea.l #SONG_TABLE`); RE-HOMED to `game_root.asm` (gated). Zero bytes.

**Kill-rows 9/45 (STOP-critical) — GREEN by construction.** They re-extract game.asm's
macro bodies, which are byte-UNTOUCHED (only the include SITE moved, main.asm→game_root.asm):
- kill row 9 — `game_loop_port` (gameDebugTick H2 mirror): **2 passed / 0 failed**.
- kill row 45 — `boot_port` (gameBootHook): **4 passed / 0 failed**.

**Identity: ANCHORS IDENTICAL ×6 — appendix-only re-freeze (chain #18 `k4-skeleton`, ab="").**
The predicted label-set shift materialized: the dissolved AS-residual labels left the deb2
symbol dump, shrinking the sound-on appendix by the one appendix-resident label
(`MovingTrucks_Bank_Start`, −20 B). The assembled `[0,EndOfRom)` prefix is byte-identical
(header-neutral anchor CRC unchanged ×6); the sigil-patched header checksum ($18E) + rom_end
($1A4) move because the total file SHRANK. NO assembled-anchor move → no STOP → the sanctioned
appendix-only re-freeze (`refreeze --freeze k4-skeleton`, no `--ab`, the tool's anchor
discipline pre-check passed).

| target | full_crc (was → now) | anchor_crc (UNCHANGED) / anchor_end |
|---|---|---|
| s4 | `94708cfb` → **`5f72b9c3`** / 412134 | `658c623b` / 0x5DB00 |
| s4_debug | `6f0a1948` → **`e6171a80`** / 421970 | `b137c411` / 0x5F5F2 |
| config_a | `b3ce9e67` → **`f92f0333`** / 422305 | `19b793ec` / 0x5F5F2 |
| demo / demo_debug / config_b | UNCHANGED (residual labels not in their appendix) | e2dc9207 / b7d81931 / 8b490cfb |

Gates: strict **2903 / 0 / 4** (baseline; no new test binary — pure deletion + re-key);
`repin --check` clean (0 pins changed); K1 order-validation green (the order lists never
named the dissolved zero-byte markers). Bookkeeping same-branch: census #50/#19/#50b(engine.inc)
→ DELETED; kill-list AS-org-arm closure note (rows 5/6/58/60/64/67/72/73/79/84/85/86/88/89/
90/91); gap-ledger K4 inc-6B section (mt_syms re-home, vestigial repin paste blocks, vestigial
code_gates).

## §0 — THE K END-STATE, CERTIFIED (what is now TRUE)

- **Placement authority = the DECLARED map, not bootstrapped tables.** Every section's
  order/anchor/hole/budget is a reviewed `games/<g>/map.toml` fact; the frozen tables are
  now pure per-freeze MEASUREMENT caches (they record what the pack produced — the last
  authoring role, the object-bank `__BUDGET_DATA` cursor, moved to the map this increment).
- **`games/sonic4/main.asm` + `games/demo/main.asm` + `engine/engine.inc` DELETED.** ✅
- **`engine/macros.asm` DELETED** (K4 inc-1). ✅
- **Every included module / data island is manifest-placed `.emp`.** ✅ — header (inc-2),
  collision + Sonic art (inc-3), the whole sound bank (inc-5: DAC + MT + SFX bodies + the
  phase-bank head, `sound_bank.inc` deleted), the epilogue/EndOfRom (B1), ObjCodeBase +
  NullInterrupt (inc-6A). The OJZ level tree + all engine code went `.emp` in K1–K3.
- **The 106 inert `org` lines are GONE** — they died with the files that carried them; the
  proof is made visible (the ROM is byte-identical without them).

### The named survivors — the honest 100%
Everything else is `.emp`. What remains on the AS side, and WHY each is a sanctioned survivor
(spec §0), not an unfinished port:

1. **`games/sonic4/config/game.asm` + `games/demo/config/game.asm`** — the game CONTRACT:
   `GAME_CAMERA_JUMP_LOCK`, `Game_Entry`/`GAME_ENTRY_ID`, and the `gameBootHook`/`gameDebugTick`
   macro bodies. Their retirement is the **game-contract-hook language construct**, an
   explicitly-deferred language-round item (spec §4). The kill-row-9/45 combo matrices are
   their standing guard (they byte-diff these exact macro bodies' AS expansion every run).
2. **The vendored MD Debugger tree — `engine/debug/debugger.asm`** — Volence's ruling (an own
   debugger is planned). Definitions + macros only; the error-handler blob itself is native
   (`error_handler.emp`), and debugger.asm resolves `MDDBG__*` as link externs off that base.
3. **`engine/sound/generated/mt_syms{,_debug}.asm`** — an EMITTED ARTIFACT (like the sound
   `.bin`), not hand-authored source: the seam-2 emit writes SongTable/SongPatchTable as
   absolute equs because they sit at mid-blob offsets a single `embed()` can't label. A
   future emit that emits them as separate tiny artifacts drops it (gap-ledgered).
4. **The two A1 stubs `games/<g>/game_root.asm`** — the ~5-line AS entry that loads survivors
   1–3 into the residual symbol environment. Ruled A1: the minimal loader for the named
   survivors, documented against the game-contract-hook ledger row. Emits ZERO bytes,
   declares NO orgs.

**The honest-100% statement:** the from-scratch ROM layout is now 100% the declared sigil
map + registry; the AS side is FOUR files (2 game contracts + 1 vendored debugger + 1 emitted
syms artifact) loaded by 2 five-line stubs, every one a spec-§0-named survivor with a ruled
reason to remain. No `.asm` file emits a ROM byte or declares an `org`. Parcel K is complete.
