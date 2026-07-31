# §17 Optimization Sweep — design note (design-gate deliverable)

**Branch:** `opt-sweep` (off sigil master `1b70d8e`). Aeon read-only at `d9d2385`.
**Provenance baseline (this arc's zero point):** s4 `7f071417`/412306 · s4d `0b8efc7a`/422147 ·
demo `705a5871`/90436 · demod `37ded207`/92935 · ca `1b4c49d2`/422483 · cb `bfe2509e`/303660,
six assembled anchors as primary provenance. Strict 2856/0 (1 ignored).

This is a DESIGN-ONLY note. Nothing here changes bytes. It defines the protocol the sweep runs
under, then STOPS for the overseer's countersign.

---

## 0. The world this arc is born into (why the protocol must change)

The flip is **DONE** — Stage 2 executed. `build.sh` IS `sigil build` (asl/p2bin/fixheader have
left the pipeline; `aeon/build.sh:4`), and the .asm twins of the named §17 engine files are
**retired**: `tile_cache.emp`, `plane_buffer.emp`, `sprites.emp`, `rings.emp`, `animate.emp`,
`entity_window.emp`, `core.emp`, `parallax.emp`, `section.emp`, and game-side
`player_ground.emp` are now **.emp-ONLY** (only 6 .asm/.emp pairs survive corpus-wide, none of
them a §17 target). Two consequences dominate the whole design:

1. **The parcel-scope amendment's "~half cost" is now literal, not aspirational.** Each item pays
   NO twin lockstep and NO `.asm` re-emit — one source file per edit. The remaining ceremony is
   the re-pin + golden re-freeze (below), which is mechanizable.

2. **THE ORACLE-MODEL SHIFT is now forced by the toolchain, not merely stated.** The frozen
   goldens are **sigil's own frozen output** — sigil builds the ROM, then compares it against a
   committed copy of that same build (`native_full_rom.rs` full-file CRC pin;
   `native_offcanonical_rom.rs` anchor byte-compare vs `golden/*.bin`). For UNCHANGED code this is
   a perfect regression oracle (any accidental drift trips it). For a **deliberately** byte-changing
   optimization it proves nothing about correctness — you are changing the very bytes it pins.
   **Correctness moves to the emulator A/B (the oracle); the golden is re-frozen afterward as the
   new regression baseline.** This note's central job is getting that hand-off right.

The genuinely *frozen* historical witness — the **asl-witnessed** CRCs recorded in
`crates/sigil-harness/golden/PROVENANCE.md` (assembled anchors `e5765873`/`dab4f06c` for the
canonical pair) — is the LAST asl-provable identity of this codebase. The first byte-changing
optimization parcel **permanently severs** the shipped ROM from that witness. That severance is
the arc's defining event and the reason the provenance invariant must be rewritten (§1).

---

## 1. The re-freeze protocol & provenance discipline

### 1.1 The new provenance invariant (replaces "anchors never move")

OLD invariant (conversion era): *the six assembled anchors are byte-frozen; any drift is a bug.*

NEW invariant (optimization era):

> **An anchor moves ONLY inside a named optimization commit that carries frame-anchored emulator
> A/B evidence in its packet. Every such commit appends one link to the provenance chain in
> `PROVENANCE.md`: `<parent-anchor> --[<parcel>, A/B ref]--> <new-anchor>`. The asl-witnessed CRCs
> stay recorded as the historical root of that chain, never overwritten.**

So provenance becomes a *chain*, not a *point*: `asl-witness (frozen root) → [opt commit 1, A/B] →
[opt commit 2, A/B] → … → current sigil-canonical golden`. A reviewer can walk any current ROM
back to the asl root through a sequence of A/B-evidenced steps. An anchor that moved WITHOUT a
paired A/B ref in the same commit is the new "gate red" — the tripwire the design must make loud.

### 1.2 Per-parcel, not per-batch, re-freeze

**Ruling: re-freeze per parcel.** Each byte-changing parcel is its own bisectable commit with its
own A/B evidence and its own golden re-freeze, so the provenance chain has one link per behavioral
decision. Per-batch re-freeze would collapse several byte-changing decisions behind one anchor
move, destroying bisectability and letting a bad item hide behind a good one's A/B. Parcels may be
*grouped* into a merge wave (§5), but each carries its own re-freeze commit inside the wave.

### 1.3 The A/B evidence bar, by item class

Every byte-changing parcel names its observations **frame-anchored to `Frame_Counter`, never
press-count** (standing oracle rule), on a **deterministic-from-reset** scene. The bar scales with
what the byte gate is blind to:

| Class | Definition | A/B bar |
|---|---|---|
| **PS — pure-size / value-identical** | restructure proven to emit the same *observable* result by construction (segment copy of the same words; move.l pairing of word-granular copies; tail-call `jbra`; loop-invariant hoist) | (a) determinism: two native builds byte-identical (already gated); (b) **state-identity A/B**: at N anchor frames, the affected RAM/VRAM/CRAM region is byte-identical OLD vs NEW via the hash/screenshot pipeline (§3) — proves the restructure didn't perturb output. Screenshot `cmp` for visible plane; region hash for off-screen. Frames: ≥3 anchor frames spanning the exercised path (e.g. a scroll-crossing frame for tile_cache). |
| **BA — behavior-adjacent / hazard-fix** | changes a value the byte gate can't see is wrong (G9 high-word clear; a dropped/added guard; an inherited-flag change) | PS bar PLUS a **named positive observation of the fixed effect** at the exact frame it manifests (the t24 pattern: "`render_flags` `$00→$08` the instant the creator returns"). For G9: drive a scene where `d7`'s high word is dirty at `Ground_Move_Cap` entry and show OLD mis-decodes / NEW decodes correctly — plus the benign-under-current-dispatch confirmation that shipped scenes are unchanged. |
| **PF — perf-affecting** | intended to move cycles/VBlank budget | PS bar PLUS a **live-profiler A/B on an UNFROZEN drive** (frozen scenes under-load — the Probe-A caveat: `Debug_Scene_Freeze` skips `EntityWindow_Scan`, so lag can't appear). Report before/after self-time on the hot proc AND the frame-lag counter (`Lag_Frame_Count`) on a real max-H / max-V drive. The threshold rule (parcel-scope amendment) applies: cut ≥~1k cyc/f steady-state, log-and-skip below with numbers. VBlank-DMA-bandwidth items (sprites H3) need a worst-case VBlank wall-time audit, not CPU self-time (the profiler can't see DMA). |

A single parcel can be multiple classes (a PF segment-restructure is also PS for its output
identity). The A/B plan in the packet enumerates the union.

### 1.4 The frozen root stays runnable

`capture_goldens.sh` can only reproduce the off-canonical Config-A/B goldens *while asl is live*
(the header comment: their native reproduction is Stage-2-coupled). Post-flip they are frozen
blobs. **Therefore: the asl-witnessed root CRCs are archival only — do NOT attempt to re-derive
them.** The living re-freeze machinery (§4) regenerates from *sigil's own* resolved layout, which
is exactly what a post-asl world should do.

---

## 2. The item census

**Method note (now RESOLVED by a source cross-check).** The 2026-07-16 review's two headline
segment-restructure levers overlap the pass-2/pass-3 streaming-perf campaign (gap-ledger §17 rows
22, 2, 4-11) — and the cross-check confirms **exactly those two shipped**: **tile_cache #1**
(FillRow/FillColumn/CopyBlockColumn now precomputed contiguous segment copies with move.l pairs +
move.w odd-word tails) and **plane_buffer #1** (`Draw_TileRow_FromCache .emit_row_run` two-leg
segment copy). **All other 14 review items are OPEN in current source** — no PARTIALs. (Caution:
the sprites four-variant emit-loop unroll can *look* like H2 progress but is pre-existing
infrastructure, not the H2 win.) Effort below reflects post-flip single-file cost (no twin
lockstep); the driver is region size + risk class + A/B weight, plus RAM-layout ripple where noted.

