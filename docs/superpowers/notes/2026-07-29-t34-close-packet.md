# 2026-07-29 — t34 close packet (game-side P1: the PLAYER KEYSTONE — player_common + sonic)

Porters: Opus subagents (Fable-dispatched, direct) — a **two-porter tranche**. The
first porter landed step 1 (byte-exact, gate-green) and **context-valve-stopped with a
fully-scoped handoff**: the authorized step-2 wave, the loop, and the panel. The
continuation porter ran the wave + loop + dry-panel to checkpoint (b). Both checkpoints
overseer-countersigned (own dual rebuild + own paired strict).

Brief: `2026-07-29-t34-p1-player-keystone-brief.md`. Design: `2026-07-29-t34-step0-design.md`.

## Outcome

The census's P1 tranche — the **HARD ORDERING ROOT of the player cluster** — ported.
`player_common.emp` (the keystone: PlayerV overlay, `_pl_*`/`PPHYS_*` equates, 4 macro
templates, the frame skeleton + state machine + hooks) and `sonic.emp` (character
data/asset wiring). This is the **SECOND canonical change since t24** (t31 was the first);
the step-2 wave is a **byte-moving game-side wave** carrying the full t31 discipline.

**New canonical baseline:** plain **ee4de2ed/421041** · debug **b5f76eee/429102** (sizes
UNCHANGED — the object-bank shrink is pad-absorbed). Strict **2796/0 (3 ignored — 1
baseline + the 2 `#[ignore]`d `mixed_tranche34` whole-ROM tests, the known combined-link
PcRel8 bug on its own finisher branch)** — unchanged from the pre-wave count.

Branch tips at close (pre-merge): aeon **`472dce4`**, sigil **`289f636`** (+ this packet /
kill rows / ledger ruling fold into the sigil close commit).

## Headline: the extern-count asterisk

The design's "**ZERO externs**" is a claim about CALL escapes — every callee resolves by
**bare link** (module-to-module in the mixed build, link-symbol when windowed): engine
`.emp` procs (AnimateSprite/Draw_Sprite/Perform_DPLC/Sound_PlaySFX), surviving-AS procs
(Player_SensorFloor, the PState_* handlers, Sonic_InitAssets/LoadArt), and ROM/RAM data.
No `jsr` goes through an `extern()` escape. The asterisk (the `extern()`/`extern proc`
usages that DO exist, all non-call):
- **1 `extern proc` boundary decl** — `Player_AtLedgeEdge` (the surviving player_sensors
  callee, so its contract `clobbers(d0-d5/a1-a2)` is locally visible; **killed at P4**).
- **21 `extern("PState_X"|"PHook_X") - extern("Table")`** across the 3 offset tables — the
  t31 cross-seam-DATA form (`[here.provisional]` blocks bare-local-minus-extern in DATA);
  the P4-close `offsets`-adoption candidate (ledgered, ruled below).
- **~49 drift-guard `ensure(extern("NAME") == NAME)`** — the game-config / engine-constant
  mirrors (the row-54/row-1 mirror class).

## The wave (byte-moving, full t31 discipline)

| item | change | Δ |
|---|---|---|
| 1 — branch modernization | player_common.emp/sonic.emp → bare Bcc + `jbsr`/`jbra`; computed `jsr/jmp (a1,d1.w) as PlayerState/PlayerHook` unchanged; cross-seam `bsr.w Sonic_InitAssets` relaxes byte-neutral. **4 AS-twin shrinks in lockstep**: RefreshPhysics/Animate `bsr.w→.s` (adjacent), Sonic_LoadArt/Player_SetState `jmp→bra.w` (same-bank in-range). | **−0x8** at PLAYER_COMMON |
| 2 — the dead ext.l cut | `distToFix` (always-emitted header) drops the leading `ext.l`; `dist_to_fix` becomes a bare `pixels_to_coord(v)` splice. | **−0x2** player_common (×1) + **−0x8** player_air (×4, surviving AS) |

**Per-region delta:** PLAYER_COMMON −0xA · player_ground −0xA (slide) · player_air −0x12
(−0xA slide + −0x8 internal) · player_spindash → sonic → test_* → objdefs → act_descriptor
→ sonic_anims → particle_anims all **−0x12** (cumulative slide, pad-absorbed before the
sound bank; ERROR_HANDLER + ROM tail unmoved). `sonic.emp`'s `jsr Perform_DPLC→jbsr` /
`jmp Draw_Sprite→jbra` are byte-neutral (far cross-module abs.w).

