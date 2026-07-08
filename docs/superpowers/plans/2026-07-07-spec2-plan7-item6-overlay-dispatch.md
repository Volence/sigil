# SST overlay + `dispatch` (Plan 7 backlog #6) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** (A) SST overlay field access as displacement — `vars PitcherPlantV: sst_custom { timer: u8 }` + `timer(a0)` lowering to `$2E(a0)`-class bytes; (B) `dispatch Name (encoding: …) { … }` — the encoding-agnostic typed state-dispatch table. Plus three #5-audit resolver fixes as pre-tasks. Closes Plan 7 backlog #6 (research T1-b + T2-c per R1/R2).

**Architecture:** Overlay field access is **pure comptime**: struct layout is already memoized and `offsetof` ships, so `timer(a0)` resolves to an integer displacement in `map_operand`'s existing `DispInd` arm — no new fragments, no link work. Proc param types get threaded into the asm evaluator (`eval_proc_body` gains a params argument) to type address registers. `dispatch` reuses the shipped `offsets` emission machinery (RelOffset cells) for `word_offsets` and the existing label-pointer data cell for `long_ptrs`; ordinals mirror the `offsets` path in `eval_path`, pre-scaled by encoding.

**Tech Stack:** Rust workspace (`cargo`). Crates: `sigil-frontend-emp` (all feature work), `sigil-cli` (end-to-end tests).

**Green gate before EVERY commit (non-negotiable — note the changed baseline):**
```
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
```
`--no-fail-fast` is REQUIRED: 4 sigil-harness tests are pre-existing red on clean master (upstream aeon sound-driver refactor; AS front-end `strlen` failure): `full_build_reproduces_sound_driver_regions`, `vector_table_matches_reference_rom_first_256_bytes`, `full_debug_rom_matches_assembled_reference`, `full_rom_matches_assembled_reference`. The gate is **zero NEW failures beyond exactly these 4**. Everything else (incl. all of sigil-frontend-emp, sigil-cli) must be green.

**Design doc (authoritative for semantics — read it first):**
`docs/superpowers/specs/2026-07-07-spec2-plan7-item6-overlay-dispatch-design.md` (D6.A1–A10, D6.B1–B6, Part 0 dispositions, and the explicit OUT-of-scope list — do not creep).

**Conventions to respect:**
- Where spec/plan and code disagree, the CODE is authoritative — verify by grep; note drift in the task report, do not redesign.
- Overflow/unknown/ambiguous is an ERROR (totality), never silent. New diagnostics use the bracketed-code style (`[overlay.window-overflow]` etc.) — grep an existing one (e.g. `[offsets.non-68k]`) and mirror the formatting exactly.
- Code blocks below are **EXACT** (verified 2026-07-07/08 against master `d1e4288`) or **MIRROR** (adapt to the cited exemplar's local idiom; the task's test is the contract).
- Byte expectations in tests were hand-assembled and cross-checked; if a test fails ONLY on expected bytes, re-derive before touching the implementation (report it — do not silently change expectations).

---

## File Structure

| File | Change | Responsibility |
|------|--------|----------------|
| `crates/sigil-frontend-emp/src/parser.rs` | modify | Nested-section rejection; dotted window path in `vars`; `dispatch` item + `(encoding:)`; OPENERS. |
| `crates/sigil-frontend-emp/src/ast.rs` | modify | `VarsDecl.region: Vec<String>`; `Item::Dispatch(DispatchDecl)` + `DispatchDecl`/`DispatchEncoding`/`DispatchMember`. |
| `crates/sigil-frontend-emp/src/resolve/mod.rs` | modify | Audit fixes 0b/0c (recurse `ambient_items`/`pub_comptime_name`/`reachable_modules`). |
| `crates/sigil-frontend-emp/src/resolve/imports.rs` | modify | Audit fix 0c (`ResolveEnv::build` use-collection recursion). |
| `crates/sigil-frontend-emp/src/eval/mod.rs` | modify | Index `Item::Vars` overlays + `Item::Dispatch`; `eval_proc_body` gains `params`. |
| `crates/sigil-frontend-emp/src/layout.rs` | modify | `OverlayInfo` lazy layout (window resolution, field offsets, overflow/shadow checks), memoized like `struct_layout_memo`. |
| `crates/sigil-frontend-emp/src/eval/expr.rs` | modify | `offsetof`/`sizeof` on overlays; dispatch ordinals (`Name.Member` scaled, `Name.count`). |
| `crates/sigil-frontend-emp/src/eval/asm.rs` | modify | Register typing from proc params; field-space resolution in the `DispInd` arm (bare + qualified); field-overrun check. |
| `crates/sigil-frontend-emp/src/lower/mod.rs` | modify | `Item::Vars` validation arm (always-on overlay checks, zero bytes); `Item::Dispatch` lowering (both encodings). |
| `crates/sigil-frontend-emp/src/lower/proc.rs` | modify | Pass `proc.params` into `eval_proc_body`. |
| `crates/sigil-frontend-emp/tests/parser_decls.rs` | modify | Parser tests (nested section, dotted window, dispatch). |
| `crates/sigil-frontend-emp/tests/overlay.rs` | create | Overlay layout/diagnostics/access unit tests. |
| `crates/sigil-frontend-emp/tests/dispatch.rs` | create | Dispatch ordinal/lowering/byte tests. |
| `crates/sigil-cli/tests/module_resolution.rs` | modify | Audit-fix regressions (0b/0c); cross-module `pub vars` overlay. |
| `crates/sigil-cli/tests/ports.rs` | modify | End-to-end byte-exact ports; example-file compiles. |
| `examples/sst_overlay.emp` | create | Part-A worked exhibit (compiles end-to-end). |
| `examples/dispatch.emp` | create | Part-B worked exhibit (compiles end-to-end). |

