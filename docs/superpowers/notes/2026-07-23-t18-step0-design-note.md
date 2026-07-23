# t18 — parallax port: STEP-0 DESIGN NOTE (porting agent → overseer gate)

**Mode:** DAYTIME run (normal loop, live gates, dry-panel closes the cycle).
**Masters at dispatch (fetch-verified, origin==local both):** aeon `c39f308` /
sigil `93a48dc` (the daytime-brief commit atop 48ffe9f). Canonical at dispatch:
plain `ab787bd1`/421122 · debug `6a19669f`/429165.
**Branches:** `port-tranche18` both repos, seeded worktrees `aeon-t18` / `sigil-t18`
(aeon seed = `tools/seed-worktree.sh`, in flight).
**This note is the gate artifact. NO code has been cut. Bringing it to the gate
before step 1, per loop discipline.**

---

## 0. Scope of t18

The branch carries THREE distinct workstreams — one merge, but they are not one
change and the gate should rule on each:

1. **parallax.emp** — the fresh from-scratch port of `engine/level/parallax.asm`
   (Wave-3 hot .asm, **no existing .emp twin, no gate**) through the full loop
   0 → 1 → 2 → (3→4→5)* → 6. This is the milestone target (last big un-ported
   engine domain; measured 18–22 %/frame — real step-5 material).
2. **HBlank RAM-jmp trampoline** (row 1088, BINDING) — a **byte-changing** rework
   of the *already-ported* `hblank.emp` + twin + boot + vector. Inherited as
   binding; scheduled to execute here. See §4 — there is a load-bearing finding.
3. **Hscroll_Dirty deletion rider** (Phase-2.5 parked) — delete the dead
   `Hscroll_Dirty_Start/_End` stores + RAM symbols (parallax step-2). See §5.

All three are byte-changing. **t18 is therefore a PROVENANCE-RE-BASELINING
tranche** — the canonical CRC changes (unlike byte-neutral G5). Merge-ceremony
re-baseline + SHA-append is expected at the gate after the dry-panel closes.

---

## 1. File recon — parallax.asm (838 lines, 11 procs)

| Proc | Lines | Role | Heat |
|---|---|---|---|
| `Parallax_Init` | 15–38 | wipe state, seed sentinels, one snap-Update | init |
| `Parallax_CheckBoundary` | 61–95 | per-section config re-select on centre crossing | cold (edge) |
| `Parallax_StartTransition` | 117–161 | stage snap/lerp + VDP $0B mode-set-3 shadow | cold |
| `Vscroll_Write` | 183–203 | VBlank VSRAM emitter (whole-plane / 20-col) | VBlank |
| `Parallax_Update` | 219–329 | **per-frame builder**: config resolve + band lerp | HOT |
| `Parallax_Step4_Fill` | 331–448 | vscroll-rotate bands → screen space, dispatch fill | HOT |
| `Parallax_Step5_Vscroll` | 451–534 | whole-plane + per-column V-scroll compute | HOT |
| `Decode_Factor_A` | 543–571 | negated Plane-A scroll (shift-add, s1/s2/op) | HOT |
| `Decode_Factor_B` | 576–603 | Plane-B twin of Decode_Factor_A | HOT |
| `Parallax_Fill_PerLine` | 626–786 | **the bulk** (21k/28k cyc): 224-line 4-way deform fill | HOTTEST |
| `Parallax_Fill_PerCell` | 797–837 | 28-longword flat fill | HOT |

**Control-flow chain (unusual — note for step-1 fidelity):** `Parallax_Update`
falls into `Parallax_Step5_Vscroll` (`bra.w` :327) which falls into
`Parallax_Step4_Fill` (`bra.w` :534). Step 5 runs *before* Step 4 by design (Step
4a's band rotation needs this frame's Vscroll_BG — comment :325). The three procs
are one logical pipeline reached by fall-through; the transcribe must preserve the
`bra.w` fall-through order exactly.

**Data shapes (structs.asm, verified):**
- `parallax_config` = **28-byte header** (`parallax_config_len=28`) + N×`band_entry`
  inline. Header fields $00 band_count / $01 v_factor_bg / $02 v_factor_fg (RSVD) /
  $03 layer_mask / $04 v_center_y / $06 v_offset / $08 transition / $09 deform_speed_fg /
  $0A deform_speed_bg / $0B pad / $0C deform_table_fg / $10 deform_table_bg /
  $14 v_deform_table_bg / $18 v_deform_speed_bg / $19 v_deform_shift_bg / $1A pad2.
