# Sigil M1.D T1 — String-valued `set` + `__FSTRING` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the AS front-end so `set` can hold a string value and the existing string builtins resolve a string-valued symbol, making `error_handler.asm`'s `__ErrorMessage` macros assemble in the non-debug ROM (the `strstr`/`strlen`/`while`/`trailing`-token diagnostic classes drop to 0).

**Architecture:** Strings live **only** in a new front-end-local `str_env: HashMap<String,String>` (§7.4: `sigil_ir::SymbolValue` stays `Int | Poison` — nothing string-typed enters the IR). `directive_set` detects a string RHS via the existing `eval_str` and stores it in `str_env`; `eval_str` gains a branch that resolves a bare identifier to its `str_env` value (qualified by the current scope, mirroring `SymbolTable::resolve`). With strings resolving, `strstr(.__str,"%<")` returns -1 for token-free strings, so the `while (.__pos>=0)` scan converges. Separately, infix `!` is corrected from bitwise-OR to **XOR** (probe-verified; it sits on the `__ErrorMessage` `.__align_flag` emit path).

**Tech Stack:** Rust workspace; `sigil-frontend-as` (evaluator), `sigil-ir` (`BinOp`/fold); asl 1.42 golden vectors via `gen_snippet_vectors`.

**Ground truth:** All semantics probe-verified against live asl in
`docs/superpowers/notes/2026-07-04-m1d-t1-string-set-probes.md`. Recon baseline
(before): `2000198 strstr` + `99 strlen` + `99 while` + `297 trailing` + `1 budget`
+ 6 EA sites (T2) + 2 one-offs.

---

## File Structure

- `crates/sigil-ir/src/expr.rs` — add `BinOp::Xor` variant + fold arm (`a ^ b`).
- `crates/sigil-frontend-as/src/expr.rs` — remap `Bang => (4, BinOp::Xor)`.
- `crates/sigil-frontend-as/src/eval.rs` — `str_env` field; `directive_set` string branch; `eval_str` identifier-resolution branch; `resolve_str` helper.
- `crates/sigil-frontend-as/tests/snippets_golden.txt` — new asl-verified golden blocks.
- `crates/sigil-frontend-as/tests/asl_snippets.rs` — no change (data-driven).

---

## Task 1: Correct infix `!` to XOR (`BinOp::Xor`)

Independent of the string work; do it first because it is small and self-contained.

**Files:**
- Modify: `crates/sigil-ir/src/expr.rs` (enum `BinOp` ~line 37; `fold` match ~line 123)
- Modify: `crates/sigil-frontend-as/src/expr.rs:38`
- Test: `crates/sigil-ir/src/expr.rs` (unit test module)

- [ ] **Step 1: Add a failing unit test in `sigil-ir/src/expr.rs`** (in the `#[cfg(test)] mod tests`, near the existing `bin(Or, …)` test)

```rust
#[test]
fn xor_folds() {
    // asl-verified infix `!`: 1!1=0, 3!1=2, 5!3=6 (probe 2026-07-04).
    assert_eq!(fold_pure(&bin(Xor, 1, 1)), Fold::Value(0));
    assert_eq!(fold_pure(&bin(Xor, 3, 1)), Fold::Value(2));
    assert_eq!(fold_pure(&bin(Xor, 5, 3)), Fold::Value(6));
}
```

- [ ] **Step 2: Run it — expect a compile error** (`Xor` not a variant)

Run: `cargo test -p sigil-ir xor_folds`
Expected: FAIL — `no variant named Xor found for enum BinOp`.

- [ ] **Step 3: Add the `Xor` variant + fold arm**

In `enum BinOp`, after `Or,`:
```rust
    Or,
    /// `!` — bitwise XOR (asl's infix `!`; probe-verified 2026-07-04:
    /// `1!1`=0, `3!1`=2, `5!3`=6 — NOT bitwise-OR).
    Xor,
```
In `fold`, after the `BinOp::Or => …` arm:
```rust
                    BinOp::Or => Fold::Value(a | b),
                    BinOp::Xor => Fold::Value(a ^ b),
```
(Grep `BinOp::Or` across the workspace first — if any other exhaustive match on
`BinOp` exists, add a `Xor` arm there too. As of this plan, `fold` is the only
match without a wildcard.)

