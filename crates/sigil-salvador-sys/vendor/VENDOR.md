# Vendored salvador source

- **Upstream project**: https://github.com/emmanuel-marty/salvador
- **Upstream version string**: `1.4.2` (`TOOL_VERSION` in `salvador.c`)
- **Pinned revision**: ⚠️ **NOT RECORDED — see "The gap" below.**
- **Immediate source**: `aeon/tools/salvador/src/` (a sibling working tree, added
  to aeon 2026-06-11 in `6f870d81`), not a fresh upstream checkout
- **Date vendored into sigil**: 2026-07-05 (`a7bee61c`)
- **Bundled dependency**: `libdivsufsort` by Yuta Mori
  (https://github.com/y-256/libdivsufsort), MIT — vendored as part of the same copy

These files are compiled unmodified by this crate's `build.rs`. Do not hand-edit
them here — any fix belongs upstream (or, if truly Sigil-specific, should be
re-derived and re-vendored with a note explaining the divergence).

## The gap, stated plainly

The other two vendored C dependencies (`sigil-clownlzss-sys`,
`sigil-clownnemesis-sys`) pin an exact upstream commit hash and vendoring date.
This one does not, because it was copied from a sibling working tree rather than
from upstream, and no revision was recorded at the time. **The hash is not
knowable from anything in either repository, so none is written here** — a
plausible-looking hash would be worse than an admitted gap.

This is the workspace's weakest reproducibility link, and it matters more than it
looks: every whole-ROM golden argument rests on the build being reproducible, and
this compressor's output is baked into the shipped ROM (lens sweep, seat ARCH,
finding S8).

**To close it** (needs network access, which is why it is not done here): fetch
upstream, find the commit whose `src/` matches the table below byte-for-byte,
record it as `Pinned revision`, and delete this section.

## What IS verified

The vendored copies are byte-identical to their `aeon/tools/salvador/src/`
counterparts as of 2026-08-14 — checked file by file, not assumed:

| Vendored path | aeon path | Compiled by `build.rs` |
|---|---|---|
| `shrink.c` | `tools/salvador/src/shrink.c` | yes |
| `matchfinder.c` | `tools/salvador/src/matchfinder.c` | yes |
| `expand.c` | `tools/salvador/src/expand.c` | yes |
| `shrink.h` | `tools/salvador/src/shrink.h` | header |
| `matchfinder.h` | `tools/salvador/src/matchfinder.h` | header |
| `expand.h` | `tools/salvador/src/expand.h` | header |
| `format.h` | `tools/salvador/src/format.h` | header |
| `libsalvador.h` | `tools/salvador/src/libsalvador.h` | header |
| `salvador.c` | `tools/salvador/src/salvador.c` | **no** — the CLI driver (has `main`), kept for reference only |
| `libdivsufsort/lib/*.c` | bundled with the above | yes |
| `libdivsufsort/include/*.h` | bundled with the above | headers |

## Licenses

`LICENSE`, `LICENSE.zlib.md`, `LICENSE.cc0.md` (salvador) and
`libdivsufsort/LICENSE` (libdivsufsort, MIT) are included alongside the sources.
