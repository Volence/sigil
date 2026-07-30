# 2026-07-30 — seam-2 stage-1: the f828406-oracle RE-BASELINE (byte-neutral, GREEN)

Status: **EXECUTION stage 1 (the finisher's FIRST duty) — the stale-pin
re-baseline audit. BYTE-NEUTRAL: no ROM-emitting code touched; the whole edit
is comment prose + one test's map/expectation pins moved to the current truth.
Provenance UNCHANGED.** Sigil branch `seam2-banked-data`, aeon branch
`seam2-banked-data`. Follows porter-1's step-2a handoff (the STALE-PIN FINDING).

## The verified current-baseline truth (from aeon's authoritative `s4.lst`)

Every address below was read from `s4.lst` / `s4.debug.lst`, NOT from any header
or oracle pin. The WHOLE sound-bank region sits one `$8000` bank LOWER than the
stale "aeon-f828406" headers claimed (a uniform −$8000 shift):

| symbol | current LMA | bank id | stale header claimed |
|---|---|---|---|
| `Dac_Temp_Blip` (`dac_blip_bank`) | `$48000` | `$9` | `$50000` / `$A` |
| `Dac_Kick` (`dac_shared_bank`) | `$50000` | `$A` | `$58000` / `$B` |
| shared-bank end | `$578BC` | — | — |
| `MovingTrucks_Bank_Start` (head) | `$58000` | `$B` | `$60000` / `$C` |
| `Song_MovingTrucks` (`mt_bank` body) | `$58607` | `$B` | `$60607` |
| `SongPatchTable_End` / `Sfx_33` (`sfx_bank`) plain | `$5BAE8` | `$B` | `$63AE8` |
| `Sfx_33` (`sfx_bank`) debug | `$5D53A` | `$B` | `$6553A` |
| `SfxTable_End` plain / debug | `$5C230` / `$5DC82` | — | `$64230` / `$65C82` |
| `SfxBlobWinTab` (phase `$845F` → head) | ROM `$5845F` | — | `$6045F` |

The PTR/LEN halves of every `SND_*` triple are PLACEMENT-INVARIANT
(window-relative `(L & $7FFF) | $8000` / comptime `.len`) — they were already
correct and unchanged. Only the BANK ids moved (`$A/$B` → `$9/$A`).

## What this commit changed (byte-neutral)

**Sigil worktree:**
- `crates/sigil-cli/tests/dac_port.rs` — the exemplar SND_* fold oracle
  RE-BASELINED: map pins `$50000/$58000 → $48000/$50000`; the 10 `*_BANK`
  expectations `$A/$B → $9/$A`; header + falsification prose to current truth.
  It stays self-consistently GREEN (it folds `bankid()` from the placed address,
  so moving the map + the expectation together keeps it a true fold-proof — now
  of the CURRENT ROM, not the f828406 one). The seam2_dac_emit.rs payload gate
  independently pins the emitted banks to the real `s4.bin` slices.
- `crates/sigil-harness/tests/mixed_dac_rom.rs` — CODE was already current
  (green in strict); only its stale top DOC-COMMENTS (the composition prose, the
  gap-fill block `$50B40/$5F8BC/$60000/$4867A`, the two map doc-blocks, the
  win-tab comment `$6045F/$63AE8`) rewritten to the current addresses.
- `crates/sigil-harness/src/lib.rs` — the `assemble_mixed_mt/sfx_as_side` doc
  comments' stale org-resume cites (`$6553A/$63AE8 → $5D53A/$5BAE8`,
  `$64230/$65C82 → $5C230/$5DC82`). `REGION_B_LMA` const was already `$58000`.

**Aeon worktree (pure comment prose — byte-neutral to both ROMs):**
- `dac_samples.emp` header — `$50000/$58000, $A/$B → $48000/$50000, $9/$A`.
- `mt_bank.emp` header — `Song_MovingTrucks $60607 → $58607`; head window
  `$60000..$60607 → $58000..$58607`; region `$60607..$68000 → $58607..$60000`;
  the DEBUG `_hcz2_align` pad ROM byte `$654A1 → $5D4A1`.
