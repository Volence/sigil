# Design — Spec 2 · Plan 7 backlog #8: `jbra`/`jbsr` + unsized-conditional relaxation

Date: 2026-07-08 (Fable, overnight session). Decisions locked per role split (Volence defers
technical calls; checkpoint in the morning, NO merge tonight). Both halves are ratified spec
(§5.4 + D2.18) — this is implementation design, not language design. Inputs: the overnight
handoff, the item-4 gap analysis (b1/b2), spec §5.4/D2.18, a fresh code-seam survey of
`sigil-link/src/relax.rs` + `sigil-ir` + the emp branch-lowering path, and two post-merge
audits of the #6 merge (d1e4288..90a21b6).

## Verified code facts this design stands on (re-verified at T0, 2026-07-08)

- **Green gate on master:** exactly the 4 allowlisted sigil-harness reds (aeon sound-driver
  strlen drift); clippy clean.
- **PC-relative fixup kinds already exist and link cross-module** (fixup.rs:28–36):
  `PcRel8` (disp = target − (site+1), i.e. target − (op+2); range i8), `PcRelDisp16`
  (disp = target − site where site is the extension word's own VMA = op+2; range i16, BE).
  Sized `bra.s/.w`/`Bcc` lower to Data + these fixups (backend-m68k lib.rs:96–132) and the
  linker resolves targets globally — cross-module sized branches work today.
- **ERRATUM (caught by the T1 spec review):** the claim that the AS frontend never emits
  PcRel8/PcRelDisp16 fixups was WRONG — `lower_m68k_branch` (frontend-as eval.rs:2367) routes
  `.s` branches through the backend's symbolic `PcRel8` fixup, resolved at link. The disp-0
  apply guard is nevertheless safe for AS ports: a `.s` branch to op+2 is unencodable on the
  68000 (asl rejects it too), so no byte-exact port can legitimately trip it; the full AS
  suite stays green with the guard in place.
- **The relaxation fixpoint** (relax.rs `resolve_layout`) is grow-only W→L over per-fragment
  `AbsWidth` state, pass-capped by total flips + 1, with an Org/Reserve mixing guard
  (`has_relaxable`). `AbsWidth` is consumed nowhere outside relax.rs + the abs-sym frontend
  seam — the per-fragment width state is **internal** and can be generalized without API churn.
- **`Fragment` has exactly 7 exhaustive match sites** needing a new arm: `vma_len`,
  `placement_span`, `image_bytes` (sigil-ir/lib.rs:150–255), `link()`'s fixup walk
  (sigil-link/lib.rs:91–126), `frag_len`/`frag_span`/the resolve_layout lowering
  (relax.rs:26–98, 285–300). `JmpJsrSym`/`RelaxAbsSym` are `unreachable!()` in
  vma_len/image_bytes/link (resolved before link); `placement_span` uses the MAX length.
- **Branch opcodes** (isa m68k.rs): base `0x6000 | cc<<8`; bra cc=0, bsr cc=1, bne cc=6,
  bhi cc=2. `.s` = opcode word with disp in low byte (2B); `.w` = opcode word + disp word (4B).
  jmp abs.w/l = 4EF8/4EF9, jsr = 4EB8/4EB9, operand at offset 2 (relax.rs `lower_jmp_jsr`).
- **`[branch.missing-size]`** is raised in `lower_m68k_branch` (frontend-emp lower/code.rs:
  160–196) when `size == None`; targets arrive as a single `CodeOperand::Sym` already renamed
  by the #5 hygiene pass (proc-locals → mangled/dotted global names), same as jmp/jsr.
- **`@as_compat`** is read from `file.attrs` into `ProcCtx.as_compat` (lower/mod.rs:83–92,
  proc.rs:56) — available exactly where branch lowering runs.
- **68000 encoding quirk:** a Bcc/BRA/BSR byte displacement of `0x00` is not a displacement —
  it escapes to the word form. So `.s` with disp 0 is unencodable; the current emp path would
  silently emit a desynced instruction (asl-parity irrelevant: AS ports don't take this path).

## Part 0 — #6 post-merge audit dispositions (fix on this branch, before feature work)

Two opus audits of d1e4288..90a21b6. Byte-exactness and no-regression both corroborated clean
(offsets emission byte-identical master-vs-merge; all new encodings match hand-derived bytes).
Two composition findings, one real defect:

- **0a. DEFECT — cross-module bare-window overlays re-resolve in the CONSUMER's namespace**
  (independently reproduced by Fable: `pub vars PlantV: sst_custom {…}` over `lib.Sst`
  imported into a module whose own struct has a same-named `sst_custom` field emits
  `tst.b $8(a0)` instead of `$2(a0)` — silent wrong bytes; with both structs in scope it
  instead produces a spurious `[overlay.ambiguous-window]` anchored in the LIBRARY file and
  poisons even qualified access). Root cause: `resolve_bare_window` scans the consumer
  evaluator's structs/overlays. **Fix (semantic, locked): a bare window binds at the
  overlay's DEFINITION site — resolved once against the defining module's namespace, and the
  resolved binding travels with the injected overlay.** A consumer's unrelated same-named
  field must neither rebind nor break a library overlay. Dotted-window overlays are immune
  (confirmed) and unchanged. Mechanism (resolve-at-defining-lowering + memoized layout on the
  overlay record, vs. re-resolving in the origin namespace) is the implementer's choice;
  the contract is fixed by tests on both repro variants + a still-works bare-access case.
- **0b. NOISE — struct-declaration diagnostics double-report cross-module** when a `pub vars`
  overlay forces the base struct's layout in the defining module and a consumer forces it
  again (`(size:)` mismatch block prints twice; `dedup_overlay_pass_diags` is per-module by
  construction). **Fix: key the dedup once-per-compile (build_program level) for
  struct-declaration diagnostics.** Timeboxed: if it turns invasive, drop with a recorded
  note — it is noise, not miscompile.
- Auditor-B NOTE (cosmetic, pre-existing, shared with offsets): `<unresolved>` placeholder
  secondary error after `[dispatch.target-not-code]`. Ledger, not tonight.

## Part A — Core: the generic relaxation ladder (`Fragment::RelaxLadder`)

One new **additive** fragment (D-P4.7/BankPtr16Be precedent — no change to existing variants;
`RelaxAbsSym` is NOT refactored onto it tonight, rule of three, ledger note):

```rust
/// An instruction with several complete candidate encodings, ordered from
/// smallest to largest; resolve_layout picks the FIRST candidate whose fixup
/// kind can reach the resolved target, never shrinking a prior choice.
RelaxLadder {
    candidates: Vec<RelaxCandidate>,   // ≥1; bytes.len() non-decreasing
    target: Expr,
    span: Span,
}
```

- **Reach is derived from each candidate's `FixupKind`** — no branch semantics leak into a
  new tag enum, and the ladder is CPU-agnostic (a future Z80 `jr`→`jp` ladder reuses it via
  `Z80JrRel8`; ledger):
  - `PcRel8`: disp = target − (site_vma + 1) fits i8 **and disp ≠ 0** (the 0x00 word-form
    escape — unencodable as a byte displacement);
  - `PcRelDisp16`: disp = target − site_vma fits i16;
  - `Abs16Be`: `asl_width_rule(target) == W`;
  - `Abs32Be`: always reaches;
  - any other kind in a ladder: construction contract violation → loud error.
- **Fixpoint generalization, internal to relax.rs:** per-fragment state goes from
  `AbsWidth` to a **rung index** (`usize`); JmpJsrSym/RelaxAbsSym map 0→W, 1→L unchanged.
  Selection per pass: minimal reaching rung `m`; `new = max(cur, m)` (grow-only). If nothing
  reaches mid-pass, hold at the last rung provisionally — the error is only reported at the
  **convergence sweep** (addresses are still moving mid-fixpoint; a premature error could be
  spurious). `grew` is set only when the fragment's LENGTH changes (a same-length rung move,
  bra.w→jmp.w, needs no relayout). Pass cap: Σ(rungs−1) over all relaxable fragments, + 1 —
  the same each-fragment-flips-boundedly termination argument, now ≤3 flips for a 4-rung
  ladder.
- **Convergence sweep:** after `!grew`, every RelaxLadder's chosen candidate must reach.
  A conditional ladder maxed at `.w` that still can't reach reports
  `[branch.out-of-reach] … target … is N bytes away (max ±32766); conditional branches have
  no far form — jbcc trampolines are deferred (D2.18)` with the signed distance N.
- **Org/Reserve guard:** `has_relaxable` extends to `RelaxLadder` (same hazard, same loud
  refusal).
- **Match-site arms:** `frag_len` = chosen rung's `bytes.len()`; `placement_span` = LAST
  candidate's length (max); `vma_len`/`image_bytes`/`link()` = `unreachable!()` like the other
  relaxables; `frag_span` trivial. Lowering at convergence = chosen candidate's bytes + its
  fixup as a `Data` fragment (identical shape to RelaxAbsSym's lowering — the linker encodes
  nothing).
- **Drive-by hardening (same task):** `apply_fixup` for `PcRel8` rejects disp == 0 with a
  loud link error instead of silently writing the 0x00 escape byte (reachable today via an
  explicit `bra.s` to the next instruction; .emp-only path, AS ports unaffected — verified
  fact above).

## Part B — Frontend: `jbra`/`jbsr` (D2.18)

Recognized in emp instruction lowering BEFORE the isa mnemonic table (they are emp-only
mnemonic-position words per §10's headroom rule — they must NOT enter sigil-isa's shared
table, or the AS frontend would start accepting them):

- `jbra L` → RelaxLadder: `[0x60,0x00]+PcRel8@1` → `[0x60,0x00,0x00,0x00]+PcRelDisp16@2` →
  `[0x4E,0xF8,0,0]+Abs16Be@2` → `[0x4E,0xF9,0,0,0,0]+Abs32Be@2`. Baseline advance 2.
- `jbsr L` → same with 0x61 / 4EB8 / 4EB9.
- Candidate byte-blocks are built by the m68k BACKEND (new helper alongside `lower_branch`/
  `lower_jmp_jsr_sym`) so instruction encodings stay out of lower/code.rs, matching the
  RelaxAbsSym precedent.
- bra.w (4B, pc-rel) is deliberately ranked before jmp abs.w (4B): same length, relocatable,
  and matches D2.18's "bra.s/bra.w/jmp" ladder order.
- Diagnostics: `jbra.s`/`jbra.w` → `[jbra.sized] jbra sizes itself — drop the suffix (pin
  with bra.s/bra.w, or use jmp for computed targets)`. Non-label operand →
  `[jbra.label-only]` naming `jmp (a0)`-style computed transfer as the alternative. Non-68k
  section → error (Z80 ladder is deferred; mirror `[dispatch.non-68k]`'s shape).
- **`jbra` is an unconditional terminator** for `[proc.undeclared-fallthrough]` (the gap
  analysis's three artifact errors); `jbsr` is not.
- Targets: single `CodeOperand::Sym`, already hygiene-renamed — same contract as sized
  branches and jmp/jsr today. `jbra` is legal in `@as_compat` files too (it is not an AS
  mnemonic, so it cannot appear in faithful ports; no special-casing either way).

## Part C — Frontend: unsized conditional relaxation (§5.4)

- Unsized `bne L`/`bhi L`/any `Bcc` in a NON-`@as_compat` module → 2-rung RelaxLadder
  (`[0x60|cc,0x00]+PcRel8@1` → 4-byte word form+PcRelDisp16@2). No far form: out of ±32K
  reach is the Part-A convergence error naming the distance.
- **Unsized `bra L`/`bsr L` relax the same 2 rungs** (ratified here for uniformity — §5.4
  says "unpinned branches are sized by Core's relaxation", full stop; the jmp fallback stays
  exclusive to `jbra`/`jbsr` per D2.18, and the out-of-reach error for an unsized `bra`
  suggests `jbra`).
- Under `@as_compat`: `[branch.missing-size]` stays verbatim (ports pin widths). Plumb
  `ProcCtx.as_compat` into `lower_m68k_branch`.
- Explicit `.s`/`.w` remain pins everywhere (byte-identical to today).
- `[branch.suboptimal-size]` informational lint: **deferred** (needs a pinned-width audit at
  link time — not trivial; ledger).

## Acceptance (what "done" means for #8)

- Core: ladder unit tests in relax.rs style — short/word/jmpw/jmpl selection, backward
  branches, disp-0 → word rung, growth cascade shifting labels (the multi-jmp cascade test's
  shape), org-guard, out-of-reach conditional error with distance, PcRel8 disp-0 link guard.
- Frontend: jbra/jbsr forms + all diagnostics above; unsized Bcc/bra/bsr relaxation;
  `@as_compat` retention; fallthrough-terminator recognition.
- `examples/reach_branches.emp` exhibit + ports.rs byte-exact image, hand-derived bytes
  independently verified by the controller.
- pitcher_plant b1/b2 error classes verified gone via scratch probe (full exhibit compile is
  Step 2's acceptance, not #8's).
- Green gate per commit: workspace tests with ONLY the 4 allowlisted reds; clippy -D warnings.

## Deferred (ledger candidates)

- `jbcc` conditional trampolines (D2.18 — demonstrated need only).
- Z80 `jr`→`jp` ladder on RelaxLadder via `Z80JrRel8`.
- Folding `RelaxAbsSym` (and possibly `JmpJsrSym`'s linker-side encoding) onto `RelaxLadder`
  — rule of three; revisit at the third ladder client.
- `[branch.suboptimal-size]` lint.
- Auditor-B cosmetic `<unresolved>` secondary diagnostic (shared offsets/dispatch pattern).
- **`here()` vs relaxation (whole-branch review NOTE-1, PRE-EXISTING but now routine):**
  `here()` reads the mid-lowering cursor, which advances relaxables at their BASELINE rung
  (jbra: 2 bytes; bare jmp/jsr: 4) — a comptime value emitted from `here()` after a fragment
  that later GROWS is stale by the growth delta, though the image layout itself is correct.
  Master has the identical edge via JmpJsrSym abs.w→abs.l; ladders make it reachable from any
  `jbra`/unsized branch crossing >127 bytes. Affects the `ensure_fatal(here() <= $9000)`
  budget idiom (guards.emp:58): a section overrunning by ≤ Σ(growth) could pass silently.
  Ledger: either spec the "no relaxables between here() and the position it guards"
  constraint, or teach relaxation to fix up emitted here() values. Not fixed on this branch.
