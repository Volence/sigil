# .emp Diagnostics Construct (`assert` + `raise_error`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the `assert`/`raise_error` statement constructs (spec:
`docs/superpowers/specs/2026-07-11-emp-diagnostics-construct-design.md`),
then retrofit rings.emp + core.emp and close the bookkeeping.

**Architecture:** Grammar-level statement construct, `table`-precedent.
Parser adds two `AsmStmt` variants; the eval layer DESUGARS each into the
twin-parity expansion (a synthesized `Vec<AsmStmt>` of instructions,
labels, and `dc.b` data — the `lower/script.rs` synthesis pattern) and
evaluates it inline, exactly like the `AsmStmt::If` arm evaluates its
chosen branch. The FSTRING encoder is a pure Rust module with verbatim
byte vectors from the existing transliterations as ground truth.

**Tech Stack:** Rust (crates/sigil-frontend-emp), existing test harness
(`cargo test -p sigil-frontend-emp`, `cargo test -p sigil-cli`), aeon
dual-build byte gates for the retrofit.

**Read first:** the spec (above); `eval/asm.rs` (the `AsmStmt` match —
your desugar arm goes here); `parser.rs:1418-1523` (`asm_stmt()` — the
`trap!`/`let` arms are the pattern); `ast.rs:1187-1245` (`AsmStmt`);
`lower/script.rs:380-540` (synthesizing `AsmStmt::Instr`/`Label`);
`aeon/engine/debug/debugger.asm:85-130` (the format-param + `_eh_*`
equates — copy constants from THERE, not from memory);
`aeon/engine/objects/rings.emp:73-110` + `core.emp:277-337` (the
transliterations you must reproduce byte-for-byte).

**Branch:** `diag-construct` off master. Do NOT touch `port-tranche11`.

---

### Task 1: FSTRING encoder (pure module, ground-truth vectors)

**Files:**
- Create: `crates/sigil-frontend-emp/src/eval/diag.rs` (register `mod diag;` in `eval/mod.rs`)
- Test: same file, `#[cfg(test)]` module (house style — check a sibling like `eval/s4lz.rs` and match)

- [ ] **Step 1: Write the failing tests.** Ground truth is the rings/core
  transliterations, verbatim:

```rust
#[test]
fn control_tokens_encode() {
    // constants from debugger.asm's equates block — verify against source
    assert_eq!(encode_fstring("A%<endl>B%<pal2>C").unwrap().bytes,
               vec![b'A', 0xE0, b'B', 0xEC, b'C', 0x00]);
}

#[test]
fn rings_assert_message_vector() {
    // the exact dc.b run from rings.emp:104-106 (assert.b d4, eq, #0)
    let m = assert_message(Width::B, "d4", "eq", Some("#0"));
    let mut expect = Vec::new();
    expect.extend(b"Assertion failed:");
    expect.push(0xE0); expect.push(0xEC);
    expect.extend(b"> assert.b ");
    expect.push(0xE8); expect.extend(b"d4,");
    expect.push(0xEC); expect.extend(b"eq");
    expect.push(0xE8); expect.extend(b",#0");
    expect.push(0xE0); expect.push(0xEA);
    expect.extend(b"Got: ");
    expect.push(0x80); // %<.b src> descriptor: hex|width_bits(b=0)
    expect.push(0x00); // terminator
    assert_eq!(m, expect);
}

#[test]
fn core_long_message_vector() {
    // core.emp:297-299 — assert.l a0, hs, #Object_RAM → descriptor $83
    let m = assert_message(Width::L, "a0", "hs", Some("#Object_RAM"));
    assert_eq!(*m.last().unwrap(), 0x00);
    assert_eq!(m[m.len() - 2], 0x83); // hex($80) | long(3)
}

#[test]
fn exit_flag_parity_both_ways() {
    // rings: flag at ODD offset → $20|$80 + $00 pad; core: EVEN → bare $20
    assert_eq!(exit_flag_bytes(/*odd_offset=*/true),  vec![0xA0, 0x00]);
    assert_eq!(exit_flag_bytes(/*odd_offset=*/false), vec![0x20]);
}

#[test]
fn tst_form_message_omits_dest() {
    let m = assert_message(Width::W, "d1", "eq", None);
    // no ",dest" segment: "eq" is followed directly by $E0 $EA "Got: "
    let s = m.windows(2).position(|w| w == [0xE8, b',']);
    assert!(s.is_none());
}
```

