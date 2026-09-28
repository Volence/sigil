# SEAM2-TABLE-REGION-UNENFORCED: measurement (2026-09-28)

Branch `parcel/seam2-region-unenforced`, cut from master `cde38353`. Measurement only, no fix.
Measurement test: `crates/sigil-harness/tests/seam2_region_unenforced.rs` (commit `a848dbac`).
Aeon trees read: `/home/volence/sonic_hacks/.aeon-seam2-region-4523ca5f` (my own detached worktree at aeon
`origin/master` `4523ca5f`, which contains `39a2b4dd`; clean before and after every run) and the standing reference
`/home/volence/sonic_hacks/.aeon-sigil-ref-1ee78b88` (read only; clean before and after).

## Verdict

Aeon's observation reproduces: the real `sound_tables_z80` is 1,116 B (0x45C) at aeon `4523ca5f`, and
`seam2::emit_sound_tables_z80` returns Ok through its synthetic `size = 0x400` region. At the reference `1ee78b88`
it is 985 B, under the literal. The growth is aeon commit `70027733` (PSG envelope `$4C`, +1 id byte, +2 pointer
bytes, +128 body bytes = +131 B).

It is a sigil-side gap in a check that cannot fail, not a silent ROM overflow. The 0x400 region is a scratch map
used to lower and link the tables module in isolation; its size is read by nothing on that path. The tables' real
placement in the ROM is length-derived and bounded elsewhere, and that bound is enforced. Separately, the
measurement found two shapes in which `emit_rom` itself accepts a by-name region overflow (question 1, cases E and
F). Neither is reached by aeon's build today, but they are real linker gaps for any `--map` user.

## 1. Does placement refuse, warn, or accept?

`sigil_frontend_emp::resolve::place_sections` (not `sigil_link`, as the brief had it;
`crates/sigil-frontend-emp/src/resolve/mod.rs:1000`) never compares a section's span with its region's `size`.
Its doc says so: "region-budget overflow is caught later by `emit_rom`/`validate_section` (§7.3)". It returns no
diagnostic of any level for an overflow. `sigil_link::load_map` (`crates/sigil-link/src/map_load.rs:30`) only
parses; it checks nothing about sizes or overlap between regions.

The enforcement is `MemoryMap::validate_section` (`crates/sigil-ir/src/map.rs:135`), called only from
`sigil_link::emit_rom` (`crates/sigil-link/src/lib.rs:1042`). It looks the region up BY LMA
(`region_for(lma)`, the first `Rom`-kind region containing the LMA), not by the region the section was named into.

Measured (all through the real functions, sections of raw data bytes):

| case | shape | `place_sections` | `resolve_layout`+`link` (the seam-2 tail) | `emit_rom` |
|---|---|---|---|---|
| A | region 0x400, one section 0x401 | silent (no diag) | Ok | refuses: "section `sound_tables_z80` [0xB8000,0xB8401) overflows region `sound_tables_z80` (ends 0xB8400), over by 1 bytes" |
| B | region 0x400, one section 1,116 B | silent | Ok, 1,116 B linked | (refuses, as A) |
| C | region 0x400, one section exactly 0x400 | silent | Ok | accepts |
| D | region 0x400, sections 0x300 + 0x200 | silent | Ok | refuses, over by 256 bytes |
| E | region `a` [0x1000,+0x400) and `b` [0x1400,+0x400); two sections named `a`, 0x400 + 0x10 | silent; the spill is placed at 0x1400 | Ok | ACCEPTS: the spill is validated against `b` |
| F | `rom` [0,0x400000) listed first, `sound_tables_z80` [0xB8000,+0x400); one section 1,116 B | silent | Ok | ACCEPTS: validated against `rom` |

Case D refuses only because the second section starts inside the region. Case E is the multi-section case where the
first section exactly fills the region: the overflow is then attributed to whichever region contains the spill's
start. `flatten_checked` catches it only if the neighbouring region already holds bytes at that address. Case F is
the shape of `games/sonic4/map.toml`, whose first region is `rom` [0, 0x400000).

Command and output (baseline `a848dbac`):

```
$ CARGO_TARGET_DIR=$PWD/.target cargo test -p sigil-harness --test seam2_region_unenforced
test measure_real_sound_tables_z80_through_seam2 ... ignored, needs SEAM2_REGION_AEON=<aeon tree>; run explicitly with --ignored
test gap_place_sections_is_silent_on_single_section_overflow ... ok
test gap_seam2_tail_links_an_overflowing_section_with_no_diagnostic ... ok
test enforced_emit_rom_refuses_single_section_overflow_by_one ... ok
test enforced_emit_rom_refuses_two_sections_summing_past_the_region ... ok
test gap_emit_rom_accepts_a_spill_that_starts_exactly_at_the_next_region ... ok
test enforced_emit_rom_accepts_exact_fill ... ok
test gap_emit_rom_accepts_overflow_of_a_region_nested_in_an_earlier_rom_region ... ok
test result: ok. 7 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
```

The tests pin current behaviour, gaps included (`gap_*` names a gap, `enforced_*` names enforcement). They were
shown able to fail:

