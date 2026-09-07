# 2026-09-07: LS-12 blob re-pin (6163 / 6293 -> 6176 / 6306) and the LS-18 probe repoint

Branch `parcel/ls12-blob-repin` off sigil master 43bf606a. Aeon's
`parcel/ls12-dma-guard-tick-paths` (c1ac4a60) changes
`engine/sound/z80_sound_driver.emp` and grows the resident Z80 blob by 13 bytes in
both shapes; the seam-1 tripwire compiled into `emit_sound_blob` refused that tree
until this re-pin. No emulator was run anywhere in this parcel.

Trees: `.aeon-ls12` = aeon c1ac4a60 (LS-12 tip), `.aeon-ls12-ctl` = aeon 141dff5e
(aeon origin/master, pre-LS-12). Cargo target `/home/volence/sonic_hacks/.target-ls12`.
`build.sh` was run one shape per invocation, foreground, with `SIGIL_BUILD` and
`SIGIL_EMIT` pointed at that target dir. Every shape exited 0 and printed
`Build complete`; four shapes per arm, eight builds total.

## Task 1: confound control (unmodified 43bf606a binaries, ctl tree)

Binaries built from unmodified 43bf606a (`git status --porcelain` empty):
`sigil` b520f2ebbc63fbd92877e72a58f0ff1d, `emit_sound_blob` 67d44bb011025a0577df097db6d53a6b.

| shape          | md5                              | crc32    | size   | aeon's md5 (e6e942e5 binary) | verdict |
|----------------|----------------------------------|----------|--------|------------------------------|---------|
| s4.bin         | 2043e6ad994aa2ae600413685aff62df | 6de2818f | 820229 | 2043e6ad994aa2ae600413685aff62df | match |
| s4.debug.bin   | 3307a21297924a481d48674a8668da20 | e3a3d80c | 846529 | 3307a21297924a481d48674a8668da20 | match |
| demo.bin       | 26b35c26b23754e1223ad55abca09e6e | 0ad17404 | 96863  | 26b35c26b23754e1223ad55abca09e6e | match |
| demo.debug.bin | 177037ab054b3bcc57a00d65eb7fc6cf | 2565ece2 | 103185 | 177037ab054b3bcc57a00d65eb7fc6cf | match |

The relaxation-engine work on sigil master between e6e942e5 and 43bf606a moves none
of aeon's four ROMs. Wall clock 08:03:30 to 08:14:11 for the four builds.

## Task 2: the re-pin, red first (commit 698e053b)

Red. The unmodified emitter against `.aeon-ls12`, exit 1:

    error: emit_sound_blob (seam-1 resident blob) failed: plain blob is 6176 bytes, expected 6163 ($1813), the module bases are DERIVED, so this is the size tripwire, not a placement input: if the size change is intended, re-pin BLOB_LEN_PLAIN and the Z80_SOUND_SIZE mirrors

The plain check short-circuits before the debug one; `SIGIL_BLOB_LEN_DRIFT=warn` on
the same emitter reports both:

    warning: plain blob is 6176 bytes, expected 6163 ($1813), ... [SIGIL_BLOB_LEN_DRIFT=warn]
    warning: debug blob is 6306 bytes, expected 6293 ($1895), ... [SIGIL_BLOB_LEN_DRIFT=warn]

and the emitted files measure 6176 B (`z80_sound_blob.bin`) and 6306 B
(`z80_sound_blob_debug.bin`). Delta stays $82.

Edits (three sites):

- `crates/sigil-harness/src/seam1.rs`: `BLOB_LEN_PLAIN` 0x1813 -> 0x1820,
  `BLOB_LEN_DEBUG` 0x1813 + 0x82 -> 0x1820 + 0x82; comments state both lengths even,
  no pad, headroom under the $18F0 ceiling 78 B (the ceiling is prose in
  `repin_pins.rs`, not an enforced constant).
- `crates/sigil-cli/tests/seam1_native_link.rs`: the literal in
  `blob_lengths_are_canonical` 0x1813 -> 0x1820, message text 6176 B; the parity
  comment now says both lengths are even.
