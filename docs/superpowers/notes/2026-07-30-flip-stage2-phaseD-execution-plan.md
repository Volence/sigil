# 2026-07-30 — FLIP STAGE 2 · PHASE D execution plan (rulings in hand; ready to execute)

Status: **the overseer ruled Q1/Q2/Q3 (row-91 witness = independent recomputation;
tranche family retires coverage-subsumed; the reassembly harness fns delete with it).
The Phase-D transformation is FULLY MAPPED (below) and ready to execute. Stopping at the
green boundary (aeon `97a9127` / sigil `08d6a62`, strict 2939/0/1) with this plan rather
than rushing the delicate row-91 witness construction — the Volence-protected coverage —
at a long session's tail. This note is the complete recipe; no re-investigation needed.**

## The ruled shape (Q1/Q2/Q3)

- **Row-91 witness = the EXISTING per-bank composition tests** (they compose `.emp` in
  memory via lower→place→link and slice-compare a golden ROM — they SURVIVE, they are NOT
  the tranche scaffolding). Build them to bars (a)-(d): (a) recompute from `.emp` via the
  composition path, (b) assert vs FROZEN GOLDEN slice, (c) t24 doctor a `.emp` input →
  diverge, (d) cover EVERY `.emp`-sole-sourced bank.
- **Retire** the twin-inclusive AS-reassembly oracles (coverage subsumed by the native
  whole-ROM golden gates + the surviving `.emp`-region gates + the row-91 witness).
- **Delete** the reassembly harness fns with the collapse.

## D1 — the ROW-91 WITNESS (per-bank, bars a-d). Delicate; do FIRST (before retiring the
scaffolding that carries some banks' t24).

The surviving witness files and their banks (composition fn → golden LMA):

| bank | witness test file | composition fn | golden LMA (plain/debug) | has t24? |
|---|---|---|---|---|
| resident blob | `seam1_native_link::native_blob_*` | `seam1::native_blob_doctored` | `$3DE`/`$3E2` | YES (`blob_diverges_when_{banked_carrier,const}_doctored`) |
| DAC blip body | `dac_port` + `seam2_dac_head_colink::colink_banks_still_match_reference` | `seam2::emit_dac_body_and_head` | `$48000` | NO — ADD |
| DAC shared body | same | same | `$50000` | NO — ADD |
| DAC head `DacSampleTable` | `seam2_dac_head_colink::colinked_dac_head_matches_the_reference_rom_slice_both_shapes` | `emit_dac_body_and_head` | `$585AD` | NO — ADD |
| MT bank | `mt_port::mt_bank_region_matches_reference{,_debug}` | `emit_mt_bank` | `$58607` | YES (`mt_negative_probes`) |
| SFX body | `sfx_port::sfx_bank_region_matches_reference{,_debug}` | `emit_sfx_body_and_head` | `$5BAE8`/`$5D53A` | YES (`sfx_negative_probes`) |
| SFX head `SfxBlobWinTab` | `seam2_sfx_head_colink` | `emit_sfx_body_and_head` | `$5845F` | NO — ADD |
| `seq_opcode_tab` | `seam2_seq_colink` | `emit_seq_opcode_tab` | `$5856D` | NO — ADD |
| `sound_tables_z80` | `seam2_soundtables_colink` | `emit_sound_tables_z80` | `$58000` | NO — ADD |
| `movingtrucks_pitchtable` | `seam2_pitchtable` | `emit_pitchtable` | `$58357` | NO — ADD |

Steps:
1. **(bar b) Re-point** every witness's comparand from live `aeon/s4.bin` → the FROZEN
   `crates/sigil-harness/golden/s4.bin` (+ `s4.debug.bin`). Byte-identical NOW (golden ==
   the restored asl `s4.bin`), so green; it makes the witness independent of the
   now-sigil-built tree ROM (else compose-`.emp`-vs-sigil-built = circular). Use a
   `golden(name)` helper reading `env!("CARGO_MANIFEST_DIR")` (harness: `golden/`; cli:
   `../sigil-harness/golden/`), mirroring `native_offcanonical_rom.rs`.
2. **(bar c) Add the missing t24 doctored probes** for DAC body+head, SFX head,
   seq_opcode_tab, sound_tables_z80, pitchtable. The clean pattern is
   `seam1::native_blob_doctored(aeon, debug, doctor: Option<(&str,i64)>)` — a composition
   fn that recomposes from a `.emp`-level doctored input and returns bytes. ADD analogous
   doctored variants to `crates/sigil-harness/src/seam2.rs` for `emit_dac_body_and_head` /
   `emit_seq_opcode_tab` / `emit_sound_tables_z80` / `emit_pitchtable` /
   `emit_sfx_body_and_head` (doctor one input const/byte), then each witness asserts the
   doctored composition != the golden slice. (The current `seam2_*_rom` diverge probes
   doctor the emitted `.bin` then assemble the twin ROM — that path RETIRES, so the t24
   must move to the composition witness.) This is the careful construction — do it well.
