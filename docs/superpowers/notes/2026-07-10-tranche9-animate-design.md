# Tranche 9 step 0 — animate.asm port design (2026-07-10)

Target: `engine/objects/animate.asm` (408 lines; procs `AnimateSprite`,
`AnimateSprite_PerFrame`, `RefreshSpritePieceCount`; the `reloadAnimTimer`
macro; the AF_* control-code block) → `engine/objects/animate.emp`.
Ratified by Volence at the tranche-8 gate (handoff
`notes/2026-07-10-tranche9-handoff.md`).

Baseline verified BEFORE any edit: `s4.bin` = `c973091d…` ✓,
`s4.debug.bin` = `6a0f9c3f…` ✓ (both match the PROVENANCE pins; listings
current, 2026-07-10 20:20).

## Region geometry (from the 2026-07-10 master listings; RE-DERIVE at pin time)

animate runs from sprites.asm's end to collision's gate base:

- plain: `s4.bin[$2D78..$308A]` — 0x312 bytes
- debug: `s4.debug.bin[$3032..$3344]` — 0x312 bytes

Length is **shape-INVARIANT** (no `__DEBUG__` code — the handoff called
it). The `Shape` struct keeps per-shape `len` fields anyway (the rings
precedent — equal values, one less special case). Region tail is clean:
`RefreshSpritePieceCount`'s `rts` lands exactly on the boundary, no
padding. Proc offsets (plain listing): AnimateSprite +0,
AnimateSprite_PerFrame +$17A ($2EF2), RefreshSpritePieceCount +$2F4
($306C). Debug offsets identical (invariant length; debug listing agrees:
$31AC/$3326 from base $3032).

**UPSTREAM EXPOSURE (hazard 4)**: any step-2/5 byte change slides
collision ($308A/$3344), rings ($31F0/$34AA), collision_lookup
($4C1A/$543E), sound_api ($5F4A/$7408) gate orgs + region bases + every
label pin + the mixed_dac_rom map fns. The generalized re-pin rule
applies: re-derive EVERY pin in the window FROM LISTINGS, sweep grep hex
literals, let the strict suite name survivors.

## D1 — rows 2/3 kill condition: CONSOLIDATION, not flip (hazard 1 settled)

The written kill ("animate.asm ports → flip") is unexecutable, exactly as
row 13's lesson predicts. Verified AS-side AF_* readers that survive this
port:

- `games/sonic4/data/sprites/pitcher_plant/anims.asm` (AF_END) — NOT
  gated; assembles in the MIXED build.
- `sonic_anims.asm` / `particle_anims.asm` / test-object twins (AF_END,
  AF_BACK, AF_DELETE, DUR_DYNAMIC) — gate-off twins; assemble in the
  REFERENCE build.

With `SIGIL_EMP_ANIMATE` set, animate.asm vanishes from the AS side —
and the AF_* equs it owns would vanish with it, breaking the mixed build.
A reverse seam (.emp exports equs to AS) is row-4-stage-2 / Spec-5 era.

**Decision: re-home the AS-side truth.** The nine AF_* equs move from
animate.asm to `engine/constants.asm`, directly after `DUR_DYNAMIC`
(line ~255 — the animation neighborhood; the DUR_DYNAMIC comment already
narrates AnimateSprite's contract). Byte-neutral (equ moves emit
nothing). animate.asm keeps a pointer comment. Kill-list outcome:

- **Row 2 reworded**: truth becomes `engine/constants.asm`; kill joins
  row 1's condition (constants.asm ports → flip).
- **Row 3 CLOSED by consolidation**: sonic_anims.emp's module-local
  AF_END/AF_BACK/DUR_DYNAMIC + 3 ensures die; it imports from
  `engine.constants` like particle_anims already does for AF_DELETE.

## D2 — constants twin growth (consumed-only, 24 → 30)

`engine/system/constants.emp` grows exactly what .emp consumers demand
(no speculative block — AF_CHANGE/ROUTINE/CALLBACK/SOUND/COLLISION have
no .emp consumer yet and stay unmirrored):

