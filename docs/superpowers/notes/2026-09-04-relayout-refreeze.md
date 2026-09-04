# 2026-09-04 — the ROM re-layout refreeze, and the ratchet that blocked it

Aeon moved two ROM bank anchors to buy data headroom. **A `[[anchor]]` places nothing on
its own**, so the re-layout could not take effect until sigil's frozen tables moved. They
have. Chain 202 is frozen against aeon `5875e60e`.

Getting there required fixing a gate that refused correct code, and that fix is the more
durable half of this parcel.

## Identity

| | |
|---|---|
| aeon revision frozen against | `5875e60e5c5213b45b9e24059cd337a2ac22f394` (branch `parcel/rom-relayout-more-room`) |
| durability | `git ls-remote origin` at measurement time — tip of `refs/heads/parcel/rom-relayout-more-room`. NOT on `origin/master` |
| sigil base | master `48313276` |
| assembler | `sigil 0.1.0 (27489944)`, built into `/home/volence/sonic_hacks/.sigil-refreeze-target` |
| reference tree | `/home/volence/sonic_hacks/.aeon-relayout-freeze`, clean detached worktree at `5875e60e` |
| attest tree | `/home/volence/sonic_hacks/.aeon-attest-201`, clean detached worktree at the pinned `4f5ad5a1` |
| shared `target/release/sigil` | md5 `6c2378ae8a657e26684d4019a7d976d7`, unchanged throughout |

## The mechanism, confirmed at the build rather than taken on report

`native::load_frozen_table` reads `golden/offcanonical_sizes/<shape>.txt` out of the
sigil checkout **at run time**, `true_bases_by_index` builds every ROM section's
provisional base from those rows, and `map.toml`'s anchors reach the packing walk only as
a `HashSet<u32>` of addresses — matched by address, never by name — that *authorize* a
section to sit where the table already put it.

Building aeon `5875e60e` against the UNMOVED tables:

```
error: native build (sonic4 plain): [map.undeclared-island] ROM section at 0x90000
is an ANCHOR_GAP-inferred island but no `[[anchor]] at = 0x90000` is declared
```

The DAC bank had not moved. After the two rows moved, all four canonical shapes build and
`s4.lst` reads `Dac_Temp_Blip: A8000`.

## What moved by hand, and what was derived

`Dac_Temp_Blip 0x90000 -> 0xA8000` and `SoundTablesZ80_Head 0xA0000 -> 0xB8000`, in the
four SOUND-ON tables (`s4`, `s4_debug`, `config_a`, `lean`) — eight rows. `config_b`,
`demo` and `demo_debug` carry neither row; that was grepped, not assumed. Everything
downstream was re-packed by `derive_offcanonical_sizes.sh`, per the 08-26 precedent.

## THE CONTROL — the relayout's own share is exactly +0x18000

The raw before/after is CONFOUNDED. The committed goldens were frozen at aeon
`4f5ad5a1`, and `5875e60e` is **195 commits past it** (191 on aeon master, 4 on the
branch). A naive diff therefore mixes the re-layout with two days of engine content, and
it moves `config_b` — which the re-layout cannot touch.

The separating measurement: derive the tables TWICE at the SAME aeon revision, once with
the anchors and island rows reverted to `0x90000`/`0xA0000` in a second worktree.
Identical content; layout the only variable.

| shape | per-label delta, control -> new |
|---|---|
| `s4` | `+0x18000` ×8, zero ×60 |
| `s4_debug` | `+0x18000` ×9, zero ×71 |
| `config_a` | `+0x18000` ×9, zero ×77 |
| `lean` | `+0x18000` ×8, zero ×60 |
| `config_b` | zero ×68 — **nothing moved** |
| `demo` / `demo_debug` | zero ×40 / zero ×42 |

**Not one symbol moved by anything other than `+0x18000` or zero.** The `+0x18330` a
naive diff shows on the two DEBUG shapes (`Replay_OJZ_Fixture`, `BusError`, `EndOfRom`)
is `0x330` = 816 B accruing between `GameState_OJZScroll_Init` and `Replay_OJZ_Fixture`,
in the DEBUG-only region; aeon's diff carries a matching `ojz_scroll_test.emp` growth.

## The seven shapes

| shape | before (crc/size) | after (crc/size) | EndOfRom before -> after |
|---|---|---|---|
| `s4` | `14ee2440`/719700 | `6f047af2`/819123 | `0xa5c82` -> `0xbdc82` |
| `s4.debug` | `142294b3`/737683 | `d772f7d8`/840179 | `0xa81fc` -> `0xc052c` |
| `config_a` | `b9574a32`/738015 | `f598841a`/840531 | `0xa81fc` -> `0xc052c` |
| `config_b` | `46dc1eda`/615905 | `07002ea1`/618293 | `0x8c9ac` -> `0x8cea2` |
| `lean` | `6678ba60`/674816 | `7c8ec3e0`/773120 | `0xa4c00` -> `0xbcc00` |
| `demo` | `0c456778`/96474 | `3c5dcde6`/96602 | `0x1121a` unchanged |
| `demo.debug` | `2e603d53`/101339 | `36014485`/102818 | `0x1121a` unchanged |

The capture ran twice, hours apart and across a source change to `refreeze` itself, and
produced bit-identical CRCs for all seven.

