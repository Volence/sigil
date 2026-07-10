# Twin-scaffolding kill list (living doc)

Volence's standing ask (2026-07-10): every twin-related TEMPORARY mirror —
a value declared on both sides of the .emp/AS seam with a drift guard
holding the copies together — gets a row HERE with its kill condition.
Nothing on this list is a bug; everything on it is scheduled demolition
the campaign must not forget.

**The rule:** a mirror exists only while the value's OWNER is still on the
AS side. Each kill is an ownership flip (the .emp side becomes the only
definition, exported as an equ to remaining AS readers — the reverse seam)
or a consolidation (per-file mirror moves into the shared constants twin).
When a row dies, delete its mirror consts AND its drift guards together.

**Cadence:** every port that adds a mirror adds a row; every checkpoint
packet reviews the list; the campaign-end sweep closes whatever survives.

## Live mirrors

| # | Mirror | Where | Guards | Kill condition |
|---|--------|-------|--------|----------------|
| 1 | `engine.constants` twin — `HW_PORT_1_DATA`, `HW_PORT_2_DATA`, `BUTTON_UP/DOWN/LEFT/RIGHT`, `CTYPE_AIR`, `VDP_Shadow_len` | `engine/system/constants.emp` mirroring `engine/constants.asm` (+ the struct-generated `VDP_Shadow_len`) | 8 ensures, riding every twin consumer's gate | `constants.asm` ports → ownership flip (twin becomes THE file, AS readers take exported equs). `VDP_Shadow_len` dies earlier if the `VDP_Shadow` struct ports with `vars`. |
| 2 | `AF_DELETE` local const | `games/sonic4/data/animations/particle_anims.emp` (truth: `engine/objects/animate.asm`) | 1 ensure | Near: consolidate into the constants twin's animation block when a 3rd consumer appears (ledgered). Final: `animate.asm` ports → flip. |
| 3 | `AF_END`, `AF_BACK`, `DUR_DYNAMIC` local consts | `games/sonic4/data/animations/sonic_anims.emp` (truth: `animate.asm` + `engine/constants.asm`) | 3 ensures | Same as row 2 — same block, same flip. |
| 4 | `ANIM_*` ordinal guards ×12 | `sonic_anims.emp` checking `games/sonic4/config/constants.asm` | 12 ensures | TWO-STAGE (Volence-ratified 2026-07-10): **[1 DONE]** the flip mechanics proven (`reverse_seam_ordinals.rs` — .emp `equ` exports of ordinals, AS consumes through the link; D2.34); **[2 at Spec 5]** delete the config block + swap player code to the exports — the gate-off shape can't read .emp exports until the dual build dies, and the imm deferral needs `.b`/`.w` widths first (ledgered). |
| 5 | The AS twins themselves — `collision_lookup.asm`, `vdp_init.asm`, `hblank.asm`, `controllers.asm`, `math.asm`, `sonic_anims.asm`, `particle_anims.asm`, `dac_samples.asm`, `mt_bank` block, `sfx` block | every ported file's gate-off body, kept in LOCKSTEP with its .emp (the step-5 mechanics) | the byte gates ARE the guard | **Spec 5** (AS front-end deletion): each `ifndef SIGIL_EMP_*` gate collapses to the .emp include, the .asm twin is deleted. Until then every byte-changing edit pays double (edit both, re-pin). |
| 6 | Per-shape gate pins — the `org` resume addresses in `engine.inc`/`main.asm` + the region bases/sizes/windows in the sigil harness | 6 engine pairs + 2 game-data pairs (and growing) | strict gates fail loud on drift | Same as row 5 — the pins exist only while the dual build exists. Every reference re-baseline re-derives them (the re-pin tax). |
| 7 | `Act`/`Sec` struct twins | `act_descriptor.emp` (truth: `engine/structs.asm`), layout-pinned via `sizeof() == extern("Act_len"/"Sec_len")` | 2 ensures | `structs.asm` ports (the vars-era struct story) → ownership flip; or consolidate into a shared game structs module when act 2 arrives. |
| 8 | `MAX_ACT_SECTIONS`, `SECTION_SIZE_SHIFT`, `EDGE_CLAMP` mirrors | `act_descriptor.emp` (truth: engine constants; mirrored so the grid/clamp checks run at COMPTIME, and EDGE_CLAMP because a link-folded extern can't range-prove into a u8 field) | 3 ensures | owning engine files port → flip; or the constants twin grows a level block (same consolidation class as rows 2/3). |
| 9 | sonic4's `gameDebugTick` EXPANSION (`jsr Debug_MusicToggle` under `SOUND_DEBUG_HOTKEYS && SOUND_DRIVER_ENABLED`) | `engine/system/game_loop.emp` mirroring `games/sonic4/config/game.asm`'s macro body (tranche-5 H2 ruling — the game-contract macro seam) | `game_loop_port.rs`'s combo matrix re-extracts the macro body from the REAL game.asm every run and byte-diffs all four define combos — drift fails the matrix; LOCKSTEP comment at the macro | a ratified .emp game-contract-hook mechanism (extern-macro or link-time hook — Spec-5 neighborhood, same era as row 4 stage 2), or GameLoop migrating game-side. First game-contract macro with a NON-TRIVIAL body to port is the construct's demand moment (ledgered). |
| 10 | sound_api immediate mirrors — `SND_ALIVE_MARKER`, `SND_MUSIC_STOP`, `SND_FADE_CMD_OUT/IN`, `SFX_RING_MASK`, `SFXID_RING_RIGHT/LEFT` | `engine/sound/sound_api.emp` (truth: `engine/sound_constants.asm` + `games/sonic4/config/sound_ids.asm`) — immediate-POSITION only; the slot ADDRESSES stay AS-owned as extern-equ sums (nothing to drift there) | 7 ensures, riding the thirteen-module gates | `.b`/`.w` imm-link deferral lands (the ledgered width extension) → the immediates read through extern like the addresses do; or the owning constants files port → flip. Same consolidation class as rows 2/3/8. |

## NOT on this list (deliberately)

- **Co-residency ensures** (mt_bank ×5, sfx_bank ×1): they check PLACEMENT
  facts across the seam, not duplicated values — they survive the campaign
  as permanent invariants (possibly re-expressed once both sides are .emp).
- **Module-local consts with no AS counterpart** (`VDP_REG_CMD`/`VDP_REG_STEP`
  in vdp_init.emp): single-definition, nothing to kill.
