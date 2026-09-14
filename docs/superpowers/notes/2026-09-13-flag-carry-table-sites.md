# The carry-table consumers' sites, measured before the tables changed (2026-09-13)

Ledger row: `FLAG-CARRY-TABLE-NOT-ISA-DERIVED`. The carry tables in `flag_check.rs`
(`consumes_carry`, `z80_reads_carry`, `writes_carry`, `z80_writes_carry`,
`writes_ccr_operand`) have two consumers, and both are build-gated through
`sigil build`'s contract gate:

1. `[call.flag-result-unused]`: the must-use walk (`abandons_flag`, through
   `TrackedFlag::consumed_by` / `redefined_by`), both CPUs.
2. `[call.result-invalid-path]`: `Cfg::invalid_edge`, which walks from a call to the
   first branch testing the guard and bails on `writes_carry(.., M68000) ||
   writes_ccr_operand(ops)`, 68k only.

This note is the census of both, taken with the tables as they were, so that a
re-derived table cannot add a firing nobody has looked at.

## Where and with what

- sigil `a11e33a7` (master at dispatch), branch `parcel/flag-carry-isa-derived`, tables
  unchanged.
- aeon `ec640bcf`: the reference tree `/home/volence/sonic_hacks/.aeon-sigil-ref` the
  landing gate reads (clean, four ROMs built).
- aeon `55c062a41645c6e2853e74665f0f31fdf4d263c0`: aeon's published tip, read with
  `git ls-remote origin refs/heads/master` at measurement time. Exported with
  `git archive` into a scratch directory; the export lacks the gitignored
  `engine/debug/generated/compression_vectors.emp`, which `corpus_sources` requires, so it
  was generated there by the tree's own `tools/gen_compression_vectors.py`, with the
  `tools/bin/salvador` binary copied from the reference tree (the export has no build
  products; nothing else was added).
- The instrument is `corpus_flag_results_are_all_consumed` (seven shipped shapes, the four
  `build.sh` shapes among them), run `--exact --nocapture` under `SIGIL_STRICT_GATE=1`,
  plus a temporary trace (not committed): for every flag-result call site, the
  instructions its walk reaches and what each did to the tracked flag (`C` consumed,
  `W` redefined, `.` walked through, `RET` / `FALLOFF` / `OUT` for the exits); for every
  invalid-path site, each instruction from the call to where `invalid_edge` stops.

## Census before the change

Per shape: must-use firings / carry sites walked (Z80 of them); invalid-path firings /
sites walked / sites with no guard branch. Identical at both revisions except where
marked.

| shape | must-use firings | carry walked (Z80) | invalid-path firings | invalid walked | no guard |
|---|---|---|---|---|---|
| sonic4 plain | 0 | 38 (28) | 0 | 23 | 0 |
| sonic4 debug | 0 | 39 (28) | 0 | 23 | 0 |
| demo plain | 0 | 37 (28) | 0 | 23 | 0 |
| demo debug | 0 | 38 (28) | 0 | 23 | 0 |
| config_a | 0 | 39 (28) | 0 | 23 | 0 |
| config_b | 0 | 38 (28) | 0 | 23 | 0 |
| lean | 0 | 38 (28) | 0 | 23 | 0 |

The census lines as printed at `ec640bcf` (the zero sites and discards are not part of
this row, and are here for the site totals):

```
census `sonic4 plain`: 0 firing(s) over 77 site(s): 45 walked (28 Z80), 9 discarded, 0 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `sonic4 debug`: 0 firing(s) over 83 site(s): 48 walked (28 Z80), 12 discarded, 0 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `demo plain`: 0 firing(s) over 76 site(s): 44 walked (28 Z80), 9 discarded, 0 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `demo debug`: 0 firing(s) over 82 site(s): 47 walked (28 Z80), 12 discarded, 0 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `config_a`: 0 firing(s) over 83 site(s): 48 walked (28 Z80), 12 discarded, 0 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `config_b`: 0 firing(s) over 77 site(s): 45 walked (28 Z80), 9 discarded, 0 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `lean`: 0 firing(s) over 77 site(s): 45 walked (28 Z80), 9 discarded, 0 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
```

At `55c062a4` the carry and invalid-path columns are the same; the zero sites fall by 2
in every shape (43/46/42/45/46/43/43 walked, 5 or 7 of them zero) and the discards rise
by 1 (10/13/10/13/13/10/10).

## What the walks reach (the engagement)

The trace, summed over one shape's walked carry sites (`sonic4 plain`, `ec640bcf`; the
other shapes differ only in how many `bcs` sites they build, and `55c062a4` is the same):

- consumers, where each path is decided: `bcs` x8, `bcc` x2 (68k); `jr c`/`jr nc` x18,
  `jp c`/`jp nc` x2, `ret c`/`ret nc` x8 (Z80);
- walked through before the consumer: `movem.l (sp)+, ...` x3 (`Perform_DPLC`,
  `Perform_DPLC_Deferrable`, `PageIn_EnqueueLanding`, each then `bcs`), `pop hl` x2
  (`Seq_Op_Jump`, `Seq_Op_PsgNoise`, each after `call Snd_ChanClass`), `ld a, (hl)` x1
  and `inc hl` x1 (`Seq_Op_PsgNoise`, before its `jr nc`);
- no path reaches a redefiner, a return or the end of the body.

So every walked carry site is decided by a carry-testing branch, and the instructions
walked through on the way are `movem`, `pop hl`, `ld` and 16-bit `inc`. None of them is
in the ledger's list of disagreeing rows: the `pop` is `pop hl`, not `pop af`.

The 23 invalid-path sites (`AllocDynamic`, `AllocEffect`,
`TileCache_FindStagedBlock`, all `a1 if eq`) each meet their guard as the very next
instruction: 4 `beq` (the guard's own sense) and 19 `bne` (its negation). No instruction
sits between any of these calls and its guard, so `invalid_edge` consults the carry
table at no instruction at either revision; its behaviour on the corpus cannot move with
the table. That also answers the sibling row
`RESULT-INVALID-PATH-GUARD-CLOBBER-IS-CARRY`'s open count: of the 23 walked sites, none
has an instruction between the call and the guard.

## Prediction for the change

A re-derived carry table moves neither rule on aeon at either revision, since no walk
reaches a changed row. The after-census must show the same walked counts, the same
per-site traces, and zero firings; an empty diff here is backed by the walked counts
above, and a planted abandonment (below, after the change) shows the census can fire.
