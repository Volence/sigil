# 2026-07-28 — Z80 "T1" operand model for `.emp` (design note)

**OVERSEER-APPROVED 2026-07-28 (Fable):** all four decisions ratified as drafted
(flat `CodeOperand` extension · sibling `Value::Z80Reg` · module `(cpu:)` attribute
defaulting M68000 no-warn · corpus-scoped wiring, enum defined whole). Implementation
authorized on branch `z80-t1-operand-model`, TDD ladder §7 items 1-5; item 6
(the satellite ports) is rung 1 of the Z80 ladder, a separate tranche.

**IMPLEMENTED 2026-07-28/29, overseer-countersigned (branch tip `48fb85f`, 23 tests,
own strict-paired full suite 2643/0 — the frozen-68k bar proven at whole-ROM scale).**
Queued behind t26 in the merge queue. §9 naming call: `Value16Le` reused (BankPtr16Le
masks into the window; a new Abs16Le would duplicate identical range semantics).
**RUNG-2 TEST OBLIGATION (carried from item 5):** the `Value::Z80Reg`-in-a-68k-section
splice-kind error is implemented but has no source producer until Z80 proc register
params land — the rung-2 tranche that adds them MUST add the source-level test for
that direction (today only the 68k-reg-in-Z80-section direction is source-tested).

Status: **DRAFT design** (overseer session, 2026-07-28). Sibling to the recon
`2026-07-28-z80-recon-emp-design.md` — this note fills in §4.1 (the operand model)
and the rung-1 acceptance context (§5 rung 1). RULED context: the recon's §5
re-derivation IS the ladder; ruling 3 fixes module-scope `invariant` as a LATER
contract class (rung 2), so this note designs the module-CPU *interface point* only,
not the invariant vocabulary. Expected byte movement: **ZERO** — every change here
is sigil-side until rung 1's ports, which are byte-locked against asl.

The T1 branch runs in PARALLEL with the game-side tranches (ruling 4). T1 gates every
Z80 rung; it is "done" when the rung-1 satellites (`z80_init.asm`, `seq_opcode_tab.asm`,
`dac_sample_tab.asm`) assemble from `.emp` byte-identically to the asl-built resident blob.

---

## 0. The charter in one line

`.emp` already lowers Z80 *sections* (per-CPU dispatch at `lower/code.rs:77-81`,
`lower/code.rs:105-107`) and Z80 *data* (LE scalars `lower/data.rs:132-136`; `winptr`/
`bankid`/`u16le`/`Value16Le`/`BankPtr16Le` at `lower/data.rs:159-218`). What it does NOT
have is a Z80 *operand* representation: `value.rs`'s `Reg` is `d0..a7` (`value.rs:556-590`)
and every `CodeOperand` variant is a 68k EA mode (`value.rs:361-500`). So `lower_z80_instr`
accepts only symbolic `jr`/`djnz` and eleven no-operand forms and fatals everything else
with `[lower.z80-unsupported]` (`lower/code.rs:1653-1692`, the gate at `:1684-1691`).
T1 = the Z80 operand representation + its mapper + its module-CPU source, scoped to what
the rung-1 corpus demands.

---

## 1. What the rung-1 corpus actually demands (scope = corpus, not speculation)

The whole point of landing T1 against the satellites is that they enumerate the operand
forms for us. Reading the three files:

### 1.1 `engine/system/z80_init.asm` (38 L) — the only rung-1 file with *code*

Every operand form it uses, mapped to the ISA model in `sigil-isa/src/z80.rs`:

