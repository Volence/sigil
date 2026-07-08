# Spec 2 · Plan 7 #7-main — `bank:` sections + `bankid()` on link-expr cells: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. PREREQUISITE: the 7-pre plan (final-size placement) is COMPLETE on this branch.

**Goal:** The Z80-window banking invariant becomes a section property and checked builtins (design D7.2/D7.3/D7.5/D7.6): `section name (bank: $8000)` never straddles a 32KB boundary (bump-only-when-straddling placement + an always-on generated check), `bankid(Label)` yields the link-time `(Sym & $7F8000) >> 15` riding D2.23's LinkExpr machinery, and general link-expr DATA CELLS (ledger S2-D13f) are un-deferred as its carrier. Exhibit: aeon's `dac_samples.asm` shape ported to `examples/game`.

**Architecture:** `bank:` parses in `section_attrs` beside `cpu:`/`vma:`, threads to `ir::Section` as `bank: Option<u32>`, and is enforced in 7-pre's link-time placement pass at the marked seam (bump the chained base to the next N boundary iff the final extent would straddle; content > N is the §7.3 "over by K bytes" error; a post-placement check diagnoses ANY bank section that still straddles — pins included). `bankid()` is a new non-shadowable special call that returns `Value::LinkExpr` over the existing operator lifting; emitting any LinkExpr into a data cell lowers to a new `Cell::Expr { expr, width }` → a width-selected value fixup folded at link and range-checked on write. `winptr()` is untouched (byte-proven; L7.3 gates any re-expression).