Branch: worktree `sigil/.worktrees/plan7-item6-overlay-dispatch`, branched from `master` (`d1e4288`).

---

## Task 0: Seam re-verification (10 min, no commit)

- [ ] Confirm the seams this plan cites still hold (live-verified 2026-07-07/08 on `d1e4288`):

```bash
grep -n 'DispInd' crates/sigil-frontend-emp/src/eval/asm.rs        # ~254 — displacement eval arm
grep -n 'fn map_operand' crates/sigil-frontend-emp/src/eval/asm.rs # ~218
grep -n 'Disp16An' crates/sigil-frontend-emp/src/lower/code.rs     # ~393 — i16 range check
grep -n 'fn eval_proc_body' crates/sigil-frontend-emp/src/eval/mod.rs   # ~600 — NO params arg today
grep -n 'eval_proc_body' crates/sigil-frontend-emp/src/lower/proc.rs    # ~82 — the caller to update
grep -n 'fn layout_of_struct' crates/sigil-frontend-emp/src/layout.rs   # ~383 — memoized; the exemplar
grep -n 'struct_name_for_offsetof' crates/sigil-frontend-emp/src/layout.rs  # ~242 — bottoms through newtype/refined
grep -n 'overlay region' crates/sigil-frontend-emp/src/parser.rs   # ~654 — expect_ident to replace
grep -n 'const OPENERS' crates/sigil-frontend-emp/src/parser.rs    # ~256 — 15 entries today
grep -n 'fn lower_offsets_item' crates/sigil-frontend-emp/src/lower/mod.rs  # ~306 — word_offsets exemplar
grep -n 'offsets.non-68k' -r crates/sigil-frontend-emp/src/        # the CPU-guard exemplar
grep -n 'lower_section_items' crates/sigil-frontend-emp/src/lower/mod.rs    # ~241 — no Section arm (fix 0a)
grep -n 'fn ambient_items\|pub_comptime_name' crates/sigil-frontend-emp/src/resolve/mod.rs  # fix 0b
grep -n 'fn reachable_modules' crates/sigil-frontend-emp/src/resolve/mod.rs # ~303 — flat use-scan (fix 0c)
```

Also find the label-pointer data cell (the mechanism behind `code: "init"` in struct data —
`grep -n 'Abs32\|LabelRef\|Cell::' crates/sigil-frontend-emp/src/layout.rs crates/sigil-frontend-emp/src/lower/data.rs | head -30`)
and note its name for Task 11. If any seam moved materially, note it in the task report; do not redesign.

---

## Task 1 (audit fix 0a): reject nested `section {}` — `[section.nested]`

From the 2026-07-07 #5-merge audits (both auditors, reproduced): `lower_section_items`
(`lower/mod.rs:241–273`) has no `Item::Section` arm, so a section nested in a section silently
drops everything inside — data bytes, `ensure_fatal` guards, `max_size` checks — while
eval/resolve DO recurse. Decision (design doc Part 0a): nested sections are REJECTED loudly;
placement-within-placement has no ratified meaning.

**Files:** Modify `crates/sigil-frontend-emp/src/parser.rs` (`section_decl`, ~1263, the `item()` loop). Test in `crates/sigil-frontend-emp/tests/parser_decls.rs` + e2e in `crates/sigil-cli/tests/module_resolution.rs` (or the nearest single-file CLI test home — grep `built:` asserts).

- [ ] **Step 1: failing parser test** (MIRROR the neighboring diagnostics-asserting tests in parser_decls.rs):

