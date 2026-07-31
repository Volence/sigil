# Wave-B B-0b — RAM packing (the mechanism map, and why it already holds)

**Parcel goal (as briefed):** build the RAM analog of B-0's packed placement —
`.emp` RAM sections given anchors + live-size packing, even-alignment asserted, a
fold-identity bar, a t24 control, a growth probe. The premise: "RAM sections are
still hand-pinned (`pins.rs` `Pin` table)."

**What the map found (verified, not assumed):** the premise does not hold as
stated. There are **no `.emp` RAM sections**, and RAM was **never a build-input pin**.
RAM placement is already the B-0 analog — realized by AS `phase`/`dephase` plus
`phase Engine_RAM_End`, executed natively by sigil's AS frontend. Growth already
flows downstream automatically, in even steps, both shapes, with zero ROM-layout
perturbation. B-0b is therefore a **verification + a committed invariant guard**, not
a new mechanism. The rest of this note is the evidence.

## 1. The current RAM placement mechanism (the map)

### Where RAM lives
All RAM is authored in **AS**, not `.emp`:

- `aeon/engine/ram.asm` — engine RAM. Two `phase` blocks:
  - `phase $FFFF0000` (lower, `.l`-addressed large buffers: `Tile_Cache_Nametable`,
    `Tile_Cache_Collision`, `Block_Stage_Buffers`) → `Lower_RAM_End`, `dephase`.
  - `phase $FFFF8000` (upper, hot `.w` data: system, DMA queue, sprite/camera/tile-
    cache metadata, entity/ring window, sound state) → `Engine_RAM_End`, `dephase`.
