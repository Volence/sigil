# 2026-07-29 — t41 step-0 port-execution plan (object_test_state + ojz_scroll_test)

Porter: t41. Follows the STOP-1 survey + the CARRY-AS-IS adjudication. This is the
step-0 design/recon note that de-risks the two-file port BEFORE the byte-exact code —
every idiom mapping, pin value, gate arm, constant mirror, and the two demanded-feature
findings, grounded in the resident tree. The port is CANONICAL-BYTES (byte movement
ZERO both shapes; house-format modernization must stay byte-neutral against the
canonical ROM windows).

## 1. Per-shape region pins (from the fresh listings, canonical CRCs verified)

| region | plain base | plain len | debug base | debug len |
|---|---|---|---|---|
| `OBJECT_TEST_STATE` | `$5C230` | `$5BC` (1468) | `$5DC82` | `$658` (1624) |
| `OJZ_SCROLL_TEST`   | `$5C7EC` | `$2C2` (706) | `$5E2DA` | `$2CE` (718) |

Both SHAPE-DEPENDENT → separate plain/debug pins + windows per file (repin.toml adds
2 regions; pins.rs gets `OBJECT_TEST_STATE`/`OJZ_SCROLL_TEST` with plain/debug
base+len). Region-end boundaries: object_test_state ends where ojz begins; ojz ends
after `PlayerMarkerTile` (128 B) where the main.asm level-1 include resumes.

## 2. main.asm gate arms (gameStatesIncludes, :485-487) — BOTH per-shape

Both files are shape-dependent, so BOTH gates carry per-shape resume orgs (the
path_swap precedent, t39 §3.2 — NOT the shape-invariant single-org form). Model:

```
gameStatesIncludes macro {GLOBALSYMBOLS}
    ifndef SIGIL_EMP_OBJECT_TEST_STATE
      include "games/sonic4/test/object_test_state.asm"
    else
      ifdef __DEBUG__
        org $5E2DA          ; ojz start (debug)
      else
        org $5C7EC          ; ojz start (plain)
      endif
    endif
    ifndef SIGIL_EMP_OJZ_SCROLL_TEST
      include "games/sonic4/test/ojz_scroll_test.asm"
    else
      ifdef __DEBUG__
        org $5E5A8          ; ojz end (debug) — level-1 include resumes
      else
        org $5CAAE          ; ojz end (plain)
      endif
    endif
    endm
```

Kill rows same-commit: row-5 class (2 gate-off `.asm` body twins) + row-6 class (2
region pins) + the per-shape-org ripple rider (t39 row-1257 class, ×2). Gate defines
must NEVER be set for other games (demo takes the includes).

## 3. Idiom map (grounded in the resident .emp corpus)

- **Module headers:** `module games.sonic4.object_test_state in object_test_state` /
  `... .ojz_scroll_test in ojz_scroll_test`. These are game-STATE code (no objdef
  header); the labels are plain `GameState_*` procs. Header states role + gate +
  canonical-emission status + the C3 hardware claims (module-header rule, item 7).
- **Cross-module calls** `jsr X` → `jbsr X` (byte-neutral: test_animated.emp
  AnimateSprite/Perform_DPLC precedent — jbsr picks the same width asl emits for the
  cross-section `jsr`). Tail `bra`/`jmp` → `jbra`. Local `bra.s`/`bne.s`/`blo.s` →
  `jbra` / bare `bne` / bare `blo` (bare Bcc, auto-width).
- **SST field access** on the typed param `a0: *Sst` → bare `field(a0)`; on a scratch
  register (a1 from `lea Player_1,a1` / `AllocDynamic`) → qualified `Sst.field(a1)`
  (test_churn.emp:90/92 precedent).
- **objroutine(Label)** → inline `#Label - ObjCodeBase` (test_churn.emp precedent; no
  `objroutine` comptime fn exists in the corpus). `ObjCodeBase` = cross-seam bare link.
- **Absolute RAM/ROM EA** = bare symbol + auto-width (the abs-EA idiom, step-2 item 5):
  `(Palette_Buffer).w` → `Palette_Buffer` (.w) with a `// (Palette_Buffer).w` twin-note;
  `(VDP_HV_COUNTER).l` → `VDP_HV_COUNTER` (.l, ≥$8000); `(VDP_CTRL).l`/`(VDP_DATA).l` →
  bare (.l). Link-time immediates `#Dynamic_Free_Stack+NUM_DYNAMIC*2` stay bare
  (label-in-immediate).
