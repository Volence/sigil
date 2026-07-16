# Tranche 17 — `plane_buffer.emp` FULL MERGE PACKET (→ Fable merge gate)

**Branches:** `port-tranche17` both repos. **Byte-neutral end-to-end** —
`plane_buffer.asm` (the AS twin) was NEVER touched; the shipped ROMs stay
canonical through every step. No re-pin.

- aeon tip `8732c3c` (6 commits above master `513ba30`)
- sigil tip `29b40aa` (7 commits above master `a67a1d1`)
- Canonical ROMs UNCHANGED: plain **453087 / b335bdc6**, debug **461110 / 827e18c4**
- Paired-state strict (`SIGIL_STRICT_GATE=1 AEON_DIR=<worktree> cargo test --workspace`,
  **after a full clean rebuild**): **2262 / 0** (2257 baseline + 5 plane_buffer_port).
  `repin_pins::pins_rs_is_current` green.

## Loop shape traversed
`0 → 1 → 2 → (3→4→5)c1 → (3→4→5)c2-DRY → 6`. Steps 0/1/2 once; circuit 1 found
+ did things; circuit 2 came up empty at all three (a fresh retrospect found no
new reads-wrong, step 4 built/adopted/deleted nothing, step 5 took nothing) →
DRY. Step 6 sweep ran once after. **Result: the whole port is byte-neutral.**

---

