# 68k port campaign — gap ledger (living doc)

Volence's standing instruction (2026-07-09, at port #1): the sound arc + the badnik exhibits
never exercised the whole language — as real conversions run, **jot down anything we haven't
implemented that would be nice to have** (missing spellings, constructs, diagnostics, tooling
comforts), whether or not we build it now. `sp` is the type specimen: three sound tranches and
two badnik exhibits never spelled it; the first 18-line code port tripped on it immediately.

**Cadence:** every port's final task sweeps the session for entries and appends them here.
Implement-now vs jot-down triage: build it now if it blocks the port or is cheap and clearly
right (the sp rule); otherwise record it with enough context to build later. Periodically
triage entries up into the spec's deferred ledger (S2-D rows) or into a tranche's scope —
this file is raw observations, not ratified decisions.

Format: `- [port/date] OBSERVATION — status (SHIPPED / OPEN / SPEC-LEDGERED S2-Dxx)`

**THE TRANCHE LOOP (Volence, ratified 2026-07-09 at tranche 2 — supersedes the paragraph
below where they differ; ~4 steps per tranche/batch):**
1. **Transcribe** — byte-exact `.emp` under `@as_compat`, verbatim instruction lines, byte
   gates green (the reviewable 1:1 port commit).
2. **Modernize** — a separate commit taking the file to the best Sigil form. Two tiers:
   (2a) DEFAULT, byte-neutral — jbra/jbsr spellings, erasing types (newtypes/fixed/refinements
   §8.3), named args, doc comments; gates re-run green as the proof. (2b) RARE, byte-changing
   rewrites ("re-write pieces completely" is sanctioned) — a knowing, recorded, per-file R7
   re-baseline: reference re-pins to the Sigil-built ROM, correctness proof shifts to
   behavior (boot-check/emulator); spend sparingly while asl-identity remains the cheap
   safety net for the rest of the tree.
3. **Retrospect** — walk this ledger's new entries with Volence: missed idioms, Sigil
   improvements, anything that could be nicer.
4. **Implement** — build ratified items in Sigil, apply back to the current tranche's files
   if relevant, final gate pass. Then the next tranche.

**Step 2 is the campaign's quality apex (Volence, 2026-07-09): nothing holds it back.** The
other steps exist to feed it — if the best version of a file wants a missing construct,
shared module, or assert form, BUILD it (the equ-fix precedent) rather than settle. The
output of step 2 is the codebase the community lives in; "best of the best" is the bar.

**Cadence (Volence, 2026-07-09, clarified same day): a retrospect PER CONVERSION, not one
review at campaign end.** Each port's checkpoint packet carries a short retrospect section:
the port's new ledger entries, each with a recommendation — implement in the next tranche /
promote to a spec S2-D row / leave jotted — and Volence rules on them AT that checkpoint. So
language additions happen rolling, one recorded decision at a time (A-Spec2.3), while the
conversion mileage is fresh. The separately-scheduled Spec-2-close items (S2-D17 patch/bind
complete-or-demote; the S2-D12 seams; S2-D7(c) cycle budgets) keep their own triggers; a final
end-of-campaign sweep of anything still OPEN here is a wrap-up, not the decision point.

## Language / lowering

- [port #1 hblank, 2026-07-09] **`sp` register spelling** — the whole aeon tree spells `sp`,
  no test before this port ever used it. — SHIPPED with port #1 (general alias at
  `Reg::from_name`, byte-identical to `a7`).
