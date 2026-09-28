# AS-PERF-MEASURE: where sigil's time and memory go on the three disassemblies, 2026-09-28

Measurement parcel for queue row `AS-PERF-MEASURE` (project `SIGIL-AS-REPLACEMENT`). It
measures and proposes; it optimises nothing. The only code it lands is the
`SIGIL_PHASE_TIMING` instrument (commits `a9b6e414`, `187eecfc`).

## Summary in plain language

- **Almost all of the time is the front end walking the source, again and again.** On every
  corpus 94 to 98 percent of wall time is the AS front end's passes. Everything after it
  (layout, link, compression, header, write, freeing memory) is 2 to 6 percent together.
- **sigil walks the source more times than asl does.** asl reports 2 passes on all three
  corpora. sigil takes 3 on Sonic 1, 4 on Sonic 2 and 3 on Sonic 3&K, and every pass costs
  about the same (one pass is 24 to 31 percent of the run). That alone explains most of the
  2x to 2.5x gap. Two different causes:
  1. **The convergence test counts changes nobody read.** On S3K the third pass exists only
     because pass 1 ADDED 18 names (all `._headpos`, a `:=` variable that is always assigned
     before it is read); not one value changed. On S2 the fourth pass is the same shape (9
     added `APM_*_Len` set-variables, 3 changed names that are each assigned before they are
     read). A test that asks "did any value this pass READ from the previous pass change?"
     would stop one pass earlier on S2 and S3K.
  2. **sigil's first pass lays some code out differently from its later passes**, so its
     second pass has to correct thousands of addresses (S1: 6286 values move, the first at
     `EEgg_Wait` 0x5a9a to 0x5a9c; S2: 18037 move, the first at `paddingSoFar` 0x84f to
     0x661). asl's first pass evidently does not, since it finishes in 2. Not diagnosed here.
