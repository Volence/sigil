# Sound-migration T2 (MT streaming bank) — COMPLETE, awaiting Volence checkpoint

Written 2026-07-08 at the end of the T2 implementation session. **Both branches are UNMERGED
— NO merge without Volence's checkpoint.** This note is the checkpoint packet + next-session
orientation.

## What shipped

Plan (authoritative, incl. every deviation in `## Execution notes`):
`docs/superpowers/plans/2026-07-08-sound-migration-t2-mt-bank.md` (rulings R1–R7).

**sigil branch `sound-migration-t2`** (worktree `.worktrees/sound-migration-t2`, off master
`2b11acb`, 13 commits `6c03265..402c9b6`):
- **Comptime defines** (`-D NAME=INT`, `LowerOptions.defines`) — the AS `-D __DEBUG__` mirror
  for `.emp` build shapes; `[defines.collision]` covers ALL item names incl. proc/script;
  every lowering-path evaluator seeded (loud unknown-name at the few non-item entry points).
- **imm32 operand deferral** in sigil-frontend-as (`try_defer_long_imm`): unresolved
  `movea.l/move.l #Sym, aN/dN` → placeholder-0 through the REAL encoder + `Value32Be` fixup
  @ offset 2; imm8/imm16/branches/other mnemonics/memory dests stay loud (pinned by 12 tests).
  Resolved path byte-neutral (all reference gates re-verified).
- **Capability probes** (11 tests, all instant-green — the language needed NO new features for
  the port) + **mt_port.rs** region byte-gate + **mixed DAC+MT full-ROM acceptance** (2 new
  tests in mixed_dac_rom.rs, both build shapes, `assert_rom_matches` with the m1d allowlists,
  5-ensure count pinned) + **mt_negative_probes.rs** (straddle / wrong-bank / table-length /
  missing-define all loud, each falsified).

**aeon branch `sigil-emp-mt`** (off master `e5b256c`, 6 commits `34fcf68..9302751`):
- `games/sonic4/data/sound/mt_bank.emp` — the port: 6 stream embeds, SongTable/SongPatchTable
  as `[*u8; SONG_COUNT]` if-expressions, asl-align-equivalent conditional pads (HCZ2's fires:
  6511-byte odd blob + $00 pad), MT_PITCHTAB_OFFSET detune ensure, 5 co-residency ensures via
  `bankid("MovingTrucks_Bank_Start")` (bare cross-seam equ reads don't exist — documented
  deviation), `bank: $8000` subsumes all four straddle/window-top fatals.
- `main.asm` `ifndef SIGIL_EMP_MT` gate (org-resume $63AE8 plain / $6553A debug); SONG_* id
  equates moved to `config/sound_ids.asm` (R2 — kills the moveq cross-seam need); six stream
  `.bin`s committed with narrow gitignore exceptions (verify_emit_bin: all match dc.b twins).
- asl build byte-verified UNCHANGED four times (plain `8ce6dd7e…`, debug `13c7b063…`).

## Verification state

- Mixed `.asm`+`.emp` ROM (BOTH gates on): **byte-identical both shapes** (modulo the same
  convsym/fixheader allowlist as m1d). The imm32 deferral resolves `#SongTable/#SongPatchTable`
  to $63AE0/$63AE4 (plain) and $65522/$6552E (debug) — byte-inspected.
- Full workspace: green (`cargo test --workspace`, ~1445+ tests), clippy `-D warnings` clean,
  all strict-gate harness tests green.
- Two-pronged whole-branch adversarial review: **both prongs checkpoint-ready, zero Critical.**
  Fix-first items (5-ensure count pins, single-lower refactor, diff context, SONG_* mirror
  annotation) all landed (`402c9b6` / `9302751`).

## Recorded-not-fixed (carry forward)

1. **Regen tripwire gap (arc-level, T1 shares it):** `tools/verify_emit_bin.py` is manual-run
   only. Regenerating a song's `.asm` WITHOUT its `.bin` twin drifts the asl ROM while sigil
   gates stay green against a stale reference until a PROVENANCE rebuild. Candidate fix: wire
   verify_emit_bin into the harness or a pre-commit + twin-regen requirement in generated-file
   headers. Raise at checkpoint.
2. **Defines are program-wide reserved names** under `--root` (right for T2, mirrors AS -D;
   footgun at public-release scale) — carry to the empyrean spec-integration pass (Fable owes
   T2's pass anyway: defines + imm32 deferral + the ensure-spelling limitation below).
3. **Bare cross-seam symbol reads in `.emp` exprs don't exist** — only `bankid("Label")`-style
   builtin calls take link symbols. mt_bank.emp works fine with the label substitution, but the
   sketch-level idiom `ensure(x == SOME_ASM_EQU)` is a language gap someone will hit again.
   Spec-integration candidate (S2-D14 adjacent).
4. imm32 deferral scope is deliberately movea.l/move.l-to-bare-register only.
5. Misleading "internal: … anchor label" diagnostic when BOTH sides of a link ensure are
   unresolved (standalone no-map compiles only) — diagnostics-quality nit.

## Checkpoint asks for Volence

1. **Boot-check the bg restore** (separate pending item from earlier today — see
   `notes/2026-07-08-next-session-handoff-bg-restore-then-t2.md` STATUS block), then commit
   aeon's working tree (bg files) on master.
2. **Merge decision for T2**: sigil `sound-migration-t2` → master (--no-ff, house pattern) and
   aeon `sigil-emp-mt` → master. Note the aeon branch is off `e5b256c` and does NOT contain the
   bg working-tree changes — merge order (bg commit first, then the branch) or rebase is
   Volence's call; the sound files are disjoint from the bg files either way.
3. The regen-tripwire gap (item 1 above) — adopt a guard now or defer?

## Next after merge: T3 (SFX tranche)

The last sound tranche: 9 SFX blobs + patch banks + sfx_table as `.emp`, the two main.asm SFX
fatals (239/247) → ensures, and the DSM.7 win-tab call (sfx_blob_win_tab lives INSIDE the .asm
phase head — likely stays .asm this arc; the LE u16 cell is probe-proven regardless). T2's
patterns cover everything new T3 needs except: sfx_table.asm is BUILD-GENERATED every run
(prebuild sfx_transcode.py generate) — the .emp equivalent needs the generator to emit/refresh
the .emp or the table to be hand-owned; design-check with Volence recommended before starting.