- **setVDPReg reg, val** → inline 2-liner (parallax.emp:229-230 precedent):
  `move.b val, VDP_Shadow_Table + VDP_MODE2_OFF` + `ori.l #(1<<VDP_MODE2_OFF),
  VDP_Dirty_Mask`. (`use engine.vdp.{VDP_MODE2_OFF, VDP_MODE3_OFF, VdpTarget, VdpOp,
  vdp_comm, VDP_DATA, VDP_CTRL}`.) The "shared set_vdp_reg helper" is ledgered adoption
  debt (parallax census) — a step-4 build candidate this tranche adds a 3rd+ consumer to.
- **vdpComm(vram_bytes(T),VRAM,WRITE)** → `vdp_comm(vram_bytes(T), VdpTarget.Vram,
  VdpOp.Write)` (typed, engine.vdp; bg.emp precedent). `vram_bytes` from
  `engine.objects.objdef` (`use engine.objects.objdef.{vram_bytes}` — hoisted, kill row
  63 closed).
- **stopZ80/startZ80** → `stop_z80()`/`start_z80()` (`use engine.z80_bus.{stop_z80,
  start_z80}`; bg.emp precedent).
- **VBlank-masked copy** (ojz :78-95 marker tiles) → the bg.emp:64-97 shape (`move.w
  sr,-(sp)` / `move.w #$2700,sr` / `stop_z80` / `move.l #vdp_comm(...),VDP_CTRL` / …
  loop … / `start_z80` / `move.w (sp)+,sr`). KEEP ojz's FAITHFUL spelling: direct
  `move.l (a0)+, VDP_DATA` loop (NO `lea VDP_DATA,a2`) + `lea PlayerMarkerTile(pc),a0`.
- **pc-relative lea** `lea Label(pc), aN` → identical in .emp (boot.emp:94 precedent).
  KEEP `lea TestPalette(pc)` / `lea PlayerMarkerTile(pc)` / `lea
  OJZ_SectionMarkerColors(pc)` etc.
- **swap/clr.w Coord promotion** (object_test_state emitter/churn spawn; ojz Player_1
  init :59-68) — a `pixels_to_coord` (engine.coords, kill row 49) STEP-4 adopt
  candidate; step-1 keeps `swap`/`clr.w` inline (byte-identical, the helper IS
  swap+clr.w). ojz is the ledgered step-6 sweep site for row 49.
- **DEBUG-divergent block** `ifdef __DEBUG__ … else … endif` → `if DEBUG == 1 { … }
  else { … }` (object_test_state profiling; ojz's 2 Debug_Scene_Freeze skip-blocks —
  `if DEBUG == 1 { tst.b Debug_Scene_Freeze / bne .skip }`).

## 4. Constant mirrors + drift guards (row-54/65 game-config class)

- `const VRAM_TEST_OBJ: VramTile = $03E0` + `ensure(extern("VRAM_TEST_OBJ")==...)`
  (both files — object_test_state DMA + ojz; row-65 class, now +2).
- `const VRAM_TEST_MARKER: VramTile = $03F8` + ensure (ojz marker tiles; `=
  VRAM_RING_PLACEHOLDER+16 = VRAM_TEST_OBJ+$18`).
- `const STUB_FLOOR_Y = 192` + `ensure(extern("STUB_FLOOR_Y")==192)`
  (object_test_state; the test_player.asm surviving-header equate, t39 §9 — used in
  `#STUB_FLOOR_Y<<16` and `dc.w …,STUB_FLOOR_Y,…`, so a comptime const, not a bare link).
- `const CAM_SCREEN_HALF_W = 160` / `CAM_SCREEN_HALF_H = 112` + ensures (ojz Player_1
  spawn; camera.asm equates, not exported by camera.emp — file-local mirror class).
- `use engine.constants.{NUM_DYNAMIC, NUM_EFFECTS, SECTION_SIZE_SHIFT, SCREEN_WIDTH,
  SCREEN_HEIGHT}` (object_test_state slot math; ojz section math).

## 5. TWO demanded-feature findings (step-1 — the data-table surface)

Both files carry LABEL-BEARING DATA TABLES + BINARY BLOBS inside their region windows,
so the port emits data, not just code. This is the first game-STATE port to do so
(the object ports were code-only; their art/mappings stayed AS-side BINCLUDE carriers).

