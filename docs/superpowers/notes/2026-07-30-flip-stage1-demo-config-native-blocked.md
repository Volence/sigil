# 2026-07-30 — FLIP STAGE 1 · demo + Config-A/B native drivers BLOCKED — MANDATORY STOP

Status: **STOP — the demo native whole-ROM driver (my Stage 1) AND the Config-A/B
native drivers (my Stage 2) are NOT buildable against the frozen aeon tree
(`bcb8f64`). All three off-canonical targets require PER-GAME / PER-CONFIG gate
RESUME ORGS that the frozen tree does not provide — exactly the S1.2
computed-resume-org capability, which was DEFERRED to Stage 2 (824fed5). Only the
CANONICAL sonic4 plain/debug native drivers (S1.1, already landed) work in the
frozen tree, precisely because `engine.inc` hardcodes exactly those two layouts'
resume orgs.** This is a design fork the overseer must rule before demo/Config
native can land; per the valve and the STOP-on-design-fork directive it is
surfaced, not papered over with an aeon edit. The un-blocked golden-freeze
infrastructure (fresh-build capture script + provenance) proceeds separately.

The prior demo scoping note (executor-3, ba39390) characterized the demo LAYOUT
and the demo PIN derivation correctly, but its item 3 ("engine code gates ON")
did not account for the gate RESUME ORGS being sonic4-hardcoded. This note closes
that gap: the resume orgs are the missing piece, and they are S1.2 work.

## The root cause — gate resume orgs are hardcoded CANONICAL sonic4 addresses

The all-gates native driver flips every `SIGIL_EMP_*` gate ON. When a gate is ON
the AS side SKIPS the `.asm` twin and `org`s to the region END (the RESUME point),
leaving a hole the natively-placed `.emp` fills. Those resume `org` values live as
LITERALS in `engine/engine.inc` and `games/sonic4/main.asm`, and every one is a
CANONICAL sonic4 address (plain = `s4.lst`, debug = `s4.debug.lst`, sound-ON).
Each carries an explicit in-tree NOTE, e.g. (engine.inc BOOT):

```
      ; NOTE: these org values are sonic4-shape addresses — the gate define
      ; must never be set for other games (demo builds take the include).
    ifdef __DEBUG__
        org     $3AC
      else
        org     $3A8
      endif
```

The same NOTE guards VDP_INIT, DMA_QUEUE, …, COMPRESSION_SELFTEST, SOUND_API,
SOUND_DEBUG, ERROR_HANDLER, and (main.asm) GAME_DEBUG. So flipping a gate ON for
ANY layout other than canonical sonic4 resumes at the WRONG address.

The design ALREADY anticipated this: §2.3 OQ-6 ruled "declare region geometry +
section ordering; **COMPUTE every per-shape / off-canonical resume `org` as a link
output.** Do not pin any resume address the map can derive from placement." That
is S1.2. Executor-2 DEFERRED S1.2 to a Stage-2 companion task (824fed5,
map-region growth is placer-gated). **Demo-native and Config-A/B-native each ⊇
S1.2** — they cannot precede it. This is the sequencing correction.

## The empirical proof (three targets, all airtight — aeon `bcb8f64`)

Method: assemble each target through `sigil-frontend-as` (byte-identical to asl,
M0/M1), `resolve_layout`, read each region-boundary label's final VMA. Canonical
truths are `pins.rs` / `s4.lst` / `s4.debug.lst`.

### Target 1 — DEMO (sound-off). BLOCKED.

`demo.lst` truth vs the sonic4 resume org the BOOT gate would use:

| label | demo (real) | sonic4 resume org (engine.inc) |
|---|---|---|
| `BootData` | **0x3A2** | **0x3A8** (plain) / 0x3AC (debug) |
| `GameLoop` | 0xAAE | 0x239A |
| `AnimateSprite` | 0x16E6 | 0x2F3C |

Demo is sound-OFF, so the resident Z80 driver is absent and the ENTIRE engine
layout is shifted down. Gating BOOT ON for demo resumes at `$3A8` (sonic4) but
demo's `boot_data.asm` belongs at `$3A2` — a 6-byte error that misassembles
everything downstream. (Sound-off also drops sound-provided symbols like
`SFXID_RING_RIGHT`, a second, separate consequence — but the resume-org
mismatch alone is fatal.)

### Target 2 — CONFIG-B (sonic4, sound-off, plain). BLOCKED.

Config-B reference (gates OFF, `SOUND_DRIVER_ENABLED` unset) vs canonical plain:

| label | Config-B (real) | canonical plain |
|---|---|---|
| `BootData` | 0x3A2 | 0x3A8 |
| `GameLoop` | **0xB48** | **0x239A** |
| `Section_Init` | 0x3D46 | 0x55A4 |
| `BusError` | **0x42420** | **0x5CAB0** |

Sound-off removes the resident Z80 driver → the whole engine layout collapses
downward (GameLoop −0x1852, BusError −0x1A690). The canonical resume orgs are
wholesale wrong for Config-B. (Note Config-B's `AnimateSprite`=0x16E6 equals
demo's — both are sound-off, confirming sound-on/off is the dominant layout
driver.)

