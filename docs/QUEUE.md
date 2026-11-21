# Sigil queue rows, full text

**A ROW'S STATE LIVES IN ITS OWN HEADING, and this rule exists because the file had nowhere to put
a closing reason.** On 2026-09-06 one commit closed four rows and wrote all four closing reasons
into the tail of a fifth row's body. The result: four closed rows whose own headings read `open`,
and one genuinely open row appearing to carry four closure notices. Nobody misread it deliberately;
the file offered no other place to write. The four were rehomed 2026-09-09 and each says where it
had been sitting.

So: when a row closes, mark the HEADING (`## ROW: LANDED` or `## ROW: CLOSED`) and put the reason
directly under it. A reader scanning headings must never have to read a neighbour's body to learn
this row's state.

**And a row can go stale faster than a day.** `PINS-DEAD-TESTS-FIELD-AND-ORPHANS` was written into
the dispatch slot at 19:29 and the parcel it describes landed at 19:59, thirty minutes later, both
by the same seat in one evening. Before dispatching from any row here, ground its claims against the
tree first: a stale open row does not read as stale, it reads as work.

Verbatim row text, moved out of `docs/lane-status.json` on 2026-09-06 under
`empyrean/contract/LANE_STATUS.md` rule 7 (a row states its state; the history has its own file).
The board keeps a short title and this file keeps the argument. Rows closed on that date are kept
here with their closing reason rather than deleted.

## AS-NAMELESS-LABELS-RC1

- state: **LANDED** 2026-09-10 at merge `6b26f7f7`, pushed  size: `L`  project: `SIGIL-AS-REPLACEMENT`
- Landed on `d-29`, answered `land` **by the HUB** under the owner's delegation, not by him; he overturns
  it with one word. Landing run GREEN: 5,017 passed / 0 failed, clippy clean, zero skips. Corpus headline
  reproduced independently at landing (149 rows, `$F9198`, `$989C`, zero unresolved slots).
- history, kept: blockedBy the owner, as decision **d-29** filed 2026-09-09: THE WORK IS NOW BUILT AND UNLANDED on branch
  `parcel/as-nameless-labels` (tip `c22f6050`, six commits, nothing merged). The hold below still stands and was
  correct; a 2026-09-09 session REVERSED IT WITHOUT READING IT, dispatched the parcel, and told the owner the item
  had never been waiting on him. That was wrong. The question put to him is no longer whether to spend the effort,
  which is already spent, but whether the feature lands. Size L sent for his eye, and no owner decision exists on
  the item itself. A hub go is not his go.

THE Sonic 2 item, now measured: AS nameless temporary labels (+, -, /), unimplemented entirely, are the single root cause of 4,985 rows across four message classes, 86.5 percent of the whole run. ACCEPTED BY THE HUB (d-22, 2026-09-03, answered by: hub, under the project declaration) and NOT BY THE OWNER: this item has had no owner decision of any kind, only a size sent to him. On 2026-09-06 the hub sent a go resting on d-22 read as an acceptance in hand; this lane held and the hub withdrew it. Do NOT size the post-fix count by subtraction: closing it reaches code currently abandoned and can add rows as well as remove them.

## AS-MESSAGE-AND-INTERPOLATION: LANDED 2026-09-07, and this row sat open for two days

- state at archive: `LANDED`  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing; nothing is left to do

**BOTH halves shipped at `67fdea97`** (2026-09-07 12:16 local, verified an ancestor of master),
*"`message` reaches stdout on the converged pass; `\{expr}` pastes floats and strings"*, 39 files
including asl probe transcripts. `message` now carries a stream of its own out of the front end as
`Assembled::messages` / `Failure::messages`, one line per firing from the converged pass, and the
`a_message_is_dropped_on_every_pass_including_the_final_one` pin was deleted on purpose as this row
asked. The float interpolation half is pinned too: `as_interp_shapes.rs` asserts
`("(2048-1024)/1024.0", "1")` and `as_message_stdout.rs` carries the exact ROM-size idiom the row
named. Reading the row's text against the tree, every clause it states as outstanding is done.

**⚠ THE ROW WAS THE NEXT THING THIS LANE WAS ABOUT TO DISPATCH, AND ONLY GROUNDING CAUGHT IT.** On
2026-09-09 the fallback sequence reached this row, and the brief was one command from being written.
What stopped it was reading the message arm in `eval.rs` before describing it, which is a habit this
lane adopted tonight only because three earlier briefs had each named a wrong location. **A stale
open row does not read as stale; it reads as work.**

This is the boot read's own banked defect arriving a second time: a parcel's completion has to be
written back to the document that DISPATCHES it, and no gate anywhere makes that happen. The first
instance was the R7 block saying *"what remains is the flip itself"* for a week after the flip
landed. **The cost here would have been an agent sent to build what exists**, and the likelier
failure is not wasted effort but a plausible second implementation of a solved problem.

The old row text is kept below, unedited, because a reader who has already carried it away needs to
meet the correction rather than find the row silently rewritten.


Two rows that must land as ONE parcel, per the census. (a) sigil's message arm evaluates its string and discards it on EVERY pass, converged included, so it is not a pass question at all and the old row pairing it with warning was wrong. asl writes it to STDOUT, unprefixed and outside the diagnostic stream, so implementing it moves NO corpus diagnostic count while changing what a runner capturing stdout sees. 39 sites; two print under asl today and sigil prints neither. (b) the one corpus site that would fire needs an interpolation form sigil drops: \{(EndOfRom-StartOfRom)/1024.0} comes back UNINTERPOLATED, and that idiom is how all five s2disasm table-size messages are written. warning shares interp_string and inherits this the day a corpus site fires. Deleting the a_message_is_dropped pin is part of the work and is deliberate, not incidental.

## EMP-Z80-MNEMONIC-TABLE: LANDED

- **REOPENED 2026-09-25 as card `d-34-reopened`:** the hub withdrew its ruling at empyrean `8a8c29ad` (outside the delegation, d-6 reserves spellings to him). Stays LANDED until he answers; a revert moves no ROM byte.
- state: **LANDED** 2026-09-25 at merge `bc88480d`, pushed. Landed on `d-34`, answered `land-all-21` **by the HUB** under the owner's delegation (empyrean `796b508f`), not by him; he overturns it with one word at no cost while no game source uses the `(c)` port spelling. Landing run on the merged tree: 494 suites, 5611 passed, 0 failed.
- history, kept:

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: you: this is language surface, not assembler internals

TAGGED by the Z80 coverage parcel and deliberately not done. The AS side now encodes 21 more instructions; our own .emp language's Z80 mnemonic table was NOT extended to match, because what the game can be written in is language surface and yours rather than mine. Extending it also needs a new operand kind for the in/out port-in-C form. Nothing depends on it today.

## ROWREMAP-HEAD-LABEL-RULED: CLOSED

- state at archive: `open`  size: `S`  project: `EFFECTS-W1`
- blockedBy: the engine lane: the fix is at their generator, not here

SENT to aeon 2026-09-06 with its anchor: row D2 in docs/superpowers/notes/2026-09-05-decouple-aeon-side-inventory.md (b0cf2eeb). RULED, no sigil change: the ojz_effects_editor_act1 order row must key by SECTION NAME, not head label, because the head label is content-derived. Nothing breaks today (SECTION-ROW resolves name to head label at placement); the fix is at the engine lane's generator, and aurora's read is that only determinism AMONG tables is required, so it is probably a one-line move, unproven until a build refuses or does not.

**CLOSED 2026-09-06 as far as this lane goes: ruled, no sigil change, sent to aeon with its anchor. The fix is at their generator.**

**Its closing reason was written into PROSE-STATED-BOUNDS's body by the commit that closed four rows at once, so this row read `open` for three days while its own closure sat under an unrelated heading. Moved here 2026-09-09.**

## AEON-TOOL-DEFECTS-NAME-AS-POSITION: CLOSED

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

SENT to aeon 2026-09-06 with paths and SHAs; they book from the files, not from a restatement. Five findings for the engine lane, all committed in sigil since 2026-09-05 and reachable from origin/master: the lint table already wrong, four mis-measuring routine-extent copies, pointer identity by name-string (these three are sections S1/S3/S9 of docs/superpowers/notes/2026-09-05-consuming-end-name-as-position.md), plus a margin decaying 440 B/day into a forbidden band and Knuckles' ceiling equalling its bar with nothing saying so (2026-09-05-p6-resweep-current-layout.md, section 6, items 2 and 3). RENAMED from AEON-S1-S3-S9-ROUTED: the old id was built from this lane's own internal section numbers and was unreadable to the only lane that needed it, which is how aeon came to believe the findings were lost in a message while they sat committed and unread. Recording was never the failure; routing was.

**CLOSED 2026-09-06. The aeon lane worked all five at aeon 36e3e409: four held and landed, S1 declined on the owner's scope cut with its ruling recorded. One of the five refuted a measurement of this lane's (see the bound-versus-ceiling correction at sigil 4221cfee).**

**Its closing reason was written into PROSE-STATED-BOUNDS's body by the commit that closed four rows at once, so this row read `open` for three days while its own closure sat under an unrelated heading. Moved here 2026-09-09.**

## STABILITY-RUNNER-MISSING-WHERE-CLAIMED: SWEPT 2026-09-16, and the "five" is retired as unfounded

- state at archive: `open` -> swept  size: `S`  project: `-`
- blockedBy: nothing

Original text, kept verbatim: *"Sweep owed, now with a named population: five notes from the same
week state their ground truth as the UNRELIABLE copy cited by version banner, with no committed
probe directory to re-run."*

**"Now with a named population" was not true: the row names none of the five, and no instrument
returns five.** The 2026-09-08 staleness sweep reached the same conclusion and refused to invent a
population; that refusal is upheld rather than overridden. **The count is retired as an unfounded
figure** of the same family as the three premise-less rows found the same morning.

**What was done instead**, note `docs/superpowers/notes/2026-09-16-asl-citation-form-sweep.md`,
instrument `scripts/sweep_asl_citation_form.sh` (committed, which is this row's own standing fix
applied to itself): all four `asl` binaries here print the same version banner, so a note citing its
oracle by banner has named nothing. **22 notes cite the banner and never an md5.** The script prints
the list and carries its own scope limits, a positive control, and the two workspace facts it depends
on. Figures move when notes discussing the defect are edited, so the script is the authority.

**Left open deliberately:** whether any of the 22 states a claim that actually DEPENDS on which
binary produced it. That needs reading rather than grepping and is a separate parcel. The standing
fix stands: a stability or provenance claim cites a committed script or an md5, or it is not made.

**2026-09-26: the open question is answered.** The sweep at `9212d6e0` returns 22; read against
what is actually known to differ between the four builds, and re-run on all four where the input
survived or could be rebuilt: **3 DEPENDS, 19 DOES-NOT-DEPEND, 0 UNDETERMINED**, and every DEPENDS
row is already settled or superseded. Note
`docs/superpowers/notes/2026-09-26-asl-banner-citation-dependence.md`, probes and the all-builds
runner beside it. The three are booked below.

### ASL-BANNER-DEPENDS-THREE: CLOSED 2026-09-26 at landing (merge `7478f0fe`), no re-run owed

- was: state `open`  size: `S`  project: `-`
- blockedBy: nothing

Three banner-only notes state a claim that depends on which `asl` produced it. Each is settled or
superseded in `docs/superpowers/notes/2026-09-26-asl-banner-citation-dependence.md`:
`2026-09-03-as-macrosetup-three-sites.md` (a whole-Sonic-2 run's silence at 34 lines, on
`0dee1f98`; superseded, `AS-WORD-IMM-RAM-LABEL` closed at `2026-09-04-as-end-probes/README.md`),
`2026-09-07-as-nesting-relex.md` (asl timing, taken through `asl_run`, which accepts only
`61e67256`; settled), `2026-09-08-extra-traversal-measure.md` (asl timing and a whole-`s2.asm`
success; identified as `61e67256` for s1disasm and `0dee1f98` for s2disasm from its surviving
scratch copies; settled). The notes themselves are historical and were not edited.


### AS-REPLACEMENT-STATE: LANDED

- state: **LANDED** 2026-09-27 at merge `6128bb1e`, pushed  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- Measurement only: `docs/superpowers/notes/2026-09-27-as-replacement-state.md`. Headline: all five stock entry
  points build byte-identical through sigil's own command; no disassembly's own build script can use sigil as
  `asl` today. The note's section 8 is the source of the five rows below; re-derive before dispatching any.
- Overseer re-check at landing: S1 `afe05eee`/524288 and S3K `0658f691`/2097152 rebuilt both ways from fresh
  archives (s1disasm `f6ece65`, skdisasm `2fcd861`), zlib CRC-32, cmp identical, planted-byte control seen.
- The first run of this parcel died uncommitted in the 2026-09-27T09:14Z lane restart; its worktree
  `agent-a83b3b56407567d30` (branch `parcel/as-replacement-state`) holds only scratch and can be pruned.

### AS-DROPIN-ASL-CONTRACT: LANDED

- state: **LANDED** 2026-09-27 at merge `31c65791`, pushed. Full record: the merge body and
  `docs/superpowers/notes/2026-09-27-as-dropin-asl-contract.md`. Deviations ratified: `-U` required in asl
  mode; the two S2 `ACK_DISAGREE` entries scoped to the direct route rather than removed. Left open, per the
  note: `-L` writes no listing, `-i` takes only the source's own directory, one source per call, a `before`
  blob after a run over 0xFFFF bytes may be placed differently (no corpus has it), no Windows build.
- original row, kept:
- state: **next**  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- Let a disassembly's unmodified `build.lua` run sigil in place of `asl`: accept asl's flags
  (`-xx -n -q -A -L -U -E -i . -c -D`), write a `.p` the stock `p2bin` accepts, the `-c` share file and the
  `-E` log. Sized off the shim (5/5 stock ROMs) and the `.p` writer probe (3/3 with p2bin md5 `4f2fff99`),
  note sections 3 and 8. Measured constraints: records keep emission order, and the run just before each Z80
  record stays separate. Should also retire both S2 `ACK_DISAGREE` legs (share file), proved by the option
  sweep through the drop-in.
- Invocation shape is forced by the goal (the script must not be edited), so it is this lane's call under
  `d-2`, logged as a notable call when made, not an owner blocker. No `.emp` surface.

### AS-DROPIN-NIGHTLY

- state: **LANDED** 2026-09-28 at merge `38d6aead`, pushed; landing run GREEN 5846 passed 0 failed (5846 + 0 new, scripts and docs only). First two-route nightly: 02:17 2026-09-29, read `~/.local/state/sigil-switch-sweep/nightly.log`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- The nightly switch sweep runs only the direct route; `--route dropin` exists since `31c65791` (40/40, and
  808/808 with `--cross`, on 2026-09-27). Add a drop-in pass so the route a disassembly user takes is watched
  every night. The direct route's S2 share-file gap stays acknowledged there.
- 2026-09-28, on the branch: `nightly_switch_sweep.sh` runs `--cross` twice, `--route direct` (log
  `sweep.log`) then `--route dropin` (`sweep-dropin.log`), one sigil build and one pair of corpus SHAs for
  both; each pass reconciled by the direct pass's rules plus its own `ROUTE` line; the dropin pass runs even
  when the direct one could not; both share a four-hour budget inside the unit's five; one verdict line names
  both routes, exit the worse of the two. Hand run at `f79c0059`, GREEN: direct 808 legs (790 planned + 18
  rescue) 22m23s, dropin 808 legs (790 + 18) 24m38s, 47m19s job wall. Reds proved through a log-replaying
  stub sweep: dropin marker dropped exit 2, dropin finding exit 1, dropin log naming the wrong route exit 2,
  direct abort exit 2 with the dropin pass still run green, budget spent exit 2 with dropin not started.

### LANDING-LOG-CHECKER-READS-THE-WRONG-FILE

- state: **LANDED** 2026-09-28, fast-forward to `17361152` (fix `28e51dc8`), pushed; landing run GREEN at `f37dfc77`, 5852 passed 0 failed  size: `S`
- `scripts/check_landing_log.py <log>` reports NO VERDICT, RED on the log `landing-run.sh` writes (`$LOG`),
  because the script echoes its verdict block to stdout only and writes just the `*_EXIT=` lines into `$LOG`.
  Measured 2026-09-27 on the drop-in parcel's `landing-2.log` and on `.landing-audit-194834.log`; the
  `.stdout` capture passes the checker while it reports every exit gate NOT REPORTED. `landing-run.sh
  --verdict-only <log>` judges `$LOG` correctly. Make one artifact carry both, or point the checker's docs
  at `--verdict-only`; either way the two instruments must agree on one file.
