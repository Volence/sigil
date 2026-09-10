# UXa F5: the error list is silently partial across phase boundaries

The reproduction behind ledger row `sig-ux-partial-error-list`, reconstructed
and committed here so it cannot go missing again.

**Why it had to be reconstructed.** The row's stated closing condition was "the
seat's 7/5/2 repro re-run against `451cb3e2`". The commit message of `451cb3e2`
cites its witness as `.scratch/repro/control.asm`, which is untracked and does
not exist: `.scratch/` is a per-worktree scratch area and the worktree is gone.
A row cannot be closed against a witness nobody can run, which is the reason
this directory is tracked and the reason the gate below reads these files rather
than writing its own copies into a temp directory.

**This is a reconstruction, not an equivalent.** The source of truth is the
seat's own transcript, Job 3 of
`../2026-09-09-ux-seat-a-task-walk.md` (the "Multi-error ordering, seven
deliberate errors on lines 2 through 8" block, and the `many2.asm` block that
follows it). `many.asm` and `many2.asm` here reproduce all seven of the seat's
diagnostic lines verbatim, including the two fixup offsets `9` and `13`, which
are a function of the byte sizes of the instructions the seat wrote and so could
not have been matched by an approximation. Two differences from the seat's
printed transcript are the product of parcels that landed AFTER its sweep and
are not deviations of this reconstruction:

- `many.asm(2):2:` carries a column. The seat recorded `many.asm(2):`. The
  column is the UXb closure on `parcel/ux-message-stream-and-diagnostic-columns`.
- a trailing `assembly failed: N errors (reported on stderr)` line on stdout.
  That is the UXa F8 closure on the same branch, pinned by
  `crates/sigil-cli/tests/asm_failure_line.rs`.

## The phases, which are what the finding is about

`run_asm` in `crates/sigil-cli/src/main.rs` runs five stages in order, and each
one exits the process on failure:

| # | stage | call | on error |
|---|---|---|---|
| 1 | front end | `assemble_root_located_warned` | `fail_asm` |
| 2 | layout | `sigil_link::resolve_layout` | `fail_asm` |
| 3 | link | `sigil_link::link` | `fail_asm` |
| 4 | image checks | `sigil_link::check_image_bounds` | `fail_asm` |
| 5 | output | `flatten` / `install_artifact` | `fail_asm` |

A stage that fails takes every later stage's diagnostics with it. That is the
mechanism F5 names, and it is NOT the mechanism the owner ruling `d-28`
describes: `d-28`'s question attributes the withholding to "a second, cautious
pass the assembler runs on a failing build", which is the front end's
`defer_unresolved_jsr_jmp` deferral pass, entirely inside stage 1. Removing that
pass (`451cb3e2`) made stage 1 report more. It did not and could not move a
diagnostic across a stage boundary.

## The probes

Each pair is one boundary. The first file holds the errors of two stages; the
second is the same file with the earlier stage's error alone repaired, which is
what makes the later stage's diagnostics appear and so proves they were present
in the first file all along. Without the second file a report of "nothing was
withheld" and a report of "the probe never reached the boundary" are the same
observation.

| probe | boundary | errors present | reported | withheld |
|---|---|---|---|---|
| `many.asm` -> `many2.asm` | front end -> link | 7 | 5 | **2** |
| `layout-then-link.asm` -> `layout-then-link-fixed.asm` | layout -> link | 3 | 1 | **2** |
| `frontend-then-layout.asm` -> `frontend-then-layout-fixed.asm` | front end -> layout | 2 | 2 | 0 |

The third row is the control that keeps the other two honest, and it is also the
one thing `451cb3e2` did change. That is measured here rather than assumed: all
six probes were run against a `sigil` built from `27db72a7`, the commit
immediately before the landing, and against one built from `ee6d1941`.

| probe | at `27db72a7` | at `ee6d1941` |
|---|---|---|
| `many.asm` | 5 errors | 5 errors |
| `many2.asm` | 2 errors | 2 errors |
| `layout-then-link.asm` | 1 error | 1 error |
| `layout-then-link-fixed.asm` | 2 errors | 2 errors |
| `frontend-then-layout.asm` | **1 error** | **2 errors** |
| `frontend-then-layout-fixed.asm` | 1 error | 1 error |

