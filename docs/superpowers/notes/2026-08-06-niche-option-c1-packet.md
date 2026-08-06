# Niche-sentinel Option — C1 parcel packet (Track C)

Spec: `docs/superpowers/specs/2026-08-03-niche-option-spec.md` (+ the §6 amendment,
59ca7884). Scope: C1 only — the `T ? sentinel` newtype, `.none`, the three lints,
trusted `assume_some!`, the two flagship corpus conversions. C2 NOT built.

## The panel round is the headline

The A/B/C lens panel found **two soundness defects and a dead-scaffolding class**
in code whose gate was already fully green. Foregrounded because "green" is
exactly what made them dangerous:

1. **The regression test pinned an UNSOUND guard as passing** — and it was the
   repo's only "how to use `assume_some!`" exemplar. Payload `Slot = u8 where
   0..$7E`, guard `tst.b d0 / beq`: that branches away when d0 == 0, but **0 is a
   valid payload**, so the `$FF` sentinel sailed onto the fall-through path, got
   retyped, and reached `Use(d0: Slot)`. The one excluded path was the only
   already-safe one. It passed because C1 TRUSTS the marker by design — so the
   test read as validation of a shape a guard-dominance checker is required to
   REFUSE. Fixture rewritten to the spec §2 form (`cmpi.b #SlotRef.none, d0 /
   beq`), with the reasoning stated in-test so it cannot silently regress.
2. **`[option.niche-overlap]` did not normalize the sentinel to the payload's
   storage** — one unbounded-i128 interval test, so `newtype O = u8 ? -1` was
   accepted (`-1 ∉ [0,255]`) while `.none` emits `$FF`, byte-identical to payload
   255: the niche was never carved. `? $100` on u8 truncated to 0, likewise a
   valid payload. **My own register flagship uses the `? -1` spelling**
   (`EntryRef = EntryIndex ? -1`, sound only because its payload is signed), so
   the idiom was one copy-paste from a silently unsound option with a green gate.
   Fixed: the sentinel is reinterpreted into the underlying's width+signedness
   before the interval test, and a sentinel the storage cannot hold at all is
   refused. Three new tests cover both polarities and the width case.
3. **`sentinel_of` was a write-only field** on a public enum variant — never read
   from any `CodeItem` (the only consumer read the LOCAL computed before the push),
   yet it forced 19 mechanical `sentinel_of: None` lines across seven files and its
   doc asserted two false things. Deleted; the local stays.

## What shipped

- **Grammar.** New `?` lexer token; `newtype N = U [where LO..HI] [? SENTINEL]`.
  Both spellings work: two-decl (`SlotTag ? $FF`) and inline (`u8 where 0..$FE ? $FF`).
- **`.none`** — a typed const of the NORMALIZED sentinel; erases to the sentinel
  byte (byte-identical to the raw literal, proven). Resolution keys on the decl
  first (like the `offsets`/`dispatch`/`table` steps), so a typo'd member or a
  `.none` on a non-option is a loud error instead of falling through to the
  link-symbol path and surfacing as an undefined symbol at link.
- **`[option.niche-overlap]`** (error, once-per-compile) — storage-normalized, as above.
- **`[option.unguarded-use]`** (error) — the type-slice engine's own message where a
  niche-option flows into its own payload slot; the option id REPLACES the generic
  `[call.slot-type-mismatch]` at that site, never both. It now carries its own
  REMEDY (guard the sentinel, then `assume_some!`) — a distinct id whose text was
  byte-identical to the generic row told the author nothing.
- **`[option.raw-sentinel]`** (warn, build-emitted) at option-typed struct fields.
- **`assume_some!`** — register-only, trusted, zero bytes/zero cycles. Now validates
  its SHAPE: `[option.assume-not-option]` refuses an undeclared payload name (which
  previously blessed the register with a type the engine ignores — a silent no-op
  marker reading as a checked one) and a newtype no option wraps;
  `[asm.assume-payload]` names what it got.
- **Marker taught to the consumers that key on mnemonics**: `cc_inert_data_op` (it
  was CC-opaque, silently dropping edge credit in `Cfg::valid_edge` for a marker
  between a conditional-out call and its guard) and `z80_cycles::instr_cost` (a Z80
  `@budget` proc hard-bailed `[cycles.unknown-op]`).
- **Shared bounds ladder.** `effective_scalar_bounds` had already DRIFTED from
  `check_value_fits_ty_labeled` — missing the `CycleStack::Refine` guard, terminating
  only because the sibling's guard caught the re-entry. Both now go through one
  `newtype_refine_bounds`.
- **Corpus.** `Sst.slot_tag: TagRef` (field niche, `#TagRef.none` writers) and
  `EntityWindow_EntryForSection out(d0: EntryRef)` (register niche) → four guarded
  callers `assume_some! d0, EntryIndex` into `EntityLoaded_Clear(d0: EntryIndex)`.
  Both flagships use the **two-decl** form (ruling): the inline form's underlying is
  a prim, so `collect_option_payloads` never registers it and `[option.unguarded-use]`
  cannot fire on it — which had made the ZERO-firings claim vacuous for `TagRef`.
  `SLOT_TAG_UNTAGGED` (the orphaned `$FF` mirror this feature exists to eliminate)
  and its two dead imports are gone; three falsified header comments corrected.

## Gates (all own-run, post-rebase; see final report for the exact bases)

- Byte bar SEVEN targets byte-NEUTRAL; `refreeze --check` OK. Warn-tier firing id
  SET identical ×7 — `[option.raw-sentinel]` zero-fires (corpus uses `.none`), so
  the `warn_tier_corpus` baseline is UNCHANGED.
- Full strict green with closing arithmetic; clippy `-D warnings` clean.
- **Corpus non-vacuity proven by REVERT PROBE**: deleting one `assume_some!` from
  `rings.emp` produces exactly one `[option.unguarded-use]` (RingCollision →
  EntityLoaded_Clear, d0 expects EntryIndex but found EntryRef, with the remedy);
  restoring it returns to zero. The zero is a real zero.
- Both polarities probed per lint; plus a non-vacuity twin (same guarded body,
  marker removed → still fires) and a join-leak probe (marker on one path, use
  after the join → still fires; the meet degrades to untyped, so it reports the
  generic id — pinned as such).

## Ledgered gaps (C1 boundaries)

- `[option.raw-sentinel]` covers option-typed FIELD positions via the explicit
  `Struct.field(reg)` qualifier only; register-held options and the bare
  `field(reg)` form on a typed register need the corpus-analysis dataflow.
- `assume_some!` is REGISTER-only; a var/field-path retype needs new keying.
- The inline-form / `collect_option_payloads` asymmetry (new ledger row).
- C2 NOT built — substrate (dominance + compare-const threading) confirmed absent.

## Neither-bucket headline

No step-1 byte-diff verifier stage exists for this parcel (it is a from-scratch
language feature, not a port): the byte bar proves the corpus ADOPTION
byte-neutral, and correctness rides the test suite, the both-polarity probes, and
the revert probe. The panel's two soundness findings both lived in exactly the gap
that shape leaves — a green byte bar says nothing about whether a type rule is
sound, and neither does a test that pins the wrong shape.