- Mutation 1, on disk in `crates/sigil-frontend-emp/src/resolve/mod.rs` after the cursor advance:
  `if *cursor > region.size { diags.push(Diagnostic { level: Level::Error, message: format!("MUTATION overflow `{}`", sec.name), .. }); }`
  Result: `FAILED. 1 passed; 6 failed` (every test that asserts `place_sections` is silent on an overflow; only
  `enforced_emit_rom_accepts_exact_fill` stays green, correctly).
- Mutation 2, on disk in `crates/sigil-ir/src/map.rs`: `if end > region_end {` became
  `if false && end > region_end { // MUTATION`. Result: `FAILED. 5 passed; 2 failed`
  (`enforced_emit_rom_refuses_single_section_overflow_by_one`, `enforced_emit_rom_refuses_two_sections_summing_past_the_region`).
- Both restored with `git checkout --` from the committed baseline; `git status` clean of tracked changes; rerun
  `ok. 7 passed; 0 failed; 1 ignored`.

## 2. Why the seam-2 path did not refuse

Not because the path is unreached, not because the section is a different one, not a filtered diagnostic, and the
1,116 B figure is the right section. The cause is structural: the seam-2 emitter
(`sound_tables_z80_linked_from_src`, `crates/sigil-harness/src/seam2.rs:1187`) runs `place_sections` +
`resolve_layout` + `check_link_asserts` + `link` and never `emit_rom`, so the one function that reads a region's
`size` is never called. There is no diagnostic to filter: `place_sections` produces none (case B asserts the diag
list is empty, not merely error-free).

Real-tree measurement through the real emitter (`seam2::emit_sound_tables_z80`, which is the same core the
artifact writer uses):

```
$ SEAM2_REGION_AEON=/home/volence/sonic_hacks/.aeon-seam2-region-4523ca5f cargo test -q -p sigil-harness \
    --test seam2_region_unenforced -- --ignored --nocapture
SEAM2_REGION_MEASURE aeon=/home/volence/sonic_hacks/.aeon-seam2-region-4523ca5f sound_tables_z80_len=1116 region_size=1024
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out
$ SEAM2_REGION_AEON=/home/volence/sonic_hacks/.aeon-sigil-ref-1ee78b88 ... (same)
SEAM2_REGION_MEASURE aeon=/home/volence/sonic_hacks/.aeon-sigil-ref-1ee78b88 sound_tables_z80_len=985 region_size=1024
```

Under mutation 1 the same run at `4523ca5f` fails with
`place_sections errors: [Diagnostic { level: Error, message: "MUTATION overflow `sound_tables_z80`", .. }]`, and at
`1ee78b88` still passes at 985 B.

The brief's hypothesis that the path is harness-only is wrong: seam2.rs is linked into the product.
`sigil build --native` for every sound-on profile calls `native::emit_generated_in`
(`crates/sigil-harness/src/native.rs:3835` in `resolve_chained`, and `:4075`), which calls
`seam2::emit_sound_tables_artifacts_in`; aeon's `build.sh` also runs the `emit_sound_blob` preflight, which reaches
the same emitter. So the dead literal is in product code. What it would have guarded is not a ROM boundary.

