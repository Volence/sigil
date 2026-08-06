# Niche-sentinel Option — C1 parcel packet (Track C)

Spec: `docs/superpowers/specs/2026-08-03-niche-option-spec.md` (+ the §6 amendment,
59ca7884). Scope: C1 only — the `T ? sentinel` newtype, `.none`, the three lints,
trusted `assume_some!`, the two flagship corpus conversions. C2 NOT built.

## What shipped

- **Grammar.** New `?` lexer token; `newtype NAME = UNDERLYING [where LO..HI] [? SENTINEL]`
  (`NewtypeDecl.sentinel`). Both spellings supported: two-decl (`SlotId ? $FF`)
  and inline-ranged (`u8 where 0..$FE ? $FF`).
- **`.none`.** A two-segment member on any option newtype → a typed const of the
  sentinel (`Value::Typed{Newtype(opt), Int(sentinel)}`); erases to the sentinel
  byte in an operand (byte-identical to the raw literal, proven).
- **`[option.niche-overlap]` (error).** Once-per-compile evaluator pass
  (`validate_option_newtypes`, wired in `lower_module_inner`): the sentinel must
  lie OUTSIDE the payload's effective range — the option's own `where` refinement
  wins, else the underlying's range; an unranged payload makes the full-range
  overlap immediate.
- **`[option.unguarded-use]` (error).** The existing type-slice engine, extended
  with an option→payload map: where a niche-option flows into its own payload
  slot, the option id REPLACES the generic `[call.slot-type-mismatch]` at that
  site — one id per site, never both (`FiringKind`).
- **`[option.raw-sentinel]` (warn, build-emitted).** At an option-typed struct
  FIELD store/compare, a raw sentinel immediate (not `#opt.none`) nudges toward
  `.none`. Detected locally at lowering (`check_raw_sentinel`) so it rides the
  warn-tier build channel; register-held-option positions are the ledgered gap
  (they need the cross-file type-slice dataflow, which does not run per-file).
- **`assume_some! <reg>, <Payload>` (trusted, register-only).** A new bang-family
  statement lowering to the reserved zero-byte `assume_some` mnemonic (skipped at
  emission like `jbra`; 0 cycles; writes no register) carrying the payload in
  `as_type`, which the type-slice lattice reads as an `as Payload` register bless.
- **Corpus (both flagships).** `Sst.slot_tag: TagRef` (the `$FF` field niche;
  untagged writers now `#TagRef.none`) and `EntityWindow_EntryForSection
  out(d0: EntryRef)` (the `-1` register niche) → its four `tst.w/bmi`-guarded
  callers `assume_some! d0, EntryIndex` into `EntityLoaded_Clear(d0: EntryIndex)`.

## Gates (own-run, post-rebase onto sigil 686e6f62 / aeon 76013f2)

- Byte bar SEVEN targets byte-NEUTRAL: `refreeze --check` OK (tip enum-dispatch,
  chain 48); every CRC equals the frozen chain. Parcel is byte-neutral by design.
- Full strict: 3461 passed + 4 ignored = 3465 = branch `#[test]` total; 0 failed.
  (baseline 3451 + 14 new `niche_option.rs` tests.)
- Warn-tier: firing id SET identical ×7 — `[option.raw-sentinel]` zero-fires on
  the corpus (conversions use `.none`), so `warn_tier_corpus` baseline UNCHANGED.
- Corpus option/slot firings: ZERO (all four `assume_some!` sites clear the
  unguarded-use that the typed `EntityLoaded_Clear` slot would otherwise raise).
- clippy `-D warnings` clean.
- Both lints probed both polarities; niche-valid asserts a clean build (non-vacuity).

## Findings

- **step-3 (design catch).** `assume_some!` cannot be a `TrapKind` (those lower to
  `illegal`); modelled instead as a reserved zero-byte mnemonic that IS a CFG node,
  so the type-slice IN/OUT dataflow is edge-precise (the marker sits only on the
  guard's not-equal edge) with no CFG surgery — it reuses the existing `as`-bless.
- **step-3 (bug caught + fixed in-parcel).** The inline `where … ? …` form first
  reported the payload as bare `u8` — `check_option_niche` resolved the underlying
  and ignored the option's OWN `refine`. Fixed to prefer `decl.refine`; pinned by
  `inline_where_then_sentinel_parses_and_resolves`.
- **step-5 (cross-module).** A struct-FIELD niche-option must live where the
  Coord-family field types live (`engine.types`), not in the struct's own module —
  every Sst importer resolves the field type there. `TagRef` relocated accordingly.

## Ledgered gaps (C1 boundaries, not bugs)

- `[option.raw-sentinel]` covers option-typed FIELD positions only; the register-
  held-option compare position needs the type-slice dataflow (corpus-analysis-only,
  not per-file) — deferred.
- `assume_some!` is REGISTER-only; a var/field-access-path retype needs a new
  keying structure (flow-sensitive type state is register-keyed today).
- `option_field_in_operands` reads the EXPLICIT `Struct.field(reg)` qualifier; the
  bare `field(reg)` form on a typed register is not yet resolved for raw-sentinel.
- C2 (guard-dominance promotion of `assume_some!`) NOT built — its substrate
  (dominance + compare-const-location threading) is confirmed ABSENT (2026-08-06).

## Neither-bucket headline

The parcel is a from-scratch language feature (not a port), so there is no
step-1 byte-diff verifier stage; the byte bar proves the corpus adoption
byte-neutral instead, and the whole feature's correctness rides the `#[test]`
suite + both-polarity probes.
