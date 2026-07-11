# .emp ergonomics audit — the converted files, read for "good to code in" (2026-07-11)

Volence's ask (same session as the `table` ratification): beyond `table`, what other
macro/idiom constructs should we consider — grounded in the converted `.emp` files, which must
feel safe, nice, and above all good to code in. Method: three parallel auditors read all 24
converted files (object spine / system+sound / game data) with an ergonomics-only lens; findings
cross-referenced against the campaign gap-ledger and the frozen spec's rulings. This note is the
triage; rulings recorded inline.

## The headline

Three-quarters of what "feels bad" in the converted files is **not missing language**. It sorts
into four buckets:

- **A. Twin tax** — scaffolding that dies on schedule (kill list / Spec 5). Don't build for it.
- **B. Table-shaped** — more instances of the construct we just designed; fold in as v1.1 knobs.
- **C. Real language asks** — mostly small, several already ledgered with demand counts.
- **D. Anti-tenet suggestions** — auditors proposed them, the spec already rejects the class.

## A. Twin tax — dies on schedule, do NOT build language for it

The single biggest line-count offender across all three audits: **mirrored const + `ensure`
drift-guard pairs** (41 in constants.emp, 30 in sst.emp, 4 in rings.emp, 7 in sound_api.emp,
6 in the sound data files). Every one is a row on the twin-scaffolding kill list — they exist
because the `.asm` twin still owns the truth, and they die at the constants ownership flip
(kill row 1) / struct ports / Spec 5. A `mirror_const` sugar would be **building surface for
scheduled demolition**. Same verdict for: the assert TRANSLITERATION blocks (kill row 16,
demand 1/2 — the real construct waits for the debugger.asm port era), the RAM-address ownership
comments (die when ram.asm ports to §4.6 `vars`), and the re-pin/PROVENANCE headers (harness
process, not language). **The fix for bucket A is finishing the campaign.**

Also bucket-A-adjacent: dplc's twin procs and animate's duplicated interpreter machinery are
already RECORDED as lockstep-blocked (a comptime-fn dedup diverges the twin shapes) — deferred
until the twins die. The auditors independently rediscovered both; no new ruling needed.

## B. Table-shaped — fold into the `table` construct as v1.1 knobs

The data-files auditor found that **dac_samples.emp and mt_bank.emp are also `table`-shaped**,
each demanding one knob the sfx_bank design didn't need:

1. **Derived per-row exports** — dac_samples' 10 samples × 3 hand-written equs
   (`SND_X_BANK = bankid(...)`, `SND_X_PTR = winptr(...)`, `SND_X_LEN = Blob.len` — 30
   mechanical lines). A `exports: (bank: bankid, ptr: winptr, len: .len)`-style knob (naming
   scheme per row) collapses them. The equs are the cross-seam contract, so they must stay
   REAL `pub equ` link symbols — same machinery, generated.
