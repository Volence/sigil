# Workspace cleanup, 2026-09-06 — sigil lane

Run at 2026-09-06T14:57:55Z on the owner's own words, relayed by the hub: *"can we get a cleanup of the
board and then a cleanup of the worktrees or whatever all this is"*.

## Result

| | before | after |
|---|---|---|
| hidden directories at `~/sonic_hacks/` | 227 | 122 |
| space held by them | 186.6 GB | ~14.7 GB |
| reclaimed | | **171.9 GB** |

## What was deleted, and the proof it was safe

84 cargo target roots, each one **proved by a positive content marker at the moment of
deletion**, not by its name: `CACHEDIR.TAG` (60) or `.rustc_info.json` (24). The assertion
re-ran inside the delete loop, so a directory that lost its marker between planning and
deletion would have stopped the run rather than being removed on a stale classification.

Cargo target contents are **derived data**: every byte is reproducible by rebuilding, and
nothing in them is a record of anything.

### The hazard this nearly hit, recorded because the rule that caught it is the deliverable

The first discriminator was *a directory named `debug`, `release` or `tmp` is build output*.
Run against the workspace it selected **`.aeon-sigil-stepgap/engine/debug`**, which is
**aeon source** (`engine/debug/debugger.asm` lives there), sitting in a stale worktree whose
registration had been pruned. A name-shaped rule would have deleted another lane's source
code and reported a clean sweep. **Name is not behaviour**: the shipped rule matches on a
file cargo itself writes, which no source tree carries.

## What was kept, and why

| kept | size | reason |
|---|---|---|
| `.pinned` | 7.5 MB | the pinned assembler this lane's freezes are cited against; no worktree list shows it |
| `.aeon-ref-relayout-master` | 63 MB | the reference tree at the provenance chain tip `aeon_rev` 483b3e12; the strict gate needs a paired aeon tree and this is it |
| `.sigil-orgfix-cb6504f1` | 13 MB | the parked read-only binaries the aeon lane reproduced this morning's byte-neutrality against |
| `.claude` | 24 KB | harness configuration |
| `.build-logs-kept/` | 70 MB | 216 log and evidence files that were sitting **inside** deleted target roots, copied out first |
| `.sigil-evidence/` | 686 MB | 26 loose probe and log directories, consolidated from the root into one |
| 6 registered worktrees of mine | ~2.7 GB | a worktree can hold uncommitted work; pruning them is a per-tree judgement, not a sweep |

**None of the three standing artifacts appears in any lane's worktree listing**, which is why
a prefix-driven sweep was the wrong instrument for this job.

## Not touched, and whose it is

113 directories belong to other lanes by prefix (`.oracle-*`, `.aurora-*`, `.aeon-*`,
`.parcel-*`, `.relayout-*`, `.reserve-*`). **110 of them are aeon's**, and they are the
largest remaining population at the root. They are aeon's to prune: a stale worktree can
hold uncommitted work only its own lane can judge. Routed to that lane rather than swept.

## The measurement that reframed the ask

The instruction said *worktrees*. **178.6 GB of the 186.6 GB was not worktrees at all**, it
was cargo build output; the registered worktrees hold 7.9 GB between all six repos. So the
space was never a worktree problem, and the deletion class turned out to be the safe one
(pure derived data) rather than the dangerous one (checkouts that may hold work).