| Source line | Form | ISA `Operand` (z80.rs) | Notes |
|---|---|---|---|
| `xor a` | one-op ALU, reg A | `Reg(Reg8::A)` | already encodes (`z80.rs:564-569`); needs the emp reg8 operand |
| `ld bc, (expr)` | Pair ← Imm16 (comptime const) | `Pair`, `Imm16` | `(Z80_RAM_END-Z80_RAM)-…-1` folds to u16 |
| `ld de, L+1` / `ld hl, L` | Pair ← **symbolic** Imm16 | `Pair`, link-time value16 | label operand → Value16Le fixup (§4) |
| `ld sp, hl` | Pair ← Pair | `Pair(Sp)`,`Pair(Hl)` | `z80.rs:581` |
| `ld (hl), a` | (hl) ← reg | `IndHl`,`Reg` | `z80.rs:549` |
| `ld (hl), 0E9h` | (hl) ← Imm8 | `IndHl`,`Imm8` | `z80.rs:550` |
| `pop ix` / `pop iy` | pop index pair | `Pair(Ix/Iy)` | `z80.rs:457-459` |
| `pop de/hl/af/bc` | pop pair | `Pair` | `z80.rs:616-620` |
| `ld i, a` / `ld r, a` | RegI/RegR ← A | `RegI`/`RegR`,`Reg` | `z80.rs:599-600` |
| `ex af, af'` | shadow swap | `Pair(Af)`,`AfShadow` | `z80.rs:580` |
| `im 1` | mode-1 imm | `Imm8(1)` | `z80.rs:680-683` |
| `jp (hl)` | jump `(hl)` | `Jp`,`IndHl` → `0xE9` | `z80.rs:582` |
| `xor a`, `di`, `ldir`, `exx` | no-op forms | — | already wired (`lower/code.rs:1695-1711`) |

Plus the section mechanics — `save/cpu z80/phase 0/dephase/restore` become the section's
`(cpu: z80, vma: $0000)` attributes (already threaded as `Cpu` into data/patch lowering),
and the `Z80_IDLE_SIZE = end - start` equate is an existing `.emp` equate.

**NOT demanded by rung 1:** `(ix+d)`, `(nn)`, `(bc)`/`(de)`, condition codes, bit numbers,
`{r}` splices, typed proc register params, `clobbers`/`preserves` on Z80 regs. Those first
appear in the rung-2+ files (psg/fm/sequencer). T1 *designs* their representation so each is
a one-arm addition, but *wires and tests* only the forms a landed file demands (§7).

### 1.2 `seq_opcode_tab.asm` (68 L) and `dac_sample_tab.asm` (110 L) — data only

`seq_opcode_tab` is 32 × `dw <resident-label>` in a banked section: symbolic 16-bit LE cells
→ `Cell::Expr { width: 2, le: true }` / `Value16Le` (the machinery exists,
`lower/data.rs:216`; rung-0 probe closes the "unprobed on Z80" gap-ledger row ~825-831).
`dac_sample_tab` is `db`/`dw` of build-time `SND_*` constants + an `if/fatal` size guard —
comptime-constant LE cells, no pointers, no operands. Both ride the existing `.emp` `data`
path; they demand NOTHING new from the operand model. They are in rung 1 only to prove the
banked `(cpu: z80, vma: $8000)` section + LE-dw + link-value story end-to-end alongside the
first code port.

---

## 2. Decision: one flat `CodeOperand` enum extended with Z80 variants (NOT a trait split, NOT a tag field)

The recon frames the question (§4.1): *one CPU-tagged `CodeOperand` enum vs a per-CPU enum
behind a trait.* Three shapes were on the table.

**Option A — flat extension.** Add `Z80*` variants to the existing `CodeOperand`
(`value.rs:361-500`). `CodeBuf`/`CodeItem`/`Value::Code` stay CPU-agnostic containers.

**Option B — enum-of-enums.** `enum CodeOperand { M68k(M68kOp), Z80(Z80Op), Shared(…) }`.
Cleaner exhaustiveness per CPU.

**Option C — trait / generic.** `CodeBuf<C: Cpu>` with an associated operand type.

### Argument

Option C ripples a generic parameter through the entire comptime `Code` machinery —
`CodeBuf`, `CodeItem`, `Value::Code`, the `++` monoid (`value.rs:628-643`), the `asm{}`
instantiation path, `Display`, every eval site that carries a `Value`. It also breaks the
homogeneity assumption in the wrong place: a section is a single CPU, so the *container*
never needs to mix CPUs — only the *lowering* does, and lowering already knows its CPU from
the section (`lower/code.rs:77-81`). Rejected.

Option B forces every existing 68k operand to be rewritten `CodeOperand::Reg` →
`CodeOperand::M68k(M68kOp::Reg)` across the byte-frozen 68k lowering — exactly the churn on
proven paths the campaign avoids mid-flight. The only thing it buys (per-CPU exhaustiveness)
is worth little: a Z80 operand can only ever be produced by the Z80 mapper, which only runs
in a Z80 section, so a Z80 variant reaching `lower_m68k_instr` is structurally impossible.
Rejected.

