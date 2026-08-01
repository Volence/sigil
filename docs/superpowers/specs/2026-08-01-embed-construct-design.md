# The `embed` data expression (binary include) — design

**Status: SHIPPED (conv-h2, 2026-08-01).** The construct is live in the `.emp`
frontend as the `embed(...)` comptime data expression (`eval_embed` in
`crates/sigil-frontend-emp/src/eval/sandbox.rs`), proven byte-exact against a
golden fixture (`tests/sandbox_embed.rs`, 9 tests). Shipped surface vs this
design — deviations, all deliberate and ledgered in
`docs/superpowers/notes/2026-08-01-conv-h2-embed.md`:
- **Slice form is NAMED, not positional.** Shipped: `embed("p", skip: N,
  len: M)`. Spec §1 floated positional `embed("p", offset, len)` "only if a
  consumer needs it" — no consumer does (boot_data embeds the whole blob), and
  the named form was already shipped + tested at T1. KEPT named.
- **`[u8; _]` inference is NOT the spelling; omitting the annotation IS.**
  `_` in array-length position parses as an ordinary path and is not an
  inference hole (confirmed in `layout.rs::resolve_type`). The inferred-length
  intent is served by omitting the type entirely (`pub data BootBlob =
  embed(...)`), which the `Value::Data` lowering already length-infers. The
  spec's own §1 contingency ("if `_` is not [supported] … the explicit-length
  form is the v1 whole") governs: v1 = omit-to-infer, or `[u8; N]` to assert.
- **Length assertion is `[emit.size-mismatch]`, not `[embed.length-mismatch]`.**
  An explicit `[u8; N]` over an embed routes through the SAME general
  data-item size check every initializer uses (`emit.rs::lower_data_value`),
  which already reports actual-vs-declared. A bespoke `[embed.length-mismatch]`
  would be strictly narrower for no gain (port-loop "better not same"). KEPT
  the general diagnostic.
- **`[embed.not-found]`** now names the RESOLVED ABSOLUTE path (spec §2), fixed
  from the T1 `[embed.read] <relative>`. `[embed.range]`, `[sandbox.no-root]`,
  `[sandbox.path-escape]` unchanged. Freshness: the capture ledger records each
  embed's SHA-256 + length (`record_capture`), so a changed binary is tracked.
- **Consumer status (§3): the #12 boot_data blob half is now expressible; the
  org-$3FE hole half is NOT** — `.emp` has no absolute-`org`/hole surface, and
  the hole is filled by a SEPARATELY-chained module (`z80_init.emp`), live for
  the demo/demo.debug (sound-OFF) targets. Per the §3 STOP rule the full
  boot_data → `.emp` port STOPS with that finding (numbers in the parcel note);
  boot_data.asm stays AS-residual. Parcel J (#25/#26/#27) shipped as deletes.

**Status: RATIFIED (overseer).** Authorized at the Parcel-H countersign: #12
boot_data is blocked on the absence of a binary-embed surface in `.emp` —
`BINCLUDE` exists only in the AS frontend (`eval.rs::directive_binclude`).
Real-world assembly relies on binary includes constantly (art, blobs, baked
tables); the language's adoption goal requires a first-class spelling. Small
surface, no new semantics domain.

## §1 — Surface

`embed("relative/path.bin")` is a **data expression** usable where a data
item's initializer is expected:

```
pub data BootBlob: [u8; _] = embed("engine/sound/generated/z80_boot.bin")
```

- The path is relative to the AEON ROOT (the build's `--root`), matching
  BINCLUDE's resolution — one convention, no per-file relativity surprises.
- `[u8; _]` — the length infers from the file (the `_` spelling already used
  for inferred array lengths; if it is not, the implementer reports and the
  explicit-length form below is the v1 whole).
- An explicit length `[u8; N]` asserts the file is EXACTLY N bytes
  (`[embed.length-mismatch]`, with actual vs declared).
- Optional slice form `embed("path", offset, len)` — the BINCLUDE
  `start,len` analog; out-of-range is `[embed.range]`. Only if boot_data (or
  a known consumer) needs it in v1; otherwise ledger it.
- The file participates in build freshness (a changed binary rebuilds the
  module — however comptime inputs are tracked today; if they are not, that
  is a REPORTED limitation, not silently stale output).

## §2 — Semantics

- Bytes are emitted verbatim at the item's place; no endianness, no
  interpretation. Z80/68k agnostic.
- comptime visibility: `sizeof` of the item works as usual. Reading embedded
  BYTES at comptime is OUT of v1 (no demand; keeps the eval pure).
- A missing/unreadable file is a compile error naming the resolved absolute
  path (`[embed.not-found]`).

## §3 — Consumers and the proof

- First consumer: #12 `boot_data` — the mid-table resident-Z80-blob BINCLUDE
  + the conditional `org $3FE` hole (the hole expresses as the existing
  data/align/pad surfaces around the embed; if the org-hole shape needs more,
  STOP with the finding). Bar: six-target byte identity (the blob is
  shape-varying sigil-emitted output — the embed must read the SAME generated
  file the AS build reads).
- The parked editor exports (#25/#26/#27) are NOT consumers — they are
  pre-ruled DELETE (Parcel J) and ride the same parcel as removals.
