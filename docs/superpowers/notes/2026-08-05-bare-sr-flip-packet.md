# bare-sr-flip micro-lane packet — 2026-08-05

Executes the overseer ruling on ledger row "[sr-split, 2026-08-05] Bare `preserves(sr)`
claims both halves…": the four bare-`sr` adopters whose CCR half is false flip to the
honest `preserves(sr.mask)`, and `irq.emp`'s bracket-adopter prescription is amended.
Branch `bare-sr-flip` in both worktrees, cut from sigil `d52a3ab1` / aeon `655ff86`.
NOT merged, NOT pushed.

## Per-proc verdicts (each body re-verified before flipping — none left bare)

| proc | file | verdict | body evidence |
|---|---|---|---|
| `VSync_Wait` | `engine/system/vblank.emp:273` | **FLIPPED** | `.wait` loop (`tst.b`/`beq`) + `move.b d0, VBlank_Flag` run AFTER the bracket's restore — CCR at `rts` is the wait loop's, never the caller's. Mask half true: the one bracket round-trips. |
| `Sound_PlayMusic` | `engine/sound/sound_api.emp:190` | **FLIPPED** | per-iteration `z80_stopped` brackets with `tst.b d1` after each release; `andi`/`subq`/`lsl` id arithmetic between brackets; the final `ints_off` save is mid-proc, so its release restores mid-proc flags, not entry flags. Mask half true. |
| `Parallax_Update` | `engine/level/parallax.emp:345` | **FLIPPED** | the mode-3 bracket is mid-proc; `.mode3_reg_live` onward is all flag traffic (`moveq`/`move.b`/`beq`/band loop) to a bare `rts`. Mask half true. |
| `GameState_OJZScroll_Init` | `games/sonic4/test/ojz_scroll_test.emp:69` | **FLIPPED** | the marker-tile bracket is followed by `jbsr` boot calls (callee flags) and `move`/`st` traffic to `rts`. Mask half true. |

The three honest bare adopters stay bare, re-confirmed against their bodies:
`Sound_PostByte` (whole body inside the bracket), `Sound_Init` (hand save first,
restore last; the DEBUG watchdog and `raise_error` sit inside the pair), `BG_Init`
(only `movem`/`movea` outside the hand pair).

Header prose truth-fixed in the same token pass (present-tense contract facts, no
history narration): `VSync_Wait`'s precondition note now says `preserves(sr.mask)`
and states the CCR is not the caller's; `Sound_PlayMusic`'s "SR restored" became
"interrupt mask restored" with the CCR fact.

## irq.emp amendment

`engine/irq.emp` header, `ints_off` paragraph: an adopter declares the claim its
WHOLE BODY earns, not the one the bracket alone would — `preserves(sr.mask)` when
any instruction outside the bracket writes flags on a path to return (including a
mid-proc save restoring mid-proc flags), bare `preserves(sr)` only when every CCR
effect sits inside the pair. Either spelling's mask half is enforced by the spliced
save/restore. The existing hand-spelled-sites paragraph (declare the halves when CCR
does not round-trip) already agreed and is untouched.

## Verifier proof of the flipped claims

`check_preserves_sr` (crates/sigil-frontend-emp/src/lower/proc.rs) runs for
`preserves(sr.mask)` exactly as for bare `sr` — the same whole-SR round-trip slice,
dispatched at proc.rs:1311 whenever the mask half is claimed. Non-vacuous for all
four (each body splices/spells the save/restore pair, so SR writes exist and the
slice must find them bracketed). Every build shape compiles with zero errors and no
`[proc.preserves-sr-unbalanced]` / `[proc.preserves-unverifiable]` — all four flipped
claims are machine-proven, none landed on a refusal.

## Advisory detector: NOT enabled — the ledger's "zero new analysis" claim is false in DEBUG shapes