- `band_entry` = **10 bytes** (`band_entry_len=10`): top_cell, factor_a_{s1,s2,op},
  factor_b_{s1,s2,op}, deform_shift_a, deform_shift_b, phase_offset.
- `Sec.sec_parallax_config` = $14, `Act.act_parallax_config` — Sec/Act reg-relative
  reads → **`use engine.structs`** (row 1051 CLOSED; step-2 item 5).
- Constants consumed: `MAX_PARALLAX_BANDS=8`, `PARALLAX_TRANS_DEFAULT=16`,
  `PARALLAX_LERP_SHIFT=4`, `SECTION_SIZE_SHIFT=11`, `SCREEN_WIDTH/HEIGHT=320/224`
  → **`use engine.constants`** (never a file-local mirror; step-2 item 5).
- Macros consumed: `setVDPReg` (:158), `vdpComm` (:185) — the typed VDP interface
  already lives in the corpus (section.emp t15); **adopt** (macro-port rule: their
  .emp counterparts exist, so this is adoption not redesign).

**RAM (ram.asm):** `Hscroll_Buffer` 896B (:135), `Hscroll_Dirty_Start/_End` 1+1B
(:136-137 — see §5), `Vscroll_Factor` 4B (:139), `Parallax_State` block ~126B
(:144-169, longword-multiple — Init zeroes it in longwords).

**External refs:** `Camera_X/Y`, `Current_Act_Ptr`, `Vscroll_Factor`, VDP_CTRL/DATA,
`Section_GetSecPtrXY` (cross-module → §7).

---

## 2. Ledger + kill-list TRIP-CHECK (the step-0 sweep)

Swept the gap-ledger + kill-list for `parallax` / proc symbols / consumed consts +
symbols / at-next-touch rows naming files this tranche touches. Rows this port
**trips or touches**:

| Row | Subject | Disposition in t18 |
|---|---|---|
| **1088** | HBlank RAM-jmp trampoline — RATIFIED, "executes at t18 step-0/1 as first-consumer design" | **BINDING** — §4 (with a finding + a gate question) |
| **1058** | Parallax_Update = next vertical-streaming lag lever (25178 cyc/f 18.2 % @16px; Fill_PerLine 21069 the bulk) — **the step-5 perf charter** | step-5 owns it — §6 |
| **1085** | Master opt-review: parallax **B1/B2/B3** (transition-logic correctness), **H2/H3/M1-M5** (Tier-B), false "~410 cyc" comment (:594), d6-across-CheckBoundary fragility (:558) | step-3(b)/step-5 loop material — §6 |
| **1091** | D7 dead-code batch names `Hscroll_Dirty_Start/_End` "the dead dirty mechanism" | **take ONLY the Hscroll_Dirty pair** (file-implicated); the rest of D7 stays its own batch — §5 |
| **1050** | standalone mixed AS build won't assemble — parallax.asm:79 refs Section_GetSecPtrXY (gated-out) | INHERENT, not a defect; the sigil-link mixed mechanism resolves it — §7 |
| **1051** | Sec/Act shared-struct module (CLOSED) | parallax.emp `use engine.structs` for Sec/Act fields — step-2 item 5 |
| **1046** | `(Sym).w` paren-width backlog (7 pre-ratification files) | parallax is NEW → starts bare; conform from line 1, no backlog added |
| **1047** | codename-narration backlog | parallax.asm carries ephemeral `T6 stub`/`T8+`/`T12`/`(T12)` tranche codenames; hblank.asm carries `item 8`. **Fix on touch** (step-2) — durable anchors (`§4.6`, `§0.10`) stay |
| **1268** | mixed_dac_rom tranche-map bases hand-maintained (hblank listed) | hblank re-pin touches this — §8 ripple |
| **964** | (CLOSED) HBlank_Handler_Ptr is a spliced pin-lo16 site in mixed_dac_rom slices | trampoline changes/removes HBlank_Handler_Ptr → that splice site updates — §8 |
| **1095** | (OPEN) subcontract-target detection waits for a typed pointer cell — names `HBlank_Handler_Ptr` | the trampoline reshapes this cell; note the interaction, not a t18 deliverable |

No **kill-list rows fire** this tranche beyond the twin-scaffolding rows the port
itself will *add* (parallax.asm becomes a gate-off twin — kill-list row 5 backfill).

