# Sigil Spec 2 · Plan 6 — `@as_compat` + mixed `.asm`/`.emp` build + per-file byte-diff — Design

> Design doc for the Spec 2 **capstone-proof** milestone. Written 2026-07-06 during
> the Plan 6 brainstorming session with Volence. Scope + decisions approved in that
> session; this doc precedes the T-task implementation plan (`empyrean/docs/plans/`).
> The implementation lands on an isolated worktree branch and is **NOT merged to
> master without a Volence checkpoint** (milestone rule, same as Plans 2–5).

## 1. Goal (what Plan 6 proves)

Take a **real Aeon `.asm` file**, port it to `.emp`, compile it through the modern
front-end, and prove the emitted bytes are **byte-identical** to the AS-assembled
original — the payoff of everything Plans 1–5 built. Plus: prove that an `.emp`
module and an `.asm` file **compose into one image through the shared link layer**
(a symbol defined by one resolves for a consumer in the other).

Plan 6 proves the **mechanism**. It does **not** port the ROM (that's the migration
era) and does **not** build new language features (that's Plan 7). See §7.

## 2. Prerequisite assessment (verified in CODE, not the spec)

The ⚠️ prerequisites the handoff demanded, answered by reading the real code:

- **The mixed-build link seam already exists.** Both front-ends emit
  `sigil_ir::Section`; `sigil_link::link` (crates/sigil-link/src/lib.rs:50) builds
  **one flat symbol table across all sections regardless of producing front-end**,
  resolves every `Fixup` against it, and flags cross-section name collisions. Width
  selection (`asl_width_rule`) is shared via `resolve_layout`. So "merge `.asm` +
  `.emp` into one image with a shared symbol namespace" is **not new
  infrastructure** — concatenate the two `Vec<Section>` and link once.
- **`include` is not a section boundary.** The AS front-end opens sections only on
  CPU-switch / `phase` / `dephase` / `org` — never on `include`. The whole main 68k
  ROM is one giant section; an included file's bytes are spliced mid-section. So a
  full-ROM *in-place splice* of one included file is a materially larger problem
  than the link seam itself — **deferred** (see §7; the end-state replaces `include`
  with modules + map-file placement anyway, so splicing is a transitional dead-end).
- **`@as_compat` is parsed but completely inert.** It lands in `File.attrs`
  (parser.rs:120) and **nothing in lowering reads it** (mod.rs consumes only
  section-level `sec.attrs`). For a *data* file, byte-exactness is already
  structurally guaranteed (no-implicit-padding rule §4.3 + shared relax width rule),
  so `@as_compat`'s real content is thin: silence modernization/suboptimal-size
  lints, honor width pins, and mark the file as opted into the diff contract.
- **`.emp` data cannot express a symbol *difference* today.** `Cell::SymRef`
  (lower/data.rs) only yields **absolute** fixups (`Abs16Be`/`Abs32Be`/`BankPtr`).
  There is no `LabelB - LabelA` (relative-offset) mechanism and no way to intersperse
  labels in a data stream. This is the **offset-table** pattern — and it blocks the
  representative data files (see §4). It is the #1 Plan 7 feature, not Plan 6.

## 3. Architecture — the four pieces

### 3.1 Mixed-build composition (link seam)
No new link infra. A driver assembles the AS side (`sigil-frontend-as`) and the emp
side (`sigil-frontend-emp::lower::lower_module`), concatenates their `Vec<Section>`,
and calls `resolve_layout` + `link` once. Cross-front-end symbol references resolve
through the shared table; a name collision across the two is a hard `Error`. Plan 6
proves this with a **link-seam test**: an emp-produced section defining the ported
symbol + an AS-produced section that references it → one link → symbol resolves and
the bytes are correct. (Full-ROM in-place splice is out — §7.)

### 3.2 `@as_compat` semantics (wire the inert attribute)
Read `File.attrs` in lowering; when `@as_compat` is present, set a `LowerOptions`
flag that:
- **silences** modernization / `[branch.suboptimal-size]` lints for the module
  (§8.2) — these are the "this file is a faithful port, not new-style code" markers;
- **requires** pinned branch/EA widths (a data-only first target has none, so this
  is asserted-but-unexercised in Plan 6 — documented, tested minimally);
- is the **switch that opts the module into the byte-diff contract** (§3.3).
Honest scope note: for a data file the observable byte effect of `@as_compat` is
*nil* (data emission is already AS-faithful via §4.3). Plan 6 wires the attribute
and proves it is byte-neutral on the data path; its load-bearing effect (width/lint
pinning on instruction-bearing ports) rides later milestones.

### 3.3 Per-file byte-diff harness
Extend the `sigil diff` surface (or add a harness entry point) that, for one file:
1. assembles the AS original **in isolation** via `sigil-frontend-as` (supplying any
   constants it needs, e.g. via `defines`/a minimal include-root) → reference bytes;
2. compiles the `.emp` port via `sigil-frontend-emp` → candidate bytes;
3. byte-compares; on divergence reports the first differing offset mapped to a span.
Cross-checked against the file's slice of the reference `aeon/s4.bin` where a stable
ROM offset is derivable from the symbol table (belt-and-suspenders).

### 3.4 Minimal placement
The ported module declares its region (`module … in <section>` / a `section (vma:)`);
Plan 6 uses a **minimal, hardcoded** placement to land the section at a real LMA —
enough to show composition. The general scan-and-index manifest + map-file placement
(S2-D3) is **deferred** (§7).

## 4. First-port target

**Primary: a pure-`dc.b` data blob — `data/sound/song_drumtest.asm` or
`data/sound/sfx/sfx_33.asm`.** Portable *today*: opaque byte tables + a start/end
label + `align 2`, no symbol differences, no comptime. Real files, referenced by a
pointer table (the cross-file link-seam consumer). The implementation's first task
confirms which gives the cleanest single-consumer link-seam demo.

**Explicitly NOT the first target (and why it matters):** `particle_anims.asm`,
`test_mappings.asm`, `sonic_anims.asm` — all use the offset-table pattern
(`dc.w Frame - Base`) that `.emp` cannot express yet (§2). This is the key empirical
finding: the offset-table isn't merely the most common idiom (14,173 uses in S3K) —
it is the concrete **blocker** for porting 3 of the 4 real data files examined. It is
the #1 Plan 7 item. Plan 6 documents this cleanly rather than working around it.

## 5. Design decisions (to be recorded as D-P6.x in the plan doc)

- **D-P6.1** Plan 6 proves the mechanism on a portable-today pure-byte file; it does
  NOT implement the offset-table / label-difference-in-data feature (that is Plan 7).
- **D-P6.2** Mixed build = concat `Vec<Section>` + one link. No full-ROM in-place
  splice; no new link infrastructure.
- **D-P6.3** `@as_compat` is wired in lowering (reads `File.attrs`), proven
  byte-neutral on the data path; width/lint pinning asserted but unexercised by a
  data target.
- **D-P6.4** Per-file byte-diff assembles the AS original in isolation as the
  reference (with a ROM-slice cross-check where a stable offset exists).
- **D-P6.5** Minimal hardcoded placement; scan-and-index manifest + map-file
  placement (S2-D3) deferred to the post-Plan-6 language-completion work.
- **D-P6.6** No follow-ups (reserve math/as, salvador port, Plan-4 pool) land first.

## 6. Acceptance criteria

- A real Aeon data file compiles via `sigil-frontend-emp` to bytes **byte-identical**
  to its AS-assembled original (per-file diff green).
- The ported module + an AS consumer link into one image with the ported symbol
  resolving across the seam; a deliberate cross-section name collision errors.
- `@as_compat` is read in lowering and proven byte-neutral on the data path.
- Green gate throughout: `cargo test --workspace` + `cargo clippy --workspace
  --all-targets -- -D warnings`; the s4.bin harness (`m1d_rom` etc.) stays green.
- Whole-branch adversarial review byte-diffs the port against the AS reference.

## 7. What Plan 6 does NOT do — the road after (handoff to Plan 7 + migration)

Plan 6 is a proof-of-concept, **the start of the migration era, not the end.** Scoped
out, in dependency order:

1. **Offset-table / label-difference-in-data** (#1) — blocks representative data
   ports; #1 by frequency. Plan 7 implementation.
2. **Scan-and-index manifest + map-file placement + game prelude (S2-D3)** — the
   composition spine; unblocks porting anything with code.
3. **Branch relaxation** — `jbra`/`jbsr`, conditional-branch relaxation, `jbcc`
   (see the `jbra-jbsr-auto-reaching-branches` decision).
4. **Script DSLs, typed PLC lists** — the other data-table constructs.
5. **Plan-4 pool** — SST-overlay field access, proc-name-as-pointer, symbolic
   straight-line operands, `ProvFrame::Comptime`, `patch`/`bind` surface,
   `CodeItem::Inline`.
6. **Pure-Rust salvador port.**

**Plan 7 = language finalization (research + implementation).** Its *research* half
(done overnight after Plan 6) mines all local disassemblies + online sources for any
further idioms, validates the candidate feature set, and closes the deferred ledger,
producing a frozen `SIGIL_SPEC2_LANGUAGE.md`. Its *implementation* half builds the
finalized features. After that: the migration campaign (68k source first, cycle-exact
Z80 DAC driver last), then **Spec 5** deletes the AS front-end — whose gate is
literally "every load-bearing AS feature has a byte-exact Spec-2 equivalent."
