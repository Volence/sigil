# t26 — TRANCHE CLOSE PACKET (two lanes: vectors.emp ENGINE + game_debug.emp — the FIRST game-side .emp code module)

**Eighth tranche under the LEAN amendment.** Scope (brief `2026-07-28-t26-vectors-gamedebug-brief.md`): lane A `engine/system/vectors.asm → vectors.emp` (the dc.l shakedown), lane B `games/sonic4/debug/game_debug.asm → game_debug.emp` (the first game-side .emp CODE module, off-canonical oracle). Full loop `0 → 1 → 2 → (3→4→5)×2 → panel → 6`. Porter = Opus subagent dispatched + driven by the overseer (Fable); two checkpoints countersigned (own rebuild + own strict each).

Branch tips at close: **aeon `61ce926` / sigil `<this packet's commit>`** (bases aeon `d696792` / sigil `9dac348`; sigil master advanced to `d8fb034` during the branch's life — doc-only T1 design/countersign commits).
Branch ROMs at close: **plain `c51342d0`/421041 · debug `992d9e7d`/429102** (UNCHANGED from canonical — t26 is byte-neutral end to end).
Full paired strict at every checkpoint; final **2631/0** (baseline 2620 + 5 vectors_port + 6 game_debug_port; the 2 t25 parser-hang repros stay `#[ignore]`d, uncounted).

## Scoreboard

| Workstream | Outcome |
|---|---|
| **vectors.emp** (64-entry CPU/interrupt/trap table, org 0, $000-$0FF) | byte-identical BOTH shapes at [0,$100); FIXED 256-byte region, structurally byte-neutral. Gate `SIGIL_EMP_VECTORS` (org-$100 resume else-arm). |
| **P-A1 (dc.l label comma-lists)** | **ALREADY SUPPORTED — no demanded feature.** `lower_dc` loops over all operands, one `Cell::SymRef` per label; the t25 single-`dc.l <label>` capability covers the 4-per-line form. Proven link-time + positive control. |
| **The first .emp→.emp vector reference** | vectors.emp's 12 exception entries resolve module-to-module to error_handler.emp's stub labels (`vector_labels_resolve_to_error_handler_emp`). |
| **game_debug.emp** (Debug_MusicToggle + Dbg_SfxIdTable) | byte-identical to the AS twin at the HOTKEYS shape (off-canonical oracle); canonically EMPTY both shapes (twin whole-file ifdef). |
| **Kill row 33 KILLED** | game_loop.emp's Debug_MusicToggle extern deleted → module-to-module. **The extern HID a transitive under-declaration** (see headlines). |
| **The two neither-bucket catches** | (1) honest clobbers `d0-d4/a0-a1` ≠ the twin's `d0-d2/a0/a1` (extern hid Sound_PlayMusic's d3/d4 leak); (2) the type-layer gate DEMANDED SongId/SfxId blesses at step 1. |
| **Step-2 modernize** | lane A pure data (no code changes); lane B — branch conversions NOT-TAKEN (a NEW structural width-pin class: trailing align after data pins preceding widths). |
| **PANEL (A1+B1+C2)** | NOT clean-dry — A1/B1 converged on a data-construct category finding (PROBE-SETTLED: keep proc-with-dc.l), + a free header-precedence fix; C2 all-clean. 2 byte-neutral fixes applied, 4 asks logged. |
| **Step-6 census** | the corpus has exactly ONE remaining `extern proc` (Sound_DebugMirror) — honest+leaf; its stale twin comment fixed. |

## Step 0 (design note `2026-07-28-t26-step0-design.md`, committed BEFORE code)

