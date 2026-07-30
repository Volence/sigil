# 2026-07-29 — seam-1 FINISHER: checkpoint (b) close packet

**HEADLINE: THE RESIDENT SOUND BLOB IS NATIVE, FIVE TWINS DELETED, THE TRANSITIVE
DIAGNOSTIC LIVE.** The three remaining seam-1 items are done: the 47 intra-blob
`extern proc` decls became module imports (byte-neutral); `[call.clobbers-incomplete]`
— the Z80 transitive-clobbers diagnostic — ships and is GREEN in scope; the loop +
dry panel ran on the touched surfaces. STOP at checkpoint (b) for overseer countersign.

Branches (both `seam1-native-link`): sigil `28af802` · aeon `4896039`.
Builds need `SIGIL_EMIT=<sigil-wt>/target/release/emit_sound_blob`; the strict suite
runs from the sigil worktree with that env + `AEON_DIR`.

## 0. Bars (all GREEN)

- **Emitted blob (byte-identical through ALL four finisher commits):** plain
  `z80_sound_blob.bin` = **c7534c84** (6172 B) · debug `z80_sound_blob_debug.bin` =
  **fd2a845d** (6298 B) · `z80_sound_syms.asm` = **87b87b1b**.
- **Both-shape ROMs canonical:** artifact CRC/size plain **22f69f77 / 414414** · debug
  **d4e8d043 / 422466** (the porter-1 SECONDARY full-file bar); the PRIMARY assembled-ROM
  bar (`assert_rom_matches_convsym`, 0..EndOfRom + convsym fields — e5765873 / dab4f06c)
  is proven by `mixed_seam1_rom_matches_reference_{plain,debug}`.
- **seam1 gates 9/9** (`seam1_native_link.rs`); **diagnostic 5/5** (`z80_clobbers_incomplete.rs`).
- **Strict 2880/0 (1 ignored)** = 2898 baseline − 23 retired windowed-oracle tests + 5 new
  diagnostic tests. Full workspace, `--no-fail-fast`, AEON_DIR + SIGIL_EMIT at this branch.
- **Byte movement ZERO** across all four commits: every change is a contract annotation
  or an import decl (emits nothing) or a test/harness change; re-verified after each.

## 1. Item 1 — the 47 extern proc → module imports (byte-neutral)

The five resident sound `.emp` files replace their hand-written cross-file `extern proc`
decls with `use engine.sound_*.{...}` imports (psg 3 / fm 1 / sequencer 26 / sfx 13 /
driver 4 = 47; every target verified to a sibling `pub proc`, all 126 pub procs globally
unique). The resident-blob linker (`seam1::lower_one`) resolves each import to a contract
stub DERIVED from the sibling definition:

    preserves(stub) = (Z80 universe − declared_clobbers − out)     ← clobbers-complement
                    ∪ declared_preserves ∪ home-module invariant

— the honest-contract theorem (a proc with complete clobbers preserves everything it does
not clobber-or-return), SOUND precisely because item 2 verifies the corpus's clobbers are
complete. There is no separate decl to drift from the def → **the t36 extern-decl-vs-def
hazard is retired STRUCTURALLY for the intra-blob set.** Byte-identical (a decl emits
nothing): emitted blob + both ROMs unchanged.

The 5 per-file windowed reference-slice oracles (sound_{fm,psg,sequencer,sfx}_port +
z80_sound_driver_port) are RETIRED: the native link is ONE module, so a standalone per-file
lowering can no longer resolve cross-file contracts, and the whole-blob gates already
supersede them (the blob IS the concatenation of every file window; t24 controls
duplicated). This completes the design §3.2 transformation porter-1 began.

## 2. Item 2 — `[call.clobbers-incomplete]` (the headline diagnostic)

Over the ONE linked module set: a proc's declared `clobbers` must be a SUPERSET of its
transitive effect (local writes ∪ reachable-callee clobbers − verified preserves). Z80 has
NO per-proc clobber lint (68k-only), so this was checker-invisible across `call`s (the t37
proof). The native link is the precondition that makes it computable — every callee body is
in scope.

- **Engine:** `seam1::z80_clobbers_report{,_doctored}` builds a Z80 `ProcNode` map (local
  writes via the new `z80_preserves::z80_written_registers`; direct call/tail edges;
  verified preserves = declared ∪ module invariant; declared clobbers) and runs the EXISTING
  generic `closure.rs` fixpoint + `check_firings` — no re-implementation of the closure.