**Chosen: Option A.** It mirrors what `CodeOperand` already is — a flat union of *all* 68k
EA modes; Z80 forms are just more variants. It touches ZERO existing 68k *production* arms;
it adds Z80 variants that the 68k *consumption* matches must acknowledge, but those matches
already carry catch-all `_ =>`/`other =>` arms (e.g. the splice-kind fallthrough
`eval/asm.rs:1987-1998`), so the touch is a handful of `unreachable!()`/diagnostic arms where
a match is exhaustive, nothing more. This is also precisely the incremental discipline
`z80.rs` itself uses ("insert new index arms above this line", `z80.rs:484`).

The mirror is deliberate: the emp-side operand enum is the 1:1 image of `z80.rs`'s `Operand`
(`z80.rs:64-97`), one comptime-side abstraction layer up (no ISA import in `value.rs` — same
rule that keeps the emp `Reg`/`Cc`/`Width` ISA-free, `value.rs:502-590`).

### 2.1 New `CodeOperand` variants (value.rs)

```rust
// value.rs — appended to the existing flat CodeOperand enum (Z80 image of z80.rs Operand).
// Immediates reuse the CPU-neutral CodeOperand::Imm(i128); the Z80 lowering picks the
// imm8 vs imm16 encoding by instruction form, exactly as z80.rs::encode does. Symbolic
// 16-bit immediates reuse CodeOperand::Sym (link-time; §4).
    Z80Reg8(Z80Reg8),                        // a b c d e h l            (z80.rs Reg8)
    Z80Pair(Z80Pair),                        // bc de hl sp af ix iy     (z80.rs Reg16)
    Z80IndHl,                                // (hl)     — also the jp (hl) target
    Z80IndBc,                                // (bc)     — rung 3
    Z80IndDe,                                // (de)     — rung 3
    Z80Indexed { reg: Z80Index, disp: i128 },// (ix+d)/(iy+d), i8-fold-checked (§3) — rung 2
    Z80Mem { addr: i128 },                   // (nn) comptime — rung 2/3
    Z80AfShadow,                             // af'
    Z80RegI,                                 // i
    Z80RegR,                                 // r
    Z80Cc(Z80Cond),                          // condition code           — rung 2 (§6)
    Z80Bit(u8),                              // 0..=7                    — rung 2 (§6)
```

`Z80Reg8`/`Z80Pair`/`Z80Index`/`Z80Cond` are emp-side enums in `value.rs`, images of
`z80.rs`'s `Reg8`/`Reg16`/`IndexReg`/`Cond`, with `from_name`/`Display` pairs mirroring
`Reg::from_name` (`value.rs:605-625`) and `Reg`'s `Display` (`value.rs:680-701`). Wiring
per §7 lands the variants incrementally; the enum is defined whole so later rungs are pure
additions.

### 2.2 The splice value (value.rs — parallel to `Value::Reg`)

`{r}` on Z80 needs a comptime register value, the analog of `Value::Reg(Reg)` (`value.rs:114`).
Do **not** widen `Value::Reg` to span both CPUs — 68k `Reg` is `d0..a7`, a disjoint universe
from Z80's `a/hl/ix/…`; widening churns the frozen 68k splice path. Add a sibling:

```rust
    // value.rs Value — a new variant beside Reg(Reg)/Cc(Cc)/Width(Width).
    /// A comptime Z80 register class — the value a `{r}` splice resolves to in a
    /// Z80 section (§4.1). Reg8/Pair/Index share one value; cc/bit are separate
    /// splice kinds (added with the rung that demands them).
    Z80Reg(Z80RegClass),   // enum Z80RegClass { R8(Z80Reg8), Pair(Z80Pair), Index(Z80Index) }
```

This forces one arm each into `Value::type_name` (`value.rs:711-735`) and `Display`
(`value.rs:752-834`) — the exact mechanical cost paid when `Reg`/`Cc`/`Width` were added,
no more. The section CPU + the value kind must AGREE: a `Value::Z80Reg` classified in a 68k
section, or a `Value::Reg` in a Z80 section, is a loud `[asm.splice-kind]` error — a free
correctness win the single-CPU-per-section fact hands us (§7 positive control).

---

## 3. Operand grammar — the parser needs no new shapes

