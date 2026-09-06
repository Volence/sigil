# AS-WARNING-MESSAGE-NONFINAL-PASS: the reason the scope line was drawn is false, and the population behind it is empty

Parcel note. Branch `parcel/as-warning-message-nonfinal-pass`.

## The question

Sigil's AS frontend converges by iterating passes and returns the CONVERGED
pass's diagnostics. A `fatal` raised on a non-final pass is carried forward
(`eval.rs::merge_carried_fatals`). A `warning` or a `message` raised on a
non-final pass was DROPPED, and that was pinned as a decision rather than left
as an oversight.

The reason written into the pin was:

> asl treats a `warning` as a diagnostic and keeps assembling, so a later pass
> genuinely does supersede it.

**That reason is false, and it is the first thing this parcel measured.**

## Provenance

| | |
|---|---|
| sigil | branch `parcel/as-warning-message-nonfinal-pass`, base master `ffb05a6d`, tip `7731d3a2`; built into `.target-land` in this worktree. Before-binary md5 `ecbe35e2077db690e7e63f53fce8f31a` (master `ffb05a6d`), after-binary md5 `17aff75ab6577fa6f669a959dc013917` (tip) |
| corpora | `s2disasm` `e45ebf33`, `s1disasm` `f6ece657`, `skdisasm` `2fcd861`, each in its OWN detached worktree under `/home/volence/sonic_hacks/.corpus-wmp/`; the shared live checkouts were never written |
| preparation | `scripts/corpus-prepare.sh` under `Lua 5.5.1`; 74 / 8 / 100 generated files written; `corpus-baseline.sh` reports `READY (39/39)`, `(4/4)`, `(50/50)` |
| baselines reproduced | 5,162 / 42 / 2,126, matching `2026-09-06-corpus-generated-includes.md` exactly |
| oracle | `asl` md5 `61e672562465725a8c102288a7da9098` through `docs/superpowers/notes/asl-reference/asl_ref.sh`'s `asl_run`, which reports the exit status and classifies the listing footer. The `s2disasm` build (md5 `0dee1f98...`) was never invoked |
| emulator | none. No runtime confirmation was attempted or is implied |

## Two counts, and they are not the same count

### Source sites (TEXT OCCURRENCES, not firings)

Re-derived from the brief's table, over tracked `*.asm` and `*.inc`, at column
start, with `dc.b` as the positive control that says the instrument fires.
**The brief's table reproduces exactly, including the control.**

| corpus | `warning` | `message` | `error` | `fatal` | `dc.b` (control) |
|---|---|---|---|---|---|
| s2disasm | 3 | 16 | 2 | 40 | 5,249 |
| s1disasm | 9 | 12 | 4 | 19 | 2,405 |
| skdisasm | 1 | 11 | 3 | 27 | 37,546 |
| **aeon** | **0** | **0** | 1 | 0 | 207 |

The aeon row is new and it settles the scope-boundary question before any code
was touched: **aeon names `warning` and `message` in no `.asm` or `.inc` file at
all**, so nothing on aeon's build path can raise one, let alone carry one.

### Firings (the instrument, per pass)

A temporary census instrument recorded every execution of `warning`, `message`,
`error` and `fatal` with the pass index it ran on, and printed a `RETURN` marker
naming the pass whose diagnostics the run returns. The patch is 113 lines and
was reverted before implementation; its shape is the house's existing
`SIGIL_CENSUS_EXPLABEL` shape.

**Feed, stated beside every count**: 13 `warning` source sites and 39 `message`
sites, over **12 pass evaluations** (s2disasm 5: passes 0,1,2,3 plus the bonus
pass; s1disasm 3: passes 0,1,2; skdisasm 4: passes 0,1,2 plus the bonus).

| corpus | firings, all kinds, all passes | what fired | on which passes |
|---|---|---|---|
| s2disasm | 4 | `message` at `s2.asm(91272)` | 1, 2, 3, bonus |
| s1disasm | 2 | `fatal` at `sound/z80.asm(229)` | 1, 2 |
| skdisasm | 4 | `fatal` at `Sound/Z80 Sound Driver.asm(345)` | 0, 1, 2, bonus |

**Zero firings of the `warning` directive on any pass of any corpus.**

## The partition the parcel asked for

Of the sites that fire at all (three, across three corpora):

| partition | count |
|---|---|
| fires on a NON-FINAL pass only | **0** |
| fires on the FINAL pass only | 0 |
| fires on BOTH | 3 |

