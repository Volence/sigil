# Parcel K5 — the order-drive flip + sigil.map.toml retirement (the K capstone closes)

**Porter: Opus (k5-order-drive branch, aeon + sigil worktree). Overseer/spec owner: Fable.**
K5 is the FINAL parcel of the K capstone. It flips the packer so the declared placement
map DRIVES section order (completing spec §1-fact-1 STAGE-2, ruled R2+R1 at K1) and retires
`sigil.map.toml` (the K1-deferred geometry-region duplication). Pure authority swap —
**full fold identity ×6, byte-for-byte, appendix included, NO re-freeze.** Spec:
`specs/2026-08-01-k-capstone-design.md` (§1 fact-1 STAGE-2, §5). Predecessor: K1
(`notes/2026-08-01-k1-map-authority.md`, the VALIDATE stage this DRIVES-inverts).

Commits (merge state lives in the campaign log, not here): sigil `7cea9f51` (order-drive flip),
`46101a02` (sigil.map.toml retirement); aeon `9284631` (the per-game map edits).

---

## §1 — The order-drive flip (sigil `7cea9f51`)

### What changed
`packed_true_bases` (the §17 Wave-B B-0 packing walk) previously computed the walk order by
`order.sort_by_key(prov)` — sorting the ROM sections by their FROZEN provisional bases. K5
replaces that sort: the walk order is now DRIVEN by the per-game map's declared `order` list.
Each ROM section carrying a map-declared min-offset head-label sorts by its **map rank**; a
zero-byte boundary section the map does not name rides the rank of the nearest preceding
named section by prov, then prov, then its stable index (a pure measurement-cache role — it
emits no bytes, so its slot never moves a ROM byte). `map_order` threads in through
`true_bases_by_index` from the two call sites (`build_rom_chained_with_listing`,
`resolve_frozen_sections`), which load it once from `profile.map_path`.

### Why it is fold-identical ×6 (the ground truth, probe-established)
For every shipped shape the byte-emitting sections' frozen provisional bases already ascend
in exactly the declared order. A drive-time probe on all six targets confirmed the invariant:
taking the sections in prov-walk order, the map ranks of the ranked sections are **strictly
increasing** — so ranking by the map reproduces the prov order byte-for-byte, while making the
DECLARATION (not the frozen table) the thing that authored it. The probe also confirmed the
only sections NOT in the map order are **zero-byte**: the label-less boot-region blobs (prov
< 0x100, `img = 0`) and the `EndOfRom` epilogue terminus (always last). No byte-emitting
section is ever unranked, and no unranked section emits a byte — so the merge (named by rank,
zero-byte boundary sections by inherited-rank+prov) is provably byte-neutral.

| target | ranked (map-driven) | only labeled-unranked | strict-increasing ranks |
|---|---|---|---|
| s4 | 72 | `EndOfRom` (0-byte) | ✓ |
| s4_debug | 73 | `EndOfRom` | ✓ |
| config_a | 75 | `EndOfRom` | ✓ |
| config_b | 68 | `EndOfRom` | ✓ |
| demo | 40 | `EndOfRom` | ✓ |
| demo_debug | 41 | `EndOfRom` | ✓ |

