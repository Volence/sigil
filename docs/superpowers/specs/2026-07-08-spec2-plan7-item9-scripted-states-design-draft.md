# Design — Spec 2 · Plan 7 backlog #9: scripted states / coroutines on the `dispatch` seam

Date: 2026-07-08 (Fable, overnight session, Step 3 — DESIGN ONLY per the locked scope; NO
implementation tonight). Status: **RATIFIED by Volence, 2026-07-08 (day checkpoint)** —
D9.1–D9.5 approved as proposed; the six open questions resolved (rulings recorded inline
below); Q6 promoted to **D9.6**. 9a and 9b are cleared for implementation. Inputs: research Part II/R2 + T1-c (the
scripted-coroutine merge), #6's reserved `Member: { … }` seam (erroring specifically today),
Batman & Robin's threaded-code object model (the `$0820` yield-PC-as-state pattern), the
shipped `offsets`/`dispatch`/overlay/jbra machinery, and tonight's pitcher_plant corpus
(the first end-to-end object the construct must eventually subsume).

## The one-paragraph thesis

R2's finding: byte-command DSLs (animation/palette/SMPS) and state dispatch are the same
construct at different scales, and the most advanced engines (Treasure/Batman-class) fuse
them into a threaded-code interpreter where **the saved script PC IS the object's state** —
a coroutine. `.emp` should ship that as a first-class construct with `yield` as a language
primitive and a compiler-tracked resume point, lowering to the encoding-agnostic dispatch
machinery #6 already built. Per tenet R1 (enable, don't impose), it must coexist with the
plain proc-pointer model pitcher_plant uses today — an engine chooses script, dispatch, or
raw procs per object, and mixing is legal.

## Decisions proposed (D9.1–D9.5, for ratification)

- **D9.1 — the reserved seam resolves as the SMALL increment, not the coroutine.**
  `dispatch Name (encoding: …) { Member: { <instructions> } }` inline bodies become sugar
  for an anonymous per-member proc (hygienic label, same encoding row as a named target).
  This is what the seam's error message has promised since #6, it is mechanical over
  shipped machinery, and it deliberately does NOT introduce state/yield semantics.
  The coroutine gets its own surface (D9.2) rather than overloading dispatch bodies —
  a dispatch is a TABLE; a script is a PROGRAM. Conflating them bends tenet 1.
- **D9.2 — the coroutine construct: `script`.** Sketch (surface deliberately unfrozen):

  ```
  script pitcher_plant_brain (a0: *Sst) {
      loop {
          wait_frames WAIT_TIME              // comptime helper, as today
          yield                              // save resume point, return to engine
          ...
      }
  }
  ```

  Semantics: `yield` compiles to "store <resume label> into the object's resume slot;
  rts to the dispatcher." Each yield site generates a resume member in a hidden
  `dispatch`-encoded table (word_offsets or long_ptrs — encoding-agnostic like #6;
  Treasure's pre-shifted-index variant is the ledger's third encoding). The **resume slot
  is a typed Sst field** (declared, not conventional: `resume: ScriptPc` — a newtype the
  construct defines; writing a raw int to it is a type error — totality). The engine-side
  dispatcher is ordinary user code (`movea`+`jmp` per Batman, or the classic indexed jsr);
  the construct only guarantees the table+slot contract, keeping the engine unimposed.
  Compiler-tracked resume-point typing is the differentiator: a script's yields form a
  closed set, so "jumped to a stale/garbage resume value" becomes unconstructible.
