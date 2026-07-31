
## Overseer A/B results + adjudication (countersign)

**State identity (plain, quantum anchors fc 234/414/694, double-runs deterministic):**
VRAM/CRAM/VSRAM/regs identical at EVERY anchor (including the register file — no
capture aliasing this time). Zone-aware RAM (game RAM +$340 behind the new engine
tail): game RAM byte-identical; stack = the established dead-residue class; and the
ONLY engine-RAM diffs sit ENTIRELY inside `Block_Stage_Buffers` — the staging slots
for empty claims are now dead storage (OLD zeroed them, NEW leaves stale bytes and
serves the shared zero page). Zero diffs outside the buffers at all three anchors:
the read-only pointer contract is empirically proven, not just argued.

**Profiler (max-H):** whole-loop **−684 cyc/f** (DecompressBlock −703 with S4LZ ±0 —
the win is exactly the non-decode staging work; FindStagedBlock +12 = the three
pointer stores, the only adverse cost anywhere). The win case this drive cannot
show: empty/raw-heavy streaming (world edges, sky regions) — analytically up to
~35k cyc/f on a 6-empty-block frame that today pays 6×768B `clr.l`.

**Rulings:**
1. **KEEP — with the sub-bar test AMENDED (the work-removal clause):** the three-leg
   test governs sub-bar parcels with ADVERSE TRADEOFFS. A parcel that strictly
   removes work — byte-identical consumer-visible state PROVEN on the oracle, net
   cost ≤ noise on every frame class, ROM not grown (here: shrunk −19/−51) — is a
   step-5 improvement and keeps on those grounds. core #1 stays dead (measured
   adverse cost on the binding drive); pb#3 stays dead (format risk, no removal).
2. **The RAM zero page is ACCEPTED** (768B engine-tail RAM + 64B pointers). The
   review's ROM form would cost a new anchored island against a ~496B debug
   pre-bank margin. LEDGERED: if RAM pressure arrives, the ROM-island form becomes
   attractive once the residual-split capstone makes adding islands cheap.
3. Oracle ops note: the A/B harness now boots ROMs from a fixed
   `~/.config/oracle/ab_current.bin` path via the persisted LastRomPath and swaps
   FILE CONTENT between GUI restarts, verifying identity in-emulator (ROM-end
   pointer) — the flaky reload path is out of the loop entirely
   (`profile_drive2.py`, SKIP_RELOAD).