- [ ] **Step 4: Remap `Bang` in the front-end parser**

`crates/sigil-frontend-as/src/expr.rs:38`, replace the `Bang => (4, BinOp::Or)`
arm (and its now-wrong comment) with:
```rust
        // `!` — AS's infix bitwise XOR (asl-verified 2026-07-04: `1!1`=0,
        // `3!1`=2, `5!3`=6; the earlier bitwise-OR reading was wrong — the
        // only prior golden `3!4`=7 can't tell OR from XOR). Same tier as `|`.
        // Drives `__ErrorMessage`'s `.__align_flag: set (((*)&1)!1)*$80`.
        Bang => (4, BinOp::Xor),
```

- [ ] **Step 5: Run both crates' tests**

Run: `cargo test -p sigil-ir xor_folds && cargo test -p sigil-frontend-as`
Expected: PASS. (The existing `(3!4)&$FF`=7 golden still holds: `3^4 == 7`.)

- [ ] **Step 6: Commit**

```bash
git add crates/sigil-ir/src/expr.rs crates/sigil-frontend-as/src/expr.rs
git commit -m "fix(sigil-m1d): infix ! is XOR not bitwise-OR (T1; drives __ErrorMessage .__align_flag)"
```

---

## Task 2: String-valued `set` symbols

**Files:**
- Modify: `crates/sigil-frontend-as/src/eval.rs` (struct `Asm` ~line 136; `Asm::new` ~line 184; `eval_str` ~line 570; `directive_set` ~line 1786; add `resolve_str` helper)

- [ ] **Step 1: Add the `str_env` field to `struct Asm`**

After the `env: SymbolTable,` field (~line 141):
```rust
    /// Front-end-only string-valued symbols (`.__str set "BUS ERROR"`).
    /// §7.4: strings NEVER enter `sigil_ir::SymbolValue`; they live here in the
    /// evaluator. Keyed by fully-qualified name exactly like `env` (see
    /// `resolve_str`). NOT carried across passes — asl `set` is a sequential
    /// per-pass assignment and every string symbol in the `__FSTRING` scan is
    /// assigned before it is read (probe p1/p4).
    str_env: std::collections::HashMap<String, String>,
```

- [ ] **Step 2: Initialise it in `Asm::new`**

In the `Asm { … }` literal (after `env: SymbolTable::new(),`):
```rust
            str_env: std::collections::HashMap::new(),
```

- [ ] **Step 3: Add the `resolve_str` helper** (near `eval_str`, ~line 560)

```rust
    /// Resolve a bare identifier reference to its string value, if it names a
    /// string-valued `set` symbol. Key-building mirrors `SymbolTable::resolve`:
    /// `.foo` → `"{scope}.foo"` (needs a scope), `A.b`/`foo` → verbatim.
    fn resolve_str(&self, name: &str) -> Option<String> {
        let key = if let Some(local) = name.strip_prefix('.') {
            format!("{}.{}", self.scope.as_deref()?, local)
        } else {
            name.to_string()
        };
        self.str_env.get(&key).cloned()
    }
```

- [ ] **Step 4: Teach `eval_str` to resolve a bare identifier**

In `eval_str` (~line 570), immediately after the `Tok::Str` literal branch and
before the `substr`/`lowstring` handling, add a single-identifier branch:
```rust
        if let [Token {
            tok: Tok::Ident(name),
            ..
        }] = toks
        {
            if let Some(s) = self.resolve_str(name) {
                return Some(s);
            }
        }
```
(Place it so a lone identifier that is a known string symbol resolves, but a
`substr(...)`/`lowstring(...)` call — which starts with an ident followed by
`(` — still reaches the existing call-shape branches below. The `[Token…]`
single-element slice pattern guarantees it only matches a lone identifier.)

- [ ] **Step 5: Teach `directive_set` to store a string RHS**

Replace the body of `directive_set` (~line 1786):
```rust
    fn directive_set(&mut self, name: &str, rest: &[Token], span: Span) {
        let q = qualify(name, self.scope.as_deref());
        // asl: `set` may bind a STRING (`.__str set "BUS ERROR"`,
        // `.__str set substr(.__str,0,.__pos)`). Detect the string shape via
        // `eval_str` (literal / substr / lowstring / string-symbol copy) BEFORE
        // the numeric fold, and store it front-end-only (§7.4). Probe p1/p4.
        if let Some(s) = self.eval_str(rest) {
            self.str_env.insert(q, s);
            return;
        }
        if let Some(v) = self.eval_all(rest, span) {
            self.env.define(&q, SymbolValue::Int(v));
        }
    }
```