### Target 3 — CONFIG-A (sonic4, __DEBUG__ + sound-on + hotkeys + mirror). BLOCKED.

Config-A reference (gates OFF, Config-A defines) vs canonical debug:

| label | Config-A (real) | canonical debug | Δ |
|---|---|---|---|
| `Section_Init` | 0x6408 | 0x633C | **+0xCC** |
| `Camera_Init` | 0x67E4 | 0x6718 | +0xCC |
| `BG_Init` | 0x6F6C | 0x6EA0 | +0xCC |
| `BusError` | 0x5E5AA | 0x5E5AA | 0 (absorbed) |

At Config-A `game_debug` (+SOUND_DEBUG_HOTKEYS) and `sound_debug` (+SOUND_DBG_MIRROR)
become NON-empty (canonically zero bytes), shifting the `$6xxx` engine-level
regions (SECTION/CAMERA/PARALLAX/LOAD_ART/BG/BG_ANIM) by +0xCC. `BusError` (after
the FIXED `org $10000` object-code bank) is unshifted — the fixed bank org absorbs
the slack. So a native all-gates Config-A build using canonical resume orgs
misplaces every `$6xxx` engine region. (The existing `mixed_offcanonical_rom.rs`
gates work only because they flip JUST game_debug+sound_debug and leave every
other engine `.asm` AS-included, so the reference and mixed share one Config-A
layout — an all-gates build cannot.)

## Why this is a STOP, not a workaround

The only ways to unblock are:

1. **Pull S1.2 forward** — make the engine.inc/main.asm resume orgs COMPUTED link
   outputs (or game-parameterized). This is an AEON edit across ~30 gate sites,
   must keep the asl default byte-identical, and is exactly the deferred Stage-2
   work. The brief binds aeon read-only ("zero aeon commits expected — STOP and
   report if one seems needed").
2. **Add demo/Config-shape resume-org arms** to each gate (`ifdef` keyed on a
   game/config marker). Also an aeon edit; also essentially S1.2.
3. **Defer demo + Config-A/B native to Stage 2**, where S1.2 lands and the `.asm`
   twins delete anyway (at which point there is no "include arm" and the resume
   orgs MUST be computed regardless).

Options 1–2 are aeon changes the brief forbids without a ruling. Option 3 is the
sequencing the campaign structure already implies (demo lockstep is a Stage-2
precondition; S1.2 is Stage-2). No option is buildable in THIS session without an
aeon edit — hence the STOP.

## Consequence for my four stages

- **Stage 1 (demo native driver): BLOCKED** on S1.2. The demo PIN derivation +
  engine-only registry are ready groundwork (reuse `repin::parse_listing` over
  `demo.lst`/`demo.debug.lst` + the engine-region subset of `repin.toml`), but the
  whole-ROM driver cannot assemble a correct demo ROM until the resume orgs are
  computed. Not committed (untestable end-to-end while blocked).
- **Stage 2 (Config-A/B native): BLOCKED** on S1.2, same root cause. The
  `mixed_offcanonical_rom.rs` per-module gates remain the live Config-A/B witness.
- **Stage 3 (golden freeze): PARTIALLY un-blocked.** The 4 REAL asl-produced ROMs
  (`s4.bin`, `s4.debug.bin`, `demo.bin`, `demo.debug.bin`) can be frozen as
  durable blobs with a fresh-build capture script (they are asl outputs; no native
  driver needed to CAPTURE them). Config-A/B have NO shipped asl file and their
  native reproduction is blocked, so they cannot be frozen this session. The demo
  golden BLOBS freeze fine; the demo NATIVE gate (native == asl) is blocked.
  Delivered: the capture script + PROVENANCE record (see the golden-freeze note).
- **Stage 4 (close checkpoint): the honest matrix** = canonical sonic4 plain/debug
  native == asl (S1.1/S1.4, green); everything off-canonical BLOCKED on S1.2.

## Recommended re-scope (for the overseer to rule)

1. **Confirm demo + Config-A/B native are Stage-2-coupled** (they ⊇ S1.2), and
   RE-SEQUENCE: S1.2 (computed resume orgs) becomes the PREREQUISITE for the demo
   and off-canonical native gates, all landing together at/after the map-manifest
   growth — not as independent Stage-1 steps.
2. **OR authorize the S1.2 aeon edit now** (compute/parameterize the resume orgs),
   which unblocks all three targets in Stage 1 — a larger aeon change than the
   brief's "zero aeon commits" envisioned; needs an explicit ruling.
3. Either way, the CANONICAL sonic4 plain/debug native drivers (S1.1) + the
   split-golden full-file gates (S1.4) stand green and unaffected; the golden
   freeze of the 4 real ROMs proceeds as un-blocked infrastructure.

## Stage-2 deletion-manifest additions (this note)

- When S1.2 lands and the engine `.asm` twins delete, the resume orgs MUST already
  be computed (there is no include arm to org-resume past). The demo + Config-A/B
  native gates then become buildable and re-comparand to their frozen goldens.
- The demo pin derivation (engine-region subset of `repin.toml` resolved against
  `demo.lst`/`demo.debug.lst`) is the demo driver's first sub-task once unblocked.
