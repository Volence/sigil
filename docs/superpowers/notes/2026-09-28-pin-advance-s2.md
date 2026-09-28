# PIN-ADVANCE-S2-ENVELOPE-RESTATEMENTS: sigil's reference pin, aeon ec640bcf to 1ee78b88

Branch `parcel/pin-advance-s2`, cut from master `ee4089f8`. No emulator was used.

## The reference tree

`scripts/provision-aeon-ref.sh /home/volence/sonic_hacks/.aeon-sigil-ref-1ee78b88 1ee78b88`
with `REF_BUILD_DEMO=1` (all four shapes BUILT, not copied), assembler built from this tree
(`sigil 0.1.0 (ee4089f8)`, closure-revision `88eb2e35`), target dir `.target-ref` in the
worktree. `PROVISION_END rc=0`. HEAD `1ee78b88b003da5ffbd8fa061eab7d56505c201a`, `git status`
clean (only ignored build products). zlib CRC-32 + size, recomputed with python `zlib.crc32`:

| file | CRC-32 / size |
|---|---|
| s4.bin | `50401674/845398` |
| s4.debug.bin | `198fd717/865722` |
| demo.bin | `2f9e2d4c/99994` |
| demo.debug.bin | `64717bac/106981` |

The two s4 figures EQUAL aeon's own claim (s4 `50401674/845398`, s4.debug `198fd717/865722`,
built on sigil pair `1173bb31`), although this assembler is `ee4089f8`, not `1173bb31`.

The jump is 2161 aeon commits (`git rev-list --count ec640bcf..1ee78b88`), not the handful the
queue row's heads-ups describe. `c06c786b` (parallax round 2) is an ancestor of the target.

`tools/test_extern_guard_reachability.py` IS PRESENT at `1ee78b88` (absent at `ec640bcf`), so
aeon's pytest lane can write into any tree holding their `tools/`. Reported, not acted on. The
provisioner builds with `NO_LINT=1`, which skips that lane.

## Baseline, before any change

- `refreeze --check`: OK, tip `link-zero-byte-move-placement`, chain len 206 (the two known
  DIVERGENT sigil revisions #181, #201). The chain is healthy, so the advance is a DECISION
  (aeon named the revision, hub steer), not a repair. Log: `refreeze-check-before.log`.
- `repin --check` against the new tree: rc 2, `region test_particle plain_anchor: symbol
  ObjDef_PathSwap not found in the listing symbol table`. aeon deleted the path-swap object
  (`2dc0e9da`). Log: `repin-check-before.log`.
- Full suite, `scripts/landing-run.sh --aeon <new tree>` (strict, `--no-fail-fast`, target
  `.target-ref`), at unmodified `ee4089f8`: 520 of 520 binaries launched and reported,
  5657 passed, 202 failed, 2 ignored, CARGO_EXIT 101, clippy 0, ledger 0. Wall clock 7m47s.
  Failing tests by binary: `baseline-failing.txt`; first panic line of each:
  `baseline-panics.txt` (a few lines there are `should_panic` tests that passed).
