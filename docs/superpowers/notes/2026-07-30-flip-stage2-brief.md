# 2026-07-30 — FLIP STAGE 2 brief: THE FLIP (the point of no return; Volence-ruled GO)

Stage 2 of the ratified flip design, executed under the AMENDED SEQUENCE recorded
at the Stage-1 close (sigil `9f40dc6`, note `2026-07-30-flip-stage1-*`): the
S1.2 companion work first, then all six native==golden proofs, and ONLY THEN the
flip commit. Volence ruled Stage 2 GO on 2026-07-30; the overseer gates the
no-return commit itself.

## 0. Rulings in force (binding)

- **Stage 2 = GO** (Volence). **Demo flips in lockstep** (Volence). **The row-91
  DSM composition witness SURVIVES** (Volence — see §3). **sigil-frontend-as is
  PERMANENT.** Nothing from the old toolchain is retained in the build; the
  physical deletion of `tools/asl`/`p2bin` rides Stage 3 (keep the flip parcel
  focused); `convsym` SURVIVES (it consumes sigil's listing); the checksum lives
  in the sigil emit path (fixheader retires from the default path with the flip).
- Baseline: masters aeon `bcb8f64` / sigil `9f40dc6`; strict 2914/0 (1 ignored);
  the SIX frozen comparands in `crates/sigil-harness/golden/` (full-file blobs +
  assembled anchors; PROVENANCE.md). PRIMARY anchors: sonic4 e5765873/dab4f06c ·
  demo cfda98d3/20c5571d · config_a 3d9bac53 · config_b fd3f7f8e.
- Branch pair `flip-stage2` (sigil worktree `.worktrees/flip-stage2`; aeon
  worktree `.worktrees/flip-stage2` — fresh worktrees need NO seeding post-levelgen,
  only SIGIL_EMIT).

## 1. THE AMENDED SEQUENCE (strict order; the no-return protection)

1. **S1.2 companion** (aeon + sigil, every commit keeping the asl default
   byte-identical — the gates-ON arms are not exercised by the default build):
   delete/retire the gate else-`org` resume arms; split the AS residual at the
   natural section boundaries (byte-neutral to emitted bytes; linker granularity
   re-proven); migrate placement pins→map (`sigil.map.toml` becomes the placement
   surface; computed resume points; the $20000 object-bank budget as a map region
   check); kill rows 6/58 close here.
2. **The six-proof preamble**: extend the native driver to demo + Config-A/B (now
   buildable — the hardcoded resume orgs are gone); ALL SIX targets prove
   `native == frozen golden` at BOTH layers (assembled anchor + full file), with
   the S1.4 functional gate family and t24 controls on each. **CHECKPOINT to the
   overseer with the full 6×2 matrix — the no-return commit waits for the
   overseer's explicit go.**
3. **THE FLIP COMMIT(S)** (only after the go): `build.sh` (both games) drives
   `sigil build` (assemble→link→emit_rom w/ checksum→listing→convsym); the row-5
   `.asm` code twins DELETE — standalone plain-spoken deletion commits, grouped
   by subsystem, each naming its already-banked dual proof; the gate
   wrappers/includes die; the keystone code halves die and their sections flip to
   native-owned; the objdef.emp `engine.system.constants` import-id one-liner
   lands; the numeric ErrorHandler/Game_Entry equs resolve natively (rows 52/90);
   the 4 drift guards + the Stage-1 allowlist machinery retire with their twins.
   After the flip, the default artifacts are the SIGIL-CANONICAL full files
   (sonic4 2198deb2/395374 · 1d895fcb/402696; demo/config values pinned by the
   preamble gates) — the PRIMARY assembled anchors DO NOT MOVE. State the new
   artifact ledger explicitly in the close packet.

## 2. Test-suite transformation (with the flip commit)

- Windowed oracles' AS-twin-reassembly halves retire (no `.asm` to assemble);
  region gates re-comparand to frozen-golden slices (the seam-1 precedent).
- Whole-ROM gates re-comparand to the frozen goldens (drop the `== asl` clause —
  designed to be mechanical).
- t24 positive-control/negative-probe discipline preserved VERBATIM on every
  surviving golden gate (they are what stops the goldens going vacuous).
- Report the exact strict count at every boundary; the drop is expected and must
  be itemized (which tests retired, which transformed), not just netted.

## 3. THE ROW-91 EXCEPTION (Volence ruling — coverage kept)

The DSM in-memory composition of the `.emp` sound banks is an INDEPENDENT witness
of the sound stack and SURVIVES Stage 2. Its comparand may transform (frozen
golden slices instead of a live AS ROM), and its harness may keep using
sigil-frontend-as scaffolding as needed, but the composition-vs-reference
coverage must not be deleted or made vacuous. If preserving it requires design
choices, STOP and present them.

## 4. Standing discipline

- Deletion commits are STANDALONE and plain-spoken; each names its banked proof.
- Strict suite at every commit boundary, failures-first, explicit counts.
- Until build.sh flips: `./build.sh` + `DEBUG=1 ./build.sh` (+ demo positional)
  per aeon-touching commit. After: the sigil-build equivalents, both shapes, both
  games, vs the goldens.
- `verify_emit_bin.py` / `verify_level_bin.py` preflights: verify what they still
  check; retire verify_emit_bin only if its subjects are single-sourced (design
  §1.5), as its own commit.
- Kill-list rows updated same-commit as their closures (rows 5, 6, 18, 22-28,
  36-51, 52, 55, 58-69, 72-77, 79, 81-91 per the design §3.1 table; add rows for
  anything newly discovered).
- The valve stands; STOP on any guard relaxation, identity surprise, or design
  fork. Merges are the overseer's.

## 5. Close packet

The full proof matrix (6×2 pre-flip and post-flip), the new artifact ledger, the
itemized test transformation, the deletion inventory (files + line counts), the
kill-row closures, per-pass step-3 vs step-5 findings + neither-bucket headlines,
and the Stage-3 handoff (repin/asl-binary/p2bin removal, ownership flips,
game-constants .emp, debug-runtime, the ledgered post-flip rows).