**$8000 bar: SATISFIED (N/A direction).** A SHRINK, not growth; the object-bank plain==debug
bases still coincide (TEST_STATIC 10C54/10C54, TEST_PARENT 1101E/1101E, P_STATE_GROUND
10448/10448…) — no `jsr (Sym).w`→abs.l widening, no bank slide between shapes.

## The wave-ripple census (the reusable checklist instance — future game waves inherit)

A game-side object-bank shrink at the KEYSTONE slides the whole object-bank + game-data run
by the shrink amount (here −0x12 from player_common down through particle_anims). What the
−0x12 slide touched, HAND-verified (suite-green is the completeness proof):

1. **repin → pins.rs: 73 pins regenerated** (object-bank regions/labels + game-data regions:
   OBJDEFS/ACT_DESCRIPTOR/SONIC_ANIMS/PARTICLE_ANIMS + every OJZ_SEC*_{BLOCKS,OBJECTS,RINGS,
   TYPE_TABLE} + MAP_*/DPLC_SONIC/ART_SONIC/BG_ANIM_TABLE/OJZ_* + the P_STATE_* pins). Δ −0xA
   for player_ground-interior pins (before the player_air ext.l point), −0x12 for everything
   after.
2. **main.asm gate-resume orgs ×14** — sonic ($10C54) · test_static ($10C58) · test_animated
   ($10CB2) · test_objects ($10FCA) · test_emitter ($1101E) · test_parent ($1113E) ·
   test_stress_emitter ($11198) · test_churn ($11210) · objdefs (plain $11D94 / debug $11DFC) ·
   sonic_anims (plain $25760 / debug $257C8) · particle_anims (plain $25768 / debug $257D0).
3. **player_common.asm else-org** ($10452 → $10448) — the internal gate's resume.
4. **act_descriptor.asm SELF-GATE orgs ×2** (plain $14DC6→$14DB4 / debug $14E2E→$14E1C) — the
   NON-obvious one: a fixed self-gate `org` that RE-DID the slide for everything after it until
   updated. This is the arm that initially broke tranche4-9 mixed builds; it lives in the data
   file, NOT main.asm. **Checklist lesson: sweep every `org $` in ALL game `.asm`, not just
   main.asm.**
5. **5-site doctrine hand-edits:** engine.inc `org $10000` UNAFFECTED (no engine region slid) ·
   repin_pins.rs CLEAN · mixed_dac_rom.rs **7 hardcoded reference-ROM read windows** (test_solid
   0x10F7C→0x10F6A, test_particle 0x10F8A→0x10F78 ×2 each shape, sonic_anims 0x25704→0x256F2,
   particle_anims 0x25772→0x25760, act_descriptor 0x14B52→0x14B40) **+ the test_solid objroutine
   LITERAL `$F86→$F74`** (TestSolid_Main−ObjCodeBase is a self-relative word that slid) ·
   test_objects_port.rs 2 doc-comment addresses.
6. **Canary spot-check fix:** `mixed_tranche29`'s objroutine-word assertion checked `TestAnimated`
   (base) — a **coincidental pre-slide byte match**; the real stored word is `TestAnimated_Main`
   (the `code_addr` objroutine). Corrected to `TestAnimated_Main` (matching the sibling
   `TestStatic_Main` check) — a latent spurious-pass the slide exposed.
7. **Canaries green post-wave (by name):** g1/g2/g3 (windowed, both shapes) · mixed_tranche29/30/31
   (both shapes) · test_p1 8/8 (both shapes).

## Loop findings (0→1 first porter; 2→(3→4→5)→panel continuation porter; dry by pass 2)

