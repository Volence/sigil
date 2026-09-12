# LINK-ZERO-BYTE-MOVE-PLACEMENT: a zero-byte `ensure` moved placement through the far scratch (2026-09-12)

Queue row LINK-ZERO-BYTE-MOVE-PLACEMENT, routed from aeon `cae58661` (commit message) and
`2026-09-11-aeon-lens-pins-findings.md` section 2. Branch `parcel/link-zero-byte-move-placement`.
The repro files and every script used below are committed beside this note in
`2026-09-12-link-zero-byte-move-placement/`.

**The principle under test:** placement must never depend on where a zero-byte item (a const,
an ensure) sits in a source file. **Verdict: it did.** One top-level `ensure` after a file's
last `section {}` block moved the layout of every shape measured, and turned sonic4 DEBUG red.
The mechanism is the far-scratch measuring cursor in `native.rs::lens_pinned`, whose slots wrap
the 24-bit bus: a known hazard (campaign gap ledger, BGROOM-3 row) that this is the first
observed consequence of.

## 1. Reproduction

Trees: aeon repro tree `/home/volence/sonic_hacks/.sigil-zbm-aeon-cae58661` (detached at aeon
`cae58661`); sigil at this branch's base `dec6dcb0`, built with a private target dir, both
`sigil` and `emit_sound_blob`. The red tree was never committed in aeon, so it is
reconstructed: `boot_data.red.emp` is the pristine `cae58661` file with `VDP_REG_PLANE_SIZE`
dropped from the first `use` line, `PLANE_H_CELLS, PLANE_V_CELLS` folded into the existing
`VRAM_SPRITE_TABLE` `use` line, and the `f3664087` comment, const and ensure appended at file
end (`make_red.py`). `constants.emp`'s copy of the const (added by `cae58661` itself) was
REMOVED for the faithful form, since the red tree predates it; section 2 shows it does not
matter either way.

`FAST=1 DEBUG=1 ./build.sh sonic4` with the red file exits 1:

```
error: native build (sonic4 debug): span pass (declared): span pass: resolve_layout: 1 diag(s); first Some(Diagnostic { level: Error, message: "sections `ojz_scroll_test\092` [0xBE2D4, 0xC02F2) and `replay_fixture\0105` [0xC02EC, 0xC054C) overlap in the image (colliding pins)", ... })
```

The same sections, addresses and 6-byte overlap aeon reported.

**Control and red, FAST, sigil `dec6dcb0`** (CRC32 and size of the built ROM):

| shape | pristine `cae58661` | red `boot_data.emp` |
|---|---|---|
| sonic4 plain | exit 0, `8FBE85E7` 821103 | exit 0, `3BE2988B` 821101: 1157 of 2526 listing labels moved (+2 from `HBlank_Install` on) |
| sonic4 debug | exit 0, `07343B59` 847367 | **exit 1, the overlap above** |
| demo plain | exit 0, `F7FDF77D` 97051 | exit 0, `E63260F9` 97051: 1059 of 1506 labels moved (-2 from `HBlank_Install` on) |
| demo debug | exit 0, `FF11E22D` 103335 | exit 0, `63349032` 103335 (labels not diffed) |

Only sonic4 debug goes red, but **no shape is unmoved**: aeon's "no emitted byte changed" held
for the `dc.b` byte and not for the image. The canonical (non-FAST) control is in section 5.

## 2. Reduction

FAST DEBUG sonic4, `dec6dcb0` binaries, each variant alone on the pristine tree:

| variant (file here) | change | result |
|---|---|---|
| `boot_data.v_const_end.emp` | const moved after its `dc.b` use, NO ensure (constants.emp copy removed) | exit 0, ROM `07343B59` 847367, identical to control |
| `boot_data.v_ensure_end.emp` | the f3664087 ensure alone appended at file end, const left in constants.emp | **red, same text and addresses** |
| `boot_data.v_trivial_end.emp` | `ensure(1 == 1, "zbm")` appended to the pristine file | **red, same text and addresses** |

