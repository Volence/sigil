# 2026-08-01 — ITEM #7b: engine/ram.asm → the `vars` region form (CHECKPOINT — cross-seam STOP for countersign)

Status: **STOP for the spec-owner's countersign on the cross-seam mechanism.**
Branch `item7b-engine-ram` (sigil + aeon). Not merged. The `place_sections`
RAM-skip (the brief's stated first task) is DONE, committed, and green. The port
itself is BLOCKED on a spec-vs-reality gap in §3.3's cross-seam assumption that
is above a porter's unilateral call (the "do not hack around" clause). Everything
needed to finish once the mechanism is ruled is captured below (the port map is
design-complete).

## §0 — HEADLINE

- **DONE (sigil, committed):** `resolve::place_sections` now SKIPS reserve-only
  RAM sections (`vma_origin >= $F00000`) so a region-form `vars` section
  (`upper_ram`/`lower_ram`) no longer trips the name→ROM-region match. Test
  `place_sections_skips_ram_reserve_section` (green). This is spec §8.3's #7b-first
  task, and it is the ONLY change that is resolution-independent + risk-free.
- **BLOCKED (the real port):** deleting `engine/ram.asm` and moving its labels to
  `engine/ram.emp` makes the engine RAM labels `.emp` symbols. Three residual-AS
  seams then reference those labels EAGERLY (not link-deferred), and the AS
  frontend hard-errors on an unresolved symbol in those positions. Spec §3.3
  ("residual .asm reads RAM labels … pub vars fields/marks export identically —
  **no harvest needed**") is FALSE against the frontend as built. Resolving it
  changes either the AS frontend or the harvest architecture — a spec-owner call.

## §1 — WHAT SHIPPED (sigil, committed)

`crates/sigil-frontend-emp/src/resolve/mod.rs` — `place_sections`: at the top of
the placement loop, `if sec.vma_origin() >= 0x00F0_0000 { continue; }`. The
reserve-only RAM section is VMA-placed at its region base already and flattens
zero image bytes, so its `lma` is irrelevant and it must not be matched against
the ROM map. `$F00000` is the exact `native::is_rom_section` threshold (RAM lives
at `$FFFF0000`/`$FFFF8000`+). Test in `tests/region_lower.rs`:
`place_sections_skips_ram_reserve_section` — a module lowering to BOTH a RAM
reserve section (`upper_ram`) and a ROM data section (`rom_sec`); the map declares
only the ROM region; asserts no error (RAM skipped), ROM placed at its region
base, RAM VMA base preserved. `region_lower` 16/0, `tranche8_negative_probes`
(the other `place_sections` caller) 7/0.

## §2 — THE CROSS-SEAM GAP (empirically confirmed, three interlocking parts)

The build assembles the residual AS and lowers the `.emp` modules into ONE joint
`resolve_layout`, so labels cross-resolve. `.emp`→AS references (every RAM label
consumer today) work because the LINKER resolves them. The reverse — AS
referencing an `.emp` label — works ONLY where the AS frontend DEFERS an
unresolved symbol to a link fixup. It defers in exactly three places:
`jsr`/`jmp` bare targets (`JmpJsrSym`), `dc.l`/`dc.w` bare `Sym` (Abs32/16Be),
and `lea (Sym).w/.l, aN` (`try_defer_lea_abs`). It does NOT defer an unresolved
symbol used as an **absolute-EA data operand** — it hard-errors.

### Gap 1 — AS absolute-EA writes to engine RAM labels (the blocker)

After the port, these LIVE residual-AS sites reference `.emp` RAM labels as
absolute-EA operands that the frontend cannot defer:

| site | statement | frontend path | result |
|---|---|---|---|
| `games/demo/demo_state.asm:9` | `move.b #$0F, (Palette_Dirty).w` | `convert_one_atom_m68k` M68kAbs → `fold_imm` | **error** (unresolved) |
| `demo_state.asm:18–19` | `move.l #0, (Camera_X).w` / `(Camera_Y).w` | `try_defer_long_imm` `[Imm,M68kAbs]`: dest folded EAGERLY (see its comment) | **error** |
| `demo_state.asm:25` (`setVDPReg` macro, `engine/macros.asm:252-253`) | `move.b val,(VDP_Shadow_Table+reg).w` + `ori.l #(1<<reg),(VDP_Dirty_Mask).w` | eager M68kAbs / generic | **error** |
| `demo_state.asm:27` | `move.l #GameState_Demo, (Game_State).w` | as Camera_X | **error** |
| `demo_state.asm:4` | `lea (Palette_Buffer).w, a1` | `try_defer_lea_abs` | **OK (defers)** |

demo_state.asm is `include`d by `games/demo/main.asm` (live in the demo + demo.debug
builds), and #7b's bar is demo/demo.debug byte-identity — so this is a hard
blocker, not a corner case. (All other AS "references" the label-sweep flagged —
`main.asm`'s `Debug_Scene_Freeze`, `game.asm`'s `Ring_Sfx_Speaker`,
`caves.asm`'s `Camera_Y`, and the `DMA_*`/`Ring_Buffer`/`Static_Pal_Line0`
mentions in `macros.asm` — are COMMENTS or macro-parameter examples, not real
operands. The real operand set is exactly the table above.)

### Gap 2 — `phase Engine_RAM_End` (the AS game-RAM continuation)

`games/{sonic4,demo}/config/ram.asm` (still AS in #7b — #7c ports them) open with
`phase Engine_RAM_End`. `directive_phase` calls `eval_all` EAGERLY (a phase
displacement must be known at assemble time — it shifts every subsequent label),
so `Engine_RAM_End` MUST resolve during AS assembly. As an `.emp` `mark` it does
not. Same class as Gap 1 (AS needs an eager value for an `.emp`-owned address),
same resolution.

### Gap 3 — game-config constants sized into ENGINE RAM (a smaller wiring gap)

`engine/ram.asm` sizes three arrays with GAME-config constants that are NOT in
`engine/system/constants.emp` and NOT in the harness `emp_defines`:
`RING_BUFFER_ENTRY_SIZE`, `COLLECTED_SLOT_SIZE`, `COLLECTED_PARK_SLOTS`,
`COLLECTED_PARK_ENTRY_SIZE` (all game-config-only; only `MAX_RING_BUFFER` +
`COLLECTED_WINDOW_SLOTS` are seeded to `.emp` today). `ram.emp` cannot lower
without them. This one is NOT architectural — it is mechanical: add the four to
each profile's `emp_defines` (mirrors the existing `MAX_RING_BUFFER` row), values
from `games/{sonic4,demo}/config/constants.asm`. Flagged so it is not missed; not
a STOP by itself. (`SFX_RING_DEPTH`/`band_entry`/`EntityScanState_len` etc. are
engine-owned — `engine.constants` or harvested struct sizeofs — and are fine.)

## §3 — THE DECISION (spec-owner countersign requested)

Gap 1 (and Gap 2, same cause) forces a choice the spec did not anticipate:

**Option A — extend the AS frontend to defer unresolved absolute-EA operands**
(realizes §3.3 as written; no harvest). Extend the existing deferral family
(`try_defer_lea_abs`/`try_defer_long_imm` + a `move.b`/`ori` abs-dest path) so an
unresolved `(Sym).w/.l` destination emits a width-pinned `Abs16Be`/`Abs32Be`
fixup instead of erroring. `Engine_RAM_End` still needs an eager value for
`phase`, which this does NOT give (phase is not an operand) — so Option A alone
does not close Gap 2; it would still need a value bridge for the phase.
- Pro: `.emp` `pub` RAM labels stay the single link authority; matches spec §3.3;
  repin snapshots the `.emp` section directly.
- Con: an AS-frontend byte-surface change (only the current error path, so
  byte-neutral for resolved operands — but must be PROVEN by the six CRCs);
  several instruction shapes; does not solve `phase`.

**Option B — harvest engine-RAM addresses → seed as plain AS `defines`** (values
only, NOT `guarded_defines`, so NOT re-exported as `EquSym`s). `.emp` `pub vars`
labels remain the link-export authority; the harvest gives the AS side EAGER
values so its absolute-EA operands bake the correct address and `phase
Engine_RAM_End` resolves. No AS-frontend change; closes Gaps 1 AND 2 uniformly.
- Pro: reuses the proven harvest infrastructure (constants/struct-offset flips);
  one mechanism for both gaps; zero AS-frontend risk; `Engine_RAM_End` is just
  another harvested value (no double-export — plain `defines` don't emit link
  symbols, verified: `run_impl` seeds `defines` into the env but only
  `attach_guarded_equ_exports` exports, and it exports `guarded_defines` only).
- Con: contradicts spec §3.3's "no harvest needed" — the emp labels are the link
  authority, but the AS side reads harvested VALUES (a bridge, not a second
  author). Harvest must be shape-specific (the debug prof block shifts every
  label after it) and needs a focused `ram.emp` lower (its `use` comptime deps).

**Porter recommendation: Option B.** It is the lowest-risk faithful bridge,
handles Gap 2 for free, and keeps the `.emp` `pub vars` block as the address
authority (so §3.3's INTENT — labels not equates — holds; only the "no harvest"
sentence relaxes to "a value-only harvest bridges AS's eager-operand gap until
#7c ports the AS game-RAM/demo_state consumers"). It sidesteps a byte-surface
AS-frontend change on the critical ROM-identity path. If the overseer prefers
Option A (keep §3.3 literal), the `ram.emp` authoring + `place_sections` work is
100% reusable; only the bridge swaps.

Note: whichever option, the `.emp` `pub vars` labels are exported; Gap-1's
duplicate-symbol hazard for `Engine_RAM_End` is avoided because the harvest
(Option B) seeds a PLAIN define (no `EquSym`), and Option A does not harvest at
all. `link()` treats ANY duplicate name (label-vs-label, equ-vs-label) as a hard
error regardless of value — so a harvested `guarded_define` of a name the `.emp`
also `pub`-exports would collide; plain `defines` (Option B) do not.

## §4 — THE CONSTRUCT-BY-CONSTRUCT PORT MAP (design-complete; authoring is mechanical once the bridge is ruled)

`engine/ram.emp` = `module engine.ram` + `use engine.constants`, `use
engine.structs.{DMAEntry, VdpShadow, parallax_config}` (for `sizeof`),
`use engine.objects.sst.Sst`, `use engine.objects.entity_window.EntityScanState`,
`use engine.level.parallax.band_entry` — then two `region` decls + two `pub vars`
blocks (single owner module). Reachability: add `use engine.ram` to the harness
synthetic entry (`synthetic_entry_src`), so its reserve section is built in both
sonic4 + demo shapes (both need engine RAM).

Region decls (spec §2.1 / §4):
```
pub region lower_ram @ $FFFF0000 .. $FFFF8000
pub region upper_ram @ $FFFF8000 .. SYSTEM_STACK, w_addressable
```
- `lower_ram` limit `$FFFF8000` ⇒ replaces `if Lower_RAM_End > $FFFF8000 error`.
- `upper_ram` limit `SYSTEM_STACK` ⇒ replaces `if Engine_RAM_End >= SYSTEM_STACK
  error`; `w_addressable` ⇒ replaces `if (Object_RAM & $FFFF) < $8000 error`
  (strictly stronger — covers every symbol, not just Object_RAM).

Field mapping (every ram.asm construct):
| ram.asm | ram.emp |
|---|---|
| `Name: ds.b N` | `Name: [u8; N]` (or `Name: u8` for N=1) |
| `Name: ds.w N` / `ds.l N` | `Name: [u16; N]` / `[u32; N]` (scalars for N=1) |
| `ds.b SST_len` (Player_1/2) | `Player_1: Sst` (typed; `sizeof(Sst)` bytes) |
| `ds.b SST_len * NUM_DYNAMIC` | `[Sst; NUM_DYNAMIC]` |
| `ds.b DMA_CRITICAL_SLOTS*DMAEntry_len` | `[u8; DMA_CRITICAL_SLOTS * sizeof(DMAEntry)]` (or `[DMAEntry; DMA_CRITICAL_SLOTS]`) |
| `ds.b VDP_Shadow_len` | `[u8; sizeof(VdpShadow)]` |
| `ds.b band_entry_len*MAX_PARALLAX_BANDS` | `[u8; sizeof(band_entry) * MAX_PARALLAX_BANDS]` |
| `ds.b MAX_TRACKED_SECTIONS*EntityScanState_len` | `[u8; MAX_TRACKED_SECTIONS * sizeof(EntityScanState)]` |
| anonymous `ds.b 1` / `ds.b 2` even-pad | `pad(1)` / `pad(2)` |
| `Name:` (marker label) | `mark Name` |
| `Art_Staging_Buffer = Tile_Cache_Nametable` | `Art_Staging_Buffer: alias(Tile_Cache_Nametable)` |
| `if ART_STAGING_BUFFER_SIZE > TILE_CACHE_NT_SIZE error` | `ensure(ART_STAGING_BUFFER_SIZE <= TILE_CACHE_NT_SIZE, "…")` |
| `align 256` (Pos_table is `ds.b 256` — no align in engine; the 256-align is in GAME ram, #7c) | n/a for engine; `@align(256)` is #7c's Player_Pos_Ring |
| `ifdef __DEBUG__ … (Prof block, size-VARYING) endif` (`ram.asm:192-217`) | `if DEBUG == 1 @shape_divergent { DMA_Bytes_ThisFrame: u16, … Debug_Scene_Freeze: u8, pad(1) }` |
| `ifdef __DEBUG__ Dynamic_Live_Walking: ds.b 1 else ds.b 1 endif` (size-EQUAL, `ram.asm:453-464`) | `if DEBUG == 1 { Dynamic_Live_Walking: u8 } else { pad(1) }` (compiler proves invariance — no annotation) |

Two-block structure of the upper region (needed for the `Engine_RAM_End` mark
under Option B — see below):
- `pub vars upper_ram { RAM_Start … Block_Stage_ZeroPage }` — every exported
  engine RAM label, in ram.asm order.
- `Engine_RAM_End`: under Option B, place `mark Engine_RAM_End` in a TRAILING
  **non-`pub`** `vars upper_ram { mark Engine_RAM_End }` block (single-owner allows
  multiple source-order blocks) so it is a harvest anchor but NOT a link export
  (avoids colliding with the plain-define value the AS side reads). Under Option A
  it can be a plain `pub mark` in the main block (no harvest); Gap 2 then needs a
  separate value bridge for `phase`.

Comments: port the present-tense contract comments (the §-refs, the bit-15/pad
rationale, the tail-placement rationale for HBlank_Vector_Slot / Dynamic_Live /
the prefetch memo block) verbatim-adapted; NO change-history narration (house
rule). The `Debug_Scene_Freeze` sits inside the `@shape_divergent` DEBUG group,
exactly as ram.asm has it inside `ifdef __DEBUG__`.

## §5 — CROSS-SEAM CONSUMERS AFTER THE PORT (how each resolves)

- `.emp` consumers (all resolve at the joint link against `ram.emp`'s `pub`
  labels — identical to how they resolve against `ram.asm` today):
  `boot.emp` (`RAM_Start`, bare), `parallax.emp`
  (`extern("Parallax_State"/"Parallax_State_End")`), `core.emp`
  (`extern("Object_RAM"/"Object_RAM_End")`), and every `lea RamLabel`/`extern(..)`
  in tile_cache/section/plane_buffer/etc.
- AS consumers: the Gap-1 operand table (needs the bridge) + `phase
  Engine_RAM_End` (Gap 2). The `lea (Palette_Buffer).w` already defers.

## §6 — GATE STATE (this checkpoint)

- `place_sections` change: `region_lower` 16/0, `tranche8_negative_probes` 7/0.
- Full strict suite / six CRCs: NOT re-run for the port (the port is not wired —
  `ram.asm` still present, `ram.emp` not authored). Baseline 2864/0/4 is
  undisturbed by the `place_sections` skip (it is inert until a RAM section
  reaches placement, which no aeon build does yet — same argument as #7a's
  six-CRC identity). The `place_sections`-skip commit is on top of the chain-9
  baseline; the aeon tree is UNTOUCHED (ram.asm intact).

## §7 — NEXT STEPS (once the bridge is ruled)

1. (If Option B) add `harvest_ram_labels(aeon, debug)` — focused `ram.emp` lower,
   read every section label VMA, seed as plain `defines` in `assemble_as_side`
   (before assembly). (If Option A) extend the AS abs-EA deferral family + a
   value bridge for `phase Engine_RAM_End`.
2. Add the four game-config constants (Gap 3) to both profiles' `emp_defines`.
3. Author `engine/ram.emp` per §4; add `use engine.ram` to `synthetic_entry_src`.
4. Delete `engine/ram.asm`; drop `include "engine/ram.asm"` from `engine/engine.inc`.
5. `cargo run -p sigil-harness --bin repin`; prove the pins.rs RAM-cell diff is
   ZERO both shapes (address identity); `ram_packing_invariants_{plain,debug}`
   green; six-target byte-identity to chain-9; strict 2864/0/4 + the new
   placement test; retirements enumerated with re-homes.
6. Kill-list: the `place_sections` RAM-skip acquires its kill row when #7c lands
   the cross-module chain (per #7a note §6). No kill-list closures this parcel.

## §8 — STEP-3 / STEP-5 / GAP-LEDGER

- **Step-3 (retrospect / language-ask):** §3.3's cross-seam claim needs
  amending — "residual .asm resolves `.emp` RAM labels through the existing link
  path" is TRUE only for the link-deferred positions (jsr/jmp, dc.l/dc.w, lea
  abs); it is FALSE for absolute-EA data operands and for `phase` (eager). Two
  language/tooling asks fall out: (a) an AS-frontend "defer unresolved absolute-EA
  operand" capability (Option A — generalizes the existing deferral family), and
  (b) the recognition that `phase <emp-label>` is fundamentally eager, so the
  engine→game RAM chain is always a harvest/`after()` bridge until BOTH sides are
  `.emp` (#7c's `region game_ram @ after(upper_ram)` removes it entirely).
- **Step-5 (engine opt):** none — no engine byte moved; ram.asm untouched.
- **Gap-ledger:** no new jotted row (the §4 map has no unimplemented nice-to-have;
  the region form already shipped in #7a). Rows 153/157/165 close with #7c per the
  spec, not here.
