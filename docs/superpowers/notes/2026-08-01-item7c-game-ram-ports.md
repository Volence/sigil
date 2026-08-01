# 2026-08-01 — ITEM #7c: the game RAM ports + the item-7 arc close (CLOSE PACKET)

Status: **DONE — branches unmerged for the overseer's countersign.** Branches
`item7c-game-ram` (sigil + aeon). Both game RAM files are authored in the `.emp`
region form (`games/{sonic4,demo}/config/ram.emp`), chained cross-module onto the
engine RAM (`game_ram @ after(upper_ram)`); the `.asm` files are deleted; the
residual AS reads the one game RAM address it needs eagerly via the extended
harvest. **All six targets byte-identical to chain-9; repin RAM-cell diff ZERO
both shapes; strict 2868/0/4 (2866 baseline + 2 new cross-module tests).**

This parcel CLOSES the item-7 arc (the conv-C census rows 5/23/16, gap-ledger
`vars/RAM regions`, the item-6 OUT-list) — the region-`vars` feature is fully
consumed, `ram.asm` is gone from the tree.

## §0 — HEADLINE

`games/sonic4/config/ram.emp` = `module games.sonic4.ram` + one `region game_ram
@ after(upper_ram) .. SYSTEM_STACK, w_addressable` + one `pub vars game_ram`
block (player physics table, quadrant/jump/death, the 256-aligned history rings).
`games/demo/config/ram.emp` = the empty-region analog producing `Game_RAM_End`.
The `phase Engine_RAM_End` continuation became the **cross-module `after(..)`**
chain; the `ifdef __DEBUG__` counters (`Dbg_Music_On`/`Dbg_Sfx_Sel`) became an
`if DEBUG == 1 @shape_divergent` group; `align 256` became `@align(256)`.

**Two engine mechanisms built this parcel needed (sigil):**
1. **Whole-program cross-module `after(..)` resolution** — `#7a` shipped the
   intra-file DAG + cycle detection; `#7c` extends resolution WHOLE-PROGRAM so a
   region can chain onto a region declared in another module. A dedicated pre-pass
   (`resolve_program_region_ends`, a fixpoint over the cross-module `after`-DAG)
   resolves every region's running end ONCE before the per-module lowering loop,
   and threads the ends back in so a chained region reads its parent's end. A
   parent resolves before its dependents **regardless of module order** (fixpoint,
   principled — not incidental).
2. **The per-game RAM harvest** — `harvest_engine_ram_addresses` now `use`s the
   game RAM module too (`profile.game_ram_module`), so the residual AS
   (`game.asm`'s `move.b #1,(Dbg_Music_On).w`) folds the game RAM address it reads
   eagerly. ONE layout authority preserved: the harvest and the real build lower
   through the SAME `lower_regions` path.

## §1 — GATES (failures-first)

| gate | result |
|---|---|
| full strict `cargo test --workspace` | **2868 / 0 / 4** (2866 baseline + 2 new cross-module tests) |
| `native_full_rom` (six-target byte identity) | ok |
| `native_offcanonical_full` / `_rom` / `_placement` (config_a/b) | ok |
| `ram_packing_invariants_{plain,debug}` | 2 / 0 |
| `repin` RAM-cell diff (both shapes) | **`pins.rs unchanged`** (zero diff) |
| `region_lower` | 19 / 0 (17 + `cross_module_after_chains_across_modules`, `cross_module_after_cycle_reported`) |
| `m1c_vector_table` (re-homed) | 1 / 0 (assertion ran, not skipped) |

Six CRCs (all == chain-9 tips):

| target | built | chain-9 |
|---|---|---|
| s4 | `6cf74e65` / 412127 | `6cf74e65` / 412127 ✓ |
| s4.debug | `16615e46` / 421958 | `16615e46` / 421958 ✓ |
| demo | `9bb8c993` / 90506 | `9bb8c993` / 90506 ✓ |
| demo.debug | `bc7678d0` / 93006 | `bc7678d0` / 93006 ✓ |
| config_a | `78df5e6a` / 422297 | `78df5e6a` / 422297 ✓ |
| config_b | `f38f609b` / 303501 | `f38f609b` / 303501 ✓ |

## §2 — ADDRESS-IDENTITY PROOF

- **`repin` regenerated `pins.rs` → `pins.rs unchanged`** (git diff empty). Every
  RAM cell (engine + game, both plain + debug columns) is byte-identical, so every
  game RAM label sits at the exact address `ram.asm` authored it — an ADDRESS-EXACT
  ownership move.
- Spot-validated game RAM (plain / debug): `Player_Phys` $FFFFB3CA / $FFFFB3EE (=
  Engine_RAM_End each shape); `Player_Pos_Ring` $FFFFB500 / $FFFFB600;
  `Player_Ring_Index` $FFFFB700 / $FFFFB800; `Game_RAM_End` $FFFFB702 / $FFFFB802.
  The +$100 debug shift = the `if DEBUG == 1 @shape_divergent` counters (2 words)
  + the align re-rounding one boundary higher — matching the retired `.asm` exactly.
- **RAM addresses flow into ROM operands** (the fold-identity contrapositive), and
  all six ROMs are byte-identical — so the address identity is real, not vacuous.

## §3 — THE `@align` SPEC-VS-REALITY CATCH (correctness-hardening)

The first port build placed `Player_Pos_Ring` at `$FFFFB400`, not the reference's
`$FFFFB500` — a 256-byte low bias on every ring address (8 divergent ROM operand
bytes). Root cause: my region `@align(256)` used a plain round-up
(`round_up(cursor, n)`), but AS's `align` INSIDE A PHASE (all Aeon RAM,
`disp != 0`) is NOT a plain round-up — the AS frontend's own `directive_align`
(asl 1.42, live-probed) advances by **`round_up(cursor + n, n)`**: ALWAYS at least
one full `n` beyond the cursor, even when already `n`-aligned. From cursor
$FFFFB3DE that gives $FFFFB500, not $FFFFB400.