- [ ] **Step 2: Run to verify failure** (`cargo test -p sigil-frontend-emp diag` → compile error, module missing).

- [ ] **Step 3: Implement.** Shape:

```rust
pub enum Width { B, W, L }           // width_bits: B=0, W=1, L=3

pub struct FStringArg { pub width: Width, pub operand_spelling: String, pub param: String }
pub struct EncodedFString { pub bytes: Vec<u8>, pub args: Vec<FStringArg> }

/// Tokens: literal text; %<endl|cr|pal0..3>; %<setw N>-class (control byte
/// + 1 param byte); %<.b|.w|.l operand [param]> → descriptor byte
/// param_base | width_bits, and the arg recorded (in order) for push-code
/// generation. Trailing $00 terminator. param defaults to hex.
pub fn encode_fstring(s: &str) -> Result<EncodedFString, String>;

/// The AS macro's auto-message template (spec §4.4), built from spellings;
/// includes descriptor + terminator, NOT the exit flag.
pub fn assert_message(w: Width, src: &str, cond: &str, dest: Option<&str>) -> Vec<u8>;

/// _eh_return ($20), |$80 + $00 pad iff landing at an odd offset (spec §4.5).
pub fn exit_flag_bytes(odd_offset: bool) -> Vec<u8>;
```

Copy the param-name → byte table (hex/dec/bin/sym/symdisp/str/signed/
split/forced/weak) and control-token table from debugger.asm's equates
(lines ~85-130) into a `const` table with a comment citing the source
lines. Enforce the macro's own `param >= $80` check as an `Err`.

- [ ] **Step 4: Run tests → PASS.**
- [ ] **Step 5: Commit** (`feat(diag): FSTRING encoder + assert auto-message, transliteration ground-truth vectors`).

---

### Task 2: AST variants + parser

**Files:**
- Modify: `crates/sigil-frontend-emp/src/ast.rs` (append to `AsmStmt`, after `If`)
- Modify: `crates/sigil-frontend-emp/src/parser.rs` (`asm_stmt()`, alongside the `trap!` arm at ~1447 and `let` at ~1493)
- Test: parser tests (find the existing parser-test home with `grep -rn "asm_stmt\|AsmStmt::Trap" crates/sigil-frontend-emp/tests/` and colocate)

- [ ] **Step 1: Failing parse tests** — `assert.b d4, eq, #0` /
  `assert.w d1, eq` (tst form) / `raise_error "X%<endl>Got: %<.b d0>"`
  each parse to the expected variant; negatives: missing width, unknown
  cond, `raise_error` with a second argument (consoleprogram) → parse
  errors whose messages name the fix (spec §5).

- [ ] **Step 2: Run → FAIL.**

- [ ] **Step 3: Implement.** AST:

```rust
/// `assert.<w> src, cond [, dest]` (spec §3) — self-gated on DEBUG.
Assert {
    width: Width,                    // re-export from eval::diag or mirror
    src: Operand, src_spelling: String,      // spelling = span slice, verbatim
    cond: String,                    // one of the 16 Bcc codes, lowercase
    dest: Option<(Operand, String)>, // operand + verbatim spelling
    span: Span,
},
/// `raise_error "<fstring>"` — unconditional (spec §4.1).
RaiseError { fstring: String, span: Span },
```

