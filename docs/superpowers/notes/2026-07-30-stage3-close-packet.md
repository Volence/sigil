# 2026-07-30/31 — STAGE 3 CLOSE PACKET (P5/P6 executor span)

Status: **Checkpoint report for the overseer's merge.** Branch pair `stage3-p5`
(off masters aeon `c8f2948` / sigil `6aea17b`). The OQ-5 spike, the P5 constants
ownership flip, and the bg/camera Poison-class flip (row 93 data-half) are DONE,
byte-neutral, strict green. The structs flip, the P6 game-constants module, and a
few appendix items are LEDGERED with the now-proven mechanism as their path. The
merge is the overseer's.

## §0 — THE HEADLINE

The OQ-5 spike found the export mechanism MISSING (link-deferral serves only
link-position readers; residual AS has comptime readers of the flipped constants).
The overseer authorized **Option A** — harvest the `.emp`-owned constant values,
inject as GUARDED AS `-D` defines. That mechanism is now built, proven, and used to
flip **117 engine constants** (114 in the constants block + the 3 bg/camera Poison
constants) from AS-authored to `engine.constants`-owned — **byte-neutral to all six
golden ROMs and all six assembled anchors.**

---

## §1 — WHAT THIS SPAN DID (commits, both repos)

### The mechanism (sigil)
- `sigil-frontend-emp::eval::eval_all_pub_consts` — harvests every `pub const` in a
  module to its resolved `i64` (the compile path's `resolve_const`, so derived
  consts read resolved siblings).
- `sigil-frontend-as` `AsOptions.guarded_defines` — a SEPARATE define channel from
  the ordinary defines (which keep asl's silent-override for code gates / config
  overrides that legitimately coexist with in-file `=`). A guarded name seeds the
  env like a define, but an in-file `=`/`equ` of it is a hard `[defines.collision]`
  (the no-silent-shadowing guard — **bar 2**). `attach_guarded_equ_exports` exports
  each guarded define as a link `EquSym` so BARE-LINK-SYMBOL `.emp` consumers
  (boot/controllers `move.b #$40, HW_PORT_1_DATA`) resolve exactly as the deleted
  `.asm` `=` did — byte-neutral (equates are filtered from the deb2 appendix,
  verified: the 114 names appear in neither `.lst` nor `s4.bin`).
- `sigil-harness::native::harvest_engine_constants` + wiring in `assemble_as_side`:
  harvest `engine.constants` FIRST (excluding struct-generated `VDP_Shadow_len`),
  seed as guarded defines BEFORE the residual AS assembles — **the ordering the flip
  makes real** (`.emp` defs flow INTO the AS assembly), stated as a commented step
  (**bar 4**).

### Proofs (bar 1 + bar 2)
- `guarded_defines.rs` — a guarded `-D` value folds BYTE-IDENTICAL to an in-file `=`
  across every consuming position (`ds` count / `if` / shifted `dc.b` / derived
  equate / `dc.w`); the collision probe (in-file redefinition fails loud); ordinary
  defines keep silent-override.
- `harvest_pub_consts.rs` — harvest incl. derived + private-exclusion.
- `p5_constants_flip.rs` — the t24 negative probe on the REAL harvest: reintroducing
  an in-file `=` for a flipped constant fails `[defines.collision]`; undoctored reads
  it clean (the positive control).

