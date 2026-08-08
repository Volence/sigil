# HANDOFF — 2026-08-08, contract-verification arc CLOSED and MERGED

Everything below was verified at handoff time, not remembered. **Re-derive anything
you build on** — that rule produced every catch of this session, including two
against the overseer's own claims. Commands inline.

## FIRST — read the boilerplate

`docs/superpowers/notes/porter-brief-boilerplate.md`, in full. **Known stale in one
place:** it names TWO gitignored aeon seed paths; there are FOUR (see TRAPS).

## STATE (re-derive: `git -C <repo> log --oneline -1`, `git rev-list --count origin/master..master`)

| | master | unpushed |
|---|---|---|
| sigil | `0376cb6d` | **27** |
| aeon | `adee20c` | **3** |

- **THE PUSH IS VOLENCE'S GATE. It was offered and NOT given. Do not push.**
- aeon's working tree has 5 dirty entries (`M .gitignore` + 4 untracked
  `docs/research/2026-08-07-*`) belonging to a **CONCURRENT SESSION**. Leave them.
  Never `git add -u` in either main checkout.
- Merged master, own-run: **strict 3548 / 0 / 4 = 3552**, equal to master's own
  `#[test]` total. 8 byte gates present and green. `refreeze --check` OK, **chain 53**.
  clippy `--workspace --release --all-targets -- -D warnings` clean. Byte-NEUTRAL —
  the ROM is identical to the chain-53 tip.
- **`[proc.out-unverified]` residue: 30 → 16.**

## WHAT LANDED

Four merges, aeon first (a measured constraint — see TRAPS):

1. **`outw`** — the parcel. The out verifier now charges a claim at the width its
   TYPE declares. `out(dN: T)` is satisfied by a write T-wide or wider; bare
   `out(rN)` still means 32 bits, so **no existing declaration changed meaning**.
   14 rows closed, 0 opened, byte-neutral.
2. **`triage`** — the residue census (30 rows, four causes, width class is exactly
   HALF not "dominant").
3. **`probe2`** — the `.cl_hanging` verdict.
4. **`s4lz`** — the `out(a1)` root verdict + a second CFG blind spot.

Two design facts worth carrying, both paid for by four review rounds:

- **An out claim answers TWO widths, not one.** `OutClaim { strict, credit }` —
  widest for the proc's own obligation, narrowest for anything drawing on it —
  because those consumers have OPPOSITE fail-safe directions. Invariant
  `credit <= strict`, enforced by **field privacy** (the `debug_assert!` is compiled
  out under `--release`, which is the gate that matters).
- **A write produces only the bits it covers.** `ext` writes the half ABOVE what it
  reads; the single-bit forms write one bit. Neither discharges a claim over bytes
  it never wrote. `Scc` is NOT in that family (`seq.b` writes all 8 bits).

## THE QUEUE — ranked, all disjoint

**1. `S4LZ_Decompress::a1` — fully specified and measured, do this first.**
Verdict CONTRACT-ONLY: a stream decompressing to zero bytes never writes `a1`, so
the value returned is the caller's own pointer — correct but not *produced*. Fix:
retire `out(a1)` → `clobbers(a1)` on all three procs. Measured set delta: GONE =
exactly `{Art_Decompress::a1, S4LZ_Decompress::a1, S4LZ_DecompressDict::a1}`,
NEW = `{}`, residue 16 → 13. **When it closes, re-run the site-2 `falls_into`
plumbing mutant and confirm it goes RED. If it stays GREEN that is a FINDING, not a
footnote** — it was already armed once under simulation and fired correctly, with
TWO gates catching it. Full verdict:
`docs/superpowers/notes/2026-08-07-s4lz-out-a1-root-verdict.md`.