Every site that fires at all fires on the pass that returns. The non-final-only
set is empty, and the pass-carrying rule has nothing in the corpus to act on.

## The zero could have been found, and here is the control

A canary that provably warns on a non-final pass only, run through the
instrumented binary. `cw1.asm`:

```
	cpu 68000
	padding off
	org 0
	if MOMPASS=1
	warning "canary warning first pass only"
	message "canary message first pass only"
	endif
	dc.b $11
	dc.w Later-*
Later:
	end
```

The instrument flagged it, which is the thing a zero elsewhere depends on:

```
CENSUS	pass=0	kind=warning	at=cw1.asm(5)	text=canary warning first pass only
CENSUS	pass=0	kind=message	at=cw1.asm(6)	text=canary message first pass only
CENSUS	RETURN	final_pass=1	bonus=no
```

A canary proves the RULE can fire and says nothing about whether the input
arrived, so the feed audit is below and separate.

## What asl does, which is what decides the ruling

Three canaries, the reference build, `asl_run -xx -n -q -A -L -U -i .`, exit
status and listing footer checked on every one.

| probe | shape | asl | sigil BEFORE |
|---|---|---|---|
| `cw1` | `if MOMPASS=1` | **prints it**, exit 0, footer `2 passes`, no incomplete line | **nothing at all**, exit 0 |
| `cw2` | `if MOMPASS>1` | prints it once, exit 0 | prints it once, exit 0 |
| `cw3` | unguarded | prints it **TWICE**, once per pass | prints it once |

```
> > > cw1.asm(5): warning: canary warning first pass only
canary message first pass only
ASL_EXIT=0
ASL_DIAG=complete
```

**asl does not retract a warning it has printed.** Its output over a run is the
UNION over passes, not the last pass's view. So there is nothing for a later
pass to supersede, and the supersession argument that correctly separates
`fatal` from an assembler-raised diagnostic does not separate the `warning`
DIRECTIVE from `fatal`: both are lines the author wrote in order to be told
something, and both were dropped for the same mechanical reason.

The part of the pin's reasoning that IS correct, and is confirmed here, is that
asl prints one once per pass rather than once per run. That is `cw3`.

## The oracle over the corpus, and where it refuses to be one

| corpus | asl invocation | exit | listing footer | author `warning` lines | `message` lines (stdout) |
|---|---|---|---|---|---|
| s1disasm | corpus's own (no extra args) | 0 | `2 passes`, complete | **0** | **1**: `Uncompressed driver size: 1BC6h bytes.` |
| skdisasm | corpus's own `-D Sonic3_Complete=0` | 0 | `2 passes`, complete | **0** | **0** |
| s2disasm | corpus's own (no extra args) | 2 | `2 passes` **+ `Additional necessary passes not started`** | 0 | 1 (before the failure) |

**The s2disasm row is not an oracle answer and is not used as one.** s2disasm
targets the flamewing fork; the reference build reports 570 diagnostic lines
against it and its listing carries the incomplete-pass line, so the diagnostics
of the pass asl refused to start were never looked for. It is recorded because
what it DID print agrees with sigil (one message at `s2.asm(91272)`, no author
warning), not because that agreement proves anything on its own.

The one `warning:` line the s2 run produced is asl's OWN
(`s2.asm(91275): warning #30: no sharefile created, SHARED ignored`), not a
`warning` directive. That distinction is why this table counts author warnings
rather than lines matching `warning`.

**Without the corpus's own `-D Sonic3_Complete=0`, skdisasm is also not an
oracle**: it exits 2 with nine `#1820: expression must be evaluatable in first
pass` and a ONE-pass footer. The first run in this parcel was that run, and it
would have been read as "asl also fails here" had the footer not been checked.

## The feed audit, which is where a zero usually goes wrong

Sigil's `fatal` sets `aborted`, which truncates the pass. Every site after the
aborting line is UNREACHED, and a site that is never reached cannot fire, which
is indistinguishable in the census from a site whose guard is false.

| corpus | where sigil's pass stops | `warning` sites reached | `message` sites reached |
|---|---|---|---|
| s2disasm | nowhere (no `fatal` fires); the pass reaches `s2.asm(91272)` | **3 of 3** | 16 of 16 |
| s1disasm | `sound/z80.asm(229)`, reached from `s1.sounddriver.asm:2632`, reached from `sonic.asm:5229` | **8 of 9** | 3 of 12 |
| skdisasm | `Sound/Z80 Sound Driver.asm(345)`, from `sonic3k.asm:201104` | **1 of 1** | 11 of 11 |

