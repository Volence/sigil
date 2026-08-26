# Hand-typed ROM addresses outside the five-site ripple doctrine

2026-08-26. Written alongside the `boot_port` fix that derived
`GameState_OJZScroll_Init` from `pins::OJZ_SCROLL_TEST`.

## Why this inventory exists

The five-site ripple doctrine names the files a byte-moving landing must
hand-edit: `pins.rs`, aeon's `engine.inc`, `mixed_dac_rom.rs`, `repin_pins.rs`,
`repin.toml`. The `parcel/rom-relayout` landing went red at 3941/2 on a sixth
site in none of them — two hand-typed addresses in `crates/sigil-cli/tests/boot_port.rs`.

The doctrine's file list is not the real boundary. The real boundary is a
**property**: a literal ROM address that a cartridge re-layout forces a human to
re-type. This note enumerates that property across the tree so the next landing
can sweep by it instead of by filename.

Enumerated by sweeping large hex literals across `crates/**/*.rs`, every
`build.rs`, `src/bin/*`, and every checked-in `.toml`/`.txt`/`.emp`/`.asm`/`.inc`
fixture under `crates/`, then triaging each hit. Deliberately NOT by grepping the
five filenames, and NOT by grepping the values that moved on 2026-08-26 — that is
the enumeration mistake that produced the miss.

Triaged crates: `sigil-cli` (142 test files), `sigil-frontend-emp` (122),
`sigil-isa` (16), `sigil-frontend-as` (14), `sigil-harness` (11), `sigil-link`
(2), `sigil-clownlzss-sys` (2), `sigil-clownnemesis-sys` (2), `sigil-s4lz` (1),
`sigil-salvador-sys` (1). `sigil-isa`, `sigil-link`, `sigil-s4lz`, `sigil-ir`,
`sigil-frontend-as` and the `-sys` crates triage out wholesale: none of them
reference `AEON_DIR`, the golden ROMs, or `pins::` at all.

## The triage rule

Not every literal ROM address is a defect. The class splits three ways, and the
split is what makes the sweep cheap next time:

1. **INPUT literals** — fed into the assembler, or used to slice the golden.
   These should derive. A literal here is usually a *copy* of the structural
   source, so it is not an independent witness, only a re-type obligation with a
   silent desync window between the refreeze and the re-type. This is what
   `boot_port` was.
2. **ORACLE literals** — the independent expectation a derivation is checked
   against. These must STAY literal; deriving them makes the test circular. They
   are still mandatory hand-edits at every re-layout, and that is exactly why
   they belong on a sweep list.
3. **SYNTHETIC literals** — fixture geometry in self-contained tests that read no
   aeon tree and no golden. Out of the class entirely.

`crates/sigil-cli/tests/ports.rs:760` is the exemplar of doing 3 right, and its
comment states the discipline outright: one synthetic constant, and "every
expected byte below is DERIVED from this one constant by the fold's own formula,
never retyped, so the probes stay non-vacuous wherever it points."

The house precedent for doing 1 right is `core_negative_probes.rs:197,216,240,246`
and `dplc_negative_probes.rs:173,196,232,238`: the *real* arm is
`format!("{:#x}", pins::CORE.plain_base)`, and only the *deliberately wrong* arm
stays a literal.

---

## FINDING OF RECORD — stale bases in probes that cannot detect the staleness

This is the most important item in the note and it is not merely hygiene.

