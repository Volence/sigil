# 68k Cfg::edges trailing-label unification (2026-08-06)

Status: RULED (Fable). Closes ledger row 2103 (the tail-spelling family's 68k
builder instance). Scout ground truth 2026-08-06: `Cfg::edges`'
unconditional arm (flag_check.rs:314-320) maps every `label_target` miss to
`Edge::Defer`, so a trailing local label (a label that closes the body) and a
genuine external symbol are indistinguishable; the CONDITIONAL taken edge
(flag_check.rs:332-345) has the same miss; `z80_branch_edge`
(flag_check.rs:428-437) already does the correct three-way via
`is_local_label`; the builder's own doc (flag_check.rs:300-307) promises "the
`Return`/`FallOff` choice is made in an edge BUILDER and nowhere else" and the
trailing-local case breaks that promise; two narrow consumer shims exist
(check_noreturn's `is_trailing_local`, proc.rs:344-346; ccr_bracket_refusal's
`label_index` gate, proc.rs:1972-2028) plus a third reader in the dispatch
check (cycle_budget.rs:576-587); the corpus has exactly TWO 68k sites
(buffers.emp:57 `jbra .done`; frames.emp:67 `jbra .rpc_done`).

## 1 · The fix — one shared branch-edge resolution, both CPUs

Extract the target-resolution three-way into ONE helper (it is CPU-independent
— `label_target` hit → `Follow(tgt)`; miss but `is_local_label` → `FallOff`;
else `Defer`) and route BOTH builders' unconditional arms AND the 68k
conditional taken edge through it. `z80_branch_edge` becomes a caller of (or
is renamed to) the shared helper — do not leave two spellings of the same
resolution alive; that is this defect family's breeding ground (rows 1882,
2150 are the same family one layer out).

The builder doc's promise becomes true again; extend it with the trailing-
label sentence so the next reader knows the case is deliberate.

## 2 · Blast radius — audited per consumer, measured on the corpus

The scout's consumer census is the checklist. For each, the porter states the
expected behavior change, then MEASURES corpus diagnostics across all seven
shapes pre/post (the two corpus sites are the live subjects):

1. **preserves + stack balance** (preserves.rs:464-472, 1099-1102): a
   trailing-local transfer becomes a body-ending exit (`ends_this_body` true)
   — an obligation site for register-preserves and a charged exit for balance
   under its `charge_fall_off_end` policy. This is CORRECT — it was the lie.
   If either corpus site newly fires, read the proc: the firing is either a
   real latent defect (report as a catch, fix in aeon honestly) or a missing
   `falls_into` policy in the consumer — adjudicate in the packet, do not
   suppress.
2. **out_verify** (out_verify.rs:289-357): trailing transfer stops routing
   through the `is_uncond_tail` Defer re-check and goes straight to
   `check_return`. Note but do NOT take row 2147's `Edge::TailOut` split.
3. **cycle_budget** (cycle_budget.rs:659-786): trailing transfer refuses as
   FallOff instead of Defer — the diagnostic is IDENTICAL either way (both
   arms funnel to `UnboundedTransfer` before any cost accumulates, so a
   FallOff-classified trailing transfer can never yield a wrong total). The
   `divergent_terminal` escape gates on Defer arms only; its non-application
   to trailing shapes holds because the AssertDesugar×trailing-local
   intersection is empty on the corpus (rails jmp to external divergent
   blobs) — state that in the census, since the arms do not structurally
   exclude it. Confirm Process_DMA_Critical's @budget is unmoved.
   [Amended per t-edges Lens C: the original text predicted a message change
   that does not occur.]
4. **flag_check::abandons_flag** (flag_check.rs:854-874): FallOff reads
   "abandoned" where Defer read "flows out" — `[call.flag-result-unused]` can
   newly fire. buffers.emp's proc is a carry-out site (`andi/ori #_,ccr`
   around the branch): measure it specifically. A false fire on a
   `falls_into` proc means abandons_flag needs the falls_into consult the
   edge-split packet noted no consumer states — if so, state it THERE
   (flags flow to the declared successor), narrowly, with its own pin.
5. **enumerated dispatch** (cycle_budget.rs:556-643): the `[Edge::Defer]`
   computed-transfer gate no longer sees trailing-label transfers; the
   `DispatchFindingKind::Trailing` refusal (b8 reading) stays — it polices
   `targets(...)` author claims, a separate axis — but re-derive it through
   the unified builder if that deletes code rather than adds.
6. **context** (context.rs:346-372): all three non-Follow edges escape alike —
   expect zero change; confirm by census.
7. **branch_const / type_slice / calls**: Follow-only consumers — no change;
   say so once, no per-file ceremony.

## 3 · Shim retirement

The b8 narrow fixes retire where the builder now answers them:
- `check_noreturn::is_trailing_local` (proc.rs:344-346) — DELETE; the
  `Edge::Defer if is_trailing_local` arm collapses into the FallOff arm
  (falls_into∘noreturn composition unchanged). Its two pinned tests
  (`noreturn_trailing_local_transfer_is_a_fall_off`,
  `ccr_trailing_local_transfer_is_a_leave`) MUST survive verbatim — they pin
  behavior, not mechanism.
- `ccr_bracket_refusal`'s trailing-label gate: this walk reads `label_index`
  directly, not `edges()` — audit whether the unified builder can serve it;
  if the shim stays because the walk is not edge-based, KEEP it and say why in
  its comment (present tense), and record nothing — a reader-level gate that
  cannot drift from the builder is not a defect.
- Row 1855's kill condition ("next parcel that touches the shared CFG fixes
  the classifier") — this parcel touches flag_check.rs: re-read the row; the
  `!= "bsr"` exclusion shipped in B′-2, so if the row's demand is already
  satisfied, CLOSE it with the citation; if it asks for more (the ISA-crate
  classifier of row 2150), leave 2150 open and close 1855 against B′-2.

## 4 · What this parcel must NOT do

- No new Edge variants (row 2147 stays OPEN; row 2109 is precedent that a
  variant split is its own parcel).
- No mnemonic-classifier move to the ISA crate (row 2150 stays OPEN).
- No preserves-through-tail credit and no falls_into threading into the
  register-preserves walk — the t-credit lane owns that and rebases over this
  merge. If §2.1's measurement shows a corpus firing that WANTS the credit,
  record it for t-credit, do not build it here.

## 5 · Bars

Byte bar seven targets identical (builder + analyses only — the two corpus
sites' machine code is untouched). Full strict with closing arithmetic;
refreeze --check chain 48; repin unchanged; warn tiers id-identical ×7 unless
§2.4 produces a deliberate, named delta (baseline updated same parcel, delta
named in the packet). Tests: builder unit pin for the 68k trailing-local
three-way (mirror of `a_jump_to_a_closing_label_falls_off_it_does_not_
transfer_out`, flag_check.rs:1028) covering unconditional AND conditional-
taken edges; the two b8 behavior pins survive; per-consumer corpus census in
the packet (diagnostics identical ×7 except named deltas). Ledger: row 2103
CLOSED; row 1855 per §3; anything §2 uncovers appended to standing rows.

## 6 · Merge position

FIRST in the tail-seams queue — t-credit and t-invoke rebase over this merge
and re-measure; neither may special-case trailing labels while waiting.
