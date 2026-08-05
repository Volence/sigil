# 2026-08-04 — THE WARNING TIER: make the invisible visible (close packet)

Status: Merge state lives in the campaign log, not here. Sigil-only; built as
branch `warn-tier` off master `4993825e`. **No aeon commit** — the parcel retires
no corpus warning, and the seven ROM targets are byte-identical.

**Superseded in one place by a same-day follow-up:** §0's headline finding that
`module.path-mismatch` is 84% of the default tally line (93 of 111) was ruled on
by Volence immediately — the lint's rule narrowed to "last id segment == file
stem", taking it 93 → 12 and the whole line 111 → 30. See
`notes/2026-08-04-path-mismatch-narrowing.md`. The measurement below stands as
recorded; only the corpus it describes has moved.

## §0 — THE HEADLINE

**The corpus was never the problem. The compiler was.**

The warn tier was invisible, so nobody had ever counted it. Counted, across all
seven build shapes, it was **252 distinct warnings**. Of those, **nine are true
positives.** The other 243 split three ways:

- **88** were sigil warning the user about a source file **sigil itself
  generated** — `build_emp` writes a synthetic entry module of bare `use <module>`
  lines to drive the reachability BFS, and `[import.no-names]` fires on every one
  of them. The real corpus contains **zero** bare `use` statements across all 122
  modules. Nothing to fix; nothing to even open.
- **43** were sigil warning the user that a proc writes `sr` — because **sigil's own
  `assert` desugaring** emits `move.w sr,-(sp)` … `move.w (sp)+,sr`, a balanced
  save/restore (`eval/diag.rs::build_assert_expansion`, steps 1 and 11). The
  compiler blames the author for the compiler's own emitted code, and the lint's
  message even names the right answer (`preserves(sr)`) for a property the
  expansion already has.
- **93** are a lint asserting a convention the corpus has comprehensively rejected:
  `[module.path-mismatch]` wants each module's id to mirror its directory path;
  **93 of 122 modules** (76%) use flat `engine.<name>` / `games.<g>.<name>` ids over
  a deeper tree, deliberately.

That reframes the parcel's central tension. The overseer's recommendation
(default-on, summarised, plus a count ratchet) was right about the surface and I
have built it. It was wrong about the ratchet, and measuring past the corpus is why
we know: **pinning a count would freeze 243 units of tool defect into a gate.** The
ruling and the alternative I built instead are §3.

## §1 — What was verified OPEN before building

Every claim in the brief was re-derived against this worktree, not taken on faith.

| Claim | Verified |
|---|---|
| `build_emp` filters all its diag sources for `Level::Error` and drops the rest | Yes — master `native.rs:1325` (`mdiags`), `:1354` (`pdiags`), `:1397` (`bdiags`), `:1428` (`place_diags`). Four sources, four `.filter(…== Level::Error)`, nothing else consumes the vectors. |
| `run_build_native` never renders them | Yes — master `main.rs:993` destructures `(rom, listing)` from a 2-tuple; warnings never left `build_emp`'s stack frame. |
| No surface exists anywhere | Yes — `grep -rn 'SIGIL_SHOW_WARNINGS\|--warnings\|show_warnings\|SIGIL_WARNINGS' crates/` → 0 hits at master. |
| Two of `Level`'s three variants are unreachable in practice | Yes for Warning (never rendered). Yes for Note, and *more* so: the corpus emits **zero** Notes, so the tier was both unrendered and unexercised. |
| The last measurement (156) is stale | Yes — the true pre-fix figure is 111 (sonic4 plain) … 242 (config_a), union 252. The 156 predates chain 41/42 and the D1/D2/D3 merge, as the brief warned. |
| The byte bar is SEVEN targets | Derived from `crates/sigil-harness/golden/*.bin` in this worktree: `s4`, `s4.debug`, `demo`, `demo.debug`, `config_a`, `config_b`, `lean`. |

**And one the brief did not name, which the lens panel found (§8, Lens B B1).** The
`.emp` LOWERING was not the only warn-tier producer. `sigil_ir::LinkAssert` carries
its own `level`, and `[layout.odd-item]`'s data-item check is `Level::Warning` — it
is evaluated at LINK time, in `check_link_asserts`, long after every lowering
diagnostic. Both ROM drivers filtered its output to `Level::Error` and dropped the
rest, exactly the way `build_emp` did. **Measured: 75 warn-level asserts are
recorded per sonic4 build** (11 for demo), so the channel is real traffic; all 75
currently pass, so nothing was being lost today — but a word table that landed at an
odd address in a shipped ROM would have warned nobody. Fixed (§4.6), and the fix
carries the only proof the corpus cannot give (§6).

## §2 — THE MEASURED INVENTORY

Measurement was taken **before** any policy was designed (the D-batch lesson), by
threading the collection out of `build_emp` and running all seven shapes with the
full listing on.

### 2.1 Pre-fix, by shape

| shape | warnings | shape | warnings |
|---|---|---|---|
| sonic4 plain | 188 | config_a | 242 |
| sonic4 debug | 240 | config_b | 184 |
| demo plain | 153 | lean | 188 |
| demo debug | 204 | **union (distinct)** | **252** |

### 2.2 Pre-fix, by id

| id | union | verdict |
|---|---|---|
| *(no id)* — `whole-module use … imports no names` | 88 | **FALSE POSITIVE, self-inflicted** |
| *(no id)* — `module X is at Y, which suggests id Z` | 93 | **NOISE — rejected convention** |
| `proc.sr-undeclared` | 52 | **43 FP / 9 TRUE POSITIVE** |
| `proc.clobber-undeclared` | 10 | **FALSE POSITIVE ×10** |
| `proc.undeclared-fallthrough` | 6 | **FALSE POSITIVE ×6** |
| `proc.out-unwritten` | 3 | **FALSE POSITIVE ×3** |

Two of the six classes carried **no `[area.name]` id at all** — 181 of 252 firings,
which would have tallied as one meaningless `unclassified` bucket. Both are fixed
(§4).

### 2.3 THE TRIAGE, class by class, with the evidence

