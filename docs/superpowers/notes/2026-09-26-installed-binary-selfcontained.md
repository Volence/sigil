# INSTALLED-BINARY-READS-BUILD-TREE: the installed pair no longer needs its build tree

2026-09-26. Branch `parcel/installed-binary-selfcontained`, base master `c024aba8`. Aeon reference tree
`/home/volence/sonic_hacks/.aeon-ref-selfcontained` at the provenance tip's `aeon_rev` `ec640bcf`, provisioned by
`scripts/provision-aeon-ref.sh`, witnessed by `repin --check` printing `pins.rs unchanged`.

## 1. What the installed binaries read from the tree they were compiled in

Two instruments, each run on binaries built here in a throwaway worktree (`git worktree add --detach`, own
`CARGO_TARGET_DIR`), not on the shared pair.

**Binary strings.** `strings`/`grep -ao` for the throwaway tree's path in `sigil` and `emit_sound_blob` built from
master `c024aba8`. Rust registry paths (`~/.cargo/registry/...`) also appear; they are panic-location text for
third-party crates and name no file anything opens.

| hit in the binary | source | binary | class |
|---|---|---|---|
| `<tree>` + `crates/sigil-harness` + `golden/offcanonical_sizes` | `native.rs` `load_frozen_table` | sigil | **run-time read on the build path: fixed here** |
| `<tree>` in the banner's `source:` and `drift-check` lines | `sigil-cli/build.rs` `SIGIL_SOURCE_DIR` etc. | sigil | informational, printed only |
| `<tree>/crates/sigil-harness` beside `EMPYREAN_SUITE_ROOT` | `test_support.rs` `derived_suite_root` | both | not a dependency (below) |
| `<tree>/crates/sigil-harness` beside path-prefix text | `source_digest.rs` `sigil_source_root` | sigil | not a read (below) |

At the parcel tip `d2171ad8` the first row is gone from `sigil`; the other three remain in the same places.

**Source grep.** `CARGO_MANIFEST_DIR`, `env!(` and `concat!(env!` across `crates/`, outside `tests/`:

| site | reachable from `sigil build` / `emit_sound_blob`? | class |
|---|---|---|
| `sigil-harness/src/native.rs` `load_frozen_table` | yes, every profile constructor, every shape | **run-time read: fixed** |
| `sigil-harness/src/source_digest.rs` `sigil_source_root` | yes (digest rendering) | computes a path to place reads under `root=sigil`; `canonical()` falls back to the absolute path when the directory is gone; opens nothing |
| `sigil-harness/src/test_support.rs` `derived_suite_root` | only via `seam2::require_named_reference_tree` when `AEON_DIR` is unset, which both binaries set from `--aeon` before emitting | builds the text of a refusal; with the tree gone the derivation errs and the refusal says so. Not a dependency |
| `sigil-harness/src/harness_root.rs` `BUILT_FROM` | no (refreeze/repin announce only) | informational, printed |
| `sigil-harness/src/reference_dependence.rs` `workspace_root` | no (suite census) | test tooling |
| `sigil-harness/src/bin/derive_offcanon.rs` | separate tool, run by `derive_offcanonical_sizes.sh` right after it builds it | tooling; default output dir and golden blobs |
| `sigil-isa/src/bin/gen_*_vectors.rs`, `sigil-frontend-as/src/bin/gen_snippet_vectors.rs` | separate tools | tooling |
| `sigil-frontend-emp/src/eval/sandbox.rs:717`, `sigil-cli/src/main.rs:3675` | `#[cfg(test)]` | test-only |
| `sigil-cli/src/main.rs` `env!("SIGIL_*")`, `CARGO_PKG_VERSION` | yes | build-time constants for `--version` and the digest's `DIGEST-ASSEMBLER`; informational |
| `sigil-cli/build.rs` `std::env::var("CARGO_MANIFEST_DIR")` | build time only | the build script |

**Measured, not inferred: `emit_sound_blob` never depended on the tree.** Master's `emit_sound_blob`, run with its
build tree removed, emits all 19 artifacts, byte-identical to the parcel's (`diff -r`). Only `sigil` broke on
2026-09-26.