- `games/sonic4/config/ram.asm` — game RAM, **`phase Engine_RAM_End`** (it packs
  onto the engine block; when engine RAM grows, `Engine_RAM_End` moves and game RAM
  repacks — the RAM analog of B-0's contiguous packing, done by AS).

A grep of the `.emp` corpus (`engine/`, `games/`) confirms every `.emp` `section`
decl is Z80 sound or a CPU-code module (`cpu: z80` / `module ... in section`); **none
declares RAM**. Address values `$FFFF0000/$FFFF8000` appear in `.emp` only as
consumers (e.g. `tile_cache.emp`, `parallax.emp` referencing RAM labels), never as
declarations.

### How the addresses are computed
sigil's AS frontend assembles `ram.asm`. Each `phase` block becomes one `Section`
with `vma_base` at the phase address; every `ds.b/ds.w/ds.l` advances the location
counter; each label takes the running counter as its VMA. Growth is intrinsic:
a `ds.b 2` inserted mid-block shifts every downstream label by 2, and the game-RAM
block (phased from `Engine_RAM_End`) shifts with it. No table, no pin, no relaxation.

The canonical resolve (`native::resolve_canonical_sections`, plain shape) yields
exactly **3 RAM sections**, all even-based, sorted by VMA:

| section        | vma_base   | labels | span    | ends      |
|----------------|------------|--------|---------|-----------|
| lower engine   | `$FFFF0000`| 4      | `$6842` | `$FFFF6842` (< `$FFFF8000` cap) |
| upper engine   | `$FFFF8000`| 170    | `$307A` | `$FFFFB07A` = `Engine_RAM_End`  |
| game RAM       | `$FFFFB07A`| 17     | `$388`  | `$FFFFB402`                     |

The upper-engine block and the game-RAM block **abut exactly** (`$FFFF8000 + $307A ==
$FFFFB07A`): game RAM is packed contiguously onto engine RAM. That contiguity IS the
packing; it moves with any engine-RAM growth.

### How RAM is kept out of the ROM image
`native::is_rom_section` classifies a section as RAM when `vma_base >= 0x00F0_0000`.
In `apply_declared_chain` the sections are `partition`ed: ROM sections sort by true
base and pin/chain (the B-0 walk); **RAM sections are appended unchanged** (never
chained, never given a ROM base). The resolve confirms **0** ROM sections carry a
high (`>= $FFFF0000`) lma. So RAM sizes cannot perturb ROM section bases — the
property that makes a RAM-growing parcel ROM-layout-safe.

### Where the guards already are (ram.asm, hand-authored)
- **Even alignment** (68k address-error avoidance): enforced by explicit pad bytes
  after every odd run (e.g. `VDP_Shadow_len` +1, `Section_Plane_Dirty` +1, the
  `__DEBUG__` block pad). AS does **not** auto-align `ds.w/ds.l` — the trap the brief
  flagged — so the pads are load-bearing and hand-placed.
- **Lower-RAM overflow:** `if Lower_RAM_End > $FFFF8000 -> error`.
- **Stack overflow:** `if Engine_RAM_End >= SYSTEM_STACK -> error`.
- **Object_RAM `.w` reachability:** `if (Object_RAM & $FFFF) < $8000 -> error`.

These fire loud at assemble time — the "overflow into an anchored island or the stack
fails loud" requirement is already met, by AS `error` directives.

### What `pins.rs` RAM cells actually are
The `Pin { plain, debug }` RAM cells (`PLANE_BUFFER_BASE`, `CAMERA_X`, `RING_BUFFER`,
`OBJECT_RAM`, ...) are **generated verification snapshots** (emitted by `repin` from
sigil's own resolve) consumed **only by port test gates** (`sprites_port`,
`core_port`, `camera_port`, `section_port`, `tile_cache_port`, `entity_window_port`,
...). Grep confirms zero RAM-pin references in the build path (`native.rs` uses only
the ROM `Region` pins, and only under the `PinnedBaked` map; the shipped `Frozen` map
reads resolved sections). RAM pins are outputs of the resolve, never inputs. "RAM is
hand-pinned" conflates these test snapshots with a build authority; there is none.

## 2. Fold-identity (unchanged sources) — all green

`SIGIL_STRICT_GATE=1 SIGIL_EMIT=.../emit_sound_blob SIGIL_BUILD=.../sigil
AEON_DIR=.../aeon`:

| gate                          | result                    |
|-------------------------------|---------------------------|
| `native_full_rom`             | ok — 3 passed / 0 failed  |
| `native_offcanonical_rom`     | ok — 4 passed / 0 failed  |
| `native_offcanonical_full`    | ok — 7 passed / 0 failed  |
| full strict (`cargo test --workspace`) | **2859 passed / 0 failed / 4 ignored** |

Baseline preserved (matches master 68d2349). No mechanism change was needed to hold
fold identity, because the mechanism did not change.

## 3. The growth probe (temporary, reverted) — the equivalent control

The brief's t24 shape (doctor a table anchor -> ROM moves; doctor a contiguous entry
-> inert) is **table-driven** and specific to the ROM frozen-table walk. RAM has no
such table — it is authored in AS and there is **no clean sigil-side knob** to doctor
a RAM size (the profile injects `emp_defines` cross-checked against the AS config via
`ensure(extern(..)==..)`; forcing a mismatch trips that guard, not the layout). The
faithful equivalent control for a non-table-driven mechanism is a **direct source
growth probe**, run manually and reverted:

**Probe:** insert `ds.b 2` after `Plane_Buffer_Ptr` in `engine/ram.asm`, regenerate
(`repin`), and rebuild.

**Results:**
- **Build resolves** — no address error, no overflow; `repin` wrote a fresh
  `pins.rs`.
- **Downstream RAM shifts exactly +2, both shapes.** Upstream-of-growth pins
  UNCHANGED (`PLANE_BUFFER_PTR`, `PLANE_BUFFER_BASE`, `CAMERA_X/Y`, `OBJECT_RAM`);
  downstream pins all +2 (`CACHE_LEFT_COL` A836->A838, `RING_BUFFER` A914->A916,
  `RING_COUNT` AC14->AC16, `BLOCK_STAGE_KEYS` A860->A862, `CAMERA_DEADZONE_BASE`
  A82E->A830; 76 RAM pins moved total).