The forward reference is not the trigger. **Any top-level `ensure` after the file's last
`section {}` block is.** It needs no const, no reference to anything, and nothing in
`boot_head`: `lower_module_inner` (`crates/sigil-frontend-emp/src/lower/mod.rs`) calls
`ensure_default` for an `ast::Item::Ensure`, and after a `section {}` block has closed the
default section that opens a NEW default `text` section, zero bytes and label-less. A `const`
does not call `ensure_default`, which is why the const-only move is inert. The pristine file's
own top-level ensures sit before `section boot_head` and share the default section already
open there, so they add nothing.

What the reduction supplies that the original lacked, and the reverse: the trivial ensure
supplies nothing the original did not (it is the original's ensure with the expression and
message removed); the original supplies one thing the const-only variant lacks, the ensure,
and that is the whole trigger. The reduction's trigger is also not boot_data-specific: the
extra section matters only because it sits AHEAD of other never-pinned sections in
declaration order (section 3), so a trailing ensure in any module early enough in the
program has the same effect, and one late enough has none.

## 3. Mechanism

### Measured (an instrumented copy of `lens_pinned` dumping every measuring pass)

1. **One extra ROM section.** Red has 122 sections where the control has 121: index 12, a
   `text` section, label-less, zero bytes, directly after `boot_head`/`boot_tail` (10, 11).
   Every later index shifts by one (`ojz_scroll_test\092` in the red diagnostic is `\091` in
   the control).
2. **It draws a far-scratch slot.** A ROM section with no frozen-table label measures at
   `0x70_0000 + k*0x10_0000`, k its ordinal among the never-pinned ROM sections. The new
   section takes k=4 and every later never-pinned section moves up one slot:

   | k | control | red |
   |---|---|---|
   | 4 | `replay` 0xB00000 | the new `text` 0xB00000 |
   | 8 | `preset` 0xF00000 | `palette` 0xF00000 |
   | **9** | **`page_in` 0x1000000, masked 0x0** | **`preset` 0x1000000, masked 0x0** |
   | 10 | `page_cache` 0x1100000 | `page_in` 0x1100000 |

   k = 9 is `0x0100_0000`; `sigil_ir::asl_width_rule` masks to 24 bits and returns abs.w for
   0x0. (k = 25 is also an alias slot; it holds a zero-byte `text` in both runs.)
3. **Widths follow the slot.** Relaxable sites whose lowered length differs between the two
   packed rounds (the sections a width depends on):

   | section | site | control | red | why |
   |---|---|---|---|---|
   | `ojz_scroll_test` | `ojz_scroll_test.emp:910` `jbsr Effects_ResolveParallax` | 6 | 4 | `preset` onto the alias slot |
   | `ojz_scroll_test` | `ojz_scroll_test.emp:2253` `jbsr Effects_InstallPreset` | 6 | 4 | same |
   | `ojz_scroll_test` | `ojz_scroll_test.emp:3043` `jbsr Effects_ResolveParallax` | 6 | 4 | same |
   | `parallax` | `parallax.emp:1227` `jbsr Effects_InstallPreset` | 6 | 4 | same |
   | `vblank` | `vblank.emp:503` `jbsr PageIn_Process` | 4 | 6 | `page_in` off the alias slot |

   `ojz_scroll_test` nets -6: the walk measures it 0x2018, packs `replay_fixture` at
   0xBE2D4 + 0x2018 = 0xC02EC.
4. **The declared-span pass measures a different layout.** `declared_spans_by_index` pins EVERY
   ROM section at its true base (never-pinned ones included), where `preset` is at 0x8F5A, out
   of `bsr.w` reach of 0xBE2D4, so the three `jbsr`s are `jsr` abs.l again: 0x201E, and
   `[0xBE2D4, 0xC02F2)` overlaps `replay_fixture` at 0xC02EC. That is the refusal.
5. **The green control was already riding the same seam.** Comparing the walk's final checked
   pass against the declared-span pass in the CONTROL, 15 sections measure differently, and
   every one is SHORTER at its true base (`vblank` 0x210 against 0x20C, `preset` 0xE4 against
   0xD4, ...). A shrink is absorbed (the declared span is the packed gap); a growth is an
   overlap. The design is sound only while the scratch measurement is never shorter than the
   real one, and the alias slot breaks exactly that: whichever section sits on k=9 has every
   far reference into it measured SHORT. In the control that section is `page_in`, whose
   inbound site (`vblank.emp:503`) happens to be in `bsr.w` reach at the real base as well, so
   nothing grows; in red it is `preset`, whose inbound sites are not.

