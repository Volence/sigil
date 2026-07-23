# t18 — DRY-PANEL DEBUT: packet + adjudication (porting agent → gate)

The campaign's first dry-panel. 5 fresh read-only lenses over the 2 byte-changing
t18 workstreams (parallax.emp port + hblank.emp trampoline), per the ratified
weighting **A1 + B1 + C1 + C2 + C3** (C3 active — VDP/IRQ file). Panel reports,
never edits; findings adjudicated below; gate ratifies.

**HEADLINE: the panel is NOT dry — C2 caught a REAL gate-blind bug (B6). One C2
hazard REFUTED on verification. Everything else = byte-neutral fold / ledger.**

---

## THE MARQUEE CATCH — B6 (C2 HAZARD 1): CC-clobber skips the parallax rebuild on every transition-completion frame

**CONFIRMED REAL, gate-blind.** `Parallax_Update` promote path (parallax.emp
:361-364, twin parallax.asm :229-233 byte-identical):
```
move.l  Parallax_Target_Config, d0      // intended flag source (Z = target==0)
move.l  d0, Parallax_Current_Config
move.l  #0, Parallax_Target_Config      // MOVE sets Z from SOURCE (imm 0) → Z=1 ALWAYS
jbra    .config_resolved
.config_resolved:
beq     .no_config                      // reads Z=1 → ALWAYS taken on the promote path
movea.l d0, a0                          // (the intended fall-through — skipped)
```
On the 68000 a `MOVE` sets N/Z from its source. The promote path's last flag
setter is `move.l #0,…` → Z=1, so `.config_resolved: beq .no_config` is
unconditionally taken **only on the promote frame** — the whole parallax rebuild
(Step5+Step4+fill) is skipped, and Hscroll/Vscroll keep the previous frame's
contents. Result: a one-frame parallax freeze at the end of every smooth
(`pcfg_transition=0`) config crossing. The `.use_target`/`.use_current` paths
correctly end with `move.l <mem>,d0` (Z from d0), so ONLY the promote path bites.
Byte gate blind (twin-identical, verified — a faithful-but-wrong port).