- **TDD (5 tests):** RED = the t37 `Sfx_Frame` iy under-claim (reverted header) fires
  transitively; GREEN = honest corpus 0 in-scope BOTH shapes; non-vacuity = an injected
  `Sfx_Steal` iy under-claim fires; completeness = 0 dropped + 0 unresolved callees
  (**OQ-4 confirmed:** every resident code-call is intra-blob, the banked side is data-read
  only); scope non-vacuity = the excluded sub-machine genuinely fires and is fully classified.

### 2.1 The fixpoint surfaced 4 GENUINE in-scope under-claims — all fixed honestly

Never caught before (no Z80 clobber lint). All fixes byte-neutral (contract annotations):

| proc | fix | why honest |
|---|---|---|
| `Sequencer_Frame` | +iy | its `jp Sfx_Frame` tail-call folds Sfx_Frame's clobbers; Sfx_Frame destroys iy |
| `Run_SeqFrame_OnSongBank` | DROP false `preserves(iy)` → clobbers(...iy) | `call Sequencer_Frame` with NO push/pop bracket |
| `SndDrv_IdleTick` | DROP false `preserves(iy)` → clobbers(...iy) | same — the claim only "held" on Sequencer_Frame's incomplete clobbers |
| `SndDrv_Idle` | +iy | calls the above chain |
| `SndDrv_VBlank` | clobbers() → clobbers(ix,iy) | `jp SndDrv_ISR` (which clobbers ix,iy) |
| `Z80_Sound_Entry` | clobbers() → full set | non-returning reset entry, runs the whole driver |
| `SndDrv_Init` | +ix,iy | `falls_into SndDrv_Idle` (which clobbers them); non-returning entry — exposed by the dry-panel `falls_into` edge modeling |
| `Seq_HookNoteOff` | clobbers(af) → +bc | tail-jumps to Fm/Psg_NoteOff (clobber bc) — dry-panel scope-narrow brought it in scope |
| `Seq_HookSetVol` | clobbers(af,bc) → +de,hl | tail-jumps to Fm_SetVolume (clobbers de,hl) — same |

(The last three landed in the dry-panel refinement commits — see §5. All byte-neutral.)

The `preserves(iy)` cascade is the correctness catch: item-1's clobbers-complement
derivation, applied to Sequencer_Frame's *incomplete* clobbers, was PROPAGATING a false
`preserves(iy)` credit that let the driver's unsound `preserves(iy)` chain verify. Fixing
Sequencer_Frame's clobbers removes the false credit and forces the driver's honest
declaration. Safe: idle/ISR context holds no live ix/iy; no returning caller relies on the
dropped preserves.

### 2.2 The scope boundary (design §4 face-4 / OQ-4)

The computed opcode-dispatch sub-machine — TRAMPOLINE-ONLY: the `Seq_Op_*` dispatch targets
+ the loop re-entry `Seq_ContinueFetch`, reached ONLY via the `ex(sp),hl; ret` trampoline that
threads `hl` as the stream cursor — is OUT of the direct-call closure and reported SEPARATELY.
Its transitive clobbers depend on the un-traversable computed edge (e.g. `Seq_Op_Patch` clobbers
ix through it), so a direct-call closure CANNOT soundly verify it — the boundary the design's
`[dispatch.trampoline]` annotation + face-4 (the bsr-classifier) name. The external entry
`Sequencer_Channel` carries the honest broad clobbers the loop actually inflicts, so nothing
outside the sub-machine consumes an under-claimed handler contract. `is_opcode_dispatch_proc`
/ `in_scope_firings` implement the split; the sub-machine genuinely fires (104 firings) and is
asserted fully-classified, so the filter is non-vacuous. (Dry-panel C2 tightened this from a
`Seq_*`-name filter to trampoline-only: the `Seq_Hook*` event helpers are straight-line
call/ret — bounded, closure-verifiable, directly called by in-scope `Sequencer_NextOpcode` —
so they are CHECKED in scope, not excluded.)

## 3. Ledger / kill-list

- **Gap-ledger:** new finisher row (extern-decl-vs-def MOOT-for-imports; row 1798
  transitive-clobbers-completeness DISCHARGED = the diagnostic landed; row 1810
  bsr-classifier STAYS ledgered — same computed-dispatch boundary, verifier-side).
- **Kill-list:** rows 70/71/78/83/87 already CLOSED by porter-1 (the twin deletion); their
  KILL conditions named "closes the extern-decl-vs-def AND transitive-clobbers ledger rows"
  — now discharged by this parcel.

