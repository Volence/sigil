# Tranche 8 step 0 — rings.asm port design (2026-07-10)

Target: `engine/objects/rings.asm` (265 lines, 5 procs: RingBuffer_Add /
RingBuffer_Remove / RingBuffer_Clear / DrawRings / RingCollision) →
`engine/objects/rings.emp`. Ratified by Volence as tranche 8 (handoff
`notes/2026-07-10-tranche8-handoff.md`).

## Region geometry (from the 2026-07-10 master listings; RE-DERIVE at pin time)

The rings block runs from collision's resume org to entity_window's
`Collected_Init`:

- plain: `s4.bin[$31F0..$33A8]` — 0x1B8 bytes
- debug: `s4.debug.bin[$34AA..$36BE]` — 0x214 bytes

**FIRST shape-dependent-length region in the campaign** (all prior regions
were shape-invariant length; only the base moved). The 0x5C-byte delta is
the `__DEBUG__` assert block in `RingBuffer_Add.full`. Harness `Shape`
grows a per-shape `len` field; the map toml's region size becomes
per-shape.

## H1 — the DEBUG assert block (THE design question)

Rings is the FIRST ported file carrying `__DEBUG__`-conditional CODE. The
`assert.b d4, eq, #0` expands through debugger.asm's macro tower
(`assert` → `_assert` → `RaiseError` → `__FSTRING_*`) into, per the debug
listing ($34E8..$3548):

```
1838 AC18       move.b  (Ring_Add_Dropped).w, d4    ; abs.w RAM (shape-dep, link)
40E7            move.w  sr, -(sp)
B83C 0000       cmp.b   #0, d4                      ; CMP encoding, not CMPI
6700 004E       beq.w   .skip                       ; .skip = CCR restore below
487A FFFE       pea     *(pc)
40E7            move.w  sr, -(sp)
554F            subq.w  #2, sp
1F44 0001       move.b  d4, 1(sp)                   ; FSTRING arg push (%<.b d4>)
4EB9 0006644C   jsr     (MDDBG__ErrorHandler).l     ; abs.l link symbol
<string blob>   "Assertion failed:…" + console tokens + arg descriptor
                + flag byte + align                 ; CONSTANT bytes
4EF9 00067212   jmp     (MDDBG__ErrorHandler_PagesController).l  ; abs.l link
46DF   .skip:   move.w  (sp)+, sr
```

**Decision: transliterate, don't port the macro tower.** The .emp spells
every INSTRUCTION as real asm inside `if DEBUG == 1 { … }` (the two abs.l
targets and the abs.w RAM read resolve through the link seam like any
other extern), and the FSTRING string/descriptor/flag data as a `dc.b`
blob copied verbatim from the reference listing, with a comment naming its
generator (`__FSTRING_GenerateDecodedString` over the assert's format
string).

- Precedent: kill row 9 (gameDebugTick macro seam) — mirror the macro
  BODY with a loud drift guard rather than port the macro system.
- Drift guard: the debug-shape byte gate covers the whole block — any
  debugger.asm macro change that alters the expansion fails the gate
  loudly. No separate ensure needed (the gate IS the guard).
