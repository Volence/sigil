# Sigil M1.C — Spike 0 + Backend Multiplexing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** De-risk the M1.C unknowns (extract the 4 `deform_table_sine` golden vectors, pin `int()` truncation semantics, enumerate the exact operator/builtin/directive surface) and land the CPU→backend dispatch fork in `sigil-frontend-as` so 68k instructions route to `M68kBackend`.

**Architecture:** Spike 0 is investigation with committed artifacts (golden `.bin` vectors + a findings note) — no production logic. Task 1 is a pure refactor: split the Z80-hardcoded `lower_instruction` into a `state.cpu` dispatch calling `lower_z80` (existing logic, renamed backend field) vs a `lower_m68k` stub that later tasks (T4/T5) flesh out. Existing Z80 tests stay green.

**Tech Stack:** Rust (workspace crates), `dd`/`python3` for vector extraction + truncation probe, `aeon/tools/asl` (native AS 1.42 Bld 212) as the diff oracle. Design doc: `docs/superpowers/specs/2026-07-03-sigil-m1c-as-68k-frontend-design.md`.

---

## File Structure

| File | Responsibility |
|---|---|
| `crates/sigil-frontend-as/tests/vectors/sine_goldens/*.bin` | 4×256-byte golden tables extracted from ref ROM (Spike 0) |
| `docs/superpowers/notes/2026-07-03-m1c-spike0-findings.md` | Truncation direction + full operator/builtin/directive enumeration (Spike 0) |
| `crates/sigil-frontend-as/src/eval.rs` | Add `m68k` backend field; split `lower_instruction` → `lower_z80` / `lower_m68k` (Task 1) |

---

## Reference facts (verified 2026-07-03)

- Ref ROM: `aeon/s4.bin` (aeon `9bacc93`), flat image, VMA = file offset.
- Sine table ROM offsets (each 256 bytes):
  - `DeformTable_OJZ_Calm`  A=96 P=64 → `0x11402`
  - `DeformTable_Rocking`   A=20 P=64 → `0x118D2`
  - `DeformTable_Haze`      A=16 P=64 → `0x1169A`
  - `DeformTable_Shimmer`   A=8  P=32 → `0x11528`
- Macro (`aeon/engine/parallax_macros.inc:211`): `dc.b int(AMPLITUDE * sin(6.283185307179586 * i / PERIOD))`, `i` = 0..255.
- `Cpu` enum (`sigil-ir/src/backend.rs:5`): `{ Z80, M68000 }` (only two variants).
- `M68kBackend` (`sigil-backend-m68k/src/lib.rs:18`): unit struct `pub struct M68kBackend;`, methods `lower_inst`, `lower_branch`, `lower_jmp_jsr_sym`, `lower_pcrel_ea`.
- eval.rs imports at line 9–10: `use sigil_backend_z80::z80::{Cond, Mnemonic, Operand, Reg16, Reg8}; use sigil_backend_z80::Z80Backend;`
- `Asm.backend: Z80Backend` field at `eval.rs:117`, constructed at `:147`.
- `lower_instruction` body: `eval.rs:893–924`.

---

## Task 0 (Spike): Extract sine goldens + pin truncation + enumerate AS surface

**Files:**
- Create: `crates/sigil-frontend-as/tests/vectors/sine_goldens/ojz_calm_a96_p64.bin`
- Create: `crates/sigil-frontend-as/tests/vectors/sine_goldens/rocking_a20_p64.bin`
- Create: `crates/sigil-frontend-as/tests/vectors/sine_goldens/haze_a16_p64.bin`
- Create: `crates/sigil-frontend-as/tests/vectors/sine_goldens/shimmer_a8_p32.bin`
- Create: `docs/superpowers/notes/2026-07-03-m1c-spike0-findings.md`

- [ ] **Step 1: Extract the 4 golden tables from the reference ROM**

Run (from `/home/volence/sonic_hacks/sigil`):

```bash
mkdir -p crates/sigil-frontend-as/tests/vectors/sine_goldens
REF=/home/volence/sonic_hacks/aeon/s4.bin
dd if=$REF of=crates/sigil-frontend-as/tests/vectors/sine_goldens/ojz_calm_a96_p64.bin bs=1 skip=$((0x11402)) count=256 status=none
dd if=$REF of=crates/sigil-frontend-as/tests/vectors/sine_goldens/rocking_a20_p64.bin  bs=1 skip=$((0x118D2)) count=256 status=none
dd if=$REF of=crates/sigil-frontend-as/tests/vectors/sine_goldens/haze_a16_p64.bin     bs=1 skip=$((0x1169A)) count=256 status=none
dd if=$REF of=crates/sigil-frontend-as/tests/vectors/sine_goldens/shimmer_a8_p32.bin   bs=1 skip=$((0x11528)) count=256 status=none
for f in crates/sigil-frontend-as/tests/vectors/sine_goldens/*.bin; do printf '%s %s\n' "$(wc -c < "$f")" "$f"; done
```

Expected: each file reports `256`. Sanity: index 0 of every table is `0x00` (`int(A*sin(0))=0`).

