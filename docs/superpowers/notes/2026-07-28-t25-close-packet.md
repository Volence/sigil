# t25 — TRANCHE CLOSE PACKET (the debug trio: error_handler PORTED, debugger RECLASSIFIED, sound_debug DEFERRED)

**Seventh tranche under the LEAN amendment.** Scope (brief `2026-07-28-t25-debug-trio-brief.md`): the debug trio — `engine/debug/debugger.asm` RECLASSIFIED (not ported), `engine/debug/error_handler.asm → error_handler.emp` (primary lane, PORTED), `engine/debug/sound_debug.asm → sound_debug.emp` (small lane, DEFERRED). Full loop `0 → 1 → 2 → (3→4→5)×2 → panel → 6`. Porter = Opus subagent dispatched + driven by the overseer (Fable); three checkpoints countersigned (own rebuild + own strict each).

Branch tips at close: **aeon `4e2f9c6` / sigil `<this packet's commit>`** (bases aeon `4df4ad8` / sigil `184ca66`).
Branch ROMs at close: **plain `c51342d0`/421041 · debug `992d9e7d`/429102** (UNCHANGED from canonical — t25 is byte-neutral end to end; PROVENANCE re-baseline is a formality, no image change).
Full paired strict at every checkpoint; final **2620/0** (baseline 2604 + 4 error_handler_port + 2 mixed_error_handler + 6 diag_assert_vector + 3 diag unit + 1 lower_data; the parser hang repro is `#[ignore]`d, uncounted).

## Scoreboard

| Workstream | Outcome |
|---|---|
| **debugger.asm** | RECLASSIFIED — zero ROM bytes (config equates + macro tower); dropped from the 68k backlog by reclassification, POST-TWIN-RETIREMENT home ledgered |
| **error_handler.emp** (12 exception stubs + the 0xF56 vendored blob) | byte-identical BOTH shapes at the first linking compile; region 0x10B0 each (stub table 0x15A + blob 0xF56) |
| **The demanded feature (reshaped)** | `raise_exception` construct + options form — the `__ErrorMessage` counterpart (frame-less); the brief's "raise_error options form" was a UNDERSHOOT (see corrections) |
| **New capability** | `dc.l <label>` absolute-symbol pointers in `dc` position (closed a tranche-8 consumer-gated ledger row) |
| **The mixed whole-ROM gate** | unblocked by the overseer's numeric-org ruling (an equ off an external base won't resolve; off a numeric base it folds) — SHIPPED both shapes |
| **sound_debug.emp** | DEFERRED — hit two frontend gaps (extern-in-lea-disp parse; a deep parser infinite-loop); repro committed `#[ignore]`d, bounded partial fix taken, both ledgered demand-1; kill row 42 stays OPEN |
| **Step-2 modernize** | near-empty by design (blob untouchable, stubs already minimal); the two ratified feed-forward spellings executed into the canonical checklist |
| **PANEL (A1+B1+C2)** | DRY STANDS after adjudication — comment/dedup/ledger only, no algorithmic/construct/optimization rework (C2: zero real bugs) |
| **Ownership flips** | the 12 vector labels (BusError..ErrorTrap) flip to `.emp`; proven by a two-module link test |

## Step 0 (design note `2026-07-28-t25-step0-design.md`, committed before code)

Region confirmed from the real listings; bases + canonical CRCs verified BEFORE any code. The ONE demanded feature (`raise_exception` + options form) TDD'd against the REAL `__ErrorMessage` macro via the `diag_assert_vector.rs` cross-front-end harness. **P1 verdict: the brief's literal probe (bare `raise_error` == an opts=0 stub) FAILS** — `raise_tail` emits a 6-byte `pea *(pc)` + `move.w sr,-(sp)` frame the CPU-vectored stub must NOT; recorded as the finding that motivated `raise_exception`. Hazard sweep found the sound_api-local stopZ80 lift already resolved (engine.z80_bus), the sr-undeclared lint NOT triggered by the frameless stubs, and §23 as the t25 cluster.

## Step-1 gate list — FILLED (every gate named to its artifact)