3. Note the witness mapping (which `mixed_dac_rom` assertions were composition vs tranche
   scaffolding) in the commit, per Q1.

## D2 — retire the AS-reassembly oracle family + delete the harness fns (Q2/Q3).
AFTER D1 (so no bank loses t24). Removing passing tests + now-uncalled fns → green.

RETIRE (whole file, `git rm`): `crates/sigil-harness/tests/{m1d_rom,m1d_debug_rom,
m0_regions,mixed_dac_rom,mixed_offcanonical_rom}.rs`;
`crates/sigil-cli/tests/{seam2_dac_rom,seam2_mt_rom,seam2_sfx_rom,test_t1_harness_states_port}.rs`.

RETIRE (partial — drop only the twin-inclusive tests, keep the witness/region tests):
- `seam1_native_link.rs`: drop `mixed_seam1_rom_matches_reference_{plain,debug}` +
  `mixed_seam1_rom_diverges_when_blob_doctored` + the `build_seam1_rom` helper (:166) +
  the `assemble_mixed_z80sound_as_side` import. KEEP `native_blob_*`, `blob_*`,
  `handler_symbol_contract_complete`.
- `vblank_port.rs`: drop `vblank_sound_off_twin_parity_{plain,debug}`,
  `vblank_mirror_shape_twin_parity`, `as_full_module` (:256), `run_twin_parity` (:286) +
  dead helpers. KEEP the two region tests.
- `boot_port.rs`: drop `boot_sound_off_twin_parity_{plain,debug}`,
  `boot_hotkeys_shape_twin_parity`, `as_full_module`, `oracle_value`, `run_twin_parity`
  (the SAME edit I proof-tested earlier — head -n 367 + the coverage note). KEEP the 3
  golden-backed tests.
- `m1c_vector_table.rs`: NOT retired now — it did NOT break on the compression deletion
  (its `m1c_root.asm` fixture does not include the compression twins). It retires when
  the twins IT includes (vectors/boot) delete in their groups.

DELETE harness fns (`crates/sigil-harness/src/lib.rs`, ranges from the inventory):
`assemble_full_rom` (71-78), `assemble_full_rom_debug` (79-84), the private
`assemble_full_rom_with` (85-112), all `assemble_mixed_*_as_side` (113-747 for tranches
2-9 + dac/mt/sfx/hblank; 890-1239 for tranche 20-41; `assemble_mixed_z80sound_as_side`
1240-1264; `assemble_mixed_error_handler_as_side` 1337-1350), all
`assemble_seam2_*_rom_as_side` (1265-1336). KEEP `region_at_lma`,
`derive_convsym_rewritten`, `assert_rom_matches_convsym`, `assert_rom_matches`, the LMA
consts.

HANDLE non-test callers of the deleted fns:
- `crates/sigil-cli/src/main.rs`: `run_build` legacy no-`--native` path (:821) and
  `run_diff` (:1022,:1030) use `assemble_full_rom`. Post-flip there is no all-AS build —
  make `--native` implicit (drop the legacy branch) OR delete `run_diff` (its job — sound
  regions == asl — is subsumed by the seam witnesses). Recommend: drop the legacy
  `run_build` branch (native is the only build) and retire `run_diff`.
- `crates/sigil-harness/examples/{emit_s4_rom,diff_s4_debug,m1c_rom,m1c_full}.rs` — delete
  or port off `assemble_full_rom*` (they are dev examples, not gates).

## D3 — the first twin deletion (compression) — the exact edit is proven.
`git rm engine/compression/{s4lz_decompress,zx0_decompress}.asm`; collapse the two
`engine.inc` gates (lines 231-260) to the bare resume-`org` blocks (I already wrote the
present-tense comment + verified the native build stays byte-identical — CRC 2198deb2/
1d895fcb held). After D2 this is green (the twin-inclusive oracles are gone).

## D4+ — the remaining per-subsystem twin deletions + the mass comparand re-point.
- The ~50 OTHER `.emp`-region gates (GROUP C inventory) read `aeon/s4.bin`; re-point them
  to `golden/s4.bin` too (byte-identical, removes the "keep asl artifacts restored"
  crutch). Mechanical; can be its own commit or ride each subsystem group.
- `boot_port` `s4.lst` symbol lookups (`Z80_SOUND_SIZE`/`GAME_ENTRY_ID`/`Game_Entry`,
  `listing_symbol` :67): pin the values or read from the golden — needed once the tree's
  `s4.lst` is sigil-canonical (GROUP D).
- Then the twin deletions proceed per-subsystem (engine/system, objects, level, debug,
  player, game objects, test/data + keystones + vectors), each byte-proven by the native
  gates + strict green, per the design §3.2 enumeration.

## Per-commit bar (unchanged)
Both games' native builds at their pinned CRCs (native gates) + strict worktree pair
green, failures-first, explicit counts, after EVERY commit. t24 verbatim on every
surviving golden gate + the row-91 witness. The close-packet keeps the itemized per-class
test accounting (retired / transformed / added counts).