### The code path

- `crates/sigil-frontend-emp/src/lower/mod.rs::lower_module_inner` (the `ast::Item::Ensure`
  arm) and `ensure_default`: the zero-byte label-less section.
- `crates/sigil-harness/src/native.rs::lens_pinned` (through `measure_pinned` and
  `image_lens_pinned`, called by `packed_true_bases`): the far-scratch cursor, k by ordinal,
  crossing the 24-bit wrap at k = 9, 25, 41.
- `crates/sigil-ir/src/width.rs::asl_width_rule`: the 24-bit mask that turns 0x0100_0000 into
  an abs.w address.
- `crates/sigil-harness/src/native.rs::declared_spans_by_index` then `resolve_layout`: the
  pass that measures at real bases and refuses.

### Inferred, not measured

- That the alias is the ONLY way a slot ordinal reaches a width. Pc-relative reach is
  computed on raw addresses (`relax.rs::rung_reaches`, `disp = target - site_vma`, no mask),
  and a far slot is at least 0x70_0000 from any pinned ROM referrer, so no `bsr` reaches one;
  read from the code, not instrumented. The fixed build's byte identity between the pristine
  and red trees (section 5) is the measurement that would falsify it.

### The two hypotheses in the brief

- **Frozen-table seeding (`native::load_frozen_table`)**: not the mechanism. The frozen table
  only decides which sections are pinned; it is the same file, read by the same binary, in
  both runs. The three sections involved (`preset`, `page_in`, the new `text`) have no
  frozen row, which is what puts them on the scratch cursor at all.
- **A forward reference costing an extra pass**: refuted by `v_const_end`, byte-identical to
  the control.

### Prior art

The hazard is booked: `campaign-gap-ledger.md`, row "[BGROOM-3 measure-at-packed-base,
2026-08-26] The legacy far-scratch cursor for never-pinned sections still strides past the
24-bit wrap", and `OVERSEER-LOG.md` "THE FAR-SCRATCH SLOTS ALIAS ACROSS THE 24-BIT BUS"
(2026-08-30), which asked for the count before R2 is priced: **at `cae58661` sonic4 debug,
`page_in` sits on k = 9 and its inbound `jbsr` is measured at the alias**. The `measure_pinned`
doc said the alias was "not a live measuring input, because every FROZEN-labeled section now
measures at a real base in every round". That covered the frozen sections' own lengths, not
the width of a frozen section's reference INTO a never-pinned one, which is where it bit.

## 4. Fix

`crates/sigil-harness/src/native.rs`: `lens_pinned` draws each never-pinned section's slot
through `far_scratch_slot`, which skips any slot whose whole stride (`SCRATCH_STRIDE`,
0x10_0000) does not encode abs.l under `sigil_ir::asl_width_rule`; `next_scratch` refuses
rather than wraps at the top of `u32`. The far scratch keeps its purpose (asl's abs.l width
for a reference into a section outside the frozen table) and loses the one property that
made a slot ORDINAL a width input, so where a zero-byte section sits decides nothing.
Skipped today: every slot whose raw address is a multiple of 0x100_0000 (masked 0x0) and
every slot at masked 0xF0_0000 (its stride reaches 0xFF_8000). The `measure_pinned` doc now
says the slot is a live input for pinned referrers, which is the claim this parcel measured.

Not changed, on purpose: the `.emp` lowering still opens a zero-byte `text` section for a
trailing top-level `ensure`. With the scratch sound it is inert, and suppressing it would
shift every later never-pinned ordinal in the shipped trees, a byte change bought for
nothing.

### Tests (runner: `cargo test -p sigil-harness --lib`, module `native::derived_layout_tests`)

- `a_reference_into_the_far_scratch_measures_abs_l_at_every_slot_ordinal`: a pinned
  `lea T, a0` (`code_abs`) measured with `T`'s never-pinned section behind 0..=41 zero-byte
  label-less sections; expects 6 B (the abs.l form, literal from the 68000 encoding) at every
  ordinal.