- [port #1 hblank, 2026-07-09] **`movem` register lists** (`d0-d1/a0`, ranges + `/` unions) —
  ubiquitous in code files, absent from every prior test. — SHIPPED with port #1
  (mnemonic-directed reinterpretation of the parsed expr tree, canonical mask, AS-parity
  refusals: descending ranges, `movem.b`).
- [port #1 hblank, 2026-07-09] **Symbolic absolute operand targeting an `equ` fails**
  (`unresolved symbolic absolute operand`) — the width-relaxation pass reads layout *labels*,
  not equ symbols, so `movea.l SOME_EQU, a0` can't lower even though the linker knows the
  value. Port #1 dodged it (its target is a RAM label); any port referencing an AS-side
  equated ADDRESS as an operand will hit it. Also wants a better diagnostic (name the symbol,
  say WHY — equ vs label — and point at the workaround). — SHIPPED with port #1 (Task 5
  follow-up, `relax.rs`): each relaxation pass now overlays a best-effort `equ` fold
  (`equ_lookup_overlay`) on top of that pass's label table before selecting a
  `RelaxAbsSym`/`JmpJsrSym` rung, so a target naming an equ, an equ-on-equ
  chain, or an equ derived from a label all resolve — grow-only width protection is
  automatic (same `v`/gate as label targets, no new policy). The FINAL, loud `fold_equ_syms`
  pass at convergence is unchanged (still the authoritative cycle/unresolved-equ error).
  Review narrowing (same day): the equ overlay applies to the ABS-ONLY fragments above ONLY —
  the jbra/ladder-to-equ shape is REFUSED by review ruling (a ladder's pc-relative rungs
  would silently branch pc-relative to a NEAR absolute equ value, e.g. `equ R = $420` near
  the section → `60 1E`; branch targets are labels, use jmp/jsr for absolutes — the ladder's
  unresolved-target diagnostic says exactly that when the target is an equ).
- [port #1 hblank, 2026-07-09] **movem `(0,An)` → `(An)` collapse not ported** from the AS
  front-end — exists there to fold forward-reference displacements that resolve to 0
  post-pass, which `.emp`'s resolved eval model doesn't produce. Believed unreachable in
  `.emp`; noted in a doc comment at the movem lowering. Re-check if a port ever spells
  `movem.l list, 0(aN)` deliberately. — OPEN (watch).
- [port #1 hblank T3, 2026-07-09] **`try_defer_long_imm`'s R3 imm32 deferral only covered
  register destinations** (`movea.l #imm,aN` / `move.l #imm,dN`) — the REAL `boot.asm:185`
  shape, `move.l #HBlank_Null, (HBlank_Handler_Ptr).w`, has an ABSOLUTE `(abs).w` destination
  and hard-errored (`unresolved symbol` at the converged pass) the moment the cross-seam source
  immediate was genuinely unresolved (i.e. the moment `SIGIL_EMP_HBLANK` was actually turned on
  against the real tree — not caught by any prior port because none needed BOTH an absolute
  destination AND an unresolved source in the same instruction). — SHIPPED with port #1 T3:
  extended `try_defer_long_imm` to accept an `OperandAtom::M68kAbs` destination for `move.l`
  (the destination address resolves eagerly — only the source immediate defers; verified against
  the real reference encoding `21FC 0000228E 8022`, s4.lst:5794, byte-for-byte). Found via the
  bare-name-proof synthetic consumer in `hblank_port.rs`, which reproduces the exact AS-side
  shape a real cross-seam `move.l #procname, (ramlabel).w` idiom needs — worth flagging that
  T3's synthetic-consumer requirement (not just T2's real-file port) is what surfaces front-end
  gaps like this; a port whose T3 never builds a REAL AS-shaped consumer could ship silently
  incomplete.
- [port #1 hblank T3, 2026-07-09] **`resolve_layout`'s `RelaxAbsSym` diagnostic doesn't name the
  symbol** (`"unresolved symbolic absolute operand in section {name}"`) — unlike
  `check_link_asserts`'s Item-C wording (which DOES name missing symbols, "references symbol(s)
  `X` not defined in this link — expected when compiling a cross-seam module standalone"), the
  plain-operand relaxation path only names the SECTION, not the symbol. Confirmed still true at
  port #1: `hblank.emp`'s standalone-compile negative probe fires this exact under-specified
  message (no `ensure`/`extern` in the module to route through the better-worded path). Same
  root issue as the equ-operand gap above (both live in `relax.rs`'s `RelaxAbsSym` handling) —
  candidate for ONE fix (thread the symbol name through both diagnostics) rather than two. —
  SHIPPED with port #1 (Task 5 follow-up, same fix as the equ-operand entry above): the
  `RelaxAbsSym` unresolved-target diagnostic now names the symbol and, when it isn't an `equ`
  anywhere in the link, uses the Item-C cross-seam-standalone wording ("references symbol `X`
  not defined in this link — expected when compiling a cross-seam module standalone..."); when
  the name IS an equ that never resolved (cycle / dangling dependency), a distinct wording
  applies so a reader doesn't mistake a cycle for a plain missing symbol.
  `hblank_negative_probes.rs`'s standalone-compile probe updated to pin the new wording.

## vars / RAM regions (ram.asm pre-port audit, 2026-07-09)

Volence asked whether the language has a good answer to `ram.asm`'s shape (bare `Name: ds.b N`
runs, hand pads, invisible addresses). Audit verdict: the frozen §4.6 `vars` surface already
covers the core — map-file regions with budgets (kills the three overflow `if`s + bit-15
check), `@align(N)` on fields (kills the 256-align guard and the ~20 hand `ds.b 1` evenness
pads, spelled as intent on the following field), typed/struct fields (kills the
`Parallax_State_End`-style label runs and the `Player_Phys` "must match PHYS_* order" sync
comment), `[layout.odd-field]`, item-position `ensure`, and `pub equ` export for `.asm`
consumers. NOTE for the eventual port tranche: RAM emits no bytes, so its byte-exact gate is
**address-exact, on BOTH build shapes** (ROM operands pin every RAM address transitively);
symbol-table diff vs the AS reference is the sharp diagnostic. Gaps found:

- [ram.asm audit, 2026-07-09] **Conditional fields inside `vars`** — engine `ram.asm` has a
  mid-region `ifdef __DEBUG__` block (Prof_* / DMA debug counters), so DEBUG and release have
  different downstream addresses; two-shape address-exactness needs comptime-`if`-over-fields
  in `vars`, driven by the existing `-D` defines (D2.27). Non-breaking growth internal to the
  block, but needs a recorded decision. The port BLOCKS on this (or on first moving the debug
  block to the region tail as a deliberate pre-port .asm change). — OPEN (build with the ram
  tranche).
- [ram.asm audit, 2026-07-09] **Checked buffer-reuse overlay** — `Art_Staging_Buffer =
  Tile_Cache_Nametable` + hand size `if` + lifetime comment ("INIT-ONLY"). Expressible today
  as `pub equ` alias + `ensure(size fits)`, so NOT a port blocker; the nicety is a declared
  region-level overlap (SST overlays exist, D2.21, but only over struct `[u8;N]` windows) that
  checks size at the declaration and states the lifetime. — OPEN (jotted).
- [ram.asm audit, 2026-07-09] **Debug-layout-stability lint** — the `Sound_Dbg_Mirror`
  precedent (declared unconditionally, comment explains why) shows the hazard class:
  conditional fields silently shifting the other shape's addresses. Once conditional `vars`
  fields exist, a lint ("conditional field not at region tail" or "shapes diverge here") makes
  the hazard visible. — OPEN (jotted; design with the conditional-fields decision).
- [ram.asm audit, 2026-07-09] **RAM map report** — "never know what their real number is":
  nothing on the page shows where a field lands. A `sigil`-emitted per-region address map
  (name, address, size, padding, headroom vs budget) is pure tooling, no language surface;
  Spec-3 inlay hints are the eventual in-editor answer. Cheap; could ride any tranche. —
  OPEN (jotted).

## Tooling / build / process

- [port #1 hblank, 2026-07-09] **Aeon clean-tree build is not reproducible**: a fresh
  worktree `./build.sh` at the SAME commit (a103e46) emits ROMs ~131KB larger than the pinned
  references (582260 vs 451198 plain; both shapes uniformly bigger) — the prebuild generators
  produce different output than the untracked generated blobs sitting in the main tree, which
  are what the boot-checked ROM + all green harness gates key on. Not caused by the port
  (before/after neutrality was proven same-environment). Consequences today: aeon port
  branches must run IN the main tree (worktrees don't inherit untracked files); never rebuild
  in the main tree without re-pinning. Wants a real fix: either track/pin the generated
  outputs, or make the generators deterministic from tracked inputs and re-baseline. — OPEN
  (raise at a checkpoint; owns a session of its own).
- [port #1 hblank, 2026-07-09] **Source formatting convention set** (Volence's catch: the
  first draft of hblank.emp was flat-left inside its section braces — "start off strong"):
  code files use the braceless `module X in <section>` form (procs at col 0, classic asm
  indent inside); explicit `section { }` blocks indent members 4 (the sfx_bank precedent);
  instruction lines keep the .asm column style. Recorded in aeon CODING_CONVENTIONS.md §10;
  restyle byte-neutrality proven by re-running the port gates. Convention-only until
  `sigil fmt` (S2-D11(c)) — every new gap-ledger retrospect should eyeball formatting until
  then. — SHIPPED (convention; fmt tooling stays SPEC-LEDGERED S2-D11(c)).
- [port #1 hblank T4 review, 2026-07-09] **`initial_cpu: Cpu::M68000` is caller convention,
  not module fact** — hardcoded at four call sites (CLI + test paths); a braceless `.emp`
  module carries no cpu attribute and silently depends on every caller passing M68000. A
  future Z80 module (or a forgetful caller) mis-lowers with no module-level signal. Candidate:
  modules self-declare target CPU (`module x in y (cpu: z80)`?) or the pipeline
  defaults-and-warns. — OPEN.
- [tranche 2 T1 review, 2026-07-09] **`pc` is a reserved EA token in inner-base position** — a
  user symbol literally named `pc` can't be the sole inner base of `Sym(pc)` (the pc-rel
  carve-out wins, matching AS); `pc` as a displacement over a real register still works. One
  doc line owed in the .emp EA docs. — OPEN (docs-only).
- [tranche 2 T1 review, 2026-07-09] **PcRel range-check errors name distance+section but not
  the target SYMBOL** (sigil-link lib.rs ~482/498) — house style shared with bra/bsr messages;
  a cross-section disp8 target is almost always out of range, so the symbol name would pay.
  Repo-wide message-quality item. — OPEN.
- [tranche 2 T1 review, 2026-07-09] **abs.l as an .emp DESTINATION is unsupported**
  (`move.w x, ($abs).l` → "indirect base must be a register") — pre-existing, surfaced by an
  adversarial probe; will matter for some future port (VDP register writes spell this). — OPEN.
- [tranche 2 T1 review, 2026-07-09] **`Owner.label(pc)` multi-segment pc-rel target untested**
  — path shared verbatim with tested branch resolution; one-line test owed. — OPEN (low risk).
- [tranche 2 T3, port #2 (controllers.emp + math.emp), 2026-07-09] **`embed()` paths resolve
  relative to `include_root` directly — there is no separate "module's own directory" concept**
  — `math.emp`'s `embed("../data/sine.bin")` (the module lives in `engine/system/`, the embed
  target in `engine/data/`) could not resolve under ANY `include_root` value: the sandbox
  (`sigil-frontend-emp/src/eval/sandbox.rs::resolve_sandbox_path`) joins relative paths onto
  `include_root` and checks containment against that SAME root at every `..` pop — a single
  root can never both be narrow enough to serve as a sane "current directory" AND broad enough
  to contain a sibling directory one level up; the hazard is structural, not a wrong root
  choice (every prior port's `embed`s were bare same-directory filenames, so this never
  surfaced). Genuinely load-bearing: the real production CLI path
  (`sigil-cli/src/main.rs::run_build`) already sets `include_root` to the whole manifest
  `--root`, not a per-module directory, so this would have bitten a real build the first time
  any module's embed climbed a directory. — SHIPPED (port #2, per Volence's "step 2 is the
  quality apex" ruling above — build the missing construct rather than settle): a second,
  independent field, `embed_base` (`LowerOptions`/`Placement`/`Evaluator`), is the join BASE
  relative paths resolve against; `include_root` stays the sole containment boundary
  (`resolve_sandbox_path`'s final `starts_with` check is unchanged). `None` (the default)
  means "same as `include_root`" — every pre-existing caller is behavior-identical
  (`eval_data_with_root` still exists with its original signature, delegating to the new
  `eval_data_with_root_and_base` with `embed_base: None`). `~45` call sites needed a
  mechanical `embed_base: None,` addition to their `LowerOptions { .. }` literals (Rust's
  exhaustive-struct-literal rule, no `Default` on `LowerOptions`) — wide but shallow, each a
  single trivial line, zero behavior change. TDD: `sandbox.rs` gained two new unit tests
  (`resolve_sandbox_path_embed_base_allows_climbing_within_include_root`,
  `resolve_sandbox_path_embed_base_cannot_escape_include_root` — the latter proving
  `embed_base` grants NO extra reach past `include_root`).
- [tranche 2 T3, port #2 (math.emp), 2026-07-09] **`jsr`/`jmp` to a bare symbol genuinely
  undefined within the SAME AS compile unit hard-errors at the front-end's pass-convergence
  check — it never reaches the linker.** Unlike `movea.l #imm,aN`/`move.l #imm,dN`/
  `move.l #imm,(abs).w` (port #1 T3's `try_defer_long_imm`, which defers a genuinely-external
  immediate SOURCE to a `Value32Be` link fixup) and unlike `bra.w`/`bsr.w` (always PC-relative,
  always deferred via `PcRelDisp16`), `jsr`/`jmp`'s bare-symbol ABSOLUTE-target width
  (abs.w vs abs.l) is selected inside the AS front-end's OWN multi-pass fixpoint
  (`sigil-frontend-as/src/eval.rs::lower_m68k`, M1.D T3) — `Fragment::JmpJsrSym` (the
  length-variable deferred form, already fully supported end-to-end by
  `sigil-link/src/relax.rs`'s relaxation ladder AND already used unconditionally by the
  `.emp` front-end's `jbra`/`jbsr`) was NEVER constructed by the AS front-end; a target still
  Poison at convergence was unconditionally promoted to a hard `"unresolved symbol"` error.
  This is exactly the real shape aeon's `games/sonic4/objects/test_parent.asm:96` /
  `games/sonic4/player/player_ground.asm` (six sites) take — unconditional AS-side `jsr
  GetSineCosine` calls into a proc that is EITHER AS-side (`math.asm`, gate off) OR `.emp`-side
  (`math.emp`, gate on, resolved only at joint link) — so this would have broken the real
  `SIGIL_EMP_MATH`-gated mixed build, not just this port's synthetic test. — SHIPPED (port #2,
  TDD): `run()` now performs ONE bonus final pass (seeded from the SAME converged env) ONLY
  when ordinary convergence still leaves `poison` non-empty; that bonus pass's `jsr`/`jmp`
  sites still-Poison at that point emit `Fragment::JmpJsrSym` (via the backend's existing
  `lower_jmp_jsr_sym`, already used by `.emp`) instead of erroring — every OTHER operand kind
  is byte-identical to the ordinary pass, so the bonus pass's own leftover `poison` still names
  only genuinely-undefined symbols of any kind. Zero cost/behavior change for the overwhelming
  common case (empty `poison` at convergence — skips the bonus pass entirely). Proven inert:
  the FULL existing `m1d_rom`/`m1d_debug_rom`/all four prior mixed-ROM gates stayed
  byte-identical with this change in place — the deferral never fires unless something was
  ALREADY going to hard-error. Honest caveat (I1, whole-branch review): the TYPO case DID
  change — a pure-AS `jsr Nonexistent` now errors at LINK (resolve_layout's JmpJsrSym arm)
  instead of at assemble-time, and that arm's message initially named only the section, not
  the symbol; fixed same tranche by routing the arm through the shared
  `unresolved_abs_target_diag` (the `RelaxAbsSym` diagnostic machinery generalized), so the
  link-time error names the symbol with the cross-seam steer and the equ-cycle discrimination.
- [tranche 2 T3, port #2 (math.emp), 2026-07-09] **`resolve_layout` refuses ANY section
  mixing an `Org` back-patch with a relaxable fragment (`JmpJsrSym`/`RelaxAbsSym`/
  `RelaxLadder`), and this collision is REAL, not a false positive, for aeon's actual ROM
  layout** — `engine/engine.inc`'s `org $10000` opens the object-code-bank section and NEVER
  closes it before `gameDataIncludes` chains straight into the parallax data tables in the
  SAME section; `engine/parallax_macros.inc`'s `parallax_section_end` macro emits a genuine
  mid-section back-patch (`org pscStart` / `org pscEndPos`, a real `Fragment::Org`), and (once
  the jsr/jmp deferral above ships) `test_parent.asm`'s/`player_ground.asm`'s six `jsr
  GetSineCosine` sites land EARLIER in that same section as `Fragment::JmpJsrSym`. The M1.C
  T6b guard (`sigil-link/src/relax.rs:495-536`) fires exactly as designed: its
  `shift_breakpoints` prefix-sum layout math treats every `Org` as contributing zero length
  and never reads `Org.target`, which is sound ONLY when nothing before an `Org` in the same
  section can ever grow — true in every case examined before this port (M1.C T6b: "today's
  real Aeon sections either mix pure back-patched data with no relaxable... or relaxable-
  bearing code with no Org"), false now. Confirmed via two independent research passes: making
  the guard fixpoint-aware (only reject if a relaxable BEFORE an Org in the same section
  actually GROWS past its baseline rung — `GetSineCosine`'s real address is low enough to
  always fold to abs.w, so the growth this guards against could never fire here) requires
  `shift_breakpoints`/`frag_start_vma` to become genuinely `Org`-target-aware, not a
  guard-condition tweak — a real linker algorithm change. — SHIPPED (port #2 task 4, dedicated
  session): `Org` is now a POSITION BARRIER in `resolve_layout`'s width-shift math.
  `shift_breakpoints` seeks BOTH the current and baseline cursors to the org target at each
  barrier (resetting the per-run growth delta to 0), `frag_start_vma` does the same for its
  baseline cursor, and `shift_offset` scans last-wins (org resets make authored offsets
  non-monotonic; last-wins mirrors `image_bytes`' overwrite order and is identical to the old
  break-on-first for the monotonic no-org case). Growth of a relaxable shifts only fragments
  after it in its OWN run (up to the next org); post-org content stays pinned to the org
  target. The M1.C T6b categorical refusal is REPLACED by `run_overrun_diag`, a precise
  post-fixpoint check: a FORWARD org (judged at baseline) that a grown run overruns is a loud
  error naming the section/target/extent/overrun (AS/asl's spirit — never silently overlap); a
  backward org (the overwrite idiom) is never an overrun and `image_bytes`' overwrite semantics
  are unchanged. The Org+Reserve refusal survives (a distinct, still-latent hazard). The two
  six-module full mixed-ROM gates (`mixed_tranche2_rom_matches_assembled_reference`/`_debug_`)
  are UN-IGNORED and byte-identical to `aeon/s4.bin`/`aeon/s4.debug.bin`; every pre-existing
  byte gate (hblank 2/2, mixed 10/10, m1d plain+debug full-ROM, all port suites) stayed green,
  proving the algorithm change is byte-neutral for every previously-working layout. TDD: four
  new `relax.rs` unit tests (forward-org run-local shifting + pinned post-org content;
  run-growth-past-barrier loud error; backward-org overwrite byte-identity with an earlier
  relaxable; three-run run-local shifting), plus the two old categorical-refusal guard tests
  repurposed as allow-tests.
- [tranche 2 T4 review, 2026-07-09] **Backward `org` into a FIXUP's byte range diverges
  `image_bytes` from `link()`** (pre-existing, NOT introduced by the org-aware relaxation —
  reproduces with zero relaxables): `link()` replays the source-order overwrite then applies
  ALL fixups afterward, so a backward-org byte that lands inside an earlier fragment's fixup
  site gets re-clobbered by the fixup (probe: image_bytes `EE` vs link `12 34 56`-class).
  Latent — aeon's backward-org idiom (parallax_section_end `dc.b` back-patch) never seeks into
  a fixup site. Fix shape: fixup application should respect overwrite order (or refuse the
  overlap loudly). — OPEN (standalone linker hazard; distinct from the shipped org work).
- [port #2 tranche, step 2 (modernize pass), 2026-07-09] **`resolve::build_program`'s
  whole-program `report_unresolved` is incompatible with a cross-seam `.emp` module that
  has BOTH a `use` edge to another `.emp` module AND genuinely AS-side-only bare symbol
  references.** Discovered wiring `engine/system/controllers.emp`'s new `use
  engine.constants.{...}` (the constants-twin port) into `controllers_port.rs`: switching
  from plain `lower_module` to `Manifest::scan` + `build_program` (needed so the `use` edge
  resolves) makes `report_unresolved` (`resolve/mod.rs:500`) hard-error on `Ctrl_1_Held` &
  co — real RAM labels that live ONLY in `engine/ram.asm`, never in any `.emp` file, and are
  legitimately supplied to the test as a synthetic AS-side section appended AFTER
  `build_program` returns (the established cross-seam-port pattern since port #1). No prior
  port needed this: `controllers.emp` is the FIRST `.emp`-to-`.emp` `use` edge in a
  cross-seam file (`mixed_dac_rom.rs`'s `placed_module_sections` lowers six modules but only
  `controllers.emp` has a `use`). — WORKED AROUND (not fixed) per-callsite: each of the three
  affected test files (`controllers_port.rs`, `tranche2_negative_probes.rs`,
  `mixed_dac_rom.rs`) parses `constants.emp` once and manually prepends its items to
  `controllers.emp`'s own AST items before calling plain `lower_module` — mirroring
  `build_program`'s own internal `ambient_items`/synthetic-`File`-prepend technique
  (`resolve/mod.rs:132`, `:279-300`) minus the too-strict whole-program closure check. Two
  independent investigation passes (a background research agent + direct code reading)
  converged on the same diagnosis and the same workaround shape. The PRINCIPLED fix — either
  a `build_program` variant/parameter that tolerates a caller-declared set of "known
  external, resolved elsewhere" symbol names, or a way to compose a bounded ambient-const
  set without the BFS+report_unresolved machinery at all — is a source change to
  `sigil-frontend-emp`'s resolver, deliberately deferred rather than bundled into this
  byte-neutral pass (house rule: 2a-tier only). Re-open when a THIRD cross-seam `use` edge
  needs the same treatment (rule of three), or sooner if the per-callsite duplication (now
  three copies of the same ~15-line prepend pattern) starts drifting.
