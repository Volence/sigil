# Vendored salvador source

This directory is a verbatim copy of the C sources from `aeon/tools/salvador/src/`
(salvador v1.4.2 by Emmanuel Marty — https://github.com/emmanuel-marty/salvador),
including the bundled `libdivsufsort` library by Yuta Mori
(https://github.com/y-256/libdivsufsort).

These files are compiled unmodified by `build.rs` in this crate. Do not hand-edit
them here — any fix belongs upstream (or, if truly Sigil-specific, should be
re-derived and re-vendored with a note explaining the divergence).

Licenses are included alongside: `LICENSE`, `LICENSE.zlib.md`, `LICENSE.cc0.md`
(salvador) and `libdivsufsort/LICENSE` (libdivsufsort, MIT).

`salvador.c` (the CLI driver, with `main`) is present for reference but is NOT
compiled by `build.rs` — only the library entry points (`shrink.c`,
`matchfinder.c`, `expand.c`) and the divsufsort sources are built.
