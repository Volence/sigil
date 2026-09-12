# Oracle A/B protocol — the §17 optimization sweep

The byte gate is BLIND on a byte-changing optimization (you are changing the very bytes
it pins) and doubly blind on a faithful-but-wrong change (both the old and new build
would agree if the change were a no-op). So every byte-changing parcel proves behaviour
on the **oracle** (the emulator) with **frame-anchored, named** observations, then
re-freezes the goldens (`refreeze --freeze`). This file is the concrete procedure.

The runner is the **agent** driving the oracle over MCP (`emulator_*`). The one
mechanical helper is `region-hash.sh` (client-side hash + the length assert). Client-side
hashing is deliberate (overseer ruling, OQ-3): a 14.4 KB region is one or a few
`emulator_read_memory` calls; A/B events are rare, so determinism matters, speed does
not. If this proves flaky in practice, STOP and revisit an oracle-side `memory_hash` —
not before the evidence.

## Determinism (binding on every A/B)

0. **PROVE WHICH CART IS LOADED, BEFORE EITHER ARM, AND AGAIN AFTER EACH LOAD.** Hash the ROM
   file on disk, load it, and read enough of the loaded cart back to confirm the emulator holds
   THAT file. `emulator_reload_rom` recovers a stale one, but only for a runner who checks first.
   Everything below this line hashes RAM, VRAM, CRAM and the plane: **nothing in this protocol
   hashed the CART until 2026-09-12**, and that is the one input both arms share.

   **Why this is step 0 and not a footnote: a stale cart makes an A/B agree, and agreement is the
   direction nobody audits.** If the emulator was started against some other ROM and neither arm
   reloads, OLD and NEW measure the identical cart, every region hash matches, every `cmp` is
   pixel-identical, and the packet reports a clean PS verdict having compared nothing. The
   existing bar *an A/B whose arms agree may have measured nothing* is exactly this, arriving
   one layer below where that bar looks: it asks what each arm PRODUCED, and here both arms
   produce real, correct, identical measurements of the wrong program.

   **This is live, not hypothetical, and it was flagged by the aeon lane on 2026-09-12 rather
   than found here.** Nine `oracle-aether` processes were running, one per Claude session and
   spawned at session start, every one preloading `/home/volence/sonic_hacks/aeon/s4.debug.bin`.
   Verified firsthand here at 19:4xZ: that file is md5 `c34c3f92b1fb5693680de20b23cfed39`,
   847367 B, mtime 2026-09-11T02:24:34, while `origin/master` builds md5 `06e50f02`, 847533 B.
   **166 bytes and two days apart, behind a `romPath` that reads correctly.** Killing them fixes
   nothing; they respawn on the same file. The root cause is aeon's main checkout sitting behind
   its remote, which is an open owner call and not this lane's to take.

   **Scope, stated so this is not read as wider than it is:** no gate in `scripts/` invokes
   anything in this directory (checked, zero hits), so no automated sigil gate can go green on a
   stale cart. The exposure is hand-run A/B measurement only, which is the only way this
   directory is ever used. **That makes it worse to leave unwritten, not better** — a hand-run
   instrument has no CI to catch it and no second reader.

   *(The discipline existed in this lane's session memory as "verify the cart by hash" and was
   never written into the file the runner reads. That gap is the day's recurring shape: an
   obligation recorded where the person who must discharge it will not be standing.)*

1. **Reset-deterministic scene** — no human input timing. Drive from `emulator_reset`
   then a fixed poke/press sequence, or the ObjectTest soak via the `Game_Entry` flip.
2. **Frame-anchored** — advance to a fixed `Frame_Counter` (run_to / step by frames),
   NEVER a press count. Capture OLD and NEW at the *same* frame numbers.
3. **Identical drive** — the OLD and NEW ROMs run the byte-for-byte same scene script.
4. **No hand-transcribed hex** — screenshots write PNG to disk directly (`cmp`);
   off-screen regions are saved as raw bytes and run through `region-hash.sh` (which
   length-asserts). A dropped byte must surface as a length failure, never a false diff.

## The per-class bar (from the design note §1.3)

| Class | What to capture |
|---|---|
| **PS** pure-size / value-identical | (a) determinism: two native builds byte-identical (the `native_*_rom` gate already asserts this). (b) STATE-IDENTITY: at ≥3 anchor frames spanning the exercised path, the affected RAM/VRAM/CRAM region is byte-identical OLD vs NEW (`region-hash.sh --diff`), and the visible plane `cmp`s pixel-identical. |
| **BA** behaviour-adjacent / hazard-fix | PS bar PLUS a NAMED positive observation of the fixed effect at the exact frame it manifests. For a provably-inert hazard fix (e.g. G9), the positive half is: the shipped scene is byte-identical (fix inert in normal dispatch) AND the guarded value is shown clean at the guard site (the static benign-under-current-dispatch confirmation, made concrete on the oracle). |
| **PF** perf-affecting | PS bar PLUS a live-profiler A/B on an UNFROZEN drive (frozen scenes under-load — they skip `EntityWindow_Scan`, so lag can't appear). Report before/after self-time on the hot proc AND `Lag_Frame_Count` on a real max-H / max-V drive. Threshold rule: cut ≥~1k cyc/f steady-state, else log-and-skip with the numbers. VBlank-DMA items need a worst-case VBlank wall-time audit, not CPU self-time. |

## The loop (per byte-changing parcel)

1. **step-0 re-confirm** the item against current source (the §17 review vs the .emp).
2. **Build OLD** (pre-change) and **NEW** (post-change) ROMs; keep both files.
3. **Drive the A/B** per the class bar; save captures under a parcel-named scratch dir;
   record the crc/size + diff verdicts (the packet's evidence).
4. **Re-freeze**: `refreeze --freeze <parcel> --ab <path-to-this-evidence>` — regenerates
   blobs + size tables + pins.rs and appends the provenance chain entry. An anchor that
   moved without a real `--ab` ref is a HARD failure (the bin refuses).
5. **Gate**: `refreeze --check` green; the strict suite green.

The evidence path handed to `--ab` is what the provenance chain records as the proof
that this anchor move was earned. Keep it durable (a committed note), not a scratch file.