**`import.no-names` — 88 — FALSE POSITIVE, generated by the harness itself.**
`grep -rn --include='*.emp' -E '^\s*use\s+[a-z_0-9.]+\s*$' aeon/` returns **0**.
Every firing is against `__native_flip_entry__.emp`, which `build_emp` synthesises
(`synthetic_entry_src`) and which has no on-disk file — the diagnostics rendered
with no `path:line:col` because there is no path. A build report is for code its
reader can edit. **Taken in this parcel** (§4.3); this is surface correctness, not
backlog retirement.

**`module.path-mismatch` — 93 of 122 modules — the corpus has rejected the
convention.** A sample of the shape:

```
engine.boot          at engine/system/boot.emp        → wants engine.system.boot
engine.tile_cache    at engine/level/tile_cache.emp   → wants engine.level.tile_cache
games.sonic4.sonic   at games/sonic4/player/sonic.emp → wants games.sonic4.player.sonic
```

The ids are flat and semantic; the tree is deep and organisational. That is a
consistent deliberate choice across 93 files, not 93 mistakes. **NOT mine to rule** —
it is a corpus/language-convention decision and it belongs to Volence/Fable. The
lint's *useful* core (catch a typo'd or stale id) survives a weaker rule — "the id's
LAST segment must equal the file stem" — which is recommended in §9. Five entries
are inconsistent even with the corpus's own convention and are worth a look on their
own (`games/sonic4/data/sound/*.emp` declare `data.dac_samples`, `data.mt_bank`, …,
i.e. a third top-level namespace that is neither `engine.` nor `games.`).

**`proc.sr-undeclared` — 52 — 43 FP, 9 TP.** Split by shape: 1 plain-only, 8 in
both, **43 debug-only**. Every one of the 43 debug-only sites is an `assert.w` /
`assert.l` / `assert.b` line. `eval/diag.rs::build_assert_expansion` emits, as steps
1 and 11 of the expansion:

```
move.w  sr, -(sp)          // 1. CCR save
…                          // cmp / bcc / raise tail
.skip:
move.w  (sp)+, sr          // 11. CCR restore
```

`lower/proc.rs:580-586` fires on any instruction whose destination operand is `Sr`
unless the contract declares `clobbers(sr)` / `out(sr)` / `preserves(sr)`. It never
asks whether the write is a **restore of a matching save** — even though the same
function subtracts `verified_preserves_regs(proc, buf)` for the general-register arm
just below it. So a debug `assert` in a proc silently obliges that proc to declare
`clobbers(sr)`, which would then be a lie in the plain shape. **43 FP, one root
cause, precisely located.**

The other **9 are genuine and each is a one-line fix**: `irq.emp:30/32`,
`dma_queue.emp:105/136/144/180`, `ojz_scroll_test.emp:132/141`,
`release_fault.emp:57` — hand-written `move.w #$2700, sr` masks and `move.w (sp)+,
sr` restores in ~4 procs that should declare `preserves(sr)` (or `clobbers(sr)` for
the terminal `release_fault` mask). **These are the parcel's nine real findings.**

**`proc.clobber-undeclared` — 10 — all FP, two mechanisms.**
- 9 debug-only, in `entity_window.emp` assert scan loops. Three of them are
  `move.w (sp)+, d2` / `move.w (sp)+, d5` — **pop halves of push/pop pairs**, i.e.
  the proc *preserves* the register and the lint counts the restore as a clobber.
  The general-register arm does subtract `verified_preserves_regs`, so this is the
  verifier failing to see a save/restore that lives inside a comptime `if DEBUG == 1`
  block.
- 1 always-on: `boot.emp:52`, `lea (SYSTEM_STACK).w, sp` in `EntryPoint`. The reset
  vector establishing the stack pointer is not a clobber of anybody's contract;
  `EntryPoint` has no caller.

**`proc.undeclared-fallthrough` — 6 — all FP, two mechanisms.**
- 5 are **data-only procs**: `Vectors` (the 68000 vector table, pure `dc.l`),
  `BootData`, `BootData_VDPRegs`, `BootData_PostBlob`, `ErrorHandlerBlob`. A proc
  that emits only data has no control flow to fall out of.
