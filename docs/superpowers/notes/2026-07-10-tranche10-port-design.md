# Step-1 design — dplc.emp + core.emp transcription (tranche 10)

Step 0 SHIPPED (repin tool + generated pins.rs, 18 files migrated, strict
2068/0). This note settles the 6 port hazards from the t10 handoff BEFORE
the transcription. Reference: dplc.asm (107 ln), core.asm (328 ln), both
`engine/objects/`. Model port = animate.emp (leanest); rings.emp for the
debug-shape `ifdef __DEBUG__` transliteration pattern.

## Geometry (from current listings; already in pins.rs)

| region | start sym | plain | debug | plain len | debug len |
|---|---|---|---|---|---|
| dplc | Perform_DPLC | $26FC | $288E | 0x98 | 0x98 |
| core | InitObjectRAM | $2794 | $2926 | **0x1C4** | **0x2EC** |

core is the first SHAPE-DEPENDENT length since rings: the debug surplus
(0x2EC − 0x1C4 = 0x128) is the two `ifdebug bsr.w Debug_AssertObjLoop`
call sites (in .run_always / .run_culled) + the `Debug_AssertObjLoop` proc
itself (three `assert.l/.w` expansions). pins.rs handles both lens; the
port test slices `base..base+{plain,debug}_len` per shape.

## Hazard rulings

**H1 — Gates: per-file pair `SIGIL_EMP_DPLC` + `SIGIL_EMP_CORE`.** Prior
art (rings/collision, sound_api/game_loop) favors per-file. engine.inc
order is `dplc → core → sprites → animate`; each gate's `else` resumes org
at the region END so the downstream `.asm` includes still land at their
current addresses:
- dplc gate resume: plain `org $2794`, debug `org $2926` (= core start).
- core gate resume: plain `org $2958`, debug `org $2C12` (= InitSpriteSystem
  = sprites.asm start; core's end).
The `repin` bin PRINTS both paste-blocks (D-T10.7). Sound driver dimension:
neither file has a SOUND_DRIVER_ENABLED path — no combo probe needed
(simplest since vdp_init).

**H2 — NO re-pin wave in step 1.** Byte-exact transcription ⇒ the reference
ROMs don't move (gates unset in the reference build; the .emp is only built
by the mixed harness with the gate set). The upstream slide (animate base +
DeleteObject) the handoff warns about is a STEP-2 concern (when we shrink
bytes). Step 1: add the two manifest symbol sets, regenerate pins, wire the
tests — the tool makes this a symbol-list edit, not hand-typing.

**H3 — Two files, one tranche, dplc FIRST.** dplc is upstream and tiny
(2 near-identical procs, no debug divergence, no new constants). Land it
green, then core. Separate port tests (`dplc_port.rs`, `core_port.rs`),
separate gates, separate commits — but one branch/tranche/packet.

