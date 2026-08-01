# Item #7 — RAM regions: the `vars` region form (design)

**Status: ✅ SHIPPED (2026-08-01).** All three parcels done: #7a (the feature),
#7b (`engine/ram.emp`), #7c (`games/{sonic4,demo}/config/ram.emp` + cross-module
`after(..)` + the per-game RAM harvest). All six targets byte-identical to
chain-9; repin RAM-cell zero-diff both shapes. One spec refinement landed en
route: region `@align(N)` uses AS's IN-PHASE align semantics `round_up(cursor +
n, n)` (§2.3's "next multiple of N" is imprecise for the corpus — regions are
RAM-only, so the asl 1.42 phased-align quirk always applies; it is what places
`Player_Pos_Ring` at `$FFFFB500`). Notes: `2026-08-01-item7b-engine-ram-port.md`,
`2026-08-01-item7c-game-ram-ports.md`.

**Original status: RATIFIED design, ready for implementation.** Volence ruling 2026-07-31
on the conv-C blocked finding: *"Let's do it properly, there's no rush right now
(as much as I'd like to start features, let's do it right)."* Path 1 of the
conv-C §10 decision — build the feature as its own spec + plan + parcels, then
port `ram.asm` under it. **This ruling supersedes the Plan-7 item-6 OUT-list for
this item** (the OUT was a scope fence, not a design rejection; the fence did
its job — nobody crept, and the feature now arrives whole).

Spec owner: Fable (overseer). Implementers: Opus porters, one parcel per §6.
Inputs: SPEC2_LANGUAGE §4.6 (the frozen aspirational surface), the conv-C close
packet §2b (the 9-piece scope), `engine/ram.asm` + `games/{sonic4,demo}/config/ram.asm`
(the port targets), the B-0b RAM-packing note (the layout contract + invariant
guards), gap-ledger rows 153/157/165 (conditional fields + the stability lint).

## §1 — What this is

RAM in aeon is authored today in three `.asm` files as `phase`-block `ds.*`
runs. Item #7 makes the `.emp` `vars` **region form** real: regions declare
their address windows, `vars` blocks allocate variables inside them at exact,
deterministic addresses, and the hand-maintained guards (`if/error` overflow
checks, bit-15 comments, `ds.b 1` even-pads) become compiler checks and lints.
The bar for the eventual ports is **address-exactness**: every RAM symbol at the
same address as the `.asm` authored it, both shapes, proven by repin-snapshot
identity + six-target ROM byte-identity (RAM addresses flow into ROM operands —
the fold-identity contrapositive).

## §2 — The surface

### 2.1 `region` — a module item (v1: source-hosted map-file content)

```
pub region lower_ram  @ $FFFF0000 .. $FFFF8000
pub region upper_ram  @ $FFFF8000 .. SYSTEM_STACK, w_addressable
pub region game_ram   @ after(upper_ram) .. SYSTEM_STACK, w_addressable
```

- `@ base .. limit` — base VMA and exclusive limit. Comptime expressions
  (consts resolve; `SYSTEM_STACK` is an ordinary const).
- `after(<region>)` — the base is the running end of another region (the
  `phase Engine_RAM_End` idiom). Chaining forms a DAG; a cycle is
  `[region.chain-cycle]`. A chained region lays out after its parent
  completes (§3.2).
- `w_addressable` — asserts every byte in `[base, limit)` is reachable by
  sign-extended `.w` addressing (bit 15 of the low word set across the whole
  window, i.e. `base & $FFFF >= $8000` and `limit - 1` still in range).
  Checked once at the region — subsumes the per-symbol
  `Object_RAM & $FFFF < $8000` hand guard, and is strictly stronger (covers
  every symbol in the region, not one).
- A region is declared **exactly once** per link (`[region.duplicate]`);
  `vars` naming an undeclared region is `[region.unknown]` (today's
  fail-loudly-downstream becomes a first-class diagnostic).
- **Deviation from §4.6 recorded:** the spec says regions "live in the map
  file." No physical map file exists yet — ROM placement authority is the B-0
  harness tables, and inventing a RAM-only map format would create a second
  authority. v1 hosts `region` items in `.emp` source (they ARE map-file
  content); when the residual-split capstone materializes the real map file,
  `region` items lift there mechanically. SPEC2 §4.6 gets amended with this
  note when the feature ships.

### 2.2 `vars` region blocks — allocation

```
vars upper_ram {
    VBlank_Flag:     u8,
    pad(1),
    Frame_Counter:   u16,
    Game_State:      u32,
    mark DMA_Queue,
    DMA_Critical:    [u8; DMA_CRITICAL_SLOTS * sizeof(DMAEntry)],
    mark DMA_Critical_End,
    Pos_table:       [u8; 256] @align(256),
    Art_Staging_Buffer: alias(Tile_Cache_Nametable),
    if DEBUG == 1 @shape_divergent {
        Lag_Frame_Count: u32,
        Debug_Scene_Freeze: u8,
        pad(1),
    }
    mark Engine_RAM_End,
}
```

Field kinds, in declaration order at a running location counter from the
region base:

- **Typed fields** — `name: T` where `T` is a primitive, `[T; N]` (N any
  comptime expression), or a struct type (`[Sst; NUM_DYNAMIC]` allocates
  `N * sizeof(Sst)`; the struct types are already sole-authored in `.emp`
  post conv-A). Each field defines a link-visible label at its address;
  `pub vars` exports them (residual `.asm` consumers resolve them through the
  existing cross-seam link path — same mechanism as every ported label today).
- **`pad(N)`** — anonymous reserve of N bytes (the `ds.b 1` even-pad idiom,
  intent-named, no label pollution).
- **`@align(N)`** — on a field: advance the counter per AS's IN-PHASE align
  semantics, `cursor = round_up(cursor + N, N)` (the asl 1.42 regime the whole
  corpus was authored under — byte-identity with the retired ram.asm REQUIRES
  it; AMENDED at the #7c countersign, was imprecisely "next multiple of N").
  RESERVE semantics — no bytes emitted anywhere; this is RAM. Distinct from the
  shipped ROM item `align N` (which emits fill). A true next-multiple variant
  is gap-ledgered for green-field demand; one spelling until then.
- **`mark Name`** — a zero-size label at the current counter (`DMA_Queue`,
  `Object_RAM`, `Parallax_State_End`, `Engine_RAM_End`, `RAM_Start`,
  `Lower_RAM_End` — ram.asm's marker-label idiom, ported verbatim).
- **`name: alias(Other)`** — a label equal to another field's address (the
  `Art_Staging_Buffer = Tile_Cache_Nametable` buffer-reuse idiom). Pure
  equate: allocates nothing. Guards ride ordinary `ensure` (e.g.
  `ensure(ART_STAGING_BUFFER_SIZE <= TILE_CACHE_NT_SIZE, "...")` replaces the
  `if/error`). The §4.6 "declared region-level overlap" aspiration stays OUT
  (v2 candidate); alias+ensure is the audit-accepted faithful form.
- **`if <comptime-cond> { fields... }`** — conditional field groups driven by
  the existing define environment. The condition is an arbitrary comptime
  expression; the corpus convention is `-D DEBUG=0|1` + `if DEBUG == 1`
  (RULED at #7a countersign — one conditional idiom, no `defined()`
  presence-semantics construct; an undefined bareword errors loudly). Shapes may diverge downstream
  of a size-varying group — that is INTENDED for the two existing debug blocks.
  A size-varying conditional group must carry **`@shape_divergent`** or it is
  the `[vars.shape-divergent]` ERROR: the annotation is the author declaring
  "yes, everything after this moves between shapes" (gap-ledger 165's
  stability lint, made an opt-in declaration instead of a warning — accidents
  are impossible, intent is visible at the divergence point). A group whose
  arms are size-equal (e.g. `Dynamic_Live_Walking: u8` vs `pad(1)`) needs no
  annotation — shape-INVARIANT conditionals are the preferred form and the
  compiler proves the invariance.

### 2.3 Layout rules

- **No auto-alignment, ever** (the language's §4.3 tenet; also AS's actual
  behavior — the source of the address-error trap). The **`[layout.odd-field]`
  lint** fires on any u16/u32/i16/i32/struct/word-array field placed at an odd
  address: the silent-crash class becomes a diagnostic. Fix is an explicit
  `pad(1)` or `@align(2)` — the layout stays author-owned and byte-exact.
- **Single owner module per region (v1).** All `vars` blocks for a region must
  live in ONE module (`[region.multiple-owners]` otherwise); multiple blocks in
  that module allocate in source order. This makes layout deterministic without
  a cross-module ordering story (which is exactly the trap: link-order-dependent
  RAM layout). Cross-module contribution is v2, gated on the map-file ordering
  manifest (§3.3's declared-order end-state). The port needs single-owner only —
  ram.asm is one file per region set.
- **Region overflow** — counter past `limit` is the `[region.overflow]` error
  "over by N bytes," naming the region and the field that crossed (replaces the
  three `if/error` guards; the chained region checks against ITS limit, which
  covers `Game_RAM_End >= SYSTEM_STACK`).

### 2.4 Emission and integration

- A `vars` region block lowers to a **reserve-only Core section**: VMA-placed at
  the region counter, contributes **zero image bytes** (RAM classification —
  `vma_base >= $F00000`, `is_rom_section=false`, the appended-never-baked path
  that already exists). Labels index like any `.emp` label; `pins.rs` RAM cells
  remain repin-generated TEST SNAPSHOTS of these addresses (B-0b's finding is
  unchanged — snapshots, never build inputs).
- The §4.6 sentence "Region overflow, the `.w`-addressability bit-15 rule, and
  align-under-vma correctness are compiler checks" becomes TRUE with:
  `[region.overflow]`, `w_addressable`, and `@align` resolved against the real
  VMA (`[layout.odd-field]` keys on region-absolute addresses).

## §3 — Semantics details

1. **Determinism:** layout is a pure function of (region decls, owner-module
   source order, comptime env). No link-order input anywhere.
2. **Chained ordering:** a region lays out when its base is known; `after(R)`
   waits for R's owner module. The engine's regions resolve first, then each
   game's `game_ram` (the game module chain-depends on the engine module —
   already a legal symbol dependency).
3. **Cross-seam exports (AMENDED at the #7b stop — the original "no harvest
   needed" claim was FALSE for eager references):** `pub vars` fields and
   `mark`s link-export for DEFERRED AS references (`jsr`/`jmp`, `dc.l`/`dc.w`,
   `lea (Sym).w`). But the AS frontend folds absolute-EA DATA operands
   (`move.b #x,(Sym).w`) and `phase <Sym>` displacements EAGERLY — those need
   values at assemble time. RULED (Option B): a third harvester,
   `harvest_engine_ram_addresses`, lays out the regions and seeds every RAM
   label as a plain value DEFINE on the AS side (values only, NOT exported
   EquSyms — the `.emp` labels stay the sole link authority, no duplicate
   symbols). HARD REQUIREMENT: the harvester and the lowering must share ONE
   layout entry point (`lower/regions.rs`) — two authorities for the same
   address must be structurally impossible, not merely tested. Deferred and
   eager paths then agree by construction.
