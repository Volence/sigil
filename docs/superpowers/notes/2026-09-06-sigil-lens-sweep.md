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