### 2.1 Census table

| Item | Verdict | Class | Effort | Evidence / note |
|---|---|---|---|---|
| tile_cache #1 (FillRow/Col/CopyBlock → segments) | **DONE** | — | — | `tile_cache.emp:1548-1612`,`417-493`,`1316-1373` — segmented, move.l+move.w tails |
| plane_buffer #1 (`Draw_TileRow_FromCache` → segment runs) | **DONE** | — | — | `plane_buffer.emp:310-353` two-leg segment copy |
| tile_cache #2 (per-slot staging ptr: empty→zero-ROM, raw→ROM-direct) | OPEN | PF | **M** | `.raw_direct` still 24-burst `movem.l` `:342-347`; `.empty_block` `clr.l` loop `:351-356`; **RAM per-slot ptr array = RAM-layout change → full ripple + provenance** |
| plane_buffer #2 (VInt_DrawLevel column drain → move.l pairs) | OPEN | PF | **S-M** | `.drain_col` still `move.w` `:452-454`; **$E000 odd-word edge = the test vector** |
| plane_buffer #3 (ready VDP command longword in entry header) | OPEN | PF | **M** | header stores bare 2-B addr `:116,:236`; **entry-format change → re-prove b96c861 tear invariant + section.emp reserve consts (cross-file)** |
| plane_buffer #4 (Draw_TileColumn wrap-check hoist) | OPEN | PF | **S** | per-iter `cmpa.l/blo` `:135-136,:177-178` |
| collision_lookup #1-3 (fused GetType+GetCollision, Row80 build-time table) | OPEN | PF | **L** | separate bounds compares `:23-36`; ×80 shift-add `:110-111,:160`; ~30%/sensor lever |
| sprites H1 (resolve-once frame offset in SST) | OPEN | BA | **M** | resolved twice `sprites.emp:81-84`+`:277`. **REOPEN-GATED: mapping_frame corpus-wide writer sweep is a hard prereq — frame-anchored A/B is blind to the drift it guards** |
| sprites H2 (emit stream-order + size\|link word merge) | OPEN | PS | **S** | `size_link` two `move.b` `:512-520`; low residual (~0.2%) |
| sprites H3 (SAT DMA length = `Sprites_Rendered*8`) | OPEN | PF(DMA) | **S** | `buffers.emp:137` fixed `640`. **REOPEN-GATED on a worst-case VBlank-bandwidth audit; profiler can't see it** |
| rings R2 (camera-bias fold to cull side) | OPEN | PF-small | **S** | fold on SAT side w/ per-ring undo `:211-220` |
| rings R3 (hoist player dims out of ring loop) | OPEN | PF-small | **S** | reloads `width/height_pixels(a2)` per ring `:290,:297` |
| animate A2 (dirty-check mapping_frame) | OPEN | PS | **S** | unconditional write+jbsr `:111-113`; ~60c |
| animate A3 (tail-call `.set_frame` via jbra) | OPEN | PS | **S** | jbsr+rts `:113-115`; ~24c |
| entity_window High #1 (per-section trigger cache) | OPEN | PF | **M** | no cache; **the `ess_*_left_idx` reuse plan is VOID — those fields were deleted phase2.5 c6 (struct $1A→$16); a reopen adds FRESH EntityScanState scratch** |
| core #1 (register-cached camera + branchless cull) | OPEN | PF | **M** | reloads `Camera_X/Y` per entry `core.emp:509,516`; branchy `abs_w+cmpi` `:510-518`; hot dispatch loop |
| **G9** (`Ground_Move_Cap` d7 high-word clear) | OPEN | BA | **S** | see §2.2 — one `moveq #0,d7`; byte-gate-blind hazard |
| **parallax row-35** (engine per-frame mode-3 write; kill harness force-write) | OPEN | PF+BA+C3 | **M** | see §2.2; two named A/B checks + reg-$0B sub-question (OQ-5) |

