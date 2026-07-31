# 2026-08-01 — ITEM #7b: engine/ram.asm → the `vars` region form (CLOSE PACKET)

Status: **DONE — branches unmerged for the overseer's countersign.** Branches
`item7b-engine-ram` (sigil + aeon). The engine RAM layout is authored in
`engine/ram.emp` (the region form); `engine/ram.asm` is deleted; the residual AS
reads the addresses it needs eagerly via the Option-B harvest bridge. **All six
targets byte-identical to chain-9; repin RAM-cell diff ZERO both shapes; strict
2866/0/4.**

Overseer ruling (spec commit `2d9518f0`, §3.3 rewritten + §9 decision record):
**Option B — value-only harvest → plain AS defines; `.emp pub vars` labels are
the sole link authority.** Implemented exactly as ruled. §7 below preserves the
original STOP analysis as the §9 evidence.

## §0 — HEADLINE

`engine/ram.emp` = `module engine.ram` + two `region` decls + two `pub vars`
blocks (single owner), faithful to `ram.asm` per spec §4. The three `if/error`
guards became region `limit`s + `w_addressable`; the buffer-reuse became
`alias()`+`ensure`; the two `ifdef __DEBUG__` blocks became `if DEBUG == 1`
groups (prof block `@shape_divergent`; the size-equal `Dynamic_Live_Walking`
if/else compiler-proven). `engine.inc` drops the include. The AS side gets the
RAM addresses it references eagerly (demo_state writes + `phase Engine_RAM_End`)
from `harvest_engine_ram_addresses`, which lowers `ram.emp` through the SAME
`lower_regions` path the real build uses (§3).

## §1 — GATES (failures-first)

| gate | result |
|---|---|
| full strict `cargo test --workspace` | **2866 / 0 / 4** (2864 baseline + 2 new tests) |
| `native_full_rom` (six-target byte identity) | 3 / 0 |
| `ram_packing_invariants_{plain,debug}` | 2 / 0 |
| `repin` RAM-cell diff (both shapes) | **`pins.rs unchanged`** (zero diff) |
| `region_lower` | 17 / 0 (+`place_sections_skips_ram_reserve_section`, +`conditional_group_accepts_comparison_condition`) |

Six CRCs (all == chain-9 tips):

| target | built | chain-9 |
|---|---|---|
| s4 | `6cf74e65` / 412127 | `6cf74e65` / 412127 ✓ |
| s4.debug | `16615e46` / 421958 | `16615e46` / 421958 ✓ |
| demo | `9bb8c993` / 90506 | `9bb8c993` / 90506 ✓ |
| demo.debug | `bc7678d0` / 93006 | `bc7678d0` / 93006 ✓ |
| config_a | `78df5e6a` / 422297 | (strict `native_offcanonical` gate) ✓ |
| config_b | `f38f609b` / 303501 | (strict `native_offcanonical` gate) ✓ |

## §2 — ADDRESS-IDENTITY PROOF

- **`repin` regenerated `pins.rs` → `pins.rs unchanged`** (git diff empty). Every
  RAM cell (both plain + debug columns) is byte-identical, so every engine RAM
  label sits at the exact address `ram.asm` authored it — the port is an
  ADDRESS-EXACT ownership move. Spot-validated against the golden pins during
  bring-up: `Tile_Cache_Nametable` $FFFF0000, `RAM_Start` $FFFF8000, `Object_RAM`
  $FFFF89EE/$FFFF8A12, `Camera_X` $FFFFA11C/$FFFFA140, `Ring_Buffer`
  $FFFFA914/$FFFFA938, `Dynamic_Live` $FFFFB00C/$FFFFB030, `Art_Staging_Buffer`
  aliased to $FFFF0000; `Engine_RAM_End` $FFFFB3CA/$FFFFB3EE (the +$24 debug shift
  = the 36-byte `@shape_divergent` prof block).
- **RAM addresses flow into ROM operands** (the fold-identity contrapositive), and
  all six ROMs are byte-identical — so the address identity is real, not vacuous.

