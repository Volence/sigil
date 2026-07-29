# 2026-07-29 — t32 close packet (the sound_psg port — rung-2 CONTRACT-model acceptance)

Status: **CLOSE PACKET** — merge gate open (overseer countersigned checkpoint (b)). Porter =
Opus subagent, direct-dispatch (Fable). Target = the FIRST resident-blob Z80 CODE port:
`engine/sound/sound_psg.asm` (526 L, 15 procs) → `sound_psg.emp`, the rung-2 contract system's
named acceptance corpus. Two passes on one branch pair: the porter's item-9 (oracle + byte
identity + checkpoint-(a) findings), then an overseer-dispatched finisher closing the three
under-wired contract gaps the acceptance surfaced (63 firings → 0). This packet closes the
tranche for the sequential merges.

## 0. Bars (overseer-countersigned at checkpoints a + b)
- Branches `port-tranche32` both repos; rebased onto the post-t31 masters. Tips: **sigil `6bb52aa`**
  / **aeon `766d273`**, clean.
- **Canonical ROMs = the post-t31 canonical, UNCHANGED by this tranche**: plain **`85111814`/421041**
  · debug **`eb5e94be`/429102** (t31's game-bank wave set the new baseline; the psg `.emp` adds
  ZERO — not wired into any build; the engine-blob windows $1660/$16DE are game-bank-independent).
  Verified by the porter's own dual rebuild AND the overseer's.
- **Strict-paired 2783/0 (1 ignored)** with `AEON_DIR` at the aeon worktree (overseer-countersigned).
- **EXPECTED BYTE MOVEMENT: ZERO — HELD** (the `.asm` twin stays canonical; blob-precedes-engine).

## 1. What landed

**Porter item-9 (byte identity + the acceptance findings):**
- aeon `9f239c6` + sigil `961566d`/`3676c13` — `sound_psg.emp` (15 procs) + the `sound_psg_port`
  WINDOWED oracle (5 tests, byte-identical BOTH shapes) + the demanded-feature operand fix
  (bare comptime symbol in a Z80 8-bit imm/ALU/bit operand) + kill row 67 (now 70) + the
  checkpoint-(a) findings ledgered. The FULL faithful contract set fired **63** and could not go
  live → psg LANDED with the reduced clean set + prose (the STOP for the overseer).

**Finisher pass (overseer-dispatched, the three under-wired gaps — sigil `d1830ee`/`7441795`/`fa122a3`):**
- **Gap 1** (§13.5-A) — `clobbers`/`out` wired to the Z80 regfile (`check_clobbers`/`check_out`
  gain a `cpu` param → `expand_reglist(RegFile::Z80)`; out∩clobbers/preserves overlap Z80-expanded;
  out-UNWRITTEN skipped on Z80, honest like `preserves`). 43 firings cleared.
- **Gap 2** (§13.5-B) — the `CalleePreserves` oracle (the Z80 analog of `preserves.rs`'s
  `CallPolicy::Oracle`): a `call` clobbers only the units the callee does NOT declare-preserve.
- **Gap 3** (§13.5-C/D) — the vacuous tail-jp pass CLOSED: `Edge::Defer` now checkpoints + sets
  `saw_return`; the self-catch `Cfg::is_local_label` routes a local end-label jump to `Edge::Abandon`
  (in-proc fall-off), not the external-tail edge. 20 firings cleared.
- aeon `766d273` — the FULL rung-2 register contract set applied: **63 → 0**, `invariant(ix)` LIVE +
  inherited on all 15 procs, `clobbers`/`out(<reg>)`/`out(hl, carry: found)`, `falls_into`, 3
  `extern proc` callee contracts. Byte-identical both shapes (`sound_psg_port` 5/5).

**Porter step-3 (records, sigil `6bb52aa`):** kill row 70 present-tense; the two STOP-findings marked
CLOSED with refs; the header-inaccuracy row updated (3 over-claims machine-corrected); two §13.5
residue rows added.

## 2. Byte-delta table — ZERO
| Commit | Repo | Byte delta |
|---|---|---|
| 9f239c6 · 766d273 | aeon | 0 (`.emp` not wired into any build; oracle-only) |
| 961566d · 3676c13 · d1830ee · 7441795 · fa122a3 · 6bb52aa | sigil | 0 (frontend fix + contract wiring + tests + notes) |

Dual build **85111814/421041 · eb5e94be/429102** unchanged at every step.

## 3. THE RUNG-2 ACCEPTANCE — FINAL VERDICT (headline: 63 → 0)

The full contract system is LIVE and machine-checked on all 15 procs, byte-identical both shapes.

