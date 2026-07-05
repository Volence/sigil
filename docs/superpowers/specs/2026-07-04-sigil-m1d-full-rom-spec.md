# Sigil M1.D — Full-ROM Byte-Exactness — Design (audit-revised)

**Status:** approved (independent audit revision 2026-07-04; supersedes the ordering in
`2026-07-04-sigil-m1d-full-rom-handoff.md` — that doc remains valid as the M1.C landing
record)
**Milestone:** M1.D (follows M1.C, merged `aadf98b`)
**Crates:** `sigil-frontend-as`, `sigil-link`, `sigil-ir`, `sigil-harness` (+ doc backports
to `empyrean/docs/SIGIL_CORE_SPEC.md`)
**Reference toolchain:** `asl 1.42 Beta Build 212` → `p2bin` → `convsym` (no-op) → `fixheader`
**Reference source pin:** re-pin aeon at M1.D start (T0.0); the handoff-era pin was `9bacc93`

---

## 0. What changed since the handoff — the 2026-07-04 audit

Four independent review passes (architecture-vs-spec, phase/dephase semantics vs live asl,
linker width-fold soundness, golden-vector provenance) ran against merged M1.C. Confirmed
sound: crate quarantine + one-way graph (enforced by `crate_graph.rs`), the oracle
discipline (all 368 committed goldens regenerate byte-identical from real asl — non-circular
proven; fresh 24-snippet differential vs live asl scored 23/24), the phase/dephase
continuous-physical model (14 live asl probes, all match), and the M1.B width rule
(`asl_width_rule` boundary-swept, `-A` proven irrelevant).

The audit **upgraded two known items and found two new ones**. These reshape M1.D:

