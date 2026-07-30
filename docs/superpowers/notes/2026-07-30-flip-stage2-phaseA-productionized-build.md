# 2026-07-30 — FLIP STAGE 2 · PHASE A: the productionized native build path (pre-deletion)

Status: **PHASE A DONE and proven; Phase B ruled a Phase-C companion (below). The
productionized path is additive + fully reversible — asl stays the default; NOTHING
deleted. Checkpoint 1 to the overseer; the no-return line (Phase C) is NOT crossed —
it waits for the overseer's mid-parcel countersign.**

Branch tips: sigil `flip-stage2` (this commit on `ebe86ae`); aeon `flip-stage2`
(build.sh commit on `28098af`).

## What landed (Phase A — item 1 of the parcel)

The native driver is now the CLI build, driving the SAME code path the native gates
bank (so a green gate vouches for the bytes build.sh ships):

- **`crates/sigil-cli/src/main.rs`** — `sigil build --aeon <dir> --native [--game
  sonic4|demo] [--debug] [--config-a|--config-b] [-o <out>] [--emit-lst <lst>]`.
  Selects the target's `GameProfile`, gets `(rom, listing)` from its driver (the
  PINNED `build_native_rom_with_listing` for canonical sonic4 — the `native_full_rom`
  gate path; the declared-order `build_rom_chained_with_listing` for the off-canonical
  shapes — the `native_offcanonical_full` gate path), then the `convsym`+`fixheader`
  deb2 appendix over that same `(rom, listing)` via `append_deb2_appendix`. Byte-
  identical to `build_native_full_file` / `build_full_file_chained` by construction.
  `--emit-lst` drops the sigil-canonical `.lst` (the `.lst`-consumer drop-in for
  s4budget/oracle/repin). Prints `crc=<crc32> len=<bytes>`.
  - `--config-a`/`--config-b` fix the whole shape (sonic4 game); they conflict with
    `--game`/`--debug`. Unknown `--game` refused. Locked by
    `parse_build_args_native_target_selection` (t24-style accept/reject unit test).
  - The legacy no-`--native` path (`assemble_full_rom` raw image) is unchanged.

- **`build.sh`** (aeon) — a `SIGIL_NATIVE=1` branch wraps the whole
  asl→p2bin→convsym→fixheader tail: one `"${SIGIL_BUILD}" build --aeon . --native
  --game ${GAME} [--debug] -o ${ROM_NAME}.bin --emit-lst ${ROM_NAME}.lst`. asl stays
  the DEFAULT (flag off). `SIGIL_BUILD` must point at the sigil binary (hard error if
  unset, mirroring the `SIGIL_EMIT` preflight). The budget summary reads the sigil
  `.lst` unchanged.

## Proof — SIGIL_NATIVE reproduces the gate outputs EXACTLY

All six targets, via the real `sigil build --native` binary AND via `build.sh
SIGIL_NATIVE=1` (both games, both shapes):

| target | full-file crc / len | gate pin |
|---|---|---|
| sonic4 plain | `2198deb2` / 395374 | native_full_rom |
| sonic4 debug | `1d895fcb` / 402696 | native_full_rom |
| demo plain   | `0646d4bf` / 76851  | native_offcanonical_full |
| demo debug   | `7e4a358a` / 77244  | native_offcanonical_full |
| config_a     | `80e602df` / 402742 | native_offcanonical_full |
| config_b     | `9eb2e8a1` / 286904 | native_offcanonical_full |

PRIMARY assembled anchors intact (native_rom green → assembled prefix `[0,EndOfRom)`
header-neutral == asl e5765873/dab4f06c). The reference `s4.bin`/`s4.debug.bin`/
`demo.bin` (gitignored asl artifacts the suite reads) were backed up, the SIGIL_NATIVE
builds proven against a scratch tree, then RESTORED — aeon tree clean but for build.sh.

Strict worktree suite (`AEON_DIR=<aeon>/.worktrees/flip-stage2`): **2939 / 0 / 1
ignored** (= prior 2938 + the one new CLI grammar unit test), failures-first: 0
failures across the workspace.

## Phase B — RULED a Phase-C companion (not pre-deletion work)

The parcel's Phase B (AS-residual section-split at declared boundaries + the `$20000`
object-bank budget as a map-region check + pins→map placement authority) is, per the
overseer's OWN S1.2 ruling (`2026-07-30-flip-stage1-S1.2-map-growth-finding.md`
§68-85), coupled to the gate deletion and MUST NOT precede it:

- The computed-placement half ("resume points become link outputs") is ALREADY
  realized by the declared-order chainer in-harness (the six-proof matrix is green
  with ZERO aeon edits).
- The section-split + `$20000` map-region budget models the DUAL-STATE geometry — the
  contiguous AS-residual section that CROSSES `$20000` (proven: one AS section spans
  `[0x11D7E, 0x256DC)`, object-bank tail + level data, so `$20000` is an INTRA-section
  checkpoint a map region cannot express). That section is exactly what Stage 2
  DELETES; the boundaries become natural only once the twins + gate scaffolding go.
  The overseer ruled this a STAGE-2 COMPANION that "lands WITH the gate deletion" —
  doing the linker-granularity change now is "work against a corpse."
- The object-bank budget is meanwhile enforced by the AS-residual `engine.inc`
  `if * > $20000 / error` (the resume-org cursor advances through the bank); it does
  not lapse pre-deletion.

So there is no safe, standalone, additive Phase-B increment before the deletions; its
cargo rides the flip commit (Phase C) alongside `repin`'s `.lst`-parse retirement
(row 34). Recorded here so checkpoint 1 is honest about scope.

## The valve — pre-deletion, reversible

Additive CLI + build.sh flag only; asl default byte-identical; ZERO twin deletions;
six-proof matrix green; strict green. **The flip commit (Phase C = the point of no
return) WAITS for the overseer's mid-parcel countersign** (checkpoint 2 = after the
build.sh flip + the first deletion group).
