# Vendored clownnemesis source

- **Upstream**: https://github.com/Clownacy/clownnemesis
- **Pinned revision**: `7abcddc7e220f33c1e03ccaecb46a0c0323182c8`
- **clowncommon submodule revision**: `37d1efd90725a7c30dce5f38ea14f1bc3c29a52f`
  (upstream path `clowncommon/`, a single-header dependency — vendored below
  at `clowncommon/clowncommon.h`, preserving that exact relative subpath
  because `common-internal.h` and `compress.c` `#include
  "clowncommon/clowncommon.h"`)
- **Date vendored**: 2026-07-09

These files are verbatim copies from the upstream repository. They are
plain C89, callback-based streaming API, compiled unmodified by `build.rs`.
No shim is needed for this crate (unlike `sigil-clownlzss-sys`) — the C API
is directly `extern "C"`-compatible. Do not hand-edit these files here — any
fix belongs upstream (or, if truly Sigil-specific, should be re-derived and
re-vendored with a note explaining the divergence).

## Vendored files and their upstream paths

| Vendored path | Upstream path |
|---|---|
| `compress.c` | `compress.c` |
| `compress.h` | `compress.h` |
| `decompress.c` | `decompress.c` |
| `decompress.h` | `decompress.h` |
| `common.h` | `common.h` |
| `common-internal.c` | `common-internal.c` |
| `common-internal.h` | `common-internal.h` |
| `LICENCE.txt` | `LICENCE.txt` |
| `clowncommon/clowncommon.h` | `clowncommon/clowncommon.h` (submodule) |
| `clowncommon/licence.txt` | `clowncommon/licence.txt` (submodule) |

## License

Verbatim ISC-style permission text by Clownacy (no SPDX tag upstream, but the
wording is the standard ISC license). See `LICENCE.txt` (clownnemesis) and
`clowncommon/licence.txt` (clowncommon). The crate's `Cargo.toml` `license`
field is set to `"ISC"`.

## Load-bearing wrapper contract: input callback must rewind on EOF

`ClownNemesis_Compress` (`compress.c`) is a **multi-pass** encoder: it calls
the input read callback across several full passes over the same data —
`ComputeCodes` runs `ComputeCodesInternal`/`FindRuns` in regular mode, then
XOR mode, and (if regular mode won) a third time in regular mode again; then
`EmitCodes` runs `FindRuns` a further time to actually emit the bitstream.
Each pass reads from byte 0 through EOF again.

`ReadByte` (`common-internal.c`) does NOT longjmp on `CLOWNNEMESIS_EOF` during
compression (`state.common.throw_on_eof = cc_false` is set in
`ClownNemesis_Compress`) — it just returns whatever the read callback
returned, including the `CLOWNNEMESIS_EOF` sentinel, to the caller. This means
the read callback's contract is: "after you've returned EOF once, you WILL be
asked to read again — rewind your internal cursor back to position 0 so the
next pass sees the same data from the start." A naive read callback that
exhausts once and then always returns EOF causes `ClownNemesis_Compress` to
report success while silently emitting a truncated stream (recon-verified:
3-byte output instead of the correct ~218 bytes for a 576-byte test tile
buffer). See `../tests/rewind_regression.rs` for the RED/GREEN proof.
