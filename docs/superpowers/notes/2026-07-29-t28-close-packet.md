# t28 — CLOSE PACKET (checkpoint (b): parser debt + DEMAND-3 machinery + sound_debug — kill row 42 KILLED)

**Ninth tranche under the LEAN amendment.** Scope (brief `2026-07-29-t28-parser-debt-sound-debug-brief.md`):
Lane P (parser debt P1/P2/P3), Lane M (the DEMAND-3 whole-ROM off-canonical machinery),
Lane S (`sound_debug.asm → sound_debug.emp`). Porter = Opus subagent dispatched + driven
by the overseer (Fable); checkpoint (a) countersigned (own rebuild + own strict + four
rulings). This packet closes at checkpoint (b) for the merge gate.

**Branch tips:** sigil **`8f2149e`** / aeon **`c48e3f6`** (bases sigil `e48d4f2` / aeon `20f1136`).
**Branch ROMs:** plain **`c51342d0`/421041** · debug **`992d9e7d`/429102** — UNCHANGED from
canonical (t28 is byte-neutral end to end; EndOfRom 0x5DB60/0x5F65A unchanged).
**Full paired strict at every checkpoint; final `2685/0` (1 ignored)** — the un-ignore math below.

## Scoreboard

| Workstream | Outcome |
|---|---|
| **P1** (parser recovery hang) | FIXED — a contextual `extern` opener spun `recover_to_next_decl` (NOT "deeper in operand/expr parse" as the brief said). One-site guard extension. Repro un-ignored. |
| **P2** (extern-in-lea-disp) | FIXED — the gap was the WRAPPED-PAREN `(disp)(An)` form for ANY disp (extern incidental). Boundary: `.field`-in-arith stays a separate LOWERING gap. |
| **P3** (leftmost local-label immediate) | FIXED in shared `eval_path` (byte-neutral, 0 live instances). Positive control `.x+1`==`1+.x`; negative control (undefined leftmost is loud). |
| **Lane M** (DEMAND-3 machinery) | LANDED — 3 whole-ROM off-canonical gates green by name (game_debug Config-A, combined Config-A, z80_init Config-B). Pure-`assemble_root` reference + `assert_rom_matches` empty allowlist (ruling 1); window oracles kept (ruling 2). |
| **Lane S** (sound_debug.emp) | PORTED — the deferred t25 mini-tranche. Unblocked by P1+P2 + the `stop_z80()` proc-body spelling + const mirrors (SeqChannel_len=60/SND_SEQ_TRACE_LEN=32, TILE_SIZE class). Zero bytes both canonical shapes. |
| **KILL ROW 42 KILLED** | vblank.emp's `extern proc Sound_DebugMirror` deleted — **the corpus's LAST `extern proc`; ZERO remain.** |
| **PANEL (A1+B1+C2+C3)** | 4/4 useful, DRY STANDS — all findings byte-neutral comments + ledger notes (no algorithmic/construct/optimization rework). |

## The un-ignore / test-count math (from 2675 baseline)

| Landing | Δ tests | Running |
|---|---|---|
| Baseline (checkpoint a start) | — | 2675 / 2 ignored |
| P1 (`a375526`) — un-ignore the 1 parser-hang repro (renamed) + 1 new minimal | +2, ignored 2→1 | 2677 / 1 |
| P2 (`feabcfb`) | +3 | 2680 / 1 |
| P3 (`6c2c19b`) | +2 | 2682 / 1 |
| Lane M game_debug gate (`9d6b956`) | +1 | 2683 / 1 |
| Lane S combined gate (`8fbd994`) | +1 | 2684 / 1 |
| Lane M z80_init gate (`3a1dea5`) | +1 | **2685 / 1** |