- [ ] **Step 6: Verify the workspace still builds and existing tests pass**

Run: `cargo test -p sigil-frontend-as`
Expected: PASS (no regression; new behaviour is covered by Task 3 goldens).

- [ ] **Step 7: Commit**

```bash
git add crates/sigil-frontend-as/src/eval.rs
git commit -m "feat(sigil-m1d): string-valued set symbols via front-end str_env (T1; §7.4-clean)"
```

---

## Task 3: asl-verified snippet goldens

Every byte-affecting change lands with real-asl goldens (non-circularity
invariant: `gen_snippet_vectors` must churn ONLY the new blocks).

**Files:**
- Modify: `crates/sigil-frontend-as/tests/snippets_golden.txt` (append blocks)

- [ ] **Step 1: Append the new snippet blocks** (leave `--- bytes ---` empty; the generator fills them)

```text
=== t1_str_set_substr_whole ===
	cpu 68000
	padding off
	org 0
S:	set	"BUS ERROR"
	dc.b	substr(S, 0, 0)
	dc.b	0
--- bytes ---
=== t1_str_strstr_miss ===
	cpu 68000
	padding off
	org 0
S:	set	"BUS ERROR"
P:	set	strstr(S, "%<")
	dc.b	P&$FF
--- bytes ---
=== t1_str_strlen ===
	cpu 68000
	padding off
	org 0
S:	set	"BUS ERROR"
	dc.b	strlen(S)
--- bytes ---
=== t1_str_reassign_substr_self ===
	cpu 68000
	padding off
	org 0
S:	set	"HELLO"
S:	set	substr(S, 0, 3)
	dc.b	strlen(S)
	dc.b	substr(S, 0, 0)
--- bytes ---
=== t1_str_val_sym ===
	cpu 68000
	padding off
	org 0
S:	set	"$80"
	dc.b	val(S)
--- bytes ---
=== t1_str_empty ===
	cpu 68000
	padding off
	org 0
S:	set	""
	dc.b	strlen(S)
	dc.b	$AA
--- bytes ---
=== t1_bang_xor ===
	cpu 68000
	padding off
	org 0
	dc.b	(1)!1
	dc.b	(3)!1
	dc.b	5!3
--- bytes ---
=== t1_errormessage_representative ===
	cpu 68000
	padding off
	org 0
_eh_address_error	equ	$01
_eh_return		equ	$20
_eh_align_offset	equ	$80
DEBUGGER__EXTENSIONS__ENABLE	equ	1
_eh_default:	equ	0
MDDBG__ErrorHandler	equ	$000400
MDDBG__ErrorHandler_PagesController	equ	$000500
__FSTRING_GenerateArgumentsCode: macro string
	.__pos:	set	strstr(string,"%<")
	.__str:	set	string
	while (.__pos>=0)
		.__substr:	set	substr(.__str,.__pos,0)
		if (.__pos>0)
			.__str:	set	substr(.__str, 0, .__pos)
			.__pos:	set	strstr(.__str,"%<")
		else
			.__pos:	set	-1
		endif
	endm
	endm
__FSTRING_GenerateDecodedString: macro string, addnewline
	dc.b	substr(string, 0, 0)
	dc.b	0
	endm
__ErrorMessage:	macro string, opts
	__FSTRING_GenerateArgumentsCode string
	jsr	(MDDBG__ErrorHandler).l
	__FSTRING_GenerateDecodedString string, 0
	.__align_flag: set (((*)&1)!1)*_eh_align_offset
	dc.b	(opts)+_eh_return|.__align_flag
	!align	2
	jmp	(MDDBG__ErrorHandler_PagesController).l
	endm
BusError:
	__ErrorMessage	"BUS ERROR", _eh_default|_eh_address_error
--- bytes ---
```

Note: the `t1_errormessage_representative` block uses a **simplified** copy of the
`__FSTRING` macros — enough to exercise the string-set / while-converge / align
path and produce the exact production emit bytes, without the full EA switch
machinery (the aeon error strings carry no `%<` token, so the inner branches are
dead). The verified reference bytes are
`4EB9 00000400 / "BUS ERROR" / 00 / A1 / 00 / 4EF9 00000500`.