| new twin entry | value | .emp consumer | truth after D1 |
|---|---|---|---|
| `AF_END` | $FF | sonic_anims.emp (row-3 consolidation) | engine/constants.asm |
| `AF_BACK` | $FE | sonic_anims.emp (row-3 consolidation) | engine/constants.asm |
| `AF_SET_FIELD` | $F7 | animate.emp (the $F7+ dispatch threshold, cmpi ×10) | engine/constants.asm |
| `DUR_DYNAMIC` | $FF | animate.emp (reload_anim_timer) + sonic_anims.emp | engine/constants.asm (already owns it) |
| `OBJ_CODE_BANK` | 1 | animate.emp (AF_CALLBACK moveq/swap bank assembly) | engine/constants.asm:148 |
| `FRAME_PIECE_COUNT` | 4 | animate.emp (RefreshSpritePieceCount d8-index disp) | engine/constants.asm:481 |

All sit in immediate/moveq or d8-displacement positions — the imm-link
deferral gap forces mirrors (row 10/18 class). `AF_DELETE` already in the
twin. `test_support.rs::engine_constant_equs()` grows 24 → 30; every
count derives via `twin_guards()`. Harness drift-guard total for
animate_port.rs: 30 SST + 30 twin = 60. sonic_anims' own port-test counts
adjust down 3 (its locals die).

## D3 — `reloadAnimTimer` macro → module-local comptime fn (hazard 2 settled)

`comptime fn reload_anim_timer(src: Reg) -> Code` in animate.emp — the
aabb precedent. The `tag` param IS the utag-death pattern: hygiene gives
each expansion its own `.rt_static`, the param is obsolete. Module-local
(not pub) → no cross-module twin → **no new kill row**; the .asm twin
keeps the macro, LOCKSTEP comments both sides. Two call sites
(`reload_anim_timer(a1)` in the advance and anim-changed paths).

## D4 — SND blocks (hazard 3)

Two `ifdef SOUND_DRIVER_ENABLED` sites (both AF_SOUND handlers: the
`movem.l a1/d1` save + `bsr.w Sound_PlaySFX` + restore). `.emp` spells
`if SOUND_DRIVER_ENABLED == 1 { … }`, `-D` convention. Reference gates
run SND=1 both shapes; an `as_twin_bytes(snd_on)` combo probe (rings
shape — animate.asm is include-free, simpler) covers SND=0/1 at the
PLAIN base against a freshly assembled AS-twin oracle. No `__DEBUG__`
dimension exists in this file.

## D5 — new/edge operand + reference forms (hazard 5 expanded)

1. **`jmp .cc_table-4(pc,d0.w)`** (×2) — local-LABEL ARITHMETIC in the
   d8 slot of a PC-indexed EA, target computed (jmp's reserved role,
   stays jmp). Prior art: `jsr Touch_HandlerTable(pc, d4.w)`
   (module-item, no arithmetic), t8's local-label disp operands (d16(An)).
   The `-4` cannot be absorbed by a relocated label (it lands inside the
   jmp instruction itself). If the form doesn't lower, it's a step-1
   demanded feature (the file demands the bytes).
2. **`bra.w AnimateSprite.cc_delete`** — the PerFrame dispatch table's
   $FB entry branches into ANOTHER proc's local label. Spec §5 has the
   surface (`ProcName.label`, read-only; possibly via `export .name`).
   First real consumer in the corpus. If lowering/export is incomplete,
   step-1 demanded feature.
3. **`jsr (a2)`** — register-indirect computed call (AF_CALLBACK), jsr's
   reserved role, stays.
4. `bra.w` dispatch tables (×2, 9 entries each) — LOAD-BEARING
   (pc-indexed jmp lands on 4-byte slots): exempt from the bare-Bcc rule
   at step 2, commented in place (the ratified exception class).
5. `1(a1,d1.w)` / `2(a1,d1.w)` / `3(a1,d1.w)` d8-index EAs throughout —
   collision/rings precedent, expected to just work.
