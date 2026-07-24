# t19 — TRANCHE CLOSE PACKET (camera / bg / bg_anim conversion)

**The first tranche under the corrected LEAN amendment (in-tranche step 5 FULL;
no standalone parcels on other files).** Scope: `engine/level/camera.asm` →
`camera.emp`, `bg.asm` → `bg.emp`, `bg_anim.asm` → `bg_anim.emp`, full loop
`0 → 1 → 2 → (3→4→5)* → 6`.

Branch tips at close: **aeon-t19 `d5bd9ed` / sigil-t19 `413b37b`** (+ this packet).
Branch ROMs at close: **plain `eab19b3f`/421159 · debug `f1c1aa12`/429204**
(PROVENANCE re-baseline at merge; see the byte-delta table below).
Full paired strict at every byte-changing commit; final: **2509/0**
(baseline 2499 + camera_port 5 + bg_port 3 + bg_anim_port 2).

## Scoreboard

| Workstream | Outcome |
|---|---|
| **camera.emp** (Camera_Init/Camera_Update) | byte-identical transcribe → modernized (−6 twin relaxation) → typed `*Act` param + contract tightenings → clamp-clone template → live behavior-verified (P2/P3) |
| **bg.emp** (BG_Init) | byte-identical first compile → 2nd/3rd-consumer consolidations (z80_bus lift + VDP-const hoist) → `preserves(a3/sr)` closure-verified |
| **bg_anim.emp** (BgAnim_Init/Update) | byte-identical first compile → struct-derived record walk → DEBUG band-count assert (+0x62 debug only) → VramAddr vocabulary |
| **engine/z80_bus.emp** | NEW shared module (2nd-consumer lift executed; kill row 36) |
| **engine.vdp** | VDP_DATA/VDP_CTRL hoisted (3rd consumer; t18 B1 debt executed) |
| **game seam** | `-D GAME_CAMERA_JUMP_LOCK` + the comptime-select mirror idiom, =0 property PROVEN by link probe |

## Step-1 gate lists (artifacts — all EXECUTED)

**camera** (region plain $59CE / debug $6658; $16A at step 1 → $164 at step 2, shape-invariant):
- byte gates both shapes: `camera_port::camera_region_matches_reference` + `::camera_debug_region_matches_reference` (green at step 1 AND re-green after every byte-changing step)
- negative probes: `doctored_cam_max_y_step_fires_its_guard` (16→17), `doctored_pstate_jump_fires_its_guard` (8→9) — both fire NAMING the constant
- engine/game-split property gate: `jump_lock_off_compiles_without_game_symbols` — `-D GAME_CAMERA_JUMP_LOCK=0` lowers+links with NO `_pl_state`/`PSTATE_*` equs provided AND asserts the gated block is elided (shorter region)
- region pin: `pins::CAMERA` (repin-derived); gate `SIGIL_EMP_CAMERA` + per-shape orgs in engine.inc; gate-off dual rebuild reproduced then-current CRCs exactly
- no ownership flip (zero .emp callers of any t19 proc — checked) → no two-module link test owed

**bg** (region plain $6116−6 / debug $6DEA−6; $AE shape-invariant):
- byte gates both shapes: `bg_port::bg_region_matches_reference` + debug — green FIRST compile
- negative probe: `doctored_vram_sprite_table_fires_its_guard` ($B800→$B000, the SAT ceiling)
- region pin `pins::BG`; gate `SIGIL_EMP_BG`; gate-off CRCs exact