* s1disasm's three `sonic.asm` warning sites are at lines 139, 2668 and 2671,
  all before the include at 5229, so they were reached and did not fire. The
  one unreached warning site is `sound/_smps2asm_inc.asm:405`, included at
  `s1.sounddriver.asm:2639`, seven lines past the include sigil aborts inside.
* skdisasm's single warning site is in `Sound/_smps2asm_inc.asm`, included at
  `sonic3k.asm:28`, two hundred thousand lines before the abort.

So **12 of the 13 warning sites were reached and none of them fired**, and for
the two corpora where the reference asl gives a complete answer it independently
reports zero author warnings. The zero is not a reach artifact.

### The one place the feed DID fail, and it is a different defect

s1disasm's `sound/z80.asm(231)` is the `message` asl prints, and it is **two
lines past the line sigil aborts on**. Sigil never reaches it. The cause is not
the pass rule: sigil raises

```
sound/z80.asm(229): error: The driver is too big; the maximum size it can take
is 1FFCh. It currently takes 73DFDh bytes.
```

while asl assembles the same tree cleanly and its own `message` two lines later
reports the driver at `1BC6h`. Sigil computes the Z80 driver size roughly sixty
times too large. That is already inside the 42 and is a separate row.

## The ruling

**The scope line does not stand, and the reason it was drawn is refuted.** The
`warning` DIRECTIVE now carries forward.

The `fatal` parcel landed its carry on exactly this evidence shape: a canary
where asl speaks and sigil is silent, plus a measured zero corpus delta, plus
the argument that carrying is strictly additive so a run can become louder and
never quieter. The only thing that held `warning` back was a claim about asl
that turns out to be false. With it gone, the same argument applies unchanged,
and leaving a measured divergence in place because no corpus file happens to
trigger it would be reading the corpus's silence as proof the behaviour is
right.

**One line per source position, not one per firing.** asl's multiplicity is a
property of how many passes it happened to run: asl runs 2 over every corpus
root, sigil evaluates 5, 3 and 4. Reproducing the count would reproduce an
artifact of asl's pass loop rather than anything about the program. One per
position is what sigil already reports for every other repeated diagnostic
(`cond_faults_seen`), it is never MORE than asl prints for a site that fires at
all, and the converged pass's own text wins whenever that pass fires the site
too.

**`message` is booked, not implemented, and the row that pairs it with
`warning` is wrong.** See below.

### Corpus delta: zero, measured

`corpus-baseline.sh --compare` against the before-streams, class by class,
unresolved-symbol name sets in both directions, and whole line:

| corpus | before | after | delta |
|---|---|---|---|
| s2disasm | 5,162 | 5,162 | **+0** |
| s1disasm | 42 | 42 | **+0** |
| skdisasm | 2,126 | 2,126 | **+0** |

No class rose, none appeared, and the whole-line comparison is empty in both
directions.

### It cannot move a byte, and that is structural rather than measured

Every carried entry is `Level::Warning`. `run_impl` returns `Err` iff some
diagnostic is `Level::Error`, so no carried line can turn an `Ok` into an `Err`;
none of them reaches the module; and the module is what produces bytes. The
change is confined to diagnostic collection.

### Red-first

With `eval.rs` reverted to the committed baseline (`git checkout HEAD --`,
mutation shown on disk by md5: `7c6582f2...` under test against `7c39ba73...`
set aside, and by `merge_carried_author_warnings` returning a grep count of 0)
and the new tests left in place:

```
test a_warning_on_a_non_final_pass_is_reported ... FAILED
test a_carried_warning_names_the_file_it_was_written_in ... FAILED
test result: FAILED. 7 passed; 2 failed
```

Exactly the two predicted tests failed and the other seven passed, so the
mutation is specific rather than a blanket break. Restoring from the set-aside
file returned 9 passed, 0 failed. Stated before the run: a run in which every
test passed would have been a runner defect, not a pass.

## `message` is a different mechanism, and a live divergence today

The brief's row treats `warning` and `message` together. They do not behave
alike, and the difference is not about passes at all.

```rust
"message" => {
    let _ = self.interp_string(rest);
}
```

Sigil evaluates the string and discards it on EVERY pass, the converged one
included. **No pass-carrying rule can reach it**, and the pin's framing ("a
`message` raised on a non-final pass is dropped") understates it: a `message`
raised on the FINAL pass is dropped too.

asl writes it to STDOUT, unprefixed, outside the diagnostic stream. Two corpus
sites print there today while sigil prints nothing:

| corpus | asl stdout | sigil |
|---|---|---|
| s1disasm | `Uncompressed driver size: 1BC6h bytes.` | nothing |
| s2disasm | `ROM size is $100000 bytes (1024 KiB). About $8F1 bytes are padding.` | nothing |

Implementing it is a decision about a NEW output stream (stdout, not the
diagnostic stream, so every corpus count is unaffected but every runner that
captures stdout is), and it is out of this parcel's scope. It is pinned as
`a_message_is_dropped_on_every_pass_including_the_final_one` so the gap reads as
booked, and so that implementing it has to come to that test and delete it on
purpose. It is in the gap ledger.

A third finding sits inside that one. Sigil's census of the s2 site shows the
text it would have printed:

```
ROM size is $FFFED bytes (\{(EndOfRom-StartOfRom)/1024.0} KiB). About $2375 bytes are padding.
```

against asl's `$100000` / `1024 KiB` / `$8F1`. The `\{...}` sequence is
UNINTERPOLATED, so `interp_string` does not handle that float-division form.
Anyone implementing `message` inherits that, and it is in the gap ledger too.

## Where carrying could still print something asl does not

The hazard is real, it is bounded, and it is UNMEASURED rather than ruled out.

`MOMPASS` cannot produce it. Sigil reports 1 on pass 0 and 2 on every later
pass; asl counts. Both fire `if MOMPASS=1` on their first pass and
`if MOMPASS>1` on the pass that returns, so both guards agree.

What could produce it is a `warning` guarded on a symbol value that is
transiently true in one of sigil's EXTRA intermediate passes. Sigil evaluates 5,
3 and 4 passes where asl runs 2, so sigil visits intermediate states asl never
computes. **No corpus site fires at all, so there is no instance to observe, and
this parcel did not construct one.** The design bounds the damage rather than
removing it: at most one extra line, carrying the author's own text, naming the
author's own line, in the louder direction. That is the direction this repository
already chose for `fatal`.

## Anything in the brief I concluded was wrong

1. **The pass-structure argument, which the brief flagged as a second-hand
   restatement and asked to have corrected rather than agreed with.** It is
   wrong. "asl keeps assembling past a warning, so a later pass genuinely does
   supersede an earlier pass's warning" describes asl's behaviour for a
   diagnostic asl RAISES, not for the `warning` directive. asl prints an author
   warning and never retracts it, and probe `cw1` is the measurement.

2. **The row treating `warning` and `message` together is wrong.** They share a
   table cell and nothing else. A `warning` on the final pass is reported; a
   `message` on the final pass is discarded. The `message` gap is larger than
   the pin implies and is not a pass question.

3. **"a `message` raised on a non-final pass is still DROPPED" is true but
   misleading** for the same reason: it invites a reader to think the final-pass
   case works.

4. **The brief's source-site table is correct**, including the `dc.b` control,
   and reproduces exactly. Its warning about source sites not being firings was
   the load-bearing warning: 13 warning sites produce 0 firings.

5. **The brief said to use the reference asl and refuse the other build. Doing
   so leaves s2disasm without an oracle at all**, and the brief does not say so.
   s2disasm targets the flamewing fork; the reference build exits 2 against it
   with an incomplete-pass footer. The ruling therefore rests on s1disasm and
   skdisasm for corpus oracle evidence and on the canaries for the shape.

6. **`asl_diag_state` earned its place here.** The first skdisasm oracle run
   exited 2 with a ONE-pass footer because the invocation lacked the corpus's own
   `-D Sonic3_Complete=0`. Read by exit status alone that is "asl fails on
   skdisasm too"; read with the footer it is "this run answered nothing".

## What is left open

* **`message` is unimplemented on every pass.** Two corpus sites and an
  uninterpolated float-division form. Gap ledger.
* **Sigil computes s1disasm's Z80 driver size as `73DFDh` where asl's own
  message says `1BC6h`**, which fires a spurious `fatal` and truncates the pass
  at `sound/z80.asm(229)`. Already inside the 42; named here because it is what
  cost this census its reach over nine of s1disasm's twelve `message` sites.
* **The intermediate-pass false-positive shape is unmeasured**, above. Closing
  it needs a construct that fires on a sigil-only pass, and none exists in the
  corpus to derive one from.
* **`s2disasm` has no usable asl oracle in this lane.** Anything needing one has
  to either accept the flamewing fork with its non-determinism understood, or go
  without.
* **No runtime confirmation was attempted.** Nothing here needs one: the change
  emits no bytes.
