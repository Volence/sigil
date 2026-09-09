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

- state at archive: `next`  size: `L`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: the owner: size L sent for his eye, and no owner decision exists on the item itself. A hub go is not his go.

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

## EMP-Z80-MNEMONIC-TABLE

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

## STABILITY-RUNNER-MISSING-WHERE-CLAIMED

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

Sweep owed, now with a named population: five notes from the same week state their ground truth as the UNRELIABLE copy cited by version banner, with no committed probe directory to re-run.

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
