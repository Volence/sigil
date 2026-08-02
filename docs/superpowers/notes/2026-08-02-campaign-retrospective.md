# THE SPEC-2 CAMPAIGN RETROSPECTIVE (2026-08-02)

The look-back over the conversion campaign, written at its close. Facts below
are countersigned against the repos (research pass 2026-08-02 + spot-checks);
numbers carry their method where they could mislead. Masters at writing:
aeon `e03aad8` / sigil `ea686380` / oracle `250428c`; strict 2990/0/4;
provenance chain 22.

## §1 — What happened, in one paragraph

In roughly one month (2026-07-01 → 2026-08-02, ~2,760 commits across the two
masters), the aeon engine and game were converted from a 115-file / ~28,700-line
AS-assembler `.asm` tree to a 119-file / ~29,700-line `.emp` corpus compiled by
sigil — a from-scratch Rust assembler/compiler (~152k lines, 14 crates) that
became the ONLY toolchain: asl/p2bin/fixheader left the pipeline entirely (the
flip, 07-30), the ROM layout became a reviewed declared map (`map.toml`, the K
capstone), and the final arc (A1/A2/A3, tonight) removed the last cross-seam
sound residue and made the emit tool a consumer of the map authority. What
survives on the AS side is THREE files totaling 854 lines — two 24-line
`game_root` stubs and the vendored MD-Debugger (806 lines, its own ruled
replacement pending) — none of which emits a ROM byte or declares an org. The
whole way down, every step held a byte-identity or anchors-identity bar against
six golden targets, and the strict suite grew from ~0 to 2,990 tests.

## §2 — The numbers

| Metric | Start (≈07-01) | Close (08-02) |
|---|---|---|
| `.asm` files / lines (aeon) | 115 / 28,705 | 3 / 854 (zero bytes emitted) |
| `.emp` files / lines | 0 / 0 | 119 / 29,694 |
| sigil Rust LOC | (M0 core) | 152,491 across 14 crates |
| strict suite | — | 2,990 pass / 0 fail / 4 ignored |
| provenance chain | — | 22 frozen entries, ×6 goldens |
| register contracts | 0 | 459 `clobbers` / 130 `preserves` / 93 `out` |
| comptime guards | 0 | 268 `ensure` (49 cross-seam `extern` drift walls) |
| commits since 07-01 | — | aeon 827 (140 merges) + sigil 1,935 (191 merges) |

Line counts by `git ls-tree`+`wc` at `f0749b56` vs HEAD; the pre-July M0/M1
backend work predates the window.

## §3 — The phases (dates + headline)

M1 backend → byte-identical full ROM (07-03/04) · Spec-2 planning P3-P7
(07-05/08) · sound tranches + 68k port t0-t17 (07-08/16) · contract grammar
G1-G4.5 (07-17/18) · pass-2/3 optimize + clobbers census (07-21/22) · tranches
t18-t26 (07-23/28) · t27-t41: THE SOUND STACK — driver, sequencer, FM/PSG/SFX,
player keystone (07-29) · seam-1/seam-2 twin deletions (07-29/30) · THE FLIP
Stages 0-3: sigil IS the build (07-30) · §17 opt waves A/B/C (07-31) ·
conversion tail A-K + the K capstone: the declared map, main.asm/engine.inc
deleted (07-31/08-01) · the language round: L1 game contract, type layer
L5/L8/L9, checked-clobbers S2-D6 (08-01/02) · the modernization + lens sweep +
M-1 VBlank budget fix (08-02) · the A1/A2 seam-2 arc: mt_syms kill, registry
unification, `span` (08-02, overnight). Full table with artifacts in the
research annex (session record) and per-phase notes in this directory.

## §4 — What the language became

Shipped surface, roughly in dependency order: byte-exact
`data`/`proc`/sections with `@as_compat`; `jbra`/`jbsr` reach-relaxed
branches; modules + `use`; `ensure`/`ensure_fatal` + `extern()` link-asserts;
banks (`bank:`/`vma:`); overlay dispatch / scripted object states; the
register-contract grammar (`clobbers`/`preserves`/`out`/`falls_into`) with the
ISA-derived checked-clobbers lint and `@allow(reason)` escapes; structs +
`[Struct; N]` + `sizeof`/`offsetof` with per-field drift walls; `comptime fn`
+ `comptime test`; native compression builtins (s4lz/kosinski/nemesis);
newtypes (GridX/GridY/SectionId; SongId/SfxId; AnimId/MappingFrame;
VramTile/VramAddr); the `offsets` self-relative table construct; `embed()`;
`vars`/RAM regions with comptime-`if` fields, `alias()` reuse, `@align`;
the parallax DSL; the map manifest (order/anchors/holes/budgets — the
placement AUTHORITY); the L1 game contract (engine declares the typed `Game`
interface); `raise_exception`; `state_hash`; and tonight, `span(ProcName)`.
Deferred with reasons on file: mul_const (spec exists), L2/L7 human data-DSLs
(content-gated), `save_across`, dotted offsets targets, `base:` anchors, the
emit-tool struct-type gap (N-4 — the highest-value open row).