The `.emp` operand grammar is already CPU-neutral at the AST (`ast.rs:1506-1557`):
`Operand::{Imm, PreDec, PostInc, Ind, DispInd, Plain, Splice}`. Every Z80 spelling parses
into these existing nodes today; T1 only teaches the *mapper* (eval/asm.rs) to read them
under a Z80 section. The AS front-end already proves this mapping is sufficient — its
`OperandAtom` (`frontend-as/src/operands.rs:12-44`) classifies the full Z80 operand surface
(`IndReg`, `Indexed`, `Mem`, `RegOrCond`, `AfShadow`) from the same token shapes.

Concrete node → Z80 `CodeOperand`:

- `a`, `hl`, `ix`, `af'`, `i`, `r`, `nz` → `Operand::Plain` with a single-segment path. The
  Z80 branch of `map_plain` (`eval/asm.rs:1803-1836`) resolves the segment through a
  CPU-aware reg map (§3.1) → `Z80Reg8`/`Z80Pair`/`Z80AfShadow`/`Z80RegI`/`Z80RegR`, or a
  condition code in control-flow first-operand position (mirrors the AS
  `control_flow && i==0` rule, `eval.rs:3601-3611`).
- `(hl)`/`(bc)`/`(de)` → `Operand::Ind { parts: [reg], .. }` → `Z80IndHl`/`Z80IndBc`/`Z80IndDe`.
- `(ix+d)` → `Operand::DispInd { disp, inner: Ind[ix] }` (or `Ind` with two parts) →
  `Z80Indexed`. The displacement folds to i128 then range-checks to **i8** — reusing the
  SAME check the 68k brief-extension disp8 path already runs (`eval/asm.rs:1540-1550`,
  `i8::MIN..=i8::MAX`), not a copy of the AS front-end's `fold_imm(-128,127)`
  (`frontend-as/eval.rs:3623-3627`). One shared invariant ("(ix+d) disp is i8"), enforced by
  the evaluator both front-ends already own; `z80.rs`'s `Operand::Indexed { disp: i8 }` is
  the type-level backstop.
- `(nn)` → `Operand::Ind` with a non-register expr part → `Z80Mem { addr }` (comptime) or a
  symbolic `Sym` (link-time).
- `#n` / bare `5` / `0E9h` immediates → `Operand::Imm` / `Plain` comptime int → the neutral
  `CodeOperand::Imm(i128)`; the Z80 lowering (`lower_z80_instr`) folds to imm8 or imm16 by
  form and range, as `z80.rs::encode` splits `Imm8`/`Imm16` today.

### 3.1 `{r}` splices and typed proc register params

- **`{r}` splice.** `classify_operand_splice` (`eval/asm.rs:1978-2000`) gains one arm:
  `Value::Z80Reg(rc) => Some(<Z80 CodeOperand for rc>)`, beside the existing
  `Value::Reg(r) => CodeOperand::Reg(r)`. So a Z80 comptime template reads exactly like
  `engine/coords.emp`'s `pixels_to_coord(v: Reg) -> Code { asm { swap {v} … } }` — a Z80
  `fn(ch: SeqReg) -> Code { asm { ld a, {ch} … } }`. This is the tenet-3 payoff: hand-written
  Z80 lines, register hole filled by comptime.
- **Typed proc register params.** 68k procs bind a register name to a pointer type in the
  header — `pub proc CreateChild_Normal (a0: *Sst, a1: *u8)` (`engine/objects/children.emp:120`).
  The Z80 analog `proc Seq_Op_Vol (hl: *SeqChannel, a: u8)` binds `hl`/`a` through the
  CPU-aware reg map so the param evaluates to a `Value::Z80Reg` usable in `{hl}` splices.
  Touch point: the proc-param binder (`reg_from_name` / `Reg::from_name`, `eval/asm.rs:2026`,
  `value.rs:605`) becomes CPU-parameterized — under a Z80 module it consults the Z80 map.
  **NOT rung-1-demanded** (`z80_init` is a param-less leaf): defined here, wired with the
  first rung-2 proc that takes a register param.

---

## 4. Symbolic 16-bit immediates (a real rung-1 requirement)