```rust
#[test]
fn nested_section_is_rejected() {
    let (_f, diags) = parse_str(
        "module m\nsection outer {\n  section inner {\n    data d: [u8; 1] = [$FF]\n  }\n}\n");
    assert!(diags.iter().any(|d| d.message.contains("[section.nested]")),
        "want [section.nested], got: {diags:?}");
}
```

- [ ] **Step 2: run to verify it fails** — `cargo test -p sigil-frontend-emp --test parser_decls nested_section` → FAIL (no diagnostic today).
- [ ] **Step 3: implement** — in `section_decl`'s item loop: when the parsed inner item is `Item::Section(inner)`, emit an error at the inner section's span:
  `[section.nested] section \`{inner.name}\` is nested inside section \`{outer.name}\` — sections do not nest; declare it at module level`
  and DROP the inner item (the loud error makes dropping safe). Everything else unchanged.
- [ ] **Step 4: e2e regression** — CLI test: the audit repro (`ensure_fatal(false, …)` + over-`max_size` data inside a nested section) now produces the `[section.nested]` error instead of `built: 0 bytes` silence. MIRROR an existing error-asserting CLI test.
- [ ] **Step 5: green gate, then commit** — `fix(emp-parse): reject nested section{} — [section.nested], was silently dropped by lowering`

## Task 2 (audit fix 0b): section-nested `pub` comptime defs never injected cross-module

Audit-reproduced: `ambient_items` (resolve/mod.rs ~:50–54, prelude path) and the `use`
Glob/List injection (~:72–89) iterate `file.items` flat. A `pub const`/`struct`/`enum`/
`bitfield`/`newtype`/`comptime fn` nested in a `section {}` is *exported* (faf5191 fixed the
collectors) but its definition is never injected into the consumer — `unknown name` at use.

**Files:** Modify `crates/sigil-frontend-emp/src/resolve/mod.rs`. Test: `crates/sigil-cli/tests/module_resolution.rs`.

- [ ] **Step 1: failing tests** (MIRROR the existing multi-module `--root`/`--prelude` test harness in module_resolution.rs — it already builds tempdir module trees):
  - prelude module with `section s { pub const MAGIC: u8 = $42 }`, consumer uses `MAGIC` in a data item → today `unknown name MAGIC`; want: builds, byte `42`.
  - lib module with `section s { pub struct Pt (size: 2) { x: u8, y: u8 } }`, consumer `use lib.{Pt}` + `data p = Pt{ x: 1, y: 2 }` → today `unknown type: Pt`; want bytes `01 02`.
- [ ] **Step 2: run to verify both fail** with today's exact errors.
- [ ] **Step 3: implement** — recurse into `Item::Section` in the flat loops (single level is sufficient — deeper nesting is now a parse error per Task 1). MIRROR faf5191's recursion shape (`git show faf5191 -- crates/sigil-frontend-emp/src/resolve/imports.rs`).
- [ ] **Step 4: green gate, then commit** — `fix(resolve): inject section-nested pub comptime defs cross-module (prelude + use paths)`

## Task 3 (audit fix 0c): `use` nested in `section {}` silently ignored

Audit-reproduced: all three of `reachable_modules` BFS (resolve/mod.rs:303),
`ResolveEnv::build` (resolve/imports.rs:137), and `ambient_items` scan `Item::Use` flat, so a
section-nested `use helper.{Helper}` is ignored — the diagnostic even *suggests adding the
line that is already present*.

**Files:** Modify `crates/sigil-frontend-emp/src/resolve/mod.rs`, `crates/sigil-frontend-emp/src/resolve/imports.rs`. Test: `crates/sigil-cli/tests/module_resolution.rs`.

- [ ] **Step 1: failing test** — module with `section s { use helper.{Helper} data d: [u8;1] = [Helper] }` (helper module: `pub const Helper: u8 = 7`) under `--root` → today `unresolved name Helper — add use helper.{Helper}`; want: builds, byte `07`. Add a second case where the `use`d name is a proc label referenced by `jmp` (exercises the rename map, not just ambient injection).
- [ ] **Step 2: verify both fail.**
- [ ] **Step 3: implement** — recurse `Item::Section` (single level) in all THREE sites.
- [ ] **Step 4: green gate, then commit** — `fix(resolve): honor use decls nested in section{} (BFS, rename env, ambient injection)`

---

## Task 4: parser — dotted window path in the `vars` overlay form

D6.A1 needs `vars X: Sst.sst_custom { … }` as the ambiguity fix-it. Today `parser.rs:654`
does `expect_ident("overlay region (e.g. sst_custom)")` — single segment only.