## 2. The fix, and the rejected option

**Chosen: compile the tables in.** `native::FROZEN_TABLES` holds the seven committed
`golden/offcanonical_sizes/*.txt` as `include_str!` rows; `load_frozen_table` parses the row by name and panics,
naming the rows it carries, on an unknown name (it panicked on a missing file before).

**Rejected: resolve the tables from a named reference location at run time** (an environment variable, or a path
beside the binary). It moves the dependency rather than removing it: every consumer that runs the binary (aeon's
`build.sh`, the nightly, every agent) would have to supply and keep that location in step with the binary, and a
wrong location reads a table that belongs to another revision while the binary's own identity says nothing about
it. The tables are data the assembler's placement code was written against; they version with it.

## 3. What consumed the run-time read, and what each consumer does now

**The read-set recorder and the `.lst` source digest.** `load_frozen_table` read through
`sigil_span::read_set::read_to_string`, so every build's digest carried one `DIGEST-READ ... origin=external
root=sigil path=crates/sigil-harness/golden/offcanonical_sizes/<shape>.txt` row: the only `root=sigil` row a
build wrote. That row is gone (aeon `s4.lst` after the parcel-arm build: 0 `root=sigil` rows, `reads=325`). The
table is now part of the assembler, covered by `DIGEST-ASSEMBLER revision=` like the rest of its code. The kernel
witness in `lst_source_digest.rs` (opens equal rows, both directions) stays consistent because the read and the row
left together. The `root=sigil` grammar stays, so an older listing still parses.

**Aeon's freshness verdict** (`tools/artifact_provenance.py` at aeon `origin/master` `d3b98868`) resolved
`root=sigil` rows against the `source:` line of `$SIGIL_BUILD --version` and opened the file there: the same build
tree, from aeon's side. With no such row it opens nothing in the sigil tree. It still requires the `source:` line to
be printed (it raises without one), and the banner still prints it. Nothing in aeon requires a `root=sigil` row to
exist (searched: `tools/`, `*.py`, `*.sh`).

**The closure derivation behind `--version`.** `sigil-cli/build.rs` narrowed each package to its targets' source
directories, so a table embedded from `golden/` would have been a compile input the closure called harmless: the
existing escape gate `no_compiled_source_reaches_a_file_outside_the_closure` goes red on exactly that (shown in 5).
`build.rs` now follows every path-attribute and include-macro reference in a compiled source to its file, and the
test's `cargo_closure` mirror does the same with the escape gate's own scanner. The closure grows from 29 to 38 paths:
the seven tables, plus `crates/sigil-isa/tests/corpus/mod.rs` and `tests/corpus_m68k/mod.rs`, which the
`gen_*_vectors` bins in `sigil-isa/src/bin/` name by path attribute (a second binary, the documented safe-side
over-approximation). The tables are therefore rerun triggers of the tree word, and an edit to one moves the closure
revision.

**Refreeze and repin, which regenerate the tables.** A table edit now takes effect on the next compile. cargo tracks
`include_str!` files, so every `cargo build`/`run`/`test` after a table changes recompiles `sigil-harness`.
`refreeze --freeze` runs (1) `capture_goldens.sh` with the prebuilt `SIGIL_BUILD`, before any table changes, (2)
`derive_offcanonical_sizes.sh`, which builds `derive_offcanon` immediately before running it and constructs every
profile before writing any table, then (3) `repin`, through `cargo run` by default, which recompiles against the
fresh tables. One path relied on the run-time read picking up a freshly written table: `REPIN_BIN` (a prebuilt
`repin`) in step 3. It read the new tables only when it was built in the same tree; it now always carries the tables
it was compiled with. Nothing in the repo sets `REPIN_BIN` (searched), and `native_offcanonical_placement`'s
`*_size_table_rederives_native` gates re-derive against the committed files in a freshly compiled test binary.

## 4. The falsifier: build tree removed

Both arms: `git worktree add --detach` under the scratch dir, `cargo build --release -p sigil-cli --bin sigil -p
sigil-harness --bin emit_sound_blob` into its own `CARGO_TARGET_DIR`, binaries copied out, `git worktree remove`,
then aeon `./build.sh` per shape with `SIGIL_BUILD`/`SIGIL_EMIT` pointing at the copies. CRC-32 is zlib's.

**Control, master `c024aba8`** (banner `source:` names the removed tree): every shape fails, no ROM written.

| shape | exit | first error |
|---|---|---|
| s4 | 1 | `panicked at crates/sigil-harness/src/native.rs:250:29: read frozen table <removed tree>/crates/sigil-harness/golden/offcanonical_sizes/s4.txt: No such file or directory (os error 2)` |
| s4 DEBUG=1 | 1 | same |
| demo | 1 | same (the expect-fail sentinel stage builds the sonic4 profile first) |
| demo DEBUG=1 | 1 | same |

**Parcel, tip `d2171ad8`** (banner `source:` names the removed tree), against the provenance tip
`link-zero-byte-move-placement`:

| shape | ROM | built | tip |
|---|---|---|---|
| s4 | `s4.bin` | `91c46c94/820209` | `91c46c94/820209` |
| s4 DEBUG=1 | `s4.debug.bin` | `8a378de6/846509` | `8a378de6/846509` |
| demo | `demo.bin` | `1c7a34d3/96863` | `1c7a34d3/96863` |
| demo DEBUG=1 | `demo.debug.bin` | `72e405a5/103185` | `72e405a5/103185` |

All four `build.sh` runs exited 0. Byte-neutral: nothing under `golden/`, `pins.rs` or `repin.toml` changed.

## 5. Gates, each red first

| gate | what it holds | red-first mutation, on disk | red output |
|---|---|---|---|
| `sigil-cli/tests/installed_binary_needs_no_build_tree.rs` | the real `sigil`, under `bwrap` with the compiling checkout replaced by an empty tmpfs, builds all four shapes byte-identical to an unsandboxed build, with no `root=sigil` digest row | `load_frozen_table` reverted to the run-time read | `s4: sigil build failed (exit status: 101) ... read frozen table <tree>/.../s4.txt: No such file or directory` |
| same, hiding control | the sandbox really hides the checkout | `--tmpfs` pointed at `/mnt` instead of the checkout | `the sandbox still shows <tree>/crates/sigil-harness/golden/provenance.toml` |
| `sigil-harness/tests/installed_binary_needs_no_build_tree.rs` | the real `emit_sound_blob`, same sandbox, writes the same file set and bytes | a build-tree read added at the top of `seam1::emit_sound_blob_in` | `hidden: emit_sound_blob failed (exit status: 1) ... read frozen table: No such file or directory` |
| `sigil-harness/tests/frozen_tables_embedded.rs` | `FROZEN_TABLES` equals the directory, each row its own file | (a) `s4.txt` row embedding `s4_debug.txt`; (b) `lean.txt` row deleted | (a) `the FROZEN_TABLES row s4.txt does not carry .../s4.txt`; (b) `FROZEN_TABLES and ... disagree about which tables exist` |
| `version_provenance` (existing, extended mirror) | the closure covers every compile input | `build.rs` drops the followed references | `no_compiled_source_reaches_a_file_outside_the_closure` and `the_closure_is_cargos_current_answer_not_a_baked_list` FAILED, seven `reaches crates/sigil-harness/golden/offcanonical_sizes/<t>.txt` lines |

Each mutation was restored with `git show HEAD:<path> > <path>` from the committed tip, `git status` clean after.

The two sandbox gates are unmeasurable, loudly, without `bwrap` or with `AEON_DIR` inside the checkout; with no
reference tree they follow the house pattern (skip, or fail under `SIGIL_STRICT_GATE=1`). A target directory inside
the checkout is bound back into the sandbox (probed by hand: a binary and an output directory re-bound under a
hidden directory run and write).

## 6. The standing rule "lock the build tree at swap time"

It is unnecessary for any pair built from this commit or later: such a `sigil` reads nothing from its build tree
and writes no digest row that makes aeon read there. It still holds for the pair installed today (built at
`4ce2509d`, before this change), whose tree `.worktrees/land-4ce2509d` must stay until a pair built from this
change is swapped in. The unlock is the overseer's.
