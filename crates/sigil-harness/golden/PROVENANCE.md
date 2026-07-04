# Golden provenance — Sigil M0 acceptance gate

These committed golden blobs certify that Sigil reproduces the Aeon Z80 sound
driver byte-for-byte:

- `region_a.bin` / `region_b.bin` — the reference regions sliced from a fresh
  `asl` build of `aeon/s4.bin` (region A `[Z80_Sound_Start:Z80_Sound_End]`,
  region B `[MovingTrucks_Bank_Start:Song_MovingTrucks]`).
- `sigil_a.bin` / `sigil_b.bin` — Sigil's own assembled output for the same two
  regions. They are **byte-identical** to the reference blobs (that identity is
  the M0 acceptance gate).
- `windows.toml` / `stub-syms.toml` — the extraction windows and the 68k leaf
  stub set, both re-derived from the fresh build's `s4.lst`.

## The reference is a moving target (read this before trusting a stale blob)

Aeon is under **active development**, and `aeon/build.sh` re-runs code/data
**generators** (sfx transcode, BG-tile/ojz blob gen, …) on every build, so the
reference ROM's byte layout drifts as Aeon changes — the resident driver region
has been observed at both `0x1720` (5920 B) and `0x1708` (5896 B). **Sigil has
been verified byte-identical to the `asl` reference at every observed state** —
the drift is in the Aeon source, not in Sigil.

Consequently:

- The **hermetic** test `m0_acceptance_sigil_matches_reference_blobs` compares
  these committed `sigil_*.bin` vs `region_*.bin` — it certifies *snapshot
  consistency* (Sigil-output == asl-reference at capture time), and stays green
  regardless of later Aeon drift.
- The **live** gate — `harness_assembles_regions_a_and_b_together` (`--ignored`),
  the `sigil diff` CLI, and `regen` — assembles against the **current** Aeon tree
  and will report a length/first-diff mismatch if these blobs are stale relative
  to it. That is expected when Aeon has moved; it is **not** a Sigil regression.

## Provenance of this snapshot

- Captured during Plan 5 (Sigil M0). Aeon repo HEAD at merge time:
  **`0ac34034829cc0a9f993413266f2e2f109b3a980`** (working tree carried unrelated
  BG/editor WIP; the sound-driver source was stable for the capture).
- Sigil branch: `sigil-m0-p5-integration-harness` (merged to `master`).

## To refresh these goldens against the current Aeon

```
cargo run -p sigil-harness --bin regen
```

`regen` rebuilds Aeon, re-derives the windows + stub set from the fresh `.lst`,
re-slices the reference regions, re-assembles with Sigil, and **exits non-zero
if Sigil diverges from the reference** (writing both blobs for diffing). A clean
run overwrites these files with a fresh, self-consistent snapshot. Commit the
result to re-baseline. (M1 replaces this whole snapshot dance with a full-ROM
`sha256` gate once the 68000 backend assembles the real 68k data.)

## M1.B reference-ROM pin (acceptance gate)

The `m1b_gate` integration test (`crates/sigil-harness/tests/m1b_gate.rs`) checks
the linker's byte-mutating passes against the live Aeon reference ROM. The pin
below records the exact reference the gate was validated against. This is a
**moving target** — the Aeon ROM (and thus its stored checksum) changes as the
engine evolves; re-baseline this pin *deliberately* when intentionally tracking a
new Aeon build, and re-confirm the gate passes against it.

- Aeon repo commit (reference): **`9bacc939ae7c7c5300fc7e50548d851373128a23`**
- `aeon/s4.bin` length: **458666 bytes** (`0x6FF2A`)
- Stored header checksum at `0x18E`: **`0x5CBE`**

`header_checksum_reproduces_reference_rom_18e` does not hardcode `0x5CBE`: it
reads the stored word from the reference ROM, zeroes `0x18E`, recomputes via
`sigil_link::apply_header_checksum`, and asserts equality — so the gate stays
correct across re-baselines as long as the Sega checksum algorithm holds. The
values above are the observed pin at the time this gate was written.