- 2026-09-28, on the branch: a run appends its verdict block to its own `$LOG` inside a
  `##### VERDICT SPAN` with `LANDING_EXIT=<exit code>`, computed first and appended after, so the verdict
  never reads a growing file; a failed append exits 2. The verdict is one function over the RUN RECORD, the
  log above that span, because the block quotes run-shaped text (a silent binary's own `Running` line) that
  a re-judge reading the span would count again. `--verdict-only` writes nothing. The checker reads RESULT
  from the one span only and the exit lines from the record; exit 0 needs a recorded GREEN, `LANDING_EXIT=0`
  and all three gate exits 0. Refused with exit 2: a pre-span log (naming `--verdict-only <log>`), a captured
  stdout (naming the log), two spans, a truncated span, and a GREEN its exit lines contradict. Header (9a),
  usage and the checker docstring now name the log and both instruments.
- Verified: `landing_verdict.rs` 23 to 29 tests; three run the real script in run mode against a stub
  toolchain (stub cargo on PATH, stub ledger and census). Red-first: pre-fix append (2 red), run record
  reading the whole file (1 red, 2 launched read back as 3), `--verdict-only` appending (1 red), pre-fix
  checker (5 red). Mutation evidence is in the commit bodies.
- Landing run at `f37dfc77` (clean), reference `.aeon-sigil-ref` @ `ec640bcf`: GREEN, 5852 passed 0 failed
  2 ignored, 519 suites, 519 launched = 519 reported = census, 5846 + 6 new, clippy and ledger clean. On
  its `$LOG`: `check_landing_log.py` exit 0 `RESULT GREEN` with `LANDING_EXIT=0` and all three gate exits 0;
  `--verdict-only` exit 0 `RESULT GREEN` twice, byte-identical to each other and to the recorded block, log
  md5 unchanged.

### AS-AUTHOR-FORMS-EXACT

- state: **LANDED** 2026-09-27 at merge `cb5ab42b`, pushed; landing run GREEN 5784 passed 0 failed (5770 + 14 new), clippy clean  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- 31 of 167 common 68000 spellings a hack author writes are refused by sigil and accepted by `asl`; the three
  disassemblies use none, so no corpus test sees it (note section 4). Twenty are plain, plus unquoted
  `binclude` and refusing `swap.w`. Stage 0: check whether refusing the `adda`/`eori` shorthand was a ruling;
  if it was, those rows go to the owner.
- Stage 0 answered: the ISA's refusal of `add Dn,An` is the `01121362` fix for a silent ADDX mis-encode, not
  a ruling against the alias. It stays on the `.emp` route; the AS route rewrites to `adda`/`suba` as asl does.
- 2026-09-27, on the branch: every form byte-identical to the pinned asl (md5 `61e67256`, `asl_run`, exit 0),
  probes in `docs/superpowers/notes/2026-09-27-as-author-forms-exact-probes/`, tests
  `sigil-frontend-as/tests/as_author_forms.rs`.
  - ISA (both routes' encoder, `.emp` spelling unchanged): `chk.w`, `link`, `unlk`, `reset`, `rtr`, `stop`,
    `trapv`, and `eori.b #,ccr`, `andi.w`/`ori.w`/`eori.w #,sr`, which sigil refused even SUFFIXED. Decoder
    and capstone sweep agree, no new exclusion.
  - AS route only: the eight mnemonics; `.b` branch suffix; unsuffixed `andi`/`ori`/`eori` to `ccr`/`sr`;
    `add`/`sub` to An as `adda`/`suba` (every source mode) and `eor #imm,Dn` as `eori`; `$1234.w`/`.l`
    unparenthesised, and `.W`/`.L` in either form; bare `binclude` paths.
  - Refusals added, each an asl refusal sigil accepted: `swap.b`/`.w`/`.s` (asl takes bare and `.l`),
    `andi.w`/`ori.w`/`eori.l` to `ccr`, and suffixes on `reset`/`rtr`/`trapv`/`stop`/`illegal`/`unlk`.
  - A silent-wrong-bytes defect found beside item 6: `binclude "f",off,len` ignored `off` and `len` and
    emitted the whole file. Now sliced as asl slices it. No shipped source or corpus uses the operands.
  - `.emp` unchanged, pinned against master `3ceed19b`: `sigil-frontend-emp/tests/as_author_forms_stay_off_emp.rs`.
- Left open, each recorded not guessed:
  - `chk.l` assembles under asl (`4300`, the 68020 form) and sigil refuses it by name; bare `chk` assembles
    under asl (`4380`) and sigil refuses it with bare `muls`/`divs`, under AS-UNSIZED-DEFAULTS.
  - A `.s`/`.b` branch to the next instruction: asl emits `4E71` (a NOP) and sigil refuses the zero
    displacement. Unchanged here.
  - ~~`jmp Foo.w` with `Foo` a symbol assembles under asl (`4EF8 1234`); sigil lexes `Foo.w` as one name.~~
    Done in `AS-SYMBOL-SIZE-SUFFIX` below.
  - `.emp` spelling of the seven new mnemonics: needs `link`/`unlk` in the clobber model and `rtr` in the
    terminator set, and is an owner-reviewed language change.

### AS-SYMBOL-SIZE-SUFFIX

- state: **LANDED** 2026-09-28, fast-forward to `5bc05325` (fix `88eb2e35`), pushed; landing run GREEN at `94df424e`, 5859 passed 0 failed  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- From AS-AUTHOR-FORMS-EXACT's "Left open": `jmp Foo.w` with `Foo` a symbol. sigil's lexer makes `Foo.w` one
  identifier (`.` is an identifier character, `Parent.local` a local label), so sigil refused it, and read
  the dotted name SILENTLY where one existed (a `Foo.w` equate, or a local `.w` under `Foo`).
- Stage 0, pinned asl (md5 `61e67256`, `asl_run`), 97 probes in
  `docs/superpowers/notes/2026-09-28-as-symbol-size-suffix-probes/` (`asl.out`, `sigil-before.out`,
  `sigil-after.out`). asl peels a trailing `.w`/`.l`/`.b`/`.s` off an effective-address operand BEFORE the
  lookup: `jmp Foo.w` is `Foo` at word width even when `Foo.w` exists; out-of-window values are #1340
  (no silent truncation, forward or not); `.b`/`.s` on an absolute are refused. It reads the name WHOLE in
  `dc`, `#imm`, `(Foo.w).w`, `Foo.w+2`, and in branch and `dbcc` targets (`bra.s T.w` goes to the local
  `T.w`). The peel also covers `(Foo.w)` and displacements (`.w` before `(An)`/`(pc)` and in `(d,An)`, `.b`
  before `(An,Xn)`). The three disassemblies and the aeon reference tree spell none of this (scan: 0 hits
  each, 2 of 2 in a planted control), so their builds could not move.
- Changed (AS route, `operands.rs`): `peel_width_suffix` applied to absolute, `(abs)` and displacement
  operands with the width asl accepts there, every other width refused by name; branch and `dbcc` go
  through `parse_target_operands`, which reads the name whole. Numbers take the same peel, so `4.w(a0)`,
  `(4.w,a0)`, `0.w(pc)`, `4.b(a0,d0.w)` and `($1234.w)` now assemble, each measured. `.emp` unchanged.
- Divergence, recorded not matched: `move.w Foo.b(a0),d0` exits 0 under asl with no diagnostic and NO BYTES
  for the line; sigil refuses it.
- Verified: `sigil-frontend-as/tests/as_symbol_size_suffix.rs`, 6 tests over all 97 probes, bytes from
  `asl.out` only; sigil agrees with asl on 96 of 97 (41 before, with 11 probes exit 0 on bytes or acceptance
  asl does not give), the 97th being the divergence above. `.emp` pinned at `4915361c` in
  `as_author_forms_stay_off_emp.rs`. Red-first: pre-fix `operands.rs`/`eval.rs` (5 of 6 red), branch
  targets through the peel (1 red: `bra_local_w`, `bsr_local_l`, `dbf_local_w`), any displacement width
  accepted (1 red), and an altered `.emp` pin expectation (1 red). Landing run at `94df424e`, reference
  `.aeon-sigil-ref` @ `ec640bcf`: GREEN, 5859 passed 0 failed 2 ignored, 520 suites launched = reported =
  census, 5852 + 7 new, clippy and ledger clean; `check_landing_log.py` and `--verdict-only` both GREEN.
  Switch sweep (s1disasm, s2disasm): 40 legs launched = reported, 0 unacknowledged, SWEEP PASSED.

### AS-UNSIZED-DEFAULTS

- state: **open, owner question**  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- `asl` silently defaults an unsized instruction to `.w`, accepts bare `ds`, and sizes unsized branches itself;
  sigil refuses all three (note section 4). Matching `asl` means accepting a silent default in the AS frontend,
  against this lane's refuse-by-name stance (AS-DEFAULT-CPU ruling). Mechanics exist (`RelaxLadder`); the
  question is direction, so it is put to the owner before any dispatch.

### AS-DIAG-FATAL-FIRST

- state: **landed** (merge `cf520369`, landing run GREEN 5770 passed 0 failed)  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- When the source's own `fatal` fires (S3K, a FixMusicAndSFXDataBugs build), sigil prints 563 error lines and
  the cause is line 468; `asl` prints one (note section 5). Report the `fatal` first and drop the unresolved
  follow-ons it caused.
- 2026-09-27, on branch `parcel/as-diag-fatal-first`, since merged: the `fatal` is printed first; an error
  is dropped, and counted in one note, when a second assembly that does not stop at the `fatal` no longer
  raises it. S3K leg: 563 lines with the cause 468th became the `fatal` plus `562 further errors not
  reported`, exit 1 both. Tests: `sigil-cli/tests/as_fatal_reported_first.rs`.

### AS-PERF-MEASURE

- state: **LANDED 2026-09-28 at merge `4bbc972b`** (tip `e01364c8`), measurement done  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- Answer: `docs/superpowers/notes/2026-09-28-as-perf-measure.md`. Extra front-end passes (S1/S2/S3K 3/4/3 vs asl 2)
  are 94 to 98 percent of wall; layout's second copy of every section sets peak RSS. The candidates below measured
  under 0.5 percent each. The fixes are booked as `AS-PERF-FIX-PEAK` (next) and `AS-PERF-FIX-PASSES`.
- sigil is 2.0x to 2.5x slower than `asl`+`p2bin` and uses 5x to 18x the memory on the `.asm` route (load 9 to
  11 at the time, note section 6). No profiler here; per-phase timing is a code change, so measure first.
- 2026-09-27, CORRECTED 2026-09-28: the first dispatch was NOT dead. Its transcript went quiet for 14 hours while it
  waited on its own backgrounded release build (the brief forbade waiting on a notification; it did anyway), and the
  overseer removed its worktree on the transcript's silence alone, without checking for a live process. It then
  reported the loss. Nothing committed was lost; its read-only findings, below, are the head start.
- Environment facts it measured: no `perf`, `valgrind`, `samply`, `heaptrack`, `hyperfine` or `/usr/bin/time`, so CPU
  seconds come from `getrusage` in-process. The worktree-isolation hook refuses compound shell commands mentioning git;
  single plain commands pass. Tell the next agent both, and to run its build in the FOREGROUND.
- Code candidates (read, NOT measured), `crates/sigil-frontend-as/src/eval.rs`: `one_pass_with_defer` builds a fresh
  `Asm` and clones the whole symbol environment, macro, function and known-label tables every pass; every `include`
  is re-read from disk each pass through `sigil_span::read_set` (CRC-32, canonicalise, mutex), given a fresh
  `SourceMap` entry and re-lexed by `split_src_lines`; `run_passes` keeps a whole `SymbolTable` per pass in `history`
  for the oscillation proof (a peak-RSS candidate growing with pass count); a converged run with `force_relocate` or
  leftover poison runs a bonus pass. CLI phases after the front end (`run_asm`, `sigil-cli/src/main.rs`):
  `resolve_layout_placing`, `link`, `check_image_bounds`, `flatten_placing` (compression), `apply_sega_header`,
  `emit_image`. Its planned instrument: env var `SIGIL_PHASE_TIMING`, read once via `OnceLock` like `SIGIL_CENSUS_*`.

### INOUT-PROOF-INVOKE-HOOK

- state: **LANDED 2026-09-28 at merge `87cdb675`** (tip `f1c7b70a`)  size: `S`  project: `-`  reported by: aeon, 2026-09-27
- Reproduced and real: three checkers read only a bare `Sym` as a call target, and a bound `invoke` lowers to
  `jsr (Proc).l` (`AbsSym`). One shared reader, `flag_check::direct_proc_target`, now takes both. Full record:
  `docs/superpowers/notes/2026-09-28-inout-proof-invoke-hook.md`. aeon's `Rings_AnimTick` split is now optional.
- **Booked from it, open:** `preserves.rs` `call_target` is a fifth `Sym`-only reader feeding the dead-save walk; a
  fix there can move bytes, so it is its own parcel. Also: the worktree tree-walk tests skip only a dir named
  `target`, so a cargo target dir named anything else leaves scratch copies that red `scripts_name_their_tree`.
- *(Report as filed, kept below.)*
- aeon's agent reports that while landing their merge `9496bbb3` (verified on aeon `origin/master` here), our
  contract checker treated `invoke Game.ring_frame` inside `DrawRings` as an unknown callee and failed its
  `inout` proof (d5/a4), so they moved the tick into its own proc. A hook call has a declared clobber bound,
  so an `inout` proof across it should be decidable. **Unreproduced here.** Stage 0: rebuild the pre-workaround
  shape (aeon's `parcel/vram-tier1` history, or a minimal `.emp` with an `invoke` of an interface hook inside a
  proc carrying an `inout` claim) and see whether the refusal is real and whether it is the right answer.
- **Reproduction pointer (aeon, verified here):** the only record is the body of aeon `3d369bc7`, reachable from
  their `origin/master`, lines 16 to 17: the inlined `invoke` fired `[proc.inout-unverified] DrawRings ::
  inout(d5)/inout(a4)`. The failing file was never committed; rebuild it by inlining `Rings_AnimTick`'s body
  (its `invoke Game.ring_frame`) back into `DrawRings` in `engine/objects/rings.emp` at `3d369bc7`.
- Related, not a defect: their `rings_port` byte change reaches us only at the next pin advance
  (`PIN-ADVANCE-S2-ENVELOPE-RESTATEMENTS`); the reference tree stays at `ec640bcf`.

### PHASE-ROW-CPU

- state: **open, parked**  size: `S` to `M`  project: `-`  asked by: oracle, 2026-09-27
- **The ask:** a trailing token on each listing `PHASE` row naming the processor, so oracle stops guessing
  whether a phased VMA is a Z80 or a 68000 address (it named real 68000 code at `$8000` as
  `SoundTablesZ80_Head+...`).
- **Why it is parked, found at step 0 and verified by the overseer at the remote tips 2026-09-27:**
  (1) `Section.cpu` names the ENCODER (`crates/sigil-ir/src/lib.rs`, "which CPU this section's instruction
  bytes target"), not the bus the VMA belongs to. All six live rows come from aeon
  `games/sonic4/data/sound/soundbankhead.emp:58`, `section soundbankhead (cpu: m68000, vma: $8000)`, which is
  68000-assembled data in the Z80 bank window, so threading `sec.cpu` prints the wrong answer on every live
  row. What oracle needs is the VMA's ADDRESS SPACE, which no `.emp` section records
  (`AddressSpace::Foreign` is set only by the AS frontend and describes the LMA). Recording it is a new section
  attribute, so language surface: propose, discuss, land.
  (2) Two readers break on a trailing token: oracle `crates/oracle-core/src/symbols.rs:1537` matches exactly six
  tokens (origin/main `980d0d7d`), and aeon `tools/clip_anchors.py:107` is `\s*$`-anchored (origin/master
  `aa9cb67b`). Both loosen first, before any row bytes change.
  (3) Spelling, if it lands: `.emp` says `z80`/`m68000` (d-27, `m68k` refused), the AS frontend and linker
  diagnostics say `Z80`/`68000`. Undecided.
- **Oracle's interim rule** (VMA != LMA means the VMA is a Z80-window address, the 68000 address is the LMA) is
  right for the whole live population. **Kill condition / trigger:** a phased 68000 block whose VMA is a 68000
  address (code copied to RAM) appears in aeon; then this row becomes a language proposal to the owner.

### ASL-CITATION-SWEEP-FOLLOWUPS: LANDED

- **LANDED 2026-09-27 at merge `103c2771`.** All three done: the sweep script sees 8-hex prefixes and
  all four digests and excludes a reviewed mention-only list, printing the old rule as its "historical
  count" (22 at `702b2397`; corrected 20 there, 19 from this parcel on, because the enum README now names
  its oracle by digest). Row 13 of the enum probe README is corrected with two more probes. The
  asl-reference README's two `a8cd8b80` rows now read `(i386-unknown-linux)`, applied after the
  overseer's review re-ran all seven table paths. **The "22" quoted in the row above and in the
  2026-09-16 and 2026-09-26 lane-log entries is the historical rule's figure and stays as recorded**;
  the script's output is the authority for the current count.
- state at landing: `open`  size: `S`  project: `-`
- Three findings from `docs/superpowers/notes/2026-09-26-asl-banner-citation-dependence.md`, none acted on:
  (1) `scripts/sweep_asl_citation_form.sh` misses digests written as 8-character md5 prefixes and counts notes that
  only MENTION the banner, so its banner-only population over-counts (`2026-09-05-asl-silent-decline-regime.md` is in
  it only for that reason); (2) `asl-reference/README.md` gives `a8cd8b80`'s second banner line as x86_64, the binary
  prints `(i386-unknown-linux)`; (3) the enum probe README's row 13 "value folds to 0" is not a rule (one byte above
  the line gives `0101` on all four builds, from a run that exited with errors). Also: banner line 2 separates three
  families, so only `0dee1f98` and `aa6de52f` are banner-indistinguishable.

## S1-BUILD-PROFILE

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

Why we take about 1.3 seconds on Sonic 1 where the old assembler takes half. One leg answered: there is essentially no fixed startup cost, so what we spend is proportional to input.

## AS-LEADING-DIGIT-IDENTIFIER

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

Whether a name may begin with a digit. The one-byte-short defect is closed; what remains splits into a compatibility call (mine) and a language call (yours).

## EMP-USP-CCR-SURFACE

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

Two of the new instructions are deliberately NOT exposed in our own language - they need a new operand kind, which is language surface and your call rather than mine. Nothing depends on it today.

## EMP-ALIGN-SHARED-RULE: LANDED

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

Our own language's align still rounds up plainly. That is correct for every ROM section, which is all the engine currently has, so nothing is wrong today - but the moment anything aligns on the RAM side it needs the signed rule the assemble

**LANDED at `3ed600d4` / merge `4f7fc54f`, established by the 2026-09-08 staleness sweep rather than assumed from the merge title: both `.emp` lowering sites call `sigil_ir::asl_align_pad` (`lower/mod.rs:1034`, `regions.rs:565`), and the rule is pinned by `align_as_parity.rs:138`, `a_negative_vma_align_follows_the_shared_signed_rule`. The row's claim that our align "still rounds up plainly" and needs the signed rule the day anything sits on the RAM side is answered on both halves.**

## CLOBBER-UNEXERCISED

- state at archive: `open`  size: `L`  project: `-`
- blockedBy: the payoff measurement, and then the engine lane: the declarations are theirs, not ours

RULED d-26 on 2026-09-06 BY THE HUB IN THE OWNER'S PLACE, overturnable by him, NOT his word. Measure the payoff first as a size S row, AFTER the driver-size parcel, never beside it; the L fix does not start on the unnumbered claim. If the number justifies it, a witness derived from EMITTED code is mandatory before any declaration is removed, because a witness taken from the declaration being corrected shares the frame it exists to check. Hand-editing the frozen baseline is REJECTED. Scope rider verified here rather than relayed: 89 aeon .emp files carry clobbers against 3 in sigil, so the correction is an AEON-side landing serialized behind the byte chain and routed to them, not landed from sigil.

## SIGIL-DECOUPLE

- state at archive: `open`  size: `L`  project: `-`
- blockedBy: nothing

Item 14. Both owed reads done (§6 a and b). Verdict unchanged - NOT YET - but its argument is rewritten: the census was never stale, and the live risk is a decaying margin routed to the engine lane.

## AGENT-WORKTREE-TARGET-DIRS: CLOSED

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

SUPERSEDED BY THE 2026-09-05 SWEEP and restated to what is left. 49 GB reclaimed, 31 worktrees to 13, 246 branches to 17, every removed tree verified 0 tracked-dirty, 0 untracked and tip in master, locks cleared with unlock and never a second -f. WHAT REMAINS: 12 trees hold something, mostly a single untracked build artifact each, and 7 branches are unmerged and kept by name. The residue needs a per-tree judgement rather than a rule, so it is not a sweep.

**CLOSED 2026-09-06 at the root level by the cleanup the owner asked for: 171.9 GB reclaimed, 227 hidden directories to 122, manifest at docs/2026-09-06-workspace-cleanup.md. What remains is 110 aeon-owned directories (routed to that lane) and 6 registered worktrees of mine, which need a per-tree judgement rather than a rule.**

**Its closing reason was written into PROSE-STATED-BOUNDS's body by the commit that closed four rows at once, so this row read `open` for three days while its own closure sat under an unrelated heading. Moved here 2026-09-09.**

## RESOLVER-FALLS-THROUGH-TO-SHARED-TREE: CLOSED

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

**CLOSED 2026-09-08 on `parcel/resolver-fallthrough`.** The read moved behind the door: `test_support::oracle_legacy_dir` resolves `oracle-old` by the SAME precedence the engine tree uses (`ORACLE_DIR`, then `EMPYREAN_SUITE_ROOT/oracle-old`, then a step-3 derivation that `names_a_reference_tree` refuses), and `m1b_gate` now calls it instead of reading the variable itself. One algorithm over a `CheckoutSpec`, not a second resolver: the second tree is a value.

**THE UNFORCED CASE REFUSES, WITH THE DIRECTORY PRESENT, AND IT IS SHOWN.** `/home/volence/sonic_hacks/oracle-old` and its `linux-port/gui/Symbols.cpp` both exist; with every variable unset, `cargo test -p sigil-harness --test m1b_gate -- --exact oracle_loadfromaslisting_resolves_emit_listing` FAILS with *"NO REFERENCE TREE IS NAMED … This run DECLINED to use /home/volence/sonic_hacks/oracle-old, which step 3 derived"*. `ORACLE_DIR=…/oracle-old` on the same command announces *"reference-tree: /home/volence/sonic_hacks/oracle-old (SUITE_PATHS step 1, named by ORACLE_DIR)"* and passes; `SIGIL_ALLOW_PARTIAL=1` skips it against the stand-in path. Three directions, and the third is not a formality: a resolver that refused unconditionally satisfies the first two.

**THE POPULATION, not the one site.** `reference_env_read_is_routed` ruled exactly `AEON_DIR` and `EMPYREAN_SUITE_ROOT`, and named the gap in its own module doc, *"A THIRD tree-naming variable … until then a read of it would pass unseen."* That gap was live: `ORACLE_DIR` was the third variable, it was being read privately, and the gate was green over it for as long as the read existed. The rule now iterates a `RULED_VARIABLES` list that includes it, so a test reading `ORACLE_DIR` for itself is a failing row. The gap for a FOURTH variable is restated rather than deleted, because it is still real.

**The operator names it too.** `scripts/landing-run.sh` resolves `oracle-old` through the shell include, exports `ORACLE_DIR` to the child, and stamps the path and the step into the log header; `suite_paths.sh` and its Python twin gained the matching marker row (`linux-port/gui/Symbols.cpp` + `Devices`) so a value naming the Rust `oracle/` is a set-but-wrong hard error at its own step rather than a tree with no `linux-port/` in it. Without this the landing run would have stopped inside the suite with the resolver's message instead of at its own front door.

**A LIMIT THE FIX CARRIES, named rather than left to be found.** When `landing-run.sh` resolves at STEP 3 it exports a DERIVED tree under the name of an explicitly-chosen one, and from inside the suite those two are indistinguishable: the resolver sees a variable and cannot see who set it. The `AEON_DIR` line beside it has always made the same trade; the mitigation for both is that the log header stamps the STEP and not only the path, so a reader asking which value was in effect can answer it from the record. The suite cannot answer it on its own, and this parcel does not change that.

**WHAT DID NOT CLOSE.** The gate's CURRENCY question is untouched and this parcel does not touch it: `emit_listing` is still pinned against the legacy C++ reader, and the shipping debugger's Rust symbol loader is still not a subject of any gate here. That is the standing gap-ledger row (`campaign-gap-ledger.md`, const-arity lane 2026-08-22), whose own kill condition says *"it runs again" is NOT a kill*; naming the tree does not make the tree current either.

`crates/sigil-harness/tests/m1b_gate.rs:51` resolves `ORACLE_DIR`, then falls through to the fixed path `/home/volence/sonic_hacks/oracle-old` — another lane's checkout. Found 2026-09-07 by turning aurora's fifth Roster C rule (no resolver in a seat's rig may fall through to a shared last resort) on this lane's own rig; the rule is in the lens UX amendment at empyrean `3ad431f` per the hub's relay, NOT read here.

**The existing mitigation covers the wrong failure.** The gate skips when the sibling repo is ABSENT and fails hard on absent under `SIGIL_STRICT_GATE=1`. On this machine the directory is PRESENT, so the unforced case does not refuse: it measures whatever that tree contains and attributes the result to nothing. Fix shape: the value is set explicitly by whoever runs the gate, and the unforced case is PROVEN to refuse once, rather than assumed.

**Checked and clear in the same sweep, named so the zero has an instrument behind it:** `test_support::aeon_dir` (step 3, the derived sibling, excluded from `names_a_reference_tree` by name at `test_support.rs:795`; a run naming no tree refuses or skips by declared intent), `scripts/landing-run.sh` (refuses at the end of its chain), `P2BIN_BIN` in the three vector generators (derives from an already-explicit `ASL_BIN` that exits 2 when unset, then asserts the result is a file). **`refreeze.rs:785`'s `AEON_DIR` default is now audited and is NOT this class.** It is `unwrap_or_default()`, an empty-string default and not a path, and it is unreachable with an empty value: `resolve_aeon_rev()` five lines above it already refuses the whole `--attest` run when `AEON_DIR` is unset or names no directory, and the local's only consumer is the log header at `:866`. Nothing was changed there.

**Its neighbouring PROSE was stale, though, and that is the same defect the `OVERSEER-REFERENCE.md` note below records.** The doc comment at `refreeze.rs:105` described `capture_goldens.sh` as carrying a `${AEON_DIR:-/home/volence/…/aeon}` fallback that "silently builds against the owner's LIVE working tree". That script refutes it: it resolves through `suite_resolve_checkout` and REFUSES in as many words (*"This does NOT fall back to a live"*). A comment that narrates a fall-through the code has since removed reads as a live hazard to anyone auditing for exactly this class. Rewritten to the present-tense contract fact in the same change.

Related and already fixed: `docs/OVERSEER-REFERENCE.md` claimed `aeon_dir` still defaulted to the owner's live checkout, which the code refutes; corrected at `2802d189`. The practice that paragraph teaches survives, its argument did not.

**How the closing sweep enumerated, so the zeros have instruments behind them.** `git grep -n "/home/"` over tracked `*.rs *.sh *.toml *.py` (129 hits) and `git grep -n "sonic_hacks"`, plus `git grep -nE 'env::var(_os)?\([^)]*\)[^;]*unwrap_or'` for the Rust shape and `git grep -nE '\$\{[A-Z_][A-Z0-9_]*:?-[^}]+\}'` for the shell one, and `/usr/bin/grep -rn -A3 "env::var"` for a fallback `rustfmt` had wrapped onto a following line. POSITIVE CONTROL: every one of those instruments returned `m1b_gate.rs:51`, the known instance, so no zero below rests on a query that silently matched nothing. **In shipped `crates/**` test and harness code the set was exactly one: `m1b_gate.rs:51`.** Outside it, and deliberately not converted, are the throwaway probe scripts under `.f1probe/`, `.s1probe/` and `docs/superpowers/notes/*-probes/`, roughly a dozen `SIGIL=${SIGIL:-/home/…}` lines naming per-parcel target directories rather than a peer's checkout, in scripts that are evidence of a finished investigation and are not run by any gate.

## LENS-UX-SEAT-DIAGNOSTICS

- state at archive: `open`  size: `M`  project: `-`
- blockedBy: oracle's pilot notes on the brief, relayed by the hub; and the in-flight lens fixes and the owed shared-pair refresh, both ahead of it

OWNER-RULED 2026-09-07, verified firsthand at empyrean `97cd725` (an ancestor of their `origin/main`; text at `docs/2026-09-07-lens-ux-seat-amendment.md`, protocol lands in aeon's `LENS_PROTOCOL.md`). His words in the artifact, not the relay: asked whether the lens should have a UX seat, *"I think it should have one right?"*, and on scope *"I think we draft it and run on oracle, aurora, and sigil for now"*. So the owner decision exists AND it is this question; the SEQUENCING (oracle first as pilot) is the hub's on his words and the artifact says so itself.

The sigil variant is pointed at DIAGNOSTICS. **UXa, task walk:** the seat writes wrong code on purpose, three to five classes named in the controller's charter (a typo'd mnemonic, a width mismatch, an unresolved label, a macro misuse, a section overflow), as a newcomer who has read the README and nothing else, and judges whether each message gets them out — logging every stall, guess, and moment it had to read our source to proceed, ranked by time burned. **UXb, heuristic audit:** walk the message catalog against the checklist — can the cause be found from the text, does it say what to do next, is it consistent with its neighbours.

Three conditions that are not optional: every finding ships the diagnostic text VERBATIM (no evidence, no finding — the amendment's own guard against a verdict of "nothing found", which is indistinguishable from clean); the panel is LATE, at a new pin, and the packet is amended to name it; and the no-emulator-MCP line stands unchanged.

**⚠ THE BRIEF HAS MOVED AND THIS ROW WAS WRITTEN FROM THE OLDER TEXT.** Everything above was read at empyrean `97cd725`. The amendment is now at empyrean `6a12740` (same file, +25 lines, verified here an ancestor of their `origin/main`), carrying four rules added from oracle's pre-run reading. **This row does NOT restate them, because this lane has not read them** — bumping a citation to a revision you have not read is the verified-at defect wearing a freshness costume. Two were relayed by the hub and are recorded as RELAYED, not verified: that failing to get a newcomer from launch to a first successful assemble is a FINDING and never BLOCKED, and that the charter names the surface each job is walked in. **THE AUTHORITATIVE TEXT IS THE PROTOCOL, NOT THE AMENDMENT, AND THIS ROW DELIBERATELY DOES NOT PIN ITS REVISION.** Roster C lives in aeon's `docs/superpowers/LENS_PROTOCOL.md`. **Resolve `aeon origin/master` at RUN TIME and read it there**; write the charter from the protocol, never from the empyrean amendment (the draft that fed it) and never from this row, since citing a draft over the landed text is how a superseded rule gets enforced.

**Why no SHA: this row's protocol citation went stale TWICE IN EIGHT MINUTES.** Measured here — that file took two commits tonight, `8def2240` at 19:05 (Roster C lands) and `82540fab` at 19:13 (rule five changes from *proves the unforced case refuses* to *proves WHICH VALUE WAS IN EFFECT*). Both were relayed to this lane as the one to cite, and the second REVERSED the obligation the first carried. A pin into a document being rewritten on that cadence is coordinate-rot at document scale (protocol bar 18's corollary: subscribe to the event you depend on, re-read at point of use), and the failure is not that a pin goes stale but that it goes stale while still resolving, so a session writes a charter to a superseded obligation with a verified SHA beside it.

**Known-good floor, for content presence only and NOT as the revision to cite:** at `82540fab` (verified here an ancestor of their `origin/master`) the file carries Roster C, the fall-through rule, and the artifacts'-location proof form with *"absent from the shared default"* verbatim. If the text you resolve at run time lacks any of those, you have gone BACKWARDS and should ask, not proceed. Grep note from the hub: the landed text capitalises WHICH VALUE WAS IN EFFECT.

**THE FALL-THROUGH RULE HAS A DIRECTION AND BOTH ARROWS ARE LIVE HERE** (oracle's refinement, relayed by the hub; their framing, this lane's instances). Ask what is SHARED and which way this tool moves against it, rather than pattern-matching a resolver.

*Reaching the shared thing* — the seat resolves onto something it does not own: booked as `RESOLVER-FALLS-THROUGH-TO-SHARED-TREE`.

*Becoming the shared thing* — the seat's own run turns into what a LATER run resolves to. **This lane already manages three instances and had never seen them as one class**, which is the framing's real yield here; each was learned from its own incident and none of them names the other two:
1. a build in the main checkout relinks `target/release/sigil`, the assembler aeon's `build.sh` resolves to (it cost a peer four legs assembled by two different binaries on 2026-09-06); managed by the hub-window rule;
2. `~/sonic_hacks/.sigil-pin-af35fa56` (was `.sigil-ls12-pin`, retired 2026-09-07) became a path aeon's `build.sh` currency-checks, so sweeping it turns their provenance to `unknown`; managed by a declaration on both boards;
3. `sigil build --aeon <tree>` WRITES into the tree it is handed; managed by the exclusive-tree rule in every brief.

**The one that is NOT yet managed and is the charter's own risk:** `scripts/nightly_ref_drift.sh` and `scripts/nightly_source_gates.sh` write under `${XDG_STATE_HOME:-$HOME/.local/state}/sigil-{ref-drift,source-gates}` — the reaching arrow in the default, and the becoming arrow in the ledger, which a later run appends to and reads. Audited 2026-09-07: no pollution TODAY (the `--selftest-fail` path exits before writing, and `record_unmeasured`'s `unknown` rows are a deliberate record of a quiet night, not an accident). But **a record names its revisions and its time and NOT its writer**, so a seat that ran either script would append observations indistinguishable from the nightly lane's. **The charter gives the seat its own state dir explicitly and proves WHICH VALUE WAS IN EFFECT, read back from the run's own output** (the hub's generalised form of rule five's proof obligation: terminate the chain explicitly and prove the value, a demonstrated refusal being one such proof and not the only one). That generalisation matters here specifically: the older refusal-only form would have had this lane ADD a refusal to a production nightly script purely so a seat could prove something, which is changing the subject to suit the instrument.

**But the readback is NOT where the relay said it was, and a charter written on the relay would look for a line that is not there.** Checked at `9af282ae`: both scripts mention `$STATE` only inside FAILURE notes (*"see $STATE/build.log"*). A clean run prints no such line. The proof is therefore the artifacts' LOCATION, not a printed path: the run WRITES `nightly.log`, `build.log`, `provision.log`, `gates.log` and the ledger under the resolved dir, so the seat asserts those exist under its own dir and are absent from the shared default. Same obligation, available on the green path, and it still needs nothing added to the script.

**DO NOT START until the hub relays oracle's pilot notes.** The brief is the thing under test on the first run, which is the whole reason oracle goes first — starting early would spend this lane's run on a brief the pilot is about to correct.

## S4BUDGET-STALE-ASSUMPTION: CLOSED

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

✅ GROUNDED 2026-09-07 and ROUTED to aeon; retire at the next boundary. The 2026-09-06 non-match was a search of the WRONG TREE: the row is a finding about aeon's tool, so no artifact for it could exist here. Grounded at aeon tools/s4budget.py (the premise appears twice, in load_vram_layout's docstring and in the user-facing VRAM message) against this tree's built listings, which carry 780 EQU rows of which 32 are VRAM_*, in the form `EQU VRAM_PLANE_A = $0000C000`. The row's figure of 17 is not restated; the measurement is 32 by enumeration. Their CONCLUSION (read vram.toml) survives on the union/overlay_with argument; only the premise is stale. Superseded text follows. ⚠ UNGROUNDED, DO NOT ROUTE UNTIL RESOLVED. The row reads 'their budget tool reports a value UNMEASURED while the data it needs sits in the listing it just read', and on 2026-09-06 that wording matched NO artifact in this tree. The instrument was working (s4budget appears in 14 files), so it is a genuine non-match: the row has either lost its source or never had one. Deliberately NOT sent to aeon as bookable. What IS grounded nearby, and is aeon's independently: row F4 in docs/superpowers/notes/2026-09-05-decouple-aeon-side-inventory.md, EndOfRom equals the ROM file size, structurally un-failable because the disagreement prints as a NOTE and never reaches breaches, so padding, a stale file and a real placement error read identically. Next action is to ground this row or retire it, not to work it.

**CLOSED. Grounded 2026-09-07 and routed to the engine lane, and they landed the fix at aeon `c834cb66`, whose docstring now quotes this lane's own 780 EQU / 32 VRAM figures. The row's own body already carried its closing reason while its heading read `open`, which is the same defect as the four rehomed above wearing a different shape.**

## CLOBBER-PAYOFF-MEASURE

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing; **the blocker LANDED at `ee136317`, verified an ancestor of master by the
  2026-09-08 sweep, so the d-26 ordering is satisfied and this row is runnable now**

The S row d-26 ordered before the L fix may start. Measure what the 76 over-declared clobber lists actually cost across the shipped ROM in bytes and cycles. MEASURE AT THE CONSUMING END: the cost is wasted saves and restores at CALL SITES, so 76 of 387 procs is a producer count and says nothing about how often they are called. A small answer retires the item honestly and that is a real result, not a failure.

## EMP-ORG-TWIN-CHECK: CLOSED

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

ANSWERED by the org parcel and kept only so it is not re-opened: the .emp frontend CANNOT have this defect, because it has no org at all. lower/mod.rs always opens with switch_section_lma, so LMA is a monotone chain that can never take a lower target, and vma: is a VMA pin only. That is a DESIGN DIFFERENCE, not twin agreement, which is the distinction that matters: twin agreement would have proved nothing. No parcel site exists. Close this row at the next boundary.

**CLOSED 2026-09-06. Answered by the org parcel: the .emp frontend cannot have the defect, no parcel site exists. Its own text said to close it at the next boundary.**

**Its closing reason was written into PROSE-STATED-BOUNDS's body by the commit that closed four rows at once, so this row read `open` for three days while its own closure sat under an unrelated heading. Moved here 2026-09-09.**

## PROSE-STATED-BOUNDS

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

Raised by the aeon lane 2026-09-06 from their own instance: inject_editor_bg's refusal told authors 'the limit is 12 KiB' while enforcing 20,480, stale two days after the owner raised the constant. A BOUND STATED IN PROSE inside a diagnostic is a third population, reached by neither an identifier grep nor a quoted-key grep, and it is read at the exact moment an author hits the limit. NOT YET MEASURED HERE: sigil's diagnostics are this lane's own surface and nobody has asked whether any of them state a numeric bound in words rather than deriving it from the constant. The sweep is the measurement, and a zero needs an instrument that could have returned non-empty. Their fix is the shape to copy: derive the figure from the constant and prove it by poisoning the constant and watching every printed figure follow.





## LENS-PASS (closed 2026-09-06, dropped from the board)

- state at archive: `open`  size: `L`  project: `-`
- blockedBy: nothing

DONE and landed (packet 2570f724, 22 of 22 seats); findings booked same day in docs/superpowers/notes/campaign-gap-ledger.md. What remains is the FIX ORDER, which is yours: 4 byte-changing, 4 silent-acceptance or abort, 5 gates that cannot fail, 3 structural, 3 measure-first. Nothing fixed during the sweep by design, and nothing is authorized by being booked.

## PINS-DEAD-TESTS-FIELD-AND-ORPHANS: LANDED, and it was stale in the DISPATCH SLOT within thirty minutes

- state at archive: `next`  size: `S`  project: `-`
- blockedBy: nothing; the engine lane is TOLD before it lands, not asked

The two low lens findings in the pin manifest, `sig-pins-tests-field` and `sig-orphan-pins`, taken
as one parcel because they are one file's two halves. Grounded 2026-09-08 before briefing, and the
grounding changed what the parcel is:

**Half one is already solved and only the cleanup is left.** The manifest's own header says
`tests` IS NO LONGER READ BY ANYTHING: the rerun hint is DERIVED at print time from which
`crates/*/tests/*.rs` actually reference each constant, proved on a real pin move where doctoring
`MAP_TEST_OBJ` emitted all five consuming binaries against a declared list naming one. What remains
is deleting the 412 dead `tests` lines, which the header itself books as belonging on its own commit
and calls the correct end state, for the stated reason that a field nobody reads and nobody
maintains will be read by a future author as a record of something. The generator stamps that field
into the generated file's doc lines, so `pins.rs` is regenerated in the same change and its diff
must be doc lines only, with no pin VALUE moving.

**Half two needed a mechanism check before it could be briefed, and the check inverted the obvious
fix.** An orphan pin is NOT inert. `repin_pins::pins_rs_is_current` regenerates the whole file from
the manifest against the tree and compares the entire text, so every pin, consumed or not,
participates: a symbol that moves in the engine tree makes the generated text differ, the gate goes
red, and someone regenerates. So a pin no test reads still costs a red on unrelated movement, and
still shows what moved. Deleting the 22 would have looked like tidying and would have removed drift
coverage of 22 symbols. **A brief saying "delete the orphans" would have been acted on.**

**RULED HERE, and it follows this repo's own precedent rather than taste:** the zero-consumer set is
REPORTED by the repin tool on every run, enumerated by name, and GATED BY NOTHING. Same shape as
`PROVENANCE-REV-REACHABILITY`, and for the same reason: a declared exception list is a population to
maintain whose failure mode is green because nobody maintained it. Reporting makes the set visible
and count-free, which is what the finding actually asked for. Nothing is deleted on this parcel.

**Both halves touch `crates/sigil-harness/repin.toml` and `crates/sigil-harness/src/pins.rs`**, the
two files the boot read puts on the coordinate list. Under the 2026-09-02 cut the engine lane
freezes alone, so this is a notification rather than a permission: this seat undertook to the engine
lane on 2026-09-08 that they hear before such a parcel lands, not after, and that undertaking is
what governs.

**LANDED at `53bc85de`, merge `d39b7ce9`. The manifest carries zero `tests` lines, re-adding one is a parse error, and `zero_consumer_report` ships.**

**⚠ THIS ROW IS THE SHARPEST STALENESS INSTANCE THIS LANE HAS.** It was written at `fa9d1e1d`, 19:29 local, in state `next`, which is the dispatch slot. The parcel it describes landed at `53bc85de`, 19:59 local. **Thirty minutes, same evening, same repo, both by this seat.** Staleness here was being modelled as multi-day drift and it is two orders of magnitude faster than that: a row can be stale before the session that wrote it has finished its turn. Found by the 2026-09-08 sweep, not by the seat that wrote and then invalidated it.

## INDIRECT-COST-REPORT-UNCOVERED: covered by `parcel/cost-report-shape-test`, both halves

- state at archive: `open` until that branch merges; nothing further is owed by it  size: `S`  project: `-`
- blockedBy: nothing

Booked at the landing of `parcel/quoted-cost-deferrals` (merge 68351ffc) by the controller, against
that parcel's own delivery, because an unstated gap reads as coverage. The gap was real and was
re-measured before it was worked: at `bfd34618` the only occurrence of the flag or either new field
outside their own source was a comment in `crates/sigil-frontend-emp/tests/contract_closure.rs`
citing the command, and `crates/sigil-cli` had no test naming it at all.

**WHAT CLOSED.** `the_indirect_cost_report_is_wired_and_internally_consistent`, in
`crates/sigil-cli/tests/contract_closure_corpus.rs` beside the `--report contracts` gate it mirrors,
drives the real binary over all three shipped shapes and reads its real stdout. It asserts the report
does not contradict itself and asserts no number: each section's declared count against the rows it
renders, the headline cost against the two policy counts printed above it, the row grammars, and that
the walk ran under the target's defines rather than define-free. Proven red-first with three
mutations of the SUBJECT, each shown applied on disk before its run: swapping the cost operands (red
on the cost relation, 0 against 59), a site header that declares 11 and renders 0 (red on the site
relation), and a firing list one row short of its own count (red on the firing relation). Runner:
`cargo test --release -p sigil-cli --test contract_closure_corpus`, which CI's `cargo test
--workspace` already runs.

The script half closed too, since the row named it and it was cheap in the same parcel:
`crates/sigil-harness/tests/s8_seam_size.rs`, beside the existing script tests in that directory.
Two cases. The arithmetic case asserts the report's totals against the rows they total, the headline
against the seam totals, and THE PARTITION, that the move plus the unassigned remainder is the whole
crate; that last relation is the one the silent-undercount class breaks, and it was proven red-first
by dropping a seam from the headline (16644 against 18257, the same failure re-demonstrated) and
again by claiming a module in neither list (741 lines accounted for nowhere). The refusal case builds
a throwaway fixture tree and runs it twice, whole (the control, which must exit 0) and with one
seam-named module removed (which must exit 2 and name it); without the control the exit 2 would be
evidence only that the fixture was broken.

**WHAT DID NOT CLOSE, and none of it is owed by this parcel.**

1. Nothing here says the reported figure is RIGHT. Both gates assert self-consistency, so a closure
   that computed the wrong firing sets would still render a report that balances. That is the
   deliberate limit of a shape gate and the price of a figure that is free to move.
2. The cost relation is partly vacuous while the trusting count is 0: `cost == forced` and
   `cost == forced - trusting` are the same claim today, so a defect on the trusting operand would
   pass. It discriminates on the forced operand now and on both the day the warn tier is non-empty.
   Stated in the test rather than left to be found.
3. The relation is `forced.saturating_sub(trusting)` and NOT `forced >= trusting`, which was
   considered and rejected as a check that fires on correct code: `check_firings` collapses a proc
   whose effective set is TOP into a SINGLE unbounded firing and returns, where the trusting reading
   of the same proc can fire once per register, and `@allow("clobbers.unanalyzable")` suppresses the
   unbounded case outright. A legitimately smaller forced count is reachable, and with the warn tier
   empty nothing would have noticed the assertion was wrong until it was.
4. The cross-check the report prints (grep the source for `as Type` sites the walk cannot see) is
   still a manual step. Nothing compares the walk's site list against the source text, so a site
   inside an unresolved splice template remains invisible to every check, exactly as the report's
   own note says.
5. The s8 refusal is proven on a fixture, not on the real tree, because proving it there means
   removing a harness module.

## CLOSURE-MISSES-FALLS-INTO-EDGE: LANDED 2026-09-09, and one of its two halves had no instance

- state at archive: `LANDED`  size: `S`  project: `-`
- blockedBy: nothing; it was OUR code

**The edge is modelled.** `ProcNode::falls_into` carries the declaration and the closure fixpoint
charges the successor's whole effect to the falling proc, exactly as it charges a tail transfer,
with the same hole treatment for an unresolvable successor. `seam1`'s Z80 node builder had modelled
the same edge since it was written (by pushing the successor into `direct_callees`) and now uses the
field, so one edge has one spelling. Per-row evidence, the adjudication of every baseline movement,
and the red-first proofs: `docs/superpowers/notes/2026-09-09-falls-into-edge.md`.

**The asymmetry it removes, in the corpus's own words.** `Player_SensorFloor` ends
`jbra Player_SensorSurface`; `Player_SensorCeiling` declares `falls_into Player_SensorSurface`. One
transfer spelled two ways, and the closure read the first as clobbering the shared body's ten
registers and the second as clobbering `d6/d7`.

**WHAT CLOSED.** The census half, in full: over-declaring procs drop 90 to 66 on sonic4 plain (71 to
47 debug, 101 to 77 demo plain, 82 to 58 demo debug, 71 to 47 config_a, 91 to 67 config_b, 90 to 66
lean) and the `falls_into` artifact class goes 16 to 0 on every shape. The row's figure of 11 is the
same population under `clobber_payoff`'s bucket ORDER, which counts an empty-effective proc in the
comptime-empty bucket first. The drop is 24 rather than 16 because callers of a falling proc gain its
successor's registers too.

**WHAT DID NOT CLOSE, and it is the half this row called urgent.**
`[proc.clobber-undeclared]` closure firings are **0 before and 0 after, on all seven shapes**. No
proc in the corpus falls into a successor writing more than the head declares, so the suppressed
under-declaration firing this row was booked for **has no instance in today's corpus**. The
mechanism is real and a test now holds it, so the firing cannot be suppressed the day someone writes
that pair; but the row's claim that the destructive direction was the reason to run now does not
survive the measurement, and anyone re-reading the CLOBBER-PAYOFF note's closing paragraph should
read this sentence beside it.

**WHAT MOVED INSTEAD.** `[call.live-clobbered]` (D1c) moved identically on all seven shapes, one row
relocating and six appearing, all of them the player sensors returning their result in an
undeclared `d0`/`d1`. Three rows of that class were already frozen. Each new row is adjudicated at
its own row in `crates/sigil-harness/src/contract_baseline.rs`; the GONE row
(`Air_Collide @ Air_WallProbeRight :: d1`) did not vanish but moved to the later call on the one
path that reaches the read, and `Air_Collide :: d1` is one row before and one row after.
`[proc.dead-save]` is unchanged by count AND by row on every shape, which is the direction the
baseline's ratchet warns about.

**The follow-on this leaves, not booked by this parcel:** the sensors declare no `out(...)` for
results their own headers document (`Out: d0.w dist, d1.b angle, d2.b attr`). Declaring that
surface in aeon would dissolve nine D1c rows at once, and it is an aeon contract change, not a
sigil one.

Original row text, unedited, because a reader who carried it away needs to meet the correction
rather than find the row silently rewritten:

> Booked 2026-09-09 from the CLOBBER-PAYOFF-MEASURE result, and it is the reason that measurement was
> ordered before the fix rather than beside it.
>
> `corpus_contracts.rs` builds callee edges from call and tail mnemonics only, so **a `falls_into`
> declaration is not modelled as an edge at all.** Consequence, measured: 11 of the 90 procs the census
> reports as over-declaring are not loose, they are correct. `S4LZ_DecompressDict` reads as never
> writing 8 of its 9 declared registers, while `S4LZ_Decompress`, the proc it falls into, writes them.
>
> **⚠ THE DIRECTION THAT MATTERS: had the large fix started on the producer count, a register was on the
> list to be deleted from a contract that needs it, along with its caller's correct save.** The
> over-declaration census is the SAFE direction of this gap. The same missing edge suppresses a real
> under-declaration firing, which is the destructive one, and that half wants its own row rather than
> riding this one.
>
> Fix shape: teach the closure the `falls_into` edge. It removes 11 false census rows and closes the
> missed-firing direction in one change, in our tree.

## AS-Z80-FUNCTION-CALL-IN-OPERAND: LANDED 2026-09-09, and it was TWO defects, not one

- state at archive: `LANDED`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing; nothing is left to do

**Shipped on `parcel/z80-function-operand` at `4cc75c75`**, *"as: a user `function` call in a Z80
operand, and the paren the expansion adds"*. sigil now reports `1BC6h` on s1disasm `f6ece657`, the
figure asl (md5 `61e672562465725a8c102288a7da9098`) reports from the same tree at exit 0.

The row's diagnosis was right about the cause and wrong about the count. Fixing what it describes
gets to `1BC5h`, not `1BC6h`, and raises a diagnostic the row does not predict.

1. `lower_z80` handed its operands straight to `parse_operands`, running none of the
   `expand_operand_builtins` layer that `dc.b`/`dc.w`/`dc.l` and the 68000 path both run. There is no
   call syntax in `parse_expr`, so `zmake68kPtr(SegaPCM)` parsed as the bare symbol `zmake68kPtr`
   with `(SegaPCM)` left over. This is what the row describes, and routing through the existing layer
   closes it.
2. That alone lands on `1BC5h` and a NEW refusal at line 197, `unsupported form: Ld, [Reg(B),
   Mem(11)]`. `expand_calls` wraps every expansion in parens for precedence, and on the Z80 an
   operand that is one whole paren group is an INDIRECTION. So the expansion rewrote
   `ld de,zmake68kPtr(SegaPCM)` (3 bytes) into `ld de,(nn)` (4 bytes) and `ld b,pcmLoopCounter(16000)`
   into `ld b,(nn)`, which is not a Z80 instruction: +4 and +0 where the reference has +3 and +2. The
   68000 never saw this because an immediate there carries a `#`, which `classify` settles before it
   looks at parens. The fix lets the WRITTEN shape decide the addressing mode: a group the programmer
   did not parenthesise stays a value however many parens the expansion added.

