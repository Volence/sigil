# 2026-07-28 — Z80 recon: the deferred corpus + what .emp needs for Z80

Status: **read-only recon** (Fable, overseer session, 2026-07-28). Zero-conflict with t25
per the standing rules: no master landings until t25 closes; no aeon Z80 byte changes
(the blob precedes the engine — one Z80 byte slides the whole corpus). This note is the
input a future Z80 tranche brief will be cut from. Companion recon shape: the t25 debug-trio
brief (`2026-07-28-t25-debug-trio-brief.md`).

---

## 1. The deferred Z80 corpus (the "~5 Volence-deferred" resolved to files)

The resident blob is exactly **five code files**, assembled as ONE `cpu z80 / phase 0`
unit under `z80_sound_driver.asm` (include order: sequencer → sfx → fm → psg →
tables), emitted inside the 68k `BootData` table and byte-copied into Z80 RAM at boot
(`boot.asm:83-91`; the blob self-pads even, `z80_sound_driver.asm:1476-1482`).

| File | Lines | Role | Port class |
|---|---|---|---|
| `engine/sound/z80_sound_driver.asm` | 1495 | Blob top: DAC streaming loop (MegaPCM-2 model), mailbox, song loader, bank switch, Timer-A tick | **CYCLE-EXACT** — hardest, port LAST |
| `engine/sound/sound_sequencer.asm` | 2091 | Music event-list interpreter + per-frame engine (ModUpdate, envelopes, 30+ `Seq_Op_*` handlers) | Interpreter; off the hot loop (free `jr` use) |
| `engine/sound/sound_sfx.asm` | 1627 | SFX steal/restore engine over 7 `SfxChannel`s; reuses sequencer's `ModUpdate`/`Sequencer_Channel` | Interpreter; struct-prefix mirror contract |
| `engine/sound/sound_fm.asm` | 998 | YM2612 writer + patch load; absolute-addressed port writes (the `de=$4001` coexistence contract) | Writer; invariant-heavy |
| `engine/sound/sound_psg.asm` | 526 | SN76489 writer; per-routine clobber/preserve headers, `ix` preserved everywhere | Writer; **smallest — best first code port** |

Satellites (not counted in the five, but in scope for any tranche):

| File | Lines | Role |
|---|---|---|
| `engine/sound/sound_tables_z80.asm` | 116 | GENERATED (`tools/gen_sound_tables.py`) FM/PSG tables + hand-authored envelope tables; included BOTH resident and banked |
| `engine/sound/seq_opcode_tab.asm` | 68 | Banked 32-entry `dw` jump table (`$E0..$FF`); entries are RESIDENT addresses read through the window as data |
| `engine/sound/dac_sample_tab.asm` | 110 | Banked 9-byte `DacSample` descriptor table; label == its $8000-window pointer |
| `engine/system/z80_init.asm` | 38 | Idle program (no-sound builds); the corpus's ONLY self-modifying code (`ld (hl),0E9h` = patch a `jp (hl)` at addr 0) |
| `games/sonic4/main.asm:201-217` | — | The `save / cpu z80 / phase 08000h` banked head (`soundBankHead` macro, `sound_bank.inc`) |
| `engine/sound_constants.asm` | 1481 | **Single source of truth** for every Z80 RAM offset, mirrored struct layout, and seam assert (68k code + Python tools mirror it) |

House rule confirmed in-source: **no Z80 code executes from the $8000 window — data only**
(`sound_bank.inc:13-17`, `sound_sequencer.asm:117-120`; opcode fetches through the window
traverse the 68k bus and corrupt under contention).

## 2. What sigil already has (more than expected)

- **A working corpus-driven Z80 encoder.** `crates/sigil-isa/src/z80.rs` (1510 L,
  ~74+ catalog forms per `SIGIL_M0_CATALOG.md` §2), golden-oracled against asl
  (`z80_golden_vectors.txt`, 120 vectors), wired through `sigil-backend-z80` and the
  **AS front-end**, which parses the full corpus operand surface and builds the ROM
  byte-identically today. Scope is aeon's blob, not the full architectural ISA
  (e.g. `im 1` only; disassembly deferred).