**H4 — core's cross-seam surface (largest yet).**
- **RAM labels** (ram.asm, per-shape .w VMAs — synthetic seam sections like
  collision_port's Player_1/Dynamic_Slots): Object_RAM (=Player_1 $89EE/$8A10),
  Dynamic_Slots ($8A8E/$8AB0), System_Slots ($970E/$9730), Effect_Slots
  ($998E/$99B0), Object_RAM_End, Dynamic_Free_Stack/SP, Effect_Free_Stack/SP,
  Spawn_Count ($9F02/$9F24), Game_Paused ($A126/$A148), Camera_X ($A11E/$A140),
  Camera_Y ($A122/$A144). PLAYER_1/DYNAMIC_SLOTS/CAMERA_X/CAMERA_Y already in
  pins.rs; the rest are manifest ADDITIONS (`[[symbol]]` rows, `repin`
  regenerate — the tool's first real re-derivation exercise).
- **cross-proc calls**: Draw_Sprite ($2970/$2C2A, sprites.asm — pins.rs has
  it), Debug_AssertObjLoop (intra-module, debug-only). dplc: QueueDMA_Important
  ($1D84/$1E06), QueueDMA_Deferrable ($1D8E/$1E10) — manifest additions.
- **constants** (values via engine.constants twin): existing NUM_PLAYERS,
  NUM_DYNAMIC, NUM_SYSTEM, NUM_EFFECTS, OBJ_CODE_BANK; NEW = NUM_TOTAL_SLOTS
  (=NUM_PLAYERS+DYNAMIC+SYSTEM+EFFECTS derived), CULL_DISTANCE_X ($300),
  CULL_DISTANCE_Y ($200), SLOT_TAG_UNTAGGED ($FF). These must be ADDED to
  constants.emp (with `ensure(extern(...))` guards) AND to
  test_support::engine_constant_equs() (twin 30 → 34) — twin_guards() derives
  the count so the port tests don't hardcode it.
- **SST fields**: SST_len, SST_slot_tag, SST_x_pos/y_pos/x_vel/y_vel,
  SST_mapping_frame, SST_prev_frame — all already in sst.emp / sst_field_equs.

**H5 — dplc's contract.** Reads mappings/art via `(a2,d0.w)` indexed EA
(probed, lowers to `D4 F2 …`); no FRAME_* constants; no game-contract
symbols (row-18 class absent). Cross-seam = only QueueDMA_{Important,
Deferrable}. The two procs are near-identical (Important vs Deferrable DMA
priority) — transcribe both verbatim; step-2 dedup candidate noted for the
retrospect, NOT step 1.

**H6 — step-5 target = RunObjects' pool loop.** Per-slot overhead ×66 slots
×60fps. That's a step-5 concern with LIVE oracle profiling
(emulator_get_profiler) — recorded here, not touched in step 1. Step 1 is
byte-identity only.

## Transcription specifics (byte-exact spellings)

- **Explicit widths** pin bytes 1-1 in step 1 (`beq.s`/`bne.w` probed:
  `67 xx` / `66 00 xxxx`). Bare Bcc is STEP 2. The `bra.s`/`bhs.s`/`bpl.s`
  in core stay `.s` for step 1.
- **`ifdebug bsr.w Debug_AssertObjLoop`** → `if DEBUG == 1 { bsr.w
  Debug_AssertObjLoop }` (rings.emp DEBUG-block pattern; the debug-shape
  byte gate is the drift guard). `Debug_AssertObjLoop`'s three `assert.l/.w`
  lines transliterate the debugger.asm `assert` macro expansion (rings.emp
  `.full` block is the exhibit) — pinned by the debug byte gate, kill-list
  row 16 class.
- **The `$50`-byte DeleteObject clear** — 20 `move.l d0,(a0)+` unrolled;
  transcribe verbatim (step-5 might question it, not step 1).
- **`move.w #Dynamic_Free_Stack+NUM_DYNAMIC*2, (Dynamic_Free_SP).w`** — the
  imm-link-with-addend form: `#extern("Dynamic_Free_Stack") + NUM_DYNAMIC*2`
  as a `.w` immediate. IR Expr::Binary folds it at link (Value16Be fixup).
  Probe FIRST that a `#Sym+const` word-immediate lowers (the `#Sym` bare
  form failed the naive spelling — needs `extern(...)` or the imm-link path;
  confirm the exact spelling before committing the whole file).
- **`(Object_RAM_End-Object_RAM)/4-1`** as a `.w` immediate — a link-time
  symbol difference `/4-1`; RelWord-class fold or comptime — probe.
- **`Sst.<field>(a0)`** field access replaces `SST_<field>(a0)`.

## Deliverables (step 1 gate, per file)

Gate + region pin (pins::DPLC / pins::CORE) + byte gates BOTH shapes +
mixed-build acceptance (mixed_dac_rom gains DPLC/CORE tranche rows) +
negative probes (doctored-byte genuineness, standalone-compile loud,
wrong-base placement) + gate-off neutrality (reference build unchanged with
gate unset). Demanded features ship here; byte-exactness proven here.

## Open probes to run BEFORE bulk transcription (de-risk) — RESULTS

1. **`#extern("X")+K` word-imm-with-addend** → LOWERS (Value16Be @ offset 2;
   the `+K` folds into the LinkExpr). Bare data-label `#Label+K` does NOT
   (comptime-eval chokes — data labels aren't comptime values); MUST spell
   the cross-seam symbol as `extern("X")`. ✓
2. **`#(extern("A")-extern("B"))/4-1` symbol-difference division** → LOWERS
   (Value16Be @ 2, linker folds the arithmetic). ✓
3. **THE core-demanded gap (step-1 blocker, demanded-features law):**
   `move.w #<link-imm16>, (<link-abs>).w` and `cmpi.w #<link-imm16>,
   (<link-abs>).w` — a link immediate SOURCE **plus** a pinned-abs.w link
   DESTINATION — is REJECTED today (`[lower.imm-link] a link-time immediate
   combined with another symbolic operand is not yet supported`,
   code.rs:714). Reference encodings (s4.lst): `move.w
   #Dynamic_Free_Stack+NUM_DYNAMIC*2, (Dynamic_Free_SP).w` = `31FC 9EDE
   9EDE`; `cmpi.w #Dynamic_Free_Stack, (Dynamic_Free_SP).w` = `0C78 9E8E
   9EDE`. Two INDEPENDENT fixups (imm Value16Be @ offset 2, abs.w Abs16Be @
   offset 4 — no collision; the "fixups would collide" comment is
   over-broad). Four core sites (Init×2, Alloc×2). Not avoidable: the abs
   targets are per-shape RAM cells, not comptime consts. **RULING: extend
   `lower_m68k_imm_link` to admit ONE pinned-abs operand (`AbsSym{long}`)
   alongside the ImmLink, second fixup at `2 + imm_field_width` — ships in
   step 1. Keep rejecting relaxable `Sym`/`SymOff` (width selection would
   genuinely conflict).** → gap-ledger + kill-list nothing; this is a
   permanent language capability, not scaffolding.
4. The debug-block `assert` transliteration reproduces Debug_AssertObjLoop's
   bytes — deferred to the core transcription (compare vs s4.debug.lst
   $2AA2 window; rings.emp `.full` is the exhibit).