**2. The `inout` facet — DESIGN PASS FIRST, do not let a porter build it.**
Proposed independently by two lanes for the threaded cursor/counter shape
(`DrawRings`, `InsertSpriteMasks`, and `S4LZ`'s `a1`). **The naive form is PROVEN
VACUOUS** — it would verify `InsertSpriteMasks`' `addq.b #1, d5`. It must compose
across calls, with `TileDelta_Undo` as the corpus exhibit forcing that rule, and it
needs a ruling on whether `inout` implies anything about `clobbers` membership the
way `out(rN if cc)` does. Worth 4 rows.

**3. The two CFG blind spots.** Both fail-safe (they over-fire; neither can bless a
false contract), so this is precision, not urgency.
- A block entered only by a local `bsr`/`jbsr` is UNREACHABLE to `out_verify` in
  both directions. Worth 8 rows — but **width types and local-`bsr` credit each
  close ZERO alone; together they close all 8.** A lane expecting one to move them
  will misread the non-move.
- A computed intra-proc dispatch (`jmp .table(pc,Xn)`) is modeled as a transfer OUT
  of the proc, so obligation sites appear that are not return paths. 6 sites / 5
  procs; only `S4LZ_Decompress` declares an out, so 4 are latent.

**4. Z80 output contracts — the largest genuine gap left.** **27 declared Z80 outs
across 6 modules, ZERO production checking of any kind, none on any baseline.**
Corrects an older framing: the `falls_into` exemption is *inert* on Z80, so it is
not the hazard — the ABSENCE of any check is. Tractable: `z80_writes` and
`z80_edges` both already exist.

**5. The sprite-counter widening — byte-CHANGING, its own parcel.** FOUR
`addq.b #1, d5` sites across THREE procs (`sprites.emp:583`, `:589`, `:760`,
`rings.emp:232`) increment a byte while nine call sites read `.w`. Correct only
because `moveq #0, d5` clears the long and `MAX_VDP_SPRITES = 80`. Widen the
increments (same encoding, same cycles), then adopt `out(d5: u16)` on all three.
**`out(d5: u8)` is the WRONG adoption** — it closes a row by publishing a byte
contract to word-consuming callers.

## VOLENCE'S OPEN GATES — do not act on these without him

1. **THE PUSH** — 27 sigil / 3 aeon.
2. **The arc-A play-test** (timing-only change, cleanly revertible).
3. **The `Section_RedrawPlanes` clamp question** — an ORACLE experiment, not a
   reading. The left tracker is clamped (`cmp`/`bge`); its twin `d7` is assigned
   unconditionally, both under one comment saying "Clamp to cache range". Every
   sibling site in the engine spells the right edge `min(x, Cache_Head_Col)`. The
   cache is WIDER than the plane (80 vs 64 columns), so the tracker claims columns
   the loop never painted. **Not called a bug**: the plane is a 64-cell ring, so
   over-claimed columns may already be correct by wrap. Ledgered with the settling
   experiment.

## TRAPS — every one of these cost real time

- **MERGE ORDER IS CONSTRAINED AND MEASURED: aeon first, or together. NEVER sigil
  first** — sigil-first REDs `contract_baselines_hold_for_every_shipped_shape` on
  master with 13 new rows. Verified by execution twice.
- **The aeon worktree seed is FOUR gitignored paths**, not the two the boilerplate
  names: `games/sonic4/data/editor/`, `engine/debug/generated/`,
  `engine/sound/generated/`, `games/sonic4/data/sprites/pitcher_plant/`. **Enumerate
  with `git status --ignored --short | grep '^!!'` rather than trusting any written
  list, including this one — it has now grown four times.** Prove the seed by
  building byte-identical BEFORE any edit.
- **A strict run without `AEON_DIR` exported silently measures the MAIN aeon
  checkout** and fails for a reason that looks like a real defect. It has bitten a
  reviewer here. `capture_goldens.sh` needs `SIGIL_EMIT` **and** `SIGIL_BUILD`.
- **The shell cwd RESETS to the main checkout between tool calls.** Prefix every
  command with an explicit `cd` and print `pwd`.
- **A suspiciously fast gate is a PROVENANCE question before it is a result** — ask
  which tree and which binary before reading the number.
- **After master moves ahead, `git diff master..branch` shows MASTER'S own content
  in reverse.** A docs-only branch will look like it touches 13 crate files. Use
  `git log master..branch` and diff from the merge-base.
- **`git checkout -- <file>` / `git checkout HEAD -- <file>` are FORBIDDEN in a lane
  worktree.** Revert probes by string-replace and prove it with `git diff`.
- **Worktree naming:** `sigil/.worktrees/collision` holds the `triage` branch. Check
  `git rev-parse --abbrev-ref HEAD` before assuming which lane you stand in.
  `outw`, `probe2`, `s4lz` worktrees exist in both repos and are all merged now —
  safe to remove.
- **Read-only lenses must be told explicitly not to mutate**, and a lens panel is
  dispatched only against a CLEAN worktree with the review SHA named.

## STANDING BARS RATIFIED THIS ARC

- **A gate over ONE consumer of a multi-consumer value proves NOTHING about the
  others.** The same defect recurred three times, one layer down each round, because
  each round's gate covered the consumer that round was thinking about. Round 3's
  fix was correct code with NO gate at all — flipping the read left the entire
  2341-test suite green. Enumerate every consumer by grep; state per consumer which
  direction is conservative FOR IT; require the gate to exercise every one. Treat a
  charge-site enumeration in a spec as a **soundness artifact** — anything missing
  from the list is something nothing is checking.
- **"Name the mutant, run it, and record the EXACT mutated string."** One claimed-RED
  mutant turned out spelling-dependent: the obvious spelling was inert and left the
  suite green.
- **Read measurements as SET DIFFS, never counts.**
- **The lens panel is not optional.** Four rounds, and every round found real
  defects that ALL gates missed — including two soundness holes with executed
  differentials. The lane's own gates were green every single time.
- **Two-sided adoption**: a contract must state what the body produces AND what
  callers can soundly consume. Where those disagree, ESCALATE — do not pick the one
  that closes the row.

## AUTHORITATIVE RECORDS

- `docs/superpowers/notes/campaign-gap-ledger.md` — tail is newest, every row carries
  a kill condition. **Resolve rows by SUBSTANCE (grep the claim), never by a row
  number you were handed.**
- `docs/superpowers/specs/2026-08-07-out-type-width-design.md` — the width ruling.
- `docs/superpowers/notes/2026-08-07-outw-packet.md` — the parcel.
- `docs/superpowers/notes/2026-08-07-out-residue-fixpoint-census.md` — the census.
- `docs/superpowers/notes/2026-08-07-probe-core-out-residue-verdict.md`
- `docs/superpowers/notes/2026-08-07-s4lz-out-a1-root-verdict.md`
- `crates/sigil-harness/src/contract_baseline.rs` — the ONE baseline copy.
