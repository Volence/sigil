# The extra traversal, measured: what it costs, where it comes from, and what a fix buys

2026-09-08. Branch `measure/extra-traversal` off master `bfd34618`. Finding under measurement:
`sig-extra-traversal` in `docs/lens-findings.jsonl`, raised by seats P1a and P1b in
`docs/superpowers/notes/2026-09-06-sigil-lens-sweep.md` (performance section). **This parcel moves
no code.** It prices the finding, and it contradicts the finding's headline.

## The one-line answer

The 2x traversal ratio is real on the corpus roots, and it is **not the convergence rule**. It is
sigil's FAILURE path being timed against asl's SUCCESS path. On a source sigil assembles cleanly
the front end takes **2** traversals, which is exactly what asl takes on both corpus roots. The
third traversal on s1disasm and the fifth on s2disasm are the poison bonus pass plus, on s2disasm,
genuine non-convergence, and both exist only because sigil cannot yet assemble those trees. The
one structural extra traversal that survives is the `force_relocate` bonus pass, which every aeon
build pays, and it is worth **0.021 s**.

Per traversal, sigil is at or near parity with asl: 1.24x on s1disasm and 1.13x on s2disasm
against an output-free asl run. The whole measured deficit is traversal COUNT.

## Provenance of every number below

| Instrument | Identity |
|---|---|
| sigil | `.target-parcel/release/sigil`, md5 `c8a3ba8b5b7c6362859bd762872d8a1c`, 8180920 bytes, built 2026-09-08 20:43:28, `sigil 0.1.0 (bfd34618)` |
| asl | Macro Assembler 1.42 Beta [Bld 212] (x86_64-Linux), each corpus's own `build_tools/Linux-x86_64/asl` |
| s1disasm | private copy at rev `f6ece657`, prepared (its `sound/dac/dpcm/generated/` is populated; asl exits 0 on it) |
| s2disasm | private copy at rev `e45ebf33`, prepared here by `scripts/corpus-prepare.sh` (74 files written; **an unprepared copy makes asl exit 3 on a missing `sound/DAC/generated/Kick.inc` and is not a baseline**) |
| aeon | `/home/volence/sonic_hacks/.aeon-ls12-fix`, detached at `ec640bcf`, READ-ONLY (see the read-only proof below) |
| timing harness | `.scratch/traversal/timeit.py`, N reps, `/proc/loadavg` sampled immediately before each rep, all samples kept, median and full range reported |
| pass census | the EXISTING `SIGIL_CENSUS_BUDGET` gate in `eval.rs` (`one_pass_with_defer` prints one `budget-census` line per traversal). No instrumentation was added to any `crates/*/src/` file. |
| aeon driver | `crates/sigil-harness/examples/traversal_census.rs`, written for this parcel, **deliberately left uncommitted**; a copy is kept at `.scratch/traversal/traversal_census.rs.instrument` |

All work, corpora, probes and logs live in `/home/volence/sonic_hacks/.scratch/traversal`, a private
directory, after the 2026-09-06 incident in which a shared scratchpad cost the reverse-perf seat its
pinned instrument.

## Item 1: what the exit condition actually is