byte gate plain / debug — **windowed**: `error_handler_port::error_handler{,_debug}_region_matches_reference` (green FIRST compile, `[BusError,EndOfRom)` 0x10B0) · byte gate — **whole-ROM**: `mixed_dac_rom::mixed_error_handler{,_debug}_rom_matches_assembled_reference` (`assemble_mixed_error_handler_as_side` SIGIL_EMP_ERROR_HANDLER + placed `.emp` + `assert_rom_matches_convsym`, ASSEMBLED_LEN pinned) · **ownership flip**: `error_handler_port::vector_labels_resolve_to_emp_ownership` (a synthetic vectors.asm dc.l table resolves all 12 stubs to the `.emp` labels) · **MDDBG__ equ derivation**: proven by the mixed gate (the table folds off the numeric ErrorHandler) · region pin `pins::ERROR_HANDLER` via repin (repin.toml region block added) · gate-off dual rebuild exact (c51342d0/992d9e7d) · the **capability-gap finding** `derived_equ_off_external_base_is_unresolved_today` (retirement-gated kill row 52) · demanded-feature TDD: P1 parity + 2 address_error vectors + the frame-difference finding + 2 negative probes (`diag_assert_vector`, 15/15) · paired strict.

## Step-2 checklist — FILLED (all seven, explicit outcomes)

