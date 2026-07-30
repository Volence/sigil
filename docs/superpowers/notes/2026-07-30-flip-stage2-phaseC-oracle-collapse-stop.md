# 2026-07-30 — FLIP STAGE 2 · PHASE C: the build.sh flip landed; the twin deletion hits the AS-reassembly oracle family — a VALVE STOP for the row-91 design ruling

Status: **build.sh flip COMMITTED and proven (aeon `97a9127`). The FIRST twin
deletion (compression) revealed that ANY twin deletion breaks the entire AS-reassembly
oracle family AT ONCE — 11 test files including the 5714-line `mixed_dac_rom` (the
row-91 DSM witness) and the whole-ROM `m1d_rom`. Per the brief §3 (row-91: "if
preserving the witness amid the collapse requires design choices, STOP and present
them") + the valve (identity/scope surprise, design fork), I STOPPED, reverted the WIP
to the green build.sh-flip boundary, and present the ruling question below.**

Boundary: aeon `flip-stage2` `97a9127`, sigil `flip-stage2` `6840ae6`; both trees
clean; strict worktree pair **2939 / 0 / 1 ignored** (green). NOTHING deleted.

## 1. What landed (the build.sh flip — Phase C item 3)

`97a9127` (aeon): `build.sh` drives ONE `sigil build --native` invocation; asl/p2bin/
fixheader out of the pipeline; convsym survives on sigil's listing. Artifact ledger
stated in the commit (full files now sigil-canonical; PRIMARY anchors unchanged). Both
games, both shapes proven at their pinned CRCs. The `.asm` twins still PRESENT (so the
suite's AS-reassembly oracles still build) — the deletions were to follow. Strict green.

## 2. The discovery — the deletions and the oracle collapse are INSEPARABLE

I staged the first deletion group (compression: `s4lz_decompress.asm`,
`zx0_decompress.asm` + the two `engine.inc` gate collapses). The native build stayed
byte-identical (native_rom/native_full_rom/native_offcanonical all green — the gates
build gate-ON, so the twins were never included). BUT the full strict suite went red in
**11 files**:

```
seam1_native_link  seam2_dac_rom  seam2_mt_rom  seam2_sfx_rom
test_t1_harness_states_port  vblank_port
m0_regions  m1d_debug_rom  m1d_rom  mixed_dac_rom  mixed_offcanonical_rom
```

Root cause (one class): every one of these assembles a **twin-INCLUSIVE AS build** as
its oracle/reference — `assemble_full_rom` (all gate-off), the ~30
`assemble_mixed_tranche*_as_side` (partial gate-off — the tranche's modules native, the
REST including compression via `.asm`), `assemble_mixed_z80sound_as_side`, and the
gate-off `assemble_root` (`vblank_port`, and `boot_port`/`m1c_vector_table`). Deleting
ANY referenced twin removes its symbols → `unresolved symbol S4LZ_Decompress …` at link.

**So there is NO "small clean first subsystem": the first twin deletion breaks the whole
AS-reassembly oracle family simultaneously.** These are exactly the oracles the design
§2.5 says COLLAPSE ("the ~52 DSM tranches fold into the single whole-ROM golden gate")
and whose AS-twin-reassembly halves DIE — their coverage subsumed by the native
whole-ROM golden gates (`native_rom`/`native_full_rom`/`native_offcanonical_*`, all
green) + the surviving per-module `.emp`-region-vs-golden-slice gates + t24. But that
collapse is a LARGE coherent test-transformation that must PRECEDE/ACCOMPANY the first
deletion — not a per-subsystem edit that rides each group.

## 3. THE ROW-91 DESIGN QUESTION (the explicit STOP trigger)

`mixed_dac_rom.rs` is a **5714-line** file that, in ONE place, holds BOTH:
- the ~30 tranche oracles (DSM.9 acceptance bars) that the design says COLLAPSE, AND
- the **row-91 DSM in-memory composition witness** that Volence RULED must SURVIVE,
  re-comparanded and kept non-vacuous (`SIGIL_EMP_*_BODY_STUB` composition).

Separating "retire the collapsed tranches" from "preserve + re-comparand the DSM
witness" inside this file is a design choice the brief §3 reserves for a ruling. The
question set for the overseer/Volence:

- **Q1 (witness identity):** which exact assertion(s) in `mixed_dac_rom` ARE the row-91
  witness to preserve (the SFX/DAC BODY_STUB in-memory composition == reference), vs the
  tranche oracles that collapse? I will characterize the file precisely and propose a
  minimal surviving witness re-comparanded to the frozen golden slice + a t24 doctor.
- **Q2 (collapse mechanism):** confirm the tranche oracles retire as "coverage subsumed
  by the native whole-ROM golden gates + the `.emp`-region gates," rather than being
  re-comparanded one-by-one (which would be ~30 bespoke rewrites for zero net coverage
  over the whole-ROM golden gate).
- **Q3 (the harness fns):** the `assemble_mixed_tranche*_as_side` / `assemble_full_rom` /
  `assemble_mixed_z80sound_as_side` harness functions become dead once their only callers
  (the tranche oracles) retire — confirm they delete WITH the collapse (they are
  themselves AS-reassembly machinery the flip retires).

## 4. Proposed plan (pending the ruling)

A dedicated Phase-D test-transformation commit (sigil), landing WITH or immediately
BEFORE the first twin deletion:
1. Retire the AS-reassembly oracle family whose coverage the native whole-ROM golden
   gates subsume: `m1d_rom`/`m1d_debug_rom` (== `native_rom`), the `mixed_dac_rom`
   tranches (== `native_full_rom`), `mixed_offcanonical_rom` (== `native_offcanonical_*`),
   the `seam1`/`seam2` mixed AS-side gates, `m0_regions`, and the gate-off `assemble_root`
   arms in `vblank_port`/`m1c_vector_table`/`boot_port` twin-parity.
2. PRESERVE the row-91 DSM witness (per Q1) re-comparanded to the golden slice + t24.
3. Keep every per-module `.emp`-region-vs-golden-slice gate (they compile the `.emp`, not
   the `.asm`) and their t24 controls VERBATIM. Re-point their comparand `aeon/s4.bin`
   → the frozen `golden/*.bin` (byte-identical; independence from the now-sigil-built tree
   artifact). Handle the `aeon/s4.lst` asl-symbol lookups (e.g. boot's `Z80_SOUND_SIZE`)
   by pinning the value or reading the golden.
4. THEN the twin deletions proceed clean, per-subsystem, each byte-proven by the native
   gates + strict green.

## 5. Proof-of-pattern (demonstrated, then reverted to keep the boundary clean)

I transformed `boot_port.rs` as a concrete pattern: its 3 gate-off full-AS twin-parity
tests (sound-off plain/debug, hotkeys) retire — their coverage subsumed by the
config_b (sound-off) / config_a (hotkeys) whole-ROM golden gates; the 3 canonical
golden-backed region tests + the doctored-PSG t24 control SURVIVE verbatim; the dead
`as_full_module`/`oracle_value`/`run_twin_parity` helpers go. It compiled and passed
(3/3) standalone. Reverted so the checkpoint boundary is exactly `97a9127`/`6840ae6`
with no half-transformation; the diff is ready to re-apply as part of the ruled plan.

## 6. Ask

Countersign the Phase-D collapse plan (§4) + rule the row-91 witness identity (§3 Q1-Q3),
then I execute the transformation + the first deletion group and return for the
checkpoint-2 countersign. The build pipeline is already flipped and proven; this is the
test-oracle transformation the flip mandates, stopped at the row-91 design gate exactly
as the brief instructs.