**bg_anim** (region plain $61C4−6 / debug $6E98−6; $A0 → $9E plain / $100 debug):
- byte gates both shapes: `bg_anim_port::bg_anim_region_matches_reference` + debug — green FIRST compile; debug re-green after the assert (+0x62, the .emp assert expansion byte-matched the twin macro blob on the first run — row-21 twin-parity held)
- no doctored probe: bg_anim declares no extern-locked mirrors (documented in the test header)
- `BgAnim_Table` rides the PIN TABLE (game data — shifts with the data region), `QueueDMA_Deferrable` extern-proc decl = dplc row-32 spelling verbatim
- region pin `pins::BG_ANIM` (literal-length region — the next placement's only label is `__DEBUG__`-gated; sound_api precedent; now shape-DEPENDENT)

## Byte-delta table (measured, not predicted)

| Change | Δ plain | Δ debug | Absorbed by |
|---|---|---|---|
| camera step-2 bare-Bcc (twin relaxes `bra.w .no_move`/`bne.w .clamp_y`/`bra.w .clamp_y`, distances 74/108/40) | −0x6 | −0x6 | repin (4 regions + 3 sound pins), engine.inc orgs ×3, repin_pins changelog |
| bg step-2 | 0 | 0 | — (twin already all-`.s`) |
| bg_anim step-2 (`beq .exit` relaxes — measured in-range, my manual estimate was wrong; `jsr`→`jbsr` re-emits bsr.w, size-neutral vs abs.w jsr) | −0x2 | −0x2 | repin.toml literal len, engine.inc orgs ×2, repin_pins changelog |
| pass-1 step-4 band-count assert | 0 (self-gates) | +0x62 | bg_anim debug_len $9E→$100 (shape-dependent), engine.inc debug orgs ×2, repin_pins changelog |
| panel adjudication: piece-1-length assert + twin's 2 ifdef-widened spanning branches; Sst.y_vel(a0) encoding (same-length) | content only | +0x58 | bg_anim debug_len $100→$158, engine.inc debug orgs ×2, repin_pins changelog, sound_api_port synthetic consumer $8000→$9000 (debug region end crossed $8000 — the pinned-collision class) |
| convsym symbol appendix (`.no_move`→`.x_done` rename + new consts) | CRC+size only, zero code bytes | same | none (PROVENANCE at merge) — NOTE: "region-byte-neutral" ≠ "ROM-CRC-neutral" when symbol names change |

## Step-2 filled checklist (per file — all seven items walked)

1. Branch conversions: all three files bare-Bcc + jbra/jbsr; deltas above; `jsr QueueDMA_Deferrable`→`jbsr` judged engine-internal (dma_queue is fixed-placement engine bank — the game_loop `jsr Debug_MusicToggle` cross-section exception does NOT apply).
2. Width pins with site comments: NONE kept in any of the three files (every conditional went bare; no structural-pin classes present).
3. Bare-symbol width-rule: complete (all RAM refs bare; `Z80_BUS_REQUEST` bare→abs.l; `BgAnim_Table` bare→abs.l).
4. Brace-indent: file-wide, all three.
5. Idiom list walked: `Sst.x_pos/y_pos` + `offsetof(Sst, y_vel)` (camera); `Act.field(aN)` everywhere (no file-local Act offset mirrors); typed VDP `vdp_comm` (bg); contract reglists in RANGE form throughout; label-in-immediate n/a; `(Sym).w` operand-override NOT used (bare spellings; the one link-sum width problem was solved by the comptime-select equ, not a pin).
6. Type-layer walk: ADOPTED `(a0: *Act)` on Camera_Init + BG_Init (section.emp idiom); `vram_dest: VramAddr` field (F3 vocabulary). LOGGED-not-typed with reasons: camera start-pos/grid shift-chains + 16.16 packs (A4-i-gated, item-13 wave-2); bg length values (VramAddr-in-arithmetic, same gate); bg_anim driver select 0/1/2 (closed vocabulary but runtime-table words; emitter is Python); player-state byte (game-domain — engine can't own the game vocabulary; candidate when player files port).
7. Noticing: ONE proposed addition — **the comptime-select game-contract mirror idiom** (gated code = proc-body `if GATE==1`; gated drift-ensures = `ensure(select_fn())`; gated link address = `equ = select_fn()`); camera.emp is the worked example, property proven by the =0 link probe. Ledgered as a step-2 checklist candidate.

## PER-PASS: step-3 vs step-5

**Pass — steps 1-2 (per file):**
- *step-3 flavored:* the game-seam spelling settled by three binding-class probe rounds (module-scope `if` not grammar; `Bool||LinkExpr` rejected; unused equ resolves eagerly; operand-override can't defer a link base — row-1004 confirmed live); SECTION_SIZE_SHIFT split-shift spelled as `#8` + `#SECTION_SIZE_SHIFT-8` with a range ensure (both twins); closure gate caught BG_Init's undeclared a3 → `preserves(a3/sr)`.
- *step-5:* branch modernization relaxed 4 conservative widths (−8 total both shapes) — size wins, not cycle wins.

**Pass 1 — 3(b) fixes (aeon `e5f5581`):**
- Camera_Update header Out-claim fixed (X AND Y — overseer catch, both twins); Camera_Init tightened `clobbers(d0/a0)`→`clobbers(d0)` and BG_Init dropped a0 (bodies never write a0; callers checked); `CAM_X_DEADZONE_INIT`/`CAM_Y_DEADZONE` named (both twins); `.no_move`→`.x_done` rename; comment-claim audits ALL VERIFIED TRUE against current sources (MEGA-ACT ensures live at act_descriptor.emp:42-44; inject_editor_bg.py BG_TILE_CAPACITY=448; spindash writes #16).
- **Tightening REVERTED by the verifier:** `preserves(a3-a4)` on BgAnim_Update → `[proc.preserves-unverifiable]` (the 3-word SP juggle blocks the movem round-trip proof). Honest wide license kept + comment; proof-extension candidate ledgered.

**Pass 1 — step-4 (all queued adjudications named):**
- stop_z80/start_z80 → **`engine/z80_bus.emp` BUILT** (2nd-consumer lift; kill row 36; sound_api retrofit = step 6).
- VDP_DATA/VDP_CTRL → **hoisted into `engine.vdp`** (3rd consumer; t18 B1 debt executed; parallax/plane_buffer retrofit = step 6).
- `vdp_reg`/$8F02 → **NOT built** (t17 verb-(c) stands; demand +2 ledgered; both bg sites keep literal + comment).
- `vdp_blocking_blit` (bg's 2-site copy clone) → **NOT built, reason logged** (site-varying middle needs inline-Code args or a split begin/loop pair with an unpaired-use hazard; 2 sites don't pay it; demand data attached to the typed-VDP-home ask).
- `bganim_band` struct → **BUILT** (all walk magics derived + sizeof==44 ensure; emitter LOCKSTEP note same-commit).
- Band-count assert → **ADOPTED per gate ruling** (`assert.w d7, ls, #BGANIM_MAX_BANDS` both twins; debug +0x62; plain zero bytes).
- `clamp_camera_axis` → **BUILT** (13-instr X/Y clone single-sourced, byte-identical; MEGA-ACT comment single-sourced; kill row 37).

**Pass 1 — step-5 (FULL interrogation + measured numbers):**
- *Invariant ladder:* Camera_Update is LOOP-FREE (n/a); BgAnim_Update band loop has no hoistable invariants (per-band record reads).
- *Counter/cache audit:* Camera_Hold_Frames writers/readers balanced (spindash 16 / Init clr / Update subq+read); BgAnim_LastStep commit-on-success asymmetry documented-intended (partial pair → redo next frame).
- *Guard-coverage:* camera clamps run on ALL paths incl. freeze (designed); bg capacity clamp is the sole SAT guard (load-bearing — named); bg_anim band walk now assert-bounded (DEBUG).
- *Hardware cross-check:* camera touches no VDP; bg/bg_anim deferred to lens C3.
- *Silent-tradeoff comments:* freeze-skips-follow-but-clamps-run, asymmetric X window, jump-lock bottom-edge failsafe, partial-pair redo — all carried and verified present.
- *Measured (overseer-run oracle, plain branch build; ROM identity byte-verified at $59CE first; 120-frame profiler averaging cap noted):* **Camera_Update 670 cyc/f ambient / 716 cyc/f saturated follow (0.5-0.6% of ~128k)**; **BgAnim_Update 124 cyc/f** (BgAnim_Table read live at $256DE = $0000 — count-0 exit path confirmed, number is exit+attribution overhead). Follow-path method: teleport-chase saturating the 16px/f cap (MCP pacing can't do per-frame pokes — deviation recorded); single-frame sampling can't prove spike absence, but the proc is loop-free so the worst path is statically bounded a few dozen cycles above measured.
- *Live behavior bonus (P3):* right-chase parked at Camera_X = player−160 EXACTLY (centre boundary), left-chase at player−144 (the CAM_X_DEADZONE_INIT=16 asymmetric window's left edge), Camera_Y clamp held — the documented window semantics observed live on the modernized code.
- **VERDICT (gate-endorsed): NO CUT — nothing reaches the ≥1k cyc/f threshold (670-716 measured against it).** Log-and-skip items: double `lea Player_1` reload + Camera_X re-read in the clamp (~tens of cycles, blocked-ish by the d4 reservation risk profile); BgAnim worst-case 4-band change-frame ≈0.7-1.4k is DESIGNED enqueue work already change-gated.
- *Type-layer rider:* no register reshuffles taken → no blessings moved.

**Pass 2 (post-probe):**
- *step-3/4:* Camera_Pan_Offset adjudicated (below); `vram_dest: VramAddr` typed (domain-scan take).
- *step-5:* numbers folded; no new surface (pass-1 changes byte-neutral or debug-only). EMPTY otherwise.

**Pass 3: EMPTY at all three steps → dry claim → panel dispatched; round adjudicated above (DRY STOOD).**

**Panel-adjudication pass (post-round, gate-visible):**
- *step-3 flavored:* truthful retry/topology/Z80-hold comments (both twins); d4
  NOTE placement; template control-flow clause; CAM_MAX_X_STEP asymmetry clause;
  kill row 32 second site; 6 ledger rows (asks + hardware + SR-bracket).
- *step-5 flavored:* piece-1-length DEBUG assert (+0x58 debug; plain zero) —
  diagnostics, not behavior; the C1 sub-threshold batch row (threshold held);
  `Sst.y_vel(a0)` encoding + `banks[BGANIM_BANKS]` + `GRID_* >= 1` ensure.

## PANEL ROUND (A1+B1+C1+C2+C3, all read-only; one round per the dry-claim rule)

**DRY STOOD** (t18 precedent: adjudication yielded diagnostics/comments/vocabulary
+ ledger rows — no algorithmic, construct, or optimization re-work).

- *C1 (perf):* verdict ENDORSED-with-teeth — "no cut ≥1k stands, but closer than
  the packet implied": ~330-420 cyc/f of real sub-threshold folds enumerated with
  numbers (clamp-bound hoist ~150 + skip-write ~50, `<<16` swap idiom −44/site,
  abs reads −16, zero-band early-out −84 LIVE in the shipped act, cold move.l
  blit). ALL banked in one ledger batch row for the post-conversion sweep
  (each is a lockstep+repin byte-changer — the reason they batch).
- *C2 (correctness):* cursor arithmetic re-derived CLEAR on all four exit paths;
  CC-clobber CLEAR (incl. carry-across-rts at both bcs sites and the
  SR-preserving assert expansion); d4 reservation CLEAR through the spliced
  template; stack balance CLEAR. Four real catches: (1) piece-1 length has no
  guard against table drift (→ 128KB spray) — **DEBUG assert SHIPPED** (the
  gate's band-count ruling applied to its own class; twin takes 2
  ifdef-__DEBUG__ `.w` spanning branches, .emp stays bare per-shape); (2) bg
  length==1 defeats the SAT clamp from below — assert TRIED and REVERTED
  (debug blob pushes `.skip_tiles` past short-branch reach = shape-dependent
  width pins; gate escape clause applied: honest comment + ledger); (3)
  grid-dim==0 makes the clamp bound negative — **`GRID_* >= 1` ensure SHIPPED**
  in act_descriptor.emp (zero-byte comptime); (4) the 128KB single-slot
  carry-clear edge makes bg_anim's commit stick torn art — ledgered onto the
  existing dma_queue rollback work.
- *C3 (hardware):* bracket ordering / autoinc / enqueue-path interrupt hygiene
  ALL-CLEAR with mechanisms cited. Marquee: the wrapped-pair needs no interrupt
  bracket ONLY because deferrable drains solely in VInt_Level, which cannot run
  until VSync_Wait — an undocumented load-bearing call-position invariant, now
  a header CONTRACT COMMENT in BgAnim_Update; even the happy path can split the
  pair across VBlanks via per-entry budget expiry (accepted one-frame seam,
  documented). Z80 held ~21+12 ms at load (DAC gap if music plays — comment +
  ledger); flip-flop status-read hardening + budget overshoot-by-one ledgered.
- *A1 (cold reader):* 5 language asks ledgered (wide-immediate-shift auto-split,
  scoped register reservation for the d4 class, a `mirror const` declaration
  form, typed stream-cursor reads as the unfinished half of the record work,
  named spill frames — the last also unlocks the blocked preserves() proof);
  the partial-retry comment overpromise (**the panel's marquee correctness
  find**, converged with C3): a driver parked back on the committed step leaves
  a half-rotated band indefinitely — truthful comments shipped both twins, fix
  sketch ledgered (poison LastStep with the Init sentinel; behavior fix →
  post-merge, dma_queue-adjacent); cold-reader trace fixes shipped (template
  control-flow clause, .apply_x fallthrough, d4 NOTE placement, banks array
  length derived, CAM_MAX_X_STEP asymmetry clause).
- *B1 (corpus):* `Sst.y_vel(a0)` encoding adopted (a0 live — same length);
  QueueDMA extern-decl duplication adjudicated KEEP (dies at the dma_queue
  port; kill row 32 updated); SR-mask bracket found PAST the consolidation bar
  (6 sites) — ledgered as a construct candidate with the pair-use design
  question, NOT built at adjudication; two step-6 targets contributed
  (section/vdp_init bare VDP spellings); entity_window magic numbers ledgered
  at-next-touch.

## Step-6 corpus sweep (enumeration, per-site outcomes)

Additions with prior-file reach, every site named:
1. **engine.z80_bus** — corpus census `stop_z80|start_z80`: sound_api.emp (2
   local fns + 4 call sites) → **RETROFITTED** (local fns deleted; `use` added;
   sound_api_port + tranche5_negative_probes + mixed_dac_rom ambient arm all
   prepend the module; byte gates re-green = byte-neutral proven). No other
   corpus sites. Kill row 36 updated (row 24's fn-side collapsed in).
2. **engine.vdp VDP_DATA/VDP_CTRL** — corpus census: parallax.emp local
   mirror → **RETROFITTED** (consts + 2 ensures deleted, `VDP_DATA_OFF` derive
   stays local); plane_buffer.emp local mirror → **RETROFITTED** (same shape;
   `VDP_CTRL_OFF` derive stays); section.emp bare-link spelling →
   **RETROFITTED** (imports the consts; abs.l either way, byte-identical);
   vdp_init.emp single bare `lea VDP_CTRL` → **LEDGERED at-next-touch** (its
   port test carries no vdp.emp prepend; the churn isn't paid for one const).
   t18 B1 debt row CLOSED.
3. **clamp_camera_axis** — B1 census: no corpus instance (corpus clamps are
   one-sided cell-counter compares) → not-an-instance.
4. **comptime-select define-gating** — no other GAME_* gates exist in .emp →
   not-an-instance (the step-2 checklist candidate carries it forward).
5. **struct-derived walk consts practice** — entity_window's magic layout
   numbers → LEDGERED at-next-touch (B1.8 row).
6. **bganim_band struct / band asserts** — file-unique → no sweep.

## NEITHER-BUCKET HEADLINES

- **The engine/game-split seam got its idiom:** `-D GAME_CAMERA_JUMP_LOCK` + comptime-select guard fns/equ — the =0 arm short-circuits before `extern()` evaluation, so a lock-less game never resolves the symbols; proven at the REAL binding class by a link probe, not a comment. Three probed-out walls ledgered (module-scope `if` absent; `Bool||LinkExpr` residual rejected; **unused equ resolves eagerly**).
- **Two consolidation debts EXECUTED mid-tranche** (z80_bus 2nd-consumer lift; VDP-const 3rd-consumer hoist) — the demand-gated rows paid out exactly as written.
- **The contract verifier earned its keep twice:** caught BG_Init's undeclared a3 (closure corpus), and REFUSED an over-eager preserves(a3-a4) tightening the porter believed true ([proc.preserves-unverifiable]) — the second one is a soundness win: the tool would not let documentation outrun proof.
- **Camera_Pan_Offset feature-dead-code flag** (write-only engine-wide; Sec.sec_camera_lookahead corroborates lookahead intent) — verb-(d) KEEP ruling, site comments + ledger row with kill condition; surfaced to Volence for possible override.
- **The debug-blob-vs-branch-reach tension surfaced as a real design constraint:** a DEBUG assert whose blob sits inside a short branch's span forces either shape-dependent twin widths (taken for bg_anim: `ifdef __DEBUG__ beq.w`, .emp stays bare and relaxes per shape — a NEW twin-side pattern) or dropping the assert (taken for bg). Feeds the wide-immediate/lazy-width asks.
- **A pinned-carrier collision class resurfaced benignly:** the bg_anim debug slide pushed sound_api's region end past $8000, colliding with sound_api_port's synthetic consumer pinned there since tranche 5 — caught by resolve_layout, consumer moved to $9000 (the mulu/13-gate "+0x78 bank growth" class, this time inside one test).
- **Probe results recorded verbatim:** profiler averages cap at 120 frames; P2 ran as teleport-chases (MCP pacing can't do per-frame pokes); P3's park positions matched the documented asymmetric window EXACTLY (player−160 right / player−144 left).
- **Process:** cwd-reset discipline bit twice more (camera.emp first written into aeon MAIN — moved same minute, main verified clean; two cargo runs executed in the wrong tree — re-run); default Bash cwd is sigil MAIN, every cargo/cd is explicit.

## POST-MERGE QUEUE (for the record)
- item-13 wave-2 (A4-i-GATED): Coord/Velocity + the camera shift-chain candidates logged this tranche.
- `vdp_reg` typed register-SET ask: census + design (demand now plane_buffer 2 + bg 2 + section literals).
- Sprites-hardening parcel: PARKED (post-conversion).
- dma_queue port (a later tranche) flips `QueueDMA_Deferrable` ownership → owes the two-module link test + deletes the dplc/bg_anim extern decls same-commit.
