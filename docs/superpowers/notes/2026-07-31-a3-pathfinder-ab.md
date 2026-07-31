# Wave-A pathfinder — animate A3 tail-call: A/B evidence

**Parcel:** animate A3 — `AnimateSprite.set_frame` `jbsr RefreshSpritePieceCount` + fall-into
`.done: rts` → `jbra RefreshSpritePieceCount` (tail-call). Engine-block, **class PS**
(behaviour-identical). aeon branch `opt-a3` off master `d9d2385`.

**step-0 re-confirm (review doc vs current source):** review
`2026-07-16-emp-port-optimization-review.md:1216` ("second `jbsr` + rts → `jbra` tail call")
and `:1524`/`A3`. Current source `engine/objects/animate.emp:111-115` still `jbsr`+`.done: rts`
— OPEN, matches the census. NOTE the census "−2 shrink" was slightly off: `.done: rts` is the
target of `bpl .done` at `:97`, so the `rts` must STAY. The fix is therefore SIZE-NEUTRAL
(`bsr.w`→`bra.w`), not a shrink — even cleaner (no layout shift at all).

## The change is provably surgical — a 2-byte diff

Full-ROM `cmp -l` OLD vs NEW (both shapes rebuilt from `opt-a3`):

| ROM | OLD | NEW | size |
|---|---|---|---|
| s4.bin | 7f071417 / 412306 | 8692e93a / 412306 | SAME |
| s4.debug.bin | 0b8efc7a / 422147 | 1f84bdbc / 422147 | SAME |

`s4.bin` differs at **exactly two bytes**:
- `0x2F90`: `0x61 → 0x60` — the `bsr.w`→`bra.w` opcode at `AnimateSprite.set_frame` (the
  tail-call; `.set_frame` is at `0x2F8C`, per `s4.lst`).
- `0x18E`: the header checksum (recomputed because a code byte changed).

Nothing else moves — the size-neutral edit shifts no section, so the anchor change is provably
just the tail-call opcode + checksum. This is the byte-level proof the transformation is exactly
what was intended and nothing else.

## Static / contract proof of behavioural equivalence (PS)

`jbsr X; <fall to> rts` → `jbra X` is a textbook tail-call, safe iff `X` ends in `rts` and does
not depend on the return address / stack depth:
- `RefreshSpritePieceCount` (animate.emp:279) is a leaf `proc (a0: *Sst) clobbers(d2, a1)`
  ending in `rts` — `d2,a1 ⊆ AnimateSprite.clobbers(d0-d2/a1-a2)`, same `a0`.
- The `.emp` contract system ACCEPTED the tail-call (build passes both shapes) — the same idiom
  is already used at `:196` (`jbra AnimateSprite`) and `:203` (`jbra DeleteObject`).
- `.done: rts` is retained (still the target of `bpl .done` at `:97`); only the `.set_frame`
  path now tail-jumps instead of call-return-through-`.done` (the ~24-cyc win).

## Determinism

Both shapes build deterministically (the `native_full_rom` gate asserts a second build is
byte-identical; re-confirmed at the re-freeze).

## Oracle empirical A/B — EXECUTED (overseer countersign pass)

The porter-session "oracle would not switch ROMs" hold was diagnosed and cleared at the
countersign: the GUI's **main UI loop was wedged** (blocked in `startRomLoad`'s
main-thread `romLoadThread.join()` on an earlier stuck reload worker), so every
main-loop-drained op (reload, screenshot, press) queued forever while socket-thread ops
(status, resume) and the emulation thread kept running — exactly the observed symptoms.
An emulator-process restart recovered it (the recovery the GUI's own error strings
prescribe); reload then applied first try.

