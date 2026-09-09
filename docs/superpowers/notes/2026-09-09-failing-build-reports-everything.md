# Landing d-28: a failing build reports what an ordinary pass found

2026-09-09. Branch `parcel/failing-build-reports-everything` off master `27db72a7`.
Owner ruling d-28, `report_everything`. This note is the landing measurement; the
decision's own measurement is `2026-09-08-failed-run-bonus-pass-measurement.md`,
which is re-run here rather than inherited.

## The change

`eval::run_impl`'s post-convergence gate becomes

```rust
let already_failed =
    !carried_fatals.is_empty() || diags.iter().any(|d| d.level == Level::Error);
if !force_relocate && (poison.is_empty() || already_failed) { … }
```

and the ordinary arm now reports the converged pass's leftover `poison` the way
the bonus branch reports its own. That second half is not tidiness. The message
`unresolved symbol \`X\` in operand` had EXACTLY ONE emit site in the whole crate,
inside the bonus branch, so an arm that skips the bonus pass without adding the
push loses every one of those errors. Measured: with the push removed, five of
eleven corpus roots go QUIETER than master (181 diagnostic lines vanish), which
is the one direction the nine-root measurement forbade. Nothing else in the repo
sees it; see "What the gates can and cannot see".

## The guarded-site population, re-derived by symbol

The note cites seven `keep_labels_symbolic()` sites at `eval.rs` 5725, 6394,
6472, 6863, 6896, 7133, 7980 at master `6f86cb90`. Those coordinates are dead:
`eval.rs` has since taken the integer-selector parcel and the `charset`
threading. Re-derived by symbol at `27db72a7`, and it is STILL SEVEN, the same
seven functions, only moved:

| fn | line now | shape |
|---|---|---|
| `directive_equate` | 6009 | `if self.keep_labels_symbolic() {` |
| `directive_dc_w` | 6824 | `&& self.expr_refs_label(&qed)` |
| `directive_dc_l` | 6912 | `&& self.expr_refs_label(&qe)` |
| `lower_m68k` | 7499 | `let ft = … && self.expr_refs_label(&target)` |
| `lower_m68k` | 7532 | `&& self.expr_refs_label(&qualified)` |
| `try_defer_long_imm` | 7769 | `let refs_label = …` |
| `fixup_target` | 8616 | `&& self.expr_refs_label(&qualified)` |

The count is stable, but the brief's framing of the surface is worth sharpening:
`keep_labels_symbolic()` is `self.defer_unresolved_jsr_jmp`, the SAME field the
`jsr`/`jmp` deferral arm reads at 7487. So the suppression surface is not "seven
sites plus a second mechanism"; it is eight consumers of ONE flag, and the flag
is set on exactly one pass. That is why gating the pass gates all eight at once.

## The roots, re-enumerated