### The inverted validation semantics
K1 landed `validate_placement` as a VALIDATION of a frozen-DERIVED order (`derived ⊆
declared`, the subsequence check). K5 flipped the packer so the map DRIVES, and the pass
inverted to a post-resolve **DRIVE-CONFIRMATION** — the direction reversed (the declaration is
the input; this checks the build honoured it and is complete):
- `[map.order-undeclared]` (the inversion's teeth): the map DRIVES, so every byte-emitting
  section MUST be declared in `order`. A byte-emitting section the map omits fails loud (it
  could not be driven — it fell to its frozen provisional slot — so it is never silently
  placed). Zero-byte markers (`__BUDGET_DATA`, `EndOfRom`) are excluded.
- `[map.order-diverged]`: the resolved byte-emitting sequence must follow the declared
  positions strictly. Post-drive this can only fire on a packer BUG (the walk did not honour
  the declaration) — it is the drive's own guard.
- `[map.undeclared-island]` / `[map.anchor-absent]` / the hole check: unchanged (K1).

The negative probes stay green, updated to drive semantics. A NEW `drives_order_by_map_rank`
unit test proves the packer consumes the DECLARATION, not the frozen bases: two labeled
sections whose provisional bases would sort `Low`(0x100) before `High`(0x200) are declared in
the OPPOSITE order; the walk places `High` first (run head @ its prov 0x200) and packs `Low`
after it @ 0x210 — inverting the prov order — while the empty-map control falls back to the
prov sort (Low@0x100, High@0x110). This is the one test that DISAGREES the map with prov, so
it isolates that `map_order` drives.

---

## §2 — What the frozen tables STILL carry, and why (the demotion inventory)

The flip DEMOTES the frozen tables (`golden/offcanonical_sizes/{s4,s4_debug,config_a,
config_b,demo,demo_debug}.txt`) from ORDER AUTHORITY to a pure measurement cache. They no
longer author the sequence. Precisely what survives, and its sole justification:

1. **The provisional BASE of each labeled ROM section** (`frozen[L] − offset[L]`) — used for:
   - **Island anchor positions.** An island (run head, or a section opening a `> ANCHOR_GAP`
     gap past the running cursor, or a phase bank) is pinned absolutely at its provisional
     base. These positions coincide with the map's `[[anchor]]` declarations (0x0, 0x10000,
     0x48000, 0x58000) — the map DECLARES the anchor; the frozen base is the measured
     boundary key `validate_placement` confirms against it.
   - **Packed-section alignment** — `align_of(prov)` gives the power-of-two the provisional
     layout implies (byte-neutral at unchanged sizes, where the packed base already equals
     prov; it only re-derives padding under growth).
   - **Round-0 measurement pins** — the labeled sections are pinned at prov to measure image
     lengths at approximately-correct positions so relaxation (branch widths) settles right;
     the walk then re-measures at the packed bases to a fixpoint.
2. **Boundary keys** — `derive_frozen_table` resolves the frozen chain and reads each boundary
   label's ROM address back; the fixpoint (the derivation reproduces the committed table) is
   the proof sigil's own resolve is the authority.

So the frozen table records WHERE THINGS MEASURE, not WHAT ORDER THEY GO IN. Reordering the
map reorders the layout; a byte-emitting section the map omits fails loud. That is the flip.

**Spec-vs-reality:** no section class was found that cannot key stably post-K4 — every
byte-emitting section is a stable-named `.emp` module (or an AS-residual section with a
stable head-label already in the frozen table and the map). No STOP.

---

## §3 — sigil.map.toml retirement (sigil `46101a02` + aeon `9284631`)

The K1-deferred cleanup. `sigil.map.toml`'s region geometry (`rom`, `object_bank`,
`z80_moving_trucks_bank`) + the object-bank budget were DUPLICATED in `games/{sonic4,demo}/
map.toml` since K1. With the per-game maps now the placement authority, the project-wide file
retires. Census of the retirement + re-homes:

| retired / moved | re-home | note |
|---|---|---|
| `sigil.map.toml` (41 ln) | DELETED | region geometry + budget now sole-owned by the per-game maps |
| `native::project_memory_map()` | `project_memory_map(aeon)` → `games/sonic4/map.toml` | the only LIVE reader (the object-bank budget gate) |
| `build_native_rom_with_listing` PinnedBaked fallback | `sonic4_profile.map_path(aeon)` | dead for shipped Frozen builds; re-pointed for correctness |
| `native_object_bank_budget` gate (`placement_map()` + 2 `project_memory_map` calls) | `games/sonic4/map.toml` via `aeon_dir()` | identical object_bank region + budget cursor |
| `native_chained_resume` emit helper (feeds 2 RETIRED archaeology tests) | `aeon/games/sonic4/map.toml` | coherence even though `#[ignore]` |
| `map_load.rs` doc | "the per-game `games/<g>/map.toml`" | descriptive |
| demo `z80_moving_trucks_bank` region | SLIMMED AWAY | sound-off, nothing places there (past the ROM end @ 0x60000); byte-neutral |

