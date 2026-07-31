# 2026-08-01 — CONV-C PARCEL C: the ram.asm ports to `vars` (close packet — BLOCKED FINDING)

Status: **Checkpoint for the overseer's countersign.** Branch pair `conv-c-ram`
(aeon + sigil). The parcel is **re-scoped to a construct-gap finding**: the
region-form `vars` construct the census names as the port mechanism **does not
exist as a working lowering** — it parses and is then INERT by a recorded
campaign decision. No `ram.asm` file is completable via `vars` today, so the
disciplined outcome (the brief's explicit scale-warning clause) is the finding +
census/ledger annotation, **NOT a forced partial mechanism**. Zero build-input
edits; every target byte-identical to chain-9; strict unchanged at 2849/0/4.
NOT merged — the merge is the overseer's.

## §0 — THE HEADLINE

**Parcel C's port half is blocked on an unbuilt language feature and a recorded
OUT-list decision.** The census (#5/#23/#16) says "port `engine/ram.asm` +
`games/sonic4/config/ram.asm` + `games/demo/config/ram.asm` to the `vars`
construct on the computed RAM layout." Verified against current source:

1. `vars` has **two forms**. The **SST-overlay form** (`vars V: sst_custom { … }`)
   is fully built and heavily used (t29/t30/t31/t34 overlays). The **region form**
   (`vars upper_ram { Player_Pos_Ring: [u8;256] @align(256), }`) — the one ram.asm
   needs — **parses to `Item::Vars{name:None}` and then emits ZERO bytes,
   allocates NO addresses, defines NO labels, indexes NO symbols, and runs NONE of
   the spec §4.6 checks.** It is inert by design at all three lowering sites
   (`lower/mod.rs:394`, `:583`, `eval/mod.rs:661` — each comments "region form
   (`name: None`) is inert by design (Plan 7 #6 OUT-list — no region allocation)").

2. That inertness is a **recorded campaign decision, not an oversight**: region-form
   `vars` address allocation was "**Explicitly OUT of Part A** (recorded so nobody
   creeps): … map-file work, item-#7 territory — the region form stays
   parse-accepted and inert, which fails loudly downstream via unknown names"
   (`specs/2026-07-07-spec2-plan7-item6-overlay-dispatch-design.md:105`). Item #7
   (bank/window placement + map-file regions) has **no plan and no implementation**.

3. Even if item #7 were built, the port **still blocks** on the conditional-fields
   sub-gap the 2026-07-09 pre-port audit already flagged ("**The port BLOCKS on
   this**", gap-ledger:157): the mid-region `ifdef __DEBUG__` block needs
   comptime-`if`-over-fields inside `vars` — its own recorded decision.

So: **no completable subset via `vars` exists** (all three files need region
allocation; even the 9-line demo stub cannot produce its `Game_RAM_End` label
without it). Building the feature is a capstone-scale, multi-decision language
build that governance deliberately deferred — a milestone-boundary call for the
overseer, not a porter-session flip. This packet delivers the rigorous finding,
a decision-ready scope estimate, and the census/ledger reconciliation.

## §1 — PREMISE CORRECTIONS (census vs reality)

- **Census: "PORT (`vars` layout); the `vars` construct."** WRONG as a mechanical
  premise. The `vars` construct the census points at (region form) is **inert** —
  a parse-only stub on the item-#7 OUT-list. The census inherited the assumption
  from the 2026-07-09 pre-port audit, which was a **spec-surface audit** ("the
  frozen §4.6 `vars` surface already covers the core") and did **not** verify the
  implementation. Implementation reality (this packet, three lowering sites): the
  surface exists; the lowering does not.
- **Census: "files: 5, 23, 16; effort L; class BN (fold-identity)."** The
  fold-identity BAR is right (RAM emits no bytes → the proof is address-exactness,
  transitively pinned by ROM operands, both shapes). But **effort L understates
  it**: this is not a port at effort L, it is a **language-feature build** (item #7
  region allocation + conditional `vars` fields + cross-region base chaining), each
  a recorded-decision gate. Capstone-class, not L.
- **Census Parcel-C framing conflated two halves.** Parcel C = (a) "build B-0b RAM
  packing FIRST" + (b) "then port the three ram.asm files." Half (a) is **already
  DONE** — the 2026-08-01 B-0b note proved RAM's B-0 analog is AS `phase`-from-symbol
  (already live, growth flows automatically), and committed the
  `ram_packing_invariants_{plain,debug}` guards. Half (b) is what this packet finds
  blocked. The two are not the same information: (a) needed no `vars`; (b) needs the
  whole unbuilt region-`vars` feature.
- **B-0b premise re-confirmed against current source:** engine RAM = two AS `phase`
  blocks (`$FFFF0000` lower `.l`, `$FFFF8000` upper `.w`) → `Engine_RAM_End`; game
  RAM = `phase Engine_RAM_End` (sonic4: player/phys/history-rings; demo: empty
  stub). All even-hand-padded (AS does not auto-align `ds.w/.l`; the address-error
  trap). `pins.rs` RAM cells are repin-generated TEST snapshots, never build inputs.
  Unchanged since the B-0b note (master last touched this at ce03c7f-era; the tree
  here builds the chain-9 tips — §3).

## §2 — THE CONSTRUCT GAP, IN FULL (the step-3 deliverable)

### 2a. What `vars` region form does today (verified)
- **Parse:** `parser.rs:422` → `vars_decl` → `Item::Vars{ name:None, region:["upper_ram"],
  fields:[{name, ty, align}] }`. Test `vars_region_and_overlay_forms`
  (`parser_decls.rs:227`) pins the shape. So the *grammar* is live.
- **Lower (top-level, `lower/mod.rs:394`):** `if let Some(name) = &decl.name` — only
  the NAMED overlay form validates+lays out; the region form (`None`) falls through
  to nothing. Comment: "Region form (`name: None`) is inert by design (Plan 7 #6
  OUT-list)."
- **Lower (in-section, `lower/mod.rs:583`):** identical — "region form → inert."
- **Index (`eval/mod.rs:661`):** "Only the NAMED overlay form … is indexed; the
  region form … is inert by design (Plan 7 #6 OUT-list — no region allocation)."

Net: a region-form `vars` block is a **no-op**. None of the spec §4.6 promises
("Region overflow, the `.w`-addressability bit-15 rule, and align-under-vma
correctness are compiler checks (replacing the hand guards in `ram.asm`)") are
implemented — those are **aspirational spec text describing item #7**, not shipped
behavior.

### 2b. What building it (item #7 + the RAM story) actually requires
To author `engine/ram.asm` in `.emp` at address-exact parity, a porter would need
ALL of the following, each currently absent:

1. **Region base authority (the "map file").** The region names (`lower_ram`,
   `upper_ram`, and the game continuation) must resolve to base VMAs
   (`$FFFF0000` / `$FFFF8000` / `Engine_RAM_End`). No RAM map file exists; the
   phase addresses live only in `ram.asm` today. Item #7 is described precisely as
   "map-file work."
2. **Region-allocation lowering.** Create a Core `Section` at the region base
   (classified RAM by `vma_base >= $F00000`, so `is_rom_section=false` → appended,
   never baked — the mechanism the B-0b note documents already exists on the ROM
   side); lay out each field advancing a location counter (`[u8;N]`→N bytes,
   `[u16;N]`→2N, `[u32;N]`→4N — the `ds.b/ds.w/ds.l` analog), emit a `pub`-exportable
   label at each field's running address so `.asm`/`.emp` consumers resolve it.
3. **`@align(N)` under a VMA base.** Field-level alignment that pads within the RAM
   section (kills the `align 256` guard + the ~20 hand `ds.b 1` even-pads). Must be
   RESERVE semantics (advance counter, emit no ROM bytes) — distinct from the
   shipped item-position `align N` which emits `$00` fill.
4. **Conditional fields (`ifdef __DEBUG__` mid-region).** comptime-`if`-over-fields
   in `vars`, driven by the existing `-D __DEBUG__` define, so DEBUG and release
   diverge downstream address-exactly. **Recorded as a hard blocker** (gap-ledger:153,
   "The port BLOCKS on this"), needs its own decision (or a pre-port `.asm` move of
   the debug block to the region tail). The engine RAM has one such block
   (`Prof_*`/DMA debug counters, `ram.asm:192`); the sonic4 game RAM has another
   (`Dbg_Music_On`/`Dbg_Sfx_Sel`, `config/ram.asm:7`).
5. **Cross-region base chaining.** `games/*/config/ram.asm` uses `phase
   Engine_RAM_End` — the game region's base IS the end of the engine region. The
   region-form `vars` grammar (region name + map-file base) has **no spelling** for
   "base = another region's end." A new surface (or a map-file "chain" relation) is
   required, plus the ordering guarantee (game RAM lowered after engine RAM).
6. **Buffer-reuse overlay.** `Art_Staging_Buffer = Tile_Cache_Nametable` + size
   `if` + lifetime comment. The audit rules this NOT a hard blocker (expressible as
   `pub equ` alias + `ensure(size fits)`), but a faithful port wants the declared
   region-level overlap (§4.6 SST overlays are struct-`[u8;N]`-window only today).
7. **The reserved compiler checks.** The three overflow `if/error`s
   (`Lower_RAM_End > $FFFF8000`, `Engine_RAM_End >= SYSTEM_STACK`, `Game_RAM_End >=
   SYSTEM_STACK`), the `Object_RAM .w` bit-15 reachability check, and the
   `Player_Pos_Ring & $FF` align check must re-home to region budgets / `ensure`s.
8. **Debug-layout-stability lint** (gap-ledger:165) so a future conditional field
   not at a region tail is caught before it silently shifts the other shape.
9. **The repin + port-gate ripple.** Once RAM is `.emp`-authored, `pins.rs` snapshot
   regeneration and every port gate that reads a RAM symbol (`sprites_port`,
   `core_port`, `camera_port`, `section_port`, `tile_cache_port`,
   `entity_window_port`, `dma_queue_port`, …) must re-verify address-exactly, both
   shapes.

Items 1–5 and 7 are load-bearing prerequisites; NONE ship today. This is the
"blocked on a missing `vars` capability" case the brief names — reported before
any hack, per the brief's mandate.

## §3 — THE BYTE-IDENTITY PROOF (all six vs chain-9)

Zero build-input edits were made (docs only). The proof is therefore that the
tree assembles the chain-9 tips unchanged — verified by a fresh plain + debug
build:

| target | built (this packet) | chain-9 tip | match |
|---|---|---|---|
| s4          | `6cf74e65` / 412127 | `6cf74e65` / 412127 | ✓ (direct) |
| s4.debug    | `16615e46` / 421958 | `16615e46` / 421958 | ✓ (direct) |
| demo        | `9bb8c993` / 90506  | `9bb8c993` / 90506  | ✓ (direct) |
| demo.debug  | `bc7678d0` / 93006  | `bc7678d0` / 93006  | ✓ (direct) |
| config_a    | `78df5e6a` / 422297 | `78df5e6a` / 422297 | ✓ (strict-suite byte-identity) |
| config_b    | `f38f609b` / 303501 | `f38f609b` / 303501 | ✓ (strict-suite byte-identity) |

s4/s4.debug/demo/demo.debug verified directly (one shape per invocation; `DEBUG=1`
writes the separate `*.debug.bin` — the stale-artifact trap was avoided).
config_a/config_b via the strict suite's native-chained byte-identity gates.

## §4 — THE RAM-LAYOUT IDENTITY PROOF

No `ram.asm` (engine or game) was touched; no `.emp` RAM authorship was added; no
`pins.rs` was regenerated. The RAM layout is therefore **identical by
construction** — every RAM symbol resolves to its chain-9 address. Positively
confirmed by:
- `ram_packing_invariants_plain` / `ram_packing_invariants_debug` — **both ok** (the
  even-base, no-ROM-section-in-RAM, and engine→game contiguity invariants hold).
- The full ROM byte-identity (§3): RAM addresses flow into ROM operands as
  immediates, so an unchanged ROM CRC across all six targets is the transitive
  witness that no RAM symbol moved (the B-0b note's fold-identity contrapositive —
  a moved RAM address DOES diverge the ROM gates).
- `git status` on both trees shows only doc files changed (§7 sweep).

## §5 — FLIPPED / REMAINING CENSUS PER FILE

| # | file | flipped? | reason |
|---|---|---|---|
| 5  | `engine/ram.asm` (524 ln) | **NO — parked** | needs region-`vars` allocation (item #7) + conditional-fields decision + reserve-`@align` + the reserved compiler checks. All unbuilt. |
| 23 | `games/sonic4/config/ram.asm` (60 ln) | **NO — parked** | needs all of #5's prerequisites PLUS cross-region base chaining (`phase Engine_RAM_End`) + its own `ifdef __DEBUG__` block (`Dbg_Music_On`/`Dbg_Sfx_Sel`). |
| 16 | `games/demo/config/ram.asm` (9 ln) | **NO — parked** | the 9-line empty stub STILL needs region allocation + cross-region chaining to produce its `Game_RAM_End` label; a region-form `vars` emits nothing. No trivially-completable subset. |

**Completed this parcel:** the finding, the scope estimate, the census annotation,
the gap-ledger correction. **Half (a) of census Parcel C (B-0b) was already done**
(2026-08-01 B-0b note; guards committed) — this packet confirms it still holds
(`ram_packing_invariants` green).

## §6 — RETIRED-TEST ENUMERATION

**None.** No code changed; strict stays at **2849 passed / 0 failed / 4 ignored**
(re-run this packet, exit 0, no failures; `ram_packing_invariants_{plain,debug}`
ok). No walls retired, no re-homes needed.

## §7 — STEP-3 (retrospect) vs STEP-5 (engine optimization) FINDINGS

- **Step-3 (THE headline, feeds the language-ask round):** the region-form `vars`
  construct — the designed answer to `ram.asm`'s shape (§4.6) — is **spec-complete
  but implementation-absent** (inert, item-#7 OUT-list). The 2026-07-09 pre-port
  audit's optimistic "the surface already covers the core" was a surface read; the
  implementation gap (this packet) is the correction. Building it is a bounded but
  capstone-scale feature: §2b enumerates the 9 required pieces (region map-file base
  authority, region-allocation lowering, reserve-`@align`, conditional `vars`
  fields, cross-region base chaining, buffer-reuse overlay, the reserved compiler
  checks, the layout-stability lint, the repin/port-gate ripple). Recommend the
  overseer route this as **its own item-#7 parcel with recorded decisions**, or
  accept AS-authored RAM as the standing mechanism (the B-0b note already
  established AS `phase`-from-symbol IS the working RAM analog of B-0 — nothing is
  broken or blocked by leaving RAM in AS).
- **Step-5:** none. No lowering changed, no bytes moved.

## §8 — NEITHER-BUCKET HEADLINES

- **The demo config stub (#16) is the cleanest proof that there is no easy subset.**
  9 lines, empty region, yet un-portable: a region-`vars` produces no section and no
  `Game_RAM_End` symbol, so even "convert the trivial file first" fails. The whole
  parcel is gated on one feature.
- **The census's own OUT-vs-blocked confusion is worth flagging upstream.** Parcel C
  was written as "build B-0b, then port ram.asm," but B-0b (RAM packing) needs NO
  `vars` and the ram.asm port needs the ENTIRE unbuilt region-`vars` feature — they
  share only the RAM domain, not the mechanism. The active-arc synergy the census
  claimed (entity_window/tile_cache "need this parcel") is satisfied by B-0b alone,
  already delivered; those two increments grow RAM by editing `ram.asm` directly
  (B-0b note §6), needing nothing from the ram.asm port.
- **`SYSTEM_STACK` is the one AS `=` in `engine/ram.asm`'s neighborhood** used by the
  overflow guards; it already flipped to `engine.constants` (conv-B). So the guards'
  re-home target (an `.emp` `ensure` reading `SYSTEM_STACK`) is ready — the block on
  the port is purely the region-allocation feature, not its constants.

## §9 — GAP-LEDGER SWEEP + KILL-LIST

- **Gap-ledger corrected (this packet):** the "vars / RAM regions (ram.asm pre-port
  audit, 2026-07-09)" section header now carries an implementation-status correction
  — the region-form `vars` allocation the audit assumed present is **inert /
  item-#7-unbuilt**; the conditional-fields row remains OPEN and is the *second*
  blocker behind the allocation feature itself. No new nice-to-have was implemented
  (nothing was portable), so no new "jotted" rows beyond the correction.
- **Kill-list:** **no closures.** Every `vars`/RAM row in
  `twin-scaffolding-kill-list.md` (7, 61, 66, 68, 74, 84, 85) is an **SST-overlay**
  row (struct-flip / Spec-5 territory), keyed to gated object twins — none is keyed
  to the region-form RAM authoring this parcel targets. The ram.asm port has no
  scaffolding-kill row yet (it would be born WITH the item-#7 feature).

## §10 — RECOMMENDATION (overseer decision)

The brief's scale-warning clause is squarely triggered: the flip is blocked on a
missing `vars` capability that a recorded decision deferred. Three honest paths,
for the overseer/Volence to rule at this checkpoint:

1. **Build item #7 as its own parcel** (region map-file allocation + conditional
   `vars` fields + cross-region chaining), then port the three files under it. The
   correct-and-complete path to "everything converted," but capstone-scale and
   decision-gated (§2b). Recommend a dedicated spec + plan first.
2. **Accept AS-authored RAM as the standing mechanism.** The B-0b note established
   AS `phase`-from-symbol IS the working RAM analog of B-0; nothing is broken. RAM
   would be the honest exception to "100% `.emp`" (like the vendored debugger),
   documented as such. Cheapest; defers item #7 indefinitely.
3. **Partial pre-port hygiene now, feature later.** Move both `ifdef __DEBUG__`
   blocks to their region tails so a future item-#7 port is unconditional-only
   (removing the conditional-fields blocker ahead of time). **NOT byte-neutral:**
   plain-shape is unaffected, but moving a mid-region debug block to the tail shifts
   every DEBUG-shape field that currently follows it (Object_RAM onward) back to its
   release address → the debug-shape RAM addresses change → `s4.debug`/`demo.debug`
   CRCs move → it needs a **DEBUG re-freeze** (the plain goldens hold). De-risks path
   1 but is a byte-CHANGING debug edit. (NOT done here — a re-freezing `.asm` change
   must ride its own gate + the overseer's nod, not a docs-only finding packet.)

This packet takes **none** of the three unilaterally — item #7 is OUT-listed by
governance and the choice is a milestone-boundary call. It delivers the finding,
the scope, and the reconciliation so the call can be made with numbers.
