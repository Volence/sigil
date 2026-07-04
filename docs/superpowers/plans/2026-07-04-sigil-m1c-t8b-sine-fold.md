# Sigil M1.C — T8b: sin/int Float Builtins + deform_table_sine Fold Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. TDD, gated on the
> 4 committed sine goldens + real asl. CI runs `clippy --workspace --all-targets -- -D warnings`.

**Goal:** Assemble `deform_table_sine` byte-exact vs asl — the flagship AS float-folding path.
Three pieces: (1) `#` **modulo** operator (Spike-0 MISS: the macro guard `(256 # PERIOD) <> 0`
uses it; Spike 0's grep missed `engine/parallax_macros.inc`); (2) `sin(expr)`/`int(expr)` **f64
builtins**; (3) the full macro expansion. **De-risked:** Spike 0 already proved libm `sin` +
`int`=**floor** bit-matches all 4 golden 256-byte tables — no source-cure needed.

## The exact macro (`engine/parallax_macros.inc:211`)
```
deform_table_sine macro AMPLITUDE,PERIOD
    if "AMPLITUDE" = ""            ; string compare — T2 DONE
        fatal ...
    endif
    if "PERIOD" = ""
        fatal ...
    endif
    if (256 # PERIOD) <> 0         ; `#` = MODULO (NEW) + `<>` (T2 done)
        fatal "...must divide 256"
    endif
deform_sine_i set 0               ; set accumulator — T8 DONE
    rept 256                      ; rept — M0 DONE
        dc.b int(AMPLITUDE * sin(6.283185307179586 * deform_sine_i / PERIOD))  ; sin/int f64 — NEW
deform_sine_i set deform_sine_i + 1
    endr
    endm
```
Invoked with keyword args (T7 DONE): `deform_table_sine AMPLITUDE=96, PERIOD=64` etc.

## Design constraints
- **§7.4:** `sin`/`int` are **front-end** builtins — they must NOT become `sigil_ir::Expr`
  nodes. Evaluate them to an `i64` constant inside the front-end expression evaluator, before/at
  fold time. (`#` modulo, by contrast, is a generic op → `BinOp::Mod` in `sigil-ir` is fine, the
  front-end maps the AS `#` syntax onto it.)
- **`#` is dual-use:** operand-level immediate prefix (`#5`, `Punct::Hash`, consumed in
  `operands.rs::classify` before expr parsing) AND infix modulo inside expressions
  (`256 # PERIOD`). Adding `#`→`Mod` to `expr.rs::infix_bp` is safe — the immediate `#` is
  consumed before the expr parser runs. Verify existing `#imm` snippets stay green.
- **f64 fold:** `int(AMP*sin(2π·i/PER))` — evaluate the argument tree in f64 (Int→f64, `*`/`/`
  in f64, `sin`→`f64::sin`, `6.28…` float literal), then `int()`=`floor` → `i64`. Confirm f64
  `sin` bit-matches (Spike 0 says it does; the golden gate proves it). Only `int()`/`sin()` and
  a float literal need f64 — everything else stays i64.

## Files
- `crates/sigil-ir/src/expr.rs` — add `BinOp::Mod` (rem_euclid or `%`? — probe asl: `256 # 64`,
  and a NEGATIVE case like `(-5) # 3` to pin the sign convention; likely truncating `%`).
- `crates/sigil-frontend-as/src/expr.rs` — map `#`→`BinOp::Mod` in `infix_bp`; parse `sin(...)`/
  `int(...)` call syntax (front-end only) + a float-literal token if needed.
- `crates/sigil-frontend-as/src/eval.rs` — f64 evaluator for `int(...)`/`sin(...)` expressions
  producing an i64; wired into the `dc.b` operand fold path.
- `crates/sigil-frontend-as/tests/` — a golden-comparison test.

## Steps (TDD)
- [ ] **Step 1 — `#` modulo, standalone gate.** Probe asl for `dc.b 256#64, 100#7, (-5)#3`
  (sign convention), add asl snippets, then implement `BinOp::Mod` + `#`→Mod. asl-gated.
- [ ] **Step 2 — sin/int golden test.** Add a test (`deform_table_sine.rs`) that assembles, for
  each `(AMP,PER)` ∈ {(96,64),(20,64),(16,64),(8,32)}, a source that defines the macro (inline
  the body or `include` it) and instantiates it once, then compares the 256 emitted bytes to
  `tests/vectors/sine_goldens/<name>.bin`. Watch it FAIL (sin/int unimplemented).
- [ ] **Step 3 — implement sin/int f64 eval.** Parse `sin(e)`/`int(e)`; evaluate in f64
  (`f64::sin`, `int`=`f64::floor as i64`), keeping them front-end-only. Make the golden test PASS
  for all 4 tables (byte-for-byte).
- [ ] **Step 4 — full-macro asl gate.** Add an asl_snippet that inlines the *entire*
  `deform_table_sine` macro + one instantiation (e.g. shimmer A=8/P=32, 256 bytes) and regen via
  `gen_snippet_vectors` — proves front-end == asl == golden through the guards + rept + set +
  fold together. (If the macro is easier to `include`, confirm the snippet harness supports it;
  else inline.)
- [ ] **Step 5 — suite green.** `cargo test --workspace` PASS (incl. the 4-golden test + the
  asl snippet); existing `#imm` and expr tests unregressed.
- [ ] **Step 6 — `clippy --workspace --all-targets -- -D warnings` + build clean.**
- [ ] **Step 7 — commit** `feat(sigil-frontend-as): sin/int f64 builtins + # modulo + deform_table_sine byte-exact vs goldens`.

## Self-Review
- Spec coverage: `#` modulo, sin/int f64, full deform_table_sine — gated on ROM-extracted goldens
  AND real asl. Records the Spike-0 `#` correction.
- §7.4: sin/int stay in the front-end (never IR nodes); only `BinOp::Mod` (generic) enters IR.
- Escalate if: f64 `sin` does NOT bit-match a golden (→ this is the §12 R7 source-cure trigger —
  STOP and report which table/index diverges; do not hack per-value), or `#` modulo's asl sign
  convention is surprising.
