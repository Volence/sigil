# 68k engine-port campaign — kickoff handoff (written overnight 2026-07-09)

For Volence's morning. Both overnight arcs are done and checkpoint-ready (details in their
own notes); with them merged, **Plan 7 is COMPLETE (#1–#10 all landed)** and Spec 2 is ready
to FREEZE. This note proposes the freeze checklist, the first migration targets, and the
morning checkpoint list.

## Morning checkpoint list — ITEMS 1–3 EXECUTED 2026-07-09 (Volence approved; Fable ran the sweep)

1. ~~Merge sigil `seam-reeval` → master~~ **DONE — merge `bf247da`** (`--no-ff`). Item-B
   ruling (`extern("NAME")` + AS equ export) + Item-A dispositions ratified with the merge
   (`specs/2026-07-09-seam-reeval-decisions.md`; packet:
   `notes/2026-07-09-seam-reeval-complete.md`).
2. ~~Merge sigil `compression-builtins` → master~~ **DONE — merge `2576d91`** (`--no-ff`;
   packet: `notes/2026-07-09-compression-builtins-complete.md`).
   **Post-merge master validation: 1654/0 workspace, strict gates 15/0, clippy clean;
   pushed. Worktrees removed, branches deleted. Plan 7 #1–#10 COMPLETE ON MASTER.**
3. ~~Commit the empyrean spec working tree~~ **DONE — empyrean `070a118`, pushed** (the
   three stacked passes D2.25/D2.26/D2.27 + ledger dispositions + the Plan 4/5/6 plan
   docs; the docs-cadence backlog is clear).
4. **Ratify this kickoff** (freeze checklist + first targets below) → campaign starts.
   Volence indicated go (2026-07-09 morning); the freeze ceremony below is the campaign
   session's first act.

## Spec-FREEZE checklist (proposed)

Plan 7's contract was: research → implement → FROZEN spec. With #10 merged:

- [x] ~~Volence commits the empyrean working tree~~ DONE (`070a118`) — the spec text now
      matches shipped reality through D2.27.
- [x] **Pre-freeze completeness audit (Fable, 2026-07-09, Volence-requested)** — full-spec +
      research-doc + campaign-target sweep. Findings folded into the empyrean working tree:
      **D2.28** (compression builtins recorded — `2576d91` had shipped them with no spec pass;
      §6.7/§10 updated), **D2.29** (`align N` ratified, NEW §4.8 — NOT implemented; 3 of the 4
      data quick-wins below need it, so it rides the first data tranche), **S2-D15** (the
      control-flow-sugar "no", previously prose-only), **S2-D16** (Tier-3 no-demand bundle:
      charmap impl, decompression builtins, fill knob, T3-b/c/h/i/j/k, vector-table construct,
      cheap-local-labels DECIDED AGAINST), **S2-D7(c)** freeze disposition (stays deferred; on
      the tranche-1 re-eval list), **S2-D11** recommendations recorded. **Ratify-at-freeze
      remainder (Volence):** S2-D11 (a)(d)(e) in/out calls. (The `yield <label>` spelling
      question was SETTLED by D2.30 — Volence's readability review drove the whole batch;
      formal nod rides the freeze stamp.)
- [x] **Audit follow-up (Volence's catch, 2026-07-09):** the `script`/`dispatch` **`code_word`
      encoding** recorded in S2-D12(b) — real Aeon dispatches table-lessly (`move.w (a0)→jsr`
      into the 64KB obj bank; `objroutine(x) = x − ObjCodeBase`; SST offset 0 IS the routine
      word; objects have no runtime ids), so neither shipped table encoding runs on it. Additive,
      non-breaking (R1's required-encoding knob exists for exactly this); REQUIRED before the
      first scripted-object port. Exhibit comments updated to name the mock-vs-Aeon distinction.
- [x] ~~Declare §10's concept inventory CLOSED for v1~~ **DONE 2026-07-09** — the spec's
      Status line now carries the v1 FREEZE stamp with the audit's additions (D2.28–D2.30,
      code_word, S2-D15–D17) folded in; A-Spec2.3 governs amendments from here.
- [x] ~~The deferred ledger is the freeze's "explicitly NOT in v1" list~~ **DONE** — the
      audit completed it (S2-D15 control-flow "no", S2-D16 Tier-3 bundle, S2-D17 patch/bind,
      S2-D7(c) disposition, S2-D11 (a)(d)(e) ratified IN); every row has an owner/unblock.
- [x] ~~Tag/record the freeze in empyrean~~ **DONE** — the Status-line version stamp is in
      the spec working tree; **Volence's empyrean commit IS the remaining ceremony** (his
      docs cadence; everything above it is written).
- [ ] Known non-blockers carried into the campaign (recorded, not gating): F1 flake watch
      item; the mt/sfx extern()-migration ride-along; `.emp` adoption of `s4lz()` in
      aeon's build (below).

## First migration targets (surveyed tonight, ranked by blast radius)

The campaign's port loop per file: add a `SIGIL_EMP_<NAME>` gate (copy the exact
`ifndef … include … else org $ADDR endif` spelling from `games/sonic4/main.asm:111/:154/:232`),
write the `.emp`, pin the region in `sigil.map.toml`, byte-gate both shapes, negative
probes, merge. All six code candidates below are `__DEBUG__`-define-free — the cheap
(sfx-style) gate shape.