- **D9.3 — the byte-command DSL (T1-c) is DEFERRED, with a re-evaluation gate.** Not in
  #9. Rationale: T1-c's own scope caution (largest single feature); the data half is
  already covered by `offsets`/`dispatch`; the animation-script case (182 blobs) should
  first be attempted as a PRELUDE-LIBRARY pattern (comptime fns + arrays + guards emitting
  the classic `[speed, frames…, $FF]` form — tonight's corpus proves the ingredients) during
  the aeon migration; SMPS, the case that genuinely needs the general DSL, gates on #7
  banks anyway. Re-evaluate when the sound migration starts — the same
  build-on-demonstrated-need logic that deferred jbcc (D2.18).
- **D9.4 — staging.** 9a = D9.1 inline bodies (small; could ride any nearby branch).
  9b = `script`/yield MVP: `loop`/straight-line/`yield`, comptime helpers legal inside,
  lowering onto dispatch tables + the typed resume slot; pitcher_plant's brain REWRITTEN
  as the exhibit alongside the proc version (both compile; the spec argues equivalence).
  9c = value-carrying yields (`yield frames(5)` — the dispatcher-side protocol), `for`
  loops, script-calls-script. 9d = the byte-command DSL, gated per D9.3.
- **D9.5 — relation to `routine`.** In the script model, the SST `routine` pointer and the
  resume slot are the SAME storage (the script PC is the state). The prelude's `routine`
  helper (tonight's `pea`-based pointer store) is the manual spelling of what `yield`
  automates; the design should name this equivalence and let both coexist (manual procs
  keep using `routine`; scripts own their slot).

## Open questions — RESOLVED (Volence, 2026-07-08 day checkpoint)

1. **Surface name: `script` — RATIFIED.** A distinct construct gets a distinct contextual
   opener (§10 headroom rule); a script compiles to resume-segments, not one linear
   routine, so overloading `proc` would hide the model change.
2. **Bare `yield` in 9b — RATIFIED** (Volence delegated; Fable's call): timers stay
   helper-managed (`wait_frames` sugar) in the MVP; the value-carrying protocol
   (`yield frames(N)` + the dispatcher-side contract) is 9c, designed against a real
   consumer.
3. **Typed Sst field — RATIFIED.** The resume slot IS the engine's existing next-frame
   pointer (the storage today's `routine` helper writes), declared as a normal Sst field
   with the construct's newtype (`resume: ScriptPc`); the engine keeps choosing the
   offset. The compiler's contribution is the type: only real resume points / proc
   entries are writable — stale/garbage jump targets become compile errors. (Clarified
   at the checkpoint: this is not new storage, it is the `0(a0)`-era routine pointer,
   typed; D9.5's equivalence is the load-bearing fact.)
4. **Z80: out of scope for 9b (68k first) — noted**, SMPS end-state revisits (gates on #7
   anyway per D9.3).
5. **Shipped encodings only in 9b — RATIFIED**: word_offsets + long_ptrs (both
   byte-proven by #6). Pre-shifted index (Ristar/Treasure ×4) waits for a demanding
   port — the jbcc demonstrated-need logic.
6. **→ promoted to D9.6, ratified as written — the per-frame epilogue.** Yield does NOT
   freeze the object — the engine visits it every frame, and today's procs end with
   `jbra Draw_Sprite` (or a mark-offscreen variant, or nothing for invisible
   controllers), never a bare return. `yield` lowers to "store resume point, then
   `jbra <epilogue>`" — the same exit every proc hand-writes. An epilogue is declared
   once per script (`script brain (a0: *Sst) shows Draw_Sprite { … }`-shaped) with
   per-site override (`yield Draw_Sprite`); a bare `yield` with NO epilogue declared is
   a compile error rather than a silent rts (an object that never draws is the footgun).
   `wait_frames N` is per-frame sugar (tick timer, yield through the epilogue until
   elapsed), not a blocking wait.

## What this is NOT (scope guards)

- Not an object system: no update/render phases (T2-f), no collections (T2-d/h), no
  hot-swap IRQ (T2-e) — separate ledger items, explicitly out.
- Not an interpreter runtime: the construct emits tables + resume stores; the dispatch
  loop remains user assembly (tenet 3).
- No implementation tonight — this document is the deliverable.