## §3 — THE OPTION-B BRIDGE (single layout authority)

`harvest_engine_ram_addresses(aeon, profile)` (sigil-harness `native.rs`):

1. Scans the manifest, publicizes the comptime helpers `ram.emp` reads
   (`engine.types/coords/constants/structs/objects.sst`), injects a synthetic
   `use engine.ram`-only entry, and runs `build_program_open_embed` — **the SAME
   `lower_regions` code path the real build lowers `ram.emp` through**. Reads the
   resolved RAM section labels (`vma_origin >= $F00000`) + alias equates as
   `(name, address)` pairs. Drift between harvest-time and lower-time addresses is
   impossible BY CONSTRUCTION: one code path, one comptime env (`profile.emp_defines`,
   shape-specific via `DEBUG`).
2. `assemble_as_side` seeds these as PLAIN value defines (NOT `guarded_defines` →
   NOT re-exported as `EquSym`s). The AS side folds its eager absolute-EA operands
   and `phase Engine_RAM_End` at comptime; the `.emp pub vars` labels remain the
   sole joint-link authority. No duplicate symbol (plain defines emit no link
   symbol; verified: `run_impl` seeds `defines` into the env, only
   `attach_guarded_equ_exports` exports and it exports `guarded_defines` only).
3. `synthetic_entry_src` gains `use engine.ram` so the real build builds ram.emp's
   reserve section and exports its labels (both sonic4 + demo).

Requirement #4 (game configs still assemble against the harvested `Engine_RAM_End`):
CONFIRMED — both `games/{sonic4,demo}/config/ram.asm` `phase Engine_RAM_End` and
all four game ROMs build byte-identical. They stay AS in #7b (#7c ports them to
`region game_ram @ after(upper_ram)`, retiring the harvest of `Engine_RAM_End`).

## §4 — CONSTRUCT-BY-CONSTRUCT PORT MAP (realized)

| ram.asm | ram.emp |
|---|---|
| `phase $FFFF0000 … dephase` | `region lower_ram @ $FFFF0000 .. $FFFF8000` + `pub vars lower_ram {}` |
| `phase $FFFF8000 … dephase` | `region upper_ram @ $FFFF8000 .. SYSTEM_STACK, w_addressable` + `pub vars upper_ram {}` |
| `Name: ds.b/w/l N` | `Name: [u8/u16/u32; N]` (scalar for N=1) |
| `Player_1: ds.b SST_len` / pools | `Player_1: Sst` / `[Sst; N]` (typed) |
| `ds.b N*DMAEntry_len` / `ds.b DMAEntry_len` | `[u8; N*sizeof(DMAEntry)]` / `[u8; sizeof(DMAEntry)]` |
| `ds.b VDP_Shadow_len` | `[u8; sizeof(VdpShadow)]` |
| `ds.b band_entry_len*N` / `ds.b N*EntityScanState_len` | `[u8; BAND_ENTRY_LEN*N]` / `[u8; N*ENTITY_SCAN_STATE_LEN]` (local const + drift ensure) |
| anonymous `ds.b 1/2/4` pad | `pad(1)` / `pad(2)` / `pad(4)` |
| `ds.b (…)&1` (defensive even-pad) | `pad((COLLECTED_PARK_SLOTS*COLLECTED_PARK_ENTRY_SIZE)&1)` |
| `Name:` marker | `mark Name` (RAM_Start, DMA_Queue, DMA_*_End, Parallax_State/_End, Object_RAM/_End, Lower_RAM_End, Engine_RAM_End) |
| `Art_Staging_Buffer = Tile_Cache_Nametable` | `Art_Staging_Buffer: alias(Tile_Cache_Nametable)` |
| `if ART_STAGING_BUFFER_SIZE > TILE_CACHE_NT_SIZE error` | `ensure(ART_STAGING_BUFFER_SIZE <= TILE_CACHE_NT_SIZE, …)` |
| `if Lower_RAM_End > $FFFF8000 error` | `lower_ram` limit `$FFFF8000` (`[region.overflow]`) |
| `if Engine_RAM_End >= SYSTEM_STACK error` | `upper_ram` limit `SYSTEM_STACK` (`[region.overflow]`) |
| `if (Object_RAM & $FFFF) < $8000 error` | `upper_ram` `w_addressable` (`[region.not-w-addressable]`) |
| `ifdef __DEBUG__` (size-VARYING prof block) | `if DEBUG == 1 @shape_divergent { … }` |
| `ifdef __DEBUG__ X else pad endif` (size-EQUAL) | `if DEBUG == 1 { Dynamic_Live_Walking: u8 } else { pad(1) }` (invariance compiler-proven) |