| Capability | At checkpoint (a) | Now |
|---|---|---|
| `module invariant: preserves(ix)` (FIRST live use) | ❌ false-fired on 5 calling procs | ✅ LIVE, inherited + verified on all 15 (push/pop model + callee oracle) |
| `clobbers(...)` / `out(<reg>)` on Z80 regs | ❌ 43 firings (68k-regfile validation) | ✅ WIRED to the Z80 regfile (incl. the `af`→`out(a) clobbers(f)` split) |
| calling-proc `preserves` (5 procs) | ❌ 18 firings (call-clobber-all) | ✅ verified via the `CalleePreserves` oracle |
| tail-jp `preserves` (3 procs) | ⚠️ passed VACUOUSLY (silent-pass hole) | ✅ checkpointed at the tail edge (soundness hole CLOSED) |
| `out(carry: found)` | ✅ declared/valid | ✅ (cross-proc consumption awaits the sequencer port) |
| `falls_into` chain (ApplyMod→EmitDivisor→EmitDivisorTo) | ✅ | ✅ |

### The acceptance dividend — what porting psg caught that the rung-2 tests never did
- **Two implementation gaps** vs the design: (a) `clobbers`/`out` never plumbed to the CPU-aware
  recognizer (§13.5-A); (b) the callee-preserves oracle the 68k side got at t30 was never given to
  the Z80 sibling (§13.5-B — the §3.2 "invariant(ix) trivially satisfied" claim contradicted by its
  own acceptance corpus, because every item-5/6 fixture was call-free).
- **One soundness hole + its self-catch:** the vacuous tail-jp pass silently verified a broken
  `preserves` (§13.5-C); closing it surfaced a local end-label mis-read as an external tail (§13.5-D).
- **Three 15-year-old header over-claims**, now machine-corrected: Psg_EnvCursorReset `Clobbers af`
  (clobbers nothing); Psg/FmVolEnv_Resolve `Clobbers bc` (only `b` dies, `c` survives); Psg_EmitDivisor
  `Clobbers af, bc, de` (`de` push/pop-bracketed + `b` survive) — the exact "a false clobber comment
  caused a prior bug" class the psg header warns of.
- **One demanded operand-model feature** shipped en route: a bare comptime symbol in a Z80 8-bit
  immediate / ALU / bit-number operand now folds (psg is the first Z80 code with symbolic imm8s).
- **One parser sharp-edge** worked around byte-identically: `(ix+field+k)` compound displacement →
  parenthesize `(ix+(field+1))` (ledgered; parser fix deferred).

## 4. Window derivation (the scale-1 proof)
psg is the 4th of 5 includes in z80_sound_driver.asm's `cpu z80 / phase 0` blob. **SHAPE-VARIANT
window** (correcting the brief §1 shape-invariant claim): base **$1660** (plain, s4.lst) vs **$16DE**
(debug, s4.debug.lst), +$7E from upstream `__DEBUG__` growth. psg's OWN layout is shape-invariant (no
`__DEBUG__`; every inter-label delta matches), so the shapes differ ONLY in the internal call/jp
absolute targets + the Snd_ChanClass address — hence BOTH shapes are byte-gated (444 B / $1BC each).
Comptime constants ride as `-D` defines; cross-seam symbols (banked tables + external calls) as equ
carriers. Oracle: `sound_psg_port` 5/5 — the two byte gates + `plain_and_debug_shapes_differ`
(shape-variance) + the two t24 controls.

## 5. Step-6 sweeps (overseer-ordered)

