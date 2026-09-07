# span: location() indexes line starts instead of walking the file (sig-diag-location), 2026-09-07

Parcel `parcel/span-location-index`, branched from master `0a07b4ac`. Fix commit `f3cbf135`;
this note is the commit after it. The finding was a derived claim from a read-only review; the
measurements below outrank it and are what the fix was judged against.

## The finding, measured (committed code at 0a07b4ac)

`SourceMap::location` in `crates/sigil-span/src/lib.rs` walked `char_indices()` from byte 0 on
every call. A private probe (a scratch crate with a path dependency on the worktree's
`sigil-span`, deleted at the end of the parcel) built a synthetic source of 50-byte lines and
timed 5,229 lookups, release build:

| source | where | before, per lookup | before, 5,229 lookups | uptime load at the run |
|---|---|---|---|---|
| 1,600,000 B (32,000 lines) | last byte | 773 us | 4.04 s | 4.10, 2.96, 3.16 (11:53:05) |
| 1,600,000 B | one per line over the last 5,229 lines | 709 us | 3.71 s | same run |
| 1,600,000 B | byte 0 | 0.001 us | 6.4 us | same run |
| 2,709,750 B (54,195 lines, the size of `s2disasm/s2.asm`) | last byte | 1,262 us | 6.60 s | same run |
| 2,709,750 B | one per line over the last 5,229 lines | 1,205 us | 6.30 s | same run |

The brief's derived figures (18 us at the top, 0.45 ms at the bottom of 1.6 MB) had the right
shape and the wrong magnitude: the bottom is 773 us here and the top is nanoseconds, because the
walk breaks on the first character when `span.start` is 0.

On the real corpus the walk was half the run. `s2disasm` at `e45ebf33` (dirty 0), root `s2.asm`,
tree UNPREPARED (39 generated includes absent, which is the state every count in this repository
was measured in; see `scripts/corpus-baseline.sh`), 5,240 diagnostics on stderr, three direct
runs each (`cd s2disasm && sigil s2.asm`, the invocation `corpus-baseline.sh` makes):

| binary | md5 | wall (3 runs) | user |
|---|---|---|---|
| before (0a07b4ac) | `bdcbc8f53f6b797cb21120cdbc6f1d08` | 6.00 s, 6.62 s, 6.00 s | 5.87, 6.35, 5.93 s |
| after (f3cbf135) | `7f650e7495cedf0f9fec408c6878b4bc` | 3.16 s, 3.35 s, 3.47 s | 3.11, 3.21, 3.42 s |

uptime at those six runs: 2.63, 2.96, 3.12 (11:58:58) to 3.00, 3.01, 3.13 (11:59:27).

## The fix

Each source in the `SourceMap` carries a `OnceLock<Vec<u32>>` of line-start byte offsets, built on
the first `location` call against that source (a source that never locates a span never pays).
The line is a `partition_point` over that index; the column counts UTF-8 lead bytes (any byte that
is not a continuation byte, `b & 0xC0 != 0x80`) from the line start up to `span.start`, clamped to
the text length. `OnceLock` keeps `SourceMap` `Send + Sync`, which it was before; the map crosses
threads in four test files that `thread::spawn` an assembly.

After, same probe, same build, uptime 2.63, 3.11, 3.19 (11:56:14):

| source | where | after, per lookup | after, 5,229 lookups | first call (builds the index) |
|---|---|---|---|---|
| 1,600,000 B | last byte | 0.035 us | 181 us | 529 us |
| 1,600,000 B | last 5,229 lines | 0.011 us | 57 us | |
| 1,600,000 B | byte 0 | 0.011 us | 57 us | |
| 2,709,750 B | last byte | 0.036 us | 186 us | 963 us |
| 2,709,750 B | last 5,229 lines | 0.013 us | 65 us | |

The bottom lookup is 22,000x cheaper; the top lookup is 10x dearer (1 ns to 11 ns: an atomic
load plus a 15-step binary search) and that is the whole regression. Every timed block's checksum
over its (line, col) results was identical before and after (6 blocks, 31,374 lookups).

### Index memory

One `u32` per line plus one. Largest files that reach a `SourceMap` here:

| file | bytes | lines | index |
|---|---|---|---|
| `s2disasm/s2.asm` (the corpus this parcel measured) | 2,709,725 | 91,276 | 365,108 B |
| `skdisasm/s3.asm` | 2,835,820 | 119,606 | 478,428 B |
| `skdisasm/sonic3k.asm` (largest source in the workspace) | 4,732,602 | 203,592 | 814,372 B |

No 1.6 MB corpus file exists in `s1disasm`, `s2disasm`, `skdisasm`, `aeon` or `sonic_hack` at
depth 3; the brief's "1.6 MB" is unlocated and was measured here as a synthetic size beside the
real 2.7 MB one.

## Column semantics: characters, kept

