# Spec 2 · Plan 7 #7-pre — the L-H.1 final-size placement fix: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Section placement tells the truth (design D7.4): a chained section's final base derives from its predecessors' FINAL (post-relaxation) extents; pins stay pins; any surviving overlap is a loud link error — never silent image corruption. This is the correctness prerequisite for #7-main (`bank:`/`bankid()`), which computes bank ids from final addresses.

**Architecture:** Placement moves to ONE link-time pass, joint-fixpointed with relaxation. Every `ir::Section` gains explicit placement provenance (`Pinned` vs `Chained`) plus the span its original placer reserved for it. The link pass walks sections in program order keeping per-group cursors: a `Pinned` section resets its group cursor to its baked lma; a `Chained` section lands at the cursor; the cursor then advances by `max(reserved_span, final_size)`. That single formula makes the pass degenerate BYTE-IDENTICALLY to today everywhere today is sound — the multi-module max-span paths (`place_sections`/`place_sequential`, whose gap behavior is pinned in `module_resolution.rs`) always have `final ≤ reserved`, and the baseline-baked chains (`next_lma`/`phys_base`) match exactly when nothing grows — while correcting exactly the understatement class (growth past baseline = today's silent overlap). Placement moving can change cross-section branch distances, so placement⇄relaxation iterate to a joint fixpoint (termination: rungs are grow-only and bounded; placement is a deterministic function of rungs + pins).

**Tech Stack:** crates `sigil-ir` (Section field), `sigil-link` (the pass + fixpoint, lib.rs/relax.rs), `sigil-frontend-emp` (lower/mod.rs provenance + resolve/mod.rs placers), `sigil-frontend-as` (eval.rs provenance), `sigil-cli` (link tails). Tests: new `crates/sigil-link/tests/final_placement.rs`, new `crates/sigil-cli/tests/placement_fix.rs` (program-path, CLI-invoked — the #9 unit-test blind spot watch-out), harness = regression net.

**Ratified basis:** design doc `docs/superpowers/specs/2026-07-08-spec2-plan7-item7-banks-design.md` (APPROVED; D7.4 is this plan's mandate). Technical calls delegated and frozen below as R7p.1–R7p.8.

**Worktree:** `/home/volence/sonic_hacks/sigil/.worktrees/plan7-item7-banks` (branch `plan7-item7-banks`). Notes file (RED evidence, house format): `docs/superpowers/notes/2026-07-08-item7-implementation-notes.md` (new).

---

## Verified code facts this plan stands on (T0 recon, 2026-07-08, master dfe6e7b)

- `IrBuilder` cursor counts relaxables at their BASELINE rung: `emit_fragment(JmpJsrSym, advance = 4)` (lower/code.rs:150; jmp abs.l is 6). So the baked chains understate under growth.
- emp lower: `next_lma += builder.current_offset()` then `switch_section_lma(&sec.name, cpu, Some(vma), next_lma)` (lower/mod.rs:228–231); `section_attrs` (mod.rs:599–635) defaults `vma = 0` and ALWAYS passes `Some(vma)` for named sections.
- AS front-end: identical chain via `phys_base` (frontend-as eval.rs:183–190, 1728–1756; `switch_section_lma(..., Some(vma_base), self.phys_base)` at :1742).
- Multi-module CLI: `place_sections` (resolve/mod.rs:330–366, per-region cursor advancing by `placement_span()`) on the `--map` path (main.rs:419); `place_sequential` (resolve/mod.rs:368–383, global cursor from 0) on the no-map path (main.rs:438). `placement_span()` (ir lib.rs:234–258) counts relaxables at MAX rung → `final_size ≤ placement_span` always. The resulting gaps are PINNED behavior (module_resolution.rs:36–52, :93–105, :893–904 — comments spell the 2-byte gaps out).
- Single-file CLI (`link_to_image`, main.rs:200–214) and the harness AS path (resolve_layout → link → emit_rom) do NOT run any placer — baked chains survive to the linker. The no-map single/program flatten is UNCHECKED (`flatten`, link lib.rs:415–432); only `emit_rom` uses `flatten_checked`.
- `resolve_layout` (relax.rs:234–571) treats `sec.lma`/`vma_base` as FIXED inputs; labels shift within sections; the symbol table rebuilds each pass from `vma_origin() + shifted offset`. `vma_origin() = vma_base.unwrap_or(lma)`.
- Section order = program vec order (BFS discovery; declaration order within a module). No reordering anywhere.
- Green gate at T0: exactly the 4 allowlisted sigil-harness reds (aeon strlen drift; match on exact names), clippy `-D warnings` clean.

## Design rulings frozen by this plan (R7p.1–R7p.8 — spec review scrutinizes these)

- **R7p.1 — placement provenance on `ir::Section`.** New fields (exact shape is the implementer's within this semantic):
  ```rust
  pub enum SectionPlacement { Pinned, Chained }
  pub struct Section {
      ...,
      pub placement: SectionPlacement, // how this section's BASE derives at link
      pub reserved_span: u32,          // the span its placer reserved for it
      pub group: Option<String>,       // placement group (region name under --map); None = the anonymous group
  }
  ```
  Set by: emp lower (first section of a module-lowering run — i.e. the first `switch_section_lma` — `Pinned` at its baked lma; every subsequent one `Chained`; `reserved_span` = the baseline length the old `next_lma` advance would have used); AS eval (same rule over `phys_base`); `place_sections` (first section placed in each region `Pinned` at `region.lma_base`, subsequent in-region `Chained`, `reserved_span = placement_span()`, `group = Some(region.name)`); `place_sequential` (first `Pinned` at `base`, rest `Chained`, `reserved_span = placement_span()`). Multi-module `build_program` concatenation: the FIRST section of every non-entry module is `Chained` too once a placer runs (the placer overwrites provenance — placers are now provenance-writers, not lma-writers; baked lma remains the `Pinned` anchor value only).
- **R7p.2 — the link-time placement pass.** New `sigil-link` fn (called from inside the joint fixpoint, R7p.3): walk sections in vec order with a cursor per `group`; `Pinned` → `base = sec.lma`, group cursor := base; `Chained` → `base =` group cursor; then cursor := `base + max(reserved_span, final_size(sec, rungs))`. `final_size` = the relaxed image extent (same arithmetic as `frag_len` sums under current rungs, honoring `Org` extent like `placement_span` does). The pass REWRITES `sec.lma`. (#7-main adds the bank-bump between "base =" and "cursor :=" — leave a marked seam.)
- **R7p.3 — the joint fixpoint.** Outer loop: placement pass → relaxation (rung selection with CURRENT origins) → if any rung grew, repeat. Rung state persists across outer iterations and is grow-only (never reset — the existing ladder's own invariant). Termination: rungs are bounded above and monotone; once rungs are stable, one final placement is deterministic. Cap iterations at a generous bound (e.g. 64) with an internal-error diagnostic if exceeded (cannot happen by the monotonicity argument — the diagnostic is the honesty backstop, same spirit as D-H.6's Poison arm). `resolve_layout`'s signature grows to accept/return what the orchestration needs; the AS `dash_a` flag threads through unchanged.
- **R7p.4 — overlap = loud link error, all paths.** After the fixpoint, any two non-empty placed sections whose `[lma, lma+final_size)` ranges intersect → Error diagnostic naming both sections and both extents. The no-map paths switch from unchecked `flatten` to this check (the check subsumes `flatten_checked`'s job; `emit_rom` keeps its region validation). By construction chained sections cannot overlap; this catches colliding pins.
- **R7p.5 — the vma default follows placement.** Named emp sections WITHOUT an explicit `vma:` attr now bake `vma_base = None` (labels resolve from final LMA — same semantics default sections already have) instead of today's implicit `Some(0)`. An explicit `vma:` stays a pin (unchanged). This is required for #7-main (`bankid(Label)` reads the symbol = VMA; for ROM-resident data VMA must track the placed LMA) and retroactively strengthens labels per D7.4. Byte-impact bar: the corpus probe (T7) must show zero diffs, or every diff itemized as a correction (a named no-vma section whose labels silently resolved from 0 while placed elsewhere IS the silent-wrong class).
- **R7p.6 — degeneracy bar (byte-identity).** When no rung grows past baseline in a chained group — and ALWAYS in max-span groups — output is byte-identical to master. Evidence: the s4 harness stays at exactly the 4 allowlisted reds; the full corpus byte-diff probe (T7) is clean except itemized corrections. The module_resolution.rs gap pins stay GREEN UNTOUCHED.
- **R7p.7 — `Value`-audit discipline.** The `Section` field additions force compiler-driven audits of every construction site (builder.rs close(), relax.rs's rebuild at :513–571, tests) — fix them all; no `..Default::default()` escape hatches that would silently mis-provenance a new call site.
- **R7p.8 — out of scope.** No placement policy changes beyond the formula (no gap compaction — max-span spacing is sound and pinned); no `org`-pin rework; no Z80 ladder; the bank bump itself is #7-main (only its seam is left here).

---

### Task 0: Baseline probes + notes file

**Files:**
- Create: `docs/superpowers/notes/2026-07-08-item7-implementation-notes.md`
- Create: `scripts/corpus_bytediff.sh` (worktree-local helper; NOT committed if the house prefers — commit it, it's the review probe #9 lacked)

- [x] **Step 1:** Create the notes file with the here-fix format header (see `2026-07-08-item9b-implementation-notes.md` for shape): worktree, branch, T0 evidence (4 allowlisted reds + clippy clean), and an empty "RED evidence" table.
- [x] **Step 2:** Write `scripts/corpus_bytediff.sh`: builds every `examples/*.emp` single-file (`sigil emp <f> -o …`) and the two standing game invocations (`--root examples/game --prelude prelude` for `badniks/pitcher_plant.emp` and `badniks/pitcher_plant_script.emp`) from BOTH the worktree and a pristine master checkout (use `git worktree` or `git -C … stash`-free build via the main checkout), then byte-diffs each pair and prints a per-file verdict. Skip files master fails to build.
- [x] **Step 3:** Run it worktree-vs-master at HEAD==master; expect all-identical (sanity that the probe itself works). Record in notes.
- [x] **Step 4:** Commit: `test(7-pre): corpus byte-diff probe + notes scaffold`.

### Task 1: The RED repro — today's silent overlap, recorded

**Files:**
- Create: `crates/sigil-cli/tests/placement_fix.rs`

- [x] **Step 1:** Write the failing test `single_file_growth_overlap_is_fixed` (program-path: spawn the CLI like module_resolution.rs does). Source shape (single file, two sections — the baked `next_lma` chain path):
  ```
  module m
  section code (vma: $8000) {
      proc p (a0: *u8) {
          jmp p
      }
  }
  section data {
      data Tail: [u8; 4] = [$DE, $AD, $BE, $EF]
  }
  ```
  `jmp p` targets VMA $8000 → asl width rule picks abs.l (6 bytes) — 2 bytes past the baseline-4 the `next_lma` chain counted, so on master `data`'s baked lma overlaps `code`'s last 2 bytes and unchecked `flatten` writes `$DE $AD` over the jmp operand's tail. Hand-derive BOTH images in the test comment (master's corrupt 8-byte image vs the correct 10-byte one: `4E F9 00 00 80 00  DE AD BE EF`) and assert the CORRECT bytes.
- [x] **Step 2:** Run it; confirm it FAILS on the branch (which is still master-identical) with the corrupt image. Paste the failure into the notes file's RED table.
- [x] **Step 3:** Commit: `test(7-pre): RED repro — baseline-chained section silently overlapped under jmp growth`.

### Task 2: `ir::Section` placement provenance

**Files:**
- Modify: `crates/sigil-ir/src/lib.rs` (Section struct, ~:200), `crates/sigil-ir/src/builder.rs` (OpenSection + close() + switch_section_lma)
- Test: `crates/sigil-ir/src/builder.rs` unit tests

- [x] **Step 1:** Failing unit test: two `switch_section_lma` calls on one builder yield sections `[Pinned, Chained]` with `reserved_span` = each section's cursor length at close and `group == None`.
- [x] **Step 2:** Add `SectionPlacement`, the three fields (R7p.1), thread through `OpenSection`/`close()`. First `switch_section_lma` on a builder → `Pinned`; subsequent → `Chained`. `reserved_span` = closing cursor (`max_offset`). Let the compiler drive every other construction site (relax.rs :513–571 rebuild PRESERVES placement/reserved_span/group verbatim; test builders).
- [x] **Step 3:** Workspace builds + test passes. Commit: `feat(7-pre): Section placement provenance (Pinned/Chained + reserved_span + group)`.

### Task 3: Placers become provenance-writers

**Files:**
- Modify: `crates/sigil-frontend-emp/src/resolve/mod.rs:330–383` (place_sections, place_sequential)
- Test: existing resolve tests + new unit tests alongside

- [x] **Step 1:** Failing tests: (a) `place_sequential` marks first `Pinned(base)`, rest `Chained`, all `reserved_span = placement_span()`, `group = None`; (b) `place_sections` sets `group = Some(region)` and per-region first-`Pinned`-at-`lma_base`.
- [x] **Step 2:** Implement. KEEP the lma-writing behavior for now (the link pass takes over in Task 4; green gate must hold at every commit).
- [x] **Step 3:** Full `cargo test --workspace` — only the 4 allowlisted reds + the Task-1 RED. Commit.

### Task 4: The link-time placement pass + joint fixpoint

**Files:**
- Modify: `crates/sigil-link/src/relax.rs` (resolve_layout orchestration), `crates/sigil-link/src/lib.rs` (overlap check, flatten call sites)
- Create: `crates/sigil-link/tests/final_placement.rs`
- Modify: `crates/sigil-cli/src/main.rs:200–214, :395–449` (both tails call the new seam)

- [x] **Step 1:** Failing linker-level tests (build Sections by hand): (a) chained section after a JmpJsrSym section whose rung grows → successor base = pred base + 6 (not 4); (b) max-span provenance (`reserved_span` = 6, final 4) → successor stays at +6 (degeneracy); (c) two Pinned sections colliding → Error naming both; (d) fixpoint: growth caused BY re-placement (a jmp whose target crosses $8000 only after its section moves) converges and both effects land.
- [x] **Step 2:** Implement R7p.2 + R7p.3: placement fn, outer fixpoint (persisted grow-only rungs), overlap check (R7p.4), leave the `// #7-main: bank bump seam` marker. Wire both CLI tails + the harness path through the new orchestration (the old direct `resolve_layout` callers migrate; keep the public API story coherent).
- [x] **Step 3:** Task-1 RED goes GREEN. Run the FULL gate: workspace tests (exactly 4 allowlisted reds), clippy clean, `scripts/corpus_bytediff.sh` (all identical except the Task-1 class — none expected in the shipped corpus), module_resolution gap pins untouched.
- [x] **Step 4:** Record evidence in notes; commit: `feat(7-pre): final-size placement — link-time pass + placement⇄relaxation joint fixpoint (D7.4)`.

### Task 5: AS front-end provenance + s4 byte-identity

**Files:**
- Modify: `crates/sigil-frontend-as/src/eval.rs:1728–1756`

- [x] **Step 1:** Set provenance at the AS `switch_section_lma` site per R7p.1 (first `Pinned`, rest `Chained`, `reserved_span` = the `phys_base` advance).
- [x] **Step 2:** Run the harness (`cargo test -p sigil-harness`): EXACTLY the same 4 allowlisted reds, every other test green — the m1b/m0/m1c placement-sensitive pins are the proof the joint fixpoint degenerates on the AS path. Any NEW red = stop, diagnose, never allowlist.
- [x] **Step 3:** Commit: `feat(7-pre): AS front-end placement provenance — s4 path byte-identical`.

### Task 6: The vma-follows-placement default (R7p.5)

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs:599–635` (section_attrs) + :231 (the Some(vma) pass-through)
- Test: `crates/sigil-cli/tests/placement_fix.rs`

- [x] **Step 1:** Failing CLI test `named_section_labels_follow_placed_lma`: two modules, second holds `section blob { pub data X … }` (NO vma attr), entry jmp-references a proc after it; assert X's emitted address (via a pointer cell to X in the entry) equals its PLACED address, not 0.
- [x] **Step 2:** `section_attrs` returns `Option<u32>` for vma (None when absent); thread `Some(vma)` → attr-present only. Default section behavior unchanged (verify what `ensure_default` passes today and pin it in a test).
- [x] **Step 3:** Full gate + corpus byte-diff. Any diff = itemize in notes with the argument (R7p.5 bar). Commit.

### Task 7: Whole-plan gate + review checkpoint

- [x] **Step 1:** Full workspace gate + clippy + corpus byte-diff + harness, all recorded in notes.
- [x] **Step 2:** Two-stage review (spec review against D7.4/R7p.*, then code-quality review) — subagent-driven per house process; controller independently re-runs the probes.
- [x] **Step 3:** Commit any fold-ins; mark this plan's checkboxes; hand off to the 7-main plan.

## Self-review notes (plan author)

- Spec coverage: D7.4 (pass+fixpoint = T4; pins = R7p.1/T4c; loud overlap = R7p.4/T4c; label strengthening = T6), D7.6's byte-identity clause (R7p.6, T0 probe, T4/T5 gates). D7.2's bump lands in 7-main at the T4 seam.
- The riskiest unknown is the AS-path degeneracy (T5): if a harness pin moves, the fixpoint does NOT degenerate — that is a STOP-and-diagnose, likely a `reserved_span` mis-recording, not an allowlist.
- Placeholder scan: task steps carry concrete shapes; exact new-API signatures are implementer latitude WITHIN R7p.1–R7p.4 (recorded rulings, not gaps).