- [ ] **Step 2: Pin `int()` truncation direction against the goldens**

The goldens ARE asl's output, so determine which rounding mode reproduces all 4 tables. Run:

```bash
python3 - <<'PY'
import math
cases = {
  "ojz_calm_a96_p64.bin": (96, 64),
  "rocking_a20_p64.bin":  (20, 64),
  "haze_a16_p64.bin":     (16, 64),
  "shimmer_a8_p32.bin":   (8, 32),
}
base = "crates/sigil-frontend-as/tests/vectors/sine_goldens/"
def s8(b): return b-256 if b>=128 else b
modes = {
  "trunc":      lambda x: math.trunc(x),
  "floor":      lambda x: math.floor(x),
  "round_half": lambda x: int(math.floor(x+0.5)),
  "round_even": lambda x: round(x),
}
for name,(A,P) in cases.items():
    gold = open(base+name,"rb").read()
    for mname,fn in modes.items():
        ok = all(s8(gold[i]) == fn(A*math.sin(6.283185307179586*i/P)) for i in range(256))
        print(f"{name:24} {mname:10} {'MATCH' if ok else 'differ'}")
PY
```

Expected: exactly one mode reports `MATCH` for all 4 files (hypothesis: `trunc` — AS `int()` truncates toward zero). Record which mode wins.

- [ ] **Step 3: Enumerate the exact AS operator / builtin / directive surface**

Run (from `/home/volence/sonic_hacks/aeon`):

```bash
cd /home/volence/sonic_hacks/aeon
echo "== operators =="; grep -rhoE '<>|~~|!=|\|\||&&|\bmod\b|\bshl\b|\bshr\b' games engine --include='*.asm' --include='*.inc' | sort | uniq -c | sort -rn
echo "== builtins =="; grep -rhoE '\b(sin|cos|sqrt|int|strlen|substr|strstr|upstring|lowstring|charfromstr)\b\s*\(' games engine --include='*.asm' --include='*.inc' | sed 's/[( ].*//' | sort | uniq -c | sort -rn
echo "== data/reserve directives =="; grep -rhoE '\b(dc|ds)\.(b|w|l)\b|\b(even|align|org)\b' games engine --include='*.asm' --include='*.inc' | sort | uniq -c | sort -rn
echo "== !name escapes =="; grep -rnE '(^|[^a-zA-Z0-9_])![a-z]' games engine --include='*.asm' --include='*.inc' | grep -vE '!=' | head
echo "== .ATTRIBUTE / ALLARGS / MOMCPUNAME =="; grep -rnE '\.ATTRIBUTE|ALLARGS|MOMCPUNAME' games engine --include='*.asm' --include='*.inc' | head
```

- [ ] **Step 4: Write the findings note**

Create `docs/superpowers/notes/2026-07-03-m1c-spike0-findings.md` with:
- The confirmed `int()` rounding mode (from Step 2) and the T8 gate implication (bit-match feasible → keep D2 primary path; if a mode matched, source-cure fallback is not triggered).
- The complete operator set to add in T2 (from Step 3), with the AS semantics for each (`mod`/`#`=modulo, `<>`/`!=`=not-equal, `||`/`&&`=logical-or/and yielding `0`/`-1`, `!`=bitwise-or, `~~`=boolean-not).
- The complete builtin set for T3.
- The complete data/reserve/align directive set for T6.
- The `!name` escape sites and `.ATTRIBUTE`/`ALLARGS`/`MOMCPUNAME` sites for T7/T9.
- A one-line-per-task scope confirmation so T2–T9 are pinned, not guessed.

- [ ] **Step 5: Commit**

```bash
cd /home/volence/sonic_hacks/sigil
git add crates/sigil-frontend-as/tests/vectors/sine_goldens docs/superpowers/notes/2026-07-03-m1c-spike0-findings.md
git commit -m "spike(sigil-m1c): sine goldens + int() truncation + AS surface enumeration"
```

---

## Task 1: CPU→backend dispatch fork (`lower_z80` / `lower_m68k`)

