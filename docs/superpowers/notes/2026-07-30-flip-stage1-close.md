# 2026-07-30 — FLIP STAGE 1 · CLOSE CHECKPOINT (scope amended on the record)

Status: **Stage 1 closes with its scope AMENDED per the overseer's Option-1 ruling.**
The canonical sonic4 native proof is complete; the six-target asl-derived golden
freeze is committed while asl is live; the demo + Config-A/B NATIVE proofs are
RE-SEQUENCED into Stage 2 (they ⊇ S1.2, the computed-resume-org work). No aeon edit.
Merge is the overseer's. Sigil `flip-stage1` tip `69bbad5`; aeon read-only `bcb8f64`
(zero aeon commits). Strict **2914/0 (1 ignored)**.

## THE SCOPE AMENDMENT (why Stage 1's exit bar changed)

The design's §6.1 listed `native_rom_{config_a,config_b,demo_plain,demo_debug}` as
Stage-1 gates. This session PROVED they are not buildable against the frozen tree:
every `SIGIL_EMP_*` gate's else-arm RESUME ORG in `engine.inc`/`main.asm` is a
hardcoded CANONICAL sonic4 (sound-on) address (each with the in-tree NOTE "never set
for other games"). Any non-canonical layout resumes at the wrong address:
- **demo / Config-B (sound-off):** the resident Z80 driver is absent → the whole
  engine layout shifts (demo `BootData` 0x3A2 vs resume 0x3A8; Config-B `GameLoop`
  0xB48 vs 0x239A, `BusError` 0x42420 vs 0x5CAB0).
- **Config-A (hotkeys+mirror):** `game_debug`/`sound_debug` go non-empty → the $6xxx
  engine regions shift +0xCC (`Section_Init` 0x6408 vs 0x633C).

The fix is S1.2 (OQ-6: compute resume orgs as link outputs), already DEFERRED to
Stage 2. **Overseer ruling (Option 1): re-sequence, do NOT parameterize ~30 sites per
game×config — that is pin-tax built into a corpse (kill-row-6 doctrine: the
resume-org scaffolding dies at the Stage-2 gate-deletion).** Evidence:
`2026-07-30-flip-stage1-demo-config-native-blocked.md`.

## THE FULL PROOF MATRIX (Stage-1 exit state)

Two layers per target: ASSEMBLED ANCHOR `[0,EndOfRom)` header-neutral (the drift-
stable bar the native driver reproduces) and FULL FILE (assembled + deb2 appendix).

| target | native == asl (assembled) | golden frozen (both layers) | native == golden |
|---|---|---|---|
| sonic4 plain | ✅ S1.1 `native_rom_plain` + S1.4 prefix == e5765873 | ✅ `s4.bin` full eff2396f/413577 · anchor e5765873 | ✅ S1.4 `native_full_sonic4_plain` (sigil-canonical full 2198deb2/395374) |
| sonic4 debug | ✅ S1.1 `native_rom_debug` + S1.4 prefix == dab4f06c | ✅ `s4.debug.bin` full 1e9097bc/421579 · anchor dab4f06c | ✅ S1.4 `native_full_sonic4_debug` (1d895fcb/402696) |
| demo plain | ⛔ Stage-2 (⊇ S1.2) | ✅ `demo.bin` full 18c64002/90776 · anchor cfda98d3 | ⛔ Stage-2 |
| demo debug | ⛔ Stage-2 (⊇ S1.2) | ✅ `demo.debug.bin` full b0475a59/91584 · anchor 20c5571d | ⛔ Stage-2 |
| Config-A | ⛔ Stage-2 (⊇ S1.2); live witness = `mixed_offcanonical` combined gate | ✅ `config_a.bin` full b4a6756d/421898 · anchor 3d9bac53 | ⛔ Stage-2 |
| Config-B | ⛔ Stage-2 (⊇ S1.2); live witness = `mixed_offcanonical` z80_init gate | ✅ `config_b.bin` full 92776720/304961 · anchor fd3f7f8e | ⛔ Stage-2 |

Anchor computation VALIDATED: it reproduces the campaign PRIMARY e5765873/dab4f06c
exactly for the two sonic4 shapes, so the demo/Config anchors are trustworthy.

## CONSOLIDATED STAGE-2 HANDOFF

**THE SEQUENCE AMENDMENT (new — the load-bearing change).** Stage 2 opens with the
S1.2 companion work — gate-scaffolding deletion + residual section-split + pins→map /
COMPUTED placement (this is exactly what makes demo/Config native buildable) — then
proves ALL SIX targets `native == frozen golden` (assembled anchor + full-file
layer), and ONLY THEN, in the same gated parcel, flips `build.sh` and deletes the
twins. Point-of-no-return protection intact: nothing deletes until every
native-vs-golden proof is green. Demo flips in the SAME Stage-2 parcel, gated
identically (Volence lockstep ruling). The six committed goldens are THE comparands.

**Previously-ledgered Stage-2 items (carried forward):**
1. The 4 inapplicable drift guards + the allowlist machinery
   (`STAGE1_INAPPLICABLE_GUARDS` / `enforce_inapplicable_allowlist` + t24 tests)
   retire WITH their twins — VRAM_PLANE_B_BYTES ← `engine/level/bg.asm`,
   CAM_SCREEN_HALF_{W,H} ← `engine/level/camera.asm`.
2. `objdef.emp` import-id one-line fix: `use engine.constants.{RF_PRIORITY_SHIFT}`
   (currently the id `engine.constants` vs import `engine.system.constants`); drop the
   `HELPER_ALIAS_DROP` alias in `native.rs` when it lands.
3. Keystone `.emp` byte-sections (player_common / test_player / test_enemy): at Stage
   2 the `.asm` twins delete and the native driver FLIPS TO OWNING those sections
   (remove from `AS_OWNED_KEYSTONES`, add to the registry with PLAYER_COMMON /
   TEST_PLAYER / TEST_ENEMY pins, gate ON).
4. S1.2 companion (map-region growth): the object-code-bank region + section ordering
   DECLARED; every per-shape / off-canonical resume org COMPUTED (kills the row-6/58
   pin-tax). This is the PREREQUISITE that unblocks demo + Config-A/B native.
5. Row-34 repin dependency; the split-golden model statement (Option A); the
   demangler + content-audit ledger rows (the convsym `$`-name drop → ~1394 lift).

**NEW this session (the resume-org evidence → the amendment):** demo-native and
Config-A/B-native each ⊇ S1.2. They cannot be independent Stage-1 gates; they land
with the S1.2 gate-deletion parcel, comparanded to the six frozen goldens above.

## THE VALVE / CONSTRAINTS (honored)

Additive only — asl default path byte-identical (canonical s4.bin/s4.debug.bin
rebuilt + verified restored to eff2396f/1e9097bc after the Config-A/B capture
clobber). Zero aeon commits. Strict green at each commit boundary (2914/0/1). t24
discipline intact (no new gate machinery added — the freeze is data + a fresh-build
script). STOP honored on the design fork rather than papering it with an aeon edit.

Commits this session (on `flip-stage1`, over `ba39390`):
- `3773bfd` — the demo+Config native-blocked finding (the STOP).
- `014c55e` — golden-freeze infra (script + provenance), superseded by ↓.
- `69bbad5` — the SIX-target asl-derived golden freeze (blobs + extended script +
  amended provenance) — the Stage-2 comparands.
