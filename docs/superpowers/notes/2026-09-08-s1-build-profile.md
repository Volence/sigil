# Why sigil takes 0.88 s on Sonic 1: a decomposition, and what it is doing with the time

2026-09-08. Branch `measure/s1-build-profile` off master `ec09c700`. **This parcel moves no
shipped code.** It answers a question the owner asked by hand: *"Do we want to have sigil look into
why it takes so long compiling sonic 1? Since it's not really doing anything?"*

## The one-line answer

It is doing a great deal; it just throws two thirds of it away and keeps no output at all.

Each traversal of Sonic 1 expands **14,449 macro invocations** over 72,015 source lines into
216,249 effective lines. sigil does that **three times**, identically, and the three passes cost
0.292 s, 0.292 s and 0.295 s. asl does the same expansion **twice** and writes a 529 KB object file;
sigil does it three times and writes **nothing**, because the assembly fails and the CLI exits
before layout, link and flatten.

So the honest form of the second sentence is not that the assembler idles. It is that **on this
input sigil is the only one of the two that produces no file.** asl spends 0.451 s and hands back
`sonic.p`; sigil spends 0.876 s and hands back fifty lines of text. That is the comparison, and it
is the sentence worth keeping.

## Provenance

| Instrument | Identity |
|---|---|
| sigil | `.target-parcel/release/sigil`, md5 `8b58d49ee1736feae0726dc2809c7681`, 8180992 bytes, `sigil 0.1.0 (ec09c700)` |
| asl | Macro Assembler 1.42 Beta [Bld 212] (x86_64-Linux), the corpus's own `build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098` |
| corpus | PRIVATE copy of s1disasm at rev `f6ece657`, prepared, at `/home/volence/sonic_hacks/.scratch/s1-profile/s1disasm`; zero dirty tracked paths, the two modified `artnem/*.nem` blobs and the five `sonic_k*.asm` splices inherited from the previous parcel's copy were restored and removed before any measurement |
| phase timing | `crates/sigil-harness/examples/s1_phase_profile.rs`, written for this parcel and COMMITTED with it (the previous parcel's `traversal_census.rs` drives aeon shapes, not a corpus root, so it was replaced rather than extended) |
| end-to-end timing | `.scratch/s1-profile/timecpu.py`: wall from `perf_counter`, child CPU from `getrusage(RUSAGE_CHILDREN)` differenced across the one call, `/proc/loadavg` sampled immediately before each rep |
| pass boundaries | the EXISTING `SIGIL_CENSUS_BUDGET` gate in `eval.rs:737`, its stderr lines timestamped by the reading end of a pipe. No instrumentation was added to any `crates/*/src/` file. |
| sampling profiler | `.scratch/s1-profile/sample.py`: gdb as the inferior's parent (`ptrace_scope=1` refuses `gdb -p` on a sibling), `handle SIGALRM nopass print stop`, an external shell ticker at 10 ms, 800 samples over 9 front-end runs |

`perf`, `valgrind` and `hyperfine` are all absent from this machine, which is why the profiler is
built out of gdb and a signal.

## The load problem, and why every ratio below is CPU time

This machine ran 5 to 9 other lanes throughout. The 1-minute load average moved between **5.3 and
41.8** during this parcel, and on an unchanged subject that moved WALL clock by more than a factor
of four:

| load1 | sigil wall on Sonic 1 |
|---|---|
| 41.8 | 1.5701 to 4.0487 |
| 24 to 26 | 0.8651 to 1.4190 |
| 5.3 to 5.6 | 0.8806 to 0.9241 |

CPU time (user + system) over the same range moved from 0.84 to 1.07, and inside the quiet window
from 0.8733 to 0.9160. **Every ratio in this note is taken over CPU time**, with wall reported
beside it so the contamination stays visible. The three end-to-end subjects were also run
**interleaved**, one rep of each in turn, so they share whatever load there was rather than each
owning a different minute of it.

## Item 1: the end-to-end comparison, re-derived

Nine reps of each, interleaved, load1 5.28 to 5.58 throughout.

| Subject | passes | CPU median | CPU min | CPU max | wall median | writes |
|---|---|---|---|---|---|---|
| `sigil sonic.asm` | **3** | **0.8759** | 0.8733 | 0.9160 | 0.8889 | nothing |
| `asl -xx -n -q -A -L -U -E -i .` (as `build.lua` runs it) | **2** | **0.5316** | 0.5275 | 0.5488 | 0.5573 | `sonic.p` 529364 B + `sonic.lst` 10264800 B |
| `asl` same without `-L` | **2** | **0.4507** | 0.4459 | 0.4659 | 0.4634 | `sonic.p` 529364 B |

- sigil is **1.65x** asl-as-the-build-runs-it and **1.94x** asl without its listing.
- Per traversal: sigil **0.2920**, asl **0.2658** with listing, **0.2254** without. So sigil is
  **1.30x** asl per traversal against the listing-free run, **1.10x** against the full one.
- The end-to-end ratio is therefore mostly, but no longer entirely, traversal COUNT: 1.5x of the
  1.94x is count and the remaining 1.30x is per-traversal cost.

There is no such thing as an output-free asl run here: `-L` can be dropped but `sonic.p` cannot,
so even the 0.4507 s column includes writing a 529 KB object. sigil's 0.8759 s includes writing
zero bytes.

## Item 2: the three passes, timed individually

`SIGIL_CENSUS_BUDGET=1` with the census lines timestamped as they arrive, 5 reps at load1 8.64:

| Boundary | median | min | max |
|---|---|---|---|
| start to end of pass 1 | 0.2866 | 0.2853 | 0.2948 |
| pass 2 | 0.2921 | 0.2859 | 0.3049 |
| pass 3 (the poison bonus pass) | 0.2947 | 0.2853 | 0.3029 |
| remainder: failure assembly, diagnostic rendering, teardown | 0.0087 | 0.0081 | 0.0099 |

Every pass reports **`macro-expansions=14449 rept-bodies=434 while-bodies=0`**. The three numbers
are identical on all three passes of every rep. The passes are not converging on something; they
are the same computation performed three times.

Pass 3 is the bonus pass after convergence, and on this input it is forced by `poison`, not by
`force_relocate`: the CLI calls `assemble_root_located_warned`, so `force_relocate` is false here
and only the unresolved symbols of a FAILING run ask for the extra traversal. Confirmed by the
previous parcel's `poison.asm` probe and unchanged by this one.

Pass 2 is not waste in the same sense: the convergence rule is `pass > 0 && env == prev`, so a
second traversal is how the first one is known to have settled, and **asl pays exactly the same two
passes**. The comparison that matters is 3 against 2, not 3 against 1.

## Item 3: where the time goes inside a traversal

800 samples at a nominal 10 ms interval across 9 front-end runs, gdb backtraces, each sample charged
to the innermost `sigil` frame with allocator and libc frames rolled up into their caller. Buckets
are mutually exclusive and sum to 100%. One-sigma sampling error on a 25% bucket at N=800 is about
1.5 percentage points.

| Share | of 0.876 s | Bucket |
|---|---|---|
| 26.25% | 0.230 s | macro expansion (`expand::substitute_frame`, `expand_macro_inner`, `subst_frame_text`, `expand_calls`) |
| 19.88% | 0.174 s | line dispatch (`exec`, `dispatch`, `dispatch_resolved`, `head_of_tokens`, `line_keyword`) |
| 16.75% | 0.147 s | lexing (`lexer::lex_line_recover`, `split_src_lines`, `is_op_keyword`) |
| 12.62% | 0.111 s | other (`qualify`, `check_call_args`, `split_commas`, `sym_key`, integer formatting, hash inserts) |
| 7.12% | 0.062 s | string builtins (`scan_str_builtins`, `expand_*_builtin`, `expand_str_comparisons`) |
| 6.25% | 0.055 s | copying token vectors (`Token to_vec`) |
| 4.38% | 0.038 s | symbol table (`sigil_ir::symbols::resolve` / `define`) |
| 2.62% | 0.023 s | expression parse and evaluation (`expr::`, `eval_all`) |
| 2.12% | 0.019 s | pass driver (`one_pass_with_defer`, `run_impl`, `assemble_root*`) |
| 1.75% | 0.015 s | instruction lowering and emit (`lower_m68k`, `emit`) |
| 0.25% | 0.002 s | include machinery (`directive_include`) |

Inclusive figures for the same 800 samples: `run_impl` **99.62%**, `one_pass_with_defer` 98.50%,
**`expand_macro_inner` 69.50%**, `lexer::lex_line_recover` 14.88%, `expr::` 1.62%, `lower_m68k`
7.62%, `emit` 0.38%.

**The single most useful number in the profile:** the innermost frame is in the allocator or in
libc's memmove for **28.75%** of samples (0.252 s). Their callers are led by
`expand::substitute_frame` (7.25% of the whole run on its own), then `SymbolTable::resolve` and
`head_of_tokens`. Add the explicitly-attributed `Token to_vec` copies and `String::clone` and the
front end is spending roughly a third of its life allocating, copying and freeing token vectors and
strings. It is not spending it on arithmetic: expression evaluation is 2.62% and instruction
encoding is 1.75%.

## Item 4: the two terms the brief expected to be large, measured, and both are not

**Diagnostic rendering: 0.00015 s, or 0.017% of the run.** Measured by calling exactly what
`render_as_diags` calls (`sources.label(d.primary)` plus the `format!`) with the result consumed so
it cannot be optimized away, 9 reps: 0.000142 to 0.000225 s, and below the 10 ms resolution of
process CPU accounting. Zeroing it entirely buys **nothing measurable**.

The brief's mechanism for expecting otherwise is refuted by the code as well as by the clock.
`SourceMap::location` does **not** walk the file from byte zero: it binary-searches a per-source
line-start index held in a `OnceLock` and built once, then counts characters within **one line**
only. Cost per diagnostic is therefore about 3 microseconds and does **not** grow with position in
the file. The one linear scan is building a file's index, once per file that carries a diagnostic,
and it shows up as the difference between the first rep (0.0054 s, indexes cold) and every later one
(0.0002 s). Priced forward: even at the 9,739 diagnostics of the 2026-09-03 baseline, rendering
would be about 0.03 s, or 3% of the run.

**File reading: 0.0022 s for all three traversals, or 0.25% of the run.** The run splices 440
distinct files totalling 2,741,035 bytes, and re-reads every one of them on every pass with a bare
`fs::read_to_string` and no cache. Reading all 440 files three times, measured directly, costs
0.0022 s warm (0.0519 s on the first cold rep). The re-reading is real and it is free: the page
cache absorbs it. A file cache would buy 0.25%.

**The symbol table clone was also checked and is not a term.** `run_impl` does `prev = env.clone()`
on every pass; zero of 800 samples landed in a `SymbolTable` clone or comparison, and all symbol
table work together is 4.38%.

## Item 5: the diagnostic count, settled

The count today is **50**, and the two figures the brief put in conflict are both real. They are a
time series, not a contradiction:

| Date | Count | Source |
|---|---|---|
| 2026-09-03 | 9,739 | `docs/lane-log.jsonl:139`, baseline at master `ecb69f5a` against s1disasm `f6ece657` |
| 2026-09-03, later | 1,367 | same log, after the eval rebase; `unrecognized 68000 mnemonic` 8,939 -> 728 |
| 2026-09-08 (this parcel) | **50** | `sigil sonic.asm` at `ec09c700`, same corpus rev `f6ece657` |

50 is not a cap. There is no error limit anywhere in `crates/`, and the 50 lines decompose into ten
distinct message classes across seven files, the largest being 18 `unexpected character` on a single
line of `MacroSetup.asm`:

| Count | Class |
|---|---|
| 18 | `MacroSetup.asm(98)`: unexpected character |
| 9 | `sonic.asm(2616-2675)`: `charset` is not a recognized 68000 mnemonic |
| 6 | `sound/_smps2asm_inc.asm`: `case` needs a string literal |
| 6 | `_incObj/82, 83 SBZ Eggman Cutscene and Crumbling Floor.asm`: bad immediate expression |
| 4 | `sound/z80.asm`: trailing tokens in operand |
| 2 + 2 | `sound/_smps2asm_inc.asm`: unresolved `if` condition; `switch` needs a string expression |
| 1 + 1 + 1 | `listing` / `page` directives, in `MacroSetup.asm` and `sound/z80.asm` |

Where they sit in the file was the brief's question because rendering was thought to be
position-dependent. It is not, so the positions do not matter to cost. They matter to the fix: nine
of the ten classes are missing directives (`charset`, `listing`, `page`) and string-flavoured
`switch`/`case`, which is a language-coverage list, not a performance one.

The run also gets far enough to print `Uncompressed driver size: 1BBDh bytes.` on stdout, which is
the source's own `message` directive. sigil reaches it.

## Item 6: priced against the reference

Each term is what the run would cost if that term went to zero. CPU seconds, and share of sigil's
0.8759 s.

| If this went to zero | Saves | Share of run | sigil would then be | vs asl (no listing) |
|---|---|---|---|---|
| the poison bonus pass (pass 3) | **0.2920** | **33.3%** | 0.584 s | 1.30x |
| the per-traversal excess over asl, all 3 passes | 0.200 | 22.8% | 0.676 s | 1.50x |
| **both of the above** | 0.425 | 48.5% | **0.451 s** | **1.00x, parity** |
| the allocator and memmove time | 0.252 | 28.8% | 0.624 s | 1.38x |
| diagnostic rendering | 0.00015 | 0.017% | 0.876 s | 1.94x |
| file re-reading on passes 2 and 3 | 0.0015 | 0.17% | 0.874 s | 1.94x |
| the symbol table entirely | 0.038 | 4.4% | 0.838 s | 1.86x |

The last three rows are the point of the table: the two terms that sound expensive when described
in prose, re-reading 2.7 MB of source three times and rendering diagnostics one file-walk at a
time, are together worth **0.2%** of the run.

## Item 7: his second sentence, answered in numbers

*"Since it's not really doing anything?"*

**Reading it as a claim about the assembler, it is half right, and the half that is right is worse
than the claim.**

- Work that produces a **file the user wants**: **0.000 s of 0.876 s, 0%.** The run exits at
  `sigil-cli/src/main.rs:124` before `resolve_layout`, `link`, `check_image_bounds` and `flatten`.
  asl on the same input spends 0.451 s and writes a 529 KB object.
- Work that produces the **text the user reads** (50 diagnostics plus one `message` line, 3,336
  bytes): **0.00015 s, 0.017%.**
- Work that is a **repeat of a computation already performed**: passes 2 and 3, **0.587 s, 67%**.
  Both re-expand the same 14,449 macros to the same counts. Pass 2 is the convergence proof, which
  asl also pays; pass 3 exists only because this run fails.
- Work that is **genuinely first-time**: one traversal, **0.292 s, 33%** -- and 0.230 s of that one
  traversal is macro expansion, with 0.055 s more copying the token vectors it produces.

So the assembler is never idle. It expands 43,347 macro invocations across the run, and the reason
that is invisible is that all three passes are discarded: diagnostics are returned from the
converged pass alone (`eval.rs`, and the comment there says so), and the module is discarded at the
`Err` arm. **The user pays for three full assemblies of Sonic 1 and receives the diagnostics of
one.**

**Reading it as a claim about the lane, it is also right,** and the correction is that the queue row
`S1-BUILD-PROFILE` says "about 1.3 seconds where the old assembler takes half". Neither half of that
sentence survives: 1.3 s is a loaded-machine WALL figure (it sits inside the 1.42 to 4.05 s range I
measured at load 25 to 42), the quiet-machine figure is 0.876 s CPU, and asl takes 0.451 to 0.532 s,
which is 51% to 61% of sigil rather than "half".

## Item 8: the obvious fix, described and NOT done

**Do not run the bonus pass when the run has already failed.** Its whole purpose is to reclassify a
`jsr`/`jmp` bare-symbol target that still folds to Poison as a deferred cross-seam reference for the
linker. On a run that is going to exit 1 there is no linker to defer to and no module to hand it, so
the third traversal of Sonic 1 computes a `Fragment::JmpJsrSym` set that is dropped microseconds
later at `main.rs:124`. Gating it on "poison remains AND the run is otherwise clean" would take
Sonic 1 from 0.876 s to 0.584 s, a **33% cut**, and would change nothing on any shipping shape,
because `force_relocate` is what triggers the bonus pass on aeon builds and that arm is untouched.
The risk is that the diagnostics returned from the converged pass would then come from pass 2 rather
than pass 3, and the two are not obviously identical on a failing run: pass 3's whole job is to
reclassify some of pass 2's errors into deferrals, so dropping it could make a failing run LOUDER.
That has to be measured over all six corpus roots before anyone touches it. **Not done here; it
needs the owner's word.**

The larger term, and the one that is not obvious or small, is allocation: 28.8% of the run is the
allocator and memmove, most of it under `expand::substitute_frame` copying token vectors and
strings. That is a real optimization campaign, not a parcel.

## What in the brief turned out to be wrong

1. **"Diagnostic rendering walks the file from byte zero per diagnostic, so cost grows with position
   in the file."** It does not. `SourceMap::location` binary-searches a cached per-source line-start
   index and then counts characters within one line. Cost per diagnostic is constant in position,
   about 3 microseconds, and the whole term is 0.017% of the run.
2. **"The note records 50 diagnostics, the hub's log records 1,367; those cannot both describe the
   same run."** They do not describe the same run and neither is wrong: 9,739 (2026-09-03 baseline),
   1,367 (2026-09-03, post-rebase), 50 (2026-09-08). It is a time series over five days against the
   same corpus rev.
3. **"If the real count is in the thousands, diagnostic rendering is a large term."** The premise is
   false and so is the conclusion: at 9,739 diagnostics rendering would still be about 3% of the run.
4. **"Per traversal sigil is 1.24x an output-free reference, so the deficit is count rather than hot
   code."** Re-derived tonight as **1.30x**, and there is no output-free reference: asl always writes
   its object file. More importantly the framing does not hold any more -- at 1.94x end-to-end
   against 1.5x on count, per-traversal cost is now 23% of the run's excess, not a rounding error.
5. **The queue row's "about 1.3 seconds where the old assembler takes half."** 1.3 s is a loaded
   wall figure; the CPU figure is 0.876 s and asl is 51% to 61% of it, not 50%.
6. Last night's **0.8532 s** median stands: 0.8889 s wall / 0.8759 s CPU tonight at a comparable
   load, on a different binary five commits later.

## Instrument disclosures

- `crates/sigil-harness/examples/s1_phase_profile.rs` is new and is committed. It is an example, not
  `crates/*/src/`; no shipped code was altered by this parcel, which `git show --stat` on the commit
  shows directly.
- The **sampled** binary was built with `RUSTFLAGS="-C debuginfo=1"` into a separate target
  directory. Debug info does not change codegen, and that was checked rather than assumed: front-end
  CPU median was **0.8700 s** for the ordinary release binary and **0.8800 s** for the
  debuginfo binary *while it was being stopped 800 times by the profiler*, so neither the debug info
  nor the sampling perturbation moved the subject out of its own spread. Every timing figure in
  items 1, 2, 4 and 6 comes from the ORDINARY binary; only the item 3 decomposition comes from the
  debuginfo one.
- Both builds used `CARGO_TARGET_DIR` inside this worktree. The shared assembler binary other lanes
  pin by digest was never relinked.
- Nothing outside `/home/volence/sonic_hacks/.scratch/s1-profile` and this worktree was written.
  `AEON_DIR` was never read or touched: this parcel's subject is a corpus root, not an aeon shape.