The note's stated method (every directory directly under `/home/volence/sonic_hacks/`,
kept if it holds a top-level `sonic|s1|s2|s3|sk.asm` or a `build.{lua,bat,sh}`)
DOES NOT PRODUCE ITS OWN NINE. Batman's root is `The Adventures of Batman and
Robin/disasm/batman.asm`: one level deeper, and named by none of those stems. The
method as written finds neither it nor MD-OS's `_Assemble.bat`. Re-run at depth 3
it also finds two roots the note does not have, `AP Backups/Awesome Project` and
`AP Backups/Malevolent Hack`, both `win32/asw -xx -c -E -A s2.asm`, i.e. genuine
AS front ends. So the sweep here is ELEVEN roots, the note's nine plus those two.

The two additions turn out to contribute nothing: both `s2.asm` files are not
valid UTF-8, so the front end never opens them (`cannot read s2.asm: stream did
not contain valid UTF-8`) and the pass loop is never reached. They are kept in
the table anyway so that no root is silently dropped.

Corpora are PRIVATE `rsync -a --exclude .git` copies under this worktree's
`.scratch/corpus/`. No shared checkout was read during a measured run.
Revisions, with dirt, inherited equally by both sides: s1disasm `f6ece65` (4),
s2disasm `e45ebf3` (0), s2disasm-mompass-clean `e45ebf3` (0), skdisasm `2fcd861`
(2), sonic_hack `858af72` (17), S.C.E. `d2f1988` (0); Batman, MD-OS and the two
AP Backups trees are not git repositories.

## The sweep, compared in both directions

Exact-line multiset compare of stderr, `Counter(base) - Counter(cut)` and the
reverse, so an equal-size swap cannot pass. Stdout compared whole.

Instruments: base `sigil` at `27db72a7` md5 `02b1c129666d53cafb508d03d94c9507`;
cut `sigil` at `451cb3e2` md5 `695a76ee4c52021d7a49031fb3f9d6fb`.

| Root | exit b/c | lines base | lines cut | vanished | appeared | new classes | stdout |
|---|---|---|---|---|---|---|---|
| s1disasm | 1/1 | 1 | 1 | 0 | 0 | none | same |
| s2disasm | 1/1 | 5136 | 5136 | 0 | 0 | none | same |
| s2disasm-mompass-clean | 1/1 | 5136 | 5136 | 0 | 0 | none | same |
| skdisasm sonic3k | 1/1 | 367 | 367 | 0 | 0 | none | same |
| skdisasm s3 | 1/1 | 197 | 197 | 0 | 0 | none | same |
| sonic_hack | 1/1 | 3080 | 3083 | **0** | **3** | **`unresolved long expression`** | error count only |
| S.C.E. | 1/1 | 87 | 87 | 0 | 0 | none | same |
| Batman | 1/1 | 9466 | 10700 | **0** | **1234** | none | error count only |
| MD-OS | 1/1 | 908 | 908 | 0 | 0 | none | same |
| AP Awesome | 1/1 | 1 | 1 | 0 | 0 | none | same |
| AP Malevolent | 1/1 | 1 | 1 | 0 | 0 | none | same |

**Containment holds: 0 lines vanish anywhere, at the level of exact lines rather
than counts.** The two stdout differences are the `assembly failed: N errors`
line following its own stderr.

The difference is REPORTED, not asserted by size. sonic_hack's three, in full:

```
code/engines/art_data.asm(23):7: error: unresolved long expression
code/engines/art_data.asm(24):7: error: unresolved long expression
code/engines/hud.asm(1408):3: error: unresolved symbol `LoadLevelLayout` in operand
```

Lines 23 and 24 are `dc.l (plc1<<24)|art` and `dc.l (plc2<<24)|map16x16` in the
`levartptrs` macro. Line 25 (`dc.l (palette<<24)|map128x128`) stays clean, which
is the whole mechanism visible in one macro: `PLCID_Ojz1`/`PLCID_Ojz2` are
`id(PLCPtr_Ojz*)` equs that never resolve while `PalID_OJZ` does, and the art
pointer beside each is a real label, so `expr_refs_label` is true and the fold is
`Poison`, the `keep_labels_symbolic` short-circuit exactly. Batman's 1234 are all
`unresolved symbol \`X\` in operand` on `jsr`/`jmp` targets like `loc_005406.l`,
an operand syntax the front end reads as one symbol name.

The compare instrument was canaried before its zero was believed
(`.scratch/canary.py`): it asserts its input line counts, then shows it fires on
a removed line, on a multiplicity change, and on an equal-size swap that a count
check waves through.

Sonic 1's profile is not what the note measured (1 diagnostic here against 50
there): its front end is now clean and the run reaches layout, where it stops on
colliding pins. Expected, and a reason the sweep was re-run rather than quoted.

## Timing, which is NOT the justification

The owner decided the reporting question. These numbers are recorded only so a
reader is not left guessing, and they are NOT comparable to the note's: load1 ran
10.84 to 23.20 (median 19.57) here against 2.89 to 4.77 there. 7 interleaved
reps, child user+sys CPU, page cache warmed once per root. Medians, cut/base:
s1disasm 0.957, s2disasm 0.750, s2disasm-mompass-clean 0.686, skdisasm sonic3k
0.866, skdisasm s3 0.979, sonic_hack 0.797, S.C.E. 0.898, Batman 0.672, MD-OS
1.061, AP roots 0.901/0.944. The two millisecond-scale roots and the two
non-UTF-8 ones carry no signal at any load.