1. **`embed()` for the BINCLUDEs.** `.emp` HAS `embed(path, skip:N, len:M)` (comptime
   file read, eval/sandbox.rs; `pub data X: [u8; N] = embed("...")` — math.emp:37
   Sine_Table precedent). object_test_state: `TestArt` (a `rept 4` dc.l pair +
   `BINCLUDE "…/test/ring_art.bin"` + `TestArt_End`) and `TestPalette` (`BINCLUDE
   "art/palettes/sonic.bin"`). PATH-ESCAPE RISK: `art/palettes/sonic.bin` is OUTSIDE
   `games/sonic4/test/` — the embed sandbox root (embed_base) must be the aeon repo
   root for `embed("../../../../art/palettes/sonic.bin")` (or an equivalent) to
   resolve; verify the build + test embed_base. This is a genuine step-1 demanded
   surface — confirm `embed` handles the ring_art.bin / sonic.bin blobs byte-exactly.
2. **`rept N { dc.l … }` unroll** → a `|> fold(asm{} , …)` comptime splice (dma_queue.emp
   fill_slot_markers / core.emp:41 precedent), OR a `pub data` array literal. TestArt's
   two 128-byte colour squares are `rept 4 { 8× dc.l }` — a `pub data TestArt_sq1:
   [u32; 32] = [$11111111; 32]` style, OR fold. Byte-exactness is the gate.

The label-bearing tables (`.emitter_positions`, `TestObjectList` with mixed
`dc.l ObjDef_* / dc.w x,y,subtype` rows referencing cross-module `ObjDef_Enemy/Solid/
Parent` + the `STUB_FLOOR_Y` const, `OJZ_SectionMarkerColors`, `PlayerMarkerTile`)
emit as `.emp` data with link-symbol + const refs — the sharpest byte-exact surface;
budget for byte-gate iteration here.

## 6. Proof machinery (the standing multi-file pattern)

- PER-FILE gates `SIGIL_EMP_OBJECT_TEST_STATE` / `SIGIL_EMP_OJZ_SCROLL_TEST`.
- ONE shared test file `crates/sigil-cli/tests/test_t1_harness_states_port.rs` (model
  on test_g4_final_objects_port.rs): windowed byte gates BOTH shapes per file +
  positive control (`t1_undoctored_compile_equals_the_reference_window`) + t24 negative
  probe (`t1_doctored_reference_diverges`) + the drift-guard counts.
- ONE mixed-build fn `build_mixed_tranche41_rom` (+ debug sibling) in mixed_dac_rom.rs —
  whole-ROM both shapes.
- The `as_constant_equs` seam carries: sst_field_equs + engine_constant_equs +
  ObjCodeBase + VRAM_TEST_OBJ/MARKER + STUB_FLOOR_Y + CAM_SCREEN_HALF_W/H + Player_1
  (shape-dependent RAM) + Camera_X/Y + Palette_Buffer + all the Prof_*/Dynamic_*/
  Effect_*/Entity_Window_Active RAM symbols + the cross-module callee labels + the
  parallax_config/Act/Sec field equs (ojz) + BGND_Palette/OJZ_Palette/
  OJZ_Act1_Descriptor (ojz).

## 7. C3 surface staged for the panel (A1+B1+C2+C3 active; C1 flagged-basis)

ojz: setVDPReg ×3 (mode2/mode3), Level_LoadArt, QueueDMA_Critical, palette CRAM copies,
the VBlank-masked marker VRAM copy (:78-95), the row-35 mode-3 block + the
command-state-machine-reset claim (:263-267), stop_z80/start_z80 bus wraps.
object_test_state: setVDPReg mode2, QueueDMA_Critical, the VDP_HV_COUNTER profiling
timing claim (:4/:92-136, DEBUG-only). C2 (highest weight) bars: ojz Debug_Scene_Freeze
skips bit-exact (oracle cache-fill hooks); the row-35 block ported verbatim (present-
tense comment naming the compensation + the ledgered parcel — the forced-spelling
site-comment rule); object_test_state as the object-churn soak vehicle — bit-exact.

## 8. Row-35 verbatim block — the present-tense comment (carry-as-is)

The ojz :234-273 mode-3 force-write ports VERBATIM. Its `.emp` comment states the
contract as a present-tense FACT (no history narration, exhibit-comment rule) AND names
the ledgered parcel per the forced-spelling site-comment rule, e.g.:

    // Force VDP mode-set-3 (shadow + reg $0B direct) every frame from the active
    // config. Re-asserts the mode because Parallax_StartTransition writes it only on
    // a section-boundary crossing (edge-triggered); its same-config short-circuit
    // would otherwise leave the register stale across a transition. Redundant while
    // every act section shares one config (the transition never fires) — correct when
    // they do not. The parallax-hardening parcel (gap-ledger t41) moves this write
    // into Parallax_Update, after which this block is deleted (kill row 35).
