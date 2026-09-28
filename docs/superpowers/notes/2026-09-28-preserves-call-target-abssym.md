# PRESERVES-CALL-TARGET-ABSSYM

Booked by `2026-09-28-inout-proof-invoke-hook.md` ("Booked, not fixed"). Sigil base
`a9086d5b`. Reference tree `/home/volence/sonic_hacks/.aeon-sigil-ref` (aeon `ec640bcf`,
the last `golden/provenance.toml` entry).

## The answer, plain

`preserves.rs` had a private call reader, `call_target` (last `Sym` operand of any arity,
no `$` filter). It fed the verified-`preserves` call transfer (whose oracle verdict feeds the
closure's upgrade round, the word-facet gate and the cond-out-survives check) and the
`[proc.dead-save]` walk. A bound `invoke Game.hook` lowers to `jsr (Bound).l`, a sole
`AbsSym`, which `call_target` could not see: both consumers charged it as an unknown callee.
That reader is removed; both consumers now use `flag_check::direct_proc_target`, the reader
`calls.rs`, `out_verify.rs` and `type_slice.rs` already share.

Nothing moves in the corpus. The four ROM shapes and the four `--report contracts` outputs
are byte-identical between base and fixed.

## The three axes, decided per consumer

Both consumers use the returned name only as a key into the closure's `effective` map, and
both answer a miss the conservative way (`callee_clobbers` reads an absent key as
clobber-all: the preserves oracle refuses, the dead-save walk marks the save needed).

| axis | old `call_target` | `direct_proc_target` | verdict for both consumers |
|---|---|---|---|
| kinds: `AbsSym` | not read (unknown callee) | read | gained. `jsr (Bound).l` enters `Bound`'s entry, exactly like `jsr Bound`, so `Bound`'s `effective` describes it |
| kinds: `SymOff` | not read | not read | kept out. `jsr Item.field` enters mid-body and can skip `Item`'s own save, so `Item`'s contract does not describe it; for a worklist of saves to delete this is the safe side. (`transfer_target_sym`, the fold target the ledger's kill condition names, does read it; that is why this parcel did not fold onto it) |
| position | last `Sym` of any arity | sole operand | inert: the consumers only ask for `jsr`/`jbsr`/`bsr`, which take exactly one operand |
| `$` filter | `$`-mangled local returned | `None` | added. `jbsr .L` runs code inside the caller, not a proc's entry. The real map is keyed by source-spelled proc names, and `lower/hygiene.rs` `local_symbol` makes the `$` spelling unspellable from source, so the old `Some(mangled)` was always a lookup miss; `None` gives the same answer as a rule rather than a coincidence |

Tail charges are unchanged: `checkpoint_after_tail` reads `transfer_target_sym`, which already
recognised `AbsSym`.

## Red-first

`crates/sigil-frontend-emp/tests/preserves_invoke_hook.rs`, 11 tests.

- On unmodified base `a9086d5b` (commit `test:` on the branch): 7 passed, 4 FAILED.
  `invoke_of_preserving_hook_verifies_preserves` (left `NotPreserved`, right `Verified`),
  `save_around_invoke_of_preserving_hook_is_dead` (left `[]`, right `[(D5, ["Bound"])]`),
  `local_helper_is_not_credited_by_preserves` (left `Verified`, right `NotPreserved`),
  `local_helper_save_is_never_dead` (left `[(D5, ["$m$P$helper"])]`, right `[]`). The two `$`
  tests put the mangled key in the map on purpose; they pin the rule, not a live corpus defect.
- With the fix: 11 passed.
- Mutation A, on disk in `flag_check.rs`: the `AbsSym` arm replaced by
  `// MUTATION A: AbsSym arm removed`. 9 passed, 2 FAILED (the two invoke-credit tests).
- Mutation B, on disk: `[CodeOperand::Sym(name)] => Some(name.as_str()), // MUTATION B: $ filter dropped`.
  9 passed, 2 FAILED (the two local-helper tests). Each restored with
  `git restore --source=HEAD`.

## Byte movement

Binaries: `sigil-base` built from `a9086d5b` sources (the fixed tree with `preserves.rs`
restored from `a9086d5b`; `git diff a9086d5b -- crates/` otherwise touched only the new
test), `sigil-fixed` from the fix commit. `sigil build --aeon <ref> -o <own path>`.

| shape | base | fixed |
|---|---|---|
| sonic4 | `91c46c94/820209` | `91c46c94/820209` |
| sonic4 `--debug` | `8a378de6/846509` | `8a378de6/846509` |
| demo | `1c7a34d3/96863` | `1c7a34d3/96863` |
| demo `--debug` | `72e405a5/103185` | `72e405a5/103185` |

All eight equal the `ec640bcf` provenance entry. `sigil build --report contracts`: identical
(`cmp`) for all four shapes; the dead-save worklist is the same one row
(`TestChurnObj_Main a0 bracketing AllocDynamic`).

Reach: the fix does meet real sites. sonic4 binds `ring_collected`, `spring_launched` and
`solid_pushed` (`games/sonic4/config/game.emp:152-163`), so five `invoke` sites
(`collision.emp` 446, 555, 575, 597; `rings.emp` 368) now resolve to a proc. None sits inside
a save bracket or decides a `preserves` verdict, so no output changes. demo binds none of them.

The dead-save walk does not edit code: it produces the D1d worklist a porter cuts from. A
change there shows in `--report contracts`, not in ROM bytes.

## Full suite

`cargo test --workspace --no-fail-fast`, `AEON_DIR=.aeon-sigil-ref`, `SIGIL_STRICT_GATE=1`,
`CARGO_TARGET_DIR` inside the worktree, at the fix commit `9b7d8506`: 519 result lines (506
test binaries plus 13 doc-test legs), 5839 passed, 7 failed, 2 ignored. Both failing binaries
are environment, not this change, and each went green on a targeted rerun on the same tree:

- `sigil-harness --test m1b_gate`, `oracle_loadfromaslisting_resolves_emit_listing`: no
  `ORACLE_DIR` was named, and the resolver refuses a derived sibling checkout. Rerun with
  `ORACLE_DIR=/home/volence/sonic_hacks/oracle-old`: 5 passed.
- `sigil-harness --test scripts_name_their_tree`, 6 tests: `find_named` skips only a
  directory named `target`, so with the target dir at `target-parcel/` it also found a
  fixture copy of `suite_paths.py` another test had left under `target-parcel/tmp/`. With that
  fixture directory removed: 7 passed. A target dir inside the worktree under any name other
  than `target` trips this.

## Still open

- The preserves TAIL charge reads `transfer_target_sym`, which accepts `SymOff`: a
  `jmp Item.field` tail is charged `Item`'s `effective`. That is the mid-entry question in the
  crediting direction. Corpus-inert today (ledger: no corpus call or tail carries a `SymOff`).
- The closure's `resolve_callee_key` maps a dotted `Owner.label` callee to `Owner`;
  `callee_clobbers` in preserves does not, and reads a miss (the conservative side).
