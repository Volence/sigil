# Design — Spec 2 · Plan 7 backlog #6: SST overlay + encoding-agnostic dispatch

Date: 2026-07-07 (Fable). Decisions locked per role split (Volence defers technical calls;
checkpoint at merge). Inputs: item-6 handoff, research T1-b/T2-c/R1/R2, spec §4.3/§4.6/§4.7/§5.1
(D2.15, D2.20, S2-D10), the item-4 pitcher-plant gap analysis, a fresh code-seam survey, and two
adversarial audits of the #5 merge (ba3fb98..d1e4288).

## Verified code facts this design stands on (re-verify at T0)

- `Operand::DispInd`/`CodeOperand::DispInd`/`M68kOperand::Disp16An` exist end-to-end with i16
  range checking (ast.rs:777, value.rs:283, eval/asm.rs:254, lower/code.rs:393). `timer(a0)`
  already PARSES as `DispInd { disp: Path("timer"), reg: a0 }`; it fails only at
  `eval_path` → "unknown name" (eval/expr.rs:162).
- Struct layout is comptime-known and memoized (`layout_of_struct`, layout.rs:383), honors
  explicit `@ offset` fields and `(size:)`, and `offsetof` ships. **Therefore overlay field
  access needs NO link-time fixup: the displacement is a plain comptime integer.**
- `VarsDecl` (overlay form `vars Name: region { fields }`) parses (parser.rs:649, ast.rs:247)
  but is indexed/lowered NOWHERE (eval/mod.rs:316, lower/mod.rs:118–168 have no `Vars` arm).
- Proc params `(a0: *Sst)` are declared but NOT bound during body eval (eval/mod.rs:610).
- `offsets` emission (RelOffset cells, base label, signed-word check) and ordinals are shipped
  and byte-exact — the `word_offsets` dispatch encoding lowers onto this machinery.
- The intended base-struct shape exists in the parser corpus:
  `struct Sst (size: $50) { …, sst_custom: [u8; 34] @ $2E }` (parser_decls.rs:173–186).

## Part 0 — #5 audit dispositions (fix on this branch, before feature work)

Two opus audits of ba3fb98..d1e4288: the feature half (guards/max_size) and the dotted-label
qualification survived adversarially; three defects of the SAME flat-iteration class faf5191
was meant to close were reproduced. Dispositions (locked):

- **0a. Nested `section{}` is silently swallowed by lowering** (lower/mod.rs:241–273 has no
  `Section` arm) — items, `ensure_fatal`, and `max_size` checks inside a section-in-section
  vanish with zero diagnostics, while eval/resolve DO recurse. **Fix: reject nested
  `section{}` with a loud `[section.nested]` error at the inner section's span.** Rationale:
  placement-within-placement has no defined meaning in §7.1; rejection is honest, total, and
  additively reversible if a meaning is ever ratified. Do not invent recursion semantics.
- **0b. Section-nested `pub` comptime defs are never injected cross-module** (resolve/mod.rs
  ~:50–54 prelude path, ~:72–89 use path iterate items flat) — exported per faf5191, then
  "unknown name" at the consumer. **Fix: mirror faf5191's recursion in
  `ambient_items`/`pub_comptime_name` collection.** (Single-level sections only, per 0a.)
- **0c. `use` nested in a `section{}` is silently ignored** by all three of `reachable_modules`
  BFS (resolve/mod.rs:303), `ResolveEnv::build` (imports.rs:137), and `ambient_items`.
  **Fix: recurse all three.** The spec ratifies section-nested items as first-class
  importables; imports must follow the same contract.

Deferred without action: max_size diagnostics anchoring at 1:1 (already an acknowledged nit)
and the malformed-attribute error cascade (cosmetic recovery).

## Part A — SST overlay + field-access-as-displacement (T2-c; the pitcher_plant blocker)

Surface is already ratified (§4.6): `vars PitcherPlantV: sst_custom { timer: u8 }`, access
`timer(a0)` with `a0: *Sst`. What follows pins the semantics.

- **D6.A1 — Window resolution.** The overlay's region name resolves to a **byte-array field**
  (`[u8; N]`, possibly with explicit `@ offset`) of an in-scope struct — that struct is the
  overlay's *base struct*. A bare name (`sst_custom`) must match **exactly one** such field
  across in-scope structs; two or more matches is `[overlay.ambiguous-window]` with the dotted
  fix-it; zero is `[overlay.unknown-window]`. The dotted form `vars X: Sst.sst_custom { … }`
  is always legal and is the disambiguator. Non-`[u8; N]` window targets are an error in v1.