- 1 is `Debug_AssertObjLoop` (`core.emp:564`), whose `rts` sits inside a comptime
  `if DEBUG == 1 { … }`. In the plain shape the whole body elides to zero bytes (the
  source comment says so and `core_port`'s `debug_shape_length_diverges` pins it);
  in the debug shape the terminator is there but the lint does not credit a
  comptime-conditional block's terminator.

**`proc.out-unwritten` — 3 — all FP, ONE root cause.** All three declare an `out`
register that a **callee or fallthrough target** writes, and each of those targets
declares the same `out`:

| proc | how `out` is discharged | target's contract |
|---|---|---|
| `S4LZ_DecompressDict` (`s4lz_decompress.emp:74`) | `falls_into S4LZ_Decompress` (declared in its own signature) | `out(a0, a1)` |
| `Art_Decompress` (`load_art.emp:46`) | `jbra ZX0_Decompress` / `jbra S4LZ_Decompress` | both `out(a0, a1)` |
| `Load_Object` (`load_object.emp:38`) | `jbsr AllocDynamic` | `out(a1 if eq)` |

The lint checks only the proc's own instruction writes. **`out()` is not discharged
by a callee's or a fallthrough target's declared `out()`.** One fix retires the whole
class — and it is exactly the kind of contract-closure reasoning `closure.rs` already
does for `clobbers`.

### 2.4 Post-fix inventory (what a build reports today)

| shape | warnings | shape | warnings |
|---|---|---|---|
| sonic4 plain | 111 | config_a | 162 |
| sonic4 debug | 162 | config_b | 111 |
| demo plain | 109 | lean | 111 |
| demo debug | 159 | **union (distinct)** | **164** |

| id | union | shape range |
|---|---|---|
| `module.path-mismatch` | 93 | 93 in every shape |
| `proc.sr-undeclared` | 52 | 6 … 51 |
| `proc.clobber-undeclared` | 10 | 1 … 10 |
| `proc.undeclared-fallthrough` | 6 | 5 … 6 |
| `proc.out-unwritten` | 3 | 3 in every shape |

`unclassified` is **0** in every shape, and every reported warning carries a
`path:line:col` its reader can open. Both are gated (§6).

The default build line, verbatim:

```
warning: 111 warnings — module.path-mismatch 93, proc.sr-undeclared 8, proc.undeclared-fallthrough 6, proc.out-unwritten 3, proc.clobber-undeclared 1; SIGIL_WARNINGS=full to list
```

## §3 — THE POLICY RULING

**RULED: default-ON, SUMMARISED — one tally line naming every firing lint id with
its count. `SIGIL_WARNINGS=full` lists every site; `SIGIL_WARNINGS=off` silences.
Unset or misspelled reads as summary. NO count ratchet; instead a frozen LINT-ID SET
gate.**

### 3.1 Why summary, and the alternatives rejected

**default-on-full — rejected.** 111 lines on a plain build, 162 on debug, against a
build log whose entire useful output is four lines. 96% of those lines were the
compiler's own doing. People would add `2>/dev/null` within a week and the channel
would be lost permanently — the same failure as invisibility, wearing better
manners.

**opt-in behind a flag — rejected.** It is the status quo with extra steps. Nobody
sets a flag they do not know exists; `[operand.const-as-address]`,
`[table.name-collision]` and `[name.shadows-*]` were born invisible *because* there
was no default surface, and adding one that defaults to silent reproduces the bug
exactly.

**default-on-summary — ACCEPTED.** One line, and — this is the property that
matters — **bounded by the number of distinct ids, not the number of firings.**
Today that is five. The count can drift by dozens on ordinary work and the line
stays one line. And the anti-rot property is precise: **a new lint cannot fire for
the first time without changing that line**, because the line enumerates ids. That
is the exact event that went unnoticed three times today.

**a count ratchet pinned at the measured baseline — rejected, with reasons.**

1. **It would pin the wrong party's number.** 243 of 252 firings move when *sigil's
   lints* change, not when the engine changes. A gate that fails because someone
   improved a lint, and passes because someone deleted a proc, is teaching the wrong
   lesson.
2. **It is not shape-stable.** The count runs 109…162 across seven shapes. One
   frozen number cannot describe seven shapes; seven frozen numbers is seven places
   to forget.
3. **It would churn on unrelated work and get rubber-stamped.** 43 of the 52
   `proc.sr-undeclared` firings are `assert` desugar, so *adding one debug assert to
   one proc* moves the baseline. A baseline that must be bumped by parcels that have
   nothing to do with it is a baseline nobody reads before bumping — the slow version
   of the green-test-that-asserts-nothing failure.
4. **Pinning noise makes the noise permanent.** This is the D-batch exemption-list
   lesson: a list written over unmeasured content freezes whatever was there.

### 3.2 What I built instead — the LINT-ID SET gate

`crates/sigil-cli/tests/warn_tier_corpus.rs` pins `WARN_ID_BASELINE`: the **set of
lint ids that fire**, per shape, in the house `D1C_BASELINE` shape (frozen constant,
set diff both directions, failure message that demands adjudication).

Why this is the honest ratchet:

- **It ratchets what the project controls.** *Which lints fire at all* is a decision;
  how many times one of them fires is mostly weather.
- **It is shape-invariant** — measured: the same five ids fire in all seven shapes.
  So four canonical profiles cover the whole byte bar (`config_a` lowers `sonic4
  debug`'s module set, `config_b`/`lean` lower `sonic4 plain`'s).
- **It is low-churn and therefore not rubber-stampable.** An id changes rarely, and
  each change is a one-line diff that has to be explained.
- **It has teeth in BOTH directions.** A new lint firing → `NEWLY FIRING` → someone
  must decide. A class retired → `NO LONGER FIRING` → the win must be recorded.
  Merging D1/D2/D3 today would have tripped it.

**Its limitation, stated plainly: it does not catch growth WITHIN an
already-firing class.** That is the tally line's job, and the tally line is now on
every build. Once the false-positive classes in §2.3 are retired the surviving set
will be small enough that a per-id count baseline becomes cheap and meaningful —
**that is the recommended next step, not this one** (§9).

### 3.3 `Level::Note` renders

It rides the same channel, with its level named (`note:` not `warning:`), and it is
counted separately in the tally head (`3 warnings, 1 note — …`).

The reason is the parcel's own thesis one level down: a tier that is defined,
unreachable **and** unrendered is the same bug. Rendering costs nothing and means
the next `Note` anyone adds is born visible.

**The measurement here is honestly vacuous and I am not hiding it: the aeon corpus
emits ZERO notes.** No corpus evidence can prove this arm. It is proven by unit test
instead (`warning_summary_counts_notes_apart_from_warnings`), and the packet says so
rather than quoting a corpus number that would read the same if the code were
deleted.

## §4 — THE FIX

### 4.1 One location authority (`sigil-frontend-emp`)

New `resolve::manifest::SourceIndex` — builds a `SourceMap` whose index equals the
`SourceId`, and answers `locate(span) -> Option<String>` as `path:line:col`. A
source whose text does not read carries no path, so `locate` returns `None` rather
than a position computed against empty text.

The CLI's `render_program_diags` (the **error** tier) was doing this inline; it now
calls `SourceIndex`, so **errors and warnings render through one code path** and
cannot drift. Errors also gained their level in the rendered line
(`path:line:col: error: …`) — previously the located arm printed no level and the
fallback arm printed `error:` for *any* diagnostic including a warning.

### 4.2 Warnings leave the build (`sigil-harness`)

- `BuildWarning { level, id, location, message, primary }` — a non-error diagnostic
  with its `path:line:col` already resolved (the `Manifest` that owns the source
  texts lives and dies inside `build_emp`), its bracketed lint id parsed out, and its
  span retained so it stays a superset of `Diagnostic` rather than a lossy
  projection. `Display` gives the same shape as the error tier.
- `build_emp` returns `EmpProgram { sections, link_asserts, warnings, sources }`,
  collecting **all four** lowering diagnostic sources — `mdiags`, `pdiags`, `bdiags`,
  `place_diags`. Not one; all four. A surface that caught `bdiags` and kept dropping
  `mdiags` would have recreated the bug one layer down, and `mdiags` is where 93 of
  today's 164 live.
- `build_rom_chained_with_listing` / `build_native_rom_with_listing` return
  `RomBuild { rom, listing, warnings }`. Struct returns rather than growing tuples;
  ~10 call sites updated.
- `collect_warnings(index, sources, generated)` deduplicates by
  `(level, message, span)` — the key `build_program_with` already uses for its own
  cross-module merge — and preserves source order so a diff of two builds is
  meaningful.
- `Level` and `Span` gained `Hash` (they take part in that key).

### 4.3 The tool does not lint code the tool wrote

`collect_warnings` drops diagnostics whose primary span belongs to the synthetic
entry module, by **exact `SourceId`** — the id is a local in `build_emp`, so the
exclusion is the tool/user boundary and cannot widen into an exemption over corpus
content. This is the 88.

`build_emp` (and its ram-harvest twin) now `assert!`s that the id it mints for that
module is not already a scanned file's. The freshness invariant held before and holds
now, but it spans two crates and was unstated; a collision would have silently
rebound a real module's path to a nonexistent one and then filtered that module's
warnings away.

### 4.4 The surface (`sigil-cli`)

`WarningView::{Off, Summary, Full}` from `SIGIL_WARNINGS`, listed in `sigil build`'s
usage text; `warning_summary` builds the tally and `warning_report_lines` builds the
exact stderr lines (both pure and tested); `report_warnings` prints them. A clean
build prints nothing — silence means zero, and only zero.

One policy governs every warn-tier surface of `sigil build`: the ROM build and
`--ram-report` both route through `report_warnings`. Errors are never suppressed by
it — `--ram-report` renders those in full and exits regardless of the view.

### 4.5 Three lints given ids

`[module.path-mismatch]`, `[import.no-names]`, `[ram.no-region]` — the only
warn-tier diagnostics in non-test code that lacked the corpus's `[area.name]`
convention (swept: all 20 non-test `Level::Warning`/`Level::Note` emitters checked).
Message text only; no behaviour change. Without ids, 181 of 252 firings would have
tallied as one `unclassified` bucket and the id-set gate would have had nothing to
key on.

### 4.6 The LINK tier joins the warn tier

Both ROM drivers now fold `check_link_asserts`' non-error output into
`RomBuild.warnings`. That needs a locator at a point where the manifest is long
gone, so `EmpProgram` hands out the `SourceIndex` it already built — one index per
build, shared by the lowering and link tiers, which is also why the index is now
built unconditionally rather than lazily (it costs 543 µs against a 10.2 s build).

### 4.7 What I deliberately did NOT do

**No corpus warning was retired and no lint semantics were changed.** The
false-positive classes in §2.3 are diagnosed, root-caused and recommended (§9), not
fixed. Retiring 43 `proc.sr-undeclared` means changing a lint's semantics; that
deserves its own parcel with its own negative controls, not a rushed rider on the
parcel that discovered it. The 93 `module.path-mismatch` are a convention ruling
that is not mine to make.

## §5 — THE SEVEN-TARGET BYTE BAR

Target list derived from `crates/sigil-harness/golden/*.bin` in this worktree, not
from memory. Built in `capture_goldens.sh` order (four canonical via `./build.sh
<game>`, one shape per invocation; then `--config-a`, `--config-b`, `--lean`, the
last two both clobbering `s4.bin`), canonical rebuilt afterwards. Compared with
`cmp`, not eyeballed CRCs. Re-run in full after the lens-panel fixes.

| target | bytes | crc32 | vs `golden/` |
|---|---|---|---|
| `s4.bin` | 413268 | `36e875f1` | **IDENTICAL** |
| `s4.debug.bin` | 423388 | `ca450ce0` | **IDENTICAL** |
| `demo.bin` | 91224 | `12289484` | **IDENTICAL** |
| `demo.debug.bin` | 93963 | `18e5ec7f` | **IDENTICAL** |
| `config_a.bin` | 423765 | `fa15ffa1` | **IDENTICAL** |
| `config_b.bin` | 304788 | `ed2ad40e` | **IDENTICAL** |
| `lean.bin` | 379822 | `a46a39f6` | **IDENTICAL** |

Seven for seven. The four canonical match the overseer's pre-edit baseline exactly.

`repin --check` → `pins.rs unchanged`.
`refreeze --check` → `OK (tip 'crash-report', chain len 42)`.

## §6 — TESTS, AND WHY EACH IS NON-VACUOUS

+15 tests. For each, the reason it would FAIL if the thing it tests were removed.

**`crates/sigil-cli/tests/warn_tier_corpus.rs` (4, corpus-gated, all SEVEN shapes)**

- `warn_tier_lint_ids_match_the_frozen_baseline` — **negative controls run, both
  directions.** Dropping `proc.out-unwritten` from the baseline produced
  `NEWLY FIRING ["proc.out-unwritten"]` and FAILED; adding a `bogus.never-fires` id
  produced `NO LONGER FIRING ["bogus.never-fires"]` and FAILED; restoring returned
  green.
- `every_corpus_warning_carries_a_lint_id` — asserts the warning list is non-empty
  first, so it cannot pass by measuring nothing. The property it asserts was FALSE at
  master: 181 of 252 firings had no id.
- `the_generated_entry_module_is_not_reported` — asserts `import.no-names` count is 0
  **and** that every reported warning has a location. Non-vacuous in both directions:
  88 such warnings existed before the exclusion, and the lint's teeth against real
  code are proven separately in `module_resolution.rs`, so this cannot be green
  because the lint is dead.
- `the_build_binary_prints_the_tally_and_off_silences_it` — **added because the panel
  found the deliverable untested** (§8, Lens C B1). Runs the real `sigil` binary,
  reads its real stderr, and asserts the tally line appears, names the firing class,
  and disappears under `SIGIL_WARNINGS=off`. **Negative control run:** deleting the
  `report_warnings` call makes it FAIL. Every other gate here reads
  `EmpProgram::warnings` directly and stays green through that deletion.

**`native.rs::warn_tier_tests` (5, pure, no corpus)**

- `collects_only_reportable_non_errors` — one Warning at a real on-disk fixture, one
  Error, one Warning at the generated source, one Note; asserts exactly two survive,
  in order, and that the located one reads `real.emp:3:1`. Stubbing any of the four
  filters changes the result.
- `deduplicates_a_replayed_diagnostic_but_not_distinct_sites` — the same diagnostic in
  three sources plus one at a different offset → exactly 2.
- `errors_alone_produce_an_empty_tier` — errors are reported through `Err`, never
  twice.
- `a_failing_warn_level_link_assert_survives_the_error_filter` — **the only proof
  available for §4.6, and the panel's finding is why it exists.** Drives a
  `Level::Warning` `LinkAssert` to failure through the real `check_link_asserts` and
  asserts the resulting diagnostic survives `collect_warnings` with its
  `[layout.odd-item]` id and its location; the `Level::Error` twin must NOT. The
  corpus records 75 warn-level asserts per build and **all of them pass**, so a corpus
  measurement of this path would read identically whether or not the wiring exists —
  stated here rather than papered over.
- `an_unbracketed_message_has_no_id_and_an_unlocatable_span_renders_bare`.

**`resolve_manifest.rs` (2)**

- `source_index_locates_real_spans_and_declines_unreadable_ones` — a real span
  resolves to `engine/thing.emp:3:1` (a *specific* line, so an off-by-one fails); a
  registered-but-unreadable source and an out-of-range id both give `None` rather
  than panicking.
- `path_mismatch_lint_carries_its_id` — would have failed at master.

**`main.rs` (4, pure)**

- `warning_summary_tallies_by_id_most_frequent_first` — pins the exact string,
  including the count-descending / id-ascending tie-break and the `unclassified`
  bucket.
- `warning_summary_counts_notes_apart_from_warnings` — the Note arm the corpus cannot
  prove (§3.3), stated as such in the test's own doc.
- `warning_view_defaults_to_summary_on_anything_unrecognised` — **caught and fixed
  mid-parcel:** the first draft re-implemented the match arms in the test body, which
  would have read identically had `WarningView::parse` been deleted. `from_env` was
  split into a pure `parse(Option<&str>)` and the test now calls the real function.
- `warning_report_lines_render_each_view` — the exact stderr lines per view, including
  that `Full` puts the tally LAST. `warning_summary` alone would still pass with the
  printer unwired.

## §7 — STRICT SUITE

```
AEON_DIR=<aeon-wt> SIGIL_EMIT=… SIGIL_BUILD=… SIGIL_STRICT_GATE=1 \
  cargo test --workspace --release
```

| | master `4993825e` | branch `warn-tier` | delta |
|---|---|---|---|
| result lines | 305 | **306** | +1 (the new gate binary) |
| passed | 3080 | **3095** | **+15** |
| failed | 0 | **0** | 0 |
| ignored | 4 | **4** | 0 |

Delta accounting: +4 `warn_tier_corpus.rs`, +5 `native.rs::warn_tier_tests`,
+2 `resolve_manifest.rs`, +4 `main.rs`. **Exactly 15.**

Cross-check: `git grep -c '^\s*#\[test\]' -- 'crates/**/*.rs'` gives 3084 at master
and **3099** on the branch (+15), and 3095 passed + 4 ignored = **3099**. Nothing is
being silently skipped.

**Clippy.** `cargo clippy -D warnings` fails on master and keeps failing —
`sigil-ir/src/symbols.rs:55`, plus `sigil-frontend-as/src/eval.rs:296/2328`,
`sigil-link/src/relax.rs:2181`, `sigil-frontend-emp/src/lower/proc.rs:300` and
`regions.rs:116`. All toolchain drift, none mine. To answer "did I add findings", the
whole workspace was linted with the DEFAULT lint set and the results collected as
structured JSON: **62 findings, none in any line this diff adds** (verified against
`git diff -U0` hunk ranges for every touched file). Three findings *were* mine during
the parcel — `doc_lazy_continuation`, `cloned_ref_to_slice_refs`, `type_complexity` —
and all three are fixed. My first sweep used a restricted lint set and would have
missed two of them; Lens A caught one, which is why the final sweep runs the default
set.

## §8 — LENS PANEL

Three fresh read-only lenses over the finished diff. They returned **two genuine
blockers on work that had already passed every gate green**, and one of them
(Lens B B1) is a warn-tier producer the brief did not name and I had actively
mis-verified as absent — my first grep for `LinkAssert { … level`  used too small a
context window and found only the `Level::Error` construction site.

### Lens A — ceremony / style (20 findings)

| # | finding | adjudication |
|---|---|---|
| 1-4 | BLOCKER: change-history narration in four doc comments ("was FALSE before…", "like the warn tier was", "born invisible on the day they merged", "now on every build") | **FIXED** — all four rewritten to present-tense contract facts. |
| 5 | `Ord`/`PartialOrd` on `Level` are dead, and the doc claims a sort nothing performs | **FIXED** (self-caught before the report landed) — derives dropped, doc rewritten to the actual use. |
| 6 | comment says "two-line fixture"; it is three lines and the test depends on it | **FIXED** — states the offset the test relies on. |
| 7 | `Diagnostic` imported twice (`Diagnostic` and `Diagnostic as D`) | **FIXED**. |
| 8 | test name says what is exercised, not what is guaranteed | **FIXED** — renamed. |
| 9 | `assert!(a == b)` because `WarningView` lacks `Debug`; failures print nothing | **FIXED** — `Debug` derived, all seven switched to `assert_eq!`. |
| 10 | counterfactual narration in `collect_warnings`' call-site comment | **FIXED**. |
| 11 | the gate references a note file that is untracked | **FIXED** — the note is committed in the same commit. |
| 12 | `SIGIL_WARNINGS` is undiscoverable (`off` never mentioned anywhere) | **FIXED** — added to `sigil build`'s usage text. |
| 13 | two warn surfaces, two policies: `--ram-report` ignores `WarningView` | **FIXED** — routed through `report_warnings`; errors still render unconditionally. |
| 14 | NIT: measurements frozen into gate comments that no gate holds, and that the file's own thesis says will churn | **FIXED** — digits removed, the note cited. |
| 15 | NIT: four byte-identical baseline rows | **FIXED** (with Lens C N8) — shared `CORPUS_LINTS` const plus a per-shape slot, so a corpus-wide change is a one-line diff and a shape divergence is still expressible. |
| 16 | NIT: `fn aeon()` vs the house `fn aeon_dir()` | **FIXED**. |
| 17 | NIT: `report_warnings`' doc is a nine-line essay | **FIXED** — trimmed; the argument lives here. |
| 18 | NIT: two lines over the ~100-col house width | **FIXED** (the third was pre-existing). |
| 19 | NIT: new `clippy::cloned_ref_to_slice_refs` in added test code | **FIXED** — `slice::from_ref`. Also prompted re-running the clippy sweep with the default lint set (§7). |
| 20 | NIT: 12 corpus lowerings per gate run | **FIXED** — `OnceLock`, one lowering per shape per binary. |

Lens A also confirmed clean: brace-indent throughout, doc comments on every new
public item, no leftover debug code, and that the three id additions break no
existing message assertion.

### Lens B — corpus pattern (1 blocker, 5 should-fix, 4 nits)

| # | finding | adjudication |
|---|---|---|
| **B1** | **BLOCKER: `check_link_asserts`' Warning tier is still dropped, in the two functions this parcel rewrote to carry warnings out.** `LinkAssert::level` exists for exactly this; `[layout.odd-item]` is `Level::Warning`; `sigil emp --root` has handled it correctly the whole time and the native drivers diverge. | **FIXED** (§4.6). Verified open by reading `lower/mod.rs:1007-1016` — my earlier "only one construction site, always `Error`" was wrong. **Measured: 75 warn-level asserts recorded per sonic4 build, 11 for demo; zero currently fail**, so the count did not move. Proven by `a_failing_warn_level_link_assert_survives_the_error_filter`, which drives the failure the corpus cannot. |
| S2 | `BuildWarning` discards the `Span`, closing the door on `--deny-todo` / caret rendering / LSP | **FIXED** — `pub primary: Span` added; it also became the dedup key (Lens C N6). |
| S3 | `BuildWarning`/`collect_warnings` live in `native.rs` (a ROM-driver module) though nothing in them is ROM-specific; the CLI now reaches through `native` for a generic utility | **DECLINED, ledgered.** Correct, and the right home is beside `SourceIndex` — but it is a cross-crate move touching every call site, in a parcel that already moves ~800 lines, and it would obscure the diff the overseer has to review. §9 item 9. |
| S4 | four other diagnostic renderers in `main.rs` (`check` / `test` ×2 / single-file `emp`) still hand-roll a `SourceMap` and print without the level word | **DECLINED, ledgered.** Pre-existing drift this parcel did not create; unifying five renderers is its own parcel. The two the parcel touched do share one path. §9 item 10. |
| S5 | the default line is 84% a lint the corpus has formally rejected, and no `@allow` path exists for any firing class | **DECLINED, with argument.** The line reading `module.path-mismatch 93` is not noise about the corpus — it is the measurement that forces the ruling, and it is the first thing anyone will ask about. Silently demoting or `@allow`-ing it would bury the decision the surface exists to surface. §9 item 6 now carries both numbers (93 under the current rule, **12** under the weaker one) so the ruling costs one line. |
| S6 | the harvest-subset claim is right about `harvest_engine_ram_addresses` but wrong about the other three, which run *different analyses* (`eval_all_pub_consts`, `layout_struct_ambient`) | **ACCEPTED and re-measured.** The argument was structural and false. Replaced with a measurement: instrumented all three and counted their non-error diagnostics over the real corpus — **zero**. §9 item 11 keeps the row. |
| N1 | sequential partial moves where the parcel's own idiom is destructuring | **FIXED** — five sites. |
| N2 | `SourceIndex` is a `sigil-span` concept living in `sigil-frontend-emp` | **DECLINED, ledgered** with S3. |
| N3 | the synthetic-entry `SourceId` density invariant is real and unstated | **FIXED** — `assert!` at both mint sites (also Lens C S2). |
| N4 | four identical baseline rows | **FIXED** with Lens A 15. |

Lens B also confirmed: the `[area.name]` id sweep is complete (all 20 non-test
emitters enumerated), the struct-return conversion left no mixed convention, the
shared locator cannot panic, `aeon/build.sh` does not filter stderr, and the AS
frontend's structurally-lossy `Ok`-path drop is **latent, not live** (it emits no
warn-tier diagnostic today) — ledgered, not this parcel's debt.