### 5.1 Z80 contract coverage census — the demand table for fm (9b) + rung 3
Of the five resident Z80 code files, **psg is now fully contracted**. The header-declared contract
surface remaining (grep of `;`-comment `Clobbers`/`Preserves`/`Out:` lines — INDICATIVE upper bounds;
multi-line headers slightly over-count, cf. psg's 17/16 for 15 routines):

| File | Lines | Routines | Clobbers-hdrs | Preserves-hdrs | Out-hdrs | Port class |
|---|---|---|---|---|---|---|
| **sound_psg** | 526 | 15 | — | — | — | **DONE (t32)** — full contract set LIVE |
| sound_fm | 998 | 22 | 20 | 22 | 6 | **item 9b (next)** — invariant-heavy (de=$4001/$2A prose, ix everywhere) |
| sound_sequencer | 2091 | 51 | 25 | 20 | 2 | rung 3 — interpreter; the `ex (sp),hl` trampoline lives here |
| sound_sfx | 1627 | 23 | 28 | 11 | 11 | rung 3 — struct-prefix mirror; reuses sequencer's Mod* |
| z80_sound_driver | 1495 | 21 | 17 | 4 | 3 | rung 4 — cycle-exact DAC loop; di/ei + shadow-set |

Reading for fm (9b): 22 routines, ~22 `Preserves` + 20 `Clobbers` + 6 `Out` headers to make machine
contracts. fm additionally demands the `preserves(ix)` module invariant (every fm header lists ix)
and will exercise the DEEP push/pop LIFO nesting (`sound_fm.asm:219-239/265-331/463-529`) the psg
proof only lightly touched (psg's deepest is a single `push de`/`pop de` + `push hl`/`pop hl`
brackets). fm's `$2A`-parked / `$2B`-never-written are the hardware-lint SEAM (design §3.3/§9-B),
NOT register contracts. The `out(carry:)` cross-proc credit first exercises end-to-end when
sequencer ports (its `FmVolEnv_Resolve`/`PsgVolEnv_Resolve` callers live there).

### 5.2 Declaration-trust exposure census — the psg oracle's trust surface
The callee-preserves oracle credits DECLARED preserves without a per-proc closure (§13.5-B residue),
so every `extern proc` contract the psg oracle trusts is a trust obligation. **All 3 were
verified-against-header (C3, read-only against the resident tree) — the trust is well-placed:**

| Extern (trusted) | `.emp` declared | Real resident header | Verified |
|---|---|---|---|
| `Snd_ChanClass` | `preserves(bc, de, ix)` | `Clobbers af. Preserves bc,de,ix` (sound_fm.asm:119; Out: hl=ix) | ✅ |
| `Mod_ReArm` | `preserves(bc, de, hl, ix)` | `Clobbers af. Preserves bc,de,hl,ix` (sound_sequencer.asm:~800) | ✅ |
| `Mod_Advance` | `preserves(ix)` | `Clobbers af,bc,de,hl. Preserves ix` (returns de; sound_sequencer.asm:~862) | ✅ |

The trust surface is 3 procs, ALL in files not yet contracted (fm, sequencer). When those port,
the externs become CHECKED definitions and the trust converts to verification — the closure work's
input set is exactly these 3 today. Note: Snd_ChanClass does NOT preserve hl (it writes hl=ix), so
psg brackets hl with push/pop around it — correctly uncredited by the oracle (not in its preserves).

## 6. Corrections list (brief/design errors caught — tree wins)
- **(a) Brief §1 "shape-INVARIANT" claim FALSE** — psg's phase-0 window is shape-VARIANT ($1660
  plain / $16DE debug); the brief expected invariant and asked for evidence either way. Caught at
  step 0 from the listings; both shapes byte-gated as a result. (psg's own layout IS invariant —
  the variance is the upstream `__DEBUG__` base shift.)
- **(b) The "PURE PORT" premise falsified the t27 way** — the brief framed t32 as pure port (T1 +
  rung-2 complete). The acceptance corpus surfaced THREE contract-system gaps + one operand-model
  gap the wired set lacked (the exact t27 pattern: "the satellites demand nothing new" was false).
  Resolved in-tranche (the operand fix by the porter; the three contract gaps by the finisher) —
  NOT deferred, because each was a small demanded-feature matching documented design intent.
- **(c) Design §3.2 self-contradiction** — "invariant(ix) trivially satisfied (no instruction writes
  ix)" is true only for a call-free proc; the call-clobber-all model made a `call` write-all, so the
  claim failed on psg's 5 calling procs. The acceptance corpus is precisely what caught it (§13.5-B).

## 7. Kill-list + ledger state
- **Kill row 70** (sound_psg twin) — present-tense: full contract set LIVE; the 3 header over-claims
  machine-corrected; oracle = the sole drift guard until the scale-2 resident-blob seam.
- **Ledger** (section "t32 psg port"): the bare-symbol-imm8 fix (SHIPPED); the `(ix+field+k)`
  parser ask (RECORDED, stands); the two STOP-findings (CLOSED with refs); the header row (all 3
  over-claims machine-verified); two residue rows (declaration-trust cycles; out(carry:) cross-proc).

## 8. Residue for future rungs (ledgered)
1. **Callee-preserves declaration-trust cycles** — the per-proc oracle trusts declarations without a
   verified `effective` fixpoint; a cyclic mutual-preserve would be trusted (sound for acyclic psg;
   the 68k closure would fold it conservative). Named for a Z80 preserve-cycle if one appears.
2. **`out(carry:)` cross-proc consumption** — declared/valid in psg but the four `jr c` consumers are
   in sound_sequencer.asm; the `[call.flag-result-unused]` credit exercises end-to-end at the rung-3
   sequencer port.
3. **The `(ix+field+k)` flat-form parser fix** — bind a leading index register before the
   displacement arithmetic; parenthesized `(ix+(field+1))` is the standing house form until then.
4. **fm's `$2A`-parked / `$2B`-never-written hardware-port lint** (`[fm.dac-repark]`, `[bus.*]`
   family) — the item-9b seam, NOT a register contract (design §3.3/§9-B).

## 9. Named next Z80 steps
1. **fm (item 9b)** — invariant-heavy; the coverage census (§5.1) seeds its brief. Exercises the deep
   push/pop LIFO nesting + the module `preserves(ix)` invariant + the hardware-lint seam.
2. **The interpreters (rung 3)** — sequencer + sfx; the `ex (sp),hl` trampoline bail goes live; the
   `out(carry:)` cross-proc credit closes; the extern trust surface (§5.2) converts to verification.
3. **The driver top (rung 4)** — cycle-exact DAC loop; di/ei lattice + shadow set + the T-state
   accounting the design reserved.