- **D6.A2 — Overlay layout + capacity.** Overlay fields lay out by §4.3 struct rules
  (declaration order, no implicit padding; `[layout.odd-field]` applies). Total size exceeding
  the window's N bytes is an error **at the overlay declaration**, phrased like `max_size`:
  ``overlay `PitcherPlantV` is M bytes — exceeds `Sst.sst_custom` window of N bytes (over by
  K)`` (`[overlay.window-overflow]`). This absorbs `objvarsCheck` and closes the overlay half
  of D5.5's parked `fits_within`.
- **D6.A3 — Field access, bare form (typed registers only).** In a proc body, for displacement
  position `f(aN)` where `f` is a **bare identifier** and register `aN` has a declared param
  type bottoming out (through newtype/refined) at `*S` for struct `S`: `f` resolves **only in
  field space** — S's direct fields ∪ fields of in-scope overlays whose window belongs to S.
  Displacement = `offsetof(S, f)` for a direct field; `offsetof(S, window) + overlay-relative
  offset` for an overlay field. Not found → `[operand.unknown-field]` ("`*Sst` has no field or
  in-scope overlay field `f`") — **no const fallback on a typed register** (silent shadowing
  is impossible by construction). Found in ≥2 in-scope overlays → `[operand.ambiguous-field]`
  naming the candidates, fix-it = qualified form.
- **D6.A4 — Field access, qualified form (any address register).**
  `PitcherPlantV.timer(a1)` / `Sst.x_pos(a1)` resolves in field space explicitly and is legal
  on ANY address register — the qualification is the author's type assertion (this is the
  deliberate cross-object-access idiom; accidental cross-reads stay impossible because the
  bare form is scope- and type-gated).
