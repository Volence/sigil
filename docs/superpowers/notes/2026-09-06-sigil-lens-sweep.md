# sigil lens sweep, 2026-09-06

Ratified Lens Sweep Protocol (aeon `docs/superpowers/LENS_PROTOCOL.md` at aeon `61f22403`,
verified an ancestor of their `origin/main`). Run on the owner's own words, 2026-09-06T07:23:14Z,
verified firsthand at empyrean `origin/main`: *"I'd really like to finish up the raster/parallax
and then run a full lens suite on everything"*. EFFECTS-W1 closed at aeon `dad5c395`, also
verified reachable, which is the precondition his sentence names.

**Review SHA: sigil `ebc3d17e`**, clean detached worktree at `.lens-2026-09-06`.
**Roster B at corpus scale, 22 seats**, every one a fresh subagent, launched read-only.

## Charter, and what is UNEXAMINED rather than cleared

In scope: all 14 workspace crates, 508 `.rs` files, 251,546 lines, plus `repin.toml`,
`golden/provenance.toml`, `scripts/`, and the test tree.

**Out of scope and therefore UNEXAMINED, not blessed:** vendored C (`clownlzss`, `salvador`,
`libdivsufsort`), the aeon corpus itself (seats were scoped to this worktree, so every
"does the corpus reach this?" question is open and marked as such per finding), and `docs/`
beyond what a finding had to resolve.

**A constraint that shaped every seat: no cargo.** A build here would relink the shared
`target/release/sigil` that other lanes pin by md5, and would recreate the target directories
reclaimed hours earlier. So **the panel is a reading, not a run.** Every seat disclosed this;
several found ways to measure anyway (an independent Python re-implementation of a gate's lexer,
863 forms swept against the reference `asl`, a static model validated by reproducing a historical
figure). Where a finding needs a build to become byte-proven, it says so and names the command.

## Step 0 — the standing findings, re-verified rather than blank-slated

The 2026-08-13 packet is closed except two, and both moved:

- **S12** (the trusting closure kept for warn-tier analyses) — CONFIRMED-STILL-OPEN, and the
  **cost re-derived rather than quoted: 53 to 59**. Six new procs, all aeon code written since
  August; the dispatch-site count did not move, so the growth is ordinary object work reaching
  the dispatch loop at roughly +0.25 contracts/day. The in-code doc and the gap ledger both now
  state a stale number. **The derivation carries its own control**: the model reproduced the
  historical 53 exactly at three separate sweep-era revisions, which is what makes 59 credible.
- **S8** (the deferred harness crate split) — CONFIRMED-STILL-OPEN and **~70% larger**: a
  ~9,300-LOC move is now ~15,800. The harness doubled (11,695 to 24,540 LOC) and gained a
  dependency the seam plan predates.

Two corrections to the prior packet's own text, found while re-deriving: it said S12 was live at
"8 sites" and there were **12**, at both revisions; and one of those sites sits inside a comptime
splice template that the analyzer structurally cannot see, which moves that sub-finding from
*latent* to *live*.

## THE HEADLINE: one class, seven independent seats

**A property is verified at the producer and the consumer never sees it.** Independently found by
CGa, CGb, SAFE, COMPTIME, RELAX, FUZZ and B2a, in different crates, by different walks:

| site | guard | consumer | result |
|---|---|---|---|
| AS `align` | `n > 0` on `i64` | `n as u32` | `align $100000000` becomes 0, `wrapping_rem(0)` **panics in release** |
| AS `ds.b/w/l` | `v >= 0` on `i64` | `v as u32 * unit` | truncates to 0, reserving nothing |
| `.emp` `bank:` | positive power of two on `i128` | `v as u32` | `2^32` passes the test, arrives as 0 |
| Z80 symbolic `jp`/`call` | none | `value as u16` | truncating fixup where the emp twin range-checks |
| AS `dc.w` / `dc.l` | none | `v as u16` / `as u32` | silent truncation, while `dc.b` on the same page checks |
| abs.w window | checked on 3 paths | 4th path eager-folds | `($C00004).w` writes to `$00000004` |
| comptime vs link | `checked_*`, refuses | `wrapping_*`, silent | `1<<64` is an error at comptime and `1` at link |

**The codebase contains the correct idiom too**, at three sites — so these are divergences, not an
unknown gap. And in two cases the repo's own comment, a few lines away, argues at length that the
truncating form is "the silent-wrong-bytes class."

## Findings that change bytes