**For the headroom rule:** `s4` `EndOfRom` = `0xBDC82` = **777,346**; `s4.debug`
`EndOfRom` = `0xC052C` = **787,756**. Both far under `0x100000` (1,048,576). If the rule
means the whole cartridge image rather than the assembled anchor, the full-file sizes are
819,123 and 840,179 — also under. Note that the hub's predicted 835,987 for the debug
shape matches neither figure; it is nearest the full file size, 4,192 bytes low.

## THE RATCHET — a check that refused correct code

`--freeze` refused the ledger append: tip #201 `corpus-pin-advance` carried no strict run.
So chain 201 was attested first, from a `provision-aeon-ref.sh` tree whose rebuild control
printed `MATCHES THE GOLDEN` for both s4 shapes and whose `repin --check` printed
`pins.rs unchanged` — the named positive witness. **That suite was green.**

`--attest` refused anyway: *"strict_bodies FELL from 30 to 29 since the last recorded
strict run ... Restore the gate, or say why it is gone: `--retired-strict-gates`."*

**No gate had been retired.**

* The baseline was `chain.entry.iter().rev().find_map(|e| e.strict.as_ref())` — the last
  RECORDED run, **with no filter on outcome**, fourteen lines above the code that computes
  `OUTCOME_FAILED` from `run.failed > 0`. The concept was already in the same function.
* That entry is #200 `tails-jump-gate`, `outcome = "failed"`, 12 failing. It is the ONLY
  one of the chain's 28 recorded strict runs to record 30; **#173 through #199 all record
  29**, across both outcomes.
* The declared strict-gate site population is **byte-identical** between #200's own sigil
  rev `64bc7158` and HEAD: 38 `if !strict_gate()` consultations across the same 12 files.
* The population census — the detector this module exists to be, built because "a gate
  going dark showed up as a SMALLER GREEN" — was green.

Because the chain is append-only, that 30 was a permanent floor no honest later run could
clear. The cheapest exit was a permanent ledger field asserting a retirement that did not
happen: **the damage was written into the remediation advice, not the check.** A rule
whose remedy is a false statement is worse than no rule.

**The fix** (`ratchet_baseline`) takes the last run whose `outcome` is `passed`, and stays
dormant while none has — the rule may not arm off a red run at all. Max-over-all-entries
was rejected: it would anchor to the same anomalous 30 permanently, the identical bug with
a monotonic face.

Red-first, with the mutation shown applied on disk (`git diff --stat` naming the file) and
restored from a committed baseline. Mutated back to unfiltered, the two new tests failed on
`left: Some(30), right: Some(29)` — the real anomaly — while
`the_strict_body_ratchet_fails_on_a_shrink_and_has_a_named_exit` still passed, showing the
mutation was targeted rather than a blanket break. The chain-fixture test asserts the
anomaly is STILL PRESENT before asserting the baseline skips it, so it fails loudly instead
of going vacuous if the fixture ever moves; it is bounded to the immutable prefix through
`tails-jump-gate`, so it cannot rot as the chain grows.

On the live chain: `refreeze --attest: strict-body ratchet: 29 -> 29, held.`

## The red attestation that was discarded, and why

The first `--attest` after the ratchet fix came back **RED on exactly one test**:
`harness_root::root_derivation::the_compile_time_manifest_dir_is_only_ever_displayed`
(4376 passed / 1 failed / 2 ignored / 0 `skip:` across 379 suites). The cause was mine —
the new chain-fixture test read `provenance.toml` through `env!("CARGO_MANIFEST_DIR")`,
and that gate forbids `refreeze.rs` and `repin.rs` from carrying any compile-time path,
the macro's absence being the whole proof. The gate was right and caught it.

That record was **uncommitted** and was discarded rather than filed. The reasoning, stated
so it can be disagreed with: entry #201's strict record answers "does the strict suite pass
on the goldens this tip names", and a defect in the *attesting* tree is not an answer to
that question. Filing it would have implied the tip's ROMs were suspect and forced a
spurious `--supersede-tip` abandonment of another lane's entry. The test was fixed to go
through `resolve_harness_root`, and the re-run recorded `passed`. This paragraph is the
record the ledger does not carry.

The distinction being relied on: discarding an *uncommitted* tool output after fixing a
defect in the runner is not the same act as editing a committed entry, which is the
forgery the provenance docs warn against. Nothing committed was altered.

## Booked, not fixed here

**`scripts/provision-aeon-ref.sh` refuses a legitimately pushed non-master revision.** Its
own comment states the intent as "REACHABLE FROM THE REMOTE, read with ls-remote"; the
implementation is `git merge-base --is-ancestor "$REV" origin/master`. `5875e60e` is
durable on the remote as a branch tip and was refused, so the freeze tree was
hand-provisioned to the same steps. Deliberately not fixed in this parcel — off the
critical path, and it should not be entangled with a freeze.

**A bare `git worktree add --detach` of aeon is not enough** for the DEBUG shapes: the
control tree's first derive died on `no module engine.compression_vectors` plus a run of
`[embed.not-found]`. That is the gitignored-artifact class, not divergence.

## Trees

`/home/volence/sonic_hacks/.aeon-relayout-freeze` (`5875e60e`) and
`/home/volence/sonic_hacks/.aeon-attest-201` (`4f5ad5a1`) are provisioned and kept. The
control tree carried a DOCTORED `map.toml` and was removed rather than left where a later
run could resolve to it.