**Step 2 (item 1)** + **step 5 (item 2)** = the wave. **Step 3(b) contract-comment corrections**
(byte-neutral, both twins): Player_Init `d0-d1→d0-d2` (d2 via Player_SetState's hook dispatch),
Player_Display `d0-d4→d0-d5` (d5 via Player_Animate's balance-path ledge probe), Player_Animate
`d5-d6→d5` (Player_AtLedgeEdge clobbers d0-d5 not d6, verified against player_sensors.asm),
Player_Main `.asm` header aligned to the `.emp` d7-clobber rationale (d7's HIGH word IS clobbered;
only the RunObjects low word is round-tripped). **Step 4** — no construct built (the `offsets`
adoption is ruled below). **C1 live-question (named sites):** Player_Main irreducible per-frame
work; Player_Animate's up-front d3 speed-hold is a ~5cyc/frame hoist candidate → **sub-threshold,
logged-and-skipped**; the macros are minimal.

## Panel adjudication (A1 + B1 + C2; C3 inactive — no VDP/DMA; C1 answered above)

- **C2 (correctness — the mandatory ext.l re-derivation): ext.l DEAD, CONFIRMED both axes, 5
  sites → item 2 STANDS.** Data: `swap`+`clr.w` put the signed `.w` into the high word for both
  signs regardless of the incoming high word (which lands in the low word and is cleared —
  `ext.l` only made the LOW word deterministic, exactly the word `clr.w` destroys). CC: `clr.w`
  was already the LAST flag-setter in both old and new forms, so the post-macro CCR is identical;
  no site reads the macro CC before its own flag-set (SnapToSurface→`move.b`; air 229/464→`sub.l`
  then `tst.w`+`bpl`; air 427/444→`add.l`/`sub.l`+`clr.w`). Branch shrinks + clobbers spot-checks
  sound.
- **B1 (corpus): ONE step-4 ADOPT candidate → OVERSEER-RULED LEDGERED.** The 3 player-state offset
  tables re-hand-roll the shipped `offsets` construct; adoption is byte-preserving (same
  RelWord16Be fixup) and yields `.count`+range-check. **RULING (Fable, 2026-07-29): NOT TAKEN —**
  (1) the cross-seam `Ref`-to-AS-extern path is UNEXERCISED → would debut untested machinery on
  the campaign's most depended-on new file; (2) it overrides design-note §7's deliberate choice,
  and a same-day-pre-ruling override needs stronger evidence than byte-equivalence; (3) the natural
  moment is **P4's close** — when player_sensors ports, the PState_* targets become internal and
  the tables become pure-internal offset tables (the construct's SIMPLE path). **KILL CONDITION:
  adopt at the P4 close.** Branch idioms + `dist_to_fix→pixels_to_coord` delegation consistent
  with corpus.
- **A1 (cold-reader): two comment-claim defects FIXED (byte-neutral).** (1) Player_Animate "mutates
  only skid_latch" was false (also decrements the persistent `getup_timer`); (2) Player_Main `.asm`
  "d7 preserved" contradicted the `.emp` (see step-3(b)). Mirror-with-drift-guard DSL (~45
  `const`+`ensure` pairs) = the pre-existing standing gap-ledger item, unchanged.
- **C2 note (pre-existing, ledgered):** `Sound_PlaySFX` declares `preserves(d1/a0)` with no explicit
  `clobbers()` — Player_Animate's contract around the DEBUG-gated skid-SFX call isn't locally
  verifiable. NOT a t34 regression.

## Kill-list rows (added same-merge — rows 72-77)

72 (player_common internal gate + gate-off code twin + org arm) · 73 (sonic whole-file gate +
org arm) · 74 (PlayerV 13-field guarded overlay + 5 drift guards, row-61 class) · 75 (the 4
macro→comptime templates: set_standing_size/set_ball_size/mask_opposing_lr/dist_to_fix; dist_to_fix
adopts pixels_to_coord row 49, the ext.l DIED in the wave) · 76 (player_common game-config +
engine-constant mirrors, ~40 pairs) · 77 (sonic game-config + physics mirrors, 9 pairs). Rows
72/74/75 carry the **P4-close kill condition** (the surviving-AS readers/expanders vanish → the
overlay/templates/header become `.emp`-owned). No NEW scaffolding from the wave (it modernized
existing twins).

## Census STATUS AMENDMENT (fold into 2026-07-29-game-side-census.md)

**P1 PORTED** (player_common.emp + sonic.emp; byte-moving wave −0xA/−0x12, new canonical
ee4de2ed/b5f76eee). **P2/P3/P4 UNBLOCKED for dispatch PENDING the combined-link PcRel8 finisher**
— every P2-P4 player file has duplicate local-label names (the `.keep`/`.abs`/`.draw` class), so
the player-cluster whole-ROM gate stays blocked until the link fix lands (its own branch, merges
behind t34); the P2-P4 tranches ride windowed byte gates + gate-off dual-build identity meanwhile
(as strong as t29). t34's merge proof is exactly that pair (the 2 `mixed_tranche34` whole-ROM tests
`#[ignore]`d with the inline reason). The oracle consumer count and the P4-close `offsets`-adoption
kill are the standing forward hooks.