## What the gates can and cannot see

Established by breaking the change on purpose, three mutations, each proven to
have landed on disk (anchor count 1 before, 0 after) and each restored from the
commit rather than over a dirty tree.

| Mutation | new test | workspace suite | four-shape CRC | corpus containment sweep |
|---|---|---|---|---|
| M1 revert the gate to master's | **RED**, 3 of 8 | (green: this is master) | green | n/a |
| M2 drop the poison push (the run goes QUIETER) | **RED**, 2 of 8 | **BLIND**: 5004 passed, 2 failed, and both failures ARE the new test | **BLIND**: all four shapes still `b09ccd65`/`1b7fe316`/`0ad17404`/`2565ece2` at their pinned sizes | **RED**: containment BROKEN, 181 lines vanish across s2disasm 24, s2disasm-mompass-clean 24, skdisasm sonic3k 1, sonic_hack 18, Batman 114 |
| M3 never run the bonus pass on a pinned build | **RED**, 2 of 8 | not run | not run | not run |

So: the four-shape byte gate is structurally blind to this parcel in both
directions, because `force_relocate` is the gate's first conjunct and the aeon
path never reaches the arm. The pre-existing workspace suite is blind too: a
5006-test run under M2 fails on nothing but the two tests added here. The only
instruments that see the dangerous direction are the new test file and the
eleven-root containment sweep, and the sweep is not wired into anything.

## The note's caveat is real, and here is its witness

The measurement note says an `Ok`→`Err` flip "is not structurally guaranteed"
and that no corpus root is such a run. It is not hypothetical; it takes four
lines:

```
	cpu 68000
	org 0
Lbl:
	dc.l (NoSuchPlc<<24)|Lbl
	jsr NoSuchTarget
```

Base: `assemble` returns `Ok`, and the CLI fails at LINK with `unresolved
jmp/jsr target … not defined in this link`. Cut: `assemble` returns `Err` with
`unresolved long expression` and `unresolved symbol \`NoSuchTarget\``. Exit
status is 1 either way, which is why the nine-root sweep could not see it.

It is bounded rather than open: reaching the arm at all needs non-empty `poison`,
and a bonus pass that then returns `Ok` must have emptied it by building at least
one `Fragment::JmpJsrSym`, which a link with no composition refuses. So every
module the gate newly rejects was already unlinkable standalone; the refusal
moves from the linker to the front end and gets more precise on the way, and
`force_relocate` excludes the chained path that legitimately hands those
fragments to a composition. Both directions are pinned in
`failed_run_reports_everything.rs`.

## Shipping path

Enumerated firsthand, not inherited. `run_impl`'s `force_relocate` has three
callers: `run` and `run_located` (`false`), `run_relocating` (`true`).
Repo-wide, the relocating route's only consumer outside tests is
`crates/sigil-harness/src/native.rs:1538`; the non-relocating route's only one is
`crates/sigil-cli/src/main.rs:306` (`assemble_root_located_warned`; the note's
`main.rs:111` is another rotten coordinate). Four shapes built with the cut
binary against `AEON_DIR=/home/volence/sonic_hacks/.aeon-sigil-ref` (`ec640bcf`,
0 tracked modifications) match the committed pins at `aeon_rev ec640bcf`
exactly: `b09ccd65`/820229, `1b7fe316`/846529, `0ad17404`/96863,
`2565ece2`/103185. Per the table above that is a REPRODUCTION, not evidence.

## Suite and lint

`cargo test --release --workspace --no-fail-fast` with `AEON_DIR` and
`ORACLE_DIR` named: **5006 passed, 0 failed, 2 ignored**, exit 0, run from
`/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-af9ee78f0abdc942f` at
`451cb3e2` on `parcel/failing-build-reports-everything`, with the eight new
tests present in the log. A run WITHOUT `ORACLE_DIR` is red on
`oracle_loadfromaslisting_resolves_emit_listing` and one without `AEON_DIR` is
red on the two `act_descriptor_*` rows; both are the reference-tree resolver
refusing to measure, not this change.
`cargo clippy --release --workspace --all-targets -- -D warnings`: `CLIPPY_EXIT=0`.
