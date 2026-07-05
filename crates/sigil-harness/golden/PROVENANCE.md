# Golden provenance — Sigil reference gates

## M0 acceptance — the Z80 sound driver (RE-EXPRESSED in M1.D T6)

The M0 milestone proved Sigil reproduces the Aeon Z80 sound driver (Region A =
the resident phase-0 driver; Region B = the phase-`08000h` Moving-Trucks / SFX
bank) byte-for-byte. It was originally gated by a **bounded harness** that
assembled those two regions *in isolation*, stubbing the ~42 68k leaf symbols the
driver referenced but that the isolated build did not define
(`golden/stub-syms.toml`, re-derived by the `regen` bin from a fresh `s4.lst`).

**That whole apparatus was retired in M1.D T6.** Sigil now assembles the entire
`main.asm` include tree byte-exact with **zero stubs** (the `m1d_rom` gate), so
the sound-driver regions fall out of the full build directly. The following were
deleted: `harness_root.asm`, `golden/{stub-syms.toml,windows.toml,region_a.bin,
region_b.bin,sigil_a.bin,sigil_b.bin}`, the `regen` bin, and the
`build_harness`/`assemble_reference_regions` lib helpers.

M0 acceptance is now the `m0_regions` gate
(`crates/sigil-harness/tests/m0_regions.rs`): it runs the full non-debug build
(no stubs), locates the linked sections at LMA `0x3EA` (Region A) and `0x60000`
(Region B), and asserts each is byte-identical to the corresponding window of the
**live** `aeon/s4.bin`. Region lengths are read from the live sections (not
pinned), so the gate tracks driver growth automatically. It is reference-gated
(needs the sibling `aeon` tree) and self-tracks Aeon drift the same way
`m1d_rom` does — no committed golden blob to go stale.

To run: `SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness
--test m0_regions`.

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

## M1.D reference pin (T0.0, 2026-07-04) — the authoritative full-ROM target

Re-pinned at the start of M1.D. **The M1.B pin above (458666 B / `0x5CBE`) was
captured with a DIRTY aeon working tree** (BG/editor WIP carried in, per the M0
snapshot note). A *clean* `9bacc93` produces a different, smaller ROM — the
build is non-hermetic (`build.sh` runs python generators that consume the editor
JSON), so the reference depends on the working-tree state, not just the commit.
This pin is the reproducible clean-tree reference every M1.D gate measures
against:

- Aeon repo commit: **`9bacc939ae7c7c5300fc7e50548d851373128a23`**, working tree
  **clean** (uncommitted forest_bg_gen/editor experiments stashed at pin time).
- `aeon/s4.bin` length: **450878 bytes**
- sha256: **`605631da01e2fb889d0babfebf8f1341f86a0fba0e63286cbc0671f068ad5117`**
- Stored header checksum at `0x18E`: **`0xcfc3`**

Regen against this pin is byte-identical: region A **5896 B**, region B **1543 B**
(both MATCH). To reproduce: stash any aeon WIP → `./build.sh sonic4` → `cargo run
-p sigil-harness --bin regen`.
