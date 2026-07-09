# Next-session handoff — sound-migration T3 (the SFX tranche)

Written 2026-07-08 (Fable, end of the T2 session) for the next session. **T2 is MERGED &
PUSHED** (sigil master `736e7f8`, aeon master `4710ef3`); T3 is the LAST sound-data tranche.
Unlike T2, T3 needs a **SHORT design-check with Volence BEFORE the plan** (one real open
question, below).

## Where everything stands

- **T2 merged**: mt_bank.emp + SIGIL_EMP_MT gate + comptime defines (`-D`) + imm32 deferral;
  mixed DAC+MT ROM byte-identical both shapes; workspace 1466/0 strict-gate. Read
  `notes/2026-07-08-sound-migration-t2-complete.md` + the plan's Execution notes
  (`plans/2026-07-08-sound-migration-t2-mt-bank.md`, rulings R1–R7) for every pattern and
  deviation — T3 reuses nearly all of it.
- **The twin-drift guard is live**: aeon `build.sh` now runs `tools/verify_emit_bin.py`
  (~26 ms) before assembling; a regen that updates a generated `.asm` without its `.bin` twin
  fails the build. NOTE: `sfx_table.asm` is deliberately EXCLUDED from the verifier (it's a
  `dc.l` pointer table, no byte-payload twin).
- **⚠ STILL PENDING: the forest-bg boot-check.** Aeon's working tree still carries the
  UNCOMMITTED bg-restore files (+ untracked s3k_art_style_demo.html, sprites/plantbadmaps/) —
  Volence approved the T2 merge but has NOT yet boot-checked the bg. The reference ROMs
  (s4.bin `8ce6dd7e…`, s4.debug.bin `13c7b063…`) INCLUDE the bg content, so the sigil gates
  depend on that working tree staying put until it's committed. Nag for the boot-check +
  commit early. (Handoff w/ details: `notes/2026-07-08-next-session-handoff-bg-restore-then-t2.md`.)
- Memory file [[spec2-progress]] is current through the T2 merge.
- Other open ledger items (unchanged): empyrean working tree holds uncommitted #7/D2.25 +
  D2.26 spec integrations (Volence commits); Fable now ALSO owes the T2 spec-integration pass
  (defines incl. the global-reserved-names note, imm32 deferral, the bare-cross-seam-symbol
  language gap — S2-D14-adjacent); collision stays PINNED.

## T3 scope (from DSM.8)

Port the SFX block to `.emp`: the 9 SFX blobs + their patch banks (18 generated files, all
verify_emit_bin-covered with `.bin` twins already emitted), `sfx_table`, and the two SFX
fatals — now at aeon `main.asm:252` (straddle) and `main.asm:260` (co-residency with
`SND_ENGINE_TABLE_BANK`) — as ensures. Plus the DSM.7 win-tab call (below).

Layout facts (from the current s4.lst; re-derive per shape before pinning): the SFX block
runs `Sfx_33`..`Sfx_B9_Patches_End` = `$63AE8..$64014` plain / `$6553A..$65A66` debug — the
whole block SHIFTS between shapes (it sits after the shape-dependent song tables), unlike
T2's fixed streams. Per-shape map pins / org resumes, same mechanics as T2's gate else-arm.

## The design-check question (hold with Volence BEFORE planning)

**`sfx_table.asm` is regenerated at EVERY build** (prebuild → `sfx_transcode.py generate`,
games/sonic4/prebuild.sh:78) — the only build-generated file in the sound tree (songs are
manually regenerated, outputs committed). If the table ports to `.emp`, the options are:
  (a) the generator emits/refreshes an `.emp` table alongside (generator change, keeps
      today's auto-freshness);
  (b) the table becomes HAND-OWNED `.emp` (like SongTable — it's 9 stable entries; drop the
      per-build regen for the table only, keep it for blobs; a comptime ensure pins
      blob-count == table-count so a new SFX fails loud);
  (c) the table stays `.asm` this arc (only blobs+patches port; smallest scope, but the
      table is exactly the symbol-difference-free `dc.l` shape `.emp` already proves).
Fable's read: (b) fits the house taste (hand-owned contract, generated payloads) — but the
per-build regen was presumably chosen for a reason; Volence's call.

## T3 wrinkles (verified against live files)

1. **`sfx_blob_win_tab.asm` lives INSIDE the `.asm` phase head** (`soundBankHead` macro arg,
   main.asm:147). DSM.7 pre-ruled: if inseparable from the head, it STAYS `.asm` this arc.
   Its `dw` entries are winptr-style exprs over SFX blob labels — which will become
   `.emp`-defined. The T0 dw-deferral handles `.asm`-side `dw` reading `.emp` symbols
   (compound exprs included), so staying `.asm` should Just Work — but this is exactly the
   cross-seam surface to PROBE FIRST (T2's Task-2 pattern: prove before porting).
2. **Bare cross-seam equ reads still don't exist** in `.emp` exprs — use the
   `bankid("Label")` idiom (T2's mt_bank.emp deviation, documented there). The co-residency
   fatal compares `Sfx_33>>15` to `SND_ENGINE_TABLE_BANK` → spell it
   `bankid("Sfx_33") == bankid("MovingTrucks_Bank_Start")` (or by then, close the language
   gap — spec-integration candidate).
3. **The SFX gate org-resume target**: after the SFX block comes whatever follows in
   main.asm — derive the resume address per shape from the .lst at planning time (it's the
   content after sfx_table; check what's next in the include order past main.asm:246).
4. sfx_table is a `dc.l` table → in `.emp` it's the same `[*u8; N]` shape as SongTable
   (proven). PSG-only SFX have EMPTY patch banks (zero-length data items — proven by T2's
   P1 probe).
5. The 18 blob/patch `.bin` twins are still GITIGNORED (T2 un-ignored only its six) —
   T3 repeats the narrow-exception + verifier step for its files.

## Process

The T2 pattern verbatim: design-check → plan doc with frozen rulings (worktree off sigil
master) → capability probes first → aeon prep (gate `SIGIL_EMP_SFX`, byte-verify both
shapes) → the port → region byte-gate → mixed full-ROM acceptance (extend the harness — a
third gate define; keep DAC-only and DAC+MT tests intact) → negative probes → two-pronged
whole-branch adversarial review → Volence merge checkpoint. TDD with recorded RED
throughout; subagent-driven with two-stage reviews on load-bearing tasks.

After T3: the sound-data migration arc is DONE → re-evaluate S2-D14(a)(d)(e) + 9d against
what the arc demanded, then the remaining Plan-7 items (#10 compression builtins) → spec
FREEZE → the wider 68k migration campaign.