Parser: keyword-detect `assert` / `raise_error` at statement position
(same lookahead style as the `let` arm). Width suffix is REQUIRED for
`assert` (parse like `InstrLine`'s size suffix). Validate cond against
the 16-code set AT PARSE TIME (error lists all 16). Capture
`src_spelling`/dest spelling by slicing the source at the operand's span
(the parser owns the source text — find the existing span-slice helper
with `grep -n "fn src_slice\|span_text" parser.rs`; if none exists, add
one). `raise_error` takes exactly one string literal.

- [ ] **Step 4: Run → PASS.** Also `cargo test -p sigil-frontend-emp` (no regressions — `assert`/`raise_error` were previously undefined-mnemonic errors, nothing legal changes meaning).
- [ ] **Step 5: Commit** (`feat(diag): assert/raise_error grammar + AST`).

---

### Task 3: Desugar arms in eval/asm.rs

**Files:**
- Modify: `crates/sigil-frontend-emp/src/eval/asm.rs` (two new match arms, after `Trap`)
- Modify: `crates/sigil-frontend-emp/src/eval/diag.rs` (the expansion builder lives here, next to the encoder)
- Test: `crates/sigil-frontend-emp/tests/diag_construct.rs` (new — model on `tests/table_construct.rs`)

- [ ] **Step 1: Failing tests** (via the same harness `table_construct.rs`
  uses to evaluate a proc body and inspect emitted bytes):
  - `assert` with `DEBUG=0` (or undefined→error per spec §5) emits ZERO bytes.
  - `assert.b d4, eq, #0` with `DEBUG=1` emits the full rings expansion —
    assert equality against the literal byte sequence transcribed from the
    rings.emp DEBUG-shape reference (get it from the existing debug-shape
    pin/listing, or assemble the AS twin's site — do NOT hand-derive).
  - tst form emits `tst.<w>` not `cmp`.
  - hygienic labels: two asserts in one proc don't collide.
  - `raise_error` emits with NO DEBUG gate and NO cmp/branch/CCR wrapper.

- [ ] **Step 2: Run → FAIL.**

- [ ] **Step 3: Implement.** In `diag.rs`, a builder that returns the
  synthesized statements (spec §4.2 order, `lower/script.rs:380-540`
  construction pattern):

```rust
pub fn desugar_assert(a: &AssertParts, fresh: &mut impl FnMut(&str) -> String)
    -> Vec<AsmStmt>
{
    // 1. move.w sr, -(sp)
    // 2. cmp.<w> dest, src   |   tst.<w> src
    // 3. b<cond>.w .skip     (pinned .w — spec §4.2 step 3; synthesize the
    //                         InstrLine with size Some(Text("w")))
    // 4. .raise: pea .raise(pc)         (fresh label; self-address)
    // 5. move.w sr, -(sp)
    // 6. arg push per width: B → subq.w #2,sp + move.b src,1(sp)
    //                        W → move.w src,-(sp) ; L → move.l src,-(sp)
    // 7. jsr (MDDBG__ErrorHandler).l
    // 8. dc.b <assert_message bytes>    (one synthesized dc.b InstrLine;
    //     operands: Imm(int) per byte — or string+ints mixed like the
    //     hand-written transliteration; whichever the dc lowering accepts
    //     from synthesized AST, verify against lower/code.rs "dc")
    // 9. dc.b <exit_flag_bytes(parity)> (parity = offset of flag byte from
    //     expansion start; every instruction before it is fixed-size at
    //     desugar time EXCEPT operand-dependent cmp — compute from the
    //     encoded message length + the known even instruction sizes:
    //     instructions are always even, so parity == message_len % 2)
    // 10. jmp (MDDBG__ErrorHandler_PagesController).l
    // 11. .skip: move.w (sp)+, sr
}
```

  The parity insight in step 9 makes this deterministic BEFORE lowering:
  68k instructions are word-sized, so the flag's parity equals the parity
  of the message byte-run alone. Assert this in a debug_assert + unit test.

  In `eval/asm.rs`: the `Assert` arm checks `DEBUG` from the comptime env
  (same lookup the `If` arm's cond eval uses; undefined → the spec-§5
  error), and if 1, evaluates the desugared statements by recursing into
  the same statement loop (pattern: the `If` arm at asm.rs:41/289).
  `RaiseError` arm: no gate, desugar steps 4-10 with `encode_fstring`'s
  bytes and one arg-push per recorded arg in REVERSE order (spec §4.3);
  arg operands limited to registers/immediates (spec §5), else steering
  error. `Assert` src must be a register (`Operand` shape check), else
  the "move to a register first" error naming rings.emp precedent.

- [ ] **Step 4: Run → PASS**, full crate suite green.
- [ ] **Step 5: Commit** (`feat(diag): assert/raise_error desugar + DEBUG gating`).

---

### Task 4: Acceptance vector vs the AS twin (CLI-level)

**Files:**
- Create: `crates/sigil-cli/tests/diag_assert_vector.rs` (model on `tests/table_plc_vector.rs` — same fixture/harness style)

- [ ] **Step 1: Write the vector test**: a minimal fixture pair (AS source
  using debugger.asm's real `assert` macro; .emp source using the
  construct) assembled to identical bytes in the DEBUG shape, covering:
  `.b` no-dest, `.w` with-dest immediate-symbol, `.l` with-dest (the $20
  no-pad parity case), and one `raise_error` with a `%<.b dN>` arg.
  Follow `table_plc_vector.rs`'s mechanism for producing the AS side
  (whatever it does — pinned listing or live asl run — do the same).
- [ ] **Step 2: Run → FAIL (construct side empty), then PASS once wired.**
- [ ] **Step 3: Negative probes** as compile-error tests: memory src,
  unknown cond, unknown fstring token, consoleprogram arg, param < $80.
- [ ] **Step 4: Full workspace test run** (`cargo test --workspace`) green.
- [ ] **Step 5: Commit** (`test(diag): AS-twin acceptance vectors + negative probes`).

---

### Task 5: Retrofit rings.emp + core.emp (aeon, step-6 sweep)

**Files (aeon repo — coordinate branch with sigil merge):**
- Modify: `aeon/engine/objects/rings.emp:73-110` → the block becomes:

```
if DEBUG == 1 {
    // register comparand — the handler message can't take a parenthesised
    // memory operand (rings.emp precedent / AS error #1300)
    move.b  Ring_Add_Dropped, d4    // d4 = declared clobber
    assert.b d4, eq, #0             // drop = content bug, fatal in DEBUG
}
```

- Modify: `aeon/engine/objects/core.emp:277-337` — Debug_AssertObjLoop's
  three transliterations → `assert.l a0, hs, #Object_RAM` /
  `assert.l a0, lo, <spelled exactly as the twin's dest>` /
  `assert.w d7, lo, <ditto>` (KEEP OPERAND SPELLINGS IDENTICAL to the AS
  twin — the message embeds them; spec §4.4).

- [ ] **Step 1: Make the edits** (keep behavior comments; DELETE the
  transliteration-mechanics commentary — it describes the dead pattern).
- [ ] **Step 2: Byte gates**: build both shapes + mixed; DEBUG-shape gate
  must stay GREEN (byte-neutral retrofit — if it reddens, the construct's
  emission is wrong: fix the construct, never re-pin around it).
- [ ] **Step 3: Strict run + gate-off neutrality** per house rules
  (`cargo run -p sigil-harness` targets — same commands the tranche-10
  packet used; they're in the harness README/notes).
- [ ] **Step 4: Commit both repos** (lockstep message:
  `retrofit(diag): rings+core asserts → one-line construct (kill row 16)`).

---

### Task 6: Bookkeeping (same commit wave as Task 5)

**Files (sigil):**
- Modify: `docs/superpowers/notes/twin-scaffolding-kill-list.md` — row 16:
  mark KILLED (retrofit commit hash). ADD new row: construct's twin-parity
  emission mirrors debugger.asm token encoding + engine config (extensions
  on); kill = Spec 5 (twins die → message format + `b<cond>.w` pin freed).
- Modify: `docs/superpowers/notes/campaign-gap-ledger.md` — close the
  "assert/diagnostics demand 1/2" entry (ratified: 30 sites); add rows:
  Console/KDebug construct (demand 0), consoleprogram param (demand 0),
  memory-operand arg push (demand 0), comparison-operator assert sugar
  (post-Spec-5 taste).
- Modify: `docs/superpowers/notes/campaign-port-loop.md` — construct
  inventory: add `assert`/`raise_error` line under the step-4 cheat-sheet.

- [ ] **Step 1: Make the three edits; commit** (`docs(diag): kill row 16, ledger closures + demand-0 rows, inventory entry`).
- [ ] **Step 2: Checkpoint packet to Volence** (house format: per-pass
  findings buckets), then his gate → `--no-ff` merge both sides + push.

---

## Self-review notes (spec-coverage check, done at write time)
- Spec §3 surface → Tasks 2-3; §4.1 gating → Task 3; §4.2/4.3 expansions →
  Task 3; §4.4 message → Task 1; §4.5 parity → Tasks 1+3 (incl. the
  message-parity-only proof); §5 validation → Tasks 2-4; §6 map → Tasks
  1-3; §7 vectors → Tasks 4-5 (drift guard preserved by riding the debug
  gates); §8 bookkeeping → Task 6; §9 sequencing → branch note + Task 5.
- Constants deliberately sourced from debugger.asm equates, not this doc,
  except vector-confirmed ones ($E0/$E8/$EA/$EC, $80/$83, $20/$A0).
- Open implementation question left to the builder (both answers work):
  whether synthesized `dc.b` rows carry one Imm per byte or mixed
  string+ints — decided by what lower/code.rs's `dc` path accepts (Task 3
  step 3 note).