`z80_init`'s `ld hl, Z80_IdleProgram_CodeEnd` and `ld de, …CodeEnd+1` are label-bearing
16-bit immediates in a phase-0 section — they cannot fold at eval; they defer to link exactly
as symbolic `jr`/`djnz` already do (`lower/code.rs:1666-1673` → `Z80JrRel8`). T1 adds the
16-bit sibling: a symbolic operand in a Z80 `ld rr,nn` lowers to a 2-byte hole + one
**`Value16Le`/`Abs16Le`** fixup — the same LE value-fixup family the data path selects
(`lower/data.rs:211-218`), now reachable from code. This is the one genuinely new *fixup*
wiring in T1 (everything else in §1.1 folds to bytes at eval); it is corpus-demanded by three
`z80_init` lines, so it is in scope, not speculative.

---

## 5. Module-declared CPU (gap-ledger rows 196-201)

The hazard (ledger `campaign-gap-ledger.md:196-201`): `initial_cpu: Cpu::M68000` is a caller
convention hardcoded at four call sites; a braceless Z80 module carries no CPU signal and
silently depends on every caller passing M68000. The day `z80_init.emp` exists, a forgetful
caller mis-lowers it with no module-level fact to catch it.

**Proposal (minimal): the module declares its CPU as an attribute; the pipeline reads it
instead of taking it as a caller argument.**

```
module z80_init in z80_resident (cpu: z80)
```

- **Surface:** an optional `(cpu: z80)` attribute on the `module … in <section>` header. This
  is the row 196-201 candidate spelled directly, and it is the ONE forward-compatible slot
  ruling 3's module-scope `invariant(de)` etc. will later attach to — T1 opens the attribute
  list and adds only the `cpu:` key; the invariant vocabulary is rung 2 (interface point
  named, not designed).
- **Default:** omitted ⇒ `M68000`. Every existing 68k module writes nothing and stays
  byte-frozen; Z80 is opt-in-explicit. This flips the safety: `initial_cpu` stops being a
  caller argument and becomes a module fact, so a Z80 module CANNOT silently ride a caller's
  default — it MUST carry `(cpu: z80)`.
- **No default warn.** A warn on the omitted case would fire on every 68k module (noise). The
  default is the safe CPU; the unsafe direction (Z80 mnemonics under a defaulted M68000) is
  already loud — it fails mnemonic recognition (`lower/code.rs:105-107`).
- **Section vs module coherence.** The section a module opens still carries the authoritative
  `(cpu:)` for its instruction lowering (a section is one CPU; `lower/data.rs` already threads
  it). The module attribute seeds `initial_cpu` and asserts agreement: a `(cpu: z80)` module
  that opens a `(cpu: m68000)` section is a `[module.cpu-mismatch]` error. For the corpus,
  `z80_init.emp` is single-CPU — module and section agree trivially.

Scope guard: define `Cpu` from the module attribute; keep everything else (per-section CPU,
the data/patch lowering signature) as-is.

---

## 6. What T1 explicitly does NOT include (interface points named)

Each is a later rung; each has a T1-side interface point so nothing here blocks it.

- **Contract vocabulary** (`clobbers`/`preserves`/`out`/`shadow`/module `invariant`) — rung 2.
  Interface: (a) the `(cpu: z80)` module attribute list is where `invariant(de)` attaches; (b)
  the CPU-aware reg map (§3.1) is what a Z80 `clobbers(hl/de)` reglist parser will consult;
  (c) `preserves` as push/pop pairing (not movem) is a new checker — T1 only guarantees the
  `CodeItem::Instr` stream carries resolved mnemonics+ops (it already does, `value.rs:333-348`),
  the input any such pass reads.
- **T-state accounting** (rung 4). Interface: `z80.rs::encode` already knows every form's byte
  cost; a future `cycles()` pass folds over the same `CodeItem::Instr` op stream T1 produces.
  No T1 shape change — keep `Instr.ops` resolved, done.