**Tech Stack:** `sigil-frontend-emp` (lower/mod.rs attrs, eval/call.rs + builtins.rs, value.rs Cell, lower/data.rs), `sigil-ir` (Section.bank, fixup kind), `sigil-link` (bump + check + fixup apply), exhibit + fixtures in `examples/game/`. Tests: `crates/sigil-frontend-emp/tests/banks.rs` (new), CLI acceptance in `crates/sigil-cli/tests/dac_bank_acceptance.rs` (new, program-path per the #9 watch-out).

**Ratified basis:** design doc D7.1–D7.7 (APPROVED). Delegated calls frozen as R7m.1–R7m.8.

**Worktree/notes:** same as 7-pre (`plan7-item7-banks` branch; `docs/superpowers/notes/2026-07-08-item7-implementation-notes.md`).

---

## Verified code facts (recon 2026-07-08)

- `section_attrs` at lower/mod.rs:599–635 (cpu/vma precedent; unknown attrs → Error naming the attr).
- Builtin dispatch: non-shadowable special calls in eval/call.rs:54–89 (`ensure`/`byte`/`winptr` at :68/`here` at :72/`embed`/…); `bankid` joins this list, NOT `is_builtin()` (builtins.rs:509 — that set is receiver-methods only).
- LinkExpr creation + operator lifting: eval/expr.rs:28–31, :433–457, `lift_binary` :632, `lift_to_link_expr` :777–793; the `[here.provisional]` refusal text/mechanism :745–755.
- Guard deferral (`defer_guard`) guards.rs:160–187 — a `LinkExpr` condition already defers to `LinkAssert` automatically; `bankid()` in `ensure` costs nothing new.
- Cells: value.rs:170–218 (`Scalar`/`Bytes`/`SymRef`/`RelOffset` — NO expr-carrying cell yet). Fixup kind selection (D-P4.5) lower/data.rs:126–161; kinds fixup.rs:7–42; fixups applied + folded in link lib.rs:82–132 (targets are already `ir::Expr` folded against the final table).
- `winptr` builtins.rs:414–452 → `Cell::SymRef { width: 2, windowed: true }`; masks live in the linker's apply (BankPtr16Le lib.rs:267–279).
- `embed()` sandbox.rs:164–257 → `Value::Data` (one `Cell::Bytes`); `len` is dispatched at builtins.rs:88/:97/:223 — RECON FLAG: whether `.len` accepts a `Value::Data` receiver is UNVERIFIED; T4 step 1 resolves it (R7m.7).
- aeon reference (exhibit source): dac_samples.asm — `align $8000`, per-sample `(addr & $7F8000) >> 15` / `(addr & $7FFF) | $8000` / `End - Start` constants, straddle fatals `(a >> 15) <> ((end-1) >> 15)`, length guards `0 < len < $8000`; 9 fatal guards total across dac_samples/song_table/main.asm:231–241.

## Design rulings frozen by this plan (R7m.1–R7m.8)

- **R7m.1 — surface.** `section dac_bank (bank: $8000) { … }`. The attr value: comptime int, power of two, > 0 — else Error at the expr span naming the section and the offense (`section `dac_bank` `bank:` must be a positive power-of-two comptime integer`). Composes with `cpu:`/`vma:` in any order. Threads as `ir::Section { bank: Option<u32> }` (default None; relax.rs's :513–571 rebuild and every construction site preserve it — compiler-driven audit).
- **R7m.2 — enforcement is placement-constructive + check-total.** In the placement pass (7-pre seam): after `base` is chosen and `final = final_size(sec)` known — if `sec.bank == Some(n)` and `base / n != (base + final - 1) / n` then `base = base.next_multiple_of(n)` (Chained sections only; a Pinned straddling section is NOT moved — pins stay pins). If `final > n` → Error in the §7.3 budget style: `section `dac_bank` ($<final> bytes) cannot fit a $<n> bank — over by K bytes`. Post-placement, EVERY `bank:` section (pinned included) is checked `first_byte / n == last_byte / n`; failure → Error naming the section, its final `[start,end)` extent, and the boundary it crosses. D7.5's "generated link assertion" is DISCHARGED STRUCTURALLY in the linker (same diagnostic channel/level as `check_link_asserts` output) rather than via a synthesized `LinkAssert` row — no anchor-symbol pollution, the check is always-on and sees exactly the final addresses; record this reading in the notes (spec-review item).
- **R7m.3 — `bankid()`.** `bankid(<label ident or FnRef/Str, exactly the winptr argument contract>)` → `Value::LinkExpr(((Sym & 0x7F8000) >> 15))` — the residual tree built from `Expr::Binary`; the mask/shift constants appear ONLY here. All D2.23 consequences inherited: `ensure(bankid(A) == bankid(B), …)` defers to link; arithmetic composes via the existing lifting; comptime-required contexts refuse via the EXISTING refusal machinery with a bankid-steering message under the code **`[bank.provisional]`**: `bankid() is a link-time value; it cannot size or steer comptime evaluation — emit it into a data cell or guard it with ensure`. (Implementation: the refusal site takes the code+message from the value's provenance; smallest honest change wins — if threading provenance is invasive, `[here.provisional]` text gains a parenthetical naming bankid and a follow-up note records the debt. Message must steer; code choice is secondary. Spec review checks the message, not the plumbing.)
- **R7m.4 — the general link-expr data cell (S2-D13f un-deferred).** New `Cell::Expr { expr: sigil_ir::expr::Expr, width: u8 }`. A `Value::LinkExpr` landing in a data cell of declared width w ∈ {1, 2, 4} lowers to it (this REPLACES the D-H.3 arithmetic-then-emit refusal — that path now works; plain provisional `here()` keeps its existing SymRef lowering, byte-identical). Lowering → a fixup whose target is the carried expr and whose kind is width/CPU-selected: new kinds `Value8` (any CPU), `Value16Be`/`Value16Le`, `Value32Be`/`Value32Le` (endianness by section CPU: 68k=Be, Z80=Le) — VALUE kinds write the folded integer verbatim after an UNSIGNED-window range check (`0 ≤ v < 2^(8w)`; a signed value that folds negative is an error naming the cell's span, expr, and folded value). They are deliberately distinct from the ADDRESS kinds (Abs16Be range-checks as an address and BankPtr16Le masks — semantics we must not inherit). Width-1 is REQUIRED (aeon's `ds_bank` is one byte).
- **R7m.5 — `winptr()` untouched; one Z80 probe.** No churn to winptr (L7.3). One probe test discharges D7.7's verify clause: a `bankid()` byte cell inside a `(cpu: z80)` section folds and writes correctly (little-endian irrelevant at width 1; use width 2 to see Value16Le).
- **R7m.6 — exhibit.** `examples/game/data/dac_samples.emp` + tiny committed fixtures `examples/game/data/dac/{kick,snare,hat}.bin` (a few bytes each — synthetic, NOT aeon's PCM). One `(bank: $8000)` section holding three `embed` blobs; a 68k data table emitting per-sample `bankid()`/`winptr()`/length (the `SND_*_BANK/PTR/LEN` shape, 9-byte-descriptor-adjacent but exhibit-simplified); comptime `ensure` on each length (`0 < len < $8000` — lengths are comptime, guards stay comptime); the layout byte-argued in the acceptance test's comment against aeon's scheme (padding/alignment differences per D7.2 EXPECTED and documented — the exhibit argues equivalence of derived VALUES, not padding identity). Acceptance: new CLI test compiling with `--root examples/game --prelude prelude` pinning the full image. NEGATIVE probe (separate test, tmpdir source): a `(bank: $10)` section with >$10 bytes → "over by" error; and a two-section arrangement where a chained bank section WOULD straddle → assert it got bumped to the boundary (positive bump pin).
- **R7m.7 — embed length exposure.** The exhibit needs `embed(...).len` (or an equivalent comptime length). T4 first VERIFIES whether `.len` on `Value::Data` already works; if not, extend the `len` method to `Value::Data` receivers (cell byte-length sum) — smallest honest change, unit-tested, recorded in notes.
- **R7m.8 — docs + flags.** Ledger L7.1–L7.4 untouched. Empyrean spec integration (§7.x/D2.25) is EXPLICITLY NOT DONE here — flagged for Fable at the post-merge checkpoint. The plan's completion note lists it.

---

### Task 1: `bank:` attr parse + IR threading

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs:599–635`, `crates/sigil-ir/src/lib.rs` (Section), `crates/sigil-ir/src/builder.rs`
- Test: `crates/sigil-frontend-emp/tests/banks.rs` (new)

- [x] **Step 1:** Failing tests: (a) `section s (bank: $8000)` lowers to `Section.bank == Some(0x8000)`; (b) `bank: 3` → the R7m.1 error; (c) `bank: 0` → same; (d) unknown-attr diagnostics unchanged.
- [x] **Step 2:** Implement (attr eval mirrors `vma:`'s `eval_attr_int`; power-of-two check `n & (n-1) == 0 && n > 0`). `switch_section_lma` grows the bank parameter or a builder setter — compiler-driven audit of construction sites.
- [x] **Step 3:** Green + full gate. Commit: `feat(7): bank: section attribute → ir::Section.bank`.

### Task 2: Bump + no-straddle check in the placement pass

**Files:**
- Modify: `crates/sigil-link/src/relax.rs` (the 7-pre seam), `crates/sigil-link/src/lib.rs`
- Test: `crates/sigil-link/tests/final_placement.rs` (extend)

- [x] **Step 1:** Failing linker tests: (a) chained bank-$100 section at cursor $F8 with $10 bytes → placed at $100 (bumped); (b) same section at cursor $F8 with $8 bytes → stays $F8 (no straddle → NO bump — the not-aeon's-always-align point of D7.2); (c) content $110 > $100 → "over by $10 bytes" error; (d) a PINNED bank section pinned astride a boundary → the R7m.2 post-check error (not silently moved); (e) bump interacts with the fixpoint (a bump that changes a branch distance re-relaxes and converges).
- [x] **Step 2:** Implement at the seam per R7m.2.
- [x] **Step 3:** Green + full gate + corpus byte-diff (no bank: users in the shipped corpus → zero diffs). Commit: `feat(7): no-straddle bank placement — bump-only-when-straddling + always-on final check (D7.2/D7.5)`.

### Task 3: `Cell::Expr` + value fixup kinds (S2-D13f)

**Files:**
- Modify: `crates/sigil-frontend-emp/src/value.rs:170–218`, `crates/sigil-frontend-emp/src/lower/data.rs:56–161`, `crates/sigil-ir/src/fixup.rs`, `crates/sigil-link/src/lib.rs` (apply arm)
- Test: `crates/sigil-frontend-emp/tests/banks.rs` + a linker fold/range test

- [x] **Step 1:** Failing tests: (a) a LinkExpr emitted at width 2 in a 68k section produces `Value16Be` bytes of the folded value; (b) width 1 works; (c) fold overflowing the width → error naming cell + value; (d) Z80 section width 2 → little-endian (the R7m.5 probe rides this); (e) the OLD `[here.provisional]` arithmetic-then-emit refusal case (here-fix acceptance #5) now EMITS correctly instead — update that pinned test deliberately and note it (design-sanctioned un-deferral, D7.3 verbatim).
- [x] **Step 2:** Implement: Cell variant, D-P4.5 table extension, fixup kinds + apply arms with unsigned range check.
- [x] **Step 3:** Green + full gate. Commit: `feat(7): general link-expr data cells — Cell::Expr + ValueN fixup kinds (S2-D13f un-deferred)`.

### Task 4: `bankid()` builtin (+ embed `.len` verification)

**Files:**
- Modify: `crates/sigil-frontend-emp/src/eval/call.rs:54–89`, `crates/sigil-frontend-emp/src/eval/builtins.rs`
- Test: `crates/sigil-frontend-emp/tests/banks.rs`

- [x] **Step 1:** RECON step (R7m.7): a test `embed("f.bin").len` — if it already passes, pin it; if not, extend `len` to `Value::Data` first (own failing-test/implement/commit micro-cycle).
- [x] **Step 2:** Failing tests for `bankid`: (a) `bankid(Label)` in a width-1 cell emits the folded `(addr & $7F8000) >> 15`; (b) `ensure(bankid(A) == bankid(B), …)` with A/B in different banks fails AT LINK with the message; (c) same-bank passes silently; (d) `bankid` in a comptime-required position (array length) → the R7m.3 refusal message; (e) argument-form errors mirror winptr's.
- [x] **Step 3:** Implement per R7m.3 (non-shadowable list + an `eval_bankid` beside `eval_winptr`).
- [x] **Step 4:** Green + full gate. Commit: `feat(7): bankid() — link-time bank id over LinkExpr (D7.3)`.

### Task 5: The dac_samples exhibit + acceptance

**Files:**
- Create: `examples/game/data/dac_samples.emp`, `examples/game/data/dac/*.bin`, `crates/sigil-cli/tests/dac_bank_acceptance.rs`

- [x] **Step 1:** Author fixtures + exhibit per R7m.6. Hand-derive the full expected image in the test (aeon-scheme equivalence argued in comments: each SND_* value cross-computed from the fixture addresses).
- [x] **Step 2:** Failing acceptance test (exhibit not yet written correctly ≡ RED first), then make it pass. (RED evidence: bank at `vma: $0000` → bankid folds to 0, byte-diff at 0xF.)
- [x] **Step 3:** Negative straddle probe + positive bump pin (R7m.6, tmpdir). (Bump pin proved load-bearing: `bank: $40` fits → no bump; `bank: $10` straddles → bumped.)
- [x] **Step 4:** Standing pins re-verified: pitcher_plant 340B + script 358B untouched. Full gate + corpus byte-diff. Commit: `feat(7): dac_samples exhibit — bank section + bankid/winptr table + straddle probes (D7.6)`.

### Task 6: Whole-branch adversarial review + checkpoint prep

- [ ] **Step 1:** Full gate + corpus byte-diff + harness, recorded.
- [ ] **Step 2:** Two-stage reviews on T2/T3/T4 (load-bearing); whole-BRANCH adversarial review (7-pre + 7-main together) with byte-diff probes vs master; controller independently verifies.
- [ ] **Step 3:** Update `docs/superpowers/notes/` with the completion checkpoint note (open flags: empyrean spec integration → Fable; L7.1 gate at sound migration). NO MERGE without the Volence checkpoint.

## Self-review notes (plan author)

- Spec coverage: D7.2 (T1 surface, T2 bump/over-by, composition covered by T2e+T5), D7.3 (T3 carrier + T4 builtin + refusal, winptr untouched = R7m.5), D7.5 (T2 always-on check; structural-discharge reading recorded for spec review), D7.6 (T5 exhibit + negative probe + byte-identity via corpus probe), D7.7 (Z80 probe = T3d/R7m.5; nothing else built).
- Known deliberate pin update: the here-fix acceptance case (5) "arithmetic-then-emit" flips from refusal to emission (T3e) — design-sanctioned; itemize in notes and the review brief.
- Type consistency: `Cell::Expr{expr,width}` (T3) is what T4's bankid emission and T5's exhibit consume; `Section.bank: Option<u32>` (T1) is what T2 reads.