## Step 1 — transcribe (gated PASS-with-rider by Fable; full artifacts in the step-1 checkpoint note)
5 procs 1-1 from plane_buffer.asm; SIGIL_EMP_PLANE_BUFFER gate ($42FA/$4EC4 resume);
PLANE_BUFFER region pin + PLANE_BUFFER_BASE. Gate artifacts: `plane_buffer_region_matches_reference`
(+`_debug_`), the 3rd ownership FLIP `two_module_ownership_flip_{plain,debug}` (plane_buffer.emp
+ section.emp, section's Draw_* labels dropped), negative probe `doctored_plane_buffer_size_fires_its_guard`,
gate-off canonical CRCs. **Rider discharged:** kill-list row-5 body-twin backfill written as a proper
gap-ledger row (not just the packet). **repin tooling:** `SymbolSpec` gained an optional `const_name`
override (Plane_Buffer's snake-upper collided with the region const → PLANE_BUFFER_BASE).

## Step 2 — modernize (the filled 6-item checklist)
BYTE-NEUTRAL — no relaxation, no region shrink, `.asm` twin untouched.
1. **Branch conversions:** all conditional Bcc → bare; `bra.s` → `jbra` (3 sites). **No shrink** —
   the six `.w` guards (`bhi/blt/bgt .done` in the two producers) measure **154–274 bytes** to `.done`
   (listing-verified), all out of ±127 `.s` range, so bare resolves to `.w` unchanged; every other
   branch was already `.s`; `jbra`→`bra.s`. Byte delta = 0, no downstream absorption needed.
2. **Structural width-pins:** NONE kept — after conversion there are zero explicit branch/abs width
   overrides. The six far guards use bare (assembler picks `.w` by distance = normal relaxation, not a
   forced pin); `cmpa.l`/`lsl.l`/`move.l`/`movea.l` are data-op widths (not relaxable). Nothing to comment.
3. **Bare-symbol width-rule spellings:** complete — all RAM abs (`Plane_Buffer_Ptr`, `Cache_*`,
   `Section_Right_Col_Written`, `Current_Act_Ptr`, `Plane_Buffer`), `Tile_Cache_Nametable[+NT_SIZE]`,
   and `VDP_DATA` are bare (`(X).w`/`.l` → `X`). No comptime-fn/asm-template bodies in this file.
4. **Brace-indent:** conformant from birth (proc bodies 8-sp, labels 4-sp; tile_cache/section pattern).
5. **Idiom list (per-line):**
   - Sec/Act field access: `Sec.sec_bg_layout(a0)` / `Act.act_bg_layout(a2)` via `use engine.structs` — satisfied from step 1 (directed).
   - TILE_CACHE_{COLS,ROWS,STRIDE,NT_SIZE}: `use engine.constants`, no file-local mirror — satisfied from step 1 (directed).
   - absolute-EA over link base bare `sym + const`: yes (`Tile_Cache_Nametable+TILE_CACHE_NT_SIZE`, RAM labels) — NOT the `(sym).w/.l` override.
   - contract reglists movem-RANGE form: yes (`d0-d5/a0-a2`, `d0-d3/a1-a2`, `d0-d1/a0/a5-a6`) — no comma-enumerated contiguous run.
   - typed VDP fns: NOT-APPLICABLE-at-step-2 — VInt_DrawLevel's raw `$8Fxx` + runtime shuffle is a step-3(a)/4 typed-VDP question (§7), surfaced there (see step 6).
   - Sst.field / bareword winptr,bankid / label-in-immediate: not-applicable (no object-SST access, no window/bank ptrs, no label immediates).
6. **Noticing clause:** proposed list item — a compound-constant displacement `A-B(An)` is spelled as a
   named derived const (`const D = A-B; D(An)`) until the grammar ask lands. Fable pre-blessed
   `VDP_CTRL_OFF` as the house spelling for this class; proposing it as a ratified step-2 idiom line.

## Circuit 1 — (3 → 4 → 5)

### Step 3(a) interrogation (language/format asks)
- **Ceremony scan:** lines-per-intent tight (68k). Only recurring shape = the VDP command-long shuffle
  ×2 (VInt drain heads) → step-4 candidate (resolved at step 6: it's section's `vdp_comm_reg`).
- **Comment-as-compensation:** the VInt entry-format header comments describe a raw word protocol
  (`[addr][flags|count][data]`) — a typed plane-buffer-entry writer is a speculative type ask (noted, not built).
- **Escape-hatch census:** 7 drift-lock `ensure(extern())` (standard twin pattern), 1 `VDP_CTRL_OFF`
  displacement workaround (ledgered). No recurring NEW shape.
- **Domain-type scan:** VramAddr / tile GridCoord candidates (on the existing newtype list) flow through
  bit-manipulation + stride arithmetic — marginal, high friction; noted, not built.
- **Noticing:** none new.
- **ASKS RAISED:** (i) `$8Fxx` typed VDP register-SET word `vdp_reg(reg,val)` — verb-(c), the register-word
  axis section's `vdp_comm_reg` doesn't cover (ledgered, census to-run); (ii) compound-const displacement
  `A-B(An)` grammar (ledgered at step-1, restated).

### Step 3(b) interrogation (reads-wrong)
- **Comment-claim audit:** ONE false claim — `Plane_Buffer_Reset`'s "call each frame after drain" (it has
  NO callers; VInt_DrawLevel resets inline). **FIXED** (rewritten to name its real role: the level/act-transition
  reset hook — converges with the step-5 stale-drain audit). All other claims verified TRUE (the 136/132/68
  overflow budgets, the ×80/×160 shift-adds, the `$801F` header, the row/col drain modes, the 60/59 wrap).
- **Contract audit:** all 5 procs' clobbers/out match actual register usage (Draw_BG preserves its `a0` input; verified).
- **Name audit:** labels/consts accurate.
- **Magic-number audit:** `$8000` column flag → NAMED `PLANE_ENTRY_COL_FLAG` (step-4 build). The remaining
  literals (`#63`/`#64`/`#32`/`#128`/`$FFC0`/`$8Fxx`) are comment-documented; not named — mixed cell/byte
  semantics make names LESS clear (not-taken-with-reason).
- **Cold-reader / codename-reference / noticing:** clean (spec §4.1/§4.2 + gap-ledger are durable anchors; no session codenames).

### Step 4 — construct pass
- **build:** `PLANE_ENTRY_COL_FLAG = $8000` (names the entry-header column-mode bit; adopted at 3 sites). .emp-only readability const, no AS twin → no kill row.
- **build→REVERTED:** `vdp_cmd_from_addr` comptime-fn (the shuffle) — the step-6 sweep proved it's a byte-exact duplicate of section.emp's `vdp_comm_reg(Vram, Write, clr=false)`; a build-where-adopt-was-available miss, reverted (see step 6).
- **adopt / ask / delete:** delete = NONE (both no-caller procs KEPT per ruling); ask = the two §3(a) asks.

### Step 5 — optimize (the filled interrogation + Probe A; charter = ledger row 1066)
**Probe A RAN** (live oracle, shipped tip 827e18c4 hash-verified, OJZScroll + Debug_Scene_Freeze camera-poke
sustained-max-H, 6-frame @ 100% budget). **Outcome:** Draw_TileColumn = **7.5%/frame** (~4800 cyc/call, matches
the row-1057 jot) — a REAL cost but **~5× secondary** to tile_cache's copy/decompress half (Tile_Cache_Fill 37.6%
incl. = FillColumn 35% + CopyBlockColumn 21.4% + decompress ~11%). VInt_DrawLevel drain 2.0% (VBlank). The
H-crossing lever is tile_cache's charter, not plane_buffer's.
- **Invariant ladder:** found Draw_TileColumn's per-word wrap-check (`cmpa.l a1,a0 / blo / suba.w` every
  iteration though the row-59→0 wrap happens ≤ once/column) — a hoistable ~1.4%-of-frame candidate (the
  horizontal analog of Wave-1's FillRow hoist+SR). **NAMED + DEFERRED** to a coherent H-streaming pass with
  tile_cache's dominant half (ledgered) — optimizing a 1.4% secondary lever in isolation is worse sequencing,
  and the frozen-scene drive can't run Probe B's lag-A/B (no lag under freeze).
- **Counter/cache audit → the pre-registered finding (b):** the ONLY `Plane_Buffer_Ptr` writers are inside
  plane_buffer.asm (3 producer commits + VInt's post-drain zero) + cold-boot RAM clear; nothing on the
  level/act-init path clears it. **Outcome: NO live bug** (single-level harness — no act transition exists yet),
  **latent hazard** (a pre-transition lag-frame's undrained entries would drain stale old-level addresses into
  new VRAM when multi-act transitions are wired) — and `Plane_Buffer_Reset` is EXACTLY that reset hook (finding
  b + the §9 dead-code flag converge; the proc is kept + its header now says so).
- **Guard-coverage:** the 3 producer overflow checks are each on their sole append path (load-bearing). ✓
- **Hardware cross-check:** VInt_DrawLevel runs in VBlank (Z80 stopped by caller); beam-position gate is dead
  ([[tile-cache-fill-runs-in-vblank]]); the `$8F02/$8F80` autoinc + command longs are correct VDP protocol.
- **Silent-tradeoff:** "silently drops if buffer full" + the rows_A clamp/zero-fill are documented CHOSEN compromises. ✓
- **Step-5 decision: NO byte-changing change this tranche** — recorded with the Probe A split + the named candidate.

## Circuit 2 — (3 → 4 → 5) — DRY
Fresh retrospect over circuit-1's byte-neutral spelling/naming: step 3 found no new reads-wrong or ask,
step 4 built/adopted/deleted nothing, step 5 took nothing. Empty at all three → dry.

## Step 6 — corpus sweep (runs once after the dry circuit)
Enumeration of every NEW thing t17 added that prior files could use:
- **VDP addr→command shuffle:** grep hit `section.emp:118` — my step-4 `vdp_cmd_from_addr` is a byte-exact
  duplicate of `vdp_comm_reg(Vram, Write, clr=false)`. **REVERTED** the duplicate; **LEDGERED** the real finding:
  the typed-VDP command interface (VdpTarget/VdpOp/mappers/vcr_*/vdp_comm_reg/vdpComm) has its 2nd .emp consumer
  → hoist to a shared module (2nd-consumer consolidation, TILE_CACHE_* precedent; own batch — cross-file, not
  `pub`, wrong direction to import from section).
- **PLANE_ENTRY_COL_FLAG:** plane-buffer-entry-specific — no other consumer (Draw_* is the sole producer). Not-an-instance.
- **compound-const displacement (`VDP_CTRL_OFF` pattern):** grep found NO other `A-B(An)` site in the corpus. Not-an-instance.
- **bare-Bcc/jbra/bare-abs:** already house format (not new to t17). Not-an-instance.

---

## Rulings discharged
- KEEP both no-caller procs — done; `Draw_BG_TileColumn` (forward-scaffolding) + `Plane_Buffer_Reset` (the
  now-documented act-transition reset hook) ported faithfully. Pre-registered findings (a) header + (b)
  stale-drain BOTH discharged (converged on the reset-hook rewrite).
- Row 1052 NOT claimed (zero vdpComm in the file). §7 typed-VDP asks surfaced at 3(a)/4/6, not step 1.
- Sec/Act via engine.structs + TILE_CACHE_* via engine.constants (batch updates a/b) applied.

## Ledger / kill-list deltas
- gap-ledger: +5 rows (compound-const disp; kill-row-5 backfill [rider]; `$8Fxx` typed vdp_reg ask; VDP-interface
  2nd-consumer hoist; Probe A split + wrap-hoist candidate).
- kill-list: row 5 (+plane_buffer.asm twin + row-5-lag flag), row 6 (+PLANE_BUFFER pin). No new comptime-fn twin
  (row 29 added then removed with the revert).

## What each pass added
- **Step-1 demanded features:** NONE (clean transcribe).
- **Circuit 1 — step-3 findings:** 1 reads-wrong FIX (Plane_Buffer_Reset header); 2 language asks ($8Fxx vdp_reg,
  compound-const disp); 1 magic-number naming (PLANE_ENTRY_COL_FLAG); magic-number not-taken decisions recorded.
- **Circuit 1 — step-5 findings:** Probe A measurement (Draw_TileColumn 7.5%, secondary); wrap-check-hoist
  optimization NOT-TAKEN (deferred to coherent H-streaming pass, ledgered); stale-drain audit (latent, hook named).
- **Circuit 2:** empty (dry).
- **Step 6 (own section):** VDP-interface 2nd-consumer consolidation (duplicate build reverted + ledgered).
- **Neither bucket:** repin `const_name` override (step-1 tooling); the compound-const disp workaround
  (`VDP_CTRL_OFF`, byte-neutral, existing features).

## Testing-environment note (not a code finding)
The paired-state strict suite initially showed spurious `NotFound`-fixture failures across unrelated crates
(sigil-cli embed, sigil-clownlzss-sys golden) — STALE test binaries with a bad baked `CARGO_MANIFEST_DIR` from
the long session's incremental builds. A **full `cargo clean` + rebuild** cleared them (2262/0). Recommend the
merge-gate strict run be done from a clean build to avoid this false signal.

## Merge mechanics
`--no-ff` both repos, pushed TOGETHER (paired-state; coupled masters, no stale window). Post-merge provenance
UNCHANGED (byte-neutral port): plain 453087/b335bdc6, debug 461110/827e18c4 stay the reference. No re-pin.
