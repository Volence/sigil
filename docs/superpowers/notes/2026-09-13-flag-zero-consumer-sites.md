# Flag-result sites with no consumer model, measured before the model (2026-09-13)

Ledger row: `FLAG-RESULT-NONCARRY-UNCHECKED`. This note is the measurement the row's kill
condition asks for: every call site to a callee declaring a flag result other than
`carry`, classified by reading the code after the call, taken BEFORE a consumer model
for that flag exists, so the build gate (which refuses every `FlagFiring` since
`eb0b3915`) never meets a site nobody has looked at.

## Where and with what

- aeon `cb4805b5` (`origin/master`, read with `git ls-remote` on 2026-09-13), provisioned
  at `/home/volence/sonic_hacks/.aeon-flag-noncarry` by `scripts/provision-aeon-ref.sh`.
- sigil at `1532b72f` (tool closure `2d45fa39`), i.e. master before this parcel: the
  census below is the pre-change `FlagSiteOutcome`, from `corpus_flag_results_are_all_consumed`
  with a temporary per-site print (not committed), `AEON_DIR` pointed at the tree above.
- The site population is the census the must-use loop records (`check_flag_unused_sites`),
  so it counts calls in the evaluated CodeBufs of each shape, comptime arms resolved.

## The declared population

Declarations were enumerated with `git grep` at `cb4805b5` for every flag name the grammar
accepts (`VALID_FLAGS` in `lower/proc.rs`: `carry`, `zero`, `negative`, `overflow`,
`extend`), in `proc` and `extern proc` headers, both CPUs:

| flag | 68k procs | Z80 procs |
|---|---|---|
| `carry` | 7 | 12 |
| `zero` | 3: `Load_Object` (`zero: success`), `Section_GetSecPtrXY` (`zero: none`), `Parallax_Active_Config` (`zero: inert`) | 0 |
| `negative`, `overflow`, `extend` | 0 | 0 |

So `zero` is the only non-carry flag with a population, and it is 68k only. No Z80 call
site owes a non-carry result.

## Census lines before the model (from `--nocapture`)

```
census `sonic4 plain`: 0 firing(s) over 78 site(s): 38 walked (28 Z80), 10 discarded, 7 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `sonic4 debug`: 0 firing(s) over 84 site(s): 39 walked (28 Z80), 13 discarded, 9 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `demo plain`: 0 firing(s) over 77 site(s): 37 walked (28 Z80), 10 discarded, 7 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `demo debug`: 0 firing(s) over 83 site(s): 38 walked (28 Z80), 13 discarded, 9 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `config_a`: 0 firing(s) over 84 site(s): 39 walked (28 Z80), 13 discarded, 9 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `config_b`: 0 firing(s) over 78 site(s): 38 walked (28 Z80), 10 discarded, 7 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
census `lean`: 0 firing(s) over 78 site(s): 38 walked (28 Z80), 10 discarded, 7 no consumer model, 23 invalid-path walked, 0 no guard branch, 0 unknown register
```

Every "no consumer model" site is a `zero:` site (the per-site print listed no other
flag). The debug-family shapes (`sonic4 debug`, `demo debug`, `config_a`) carry 9, the
others 7, which matches the ledger's census at `dbb67085`.

## The sites

All ten are 68k. CONSUMED means the first instruction the path reaches after the call
reads Z, per the M68000PRM Table 3-19 conditional tests (`EQ` = Z, `NE` = not Z).

| # | caller | callee | result | file:line | shapes | classification | deciding instruction |
|---|---|---|---|---|---|---|---|
| 1 | `Vscroll_Write` | `Parallax_Active_Config` | `zero: inert` | `engine/level/parallax.emp:1497` | all 7 | CONSUMED | `beq .whole_plane` (1498) |
| 2 | `Parallax_CheckBoundary` | `Section_GetSecPtrXY` | `zero: none` | `engine/level/parallax.emp:1183` | all 7 | CONSUMED | `beq .no_crossing` (1184) |
| 3 | `Parallax_InstallScratch` | `Parallax_Active_Config` | `zero: inert` | `engine/level/parallax.emp:4010` | debug family only (`if DEBUG == 1`) | CONSUMED | `beq .refuse` (4011) |
| 4 | `Section_RedrawPlanes` | `Section_GetSecPtrXY` | `zero: none` | `engine/level/section.emp:489` | all 7 | CONSUMED | `beq .plb_use_act_layout` (490) |
| 5 | `EntityWindow_BuildEntries` | `Section_GetSecPtrXY` | `zero: none` | `engine/objects/entity_window.emp:721` | all 7 | CONSUMED | `beq .void_entry` (722) |
| 6 | `EntityWindow_TrySpawnObject` | `Load_Object` | `zero: success` | `engine/objects/entity_window.emp:1287` | all 7 | CONSUMED | `bne .gated` (1288) |
| 7 | `Enqueue_Dirty_Buffers` | `Parallax_Active_Config` | `zero: inert` | `engine/system/buffers.emp:505` | all 7 | CONSUMED | `beq .hs_done` (506) |
| 8 | `GameState_OJZScroll_Init` | `Section_GetSecPtrXY` | `zero: none` | `games/sonic4/test/ojz_scroll_test.emp:904` | all 7 | CONSUMED | `beq .init_use_act_config` (905) |
| 9 | `GameState_OJZScroll_Update` | `Load_Object` | `zero: success` | `games/sonic4/test/ojz_scroll_test.emp:484` | debug family only | CONSUMED | `bne .objreq_full` (485) |
| 10 | `Load_ObjectList` | `Load_Object` | `zero: success` | `engine/objects/load_object.emp:116` | all 7 | DISCARDED (`@discards(success)`) | the walk does not run; the code after it (`jbra .loop` then `move.l a1, d0`) would redefine Z unread, which is what the attribute says is intended |

Counts: 9 sites without `@discards`, 7 of them in every shape and 2 (#3, #9) in the
debug family only; all 9 CONSUMED by the very next instruction. No ABANDONED site, and
no consumer shape a simple model would not know: every consumer is a `beq`/`bne`
immediately after the call, with no instruction in between, so no site depends on how
the model classifies a redefiner.

Two observations about the population itself, recorded rather than acted on:

- The census walks the whole tree for every shape, so the demo shapes count the
  `games/sonic4/test/ojz_scroll_test.emp` sites (#8, #9). That is a property of the corpus
  walk, not of the zero sites.
- The ledger's "9 code call sites without `@discards` (a grep of the three names)" and
  this census agree on the nine, and the census adds the shape split: a grep cannot see
  that #3 and #9 sit in comptime arms only the debug family builds.