1. **`shift_offset` returns wrong label addresses after a backward `org` with growth.**
   `sigil-link/src/relax.rs:411`. Takes the LAST breakpoint at-or-before an offset, over a table
   that a backward `org` makes non-monotonic, so a later small entry overwrites the delta owning
   an earlier label. Derived from an existing test with one value changed: correct 7, returns 5.
   **Three seats hit this function and two disagreed; the adjudication is below.**
2. **`btst #n,Sym(pc)` patches the bit-number word instead of the displacement.** Found by CGa
   (via the addressing-mode model) and independently by B1 (via blessed-construct-versus-copy).
   Both front-ends pass a hardcoded extension offset of 2; `encode_bit` emits the bit-number word
   first, so the correct offset is 4. **The repo already knows**: `capstone_diff.rs:709` documents
   this exact population with the asl witness `083A 0001 0002`. The blessed sentinel-probe
   technique exists two functions away and is used at two of four offset sites.
3. **`.emp` `align` still emits bytes the AS side stopped emitting two days ago.** `84c48a7b`
   (09-04) moved AS `align` to `reserve` because asl records nothing for the addresses an align
   steps over, and touched only that side. `.emp` still emits `Fill` and still uses a plain
   unsigned round-up where the shared rule is signed. **The parity gate cannot see it**: all three
   cases place data AFTER the align and flatten with `0x00`, so fill and no-fill are identical.
   **A coupling nobody wrote down**: the DAC intra-bank recompute only still fires because `.emp`
   align emits `Fill`, so fixing this alone lands the drum bank off its `$8000` boundary.
4. **`here()` is baked from a pre-placement counter while labels follow placement.** Two comments
   in the same crate now contradict each other, and the wrong one is the one a reader trusts:
   `builder.rs:101` says staleness "hits labels and `here()` equally"; `lower/mod.rs:503` says the
   section's labels follow wherever it is PLACED, then passes the pre-placement counter as
   `here()`'s base on the next line. No `here()` test runs a placement pass. Exposure is
   **data sections**, not code, because a relaxable branch makes `here()` provisional and correct.

## Findings that lose bytes silently, or hang

5. **An unterminated `if` / `rept` / `macro` deletes source and exits 0.** `find_block_end` treats
   the file's last line as the missing closer. A file missing one `endif` assembles to
   `AA 01 02 03 04` where the truth is `AA 01 02 03 04 05`. **Reproduced against a real binary.**
   Twelve lines away, the code refuses a no-verdict `if` condition for precisely this reason.
6. **A macro that calls itself twice never terminates.** Depth is capped at 64; breadth is not.
   The same file already argues this exact point for nested `while` and adds a global budget;
   the argument was never carried to macros or to `rept`, which has no iteration budget at all.
7. **`org -1` materialises a 4 GiB image; with `--hex` the process aborts** asking for 96 GiB.
8. **`flatten` sizes its buffer from a section's placed ADDRESS.** One stray byte in a RAM-phased
   section attempts a 4 GiB allocation and **aborts** before any diagnostic. The correct guard
   exists (`validate_section`) and is not on the CLI path.

## Gates that cannot fail

9. **The skip-marker lint is blind to a wrapped string literal**, and five live announcements are
   already through the hole, in a file with no strict gate. Their text is in the lint's own
   forbidden vocabulary; they are green only because of a line break. **Five compliant sites are
   already over rustfmt's default width and there is no `rustfmt.toml` and no `cargo fmt` check**,
   so an ordinary formatting pass would silently shrink the gate's census.
10. **The A/B-evidence guard checks only that the field is non-empty.** Five anchor-moving freezes
    satisfy it with an `ab` value that is literally the name of the preceding chain entry,
    chaining back to the string `"master"`.
11. **A trio of port tests whose comments claim a proof the code does not perform** — the zero is
    the branch CONDITION, and one arm asserts a slice is empty when its own bounds are `[b..b+0]`.
    A prior sweep hardened the comparison helper and missed the arm that skips the helper, so the
    hardened helper now makes the trio look covered.
12. **`refreeze --attest` records `passed` with skips**, which no defect check reads. Known and
    deferred; never yet exercised (all 30 strict records carry zero).
13. **33 pin constants have no consumer**, 29 of them carrying a generated `tests:` line naming a
    binary that never reads them; and the manifest declares that field dead while the generator
    still stamps it into 512 doc lines.

## Seat conflict, adjudicated