- **Inside a pass, the time is spread over macro handling, not file I/O.** The prior agent's
  read-only suspects are mostly refuted: re-reading and re-splitting include files is 2 to 3.3
  percent, cloning the tables into each pass is under half a percent, the oscillation
  history is under a quarter of a percent and never costs a comparison, and the bonus pass
  never runs on these builds. Lexing is 10 to 11 percent. The large items are inside macro
  expansion: scanning ahead for the `endif` of every `if` in a macro body (6 to 22 percent,
  and it re-substitutes the macro's parameters into each line it scans), macro parameter
  substitution (6 to 17 percent), the per-operand builtin-function scan on every 68000
  instruction (5 to 13 percent), and allocation (a quarter to a third of all samples sit
  under an allocator frame).
- **Peak memory is set by layout, not by the front end.** On all three corpora the process's
  peak resident set is reached in `layout`, because the CLI keeps the front end's whole
  module alive while layout builds a second copy of every section. S3K: front end peaks at
  about 190 MB, layout at 243 to 245 MB. The oscillation history is an estimated 1 to 4 MB.

## Instruments

| what | identity |
|---|---|
| sigil, master | `7051ff7a`, release build in this worktree, md5 `9241feb8fc55446df6468cb7c8cd0cf8` (`sigil-base` below) |
| sigil, instrumented | `a9b6e414`, md5 `ffae469c6fe5da79c6b9e47baea05a9c` (`sigil-inst`); `187eecfc`, md5 `3bb63b5c7179f2f9599938998ad1360d` (`sigil-final`) |
| sigil, profiling | `a9b6e414` built with `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`, md5 `f818b88b26054ea8f6d548a68427dbd6` |
| asl | md5 `61e672562465725a8c102288a7da9098` (S1, S3K); S2's own `0dee1f98e6480a4783d27ffd8b90896f` (the only asl that builds S2, as in the 2026-09-27 note) |
| p2bin | md5 `4f2fff99c3347bafb93b12d5be1db754` |
| s1disasm | `f6ece657c1cf253404312137dfcb8ec15fa42318` |
| s2disasm | `e45ebf332f39987424ca3102e50c717628f71269` |
| skdisasm | `2fcd861c208f342b6d14df694c6422c74f20a4be` (`sonic3k.asm -D Sonic3_Complete=0`) |
| machine | 16 cores, shared, load average 8 to 11 throughout |

Each corpus was extracted with `git -C <repo> archive --output=<tar> <sha>` into a dot-directory
inside this worktree and built once by its own stock script (`build.lua`, `build.lua`,
`buildSK.lua`) to produce its generated inputs and the reference ROM. The reference CRCs
reproduce the 2026-09-27 note: S1 `afe05eee/524288`, S2 `7b905383/1048576`, S3K
`0658f691/2097152`. No disassembly repo was written to.

**Every timed sigil image, all 63 of them (3 binaries, 7 rounds, 3 corpora), is byte-identical
to the stock ROM** (whole-file compare), and so is every asl+p2bin image (the stock script's
later header and driver-size fixes leave these three unchanged).

### The phase instrument

`SIGIL_PHASE_TIMING=1 sigil <file.asm> ...` prints one line per phase to stderr:

```text
SIGIL-PHASE <tab> <name> <tab> wall_ms=.. <tab> hwm_kb=.. <tab> rss_kb=.. [<tab> key=value ...]
```

Top-level phases: `cli.args`, `frontend.read_root`, `passN` (one per front-end pass), `bonus`
(only when the bonus pass runs), `frontend` (the tail after the last pass), `cli.render`,
`layout`, `link`, `bounds`, `flatten`, `header`, `emit`, `teardown`, `total`. Inside each
pass, indented steps: `pass.seed` (new `Asm`, seed-table clones), `pass.exec` (the walk, with
accumulated `include_read`, `split_lines`, `binclude_read` and `lex` time, calls and bytes),
`pass.finish`, `pass.converge_check`, `pass.oscillation_check`, `pass.history_push`. The pass
line carries the environment size, how it differs from the seed (added, removed, changed,
samples, and the changed name with the lowest previous value), and what the oscillation
history holds.

Memory: every MB in this note is MiB (kB / 1024). `hwm_kb` is `VmHWM` since the last top-level phase; each top-level phase restarts the
kernel's peak counter (`/proc/self/clear_refs`, value 5) after printing, so a top-level line's
`hwm_kb` is that phase's own peak. Two consequences for reading the lines: the `frontend` line
covers only the tail after the last pass (the front end's peak is the largest `passN` peak),
and `getrusage` `ru_maxrss` of a timed run reports only the last window, so a timed run's
process peak is the largest `hwm_kb`, not `ru_maxrss`.

Cost when the variable is unset: one `OnceLock` load per call site, no clock read. Measured
as CPU time, master against this branch, 12 interleaved rounds per corpus, order alternating,
04:34 to 04:36 local, load 9.3 to 10.9: median ratio 0.994 (S1), 1.010 (S2), 0.978 (S3K), per
round 0.94 to 1.06. **No cost is resolvable at this load**; anything under about 5 percent is
below the noise.

Cost when set: `total` rises by 0.04 to 0.11 s against the unset run (medians below), almost
all of it the two clock reads around each of the 1.2 to 2.7 million `lex` calls (about 40 ns a
call). So `lex_ms` is an upper bound; the sampling profile below gives the unperturbed share.

### The sampling profile

No `perf` here, but `gdb` and `eu-stack` are installed. A small driver starts sigil with
`prctl(PR_SET_PTRACER, PR_SET_PTRACER_ANY)` in the child before `exec` (Yama
`ptrace_scope` is 1), then takes a raw-address stack with `eu-stack -q -p <pid>` every 5 ms
until it exits, and symbolises offline with `eu-addr2line -f -i -C` against the line-table
build, so inlined frames are named. Shares are fractions of samples; each sample pauses the
process briefly, which stretches wall time but not the proportions. Samples: S1 793 (4
builds, 04:27:58, load 8.9), S2 1556 (3 builds, 04:28:12, load 9.5), S3K 1258 (3 builds,
04:28:27, load 10.0). libc has no symbols for its internal functions, so libc leaf samples
are attributed to their nearest sigil caller.

## Tables

### Wall time and memory against asl + p2bin

7 interleaved rounds per corpus (asl, p2bin, sigil-base, sigil-inst unset, sigil-inst set),
2026-09-28 04:24:18 to 04:26:43 local (08:24:18Z start). Ranges are over the 7 rounds.

| corpus | load (1 min) | asl + p2bin wall | sigil master wall | ratio per round | asl peak RSS | sigil peak RSS |
|---|---|---|---|---|---|---|
| S1 | 8.2 to 8.9 | 0.52 to 0.60 s | 0.98 to 1.10 s | 1.83 to 1.92 (med 1.89) | 15 to 16 MB | 69 to 72 MB |
| S2 | 8.1 to 9.4 | 1.03 to 1.17 s | 2.59 to 2.71 s | 2.30 to 2.56 (med 2.47) | 16 to 17 MB | 109 to 111 MB |
| S3K | 8.0 to 8.3 | 0.88 to 0.92 s | 2.11 to 2.68 s | 2.29 to 2.92 (med 2.50) | 17 to 19 MB | 242 to 244 MB |

CPU (user + sys) is within 0.05 s of wall for sigil; it is single-threaded. These reproduce
the 2026-09-27 note's section 6 (2.0x, 2.5x, 2.5x; 70 to 73, 112, 250 MB).

### Phases, per corpus (sigil-inst with the variable set, 7 runs each)

Wall share is the median of `phase wall / total wall`. Peak is the phase's own `hwm_kb`.

| phase | S1 wall ms | S1 share | S2 wall ms | S2 share | S3K wall ms | S3K share |
|---|---|---|---|---|---|---|
| read root | 0.4 to 1.7 | 0.0% | 4.5 to 4.9 | 0.2% | 7.9 to 8.3 | 0.3% |
| front-end passes (sum) | 1001 to 1158 | 96.2% | 2641 to 2814 | 97.7% | 2122 to 2314 | 94.2% |
| layout | 10.0 to 12.0 | 1.0% | 21.2 to 33.3 | 1.0% | 69.7 to 89.4 | 3.3% |
| link | 3.5 to 4.5 | 0.3% | 8.1 to 11.7 | 0.4% | 21.4 to 30.3 | 1.0% |
| flatten (incl. compression) | 9.3 to 9.9 | 0.9% | 0.5 to 0.8 | 0.0% | 6.8 to 7.6 | 0.3% |
| header | 0.1 | 0.0% | 0.1 to 0.2 | 0.0% | 0.3 | 0.0% |
| emit | 11.6 | 1.1% | 11.6 to 13.4 | 0.4% | 3.3 to 11.9 | 0.2% |
| teardown | 4.1 to 6.1 | 0.5% | 8.6 to 13.6 | 0.4% | 21.0 to 26.7 | 1.0% |
| total | 1041 to 1198 | 100% | 2703 to 2882 | 100% | 2251 to 2464 | 100% |

### Passes

| corpus | passes | bonus pass | wall per pass (med) | pass peaks (MB, med) | layout peak (MB) |
|---|---|---|---|---|---|
| S1 | 3 | never | 349, 334, 337 ms | 38, 46, 49 | 69 to 72 |
| S2 | 4 | never | 678, 651, 659, 661 ms | 68, 82, 88, 95 | 110 to 112 |
| S3K | 3 | never | 716, 703, 707 ms | 138, 160, 186 | 243 to 244 |

asl, same flags without `-q`, reports `2 passes` on all three (and 216249, 358995 and 368400
lines including macro expansions).

What moved between passes (commit `187eecfc`'s pass line; one run per corpus, images
identical):

| corpus | pass 1 against pass 0 | pass 2 against pass 1 | pass 3 against pass 2 |
|---|---|---|---|
| S1 | +91 names, 6286 changed (lowest `EEgg_Wait` 0x5a9a to 0x5a9c) | converged | |
| S2 | +629 names, 18037 changed (lowest `paddingSoFar` 0x84f to 0x661) | +9 names (`APM_*_Len`), 3 changed (`ConvRowColBlocks`, `RawColBlocks`, `current_offset_table`) | converged |
| S3K | +18 names (`PalSPtr_*._headpos` and similar), 0 changed | converged | |

Read against the source: `._headpos :=` (`sonic3k.macros.asm` 226, 259), `APM_*_Len :=`
(`s2.asm` 86229, via `__LABEL__`), `current_offset_table :=` (`s2.macros.asm` 156) are set
variables assigned before every read; `RawColBlocks = ColArrayVertical` (`s2.asm` 43362) is
read only after its definition, and `ColArrayVertical` itself did not change.

### Inside the passes (summed over all passes, 7 runs)

| step | S1 | S2 | S3K |
|---|---|---|---|
| exec (the walk) | 980 to 1136 ms, 94.2% | 2577 to 2742 ms, 95.1% | 2028 to 2205 ms, 90.0% |
| include read (read-set recorder included) | 18.7 to 21.5 ms, 1.8% (1317 reads, 7.6 MB) | 14.8 to 17.3 ms, 0.5% (1372, 3.6 MB) | 19.5 to 22.8 ms, 0.9% (1992, 5.2 MB) |
| line split | 10.8 to 12.8 ms, 1.0% | 25.3 to 28.5 ms, 1.0% | 35.7 to 41.2 ms, 1.6% |
| BINCLUDE read | 5.1 to 5.6 ms, 0.5% (1008 reads, 1.1 MB) | 13.5 to 14.2 ms, 0.5% (2416, 3.0 MB) | 17.5 to 19.2 ms, 0.8% (2520, 4.2 MB) |
| lex (upper bound, timer inside) | 133 to 151 ms, 12.7% (1.17 M calls) | 328 to 346 ms, 12.1% (2.68 M) | 242 to 262 ms, 10.8% (2.19 M) |
| seed (new `Asm`, table clones) | 1.5 to 1.7 ms, 0.1% | 5.6 to 8.1 ms, 0.2% | 7.3 to 11.4 ms, 0.4% |
| converge check (`env == prev`) | 0.4 ms | 0.6 to 1.0 ms | 1.1 to 1.3 ms |
| oscillation check | 0.0 ms | 0.0 ms | 0.0 ms |
| history push (one table clone) | 1.1 to 1.2 ms, 0.1% | 4.0 to 5.6 ms, 0.2% | 4.9 to 7.0 ms, 0.2% |
| finish (builder, section naming) | 0.0 ms | 0.1 ms | 0.0 ms |

Lex calls per pass are 1.8 to 2.0 times asl's own "lines including macro expansions" (S1 390k
against 216k, S2 670k against 359k, S3K 730k against 368k): lines are lexed more than once
per pass.

### Where exec spends it (sampling profile, inclusive share of all samples)

A function's share counts every sample with it anywhere on the stack, so rows overlap.

| function (inclusive) | S1 | S2 | S3K |
|---|---|---|---|
| front-end passes (`run_passes`) | 96.7% | 97.9% | 94.2% |
| macro expansion (`expand_macro_inner`) | 64.8% | 66.5% | 31.2% |
| `if` blocks (`exec_if`) | 45.1% | 48.1% | 17.6% |
| block-end scan (`find_block_end`, from `exec_if` in macro bodies) | 21.9% | 16.7% | 6.4% |
| macro parameter substitution (`subst_frame_text`) | 16.9% | 14.1% | 5.6% |
| lexing (`lex_line_recover`) | 11.3% | 10.6% | 10.1% |
| instruction lowering (`lower_instruction`) | 10.7% | 11.9% | 24.2% |
| per-operand builtin scan (`expand_operand_builtins`) | 4.8% | 9.5% | 12.8% |
| line token parse and copy (`parse_line_tokens`) | 5.2% | 4.6% | 3.0% |
| expression folding (`fold`) | 4.8% | 4.8% | 7.6% |
| include file read (`read_to_string`) | 2.9% | 1.0% | 1.4% |
| layout (`resolve_layout*`) | 1.3% | 1.1% | 3.1% |
| any allocator frame (malloc, free, realloc, Rust alloc shims, `finish_grow`) | 25.9% | 27.2% | 30.5% |
| drop glue | 15.5% | 16.8% | 17.2% |
| `clone`, `to_vec`, `to_owned`, `to_string` | 20.8% | 18.6% | 21.7% |
| slice equality (`memcmp`, `equal_same_length`) | 10.1% | 9.1% | 4.8% |

Call paths that explain the top rows:

- `find_block_end < block_end < exec_if < exec < expand_macro_inner` is 15.1% of S1's samples.
  For each line it scans, `line_keyword` calls `dispatch_head_checked`, which substitutes the
  whole line through `substitute_frame` and re-lexes it. The per-line head memo is keyed by the
  expansion's frame stamp, so it cannot carry across expansions of the same body.
- `starts_with` inside `substitute_frame` alone is 10.4% of S1's samples: the substitution
  tries every parameter name with `starts_with` at every byte of the line.
- `expand_operand_builtins < expand_instruction_operand < expand_calls_m68k_operands <
  lower_m68k` is 9.3% of S3K's samples: every 68000 operand is scanned, with clones, for
  builtin calls such as `strlen`, although almost none contain one.
- `to_vec < parse_line_tokens < head_of_tokens` (2 to 4%): the token vector is copied to
  peel a label; `to_ascii_lowercase` in `fold_kw`, `is_m68k_areg_name` and
  `is_expr_register_name` allocates a string per check (about 1.5% on S3K).

### Memory: where the peak is reached (S3K trace, round 2)

| point | RSS | peak in window |
|---|---|---|
| root read | 10.5 MB | 10.5 MB |
| end of pass 0 exec | 121 MB | 139 MB |
| start of pass 1 (pass 0's module dropped) | 62 MB | |
| end of pass 1 exec | 161 MB | 161 MB |
| end of pass 2 exec | 147 MB | 188 MB |
| after front end | 143 MB | |
| end of layout | 201 MB | **244 MB** |
| link, flatten, emit | 201 MB | 202 MB |
| after teardown | 115 MB | |

`run_asm` passes `&module.sections` to `resolve_layout_placing`, which returns a new
`Vec<Section>`, and `module` stays alive to the end of the function. So during layout the
front end's module (314k fragments on S3K) and its placed copy coexist. The same holds on S1
(front-end peak 49 MB, layout 72 MB) and S2 (95 MB, 111 MB).

The front end's own peak grows by pass (S3K 139, 161, 188 MB in that trace; S2 68, 82, 88, 95; S1 38, 46,
49). What is alive during pass N but not pass 0: the seed environment, macro, function and
label tables and their clones inside the new `Asm`, the previous pass's `SourceMap`
(`last_sources`, 6.2 MB of source text on S3K), the history, and allocator fragmentation.
**The split between these is not measured**: `VmHWM` cannot attribute bytes to owners, and
no allocation counter exists. The history's share is estimated from its measured contents:
`BTreeMap<String, SymbolValue>` at about 62 bytes of node per entry plus a 32-byte key
allocation (keys average 12 to 21 bytes) is about 94 bytes an entry, so S1 12359 entries is
about 1.2 MB, S2 45637 about 4.3 MB, S3K 36371 about 3.4 MB: 2 to 5 percent of the front-end
peak.

## Verdict on the prior agent's candidates

| candidate (read, not measured, in the queue row) | measured | verdict |
|---|---|---|
| `one_pass_with_defer` rebuilds `Asm` and clones every table each pass | seed step 0.1 to 0.4% of wall | **refuted** as a time cost; its memory share is part of the unmeasured per-pass growth |
| includes re-read from disk each pass (CRC, canonicalise, mutex) | include read 0.5 to 1.8%, BINCLUDE read 0.5 to 0.8% | **confirmed but small**: 1.0 to 2.3% together, of which caching would save the non-first passes, about two thirds |
| includes re-lexed by `split_src_lines` | line split 1.0 to 1.6% | **confirmed but small**. The real lexing cost is `lex_line` at 10 to 11%, which runs per line per pass and more than once per line |
| `run_passes` keeps a whole `SymbolTable` per pass in `history` | push 0.1 to 0.2%, check 0.0 ms, about 1.2 to 4.3 MB | **refuted** as a significant cost, time or memory |
| bonus pass on `force_relocate` or leftover poison | fired 0 times in 21 builds | **refuted** for this route (no `force_relocate`, no leftover poison) |
| CLI phases after the front end | 2.2 to 5.8% of wall together | **refuted for time; confirmed for memory**: layout sets the process peak on all three corpora |

What none of the candidates named: the pass count itself (one extra pass on S1 and S3K, two
on S2, against asl), and macro-body work inside exec.

## Ranked proposal (not implemented)

Byte identity on S1, S2, S3K (with the stock-ROM compare this parcel used) and on aeon is the
bar for every item. "Saving" is the measured share the item attacks and an honest estimate of
how much of it goes; estimates are marked as such.

1. **Stop when nothing a pass read has changed (read-set convergence).** Attacks one whole
   pass on S2 (1 of 4, a pass is 24% of wall) and S3K (1 of 3, 31%); none on S1, whose second
   pass changes 6286 values its first pass read. Mechanism: the pass records every name it
   looked up in its SEED (value reads, `defined`/`ifdef`/`ifndef` presence tests, and the
   seeded label and label-ref sets), and the run has converged when each recorded lookup
   gives the same answer in the pass's own output environment. Expected: S2 about 2.67 to
   2.0 s (1.9x asl), S3K about 2.24 to 1.55 s (1.8x asl); the front-end peak also drops to the
   earlier pass's (S3K about 165 MB). Risk: **high**. The proof is only as good as the
   read log is complete; a missed consumer turns into a silently stale byte. Needs a
   positive control (a source whose extra pass is required, and must still happen), the
   three corpora, aeon, and the oscillation proof re-argued, since it currently compares
   whole environments. Size: M.
2. **Find why pass 0 lays code out differently from later passes** (S1 near `EEgg_Wait`,
   2 bytes; S2 `paddingSoFar`, 0x1EE). If pass 0 sized forward-referenced constructs as asl's
   first pass does, S1 would converge in 2 (saves 1 of 3 passes, about 30%) and S2, with item
   1, in 2 (another 24 to 32%). Size: S to investigate (bisect the first moved address to the
   instruction whose size differs), fix unknown until then. Risk: **high**, sizing is byte
   identity.
3. **Keep the front end's module from coexisting with layout's copy** (consume it, or drop it
   before link). Attacks the process peak on all three: estimated S3K 243 to about 190 MB
   (the front end's own peak), S2 111 to about 95 MB, S1 72 to about 50 MB, a 15 to 30
   percent cut. The estimate assumes layout's transient working set stays below the front
   end's peak; measure it with this instrument. Risk: **low** (ownership only, no value
   changes). Size: S.
4. **Stop re-scanning macro bodies for block ends.** `find_block_end` from `exec_if` inside
   expansions: 21.9%, 16.7%, 6.4%. Compute each macro body's opener-to-closer map once per
   definition, from heads that no parameter can reach, and fall back to today's scan for a
   line whose head a parameter could change. Estimated saving most of the share, say 15%,
   12%, 4%. Risk: **medium** (a head that substitution changes must never use the cached
   map; asl's rule for that has to be stated and probed). Size: M.
5. **Make parameter substitution cheap when there is nothing to substitute.** 16.9%, 14.1%,
   5.6%, overlapping item 4. Reject a line with no possible parameter start in one pass (a
   first-byte set), build `all_args()` once per frame rather than per call, and for head
   lookup substitute only the head. Estimated half the share. Risk: **low to medium** (the
   single-pass no-rescan rule in `substitute_frame` must hold). Size: S.
6. **Skip the builtin-function scan for operands that name no builtin.** 4.8%, 9.5%, 12.8%.
   A token pre-check before `expand_operand_builtins` clones anything. Estimated most of the
   share. Risk: **low**. Size: S.
7. **Allocation churn.** A quarter to a third of samples are under allocator frames; token
   vectors copied to peel labels, a lowercase `String` per register or keyword check, drop
   glue 15 to 17%. No single fix; items 4 to 6 remove part of it. Worth a targeted pass with
   `Rc<[Token]>` or slices and case-insensitive compares after the above. Risk: low per
   change. Size: M, incremental.
8. **Cache include text and its split lines across passes.** 2.0 to 3.3% together, about two
   thirds recoverable (1.3 to 2.2%). The read-set recorder already keys by path, so the digest
   is unaffected if the first read is recorded. Risk: **low**, but a file that changes between
   passes is today a recorded conflict and would become invisible. Size: S. Low priority.
9. **Cache tokens for file lines across passes.** Lexing is 10 to 11% by sampling. The
   cacheable part (file lines, not substituted macro lines) is not separated here; the
   ceiling is about two thirds of lexing. Risk: low to medium (tokens depend on CPU and
   charset state, which the head memo already keys). Size: M. Measure the split first.

Not proposed: anything on the history, the oscillation check, the seed clones or the bonus
pass (all under half a percent here), or on link, flatten, header and emit (under 2%).

A fair expectation for items 1, 3, 4, 5 and 6 together is S2 and S3K near 1.4x to 1.6x asl
and every peak below 200 MB; item 2 is what could reach asl's pass count. These are
estimates from shares, not measurements.

## What could not be measured, and why

- **Which owner holds the front end's per-pass memory growth.** No allocation counter, and
  `VmHWM` does not attribute. A counting global allocator behind a feature would, but it is a
  larger instrument than this row asked for.
- **libc-internal time by function** (`memcpy` against `_int_malloc`): the system libc has no
  symbols for them; they are attributed to their nearest sigil caller.
- **The instruction behind pass 0's layout difference** on S1 and S2: located to an address,
  not bisected.
- **Lex time without the timer's cost**: the instrument's figure is an upper bound; the
  sampling profile gives the share instead.
- **A cost of the instrument when unset below about 5 percent**: the machine's noise at load
  9 to 11 is larger than that.

## Reproduction

Scratch was in the uncommitted `.perf-scratch/` of this worktree; the scripts are described
here rather than committed.

1. Build: `CARGO_TARGET_DIR=<worktree>/.target-perf cargo build --release -p sigil-cli`
   (and once more with `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` for the profile).
2. Corpora: `git -C /home/volence/sonic_hacks/<repo> archive --output=<scratch>/<x>.tar <sha>`,
   untar, run the stock script once (`lua build.lua`, `lua build.lua`, `lua buildSK.lua`).
3. Commands (from each tree): asl as `build_tools/lua/common.lua` composes it,
   `asl -xx -n -q -A -L -U -E -i . [-c] [-D Sonic3_Complete=0] <root>.asm`, then `p2bin <p2bin
   args> <root>.p <out> [s2.h]`; sigil as in the 2026-09-27 note's section 1, for example
   `sigil sonic3k.asm -o out.bin -D Sonic3_Complete=0 -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before`.
4. `timing.py`: per round, asl, p2bin, sigil-base, sigil-inst unset, sigil-inst with
   `SIGIL_PHASE_TIMING=1` (stderr kept), each child timed by wall clock and its own
   `os.wait4` rusage, every image compared whole to the stock ROM, `/proc/loadavg` before and
   after each round. `analyze.py` summarises the phase lines as in the tables above.
5. `overhead.py`: master against this branch, variable unset, CPU time by `os.wait4`, 12
   rounds, order alternating.
6. `sample.py` and `symbolize.py`: the sampling profiler described under Instruments. The
   selected-function table is the share of samples whose symbolised stack contains the name.
7. asl's pass count: the same asl command without `-q`; it prints `N passes`.

## After AS-PERF-FIX-PEAK, 2026-09-28

Queue row `AS-PERF-FIX-PEAK`: item 3 of the ranked proposal, then items 5 and 6.

### In plain language

- **Peak memory is down by a quarter to a third, and layout no longer sets it.** sigil used to
  hold the whole program three times at the end of layout: the front end's copy, layout's
  working copy, and the lowered result. It now hands the front end's copy to layout, which
  lowers it in place. Peak resident memory: S1 72 to 49 MB, S2 110 to 94 MB, S3K 244 to 185
  MB. The process peak is now the front end's last pass on all three, as predicted.
- **Wall time is down 13 to 17 percent**, from the two small front-end fixes: macro
  parameter substitution now copies stretches of a line that cannot hold a name in one
  step, and operands with no function-call shape skip the builtin scans. Layout itself
  also got faster (S3K 85 to 35 ms) because it no longer copies every fragment.
- **Every image is byte-identical to the stock asl+p2bin ROM**, all 168 timed runs across
  the four comparisons.
- Against the asl+p2bin figures measured earlier in this note (not re-measured here), sigil
  is now about 1.6x to 2.2x asl's wall time and about 3x to 10x its memory. The remaining gap is the extra passes (`AS-PERF-FIX-PASSES`) and the front end's per-pass
  memory growth, which this row did not touch.

### What changed

| commit | change | why it is safe |
|---|---|---|
| `3d9bdd21` | `resolve_layout_impl` owns its working copy and its final lowering consumes it, moving every fragment, label and folded equ list; `resolve_layout_placing` takes `Vec<Section>`, and `run_asm` and asl mode pass the front end's sections by value; new `resolve_layout_owned` | ownership only: placement rewrites only `lma`, the pre-fixpoint checks read the same sections, the lowering picks the same candidate by the same rung index. Structural test `relax::tests::owned_layout_moves_fragments_and_labels_rather_than_copying_them` (same heap buffers in and out) |
| `589b9976` | `substitute_frame` copies runs of bytes no candidate can start with in one step; `ARGCOUNT` text formatted only on a match; `ALLARGS` borrowed | at a byte where a candidate can start, the same candidates are tried in the same order with the same boundary rule, so the single-pass rule is untouched. Test `expand::tests::copied_runs_never_skip_a_candidate_start` |
| `4cd23814` | `expand_operand_builtins` skips the call and builtin layers when no identifier stands before a `(`, and the comparison layer too when there is no `=`/`<>` | those layers are the identity (a copy, no diagnostic) on such a slice. Test `eval::tests::operand_builtin_layers_still_run_where_they_can_act` |

### Figures

Interleaved rounds, order alternating per round, every child timed by wall clock and its own
`os.wait4` rusage with `SIGIL_PHASE_TIMING` unset (so `ru_maxrss` is the process peak). Master
`e1761831` is md5 `94b4a7db`; the tip binary is md5 `17b8a49a`.

Master against the tip, 7 rounds, 05:17:38 to 05:18:57, load 10.9 to 16.8 (a spike to 16.8 at
the start):

| corpus | wall, master | wall, tip | wall ratio per round (med) | peak, master | peak, tip | peak ratio per round (med) | CRC |
|---|---|---|---|---|---|---|---|
| S1 | 0.97 to 1.29 s (1.05) | 0.84 to 0.99 s (0.88) | 0.718 to 0.941 (0.842) | 70.0 to 72.0 MB | 45.9 to 50.0 MB | 0.640 to 0.713 (0.683) | `afe05eee/524288` |
| S2 | 2.59 to 2.85 s (2.72) | 2.15 to 2.40 s (2.26) | 0.775 to 0.901 (0.835) | 109.8 to 111.5 MB | 92.3 to 95.1 MB | 0.829 to 0.865 (0.848) | `7b905383/1048576` |
| S3K | 2.17 to 2.94 s (2.28) | 1.88 to 2.15 s (1.97) | 0.640 to 0.966 (0.867) | 243.4 to 244.4 MB | 184.1 to 186.0 MB | 0.754 to 0.764 (0.757) | `0658f691/2097152` |

Each commit against its parent, 7 rounds each (median ratios; ranges in the commit bodies):

| step | load | S1 wall | S2 wall | S3K wall | S1 peak | S2 peak | S3K peak |
|---|---|---|---|---|---|---|---|
| layout by value | 10.9 to 11.9 | 0.995 | 1.021 | 0.977 | 0.684 | 0.846 | 0.757 |
| substitution runs | 9.2 to 10.3 | 0.901 | 0.892 | 0.990 | 0.986 | 1.002 | 0.992 |
| builtin-scan skip | 8.0 to 8.7 | 0.974 | 0.932 | 0.940 | 1.006 | 1.003 | 0.998 |

Where the peak is now (`SIGIL_PHASE_TIMING=1`, one run each, 05:19, load 10.3; phase `hwm_kb`
in MB):

| corpus | last pass, master | layout, master | last pass, tip | layout, tip |
|---|---|---|---|---|
| S1 | 49.8 | 72.5 | 46.8 (pass 1) | 46.0 |
| S2 | 93.8 | 110.1 | 92.6 | 90.2 |
| S3K | 186.7 | 243.5 | 185.5 | 140.7 |

The estimate in the proposal (S3K about 190, S2 about 95, S1 about 50 MB) holds.

Item 6 saved less than its profiled share (12.8 percent on S3K, measured 6): the per-operand
comma split and string packing in `expand_instruction_operand` still run and still copy.
Item 5's S3K share (5.6 percent) is inside the noise at this load.

### The `.emp` route

The same shape was on `sigil build`: `resolve_chained` and `resolve_frozen_sections` in
`sigil-harness/src/native.rs` built an owned section list, passed it to `resolve_layout` by
reference and never read it again. Both now call `resolve_layout_owned`. Every other
`resolve_layout` caller also gains, since the lowering no longer clones (two copies where
there were three). aeon's bytes are covered by the strict suite's byte gates (the landing
run's report names them).

Left as they are, listed only: `link_to_image` in `sigil-cli/src/main.rs` (`sigil emp`,
single file) borrows `module.sections` through `link_sections`; `measure_sections` in
`native.rs` builds a tagged copy (`tagged`) it could hand over by value, but its
`resolve_layout_measuring` has no owned form. Neither is a whole-program peak on any corpus
measured here.

### Reproduction

As in the Reproduction section above, with the scratch in an uncommitted `.peak-scratch/`
of this worktree: corpora from `git archive` at the SHAs in the Instruments table, stock
script run once each for the reference ROM, `timing.py` (per round, each binary on each
corpus, `os.wait4` rusage, whole-image compare, `/proc/loadavg` before and after),
`summarize.py` for the ranges and per-round ratios, `phases.sh` for the phase lines.