**P1a / deform H1 — do NOT re-derive (confirmed shipped):** `deformShiftDefault=15`
is live in `ojz_default.asm:45`, `caves.asm:22`, `locked_clouds.asm:27`. The
flat-path shortcut is already taken by the production configs. H1 is a DATA change
in the game configs, not engine code; parallax.asm's per-line path is unchanged by
it. Off t18's plate.

---

## 3. Inherited obligation #1 — HBlank RAM-jmp trampoline (row 1088)

### Current state (verified)

- Vector $70 (IRQ4/HBlank) → `HBlank_Dispatch` in ROM (vectors.asm:36).
- `HBlank_Dispatch` (hblank.emp:23-29): `movem.l d0-d1/a0,-(sp)` / `movea.l
  HBlank_Handler_Ptr,a0` / `jsr (a0) as HBlankHandler` / `movem.l (sp)+,d0-d1/a0`
  / `rte`. `HBlank_Null` = bare `rts`.
- `HBlank_Handler_Ptr` (ram.asm:72) set **once**, to `HBlank_Null`, at boot.asm:185.
- **HInt is NOT enabled at the VDP.** boot.asm:186 enables only VInt
  (`vdp_mode2 #$34`, reg $01 bit 5). No reg $00 IE1 bit, no reg $0A counter write
  anywhere in the corpus.

### THE LOAD-BEARING FINDING — there is no consumer