- `crates/sigil-cli/tests/boot_port.rs` `frozen_symbol`: `Z80_SOUND_SIZE` is the blob
  length rounded up to even; both new lengths are even so the round-up is the
  identity: plain 0x1814 -> 0x1820 (6176), debug 0x1896 -> 0x18A2 (6306).

Green. Rebuilt emitter against `.aeon-ls12`: exit 0, 6176 / 6306.

Post-LS-12 ROMs (`.aeon-ls12`, re-pinned binaries), the shapes aeon's `build.sh` will
produce after merging LS-12 with this sigil:

| shape          | md5                              | crc32    | size   | vs control |
|----------------|----------------------------------|----------|--------|------------|
| s4.bin         | 7d1863b1b60098208b84895cb92e2015 | b09ccd65 | 820229 | moved (blob +13 B) |
| s4.debug.bin   | 019e3538367d00e0d5d6e1278f10d621 | 1b7fe316 | 846529 | moved (blob +13 B) |
| demo.bin       | 26b35c26b23754e1223ad55abca09e6e | 0ad17404 | 96863  | identical (demo ships no sound blob) |
| demo.debug.bin | 177037ab054b3bcc57a00d65eb7fc6cf | 2565ece2 | 103185 | identical |

ROM file sizes do not change; the blob growth is absorbed inside the padded image.

`AEON_DIR=.aeon-ls12 cargo test --release --no-fail-fast -p sigil-cli --test seam1_native_link --test boot_port`:

- boot_port: 0 passed, 2 failed. `boot_region_matches_reference` and
  `boot_debug_region_matches_reference` (boot_port.rs:394), first diff at region
  offset 0x6b: candidate `18 1F` / `18 A1` vs golden `18 13` / `18 95`. That is the
  `#Z80_SOUND_SIZE-1` immediate, i.e. the new values minus one against goldens
  frozen before LS-12. Expected until the chain refreeze.
- seam1_native_link: 5 passed, 2 failed. `native_blob_matches_reference_plain` and
  `native_blob_matches_reference_debug` (seam1_native_link.rs:77): the LS-12 blob vs
  the pre-LS-12 golden ROM window, first diff at blob offset 0x39. Expected until the
  chain refreeze. `blob_lengths_are_canonical` and `module_bases_are_a_gapless_cursor`
  pass against the LS-12 tree.

Without `--no-fail-fast`, cargo stops after boot_port fails and seam1_native_link
never runs; the totals above are from the no-fail-fast run.

## Task 3: LS-18 probe repoint (commit 9af27c83)

`crates/sigil-cli/tests/tranche6_negative_probes.rs`: both objroutine probes now read
`games/sonic4/test/fixtures/sigil_objroutine_probe.emp` (aeon 6fd02d75) instead of the
live `games/sonic4/objects/test_solid.emp`. The guard is the house
`test_support::reference_tree` naming types.emp, sst.emp and the fixture;
`test_objects_port.rs` and `objdef_port.rs` are untouched.

- ctl tree (fixture present), strict and non-strict: 3 passed, 0 failed.
- `.aeon-ls12` (fixture absent), strict: exit 101, both probes panic
  `SIGIL_STRICT_GATE set but reference missing: /home/volence/sonic_hacks/.aeon-ls12/games/sonic4/test/fixtures/sigil_objroutine_probe.emp`,
  1 passed / 2 failed. Non-strict: exit 0, `skip: reference not at ... (set AEON_DIR)`,
  3 passed.
- Red-first on the refusal itself: with line 48 mutated on disk to
  `..._probe_MISSING.emp`, strict against the ctl tree fails naming
  `.../sigil_objroutine_probe_MISSING.emp` (1 passed / 2 failed); restored from the
  committed baseline, `git status --porcelain` empty.

## Left open (controller)

- The chain refreeze (`pins.rs`, `repin.toml`, `golden/`, `provenance.toml`) after aeon
  merges LS-12; it closes the four golden-lag failures above and regenerates the
  stale `pins::TEST_SOLID.plain_len`.
- The full sigil suite (cross-seam change, the controller's landing gate).
- Runtime confirmation of the LS-12 ROMs is aeon's; nothing here ran an emulator.