### Lens C — perf, hazard, policy (1 blocker, 5 should-fix, 8 nits)

| # | finding | adjudication |
|---|---|---|
| **C1** | **BLOCKER: the deliverable has zero coverage. Delete `report_warnings(&warnings)` and the whole suite stays green** — every gate reads `EmpProgram::warnings` directly and nothing ever reads stderr. "The exact defect class the parcel is fixing, reintroduced one level up." | **FIXED.** `warning_report_lines` split out and unit-tested per view, plus `the_build_binary_prints_the_tally_and_off_silences_it`, which drives the real binary and reads real stderr. **Negative control run: deleting the call FAILS it.** |
| S1 | the gate's stated reason for watching 4 of 7 shapes is measurably false — `lean` drops one firing and gains one in a *different class*; the id sets coincide by luck | **FIXED, the strong way.** The gate now watches **all seven shipped shapes** (13 s), and the comment states the measured divergence instead of the false claim. |
| S2 | the synthetic-`SourceId` freshness invariant is load-bearing, cross-crate and unasserted; a collision fails silently and compounds | **FIXED** — `assert!` at both sites. |
| S3 | the policy is not uniform; `[ram.no-region]` is an orphan id no surface can ever show | **FIXED** — `--ram-report` routes through `report_warnings`, and the new end-to-end test drives exactly that id through the tally. |
| S4 | the dedup test is near-vacuous and its doc overstates the guarantee: cross-source replay is structurally impossible, `pdiags` is 100% filtered, and `build_program_with` already dedups on a stricter key | **ACCEPTED** — doc rewritten to the true claim and keyed on the same `(level, message, span)` triple `resolve/mod.rs` uses. The test stays as a forward contract; the always-filtered `pdiags` slot stays because the source list is what makes "all four sources" checkable at a glance. |
| S5 | the ram-harvest twin still drops its tier | **DECLINED, ledgered** — measured zero non-error diagnostics (with B S6), and it is a harvest pass, not a shipped artifact. §9 item 11. |
| N1 | dead `Ord`/`PartialOrd` | **FIXED** (Lens A 5). |
| N2 | the tally line is 178 chars at five ids; suggest a multi-line form above six | **DECLINED, with argument.** One greppable line is the entire ergonomic case for the summary view; turning it into a six-line block at five classes rebuilds, in miniature, the wall the design rejects. The length is bounded by lint *classes*, not sites. Worth revisiting past ~10 classes; recorded. |
| N3 | `a_clean_build_reports_nothing` reads the same with the early return deleted | **FIXED** — renamed to `errors_alone_produce_an_empty_tier`, which is what it pins. The early return itself is gone (§4.6 needs the index unconditionally). |
| N4 | the clean-case early return is oversold at 543 µs / 10.2 s, and `=off` suppresses printing, not work | **RESOLVED by §4.6** — the early return no longer exists. That `=off` does not skip the collection is correct and now unremarked rather than mis-claimed. |
| N5 | `SourceMap::location` is O(bytes-before-span); 49 µs worst case today, ~5 s at 100 000 warnings | **DECLINED, ledgered.** Measured immaterial (82 µs for all 111). §9 item 12. |
| N6 | dedup keys on the rendered location string, so same-start/different-end spans collapse and unlocated warnings collapse across sources | **FIXED** — keyed on `Span`. |
| N7 | `link_rom`'s `emit_rom` error fabricates `SourceId(0)`, and the added `error:` makes the false attribution read more authoritative | **FIXED** — an out-of-range id, so it degrades to a bare `error: <msg>`. |
| N8 | four identical baseline rows | **FIXED** with Lens A 15. |