- **F1 (NEW, byte-affecting): `padding` is wrong across `save`/`cpu`/`restore`.**
  Live-asl probes show `restore` re-applies the saved CPU, and a CPU *switch* resets
  `padding` to the new CPU's default (ON for 68k): `padding off; save; cpu z80; …;
  restore` ends with padding **ON** (probe t14), while `save; restore` with no CPU change
  preserves it (t12); `cpu z80; cpu 68000` alone resets it (t13). Aeon's only save/restore
  pattern (boot.asm's Z80 load blocks, ~`boot.asm:280`) changes CPU — so in the **real**
  reference build, everything after boot.asm assembles with padding **ON**, despite
  `padding off` at `main.asm:3`. Sigil preserves OFF. Additionally sigil implements
  padding only as struct `ds.w/ds.l` offset rounding (`eval.rs:1413`), not asl's observed
  pad-byte-before-word-emission-at-odd-PC (t9/t10). Currently latent (no tested region
  emits a word at odd PC), i.e. a first-diff landmine. The `state.rs` header comment
  claiming this exact behavior was "probe-verified" is **contradicted by the probes** —
  see the probe-first process rule (§4).
- **F2 (UPGRADED risk→certainty): stale-fold under jmp/jsr width growth.** The object
  bank (`org $10000`) contains **11 bare-symbol `jmp/jsr` sites that grow abs.w→abs.l**
  (asl's own listing shows `4EF9` at ROM `$1012E`, `$1041E`, `$10486`, …; +22 bytes
  total). The front-end advances the cursor by a fixed 4 per site (`eval.rs:2082`) and
  folds nearly every label reference at those **baseline** values; `resolve_layout` shifts
  labels/fixups but not already-folded bytes. Two guaranteed failures once emit runs:
  stale folded pointer tables inside the bank, and — **unflagged by the handoff** — stale
  **downstream section LMAs** (`phys_base` accumulates baseline lengths, so the Z80 driver
  lands 22 bytes early; `resolve_layout` never re-flows section LMAs, `relax.rs:284-291`).
  Today the `Org+JmpJsrSym` guard (`relax.rs:158-181`) fails loud on that exact section —
  a tripwire, not a fix. Fix direction is settled: **move width selection into the
  front-end pass loop** (T3).
- **F3 (NEW, small): `cmpm` is encodable (`sigil-isa`) but missing from the front-end
  mnemonic table** — hard diagnostic, debug-build-only exposure
  (`engine/debug/compression_selftest.asm:83` under `ifdef __DEBUG__`). Blocks A2.
- **F4 (confirmed): the empyrean spec is stale on ≥4 asl-verified points** (comparisons
  fold 0/1 not 0/−1; `int()` = floor; the `strstr` last-char bug does not exist in asl
  1.42; `save/restore` state set) and describes never-built architecture
  (`ProvenanceStack`, `RelaxableFragment`/`ChosenSizes`, full `Diagnostic`, full
  `IrStreamer`, frontend→backend edge prohibition). A future agent grounding on it will
  mis-build — this failure mode has already occurred once in project history (see
  `SIGIL_PLAN_AUDIT_2026-07-01.md` F3). Backport is now in-milestone scope (T7).

Recon reality check (re-run 2026-07-04): the raw diagnostic count is ~2.0 M, but it
reduces to **one root cause** (string-valued `set` / `__FSTRING`, fanned out by a
non-convergent `while` scan loop) + **6 deferred EA sites over 3 distinct symbols**
(`OJZ_Act1_Descriptor` ×4, `BGND_Palette`, `OJZ_Palette`) + 2 one-offs (`… directive
expects …`, `unresolved long expression`). The handoff's "~19" counted distinct sites.

---

## 1. Goal & exit criteria

| Gate | Criterion |
|---|---|
| **A1** | `sha256(sigil_s4.bin) == sha256(ref_s4.bin)` for the **non-debug** build, full pipeline (assemble → resolve_layout → link → emit_rom), reference freshly built by `aeon/build.sh` at the T0.0 pin |
| **A2** | Same for the **`__DEBUG__`** build (requires F3 fix + a deliberately-built debug reference ROM) |
| **A3** | The ~42-symbol M0 stub table is **deleted**; the full build defines everything itself (the recon already runs with zero stubs) |
| **A4** | `SIGIL_CORE_SPEC.md` reconciled: 4 verified corrections backported, architecture supersessions recorded (T7) |

Everything is measured against **fresh** asl output at a pinned aeon commit; never
against stale on-disk artifacts (golden-provenance rule, `golden/PROVENANCE.md`).

---

## 2. Task breakdown (dependency order)

### T0 — Groundwork (independent of the assembly blocker; start immediately)

**T0.0 — Re-pin the reference. ✅ DONE (2026-07-04).** Pinned clean `9bacc93`:
len **450878**, sha256 `605631da…ad5117`, `0x18E`=**`0xcfc3`**. **Key finding:** the
M1.B pin (458666/`0x5CBE`) was captured with a *dirty* aeon tree — the build is
non-hermetic (python generators consume editor JSON), so a clean `9bacc93` differs.
Stashed aeon's forest_bg/editor WIP to get a reproducible pin. `regen` byte-identical
(region A 5896 B, B 1543 B). Recorded in `PROVENANCE.md`. Recon re-run against the
clean tree: same structure (2M → T1 string-set root cause + 6 T2 EA sites + 3 one-offs).

**T0.1 — `padding` fidelity (fixes F1). ✅ DONE (2026-07-04).** Probe-first: 17
live-asl probes (matrix in `docs/superpowers/notes/2026-07-04-m1d-t0.1-padding-probes.md`)
established **hypothesis A** (fits every row): (1) the `cpu X` directive resets
padding/supmode to CPU defaults **unconditionally, even same-CPU** (probe d: `padding
off; cpu 68000`→ON); (2) `save` snapshots only the CPU; `restore` re-applies it and
resets padding/supmode to default **only on an actual CPU change** (t14→ON, t12→OFF) and
**never restores the saved padding value** (probes b/c decisive). Also verified padding
aligns on the **logical `$`** (physical+disp), not physical (phase_logodd/logeven probes),
and pads before `dc.w`/`dc.l`/**instructions** at odd `$`. Implemented: `state.rs` `set_cpu`
(unconditional reset) + change-conditional `restore` + rewritten header/tests; `eval.rs`
`directive_cpu`→`set_cpu`, new `pad_word_align` helper wired into dc.w/dc.l/`lower_m68k`.
Fixed the "probe-verified" lie in the state.rs header. 7 byte-affecting snippet goldens
(regenerated from real asl: pad-on/off word/long/instr at odd PC, t14/t12/d state cases).
**All strict gates still green** (regen A+B byte-identical, m1b_gate 5, m1c_vector_table 1
— the padding-ON change is post-boot.asm, so those padding-off regions are unaffected).
The old probe-matrix items:
1. **Probe matrix against live asl** (the audit's t9–t14 probes live in a session
   scratchpad and must be re-created as committed artifacts): {padding on/off} ×
   {save/restore with and without intervening `cpu` change} × {explicit `padding`
   between save and restore} × {supmode, for completeness}; plus emission probes: `dc.w`
   / `dc.l` / `ds.w` / a 68k instruction, each at odd PC, padding on and off; struct
   `ds.w` offset rounding both ways (already covered by T7-era goldens — keep them
   green). Record results in a notes doc; encode the byte-affecting cases as
   `gen_snippet_vectors` snippet goldens so they regenerate from real asl forever.
2. **Implement to match.** Expected model (verify, don't trust): `restore` re-applies the
   saved CPU; a CPU **switch** (and only a switch) resets `padding`/`supmode` to the new
   CPU's defaults; `restore` without a CPU change leaves padding as-is. Plus real
   pad-byte-before-word/long-emission-at-odd-PC when padding is ON (this is what the
   post-boot.asm reference build actually does).
3. **Fix the `state.rs` header comment** to state the verified semantics and point at the
   committed probes.

Acceptance: new snippet goldens green; all existing strict gates still green
(`m0_acceptance`, `m1b_gate`, `m1c_vector_table` — these currently pass *with* the wrong
model, so any regression here means the new model was implemented where it shouldn't be).

**T0.2 — Commit the stale-fold reproducer (fixes nothing; pins F2). ✅ DONE
(2026-07-04).** `crates/sigil-frontend-as/tests/stale_fold_repro.rs`. Used the
**guard-free variant** (`phase $10000` sets the base via displacement, emits no `Org`
fragment → labels resolve `>$8000` → the bare `jmp` grows abs.w→abs.l, without tripping
resolve_layout's `Org+JmpJsrSym` guard) — isolates the defect cleanly. Two
`#[ignore = "flips green in T3"]` tests assert asl-1.42-verified correct output:
`dc_l_after_grown_jmp_folds_correctly` (single section, flatten = `4EF9 0001 0006 0001
0006` — the folded `dc.l` must be `$10006` not the stale baseline `$10004`) and
`downstream_section_lma_reflows_after_growth` (two sections; `SecondSection` LMA must be
`$0A`, not the stale `$08` — **this pins the half the handoff did not flag**). Companion
`*_documents_current_bug` tests pass today (tripwires; T3 deletes them). Verified: default
run 2 pass / 2 ignored; `--ignored` run the 2 correct-behavior tests FAIL (`...04` vs
`...06`; lma 8 vs 10). Both asl-truths captured live.

**T0.3 — Repair the M0 live harness. ✅ DONE (2026-07-04).** Added `padding off` to
`harness_root.asm` (after `cpu 68000`, mirroring aeon `main.asm:3`) — cleared the
`DacSample struct is 10 bytes, expected 9` regen abort. `regen` now exits 0 and
byte-matches (region A 5896 B, B 1543 B); goldens refreshed. Both live tests
(`harness_assembles_regions_a_and_b_together`, `assemble_reference_regions_returns_both_sections`)
pass with `--ignored` (the `#[ignore]` stays — it gates on the live aeon tree, like
the strict gates). **T0.1 interaction still open:** once padding-on/pad-byte semantics
land, re-verify the harness reproduces aeon's *effective* padding at each region.

