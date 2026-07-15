# Design — Spec 2 · Plan 7: the pitcher_plant completion tranche

Date: 2026-07-08 (Fable, overnight session). Branch `plan7-pitcher-plant-tranche`, STACKED on
`plan7-item8-jbra-relaxation` (separately checkpointable; do not merge tonight). Goal: the
standing acceptance exhibit `examples/pitcher_plant.emp` compiles end-to-end and its image is
byte-argued (Appendix D declares "no byte argument needed — new-style code", so the oracle is
hand-derived bytes in ports.rs, independently re-derived by the controller).

## Verified capability facts this design stands on (probed 2026-07-08, fresh tempdir probes)

What the gap analysis assumed missing but ALREADY WORKS today:
- **Statement-position PAREN calls of Code-returning comptime fns** (`seed_init()` inside a
  proc body emits the template bytes; hygiene gives fresh `$asm{k}$label` locals per
  instantiation). b3's true gap is ONLY the bare no-paren spelling.
- Hygienic local labels + branches INSIDE `asm{}` templates; `bra.s` to external labels from
  a template; `jsr {t}`/`lea {t}, a1` splices via `string`-typed params.
- Comptime struct values end-to-end: `f(Vec{ x: -16, y: -4 })` + `v.x` reads in the body.
- Enum-typed consts: `pub const inherit: Flip = Flip.inherit` works, including as a struct
  field initializer and as an enum-typed fn param compared with `==`. Zero keyword cost.
- `--prelude` auto-import of pub consts/structs/comptime-fns/procs (incl. proc labels
  referenced from data), zero `use` lines.
- `offsetof(S, f)(a0)` chains directly as a displacement; int params splice into displacement
  position via the BARE name form `off(a0)` (the `{off}(a0)` brace form does not parse —
  acceptable, bare form is idiomatic).
- Struct-literal data items work but require EVERY field, `_pad` fillers included
  (zero-default/rest-fill is a ledger candidate, not tonight).