1. **Branch conversions**: NONE — the 12 stubs have no branches (`raise_exception` is a construct); the blob is data. Not-applicable, named.
2. **Structural width pins**: NONE. Named not-applicable.
3. **Bare-symbol width rule**: the blob's `dc.l MDDBG__Debugger_*` are bare symbols (the new capability); no absolute-EA operands in the stubs.
4. **Brace-indent**: conformant (the stub `{}` bodies indent one level).
5. **Idiom list**: **ADOPTED `raise_exception` and `dc.l <label>` (the two ratified feed-forward spellings, executed into campaign-port-loop.md this commit)**. Not-applicable and named: Sst.field, bareword winptr/bankid, typed VDP fns, Sec/Act, movem-range (the stubs' `clobbers()` is empty), abs-EA-over-link-base.
6. **Type-layer walk**: not-applicable — the stubs are `()` procs (no domain-valued register params); the blob is opaque data.
7. **Noticing**: `raise_exception` (the exception-vector diag construct, frame-less) and `dc.l <label>` are the two new house-format items — both ADDED to the step-2 checklist + the step-4 construct inventory this commit (the feed-forward rule).

## Byte-delta table (measured, not predicted)

| change | Δ plain | Δ debug | absorbed by |
|---|---|---|---|
| step 1 (error_handler.emp + gate + all gates/tests) | 0 | 0 | additive; gate-off exact |
| numeric-org restructure (mddbg_symbols extract + gate equ + ErrorHandlerBlob rename) | 0 | 0 | equ table emits 0 bytes; gate-off exact |
| step-2 (the two spelling additions are doc-only) | 0 | 0 | — |
| pass-1 3(b) + panel cleanup (comments, named consts, test dedup) | 0 | 0 | — |
| **NET** | **0** | **0** | — |

**Zero byte movement end to end.** Canonical CRCs and EndOfRom (0x5DB60/0x5F65A) UNCHANGED at every commit — the brief's byte bar held without a single STOP.

## CORRECTIONS LIST — the TWO BRIEF ERRORS (overseer-error rows, house format)

| claimed (brief) | true | note |
|---|---|---|
| "the 12 stubs port via `raise_error`; the `__ErrorMessage` stub is EXACTLY what `raise_error` lowers to (diag.rs raise_tail)" | a bare `raise_error` is **+6 bytes** wrong at an exception vector: `raise_tail` emits `pea *(pc)`(4) + `move.w sr,-(sp)`(2), the deliberate-raise frame simulation, which the CPU-vectored `__ErrorMessage` must NOT (hardware pushed SR+PC) | **OVERSEER-ERROR (recon).** The frame/no-frame distinction is the whole construct boundary; the brief conflated the deliberate-raise and exception-vector shapes. Surfaced exactly as P1 was designed to. The demanded feature was therefore a superset (`raise_exception`), not just the options form. Verified against the real listing (BusError starts at `jsr`) + a live `raise_error` site (487A FFFE / 40E7). |
| "the MDDBG__ equ table derives off the flipped `ErrorHandler` — `equ ErrorHandler+$xxx` over a link-resolved base; prove it resolves" | sigil does NOT resolve an equ off a link-EXTERNAL base (the derived symbol stays unresolved) — the clean flip is **not buildable** as stated | **OVERSEER-ERROR (assumed capability).** Caught by the porter's probe + captured as an executable finding. RESOLVED by the overseer's own checkpoint-(a) ruling: derive off a NUMERIC base (folds at assemble time) instead — the numeric-org derivation shipped the mixed gate. The external-base equ is now a ledgered demand + kill row 52. |

Both errors were report-items at checkpoint (a), ruled on by the overseer, and are logged here per the campaign's own overseer-error practice.

## PER-PASS: step-3 vs step-5

- **Pass 0-2**: P1 finding (frame prefix); the mixed-gate block + its numeric-org unblock; the demanded-feature-reshaped step 1; the near-empty step 2 (blob untouchable, stubs minimal). *step-5:* no changes (cold error path, opaque blob).
- **Pass 1** — *3(a):* one ask, the `exception_vectors` table construct (verb-c, LOG not build — the explicit stubs are clearer; ledgered). *3(b):* fixed the stale blob header comment (ErrorHandlerBlob rename + MDDBG__ spelling + numeric-org). *4:* nothing built (the table is verb-c). *5:* no changes — cold error path (C1 inactive) + opaque vendored blob, recorded.
- **Pass 2**: EMPTY at all three steps → dry claim → panel.

## PANEL ROUND (A1 + B1 + C2 — three lenses; C1 inactive cold-path, C3 inactive sound_debug-deferred; read-only, one round)

**DRY STANDS** (t22 bar: adjudication yielded comments, named consts, test dedup, kill/ledger rows, and record corrections — no algorithmic, construct, or optimization re-work). Per-lens:

- **C2 (correctness): ZERO real bugs.** Every hand-computed value re-derived clean: `0x15A+0xF56=0x10B0`; `base+0x10B0=EndOfRom` both shapes; ErrorHandler/PagesController/BTN offsets; the self-rel `$148 = $30C-$1C2-2`; the numeric equ = pure-AS ErrorHandler EXACTLY. Exit-flag parity holds with `frame=false` (the prefix is still all word-sized). The `dc.l` register guard, the flag-list parser (terminates every path), the numeric-org collision-avoidance (nothing references bare `ErrorHandler`), and the asm_body progress guard (fires only on genuinely-stuck states) all verified. Two latent config-coupling notes → kill row 53. One minor: `dc.l sr/ccr` weren't rejected → FIXED (aligned with `bare_symbol_seg`).
- **A1 (cold reader):** real items, ALL byte-neutral, ALL taken — stale comment claims (error_handler.emp blob comment; diag.rs "later tasks" preamble; the raise_tail parity enumeration missing the raise_exception case), an inert 12× `#[allow(dead_code)]` cluster (removed — pub items are reachable), change-history narration (trimmed per the exhibit-comment rule), duplicated `0xE6C`/`0xEB8` literals (named `BTN_A/B_OFFSET`) + a duplicated test helper (extracted `synthetic_handlers()`). Two language asks ledgered: a `noreturn` proc marker (demand 1) and vendored-binary-as-dc.l (folded into the POST-TWIN-RETIREMENT runtime row).
- **B1 (corpus):** 7 clean; one actionable — the numeric-ErrorHandler-equ gate arm is off-pattern (all other gate arms org-only) but justified/documented/retirement-gated → captured as **kill row 52** with the `derived_equ` test as its kill condition. `raise_exception` (shared `raise_tail` core, exemplary), `dc.l <label>` (shares the `Cell::SymRef` primitive with lower_ptr; separate sites justified by width/context), and the mixed+windowed tests all follow corpus convention.

## SOUND_DEBUG DEFERRAL (kill row 42 stays OPEN)

The sound_debug mirror lane is DEFERRED to its own mini-tranche (overseer ruling 3). `sound_debug.emp` was written (transliteration, in history at aeon `0618dd4`) but does NOT lower — it hit two frontend gaps: (1) an `extern(...)` in a `lea` DISPLACEMENT expression does not parse; (2) a DEEP parser infinite-loop (context-sensitive; minimization could not reduce below ~26 lines). Both ledgered demand-1; the hang repro is committed `#[ignore]`d (`parser_recovery_hang.rs`) with a bounded PARTIAL fix (an `asm_body` zero-progress guard — a real robustness improvement, but the target loop is deeper). The file was REMOVED from the tree (a non-lowering `.emp` hangs the corpus-scanning tests). **Kill row 42 (vblank.emp's `Sound_DebugMirror` extern decl) stays OPEN** — open-reason: the decl deletion is same-commit with a PROVEN port, which needs a lowering `.emp`; the lane returns and closes row 42 when both frontend gaps close. C3 was therefore INACTIVE (recorded, not run empty).

## Step-6 corpus sweep (enumeration, per-site outcomes — EXECUTED)

- **(a) `raise_exception` class** — grep `__ErrorMessage` / exception-vector shapes corpus-wide: hits are `error_handler.emp` (the port), `debugger.asm` (the MACRO def), `error_handler.asm` (the twin). **error_handler is the SOLE exception-vector region** — `raise_exception` has no other adoption site. NOT-AN-INSTANCE elsewhere.
- **(b) `dc.l <label>` class** — the census target is `vectors.asm` (40 `dc.l Label` entries, the CPU/interrupt/trap vector table): it is NOW portable via the new capability but is OUT OF SCOPE (t25 flipped the 12 error labels it references, not the table). Enumerated, NOT ported. The other `dc.l Label` `.asm` hits (song_table, sfx_table, act_descriptor, entity_data, load_object, tile_cache, boot_data, bg_anim, structs, macros) are DATA tables that port via typed `data`/`table`/`lower_ptr` pointer fields, NOT raw `dc`-in-code — so they are not `dc.l <label>`-capability instances. **vectors.asm is the one genuine future consumer.** (This closed the tranche-8 consumer-gated ledger row.)
- **(c) numeric-org-equ class (kill row 52)** — grep the corpus for `X: equ Label+offset`: the ONLY instances are the 45 MDDBG__ equs (this port). A broad `equ`-off-symbol sweep finds exactly one other hit — `OJZ_Sec4_Blocks equ OJZ_Sec2_Blocks` (a same-file DATA-label alias, not a flippable-code-region label). **The numeric-org-equ class is a SINGLETON** — no other gate arm will need the workaround when its file flips.
- **(d) the zero-progress parser guard** — strict runs 2620/0 WITH the guard active; the guard fires only when a statement parse consumes NOTHING (a genuinely-stuck state), and every valid statement form consumes ≥1 token. **No corpus file changes behavior under it** (strict green implies it, stated).

## NEITHER-BUCKET HEADLINES

- **The P1 probe did exactly its job** — it caught a load-bearing brief recon error (the frame-simulation prefix) before the port leaned on it. The whole `raise_exception` construct exists because the probe fired; "prove before you lean on it" paid for itself the same tranche it was mandated.
- **The overseer's numeric-org insight turned a blocked mixed gate into a shipped one with zero new sigil capability** — an equ off a numeric base folds at assemble time where an equ off an external base does not resolve; the workaround is a per-shape numeric `ErrorHandler` equ in the gate-ON arm (kill row 52, retirement-gated by the finding test).
- **The engine 68k conversion backlog is EMPTY at t25 close** — debugger reclassified, error_handler ported, sound_debug deferred to a mini-tranche. §23 annotated. The remaining engine backlog is the ~5 Volence-deferred Z80 items + ~10 game-side.
- **A parser ROBUSTNESS bug surfaced+ledgered+partially-fixed** — a front-end must error, never spin; the sound_debug body found a context-sensitive infinite recovery loop. Repro committed, `asm_body` guard added (partial), the deep loop ledgered demand-1.
- **Two POST-TWIN-RETIREMENT rows born** — debugger.asm's config/macro-tower home, and the own `.emp`-native diagnostics runtime (Volence-ruled: sigil-emitted symbol table kills convsym, the diag construct sheds the third-party format mirror, sized to the used surface — the largest single retirement dividend in the debug cluster).

## OPEN AT MERGE (ledger duties done; the rest is not mine to close)

Kill rows 52 (numeric-ErrorHandler-equ, kill = the derived_equ test flips) + 53 (raise_exception config-pin) born; **kill row 42 stays OPEN** (sound_debug mini-tranche). Demand-1 ledger rows: extern-in-lea-displacement parse; the deep parser-recovery loop (repro committed); the `noreturn` proc marker; the `exception_vectors` table. Two POST-TWIN-RETIREMENT rows. Tranche-8 `dc.l`-label row CLOSED (68k half). The step-2 checklist + step-4 construct inventory carry the two ratified spellings. PROVENANCE re-baseline is a formality (byte-neutral, no image change).