**T0.4 — `cmpm` in the front-end mnemonic table (fixes F3). ✅ DONE (2026-07-04).**
Root cause was even smaller than expected: `M68kMnemonic` is a *type alias* for the
isa `Mnemonic`, so `Cmpm` already existed and the `(Ay)+,(Ax)+` operand shape already
lowered (shared with `addx`). One line — `"cmpm" => Cmpm` in the mnemonic table — plus
two asl-generated goldens: `cmpm.w (a0)+,(a1)+`→`B3 48` (aeon's real form,
compression_selftest.asm:83) and `cmpm.b (a3)+,(a2)+`→`B5 0B` (matches the spec's
predicted `B5 0B`). Regen churned only the 2 new blocks (non-circularity intact).
Unblocks A2.

### T1 — String-valued `set` symbols + `__FSTRING` (the assembly blocker). ✅ DONE (2026-07-04).

**Result:** recon **2,000,702 → 7 diagnostics** — the `strstr` (2,000,198) /
`strlen` (99) / `while`-non-convergent (99) / `trailing tokens` (297) /
`per-pass budget` (1) classes all dropped to **0**, and the 2 prior one-offs
(`directive expects …`, `unresolved long expression`) cleared as downstream of
the string failures. Remaining 7 = the 6 T2 EA sites + **1 newly-exposed**
(bucketed below).

