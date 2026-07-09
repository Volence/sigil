# Test vector provenance

Real committed compressed-asset blobs, copied read-only from `s2disasm`
(not the sigil repo) to serve as round-trip gates.

| File | Source path | Compressed size | Decompressed size |
|---|---|---|---|
| `numbers.nem` / `.raw` | `s2disasm/art/nemesis/Numbers.nem` | 217 B | 576 B |
| `seal.nem` / `.raw` | `s2disasm/art/nemesis/Seal.nem` | 284 B | 448 B |

Both decompressed sizes are multiples of `0x20` (576/32=18, 448/32=14),
satisfying the compressor's tile-granularity constraint.

`golden_nemesis.bin` is the compression golden: `numbers.raw` (576 bytes)
compressed with the vendored `ClownNemesis_Compress(accurate=0, ...)`
through a correctly-rewinding read callback (see `src/lib.rs`'s
`compress()` for the production implementation, and
`tests/rewind_regression.rs` for the RED/GREEN story behind why the
callback must rewind). Round-trip verified: decompressing
`golden_nemesis.bin` reproduces `numbers.raw` exactly. This is a
"byte-exact vs clownnemesis@7abcddc" regression vector, not vs Sega's own
compressor (`accurate=1`/Fano mode is out of scope for this task per CR1).