Lens C also independently cleared: `SIGIL_WARNINGS=off` cannot silence the gate
(which never constructs a `WarningView`); `build.sh` does not redirect stderr;
typo'd/uppercase/empty env values all fall through to summary (verified live);
`SourceIndex` gap and out-of-range handling cannot panic; `render_program_diags`
has no perf regression (identical work, now shared); `build_emp` runs once per
build; and three of the four gates it examined are genuinely non-vacuous with the
reasons spelled out.

## §9 — RECOMMENDED FOLLOW-UPS (ranked; none taken here)

1. **`proc.out-unwritten` — credit callee/fallthrough `out()`.** 3 FP → 0, one fix,
   the smallest and cleanest. `closure.rs` already does this shape of reasoning for
   `clobbers`.
2. **`proc.sr-undeclared` — do not lint the compiler's own assert desugar.** 43 FP →
   9 real. Two candidate mechanisms: extend `verified_preserves_regs`-style balance
   detection to the `sr` arm (a matching `move.w sr,-(sp)` / `move.w (sp)+,sr` pair
   is a preserve), or mark generated instructions as tool-emitted and exempt them.
   The first is better — it also fixes hand-written balanced pairs.
   **DONE 2026-08-05 (codeitem-author parcel), via the second mechanism made
   sound:** `CodeItem::Instr` carries `author: ItemAuthor`; the desugar's items
   are `AssertDesugar`-authored and exempt, with the balance proof pinned at the
   emission site (not per-corpus). DEBUG-shape firings 42/41/42 (chain-47
   measurement) → 0; every `WARN_ID_BASELINE` row is empty, so a new
   hand-written undeclared SR write APPEARS in the id set instead of joining a
   crowd. (The hand-written-balanced-pair half of option one remains undone —
   a hand pair still wants `preserves(sr)` declared, which is the honest
   spelling anyway.)
