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

## After the change (sigil `fe0ca8e3`, the derived table and its tests)

Same instrument, same trees. The census lines are identical to the table above at both
revisions, 0 firings from either rule in every shape. The per-site trace (every
instruction every walk reached, and its verdict) is byte-identical to the one taken
before: 562 lines at `ec640bcf`, 555 at `55c062a4`, `diff` empty both times. So the
prediction held, and the empty diff covers 38 or 39 walked carry sites and 23 walked
invalid-path sites per shape, not a walk that reached nothing.

A third consumer of the walk the table feeds, found while closing: the verified-credit
recomputation of the invalid-path check (`flag_firings_verified_credit`), which
`corpus_flag_results_declared_vs_verified_credit_agree` holds equal to `flag_firings`.
It walks the same bodies with the same table, so it cannot move where the census did not.

## The census can fire (positive control)

A scratch copy of the `55c062a4` export with two plants, `diff -r` against the export
showing exactly these three lines:

- `engine/sound/sound_sequencer.emp:1740`, in `Seq_Op_Jump`: `pop hl` after
  `call Snd_ChanClass` made `pop af`, so the `jr c` tests the F restored from the stack;
- `games/sonic4/test/object_test_state.emp`: `addq.l #2, a2` inserted between
  `jbsr AllocDynamic` and its `bne .emitters_done`, and `move.w (a1), d0` inserted after
  `.emitters_done:`, the invalid (`ne`) edge.

With the derived table, every shape reports both:

```
CENSUS2 `sonic4 plain`: must-use firings 1, carry walked 38 (28 Z80); invalid-path firings 1, walked 23, no guard 0
FIRING `sonic4 plain` @5544: [call.result-invalid-path] `GameState_ObjectTest_Init` calls `AllocDynamic`, whose `a1` result is valid only where `eq` holds, and reads `a1` on the path where `eq` does not hold. Read `a1` only on the `eq` path, or redefine it first
FIRING `sonic4 plain` @98136: [call.flag-result-unused] `Seq_Op_Jump` calls `Snd_ChanClass` and abandons its `carry` result `music` on some path: the flag is redefined, or the proc returns, before anything reads it. Consume it (a conditional branch on `carry`) before it is redefined, or mark the call `@discards(music)` if dropping it is intended
```

With the tables as they were (the pre-change `flag_check.rs` restored into the working
tree for one run), the same tree reports neither, in every shape:

```
CENSUS2 `sonic4 plain`: must-use firings 0, carry walked 38 (28 Z80); invalid-path firings 0, walked 22, no guard 1
TRACE mu|Z80|Seq_Op_Jump|Snd_ChanClass|carry|@98136|fired=false|pop [Z80Pair(Af)]=.; jr [Z80Cc(C), ...]=C
TRACE ip|GameState_ObjectTest_Init|AllocDynamic|a1 if eq|@5544|walked=false|fired=false|addq [Imm(2), Reg(A2)]=W
```

So both firings come from the derived rows: the old gate walked through the `pop af`,
and stopped the invalid-path walk at an address-register ADDQ that leaves the CCR.

## The table change, checked mechanically

A temporary test (not committed) held verbatim copies of the five old functions and
compared them with `carry_role` over 430 spellings: every 68k family through the string
path with data- and address-register operands, the 18 condition spellings in the Bcc,
Scc and DBcc forms, ANDI/ORI/EORI/MOVE to CCR and SR at seven immediates, MOVE from SR,
and the `.emp` words; every Z80 mnemonic with the eight conditions, PUSH/POP of four
pairs, and both EX forms. 61 changed and 369 did not, and every change falls in a class
the commit `37426527` lists: address-register MOVE/ADD/SUB/ADDQ/SUBQ, ANDI/ORI/EORI to
CCR or SR by bit 0, DBLO/DBHS, MOVE from SR, RTE, EXTB; Z80 ADC/SBC/RLA/RRA/RL/RR/DAA,
CCF, POP AF, EX AF, AF', PUSH AF, SLL.
