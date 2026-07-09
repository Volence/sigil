# Vendored clownlzss source

- **Upstream**: https://github.com/Clownacy/clownlzss
- **Pinned revision**: `8055bd2e2d258dab604ea26ded9e42144562db54`
- **Date vendored**: 2026-07-09

These files are verbatim copies from the upstream repository (not the whole
repo — just the optimal-parse core plus the six format headers this crate
exposes: kosinski, kosinskiplus, saxman, enigma, comper, rocket). They are
compiled/included unmodified by `build.rs` and the crate's shim. Do not
hand-edit them here — any fix belongs upstream (or, if truly Sigil-specific,
should be re-derived and re-vendored with a note explaining the divergence).

## Vendored files and their upstream paths

| Vendored path | Upstream path |
|---|---|
| `bitfield.h` | `bitfield.h` |
| `common.h` | `common.h` |
| `LICENCE.txt` | `LICENCE.txt` |
| `compressors/clownlzss.c` | `compressors/clownlzss.c` |
| `compressors/clownlzss.h` | `compressors/clownlzss.h` |
| `compressors/common.h` | `compressors/common.h` |
| `compressors/kosinski.h` | `compressors/kosinski.h` |
| `compressors/kosinskiplus.h` | `compressors/kosinskiplus.h` |
| `compressors/saxman.h` | `compressors/saxman.h` |
| `compressors/enigma.h` | `compressors/enigma.h` |
| `compressors/comper.h` | `compressors/comper.h` |
| `compressors/rocket.h` | `compressors/rocket.h` |
| `decompressors/common.h` | `decompressors/common.h` |
| `decompressors/kosinski.h` | `decompressors/kosinski.h` |
| `decompressors/kosinskiplus.h` | `decompressors/kosinskiplus.h` |
| `decompressors/saxman.h` | `decompressors/saxman.h` |
| `decompressors/enigma.h` | `decompressors/enigma.h` |
| `decompressors/comper.h` | `decompressors/comper.h` |
| `decompressors/rocket.h` | `decompressors/rocket.h` |

Formats NOT vendored (present upstream but out of scope for this crate):
`chameleon`, `faxman`, `gba`, `rage`.

## License

Verbatim ISC-style permission text by Clownacy (no SPDX tag upstream, but the
wording is the standard ISC license). See `LICENCE.txt`. The crate's
`Cargo.toml` `license` field is set to `"ISC"` to reflect this — it is NOT
0BSD (an earlier research pass mislabeled it; corrected per the frozen plan's
CR2 ruling).

## The shim is NOT vendored

`../src/shim.cpp` (outside this `vendor/` directory) is hand-written
Sigil-original code. The optimal-parse core (`compressors/clownlzss.c`) is
plain C89, but every per-format compressor/decompressor here is a C++20
template header with no C linkage entry point — the shim instantiates each
template against a raw `unsigned char*` (random-access iterator) and exposes
one `extern "C"` function per format so the safe Rust wrapper in `src/lib.rs`
can call it without a C++ ABI dependency.