`crates/sigil-frontend-as/src/eval.rs`, `run_impl`. The loop is at line **261** in this checkout
(the finding's `where` says 264, against sweep sha `2570f724`; it is the same loop).

```
for pass in 0..PASS_CAP {          // PASS_CAP = 16
    ... one_pass(...) ...
    if pass > 0 && env == prev { ... return ... }
    prev = env.clone(); seed = env;
}
```

The exit condition is **`pass > 0 && env == prev`**: two consecutive passes whose whole
`SymbolTable` compares equal, with the pass-0 comparison structurally excluded (`prev` starts as
the seed of `-D` defines, so an equal pass 0 could otherwise exit at 1). So the floor is 2
traversals. That much of the finding is correct.

What the finding does not say, and what dominates every count measured here, is that **converging
is not the end of the loop**. Reaching `env == prev` takes one of two branches:

- `poison.is_empty() && !force_relocate` returns the converged module immediately. Cost: N passes.
- otherwise a **bonus pass** (`one_pass_with_defer(..., true, LATER_PASS)`) runs. Cost: N + 1.

`force_relocate` is `true` for every aeon build (`sigil-harness/src/native.rs:1538` calls
`assemble_root_relocating_warned`; the only call site). `poison` is non-empty whenever the run has
an unresolved symbol, which is every run that fails.

A traversal is a genuine re-read: `eval.rs:3121` is a bare `std::fs::read_to_string` per `include`
per pass with no cache anywhere in the crate, so each pass re-reads, re-lexes and re-executes every
included file.

Verified by probe (`.scratch/traversal/probe/`, `SIGIL_CENSUS_BUDGET=1`):

| Probe | sigil traversals | asl passes |
|---|---|---|
| `flat.asm` (no forward reference) | 2 | **1** |
| `fwd.asm` (forward branch, clean) | 2 | 2 |
| `poison.asm` (one unresolved symbol) | **3** | n/a |
| `chain3.asm` (3-link forward `equ` chain) | 4 | 2, **and 2 errors: asl refuses it** |

The `poison.asm` row is the control that identifies the third traversal on the corpora: an
otherwise trivial 5-line file goes from 2 to 3 traversals purely by carrying one unresolved symbol.

## Item 2: traversals per shape

### The four aeon shapes (and the other three shipped shapes)

`SIGIL_CENSUS_BUDGET=1`, one shape per invocation, driving `assemble_as_side` directly:

| Shape | traversals | pattern |
|---|---|---|
| sonic4 plain | 3 | `pass=1`, `pass=2`, `pass=2` |
| sonic4 debug | 3 | same |
| demo plain | 3 | same |
| demo debug | 3 | same |
| config_a, config_b, lean | 3 | same |

Decomposition, from the code plus the probes: **2 convergence passes (the floor, hit on the first
possible pass) plus 1 unconditional `force_relocate` bonus pass.** No aeon shape needs a third
convergence pass; the AS residual converges as early as the rule permits.

Scale worth stating, because it reframes the finding's importance for aeon: the AS residual is now
`games/<game>/game_root.asm`, **50 lines** plus the vendored `engine/debug/debugger.asm`, emitting
no bytes and reporting `macro-expansions=0`. Its whole cost is 0.06 s.

### The Sonic corpus

| Root | sigil traversals | asl passes | sigil errors |
|---|---|---|---|
| s1disasm `sonic.asm` (72015 source lines, 216270 with macro expansion) | **3** | **2** | 50 |
| s2disasm `s2.asm` | **5** | **2** | 5151 |

sigil's s1disasm 3 = 2 convergence + 1 poison bonus. sigil's s2disasm 5 = 4 convergence + 1 poison
bonus; s2disasm is the one root measured here that genuinely needs more than the floor to converge,
and with 5151 unresolved symbols churning the symbol table that count cannot be attributed to the
convergence rule as opposed to the failure. It is recorded, not explained.

## Item 3: what a traversal costs

Every figure is the median of the reps named, with the full min and max, and the 1-minute load
average sampled immediately before each rep. This machine ran 5 to 9 other lanes throughout.

### Corpus, end to end

| Run | n | median | min | max | spread | load1 range | traversals | per traversal |
|---|---|---|---|---|---|---|---|---|
| sigil s1disasm | 7 | 0.8532 | 0.8118 | 0.8723 | 0.0605 | 6.68 to 6.82 | 3 | **0.2844** |
| asl s1disasm (`-A -L -E`, as its own build.lua runs it) | 7 | 0.5435 | 0.5257 | 0.5979 | 0.0722 | 6.70 | 2 | 0.2717 |
| asl s1disasm (no listing, no object) | 7 | 0.4585 | 0.4416 | 0.4663 | 0.0247 | 5.96 to 6.12 | 2 | 0.2293 |
| sigil s2disasm | 7 | 2.8214 | 2.7503 | 2.8729 | 0.1226 | 6.51 to 6.70 | 5 | **0.5643** |
| asl s2disasm (`-A -L -E`) | 7 | 1.0988 | 1.0670 | 1.1361 | 0.0691 | 6.43 to 6.51 | 2 | 0.5494 |
| asl s2disasm (no listing, no object) | 7 | 1.0003 | 0.9506 | 1.0309 | 0.0803 | 6.11 to 6.12 | 2 | 0.5001 |

An earlier s1disasm pair taken at load 8.4 to 8.7 gave sigil 0.8604 and asl 0.5419: the same
medians within the spread, which is the re-run the shared-machine hazard demands.

**Per traversal, sigil is 1.24x asl on s1disasm and 1.13x on s2disasm** against an output-free asl,
and 1.05x / 1.03x against asl also writing its listing and object. The end-to-end ratios (1.57x and
2.57x) are almost exactly the traversal-count ratios (1.5x and 2.5x). The packet's "within ~10 to
60%" per-traversal bracket holds, at its low end.

### Marginal calibration, this lane's own re-run of the reverse seat's instrument

Forward `equ` chains of k links spliced into `sonic.asm` after line 64 (variants
`sonic_k{0,2,4,6,8}.asm` in the private corpus copy), 5 reps each:

| k | traversals | median | min | max |
|---|---|---|---|---|
| 0 | 3 | 0.8232 | 0.8165 | 0.8478 |
| 2 | 4 | 1.1159 | 1.0969 | 1.1341 |
| 4 | 6 | 1.6534 | 1.6373 | 1.6575 |
| 6 | 8 | 2.2875 | 2.2092 | 2.4014 |
| 8 | 10 | 2.8048 | 2.7026 | 2.8151 |

Loads across the whole sequence fell from 8.21 to 6.54, which is why the slope is fitted over five
points rather than taken from one delta. Least squares over the five: **slope 0.2852 s per
traversal, intercept -0.0315 s**. Consecutive marginal slopes: 0.2927, 0.2688, 0.3171, 0.2587.
The near-zero intercept is the load-bearing part: the front end has essentially no fixed cost, so
total time divided by traversal count is a sound estimate, and 0.8532/3 = 0.2844 agrees with the
fitted 0.2852 to within 0.3%.

The reverse seat's 0.246 s per traversal is the same measurement on a differently loaded machine
and is corroborated.

Pass count against k is `k + 2` from k=2 upward (k=0 is 3, the poison floor): **one extra traversal
per forward `equ` link**, confirmed.

### aeon

AS side only, 5 reps per shape, all at load1 5.72:

| Shape | median | min | max | per traversal (of 3) |
|---|---|---|---|---|
| sonic4 plain | 0.0621 | 0.0607 | 0.0654 | 0.0207 |
| sonic4 debug | 0.0613 | 0.0591 | 0.0630 | 0.0204 |
| demo plain | 0.0583 | 0.0571 | 0.0590 | 0.0194 |
| demo debug | 0.0586 | 0.0578 | 0.0642 | 0.0195 |
| config_a | 0.0618 | 0.0605 | 0.0636 | 0.0206 |
| config_b | 0.0634 | 0.0630 | 0.0666 | 0.0211 |
| lean | 0.0613 | 0.0598 | 0.0628 | 0.0204 |

Denominator, the whole chained ROM build, demo plain, 3 reps at load1 5.15: median **0.4073** s
(0.4072, 0.4073, 0.4165), ROM 70170 bytes. So the AS front end is **14.3%** of a whole demo build,
and one of its three traversals is **4.8%**.

The sonic4 denominator is **BLOCKED**: a sound-on build calls `emit_generated`, which writes into
`$AEON_DIR/engine/sound/generated/`, and this parcel is read-only against that tree. The demo
figure is the honest stand-in and is an upper bound on the percentage, since sonic4's AS residual is
the same size while its ROM is 820229 bytes against demo's 70170.

## Item 4: what a fix would buy, priced

Four distinguishable rule changes. None implemented.

**(A) Exit after one traversal when the pass consumed no forward reference** (what asl does: 1 pass
on `flat.asm`). Saving on every shape measured here: **zero**. s1disasm, s2disasm and every aeon
residual contain forward references, and asl itself takes 2 passes on all of them. The "asl stops at
1" clause describes a source shape that no real assembly program has. Price: 0.000 s.

**(B) Fold the `force_relocate` bonus pass into the converged pass** (run the convergence loop with
`defer_unresolved_jsr_jmp` already set, so the pass that converges is itself the relocating one).
Aeon goes 3 traversals to 2.
  Saving = 1 x 0.0207 s = **0.021 s per aeon build** (sonic4 plain; 0.019 to 0.021 across shapes).
  As a fraction: 33% of the AS front end, **4.8%** of a whole demo build (0.0194/0.4073), and less
  than that of a sonic4 build. This is the only saving on the shipping path.

**(C) Fold the poison bonus pass** (do not re-run the whole source to reclassify unresolved
`jsr`/`jmp` targets on a run already carrying errors).
  s1disasm: 3 to 2, saving 0.2844 s of 0.8532, **33%**.
  s2disasm: 5 to 4, saving 0.5643 s of 2.8214, **20%**.
  It fires only on a FAILING build, which is the build a developer runs repeatedly, so it is worth
  more to a person than to CI. It buys nothing on any shipping shape.

**(D) Chase equate chains inside one pass** (a worklist over unresolved equates rather than one link
per traversal).
  On the k=8 probe: 10 traversals to 3, saving 7 x 0.2852 = **2.00 s** of 2.80.
  On s1disasm, s2disasm and every aeon shape: **zero**, because none of them contains a forward
  `equ` chain. The chain cost is real and it is currently unexercised by anything this project
  builds. It is also, as measured below, a capability asl does not have at all.

**The combined best case on the shipping path is (B) alone: 0.021 s per aeon build.** The
combination that would move the corpus numbers, (C) plus whatever fixes s2disasm's 4 convergence
passes, is worth 0.28 to 1.13 s but only while sigil cannot assemble those trees, and it disappears
when it can.

## Item 5: the confound, checked

The seat's claim, as briefed: "sigil exits before `resolve_layout` and `link`, so the corpus figure
is a front-end-only, output-free run against asl's complete assembly plus an object file and a
listing; the gap is UNDERSTATED."

**Verdict: the conclusion is right and the mechanism is wrong.**

The route is not front-end-only. `crates/sigil-cli/src/main.rs` runs `resolve_layout` (line 141),
`link` (150), `check_image_bounds` (165) and `flatten` (170) on the default `sigil <root.asm>` path,
which is exactly the path `scripts/corpus-baseline.sh:160` invokes and the one timed here. What
happens on the corpora is that the front end returns `Err` and the process exits at
`main.rs:124` before reaching any of them. Established firsthand: sigil exits **1** on both roots,
with 50 diagnostics on s1disasm and 5151 on s2disasm.

So the asymmetry is real: on the same input asl exits 0 having written `sonic.p` (531434 bytes) and
`sonic.lst` (10265043 bytes), or `s2.p` (1056949) and `s2.lst` (15647018), while sigil writes
nothing. It is understated, and by a measurable amount: dropping asl's listing and object takes it
from 0.5435 to 0.4585 on s1disasm and from 1.0988 to 1.0003 on s2disasm, so **16% and 9% of asl's
measured time is output sigil never produced**.

But the mechanism matters, because it is the same failure that produces the extra traversal. sigil
is front-end-only on those runs BECAUSE they fail, and it takes a third traversal BECAUSE they fail.
Both halves of the "2x" are artifacts of one thing: sigil cannot yet assemble s1disasm or s2disasm.
A future seat that fixes the corpus diagnostics will find the traversal gap close on its own, and
will find the output asymmetry reverse.

## Read-only proof for the aeon tree

`AEON_DIR` fingerprinted before any run and again after every run, as
`find -type f -printf '%T@ %s %p\n' | md5sum` (mtime, size and path of every file):
`806e2a3e3b22614614de3fb01a6df5bf` before, unchanged after the census, after the AS-side timing, and
after the whole demo builds. `build.sh` was never run, `sigil build --aeon` was never run, and the
driver refuses `WHOLE=1` for any `sound_on` shape precisely because `build_rom_chained` would then
call `emit_generated`.

One observation to hand on rather than act on: `$AEON_DIR/engine/sound/generated/*` carries mtime
2026-09-08 20:38, about five minutes before this parcel's first command, while the directory itself
is dated 2026-09-07 11:59. Something wrote into the reference tree today. Not this lane.

## What in the brief and in the finding turned out to be wrong

1. **"asl stops at 1."** asl takes **2** passes on s1disasm, 2 on s2disasm, and 2 on a clean
   forward-branch probe. 1 pass happens only when the source has no forward reference at all. The
   floor comparison is 2 against 2 on every real program measured.
2. **"asl resolves a chain of any length in 2."** asl **refuses** a forward `equ` chain:
   `chain3.asm(3):7: error #1010: symbol undefined`, twice, exit non-zero. sigil resolves the same
   chain correctly (emits `30 3C 00 05`, the value 5, exit 0). The per-link traversal is the price
   of a capability asl does not have, not a regression against it.
3. **"the cause is the outer convergence loop."** The convergence loop hits its floor of 2 on
   s1disasm and on every aeon shape. The extra traversal is the **bonus pass after convergence**, a
   separate mechanism in the same function: `force_relocate` on aeon, `poison` on the corpora.
4. **"we traverse about twice as many times."** 3 against 2 on s1disasm (1.5x), 5 against 2 on
   s2disasm (2.5x), 2 against 2 on any source sigil assembles cleanly.
5. **"per traversal within ~10 to 60% of asl."** Measured 1.24x and 1.13x against an output-free
   asl, 1.05x and 1.03x against asl writing its listing and object. The low end of the bracket.
6. **The confound's mechanism** (see item 5): the route does contain layout and link.
7. The brief's caution that the candidate site "is there but not established as the loop the finding
   means": it IS the loop that re-reads the source, and its exit condition is as described. The site
   is right; what the finding says the site does is not.