The row's "substituting literals for exactly those four operands yields `1BC6h`" was a sound
measurement of the GAP and not of the fix: a literal carries no parens, so it never exercised defect
2. The defect the substitution could not see is the one that cost the ninth byte.

Ripple over all four corpus roots, both directions, nothing newly present anywhere: s1 `sonic.asm`
50 -> 46 (4 absent), s2 `s2.asm` 5229 -> 5227 (2), skdisasm `sonic3k.asm` 2424 -> 2417 (7),
`s3.asm` 1433 -> 1429 (4). The two Sonic 3 roots need their generated PCM includes present first;
without them the newly-reached line 4404 reports `unresolved symbol SEGA_PCM.sample_rate`, which is
a corpus-preparation artefact downstream of an include failure both runs already report, not a
regression. Pinned by `crates/sigil-frontend-as/tests/as_z80_function_operand.rs`, expectations read
out of an asl listing committed beside it, red under three separate mutations of the subject
(no expansion; expansion without the written-shape rule; the written-shape rule over-reaching).

## DEAD-SAVE-DOC-AND-FLOOR-STALE

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

`dead_save_corpus.rs`'s doc comment says one firing in every shape. Measured 2026-09-09: **1 in the
non-debug shapes and 10 in the three debug ones.** Its floor is `widest >= 1`, which is tolerant in the
direction that matters: it could never notice growth. Same class as the stale-comment rows this file
already carries, with the addition that the gate's own tolerance hides the drift its comment
misdescribes.

## WORDING-LOCKS-LATENT-TWO: OPEN

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing
- moved out of `docs/lane-status.json` 2026-09-12, under rule 7's 20-row bound, to make headroom

Checked: only ONE test pins a sentence for a feature that is not built yet (field assignment,
`eval_control_flow.rs`); the second was a permanent error, not a wording lock. Low value, and the
reason is worth keeping rather than just the verdict: whoever eventually builds that feature has to
rewrite the test anyway, so the lock costs nothing until the day it is removed as a side effect of
the work it would have obstructed. Moved here rather than closed because "low value" is a judgement
that a later seat may reverse, and a closed row does not get re-read.

## PARTIAL-ROWS-PRINT-PLAIN-OK: OPEN

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing
- moved out of `docs/lane-status.json` 2026-09-12, under rule 7's 20-row bound, to make headroom

Narrower than it was first booked. The partial-run marker IS printed and the nightly check reads it,
so the automated backstop is intact. What remains is that a HAND-RUN suite hides it, because cargo
swallows a passing test's stdout unless asked for it. So the exposure is an operator reading a green
scoped run and not seeing which binaries went unmeasured, which is the same hand-run-has-no-second-reader
shape as `AB-PROTOCOL-CART-UNVERIFIED`. Kill: have the partial banner print to stderr, or have the
runner script surface the unmeasured count in its own summary line.

## ASL-WARN-PARITY-ALIGN: PREMISE UNVERIFIED, do not dispatch as written

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing, but see below

Board text, reproduced verbatim so it is greppable: *"The old assembler warns about a misaligned
address and we say nothing, on every leg of one option. Bytes agree, so it is a missing warning
rather than a bad ROM. We already have this check on the other language."*

**Nothing in the tree records the measurement behind this.** Not the option whose legs were walked,
not which language already carries the check, not the comparison that showed `asl` warning where we
are silent. Measured 2026-09-16 with a positive control. **It was the `next` row**, so a session
taking the board at its word would have briefed an agent to add a warning with no artifact showing
one is missing. Re-derive against `asl` before working it, under this file's own opening rule that a
stale open row does not read as stale, it reads as work.

**Correction, 2026-09-17: the premise WAS in the tree; it was found by content, not by this row's id.**
"Nothing in the tree records the measurement behind this" is wrong. The measurement is
`docs/superpowers/notes/2026-09-16-switch-matrix-sweep.md`, section "Diagnostic parity: sigil is silent
where asl warns, once", commit `6348b831`: asl's `warning #180: address is not properly aligned` on
`move.w (1).w,d0` at `s2.asm:30438` over every `gameRevision=0` leg, sigil silent, bytes agreeing; the
option is `gameRevision`, the other language is `.emp` (`[layout.odd-field]`), and the gap ledger row
"sigil has no odd-address lint on the AS route" carried it. None of those name `ASL-WARN-PARITY-ALIGN`,
which is why a search by row id found nothing. Worked on `parcel/as-warn-odd-address-align`: asl's rule
re-derived by 51 probes, `[as.odd-address]` implemented on the AS front end, notes and probe table in
`docs/superpowers/notes/2026-09-17-asl-warn-180/asl-warn-180.md`.

## S3K-FOUR-CLASSES: CLOSED, Sonic 3 & Knuckles builds whole and byte-identical

- **CLOSED 2026-09-25.** All four classes landed (codepage `8de29c7f`, `$$` labels `f535d779`, bcd and PC-indexed `cbc81b99`); the whole S3K image equals the reference, CRC32 `0658f691`, 0 bytes differ (`docs/superpowers/notes/2026-09-25-s3k-whole-rom.md`). Sonic 3 Complete also identical.
- history, kept:


- state at archive: `open`  size: `M`  project: `-`
- blockedBy: nothing, but see below

Board text: *"Sonic 3 and Knuckles is the next game that does not build: 120 rows in four classes.
Start with the one where our tool accepts a directive and then ignores it, which is the fault shape
we just measured."*

**The only 120 in the tree counts Sonic 1 BUILD CORNERS** (`docs/superpowers/notes/2026-09-16-as-width-suffix-bare-expr.md`),
a different subject entirely. This row's 120 is a count of S3K source rows in four fault classes and
has no artifact. The S3K hits under `docs/superpowers/notes/` are July handoffs about something else.
**Whether a real census was run and never written down, or the figure was absorbed from the adjacent
one, is not decidable from here, and that is the finding.** Re-measure before sizing anything off it.

**CORRECTED 2026-09-25: the premise IS in the tree, and the paragraph above is wrong about that.** The
census is `docs/superpowers/notes/2026-09-16-as-corpus-census.md`, headline 5 and section *Q6. S3K, the
next loud corpus*: 120 rows at sigil `72aca2e2`, skdisasm `2fcd861c`, in four classes (93 `$$name`
labels, 19 `codepage`, 6 `abcd`/`subx`, 2 `(d8,PC,Xn)`), with its raw row multisets beside the note.
The sweep that booked this looked under the right directory and missed it. Re-derive:
`grep -n "120 rows in four" docs/superpowers/notes/2026-09-16-as-corpus-census.md`. The re-measure on
current master is still owed and is step 0 of the parcel now working the row.

## OVER-ACCEPTANCE-THREE-SHAPES: PREMISE UNVERIFIED

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing, but see below

Board text: *"We accept three shapes the old assembler refuses. Your decision puts them in the
settle-everything-else half, so closing them would be a second call, not part of this one."*

**The three shapes are named nowhere in the tree**, so the row cannot be worked without re-deriving
them, and its claim about which half of the owner's decision they fall in cannot be checked against
the decision either.

**CORRECTED 2026-09-25: the three shapes ARE in the tree.** `docs/superpowers/notes/2026-09-10-as-circular-split.md`,
section 6: `c4` (a `rept`/`ds` count naming a forward constant), `c5` (one naming a forward label past
an `org`) and `d2` (an `if` naming a forward label that settles), each refused by asl with *expression
must be evaluatable in first pass* and left accepted because the circular-split ruling's own words put
them in "everything else settles". They are watched: probes `firstpass_rept_forward_constant`,
`firstpass_rept_org_separated`, `firstpass_if_settles_false` in `crates/sigil-frontend-as/tests/as_over_acceptance.rs`,
rows of `over_acceptance/ledger.txt`. Closing them is a second decision, as the row says. Re-derive:
`grep -n "Sigil accepts three shapes" docs/superpowers/notes/2026-09-10-as-circular-split.md`.

## Where these three came from, and the correction to where they were first filed

Found 2026-09-16 by trying to start the `next` row and failing to locate its premise. Swept all 20
board rows with `git grep -l <id> -- docs/` and a positive control: 17 traceable, these 3 not.

**They were first written into `docs/OVERSEER-ROW-HISTORY.md`, which was the wrong file, and the
reasoning that put them there was wrong in an instructive way.** That file's header promises every
board row is reproduced verbatim, so it looked like the home for an untraceable row. **It is a
HISTORY file** — rows land there when they leave the board — **and THIS file is the live tracked
queue.** The seat hand-certified the first artifact it found carrying the right-sounding promise and
never swept for a sibling, which is the same defect the same seat had spent the hour correcting in
two other places. The `ROW-HISTORY` section is kept, since the sweep and its lesson are real, and now
points here for the live rows.

## The 2026-09-18 durability census, and the seven rows it moved here

`docs/lane-status.json` is gitignored in this repo, so a row that lives only on the board dies with
the session. The hub raised it after aurora measured its own board and found 8 of 20 rows in no
committed file at all. Census run here the same hour, one `git grep` per row id against
`origin/master`: **16 rows, 9 already booked in a file that records OPEN work** (this file, or
`docs/superpowers/notes/campaign-gap-ledger.md`), **7 not.** The seven are below.

**The question is not "does the id appear" and the census is wrong in both directions if you ask
it that way.** `docs/lane-log.jsonl` records what HAPPENED and `docs/decisions.jsonl` records a
QUESTION; an id in either looks booked to a naive grep and books no work. In the other direction,
`EDITOR-BINDING-COUPLING` has a whole committed note on its subject
(`docs/superpowers/notes/2026-09-10-editor-binding-cross-repo-coupling.md`) that never says the id,
so a grep calls it an orphan while the argument is durable. **Classify by what the FILE is for, then
check the subject separately.**

Rows keep their board state. Where a row's argument already lives in a committed note, the row
points at it rather than restating it, so there is one copy to keep true.

### SWEEP-OPTIMISED-COMPRESSORS: LANDED

- state: **LANDED** 2026-09-18 at merge `ac314f5c` plus lint fix `a1d0975f`, pushed  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- Landing gate on the merged tree against `.aeon-sigil-ref` (aeon `ec640bcf`): 492 suites, 5,591
  passed, 0 failed, 0 skips, 492 of 492 binaries launched and reported, reconciling 5,581 baseline
  plus 10 new, clippy 0, GREEN, stamped tree `a1d0975f`.
- **It was identification, not implementation.** Both optimised formats are clownlzss's optimal
  parser, already vendored here, so they were byte-identical to p2bin on all 14 vectors first try
  with no tie-break tail. Plain `saxman` fell out free. `kosinskiplus` is now the only refused
  format and no corpus here selects it.
- Sonic 1 under `improved_dac_driver_compression` AGREES end to end (crc32 `faa36f4d`, 524,288 B);
  its cross product goes from 288 of 576 corners comparing nothing to 528 built by both, 528 agree.
- **What the refusal was hiding, now the leg compares bytes:** Sonic 2 under
  `improved_sound_driver_compression` differs by two, and it is not the compressor. Booked as
  campaign-gap-ledger **Open 5**, offsets pinned. Deliberately not refused; the argument is there.
- **The lint the parcel's own run never exercised:** implementing four of the five formats left a
  single-element loop that clippy rejects under `-D warnings`. Population sized without that flag
  at zero other sites, so one site was all of it. The agent reported no clippy run at all, which is
  the gap worth remembering: a returned green that never ran the lint bar is not a landing gate.
- Sonic 1 and Sonic 2 each ship a setting that squeezes data harder
  (`s1disasm/build.lua:10` `improved_dac_driver_compression`, `s2disasm/build.lua:10`
  `improved_sound_driver_compression`), selecting p2bin's `kosinski-optimised` and
  `saxman-optimised`. sigil implements 3 of p2bin's 7 formats and refuses both by name, so half of
  each corpus's swept corners record a refusal instead of comparing a byte.
- Argument and kill condition: `docs/superpowers/notes/campaign-gap-ledger.md`, the 2026-09-17
  `SWEEP-NIGHTLY` section, **Open 1**. Only the row ID was absent from a committed file; the work
  was booked there all along.
- Oracle: the corpora's own committed `build_tools/Linux-x86_64/p2bin`, which runs here.

### BRA-W-RANGE-UNCHECKED

- state: **open (watch)**  size: `S`
- ANSWERED and good: where the old assembler refuses a too-far jump, we refuse too, same file and
  line. No silent wrong ROM. Kept only until the standing check that now guards it
  (`scripts/switch_matrix_sweep.py`) has run a few nights.
- **Kill:** delete the row once the nightly has carried the check for several consecutive greens.
  Nothing announces that; it is a date check, not a red.

### SOUND-BLOB-REFREEZE-SP6

- state: **blocked**  size: `M`  blockedBy: aeon, then the owner (their card `SP6-MODULATION-DIVERGENCE`)
- Not a deadlock, though two boards read like one: the engine lane can merge on his ear alone, and
  our pin follows and repairs their build. Three sites plus a refreeze, not one line.
- Was booked ONLY in `docs/lane-log.jsonl`, which records landings and books nothing open.
- The wait is aeon's to carry and the hub has the card in front of him as the suite's oldest; this
  lane chases neither.

### EDITOR-BINDING-COUPLING

- state: **open**  size: `M`
- Four places in our tool are keyed to a name the engine's own tools invent. Rename a scene binding
  there and our checks break, with nothing in either repo saying why. Booked, not fixed.
- Full argument, committed and durable:
  `docs/superpowers/notes/2026-09-10-editor-binding-cross-repo-coupling.md`. That note does not
  carry this id, which is why the census flagged it; the subject was never at risk.

### EMP-RESIDENT-CONST-USE: CLOSED

- **CLOSED 2026-09-25, nothing built:** `d-31` answered `by-name` **by the HUB** in the owner's place (empyrean `796b508f`), the route that already works. He can reopen it with one word.
- history, kept:

- state: **blocked**  size: `M`  blockedBy: the owner, card `d-31` (filed 2026-09-12)
- Should a shared constant be importable between sound modules the way it already is elsewhere? A
  language question for him: four options costed, one already works today.
- Was booked ONLY in `docs/decisions.jsonl`. **A decision entry books the QUESTION; it does not book
  the work that lands behind the answer**, and this row is where his go arrives.
- Language surface, so it is propose-discuss-land under `d-6`: the design is this lane's, the
  agreement is his.

### DEB2-Z80-LABEL-IN-68K-TABLE

- state: **open**  size: `S`
- The debugger's name list files a sound-processor label at main-CPU address 0x8000, so a real label
  there loses its name. Needs a check against the old assembler and a talk with the engine lane first.
- Argument, and the reasons it was ranked out of `next` on 2026-09-12:
  `docs/superpowers/notes/2026-09-12-deb2-minus-20-bytes.md`, which names this id and carries the
  per-tree check (`grep -E ' : 8000 [A-Z] \|' <shape>.lst`). Booked in a dated note rather than
  here, so nothing swept it.

### PUB-STRUCT-SIZE-MANDATORY

- state: **open**  size: unmeasured (absent rather than guessed)
- The language spec says shared records must state their size, but nothing enforces it and 42 engine
  records do not. Enforce it or drop the word: ours to propose to him once something depends on it.
  Spec marks it open, empyrean `b5fe6efa`.
- **The only row of the seven booked NOWHERE**, under any name. Adjacent but not the same subject:
  `d-32` (answered) is about the size check running only when something else in the build happens to
  use the record. Re-derive the 42 before quoting it; it is a snapshot.

### AS-OVERACCEPT-SILENT-THREE: LANDED

- **LANDED** 2026-09-25 at gated merge `e5b6ee06`, pushed; 5640 passed, 0 failed. Function arity was already fixed at `1b8cb9ae`.
- was: state **doing** from 2026-09-25  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- The three over-acceptances the 2026-09-10 gate found that no ruling chose to keep:
  `dir_padding_not_on_off` (`padding maybe` silently turns padding ON; `supmode` shares the helper),
  `expr_function_arg_count` (no arity check on user `function` calls) and `save_missing_restore`
  (the save stack is never checked at end of unit). Rows 44 to 46 of
  `crates/sigil-frontend-as/tests/over_acceptance/ledger.txt`. Unlike `c4`/`c5`/`d2` these are not in
  any ruling's keep-settling half: each is a typo or mistake asl refuses, and the first builds a ROM
  with a layout flag set the wrong way. Refusing them the way asl does is assembler internals under
  the autonomy directive, with a lane-log note.

### OVERACCEPT-LEDGER-STALE-ROW-SILENT: LANDED

- **LANDED** 2026-09-26 at merge `538382a0` (tip `53749139`), pushed. Ruled: a ledger row that no longer diverges from asl FAILS the gate (`no_stale_ledger_row`), both directions, naming the row. First run retired `dir_unknown_codepage`. 1046 passed, 0 failed.
- was: state **open**  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- The over-acceptance gate REPORTS a ledgered row that sigil now refuses and does not FAIL on it,
  so a fixed over-acceptance can stay listed as open indefinitely. Measured 2026-09-25:
  `expr_function_arg_count` was fixed at `1b8cb9ae` on 2026-09-11 and sat in
  `crates/sigil-frontend-as/tests/over_acceptance/ledger.txt` for two weeks, and a parcel was
  dispatched to fix it. Decide whether a retirable row should fail the gate (forcing retirement in
  the fixing commit) or whether reporting is enough; the evidence is the two weeks. Source:
  `docs/superpowers/notes/2026-09-25-as-overaccept-silent-three.md`.

### CLIP-OWN-ANCHOR-PRICING: LANDED

- **LANDED** 2026-09-25 at merge `011fcc5e`; merged-tree gate GREEN, 503 suites, 5723 passed, 0 failed
  (5707 + 16 new). Aeon's end-to-end verdict at `a1ff8796`: the format works. Their half lands after
  the shared binary is refreshed in the hub's window.
- was: state **open**, unblocked 2026-09-25: the owner answered `d-35-revised` = `clip-overlay-file` (record
  `d-35-revised-answered` in `docs/decisions.jsonl`; heard directly by aeon, relayed ~14:20Z, and on
  the console card). Build: a new `sigil build` switch naming a small positions file in the clip's own
  folder, written the way `map.toml` writes anchors, applied only when passed; `map.toml` untouched.
  Sigil lands first (switch + file format contract, and the emit must read the same positions),
  then aeon adds the file and passes the switch. Aeon re-derives the values as the act grows (their
  `7d409cdb` costing: one more CPZ section already needs 0xC0000/0xD0000 in DEBUG), so nothing here
  pins them. The companion check is `BANK-ID-EMIT-VS-PLACE-UNCHECKED`.
- landed on branch `parcel/clip-overlay` (not merged): `--anchor-overlay <path>` on `sigil build`
  and `emit_sound_blob`, one-pass apply over the map anchors and the frozen island rows, every
  refusal with its id; contract for aeon in
  `docs/superpowers/notes/2026-09-25-clip-overlay-contract.md`. Aeon passes the switch to
  `sigil build` only: on the preflight emit it reds
  `test_check_does_not_perturb_generated_sound_artifacts` (measured, see the contract).
  Canonical byte-neutral (s4, s4_debug, demo, demo_debug equal the provenance tip); landing run at
  `eb73887d` GREEN, 5723 passed, 0 failed (baseline 5707 + 16 new).
- was: state **blocked** on card `d-35`  size: `M` to build  project: `-`
- Priced 2026-09-25 at merge `603087e0`: `docs/superpowers/notes/2026-09-25-clip-own-anchor-pricing.md`.
  Sigil cannot tell the S2CLIP build from canonical sonic4, so the hub's own-anchor ruling (empyrean
  `e229e1a3`) needs a new map.toml key plus a build flag (design A, recommended) or a narrower flag
  (C). That is surface the engine's layout file is written in, so it is `d-35`. Gates aeon's row 8
  zone work, nothing tonight. Sigil lands first; aeon then adds the rows and passes the flag.

### MAP-WHEN-UNKNOWN-VALUE-SILENT: LANDED

- **LANDED** 2026-09-25 with CLIP-OWN-ANCHOR-PRICING, merge `011fcc5e`.
- was: state **open**  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- `when_applies` in `crates/sigil-harness/src/map_placement.rs` treats any `when` value other than
  `sound_on`/`sound_off` as applying to every build (`Some(_) => true`). A typo in map.toml's `when`
  silently places a row in every shape. Both aeon maps use only the two known values (per the pricing
  note, inferred from a read, not a build), so refusing unknown values should move no bytes. Folded
  into design A of `d-35`; worth doing alone if `d-35` waits.
- landed on branch `parcel/clip-overlay` (not merged, commit `0a383b11`): `when` is a closed enum
  (`[map.when-unknown]`), and `[[anchor]]`/`[[hole]]` rows refuse an unknown key. Byte-neutral:
  the four canonical shapes equal the provenance tip.

### BANK-ID-EMIT-VS-PLACE-UNCHECKED

- state: **open**  size: `S`  project: `-`
- `emit_sound_blob` bakes sound-bank ids into the resident Z80 blob and `dac_sample_tab.bin` (aeon
  measured `ld a,$17 ; call SetBank` at blob 0x235, 0xD5E, 0xED4, 0x122C, `ld a,$15` at 0x41F, table
  bytes 0x15/0x16) before `sigil build` places the banks. Nothing is known to check that the baked ids
  equal the placed banks' ids, so a mismatch would play sound from the wrong bank silently. Our pricing
  note flagged the cache half (the sound fold check covers MT/SFX addresses, not DAC bank ids). Add a
  link-time check, proven red by moving a bank. Needed by `d-35-revised` either way; worth doing
  alone. Source: aeon 3f208336, docs/research/2026-09-25-clip-own-anchor-pricing.md.
- landed on branch `parcel/bank-id-emit-vs-place` (not merged): `[sound.bank-id-vs-placement]` in
  `crates/sigil-harness/src/sound_bank_ids.rs`, run after link in every full chained build. It reads every baked
  id out of the linked image, finding each site by re-emitting with the id moved, and compares it with
  `bank_id_of` of the placed bank label. It measured a hole the existing checks miss: DAC relocated to a
  consistent anchor plus an extra window in `dac_banks.emp` builds green at master with drums baked one bank
  low; the branch refuses it. Four shapes byte-identical to the provenance tip. The note's `ld a,$15` at 0x41F
  is the YM timer rearm, not a bank id.

### AS-REGISTER-SPELLED-LABEL-SILENT: LANDED

- **LANDED** 2026-09-25, fast-forward to `09af6350` (gate run at `29f64ad8`: 504 suites, 5728 passed, 0
  failed, 5723 + 5 new). 47 over-accepts refused, one rule; aeon four shapes and five corpus ROMs unchanged.
- was: state **open**  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- With a label named like a register (`A1:`), sigil assembles `#A1+Lab2` where the pinned asl refuses
  it as a register, so a program asl rejects builds here with a value. Pre-existing, measured by the
  `$$` parcel with a `$$`-free control: `docs/superpowers/notes/2026-09-25-s3k-dollar-labels.md` and
  its gap-ledger rows (which also carry the `.`-local scope not following `cpu`/`padding`/`supmode`/
  `listing`/`restore`/`endstruct` as asl's does).
- On branch `parcel/as-register-label` (measurement `a2080de5` and `4d18bacc`, fix `63a678a5`): asl's rule
  measured over 127 probes (`docs/superpowers/notes/2026-09-25-as-register-spelled-label.md`): on the 68000
  `d0`-`d7`/`a0`-`a7`/`sp` in any case is the register in every expression whatever symbol exists. sigil now
  reads it so: 47 over-accepts to 0 (i09, j07, j08, z01 all refused with a sentence naming the shadowed
  symbol); the 58 accepted shapes keep asl's bytes; S1, S2, S3K, S3C, S3 byte-identical and refusal sets
  unchanged. The 6 former BYTES-DIFFER rows are now named refusals (silent-empty `dc` ruling, and
  `AS-REGISTER-ALIAS-SYMBOL` below).
  Aeon's four shapes at `ec640bcf` match the provenance tip (s4 `91c46c94/820209`, s4.debug
  `8a378de6/846509`, demo `1c7a34d3/96863`, demo.debug `72e405a5/103185`). Landing run at `29f64ad8`:
  GREEN, 504 suites, 5728 passed (5723 + 5 new), 0 failed, 2 ignored, clippy and ledger clean.

### AS-REGISTER-ALIAS-SYMBOL

- state: **open**  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- asl lets `X equ d3` and `X reg d3` name a register: `move.w X,d0` is `3003` (exit 0). sigil refuses both
  (`d3` is a register, not a value; `reg` is not a recognized 68000 mnemonic). With `A1:` defined,
  `X equ A1` is the alias of `a1` to asl (`3009`); sigil wrote `3038 1200` there before
  AS-REGISTER-SPELLED-LABEL-SILENT and refuses the line by name after it. Probes `alias_equ_d3`,
  `alias_reg_d3`, `alias_equ_A1_label` in `docs/superpowers/notes/2026-09-25-as-register-spelled-label/`.
  An over-refusal, never a wrong byte; no corpus or aeon source uses it (all build unchanged).

### PORT-TESTS-RED-AT-AEON-TIP

- state: **landed on branch** `parcel/port-tests-aeon-tip` (awaiting merge)  size: `M`  project: `SIGIL-DECOUPLE`
- Outcome 2026-09-25 (`docs/superpowers/notes/2026-09-25-port-tests-aeon-tip.md`), pin `ec640bcf`,
  tip held at aeon `8a6f92c4`: all 26 measured failures and the predicted `ojz_run_b_port` failure
  were repaired. The `ojz_run_b_port` one was MASKED behind an earlier section's byte failure;
  the gate now lowers every section first. None of the 14 port binaries fails on a name at the tip
  (26 name-shaped lines on master@tip, 0 on the branch). All 14 are green at the pin, and the full
  suite at the pin fails only the environmental `m1b_gate` row. Against pins regenerated at the tip
  (scratch tree, committed `pins.rs` untouched) 38 of the 40 tests are byte-identical.
  `test_objects_port` was a real bug, proven red-first: embeds joined the module directory instead
  of the aeon root. Values are derived from the tree under test: `sonic4_shape_defines`, zero-byte
  declaring modules, `listing_labels_if_defined`, family sweeps, and per-shape
  `mddbg_entry_labels`.
- OPEN for the pin advance: `ojz_run_b_port`'s declared `ojz_act_assets` shape delta (0xF) is
  stale at the tip. The actual delta is 0x5EC2, the three `OJZ_DEBUG_TEST_BGS` backgrounds. Derive
  it from the module; do not raise it. Masking in the other multi-section byte gates is unprobed.
- Measured 2026-09-25 (`docs/superpowers/notes/2026-09-25-aeon-481ac02e-port-impact.md`): against aeon
  at `b2820dc3`/`481ac02e`, 167 sigil tests fail that pass at the pin `ec640bcf`. 141 are expected
  (bytes and pins moved past the pin). **26 are unresolved-name failures in `*_port` tests** from aeon
  commits after the pin (`OJZ_Preset_Night`, `BG_VSCROLL_*`, `BG_Bands_Hold`, `GAME_SCANLINE_CAPS`,
  `CRASH_REPORT`, `MDDBG__ErrorHandler`/`DMA_Queue_End`, `Region_Resolve`/`Plane_Buffer_Peak`, and a
  doubled `embed` path in `test_objects_port`). None is from `481ac02e`. Also re-check `parallax_port` for aeon `6e1a4f80`'s new `pub equ band_record_len = sizeof(band_record)` when the pin moves (inferred same-module, not run). These must be repaired
  before the pin next advances; they are the port tests' hand-assembled dependency seams drifting.
- Predicted by aeon 2026-09-25, not run here: aeon `7c0baa73` (verified reachable from their
  `origin/master`) adds `use games.sonic4.ojz_clip_act_act1.{OJZ_CLIP_ACT}` to
  `games/sonic4/data/levels/ojz/act1/act_assets.emp`. `ojz_run_b_port` compiles that file standalone
  supplying only `BG_LAYOUT_SIZE`, so at the next pin advance it should report `unknown name
  OJZ_CLIP_ACT` unless the port supplies it, derived from aeon's neutral
  `games/sonic4/data/generated/ojz/act1/clip_act.emp` (value 0 there), never a copied literal.

### PORT-TESTS-HIDDEN-NAME-FAILURES

- state: **done in branch** `parcel/port-hidden-names` (awaiting merge)  size: `M`  project: `SIGIL-DECOUPLE`
- Outcome 2026-09-26 (`docs/superpowers/notes/2026-09-26-port-hidden-names.md`), pin `ec640bcf`,
  tip held at aeon `9caa1368`: a static scan of all 4511 tests (`mask_scan.py`, banked beside the
  note; positive control: it flags `ojz_run_b_port` at `78f5c7ca^` and clears it at HEAD) flagged
  66 aeon-reading tests; hand triage found NINE, in seven files, where a comparison that can fail
  on aeon drift ran before another module, shape or link was produced: `ojz_run_a_port` (2),
  `seam2_seq_colink` and `seam2_sfx_head_colink` both-shapes gates, `soundbankhead_port`'s pin
  test, `game_debug_port`'s flip, `seam2_phased_head` (2), `section_row_fixture`'s both-spellings
  gate. All nine now produce every part first (`a6691120`, `79c1cd94`), green at the pin, and
  red-first proven: the same mutations (earlier part wrong, a real unknown name in the later part)
  hide the name in the old structure and report it in the new one.
- At `9caa1368` nothing was hidden: every restructured gate reports the same first failure as
  before, or stays green. `game_debug`'s link stage is unmeasured there (resolve stops first on
  a visible name). The other 57 flags are single-unit, same-source reruns, or invariant checks;
  the note classifies each.
- OPEN: 12 tests fail on VISIBLE names at `9caa1368` (`Music_Service` in `game_loop`,
  `Music_Want` in `parallax`/`sound_api`), from aeon commits after `8a6f92c4`. They need the
  derive-from-the-tree repair before the pin advances past them.
- Suites at `6cd988ea`: pin 504 binaries, 5732 passed, 0 failed, 2 ignored; `repin --check`
  says `pins.rs unchanged`. Tip: the same 176 failures as before the parcel, 12 name lines both.

### MODULE-REGISTRY-HARDCODES-AEON-FILES

