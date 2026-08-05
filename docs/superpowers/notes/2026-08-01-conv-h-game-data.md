# 2026-08-01 — CONV-H PARCEL H: the remaining game data (close packet)

Status: Merge state lives in the campaign log, not here. Branch pair
`conv-h-game-data` (aeon + sigil). Three files retired/ported to the byte-identity
bar (#34, #46, #35); the demo tree (#14/#15/#17/#18/#20) and #12 boot_data are
scoped named remainders with numbered premise corrections.

## §0 — THE HEADLINE

- **#46 pitcher_plant anims — DELETED** (Volence pre-ruled). Verified unwired.
- **#34 act_descriptor — the dead AS descriptor twin RETIRED.** The typed
  `Act`/`[Sec; 9]` literal already lived in `act_descriptor.emp` (gate
  `SIGIL_EMP_ACT_DESCRIPTOR`, on for every sonic4 native build); the `.asm`'s
  `ifndef`-gated descriptor twin + the now-duplicated grid/axis-clamp asserts are
  gone. The generated includes / BINCLUDEs (pool, dicts, blobs, palettes, BG) +
  the org resume stay AS-side. **Six targets byte-identical.**
- **#35 test_mappings — PORTED to `.emp` (the `offsets` construct).** `Map_TestObj`
  is now a §4.7 word-offset table over three struct-typed `MapFrame1` inline bodies
  (the frame index), authored in `games/sonic4/data/mappings/test_mappings.emp`,
  placed natively at the pinned `TEST_MAPPINGS` region. **Assembled ROM
  byte-identical** (the anchor is UNCHANGED on all four sonic4-shape targets); the
  full-file goldens **re-freeze** on the deb2-appendix symbol rename (the offsets
  members are hygienic `__offsets$…` labels where the AS twin named them
  `Map_TestObj_F0/F1/F2`) — the sanctioned appendix-only drift.
- **Named remainders (premise corrections below):** #12 boot_data (blocked on a
  missing `.emp` binary-embed construct — the mid-table Z80 blob BINCLUDE); the
  demo tree #14/#15/#17/#18/#20 (blocked on the demo profile having ZERO native
  game-module placement infrastructure — a parcel-sized build of its own).

## §1 — #34 act_descriptor (byte-identical, KEEP the residual .asm)

`act_descriptor.emp` was already the canonical descriptor (Parcel A structs flip is
DONE — `engine/structs.asm` is gone; `Act`/`Sec` are `engine.structs` twins). The
gate `SIGIL_EMP_ACT_DESCRIPTOR` is in `code_gate_defines()` (native.rs), so the
`ifndef SIGIL_EMP_ACT_DESCRIPTOR` AS descriptor block was already DEAD in every real
build. The port = delete that dead twin + the grid/axis-clamp `if/error` walls the
`.emp` now owns as comptime `ensure`s. The residual `.asm` keeps the generated
includes (pool manifest/pool/dicts/blobs), the BINCLUDEs (OJZ_Palette, BGND_Palette,
BG layout/tiles), the `POOL_TILE_CEILING` assert (guards the AS-side generated pool),
and the `org $14D9E/$14E06` resume. Six targets byte-identical.

## §2 — #46 pitcher_plant anims (DELETE, verified unwired)

`games/sonic4/data/sprites/pitcher_plant/anims.asm` (`Ani_PitcherPlant`, 6 lines).
Grep proof: no build file (`.asm`/`.inc`/`.emp`) includes it; the only reference is
`pitcher_plant/sprite.json`'s `"animTable": "Ani_PitcherPlant"` editor metadata, and
the pitcher_plant object was never wired into the build. Deleted per the ruling.
(The stale `sprite.json` metadata is left — the editor is the source of truth for the
parked object; a live-editor decision, not a build artifact.)

## §3 — #35 test_mappings (the `offsets` port · assembled-identical · appendix re-freeze)

The AS file (`Map_TestObj` word-offset table + 3 frame records, 0x30 bytes) →
`games/sonic4/data/mappings/test_mappings.emp`:

- `spr_size(w, h)` comptime fn = `((h-1)<<2)|(w-1)` (the AS `sprSize(w,h) >> 8` high
  byte). `MapPiece` struct (y_off/size/link/tile/x_off = 8 bytes) + `MapFrame1`
  (bbox + piece_count + one MapPiece = 14 bytes) + a `centered()` constructor.
- `pub offsets Map_TestObj { F0/F1/F2: MapFrame1 = centered(...) }` — the table
  emits `dc.w Frame - Map_TestObj` per entry then the three frames inline. Proven to
  emit the exact 48 golden bytes (offset table `00 06 00 14 00 22`, F0 bbox
  `f8 08 f8 08`, size byte 5 = spr_size(2,2)).

**Byte-identity proof (assembled ROM — the PRIMARY bar):**

| target | anchor (built) | anchor (g9 tip) | assembled body |
|---|---|---|---|
| s4          | 658c623b | 658c623b | identical |
| s4.debug    | b137c411 | b137c411 | identical |
| config_a    | 19b793ec | 19b793ec | identical |
| config_b    | 8b490cfb | 8b490cfb | identical |

