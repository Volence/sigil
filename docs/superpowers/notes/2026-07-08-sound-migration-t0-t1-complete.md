# Sound-migration T0+T1 COMPLETE — awaiting Volence checkpoint

2026-07-08 (Fable, controller). Design → plan → implementation all same day. Branches, both UNMERGED pending checkpoint:
- **sigil** `sound-migration-t0-t1` (worktree `.worktrees/sound-migration-t0-t1`), 10 commits `79156bb..9e2bce4` off master `9dd4843`.
- **aeon** `sigil-emp-dac`, 3 commits (`4782cde` tools --emit-bin, `916f763` dac_samples.emp, `c0ef67a` main.asm gate) off master.

Read first: the design (`specs/2026-07-08-sound-migration-tranche1-design.md`, DSM.1–9) and the plan's `## Execution notes` (`plans/2026-07-08-sound-migration-t0-t1.md`) — the notes carry every delta, the Task-4 premise correction, and the collision-bin incident.

## What shipped

**T0 (language gaps):** `u16le` cells; `bank:`+`vma:` reject (L7.5 done in code); **`equ` items end-to-end** (the .emp→.asm export mechanism: AST → lazy eval → `ir::Section.equ_syms` → folded post-placement in `resolve_layout` → defined before fixups in `link()`); AS `db`/`dw` general unresolved-expr deferral (`Value8`/`Value16Le` trees; no silent masking anywhere); `winptr()` re-expressed over link-exprs (**L7.3 DISCHARGED**, byte-diff-clean proven); both-direction seam probes green; aeon emitters `--emit-bin` + verifier (24/24 byte-equal, no interior labels — T2/T3 embeds will be clean).

**T1 (the DAC port):** `games/sonic4/data/sound/dac_samples.emp` — 10 embeds, two `(bank: $8000)` sections, all 30 `SND_*` equs; consumers (`engine/sound/dac_sample_tab.asm`) untouched. `SIGIL_EMP_DAC` gate in main.asm, byte-proven inert for asl. **The acceptance: `mixed_dac_rom.rs` builds the mixed .asm+.emp ROM byte-identical to `s4.bin` AND `s4.debug.bin`** (same convsym allowlist as m1d, zero new entries). Negative probes: straddle/length/dup/overlap all loud, all falsified.

## Verification state

Workspace FULLY GREEN (~1429 tests, strict gate), clippy `-D warnings` clean, `corpus_bytediff.sh` all-identical, m1d + m1d_debug + mixed×2 + dac_port + ports all green, aeon asl build byte-identical (sha256 vs PROVENANCE pins). Two-stage reviews on Tasks 3/4/7/9 (all approved; one Important fixed in `447aa8c` — assert_rom_matches dedup). **Whole-branch adversarial review: checkpoint-ready, zero real defects** (hunted: equ-fold vs relaxation/bank-bump ordering, range edges, partial-resolve trees, le-flag threading, DEBUG-shape composition, exhaustive SND_* consumer sweep; independently re-derived all 30 values + spot-checked descriptor bytes in the reference ROM at $605AD).

## Open items for the checkpoint

1. **⚠ Collision-generator drift (aeon, pre-existing, surfaced this session):** running aeon's tools pytest suite regenerates untracked `games/sonic4/data/collision/*.bin` with content that DIFFERS from what the current ROMs bake. We restored the originals from the reference ROM bytes ($2C1FA..$2E3FA). Decide: adopt the new generator output intentionally (rebuild + re-baseline) or pin the inputs. Until then, any full tools-suite run re-clobbers.
2. **`movingtrucks_pitchtable.asm` hand edits** (SndDefaultPitchTable alias + assert) get clobbered by `--emit-pitchtable` regeneration — reconcile before T2 touches the MT bank.
3. **Production map home** for the zero-byte equ carrier (`text`) when aeon's real build cuts over to the sigil map (test maps park it at LMA 0, enforced zero-byte).
4. Spec integration (Fable, empyrean working tree, at checkpoint per practice): `equ` item + `u16le` (§ additions), L7.3/L7.5 ledger dispositions, DSM rows already in the sigil design doc. Note empyrean's tree still carries the uncommitted #7 integration.

## T2 pointers (the MT streaming bank, next tranche)

The `ensure(bankid(X) == SND_ENGINE_TABLE_BANK, ...)` cross-source assert pattern is proven (ports.rs Probe B). `--emit-bin` outputs exist for every stream. The MT bank co-locates the .asm engine-table head + streams — the .emp side will pin AFTER the head; the DEBUG-conditional members (DrumTest, HCZ2) mean TWO build shapes per DSM.8. Interior labels: none (verified), so streams embed cleanly.
