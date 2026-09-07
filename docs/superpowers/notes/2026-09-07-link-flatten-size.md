# link: the flat image is sized by the cartridge window (sig-flatten-lma-alloc), 2026-09-07

Parcel `parcel/link-flatten-size`, branched from master `435e1148`. Tip at the time of writing:
`63349e86` (three commits, listed at the end). The finding was a derived claim from a read-only
review; the reproduction below outranks it and is what the fix was measured against.

## Reproduction (committed code at 435e1148, release build)

Inputs, each a three-line `.asm` (the `cpu` line is required by the AS surface):

```
    cpu 68000            cpu 68000            cpu 68000            cpu 68000
    org -1               org $FF0000          org $400000          org $3FFFFF
    dc.b 1               dc.b 1               dc.b 1               dc.b 1
```

Before:

| input | invocation | behaviour | exit |
|---|---|---|---|
| `org -1` | no `-o`, `ulimit -v` 3 GiB | `memory allocation of 4294967296 bytes failed`, SIGABRT | 134 |
| `org -1` | `-o /dev/null`, uncapped | a lazy 4 GiB calloc, no diagnostic | 0 |
| `org -1` | `--hex`, `ulimit -v` 8 GiB | `memory allocation of 103079215104 bytes failed` (96 GiB), SIGABRT | 134 |
| `org $FF0000` | `-o` | a 16 MiB image with a work-RAM byte at its end, no diagnostic | 0 |
| `org $400000` | none | a 4 MiB + 1 image, no diagnostic | 0 |
| `org $3FFFFF` | `--hex` | a 4 MiB image ending in `01` (correct, the control) | 0 |

Mechanism, confirmed at `crates/sigil-link/src/lib.rs` `flatten`: the buffer length was
`max(lma + len)` over the byte-emitting sections, i.e. taken from an ADDRESS. `org -1` sets the
physical counter to `0xFFFFFFFF` (`directive_org` in the AS front end jumps `phys_base`), the
next `dc.b` opens a one-byte section there, and `vec![fill; 0x1_0000_0000]` follows. `--hex`
then collects one `String` per byte. `catch_unwind` appears nowhere in the code; the review's
remark about it was hypothetical, and an allocator abort is not a panic in any case.

## The fix (f28aaa33)

One named source for the windows, three layers, the `.asm` route's `resolve_layout` seam
untouched:

- `crates/sigil-ir/src/map.rs`: `CARTRIDGE_LMA_BASE`/`CARTRIDGE_SIZE` (`0x000000`, 4 MiB) and
  `M68K_RAM_LMA_BASE`/`M68K_RAM_SIZE` (`0xFF0000`, 64 KiB); `MemoryMap::mega_drive()` builds
  regions `cartridge` (rom) and `work_ram` (m68k_ram) from them; `Region::contains`;
  `RegionKind::label` (the `map.toml` spellings). `validate_section` now says what an
  out-of-ROM LMA landed in: the non-ROM region holding it, else every ROM window.
- `crates/sigil-link/src/lib.rs`: `image_extent` validates every byte-emitting section against
  `MemoryMap::mega_drive()` and returns the end of the highest one, so the length is derived
  from validated extents and cannot exceed `CARTRIDGE_SIZE`; `flatten` is now
  `Result<Vec<u8>, String>` and allocates only after that; `flatten_checked` and `emit_rom`
  ride the same path. `check_image_bounds(image, resolved_sections)` is the located form:
  every refusal as a `Diagnostic` at the span of the section's first byte-emitting fragment
  (`Data` or `Fill`; `Reserve` skipped), matched by name and LMA, falling back to the
  unresolvable `SourceId(u32::MAX)` span so no refusal is dropped for want of a line.
- `crates/sigil-cli/src/main.rs`: the single-file `.asm` route calls `check_image_bounds`
  between `link` and `flatten` and renders through `render_located_diags` (`file(line):`);
  the `.emp` no-map tail `link_sections` does the same through `render_program_diags`
  (`path:line:col:`), with `link_to_image` also handing back the resolved sections. `link_rom`
  (the `--map` tail) is unchanged apart from sharing the new `unlocated_error` helper.

