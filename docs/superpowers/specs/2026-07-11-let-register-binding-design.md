# Design — Local typed-register binding (`let rN: Type`) · Spec 2 · C2

Written 2026-07-11 (Fable), from the ergonomics audit (bucket C2) + the tranche-7 ledger row.
Volence delegated the spec. **DESIGN ONLY**; erasing and **byte-neutral** by construction
(bindings emit nothing — the acceptance bar is the TouchResponse retrofit under the existing
byte gates).

## Problem

Typed registers exist only as **proc params**: a handler declaring `(a2: *Sst)` gets bare
field-displacement access (`x_pos(a2)`), but a proc that loads the pointer ITSELF has no way
to say so — TouchResponse self-loads a2/a3 and pays the qualified `Sst.field(aN)` spelling at
**thirteen sites** (ledger, tranche 7). Every proc that computes or loads a struct pointer
mid-body inherits the same tax; the audits rank it the most visible per-line ceremony in the
converted files. The same asymmetry applies to data registers (`d0: Angle` params work,
tranche 3; no body-position equivalent).

## Surface

```emp
proc TouchResponse () clobbers(d0-d7/a0-a3) {
        lea     Player_Slots, a2
        let     a2: *Sst                    // from here down, a2 is a typed Sst pointer
        tst.w   code_addr(a2)               // bare field form — was Sst.code_addr(a2)
        ...
        let     a2: *RingEntry              // rebinding: a2 now views a ring-buffer record
        move.w  x(a2), d0
}
```

- **`let <reg>: <Type>`** at statement position in a proc/asm body. `<reg>` is a register
  token (aN or dN); `<Type>` is anything a typed PARAM accepts — **one type surface, exactly
  the param rules** (`*Struct` pointer views on address registers; value newtypes like
  `Angle` on data registers). No initializer — the register already holds its value; the
  binding is the author's typing assertion about it.
- **Scope is lexical**: from the `let` to the end of the enclosing block (proc body, or the
  comptime-`if` branch it appears in), or until a subsequent `let` rebinds the same register.
  Deliberately NOT flow-sensitive — a read below the `let` gets the typed form regardless of
  how control arrived, exactly as a param's typing covers the whole body. (S2-D7's dataflow
  pass is where flow-sensitivity lives, later; see Trust below.)
- **Semantics identical to a typed param** after the binding point, including the tranche-7b
  field-namespace closure: the displacement slot on a typed register is CLOSED to the struct's
  fields — a bare const does not resolve there (the call-expr escape `interact_off()(a2)`
  stays the sanctioned spelling). Consistency with params is the point; no third behavior.
- **Grammar/disambiguation**: `let` is already reserved statement-leading (S2-D1). The form
  `let <register-token> : <Type>` (no `=`) is syntactically disjoint from any comptime value
  binding (`let name = expr`) — the register token class + the absence of `=` decide. No
  ambiguity with labels (register names are lexed as registers, never label names).

## Trust model

The binding is an **assertion, not a verification** — the same trust level as params today
(nothing verifies a caller actually passes an `Sst` pointer either). Lying (`let a2: *Sst`
then clobbering a2 with an unrelated value) is the author's bug; the S2-D6/D7 dataflow pass is
the recorded future home for checking binding consistency (a write to a bound register between
the `let` and a typed use is the obvious lint). Jot that as the pass's demand row — do NOT
gate this construct on it (params shipped on the same trust).

## What it deliberately is not

- **Not a value binding** — no initializer, no move emitted, zero bytes. The author loads the
  register with real instructions; `let` types what's already there (tenet 3: instruction
  lines stay asm).
- **Not flow-sensitive typing** — lexical only, v1.
- **Not an untype form** — no `let a2: _`; rebind to another type if the register's role
  changes, or let the binding ride (a stale binding on a dead register is harmless — reads
  are what it affects). Add an untype spelling only on demonstrated demand.

## Machinery

Small and front-end-only: the body-statement parser gains the `let reg: Type` arm; the
operand-resolution context that today consults the PARAM register-type map consults a
body-scoped overlay of it (a scope stack pushed/popped with blocks, entries written by `let`).
Field-displacement resolution, the namespace closure, and `[asm.splice-kind]`-class checks all
read the same map they read for params — no new lowering, no IR change, nothing in
`sigil-ir`/`sigil-link`.

## Acceptance

1. **TouchResponse retrofit**: `let a2: *Sst` / `let a3: *Sst` after the self-loads; the 13
   qualified sites go bare; **byte-identical** under the existing collision gates (both
   shapes).
2. The ring-buffer half of rings.emp is NOT retrofitted yet — its 6-byte entries want the
   record-over-raw-RAM view (ledger, demand 1/2), which `let aN: *RingEntry` will CONSUME
   when that construct lands; note the pairing, don't front-run it.
3. Negative probes: `let` on a non-register name errors; a bare const in displacement
   position on a `let`-typed register stays closed (7b parity); rebinding works; a binding
   inside a comptime-if branch doesn't leak past the branch.

## Decisions taken (standing autonomous arrangement)

- Keyword `let` (not `bind` — that's the patch/bind slot primitive; not bare `a2: *Sst` —
  too label-shaped at line start for a human scanning columns).
- Lexical scope + rebinding, no untype form, param-identical semantics incl. namespace
  closure.
- dN value-type bindings included (one surface with params), though the demand evidence is
  address registers.