- `sfx/sfx_bank.emp` header — `$63AE8/$6553A → $5BAE8/$5D53A`.

## The mt/sfx pre-emit pin AUDIT (finisher duty b) — DONE

Verified every reference-pinned banked oracle against `s4.lst`. FINDING: the
reference-pinned gates were ALL already current — no false-green trap remains on
the banked side:
- `mt_port.rs` — reference-pins `s4.bin[0x58607..0x5BAE8]` / debug
  `[0x58607..0x5D53A]`. CURRENT. ✓
- `sfx_port.rs` — reference-pins `[0x5BAE8..0x5C230]` / `[0x5D53A..0x5DC82]`.
  CURRENT. ✓
- `mixed_dac_rom.rs` CODE (incl. the win-tab assert `rom[0x5845F..]`). CURRENT. ✓
- `seam2_dac_emit.rs` — pins `$48000` blip / `$50000` shared. CURRENT. ✓

## LEDGER — the SYNTHETIC (non-reference-pinned) stale oracles owed a re-baseline

These are NOT a false-green risk (they are self-consistent structural probes /
frontend fold unit tests, green regardless of absolute address; none claims to
match the reference ROM). They carry stale "the real `$60607`/`$63AE8`, bank $C"
claims and belong with the stages that actively re-prove them:

- `crates/sigil-cli/tests/mt_negative_probes.rs` — doctored placement at `$60607`
  bank $C, cross-seam label at `$58000` "wrong bank $B". At the current baseline
  `$58000` IS the real head bank ($B), so the doctored-vs-real semantics must be
  re-derived. **Owed at stage 2c (mt_bank emit).**
- `crates/sigil-cli/tests/sfx_negative_probes.rs` — same shape at `$63AE8` /
  "real `$60000` bank $C". **Owed at stage 2d (sfx_bank emit).**
- `crates/sigil-frontend-as/tests/partial_fold_defer.rs` — uses `$63AE8` as an
  arbitrary ExtSym to unit-test the `sfx_winptr` fold `(L & $7FFF) | $8000`. The
  value is arbitrary (any address folds); re-baseline to `$5BAE8` for parity when
  2d lands. Not stale in a correctness sense.

## Provenance (UNCHANGED — this is a byte-neutral commit)