## 4. Loop record (2 → 3 → 4 → 5)

- **Step 2 (modernize):** the extern→import conversion (item 1); the import-derived contract
  stubs. Byte-neutral.
- **Step 3 (retrospect) + Step 5 (engine optimize) — the SAME findings:** the transitive
  fixpoint (item 2) is itself the step-5 correctness net; it surfaced the 4 genuine
  under-claims (§2.1) — step-3 findings fixed in-parcel (not merely ledgered) because they
  are soundness bugs the seam exists to close.
- **Step 4 (back-prop):** none — the imports/diagnostic are seam-local; no upstream engine
  file gains a construct.
- **Neither-bucket:** the `preserves(iy)` cascade (a latent unsound contract the seam's own
  clobbers-complement derivation would otherwise have laundered) — foregrounded as the catch.

## 5. Dry panel (A1 + B1 + C2 + C3; C1 named-basis)

*(A1 ceremony/style + C3 hardware-prose, B1 corpus-pattern, and C2 correctness ran as
fresh read-only lens subagents; C2 owed one import-conversion re-derivation, the diagnostic's
fixpoint math on one real proc chain, and one emitted-blob-identity re-check.)*

**VERDICT: NOT trivially dry — the panel surfaced real findings, all adjudicated + applied
or ledgered; a re-run would now be dry.**

- **C2 (correctness, highest weight) — CONFIRMS the porter on all three owed re-derivations,
  + 2 precision findings FIXED:** (1) import re-derivation on `Psg_SetVolume`: derived
  preserves `{d,e,h,l,ix,iy}` ⊇ the retired extern's `{d,e,h,l,ix}` — strict superset,
  byte-safe, never over-credits (sound because the diagnostic verifies the clobbers complete).
  (2) The `Sequencer_Frame` iy fixpoint is forced by the `jp Sfx_Frame` tail; the driver
  `preserves(iy)` cascade is COMPLETE (grepped every `preserves(iy)` — none still reaches the
  frame engine). (3) `z80_written_registers` correctly excludes call/push/pop (no double-count,
  no false push/pop fire). **FINDINGS FIXED:** the `Seq_Hook*` exclusion was broader than the
  strictly-un-modelable set (NoteOn/NoteOff are directly called by in-scope
  `Sequencer_NextOpcode`) → scope narrowed to trampoline-only (`Seq_Op_*`/`Seq_ContinueFetch`),
  the 2 real `Seq_Hook*` under-claims widened honestly; the `falls_into` edge was unmodeled →
  now modeled, which exposed + fixed the genuine `SndDrv_Init` under-claim. All byte-neutral.
- **A1 (ceremony) — 3 stale-comment findings FIXED byte-neutral:** the extern→import
  conversion left `sound_psg`/`sound_fm` header sentences describing the removed `extern proc`
  mechanism (+ a history/codename phrase) and the driver's INBOUND-TRUST-CONVERSIONS block
  (history + a task codename + stale line pointers) — all rewritten to present-tense contract
  facts. Import accuracy + grouping: CLEAN (all 47 verified to a sibling `pub proc`).
- **C3 (hardware-prose) — CLEAN.** Every new/changed contract comment (the `jp Sfx_Frame`
  iy fold; `SndDrv_VBlank`→`SndDrv_ISR` ix/iy; the no-push/pop iy claims) verified against the
  code + the driver's ix/iy-invariant header; nothing overstated.