**DISPOSITION — SPLIT into the post-merge transition parcel (joins B1/B2/B3).**
Same three reasons: (1) faithfully-ported shipped behavior (a behavior fix rides
its own gated parcel + live proof, never a port merge — gate's standing rule);
(2) not live-verifiable in-tranche (a transition-completion frame needs a
section-boundary crossing with a config change, which the static scene can't
drive); (3) byte-changing. **Fix is trivial** (re-establish the flag: `tst.l d0`
after the `move.l #0`, or branch on `movea.l d0,a0` first) — so it's the FIRST
easy win of that parcel, and it compounds with B3 (the ~36% snap pop) at the same
transition-end frame. This is exactly the gate-blind CC-clobber class the C2 lens
exists for — the dry-panel debut earned its keep.

---

## REFUTED — C2 HAZARD 2: hot-swap slot-patch "race" does NOT survive verification

C2 claimed `HBlank_Install`'s `move.l a0, HBlank_Vector_Slot+2` could be split by
an in-flight HInt landing "between the target's two word writes" → torn jump →
crash. **This is a false positive:**
- The 68000 samples interrupts at **instruction boundaries, not mid-instruction**.
  Both word bus-cycles of a single `move.l` complete before any IRQ is serviced —
  there is no "between the two word writes" window for an interrupt. (C3
  independently and correctly stated this and ruled the arming SAFE.)
- The inter-*instruction* window (after `move.w #$4EF9,(slot)`, before the
  `move.l`) is also safe: on a hot-swap the opcode word is **unchanged**
  ($4EF9→$4EF9), so the intermediate slot is always a valid `jmp` to the OLD
  (still-installed, still-valid) handler; on first install IE1 is not yet live
  (armed only via the later shadow flush).
- `HBlank_Uninstall` changes only the single opcode word — atomic by construction.

Two lenses (A1 forwarded a re-install sub-nit; C2 raised HAZARD 2) converged on
the same concern; adversarial verification against the actual 68000 interrupt
model + the hot-swap opcode-unchanged invariant refutes both. **No change.** The
load-bearing invariant is now explicit here for the record: *arming safety
depends on IE1-after-slot ordering and the opcode word staying $4EF9 across a
hot-swap* — a refactor that set IE1 before the slot, or changed the opcode on
re-arm, would break it.

---

## FOLDED IN-TRANCHE (byte-neutral comment/contract fixes; CRC 00f609a5 unchanged, parallax_port 3/3)

- **C3 F1 (MED-HIGH, contract honesty) — HBlankHandler "interrupt-transparency"
  overclaimed.** `clobbers()` covers CPU state only, NOT the VDP control-port
  two-word command latch / autoincrement. A future handler author adding a
  control-port write could corrupt the interrupted context's VDP command. FOLDED:
  the contract comment now carries the CPU-state-only caveat + the VDP-access
  invariant (hblank.emp).
- **A1 F1 — Parallax_Init `Clobbers: d0,d1,a1` comment contradicted the verified
  `clobbers(d0-d7/a0-a6)`** (the snap `jbsr Parallax_Update` tail-call clobbers
  the file). FOLDED to the real tail-inclusive contract.
- **C3 F3 (LOW) — Vscroll_Write "must hold stopZ80" comment stale for the sound
  build** (true invariant = Z80 off the 68k bus, via stopZ80 OR the
  SND_CTRL_DMA_ACTIVE flag bracket). FOLDED + noted the reg-$0F=2 dependency.
- **A1 F2/F3/F7 (clarity nits — self-contradicting Update header, missing
  Step4/Step5 contract headers, Decode_Factor_A "Preserves" vocabulary):**
  fold-on-next-touch (lower value; not folded this pass to keep the diff tight).

Folds are `.emp`-side; the `.asm` twins carry the older scaffolding comments,
retired at twin death (twin-scaffolding-kill-list practice).

---

## LEDGERED (ledger rows appended)

- **B1 P1 — VDP_DATA/VDP_CTRL const block duplicates plane_buffer.emp's.**
  Parallax is the 2nd `.emp` consumer → the consolidation rule triggers (hoist
  into `engine.vdp`, both files `use` it; byte-neutral). Cross-file (touches
  plane_buffer + engine.vdp) → **ledger as VDP-comptime-home consolidation debt**,
  pairs with the `set_vdp_reg` step-4 helper (now 4 sites: parallax 1 + hblank 3 —
  B1 outward-noted). One consolidation pass does both.
- **A1 F4/F5/F6 — language-asks:** Decode_Factor_A/B parametric-dedup (comptime-fn
  over the field-offset triple); `rept N {}` construct (2 hand-unrolls: VSRAM
  emit + flat-fill); `ensure_layout(struct, "twin")` (collapse the 24-line
  drift-wall). Step-4 / Spec-5 construct candidates.
- **C3 F2 — Vscroll_Write reg-$0F=2 enforcement gap** (latent, ported behavior,
  currently safe — drain leaves $02). Hardening candidate (`move.w #$8F02,(a5)`
  at entry, byte-changing) OR leave as documented dependency. Ledger.
- **C2 stride note — Step4a `.copy_band` hardcodes d2*10** (lsl#3 + add) NOT tied
  to `sizeof(band_entry)` by an ensure, unlike every other band stride. Correct
  today (band_entry=10, drift-wall pinned); maintenance hazard if band_entry
  grows. Ledger (drift-lock candidate).
- **C1 — OJZ_Default "5 bands" header comment stale** (band_count=4). Byte-neutral
  game-data comment, fold-on-touch.

---

## CLEAN CONFIRMATIONS (the panel's other half — what held up)

- **C1: no missed ≥1k perf.** At production band_count=4, the closest missed lever
  (Step4a vshift-unchanged cache, ~500-700 on horizontal-scroll frames) is below
  the bar and overlaps the logged frame-skip design item. H2 was the right & only
  ≥1k cut. Six candidates skip-logged with numbers stand.
- **C2: the H2 `lsr #3` ×8 unroll is AIRTIGHT** — spans are ×8 on every path
  (config-own + Step4a shadow, tops = cell×8; `ble` guards the zero/malformed
  span before the lsr/dbf; shadow tops provably monotonic → exact [0,224)
  coverage, no under/over-fill). Independent confirmation of the step-5 cut.
- **C3: trampoline core SAFE** — arming order (slot-direct then IE1-in-shadow),
  uninstall late-HInt (rte-first), shadow-before-dirty + no direct reg-$00/$0A
  writes (no flush reverts IE1). The shadow-coherence binding, independently proven.
- **A1/B1: hblank.emp CLEAN both lenses.**

---

## VERDICT

**One panel round.** It caught **B6** (real gate-blind bug → post-merge transition
parcel), **refuted** one plausible-but-wrong hazard (adversarial verification did
its job), **folded 3 byte-neutral contract/clarity fixes**, and **ledgered 5
future-work items**. No in-tranche byte-CHANGING re-open (B6's fix splits;
everything else folds/ledgers). The in-tranche loop is DRY. **t18 is ready for
merge prep** (PROVENANCE re-baseline to plain `00f609a5` / debug `80d14183`)
pending the gate's ratification of: B6→parcel, HAZARD-2 refutation, the fold set,
and the dry declaration.