**Scene (reset-deterministic, per AB_PROTOCOL):** power-cycle reload → `reset(run=false)`
(paused at frame 0) → 60 deterministic frames (OJZ boot; the changed instruction provably
never executes there) → poke `Game_State` (0xFF8004) = `GameState_ObjectTest_Init`
(0x5C230) → deterministic frames to the anchors. `AnimateSprite.set_frame`'s changed
instruction (0x2F90) was breakpoint-PROVEN to execute in this scene. Runners committed
under `golden/ab/a3/` (`ab_runner_quantum.py`, `ab_runner_codepoint.py` — Aether-bus
drivers; captures written raw to disk, no hand-transcribed hex). Every configuration ran
TWICE: run1 ≡ run2 on every hash at every anchor (empirical determinism, both ROMs).

**Quantum-anchored captures** (Frame_Counter 234/474/894, paused on the exact-frame-time
quantum): full 64KB work-RAM dump + VDP state-hash (incl. hashed RGBA framebuffer) +
screenshot PNG per anchor.
- **Framebuffer: identical OLD vs NEW at all 3 anchors** — both by the state-hash RGBA
  hash AND by literal PNG `cmp`: all 12 screenshots (2 ROMs × 2 runs × 3 anchors) are
  byte-for-byte identical per anchor (the three frames committed as
  `golden/ab/a3/frame_f{240,480,900}.png`). N.B. the screenshot op ignored the requested
  `path` and wrote timestamped files to its default dir — a third oracle nit, logged
  below.
- **VRAM / CRAM / VSRAM: hash-identical at all 3 anchors.**
- Work RAM: byte-identical at 234/474; at 894 exactly ONE byte differs —
  0xFFFFFEFB, inside the VBlank exception frame's pushed PC (OLD `000022D4` /
  NEW `000022D0`, SR word `2304` beside it): the interrupt struck the `VSync_Wait`
  poll loop one instruction apart. This is the ~24-cycle saving itself, visible as
  intra-frame phase; not a state divergence.
- VDP register-file hash differed at 234/474 (converged at 894) for the same reason:
  the frame-time quantum pauses mid-VBlank-handler, and the phase skew catches the
  handler's register-write sequence at different progress. Committed memory
  (VRAM/CRAM/VSRAM/RAM) is unaffected.

**Code-point-anchored captures** (breakpoint at `GameState_ObjectTest` 0x5C2F0 — same PC,
same Frame_Counter, where phase skew cannot alias the snapshot): at the matched anchors
fc 231 and fc 890, **full 64KB RAM CRC, VDP combined hash, and VDP register-file hash are
ALL identical OLD vs NEW.** (The middle anchor's code-point capture landed one frame
apart between builds — the intra-frame skew moved which frame the break fell in — and is
covered instead by the quantum capture at fc 474, where work RAM was already
byte-identical. The GUI framebuffer snapshot races at a mid-frame PC break and is
excluded here; the deterministic quantum-anchored comparison above is the framebuffer
evidence.)

**Oracle defect intel (for the oracle tree, observed during this A/B):** (i) the
main-loop reload wedge above (`startRomLoad` joins the previous reload worker ON the UI
thread); (ii) killing a bus client mid-`run_to`/`wait_for_break` can leave the
ExecuteThread STALLED while `status` still reports `running: true` (PC frozen,
frame_token static; pause/resume does not recover — process restart does); (iii) the
`screenshot` op's `path` parameter is ignored — captures land as timestamped files in
the default screenshots dir. (i)/(ii) belong with the deterministic-mode arbiter
investigation `debug_arbiter` instruments.

**Verdict: the PS state-identity bar is MET** — visible plane pixel-identical, all VDP
memories identical, work RAM identical at matched code points; the only deltas are the
mechanically-classified signatures of the saved cycles (interrupted-PC phase inside an
idle poll; mid-VBlank register-write progress at a frame-time pause). Protocol note for
later PS parcels: **prefer code-point anchors over frame-time quanta** for state
captures — a cycle-saving change always skews intra-frame phase, and quantum-boundary
snapshots alias that skew into apparent (spurious) register/stack deltas.