Both results are 256-aligned (the author's stated intent — the ring low-byte wrap
needs a 256 boundary), so the intent held either way; but byte-identity to the
AS-authored ROM requires AS's specific choice. A `vars` region is the `.emp`
analog of an AS phased RAM section (VMA `$FFFF….`), so region `@align` must ALWAYS
use the in-phase regime. Fixed in `lower/regions.rs::Layout::align_to` (mirrors
`directive_align` exactly, valid for any `n`). **Spec §2.3's "next multiple of N"
wording is imprecise for the corpus and is refined to this** (stamped in the spec
status line). No other `.emp` uses `@align` (grep: game ram.emp is the sole
region-`@align` user in the whole corpus), so the change is contained; the two
`#7a` synthetic fixtures that hand-pinned the plain-round-up result were updated
with the corrected addresses + an explanatory comment.

## §4 — CONSTRUCT-BY-CONSTRUCT PORT MAP (realized)

sonic4 (`games/sonic4/config/ram.emp`):

| ram.asm construct | ram.emp spelling | address (plain) |
|---|---|---|
| `phase Engine_RAM_End … dephase` | `region game_ram @ after(upper_ram) .. SYSTEM_STACK, w_addressable` + `pub vars game_ram {}` | base = Engine_RAM_End $FFFFB3CA |
| `ifdef __DEBUG__ Dbg_Music_On/Dbg_Sfx_Sel ds.w 1` (size-VARYING, at top) | `if DEBUG == 1 @shape_divergent { Dbg_Music_On: u16, Dbg_Sfx_Sel: u16 }` | (debug only) $FFFFB3EE/$FFFFB3F0 |
| `Player_Phys:` marker | `mark Player_Phys` | $FFFFB3CA |
| `Phys_accel … Phys_release_cap: ds.w 1` (×8) | `Phys_accel: u16 … Phys_release_cap: u16` | $FFFFB3CA..$FFFFB3D8 |
| `Player_Phys_End:` | `mark Player_Phys_End` | $FFFFB3DA |
| `Player_Quadrant/JumpBuffer/Death_Pending: ds.b 1` | `…: u8` (×3) | $FFFFB3DA/DB/DC |
| anonymous `ds.b 1` pad | `pad(1)` | $FFFFB3DD |
| `align 256` | `@align(256)` (AS in-phase semantics — §3) | → $FFFFB500 |
| `Player_Pos_Ring/Stat_Ring: ds.b 256` | `Player_Pos_Ring: [u8; 256] @align(256)`, `Player_Stat_Ring: [u8; 256]` | $FFFFB500 / $FFFFB600 |
| `Player_Ring_Index: ds.w 1` | `Player_Ring_Index: u16` | $FFFFB700 |
| `if Player_Pos_Ring&$FF error` | subsumed by `@align(256)` (aligned by construction) | — |
| `Game_RAM_End:` | `mark Game_RAM_End` | $FFFFB702 |
| `if Game_RAM_End >= SYSTEM_STACK error` | region `limit` `SYSTEM_STACK` (`[region.overflow]`) | — |
| (new) whole-window `.w`-reachability | region `w_addressable` (`[region.not-w-addressable]`) | — |

