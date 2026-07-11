# Tranche 10 checkpoint packet — core.emp + dplc.emp (loop dry, at the merge gate)

**For Volence's gate.** The object system's SPINE is ported. Loop ran dry;
strict **2086/0**, clippy clean. NOT merged — awaiting your ruling (esp. the
RunObjects engine-architecture question below).

## Headline

- **`engine/objects/dplc.asm` (107 ln) + `engine/objects/core.asm` (328 ln)
  ported** — `Perform_DPLC`/`_Deferrable`, and core's `InitObjectRAM`,
  `Alloc{Dynamic,Effect}`, `DeleteObject`, `RunObjects` (+ pool loops),
  `RunObjects_Frozen`, `ObjectMove{,X,Y}`, `Debug_AssertObjLoop`.
- **STEP 0 first (your ask): the `repin` tool shipped** — generated
  `pins.rs` from a `repin.toml` manifest + the listings; the substitution-
  error bug class is now unwritable. It earned its keep THIS tranche: the
  −4 shrink's downstream re-pin was one `cargo run … repin` + a printed
  diff, not 60 hand edits. All 18 pin-bearing test files migrated to it.
- **TWO new language capabilities** (demanded by core, shipped per the
  demanded-features law): imm-link + a pinned-abs operand in one
  instruction, and `FixupKind::ImmWord16Be` (the correct `.w` word-immediate
  rule — I caught a latent regression here; see below).
- **core is the first shape-dependent-length region since rings**: plain
  0x1C0 / debug 0x2E8 (the debug surplus = `Debug_AssertObjLoop` + its two
  call sites, all under `if DEBUG == 1`).

## Pins (new campaign provenance — the modernized reference)

The step-2 modernization changed bytes, so the reference ROMs are new:
- plain `s4.bin`   : `15f2d69e428f64b5…`
- debug `s4.debug.bin`: `2d095a44d7fbb061…`
(superseding the step-1 `50f92f57…`/`1dfe4a4c…`.) EndOfRom UNCHANGED
(`$658B4`/`$673A2`) — the `org $10000` bank re-anchor absorbs the −4 shrink
as pad; object-bank/data regions did NOT shift. `repin --check` clean.

## What shipped, by step

**Step 1 — transcribe (byte-exact both shapes).** Gates `SIGIL_EMP_DPLC` +
`SIGIL_EMP_CORE`. Constants twin 30→34 (`NUM_TOTAL_SLOTS`, `CULL_DISTANCE_X`
$300, `CULL_DISTANCE_Y` $200, `SLOT_TAG_UNTAGGED` $FF; kill-list row 19).
Two shipped features:
- **imm-link + one pinned-abs operand** — core's free-stack SP writes
  (`move.w #Dynamic_Free_Stack+NUM_DYNAMIC*2, (Dynamic_Free_SP).w` /
  `cmpi.w …`) put a link immediate AND a pinned-abs.w link operand in one
  instruction (two independent fixups, offsets 2 and 4). The imm-link path
  rejected any second symbolic operand ("fixups would collide" — over-broad;
  they're at different offsets). Now admits ONE `AbsSym`; relaxable
  `Sym`/`SymOff` still rejected.
- **`ImmWord16Be`** — the `.w` immediate resolves core's RAM addresses
  (`$FFFF9EDE`), which `Value16Be` (unsigned `[0,0xFFFF]`) rejects. The
  transcription first routed it to `Abs16Be`, but that EA-address window
  SILENTLY REJECTS an objroutine offset in `[0x8000,0xFFFF]` — a valid
  upper-bank tranche-6 store. **Caught in adversarial review** (latent — no
  test hit it). Neither single-window kind is AS's actual rule (high 16 bits
  all-0 OR all-1). Added `ImmWord16Be` = that union; byte-neutral.