The row proposed running `ccr_bracket_refusal` advisory (warn tier) over bare-`sr`
adopters. Verified against the source instead of assumed: the walk classifies `jmp`
as `CcrEffect::Leaves` — an unconditional refusal — and the `raise_error`/`assert`
desugar ends in `jmp (pages).l` (eval/diag.rs:605). So the advisory would
false-positive on `Sound_Init` (honest adopter; DEBUG watchdog `raise_error` inside
its bracket) and on every assert-carrying bare-`sr` proc in DEBUG shapes. Enabling
takes: (1) a noreturn/diverging-tail model in the walk (a desugar `jmp` never
returns to the caller, so its flags never reach one); (2) a new warn id plus a
DEBUG-shape warn-tier baseline row; (3) a fixture audit — lower_proc.rs's
unbalanced-SR fixtures would double-fire error+warning, diag_assert_vector.rs's
`preserves(sr)` vector proc and the diag_desugar parity fixtures all carry bare `sr`;
(4) a detector test. Left for the parcel that builds the noreturn model, or S2-D7's
dataflow half, which subsumes it. Ledger row amended with this finding, same commit
as this packet.

## Bars

1. **Byte bar: ALL SEVEN targets byte-identical.** Fresh builds in
   `capture_goldens.sh` order, `cmp` against the frozen golden blobs: `s4.bin`,
   `s4.debug.bin`, `demo.bin`, `demo.debug.bin`, `config_a.bin` (via `--config-a` →
   `s4.debug.bin`), `config_b.bin` (`--config-b` → `s4.bin`), `lean.bin` (`--lean` →
   `s4.bin`); canonical `s4.bin` + `s4.debug.bin` rebuilt and re-cmp'd OK afterwards.
   Contract tokens emit nothing — confirmed. (Note for the next lane: the worktree's
   warm `target/release/sigil` predated the sr-split grammar and failed the aeon
   parse with "expected `)`, found Dot" on `sr.mask` — rebuilt at `d52a3ab1` first.)
2. **refreeze --check OK** (tip `objtest-gate`, chain len 46); **repin: pins.rs
   unchanged**; sigil tree stayed clean.
3. **Warn tier: id sets AND counts unchanged**, measured both sides of the flip
   (stash/build/pop on my own lane worktree): config_a 60 (`proc.sr-undeclared` 42,
   `module.path-mismatch` 9, `proc.undeclared-fallthrough` 5, `proc.out-unwritten` 3,
   `proc.clobber-undeclared` 1), config_b 19, lean 18 — identical pre- and post-flip.
4. **Full strict `SIGIL_STRICT_GATE=1`**: 3321 `#[test]`s — **3317 passed / 0
   failed / 4 ignored** across 309 suites (failures-first sweep of the full log:
   zero FAILED/panicked lines). Delta 0 from the `d52a3ab1` base, as expected —
   no detector assertion was added (see above).
5. **Lens C (hazard, read-only): all four flips STATE THE TRUTH of their bodies —
   none should have stayed bare.** Per-proc: mask half proven on every return path
   (including Parallax_Update's three return shapes and its SR-clean Step5/Step4
   tail pipeline — file-wide, line 429 is parallax.emp's only SR site), CCR half
   genuinely false in each. Call-site sweep clean: no caller consumes a pre-call
   flag after any of the four (load_art.emp's `bcs` correctly precedes its
   `jbsr VSync_Wait`). irq.emp prose verdict: correct, matches all seven adopters.
   **One actionable finding, ledgered not fixed (out of the ruled scope):**
   `Section_RedrawPlanes` (section.emp:209) declares `clobbers(…, sr)` while its
   body earns `preserves(sr.mask) clobbers(sr.ccr)` (mask round-trips :213→:439,
   only post-restore CCR traffic), and `Section_UpdateColumns` declares no SR
   clause while calling it — the transitive truth of OJZ-Init's flipped claim rests
   on body-stronger-than-contract in that chain. New ledger row
   `[bare-sr-flip lens C, 2026-08-05]`, which also names the related
   preserves-through-tail-transfer demand (dma_queue's Critical/Important
   siblings, already documented at dma_queue.emp:80).

## Merge-order note for the overseer

Sigil-side changes are **docs-only** (this packet + the ledger-row close); no sigil
code or test changed, no pin moved. The aeon commit carries all behavioral truth
(five .emp files, contract tokens + prose). No cross-repo build coupling was
introduced, but the standing rule stands: merging this two-repo parcel stales every
other in-flight lane's aeon worktree — refresh before their next gate runs.
