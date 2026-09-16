# Oracle A/B protocol — the §17 optimization sweep

The byte gate is BLIND on a byte-changing optimization (you are changing the very bytes
it pins) and doubly blind on a faithful-but-wrong change (both the old and new build
would agree if the change were a no-op). So every byte-changing parcel proves behaviour
on the **oracle** (the emulator) with **frame-anchored, named** observations, then
re-freezes the goldens (`refreeze --freeze`). This file is the concrete procedure.

The runner is the **agent** driving the oracle over MCP (`emulator_*`). Two mechanical
helpers: `cart_check.py` (step 0, WHICH CART, and the one thing here with a `cargo test`
gate behind it) and `region-hash.sh` (client-side hash + the length assert). Client-side
hashing is deliberate (overseer ruling, OQ-3): a 14.4 KB region is one or a few
`emulator_read_memory` calls; A/B events are rare, so determinism matters, speed does
not. If this proves flaky in practice, STOP and revisit an oracle-side `memory_hash` —
not before the evidence.

## Determinism (binding on every A/B)

0. **PROVE WHICH CART IS LOADED, BEFORE EITHER ARM, AND AGAIN AFTER EACH LOAD.** This is an
   INSTRUMENT, not an instruction. Run it, do not re-derive it:

   ```sh
   python3 crates/sigil-harness/golden/ab/cart_check.py <the-rom-this-arm-means>
   #   exit 0  CART PROVEN (coverage=total|sampled)  <path>  <crc32> / <bytes>
   #   exit 1  CART CHECK REFUSED  <the reason, by name>
   ```

   All eighteen instruments in this directory call it themselves (`verify_cart_bus`), at step 0
   and after every `reload_rom`, so a scripted arm needs nothing extra; the CLI is for a
   hand-driven one. It was prose here from 2026-09-12 until it became an instrument, and prose
   rides on the runner remembering to read step 0. `emulator_reload_rom` recovers a stale cart,
   but only for a runner who checks first. Everything below this line hashes RAM, VRAM, CRAM and
   the plane: **nothing in this protocol hashed the CART until 2026-09-12**, and that is the one
   input both arms share.

   **What it checks, and why the server's own caveat is not enough.** `status.romPath` IS the
   intended file; `status.romBytes` equals its size on disk; the cart's own BYTES agree with the
   file; and a `status.caveat` naming the cart refuses. The wire contract (item 27, §11.37) does
   oblige a server to call a stale image out loud, including the required row where the sizes
   match and the bytes differ. But that caveat answers only *has the file at `romPath` changed on
   disk since it was loaded?* It is silent on *is this the cart THIS ARM MEANT?* An emulator
   holding `/a/old.bin` while the arm intends `/b/new.bin`, with `/a/old.bin` unchanged, emits
   **no caveat at all** and every region hash below still matches. So the caveat is one input,
   never the check.

   **WHAT A GREEN MEANS, and it is two different claims.** The passing line says which:

   - `coverage=total` - every byte compared. One `memory_hash {addr:0, len:romBytes}` call; its
     `crc32` is defined to equal zlib CRC32 over the same slice of the file. One round trip,
     which is why it is preferred: a whole-image readback through `read_memory` (4096 B a call)
     would be ~200 round trips for an 820 KB image.
   - `coverage=sampled` - the fallback when a server does not serve `memory_hash`. Head, middle
     and final windows only. It proves the cart is not one of the builds that differ INSIDE those
     windows; **a targeted one-byte edit outside every window passes.** That is adequate against
     the hazard actually filed (a stale build differing in ~43% of its bytes) and is NOT a general
     identity proof. Export `CART_CHECK_REQUIRE_TOTAL=1` to make the fallback a refusal.

   Record the coverage word in the packet alongside the verdict. A green whose coverage nobody
   wrote down is a green nobody can weigh later.

   **What it does NOT cover.** It proves the emulator's CART. It says nothing about the symbol
   listing (`load_symbols` is a separate path and a listing can disagree with the image), nothing
   about a save state loaded at the window, and nothing about whether the ROM on disk is the
   build you meant to make: a freshly proven cart of a two-day-old build is still a two-day-old
   build, which is the root cause the filed incident actually had.

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

   **Scope and coverage, measured with controls.** No gate in `scripts/` or anywhere under
   `crates/` invokes anything in this directory (zero, against a control of 30 tracked files in
   `scripts/` of which 26 match `bash|python`, and 591 files matching `fn ` under `crates`), so no
   automated sigil gate can go green on a stale cart. **This claim first cited the pathspec
   `crates/*/src`, which matches NOTHING in this repo, so that half of the evidence was vacuous
   and the zero could not have been anything else. Re-measured 2026-09-12 with `crates/`: the
   conclusion holds and the evidence did not.** The
   exposure is hand-run A/B measurement only, which is the only way this directory is ever used.
   **That makes it worse to leave unwritten, not better** — a hand-run instrument has no CI to
   catch it and no second reader.

   **The coverage figure that made step 0 a rule and not a reminder, and where it stands now.**
   As measured 2026-09-12: of the 18 instruments here, 8 called `reload_rom` with an explicit
   path and **ZERO verified the cart**, against a control confirming all 18 are readable files
   the matcher could have fired on, so the zero was the subject rather than a broken matcher.
   `reload_rom` says *load this*, and nothing confirmed the emulator held it: the eight were
   defended against a stale PRELOAD and none of the eighteen against a load that silently did not
   take. **All 18 now call `cart_check.verify_cart_bus`, and the count is gated rather than
   fixed once**: `every_bus_driving_instrument_calls_the_cart_check` in
   `crates/sigil-harness/tests/ab_cart_check.rs` walks this directory, selects by the bus-client
   import and fails naming any instrument that does not call it, so a new instrument cannot join
   the population unwired.

   **Enumerate the population by the BUS CLIENT it imports (`from aether import` / `BusClient`),
   across the whole repo, and nothing else.** Three wrong populations preceded this one and each
   was published: the literal vocabulary `emulator_`/`romBytes` finds 10 of the 18, because the
   rest reach the bus through the shared client and name none of those tokens; a first pass was
   additionally truncated by a `head`; and adding `suite_paths` to the matcher inflates it with a
   path-resolution helper used all over `scripts/` and `docs/` that has nothing to do with the
   emulator, while scoping the search to `golden/ab/**` hides anything outside it. **Matcher and
   population are separate choices and fixing one does not prompt a look at the other.**

   **The number that never moved is the one that matters: ZERO, under every population.** Three
   denominators, three matchers, one numerator. A conclusion invariant across independent wrong
   framings is the one worth acting on, and it is the reason step 0 is a binding rule here rather
   than a caution.

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