2. **Column emission (#soa)** — mt_bank's SongTable/SongPatchTable are two parallel `[*u8; N]`
   arrays married only by a comment ("parallel"). One row per song with two pointer fields +
   a `columns` emission mode (each field emits as its own array) makes the parity structural.
   This is research item T3-i (Odin `#soa`) landing on its first real demand.
3. **Per-row predicate** — dac_samples' 10 identical `ensure(0 < B.len && B.len < $8000)` lines;
   a row-predicate knob (`each: |b| ...`) runs the check per row with a generated message.

Verdict: none of these are new constructs — they're `table`'s second and third acceptance
targets. When `table` is scheduled, dac_samples + mt_bank retrofits ride along (byte-neutral,
same gates), and the knobs get designed against them the way `cell:`/`hole:` was designed
against sfx_bank.

## C. Real language asks — ranked

**C1. The quality-of-life batch (small, cheap, clearly right — candidate mini-tranche):**
- **Label values in immediate exprs** + `use`-import of offsets/table labels as values
  (ledger, tranche 6 ×2): kills the self-extern ceremony
  (`equ SOLID_ROUTINE_MAIN = extern("TestSolid_Main") - extern("ObjCodeBase")`) that EVERY
  object port pays once per routine store. The single highest-frequency future tax.
- **`clobbers()` register ranges** (ledger, 3 data points): TouchResponse spelled twelve
  registers comma-by-comma while `preserves(d0-d1/a0)` takes reglists. One grammar for both.
- **`bankid`/`winptr` bareword arguments**: the string spelling (`bankid("Sfx_33")`) loses
  rename-refactor + typo checking; a bareword call argument already becomes a deferred link
  symbol elsewhere (the sfx_bank header's own idiom note). Verify + unify.
- **equ hygiene** (ledger, tranche 6): non-pub equs are link-global — modules carry hand
  prefixes (SOLID_/PARTICLE_) to dodge collisions. Mangle like non-pub procs.
- **Unexported-label hint diagnostic** (ledger, tranche 9, 1 data point): cheap teaching fix.
- **`clobbers()` entry validation** (ledger): typo'd registers silently accepted.

**C2. Local typed-register binding (`let a2: *Sst`)** — the strongest single new-surface ask.
TouchResponse pays the qualified `Sst.field(aN)` spelling at 13 sites because only proc PARAMS
get typed registers; every proc that self-loads a pointer inherits the tax. Ledgered tranche 7;
the audits confirm it reads as the files' most visible per-line ceremony. Worth ratifying as a
design item now (grammar + scope rules are the work; erasing, byte-neutral).

**C3. Ledgered demand-gated items the audits re-confirmed** (no action until trigger):
record-view over raw RAM (demand 1/2 — entity_window ratifies; rings' ×6 index chains are its
poster child), `branch_table` dispatch encoding (consumer-gated), `distinct()` template-param
predicate, force-width byte-lock idiom (`bne.w!` / twin-parity lint), shared engine-macros
module for stop_z80/start_z80-class templates (demand-gated on a second file).

**C4. `critical_section` / SR+bus-hold wrapper** (sound_api ×4): the INVARIANT half (stopZ80/
disableInts pairing, SR balance) is S2-D7's ledgered lint slice — `preserves(sr)` + its balance
check already shipped as the first slice. The SURFACE half (a body wrapper) is prelude/comptime-fn
territory, not grammar, and pays twin-lockstep tax in ported files today. Verdict: keep riding
S2-D7; no new construct.

## D. Rejected by existing rulings (recorded so nobody re-litigates)

- **Auto-generated movem from `preserves()`** — the compiler never inserts invisible
  instructions (tenet 3 / the D2.29 "bytes are the author's" principle). The CHECK is the
  feature (preserves-verification, shipped), not the codegen.
- **`zext.b_load` / `mul_index` / `store_le16` instruction-replacing helpers** — NESHLA-class
  control-of-the-instruction-stream, rejected in research Part III; the sanctioned shape is
  comptime-fn templates (aabb_axis_test precedent), which are prelude content, demand-gated,
  and mostly lockstep-blocked in ported files until Spec 5. For NEW code they're a prelude
  question, not grammar.
- **Loosening struct `default` elision** (act_descriptor's 7-of-16 `default` fields) — Volence
  RULED named-per-field at the D2.31 checkpoint (bulk `..` retired with a teaching error).
  Settled; the visible `field: default` IS the design.
- **`@drift_guard_to` auto-ensures on offsets** — the guards it would generate are bucket-A twin
  scaffolding (sonic_anims' 11 ensures die at the ANIM ownership flip, kill row 4).

## Recommendation to Volence

1. **Schedule the C1 quality-of-life batch** as a small implementation tranche (most items are
   days-not-weeks, several are pure grammar/diagnostics) — it removes the per-port taxes every
   FUTURE tranche pays, so it compounds.
2. **Ratify C2 (`let aN: *Type` body bindings)** as a design item — one design note, same
   pattern as `table`.
3. **Fold bucket B into `table`'s implementation scope** — dac_samples + mt_bank as acceptance
   targets #2/#3 alongside sfx_bank.
4. **Leave buckets A and D alone** — A dies with the campaign, D is settled.