**Files:** Modify `crates/sigil-frontend-emp/src/ast.rs` (`VarsDecl.region: String` → `Vec<String>`), `crates/sigil-frontend-emp/src/parser.rs` (~654). Test: `crates/sigil-frontend-emp/tests/parser_decls.rs` (`vars_region_and_overlay_forms`, ~199).

- [ ] **Step 1: failing test** — extend `vars_region_and_overlay_forms`:

```rust
    // dotted window form: `vars X: Sst.sst_custom { timer: u8 }`
    let f = ok("module m\nvars X: Sst.sst_custom { timer: u8 }\n");
    let Item::Vars(v) = &f.items[0] else { panic!() };
    assert_eq!(v.region, vec!["Sst".to_string(), "sst_custom".to_string()]);
```
and update the two existing asserts to `vec!["upper_ram".to_string()]` / `vec!["sst_custom".to_string()]`.

- [ ] **Step 2: verify it fails** (type error / parse error).
- [ ] **Step 3: implement** — `region: Vec<String>`; parse `ident (. ident)*`. More than 2 segments: allow in the parser (resolution errors later name the form). Fix every `v.region` consumer the compiler flags (grep `\.region`).
- [ ] **Step 4: green gate, then commit** — `feat(emp-parse): dotted window path in vars overlay form`

## Task 5: overlay semantics — window resolution, layout, always-on checks (D6.A1/A2/A7/A9)

The core Part-A data model. Overlays get a lazy, memoized layout (MIRROR
`layout_of_struct`/`struct_layout_memo`, layout.rs:383–485) and an always-on validation arm in
lowering (guards precedent: checks fire whether or not anything accesses the overlay).

**Files:** Modify `crates/sigil-frontend-emp/src/eval/mod.rs` (`index_items` ~282–319: index `Item::Vars` overlay-form decls into `self.overlays: HashMap<String, ast::VarsDecl>`; region form — `name: None` — stays unindexed/inert), `crates/sigil-frontend-emp/src/layout.rs` (OverlayInfo + resolution), `crates/sigil-frontend-emp/src/eval/expr.rs` (`offsetof`/`sizeof` on overlays), `crates/sigil-frontend-emp/src/lower/mod.rs` (`Item::Vars` arm in the top-level loop ~118–168 AND `lower_section_items` ~241: force overlay layout, emit nothing). Test: create `crates/sigil-frontend-emp/tests/overlay.rs`.

- [ ] **Step 1: failing unit tests** (create `tests/overlay.rs`; MIRROR the eval-harness style of `tests/eval_guards.rs` — build a `File` via parse, run the lowering entry the CLI uses, assert diagnostics/bytes). The test prelude used throughout:

```rust
const SST: &str = "struct Sst (size: $50) {\n    id: u16,\n    x_pos: u16 @ $10,\n    y_vel: u16 @ $1A,\n    sst_custom: [u8; 34] @ $2E,\n}\n";
```

Cases (one `#[test]` each):
1. `vars V: sst_custom { timer: u8, charge: u16 }` + SST → no diagnostics; `offsetof(V, charge) == 1`, `sizeof(V) == 3` (assert via a data item: `data d: [u8;2] = [offsetof(V, charge), sizeof(V)]` → bytes `01 03`).
2. window overflow: 35 bytes of fields → error containing `[overlay.window-overflow]` and `over by 1`; fires with NO access to the overlay (always-on).
3. `[overlay.unknown-window]`: `vars V: no_such_window { t: u8 }` (with SST in scope) → error names `no_such_window`.
4. `[overlay.ambiguous-window]`: TWO structs each with a `[u8; N]` field named `scratch`, `vars V: scratch { t: u8 }` → error names both candidates and suggests the dotted form; `vars V: Sst2.scratch { t: u8 }` variant resolves.
5. `[overlay.shadows-field]`: overlay field named `x_pos` (a direct Sst field) → error at the overlay declaration.
6. dotted window: `vars V: Sst.sst_custom { t: u8 }` → resolves identically to bare.
7. non-byte-array window: struct field `words: [u16; 8]` + `vars V: words { … }` → error (v1: `[u8; N]` windows only).