**P2 and CACHE disagreed about `shift_offset`'s cost, and both were right.** P2 called it
O(labels x fragments) and its top item; CACHE went to make the same point, measured section
granularity, found ~200 small sections, and refuted its own headline, noting the convenient result
was what prompted the check.

**They measured different corpora and neither said so.** Counted here to settle it: the Sonic 1
disassembly has **11 section-opening directives in 4 files** (control: 92 hits for a common string
in the same file), so the AS path is a handful of huge sections and P2's quadratic is real. The
`.emp` path gives each of 197 modules its own sections, so CACHE's refutation is equally real.

**The synthesis neither seat could reach alone:** on the AS path the same function is both
quadratic AND wrong, for one underlying reason - a linear last-match scan over a table that `org`
makes non-monotonic - and `org` exists ONLY on the AS path. One fix addresses both. On the `.emp`
path it is neither.

## Convergence

- `btst` pcrel offset: **CGa and B1**, opposite walks.
- The guard/consumer class: **seven seats**.
- The align parity gate samples the agreeing prefix: **LINK** (fill vs reserve axis) and **B2a**
  (value and PC-sign axis).
- `1<<N` masking: **COMPTIME** (reading the two evaluators) and **FUZZ** (running a binary).

## Seats' own corrections, recorded because they are the method working

- **CACHE** refuted its own headline on a granularity check and said the convenient result was the
  trigger.
- **A2** withdrew a count after finding its instrument had measured a different population that
  happened to land on the same integer (27).
- **CGb** returned a **clean negative** on the wrong-bytes question after the panel's heaviest
  work: an independent model validated against 150 asl-minted vectors, its own harness
  mutation-checked, then 863 forms swept against the reference assembler. Zero wrong bytes.
- **STEP0** established its number by reproducing a historical one, not by asserting a method.

## Aftermath, per protocol: seats stayed read-only, fixes are separate work

Nothing was fixed during the sweep. Triage bins are the protocol's: byte-changing fixes take their
own parcels with before/after evidence; measure-first items go to a profiler; byte-neutral
corrections (comment truth, dead-guard deletion) may land immediately; structural findings get
their own arc item; open questions go to the owner.

**Dry criterion, restated so nobody claims it early:** this track is dry only when a fresh panel,
run AFTER this round's fixes land, returns nothing new.

## CI and the nightly lanes, measured on the hub's ask (oracle's finding, applied here)

Oracle found their CI red for 46 days unnoticed, 529 of 554 runs, with five corpus-presence
guards quietly turned into guards that cannot run. Measured here, same three questions:

- **Last green sigil CI run: 2026-08-27T11:18:17Z. Red for 10 days.** Over the last 1,000 runs
  (window opens 2026-07-12), **838 failures against 162 successes**, and the most recent 400 are
  failures without exception.
- **The cause is a guard about guards.** Two rows in `crates/sigil-harness/tests/bare_run_refuses.rs`
  panic at `test_support.rs:1204`: *"NO REFERENCE TREE IS NAMED, so this run can measure nothing it
  could attribute, and STOPS."* That file exists to prove `d-18` REFUSE-BARE behaves, and CI is a
  declared partial run (`SIGIL_ALLOW_PARTIAL=1`, `ci.yml:68`) - the exact condition whose refusal
  the test is written to observe. So the check that guarantees a bare run cannot pass silently is
  itself the check CI cannot run green.
- **The nightly lanes ARE running, which is where sigil differs from oracle.** Both timers are
  armed and fired today: `sigil-source-gates` at 05:17 EDT, `sigil-ref-drift` at 07:17 EDT.
  Drift finished clean. **`sigil-source-gates` FAILED today, exit 1** - and it succeeded on
  2026-09-04 and 2026-09-05, so this is a fresh red rather than a rotted one.
- **A guard that exists only outside the landing bar** (seat V, independently): the zero-skip rule
  is documented in four places as something *"the landing bar fails on"*, and `landing-run.sh`'s
  exit decision reads `CARGO_RC`, `FAILED`, `CLIPPY_RC` and `RECONCILED` - **never `SKIPS`**. The
  only real enforcer is `nightly_source_gates.sh`, which is not the landing bar. A strict landing
  run carrying skip lines prints `RESULT GREEN` and exits 0.

**The transferable half, which is oracle's and not ours:** a red that nobody reads stops being a
signal, and the way it dies is that two failures stack and the second is assumed to be the first.
Sigil's red is 10 days old rather than 46, and the nightly lane's red is hours old - both are
still young enough to be attributed rather than archaeology.