3. **The nine TRUE POSITIVES** — declare `preserves(sr)` on the ~4 procs in
   `irq.emp`, `dma_queue.emp`, `ojz_scroll_test.emp` and `clobbers(sr)` on
   `release_fault.emp`'s terminal mask. An aeon parcel; byte-neutral.
4. **`proc.clobber-undeclared`** — see a `move.w (sp)+, dN` inside a comptime `if`
   as the pop half of a preserve; exempt `sp`/`a7` establishment in a proc with no
   caller.
5. **`proc.undeclared-fallthrough`** — do not fire on a proc that emits only data;
   credit a comptime-`if` block's terminator.
6. **`module.path-mismatch` — a ruling for Volence/Fable, and the numbers are
   measured so the decision is one line.** Under the current rule (id mirrors the
   full path) **93 of 122** modules fire. Under the weaker rule "the id's LAST
   segment equals the file stem" — which keeps every bit of the lint's typo- and
   stale-rename-catching value — **12 of 122** fire, and all twelve look like genuine
   disagreements worth a human's eye (`engine.s4lz` at `s4lz_decompress.emp`, the ten
   `…_act1`-suffixed generated modules, `games.sonic4.parallax_configs` at
   `configs.emp`). Separately, the five `data.*` modules under
   `games/sonic4/data/sound/` declare a third top-level namespace that is neither
   `engine.` nor `games.` — inconsistent with the corpus's own convention, not just
   with the lint's.