Region/shape confirmed from the real trees; bases + canonical CRCs verified (fresh aeon worktree, editor-dir rsync'd) BEFORE any edit. **P-A1 verdict: dc.l label comma-lists are ALREADY supported** (asm.rs:771 `lower_dc` loops operands → `Cell::SymRef` per label) — the lane's one candidate demanded feature does not exist; proven link-time (`dc_l_label_comma_list_resolves_each_element` + `..._doctored_diverges`). The extern-vs-import split ruled: DELETE kill row 33 (a `.emp`-owned proc + an `extern proc` decl = §11-Q4 collision; the corpus contract walk is comptime-if-agnostic, so game_debug.emp's proc resolves game_loop's call). Module-level `if` around declarations recorded as unsupported (`ast::Item` has no `If`) → external-gate model (compression_selftest precedent). Hazard sweep: kill rows 33/9, gap-ledger 1053/1092§20/§23/1585 — no in-flight blocker.

## Step-1 gate list — FILLED (every gate named to its artifact)

- **byte gate plain/debug (windowed)** — lane A: `vectors_port::vectors{,_debug}_region_matches_reference` (`[0,0x100)`, both shapes, green first compile). lane B: `game_debug_port::game_debug_matches_as_twin_at_hotkeys_shape` (the off-canonical AS-twin oracle at SOUND_DEBUG_HOTKEYS=1).
- **P-A1 probe + positive control** — `vectors_port::dc_l_label_comma_list_resolves_each_element` (link-time, 4 labels → 4 VMAs) + `..._doctored_diverges`.
- **first .emp→.emp vector flip (both gate states)** — `vectors_port::vector_labels_resolve_to_error_handler_emp` (gate-ON module-to-module; gate-OFF = the region gate's synthetic-carrier resolution).
- **off-canonical oracle non-triviality + guard liveness** — `game_debug_port::emp_diverges_from_doctored_twin` (positive control) + `doctored_extern_fires_drift_guard` (the 16 game-const drift guards fire NAMING the const).
- **canonical-emptiness both shapes** — `game_debug_port::game_debug_{plain,debug}_is_empty` (the twin's whole-file ifdef → zero bytes).
- **extern→import flip (kill row 33)** — `game_debug_port::two_module_flip_resolves_debug_music_toggle` (game_loop.emp + game_debug.emp together → jsr resolves to the placed proc VMA); gate-OFF stays green in `game_loop_port::combo_matrix_matches_as_twin`.
- **contract closure** — `contract_closure_corpus` (no extern hole, no §11-Q4 collision, no firing) after the extern deletion + the honest `d0-d4/a0-a1`.
- **type-layer gate** — `slot_type_corpus` (the SongId/SfxId blesses at the four d0 sound-call slots; demanded, not deferred).
- region pin `pins::VECTORS` (repin.toml + regenerated; base 0, len 0x100). game_debug has NO pin (no canonical address — see corrections). gate-off dual rebuild exact (c51342d0/992d9e7d). paired strict.

## Step-2 checklist — FILLED (all seven, explicit outcomes)

1. **Branch conversions**: lane A — none (data). lane B — **NOT TAKEN, structural**: the trailing `align 2` after Dbg_SfxIdTable pins every preceding instruction size; bare Bcc, `jbsr`-to-a-cross-seam-link-target, and bare-abs are all size-provisional → `[align.provisional]`. Branches/calls stay explicit `.s`/`bsr.w`, byte-identical to the twin. A NEW structural width-exception class (align-dependency).
2. **Structural width pins carry site comments**: the game_debug proc carries the align-dependency exception comment (item 1). Named.
3. **Bare-symbol width rule**: lane A — dc.l `<label>` is the raw-dc form. lane B — the abs RAM operands are explicit `(X).w` (pinned by the same align dependency; not bare). Named.
4. **Brace-indent**: both files conformant (proc bodies indent one level).
5. **Idiom list**: lane A `dc.l <label>` (the t25 spelling). lane B `data [u8;8]` table + `as SongId`/`as SfxId` blesses. Not-applicable + named: Sst.field, winptr/bankid, jbra/jbsr (pinned), raise_exception, typed VDP, Sec/Act, movem-range (clobbers is already a range `d0-d4/a0-a1`), abs-EA-over-link-base (pinned explicit by the align).
6. **Type-layer walk**: SongId/SfxId blessed at the four d0 sound-call construction sites — DEMANDED at step 1 by `slot_type_corpus`, not deferred. (The Dbg_SfxIdTable `[SfxId;8]` typing was PROBED and does NOT lower in isolation — "unknown type: SfxId" — the array-element newtype must be resolvable at the decl site; folded into the panel data-table ask.)
7. **Noticing**: the align-dependency structural width-exception class (proposed for the item-2 list); the mirror-const-with-auto-guard construct ask.

## Byte-delta table (measured, not predicted)

| change | Δ plain | Δ debug | absorbed by |
|---|---|---|---|
| lane A step 1 (vectors.emp + gate + tests + repin) | 0 | 0 | fixed 256-byte region; gate-off exact |
| lane B step 1 (game_debug.emp + extern deletion + twin clobber comment + tests) | 0 | 0 | not AS-compiled; twin edit is comment-only |
| step 2 (structural-pin comment + stale-comment fix) | 0 | 0 | comments |
| panel fixes (game_debug header precedence + vectors form note) | 0 | 0 | comments |
| step 6 (sound_debug.asm stale clobber comment) | 0 | 0 | comment |
| **NET** | **0** | **0** | — |

**Zero byte movement end to end.** Canonical CRCs + EndOfRom (0x5DB60/0x5F65A) UNCHANGED at every commit — the byte bar held without a single STOP.

## CORRECTIONS LIST (recorded like overseer-error rows)

| claimed | true | note |
|---|---|---|
| Census (2026-07-28 game-side recon): `Debug_MusicToggle` emits "at $55A4" in the plain listing | those are AS listing ECHOES of skipped-ifdef lines (address frozen, no byte column) — game_debug emits ZERO bytes in both canonical shapes | **CENSUS ERROR, overseer-OVERRULED at the brief.** Verified by `game_debug_port::game_debug_{plain,debug}_is_empty` (the twin's whole-file ifdef → zero bytes). Logged here per the campaign's overseer-error practice, extended to census errors. |
| Brief framing (adopted): the game-side backlog is ~10 files | ~20 code files + 4 config (the recon's three framing corrections, overseer-ACCEPTED) | census correction recorded; supersedes "~10" in prior notes. |

No PORTER errors this tranche; the two neither-bucket catches were correctness WINS, not corrections.

## PER-PASS: step-3 vs step-5

- **Pass 0-2**: P-A1 (already-supported); the extern→import split (kill row 33); the off-canonical oracle; **the two neither-bucket catches** (the extern-hidden `d0-d4/a0-a1` under-declaration; the slot-type-demanded blesses); the structural width-pin class. *step-5:* no changes (data + human-timescale debug path).
- **Pass 1** — *3(a):* the mirror-const-with-auto-guard construct ask + the align-dependency noticing. *3(b):* the stale `d0-d2/a0/a1` budget comment fixed. *4:* nothing built (the mirror is a verb-c ask). *5:* the 5× `Ctrl_1_Press` re-read is a micro-hoist but debug/human-timescale (C1 inactive), byte-changing → LOG not-taken.
- **Pass 2**: EMPTY at all three steps → dry claim → panel.

## PANEL ROUND (A1 + B1 + C2 — three lenses; C1 INACTIVE cold-path/data, C3 INACTIVE no-VDP/DMA/bus; read-only, one round)

**NOT a clean "dry stands"** — A1/B1 surfaced real cold-reader/corpus items; adjudicated at the gate to 2 byte-neutral fixes applied + 4 asks logged. Re-panel ruled NOT warranted (overseer: the applied fixes are comment-only; the t24 re-panel precedent is for shipped CODE changes). Per-lens:

- **C2 (correctness): ALL CLEAN.** Independently re-derived the 64-long/256-byte vector table (entry-for-entry match, critical slots $00/$04/$70/$78 + all fills), the EXACT `d0-d4/a0-a1` clobber closure (no leak beyond the corrected d3/d4), distinct button bits matching the twin, the SFX cycle (wrap/RING-special-case/bounds/`moveq #0` clear), CC-clobber cleanliness, abs.w sign-extension of the $FFFF80xx RAM operands. Confirmed the two neither-bucket catches were the whole finding set.
- **A1 + B1 CONVERGED — the data-construct category finding, PROBE-SETTLED.** vectors.emp's `proc { dc.l <label> }` is a category mismatch vs the corpus `data [*u8;N]` pointer-array shape. Settled by MEASUREMENT, not opinion: the corpus `data` arrays reject bareword cells (B1 survey — quoted-string/extern()/comptime-for only), and a typed `data [SfxId;8]` fails "unknown type: SfxId" in the isolated compile (the element newtype must be resolvable at the decl site). **VERDICT: KEEP proc-with-dc.l** (t25 built the capability for exactly this raw vector-table class; bareword link cells + a heterogeneous int SSP handled uniformly) + a byte-neutral FORM note in the file. The "should this be data?" question became a concrete, demand-attached language ask.
- **A1 (cold reader): the header omitted the priority chain** — Debug_MusicToggle read as five independent keys; the impl is a fall-through precedence (A>B>UP>C>START, first match wins). Added a byte-neutral header line. APPLIED.
- **Logged asks** (gap-ledger, with demand counts): the typed/label DATA-TABLE construct (highest-value, offset-table-roadmap #1 — 64 vectors + 8 SFX ids + the two probed frictions); the indexed/numbered data construct (vector number is prose-only); the hotkey-dispatch construct with comptime bit-disjointness (button-disjointness re-proved in prose at 4 arms); the ButtonMask/CtrlState bitfield domain-scan (corpus-wide, A4-i-adjacent, not a t26 ask).

## THE FOUR CHECKPOINT-(a) RULINGS (overseer, applied)

1. **Off-canonical whole-ROM mixed-placement machinery DEFERRED, PAIRS with the sound_debug mini-tranche** — one machinery build serves both; game_debug's main.asm gate arm + mixed entry ship with it. t26 cited first demand. Ledgered.
2. **No `pins::GAME_DEBUG`** — nothing canonical to pin (hotkeys off → absent from the listing); the oracle uses a documented test-local synthetic base ($1000). Stated so a future reader doesn't read the absence as an omission.
3. **Local BUTTON_\* mirrors** — kill-list row 54 carries them (constants ownership-flip family, rows 4/18/22); the hoist rides that class. No further action.
4. **External-gate model** — gap-ledger 1512(c) bumped to DEMAND 2, wording refreshed to name the compression_selftest (t22) + game_debug (t26) external-gate precedents it predated.

## Step-6 corpus sweep (enumeration, per-site outcomes — EXECUTED)

- **(a) .emp→.emp bareword-dc.l vector-reference class** — SOLE instance is vectors.emp→error_handler (error_handler's blob is `dc.l <numeric>`; the corpus `data [*u8;N]` arrays reference via quoted-string/extern(), a different mechanism). No other member; nothing to build.
- **(b) off-canonical twin-parity oracle class** — consumers: vblank mirror-shape (t21), game_debug (t26). NAMED NEXT: **sound_debug** (kill row 42, deferred; same shape, takes the identical oracle + the paired machinery from ruling 1).
- **(c) extern-hidden-transitive-clobber census** — after kill row 33, **exactly ONE `extern proc` remains: `Sound_DebugMirror`** (vblank.emp:19). Verdict HONEST: it is a LEAF (no calls, no transitive closure to hide), its declared `d0-d1/a0-a1` matches the real body (d1 = `moveq #SEQ_MIRROR_CHANNELS-1, d1` channel-copy counter, none restored). The catch: sound_debug.asm's HAND COMMENT under-declared ("d0/a0/a1", missing d1) — the game_debug comment-shape, but here the extern was right and the TWIN was stale. FIXED (byte-neutral); the deferred port verifies the closure. **No extern under-declares today.**
- **(d) structural width-pin class** — of three `.emp` with a trailing align, particle_anims + sonic_anims are pure `offsets` data (align pins nothing); **game_debug is the SOLE instance** where relaxable code precedes a data table + align. Not-an-instance elsewhere.

## NEITHER-BUCKET HEADLINES

- **THE EXTERN DECL HID A TRANSITIVE CLOBBER UNDER-DECLARATION — the class the port loop exists to surface (LEADS).** game_loop.emp's `extern proc Debug_MusicToggle () clobbers(d0-d2/a0/a1)` trusted the twin's hand comment; but Sound_PlayMusic (sound_api.emp) clobbers d0-d4/a0-a1, so d3/d4/a0 leak transitively. The extern's opaque-leaf trust hid it; porting the BODY made the contract closure verify it against the real callee. Honest closure `d0-d4/a0-a1`; twin comment corrected in lockstep; C2 independently confirmed the closure is exact. The step-6 census then found the corpus's last extern (Sound_DebugMirror) has the SAME twin-comment-stale shape (d1) — fixed — so no extern under-declares today.
- **The type layer earned its keep at step 1** — `slot_type_corpus` demanded the SongId/SfxId blesses at the four sound-call d0 slots before the corpus error gate would pass; not deferrable to step 2. The type layer working as designed.
- **P-A1 retired with zero new capability** — the lane's one candidate demanded feature (dc.l label comma-lists) was already supported; proven link-time with a positive control, so the lane shipped without a byte of new sigil.
- **The panel's convergent finding was settled by PROBE, not opinion** — both `data`-construct frictions (bareword rejection, typed-array element resolution) were MEASURED, confirming vectors' proc-with-dc.l is right and turning "should this be data?" into a concrete, demand-attached language ask (offset-table roadmap #1, two live instances).
- **The first game-side .emp CODE module shipped** — game_debug.emp, proven by the off-canonical oracle; the game-contract const surface mirrored + drift-guarded (row 54); the ~20-file game-side backlog is now open with its first member landed.

## OPEN AT MERGE (ledger duties done)

Kill row 33 KILLED (carries the closure-catch story), row 54 born (game_debug const mirror). Kill row 42 (Sound_DebugMirror) stays OPEN — its deferred sound_debug port verifies the closure. Gap-ledger: the ruling-1 paired whole-ROM-machinery/sound_debug row; the 1512(c) demand-2 bump; the four panel asks (data-table construct, indexed-data, hotkey-dispatch, ButtonMask); the step-6 four-class enumeration. The step-2 checklist proposal (align-dependency structural width-exception class) awaits ratification. PROVENANCE re-baseline is a formality (byte-neutral, no image change) — owned by the overseer.