4. **Z80 regions are OUT of v1** (no demand: Z80 RAM is driver-internal and
   already sigil-native). `u16le` fields are legal (data-contract cells).
5. **comptime queries:** `addressof(Field)` is the label (link-time as usual);
   `Region.end`-style comptime reflection is OUT of v1 — `mark` labels serve
   the two real uses (`Engine_RAM_End`, `Game_RAM_End`) exactly as `.asm` did.

## §4 — The faithful-port mapping (every ram.asm construct → its spelling)

| ram.asm construct | .emp spelling |
|---|---|
| `phase $FFFF0000` … `dephase` | `region lower_ram @ $FFFF0000 .. $FFFF8000` + one `vars lower_ram {}` |
| `Name: ds.b N` / `ds.w N` / `ds.l N` | `Name: [u8; N]` / `[u16; N]` / `[u32; N]` (scalars for N=1) |
| `ds.b SST_len * NUM` | `[Sst; NUM]` (typed) or `[u8; sizeof(Sst)*NUM]` (verbatim) — porter's per-site call, byte-equal either way |
| anonymous `ds.b 1` pad | `pad(1)` |
| `Name:` (marker label) | `mark Name` |
| `Name = Other` (alias) | `Name: alias(Other)` |
| `if COND > LIMIT … error` | region `limit` / `ensure(...)` |
| `ifdef __DEBUG__ … endif` (size-varying) | `if DEBUG == 1 @shape_divergent { … }` |
| `ifdef __DEBUG__ X else pad endif` (size-equal) | `if DEBUG == 1 { X: u8 } else { pad(1) }` — compiler proves invariance |
| `(Object_RAM & $FFFF) < $8000` guard | region `w_addressable` |
| `align 256` (game ram) | `@align(256)` |