7. **Then, and only then, the per-id COUNT ratchet.** With the FP classes retired the
   surviving surface is small enough that per-`(shape, id)` counts stop churning and
   start meaning something.
8. **`[layout.odd-item]` now reaches the surface but has never fired.** 75 warn-level
   asserts are recorded per sonic4 build and every one passes. Worth one deliberate
   negative control on the corpus (misalign a word table in a scratch branch and
   watch the tally) so the path is known-good end to end, not only unit-proven.
9. **Move `BuildWarning` / `collect_warnings` out of `native.rs`** (Lens B S3) — they
   are diagnostic-presentation types with nothing ROM-specific in them, and the CLI's
   `--ram-report` path now reaches through a ROM-driver module to get at them. Beside
   `SourceIndex` is the right home; `SourceIndex` itself arguably belongs in
   `sigil-span` with a `from_manifest` adapter (Lens B N2).
10. **Unify the remaining four diagnostic renderers** (Lens B S4) — `sigil check`,
    `sigil test` ×2 and single-file `sigil emp` each hand-roll a `SourceMap` and print
    without naming the level, so the same binary labels severities on one path and not
    on four. Blocked on item 9 (they cannot use a `Manifest`-coupled `SourceIndex`).
11. **The harvest paths still drop their warn tier** (Lens B S6, Lens C S5) —
    `harvest_engine_constants`, `harvest_game_constants`,
    `harvest_engine_struct_offsets`, `harvest_engine_ram_addresses`. Measured to emit
    **zero** non-error diagnostics over today's corpus, and they produce no shipped
    artifact, so nothing is lost now. But three of them run analyses `build_emp` never
    runs (`eval_all_pub_consts`, `layout_struct_ambient`), so "the main build is a
    superset" is a measurement, not a structural guarantee.