- **`jr→jp` relaxation ladder** (D2.18 / S2-D13(b)). Interface: symbolic `jr`/`djnz` stay a
  link-time `Z80JrRel8` fixup (`lower/code.rs:1666-1673`) — T1 does NOT hard-encode a fixed
  jr8, so when the CPU-agnostic `RelaxLadder` grows a Z80 rung the branch is already a relax
  candidate. T1 must NOT pin hot-loop `jp`s either (that is rung 4's structural-pin call).
- **`u16be`** (song-header packer big-endian ptrs). Interface: purely data-side —
  `Cell::Scalar`/`Cell::Expr` already carry an `le: bool` override (`value.rs:184`, `:241`);
  `u16be` is the symmetric `be` override, added where the packer files land (rung 3). Not an
  operand-model concern; T1 does not touch it.
- **`(bc)`/`(de)`, `(ix+d)`, `(nn)`, condition codes, bit numbers** — representation defined
  (§2.1) but wired with the rung-2/3 file that first spells them.

---

## 7. TDD ladder (every capability lands with a demanded-by-a-real-file test)

Mirrors the house style and the t24 positive-control rule (a negative check must be able to
fail). The ISA layer is already asl-golden-oracled (`z80.rs` + `z80_golden_vectors.txt`, 120
vectors) — T1's tests are FRONTEND tests: `.emp` source → bytes, checked against the asl
golden for the same source.

1. **Probe (no aeon change).** One-form round-trip: an `asm { ld a, 5 }` in a
   `(cpu: z80, vma: 0)` section emits `[0x3E,0x05]` (== `z80.rs` golden). Proves the mapper +
   `lower_z80_instr` operand path end-to-end on the smallest form.
2. **z80_init operand forms, one test per row of §1.1.** `ld sp,hl`→`F9`, `ld (hl),a`→`77`,
   `pop ix`→`DD E1`, `ld i,a`→`ED 47`, `ex af,af'`→`08`, `im 1`→`ED 56`, `jp (hl)`→`E9`,
   `ld (hl),0E9h`→`36 E9`, `xor a`→`AF`. Bytes from the existing golden; these are pins on the
   emp mapper, not new ISA facts.
3. **Symbolic imm16 (§4).** `ld hl, L` with `L` a phase-0 label emits a 2-byte hole + one
   `Value16Le` fixup; link resolves it to `L`'s VMA. Test at both the fragment level (fixup
   kind/width) and the linked-bytes level.
4. **Module-CPU (§5).** (a) `module m in s (cpu: z80)` lowers Z80 mnemonics; (b) a `(cpu: z80)`
   module opening a `(cpu: m68000)` section → `[module.cpu-mismatch]`; (c) an omitted attribute
   defaults M68000 and a Z80 mnemonic under it stays unrecognized (the existing loud path).
5. **Positive/negative controls (t24 rule — the probe must be able to fail).**
   - `Value::Reg(d0)` spliced in a Z80 section → `[asm.splice-kind]`; `Value::Z80Reg(hl)` in a
     68k section → same. (Confirms the CPU/kind-agreement check actually fires.)
   - `(ix+128)` → the i8-range error (`eval/asm.rs:1541` path), `(ix+127)` accepted.
   - A form the corpus does NOT demand (e.g. `bit 0,(ix+d)`) still reports
     `[lower.z80-unsupported]` — proves T1's wired scope is BOUNDED, not silently accepting
     un-oracled bytes.
6. **Acceptance (the port gate).** `z80_init.emp` + `seq_opcode_tab.emp` + `dac_sample_tab.emp`
   assemble byte-identical to the asl-built resident blob slice, under the standard strict
   run. Nonzero delta = STOP (blob-precedes-engine: any Z80 byte re-baselines the whole
   corpus), never absorb.

---

## 8. Byte-movement statement

**ZERO.** T1 is sigil-side: new enum variants, new mapper arms, new module attribute, new
frontend tests. No aeon source changes until rung 1's ports, and those are byte-locked against
asl by the acceptance gate (step 7.6). Any nonzero movement at rung 1 is the STOP condition,
not an absorb.

---

## 9. Open questions

None requiring the overseer — the four recon questions are already RULED and the decisions
above (flat `CodeOperand` extension; sibling `Value::Z80Reg`; module `(cpu:)` attribute
defaulting M68000, no warn; corpus-scoped wiring with the enum defined whole) are inside the
delegated "trust your takes" envelope. Flagged for the *implementer*, not for a ruling:

- The exact fixup-kind name for a symbolic Z80 `ld rr,nn` (§4) — `Value16Le` vs a dedicated
  `Abs16Le` — is a naming call to settle against the linker's existing Z80 fixup set at
  implementation time; both encode identically (2-byte LE hole). Pick whichever the linker
  already dispatches, add the other only if the range-check semantics differ.
