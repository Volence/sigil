# Design — Sigil Spec 2 · Plan 7 #4: Module Resolution + Placement + Prelude (S2-D3)

**Date:** 2026-07-07 · **Author:** Opus (implementer) · **Reviewer:** Fable
**Spec anchors:** `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` §3 (Modules), §7 (Sections & dual-CPU), §4.7 (offsets cross-module deferral), S2-D3 ledger row (line 507).
**Predecessor handoff:** `docs/superpowers/notes/2026-07-07-spec2-plan7-item4-module-resolution-handoff.md`

## What this milestone turns `.emp` into

Today `sigil emp <file.emp>` compiles exactly **one** file: parse → `lower_module` → `resolve_layout` → `link`, both symbol tables empty. This milestone makes `.emp` a real **multi-module** assembler — the gate that lets `examples/pitcher_plant.emp` (and any real Aeon subsystem) actually lower and link instead of just parse. It replaces AS's ordered `include` chain with a proper module system: membership, placement, and cross-module reference pulled apart into three explicit mechanisms (as the `examples/main.emp` mock already illustrates).

Scope decided with Volence (2026-07-07): **full T1–T5** in one branch (not a thin slice), and **the implementer drafts the prelude `.emp`** with contents reviewed at the milestone checkpoint. Volence defers technical calls; bar is best-in-class; checkpoint at the milestone boundary before any merge.

## Verified starting state (grepped 2026-07-07, code is authoritative)

These facts were re-confirmed against the current tree, not taken from the handoff's memory:

- **`use` parses, never resolves.** `Item::Use(UseDecl { base: Path, names: UseNames, span })` exists (`crates/sigil-frontend-emp/src/ast.rs:76`); lowering has no `Item::Use` arm — it falls through the `_ => {}` catch-all (`lower/mod.rs`). Zero hits for `UseDecl`/`prelude` in `lower/*.rs`.
- **`module … in <section>` is ignored.** `ModuleDecl.in_section: Option<String>` exists (`ast.rs:42`); zero reads of `in_section` in lowering. Single-file lowering routes all items into a lazily-opened default `"text"` section.
- **`sigil emp` is single-file.** `run_emp` → `compile_emp` (`sigil-cli/src/main.rs`): `parse_str(src)` → `LowerOptions { initial_cpu: M68000, include_root }` → `lower_module` → `resolve_layout(&sections, &SymbolTable::new(), true)` → `link(&resolved, &SymbolTable::new())`. Both symbol tables freshly empty.
- **The link seam is already multi-module.** `link(sections: &[Section], stubs: &SymbolTable)` (`sigil-link/src/lib.rs:50`) builds **one flat symbol table across all sections** (pass 1: stubs then every section's labels at `vma_origin + offset`; pass 2: resolve every fixup). Cross-module build = concat `Vec<Section>` from N modules + one `link`. **No linker change required.**
- **`LowerOptions` has exactly two fields:** `initial_cpu: Cpu`, `include_root: Option<PathBuf>`.
- **The "many inputs, one link" precedent exists** in the AS path: `assemble_root` (`sigil-frontend-as`) walks the `main.asm` include tree with a `visited` DAG guard, `next_lma` places sections sequentially in declaration order, then `resolve_layout` + `link`. `run_build`/`run_diff` drive it.
- **A map file already exists and is live:** `sigil.map.toml` — `[[region]]` entries `{ name, lma_base, size, kind, vma_base }`, loaded by `sigil_link::load_map` → `MemoryMap { regions, fill }`, consumed by `sigil_link::emit_rom` to validate section placement against region bounds. T4 **extends** this, it does not invent a format.

### The hazard that shapes the whole design

The flat link symbol table is keyed by **bare label name**. Single-file, nothing collides. Multi-module, two modules that each define a private `init` / `ani_idle` / top-level label **collide in the flat table**. Solving this is the actual core of the milestone. Proc-local `.dotted` labels are *not* the problem — they already ride `SymbolTable::resolve(name, scope: Option<&str>)`. Only **top-level** module names need resolving.

## Architecture: Approach A — front-end resolution + mangling (chosen)

All new logic lives in `sigil-frontend-emp`. `sigil-link` and the AS oracle stay byte-for-byte unchanged, preserving the verified thesis and the s4.bin harness.

**Rejected — Approach B (scope-aware linker):** extend `SymbolTable`/`link` with per-module scopes. More invasive, touches the load-bearing linker, risks the AS byte-exact path, and contradicts the front-end-only thesis. Not chosen.

### The resolution pass (the core)

A new pass runs after file-gathering, around lowering, before sections reach `link`:

1. **Gather / manifest.** Given an entry `.emp` and a `--root` dir, scan the root for all `.emp` files, parse each, index by its `module <dotted.path>` header. Emit the §3.1 path/dir-disagreement **lint** (not an error — keeps file moves cheap). This index is the manifest.
2. **Reachability.** From the entry module, follow `use` edges plus the always-implicit prelude to compute the build set. `use a.b.c` resolves to the module *indexed as* `a.b.c` (by header, so a moved file still resolves; the lint flags drift).
3. **Per-module resolution environment.** Each module's visible-name map = its own top-level defs + names from its `use a.b.{X,Y}` / `use a.b.*` globs + the prelude's `pub` names. **Precedence: local > explicit `use` > prelude.** Two globs importing the same name = error naming both sources.
4. **Mangling.** `pub` top-level labels → a canonical exported name; a `pub` name defined by two modules is an error. Non-`pub` top-level labels → a module-unique mangled name. Proc-local `.dotted` labels are untouched (existing scope seam).
5. **Reference rewrite.** Every symbolic fixup target — `pitcher_plant.init`, imported `Draw_Sprite`, prelude `ObjectMove`, and `offsets` member targets — rewrites to its canonical/mangled name before sections hit `link`. An unresolved name → error carrying a machine-applicable **"add `use a.b.{Name}`" fix-it** (tenet 2, §9).
6. **Concat + link.** Collect every module's lowered `Vec<Section>`, concatenate, hand to the existing `resolve_layout` + `link`.

**Where the resolution env lives** (the one open structural choice, resolved in T0): a program-wide pre-lower pass that builds the name→canonical map and hands each module its resolved environment, rather than bolting a field onto `LowerOptions`. Confirmed in T0 by spike.

**Cycles (§3.2).** Module cycles are **legal for symbols** (link-resolved) and **illegal for comptime values** (evaluation must be a DAG). The gather/reachability graph permits cycles; only the comptime-dependency graph rejects them, reported with the offending chain.

### CLI surface

Extend `sigil emp` to accept an entry `.emp` file plus `--root <dir>` (the scan root for sibling modules; defaults to the entry file's directory) and `--prelude <module.path>`, alongside existing `--map` / `-o` / `--hex`. **Single-file mode stays working unchanged** — no `use`, no sibling modules ⇒ a degenerate 1-module build through the same path.

## Placement (T4)

Unify the two placement concepts that already exist; invent no third:

- `.emp` `section obj_bank (cpu: m68k, vma: $010000)` declares a section and its **VMA**. A top-level module owns the section declarations (as `examples/main.emp` mocks); other modules route into them via `module … in obj_bank`.
- `sigil.map.toml` `[[region]]` supplies **LMA** base, `size` (budget), kind; `emit_rom` already validates against it.

T4 wires `module … in <section>` so each module's items land in the named section, places sections in declared order, and lets the existing region-budget check emit the §7.3 **"over by N bytes"** overflow error with the owning section. The hand-rolled `__BUDGET_*` accounting becomes a free per-section size report.

## Prelude (T3)

One module is named the prelude in the build config; its `pub` names auto-import into every module (step 3, lowest precedence). The implementer drafts `prelude.emp`, scoped to what earns its place — grounded in the brainstormed domain types ([[emp-sonic-newtype-candidates]]) and the NESHLA lesson (ship VDP/DMA/Z80/bank helpers, not only object names). For the acceptance exhibit it carries the object-system vocabulary `pitcher_plant.emp` uses: `Sst` + fields, `ObjDef` / `ArtTile` / `Collision` / `Size` / `Vel` / `Vec`, and the `spawn` / `anim` / `routine` / `facing_abs` / `ObjectMove` / `Draw_Sprite` helpers, plus the domain newtypes. **Contents are a game-authoring decision** — the draft is *proposed*, reviewed at the checkpoint, and nothing about the prelude is compiler magic beyond the auto-import.

## Task breakdown

All tasks: isolated git worktree `sigil/.worktrees/<branch>`, subagent-driven, TDD, commit-per-task. Two-stage review (spec-compliance then code-quality via `superpowers:code-reviewer`) on load-bearing tasks.

- **T0 — Readiness spike (short).** Confirm mangling leaves `link`/AS-oracle untouched; settle the resolution-env home (program-wide pre-lower pass vs. `LowerOptions` field). Mostly pre-confirmed by the 2026-07-07 code audit; T0 records the confirmation + the one structural choice.
- **T1 — Gather / manifest.** Scan root, index by `module` header, path/dir-disagreement lint, comptime-cycle detection with chain.
- **T2 — `use` resolution + mangling + reference rewrite.** Precedence rules, glob-collision error, the "add `use`" fix-it. Folds in the §4.7 offsets cross-module-target deferral.
- **T3 — Prelude auto-import + drafted `prelude.emp`.**
- **T4 — `module … in <section>` placement + region-budget overflow.**
- **T5 — Corpus + adversarial review.** `pitcher_plant.emp` compiles end-to-end; a hand-built multi-module program (object module + prelude + shared-data module) byte-checked against the AS reference where a byte argument exists; whole-branch adversarial review that constructs and runs cross-module programs.

## Green gate & process (non-negotiable)

- Before **every** commit: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings`. Keep the s4.bin harness (`m1d_rom` / `m1d_debug_rom`) green.
- Ground semantics in the spec, but where spec and code disagree, **the code is authoritative** — verify by grep.
- **No merge to master without a Volence checkpoint:** present summary + gap list; Volence chooses `--no-ff` merge+push or changes-first.

## Acceptance criteria

- `sigil emp` compiles `examples/pitcher_plant.emp` end-to-end (with a prelude + the sibling modules supplying `Player_1`, `Map_PitcherPlant`, `VRAM_PITCHER_PLANT`), producing bytes.
- A hand-built 3-module program (object + prelude + shared-data) links and is byte-checked against an AS reference where one exists.
- `use` resolution errors carry the machine-applicable "add `use`" fix-it.
- Section-budget overflow reports "over by N bytes" naming the owning section.
- `sigil-link` and the AS front-end are unmodified; s4.bin harness green.
- Concept inventory (§10) does not grow without a recorded decision (A-Spec2.3).

## References

- Spec: `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` §3, §7, §4.7, S2-D3 row.
- Mocks (design intent, already in tree): `examples/main.emp`, `examples/composition_pitcher_plant.emp`.
- Acceptance exhibit: `examples/pitcher_plant.emp`.
- Memory: [[spec2-progress]], [[emp-sonic-newtype-candidates]], [[emp-data-table-dsl-candidates]], [[fable-role-reviewer-spec-writer]], [[user-defers-sigil-technical-calls]].
