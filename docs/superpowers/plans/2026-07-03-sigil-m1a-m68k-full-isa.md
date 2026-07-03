# Sigil M1.A — 68000 Encoder → Full ISA Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Grow `sigil-isa::m68k` from a MOVE-only encoder to the full 68000 instruction/EA set the Aeon source (pinned @ aeon `c7aaca6`) uses, proven byte-identical to `asl` via committed golden vectors + dedicated §5.5 hazard vectors, and add the thin `sigil-backend-m68k` adapter.

**Architecture:** Per-family procedural encoder functions dispatched from `m68k::encode`, all reusing the existing shared `encode_ea`/`brief_ext`. The committed `asl` golden-vector file is the oracle (ground truth); the encoder is iterated until its bytes match. A thin `M68kBackend` mirrors the existing `Z80Backend` (`lower` → `DataFragment` for fully-resolved forms). Symbolic-target machinery (branch PcRel fixups, jmp/jsr width selection) is explicitly deferred to sub-project B.

**Tech Stack:** Rust (edition 2021), the `asl 1.42` / `p2bin` native tools in `aeon/tools/` (oracle only; never in CI), existing test harness pattern (`corpus_m68k`, `m68k_golden_vectors.txt`, `gen-m68k-vectors`).

---

## Design doc

`docs/superpowers/specs/2026-07-03-sigil-m1a-m68k-full-isa-design.md`. Read it first. Corpus facts that bound this plan: 46 mnemonic families, all 12 EA modes but **brief-extension indexed form only** (no 68020), **branches 100% size-pinned**, width-selection collapses to bare-symbol `jmp`/`jsr` only (→ B), and the flagged blind spots `movep`/`tas`/`addx`/`cmpm`/Scc/`.ATTRIBUTE`-synthesized `move/cmp/tst`.

## The asl-oracle TDD loop (how every encoder family task works)

The golden file `crates/sigil-isa/tests/m68k_golden_vectors.txt` holds `<snippet> => <asl bytes>` and is the **spec**. The shared corpus `crates/sigil-isa/tests/corpus_m68k/mod.rs` maps each snippet to an `Instruction`. Per family:

1. **RED:** extend `corpus_m68k()` with the family's `(snippet, Instruction)` pairs (needs the Task-1 vocab), regenerate the golden from `asl`, and observe the corpus-coverage / `all_forms_match_golden` tests fail (encoder returns `UnsupportedForm`).
2. **GREEN:** implement `encode_<family>` per the encoding table in the task; iterate until `encode(inst) == golden bytes`.
3. **Commit.**

**Regenerate golden** (requires the aeon tools; run once per family after adding corpus entries):
```bash
cargo run -p sigil-isa --bin gen-m68k-vectors
```
It assembles each corpus snippet through `asl` at `cpu 68000 / org 0`, extracts bytes via `p2bin`, and rewrites the golden file in `corpus_m68k()` order. **Commit the regenerated golden with the corpus change.** CI never runs `asl`; it reads the committed file.

**Encoding tables are implementation guidance; the golden vector is the authority.** Where a table bit is uncertain, the `asl` bytes decide — iterate `encode` until `all_forms_match_golden` is green. All values big-endian; `encode_ea` already yields `(mode3, reg3, ext_words)`.

---

## Task 1: Vocabulary expansion (`Mnemonic`, `Cond`, `Size::S`, new `Operand`s)

**Files:**
- Modify: `crates/sigil-isa/src/m68k.rs` (the `Mnemonic`, `Size`, `Operand` enums + `encode` dispatch)
- Test: `crates/sigil-isa/src/m68k.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing test** — add to the existing `#[cfg(test)] mod tests` in `m68k.rs` (create the module if absent):

```rust
#[cfg(test)]
mod vocab_tests {
    use super::*;

    #[test]
    fn new_vocab_constructs_and_move_still_dispatches() {
        // New mnemonics/operands exist and compile.
        let _ = (Mnemonic::Add, Mnemonic::Bcc(Cond::Eq), Mnemonic::Movem, Mnemonic::Moveq);
        let _ = (Operand::RegList(0x0001), Operand::Disp(4), Operand::Ccr, Operand::Sr);
        let _ = Size::S;
        // Non-Move mnemonics are dispatched but not yet implemented → UnsupportedForm.
        let add = Instruction { mnemonic: Mnemonic::Add, size: Size::W, ops: vec![Operand::Dn(1), Operand::Dn(0)] };
        assert!(matches!(encode(&add), Err(IsaError::UnsupportedForm(_))));
        // Move still works.
        let mv = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Operand::Dn(1), Operand::Dn(0)] };
        assert_eq!(encode(&mv).unwrap(), vec![0x30, 0x01]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sigil-isa --lib vocab_tests`
Expected: FAIL to compile (`Mnemonic::Add` etc. undefined).

- [ ] **Step 3: Expand the vocab.** In `m68k.rs`:

Replace `pub enum Mnemonic { Move }` with the full set:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mnemonic {
    Move, Movea,
    Add, Adda, Sub, Suba, And, Or, Eor, Cmp, Cmpa, Muls,
    Addi, Subi, Andi, Ori, Eori, Cmpi,
    Moveq, Addq, Subq,
    Asl, Asr, Lsl, Lsr, Rol, Ror,
    Btst, Bset, Bclr,
    Clr, Neg, Not, Tst, Tas,
    Scc(Cond),
    Jmp, Jsr, Lea, Pea, Nop, Rts, Rte, Trap, Swap, Ext,
    Bra, Bsr, Bcc(Cond), Dbcc(Cond),
    Movem, Movep, Addx, Cmpm,
    MoveToSr, MoveFromSr, // move.w <ea>,sr / move.w sr,<ea>
    AndiCcr, OriCcr,      // andi.b #imm,ccr / ori.b #imm,ccr
}
```

Add the condition enum (condition-code = bits 11–8, value = discriminant):
```rust
/// 68000 condition codes; discriminant is the 4-bit cc field (bits 11–8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cond {
    T = 0x0, F = 0x1, Hi = 0x2, Ls = 0x3, Cc = 0x4, Cs = 0x5, Ne = 0x6, Eq = 0x7,
    Vc = 0x8, Vs = 0x9, Pl = 0xA, Mi = 0xB, Ge = 0xC, Lt = 0xD, Gt = 0xE, Le = 0xF,
}
impl Cond {
    #[inline]
    pub fn cc(self) -> u16 { self as u16 }
}
```

Add `S` to `Size` (short/8-bit branch displacement; never used by non-branch forms):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size { B, W, L, S }
```