## §5 — The correctness harvest (the campaign's favorite children)

- **The VBlank overrun class, closed structurally** (M-1): riders provably
  uncharged against the DMA window; running-charge budget; floor −992 → ≥0;
  overload A/B proved OLD blew the window ~2× while NEW holds and drains next
  frame.
- **The gate-blind clobbers class C2, closed at the compiler**: a new mnemonic
  now FAILS COMPILATION until classified; 435-proc census delta empty; the
  doctored-a5 negative probe fails the caller's lint.
- **Cross-seam drift is a build error**: 49 `ensure(extern)` walls; per-field
  struct offsets; the map order-subsequence assert (A1); the revived id/ptr
  vol-env guard and measured spans (A3). Three formerly trust-me invariants
  became build errors in the final arc alone.
- **The negative-probe pattern** (doctored inputs must fail loudly) is now
  standing practice — ~141 notes reference it; every new gate ships with one.
- **Dup classes homed**: the 399-entry sound-constant mirror, the SFX
  quadruple mirror, the dual placement registry (A1) — each collapsed to one
  authority with guards, not conventions.
- Plus: 2 behavior bugs + 1 hardening from the lens sweep, ~150-250 cyc/frame
  recovered, the 69-file comment-truth purge, and the §17 wave optimizations —
  each with its own A/B evidence on file.

## §6 — What the process proved

- **The port loop** (byte gate → modernize → retrospect → back-prop →
  optimize, loop until dry-panel-adjudicated dry) survived contact with ~40
  tranches and produced the ledger discipline that made this retrospective
  writable from evidence rather than memory.
- **Twin lockstep + kill conditions**: every scaffold tracked with a kill row;
  the kill-list is now nearly empty (6 OPEN rows of ~140, mostly
  post-twin-retirement items).
- **Provenance as CRC32+size with a frozen chain** (22 entries): every
  byte-changing step is either identical, anchors-identical with a sanctioned
  appendix-only refreeze, or a STOP. The bank-anchor STOP rule was armed
  through the entire K/A arc and never tripped.
- **The overseer/porter split**: porters implement in isolated worktrees,
  overseer countersigns EVERY gate with own runs (the stale-pairing and
  stale-ROM traps were both caught by countersign discipline, twice each),
  merges are sequential with an origin precondition. Zero shared-checkout
  incidents since the rule hardened.
- **Failures-first verification** is not optional: both overnight process
  errata (a piped exit code, a stale demo.bin) were exactly the classes the
  standing rules name.

## §7 — What remains open (honest)

Top of the ledger: the emit-tool struct-type gap (N-4); the
children-never-freed dynamic-slot leak (engine bug, out-of-campaign-scope);
`preserves` accepting never-written proof; `save_across`; the parser
error-loop robustness bug; the own diagnostics runtime to replace the vendored
debugger (Volence-ruled post-twin-retirement — the twins are now retired); the
repin `.lst`-parse retirement (row 34). The step-5 optimization backlog
retains the large majority of the 2026-07-16 review's 29 sections (the §17
waves consumed the top of the priority order; the Wave-4 sound size-reclaim
and the player cluster are untouched). None of these blocks the next era.

## §8 — The next era (the direction question for Volence)

The platform is finished; the game is not started. `games/sonic4` has ONE act
(OJZ) and twelve objects, all `test_*` — no badnik, no boss, no real level.
The tools that would author content sit at: **aurora** (the level/sprite
editor) dormant since 07-01; **seraph** (the tracker/DAW) half-built, last
commit 07-16; **oracle-next** the most active sibling (214 commits, hardware
conformance suite) but explicitly not yet the daily driver. The old
sonic_hack goals (per-act level data, object migration, Tile/Block/Chunk) are
now trivially expressible in `.emp` + the map — what's missing is content and
the authoring loop. The realistic options, with the overseer's ranking, are in
the era brief delivered with this retrospective; the call is Volence's.

## §9 — Close

The campaign set out to replace an assembler and ended up replacing a
toolchain, a layout regime, and a trust model: from "the bytes are right
because nobody touched them" to "the bytes are right because the build proves
it, and here is the chain." 2,990 tests, 22 frozen links, three surviving
`.asm` files, zero unexplained bytes. The campaign is closed.