- [ ] **Step 2: run, verify all fail** (mostly "unknown name offsetof arg" / missing diagnostics).
- [ ] **Step 3: implement**:
  - `OverlayInfo { base_struct: String, window_offset: i128, window_size: i128, fields: Vec<(String, i128 /*offset in overlay*/, i128 /*size*/)>, span: Span }`, memoized `overlay_layout_memo` (poison-on-error like structs).
  - Window resolution per D6.A1: dotted `[S, w]` → struct `S`, field `w`; bare `[w]` → scan `self.structs` for byte-array fields named `w`; 0 hits → unknown-window, ≥2 → ambiguous-window (list candidates as `S1.w`, `S2.w`), exactly 1 → resolve. Window field must be `Type::Array(u8, N)` (evaluate N; reuse the struct-layout field-size machinery).
  - Overlay field layout: declaration order, no padding, §4.3 sizing (reuse the struct field-size path so `u8/u16/i16/[u8;N]`/newtypes all size identically; `[layout.odd-field]` applies as for structs).
  - `offsetof(V, f)`/`sizeof(V)`: extend the existing builtins (expr.rs:92–120, `struct_name_for_offsetof`) to check `self.overlays` when the name is not a struct.
  - Lowering `Item::Vars` arm (BOTH item loops): overlay form → force `overlay_layout(name)` so declaration diagnostics always fire; emit zero bytes. Region form → no-op (unchanged, stays inert per design OUT-list).
- [ ] **Step 4: green gate, then commit** — `feat(emp): SST overlay semantics — window resolution, layout, always-on capacity/shadow checks (D6.A1/A2/A7/A9)`

## Task 6: field-access-as-displacement, bare form (D6.A3/A5/A6/A10) — THE pitcher_plant blocker

**Files:** Modify `crates/sigil-frontend-emp/src/eval/mod.rs` (`eval_proc_body` ~600: add `params: &[(String, ast::Type, Span)]` parameter; build the register-type table), `crates/sigil-frontend-emp/src/lower/proc.rs` (~82: pass `proc.params`), `crates/sigil-frontend-emp/src/eval/asm.rs` (`map_operand` DispInd arm ~254; the instruction-build site that knows the resolved `Width` for the overrun check). Test: `crates/sigil-frontend-emp/tests/overlay.rs`.