Genuinely missing (the tranche's feature work):
- Bare directive-style statement calls (`anim Ani.Shoot`, zero-arg `despawn_below_level`).
- Registers as comptime call arguments (`facing_abs d0`) — no `Reg` param type, no register
  literal in expression grammar, and `Value::Reg` splices only from operands parsed inside
  `asm{}` itself.
- Label values: barewords naming procs/data as first-class comptime values (`code: init`,
  `routine shoot`, `spawn(SeedDef, …)`) — today only the STRING form `code: "init"` works,
  and only UNQUALIFIED (`"helper.init"` fails even imported, contradicting examples/main.emp's
  documented `code: pitcher_plant.init` intent — fold the dotted form in).
- Named args at call sites (`offset:`, `flip:`).
- `Item.field` as a straight-line memory operand (`Player_1.x_pos`) — gap b6.
- FLAGGED during probing, investigate in U3: `lea {sym}, a1` splicing a sym that names a DATA
  item emitted `43 F8 00 00` (address 0) with no diagnostic in a single-file probe — possible
  pre-existing silent-unresolved path. If real, fix or at minimum convert to a loud error.

## The decisions (D-PP.1 … D-PP.7)

- **D-PP.1 — bare statement calls (b3).** In proc-body statement position, a leading bareword
  that is NOT a recognized mnemonic for the section's CPU (and not jbra/jbsr) and RESOLVES to
  an in-scope comptime fn parses as a call: `name` (zero args), `name arg, arg, …` (args are
  comptime expressions). Sugar for the paren form — both spellings stay legal and identical
  (AS-macro muscle memory is the adoption target; parens already work). **Mnemonics win
  unconditionally** (tenet 3): a comptime fn named `move` is silently unreachable at
  statement position (callable with parens in expression position; a shadow lint is ledger
  material). A bareword that resolves to nothing keeps today's
  "not a recognized 68000 mnemonic" error; one that resolves to a NON-Code comptime value
  gets a specific error naming the type. The returned Code instantiates through the existing
  statement-position machinery (§6.2, ProvFrame::Comptime).
- **D-PP.2 — registers as comptime values (b3, the `facing_abs d0` half).** New comptime-only
  type name `Reg` (§4.2 family). In comptime CALL-ARGUMENT position (both spellings), a
  bareword naming a machine register of the current CPU parses as a register literal
  (`Value::Reg`, which already exists internally); **registers win over ordinary names in
  call-argument position** (mirrors operand position; a const named `d0` violates naming
  lints anyway). `{r}` splices of a Reg param into any register-operand position in a
  template. No register arithmetic, no Reg fields, no Reg in data — literal in, splice out.
- **D-PP.3 — label values (b7 + the `routine shoot` argument + dotted strings).** A bareword
  in comptime VALUE position (data-item field initializers, call arguments) that names a
  known proc or data item — module-local, imported, or prelude — evaluates to the SAME label
  value the string form produces today (same SymRef cell at emission, same Abs32Be fixup).
  Dotted paths (`badniks.pitcher_plant.init`) resolve when the module is reachable; this also
  fixes the string form's qualified spelling (`"helper.init"`), aligning with main.emp's
  documented intent. Comptime fn params type it as `Label` (new §4.2 name); `string`-typed
  splice params keep working unchanged (compat; migrating them is ledger). Precedence: a
  local const/comptime name SHADOWS a label value (existing name resolution wins; label-value
  resolution is the fallback for otherwise-unknown names). Unknown bareword stays a loud
  error — no silent string fallback. This closes the SCE-continuation half of R1.
- **D-PP.4 — named call arguments (b4).** At ANY comptime call site: positional args first,
  then `name: expr` named args, no positional after named, each param bound exactly once,
  every param bound (NO default values tonight — ledger). Struct literals, label values, enum
  consts all legal as named-arg values (they are just expressions). Recorded decision per the
  <!-- REVERSED 2026-07-14 (tranche 14): the "NO default values" clause is
  LIFTED — comptime-fn params now take `name: T = expr` defaults (a param
  with a default is optional; the default evaluates in a fresh global-only
  declaration scope; a param with no default stays required = the same
  `missing argument` error). Demanded by objdef()'s 12 optional params;
  sigil ab84a2e. The "every param bound" invariant now reads "every param
  WITHOUT a default bound." -->

  §10 headroom rule (call-site syntax, not a statement-leading keyword — non-breaking).
- **D-PP.5 — field-address operands (b6).** In an instruction MEMORY-operand position,
  `Item.field` — where `Item` is a data item (or other link symbol) of known struct type and
  `field` names a struct field — denotes the FIELD'S ADDRESS: lowered exactly like today's
  bare symbolic operand but with target `Expr::Add(Sym(Item), Int(offsetof))`, riding the #2
  `RelaxAbsSym` two-candidate seam unchanged (Fixup.target already holds arbitrary foldable
  exprs; RelWord16Be tables already link Sub exprs). In IMMEDIATE (`#Item.field`) and comptime
  positions the dotted path keeps its existing comptime VALUE meaning (`Def.art` reuse) — the
  address/value fork follows assembly convention (memory operand = address), same as plain
  labels today. Multi-segment nesting beyond one field (`A.b.c`) is out of scope tonight
  (error if not comptime-resolvable). If this task turns heavy, the handoff sanctions
  descoping LAST (rewrite the exhibit line as two instructions + recorded decision) — prefer
  building it; it is the final operand-class gap.
- **D-PP.6 — authoring (a1–a4).** ONE game-prelude module carries everything (§3.4: the
  prelude "carries" these names; re-export machinery does not exist and is not built tonight):
  `examples/game/prelude.emp`, `module game.prelude`. Contents: `Sst` (full dense shape with
  `_pad` fillers + `@` assertions, mirroring sst_overlay.emp's teaching layout; fields at the
  engine's canonical offsets — id @ 0, x_pos @ $10, y_pos @ $14, x_vel @ $18, y_vel @ $1A,
  anim/code fields, `sst_custom: [u8; 34] @ $2E`, size $50); `ObjDef` (all exhibit fields:
  code/map/art/col/zpri/size/anim/vel/frame — vel/frame need values in BOTH exhibit uses or
  defaults, and defaults don't exist: give Def explicit vel/frame too if required — authoring
  detail, exhibit may gain two lines, acceptable); `ArtTile`/`Size`/`Vel`/`Vec` structs
  (i16 fields per the newtype-candidates taste, no refinement types tonight); `Collision`
  enum; `Flip` enum + `pub const inherit: Flip`; engine stubs `Draw_Sprite`/`ObjectMove`
  (minimal real procs, e.g. rts-bodied with a distinctive first instruction so byte
  derivations are unambiguous) + `Player_1` (a `pub data Player_1 = Sst{ … }` struct item —
  gives b6 a typed link symbol without #7's region allocation) + art stubs
  `Map_PitcherPlant` (pub data) / `VRAM_PITCHER_PLANT` const; comptime helpers `anim(id)`,
  `routine(p: Label)`, `facing_abs(r: Reg)`, `despawn_below_level()`, `spawn(def: Label,
  offset: Vec, flip: Flip)` — bodies are honest minimal engine idioms (asm{} templates;
  field displacements via `Sst.field(a0)` qualified form, falling back to
  `offsetof(Sst, field)(a0)` chaining — both probed shapes; local labels in templates are
  hygienic and legal). The exhibit MOVES to `examples/badniks/pitcher_plant.emp` (module id
  ↔ path agreement). The colliding illustrative mock `examples/composition_pitcher_plant.emp`
  stops colliding in the least destructive way that keeps the corpus building (retitle its
  module id or relocate; deletion acceptable only if nothing clean exists — record whichever
  in the checkpoint).
- **D-PP.7 — acceptance.** `cargo run -p sigil-cli -- emp examples/badniks/pitcher_plant.emp
  --root examples --prelude game.prelude` → zero errors; ports.rs asserts the exhibit's byte
  image against hand-derived bytes (controller re-derives independently); the standing gate
  (workspace tests → only the 4 allowlisted reds; clippy -D warnings) per commit.

## Execution order (each task strict TDD, commit-per-task)

- **U1 (opus):** D-PP.1 + D-PP.2 — bare statement calls + Reg type/literals/splices.
- **U2 (sonnet):** D-PP.4 — named args. (Grammar-mechanical; escalate to opus if the call
  plumbing fights back.)
- **U3 (opus):** D-PP.3 — label values (barewords, dotted paths, `Label` param type, the
  dotted-string fix; investigate the silent-zero `lea` data-label flag while in there).
- **U4 (opus):** D-PP.5 — field-address operands on the RelaxAbsSym seam.
- **U5 (sonnet):** D-PP.6 — authoring + restructure; acceptance probe shifts from
  feature-errors to zero as U-tasks land (controller tracks the error-set shrink).
- **U6 (sonnet):** D-PP.7 — ports.rs byte-exact test + acceptance wiring.
- **U7 (opus):** whole-branch adversarial review, byte-diff probes vs the #8 branch base for
  non-participating constructs.

Two-stage reviews on U1/U3/U4 (load-bearing grammar/operand semantics); single-pass on
U2/U5/U6.

## Execution amendments (recorded as the tasks landed)

- **D-PP.6 corpus root AMENDED to `examples/game/`** (`--root examples` is unusable: four
  pre-existing `module m` exhibit files collide there — master baseline noise, not this
  branch's to fix). Prelude = `examples/game/prelude.emp` (`module prelude`); exhibit
  moved to `examples/game/badniks/pitcher_plant.emp`; the illustrative mock's module id
  retitled `composition_pitcher_plant` to stop colliding. Acceptance:
  `sigil emp examples/game/badniks/pitcher_plant.emp --root examples/game --prelude prelude`
  → **exit 0, zero diagnostics, 340 bytes — pinned byte-exact (hand-derived first, matched
  the compiler on first comparison) in crates/sigil-cli/tests/pitcher_plant_acceptance.rs**.
- Sanctioned exhibit edits: exactly two lines (`vel:`/`frame:` added to `Def` — struct
  literals require every field; no defaults tonight).
- U3 spec-review ISSUE-2 (data-item comptime field-read didn't exist; gap-analysis a4 was
  wrong) was folded into U4 and built — both halves of the `Item.field` fork shipped.
- U4 shadow defect (local item + imported type-stub → local base with IMPORTED offset)
  found by review and fixed: local wins for base AND type, coherently across both halves.
- Named args (D-PP.4) turned out mostly pre-existing; the real gap was
  positional-after-named silently mis-binding (fixed). Bare-form named args: the operand
  grammar already rejects `k: v` loudly — named args are paren-form only (recorded).
- `routine(p: Label)` helper authored as `pea {p}` / `move.w (a7)+, Sst.routine(a0)` —
  register-free so it composes under any `clobbers(...)` set (a `lea` scratch register
  would trip `[proc.clobber-undeclared]` on procs that don't declare it).
- `spawn(..., flip: Flip)`: enum values don't splice as immediates (verified fact), so the
  helper branches comptime on `flip == Flip.inherit` between template variants; the
  non-inherit arm is a documented, unexercised stub.

## Deferred (ledger candidates from this tranche)

- Default parameter values for comptime fns; struct-literal rest-fill/zero-default.
- Statement-position shadow lint (comptime fn named like a mnemonic).
- Migrating `string`-typed splice params to `Label`.
- Multi-segment field-address operands (`A.b.c`).
- Region-form `vars` + map allocation for a REAL `Player_1` (#7 — unchanged).
- Unknown-bareword label values defer to a LINK error whose span is module-level — a real
  DX cost vs the old line-precise comptime "unknown name" (U3 ISSUE-1, design-sanctioned).
- Cross-module type-only stubs inject only for data items with an EXPLICIT single-segment
  type annotation (`pub data Player_1: Sst = …`); literal-inferred types don't travel (U5).
- Shadow-warning lint when a local data item shadows an imported one (U4 fix makes it
  coherent; a lint would make it visible).
- Cross-module data-item VALUE reads (`[value.cross-module]` today, by design).
- SymOff branch-target rejection message is the generic "branch needs a single label
  target" (U4 M3). `jmp Item.field` = absolute transfer via the abs-sym seam (documented
  + byte-pinned, deliberate).
- Pre-existing vacuous fixture: resolve_rename.rs::renames_labels_and_fixup_targets
  discards lowering diags over a top-level bare-array label source (spotted during U3).
- Enum-value immediate splices (`#{flip}`) — a typed conversion story would simplify the
  spawn-helper pattern.