**Code targets (start here):**
1. **`engine/system/hblank.asm`** (18 lines) — 2 labels, 1 imported RAM equ
   (`HBlank_Handler_Ptr`, now readable via `extern()` if needed), 2 referencing files.
   The ideal first code port.
2. **`engine/system/controllers.asm`** (62 lines) — straight-line I/O, standard local
   labels, single caller (`vblank.asm`).
3. **`engine/system/math.asm`** (27 lines) — `GetSineCosine` + a BINCLUDE sine table
   (= `embed()`); more callers (player_ground ×4) but call sites only need the symbol.
4. Then: `collision_lookup.asm` (44 ln, 6 imports), `vdp_init.asm` (47 ln).

**Data quick wins (interleave anytime):** `vram_bases.asm` (8 ln, pure equ arithmetic —
now expressible end-to-end: `.emp` equ export + AS reads), `ojz_act_pool.asm` (14 ln,
BINCLUDE×3 + dc.l pointer table — the proven dac_samples shape), `particle_anims.asm`
(15 ln, the `offsets` construct's shape), `plantbadmaps_anims.asm` (6 ln).
*Audit note (2026-07-09): three of these need item-position `align` (D2.29, ratified NOT
implemented — ojz_act_pool has `align 2` ×3, both anims files have one `even` each); build
`align` with whichever data port goes first. `vram_bases` and all four CODE targets are
align-free — port #1 (hblank) is unblocked today.*

**Deliberately deferred (hazards, surveyed):** `vectors.asm` (tiny but ~20-symbol fan-in +
org 0 header adjacency), `z80_init.asm` (Z80 payload), `game_loop.asm` (SOUND_DRIVER_ENABLED
ifdef + game-supplied `gameDebugTick` macro — port after the gate pattern is proven),
macro-heavy data (parallax, test_mappings' `sprSize()`, objdefs) — these want the macro-arg
story exercised deliberately, not stumbled into on port #1.

**Byte-gate infrastructure already in place:** harness gates diff the full main.asm tree vs
`aeon/s4.bin` (pins in `crates/sigil-harness/golden/PROVENANCE.md`: plain 451198 B
`8ce6dd7e…`, debug 458982 B `13c7b063…`); mixed-build harness + convsym allowlists proven
across three sound tranches; `extern()` closes the cross-seam constant-read gap the sound
arc kept hitting.

**Recorded follow-up riding the campaign:** `.emp` adoption of `s4lz()` inside aeon's
build (replacing the tools/s4lz.py call sites in ojz_block_gen's flow) — its own byte-gate;
the K-sweep/dict-selection logic stays caller-side. Also the mt/sfx co-residency ensures →
`extern("SND_ENGINE_TABLE_BANK")` ride-along.

## TRANCHE 0 — language completion sprint (Volence's call, 2026-07-09: build the ratified
## backlog UP FRONT, before conversion starts)

The ride-along list below was re-scheduled at the freeze: everything whose design is fully
determined gets built NOW as one sprint, so the campaign converts against a finished language.
**Scope (9 items):** `align N` + `[layout.odd-item]` (D2.29), inline `offsets` bodies (§4.7),
the D2.30 trio (`yield shows`, `yield .label`, `wait_frames`), `comptime test` (S2-D11a),
`///` parse-and-attach (S2-D11d), `todo!`/`unreachable!` (S2-D11e), struct rest-fill +
defaults (S2-D13h). **Deliberately EXCLUDED: the `code_word` encoding** — consumer-coupled
(base:, ObjDef.code emission, the real spawn seam); it rides the first scripted-object port
where reality can push back, per the mock-prelude lesson. **Acceptance gate:**
`examples/game/badniks/pitcher_plant_script_next.emp` compiles progressively as items land —
when everything but its `code_word` line builds, tranche 0 is done by demonstration; pinned
exhibits + the full byte harness stay green throughout; `align` gets AS-parity vectors.
Usual discipline: worktree branch, TDD, per-item two-stage reviews, Volence checkpoint before
merge. Port #1 (hblank) follows immediately after — it needs none of these, so it can also
serve as the sprint's warm-up if sequencing allows.

## Campaign ride-alongs (audit-ranked, folded in 2026-07-09 — SUPERSEDED by Tranche 0 above
## for items 1–4 and 6–7; kept for the rationale record)

Beyond the recorded follow-ups above, the pre-freeze audit ranks these by joy-per-effort:

1. **`align N`** (D2.29) — build with the first data port that needs it (ojz_act_pool /
   either anims file); hblank and all four code targets are align-free. Ships WITH its
   companion `[layout.odd-item]` check (audit amendment: alignment bytes stay explicit, but a
   68k proc [error] or word-bearing data item [warning] at an odd address is auto-diagnosed
   with an "insert `align 2`" fix-it; Z80 + `@as_compat` exempt — so new-style authors never
   have to remember `even`, only accept the fix-it).
2. **Inline `offsets` bodies** (§4.7, design settled 2026-07-06) — ride the first anims port;
   REQUIRED test: in-block ordinal self-reference (`Shoot: [.., $FD, Ani.Idle]`).
3. **Struct-literal rest-fill + default field values** (S2-D13(h)) — recommend IN early: every
   `ObjDef` literal today carries `vel: Vel{x:0,y:0}` / `frame: 0` ceremony because struct
   literals require every field; deleting it pays at every object the campaign touches.
4. **`comptime test` blocks** (S2-D11(a), pending Volence's IN/OUT) — the campaign's
   comptime-fn feedback loop; today the only loop is a full ROM build + byte-diff.
5. **`code_word` encoding** (S2-D12(b)) — with the first scripted-object port; blocked-on for
   real-Aeon scripts (see checklist above).
6. **Script readability batch (D2.30 — RATIFIED at the audit, Volence-driven)** — rides the
   first scripted-object port with #5: `yield shows <label>` (per-site epilogue, replaces the
   bare-label form), `yield .label` (named resume — the in-script
   `move.w #objroutine(State),(a0)`, zero-cost; + note-tier lint collapsing the old
   `yield`+`jbra` pair), `wait_frames #N, <slot>` (one-line pure park; compiler expansion,
   no dispatcher protocol). Addresses the two exhibit-review findings: parking is a two-line
   self-managed idiom, and the resume point is invisible on the page.
   Still 9c, still gated (protocol + real consumer): value-carrying dispatcher yields,
   `for`/`break`, script-calls-script.
7. **`sigil expand` CLI slice** (S2-D11(b)) — on-demand only; pull forward if porters keep
   asking "what bytes did this line emit".

Staying gated, no action: typed anim-command layer (9d re-gate), sprite-mappings /
version-parameterized records (post-campaign), `jbcc`, charmap.

## Suggested campaign cadence

Port #1 (hblank) in one sitting including the gate-pattern writeup; then batch 2–3 small
files per tranche with the same worktree/checkpoint discipline as the sound arc. Re-evaluate
after the first tranche whether code ports surface new spec gaps (the ledger's
ride-the-tranche items are queued for exactly that).

**Code-sense review (Volence's standing instruction, 2026-07-09):** byte-exactness is the
GATE, not the BAR. Every port gets a second look — "is this the code we'd WANT, not just the
code that matches?" — the pitcher-plant-script treatment (that brain was byte-correct and
still read wrong; the review produced D2.30). Byte-neutral improvements (names, comments,
erasing type annotations, source layout) land WITH the port; anything touching bytes or
behavior (a better brain, a cheaper loop) goes on the tranche notes' "reads wrong" list and
lands as its own post-port commit — never silently inside a byte-gated diff.

## Construct walks (the ritual that produced today's wins — schedule them)

The pre-freeze audit found the *unrecorded*; the `code_word` encoding and the D2.30 batch came
from a different method: **Volence walking one construct against the real machine and asking
where it creaks.** ("I may be biased because I've worked a lot with assembly" was the most
productive sentence of the audit — the bias is calibration.) Ranked walk list, each ~30 min
with Volence driving, at its natural trigger:

1. **The production prelude vs the real engine** (trigger: authoring it, early campaign) —
   spawn/children/anim/routine helpers + the REAL SST layout vs `engine/objects/*` +
   `engine/macros.asm`. Highest drift risk: the mock-prelude class that hid the table-less
   dispatcher (→ code_word) lives exactly here.
2. **`vars` RAM regions vs `ram.asm` reality** (trigger: first port that declares RAM) —
   the `.w`-addressability rule, region bases, align-under-vma against the real map.
3. **The Sonic newtype set vs player physics** (trigger: production-prelude typing pass) —
   Angle/SubPixel/Speed/VramTile (memory: emp-sonic-newtype-candidates) read against real
   player_ground-class hot code; the D2.10 scale layer's first contact with the code it was
   designed for.
4. **`dispatch` — know that Aeon never uses it** (no trigger yet): the engine is table-less,
   so dispatch's first REAL walk arrives with classic-Sonic porting; not a defect, just a
   recorded fact so nobody wonders why no aeon port emits one.
5. **`patch`/`bind`** — S2-D17: mechanism shipped, no surface, no consumer; the campaign
   either finds its first real back-patch case or it's demoted at Spec-2 close.