## §5 — Diagnostics (new)

`[region.duplicate]` · `[region.unknown]` · `[region.chain-cycle]` ·
`[region.multiple-owners]` · `[region.overflow]` (with over-by and the crossing
field) · `[region.not-w-addressable]` · `[vars.shape-divergent]` (size-varying
conditional group without the annotation) · `[layout.odd-field]` (lint, region-
absolute). Region form stops being inert the moment #7a lands — the three
"inert by design" comments come OUT.

## §6 — Implementation plan (three parcels, sequential)

- **#7a — the feature** (sigil only): grammar already parses the region-form
  `vars`; add the `region` item, the allocation lowering (§2.2–§2.4), checks,
  lints, diagnostics. Proof: synthetic fixtures (a fixture region set exercising
  every §4 row + every diagnostic), strict suite green, all six targets
  byte-identical (feature unused by aeon yet = zero placement effect).
- **#7b — the engine port**: `engine/ram.asm` → `engine/ram.emp` (or a `vars`
  section of an existing module — porter's call, single owner). Bar:
  repin-snapshot identity (every RAM cell same address, both shapes),
  `ram_packing_invariants_{plain,debug}` green, six-target ROM byte-identity,
  strict green minus consciously-retired walls (§4 template). engine.inc drops
  the include.