- **Per-section CPU model, not a global mode.** `sigil_ir::backend::Cpu` rides each
  section; `cpu`/`phase`/`dephase`/`save`/`restore` semantics are probed and
  asl-verified (`frontend-as/src/state.rs` — including the "restore resets padding
  only on CPU change" subtlety).
- **Endianness plumbing**: Z80 `dw` little-endian in the AS front-end (probed); the
  .emp `data` path is section-CPU-driven LE (`lower/data.rs:132-136`) **but unprobed
  on Z80** (gap-ledger row ~825-831 — "probe at the first Z80 code port").
- **Link-imm Z80 vocabulary already specced/implemented**: `winptr()`, `bankid()`,
  `u16le`, `BankPtr16Le`/`Value16Le` fixups, and the `[cross-cpu.unwindowed-pointer]`
  error class (`lower/data.rs:143-217`, spec D2.25/D2.26).
- **The relaxation ladder core is CPU-agnostic by design** and explicitly reserved for
  a Z80 `jr→jp` ladder (spec D2.18 / S2-D13(b)); when it lands, Z80 positions become
  provisional for `here()` (re-audit D2.23's width/fixup table for Z80).
- **jr/djnz range checking at link time** (`sigil-link/src/lib.rs:460-479`) and
  `(ix+d)` fold-time i8 checks (`frontend-as/eval.rs:3623-3627`) already exist.
- **The `[bus.*]` machine-state lint** (`frontend-emp/src/z80_bus.rs`) covers the
  68k SIDE of the seam (stop/start pairing, VDP-write-unstopped) — s4lint absorption #1.

## 3. The gap: .emp's Z80 surface is near-empty

`.emp` sections can carry `(cpu: z80, vma: $0000)` and the lowering dispatches per-CPU,
but `lower_z80_instr` (`lower/code.rs:1653-1692`) accepts only symbolic `jr`/`djnz`
plus eleven no-operand forms (`nop ret exx rrca scf ei di ldir neg…`). **Every
operand-bearing Z80 instruction is `[lower.z80-unsupported]`** — the in-code comment
names the blocker: "the emp operand-class model is 68k-only pending a T1 extension."
Root cause: `value.rs`'s `Reg` is `d0..a7` and `CodeOperand` hardcodes 68k EA modes +
movem masks. There is no Z80 register/operand representation in the language.

**Flag (unresolved):** the "Z80 ladder T1/T2/T3" tiers in the 2026-07-24 Volence jot
(`2026-07-24-t23-boot-brief.md:66`) are referenced but **defined nowhere in either
repo**. Ask Volence to restate what the tiers were, or re-derive them fresh (a proposed
re-derivation is §5 below).

## 4. What Spec 2 constructs mean on Z80 — the design worklist

Ordered roughly by how load-bearing the design call is. Tenet 3 holds throughout:
instruction lines stay hand-written Z80 asm; the power lands in the comptime/type/
contract layer.

### 4.1 Operand model ("T1") — the gate everything else waits behind
Z80 needs its own comptime register classes and operand grammar:
- Register classes: `Reg8` (a,b,c,d,e,h,l), `Pair` (bc,de,hl,sp,af-for-push/pop),
  `Index` (ix,iy) — the analog of `Dreg`/`Areg`/`Reg`. `{r}` splice must work for all.
- Operand forms: `(hl)`/`(bc)`/`(de)`, `(ix+d)` with the i8 fold check, `(nn)`, imm8/16,
  `cc` condition codes, bit numbers 0-7, `af,af'`.
- Design question: one CPU-tagged `CodeOperand` enum vs a per-CPU enum behind a trait.
  The ISA crate already models this cleanly (`z80.rs` `Operand`) — the emp layer should
  mirror it, not reinvent it.
- Module-declared CPU lands here too (gap-ledger rows 196-201): `module x in y (cpu: z80)`
  or default-and-warn — the hblank-port jot's "caller convention, not module fact" hazard
  becomes real the day the first Z80 module exists.

### 4.2 proc contracts on Z80 registers
`clobbers`/`preserves`/`out` translate directly, but the Z80 corpus adds contract
classes 68k never needed:
- **Shadow set**: `exx` / `ex af,af'` mean a proc can clobber/preserve TWO banks.
  The driver holds ROM length in `hl'` across the streaming loop. Contract vocabulary
  needs primed names (`clobbers(hl')` or a `shadow` group).
- **`preserves` verification**: 68k's rule is syntactic movem-pair equality. Z80 has no
  movem — saves are `push`/`pop` sequences (order-reversed). The S2-D6(b) analog is a
  push/pop pairing check, and the sequencer's `ex (sp),hl / ret` computed-jump
  trampoline (`sound_sequencer.asm:1073-1089`) is a known idiom the checker must not
  misread as an unbalanced stack.
- **Interrupt state as contract**: `di`-whole-sample is load-bearing (register-resident
  state). The 68k analog is the deferred S2-D7 `sr` row; on Z80 the interesting slice is
  smaller (di/ei + IFF state) and could ship earlier — it maps onto the existing
  `[bus.*]`-style 3-point MUST lattice (di-held / enabled / unknown).
- **Standing register invariants** (stronger than per-proc preserves): `de=$4001` and
  "reg $2A parked" hold for the DRIVER'S WHOLE LIFETIME across every routine; PSG may
  clobber `de` only because the tick re-establishes it; `ix` is preserved-everywhere in
  the PSG file by project contract. Candidate: a module-scope `invariant` declaration
  the per-proc checker inherits (every proc in the module implicitly `preserves(de)`
  unless it re-establishes). This is exactly the class of correctness-hardening the
  language exists for — today these invariants live in comment prose at 6+ sites.

### 4.3 Cycle-exactness — the genuinely new demand (nothing 68k-side prepared us)
The DAC loop's THREE paths must cost 195/195/194 T-states; the proof is a hand-counted
comment header (`z80_sound_driver.asm:48-110`) and the pads are hand-tuned
`rept N / nop` blocks. The `jp cc`-never-`jr cc` discipline (10 constant vs 12/7) is
prose. A port that silently "improves" a branch destroys the sample clock.
- **Minimum bar (port-time)**: byte-identity gates already forbid drift — the port loop's
  step-1 verbatim gate is sufficient to LAND the port safely.
- **The .emp-native win (step-4/5 era)**: comptime T-state accounting. The encoder
  already knows every form; a `cycles { ... }` block or `ensure(cycles(.fill) ==
  cycles(.drain))` path-cost assert turns the comment proof into a checked fact, and
  pad blocks become `pad_cycles(195 - ...)` derived, not hand-counted. This is the
  standout candidate feature of the whole Z80 story (footguns-fixed-not-documented;
  Volence's correctness-hardening taste). Scope honestly: straight-line + single-path
  accounting first; whole-CFG cycle bounds are a research project — don't promise them.
- Also pin-worthy as STRUCTURAL: `jp cc` where `jr cc` would reach (the inverse of the
  68k width-shrink rule — here the WIDER form is load-bearing). The step-2 "better not
  same" rule needs a Z80 rider: on the hot paths, equal-cost is the optimization target,
  not fewer bytes.

### 4.4 Banking window as a typed surface
- `winptr()`/`bankid()` exist. The missing layer is **pointer provenance**: resident-RAM
  ptr vs $8000-window ptr vs bank id are all bare 16/8-bit ints today. Newtype
  candidates (`ResidentPtr`, `WinPtr`, `BankId`) follow the ratified value-flow rule —
  they're moved and compared across the seam with few construction sites, so they PAY.
- The **data-only-window rule** is lintable: any `(cpu: z80)` section placed banked
  (vma in $8000+) that contains code → error. Today the rule is comments.
- The **B2 stash-only rule** (window must hold SND_SONG_BANK during `Sequencer_Frame`;
  `dac_sample_tab.asm:16-32`) and the SetBank 9-bit latch sequence stay driver-side
  hand-written code (tenet 3) — but the co-location fatals become cross-source
  `ensure()` link asserts per the already-ruled S2-D14(d) shape.

### 4.5 Structs, layout, and the mirrored-offsets problem
`sound_constants.asm` hand-maintains: `SeqChannel` (60 B), `SfxChannel` (68 B, prefix
MIRRORS SeqChannel +0..+56 with an assert chain), `FmPatch` (32 B, power-of-two for
shift addressing), `DacSample` (9 B), the SH_* song-header offsets — all consumed by
Z80 code, 68k code, AND Python tools.
- .emp structs single-source this. Z80-specific layout rules the struct layer must
  learn: **no auto-align** (Z80 structs are byte-packed; the existing hand `sc_pad`
  fields become explicit), and an automatic **`(ix+d) ≤ +127` check on every field
  offset** of a struct used with index addressing — replacing the hand asserts at
  `sound_constants.asm:1094-1104`.
- Prefix-mirroring wants a first-class spelling (`struct SfxChannel extends SeqChannel.prefix(57)`
  or embed-as-first-field) rather than N offset-equality asserts.
- **Endianness split is a real trap**: table `dw` are LE, but song-header per-channel
  ptrs are BIG-ENDIAN by packer convention (`sound_constants.asm:1431-1433`). The
  spec has `u16le` as the 68k-side exception; the Z80 side symmetrically needs `u16be`
  for packer-shaped cells. Any struct/data story that assumes "Z80 ⇒ LE" corrupts the
  loader.
- The generated `sound_tables_z80.asm` + the five Python generators
  (`gen_sound_tables.py`, `song_packer.py`, `sfx_transcode.py`, `smps_import.py`,
  `zyrinx_player.py`) are candidates for comptime-fn absorption LATER — but that's a
  toolchain decision (build artifact vs comptime cost), not a language gap. Jot only.

### 4.6 Blob emission mechanics
- `phase 0` + vector placement: the `.emp` section model already carries `vma:`; what
  the port must re-express is the **vector-gap fill** (`rept 38h-$ / db 0` so
  `SndDrv_VBlank` lands at $0038 — AS's `ds` isn't valid under `cpu z80`) — an
  `@offset($0038)` label attribute or a `fill_to($0038)` builtin is the honest
  spelling; also the phase-relative `$` even-pad idiom.
- The code-ceiling guard (`fatal` if code overruns `SND_STATE_BASE=$18F0`) and the
  even-`Z80_SOUND_SIZE` rule map to `ensure()` directly.
- The blob↔BootData cursor contract (kill-list row 46) is already ruled: boot_data.asm
  retires with the Z80 ladder rework. A Z80 .emp port is what finally makes that
  retirement executable — the blob becomes a sigil-owned section whose size/evenness
  the linker knows, and the movem-head/cursor asserts become derived facts.
- The relaxation ladder rider: when jr→jp relaxation lands, hot-loop `jp`s must be
  PINNED (structural, like stride-locked jump tables) so the ladder never narrows them.

### 4.7 What ports with no new design
- Range-dispatch ladders, djnz loops, jump tables (`SeqOpcodeTable` = the offset-table
  family, dw cells of resident addresses — the existing `table`/data machinery covers
  it once operands exist).
- `ifdef __DEBUG__` trace blocks → the existing comptime-if shape.
- Local labels, rept, if/error/fatal/message → existing .emp equivalents.
- The idle program (38 L, one self-modifying store — it's just `ld (hl),$E9` as data
  bytes; no SMC "feature" needed).

## 5. Proposed port ladder (re-derivation of "T1/T2/T3", pending Volence's restatement)

All rungs byte-identical under the standard gates; the blob-precedes-engine rule makes
ANY byte movement a whole-corpus re-baseline, so expected byte movement at every rung
is ZERO (nonzero = STOP, not absorb — the t25 rule).

- **Rung 0 — probes, no aeon changes**: emp `dc`/`data` LE emission in a `(cpu: z80)`
  section at full-ROM scale (closes ledger row ~825); a `cpu: z80` section round-trip
  through the pins machinery; positive controls per the t24 rule.
- **Rung 1 — operand model (T1) + module-declared CPU** in sigil, proven by porting the
  SATELLITES: `z80_init.asm` (38 L, first Z80 code port), `seq_opcode_tab` /
  `dac_sample_tab` / envelope tables (data + link-imm `winptr`/`bankid`/`u16be`).
- **Rung 2 — contracts + jr→jp ladder**: push/pop preserves checking, shadow-set
  vocabulary, di/ei lattice; port `sound_psg.asm` (526 L, cleanest contract headers),
  then `sound_fm.asm` (invariant-heavy — the module-invariant design call lands here).
- **Rung 3 — the interpreters**: `sound_sequencer.asm` + `sound_sfx.asm` (struct
  single-sourcing, prefix-mirror, tempo-gate idiom; big but not cycle-critical).
- **Rung 4 — the driver top file LAST**: cycle-exact; wants the T-state accounting
  feature (or an explicit verbatim-pin policy if we ship without it), the standing-
  invariant contract, and the vector-gap/even-pad mechanics all in place first.

## 6. Open questions for Volence
1. Restate (or bless the §5 re-derivation of) the T1/T2/T3 ladder jot from 2026-07-24.
2. Is comptime T-state accounting (§4.3) wanted as a language feature, or is
   verbatim-pin + byte gates enough for the driver port? (Recommend: build the
   straight-line accounting; it's the feature that makes the driver port safe to ever
   touch again post-twin-retirement.)
3. Module-scope standing-invariant contracts (§4.2) — new contract class, or spell
   them as per-proc preserves + prose?
4. Timing of the whole campaign: after t25 the engine 68k backlog is empty; does the
   Z80 arc slot before or after the game-side ~10 files? (The sigil-side T1 operand
   work conflicts with nothing and can start any time on its own branch.)

## 7. Sources
Agent surveys 2026-07-28 over aeon main checkout (corpus, `sound_constants.asm`,
`boot_data.asm`, `main.asm`, `sound_bank.inc`) and sigil main checkout + empyrean
(`SIGIL_SPEC2_LANGUAGE.md` D2.7/D2.13/D2.18/D2.25/D2.26/S2-D13(b)/S2-D14(d),
`SIGIL_M0_CATALOG.md` §2, `sigil-isa/src/z80.rs`, `sigil-backend-z80`,
`frontend-as/{state,eval}.rs`, `frontend-emp/src/{value.rs,lower/code.rs,lower/data.rs,
z80_bus.rs}`, `sigil-link/src/{lib,relax}.rs`, gap-ledger rows 196-201 / 825-831 /
1493 / 1507-1508, `2026-07-08-sound-migration-design-handoff.md`,
`aeon/docs/superpowers/specs/2026-06-16-sound-z80-ram-map.md`, kill-list row 46).
Line references are as of aeon `6dc5a55` / sigil `184ca66`.