demo (`games/demo/config/ram.emp`): `region game_ram @ after(upper_ram) ..
SYSTEM_STACK, w_addressable` + `pub vars game_ram { mark Game_RAM_End }` — the
empty region still produces `Game_RAM_End` at Engine_RAM_End (a region-form `vars`
was inert pre-#7a; now it allocates + marks).

`w_addressable` on `game_ram` is an ADDED invariant (the retired `.asm` had only
the `>= SYSTEM_STACK` overflow guard, no `.w` bit-15 check). It holds — game RAM
sits in `[$FFFFB…, $FFFFFF00)`, bit 15 set throughout — and is strictly-stronger
correctness for the hot `.w`-addressed player data. **Flagged as a hardening
addition for the countersign** (drop it if the overseer prefers the retired
`.asm`'s exact guard set).

## §5 — CROSS-MODULE `after(..)` — THE MECHANISM (sigil)

- `resolve_program_region_ends(modules, defines) -> (HashMap<name, end>, diags)`
  in `lower/regions.rs`: builds each region-owning module's ambient-prepended
  synthetic file (identical to the per-module loop's), then runs a **fixpoint**
  over the cross-module `after`-DAG, re-resolving every region module against the
  ends known so far until nothing moves. An acyclic chain of N modules settles in
  ≤ N passes; still-moving after N+1 ⇒ a cross-module `after(..)` cycle →
  `[region.chain-cycle]` (the whole-program analog of the intra-file cycle `#7a`
  already caught). Per-region layout diagnostics (overflow, unknown parent,
  odd-field) stay in the per-file `lower_regions` pass — one report site.
- `build_program_with` runs the pre-pass once (before the per-module loop) and
  threads the ends into every module's lowering via
  `lower_module_with_region_ends(file, opts, &region_ends)` — a new variant;
  `lower_module` delegates to it with an empty map, so all 285 existing callers +
  the single-file tests are UNAFFECTED (zero-ripple; no `LowerOptions` field
  churn across ~295 literal sites). `lower_regions`/`resolve_regions`/`resolve_one`
  take `external_ends`: a LOCAL `after(parent)` resolves intra-file as before;
  only a NON-local parent consults `external_ends`.
- **ONE layout authority** (spec §3.3): the pre-pass and the per-module emission
  both run `resolve_one`/`Layout` in `regions.rs`; the harvest lowers the same
  `ram.emp` through the same `lower_regions`. game_ram's base = the resolved
  `upper_ram` end everywhere, by construction.

## §6 — THE `emit_rom` RAM-SECTION FIX (latent bug surfaced)

Adding `game_ram` surfaced a latent bug: `sigil_link::emit_rom`'s validate loop
range-checked EVERY linked section against a ROM map region, including reserve-only
RAM sections (a `vars` region at `$FFFF….`). At chain-9 the engine RAM sections
happened to chain to LOW physical LMAs and slipped past validation (empty → no
image bytes); game_ram left them at their true high VMAs, so `lower_ram` LMA
`$FFFFF0000` hit "in no ROM region". `flatten` already documents that empty
(reserve-only RAM) sections "must never be range-checked against the image" —
`emit_rom` violated that contract. FIX: skip empty sections in the validate loop
(matching `flatten`). Byte-neutral (empty sections contribute nothing to the
image; a NON-empty section outside every ROM region is still the hard error) —
proven by the six goldens holding.

## §7 — RETIREMENTS / RE-HOMES (consciously handled)

1. **`m1c_vector_table` / `m1c_root.asm`** — `m1c_root.asm` still `include`d the
   now-deleted `games/sonic4/config/ram.asm` (for its front-matter fidelity + the
   game-RAM `phase`). RE-HOME (mirrors `#7b`'s engine re-home): drop the include
   (exactly as `main.asm`'s `gameRamIncludes` is now empty); the game RAM labels
   come from the harvest, which `sonic4_profile` now reaches via
   `game_ram_module`. Test green (assertion ran, not skipped).
2. **`region_lower.rs` `@align` fixtures** — `mark_alias_pad_align` +
   `chained_engine_game_fixture` hand-pinned the plain-round-up align result; both
   UPDATED to the AS in-phase result (§3) with explanatory comments. Not
   retirements — corrected expectations for a corrected semantics.

No test DELETIONS / ignores. Strict rose 2866 → 2868 (the 2 new cross-module
tests). No kill-list closures of pre-existing rows (per conv-C §9 there were none
keyed to the RAM port); **ADDED** kill-list row 97 — the RAM-address harvest
bridge — with its kill condition (the last eager AS RAM reader moving to `.emp`),
which `#7b` foresaw `#7c` would create.

## §8 — ARC-CLOSE BOOKKEEPING (same-commit)

- **conv-C census** (`2026-07-31-conversion-tail-census.md`): rows **5 / 16 / 23**
  flipped to ✅ DONE (5 = `#7b`; 16/23 = `#7c`); the "Parcel C = BLOCKED/PARKED"
  outcome block gets a "RESOLVED — path (1) taken, ALL THREE ports DONE" note.
- **gap-ledger** (`campaign-gap-ledger.md`): the `vars / RAM regions` section
  header gets a "REALIZED — item #7 shipped" banner; the three OPEN rows
  (conditional fields · checked buffer-reuse overlay · debug-layout-stability
  lint) flipped to ✅ CLOSED with the shipped spelling. Only the RAM-map-report
  row (pure tooling, no language surface) stays open.
- **kill-list** (`twin-scaffolding-kill-list.md`): row 97 added (the RAM harvest
  bridge, §7).
- **item-7 spec** (`specs/2026-08-01-item7-ram-regions-design.md`): one-line
  "✅ SHIPPED" status stamp at the top (+ the `@align` refinement note); the
  ratified body is otherwise untouched.

## §9 — STEP-3 / STEP-5 / NEITHER-BUCKET / GAP-LEDGER SWEEP

- **Step-3 (retrospect / language-ask):** (a) THE headline — region `@align`
  semantics are AS's in-phase `round_up(cursor + n, n)`, not the spec's "next
  multiple"; a corpus-reality refinement (§3). The corollary for any FUTURE region
  `@align` author: the phased regime always applies (regions are RAM-only), so an
  already-aligned base still pays a full `n`. (b) The cross-module `after(..)`
  ends want a single whole-program resolution pass, NOT a per-module fixpoint
  duplicated in the loop — the pre-pass shape is the clean answer and generalizes
  to any future cross-module region dependency (the map-file ordering manifest,
  when it lands, replaces the fixpoint with a declared order). (c) `emit_rom`'s
  validate loop should honor `flatten`'s empty-section contract structurally (§6)
  — done.
- **Step-5 (engine optimization):** none — no engine byte moved; the port is
  address-exact (six goldens hold).
- **Neither-bucket headlines:** (a) **The demo empty-region port is the cleanest
  proof the feature is whole** — a 9-line stub that produces `Game_RAM_End` at the
  cross-module chain point with zero fields, exactly what conv-C flagged as
  "un-portable while the region form is inert." (b) **The stale-binary trap cost
  real time** — several mid-debug builds used `cd sigil-main && cargo build`
  (master, no changes) instead of the worktree, so `SIGIL_BUILD` (the worktree
  binary) went stale and reported phantom errors; the lesson is to always build
  from the worktree dir (bare `cargo build`) and verify the binary reflects the
  change before trusting a build result.
- **Gap-ledger sweep:** the `vars/RAM regions` section is now REALIZED (§8); no
  new nice-to-have was left unimplemented (the port is complete + byte-exact). The
  RAM-map-report row stays the one open tooling item (no language surface).

## §10 — COMMITS (unmerged)

- sigil `item7c-game-ram`:
  - cross-module `after(..)`: `resolve_program_region_ends` + `external_ends`
    threading (`regions.rs`) + `lower_module_with_region_ends` (`lower/mod.rs`) +
    the pre-pass in `build_program_with` (`resolve/mod.rs`) + the two cross-module
    tests + the `@align` in-phase semantics fix + the 2 fixture updates.
  - the per-game RAM harvest extension + `synthetic_entry_src` game edge +
    `GameProfile.game_ram_module` (`native.rs`); the `m1c` re-home
    (`m1c_root.asm` + `m1c_vector_table.rs`).
  - `emit_rom` empty-section skip (`sigil-link/lib.rs`).
  - the arc-close bookkeeping (census / gap-ledger / kill-list / spec status).
- aeon `item7c-game-ram`:
  - `games/sonic4/config/ram.emp` + `games/demo/config/ram.emp`; delete both
    `.asm` files; empty both `gameRamIncludes` macros (`games/*/main.asm`).
