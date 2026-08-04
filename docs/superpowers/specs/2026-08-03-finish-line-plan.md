# The finish-line plan — every remaining Spec-2-era work item, specced or parked

**Fable, 2026-08-03. For Volence's ratification, then porter execution.** Scope:
Volence's ask — "plan out the rest of the work with full specs for a lesser model
to finish what we stated and all the deferred work." Companion specs:
`2026-08-03-contract-unification-spec.md` (Track B, full) and
`2026-08-03-niche-option-spec.md` (Track C, full).

**The park doctrine (read first).** Half the deferred ledger is demand-gated by
its own ratified rulings (S2-D14(a)'s "3+ floating bank sections" gate, S2-D8's
content gate, S2-D15's re-open bar…). Those items are NOT specced here — a spec
written before the demand moment bakes in a shape the demand will contradict
(the campaign proved this repeatedly: pass-3 parcels kept dissolving under nets).
Each is PARKED in §4 with a wake condition and a who-specs-it. Only Volence
re-opens a park early.

**Standing bars for every parcel below** (the porter loop, unchanged): byte-neutral
×6 unless the parcel says otherwise (there are no byte-changing parcels in this
plan); strict suite green with explicit counts, failures-first; `repin --check` /
`refreeze --check` clean; no re-freeze; provenance = CRC32+size; era lens panel
(A/B/C) on every merged batch per the 2026-08-03 ruling; overseer countersigns
own-run; sequential merge queue, `origin/master` precondition.

---

## §1 — Track 0: land the round (overseer, no new specs)

Merge the checkpointed round parcels in their packet order: L1 P1 → L1 P2 →
L5+L8 → L9 → the one-sitting batch (L3/L4/L12 items). All close packets exist in
`notes/`; each is gate-green on its branch pair. This precedes Track B (the
contract spec's P1 touches lowering; merging under it is asking for conflicts).

## §2 — Track B: the contract unification (THE mountain)

Full spec: `2026-08-03-contract-unification-spec.md` — expanded to porter grade,
all six open questions resolved (my calls, flagged for your gate). Five parcels
P1–P5 + a follow-on batch F (S2-D6(c) reg aliases, S2-D6(d) scratch splices).
Absorbs S2-D6 and S2-D7 whole; closes the D2.35 auto-inc/dec ledger row inside
P1. Sequencing: P1 strict-first; then P2 ∥ P3; then P4; then P5; F last, gated
on demonstrated splice demand (may park at close — recorded either way).

## §3 — Track C + D: the small buildable set

- **C — niche-sentinel Option** (full spec: `2026-08-03-niche-option-spec.md`).
  Two slices: C1 (distinctness, buildable immediately after Track 0, independent
  of Track B) and C2 (the guard-dominance check, RIDES B-P4's CCR machinery —
  spec'd in the same doc, built after P4).
- **D1 — `[operand.const-as-address]` lint** (S2-D12(e), the forgotten-`#` bug).
  One-sitter. A memory-safety catch in the same spirit as this whole era: an
  operand naming a `const` in address position (absolute addressing of a
  non-address constant) warns unless the const is address-typed (`*T` / section
  label / `equ`-of-label). Warn-tier, `@allow`-suppressible, `@as_compat`-aware
  (ported AS code does this deliberately in known idioms — the lint fires on
  new-style files only, first slice). Tests: the classic `move.w RINGS_MAX, d0`
  vs `move.w #RINGS_MAX, d0` pair + a true address-const negative.
- **D2 — same-name `offsets`+`dispatch` collision diagnostic** (S2-D12(d)).
  One-sitter: the ordinal lookup silently prefers offsets today; make the
  same-module same-name pair a loud `[table.name-collision]` error at
  declaration, with the link-collision test promoted from incidental to pinned.
- **D3 — shadow lints pair** (S2-D13(i) + (n)): comptime fn shadowing a
  mnemonic; local data item shadowing an import. Both warn-tier one-sitters,
  shared "shadow" diagnostic family `[name.shadows-mnemonic]` /
  `[name.shadows-import]`.
- **D4 — the brace-lexer extraction** (S2-D13(r)): quality-only, parity is
  already verified; extract the shared guard-message scanner. No behavior
  change; the parity tests are the gate.
- **D5 — S2-D17 CLOSE-OUT RULING: demote `patch`/`bind`.** The row's own
  condition has FIRED: the campaign ported the whole tree (done 2026-08-01) and
  found zero consumers; the motivating idiom dissolved in the Appendix-A rewrite.
  Per the row, that outcome = demotion from the §10 inventory at Spec-2 close, "a
  recorded decision either way." Parcel: spec-repo commit removing `patch`/`bind`
  from the §10 surface inventory + §6.4 re-headed as a design-history appendix;
  `lower/patch.rs` and its standalone tests STAY (shipped mechanism, zero
  surface — deleting code is not required by the ruling and the mechanism is the
  natural substrate if a consumer ever appears). Volence's nod required (it is a
  ratified-either-way decision, but it is still a decision).

- **D6 — the parser deep-loop hang** (t25 lane, gap-ledger ~:1619). A parse
  error in operand/expression position inside an `asm` body can INFINITE-LOOP
  the parser in accumulated context; repro committed + `#[ignore]`d
  (`crates/sigil-frontend-emp/tests/parser_recovery_hang.rs`); the statement-
  loop zero-progress guard shipped but the deeper operand-position loop
  remains. A frontend must error loudly, never spin. Kill condition is the
  ledger's own: `parse_str` returns a clean error → un-ignore the test.
  Porter brief: extend the zero-progress-guard pattern down the
  operand/expression recursion (every loop gets a forced-advance floor);
  minimization notes in the ledger row. This is a BUG, not polish — it
  outranks D1–D4 in the batch order.

D1–D6 are one Opus sitting each; bundle as one branch pair ("the finish-line
one-sitting batch") mirroring the L3/L4/L12 batch shape, D6 first. D5 is a
spec-repo-only commit riding any merge window — **Volence ratified D5
2026-08-03** ("Sure"); it can land at the next window.

## §3b — Track T: the round's tooling remainder (agenda "Tooling" section)

Carried over from the ratified round agenda — BUILD items my first draft of
this plan failed to absorb (they are not language work, hence separate track;
all porter one-sitters):

- **T4 — phase-aware repin (S).** Per the agenda ruling: BUILD.
- **T1 — RAM-map report (S).** Per the agenda: BUILD, "nice." NOTE the
  dependency direction: contract-spec §7's `--report contracts` says it rides
  the report machinery T1 establishes — so T1 lands BEFORE or WITH B-P1, or
  P1 ships its report self-contained and T1 conforms to it after. Porter of
  whichever goes second conforms to whichever went first; no redesign.
- **T2 — parametric `emulator_memory_hash(addr,len)` (oracle repo).** First
  CONFIRM against the oracle tree — the MCP tool surface already lists
  `emulator_memory_hash`, so the gap may be partly or wholly closed; verify
  the parametric form works and is trustworthy (one deliberate-corruption
  probe), then build only what is missing. The §17 identity bars' "single
  highest-value tool."
- T3/T5/T6/T7 rode the modernization+lens sweep per the agenda; confirm at
  Spec-2 close review that the sweep actually discharged them, else they
  reappear as one-sitters.

## §3c — Track R: the POST-TWIN-RETIREMENT dividends (now UNBLOCKED)

The gap ledger's `POST-TWIN-RETIREMENT` bucket was gated on the `.asm` twins
being gone; the K capstone + L1 (once merged) fire that precondition. These
are Volence-ruled work, so they are SCHEDULED, not parked — but they are
corpus/engine-era in character, so they slot AFTER Spec-2 close as the next
era's opening arc (matching Volence's 2026-08-03 framing: engine work
surfaces its own demands):

- **R1 — the own `.emp`-native diagnostics runtime** (Volence-ruled
  2026-07-28; "the largest single retirement dividend"): native symbol-table
  emission at link (kills the convsym deb2 appendix + its allowlist
  ceremony), the diag construct sheds the FSTRING format mirror (frees kill
  rows 21/53), handler sized to the used surface (register dump + symbolized
  PC + backtrace), DEBUGGER__* config gets its `.emp`-era home. Needs its own
  Fable spec — the R-arc headliner.
- **R2 — the full-corpus per-file retrospect** (Volence-ruled 2026-07-29):
  every `.emp` file vs the FINAL step-2 checklist + contract conventions,
  one file at a time, byte-neutral expected; absorbs B5 (contract-comma
  normalization ~40 procs), B6/B7 + the ~40-site codename backlog (comment
  hygiene), and the era-panel-1 doc-truth nits (macros.asm dead refs,
  soundbankhead VMA literals, MAX_RING_BUFFER row qualifier). NOTE: run R2
  AFTER Track B lands — the retrospect's contract re-derivation step should
  check against the FINISHED contract system, not the pre-unification one.
- **R3 — children.emp module split** (the four `@scaffolding` zero-caller
  procs + the `emit_piece_loop` templating, Volence's keep-and-mark ruling).
- **R4 — the priority-band inheritance re-ruling** + its parked riders (the
  hoist, the FlipAware fold, the −128-margin mitigation) — an ENGINE design
  decision (three candidate shapes recorded in the ledger, t24 Volence
  ruling); explicitly the engine era's first design ruling, not sigil work.
  Listed here only so the bucket has one home.

## §4 — Track P: the park ledger (wake conditions, no specs)

| Item | Parked by | Wake condition | Who specs on wake |
|---|---|---|---|
| S2-D2 generator absorption (`ojz_*`, collision import, sfx transcode) | D2.3 | opportunistic; a porter touching the adjacent data path | porter, brief inline |
| S2-D4 terseness sugar (+S2-D16(e) tail-call/auto-moveq) | tenet 5 | post-release author feedback | Fable |
| S2-D5 onboarding/migration docs | D2.4 | Spec-2 close + public-release decision | Fable |
| S2-D8 dimensional types (T2 newtypes: Coord/Velocity, Tile/Block/Chunk) | 2026-07-23 tier ruling | A4-i / first content-era physics work | Fable |
| S2-D9 hot-swap IRQ handlers + SMC slots | Plan-7 T2-e | a demanding port (content era) | Fable |
| S2-D10 engine-architecture constructs (phase split, VBlank queue type) | Plan-7 T2-f/h | content era; pairs with S2-D9 | Fable |
| S2-D12(a) cross-module reverse ordinals; (b) dispatch knobs incl. **`code_word`**; (c) overlay knobs; (f) shared emission core | seam-gated | (b) WAKES LOUD at the first scripted-object port on real Aeon — `code_word` is REQUIRED before it (Volence, pre-freeze audit); others on seam touch | Fable for (b); porter-brief for the rest |
| S2-D13 residuals: (a) jbcc trampolines; (b) Z80 jr→jp; (c) ladder unification; (d) width lint; (g) guard re-interpolation; (h) comptime-fn param defaults + `..` rest-fill; (j)(k)(l)(m)(o)(p)(q) | demonstrated-need / seam-gated | each on its recorded signal (jbcc = demonstrated need only, restates D2.18) | porter-brief on touch |
| S2-D14(a) packing linker | re-affirmed gate ×2 | 3+ floating bank sections OR a real fit failure | Fable |
| S2-D14(b) mapper/SRAM banking | demanding port | a mapper-using port (classic-game era) | Fable |
| S2-D14(f)(g) polish rows | polish-tier | seam touch | porter |
| S2-D15 structured control flow | recorded NO | a concrete authored new-style file proving it earns its keep; new-style-only forever | Fable + Volence ruling |
| S2-D16(a) charmap layer | surface ratified (D2.16) | first text-table port | porter (surface exists) |
| S2-D16(b) decompression builtins | no consumer | a legacy-asset-migration consumer | porter (vendored decoders make it cheap) |
| S2-D16(c) align extensions + chained-base rounding | audit gate | `[layout.odd-item]` firing constantly in new-style authoring; rounding needs a ratification nod (byte-layout) | Fable |
| S2-D16(d) versioned record emission; (f) SoA/bit-packed/format-DSL; (h) vector-table construct | research tier | own demand signals; (h) rides S2-D9 | Fable |
| S2-D16(g) typed VDP/DMA builder | prelude-not-language | growing production prelude (S2-D3 ownership) | porter, prelude PR |
| L2 objdef / L7 mapping human DSLs | Volence ruling 2026-08-01 | first content moment ("the features era") — re-ask Volence | Fable |
| B-F scratch splices / reg aliases | S2-D6(c)(d) | splice-demand during P1–P5 adoption; else recorded park at close | inside Track B spec |

Closed-as-no (survives the freeze, restated so nobody re-opens by accident):
S2-D16(i) anonymous labels (decided AGAINST); S2-D14(e) `bank:`+`vma:` (reject
re-affirmed ×2); D13(e)/(f) discharged at #7.

## §5 — The order (one line)

Track 0 merges → {T1/T4/T2 ∥ B-P1} → {B-P2 ∥ B-P3 ∥ C1 ∥ D-batch(D6 first)} →
B-P4 (+C2) → B-P5 → D5 + B-F disposition → **Spec-2 close review** (confirm
T3/T5-T7 discharged; the park ledger + Track R become the next era's opening
agenda; L2/L7 re-ask happens there; R1 spec is the next Fable deliverable).

Every parcel: Opus porter, this plan + the named spec section as the brief,
overseer gates, era lens panel on merge. Nothing in this plan changes a ROM byte.