### 2.2 The two brief-named riders (verified live in source)

- **G9 — `Ground_Move_Cap` probe-direction `d7` high-word clear.** CONFIRMED PRESENT:
  `player_ground.emp:670-671` does `lea .dir_table(pc),a1 / move.b (a1,d2.w),d7` with **no**
  `moveq #0,d7`, and `d7` is consumed as a word at `.off_*`/`.cancel_*` (~:698/:715/:732). Benign
  today only because `Player_Main` enters with `d7` = RunObjects slot counter = 0. **Class BA**
  (byte-gate-blind hazard-fix). Fix = one `moveq #0,d7` (4 cyc). Effort **S**. Survives.

- **Parallax-hardening (row 35 / t41).** CONFIRMED PRESENT: `ojz_scroll_test.emp:284+` still
  carries the per-frame "Force VDP mode-set-3 (shadow + reg $0B direct) every frame" harness
  workaround; `Parallax_StartTransition` remains the sole engine mode-3 writer and is
  edge-triggered. **Class PF + BA + C3(VDP/timing)**. The parcel: a per-frame / on-active-change
  mode-3 write in `Parallax_Update` derived from `Parallax_Active_Config`, then delete the harness
  force-write block. Two named A/B checks (from t41): (a) engine writes the same mode the harness
  did — no render regression on the ojz boot scene; (b) the extra per-frame write does NOT perturb
  the deterministic `Debug_Scene_Freeze` cache-fill soak. **Step-0 sub-question** (t41, unresolved):
  does the engine also need the DIRECT reg-$0B write (VDP command-state reset against a
  half-finished 32-bit address command from `Section_UpdateColumns`), or can it rely on the normal
  VBlank shadow→reg flush? An engine write touching only the shadow is a behavioral change from the
  harness's direct write. Effort **M** (hot-path parallax wave + PARALLAX region re-pin).