Four `tranche*_negative_probes.rs` files hand-type what was, when written, the
real region base. Those bases have since moved and **the literals were never
updated** — no gate noticed, because these probes are `assert_ne!` shaped ("the
doctored compile must NOT match the reference") and that assertion is
**insensitive to the base**.

Worked example, `crates/sigil-cli/tests/tranche4_negative_probes.rs:140-144`:

```rust
let (sections, _asserts) = place(&doctored, &src, "0x309DE");
assert_ne!(
    link_bytes(&sections),
    refrom[0x309DE..0x309E6].to_vec(),
    "a drifted AF_DELETE const must NOT byte-match the reference"
);
```

`pins::PARTICLE_ANIMS` is now `{ plain_base: 0x2A402, debug_base: 0x2AC48,
plain_len: 0x0, debug_len: 0x8 }`, so `0x309DE` is nowhere near either base.

**Measured, not assumed** (2026-08-26): repointing this file's literals at the
current pin (`0x309DE` → `0x2A402`, slice `0x2A402..0x2A40A`) and re-running
leaves all six probes green — `6 passed; 0 failed`. They pass at the stale base
and at the correct base alike. So the precise finding is not "the probe is red
and nobody noticed" and not "the probe can never fail"; it is:

> The base literal is **not load-bearing** for the assertion. It is a stale,
> non-load-bearing echo of the real pin, and the probe therefore proves less than
> its comment claims — "the doctored bytes differ from the genuine window at the
> true base" degrades to "the doctored bytes differ from some ROM window", which
> is nearly always true.

The real non-vacuity work is done by the paired positive port gate
(`particle_anims_port.rs`), which *is* pin-sourced. The consequence of the
staleness is lost intent and a lying comment rather than a false green — but it
is also proof that nothing in the tree can tell you when one of these literals
rots, which is the property that matters for the doctrine. Line 153's comment,
"FALSIFIED by the port gate placing at the true `0x309DE`", records that the
author did believe it was the true base.

Full list of stale real-arm bases, each with its current pin:

| file:line | literal | pins | current value |
|---|---|---|---|
| `tranche4_negative_probes.rs:139,142,153,158,163,164` | `0x309DE`, `0x309E6` | `particle_anims` base + golden slice | `pins::PARTICLE_ANIMS` `0x2A402` / `0x2AC48` |
| `tranche4_negative_probes.rs:220,319,327,328` | `0x30970`, `0x30972` | `sonic_anims` base | `pins::SONIC_ANIMS.plain_base` `0x29FD0` |
| `tranche4_negative_probes.rs:423,463,465,466` | `0x14AE6`, `0x14AE8` | `act_descriptor` base | `pins::ACT_DESCRIPTOR.plain_base` `0x16744` |
| `tranche2_negative_probes.rs:335,336,392,462,470` | `"0x228C"` | `controllers` base | `pins::CONTROLLERS.plain_base` `0x2460` |
| `tranche2_negative_probes.rs:361,362,499,505` | `"0x2464"` | `math` base | `pins::MATH.plain_base` `0x2850` |
| `tranche5_negative_probes.rs:95` | `0x22FE` | `game_loop` base | `pins::GAME_LOOP.plain_base` `0x256E` |
| `tranche5_negative_probes.rs:200-205` | `0x2314` | game_loop base + `0x16` | `pins::GAME_LOOP.plain_base + 0x16` |
| `tranche5_negative_probes.rs:447,503` | `0x5D8E` | `sound_api` base | `pins::SOUND_API.plain_base` `0x7EAE` |
| `tranche6_negative_probes.rs:106,112` | `0x10F7C`, `0x10F8A` | `test_solid` / `test_particle` bases | `pins::TEST_SOLID` `0x12250`, `pins::TEST_PARTICLE` `0x12262` |

Severity is not uniform. The probes that compare a genuine compile against a
doctored compile *at the same base* prove their point at any base by
construction. The probes that slice `refrom` (tranche4 above) intend to say
something about the real window and no longer do. All of them should convert to
`pins::*` following `core_negative_probes.rs`; the `refrom`-slicing ones first.

`tranche5_negative_probes.rs:240` already uses `pins::GAME_LOOP.plain_len` in the
same file, so the import and the idiom are both already there.

---

## Class 1 — INPUT literals that should derive (open, not fixed here)

### `crates/sigil-cli/tests/keystone_flip_relocation.rs:48`

```rust
const DEFORM_DIVERGENT_BYTE: usize = 0x11412;
```

The doctoring site for `doctored_golden_at_deform_pointer_is_caught`.

**Derivation exists in the same file.** `deform_pointer_equals_placed_label_vma`
(`:98-125`) computes exactly this address from the build's own listing:

```
listing["ParallaxConfig_OJZ_Default"].value + DEFORM_PTR_HDR_OFF
```

Its two neighbouring constants were already de-literalized after earlier rot —
`EOR` is `pins::DEBUG_ASSEMBLED_LEN` "PIN-SOURCED since bug005: the old literal
rotted at its second hand-shift too", and `DEFORM_PTR_HDR_OFF` is the
listing-derived header offset after "the second hand-shift of the old `0x11420`
literal killed it". `DEFORM_DIVERGENT_BYTE` is the one left behind in that
cleanup.

### `crates/sigil-cli/tests/demo_native_port.rs:35-37` and `:93-95`

```rust
("demo_box",   0x10002, 0x4),
("demo_data",  0x10006, 0xFA),
("demo_state", 0x10100, 0x72),
...
assert_eq!(resolve("DemoBox_Main"),        0x10002, ...);
assert_eq!(resolve("ObjDef_DemoBox"),      0x10006, ...);
assert_eq!(resolve("GameState_Demo_Init"), 0x10100, ...);
```

These slice `golden/demo.bin` and assert placement. The file's comment says "the
demo modules carry no `pins` constant (repin resolves only the sonic4 shape), so
the window is named here" — but that is no longer the whole picture.

**Derivation exists**: `crates/sigil-harness/golden/offcanonical_sizes/demo.txt`
already carries `DemoBox_Main 0x10002` (:24), `GameState_Demo_Init 0x10100`
(:28), `ObjDef_DemoBox 0x10006` (:38), regenerated by `src/bin/derive_offcanon.rs`
— and `demo_profile()` already loads that very file via
`SizeSource::Frozen(load_frozen_table("demo.txt"))`. Replacement: read the three
values out of that table. Note the table and the `resolve(...)` asserts are two
independent copies of the same three numbers today, so the site costs two
re-types, not one.

### `crates/sigil-cli/tests/test_p2_player_states_port.rs:348` and `:375`

```rust
player_jump: 0x105F0, // filled at need; ground-local (see note)
```

The single hand-typed field in two `Shape` structs whose other 22 fields are all
`pins::*`. It is fed into the link as `("Player_Jump", shape.player_jump)` and
therefore into bytes compared against the golden — an INPUT, the same shape as
the `boot_port` miss.

**The same literal is used for both the PLAIN and the DEBUG shape.** Every
neighbouring field is per-shape (`pins::P_STATE_JUMP` is
`{ plain: 0x10AB8, debug: 0x10BC8 }`), so a single value across both shapes is
suspicious on its face and worth checking before it is derived.

No exact pin exists — `Player_Jump` is an intra-region player label. The fix is
either a `repin.toml` symbol entry for it, or an offset off `pins::P_STATE_GROUND`.

### `crates/sigil-cli/tests/native_object_bank_budget.rs:69,71`

`used < 0x10000` — the object-bank window length hard-typed. The map's own
declared ceiling is already exposed by `map_placement::PlacementMap` and already
read by `native::check_object_bank_budget` (`native.rs:4039-4047`):
`pmap.budgets.iter().find(|b| b.region == "object_bank").ceiling`.

### `crates/sigil-cli/tests/test_g4_final_objects_port.rs:146,148`

`player_base: 0x1E000`, `enemy_base: 0x1F800` — documented as scratch bases
"INSIDE the 64KB object bank window … past all real plain content". Layout-coupled
twice: it assumes the object bank spans `0x10000..0x20000` *and* that no real
plain content ever reaches `0x1E000`. No derivation in tree; would need the
object-bank anchor plus ceiling from `map_placement`.

---

## Class 2 — ORACLE literals that must STAY literal (sweep, do not derive)

### `crates/sigil-cli/tests/seam2_layout_derivation.rs:39-60`

Ten sound-bank LMAs (`dac_blip_lma: 0x90000` … `sfx_bank_lma_debug: 0xA5570`).
The file states the role: "the INDEPENDENT literal drift detector for that
derivation (the same role `pins.rs` plays for the pins)". Deriving them from
`seam2::sound_layout` would assert the derivation equals itself.

This is the single biggest omission from the doctrine after `boot_port.rs`. It
**must** be re-typed at every re-layout, and it correctly was on 2026-08-26
(commit `301bc6a6`) — but by the parcel happening to touch the sound side, not
because any doctrine named it.

### `crates/sigil-cli/tests/native_full_rom.rs:180`

```rust
("Ground_Move_Cap", 0x10912, 0x10A22),
```

The only non-pin-sourced row in `LOAD_BEARING` (every other row is
`pins::GAME_LOOP`, `pins::ERROR_HANDLER`, `pins::SECTION`, `pins::BG`,
`pins::ANIMATE`, `pins::COLLISION`, `pins::BOOT_HEAD.plain_base + 0x36`).

**This row is deliberate and must stay a literal.** The file argues it at
`:70-75`: "No pin exists for an intra-region player label, and it must STAY
unpinned: the whole point of this row is to be an INDEPENDENT expectation
checking the convsym resolve path, so deriving it from a pin would make the
assertion circular." That is correct — `repin` generates the pins from the same
listing `convsym` consumes, so a pin-derived expectation would prove nothing
about the resolve path.

What it needs is not a derivation but a doctrine entry: it is byte-move-sensitive,
no tool touches it, and it carries roughly ninety lines of change-history
narration (`:90-179`) recording every hand-shift since 2026-08-05 — which also
violates the comments-describe-function house rule and should be collapsed at the
next touch. For the person doing the hand-update, the file's own notes at `:160`
and `:172` record the arithmetic: `pins::P_STATE_GROUND.{plain,debug} + 0x2F2`
(verified: `0x10912 - 0x10620 = 0x2F2`, `0x10A22 - 0x10730 = 0x2F2`).

`:59` `("EntryPoint", 0x200, 0x200)` on the same table is layout-immune — the
fixed post-header entry.

### `crates/sigil-cli/tests/seam1_native_link.rs:126`

`assert_eq!(BLOB_LEN_PLAIN, 0x1814, …)` — the same independent-drift-detector
role for seam-1. Hand-edit site, but sound-blob-sensitive rather than
relayout-sensitive.

---

## Class 3 — synthetic, out of the class (triage record)

Recorded so the next sweep does not re-litigate them:

- `crates/sigil-cli/tests/ports.rs:760` — `PROBE_BANK_VMA = 0x58000`, documented
  synthetic, all expected bytes derived from it by the fold's own formula.
- `crates/sigil-harness/src/{native,repin,map_placement,seam2,provenance}.rs`
  `#[cfg(test)]` blocks — invented section/symbol geometry (`0x10000`, `0x58000`,
  `0x5E688`, `0x99999`, `0x15A34`) in unit tests that read no aeon tree.
- `crates/sigil-cli/tests/seam2_phased_head.rs:138,154` — `0x5856D`, explicitly
  "a representative physical address inside the bank"; self-consistent.
- Synthetic toy maps: `module_resolution.rs`, `mt_dual_carrier.rs`,
  `sound_migration_negative_probes.rs`, `dac_bank_acceptance.rs`,
  `tranche21/23_spelling_probes.rs`, `compression_selftest_port.rs:304,317`,
  `error_handler_port.rs:280,287`, `game_loop_port.rs:251`.
- Hardware/format constants — `$C00000`/`$C00004` VDP, `$A11100`/`$A00000` Z80
  bus, all `$FFFFxxxx` RAM pins, `$7F8000` bank mask, `$8000` window size,
  `lib.rs:91,98` Mega Drive header field ranges, `0x400000` cart size. RAM and
  hardware do not move with a ROM re-layout.
- Opcode/encoding/fixed-point constants across `sigil-isa`, `sigil-frontend-emp`
  eval tests, `pitcher_plant_acceptance.rs`, `sigil-clownlzss-sys`.
- Fixture files `crates/sigil-cli/tests/vectors/**`,
  `crates/sigil-frontend-emp/tests/vectors/*.toml`,
  `crates/sigil-harness/m1c_root.asm` — zero 4+-digit hex.
- `crates/sigil-isa/tests/m68k_golden_vectors.txt`,
  `crates/sigil-frontend-as/tests/snippets_golden.txt` — instruction-encoding
  goldens, unrelated to cartridge layout.

---

## Already rotted, fix regardless of the doctrine

### `assembled_len` on the off-canonical profiles — all three wrong

`crates/sigil-harness/src/native.rs:874` (demo), `:940` (config_b), `:1000`
(config_a) hand-type an `EndOfRom` ROM-tail length. Every one disagrees with the
profile's own generated frozen table:

| profile | `native.rs` | `golden/offcanonical_sizes/*.txt` `assembled_end` |
|---|---|---|
| demo | `0x11224` | `0x1121c` |
| config_b | `0x434d0` | `0x8b6f0` |
| config_a | `0x5f65a` | `0xa7c70` |

The canonical profiles beside them do it right (`:781`
`if debug { pins::DEBUG_ASSEMBLED_LEN } else { pins::ASSEMBLED_LEN }`, `:1053`
`pins::ASSEMBLED_LEN`). The only consumer in the tree is
`src/bin/derive_offcanon.rs:130` as an `.unwrap_or(profile.assembled_len)`
fallback — no test reads the field, which is exactly why it rotted unnoticed.
Fix: read `assembled_end` from the frozen table the profile already loads, or
delete the field.

### `crates/sigil-harness/src/lib.rs:69` — dead ROM address const

`pub const REGION_A_LMA: u32 = 0x3DE;` — the resident phase-0 Z80 driver base.
Its doc comment records two prior hand-slides (`0x3EA → 0x3E0 → 0x3DE`). It has
**zero code consumers** (only two mentions in `golden/PROVENANCE.md`). Its sibling
`REGION_B_LMA` was already deleted for exactly this reason, with the rationale
recorded at `:70-74`: the bank base's sole authority is the `sound_bank` anchor in
`games/<g>/map.toml`, "so the const was DELETED rather than left as a second,
drifting truth". Delete this one too.

---

## Adjacent: stale ADDRESS COMMENTS the re-layout left behind

Not load-bearing — the code beneath derives — but each now names an address that
no longer exists, which is the exact material a future session mis-grounds on:

- `crates/sigil-harness/src/seam2.rs:152-164` — the `SoundLayout` field docs still
  quote `$58357`, `$5845F`, `$5856D`, `$585AD`, `$58628`, `$5BB10`, `$5D560`. The
  struct's *header* comment (`:147-150`) acknowledges the `+$48000` shift; the
  per-field ones were not updated.
- The `//!` headers of sixteen port gates now name windows that moved:
  `act_descriptor_port.rs:36-37`, `animate_port.rs:52-53`, `collision_port.rs:58-59`,
  `controllers_port.rs:91-92`, `core_port.rs:35-36`, `dplc_port.rs:29-30`,
  `game_loop_port.rs:55-56`, `math_port.rs:70-71`, `mt_port.rs:30-31`,
  `particle_anims_port.rs:40-41`, `rings_port.rs:53-54`, `seam1_native_link.rs`,
  `sfx_port.rs:35-36`, `sonic_anims_port.rs:19-20`, `sprites_port.rs:49-50`,
  `test_mappings_port.rs:18-19`. Plus in-body message strings at
  `math_port.rs:321,364`, `controllers_port.rs:455,504`,
  `core_negative_probes.rs:246`, `dplc_negative_probes.rs:238`.
- `crates/sigil-cli/tests/sfx_negative_probes.rs` (`0x63AE8`, `0x60607`,
  `0x67C00`, `0x60000`) and `crates/sigil-harness/src/native.rs:2944,3002`
  (`0x58000`, `0x4800c`).
- `crates/sigil-harness/src/seam1.rs:26` says the plain blob is "6255 B" (it is
  `0x1814` = 6164), and `seam1_native_link.rs:120` says "Both current lengths are
  odd, hence a 1-byte pad in each shape" (both are even).

---

## `Z80_SOUND_SIZE` in `boot_port.rs` — deliberately left literal

`frozen_symbol`'s other two arms (`0x1814` / `0x1896`) are a driver SPAN, not an
address: they move with the sound blob and are untouched by a cartridge
re-layout, so they are outside this note's class.

A structural derivation does exist: `seam1::BLOB_LEN_PLAIN` (`seam1.rs:36` =
`0x1814`) and `BLOB_LEN_DEBUG` (`:43` = `0x1814 + 0x82` = `0x1896`), rounded up
to even — aeon pads the blob to an even length inside the
`Z80_Sound_Start`/`_End` brackets, per `seam1_native_link::blob_lengths_are_canonical`.
Both current lengths are already even, so the pairs are equal today.
`boot_port.rs`'s own comment already says "re-pin `Z80_SOUND_SIZE` in lockstep";
the lockstep partner is one crate away. `BLOB_LEN_*` is itself a hand-pinned
tripwire, so this trades two re-type sites for one rather than eliminating them.
Not taken in this parcel.

---

## Recommendation for the doctrine

Replace "the five files" with the property plus a sweep, and add the sites that
genuinely cannot derive. In priority order the doctrine should name:

1. `crates/sigil-cli/tests/seam2_layout_derivation.rs` — ten sound-bank LMAs,
   literal by design, mandatory re-type every relayout.
2. `crates/sigil-cli/tests/tranche{2,4,5,6}_negative_probes.rs` — nine stale real
   bases; convert to `pins::*` rather than re-type, following
   `core_negative_probes.rs`.
3. `crates/sigil-cli/tests/demo_native_port.rs` — three demo golden offsets,
   convertible to `demo.txt`.
4. `crates/sigil-cli/tests/native_full_rom.rs:180` and
   `test_p2_player_states_port.rs:348,375` — intra-region labels with no pin;
   genuine hand-edits (the first deliberately so).
5. `crates/sigil-cli/tests/keystone_flip_relocation.rs:48` — convertible to the
   listing-derived expression already present at `:98-125`.
6. `crates/sigil-harness/golden/offcanonical_sizes/*.txt` — seven checked-in
   fixtures of absolute label→address rows. They *are* regenerated
   (`src/bin/derive_offcanon.rs`, `golden/derive_offcanonical_sizes.sh`) and were
   updated in `0f2698db`, but the regeneration step is not named in the five and
   **no gate fails if you skip it**.

And the sweep itself, for anything this note missed:

```
grep -rnE '0x[0-9A-Fa-f]{5,}' crates/ --include='*.rs'
```

triaging every hit into input / oracle / synthetic. The five filenames stay
useful as the *auto-repin* list; they are not the boundary of the re-type
obligation, and treating them as such is what cost the rom-relayout landing a red
gate — and, as the tranche probes show, has been quietly costing stated intent
for longer than that.
