# SIGIL-AS-REPLACEMENT: where "sigil can build Sonic 1 and Sonic 2" stands, 2026-09-27

Measurement and proposal parcel (`parcel/as-replacement-state-r2`). It proposes; nothing lands
on it. Every figure below was re-measured in this run against a sigil built from this worktree,
unless a line says otherwise. A first run of the same brief died uncommitted; its claims were
treated as hypotheses and each one is marked below as reproduced or not.

## Instruments

| what | identity |
|---|---|
| sigil | built here, `CARGO_TARGET_DIR=<worktree>/target cargo build --release`, source `398e5b15`, binary md5 `dffd101f6707c2d08a6fe2bf25f5f42e` |
| reference asl (S1, S3K) | md5 `61e672562465725a8c102288a7da9098` (the copy each of `s1disasm` and `skdisasm` ships) |
| S2's own asl | md5 `0dee1f98e6480a4783d27ffd8b90896f`, the build OVERSEER-REFERENCE refuses as an oracle. It is the only asl that builds Sonic 2 (see below), so it is used ONLY as the S2 whole-ROM reference, never as a source of per-line values |
| p2bin | md5 `4f2fff99c3347bafb93b12d5be1db754` in all three repos |
| s1disasm | `f6ece657c1cf253404312137dfcb8ec15fa42318` (2026-08-01) |
| s2disasm | `e45ebf332f39987424ca3102e50c717628f71269` (2025-08-27) |
| skdisasm | `2fcd861c208f342b6d14df694c6422c74f20a4be` (2025-08-30) |

Each corpus was extracted with `git archive <sha>` into scratch inside this worktree (never its
working tree: `s1disasm` has two modified `.nem` files and `skdisasm` has untracked save files,
and neither can colour a result read from a commit). No disassembly repo was written to.
Checksums are zlib CRC-32 `crc/bytes`; identity claims are `cmp` over the whole file, each with a
planted-byte control that `cmp -l` saw as exactly 1 differing byte.

## 1. What builds, byte-identical, by which entry point (stock settings)

Measured 2026-09-27T09:19Z to 09:19:54Z, load average 6.7 to 10.0 (see section 5 for why timing
under this load is only indicative).

| corpus | stock build script, stock tools | sigil, direct one command | sigil dropped in as `asl` | sigil behind a two-file shim |
|---|---|---|---|---|
| S1 `build.lua` | exit 0, `afe05eee/524288` | IDENTICAL | FAILS, no ROM | IDENTICAL |
| S2 `build.lua` | exit 0, `7b905383/1048576` (S2's asl) | IDENTICAL | FAILS, no ROM | IDENTICAL (28 assembler calls: 27 songs + ROM) |
| S3K `buildSK.lua` | exit 0, `0658f691/2097152` | IDENTICAL | FAILS, no ROM | IDENTICAL |
| S3 Complete `buildS3Complete.lua` | exit 0, `a651623a/3360160` | IDENTICAL | FAILS, no ROM | IDENTICAL |
| S3 alone `buildS3.lua` | exit 0, `9bc192ce/2097152` | IDENTICAL | FAILS, no ROM | IDENTICAL |

The direct commands, each run in a copy of the stock tree after the stock script had produced
its generated inputs (PCM/DPCM, S2's compressed songs):

```text
sigil sonic.asm   -o sigil.bin -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after
sigil s2.asm      -o sigil.bin -p=0  -z=0,saxman-bugged,Size_of_Snd_driver_guess,after
sigil sonic3k.asm -o sigil.bin -D Sonic3_Complete=0 -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before
sigil sonic3k.asm -o sigil.bin -D Sonic3_Complete=1 (same -p/-z)
sigil s3.asm      -o sigil.bin -p=FF -z=0,uncompressed,Size_of_Snd_driver_guess,before -z=1300,uncompressed,Size_of_Snd_driver2_guess,before
```

The first run's `logs-direct.txt`, `logs-refs.txt`, `logs-dropin.txt` and `logs-shim.txt` all
REPRODUCE, same CRCs, on a sigil four commits newer (the difference is a queue doc).

**S2 cannot be built by the reference asl at all.** With md5 `61e67256` and its own message
catalogs in S2's tool slot, `build.lua` exits 1 on `error #1010: symbol undefined` inside the
`jsrto`/`jmpTos` macros (`s2.asm(13535)` first). This is already known to the notes ("the pinned
build cannot assemble Sonic 2", `2026-09-11-s1-driver-stage2.md`). Copying the pinned binary
alone into S2's slot fails earlier, on S2's own `.msg` catalogs ("as.msg has invalid format"),
which is what this run's first `s2-refpin` leg hit; the first run's `s2-refpin` also exited 1.
For S2, sigil's byte-identity is therefore against the refused build, which is S2's actual
toolchain; its ROM reproduced at the same CRC in both runs.

(sections 2 onward follow)