12. **`SourceMap::location` is O(bytes-before-span)** (Lens C N5) — 82 µs for all 111
    of today's warnings, but a lint that ever fires per-instruction would make it
    quadratic. Precompute line-start offsets in `SourceIndex::new` when that day comes.
13. **Revisit the one-line tally above ~10 distinct classes** (Lens C N2). At five it
    is 178 characters and still one greppable line; at fifteen it would not be.
14. **The AS frontend's `Ok`-path diagnostic drop** (Lens B, clean list) —
    `assemble_root_impl` cannot carry warnings out on success because its
    `Result<Module, Vec<Diagnostic>>` has nowhere to put them. Latent (the AS frontend
    emits no warn-tier diagnostic today), but the `sigil-frontend-as PERMANENT` ruling
    means it will not age out on its own.

## §10 — STEP-3 (LANGUAGE / TOOLING) vs STEP-5 (ENGINE)

### Step 3 — what this taught the LANGUAGE and the TOOLING

- **A diagnostic tier with no default surface is not a tier — it is dead code with
  good intentions.** Sigil had three severity levels and shipped exactly one. The
  cost was not theoretical: three lints merged in a single day, all fired on the
  corpus, and all three left no trace anywhere.
- **A compiler must not lint its own output.** Sigil does it in two places, and they
  are 131 of 252 firings: the synthetic reachability entry (88) and the `assert`
  desugar (43). Both are the same bug — a lint pass running over instructions no
  human wrote — and it is worth stating as a rule the next desugar has to obey:
  *generated code carries the generator's authorship, and the generator does not get
  to blame the author for it.*
- **A lint id is not decoration; it is the join key.** Two lints without ids were
  71% of the tier. With no id there is no tally, no gate, no `@allow`, no way to
  discuss "that class". The `[area.name]` convention should be enforced at the
  emission site, not by convention — a `push(diags, Level::Warning, …)` helper that
  takes the id as a separate argument would make an un-idded warn-tier diagnostic
  unrepresentable.
- **A contract lint has to reason over the call graph or it will lie.** All 3
  `proc.out-unwritten` and several `proc.clobber-undeclared` are the same shape: the
  lint reads one proc's instruction stream while the property it checks is a property
  of the proc *plus its declared callees and fallthrough target*. The closure
  machinery already exists for `clobbers`; `out` and `sr` never got it.
- **Ratchet what the project controls.** The instinct to pin a number is right; the
  discipline is to pin the number whose movement is a *decision*. Here that was the
  id set, not the count.
- **Measure before you rule — the brief said it and it paid for itself twice.** The
  stale 156 was wrong in both directions (the real figure was 252, and the real
  actionable figure was 9), and the recommended ratchet would have frozen the
  compiler's own defects into a gate.
- **A diagnostic tier has as many producers as it has phases, and this one has two.**
  I found the lowering tier, built a surface for it, and wrote a packet claiming the
  tier was now covered — while the LINK tier's `Level::Warning` asserts kept going
  nowhere. `sigil emp --root` had handled them correctly all along, which is the
  worst kind of gap: the codebase already knew the answer and the path that matters
  most had diverged from it. When adding a surface, enumerate the *phases* that can
  produce the thing, not the call sites you happened to grep.
- **A surface needs a test that reads the surface.** Every gate this parcel wrote
  inspected the data structure; deleting the one line that PRINTS it left them all
  green. The distinction between "collected" and "reported" is the entire parcel, and
  nothing tested it until the panel said so.

### Step 5 — what this taught the ENGINE

- **Nine honest contract omissions**, all `sr`, in four procs: `irq.emp`,
  `dma_queue.emp`, `ojz_scroll_test.emp` (`preserves(sr)` — they save and restore),
  `release_fault.emp` (`clobbers(sr)` — a terminal mask that never returns). Small,
  real, and the engine's to fix.
- **The `.emp` module naming convention is undocumented and universally practised.**
  93 of 122 modules use flat semantic ids over a deeper tree. The engine has a
  convention the tooling does not know about; one of the two should move, and either
  way it should be written down.
- **Five sound-data modules sit outside the engine's own convention** —
  `games/sonic4/data/sound/{dac_samples,mt_bank,sfx_bank,sfx_blob_win_tab,
  movingtrucks_pitchtable}.emp` declare a bare `data.*` top-level namespace that is
  neither `engine.` nor `games.`. Probably a leftover from the sound flip.
- **`entity_window.emp` and `core.emp` carry 20 and 12 of the 40 `proc.*` firings**
  between them, all in debug-gated assert blocks. Nothing is wrong there today, but
  when the lints are fixed those two files are where the remaining signal will land.

### Neither bucket — the headline that belongs to neither

**The build was lying by omission for the entire campaign, and the lie was
comfortable.** A silent build reads as a clean build. Nobody had a reason to doubt
it, so nobody counted, so three lints could merge in one day and vanish. The
expensive part was never the fix — the surface is about 120 lines — it was that no
number existed. The general rule: **if a system can produce a signal that nothing
consumes, it will eventually produce nothing but signals that nothing consumes**, and
the cheapest defence is to make the *count* impossible to miss even when the
*contents* are not worth reading.