- [ ] **Step 2: Regenerate the golden bytes from real asl**

Run: `AEON_DIR=/home/volence/sonic_hacks/aeon cargo run -p sigil-frontend-as --bin gen_snippet_vectors`
Expected: stderr "wrote N snippet vectors". Then verify non-circularity:

Run: `git diff --stat crates/sigil-frontend-as/tests/snippets_golden.txt`
Expected: ONLY the 8 new blocks gained byte lines; **no pre-existing block changed**.
(If any existing block churned, STOP — a semantic regression leaked in.)

- [ ] **Step 3: Confirm `t1_errormessage_representative` bytes match the probe**

Inspect the generated bytes for that block; expected:
`4E B9 00 00 04 00 42 55 53 20 45 52 52 4F 52 00 A1 00 4E F9 00 00 05 00`.

- [ ] **Step 4: Run the golden test with sigil's own assembler**

Run: `cargo test -p sigil-frontend-as --test asl_snippets`
Expected: PASS — sigil reproduces every golden, including the 8 new blocks.
(If `t1_errormessage_representative` fails, sigil's emit diverges from asl —
triage before proceeding; do NOT edit the golden to match sigil.)

- [ ] **Step 5: Commit**

```bash
git add crates/sigil-frontend-as/tests/snippets_golden.txt
git commit -m "test(sigil-m1d): asl goldens for string-set/builtins + ! XOR + __ErrorMessage (T1)"
```

---

## Task 4: Recon verification + strict gates

**Files:** none (verification only)

- [ ] **Step 1: Re-run the full-build recon**

Run: `AEON_DIR=/home/volence/sonic_hacks/aeon cargo run -p sigil-harness --example m1c_full 2>&1 | tail -30`
Expected: the `strstr`/`strlen`/`while loop did not terminate`/`trailing tokens`/
`per-pass budget` classes are **gone (0)**. Remaining should be the 6
`out of scope for T#` EA sites (T2) + the 2 one-offs (`directive expects …`,
`unresolved long expression`) + anything newly exposed.

- [ ] **Step 2: Bucket & record any newly-exposed diagnostics**

If new classes appear, record them in the memory note and the spec (T1 acceptance
allows "≤ 6 EA sites + anything newly exposed (bucket and record it)"). Do NOT
silently absorb them.

- [ ] **Step 3: Run the strict gates**

Run:
```bash
SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/aeon cargo test -p sigil-harness
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: all green (m1b_gate 5, m1c_vector_table 1, regen A+B byte-identical,
stale_fold_repro 2 pass/2 ignored, workspace 0 failed, clippy clean).

- [ ] **Step 4: Update spec + memory note**

Mark T1 ✅ DONE in
`docs/superpowers/specs/2026-07-04-sigil-m1d-full-rom-spec.md` (§T1), recording
the `str_env` design decision, the `!`-XOR fix, and the post-T1 recon class list.
Update the `sigil-m0-core-progress` memory note's M1.D paragraph.

- [ ] **Step 5: (No separate commit needed** if steps only verified; commit the
doc/memory updates)

```bash
git add docs/superpowers/specs/2026-07-04-sigil-m1d-full-rom-spec.md
git commit -m "docs(sigil-m1d): mark T1 done; record str_env design + ! XOR + post-T1 recon"
```

---

## Self-Review

- **Spec coverage (§T1):** `SymbolValue Int|Str` → resolved as front-end `str_env`
  (§7.4-faithful; Task 2). String threading through `set`/eval/builtins → Tasks 2.
  `while` convergence → falls out of Task 2 (verified Task 4 step 1). Representative
  `__ErrorMessage` golden → Task 3 block 8. Edge cases (empty string, `substr` len 0,
  `strstr` miss, `val` on string symbol) → Task 3 blocks 1-6. Newly-exposed `!`-XOR
  → Task 1 (probe-mandated).
- **Placeholders:** none — every step has concrete code/commands/expected output.
- **Type consistency:** `str_env: HashMap<String,String>`, `resolve_str`, `BinOp::Xor`
  used identically across tasks.
- **Probe-first:** every semantic committed in the notes doc before code; every byte
  gated by an asl golden.
