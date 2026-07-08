# Design note — Plan 7 #9c: the `script` construct's second tranche (SHORT, direction-level)

Date: 2026-07-08 (Fable, post-#7 checkpoint). Status: **DIRECTION NOTE, NOT RATIFIED** — the item-9
checkpoint ruled 9c "needs no new ratification, just its own short design note"; this is that note.
Implementation waits for demonstrated need (the standing jbcc/9d logic); the expected demand source
is the object-code migration campaign, where real brains will show which of these four earn their
keep. Basis: D9.1–D9.6 (ratified), R9b.1–R9b.12 (shipped 9b), §5.6 in the empyrean spec.

## The four deferred surfaces, in demand order

1. **`wait_frames(n)` / value-carrying yields.** The dominant real idiom (timer-tick loops around a
   yield). Direction: `yield` stays bare; `wait_frames` is a PRELUDE comptime helper that expands to
   the store/decrement/branch shape around a bare yield — no new construct, no new lowering. Only if
   the helper proves inexpressible (it needs a resume point INSIDE its own expansion — check whether
   a comptime fn can splice a `yield`; today `yield` is a ScriptStmt, not an expression, so it
   CANNOT) does 9c grow a `yield`-accepting splice position or a first-class `wait_frames`
   statement. That check is the first implementation task.
2. **`break` (out of `loop`).** Cheap and self-contained: desugars to a `jbra` past the loop's
   backedge label, same hygiene scheme (`__loop$<d>$end`). No design risk; bundle with whichever
   tranche first wants it.
3. **`for` in script bodies.** Comptime-bounded repetition around yields (unrolled, per 9b's
   single-eval flattening) is nearly free; RUNTIME loops with a counter are just `loop`+`break`+an
   Sst field and should NOT get dedicated surface until a port shows the pattern is common.
4. **script-calls-script.** The expensive one: nesting means a resume STACK or continuation
   chaining, which the one-word `ScriptPc` slot cannot express. Direction if demanded: caller
   stores the callee's table base in a second typed slot (prelude decides the field), and `call`
   is a distinct statement with its own epilogue contract. Do NOT attempt to make bare `jsr` into
   a script "just work" — the resume-table invariant (R9b.2: member 0 = entry, ordinals pre-scaled)
   doesn't survive an implicit call graph. Needs its own ratified mini-design when a real object
   demands composition.

## Constraints that carry over unchanged

- Never a silent rts: every new control edge lands on the D9.6 epilogue contract
  (`[script.no-epilogue]` class stays total).
- Hygiene: all synthesized labels keep the `$`-containing unlexable namespace and program-unique
  qualification (R9b.11).
- The hidden-table layout (R9b.2) is FROZEN — 9c may add rows, never reinterpret them; the proc
  version's byte pin is the regression net, per 9b precedent.
- Z80 scripts stay out of scope until the Z80 relaxation ladder lands (L-H.4 / S2-D13(b)).

## Gate

Take 9c up when the migration campaign produces a brain that (a) hand-writes the wait_frames shape
three times (rule of three → tranche 1+2), or (b) genuinely composes scripts (→ tranche 4 with its
own design). Until then this note is the whole 9c artifact.
