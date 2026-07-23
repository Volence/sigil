# t18 — TRANCHE CLOSE PACKET (parallax port + HBlank trampoline)

**Milestone:** the last big un-ported hot engine domain (parallax) + row 1088
(HBlank RAM-jmp trampoline). PROVENANCE-re-baselining tranche (byte-changing).
Branch tips at close: **aeon-t18 `9cd715c` / sigil-t18 `b240efa`**.
**New canonical: plain `00f609a5`/421089 · debug `80d14183`/429134.**
Full paired strict **2488/0**. Dry-panel debut: DRY (one gate-blind catch → parcel).

## Scoreboard

| Workstream | Outcome |
|---|---|
| **parallax.emp** (fresh port, 11 procs, no prior twin) | byte-identical transcribe → modernized → GridX/GridY typed → H2 perf cut (−1924 cyc/f) |
| **HBlank trampoline** (row 1088) | built + oracle 5/5 live-verify; shadow-coherence proven (self + C3) |
| **Hscroll_Dirty deletion** (Phase-2.5 rider) | PAD-ruled, executed |
| **demanded sigil-core feature** | `[lower.abs-sym-operand]` abs-sym+(d16,An), TDD shipped |
| **dry-panel debut** | 5 lenses; B6 real bug → parcel; HAZARD-2 refuted; 3 folds; 6 ledger rows |

Bugs found: **B6** (CC-clobber promote-frame skip, dry-panel) + **B1/B2/B3**
(transition-logic, step-3(b)) — all faithfully-ported, all → post-merge parcel.
Perf: **H2** −1924 cyc/f live-proven; 6 candidates skip-logged with numbers.

---

## PER-PASS: step-3 (retrospect / correctness / language-ask) vs step-5 (perf)

**Pass — parallax.emp transcribe+modernize (steps 1-2):**
- *step-3:* codename cleanup (present-tense); GridX/GridY type-layer bless at
  CheckBoundary (MOVED+COMPARED into typed Section_GetSecPtrXY); honest contracts
  surfaced by the closure gate; the `(Vscroll_Factor).w` structural pin (kept, with
  its exception comment — demanded-feature successor gap ledgered).
- *step-5:* branch modernization relaxed 6 conservative-.w→.s (asl fixpoint) — a
  size win, not a hot-path cycle win.

**Pass — parallax.emp loop (steps 3(b)/5):**
- *step-3(b):* B1 (re-cross no-cancel), B2 (mode/data disagreement), B3 (~36% snap
  pop — math-proven, false "~95%" comment fixed byte-neutral) — all SPLIT
  (not live-verifiable in-tranche: need a boundary crossing). d6-across-CheckBoundary
  audited sound-but-latent-fragile.
- *step-5:* profiler-first (Fill_PerLine 5832 cyc/f). **H2 flat-fill 8x unroll CUT**
  (×8-span guarantee verified all paths) — **live A/B 5832→3908 = −1924/f (−33%)**.
  Six candidates (H3 design-item, M1<400, M2/M5 off-flat-path, M3 ~400-500, M4 ~80)
  skip-logged with numbers.

**Pass — HBlank trampoline (row 1088, byte-changing):**
- *step-3:* interrupt-transparency contract (Q3, not blanket movem); shadow-coherence
  binding (Q1 #3) built from first cut; RAM-tail slot placement (zero existing-RAM
  churn — improvement over the note's in-place shift, logged); byte-neutral boot.
- *step-5:* the trampoline IS the perf item — it kills the ~180-cyc/HInt ROM-dispatch
  wrapper (no live raster consumer yet, so realized value is future; mechanism
  oracle-proven 5/5).

**Pass — dry-panel (5 lenses):**
- *step-3-flavored (C2/A1/B1/C3):* B6 (gate-blind CC-clobber → parcel); 3 byte-neutral
  contract/comment folds; language-asks + consolidation debt + drift gaps ledgered;
  HAZARD-2 refuted.
- *step-5-flavored (C1):* CLEAN — no missed ≥1k (H2 was right & only cut).

---

## NEITHER-BUCKET HEADLINES (not step-3, not step-5)

- **A demanded sigil-core feature shipped mid-tranche:** `[lower.abs-sym-operand]`
  (abs-sym + trailing `(d16,An)`), TDD, sentinel-relower fixup locator — parallax
  was the first consumer. A language capability, not a port finding.
- **An idiom-level assembler catch:** **AS `^` is EXPONENTIATION, not XOR** — it
  silently emitted `$00` for an IE1-clear mask; caught by the hblank byte gate at
  region +0x39; both twins now use `~`. `[as-syntax.caret-is-power]` lint candidate
  ledgered. (The byte gate is a backstop, not a guarantee — power/XOR can coincide.)
- **Two oracle-driven live proofs:** trampoline synthetic-handler 5/5 (emulated
  install/uninstall via slot+shadow pokes — no 68k register-write tool); H2 profiler
  A/B. Both on the plain OJZ bg scene.
- **The dry-panel debut itself:** the mechanism caught a gate-blind bug the whole
  loop missed AND self-corrected a false positive by adversarial verification —
  precedent set that a real finding yielding no in-tranche work does not re-open the
  loop (gate ruling).
- **A lint-net class extended:** B6 = the PATH-SENSITIVE variant of
  `[branch.condition-constant]` (would-be catch #3) — a Bcc whose reaching CCR-writers
  disagree in kind. Extension candidate ledgered; the live lint correctly stayed
  silent (3-path join, zero-FP guard held).

---

## POST-MERGE QUEUE (for the record)
- **Boundary-crossing transition parcel** (one gated parcel, crossing-drive built
  once): (0) B6 trivial flag fix → (1) window-slide observation → (2) B2 mode-contract
  [defines the state machine] → (3) B3 ramp → (4) B1 cancel. Order at Volence's word.
- **item-13 impl brief** (wave-1, ratified 7b9afa6).
- Byte-neutral consolidations: VDP_DATA/VDP_CTRL + `set_vdp_reg` comptime-fn home
  (4 sites); language-ask constructs (Decode dedup, `rept N {}`, `ensure_layout`).
