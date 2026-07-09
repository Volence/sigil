# Test vector provenance (classic-format `.emp` builtins, T2b)

All files here are byte-identical copies of the SAME committed vectors T2a
(`sigil-clownlzss-sys` / `sigil-clownnemesis-sys`) already gates on — copied
in rather than referenced by a `../../..` relative path so this crate's
tests do not reach across crate boundaries on disk. See those crates'
`tests/vectors/PROVENANCE.md` for the original upstream provenance
(s2disasm/skdisasm source paths, decompression cross-checks, etc.) — not
repeated here.

| File | Copied from |
|---|---|
| `level_select_2p.raw` | `sigil-clownlzss-sys/tests/vectors/level_select_2p.raw` |
| `sand_particles.raw` | `sigil-clownlzss-sys/tests/vectors/sand_particles.raw` |
| `golden_kosinski.bin` | `sigil-clownlzss-sys/tests/vectors/golden_kosinski.bin` |
| `golden_kosplus.bin` | `sigil-clownlzss-sys/tests/vectors/golden_kosplus.bin` |
| `golden_saxman_header.bin` | `sigil-clownlzss-sys/tests/vectors/golden_saxman_header.bin` |
| `golden_saxman_noheader.bin` | `sigil-clownlzss-sys/tests/vectors/golden_saxman_noheader.bin` |
| `golden_enigma.bin` | `sigil-clownlzss-sys/tests/vectors/golden_enigma.bin` |
| `golden_comper.bin` | `sigil-clownlzss-sys/tests/vectors/golden_comper.bin` |
| `golden_rocket.bin` | `sigil-clownlzss-sys/tests/vectors/golden_rocket.bin` |
| `numbers.raw` | `sigil-clownnemesis-sys/tests/vectors/numbers.raw` |
| `golden_nemesis.bin` | `sigil-clownnemesis-sys/tests/vectors/golden_nemesis.bin` |

`kosinski_m`/`kosplus_m` (moduled) have no committed golden `.bin` in T2a
(only a real-blob round-trip pair, `sand_particles.raw`/`.kosm`, at module
size `$1000`). Their e2e tests instead compute the expected bytes by
calling `sigil_clownlzss_sys::compress_kosinski_moduled`/
`compress_kosplus_moduled` directly in the test body (same pattern as
`tests/sandbox_zx0.rs`'s `zx0_wraps_and_compresses`, which computes its
expected bytes via `sigil_salvador_sys::compress` rather than a static
file) — this is still byte-exact against the T2a-gated function, just
computed inline instead of pre-captured to a file.
