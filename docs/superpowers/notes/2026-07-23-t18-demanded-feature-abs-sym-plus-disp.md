# t18 demanded feature — `[lower.abs-sym-operand]`: abs-symbol + `(d16,An)`

**Surfaced by:** parallax step-1 transcribe (the campaign's feature-discovery engine
working as designed — demanded-features law).
**First & only consumer today:** `parallax.emp` `Vscroll_Write` whole-plane branch:
```
move.l  (Vscroll_Factor).w, VDP_DATA_OFF(a5)     // AS twin: move.l (Vscroll_Factor).w, VDP_DATA-VDP_CTRL(a5)
```
Grep confirms **no shipped `.emp` uses an abs-symbol operand combined with a
`(d16,An)` operand** — parallax is the first. Byte-identity requires this single
instruction (splitting into load-to-reg + store changes the bytes), so the port's
step-1 byte gate is **blocked** on this lowering until it ships.

## The gap (root cause, code-verified)

`crates/sigil-frontend-emp/src/lower/code.rs:963-975` — in the abs-sym lowering, ANY
operand carrying extension words that FOLLOWS the symbolic operand is rejected:

```rust
other if operand_has_ext_words(other) && target.is_some() => {
    // Ext words AFTER the sym operand would land BEHIND the abs field and move
    // the fixup offset — still deferred. (BEFORE it they precede the abs field,
    // which stays last — allowed since tranche 5.)
    push_err(diags, span, "[lower.abs-sym-operand] …not yet supported");
    return;
}
```

The reason is the **RelaxAbsSym** append-only model: a *relaxable* abs (`.w`↔`.l`)
must have its ext words LAST so widening `.w`→`.l` only appends 2 bytes without
shifting a trailing operand's ext words. Correct for the relaxable case.

**But when the width is PINNED** (`(Sym).w`/`(Sym).l` — captured as `pinned:
Some(bool)` at code.rs:938/946-948), there is **no relaxation** — the abs field's
size is fixed and its byte offset within the instruction is statically known. A
trailing `(d16,An)` is then safe: nothing shifts. The rejection is over-broad; it
fires before the pinned check and blocks a case that is actually encodable.

## The fix (bounded; TDD-ready)

Extend the **pinned-width** path only (leave the relaxable case rejecting):

1. In the `operand_has_ext_words(other) && target.is_some()` arm, do NOT reject when
   the sym operand is pinned (`pinned.is_some()`); instead build the operand
   normally (like the trailing `other =>` arm) so both source and dest ext words are
   emitted in order.
2. **Fixup offset recomputation** — the delicate part: with a pinned abs that is NOT
   last, the fixup offset is no longer `short_bytes.len() - 2`. It is the byte
   position of the abs ext field = `opcode_words*2 + (ext words of operands BEFORE
   the abs)`. For `move.l (abs).w, (d16,An)`: opcode 1 word, abs is the FIRST
   operand (source, 0 preceding ext words) → fixup offset = 2, width `.w` (1 word);
   the dest disp follows at offset 4. Compute the offset from the lowered `M68kInst`
   operand positions, not the `len()-2` shortcut, whenever `pinned && !abs_is_last`.
3. Keep the internal consistency guard (1021) meaningful — for the pinned path the
   two-candidate diff no longer applies (single fixed encoding), so gate the
   `long_bytes.len() == short_bytes.len()+2` check to the relaxable path.

## Verification (TDD)

- Test file: `crates/sigil-frontend-emp/tests/lower_code.rs` (existing
  `[lower.abs-sym-operand]` cases at ~:874).
- RED first: `move.l (Sym).w, 4(a5)` and `move.l (Sym).w, -4(a5)` (the parallax
  form: `VDP_DATA_OFF = -4`) must lower to the exact 6-byte encoding
  `21 F8 xxxx FFFC`-class (opcode + abs.w ext + disp ext), the abs ext word linked
  to Sym's low word. Also a NEGATIVE guard: the *relaxable* form (bare `Sym`, no
  `.w`) with a trailing `(d16,An)` STILL rejects (append-only model unchanged).
- The parallax byte gate (`parallax_port` plain+debug) is the integration proof:
  the whole-plane `move.l (Vscroll_Factor).w, VDP_DATA_OFF(a5)` byte-matches the ROM.

## Scope note

Corpus-wide capability (any future port needing abs-sym + displaced operand rides
it), narrow diff (one arm + the offset computation), delicate (a wrong fixup offset
= wrong bytes at link — hence pinned-only + explicit offset-from-positions + the
negative relaxable guard). On the parallax byte-gate critical path.