Test helpers: `sigil_link::flatten(&x, 0x00)` gains `.unwrap()` in 63 files (test files plus
`eval.rs`'s own test module) across sigil-cli, sigil-frontend-as, sigil-frontend-emp and
sigil-link. Byte-neutral: every golden compares the same bytes.

## Diagnostics after the fix (AS surface; exit 1 for each)

```
org_neg1.asm(3): error: section `sec4294967295` LMA 0xFFFFFFFF is in no ROM region; ROM regions: `cartridge` [0x0,0x400000)
org_ram.asm(3): error: section `sec16711680` LMA 0xFF0000 lies in non-ROM region `work_ram` (m68k_ram) [0xFF0000,0x1000000); its 1 byte(s) have no place in the image
org_past_rom.asm(3): error: section `sec4194304` LMA 0x400000 is in no ROM region; ROM regions: `cartridge` [0x0,0x400000)
org_straddle.asm(3): error: section `sec4194303` [0x3FFFFF,0x400001) overflows region `cartridge` (ends 0x400000), over by 1 bytes
```

The `--hex` form of `org -1` prints the first line, nothing on stdout, exit 1. `org $3FFFFF` +
`dc.b 1` still builds a `0x400000`-byte image ending in `01` (the bound is the window's end,
not one short of it). The `.emp` surface renders the same messages as `path:line:col: error:`.

## Red-first evidence

Gate: `crates/sigil-cli/tests/image_bounds.rs` (six tests), committed ALONE at `5247f1a6` on
the pre-fix linker and run there (release, `ulimit -v` 8 GiB, single thread):

```
test result: FAILED. 1 passed; 5 failed
  a_byte_in_work_ram_is_refused_by_region_name         expected exit 1, got ExitStatus(unix_wait_status(0))
  a_byte_just_past_the_cartridge_is_refused            expected exit 1, got ExitStatus(unix_wait_status(0))
  a_word_straddling_... (since renamed)                expected exit 1, got ExitStatus(unix_wait_status(0))
  org_minus_one_is_refused_not_a_four_gib_allocation   expected exit 1, got ExitStatus(unix_wait_status(0))
  org_minus_one_with_hex_is_refused_before_rendering   expected exit 1, got ExitStatus(unix_wait_status(134))
                                                       stderr: memory allocation of 103079215104 bytes failed
  the_last_cartridge_byte_is_still_accepted            ok   (the control passes on both sides)
```

Mutation shown applied, against the fix (`f28aaa33`, then `63349e86`): `window_refusal`
replaced by `let _ = (window, len); None` (the `git diff` of that one line recorded in the
run), rebuilt, run, restored with `git checkout -- crates/sigil-link/src/lib.rs`
(`git status` empty afterwards), rerun:

- CLI gate, mutated: `1 passed; 5 failed`, the same five failures with the same exit statuses
  and the same allocator line for `--hex`. Restored: `6 passed; 0 failed`.
- sigil-link unit tests, mutated (`63349e86`, `ulimit -v` 8 GiB): `0 passed; 4 failed`:
  `flatten_refuses_a_byte_outside_the_cartridge_window_by_name` "accepted at 4294967296
  bytes", `flatten_refuses_a_work_ram_byte_and_ignores_a_reserve_only_ram_section` "16711681
  bytes", `flatten_accepts_the_last_cartridge_byte_and_refuses_the_straddle` "4194305 bytes",
  `check_image_bounds_locates_each_refusal_at_its_emitting_fragment`. Restored: `123 passed`.

Two instrument defects found on the way, both fixed:

- The gate's straddle case was first `dc.w 1` at `$3FFFFF` expecting "over by 1". AS pads a
  word at an odd address, so the linker correctly reported `[0x3FFFFF,0x400002)`, over by 2.
  The case is now `dc.b 1,2` with the derived overshoot of 1
  (`two_bytes_straddling_the_cartridge_end_are_refused_with_the_overshoot`).
- The first mutation run of the unit tests (against `f28aaa33`) had to be killed at 21 GB
  RSS: `unwrap_err()` on a regression Debug-formats the accepted 4 GiB image into the panic
  message. `63349e86` replaces it with a `refused` helper that panics with the length. That
  killed run is NOT read as anything; the bounded rerun above is the evidence.

## Runs (all `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-linkfix`, `--release`)

| crate / runner | result |
|---|---|
| `cargo test -p sigil-ir` | 49 passed, 0 failed |
| `cargo test -p sigil-link` | 123 (unit) + 10 (`final_placement`) + 1 (`two_section_ab`) passed, 0 failed |
| `cargo test -p sigil-harness --lib`, bare | 197 passed, 1 failed: `test_support::tests::sec_field_equ_names_match_the_harvest`, the reference-tree bare-run refusal at `test_support.rs:1204` (reads `engine/structs.emp`) |
| same with `SIGIL_ALLOW_PARTIAL=1` | 198 passed, 0 failed (that row skips inside) |
| `SIGIL_ALLOW_PARTIAL=1 cargo test -p sigil-cli --no-fail-fast` over 25 targets: `image_bounds`, `cli_diagnostic_location`, `subcommands`, `end_to_end`, `symbolic_operands`, `jbra_relaxation`, `unsized_branch_relaxation`, `pcrel_port`, `ports`, `align_as_parity`, `m68k_cond_parity`, `here_relaxation_fix`, `tranche7_disp_splice_bytes`, `tranche2{0,1,2,3,4}_spelling_probes`, `native_offcanonical_placement`, `native_object_bank_budget`, `placement_fix`, `derived_layout`, `seam1_native_link`, `native_rom`, `native_full_rom` | 168 passed, 0 failed, 1 ignored (`sigil_diff_reports_byte_identity`, aeon tree) |
| `cargo test -p sigil-frontend-as` | 568 passed, 0 failed |
| `cargo test -p sigil-frontend-emp`, bare | 371 passed, 2 failed: `cfg_blind_spots::{corpus_computed_dispatch_census_is_six_sites_five_procs, no_out_declaring_proc_carries_a_targets_dispatch}`, the same bare-run refusal at `test_support.rs:1204` |
| `SIGIL_ALLOW_PARTIAL=1 ... --test cfg_blind_spots` | 12 passed, 0 failed |
| `cargo clippy --tests -p sigil-ir -p sigil-link -p sigil-cli` | no Rust warnings (only the pre-existing C++ `-Wmaybe-uninitialized` lines from `sigil-clownlzss-sys`) |
| `cargo test --workspace --no-run` | compiles |

Unmeasured here, by the brief: every reference-dependent row (the harness harvest row, the two
emp census rows, the ignored `sigil diff`, and the sigil-cli targets not in the list above that
need an aeon tree). Byte identity on the engine's four ROM shapes is the controller's landing
gate against the reference tree; nothing in this parcel changes a byte of an image that was
inside the window, and the goldens above that compare bytes all pass.

## Commits (`git log --oneline master..HEAD` on the branch)

```
63349e86 link tests: report an accepted image by length, never by formatting it
f28aaa33 link: size the flat image from the cartridge window, refuse a section outside it by name
5247f1a6 gate: the flat image is bounded by the cartridge window (red arm)
```

The note itself lands as a fourth commit on top. The controller merges and deletes the branch
and worktree; a missing branch afterwards is expected.

## Where the brief was wrong or looser than the code

- "`validate_section` is not on the CLI path": it WAS on the `--map` route (`emit_rom`) and on
  the harness's native path (`native.rs:3729`). It was absent from the two NO-map routes: the
  single-file `.asm` route and `sigil emp` without `--map`. Those are the two now guarded.
- "sigil-link and sigil-harness likely carry the constants": no Rust constant for the cartridge
  or RAM window existed. `0x400000` appeared only inside TOML fixture strings, and the RAM/ROM
  partition is a bare `0x00F0_0000` VMA threshold at three sites (`native::is_rom_section`,
  `native.rs:1454`, emp `place_sections`). The constants were introduced in `sigil-ir::map`,
  matching the shipped `games/*/map.toml` (`size = 0x400000` for `rom`).
- "a RAM-phased section": on the AS surface `phase` moves the VMA only; a phased section's
  bytes still have a physical LMA near 0 and are legitimately in ROM (that is Aeon's RAM
  `vars` shape, reserve-only, zero bytes, and it stays exempt: proven by
  `flatten_refuses_a_work_ram_byte_and_ignores_a_reserve_only_ram_section`). The public
  trigger for a RAM-placed BYTE is `org $FF0000`, which jumps the physical counter.
- The `.emp` surface cannot reach the bound from public input on the no-map route:
  `place_sequential` packs from 0, and the emp lowering produces no `Fill`/`Reserve` that
  could pad past 4 MiB. The seam is wired in `link_sections` and proven at the library level
  (`check_image_bounds_locates_each_refusal_at_its_emitting_fragment`), not end to end.

## Open

- The three `0x00F0_0000` VMA-threshold literals are a different quantity (a VMA partition at
  `$F00000`, not the `$FF0000` LMA window) and were left alone; naming them is a separate,
  byte-neutral tidy.
- A game map whose ROM region extends past the cartridge (a mapper cart) would now pass
  `emit_rom`'s map validation and be refused by `flatten`'s cartridge check, rendered bare
  (`error: ...`) on the map route. No shipped map does this; if one ever should, the bound
  belongs on the map, not the constant.
- The `flatten` signature change touched helper lines in 63 files; a lane editing the same
  helper line conflicts trivially (`.unwrap()` appended).