6. `FRAME_PIECE_COUNT(a1,d2.w)` — comptime const in the d8-index disp
   slot on an UNTYPED register (const namespace open — row 15's
   typed-register closure doesn't apply).

## D6 — interpreter ≠ DSL (hazard 6, restated as a boundary)

This port writes the byte-command READER as plain procs. No 9d
byte-command DSL surface (re-gated TWICE, D2.26/D2.27). Scripts stay
data; sonic_anims.emp/particle_anims.emp already prove the format.

## D7 — typed surface (hazard 7)

SST anim fields already typed (sst.emp). The file reads/writes them via
`Sst.field(a0)` idiom at step 2. AnimId/FrameId newtypes: the file does
NOT demand them (all arithmetic is raw byte inc/dec/index math — a
newtype would need cast ceremony TODAY with no misuse it prevents in
this module alone). Recorded as a step-3 ask with the demand argument
(construct-walk #3 thread: the REAL demand moment is the anim-table/
player-code boundary, where anim ids cross modules), not built.

## D8 — FINDING (new, not in the handoff): `AnimateSprite_PerFrame` is DEAD

Zero callers in the tree (only its own `.pfc_change` self-loop). It is
~$1BA bytes of the region — an engine API for the S3K-style per-frame
duration format, documented in the file header, exported, never called.
Options at step 5: delete (huge re-pin, engine-surface scope call) or
keep (future content may want the format; the .emp port keeps it
byte-locked meanwhile). **Port it faithfully at step 1; headline it in
the packet for Volence's gate** — deleting an engine API surface is his
scope call, not a cycle-level optimization. (user-defers-technical-calls
covers design internals, not public-surface deletion.)

## Step-5 preview (cycle candidates to evaluate AFTER steps 1-4)

- The control-code dispatch double-jump (`jmp table(pc,d0.w)` →
  `bra.w handler`): a dc.w offset table + single indexed jmp would save
  ~2 words/dispatch and 18 bytes/table — measure against dispatch
  frequency (control codes fire once per script loop, NOT per frame —
  likely not worth the re-pin; the per-frame fast path is already
  minimal: timer-decrement + bpl out).
- `.evt_callback`'s `move.l a1,-(sp)` / restore around the jsr — required
  by the callback contract (may clobber a1), keep.
- Fast-path prologue (render_flags/status merge, 3 RMW ops per object per
  frame) — behavior-load-bearing (H/V flip sync), keep.
- Dead-export deletion (D8) — Volence's call at the gate.

## Mechanics checklist

- Branches: sigil `port-tranche9` (in-place), aeon worktree
  `.worktrees/sigil-emp-tranche9` — **SEED EDITOR DATA**
  (`cp -rp games/sonic4/data/editor .worktrees/sigil-emp-tranche9/games/sonic4/data/`),
  harness runs with `AEON_DIR` pointing at the worktree.
- engine.inc:174 gate `ifndef SIGIL_EMP_ANIMATE`, resume orgs
  `$308A` plain / `$3344` debug (re-derived above).
- Harness `animate_port.rs` (rings_port model minus the game-mirror
  probe — animate has NO game-owned mirrors and NO RAM labels; inbound
  labels are just `DeleteObject` $281C/$29AE and `Sound_PlaySFX`
  $5E66/$7324): per-shape reference gates, 60 drift guards, SND combo
  probe, outbound `jsr AnimateSprite` consumer (player_common's shape),
  spot-checks (the `jmp (DeleteObject).l` operand + PerFrame's region
  offset), negative probes, gate-off neutrality.
- lib.rs: `assemble_mixed_tranche9_as_side` (+ `SIGIL_EMP_ANIMATE`);
  mixed_dac_rom.rs: `emp_bank_map_tranche9` + placed-sections +
  acceptance both shapes. Strict 2048/0 before the packet.
- House format at step 2: jbra/jbsr; bare Bcc (exceptions: the two
  load-bearing bra.w tables, commented); Sst.field; sizeof(Sst);
  `jmp DeleteObject` → jbra candidate (BYTE CHANGE −2 → full re-pin;
  batch with any step-5 change to pay the re-pin once).
- Packet ends with the per-pass step-3 vs step-5 breakdown +
  neither-bucket headlines (D8 goes there).