BRIEF ERROR (§4 #2, confirmed): the "two t25 parser-hang repros" is ONE — the other ignored
test is `sigil_diff_reports_byte_identity` (unrelated aeon-source diff test, stays ignored).

## Byte-delta table (measured — ALL ZERO canonical)

| change | Δ plain | Δ debug | absorbed by |
|---|---|---|---|
| Lane P (P1/P2/P3 — sigil-only parser/eval, error-recovery + comptime paths) | 0 | 0 | aeon untouched; mixed byte gates green |
| Lane M gate arms (main.asm / engine.inc / boot_data.asm `else`-arm orgs) | 0 | 0 | gate-OFF canonically (regions empty / sound enabled) |
| Lane S (sound_debug.emp + engine.inc gate + vblank.emp extern deletion) | 0 | 0 | SIGIL_EMP_SOUND_DEBUG off canonically; vblank call comptime-false |
| Panel comment fixes (A1/C3 sound_debug.emp; C2 sound_debug.asm) | 0 | 0 | comments |
| **NET** | **0** | **0** | — |

Dual rebuild after every aeon-touching commit; canonical CRCs + EndOfRom UNCHANGED throughout.
Off-canonical shapes are proven by their OWN oracles (the three `mixed_offcanonical_rom` gates),
not the canonical bar — no STOP-and-report triggered (the empty-allowlist HARD BAR held).

## The four checkpoint-(a) rulings — APPLIED

1. **Lane-M reference: pure-`assemble_root` + `assert_rom_matches` (empty allowlist).** APPLIED with
   the mandated present-tense comment (mixed_offcanonical_rom.rs:15-24: why `_convsym` is excluded —
   both sides pre-convsym; deb2 coverage stays with the canonical gates) + the empty allowlist as a
   HARD BAR (no allowlist entry needed — no STOP). ✓
2. **Window oracles KEPT.** APPLIED — z80_init_port/game_debug_port/vblank_mirror_shape_twin_parity
   stay; kill row 55 amended to record the whole-ROM gate landing WITHOUT retiring the window oracle. ✓
3. **Config B plain no-sound ONLY.** APPLIED — one gate (`mixed_z80_init_config_b`); a ledger tripwire
   jotted (debug+no-sound deliberately unexercised; the else-arm is shape-shared). ✓
4. **Harness home `mixed_offcanonical_rom.rs`.** APPLIED, with the `assemble_offcanonical` +
   parameterized `placed_emp` + `flatten_with_asserts` siblings. ✓

## Lane-M machinery (as built)

- Reference = `assemble_root(main.asm, config)` (no gate) → `flatten(0x00)`. Mixed =
  `assemble_root(main.asm, config + SIGIL_EMP_*)` (region `org`-skipped) + `.emp` placed at region
  start → `flatten(0x00)`. Compared via `assert_rom_matches` (empty allowlist). Drift-guard link
  asserts checked against the JOINT symbol table.
- Configs: A = `__DEBUG__`+`SOUND_DRIVER_ENABLED`+`SOUND_DEBUG_HOTKEYS`+`SOUND_DBG_MIRROR` (one shape,
  inherently debug — serves game_debug AND sound_debug in the combined gate, the t26 "one build serves
  both", ruled YES); B = no-sound (plain).
- Region pins (Config-specific sonic4-shape orgs, re-pin-tracked — kill row 58): game_debug
  [0x6356,0x6408) org $6408; sound_debug [0x81B0,0x827C) org $827C (ABOVE $8000 — the $8000 bar
  satisfied by construction: reference at the SAME config = one bank layout); z80_init [0x3D8,0x3FE)
  org $3FE + numeric `Z80_IDLE_SIZE = $3FE-$3D8` (the boot-cursor equate the .emp code doesn't carry).

## Lane-S port (as built)

`sound_debug.emp` = the t25 draft (aeon `0618dd4`) + three fixes P1+P2 unblocked:
1. `{stop_z80()}`/`{start_z80()}` (splice form, comptime-fn bodies only) → `stop_z80()`/`start_z80()`
   (statement-position comptime call, the vblank.emp:80 proc-body spelling). **THIRD gap, masked at
   t25 by the hang — a genuine new find (§4 #5).**
2. `clobbers(d0/d1/a0/a1)` → movem-RANGE `clobbers(d0-d1/a0-a1)` (step-2 item 5; the honest set).
3. `extern("SeqChannel_len")`/`extern("SND_SEQ_TRACE_LEN")` in the d16 displacement / moveq imm →
   `const` mirrors (SEQ_CHANNEL_LEN=60 / SND_SEQ_TRACE_LEN=32) + drift-guard ensures — a link-external
   extern cannot size a comptime displacement/immediate (the TILE_SIZE precedent, ledger row 1582
   predicted it exactly). The wrapped-paren PARSE is P2; the const is the FOLD — both needed.

External gate `SIGIL_EMP_SOUND_DEBUG` (engine.inc); zero bytes both canonical shapes. KILL ROW 42:
vblank.emp's extern decl deleted same-commit; the mirror-shape call resolves module-to-module. Proof:
`vblank_mirror_shape_twin_parity` (region bytes, decl deleted, synthetic carrier at the real VMA) +
`mixed_combined_config_a` (the byte-exact AS-vblank→emp-sound_debug cross-seam call in the whole
off-canonical ROM). DEVIATION: the fully-`.emp`-to-`.emp` flip test was attempted but the harness
section model blocks combining two separately-lowered `.emp` modules (a `sec0` collision); the
whole-ROM combined gate is the STRONGER byte-level cross-seam proof and stands in its place (noted
at kill row 42).

## PANEL ROUND (A1 + B1 + C2 + C3 — 4 lenses, read-only, one round; C3 ACTIVE per lane-S design)

**DRY STANDS** (t22 bar: adjudication yielded byte-neutral comment fixes + ledger notes; no code
logic change). **4/4 useful.**
- **A1 (cold reader):** the header's "SeqChannel grew 14 → **36** bytes" contradicted `SEQ_CHANNEL_LEN
  = 60` 40 lines down — a stale Phase-3 figure. FIXED (present-tense, no history narration). Logged the
  const-mirror language ask (already ledgered). Clobbers-in-header (vs the AS twin's compensating
  narration) noted as a WIN.
- **B1 (corpus):** CLEAN. The four copy loops are the ESTABLISHED inline byte-copy spelling
  (vdp_init/zx0/entity_window) — NOT a re-hand-roll of `clear_longs` (a CLEAR). **This RETIRES the
  step-4 `copy_bytes(n)` candidate: inline is the house norm, so step 4 is genuinely empty.** const-
  mirror = TILE_SIZE class; z80_bus reuse correct; harness follows convention (parameterized
  `placed_emp` cleaner than mixed_dac's per-tranche copies). One hoist-someday ledger note.
- **C2 (correctness):** CLEAN — every hardcoded number re-derived (SEQ_CHANNEL_LEN=60 [AS endstruct+
  assert], disp 60-20=40, moveq counts 48/16/8/3/20/32, write cursor 164≤176 no overrun, honest exact
  clobbers, no CC/dbf/loop hazard, Z80_IDLE_SIZE=38 verified, org values = correct region-end resume
  points). One stale .asm comment (cited the deleted extern decl) → FIXED.
- **C3 (hardware/Z80-bus):** bus request/grant/release VERIFIED CLEAN (paired stop/start, single `rts`
  exit, grant-spin blocks before any read, no contention). The "~190µs" figure is a static-estimate
  underestimate (~several hundred µs) — a descriptive, non-load-bearing comment; SOFTENED to not
  assert an unverified number (oracle-measurable, not measured).

## PER-PASS: step-3 vs step-5

- **Pass 0-2 (lanes P/M/S):** the three parser fixes (each root-caused by probe/instrumentation, all
  correcting a brief/note framing — §4); the DEMAND-3 machinery; the sound_debug port + the const-
  mirror finding + kill row 42. *step-5:* no changes (debug-only human-timescale mirror path; C1
  inactive-recorded, the t26 game_debug precedent).
- **Pass 1 (loop on sound_debug.emp):** *3(a):* the const-mirror-for-comptime-length ask (ledgered,
  TILE_SIZE class). *3(b):* comments carried present-tense; clobbers match body. *4:* the copy-loop
  construct candidate — DEFERRED, then RETIRED by B1 (inline is corpus convention). *5:* no changes
  (debug path). → dry claim → panel.
- **Panel:** comment fixes only (A1/C3/C2) + ledger notes (B1) → DRY STANDS.

## NEITHER-BUCKET HEADLINES

- **The corpus's LAST `extern proc` is closed.** After kill row 42 (Sound_DebugMirror), ZERO
  `extern proc` decls remain — the t26 census's final member. The honest leaf contract carried exactly.
- **Every one of lane P's fixes corrected a brief/note framing** (5 corrections, §4) — P1 wasn't
  "deeper in operand/expr" (it was the top-level recovery loop), P2 wasn't extern-specific (it was the
  wrapped-paren form), and the sound_debug draft had a THIRD gap (the `{stop_z80()}` splice) the t25
  hang had masked. Probes/instrumentation did their job; "prove before you lean on it" paid off.
- **P3 fixed at the eval layer, not the parser** (overseer-endorsed) — `eval_path` is where the two
  parser AST forms converge, so the byte-proven disp/pc-rel/branch paths stay untouched; byte-neutral
  across the whole corpus (census: 0 live instances).
- **The DEMAND-3 machinery is a clean lift of the window oracles** — pure-`assemble_root` reference +
  empty-allowlist `assert_rom_matches`; three consumers / two configs, all green by name, no
  speculative configs (LEAN bar met).
- **The const-mirror finding validates the ledger's foresight** — row 1582 named the const-mirror
  workaround before the port; the `SEQ_CHANNEL_LEN=36` mis-guess made the drift guard FIRE (a live
  negative control), and 60 (the AS `endstruct` + `if <> 60` assert) resolved it.

## CORRECTIONS LIST (brief/overseer/note errors caught — tree wins, overseer accepted all five)

| claimed | true | class |
|---|---|---|
| sigil master `ad60753` (brief §0) | `e48d4f2` (the brief's own commit) | stale (cosmetic) |
| "the TWO t25 parser-hang repros are `#[ignore]`d" | ONE; the other is `sigil_diff_reports_byte_identity` (unrelated) | brief count error |
| P1 "the loop lives deeper in operand/expr parse" | top-level `recover_to_next_decl` (contextual `extern`) | brief/note root-cause error |
| P2 "extern in a lea displacement" | the WRAPPED-PAREN `(disp)(An)` form (extern incidental) | brief/ledger framing |
| (new) the sound_debug draft's `{stop_z80()}` splice | invalid in a proc body (masked by the t25 hang) | genuine new find (credited) |

## OPEN AT MERGE (ledger duties done)

Kill row 42 KILLED (last extern); rows 55 amended (ruling 2), 58 (off-canonical org pins) + 59
(sound_debug twin + const mirrors) born. Ledger: P1/P2/P3 CLOSED (with root-cause corrections);
DEMAND-3 machinery LANDED; Config-B debug+no-sound tripwire (ruling 3); the const-mirror TILE_SIZE-
class row; the B1 hoist-someday note. Row 5 (AS twins) implicitly gains sound_debug.asm (via row 59).
Standing after t28 (roadmap-owned): Z80 rung 2 (psg→fm); the banked-head seam sub-tranche; the game-
side ~20 (needs the durable census file). PROVENANCE re-baseline is a formality (byte-neutral).