- [ ] **Step 1: failing tests** (same harness; SST prelude from Task 5; each proc is `proc p (a0: *Sst) { … rts }` unless stated):
1. **The headline bytes**: body `subq.b #1, timer(a0)` with `vars V: sst_custom { timer: u8 }` → instruction bytes exactly `53 28 00 2E` (+ `4E 75` for rts). Hand-derivation: SUBQ.B #1, (d16,A0) = `0101 001 1 00 101 000` = `5328`, ext `002E`.
2. **Direct struct field**: `move.w x_pos(a0), d0` → `30 28 00 10` (MOVE.W (d16,A0),D0 = `0011 000 000 101 000` = `3028`, ext `0010`).
3. **Overlay field at non-zero overlay offset**: `{ timer: u8, charge: u16 }`, `move.w charge(a0), d0` → ext word `$2F` ($2E+1) — also expect the `[layout.odd-field]`-class warning if that lint applies to overlays (assert bytes regardless).
4. **Byte-neutrality (D6.A10)**: `timer(a0)` emission byte-identical to writing `$2E(a0)` literally in an otherwise-identical proc.
5. **No const fallback on typed reg (D6.A3)**: `const timer: u8 = 9` at module level, NO overlay in scope, `a0: *Sst`, `tst.b timer(a0)` → `[operand.unknown-field]` mentioning `*Sst` (NOT the const value 9).
6. **Untyped register keeps today's semantics (D6.A5)**: `proc p (a1: *u8)` — `tst.b MYCONST(a1)` with `const MYCONST = $20` → bytes with ext `0020` (comptime eval, unchanged); and `tst.b timer(a1)` → today's plain `unknown name` (field space not consulted).
7. **Ambiguous field (D6.A3)**: two overlays over `sst_custom` both declaring `timer`, bare `timer(a0)` → `[operand.ambiguous-field]` naming both `V1.timer` and `V2.timer`.
8. **Field-overrun (D6.A6)**: `move.w timer(a0), d0` with `timer: u8` → `[operand.field-overrun]`; narrower access `move.b charge(a0), d0` with `charge: u16` → legal, byte-checked (ext = charge's offset).
- [ ] **Step 2: run, verify all fail.**
- [ ] **Step 3: implement**:
  - `eval_proc_body(file, name, params, body, span, asm_counter_start)`; in the evaluator build `reg_pointee_struct: HashMap<Reg, String>` — for each param whose `ast::Type` is `Ptr(inner)`, resolve `inner` through newtype/refined to a struct name (MIRROR `struct_name_for_offsetof`, layout.rs:242); non-struct pointees (e.g. `*u8`) simply don't enter the map. Update the one caller (lower/proc.rs) + any test callers the compiler flags.
  - In the `DispInd` arm: get the register first; if `disp` is `Expr::Path` with ONE segment AND the register is in `reg_pointee_struct` → field-space resolution ONLY: direct fields of the struct (via `layout_of_struct`) ∪ fields of indexed overlays whose resolved `base_struct` matches. Exactly one hit → `CodeOperand::DispInd { disp: window_offset + field_offset (or field offset for direct), reg }`, remembering the field's byte size for the overrun check. Zero hits → `[operand.unknown-field]`. Two+ → `[operand.ambiguous-field]` (list qualified candidates). All other cases (multi-segment paths for now, non-path exprs, untyped register) → existing `eval_expr` path, byte-for-byte unchanged.
  - Overrun check where the instruction's `Width` is known (same function that calls `resolve_size`): resolved-field access with `width_bytes > field_size` → `[operand.field-overrun] .w access reads 2 bytes but field \`timer\` is 1 byte`. Narrower or equal: no diagnostic.
- [ ] **Step 4: green gate, then commit** — `feat(emp): SST overlay field access as displacement — timer(a0) lowers to $2E(a0)-class bytes (D6.A3/A5/A6/A10)`

## Task 7: qualified field access on any An + cross-module overlays (D6.A4/A8)

**Files:** Modify `crates/sigil-frontend-emp/src/eval/asm.rs` (DispInd arm: two-segment paths). Tests: `crates/sigil-frontend-emp/tests/overlay.rs` + `crates/sigil-cli/tests/module_resolution.rs`.

- [ ] **Step 1: failing tests**:
1. `V.timer(a1)` on an UNTYPED `a1` → resolves (qualification = type assertion), bytes ext `$2E`.
2. `Sst.x_pos(a1)` (struct-qualified, untyped reg) → ext `$10`.
3. Disambiguation: the Task-6 two-overlay ambiguity case, now `V1.timer(a0)` → resolves.
4. Non-field two-segment path still evals comptime: `offsets T { A: x, B: y }` … `tst.b T.B(a0)` → ext = `0001` (ordinal 1; the pre-existing meaning — regression guard that field space only claims OVERLAY/STRUCT first segments).
5. Cross-module (module_resolution.rs): lib has SST struct + `pub vars V: sst_custom { timer: u8 }`; consumer `use lib.{Sst, V}` + `proc p (a0: *Sst) { subq.b #1, timer(a0) rts }` → builds, bytes `53 28 00 2E 4E 75`. (Rides Task 2's fixed injection; overlay decls must be carried by the same ambient path as structs — implement whatever `Item::Vars` injection arm that needs.)
- [ ] **Step 2: verify all fail.**
- [ ] **Step 3: implement** — in the DispInd arm, for a TWO-segment `Expr::Path [q, f]`: if `q` names an indexed overlay → resolve `f` in it (any address register); else if `q` names a struct → resolve `f` as a direct field; else → existing comptime eval (preserves case 4). Cross-module: add `Item::Vars` (overlay form, `pub`) to the ambient-injection collectors from Task 2.
- [ ] **Step 4: green gate, then commit** — `feat(emp): qualified overlay/struct field displacement + pub vars cross-module (D6.A4/A8)`

## Task 8: Part-A exhibit — `examples/sst_overlay.emp`

**Files:** Create `examples/sst_overlay.emp`. Test: `crates/sigil-cli/tests/ports.rs` (MIRROR how `examples/guards.emp` is compiled+asserted there — grep `guards.emp`).

- [ ] **Step 1: write the example** — single module, self-contained: the SST struct (id/x_pos/y_vel/sst_custom shape above), one overlay (`timer: u8, charge: u16`), one const, two procs exercising bare access (`timer(a0)`, `x_pos(a0)`, `y_vel(a0)`), narrower access on `charge`, and one qualified access. Every instruction must be lowerable TODAY (sized branches, `jmp`/`rts` — no `jbra`, no helpers; this is the compiles-end-to-end counterpart to the still-blocked pitcher_plant exhibit). Comment header states what it exhibits and cites D6.
- [ ] **Step 2: failing ports test** — compile the example via the CLI harness, assert zero diagnostics + the full expected byte image (hand-assemble; every opcode used must appear in an existing byte-exact test — reuse those encodings).
- [ ] **Step 3: make it pass** (fix the example, not the compiler — any compiler bug found here goes back to Task 6/7 with a failing unit test first).
- [ ] **Step 4: green gate, then commit** — `test(emp): sst_overlay example — end-to-end overlay exhibit, byte-exact`

---

## Task 9: `dispatch` — parser + AST (D6.B1)

**Files:** Modify `crates/sigil-frontend-emp/src/ast.rs`, `crates/sigil-frontend-emp/src/parser.rs` (item dispatch, MIRROR `offsets` parsing + `data (max_size:)` attribute parens; add `"dispatch"` to OPENERS → 16 entries). Test: `crates/sigil-frontend-emp/tests/parser_decls.rs`.

- [ ] **Step 1: failing tests**:

```rust
#[test]
fn dispatch_decl_parses() {
    let f = ok("module m\ndispatch Routines (encoding: word_offsets) { Init: init, Wait: wait }\n");
    let Item::Dispatch(d) = &f.items[0] else { panic!() };
    assert_eq!(d.name, "Routines");
    assert_eq!(d.encoding, DispatchEncoding::WordOffsets);
    assert_eq!(d.members.len(), 2);
    assert_eq!(d.members[0].name, "Init");
}

#[test]
fn dispatch_requires_encoding() {
    // No default encoding — R1: enable encodings, impose none.
    let (_f, diags) = parse_str("module m\ndispatch R { A: x }\n");
    assert!(diags.iter().any(|d| d.message.contains("encoding")));
    let (_f, diags) = parse_str("module m\ndispatch R (encoding: sideways) { A: x }\n");
    assert!(diags.iter().any(|d| d.message.contains("word_offsets") && d.message.contains("long_ptrs")));
}

#[test]
fn dispatch_reserves_inline_body_form() {
    // `Member: { … }` is reserved for #9 (scripted states) — must be a CLEAR error, not a parse mangle.
    let (_f, diags) = parse_str("module m\ndispatch R (encoding: word_offsets) { A: { rts } }\n");
    assert!(diags.iter().any(|d| d.message.contains("reserved")), "got: {diags:?}");
}
```

- [ ] **Step 2: verify all fail.**
- [ ] **Step 3: implement** — `DispatchDecl { public, name, encoding, members, span }`, `DispatchEncoding { WordOffsets, LongPtrs }` (scale(): 2 | 4), `DispatchMember { name, target: Expr, span }` (target parsed like an `offsets` member target). `pub dispatch` allowed (MIRROR `pub offsets`). The `{`-after-`:` case emits: `dispatch member bodies (\`Member: { … }\`) are reserved for scripted states (backlog #9) — bind a proc label instead`.
- [ ] **Step 4: green gate, then commit** — `feat(emp-parse): dispatch item — encoding-agnostic state-dispatch table grammar (D6.B1)`

## Task 10: `dispatch` — ordinals + `word_offsets` lowering, byte-exact (D6.B2/B3/B4/B5)

**Files:** Modify `crates/sigil-frontend-emp/src/eval/mod.rs` (index dispatch decls), `crates/sigil-frontend-emp/src/eval/expr.rs` (ordinals — MIRROR the offsets arm at ~216–231, ×scale; `count` unscaled + reserved; duplicate-member error wherever offsets does it — grep), `crates/sigil-frontend-emp/src/lower/mod.rs` (`Item::Dispatch` arm in BOTH item loops — MIRROR/generalize `lower_offsets_item` ~306–321: RelOffset cells, `define_label(name)`, signed-word range check, `[dispatch.non-68k]` mirroring `[offsets.non-68k]`). Test: create `crates/sigil-frontend-emp/tests/dispatch.rs`.

- [ ] **Step 1: failing tests**:
1. **Byte-exact table**: `dispatch Routines (encoding: word_offsets) { Init: init, Wait: wait }` followed by `proc init { rts }` `proc wait { rts }` → image `00 04 00 06 4E 75 4E 75` (table 4 bytes at 0; init at 4, wait at 6).
2. **Scaled ordinals**: `data ids: [u8; 3] = [Routines.Init, Routines.Wait, Routines.count]` → `00 02 02` (ordinals ×2; count unscaled).
3. **Reserved `count`** member → compile error; **duplicate member** → compile error (mirror offsets' messages).
4. **Target-not-code (D6.B4)**: member target naming a module-local `data` item → `[dispatch.target-not-code]`; a `const` target → same; an undefined name → link-time unknown symbol (loud), NOT a silent entry.
5. **Signed-word range**: reuse the offsets range-check test shape with a far `org` target if the harness supports it; if the offsets tests cover range purely via unit cells, MIRROR that instead.
6. **Z80 section** → `[dispatch.non-68k]`.
7. **Cross-module target** (module_resolution.rs): member target in a sibling module via `use` → byte-checked (MIRROR `cross_module_offsets_table_bytes_are_exact`, ~:445).
- [ ] **Step 2: verify all fail.**
- [ ] **Step 3: implement** per the design: reuse the offsets cell-building path (generalize `eval_offsets_with_root` to take `(name, &[(member_name, target)])` or add a sibling fn — pick whichever keeps offsets' code byte-for-byte identical; the offsets tests are the non-regression net). Kind check: resolve single-segment module-local targets against `file.items` (recursing one section level): `Proc` ok; `Data`/`Const`/`Offsets`/`Vars`/`Dispatch` → `[dispatch.target-not-code] dispatch \`Routines\` member \`Init\` targets data item \`init\` — a dispatch table must point at code`; unresolvable → leave to link (cross-module, v1 unchecked per D6.B4).
- [ ] **Step 4: green gate, then commit** — `feat(emp): dispatch word_offsets encoding — table emission on offsets machinery, ×2 ordinals, target kind check (D6.B2-B5)`

## Task 11: `dispatch` — `long_ptrs` encoding (D6.B2)

**Files:** Modify `crates/sigil-frontend-emp/src/lower/mod.rs` (+ wherever the Task-0-identified label-pointer cell lives). Test: `crates/sigil-frontend-emp/tests/dispatch.rs`.

- [ ] **Step 1: failing tests**:
1. **Byte-exact**: `dispatch Routines (encoding: long_ptrs) { Init: init, Wait: wait }` + two `rts` procs → `00 00 00 08 00 00 00 0A 4E 75 4E 75` (8-byte table at 0; init at 8, wait at $0A). This assumes the default section origin is 0 — derive against the harness's actual default origin; if it is nonzero, compute the two addresses from that origin and note it in the test comment.
2. **×4 ordinals**: `data ids: [u8; 3] = [Routines.Init, Routines.Wait, Routines.count]` → `00 04 02`.
3. **Cross-module long_ptrs target** → link-resolved absolute address, byte-checked.
- [ ] **Step 2: verify they fail.**
- [ ] **Step 3: implement** — per member emit the existing 4-byte label-pointer cell (the `code: "init"` mechanism found in Task 0); base label + declaration order identical to word_offsets; ordinal scale comes from `encoding.scale()` already indexed in Task 10.
- [ ] **Step 4: green gate, then commit** — `feat(emp): dispatch long_ptrs encoding — dc.l tables, ×4 ordinals (D6.B2)`

## Task 12: Part-B exhibit + branch wrap-up

**Files:** Create `examples/dispatch.emp`. Modify `crates/sigil-cli/tests/ports.rs`. Create `docs/superpowers/notes/2026-07-0X-spec2-plan7-item6-complete-handoff.md` (X = actual date).

- [ ] **Step 1: write `examples/dispatch.emp`** — an S3K-shaped object routine table: one `dispatch … (encoding: word_offsets)` over three tiny procs + a data item consuming the scaled ordinals (the routine-byte idiom), plus a second `long_ptrs` table showing the same states in the Vectorman-class encoding. Self-contained, lowerable today. Header comments cite D6.B and R1 (why the encoding knob exists).
- [ ] **Step 2: ports test** — compile it, assert zero diagnostics + full byte image (hand-assembled).
- [ ] **Step 3: run the FULL gate one final time** on the whole branch: `cargo test --workspace --no-fail-fast` (zero new failures beyond the 4-test allowlist) + clippy `-D warnings`.
- [ ] **Step 4: completion handoff note** — what shipped (task list + commits), what was deferred (design-doc ledger lists verbatim), gate evidence (test counts, the 4 allowlisted reds), and pointers for the spec lift (design doc "Spec integration" section).
- [ ] **Step 5: commit** — `test(emp): dispatch example + completion handoff — Plan 7 #6 branch complete`

**Do NOT merge to master — Volence checkpoint required.**

---

## Self-review notes (Fable, at plan-writing time)

- Every design-doc decision maps to a task: D6.A1/A2/A7/A9→T5, A3/A5/A6/A10→T6, A4/A8→T7, B1→T9, B2→T10+T11, B3/B4/B5→T10, B6→T9 (reserved-form error); Part 0a/0b/0c→T1/T2/T3; acceptance exhibits→T8/T12.
- The two byte-critical hand encodings (`53 28 00 2E`, `30 28 00 10`) were derived from the 68k opcode tables (SUBQ 0101 ddd 1 ss eeeeee; MOVE 00 ss RRR MMM mmm rrr) — implementers: re-derive before "fixing" a byte mismatch.
- Task 6 reorders DispInd evaluation (register before displacement); if any existing test asserts diagnostic ORDER on malformed DispInd operands, adapt the implementation to preserve it (the byte behavior is the contract, not the internal order).
- Model guidance for dispatch (Fable's per-task calls): T1–T4, T8, T9, T12 = sonnet; T5, T6, T7, T10, T11 = opus. Two-stage review (spec-compliance + code-quality) on T5, T6, T10.