**Files:**
- Modify: `crates/sigil-frontend-as/src/eval.rs` (field at `:117`/`:147`; `lower_instruction` at `:893–924`)
- Test: `crates/sigil-frontend-as/src/eval.rs` (inline `#[cfg(test)]` module — follows existing convention, e.g. the test at `:1306`)

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)]` module in `eval.rs`. It proves (a) Z80 still assembles (regression) and (b) a 68k instruction under `cpu 68000` reaches the m68k dispatch arm, surfacing the temporary stub diagnostic:

```rust
#[test]
fn m68k_instruction_reaches_m68k_dispatch_stub() {
    // Minimal 68k program: switch CPU, emit one instruction.
    let src = "    cpu 68000\nStart:\n    move.w d0,d1\n";
    let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
    let res = run(src, &opts);
    // T1 only wires dispatch; the m68k path is a stub, so assembly reports the
    // sentinel diagnostic (replaced with real lowering in T4/T5).
    let diags = match res {
        Ok(_) => panic!("expected stub diagnostic, got clean assembly"),
        Err(d) => d,
    };
    assert!(
        diags.iter().any(|d| d.message.contains("68k instruction lowering not yet implemented")),
        "expected m68k stub diagnostic, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sigil-frontend-as m68k_instruction_reaches_m68k_dispatch_stub`
Expected: FAIL — either the `move.w` mnemonic errors as "not a mnemonic" (Z80 `mnemonic()` returns `None`) or the assertion trips because the sentinel message is absent.

- [ ] **Step 3: Add the `m68k` backend field**

In `struct Asm` (`eval.rs:115`), rename the Z80 field for clarity and add the m68k field:

```rust
struct Asm {
    builder: IrBuilder,
    z80: Z80Backend,
    m68k: M68kBackend,
    state: crate::state::AsmState,
    // ... rest unchanged ...
}
```

In `Asm::new` (`eval.rs:145`), update construction:

```rust
        Asm {
            builder: IrBuilder::new(),
            z80: Z80Backend,
            m68k: M68kBackend,
            state: crate::state::AsmState::new(opts.initial_cpu),
            // ... rest unchanged ...
```

Add the import near line 10:

```rust
use sigil_backend_m68k::M68kBackend;
```

Add the crate dependency to `crates/sigil-frontend-as/Cargo.toml` under `[dependencies]` (mirror the existing `sigil-backend-z80` line):

```toml
sigil-backend-m68k = { path = "../sigil-backend-m68k" }
```

- [ ] **Step 4: Split `lower_instruction` into a CPU dispatch**

Replace `lower_instruction` (`eval.rs:893–924`) with a thin dispatcher plus the extracted Z80 body and an m68k stub. `mnemonic()` is Z80-specific, so the fork must precede it:

```rust
    fn lower_instruction(&mut self, mn: &str, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        match self.state.cpu {
            Cpu::Z80 => self.lower_z80(mn, rest, span),
            Cpu::M68000 => self.lower_m68k(mn, rest, span),
        }
    }

    fn lower_z80(&mut self, mn: &str, rest: &[Token], span: Span) {
        let atoms = match parse_operands(rest) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        let m = match mnemonic(mn) {
            Some(m) => m,
            None => {
                self.err(span, "not a mnemonic");
                return;
            }
        };
        match self.build_operands(m, &atoms, span) {
            Some(Lowered::Fixed(ops)) => {
                let f = self.z80.lower(m, &ops, span);
                self.emit_frag(f, span);
            }
            Some(Lowered::Rel(cond, target)) => {
                let f = self.z80.lower_rel(m, cond, target, span);
                self.emit_frag(f, span);
            }
            Some(Lowered::Abs16(ops, target)) => {
                let f = self.z80.lower_abs16(m, &ops, target, span);
                self.emit_frag(f, span);
            }
            None => {}
        }
    }

    /// Stub: real 68k mnemonic/operand lowering lands in M1.C T4/T5.
    fn lower_m68k(&mut self, _mn: &str, _rest: &[Token], span: Span) {
        let _ = &self.m68k; // field is wired now; used from T4/T5 onward.
        self.err(span, "68k instruction lowering not yet implemented");
    }
```

Note: `open_section_if_needed()` moved up into `lower_instruction` so both paths share it. Verify no other caller of `open_section_if_needed` relied on it being inside the old body (it did not — it was the first line).

- [ ] **Step 5: Run the new test and the full Z80 regression suite**

Run: `cargo test -p sigil-frontend-as`
Expected: PASS — the new `m68k_instruction_reaches_m68k_dispatch_stub` passes, and every pre-existing Z80 test still passes (the refactor is behavior-preserving for `Cpu::Z80`).

- [ ] **Step 6: Clippy + workspace build**

Run: `cargo clippy -p sigil-frontend-as -- -D warnings && cargo build --workspace`
Expected: clean (no warnings; workspace builds).

- [ ] **Step 7: Commit**

```bash
git add crates/sigil-frontend-as/src/eval.rs crates/sigil-frontend-as/Cargo.toml Cargo.lock
git commit -m "feat(sigil-frontend-as): CPU->backend dispatch fork (lower_z80/lower_m68k stub)"
```

---

## Self-Review

**Spec coverage (this increment):** Spike 0 covers design-doc §4 "Spike 0" bullet (goldens, `int()` truncation, operator/builtin/directive enumeration) + D2 golden extraction. Task 1 covers design-doc §2 D1 (backend split) + gap-analysis row "CPU→backend dispatch". T2–T10 are deferred to follow-on plans once Spike 0 pins their scope — intentional (Spike 0 output is an input to those plans).

**Placeholder scan:** the `lower_m68k` body is a real, compiling stub (not a placeholder) with a sentinel message asserted by the Task 1 test; it is explicitly scheduled for replacement in T4/T5.

**Type consistency:** `Cpu` matched exhaustively as `{Z80, M68000}` (no wildcard arm — a future third CPU would force a compile error, which is desirable). Field renamed `backend`→`z80` consistently at struct def, constructor, and all three call sites inside `lower_z80`. `M68kBackend` constructed as a unit struct (no `::new()`), matching `sigil-backend-m68k/src/lib.rs:18`.