**Step 2 — modernize (house format; −4 shrink; re-pin wave).** Bare Bcc,
`jbra`/`jbsr` (dplc `jsr QueueDMA_*`→bsr.w; core local calls), byte-locks
kept explicit (the ifdebug/assert transliterations, MDDBG `.l` jumps,
computed `jsr (a1)`). Two shrinks — `bsr.w .run_culled` / `bsr.w Draw_Sprite`
→ `bsr.s` (both shapes, −4 total) — AS twins in lockstep. The re-pin: 15
pins shifted, ALL −4, core-and-engine-downstream only (upstream + RAM
unchanged, `DeleteObject` unmoved). Also closed a step-0 gap: migrated
`mixed_dac_rom.rs`'s downstream bases to `pins`.

**Steps 3–5 — retrospect / back-prop / optimize (loop dry).** Details below.

## Verification

- Strict **2086/0** (`SIGIL_STRICT_GATE=1`), clippy `-D warnings` clean.
- core + dplc reference gates green BOTH shapes; downstream tranches
  (animate/collision/rings) green at their shifted −4 addresses.
- Gate-off neutrality held through step 1 (reference == step-1 pins);
  step 2 changed the reference in lockstep (new pins above).
- **Live-verified in oracle** (step 5): profiled in-level with active
  objects — numbers below.

## ⚠️ THE ONE DECISION FOR YOU — RunObjects occupancy (engine architecture)

Step-5 profiling (oracle, live, 3 active objects): **RunObjects = 11,841
cycles = 9.3% of the 128k NTSC frame budget**, dominated by fixed iteration
over all 66 pool slots (~63 EMPTY in a light scene). The real lever is an
**occupancy / active-list so RunObjects skips empty slots** — but that's an
engine-architecture change to the pool-iteration contract (same class as
the tranche-9 `AnimateSprite_PerFrame` deletion: your call, not a unilateral
step-5 edit). A cheaper micro-opt (hoist the `OBJ_CODE_BANK` prefix out of
the per-slot loop, ~200 cyc/frame) was NOT taken — it adds ~8 bytes →
another re-pin for 0.15% budget. **Do you want the active-list redesign
scheduled (its own tranche), or is 9.3% acceptable at this object count?**

## What each pass added (ratified format)

**Pass 1 (steps 1→5):**
- STEP-3 (asks / reads-wrong / rows / ledger): the **byte-lock friction
  ask** — bare Bcc can't pin a width the twin forces (`bne.w
  RunObjects_Frozen` over-relaxed to `.s`, gate caught it); want a
  force-width idiom or twin-parity lint. **`org $10000` shield** process
  note (engine-internal shrinks don't re-pin object-bank/data). **Mixed-slice
  inline-target-bytes** limitation (repin can't track a target's low word in
  a `[u8]` literal; mitigation = splice `pins::X` low word). Kill-list row
  19 (constants twin 30→34). dplc dedup DEFERRED (twin-shape divergence).
- STEP-5 (optimizations): RunObjects profiled (numbers above); **both
  candidates NOT TAKEN** — the bank-prefix hoist (bad byte/re-pin trade) and
  the occupancy redesign (your architecture call). DeleteObject's 20×
  unrolled clear: not per-frame-hot, unroll fine — recorded, not taken.
- Neither bucket (step-1 demanded features / live findings): the two shipped
  language capabilities; the **ImmWord16Be latent-regression catch** (review,
  not a failing test); the byte-lock over-relaxation catch (the byte gate).

**Pass 2 (loop-until-dry retrospect):** DRY — every finding was an ask
(deferred), a not-taken opt (recorded), or a process note. No new
back-prop/optimize work. Loop terminated.

## Commits (both repos, NOT merged)

- **aeon** `sigil-emp-tranche10`: `8e15c71` `f638546` (step 1) → `a6762c0`
  `81c25e3` `740132d` (step 2).
- **sigil** `port-tranche10`: 30 commits `a757169…ef8ab45` (step 0 tool +
  migration, step 1 features + ports, step 2 re-pin, step 3-5 docs).

On your gate: `--no-ff` merge both sides + push; update the campaign
provenance hashes to `15f2d69e…`/`2d095a44…`.