The demo slim is proven byte-neutral by the demo/demo_debug rom + full gates (they compare
against the golden demo.bin/demo.debug.bin). No shipped ROM byte reads `sigil.map.toml` — the
Frozen emit path uses `profile.map_path` (the per-game map) throughout — so the deletion moves
zero ROM bytes.

---

## §4 — The bar, met (fold identity ×6, no re-freeze)

- **Six-target native gates GREEN (byte-identical vs the asl/golden ROMs):** `native_rom` 3/0,
  `native_offcanonical_rom` 7/0, `native_full_rom` 4/0, `native_offcanonical_full` 2/0,
  `native_declared_chain` 2/0, `native_offcanonical_placement` 8/0, `native_object_bank_budget`
  2/0.
- **Six CRCs = the chain-18 tips EXACTLY** (freshly recomputed from the golden blobs
  `refreeze --check` validated against the provenance chain, which the native gates prove the
  builds equal):
  - s4 `5f72b9c3/412134` · s4.debug `e6171a80/421970`
  - demo `55b70266/90576` · demo.debug `6487a47c/93073`
  - config_a `f92f0333/422305` · config_b `947e4c57/303555`
- **`refreeze --check`: OK (tip `k4-skeleton`, chain len 18).** No re-freeze — no golden moved.
- **`repin --check`: `pins.rs unchanged`.**
- **strict: 2904 passed / 0 failed / 4 ignored** — baseline 2903 + exactly one new test
  (`drives_order_by_map_rank`, the drive proof). The lint set fires: the negative probes
  (undeclared-island / anchor-absent / order-diverged / order-undeclared / shape-gating) all
  catch their doctored map, updated to the drive semantics.

## §5 — step-3 / step-5

- **step-3 (retrospect):** the flip is a sort-key swap (`prov` → `map_rank`) that is
  byte-identical because K1's subsequence proof + the K5 probe establish the map ranks
  strictly increase along the prov walk. The realization that made it tractable: EVERY
  byte-emitting section is labeled-and-declared, and EVERY unranked section is zero-byte — so
  the named/zero-byte merge cannot move a byte. Recorded as the §1 ground-truth table.
- **step-5 (optimize):** none — K5 is a pure authority swap by contract; no behaviour/byte
  change in scope, none made. The frozen tables remain (demoted, not deleted) by ruling; the
  `pins.rs`/`repin.toml` survive per the pins ruling; the row-34 P4c repin `.lst`-parse
  retirement stays separately tracked.

## §6 — THE K CAPSTONE IS COMPLETE

K0–K5 have all landed, fold-identical ×6 at every step:
- **K0** — the pre-K deletes (`aabb.inc`, `z80_sound_syms.asm`), byte-neutral.
- **K1** — the declared placement map + the reader + the anchors/holes/budget consumption +
  the order VALIDATION (R2 stage) + the `[map.undeclared-island]` lint.
- **K2** — boot_data ported to `.emp` (two sections) + the $3FE hole as declared-order
  contiguity.
- **K3** — the OJZ interior islands (generated `.emp` modules) + act_descriptor dissolution;
  `macros.asm`'s last consumers gone.
- **K4** — the skeleton dissolution: `games/{sonic4,demo}/main.asm` + `engine/engine.inc` +
  `macros.asm` DELETED; header.inc / sound_bank.inc native; the sound banks native (P2);
  the last hand-written AS bytes native (inc-6). NO `.asm` file emits a ROM byte or declares
  an `org`.
- **K5** — the order-DRIVE flip + the frozen-table demotion + the sigil.map.toml retirement.

The ROM placement is now, end to end, a DECLARED reviewed artifact: the per-game map owns the
section ORDER, the island anchors, the holes, the budget, and the region geometry; the frozen
tables are a measurement cache that records what the map-driven pack produced. The AS residual
is two 5-line game_root.asm stubs + the vendored debugger + emitted artifacts. The capstone's
§0 end-state — "placement authority is a DECLARED, reviewed artifact, not tables bootstrapped
from a build" — is reached.
