---
kind: audit-stage
system: sigil-assembler
stage: "2 Measurement"
agent: Opus
status: done
description: "Byte-exact and deterministic (7 of 7 shapes match the goldens at the pinned aeon rev); strict suite 5,864 passed / 3 failed (environmental) / 3 ignored; s4 builds in 2.9 s single-threaded with no incremental path; the .emp front end silently truncates out-of-range immediates and typed consts, and cost is quadratic in procs per module."
---
# Sigil assembler: measurement

Sigil `7630ba0d` (clean sources). Aeon measured at two revisions: **`1ee78b88`**, the reference revision the newest golden freeze names (`golden/provenance.toml`, entry `pin-advance-s2`), and **`936439fa`** (aeon HEAD, 63 commits later). Collected 2026-11-16, 13:26 to 13:47 local.

**Machine context for every timing below:** AMD Ryzen 7 9800X3D, 8 cores / 16 threads, 60 GB RAM; cargo/rustc 1.99.1; all builds and target dirs on tmpfs (`/tmp`). **The machine was shared**: other audit agents were running cargo test suites at the same time, and the 1-minute load average ran from 9 to 27 during these measurements (recorded beside each run). Treat every wall-clock figure as an upper bound under load, not a quiet-machine number. `/usr/bin/time` is not installed, so wall, user, sys and peak RSS come from a small `os.wait4` wrapper (`scratchpad/tools/vtime.py`). Its peak RSS is the largest single process in the tree, not a sum. "Release" means the cargo `release` profile of Sigil itself. "s4" or "s4.debug" names the ROM shape that Sigil builds (Aeon's `DEBUG=1`).

## Conclusion

> [!verdict] Correct and reproducible on what it is gated for, slow-ish and non-incremental, and with one real correctness hole in the `.emp` front end
> 1. **Byte identity holds.** At the pinned aeon revision, all 7 shapes (s4, s4.debug, demo, demo.debug, config_a, config_b, lean) are byte-identical to the goldens in `sigil-harness/golden/`. MEASURED. The goldens are Sigil's own output. The suite's independent oracles (a real, md5-pinned `asl` from `s1disasm`, building stock Sonic 1 and Sonic 2 whole) also pass. MEASURED.
> 2. **Determinism holds.** 5 builds of each of the 7 shapes gave 1 distinct ROM hash per shape. Listings differ only on the line that records the output path; built to the same path, the listing hashes are identical too. A build of aeon HEAD in a scratch copy reproduces, byte for byte, the `s4.bin` that already sits in the live aeon tree. MEASURED.
> 3. **The strict full suite: 5,864 passed, 3 failed, 3 ignored, across 521 test binaries, in 534 s wall** (release, load 13 to 27). The 3 failures are environmental: `landing-run.sh` refuses to run under tmpfs, and this audit's rules require tmpfs. Totals per crate match stage 0's listed count exactly (5,870). MEASURED.
> 4. **Build speed:** `sigil build` for s4 takes **2.92 s** (median of 5, release Sigil), is single-threaded (user time is about 0.9 of wall), and peaks at **119 MB**. demo takes **1.01 s**. **There is no incremental path**: touching one `.emp` file or adding a comment to it costs the same 2.85 to 2.97 s as a full build. A clean release build of Sigil takes **22 s**; compiling the test suite takes **60 s** more. MEASURED.
> 5. **New correctness defect (MEASURED):** the `.emp` front end **silently truncates out-of-range values**. `move.w #$12345, d0` assembles to `303c 2345`, `move.b #300, d1` to `123c 002c`, and `const W: u16 = $12345` / `const NEG: u8 = -1` are accepted and truncated. All exit 0 with no diagnostic. The AS front end rejects the same two instructions (`operand 74565 out of range -32768..=65535`). The `.emp` `data` path *does* check (`[emit.out-of-range] 511 does not fit u8`). So the check exists and is not applied to instruction immediates or typed consts.
> 6. **Scaling defect (MEASURED):** compile time is **quadratic in the number of procs in one module** (doubling procs costs 4x: 4k procs 1.6 s, 8k 6.9 s, 16k 28 s, 32k 122 s). It is linear in instructions per proc and linear in module count (16 modules of 1,000 procs: 2.1 s). Aeon's largest module has 55 procs, so this does not bite today. A generated or compiled tier that emits many small procs into one module would hit it.
> 7. **Diagnostics are good in content and weak in form.** Messages are specific and name the numbers (`worst-case path costs 52 cycles, over the declared budget of 30`; `SLOTS (33) must be a multiple of 8`). But they carry no source excerpt or caret, no secondary label, no help line, and they leak internals (mangled `$m$f$far`, `fixup in section text at offset 2`). Comptime errors are reported twice and cascade. A clobber-contract violation is a **warning, and the build exits 0**. The s4 build itself prints 167 warnings.
> 8. **Debug info for Oracle is symbols only** (confirmed): the s4 listing has 2,774 label rows, 834 `EQU`s, 6 `PHASE` rows and a 314-file Source Digest, and **no source file:line↔address mapping**.

## 5. Measured cost (toolchain form)

The template's fixed scenario legs (idle, run, spindash and so on) are for engine systems. For a toolchain, the equivalent fixed legs are: build Sigil, test Sigil, build each ROM shape, rebuild after one edit, determinism, peak memory, scaling, and diagnostics. All legs below ran. Two side items were **BLOCKED** and are listed at the end.

### 5.1 Building Sigil itself

All builds use `--locked --workspace`, default `-j` (16), from a `git clone --no-hardlinks` of `7630ba0d` in the scratch suite root.

| Leg | Profile | Wall | user / sys | Peak RSS (largest process) | Load at start | Label |
|---|---|---|---|---|---|---|
| Clean build, workspace libs and bins (first run, live checkout) | release | 22.44 s | 138.6 s / 6.3 s | 1,003 MB | 10.8 | MEASURED |
| Clean build, workspace (clone) | release | **22.11 s** | 137.7 s / 6.0 s | 1,017 MB | 9.3 | MEASURED |
| No-op rebuild | release | 0.03 s | | | | MEASURED |
| Rebuild after `touch crates/sigil-frontend-emp/src/lower/mod.rs` | release | **16.46 s** | 87.2 s / 2.5 s | 1,007 MB | ~16 | MEASURED |
| Compile all test binaries (`cargo test --no-run`, after the build) | release | **60.36 s** | 536.8 s / 35.2 s | 669 MB | ~16 | MEASURED |
| Clean build, workspace | DEBUG (`dev`) | **12.69 s** | 47.5 s / 5.2 s | 991 MB | 22.4 | MEASURED |
| Target dir after release build + tests | | 2.1 GB | | | | MEASURED |

The release build prints **19 warnings** from the vendored C++ (`-Wmaybe-uninitialized` via `clownlzss-sys/src/shim.cpp:114`), consistent with stage 0. Touching one file in `frontend-emp` rebuilds `frontend-emp`, `harness` and `cli` (16 s). This edit-compile loop is what a Sigil developer pays.

### 5.2 The full test suite (strict)

Command (from the scratch clone, mirroring `scripts/landing-run.sh` minus its clippy and ledger spans):

```
SIGIL_STRICT_GATE=1 AEON_DIR=<suite-ref>/aeon ORACLE_DIR=<suite-ref>/oracle-old EMPYREAN_SUITE_ROOT=<suite-ref> \
SIGIL_BUILD=<target>/release/sigil SIGIL_EMIT=<target>/release/emit_sound_blob TMPDIR=<scratch>/tmp \
env -u SIGIL_ALLOW_PARTIAL cargo test --release --locked --no-fail-fast --workspace -- --nocapture
```

The scratch suite root `audit-builds/suite-ref/` holds: `aeon` (a no-hardlinks clone at `1ee78b88`, all four canonical ROMs pre-built and checked byte-identical to the goldens), `empyrean` (`git archive HEAD`), `oracle-old/linux-port/gui` (`git archive HEAD` of `1eb09a9`, the only path the suite reads), and no-hardlinks clones of `s1disasm` (`f6ece65`) and `s2disasm` (`e45ebf3`). These two are needed: the strict suite builds stock Sonic 1 and Sonic 2 whole with Sigil's `asl` drop-in mode and compares the result against the md5-pinned real `asl` from `s1disasm/build_tools`.

**Result: CARGO_EXIT=101. 521 binaries launched, 521 reported a `test result:` line, 0 filtered out.** Wall **534.0 s** (user 621.7 s, sys 73.4 s), started at load 13.0 and finished at 27.1. MEASURED, from `audit-builds/sigil/test-full.log`, aggregated by `audit-builds/agg2.py` (every `test result:` line, mapped to its package through `cargo metadata`; duplicated binary names were resolved by position in the log).

| Crate | Passed | Failed | Ignored | Binaries (incl. doctest) |
|---|---|---|---|---|
| sigil-backend-m68k | 19 | 0 | 0 | 2 |
| sigil-backend-z80 | 6 | 0 | 0 | 2 |
| sigil-cli | 899 | 0 | 1 | 183 |
| sigil-clownlzss-sys | 47 | 0 | 0 | 6 |
| sigil-clownnemesis-sys | 10 | 0 | 0 | 4 |
| sigil-frontend-as | 1,082 | 0 | 0 | 93 |
| sigil-frontend-emp | 2,784 | 0 | 0 | 136 |
| sigil-harness | 595 | **3** | 2 | 64 |
| sigil-ir | 52 | 0 | 0 | 2 |
| sigil-isa | 150 | 0 | 0 | 16 |
| sigil-link | 173 | 0 | 0 | 4 |
| sigil-s4lz | 24 | 0 | 0 | 3 |
| sigil-salvador-sys | 2 | 0 | 0 | 3 |
| sigil-span | 21 | 0 | 0 | 3 |
| **Total** | **5,864** | **3** | **3** | **521** |

5,864 + 3 + 3 = 5,870, the count stage 0 listed. Every per-crate figure matches stage 0's `--list` column. The landing baseline in the last recorded run was 5,846 (PUBLISHED, stage 0 §6). The 18 extra passes here plausibly come from the `s1disasm`/`s2disasm` gates and from tests added after that run. ESTIMATED; not reconciled test by test.

> [!finding] The 3 failures are environmental, and the cause is this audit's tmpfs rule, not Sigil's code
> All three are in `sigil-harness/tests/landing_verdict.rs` (`a_green_run_records_its_verdict_…`, `a_captured_stdout_is_refused_…`, `a_red_run_records_its_verdict_…`). Each panics at `landing_verdict.rs:1046` with `COULD NOT MEASURE: … landing-run: REFUSING, the target directory is /tmp/…. /tmp is tmpfs on this machine and a cargo build there wedges the shell. Put it on disk.` These tests exercise the landing *script*, not the assembler. Re-running them needs a target dir on disk, which the audit's standing rules forbid. They are recorded as FAILED here, never as passes. **Their real status: DID NOT RUN validly.**

**Slowest test binaries** (cargo's own `finished in`): `sigil-cli/lst_source_digest` 43.1 s; `sigil-harness/region_end_contracts` 32.8 s; `sound_fold_heads` 32.5 s; `m68k_roundtrip_stream` 30.6 s; `sigil-cli/native_offcanonical_full` 29.3 s; `extra_entry` 28.5 s. These are whole-ROM gates that each run several full builds at about 3 s per build.

**No silent skips.** The log has no skip lines from reference-dependent tests. The one `SKIPPED` is internal to a Python selftest (`SELFTEST 20 cases, 19 as expected, 1 SKIPPED, 0 MISMATCH`, in the cart-check gate). `skip_marker_lint` reports that 166 skip-announcement sites all carry the canonical marker.

> [!note] Attempt 1, without the disassembly trees, failed loudly, as designed
> The first launch had no `s1disasm`/`s2disasm` in the scratch suite root. Under `SIGIL_STRICT_GATE=1` the AS-corpus gates failed by name (`SIGIL_STRICT_GATE set but no suite root for Sonic 1 built whole by the AS route … holds no s1disasm checkout`) instead of skipping. 9 failed out of the first 401 tests before the run was stopped on purpose to provision them. The partial log is kept at `audit-builds/sigil/test-full.attempt1-unprovisioned.log`. It is not counted above. This confirms that the strict gate does what it claims: a missing oracle is red, not green.

### 5.3 Building the ROMs

Every ROM build ran in scratch copies. `sigil build` writes 15 files under `<aeon>/engine/sound/generated/` on every invocation (MEASURED with `find -newer`; all other paths are untouched). In the git clone those 15 files came out byte-identical to the committed ones (`git status` clean).

**Direct `sigil build` (release Sigil), 5 runs per shape**, at aeon `1ee78b88`, load about 10 to 12:

| Shape | Command flags | Wall, median (min to max) | user (median) | Peak RSS | ROM size | vs golden |
|---|---|---|---|---|---|---|
| s4 | `--game sonic4` | **2.92 s** (2.89 to 2.99) | 2.60 s | 119 MB | 845,398 | identical |
| s4.debug | `--game sonic4 --debug` | **3.01 s** (2.92 to 3.07) | | 122 MB | 865,722 | identical |
| demo | `--game demo` | **1.01 s** (0.96 to 1.05) | | 93 MB | 99,994 | identical |
| demo.debug | `--game demo --debug` | **1.04 s** (0.99 to 1.09) | | 94 MB | 106,981 | identical |
| config_a | `--config-a` | 3.08 s (2.93 to 3.13) | 2.7 s | 122 MB | 866,074 | identical |
| config_b | `--config-b` | 2.33 s (2.30 to 2.39) | | 95 MB | 617,813 | identical |
| lean | `--lean` | 2.89 s (2.88 to 2.97) | | 119 MB | 793,376 | identical |
| s4, `--check` (stops before link and emit) | `--game sonic4 --check` | 2.31 s (2.27 to 2.38), 3 runs | 2.04 s | 114 MB | none | |
| `emit_sound_blob` | `--aeon . --out-dir engine/sound/generated` | 0.34 s, 1 run | | 56 MB | | |
| s4 with a **DEBUG-profile** Sigil | `--game sonic4` | **10.99 s** (10.86 to 11.07), 3 runs | 10.5 s | 128 MB | 845,398 | identical |

All MEASURED. Reading of the figures:
- **Single-threaded.** user/wall is about 0.89. The 16-thread machine is idle during a build.
- **Where the time goes.** `--check` costs 2.31 s of the 2.92 s, so front end, comptime and relaxation take about 79% and link, emit and listing about 21% (ESTIMATED by subtraction). The check reports `89905 ensure verdict(s) at comptime, 696 LinkAssert(s) decided at link`. A finer split was **BLOCKED**: `perf record` cannot open any event on this machine (paranoid=2; `cpu-clock` also refused), and Sigil has no phase timer (`NATIVE_DEBUG` prints only drift tables).
- **The PUBLISHED figure does not reproduce.** `aeon/build.sh:37-48` gives "sigil build (the ROM) 1.15 s" (16 cores, load 7 to 9, 2026-08-19). Measured here: 2.92 s at load about 10. The gap is 2.5x. Load alone is unlikely to explain it, because the build is single-threaded and the CPU has idle cores. Code growth since August is plausible. It is not attributed here. ESTIMATED: unexplained.
- **The debug-profile tool is 3.8x slower** (11.0 s against 2.92 s). That is relevant because CI runs the suite in debug (stage 0 §6).

**Through `aeon/build.sh` with `FAST=1`** (artifact lanes only: emit_sound_blob, compression vectors, the sigil build, checksum, deb2 appendix):

| Shape | Aeon rev | Wall | ROM | Matches |
|---|---|---|---|---|
| s4 | 1ee78b88 | 4.82 s, then 3.66, 3.64, 3.89 s on repeats | 845,398 | golden |
| s4 DEBUG=1 | 1ee78b88 | 3.50 s | 865,722 | golden |
| demo | 1ee78b88 | 1.05 s | 99,994 | golden |
| demo DEBUG=1 | 1ee78b88 | 1.11 s | 106,981 | golden |
| s4 | 936439fa (HEAD) | 3.94 s | 850,296 | **the live tree's `aeon/s4.bin` (sha256 `1d679068…`), not the golden** |
| s4 DEBUG=1 | 936439fa | 3.89 s | 870,560 | the live tree's `aeon/s4.debug.bin` (`41c033bb…`) |
| demo / demo DEBUG=1 | 936439fa | 1.18 / 1.29 s | 99,994 / 106,981 | golden |

> [!finding] The golden pin trails aeon HEAD by 63 commits
> The goldens and the strict suite measure aeon `1ee78b88`. At aeon HEAD `936439fa`, s4 and s4.debug differ from the goldens (the first differing byte is at offset 10, in the vector table; the sizes are 850,296 against 845,398). demo is unchanged. Between the two revisions, `git diff --stat 1ee78b88 HEAD -- engine games` shows 18 files changed (108+/44−), including `games/sonic4/map.toml` and `vram.toml`. So **"the suite is green" says nothing about the ROM aeon ships today**. There is no current byte gate for aeon HEAD's s4; the only check is that Sigil reproduces itself.

**The canonical `build.sh` (all verification lanes): BLOCKED.** Its pytest lane failed at collection with 12 `suite_paths.MissingSuitePath` errors, because it needs other suite checkouts (the emulator and others) that were not provisioned. Linking the live repos would let those lanes run against, and possibly write into, live trees. Those lanes are Aeon's gates, not Sigil. The one Sigil-heavy lane, `emp_expect_fail` (20 real Sigil builds, 22.69 s), is PUBLISHED only (`build.sh:40`).

### 5.4 Incremental rebuild

| Edit | Wall (release Sigil, direct `sigil build`, s4) | ROM |
|---|---|---|
| none (repeat build) | 2.92 s (median above) | identical |
| `touch engine/system/vblank.emp` | 2.85 s | identical |
| append `// audit: trailing comment` to `vblank.emp` | 2.97 s | identical |
| `FAST=1 ./build.sh`: no edit / after `touch vblank.emp` / repeat | 3.66 s / 3.64 s / 3.89 s | identical |

MEASURED. **Every build is a full build.** This matches the deliberate design note at `resolve/mod.rs:80-86` (stage 0). Today the cost is under 3 s, so the design is defensible. It does not stay cheap at 3 to 5 times the source (see 5.6).

### 5.5 Determinism

| Check | Result | Label |
|---|---|---|
| 5 builds × 7 shapes, distinct ROM sha256 per shape | **1** for every shape | MEASURED |
| Same 35 builds, distinct listing hashes | 5 per shape; `diff` shows the **only** difference is the `DIGEST-ROM … path=` line (output path) | MEASURED |
| Two s4 builds to the same output path | ROM `a7a76413…` and listing `f972ce25…` both identical | MEASURED |
| Scratch build of aeon HEAD against the live `aeon/s4.bin` (built 2026-11-13 by another session) | identical sha256 `1d679068eb1180ff…` | MEASURED |
| A DEBUG-profile Sigil against a release-profile Sigil, s4 | identical | MEASURED |
| Generated sound artifacts rewritten in place | byte-identical to the committed files (`git status` clean) | MEASURED |

The listing embedding the output path is a small reproducibility wart. Two identical builds to different paths give different `.lst` files, and an Oracle cache keyed on the listing hash would miss.

### 5.6 Scaling (toward the 4 MB Thunder Force IV workload)

There is no TF4-scale corpus, so synthetic `.emp` was generated (`audit-builds/sigil/scale/gen*.py`) and built with `sigil emp` (release Sigil). Each proc has about 10 instructions (moveq, move, lea abs.l, store, tst, a local `beq.s`, an optional `jsr` to the next proc, rts).

| Shape | n | Source lines | Output bytes | Wall | Peak RSS |
|---|---|---|---|---|---|
| n procs, one module | 1,000 | 12,000 | 23,996 | 0.13 s | 29 MB |
| | 4,000 | 48,000 | 101,264 | 1.56 s | 95 MB |
| | 16,000 | 192,000 | 413,264 | **45.6 s** | 352 MB |
| | 32,000 | 384,000 | 829,264 | **122.1 s** | 696 MB |
| n procs, one module, no `jsr` | 4k / 8k / 16k | | | 1.73 / 6.86 / 28.5 s | 87 / 167 / 326 MB |
| n procs, one module, no local branch | 4k / 8k / 16k | | | 1.56 / 6.33 / 27.0 s | 79 / 149 / 290 MB |
| one proc, n instructions | 32k / 64k / 128k | | | 0.09 / 0.17 / 0.33 s | 104 / 199 / **389 MB** |
| K modules × 1,000 procs (`--root`) | K = 4 / 16 | | 80,002 / 320,002 | 0.54 / **2.10 s** | 87 / 320 MB |

All MEASURED.

> [!finding] Cost is quadratic in procs per module; memory is about 3 KB per instruction
> Doubling procs in one module costs 4.0x to 4.2x, with or without cross-proc calls and local labels, so the cause is per-proc work that scans the module. The same 16,000 procs split over 16 modules take 2.1 s, not 28 s. Aeon's largest module has 55 procs (`engine/sound/sound_sequencer.emp`) and the corpus has 607 in total (MEASURED by grep), so **today this is latent**. It matters for (a) a compiled functional tier that lowers to many small procs per module, and (b) generated tables or code emitted as one module. Separately, peak RSS grows about 3 KB per instruction (389 MB at 128k instructions).

**4 MB extrapolation (ESTIMATED).** Aeon s4 is 845 KB of ROM from about 87K `.emp` lines, and builds in 2.9 s at 119 MB. Build time follows source and comptime work, not ROM bytes (art is `embed`ded). A 4 MB cart that is mostly data, with 2 to 3 times Aeon's code and comptime, would by linear scaling take about **6 to 9 s per build, single-threaded, about 250 to 400 MB**. The estimate holds only if modules stay small. There is no bank or mapper support (stage 0), so 4 MB is the hard ceiling. A **second game** cannot be built yet: `sigil build --game thunder` → `error: unknown --game 'thunder' (want sonic4 or demo)`, exit 2 (MEASURED). Shapes are hard-coded in `native.rs`.

### 5.7 Debug info for Oracle (what the listing carries)

s4 listing, release shape: 6,741 lines. **2,774 label rows** (`(0) <n>/<addr> : <label>:`, where `<n>` is a synthetic sequence number, not a source line), **834 `EQU`**, **6 `PHASE`** (the Z80 sound tables with VMA/LMA), 314 `DIGEST-READ` and 11 `DIGEST-DEFINE`. Local labels appear mangled (`$engine.dma_queue$Drain_Budgeted_Queue$loop`). **No row maps an address to a `.emp` file:line.** MEASURED (`awk` over `audit-builds/direct/s4.r1.lst`). Oracle can name an address. It cannot show the source line or step by source.

### 5.8 Diagnostics sample

Inputs are in `audit-builds/sigil/diag/`, built with `sigil emp <file> -o out.bin` (or `sigil <file.asm>`), release Sigil. Output pasted verbatim.

| # | Case | Input (essence) | Actual output | Exit |
|---|---|---|---|---|
| 1a | **Type error: typed const out of range** | `const LIMIT: u8 = 300` / `move.b #LIMIT, d0` | `built: 6 bytes, wrote out.bin` (bytes `103c 002c 4e75`: 300 truncated to $2C) | **0** |
| 1b | Raw immediates out of range | `move.w #$12345, d0` / `move.b #300, d1` | `built: 10 bytes` (bytes `303c 2345 123c 002c 4e75`) | **0** |
| 1c | Same lines, AS front end | `move.w #$12345,d0` / `move.b #300,d1` | `1f_imm.asm(3):2: error: operand 74565 out of range -32768..=65535` / `1f_imm.asm(4):2: error: operand 300 out of range -128..=255` / `this error list may be incomplete: sigil stopped at the front end, so layout, link and the image checks did not run` | 1 |
| 1d | Type errors in data and comptime | `data table: [u8; 2] = [1, 2, 3]`; `data big: [u8; 1] = [$1FF]`; `twice("hello")` | `1b_type.emp:2:1: array length mismatch: expected 2 element(s), got 3` / `1b_type.emp:3:22: [emit.out-of-range] 511 does not fit u8 (0..=255), in 'big[0]'` / `1b_type.emp:5:12: '*' not defined for string and int` / `1b_type.emp:9:5: instruction dropped, a mnemonic, size, or operand did not resolve (a missing import or type in scope?)` | 1 |
| 1e | `const NEG: u8 = -1`, `const SMALL: i8 = 200` | used in `move.b #NEG, d1` | `built: 6 bytes` (`123c 00ff`) | **0** |
| 2 | **Out-of-range branch** | `bra.s .far` over 100 `nop`s | `2_branch.emp:3:5: bra.s/Bcc.s displacement to '$m$f$far' out of range (200) in section text` | 1 |
| 2b | Same, AS front end | `bra.s Far` over `ds.b 200` | `2b_branch.asm(4):2: error: bra.s/Bcc.s displacement out of range (200) in section sec0` / `this error list may be incomplete: sigil stopped at link, so the image checks did not run` | 1 |
| 3a | **Clobber contract violation** | `proc f() clobbers(d0)` writes `d1` | `3_clobber.emp:4:5: [proc.clobber-undeclared] 'f' writes 'd1', which is not in its 'clobbers(...)' set or parameter list (heuristic lint, full register dataflow is deferred to S2-D6)` / `built: 6 bytes, wrote o.bin` | **0** |
| 3b | `preserves(d2)` violated | `caller … preserves(d2)` sets d2, calls helper | `3b_preserves.emp:8:5: [proc.clobber-undeclared] 'caller' writes 'd2', …` / `3b_preserves.emp:7:1: [proc.preserves-unverifiable] 'caller' declares 'preserves(d2)' but d2 not provably preserved, no '.l' save/restore round-trips its FULL entry value on every return path (individual push/pop, 'movem.l' pair, or '(sp)' peek), or an unmodeled sp op blocks the proof. A '.w' save/restore round-trips only the LOW WORD: if that is what the code guarantees, declare 'preserves(dN.w)'` | 1 |
| 4a | **Cycle budget** | `@budget(cycles: 30)` over moveq + 2 × `move.w d0, (abs).l` + rts | `4b_cycles.emp:2:1: [cycles.over-budget] in 'f': the worst-case path costs 52 cycles, over the declared budget of 30` | 1 |
| 4b | Same, with `move.w d0, $FF0000` (no parentheses) | | `4_cycles.emp:5:5: illegal destination EA: #imm` (×2, lines 5 and 6) / `4_cycles.emp:2:1: [cycles.over-budget] in 'f': the worst-case path costs 36 cycles, over the declared budget of 30` | 1 |
| 5a | **Undefined symbol** | `jsr DoesNotExist` then `move.w #Undefined_Const, d0` | `5_undef.emp:3:5: unresolved jmp/jsr target in section text references symbol 'DoesNotExist' not defined in this link, expected when compiling a cross-seam module standalone; supply the map/harness composition that defines it` (the second undefined name is **not** reported) | 1 |
| 5b | Undefined constant alone | `move.w #Undefined_Const, d0` | `5b_undef.emp:3:5: unresolved symbol 'Undefined_Const' for fixup in section text at offset 2` | 1 |
| 5c | Undefined `jsr`, AS front end | `jsr DoesNotExist` | `7_undef.asm(4):2: error: unresolved jmp/jsr target in section sec0 references symbol 'DoesNotExist' … supply the map/harness composition that defines it` / `this error list may be incomplete: sigil stopped at layout, …` | 1 |
| 6a | **Comptime error** | `comptime fn halve(x) { return x / (x - x) }` | `6_comptime.emp:3:12: division by zero` / `6_comptime.emp:3:12: division by zero` / `6_comptime.emp:8:5: instruction dropped, a mnemonic, size, or operand did not resolve (a missing import or type in scope?)` | 1 |
| 6b | `ensure` fails | `ensure(SLOTS % 8 == 0, "SLOTS ({SLOTS}) must be a multiple of 8")` with SLOTS = 33 | `6b_ensure.emp:6:1: SLOTS (33) must be a multiple of 8` | 1 |
| 6c | Unbounded comptime recursion | `spin(n) = spin(n + 1)` | `6c_loop.emp:3:12: recursion too deep: in ... -> spin -> spin -> … -> spin` / `6c_loop.emp:7:5: instruction dropped, …` (0.002 s) | 1 |

(Backticks inside the messages are shown as `'…'` so the table renders.)

**Judgment** (against rustc, Elm and clang-class diagnostics, and against asl):

| Quality | Verdict | Evidence |
|---|---|---|
| Says the true thing, with numbers | **Good** | 4a, 6b, 1d `[emit.out-of-range]`, 3b (it even explains the `.w` against `.l` subtlety) |
| Catches the error at all | **Fails on value range in `.emp` instructions and typed consts** | 1a, 1b, 1e exit 0 with silently wrong bytes; AS rejects the same lines (1c) |
| Contract enforcement | **Weak** | `clobbers()` violation is a heuristic **warning**, exit 0 (3a). The s4 build carries 63 `proc.clobber-undeclared` warnings among its 167 |
| Location | **Adequate** | `file:line:col` everywhere. The comptime error points into the fn body, not the call site (1d `*` at line 5, not the `twice("hello")` at line 7) |
| Presentation | **Below the state of the art** | No source excerpt, caret, secondary span, `help:` or fix-it. `[code]` sits inside the message text and is missing on most errors (2, 5a, 5b, 6a). No `error:` prefix on `.emp` errors, unlike the AS path |
| Internals leaked | **Poor** | `$m$f$far` (mangled local), `for fixup in section text at offset 2`, `cross-seam module … map/harness composition` (5a/5c: a beginner's typo gets a toolchain-architecture message) |
| Cascades and duplicates | **Poor** | 6a prints the same error twice and then an `instruction dropped` cascade. 4b computes and reports a cycle cost (36) over a body with two rejected instructions |
| Error recovery | **Mixed** | 1d reports four independent errors in one run (good). 5a stops after the first undefined symbol (link-time) |
| Speed | **Good** | single-file diagnostics return in about 2 ms |

## BLOCKED and DID NOT RUN

| Item | Status | Why |
|---|---|---|
| 3 `landing_verdict` tests | FAILED (environmental) | `landing-run.sh` refuses tmpfs targets, and the audit mandates tmpfs scratch |
| Phase-level profile of `sigil build` | BLOCKED | `perf record` cannot open events (paranoid=2, `cpu-clock` refused); Sigil has no phase timers |
| Canonical `aeon/build.sh` with all lanes | BLOCKED | the pytest lane needs suite checkouts that were not provisioned (12 `MissingSuitePath`), and linking live repos risks writes. FAST (artifact-only) builds ran instead |
| Byte gate at aeon HEAD | DID NOT RUN, none exists | the goldens pin `1ee78b88`; HEAD's s4 differs, and nothing independent says which bytes are right |
| Quiet-machine timings | Not available | other audits were loading the machine (load 9 to 27) throughout |

## Commands run

All are read-only on live repos (afterwards: `git status` in `sigil` shows only its 57 pre-existing untracked paths, `aeon` only its pre-existing `docs/audits` untracked paths, and `aeon/s4.bin`'s mtime is unchanged at 2026-11-13). Scratch root: `/tmp/claude-1000/-home-volence-sonic-hacks/f8a970f2-04e1-487a-bbda-d3729df1480b/scratchpad/audit-builds/`.

- `git clone --no-hardlinks` of sigil (`7630ba0d`), aeon (checkout `1ee78b88`), s1disasm and s2disasm into `suite-ref/`; `git archive HEAD` of aeon into `suite/aeon`, of empyrean into both suites, and of `oracle-old` `linux-port/gui`.
- `cargo build --release --locked --workspace [--bins]` (clean, no-op, after a touch); `cargo build --locked --workspace` (dev, clean); `cargo test --release --locked --workspace --no-run`; the strict `cargo test` above (`sigil/run-tests.sh`, log `sigil/test-full.log`, end marker `AUDIT_END_MARKER`).
- `sigil build --aeon . --native <shape flags> -o … --emit-lst …` × 5 per shape; `--check` × 3; `emit_sound_blob`; `FAST=1 [DEBUG=1] ./build.sh [demo]` at both aeon revisions; `cmp` and `sha256sum` against `crates/sigil-harness/golden/*.bin` and the live `aeon/s4*.bin`.
- Synthetic scaling: `python3 -I scale/gen*.py N > …emp; sigil emp … [--root]`.
- Diagnostics: `sigil emp diag/*.emp -o out.bin`; `sigil diag/*.asm -o …`.
- Aggregation: `python3 -I agg2.py sigil/test-full.log meta.json` (`cargo metadata --no-deps`).