- **#7c — the game ports + census close**: `games/sonic4/config/ram.asm` +
  `games/demo/config/ram.asm` via `region game_ram @ after(upper_ram)`; same
  bar; conv-C census rows 5/23/16 flip to DONE; kill-list sweep; the item-6
  OUT-list comments and gap-ledger 153/157/165 rows close.

## §7 — Decision record (rulings made in this spec)

1. Regions are source-hosted `.emp` items in v1 (map-file content, migration
   path recorded) — NOT a new file format.
2. Single-owner-module per region in v1; cross-module contribution deferred to
   the map-file ordering manifest.
3. Conditional fields land WITH the feature (no separate decision gate);
   `@shape_divergent` is a required declaration on size-varying groups, error
   otherwise; size-equal groups are proven invariant by the compiler.
4. Buffer overlap stays alias+ensure in v1; declared region-level overlap is a
   v2 candidate (ledgered).
5. `w_addressable` is a region attribute subsuming the per-symbol bit-15 guard.
6. No auto-alignment anywhere; `[layout.odd-field]` turns the address-error
   class into a diagnostic; pads are explicit and intent-named.
7. Z80 RAM regions and comptime region reflection are OUT of v1.

## §8 — #7a countersign addenda (rulings on the implementation stops)

1. **Condition spelling RULED:** `if DEBUG == 1`, the shipped corpus convention
   (`-D DEBUG=0|1`); the §2.2/§4 examples above are amended. No `defined()`
   construct — presence semantics is an AS-ism; the define environment is
   explicit and an undefined bareword condition errors loudly.
2. Cross-module `after()` resolution is #7c scope (accepted staging; the
   intra-file DAG + cycle detection shipped in #7a; `[region.multiple-owners]`
   is whole-program).
3. `place_sections` RAM-skip (`vma_base >= $F00000`) is #7b's first task —
   #7a left placement untouched for zero CRC risk (accepted).
4. `[layout.odd-field]` is a warning — that is what "lint" means here (accepted).

## §9 — #7b countersign addendum (the eager-reference stop, ruled)

The #7b porter empirically confirmed §3.3's export claim false for eager AS
references and stopped per brief. RULED: **Option B** (the RAM-address harvest
as value-defines; §3.3 amended above). Option A (teaching the AS frontend to
defer absolute-EA operands) was rejected: it touches the hot conversion path
every residual line rides, and cannot fix `phase` eagerness anyway. Gap-3 (the
four game-config size constants missing from `emp_defines`) is mechanical and
in #7b scope.

## §10 — #7c countersign addenda (arc close)

1. **`@align` semantics AMENDED** (§2.3 above): the AS in-phase regime is the
   shipped meaning — the corpus is the contract. True-multiple variant
   gap-ledgered, not built (no demand).
2. `w_addressable` on `game_ram` KEPT — a strictly-stronger invariant than the
   retired guard set; it holds and hardens.
3. The `emit_rom` reserve-section skip (a latent contract violation in
   sigil-link's flatten path) ACCEPTED — byte-neutral, six-golden-proven.
4. **ITEM #7 IS SHIPPED WHOLE**: feature (#7a) + engine port (#7b) + game ports
   and cross-module chaining (#7c). All RAM in the aeon tree is .emp-authored;
   conv-C census rows 5/23/16 DONE; the Plan-7 item-6 OUT-list fence is fully
   retired for this item.