### The aeon flip
- `engine/constants.asm`: 114 `=` definitions deleted (the emp-owned block).
- `engine/system/constants.emp`: the sole author now; its 114 mirror `ensure`s
  deleted (VDP_Shadow_len's KEPT — struct-generated twin). +3 bg/camera constants
  (`VRAM_PLANE_B_BYTES`, `CAM_SCREEN_HALF_W/H`).
- Consumers converted from local-mirror+extern-guard to `use engine.constants`:
  `game_debug.emp` (BUTTON_UP/C/A), `act_descriptor.emp` (SECTION_SIZE_SHIFT/
  EDGE_CLAMP), `plane_buffer.emp`/`section.emp` (VRAM_PLANE_B_BYTES), `bg.emp`/
  `camera.emp` (the moved constants), `ojz_scroll_test.emp` (CAM_SCREEN_HALF_*).
- `STAGE1_INAPPLICABLE_GUARDS` + `DEMO_INAPPLICABLE_GUARDS` emptied — the 4 Poison
  guards retired; the enforcement stays as the STRENGTHENED no-Poison invariant.

### The drift-guard-retirement test ripple (test-only, byte-neutral)
114 constants.emp drift guards + the consumer guards deleted → `test_support::
engine_constant_equs` reduced to the one surviving twin (VDP_Shadow_len); the ~13
central count assertions follow it; the doctored negative-probes for the retired
guards deleted (co-located structs/sst-wall probes KEPT); controllers/act/vector
tests supply the specific extern targets they need (HW_PORT_1_DATA; VDP_Shadow_len;
the vector-table standalone seeds `guarded_defines` from the harvest).

---

## §2 — ROW CLOSURES

| rows | what | status |
|---|---|---|
| **1, 2, 12, 14, 17, 19, 20** | the `engine.constants` twin blocks (HW_PORT/BUTTON/CTYPE/RF/AF/collision/object-core/sprite/section/tile-cache/DMA/art/VRAM/physics/screen) | **CLOSED** — engine.constants is the sole author; residual AS reads via the harvest→inject mechanism; the 114 drift guards retired. |
| **93 (data half)** | the bg/camera Poison class + `STAGE1_INAPPLICABLE_GUARDS` | **CLOSED** — the 4 Poison guards retired; the allowlist empty. |
| **7, 8, 11, 15, 25** | `Act`/`Sec`/`Sst`/`EntityScanState` struct twins + `interact_off()` | **OPEN — the structs flip (see §4).** VDP_Shadow_len's constants.emp guard is deliberately kept as the struct-twin bridge until this lands. |
| **54, 62, 65, 65b** | game-config const surface (buttons / song+sfx ids / VRAM_TEST_*) | **OPEN — P6 (see §4).** game_debug's BUTTON_UP/C/A already collapsed into `use engine.constants` this span (partial row-54 progress). |

---

## §3 — THE CURRENT-STATE LEDGER

- **Masters at branch open:** aeon `c8f2948` / sigil `6aea17b`.
- **Six goldens — UNMOVED (byte-neutral bar held every commit):**
  s4 `7f071417`/412306 · s4d `0b8efc7a`/422147 · demo `705a5871`/90436 · demod
  `37ded207`/92935 · ca `1b4c49d2`/422483 · cb `bfe2509e`/303660.
- **Six assembled anchors — UNMOVED (the never-moves invariant):**
  `e5765873` · `dab4f06c` · `cfda98d3` · `20c5571d` · `3d9bac53` · `fd3f7f8e`.
- **Strict:** 2856 passed / 0 failed / 1 ignored (baseline 2861/0/1; the arithmetic:
  +11 new tests from the mechanism proofs, −16 net from the drift-guard-retirement
  surgery — the exact number is an output, reported here).
- **`stage3-p5` commits:** sigil `0915391`(spike) → `fd446f3`(plumbing) →
  `df129cb`(wire+tests) → `017c2c2`(allowlist); aeon `f7154b7`(flip) →
  `186d560`(bg/camera). The spike note, this packet, and the OQ-5 spike note live in
  `docs/superpowers/notes/`.

---

## §4 — WHAT REMAINS LEDGERED (with the mechanism as the path)

### The structs ownership flip (rows 7/8/11/15/25) — the P5 second parcel
`engine/structs.asm` generates the `SST_*`/`Act_*`/`Sec_*`/`DMAEntry_*`/
`parallax_config_*`/`EntityScanState_*` field-offset equs (via `struct … endstruct`)
+ `VDP_Shadow_len`. The `.emp` overlays (`sst.emp`, `act_descriptor.emp`,
`entity_window.emp`, `engine.structs`) mirror them with per-field `@`-asserted
drift guards. The flip: the `.emp` structs become the sole author; residual AS
takes the offsets via the SAME harvest→inject mechanism — but the harvest is
STRUCT-OFFSET-shaped, not `pub const`-shaped, so it needs a sibling harvester
(`eval_all_struct_field_equs` or equivalent, reading `offsetof`/`sizeof` from the
`.emp` struct decls) and the injection excludes nothing (unlike VDP_Shadow_len,
which is exactly the bridge that retires HERE). Then `test_support::sst_field_equs`
+ `act_sec_field_equs` reduce like `engine_constant_equs` did, and the per-field
drift walls retire. **Scope:** ~1 harvester + `structs.asm` deletion + the struct
overlays' guard deletion + the same port-test count ripple. **Byte-neutral**
(offsets unchanged). This is the natural next parcel; the constants flip is its
exact template.

### P6 — the game-constants `.emp` module (rows 54/62/65, untyped half)
Born to absorb `SONG_*`/`SFXID_*` numeric ids, `VRAM_TEST_*`, `BUTTON_B/START` from
`config/constants.asm` + `config/sound_ids.asm`. **Census finding (this span):**
these fold mostly into IMMEDIATES / `dc` data (link-position), which the PROVEN
reverse-seam already serves — so P6's untyped half may NOT even need the guarded-
define comptime channel; a `games/sonic4/config` `.emp` module exporting the
untyped ids + the consumers reading them through the link is likely sufficient.
Confirm with a per-id position census of `config/*.asm` before scoping. The 2 TYPED
`SFXID_RING_*` mirrors STAY DEFERRED to the language round (typed-extern grammar) —
ledgered, not forced. `BUTTON_UP/C/A` already collapsed into `use engine.constants`
this span; `BUTTON_B/START` await this module.

### ram.asm / sound_constants.asm — CENSUS: DEFER (both), with reasons
- **`engine/ram.asm`** — 1 `=`, 201 `ds` (a RAM LAYOUT file, not a constants file).
  Its `ds`-reserved RAM labels are link symbols the `.emp` reads via `extern()`; it
  reads engine constants at comptime, now served by the injected guarded defines.
  **DEFER:** flipping it is a `vars`-layout port (201 slots), NOT a constants-
  ownership flip — out of P5's scope; the mechanism already serves its comptime
  reads with zero mirrors to retire.
- **`engine/sound_constants.asm`** — 321 defs, read by 5 sound `.emp` modules.
  **DEFER:** a sound-domain constants parcel (row 59), its own flip using the proven
  mechanism (harvest a `sound_constants.emp` + inject). Out of P5's engine-constants
  scope; scoped as a dedicated sound parcel.

### Appendix polish (trivial follow-ups)
- **Full `STAGE1_INAPPLICABLE_GUARDS` machinery deletion:** the allowlist is empty
  and the enforcement is now the no-Poison invariant. Deleting the `GameProfile.
  inapplicable_guards` field + the `enforce_inapplicable_allowlist*` fns + the
  partition call is trivial (the empty list makes the partition always-empty). Kept
  as the strengthened invariant; delete outright if the overseer prefers.
- **The VDP_Shadow_len bridge:** its constants.emp guard + the `test_support`
  single-entry `engine_constant_equs` retire WITH the structs flip (it is the one
  struct-generated constant, deliberately not injected to avoid colliding with the
  struct symbol).

### Language-round deps (unchanged, not P5/P6 items)
Typed externs (row 10 tail: `SFXID_RING_*`); rows 4/9/45 (`.b`/`.w` imm-link +
game-contract hook); rows 21/53 (the `.emp`-native diagnostics runtime, needs the
link-time-equ-off-external-base capability). Each a named language dep.

---

## §5 — THE POST-FLIP ARC HANDOFF (§17 sweep inputs)

- **The master step-5 optimization backlog** (`campaign-gap-ledger.md:1092`, the
  2026-07-16 emp-port review): tile_cache #1/#2, plane_buffer #1–4, collision_lookup
  #1–3, sprites H1–H3, rings R2/R3, animate A2/A3, entity_window High #1, core #1 —
  each now runs at ~HALF cost (one source file, no twin lockstep, no re-pin), the
  cheap sweep the flip unlocks.
- **G9** — `player_ground` `d7` high-word clear (byte-changing → oracle A/B).
- **The parallax-hardening parcel (row 35)** — the engine mode-3 write from
  `Parallax_Active_Config` + the harness force-write deletion, with the two named
  A/B checks (no render regression + no perturbation of the `Debug_Scene_Freeze`
  cache-fill soak).
- **Oracle A/B protocol pointers:** anchor A/B to `Frame_Counter`; the ObjectTest
  soak scene via `Game_Entry` flip; `Debug_Scene_Freeze`(0xFF8A10)+Camera poke for
  deterministic tile-cache fills; the trailing-lag indicator (the beam-deadline gate
  is dead — `Tile_Cache_Fill` runs in VBlank).

---

## §6 — DISCIPLINE HELD

Byte-neutral throughout (six goldens + six anchors unmoved, re-proven every commit
via the scratch six-target build — never `build.sh` for CRC). Census before every
flip (the readers listed in each commit). The row-91 witness untouched. t24 on the
new mechanism (the collision probe + positive control) and on the allowlist
retirement (empty-passes / any-Poison-rejected / synthetic-staleness). Kill rows
closed same-commit. The valve stood — the spike STOPPED for the mechanism ruling
before P5, and the structs flip / P6 are honestly ledgered rather than rushed.