## The data-first walk, and a count this packet is NOT going to average

**Stale addresses that only exist in prose, so no gate can reach them.** Five sound-head
addresses across seven `seam2_*` test files still name the pre-relayout bank: `$58000` where the
frozen size table says `SoundTablesZ80_Head 0xb8000` (verified here at
`golden/offcanonical_sizes/s4.txt:72` against `seam2_phased_head.rs:5,14,23`). The assertions are
derived and therefore correct; the doc comments AND the panic messages are hand-typed, so when one
of these gates goes red it tells the reader to look 384 KB from where the bytes are. Two of them
are wrong in the relative offset too, having been computed against a 270-byte SFX head that is now
274 - and one file contradicts itself, saying 270 in its header and asserting 274 in its body.

**`pins.rs` names test binaries that do not exist.** `m1d_rom`, `m1d_debug_rom` and
`mixed_dac_rom` appear in the generated `tests:` doc line on the two most-read constants in the
file; none of the three exists in the tree (control: `m1b_gate.rs` and `m1c_vector_table.rs` do).
Nine constants name a non-existent binary. This is the dead-field finding with a sharper edge than
"it is unmaintained": it is a record that a future author will read as evidence of coverage, and
it names files that were never there.

**Two counts in `repin.toml`'s own header are wrong about its own contents**: it says removing
"412 `tests` lines" is a mechanical no-op (actual: 512), and it advertises "80 region/shape pairs
declare `allotment` today" as a standing conversion backlog (actual: **1**). The gate it names is
correctly count-free and reads the live manifest, so the stale figure lives purely in prose.

### ADJUDICATION: 33 orphan pins, or 22

Seat GATE reported **33** pin constants with no code consumer. Seat B2b, walking data-first,
reported **22** and enumerated every one with its line number, stated its predicate (no reference
in any `.rs` outside `pins.rs`), and ran it beside a known-consumed control.

**This packet records 22 and does not average the two.** The rule is this lane's own, banked after
a count drifted in both directions inside twenty minutes: **write the SET, not the count.** An
enumerated list with a stated predicate and a control can be checked by anyone; a bare integer
whose predicate is unstated cannot be reconciled with anything, and the difference between the two
figures is almost certainly scope (prose-only hits, `crates/*/tests/*.rs` versus all `.rs`) rather
than a disagreement about the code. Spot-checked here: `EPILOGUE` has no consumer outside
`pins.rs`; `KNUCKLES_ANIMS`'s only other hit is `provenance.toml`, which is data, not a consumer;
the control returns two consuming files.

**The generated artifacts are internally sound, which is the honest other half.** B2b re-ran the
TIP-MATCH invariant by hand without cargo: 7/7 blobs match on CRC32, length, and the header-neutral
anchor, and it names the two ways that reconstruction could have failed and did not. 87 and 98
label pairs agree between the size tables and `pins.rs`, with zero disagreements and no hand-edit
evidence in any generated file.

## Performance: the two walks, and why doubling that seat paid

The protocol doubles perf seats deliberately, "because measuring the same thing from two ends is
the only reason a bad number gets caught." Both walks reached the same headline by different
instruments, and the reverse walk found a cost the forward walk could not see.

**Agreed, by two different methods.** The gap is the outer fixpoint, not the hot code. Forward
used an env-gated pass census; reverse used **marginal-pass calibration** - append a forward `equ`
chain of length k, find where the plateau ends - and got a clean 0.246 s per additional traversal.
Both land on: sigil traverses the source **twice as many times as asl**, and per traversal is
within ~10 to 60% of it. Convergence needs two consecutive identical symbol tables, so the
earliest exit is 2 traversals where asl stops at 1; a forward-referenced equate costs one more per
link, where asl resolves a chain of any length in 2.

**The forward walk's own headline confound, which reframes every ratio in this packet:** sigil
exits before `resolve_layout` and `link`, so 1.3 s is a front-end-only, output-free run against
asl's complete assembly plus a 531 KB object file and a listing. **The gap is understated.**

**What only the reverse walk could find: a per-nesting-level re-lex.** A block body is re-scanned
once per enclosing level, untaken arms included. Measured against depth, asl is FLAT and sigil is
linear: at depth 32, 1.697 s against asl's 0.032 s, **52x**, at +0.052 s per level - 52% of the
whole depth-0 cost, per level.

