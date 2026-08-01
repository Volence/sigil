# The `embed` data expression (binary include) — design

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