**Nothing installs an HInt handler, HInt is disabled, and parallax does per-line
HScroll via the VDP HScroll table (`Hscroll_Buffer` DMA'd in VBlank) + per-column
V-scroll via VSRAM — neither uses HInt.** The entire vector→Dispatch→Null path is
**dormant**. The opt-review's own framing (item 10 / row 1088) says "BEFORE any
real per-line HInt is active (**OJZ parallax end-state**)" and "**Zero handlers
exist yet**" — i.e. the HScroll-table → per-line-HInt migration is a *future* state
that t18 does **not** perform. So t18's parallax is **not** the trampoline's first
consumer; the trampoline is preemptive infrastructure with no live raster target
in this tranche.

I inherit row 1088 as binding (per the brief) and have designed it. But the
"first-consumer executes at t18" premise is **nominal** — this is honest and it
changes what "raster-timing live-verify" can mean (see the gate question).

### Ratified design (S3K RAM-jmp), concretized

- **New RAM slot** `HBlank_Vector_Slot` — 6 executable bytes. Idle = `rte`
  (`$4E73`); active = patched `jmp handler` (`$4EF9` + 4-byte target). Replaces the
  `HBlank_Handler_Ptr` `ds.l 1` cell (RAM-layout shift → re-pin).
- **Vector $70** → `HBlank_Vector_Slot` (the fixed RAM address) instead of
  `HBlank_Dispatch`.
- **Install helper** `HBlank_Install(a0 = handler)`: write `jmp a0` into the slot,
  enable HInt (VDP reg $00 IE1 + reg $0A scanline counter). **Uninstall**: write
  `rte` back, disable HInt. HInt is thus *disabled whenever no handler is installed*
  (the ratified invariant), and the ~116–140 cyc/line movem/jsr/rte shell is gone —
  the handler is entered directly and owns its own save/restore + `rte`.
- **hblank.emp/twin:** delete `HBlank_Dispatch` + `HBlank_Null`; add the
  install/uninstall helpers + the idle-slot initializer. The `HBlankHandler`
  contract type changes: the handler is now **rte-terminated** and owns its full
  save/restore (its `clobbers()` set is whatever it doesn't preserve) — the
  `as HBlankHandler` bless site moves onto the install helper's target argument.
- **boot.asm:** replace `move.l #HBlank_Null,(HBlank_Handler_Ptr).w` with writing
  the idle `rte` into `HBlank_Vector_Slot`; HInt stays disabled at the VDP (already
  is). The `jmp`-in-RAM must be flushed to instruction coherency before first use —
  68000 has no I-cache so a plain write suffices, but note it for the twin.

### GATE QUESTION 1 (the decision I need before cutting)

Given no consumer exists, how is the trampoline's **live raster-timing verify**
satisfied?
- **(a) RECOMMENDED — build the mechanism in t18 + synthetic-handler live-verify.**
  Install a throwaway test handler (e.g. writes a CRAM colour at a mid-screen
  scanline), enable HInt, prove on oracle: the install patches the `jmp`, HInt
  fires into the handler, `rte` returns clean, and the disabled-when-empty path
  costs zero. Parallax stays on the HScroll table (the HScroll→HInt migration is
  the future "OJZ end-state," explicitly out of t18 scope). This honors the binding
  ruling and gives the mechanism a real live proof without a real consumer.
- **(b) Decouple** the trampoline to its own byte-changing parcel (it shares no
  code with parallax.emp; bundling is calendar-coupling, not logical coupling).
- **(c) Descope from t18** and let row 1088 wait for the actual first consumer.

I lean (a): the ruling is "decide now because zero handlers exist," and (a)
executes exactly that intent. But the no-consumer reality is a genuine input to the
choice, so I am surfacing it rather than silently building dormant infra.

---

## 4. Inherited obligation #2 — Hscroll_Dirty deletion rider

**Confirmed dead** (corpus grep): `Hscroll_Dirty_Start/_End` are **written only** at
parallax.asm:440-441 (per-line path) and :445-446 (per-cell path) — 4 stores — and
**read nowhere** in the tree. They are the "dead dirty mechanism" of ledger row
1091 (D7). The consumer that would have read them (a partial HScroll DMA keyed on
the dirty span) was never built; the VBlank emitter DMAs the whole buffer.

**Plan (step-2 modernization):** drop the 4 dead stores from parallax.emp + twin,
delete the two RAM symbols from ram.asm. Byte-changing (RAM-layout shift + removed
stores) → both-shape lockstep + re-pin. Parity statement per the standing
RAM-deletion rider. **Scope discipline:** t18 takes ONLY the Hscroll_Dirty pair
(file-implicated by its sole writers living in parallax); the rest of D7's list
(Spawn_Count, CROSS_RESET_MAGIC, ess_*_left_idx, Tile_Cache_GetTile, dead consts)
stays its own gated batch — I will not widen the deletion.

---

## 5. Inherited obligation #3 — parallax H2/H3 + transition-logic (step-5 material)

These are **loop material** (step-3(b) reads-wrong / step-5 optimize), interrogated
during the 3→4→5 cycle with design + oracle-verified cuts (daytime — live work
allowed foreground). Enumerated now so the sweep is on record; NOT cut at step-0:

- **Perf charter (row 1058):** `Parallax_Fill_PerLine` is the bulk (21069 cyc/f of
  the 25178). The step-5 invariant-ladder / counter-cache / guard-coverage
  interrogation runs per hot proc. **H2/H3/M1-M5** (row 1085 item 22, Tier-B) are
  the named optimization candidates — design + interrogation, live-verified cuts.
- **B1/B2/B3 (transition-logic correctness, row 1085):** B1 re-crossing back into
  the current config's section mid-transition doesn't cancel the staged transition;
  B2 builder/DMA-length/VSRAM-mode consumers disagree on "active" config mid-
  transition (≤16-frame tear); B3 the 16-frame `>>PARALLAX_LERP_SHIFT` lerp ends
  ~36 % short → end-of-transition pop. These are **behavior-affecting** and were
  explicitly excluded from drive-by fixes ("needs a transition-logic design pass").
  I will surface them at step-3(b), design a fix, and gate it — they may be their
  own follow-on rather than in-tranche if the design is deep. Flagging for the gate.
- **Comment-claim audit (step-3(b)):** the `Parallax_Update` header "~410 NTSC
  cycles" (:213) is **false** (measured 25178) — fix on touch. `constants.asm:319`
  convergence comment is wrong (B3-adjacent).
- **Contract audit (step-3(b)):** the d6-across-`Parallax_CheckBoundary` liveness
  fragility (opt-review :558) — verify CheckBoundary's clobber contract (:59 says
  d0-d3/a0/a2) against every caller's live registers.

---

## 6. Inherited obligation #4 — type-layer walk (step-2 item 6, first outing)

Per the item-6 value-flow rule (MOVED+COMPARED pays; shift/add chains WAIT for
A4-i and get LEDGERED). Parallax is deform/shift/add-heavy, so expect mostly
LEDGER-not-type — but there is **one concrete adoption win**:

- **GridX/GridY at `Parallax_CheckBoundary` (REAL adoption, G5 types already
  exist).** :64-69 compute `cur_sec_x` in d2 and `cur_sec_y` in d3 (`asr` by
  SECTION_SIZE_SHIFT), compare them against `Prev_Sec_X/Y` (:72-74), then call
  `Section_GetSecPtrXY` (:79) — which G5 already typed `(d2: GridX, d3: GridY)`.
  So d2/d3 here are a **MOVED-and-COMPARED axis pair feeding a typed callee** —
  exactly the sec_x/sec_y swap class G5 closed (MigrateMasks family). Bless d2 as
  `GridX` / d3 as `GridY` at the `asr` producing sites so the call type-checks and
  a swap is caught. **This is the parallax port's headline type-layer result.**
- **VramTile/VramAddr (item-13 wave-1 candidate, comptime-first):** `Vscroll_Write`
  builds a VSRAM address via `vdpComm(0, VSRAM, WRITE)`. Wave-1 ruled VramTile/
  VramAddr comptime-first (VRAM_* consts + vram_bytes/vram_art), register slots
  "later wave" — so LEDGER the VSRAM-address register flow as a later-wave candidate,
  don't force a slot type here.
- **LEDGER-not-type (shift/add chains):** scanline index (d4 in Fill_PerLine),
  deform sample bytes (a5/a6 reads), band index (d5), scroll offsets (Decode_Factor
  d2), vshift (d0 in Step4a) — all live in shift/add/asr arithmetic. Per the rule
  these WAIT for A4-i arithmetic-preservation; log each as a candidate, don't
  ceremony them. The packet will carry the per-value verdict table.

---

## 7. Step-1 transcribe plan (mechanics — executed post-gate)

- **Fresh port:** create `engine/level/parallax.emp` (module `engine.parallax`,
  its own pinned section) + the `SIGIL_EMP_PARALLAX` gate wiring (engine.inc
  resume-org else-arm, mixed_dac_rom tranche-map entry) + region pin. parallax.asm
  becomes the gate-off twin (kill-list row 5 backfill, same commit).
- **Byte gates BOTH shapes** (plain + debug), mixed-build acceptance, gate-off
  neutrality CRCs, negative probes. Gate-artifact discipline: every gate row names
  its test/commit.
- **Cross-module link (Section_GetSecPtrXY):** parallax.emp will `jbsr
  Section_GetSecPtrXY` into section.emp (already ported, G5-typed). This is a
  normal cross-.emp call, **NOT a symbol-ownership flip** (section already OWNS the
  symbol in .emp; ownership doesn't change — only a new caller appears). So the
  two-module ownership-flip link test (the plane_buffer→section template) is **not**
  triggered; the standard cross-module link resolution + a link-edge gate row
  suffice. Probe binding-class: this is a **link-time** extern resolve (parallax.asm:79
  is the AS-side undefined-symbol row 1050 confirms), not comptime — the step-1
  gate must exercise the real mixed-link path, not a comptime stand-in.
- **Fall-through fidelity:** the Update→Step5→Step4 `bra.w` fall-through chain
  (§1) is transcribed verbatim at step 1; any conversion to `jbra`/reordering is a
  step-2 decision, lockstep + re-pin.

---

## 8. Ripple + PROVENANCE (the 5-site discipline)

Every byte-changing sub-workstream (trampoline, Hscroll_Dirty deletion, later
step-5 cuts) pays the ripple **inside its own commit**: pins.rs (repin), engine.inc
resume orgs (hand), mixed_dac_rom.rs tranche map + reference slices (hand — incl.
the HBlank_Handler_Ptr pin-lo16 splice, row 964), repin_pins.rs (hand), repin.toml
(only if a region is added — the trampoline adds `HBlank_Vector_Slot` but that's
RAM, and parallax.emp adds a new ROM region → repin.toml entry). **Full paired
strict failures-first at every commit boundary**, AEON_DIR pinned to THIS branch's
tree (paired-state gate — never aeon master). A red gate stops the run with a
written checkpoint.

**PROVENANCE:** t18 IS byte-changing to the canonical ROM (trampoline + deletion +
step-5). The canonical CRC re-baselines at the merge ceremony (dual-invocation
rebuild from merged aeon master → new plain/debug CRCs → SHA-append). This is a
re-baselining tranche; the byte-neutral discipline of G5/§D does not apply.

---

## 9. GATE QUESTIONS (decisions I need ruled before cutting)

1. **Trampoline live-verify strategy** (§3): (a) build in t18 + synthetic-handler
   live-verify [recommended], (b) decouple to own parcel, (c) descope. The
   no-consumer reality is the input.
2. **B1/B2/B3 transition-logic** (§5): in-tranche step-5 design+fix, or split to a
   follow-on parcel given "needs a transition-logic design pass, not a drive-by"?
   My lean: surface + design at step-3(b), size it, and if the fix is deep, gate a
   split rather than bloat t18.
3. **Trampoline contract-type change** (§3): confirm the handler becomes
   rte-terminated + full-save (the `as HBlankHandler` bless moves to the install
   target). Any objection to deleting `HBlank_Null` outright (idle state becomes the
   in-RAM `rte`, no ROM proc)?

## 10. What I will NOT do without a ruling

- Cut any code (this note precedes step 1).
- Build the trampoline before the live-verify strategy is ruled (Q1).
- Widen the Hscroll_Dirty deletion into the rest of D7.
- Perform the HScroll-table → per-line-HInt migration (out of scope; future).
- Force domain types onto shift/add register flows (ledger them per item-6).

---

**Deliverables produced this step:** branch tips both repos (aeon-t18 `c39f308` /
sigil-t18 `93a48dc`), seeded worktrees (aeon seed built canonical `ab787bd1`/421122
EXACT), this design note.

---

## 11. GATE RULINGS (overseer, 2026-07-23 — PASS; step 1 authorized)

**Q1 → (a) RATIFIED.** Build the trampoline in t18 + synthetic-handler live-verify.
Constraints (all BINDING):
1. **No shipped test code.** Preferred: oracle-inject the synthetic handler (poke
   handler bytes into free RAM, drive/emulate `HBlank_Install`, restore after).
   Fallback: DEBUG-gated handler, provably zero bytes in plain. State which in packet.
2. **Live-verify success criteria (all five, evidence in packet):** (i) HInt fires
   at the programmed scanline (run_to_scanline / HV counter); (ii) entry via the
   RAM-slot `jmp` (breakpoint at handler); (iii) clean `rte` — execution resumes,
   registers intact; (iv) uninstall → slot reads `$4E73`, IE1 off, no further HInts;
   (v) human-visible raster artifact (mid-screen CRAM split) — screenshot.
3. **BINDING — VDP shadow coherence.** Install/uninstall touch reg $00 (IE1) + $0A
   (counter). These MUST keep `VDP_Shadow_Table` coherent — a direct-only control-port
   write leaves stale shadow that a later `Flush_VDP_Shadow` reverts (silent IE1
   flip). Update shadow + mark dirty, OR update shadow + write direct (my call which),
   but shadow must never disagree with hardware on $00/$0A.
4. **Boot ordering invariant:** the slot holds `$4E73` BEFORE interrupts unmask
   (RAM-clear leaves `$0000` — not a legal idle). Stated as invariant.

**Q2 → confirmed + split rule PRE-RULED.** Surface + size B1/B2/B3 at step-3(b). A
B-fix lands in-tranche (own gated commit) ONLY if (i) locally provable, (ii)
live-verifiable in this tranche's oracle session, (iii) doesn't restructure the
transition state machine. Failing any one → follow-on parcel (ledger row + design
sketch, no code). B3 smells bounded; B1/B2 smell like the design pass — the 3(b)
sizing decides, not the smell.

**Q3 → confirmed with a CORRECTION.** rte-terminated: yes. Delete
`HBlank_Dispatch`+`HBlank_Null`, idle = in-RAM `$4E73`, vector $70 → slot: yes.
**Correction: the contract is INTERRUPT-TRANSPARENCY, not "full-save"** — the handler
saves/restores exactly the registers it touches (observable clobbers = ∅), rte. A
mandatory blanket `movem` would reinstate the wrapper cost row 1088 kills. The
note's wording ("owns its save/restore; clobbers() = whatever it doesn't preserve")
is canonical. `HBlankHandler` bless → install helper's target argument: approved.

**Also approved as scoped:** Hscroll_Dirty pair-only deletion (do NOT widen into D7);
GridX/GridY bless at CheckBoundary (**if the bless carries a typed value across a
multiply, the FlatIDXY preserves-verifier ledger-row reopen condition triggers —
surface it, don't absorb it**); "~410 cyc" comment fixed present-tense on touch;
t18 = PROVENANCE re-baselining (full 5-site ripple per commit; **pin strict-harness
AEON_DIR to the BRANCH tree** — the §D phantom-red lesson).

**Dry-panel (far end):** floor A1+B1+C1+C2; parallax triggers **C3** (hardware-timing
lens) — the trampoline + IE1 work is exactly C3 territory.