Exactly one cell moves, and it moves in the direction the ruling asked for.
Before the landing the front end deferred an unresolved `jmp`/`jsr` target
symbolically and left it to layout, so a front-end error on the same file hid
it; after it, `frontend-then-layout.asm` reports its own front-end error AND
`unresolved symbol NoSuchTarget in operand` in one run. So the instrument used
here is capable of reading a zero, and does read one, on the boundary that was
fixed. Every other zero in this directory is a zero it declined to read.

### `many.asm`, measured 2026-09-10 at master `ee6d1941`

```
many.asm(2):2: error: `mvoe` is not a recognized 68000 mnemonic
many.asm(3):2: error: unsupported form: moveq data 4660 does not fit in a signed byte
many.asm(4):2: error: `badop` is not a recognized 68000 mnemonic
many.asm(6):2: error: unsupported form: moveq data 22136 does not fit in a signed byte
many.asm(8):2: error: `zzz` is not a recognized 68000 mnemonic
assembly failed: 5 errors (reported on stderr)
```

Lines 5 and 7 are absent. `many2.asm` is `many.asm` with exactly those five
repaired and nothing else touched:

```
many2.asm(5):2: error: unresolved symbol `Nowhere1` for fixup in section sec0 at offset 9
many2.asm(7):2: error: unresolved symbol `Nowhere2` for fixup in section sec0 at offset 13
assembly failed: 2 errors (reported on stderr)
```

Seven present, five reported, two withheld. The seat's numbers, unchanged, more
than a fortnight and one owner ruling later. The withheld pair is emitted by
`crates/sigil-link/src/lib.rs:542`, in stage 3, and stage 3 never ran.

### `layout-then-link.asm`, the shape `s1disasm` is in

```
layout-then-link.asm(4):2: error: unresolved jmp/jsr target in section sec0 references symbol `NoSuchTarget` not defined in this link, ...
assembly failed: 1 error (reported on stderr)
```

`layout-then-link-fixed.asm` is the same file with `NoSuchTarget:` defined,
which repairs the layout error and touches nothing else:

```
layout-then-link-fixed.asm(5):2: error: unresolved symbol `Nowhere3` for fixup in section sec0 at offset 5
layout-then-link-fixed.asm(6):2: error: unresolved symbol `Nowhere4` for fixup in section sec0 at offset 7
```

### `s1disasm`, the live case, read-only at `f6ece65`

```
$ sigil /home/volence/sonic_hacks/s1disasm/sonic.asm -o /dev/null
Uncompressed driver size: 1BC6h bytes.
sonic.asm(81):3: error: sections `sec0` [0x0, 0x2CA) and `sec0#2` [0x0, 0x1BC6) overlap in the image (colliding pins)
assembly failed: 1 error (reported on stderr)
```

Its front end is clean, so `451cb3e2` changes nothing here (the landing note
measures it as 1 diagnostic line both before and after). It stops at stage 2 of
5. **How many errors stages 3, 4 and 5 hold is not merely unknown, it is
unknowable from the outside**, because the only thing that could count them is a
run that gets past the collision. That is the finding stated at its sharpest:
the user is given a list of one and no way to tell whether it is the whole list.

## What is fixed, and what is not

Continuing PAST a failed stage is not available and is not what landed here. The
stages are not independent checks over one input; each consumes the previous
stage's success value. The front end returns `Err(Failure)` carrying no
`Module`, so there is no module to lay out; `resolve_layout` returns
`Err(Vec<Diagnostic>)` carrying no sections, so there is nothing to link. Making
a later stage run on a failed earlier one is a redesign of the interfaces
between them, not an edit.

What landed is the other half of the owner's `d-28` option 3, which
`d-28-answered` explicitly leaves to implementation strategy under `d-2` on
condition of exactly this measurement: the run now says plainly which stages did
not run, so silence is never mistaken for completeness. See
`crates/sigil-cli/tests/partial_error_list_stage_note.rs`, which reads the
probes in this directory.