Struct-size sourcing (per-site call, §4): `sizeof()` for the comptime-only-module
structs (`Sst`/`DMAEntry`/`VdpShadow` — pulled lightweight into the harvest);
local `const` + `ensure(extern("X_len") == const)` drift guard for the two whose
owners are CODE modules (`EntityScanState` in entity_window, `band_entry` in
parallax — importing them would drag whole code modules into the focused harvest).

## §5 — RETIREMENTS (consciously-retired walls, re-homed §4 template)

1. **`parcel_8b_stage_gen_touchers::block_stage_keys_has_exactly_three_touchers`** —
   the `.emp` toucher census now sees `ram.emp`'s `Block_Stage_Keys:` field
   (a `vars` declaration, outside any proc) as an un-audited orphan. RE-HOME:
   exclude `engine/ram.emp` from the census — it is the label's DEFINITION home,
   exactly the role `engine/ram.asm`'s `Block_Stage_Keys:` declaration played
   before (residual AS, never `.emp`-scanned). Test green (1/0).
2. **`m1c_vector_table::vector_table_matches_reference_rom_first_256_bytes`** — its
   standalone `m1c_root.asm` `include`d the now-deleted `engine/ram.asm`, and its
   game-RAM `phase Engine_RAM_End` needs `Engine_RAM_End`. RE-HOME: drop the dead
   include; seed `harvest_engine_ram_addresses` (non-debug shape) into the test's
   plain defines — mirroring the real build's Option-B bridge. Test green (1/0).

No kill-list closures (as #7a foresaw: the region-`vars` scaffolding has no
twin-mirror row; it acquires one when #7c wires the cross-module `after()` chain).

## §6 — DEVIATIONS (recorded for countersign)

1. **Requirement #3 refined — the four sizes are LOCAL consts in `ram.emp`, NOT
   `emp_defines`.** The overseer's binding instruction said "add the four
   game-config size constants to the `emp_defines` profiles." On implementation I
   found all four (`RING_BUFFER_ENTRY_SIZE`, `COLLECTED_SLOT_SIZE`,
   `COLLECTED_PARK_SLOTS`, `COLLECTED_PARK_ENTRY_SIZE`) are **engine-INVARIANT**
   (byte-identical values in `games/sonic4` and `games/demo` config), and
   `RING_BUFFER_ENTRY_SIZE` is ALREADY a local `const` (=6) in `rings.emp` +
   `entity_window.emp` with an `ensure(extern(..)==..)` drift guard — the shipped
   convention for engine-invariant buffer-layout sizes. Seeding them as per-game
   `-D` defines would (a) mismodel invariants as game-varying, and (b) risk a
   `const`-vs-seeded-define collision in `rings.emp`/`entity_window.emp`. So I
   homed them the way the corpus already does: local `const` + `ensure(extern(..)
   == ..)` drift guards in `ram.emp`. This serves the requirement's GOAL (ram.emp
   resolves them, drift-guarded) with the correct model. **Flagged for override.**
2. **A #7a gap fixed en route:** the region-form group condition eval only accepted
   a bare integer (`if __DEBUG__`) — the ratified `if DEBUG == 1` (spec §8.1) is a
   comparison yielding `Value::Bool`, which `as_stored_int` rejected. Added
   `eval_group_cond` (a Bool selects its arm; a bare integer is nonzero-is-true).
   The #7a fixtures never exercised a `==` condition, so this surfaced only when
   `ram.emp`'s two `if DEBUG == 1` groups did. Committed with a positive test.