- `zero_byte_sections_ahead_of_a_never_pinned_target_move_nothing`: Head | Code (`lea T`) |
  Next | Wide (0x8000) | T, the same 0..=41 zero-byte sections ahead of T; expects the walk's
  bases to be the literals 0x1000, 0x1010, 0x1016, 0x1026, 0x9026 for every count, and
  `declared_spans_by_index` to accept the layout.

**Red-first, before the fix existed** (tests added on top of `dec6dcb0`'s `native.rs`):

```
test native::derived_layout_tests::a_reference_into_the_far_scratch_measures_abs_l_at_every_slot_ordinal ... FAILED
test native::derived_layout_tests::zero_byte_sections_ahead_of_a_never_pinned_target_move_nothing ... FAILED
assertion `left == right` failed: `lea T, a0` measured 4 B with T's never-pinned section at scratch ordinal 9 (behind 9 zero-byte label-less sections): the far scratch must encode abs.l (6 B) at every ordinal, or a pinned referrer's length depends on how many never-pinned sections precede its target
  left: 4
 right: 6
assertion `left == right` failed: with 9 zero-byte label-less sections ahead of the never-pinned `GameLoop`, the walk placed [Head, Code, Next, Wide, T] differently: a zero-byte section moved the layout through the scratch slot `GameLoop` measured at
  left: [Some(4096), Some(4112), Some(4116), Some(4132), Some(36900)]
 right: [Some(4096), Some(4112), Some(4118), Some(4134), Some(36902)]
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 223 filtered out
```

Both fail at ordinal 9 and not before, and the second one is the aeon shape in miniature:
Next packed 2 B early (0x1014) because Code was measured at the alias. With the fix:
`derived_layout_tests` 16 passed, 0 failed, 0 ignored.

## 5. Bytes: the fix moves every shipped shape

### Probe (a binary built from the uncommitted fix; the record run is below)

Method: `emit_sound_blob` then `sigil build --aeon .aeon-sigil-ref --native --game G [--debug]`,
outputs to scratch. **Control of the method:** the unfixed `dec6dcb0` binaries through the same
commands reproduce the provenance tip (`ls12-parallax-discards-refused`, aeon `ec640bcf`) on all
four shapes: s4 `b09ccd65`/820229, s4_debug `1b7fe316`/846529, demo `0ad17404`/96863,
demo_debug `2565ece2`/103185.

| shape | tip (unfixed) | fixed | k = 9 occupant (unfixed) | first moved label | labels by delta | `EndOfRom` |
|---|---|---|---|---|---|---|
| s4 | `b09ccd65`/820229 | `91c46c94`/820209 | `page_in` | `HBlank_Install` 0x241A -> 0x241C | 0: 1208, +2: 1126, +4: 33, +6: 1, +8: 53, +10: 66 | 0xBDC92 both |
| s4_debug | `1b7fe316`/846529 | `8a378de6`/846509 | `page_in` | `HBlank_Install` 0x2544 -> 0x2546 | 0: 1552, +2: 1259, +4: 40, +8: 110, +12: 131 | 0xC14F4 both |
| demo | `0ad17404`/96863 | `1c7a34d3`/96863 | `page_cache` | `Collision_GetType` 0x3FAC -> 0x3FB4 | 0: 1086, +8: 294, +10: 33, +20: 24, +22: 53 | 0x1121A both |
| demo_debug | `2565ece2`/103185 | `72e405a5`/103185 | `page_cache` | `Collision_GetType` 0x5226 -> 0x5230 | 0: 1287, +10: 324, +12: 40, +22: 72, +24: 109 | 0x1121A both |

Every move is upward, every shape's `EndOfRom` is unchanged, and the unmoved labels are the
runs behind a declared anchor: references into the section that sat on the alias slot are now
measured abs.l in the walk, as every other never-pinned target already was, and the packed runs
ahead of the next anchor open by the difference. The ROM SIZE changes (-20 B in both sonic4
shapes) are the deb2 appendix, whose size follows the addresses it records; the assembled image
ends where it did. Which site moves is INFERRED on this tree, not instrumented: in both
sonic4 shapes the first moved label is the one after `vblank`, consistent with the site
measured at `cae58661` in section 3 (`vblank.emp:503` `jbsr PageIn_Process`, 4 B at the
alias, 6 B far); in both demo shapes it is the one after `tile_cache`, consistent with a
far reference into `page_cache`.