- **ROM layout UNCHANGED.** Every ROM `Region`/symbol pin and both `ASSEMBLED_LEN`
  values were identical (0 diff lines) — RAM growth does not move ROM sections.
- **The RAM addresses flow INTO the ROM image.** With the probe in place
  `native_full_rom` DIVERGES from the golden (both shapes FAILED) — the grown RAM
  addresses are correctly baked as the new immediate values in ROM instructions. This
  is the fold-identity contrapositive: RAM movement DOES show in the ROM gates, so
  fold identity at unchanged sizes is a real (not vacuous) property.
- **Revert clean:** `git checkout engine/ram.asm` + `git checkout pins.rs`;
  `native_full_rom` back to 3/0; aeon tree pristine (`git status` clean, HEAD
  `ce03c7f`).

This is the "doctored size shifts downstream RAM, build succeeds, addresses move in
the ROM" control the brief asked for, delivered against the mechanism that actually
governs RAM.

## 4. The committed guard (aeon-free, in `native_offcanonical_placement.rs`)

Because there is no sigil-side RAM knob, the committed regression asserts the
**structural invariants** that make RAM growth safe — the properties the probe
exercised, captured without needing an aeon edit, both shapes
(`ram_packing_invariants_plain` / `_debug`):

- every RAM section (`vma_origin >= $FFFF0000`) is **even-based** (the 68k
  address-error guard — "assert it");
- **no ROM section lands in RAM** (`is_rom_section` partition holds: ROM bases are
  independent of RAM sizes);
- the **upper RAM packs contiguously** — sections at `vma >= $FFFF8000`, sorted,
  each successor base `==` predecessor end (game RAM chains onto `Engine_RAM_End`; a
  gap or overlap means the packing broke).

A regression here fires if a future change de-aligns RAM, leaks a RAM section into the
ROM image, or breaks the engine->game contiguity — the exact hazards entity_window #1
and tile_cache #2 could introduce.

## 5. Honest bounds / ledger candidates

- **The parcel re-scoped.** No new sigil mechanism was built because none is required;
  the RAM analog of B-0 is AS `phase`-from-symbol, already live. This is the
  disciplined outcome, not a shortcut — the "verify rather than assume" the brief
  mandated.
- **Even-alignment is still hand-enforced** in `ram.asm` (pad bytes), not machine-
  checked at the `ds` site. The committed guard checks the RESULT (section bases even)
  but cannot see a mid-section odd `.w` variable unless it surfaces as a checked pin.
  A structural fix (an AS-level "warn on odd `ds.w/.l`") is a **language/tooling ask**
  (step-3 class), not this parcel.
- **Pin churn on growth:** a mid-block RAM growth shifts every downstream test pin and
  requires a `repin`. The tail-append idiom already used (`HBlank_Vector_Slot`,
  `Dynamic_Live`, the prefetch memo block) ripples **zero** existing RAM addresses —
  the cheapest growth site. entity_window/tile_cache should prefer it where the data's
  access pattern allows.

## 6. What the next two parcels need from the mechanism

**Nothing new.** entity_window #1 and tile_cache #2 can grow engine RAM **today** by
editing `engine/ram.asm` directly:

1. Add the `ds.b/ds.w/ds.l` reservation — **at the RAM tail** if the new state has no
   ordering constraint (zero pin churn), else in place (downstream pins shift, which
   is fine and automatic).
2. Keep the block **even** — hand-pad odd runs (AS does not auto-align; the address-
   error trap).
3. Stay under the caps — the `Lower_RAM_End`/`Engine_RAM_End` guards fire loud if not.
4. `repin` to refresh the test-pin snapshots; re-run the port gates + fold-identity.

The ROM side (B-0) already absorbs the immediate-value changes automatically. The two
parcels are unblocked; the "RAM packing prerequisite" is satisfied by this
verification.