## §7 — DECISION-RECORD EVIDENCE (the original STOP that spec §9 cites)

The cross-seam gap that forced the Option-A/B decision (verified against the AS
frontend as built): after the port, three residual-AS seams reference engine RAM
labels EAGERLY — positions the frontend cannot defer to the linker.

- **Absolute-EA writes:** `games/demo/demo_state.asm` (live in the demo builds)
  does `move.b #$0F, (Palette_Dirty).w`, `move.l #0, (Camera_X).w`/`(Camera_Y).w`,
  `move.l #GameState_Demo, (Game_State).w`, + the `setVDPReg` macro
  (`move.b …,(VDP_Shadow_Table+reg).w`, `ori.l …,(VDP_Dirty_Mask).w`).
  `convert_one_atom_m68k`/`try_defer_long_imm` fold the absolute destination
  EAGERLY → hard error on an unresolved `.emp` symbol. (Only `lea (Palette_Buffer).w`
  defers, via `try_defer_lea_abs`.)
- **`phase Engine_RAM_End`:** `directive_phase` folds its argument eagerly.

Spec §3.3's "no harvest needed — pub vars labels export identically" was FALSE
there. Option B (value-only harvest → plain AS defines; `.emp` labels stay the
authority) resolves both uniformly with zero AS-frontend byte-surface risk, using
the proven harvest infrastructure. Ruled + implemented.

## §8 — STEP-3 / STEP-5 / GAP-LEDGER / CROSS-SEAM

- **Step-3 (retrospect / language-ask):** (a) the region-group condition needed the
  `Value::Bool` fix to honor the ratified `if DEBUG == 1` — the #7a group-condition
  eval was integer-only (now fixed; a language-conformance catch). (b) `sizeof` of
  a struct whose fields carry newtypes requires the type vocabulary in the
  consuming module's scope (`use engine.types.*` in ram.emp) — a cross-module
  `sizeof` ergonomics note for the ledger. (c) importing a DERIVED `pub const`
  evaluates its RHS in the CONSUMER's scope, so its base consts must be in scope
  too — the glob `use engine.constants.*` is the clean answer (matches how
  `tile_cache.emp` imports both bases and derivations).
- **Step-5 (engine opt):** none — no engine byte moved; the port is address-exact.
- **Gap-ledger:** the `vars / RAM regions` section (rows ~140–188) is now REALIZED
  by this port (the feature #7a built + this port consuming it); formal row closure
  rides #7c's census close per spec §6, not here. STALE-premise note (row ~353):
  "RAM labels that live ONLY in `engine/ram.asm`, never in any `.emp` file" is now
  false (`Ctrl_1_Held` & co are `ram.emp` labels); the port tests' synthetic-AS
  RAM-section workaround still passes (independent of where the real build sources
  them) — a doc-cleanup for #7c, no code impact. No new nice-to-have unimplemented.
- **Cross-seam resolution (how each RAM label resolves now):**
  - `.emp` consumers (`boot.emp` `RAM_Start`; `parallax.emp`
    `extern("Parallax_State*")`; `core.emp` `extern("Object_RAM*")`; every
    `lea RamLabel`/`extern(..)`): resolve at the joint link against `ram.emp`'s
    `pub` labels — identical mechanism to how they resolved against `ram.asm`.
  - AS consumers (demo_state abs-EA writes; `phase Engine_RAM_End`): fold the
    harvested plain value defines at comptime.

## §9 — COMMITS (unmerged)

- sigil `item7b-engine-ram`:
  - `52f5f2d1` place_sections RAM-skip + test (the pre-ruling first task).
  - (condition fix) `eval_group_cond` + `conditional_group_accepts_comparison_condition`.
  - (bridge) `harvest_engine_ram_addresses` + `use engine.ram` entry + the two
    retired-wall re-homes (parcel_8b, m1c_vector_table/m1c_root).
- aeon `item7b-engine-ram`:
  - `4fb9926` `engine/ram.emp` + `engine/ram.asm` deletion + `engine.inc` drop.