Columns were CHARACTERS before (the walk stepped `char_indices`), not bytes, and they still are.
The probe rendered `"ab\n\u{e9}\u{4e2d}x\u{1F600}y\nz"` (16 bytes, 10 chars) at every start
offset 0 through 18, plus an empty source at offsets 0 and 5 and `"x\n"` at 0 through 3, before
and after: 26 lines, byte-identical. Three facts that a naive rewrite would change and that the
new test pins:

- an offset INSIDE a multi-byte character counts that character (byte 4, inside the 2-byte
  `é` at 3..5, is column 2, the same as byte 5);
- an offset past the end of the text locates as the end (16, 17, 18 all render `3:2`);
- a trailing newline opens an empty last line (`"x\n"` at 2 and at 3 is `(2, 1)`).

## Identity on the corpus

`scripts/corpus-baseline.sh --unprepared-ok --compare` and a raw `cmp`, before binary against
after binary, `s2disasm` at `e45ebf33`:

| | before | after |
|---|---|---|
| stderr lines | 5,240 | 5,240 |
| stderr md5 | `39fa682c5f3b5a400d26b4ed8a6bae98` | `39fa682c5f3b5a400d26b4ed8a6bae98` |
| lines only in one side | 0 | 0 |

`diff` input was 10,480 lines (5,240 each side) and produced 0 lines; `cmp` reports identical.
The three direct timing runs per binary wrote the same 5,240-line, same-md5 stderr each time.

## Tests

All with `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-span`, dev profile, at f3cbf135:

| suite | passed | failed | ignored | result lines | uptime after |
|---|---|---|---|---|---|
| `-p sigil-span` | 6 | 0 | 0 | 2 | 3.07, 3.22, 3.23 (11:55:54) |
| `-p sigil-frontend-as` (`SIGIL_ALLOW_PARTIAL=1`) | 625 | 0 | 0 | 52 | 4.68, 3.46, 3.28 (11:59:59) |
| `-p sigil-frontend-emp` (`SIGIL_ALLOW_PARTIAL=1`) | 2,643 | 0 | 0 | 131 | 10.12, 4.77, 3.71 (12:00:16) |
| `-p sigil-harness` (`SIGIL_ALLOW_PARTIAL=1`, extra) | 428 | 0 | 1 | 48 | 8.66, 5.06, 3.86 (12:01:05) |

No failing names. Neither frontend crate consults `SIGIL_ALLOW_PARTIAL` (the variable is read by
`sigil-harness/src/test_support.rs`), so no reference row was skipped in those two suites: 0
ignored in each. `cargo check --workspace --all-targets` is clean; `cargo clippy -p sigil-span
--all-targets -- -D warnings` is clean. The repository carries no `rustfmt.toml` and the file was
not rustfmt-shaped before this parcel (`cargo fmt --check` rewrites `merge` and `name`, untouched
lines), so it was not run.

### Red-first

Gate: `tests::location_at_the_bottom_costs_no_more_than_at_the_top` in
`crates/sigil-span/src/lib.rs`. It asserts 5,229 last-byte lookups on a 1.6 MB source finish in
under 2 s and cost under 1000x the same number of byte-0 lookups (the ratio is load-independent;
both sides run under the same load).

Mutation: the pre-fix `location` body (the `char_indices` walk) written over the new one in the
working tree, `git diff --stat` reading `1 file changed, 14 insertions(+), 9 deletions(-)` and
`grep -n char_indices` finding it at line 118. Runner: `cargo test -p sigil-span`, dev profile,
uptime 2.27, 2.97, 3.14 (11:56:44).

```
test tests::location_at_the_bottom_costs_no_more_than_at_the_top ... FAILED
panicked at crates/sigil-span/src/lib.rs:270:9:
5229 bottom lookups took 75.704210108s; the lookup is walking the file
test result: FAILED. 5 passed; 1 failed; 0 ignored
```

Restored with `git checkout -- crates/sigil-span/src/lib.rs` (tree clean at f3cbf135), re-run:
6 passed, 0 failed, 0.02 s. The other new test,
`location_columns_count_characters_not_bytes`, passed under the mutation too: it is an identity
guard on the semantics, not a gate on the mechanism, and is recorded as such.

## Open

- The top-of-file lookup went from 1 ns to 11 ns. Not worth a fast path; recorded so no one
  measures it later and calls it a regression.
- `docs`-level doc comments elsewhere in `lib.rs` still carry em dashes from before this parcel;
  only the lines this parcel added were held to the no-dash rule. Scrubbing the rest is a separate
  byte-neutral touch.
- The `s2disasm` tree is unprepared (39 generated includes absent). Identity was measured on the
  same unprepared tree both sides, which is valid for identity and not a baseline count.
- The harness suite run here (428 tests) is `-p sigil-harness` only; the large golden suite lives
  under `sigil-cli` and was not in the brief's scope.