- **D6.A5 — Untyped registers & non-path displacements.** Bare-identifier displacement on an
  untyped register (no param binding, or pointee not a struct) keeps today's semantics
  (comptime expression; consts legal). Non-path displacement expressions (literals,
  arithmetic, calls) always comptime-eval as today, on any register. Field names participate
  only as the entire displacement expression — `timer+1(a0)` is NOT field arithmetic in v1
  (name the byte you want as its own field; that is the construct's point).
- **D6.A6 — Access size vs field size.** An access **wider** than the resolved field
  (`move.w timer(a0), d0` with `timer: u8`) is `[operand.field-overrun]` — it crosses a named
  boundary. **Narrower** access is legal with no lint (the big-endian high-byte idiom, e.g.
  integer part of an 8.8 field, is everyday Sonic code).
- **D6.A7 — Declaration-time collisions.** An overlay field whose name collides with a direct
  field of the base struct is `[overlay.shadows-field]` at the overlay declaration. Two
  overlays over the same window may coexist, including with identical field names (every
  badnik will have a `timer`) — ambiguity is handled at the USE site per D6.A3, and only when
  both are in scope.
- **D6.A8 — Scope & sharing.** Overlays are ordinary module items: module-scoped, `pub vars`
  exports, imported via `use`/prelude (§4.6: "sharing across files is by use, not
  re-declaration"). Ambient injection must carry overlay decls like it carries structs
  (rides Part 0b's fixed collector).
- **D6.A9 — `offsetof` on overlays.** `offsetof(PitcherPlantV, timer)` = the field's offset
  **within the overlay** (consistent with struct `offsetof`); the displacement sugar is what
  adds the window offset. `sizeof(PitcherPlantV)` = the overlay's laid-out size.
- **D6.A10 — Byte-neutrality (testable).** A field-name displacement takes the identical
  `DispInd → Disp16An` path as the equivalent integer literal; a test asserts byte-identical
  emission for `timer(a0)` vs `$2E(a0)`.
- **Explicitly OUT of Part A** (recorded so nobody creeps): region-form `vars` address
  allocation (map-file work, item-#7 territory — the region form stays parse-accepted and
  inert, which fails loudly downstream via unknown names); `Player_1.x_pos` straight-line
  symbolic operands (gap b6 — a `RelaxAbsSym` extension, different seam); typed-register
  field access inside `asm{}` templates / comptime fns (no param types there yet);
  `[operand.const-as-address]` lint (noted as a natural NEXT increment, not this branch).

## Part B — `dispatch`: the encoding-agnostic typed state-dispatch table (T1-b per R1/R2)

- **D6.B1 — Construct.** New item:
  `dispatch Name (encoding: E) { Member: target, … }` — a typed code-pointer table. 68k
  sections only in v1 (`[dispatch.non-68k]`, mirroring `offsets`). No implicit alignment
  (mirrors §4.7); the table's base label is `Name` at its first byte.
- **D6.B2 — Encodings, v1 = exactly two; the knob is REQUIRED.** No default encoding — R1's
  finding is that Sonic's self-relative form is a Sonic-ism; the construct enables encodings
  and imposes none.
  - `word_offsets` — emits `dc.w member_target − Name` per member in declaration order
    (signed-word range-checked; REUSES the shipped `offsets` RelOffset machinery, per the
    handoff's "don't rebuild emission"). Ids are **pre-scaled ×2**: `Name.Member` = ordinal×2
    — exactly the byte S3K stores in `routine(a0)` and `add.w`s into the table.
  - `long_ptrs` — emits `dc.l target` (Abs32 fixups) per member. Ids pre-scaled ×4
    (the Ristar pre-shifted-index form; also the natural table for Vectorman-class engines).
- **D6.B3 — Reverse constants.** `Name.Member` = pre-scaled **plain comptime int** (D2.15
  precedent — the win is the constant it replaces; a distinct state newtype is #9's surface).
  `Name.count` = member count, UNscaled. `count` reserved; duplicate member names error.
- **D6.B4 — Targets.** A member target is a label reference resolved at link, same as
  `offsets` targets (cross-module included — rides #4). A target that resolves
  module-locally to a non-code item (`data`/`const`) is `[dispatch.target-not-code]` — a
  dispatch table into data is exactly the jump-to-garbage this construct exists to kill.
  Cross-module targets are kind-unchecked in v1 (ledger note; link still fails loudly on
  unknown symbols).
- **D6.B5 — Exhaustiveness model.** The declaration IS the state set (single source of truth
  — the `offsets` lesson): a missing entry is unrepresentable, an extra/renamed state updates
  every use site by name. No separate enum binding in v1; a `dispatch Name for Enum` form is
  a deferred knob for when a state set must be shared across tables.
- **D6.B6 — #9 anticipation (design-only, nothing built).** The member grammar deliberately
  mirrors the ratified §4.7 mixed inline-target design: `Member: label` today;
  `Member: { … }` (inline body / scripted state with `yield`) is the reserved #9 extension.
  The parser must not claim that syntax for anything else. First-class-function-value
  engines (SCE `move.l #.label, code_addr(a0)` continuations) need proc-name-as-value (gap
  b7), not a table — out of #6, recorded.
- **Deferred (ledger):** ordinal-unscaled ids (Treasure word-index dispatch), no-table
  continuation style (= b7), Z80 dispatch tables, `start:`-style ordinal origin, `for Enum`
  exhaustiveness binding, per-member `pub`.

## Acceptance (what "done" means for #6)

1. `examples/sst_overlay.emp` (new, compiles end-to-end): a pitcher-plant-shaped object with
   a mini-prelude `struct Sst (size:$50)` including `sst_custom: [u8;34] @ $2E`; `timer(a0)`,
   `x_pos(a0)`-class access **byte-checked against hand-assembled AS output** (`$2E(a0)`
   class) + the D6.A10 byte-neutrality test. All new diagnostics (window overflow, unknown/
   ambiguous field, field-overrun, shadows-field, ambiguous-window) have tests.
2. `examples/dispatch.emp` (new, compiles end-to-end): an S3K-shaped routine table in
   `word_offsets` (byte-diffed word-for-word against the hand `dc.w Target-Base` form) and a
   `long_ptrs` table (byte-checked via link fixups); `Name.Member`/`Name.count` used as data.
3. Part 0's three fixes each carry the reproducing case from the audits as a regression test.
4. Green gate per commit: `cargo test --workspace --no-fail-fast` + the 4-test sigil-harness
   allowlist (aeon strlen refactor) + `cargo clippy --workspace -D warnings`. Zero NEW
   failures. pitcher_plant.emp itself is NOT a gate (still blocked on #8 jbra/jbsr + helper
   grammar — by design).

## Spec integration (Fable's, post-implementation)

Draft a proposed spec section (this repo, `specs/`), then lift into
`empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`: §4.6 gains the pinned overlay semantics
(D6.A1–A9), a new §5.5 (or §4.8) gets `dispatch`, the D-ledger gains D6 rows, and the
deferred ledger gains the Part-A/Part-B deferral lists. Update the S2-D10/D2.15 cross-refs
("dispatch is a separate encoding-agnostic construct" → "shipped as `dispatch`, #6").