- **B1 (corpus-pattern) — closure-sharing + harness placement CONFIRMED correct (reuses
  `closure.rs`, no re-implementation); 1 DRY defect LEDGERED:** `transfer_target` is a third
  divergent copy of the Z80 inter-proc-edge predicate (`z80_preserves::branch_sym` + the
  proof's local-label test); latent-zero-exposure; kill = the next `z80_preserves` CFG toucher
  unifies them (the bsr-classifier-row pattern — a byte-frozen shared-CFG change is out of a
  finisher). Gap-ledger row added.
- **C1 (cycle) INACTIVE, named basis:** no in-source T-state annotation is touched (rung-4's
  driver `cycles()`/`pad_to_cycles` unchanged; the parcel is contract- + harness-only).

## 6. Corrections list (porter-1's five findings, relayed + this parcel's)

Porter-1 (the stand-up + deletion) recorded five findings this finisher inherited/relied on:
1. **OQ-1 answered YES with EXISTING machinery, no compiler feature** — each `.emp` opens
   exactly ONE named section + no default `text` carrier, so concatenating the five
   independently-lowered modules dodges the row-1639 `sec0` collision.
2. **OQ-2 CORRECTION (tree wins over design):** the debug blob base is **$3E2**, not $3DE
   (the design §2.1 fixed the +$7E SIZE but assumed the base held; the debug shape grows +4
   UPSTREAM of BootData). Verified from the listings.
3. **The provenance model:** PRIMARY = the assembled-ROM CRC (0..EndOfRom, header-neutral —
   canonical, UNCHANGED gate-off ≡ gate-on: e5765873 / dab4f06c); SECONDARY = the full-file
   artifact CRC (drifts per deletion as symbol tables shrink: 22f69f77/414414 ·
   d4e8d043/422466). The task's canonical CRCs are the SECONDARY bar.
4. **Option A** (sigil emits the blob, asl BINCLUDEs it) + **Option B** (the assembled-ROM
   bar scopes past the deb2 debug-symbol append, which legitimately shrinks as deleted twins'
   labels leave the table).
5. **The windowed oracles "→ reference-slice gates"** — porter-1 built the whole-blob gates
   but LEFT the per-file oracles (which stayed green only on the lingering extern procs).
   **This finisher completed the retirement** (item 1) — they are structurally incompatible
   with the import model and redundant with the whole-blob gates.

This parcel's own corrections: none surfaced beyond the loop findings (§4); the dry panel's
adjudication is §5.

## 7. Seam-2 handoff

Seam-2 = the banked / `phase 08000h` DATA side the resident blob READS (dac_samples /
mt_bank / sfx_bank / dac_sample_tab + seq_opcode_tab, placed at pinned bank addresses;
design §5). What it inherits:

- **The emit-tool pattern:** `sigil emit_sound_blob` (Option A) is the template — sigil emits
  byte-deterministic build inputs from tracked `.emp` sources, asl packs the rest, the
  assembled-ROM CRC is the provenance bar, and `build.sh` fails LOUDLY if the emit binary is
  missing/stale. Seam-2 is a DATA-PLACEMENT emit of the same shape.
- **The sfx-helper coupling note:** when `sound_sfx.asm` was deleted, its build-time helpers
  for the banked `sfx_blob_win_tab.asm` (`SFX_WIN_MASK`/`SFX_WIN_BASE` + `sfx_winptr`/
  `sfx_bankid`) moved into `boot_data.asm`'s gate arm (kill row 83). Seam-2, which owns
  `sfx_bank.emp`, must re-home those helpers with the banked SFX data they serve.
- **The Volence data-format principle pointer:** the sound DATA (songs/patches/SFX) is
  GENERATED by the Python tools (`tools/zyrinx_player.py`, `song_hcz2.py`, `sfx_transcode.py`)
  — the generator is the source of truth (design §5). The `.emp` data-table DSL direction
  (offset-table `dc.w Target-Base`, Plan-6-relevant) is the memory-noted candidate for how
  that data should be expressed natively; the generator conversion sits AFTER/ALONGSIDE seam-2.
- **The scale-2 complication (gap-ledger ~1623):** Z80 `db` can't carry a link symbol, so the
  `SND_*` window-ptr / DAC-descriptor cells stay comptime — seam-2 owns closing that.

## 8. sound_api — DEFERRED with reason (its own 68k parcel)

`sound_api` is 68k engine code (the CALLER that pokes the Z80 RAM command slots), NOT in the
resident Z80 blob (design §1.2 / §3.5). Its cutover rides the already-scaffolded
`SIGIL_EMP_SOUND_API` gate + org-resume arm. It is a COUPLED-but-DISTINCT flip, recommended
as its own small 68k parcel to keep seam-1 purely the Z80 code link. **Deferred — not part of
this seam.** (Its coupled kill conditions: row 10 SND_* mirrors, rows 24/36 stop/startZ80
templates, row 43 the sr_masked bracket — all Spec-5-era, unchanged here.)

## Close duties (staged for the overseer)

- Provenance: canonical UNCHANGED (assembled-bar e5765873/dab4f06c); artifact
  22f69f77/414414 · d4e8d043/422466; strict 2880/0 (1 ignored).
- Merge queue: sigil + aeon `seam1-native-link` push TOGETHER after countersign (do not
  push/merge here — finisher STOPs at (b)).
- Next: seam-2 (data) → generator → Spec-5; sound_api anytime before Spec-5.