- Emitted blob plain `c7534c84` / debug `fd2a845d` · syms `87b87b1b`.
- Assembled ROM plain `e5765873` / debug `dab4f06c` (proven by
  `mixed_seam1_rom_matches_reference_{plain,debug}` + `mixed_dac_rom`'s 52 gates).
- Artifact full-file `22f69f77/414414` · `d4e8d043/422466`.
- Strict **2884 passed / 0 failed / 1 ignored** (full workspace, `--no-fail-fast`,
  SIGIL_STRICT_GATE=1 + SIGIL_EMIT + AEON_DIR at these worktrees) — unchanged from
  the branch-tip baseline.

## Next (the finisher's remaining stages)

- **2b:** wire the DAC emit into `emit_sound_blob` + build.sh + main.asm BINCLUDE;
  dual-prove; delete `dac_samples.asm` (kill row 5 dac arm) standalone.
- **2c/2d:** `mt_bank` then `sfx_bank` emits — re-baseline the two negative-probe
  oracles (above) as part of each; delete the `.asm` includes.
- **3:** the phased head (`seq_opcode_tab`/`dac_sample_tab`) + `sound_tables_z80`
  generator-emits-`.emp`.
- **4/5:** remaining retirements + loop + C3-heavy dry panel → checkpoint (b).

## STAGE 2b READINESS — the SND_* coupling (a design decision the next porter owes)

Scoping 2b surfaced a coupling the one-line "wire + delete dac_samples.asm"
summary hides: **`dac_samples.asm` DEFINES the 30 `SND_*` equs, and
`engine/sound/dac_sample_tab.asm` (the still-AS head descriptor) CONSUMES them**
(grep-confirmed: those two files are the only readers). `dac_sample_tab.asm` is
included UNCONDITIONALLY via `engine/sound/sound_bank.inc:37` (inside the
`soundBankHead` phase-head) — it has NO `SIGIL_EMP_*` gate yet. So deleting the
DAC body at 2b STRANDS the AS head, which needs `SND_*` at assemble time.

The build wiring facts (verified):
- `build.sh:87` runs `${SIGIL_EMIT} --aeon . --out-dir engine/sound/generated`
  (the resident-blob emit; extend THIS binary to also drop the DAC bank .bins).
- The seam-1 BINCLUDE pattern to mirror is `boot_data.asm:69-75`:
  `Z80_Sound_Start:` then `ifdef __DEBUG__ / BINCLUDE ..._debug.bin / else /
  BINCLUDE ....bin / endif` — unconditional, NO `.asm` fallback. DAC is
  shape-INVARIANT (one blip + one shared, no `-D`), so no `_debug` variant.
- The current `SIGIL_EMP_DAC` arm (main.asm:312-320) only does `org $58000`
  (the .emp banks are supplied in-memory by the TEST harness today, not the real
  build). 2b must turn that arm into the real `align $8000` + `Dac_Temp_Blip:
  BINCLUDE dac_blip_bank.bin` + `align $8000` + `Dac_SharedBank_Start: BINCLUDE
  dac_shared_bank.bin` + `align $8000` (to $58000).
- `dac_samples.asm` structure (the align/label/BINCLUDE order to reproduce
  byte-for-byte): `align $8000` → `Dac_Temp_Blip:` → blip → `align $8000` →
  `Dac_SharedBank_Start:/Dac_Kick:` → the 9 drums → (MT arm's `align $8000`).

**The `SND_*` decision (needs the overseer or an autonomous call):**
- **Option X — the syms bridge (seam-1 precedent, design §3c):** extend the
  emitter to ALSO emit a `dac_sample_syms.asm` contract of the 30 `SND_*` equ
  values (folded from `dac_samples.emp` placed at $48000/$50000 — `emit_dac_banks`
  already places there; add the equ export). `dac_sample_tab.asm` includes that
  contract; `dac_samples.asm` deleted. Keeps 2b body-only, head stays AS. The
  contract PINS the LMAs, so the AS `align $8000` BINCLUDE must land the banks at
  EXACTLY $48000/$50000 or the assembled-ROM CRC gate fires (the false-green
  guard). This is the clean seam-1-shaped intermediate — RECOMMENDED.
- **Option Y — co-link now (design §2d endpoint):** pull the stage-3
  `dac_sample_tab` → `.emp` conversion forward and co-link, so `SND_*` VANISHES
  (folds from `bankid/winptr/.len`). Bigger; merges 2b+part-of-3. Requires
  un-suppressing `dac_samples.emp`'s per-sample start labels (design OQ-4 — the
  header says they're "deliberately not ported, zero external consumers";
  `dac_sample_tab.emp` becomes the first consumer).

Recommendation: Option X for 2b (self-contained body deletion + syms bridge),
then Option Y folds the bridge away at stage 3 when `dac_sample_tab` goes native
(the bridge's kill condition). The finisher-stage-2 line "kill row 57 +
row-5's dac arm" conflates the two: row 5 (dac_samples body) closes at 2b under
Option X; row 57 (dac_sample_tab) closes at stage 3 — NOT the same commit unless
Option Y is chosen. Flag for the gate.

## VALVE STOP — clean boundary

Stopped here (Stage 1 complete, green, committed both branches; 2884/0/1) rather
than rush the coupled 2b sub-problem above. Handoff is honest and complete: the
current-baseline address truth table, the mt/sfx audit result (reference oracles
already current), the deferred synthetic-oracle ledger, and the 2b coupling
decision are all recorded for the next porter.