`native_full_rom.rs:130` (assembled prefix == asl witness, header fields excluded)
PASSES on both s4 shapes — the code+data is verifiably asl-identical. The ONLY
full-file drift is the deb2 appendix (3 symbol entries: the frame labels renamed to
`__offsets$…`) + the folded header checksum — the appendix-only re-freeze the
provenance model sanctions ("full_crc may move freely — an appendix-only change is
behaviour-inert"; anchor unchanged → no A/B needed). Golden-extraction region gate:
`test_mappings_port.rs` byte-diffs the module window vs `s4.bin[0x256B4..0x256E4]`
(plain) / `s4.debug.bin[0x2573C..0x2576C]` (debug) + the `dc.l Map_TestObj`
bare-name resolve.

**Re-freeze CRCs (post-refreeze; anchors unchanged from g9):**

| target | new full_crc | full_size |
|---|---|---|
| s4          | ff9037f2 | 412127 |
| s4.debug    | 06680f0b | 421958 |
| config_a    | 2485eab3 | 422297 |
| config_b    | d6d23298 | 303501 |
| demo        | 9bb8c993 (unchanged) | 90506 |
| demo.debug  | bc7678d0 (unchanged) | 93006 |

**Re-homes (sigil-side placement wiring):**
- `pins.rs`: `TEST_MAPPINGS` region (0x256B4/0x2573C, len 0x30). repin: unchanged
  (hand-computed base exact). The pre-existing `MAP_TEST_OBJ` label pin (objdef_port
  cross-seam map pointer) stays — same address, distinct role.
- `native.rs`: registry `games.sonic4.test_mappings @ test_mappings`; gate
  `SIGIL_EMP_TEST_MAPPINGS` in `code_gate_defines()`.
- `repin.toml`: `test_mappings` region (start Map_TestObj, end Ani_Sonic).
- frozen tables: `Map_TestObj` boundary anchor added to s4/s4_debug/config_a/config_b
  (re-derived via `derive_offcanon` — the tool re-sorted the conv-g `DeformTable_Zero`
  hand-edit into canonical position en route).
- `main.asm`: `include test_mappings.asm` dropped (native fills the hole; the existing
  org past sonic_anims resumes the AS residual). `test_mappings.asm` DELETED.

**Step-3 (retrospect):** the offset table's `dc.w Frame - Map_TestObj` hand-arithmetic
becomes the construct's link-range-checked table; the frame records become typed
structs (bbox/piece drift cannot compile). No construct WISH — §4.7 + struct literals
sufficed.

**Step-5 (engine optimization):** none — pure ownership move at unchanged assembled
bytes; the §17 opt-sweep owns byte-changers.

## §4 — NAMED REMAINDER: #12 boot_data (DESIGN FINDING — `.emp` has no binary embed)

`engine/system/boot_data.asm` (`BootData` (a5)+ cursor table) CANNOT port to a straight
`.emp` data section: the table EMBEDS the resident Z80 sound blob mid-record via
`BINCLUDE "…z80_sound_blob.bin"` (a sigil-emitted binary of shape-varying size), and
the no-sound arm uses a conditional `org $3FE` hole. **The `.emp` language has no
binary-embed / incbin construct** — `BINCLUDE` exists ONLY in `sigil-frontend-as` (the
AS-comprehension frontend), not the `.emp` language (grep: `directive_binclude` is
sigil-frontend-as-only). Porting BootData needs a new `.emp` surface (embed an external
binary blob as a data-item, mid-struct). This is the census's own flag ("Data-DSL
demand, ledger 1526 row-46 boot's cursor protocol"). **STOP per the instruction: a real
new language surface is required.** Recommendation: a dedicated `embed`/`incbin` data
construct spec, or accept AS-authored `boot_data.asm` as a standing "100% .emp"
exception (like the vendored debugger #6) — the table is 68 hand-tuned hardware bytes +
a binary blob, not readability-limited by AS.

## §5 — NAMED REMAINDER: the demo tree #14/#15/#17/#18/#20 (INFRASTRUCTURE-BLOCKED)

**Premise correction (numbers):** the census scoped these as effort-S "PORT" each. But
the demo profile has **ZERO native game modules today** — `demo_registry()` (native.rs)
is engine-only (30 engine gates, no `games.demo.*` ROM module), and there are NO demo
game-data pins, NO demo game gates, NO demo frozen-table game anchors. So each demo
ROM-data/code port (#17 demo_data, #18 demo_state code, #20 demo_box code) is BLOCKED
on first building the demo's native game-module placement path — a new pins block +
`demo_registry` ROM entries + `demo_code_gates` + `demo.txt`/`demo_debug.txt` boundary
anchors + port tests. That is the parcel-sized infrastructure build the sonic4 side got
incrementally across conv-d/parallax/#35, NOT the mechanical effort-S the census
assumed. #14 (config constants) additionally needs a demo `harvest_game_constants`
extension (the conv-f #21 shape, for the demo profile). #15 (game.asm) is mostly a
documented remainder like sonic4 #22 (header-string ROM data + defined-not-invoked
hook macros + Game_Entry cross-seam label + a `-D` gate). **Recommendation:** run the
demo native game-module path as its own parcel (H-demo), sequenced like conv-d was for
sonic4; #16 demo config ram.emp (already DONE, item #7c) is the one demo game module
that exists and it is RAM-only (no ROM placement) — which is exactly why it landed
without this infrastructure.

## §6 — GAP-LEDGER SWEEP (→ campaign-gap-ledger.md)

1. **`.emp` binary-embed / incbin construct** (#12 blocker) — a data-item that embeds an
   external binary file mid-struct. Unblocks boot_data + any future ROM-data island that
   splices a compressed/emitted blob.
2. **Demo native game-module placement path** (#5 remainder) — the demo analog of the
   sonic4 registry/pins/frozen-anchor machinery; unblocks the demo tree end-to-end (the
   "new-game pure-.emp path" Volence wants proven).

## §7 — KILL-LIST (→ twin-scaffolding-kill-list.md)

- #34: the AS `OJZ_Act1_Descriptor`/`OJZ_Act1_Sections` twin (kill row 93) — DELETED.
- #35: `test_mappings.asm` DELETED (no scaffolding survives; the `.emp` is sole source).
- #46: `pitcher_plant/anims.asm` DELETED.
