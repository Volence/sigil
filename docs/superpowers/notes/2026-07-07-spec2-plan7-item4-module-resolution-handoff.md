# Handoff — Sigil Spec 2 · Plan 7 backlog #4: module resolution + placement + prelude (S2-D3)

Orientation for a fresh agent starting the next implementation milestone. Written 2026-07-07 (Fable).
This is the **gate** that turns `.emp` from a single-file compiler into a real multi-module assembler —
it's what makes `examples/pitcher_plant.emp` actually lower/compile instead of just parse.

## Where things stand (Plan 7 so far)
- **Plans 1–6 MERGED.** `.emp` lowers to Core IR end-to-end for a SINGLE file (data + `asm{}`/Code +
  procs + sections + cross-CPU fixups). `sigil emp <file.emp> [-o out.bin] [--hex]` compiles one file.
- **Plan 7 backlog #1 (lexical gaps), #2 (symbolic operands), #3 (offset tables `offsets`) MERGED to
  master.** Master green (~1024 workspace tests, clippy `-D warnings` clean, s4.bin harness intact).
- **Plan 7 spec finalization DONE** (Fable, 2026-07-06, in `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`,
  **UNCOMMITTED** in the empyrean working tree per Volence's docs cadence — empyrean is ~38 commits
  ahead of origin): §4.7 offsets, D2.15–D2.19 (offsets/text-encoding/binary-literals/jbra-jbsr/
  comments), reserved-word policy closed (§10), Part-II research dispositions in the deferred ledger
  (S2-D9/D10/D11). Read §3 (Modules) + §7 (Sections) + the S2-D3 ledger row before starting.

## What #4 IS (S2-D3)
Three layers of ONE problem — cross-module name resolution, which does not exist today:
1. **Scan/manifest module resolution** — discover the `.emp` files in a build, index each by its
   `module <dotted.path>` header, and resolve `use a.b.{X, Y}` / `use a.b.*` + prelude auto-imports
   so a name defined in another module resolves. Replaces AS's ordered `include` chain (spec §3.2).
2. **Map-file placement** — consume `module ... in <section>` + a map/build config that names section
   ordering and region bases/budgets; place each section's LMA accordingly (spec §3.3, §7.1, §7.3).
   Today the single-file path links with an empty symbol table and no placement manifest.
3. **Game prelude** — one module named as the prelude; its `pub` names auto-import into every module
   (spec §3.4). This is ~one config line + an implicit `use` ONCE layers 1–2 work. The prelude is
   **data, not language** — its CONTENTS are a game-authoring decision (see design inputs below).

**Why it's the top unblocker:** it's the difference between "port data files" and "port a real game."
The offset-table cross-module/multi-segment-target deferral (§4.7) folds into this — a table pointing
at another module's data is just one more name resolution has to find.

## Verified code facts a fresh agent MUST know (grepped 2026-07-07, not from memory)
- **`use` PARSES but is NEVER RESOLVED.** `Item::Use(UseDecl { base: Path, names: UseNames, span })`
  exists in `ast.rs:78`; grep for `UseDecl`/`prelude` in `crates/sigil-frontend-emp/src/lower/*.rs` =
  **zero hits**. Lowering ignores `use` entirely today. This is the core gap to close.
- **`module ... in <section>` parses into `ModuleDecl.in_section: Option<String>`** (`ast.rs:38`) but
  the single-file lower path does not consume it for placement.
- **`sigil emp` compiles ONE file** (`crates/sigil-cli/src/main.rs`, `run_emp`→`compile_emp_source`
  ~line 156): `read_to_string(input)` → parse → `lower_module(&file, &opts)` → `resolve_layout(...,
  SymbolTable::new(), true)` → `link(&resolved, &SymbolTable::new())`. Both symbol tables are EMPTY —
  no gathering of sibling modules.