- state: **done in branch** (`parcel/module-registry-derive`)  size: `M`  project: `SIGIL-DECOUPLE`  asked by: aeon (HOLDING its deletion on us)
- 2026-09-26 DONE IN BRANCH: `registry()` / `demo_registry()` and the config_a/config_b push/filter edits are deleted.
  The build's module roots derive from `games/<g>/map.toml` `order` over the scanned tree
  (`crates/sigil-harness/src/module_roots.rs`): a label row names the in-scope module defining it (private heads
  included), `section:<name>` the module declaring it, `__align$<module>$N` its module; a row nothing defines is
  `[map.order-orphan]`, a doubly-defined row is settled by the `use` graph or `[map.order-ambiguous]`. Enumeration
  table + measurements: `docs/superpowers/notes/2026-09-26-module-registry-derivation.md`. RESIDUE kept in sigil and
  named: `SHAPE_GATES` (21 modules only some shapes place; the map's union `order` cannot say which; proposed home a
  per-row `when` on `order`), `ENGINE_TERMINUS` (`engine.epilogue`: the demo map has no `EndOfRom` row), and
  `SIGIL_PROBE_FIXTURES` (sigil's own `sigil_objroutine_probe`, which duplicates `TestSolid_Init`). `DECLARED` is left
  as is: a stale row is inert (measured), but a NEW section still needs a row by the R7 ruling; deriving it needs a
  ruling. Pinned tree ec640bcf, branch sigil f94cc693, all four shapes at the provenance tip: s4 91c46c94/820209,
  s4.debug 8a378de6/846509, demo 1c7a34d3/96863, demo.debug 72e405a5/103185. Aeon origin/master d3b98868 with
  path_swap deleted (the `.emp`, its `order` row, its objects.json entry, plus the two stamp files
  `tools/regenerate-level.sh` rewrites; generated level data unchanged): master sigil refuses (`no module
  games.sonic4.path_swap found under the scan root`); this branch builds s4 cf8606bb/829140, s4.debug
  08e41ec3/856276, demo 259fb22b/99127, demo.debug 8f135bcb/106273 (demo equals the undeleted control). Full landing
  run at f94cc693 GREEN: 506 suites, 5742 passed, 0 failed, 2 ignored. Aeon may delete once this lands and the
  shared pair is rebuilt.
- 2026-09-26: aeon wants to delete `games/sonic4/objects/path_swap.emp` (owner ruling: layer lines are the only layer-switch
  mechanism; aeon confirmed the path is `games/sonic4/objects/path_swap.emp`; aeon's half is booked blocked on this row) plus its `map.toml` row and
  `objects.json` entry. The build refuses, because sigil's whole-program module list is hard-coded:
  `crates/sigil-harness/src/native.rs` `registry()` (the `m!("games.sonic4.path_swap", "path_swap")` row) and
  `section_align.rs` `DECLARED` (`d("ObjDef_PathSwap", 2, WORD)`). Checked here 2026-09-26. `pins.rs` `PATH_SWAP`,
  `repin.toml`'s path_swap region and anchors, and `test_g4_final_objects_port.rs` describe the PINNED tree (ec640bcf,
  which has the file) and do not block aeon; they move with PIN-ADVANCE.
- The trap: deleting the registry row breaks builds of the pinned tree, which still ships the module. So the list must be
  derived from the tree being built, most likely from `map.toml`'s sections, which aeon edits anyway; never from a
  file's mere presence. Same class as BLOB-LEN-PIN-OFF-EMIT-PATH: a pinned-corpus fact sitting on the build path.
  First step: enumerate every other hard-coded aeon module or anchor on the build path, not just this one. Order agreed
  with aeon: sigil lands first and rebuilds the shared pair (from a durable path, then LOCK it, see
  INSTALLED-BINARY-READS-BUILD-TREE), then aeon deletes.

### INSTALLED-BINARY-READS-BUILD-TREE

- state: **done in branch**  size: `S`  project: `SIGIL-DECOUPLE`
- 2026-09-26, branch `parcel/installed-binary-selfcontained` (tip in the parcel report). The seven
  `golden/offcanonical_sizes/*.txt` tables are compiled in (`native::FROZEN_TABLES`, `include_str!`); rejected:
  resolving them from a named location at run time, which moves the dependency instead of removing it. Enumeration
  (binary strings + source grep, classified), consumers and evidence: `docs/superpowers/notes/2026-09-26-installed-binary-selfcontained.md`.
  The only run-time read of the build tree on either binary's path was `load_frozen_table`, and only `sigil` reached
  it: master's `emit_sound_blob` emits byte-identically with its tree removed. Falsifier, binaries built in a
  throwaway worktree that was then removed: master `c024aba8` fails all four aeon shapes with `read frozen table ...
  No such file`; the parcel builds all four at the provenance tip's CRC32/size. Consequences: the `.lst` digest loses
  its one `root=sigil` row (the table is now part of the assembler, covered by `DIGEST-ASSEMBLER revision=`), so
  aeon's `artifact_provenance.py` no longer opens anything in the sigil tree either; the `--version` closure now
  follows include/path references to their files (38 paths, was 29); a prebuilt `REPIN_BIN` in `refreeze` step 3 now
  keeps the tables it was compiled with (default `cargo run` recompiles). Gates: `installed_binary_needs_no_build_tree`
  (sigil-cli and sigil-harness, `bwrap` hides the checkout), `frozen_tables_embedded`, the extended
  `version_provenance` closure mirror; each red first.
- The standing rule below ("lock the build tree at swap time") is UNNECESSARY for a pair built from this change or
  later. It still binds the pair installed today (built at `4ce2509d`): keep `.worktrees/land-4ce2509d` locked until
  a pair built from this change is swapped in. The unlock is the overseer's. Heads-up for aeon at that swap: the
  digest's `reads=` drops by one (the `root=sigil` row).
- 2026-09-26: the shared `target/release/sigil` reads `<build tree>/crates/sigil-harness/golden/offcanonical_sizes/*.txt`
  at RUNTIME through a baked `CARGO_MANIFEST_DIR`-style path. The pair was built at `.worktrees/land-4ce2509d`; removing
  that worktree after the swap broke every aeon demo build ("read frozen table ... demo.txt: No such file", exit 101)
  until it was recreated at the same path and commit. It is now `git worktree lock`ed with a reason. STANDING until
  fixed: the tree an installed pair was built from is part of the install; lock it at swap time and remove it only
  after a later pair is swapped in from elsewhere. The fix: embed the frozen tables (`include_str!`) or resolve them
  from a named reference tree at run time, and grep the binary for every other baked build path (`strings | grep
  .worktrees`) before choosing. Found by aeon.
- Correction from aeon, same day: sonic4 shapes broke too while the tree was missing (a panic reading
  `offcanonical_sizes/s4.txt`), not only demo. The frozen tables of every shape are read at run time.

### BLOB-LEN-PIN-OFF-EMIT-PATH: LANDED

- **LANDED** 2026-09-26 at merge `4ce2509d` (tip `4fc0ccfa`), pushed. The emit writes the resident blob at the tree's
  length (no length refusal, `SIGIL_BLOB_LEN_DRIFT` removed); `BLOB_LEN_*` are pinned-corpus test assertions
  (`emitted_blob_lengths_are_the_pinned_corpus_lengths`, sigil-cli); `seam1_emit_length` (nightly source-gate list)
  proves a grown blob emits. No ceiling in the emit: aeon's `ensure(Z80_SOUND_SIZE <= SND_STATE_BASE)` is in
  `engine/system/boot_data.emp` (not `sound_constants.emp`, which holds only the comment). Merged-tree re-gate 23/23.
  Shared pair rebuilt from a clean worktree at `4ce2509d` and swapped by rename: sigil `1edd31eb`, emit_sound_blob
  `3c3bd0ba`; previous pair (ab0a5fe1) kept at `~/sonic_hacks/.sigil-pair-archive/ab0a5fe1/`. Old vs new emit on
  ec640bcf: 19/19 files byte-identical; with +17 B in `Psg_HwCh` old refuses, new writes 6193/6323. Aeon told.
- For PIN-ADVANCE: aeon `e6a00773` landed the PSG envelope fix on this pair: blob 6194 B plain / 6324 B debug,
  `Z80_SOUND_SIZE` `$1832` / `$18B4` (aeon's figures, not measured here), no new cross-seam names. `BLOB_LEN_*` and the
  Z80_SOUND_SIZE mirrors move with the pin when it passes that commit.
- was: state **doing** from 2026-09-26  size: `S`  project: `SIGIL-DECOUPLE`
- Asked by aeon 2026-09-26: a PSG envelope fix (`PsgEnvUpdate` once in `Seq_HookNoteOn`, +17 B resident; evidence
  aeon `67a7a374` `docs/research/2026-09-26-s2-music-volume.md`) is refused by `emit_sound_blob`: "plain blob is 6193
  bytes, expected 6176". `seam1::BLOB_LEN_PLAIN`/`BLOB_LEN_DEBUG` is both the pinned-corpus length assertion and a
  hard refusal in the emit path aeon's build runs, so a re-pin reds our pinned tests. Ruled here: the refusal leaves
  the emit path (a ceiling only if derived from the tree), the exact length stays a test assertion against the pinned
  tree. Aeon guards the real hazard itself (`Z80_SOUND_SIZE <= SND_STATE_BASE`). After landing: rebuild the shared
  `emit_sound_blob` and tell aeon the SHA. Branch `parcel/blob-len-off-emit-path`; agent tree
  `/home/volence/sonic_hacks/.aeon-sigil-ref-blob` (remove after).

### OJZ-RUN-B-SHAPE-DELTA-BOUND: LANDED

- **LANDED** 2026-09-26 at merge `0631e755` (tip `a45cb079`), pushed. `crates/sigil-cli/tests/ojz_run_b_port.rs`:
  `ojz_act_assets`' plain/debug span allowance is `ALIGN_PAD` plus the debug-only bytes `act_assets.emp` declares
  through `[u8; LEN]` types (evaluated both shapes). Pin: 0 extra. Aeon origin/master: 24,258 extra; the real delta
  there is estimated, not measured, until the pin advances. Red-first proven.

### CART-CHECK-CITES-STALE-PEER-BEHAVIOUR: LANDED

- **LANDED** 2026-09-26 at `375ad832`, pushed. The comment now states our own reasons (one round trip, whole-image coverage) and cites the contract's `memory_hash` entry (section 6) and 11.48 by section, naming no peer tool. `ab_cart_check` 24 passed, 0 failed. Announced to aeon.
- was: state **open**  size: `S`  project: `-`
- `crates/sigil-harness/golden/ab/cart_check.py:53` says aeon's `tools/evict_witness.py` "already runs
  exactly this shape" (a whole-image `memory_hash` as its stale-binary guard). At aeon `origin/master`
  that file no longer mentions `memory_hash` at all: `git -C ../aeon log -S memory_hash origin/master --
  tools/evict_witness.py` names `8c83475a` as the commit that removed it. The claim decayed with no edit
  here. Reword it to state our own reason without citing a peer's behaviour. The file sits under
  `golden/`, so it is a comment-only edit announced to aeon, not a silent one. Found 2026-09-25 while
  checking aeon's consumer notice for `27684931`, which itself changes nothing we execute.

### AS-MULTICHAR-SQUOTE-STRING: LANDED

- **LANDED** 2026-09-25 at `485fbd95`, pushed; Sonic 3 alone byte-identical (`9bc192ce`). 5699 passed, 0 failed.
- was: state **doing** from 2026-09-25  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- The whole remaining gap for Sonic 3 alone (`buildS3.lua`): 6 rows, all a multi-character `'...'`
  in `dc.b`, which asl treats as a string (one charset-translated byte per character; `'AB'+1` gives
  `4143`) and sigil's lexer turns into one integer. Requoting those 6 operands gives the reference md5
  under sigil. Probes sq1 to sq4 in `docs/superpowers/notes/2026-09-25-s3k-whole-rom/`.

### AS-CLI-DEFINE: LANDED

- **LANDED** 2026-09-25 at `ae50976a`, pushed; 5694 passed, 0 failed.
- was: state **doing** from 2026-09-25  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- `-D NAME[=VALUE]` on the assembler path, asl-exact: a SET variable (later `set` may change it, `=`
  and `equ` refused as asl #2035). Removes the S3K wrapper root. Ruled here, logged: NOT wired to
  `Options.defines`, whose documented let-the-file-win semantics would accept two programs asl
  refuses. Measured in `docs/superpowers/notes/2026-09-25-s3k-whole-rom.md`.

### SEAM2-SONG-COUNT-LITERAL

- state: **done in branch** `parcel/seam2-song-count`  size: `S`  project: `SIGIL-DECOUPLE`
- Asked by aeon (Sonic 2 zone music): the `mt_bank` cross-seam carrier restated `SONG_COUNT` as
  `if debug { 3 } else { 1 }` (and `SONG_MOVINGTRUCKS = 1`), so an aeon-only song addition fired
  `mt_bank.emp`'s drift guard in sigil's emit. Now `seam2::song_id_carrier` evaluates
  `games/sonic4/config/sound_ids.emp`'s `pub const`s through `eval_all_pub_consts` under the shape's
  `DEBUG`; `seam2::mt_bank_carrier_asm` is the one carrier source. Same fix at every other consumer:
  `mt_port.rs`, `mt_negative_probes.rs` (both carriers), `sound_api_port.rs` (`SONG_COUNT = 3` in both
  shapes, now per shape).
- `size = 0x79F9` was the room from the ORIGINAL `mt_bank` LMA `$58607` to its bank top `$60000`, never
  re-derived when the LMA became map-derived: at the pin (LMA `0xB8630`) the region ended at `0xC0029`,
  `0x29` past the `$8000` window. Now `(sound_bank + 0x8000) - mt_bank_lma` (`0x79D0` at the pin), the
  bound `mt_port.rs` already used. Placement-only; no byte change.
- Positive control, scratch copy of the pin tree, `emit_sound_blob` master vs branch: authority-only
  debug arm 3 -> 5: master exit 0 (blind), branch fires `SONG_COUNT drifted ...: 3` (the local mirror
  lags). Full aeon-side change (authority 5, mt_bank local 5, two more table rows): master fires
  `SONG_COUNT drifted from games.sonic4.sound_ids: 5` (aeon's reported failure), branch exit 0 with
  20-byte debug SongTable/SongPatchTable. Unmutated: master and branch emits identical (19 files).
- Adding a song is still a TWO-file aeon change (`sound_ids.emp` and `mt_bank.emp`'s local
  `SONG_COUNT` + table rows): the local mirror and its drift guard are aeon's design, not sigil's.
- Not fixed, same class: `sound_api_port.rs` supplies `SFXID_RING_RIGHT/LEFT = $33/$34` as literals.
- Proof at pin `ec640bcf` (`.aeon-songcount-pin`), code tip `8193dcaf`: `repin --check` "pins.rs
  unchanged"; full strict suite 504 binaries launched / 504 reported, 5729 passed, 1 failed (the
  environmental `m1b_gate::oracle_loadfromaslisting_resolves_emit_listing`), 2 ignored. Red-first:
  both a formatter literal and a core literal turn `mt_bank_carrier_song_count_follows_the_authority` red.

### SHARED-PAIR-SWAP-F52609FE

- state: **done 2026-09-25T23:07:11Z** (outgoing pair frozen at `.sigil-outgoing-1d19e60b`, md5s matched; the six steps were issued as one parallel batch rather than read one by one, outcome verified correct)
- original state: **waiting**  size: `S`  project: `-`
- Asked by the hub 2026-09-25 for aeon's music step 4. The candidate pair is built ASIDE from origin/master
  `f52609fe` in the durable detached worktree `/home/volence/sonic_hacks/.sigil-pin-f52609fe`
  (`target/release/sigil` md5 `cdf3ec1abe0d33d8042b7cf5092e5e64`, `emit_sound_blob` md5
  `ed45bedb7ab47972d570536b8d215d22`; `--version` clean, closure-revision `f52609fe`). The outgoing shared
  pair at `/home/volence/sonic_hacks/sigil/target/release/` is `1d19e60b` (sigil `fed84c1967b3b92a57402e86d494cc89`,
  emit_sound_blob `3f2e7322d1cdb1e41dec32ed1637d5e8`). Re-measure all four md5s before acting.
- **Swap ONLY when both are true:** aeon reports its two running builds done and its byte-identical control
  against the candidate clean, AND the hub opens the window. A control difference is a finding: stop.
- **Each step is its own tool call, read before the next; never a `set -e` chain** (zsh does not stop on it):
  1. `mkdir /home/volence/sonic_hacks/.sigil-outgoing-1d19e60b` and copy the live pair into it.
  2. md5 the copies and compare with the live pair. Do not continue unless both match.
  3. Copy each candidate binary into `target/release/` under a temporary name, md5 it against the candidate,
     then `mv` it over the live name (a rename, so a running reader keeps the old inode).
  4. md5 the live pair against the candidate, run `target/release/sigil --version` (expect `f52609fe`,
     source `.sigil-pin-f52609fe`), freeze the outgoing directory, tell aeon and the hub.

### SEAM1-BANKED-CARRIERS-DERIVE

- state: **done in branch** `parcel/seam1-banked-derive`  size: `M`  project: `-`
- Asked by aeon 2026-09-25; blocks their S2CLIP-REGION-MUSIC step 2 (Sonic 2 PSG envelopes). Same family as
  SEAM2-SONG-COUNT-LITERAL.
- Premise verified here at master `7c081077`: `crates/sigil-harness/src/seam1.rs` `banked_carriers()` hand-pins
  ELEVEN `$8000`-window VMAs. `check_banked_carrier_drift` compares only the three HEAD-level ones
  (`SeqOpcodeTable`, `SfxBlobWinTab`, `SndDefaultPitchTable`, derived by `seam2::banked_head_vmas`); its own doc
  comment says the rest (`FmPitchTableZ`, `LogVolumeLutZ`, `CarrierMaskTableZ`, `PsgDivisorTableZ`,
  `PsgVolEnv_Ids`, `PsgVolEnv_Ptrs`, `FmVolEnv_Ids`, `FmVolEnv_Ptrs`) are offsets inside `SoundTablesZ80_Head`
  and "stay hand-maintained and unchecked". These literals are baked into the shipped resident driver's operand
  bytes, so a stale one is a WRONG blob that a refreeze would bless. Also `seam2.rs` `SOUND_TABLES_Z80_LEN = 0x357`.
- Aeon's measurement (theirs, not re-run here): their branch `parcel/s2-music-envelopes-table` at `f7dda3e2`
  (pushed to aeon origin) appends 5 envelope bodies, table 855 -> 985 B; the drift check refuses on the three
  head members, while `PsgVolEnv_Ptrs` 0x828F -> 0x8294, `FmVolEnv_Ids` -> 0x83B7, `FmVolEnv_Ptrs` -> 0x83BA move
  silently. Updating only the three reported numbers would go green with every envelope read from a wrong address.
- Ask: derive ALL eleven and the length from the lowered `sound_tables_z80.emp` (its own labels), so a table
  change is aeon-only; retire or widen the drift check accordingly. Byte-neutral at the pin by construction.
  Positive control: aeon `f7dda3e2` builds under the new code with correct envelope addresses; master refuses it.
- Outcome: all eleven carriers and the table length are derived. `seam2` lowers and links
  `sound_tables_z80.emp` once and reads each label's window VMA off the RESOLVED section (section `vma:` plus
  final label offset); `seam2::banked_carrier_vmas_in` = the three head members from `sound_layout` then the
  eight table labels (`SOUND_TABLES_BANKED_LABELS`, names only). `seam1::banked_carriers(aeon, ov)` is that
  derivation on every emit path, under the caller's anchor overlay. An absent label is an error naming it; a
  table `vma:` that disagrees with the map's `sound_bank` window is refused. The size-only path (run from
  inside `sound_layout`) links every carrier at a `$8000` placeholder (`BANKED_CARRIER_SIZE_PROBE`, same
  argument as `DAC_SAMPLE_TABLE_SIZE_PROBE`). `SOUND_TABLES_Z80_LEN` is gone; the colink gate slices by the
  emitted length.
- Drift check RETIRED, not widened: with the carriers derived from the authority it compared against, it
  could only compare a derivation with itself. Replaced by `seam2` unit tests (the real table with one id and
  one pointer appended in memory: each carrier moves by exactly what precedes it; absent label and window
  mismatch refused by name) and `tests/banked_carrier_derivation.rs` (renamed from `banked_carrier_drift.rs`,
  `nightly_source_gates.sh` follows): all eleven produced in order, table labels inside the table, and each of
  the eleven moves the linked resident blob when doctored (every one is read by the driver).
- Byte-neutral by construction at the pin `ec640bcf`: the derivation yields exactly the eleven retired literals
  and 855; `emit_sound_blob` master vs branch, 19 artifacts byte-identical.
- Positive control at aeon `f7dda3e2`: master `emit_sound_blob` refuses naming only the head three; the branch
  derives `PsgVolEnv_Ptrs` 0x8294, `FmVolEnv_Ids` 0x83B7, `FmVolEnv_Ptrs` 0x83BA, head 0x83D9/0x84E1/0x85F3,
  length 985 (0x3D9): aeon's figures, and an independent Python count of the `.emp` agrees. The emitted blob
  carries each at its `ld hl`/`ld de` operand (plain blob `+0x16A1`: `11 94 82`), and the `ld b` scan count
  `PSGVOLENV_COUNT` reads 16.
- FOR AEON, f7dda3e2 still does not build: aeon restates the table length itself.
  `soundbankhead.emp` walls `_sound_tables.len == $357` and `_dacsamp.len == $7F`, and `dac_sample_tab.emp`
  has `DAC_HEAD_PREFIX = $357 + ...` (which sizes `DacHeadPad`, 7 -> 5 at 985 B). All loud (the walls fire).
  With those three edited in a scratch copy, `sigil build --native --game sonic4` builds (crc 8b9761e2, 822231
  B), the listing places the heads at 83D9/84E1/85F3/8633, and ROM `$B8000` holds the 985-byte table.
- Not fixed, same class: `seam2_layout_derivation.rs` pins the layout as frozen literals by design (its
  `pitchtable_lma` encodes 855) and refreshes with the pin; `PITCHTABLE_LEN = 264` and the `size = 0x400`
  sound-tables region are hand values; the 2026-08-26 note in `tests/repin_pins.rs` still describes the
  carriers as unchecked (a dated record, left as written).
- Proof at pin `ec640bcf` (`.aeon-sigil-ref`), full strict suite, `ORACLE_DIR` named, target dir outside the
  checkout: before (code `ec0a6106`) 504 result lines, 5730 passed / 0 failed / 2 ignored, exit 0; after (tip
  `d2fe50b9`) 504 result lines, 5732 / 0 / 2, exit 0 (two drift tests retired, two derivation tests and two unit
  tests added). `repin --check` "pins.rs unchanged" before and after; clippy `-D warnings` exit 0. Red-first,
  each mutation shown then restored from the commit: a restated `FmVolEnv_Ptrs` turns
  `banked_table_carriers_follow_the_table_labels` red while the pin's blob byte gate stays green (the blind
  spot); an absent label mapped to 0, a `PsgVolEnv_Ptrs` off by one (byte gate red at `ld de` `+0x16A1`), a
  doctor that skips `FmVolEnv_Ids`, and a head member off by one each turn their gate red.

### SHARED-PAIR-SWAP-AB0A5FE1

- state: **done 2026-09-26T03:20:12Z** on the hub's go, each step its own call (outgoing pair frozen at `.sigil-outgoing-f52609fe`, file and directory, refusal observed; md5s matched)
- original state: **waiting**  size: `S`  project: `-`
- For aeon's Sonic 2 envelopes (their S2CLIP-REGION-MUSIC step 2), which need SEAM1-BANKED-CARRIERS-DERIVE in the
  shared pair. The candidate is built ASIDE from origin/master `ab0a5fe1` in the durable detached worktree
  `/home/volence/sonic_hacks/.sigil-pin-ab0a5fe1` (`target/release/sigil` md5 `3fb7c0b35df4ce3e5cd4dfdbb83710d8`,
  `emit_sound_blob` md5 `f0fedab42d72ad1b5cf734591c18a57f`; `--version` clean, revision `ab0a5fe1`). The outgoing
  shared pair is `f52609fe` (sigil `cdf3ec1abe0d33d8042b7cf5092e5e64`, emit_sound_blob
  `ed45bedb7ab47972d570536b8d215d22`). Re-measure all four md5s before acting.
- **Swap ONLY when both are true:** aeon reports its byte-identical four-shape control against the candidate clean
  with no aeon build running, AND the hub opens the window (it polls aurora and oracle, then says go). Aeon's report
  alone is not a go. A control difference is a finding: stop.
- Same steps as SHARED-PAIR-SWAP-F52609FE above, with `1d19e60b` read as `f52609fe` and `f52609fe` as `ab0a5fe1`.
  Each step is its own tool call, read before the next.

### PIN-ADVANCE-S2-ENVELOPE-RESTATEMENTS