**Design decision (resolves the spec's "Extend `SymbolValue` to `Int | Str`"):**
strings do **NOT** enter `sigil_ir::SymbolValue` (it stays `Int | Poison`,
`#[derive(Copy)]` — structurally can't hold a `String`). Per §7.4, string-valued
`set` symbols live in a new **front-end-local** `str_env: HashMap<String,String>`
on `Asm`, keyed by qualified name exactly like `env`. `directive_set` tries
`eval_str` first (literal / `substr` / `lowstring` / string-symbol copy) →
`str_env`; else the numeric `eval_all` → `env`. `eval_str` gained a lone-ident
branch (`resolve_str`, mirroring `SymbolTable::resolve`) so builtins resolve a
string **symbol**, not just a literal. The `while` converges for free once
`strstr(.__str,"%<")` returns -1. NOT carried across passes (asl `set` is a
sequential per-pass assignment; every `__FSTRING` symbol is assigned before read
— probe p1/p4). Invariant documented at `directive_set`: a symbol is int XOR
string per pass; type-flipping `set` is unsupported (un-probed → not enforced).

**Newly-exposed items (probe-first, both handled during T1):**
1. **Infix `!` is XOR, not bitwise-OR** — decisive on the `__ErrorMessage`
   `.__align_flag: set (((*)&1)!1)*$80` emit path. Probed (`1!1`=0, `3!1`=2,
   `5!3`=6; OR and XOR agree only on the one prior golden `3!4`=7). Fixed:
   `BinOp::Xor` + `Bang => BinOp::Xor`. (Commit `c09752e`.)
2. **`capture_macro` colon-label param-shift** (latent, pre-existing) — the
   colon-form head `NAME: macro p` left the `macro` keyword as a phantom first
   param, shifting every real param by one. The real `__FSTRING`/`__ErrorMessage`
   macros use the colon form, so this had to be fixed for T1. Peeled via the
   existing `parse_line_tokens`; non-colon form unchanged. `m1c_vector_table`
   (real `main.asm` macro tree) still green → no regression. (Commit `498466b`.)

**Bucketed for recon-0 / T2 (do NOT re-discover):** the 1 remaining
newly-exposed diagnostic is `` `END` is not a recognized 68000 mnemonic `` —
AS's `end` directive (uppercase, at the end of the source tree), previously
masked by the string failures. Needs a no-op / entry-point directive handler.
Belongs with T2 (both must clear to reach recon-0).

**Probes committed:** `docs/superpowers/notes/2026-07-04-m1d-t1-string-set-probes.md`
(6 string-symbol cases + the `!`-XOR matrix + the full `__ErrorMessage` reference
bytes `4EB9 00000400 / "BUS ERROR" / 00 / A1 / 00 / 4EF9 00000500`). **Goldens:**
8 new asl-verified snippet blocks (`t1_*`); `gen_snippet_vectors` churned only
those 8 (non-circularity intact). **Plan:**
`docs/superpowers/plans/2026-07-04-sigil-m1d-t1-string-set.md`. **Commits:**
`c09752e` (XOR) → `cd82435` (str_env) → `498466b` (capture_macro) → `9816e53`
(goldens) → `b26972d` (review-fix docs). Two-stage review: spec ✅, code
quality ✅ (ship-ready). All strict gates green (m1b_gate 5, m1c_vector_table 1,
harness 16, workspace, clippy `-D warnings`).

---

**Original scope (for reference).** `error_handler.asm`'s
`__ErrorMessage` macros are not `__DEBUG__`-guarded, so the **non-debug** ROM runs
`__FSTRING_GenerateArgumentsCode`, which stores strings in symbols (`.__str: set "…"`)
and scans them with `substr`/`strstr`/switch-on-string (`debugger.asm:647-659`,
`error_handler.asm:31-65`). Extend `SymbolValue` to `Int | Str`, thread string values
through `set`/expression evaluation/the existing string builtins, and make the scan
`while` loop converge.

§7.4 contamination safeguard still applies: strings live in the front-end evaluator
only; nothing string-typed enters `sigil-ir`.

Probe-first: each builtin's edge cases (empty string, `substr` len 0 = to-end, `strstr`
miss = ?, `val` on a string-valued symbol, comparison of string symbols) against live
asl, encoded as snippet goldens where byte-affecting.

Acceptance: the recon (`m1c_full`) drops the `strstr`/`strlen`/`while`/`trailing tokens`
classes to **0**; remaining diagnostics ≤ the 6 known EA sites + anything newly exposed
(bucket and record them); the emitted `__ErrorMessage` bytes for a representative macro
invocation match asl (snippet golden).

### T2 — The 6 deferred EA sites (3 symbols) + `END`. ✅ DONE (2026-07-04).

**Result:** recon **7 → 0 diagnostics** — `m1c_full` now reports `ASSEMBLED OK: 8
sections`. This arms the `m1c_rom` full-ROM emit path for the first time (T4).
Commits: `e24ec2a` (AbsWidth→sigil-ir) → `be40b65` (END no-op) → `86e711a`
(abs-EA width-select, spec✅ + code-quality✅ two-stage review; polish applied:
scoped "shrink-only" doc claim + `@`-binding to drop a clone) → `4a53538`
(recon-example fix, below). All strict gates green (m1b_gate 5, m1c_vector_table
1, harness 16; workspace 47 suites 0-failed; clippy `-D warnings` clean).

**Design executed as settled below.** `asl_width_rule` + `AbsWidth` relocated
to `sigil-ir` (single source of truth; front-end can't depend on `sigil-link`);
`sigil-link` re-exports (its M1.B boundary-sweep tests untouched). Bare-symbol/
expression absolute EA lowers via a new `abs_ea_from_expr` helper
(`eval.rs`): qualify → `self.fold` (NOT `fold_imm` — that returns 0 on Poison →
optimistic abs.w) → `asl_width_rule` → `AbsW`/`AbsL`; unresolved-this-pass →
pessimistic `AbsL(0)` + `poison_refs` (converged pass still errors on genuinely-
undefined). 5 `t2_*` goldens (abs.w/abs.l/$FF8000-boundary for `lea` + EA-general
`move.w`) + the `END` no-op golden; `gen_snippet_vectors` churned only those
(non-circularity intact). One pre-existing unit test that asserted the removed
reject was updated to assert the width-selected bytes.

**Newly-exposed (bucketed + handled, `4a53538`):** with assembly now succeeding,
`m1c_full`'s success branch ran for the first time and hit an `unreachable!` in
`Section::image_len()` on unlowered `JmpJsrSym` fragments (bare jmp/jsr, only
lowered by `resolve_layout`). Fixed by dropping the pre-resolve `image_len`
print from the recon (a diagnostics collector); the full assemble→resolve_layout
→link→emit path is T4's `m1c_rom` gate. **No new front-end diagnostics** — the
recon is genuinely at 0.

**Carry to T3/T4:** the folded absolute-EA *values* for the 6 real sites (all in
`test/ojz_scroll_test.asm`, targeting high-address level data → abs.l) may be
stale by the object-bank +22 (F2) until T3 re-flows section LMAs — a byte-*value*
concern, not width (width is stable abs.l). This is the predicted T4 first-diff
territory alongside F1 padding.

Probes committed: `docs/superpowers/notes/2026-07-04-m1d-t2-abs-ea-end-probes.md`.

**Located (2026-07-04):** all 6 EA sites are bare-symbol absolute source EAs
(`lea Sym, a0`) in `test/ojz_scroll_test.asm` (included by `main.asm:415`):
`OJZ_Act1_Descriptor` ×4 (`:36,:47,:98,:117`), `BGND_Palette` (`:20`),
`OJZ_Palette` (`:27`). `END` is the bare directive at `main.asm:446`.

**Design decision (settled; supersedes "deferred to T5b"):** a bare-symbol
absolute EA is a **width-variable instruction** — asl width-selects abs.w/abs.l
via the *same* pinned `asl_width_rule` as jmp/jsr (probe-verified: `lea $100`→
`41F8` abs.w, `lea $10000`→`41F9` abs.l, boundaries `$7FFF`/`$8000`/`$FF8000`
exact; EA-general, not lea-specific — `move.w Sym,d0` too). Lower it by
**folding + width-selecting in the front-end now** (not a deferred resolve_layout
fragment): `convert_one_atom_m68k` folds the address from the current-pass env,
picks the width via `asl_width_rule`, emits `M68kOperand::AbsW`/`AbsL`;
unresolved-this-pass → pessimistic abs.l (asl's forward-symbol guess; keeps the
fixpoint shrink-only). This reuses the one pinned rule and is the T3 front-end
width-selection mechanism applied to the absolute-EA class — a stepping-stone,
not throwaway. The 6 real sites all target high-address level data → abs.l, no
per-pass flip; byte-exact **values** (F2 stale-address) stay a T3/T4 matter.
**Shared rule:** relocate `asl_width_rule`+`AbsWidth` from `sigil-link` to
`sigil-ir` (the front-end cannot depend on `sigil-link` — one-way graph);
`sigil-link` re-exports so its code + M1.B boundary-sweep stay green.

**`END`:** emission no-op (probe: bare `END` and `END <sym>` both emit zero
bytes; the arg is an entry-point marker). Add `"end" | "END" => {}` to `dispatch`.

Snippet goldens per form (`t2_*`: abs.w/abs.l/boundary for `lea` + EA-general
`move`, `END` no-op). Acceptance: recon reaches **0 diagnostics** (with T1),
which arms the `m1c_rom` emit path for the first time.

### T3 — Width selection moves into the front-end pass loop (fixes F2). ✅ DONE (2026-07-04).

The architectural fix, replacing the linker-side growth machinery on the front-end path.

**Result:** the two `stale_fold_repro.rs` `#[ignore]` reproducers flip green (`dc.l`
after a grown jmp folds to `$10006`; `SecondSection` LMA re-flows to `$0A`); the tripwires
are deleted. Both halves of F2 closed. All gates green (workspace 47 suites; strict
`m1b_gate` 5 / `m1c_vector_table` 1; harness 18 incl. `--include-ignored`; clippy
`-D warnings`); `m1c_full` still `ASSEMBLED OK: 8 sections`; 5 `t3_*` goldens
regenerate no-op. Commits: `74542f0` (backend `lower_jmp_jsr_abs`) → `2a1b2b8` (front-end
width selection + abs-EA unify + PASS_CAP) → `10a45fb` (header doc) → `9ec70a1` (reproducers
green) → `70bb0cd` (equ-target bake fix + `t3_*` goldens) → `f4d4285` (link redefinition
diagnostic) → `57a8b69`+`b62c354` (section-name dedup + the empty-section fix). Two-stage
review (spec ✅ + code-quality ✅) on the load-bearing front-end task; whole-branch review
at close.

**PROBE REFUTED THE SPEC'S EXPECTED SEMANTIC.** The spec expected asl to assume the
"long/pessimistic" form for an unknown-this-pass forward symbol. The decisive probe
(`docs/superpowers/notes/2026-07-04-m1d-t3-jmpjsr-width-probes.md`: `org $7FFA; jmp T; T:`
→ `4EF8 7FFE`, abs.w — the LEAST fixpoint, not abs.l/$8000) shows asl assumes the **short/
optimistic abs.w** form and grows W→L only when the resolved value forces it, for both
jmp/jsr AND absolute-EA. Per "never trust a spec claim over a probe" (§4), the design
follows the probe. **This SIMPLIFIED T3:** no per-site width-persistence machinery is
needed — optimistic-abs.w start makes the existing `env == prev` multi-pass loop
*inherently* grow-only (label addresses monotone-nondecreasing across passes → widths
monotone → converges to asl's least fixpoint).

**Fragment representation decision (recorded):** the front-end emits a **finished
`Fragment::Data`** (opcode + `Abs16Be`/`Abs32Be` fixup), NOT `Fragment::JmpJsrSym`.
Mirrors `abs_ea_from_expr`; the cursor advances by the true width so `phys_base` fixes
downstream LMAs by construction; `resolve_layout` sees no `JmpJsrSym` on the front-end
path → identity (the "zero growth" assertion, held trivially) and stays the live relaxer
for hand-built IR (m1b_gate). The Org+JmpJsrSym guard is kept but can no longer fire on
the front-end path — so the real object bank (`org $10000` + bare jmp/jsr + parallax `org`
back-patch) now assembles.

**One gap the goldens exposed (cured at source):** the jmp/jsr path folded the target only
for width and passed the *symbolic* expr into the fixup, but `equ` constants live only in
the front-end env, never as section labels, so the linker (symbol table = section labels
only) couldn't resolve an `equ` target. Fixed by baking the folded value (`Expr::Int(v)`)
into the fixup when resolved (mirroring `abs_ea_from_expr`), symbolic only for the Poison
case. All 22 real aeon jmp/jsr targets are code labels, so this was latent — but a real
correctness gap, caught by the `t3_*` equate-target goldens.

**`$FF8000` non-monotone region:** unreachable for aeon (all 22 bare jmp/jsr targets are
ROM code labels; RAM jumps are register-indirect `jmp (aN)`, a different path). PASS_CAP=16
backstops any pathological oscillation. Same posture as the linker.

**Hardening (both landed):** `link()` now hard-errors on a duplicate SECTION-defined
symbol (section-vs-stub still allowed). `sec{vma}` auto-names disambiguated over NON-EMPTY
sections at finalization (`dedup_section_names`) — an empty stray section (dropped before
link) must not steal the bare name. The real ROM DOES have a same-VMA-base collision
(M68000 `sec0` + Z80 region A at vma 0), so this composes with the link check and was
load-bearing, not merely defensive; the M0 harness's empty preamble `sec0` correctly keeps
region A named `sec0` (the M0 live gate keys on it).

**Original design (for reference — verify the one open semantic by probe):**
- In `lower_jmp_jsr_sym`-class lowering, pick abs.w/abs.l **per pass** from the current
  env via the existing pinned `asl_width_rule`; advance the cursor by the **true** width
  (2-word or 3-word form); let the existing `env == prev` convergence absorb growth
  exactly as asl's own repeat-until-stable does.
- **Probe first:** what width does asl assume for a symbol *unknown in the current pass*
  (expected: the long/pessimistic form — verify), and does a
  width-depends-on-own-address construct oscillate or converge in asl (the `$FF8000`
  non-monotone note in `relax.rs:126-143` says asl can oscillate — confirm it stays
  unreachable for ROM-targeting jmp/jsr and record the probe).
- Consequences to implement, not hope for: `phys_base` now accumulates **true** section
  lengths → downstream LMAs (the Z80 driver) correct by construction — the unflagged
  half of F2 closes for free. `PASS_CAP = 8` must be raised or made growth-aware
  (growth consumes extra convergence passes).
- The linker keeps `asl_width_rule` + `resolve_layout` as a **verification assert** on
  the front-end path (widths already final → zero growth expected; assert it) and as the
  live relaxer for hand-built IR (m1b_gate). Keep the `Org+JmpJsrSym` guard as a
  tripwire. Whether `Fragment::JmpJsrSym` survives on the front-end path or the
  front-end emits finished `Data`+fixup is implementer's choice at plan time — record it.
- In-scope adjacent hardening (both audit-flagged, both bite at full-ROM link): `link()`
  symbol redefinition is silently last-write-wins (`sigil-link/src/lib.rs:47-49`) — make
  it a diagnostic; `sec{vma}` auto-names collide for a future second bank phased at the
  same address (`eval.rs:1611`) — disambiguate (e.g. ordinal suffix).

Acceptance (met): T0.2's reproducer flips green with byte-correct output; all snippet
goldens and strict gates still green. The `$1012E`=`4EF9` full-ROM spot-assert needs the
emit path (`m1c_rom`), so it lands in **T4** — T3 arms it (the object bank now assembles
through resolve_layout→link with no guard trip).

### T4 — First full-ROM emit + first-diff triage (A1). ✅ DONE (2026-07-05).

**Result: Sigil's assembler is BYTE-EXACT for the entire non-debug ROM.** Proven two
ways: (a) `sigil emit_rom` output == the assembled (pre-convsym) reference, sha256
`286127635f52fa51`; (b) `sigil emit_rom` + real `convsym -a` + `fixheader` == the pinned
`s4.bin`, sha256 `605631da…ad5117`. New strict gate `crates/sigil-harness/tests/m1d_rom.rs`
(`full_rom_matches_assembled_reference`) asserts Sigil's ROM differs from `s4.bin` at
EXACTLY 4 bytes — `{0x18E,0x18F}` (checksum) and `{0x1A6,0x1A7}` (ROM-end pointer low half)
— both rewritten by the out-of-scope `convsym`/`fixheader` post-steps. Skip-green without
aeon, like the others.

**A1 scope decision (2026-07-05, user-confirmed):** the target of byte-exactness is the
**assembled ROM**, NOT `build.sh`'s literal `s4.bin`. `build.sh` runs `convsym … -a`, which
APPENDS the MD-Debugger `deb2` symbol table (~34 KB) and rewrites two header fields; that
append is debug tooling (not executed / not game content — the MD-Debugger analogue of an
ELF `.symtab`), and M1.B already modelled `convsym` as a no-op. Replicating convsym's deb2
encoder was declined as large effort for zero assembler-correctness value. This is the
key T4 finding that reshaped A1.

**Four real assembler gaps found by first-diff triage and cured at the source of the class**
(each probe-first, each with real-asl `t4_*` goldens; probes in
`docs/superpowers/notes/2026-07-04-m1d-t4-macro-local-scope-probes.md` + companion scratch):
1. **Macro-internal `.`-local scoping.** asl scopes a `.`-local defined INSIDE a macro body
   to that EXPANSION (unique per expansion — `queueStaticDMA.done` expands 7× in one scope
   with no collision), while a source-level `.`-local redefinition is a hard error (probes
   P1/P3). Sigil qualified macro `.`-locals to the caller's global label → 14 false
   `link()` duplicate-symbol collisions (the T3 hardening, firing correctly). Fix:
   `expand_macro_inner` runs the body under a fresh reserved scope `" macro#N"`. A `.`-local
   passed AS A MACRO ARGUMENT (`aabb_axis_test …,.next_object,…`) is qualified against the
   CALLER scope before substitution (`qualify_macro_arg`) — asl evaluates args in the caller
   context.
2. **Compound PC-relative targets.** `qualify_expr` was shallow (top-level `Sym` only), so
   `.`-locals nested in arithmetic — `jmp .cc_table-4(pc,d0.w)`, `bra.w .drain_end-.c*8` —
   never qualified → the linker's global fold couldn't resolve them. Fix: `qualify_expr`
   recurses (mirroring `resolve_dollar`); a new `fixup_target` helper folds-and-bakes the
   target (`Expr::Int` when resolved) so env-only `rept`/`set` counters (`.c`) resolve at
   the instruction (the counter's value HERE, not its final value) — the T3 bake pattern.
3. **asl zero-displacement optimization.** `(d16,An)` with displacement 0 → `(An)`, dropping
   the extension word (`move.b d0,0(a0)` → `1080`, not `1140 0000`) — unconditional (NOT
   `-A`-gated), EXCEPT `movep` (no register-indirect form; keeps `03C8 0000`). This was the
   first STRUCTURAL diff (a `rept` at `.c=0` in `Init_DMA_Queue`). Fix in
   `convert_atoms_m68k`: rewrite `Disp16An(0,n)` → `Ind(n)` for every mnemonic but `movep`.
   Must be a front-end choice (changes EA width 4→2) so the layout cursor stays correct.
4. **Empty RAM sections.** Pure-`ds` sections (RAM vars phased to `$FFFF0000`+, `image_len`
   0) carry a physical-counter LMA that aliases real code → false `emit_rom` overlap. Fix:
   `flatten`/`flatten_checked` skip zero-byte sections (they emit no ROM bytes, faithful to
   asl/p2bin).

The predicted first-diffs (F1 padding, object-bank layout) did NOT fire — T0.1's padding
model and T3's front-end width selection were already correct through the full ROM; the
`$1012E`=`4EF9` object-bank grow landed byte-exact with no special handling. **All gates
green** (workspace 48 suites; strict m1b 5 / m1c_vector_table 1 / m1d_rom 1 / M0 live 18
incl `--include-ignored`; clippy `-D warnings`; `gen_snippet_vectors` churns only the 7 new
`t4_*` blocks — non-circularity intact).

---
*(original T4 plan follows)*

With recon at 0, run `m1c_rom` (assemble → resolve_layout → link → emit_rom vs
`s4.bin`). Expect second-order surprises — this path has never run on real source.
Triage protocol per divergence: first differing offset → map through `s4.lst` (asl's
listing, or sigil's `emit_listing`) to the source line → classify (encoder? fold?
padding? layout?) → **cure at the source of the class, add the missing snippet golden,
re-run** — never patch bytes. F1's padding divergence is the predicted first hit if T0.1
was incomplete.

**T3 watch item to verify at bring-up:** T3's new `link()` duplicate-section-symbol
diagnostic treats *any* same-named label across two sections as a hard error (correct for
Sigil's globally-scoped auto-section layout; `m1c_full` links its 8 sections clean). If the
real ROM ever relies on asl `SECTION`-directive namespacing to legitimately reuse a label
name across regions, this would false-positive — watch for it on the first full `m1c_rom`
link and scope the check to genuine collisions if it fires.

Acceptance: `sha256` match, non-debug; promote `m1c_rom` from example to a
`SIGIL_STRICT_GATE` test `m1d_rom` (skip-green without aeon, like the others).

### T5 — `__DEBUG__` build parity (A2). ✅ DONE (2026-07-05).

**Result: Sigil's `__DEBUG__` build is BYTE-EXACT** to the deliberately-built debug
reference (`aeon/s4.debug.bin`), identical over `[0, EndOfRom=0x673A2)` except the 4
`convsym`/`fixheader` header bytes `{0x18E,0x18F,0x1A6,0x1A7}` — the same A1/A2 shape as
`m1d_rom`. New strict gate `crates/sigil-harness/tests/m1d_debug_rom.rs` + helper
`assemble_full_rom_debug`. Commits `c6c278b` (the six cures + gate) → `d3656ff` (review
fix); spec✅ + code-quality✅ review (ready-to-merge; one latent-panic fix applied).

**Six real assembler gaps found by first-diff triage on the debug surface's first
whole-image exercise, each probe-first vs live asl, each with a `t5_*` snippet golden
(regen churned only the 6 new blocks):**
1. **`==` is asl's C-style equality alias for `=`** (lexer). It lexed as two `Eq` tokens
   → `parse_expr` choked. Pervasive in `debugger.asm`.
2. **String comparisons fold as SUB-expressions of boolean logic**, not only as a whole
   `if` condition. New `expand_str_comparisons` folds `<str-expr> (=|<>) "literal"` → 0/1
   (probe: `((strlen(t)==2)&&(substr(t,0,1)=="."))`=1), wired into `eval_all` + `directive_db`;
   `trailing_str_expr_len` finds the LHS extent (literal / bare ident / `substr`/`lowstring`).
3. **`substr` edge semantics**: pos AT/PAST end → `""`, NEGATIVE len → `""` (was `None`/error)
   — the `%<…>` decoder hits `substr(string,len,-1)`.
4. **`cmp <ea>,An` ≡ `cmpa`** (`refine_m68k_mnemonic`: `(Cmp,[_,An])=>Cmpa`; probe `B3C8`).
   `assert`'s `cmp.ATTRIBUTE dest,src` with an An `dest`.
5. **A string `.`-local `set` symbol passed as a macro arg is substituted BY VALUE** (quoted)
   — the T4 reserved scope name `" macro#N.local"` can't re-lex as one identifier, so
   `switch`/`lowstring`/`substr` in the callee couldn't resolve it. `__FSTRING_PushArgument`
   consumes `.__operand`/`.__param` this way. New `bind_macro_arg`.
6. **`render_tokens` merge-aware spacing**: space two tokens only when they'd MERGE on
   re-lex (both ident chars), so `#1` renders `#1` not `# 1` — the extra space became a
   literal byte when a macro arg is embedded in a `%<…>` assert string (`dest=#1`). This one
   byte cascaded to 17.5k downstream address diffs.

The T5 review caught a latent panic (`trailing_str_expr_len` underflow on empty input,
`dc.b <>"x"`) — fixed + regression test. Non-debug ROM unaffected throughout (the fixes are
debug-only-exercised or byte-identical for non-debug; `m1d_rom` stayed green each step).

**Original scope (for reference).** Build the debug reference deliberately (aeon's
`__DEBUG__` switch; record the exact
invocation in PROVENANCE.md — the debug ROM is NOT the shipped `s4.bin`). Assemble the
same config in sigil; compare. Known dependency: T0.4 (`cmpm`). Expect the debug
surface (debugger.asm's `.ATTRIBUTE`/`switch`/`lowstring` paths, already implemented in
M1.C) to get its first whole-image exercise.

**A2 scope inherits T4's decision:** the debug reference is also post-processed by
`convsym -a` (larger symbol table, since `__DEBUG__` adds symbols), so compare against the
**assembled** debug ROM (the `m1d_rom` gate's "diff set is exactly the convsym-rewritten
header bytes" shape, adjusted for the debug build's `EndOfRom`), not the convsym-appended
artifact.

### T6 — Delete the stub table (A3). ✅ DONE (2026-07-05).

**Result:** the ~42-symbol stub table and the entire M0 **bounded harness** are retired;
the full build defines everything (zero stubs). Commit `21cf234`, code-quality review ✅
(ready-to-merge, no blocking issues; the one actioned suggestion — an empty-section guard
against a vacuous pass — is folded in).

The bounded harness (`harness_root.asm` + `golden/stub-syms.toml` + `build_harness`)
assembled Regions A+B *in isolation*, stubbing the leaf symbols the 68k side it did not
assemble would define. That scaffolding existed only because Sigil could not yet assemble
the whole 68k ROM. T4 removed that premise. So T6 **retired the bounded harness wholesale**
rather than stub-freeing it piece by piece (every consumer had to change anyway):

- **Deleted:** `harness_root.asm`, `golden/{stub-syms.toml,windows.toml,region_a.bin,
  region_b.bin,sigil_a.bin,sigil_b.bin}`, the `regen` bin, and the lib helpers
  `build_harness`/`assemble_reference_regions`/`reference_options`/`parse_stub_syms`/
  `load_stub_syms`/`golden_path`/`LmaMap`/`assign_lmas`/`derive_region_*`/
  `parse_lst_symbols`/`RegionWindow`/`diff_region`.
- **New M0 acceptance gate** `crates/sigil-harness/tests/m0_regions.rs`: the full non-debug
  build (no stubs) → locate the linked sections at LMA `0x3EA` (Region A) / `0x60000`
  (Region B) → assert each byte-identical to the live `aeon/s4.bin` window. Region
  **lengths** are read from the live sections (not pinned), so it tracks driver growth; an
  empty-section guard keeps the compare from passing vacuously. Reference-gated exactly like
  `m1d_rom` (skip-green without aeon; `SIGIL_STRICT_GATE=1` hard-fails a missing reference).
- **`lib.rs` slimmed** to one reference-build entry point: `assemble_full_rom(aeon)` +
  `region_at_lma`. `m1d_rom` was DRY'd onto `assemble_full_rom` (behavior-preserving —
  same sha256). The CLI `build`/`diff` subcommands were repointed to the full build
  (`build -o` emits the full ROM; `diff` compares Regions A+B vs `aeon/s4.bin`).
- **Docs:** `golden/PROVENANCE.md` M0 section rewritten; the `eval.rs`
  `dedup_section_names` comment's dead `build_harness` referent replaced.

All gates green: workspace tests; clippy `-D warnings`; strict `m0_regions` 1 / `m1b_gate`
5 / `m1c_vector_table` 1 / `m1d_rom` 1 (a non-`#[ignore]` skip-green test, so
`--include-ignored` is a harmless superset, not a requirement). CLI `build -o` emits sha256
`286127635f52fa51…` (== the T4 assembled ROM).

**Original scope (for reference).** The ~42-symbol M0 stub table exists only because the
M0-era harness assembled the Z80 regions without the 68k side. The full build defines
everything (recon: zero stubs). Delete it; re-run the M0 acceptance path via the full-build
machinery. Acceptance: no stub file, all gates green.

### T7 — Spec + doc reconciliation (A4). ✅ DONE (2026-07-05).

All three parts landed in `empyrean/docs/SIGIL_CORE_SPEC.md` + the sigil sweep:
1. **Backports (each with a "verified vs asl 1.42 Bld 212" note):** comparisons fold **0/1**
   (§4.5 + §7.1); infix `!` = **XOR** (§4.5 + §7.1); `==`≡`=` (§4.5 + §7.1); `int()` =
   **floor** (§7.1); the normative **`strstr` bug-for-bug paragraph is deleted / inverted**
   (§7.1) with its §12 R3 + R7 mentions corrected (bug does not reproduce); `save`/`restore`
   given the **T0.1 reset-on-CPU-switch** semantics (§4.9 — corrected the "preserve exactly
   what AS preserves" claim).
2. **Architecture supersessions** recorded as a consolidated §4 note (same status as the §3.2
   salsa deferral): `ProvenanceStack` deferred (Span-only); `RelaxableFragment`/`ChosenSizes`
   never built (→ `JmpJsrSym`+`resolve_layout` → T3 front-end width selection); `Diagnostic`
   narrowed to `{level,message,primary}`; `IrStreamer` narrowed (+ `emit_fragment`); §9.1
   frontend→backends edge blessed; `SymbolValue` stays `Int|Poison`. §8.4 CI wording +
   M0/M1 acceptance rewritten to reality (local `SIGIL_STRICT_GATE`; T6 M0 re-expression;
   A1/A2 convsym-append-out-of-scope + assembled-`EndOfRom` length, not 458737).
3. **Stale-comment sweep:** `gen_snippet_vectors.rs` + `asl_snippets.rs` ("hand-verified" →
   generator-produced / non-circular); README status table (M1 ✅, assembled-ROM target;
   `sigil-harness` crate row = the gates, `regen` retired).

**Original scope (for reference).**

1. **`empyrean/docs/SIGIL_CORE_SPEC.md` backports:** comparisons fold **0/1** (§7.1/§4.5);
   `int()` = **floor** (header/D8); **remove the normative `strstr` bug-for-bug paragraph**
   (§7.1) and its §12 R3 risk entry (disproven in asl 1.42 Bld 212); `save/restore`
   preserve cpu/padding/supmode with the **T0.1-verified reset-on-CPU-switch semantics**
   (§7.1). Each with a "verified 2026-07-0X vs asl 1.42 Bld 212" note.
2. **Architecture supersession notes** (same doc, same pattern as the existing salsa
   deferral note): `ProvenanceStack` deferred (nodes carry `Span` only — record it as a
   *decision*, revisit at Spec 3); `Diagnostic` narrowed to `{level, message, primary}`;
   §4.3 `RelaxableFragment`/`ChosenSizes` superseded by `JmpJsrSym`+`resolve_layout`,
   itself superseded by T3 front-end width selection; §4.9 `IrStreamer` narrowed (no
   `emit_instruction`/`set_phase`/push-pop state); §9.1 frontend→backends edge blessed
   (D1 rationale from the M1.C design); §8.4 CI-gate wording → reality (reference gates
   run locally via `SIGIL_STRICT_GATE=1`; GitHub CI self-skips them).
3. **Stale-comment sweep:** `gen_snippet_vectors.rs:15-17` + `asl_snippets.rs:3`
   ("hand-verified" → generator-produced, demonstrably true); README status table (M0
   live-gate caveat until T0.3 lands; M1 row); `sigil-isa/src/m68k.rs` phrasing is
   already honest ("the set Aeon uses") — make README/memory phrasing match it.

---

## 3. Sequencing & rationale

```
T0.0 → {T0.1, T0.2, T0.3, T0.4}   (parallel, no blockers)
T1 → T2 → (recon = 0) → T3 → T4 (A1) → T5 (A2) → T6 (A3)
T7 anytime; before close
```

T0 first because all four items are cheap, independent of the blocker, and two of them
(T0.1, T0.2) convert audit findings into committed, executable truth *now* — the padding
divergence and the stale-fold failure must not be re-discovered at T4 as mystery diffs.
T3 sits after recon-0 only because its acceptance needs the real growth sites
assemblable; its design work can start any time. The handoff's original ordering deferred
F1/F2 understanding to "when it bites" — this spec front-loads them.

---

## 4. Process requirements (carry-forward + audit-hardened)

- **Probe-first, committed-probe:** any claim about asl semantics is established by
  running the live binary *before* implementation, and the probe must survive the session
  — as a `gen_snippet_vectors` golden when byte-affecting, or a checked-in notes doc with
  the probe source when not. A comment saying "probe-verified" without a committed
  artifact is what produced F1. Never trust a spec/doc claim over a probe (this
  milestone deletes one disproven spec bug and corrects three spec facts).
- **Every byte-affecting change lands with real-asl goldens** (`gen_snippet_vectors`
  regenerates as a no-op — the audit's non-circularity proof depends on keeping this
  invariant).
- **Strict gates before merge:** `SIGIL_STRICT_GATE=1 AEON_DIR=… cargo test -p
  sigil-harness` + workspace tests + `clippy --workspace --all-targets -- -D warnings`.
- **Cure at the source of the class** (R7): on any ROM diff, fix the semantic class and
  add the missing golden; never special-case bytes.
- **Update this doc + the memory note as tasks land** (mark done, record snags, record
  the T3 fragment-representation decision).

---

## 5. Risks

| # | Risk | Mitigation |
|---|---|---|
| 1 | T3 probe reveals asl's unknown-forward-symbol width guess differs from expected, or an oscillation reachable in aeon | Probe before design freeze; the pinned width rule + listing spot-checks (`$1012E` = `4EF9`) bound the blast radius |
| 2 | T0.1 padding model interacts with regions that are currently byte-green | Existing strict gates double as regression armor; implement behind the probe matrix, not intuition |
| 3 | T4 exposes unknown-unknowns (this path has never run on real source) | The triage protocol + first-diff→listing mapping; budget for iteration, don't promise A1 in one pass |
| 4 | Moving aeon reference drifts mid-milestone | T0.0 pin; re-pin only deliberately, with `regen` + PROVENANCE update |
| 5 | Debug reference ROM (T5) config drift vs what aeon actually ships | Record the exact build invocation in PROVENANCE.md when first produced |