- **The LINK SEAM already supports multi-module** — `link(sections: &[Section], stubs: &SymbolTable)`
  (`sigil-link/src/lib.rs:50`) builds ONE flat symbol table across ALL sections (pass 1: stubs, then
  each section's labels). So **multi-module build = concat `Vec<Section>` from N lowered modules + one
  `link`** — Plan 6 (T4) already proved a cross-module reference resolves through this. The missing
  piece is entirely FRONT-END: gather files, resolve `use`/prelude names to symbols, assign sections
  to regions. No linker change should be needed (verify this assumption early — it's D-P?.1 material).
- **`lower_module(file, opts)`** takes `LowerOptions { include_root, cpu, as_compat, ... }`
  (`lower/mod.rs:68`). `include_root` is the embed/import sandbox root (§6.7), already threaded through
  `Placement`. Module resolution likely needs a NEW opts field (the resolved import environment) or a
  pre-pass that builds a program-wide symbol/name map handed to each module's lower.
- **`build`/`diff --aeon <dir>`** already walk the full aeon `main.asm` include tree via the AS
  front-end (`run_build`/`run_diff`) — the mixed-build precedent for "many inputs, one link" lives
  there; study it for the file-gathering + region-placement pattern.

## Design inputs ALREADY GATHERED (don't re-derive — #4 walks in with these decided)
- **Prelude CONTENTS (the open part of S2-D3):** the Sonic/Aeon domain-type vocabulary is brainstormed
  in memory [[emp-sonic-newtype-candidates]] — `Angle=u8`, `SubPixel=fixed<16,16>`/`Speed=fixed<8,8>`,
  `VramTile=u16 where 0..2047` + a typed `vram_bytes` conversion, `Tile`/`Block`/`Chunk`/`Section`,
  palette/collision/sound-id types. Plus the object-system names the pitcher-plant exhibit needs:
  `Sst` + fields, `Draw_Sprite`, `ObjectMove`, `ObjDef`/`ArtTile`/`Collision`/`Size`/`Vel`,
  `spawn`/`anim`/`routine`/`facing_abs`. NESHLA lesson: ship VDP/DMA/Z80/bank helpers too, not just
  object names. **Fable offered to draft a prelude `.emp` block** — check with Volence whether that
  exists yet; if not, drafting it is a natural T0.
- **The acceptance target:** `examples/pitcher_plant.emp` (parses today, 18 items). It compiles
  end-to-end THE DAY this milestone lands, with no grammar change. Its `Player_1.x_pos` (absolute,
  cross-module singleton), `timer(a0)` (overlay field), `spawn`/`anim`/`routine` (prelude comptime
  helpers), `Map_PitcherPlant`/`VRAM_PITCHER_PLANT` (cross-module names) are the resolution test set.
  Appendix D in the spec argues its byte layout.
- **Reserved-word / forward-compat policy is CLOSED** (§10) — new constructs enter as contextual item
  openers, so no grammar-breakage risk here.

## Suggested shape (design-first, then confirm with Volence at plan time)
- **T0** — readiness assessment IN CODE (the process demands it; Plans 2/3/4 each found a delta):
  confirm the "concat Vec<Section> + one link, front-end-only" thesis holds; decide where the resolved
  import environment lives (new `LowerOptions` field vs a pre-pass program map); decide the map-file
  format (reuse an existing pattern if one exists — check `build`/`diff --aeon`).
- **T1** — scan/manifest: gather `.emp` files, index by `module` header, detect path/dir disagreement
  (lint not error, §3.1), report module cycles for comptime values (illegal) vs symbols (legal, §3.2).
- **T2** — `use` resolution: resolve `use a.b.{X,Y}` / `.*` names to their defining module's `pub`
  items; unknown-name error carries the machine-applicable "add `use`" fix-it (tenet 2, §9).
- **T3** — prelude auto-import: one config-named module's `pub` names implicitly `use`d everywhere.
- **T4** — map-file placement: `module ... in <section>` + region bases/ordering/budgets → LMA;
  overflow = linker "over by N bytes" (§7.3).
- **T5** — corpus: `pitcher_plant.emp` compiles end-to-end + a multi-module program (obj file + prelude
  + a shared-data module) byte-checked; whole-branch adversarial review.
- Fold in the offsets cross-module-target deferral (§4.7) wherever T2 lands.

## Process (NON-NEGOTIABLE — caught a CRITICAL in Plans 2/3/4)
- Isolated git worktree `sigil/.worktrees/<branch>`; subagent-driven; TDD per task; commit-per-task.
- **Two-stage reviews** (spec-compliance THEN code-quality via `superpowers:code-reviewer`) on
  load-bearing tasks; a **whole-branch adversarial review** at the end that CONSTRUCTS + RUNS
  cross-module programs and byte-diffs against the AS reference wherever a byte argument exists.
- **Green gate before EVERY commit:** `cargo test --workspace` + `cargo clippy --workspace
  --all-targets -- -D warnings`. Keep the s4.bin harness (`m1d_rom`/`m1d_debug_rom`) green.
- Ground semantics in the spec, BUT where spec and code disagree, **the CODE is authoritative** —
  verify by grep (the `ProvFrame::Comptime` lesson: the spec said "reserved", the code never built it).
- **Milestone: do NOT merge to master without a Volence checkpoint** (the established cadence: present
  a summary + gap list, Volence chooses `--no-ff` merge+push or changes-first).
- Fable's role is reviewer/spec-writer, Opus implements ([[fable-role-reviewer-spec-writer]]); Volence
  defers technical calls, checkpoints at milestone boundaries ([[user-defers-sigil-technical-calls]]).

## After #4 — the remaining Plan-7 backlog (for context, not now)
#5 `assert!`/capacity refinements → #6 state-machine + SST overlay → #7 bank/window placement (gates
sound) → #8 jbra/jbsr + branch relaxation (D2.18; `jbcc` deferred) → #9 byte-command DSL / scripted
coroutine → #10 compression builtins. Plus the NEXT natural `offsets` increment: **inline-target
members** (co-located bodies, `Name: [u8;N] = [...]` reusing the data-item shape so the length stays
the terminator guard) — promoted by an outside reader's request, spec'd in §4.7, small sugar over
shipped machinery.

## References
- Spec: `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` §3 (Modules), §7 (Sections), S2-D3 ledger row.
- Prelude type inputs: [[emp-sonic-newtype-candidates]]; exhibit: `examples/pitcher_plant.emp`.
- Prior handoffs: `notes/2026-07-06-spec2-plan7-item3-offset-table-complete-handoff.md` (offsets),
  `notes/2026-07-06-spec2-plan7-implementation-handoff.md` (backlog + Part V solutions).
- Memory: [[spec2-progress]], [[emp-data-table-dsl-candidates]], [[emp-language-design-principles]].