---

## 3. The oracle harness

### 3.1 What exists today

- **Full MCP oracle emulator surface** (`mcp__oracle__emulator_*`): `screenshot path=…` (writes PNG
  to disk directly — transcription-free), `read_memory` / `read_vram` / `read_cram`,
  `write_memory` (scene poke), `run_to` / `run_to_scanline` / `step*`, `press` / `release_all`,
  `registers`, `object_list` / `object_slot` / `player_state`, and the **live profiler**
  (`set_profiler` / `get_profiler` / `get_profiler_frames`) — inclusive & self-time per proc,
  the instrument the streaming-perf campaign used.
- **Deterministic scene techniques (recorded facts):** anchor A/B to `Frame_Counter` not
  press-count; the ObjectTest soak scene via the `Game_Entry` flip; `Debug_Scene_Freeze`
  (`0xFF8A10`) + Camera poke = deterministic tile-cache fills for OLD/NEW byte-identity comparison.
- **The standing evidence rule (row 21):** NO hand-transcribed hex in identity evidence.
  Comparisons go through a hash/pipeline path — `screenshot path=…` → `cmp` for the visible plane;
  off-screen regions via file `md5sum` with a `wc -c` length assert (== 2×byte_count) immediately
  after every Write; prefer ≤256 B fresh in-context reads over 4 KB file pastes.

### 3.2 What the sweep needs built

- **`emulator_memory_hash(addr,len)` (oracle backlog, row 21).** The single highest-value tool: a
  CRC32/md5 computed emulator-side so no bytes cross the agent. It makes the full-region state-identity
  bar (§1.3 PS class) trivial and context-free, and fixes BOTH the input-context and
  output-generation halves that the sentinel-poke workaround fights. The PS-class bar over a
  14400-byte cache × multiple anchor frames is impractical without it. **Recommend building this
  before the first tile_cache/plane_buffer PF parcel.**
- **An A/B runner script** (thin, sigil-side or aeon-side): given OLD.bin + NEW.bin + a scene
  script (reset → poke → run_to Frame N → capture), produces the paired screenshot + region-hash
  set for the packet. Codifies the frame-anchored determinism so each parcel doesn't re-improvise
  it. Not strictly required (the MCP tools suffice by hand), but it makes the A/B reproducible and
  reviewable — recommend for the PF parcels.
- **`set-PC` / write-register** (row 21, secondary): would enable injected micro-benchmarks; not
  on the critical path for the sweep.

### 3.3 Determinism requirements (binding on every A/B)

Reset-deterministic scene (no human input timing); frame-anchored capture (`run_to` a fixed
`Frame_Counter`); OLD and NEW driven by the **identical** scene script; PF drives must be
**UNFROZEN** where lag is the measurement (frozen scenes under-load — they skip
`EntityWindow_Scan`); PS state-identity may use the frozen `Debug_Scene_Freeze` drive (its
determinism is the point). Every captured hex file gets its `wc -c` length assert.

---

## 4. Strict-suite interaction & the re-freeze machinery

### 4.1 What re-pins per byte-changing parcel

A size-changing edit to a §17 region moves that region's base/len **and every downstream region's
base**. The surfaces:

1. **`repin.toml` → `pins.rs`** (mechanical). `cargo run -p sigil-harness --bin repin` resolves
   every region/symbol against the **sigil-emitted** `s4.lst`/`s4.debug.lst` (post-flip the listing
   is sigil's own) and regenerates `src/pins.rs`. The per-module port tests read these
   (`tile_cache_port.rs` reads `pins::TILE_CACHE.plain_base/len`, etc.), so they re-baseline
   automatically once pins.rs regenerates. `tests/repin_pins.rs::pins_rs_is_current` guards
   staleness. **This half is one command.**
2. **The golden `.bin` blobs** (`crates/sigil-harness/golden/{s4,s4.debug,demo,demo.debug}.bin` +
   `config_a.bin`/`config_b.bin`). Regenerated by `capture_goldens.sh --write` (deletes → rebuilds
   → asserts-fresh → recaptures both split-golden layers + updates `PROVENANCE.md`).
3. **The full-file CRC/size consts, HAND-edited.** `native_full_rom.rs` (`want_crc`/`want_len` for
   s4/s4d — the assert message literally reads *"re-freeze the golden?"*) and
   `native_offcanonical_rom.rs` (the four off-canonical anchor CRC + `EndOfRom` offset consts, e.g.
   `anchor_matches(&config_b_profile(), "config_b.bin", 0x434d0)`). These are hand-typed constants.
4. **The frozen size tables** `golden/offcanonical_sizes/*.txt` — regenerated by
   `derive_offcanonical_sizes.sh` (drives sigil's own resolved layout — no asl, no listing).
5. **The upstream-slide / $8000 sweep** (gap-ledger standing bars): if the changed region sits
   upstream of the engine bank or a DEBUG-shape growth pushes an object-bank-called engine symbol
   across `$8000`, the ripple reaches `engine.inc` gate-orgs, `mixed_dac_rom` bases/windows,
   `main.asm`/`act_descriptor.asm` resume orgs. The §17 targets are engine-internal so most stay
   region-local, but each parcel runs the cheap check (an object-bank symbol's plain vs debug pin
   coincide iff the bank didn't slide).

### 4.2 The re-freeze mechanization gap (the "anchors-mode" question)

Today re-freeze is **four tools + one hand-edit**: `repin` (pins.rs), `capture_goldens.sh --write`
(blobs + PROVENANCE), `derive_offcanonical_sizes.sh` (size tables), and hand-edited CRC/EndOfRom
consts in two `native_*_rom.rs` files. **Recommendation: a single `re-freeze` driver** (a Stage-2
`sigil-harness` bin or shell wrapper) that runs all three scripts in order AND rewrites the CRC/size
consts from the freshly captured blobs, so a byte-changing parcel's re-freeze is *one command +
paste the printed A/B provenance link into PROVENANCE.md*, not a five-surface archaeology dig. This
is the optimization-era analog of `repin`. Its build is itself a small tooling parcel — recommend
it as **parcel 0** of the sweep (before the first byte-changer), so every subsequent parcel's
re-freeze is mechanical and mis-freeze is impossible. **OQ-4.**

---

## 5. The batch shape

Parcels grouped **by file** (a file's region re-pins once; multiple items in one file ride one
re-freeze) and ordered **low-risk-first** so the machinery is proven on cheap parcels before the
dominant hot-path ones:

- **Parcel 0 — tooling (byte-neutral):** build `emulator_memory_hash` (oracle) + the unified
  `re-freeze` driver + the A/B runner script. No golden move. Overseer countersign that the
  machinery is sound *before* the first byte-changer. (If `emulator_memory_hash` is out of scope
  for this arc, the sentinel/screenshot pipeline is the fallback — OQ-3.)

- **Wave A — the cheap, high-confidence riders** (each its own re-freeze commit):
  - G9 `moveq #0,d7` (class BA, effort S) — the first byte-changer; proves the whole
    A/B→re-freeze→provenance-link loop end-to-end on a 4-cycle change. **Recommended pathfinder.**
  - animate A2/A3, rings R2/R3, section H3, entity_window residuals — the structural/near-zero
    items IF still open (§2 census); each PS/PF-small, log-and-skip below threshold.

- **Wave B — the measured hot-path levers** (the streaming-perf residue NOT already shipped by
  pass-2/3): whatever of tile_cache #1/#2, plane_buffer (b)/(c)/(d), collision_lookup #1-3 survives
  the §2 census. These are class PF, need the unfrozen-drive profiler A/B + the memory-hash
  state-identity bar, and are effort L. plane_buffer (d) ready-command-in-header additionally must
  re-prove the **b96c861 tear invariant** (drain-only-on-complete-frames) and touches `section.emp`
  reserve consts (cross-file).

- **Wave C — the standalone hardening parcel:** parallax row-35 (class PF+BA+C3, effort M) — its
  own vehicle per t41, with the two named A/B checks and the reg-$0B step-0 sub-question resolved
  first.

- **Sprites H1/H3** are **reopen-gated**, not scheduled: H1 needs the corpus-wide `mapping_frame`
  writer sweep (frame-anchored A/B is blind to the drift it guards against — a hard prerequisite);
  H3 is VBlank-DMA-bandwidth, reopen only if a worst-case VBlank audit binds. Both stay parked with
  their reopen conditions; do NOT fold them into Wave B on ceiling estimates alone.

**Checkpoints:** the **overseer countersigns each re-freeze** (each parcel's A/B evidence +
provenance link, at its commit). **Volence's checkpoint is the ARC boundary** — the sweep's entry
(this note's ratification) and its exit (the post-sweep provenance chain + the aggregate
lag/cycle delta). Waves are merge groupings, not checkpoint units.

---

## Open Questions

**OQ-1 (census freshness — load-bearing).** The 2026-07-16 review's dominant levers (tile_cache
#1/#2, plane_buffer #1-4) overlap the pass-2/pass-3 streaming-perf campaign, which already shipped
NT-FillRow segment restructure + move.l pairing and "1.3" FillColumn/CopyBlockColumn restructure.
§2.1's table is the design-gate's source cross-check, but the *authoritative* done/stale verdict is
a step-0 read of `aeon/docs/reviews/2026-07-16-emp-port-optimization-review.md` against current
.emp by the first executing parcel. Which items in §2.1 does the overseer want re-confirmed before
Wave B is scheduled, vs trusted from this note?

**OQ-2 (the provenance-chain form).** §1.1 proposes provenance as an append-only chain in
PROVENANCE.md (`<parent> --[parcel, A/B ref]--> <new>`) rooted at the frozen asl witness. Is a
prose chain in PROVENANCE.md sufficient, or does the overseer want it machine-checkable (a
`provenance.toml` the harness reads, asserting the current golden CRC == the chain tip)? The latter
makes "an anchor moved without an A/B link" a hard gate failure rather than a review-catch.

**OQ-3 (emulator_memory_hash scope).** The PS-class full-region state-identity bar (§1.3) is
impractical over the 14400-byte cache without an emulator-side hash (row 21). Is building
`emulator_memory_hash` in-scope for parcel 0 of this arc, or does the sweep run on the
screenshot/sentinel-md5 pipeline and defer the tool? This gates how heavy the Wave-B A/B is.

**OQ-4 (unified re-freeze driver).** §4.2 recommends a single `re-freeze` bin (repin +
capture_goldens + derive_sizes + auto-rewrite the hand-edited CRC/EndOfRom consts) as parcel 0. Is
that machinery worth building up front, or does the overseer prefer the current four-tool +
hand-edit flow with a documented checklist for the (small) number of §17 parcels? The hand-edit of
CRC consts in two test files is the current mis-freeze risk.

**OQ-5 (parallax reg-$0B behavioral choice).** The parallax-hardening step-0 sub-question (t41):
should the engine per-frame mode write include the DIRECT reg-$0B hardware write (the harness's
VDP-command-state reset against a half-finished 32-bit address command from
`Section_UpdateColumns`), or rely on the VBlank shadow→reg flush? Shadow-only is a behavioral change
from the harness. This is a correctness call that must be settled BEFORE the Wave-C A/B is designed
(it changes what "no render regression" means).

**OQ-6 (threshold vs completeness for the near-zero structural items).** animate A2/A3, rings
R2/R3, section H3, core-residuals are all sub-threshold structural cleanups (~24-60 cyc). Post-flip
they're single-file byte-changers, but each still pays a full re-freeze + A/B + provenance link.
Does the overseer want them shipped for completeness (they're real, and the ledger rows close), or
log-and-skip as below-threshold given the per-parcel re-freeze cost now dominates their ~50-cyc
win? (The parcel-scope amendment's log-and-skip rule says skip; the "close the ledger" instinct
says ship. A ruling here sets Wave A's true size.)