Add the new operand variants to `Operand`:
```rust
    // ...existing EA variants...
    /// MOVEM register-list mask in canonical order bit0=D0..bit7=D7,bit8=A0..bit15=A7.
    /// The predecrement (-(An)) bit-order reversal is applied inside encode_movem.
    RegList(u16),
    /// Resolved branch / DBcc displacement (bytes measured as asl emits them).
    Disp(i32),
    /// The condition-code register (andi/ori to ccr).
    Ccr,
    /// The status register (move to/from sr).
    Sr,
```

Change the dispatch so every non-Move mnemonic returns `UnsupportedForm` for now:
```rust
pub fn encode(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    match inst.mnemonic {
        Mnemonic::Move => encode_move(inst),
        other => Err(IsaError::UnsupportedForm(format!("{other:?}"))),
    }
}
```
(`encode_ea`'s `match *op` will need a `_ => ...` arm or explicit arms for the new operand variants; return `IsaError::UnsupportedForm` from `encode_ea` for `RegList`/`Disp`/`Ccr`/`Sr` since those are handled by their family encoders, not as general EAs.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sigil-isa --lib vocab_tests` then `cargo test -p sigil-isa`
Expected: PASS; existing MOVE golden tests still green.

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-isa/src/m68k.rs
git commit -m "feat(sigil-isa): expand m68k vocab (full Mnemonic set, Cond, Size::S, RegList/Disp/Ccr/Sr operands)"
```

---

## Task 2: ALU-EA family (`add adda sub suba and or eor cmp cmpa muls`)

**Files:**
- Modify: `crates/sigil-isa/src/m68k.rs` (add `encode_alu_ea`, dispatch arms)
- Modify: `crates/sigil-isa/tests/corpus_m68k/mod.rs` (family corpus entries)
- Regenerate: `crates/sigil-isa/tests/m68k_golden_vectors.txt`
- Test: `crates/sigil-isa/tests/encode_m68k.rs` (per-family test)

**Encoding table** (base opcode word, `rrr`=Dn in bits 11–9, `ooo`=opmode in bits 8–6, EA in bits 5–0):

| mnemonic | base bits 15–12 | opmode `ooo` (`<ea>,Dn` / `Dn,<ea>`) |
|---|---|---|
| `add`  | `1101` | .b/.w/.l `<ea>,Dn`=000/001/010, `Dn,<ea>`=100/101/110 |
| `sub`  | `1001` | same as add |
| `and`  | `1100` | same (Dn direction) |
| `or`   | `1000` | same |
| `cmp`  | `1011` | `<ea>,Dn` only = 000/001/010 |
| `eor`  | `1011` | `Dn,<ea>` only = 100/101/110 |
| `cmpa` | `1011` | opmode .w=011, .l=111 (`<ea>,An`) |
| `adda` | `1101` | opmode .w=011, .l=111 |
| `suba` | `1001` | opmode .w=011, .l=111 |
| `muls` | `1100` | opmode `111` (word, `<ea>,Dn`) |

- [ ] **Step 1: RED — add corpus entries.** In `corpus_m68k()` add representative forms exercising both directions and a spread of EA modes (snippets are verbatim `asl` input):

```rust
        // --- ALU-EA family ---
        ("add.w d1,d0", Instruction { mnemonic: Mnemonic::Add, size: W, ops: vec![Dn(1), Dn(0)] }),
        ("add.w (a1),d0", Instruction { mnemonic: Mnemonic::Add, size: W, ops: vec![Ind(1), Dn(0)] }),
        ("add.l d0,(a1)", Instruction { mnemonic: Mnemonic::Add, size: L, ops: vec![Dn(0), Ind(1)] }),
        ("sub.w d1,d0", Instruction { mnemonic: Mnemonic::Sub, size: W, ops: vec![Dn(1), Dn(0)] }),
        ("and.w d1,d0", Instruction { mnemonic: Mnemonic::And, size: W, ops: vec![Dn(1), Dn(0)] }),
        ("or.b d1,d0", Instruction { mnemonic: Mnemonic::Or, size: B, ops: vec![Dn(1), Dn(0)] }),
        ("eor.w d0,d1", Instruction { mnemonic: Mnemonic::Eor, size: W, ops: vec![Dn(0), Dn(1)] }),
        ("cmp.w (a1),d0", Instruction { mnemonic: Mnemonic::Cmp, size: W, ops: vec![Ind(1), Dn(0)] }),
        ("cmpa.l a1,a0", Instruction { mnemonic: Mnemonic::Cmpa, size: L, ops: vec![An(1), An(0)] }),
        ("adda.w d0,a1", Instruction { mnemonic: Mnemonic::Adda, size: W, ops: vec![Dn(0), An(1)] }),
        ("suba.l a2,a3", Instruction { mnemonic: Mnemonic::Suba, size: L, ops: vec![An(2), An(3)] }),
        ("muls.w d1,d0", Instruction { mnemonic: Mnemonic::Muls, size: W, ops: vec![Dn(1), Dn(0)] }),
```
Note: for `adda`/`suba`/`cmpa` the register field is the **An** (destination address register); write `encode_alu_ea` to read the An operand's number for bits 11–9 and put the source `<ea>` in bits 5–0.

Regenerate the golden:
```bash
cargo run -p sigil-isa --bin gen-m68k-vectors
```

Add the per-family test to `encode_m68k.rs`:
```rust
#[test]
fn alu_ea_family() {
    check(&[
        "add.w d1,d0", "add.w (a1),d0", "add.l d0,(a1)", "sub.w d1,d0",
        "and.w d1,d0", "or.b d1,d0", "eor.w d0,d1", "cmp.w (a1),d0",
        "cmpa.l a1,a0", "adda.w d0,a1", "suba.l a2,a3", "muls.w d1,d0",
    ]);
}
```

- [ ] **Step 2: Run to verify RED**

Run: `cargo test -p sigil-isa --test encode_m68k alu_ea_family`
Expected: FAIL (`encode ... UnsupportedForm`).

- [ ] **Step 3: GREEN — implement `encode_alu_ea`** using the table above (dispatch `Add|Sub|And|Or|Cmp|Eor|Cmpa|Adda|Suba|Muls` to it from `encode`). Determine direction from which operand is the register vs the EA; assemble `base<<12 | reg<<9 | opmode<<6 | (mode<<3|reg)`; append `<ea>` extension words. Iterate until bytes match golden.

- [ ] **Step 4: Run to verify GREEN**

Run: `cargo test -p sigil-isa`
Expected: PASS (`alu_ea_family`, `all_forms_match_golden`, `golden_covers_the_full_corpus` all green).

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-isa/src/m68k.rs crates/sigil-isa/tests/corpus_m68k/mod.rs crates/sigil-isa/tests/m68k_golden_vectors.txt crates/sigil-isa/tests/encode_m68k.rs
git commit -m "feat(sigil-isa): m68k ALU-EA family (add/sub/and/or/eor/cmp/cmpa/adda/suba/muls) byte-exact vs asl"
```

---

## Task 3: ALU-immediate family (`addi subi andi ori eori cmpi` + `andi/ori` to CCR + MOVE to/from SR)

**Files:** same set as Task 2.

**Encoding table:** `0000 oooo ss eeeeee` + immediate ext word(s) **before** the EA extension. `oooo`: `ori`=0000, `andi`=0010, `subi`=0100, `addi`=0110, `eori`=1010, `cmpi`=1100. `ss`: .b=00 (imm in low byte of one word), .w=01, .l=10 (two imm words). Special fixed opcodes: `andi.b #imm,ccr`=`0000 0010 0011 1100` (`023C`) + imm word; `ori.b #imm,ccr`=`003C` + imm word; `move.w <ea>,sr`=`0100 0110 11 eeeeee` (`46xx`); `move.w sr,<ea>`=`0100 0000 11 eeeeee` (`40xx`).

- [ ] **Step 1: RED — corpus entries** (in `corpus_m68k()`):

```rust
        // --- ALU-immediate family ---
        ("addi.w #$10,d0", Instruction { mnemonic: Mnemonic::Addi, size: W, ops: vec![Imm(0x10), Dn(0)] }),
        ("subi.l #$1000,d1", Instruction { mnemonic: Mnemonic::Subi, size: L, ops: vec![Imm(0x1000), Dn(1)] }),
        ("andi.w #$00FF,d0", Instruction { mnemonic: Mnemonic::Andi, size: W, ops: vec![Imm(0x00FF), Dn(0)] }),
        ("ori.b #$01,d0", Instruction { mnemonic: Mnemonic::Ori, size: B, ops: vec![Imm(0x01), Dn(0)] }),
        ("eori.w #$FFFF,d0", Instruction { mnemonic: Mnemonic::Eori, size: W, ops: vec![Imm(0xFFFF), Dn(0)] }),
        ("cmpi.w #$0010,(a1)", Instruction { mnemonic: Mnemonic::Cmpi, size: W, ops: vec![Imm(0x10), Ind(1)] }),
        ("andi.b #$FE,ccr", Instruction { mnemonic: Mnemonic::AndiCcr, size: B, ops: vec![Imm(0xFE), Ccr] }),
        ("ori.b #$01,ccr", Instruction { mnemonic: Mnemonic::OriCcr, size: B, ops: vec![Imm(0x01), Ccr] }),
        ("move.w #$2700,sr", Instruction { mnemonic: Mnemonic::MoveToSr, size: W, ops: vec![Imm(0x2700), Sr] }),
        ("move.w sr,-(sp)", Instruction { mnemonic: Mnemonic::MoveFromSr, size: W, ops: vec![Sr, PreDec(7)] }),
```

Regenerate golden; add `alu_immediate_family` test in `encode_m68k.rs` listing all ten snippets.

- [ ] **Step 2: Run to verify RED** — `cargo test -p sigil-isa --test encode_m68k alu_immediate_family` → FAIL.
- [ ] **Step 3: GREEN — implement `encode_alu_imm`** (the six `#imm,<ea>` forms), plus `encode_ccr_imm` (`AndiCcr`/`OriCcr`) and `encode_move_sr` (`MoveToSr`/`MoveFromSr`). Immediate word ordering: imm ext first, then destination-EA ext. Iterate to match golden.
- [ ] **Step 4: Run to verify GREEN** — `cargo test -p sigil-isa` → PASS.
- [ ] **Step 5: Commit**

```bash
git add crates/sigil-isa/src/m68k.rs crates/sigil-isa/tests/corpus_m68k/mod.rs crates/sigil-isa/tests/m68k_golden_vectors.txt crates/sigil-isa/tests/encode_m68k.rs
git commit -m "feat(sigil-isa): m68k ALU-immediate family + andi/ori-to-ccr + move-to/from-sr (§5.5 CCR/SR hazard vectors)"
```

---

## Task 4: Quick family (`moveq addq subq`)

**Files:** same set.

**Encoding table:** `moveq #d,Dn` = `0111 rrr 0 dddddddd` (`rrr`=Dn, `d`=8-bit signed data). `addq #d,<ea>` = `0101 ddd 0 ss eeeeee`; `subq` = `0101 ddd 1 ss eeeeee` (`ddd`=data 1–8, with 8 encoded as `000`; `ss` size).

- [ ] **Step 1: RED — corpus entries**:

```rust
        // --- quick family ---
        ("moveq #1,d0", Instruction { mnemonic: Mnemonic::Moveq, size: L, ops: vec![Imm(1), Dn(0)] }),
        ("moveq #-1,d3", Instruction { mnemonic: Mnemonic::Moveq, size: L, ops: vec![Imm(-1), Dn(3)] }),
        ("addq.w #1,d0", Instruction { mnemonic: Mnemonic::Addq, size: W, ops: vec![Imm(1), Dn(0)] }),
        ("addq.l #8,a1", Instruction { mnemonic: Mnemonic::Addq, size: L, ops: vec![Imm(8), An(1)] }),
        ("subq.w #2,d1", Instruction { mnemonic: Mnemonic::Subq, size: W, ops: vec![Imm(2), Dn(1)] }),
```

Regenerate golden; add `quick_family` test.

- [ ] **Step 2: RED verify** → FAIL.
- [ ] **Step 3: GREEN — `encode_quick`:** for `moveq`, low byte = data as `i8`. For `addq`/`subq`, encode data 1–8 (8→`000`) into bits 11–9, size into bits 7–6, EA into 5–0. Iterate to golden.
- [ ] **Step 4: GREEN verify** — `cargo test -p sigil-isa` → PASS.
- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(sigil-isa): m68k quick family (moveq/addq/subq) byte-exact vs asl"
```

---

## Task 5: Shift/rotate family (`asl asr lsl lsr rol ror`)

**Files:** same set.

**Encoding table:** register form `1110 ccc d ss i tt rrr`: `ccc`=count (immediate 1–8, 8→`000`) or source Dn; `d`=direction (0=right,1=left); `ss`=size; `i`=0 (immediate count) / 1 (register count); `tt`=type: AS=00, LS=01, RO=11; `rrr`=destination Dn. Memory form (shift memory by 1, word only) `1110 0 tt d 11 eeeeee`.

- [ ] **Step 1: RED — corpus entries** (register form with immediate count and with register count; one memory form):

```rust
        // --- shift/rotate family ---
        ("asl.w #1,d0", Instruction { mnemonic: Mnemonic::Asl, size: W, ops: vec![Imm(1), Dn(0)] }),
        ("asr.l #3,d1", Instruction { mnemonic: Mnemonic::Asr, size: L, ops: vec![Imm(3), Dn(1)] }),
        ("lsl.w d2,d0", Instruction { mnemonic: Mnemonic::Lsl, size: W, ops: vec![Dn(2), Dn(0)] }),
        ("lsr.b #1,d0", Instruction { mnemonic: Mnemonic::Lsr, size: B, ops: vec![Imm(1), Dn(0)] }),
        ("rol.w #2,d0", Instruction { mnemonic: Mnemonic::Rol, size: W, ops: vec![Imm(2), Dn(0)] }),
        ("ror.w d1,d0", Instruction { mnemonic: Mnemonic::Ror, size: W, ops: vec![Dn(1), Dn(0)] }),
```
(Memory-shift form: add `("asl.w (a0)", ...)` only if the Aeon corpus uses it — the scout found only register forms, so a memory form is optional. Skip unless a golden regen shows Aeon needs it.)

Regenerate golden; add `shift_rotate_family` test.

- [ ] **Step 2: RED verify** → FAIL.
- [ ] **Step 3: GREEN — `encode_shift`:** distinguish immediate-count (`Imm` source) vs register-count (`Dn` source); map mnemonic → `(d, tt)`. Iterate to golden.
- [ ] **Step 4: GREEN verify** → PASS.
- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(sigil-isa): m68k shift/rotate family (asl/asr/lsl/lsr/rol/ror) byte-exact vs asl"
```

---

## Task 6: Bit-op family (`btst bset bclr` — static `#n` + dynamic `Dn`)

**Files:** same set.

**Encoding table:** dynamic (bit number in Dn) `0000 rrr 1 tt eeeeee` (`rrr`=Dn bit source; `tt`: btst=00, bchg=01, bclr=10, bset=11). Static (bit number immediate) `0000 1000 tt eeeeee` + bit-number word. Size is byte for memory EA, long for Dn EA (implicit — asl decides from the destination operand; verify via golden).

- [ ] **Step 1: RED — corpus entries**:

```rust
        // --- bit ops ---
        ("btst #7,d0", Instruction { mnemonic: Mnemonic::Btst, size: L, ops: vec![Imm(7), Dn(0)] }),
        ("bset #0,(a0)", Instruction { mnemonic: Mnemonic::Bset, size: B, ops: vec![Imm(0), Ind(0)] }),
        ("bclr #5,d1", Instruction { mnemonic: Mnemonic::Bclr, size: L, ops: vec![Imm(5), Dn(1)] }),
        ("btst d2,d0", Instruction { mnemonic: Mnemonic::Btst, size: L, ops: vec![Dn(2), Dn(0)] }),
        ("bset d1,(a0)", Instruction { mnemonic: Mnemonic::Bset, size: B, ops: vec![Dn(1), Ind(0)] }),
```

Regenerate golden; add `bit_ops_family` test.

- [ ] **Step 2: RED verify** → FAIL.
- [ ] **Step 3: GREEN — `encode_bit`:** static form when source is `Imm` (emit bit-number word then EA ext); dynamic when source is `Dn`. Iterate to golden.
- [ ] **Step 4: GREEN verify** → PASS.
- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(sigil-isa): m68k bit ops (btst/bset/bclr static+dynamic) byte-exact vs asl"
```

---

## Task 7: Single-EA family (`clr neg not tst tas` + `Scc`)

**Files:** same set.

**Encoding table:** `clr`=`0100 0010 ss eeeeee`; `neg`=`0100 0100 ss eeeeee`; `not`=`0100 0110 ss eeeeee`; `tst`=`0100 1010 ss eeeeee`; `tas`=`0100 1010 11 eeeeee` (byte only); `Scc`=`0101 cccc 11 eeeeee` (byte only, `cccc`=`Cond::cc()`).

- [ ] **Step 1: RED — corpus entries** (include the §5.5 blind spots `tas` and Scc `st`/`sf`/`sgt`):

```rust
        // --- single-EA family ---
        ("clr.w d0", Instruction { mnemonic: Mnemonic::Clr, size: W, ops: vec![Dn(0)] }),
        ("clr.l (a1)", Instruction { mnemonic: Mnemonic::Clr, size: L, ops: vec![Ind(1)] }),
        ("neg.w d0", Instruction { mnemonic: Mnemonic::Neg, size: W, ops: vec![Dn(0)] }),
        ("not.b d0", Instruction { mnemonic: Mnemonic::Not, size: B, ops: vec![Dn(0)] }),
        ("tst.w d0", Instruction { mnemonic: Mnemonic::Tst, size: W, ops: vec![Dn(0)] }),
        ("tst.l (a1)", Instruction { mnemonic: Mnemonic::Tst, size: L, ops: vec![Ind(1)] }),
        ("tas.b d0", Instruction { mnemonic: Mnemonic::Tas, size: B, ops: vec![Dn(0)] }),
        ("st d0", Instruction { mnemonic: Mnemonic::Scc(Cond::T), size: B, ops: vec![Dn(0)] }),
        ("sf d0", Instruction { mnemonic: Mnemonic::Scc(Cond::F), size: B, ops: vec![Dn(0)] }),
        ("sgt d0", Instruction { mnemonic: Mnemonic::Scc(Cond::Gt), size: B, ops: vec![Dn(0)] }),
```

Regenerate golden; add `single_ea_family` test.

- [ ] **Step 2: RED verify** → FAIL.
- [ ] **Step 3: GREEN — `encode_single_ea`** (one operand → EA in bits 5–0). Iterate to golden.
- [ ] **Step 4: GREEN verify** → PASS.
- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(sigil-isa): m68k single-EA family (clr/neg/not/tst/tas/Scc) byte-exact vs asl"
```

---

## Task 8: Control/misc family (`jmp jsr lea pea nop rts rte trap swap ext`)

**Files:** same set.

**Encoding table:** `jmp`=`0100 1110 11 eeeeee` (`4Exx`); `jsr`=`0100 1110 10 eeeeee`; `lea <ea>,An`=`0100 rrr 111 eeeeee` (`rrr`=An, long); `pea <ea>`=`0100 1000 01 eeeeee`; `nop`=`4E71`; `rts`=`4E75`; `rte`=`4E73`; `trap #n`=`0100 1110 0100 nnnn` (`4E4x`); `swap Dn`=`0100 1000 0100 0rrr` (`484x`); `ext.w Dn`=`4880+r`, `ext.l Dn`=`48C0+r`.

- [ ] **Step 1: RED — corpus entries** (jmp/jsr at **explicit** width — bare-symbol selection is B):

```rust
        // --- control / misc ---
        ("jmp ($1234).w", Instruction { mnemonic: Mnemonic::Jmp, size: W, ops: vec![AbsW(0x1234)] }),
        ("jmp ($12345678).l", Instruction { mnemonic: Mnemonic::Jmp, size: L, ops: vec![AbsL(0x12345678)] }),
        ("jsr ($1234).w", Instruction { mnemonic: Mnemonic::Jsr, size: W, ops: vec![AbsW(0x1234)] }),
        ("jmp (a0)", Instruction { mnemonic: Mnemonic::Jmp, size: W, ops: vec![Ind(0)] }),
        ("jmp (4,pc,d0.w)", Instruction { mnemonic: Mnemonic::Jmp, size: W, ops: vec![Disp8AnXn { d: 4, an: 0, xn: Xn::D(0), long: false }] }),
        ("lea (4,a0),a1", Instruction { mnemonic: Mnemonic::Lea, size: L, ops: vec![Disp16An(4, 0), An(1)] }),
        ("pea (a0)", Instruction { mnemonic: Mnemonic::Pea, size: L, ops: vec![Ind(0)] }),
        ("nop", Instruction { mnemonic: Mnemonic::Nop, size: W, ops: vec![] }),
        ("rts", Instruction { mnemonic: Mnemonic::Rts, size: W, ops: vec![] }),
        ("rte", Instruction { mnemonic: Mnemonic::Rte, size: W, ops: vec![] }),
        ("trap #0", Instruction { mnemonic: Mnemonic::Trap, size: W, ops: vec![Imm(0)] }),
        ("swap d0", Instruction { mnemonic: Mnemonic::Swap, size: W, ops: vec![Dn(0)] }),
        ("ext.w d0", Instruction { mnemonic: Mnemonic::Ext, size: W, ops: vec![Dn(0)] }),
        ("ext.l d1", Instruction { mnemonic: Mnemonic::Ext, size: L, ops: vec![Dn(1)] }),
```
Note the `(d8,PC,Xn)` jmp form uses the shared `brief_ext` with the PC-index EA mode (`111`/`011`); confirm `encode_ea` yields the PC-indexed mode for a `Disp8AnXn` whose base is PC — if the current `Disp8AnXn` only models `(d8,An,Xn)`, add a `Pcd8Xn` operand variant here and encode mode `111`,reg `011`. Verify against the golden.

Regenerate golden; add `control_misc_family` test.

- [ ] **Step 2: RED verify** → FAIL.
- [ ] **Step 3: GREEN — `encode_control`** (EA-only forms + the fixed no-operand words + `trap`/`swap`/`ext`). Iterate to golden.
- [ ] **Step 4: GREEN verify** → PASS.
- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(sigil-isa): m68k control/misc family (jmp/jsr/lea/pea/nop/rts/rte/trap/swap/ext) byte-exact vs asl"
```

---

## Task 9: Branch + DBcc family (`bra bsr Bcc`, `dbf dbeq`) — §5.5 hazard

**Files:** same set.

**Encoding table:** `Bcc`=`0110 cccc dddddddd`: 8-bit form (`Size::S`) puts the signed displacement in the low byte; 16-bit form (`Size::W`) sets the low byte to `00` and appends a 16-bit displacement word. `bra` = `cccc`=0000, `bsr` = `cccc`=0001. **No `.l` form** (2-wide only). `DBcc Dn,disp`=`0101 cccc 1100 1rrr` + 16-bit disp word (`dbf`=`Cond::F`, `dbeq`=`Cond::Eq`). Displacement values come from the `Disp(i32)` operand (resolved as `asl` emits — measured from the extension-word/PC address, mirroring the MOVE `Pcd16` convention). To keep golden snippets self-contained at `org 0`, use `*`-relative targets in the snippet and put the matching resolved `Disp` in the corpus.

- [ ] **Step 1: RED — corpus entries** (verify the 2-wide-only claim and DBcc's fixed 4-byte form). Use `asl` `*`-relative snippets; regenerate to discover the exact resolved displacement `asl` stores, then set `Disp(...)` to match (adjust after the first regen if the value differs):

```rust
        // --- branches (2-wide only) + DBcc (non-relaxable) ---
        ("bra.s *", Instruction { mnemonic: Mnemonic::Bra, size: S, ops: vec![Disp(-2)] }),
        ("bra.w *", Instruction { mnemonic: Mnemonic::Bra, size: W, ops: vec![Disp(-2)] }),
        ("bsr.s *", Instruction { mnemonic: Mnemonic::Bsr, size: S, ops: vec![Disp(-2)] }),
        ("bsr.w *", Instruction { mnemonic: Mnemonic::Bsr, size: W, ops: vec![Disp(-2)] }),
        ("beq.s *", Instruction { mnemonic: Mnemonic::Bcc(Cond::Eq), size: S, ops: vec![Disp(-2)] }),
        ("bne.w *", Instruction { mnemonic: Mnemonic::Bcc(Cond::Ne), size: W, ops: vec![Disp(-2)] }),
        ("dbf d0,*", Instruction { mnemonic: Mnemonic::Dbcc(Cond::F), size: W, ops: vec![Dn(0), Disp(-2)] }),
        ("dbeq d1,*", Instruction { mnemonic: Mnemonic::Dbcc(Cond::Eq), size: W, ops: vec![Dn(1), Disp(-2)] }),
```
**IMPORTANT:** run `gen-m68k-vectors` first, inspect the emitted bytes for the displacement field, and set each `Disp(...)` so `encode` reproduces exactly what `asl` stored (the `*` self-reference resolves to a specific signed value; do not guess — read it from the regenerated golden and the disassembly).

Add two hazard assertions to `encode_m68k.rs`:
```rust
#[test]
fn branch_family_is_two_wide_only() {
    check(&["bra.s *", "bra.w *", "bsr.s *", "bsr.w *", "beq.s *", "bne.w *"]);
    // .s form is 2 bytes; .w form is 4 bytes.
    let golden = parse_golden_m68k(GOLDEN);
    assert_eq!(golden_bytes(&golden, "bra.s *").len(), 2, "bra.s must be 2 bytes");
    assert_eq!(golden_bytes(&golden, "bra.w *").len(), 4, "bra.w must be 4 bytes");
}

#[test]
fn dbcc_is_fixed_four_bytes() {
    check(&["dbf d0,*", "dbeq d1,*"]);
    let golden = parse_golden_m68k(GOLDEN);
    assert_eq!(golden_bytes(&golden, "dbf d0,*").len(), 4, "DBcc is fixed 4 bytes (non-relaxable)");
}
```

- [ ] **Step 2: RED verify** → FAIL.
- [ ] **Step 3: GREEN — `encode_branch` + `encode_dbcc`.** Branch: `0x6000 | cc<<8`; `.s`→low byte = `disp as i8`; `.w`→low byte `00` + `disp as i16` word. Reject `Size::L`/`Size::B` for branches with `UnsupportedForm`. DBcc: `0x50C8 | cc<<8 | reg` + `disp as i16` word. Iterate to golden.
- [ ] **Step 4: GREEN verify** — `cargo test -p sigil-isa` → PASS (including the two hazard tests).
- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(sigil-isa): m68k branches (2-wide bra/bsr/Bcc) + DBcc non-relaxable — §5.5 hazard vectors"
```

---

## Task 10: MOVEM family — §5.5 register-mask bit-order reversal

**Files:** same set.

**Encoding table:** `movem`=`0100 1d00 1s eeeeee` + mask word. `d`=direction: reg→mem=0 (`0x4880`), mem→reg=1 (`0x4C80`). `s`=size: .w=0, .l=1 (`|0x0040`). **Mask bit order:** for control / postincrement / absolute modes, bit0=D0..bit7=D7, bit8=A0..bit15=A7. For **`-(An)` predecrement**, the mask is **bit-reversed** (bit0=A7..bit15=D0). `RegList(u16)` always holds the *canonical* (non-reversed) mask; `encode_movem` reverses it iff the memory operand is `PreDec`.

- [ ] **Step 1: RED — corpus entries** exercising every memory mode + D/A-crossing lists on both sides (this is the hazard; be exhaustive). Canonical mask bits: D0=bit0..D7=bit7, A0=bit8..A7=bit15.

```rust
        // --- MOVEM: register-store (to -(An)) and register-load (from (An)+/others) ---
        // masks: d0-d7 = 0x00FF; a0-a6 = 0x7F00; d0-a6 (all-but-a7) = 0x7FFF; single a3 = 0x0800; d3/d5 = 0x0028
        ("movem.l d0-d7/a0-a6,-(sp)", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![RegList(0x7FFF), PreDec(7)] }),
        ("movem.l (sp)+,d0-d7/a0-a6", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![PostInc(7), RegList(0x7FFF)] }),
        ("movem.l a2,-(sp)", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![RegList(0x0400), PreDec(7)] }),
        ("movem.l d3-d4,(a3)", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![RegList(0x0018), Ind(3)] }),
        ("movem.l d3-d4,(8,a3)", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![RegList(0x0018), Disp16An(8, 3)] }),
        ("movem.w d0-d6/a2,(a1)", Instruction { mnemonic: Mnemonic::Movem, size: W, ops: vec![RegList(0x047F), Ind(1)] }),
        ("movem.l (a0)+,d0-a4", Instruction { mnemonic: Mnemonic::Movem, size: L, ops: vec![PostInc(0), RegList(0x1FFF)] }),
```
**IMPORTANT:** confirm each canonical mask against `asl`'s bytes after regen — mask math is the exact hazard this task exists to pin. The predecrement forms (`-(sp)`) must show a *reversed* mask word in the golden vs the direct interpretation; the postinc/indirect forms must show the direct mask.

Regenerate golden; add the hazard test:
```rust
#[test]
fn movem_predecrement_mask_is_reversed() {
    // The SAME register set to -(sp) vs from (sp)+ must produce DIFFERENT mask words
    // (predecrement reverses the bit order); prove the reversal is applied.
    check(&[
        "movem.l d0-d7/a0-a6,-(sp)", "movem.l (sp)+,d0-d7/a0-a6",
        "movem.l a2,-(sp)", "movem.l d3-d4,(a3)", "movem.l d3-d4,(8,a3)",
        "movem.w d0-d6/a2,(a1)", "movem.l (a0)+,d0-a4",
    ]);
}
```

- [ ] **Step 2: RED verify** → FAIL.
- [ ] **Step 3: GREEN — `encode_movem`:** pick base by direction (which operand is the RegList vs the memory EA), OR-in size, encode the memory EA into bits 5–0, then append the mask word — **reversed (`mask.reverse_bits()`) iff the memory operand is `PreDec`**, else canonical. Iterate to golden.
- [ ] **Step 4: GREEN verify** → PASS.
- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(sigil-isa): m68k MOVEM with -(An) mask bit-order reversal — §5.5 hazard vectors"
```

---

## Task 11: Specials (`movep addx cmpm`) — remaining §5.5 blind spots

**Files:** same set.

**Encoding table:** `movep Dn,(d16,An)` / `movep (d16,An),Dn`=`0000 rrr 1 om 001 aaa` + 16-bit disp (`rrr`=Dn, `aaa`=An, `om`=opmode: mem→reg .w=100/.l=101, reg→mem .w=110/.l=111). `addx Dn,Dn`=`1101 rrr 1 ss 00 0 rrr` (`ss` size, Dn,Dn form only). `cmpm (Ay)+,(Ax)+`=`1011 xxx 1 ss 001 yyy`.

- [ ] **Step 1: RED — corpus entries**:

```rust
        // --- specials ---
        ("movep.w (4,a1),d0", Instruction { mnemonic: Mnemonic::Movep, size: W, ops: vec![Disp16An(4, 1), Dn(0)] }),
        ("movep.l d0,(8,a1)", Instruction { mnemonic: Mnemonic::Movep, size: L, ops: vec![Dn(0), Disp16An(8, 1)] }),
        ("addx.b d1,d0", Instruction { mnemonic: Mnemonic::Addx, size: B, ops: vec![Dn(1), Dn(0)] }),
        ("addx.l d3,d2", Instruction { mnemonic: Mnemonic::Addx, size: L, ops: vec![Dn(3), Dn(2)] }),
        ("cmpm.w (a0)+,(a1)+", Instruction { mnemonic: Mnemonic::Cmpm, size: W, ops: vec![PostInc(0), PostInc(1)] }),
```

Regenerate golden; add `specials_family` test.

- [ ] **Step 2: RED verify** → FAIL.
- [ ] **Step 3: GREEN — `encode_movep` / `encode_addx` / `encode_cmpm`** per the table (direction from operand order). Iterate to golden.
- [ ] **Step 4: GREEN verify** → PASS.
- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(sigil-isa): m68k specials (movep/addx/cmpm) byte-exact vs asl"
```

---

## Task 12: `sigil-backend-m68k` adapter crate

**Files:**
- Create: `crates/sigil-backend-m68k/Cargo.toml`
- Create: `crates/sigil-backend-m68k/src/lib.rs`
- Modify: `Cargo.toml` (workspace `members`)

- [ ] **Step 1: Create the Cargo.toml**

`crates/sigil-backend-m68k/Cargo.toml`:
```toml
[package]
name = "sigil-backend-m68k"
version = "0.1.0"
edition = "2021"

[dependencies]
sigil-ir = { path = "../sigil-ir" }
sigil-isa = { path = "../sigil-isa" }
sigil-span = { path = "../sigil-span" }
```

Add `"crates/sigil-backend-m68k",` to the `members` list in the workspace root `Cargo.toml`.

- [ ] **Step 2: Write the failing test + impl** — `crates/sigil-backend-m68k/src/lib.rs` (mirrors `sigil-backend-z80`):

```rust
//! 68000 `Backend` implementation: binds the CPU-agnostic `sigil_ir::Backend`
//! trait to `sigil_isa::m68k` and turns fully-resolved instructions into
//! `DataFragment`s. Symbolic-target lowering (branch PcRel fixups, jmp/jsr
//! width selection) is deferred to sub-project B with the linker.

use sigil_ir::backend::{Backend, Cpu, LowerError};
use sigil_ir::DataFragment;
use sigil_isa::m68k::{Instruction, Mnemonic, Operand};
use sigil_span::Span;

/// Re-export the m68k vocabulary so downstream crates (the AS front-end) can
/// construct instructions without a *direct* `sigil-isa` dependency.
pub use sigil_isa::m68k;

/// The 68000 backend. Stateless.
pub struct M68kBackend;

impl Backend for M68kBackend {
    type Mnemonic = Mnemonic;
    type Operand = Operand;

    fn cpu(&self) -> Cpu {
        Cpu::M68000
    }

    /// The 68000 needs a size that the current `Backend::lower` signature does not
    /// carry (Z80 never did). Rather than mutate the shared trait in A — which would
    /// ripple into `sigil-backend-z80` and the front-end — the trait method assumes
    /// **word** size (the correct default for the size-less mnemonics and the common
    /// case) and the **size-carrying tested path is `lower_inst`**. Whether to add a
    /// `size` param to the trait is a sub-project-C decision (the front-end is the
    /// real caller). Callers needing `.b`/`.l`/`.s` MUST use `lower_inst`.
    fn lower(&self, mnemonic: Mnemonic, operands: &[Operand], span: Span) -> Result<DataFragment, LowerError> {
        let inst = Instruction { mnemonic, size: sigil_isa::m68k::Size::W, ops: operands.to_vec() };
        self.lower_inst(&inst, span)
    }
}

impl M68kBackend {
    /// Lower a fully-formed `Instruction` (size already chosen) to a fragment.
    /// This is the primary, size-explicit adapter path (see the `lower` trait doc).
    pub fn lower_inst(&self, inst: &Instruction, span: Span) -> Result<DataFragment, LowerError> {
        let bytes = m68k::encode(inst).map_err(|e| LowerError { message: e.to_string() })?;
        Ok(DataFragment { bytes, fixups: vec![], span })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_isa::m68k::Size;
    use sigil_span::SourceId;

    fn span() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    #[test]
    fn lowers_resolved_instruction_via_encode() {
        let b = M68kBackend;
        // move.w d1,d0 → 30 01
        let inst = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Operand::Dn(1), Operand::Dn(0)] };
        let frag = b.lower_inst(&inst, span()).unwrap();
        assert_eq!(frag.bytes, vec![0x30, 0x01]);
        assert!(frag.fixups.is_empty());
    }

    #[test]
    fn unsupported_form_becomes_lower_error() {
        let b = M68kBackend;
        // move with immediate destination is illegal.
        let inst = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Operand::Dn(0), Operand::Imm(1)] };
        assert!(b.lower_inst(&inst, span()).is_err());
    }

    #[test]
    fn reexports_m68k_vocabulary() {
        use crate::m68k::{Cond, Mnemonic, Operand, Size};
        let _ = (Mnemonic::Bcc(Cond::Eq), Operand::Dn(0), Size::W);
    }

    #[test]
    fn cpu_is_m68000() {
        assert_eq!(M68kBackend.cpu(), Cpu::M68000);
    }
}
```
**Decision surfaced (see the `lower` doc comment above):** the current `Backend::lower` has no `size` parameter (Z80 never needed one; 68000 does). M1.A keeps the trait untouched — `lower` assumes word size and delegates to `lower_inst`, which is the size-explicit tested path. Whether to add `size` to the trait is a **sub-project C** decision (the front-end is the real caller). Keep A's change additive.

- [ ] **Step 3: Run test to verify it passes**

Run: `cargo test -p sigil-backend-m68k`
Expected: PASS (4 tests).

- [ ] **Step 4: Commit**

```bash
git add crates/sigil-backend-m68k Cargo.toml
git commit -m "feat(sigil-backend-m68k): thin Backend adapter over sigil-isa::m68k (lower_inst → DataFragment)"
```

---

## Task 13: Crate-graph guard for `sigil-backend-m68k`

**Files:**
- Modify: `crates/sigil-cli/tests/crate_graph.rs`

- [ ] **Step 1: Write the failing assertion** — add to `crate_graph_is_one_way()` (after the `sigil-backend-z80` assertion, ~line 291):

```rust
    // sigil-backend-m68k wraps the ISA: depends on sigil-ir + sigil-isa (+ span),
    // exactly mirroring sigil-backend-z80.
    assert_eq!(
        get("sigil-backend-m68k"),
        vec!["sigil-ir".to_string(), "sigil-isa".to_string(), "sigil-span".to_string()],
        "sigil-backend-m68k must depend on sigil-ir, sigil-isa, sigil-span only"
    );
```

- [ ] **Step 2: Run test to verify it passes** (the crate already has exactly those deps from Task 12)

Run: `cargo test -p sigil-cli --test crate_graph`
Expected: PASS. (If Task 12's Cargo.toml is correct, this is green immediately; if it fails, the failure names the wrong dep set — fix Task 12's Cargo.toml.)

- [ ] **Step 3: Verify `sigil-isa` stayed zero-dep**

Run: `cargo test -p sigil-cli --test crate_graph crate_graph_is_one_way`
Expected: PASS including rule (a) `sigil-isa` has no workspace deps (the new mnemonics/operands added no deps).

- [ ] **Step 4: Commit**

```bash
git add crates/sigil-cli/tests/crate_graph.rs
git commit -m "test(sigil-cli): crate-graph guard for sigil-backend-m68k (deps = sigil-ir+sigil-isa+sigil-span)"
```

---

## Task 14: Final gate — full-corpus coverage, docs, clean workspace

**Files:**
- Modify: `crates/sigil-isa/src/lib.rs` (update the M0.5-spike doc comment to reflect full-ISA scope)
- Verify only: whole workspace

- [ ] **Step 1: Update the `m68k` module doc** in `crates/sigil-isa/src/lib.rs` — replace the "M0.5 spike / MOVE EA matrix" description with the M1.A reality:

```rust
/// # 68000 encoder (M1.A — full Aeon ISA)
///
/// `m68k::encode` turns a resolved `m68k::Instruction` into big-endian bytes via
/// per-family procedural encoders sharing one `encode_ea`/`brief_ext` machinery.
/// Scope is every 68000 instruction/EA form the Aeon source (@ aeon `c7aaca6`) uses:
/// ~46 mnemonic families, all 12 EA modes (brief-extension indexed form only —
/// no 68020 extensions). Proven byte-identical to `asl` by the committed golden
/// corpus (`tests/m68k_golden_vectors.txt`), with dedicated §5.5 hazard vectors
/// (MOVEM `-(An)` mask reversal, 2-wide branches, DBcc non-relaxability, MOVE
/// SR/CCR, movep/addx/cmpm/tas/Scc). Symbolic-target width selection and PcRel
/// branch fixups are the linker's job (sub-project B); the encoder takes explicit,
/// already-resolved EA forms and displacements.
pub mod m68k;
```

- [ ] **Step 2: Confirm the corpus-coverage invariant holds** — `tests/m68k_golden.rs::golden_covers_the_full_corpus` and `golden_snippets_are_unique` and `encode_m68k.rs::all_forms_match_golden` collectively guarantee every corpus form has a unique golden and encodes to it. Verify no family was dropped:

Run: `cargo test -p sigil-isa --test m68k_golden --test encode_m68k`
Expected: PASS; `golden_covers_the_full_corpus` confirms golden count == corpus count.

- [ ] **Step 3: Whole-workspace green + clippy clean**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all green, clippy clean.

- [ ] **Step 4: Commit**

```bash
git add crates/sigil-isa/src/lib.rs
git commit -m "docs(sigil-isa): m68k module doc — full Aeon ISA (M1.A) supersedes MOVE-only spike note"
```

---

## Task 11b (added during execution): MOVEA — plan-gap fix

`movea` was listed in the design doc §2.1 move/movea family but the Task 2–11 breakdown never assigned it a step, leaving `Mnemonic::Movea` with no dispatch arm. The Task 11 implementer flagged it; the corpus scout found `movea.w`/`movea.l` used ~140× in Aeon, so the encoder is incomplete without it. Fixed in commit `966fd3a`: `encode_movea` (MOVEA == MOVE with an An destination, `.w`/`.l` only, mode `001`), 5 corpus entries + golden, `movea_family` test. This also made `encode`'s `match` exhaustive over `Mnemonic` (the `other => UnsupportedForm` catch-all became unreachable and was removed).

## Self-review checklist (run before starting execution)

- **Spec coverage:** §2 mnemonic families → Tasks 2–11; §3 vocab (`Cond`/`RegList`/`Size::S`) → Task 1; §4 adapter → Task 12; crate-graph → Task 13; §5 hazard vectors (MOVEM reversal, 2-wide branches, DBcc, SR/CCR, movep/addx/cmpm/tas/Scc) → Tasks 3/7/9/10/11 with dedicated assertions; §7 acceptance → Task 14. **Deferred to B (not in any task, by design):** bare-symbol jmp/jsr width selection, PcRel branch fixups, Pcd16→disp — verified absent from tasks intentionally.
- **Type consistency:** `Mnemonic`/`Cond`/`Operand::{RegList,Disp,Ccr,Sr}`/`Size::S` defined in Task 1 and used verbatim in Tasks 2–12; `M68kBackend`/`lower_inst` consistent between Task 12 and Task 13; `check(&[...])` helper reused from the existing `encode_m68k.rs`.
- **Golden-regen dependency:** every encoder task regenerates and commits `m68k_golden_vectors.txt`; the branch/DBcc and MOVEM tasks explicitly warn to *read* the resolved displacement/mask from `asl` rather than guess.
- **Placeholder scan:** no silent placeholders. The one design gap (the `Backend::lower` size-signature) is handled explicitly — `lower` assumes word and delegates to the size-explicit `lower_inst`, documented as a sub-project-C decision — not a silent TODO or a misleading default helper.