History: the literal came in with `52b62809` (2026-07-30, "seam-2 stage-3: emit + gate sound_tables_z80"), when the
table was frozen at `SOUND_TABLES_Z80_LEN = 0x357` and byte-gated; 0x400 was headroom in a scratch map. The table
has since become length-derived (aeon's own header in `engine/sound/sound_tables_z80.emp`: "the table may grow: no
consumer restates its length or a label address"), and the literal was never revisited because it never fired.

## 3. Is the real product exposed?

Not for the sound tables, on the evidence read here. The tables' bytes reach the ROM as
`embed("engine/sound/generated/sound_tables_z80.bin")` in `games/sonic4/data/sound/soundbankhead.emp`, placed by the
whole-ROM layout, not by the 0x400 map. Everything after the tables in the head bank is placed from the tables'
measured length (`seam2::sound_layout_in`: `pitchtable_lma = sound_tables_z80_lma + l_stz`, and so on down the
heads, `mt_bank`, `sfx_bank`), so growth moves the followers rather than overwriting them. The bank itself is
bounded by what IS enforced:

- the `song_bank_2` anchor at `0xC0000` (the `sound_bank` window end) is held absolute by the declared-chain packer;
  a run that overruns it fails at the final `resolve_layout` overlap check (`native.rs` around line 2708; unit test
  `growth_into_a_declared_anchor_still_fails_loud` run on this baseline: `ok. 1 passed`);
- `sfx_bank.emp:176` `ensure(bankid(Sfx_33) == bankid("MovingTrucks_Bank_Start"))`;
- the seam-2 SFX path refuses a base above the bank top (`checked_sub` at `seam2.rs:948`) and `mt_bank_room`
  refuses a predicted base outside the window.

What the product does not enforce, found on the way: the sonic4 map's `z80_sound_bank` region
(`kind = "z80_bank"`, size 0x8000) is parsed and read by nothing (`RegionKind::Z80Bank` appears only in the loader
and the label function), and the whole-ROM `emit_rom` validates every section against `rom` [0, 0x400000) because
that region is listed first (case F). So the per-bank ceiling rests on the anchor adjacency and the ensures above,
not on the region declaration. I did not run a whole-ROM build to show the anchor catching a sound-bank overrun;
that claim rests on the cited code and the unit test.

The `--map` CLI path (`crates/sigil-cli/src/main.rs:1771`, `place_sections` then `emit_rom`) is exposed to cases E
and F for any map with adjacent same-kind regions or a covering region listed first. Aeon's build does not use that
path.

## 4. Classification

- Sigil, product code, not aeon-side. A check that cannot fail: the `size = 0x400` in
  `seam2::sound_tables_z80_linked_from_src` is read by nothing on that path. Same class, same file: every seam-2
  synthetic map (`seam2.rs` lines 530, 736, 963, 1047, 1204, 1378, 1562) goes through `place_sections` + `link`
  without `emit_rom`, so none of their region sizes is enforced, including `sfx_bank` and `mt_bank`, whose sizes
  are real bank-room derivations and whose comments ("the ceiling every in-bank artifact must fit under") read as
  if enforced. Those two are covered downstream by the anchor and the ensures; the literal ones guard nothing.
- Not a silent ROM overflow: no failure scenario exists today in which the 1,116 B tables corrupt the ROM.
- A linker defect of its own, in `validate_section`'s by-LMA lookup (cases E and F): exact scenario, a map declares
  region `a` then an adjacent region `b` (or a covering `rom` region first), a program's `a`-named sections total
  more than `a.size` with the first exactly filling it (or any overflow, for the covering case), `b` holds nothing
  at the spill address; `sigil build --map` emits the ROM with the spill inside `b` and no diagnostic.

## Fix options and population (not applied)

Census method: a third mutation, logging every region fill in `place_sections`
(`eprintln!("CENSUS region={} size={:#X} used={:#X} over={}", ..)`), then the `emit_sound_blob` binary against my
aeon tree `4523ca5f` with `--out-dir` in this worktree's `.scratch/` (exit 0, aeon tree clean after). Every seam-2
synthetic region fill for all shapes the preflight emits:

```
4 region=sound_tables_z80 size=0x400 used=0x45C over=true
3 region=sfx_blob_win_tab size=0x200 used=0x112 over=false
3 region=seq_opcode_tab size=0x100 used=0x40 over=false
2 region=mt_bank size=0x78D0 used=0x6900 over=false
2 region=mt_bank size=0x78D0 used=0x4EB8 over=false
2 region=movingtrucks_pitchtable size=0x200 used=0x108 over=false
2 region=dac_shared_bank size=0x8000 used=0x649A over=false
2 region=dac_sample_tab size=0x100 used=0x78 over=false
2 region=dac_blip_bank size=0x8000 used=0xB40 over=false
1 region=sfx_bank size=0xFD0 used=0x644 over=false
1 region=sfx_bank size=0x6000 used=0x644 over=false
1 region=sfx_bank size=0x2A18 used=0x644 over=false
16 region=text (0x40 / 0x10) used=0x0 over=false
```

Only `sound_tables_z80` is over. The mutation was restored from the baseline afterwards.

1. Make `place_sections` refuse (Error) when a region's cumulative span exceeds its `size`. Closes the class for
   every caller at once, including case E (by-name accounting). Population: the seven seam-2 maps (only
   `sound_tables_z80` fires on aeon master today, in every sound-on shape (the tables are shape-invariant and emitted whenever `profile.sound_on`; map.toml lists s4, s4_debug, config_a, lean);
   config_b is sound-off and never emits it); the CLI `--map` path (already mostly caught by `emit_rom`; would gain
   case E); `native.rs:1941` `emp_map_frozen`, whose regions are 0x400000 per distinct name and whose comment
   ("`place_sections` does not enforce the size, so the huge size never overflows") depends on today's behaviour.
   It cannot fire short of a 4 MB name. WOULD BREAK aeon master's build unless the tables literal changes in the
   same landing.
2. Replace the literal with the real bound: size the `sound_tables_z80` region from the bank room
   (`sound_bank + 0x8000 - head_lma`, as `mt_bank_room` does), and likewise for the other literal regions
   (`sfx_blob_win_tab`, `seq_opcode_tab`, `movingtrucks_pitchtable`, `dac_sample_tab`), or drop the pretence and
   name them scratch. Without option 1 this changes nothing observable; with it, the census shows nothing fires.
3. Make `validate_section` look up the region the section was placed into (its `group`) when that names a region in
   the map, falling back to by-LMA otherwise. Closes cases E and F for `--map`. The whole-ROM native `emit_rom`
   would be unaffected only if its sections' `group` names (set from the frozen placement map) do not match regions
   in the sonic4 map: not measured, needs a whole-ROM run before landing.

The byte output does not change under any of these for a build that passes today. The recommended order is 2 then 1
in one landing (so no build starts refusing), with 3 as a separate parcel carrying its own whole-ROM measurement.
