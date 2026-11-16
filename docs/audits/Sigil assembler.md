---
kind: audit
status: awaiting-owner
wave: 1
verdict: "on par; not best in class"
description: "Keep the core, patch the .emp front end, rewrite the build driver beside. On par with the best common assemblers overall: beyond them on 68000 contract checking, below the old AS toolchain on debug info, and two ways to ship a wrong ROM from a green build (one reproduced: a Z80 bank overrun by 13,888 bytes, exit 0). CI is red on master. It cannot build the 4 MB Z80-heavy game safely or a second game at all without Rust edits."
---
# Sigil assembler

## Stages (live)

![[Audit stages.base#Sigil assembler]]

Then: 6. Lead (Fable) merges into this page; 7. Controller verifies and reports to you.

## Verdict

> [!verdict] On par with the best common assemblers. Not best in class. Keep the core, patch the `.emp` front end, rewrite the build driver beside
> - **Is it best in class? No.** Both reviewers say "on par with the best common tools overall", and I agree. The template's four-word scale has no such word. Per axis, in the template's words: **beyond** on 68000 contract checking, **best known** on encoding, relaxation and reproducibility, **below the floor** (the AS toolchain it replaced) on debug info, and **below the common tools** on region-size enforcement, `.emp` immediate range checking, diagnostics form, incremental builds and second-game support.
> - **Its "no" is not trustworthy in two places.** A green build can ship wrong bytes through an out-of-range immediate, and through a section that outgrows its region. Reviewer B built a ROM, exit 0, whose bank-1 sound data runs 13,888 bytes past the 32 KB window the Z80 can see (MEASURED, release Sigil, s4 plain shape, scratch copy of aeon HEAD). No Aeon ROM is wrong today. The check that would stop a wrong one is missing.
> - **The Z80 half is checked less than the 68000 half.** The whole-program clobber check does not gate Z80 code, and the build listing holds no Z80 driver symbol.
> - **The named workload fails on the driver, not on the assembler.** A 4 MB, Z80-heavy game is up to 128 bank windows whose sizes are not enforced. A second game is refused outright (`--game thunder`: exit 2, MEASURED) and needs Rust edits in six places plus a rebuilt Sigil.
> - **CI has been red on master since 2026-09-28** and nobody noticed. The whole-ROM goldens are Sigil's own output, pinned 63 commits behind aeon HEAD.
> - **Disposition.** Keep `sigil-isa`, `sigil-ir`, `sigil-link`, the back ends, the AS front end (frozen) and the compression crates. Patch `sigil-frontend-emp`. Rewrite beside: the build driver in `sigil-harness` (`native.rs`, the sound seams, game profiles, baselines, pins), as data owned by the game repo, with the current driver as the byte reference. Nothing here calls for replacing Sigil.
> - **Is the architecture right for what the language audit chose? Yes.** [[EMP language]] chose exact instructions + checks + fixed-expansion sugar now. All of that lives in `sigil-frontend-emp`, which stays. A later choreography tier should emit `CodeBuf` (the instruction-level form the checkers run on), not `sigil-ir`. The middle of a compiler (typed IR, register allocation, instruction selection) does not exist.

Trees: sigil `7630ba0d`, aeon `936439fa` (HEAD) and `1ee78b88` (the revision the goldens pin). Machine for every timing: Ryzen 7 9800X3D, 16 threads, shared with other audit agents (load 9 to 27), builds on tmpfs, wall clock. "Release Sigil" means Sigil's cargo `release` profile. The shape names (s4, s4.debug, demo and so on) are the ROM shapes Sigil builds.

| Term | Meaning |
|---|---|
| Driver | `sigil-harness/src/native.rs` and the seams: the code that turns "build Aeon" into front-end, placement and link calls |
| Seam | Rust code that links the Z80 sound driver and its banks ahead of the main build (`seam1.rs`, `seam2.rs`) |
| Closure gate | The whole-program check that every procedure's `clobbers` line is true through every call. It stops the build |
| Golden | A stored ROM that later builds must match byte for byte |
| `CodeBuf` | Sigil's list of instructions with labels, source spans and authorship, before encoding |

## What it does best

- **Whole-program register contracts as a hard gate, for 68000 code.** One s4 build decides 89,905 comptime `ensure` verdicts and 696 to 698 link-time asserts (MEASURED, release Sigil, s4 plain, `--check`). No assembler the research surveyed (AS, vasm, ca65, RGBDS, KickAssembler, 64tass) has this layer.
- **A small, independently gated core.** `sigil-isa` + `sigil-ir` + `sigil-link` + the back ends are about 7,600 code lines. They are checked against 329 frozen `asl` vectors, Capstone differentials, and stock Sonic 1 and Sonic 2 built whole against an md5-pinned real `asl` (MEASURED, stage 2's strict run).
- **Relaxation with a termination proof** (`sigil-link/src/relax.rs:57-75`, `:1053-1078`): grow-only, a derived pass cap, a loud report. Stricter than LLVM MC's or vasm's. This holds for the inner loop only (finding 10).
- **Determinism.** 5 builds of each of 7 shapes gave 1 ROM hash per shape; a DEBUG-profile Sigil and a release Sigil emit the same s4 (MEASURED).
- **Loud refusals where a check exists.** Under the strict flag a missing reference tree is red, not a silent skip (MEASURED: stage 2's first, unprovisioned attempt failed by name).
- **Tests of the tool itself.** Strict suite, release profile, aeon `1ee78b88`: 521 binaries, **5,864 passed, 3 failed, 3 ignored**, 534 s wall at load 13 to 27 (MEASURED; the checker re-aggregated the log and got the same totals). The 3 failures are `landing_verdict` tests that refuse a tmpfs target dir, which the audit rules required. They **DID NOT RUN validly** and are not passes.

## Findings

Ranked by how much wrong output a green build can hide, then by what blocks the named workload. Paths are under `sigil/crates/` unless they start with `aeon/`. I re-opened the cited lines for findings 1, 2 and 10 myself at `7630ba0d`; they say what the reviewers say. I ran no builds.

> [!finding] 1. A section can outgrow its region and still reach the ROM (CONFIRMED, reproduced)
> - `validate_section` picks the region with `region_for(lma)`: the **first** `rom` region that contains the address (`sigil-ir/src/map.rs:99-101`, `:135-136`). It never uses the region the section was placed into. Aeon's map lists the whole cartridge first, so every section is checked against 4 MB and nothing smaller.
> - `z80_sound_bank`'s `size` is read by nothing: shrinking it from `0x8000` to `0x100` gives the identical ROM. Only one budget can be enforced and its name is a Rust literal (`sigil-harness/src/native.rs:4479`). The seven synthetic seam-2 region sizes are unenforced; `sound_tables_z80` is 1,116 B against a `0x400` region today.
> - **The reproduction.** Add 23,008 B after the bank-1 SFX block: the build stops with `[map.undeclared-island] … add [[anchor]] at = 0xC8000`. Follow that advice: exit 0, 883,143 B, and bank 1 holds bytes from `0xC0000` to `0xC3640`. A `winptr()` is a mask, not a range check (`sigil-link/src/lib.rs:1594`), so the Z80 would read the same offset in the wrong bank.
> - What does hold: the object-bank budget, one `bank: $8000` section larger than its window, and the cartridge edge.
> - All MEASURED by reviewer B, release Sigil, s4 plain, scratch copies of aeon HEAD. This is the open queue row `SEAM2-TABLE-REGION-UNENFORCED`. Four tests pin it as current behaviour (`sigil-harness/tests/seam2_region_unenforced.rs`) and they passed in stage 2's run. Both reviewers missed it in their first round and one stated the opposite.

> [!finding] 2. `.emp` instruction immediates are truncated in silence (CONFIRMED, reproduced by stage 2, both reviewers and the checker)
> - `move.w #$12345, d0` assembles to `303C 2345`; `move.b #300, d1` to `123C 002C`; `andi.b #$1FF, d0` to `0200 00FF`. Exit 0, no diagnostic (MEASURED, release Sigil, `sigil emp`).
> - Each layer documents that the other one checks. The lowering: "the encoder does the size-specific business-rule check" (`sigil-frontend-emp/src/lower/code.rs:1396-1405`). The encoder: validation "is the front-end's responsibility", then it masks (`sigil-isa/src/m68k.rs:13-21`, `:1907-1913`).
> - The check exists three times elsewhere: the AS front end (`sigil-frontend-as/src/eval.rs:3333-3338`), link-time word immediates (`sigil-link/src/lib.rs:752-775`), and the Z80 path of the same file (`lower/code.rs:2035-2051`).
> - Two more defects of the same family: `const X: u8 = 300` is accepted and `X` stays 300 (the annotation is ignored, `eval/const_arity.rs:105-107`); `btst #40, d0` and `bset #9, (a0)` are accepted and act on bit 8 and bit 1.
> - 5,864 tests passed over this because no gate compares the `.emp` front end's operand windows with anything independent. This is the same hole as [[EMP language]] finding 3: one fix, counted once.

> [!finding] 3. Z80 code is outside the whole-program clobber gate (CONFIRMED, reproduced)
> A Z80 procedure declaring `clobbers(a, f)` that calls one clobbering `b` builds clean, at every warning level. The same mistake in 68000 code fails the build (MEASURED, release Sigil, s4 plain). The Z80 closure exists (`sigil-harness/src/seam1.rs:1037-1060`) but its only callers are in a test file, and it filters by Aeon procedure name (`:1030-1032`). The per-procedure Z80 `preserves` and `out` proofs do run in the build.

> [!finding] 4. The gates that say "green" do not cover today's ROM, and CI is red (CONFIRMED)
> - **CI:** the last 12 runs on master (2026-09-28, the day of the last commit) all failed (MEASURED with `gh run list` by the checker and reviewer A). A port gate reads an aeon file with no skip guard (`sigil-cli/tests/test_g4_final_objects_port.rs:138`, `:417-433`). The same class was recorded fixed on 2026-09-07 and came back three weeks later. The landing bar cannot see it.
> - **Goldens:** the 7 whole-ROM goldens and the 62 `*_port` gates compare Sigil with its earlier self at aeon `1ee78b88`. At aeon HEAD, s4 is 850,296 B against the golden's 845,398 B. Nothing independent says HEAD's bytes are right (MEASURED, stage 2).
> - The independent oracles are real, and they cover the encoders and the AS route. For Aeon's ROM and for the `.emp` front end there is none.

> [!finding] 5. Debug info is below the toolchain Sigil replaced (CONFIRMED)
> - The s4 listing has 2,774 label rows, 834 `EQU`s, 6 `PHASE` rows and **no address-to-`file:line` row** (MEASURED, release Sigil, s4 plain). `asl -g`, vasm `-dwarf` and ca65 `.dbg` all write a line table (PUBLISHED, stage 1).
> - Every instruction fragment already carries its source span (`sigil-ir/src/lib.rs:29-36`) and `SourceIndex::locate` turns a span into `file:line:col` (`sigil-frontend-emp/src/resolve/manifest.rs:178-182`). The listing writes one synthetic line per symbol so Oracle's AS parser accepts it (`sigil-link/src/listing.rs:157-162`). A line table is a serialisation job. One caveat: comptime-generated code carries the template's span, not the call site's (`eval/asm.rs:281-293`).
> - **No Z80 driver symbol reaches the listing.** `SndDrv_SetBank`, `Psg_NoteOn`, `Seq_Op_Dac`: 0 hits (MEASURED). The driver enters the ROM as a blob, so Oracle gets no names for 6,228 bytes of Z80 code. The `PHASE` rows carry no address space (`listing.rs:198`).

> [!finding] 6. Aeon is compiled into the assembler; a second game cannot be built (CONFIRMED)
> - `sigil build --game thunder` → `unknown --game 'thunder' (want sonic4 or demo)`, exit 2 (MEASURED).
> - Rust holds: game profiles (`native.rs:309-601`), 108 section-alignment rows (`section_align.rs`), 23 shape-gate module ids where **no match means "build into every shape"** (`module_roots.rs:129-165`, `:186-188`), the sound driver's five files, order and load address (`seam1.rs:131-151`), 399 driver constant names and 26 handler names (`seam1.rs:1426-1560`, `:84`), the bank chain (`seam2.rs:63-76`), 39 contract-baseline rows naming Aeon procedures (`contract_baseline.rs:137-146`), and a 12-module helper list where "the later helper silently wins" a duplicate name (`native.rs:1802`, `:1819`).
> - An engine refactor can therefore need a Sigil commit, a rebuilt binary and a re-freeze. `aeon/build.sh:652-660` records the shared binary once sitting three days stale.
> - The language and linker are not the obstacle: reviewer B linked a new two-module Z80 driver with banked data through `sigil emp --root --map`, no seam, correct bytes by hand (MEASURED).
> - The production driver lives in the test-harness crate: `sigil-cli` depends on `sigil-harness`, and 558 of that crate's `pub` items are used only by tests.

> [!finding] 7. Build cost: quadratic in procedures per module, at least four corpus passes, one thread, no incremental path (CONFIRMED)
> - **Today:** s4 plain builds in 2.92 s (median of 5, release Sigil, aeon `1ee78b88`, load 10 to 12), 119 MB peak. Reviewer A: 2.87 s; reviewer B: 2.74 s (one run), both at aeon HEAD. With a DEBUG-profile Sigil: 10.99 s. Touching one `.emp` file costs the same 2.85 to 2.97 s. user/wall is 0.89 on a 16-thread machine. All MEASURED.
> - **Quadratic:** an `Evaluator` is built per procedure, twice (`eval/mod.rs:2070`, `:2088`), and each one re-indexes the whole file (`:848-849`) on a newly spawned 64 MiB-stack thread (`:1916-1940`). One module of 4k / 8k / 16k procedures with no calls: 1.73 / 6.86 / 28.5 s. The same 16,000 procedures as 16 modules: 2.10 s (MEASURED, release Sigil, `sigil emp`, synthetic). Aeon's largest module has 55 procedures, so this is latent. A generated module or a compiled tier hits it at once.
> - **Passes:** the contract gate re-evaluates every procedure body (0.43 s of 2.87 s, 15%; MEASURED by A, median of 5 with and without `SIGIL_CONTRACTS=0`); the sound seam lowers the sound modules again; the 899 residual `.asm` lines cost a whole extra program build to harvest RAM addresses; then the real build (B, 18 coarse stack samples under `gdb`; the count of passes is also read from `native.rs:3833-3841`).
> - Link, emit, appendix and listing together are about 0.2 s, 7% (ESTIMATED by subtraction, A).

> [!finding] 8. The one build-stopping contract error has no location, and most warnings are noise (CONFIRMED, reproduced)
> An injected undeclared clobber prints `[proc.clobber-undeclared] closure firings, 1 firing(s); this family is zero-firing by contract.` and names no procedure, register or line (`sigil-cli/src/main.rs:2236-2248`; MEASURED by A and again by B). Meanwhile each s4 build prints 170 warnings at aeon HEAD, 63 of them a per-file clobber lint the closure finds clean (MEASURED, release Sigil, s4 plain). `sigil report contracts` already lists firings by procedure and register; the error does not say so.

> [!finding] 9. The build is not hermetic, and `--check` writes into the source tree (CONFIRMED)
> `sigil build`, including `--check`, rewrites 15 files under `aeon/engine/sound/generated/` on every run (`native.rs:3833-3836`, `:839-857`; MEASURED by stage 2). `build.sh` has already emitted them once (0.33 s each time, MEASURED, release). The crash appendix shells out to `aeon/tools/convsym` (`native.rs:4809-4868`). The listing embeds the output path. Two builds of different shapes in one tree race on the sound files, and the toolchain cannot run as a library inside an editor.

> [!finding] 10. The shipped placement wraps the proven loop in an unproven one (CONFIRMED by reading; no failure reproduced)
> `packed_true_bases` (`native.rs:2672-2844`) alternates measure and pack over whole-program layouts. It is not grow-only, stops at a literal 8 rounds with "relaxation oscillation, hand ruling needed" (`:2836-2844`), and sets spans by a heuristic (`:2959`). Every failure is loud, so on its own this stops builds; it does not corrupt them. Near a full cart it combines with finding 1.

> [!finding] 11. The crash appendix is never checked against 4 MB (CONFIRMED by reading; not reproduced)
> `append_deb2_appendix` appends past `EndOfRom` and checks a size band of `min..=0x10000` (`native.rs:4880-4917`), never `len <= 0x400000`. At about three times today's symbols the band itself refuses (ESTIMATED: about 90 KB against 64 KB).

> [!finding] 12. Smaller, each with evidence
> - **The contract gate analyses a differently resolved program** (PLAUSIBLE, no divergence reproduced). It uses a flat last-writer-wins name list (`corpus_contracts.rs:452-460`, `:1943-1968`), not the build's `use` imports. Aeon declares 49 to 50 `const` names more than once (MEASURED by grep). Related to [[EMP language]] finding 2.
> - **`.emp` Z80 cannot write `ld a, (Label)`** (`eval/asm.rs:2470-2484`; MEASURED). The AS route can. Aeon's driver works around it at 17 T-states and a lost `hl` against 13 (PUBLISHED Zilog timings). No self-modifying operand either.
> - **Cycle budgets are used on 1 of 607 procedures** (MEASURED) because any loop or call is unbounded (`cycle_budget.rs:27-31`). Owned by [[EMP language]] plan item 6.
> - **Diagnostics form:** `Diagnostic` is level, message, one span (`sigil-span/src/lib.rs:425-431`). No excerpt, caret, secondary span or help; mangled names leak (`$m$f$far`); a comptime error prints twice and cascades (MEASURED samples, stage 2 §5.8). Content is good: messages name the numbers.
> - **Instructions are strings above the encoder** (`value.rs:465-492`); the Z80 cost table sits in the front end, not in `sigil-isa` (`z80_cycles.rs:237`), so the AS route and a compiled tier cannot use it.
> - **Five `clownlzss` decompressors ignore the input length** (`shim.cpp`, `(void)data_size;` 8 times): an out-of-bounds read from a safe function, reachable only on the AS route's round-trip check (A, by reading).
> - **AS route:** with `ixl equ 5`, `ld a,ixl` assembles to `3E 05` in silence (MEASURED). Low: Aeon has no AS-route Z80 left.
> - **`sigil emp` skips the closure tier and does not say so** (MEASURED).
> - **Stale docs:** `golden/PROVENANCE.md` describes two gates deleted on 2026-07-30; `build.sh:37-48` publishes 1.15 s for a build that now measures 2.74 to 2.92 s; `docs/superpowers/` is 426,645 lines against 315,996 lines of Rust.

## Reviewer agreement

Reviewer A was Opus, reviewer B was Fable; B never read A.

| Topic | Reviewer A | Reviewer B | My call |
|---|---|---|---|
| Scale | On par with the best common tools overall | Same | Agreed. Per-axis reading in the verdict box |
| Disposition | Keep and patch the core; rewrite the driver beside | Keep the core, patch `frontend-emp`, rewrite the driver beside | Agreed |
| Immediate truncation | F1, same two comments | Finding 1, same two comments | Agreed. B's split is right: the const annotation is a separate defect |
| Quadratic cause | `index_items` per evaluator | Same, plus two evaluators per procedure | Agreed; B's is the fuller cause |
| Line table | A serialisation job | Same | Agreed. B adds the Z80 symbols and the address space |
| Compiled-tier seam | `CodeBuf`, not `sigil-ir` | Same; extract `CodeBuf` and its analyses into a crate first | Agreed. The extraction waits for a tier decision (decision 4) |
| Region overflow | First round: listed containment as a safe edge | First round: "no exceeded budget produces a ROM" | **Both wrong**, corrected in the checker round. B's reproduction is the evidence |
| Relaxation | Proven inner loop; the shipped wrapper is below | "Best known" | A. B did not read the wrapper; the checker confirmed A's three citations |
| Cycle budgets as a lead | Counted under "beyond" | Used once; cannot cross a loop or a call | B. I do not count cycle budgets as a lead today |
| Contract verification | Beyond | Beyond for 68000; a lint for Z80 | B, by reproduction |
| Speed | "Below asl but adequate"; target of 3 s met | Target of 1 s not met; "below what the design allows" | Both facts hold. I take B's target: four passes and one thread are waste, not a floor |
| Encoding correctness | On par, below asl on immediates until fixed | Best known | A's wording, until finding 2 is fixed |
| CI | "Says so honestly"; then F10 in the checker round | Not examined | Red on master (MEASURED) |
| Cost of the region fix | Hours to a day | 2 to 3 days | B's: it includes every region kind, any number of budgets, and deriving the seam-2 sizes |
| Port gates | Not addressed | Retire with the driver rewrite | B, after the golden moves to the aeon side |

Found by one reviewer only. A: the differently resolved gate, the unproven placement loop, the appendix check, the double sound emit, the FFI read, the location-free error's history. B: four passes, `--check` writes, the const annotation, bit numbers, the Z80 gate, `ld a,(Label)`, Z80 symbols, the shape-gate table, the stringly `CodeBuf`, the unused cycle budget.

## Checker corrections

44 citations re-opened: **40 hold, 2 mislabelled, 2 wrong**. Neither wrong claim changes a verdict; both changed a correctness statement.

| Correction | Effect |
|---|---|
| Both reviewers missed the open region-overflow row; B stated the opposite | Became finding 1. Both corrected their files; B reproduced it |
| Both missed that CI is red on master | Became finding 4. Stage 0 had recorded CI history as "not checked" |
| Neither reviewer had read the Z80 side or the FFI crates | B read the Z80 encoder, back end, cost table and seam headers: findings 3, 5 (Z80 half) and three bullets of 12. A read the three FFI crates and `sigil-s4lz`: one unsound class, the rest sound |
| Stage 2: "link, emit and listing about 21%" | **Wrong.** `--check` also skips the contract gate. About 7% (ESTIMATED, A) |
| Stage 2 conclusion 6 quotes one scaling series | **Mislabelled**: it mixes two generators. The clean 4x per doubling holds for the no-call series (1.73 / 6.86 / 28.5 s). The series with calls is 1.56 s at 4k, 45.6 s at 16k, 122.1 s at 32k |
| Stage 2: "`const W: u16 = $12345` accepted and truncated" | **Wrong wording.** The constant keeps its full value; the annotation is ignored |
| Stage 2 §5.8: "contract enforcement weak" | True of `sigil emp` only. `sigil build` exits 1 on a 68000 firing |
| A cited `eval.rs:2440-2470` for the AS immediate check | Mislabelled: the right lines are `:3333-3338`. The claim holds |
| A's region-containment edge note (`map.rs:135-160`) | Wrong as a safety claim; withdrawn |
| Stage 0 §8: `corpus_contracts.rs` holds Aeon-specific contracts | Wrong: it is generic. The Aeon rows are in `contract_baseline.rs` |
| Stages 0 and 1: "no independent whole-ROM oracle"; "AS front end fitted to Aeon" | Too broad. True for Aeon and `.emp`. The AS route builds stock Sonic 1 and 2 against a real `asl` (MEASURED); S3K identity is PUBLISHED only (`docs/QUEUE.md:826-828`), with no gated test |
| Stage 1 uses 1.15 s as "today" and calls build speed "about at target" | Stale: 2.74 to 2.92 s MEASURED |
| Stage 1: relaxation has "a proven pass cap" | Inner loop only (finding 10) |
| The checker's "19 `-Wmaybe-uninitialized` warnings" | 19 output lines of 2 warnings at one site, a false positive behind a Rust guard (A, by reading) |
| The location-free error "possibly regressed" from the August fix | Not a regression: that family never printed a location. The August closeout line overstates its fix |

Scope note: the task said 13 crates; there are 14 (stage 0). The "4 MB Thunder Force IV-scale" workload is a ceiling test of the flat window. Reviewer B recalls the retail cartridge as 1 MB, from memory, not checked.

## Plan

Costs are the reviewers' estimates (ESTIMATED). "Bytes" says whether the ROM changes. Items 1 to 7 are the "fix between waves" set.

| # | Improvement | Gain | Cost | Risk | Bytes |
|---|---|---|---|---|---|
| 1 | **Enforce region sizes** (finding 1): validate against the placed-into region, for every region kind; any number of `[[budget]]` rows; derive the seam-2 sizes; invert the four `gap_*` tests | Closes the reproduced wrong-ROM path; makes per-bank budgets real | 2 to 3 days | Low. `sound_tables_z80` is over its literal today, so derive that size in the same change | No, unless a live overflow exists |
| 2 | **Range-check `.emp` immediates, enforce typed consts, warn on bit numbers** (finding 2), then sweep the corpus. The same job as [[EMP language]] plan item 3 | Closes the other wrong-ROM path | 1 day plus the sweep | Low. It may flag existing lines; that list is the point | No, unless the sweep finds a real bug |
| 3 | **Run the clobber closure over Z80 modules in the build gate** (finding 3), exclusions declared in source | Z80 code gets the same "no" as 68000 code | 2 days | Low | No |
| 4 | **Make CI green and watched** (finding 4): one shared guard for every reference read; a CI-shaped run in the landing bar; the overseer loop reads `gh run list` | The one run that can see this class is trusted again | Hours | Low | No |
| 5 | **Locate the zero-firing error** (finding 8): print procedure and register now, add a span next; drop the per-file clobber warning when the closure runs | The build-stopping error becomes actionable; 63 noise lines go | Minutes, then hours | Low | No |
| 6 | **Appendix checked against the cart window; bounded `clownlzss` decompress with `catch (...)`** (findings 11, 12) | TF4-scale carts keep crash reports; removes the one unsound safe function | Hours each | Low | No |
| 7 | **A front-end differential gate**: random instruction × operand × size through `.emp` and through the AS route, compared, plus Capstone | Would have caught finding 2; guards every future operand form | Days | Low | No |
| 8 | **Line table + Z80 symbols + an address-space column**, as a sidecar Oracle reads (finding 5) | Source-level view and stepping; names for the Z80 driver; parity with `asl -g` and ca65 | 3 to 5 days across Sigil and Oracle, plus 2 days for the Z80 half | Low. The address space needs a section attribute, which is language surface | No |
| 9 | **One front-end pass per build; index a module once; one eval thread; run the closure over the build's own `CodeBuf`s** (findings 7, 12) | Removes the quadratic; about 1.5x to 3x faster (ESTIMATED, B); one program analysed, not two | About 1 week | Medium: contract baselines may shift; goldens guard the bytes | No |
| 10 | **Rewrite the driver beside** (findings 6, 9, 10): a project file in the game repo (shapes, defines, prelude, alignment as section attributes, baseline); the sound driver as plain sections (needs link-time `(Label)` operands and a load-address built-in), then delete seams 1 and 2; placement in `sigil-link` with a grow-only proof; no writes into the source tree; a native appendix | A second game and a second sound driver with no Rust edits; hermetic builds; the tool can run as a library | 2 to 3 weeks (B) / "weeks" (A) | Medium, held by byte identity against the old driver across the 7 shapes | No, by proof |
| 11 | **Move the golden to the aeon side at aeon HEAD; retire the port gates and landing scaffolding** | "Green" means today's ROM; engine edits stop being toolchain work | 2 days, after 10 | Low | No |
| 12 | **Numeric closure**: call-inclusive cycle and stack budgets, declared loop bounds. [[EMP language]] plan item 6 | Whole-frame budget proofs | 2 weeks | Medium | No |
| 13 | **Structured diagnostics**: code field, secondary span, excerpt, JSON, a generated catalogue | Readable errors; the base for an editor | 1 to 2 weeks | Low | No |
| 14 | **Extract `CodeBuf` and its analyses into a crate, typed mnemonics, Z80 cost table into `sigil-isa`** | The seam a compiled tier needs | 2 weeks | Medium | No |
| 15 | Parallel per-module lowering; a content-keyed per-module memo | Toward a sub-second edit loop and a language server | Weeks | Medium (determinism; a stale cache is "a wrong ROM that passes", `resolve/mod.rs:80-86`) | No |

Dropped, with the reason: LLVM or Cranelift as a back end (no Z80 target; M68k experimental, PUBLISHED); salsa now (fix the redundant passes first); object files and separate linking (one flat image, whole-program checks); a mapper above 4 MB (no game needs it); S.C.E. compatibility (120 errors, MEASURED; not on the engine's path); tidying `frontend-as/src/eval.rs` (gated by real `asl`, nearly unused by Aeon, pure risk).

## Beyond state of the art

| Candidate | Plausibility | Needs |
|---|---|---|
| Compiler-generated code checked against the same contracts as hand-written code, on every build. `ItemAuthor` already routes obligations by author (`value.rs:337-345`) | High. No surveyed toolchain does it | Items 9, 14 |
| Oracle showing the declared contract (clobbers, requires, held context) and per-line cycle cost at a breakpoint | High: the data is already computed | Item 8 |
| A machine-checked worst-case VBlank cost against the frame budget | Medium: loop bounds must be declared and the DMA stall modelled | Item 12 |
| Incremental relink under a section firewall, well under 100 ms | Medium; only worth it late | Items 9, 15 |

## Needs a live run or the owner's machine

| Item | Why it is open |
|---|---|
| The 3 `landing_verdict` tests | DID NOT RUN validly: the script refuses a tmpfs target dir. Needs one run with the target on disk |
| The canonical `aeon/build.sh` with every lane | BLOCKED: 12 `MissingSuitePath` errors; the lanes need other suite checkouts and could write into live trees. Only `FAST=1` builds ran |
| A phase-level profile of `sigil build` | BLOCKED: `perf` cannot open events (paranoid=2) and Sigil has no phase timers. The pass split in finding 7 is 18 coarse samples plus one A/B |
| Quiet-machine timings, and why 1.15 s (PUBLISHED, August) became 2.74 to 2.92 s | Every timing here was taken at load 9 to 27. The 2.5x gap is unexplained |
| First run of the immediate check over the corpus (item 2) | Unknown whether any Aeon line relies on truncation |
| Does any live region overflow exist once sizes are enforced (item 1)? | One is known (`sound_tables_z80` against its literal). Bank 1's real headroom was not measured |
| A byte gate for aeon HEAD's s4 | None exists (finding 4) |
| Findings 10, 11 and the differently-resolved gate | Read, not reproduced. Each has a failing-first test written in reviewer A's report |
| The Z80 cost model has no term for a banked-ROM read delay | Flagged by B; not measured. Belongs to the sound-driver audit |
| What real `asl` emits for `ld a,ixl` under `cpu z80undoc`; the retail TF4 cart size | Both from reviewer B's memory |
| Cross-machine reproducibility | Untested; determinism was shown on one machine |
| What Oracle's loader must change to read a line table | Not examined here; belongs with item 8 and [[Oracle fitness]] |
| Still unread: bodies of `z80_preserves.rs` and `z80_out_verify.rs`, the DAC/SFX/MT emitters in `seam2.rs`, `relax.rs:80-880`, `blob.rs`, `code_file.rs`, `asl_mode.rs`, `sigil-cli/build.rs`, `scripts/`, `tools/`, the four editor-binding sites, the row `WARN-TIER-UNPINNED-PREFIXES` | No reviewer judgment exists for these. The Z80 analyses' transfer functions are the ones that matter for the named workload |
| SIGINT sent to processes named `sigil` during reviewer A's runs | Unexplained; A worked around it with a renamed binary. Probably another session |

Cut: the 56 untracked landing transcripts at the Sigil repo root (24 MB) are housekeeping, listed in stage 0 §9; they need no run.

## Decisions for you

1. **Land plan items 1 to 6 before wave 2 audits the tree**, accepting that the immediate sweep and the region check may flag existing Aeon lines.
   **Recommended: yes.** They are days of work, and every later audit trusts the build's "no". Item 2 is the same change as decision 2 on [[EMP language]].
2. **Build the line table and Z80 symbols now, as a plain sidecar file Oracle reads** (item 8), ahead of the engine audits. ELF/DWARF for gdb is a later export from the same data.
   **Recommended: yes.** Wave 2's runtime work needs to name source lines and Z80 routines, and the data is already in the IR.
3. **Rewrite the build driver beside the old one, with the game's data in the game repo** (items 10 and 11), and retire the port gates and landing scaffolding when it matches byte for byte.
   **Recommended: yes, starting after decision 1.** It does not depend on the language direction: the core survives any language change. It is the only route to a second game or a different sound driver.
4. **Do the cost fix now and hold the tier seam.** Do item 9 (one pass, no quadratic) now. Hold item 14 (`CodeBuf` extraction) until a choreography tier is approved under [[EMP language]] decision 3.
   **Recommended: yes.** Item 9 is needed for generated modules whatever the language does. Item 14 serves only a tier nobody has approved.
5. **Defer the editor path**: structured diagnostics, parallel lowering, incremental builds and a language server (items 13, 15), and a mapper above 4 MB.
   **Recommended: yes, defer.** A 2.9 s full build (release Sigil, s4 plain) is not the bottleneck, and each of these is safer after the driver is hermetic.

## Reports

- [[sigil-assembler/0-inventory|0 Inventory]]
- [[sigil-assembler/1-research|1 Research]]
- [[sigil-assembler/2-measurements|2 Measurement]]
- [[sigil-assembler/3-reviewer-a|3 Reviewer A]]
- [[sigil-assembler/4-reviewer-b|4 Reviewer B]]
- [[sigil-assembler/5-checker|5 Checker]]