- state: **done in branch** `parcel/pin-advance-s2` (awaiting merge; the standing reference must move with it, see
  the note's last section)  size: was `S`, measured `L` (2161 aeon commits)  project: `SIGIL-DECOUPLE`
- Outcome 2026-09-28 (`docs/superpowers/notes/2026-09-28-pin-advance-s2.md`): refrozen at aeon `1ee78b88`, entry
  `pin-advance-s2`, chain len 207; s4 `50401674/845398` and s4.debug `198fd717/865722` equal aeon's figures.
  Baseline at unmodified master against the new tree 202 failed; after the freeze 52; after the repairs 0.
  Landing run at `9b0a9d80` against `.aeon-sigil-ref-1ee78b88`: GREEN, 520 suites, 5858 passed, 0 failed, 2 ignored.
  `repin.toml` `soundbankhead` now ends at `section:soundbankhead` (0x6B0, aeon's reading confirmed, no literal).
  `repin --check`: `pins.rs unchanged`. Findings for aeon: `mt_bank.emp`'s module id (`[module.path-mismatch]`),
  and `tools/test_extern_guard_reachability.py` is present at the new pin.
- **Also red at the next pin advance (measured 2026-09-28 by the SONG-BANK-2-EMBED-ROOT agent against aeon
  `478fa091`, not re-run here):** `mt_negative_probes.rs` wrong-bank probe expects 5 co-residency ensures to fire
  (aeon master has 9, 0 once aeon deletes them); `mt_port.rs` guard count expects 7 (aeon has 11, 2 after). Once
  aeon respells `mt_bank.emp`'s embeds root-relative, the probes that lower it with the sound dir themselves also
  break (`mt_negative_probes` straddle and wrong-bank, two `mt_port` region tests, `sfx_negative_probes` (e));
  `mt_bank_port.rs` reads `mt_bank_blob.emp`, which aeon deletes. Record:
  `docs/superpowers/notes/2026-09-28-song-bank-2-embed-root.md`.
- **Aeon landing notices to fold in at the next pin advance (2026-09-28, their words, not verified here):**
  `d33fa8d0` (+8 B inside `Parallax_State`, every later RAM address moves; new names `Parallax_Shadow_Key_Config`,
  `_VS`, `_K`); `2d006c4c` (new `Cache_H_Pfx_Run: u16` in ram.emp, `H_PFX_ARM = 128` plus an ensure in
  constants.emp; canonical s4 `80dc4338/830244`, debug `3a8a8e5a/857227`). `*_port` lists may need the new names.
  `ac17f3a6` (song-bank-2 step 2: new driver proc `SndDrv_SongBankIn`; the Z80 blob grows +20 B so labels from the
  blob end through `ObjCodeBase` move +20, and one debug jump in `GameState_OJZScroll_Init` goes long, +2; s4
  `76835839/830244`, debug `dd3cf064/857247`). A Z80 blob size change touches seam-2's predictions and the sound
  pins: re-derive them at the advance, never copy these figures.
- Aeon's heads-up 2026-09-26 (their agent's reading, NOT verified here): once aeon lands
  `parcel/s2-music-envelopes-table-2` (tip `6c3cd4cf`), these sigil-side restatements go red when our pin reaches
  it: `seam2.rs` `DAC_SAMPLE_TAB_LEN = 127` and `seam2_dac_head_colink.rs` (the table is now 120 B; the 8-byte pad
  moved into `soundbankhead.emp`, sized from the heads' `.len`); `seam2_layout_derivation.rs` pins `pitchtable_lma
  0xB8357` and the later heads; `pins::SOUNDBANKHEAD` length used by `soundbankhead_port` (head now `$6B0`); the
  "VMA $8357" comment in `seam2_pitchtable.rs`. Also aeon `movingtrucks_pitchtable.emp` vma moved `$8357` -> `$8000`
  window base (emitted .bin unchanged per their cmp).
- Second heads-up 2026-09-26 (aeon's reading, NOT verified here): aeon `b90cd650` adds the two Sonic 2 songs and
  renumbers song ids (`SONG_S2_EHZ=2`, `SONG_S2_CPZ=3`, DrumTest 2 -> 4, HCZ2 3 -> 5; `SONG_COUNT` 3 plain / 5 debug).
  Our `SONG_COUNT` derivation took it unchanged, but any sigil test or off-canonical golden (`--config-a`/`-b`, the
  hotkeys profile) naming song ids by number, or pinning song-bank offsets or sizes, moves when the pin reaches it.
- Ask when it is worked: derive what can be derived (the SEAM1 pattern), leave the pin-frozen ones to the refreeze,
  and re-derive the list from the tree at the commit rather than from this row.
- Measured at aeon `d7103d30` (with committed AND tip-regenerated pins): `docs/superpowers/notes/2026-09-26-port-music-names.md`,
  section PIN-ADVANCE. All five items confirmed; `soundbankhead_port` stays red after a repin (its length is a `repin.toml` literal).

- Sequencing, 2026-09-26: aeon says the advance (including the `repin.toml` `soundbankhead` `len` 0x630 -> 0x6B0 edit)
  is sigil's to schedule under the retired paired-freeze ruling, holds nothing for it, and will measure any aeon-side
  fact on request. That settles WHO sequences it, not WHETHER: advancing the pin is a decision, tested against the
  hub's 2026-09-02 ruling that the pin exists so the corpus does NOT track tip (`docs/OVERSEER-REFERENCE.md`, *A TASK
  RELAYED AS ROUTINE*). Run `refreeze --check` first to know whether it is a repair or a decision.
- Asked aeon 2026-09-28 (sigil session message, on the hub's steer): name a quiet aeon revision for sound and RAM
  layout (suggested after their PPB4 and woven sizing) and confirm the sound bank head span there. We advance to THAT
  SHA, never to tip, and edit `repin.toml` `soundbankhead` `len` to match it. Aeon sets the timing; nothing is held.
  Resume when their reply names the revision.
- AEON ANSWERED 2026-09-28: advance to aeon `1ee78b88` (verified here: reachable from their origin/master, contains
  song bank 2 `e03c6f1d` and PPB-4 `bf6557ef`). They report nothing in flight that moves sound bytes or
  Parallax_State/PlayerBlock/Act sizes, and will warn first if that changes. Their FAST build there on our `1173bb31`
  pair: s4 `50401674/845398`, s4.debug `198fd717/865722` (their CRCs, NOT recomputed here). Soundbankhead span, their
  READING, not a measurement: no end symbol in the listing; `PHASE SoundTablesZ80_Head VMA $8000 LMA $B8000`, last
  data label DacSampleTable at VMA $8633, next placed label Song_MovingTrucks at LMA $B86B0, so len 0x6B0 counting
  the pad. Song bank 2 did NOT lengthen soundbankhead (it is a separate section, SongBank2_Head at LMA $C0000); the
  ask sent to them wrongly guessed it would. Check our derivation against theirs, never copy it; the missing end
  symbol is why the row suggests re-expressing the `repin.toml` region with an end symbol. Row is UNBLOCKED.
- Third heads-up 2026-09-26 (aeon's agent's reading, NOT verified here): aeon `f136c486` (slope landing fix) adds a
  cross-module name `Player_SensorLand` (`games/sonic4/player/player_sensors.emp`, used from `player_air.emp`); the
  port tests run against aeon master report it as the only unresolved name, 39 failed / 24 passed, same at their base
  and tip. Needs the derive-from-the-tree repair before the pin passes it. Possibly coming, only if the owner picks it:
  `LayerLine`, `Act.act_layer_lines`, `Player_LayerLines` (geometry constants in `engine/system/constants.emp`).
- Fourth heads-up 2026-09-26 (aeon's reading, NOT verified here): that branch LANDED at aeon `332cc1ba`. New cross-module
  names: `LayerLine` (engine/structs.emp), `Act.act_layer_lines` (Act 46 -> 50 B), `LL_*` (engine/system/constants.emp),
  `PlayerBlock.ll_prev`/`ll_cursor` (games/sonic4/config/ram.emp), `Player_LayerLines` (player_common.emp),
  `OJZ_CLIP_LAYER_LINE_ROWS`/`ojz_clip_act_layer_lines` (generated clip act). A struct-size move touches every port
  that lays out `Act`. Coming: `Player_LoopCrossover`, `CrossoverTable`, `XOVER_*` retire in favour of lines (owner
  ruling, per aeon).
- Fifth heads-up 2026-09-26 (aeon's reading, NOT verified here): aeon `19978b00` REMOVES `Player_LoopCrossover`,
  `CrossoverTable` (named by our `test_p1_player_port.rs`, `repin.toml`, `pins.rs`), and `XOVER_NONE`/`XOVER_TO_A`/
  `XOVER_TO_B`/`XOVER_LAYER_BIAS`; `PlayerBlock` is 8 B shorter (`ll_prev` @20, `ll_cursor` @24); ADDS `OJZ_Act1_LayerLines`
  and module `games.sonic4.ojz_layer_lines_act1` (`OJZ_ACT1_LAYER_LINE_ROWS`).
- Sixth heads-up 2026-09-26 (aeon's reading, NOT verified here; "nothing owed"): aeon `b62f559c` renames the page cache.
  ADDS RAM `Page_Live_Masks`, `Page_Live_RowPtr`, `Page_Live_Unit`, `Page_Live_Seen`, `Page_Live_Forced` (DEBUG only);
  constants `PAGE_LIVE_UNITS`, `PAGE_LIVE_COL_OFFSET`; procs `PageCache_PickVictim`, `LiveSweep`, `LiveSweepAll`,
  `LiveReset`, `LiveUnion`, `ReleaseHolds`. CHANGES `PatchRun_Col` (new `d1` input). REMOVES `PageCache_ResetRefcounts`,
  `PF_EVICTABLE`, `PF_EVICTABLE_BIT`, `Page_Audit_Scratch`; `pf_refcount` renamed `pf_pad`. Their agent notes our
  `tile_cache_port` name list lacks these, as it already lacked GPL-2's `PageCache_DemandHoldTick`; untested.

### PIN-ADVANCE-STEP0 (A9)

- state: **done**, landed (entry #208 at aeon `936439fa`); since superseded by #209 at aeon `99cb963d`, see below
  project: Empyrean v2 step 0
- Outcome 2026-11-17 (`docs/superpowers/notes/2026-11-16-pin-advance-step0.md`): refrozen at aeon `936439fa`,
  entry `pin-advance-step0`, chain len 208; s4 and s4.debug equal aeon's figures. `repin.toml` unchanged.
  The tip `pin-advance-s2` was attested first (passed vs 1ee78b88), because the freeze refused to append on an
  unattested tip. Baseline 67 failed; after the freeze 3; after the repairs 0 (final gate in the note).
- 2026-11-18 (`parcel/pin-advance-99cb963d`, wave-3 checklist step 7): SUPERSEDED by entry #209
  `pin-advance-99cb963d` at aeon `99cb963d`. #208's strict run recorded RED at sigil `b674ed34` (`ea6195a3`):
  wave-3 sigil refuses the unpaired `936439fa` by design. Byte-neutral; see the step-0 note, step 7.

### PORT-TESTS-MUSIC-NAMES-AT-TIP

- state: **LANDED 2026-09-26 at merge `354fcac4`** (parcel tip `fd08b99a`)  size: `S`  project: `SIGIL-DECOUPLE`
- Outcome 2026-09-26 (`docs/superpowers/notes/2026-09-26-port-music-names.md`), pin `ec640bcf`, tip held at aeon
  `d7103d30` (`.aeon-music-tip`): THIRTEEN tests stopped on a name, not twelve. The thirteenth,
  `tranche5::misspelled_extern_slot_is_loud`, hid `Music_Want` behind a bool control (it failed the same way at
  `9caa1368`); its control now reports the messages, proven red-first. Repaired from the tree under test:
  `Music_Service` from each shape's listing (`game_loop_port`), `Music_Want` and `Music_Current` (the second was behind
  the first) from the listing (`parallax_port`, `sound_api_port`), harness-private carriers in the synthetic probes
  (`game_debug` flip, `tranche5`). `drain_define_is_load_bearing` asserted a literal 4-byte delta; the tip gates two
  calls, so the delta is now counted from `game_loop.emp`. `game_debug`'s flip link stage, measured at the tip: green.
- Name-shaped lines at the tip 12 -> 0; failing set 191 -> 186, none left fails on a name (byte, length, golden,
  pin or provenance drift, plus the S2 restatements the PIN-ADVANCE row covers, not all of which a repin clears).
  Against pins regenerated at the tip (scratch copy) all 15 tests in the five binaries pass.
  Suites at `9d38e63d`: pin 504 binaries, 5732 passed, 0 failed, 2 ignored, `repin --check` says `pins.rs unchanged`.

### SONG-BANK-2-STUB-PROBE

- state: **LANDED 2026-09-28 at merge `599008e8`** (parcel tip `4fe49de3`), sent to aeon  size: `M`  project: `-`
- Results: `docs/superpowers/notes/2026-09-28-song-bank-2-probe.md`. Without the anchor plain/debug refuse
  `[map.undeclared-island]` at 0xC0000; with it every shape builds, head at 0xC0000; a refreeze does not pin it.
  Debug has 1,508 B of bank-1 growth before `[map.anchor-absent]` (aeon quoted 1,509: the test is strict).
- *(The ask as filed, kept below.)*
- The ask, from aeon's overseer, full reasoning at aeon `f49dd2bd` `docs/research/2026-09-27-song-bank.md`, section
  "Our side: which shape". Shape (a): a plain-address bank (vma == lma), `$8000`-aligned, placed in `map.toml` order
  right after `Sfx_33` and before `GameState_ObjectTest_Init`, so seam-2's predicted chain is untouched.
  1. `section_align.rs` DECLARED: `d("SongBank2_Head", 0x8000, Z80_BANK_WINDOW)`. The label is a placeholder until they
     write the module, so check whether anything refuses a row whose label no shape defines.
  2. `module_roots.rs` SHAPE_GATES: `("games.sonic4.song_bank2", ShapeGate::SoundOn)` (the hand list
     `ORDER-WHEN-SHAPE-GATES` would retire; a 22nd row is acceptable meanwhile).
  3. seam-2 stops owning the song tables: aeon makes `SongTable`/`SongPatchTable` ordinary linker-resolved label arrays;
     seam-2 drops the `SongTable` export requirement (`seam2.rs:1562`) and the three-way split, and keeps measuring the
     MT length for the `Sfx_33` prediction. Land a TOLERANT version first (accepts either), so their byte-identical step
     lands alone. Expected movers: `mt_bank_port`, `mt_dual_carrier`, `seam2_layout_derivation`.
  4. Confirm a refreeze will not pin `SongBank2_Head` (no frozen row; the walk places it by alignment).
  5. Then the stub-bank probe: a COPY of their tree, stub bank 2 after `Sfx_33`, with and without the anchor.
- Verified by this lane 2026-09-27 (source reading at `b1c9bb58`): their two corrections both HOLD. (i) A gap over
  `ANCHOR_GAP` (0x400) before a section makes it an inferred island that needs a declared `[[anchor]]`
  (`native.rs:3406`, `[map.undeclared-island]`), so shape (a) needs their anchor at 0xC0000 and no frozen row. (ii)
  seam-2 refuses an `mt_bank` without `SongTable` (`seam2.rs:1562`).
- **Hazard to put in the brief and to aeon: a gap-inferred anchor is content-dependent in both directions.** Their
  measured gaps are 9,277 B plain and 2,533 B debug. If bank-1 content grows to within 0x400 of 0xC0000, no island is
  inferred and the declared anchor fires `[map.anchor-absent]`; past 0xC0000 the bank jumps to 0xC8000 and fires too.
  Both are loud, never silent, but a routine song edit can break the build with an error about placement.
- `section_alignment_declared.rs::the_requirements_above_16_are_declared_for_the_anchored_sections` and the header of
  `section_align.rs` say every row above 16 is an anchored island. The new row is the first `$8000` section the
  walk ROUNDS rather than holds, so both texts need amending in the same parcel.

### PPB-1-DECODE-FACTOR-PIN

- state: **open**, scheduled with aeon when they want it  size: `S`  project: `-`  asked by: aeon, 2026-09-28
- aeon's PPB-1: inlining `Parallax_Update`'s `Decode_Factor_A`/`Decode_Factor_B` calls saves about 0.5k cycles a
  tick, but `crates/sigil-harness/src/contract_baseline.rs` pins those call sites (`Parallax_Update` @
  `Decode_Factor_A` :: d2 in the invariant list, the `_B` twin in `D1C_DEMO_EXTRA`). The ratchet is two-way BY
  DESIGN (module header: a vanished row can mean the closure narrowed and a live save now reads dead), so their
  inline is refused until the rows go. Not a tolerant-form fix: remove both rows in a landing paired with theirs,
  after confirming the rows vanish because the calls are gone, not because the analysis narrowed.
- Their notice the same day: aeon `d33fa8d0` adds 8 B inside `Parallax_State` (every later RAM address moves) and
  three cross-module RAM names (`Parallax_Shadow_Key_Config`, `_VS`, `_K`); relevant at our next reference-tree pin
  advance, not before (reference tree stays at `ec640bcf`).

### AS-PERF-FIX-PEAK

- state: **LANDED 2026-09-28 at merge `a1bfb6fb`** (tip `809b2a35`), with fixes 5 and 6  size: `S`  project: `SIGIL-AS-REPLACEMENT`  from: AS-PERF-MEASURE fix 3
- Peak RSS S1/S2/S3K 72->49, 110->94, 244->185 MB; wall ratios 0.842 / 0.835 / 0.867 (load 10.9 to 16.8). Record:
  the note's "After AS-PERF-FIX-PEAK" section. Left open there: `sigil emp` single-file link still lends sections;
  the front end's per-pass growth (S3K 141 -> 163 -> 190 MB) has no owner measurement.
- `run_asm` keeps the front end's whole module alive while layout builds a second copy of every section. Drop or
  consume it first. Estimated peak cut 15 to 30 percent (S3K 243 to about 190 MB). Low risk; byte identity on
  S1/S2/S3K and aeon is the bar. Then fixes 5 and 6 (substitution fast path, builtin-scan skip), both small.

### AS-PERF-FIX-PASSES

- state: **LANDED 2026-09-28 at merge `b7d80a2b`** (tip `9664f887`; review record `8772017f`), fixes 1 and 2; fix 4
  (macro block structure once) NOT done, still open  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- Two passes on S1/S2/S3K like asl; ifdef now positional as in asl. An adversarial review broke the first version
  (silent wrong bytes); the rework passed a second round. Record: the perf note's "Passes" section and
  `docs/superpowers/notes/2026-09-28-passes-adversarial-review.md`. Pre-existing sigil-vs-asl differences it lists
  (abs.w stand-in, forward values accepted in if/rept/org, case-insensitive ifdef, string equates) are unbooked.
- (1) converge when no value a pass READ changed (S2/S3K third/fourth passes follow only ADDED names, each assigned
  before read); needs a complete read log including `defined`/`ifdef`; about 25 to 30 percent. (2) why pass 1 lays
  out differently (S1 first move `EEgg_Wait`, S2 `paddingSoFar`); investigate first, size S. (4) macro block
  structure computed once. High risk: a pass stopped early is a silent wrong ROM, so every step needs byte identity
  on all three corpora and aeon plus a planted control that an early stop is caught.

### SOUND-FOLD-GATE-SILENT-SKIP

- state: **LANDED 2026-09-28 at merge `a8b1d9a5`** (tip `e5f2a458`); record
  `docs/superpowers/notes/2026-09-28-sound-fold-gate-silent-skip.md`. Open from it: a head label defined in two
  sections takes the first match, not refused.  size: `S`  project: `-`  found: SONG-BANK-2-EMBED-ROOT review, 2026-09-28 (agent's reading, not
  re-verified here)
- `validate_sound_fold` (`crates/sigil-harness/src/native.rs`, about line 3307) silently skips a label it cannot
  find. If `Song_MovingTrucks` is renamed or stops being its section's first item, the check that the placed MT bank
  equals seam-2's prediction goes quiet with nothing red. After aeon's song-bank-2 change the fold gate is one of
  the four checks standing in for the deleted co-residency ensures, so a vacuous pass there matters more. Stage 0:
  confirm the skip, then make a missing expected label loud (a planted rename must go red).

### SONG-BANK-2-EMBED-ROOT

- state: **LANDED 2026-09-28 at merge `116aa23e`** (tip `6b878e3e`; onto master as `5c483786`)  size: `S`  asked by: aeon
- seam-2 lowers `mt_bank.emp` with its embeds relative to the aeon root or the sound dir; ambiguous, mixed and
  computed paths refuse loudly. Record: `docs/superpowers/notes/2026-09-28-song-bank-2-embed-root.md`. Aeon owes
  its own pytest audits a respelling (`tools/test_smps_import.py`, three in `tools/test_song_packer.py` look for
  the literal `embed("song_*.bin")`).

### PAIR-REBUILD-AFTER-AEON-STEP1

- state: **DONE 2026-09-28T11:51:59Z**, pair `1173bb31`, after aeon `cd262846`; pre-checked on their tree  size: `S`  owed to: aeon, 2026-09-28
- Rebuild the shared pair at our then-current master once aeon's step 1 (native song tables, root-relative embeds,
  ensures deleted) lands, so they build under the loud fold gate (`a8b1d9a5`). Held back on purpose so the stricter
  gate did not land mid-step. Their shape: `Song_MovingTrucks` first in `mt_bank`; the tables at the tail of
  `sfx_bank_blob` after `Sfx_33`; neither head renamed. If the gate then refuses their tree, it is a finding on
  their side. Method: the pair paragraph in `docs/OVERSEER-REFERENCE.md` (locked build tree, outgoing copy kept,
  /proc scan, stage and `mv`).

### SONG-BANK-2-OVERLAY-UNFROZEN-ANCHOR

- state: **LANDED 2026-09-28 at merge `bced52d4`** (tip `468f557b`)  size: `S`  asked by: aeon
- A clip overlay may move an anchor that holds no frozen row (bank 2 is walk-placed); the overlaid map is what
  `validate_placement` checks the placed head against, so a wrong overlay address refuses
  `[map.undeclared-island]`. Record: `docs/superpowers/notes/2026-09-28-overlay-unfrozen-anchor.md`.

### GPP-SEAM-WITNESSES

- state: **open**, not started  size: unmeasured  asked by: aeon, 2026-09-28 (offered, not owed)
- Aeon's `effects_seam_gate --lst` step 3 cannot tell whether the act descriptor's call into
  `games.sonic4.ojz_effects_editor_act1` is reached: the call is byte-neutral, the module is `use`d by four other
  modules, so its witness equates appear in the listing either way. Aeon measured it (descriptor's `use` and call
  removed, ROM crc unchanged, all six witnesses present). A real witness needs something that exists only through
  that call, which means sigil support (for example a listing record of which call sites a comptime function was
  reached from) or a generator change on their side. Evidence, read at aeon `origin/master` (reachable, notice
  commit `20187520`): `docs/DEFERRED_WORK.md` row 2 "GPP-SEAM-WITNESSES", and
  `docs/research/2026-09-26-gate-predicate-audit.md` line 60. Any new listing word is language surface: propose,
  discuss, land.

### SEAM2-TABLE-REGION-UNENFORCED: CLOSED 2026-11-16 (step-0 Task 5 / A2, branch `parcel/step0-soundness`)

- Closed by A2: `MemoryMap::validate_section(name, placed_in, lma, len)` holds a section to (1) the map
  region its placement group names, (2) every `z80_bank` window its first byte lies in, (3) the ROM
  region holding it; `object_bank` is a budget, never a container. `sigil_link::check_regions` is the
  loop `emit_rom` runs, and every seam-2 tail now ends in it (`seam2::check_placed_regions`); the seven
  seam-2 literal sizes are derived (`seam2::bank_room`, the room to the `$8000` window top). Every
  `[[budget]]` row is enforced against its own `ceiling` (`native::check_budgets`), and an unmeasurable
  row is refused. A `[[region]]` row may carry `when` (`map_for_shape`); aeon's `z80_sound_bank` row is
  `when = "sound_on"` (the aeon half of the pair). Under an anchor overlay a `z80_bank` region whose
  base equals an anchor's `at` moves with that anchor (`map_placement::move_bank_regions`): without it
  the clip-shaped overlay build (`anchor_overlay.rs`) was refused, its DAC banks moved into the map's
  fixed `$B8000` window. The measurement test is now
  `crates/sigil-harness/tests/seam2_region_enforced.rs` (the four `gap_*` tests inverted); CLI gate
  `crates/sigil-cli/tests/region_enforcement.rs`. Evidence: the Task 5 section of
  `docs/superpowers/notes/2026-11-16-step0-soundness.md`.
- state (before A2): **measured**, fix not started  size: `S`  found by: aeon's agent, 2026-09-28 (reproduced 2026-09-28, branch `parcel/seam2-region-unenforced`)
- `crates/sigil-harness/src/seam2.rs` (about line 1200, in the sound-tables placement) builds a synthetic map region
  `sound_tables_z80` with `size = 0x400`, yet aeon reports the tables at 1,116 B (after `parcel/song-bank-wfz-ooz`,
  tip `39a2b4dd`) with nothing refusing. So either `place_sections` does not refuse a section larger than its region
  (a linker-level question, the serious reading), or that placement is not on the path that measures the tables.
  Either way the 0x400 is a check that cannot fail. First step: reproduce with a section over 0x400 through that
  function and through `sigil_link::place_sections` directly, then fix the class, not the literal (derive the
  size, or make an overflow refuse loudly). Sequence after PIN-ADVANCE-S2 lands, since that parcel edits seam-2.
- Outcome (measurement, 2026-09-28): reproduced. The real table is 1,116 B at aeon `4523ca5f` (985 B at the
  reference `1ee78b88`) and `seam2::emit_sound_tables_z80` returns Ok. Cause: `place_sections` (in
  `sigil-frontend-emp`, not `sigil_link`) reads no region size; only `emit_rom`'s `validate_section` does, and the
  seam-2 emitters never call `emit_rom`, so all seven seam-2 synthetic region sizes are unenforced. The path is
  product code (`sigil build --native` reaches it through `native::emit_generated_in`), but it is not a silent ROM
  overflow: the tables reach the ROM through the whole-ROM layout, which places every follower from the measured
  length and bounds the bank by the `song_bank_2` anchor and the SFX co-residency ensure. Found on the way, a real
  linker gap: `validate_section` picks the region by LMA, so `emit_rom` accepts a spill that starts at the next
  region's base, and any overflow of a region nested in an earlier covering ROM region. Census: only
  `sound_tables_z80` is over today, so a `place_sections` refusal would break aeon master unless the literal is
  derived from the bank room in the same landing. Record, fix options and population:
  `docs/superpowers/notes/2026-09-28-seam2-region-unenforced.md`; measurement test
  `crates/sigil-harness/tests/seam2_region_unenforced.rs`.

### MT-BANK-RENAME-PAIR

- state: **steps 1 and 2 done** (merged `30793305`, pair installed 2026-09-28T21:37:22Z); step 3 is aeon's  size: `S`  asked by: aeon, 2026-09-28 (they HOLD their rename until we say the pair is ready)
- Aeon will rename `games/sonic4/data/sound/mt_bank.emp`'s declaration from `module games.sonic4.mt_bank_blob` to
  `games.sonic4.mt_bank` (fixes the `[module.path-mismatch]` we reported; zero bytes on their side). Load-bearing on
  ours: `crates/sigil-harness/src/module_roots.rs:151`, the `SHAPE_GATES` row `("games.sonic4.mt_bank_blob",
  ShapeGate::SoundOn)`, matched by exact id in `gate_of`; `None` means every shape places the module, so after the
  rename the bank would be rooted in sound-off shapes, with the installed pair (`1173bb31`) as affected as our tests.
  Mechanism read from code, not run.
- Sequence agreed with aeon: (1) land a tolerant row gating BOTH ids `SoundOn` (byte-neutral at the pin: no module has
  the new id there); prove it red-first by showing a sound-off shape roots the renamed module without the row;
  (2) rebuild the shared pair from that master (method: the pair paragraph in `docs/OVERSEER-REFERENCE.md`) and tell
  aeon; (3) aeon renames and lands. The old id's row is retired at the pin advance that passes their rename, together
  with the `warn_tier_corpus.rs` known-site row for `mt_bank.emp`, which goes red then by design.
- Takes precedence over SEAM2-TABLE-REGION-UNENFORCED's fix because a peer is holding a landing on it.
- Aeon (2026-09-28) holds the rename on branch `fix/mt-bank-module-name` until we say the tolerant row is in the
  installed pair, and its agent is measuring what the CURRENT pair (`1173bb31`) does with the renamed id (demo debug
  and canonical shapes: bytes, refusal or nothing). Asked to commit that result in their tree and send the SHA; read
  it there, not from mail, before step 1.
- Step 1 (2026-09-28, commit `07f9caaa`): `SHAPE_GATES` gates both `games.sonic4.mt_bank_blob` and
  `games.sonic4.mt_bank` `SoundOn`. The mechanism is confirmed by running it: with the new row removed, the pinned
  tree shadowed with the bank's declaration renamed roots section `mt_bank` in Config-B (sound off), and the
  hermetic twin roots `games.sonic4.mt_bank` in a sound-off shape; both tests
  (`module_roots_derivation.rs`, through `derive_module_roots`) go red without the row and green with it. Landing
  run against `.aeon-sigil-ref-1ee78b88`: 521 suites, 5867 passed, 0 failed, 3 ignored, 5865 baseline + 2 new,
  GREEN; `pins_rs_is_current` passed in-suite. Aeon's measurement of the current pair with the renamed id was not
  found committed on `fix/mt-bank-module-name` when step 1 ran.
- Step 2 (2026-09-28): the `30793305` pair is installed (method and figures: the 30793305 paragraph in
  `docs/OVERSEER-REFERENCE.md`). On aeon's rename tip `97d139ca`, the outgoing pair refuses `--config-b` with
  `[sound.fold-head-sound-off]` (`Song_MovingTrucks`, section `mt_bank`); the new pair builds it, and plain, debug
  and `--config-b` are byte-identical renamed and unrenamed (`193507d9/850296`, `ce326232/870560`,
  `d888a312/617813`). Step 3, aeon renames and lands, is theirs; tell them the pair is ready.
- Step 3 LANDED on aeon origin/master at merge `b295559d` (tip `97d139ca`), built with our `30793305` pair (their figures). At the next pin advance past `b295559d`: retire the `games.sonic4.mt_bank_blob` SHAPE_GATES row and the `warn_tier_corpus.rs` known-site row for `mt_bank.emp` (it goes red then by design).
- 2026-11-17 (step 0 Task 6, pin advance to aeon `936439fa`, branch `parcel/step0-soundness`): the `warn_tier_corpus.rs` row went red as designed and is retired (`files: &[]`). The `games.sonic4.mt_bank_blob` SHAPE_GATES row is deliberately KEPT: dead at 936439fa, but the 1ee78b88 reference tree (still the attest tree for entry #207) carries the old id. Retire it once no lane builds a pre-`97d139ca` tree.
- 2026-11-18 (`parcel/pin-advance-99cb963d`): the pin is now aeon `99cb963d` (entry #209). #207 was attested long ago (`a8973000`) and no timer, script or gate names `.aeon-sigil-ref-1ee78b88` (evidence in `.aeon-v2-harness-out/pin99/report.md`), so that tree no longer holds this row back. The row itself is still in place: retiring it edits shipping-closure source and is its own parcel.

## CI-TOOLCHAIN-FLOATS: CLOSED IN BRANCH 2026-11-17 (step-0 fix wave, final review I3; the commit adding `rust-toolchain.toml`)

- Closed by the proxy ruling recorded in the A8 task note (pin `rust-toolchain.toml` to 1.99.1 for CI and
  local use; upgrades in dedicated commits), which the final review found unimplemented (I3: no file, and
  this row still called it the owner's call). Now:
  - `rust-toolchain.toml`: channel `1.99.1`, components `clippy`, profile minimal. CI runs no rustfmt step.
  - `.github/workflows/ci.yml`: the `dtolnay/rust-toolchain@stable` step is gone; a step reads the channel
    from the file, `rustup toolchain install`s it with clippy, and fails unless `rustc --version` is that
    channel. One place to bump.
  - `scripts/landing-run.sh` pre-flight (7b): a host cargo ignores the file (this machine's rust is the
    distro's, no rustup), so the landing refuses BY NAME when `rustc --version` is not the pinned channel.
    Red-first proved by moving the pin; the `landing_verdict` fixture checkout carries the pin and a stub
    `rustc` reporting it.
  - Clippy under the pin: `cargo clippy --workspace --all-targets -- -D warnings`, release and debug, clean.
- An upgrade is its own commit: bump the channel, fix what the new clippy/cargo finds, nothing else.
- The two smaller items below stay open as written.

Original row text follows.

CI installs `dtolnay/rust-toolchain@stable`. Local landings use the host's rustc, which was 1.99.1 on
2026-11-16. Stable moved to 1.100.0 on 2026-11-09, and that version turned the branch's CI red twice
on code that was green here: the new cargo build-dir layout (`build.rs`, fixed `7d0aed5c`) and new
clippy firings (`needless_range_loop`, fixed `417dc819`). The landing's CI-shaped leg reproduces CI's
ENVIRONMENT (no reference tree, `SIGIL_ALLOW_PARTIAL=1`, debug profile). It cannot reproduce CI's
TOOLCHAIN. Two ways to close this, and choosing between them is the owner's call: pin CI to the
toolchain landings use (`rust-toolchain.toml`, which the version banner already watches), or have the
landing install and run CI's stable. Until one lands, the `gh run list` read in `docs/OVERSEER.md` is
the only thing that catches this class.

Two smaller items booked here as well:
- `scripts/check_landing_log.py` reads `CARGO_EXIT=`/`CLIPPY_EXIT=`/`LEDGER_EXIT=` and not `CI_EXIT=`.
  The recorded verdict already fails on a red leg, so a missing `CI_EXIT=` costs one cross-check of
  defence in depth.
- The CI-shaped leg runs inside the suite checkout, and here a real suite root sits beside it. CI has
  none. The suite-root readers (the second skip class in `ci.yml`) can therefore resolve differently
  in the two places.

## Z80-CLOBBER-BUILD-GATE: LANDED IN BRANCH 2026-11-17 (step-0 Task 7 / A5, branch `parcel/step0-soundness`)

- Closed by A5 (`ffb0c18a`): `[call.clobbers-incomplete] (Z80) closure firings` is a located,
  zero-firing row of `run_contract_gate`, computed by `sigil-frontend-emp/src/z80_closure.rs` over every
  `(cpu: z80)` proc in the corpus. Before, the Z80 closure ran only in a reference-dependent test over the
  five resident sound modules, and a Z80 under-claim built green (Sigil audit, reviewer B's e8). The
  computed-dispatch boundary is DECLARED in source, `@allow("clobbers.computed-dispatch", "<reason>")`,
  and excluded by annotation; seam-1's name test `is_opcode_dispatch_proc` is deleted.
- PAIR: aeon `parcel/step0-soundness-pair` `eb634bbc` carries the 27 annotations (the 26 `Seq_Op_*` and
  `Seq_ContinueFetch`). Sigil with this gate REFUSES an aeon tree without them (66 firings at
  `936439fa`), so the aeon commit must reach aeon origin/master before, or with, the sigil merge, and the
  standing reference tree (provenance tip `pin-advance-step0` = `936439fa`) then needs the next pin
  advance to a revision carrying them. Until then a strict run against the plain `936439fa` ref tree is
  red on the Z80 rows by design.
- 2026-11-18: that pin advance landed from `parcel/pin-advance-99cb963d` (entry #209, aeon
  `99cb963d`, the pair merge plus the v2 harness commits; byte-neutral). #209 was attested PASSED on 2026-11-19 from `parcel/walk-measure-in-place`
  (code = master `dbad6235`, 545 suites, 6091/0/3, 29 strict bodies), ahead of that parcel's byte-moving freeze. The standing reference is `.aeon-sigil-ref-99cb963d`.
- Booked: the corpus walk drops 10 `dc` cells in `DacSampleTable` (`bankid`/`winptr` equs of link
  symbols). `split_z80_drops` counts them apart (a `dc` is not decoded as code by the closure, dropped or
  not); a define set that resolves them would let the Z80 drop count be an exact 0.
- Booked, OPEN (review I3, fix round 1 chose (b)): a Z80 COMPUTED transfer charges nothing. `ex (sp),hl;
  ret` and `jp (hl)` have no symbolic target, so `z80_proc_node` gives them no edge and no indirect site.
  Sites at aeon `eb634bbc`: `Sequencer_NextOpcode` (`engine/sound/sound_sequencer.emp:1180`, the
  `SeqOpcodeTable` trampoline) and `Z80_IdleProgram` (`engine/system/z80_init.emp:139`, `jp (hl)`, no
  `clobbers`, so unchecked anyway). The dispatcher, and `Sequencer_Channel` through it, is never charged
  what the 26 handlers write; its declared clobbers there are a convention. Why not (a) now: the table is
  named only in a COMMENT (`// [dispatch.trampoline: SeqOpcodeTable …]`), and recognising `ex (sp),hl; ret`
  as a dispatch (versus its other uses) from code is a heuristic. The sound close: a declared dispatch
  bound on the transfer (the Z80 twin of the 68k `jsr (aN) as Type`), naming the table, so the site is
  charged the union of the table cells' effective sets, and an unbounded Z80 computed transfer reads ⊤
  as on the 68k side (with a Z80 `clobbers.unanalyzable` hatch). That is a grammar + aeon pair parcel.
- Bound on the hatch (fix round 1): `[clobbers.computed-dispatch-called]` refuses an annotated proc that an
  unannotated proc `call`s/`rst`s. A direct jump is allowed: the corpus has one (`Sequencer_NextOpcode`
  `jp z, Seq_Op_Ext`), and the closure charges the jumper the target's full effect.
- Evidence: the Task 7 section of `docs/superpowers/notes/2026-11-16-step0-soundness.md`.

## SOUND-BANKID-ODD-RESIDENT-BLOB: BOOKED 2026-11-17 (found by step-0 Task 7 / A5, e8 run)

- state: **measured**, not started  size: `S?`
- Appending 9 bytes of Z80 code (two procs, `ld b,0 / ld a,0 / ret` and `call / ret`) to section
  `sound_psg` of aeon `936439fa` (+ the A5 pair files) makes `sigil build` refuse the plain shape on BOTH
  the shipping `94cf811d` and the A5 candidate (also with `SIGIL_CONTRACTS=0`), after a fresh
  `emit_sound_blob`: ``error: native build (sonic4 plain): [sound.bank-id-vs-placement] cannot measure:
  the placed resident blob spans 6238 bytes but the emitted blob is 6237, so its bank-id offsets do not
  locate bytes in the ROM``. One more byte (a `nop`) and both build (`crc=017def01 len=850282`). So an
  odd-sized resident blob is placed padded to even and the check compares the padded span with the
  unpadded emission: a padding/measurement mismatch, not a real overlap. An otherwise valid sound edit is
  refused. Evidence: `.aeon-v2-harness-out/sigil-t7/e8.txt` (odd, both refuse), `e8b.txt` (even, builds).

## EMP-ONE-NAME-ENV: LANDED IN BRANCH 2026-11-17 (step-0 Task 8 / A4, branch `parcel/step0-soundness`)

- Closed by A4 (`a312c74a`): an imported comptime item resolves its free names where it is DEFINED.
  `resolve::hygiene` (`free_names`, `rename_free`, `hidden_name`, `HygieneMemo`, `hygienic_closure`,
  `localize`); `collect_pub_comptime` injects the closure, each dependency under `$def$name`, and each
  importer maps back the dependencies it binds to the same item, so an importer whose names already agreed
  sees the pre-A4 clones exactly. A name bound only by the importer is refused,
  `[name.unbound-at-definition]`. `sigil build --report name-env` lists the rows (EMP language audit
  finding 1/2; aeon EMP_PITFALLS §2 and §8 retired in the pair).
- Fix `5fea0ad2`: prelude items map back in each importer; hidden structs are not bare-window candidates.
- Fix round 1 `31367919` (task-8 review I1-I4, M2, M6): an `equ` binds as a VALUE like a const (a private
  equ dependency is injected as a hidden const, so no link symbol; an importer equ trips the refusal); enum
  variants are not in scope in their own discriminants/payloads; an overlay's dotted window struct is a walked
  name, and a bare window over an imported struct is qualified at the definition; fixtures pin the hidden-struct
  window filter and map-back. Corpus: BN 7/7, name-env 0 differ / 0 unbound in all 7 shapes. Pair doc `6269c7fd`.
- PAIR: aeon `parcel/step0-soundness-pair` `6b5f8d8d` (+ `3fd291f3`) imports `PalCycleScript1/2` in `palette_dsl.emp`
  (the 2 distinct unbound rows the report found at `936439fa`); byte-neutral (BN base column). Without it
  the candidate refuses the tree. Docs `c4c1fb0b` (pitfalls retired, `EMP-A4-PINS-REMOVABLE` booked aeon-side).
- Booked, OPEN (not in A4's scope):
  - The contract gate's corpus type environment is FLAT and last-writer-wins
    (`corpus_contracts.rs:489-497`, `collect_env` over every file; Sigil audit finding 12): a same-named
    declaration in two modules resolves to whichever file came last, for every file's walk.
  - The native build's hand-kept 14-module `COMPTIME_HELPERS` list (`sigil-harness/src/native.rs:1817`),
    publicized and glob-injected into every module. With A4 a helper's private items travel with the
    items that need them, so the publicize step and the list are removable; aeon must first drop the
    importer-side workarounds (aeon `EMP-A4-PINS-REMOVABLE`).
  - Ambient paths outside `collect_pub_comptime` are not hygienic: `layout_struct_ambient` (the struct-offset
    harvest), `eval_expr_in_file_ambient` (contract bind), the corpus walk above, and the seam-1/seam-2
    sound lowering. Each builds its own environment.
  - A `{name}` interpolation inside an `ensure`/`raise` message string is evaluated where the message is
    evaluated; strings are not renamed.
  - Labels (procs, data, region fields) named inside an injected comptime fn's `asm` body resolve
    through the link rename, not this rule. (An `equ` binds like a const since fix round 1.)
  - `--report name-env` lists only the TOP injected items' free names, not their hidden dependencies', and only
    names whose definition binds a comptime item or that only the importer binds (review M1). The build's
    refusal checks every injected dependency, so the report's `u` can under-count; the green build is the
    stronger check.
  - The walker walks a comptime fn's return type and parameter types/defaults before scoping any parameter
    (review M5): a dependent type or default naming an earlier parameter is treated as free. Unconfirmed
    against the evaluator's binding order; no corpus case.
- Evidence: the Task 8 section of `docs/superpowers/notes/2026-11-16-step0-soundness.md`.

## FRONTEND-DIFFERENTIAL-GATE: LANDED IN BRANCH 2026-11-17 (step-0 Task 9 / A6, branch `parcel/step0-soundness`)

- Closed by A6 (`6fdcfbee`, `9fe5b744`, `94802e92`): the `.emp` and AS front ends are compared on a generated matrix,
  `crates/sigil-cli/tests/frontend_differential.rs` over `tests/support/frontend_diff.rs`. Every
  `ALL_FAMILY_NAMES` family with an operand (78 of 85) x sizes {none, .b, .w, .l, .s} x sources (each EA
  class, its field's boundary values, `#imm` {lo-1, lo, -1, 0, 1, hi, hi+1}) x destinations (each EA class,
  none). n = 155,292, computed from the tables. SameBytes 4,978 / BothRefuse 149,345 / excluded 969 /
  Disagree 0. Z80 `ld r,n`, `ld rr,nn`, `cp n`: 98 cases, 0 Disagree. Every out-of-range sized immediate
  (6,084) is refused by both. Ten named EXCLUSIONS rows, each a narrow predicate, each used.
- Coverage (`crates/sigil-harness/tests/frontend_differential_coverage.rs`, reference-dependent): the 481
  `(family, size, EA classes)` triples the 7 shipped shapes encode (102,518 encodes) are all generated.
- RF: Task 3's checks disabled -> `move.b #256, d6`: emp `1C3C 0000`, AS `operand 256 out of range
  -128..=255` (Sigil finding 2), 627 Disagree; `Ea::Disp` dropped -> 82 missing `(d16,An)` triples.
- Finding fixed in branch (`85dd7a05`, A1 class, byte-neutral): `.emp` truncated `andi/ori #imm, ccr` at
  `.w/.l/.s` (`andi.w #65535, ccr` -> `023C 00FF`); now refused with the AS text.
- Booked below: `EMP-USP-READ-AS-SYMBOL` (closed in fix round 1), `AS-MOVE-B-NEG-TO-CCR`, `EMP-FRONTEND-DIFF-GAPS`.
- Fix round 1 (`f456fbc3`, `93c207fe`; review I1, M1-M4): `usp` reserved; four rows family-scoped; a measurement
  floor (SameBytes > 0 for every family except the 16 derived as having no `.emp` form) and 5 canaries; the
  coverage test also requires a SameBytes case per emitted triple (480/481 at size, `swap` unsized). Open
  from the review: M5 (no committed asl byte oracle for the excluded population).
- Fix round 2 (`a8dcdbd7`, `a6473695`; re-review N1, N2, N4): a spliced label/string naming a register word
  never becomes a symbol (see `EMP-USP-READ-AS-SYMBOL`); the three other usp refusals have tests; each
  canary must be a generated case. Left: N3 (the coverage test's unsized fallback is not bounded to
  implicit-size families; today it holds only `(swap, .w, [Dn])`).
- Evidence: the Task 9 section of `docs/superpowers/notes/2026-11-16-step0-soundness.md`.

## EMP-USP-READ-AS-SYMBOL: CLOSED IN BRANCH 2026-11-17 (A6 fix rounds 1-2, `f456fbc3` + `93c207fe` + `a8dcdbd7`)

- Fix round 2 (re-review N1): the comptime splice was a second route. `classify_operand_splice` mapped a
  `Label`/string straight to a symbol, so `put usp` through `asm { move.l a2, {p} }`, or `const N = "usp"`
  spliced as `{N}`, assembled `21CA 1234` beside an outside `usp` (asl `4E62`); `sr`/`ccr` the same, and a
  `use`-imported `usp` reached an operand this way. Now a spliced label/string naming a register word is
  refused (`usp`: `[operand.usp-unsupported]`; `d0`-`a7`/`sp`/`sr`/`ccr`: `[asm.splice-register-word]`;
  Z80 words in a Z80 section), and a `Label` argument naming one is refused at the call like `put d0`.
  Fixture `crates/sigil-frontend-emp/tests/register_word_splice.rs`.

- Closed: `usp` is reserved in `.emp`. `[operand.usp-unsupported]` refuses it in every operand position
  (never a symbol); `[name.reserved-register]` refuses an item, vars/region field or parameter named `usp`
  at its declaration. Refusal only; BN 7/7. The differential's frames now link against a defined `usp`
  symbol (`link_env`), so a regression is a gate failure (`21CA 1234` vs `4E62`), not an exclusion hit.
  Fixture `crates/sigil-frontend-emp/tests/usp_reserved.rs`. Exposing `move usp` itself stays
  `EMP-USP-CCR-SURFACE`. Original finding text follows.

- `.emp` has no `usp` register word, so `move.l a2, usp` reads `usp` as a SYMBOL. Standalone the link
  refuses it unresolved, but beside any item named `usp` (probe: `proc usp() { rts }`) it assembles
  `21CA 0000`, `move.l a2, ($0000).w`. asl and the AS route assemble `4E62`. A silent wrong instruction,
  the finding-2 direction, gated today only by nobody naming a symbol `usp`. aeon has no such name and
  no `usp` word in any `.emp` file (grep), and the s4 plain/debug encode probe has no `usp` operand.
- Not fixed in A6: not A1 class. Smallest fix: make `usp` a reserved register word in operand position
  and refuse it until `EMP-USP-CCR-SURFACE` exposes the form. The differential's `emp-lacks-usp` row
  then stops matching and must be removed with the fix (`every_exclusion_is_used` forces it).

## AS-MOVE-B-NEG-TO-CCR: OPEN (found by A6's asl grounding, 2026-11-17)

- The AS route encodes `move.b #-128, ccr` as `44FC FF80` and `move.b #-1, ccr` as `44FC FFFF`; asl 1.42
  (Bld 212) emits `44FC 0080` and `44FC 00FF` (it zero-extends the byte immediate into the word field).
  `move.w`/unsized agree with asl (`44FC FFFF`). The 68000 reads only the low byte, so the CPU effect is
  the same, but the AS route's contract is asl's bytes. No aeon site (0 `move… ,ccr` in the tree).
- Outside A6's gate (`.emp` refuses `move <ea>,ccr`, so the pair never compares); found by re-running
  every A6 disagreement through real asl.

## EMP-FRONTEND-DIFF-GAPS: OPEN (booked by A6, 2026-11-17)

`.emp` refuses forms asl and the AS route assemble. None emits a wrong byte (each is a refusal, or bytes
proven equal to the AS route's legal spelling); each is one named EXCLUSIONS row in
`crates/sigil-cli/tests/support/frontend_diff.rs`, so closing a gap fails `every_exclusion_is_used`
until its row is removed.

- `emp-requires-a-size-suffix` (375): no default size for shifts/rotates on memory, `move`/`andi`/`ori`/
  `eori` to sr/ccr, `bchg`, `exg`, `addx`. `btst`/`bset`/`bclr` default to `.b` but `bchg` does not
  (`m68k_default_size`), an inconsistency.
- `emp-lacks-mnemonic` (161): `abcd`, `sbcd`, `nbcd`, `negx`, `subx`, `chk`, `stop`, `link`, `unlk`.
- `emp-lacks-sr-ccr-form` (70): `move <ea>,ccr`, `eori #,ccr`, `andi`/`ori`/`eori #,sr` (see `EMP-USP-CCR-SURFACE`).
- `emp-no-implicit-variant` (119): `add`/`sub <ea>,An` (asl: `adda`/`suba`), `eor #imm,Dn` (asl: `eori`).
- `emp-label-operand-gap` (45): a label as a straight-line operand, beside another extension word, or as a
  `movem` pc-relative source. The last refuses with `internal: a PC-relative operand reached the generic
  operand mapper`, an internal message reaching the author; it should be a located refusal.
- `emp-lacks-usp` (4): `.emp` refuses `move usp` (`[operand.usp-unsupported]`); exposing it is `EMP-USP-CCR-SURFACE`.
- `branch-b-suffix` (3): `bra.b` (asl: `.s`).
- `unsized-branch-aeon-pin` (3): the AS route refuses an unsized branch by Aeon policy; asl relaxes it and
  `.emp` assembles the short form. A policy split between the two routes, not a defect of either.
- `sized-swap` (3): `.emp` accepts `swap.b/.w/.s` (asl: `invalid operand size`); bytes equal `swap`.
  Refusing it is a spelling-parity fix; aeon writes no sized `swap` (grep), so it would be byte-neutral.

## EMP-REGISTER-NAMES-CASE-SENSITIVE: OPEN, LATENT (booked by A6 fix round 2, 2026-11-17)

- `.emp` knows register words in lowercase only (`Reg::from_name`, `value.rs`), while the AS route and asl
  are case-insensitive. So an upper- or mixed-case name shadows a register: with an outside `USP` symbol,
  `move.l a2, USP` assembles `21CA 1234` (AS/asl `4E62`); `data SR: u16` + `move.w SR, d0` gives
  `3038 0000` (AS `40C0`). `.emp` may declare `data USP`, `D0`, `SR`, `CCR`; `[name.reserved-register]`
  and the splice refusal are lowercase-exact. The differential spells only lowercase, so it cannot see this.
- Latent: aeon declares no item (and writes no operand) named a register word in another case. Grep of
  every `.emp` file, comments stripped, for `[dDaA][0-7]|[sS][pPrR]|[cC][cC][rR]|[uU][sS][pP]|[pP][cC]` as
  a whole word other than the lowercase spelling: **0** at the reference tree `936439fa` (208 files) and
  **0** at the live tree `d9b51660` (the only `SR` hits are in comments, a string, and the substring of
  `PageIn_Saved_SR`).
- Needs a language ruling: refuse case variants of register words as names and operands, or fold them.


## EMP-SPLICED-DISP-LINKS-PARAM-NAME: CLOSED IN BRANCH 2026-11-17 (final review I2 + re-review N1, fix wave, `parcel/step0-soundness`)

- All EIGHT forms, each now its spliced value: `({x}).w`, `({x}).l`, `{x}(pc)`, `{x}(pc,Xn)`, `jmp {x}(pc)`
  (I2), and the wrapped displacement spellings `({x})(pc)`, `({x})(pc,Xn)`, `jmp ({x})(pc)` (final
  re-review N1: `parser.rs`'s wrapped-displacement branch hard-coded `disp_spliced: false`, so
  `lea ({t})(pc),a0` still gave `41FA FFFA` at a decoy `t` instead of `41FA FFFC`; it now passes the inner
  group's `first_spliced` through). The fixture lists all eight.

- Finding (final whole-branch review I2; first noted as a Task 9 out-of-scope line with no row): in five
  forms a spliced `Label` was ignored and the parameter's NAME was linked: `({x}).w`, `({x}).l`, `{x}(pc)`,
  `{x}(pc,Xn)` and `jmp {x}(pc)`. With `t: Label = Target` and a proc named `t`, `lea {t}(pc),a0` gave
  `41FA 0008` (aimed at `t`) and `move.l ({t}).l,d0` gave `2039 0000 0008`: wrong bytes, green build,
  the `usp`/register-word direction. Without a same-named symbol it was a confusing `unresolved symbol x`.
  `#{x}` and bare `{x}` were correct.
- Cause: `map_pcrel_operand` and `map_pinned_abs` (`eval/asm.rs`) resolve a bare path as a symbol name and
  never read the parser's `disp_spliced`; `({x})` lost its braces in `paren_operand`, so `({x}).w` and
  `(x).w` were the same AST.
- Closed: `Operand::Ind` carries `first_spliced`; both mappers evaluate a spliced target and use its
  value, exactly as the unspliced spelling of that value assembles. A label/string goes through the same
  register-word refusal `jsr {p}` uses (`[asm.splice-register-word]`, `usp` keeps
  `[operand.usp-unsupported]`); a non-symbol pc-relative target is `[asm.splice-kind]`; an integer in a
  sized absolute is the address, as `($1234).w` is. Reachability: 0 aeon sites (the only match is a comment,
  `games/sonic4/player/player_sensors.emp:215`), so byte-neutral; BN 7/7.
- Fixture `crates/sigil-frontend-emp/tests/spliced_disp_target.rs`: each form's spliced bytes equal its
  unspliced spelling's (the oracle, pinned for two forms), register words refused in all five, integers.

## MAP-BANK-WINDOW-GATE-DISAGREES: CLOSED IN BRANCH 2026-11-17 (step-0 fix wave; Task 5 residual, final review M2)

- Residual from Task 5 fix round 1 (ruling "accept: a window matching only gated-off anchors is dropped for
  that shape"): `bank_windows_for_shape` silently DROPPED a `z80_bank` region whose base was the `at` only
  of anchors not in force in the shape. Its stated reason: the literal rule refused `config_b` on an
  unpaired aeon tree (`936439fa`, no region `when`), which "would break sigil on aeon master until the pair
  lands". That reason no longer holds: wave 3 (A5's Z80 clobber gate, A4's one name environment) already
  refuses every aeon tree without the pair, so the concession kept nothing buildable and only hid a map
  whose region and anchor gates disagree (residual (c) in the Task 5 re-review).
- Closed: that case is refused, `[map.bank-window-gate-disagrees] z80_bank region R at B is in force in
  this shape, but its anchor A is not ... Give the region its anchor's when`. On the paired map the region
  carries `when = "sound_on"`, so `map_for_shape` removes it from sound-off shapes before this check and it
  never fires: refusal only, BN 7/7, AG green. Red-first by restoring the drop.
- Tests (`map_placement.rs`): `a_window_in_force_where_its_anchor_is_not_is_refused` (ungated region,
  sound-off); `todays_sonic4_bank_window_is_accepted_in_every_shape` now covers the paired map in both
  shapes and the ungated region where its anchor is in force.
- Still open from the same Task 5 minor list: no unit test for residual (a) on sound-off; overlay +
  sound-off + unpaired false refusal (fails closed, and unpaired trees are refused anyway now).

## SEAM1-STUBS-CREDIT-HATCHED-PROCS: OPEN, LATENT (booked by the step-0 fix wave from Task 7 re-review, 2026-11-17)

- `crates/sigil-harness/src/seam1.rs` `collect_pub_proc_stubs` credits every `pub proc` with "preserves
  everything not clobbered", citing `[call.clobbers-incomplete]` as what makes that sound. That includes
  procs carrying `@allow("clobbers.computed-dispatch", ...)`, whose `clobbers(...)` is exactly what that
  rule does NOT verify. A harness consumer module that called or tail-jumped into such a pub proc would be
  credited from an unverified contract. In the build gate, `[clobbers.computed-dispatch-called]` blocks
  the call case; nothing blocks the tail-jump (`jp`) case in the harness stubs.
- Latent: no such cross-module edge exists in the corpus today (Task 7 re-review, measured).
- Fix, small: skip `computed_dispatch` procs when building the stubs (a consumer then sees no contract and
  is charged ⊤ for the edge), or say in the stub doc that the hatch's procs are credited unverified. Pair
  with Task 7 N1 (no test that a jumper into an annotated proc is charged).

## FRONTEND-DIFF-Z80-COVERAGE: OPEN (booked by the step-0 fix wave from final review M3, 2026-11-17)

- A6's differential (`crates/sigil-cli/tests/frontend_differential.rs`) compares 98 Z80 cases, `ld r,n` /
  `ld rr,nn` / `cp n` only (`support/frontend_diff.rs` `cases_z80`), and its coverage test
  (`crates/sigil-harness/tests/frontend_differential_coverage.rs`) asserts that every 68k
  family/size/EA the aeon build emits is generated. Nothing asserts the same for Z80. That matches the
  plan's A6 scope, but A5 now gates Z80 code, so a Z80 form the build emits and the differential never
  compares is a front-end disagreement nothing would see.
- No silent path is known: the final review's Z80 sweep (imm8 in `ld/add/and/sub/ld (hl)/ld (ix+d),n`,
  `(ix+d)` displacement, port, `bit/set`, `im`, `rst`, imm16, `(nn)`, `jp/call nn`, link `ld hl,Far`)
  found every out-of-range form refused loudly.
- Work: a Z80 counterpart of the coverage test (the Z80 forms the aeon sound driver emits, by
  mnemonic/operand shape, each generated), then widen `cases_z80` to cover it.

## G0A-VENDORED-CONTRACT-LANDING-LEG

- state: **OPEN** 2026-11-18  size: `S`  project: `G0A-BUILD-EXPORT`
- Found in G0a Task 2. `scripts/check-vendored-contract.sh` (needs `EMPYREAN_DIR`; unset is exit 2, never a skip)
  compares `contract/build/SOURCE` and the vendored files against empyrean. It runs standalone and each G0a task's
  landing bar runs it by hand. It is NOT yet a leg inside `scripts/landing-run.sh`: that script's verdict machinery
  (`LEDGER_EXIT`, `CI_EXIT` spans, `landing_verdict.rs`, `check_landing_log.py`) is deep enough that a new leg is its
  own change, with its own red-first proof. Until it is a leg, a landing verdict does not mention a stale vendor.
- The CI half already runs everywhere: `sigil-link/tests/vendored_contract.rs::vendored_schema_matches_source`.
- `SOURCE` pins an empyrean commit on `feat/g0a-build-contract` (re-vendored 2026-11-18 at `484fb8a1` for the final review fix wave: fixed `z80-banked` vector, new `vectors/reader-red/` and `vectors/lookup/`, runner B4/B5), not yet on empyrean `main`. Re-vendor (and the
  `empyrean-commit:` line) when that merges, if the merge changes any pinned file.

## G0A-EXPORT-FILES-GAPS

- state: **OPEN** 2026-11-18  size: `S`  project: `G0A-BUILD-EXPORT`
- Found in G0a Task 3, narrowed by the controller's ruling the same day. The sidecar's `files` table holds SOURCE
  rows only (`lang` is the closed enum `emp|asm|z80`). Art and sound blobs and `map.toml` are not rows. They ARE
  hashed into `inputsSha256` (every read under a root, plus aeon `art/` under the hash-only root `aeon`), and the
  `convsym` executable is hashed by file name into the tool part. What is still true: a changed blob is visible
  only through `inputsSha256` and the ROM hash, never as a stale FILE row, so a consumer cannot say WHICH blob changed.

## G0A-SCAN-READS-TOOL-FIXTURES: CLOSED

- state: **CLOSED** 2026-11-21 (G0b Task 13a)  size: `S`  project: `G0A-BUILD-EXPORT`
- **Closed by deletion.** `UNROOTED_AEON_DIRS` and its test `tools_fixtures_is_the_one_named_hole` are gone from
  `sigil-harness/src/export_files.rs`, together with the `tools/` special case. The export now places every read by
  the build's own roots (`G0B-EXPORT-BY-ROOTS`), and a `--project` build's scan never reads `tools/fixtures/` (G0b
  Task 9). Should a source file under no root ever be read again, it is refused naming the file and each root with
  its directory (`a_source_read_outside_every_root_is_an_error_naming_the_path_and_the_roots` holds
  `tools/fixtures/anchor_sweep/f.emp` among its cases). The original text follows.
- 2026-11-20 (G0b Task 9): a `--project` build no longer reads `tools/fixtures/` (its scan is the engine and game
  roots). The constant stays while the legacy driver walks the tree; it goes at Task 11 (see
  G0A-SCAN-READS-OTHER-GAME's residue).
- Found in G0a Task 3. Four `.emp` fixtures under the aeon `tools/fixtures/anchor_sweep/` are read by every build
  only because the module scan reads every `*.emp` in the tree. They are not part of the program and fit none of
  the four export roots. Ruling: a scan-scope defect, not a missing root. `UNROOTED_AEON_DIRS = ["tools/fixtures"]`
  (`sigil-harness/src/export_files.rs`) leaves reads under exactly that directory out of the export; any other read
  under `tools/` is an error naming the path. The hole is DELETED when G0b scopes the module scan to the engine and
  game roots; then the constant and its test go with it.

## G0A-SCAN-READS-OTHER-GAME: CLOSED (game-as-data driver), residue on the legacy driver

- state: **CLOSED** 2026-11-20 (G0b Task 9) for the game-as-data driver  size: `M`  project: `G0A-BUILD-EXPORT`
- Closed by `Project::scan` (`Manifest::scan_roots` over the engine root and the game's sources root, each module
  tagged with its root), used by every scan of a `--project` build: the required-constant check, the RAM harvest,
  the program lowering and the contract gate. The sound seams' import check resolves among the same roots
  (`import_check::seam_roots`). Witnesses, each red-first: `sigil-build/tests/two_roots.rs`
  `engine_root_alone_does_not_pull_other_games` (games/demo's scan holds no sonic4 module, by id or path, and
  every module is tagged inside its root; control: the whole checkout holds them) and `project_contract_gate.rs`
  `another_games_broken_module_is_not_scanned` (a module in the other game that breaks any whole-tree scan does
  not reach the build). Measured: demo's `--project` listing DIGEST-SCAN 208 -> 80 files; its export carries no
  `game/sonic4/...` row (the comparison job's declared delta drops exactly those, byte-exact on 10 shapes).
- RESIDUE, the legacy driver (`--legacy`, deleted at Task 11): it still walks the whole tree, so its demo export
  still lists sonic4's sources. The two gate-8 assertions below and `UNROOTED_AEON_DIRS`
  (G0A-SCAN-READS-TOOL-FIXTURES) are pinned to legacy-driver exports, so they land with Task 11's cutover, when
  every build is a `--project` build.
- The original text follows.
- Found by the G0a final review (A-I2); controller ruling: BOOK, do not fix in G0a (narrowing the scan changes the
  front end beyond G0a's scope). The module scan reads every `*.emp` in the tree, so each game's export lists the
  OTHER game's sources in `files` and hashes them into `inputsSha256` (`sigil-harness/src/export_files.rs` says so in
  its module doc). Measured on the 3f211b78 sidecars: demo plain carries **124 `game/sonic4/...` rows of 206**
  (only 44 of the 206 are named by any line row); s4 plain carries **7 `game/demo/...` rows**. Consequences until G0b:
  - `inputsSha256` and the `files` table of demo move on every sonic4 source edit although the demo ROM does not.
  - Consumers must read `files` as the build's read set, not as the program (booked for the Aurora hand-off in
    empyrean `docs/v2/specs/reviews/2026-11-18-G0a-final-review-deferred.md`).
  - Gate 8's equate attribution (`build_export_gates.rs::equate_origins`) treats a name declared in any non-sonic4
    file of the read set as not sonic4-only. Because demo reads every sonic4 file, a sonic4 equate leaked into a demo
    export passes when an engine or demo file declares the same spelling. The hole is named, not closed.
- **Retargeted here (ledger T10 FINAL-REVIEW minor):** "gate 8 asserts the demo export's `files` has no game-root row
  under `sonic4/`" is red on every demo build today (124 rows), so it is done AFTER G0b narrows the scan, together with
  narrowing gate 8's `declared` map to files a line row or symbol module references.
- Do (at G0b): scope the scan to the program's import closure, then add both assertions; delete
  `G0A-SCAN-READS-TOOL-FIXTURES`'s constant in the same change.

## G0A-FINAL-REVIEW-DEFERRED

- state: **OPEN** 2026-11-18  size: `M` (each item `S`)  project: `G0A-BUILD-EXPORT`
- The G0a final review (reviewer A, sigil) rated these OK-TO-DEFER. The fix wave took A-I1 (duplicate bridge
  symbols refused), A-I2 (booked above), A-M1 (export rendered before install) and A-M10 (heading); the rest, each
  named so it can be picked up alone:
  - **M2 `--no-build-json` is not a full escape hatch.** `native.rs` `export_symbols`, `embed_site`, the ROM-bytes
    check and `z80_bridge` run on every chained build; only the layout pass is gated. Gate them on `LayoutPass != Off`
    (or a sibling flag), or say in the usage note that a side-table disagreement fails every build.
  - **M3 `inputs.rs` doc says a reader can reproduce `inputsSha256` from the export alone**; since the T3 ruling it
    cannot (blobs, `map.toml`, `aeon`/`external` hash-only reads, `toolexe` are not in the sidecar). Reword to "from
    the build's read set"; the `InputRead.root` doc omits `external`; note the line format's escaping (the shape is
    last on its line because `sonic4 plain` holds a space).
  - **M4 duplicated gate helpers:** `SHAPES` ×4 plus the cross-check's `shapes()`, `source_path` ×2 (a hand inverse of
    `export_files::place_path`), `declared_name`/`declared_equate` ×3, and `schema_verdict` still in
    `build_export_binding.rs`. Move into `sigil_harness::test_support`; export the inverse of `place_path`.
  - **M5 three cursor replays beside `Section::image_bytes`** (`lines.rs::owners`, `placed.rs::extent`,
    `export_z80.rs::embed_site`); only `owners` asserts against `image_len`. One fragment-offset iterator in sigil-ir,
    or the same `debug_assert` in the other two.
  - **M6 two structs named `ExportInputs`** (`sigil_link::export` and `sigil_harness::export_files`); rename the
    harness one (`ReadSetExport`).
  - **M7 the writer does not enforce the bank rule the readers refuse.** `native.rs::z80_windows` checks only that a
    VMA exists; refuse `lma & $7FFF != vma & $7FFF` and a window larger than `$8000` there. The contract now states
    the rule and both readers refuse it (empyrean `vectors/reader-red/`), so today a bad region fails at the consumer.
    Do it with the T9 `section_space` first-byte minor below.
  - **M8 schema values never emitted:** `kind: "data"` and `lang: "z80"`. Emit `data` for `.emp` data items, or keep
    the note in the hand-off (booked in empyrean's deferred page).
  - **M9 three spellings of one shape** (`shape.shape`, gate names, the spec's `"s4"`); for the T12 hand-off.
  - **M11 values unbounded for JavaScript:** refuse `|value| > 2^53 - 1` in `emp_consts` and `prepare_export`, the
    `ConstOutOfRange` shape.
  - **T9 `section_space` tests only a section's first byte** against the window; require `[lma, lma+image_len)` inside
    it (with M7).
  - **T8 doc comment on the wrong item:** `export_side.rs` `resolve_importer`'s doc sits above `declares_exportable`
    (the sibling in `main.rs`, `sidecar_path` over `writes_sidecar`, was fixed in the fix wave).
  - **Ledger items A rated OK-TO-DEFER and leaves as they are:** T1 unit test serialises no non-empty
    files/layouts/consts/overlays; T3 hash-only label breadth, define-not-fitting-i64 untested, repeated `(crc,size)`
    in conflicts, `--no-build-json` without `-o` accepted, sigil-root rows skip the on-disk check; T4 `outermost`/
    `label` walk duplicated, `export_locator` assert accepts either fragment; T5 two plain rebuilds (~30 s),
    same-named-section merge; T5 pub dispatch/vars/table/region not tested directly; T9 no red-first proof for the
    `z80_windows` no-vma refusal, embed-site 0/multi, ROM-bytes mismatch, `emit_sound_blob` lost its
    `handler_symbols` check (still run through `seam2::native_sound_symbols`), the gate reads raw `map.toml`
    windows; T10 `declared_equate` misses `set`, block comments, untaken `if`, proc-body `=` (fold into the
    G0A-SCAN-READS-OTHER-GAME narrowing).

## G0A-SOUND-BANK-LABEL-SPACE: CLOSED

- state: **CLOSED** 2026-11-18 (G0a Task 9)  size: `S`  project: `G0A-BUILD-EXPORT`
- Closed by ONE space rule, `sigil_link::export::section_space` (`placed.rs`), read by both the symbol table
  (`PlacedSymbols::new`) and the line walk (`walk_lines`): a Z80-CPU section is `z80`, and so is a section of any CPU
  that is PHASED INTO A Z80 BANK WINDOW the shape's memory map declares (`Z80Window`, from each `z80_bank` region in
  force after `bank_windows_for_shape`: its first byte in the window's ROM range, its VMA origin the address the
  window maps that byte to). Derived from the map, not from a name list. Everything else is `m68k`. A `z80_bank`
  region with no `vma` that holds a phased section is refused (`native::z80_windows`). On s4 (plain and debug) the
  one section it moves is `soundbankhead` (`SoundTablesZ80_Head` and the 5 other `PHASE` labels of the listing,
  VMA `$8000..`, LMA `$B8000..`, bank `$17`): 6 symbols and 5 rows, now `space: z80` with `lma`. The bank's
  UNPHASED data (Moving Trucks, the SFX block, at VMA = ROM address) stays `m68k`, correctly: its VMA is a ROM
  address. Gate `build_export_z80::banked_rows_carry_lma` holds every listing `PHASE` label in a map window to a z80
  symbol at that LMA; gate 5 still holds (the listing has no space).
- Originally found in G0a Task 5. The export's `space` for a label was its section's CPU, so the sound-on shapes'
  68000-CPU data sections phased into the Z80 bank window (`games.sonic4.soundbankhead`: `SoundTablesZ80_Head` VMA
  `$8000` LMA `$B8000`) were `space: m68k`, where `$8000` is ROM, not this table.

## G0A-EMP-DIAG-SPLICE-TRAIL

- state: **OPEN** 2026-11-18  size: `S`  project: `G0A-BUILD-EXPORT`
- Found in G0a Task 7. `.emp` template splices now record `Frame::Template` expansions (one per splice run, the
  spliced items' spans moved to it), so the build export's line rows name the splicing line. The DIAGNOSTICS surface
  was held unchanged on purpose: the whole-program driver maps every diagnostic's span back to the line as written
  (`SourceMap::physical`, `resolve::build_program_with`), and `SourceIndex::locate` / `ChainSources::locate` render an
  `.emp` expansion where it was written. So an error on a spliced line still names the template body only; the call
  site reaches it only through the eval-time `[prov.comptime]` note, which a lowering-time error (an encoder refusal,
  a proc lint over spliced items) does not get. The chain to render "body line, spliced at call line" now exists;
  rendering it is a diagnostics-surface change for its own ruling (it would also make two splices' identical errors
  distinct, which the dedup keys then have to agree with).

## G0A-EXPORT-LAYOUT-COVERAGE

- state: **OPEN** 2026-11-18  size: `S`  project: `G0A-BUILD-EXPORT`
- Found in G0a Task 8, revised by fix round 1. (1) The aeon tree declares only `comptime enum`s and no `bitfield`.
  A comptime enum carries no discriminants by design (the mapper owns them), so it has no data layout and is not
  exported; the plain-`enum` and `bitfield` layout kinds, and enum-valued constants, are therefore exercised by the
  front-end fixture (`sigil-frontend-emp/tests/export_layouts.rs`) only. That is the tree's shape, not a coverage hole
  to close. (2) Layouts, overlays and constants are exported for the modules a shape LOWERS (the `use` closure): sonic4
  plain has 62 structs and 3 overlays, debug 63 and 10. (3) COST, closed in fix round 1: the registry pass is a
  `LayoutPass` the CLI chooses. `Off` when no sidecar is written (`--no-build-json`, no `-o`); `Owner` (each item once,
  in the module that declares it, nothing in a module that declares none) when one is; `Full` (every importer's view,
  A4's two-layouts detector half) only in the `build_export_layouts_crosscheck` gate over the 7 shapes. (4) Still true:
  the owner pass evaluates each module's own `pub const`s in a registry evaluator of its own, not the lowering's. A
  `pub const` initialised with `embed(...)`/`import(...)` is therefore read a second time, through the same sandbox
  root; those files are already rows of the build's inputs digest (the lowering reads them first), so no row is new,
  but the read is duplicated. Reusing the lowering's evaluator is not clean (its evaluators are per item and private to
  `lower_module_inner`), so this is booked, not fixed.

## G0A-LINE-KIND-CODE

- state: **CLOSED** 2026-11-18 (G0a Task 6 fix round 1)  size: `M`  project: `G0A-BUILD-EXPORT`
- Found in G0a Task 6: the IR could not tell an instruction emit from a `dc`/blob, so every located line row was
  `data`. Ruled YES by the controller. Closed by `sigil_ir::Module::code` (`CodeMark`: runs of fragment indices per
  section, inert metadata like `Module.procs`; no link step reads it, `Module` derives no `Hash`/`Serialize`). The
  `.emp` lowering marks around each `CodeItem::Instr` in `lower_code_buf`, the AS front end around
  `lower_instruction`; `dc`/`ds`/`binclude`/data items are never marked. The driver hands the marks to the exporter by
  section name (`SideTables.code`), and `lines.rs::row_kind` reads them; a mark that names no single placed `Data`
  run is refused (`ExportError::CodeMark`). s4 plain: 12,086 code rows, 581 data rows. Gate 2 holds code rows to
  placed 68000 sections at even address and length, and requires code rows on s4 plain.
- Still true, unchanged: the header's ROM-end field (`$1A4`) is re-written after the link in appendix shapes
  (`append_deb2_appendix`) but stays attributed to its declaring line (`rom_end:` in `header.emp`). Only the checksum
  is a declared no-source range, per spec §4.4.

## G0A-TOOL-REVISION-STAMP: CLOSED

- state: **CLOSED** 2026-11-18 (G0a Task 10; heading given by the final review's M10, it sat under the row above)
  size: `S`  project: `G0A-BUILD-EXPORT`
- Found in G0a Task 10. `tool.revision` is `env!("SIGIL_REVISION")`, baked by `crates/sigil-cli/build.rs`, whose
  rerun triggers (git HEAD, refs, packed-refs, closure sources) DO re-capture on a commit: probed with two empty commits
  on a scratch branch, `cargo build` and `cargo test --no-run` both re-stamped. The stale `9b75e609` Task 6 saw was a
  binary no cargo invocation had rebuilt since the commit moved. Not reproduced as a build-script defect, so no build
  change; instead `build_export_gates::tool_revision_names_the_checkout_head` refuses a `tool.revision` that is not
  `unknown` or the checkout's HEAD, so a stale stamp is red in the landing bar rather than silently exported.

## FREEZE-INTO-WRITES-BEFORE-IT-VALIDATES

- state: **open** 2026-11-19 (found in G0b T-REPIN, red-first of precondition 4)  size: `S`  project: `G0B-GAME-AS-DATA`
- `provenance::freeze_into` documents "the file on disk is untouched on every refusal", but its LAST refusal is
  not: `append_block` writes the ledger (parsed, atomically) and only then does `check` run over the new chain, so
  an "appended entry FAILS validation" error leaves the invalid entry ON DISK. Shown when T-REPIN's
  precondition-4 guard was removed: the re-pin past a red tip was written, then refused by
  strict-attest-monotonic, and `tests/repin_aeon.rs`'s "a refusal must not touch the ledger" assertion fired.
  Every guard upstream of the write is meant to make this unreachable, so it is a defence-in-depth gap, not a
  live fault; the ledger is tracked, so `git restore` recovers it. Fix: run `check` on the in-memory chain
  (`parse` of `new_src` plus the blobs) before `write_atomic`, and add a test that drives the post-append path.

## G0B-TASK4-GAME-TOUCHING

- state: **open** 2026-11-19 (found in G0b Task 4)  size: `M`  project: `G0B-GAME-AS-DATA`
- Task 4 moved the game-independent driver into `crates/sigil-build` (`diag`, `place`, `map_placement`,
  `guards`, `emit`, `appendix`, `harvest`) with bodies unchanged, and sigil-harness re-exports every moved name.
  What did NOT move is what would carry a game or description fact into sigil-build unchanged. Task 5 replaces
  each from the descriptions; line numbers are `crates/sigil-harness/src/native.rs` at the Task 4 tip.
  1. **Profile (`GameProfile`, :83) and everything that reads it:** the eight constructors and `shipped_shapes`
     (:347-:639), `shape_defines` (:673), `module_shape` / `registry_from_manifest` / `module_registry`
     (:1189-:1221), `prepare_program` / `name_env_report` / `build_emp` (:1230-:1345), `resolve_chained` (:2189),
     `check_chained` (:2314), `build_rom_chained(_with_listing)` (:1992, :2324), `build_full_file_chained`
     (:2434, also the per-game deb2 floor by path), `placement_map` / `overlaid_placement` (:2456, :2471), the
     frozen-layout readers (:2493-:2590), `declares_error_handler_island` (:2930), `harvest_engine_constants`
     (:786), `harvest_game_constants` (:829), `harvest_engine_ram_addresses` (:961), `assemble_as_side` (:1058,
     also the `SIGIL_EMP_*` AS definedness defines, port history).
  2. **Frozen tables:** `FROZEN_TABLES` / `load_frozen_table` (:300, :316). **The `section_align` table:**
     `packed_chained_base` (:1578), `validate_declared_alignment` (:1609), `validate_resolved_alignment` (:1646),
     and their callers `packed_true_bases` (:1684) and `true_bases_by_index` (:1480): the packing walk itself
     stays until the evenness default replaces its one table read. **Sound (seam-2 heads, G0c):**
     `validate_sound_fold` / `check_sound_fold` (:2048, :2078), `emit_generated(_in)` / `ensure_generated`
     (:711-:747), `ChainedResolve` (:2154, holds `seam1::ResidentLink`).
  3. **Engine tables `engine.toml` now carries:** `STRUCT_OFFSET_TWINS` + `harvest_engine_struct_offsets`
     (:861, :888), `RAM_HELPERS` (inside :961), the entry seeds `engine.game_contract` / `engine.ram` in
     `synthetic_entry_src` (:1152).
  4. **Game labels and per-game constants:** `relocate_fixture_pool` (:1956, `OJZ_*` heads, `ReleaseFault` /
     `BusError`), `SONIC4_APPENDIX_FLOOR` / `DEMO_APPENDIX_FLOOR` (:2893, :2895), `STAGE1_` / `DEMO_INAPPLICABLE_GUARDS`
     (:1446, :1450, both empty), and the sonic4-canonical wrappers (`build_native_rom*`, `build_native_full_file`,
     `sigil_native_symbol_listing`, `section_extents`, `section_label_owners`, `phase_bank_lmas`,
     `project_memory_map`, `resolve_canonical_sections`, :1983 and :2655-:2880).
  5. **Moved, but carrying ENGINE facts the descriptions also hold** (not game facts, so not a blocker for Task 12's
     game-name gate, which counts engine ids without gating them): `sigil_build::appendix::ERROR_HANDLER_BLOB_LABEL`
     (`ErrorHandlerBlob`, engine.toml `error_handler_blob`), `ERROR_HANDLER_BLOB_LEN` (0xF56, the vendored blob), and
     `<aeon>/tools/convsym` inside `append_deb2_appendix` / `convsym_resolve`. Task 5 passes them in from the
     engine description rather than reading the constants. **CLOSED for the new driver in Task 5:** engine.toml
     gained `error_handler_blob_len = 0xF56` and `convsym = "../tools/convsym"`, and `sigil_build::driver` calls
     `appendix::append_deb2_appendix_with(AppendixFacts { .. })` with them. The old entry points keep the literals
     and delegate until Task 11 (`convsym_resolve` still reads `<aeon>/tools/convsym`).
  6. **What Task 5 replaced, and how (the new driver, `crates/sigil-build/src/driver.rs`).** Items 1-4 still stand
     for the OLD driver, which stays until the cutover. The new driver re-derives each from the descriptions:
     defines (`merged_defines`), gating (`roots`, `gating`), the four harvests and the AS side (private to
     `driver.rs`, duplicates of the harness bodies until Task 11), the synthetic entry (engine `entry_seeds`
     paired with `entry.game`/`entry.ram`), the fixture relocation (`fixture_placement.relocate` before the
     first engine `fault_island_heads` row), the deb2 floor (`appendix_floor`), the inapplicable-guard allowlist
     (empty, as both Rust tables are), the island exclusivity check, and the walk (`walk`, no frozen table). The
     sound seams and the side tables reach it through `driver::Seams` (`sigil_harness::driver_seams::HarnessSeams`)
     until G0c. The `SIGIL_EMP_*` AS definedness defines are NOT passed: no residual AS file tests them (grep of
     aeon for `SIGIL_EMP_` hits only `.emp` comments), and the new driver's ROMs are byte-identical to the old
     pipeline with only the walk swapped, which still passes them (see G0B-TASK5-WALK-IDENTITY; the fix-round-1 rule-only experiment also kept them and matched 7/7). `SST_interact =
     sizeof(Sst) - 2` stays a literal derivation in `driver.rs` (a residual-AS twin; no description field).

## G0B-DESCRIBE-TASK2-RESIDUE

- state: **open** 2026-11-19 (found in G0b Task 2)  size: `S`  project: `G0B-GAME-AS-DATA`
- Four things the description formats leave for later G0b tasks, each a decision rather than a defect:
  1. **Strict census scope: CLOSED in Task 2 fix round 1.** The two description-reading tests moved to
     `crates/sigil-build/tests/game_descriptions.rs`, resolve the tree through `AEON_DIR` (the mirrored A8 / d-18
     resolver `sigil_build::reference::aeon_dir`; the parcel-only `AEON_G0B_DIR` is gone), and are a
     `nightly_source_gates.sh` SOURCE_GATES member. `skip_marker_lint` and `strict_census_lint` were each shown red
     on a mutation of that file.
  5. **Misspelt define overrides: CLOSED in Task 5.** `resolve_shape` accepts a shape override of a define nobody
     declared (`defines = { STRES_EVICT = 1 }` resolves to a new define, no error): a hole in closed inputs. The
     check needs the engine's `[defines]` as well as the game's, so Task 5's driver must refuse an override of an
     undeclared define, by name. `driver::merged_defines` refuses it (`[define.undeclared-override]`), and a game
     row redeclaring an engine define or a map `[defines]` row redeclaring either (`[define.redeclared]`);
     `driver::tests::a_shape_override_of_an_undeclared_define_is_refused`.
  6. **G0b landing reference: one red remains, pending a controller ruling.** One strict landing bar against a built
     G0b copy (aeon 351acc8d, sigil bf6f222f) went 6097 passed / 2 failed, both pinned to the 99cb963d reference.
     (1) `build_export_binding::no_build_json_writes_none`: **fixed in Task 2 fix round 2** (ruling: the claim is
     "a build without -o writes no sidecar"; the test now asserts a pre-existing `demo.build.json` is unchanged,
     shown red in both pre-absent and pre-present cases). (2) `provenance_chain::aeon_dir_matches_the_provenance_tip`
     **remains**: the tree is not the provenance tip; the provenance re-pin is the controller's ruling, the test and
     `golden/provenance.toml` are untouched. A standalone copy also needs full git history and every `suite_path()`
     sibling linked beside it. **Ruled (proxy, 2026-11-19): re-pin at landing with `refreeze --freeze NAME --ab REF
     --repin-aeon`** (built and unit-tested in T-REPIN, `tests/repin_aeon.rs`; never run against the real chain yet).
     **Landing order:** written once, in `docs/OVERSEER-REFERENCE.md` ("The G0b landing order"; amended by the G0b
     final review D-I1: empyrean first, then aeon (d), sigil (e)-(g), oracle, Cherub).
  2. **SUITE_PATHS step 3.** `resolve_engine_dir` takes steps 1, 2 and 4 only. Step 3 (derive from the calling repo)
     has no meaning for an installed binary, and deriving from the game's location would find the owner's live aeon
     for a game inside an aeon worktree. A game inside the engine's own checkout (sonic4, demo) is the "own checkout
     is observed" case; the driver (Task 5) decides how it observes it.
  3. **Baseline union vs today's corpus.** Baseline rows are split by where the proc lives (engine / sonic4 / demo).
     Today's closure analyses the whole aeon corpus on every shape (`corpus_contracts.rs` `bind_corpus_interfaces`),
     so a demo shape fires the 18 sonic4 rows too; engine + game union equals today's gate only once a build's corpus
     is its own game plus the engine (Task 9). Until then the agree-with-Rust check must union all three for demo.
     **Task 8:** the union is now the driver's own rule (`sigil_build::baseline::closure_baseline`: other scanned
     games contribute their rows that name a proc in their own sources), which the project gate uses and the
     agree-with-Rust test compares. Task 9 can delete that rule only AFTER aeon settles the engine/game contract for
     `SFXID_REV_LOOP` (an engine-root `comptime` condition on a constant only sonic4 declares; aeon DEFERRED_WORK
     G0B-ENGINE-NAMES-GAME-CONST, a Task 9 prerequisite the two-root corpus does not remove; G0B-TASK5-NEW-PATH-GAPS).
     **CLOSED by Task 9a** (2026-11-20; aeon 58df4539, paired): aeon `engine/engine.toml` declares
     `required_consts = ["SFXID_REV_LOOP"]` (a game omitting one is refused by name at `load_project`,
     `sigil_build::driver::check_required_consts`) and demo declares it `-1`. A `--project` shape's closure now
     analyses only the engine root and the game's own root (`Project::closure_roots`), and `closure_baseline` reads
     the engine's and the game's `baseline.toml` and nothing else: the other-games rule is deleted. The
     agree-with-Rust test compares the Rust constants restricted to the procs that narrowed closure can fire (a Rust
     row naming a proc that lives only outside both roots is not wanted by it). Measured: demo 80 modules, sonic4
     197, the gate passes on all 10 shapes.
  4. **Facts left in Rust on purpose.** `harvest_engine_constants`' `VDP_Shadow_len` exclusion (`native.rs:940`) is a
     residual-AS collision rule, derivable as "a constant a struct twin also produces"; it is not a field. The
     `SIGIL_EMP_*` AS definedness defines (`native.rs:1228`) are port history. The `--game`/target names in
     `sigil-cli` `main.rs` (`label_and_profile`, :3792-3798) go with the legacy CLI.
  7. **Task 3 agree-with-Rust residue** (`crates/sigil-harness/tests/descriptions_agree_with_rust.rs`). (a) Its demo
     baseline was the union engine + sonic4 + demo (item 3). **(a) CLOSED by Task 9a, not Task 9** (the union could not
     go until aeon stated `SFXID_REV_LOOP` as a required game constant, which Task 9's two-root corpus does not do): the
     description side is now engine + game only, and the test restricts the whole-corpus Rust constants to the procs
     the narrowed closure can fire. (b) `GateRule.modules` is documented as "a module id, or a glob over ids" but `describe.rs`
     has no matcher; the test matches exact ids and a single `*`. No row uses a glob today; Task 5's closed gating
     owns the real matcher and must replace the test's. **(b) CLOSED in Task 5:** `sigil_build::gating::glob_match`
     (`*` within a dotted segment, `**` across) is the matcher, and the test imports it. (c) Like item 4 of the Task 2 report, the test reads the
     private Rust tables (`COMPTIME_HELPERS`, `RAM_HELPERS`, `STRUCT_OFFSET_TWINS`, `GROWABLE`, the sound labels, the
     entry seeds) as source text; Task 11 re-homes it when the old tables go. (d) Fields with no Rust counterpart
     (`outputs`, `id`, `engine.suite_path`) and Rust fields with no description counterpart (`frozen_sizes`,
     `inapplicable_guards`, `extra_entries`, `anchor_overlay`, `layout_pass`, the AS definedness defines and the
     even-alignment rows) are not compared; they are listed in the Task 3 report.

## G0B-TASK5-WALK-IDENTITY

- state: **closed** 2026-11-20 by G0b Task 7 (opened 2026-11-19 in G0b Task 5; measurement corrected in Task 5 fix
  round 1)  size: `M`
- **Closed (G0b Task 7, sigil 14c6d44e = this branch + `parcel/walk-measure-in-place` merged, aeon 559b8910):** after
  the merge the old walk measures every non-bank section in place, and `scripts/compare-drivers.sh` built all 10
  shapes (the 7 plus the 3 fixtures) with `--legacy` and `--project` from two clean clones: the ROM is SHA-256
  identical on 10/10. The one rule below was the whole ROM difference, as predicted. (The `.lst` and export are a
  separate question, G0B-COMPARE-IDENTITY-NAMES.) The old driver after the merge equals the walk tip 87b5c195's binary
  on the 7 shapes (`step0_shapes_cmp.sh`, SHAPES 7/7 IDENTICAL against aeon 99cb963d), which is the 7/7 base from now on.
- project: `G0B-GAME-AS-DATA`  ruled (proxy, 2026-11-19): Option B, a separate parcel changes the OLD walk to measure
  every non-bank section in place, freezes a new golden, and merges before cutover (done: `parcel/walk-measure-in-place`,
  merged by Task 7 step 0).
- The finding as booked in Task 5 (history):
- The new driver builds all seven shipped shapes (every guard green: placement, alignment, budgets, sound fold, bank
  ids, the blob-end contract; `tests/game_as_data_driver.rs`), but no shape is byte-identical to the old driver.
- **Measured (fix round 1), all 7 shapes: one rule accounts for the whole difference.** A scratch, never-committed
  binary changed ONE line of the old walk, the per-round remeasure filter in `native::packed_true_bases`
  (`if labeled[i] && !bankish[i]` became `if is_rom_section(&sections[i]) && !bankish[i]`, so every non-bank ROM
  section is measured at its packed base). Nothing else changed: the round-0 seed (`labeled[i]` pins only), the
  phase-run tails, the unnamed-section order, the label-less ANCHOR_GAP islands and the `SIGIL_EMP_*` defines all
  stayed old. Built from one aeon c9053d4c copy, that binary's FULL FILE equals the new driver's on all seven
  shapes (`cmp`-identical, image and appendix). So the round-0 seed, phase-run tails, unnamed order, label-less
  islands and the dropped defines are each byte-neutral on today's tree. This supersedes the Task 5 claim, which compared 4
  shapes with the whole walk swapped and overclaimed "all 7".
  Patch: `.superpowers/sdd/2026-11-17-g0b-game-as-data-driver/scratch/task5-fr1/old-walk-rule-only.patch`.
- The rule: the old walk remeasures a section at its packed base only when a FROZEN table names one of its labels;
  every other section is measured at a far scratch slot (0x700000+), where short `abs.w` forms cannot reach, so the
  measured length is the `abs.l` one, the section is emitted shorter at its real base and the difference stays as
  zero padding (sonic4 release `VBlank_Handler`: 0x1E8 at scratch, 0x1E0 in place, 8 zero bytes at 0x2508..0x2510).
  The mechanism can only shrink the image, and it does: no image grows.
- Per shape (bytes). Image = `EndOfRom`; appendix = file - image (convsym deb2, island shapes only).

  | shape | old image | new image | old file | new file | old appendix | new appendix | distinct sym addrs old/new | first image diff |
  |---|---|---|---|---|---|---|---|---|
  | release | 802428 | 802424 (-4) | 850296 | 850262 (-34) | 47868 | 47838 (-30) | 2285/2283 | 0xB |
  | debug | 810122 | 810122 (0) | 870560 | 870606 (+46) | 60438 | 60484 (+46) | 2832/2834 | 0x395 |
  | demo | 70170 | 70170 (0) | 99994 | 99980 (-14) | 29824 | 29810 (-14) | 1300/1299 | 0x3A3 |
  | demo.debug | 70170 | 70170 (0) | 106981 | 106981 (0) | 36811 | 36811 (0) | 1577/1577 | 0x393 |
  | config-a | 810122 | 810122 (0) | 870972 | 870972 (0) | 60850 | 60850 (0) | 2853/2853 | 0x395 |
  | config-b | 570932 | 570866 (-66) | 617813 | 617715 (-98) | 46881 | 46849 (-32) | 2212/2210 | 0xB |
  | lean | 798202 | 798198 (-4) | 798202 | 798198 (-4) | none | none | n/a | 0xB |

  Header bytes 0x18E..0x18F (checksum) differ in every shape and 0x1A4..0x1A7 (ROM end) wherever the file length
  moved. The debug shapes' images do not change length because the fault island is pinned and is the last section;
  the shrinkage before it becomes slack before the island, so contents move (first diff 0x393..0x3A3) but
  `EndOfRom` does not.
- **The debug +46 is entirely in the deb2 appendix**, a second, downstream effect. The symbol set is identical
  (same names, same count in every shape); only addresses move. convsym's table size depends on how many symbols
  share an address: shifting ONE symbol (`Parallax_InstallScratch`, which shares 0x7476 with `Raster_Install` in the
  new release build) by one byte in an otherwise unchanged listing grows the appendix by 14 bytes (15629 -> 15643,
  direct convsym run). Every shape's appendix delta follows its distinct-address delta: +2 -> +46 (debug), -2 -> -30
  (release), -2 -> -32 (config-b), -1 -> -14 (demo), 0 -> 0 (demo.debug, config-a; same size, re-encoded
  content). Examples of collisions that change: release/config-b gain `Parallax_InstallScratch`=`Raster_Install` and
  fold `Replay_OJZ_Fixture` into the `BgAnim_VProbe_*` group; debug loses `DacSampleTable`, `SfxBlobWinTab` and a
  pitch-table group's collisions with `Parallax_Step4_Fill` locals and gains one with `SeqOpcodeTable`.
- Nothing differs beyond the one rule. The Option B parcel can take the one-line change above as its complete
  old-walk delta for today's tree; it moves every golden (image and appendix) and is the byte-moving commit.

## G0B-TASK5-NEW-PATH-GAPS

- state: **closed** 2026-11-20 by G0b Task 8 step 0 (found in G0b Task 5; partly closed by Task 7)  size: `M`
  project: `G0B-GAME-AS-DATA`
- **Closed by Task 8 step 0 (controller ruling: a prerequisite of the demo cutover).** `--project` runs the
  CONTRACT GATE before it builds, and takes `--check` (`sigil_build::driver::check_project`, the build pipeline
  cut after the guard census, the same cut as `native::check_chained`) and every `--report` kind (contracts,
  indirect-cost, ram, name-env), with the legacy grammar's pairings and refusal words. The gate is one function
  over a `ClosureSubject` (label, defines, game prefix, baseline) each driver builds: the profile path from the
  Rust constants (`ShapeBaseline::for_profile`), the project path from the descriptions
  (`sigil_build::baseline::closure_baseline`: engine/baseline.toml, the game's baseline.toml, and, because the
  closure still analyses the whole scan, each OTHER scanned game's rows that name a proc in that game's own
  sources). The gate states its pass on stderr (`contract closure gate: passed (`).
  - **Measured, why the corpus is NOT narrowed to the game yet:** on aeon 559b8910, demo release, analysing only
    `engine.*` + `games.demo.*` leaves `Sfx_BeginSound`'s comptime condition on `SFXID_REV_LOOP` (a sonic4 id)
    unresolved, so the gate goes blind (`[comptime.unresolved]` 1). **Corrected in Task 8 fix round 1:** this is
    NOT something Task 9's two-root corpus resolves. `engine/sound/sound_sfx.emp:874` is in the ENGINE root, and
    its comment (`:872-873`) calls the constant "game-declared; -1 = feature off": the engine already treats it as
    a constant every game must provide, demo simply does not declare it, and it resolves today only because
    sonic4's sources are in the scan. An engine + demo corpus still analyses it. Settling that engine/game
    contract (aeon DEFERRED_WORK G0B-ENGINE-NAMES-GAME-CONST: (i) every game declares it, or (ii) a declared
    default / required-constant contract in engine.toml; controller to rule before Task 9) is a **prerequisite**
    for deleting `closure_baseline`'s other-games rule. Measured (scratch, aeon 3557d081, nothing committed):
    with demo declaring `SFXID_REV_LOOP = -1` and the corpus narrowed to `engine.*` + `games.demo.*` (79 of 207
    modules) against the engine + demo rows only, the gate passes on demo release and demo debug, so it is the
    only blind spot on demo; without it both shapes are blind.
  - **Proof:** `crates/sigil-cli/tests/project_contract_gate.rs` (8 tests, a two-game fixture whose baselines fit
    its corpus, so the exit code is the verdict: a planted clobber is refused with the same located line on
    `--project` and `--aeon`; a dropped other-game row is NEW; a stale game row is GONE; another game's row on an
    engine proc does not join). `descriptions_agree_with_rust` compares `closure_baseline`'s result with the Rust
    tables on all 10 shapes (its hardcoded demo union is gone). The comparison job gains `gate=` (both logs carry
    the pass line), `report=` (identical `--report contracts` censuses) and `check=` (identical `checked:`
    counts) columns, held by `driver_comparison`; its declared delta now also lists the files the project gate
    reads (the engine's and the game's baseline.toml, every other games/*/game.toml and its baseline.toml).
  - The warn-tier view needed nothing more: `report_warnings` already ran on both paths.
  - **Superseded by Task 9a (2026-11-20):** the corpus IS narrowed now (engine root + the game's own root), the
    other-games rule and its reads are gone, and the fixture test's other-game cases became "another game's
    sources and files are not this game's gate" and "a game missing a required constant is refused by name"
    (10 tests). See G0B-DESCRIBE-TASK2-RESIDUE item 3 and G0B-TASK9A-RESIDUE.
- **Closed by Task 7:** `--project` now writes the listing (`--emit-lst`, with its source digest) and the
  `<stem>.build.json` export (unless `--no-build-json`) through the SAME writer as `--aeon`
  (`write_build_artifacts`; each driver supplies a `BuildIdentity`). The `engine/debug/generated/` item below is
  handled by the comparison job, which produces the blobs in each clone with `tools/gen_compression_vectors.py`.
- **(History) Still open after Task 7, ruled as Task 8's step 0:** the CONTRACT GATE (`run_contract_gate`) does not run on
  `--project`. It is keyed to the legacy `BuildTarget`/`GameProfile` (`corpus_closure_or_exit`, `diff_d1c(profile)`,
  baselines from `contract_baseline.rs`), so a `build.sh` arm switched to `--project` would ship builds no closure gate
  checked. It does not affect the comparison (its reads were already in the old read set; the listings agree on every
  other row). Also still open: `--check`, `--report`, the warn-tier view (`report_warnings` now runs, the view is
  the same function).
- As found in Task 5, before Task 7 (history): `--project` wrote the ROM and nothing else; `--emit-lst`, `--check`
  and `--report` were refused by name and `--no-build-json` was accepted and ignored. Today `--check` and `--report`
  are still refused by name; `--emit-lst` and `--no-build-json` work as on `--aeon`. `--legacy` is accepted on the
  old grammar and not yet required for `--aeon`/`--game` (controller ruling for Task 5).
- Candidate scope is the old rule (`engine.` or the game's own module prefix), not the description's source paths;
  the probe fixture is an ordinary candidate gated `shapes = []`. Task 9's two-root corpus is where a path rule
  belongs.
- `.aeon-g0b` carries no `engine/debug/generated/` blobs (untracked build outputs), so a debug shape cannot build
  from a bare copy of it; the Task 5 comparisons copied them from the 99cb963d reference.

## G0B-GENERATED-ROOT-SCOPE: CLOSED

- state: **CLOSED** 2026-11-20 (G0b Task 9)  size: `S`  project: `G0B-GAME-AS-DATA`
- Both halves are gone. The thread-local (`sigil_frontend_emp::generated_root`) is deleted: the generated root is
  a named root of the lowering options (`LowerOptions::roots`, `sigil_span::roots::SourceRoots`), handed to the
  evaluator as a `Sandbox::Roots` like any other root. The prefix alias (`build_dir::GENERATED_MOUNT`,
  `generated_root_for`) is deleted: the seams write `<build>/generated/sound/`, aeon spells the 13 embeds
  `embed("generated/sound/<file>")` (paired aeon commit), and a build with no generated root refuses a
  `generated/` path by name ([root.missing]). The record layer (`export_files::GeneratedRoot`) keeps G0a's
  equivalence `generated/<sub>/<rest>` = `engine/<sub>/generated/<rest>`, so digest and export rows for generated
  reads are unchanged.
- Residue: the legacy driver's sound mount (`SourceRoots::with_legacy_sound_mount`) was meant to die with that
  driver at Task 11 and did not; it had no caller left and was deleted by the G0b final-review fix wave (C-I4).
  The retired prefix is refused by name (`[root.stale-generated]`); `LEGACY_SOUND_MOUNT`, an aeon layout literal in
  the language-level crate, stays only for that refusal and goes with the seams (G0c, `G0C-SOUND-SEAMS`). The
  standalone port gates still provision the tree's mount (`native::ensure_generated`); they go with the seams in G0c.
- The original text follows.
- A build's generated root (`sigil_frontend_emp::generated_root`) is THREAD-SCOPED: the build enters it, and
  `run_on_eval_stack` carries it to the comptime evaluation thread. It is not a `LowerOptions` field because the
  sandbox is reached from data lowering, constant folding and layout registration, and `LowerOptions` has 372
  struct-literal sites. Two builds on two threads each see their own root; a thread nobody handed the root to sees
  none (`generated_root::tests`). Task 9's two-root resolution names roots in the lowering options; fold the
  generated root into that and delete the thread-local then.
- The mount itself is a PREFIX ALIAS keyed to the checkout, not a named root: `build_dir.rs:65`
  (`GENERATED_MOUNT = "engine/sound/generated"`) and `:134` (`generated_root_for` mounts the directory at
  `<checkout>/engine/sound/generated`) redirect a tree path, and the aeon sources keep spelling the tree path.
  Task 9 replaces the alias with named-root resolution: the embed names the `generated` root and the path inside
  it, and nothing is keyed to the checkout (under Task 9's roots, `engine` is aeon's `engine/` directory, so the
  checkout-relative alias would have to be re-derived anyway). (Task 6 review, Minor 1.)
- Who enters a root (fix round 1): `sigil build` (both drivers), the harness's chained and frozen resolves,
  `native::build_emp` (always fresh, sound emitted into it), and the focused lowerings (`harvest_*`,
  `name_env_report`; an enclosing root or an empty one). With no root the sandbox reads a mount path from the
  tree like any path; the frontend cannot refuse it because it does not know the mount. Task 9's named root
  removes that gap structurally.
- `native::emit_generated` / `ensure_generated` still write the tree's own mount. No build calls them; they
  provision a reference tree for STANDALONE port gates (`soundbankhead_port`, `dac_bank_port`, ...) that compile one
  sound module with no build around it. Those go with the seams in G0c (aeon `docs/DEFERRED_WORK.md`, G0c).

## G0B-TASK5-AEON-DIR-LEAK: CLOSED

- state: **CLOSED** 2026-11-19 (G0b Task 6)  size: `S`  project: `G0B-GAME-AS-DATA`
- Found in the Task 5 review: `main.rs` published `--aeon` (old path) and the project checkout (`--project`) as the
  process-global `AEON_DIR`, bypassing the seam's `checkout` parameter. The only reader was the seams' write
  precondition (`seam2::require_named_reference_tree`), which asks a write INTO a tree to name it. Task 6 moved
  every build write into the build directory and made the precondition ask for a name only when the output lands
  inside the tree (`seam2::require_reference_tree_for`), so both sets are deleted. Proof: `build_hermetic` runs
  `--check`, two full builds and two `--project` builds with `AEON_DIR` removed from the child environment, and
  the new driver's ROMs are identical before and after on all 7 shipped shapes (Task 6 report).

## EXTRA-ENTRY-ONE-TIME-FLAKE

- state: **watching** 2026-11-19 (seen once in G0b Task 4, booked in Task 5)  size: `S`  project: `-`
- `sigil-cli --test extra_entry a_passing_extra_entry_moves_no_bytes` failed once during Task 4's C6 quick tests, at
  `extra_entry.rs:352`, in the `unreachable(&base)` precondition, and passed on an immediate rerun, in the full
  binary twice (iteration copy and a fresh `git clean`ed copy) and in both landing runs. Cause not found; the likely
  candidate is a concurrent-build race on the shared iteration copy (that run was the only one with other cargo
  tests running at the same time, and the binary's LOCK covers only its own tests). Reopen with the output if it
  recurs.
- G0b Task 6 removed one such race: before it, every sound-on build in any process rewrote the shared copy's
  `engine/sound/generated/`, and a build reading those files while another rewrote them could see a short file.
  Builds now write only into their own build directory.

## WALK-MEASURE-IN-PLACE

- state: **LANDING** on `parcel/walk-measure-in-place` (2026-11-19)  size: `S`  project: `G0B`
- The old walk (`native.rs` `packed_true_bases`) re-measured a section at its packed base only when a frozen size
  table named one of its labels; the rest sat at a far-scratch slot. Now every non-bank ROM section is re-measured in
  place (one filter, `labeled[i]` -> `is_rom_section`). It moves bytes in every shape; proof in
  `docs/evidence/walk-measure-in-place/` (`scripts/walk_inplace_proof.sh`, 9/9 shapes EXPLAINED, 0 unexplained).
- Found while proving it, booked:
  - The padding was NOT only after never-pinned sections. A PINNED section that branches into a scratch-measured one
    measured its `bsr.w` as `jsr abs.l` too (`vblank` 8 B, `tile_cache` 14 B, `load_art` 6 B, `parallax`,
    `game_loop`, `player_common`, `ojz_scroll_test` in sonic4 release), and an internal short branch spanning such
    sites could cascade to `.w` (`player_glide.emp:120`). The proof's site model covers all three and agrees with
    sigil's own `width_flip_report` (scratch diagnostic build, patch kept) on all 9 shapes.
  - lean carries one EMITTED width flip: `page_in.emp:290` `lea` abs.l `$8018` -> abs.w `$7FDC`, because the shift
    moved its target below `$8000`. The other shapes have none.
  - Round 0 still seeds lengths with the frozen-label pins and the far-scratch slots (`SCRATCH_FIRST`,
    `far_scratch_slot`); only the converged lengths are in place now. Seeding round 0 in place too would retire the
    scratch machinery from the old walk; byte-neutral by construction only if the fixpoint is unique, so it needs its
    own proof. Moot if G0b deletes the old driver first.
  - The cross-check diagnostic (an env-gated `width_flip_report` at convergence) is not in the tree. If walk changes
    recur, a committed `--report walk-flips` would make the proof's site model checkable without a scratch build.
  - FREEZE #210: first attempt failed, and NOT because of this parcel or Python 3.14 (my first guess, wrong). Its
    AEON_DIR was a `cp -a` of `.aeon-sigil-ref-99cb963d`, which carried `tools/__pycache__`; pytest's
    assertion-rewritten bytecode keeps the SOURCE tree's path as `co_filename`, aeon `land_gate_audit._who()`
    compares `dirname(co_filename)` to its own `tools/`, never matches, and charges 3541 docs reads to
    `<unattributed>`, so `build.sh`'s pytest lane exits 1 with every test passing. Proven red/green
    (`docs/evidence/walk-measure-in-place/pycache-red-green.txt`); the freeze then completed from a fresh clone.
  - FRAGILITY, open (sigil side; aeon books its own): any tree a sigil lane makes with `cp -a` for `AEON_DIR`
    (freeze, attest, landing, proofs) must drop every `__pycache__`
    (`find <copy> -name __pycache__ -type d -prune -exec rm -rf {} +`) or be a `git clone`/`git worktree add`.
    The durable fix is aeon's: compare paths in `_who()` in a way a copied cache survives (e.g. by the
    basename under any `tools/`, or by `PYTHONDONTWRITEBYTECODE`/`-p no:cacheprovider` plus
    `sys.pycache_prefix` for the lane). Until then a copied tree reads as broken tooling.
  - RESOLVED (controller ruling, G0b ledger; owner may override): `end_measures = "allotment"` RETIRED. Its last user,
    `player_climb`, was zero-width in both shapes under the new walk; it now ends `section:player_instashield`
    (the window has always covered `Ability_InstaShield`, so the pin does not move; the ruling said
    `section:player_climb`, which would have shrunk the pin by the insta-shield section). `region_end_contracts`
    is re-grounded: the parse guard on the manifest's `[[region]]` count, the negative control on re-spelling
    each `section:` end as its successor's head label (78 of 80). Red-first in `region-end-contracts-rf.txt`.
  - LESSON, process: tip #210's attest was recorded FAILED (6090/1) because `repin_pins`'
    `generated_pins_match_the_hand_typed_baseline` asserts literal bases that the in-place slide moved (nine
    literals). A byte-moving freeze must run that test (and fix its literals) BEFORE `refreeze --attest`; an
    attest cannot be retried. Closed by #211 `walk-measure-in-place-r2` (`--supersede-tip`, byte-identical goldens),
    attested PASSED 6091/0/3. A `refreeze --freeze` pre-flight that runs `repin_pins` would make this impossible.
  - OPEN, pre-existing (not this parcel; old sigil 4104e5df identical): aeon `tools/landing_build.sh` at 99cb963d
    passes every lane but cannot stamp, because sigil's G0a build export writes `<stem>.build.json` beside each
    ROM and aeon's land-gate reads the untracked files as the tree changing mid-run. Either aeon ignores
    `*.build.json` or sigil writes the export outside the tree. `docs/evidence/walk-measure-in-place/landing-build.txt`.

## G0B-TASK9A-RESIDUE

- state: open 2026-11-20 (found in G0b Task 9a); item 2 CLOSED 2026-11-20 (G0b Task 9)  size: `S`  project: `G0B-GAME-AS-DATA`
- **Task 9 review minor M6 (`import_check::seam_roots` hardcodes `sonic4_profile`) — left booked at Task 10, with the
  reason.** The cutover path DOES run through it: every sound-on `--project` sonic4 build lowers the seam-1/seam-2
  modules, whose standalone import check resolves among `seam_roots`. For sonic4 it names exactly the build's roots
  (`engine/` and `games/sonic4/`), so the answer is right. It is wrong only for a sound-on game other than sonic4, and
  for such a game the whole seam path is wrong, not just this function: seam 1's file specs and seam 2's
  `SOUND_IDS_REL` read sonic4's files by fixed path, and the `Seams` trait (`sigil-build/src/driver.rs`) hands the
  harness only the checkout. Threading the build's roots into `seam_roots` alone would make one of those facts
  game-relative and leave the rest fixed, which reads as agnostic and is not. Taken with the seams' removal (aeon
  DEFERRED_WORK G0C-SOUND-SEAMS, G0c). Today no sound-on game but sonic4 exists (demo and Cherub are sound-off).
- **Item 2 closed (G0b Task 9):** candidate scope is root membership (`roots::in_candidate_scope` asks
  `ParsedModule::root`), the build scans exactly the closure's roots, and `check_placed_in_analysed` refuses a placed
  module outside them by name ([gate.placed-not-analysed], red-first unit test). One rule. Consequence measured:
  sonic4's four `data.*` seam-input modules became candidates (`data.dac_samples` defines `Dac_Temp_Blip` as
  `games.sonic4.dac_banks` does) and are gated out by name in aeon's `games/sonic4/game.toml`; ROM bytes unchanged.
  Items 1 and 3 stand.
- What Task 9a (required game constants, the narrowed project gate) leaves for later tasks:
  1. **`--closure-scope corpus|game` is a legacy-only report flag.** The comparison job's `report=` column needs the
     two drivers' censuses over ONE module set, and a `--project` shape now analyses engine + its own game while the
     legacy gate keeps the whole scan (its Rust constants are pinned to it, and narrowing the legacy GATE would make
     the 7/7 reference tree, aeon 99cb963d, whose demo lacks `SFXID_REV_LOOP`, go blind). So the legacy
     `--report contracts|indirect-cost` takes `--closure-scope game` (engine root + the profile's game root
     directory) and the job passes it; refused with anything else. It goes with the legacy driver (Task 11).
  2. **Two scope rules for one question until Task 9.** The closure scopes by FILE roots
     (`Project::closure_roots`: engine dir + game sources root), which keeps sonic4's `data.*` modules (files under
     `games/sonic4/data/sound/`, ids outside `games.sonic4`) and drops `tools/fixtures/`; placement candidates still
     scope by ID prefix (`roots::in_candidate_scope`). Task 9's two-root resolution replaces the prefix test with
     the roots, which makes them one rule.
  3. **"Declares" means a `pub const` anywhere in the game's sources root**, sections included
     (`check_required_consts`). That is what the narrowed closure resolves the bare name against. It says nothing
     about the sound-on BUILD: the Z80 sound seams still read sonic4's `config/sound_ids.emp` by fixed path
     (`seam2::SOUND_IDS_REL`, and `seam1`'s table pins `SFXID_REV_LOOP` = `$AB`), so how a sound-on game's own value
     reaches the Z80 driver is a G0c question (aeon DEFERRED_WORK G0C-SOUND-SEAMS). Cherub (Task 13) is sound-off.

## G0B-UNROOTED-TREE-READS

- state: open 2026-11-20 (found in G0b Task 9)  size: `M`  project: `G0B-GAME-AS-DATA`
- Two-root resolution keeps one transitional rule: an UNROOTED path (no `engine/`, `game/` or `generated/` first
  segment) that no root holds resolves against the engine's checkout, and ONLY for a game inside that checkout
  (`SourceRoots::with_tree`; an out-of-tree game such as Cherub gets `[root.unrooted]`). An unrooted path a root
  does hold is refused `[root.ambiguous]`. aeon's spellings that ride the fallback, counted at aeon e6fb33d7 (embed/import/include spellings):
  68 `games/sonic4/data/...` (generator output included: `ojz_block_gen.py`, `bg_emit`, `effects_gen`,
  `smps_import`), 12 `art/...` at the aeon root, 2 `games/sonic4/test/...`. Respell each as `game/...` (or move
  the shared `art/` under a root) and the generators with them, then delete the tree fallback. A G0c item
  (the generators are aeon's).
- **G0b Task 10 (the sonic4 cutover) keeps the fallback.** Every sonic4 `--project` build (build.sh's release, debug,
  config-a, config-b and stress rows) meets it: recounted at aeon `feat/g0b-descriptions` 644fd722, the same 68 + 12 + 2
  spellings, unchanged. Respelling them means editing the aeon generators' templates (`ojz_block_gen.py`, `bg_emit`,
  `effects_gen`, `smps_import`) and regenerating the committed tree, and the 12 `art/` reads need a decision about
  where shared art lives (a root of its own, or under `game/`); that is the G0c item above, not a cutover step. The
  cutover is byte-identical with the fallback in place (`SHAPES 10/10 MATCH`; seven build.sh ROMs equal the
  comparison job's for their shape, and `STRESS_ART=1`'s, which build.sh assembles from a throwaway re-bake the job
  does not do, equals the same command's ROM at the pre-cutover aeon 334dba80: `50e852c5...`).

## G0B-DIGEST-OUT-OF-TREE-ROOTS

- state: closed 2026-11-21 (found in G0b Task 9)  size: `S`  project: `G0B-GAME-AS-DATA`
- **closed** 2026-11-21 (G0b final review D-M2): Task 13a settled the export side (`G0B-EXPORT-BY-ROOTS`, closed:
  every export row is root plus root-relative path, Cherub's export proves it). What remains of the listing side is
  `G0B-LISTING-ROOT-SPELLING`.
- The listing digest names a file outside the aeon tree under its build root (`root=engine|game|generated`,
  `origin=source`) and a scan member there as `<root>:<path>` (`digest_names_each_root`). The new `root=` tokens
  ride FORMAT 1; an older parser refuses an unknown token loudly.

## G0B-ORACLE-PIN-DRIFT

- state: open 2026-11-20 (found in G0b Task 10)  size: `S`  project: `G0B-GAME-AS-DATA`
- oracle's real-export gates (`tools/g0a-real-exports.sh --test`, `fixtures/g0a/{SHA256SUMS,EXPORTS.normalized,
  s4.trim.build.json}`) pin sigil `3f211b78`'s bytes. The walk merge (87b5c195) moved every shape's bytes, and from
  then on the runner was red against every sigil on this branch (with c6725ab3 and the same aeon `99cb963d`: none of
  the five ROMs reproduced), and nothing on sigil's side noticed: no sigil lane runs the oracle runner, and oracle's
  default suite reads only the trimmed fixture, which never rebuilds. Found while re-pinning the shape names at Task
  10, and re-pinned there in two commits on oracle `feat/g0b-shape-names` (the walk byte move first, with the
  `pixels_to_coord(d3)` expansion's -$1E shift; then the `--project` names). The class stays open: a sigil change
  that moves bytes should re-run the oracle runner (or a landing lane should, with the oracle checkout as an
  optional reference like `ORACLE_DIR`), so the next drift is red the day it happens. Decide at the G0b final
  landing (oracle's pins also need sigil's merged revision then).
- 2026-11-21 (G0b Task 13a): **it drifted again, and again nothing here noticed.** The runner with sigil
  `02b8a8f4` on aeon `727ae46b` reproduced all five ROMs and NONE of the five normalized exports. Not sigil's doing:
  aeon's comment-only edits after `644fd722` (Tasks 11 and 12) moved 5 sonic4 and 2 demo files' `sha256`/`size` and
  renumbered 9 `song_bank2.emp` line rows; re-hashing those rows to their `644fd722` bytes and mapping the lines back
  reproduces all five pins exactly. So aeon SOURCE edits (comments included) also move oracle's export pins, which
  widens the class: the trigger is any change to a file the export names, not only a byte move. Re-pinned on oracle
  `feat/g0b-shape-names` `430509a1` (see oracle `fixtures/g0a/PROVENANCE.md`, "Re-pinned 2026-11-21") together
  with Task 13a's root-relative `files` paths (`G0B-EXPORT-BY-ROOTS`): five exports, each classified against
  `02b8a8f4`'s as the stripped `<shape.game>/` plus `inputsSha256` and `tool.revision` only; trimmed fixture exactly
  `files[4].path`, `files[5].path`, `inputsSha256`, `tool.revision`; strict runner 9/9 OK.
- 2026-11-21 (G0b landing step 6): re-checked at the landing SHAs, sigil `57a18265` on aeon `9b80db1d`: 5/5 ROMs,
  5/5 normalized exports unchanged, the trimmed fixture moved only `inputsSha256` and `tool.revision`; strict runner
  9/9 OK; oracle `3e617113`. Lane decision (controller ruling): stays manual, run at each cross-repo landing (as it
  was at this one); moving it into a sigil landing-run lane behind an optional `ORACLE_G0A_DIR` is booked here for
  the owner to decide.

## G0B-HELPER-CLOSURE-LOCATOR

- state: **closed** 2026-11-20, G0b Task 11 (found in G0b Task 10)  size: `S`  project: `G0B-GAME-AS-DATA`
- **Closed by deletion of the input.** Task 11 deleted `COMPTIME_HELPERS` with the profile-driven driver; aeon
  50a96ab9 (`feat/g0b-descriptions`) makes the gate read `engine/engine.toml` `helpers` (the list the game-as-data
  driver reads) from its own tree, so no sigil checkout is located at all; a description without a non-empty
  `helpers` list is refused, red-first on 3 tests. The original finding, kept as the record:
- aeon `tools/emp_helper_closure.py::default_native_rs` finds "the sigil checkout paired with this aeon checkout" only
  for the `<root>/aeon/.worktrees/NAME` layout; for any other aeon tree (every G0b proof copy, which sits at
  `/home/volence/sonic_hacks/.g0b-*`, and `.aeon-g0b` itself) it falls through to `<parent>/sigil`, the MAIN sigil
  checkout. So the pytest lane's `test_emp_helper_closure` (and the gate's no-argument form) read master's
  `COMPTIME_HELPERS`, not the branch's, on every G0b aeon build: the lane passes against the wrong sigil without
  saying so. At Task 10 the two lists are identical (14 helpers), and the closure was re-run explicitly against this
  branch (`python3 tools/emp_helper_closure.py . <.sigil-g0b>/crates/sigil-harness/src/native.rs`: OK, 634 names
  across 14 helpers; the 22 tests with `SIGIL_NATIVE` set to the branch's `native.rs`: OK). Fix on aeon's side: take
  the native.rs from the `SIGIL_BUILD` binary's reported source dir (build.sh already reads it as `SIGIL_SRC`), and
  refuse rather than fall through when no paired checkout is found.

## G0B-TASK11-RESIDUE

- state: open 2026-11-20 (found in G0b Task 11)  size: `M`  project: `G0B-GAME-AS-DATA`
- What Task 11 (the profile-driven driver's deletion) left, each its own small item:
  1. **The residual AS feature defines are byte-inert on every shipped shape.** Booked on its own as
     `G0B-AS-FEATURE-DEFINES-INERT` (controller ruling, Task 11 fix round 1).
  2. **`.target-land/tmp` leaks.** Measured 39G early in Task 11 and 44G at its end; test scratch trees under `CARGO_TARGET_TMPDIR`
     are not removed on success. Find the writers (`grep -rn CARGO_TARGET_TMPDIR crates/*/tests`) and drop on
     success, or have `scripts/landing-run.sh` clear it first.
  3. ~~**`/./` in rendered game-root paths.**~~ Already fixed in 87540c32 (`Project::sources_root` drops `.`
     components; `build_check`'s unreachable-row assertion pins the clean path). Kept as a number so the
     Task 11 report's references hold.
  4. **Old-driver surface still `pub` in `sigil-build`** (corrected by the G0b final review A-M10): `lens_pinned`
     is called only inside `place.rs`, so it can be private; `roots::gate_scan` had no caller at all and is deleted
     (fix wave); `map_placement::move_island_rows` and `game_defines::merge_builtin_and_game` /
     `define_listing_rows` are called only by their own unit tests (`define_listing_rows`'s tests are what cover
     the live `define_listing_rows_from`, so moving them onto it comes first). A visibility pass, no behaviour
     change.
  5. **`scripts/walk_inplace_proof.sh` is historical.** It drives `--aeon`, which no longer parses;
     `golden/provenance.toml` cites it, so it is kept with a HISTORICAL header note rather than rewritten.
  6. **`driver_comparison`'s listing column went red at 87540c32 and stayed red through ec859616**, and the prep
     commits landed anyway (the column was not in the per-commit 7/7). Cause and the last outputs are in
     `docs/superpowers/notes/2026-11-17-g0b-game-as-data.md`. The gate is retired; the lesson is that a parcel's
     per-commit evidence must name every gate the parcel's own work can turn red.
  7. **clippy was not clean on the Task 11 prep commits** (the base 5e45644c was clean): `-D warnings
     --all-targets` failed on five lints those commits introduced (0df8f9a9 and 87540c32) (`too_many_arguments` on `Seams::validate_sound`, an orphan doc comment, a single-element
     loop, `&PathBuf` params, a manual `contains`). Fixed in a265daf1. `scripts/landing-run.sh` (7) runs the
     lint bar, so a LANDING would have caught them; the prep commits' per-commit evidence (tests + step0) did not
     run clippy, so each commit carried them silently. Per-commit evidence on a parcel should include the lint bar.

## G0B-NO-GAME-NAMES-RESIDUE

- state: open 2026-11-20 (found in G0b Task 12)  size: `S`  project: `G0B-GAME-AS-DATA`
- The no-game-names gate (`crates/sigil-cli/tests/no_game_names.rs`) scans the module trees of `sigil-cli`'s
  normal-dependency closure (15 crates, 162 files at Task 12). What it leaves open:
  1. **Five allow-list rows, exit G0c.** The 12 sound-seam literals in `sigil-harness/src/seam2.rs` (4 rows) and
     `seam1.rs` (1 row) stay until the sound seams go (owner Q3). Each row carries an exact hit count, so G0c's
     removal turns each row stale (red) and the row must be deleted with the code.
  2. **`sigil-harness` is both a library the driver links and the golden/test harness.** `test_support` (pub,
     not `cfg(test)`, ~3000 lines) is in the driver's closure only because integration tests in two crates need
     it; Task 12 moved its two game-naming pieces out by hand (a test-local const, a caller-named fixture game)
     and moved `freeze_journal` beside the `refreeze` binary (compiled by `#[path]` there and in the
     `freeze_step_gap` gate). A `test-support` cargo feature or a separate harness crate would make "not the
     driver's code" structural instead of a convention the gate polices.
  3. **Comments are not gated.** They are prose examples, not driver decisions. Since the G0b final-review fix wave
     (C-I1) the gate COUNTS them and its line says exactly what: comment lines naming a game (an id or a games path),
     test-only items left out, 29 doc (`///`, `//!`, `/** */`) and 14 plain (`//`, `/* */`) at that wave.
     Label vs code: `no_game_names.rs:269-274` (`StripTestOnly::keep_items`) leaves test-only code out of the
     plain-comment count only for whole items, though the label says test-only items are left out.
  4. **Output stems are not needles (C-I3 stem half, stopped by the ruling's bar).** Adding sonic4's `[outputs]`
     stem `s4` as a needle measured 5 new hits: `crates/sigil-harness/src/test_support.rs:520` `listing_path`'s
     `"s4.debug.lst"` / `"s4.lst"` (a real game fact in the linked library; deriving it from `[outputs]` would need
     a `games/sonic4` literal instead) and three in `pins.rs` (`S4LZ`, `S4_LZ_DECOMPRESS`, `S4_LZ_DECOMPRESS_DICT`:
     the engine's S4LZ codec, which no allow row can exempt honestly under the G0c-exit rule). Exit: G0c removes
     the `test_support` reference-listing helper with the seams' port gates, or a `test-support` feature (item 2)
     takes it out of the closure; then derive `[outputs]` stems into the fixture with a codec-name exemption.
  5. **Binaries outside the closure still name games** (G0b final review B-M13, deferred 221/236):
     `crates/sigil-harness/src/bin/cycle_fraction.rs:594` `if label != "sonic4 debug" { continue; }`. The gate does
     not reach `src/bin` by design (nothing the driver links declares it); a renamed row silently prints no detail
     section. Exit: key the detail on a shape the descriptions mark, or drop the filter.

## G0B-AS-FEATURE-DEFINES-INERT

- state: open 2026-11-20 (found in G0b Task 11; controller ruling in fix round 1)  size: `S`  project: `G0B-GAME-AS-DATA`
- aeon `engine/engine.toml` `as_feature_defines` has three rows (`SOUND_DRIVER_ENABLED` when `sound`, `__DEBUG__` when
  `debug`, `__MDDBG__` when `debug || crash_report`), read by `sigil_build::driver::as_feature_defines` and handed to the
  residual AS side (`games/<g>/game_root.asm`) as plain `-D`s. Ruled: rows and consumer stay exactly as they are; the
  table goes when `game_root.asm` goes.
- **The rows are byte-inert on all seven shipped shapes, and only a fixture unit test holds them**
  (`driver::tests::the_residual_as_feature_defines_follow_the_description`, a synthetic engine). Measured at 2cef7f08:
  narrowing `__MDDBG__` to `debug` left step0 `SHAPES 7/7 IDENTICAL`. So deleting a row from the REAL engine.toml, or
  mistyping its `when`, still passes 7/7, the goldens and the full suites: nothing that runs on a shipped tree reads
  what the residual AS did with the define.
- **A real witness** would hold the residual AS's symbol environment, not the bytes: for every shipped shape, assemble
  the residual AS side (`driver::as_side`) and assert that the definedness-gated arms it takes match the shape's
  features (e.g. the `ifdef __MDDBG__` include's `MDDBG__*` externs are present exactly on `debug || crash_report`
  shapes, the `SOUND_DRIVER_ENABLED`-gated names exactly on sound-on shapes), red-first by deleting each row from a
  tree copy's engine.toml. Or retire the rows by making the residual AS stop testing them (G0c-adjacent).

## G0B-COMPARE-IDENTITY-NAMES

- state: **closed** 2026-11-20, G0b Task 7 round 2 (found in Task 7)  size: `S`  project: `G0B-GAME-AS-DATA`
- **Ruled (proxy, Fable aae3c4f22e38f080a, owner may override): option (a), a narrow declared delta.** Implemented in
  `scripts/compare-drivers.sh`: the LEGACY listing and export are transformed into what the game-as-data driver must
  write (DIGEST-SHAPE `target=` -> the `[shapes.*]` row, checked against the copy's game.toml; exactly the two
  description DIGEST-READ rows, crc/size from the copy's files, inserted in digest order; aggregate recomputed; export
  `shape.shape` -> the row name, all else untouched) and compared byte for byte with the new artifacts; nothing is
  stripped from the new side. The delta is required (`MATCH (declared delta)`, `driver_comparison` accepts nothing
  else). Measured on aeon 559b8910: `SHAPES 10/10 MATCH`. After cutover the export shape name and DIGEST-SHAPE
  `target=` are the description's row name. The keystones fixture's pairing row is its own name; its ROM equals
  config-a's on both drivers, so it adds no separate byte evidence.
- Booked consumers (not implemented here):
  - **aeon `tools/gate_cut_shape.py`** treats `target == game` as "canonical build"; with `target=release` its rename
    guard would SILENTLY stop firing. Re-key it on (game, target), accepting both spellings until Task 10, in Task 8's
    paired aeon change. (`tools/leg_provenance.py` records `target` but needs no code change.) **DONE in Task 8**
    (aeon `feat/g0b-descriptions`, `canonical_build_ids`, red-first on 4 rename rows); dropping the old spelling at
    Task 10 is booked in aeon DEFERRED_WORK G0B-DEMO-CUTOVER-RESIDUE item 2, beside the six aeon tools that still
    build demo with `--game demo` (item 3).
  - **Oracle** re-pins `fixtures/g0a/s4.trim.build.json` and `crates/oracle-aether/tests/build_export_real.rs` at
    Task 10 (the export's `shape.shape` becomes the row name). **DONE in Task 10** (oracle `feat/g0b-shape-names`):
    `tools/g0a-real-exports.sh` builds the five shapes with `--project` from an aeon tree with descriptions, and the
    trimmed fixture, `SHA256SUMS` and `EXPORTS.normalized` are re-pinned from that build. The pins were ALREADY stale
    against sigil since the walk merge (87b5c195 moved every shape's bytes; oracle's pins are sigil 3f211b78's), so
    the re-pin carries both: every changed key is classified in the Task 10 report.
  - **aeon** dropped `gate_cut_shape`'s `(game, game)` spelling at Task 10 (aeon DEFERRED_WORK
    G0B-DEMO-CUTOVER-RESIDUE, closed).
  - **Task 11** checks whether any golden pins a listing (the DIGEST-SHAPE `target=` and read rows change at cutover).
- The text below is the finding as booked before the ruling.
- The comparison job (`scripts/compare-drivers.sh`, `crates/sigil-cli/tests/driver_comparison.rs`) builds all 10
  shapes with both drivers: ROM 10/10 identical, `.lst` 0/10, export 1/10 (lean). Every listing and export difference
  is one of exactly two kinds, on every shape (the job's `declared-delta` line checks this byte-exactly, diagnostic
  only):
  1. **The shape's name.** The legacy driver names a shape by its flag and its profile label: `DIGEST-SHAPE
     target=sonic4|demo` (plain and debug), export `shape.shape` = `sonic4 plain`, `sonic4 debug`, `demo plain`,
     `demo debug`, `config_a`, `config_b`, `lean`, `stress_evict`, `stress_art`. The game-as-data driver names it by
     the description: `target=` and `shape.shape` are the `[shapes.*]` row (`release`, `debug`, `config-a`, ...).
     They already agree for `target=` on config-a/config-b/lean/stress-* and for the export on `lean`.
     `config-a-keystones-flipped` cannot agree under any naming: the legacy driver has no target for it (its profile
     IS `config_a_profile()`, so `--config-a` is its only legacy spelling, named `config-a`/`config_a`).
  2. **The description files.** The new driver reads `engine/engine.toml` and `games/<g>/game.toml`, so its listing's
     read set has those two DIGEST-READ rows (and a different DIGEST-AGGREGATE). They are real inputs of that build;
     the export's `files` table is unchanged (it lists source files); `inputsSha256` (excluded) differs, as it hashes
     the shape record and the build's inputs.
- Nothing else differs: defines, every other read row, the symbol body, and the export's files, symbols, layouts,
  consts and lines are identical. So the spec's "identical in bytes and in G0a's export" holds for the bytes and
  cannot hold for the listing digest or `shape.shape` as the two drivers are built. G0a's spec already expected the
  name to change source ("`shape.shape` ... today's profile name, later G0b's description").
- Options for the ruling (not taken by the implementer): (a) the comparison expects exactly this declared delta (the
  `declared-delta` check promoted to the verdict: name from the pairing row, the two description rows with their
  hashes, the aggregate recomputed), nothing else tolerated; (b) the legacy driver adopts the description's names now
  (moves DIGEST-SHAPE/`shape.shape` for consumers: aeon `tools/leg_provenance.py` records `target`, Oracle's
  `build_export_real.rs` pins exports) and the comparison tolerates only the description reads; (c) another naming.
  The keystones fixture needs (a) or a ruling that its name is the pairing.
- Found with it: **the plan's red-first mutation is byte-inert on today's tree.** Dropping `ObjCodeBase`'s
  `[[align]]` (engine/engine.toml) leaves every ROM identical: both maps anchor it at 0x10000 (`holds =
  "ObjCodeBase"`). Dropping `Sfx_33`'s (games/sonic4/game.toml) is inert too: its packed base is already 8-aligned.
  An `[[align]]` row moves bytes only for a section head that is not anchored and not already aligned. Task 7's
  red-first used a planted exception instead (`DemoBox_Main` align 0x100 in games/demo/game.toml: demo and demo
  debug ROMs DIFFER).

## TEST-TEMPDIR-PID-NANOS-COLLISION

- state: **open** 2026-11-20 (found in G0b Task 8's strict landing)  size: `S`
- The strict landing on ddea73f0 went red on `deny_todo_leaves_unreachable_alone`. Its stderr named
  `/tmp/sigil_deny_todo_1127877_1795184328353958724/m.emp` holding a todo test's source
  (`[todo.present] todo!: finish the brain`). The three tests run on parallel threads of one process and named
  their directories `<pid>_<as_nanos()>` only, so two threads drew the same name and one overwrote the other's
  `m.emp`. Forcing the stamp equal (`as_nanos() * 0`) makes the original code red 5 runs out of 5 and the fix
  green 5 out of 5. **`crates/sigil-cli/tests/deny_todo.rs` is fixed** (a per-process `AtomicU64` in the name,
  G0b Task 8).
- **Still open:** the same `pid + as_nanos()` naming is used by other test files, unexamined for
  parallel-thread sharing: `crates/sigil-cli/tests/{dac_bank_acceptance,end_to_end,pitcher_plant_script_acceptance,
  pitcher_plant_acceptance,sigil_test_runner,tranche0_acceptance}.rs`,
  `crates/sigil-frontend-as/tests/{as_interp_radix,as_warning_exitm,as_odd_address}.rs`,
  `crates/sigil-harness/tests/{suite_paths_precedence,reference_tree_write_guard,reference_tree_named_write}.rs`.
  A shared helper (pid + counter, or `tempfile`) would close the class.

## G0B-EXPORT-BY-ROOTS: CLOSED

- state: **CLOSED** 2026-11-21, G0b Task 13a (found in G0b Task 13, Cherub)  size: `S`  project: `G0B-GAME-AS-DATA`
- **Found:** `sigil build --project <cherub> -o ...` refused the build export for a game in its own repository:
  "`.../cherub/game_root.asm` is a source file the build read from outside the aeon tree ... so no export root holds
  it". `export_files::place_path(aeon, sigil, path)` assigned export roots by aeon-checkout prefixes
  (`<aeon>/engine/`, `<aeon>/games/`) instead of by the build's own `SourceRoots`, and `source_row_key` (the line
  table's file key) had the same shape. The ROM built and booted; only the export was blocked.
- **Fixed (proxy ruling, Fable 2026-11-21, owner may override):** one rule for every game, a `files` row is (root
  name, path relative to that root's directory in this build). `export_files`/`source_row_key`/`place_path` take
  `ExportRoots` (the build's `SourceRoots`: `engine` = the engine root, `game` = the project directory holding
  `game.toml`, NOT `sources_root()`, `generated` = the build's generated directory; plus the sigil checkout and,
  for hash-only `aeon` data rows, the engine's checkout). Longest root wins. A source file under no root is refused
  naming each root and its directory. Spelling change for aeon's games: `game sonic4/config/header.emp` is now
  `game config/header.emp`; `shape.game` names the game. Contract version stays 1.0 (empyrean `protocol.md`).
- **The generated mount is kept, by decision (Task 13a report):** under the engine root, `<sub>/generated/<rest>` is
  still the `generated` row `<sub>/<rest>`, the mount `mount_generated` and the `.lst` digest already state. aeon's
  build.sh writes `engine/debug/generated/compression_vectors.emp` into the engine checkout, every shape reads it, and
  every export has carried it as `generated debug/compression_vectors.emp`. Placing it literally (engine
  `debug/generated/compression_vectors.emp`) would reorder `files` and shift every later file index, which the
  ruling's proof (only the game prefix moves) forbids. The mount now applies to any `<sub>`, as
  `GeneratedRoot::mounted` does, instead of the literal `sound`/`debug` pair; no aeon read changes row by it.
- **Ratified (controller ruling, G0b Task 13/13a review I1, 2026-11-21):** the mount stands as code; the contract
  text now states it (empyrean `b7717eca`, §11.55 and the schema's `files[].root` description: a `generated` row is a
  file in the build's generated directory or one under `<engine>/<sub>/generated/`, placed as `generated`
  `<sub>/<rest>`). Dropping the mount is a later contract change and costs one re-pin of file order in every export
  that carries a mounted row (aeon's debug shapes' `compression_vectors.emp`, and every file index after it).
- **Proof:** `scripts/export_roots_cmp.sh` (old 02b8a8f4 vs new, 7 shapes: old export with `<id>/` stripped from
  every game row equals the new export except `inputsSha256` and `tool.revision`), output committed under
  `docs/evidence/g0b-export-by-roots/`. `build_export_binding::an_out_of_tree_games_export_rows_are_its_listing_rows`
  builds `games/demo` copied outside the checkout and holds each export `(root, path)` to its listing row.

## G0B-LISTING-ROOT-SPELLING

- state: open 2026-11-21 (found in G0b Task 13a)  size: `S`  project: `G0B-GAME-AS-DATA`
- The `.lst` source digest still has two spellings for one file, left alone by ruling in Task 13a: a read inside the
  engine's checkout is written tree-relative with no `root=` (`path=engine/vdp.emp`,
  `path=games/sonic4/config/header.emp`, `origin=generated path=engine/debug/generated/...`), a read outside it under
  its named root (`root=game path=config/header.emp`). The build export now has one spelling for both (root plus
  root-relative path). So an in-tree game's listing and export name the same file differently, and a consumer joining
  the two must know the tree layout. `an_out_of_tree_games_export_rows_are_its_listing_rows` states the one mapping
  (`engine/<sub>/generated/<rest>` is generated `<sub>/<rest>`, other `engine/<p>` is engine `<p>`). Fix: write every
  digest row under its named root, which is a `.lst` text change for every aeon shape (its own proof, and aeon's
  `.lst` readers re-checked), or retire the digest's tree spelling with the transitional tree
  (`G0B-UNROOTED-TREE-READS`).
- Also (Task 13/13a review, minor 1): the listing digest's `game` root is `project.sources_root()` while the export's
  is `project.game_dir` (`sigil-cli/src/main.rs` ~3247-3248), so for a game whose `game.toml` sets `sources.root` to
  anything but `"."` the listing's `root=game path=` and the export's `game` row name the same file relative to
  different directories; one root must win when the spellings are unified. A further effect (G0b final review
  B-M15): with `sources.root != "."` on an out-of-tree game, `game.toml`, `map.toml` and `baseline.toml` sit under
  no named root, so by `source_digest.rs`'s placement order they are written `root=/` with an absolute path, which
  puts a machine path in the listing. No game does this today (all three use `"."`).

## G0B-BLIND-REFUSAL-UNNAMED

- state: open 2026-11-21 (found in G0b Task 13, Cherub; booked in Task 13a)  size: `S`  project: `G0B-GAME-AS-DATA`
- A game missing the defines `HAS_ACT_ART_POOL` or `VRAM_RING_PLACEHOLDER` gets only the contract closure's generic
  "analysis is blind" refusal ("1 comptime condition(s) did not resolve", "the contract closure DROPPED 1
  instruction(s)"), which names the module or proc but not the symbol. The unresolved-condition report HOLDS a `Span`
  (`sigil-frontend-emp/src/corpus_contracts.rs` ~216, `comptime_unresolved: Vec<(String, String, Span)>`) and
  `sigil-cli/src/main.rs` ~2268-2287 prints the first two fields and discards it; the dropped-instruction report
  (`dropped_by_proc: Vec<(String, usize)>`, ~201) keeps only proc and count, so naming the symbol there needs the span
  recorded first. Fix: render file:line and the unresolved name in both refusals. Pre-G0b gate code with pinned
  message text, so not landing work for G0b.

## G0C-SOUND-SEAM-GAME-IDENTITY

- state: open 2026-11-21 (G0b final review A-I3, controller ruling: BOOK)  size: `S`  project: `G0B-GAME-AS-DATA`
- `Seams::emit_sound` (and `validate_sound`) carry no game identity: the harness seams (`seam1.rs`, `seam2.rs`,
  `import_check::seam_roots`) serve sonic4's sound data by literal path (the five G0c allow-list rows in
  `tests/no_game_names.rs`). A second game with `sound = true` would be fed sonic4's banks, or fail late in sonic4's
  terms at whichever check trips first (`source_digest.rs`'s stray-scan refusal, a seam path), never by a refusal
  that names the seam (seat B Minor 14 is the same finding from the harness side).
- **G0c entry criterion:** a second game with `sound = true` either builds from its own descriptions, or is refused
  up front by a named refusal saying the sound seams serve only the game they were written for. The fix wave judged
  the up-front refusal over the ruling's 30-line bar (it needs a game identity on the `Seams` trait: the signature,
  its three driver call sites, both impls, and a test that forges a sound-on shape for a game the seams do not
  serve), so it is booked rather than added. G0c should start from that refusal as its red test.

## G0B-FINAL-REVIEW-RESIDUE

- state: open 2026-11-21 (G0b final review fix wave)  size: `M`  project: `G0B-GAME-AS-DATA`
- The four final-review seats' Minor findings and deferred-triage lines that the fix wave neither fixed nor saw
  dropped by the reviewer. Each cites its report id (`final-review-<seat>.md`, Minor n, or the deferred ledger
  line); none moves a ROM byte today. The fix-wave report's table holds the full disposition.
  - **A-M2** `describe.rs` `UnknownShape`, `MissingFeatures`, `ExtendsCycle`, `DerivedDefine`, `UndeclaredShape`,
    `BadAlign` carry no file path, and `UnknownShape` does not list the declared shapes (`no shape named `relase``).
  - **A-M4** `DERIVED_DEFINES` (`describe.rs:37`) spells `__DEBUG__` / `__MDDBG__` in Rust although `engine.toml`
    `as_feature_defines` now produces them; the rule that follows the data is "a define may not use a name an
    `as_feature_defines` row produces".
  - **A-M5** `driver.rs` `declares_island` restates `debug || crash_report` beside the engine gate rows.
  - **A-M6 / B-I1 note** `entry_seeds` is positional (contract, then RAM; `seeds[2]` reused by the RAM harvest).
    Named keys would be self-describing; FORMAT-bump question, so not in the fix wave.
  - **A-M7 / deferred 103** `walk.rs:168` an anchor whose `holds` matches more than one ROM section binds the first
    silently; refuse `hit.len() > 1`.
  - **A-M8** `align.rs` an `[[align]]` row whose label heads no section is silently inert.
  - **A-M10 rest** see `G0B-TASK11-RESIDUE` item 4 (corrected).
  - **A-M13** a residual-AS failure prints the first diagnostic in `{:?}` form, not rendered.
  - **A-M14 / deferred 229** the sound-emission block is written three times in `driver.rs` (G0c, with the seams).
  - **A-M15** engine and game trees scanned and parsed three times per build.
  - **A-M16** `build_dir::check_outside` checks only the engine checkout, not an out-of-tree game's own directory.
  - **A-M17** `--extra-entry <path>` is tried against the engine checkout and cwd, never the game directory.
  - **A-M18** the engine is found by walking every ancestor up to `/`; bound it at the enclosing repository root.
  - **A-M19** `driver_units.rs` modules all `#[cfg(test)]` behind `pub use` globs (harmless).
  - **A deferred 91** broken rustdoc links in `harvest.rs` (no `cargo doc` run); **deferred 101/102/221**
    `AsSymbols` in the public API, `Seams` returning `Box<dyn Any>` (also **B-M20**), `allow(too_many_arguments)`
    on `validate_sound`: all G0c, with the seams. **Deferred 128** `BuildDir` cleaned only on drop (with
    `G0B-TASK11-RESIDUE` item 2).
  - **B-M3** `Artifacts.output` is always `Some` (dead `None` arms); `BuildIdentity.target` always equals the shape.
  - **B-M7 rest** `section_extents_of`, `section_label_owners_of`, `phase_bank_lmas_of` return `Result` and cannot
    fail (text half fixed in the wave).
  - **B-M12** committed `pins.rs` header says "sigil-native canonical resolve" while the generator now writes the
    `sigil build --project` provenance; `pins.rs` is a generated, refreeze-listed file, so it is not hand-edited:
    landing step (3) `--repin-aeon` regenerates it.
  - **B-M16 / C deferred** `shipped_shapes.rs` set pin is reference-dependent (declared, strict-gated).
  - **B-M17 / deferred 225** `sound_fold_heads.rs` re-implements the seam's head selection (G0c).
  - **B-M18, C-M10, deferred 237 / Task 11** hand-listed games or shapes not held to the shipped set:
    `listing_defines.rs` `shape_pairs`, `build_export_layouts_crosscheck.rs` `seven()` with its copied
    `census.len() == 7`, `game_contract_env_coverage.rs` `shipped()`, `installed_binary_needs_no_build_tree.rs:37`,
    `lst_source_digest.rs:37`.
  - **B-M19** the export's hash-only root name is the literal `aeon` (pre-existing G0a format).
  - **B-M21** test and message names still say "profile" (`listing_defines.rs`, `listing_equ_shape_aware.rs`,
    `sound_fold_heads.rs`, `sound_bank_id_check.rs`; "plain" in `m68k_roundtrip_stream.rs:190-205`).
  - **B deferred 175 M6** `seam_roots` bound to sonic4 (G0c; `G0B-NO-GAME-NAMES-RESIDUE` item 1);
    **175 M7** new `root=` tokens under FORMAT 1 (noted in the closed `G0B-DIGEST-OUT-OF-TREE-ROOTS`);
    **229** `SOUND_GAME_DESC_REL` literal (G0c, allow row); **246/247** `freeze_journal` `#[path]` and module-wide
    `allow(dead_code)` (`G0B-NO-GAME-NAMES-RESIDUE` item 2).
  - **B deferred 236, C deferred Task 11 → 12** no red-first for `assert_lists_every_shipped_shape` (five gates lean
    on it) or the `golden_write_gate` stub `-*)` arm.
  - **C-M1** `.emp` and AS disagree on a `..` inside a root (`sandbox.rs` joins the unfolded `Located::Root.rel`);
    fold `rel` in `inside()`.
  - **C-M2** `resolve_rooted_path` swallows two canonicalize errors, skipping the escape check; refuse when it
    cannot decide.
  - **C-M4** `LEGACY_SOUND_MOUNT` in `sigil-span` stays for the `[root.stale-generated]` refusal until G0c.
  - **C-M11** `build_check.rs:108` `census()` accepts either game's `checked:` line; pass the game in.
  - **C-M12** `p5_constants_flip.rs` renders any `shapes::as_inputs` error as `skip:` outside strict mode.
  - **C-M13 / C deferred Task 6** `build_hermetic.rs` tracks files only (an empty directory a build creates is
    unseen), and its identical-bytes "rewrote" arm has never been shown red.
  - **C-M15** new scratch-tree writers under `CARGO_TARGET_TMPDIR` (`extra_entry.rs:91` `probe_tree`, never
    removed; `build_check.rs`, removed only on success): adds to `G0B-TASK11-RESIDUE` item 2.
  - **C deferred Task 9 M3** symlink-escape check exists only on the `.emp` side; the AS side goes with the residual
    AS (`G0B-UNROOTED-TREE-READS`).

## EMP-CONST-EQU-SAME-NAME

- state: open 2026-11-21 (A10, booked by Task 2)  size: `S`  project: `STEP0-A10`
- A `const` and an `equ` of one name in one module build, and the `const` wins for a comptime read:
  `const Foo = 1` / `equ Foo = 2` / `moveq #Foo, d0` gives `70 01` (measured at `1e200e1b`). A10 refuses
  `const` against `const` only (`[name.duplicate-const]`, parser `refuse_duplicate_consts`); `equ` twice is
  already a link error (``symbol `Foo` redefined``); the mixed pair is this entry, and is pinned unrefused by
  `crates/sigil-frontend-emp/tests/name_duplicate_const.rs` `a_const_beside_an_equ_of_the_same_name_is_not_this_refusal`.
- The same last-wins shape exists for repeated `data`/`table` attributes and `context` fields (language review A,
  stage I1 row 8).

## EMP-SPLICED-INT-OPERAND

- state: open 2026-11-21 (A10, booked by Task 3)  size: `S`  project: `STEP0-A10`
- `move.w {n}, d0` with an integer `n` still assembles as an immediate with no `#` in the template; refused
  nowhere, by decision in the A10 plan (Open question 2: a spliced integer is the macro idiom). A10 refuses only
  a number written in a 68000 operand position (`[operand.bare-number]`, `eval/asm.rs` `map_plain`); the
  `{splice}` arm reaches `classify_operand_splice` directly and keeps the immediate, pinned by
  `crates/sigil-frontend-emp/tests/operand_bare_number.rs` `a_spliced_integer_stays_an_immediate`.
- Count aeon's uses before changing it.