- New kill row (16): "assert-expansion transliteration in rings.emp
  (truth: debugger.asm's assert/RaiseError/FSTRING macros)" — dies when
  debugger.asm ports or when an .emp assert/diagnostics construct lands.
- Step-3 ask (FIRST demand data point, recorded not built): an .emp
  `assert`/diagnostic construct with a comptime format-string compiler.
  One call site does not justify the machinery; the debugger.asm port era
  does. The demanded-features law is satisfied — the file demands the
  BYTES, which `asm` + `dc.b` express; it does not demand the construct.

## H2 — build-shape defines

`-D DEBUG=0|1` and `-D SOUND_DRIVER_ENABLED=0|1` (the mt_bank/game_loop
`-D NAME=0|1` convention; the .emp spells `if DEBUG == 1`, mirroring the
AS twin's `ifdef __DEBUG__`). Two conditional sites:

- the assert block (H1) — DEBUG
- `bsr.w Sound_PlayRing` in RingCollision — SOUND_DRIVER_ENABLED

Reference gates run (DEBUG=0, SND=1) and (DEBUG=1, SND=1) — both pinned
ROMs have sound on. The other two combos have no pinned ROM: a
game_loop-style combo probe assembles the AS twin with the same defines
through the AS front-end and byte-diffs module-level (the matrix is the
drift guard for the conditional MIRRORING, not just the values).

## H3 — constants (three homes, all immediate-position → mirrors)

All uses are immediate/moveq/addq positions, so the `.b`/`.w` imm-link
deferral gap (row 10) forces comptime mirrors:

**Engine-owned → constants twin grows a rings/sprites block**
(consolidation class of rows 2/3/8; twin list 18 → 24 in
`test_support.rs::engine_constant_equs()`):
- `RING_HEIGHT` (16), `RING_ANIM_FRAMES` (4), `RING_ANIM_SPEED` (8) —
  truth `engine/constants.asm:401-403`; kill = row 1's flip.
- `MAX_VDP_SPRITES` (80), `VDP_SPRITE_X_OFFSET` (128),
  `VDP_SPRITE_Y_OFFSET` (128) — truth `engine/objects/sprites.asm:6-8`;
  NEW kill row (17): dies when sprites.asm ports (ownership flip), or
  consolidates into row 1 if the constants ever move engine-central.

**Game-owned → module-local mirrors in rings.emp + ensures** (row 10
class — the mirror lives in the consumer module; truth
`games/sonic4/config/constants.asm:48-50,70`):
- `MAX_RING_BUFFER` (128), `RING_BUFFER_ENTRY_SIZE` (6), `RING_WIDTH`
  (16), `VRAM_RING_PLACEHOLDER` (VRAM_TEST_OBJ+8 — a PLACEHOLDER value;
  the ensure makes the eventual real-art change loud on both sides).
- NEW kill row (18): dies at the imm-link width deferral, or the game
  config port, whichever first. These are game-CONTRACT symbols
  (engine.inc requires them), so the reverse-seam story at Spec 5 runs
  through the contract surface.

`NUM_PLAYERS`, SST fields, `sizeof(Sst)` — already twin/typed-covered.

## H4 — the zero-disp collapse probe (row 13's promise)

Rings' aabb calls become `aabb_axis_test(d4, a0, 0, …)` (X: ring entry +0)
and `aabb_axis_test(d5, a0, 2, …)` (Y: +2). The AS twin writes `(a0)` and
`2(a0)`. asl parity therefore requires the F1 disp-splice `{boff}({breg})`
with boff=0 to emit mode-(An) (no extension word). The general lowering
path already collapses (`lower/code.rs::collapse_zero_disp`, the tranche-6
`Sst.code_addr(a0)` demand); collision only exercised NONZERO offsets
through the splice. Step 1 ships an explicit probe (harness test asserting
the 2-byte `sub.w (a0), d1` encoding via the template path); if the splice
path somehow bypasses the collapse, that's the step-1 compiler fix.

## H5 — kill row 13 executes as CONSOLIDATION, not deletion

The row's written condition ("delete the .inc, guard and all")
OVER-PROMISED: after this port, `aabb.inc`'s remaining consumers are the
two GATE-OFF AS TWINS (collision.asm + rings.asm), which live until
Spec 5 (row 5). The .inc cannot be physically deleted while the dual
build exists — and its lockstep burden remains (an .emp template change
still edits the .inc so the twins stay byte-locked).

Execution: row 13 dies as an independent row; the .inc is re-homed under
row 5 as twin scaffolding (delete with the twins at Spec 5). LOCKSTEP
comments in aabb.inc/aabb.emp update to point at row 5's condition.
The row-13 correction is itself a retrospect item: kill conditions
written before their port should be re-verified against the gate-off
shape's needs.

## H6 — cross-seam symbol inventory

INBOUND RAM labels (per-shape VMAs from listings at pin time):
`Ring_Buffer`, `Ring_Count`, `Ring_HighWater`, `Ring_Add_Dropped`,
`Ring_Anim_Timer`, `Ring_Anim_Frame`, `Ring_Counter`, `Camera_X`,
`Camera_Y`, `Player_1`.

INBOUND code labels (per-shape): `Collected_MarkRing`,
`EntityWindow_EntryForSection`, `EntityLoaded_Clear`, `Sound_PlayRing`
(SND combos only), `MDDBG__ErrorHandler` +
`MDDBG__ErrorHandler_PagesController` (DEBUG combos only).

OUTBOUND `pub proc`s: all five (RingBuffer_Add/Remove/Clear, DrawRings,
RingCollision) — callers are entity_window.asm, sprites.asm, game states.
Outbound consumer proof per collision_port precedent.

Intra-module: RingCollision `bsr.w RingBuffer_Remove` (stays local).

## H7 — transcribe-fidelity notes

- Ring buffer entries are 6-byte packed records (x.w, y.w, section_id.b,
  list_index.b) read via literal offsets 0/2/4/5 and the ×6 index math
  (add/add shift chain). Transcribe keeps the literals. Step-3 candidate
  ask: a typed non-SST buffer-entry view (record over raw RAM — the
  role-typed-SST cousin), and whether `.emp` wants a strength-reduced
  index-scale idiom.
- `RingCollision`'s `.no_hit` target is passed into BOTH aabb splices
  (same mlab twice) — F2 exercised with a REUSED local label; the .inc
  needed utag to disambiguate `.aov`, hygiene makes the reuse free.
- `andi.b #$FE, ccr` / `ori.b #1, ccr` carry-flag returns in
  RingBuffer_Add — first CCR-immediate ops through the .emp path (check
  encoding support at transcribe).
- DrawRings' SAT-write pointer contract (a4/d5 in-out with
  Render_Sprites) is a proc-signature question for step 2 — transcribe
  keeps bare clobbers-style like TouchResponse.

## Gate mechanics (step 1 checklist shape)

- engine.inc: `ifndef SIGIL_EMP_RINGS` around the include + per-shape
  resume orgs ($33A8 / $36BE — re-derive from listings).
- Harness: `rings_port.rs` on the collision_port model, per-shape LEN,
  drift-guard count grows (twin 18→24 + rings' game-side ensures + row
  11's 30 SST pins ride along), zero-disp probe, sound-off combo probe,
  outbound consumer, gate-off neutrality, mixed-build ladder extension
  (`SIGIL_EMP_RINGS` joins lib.rs's define ladder + mixed acceptance).
- Worktree: fresh aeon worktree MUST seed editor data
  (`cp -rp games/sonic4/data/editor .worktrees/<wt>/games/sonic4/data/`).
- Branches: sigil `port-tranche8`, aeon `sigil-emp-tranche8`.