**And it is nearly invisible on the corpus this project benchmarks with.** Mean block-nesting depth
per line: **s1disasm 0.054** (about 3% overhead, which is why the headline looks benign), **aeon
1.157** (about +60%). On s2disasm, macro-heavy with conditional bodies everywhere, **sigil takes
5.82 s against asl's 0.40 s: 14.5x**. A seat measuring only the briefed corpus would have reported
the fixpoint and stopped.

**A confound that makes the deficit worse than the headline, volunteered rather than buried:** asl
is catastrophically bad at large symbol tables (200k labels: sigil 0.41 s, asl **31.0 s**), and
sigil also wins on symbol arithmetic (0.81x). So sigil takes its 2.5x deficit while WINNING the
symbol-table sub-problem, which means **the per-line deficit on the non-symbol majority is worse
than 2.5x**. The seat that found this had every reason to bank the flattering half and did not.

**Diagnostic rendering is O(file) per diagnostic.** `SourceMap::location` walks `char_indices()`
from byte 0 on every call, so an error at the top of a file costs about 18 microseconds and one at
the bottom of a 1.6 MB file costs about **0.45 ms**, growing with file size. s1disasm's 50
diagnostics are invisible; s2disasm's **5,229** are not. It only bites a FAILING build, which is
the build a developer runs repeatedly.

**Retraction, recorded because the seat volunteered it:** an earlier tcmalloc figure of 1.93x was
contaminated by a concurrent agent in the same directory and is withdrawn; the private-directory
re-run gives **1.15x** and stands. Allocator traffic is about 13% of the run, and the same seat
measured the symbol table it had expected to indict and found it a **minor** term (+15% across
four orders of magnitude of table growth), reporting it as hygiene rather than speed.

## A shared-machine hazard, reported because it damaged a measurement

The reverse-perf seat had its pinned instrument and several probe files **deleted mid-run** by a
concurrent agent writing into the same scratchpad, and files it never created appeared beside its
own. It re-copied, re-verified the md5, moved to a private subdirectory and re-ran. Two agents told
to build scratch files and not told where to put them are each other's concurrent writer by
construction; the dispatch invariant already says so about detached scripts, and this is the same
shape arriving on a measurement. **Every seat brief should name a private scratch directory.**

## Panel complete: 22 of 22

All seats returned. Nothing was fixed; every finding above is read, derived, or measured against a
pinned instrument, and every seat disclosed that it could not build.

## Three-state gates: asked of sigil's lanes on aeon's finding

Aeon's effects gate said COULD NOT RUN nine nights running because its pytest lane read listings
from the working tree, a nine-day-old listing failed the gate, and the gate exited before the step
that would have rebuilt the listing. Absent SKIPs, fresh PASSes, stale FAILs, and the false red
keeps the input stale. Asked here of both nightly lanes:

**Both distinguish all three states, with distinct exit codes, deliberately.** `0` OK, `1` a real
gate failure, `2` COULD NOT RUN. `nightly_source_gates.sh:24` states the contract in its own
header; `nightly_ref_drift.sh:118` states it and records the incident that earned it, four
unexplainable COULD NOT RUNs written to a terminal nobody kept.

**A stale input cannot jam the loop that refreshes it here, and the reason is structural rather
than careful.** The source-gate lane does not read a working tree at all: it creates FRESH
detached worktrees (`git worktree add --detach`, then `checkout --force --detach <SHA>`) and
rebuilds its own inputs, salvador and the compression vectors, BEFORE the gates run. A failure in
that preparation is `exit 2` COULD NOT RUN, which is a different state from `exit 1`. So the
refresh is unconditional and upstream of the thing that could go stale, which is precisely the
ordering aeon's lane has inverted.

**Today's red is real, named, and is the lane doing its job.** 2026-09-06T05:19:41,
`SOURCE GATES FAILED at sigil e9a9dfa6 / aeon a7a4f640, 1 failed / 185 passed:
misspelled_objroutine_target_dangles_while_control_resolves`, with both revisions named and a log
path. It succeeded on 09-04 and 09-05, so it is hours old.

**Corroborated by a second instrument that did not know about the first**: seat TEST hit the same
test failing while running the one current binary it had against the live aeon tree, and
**correctly declined to report it**, on the grounds that its binary was from a different moment
than the tree. Two instruments, one failure, and the seat that could not attribute it said so
rather than banking it. It is a live finding for the aeon lane, routed with its path rather than
restated.
